//! 微信公众号 / 小程序 API 客户端（协议层，替代 Java `WxMpService` / `WechatApiClient`）。
//!
//! 覆盖无需商户支付凭据的部分：
//! - `access_token` 获取与进程内缓存（7000s，提前 60s 过期）：走稳定版接口
//!   `POST /cgi-bin/stable_token`（`force_refresh=false`），不用老接口
//!   `GET /cgi-bin/token`——老接口每次调用都会作废上一个 token，稳定版在
//!   有效期内重复调用不刷新、配额更高（1 万次/分钟、50 万次/天），且其
//!   token 与老接口 token 互相隔离（切换后首次获取会换发新 token，老
//!   token 仍可活到自身过期，互不影响）；
//! - 客服列表 `cgi-bin/customservice/getkflist`（客服能力探测）；
//! - 客服文本消息 `cgi-bin/message/custom/send`；
//! - 小程序 `jscode2session`（code → openid/session_key）。
//!
//! 配置项（`config` 表，与 Java `SettingsEnum` 一致）：
//! `wechatAppId` / `wechatAppSecret` / `wechatServerToken` / `wechatServerSecret`
//! （后两项为服务器消息校验用，客户端调用只强依赖前两项）。
//!
//! 未配置时返回明确业务错误；微信返回的 `errcode != 0` 统一转为业务错误并保留 errcode/errmsg。
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

const API_BASE: &str = "https://api.weixin.qq.com";
const CONFIG_CODES: [&str; 4] = [
    "wechatAppId",
    "wechatAppSecret",
    "wechatServerToken",
    "wechatServerSecret",
];
const TOKEN_TTL_SECS: u64 = 7_000;
const TOKEN_EXPIRY_MARGIN_SECS: u64 = 60;

/// 公众号配置。
#[derive(Debug, Clone, Default)]
pub struct WechatMpConfig {
    pub app_id: String,
    pub app_secret: String,
    pub server_token: String,
    pub server_secret: String,
}

impl WechatMpConfig {
    /// 是否具备调用微信 API 的最小配置（appId + appSecret）。
    #[must_use]
    pub fn is_configured(&self) -> bool {
        !self.app_id.trim().is_empty() && !self.app_secret.trim().is_empty()
    }

    /// 缺失项（用于可操作错误信息）。
    #[must_use]
    pub fn missing(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.app_id.trim().is_empty() {
            missing.push("wechatAppId");
        }
        if self.app_secret.trim().is_empty() {
            missing.push("wechatAppSecret");
        }
        missing
    }
}

/// 从 `config` 表读取公众号配置（全部为空 → None）。
///
/// # Errors
/// 数据库读取失败。
pub async fn load_mp_config(repos: &Repositories) -> Result<Option<WechatMpConfig>> {
    let map: HashMap<String, String> = repos
        .system
        .config_basic
        .find_by_codes(&CONFIG_CODES)
        .await?
        .into_iter()
        .collect();
    let get = |key: &str| map.get(key).cloned().unwrap_or_default();
    let cfg = WechatMpConfig {
        app_id: get("wechatAppId"),
        app_secret: get("wechatAppSecret"),
        server_token: get("wechatServerToken"),
        server_secret: get("wechatServerSecret"),
    };
    if cfg.app_id.is_empty() && cfg.app_secret.is_empty() {
        Ok(None)
    } else {
        Ok(Some(cfg))
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

type TokenCache = Mutex<HashMap<String, (String, u64)>>;
static TOKEN_CACHE: OnceLock<TokenCache> = OnceLock::new();

fn token_cache() -> &'static TokenCache {
    TOKEN_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_get(app_id: &str) -> Option<String> {
    let cache = token_cache().lock().ok()?;
    let (token, expires_at) = cache.get(app_id)?;
    if now_secs() + TOKEN_EXPIRY_MARGIN_SECS >= *expires_at {
        return None;
    }
    Some(token.clone())
}

fn cache_put(app_id: &str, token: &str, expires_in: u64) {
    if let Ok(mut cache) = token_cache().lock() {
        let expires_at = now_secs() + expires_in.min(TOKEN_TTL_SECS);
        cache.insert(app_id.to_string(), (token.to_string(), expires_at));
    }
}

/// 清除 token 缓存（微信返回 40001/42001 等 token 失效码后可重试）。
pub fn invalidate_token(app_id: &str) {
    if let Ok(mut cache) = token_cache().lock() {
        cache.remove(app_id);
    }
}

/// 稳定版 access_token 接口 URL（`POST /cgi-bin/stable_token`，微信官方推荐，
/// 替代老接口 `GET /cgi-bin/token`）。
pub const STABLE_TOKEN_URL: &str = "https://api.weixin.qq.com/cgi-bin/stable_token";

/// 构造稳定版 access_token 请求体（纯函数，便于测试）。
///
/// `force_refresh` 恒为 `false`：有效期内重复调用不刷新 token、与老接口
/// 获取的 token 互相隔离；`true` 每天限 20 次且间隔 30s，仅供 token
/// 泄漏时强制刷新，不要开启。
#[must_use]
pub fn stable_token_request_body(app_id: &str, app_secret: &str) -> Value {
    serde_json::json!({
        "grant_type": "client_credential",
        "appid": app_id,
        "secret": app_secret,
        "force_refresh": false,
    })
}

/// 获取 `client_credential` access_token（稳定版接口 + 进程内缓存，公众号 /
/// 小程序共用同一份缓存）：先查缓存，未命中才 POST `stable_token` 并回填。
///
/// # Errors
/// 网络错误 / 微信 errcode / 响应缺少 `access_token`。
pub async fn stable_access_token(app_id: &str, app_secret: &str) -> Result<String> {
    if let Some(token) = cache_get(app_id) {
        return Ok(token);
    }
    let body = stable_token_request_body(app_id, app_secret);
    let resp = reqwest::Client::new()
        .post(STABLE_TOKEN_URL)
        .json(&body)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| Error::Internal(format!("微信接口请求失败: {e}")))?;
    let status = resp.status();
    let raw = resp
        .text()
        .await
        .map_err(|e| Error::Internal(format!("微信响应读取失败: {e}")))?;
    if !status.is_success() {
        return Err(Error::Internal(format!("微信接口 HTTP {status}: {raw}")));
    }
    let value = parse_wechat_response(&raw)?;
    let token = value
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Internal(format!("微信 token 响应缺少 access_token: {raw}")))?
        .to_string();
    let expires_in = value
        .get("expires_in")
        .and_then(Value::as_u64)
        .unwrap_or(TOKEN_TTL_SECS);
    cache_put(app_id, &token, expires_in);
    Ok(token)
}

/// 校验微信通用响应：`errcode == 0` 或不存在 → 返回 JSON；否则业务错误。
///
/// # Errors
/// 微信返回 `errcode != 0`。
pub fn parse_wechat_response(raw: &str) -> Result<Value> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|e| Error::Internal(format!("微信响应解析失败: {e}; body={raw}")))?;
    if let Some(code) = value.get("errcode").and_then(Value::as_i64) {
        if code != 0 {
            let msg = value
                .get("errmsg")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            return Err(Error::biz(1001, format!("微信接口错误({code}): {msg}")));
        }
    }
    Ok(value)
}

/// 公众号 API 客户端。
pub struct WechatClient {
    cfg: WechatMpConfig,
    http: reqwest::Client,
}

impl WechatClient {
    #[must_use]
    pub fn new(cfg: WechatMpConfig) -> Self {
        Self {
            cfg,
            http: reqwest::Client::new(),
        }
    }

    /// 从 `config` 表装配客户端（未配置 → 明确业务错误）。
    ///
    /// # Errors
    /// 未配置 / 数据库错误。
    pub async fn from_repos(repos: &Repositories) -> Result<Self> {
        let cfg = load_mp_config(repos).await?;
        let Some(cfg) = cfg else {
            return Err(Error::biz(
                1001,
                format!(
                    "微信配置未迁移：缺少 {}（设置-微信设置）",
                    WechatMpConfig::default().missing().join("/")
                ),
            ));
        };
        if !cfg.is_configured() {
            return Err(Error::biz(
                1001,
                format!(
                    "微信配置不完整：缺少 {}（设置-微信设置）",
                    cfg.missing().join("/")
                ),
            ));
        }
        Ok(Self::new(cfg))
    }

    #[must_use]
    pub fn config(&self) -> &WechatMpConfig {
        &self.cfg
    }

    /// access token（进程内缓存）。
    ///
    /// # Errors
    /// 网络错误 / 微信 errcode。
    pub async fn access_token(&self) -> Result<String> {
        stable_access_token(&self.cfg.app_id, &self.cfg.app_secret).await
    }

    /// 客服列表（`cgi-bin/customservice/getkflist`）。
    ///
    /// # Errors
    /// 网络 / 微信 errcode。
    pub async fn kf_list(&self) -> Result<Vec<Value>> {
        let token = self.access_token().await?;
        let url = format!(
            "{API_BASE}/cgi-bin/customservice/getkflist?access_token={}",
            urlencode(&token)
        );
        let raw = self.get_text(&url).await?;
        let value = parse_wechat_response(&raw)?;
        Ok(value
            .get("kf_list")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default())
    }

    /// 发送客服文本消息（`cgi-bin/message/custom/send`）。
    ///
    /// # Errors
    /// 网络 / 微信 errcode。
    pub async fn send_custom_text(&self, openid: &str, text: &str) -> Result<()> {
        let token = self.access_token().await?;
        let url = format!(
            "{API_BASE}/cgi-bin/message/custom/send?access_token={}",
            urlencode(&token)
        );
        let body = serde_json::json!({
            "touser": openid,
            "msgtype": "text",
            "text": { "content": text },
        });
        let raw = self.post_json(&url, &body).await?;
        parse_wechat_response(&raw).map(|_| ())
    }

    pub async fn get_text(&self, url: &str) -> Result<String> {
        let resp = self
            .http
            .get(url)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
            .map_err(|e| Error::Internal(format!("微信接口请求失败: {e}")))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| Error::Internal(format!("微信响应读取失败: {e}")))?;
        if !status.is_success() {
            return Err(Error::Internal(format!(
                "微信接口 HTTP {status}: {body}"
            )));
        }
        Ok(body)
    }

    pub async fn post_json(&self, url: &str, body: &Value) -> Result<String> {
        let resp = self
            .http
            .post(url)
            .json(body)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
            .map_err(|e| Error::Internal(format!("微信接口请求失败: {e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| Error::Internal(format!("微信响应读取失败: {e}")))?;
        if !status.is_success() {
            return Err(Error::Internal(format!(
                "微信接口 HTTP {status}: {text}"
            )));
        }
        Ok(text)
    }
}

/// 小程序 `jscode2session`（登录/支付 code 换取 openid）。
///
/// # Errors
/// 网络 / 微信 errcode。
pub async fn code2session(
    app_id: &str,
    app_secret: &str,
    code: &str,
) -> Result<Value> {
    let url = format!(
        "{API_BASE}/sns/jscode2session?appid={}&secret={}&js_code={}&grant_type=authorization_code",
        urlencode(app_id),
        urlencode(app_secret),
        urlencode(code)
    );
    let resp = reqwest::Client::new()
        .get(url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| Error::Internal(format!("微信 code2session 请求失败: {e}")))?;
    let body = resp
        .text()
        .await
        .map_err(|e| Error::Internal(format!("微信响应读取失败: {e}")))?;
    parse_wechat_response(&body)
}

/// 极简 percent-encoding（仅用于 query 参数值；微信 token/appid 均为 ASCII）。
pub fn urlencode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_requires_app_id_and_secret() {
        let empty = WechatMpConfig::default();
        assert!(!empty.is_configured());
        assert_eq!(empty.missing(), vec!["wechatAppId", "wechatAppSecret"]);
        let cfg = WechatMpConfig {
            app_id: "wx123".into(),
            app_secret: "secret".into(),
            ..Default::default()
        };
        assert!(cfg.is_configured());
        assert!(cfg.missing().is_empty());
    }

    #[test]
    fn parse_success_and_error_responses() {
        let ok = parse_wechat_response(r#"{"access_token":"tok","expires_in":7200}"#).unwrap();
        assert_eq!(ok["access_token"], "tok");
        let err = parse_wechat_response(r#"{"errcode":40013,"errmsg":"invalid appid"}"#).unwrap_err();
        match err {
            Error::Biz { code, msg } => {
                assert_eq!(code, 1001);
                assert!(msg.contains("40013"));
                assert!(msg.contains("invalid appid"));
            }
            other => panic!("期望业务错误，得到 {other:?}"),
        }
    }

    #[test]
    fn token_cache_expiry() {
        invalidate_token("wx-test-cache");
        assert!(cache_get("wx-test-cache").is_none());
        cache_put("wx-test-cache", "token-1", 7200);
        assert_eq!(cache_get("wx-test-cache").as_deref(), Some("token-1"));
        // 立即过期（expires_in 小于 margin 时也不缓存）
        cache_put("wx-test-cache", "token-2", 10);
        assert!(cache_get("wx-test-cache").is_none());
        invalidate_token("wx-test-cache");
    }

    #[test]
    fn url_encoding_keeps_ascii_and_encodes_symbols() {
        assert_eq!(urlencode("wx123abc"), "wx123abc");
        assert_eq!(urlencode("a b&c"), "a%20b%26c");
    }

    #[test]
    fn stable_token_body_is_client_credential_without_force_refresh() {
        let body = stable_token_request_body("wx123", "s3cr3t");
        assert_eq!(body["grant_type"], "client_credential");
        assert_eq!(body["appid"], "wx123");
        assert_eq!(body["secret"], "s3cr3t");
        assert_eq!(body["force_refresh"], false);
    }

    #[test]
    fn stable_token_url_uses_stable_endpoint_not_legacy_token() {
        assert_eq!(
            STABLE_TOKEN_URL,
            "https://api.weixin.qq.com/cgi-bin/stable_token"
        );
        assert!(STABLE_TOKEN_URL.contains("/cgi-bin/stable_token"));
        assert!(!STABLE_TOKEN_URL.contains("/cgi-bin/token?"));
        assert!(!STABLE_TOKEN_URL.contains("grant_type=client_credential"));
    }

    #[tokio::test]
    async fn stable_access_token_hits_shared_cache_without_network() {
        invalidate_token("wx-test-stable");
        cache_put("wx-test-stable", "cached-token", 7200);
        let token = stable_access_token("wx-test-stable", "unused-secret")
            .await
            .unwrap();
        assert_eq!(token, "cached-token");
        invalidate_token("wx-test-stable");
    }
}
