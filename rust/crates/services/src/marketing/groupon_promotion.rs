//! 拼团活动域服务（marketing::groupon_promotion）。
//!
//! 对应原 Java `com.nextrade.service.promotion.impl.GrouponPromotionServiceImpl`（912 行）。
//!
//! # 迁移要点（逐条对照 Java 源）
//!
//! - `create`（`@Transactional`）：校验 `@Valid` 必填 → 落 `groupon_promotion`
//!   → 同步写 `promotion` 主表（`type = 7`、`range = 3`、人群三字段）
//!   → 写 `groupon_promotion_shops`（适用店铺 JSON，恒含 `0` 商城自营）。
//! - `update`（`@Transactional`）：活动不存在 →「未知的拼团活动」；**过期活动重开**
//!   时校验其商品是否仍在其它未过期拼团活动中（文案
//!   `商品:{productName} 正在参与其他拼团活动:{name}`）→ 整行更新活动
//!   → 更新 `promotion` 人群三字段（**无条件 set**）→ 适用店铺 insert/update。
//! - `finish`：仅把活动与 promotion 的结束时间置为当前秒。
//! - `del`：活动不存在 → 报错；`now_millis > end_time` 才允许（Java 用毫秒
//!   与秒比较 → 该断言几乎恒真，本实现逐字复刻）→ 删活动 + 删 promotion 主表。
//! - `detail`：活动 + 适用店铺（附带虚拟店铺 `0 商城自营`）+ promotion 人群字段。
//! - `list`（自定义 SQL + 内存组装）：活动分页后按活动补成团记录统计
//!   （支付单数 / 应收金额 / 成团数量）与店铺名、状态。
//! - `promotionPromote`：分享推广信息（H5 域名 + 分享链接；短链/小程序码服务未迁移，见偏差）。
//! - `clientList` / `clientDetail` / `itemList` / `imageList` / `getGrouponInfo`：
//!   客户端视角，含「按会员标记过滤可见活动」与 `canJoin/restrictTip` 标记。
//!
//! # 未迁移 / 偏差（详见 `uncertain_points`）
//!
//! - `MemberAudienceService`（member_scope 编解码 / 匹配 / 旧结构转换）未迁移：
//!   `sendUserTarget`/`memberAudience` 原样 JSON 存入 `promotion.member_scope`，
//!   detail 原样返回 `sendUserTarget`，`memberAudience` 恒 null；`matchesScope`
//!   未迁移 → member_scope 非空时 `canJoin` 视为 true（仅旧 memberRange/memberMarks 生效）。
//! - `ShortUrlService` / `WechatOAuthService` 未迁移：推广接口的短链退化为
//!   直接链接（H5 域名 + 页面路径），`miniCode` 返回 null。
//! - `WholesalePromotionFilter`（批发商过滤拼团商品）未迁移 → 不做该过滤。
//! - `ProductSkuService` 真实 SKU 信息未加入 `GrouponItemVO`（Java 该 VO 也不含 SKU）。
//! - `Product` / `Shop` 实体出参：Java 直接返回实体（80+/50+ 字段，camelCase）；
//!   本实现仅输出前端所需字段（见模型 `GrouponShopVO` / `ProductRow`）。
//! - 时区：入参时间按 `chrono::Local`（东八区运行环境）解析，与 Java 默认时区一致。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_common::utils::now_seconds;
use nextrade_core::entities::{GrouponPromotion, GrouponPromotionShops, Promotion};
use nextrade_core::models::marketing::groupon_promotion::{
    admin_status, client_status, format_date, format_time, other_groupon_conflict_message,
    page_query, parse_datetime, parse_group_nums, parse_group_prices, parse_shop_ids,
    promotion_sort_column, serialize_shop_ids, sort_is_asc, status_name, validate_create_param,
    GrouponItemInfoVO, GrouponItemPageQuery, GrouponItemVO, GrouponPostVO,
    GrouponPromotionClientDetailVO, GrouponPromotionClientPageQuery, GrouponPromotionClientPageVO,
    GrouponPromotionDetailVO, GrouponPromotionImageVO, GrouponPromotionPageQuery,
    GrouponPromotionPageVO, GrouponShopVO, OperateDTO, PromotionPromotionCreateParam,
    PromotionPromotionUpdateParam, CONFIG_H5_DOMAIN, MSG_GROUPON_NOT_EXIST, MSG_MEMBER_ONLY,
    MSG_MEMBER_ONLY_TIP, MSG_NOT_ENDED_CANNOT_DELETE, MSG_NOT_IN_SCOPE_TIP,
    MSG_UNKNOWN_PROMOTION, PROMOTION_TYPE_GROUPON, PROMOTION_TYPE_GROUPON_DESC,
    PROMOTION_RANGE_PRODUCT,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::groupon_promotion::{
    AdminPromotionListFilter, ClientPromotionListFilter, GrouponItemListFilter, ProductRow,
};
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 「商城自营」虚拟店铺（Java `Shop.builder().shopId(0).shopTitle("商城自营")`）。
const SELF_OPERATED_SHOP_TITLE: &str = "商城自营";

/// 拼团活动域服务。
pub struct GrouponPromotionService {
    repos: Arc<Repositories>,
}

impl GrouponPromotionService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &nextrade_infrastructure::db::repo::marketing::groupon_promotion::GrouponPromotionRepository {
        &self.repos.marketing.groupon_promotion
    }

    // ————————————————————— 管理端：增删改 —————————————————————

    /// 创建拼团活动（Java `create`，三表事务）。
    pub async fn create(
        &self,
        param: &PromotionPromotionCreateParam,
        shop_id: Option<i32>,
    ) -> Result<()> {
        validate_create_param(param)?;
        // 多租户：店铺账号强制本店；平台账号可指定/落 0
        let shop_id = TenantScope::current().write_shop_id(shop_id)?;
        let mutation = self.build_promotion(param, Some(shop_id))?;
        let repo = self.repo();
        let mut tx = repo.begin().await?;
        let promotion_id = repo.insert_promotion_in_tx(&mut tx, &mutation).await?;

        // Java `savePromotion`
        let row = self.build_promotion_row(&mutation, param, promotion_id)?;
        repo.insert_promotion_row_in_tx(&mut tx, &row).await?;

        // Java `buildGrouponPromotionShop` + 绑定：shopIds 恒追加 0（商城自营）
        let binding = self.build_shop_binding(param, promotion_id);
        repo.insert_shop_binding_in_tx(&mut tx, &binding).await?;
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        tracing::info!("拼团活动创建成功: id={promotion_id}, name={}", mutation.name);
        Ok(())
    }

    /// 更新拼团活动（Java `update`，三表事务）。
    pub async fn update(&self, param: &PromotionPromotionUpdateParam) -> Result<()> {
        validate_create_param(&param.base)?;
        let Some(id) = param.groupon_promotion_id else {
            return Err(Error::biz_msg(MSG_UNKNOWN_PROMOTION));
        };
        let repo = self.repo();
        let db = repo
            .find_promotion(id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_UNKNOWN_PROMOTION))?;
        // 多租户：更新前校验归属
        TenantScope::current_ensure_access(db.shop_id.unwrap_or(0))?;
        let mutation = self.build_promotion_from(&param.base, db.shop_id)?;

        // 过期活动重开：校验商品是否仍在其它进行中拼团
        self.check_reopened_conflicts(&db, &mutation, &param.base)
            .await?;

        let mut tx = repo.begin().await?;
        repo.update_promotion_in_tx(&mut tx, &mutation).await?;
        // Java `updatePromotion`：无条件覆盖人群三字段
        let row = self.build_promotion_row(&mutation, &param.base, id)?;
        repo.update_promotion_row_in_tx(&mut tx, id, &row).await?;

        let binding = self.build_shop_binding(&param.base, id);
        if repo.count_shop_binding_in_tx(&mut tx, id).await? < 1 {
            repo.insert_shop_binding_in_tx(&mut tx, &binding).await?;
        } else {
            repo.update_shop_binding_in_tx(&mut tx, &binding).await?;
        }
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        tracing::info!("拼团活动更新成功: id={id}");
        Ok(())
    }

    /// 立即结束拼团活动（Java `finish`）。
    pub async fn finish(&self, dto: &OperateDTO) -> Result<()> {
        let Some(id) = dto.id else {
            return Err(Error::biz_msg(MSG_UNKNOWN_PROMOTION));
        };
        let repo = self.repo();
        let db = repo
            .find_promotion(id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_UNKNOWN_PROMOTION))?;
        // 多租户：结束活动前校验归属
        TenantScope::current_ensure_access(db.shop_id.unwrap_or(0))?;
        let now = now_i64();
        let mut tx = repo.begin().await?;
        repo.update_promotion_end_time(id, now).await?;
        repo.update_promotion_row_end_in_tx(&mut tx, db.groupon_promotion_id, now as i32)
            .await?;
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    /// 删除拼团活动（Java `del`）。
    ///
    /// ⚠️ Java 用 `Instant.now().toEpochMilli() > end_time`（毫秒 vs 秒），
    /// 实际几乎恒真；本实现逐字复刻该比较。
    pub async fn del(&self, id: Option<i64>) -> Result<()> {
        let Some(id) = id else {
            return Err(Error::biz_msg(MSG_UNKNOWN_PROMOTION));
        };
        let repo = self.repo();
        let db = repo
            .find_promotion(id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_UNKNOWN_PROMOTION))?;
        // 多租户：删除前校验归属
        TenantScope::current_ensure_access(db.shop_id.unwrap_or(0))?;
        if now_millis() <= db.end_time {
            return Err(Error::biz_msg(MSG_NOT_ENDED_CANNOT_DELETE));
        }
        let mut tx = repo.begin().await?;
        repo.delete_promotion_in_tx(&mut tx, id).await?;
        repo.delete_promotion_row_in_tx(&mut tx, id).await?;
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    // ————————————————————— 管理端：查询 —————————————————————

    /// 拼团活动详情（Java `detail`）。
    pub async fn detail(&self, id: i64) -> Result<GrouponPromotionDetailVO> {
        let repo = self.repo();
        let db = repo
            .find_promotion(id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_UNKNOWN_PROMOTION))?;
        // 多租户：详情归属校验
        TenantScope::current_ensure_access(db.shop_id.unwrap_or(0))?;
        let mut vo = GrouponPromotionDetailVO {
            groupon_promotion_id: db.groupon_promotion_id,
            name: db.name.clone(),
            group_type: Some(db.group_type),
            start_time: format_time(db.start_time),
            end_time: format_time(db.end_time),
            expiration_time: Some(db.expiration_time),
            groupon_image: db.groupon_image.clone(),
            pc_groupon_image: db.pc_groupon_image.clone(),
            share_image: db.share_image.clone(),
            groupon_explain: db.groupon_explain.clone(),
            ..Default::default()
        };

        if let Some(binding) = repo.find_shop_binding(id).await? {
            let ids = parse_shop_ids(binding.shop_ids.as_deref());
            let shops = repo.find_shops(&ids).await?;
            let mut list: Vec<GrouponShopVO> = shops
                .iter()
                .map(|shop| GrouponShopVO {
                    shop_id: shop.shop_id,
                    shop_title: shop.shop_title.clone(),
                    shop_logo: shop.shop_logo.clone(),
                    status: shop.status,
                    merchant_id: shop.merchant_id,
                })
                .collect();
            // Java 恒追加虚拟店铺 0 商城自营
            list.push(GrouponShopVO {
                shop_id: 0,
                shop_title: Some(SELF_OPERATED_SHOP_TITLE.to_string()),
                ..Default::default()
            });
            vo.shops = Some(list);
        }

        if let Some(row) = repo.find_promotion_row(id).await? {
            vo.member_range = Some(row.member_range);
            vo.member_marks = Some(parse_int_list(row.member_marks.as_deref()));
            // Java `parseScope` / `fillTargetUsers` / `toLegacy`：人群服务未迁移 → 原样/空
            vo.send_user_target = row
                .member_scope
                .as_deref()
                .and_then(|raw| serde_json::from_str(raw).ok());
            vo.member_audience = None;
        }
        Ok(vo)
    }

    /// 管理端拼团分页（Java `list`：分页后内存组装统计）。
    pub async fn list(
        &self,
        q: &GrouponPromotionPageQuery,
        header_shop_id: Option<i32>,
    ) -> Result<PageResult<GrouponPromotionPageVO>> {
        let (page, size) = nextrade_core::models::marketing::groupon_promotion::page_params(q.page, q.size);
        let pq = page_query(q.page, q.size);
        let now = now_i64();
        let filter = AdminPromotionListFilter {
            groupon_name: q.groupon_name.clone(),
            product_name: q.product_name.clone(),
            product_sn: q.product_sn.clone(),
            // Java `queryData()`：shopId 缺省取请求头
            // 多租户：店铺账号强制本店；平台账号可按请求店铺过滤
        shop_id: TenantScope::current_list_filter().or(q.shop_id.or(header_shop_id)),
            status: q.status,
            search_status_time: now,
            start_time: q.start_time.as_deref().and_then(parse_datetime),
            end_time: q.end_time.as_deref().and_then(parse_datetime),
            group_type: q.group_type,
            sort_column: promotion_sort_column(q.sort_field.as_deref()),
            sort_asc: sort_is_asc(q.sort_order.as_deref()),
            page,
            size,
        };
        let repo = self.repo();
        let (total, rows) = repo.paginate_promotions(&filter).await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&pq));
        }
        let promotion_ids: Vec<i64> = rows.iter().map(|r| r.groupon_promotion_id).collect();
        let records = repo.find_records_by_promotion_ids(&promotion_ids).await?;
        let shop_ids: Vec<i32> = rows.iter().filter_map(|r| r.shop_id).collect();
        let shop_titles: HashMap<i32, Option<String>> = repo
            .find_shop_titles(&shop_ids)
            .await?
            .into_iter()
            .collect();

        let list = rows
            .iter()
            .map(|row| {
                let current: Vec<&nextrade_core::entities::GrouponPromotionRecord> = records
                    .iter()
                    .filter(|r| r.groupon_promotion_id == row.groupon_promotion_id)
                    .collect();
                let success: Vec<&&nextrade_core::entities::GrouponPromotionRecord> =
                    current.iter().filter(|r| r.join_status == 2).collect();
                let receive_amount = success
                    .iter()
                    .fold(Decimal::ZERO, |acc, r| acc + r.paid_amount);
                let group_success_count = success.iter().filter(|r| r.join_type == 1).count();
                GrouponPromotionPageVO {
                    groupon_promotion_id: row.groupon_promotion_id,
                    name: row.name.clone(),
                    start_time: format_time(row.start_time),
                    end_time: format_time(row.end_time),
                    group_type: Some(row.group_type),
                    shop_id: row.shop_id,
                    status: Some(admin_status(row.start_time, row.end_time, now)),
                    status_name: Some(status_name(row.start_time, row.end_time, now).to_string()),
                    shop_name: row.shop_id.and_then(|id| shop_titles.get(&id).cloned().flatten()),
                    receive_amount,
                    group_success_count: i64::try_from(group_success_count).unwrap_or(0),
                    pay_order_count: i64::try_from(current.len()).unwrap_or(0),
                }
            })
            .collect();
        Ok(PageResult::new(total, list, &pq))
    }

    /// 活动推广信息（Java `promotionPromote`）。
    ///
    /// 短链 / 小程序码服务未迁移：`shortLink`/`wechatShortLink` 退化为直接链接，
    /// `miniCode` 为 null（见模块注释）。
    pub async fn post(&self, id: i64) -> Result<GrouponPostVO> {
        let repo = self.repo();
        let db = repo
            .find_promotion(id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_UNKNOWN_PROMOTION))?;
        let h5_domain = repo
            .find_config_value(CONFIG_H5_DOMAIN)
            .await?
            .unwrap_or_default();
        let share_link = format!("/pages/groupon/list?id={id}");
        Ok(GrouponPostVO {
            promotion_name: db.name.clone(),
            promotion_type: PROMOTION_TYPE_GROUPON_DESC.to_string(),
            promotion_pic_url: db.share_image.clone(),
            wechat_short_link: Some(format!("{h5_domain}{share_link}")),
            short_link: Some(format!("{h5_domain}{share_link}")),
            original_link: Some(format!("{h5_domain}{share_link}")),
            promotion_brief: db.groupon_explain.clone(),
            promotion_date_time: format!(
                "{} - {}",
                format_date(db.start_time),
                format_date(db.end_time)
            ),
            mini_code: None,
        })
    }

    /// 客户端拼团分页（Java `clientList`）。
    pub async fn client_list(
        &self,
        q: &GrouponPromotionClientPageQuery,
    ) -> Result<PageResult<GrouponPromotionClientPageVO>> {
        let (page, size) = nextrade_core::models::marketing::groupon_promotion::page_params(q.page, q.size);
        let pq = page_query(q.page, q.size);
        let now = now_i64();
        let filter = ClientPromotionListFilter {
            groupon_name: q.groupon_name.clone(),
            status: q.status,
            search_status_time: now,
            sort_column: promotion_sort_column(q.sort_field.as_deref()),
            sort_asc: sort_is_asc(q.sort_order.as_deref()),
            page,
            size,
        };
        let repo = self.repo();
        let (total, rows) = repo.paginate_client_promotions(&filter).await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&pq));
        }
        let shop_ids: Vec<i32> = rows.iter().filter_map(|r| r.shop_id).collect();
        let shop_titles: HashMap<i32, Option<String>> = repo
            .find_shop_titles(&shop_ids)
            .await?
            .into_iter()
            .collect();
        let list = rows
            .iter()
            .map(|row| GrouponPromotionClientPageVO {
                groupon_promotion_id: row.groupon_promotion_id,
                name: row.name.clone(),
                start_time: row.start_time,
                end_time: row.end_time,
                group_type: Some(row.group_type),
                shop_id: row.shop_id,
                start_time_str: format_time(row.start_time),
                end_time_str: format_time(row.end_time),
                status: Some(client_status(row.start_time, row.end_time, now)),
                status_name: Some(status_name(row.start_time, row.end_time, now).to_string()),
                shop_name: row.shop_id.and_then(|id| shop_titles.get(&id).cloned().flatten()),
            })
            .collect();
        Ok(PageResult::new(total, list, &pq))
    }

    // ————————————————————— 客户端 —————————————————————

    /// 客户端拼团详情（Java `clientDetail`：不存在抛「拼团不存在」）。
    pub async fn client_detail(
        &self,
        id: i64,
        user_id: Option<i32>,
    ) -> Result<GrouponPromotionClientDetailVO> {
        let repo = self.repo();
        let groupon = repo
            .find_promotion(id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_GROUPON_NOT_EXIST))?;
        let mut vo = GrouponPromotionClientDetailVO {
            groupon_promotion_id: groupon.groupon_promotion_id,
            groupon_image: groupon.groupon_image.clone(),
            pc_groupon_image: groupon.pc_groupon_image.clone(),
            start_time: format_time(groupon.start_time),
            end_time: format_time(groupon.end_time),
            groupon_explain: groupon.groupon_explain.clone(),
            shop_id: groupon.shop_id,
            ..Default::default()
        };
        if let Some(shop_id) = groupon.shop_id {
            if let Some(shop) = repo.find_shop(shop_id).await? {
                vo.shop_title = shop.shop_title;
            }
        }
        // Java：promotion.memberScope 非空 → 标记 canJoin/restrictTip；否则旧 memberRange
        // 不匹配直接抛「该拼团活动仅限指定会员参与」
        let row = repo.find_promotion_row(id).await?;
        let scope_non_blank = row
            .as_ref()
            .and_then(|r| r.member_scope.as_deref())
            .map(str::trim)
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        if scope_non_blank {
            let can_join = self.can_user_join(id, user_id).await?;
            vo.can_join = Some(can_join);
            if !can_join {
                vo.restrict_tip = Some(MSG_NOT_IN_SCOPE_TIP.to_string());
            }
        } else {
            if !self.can_user_join(id, user_id).await? {
                return Err(Error::biz_msg(MSG_MEMBER_ONLY));
            }
            vo.can_join = Some(true);
        }
        Ok(vo)
    }

    /// 客户端拼团商品分页（Java `itemList`）。
    pub async fn item_list(
        &self,
        q: &GrouponItemPageQuery,
        user_id: Option<i32>,
    ) -> Result<PageResult<GrouponItemVO>> {
        let (page, size) = nextrade_core::models::marketing::groupon_promotion::page_params(q.page, q.size);
        let pq = page_query(q.page, q.size);
        let now = now_i64();
        let repo = self.repo();
        let allowed_ids = self
            .allowed_groupon_ids(q.groupon_promotion_id, user_id)
            .await?;
        if allowed_ids.is_empty() {
            return Ok(PageResult::empty(&pq));
        }
        let filter = GrouponItemListFilter {
            allowed_ids,
            keyword: q.keyword.clone(),
            sort_field: q.sort_field.clone(),
            sort_order: q.sort_order.clone(),
            page,
            size,
        };
        let (total, rows) = repo.paginate_items(&filter, now).await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&pq));
        }

        let product_ids: Vec<i32> = rows
            .iter()
            .map(|r| i32::try_from(r.product_id).unwrap_or(i32::MAX))
            .collect();
        let products: HashMap<i32, ProductRow> = repo
            .find_products(&product_ids)
            .await?
            .into_iter()
            .map(|p| (p.product_id, p))
            .collect();
        let item_ids: Vec<i64> = rows.iter().map(|r| r.groupon_promotion_item_id).collect();
        let details = repo.find_details_by_item_ids(&item_ids).await?;
        let records = repo.find_records_by_item_ids(&item_ids).await?;
        let promotion_ids: Vec<i64> = {
            let mut ids: Vec<i64> = rows.iter().map(|r| r.groupon_promotion_id).collect();
            ids.sort_unstable();
            ids.dedup();
            ids
        };
        let promotions: HashMap<i64, GrouponPromotion> = repo
            .find_promotions_by_ids(&promotion_ids)
            .await?
            .into_iter()
            .map(|p| (p.groupon_promotion_id, p))
            .collect();

        let mut list = Vec::with_capacity(rows.len());
        for item in &rows {
            let product = products.get(&i32::try_from(item.product_id).unwrap_or(i32::MAX));
            let curr_records: Vec<&nextrade_core::entities::GrouponPromotionRecord> = records
                .iter()
                .filter(|r| r.groupon_promotion_item_id == item.groupon_promotion_item_id)
                .collect();
            let curr_details: Vec<&nextrade_core::entities::GrouponPromotionDetail> = details
                .iter()
                .filter(|d| d.groupon_promotion_item_id == item.groupon_promotion_item_id)
                .collect();
            let promotion = promotions.get(&item.groupon_promotion_id);
            let min_price = min_detail_price(&curr_details);
            let can_join = self.can_user_join(item.groupon_promotion_id, user_id).await?;
            list.push(GrouponItemVO {
                groupon_promotion_item_id: item.groupon_promotion_item_id,
                product_id: item.product_id,
                min_groupon_price: min_price,
                min_group_num: Some(item.min_group_num),
                product_name: product.map(|p| p.product_name.clone()),
                product_sn: product.map(|p| p.product_sn.clone()),
                product_brief: product.map(|p| p.product_brief.clone()),
                product_price: product.map(|p| p.product_price),
                pic_thumb: product.map(|p| p.pic_thumb.clone()),
                pic_url: product.map(|p| p.pic_url.clone()),
                join_num_count: i64::try_from(curr_records.len()).unwrap_or(0),
                group_num: Some(item.group_num.clone()),
                start_time: promotion
                    .map(|p| format_time(p.start_time))
                    .unwrap_or_default(),
                end_time: promotion
                    .map(|p| format_time(p.end_time))
                    .unwrap_or_default(),
                can_join: Some(can_join),
                restrict_tip: (!can_join).then(|| MSG_MEMBER_ONLY_TIP.to_string()),
            });
        }
        Ok(PageResult::new(total, list, &pq))
    }

    /// 拼团图片列表（Java `grouponPromotionImages`）。
    pub async fn image_list(
        &self,
        page: Option<u64>,
        size: Option<u64>,
        user_id: Option<i32>,
    ) -> Result<PageResult<GrouponPromotionImageVO>> {
        let (page_no, page_size) = nextrade_core::models::marketing::groupon_promotion::page_params(page, size);
        let pq = page_query(page, size);
        let now = now_i64();
        let repo = self.repo();
        let (total, rows) = repo.paginate_ongoing_promotions(now, page_no, page_size).await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&pq));
        }
        let allowed_ids = self.filter_by_user_mark(&rows, user_id).await?;
        let list: Vec<GrouponPromotionImageVO> = rows
            .iter()
            .filter(|row| allowed_ids.contains(&row.groupon_promotion_id))
            .map(|row| GrouponPromotionImageVO {
                groupon_promotion_id: row.groupon_promotion_id,
                name: row.name.clone(),
                group_type: Some(row.group_type),
                groupon_image: row.groupon_image.clone(),
                pc_groupon_image: row.pc_groupon_image.clone(),
                share_image: row.share_image.clone(),
                shop_id: row.shop_id,
                add_time: row.add_time,
            })
            .collect();
        // Java `PageUtil.convertPage(grouponPage, voList)`：total 仍取分页 total
        Ok(PageResult::new(total, list, &pq))
    }

    /// 商品 id 获取拼团活动信息（Java `getGrouponInfo`；无命中返回 null）。
    pub async fn get_groupon_info(
        &self,
        product_id: i32,
        user_id: Option<i32>,
    ) -> Result<Option<GrouponItemInfoVO>> {
        let now = now_i64();
        let repo = self.repo();
        let promotions = repo.list_active_promotions(now).await?;
        if promotions.is_empty() {
            return Ok(None);
        }
        let filtered_ids = self.filter_by_user_mark(&promotions, user_id).await?;
        if filtered_ids.is_empty() {
            return Ok(None);
        }
        let Some(item) = repo
            .find_item_by_product(product_id, &filtered_ids)
            .await?
        else {
            return Ok(None);
        };

        let mut details = repo
            .find_details_by_item_ids(&[item.groupon_promotion_item_id])
            .await?;
        let enabled_skus: HashSet<i64> = repo
            .find_enabled_sku_ids(product_id)
            .await?
            .into_iter()
            .collect();
        details.retain(|d| d.sku_id == 0 || enabled_skus.contains(&d.sku_id));
        if details.is_empty() {
            return Ok(None);
        }

        let promotion = promotions
            .iter()
            .find(|p| p.groupon_promotion_id == item.groupon_promotion_id)
            .cloned();
        let min_price = min_detail_price(&details.iter().collect::<Vec<_>>());
        let group_nums = parse_group_nums(&item.group_num);
        let group_success_count = repo
            .count_open_success_records(item.groupon_promotion_id, product_id)
            .await?;
        let join_num_count = repo.count_wait_records_by_product(product_id).await?;
        let product = repo.find_product(product_id).await?;
        let can_join = self.can_user_join(item.groupon_promotion_id, user_id).await?;

        Ok(Some(GrouponItemInfoVO {
            groupon_promotion_item_id: item.groupon_promotion_item_id,
            groupon_promotion_id: item.groupon_promotion_id,
            product_id: item.product_id,
            product_sn: product.as_ref().map(|p| p.product_sn.clone()),
            group_type: Some(item.group_type),
            group_nums: Some(group_nums),
            order_limit_num: Some(item.order_limit_num),
            is_auto_group: Some(item.is_auto_group),
            min_group_num: Some(item.min_group_num),
            name: promotion
                .as_ref()
                .map(|p| p.name.clone())
                .unwrap_or_default(),
            start_time: promotion
                .as_ref()
                .map(|p| format_time(p.start_time))
                .unwrap_or_default(),
            end_time: promotion
                .as_ref()
                .map(|p| format_time(p.end_time))
                .unwrap_or_default(),
            expiration_time: promotion.as_ref().map(|p| p.expiration_time),
            groupon_image: promotion.as_ref().and_then(|p| p.groupon_image.clone()),
            pc_groupon_image: promotion
                .as_ref()
                .and_then(|p| p.pc_groupon_image.clone()),
            share_image: promotion.as_ref().and_then(|p| p.share_image.clone()),
            groupon_explain: promotion
                .as_ref()
                .and_then(|p| p.groupon_explain.clone()),
            shop_id: promotion.as_ref().and_then(|p| p.shop_id),
            min_groupon_price: min_price,
            group_success_count,
            join_num_count,
            groupon_promotion_details: Some(
                details
                    .iter()
                    .map(|d| nextrade_core::models::marketing::groupon_promotion::GrouponPromotionMemberDetailVO {
                        groupon_detail_id: d.groupon_detail_id,
                        groupon_promotion_id: d.groupon_promotion_id,
                        groupon_promotion_item_id: d.groupon_promotion_item_id,
                        product_id: d.product_id,
                        sku_id: d.sku_id,
                        group_price: d.group_price.clone(),
                        limit_num: d.limit_num,
                        is_join: d.is_join,
                        add_time: d.add_time,
                    })
                    .collect(),
            ),
            can_join: Some(can_join),
            restrict_tip: (!can_join).then(|| MSG_MEMBER_ONLY_TIP.to_string()),
        }))
    }

    // ————————————————————— 内部：实体构造 —————————————————————

    fn build_promotion(
        &self,
        p: &PromotionPromotionCreateParam,
        shop_id: Option<i32>,
    ) -> Result<GrouponPromotion> {
        self.build_promotion_from(p, shop_id)
    }

    fn build_promotion_from(
        &self,
        p: &PromotionPromotionCreateParam,
        shop_id: Option<i32>,
    ) -> Result<GrouponPromotion> {
        let start_time = p
            .start_time
            .as_deref()
            .and_then(parse_datetime)
            .ok_or_else(|| Error::Validation("开始时间格式不正确".into()))?;
        let end_time = p
            .end_time
            .as_deref()
            .and_then(parse_datetime)
            .ok_or_else(|| Error::Validation("结束时间格式不正确".into()))?;
        Ok(GrouponPromotion {
            groupon_promotion_id: 0,
            name: p.name.clone().unwrap_or_default(),
            start_time,
            end_time,
            group_type: p.group_type.unwrap_or(0),
            expiration_time: p.expiration_time.unwrap_or(0),
            groupon_image: p.groupon_image.clone(),
            pc_groupon_image: p.pc_groupon_image.clone(),
            share_image: p.share_image.clone(),
            groupon_explain: p.groupon_explain.clone(),
            shop_id,
            add_time: now_i64(),
        })
    }

    /// 构造 `promotion` 主表行（Java `savePromotion` / `updatePromotion`）。
    fn build_promotion_row(
        &self,
        promotion: &GrouponPromotion,
        p: &PromotionPromotionCreateParam,
        promotion_id: i64,
    ) -> Result<Promotion> {
        Ok(Promotion {
            promotion_id: 0,
            promotion_name: promotion.name.clone(),
            start_time: i32::try_from(promotion.start_time).unwrap_or(i32::MAX),
            end_time: i32::try_from(promotion.end_time).unwrap_or(i32::MAX),
            r#type: PROMOTION_TYPE_GROUPON,
            shop_id: promotion.shop_id.unwrap_or(0),
            relation_id: Some(i32::try_from(promotion_id).unwrap_or(i32::MAX)),
            range: Some(PROMOTION_RANGE_PRODUCT),
            member_range: p.member_range.unwrap_or(0),
            member_marks: p.member_marks.as_deref().map(java_marks_string),
            member_scope: audience_scope_json(p),
            ..Default::default()
        })
    }

    /// 构造适用店铺绑定（Java `buildGrouponPromotionShop`：shopIds 恒追加 0）。
    fn build_shop_binding(
        &self,
        p: &PromotionPromotionCreateParam,
        promotion_id: i64,
    ) -> GrouponPromotionShops {
        let mut ids: Vec<i32> = p.shop_ids.clone().unwrap_or_default();
        ids.push(0);
        GrouponPromotionShops {
            groupon_shop_id: 0,
            groupon_promotion_id: Some(i32::try_from(promotion_id).unwrap_or(i32::MAX)),
            shop_ids: Some(serialize_shop_ids(&ids)),
        }
    }

    // ————————————————————— 内部：活动重开冲突 —————————————————————

    /// Java `getBindedOtherGtoupPromotionProduct`：
    /// 过期活动重开时，若其商品仍绑定在其它未过期拼团活动 → 报错。
    async fn check_reopened_conflicts(
        &self,
        db: &GrouponPromotion,
        mutation: &GrouponPromotion,
        _p: &PromotionPromotionCreateParam,
    ) -> Result<()> {
        let now = now_i64();
        if db.end_time >= now
            || mutation.end_time == db.end_time
            || mutation.end_time <= now
        {
            return Ok(());
        }
        let repo = self.repo();
        let products = repo.find_details_by_promotion(db.groupon_promotion_id).await?;
        if products.is_empty() {
            return Ok(());
        }
        let mut other_ids: HashSet<i64> = HashSet::new();
        let mut other_details: Vec<nextrade_core::entities::GrouponPromotionDetail> = Vec::new();
        for product in &products {
            let others = repo
                .find_details_by_product(product.product_id, db.groupon_promotion_id)
                .await?;
            for other in others {
                other_ids.insert(other.groupon_promotion_id);
                other_details.push(other);
            }
        }
        if other_details.is_empty() {
            return Ok(());
        }
        let ids: Vec<i64> = other_ids.into_iter().collect();
        let promotions = repo.find_promotions_by_ids(&ids).await?;
        let promotion_map: HashMap<i64, GrouponPromotion> = promotions
            .into_iter()
            .filter(|p| {
                p.shop_id == db.shop_id
                    && p.start_time <= mutation.end_time
                    && p.end_time >= mutation.start_time
            })
            .map(|p| (p.groupon_promotion_id, p))
            .collect();
        let mut message = String::new();
        for detail in &other_details {
            if let Some(other) = promotion_map.get(&detail.groupon_promotion_id) {
                if other.end_time > now {
                    let product = repo
                        .find_product(i32::try_from(detail.product_id).unwrap_or(i32::MAX))
                        .await?;
                    if let Some(product) = product {
                        message.push_str(&other_groupon_conflict_message(
                            &product.product_name,
                            &other.name,
                        ));
                    }
                }
            }
        }
        if message.trim().is_empty() {
            return Ok(());
        }
        Err(Error::biz_msg(message))
    }

    // ————————————————————— 内部：会员标记可见性 —————————————————————

    /// 当前用户可参与的拼团活动 ID（Java `getAllowedGrouponIds`）。
    async fn allowed_groupon_ids(
        &self,
        specified: Option<i64>,
        user_id: Option<i32>,
    ) -> Result<Vec<i64>> {
        let now = now_i64();
        let repo = self.repo();
        let promotions = match specified {
            Some(id) => match repo.find_promotion(id).await? {
                Some(promotion) if promotion.end_time >= now => vec![promotion],
                _ => return Ok(Vec::new()),
            },
            None => repo.list_not_ended_promotions(now).await?,
        };
        if promotions.is_empty() {
            return Ok(Vec::new());
        }
        self.filter_by_user_mark(&promotions, user_id).await
    }

    /// Java `filterGrouponByUserMark`：按用户标记过滤可见活动。
    async fn filter_by_user_mark(
        &self,
        promotions: &[GrouponPromotion],
        user_id: Option<i32>,
    ) -> Result<Vec<i64>> {
        if promotions.is_empty() {
            return Ok(Vec::new());
        }
        let repo = self.repo();
        let ids: Vec<i64> = promotions.iter().map(|p| p.groupon_promotion_id).collect();
        let rows = repo.find_promotion_rows(&ids).await?;
        let row_map: HashMap<i64, Promotion> = rows
            .into_iter()
            .filter_map(|row| row.relation_id.map(|rid| (i64::from(rid), row)))
            .collect();
        let user_mark = match user_id.filter(|id| *id > 0) {
            Some(id) => repo.find_user_mark(id).await?,
            None => None,
        };
        match user_mark {
            None => {
                // 未登录/查不到用户：仅隐藏「旧 memberRange=1 且无 member_scope」的活动
                Ok(ids
                    .into_iter()
                    .filter(|id| {
                        let restricted = row_map.get(id).map(|row| {
                            let scope_blank =
                                row.member_scope.as_deref().map(str::trim).unwrap_or("").is_empty();
                            scope_blank && row.member_range == 1
                        });
                        !restricted.unwrap_or(false)
                    })
                    .collect())
            }
            Some(mark) => Ok(ids
                .into_iter()
                .filter(|id| match row_map.get(id) {
                    None => true,
                    Some(row) => {
                        let scope_non_blank = row
                            .member_scope
                            .as_deref()
                            .map(str::trim)
                            .map(|s| !s.is_empty())
                            .unwrap_or(false);
                        if scope_non_blank {
                            return true;
                        }
                        if row.member_range != 1 {
                            return true;
                        }
                        let allowed = parse_int_list(row.member_marks.as_deref());
                        allowed.is_empty() || allowed.contains(&mark)
                    }
                })
                .collect()),
        }
    }

    /// 当前用户是否可参与指定拼团（Java `canUserJoinGroupon`）。
    async fn can_user_join(&self, promotion_id: i64, user_id: Option<i32>) -> Result<bool> {
        let Some(user_id) = user_id.filter(|id| *id > 0) else {
            return Ok(true);
        };
        let repo = self.repo();
        let Some(mark) = repo.find_user_mark(user_id).await? else {
            return Ok(true);
        };
        let Some(row) = repo.find_promotion_row(promotion_id).await? else {
            return Ok(true);
        };
        let scope_non_blank = row
            .member_scope
            .as_deref()
            .map(str::trim)
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        if scope_non_blank {
            // Java `memberAudienceService.matchesScope` 未迁移 → 视为可参与（见模块注释）
            return Ok(true);
        }
        if row.member_range == 1 {
            let allowed = parse_int_list(row.member_marks.as_deref());
            return Ok(allowed.is_empty() || allowed.contains(&mark));
        }
        Ok(true)
    }
}

// ————————————————————— 模块内工具 —————————————————————

/// 当前秒级时间戳。
fn now_i64() -> i64 {
    now_seconds()
}

/// 当前毫秒时间戳（Java `Instant.now().toEpochMilli()`）。
fn now_millis() -> i64 {
    nextrade_common::utils::now_millis()
}

/// Java `List<Integer>.toString()`（`"[1, 2, 3]"`）；空列表 → `"[]"`。
fn java_marks_string(marks: &[i32]) -> String {
    let body = marks
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{body}]")
}

/// 解析 Java `List.toString()` / JSON 数组文本为整数列表
/// （Java `StringUtils.str2IntList` / `JSON.parseArray`）。
fn parse_int_list(raw: Option<&str>) -> Vec<i32> {
    let Some(raw) = raw else {
        return Vec::new();
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if let Ok(list) = serde_json::from_str::<Vec<i32>>(trimmed) {
        return list;
    }
    trimmed
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .filter_map(|part| part.trim().parse::<i32>().ok())
        .collect()
}

/// `promotion.member_scope` 落库值。
///
/// Java `memberAudienceService.resolveWriteScope(sendUserTarget, memberAudience)`：
/// 优先新结构 `sendUserTarget`，回退历史 `memberAudience`，均无 → null。
/// 人群服务未迁移 → 原样序列化 JSON（见模块注释）。
fn audience_scope_json(p: &PromotionPromotionCreateParam) -> Option<String> {
    p.send_user_target
        .as_ref()
        .or(p.member_audience.as_ref())
        .map(|value| value.to_string())
}

/// 拼团明细最低拼团价（Java `flatMap(parseArray(groupPrice)).map(price).min()`）。
///
/// Java 在无明细/无价格时 `.get()` 抛 `NoSuchElementException`；本实现返回 `None`
/// （列表 VO 的 `minGrouponPrice` 为 null），见 `uncertain_points`。
fn min_detail_price(details: &[&nextrade_core::entities::GrouponPromotionDetail]) -> Option<Decimal> {
    details
        .iter()
        .flat_map(|detail| parse_group_prices(&detail.group_price))
        .filter_map(|price| price.price)
        .min()
}

/// 列表店铺过滤：店铺账号强制本店；平台账号可按请求值过滤。
fn scoped_list_shop_id(requested: Option<i32>) -> Option<i32> {
    TenantScope::current_list_filter().or(requested.filter(|id| *id > 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tenant_scope_scopes_list_and_blocks_cross_shop_groupon() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        let shop = TenantScope::for_shop(8);
        let (filter, write_shop, blocked) = with_tenant_scope(shop, async {
            (
                scoped_list_shop_id(Some(9)),
                TenantScope::current().write_shop_id(None).unwrap_or(-1),
                TenantScope::current_ensure_access(9).is_err(),
            )
        })
        .await;
        assert_eq!(filter, Some(8));
        assert_eq!(write_shop, 8);
        assert!(blocked);
        let filter = with_tenant_scope(TenantScope::platform(), async {
            scoped_list_shop_id(Some(9))
        })
        .await;
        assert_eq!(filter, Some(9));
    }

    #[test]
    fn marks_helper_matches_java_list_tostring() {
        assert_eq!(java_marks_string(&[]), "[]");
        assert_eq!(java_marks_string(&[1]), "[1]");
        assert_eq!(java_marks_string(&[1, 2, 3]), "[1, 2, 3]");
    }

    #[test]
    fn int_list_parses_java_and_json_forms() {
        assert_eq!(parse_int_list(None), Vec::<i32>::new());
        assert_eq!(parse_int_list(Some("")), Vec::<i32>::new());
        assert_eq!(parse_int_list(Some("[1, 2, 3]")), vec![1, 2, 3]);
        assert_eq!(parse_int_list(Some("[1,2,3]")), vec![1, 2, 3]);
        assert_eq!(parse_int_list(Some("[]")), Vec::<i32>::new());
        assert_eq!(parse_int_list(Some(" 4 ")), vec![4]);
    }

    #[test]
    fn audience_scope_prefers_send_user_target() {
        let mut p = PromotionPromotionCreateParam::default();
        assert_eq!(audience_scope_json(&p), None);
        p.member_audience = Some(serde_json::json!({"legacy": true}));
        assert_eq!(
            audience_scope_json(&p).as_deref(),
            Some(r#"{"legacy":true}"#)
        );
        p.send_user_target = Some(serde_json::json!({"userIds": [1]}));
        assert_eq!(
            audience_scope_json(&p).as_deref(),
            Some(r#"{"userIds":[1]}"#)
        );
    }

    #[test]
    fn min_price_picks_lowest_across_details() {
        use nextrade_core::entities::GrouponPromotionDetail;
        let d1 = GrouponPromotionDetail {
            group_price: r#"[{"groupNumKey":1,"price":19.90},{"groupNumKey":2,"price":9.90}]"#.into(),
            ..Default::default()
        };
        let d2 = GrouponPromotionDetail {
            group_price: r#"[{"groupNumKey":1,"price":15.00}]"#.into(),
            ..Default::default()
        };
        let refs = vec![&d1, &d2];
        assert_eq!(min_detail_price(&refs), Some(Decimal::new(990, 2)));
        assert_eq!(min_detail_price(&[]), None);
    }
}
