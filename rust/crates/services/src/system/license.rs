//! 授权系统客户端（system::license，**我方新增**）。
//!
//! 对接独立授权中心 `auth.recy.cc`（并行建设的独立系统，不在本仓库内）：
//! 本实例定期把「实例身份 + 管理端访问地址 + 本站域名 + 产品版本」上报
//! 授权中心，中心按域名绑定判定授权状态与档位（商业版 / 企业版），判定
//! 结果驱动管理端的授权徽标与菜单版本控制（菜单显隐在 admin-semi
//! `src/license.ts` 的集中映射表里消费本模块缓存的判定）。
//!
//! # 存储（零迁移）
//!
//! 全部状态存 `config` 表单行 `biz_code = "licenseSettings"`，`biz_val`
//! 为整块 JSON（与 `auctionSettings` / `federationSettings` 同属「单 code
//! 整块 JSON」型），经 `repos.system.config_basic` 读写：
//!
//! ```json
//! {
//!   "version": 1,
//!   "instanceKey": "<UUID，首次生成后固定>",
//!   "lastAdminHost": "admin.example.com",
//!   "lastCheckinAt": 1760000000,
//!   "lastAttemptAt": 1760000000,
//!   "lastError": null,
//!   "verdict": { "status": "authorized", "tier": "commercial", "...": "..." }
//! }
//! ```
//!
//! # 上报契约（与授权中心平台线共享，逐字一致）
//!
//! `POST {center}/api/instances/checkin`，请求体
//! `{instanceKey, adminHost, siteDomains: [], appVersion}`；响应
//! `{status, tier, expiresAt, subdomainLimit, subdomainsUsed,
//! checkedAt}`（到期时另带 `expired: true`）+ 响应体 `signature`
//! 字段的 HMAC 签名。签名口径以授权中心平台实装为准（auth-semi
//! `src/license.js`，2026-11-21 对齐）：签名密钥 =
//! `SHA-256("auth-semi-sign-v1:" + instanceKey)` 原始 32 字节；签名
//! 载荷 = `[status, tier, expiresAt, subdomainLimit, subdomainsUsed,
//! checkedAt]` 以 `|` 拼接（null 以空串参与、其余转十进制字符串，
//! `expired` 不参与签名）；签名 = HMAC-SHA256 小写 hex，见
//! [`verify_checkin_response`]。**验签不过的响应一律不采信**，
//! 缓存判定保持原样。
//!
//! # fail-soft 铁律
//!
//! 授权中心不可达 / 响应异常 / 验签失败 → 只记 `lastError` 并等下个周期
//! 重试，**绝不阻塞启动与登录、绝不锁死管理端、绝不影响前台与下单**。
//! 本模块全部公开方法都不向上抛错：上报失败静默（warn 日志），状态
//! 查询在从未成功上报过时返回 `status = "unknown"`。

use std::sync::Arc;

use hmac::{Hmac, Mac};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use nextrade_common::utils;
use nextrade_infrastructure::db::Repositories;

/// config 表中的业务编码。
pub const LICENSE_SETTINGS_CODE: &str = "licenseSettings";

/// 授权中心默认地址（可用 env `NEXTTRADE_LICENSE__CENTER_URL` 覆盖，
/// 供联调 / 自建授权中心时切换；不带尾斜杠）。
pub const DEFAULT_LICENSE_CENTER_URL: &str = "https://auth.recy.cc";

/// 上报路径（授权中心契约）。
const CHECKIN_PATH: &str = "/api/instances/checkin";

/// 本站域名来源的 config 行（站点设置里维护的 PC / H5 域名）。
const SITE_DOMAIN_CODES: [&str; 2] = ["pcDomain", "h5Domain"];

type HmacSha256 = Hmac<Sha256>;

/// 授权判定（仅来自验签通过的响应；字段与授权中心契约同名）。
#[derive(Debug, Clone, PartialEq)]
pub struct LicenseVerdict {
    /// `authorized` / `unauthorized`
    pub status: String,
    /// `commercial` / `enterprise` / None（未授权时）
    pub tier: Option<String>,
    /// 到期时间（授权中心原样字符串，如 RFC3339；无固定期限时为 None）
    pub expires_at: Option<String>,
    /// 是否已过期
    pub expired: bool,
    /// 企业版泛域名下子域名额度（商业版为 None）
    pub subdomain_limit: Option<i64>,
    /// 已用子域名数
    pub subdomains_used: Option<i64>,
    /// 授权中心判定时刻（原样字符串）
    pub checked_at: Option<String>,
}

/// `licenseSettings` 存储块（内部形态；序列化见 `settings_to_json`）。
#[derive(Debug, Clone, Default, PartialEq)]
struct LicenseSettings {
    instance_key: Option<String>,
    last_admin_host: Option<String>,
    last_checkin_at: Option<i64>,
    last_attempt_at: Option<i64>,
    last_error: Option<String>,
    verdict: Option<LicenseVerdict>,
}

fn get_str(obj: &Map<String, Value>, key: &str) -> Option<String> {
    obj.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn get_i64(obj: &Map<String, Value>, key: &str) -> Option<i64> {
    match obj.get(key) {
        Some(Value::Number(n)) => n.as_i64(),
        Some(Value::String(s)) => s.trim().parse::<i64>().ok(),
        _ => None,
    }
}

fn verdict_from_json(v: &Value) -> Option<LicenseVerdict> {
    let obj = v.as_object()?;
    let status = get_str(obj, "status")?;
    if status != "authorized" && status != "unauthorized" {
        return None;
    }
    let tier = get_str(obj, "tier").filter(|t| t == "commercial" || t == "enterprise");
    Some(LicenseVerdict {
        status,
        tier,
        expires_at: get_str(obj, "expiresAt"),
        expired: obj.get("expired").and_then(Value::as_bool).unwrap_or(false),
        subdomain_limit: get_i64(obj, "subdomainLimit"),
        subdomains_used: get_i64(obj, "subdomainsUsed"),
        checked_at: get_str(obj, "checkedAt"),
    })
}

fn verdict_to_json(v: &LicenseVerdict) -> Value {
    json!({
        "status": v.status,
        "tier": v.tier,
        "expiresAt": v.expires_at,
        "expired": v.expired,
        "subdomainLimit": v.subdomain_limit,
        "subdomainsUsed": v.subdomains_used,
        "checkedAt": v.checked_at,
    })
}

/// 解析存储块：非 JSON / 非对象 / 字段非法一律回落到空设置（fail-soft，
/// 最坏结果是重新生成 instanceKey 与状态 unknown，不抛错）。
fn parse_settings(raw: Option<&str>) -> LicenseSettings {
    let Some(raw) = raw else {
        return LicenseSettings::default();
    };
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return LicenseSettings::default();
    };
    let Some(obj) = value.as_object() else {
        return LicenseSettings::default();
    };
    LicenseSettings {
        instance_key: get_str(obj, "instanceKey").filter(|k| uuid::Uuid::parse_str(k).is_ok()),
        last_admin_host: get_str(obj, "lastAdminHost"),
        last_checkin_at: get_i64(obj, "lastCheckinAt"),
        last_attempt_at: get_i64(obj, "lastAttemptAt"),
        last_error: get_str(obj, "lastError"),
        verdict: obj.get("verdict").and_then(verdict_from_json),
    }
}

fn settings_to_json(s: &LicenseSettings) -> Value {
    json!({
        "version": 1,
        "instanceKey": s.instance_key,
        "lastAdminHost": s.last_admin_host,
        "lastCheckinAt": s.last_checkin_at,
        "lastAttemptAt": s.last_attempt_at,
        "lastError": s.last_error,
        "verdict": s.verdict.as_ref().map(verdict_to_json),
    })
}

/// 状态口输出（`GET /adminapi/license/status` 的 data）：从未成功上报过
/// （无缓存判定）→ `status = "unknown"`，其余判定字段为 null。
fn status_vo(s: &LicenseSettings) -> Value {
    match &s.verdict {
        Some(v) => json!({
            "status": v.status,
            "tier": v.tier,
            "expiresAt": v.expires_at,
            "expired": v.expired,
            "subdomainLimit": v.subdomain_limit,
            "subdomainsUsed": v.subdomains_used,
            "checkedAt": v.checked_at,
            "lastCheckinAt": s.last_checkin_at,
            "instanceKey": s.instance_key,
        }),
        None => json!({
            "status": "unknown",
            "tier": null,
            "expiresAt": null,
            "expired": false,
            "subdomainLimit": null,
            "subdomainsUsed": null,
            "checkedAt": null,
            "lastCheckinAt": s.last_checkin_at,
            "instanceKey": s.instance_key,
        }),
    }
}

/// 域名归一化：去协议 / 路径 / 端口 / 尾点，转小写；空值与非法值 → None。
/// `pcDomain` / `h5Domain` 的存量值可能是 `https://a.com/`、`a.com`、
/// `a.com:8443` 等形态，上报与绑定判定只认 host。
#[must_use]
pub fn normalize_domain(raw: &str) -> Option<String> {
    let mut host = raw.trim();
    if let Some(idx) = host.find("://") {
        host = &host[idx + 3..];
    }
    host = host.split(['/', '?', '#']).next().unwrap_or("");
    // 去掉 userinfo 与端口（域名绑定不认端口）
    if let Some(idx) = host.rfind('@') {
        host = &host[idx + 1..];
    }
    if let Some(idx) = host.rfind(':') {
        host = &host[..idx];
    }
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty() || host.contains(char::is_whitespace) {
        None
    } else {
        Some(host)
    }
}

/// 签名密钥派生（平台口径）：`SHA-256("auth-semi-sign-v1:" + instanceKey)`
/// 原始 32 字节。
fn derive_signing_key(instance_key: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"auth-semi-sign-v1:");
    hasher.update(instance_key.as_bytes());
    let digest = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&digest);
    key
}

fn hmac_hex(key: &[u8; 32], payload: &[u8]) -> String {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(key).expect("HMAC 接受任意长度密钥");
    mac.update(payload);
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// 常量时间字符串比对（与联邦 `constant_time_eq` 同语义，本地实现避免
/// 跨模块私有函数耦合）。
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// 签名载荷单字段的字符串形态（平台 `signResponse` 拼接语义）：null →
/// 空串；字符串 → 原样；整数 → 十进制字符串。字段缺失或类型不符
/// （布尔 / 浮点 / 对象 / 数组）→ None（按验签失败处理）。
fn sign_field(obj: &Map<String, Value>, key: &str) -> Option<String> {
    match obj.get(key)? {
        Value::Null => Some(String::new()),
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(i.to_string())
            } else {
                n.as_u64().map(|u| u.to_string())
            }
        }
        _ => None,
    }
}

/// 签名载荷（平台口径）：六字段按
/// `[status, tier, expiresAt, subdomainLimit, subdomainsUsed, checkedAt]`
/// 顺序以 `|` 拼接；`expired` 不参与签名。
fn signature_payload(obj: &Map<String, Value>) -> Option<String> {
    let fields = [
        sign_field(obj, "status")?,
        sign_field(obj, "tier")?,
        sign_field(obj, "expiresAt")?,
        sign_field(obj, "subdomainLimit")?,
        sign_field(obj, "subdomainsUsed")?,
        sign_field(obj, "checkedAt")?,
    ];
    Some(fields.join("|"))
}

/// 验签并解析 checkin 响应（纯函数，便于单测）：取响应体 `signature`
/// 字段，按平台口径拼六字段载荷验 HMAC-SHA256。签名缺失 / 任一签名
/// 字段缺失或类型不符 / 验签不过 / 判定字段非法 → None（调用方不
/// 采信、不动缓存判定）。
#[must_use]
pub fn verify_checkin_response(instance_key: &str, body_text: &str) -> Option<LicenseVerdict> {
    let value: Value = serde_json::from_str(body_text).ok()?;
    let obj = value.as_object()?;
    let sig = obj.get("signature").and_then(Value::as_str)?;
    let payload = signature_payload(obj)?;
    let key = derive_signing_key(instance_key);
    let expected = hmac_hex(&key, payload.as_bytes());
    if !constant_time_eq(
        expected.as_bytes(),
        sig.trim().to_ascii_lowercase().as_bytes(),
    ) {
        return None;
    }
    verdict_from_json(&value)
}

/// 组装 checkin 请求体（契约四字段，纯函数便于单测）。
fn build_checkin_body(
    instance_key: &str,
    admin_host: &str,
    site_domains: &[String],
    app_version: &str,
) -> Value {
    json!({
        "instanceKey": instance_key,
        "adminHost": admin_host,
        "siteDomains": site_domains,
        "appVersion": app_version,
    })
}

/// 授权中心基地址（env 覆盖优先）。
fn center_base_url() -> String {
    std::env::var("NEXTTRADE_LICENSE__CENTER_URL")
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_LICENSE_CENTER_URL.to_string())
}

/// 授权系统客户端服务。
pub struct LicenseService {
    repos: Arc<Repositories>,
}

impl LicenseService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    async fn load_settings(&self) -> LicenseSettings {
        match self
            .repos
            .system
            .config_basic
            .find_by_code(LICENSE_SETTINGS_CODE)
            .await
        {
            Ok(raw) => parse_settings(raw.as_deref()),
            Err(e) => {
                tracing::warn!("授权设置读取失败（按空设置处理）: {e}");
                LicenseSettings::default()
            }
        }
    }

    async fn save_settings(&self, settings: &LicenseSettings) {
        let text = settings_to_json(settings).to_string();
        if let Err(e) = self
            .repos
            .system
            .config_basic
            .save_code(LICENSE_SETTINGS_CODE, &text)
            .await
        {
            tracing::warn!("授权设置写入失败（忽略，下次上报重试）: {e}");
        }
    }

    /// 确保实例身份存在：无有效 instanceKey 时生成 UUID 并落库，随后回读
    /// 以落库值为准（并发首启时后写者赢、回读收敛到同一值）。
    async fn ensure_instance_key(&self, settings: &mut LicenseSettings) -> Option<String> {
        if let Some(key) = &settings.instance_key {
            return Some(key.clone());
        }
        let generated = uuid::Uuid::new_v4().to_string();
        settings.instance_key = Some(generated);
        self.save_settings(settings).await;
        let stored = self.load_settings().await;
        if let Some(key) = stored.instance_key {
            settings.instance_key = Some(key.clone());
            return Some(key);
        }
        settings.instance_key.clone()
    }

    /// 本站域名集合：`pcDomain` + `h5Domain` 两行配置，归一化去重。
    async fn site_domains(&self) -> Vec<String> {
        let rows = self
            .repos
            .system
            .config_basic
            .find_by_codes(&SITE_DOMAIN_CODES)
            .await
            .unwrap_or_default();
        let mut out: Vec<String> = Vec::new();
        for (_, raw) in rows {
            if let Some(host) = normalize_domain(&raw) {
                if !out.contains(&host) {
                    out.push(host);
                }
            }
        }
        out
    }

    /// 当前缓存判定（状态口）。永不失败：读不到缓存即 unknown。
    pub async fn status(&self) -> Value {
        let settings = self.load_settings().await;
        status_vo(&settings)
    }

    /// 执行一次 checkin 上报（fail-soft：任何失败只记账不抛错）。
    ///
    /// `admin_host_hint`：调用方能拿到请求 Host 时传入（管理员登录触发）；
    /// 周期上报传 None，依次回落到缓存的上次 adminHost、PC 域名。
    pub async fn checkin(&self, admin_host_hint: Option<String>) {
        let mut settings = self.load_settings().await;
        settings.last_attempt_at = Some(utils::now_seconds());

        let Some(instance_key) = self.ensure_instance_key(&mut settings).await else {
            settings.last_error = Some("instance_key_unavailable".to_string());
            self.save_settings(&settings).await;
            return;
        };

        let domains = self.site_domains().await;
        let admin_host = admin_host_hint
            .as_deref()
            .and_then(normalize_domain)
            .or_else(|| settings.last_admin_host.clone())
            .or_else(|| domains.first().cloned())
            .unwrap_or_default();
        if let Some(hint) = admin_host_hint.as_deref().and_then(normalize_domain) {
            settings.last_admin_host = Some(hint);
        }

        let body = build_checkin_body(
            &instance_key,
            &admin_host,
            &domains,
            nextrade_common::version::PRODUCT_VERSION,
        );
        let url = format!("{}{}", center_base_url(), CHECKIN_PATH);
        let outcome = async {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|e| format!("client_build: {e}"))?;
            let resp = client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("network: {e}"))?;
            let status = resp.status();
            let text = resp.text().await.map_err(|e| format!("read_body: {e}"))?;
            if !status.is_success() {
                return Err(format!("http_{}", status.as_u16()));
            }
            Ok(text)
        }
        .await;

        match outcome {
            Ok(text) => {
                match verify_checkin_response(&instance_key, &text) {
                    Some(verdict) => {
                        settings.verdict = Some(verdict);
                        settings.last_checkin_at = Some(utils::now_seconds());
                        settings.last_error = None;
                    }
                    None => {
                        // 验签不过 / 响应形态非法：不采信，缓存判定保持原样
                        settings.last_error = Some("signature_or_payload".to_string());
                        tracing::warn!("授权中心响应验签失败或形态非法，本次不采信");
                    }
                }
            }
            Err(e) => {
                settings.last_error = Some(e);
                tracing::warn!(
                    "授权上报失败（静默，等待下个周期重试）: {}",
                    settings.last_error.as_deref().unwrap_or_default()
                );
            }
        }
        self.save_settings(&settings).await;
    }

    /// 登录触发的异步上报：fire-and-forget，绝不阻塞登录请求。
    pub fn spawn_checkin(repos: Arc<Repositories>, admin_host_hint: Option<String>) {
        tokio::spawn(async move {
            LicenseService::new(repos).checkin(admin_host_hint).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_domain_variants() {
        assert_eq!(
            normalize_domain("https://Admin.Recy.cc/"),
            Some("admin.recy.cc".to_string())
        );
        assert_eq!(
            normalize_domain("https://battery.recy.cc/mobile"),
            Some("battery.recy.cc".to_string())
        );
        assert_eq!(
            normalize_domain("battery.recy.cc"),
            Some("battery.recy.cc".to_string())
        );
        assert_eq!(
            normalize_domain("battery.recy.cc:8443"),
            Some("battery.recy.cc".to_string())
        );
        assert_eq!(
            normalize_domain("  https://user@a.com:443/x?y=1 "),
            Some("a.com".to_string())
        );
        assert_eq!(normalize_domain(""), None);
        assert_eq!(normalize_domain("https://"), None);
        assert_eq!(normalize_domain("a b.com"), None);
    }

    #[test]
    fn settings_roundtrip_and_garbage_fallback() {
        let s = LicenseSettings {
            instance_key: Some("3f50b1a4-1f5c-4e52-9a1d-2c3b8f7a6d01".to_string()),
            last_admin_host: Some("admin.recy.cc".to_string()),
            last_checkin_at: Some(123),
            last_attempt_at: Some(124),
            last_error: None,
            verdict: Some(LicenseVerdict {
                status: "authorized".to_string(),
                tier: Some("enterprise".to_string()),
                expires_at: Some("2027-01-01T00:00:00Z".to_string()),
                expired: false,
                subdomain_limit: Some(5),
                subdomains_used: Some(2),
                checked_at: Some("2026-11-21T15:00:00Z".to_string()),
            }),
        };
        let parsed = parse_settings(Some(&settings_to_json(&s).to_string()));
        assert_eq!(parsed, s);
        // 垃圾输入回落空设置
        assert_eq!(parse_settings(Some("not json")), LicenseSettings::default());
        assert_eq!(parse_settings(Some("[1]")), LicenseSettings::default());
        assert_eq!(parse_settings(None), LicenseSettings::default());
        // 非法 instanceKey（非 UUID）被丢弃
        let bad = parse_settings(Some(r#"{"instanceKey":"abc"}"#));
        assert_eq!(bad.instance_key, None);
    }

    #[test]
    fn status_vo_unknown_without_verdict() {
        let vo = status_vo(&LicenseSettings::default());
        assert_eq!(vo["status"], "unknown");
        assert_eq!(vo["tier"], Value::Null);
    }

    /// 按平台口径给一组判定字段签名，返回带 `signature` 的响应体。
    fn signed_body(instance_key: &str, fields: &[(&str, Value)]) -> String {
        let mut obj = Map::new();
        for (k, v) in fields {
            obj.insert((*k).to_string(), v.clone());
        }
        let payload = signature_payload(&obj).expect("测试字段应可拼载荷");
        let sig = hmac_hex(&derive_signing_key(instance_key), payload.as_bytes());
        obj.insert("signature".to_string(), json!(sig));
        Value::Object(obj).to_string()
    }

    #[test]
    fn verify_accepts_platform_signature_and_rejects_tamper() {
        let key = "3f50b1a4-1f5c-4e52-9a1d-2c3b8f7a6d01";
        let body = signed_body(
            key,
            &[
                ("status", json!("authorized")),
                ("tier", json!("commercial")),
                ("expiresAt", json!("2027-01-01T00:00:00Z")),
                ("expired", json!(false)),
                ("subdomainLimit", json!(1)),
                ("subdomainsUsed", json!(1)),
                ("checkedAt", json!("2026-11-21T15:00:00Z")),
            ],
        );
        let verdict = verify_checkin_response(key, &body).expect("验签应通过");
        assert_eq!(verdict.status, "authorized");
        assert_eq!(verdict.tier.as_deref(), Some("commercial"));

        // 篡改任一签名字段 → 验签失败
        let tampered = body.replace("commercial", "enterprise");
        assert!(verify_checkin_response(key, &tampered).is_none());
        let tampered = body.replace("\"subdomainsUsed\":1", "\"subdomainsUsed\":2");
        assert!(verify_checkin_response(key, &tampered).is_none());
        // 错实例密钥 → 验签失败
        assert!(verify_checkin_response("0e0e0e0e-1111-4222-8333-444444444444", &body).is_none());
        // 无签名 → 不采信
        let unsigned = r#"{"status":"authorized","tier":"enterprise","expiresAt":null,"subdomainLimit":null,"subdomainsUsed":null,"checkedAt":"2026-11-21T15:00:00Z"}"#;
        assert!(verify_checkin_response(key, unsigned).is_none());
        // 签名字段缺失（六字段不全）→ 不采信
        let missing = r#"{"status":"authorized","tier":"enterprise","signature":"00"}"#;
        assert!(verify_checkin_response(key, missing).is_none());
    }

    #[test]
    fn verify_expired_flag_not_signed_and_null_fields() {
        let key = "3f50b1a4-1f5c-4e52-9a1d-2c3b8f7a6d01";
        // unauthorized + 全部可空字段为 null：null 以空串参与签名
        let body = signed_body(
            key,
            &[
                ("status", json!("unauthorized")),
                ("tier", Value::Null),
                ("expiresAt", Value::Null),
                ("subdomainLimit", Value::Null),
                ("subdomainsUsed", Value::Null),
                ("checkedAt", json!("2026-11-21T15:00:00Z")),
            ],
        );
        let verdict = verify_checkin_response(key, &body).expect("验签应通过");
        assert_eq!(verdict.status, "unauthorized");
        assert_eq!(verdict.tier, None);
        assert!(!verdict.expired);

        // expired 不参与签名：事后给响应加 expired:true 不影响验签，判定读取新值
        let flipped = body.replace("\"checkedAt\"", "\"expired\":true,\"checkedAt\"");
        let verdict = verify_checkin_response(key, &flipped).expect("expired 不参与签名应仍验过");
        assert!(verdict.expired);
    }

    #[test]
    fn checkin_body_matches_contract() {
        let body = build_checkin_body(
            "k",
            "admin.recy.cc",
            &["battery.recy.cc".to_string()],
            "0.0.2",
        );
        assert_eq!(body["instanceKey"], "k");
        assert_eq!(body["adminHost"], "admin.recy.cc");
        assert_eq!(body["siteDomains"][0], "battery.recy.cc");
        assert_eq!(body["appVersion"], "0.0.2");
    }
}
