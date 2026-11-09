//! 客服消息能力探测仓储：`config` 四键配置判定（公众号是否配置）。
//!
//! 对应原 Java `WxMpServiceHolder#get()`：读取
//! `wechatAppId` / `wechatAppSecret` / `wechatServerToken` / `wechatServerSecret`
//! 四个 `config.biz_code`，任一为空 → `wxMpService == null` →
//! `WxKefuCapabilityProbe.enabled = false`。
//!
//! 本仓储只做**本地配置判定**，不发起任何微信 HTTP 请求。

use sqlx::PgPool;
use nextrade_core::models::content::wechat_kefu_capability::WECHAT_CONFIG_CODES;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct WechatKefuCapabilityRepository {
    pool: PgPool,
}

impl WechatKefuCapabilityRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 公众号是否已完整配置（四个 bizCode 均存在且 `biz_val` 非空，`is_del = 0`）。
    pub async fn is_wechat_configured(&self) -> Result<bool> {
        let sql = "SELECT COUNT(*) FROM config \
                   WHERE biz_code IN ($1, $2, $3, $4) AND is_del = 0 \
                     AND biz_val IS NOT NULL AND TRIM(biz_val) <> ''";
        let row: (i64,) = sqlx::query_as(sql)
            .bind(WECHAT_CONFIG_CODES[0])
            .bind(WECHAT_CONFIG_CODES[1])
            .bind(WECHAT_CONFIG_CODES[2])
            .bind(WECHAT_CONFIG_CODES[3])
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0 >= WECHAT_CONFIG_CODES.len() as i64)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_is_clone() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<WechatKefuCapabilityRepository>();
    }

    #[test]
    fn config_codes_are_the_java_four() {
        assert_eq!(WECHAT_CONFIG_CODES.len(), 4);
        assert!(WECHAT_CONFIG_CODES.contains(&"wechatAppId"));
        assert!(WECHAT_CONFIG_CODES.contains(&"wechatServerSecret"));
    }
}
