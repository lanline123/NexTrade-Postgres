//! AES-256-GCM 密钥库（AI 助手 DESIGN §2.2）。
//!
//! 主密钥来自服务器环境变量 `NEXTTRADE_AI_MASTER_KEY`（Base64 编码的 32 字节），
//! 不入库、不入 git。密文与 nonce 均以 Base64 存 `ai_channel_key` 表。
//!
//! 2026-11-21 随 AI 导购立项从 admin crate（`api/admin/src/routes/ai_assistant/vault.rs`）
//! 整文件下沉到 services 共享层，代码与单测逐字不变，admin 改为再导出。

use std::sync::OnceLock;

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::Engine as _;

const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

fn cipher() -> Result<Aes256Gcm, String> {
    static CIPHER: OnceLock<Result<Aes256Gcm, String>> = OnceLock::new();
    CIPHER
        .get_or_init(|| {
            let raw = std::env::var("NEXTTRADE_AI_MASTER_KEY")
                .map_err(|_| "AI 主密钥未配置（服务器环境变量 NEXTTRADE_AI_MASTER_KEY 缺失）".to_string())?;
            let bytes = B64
                .decode(raw.trim())
                .map_err(|_| "AI 主密钥格式错误（应为 Base64）".to_string())?;
            if bytes.len() != 32 {
                return Err("AI 主密钥长度错误（应为 32 字节）".to_string());
            }
            let key = Key::<Aes256Gcm>::from_slice(&bytes);
            Ok(Aes256Gcm::new(key))
        })
        .clone()
}

/// 加密一条凭据，返回 (密文 Base64, nonce Base64)。
pub fn encrypt(plain: &str) -> Result<(String, String), String> {
    let cipher = cipher()?;
    // uuid v4 提供 122 位随机量，取前 12 字节作 GCM nonce（唯一性足够）。
    let uuid = uuid::Uuid::new_v4();
    let nonce_bytes: [u8; 12] = uuid.as_bytes()[..12]
        .try_into()
        .map_err(|_| "nonce 生成失败".to_string())?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let cipher_text = cipher
        .encrypt(nonce, plain.as_bytes())
        .map_err(|_| "凭据加密失败".to_string())?;
    Ok((B64.encode(cipher_text), B64.encode(nonce_bytes)))
}

pub fn decrypt(cipher_b64: &str, nonce_b64: &str) -> Result<String, String> {
    let cipher = cipher()?;
    let cipher_text = B64
        .decode(cipher_b64)
        .map_err(|_| "密文格式错误".to_string())?;
    let nonce_bytes = B64
        .decode(nonce_b64)
        .map_err(|_| "nonce 格式错误".to_string())?;
    let plain = cipher
        .decrypt(Nonce::from_slice(&nonce_bytes), cipher_text.as_ref())
        .map_err(|_| "凭据解密失败（主密钥不匹配或密文损坏）".to_string())?;
    String::from_utf8(plain).map_err(|_| "凭据不是合法 UTF-8".to_string())
}

/// 打码：`****` + 后 4 位（按字符计，避免多字节截断 panic）。
pub fn mask_of(plain: &str) -> String {
    let chars: Vec<char> = plain.chars().collect();
    if chars.is_empty() {
        return String::new();
    }
    if chars.len() <= 4 {
        return "****".to_string();
    }
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("****{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_key<T>(f: impl FnOnce() -> T) -> T {
        // 测试内自备主密钥（32 字节全 0x07 的 Base64），仅测试进程使用。
        let key = B64.encode([7u8; 32]);
        std::env::set_var("NEXTTRADE_AI_MASTER_KEY", key);
        f()
    }

    #[test]
    fn roundtrip_and_mask() {
        with_key(|| {
            let (cipher, nonce) = encrypt("sk-test-1234567890abcdef").expect("encrypt");
            assert_ne!(cipher, "sk-test-1234567890abcdef");
            let plain = decrypt(&cipher, &nonce).expect("decrypt");
            assert_eq!(plain, "sk-test-1234567890abcdef");
            assert_eq!(mask_of(&plain), "****cdef");
            assert_eq!(mask_of("abc"), "****");
            assert_eq!(mask_of(""), "");
        });
    }

    #[test]
    fn nonce_unique_per_encryption() {
        with_key(|| {
            let (c1, n1) = encrypt("same-plaintext").expect("e1");
            let (c2, n2) = encrypt("same-plaintext").expect("e2");
            assert_ne!(n1, n2);
            assert_ne!(c1, c2);
        });
    }
}
