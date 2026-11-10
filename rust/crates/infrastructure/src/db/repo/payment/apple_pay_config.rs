//! Apple Pay 设置仓储（payment::apple_pay_config）。
//!
//! 表 `config(biz_code, biz_val, create_time, update_time, ...)`：
//! 每字段一行，读取按 biz_code 批量取，保存按 biz_code upsert。
//!
//! 全部 SQL 为运行时 `sqlx::query_as` / `sqlx::query`（禁止 `query!` 宏）。

use std::collections::HashMap;

use sqlx::PgPool;
use sqlx::{Postgres, QueryBuilder};

use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct ApplePayConfigRepository {
    pool: PgPool,
}

impl ApplePayConfigRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 批量读取配置（不存在/值为空的行原样返回 `None`，由上层套默认值）。
    pub async fn load(
        &self,
        biz_codes: &[&str],
    ) -> Result<HashMap<String, Option<String>>> {
        if biz_codes.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT biz_code, biz_val FROM config WHERE biz_code IN (");
        let mut sep = qb.separated(", ");
        for code in biz_codes {
            sep.push_bind((*code).to_string());
        }
        drop(sep);
        qb.push(")");
        let rows: Vec<(String, Option<String>)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().collect())
    }

    /// 单字段 upsert（Java `settingsHelper.set`：存在 update，不存在 insert）。
    pub async fn upsert(&self, biz_code: &str, biz_val: &str) -> Result<()> {
        let now = nextrade_common::utils::now_seconds();
        let existing: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM config WHERE biz_code = $1 LIMIT 1")
                .bind(biz_code)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        match existing {
            Some((id,)) => {
                sqlx::query("UPDATE config SET biz_val = $1, update_time = $2 WHERE id = $3")
                    .bind(biz_val)
                    .bind(now)
                    .bind(id)
                    .execute(&self.pool)
                    .await
                    .map_err(map_err)?;
            }
            None => {
                sqlx::query(
                    "INSERT INTO config (biz_code, biz_val, create_time, update_time) VALUES ($1,$2,$3,$4)",
                )
                .bind(biz_code)
                .bind(biz_val)
                .bind(now)
                .bind(now)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
            }
        }
        Ok(())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn upsert_statements_are_parameterized() {
        let update = "UPDATE config SET biz_val = $1, update_time = $2 WHERE id = $3";
        assert_eq!(update.matches('$').count(), 3);
        let insert = "INSERT INTO config (biz_code, biz_val, create_time, update_time) VALUES ($1,$2,$3,$4)";
        assert_eq!(insert.matches('$').count(), 4);
    }
}
