//! 微信小程序 API 客户端（分享直达小程序基础设施，论坛帖 907763）。
//!
//! 与 [`super::wechat_client`]（公众号客户端）分工：公众号接口用
//! `wechatAppId`/`wechatAppSecret`，**小程序接口（wxa/*）必须用小程序自身
//! 凭据** `wechatMiniProgramAppId`/`wechatMiniProgramSecret`（`config` 表，
//! 与 Java `SettingsEnum` 一致，后台「支付设置 → 微信小程序支付」组维护，
//! 小程序登录/手机号链已在用同一对配置位）。此前 `system::util::mini_code`
//! 误用公众号凭据调 `getwxacodeunlimit`，必然被微信拒绝（api unauthorized）
//! 且被静默吞掉回退相对路径二维码——本模块即为纠正该口径而建。
//!
//! ## access_token 基础设施
//!
//! - 获取：稳定版接口 `POST /cgi-bin/stable_token`（`force_refresh=false`，
//!   与公众号客户端同口径，请求体构造复用
//!   [`super::wechat_client::stable_token_request_body`]）；
//! - 缓存：全局缓存服务（Redis 优先、多实例共享；Redis 不可用时自动降级
//!   进程内内存），键 `wechat:mini:access_token:{appId}`，值含绝对过期时间，
//!   提前 [`TOKEN_REFRESH_MARGIN_SECS`] 秒视为过期（过期刷新）；
//! - 并发安全：按 appId 分片的 `tokio::sync::Mutex` 单飞（single-flight）——
//!   同一实例内并发未命中只发一次获取请求，锁内二次查缓存；跨实例并发因
//!   稳定版接口在有效期内返回同一 token 而天然无害；
//! - 失效重试：下游接口返回 40001/42001（token 无效/过期）时清缓存并
//!   重取一次（与小程序登录链既有口径一致）。
//!
//! ## 降级铁律
//!
//! 本模块只负责「配置齐全时真调微信」：未配置时 [`WechatMiniConfig::missing`]
//! 给出缺失项，调用方（`system::util::share_link`）据此**明示降级**回退 H5
//! 链接并在响应中标注 `degraded/notice`，绝不伪造小程序码/链接成功。
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::cache::get_cache;
use nextrade_infrastructure::db::Repositories;

use super::wechat_client::{parse_wechat_response, stable_token_request_body, urlencode};

/// 小程序 AppID 配置位（`config` 表，与小程序登录链共用）。
pub const MINI_APP_ID_CODE: &str = "wechatMiniProgramAppId";
/// 小程序 AppSecret 配置位（`config` 表，与小程序登录链共用）。
pub const MINI_APP_SECRET_CODE: &str = "wechatMiniProgramSecret";
/// H5 域名配置位（降级回退链接拼接用，与短链/买家秀链同口径）。
pub const H5_DOMAIN_CODE: &str = "h5Domain";

const API_BASE: &str = "https://api.weixin.qq.com";
/// 业务错误码（与 config_basic 的 SERVICE_DATA_ERROR 口径一致）。
const SERVICE_DATA_ERROR: i32 = 1001;
/// token 提前过期边距（秒）：剩余有效期低于此值即刷新。
const TOKEN_REFRESH_MARGIN_SECS: i64 = 300;
/// URL Link 缓存提前过期边距（秒）。
const URL_LINK_CACHE_MARGIN_SECS: u64 = 3_600;
/// scene 上限（微信 getwxacodeunlimit 规定 scene 最长 32 个可见字符）。
pub const SCENE_MAX_LEN: usize = 32;
/// scene 短码 → 参数映射的缓存键前缀（与既有 `mini_scene` 读取键逐字一致）。
pub const SCENE_CACHE_PREFIX: &str = "RUNNING:MINISCENE:CODE:";
/// scene 短码映射有效期（30 天，秒）。
pub const SCENE_TTL_SECS: u64 = 30 * 24 * 3_600;
/// URL Link 默认有效期（天，微信上限 30）。
pub const URL_LINK_EXPIRE_DAYS: u32 = 30;
/// 默认发布版环境（正式版小程序）。
pub const ENV_VERSION_RELEASE: &str = "release";

/// 小程序配置（`config` 表 `wechatMiniProgramAppId`/`wechatMiniProgramSecret`）。
#[derive(Debug, Clone, Default)]
pub struct WechatMiniConfig {
    pub app_id: String,
    pub app_secret: String,
}

impl WechatMiniConfig {
    /// 是否具备调用小程序 API 的最小配置（appId + secret）。
    #[must_use]
    pub fn is_configured(&self) -> bool {
        !self.app_id.trim().is_empty() && !self.app_secret.trim().is_empty()
    }

    /// 缺失的配置位（用于降级标注文案）。
    #[must_use]
    pub fn missing(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.app_id.trim().is_empty() {
            missing.push(MINI_APP_ID_CODE);
        }
        if self.app_secret.trim().is_empty() {
            missing.push(MINI_APP_SECRET_CODE);
        }
        missing
    }
}

/// 从 `config` 表读取小程序配置（未配置 → 全空的 `Default`，不报错）。
///
/// # Errors
/// 数据库读取失败。
pub async fn load_mini_config(repos: &Repositories) -> Result<WechatMiniConfig> {
    let map: HashMap<String, String> = repos
        .system
        .config_basic
        .find_by_codes(&[MINI_APP_ID_CODE, MINI_APP_SECRET_CODE])
        .await?
        .into_iter()
        .collect();
    Ok(WechatMiniConfig {
        app_id: map.get(MINI_APP_ID_CODE).cloned().unwrap_or_default(),
        app_secret: map.get(MINI_APP_SECRET_CODE).cloned().unwrap_or_default(),
    })
}

/// 读取单个配置值（缺行 → 空串）。
///
/// # Errors
/// 数据库读取失败。
pub async fn load_config_value(repos: &Repositories, code: &str) -> Result<String> {
    let rows = repos.system.config_basic.find_by_codes(&[code]).await?;
    Ok(rows
        .into_iter()
        .find(|(k, _)| k == code)
        .map(|(_, v)| v)
        .unwrap_or_default())
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// 缓存中的 token 记录（绝对过期时间，秒）。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedToken {
    token: String,
    expires_at: i64,
}

fn token_cache_key(app_id: &str) -> String {
    format!("wechat:mini:access_token:{app_id}")
}

type FetchLocks = Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>;
static FETCH_LOCKS: OnceLock<FetchLocks> = OnceLock::new();

/// 按 appId 取单飞锁（同一 appId 的 token 获取全局串行）。
fn fetch_lock(app_id: &str) -> Arc<tokio::sync::Mutex<()>> {
    let locks = FETCH_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = locks.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .entry(app_id.to_string())
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

/// 微信小程序 API 客户端。
pub struct WechatMiniClient {
    cfg: WechatMiniConfig,
    http: reqwest::Client,
}

impl WechatMiniClient {
    #[must_use]
    pub fn new(cfg: WechatMiniConfig) -> Self {
        Self {
            cfg,
            http: reqwest::Client::new(),
        }
    }

    /// 从 `config` 表装配客户端（未配置 → 明确业务错误，列明缺失配置位）。
    ///
    /// # Errors
    /// 未配置 / 数据库错误。
    pub async fn from_repos(repos: &Repositories) -> Result<Self> {
        let cfg = load_mini_config(repos).await?;
        if !cfg.is_configured() {
            return Err(Error::biz(
                SERVICE_DATA_ERROR,
                format!(
                    "微信小程序未配置：缺少 {}（后台「支付设置 → 微信小程序支付」填写小程序 AppID/Secret）",
                    cfg.missing().join("/")
                ),
            ));
        }
        Ok(Self::new(cfg))
    }

    #[must_use]
    pub fn config(&self) -> &WechatMiniConfig {
        &self.cfg
    }

    /// 读缓存中仍有效的 token（剩余有效期 > 刷新边距）；缓存异常按未命中处理。
    async fn read_cached_token(&self) -> Option<String> {
        let cached = get_cache()
            .get_json::<CachedToken>(&token_cache_key(&self.cfg.app_id))
            .await
            .ok()??;
        if cached.expires_at - now_secs() > TOKEN_REFRESH_MARGIN_SECS {
            Some(cached.token)
        } else {
            None
        }
    }

    /// 获取 access_token（缓存 → 单飞锁内二次查缓存 → 稳定版接口获取并回填）。
    ///
    /// # Errors
    /// 网络错误 / 微信 errcode / 响应缺少 `access_token`。
    pub async fn access_token(&self) -> Result<String> {
        if let Some(token) = self.read_cached_token().await {
            return Ok(token);
        }
        let lock = fetch_lock(&self.cfg.app_id);
        let _guard = lock.lock().await;
        if let Some(token) = self.read_cached_token().await {
            return Ok(token);
        }
        self.fetch_and_cache_token().await
    }

    async fn fetch_and_cache_token(&self) -> Result<String> {
        let body = stable_token_request_body(&self.cfg.app_id, &self.cfg.app_secret);
        let resp = self
            .http
            .post(super::wechat_client::STABLE_TOKEN_URL)
            .json(&body)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
            .map_err(|e| Error::Internal(format!("微信小程序 token 请求失败: {e}")))?;
        let status = resp.status();
        let raw = resp
            .text()
            .await
            .map_err(|e| Error::Internal(format!("微信小程序 token 响应读取失败: {e}")))?;
        if !status.is_success() {
            return Err(Error::Internal(format!(
                "微信小程序 token 接口 HTTP {status}: {raw}"
            )));
        }
        let value = parse_wechat_response(&raw)?;
        let token = value
            .get("access_token")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                Error::Internal(format!("微信小程序 token 响应缺少 access_token: {raw}"))
            })?
            .to_string();
        let expires_in = value
            .get("expires_in")
            .and_then(Value::as_i64)
            .unwrap_or(7_200);
        let record = CachedToken {
            token: token.clone(),
            expires_at: now_secs() + expires_in,
        };
        // TTL 取 expires_in 本身（值内含绝对过期时间，读端再扣边距判定）。
        let ttl = u64::try_from(expires_in).unwrap_or(7_200).max(60);
        let _ = get_cache()
            .set_json(&token_cache_key(&self.cfg.app_id), &record, ttl)
            .await;
        Ok(token)
    }

    /// 清除 token 缓存（本模块缓存 + 公众号客户端共用的进程内缓存一并清，
    /// 两者按同一 appId 键、稳定版 token 同源）。
    pub async fn invalidate_token(&self) {
        let _ = get_cache().del(&token_cache_key(&self.cfg.app_id)).await;
        super::wechat_client::invalidate_token(&self.cfg.app_id);
    }

    /// 带 token 的 POST（JSON 响应）：40001/42001 时清缓存重取 token 重试一次。
    async fn post_json_with_token_retry(&self, path: &str, body: &Value) -> Result<Value> {
        for attempt in 0..2 {
            let token = self.access_token().await?;
            let url = format!("{API_BASE}{path}?access_token={}", urlencode(&token));
            let resp = self
                .http
                .post(&url)
                .json(body)
                .timeout(std::time::Duration::from_secs(15))
                .send()
                .await
                .map_err(|e| Error::Internal(format!("微信小程序接口请求失败: {e}")))?;
            let status = resp.status();
            let raw = resp
                .text()
                .await
                .map_err(|e| Error::Internal(format!("微信小程序响应读取失败: {e}")))?;
            if !status.is_success() {
                return Err(Error::Internal(format!(
                    "微信小程序接口 HTTP {status}: {raw}"
                )));
            }
            let errcode = serde_json::from_str::<Value>(&raw)
                .ok()
                .and_then(|v| v.get("errcode").and_then(Value::as_i64));
            if attempt == 0 && matches!(errcode, Some(40001) | Some(42001)) {
                self.invalidate_token().await;
                continue;
            }
            return parse_wechat_response(&raw);
        }
        unreachable!("token 重试循环必在两次尝试内返回")
    }

    /// 小程序码（不限制码数）：`POST /wxa/getwxacodeunlimit`，返回 PNG 字节。
    ///
    /// `page` 不得携带参数（参数放 `scene`，微信硬性规定）；`scene` 最长 32
    /// 字符，规划口径见 [`plan_scene`]。
    ///
    /// # Errors
    /// 网络错误 / 微信 errcode（响应为 JSON 时按 errcode/errmsg 转业务错误）。
    pub async fn wxacode_unlimit(
        &self,
        scene: &str,
        page: &str,
        env_version: &str,
    ) -> Result<Vec<u8>> {
        let body = serde_json::json!({
            "scene": scene,
            "page": page,
            "check_path": false,
            "env_version": env_version,
            "width": 430,
        });
        for attempt in 0..2 {
            let token = self.access_token().await?;
            let url = format!(
                "{API_BASE}/wxa/getwxacodeunlimit?access_token={}",
                urlencode(&token)
            );
            let resp = self
                .http
                .post(&url)
                .json(&body)
                .timeout(std::time::Duration::from_secs(15))
                .send()
                .await
                .map_err(|e| Error::Internal(format!("微信小程序码请求失败: {e}")))?;
            let status = resp.status();
            let bytes = resp
                .bytes()
                .await
                .map_err(|e| Error::Internal(format!("微信小程序码响应读取失败: {e}")))?;
            if !status.is_success() {
                return Err(Error::Internal(format!(
                    "微信小程序码接口 HTTP {status}: {}",
                    String::from_utf8_lossy(&bytes)
                )));
            }
            // 成功响应是 PNG 二进制；失败响应是 JSON（以 '{' 开头）。
            if bytes.first() == Some(&b'{') {
                let raw = String::from_utf8_lossy(&bytes).into_owned();
                let errcode = serde_json::from_str::<Value>(&raw)
                    .ok()
                    .and_then(|v| v.get("errcode").and_then(Value::as_i64));
                if attempt == 0 && matches!(errcode, Some(40001) | Some(42001)) {
                    self.invalidate_token().await;
                    continue;
                }
                // parse_wechat_response 对 errcode != 0 返回业务错误；errcode == 0
                // 的 JSON 出现在码接口即响应形态异常，同样按错误处理。
                parse_wechat_response(&raw)?;
                return Err(Error::Internal(format!(
                    "微信小程序码响应形态异常（期望 PNG，得到 JSON）: {raw}"
                )));
            }
            if bytes.is_empty() {
                return Err(Error::Internal("微信小程序码响应为空".into()));
            }
            return Ok(bytes.to_vec());
        }
        unreachable!("token 重试循环必在两次尝试内返回")
    }

    /// URL Link：`POST /wxa/generate_urllink`，返回 `https://wxaurl.cn/...` 短链。
    ///
    /// 生成结果按（appId, path, query, env_version, 有效期）缓存：URL Link
    /// 有配额且有效期内稳定，重复生成同一参数直接命中缓存。
    ///
    /// # Errors
    /// 网络错误 / 微信 errcode / 响应缺少 `url_link`。
    pub async fn generate_url_link(
        &self,
        path: &str,
        query: &str,
        env_version: &str,
        expire_days: u32,
    ) -> Result<String> {
        let days = expire_days.clamp(1, URL_LINK_EXPIRE_DAYS);
        let cache_key = url_link_cache_key(&self.cfg.app_id, path, query, env_version, days);
        if let Ok(Some(link)) = get_cache().get_str(&cache_key).await {
            if !link.is_empty() {
                return Ok(link);
            }
        }
        let body = serde_json::json!({
            "path": path,
            "query": query,
            "is_expire": true,
            "expire_type": 1,
            "expire_interval": days,
            "env_version": env_version,
        });
        let value = self
            .post_json_with_token_retry("/wxa/generate_urllink", &body)
            .await?;
        let link = value
            .get("url_link")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Internal(format!("微信 URL Link 响应缺少 url_link: {value}")))?
            .to_string();
        let ttl = (u64::from(days) * 86_400)
            .saturating_sub(URL_LINK_CACHE_MARGIN_SECS)
            .max(60);
        let _ = get_cache().set_str(&cache_key, &link, ttl).await;
        Ok(link)
    }
}

fn url_link_cache_key(app_id: &str, path: &str, query: &str, env_version: &str, days: u32) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (app_id, path, query, env_version, days).hash(&mut hasher);
    format!("wechat:mini:urllink:{app_id}:{:016x}", hasher.finish())
}

/// 分享 scene 参数（与既有前端解析口径一致：`id` / `salesmanId` / `shopId`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneParams {
    pub id: i32,
    pub salesman_id: i32,
    pub shop_id: i32,
}

/// 全量 scene 串（逐字沿用既有口径 `id={}&salesmanId={}&shopId={}`）。
#[must_use]
pub fn full_scene(params: &SceneParams) -> String {
    format!(
        "id={}&salesmanId={}&shopId={}",
        params.id, params.salesman_id, params.shop_id
    )
}

/// 组装 scene 参数对：页面内嵌 query 参数在前（页面关键参数，如拼团
/// `grouponPromotionRecordId`，`page` 本身不得带参、只能靠 scene 承载），
/// 归因参数（id/salesmanId/shopId）在后；内嵌已含同名键时归因不重复添加。
#[must_use]
pub fn scene_pairs(embedded_query: Option<&str>, params: &SceneParams) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    if let Some(query) = embedded_query {
        for pair in query.split('&').filter(|s| !s.is_empty()) {
            if let Some((key, value)) = pair.split_once('=') {
                pairs.push((key.to_string(), value.to_string()));
            }
        }
    }
    let attribution = [
        ("id", params.id.to_string()),
        ("salesmanId", params.salesman_id.to_string()),
        ("shopId", params.shop_id.to_string()),
    ];
    for (key, value) in attribution {
        if !pairs.iter().any(|(k, _)| k == key) {
            pairs.push((key.to_string(), value));
        }
    }
    pairs
}

/// 参数对 → `k=v&k=v` 串（同时用作 URL Link / H5 的落地 query）。
#[must_use]
pub fn join_scene_pairs(pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&")
}

/// scene 规划结果：直用全量串，或改用短码（全量参数另存缓存映射）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScenePlan {
    /// 全量串 ≤ 32 字符，直接作 scene（落地页本地解析，无需回查）。
    Direct(String),
    /// 全量串超 32 字符：scene 用短码，参数 JSON 存
    /// `RUNNING:MINISCENE:CODE:{短码}`，落地页经 `miniScene` 接口回查。
    ShortCode(String),
}

/// 规划 scene：全量串不超 [`SCENE_MAX_LEN`] 直用，否则短码。
#[must_use]
pub fn plan_scene(full: &str) -> ScenePlan {
    if full.chars().count() <= SCENE_MAX_LEN {
        ScenePlan::Direct(full.to_string())
    } else {
        ScenePlan::ShortCode(short_code_for_str(full))
    }
}

/// 全量串 → 确定性短码（同串同码，幂等；11 位 base62）。
#[must_use]
pub fn short_code_for_str(full: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    full.hash(&mut hasher);
    base62(hasher.finish())
}

/// 参数 → 确定性短码（无内嵌参数时的便捷形式）。
#[must_use]
pub fn short_code_for(params: &SceneParams) -> String {
    short_code_for_str(&full_scene(params))
}

fn base62(mut value: u64) -> String {
    const CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mut buf = [b'0'; 11];
    for slot in buf.iter_mut().rev() {
        *slot = CHARS[usize::try_from(value % 62).unwrap_or(0)];
        value /= 62;
    }
    String::from_utf8_lossy(&buf).into_owned()
}

/// 短码映射的缓存键（与 `system::util::mini_scene` 既有读取键一致）。
#[must_use]
pub fn scene_cache_key(code: &str) -> String {
    format!("{SCENE_CACHE_PREFIX}{code}")
}

/// 短码映射的参数 JSON（与前端 `resolveMiniScene` 消费口径一致）。
#[must_use]
pub fn scene_payload(params: &SceneParams) -> Value {
    serde_json::json!({
        "id": params.id,
        "salesmanId": params.salesman_id,
        "shopId": params.shop_id,
    })
}

/// 参数对 → 短码映射 JSON：纯数字值转 JSON 数字（落地页按数字消费
/// id/grouponPromotionRecordId 等），其余保留字符串。
#[must_use]
pub fn scene_payload_from_pairs(pairs: &[(String, String)]) -> Value {
    let mut map = serde_json::Map::new();
    for (key, raw) in pairs {
        let value = match raw.parse::<i64>() {
            Ok(num) => Value::from(num),
            Err(_) => Value::String(raw.clone()),
        };
        map.insert(key.clone(), value);
    }
    Value::Object(map)
}

/// 解析 query 形式的 scene（`id=1&salesmanId=2&shopId=3`，兼容别名 `s`）；
/// 非 query 形式（短码）返回 `None`。
#[must_use]
pub fn parse_scene_query(scene: &str) -> Option<Value> {
    if !scene.contains('=') {
        return None;
    }
    let (mut id, mut salesman_id, mut shop_id) = (None, None, None);
    for pair in scene.split('&') {
        let Some((key, raw)) = pair.split_once('=') else {
            continue;
        };
        let Ok(num) = raw.parse::<i32>() else {
            continue;
        };
        match key {
            "id" => id = Some(num),
            "salesmanId" | "s" => salesman_id = Some(num),
            "shopId" => shop_id = Some(num),
            _ => {}
        }
    }
    id.map(|id| {
        serde_json::json!({
            "id": id,
            "salesmanId": salesman_id.unwrap_or(0),
            "shopId": shop_id.unwrap_or(0),
        })
    })
}

/// 由小程序页面路径 + query 拼 H5 分享链接（降级回退目标）。
///
/// 口径与短链服务一致：`h5Domain` 为 H5 完整根（如
/// `https://example.com/mobile`），长路径先剥 `/mobile` 前缀再拼；
/// `h5Domain` 未配置时返回相对路径 `/mobile/...`（调用方须在降级标注中
/// 一并提示域名未配置）。
#[must_use]
pub fn build_h5_url(h5_domain: &str, page: &str, query: &str) -> String {
    let page = page.trim().trim_start_matches('/');
    let mut long_path = format!("/mobile/{page}");
    if !query.is_empty() {
        long_path.push('?');
        long_path.push_str(query);
    }
    let domain = h5_domain.trim().trim_end_matches('/');
    if domain.is_empty() {
        long_path
    } else {
        format!("{}{}", domain, long_path.replace("/mobile", ""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_missing_lists_both_codes() {
        let cfg = WechatMiniConfig::default();
        assert!(!cfg.is_configured());
        assert_eq!(cfg.missing(), vec![MINI_APP_ID_CODE, MINI_APP_SECRET_CODE]);
        let cfg = WechatMiniConfig {
            app_id: "wx123".into(),
            app_secret: "s".into(),
        };
        assert!(cfg.is_configured());
        assert!(cfg.missing().is_empty());
    }

    #[test]
    fn scene_direct_when_within_limit() {
        let params = SceneParams {
            id: 7,
            salesman_id: 0,
            shop_id: 0,
        };
        assert_eq!(full_scene(&params), "id=7&salesmanId=0&shopId=0");
        assert_eq!(
            plan_scene(&full_scene(&params)),
            ScenePlan::Direct("id=7&salesmanId=0&shopId=0".into())
        );
    }

    #[test]
    fn scene_short_code_when_over_limit() {
        let params = SceneParams {
            id: 2_147_483_647,
            salesman_id: 2_147_483_647,
            shop_id: 2_147_483_647,
        };
        assert!(full_scene(&params).chars().count() > SCENE_MAX_LEN);
        let ScenePlan::ShortCode(code) = plan_scene(&full_scene(&params)) else {
            panic!("超长 scene 应走短码");
        };
        assert_eq!(code.len(), 11);
        assert!(code.chars().all(|c| c.is_ascii_alphanumeric()));
        // 确定性：同参数同码
        assert_eq!(code, short_code_for(&params));
        // 不同参数不同码
        let other = SceneParams {
            id: 1,
            ..params
        };
        assert_ne!(code, short_code_for(&other));
    }

    #[test]
    fn scene_pairs_carry_embedded_page_params_first() {
        let params = SceneParams {
            id: 5,
            salesman_id: 6,
            shop_id: 7,
        };
        let pairs = scene_pairs(Some("grouponPromotionRecordId=9"), &params);
        assert_eq!(
            join_scene_pairs(&pairs),
            "grouponPromotionRecordId=9&id=5&salesmanId=6&shopId=7"
        );
        // 内嵌已含同名键时不重复添加归因键
        let pairs = scene_pairs(Some("id=42"), &params);
        assert_eq!(join_scene_pairs(&pairs), "id=42&salesmanId=6&shopId=7");
        // 无内嵌时即全量 scene 串
        let pairs = scene_pairs(None, &params);
        assert_eq!(join_scene_pairs(&pairs), full_scene(&params));
        // 拼团短码映射须含页面关键参数（数字转 JSON 数字）
        let payload = scene_payload_from_pairs(&scene_pairs(
            Some("grouponPromotionRecordId=9"),
            &params,
        ));
        assert_eq!(payload["grouponPromotionRecordId"], 9);
        assert_eq!(payload["id"], 5);
    }

    #[test]
    fn scene_payload_and_cache_key_match_legacy_reader() {
        let params = SceneParams {
            id: 3,
            salesman_id: 4,
            shop_id: 5,
        };
        let payload = scene_payload(&params);
        assert_eq!(payload["id"], 3);
        assert_eq!(payload["salesmanId"], 4);
        assert_eq!(payload["shopId"], 5);
        assert_eq!(scene_cache_key("abc"), "RUNNING:MINISCENE:CODE:abc");
    }

    #[test]
    fn parse_scene_query_forms() {
        let v = parse_scene_query("id=9&salesmanId=8&shopId=7").unwrap();
        assert_eq!(v["id"], 9);
        assert_eq!(v["salesmanId"], 8);
        assert_eq!(v["shopId"], 7);
        // 别名 s
        let v = parse_scene_query("id=9&s=8").unwrap();
        assert_eq!(v["salesmanId"], 8);
        assert_eq!(v["shopId"], 0);
        // 短码（无 =）不解析
        assert!(parse_scene_query("a1B2c3D4e5F").is_none());
        // 无 id 不成形
        assert!(parse_scene_query("salesmanId=8").is_none());
    }

    #[test]
    fn h5_url_building_matches_short_url_caliber() {
        assert_eq!(
            build_h5_url("https://m.example.com/mobile", "pages/product/index", "id=1"),
            "https://m.example.com/mobile/pages/product/index?id=1"
        );
        assert_eq!(
            build_h5_url("https://m.example.com/mobile/", "/pages/product/index", ""),
            "https://m.example.com/mobile/pages/product/index"
        );
        // 域名未配置 → 相对路径（仍可由前端标注降级）
        assert_eq!(
            build_h5_url("", "pages/product/index", "id=1"),
            "/mobile/pages/product/index?id=1"
        );
    }

    #[test]
    fn url_link_cache_key_is_stable_and_scoped() {
        let a = url_link_cache_key("wx1", "pages/product/index", "id=1", "release", 30);
        let b = url_link_cache_key("wx1", "pages/product/index", "id=1", "release", 30);
        let c = url_link_cache_key("wx1", "pages/product/index", "id=2", "release", 30);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.starts_with("wechat:mini:urllink:wx1:"));
    }
}
