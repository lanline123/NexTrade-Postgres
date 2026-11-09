//! 店铺保证金设置 域仓储（merchant::deposit_setting）—— wave9。
//!
//! 表 `config`（`biz_code` / `biz_val` / `is_del`），语义同 system 域：
//! 读取过滤 `is_del`，保存「先删后插」事务。运行时 sqlx。

use sqlx::PgPool;

use nextrade_core::models::merchant::deposit_setting::CODES;
use nextrade_core::{Error, Result};

const ALIVE: &str = "(is_del IS NULL OR is_del = 0)";

/// 保证金设置仓储。
#[derive(Clone)]
pub struct DepositSettingRepository {
    pool: PgPool,
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

impl DepositSettingRepository {
    /// 构造仓储。
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 读取全部设置项。
    pub async fn find_settings(&self) -> Result<Vec<(String, String)>> {
        let marks = vec!["?"; CODES.len()].join(", ");
        let sql = format!("SELECT biz_code, biz_val FROM config WHERE biz_code IN ({marks}) AND {ALIVE}");
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, (String, Option<String>)>(&sql);
        for code in CODES {
            q = q.bind(code);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows
            .into_iter()
            .filter_map(|(c, v)| v.map(|v| (c, v)))
            .collect())
    }

    /// 保存设置项（事务内先删后插）。
    pub async fn save_settings(&self, pairs: &[(String, String)]) -> Result<()> {
        if pairs.is_empty() {
            return Ok(());
        }
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        let now = nextrade_common::utils::now_millis();
        for (code, value) in pairs {
            sqlx::query("DELETE FROM config WHERE biz_code = $1")
                .bind(code)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
            sqlx::query(
                "INSERT INTO config (biz_code, biz_val, create_time, update_time, is_del) \
                 VALUES ($1, $2, $3, $4, 0)",
            )
            .bind(code)
            .bind(value)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        }
        tx.commit().await.map_err(map_err)?;
        Ok(())
    }
}
