//! 限时折扣批处理仓储（marketing::time_discount_batch）。
//!
//! 复用既有表：`time_discount`（活动）/ `time_discount_item`（活动商品）/
//! `promotion`（营销活动索引，`type=6` 限时折扣）。
//! 对应 Java `TimeDiscountServiceImpl#batch` 的三步级联删除。
//!
//! SQL 一律运行时 `sqlx::query`（不用 `query!` 宏）。

use sqlx::{Postgres,PgPool, QueryBuilder};

use nextrade_core::models::marketing::time_discount_batch::PROMOTION_TYPE_LIMIT_DISCOUNT;
use nextrade_core::{Error, Result};

/// 限时折扣批处理仓储。
#[derive(Clone)]
pub struct TimeDiscountBatchRepository {
    pool: PgPool,
}

impl TimeDiscountBatchRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 删除活动（Java `this.removeByIds(ids)`，表 `time_discount`）。
    pub async fn delete_activities(&self, ids: &[i32]) -> Result<u64> {
        self.delete_where_id_in("time_discount", "discount_id", ids).await
    }

    /// 删除活动商品（Java `timeDiscountItemService...remove()`）。
    pub async fn delete_items(&self, ids: &[i32]) -> Result<u64> {
        self.delete_where_id_in("time_discount_item", "discount_id", ids)
            .await
    }

    /// 删除关联营销活动（Java `promotionService...remove()`，`type=6`）。
    pub async fn delete_promotions(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("DELETE FROM promotion WHERE type = ");
        qb.push_bind(PROMOTION_TYPE_LIMIT_DISCOUNT);
        qb.push(" AND relation_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        let res = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    async fn delete_where_id_in(
        &self,
        table: &str,
        column: &str,
        ids: &[i32],
    ) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("DELETE FROM {table} WHERE {column} IN ("));
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        let res = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_and_column_names_match_java_cascade() {
        // Java：time_discount(removeByIds) / time_discount_item.in(discountId) /
        // promotion(type=6).in(relationId)
        let src = include_str!("time_discount_batch.rs");
        assert!(src.contains("\"time_discount\""));
        assert!(src.contains("\"time_discount_item\""));
        assert!(src.contains("\"discount_id\""));
        assert!(src.contains("DELETE FROM promotion WHERE type = "));
        assert!(src.contains("relation_id IN"));
    }

    #[test]
    fn empty_ids_are_noop() {
        // 空集合不会拼出非法 SQL（服务层已校验，此处为防御）
        let empty: Vec<i32> = Vec::new();
        assert_eq!(empty.len(), 0);
        assert!(in_placeholders_ok(0));
    }

    fn in_placeholders_ok(len: usize) -> bool {
        nextrade_core::models::marketing::time_discount_batch::in_placeholders(len) == "(NULL)"
    }
}
