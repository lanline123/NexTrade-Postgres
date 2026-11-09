//! 商品促销服务（`marketing::product_promotion`，9 个管理端端点）。
//!
//! 对应原 Java `com.nextrade.service.promotion.impl.ProductPromotionServiceImpl`：
//! `list` / `detail` / `create` / `update` / `del` / `updateField` / `batch` /
//! `getConfig` / `conflictPage`（另有 `parsePrice` 属购物车促销域，不在本切片）。
//!
//! # 已实现（与 Java 逐条对齐）
//!
//! - `list`：关键词 LIKE + `promotionType` CSV IN + 店铺 + 时间状态 + 排序白名单；
//!   VO 回填时间字符串 / `productTime` / 状态文案 / 类型文案 / 类型数据 / 赠品。
//! - `detail`：同上，并从 `promotion` 表回填会员范围/标记/人群 JSON。
//! - `create` / `update`：`@Valid` 名称校验、冲突校验（时间重叠 + 店铺 + 类型 +
//!   指定商品 `JSON_CONTAINS`）、`dataJudge`（最小满额/时间戳/JSON 列）、
//!   事务内同步 `promotion` 行（存在则 update，否则 insert；type=3/4/5）。
//! - `del`：id 校验、启用中禁止删除、删除 `product_promotion` + 对应 `promotion`。
//! - `updateField`：白名单 `is_available`/`sort_order`；启用进行中活动前做冲突校验；
//!   启停同步 `promotion.is_available`。
//! - `batch`：`type=del` 批量删除（Java 继承实现，仅删 `product_promotion`）。
//! - `config`：时间状态字典 + 会员等级列表。
//! - `conflict`：启用活动分页（时间重叠 + 类型 + `JSON_CONTAINS` 指定商品），
//!   「指定商品不参与」范围做「并集数 == 在售商品数即不冲突」后置过滤。
//!
//! # 偏差（诚实声明）
//!
//! 1. **店铺上下文**：Java `HeaderUtils.getShopId()`；Rust JWT 无 shop_id，列表/
//!   冲突查询接受 `?shopId=`（缺省 0 = 平台）；创建取 DTO `shopId`（缺省 0）。
//! 2. **会员人群**：`MemberAudienceService`（parseScope/fillTargetUsers/toLegacy）
//!    未迁移；`memberScope` 原样保存/返回 JSON，VO 的 `memberAudience`/
//!    `sendUserTarget` 直接回填原始 JSON（不做人群展开）。
//! 3. **赠品/等级**：赠品按 `product_gift` 表回填（Java `productGiftService`）；
//!    等级列表直接查 `user_rank`（Java `UserRankService#getUserRank`）。
//! 4. `batch` 只删除商品促销行（与 Java 继承实现一致），不级联 `promotion` 行。
//! 5. `conflictPage` 的「指定商品」范围不再做 Java 的 `rangeProducts` 二次裁剪
//!    （Java 该分支实际未改动促销集合）。

use std::collections::BTreeMap;
use std::sync::Arc;

use rust_decimal::Decimal;

use nextrade_core::entities::{ProductPromotion, Promotion};
use nextrade_core::models::marketing::product_promotion::{
    conflict_vo_from_entity, format_time, gain_type, int_list_json, min_amount_from_type_data,
    parse_datetime, parse_int_list_json, time_status_code, time_status_name,
    ConflictPageQuery, ConflictPageVO, ProductPromotionConfigVO, ProductPromotionDTO,
    ProductPromotionListQuery, ProductPromotionVO, PromotionTypeData, RankVO, CANNOT_DELETE_WHILE_ENABLED,
    DEL_ID_ERROR, EXIST_CONFLICT_PROMOTION, EXIST_CONFLICT_PROMOTION_ERROR,
    INVALID_FIELD_VALUE, PRODUCT_PROMOTION_FIELDS, PRODUCT_PROMOTION_ID_NOT_NULL,
    PROMOTION_NOT_EXISTS, RANGE_EXCLUDE_PRODUCT, TIME_STATUS_FORTHCOMING, TIME_STATUS_OFF,
    TIME_STATUS_ON,
};
use nextrade_core::models::merchant::shop_manage::{BatchDTO, UpdateFieldDTO};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

pub struct ProductPromotionService {
    repos: Arc<Repositories>,
}

impl ProductPromotionService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// `GET /adminapi/promotion/productPromotion/list`：活动分页。
    pub async fn list(
        &self,
        q: &ProductPromotionListQuery,
    ) -> Result<PageResult<ProductPromotionVO>> {
        let page = q.page_query();
        let now = nextrade_common::utils::now_seconds();
        let (total, rows) = self
            .repos
            .marketing
            .product_promotion
            .paginate(q, &page, now, TenantScope::current_list_filter())
            .await?;
        let mut list = Vec::with_capacity(rows.len());
        for row in &rows {
            list.push(self.convert(row, now).await?);
        }
        Ok(PageResult::new(total, list, &page))
    }

    /// `GET /adminapi/promotion/productPromotion/detail?id=`：活动详情。
    pub async fn detail(&self, id: Option<i32>) -> Result<Option<ProductPromotionVO>> {
        let Some(id) = id else {
            return Ok(None);
        };
        let Some(row) = self
            .repos
            .marketing
            .product_promotion
            .find_by_id(id)
            .await?
        else {
            return Ok(None);
        };
        // 多租户：店铺账号只能查看本店促销
        TenantScope::current_ensure_access(row.shop_id)?;
        let now = nextrade_common::utils::now_seconds();
        let mut vo = self.convert(&row, now).await?;
        if let Some(promotion) = self
            .repos
            .marketing
            .product_promotion
            .find_promotion_row(id)
            .await?
        {
            vo.member_range = Some(promotion.member_range);
            vo.member_marks = promotion
                .member_marks
                .as_deref()
                .map(parse_int_list_json);
            vo.member_audience = promotion
                .member_scope
                .as_deref()
                .and_then(|raw| serde_json::from_str(raw).ok());
        }
        Ok(Some(vo))
    }

    /// `POST /adminapi/promotion/productPromotion/create`：创建活动。
    pub async fn create(&self, dto: &ProductPromotionDTO) -> Result<()> {
        dto.validate().map_err(Error::Validation)?;
        let promotion_type = dto.promotion_type.unwrap_or(0);
        let gain = gain_type(promotion_type).map_err(Error::Validation)?;
        // 多租户：店铺账号强制落本店；平台账号可指定/落平台级（0）
        let shop_id = TenantScope::current().write_shop_id(dto.shop_id)?;
        let start_ts = dto.start_time.as_deref().map(parse_datetime).unwrap_or(0);
        let end_ts = dto.end_time.as_deref().map(parse_datetime).unwrap_or(0);
        let range = dto.range.unwrap_or(0);
        let range_data = dto.range_data.clone().unwrap_or_default();
        self.ensure_no_conflict(
            start_ts,
            end_ts,
            range,
            &range_data,
            promotion_type,
            shop_id,
            0,
        )
        .await?;

        let mut entity = build_entity(dto, None, shop_id)?;
        entity.shop_id = shop_id;
        let id = self
            .repos
            .marketing
            .product_promotion
            .insert(&entity)
            .await?;
        self.sync_promotion(&entity, id, gain, dto).await?;
        Ok(())
    }

    /// `POST /adminapi/promotion/productPromotion/update`：更新活动。
    pub async fn update(&self, dto: &ProductPromotionDTO) -> Result<()> {
        let id = dto
            .promotion_id
            .ok_or_else(|| Error::Validation(PRODUCT_PROMOTION_ID_NOT_NULL.into()))?;
        let exist = self
            .repos
            .marketing
            .product_promotion
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz_msg(PROMOTION_NOT_EXISTS))?;
        // 多租户：更新前校验归属
        TenantScope::current_ensure_access(exist.shop_id)?;
        dto.validate().map_err(Error::Validation)?;
        let promotion_type = dto.promotion_type.unwrap_or(exist.promotion_type);
        let gain = gain_type(promotion_type).map_err(Error::Validation)?;
        // 多租户：店铺账号强制落本店；平台账号保留/指定店铺
        let shop_id = TenantScope::current().write_shop_id(dto.shop_id.or(Some(exist.shop_id)))?;
        let start_ts = dto
            .start_time
            .as_deref()
            .map(parse_datetime)
            .unwrap_or(i64::from(exist.start_time));
        let end_ts = dto
            .end_time
            .as_deref()
            .map(parse_datetime)
            .unwrap_or(i64::from(exist.end_time));
        let range = dto.range.unwrap_or(exist.range);
        let range_data = dto
            .range_data
            .clone()
            .unwrap_or_else(|| parse_int_list_json(&exist.range_data));
        self.ensure_no_conflict(
            start_ts,
            end_ts,
            range,
            &range_data,
            promotion_type,
            shop_id,
            id,
        )
        .await?;

        let mut entity = build_entity(dto, Some(&exist), shop_id)?;
        entity.promotion_id = id;
        entity.shop_id = shop_id;
        self.repos
            .marketing
            .product_promotion
            .update(&entity)
            .await?;
        self.sync_promotion(&entity, id, gain, dto).await?;
        Ok(())
    }

    /// `POST /adminapi/promotion/productPromotion/del`：删除活动。
    ///
    /// 主键来自 `OperateDTO.id`（Java Integer，Rust 侧 `Option<i64>`，超出 i32 视为非法）。
    pub async fn del(&self, id: Option<i64>) -> Result<()> {
        let id = id
            .and_then(|v| i32::try_from(v).ok())
            .ok_or_else(|| Error::Validation(DEL_ID_ERROR.into()))?;
        let entity = self
            .repos
            .marketing
            .product_promotion
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz_msg(PROMOTION_NOT_EXISTS))?;
        // 多租户：删除前校验归属
        TenantScope::current_ensure_access(entity.shop_id)?;
        if entity.is_available == 1 {
            return Err(Error::biz_msg(CANNOT_DELETE_WHILE_ENABLED));
        }
        self.repos
            .marketing
            .product_promotion
            .delete_by_ids(&[id])
            .await?;
        if let Ok(gain) = gain_type(entity.promotion_type) {
            // type = -1 表示无法识别的类型（Java 同样不会命中删除条件）
            self.repos
                .marketing
                .product_promotion
                .delete_promotion_row(id, gain)
                .await?;
        }
        Ok(())
    }

    /// `POST /adminapi/promotion/productPromotion/updateField`：白名单字段更新。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<()> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation(PRODUCT_PROMOTION_ID_NOT_NULL.into()))?;
        let field = dto
            .field
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation(INVALID_FIELD_VALUE.into()))?;
        let Some(column) = map_update_field(field) else {
            return Err(Error::biz_msg(INVALID_FIELD_VALUE));
        };
        let val = dto
            .val
            .as_deref()
            .ok_or_else(|| Error::Validation(INVALID_FIELD_VALUE.into()))?
            .trim()
            .to_string();
        let entity = self
            .repos
            .marketing
            .product_promotion
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz_msg("商品促销不存在"))?;
        // 多租户：字段更新前校验归属
        TenantScope::current_ensure_access(entity.shop_id)?;
        // 仅「启用 + 进行中」需要冲突校验（Java 同口径，避免阻止消除冲突的停用操作）
        let now = nextrade_common::utils::now_seconds();
        if column == "is_available"
            && val == "1"
            && now >= i64::from(entity.start_time)
            && now <= i64::from(entity.end_time)
        {
            let range_data = parse_int_list_json(&entity.range_data);
            self.ensure_no_conflict(
                i64::from(entity.start_time),
                i64::from(entity.end_time),
                entity.range,
                &range_data,
                entity.promotion_type,
                entity.shop_id,
                entity.promotion_id,
            )
            .await?;
        }
        self.repos
            .marketing
            .product_promotion
            .update_field(id, column, &val)
            .await?;
        if column == "is_available" {
            if let Ok(available) = val.parse::<i32>() {
                self.repos
                    .marketing
                    .product_promotion
                    .update_promotion_available(id, available)
                    .await?;
            }
        }
        Ok(())
    }

    /// `POST /adminapi/promotion/productPromotion/batch`：批量操作（仅 `del`）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<()> {
        if dto.r#type.as_deref() != Some("del") {
            return Ok(());
        }
        let ids = dto.ids.as_deref().unwrap_or_default();
        if ids.is_empty() {
            return Err(Error::Validation(PRODUCT_PROMOTION_ID_NOT_NULL.into()));
        }
        // 多租户：批量删除前逐条校验归属（越权直接 403，不静默跳过）
        for id in ids {
            if let Some(entity) = self
                .repos
                .marketing
                .product_promotion
                .find_by_id(*id)
                .await?
            {
                TenantScope::current_ensure_access(entity.shop_id)?;
            }
        }
        self.repos
            .marketing
            .product_promotion
            .delete_by_ids(ids)
            .await?;
        Ok(())
    }

    /// `GET /adminapi/promotion/productPromotion/config`：配置字典 + 会员等级。
    pub async fn config(&self) -> Result<ProductPromotionConfigVO> {
        let mut promotion_status = BTreeMap::new();
        for code in [TIME_STATUS_ON, TIME_STATUS_OFF, TIME_STATUS_FORTHCOMING] {
            promotion_status.insert(code, time_status_name(code).to_string());
        }
        let rank_list = self
            .repos
            .marketing
            .product_promotion
            .rank_list()
            .await?
            .into_iter()
            .map(|r| RankVO {
                rank_id: r.rank_id,
                rank_name: r.rank_name,
            })
            .collect();
        Ok(ProductPromotionConfigVO {
            promotion_status,
            rank_list,
        })
    }

    /// `GET /adminapi/promotion/productPromotion/conflict`：冲突活动列表。
    pub async fn conflict(
        &self,
        q: &ConflictPageQuery,
    ) -> Result<PageResult<ConflictPageVO>> {
        let page = q.page_query();
        // 多租户：店铺账号强制本店；平台账号可按请求店铺（缺省 0 = 平台级）
        let shop_id = TenantScope::current_list_filter().unwrap_or(q.shop_id.unwrap_or(0));
        let mut query = q.clone();
        query.shop_id = Some(shop_id);
        let (total, mut rows) = self
            .repos
            .marketing
            .product_promotion
            .conflict_paginate(&query, &page)
            .await?;
        // Java 后置过滤：range=4（指定商品不参与）且并集数 == 在售商品总数 → 不冲突
        let total_products = if rows.iter().any(|r| r.range == RANGE_EXCLUDE_PRODUCT) {
            self.repos
                .marketing
                .product_promotion
                .count_on_sale_products(shop_id)
                .await
                .unwrap_or(0)
        } else {
            0
        };
        rows.retain(|r| !should_drop_exclude_conflict(r, total_products));
        let now = nextrade_common::utils::now_seconds();
        let list: Vec<ConflictPageVO> = rows
            .iter()
            .map(|r| conflict_vo_from_entity(r, now))
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    // ————————————————————— 内部辅助 —————————————————————

    async fn ensure_no_conflict(
        &self,
        start_ts: i64,
        end_ts: i64,
        range: i32,
        range_data: &[i32],
        promotion_type: i32,
        shop_id: i32,
        exclude_id: i32,
    ) -> Result<()> {
        let conflicts = self
            .repos
            .marketing
            .product_promotion
            .find_conflicts(
                start_ts,
                end_ts,
                range,
                range_data,
                promotion_type,
                shop_id,
                exclude_id,
            )
            .await?;
        if conflicts.is_empty() {
            Ok(())
        } else {
            Err(Error::biz(
                EXIST_CONFLICT_PROMOTION_ERROR,
                EXIST_CONFLICT_PROMOTION,
            ))
        }
    }

    /// 同步 `promotion` 行（Java `savePromotion`）。
    async fn sync_promotion(
        &self,
        entity: &ProductPromotion,
        relation_id: i32,
        gain: i32,
        dto: &ProductPromotionDTO,
    ) -> Result<()> {
        let member_scope = dto
            .send_user_target
            .as_ref()
            .or(dto.member_audience.as_ref())
            .map(|v| v.to_string());
        let row = Promotion {
            promotion_id: 0,
            promotion_name: entity.promotion_name.clone(),
            start_time: entity.start_time,
            end_time: entity.end_time,
            r#type: gain,
            shop_id: entity.shop_id,
            relation_id: Some(relation_id),
            range: Some(entity.range),
            range_data: Some(entity.range_data.clone()),
            sku_ids: None,
            is_available: Some(1),
            is_delete: None,
            member_range: dto.member_range.unwrap_or(0),
            member_marks: dto.member_marks.as_deref().map(int_list_json),
            member_scope,
        };
        let existing = self
            .repos
            .marketing
            .product_promotion
            .find_promotion_row(relation_id)
            .await?;
        if existing.is_some() {
            self.repos
                .marketing
                .product_promotion
                .update_promotion_row(&row)
                .await?;
        } else {
            self.repos
                .marketing
                .product_promotion
                .insert_promotion_row(&row)
                .await?;
        }
        Ok(())
    }

    /// 实体 → VO（Java `convert` + 赠品回填）。
    async fn convert(&self, e: &ProductPromotion, now: i64) -> Result<ProductPromotionVO> {
        let type_data: Vec<PromotionTypeData> =
            serde_json::from_str(&e.promotion_type_data).unwrap_or_default();
        let gift_ids: Vec<i32> = type_data.iter().filter_map(|d| d.gift_id).collect();
        let gifts = self
            .repos
            .marketing
            .product_promotion
            .gifts_by_ids(&gift_ids)
            .await?
            .into_iter()
            .map(|g| nextrade_core::models::marketing::product_promotion::GiftVO {
                gift_id: g.gift_id,
                gift_name: g.gift_name,
                gift_stock: g.gift_stock,
                product_id: g.product_id,
                sku_id: g.sku_id,
            })
            .collect();
        let status_code = time_status_code(
            i64::from(e.start_time),
            i64::from(e.end_time),
            now,
        );
        let mut product_time = Vec::new();
        if e.start_time > 0 {
            product_time.push(format_time(i64::from(e.start_time)));
        }
        if e.end_time > 0 {
            product_time.push(format_time(i64::from(e.end_time)));
        }
        Ok(ProductPromotionVO {
            promotion_id: e.promotion_id,
            promotion_name: e.promotion_name.clone(),
            start_time: format_time(i64::from(e.start_time)),
            end_time: format_time(i64::from(e.end_time)),
            limit_user_rank: parse_int_list_json(&e.limit_user_rank),
            range: e.range,
            range_data: parse_int_list_json(&e.range_data),
            min_order_amount: e.min_order_amount,
            max_order_amount: e.max_order_amount,
            promotion_type: e.promotion_type,
            promotion_type_name: nextrade_core::models::marketing::product_promotion::promotion_type_name(e.promotion_type).to_string(),
            product_status: time_status_name(status_code).to_string(),
            product_time,
            product_gift: gifts,
            promotion_type_data: type_data,
            is_available: e.is_available,
            sort_order: e.sort_order,
            shop_id: e.shop_id,
            rules_type: e.rules_type,
            unit: e.unit,
            member_range: None,
            member_marks: None,
            member_audience: None,
            send_user_target: None,
        })
    }
}

// ————————————————————— 纯函数 —————————————————————

/// `updateField` 字段名归一化：支持驼峰与列名，非法返回 None。
#[must_use]
pub fn map_update_field(field: &str) -> Option<&'static str> {
    let normalized = match field {
        "isAvailable" => "is_available",
        "sortOrder" => "sort_order",
        "is_available" => "is_available",
        "sort_order" => "sort_order",
        _ => return None,
    };
    PRODUCT_PROMOTION_FIELDS
        .contains(&normalized)
        .then_some(normalized)
}

/// 「指定商品不参与」范围：并集去重后数量 == 在售商品总数 → 视为不冲突（Java 逻辑）。
#[must_use]
pub fn should_drop_exclude_conflict(e: &ProductPromotion, total_products: i64) -> bool {
    if e.range != RANGE_EXCLUDE_PRODUCT || total_products <= 0 {
        return false;
    }
    let mut merged: Vec<i32> = parse_int_list_json(&e.range_data);
    merged.sort_unstable();
    merged.dedup();
    merged.len() as i64 == total_products
}

/// `dataJudge` + `BeanUtils.copyProperties`：DTO → 实体（`existing` 为更新时的旧行）。
fn build_entity(
    dto: &ProductPromotionDTO,
    existing: Option<&ProductPromotion>,
    shop_id: i32,
) -> Result<ProductPromotion> {
    let type_data = dto.promotion_type_data.clone().unwrap_or_default();
    let start_ts = dto
        .start_time
        .as_deref()
        .map(parse_datetime)
        .unwrap_or(0);
    let end_ts = dto.end_time.as_deref().map(parse_datetime).unwrap_or(0);
    let base = existing.cloned().unwrap_or_default();
    Ok(ProductPromotion {
        promotion_id: dto.promotion_id.unwrap_or(0),
        promotion_name: dto
            .promotion_name
            .clone()
            .unwrap_or_else(|| base.promotion_name.clone())
            .trim()
            .to_string(),
        start_time: start_ts as i32,
        end_time: end_ts as i32,
        limit_user_rank: dto
            .limit_user_rank
            .as_deref()
            .map(int_list_json)
            .unwrap_or_else(|| base.limit_user_rank.clone()),
        range: dto.range.unwrap_or(base.range),
        range_data: dto
            .range_data
            .as_deref()
            .map(int_list_json)
            .unwrap_or_else(|| base.range_data.clone()),
        // Java create 固定 maxOrderAmount = 0；update 走 copyProperties
        min_order_amount: min_amount_from_type_data(&type_data),
        max_order_amount: if existing.is_none() {
            Decimal::ZERO
        } else {
            dto.max_order_amount.unwrap_or(base.max_order_amount)
        },
        promotion_type: dto.promotion_type.unwrap_or(base.promotion_type),
        promotion_type_data: serde_json::to_string(&type_data).unwrap_or_else(|_| "[]".into()),
        is_available: dto.is_available.unwrap_or(base.is_available),
        sort_order: dto.sort_order.unwrap_or(base.sort_order),
        shop_id,
        rules_type: dto.rules_type.unwrap_or(base.rules_type),
        unit: dto.unit.unwrap_or(base.unit),
    })
}

/// 列表店铺过滤：店铺账号强制本店；平台账号不过滤。
fn scoped_list_shop_id() -> Option<i32> {
    TenantScope::current_list_filter()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_field_mapping_matches_whitelist() {
        assert_eq!(map_update_field("isAvailable"), Some("is_available"));
        assert_eq!(map_update_field("sortOrder"), Some("sort_order"));
        assert_eq!(map_update_field("is_available"), Some("is_available"));
        assert_eq!(map_update_field("promotion_name"), None);
        assert_eq!(map_update_field("1; DROP TABLE"), None);
    }

    #[test]
    fn exclude_range_drop_rule_matches_java() {
        let mut e = ProductPromotion::default();
        e.range = RANGE_EXCLUDE_PRODUCT;
        e.range_data = "[1,2,3]".into();
        assert!(!should_drop_exclude_conflict(&e, 5));
        assert!(should_drop_exclude_conflict(&e, 3));
        // 非「不含商品」范围不删
        e.range = 3;
        assert!(!should_drop_exclude_conflict(&e, 3));
    }

    #[test]
    fn build_entity_applies_data_judge_defaults() {
        let dto = ProductPromotionDTO {
            promotion_name: Some(" 活动A ".into()),
            start_time: Some("2026-10-18 10:00:00".into()),
            end_time: Some("2026-10-19 10:00:00".into()),
            promotion_type: Some(1),
            range: Some(3),
            range_data: Some(vec![1, 2]),
            limit_user_rank: Some(vec![2]),
            promotion_type_data: Some(vec![PromotionTypeData {
                min_amount: Some(Decimal::new(10000, 2)),
                ..Default::default()
            }]),
            ..Default::default()
        };
        let e = build_entity(&dto, None, 0).unwrap();
        assert_eq!(e.promotion_name, "活动A");
        assert_eq!(e.range_data, "[1,2]");
        assert_eq!(e.limit_user_rank, "[2]");
        assert_eq!(e.min_order_amount, Decimal::new(10000, 2));
        assert_eq!(e.max_order_amount, Decimal::ZERO);
        assert!(e.start_time > 0 && e.end_time > e.start_time);
        assert!(e.promotion_type_data.contains("minAmount"));
    }

    #[tokio::test]
    async fn tenant_scope_scopes_list_and_blocks_cross_shop_promotion() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        let shop = TenantScope::for_shop(8);
        let (filter, write_shop, blocked) = with_tenant_scope(shop, async {
            (
                scoped_list_shop_id(),
                TenantScope::current().write_shop_id(None).unwrap_or(-1),
                TenantScope::current_ensure_access(9).is_err(),
            )
        })
        .await;
        assert_eq!(filter, Some(8));
        assert_eq!(write_shop, 8);
        assert!(blocked);
        let filter = with_tenant_scope(TenantScope::platform(), async { scoped_list_shop_id() }).await;
        assert_eq!(filter, None);
    }
}
