//! 表单令牌（CSRF token）服务。
//!
//! 对应原 Java `com.nextrade.adminapi.controller.common.CSRFController` 与
//! `com.nextrade.api.controller.common.CSRFController` 的 `create()`：
//!
//! 1. `token = md5Hex(String.valueOf(System.currentTimeMillis()))`
//! 2. Redis 写入 `token -> token`，TTL 10 分钟
//! 3. 返回 token 字符串
//!
//! # 语义说明（与 Java 一致）
//!
//! - Java 侧的 CSRF **校验逻辑已被注释掉**（见 `LoginFilter#attemptAuthentication`
//!   与 `ClientLoginFilter`），因此本服务只负责「生成并落缓存」，不提供校验；
//! - 原 Java 返回裸 `String`（非 `AjaxResult`）。本实现遵循本项目统一响应包装
//!   （`ApiResponse`），`data` 即 token 字符串；见 `uncertain_points`。
//!
//! # 降级
//!
//! Redis 不可用时（`AppConfig` 允许降级），仍返回 token，但不会落缓存 ——
//! 与 Java 的 `RedisCache` 强依赖不同，属于 Rust 侧既有的降级约定。

use md5::{Digest, Md5};

use nextrade_common::utils;
use nextrade_core::Result;
use nextrade_infrastructure::cache::CacheService;

/// Redis key 的 TTL（Java `10, TimeUnit.MINUTES`）。
pub const CSRF_TOKEN_TTL_SECS: u64 = 600;

/// 表单令牌服务（无状态，仅持有可选 Redis 句柄）。
#[derive(Clone)]
pub struct CsrfService {
    cache: Option<CacheService>,
}

impl CsrfService {
    /// 构造：`cache` 为 `None` 时进入「无缓存降级」模式。
    #[must_use]
    pub fn new(cache: Option<CacheService>) -> Self {
        Self { cache }
    }

    /// 生成并缓存一个新的表单令牌。
    ///
    /// 返回值为 32 位小写十六进制 MD5（与 Hutool `DigestUtil.md5Hex` 一致）。
    pub async fn create(&self) -> Result<String> {
        let seed = utils::now_millis().to_string();
        let token = md5_hex(seed.as_bytes());
        if let Some(cache) = &self.cache {
            // Java: redisCache.setCacheObject(md5Hex1, md5Hex1, 10, TimeUnit.MINUTES)
            cache
                .set_str(&token, &token, CSRF_TOKEN_TTL_SECS)
                .await?;
        }
        Ok(token)
    }
}

/// 计算 32 位小写十六进制 MD5。
#[must_use]
pub fn md5_hex(bytes: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_matches_known_vector() {
        // RFC 1321 测试向量：md5("abc") = 900150983cd24fb0d6963f7d28e17f72
        assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
        // 空串
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn md5_is_32_lower_hex_chars() {
        let token = md5_hex(b"1700000000000");
        assert_eq!(token.len(), 32);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[tokio::test]
    async fn create_without_cache_still_returns_token() {
        let svc = CsrfService::new(None);
        let token = svc.create().await.expect("无缓存也应返回 token");
        assert_eq!(token.len(), 32);
    }
}
