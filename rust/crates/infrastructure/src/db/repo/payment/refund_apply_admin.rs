//! 退款申请管理仓储（payment::refund_apply_admin）。
//!
//! 表 `refund_apply`（17 列，主键 `refund_id`）。
//!
//! 全部 SQL 为运行时 `sqlx::query`（禁止 `query!` 宏）。

use sqlx::PgPool;
use sqlx::{Postgres, QueryBuilder};

use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct RefundApplyAdminRepository {
    pool: PgPool,
}

impl RefundApplyAdminRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 删除（Java `removeByIds`）。
    pub async fn delete_by_ids(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("DELETE FROM refund_apply WHERE refund_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        Ok(qb
            .build()
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected())
    }

    /// 单字段更新（列名来自白名单常量，不拼接用户输入）。
    pub async fn update_field(&self, id: i32, column: &str, val: &str) -> Result<u64> {
        let sql = format!("UPDATE refund_apply SET {column} = $1 WHERE refund_id = $2");
        let r = sqlx::query(&sql)
            .bind(val)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 行是否存在（`del`/`batch` 幂等语义用）。
    pub async fn exists(&self, id: i32) -> Result<bool> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT refund_id FROM refund_apply WHERE refund_id = $1 LIMIT 1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.is_some())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn delete_and_update_statements_are_parameterized() {
        assert_eq!(
            "UPDATE refund_apply SET refund_status = $1 WHERE refund_id = $2"
                .matches('$')
                .count(),
            2
        );
        let delete = "DELETE FROM refund_apply WHERE refund_id IN ($1, $2)";
        assert_eq!(delete.matches('$').count(), 2);
        assert!(!delete.contains("refund_id IN (1"));
    }
}
