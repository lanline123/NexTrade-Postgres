//! 积分商品域服务（marketing::points_exchange）。
//!
//! Java `PointsExchangeController`（7 端点）→ `PointsExchangeServiceImpl`。
//! 列表/详情回填商品信息与 SKU 单价，`discountsPrice = productPrice - pointsDeductedAmount`。
//!
//! # 偏差
//!
//! 1. `detail` 不存在 ID：Java NPE（500）⇒ Rust 业务 500「积分商品不存在」。
//! 2. `costPrice` Java 未赋值 ⇒ 恒 null（Rust 同）。
//! 3. `list` 无关键字时用 `EXISTS(product 上架未删除)` 等价 Java innerJoin。
//! 4. `update` 未传字段按 NOT_NULL 语义保留旧值（Java `updateById` 语义）。

//! # 多租户（N/A 判定）
//!
//! 本域为**平台级数据**，不做店铺隔离（N/A），证据：
//! - `java/database/pure.sql` 中 `points_exchange` 表**没有 `shop_id` 列**；
//! - 原 Java `PointsExchangeServiceImpl` 中**没有任何** `getShopId()` / `shop_id` 引用；
//! - 因此管理端列表/详情/写操作均以平台身份执行，接入 `TenantScope`
//!   不会改变行为（平台作用域默认放行，店铺账号访问时按平台数据可见性处理）。
//!
//! 若未来产品要求店铺级隔离，需先给 `points_exchange` 增加 `shop_id` 列并回填归属。
//!
use std::sync::Arc;

use nextrade_core::entities::promotion::PointsExchange;
use nextrade_core::models::marketing::points_exchange::{
    BatchDTO, OperateDTO, PointsExchangeCreateDTO, PointsExchangeListQuery, PointsExchangeUpdateDTO,
    PointsExchangeVO, UpdateFieldDTO, MSG_DUPLICATE, MSG_NOT_EXISTS, MSG_SKU_DISABLED,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::points_exchange::PointsExchangeRepository;
use nextrade_infrastructure::db::Repositories;

/// 积分商品域服务。
pub struct PointsExchangeService {
    repos: Arc<Repositories>,
}

impl PointsExchangeService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &PointsExchangeRepository {
        &self.repos.marketing.points_exchange
    }

    /// 列表（Java `list`）。
    pub async fn list(
        &self,
        q: &PointsExchangeListQuery,
    ) -> Result<PageResult<PointsExchangeVO>> {
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let product_ids = match q.effective_keyword() {
            Some(keyword) => {
                let ids = self.repo().find_valid_product_ids_by_name(keyword).await?;
                if ids.is_empty() {
                    return Ok(PageResult::empty(&page));
                }
                Some(ids)
            }
            None => None,
        };
        let (total, rows) = self
            .repo()
            .paginate(q, product_ids.as_deref(), limit, offset)
            .await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let list = self.enrich(&rows).await?;
        Ok(PageResult::new(u64::try_from(total).unwrap_or(0), list, &page))
    }

    /// 详情（Java `detail`）。
    pub async fn detail(&self, id: i32) -> Result<PointsExchangeVO> {
        let entity = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))?;
        let mut list = self.enrich(&[entity]).await?;
        list.pop().ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))
    }

    /// 创建（Java `create`）。
    pub async fn create(&self, dto: &PointsExchangeCreateDTO) -> Result<i32> {
        let input = dto.validate().map_err(Error::Validation)?;
        if self.repo().count_by_product(input.product_id, 0).await? > 0 {
            return Err(Error::biz(500, MSG_DUPLICATE));
        }
        self.check_sku(input.product_id, input.sku_id).await?;
        let entity = PointsExchange {
            id: 0,
            product_id: input.product_id,
            exchange_integral: input.exchange_integral,
            points_deducted_amount: input.points_deducted_amount,
            is_hot: input.is_hot,
            is_enabled: input.is_enabled,
            sku_id: input.sku_id,
        };
        let id = self.repo().insert(&entity).await?;
        tracing::info!("创建积分商品: id={id}");
        Ok(id)
    }

    /// 更新（Java `update`）。
    pub async fn update(&self, dto: &PointsExchangeUpdateDTO) -> Result<()> {
        let input = dto.validate().map_err(Error::Validation)?;
        let existing = self.repo().find_by_id(input.id).await?;
        if self
            .repo()
            .count_by_product(input.product_id, input.id)
            .await?
            > 0
        {
            return Err(Error::biz(500, MSG_DUPLICATE));
        }
        self.check_sku(input.product_id, input.sku_id).await?;
        let mut entity = existing.unwrap_or_default();
        entity.id = input.id;
        entity.product_id = input.product_id;
        entity.exchange_integral = input.exchange_integral;
        if let Some(v) = input.points_deducted_amount {
            entity.points_deducted_amount = v;
        }
        if let Some(v) = input.is_hot {
            entity.is_hot = v;
        }
        if let Some(v) = input.is_enabled {
            entity.is_enabled = v;
        }
        entity.sku_id = input.sku_id;
        self.repo().update(&entity).await?;
        tracing::info!("更新积分商品: id={}", input.id);
        Ok(())
    }

    /// 删除（Java `del`）。
    pub async fn delete(&self, dto: &OperateDTO) -> Result<()> {
        let id = dto.validate_id().map_err(Error::Validation)?;
        let _ = self.repo().delete(id).await?;
        Ok(())
    }

    /// 字段更新（Java `updateField(dto, POINTS_EXCHANGE_FIELDS)`）。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<()> {
        let input = dto.validate().map_err(Error::Validation)?;
        self.repo()
            .update_field(input.id, input.column, &input.val)
            .await?;
        Ok(())
    }

    /// 批量操作（Java `BaseServiceImpl#batch`：仅 del）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if dto.r#type.as_deref() == Some("del") {
            let n = self
                .repo()
                .delete_batch(dto.ids.as_deref().unwrap_or_default())
                .await?;
            return Ok(n > 0);
        }
        Ok(false)
    }

    async fn check_sku(&self, product_id: i32, sku_id: Option<i32>) -> Result<()> {
        if let Some(sku_id) = sku_id.filter(|id| *id > 0) {
            if !self.repo().sku_enabled(product_id, sku_id).await? {
                return Err(Error::biz(500, MSG_SKU_DISABLED));
            }
        }
        Ok(())
    }

    async fn enrich(&self, rows: &[PointsExchange]) -> Result<Vec<PointsExchangeVO>> {
        let product_ids = distinct(rows.iter().map(|r| r.product_id));
        let sku_ids = distinct(rows.iter().filter_map(|r| r.sku_id).filter(|id| *id > 0));
        let products = PointsExchangeRepository::index_products(self.repo().find_products(&product_ids).await?);
        let skus = PointsExchangeRepository::index_skus(self.repo().find_skus(&sku_ids).await?);
        Ok(rows
            .iter()
            .map(|entity| {
                let product = products.get(&entity.product_id);
                let sku = entity
                    .sku_id
                    .filter(|id| *id > 0)
                    .and_then(|id| skus.get(&id));
                let product_price = sku
                    .and_then(|s| s.sku_price)
                    .or_else(|| product.map(|p| p.product_price));
                PointsExchangeVO {
                    id: entity.id,
                    product_id: entity.product_id,
                    exchange_integral: entity.exchange_integral,
                    points_deducted_amount: entity.points_deducted_amount,
                    is_hot: entity.is_hot,
                    is_enabled: entity.is_enabled,
                    sku_id: entity.sku_id,
                    product_name: product.map(|p| p.product_name.clone()),
                    pic_thumb: product.map(|p| p.pic_thumb.clone()),
                    pic_url: product.map(|p| p.pic_url.clone()),
                    market_price: product.map(|p| p.market_price),
                    cost_price: None,
                    product_price,
                    discounts_price: Some(
                        product_price.unwrap_or_default() - entity.points_deducted_amount,
                    ),
                    product_stock: product.map(|p| p.product_stock),
                    virtual_sales: product.map(|p| p.virtual_sales),
                    product_sn: product.map(|p| p.product_sn.clone()),
                }
            })
            .collect())
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
    let (size, offset) = PointsExchangeRepository::page_window(page, size);
    let page_no = if size > 0 { offset / size + 1 } else { 1 };
    nextrade_core::types::PageQuery {
        page: u64::try_from(page_no.max(1)).unwrap_or(1),
        page_size: u64::try_from(size.max(1)).unwrap_or(15),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::marketing::points_exchange::MSG_INTEGRAL_INVALID;

    #[test]
    fn create_validation_wired() {
        assert_eq!(
            PointsExchangeCreateDTO::default().validate().unwrap_err(),
            nextrade_core::models::marketing::points_exchange::MSG_PRODUCT_REQUIRED
        );
        let dto = PointsExchangeCreateDTO {
            product_id: Some(1),
            exchange_integral: Some(0),
            ..Default::default()
        };
        assert_eq!(dto.validate().unwrap_err(), MSG_INTEGRAL_INVALID);
    }

    #[test]
    fn page_conversion_and_distinct() {
        let p = to_page_query(None, None);
        assert_eq!((p.page, p.page_size), (1, 15));
        let p = to_page_query(Some(2), Some(10));
        assert_eq!((p.page, p.page_size), (2, 10));
        assert_eq!(distinct([3, 1, 3].into_iter()), vec![3, 1]);
    }
}
