//! 门店商品（`store_product`）仓储 —— 商品分配落库。
//!
//! 对应原 Java（overseas）：`com.nextrade.mapper.o2o.StoreProductMapper`
//! （BaseMapper；参考工程未提供 `StoreProductController`，本仓储按
//! `store_product` 真实 schema + 前端契约实现最小分配语义）。
//!
//! 列名对齐 `crates/core/src/entities/o2o.rs::StoreProduct`。

use rust_decimal::Decimal;
use sqlx::{PgPool, QueryBuilder};

use nextrade_core::entities::o2o::StoreProduct;
use nextrade_core::{Error, Result};

/// `store_product` 固定 SELECT 列。
const STORE_PRODUCT_SELECT: &str = "SELECT store_product_id, product_id, product_name, product_price, product_status, product_stock, shop_id, add_time, card_group_id, is_delete, shop_category_id, reservation_tpl_id, delivery_service_fee_type, delivery_service_fee_value, server_service_fee_type, server_service_fee_value, pic_url, product_desc FROM store_product";

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct StoreProductRepository {
    pool: PgPool,
}

impl StoreProductRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 按主键批量取门店商品。
    pub async fn find_by_ids(&self, ids: &[i64]) -> Result<Vec<StoreProduct>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new(format!("{STORE_PRODUCT_SELECT} WHERE store_product_id IN ("));
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        let rows: Vec<StoreProduct> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows)
    }

    /// 商品 id → 已分配门店 id 集合。
    pub async fn find_allocated_shop_ids(&self, product_ids: &[i32]) -> Result<Vec<(i32, i64)>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
            "SELECT product_id, shop_id FROM store_product WHERE is_delete = 0 AND product_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in product_ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        let rows: Vec<(i32, i64)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows)
    }

    /// 为商品分配门店（存在同 `(shop_id, product_id)` 未删除行时跳过）。
    pub async fn allocate(
        &self,
        rows: &[nextrade_core::models::o2o::store_product::AllocationRow],
        now: i64,
    ) -> Result<u64> {
        let mut affected: u64 = 0;
        for row in rows {
            let exist: Option<(i64,)> = sqlx::query_as(
                "SELECT store_product_id FROM store_product WHERE shop_id = $1 AND product_id = $2 AND is_delete = 0 LIMIT 1",
            )
            .bind(row.shop_id)
            .bind(row.product_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?;
            if exist.is_some() {
                continue;
            }
            let out = sqlx::query(
                "INSERT INTO store_product (product_id, product_name, product_price, product_status, product_stock, shop_id, add_time, card_group_id, is_delete, shop_category_id, reservation_tpl_id, delivery_service_fee_type, delivery_service_fee_value, server_service_fee_type, server_service_fee_value, pic_url, product_desc) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, $9, $10, 0, 0.00, 0, 0.00, $11, $12)",
            )
            .bind(row.product_id)
            .bind(&row.product_name)
            .bind(row.product_price)
            .bind(row.product_status)
            .bind(row.product_stock)
            .bind(row.shop_id)
            .bind(now)
            .bind(row.card_group_id)
            .bind(row.shop_category_id)
            .bind(row.reservation_tpl_id)
            .bind(&row.pic_url)
            .bind(&row.product_desc)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
            affected += out.rows_affected();
        }
        Ok(affected)
    }

    /// 全部门店 id（`type = ALL` 分配用；仅启用店铺，最多 2000）。
    pub async fn find_all_shop_ids(&self) -> Result<Vec<i64>> {
        let rows: Vec<(i64,)> =
            sqlx::query_as("SELECT shop_id FROM shop WHERE is_delete = 0 LIMIT 2000")
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }
}

/// SQLx 错误 → 统一错误类型。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("门店商品不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

/// `Decimal` 便捷重导（仓储签名使用；避免服务层直接依赖 sqlx 类型路径）。
pub type Money = Decimal;
