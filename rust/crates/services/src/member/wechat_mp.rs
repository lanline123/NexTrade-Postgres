//! 微信公众号安全模式（AES-CBC）与回调签名工具。
//!
//! 微信公众平台「安全模式」消息体：
//! `AESKey = base64_decode(EncodingAESKey + "=")`（32 字节），IV = 前 16 字节；
//! 明文 = random(16) + msg_len(4, big-endian) + msg_xml + appid。
//! 本模块只用 `aes` crate 做 AES-256 分组运算，手工实现 CBC + PKCS#7，
//! 避免为一个回调额外引入 `cbc`/`block-padding` 依赖。

use aes::cipher::{generic_array::GenericArray, BlockDecrypt, BlockEncrypt, KeyInit};
use aes::Aes256;
use base64::Engine;
use sha1::{Digest, Sha1};
use nextrade_core::{Error, Result};

/// 微信签名（`signature = sha1(sort(token, timestamp, nonce))`）。
#[must_use]
pub fn verify_signature(token: &str, timestamp: &str, nonce: &str, signature: &str) -> bool {
    let mut parts = [token, timestamp, nonce];
    parts.sort_unstable();
    sha1_hex(&parts.concat()) == signature
}

/// 安全模式消息签名（`sha1(sort(token, timestamp, nonce, encrypt))`）。
#[must_use]
pub fn verify_msg_signature(
    token: &str,
    timestamp: &str,
    nonce: &str,
    encrypt: &str,
    msg_signature: &str,
) -> bool {
    let mut parts = [token, timestamp, nonce, encrypt];
    parts.sort_unstable();
    sha1_hex(&parts.concat()) == msg_signature
}

/// 把 EncodingAESKey 解码为 32 字节 AES Key。
///
/// 微信控制台生成的是 43 位无填充 base64；个别历史配置末尾字符可能不满足
/// base64 的 canonical trailing bits 要求，Java `Base64.decodeBase64` 会忽略
/// 末字符低位。这里先按标准解码，失败后按 Java 的宽松口径取高 4 位，保持兼容。
pub fn decode_aes_key(encoding_aes_key: &str) -> Result<Vec<u8>> {
    let raw = encoding_aes_key.trim();
    let padded = format!("{raw}=");
    let key = match base64::engine::general_purpose::STANDARD.decode(padded.as_bytes()) {
        Ok(key) => key,
        Err(_) => decode_base64_lenient(raw)?,
    };
    if key.len() != 32 {
        return Err(Error::Validation(format!(
            "EncodingAESKey 解码后应为 32 字节，实际 {}",
            key.len()
        )));
    }
    Ok(key)
}

fn decode_base64_lenient(raw: &str) -> Result<Vec<u8>> {
    let raw = raw.trim();
    if raw.len() != 43 {
        return Err(Error::Validation(format!(
            "EncodingAESKey 应为 43 位，实际 {}",
            raw.len()
        )));
    }
    let value = |c: u8| -> Result<u8> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(Error::Validation("EncodingAESKey 含非法 base64 字符".into())),
        }
    };
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(32);
    let mut i = 0;
    while i + 4 <= 40 {
        let a = value(bytes[i])?;
        let b = value(bytes[i + 1])?;
        let c = value(bytes[i + 2])?;
        let d = value(bytes[i + 3])?;
        out.push((a << 2) | (b >> 4));
        out.push((b << 4) | (c >> 2));
        out.push((c << 6) | d);
        i += 4;
    }
    // 剩余 3 个字符 → 2 字节；末字符只取高 4 位（与 Java 宽松解码一致）
    let a = value(bytes[40])?;
    let b = value(bytes[41])?;
    let c = value(bytes[42])?;
    out.push((a << 2) | (b >> 4));
    out.push((b << 4) | (c >> 2));
    Ok(out)
}

/// 解密微信安全模式消息，返回 XML 明文；同时校验末尾 AppID。
pub fn decrypt_message(
    encoding_aes_key: &str,
    expected_app_id: &str,
    encrypted: &str,
) -> Result<String> {
    let key = decode_aes_key(encoding_aes_key)?;
    let ciphertext = base64::engine::general_purpose::STANDARD
        .decode(encrypted.as_bytes())
        .map_err(|e| Error::Validation(format!("Encrypt 不是合法 base64: {e}")))?;
    if ciphertext.is_empty() || ciphertext.len() % 16 != 0 {
        return Err(Error::Validation("微信密文长度不是 16 的倍数".into()));
    }
    let cipher = Aes256::new(GenericArray::from_slice(&key));
    let mut iv = [0u8; 16];
    iv.copy_from_slice(&key[..16]);
    let mut plain = Vec::with_capacity(ciphertext.len());
    let mut prev = iv;
    for chunk in ciphertext.chunks(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.decrypt_block(&mut block);
        for (i, byte) in block.iter().enumerate() {
            plain.push(*byte ^ prev[i]);
        }
        prev.copy_from_slice(chunk);
    }
    let plain = unpad_pkcs7(&plain)?;
    if plain.len() < 20 {
        return Err(Error::Validation("微信明文长度不足".into()));
    }
    let msg_len = u32::from_be_bytes([plain[16], plain[17], plain[18], plain[19]]) as usize;
    if plain.len() < 20 + msg_len {
        return Err(Error::Validation("微信明文消息长度越界".into()));
    }
    let xml = &plain[20..20 + msg_len];
    let app_id = &plain[20 + msg_len..];
    if !app_id.is_empty() && app_id != expected_app_id.as_bytes() {
        return Err(Error::Validation("微信回调 AppID 不匹配".into()));
    }
    String::from_utf8(xml.to_vec())
        .map_err(|e| Error::Validation(format!("微信明文不是 UTF-8: {e}")))
}

/// 加密微信安全模式消息（用于本地测试/被动回复）。
pub fn encrypt_message(
    encoding_aes_key: &str,
    app_id: &str,
    xml: &str,
) -> Result<String> {
    let key = decode_aes_key(encoding_aes_key)?;
    let mut random = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut random);
    let mut plain = Vec::with_capacity(20 + xml.len() + app_id.len());
    plain.extend_from_slice(&random);
    plain.extend_from_slice(&(xml.len() as u32).to_be_bytes());
    plain.extend_from_slice(xml.as_bytes());
    plain.extend_from_slice(app_id.as_bytes());
    let plain = pad_pkcs7(&plain);
    let cipher = Aes256::new(GenericArray::from_slice(&key));
    let mut iv = [0u8; 16];
    iv.copy_from_slice(&key[..16]);
    let mut prev = iv;
    let mut ciphertext = Vec::with_capacity(plain.len());
    for chunk in plain.chunks(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        for (i, byte) in block.iter_mut().enumerate() {
            *byte ^= prev[i];
        }
        cipher.encrypt_block(&mut block);
        ciphertext.extend_from_slice(&block);
        prev.copy_from_slice(&block);
    }
    Ok(base64::engine::general_purpose::STANDARD.encode(ciphertext))
}

/// 从 XML 中取简单标签值，兼容 `<tag><![CDATA[value]]></tag>` 与 `<tag>value</tag>`。
#[must_use]
pub fn xml_field(xml: &str, tag: &str) -> Option<String> {
    let cdata_open = format!("<{tag}><![CDATA[");
    let cdata_close = format!("]]></{tag}>");
    if let Some(start) = xml.find(&cdata_open) {
        let rest = &xml[start + cdata_open.len()..];
        if let Some(end) = rest.find(&cdata_close) {
            return Some(rest[..end].to_string());
        }
    }
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].to_string())
}

fn sha1_hex(input: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn pad_pkcs7(data: &[u8]) -> Vec<u8> {
    let pad = 32 - (data.len() % 32);
    let mut out = data.to_vec();
    out.extend(std::iter::repeat(pad as u8).take(pad));
    out
}

fn unpad_pkcs7(data: &[u8]) -> Result<Vec<u8>> {
    let Some(&pad) = data.last() else {
        return Err(Error::Validation("微信明文为空".into()));
    };
    let pad = pad as usize;
    if pad == 0 || pad > 32 || pad > data.len() {
        return Err(Error::Validation("微信明文 PKCS#7 填充非法".into()));
    }
    if data[data.len() - pad..].iter().any(|b| *b as usize != pad) {
        return Err(Error::Validation("微信明文 PKCS#7 填充非法".into()));
    }
    Ok(data[..data.len() - pad].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY";

    #[test]
    fn signature_and_msg_signature() {
        let token = "Xk9mP2vLqR7wN4jYs8bHd1fGc5aZu3eT";
        let ts = "1700000000";
        let nonce = "abc123";
        let mut parts = [token, ts, nonce];
        parts.sort_unstable();
        let sig = sha1_hex(&parts.concat());
        assert!(verify_signature(token, ts, nonce, &sig));
        assert!(!verify_signature(token, ts, nonce, "bad"));

        let encrypt = "ENCRYPTED";
        let mut parts = [token, ts, nonce, encrypt];
        parts.sort_unstable();
        let msg_sig = sha1_hex(&parts.concat());
        assert!(verify_msg_signature(token, ts, nonce, encrypt, &msg_sig));
    }

    #[test]
    fn aes_roundtrip_extracts_xml_and_appid() {
        let xml = "<xml><FromUserName><![CDATA[openid-1]]></FromUserName><Event><![CDATA[SCAN]]></Event></xml>";
        let encrypted = encrypt_message(KEY, "wx23ad0efee8d1bba1", xml).unwrap();
        let plain = decrypt_message(KEY, "wx23ad0efee8d1bba1", &encrypted).unwrap();
        assert_eq!(plain, xml);
        assert_eq!(xml_field(&plain, "FromUserName").as_deref(), Some("openid-1"));
        assert_eq!(xml_field(&plain, "Event").as_deref(), Some("SCAN"));
    }

    #[test]
    fn appid_mismatch_is_rejected() {
        let encrypted = encrypt_message(KEY, "wx-other", "<xml/>").unwrap();
        assert!(decrypt_message(KEY, "wx23ad0efee8d1bba1", &encrypted).is_err());
    }
}
