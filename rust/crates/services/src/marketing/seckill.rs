//! 秒杀服务：活动 CRUD、库存/销量增减、结算页取价、参与资格判定。
//! 对应原 Java `SeckillServiceImpl`（908 行）。
//!
//! 迁移要点（逐条对照 Java 源）：
//! - `decStock`：`WHERE seckill_stock >= quantity` 原子扣减，不足则报错（防超卖）
//! - `incSales` / `decSales`：销量增减；回退用 `GREATEST(..., 0)` 防负
//! - `parsePrice`：按 `skuId` 批量预取明细，避免逐项查询（N+1）
//! - `canUserJoinSeckill`：未登录不拦截；无进行中活动不拦截；
//!   活动配置了人群（`member_scope`）时按人群匹配
//! - `savePromotion`：创建/更新同步 `promotion` 镜像行（type=1、
//!   relation_id=活动 id；窗口/名称/会员字段跟随，删除级联删镜像），
//!   详情促销聚合（`find_available` 活动池）据此组装秒杀条目并回填价/库存
//!
//! 未迁移（需人群服务就绪后补）：
//! - `collectSeckillRestrictions`（存量 `memberRange` / 新建 `member_scope` 的置灰逻辑）
//! - 批发商排除逻辑（依赖 `wholesale` 域）
//! - 人群匹配本身（镜像行 `member_scope` 已按 Java 写侧口径落库，读侧判定未迁移）

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_common::utils;
use nextrade_core::entities::{Product, Promotion, Seckill, SeckillItem};
use nextrade_core::models::marketing::coupon_manage::java_list_string;
use nextrade_core::models::marketing::promotion::TYPE_SECKILL;
use nextrade_core::models::marketing::seckill::{
    resolve_member_scope, SeckillCreateDTO, SeckillItemDTO, SeckillItemVO, SeckillListQuery,
    SeckillPromotionVO, SeckillStatus, SeckillUpdateDTO, SeckillVO,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 秒杀库存不足时的业务提示（与原 Java 文案一致）。
const ERR_STOCK: &str = "秒杀商品库存不足";

/// 活动范围：指定商品（Java `PromotionRangeEnum.PROMOTION_RANGE_PRODUCT`）。
const PROMOTION_RANGE_PRODUCT: i32 = 3;

/// 镜像行启用标记（活动池 `find_available` 以 `is_available = 1` 过滤；
/// 与优惠券 `savePromotion` 的落值口径一致）。
const PROMOTION_AVAILABLE_YES: i32 = 1;

pub struct SeckillService {
    repos: Arc<Repositories>,
}

impl SeckillService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 管理端 —————————————————————

    /// 秒杀活动分页（含明细与派生状态）。
    pub async fn list(&self, q: &SeckillListQuery) -> Result<PageResult<SeckillVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        let now = utils::now_seconds() as i32;
        let (total, rows) = self
            .repos.marketing.seckill
            .paginate(q.keyword.as_deref(), scoped_list_shop_id(q.shop_id), &page)
            .await?;

        let mut list = Vec::with_capacity(rows.len());
        for s in &rows {
            list.push(self.to_vo(s, now).await?);
        }
        Ok(PageResult::new(total, list, &page))
    }

    /// 秒杀活动详情。
    pub async fn detail(&self, seckill_id: i32) -> Result<SeckillVO> {
        let s = self
            .repos.marketing.seckill
            .find_by_id(seckill_id)
            .await?
            .ok_or_else(|| Error::NotFound("秒杀活动不存在".into()))?;
        // 多租户：店铺账号只能查看本店活动
        TenantScope::current_ensure_access(s.shop_id)?;
        self.to_vo(&s, utils::now_seconds() as i32).await
    }

    /// 创建秒杀活动（含明细），返回 `seckill_id`。
    ///
    /// 校验链：
    /// 1. 活动名非空
    /// 2. 起止时间合法（开始 < 结束）
    /// 3. 明细非空
    /// 4. 每条明细：商品必填、SKU 可空、秒杀价 > 0、库存 >= 0、时间窗合法
    /// 5. 同款 SKU 不得重复出现在其他活动中
    pub async fn create(&self, dto: &SeckillCreateDTO) -> Result<i32> {
        self.validate_create(dto)?;

        let start = dto.seckill_start_time.unwrap_or_default();
        let end = dto.seckill_end_time.unwrap_or_default();
        let seckill = Seckill {
            seckill_name: dto.seckill_name.clone(),
            seckill_start_time: dto.seckill_start_time,
            seckill_end_time: dto.seckill_end_time,
            seckill_limit_num: dto.seckill_limit_num,
            product_id: dto.product_id,
            // 多租户：店铺账号强制落本店；平台账号可指定/落平台级（0）
            shop_id: TenantScope::current().write_shop_id(dto.shop_id)?,
            ..Default::default()
        };
        let seckill_id = self.repos.marketing.seckill.insert(&seckill).await?;

        for item in &dto.seckill_item {
            self.check_item_conflict(item, Some(seckill_id)).await?;
            let row = self.build_item(seckill_id, item, start, end)?;
            self.repos.marketing.seckill.insert_item(&row).await?;
        }

        // Java `savePromotion`：同步 promotion 镜像行（详情促销聚合的活动池来源）
        self.save_promotion(seckill_id, &seckill, dto).await?;

        let log_info = format!(
            "新增秒杀活动:{}",
            seckill.seckill_name.as_deref().unwrap_or("")
        );
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        let sid = seckill.shop_id;
        if let Err(e) = admin_log
            .record_current_log(if sid > 0 { "shop" } else { "admin" }, sid, &log_info)
            .await
        {
            tracing::warn!("写管理操作日志失败（新增秒杀活动）: {e}");
        }

        tracing::info!("创建秒杀活动成功: id={seckill_id}, 明细 {} 条", dto.seckill_item.len());
        Ok(seckill_id)
    }

    /// 更新秒杀活动（明细整体重建）。
    ///
    /// 采用「先删后插」而非逐条 diff：与原 Java `saveSeckillItem` 的
    /// 整体覆盖语义一致，且避免残留已下架明细。
    pub async fn update(&self, dto: &SeckillUpdateDTO) -> Result<()> {
        let base = &dto.base;
        self.validate_create(base)?;

        // 活动必须存在；并校验归属（多租户）
        let existing = self
            .repos
            .marketing
            .seckill
            .find_by_id(dto.seckill_id)
            .await?
            .ok_or_else(|| Error::NotFound("秒杀活动不存在".into()))?;
        TenantScope::current_ensure_access(existing.shop_id)?;

        let start = base.seckill_start_time.unwrap_or_default();
        let end = base.seckill_end_time.unwrap_or_default();

        let seckill = Seckill {
            seckill_id: dto.seckill_id,
            seckill_name: base.seckill_name.clone(),
            seckill_start_time: base.seckill_start_time,
            seckill_end_time: base.seckill_end_time,
            seckill_limit_num: base.seckill_limit_num,
            product_id: base.product_id,
            // 多租户：店铺账号强制落本店
            shop_id: TenantScope::current().write_shop_id(base.shop_id)?,
        };
        self.repos.marketing.seckill.update(&seckill).await?;

        // 明细重建
        self.repos.marketing.seckill.delete_items(dto.seckill_id).await?;
        for item in &base.seckill_item {
            self.check_item_conflict(item, Some(dto.seckill_id)).await?;
            let row = self.build_item(dto.seckill_id, item, start, end)?;
            self.repos.marketing.seckill.insert_item(&row).await?;
        }

        // Java `savePromotion`：镜像行跟随窗口/名称/会员字段（改价经明细回填，不入镜像）
        self.save_promotion(dto.seckill_id, &seckill, base).await?;

        let log_info = format!(
            "更新秒杀活动:{}",
            seckill.seckill_name.as_deref().unwrap_or("")
        );
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        let sid = seckill.shop_id;
        if let Err(e) = admin_log
            .record_current_log(if sid > 0 { "shop" } else { "admin" }, sid, &log_info)
            .await
        {
            tracing::warn!("写管理操作日志失败（更新秒杀活动）: {e}");
        }

        tracing::info!("更新秒杀活动成功: id={}", dto.seckill_id);
        Ok(())
    }

    /// 删除秒杀活动（连同明细）。
    pub async fn delete(&self, seckill_id: i32) -> Result<()> {
        let existing = self
            .repos
            .marketing
            .seckill
            .find_by_id(seckill_id)
            .await?
            .ok_or_else(|| Error::NotFound("秒杀活动不存在".into()))?;
        // 多租户：删除前校验归属
        TenantScope::current_ensure_access(existing.shop_id)?;
        self.repos.marketing.seckill.delete(seckill_id).await?;
        // Java `del`：级联删除 promotion 镜像行（批量删除路径已在 seckill_batch 处理）
        self.repos
            .marketing
            .promotion
            .delete_by_relation(seckill_id, TYPE_SECKILL)
            .await?;
        tracing::info!("删除秒杀活动: id={seckill_id}");
        Ok(())
    }

    /// 同步 `promotion` 镜像行（Java `SeckillServiceImpl#savePromotion`）。
    ///
    /// 镜像已存在（relation_id + type=1）→ 仅更新窗口/名称/会员字段
    /// （`range`/`range_data` 在活动带商品时一并更新）；不存在 → 新建整行。
    /// 镜像是详情促销聚合的活动池来源：缺行则详情页组装不出秒杀条目、
    /// 秒杀价不回填（批次十五 10 个长期秒杀即因此缺行，本方法为其写侧修复）。
    async fn save_promotion(
        &self,
        seckill_id: i32,
        seckill: &Seckill,
        dto: &SeckillCreateDTO,
    ) -> Result<()> {
        let repo = &self.repos.marketing.promotion;
        let row = build_seckill_promotion(seckill, dto, seckill_id);
        if repo.find_by_relation(seckill_id, TYPE_SECKILL).await?.is_some() {
            repo.update_mirror_fields(&row).await?;
        } else {
            repo.insert(&row).await?;
        }
        Ok(())
    }

    // ————————————————————— 库存与销量 —————————————————————

    /// 扣减秒杀库存。库存不足返回业务错误。
    ///
    /// 对应 Java `SeckillServiceImpl#decStock`。**这是防超卖的核心**：
    /// SQL 层 `WHERE seckill_stock >= ?` 保证并发下不会扣成负数。
    pub async fn dec_stock(
        &self,
        product_id: i32,
        sku_id: Option<i32>,
        quantity: i32,
    ) -> Result<()> {
        if quantity <= 0 {
            return Err(Error::Validation("扣减数量必须大于0".into()));
        }
        let now = utils::now_seconds() as i32;
        let ok = self
            .repos.marketing.seckill
            .dec_stock(product_id, sku_id, quantity, now)
            .await?;
        if !ok {
            // 区分「无进行中活动」与「库存不足」：前者静默通过（与原 Java 一致）
            let has_activity = self
                .repos.marketing.seckill
                .find_active_item(product_id, sku_id, now)
                .await?
                .is_some();
            if has_activity {
                return Err(Error::biz(400, ERR_STOCK));
            }
            tracing::debug!("商品 {product_id} 无进行中秒杀活动，跳过库存扣减");
        }
        Ok(())
    }

    /// 回补秒杀库存（取消订单/退款）。
    pub async fn inc_stock(
        &self,
        product_id: i32,
        sku_id: Option<i32>,
        quantity: i32,
    ) -> Result<()> {
        if quantity <= 0 {
            return Ok(());
        }
        self.repos.marketing.seckill
            .inc_stock(product_id, sku_id, quantity)
            .await
    }

    /// 累加秒杀销量。
    pub async fn inc_sales(
        &self,
        product_id: i32,
        sku_id: Option<i32>,
        quantity: i32,
    ) -> Result<()> {
        if quantity <= 0 {
            return Ok(());
        }
        self.repos.marketing.seckill
            .inc_sales(product_id, sku_id, quantity)
            .await
    }

    /// 扣减秒杀销量（订单取消回退）。
    pub async fn dec_sales(
        &self,
        product_id: i32,
        sku_id: Option<i32>,
        quantity: i32,
    ) -> Result<()> {
        if quantity <= 0 {
            return Ok(());
        }
        self.repos.marketing.seckill
            .dec_sales(product_id, sku_id, quantity)
            .await
    }

    // ————————————————————— 结算页取价 —————————————————————

    /// 批量解析秒杀价：返回 `sku_id -> 秒杀价`。
    ///
    /// 对应 Java `SeckillServiceImpl#parsePrice`：按 `skuId` **批量预取**
    /// 秒杀明细，避免购物车逐项查询（N+1）。
    pub async fn resolve_prices(
        &self,
        seckill_id: i32,
        sku_ids: &[i32],
    ) -> Result<HashMap<i32, Decimal>> {
        let items = self
            .repos.marketing.seckill
            .find_items_by_skus(seckill_id, sku_ids)
            .await?;
        Ok(items
            .into_iter()
            .filter_map(|it| it.sku_id.map(|sid| (sid, it.seckill_price.unwrap_or_default())))
            .collect())
    }

    /// 查询商品/SKU 当前进行中的秒杀价（单条）。
    pub async fn current_seckill_price(
        &self,
        product_id: i32,
        sku_id: Option<i32>,
    ) -> Result<Option<Decimal>> {
        let now = utils::now_seconds() as i32;
        let item = self
            .repos.marketing.seckill
            .find_active_item(product_id, sku_id, now)
            .await?;
        Ok(item.and_then(|it| it.seckill_price))
    }

    // ————————————————————— 前台 —————————————————————

    /// 秒杀专区列表（当前进行中且有余量）。
    pub async fn active_list(
        &self,
        page_no: Option<u64>,
        page_size: Option<u64>,
    ) -> Result<PageResult<SeckillPromotionVO>> {
        let page = PageQuery {
            page: page_no.unwrap_or(1),
            page_size: page_size.unwrap_or(20),
        };
        let now = utils::now_seconds() as i32;
        let (total, items) = self.repos.marketing.seckill.find_active_items(now, &page).await?;
        // 前台卡片需要商品名称/主图/原价，按商品 ID 批量回填，避免 N+1。
        let product_ids: Vec<i32> = items
            .iter()
            .filter_map(|it| it.product_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        let products = self.repos.catalog.product.find_by_ids(&product_ids).await?;
        let product_map: HashMap<i32, Product> =
            products.into_iter().map(|p| (p.product_id, p)).collect();
        let list = items
            .iter()
            .map(|it| {
                let product = it.product_id.and_then(|id| product_map.get(&id));
                to_promotion_vo(it, now, product)
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 判断用户能否参与该商品当前进行中的秒杀。
    ///
    /// 对应 Java `SeckillServiceImpl#canUserJoinSeckill`：
    /// - 未登录（`user_id <= 0`）→ 不拦截
    /// - 无进行中秒杀活动 → 不拦截
    ///
    /// **未迁移**：`promotion.member_scope` 人群匹配与存量 `memberRange`
    /// 标记匹配（依赖 `promotion` 表与人群服务）。当前实现保持
    /// 「无活动即放行」的安全默认；人群限制需在 `promotion` 域补齐后接入。
    pub async fn can_user_join(
        &self,
        product_id: i32,
        sku_id: Option<i32>,
        user_id: i32,
    ) -> Result<bool> {
        if user_id <= 0 {
            return Ok(true);
        }
        let now = utils::now_seconds() as i32;
        let item = self
            .repos.marketing.seckill
            .find_active_item(product_id, sku_id, now)
            .await?;
        match item {
            None => Ok(true),
            Some(it) => {
                // 无进行中活动或无库存则不可参与
                let stock = it.seckill_stock.unwrap_or(0);
                if stock <= 0 {
                    return Ok(false);
                }
                tracing::debug!(
                    "用户 {user_id} 可参与秒杀 rec_id={:?}（人群限制待 promotion 域接入）",
                    it.rec_id
                );
                Ok(true)
            }
        }
    }

    /// 校验每人限购数量。
    ///
    /// 对应 Java 侧 `seckill_limit_num` 语义：0 表示不限购。
    pub async fn check_purchase_limit(
        &self,
        seckill_id: i32,
        quantity: i32,
    ) -> Result<()> {
        let s = self
            .repos.marketing.seckill
            .find_by_id(seckill_id)
            .await?
            .ok_or_else(|| Error::NotFound("秒杀活动不存在".into()))?;
        let limit = s.seckill_limit_num.unwrap_or(0);
        if limit > 0 && quantity > limit {
            return Err(Error::biz(
                400,
                format!("该秒杀活动每人限购 {limit} 件"),
            ));
        }
        Ok(())
    }

    // ————————————————————— 内部 —————————————————————

    /// 创建/更新参数校验。
    fn validate_create(&self, dto: &SeckillCreateDTO) -> Result<()> {
        let name = dto
            .seckill_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("秒杀活动名称不能为空".into()))?;
        if name.chars().count() > 120 {
            return Err(Error::Validation("秒杀活动名称过长".into()));
        }

        let start = dto
            .seckill_start_time
            .ok_or_else(|| Error::Validation("秒杀开始时间不能为空".into()))?;
        let end = dto
            .seckill_end_time
            .ok_or_else(|| Error::Validation("秒杀结束时间不能为空".into()))?;
        if start >= end {
            return Err(Error::Validation("秒杀开始时间必须早于结束时间".into()));
        }

        if dto.seckill_item.is_empty() {
            return Err(Error::Validation("秒杀商品明细不能为空".into()));
        }

        if let Some(limit) = dto.seckill_limit_num {
            if limit < 0 {
                return Err(Error::Validation("限购数量不能为负数".into()));
            }
        }

        for (idx, item) in dto.seckill_item.iter().enumerate() {
            let no = idx + 1;
            if item.product_id.unwrap_or(0) <= 0 {
                return Err(Error::Validation(format!("第 {no} 条明细缺少商品ID")));
            }
            // 单规格商品无 product_sku 行，skuId 空/0 合法，与原版读侧容忍口径一致。
            let price = item.seckill_price.unwrap_or_default();
            if price <= Decimal::ZERO {
                return Err(Error::Validation(format!("第 {no} 条明细的秒杀价必须大于0")));
            }
            if item.seckill_stock.unwrap_or(0) < 0 {
                return Err(Error::Validation(format!("第 {no} 条明细的库存不能为负数")));
            }
            // 明细时间窗缺省时继承活动级时间
            let s = item.seckill_start_time.unwrap_or(start);
            let e = item.seckill_end_time.unwrap_or(end);
            if s >= e {
                return Err(Error::Validation(format!(
                    "第 {no} 条明细的开始时间必须早于结束时间"
                )));
            }
        }
        Ok(())
    }

    /// 组装明细实体（时间窗缺省继承活动级）。
    fn build_item(
        &self,
        seckill_id: i32,
        item: &SeckillItemDTO,
        default_start: i32,
        default_end: i32,
    ) -> Result<SeckillItem> {
        Ok(SeckillItem {
            seckill_id: Some(seckill_id),
            product_id: item.product_id,
            sku_id: item.sku_id,
            seckill_price: item.seckill_price,
            seckill_stock: item.seckill_stock,
            // 新增明细销量从 0 起；更新场景整体重建亦从 0 起（与原 Java 覆盖语义一致）
            seckill_sales: Some(0),
            seckill_start_time: Some(item.seckill_start_time.unwrap_or(default_start)),
            seckill_end_time: Some(item.seckill_end_time.unwrap_or(default_end)),
            ..Default::default()
        })
    }

    /// 校验同款 SKU 是否已存在于其他秒杀活动。
    async fn check_item_conflict(
        &self,
        item: &SeckillItemDTO,
        exclude_seckill_id: Option<i32>,
    ) -> Result<()> {
        let pid = item.product_id.unwrap_or(0);
        let sid = item.sku_id.unwrap_or(0);
        if self
            .repos.marketing.seckill
            .exists_conflicting_item(pid, sid, exclude_seckill_id)
            .await?
        {
            return Err(Error::biz(
                400,
                format!("商品 {pid} 的 SKU {sid} 已存在于其他秒杀活动"),
            ));
        }
        Ok(())
    }

    /// 实体 → 管理端 VO（含明细与派生状态）。
    async fn to_vo(&self, s: &Seckill, now: i32) -> Result<SeckillVO> {
        let items = self.repos.marketing.seckill.find_items(s.seckill_id).await?;
        let start = s.seckill_start_time.unwrap_or(0);
        let end = s.seckill_end_time.unwrap_or(0);
        let status = SeckillStatus::of(now, start, end);
        Ok(SeckillVO {
            seckill_id: s.seckill_id,
            seckill_name: s.seckill_name.clone().unwrap_or_default(),
            seckill_start_time: start,
            seckill_end_time: end,
            seckill_limit_num: s.seckill_limit_num.unwrap_or(0),
            product_id: s.product_id.unwrap_or(0),
            shop_id: s.shop_id,
            seckill_item: items.iter().map(to_item_vo).collect(),
            status: status as i32,
            status_name: status.name().to_string(),
        })
    }
}

/// 明细实体 → VO。
fn to_item_vo(it: &SeckillItem) -> SeckillItemVO {
    SeckillItemVO {
        rec_id: it.rec_id,
        seckill_id: it.seckill_id.unwrap_or(0),
        product_id: it.product_id.unwrap_or(0),
        sku_id: it.sku_id.unwrap_or(0),
        seckill_price: it.seckill_price.unwrap_or_default(),
        seckill_stock: it.seckill_stock.unwrap_or(0),
        seckill_sales: it.seckill_sales.unwrap_or(0),
        seckill_start_time: it.seckill_start_time.unwrap_or(0),
        seckill_end_time: it.seckill_end_time.unwrap_or(0),
    }
}

/// 明细实体 → 前台 VO（计算剩余时间与售罄进度，并回填商品展示字段）。
fn to_promotion_vo(
    it: &SeckillItem,
    now: i32,
    product: Option<&Product>,
) -> SeckillPromotionVO {
    let start = it.seckill_start_time.unwrap_or(0);
    let end = it.seckill_end_time.unwrap_or(0);
    let stock = it.seckill_stock.unwrap_or(0);
    let sales = it.seckill_sales.unwrap_or(0);
    let started = now >= start;

    // 进行中显示距结束剩余，未开始显示距开始剩余；已结束为 0
    let remain_seconds = if now > end {
        0
    } else if started {
        i64::from(end - now)
    } else {
        i64::from(start - now)
    };

    // 售罄进度：已售 / (已售 + 剩余)。总量为 0 时视为 100%（售罄）
    let total = i64::from(sales.max(0)) + i64::from(stock.max(0));
    let sold_percent = if total <= 0 {
        100
    } else {
        ((i64::from(sales.max(0)) * 100) / total).clamp(0, 100) as i32
    };

    SeckillPromotionVO {
        rec_id: it.rec_id,
        seckill_id: it.seckill_id.unwrap_or(0),
        product_id: it.product_id.unwrap_or(0),
        sku_id: it.sku_id.unwrap_or(0),
        seckill_price: it.seckill_price.unwrap_or_default(),
        seckill_stock: stock,
        seckill_sales: sales,
        seckill_start_time: start,
        seckill_end_time: end,
        remain_seconds,
        started,
        sold_percent,
        product_name: product.map(|p| p.product_name.clone()).unwrap_or_default(),
        product_sn: product.map(|p| p.product_sn.clone()).unwrap_or_default(),
        product_brief: product.map(|p| p.product_brief.clone()).unwrap_or_default(),
        pic_url: product.map(|p| p.pic_url.clone()).unwrap_or_default(),
        pic_thumb: product.map(|p| p.pic_thumb.clone()).unwrap_or_default(),
        product_price: product.map(|p| p.product_price).unwrap_or_default(),
        market_price: product.map(|p| p.market_price).unwrap_or_default(),
        product_type: product.and_then(|p| p.product_type),
    }
}

/// 构造 `promotion` 镜像行（Java `SeckillServiceImpl#savePromotion` 新建分支）。
///
/// 字段口径逐条对照 Java：type=1、relation_id=活动 id、名称/窗口取活动本体、
/// shop_id 取活动落库店铺、range=指定商品（3）、range_data = 活动商品
/// `[productId]`（Java `List.toString()`）、sku_ids 见 [`mirror_sku_ids`]、
/// member_range/member_marks/member_scope 取 DTO（scope 经
/// [`resolve_member_scope`] 归一）；is_available=1 / is_delete=0
/// （Java 依赖表默认，PG 默认同为 1/0，此处显式落值与优惠券口径一致）。
fn build_seckill_promotion(
    seckill: &Seckill,
    dto: &SeckillCreateDTO,
    seckill_id: i32,
) -> Promotion {
    Promotion {
        promotion_id: 0,
        promotion_name: seckill.seckill_name.clone().unwrap_or_default(),
        start_time: seckill.seckill_start_time.unwrap_or(0),
        end_time: seckill.seckill_end_time.unwrap_or(0),
        r#type: TYPE_SECKILL,
        shop_id: seckill.shop_id,
        relation_id: Some(seckill_id),
        range: Some(PROMOTION_RANGE_PRODUCT),
        range_data: seckill.product_id.map(|pid| java_list_string(&[pid])),
        sku_ids: mirror_sku_ids(&dto.seckill_item),
        is_available: Some(PROMOTION_AVAILABLE_YES),
        is_delete: Some(0),
        member_range: dto.member_range.unwrap_or(0),
        member_marks: dto.member_marks.as_deref().map(java_list_string),
        member_scope: resolve_member_scope(
            dto.send_user_target.as_ref(),
            dto.member_audience.as_ref(),
        ),
    }
}

/// 镜像行 `sku_ids`（Java 新建分支：明细中 `skuId != null` 者收集为
/// `List.toString()`；为空则不写、落 NULL）。
///
/// Java 另有 `secondsSeckill == 1` 过滤：我方 DTO 与 admin 提交均无此字段
/// （全部明细皆为参与项），故仅按 skuId 是否存在判定。
fn mirror_sku_ids(items: &[SeckillItemDTO]) -> Option<String> {
    let sku_ids: Vec<i32> = items.iter().filter_map(|it| it.sku_id).collect();
    if sku_ids.is_empty() {
        None
    } else {
        Some(java_list_string(&sku_ids))
    }
}

/// 列表店铺过滤：店铺账号强制本店；平台账号可用请求值精确过滤（None = 全部）。
fn scoped_list_shop_id(requested: Option<i32>) -> Option<i32> {
    TenantScope::current_list_filter().or(requested.filter(|id| *id > 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn item(start: i32, end: i32, stock: i32, sales: i32) -> SeckillItem {
        SeckillItem {
            rec_id: 1,
            seckill_id: Some(2),
            product_id: Some(3),
            sku_id: Some(4),
            seckill_price: Some(Decimal::from_str("9.90").unwrap()),
            seckill_stock: Some(stock),
            seckill_sales: Some(sales),
            seckill_start_time: Some(start),
            seckill_end_time: Some(end),
        }
    }

    #[test]
    fn promotion_vo_running_counts_down_to_end() {
        let vo = to_promotion_vo(&item(100, 300, 10, 30), 200, None);
        assert!(vo.started);
        assert_eq!(vo.remain_seconds, 100); // 距结束
        assert_eq!(vo.sold_percent, 75); // 30 / (30+10)
    }

    #[test]
    fn promotion_vo_not_started_counts_down_to_start() {
        let vo = to_promotion_vo(&item(100, 300, 10, 0), 40, None);
        assert!(!vo.started);
        assert_eq!(vo.remain_seconds, 60); // 距开始
        assert_eq!(vo.sold_percent, 0);
    }

    #[test]
    fn promotion_vo_finished_has_zero_remain() {
        let vo = to_promotion_vo(&item(100, 300, 0, 50), 400, None);
        assert_eq!(vo.remain_seconds, 0);
        // 库存与销量均为边界：总量 = 50，全部售出
        assert_eq!(vo.sold_percent, 100);
    }

    #[test]
    fn promotion_vo_sold_out_when_total_zero() {
        // 库存与销量都为 0：视为售罄，进度 100%，避免除零
        let vo = to_promotion_vo(&item(100, 300, 0, 0), 200, None);
        assert_eq!(vo.sold_percent, 100);
    }

    #[test]
    fn item_vo_maps_all_fields() {
        let vo = to_item_vo(&item(100, 300, 10, 3));
        assert_eq!(vo.rec_id, 1);
        assert_eq!(vo.seckill_id, 2);
        assert_eq!(vo.product_id, 3);
        assert_eq!(vo.sku_id, 4);
        assert_eq!(vo.seckill_stock, 10);
        assert_eq!(vo.seckill_sales, 3);
    }

    #[test]
    fn item_vo_tolerates_null_columns() {
        // 库中这些列可空，全空时不应 panic
        let vo = to_item_vo(&SeckillItem::default());
        assert_eq!(vo.rec_id, 0);
        assert_eq!(vo.seckill_price, Decimal::ZERO);
    }

    #[tokio::test]
    async fn tenant_scope_scopes_list_and_blocks_cross_shop_writes() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        let shop = TenantScope::for_shop(8);
        let (filter, write_shop, blocked) = with_tenant_scope(shop, async {
            (
                scoped_list_shop_id(None),
                TenantScope::current().write_shop_id(None).unwrap_or(-1),
                TenantScope::current_ensure_access(9).is_err(),
            )
        })
        .await;
        assert_eq!(filter, Some(8));
        assert_eq!(write_shop, 8);
        assert!(blocked);
        // 平台账号：列表不过滤、写入可指定店铺
        let (filter, write_shop) = with_tenant_scope(TenantScope::platform(), async {
            (
                scoped_list_shop_id(Some(9)),
                TenantScope::current().write_shop_id(Some(9)).unwrap_or(-1),
            )
        })
        .await;
        assert_eq!(filter, Some(9));
        assert_eq!(write_shop, 9);
    }

    #[tokio::test]
    async fn validate_create_allows_item_without_sku() {
        let pool = sqlx::PgPool::connect_lazy("postgres://test:test@127.0.0.1:5432/test")
            .expect("lazy pool should not connect");
        let svc = SeckillService::new(Arc::new(Repositories::new(pool)));
        let dto = SeckillCreateDTO {
            seckill_name: Some("无 SKU 商品秒杀".into()),
            seckill_start_time: Some(100),
            seckill_end_time: Some(200),
            seckill_limit_num: Some(0),
            product_id: Some(3),
            shop_id: Some(0),
            seckill_item: vec![SeckillItemDTO {
                product_id: Some(3),
                sku_id: None,
                seckill_price: Some(Decimal::from_str("9.90").unwrap()),
                seckill_stock: Some(10),
                ..Default::default()
            }],
        };

        assert!(svc.validate_create(&dto).is_ok());
    }

    fn mirror_dto(items: Vec<SeckillItemDTO>) -> SeckillCreateDTO {
        SeckillCreateDTO {
            seckill_name: Some("长期秒杀".into()),
            seckill_start_time: Some(100),
            seckill_end_time: Some(200),
            product_id: Some(1581),
            shop_id: Some(0),
            seckill_item: items,
            ..Default::default()
        }
    }

    #[test]
    fn seckill_promotion_mirror_matches_java_save_promotion() {
        // 无 SKU 明细（批次十五 10 个长期秒杀的形态）：sku_ids 落 NULL
        let dto = mirror_dto(vec![SeckillItemDTO {
            product_id: Some(1581),
            seckill_price: Some(Decimal::from_str("8.91").unwrap()),
            seckill_stock: Some(100),
            ..Default::default()
        }]);
        let seckill = Seckill {
            seckill_name: dto.seckill_name.clone(),
            seckill_start_time: dto.seckill_start_time,
            seckill_end_time: dto.seckill_end_time,
            product_id: dto.product_id,
            shop_id: 0,
            ..Default::default()
        };
        let row = build_seckill_promotion(&seckill, &dto, 3);
        assert_eq!(row.r#type, TYPE_SECKILL);
        assert_eq!(row.relation_id, Some(3));
        assert_eq!(row.promotion_name, "长期秒杀");
        assert_eq!(row.start_time, 100);
        assert_eq!(row.end_time, 200);
        assert_eq!(row.shop_id, 0);
        assert_eq!(row.range, Some(3));
        assert_eq!(row.range_data.as_deref(), Some("[1581]"));
        assert_eq!(row.sku_ids, None);
        assert_eq!(row.is_available, Some(1));
        assert_eq!(row.is_delete, Some(0));
        assert_eq!(row.member_range, 0);
        assert_eq!(row.member_marks, None);
        assert_eq!(row.member_scope, None);
    }

    #[test]
    fn seckill_promotion_mirror_collects_sku_ids_and_member_fields() {
        let mut dto = mirror_dto(vec![
            SeckillItemDTO {
                product_id: Some(1581),
                sku_id: Some(101),
                ..Default::default()
            },
            SeckillItemDTO {
                product_id: Some(1581),
                sku_id: Some(102),
                ..Default::default()
            },
            // 无 skuId 的明细不进 sku_ids
            SeckillItemDTO {
                product_id: Some(1581),
                ..Default::default()
            },
        ]);
        dto.member_range = Some(1);
        dto.member_marks = Some(vec![1, 2]);
        dto.send_user_target = Some(serde_json::json!({"scope": 1, "tagIds": [1, 2]}));
        let seckill = Seckill {
            seckill_name: dto.seckill_name.clone(),
            seckill_start_time: dto.seckill_start_time,
            seckill_end_time: dto.seckill_end_time,
            product_id: dto.product_id,
            shop_id: 0,
            ..Default::default()
        };
        let row = build_seckill_promotion(&seckill, &dto, 9);
        assert_eq!(row.sku_ids.as_deref(), Some("[101, 102]"));
        assert_eq!(row.member_range, 1);
        assert_eq!(row.member_marks.as_deref(), Some("[1, 2]"));
        let scope: serde_json::Value =
            serde_json::from_str(row.member_scope.as_deref().unwrap()).unwrap();
        assert_eq!(scope["scope"], 1);
        assert_eq!(scope["tagIds"], serde_json::json!([1, 2]));
    }
}
