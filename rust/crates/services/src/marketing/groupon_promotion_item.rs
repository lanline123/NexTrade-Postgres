//! 拼团活动商品服务（`marketing::groupon_promotion_item`，8 个管理端端点）。
//!
//! 对应原 Java `com.nextrade.service.promotion.impl.GrouponPromotionItemServiceImpl`：
//! `create` / `update` / `del` / `batch` / `detail` / `list` / `post` / `productList`。
//!
//! # 已实现（与 Java 逐条对齐）
//!
//! - `create`：`@Valid` 校验（逐字消息）、秒杀/限时折扣/其它拼团时间窗冲突校验
//!   （`seckill_item` / `time_discount_item` / `groupon_promotion_item` JOIN
//!   `groupon_promotion`，SQL 与 `GrouponPromotionItemMapper.xml` 一致）、
//!   SKU 启用校验（`product_sku.is_enabled=1`）、明细落库、
//!   `promotion`（type=7）的 `range_data`/`sku_ids` 同步。
//! - `update`：存在性校验 + 明细重建 + `promotion` 同步。
//! - `del` / `batch`：删除商品行与明细（`batch` 仅 `del`，与 Java 一致）。
//! - `detail`：商品原价/库存 + 启用 SKU + 明细（含拼团价 JSON 解析）。
//! - `list`：LEFT JOIN product 分页 + `productGroupId` 分组展开 + 明细聚合
//!   （最低/最高拼团价、团规模、可售数量范围）。
//! - `post`：Java 实现恒返回 null，本实现同样返回 `None`。
//! - `productList`：上架商品分页 + 是否已加入/其它拼团/秒杀/限时折扣/预约/预售标记。
//!
//! # 偏差（诚实声明）
//!
//! 1. **店铺上下文**：Java 通过请求头取 `shopId` 参与秒杀/限时折扣过滤；Rust JWT
//!    无 shop_id，本实现按商品自身 `shop_id` 侧（不做 O2O 门店改写）。
//! 2. `update` 的明细采用**先删后插**（Java `updateBatchById` 保留明细主键，
//!    本实现 `groupon_detail_id` 会变化）。
//! 3. `productSkus`/`product` 返回精简结构（Java `ProductSkuDTO`/`Product` 全字段
//!    含翻译/多图字段，本实现保留核心展示字段）。
//! 4. `post` 恒 `null`（与 Java 现状一致，推广能力未在 Java 实现）。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::entities::{GrouponPromotionDetail, GrouponPromotionItem};
use nextrade_core::models::marketing::groupon_promotion_item::{
    assemble_ranges, build_group_num_json, build_group_price_json, group_num_range,
    is_presale_product, is_reservation_product, parse_group_num_json, parse_price_json, CreateParam,
    DetailVO, GrouponPostVO, GrouponProductVO, GrouponNumVO, ItemBatchParam, ItemDetailVO,
    ItemPageQuery, ItemPageVO, PriceParam, ProductLite, ProductPageQuery, SkuLite, UpdateParam,
    MSG_EXIST_OTHER_GROUPON, MSG_EXIST_SECKILL, MSG_EXIST_TIME_DISCOUNT, MSG_SKU_DISABLED,
    MSG_UNKNOWN_ITEM, MSG_UNKNOWN_PROMOTION,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

pub struct GrouponPromotionItemService {
    repos: Arc<Repositories>,
}

impl GrouponPromotionItemService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// `POST /adminapi/promotion/groupon/item/create`：创建拼团商品。
    pub async fn create(&self, param: &CreateParam) -> Result<()> {
        param.validate().map_err(Error::Validation)?;
        let promotion_id = param.groupon_promotion_id.unwrap_or(0);
        let promotion = self
            .repos
            .marketing
            .groupon_promotion_item
            .find_promotion(promotion_id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_UNKNOWN_PROMOTION))?;
        // 多租户：店铺账号只能向本店拼团活动添加商品
        TenantScope::current_ensure_access(promotion.shop_id.unwrap_or(0))?;
        let product_id = param.product_id.unwrap_or(0);
        self.check_product_conflicts(product_id, promotion_id, promotion.start_time, promotion.end_time)
            .await?;

        let now_ms = nextrade_common::utils::now_millis();
        let item = build_item(param, 0, now_ms);
        let item_id = self
            .repos
            .marketing
            .groupon_promotion_item
            .insert_item(&item)
            .await?;
        self.save_details(param, promotion_id, item_id, now_ms).await?;
        self.sync_promotion_range(promotion_id).await?;
        Ok(())
    }

    /// `POST /adminapi/promotion/groupon/item/update`：更新拼团商品。
    pub async fn update(&self, param: &UpdateParam) -> Result<()> {
        let item_id = param
            .groupon_promotion_item_id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::Validation("拼团活动商品 ID 不能为空".into()))?;
        param.base.validate().map_err(Error::Validation)?;
        let exist = self
            .repos
            .marketing
            .groupon_promotion_item
            .find_item(item_id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_UNKNOWN_ITEM))?;
        let promotion_id = param.base.groupon_promotion_id.unwrap_or(exist.groupon_promotion_id);
        let promotion = self
            .repos
            .marketing
            .groupon_promotion_item
            .find_promotion(promotion_id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_UNKNOWN_PROMOTION))?;
        // 多租户：归属校验（经拼团活动 shop_id）
        TenantScope::current_ensure_access(promotion.shop_id.unwrap_or(0))?;
        let product_id = param.base.product_id.unwrap_or(exist.product_id);
        self.check_product_conflicts(product_id, promotion_id, promotion.start_time, promotion.end_time)
            .await?;

        let now_ms = nextrade_common::utils::now_millis();
        let mut item = build_item(&param.base, item_id, exist.add_time);
        item.groupon_promotion_item_id = item_id;
        self.repos
            .marketing
            .groupon_promotion_item
            .update_item(&item)
            .await?;
        // 明细先删后插（Java `updateBatchById` 保留主键；见模块头偏差 2）
        self.repos
            .marketing
            .groupon_promotion_item
            .delete_details_by_items(&[item_id])
            .await?;
        self.save_details(&param.base, promotion_id, item_id, now_ms).await?;
        self.sync_promotion_range(promotion_id).await?;
        Ok(())
    }

    /// `POST /adminapi/promotion/groupon/item/del`：删除拼团商品。
    pub async fn del(&self, id: Option<i64>) -> Result<()> {
        let id = id.ok_or_else(|| Error::biz_msg(MSG_UNKNOWN_ITEM))?;
        let item = self
            .repos
            .marketing
            .groupon_promotion_item
            .find_item(id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_UNKNOWN_ITEM))?;
        // 多租户：删除前校验归属（经拼团活动 shop_id）
        self.ensure_promotion_access(item.groupon_promotion_id).await?;
        self.repos
            .marketing
            .groupon_promotion_item
            .delete_items(&[id])
            .await?;
        self.repos
            .marketing
            .groupon_promotion_item
            .delete_details_by_items(&[id])
            .await?;
        self.sync_promotion_range(item.groupon_promotion_id).await?;
        Ok(())
    }

    /// `POST /adminapi/promotion/groupon/item/batch`：批量操作（仅 `del`）。
    pub async fn batch(&self, param: &ItemBatchParam) -> Result<()> {
        if param.r#type.as_deref() != Some("del") {
            return Ok(());
        }
        let ids = param.ids.as_deref().unwrap_or_default();
        if ids.is_empty() {
            return Ok(());
        }
        // 多租户：批量删除前逐条校验归属（经拼团活动 shop_id）
        for id in ids {
            if let Some(item) = self
                .repos
                .marketing
                .groupon_promotion_item
                .find_item(*id)
                .await?
            {
                self.ensure_promotion_access(item.groupon_promotion_id).await?;
            }
        }
        self.repos
            .marketing
            .groupon_promotion_item
            .delete_items(ids)
            .await?;
        self.repos
            .marketing
            .groupon_promotion_item
            .delete_details_by_items(ids)
            .await?;
        // Java `batch` 未同步 promotion range（仅 del 单条时同步）
        Ok(())
    }

    /// `GET /adminapi/promotion/groupon/item/detail?grouponPromotionItemId=`：详情。
    pub async fn detail(&self, id: i64) -> Result<ItemDetailVO> {
        let item = self
            .repos
            .marketing
            .groupon_promotion_item
            .find_item(id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_UNKNOWN_ITEM))?;
        // 多租户：详情归属校验（经拼团活动 shop_id）
        self.ensure_promotion_access(item.groupon_promotion_id).await?;
        let details = self
            .repos
            .marketing
            .groupon_promotion_item
            .find_details_by_item(id)
            .await?;
        let product = self
            .repos
            .marketing
            .groupon_promotion_item
            .find_product(item.product_id)
            .await?;
        let skus = self
            .repos
            .marketing
            .groupon_promotion_item
            .find_enabled_skus(&[item.product_id])
            .await?;
        let sku_map: HashMap<i64, SkuLite> = skus
            .iter()
            .map(|s| (i64::from(s.sku_id), SkuLite::from_entity(s)))
            .collect();
        let detail_vos: Vec<DetailVO> = details
            .iter()
            .map(|d| DetailVO {
                groupon_detail_id: d.groupon_detail_id,
                product_id: d.product_id,
                sku_id: d.sku_id,
                product_sku: sku_map.get(&d.sku_id).cloned(),
                limit_num: d.limit_num,
                is_join: d.is_join,
                group_promotion_detail_prices: parse_price_json(&d.group_price),
            })
            .collect();
        Ok(ItemDetailVO {
            groupon_promotion_item_id: item.groupon_promotion_item_id,
            groupon_promotion_id: item.groupon_promotion_id,
            product_id: item.product_id,
            product_skus: skus.iter().map(SkuLite::from_entity).collect(),
            product_price: product
                .as_ref()
                .map(|p| p.product_price)
                .unwrap_or_default(),
            product_stock: product.as_ref().map(|p| p.product_stock).unwrap_or(0),
            group_type: item.group_type,
            order_limit_num: item.order_limit_num,
            is_auto_group: item.is_auto_group,
            min_group_num: item.min_group_num,
            sort: item.sort,
            group_nums: parse_group_num_json(&item.group_num),
            groupon_promotion_details: detail_vos,
        })
    }

    /// `GET /adminapi/promotion/groupon/item/post`：推广信息（Java 恒 null）。
    pub async fn post(&self, _id: i64) -> Result<Option<GrouponPostVO>> {
        Ok(None)
    }

    /// `GET /adminapi/promotion/groupon/item/list`：拼团商品分页。
    pub async fn list(&self, q: &ItemPageQuery) -> Result<PageResult<ItemPageVO>> {
        let page = q.page_query();
        let group_product_ids: Option<Vec<i32>> = match q.product_group_id {
            Some(group_id) => {
                let ids = self
                    .repos
                    .marketing
                    .groupon_promotion_item
                    .product_group_ids(group_id)
                    .await?
                    .unwrap_or_default();
                Some(if ids.is_empty() { vec![0] } else { ids })
            }
            None => None,
        };
        let (total, rows) = self
            .repos
            .marketing
            .groupon_promotion_item
            .page_items(q, &page, group_product_ids.as_deref())
            .await?;
        if rows.is_empty() {
            return Ok(PageResult::new(total, Vec::new(), &page));
        }
        let item_ids: Vec<i64> = rows
            .iter()
            .map(|r| r.groupon_promotion_item_id)
            .collect();
        let all_details = self
            .repos
            .marketing
            .groupon_promotion_item
            .find_details_by_items(&item_ids)
            .await?;
        let mut list = Vec::with_capacity(rows.len());
        for row in &rows {
            let details: Vec<DetailVO> = all_details
                .iter()
                .filter(|d| d.groupon_promotion_item_id == row.groupon_promotion_item_id)
                .map(|d| DetailVO {
                    groupon_detail_id: d.groupon_detail_id,
                    product_id: d.product_id,
                    sku_id: d.sku_id,
                    product_sku: None,
                    limit_num: d.limit_num,
                    is_join: d.is_join,
                    group_promotion_detail_prices: parse_price_json(&d.group_price),
                })
                .collect();
            let (min_price, max_price, _, _, min_limit, max_limit) =
                assemble_ranges(&details);
            let nums = parse_group_num_json(&row.group_num);
            let (min_group_num, max_group_num) = group_num_range(&nums);
            list.push(ItemPageVO {
                groupon_promotion_item_id: row.groupon_promotion_item_id,
                product_id: row.product_id,
                group_num: row.group_num.clone(),
                sort: row.sort,
                product_name: row.product_name.clone(),
                product_sn: row.product_sn.clone(),
                pic_thumb: row.pic_thumb.clone(),
                pic_large: row.pic_large.clone(),
                pic_url: row.pic_url.clone(),
                product_price: row.product_price,
                min_group_price: min_price.or(row.min_group_price),
                max_group_price: max_price,
                min_group_num,
                max_group_num,
                min_limit_num: min_limit,
                max_limit_num: max_limit,
                product_brief: row.product_brief.clone(),
            });
        }
        Ok(PageResult::new(total, list, &page))
    }

    /// `GET /adminapi/promotion/groupon/item/productList`：可选商品分页。
    pub async fn product_list(
        &self,
        q: &ProductPageQuery,
    ) -> Result<PageResult<GrouponProductVO>> {
        let promotion_id = q
            .groupon_promotion_id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::biz_msg(MSG_UNKNOWN_PROMOTION))?;
        let promotion = self
            .repos
            .marketing
            .groupon_promotion_item
            .find_promotion(promotion_id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_UNKNOWN_PROMOTION))?;
        let page = q.page_query();
        let (total, products) = self
            .repos
            .marketing
            .groupon_promotion_item
            .page_products(q, &page)
            .await?;
        if products.is_empty() {
            return Ok(PageResult::new(total, Vec::new(), &page));
        }
        let product_ids: Vec<i64> = products.iter().map(|p| i64::from(p.product_id)).collect();
        let skus = self
            .repos
            .marketing
            .groupon_promotion_item
            .find_enabled_skus(&product_ids)
            .await?;
        let bound = self
            .repos
            .marketing
            .groupon_promotion_item
            .bound_product_ids(promotion_id)
            .await?;
        let now = nextrade_common::utils::now_seconds();
        let mut list = Vec::with_capacity(products.len());
        for p in &products {
            let pid = i64::from(p.product_id);
            let product_skus = skus
                .iter()
                .filter(|s| s.product_id == p.product_id)
                .map(SkuLite::from_entity)
                .collect();
            let is_other = self
                .repos
                .marketing
                .groupon_promotion_item
                .count_groupon_conflict(pid, promotion_id, now, promotion.end_time)
                .await?
                > 0;
            let is_seckill = self
                .repos
                .marketing
                .groupon_promotion_item
                .count_seckill_conflict(pid, now, now)
                .await?
                > 0;
            let is_time_discount = self
                .repos
                .marketing
                .groupon_promotion_item
                .count_time_discount_conflict(pid, now, now)
                .await?
                > 0;
            list.push(GrouponProductVO {
                product: Some(ProductLite::from_entity(p)),
                product_skus,
                is_added: bound.contains(&pid),
                is_other_groupon_product: is_other,
                is_reservation_product: is_reservation_product(p.product_type),
                is_presale_product: is_presale_product(p.sale_mode),
                is_seckill_product: is_seckill,
                is_time_discount_product: is_time_discount,
            });
        }
        Ok(PageResult::new(total, list, &page))
    }

    // ————————————————————— 内部辅助 —————————————————————

    /// 归属校验：经拼团活动 shop_id（店铺账号越权 → 403）。
    async fn ensure_promotion_access(&self, promotion_id: i64) -> Result<()> {
        if let Some(promotion) = self
            .repos
            .marketing
            .groupon_promotion_item
            .find_promotion(promotion_id)
            .await?
        {
            TenantScope::current_ensure_access(promotion.shop_id.unwrap_or(0))?;
        }
        Ok(())
    }

    async fn check_product_conflicts(
        &self,
        product_id: i64,
        promotion_id: i64,
        start_time: i64,
        end_time: i64,
    ) -> Result<()> {
        if self
            .repos
            .marketing
            .groupon_promotion_item
            .count_seckill_conflict(product_id, start_time, end_time)
            .await?
            > 0
        {
            return Err(Error::biz_msg(MSG_EXIST_SECKILL));
        }
        if self
            .repos
            .marketing
            .groupon_promotion_item
            .count_time_discount_conflict(product_id, start_time, end_time)
            .await?
            > 0
        {
            return Err(Error::biz_msg(MSG_EXIST_TIME_DISCOUNT));
        }
        let now = nextrade_common::utils::now_seconds();
        if self
            .repos
            .marketing
            .groupon_promotion_item
            .count_groupon_conflict(product_id, promotion_id, now, end_time)
            .await?
            > 0
        {
            return Err(Error::biz_msg(MSG_EXIST_OTHER_GROUPON));
        }
        Ok(())
    }

    async fn save_details(
        &self,
        param: &CreateParam,
        promotion_id: i64,
        item_id: i64,
        now_ms: i64,
    ) -> Result<()> {
        let details = param
            .groupon_promotion_details
            .as_deref()
            .unwrap_or_default();
        for d in details {
            let sku_id = d.sku_id.unwrap_or(0);
            let product_id = d.product_id.unwrap_or(0);
            if sku_id > 0
                && !self
                    .repos
                    .marketing
                    .groupon_promotion_item
                    .sku_enabled(sku_id, product_id)
                    .await?
            {
                return Err(Error::biz_msg(MSG_SKU_DISABLED));
            }
            let prices = d
                .group_promotion_detail_prices
                .as_deref()
                .unwrap_or_default();
            let detail = GrouponPromotionDetail {
                groupon_detail_id: d.groupon_detail_id.unwrap_or(0),
                groupon_promotion_id: promotion_id,
                groupon_promotion_item_id: item_id,
                product_id,
                sku_id,
                group_price: build_group_price_json(prices),
                limit_num: d.limit_num.unwrap_or(0),
                is_join: d.is_join.unwrap_or(0),
                add_time: now_ms,
            };
            self.repos
                .marketing
                .groupon_promotion_item
                .insert_detail(&detail)
                .await?;
        }
        Ok(())
    }

    /// Java `updPromotion`：按活动维度全部明细回写 `promotion.range_data/sku_ids`。
    async fn sync_promotion_range(&self, promotion_id: i64) -> Result<()> {
        let details = self
            .repos
            .marketing
            .groupon_promotion_item
            .find_details_by_promotion(promotion_id)
            .await?;
        let mut product_ids: Vec<i64> = details.iter().map(|d| d.product_id).collect();
        product_ids.sort_unstable();
        product_ids.dedup();
        let mut sku_ids: Vec<i64> = details
            .iter()
            .map(|d| d.sku_id)
            .filter(|id| *id > 0)
            .collect();
        sku_ids.sort_unstable();
        sku_ids.dedup();
        self.repos
            .marketing
            .groupon_promotion_item
            .update_groupon_promotion_range(promotion_id, &product_ids, &sku_ids)
            .await?;
        Ok(())
    }
}

/// `CreateParam` → `GrouponPromotionItem`（Java `buildGrouponPromotionItem`）。
#[must_use]
pub fn build_item(param: &CreateParam, item_id: i64, add_time: i64) -> GrouponPromotionItem {
    GrouponPromotionItem {
        groupon_promotion_item_id: item_id,
        groupon_promotion_id: param.groupon_promotion_id.unwrap_or(0),
        product_id: param.product_id.unwrap_or(0),
        group_type: param.group_type.unwrap_or(0),
        group_num: build_group_num_json(param.group_nums.as_deref().unwrap_or_default()),
        order_limit_num: param.order_limit_num.unwrap_or(0),
        is_auto_group: param.is_auto_group.unwrap_or(0),
        min_group_num: param.min_group_num.unwrap_or(0),
        sort: param.sort.unwrap_or(0),
        add_time,
        min_group_price: param.min_group_price,
    }
}

/// 成团人数 VO 便捷转换（供测试/扩展）。
#[must_use]
pub fn to_group_num_vos(items: &[PriceParam]) -> Vec<GrouponNumVO> {
    items
        .iter()
        .map(|p| GrouponNumVO {
            group_num_key: p.group_num_key,
            group_num: None,
        })
        .collect()
}

/// 列表/写操作的店铺范围（经拼团活动 shop_id）。
fn scoped_list_shop_id() -> Option<i32> {
    TenantScope::current_list_filter()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tenant_scope_blocks_cross_shop_groupon_item() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        let shop = TenantScope::for_shop(8);
        let (filter, blocked) = with_tenant_scope(shop, async {
            (
                scoped_list_shop_id(),
                TenantScope::current_ensure_access(9).is_err(),
            )
        })
        .await;
        assert_eq!(filter, Some(8));
        assert!(blocked);
        let filter = with_tenant_scope(TenantScope::platform(), async { scoped_list_shop_id() }).await;
        assert_eq!(filter, None);
    }

    #[test]
    fn build_item_matches_java_builder() {
        let param = CreateParam {
            groupon_promotion_id: Some(7),
            product_id: Some(99),
            group_type: Some(2),
            order_limit_num: Some(3),
            is_auto_group: Some(1),
            min_group_num: Some(2),
            min_group_price: Some(rust_decimal::Decimal::new(990, 2)),
            sort: Some(4),
            group_nums: Some(vec![nextrade_core::models::marketing::groupon_promotion_item::GrouponNumParam {
                group_num_key: Some(1),
                group_num: Some(2),
            }]),
            ..Default::default()
        };
        let item = build_item(&param, 0, 1_700_000_000_000);
        assert_eq!(item.groupon_promotion_id, 7);
        assert_eq!(item.product_id, 99);
        assert_eq!(item.group_type, 2);
        assert!(item.group_num.contains("groupNumKey"));
        assert_eq!(item.add_time, 1_700_000_000_000);
    }

    #[test]
    fn group_num_vos_from_prices() {
        let vos = to_group_num_vos(&[PriceParam {
            group_num_key: Some(2),
            price: None,
        }]);
        assert_eq!(vos.len(), 1);
        assert_eq!(vos[0].group_num_key, Some(2));
    }
}
