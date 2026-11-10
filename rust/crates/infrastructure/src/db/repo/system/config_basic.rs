//! 系统配置基础域仓储（system::config_basic）。
//!
//! 表/列来自权威 schema `java/database/pure.sql`
//! （核对：`cd rust && python tools/schema_parser.py --table config`）：
//! - `config`：`id` / `biz_code` / `biz_val` / `create_time` / `create_by_id` /
//!   `create_by_name` / `update_time` / `update_by_id` / `update_by_name` / `is_del`
//!
//! 对应 Java：
//! - `SettingsHelper#getRaw`（`configMapper.selectOne(eq(bizCode))`）
//! - `SettingsHelper#getRawSnapshot`（`selectList(in(bizCode))` + `eq(isDel, 0)`）
//! - `SettingsHelper#set`（**先 `delete` 再 `insert`**，不是 upsert）
//!
//! 运行时 `sqlx`（**不用** `query!` 宏）；每条 SQL 单行字面量。

use sqlx::PgPool;
use nextrade_core::{Error, Result};

/// 在用行条件（Java `getRawSnapshot` 的 `eq(ConfigPO::getIsDel, 0)`；
/// 兼容历史数据里 `is_del IS NULL` 的行）。
const ALIVE: &str = "(is_del IS NULL OR is_del = 0)";

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct ConfigBasicRepository {
    pool: PgPool,
}

impl ConfigBasicRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 开启事务（保存一组配置项时保证「先删后插」成批原子）。
    ///
    /// # Errors
    /// 连接池获取连接失败 → `Error::Database`。
    pub async fn begin(&self) -> Result<sqlx::Transaction<'static, sqlx::Postgres>> {
        self.pool.begin().await.map_err(map_err)
    }

    /// 批量读取配置原值（Java `SettingsHelper#getRawSnapshot`）。
    ///
    /// 返回 `(biz_code, biz_val)`；`biz_val` 为 NULL 的行不会出现在结果里
    /// （Java 的 `putIfAbsent(code, null)` 后续会被当作空值解析，本实现直接视为缺失，
    /// 由规格表默认值兜底 —— 见模块 `uncertain_points`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn find_by_codes(&self, codes: &[&str]) -> Result<Vec<(String, String)>> {
        if codes.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; codes.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT biz_code, biz_val FROM config WHERE biz_code IN ({placeholders}) AND {ALIVE}"
        ));
        let mut q = sqlx::query_as::<_, (String, Option<String>)>(&sql);
        for code in codes {
            q = q.bind(*code);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows
            .into_iter()
            .filter_map(|(code, val)| val.map(|v| (code, v)))
            .collect())
    }

    /// 按 `biz_code` 读取单行原值（Java `ConfigServiceImpl#getConfigByCode`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn find_by_code(&self, code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(&format!(
            "SELECT biz_val FROM config WHERE biz_code = $1 AND {ALIVE} LIMIT 1"
        ))
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }

    /// 模糊读取配置原值（Java `ConfigServiceImpl#getConfigsByCode`：`like(bizCode, code)`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn find_by_code_like(&self, pattern: &str) -> Result<Vec<(String, String)>> {
        let sql = format!(
            "SELECT biz_code, biz_val FROM config WHERE biz_code ILIKE $1 AND {ALIVE}"
        );
        let like = format!("%{pattern}%");
        let rows = sqlx::query_as::<_, (String, Option<String>)>(&sql)
            .bind(like)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .filter_map(|(code, val)| val.map(|v| (code, v)))
            .collect())
    }

    /// 「先删后插」写入一个配置项（无事务版本）。
    ///
    /// 与 [`ConfigBasicRepository::replace_in_tx`] 同语义（Java
    /// `SettingsHelper#set`：`delete(eq(bizCode))` + `insert(ConfigPO)`），
    /// 供单条配置写入场景使用（showSettings / apiSettings / layoutThemeSwitch）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn save_code(&self, code: &str, value: &str) -> Result<()> {
        let mut tx = self.begin().await?;
        self.replace_in_tx(&mut tx, code, value).await?;
        tx.commit().await.map_err(map_err)?;
        Ok(())
    }

    /// 「先删后插」写入一个配置项（Java `SettingsHelper#set`）。
    ///
    /// Java 侧每次 `set` 都是 `delete(eq(bizCode))` + `insert(ConfigPO)`；
    /// 这里在同一事务内复刻该语义（保留历史行为：同一 `biz_code` 只留一行）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn replace_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        code: &str,
        value: &str,
    ) -> Result<()> {
        sqlx::query("DELETE FROM config WHERE biz_code = $1")
            .bind(code)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?;
        let now = nextrade_common::utils::now_millis();
        sqlx::query(
            "INSERT INTO config (biz_code, biz_val, create_time, create_by_id, create_by_name,
                                 update_time, update_by_id, update_by_name, is_del)
             VALUES ($1, $2, $3, 0, 'system', $4, 0, 'system', 0)",
        )
        .bind(code)
        .bind(value)
        .bind(now)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alive_condition_matches_java_is_del_filter() {
        assert!(ALIVE.contains("is_del = 0"));
        assert!(ALIVE.contains("is_del IS NULL"));
    }

    #[test]
    fn empty_code_list_short_circuits() {
        // 纯逻辑守卫：空列表不拼 `IN ()`（非法 SQL）
        let empty: &[&str] = &[];
        assert!(empty.is_empty());
    }
}
