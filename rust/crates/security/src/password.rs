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

/// 服务端密码强度校验（账号安全批 2026-11-21，我方新增）。
///
/// 规则与 H5 前台既有校验逐条同规（uniapp `pages/user/security/password.tsx`
/// 自旧 uniapp 逐行移植的表单规则，即原产品口径）：
/// 6–20 位、不含空白、不能全为数字 / 全为字母 / 全为符号、不能全为相同字符。
/// 「字母 / 数字 / 符号」分类与前端正则一致：数字 = `[0-9]`，字母 = `[a-zA-Z]`，
/// 符号 = 非 `[0-9a-zA-Z_]`（前端 `\W`，下划线不计入符号）。
/// 此前服务端只有注册 / 后台改密处的「长度 ≥ 6」弱校验，前端规则可被绕过；
/// 本函数供注册 / 改密 / 重置各入口统一调用。仅约束**新设**密码，存量密码
/// 不受影响、不强制失效。
///
/// # Errors
/// 不满足任一规则 → `Error::Validation`（文案与前端提示逐字一致）。
pub fn validate_password_strength(password: &str) -> Result<()> {
    let chars: Vec<char> = password.chars().collect();
    if !(6..=20).contains(&chars.len()) {
        return Err(Error::Validation("密码应为6-20位字符".into()));
    }
    if chars.iter().any(|c| c.is_whitespace()) {
        return Err(Error::Validation("密码中不允许有空格".into()));
    }
    let has_digit = chars.iter().any(|c| c.is_ascii_digit());
    let has_letter = chars.iter().any(|c| c.is_ascii_alphabetic());
    let has_symbol = chars
        .iter()
        .any(|c| !c.is_ascii_alphanumeric() && *c != '_');
    if chars.iter().all(|c| c.is_ascii_digit()) {
        return Err(Error::Validation("密码不能全为数字".into()));
    }
    if !has_digit && !has_symbol {
        return Err(Error::Validation("密码不能全为字母".into()));
    }
    if !has_letter && !has_digit {
        return Err(Error::Validation("密码不能全为符号".into()));
    }
    if chars.iter().all(|c| *c == chars[0]) {
        return Err(Error::Validation("密码不能全为相同字符或数字".into()));
    }
    Ok(())
}

/// 禁复用校验：新密码不得与当前密码相同（改密 / 重置入口调用，
/// 账号安全批 2026-11-21，我方新增）。通过现有哈希校验判定，
/// 不接触明文存量。
///
/// # Errors
/// 新密码与当前密码一致 → 业务错误 400；哈希解析失败 → `Error::Internal`。
pub fn validate_password_not_reused(new_password: &str, current_hash: &str) -> Result<()> {
    if verify_password(new_password, current_hash)? {
        return Err(Error::biz(400, "新密码不能与当前密码相同"));
    }
    Ok(())
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

    #[test]
    fn strength_accepts_mixed_passwords() {
        // 字母+数字 / 字母+符号 / 数字+符号 / 三类齐全 / 含下划线与中文（中文计符号类）
        for ok in ["abc123", "abc!@#", "123!@#", "Abc123!", "abc_123", "密码abc123"] {
            assert!(validate_password_strength(ok).is_ok(), "应通过: {ok}");
        }
    }

    #[test]
    fn strength_rejects_single_class_and_shape_violations() {
        let cases: [(&str, &str); 10] = [
            ("abc12", "密码应为6-20位字符"),
            ("abcdefghijklmnopqrstu", "密码应为6-20位字符"),
            ("abc 123", "密码中不允许有空格"),
            ("123456", "密码不能全为数字"),
            ("abcdef", "密码不能全为字母"),
            ("abc___", "密码不能全为字母"),
            ("!@#$%^", "密码不能全为符号"),
            ("______", "密码不能全为字母"),
            // 与前端校验顺序一致：单一字符重复的纯字母串先命中
            // 「全为字母」（前端最后一档「全为相同」对单类重复串实际
            // 被前置规则遮蔽，本实现保持同一顺序与口径）。
            ("aaaaaa", "密码不能全为字母"),
            ("111111", "密码不能全为数字"),
        ];
        for (pwd, msg) in cases {
            match validate_password_strength(pwd) {
                Err(Error::Validation(m)) => assert_eq!(m, msg, "密码 {pwd} 文案不符"),
                other => panic!("密码 {pwd} 应报校验错误，实际: {other:?}"),
            }
        }
    }

    #[test]
    fn not_reused_compares_against_current_hash() {
        let h = hash_password("OldPass123").unwrap();
        assert!(validate_password_not_reused("OldPass123", &h).is_err());
        assert!(validate_password_not_reused("NewPass456", &h).is_ok());
    }
}