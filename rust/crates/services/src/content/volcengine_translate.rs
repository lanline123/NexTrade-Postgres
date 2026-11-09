//! 火山引擎机器翻译客户端（Volcengine SigV4，`TranslateText` 2020-06-01）。
//!
//! Java 侧使用 `volc-sdk-java` 的 `TranslateServiceImpl`；本实现按同一签名协议
//! 直接调用 `https://translate.volcengineapi.com/`，配置项：
//! `langVolcengineAccessKey` / `langVolcengineSecret`。
//!
//! 失败（密钥缺失、账户欠费、网络/权限错误）一律返回明确业务错误，不伪造译文。

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

type HmacSha256 = Hmac<Sha256>;

const HOST: &str = "translate.volcengineapi.com";
const REGION: &str = "cn-north-1";
const SERVICE: &str = "translate";
const ACTION: &str = "TranslateText";
const VERSION: &str = "2020-06-01";

/// 火山翻译客户端。
pub struct VolcengineTranslateClient {
    access_key: String,
    secret_key: String,
}

impl VolcengineTranslateClient {
    /// 从 `config` 表读取 AK/SK；缺失时返回可操作错误。
    pub async fn from_repos(repos: &Repositories) -> Result<Self> {
        let pairs = repos
            .system
            .config_basic
            .find_by_codes(&["langVolcengineAccessKey", "langVolcengineSecret"])
            .await?;
        let map: std::collections::HashMap<String, String> = pairs.into_iter().collect();
        let access_key = map
            .get("langVolcengineAccessKey")
            .map(String::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::biz(1001, "请检查语言翻译服务密钥配置（缺少 langVolcengineAccessKey）"))?;
        let secret_key = map
            .get("langVolcengineSecret")
            .map(String::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::biz(1001, "请检查语言翻译服务密钥配置（缺少 langVolcengineSecret）"))?;
        Ok(Self {
            access_key: access_key.to_string(),
            secret_key: secret_key.to_string(),
        })
    }

    /// 单条文本翻译。
    pub async fn translate_text(&self, target_language: &str, text: &str) -> Result<String> {
        let body = serde_json::json!({
            "TargetLanguage": target_language,
            "TextList": [text],
        })
        .to_string();
        let (x_date, body_hash, authorization) = self.sign(&body)?;
        let url = format!("https://{HOST}/?Action={ACTION}&Version={VERSION}");
        let resp = reqwest::Client::new()
            .post(url)
            .header("Content-Type", "application/json")
            .header("X-Date", &x_date)
            .header("X-Content-Sha256", &body_hash)
            .header("Authorization", authorization)
            .body(body)
            .timeout(std::time::Duration::from_secs(20))
            .send()
            .await
            .map_err(|e| Error::Internal(format!("火山翻译请求失败: {e}")))?;
        let status = resp.status();
        let raw = resp
            .text()
            .await
            .map_err(|e| Error::Internal(format!("火山翻译响应读取失败: {e}")))?;
        if !status.is_success() {
            return Err(Error::Internal(format!("火山翻译 HTTP {status}: {raw}")));
        }
        let value: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| Error::Internal(format!("火山翻译响应解析失败: {e}; body={raw}")))?;
        if let Some(err) = value.pointer("/ResponseMetadata/Error") {
            let code = err.get("Code").and_then(|v| v.as_str()).unwrap_or("");
            let message = err.get("Message").and_then(|v| v.as_str()).unwrap_or("");
            return Err(Error::biz(
                1001,
                format!("火山翻译失败: {message}({code})"),
            ));
        }
        let translation = value
            .pointer("/TranslationList/0/Translation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::biz(1001, format!("火山翻译未返回译文: {raw}")))?;
        Ok(translation.to_string())
    }

    fn sign(&self, body: &str) -> Result<(String, String, String)> {
        let x_date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let date = &x_date[..8];
        let body_hash = hex::encode(Sha256::digest(body.as_bytes()));
        let canonical_headers = format!(
            "content-type:application/json\nhost:{HOST}\nx-content-sha256:{body_hash}\nx-date:{x_date}\n"
        );
        let signed_headers = "content-type;host;x-content-sha256;x-date";
        let canonical_request = format!(
            "POST\n/\nAction={ACTION}&Version={VERSION}\n{canonical_headers}\n{signed_headers}\n{body_hash}"
        );
        let credential_scope = format!("{date}/{REGION}/{SERVICE}/request");
        let string_to_sign = format!(
            "HMAC-SHA256\n{x_date}\n{credential_scope}\n{}",
            hex::encode(Sha256::digest(canonical_request.as_bytes()))
        );
        let k_date = hmac_sha256(self.secret_key.as_bytes(), date.as_bytes());
        let k_region = hmac_sha256(&k_date, REGION.as_bytes());
        let k_service = hmac_sha256(&k_region, SERVICE.as_bytes());
        let k_signing = hmac_sha256(&k_service, b"request");
        let signature = hex::encode(hmac_sha256(&k_signing, string_to_sign.as_bytes()));
        let authorization = format!(
            "HMAC-SHA256 Credential={}/{}, SignedHeaders={signed_headers}, Signature={signature}",
            self.access_key, credential_scope
        );
        Ok((x_date, body_hash, authorization))
    }
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC 支持任意长度 key");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing_is_deterministic_for_fixed_time() {
        // 签名函数内部取当前时间，无法断言固定值；这里只锁定 HMAC 链路不 panic。
        let client = VolcengineTranslateClient {
            access_key: "ak".into(),
            secret_key: "sk".into(),
        };
        let (date, body_hash, auth) = client.sign("{}").unwrap();
        assert!(date.ends_with('Z'));
        assert_eq!(body_hash.len(), 64);
        assert!(auth.starts_with("HMAC-SHA256 Credential=ak/"));
        assert!(auth.contains("SignedHeaders=content-type;host;x-content-sha256;x-date"));
    }
}
