//! 限时折扣服务：活动 CRUD、折扣计算、结算取价、跨活动冲突校验。
//! 对应原 Java `TimeDiscountServiceImpl`（491 行）。
//!
//! 迁移要点（逐条对照 Java 源）：
//! - `getTimeDiscountPrice`：三种折扣类型（折扣率/立减/固定价）
//! - `appliesToSku`：`sku_ids` 为空表示作用于该商品全部规格
//! - `checkPromotionByProduct`：**跨活动冲突校验** —— 同一商品在同一
//!   时间窗内不得同时参与限时折扣、秒杀、拼团
//! - `handleStatusName`：时间状态推导（边界包含）
//! - `parsePrice`：已按目标价则不再重复累计优惠（`alreadyApplied` 判定）
//!
//! 未迁移（依赖 `promotion` 表与人群服务）：
//! - `savePromotion`（写 `promotion` 表关联记录）
//! - `member_scope` / `memberRange` 人群匹配
//! - 拼团冲突校验（依赖 `groupon_promotion_item` 表，见 `groupon` 域）

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;
use rust_decimal::RoundingStrategy;
use nextrade_common::utils;
use nextrade_core::entities::{TimeDiscount, TimeDiscountItem};
use nextrade_core::models::marketing::time_discount::{
    parse_sku_ids, serialize_sku_ids, DiscountType, PromotionTimeStatus, TimeDiscountCreateDTO,
    TimeDiscountDetailVO, TimeDiscountItemDTO, TimeDiscountItemVO, TimeDiscountListQuery,
    TimeDiscountPromotionVO, TimeDiscountUpdateDTO,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 商品已在其他限时折扣活动中的提示（与原 Java 文案一致）。
const ERR_EXIST_DISCOUNT: &str = "该商品在时间段内已参与其他限时折扣活动";
/// 商品已在秒杀活动中的提示。
const ERR_EXIST_SECKILL: &str = "该商品在时间段内已参与秒杀活动";

pub struct TimeDiscountService {
    repos: Arc<Repositories>,
}

impl TimeDiscountService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 管理端 —————————————————————

    /// 活动分页（含明细与派生状态）。
    pub async fn list(&self, q: &TimeDiscountListQuery) -> Result<PageResult<TimeDiscountDetailVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        let now = utils::now_seconds() as i32;
        let (total, rows) = self
            .repos.marketing.time_discount
            .paginate(q.keyword.as_deref(), scoped_list_shop_id(q.shop_id), &page)
            .await?;

        let mut list = Vec::with_capacity(rows.len());
        for d in &rows {
            list.push(self.to_vo(d, now).await?);
        }
        Ok(PageResult::new(total, list, &page))
    }

    /// 活动详情。
    pub async fn detail(&self, discount_id: i32) -> Result<TimeDiscountDetailVO> {
        let d = self
            .repos.marketing.time_discount
            .find_by_id(discount_id)
            .await?
            .ok_or_else(|| Error::NotFound("限时折扣活动不存在".into()))?;
        // 多租户：店铺账号只能查看本店活动
        TenantScope::current_ensure_access(d.shop_id)?;
        self.to_vo(&d, utils::now_seconds() as i32).await
    }

    /// 创建限时折扣活动，返回 `discount_id`。
    ///
    /// 校验链：
    /// 1. 活动名非空、时间窗合法
    /// 2. 明细非空，每条明细的商品/折扣值/折扣类型合法
    /// 3. **跨活动冲突校验**：商品在该时间窗内不得已参与其他限时折扣或秒杀
    pub async fn create(&self, dto: &TimeDiscountCreateDTO) -> Result<i32> {
        let (start, end) = self.validate(dto)?;

        // 跨活动冲突校验（对应 Java checkPromotionByProduct）
        let product_ids = collect_product_ids(dto);
        self.check_conflicts(&product_ids, start, end, None).await?;

        let activity = TimeDiscount {
            promotion_name: dto.promotion_name.clone().unwrap_or_default(),
            start_time: start,
            end_time: end,
            shop_id: TenantScope::current().write_shop_id(dto.shop_id)?,
            add_time: Some(utils::now_seconds() as i32),
            ..Default::default()
        };
        let discount_id = self.repos.marketing.time_discount.insert(&activity).await?;

        for item in &dto.item {
            let row = build_item(discount_id, item, start, end);
            self.repos.marketing.time_discount.insert_item(&row).await?;
        }

        let log_info = format!("新增限时折扣活动:{}", activity.promotion_name);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        let sid = activity.shop_id;
        if let Err(e) = admin_log
            .record_current_log(if sid > 0 { "shop" } else { "admin" }, sid, &log_info)
            .await
        {
            tracing::warn!("写管理操作日志失败（新增限时折扣活动）: {e}");
        }

        tracing::info!(
            "创建限时折扣活动成功: id={discount_id}, 明细 {} 条",
            dto.item.len()
        );
        Ok(discount_id)
    }

    /// 更新限时折扣活动（明细整体重建）。
    pub async fn update(&self, dto: &TimeDiscountUpdateDTO) -> Result<()> {
        let (start, end) = self.validate(&dto.base)?;

        let existing = self
            .repos
            .marketing
            .time_discount
            .find_by_id(dto.discount_id)
            .await?
            .ok_or_else(|| Error::NotFound("限时折扣活动不存在".into()))?;
        // 多租户：更新前校验归属
        TenantScope::current_ensure_access(existing.shop_id)?;

        let product_ids = collect_product_ids(&dto.base);
        self.check_conflicts(&product_ids, start, end, Some(dto.discount_id))
            .await?;

        let activity = TimeDiscount {
            discount_id: dto.discount_id,
            promotion_name: dto.base.promotion_name.clone().unwrap_or_default(),
            start_time: start,
            end_time: end,
            // 多租户：店铺账号强制落本店
            shop_id: TenantScope::current().write_shop_id(dto.base.shop_id)?,
            ..Default::default()
        };
        self.repos.marketing.time_discount.update(&activity).await?;

        self.repos.marketing.time_discount
            .delete_items(dto.discount_id)
            .await?;
        for item in &dto.base.item {
            let row = build_item(dto.discount_id, item, start, end);
            self.repos.marketing.time_discount.insert_item(&row).await?;
        }

        let log_info = format!("更新限时折扣活动:{}", activity.promotion_name);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        let sid = activity.shop_id;
        if let Err(e) = admin_log
            .record_current_log(if sid > 0 { "shop" } else { "admin" }, sid, &log_info)
            .await
        {
            tracing::warn!("写管理操作日志失败（更新限时折扣活动）: {e}");
        }

        tracing::info!("更新限时折扣活动成功: id={}", dto.discount_id);
        Ok(())
    }

    /// 删除活动（连同明细）。
    pub async fn delete(&self, discount_id: i32) -> Result<()> {
        let existing = self
            .repos
            .marketing
            .time_discount
            .find_by_id(discount_id)
            .await?
            .ok_or_else(|| Error::NotFound("限时折扣活动不存在".into()))?;
        // 多租户：删除前校验归属
        TenantScope::current_ensure_access(existing.shop_id)?;
        self.repos.marketing.time_discount.delete(discount_id).await?;
        tracing::info!("删除限时折扣活动: id={discount_id}");
        Ok(())
    }

    // ————————————————————— 折扣计算 —————————————————————

    /// 计算折扣后价格。
    ///
    /// 对应 Java `getTimeDiscountPrice`：
    /// - `discount_type = 1`（折扣率）：`原价 × 折数 / 10`，保留 2 位小数（四舍五入）
    /// - `discount_type = 2`（立减）：`原价 - 减免额`
    /// - `discount_type = 3`（固定价）：直接取 `value`
    ///
    /// 结果不会为负（立减超过原价时归零），也不会高于原价。
    pub fn calc_discount_price(
        product_price: Decimal,
        discount_type: i32,
        value: Decimal,
    ) -> Decimal {
        let ten = Decimal::from(10);
        let price = match DiscountType::try_from(discount_type) {
            Ok(DiscountType::Rate) => {
                // 与原 Java 一致：divide(10, 2, HALF_UP) 后相乘
                let rate = value / ten;
                let rate = rate.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
                product_price * rate
            }
            Ok(DiscountType::Reduce) => product_price - value,
            Ok(DiscountType::Fixed) => value,
            // 未知类型不改价（保守兜底，避免误打折）
            Err(_) => product_price,
        };
        // 折后价约束在 [0, 原价] 区间
        price.max(Decimal::ZERO).min(product_price)
    }

    /// 判断明细是否作用于指定 SKU。
    ///
    /// 对应 Java `appliesToSku`：`sku_ids` 为空表示作用于该商品全部规格。
    pub fn applies_to_sku(item: &TimeDiscountItem, sku_id: Option<i32>) -> bool {
        let ids = parse_sku_ids(item.sku_ids.as_deref());
        if ids.is_empty() {
            return true;
        }
        match sku_id {
            Some(sid) => ids.contains(&sid),
            // 未指定 SKU 时（商品级查询）视为适用，交由调用方按商品维度处理
            None => true,
        }
    }

    /// 计算某商品在**当前进行中**折扣下的折后价。
    ///
    /// 返回 `None` 表示无适用折扣。
    pub async fn current_discount_price(
        &self,
        product_id: i32,
        sku_id: Option<i32>,
        product_price: Decimal,
    ) -> Result<Option<Decimal>> {
        let now = utils::now_seconds() as i32;
        let items = self
            .repos.marketing.time_discount
            .find_active_items(product_id, now)
            .await?;

        // 取优惠力度最大（折后价最低）的一条
        let best = items
            .iter()
            .filter(|it| Self::applies_to_sku(it, sku_id))
            .map(|it| Self::calc_discount_price(product_price, it.discount_type, it.value))
            .min();

        Ok(best.filter(|p| *p < product_price))
    }

    /// 批量计算折扣价：返回 `(product_id, sku_id) -> 折后价`。
    ///
    /// 对应 Java `parsePrice`：按活动批量取明细后逐购物车行匹配。
    /// 同时返回**单件优惠金额**，便于上层累计促销总额。
    pub async fn resolve_prices(
        &self,
        discount_id: i32,
        lines: &[(i32, Option<i32>, Decimal)],
    ) -> Result<HashMap<(i32, Option<i32>), Decimal>> {
        let mut out = HashMap::new();
        // 按商品聚合，避免同一商品重复查库
        let mut by_product: HashMap<i32, Vec<&(i32, Option<i32>, Decimal)>> = HashMap::new();
        for line in lines {
            by_product.entry(line.0).or_default().push(line);
        }

        for (product_id, group) in by_product {
            let items = self
                .repos.marketing.time_discount
                .find_items_for_product(discount_id, product_id)
                .await?;
            for line in group {
                let (_, sku_id, price) = *line;
                let best = items
                    .iter()
                    .filter(|it| Self::applies_to_sku(it, sku_id))
                    .map(|it| Self::calc_discount_price(price, it.discount_type, it.value))
                    .min();
                if let Some(p) = best.filter(|p| *p < price) {
                    out.insert((product_id, sku_id), p);
                }
            }
        }
        Ok(out)
    }

    /// 计算购物车行的优惠金额（单件优惠 × 数量，不为负）。
    ///
    /// 对应 Java `parsePrice` 中 `discountAmount` 的累计逻辑。
    /// `already_applied` 为真时返回 0（价格已按目标价，不重复计优惠）。
    pub fn calc_saved(
        original_price: Decimal,
        discount_price: Decimal,
        quantity: i32,
        already_applied: bool,
    ) -> Decimal {
        if already_applied {
            return Decimal::ZERO;
        }
        let saved = (original_price - discount_price) * Decimal::from(quantity.max(0));
        saved.max(Decimal::ZERO)
    }

    /// 时间状态（对应 Java `handleStatusName`）。
    pub fn status_of(now: i32, start: i32, end: i32) -> PromotionTimeStatus {
        PromotionTimeStatus::of(now, start, end)
    }

    // ————————————————————— 前台 —————————————————————

    /// 折扣专区列表（当前进行中）。
    pub async fn active_list(
        &self,
        page_no: Option<u64>,
        page_size: Option<u64>,
    ) -> Result<PageResult<TimeDiscountPromotionVO>> {
        let page = PageQuery {
            page: page_no.unwrap_or(1),
            page_size: page_size.unwrap_or(20),
        };
        let now = utils::now_seconds() as i32;
        let (total, items) = self
            .repos.marketing.time_discount
            .find_active_page(now, &page)
            .await?;
        let list = items.iter().map(|it| to_promotion_vo(it, now)).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 判断用户能否参与该商品当前进行中的限时折扣。
    ///
    /// 对应 Java `canUserJoinTimeDiscount`：未登录不拦截；无适用活动不拦截。
    ///
    /// **未迁移**：`promotion.member_scope` 人群匹配与存量 `memberRange`
    /// 标记匹配（依赖 `promotion` 表与人群服务）。在人群能力接入前，
    /// 已登录且存在适用活动时**恒放行** —— 与 Java 在 `promotion` 记录
    /// 缺失时 `continue` 跳过判定、最终返回 `true` 的行为一致。
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
        let items = self
            .repos.marketing.time_discount
            .find_active_items(product_id, now)
            .await?;
        let applies = items.iter().any(|it| Self::applies_to_sku(it, sku_id));
        if applies {
            tracing::debug!(
                "用户 {user_id} 命中限时折扣 product={product_id}（人群限制待 promotion 域接入）"
            );
        }
        Ok(true)
    }

    // ————————————————————— 内部 —————————————————————

    /// 参数校验，返回 `(start, end)`。
    fn validate(&self, dto: &TimeDiscountCreateDTO) -> Result<(i32, i32)> {
        let name = dto
            .promotion_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("活动名称不能为空".into()))?;
        if name.chars().count() > 255 {
            return Err(Error::Validation("活动名称过长".into()));
        }

        let start = dto
            .start_time
            .ok_or_else(|| Error::Validation("开始时间不能为空".into()))?;
        let end = dto
            .end_time
            .ok_or_else(|| Error::Validation("结束时间不能为空".into()))?;
        if start >= end {
            return Err(Error::Validation("开始时间必须早于结束时间".into()));
        }

        if dto.item.is_empty() {
            return Err(Error::Validation("折扣商品明细不能为空".into()));
        }

        for (idx, item) in dto.item.iter().enumerate() {
            let no = idx + 1;
            if item.product_id.unwrap_or(0) <= 0 {
                return Err(Error::Validation(format!("第 {no} 条明细缺少商品ID")));
            }
            let dt = item
                .discount_type
                .ok_or_else(|| Error::Validation(format!("第 {no} 条明细缺少折扣类型")))?;
            let dtype = DiscountType::try_from(dt).map_err(|_| {
                Error::Validation(format!("第 {no} 条明细的折扣类型非法（应为 1/2/3）"))
            })?;
            let value = item.value.unwrap_or_default();
            if value <= Decimal::ZERO {
                return Err(Error::Validation(format!("第 {no} 条明细的折扣值必须大于0")));
            }
            // 折扣率需落在 (0, 10] 区间，否则折后价会高于原价或为负
            if dtype == DiscountType::Rate && value > Decimal::from(10) {
                return Err(Error::Validation(format!(
                    "第 {no} 条明细的折扣率不能大于10（10 表示不打折）"
                )));
            }
            let s = item.start_time.unwrap_or(start);
            let e = item.end_time.unwrap_or(end);
            if s >= e {
                return Err(Error::Validation(format!(
                    "第 {no} 条明细的开始时间必须早于结束时间"
                )));
            }
        }
        Ok((start, end))
    }

    /// 跨活动冲突校验（对应 Java `checkPromotionByProduct`）。
    ///
    /// 校验同商品在同时间窗内是否已参与：
    /// 1. 其他限时折扣
    /// 2. 秒杀活动
    ///
    /// 拼团冲突校验待 `groupon` 域就绪后补充。
    async fn check_conflicts(
        &self,
        product_ids: &[i32],
        start: i32,
        end: i32,
        exclude_discount_id: Option<i32>,
    ) -> Result<()> {
        if product_ids.is_empty() {
            return Ok(());
        }

        let discount_conflicts = self
            .repos.marketing.time_discount
            .count_conflicting_discount(product_ids, start, end, exclude_discount_id)
            .await?;
        if discount_conflicts > 0 {
            return Err(Error::biz(400, ERR_EXIST_DISCOUNT));
        }

        let seckill_conflicts = self
            .repos.marketing.time_discount
            .count_conflicting_seckill(product_ids, start, end)
            .await?;
        if seckill_conflicts > 0 {
            return Err(Error::biz(400, ERR_EXIST_SECKILL));
        }

        Ok(())
    }

    /// 实体 → 详情 VO。
    async fn to_vo(&self, d: &TimeDiscount, now: i32) -> Result<TimeDiscountDetailVO> {
        let items = self.repos.marketing.time_discount.find_items(d.discount_id).await?;
        let status = PromotionTimeStatus::of(now, d.start_time, d.end_time);
        Ok(TimeDiscountDetailVO {
            discount_id: d.discount_id,
            promotion_name: d.promotion_name.clone(),
            start_time: d.start_time,
            end_time: d.end_time,
            shop_id: d.shop_id,
            add_time: d.add_time.unwrap_or(0),
            item: items.iter().map(to_item_vo).collect(),
            status: status as i32,
            status_name: status.description().to_string(),
        })
    }
}

/// 收集参与活动的商品 ID（显式列表优先，缺失时从明细推导）。
fn collect_product_ids(dto: &TimeDiscountCreateDTO) -> Vec<i32> {
    if !dto.product_ids.is_empty() {
        let mut ids = dto.product_ids.clone();
        ids.sort_unstable();
        ids.dedup();
        return ids;
    }
    let mut ids: Vec<i32> = dto.item.iter().filter_map(|i| i.product_id).collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// 明细 DTO → 实体。
fn build_item(
    discount_id: i32,
    item: &TimeDiscountItemDTO,
    default_start: i32,
    default_end: i32,
) -> TimeDiscountItem {
    TimeDiscountItem {
        discount_id,
        start_time: item.start_time.unwrap_or(default_start),
        end_time: item.end_time.unwrap_or(default_end),
        product_id: item.product_id.unwrap_or(0),
        sku_ids: serialize_sku_ids(&item.sku_ids),
        value: item.value.unwrap_or_default(),
        discount_type: item.discount_type.unwrap_or(1),
        ..Default::default()
    }
}

/// 明细实体 → VO。
fn to_item_vo(it: &TimeDiscountItem) -> TimeDiscountItemVO {
    let dtype = DiscountType::try_from(it.discount_type).ok();
    TimeDiscountItemVO {
        item_id: it.item_id,
        discount_id: it.discount_id,
        product_id: it.product_id,
        sku_ids: parse_sku_ids(it.sku_ids.as_deref()),
        value: it.value,
        discount_type: it.discount_type,
        discount_type_name: dtype.map(|d| d.name().to_string()).unwrap_or_default(),
        start_time: it.start_time,
        end_time: it.end_time,
    }
}

/// 明细实体 → 前台 VO。
fn to_promotion_vo(it: &TimeDiscountItem, now: i32) -> TimeDiscountPromotionVO {
    let started = now >= it.start_time;
    let remain_seconds = if now > it.end_time {
        0
    } else if started {
        i64::from(it.end_time - now)
    } else {
        i64::from(it.start_time - now)
    };
    let dtype = DiscountType::try_from(it.discount_type).ok();
    TimeDiscountPromotionVO {
        item_id: it.item_id,
        discount_id: it.discount_id,
        product_id: it.product_id,
        sku_ids: parse_sku_ids(it.sku_ids.as_deref()),
        value: it.value,
        discount_type: it.discount_type,
        discount_type_name: dtype.map(|d| d.name().to_string()).unwrap_or_default(),
        discount_label: dtype.map(|d| d.label(it.value)).unwrap_or_default(),
        start_time: it.start_time,
        end_time: it.end_time,
        remain_seconds,
        started,
    }
}

/// 列表店铺过滤：店铺账号强制本店；平台账号可用请求值精确过滤（None = 全部）。
fn scoped_list_shop_id(requested: Option<i32>) -> Option<i32> {
    TenantScope::current_list_filter().or(requested.filter(|id| *id > 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn item_with_skus(sku_ids: Option<&str>) -> TimeDiscountItem {
        TimeDiscountItem {
            item_id: 1,
            discount_id: 2,
            product_id: 3,
            sku_ids: sku_ids.map(String::from),
            value: dec("8.5"),
            discount_type: 1,
            start_time: 100,
            end_time: 200,
        }
    }

    #[test]
    fn rate_discount_calculation() {
        // 8.5 折：100 × 0.85 = 85.00
        let p = TimeDiscountService::calc_discount_price(dec("100"), 1, dec("8.5"));
        assert_eq!(p, dec("85.00"));
    }

    #[test]
    fn rate_is_rounded_before_multiply_not_after() {
        // 忠实复刻 Java `getTimeDiscountPrice`：
        //   value.divide(10, 2, HALF_UP) 只对**折数**取 2 位，乘积不再取整。
        // 9.9 折 → rate = 0.99（2 位）；10.05 × 0.99 = 9.9495（保留 4 位小数）
        // 若误在乘积后取整会得到 9.95，与 Java 行为不符。
        let p = TimeDiscountService::calc_discount_price(dec("10.05"), 1, dec("9.9"));
        assert_eq!(p, dec("9.9495"));
    }

    #[test]
    fn rate_is_rounded_to_two_decimals_losing_precision() {
        // 忠实复刻 Java：折数先除以 10 并 HALF_UP 取 2 位小数。
        // 8.33 折 → 0.833 → 取 2 位 → 0.83（**精度损失**，与 Java 一致）
        // 100 × 0.83 = 83.00
        let p = TimeDiscountService::calc_discount_price(dec("100"), 1, dec("8.33"));
        assert_eq!(p, dec("83.00"));
    }

    #[test]
    fn rate_rounding_half_up_at_boundary() {
        // 8.35 折 → 0.835 → HALF_UP 取 2 位 → 0.84（进位）
        let p = TimeDiscountService::calc_discount_price(dec("100"), 1, dec("8.35"));
        assert_eq!(p, dec("84.00"));
    }

    #[test]
    fn reduce_discount_calculation() {
        // 立减 30：100 - 30 = 70
        assert_eq!(
            TimeDiscountService::calc_discount_price(dec("100"), 2, dec("30")),
            dec("70")
        );
    }

    #[test]
    fn reduce_never_below_zero() {
        // 立减超过原价 → 归零，不产生负价
        assert_eq!(
            TimeDiscountService::calc_discount_price(dec("10"), 2, dec("50")),
            Decimal::ZERO
        );
    }

    #[test]
    fn fixed_discount_takes_value() {
        assert_eq!(
            TimeDiscountService::calc_discount_price(dec("100"), 3, dec("9.90")),
            dec("9.90")
        );
    }

    #[test]
    fn fixed_price_capped_at_original() {
        // 固定价高于原价时不应抬价
        assert_eq!(
            TimeDiscountService::calc_discount_price(dec("5"), 3, dec("9.90")),
            dec("5")
        );
    }

    #[test]
    fn unknown_discount_type_leaves_price_unchanged() {
        // 未知类型保守处理：不改价，避免误打折
        assert_eq!(
            TimeDiscountService::calc_discount_price(dec("100"), 99, dec("1")),
            dec("100")
        );
    }

    #[test]
    fn applies_to_sku_empty_means_all() {
        // sku_ids 为空 → 作用于全部规格
        let it = item_with_skus(None);
        assert!(TimeDiscountService::applies_to_sku(&it, Some(7)));
        assert!(TimeDiscountService::applies_to_sku(&it, None));
    }

    #[test]
    fn applies_to_sku_restricted() {
        let it = item_with_skus(Some("[1,3]"));
        assert!(TimeDiscountService::applies_to_sku(&it, Some(1)));
        assert!(TimeDiscountService::applies_to_sku(&it, Some(3)));
        assert!(!TimeDiscountService::applies_to_sku(&it, Some(2)));
    }

    #[test]
    fn saved_amount_scales_by_quantity() {
        // 原价 100，折后 85，买 3 件 → 省 45
        assert_eq!(
            TimeDiscountService::calc_saved(dec("100"), dec("85"), 3, false),
            dec("45")
        );
    }

    #[test]
    fn saved_amount_zero_when_already_applied() {
        // 已按目标价成交时不再重复累计优惠
        assert_eq!(
            TimeDiscountService::calc_saved(dec("100"), dec("85"), 3, true),
            Decimal::ZERO
        );
    }

    #[test]
    fn saved_amount_never_negative() {
        // 折后价高于原价（异常数据）时优惠为 0
        assert_eq!(
            TimeDiscountService::calc_saved(dec("85"), dec("100"), 2, false),
            Decimal::ZERO
        );
    }

    #[test]
    fn collect_product_ids_prefers_explicit_list() {
        let dto = TimeDiscountCreateDTO {
            product_ids: vec![9, 9, 3],
            item: vec![TimeDiscountItemDTO {
                product_id: Some(7),
                ..Default::default()
            }],
            ..Default::default()
        };
        // 显式列表优先且去重排序
        assert_eq!(collect_product_ids(&dto), vec![3, 9]);
    }

    #[test]
    fn collect_product_ids_falls_back_to_items() {
        let dto = TimeDiscountCreateDTO {
            product_ids: vec![],
            item: vec![
                TimeDiscountItemDTO {
                    product_id: Some(5),
                    ..Default::default()
                },
                TimeDiscountItemDTO {
                    product_id: Some(2),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert_eq!(collect_product_ids(&dto), vec![2, 5]);
    }

    #[test]
    fn promotion_vo_builds_label() {
        let it = item_with_skus(Some("[1]"));
        let vo = to_promotion_vo(&it, 150);
        assert_eq!(vo.discount_label, "8.5折");
        assert!(vo.started);
        assert_eq!(vo.remain_seconds, 50);
        assert_eq!(vo.sku_ids, vec![1]);
    }

    #[test]
    fn promotion_vo_not_started_counts_to_start() {
        let it = item_with_skus(None);
        let vo = to_promotion_vo(&it, 40);
        assert!(!vo.started);
        assert_eq!(vo.remain_seconds, 60);
    }

    #[tokio::test]
    async fn tenant_scope_scopes_list_and_blocks_cross_shop_access() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        let shop = TenantScope::for_shop(8);
        let (filter, blocked) = with_tenant_scope(shop, async {
            (
                scoped_list_shop_id(None),
                TenantScope::current_ensure_access(9).is_err(),
            )
        })
        .await;
        assert_eq!(filter, Some(8));
        assert!(blocked);
        let filter = with_tenant_scope(TenantScope::platform(), async {
            scoped_list_shop_id(Some(9))
        })
        .await;
        assert_eq!(filter, Some(9));
    }
}