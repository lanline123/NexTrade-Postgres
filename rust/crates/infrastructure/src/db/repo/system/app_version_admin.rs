//! app 版本管理域（system::app_version_admin）仓储：`config`（系统配置表）。
//!
//! 对应原 Java：
//! - `AppVersionServiceImpl#getDetail` → `ConfigServiceImpl#getConfigMapBySettings`
//! - `AppVersionServiceImpl#update` → `ConfigService.lambdaUpdate().in(bizCode).remove()`
//!   + `ConfigService.saveBatch(configs)`
//!
//! 列名严格对齐 `java/database/pure.sql`
//! （`python tools/schema_parser.py --table config` 可复核）：
//! - `config`：`id`（`bigint AUTO_INCREMENT`）/ `biz_code` / `biz_val` /
//!   `create_time` / `create_by_id` / `create_by_name` / `update_time` /
//!   `update_by_id` / `update_by_name` / `is_del`
//!
//! 逻辑删除约定同 MyBatis-Plus `@TableLogic`：写入 `is_del = 0`，
//! 删除为 `UPDATE ... SET is_del = 1`，读取一律带 `is_del = 0`。
//!
//! 查询统一使用运行时 `sqlx::query_as`（**不使用 `query!` 宏**）。

use std::collections::HashMap;

use sqlx::PgPool;
use nextrade_core::{Error, Result};

/// `config` 表仓储（app 版本管理域独占）。
#[derive(Clone)]
pub struct AppVersionAdminRepository {
    pool: PgPool,
}

impl AppVersionAdminRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 批量取未删除的配置值（键为 `biz_code`）。
    ///
    /// 对应 Java `ConfigServiceImpl#getConfigMapBySettings`：`lambdaQuery().in(bizCode,
    /// bizCods).list()`（`@TableLogic` 自动追加 `is_del = 0`）后收集为 Map。
    ///
    /// 与原 Java 的两点差异（均为防御性处理，见模块 `uncertain_points`）：
    /// - Java `Collectors.toMap` 遇到 `biz_val IS NULL` 会抛 NPE，这里**跳过空值**，
    ///   等效于「该配置项不存在」；
    /// - 命中多行时 Java 也会抛 `IllegalStateException`，这里按 `biz_code` 覆盖取值。
    pub async fn find_config_vals(&self, biz_codes: &[&str]) -> Result<HashMap<String, String>> {
        if biz_codes.is_empty() {
            return Ok(HashMap::new());
        }
        let sql = format!(
            "SELECT biz_code, biz_val FROM config WHERE is_del = 0 AND biz_code IN ({})",
            placeholders(biz_codes.len())
        );
        let mut q = sqlx::query_as::<_, (String, Option<String>)>(&sql);
        for code in biz_codes {
            q = q.bind(*code);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows
            .into_iter()
            .filter_map(|(code, val)| val.map(|v| (code, v)))
            .collect())
    }

    /// 用给定键值对**整体替换**配置项：先逻辑删除这些 `biz_code` 的旧行，
    /// 再逐条插入新行（同一事务内，避免删除成功而插入失败导致配置丢失）。
    ///
    /// 对应 Java `AppVersionServiceImpl#update` 中的
    /// `configService.lambdaUpdate().in(ConfigPO::getBizCode, bizCodes).remove()`
    /// + `configService.saveBatch(configs)`。
    ///
    /// 审计字段对齐 MyBatis-Plus `MybatisPlusMetaObjectHandler#insertFill`：
    /// `create_time` / `update_time` 为 **epoch 秒**，`create_by_id` / `update_by_id`
    /// 为当前管理员 ID，`create_by_name` / `update_by_name` 为当前管理员用户名
    /// （Java 在用户名为空时写 `"unknown"`），`is_del = 0`。
    pub async fn replace_configs(
        &self,
        entries: &[(String, String)],
        admin_id: Option<i64>,
        admin_name: &str,
    ) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await.map_err(map_err)?;

        let codes: Vec<&str> = entries.iter().map(|(code, _)| code.as_str()).collect();
        let delete_sql = format!(
            "UPDATE config SET is_del = 1 WHERE is_del = 0 AND biz_code IN ({})",
            placeholders(codes.len())
        );
        let mut del = sqlx::query(&delete_sql);
        for code in &codes {
            del = del.bind(*code);
        }
        del.execute(&mut *tx).await.map_err(map_err)?;

        let now = chrono::Utc::now().timestamp();
        for (code, val) in entries {
            // 单行书写：`tools/check_sql_columns.py` 只能解析闭合在同一行/连续字面量中的 SQL。
            sqlx::query(
                "INSERT INTO config (biz_code, biz_val, create_time, create_by_id, create_by_name, update_time, update_by_id, update_by_name, is_del) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0)",
            )
            .bind(code)
            .bind(val)
            .bind(now)
            .bind(admin_id)
            .bind(admin_name)
            .bind(now)
            .bind(admin_id)
            .bind(admin_name)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        }

        tx.commit().await.map_err(map_err)?;
        Ok(())
    }
}

/// `?,?,?` 占位符（长度由调用方控制，恒为合法 SQL 片段）。
fn placeholders(n: usize) -> String {
    std::iter::repeat_n("?", n).collect::<Vec<_>>().join(",")
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("配置项不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_count() {
        assert_eq!(placeholders(1), "?");
        assert_eq!(placeholders(4), "?,?,?,?");
    }
}