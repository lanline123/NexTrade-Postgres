//! 进程内 JWT 吊销表（对应 Java Redis 黑名单）。
//!
//! 单实例部署可用；多实例需换 Redis。键为完整令牌字符串，值为过期时间（秒）。
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

static REVOKED: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();

fn store() -> &'static Mutex<HashMap<String, i64>> {
    REVOKED.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 吊销令牌，`exp` 为令牌过期时间（秒），过期后自动清理。
pub fn revoke(token: &str, exp: i64) {
    if let Ok(mut map) = store().lock() {
        map.insert(token.to_string(), exp);
    }
}

/// 令牌是否已吊销。
pub fn is_revoked(token: &str) -> bool {
    let Ok(mut map) = store().lock() else {
        return false;
    };
    let now = chrono::Utc::now().timestamp();
    map.retain(|_, exp| *exp > now);
    map.contains_key(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revoke_flow() {
        let token = "test-token-revocation";
        assert!(!is_revoked(token));
        revoke(token, chrono::Utc::now().timestamp() + 60);
        assert!(is_revoked(token));
    }
}
