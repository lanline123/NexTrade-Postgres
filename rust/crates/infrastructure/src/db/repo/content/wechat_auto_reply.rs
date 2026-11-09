//! 公众号自动回复仓储：`config` 表的 `biz_code` / `biz_val` 读写。
//!
//! 对应原 Java `ConfigMapper`（仅本域用到的部分）。
//!
//! 列名严格对齐 `python tools/schema_parser.py --table config`。
//!
//! 注意：Java `WechatAutoReplyServiceImpl` 查 `config` 时**不带 `is_del` 过滤**，
//! 本实现逐字保持一致（见 models 模块头差异 3）。

use sqlx::PgPool;
use nextrade_common::utils::now_seconds;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct WechatAutoReplyRepository {
    pool: PgPool,
}

impl WechatAutoReplyRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 按 bizCode 读 `biz_val`（Java `configMapper.selectOne(eq bizCode)`）。
    ///
    /// 返回三态：`None` = 无记录；`Some(None)` = 记录存在但 `biz_val` 为 NULL；
    /// `Some(Some(v))` = 正常值。
    pub async fn find_biz_val(&self, biz_code: &str) -> Result<Option<Option<String>>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = $1 LIMIT 1",
        )
        .bind(biz_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(v,)| v))
    }

    /// upsert（Java `upsert`，`@Transactional`）：存在则更新 `biz_val`，
    /// 否则插入一条 `config(biz_code, biz_val)`。
    pub async fn upsert(&self, biz_code: &str, biz_val: &str) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        let existing: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM config WHERE biz_code = $1 LIMIT 1")
                .bind(biz_code)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_err)?;
        let now = now_seconds();
        if let Some((id,)) = existing {
            sqlx::query("UPDATE config SET biz_val = $1, update_time = $2 WHERE id = $3")
                .bind(biz_val)
                .bind(now)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
        } else {
            sqlx::query(
                "INSERT INTO config (biz_code, biz_val, create_time, update_time, is_del) \
                 VALUES ($1, $2, $3, $4, 0)",
            )
            .bind(biz_code)
            .bind(biz_val)
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

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("配置不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn biz_codes_match_java_constants() {
        use nextrade_core::models::content::wechat_auto_reply::{
            BIZ_DEFAULT, BIZ_LOGIN_SUCCESS, BIZ_SUBSCRIBE,
        };
        assert_eq!(BIZ_SUBSCRIBE, "wechatSubscribeReply");
        assert_eq!(BIZ_DEFAULT, "wechatDefaultReply");
        assert_eq!(BIZ_LOGIN_SUCCESS, "wechatLoginSuccessReply");
    }

    #[test]
    fn repository_is_clone() {
        // 仅验证结构体可 Clone（组合根装配需要）
        fn assert_clone<T: Clone>() {}
        assert_clone::<WechatAutoReplyRepository>();
    }
}
