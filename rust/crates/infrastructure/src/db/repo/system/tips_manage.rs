//! 提示管理域（system::tips_manage）仓储。
//!
//! 只读一张表：
//! - `config`（系统配置表）：`biz_code` / `biz_val` / `is_del`
//!
//! 对应 Java `ConfigServiceImpl#getConfigVal(SettingsEnum)`：
//! `lambdaQuery().eq(ConfigPO::getBizCode, bizCode).eq(ConfigPO::getIsDel, 0).one()`，
//! 命中返回 `biz_val`（可能为 null），未命中返回空串 `""`。
//!
//! 查询使用运行时 `sqlx::query_as`（**不使用** `query!` 宏，本机无 `DATABASE_URL`）。

use sqlx::PgPool;

use nextrade_core::{Error, Result};

/// 提示管理域仓储（持有连接池）。
#[derive(Clone)]
pub struct TipsManageRepository {
    pool: PgPool,
}

impl TipsManageRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（与其它仓储一致，供上层复用）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 读取配置项原始值（对应 Java `ConfigServiceImpl#getConfigVal`）。
    ///
    /// 返回值是**两层 `Option`**，用于精确复刻 Java 的三态：
    /// - `None`：无匹配行（`biz_code` 不存在或 `is_del <> 0`）
    ///   ⇒ Java 返回空串 `""`
    /// - `Some(None)`：行存在但 `biz_val` 为 SQL NULL ⇒ Java 返回 `null`
    /// - `Some(Some(value))`：正常取值
    ///
    /// 多条命中时 Java `.one()` 会抛 `TooManyResultsException`；这里 `LIMIT 1`
    /// （`biz_code` 实际唯一，见 `uncertain_points`）。
    pub async fn find_config_value(&self, biz_code: &str) -> Result<Option<Option<String>>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1",
        )
        .bind(biz_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(value,)| value))
    }
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

    /// 两层 Option 的三态语义（不触库，仅校验类型契约）。
    #[test]
    fn config_value_three_states_are_distinguishable() {
        let missing: Option<Option<String>> = None;
        let null_value: Option<Option<String>> = Some(None);
        let value: Option<Option<String>> = Some(Some("x.com".into()));

        assert!(missing.is_none());
        assert_eq!(null_value, Some(None));
        assert_eq!(
            value.as_ref().and_then(|v| v.as_deref()),
            Some("x.com")
        );
    }
}
