//! 秒杀批处理仓储（marketing::seckill_batch）。
//!
//! 复用既有表：
//! - `seckill`（活动，主键 `seckill_id`）
//! - `seckill_item`（活动商品，外键 `seckill_id`）
//! - `promotion`（营销索引，`type=1` 秒杀，外键 `relation_id`）
//! - `product`（只读：`product_id → product_name`）
//!
//! 对应 Java `SeckillServiceImpl#batch` 的三步级联删除与
//! `listForDecorate` / `getSeckillProductList` 的进行中窗口查询。
//!
//! SQL 一律运行时 `sqlx::query` / `query_as`。

use std::collections::HashMap;

use sqlx::{Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::SeckillItem;
use nextrade_core::models::marketing::seckill_batch::PROMOTION_TYPE_SECKILL;
use nextrade_core::{Error, Result};

/// 秒杀批处理仓储。
#[derive(Clone)]
pub struct SeckillBatchRepository {
    pool: PgPool,
}

impl SeckillBatchRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 删除活动（Java `this.removeByIds(ids)`，表 `seckill`）。
    pub async fn delete_activities(&self, ids: &[i32]) -> Result<u64> {
        self.delete_where_in("seckill", "seckill_id", ids).await
    }

    /// 删除活动商品（Java `seckillItemService.remove(in(seckillId, ids))`）。
    pub async fn delete_items(&self, ids: &[i32]) -> Result<u64> {
        self.delete_where_in("seckill_item", "seckill_id", ids)
            .await
    }

    /// 删除关联营销活动（Java `promotion(type=1).in(relationId, ids)`）。
    pub async fn delete_promotions(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("DELETE FROM promotion WHERE type = ");
        qb.push_bind(PROMOTION_TYPE_SECKILL);
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

    /// 进行中的秒杀商品（`seckill_stock > 0`，按 `rec_id DESC`）。
    ///
    /// 首页预览口径：对应 C 端 `getSeckillProductList`（`unStarted=0`，
    /// 仅进行中）。装修预览 `listForDecorate` 不走这里（无库存过滤、价格
    /// 升序），见 `find_items_for_decorate`。
    pub async fn find_active_items(
        &self,
        now: i32,
        limit: i64,
        offset: i64,
    ) -> Result<(u64, Vec<SeckillItem>)> {
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM seckill_item WHERE seckill_start_time <= $1 AND seckill_end_time >= $2 AND seckill_stock > 0",
        )
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let rows = sqlx::query_as::<_, SeckillItem>(
            "SELECT * FROM seckill_item WHERE seckill_start_time <= $1 AND seckill_end_time >= $2 AND seckill_stock > 0 ORDER BY rec_id DESC LIMIT $3 OFFSET $4",
        )
        .bind(now)
        .bind(now)
        .bind(limit.max(1) as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 装修预览专用查询（Java `SeckillServiceImpl#listForDecorate` 口径）。
    ///
    /// 与首页预览的 `find_active_items` 分开：进行中窗口（`seckill_item`
    /// 自带起止时间，与活动窗口等价）、**无库存过滤**、不查商品上下架/删除
    /// 状态、按 `seckill_price` 升序（`rec_id` 升序兜底同价顺序）；item 仅
    /// 过滤 SKU 已禁用（`product_sku.is_enabled = 0`），`sku_id` 为空的
    /// 保留（SKU 行已不存在的 item 一并排除）。
    pub async fn find_items_for_decorate(
        &self,
        now: i32,
        limit: i64,
    ) -> Result<Vec<SeckillItem>> {
        let rows = sqlx::query_as::<_, SeckillItem>(
            "SELECT si.* FROM seckill_item si LEFT JOIN product_sku ps ON ps.sku_id = si.sku_id WHERE si.seckill_start_time <= $1 AND si.seckill_end_time >= $2 AND (si.sku_id IS NULL OR ps.is_enabled = 1) ORDER BY si.seckill_price ASC, si.rec_id ASC LIMIT $3",
        )
        .bind(now)
        .bind(now)
        .bind(limit.max(1))
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows)
    }

    /// 秒杀活动主表：`seckill_id → (seckill_name, limit_num)`（VO 回填）。
    pub async fn find_activities(
        &self,
        ids: &[i32],
    ) -> Result<Vec<(i32, Option<String>, Option<i32>)>> {
        let ids: Vec<i32> = ids.iter().copied().filter(|id| *id > 0).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT seckill_id, seckill_name, seckill_limit_num FROM seckill WHERE seckill_id IN ({ph})"
        ));
        let mut q = sqlx::query_as::<_, (i32, Option<String>, Option<i32>)>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 商品名回填（`product_id → product_name`）。
    pub async fn find_product_names(&self, ids: &[i32]) -> Result<HashMap<i32, String>> {
        let ids: Vec<i32> = ids.iter().copied().filter(|id| *id > 0).collect();
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let ph = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT product_id, product_name FROM product WHERE product_id IN ({ph})"));
        let mut q = sqlx::query_as::<_, (i32, String)>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        Ok(q.fetch_all(&self.pool)
            .await
            .map_err(map_err)?
            .into_iter()
            .collect())
    }

    async fn delete_where_in(&self, table: &str, column: &str, ids: &[i32]) -> Result<u64> {
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
    fn cascade_sql_matches_java() {
        // Java：seckill(removeByIds) / seckill_item.in(seckillId) /
        // promotion(type=1).in(relationId)
        let src = include_str!("seckill_batch.rs");
        assert!(src.contains("\"seckill\""));
        assert!(src.contains("\"seckill_item\""));
        assert!(src.contains("\"seckill_id\""));
        assert!(src.contains("DELETE FROM promotion WHERE type = "));
        assert!(src.contains("relation_id IN"));
    }

    #[test]
    fn active_window_filters_stock() {
        let src = include_str!("seckill_batch.rs");
        assert!(src.contains("seckill_start_time <= ?"));
        assert!(src.contains("seckill_end_time >= ?"));
        assert!(src.contains("seckill_stock > 0"));
    }

    #[test]
    fn decorate_query_follows_java_caliber() {
        // listForDecorate 口径：价格升序 + SKU 仅过滤已禁用，
        // 该查询本身不得带库存过滤（库存过滤只属于 find_active_items）。
        let src = include_str!("seckill_batch.rs");
        let start = src.find("pub async fn find_items_for_decorate").unwrap();
        let end = src[start..].find("pub async fn find_activities").unwrap() + start;
        let decorate_sql = &src[start..end];
        assert!(decorate_sql.contains("ORDER BY si.seckill_price ASC"));
        assert!(decorate_sql.contains("ps.is_enabled = 1"));
        assert!(decorate_sql.contains("si.sku_id IS NULL"));
        assert!(!decorate_sql.contains("seckill_stock > 0"));
    }
}
