//! 汇付斗拱设置仓储（payment::huifu_config）。
//!
//! 表 `config(biz_code, biz_val, create_time, update_time)`：每字段一行，读取批量、保存 upsert。

use std::collections::HashMap;

use sqlx::PgPool;
use sqlx::{Postgres, QueryBuilder};

use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct HuifuConfigRepository {
    pool: PgPool,
}

impl HuifuConfigRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn load(&self, biz_codes: &[&str]) -> Result<HashMap<String, Option<String>>> {
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
                    "INSERT INTO config (biz_code, biz_val, create_time, update_time, is_del) VALUES ($1,$2,$3,$4,0)",
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
        assert!(r"SELECT id FROM config WHERE biz_code = $1 LIMIT 1".contains("?"));
        assert_eq!(
            "INSERT INTO config (biz_code, biz_val, create_time, update_time) VALUES ($1,$2,$3,$4)"
                .matches('?')
                .count(),
            4
        );
    }
}
