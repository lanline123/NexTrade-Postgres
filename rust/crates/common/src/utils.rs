//! 通用工具函数，替代原 Java `com.nextrade.common.utils.*` 与 Hutool 工具类。

use sha2::{Digest, Sha256};

/// 生成订单号：`yyyyMMddHHmmss` + 6位随机数（与原 Java 订单号格式一致）。
pub fn generate_order_sn() -> String {
    use rand::Rng;
    let now = chrono::Local::now();
    let rand: u32 = rand::thread_rng().gen_range(0..1_000_000);
    format!("{}{:06}", now.format("%Y%m%d%H%M%S"), rand)
}

/// 生成指定长度的随机十六进制串（短信验证码/token 等）。
pub fn random_hex(len: usize) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| format!("{:x}", rng.gen_range(0..16)))
        .collect()
}

/// SHA-256 十六进制摘要（原 Java DigestUtil.sha256Hex 等价）。
pub fn sha256_hex(input: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_ref());
    hex::encode(hasher.finalize())
}

/// 生成带前缀的业务流水号，如 `UC20260101120000ab12cd`。
///
/// 用于优惠券 SN（`user_coupon.coupon_sn`）等需要唯一短号的场景。
pub fn gen_sn(prefix: &str) -> String {
    let now = chrono::Local::now();
    format!("{prefix}{}{}", now.format("%Y%m%d%H%M%S"), random_hex(6))
}

/// 毫秒时间戳。
pub fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// 秒时间戳。
pub fn now_seconds() -> i64 {
    chrono::Utc::now().timestamp()
}

/// 手机号脱敏：`138****8000`。
pub fn mask_mobile(mobile: &str) -> String {
    let chars: Vec<char> = mobile.chars().collect();
    if chars.len() >= 11 {
        let head: String = chars[..3].iter().collect();
        let tail: String = chars[7..11].iter().collect();
        format!("{head}****{tail}")
    } else if mobile.is_empty() {
        String::new()
    } else {
        "*".repeat(mobile.len())
    }
}

/// 邮箱脱敏：`ab****@example.com`。
pub fn mask_email(email: &str) -> String {
    match email.split_once('@') {
        Some((user, domain)) if user.len() > 2 => {
            format!("{}****@{}", &user[..2], domain)
        }
        _ => "***".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_sn_format() {
        let sn = generate_order_sn();
        assert_eq!(sn.len(), 20); // 14 时间 + 6 随机
        assert!(sn.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn gen_sn_has_prefix_and_is_unique() {
        let a = gen_sn("UC");
        let b = gen_sn("UC");
        assert!(a.starts_with("UC"));
        assert_eq!(a.len(), 2 + 14 + 6);
        assert_ne!(a, b);
    }

    #[test]
    fn mask() {
        assert_eq!(mask_mobile("13812348000"), "138****8000");
        assert_eq!(mask_email("zhangsan@example.com"), "zh****@example.com");
    }

    #[test]
    fn sha256_known_value() {
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}