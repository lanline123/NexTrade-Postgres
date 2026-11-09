//! 密码哈希。
//!
//! 与原 Java `BCryptPasswordEncoder` 保持兼容（BCrypt `$2a$` 前缀），
//! 新密码可使用强度更高的 Argon2，通过前缀自动识别算法。

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use nextrade_core::{Error, Result};

/// 生成密码哈希（BCrypt，兼容原 Java 数据）。
pub fn hash_password(password: &str) -> Result<String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| Error::Internal(format!("密码哈希失败: {e}")))
}

/// 校验密码。
///
/// - `$2a$` / `$2b$` 前缀 → BCrypt（原 Java 写入的历史密码）
/// - `$argon2` 前缀 → Argon2（新算法）
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    if hash.starts_with("$argon2") {
        let parsed = PasswordHash::new(hash)
            .map_err(|e| Error::Internal(format!("密码哈希格式错误: {e}")))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    } else {
        bcrypt::verify(password, hash)
            .map_err(|e| Error::Internal(format!("密码校验失败: {e}")))
    }
}

/// 使用 Argon2 生成新密码哈希（推荐新用户使用）。
pub fn hash_password_argon2(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| Error::Internal(format!("Argon2 哈希失败: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bcrypt_roundtrip() {
        let h = hash_password("secret123").unwrap();
        assert!(h.starts_with("$2"));
        assert!(verify_password("secret123", &h).unwrap());
        assert!(!verify_password("wrong", &h).unwrap());
    }

    #[test]
    fn java_2y_bcrypt() {
        // Java `BCryptPasswordEncoder` 生成的历史密码使用 `$2y$` 前缀。
        let hash = "$2y$10$KW/snmOoevmgubZFY9eOFeW3zsxQcJMnDmWEuDboVLm3.5VkG5BL6";
        assert!(verify_password("admin123", hash).unwrap());
        assert!(!verify_password("wrong", hash).unwrap());
    }

    #[test]
    fn argon2_roundtrip() {
        let h = hash_password_argon2("secret123").unwrap();
        assert!(h.starts_with("$argon2"));
        assert!(verify_password("secret123", &h).unwrap());
    }
}