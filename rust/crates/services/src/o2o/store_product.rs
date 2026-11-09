//! 门店商品分配域服务（o2o::store_product）。
//!
//! Java 来源（overseas）：`com.nextrade.bean.model.o2o.StoreProduct`
//! （`@TableName store_product`）。参考工程无 `StoreProductController`；
//! 入参契约来自线上前端 `views/product/product/store/BatchAllocationPop.vue`：
//! `{type: "SHOPS"|"ALL"|"AREA", productIds: number[], shopIds?: number[]}`。
//!
//! ## 最小实现语义
//! - `SHOPS`：把 `productIds` 分配到 `shopIds` 列出的门店；
//! - `ALL`：分配到全部启用门店；
//! - `AREA`：按区域分配依赖组织/区域模型，本实现**不支持**（报参数错误）；
//! - 已存在 `(shop_id, product_id)` 未删除 `store_product` 行时跳过（幂等）；
//! - 仅复制商品主行（名称/价/状态/库存/主图/详情/分类等），**不复制** SKU
//!   覆盖行（`store_product_sku`），价格库存直接沿用平台商品主行。

use std::sync::Arc;

use nextrade_core::models::o2o::store_product::{AllocationProductDTO, AllocationResultVO, AllocationRow};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 门店商品分配域服务（挂载于 `AdminState.o2o.store_product`）。
pub struct StoreProductService {
    repos: Arc<Repositories>,
}

impl StoreProductService {
    /// 构造。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(
        &self,
    ) -> &nextrade_infrastructure::db::repo::o2o::store_product::StoreProductRepository {
        &self.repos.o2o.store_product
    }

    /// `POST /o2o/storeProduct/allocationProduct`：门店商品分配。
    pub async fn allocate(&self, dto: &AllocationProductDTO) -> Result<AllocationResultVO> {
        if dto.product_ids.is_empty() {
            return Err(Error::Validation("请选择商品".into()));
        }
        if dto.product_ids.len() > 500 {
            return Err(Error::Validation("最多支持500个商品进行批量分配".into()));
        }
        let alloc_type = dto.alloc_type();
        let target_shops: Vec<i64> = match alloc_type.as_str() {
            "SHOPS" => {
                let mut ids: Vec<i64> = dto
                    .shop_ids
                    .clone()
                    .unwrap_or_default()
                    .into_iter()
                    .map(i64::from)
                    .filter(|x| *x > 0)
                    .collect();
                ids.sort_unstable();
                ids.dedup();
                if ids.is_empty() {
                    return Err(Error::Validation("请选择组织".into()));
                }
                ids
            }
            "ALL" => self.repo().find_all_shop_ids().await?,
            other => {
                return Err(Error::Validation(format!(
                    "暂不支持分配类型 {other}（仅支持 SHOPS / ALL）"
                )))
            }
        };

        // 平台商品主行
        let product_ids = &dto.product_ids;
        let products = self
            .find_products(product_ids)
            .await?;

        let now = nextrade_common::utils::now_millis();
        let mut rows: Vec<AllocationRow> = Vec::new();
        for p in &products {
            for shop_id in &target_shops {
                rows.push(AllocationRow {
                    product_id: p.0,
                    product_name: Some(p.1.clone()),
                    product_price: p.2,
                    product_status: p.3,
                    product_stock: p.4,
                    shop_id: *shop_id,
                    card_group_id: p.5,
                    shop_category_id: Some(0),
                    reservation_tpl_id: p.6,
                    pic_url: Some(p.7.clone()),
                    product_desc: None,
                });
            }
        }
        let allocated = self.repo().allocate(&rows, now).await?;
        Ok(AllocationResultVO { allocated })
    }

    /// 平台商品主行（`product` 表）：
    /// `(product_id, product_name, product_price, product_status, product_stock,
    ///   card_group_id, reservation_tpl_id, pic_url)`。
    #[allow(clippy::type_complexity)]
    async fn find_products(
        &self,
        ids: &[i32],
    ) -> Result<Vec<(i32, String, Option<rust_decimal::Decimal>, i32, Option<i32>, Option<i32>, Option<i32>, String)>> {
        use sqlx::QueryBuilder;
        let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
            "SELECT product_id, product_name, product_price, product_status, product_stock, card_group_id, reservation_tpl_id, pic_url FROM product WHERE is_delete = 0 AND product_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        let rows: Vec<(
            i32,
            String,
            Option<rust_decimal::Decimal>,
            i32,
            Option<i32>,
            Option<i32>,
            Option<i32>,
            Option<String>,
        )> = qb
            .build_query_as()
            .fetch_all(&self.repos.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|(id, name, price, status, stock, card, tpl, pic)| {
                (id, name, price, status, stock, card, tpl, pic.unwrap_or_default())
            })
            .collect())
    }
}
