//! 活动赠品域服务（marketing::product_gift）。
//!
//! Java `ProductGiftController`（7 端点）→ `ProductGiftServiceImpl`。
//! 列表/详情回填商品与 SKU 信息；创建/更新做「同商品唯一 + 礼品卡电子卡禁止 +
//! 库存不超过商品/规格库存」校验。
//!
//! # 偏差
//!
//! 1. `shop_id` 由 `X-Admin-Type`/`X-Shop-Id` 解析（admin→0、shop/pos→头值）；
//!    无法解析时列表返回空页（Java `eq(shop_id, null)` 同样无匹配）。
//! 2. `detail` 不存在：业务 500「活动赠品不存在」（Java NPE）。
//! 3. VO 的 `productInfo/skuInfo` 为**商品子集 JSON**（商品域未迁移全量实体）。
//! 4. `updateField` 白名单 GENERAL_FIELDS 在本表无对应列 ⇒ 恒「field 的值有误」。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::entities::promotion::ProductGift;
use nextrade_core::models::marketing::product_gift::{
    BatchDTO, OperateDTO, ProductGiftCreateDTO, ProductGiftListQuery, ProductGiftUpdateDTO,
    ProductGiftVO, UpdateFieldDTO, MSG_GIFT_CARD_ECARD,
    MSG_NOT_EXISTS, MSG_PRODUCT_EXISTS, MSG_SKU_DISABLED, MSG_STOCK_EXCEED_PRODUCT,
    MSG_STOCK_EXCEED_SKU,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::product_gift::{
    GiftProductRow, GiftSkuRow, ProductGiftRepository,
};
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 活动赠品域服务。
pub struct ProductGiftService {
    repos: Arc<Repositories>,
}

impl ProductGiftService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &ProductGiftRepository {
        &self.repos.marketing.product_gift
    }

    /// 列表（Java `list`）。
    pub async fn list(&self, q: &ProductGiftListQuery) -> Result<PageResult<ProductGiftVO>> {
        // 多租户：店铺账号强制本店；平台账号按平台级（0）
        let shop_id = scope_shop_id();
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let (total, rows) = self.repo().paginate(q, shop_id, limit, offset).await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let product_ids = distinct(rows.iter().map(|r| r.product_id));
        let sku_ids = distinct(rows.iter().map(|r| r.sku_id).filter(|id| *id > 0));
        let products = self.repo().find_products(&product_ids).await?;
        let skus = self.repo().find_skus(&sku_ids).await?;
        let list = rows
            .iter()
            .map(|entity| to_vo(entity, &products, &skus))
            .collect();
        Ok(PageResult::new(u64::try_from(total).unwrap_or(0), list, &page))
    }

    /// 详情（Java `detail`）。
    pub async fn detail(&self, id: i32) -> Result<ProductGiftVO> {
        let entity = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))?;
        // 多租户：店铺账号只能查看本店赠品
        TenantScope::current_ensure_access(entity.shop_id)?;
        let products = self.repo().find_products(&[entity.product_id]).await?;
        let skus = self
            .repo()
            .find_skus(&[entity.sku_id])
            .await?;
        Ok(to_vo(&entity, &products, &skus))
    }

    /// 创建（Java `create`）。
    pub async fn create(&self, dto: &ProductGiftCreateDTO) -> Result<i32> {
        let input = dto.validate().map_err(Error::Validation)?;
        if self.repo().count_by_product(input.product_id, 0).await? > 0 {
            return Err(Error::biz(500, MSG_PRODUCT_EXISTS));
        }
        self.check_gift_eligibility(input.product_id, input.sku_id, input.gift_stock)
            .await?;
        let entity = ProductGift {
            gift_id: 0,
            gift_name: input.gift_name,
            gift_stock: input.gift_stock,
            product_id: input.product_id,
            sku_id: input.sku_id.unwrap_or(0),
            // 多租户：店铺账号强制本店；平台账号落 0
            shop_id: TenantScope::current().write_shop_id(None)?,
        };
        let id = self.repo().insert(&entity).await?;
        tracing::info!("创建活动赠品: id={id}");
        Ok(id)
    }

    /// 更新（Java `update`）。
    pub async fn update(&self, dto: &ProductGiftUpdateDTO) -> Result<()> {
        let input = dto.validate().map_err(Error::Validation)?;
        let existing = self
            .repo()
            .find_by_id(input.gift_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))?;
        // 多租户：更新前校验归属
        TenantScope::current_ensure_access(existing.shop_id)?;
        let existing_shop = existing.shop_id;
        let product_id = input.product_id.unwrap_or(existing.product_id);
        let sku_id = input.sku_id.unwrap_or(existing.sku_id);
        let gift_stock = input.gift_stock.unwrap_or(existing.gift_stock);
        if self
            .repo()
            .count_by_product(product_id, input.gift_id)
            .await?
            > 0
        {
            return Err(Error::biz(500, MSG_PRODUCT_EXISTS));
        }
        self.check_gift_eligibility(product_id, Some(sku_id), gift_stock)
            .await?;
        let mut entity = existing;
        entity.product_id = product_id;
        entity.sku_id = sku_id;
        entity.gift_stock = gift_stock;
        if let Some(name) = input.gift_name {
            entity.gift_name = name;
        }
        // 多租户：店铺账号强制本店；平台账号保留原店
        entity.shop_id = TenantScope::current().write_shop_id(Some(existing_shop))?;
        self.repo().update(&entity).await?;
        Ok(())
    }

    /// 删除（Java `del`）。
    pub async fn delete(&self, dto: &OperateDTO) -> Result<()> {
        let id = dto.validate_id().map_err(Error::Validation)?;
        // 多租户：删除前校验归属
        if let Some(entity) = self.repo().find_by_id(id).await? {
            TenantScope::current_ensure_access(entity.shop_id)?;
        }
        let _ = self.repo().delete(id).await?;
        Ok(())
    }

    /// 字段更新（本表白名单恒无效，Java 同）。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<()> {
        let _ = dto.validate().map_err(Error::Validation)?;
        Ok(())
    }

    /// 批量操作（仅 del）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if dto.r#type.as_deref() == Some("del") {
            let ids = dto.ids.as_deref().unwrap_or_default();
            // 多租户：批量删除前逐条校验归属（越权直接 403）
            for id in ids {
                if let Some(entity) = self.repo().find_by_id(*id).await? {
                    TenantScope::current_ensure_access(entity.shop_id)?;
                }
            }
            let n = self.repo().delete_batch(ids).await?;
            return Ok(n > 0);
        }
        Ok(false)
    }

    async fn check_gift_eligibility(
        &self,
        product_id: i32,
        sku_id: Option<i32>,
        gift_stock: i32,
    ) -> Result<()> {
        let product = self
            .repo()
            .find_product(product_id)
            .await?
            .ok_or_else(|| Error::biz(500, "商品不存在"))?;
        if let Some(group_id) = product.gift_card_groups_id.filter(|id| *id > 0) {
            if self.repo().find_gift_card_type(group_id).await? == Some(1) {
                return Err(Error::biz(500, MSG_GIFT_CARD_ECARD));
            }
        }
        match sku_id.filter(|id| *id > 0) {
            None => {
                if product.product_stock < gift_stock {
                    return Err(Error::biz(500, MSG_STOCK_EXCEED_PRODUCT));
                }
            }
            Some(sku_id) => {
                let sku = self
                    .repo()
                    .find_sku(sku_id)
                    .await?
                    .filter(|s| s.product_id == product_id && s.is_enabled == 1)
                    .ok_or_else(|| Error::biz(500, MSG_SKU_DISABLED))?;
                if sku.sku_stock.unwrap_or(0) < gift_stock {
                    return Err(Error::biz(500, MSG_STOCK_EXCEED_SKU));
                }
            }
        }
        Ok(())
    }
}

fn to_vo(
    entity: &ProductGift,
    products: &HashMap<i32, GiftProductRow>,
    skus: &HashMap<i32, GiftSkuRow>,
) -> ProductGiftVO {
    let product = products.get(&entity.product_id);
    let sku = skus.get(&entity.sku_id);
    ProductGiftVO {
        gift_id: entity.gift_id,
        gift_name: entity.gift_name.clone(),
        gift_stock: entity.gift_stock,
        product_id: entity.product_id,
        sku_id: entity.sku_id,
        shop_id: entity.shop_id,
        product_info: product.map(|p| {
            serde_json::json!({
                "productId": p.product_id,
                "productName": p.product_name,
                "productSn": p.product_sn,
                "productStock": p.product_stock,
                "productPrice": p.product_price,
                "marketPrice": p.market_price,
                "picThumb": p.pic_thumb,
                "picUrl": p.pic_url,
            })
        }),
        sku_info: sku.map(|s| {
            serde_json::json!({
                "skuId": s.sku_id,
                "productId": s.product_id,
                "skuPrice": s.sku_price,
                "skuStock": s.sku_stock,
                "isEnabled": s.is_enabled,
            })
        }),
    }
}

fn distinct(ids: impl Iterator<Item = i32>) -> Vec<i32> {
    let mut out: Vec<i32> = Vec::new();
    for id in ids {
        if !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

fn to_page_query(page: Option<i64>, size: Option<i64>) -> nextrade_core::types::PageQuery {
    let (size, offset) = ProductGiftRepository::page_window(page, size);
    let page_no = if size > 0 { offset / size + 1 } else { 1 };
    nextrade_core::types::PageQuery {
        page: u64::try_from(page_no.max(1)).unwrap_or(1),
        page_size: u64::try_from(size.max(1)).unwrap_or(15),
    }
}

/// 当前请求的店铺 ID：店铺账号=本店，平台账号=0（Java HeaderUtils.getShopId() 同义）。
fn scope_shop_id() -> i32 {
    TenantScope::current().shop_id().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::marketing::product_gift::{
        MSG_GIFT_NAME_REQUIRED, MSG_STOCK_REQUIRED,
    };

    #[test]
    fn create_validation_wired() {
        assert_eq!(
            ProductGiftCreateDTO::default().validate().unwrap_err(),
            MSG_GIFT_NAME_REQUIRED
        );
        let dto = ProductGiftCreateDTO {
            gift_name: Some("赠品".into()),
            gift_stock: Some(0),
            ..Default::default()
        };
        assert_eq!(dto.validate().unwrap_err(), MSG_STOCK_REQUIRED);
    }

    #[test]
    fn page_conversion() {
        let p = to_page_query(None, None);
        assert_eq!((p.page, p.page_size), (1, 15));
        let p = to_page_query(Some(2), Some(10));
        assert_eq!((p.page, p.page_size), (2, 10));
        assert_eq!(distinct([2, 1, 2].into_iter()), vec![2, 1]);
    }

    #[tokio::test]
    async fn tenant_scope_selects_shop_or_platform_scope_for_product_gift() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        let shop = TenantScope::for_shop(8);
        let (scope, write_shop, blocked) = with_tenant_scope(shop, async {
            (
                scope_shop_id(),
                TenantScope::current().write_shop_id(None).unwrap_or(-1),
                TenantScope::current_ensure_access(9).is_err(),
            )
        })
        .await;
        assert_eq!(scope, 8);
        assert_eq!(write_shop, 8);
        assert!(blocked);
        let scope = with_tenant_scope(TenantScope::platform(), async { scope_shop_id() }).await;
        assert_eq!(scope, 0);
    }
}
