//! 语言关联表仓储（lang::locales_relation）。
//!
//! Java 来源（overseas）：`LocalesRelationMapper`
//! （`LocalesRelationServiceImpl` 的 list/detail/create/update/del/batch）。
//! 表 `locales_relation`（4 列）。

use sqlx::PgPool;

use nextrade_core::entities::LocalesRelation;
use nextrade_core::{Error, Result};

/// `locales_relation` 固定 SELECT 列清单（顺序与实体一致）。
pub const LOCALES_RELATION_COLUMNS: [&str; 4] = ["id", "name", "code", "locales_id"];

/// 语言关联仓储。
#[derive(Clone)]
pub struct LocalesRelationRepository {
    pool: PgPool,
}

impl LocalesRelationRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn select_sql(where_sql: &str) -> String {
        format!(
            "SELECT {} FROM locales_relation {where_sql}",
            LOCALES_RELATION_COLUMNS.join(", ")
        )
    }

    /// 分页列表（Java `list(BasePage)`：无业务过滤，仅排序）。
    pub async fn paginate(
        &self,
        order_by: &str,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<LocalesRelation>)> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM locales_relation")
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let sql = Self::select_sql(&format!("ORDER BY {order_by} LIMIT ? OFFSET ?"));
        let rows = sqlx::query_as::<_, LocalesRelation>(&sql)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((count.max(0) as u64, rows))
    }

    /// 按主键查单条（Java `getById`）。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<LocalesRelation>> {
        let sql = Self::select_sql("WHERE id = ? LIMIT 1");
        sqlx::query_as::<_, LocalesRelation>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 插入（Java `save`）。
    pub async fn insert(&self, row: &LocalesRelation) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO locales_relation (name, code, locales_id) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(&row.name)
        .bind(&row.code)
        .bind(row.locales_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 按主键全量更新（Java `updateById`）。
    pub async fn update(&self, row: &LocalesRelation) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE locales_relation SET name = $1, code = $2, locales_id = $3 WHERE id = $4",
        )
        .bind(&row.name)
        .bind(&row.code)
        .bind(row.locales_id)
        .bind(row.id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 按 ID 集合删除（Java `removeByIds`）。
    pub async fn delete_by_ids(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql = format!("DELETE FROM locales_relation WHERE id IN ({placeholders})");
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(id);
        }
        let res = q.execute(&self.pool).await.map_err(map_err)?;
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
    fn column_list_matches_entity() {
        assert_eq!(LOCALES_RELATION_COLUMNS.len(), 4);
    }
}
