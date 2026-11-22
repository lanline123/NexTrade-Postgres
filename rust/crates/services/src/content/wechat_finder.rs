//! 微信视频号客户端（视频号直播记录，content::wechat_finder）。
//!
//! ## 凭据口径（与公众号 / 平台小程序两套凭据严格区分）
//!
//! 本模块调用微信「视频号助手」开放接口
//! `POST /channels/ec/finderlive/getfinderliverecordlist`（微信官方文档
//! 「视频号获取当前的直播记录」：无请求体，返回当前直播列表
//! `live_list[{export_id, description, cover_url, nickname, head_url}]`）。
//! 原版 Java 的调用链为 `WechatLiveServiceImpl#getFinderLiveRecord` →
//! `WechatOAuthServiceImpl#getLiveMiniAccessToken`，用的是**直播小程序**
//! 凭据 `wechatLiveMiniAppId` / `wechatLiveMiniAppSecret`（`config` 表，
//! 后台「设置 → 微信直播设置」维护，与小程序直播间的 refresh/sync 等
//! 接口同源）——既不是公众号凭据（`wechatAppId`/`wechatAppSecret`，
//! [`super::wechat_client`]），也不是平台小程序凭据
//! （`wechatMiniProgramAppId`/`wechatMiniProgramSecret`，
//! [`super::wechat_mini`]，wxa/* 专用）。本模块按原版口径使用直播
//! 小程序凭据，不挪用另两套；用户须在后台填好这对凭据本功能才亮起。
//!
//! ## access_token 基础设施（复用分享直达小程序线 @7c33da5 的底座口径）
//!
//! 与 [`super::wechat_mini`] 同构：稳定版接口 `POST /cgi-bin/stable_token`
//! （`force_refresh=false`，请求体复用
//! [`super::wechat_client::stable_token_request_body`]）；全局缓存服务
//! （Redis 优先、多实例共享，不可用时自动降级进程内内存），键
//! `wechat:live:access_token:{appId}`（与 Java 为直播小程序单设的缓存键
//! TOKEN_KEY_LIVE_MINI 同口径，与平台小程序键 `wechat:mini:*` 隔离）；
//! 提前 [`TOKEN_REFRESH_MARGIN_SECS`] 秒视为过期；按 appId 分片的单飞
//! （single-flight）串行刷新；下游返回 40001/42001 时清缓存重取一次。
//!
//! ## 降级铁律
//!
//! 未配置直播小程序凭据时 [`WechatFinderClient::from_repos`] 返回明确
//! 业务错误（列明缺失配置位与填写位置）；微信侧 errcode（如 48001
//! 视频号助手开放能力未开启）原样保留在业务错误中。绝不伪造直播记录。
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use nextrade_common::utils::now_seconds;
use nextrade_core::models::content::api_wechat_live::FinderLiveRecordVO;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::cache::get_cache;
use nextrade_infrastructure::db::Repositories;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::wechat_client::{parse_wechat_response, stable_token_request_body, urlencode};

/// 直播小程序 AppID 配置位（`config` 表，后台「设置 → 微信直播设置」）。
pub const LIVE_MINI_APP_ID_CODE: &str = "wechatLiveMiniAppId";
/// 直播小程序 AppSecret 配置位（`config` 表，后台「设置 → 微信直播设置」）。
pub const LIVE_MINI_APP_SECRET_CODE: &str = "wechatLiveMiniAppSecret";

const API_BASE: &str = "https://api.weixin.qq.com";
/// 视频号获取当前的直播记录（视频号助手开放接口，官方路径）。
const FINDER_LIVE_RECORD_PATH: &str = "/channels/ec/finderlive/getfinderliverecordlist";
/// 业务错误码（与 config_basic 的 SERVICE_DATA_ERROR 口径一致）。
const SERVICE_DATA_ERROR: i32 = 1001;
/// token 提前过期边距（秒）：剩余有效期低于此值即刷新。
const TOKEN_REFRESH_MARGIN_SECS: i64 = 300;

/// 直播小程序配置（`config` 表 `wechatLiveMiniAppId`/`wechatLiveMiniAppSecret`）。
#[derive(Debug, Clone, Default)]
pub struct WechatFinderConfig {
    pub app_id: String,
    pub app_secret: String,
}

impl WechatFinderConfig {
    /// 是否具备调用视频号助手接口的最小配置（appId + secret）。
    #[must_use]
    pub fn is_configured(&self) -> bool {
        !self.app_id.trim().is_empty() && !self.app_secret.trim().is_empty()
    }

    /// 缺失的配置位（用于降级标注文案）。
    #[must_use]
    pub fn missing(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.app_id.trim().is_empty() {
            missing.push(LIVE_MINI_APP_ID_CODE);
        }
        if self.app_secret.trim().is_empty() {
            missing.push(LIVE_MINI_APP_SECRET_CODE);
        }
        missing
    }
}

/// 从 `config` 表读取直播小程序配置（未配置 → 全空的 `Default`，不报错）。
///
/// # Errors
/// 数据库读取失败。
pub async fn load_finder_config(repos: &Repositories) -> Result<WechatFinderConfig> {
    let map: HashMap<String, String> = repos
        .system
        .config_basic
        .find_by_codes(&[LIVE_MINI_APP_ID_CODE, LIVE_MINI_APP_SECRET_CODE])
        .await?
        .into_iter()
        .collect();
    Ok(WechatFinderConfig {
        app_id: map.get(LIVE_MINI_APP_ID_CODE).cloned().unwrap_or_default(),
        app_secret: map
            .get(LIVE_MINI_APP_SECRET_CODE)
            .cloned()
            .unwrap_or_default(),
    })
}

/// 缓存中的 token 记录（绝对过期时间，秒）。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedToken {
    token: String,
    expires_at: i64,
}

fn token_cache_key(app_id: &str) -> String {
    format!("wechat:live:access_token:{app_id}")
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

/// 微信视频号客户端（直播小程序凭据）。
pub struct WechatFinderClient {
    cfg: WechatFinderConfig,
    http: reqwest::Client,
}

impl WechatFinderClient {
    #[must_use]
    pub fn new(cfg: WechatFinderConfig) -> Self {
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
        let cfg = load_finder_config(repos).await?;
        if !cfg.is_configured() {
            return Err(Error::biz(
                SERVICE_DATA_ERROR,
                format!(
                    "微信视频号直播未配置：缺少 {}（后台「设置 → 微信直播设置」填写直播小程序 AppID/Secret）",
                    cfg.missing().join("/")
                ),
            ));
        }
        Ok(Self::new(cfg))
    }

    #[must_use]
    pub fn config(&self) -> &WechatFinderConfig {
        &self.cfg
    }

    /// 读缓存中仍有效的 token（剩余有效期 > 刷新边距）；缓存异常按未命中处理。
    async fn read_cached_token(&self) -> Option<String> {
        let cached = get_cache()
            .get_json::<CachedToken>(&token_cache_key(&self.cfg.app_id))
            .await
            .ok()??;
        if cached.expires_at - now_seconds() > TOKEN_REFRESH_MARGIN_SECS {
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
            .map_err(|e| Error::Internal(format!("微信视频号 token 请求失败: {e}")))?;
        let status = resp.status();
        let raw = resp
            .text()
            .await
            .map_err(|e| Error::Internal(format!("微信视频号 token 响应读取失败: {e}")))?;
        if !status.is_success() {
            return Err(Error::Internal(format!(
                "微信视频号 token 接口 HTTP {status}: {raw}"
            )));
        }
        let value = parse_wechat_response(&raw)?;
        let token = value
            .get("access_token")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                Error::Internal(format!("微信视频号 token 响应缺少 access_token: {raw}"))
            })?
            .to_string();
        let expires_in = value
            .get("expires_in")
            .and_then(Value::as_i64)
            .unwrap_or(7_200);
        let record = CachedToken {
            token: token.clone(),
            expires_at: now_seconds() + expires_in,
        };
        // TTL 取 expires_in 本身（值内含绝对过期时间，读端再扣边距判定）。
        let ttl = u64::try_from(expires_in).unwrap_or(7_200).max(60);
        let _ = get_cache()
            .set_json(&token_cache_key(&self.cfg.app_id), &record, ttl)
            .await;
        Ok(token)
    }

    /// 清除 token 缓存（本模块缓存 + 进程内共用缓存一并清，稳定版 token 同源）。
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
                .map_err(|e| Error::Internal(format!("微信视频号接口请求失败: {e}")))?;
            let status = resp.status();
            let raw = resp
                .text()
                .await
                .map_err(|e| Error::Internal(format!("微信视频号响应读取失败: {e}")))?;
            if !status.is_success() {
                return Err(Error::Internal(format!(
                    "微信视频号接口 HTTP {status}: {raw}"
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

    /// 获取当前的直播记录列表（`POST /channels/ec/finderlive/getfinderliverecordlist`，
    /// 无请求体）。返回当前正在直播的记录（可为空 = 当前未在直播）。
    ///
    /// # Errors
    /// 网络错误 / 微信 errcode（如 48001 开放能力未开启，原文保留在
    /// 业务错误中，并冠以 Java 同口径前缀「获取视频号直播记录失败：」）。
    pub async fn finder_live_record_list(&self) -> Result<Vec<FinderLiveRecordVO>> {
        let value = self
            .post_json_with_token_retry(FINDER_LIVE_RECORD_PATH, &serde_json::json!({}))
            .await
            .map_err(wrap_finder_error)?;
        Ok(parse_finder_live_list(&value))
    }
}

/// 微信业务错误冠以 Java 同口径前缀（`获取视频号直播记录失败：` +
/// 原错误信息，errcode/errmsg 原文保留）；内部错误原样透传。
fn wrap_finder_error(e: Error) -> Error {
    match e {
        Error::Biz { code, msg } => Error::biz(code, format!("获取视频号直播记录失败：{msg}")),
        other => other,
    }
}

/// 解析微信 `getfinderliverecordlist` 响应体（纯函数）：
/// 取 `live_list` 数组逐条映射 VO；缺 `live_list` → 空列表；
/// `export_id` 为空的条目跳过（无稳定键、不可选取/落库）。
#[must_use]
pub fn parse_finder_live_list(value: &Value) -> Vec<FinderLiveRecordVO> {
    let Some(list) = value.get("live_list").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|item| {
            let export_id = item
                .get("export_id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())?
                .to_string();
            let s = |k: &str| {
                item.get(k)
                    .and_then(Value::as_str)
                    .filter(|v| !v.is_empty())
                    .map(str::to_string)
            };
            Some(FinderLiveRecordVO {
                export_id: Some(export_id),
                description: s("description"),
                cover_url: s("cover_url"),
                nickname: s("nickname"),
                head_url: s("head_url"),
            })
        })
        .collect()
}

/// 拉取当前直播记录并落库（前台 `api_wechat_live` 与管理端 `wechat_live`
/// 共用）：未配置凭据 → 明确业务错误；微信 errcode → 业务错误；成功
/// （含空列表）→ 整批落库刷新当前标记后返回本次列表。
///
/// # Errors
/// 未配置 / 网络 / 微信 errcode / 数据库错误。
pub async fn fetch_and_persist_live_records(
    repos: &Repositories,
) -> Result<Vec<FinderLiveRecordVO>> {
    let client = WechatFinderClient::from_repos(repos).await?;
    let records = client.finder_live_record_list().await?;
    let now = now_seconds() as i32;
    repos
        .content
        .api_wechat_live
        .replace_current(&records, now)
        .await?;
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_requires_live_mini_app_id_and_secret() {
        let empty = WechatFinderConfig::default();
        assert!(!empty.is_configured());
        assert_eq!(
            empty.missing(),
            vec!["wechatLiveMiniAppId", "wechatLiveMiniAppSecret"]
        );
        let cfg = WechatFinderConfig {
            app_id: "wx-live".into(),
            app_secret: "secret".into(),
        };
        assert!(cfg.is_configured());
        assert!(cfg.missing().is_empty());
    }

    #[test]
    fn token_cache_key_is_live_namespace_not_mini() {
        // 与平台小程序键隔离（wechat:mini:*），同 Java 独立缓存键口径
        assert_eq!(
            token_cache_key("wx-live"),
            "wechat:live:access_token:wx-live"
        );
        assert!(!token_cache_key("wx-live").starts_with("wechat:mini:"));
    }

    #[test]
    fn parse_live_list_maps_official_fields() {
        let value = serde_json::json!({
            "errcode": 0,
            "errmsg": "ok",
            "live_list": [
                {
                    "export_id": "exp-1",
                    "description": "描述一",
                    "cover_url": "https://cover/1",
                    "nickname": "昵称一",
                    "head_url": "https://head/1"
                },
                { "export_id": "exp-2", "nickname": "昵称二" },
                { "description": "无 id 条目应跳过" }
            ]
        });
        let records = parse_finder_live_list(&value);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].export_id.as_deref(), Some("exp-1"));
        assert_eq!(records[0].description.as_deref(), Some("描述一"));
        assert_eq!(records[0].cover_url.as_deref(), Some("https://cover/1"));
        assert_eq!(records[0].nickname.as_deref(), Some("昵称一"));
        assert_eq!(records[0].head_url.as_deref(), Some("https://head/1"));
        assert_eq!(records[1].nickname.as_deref(), Some("昵称二"));
        assert!(records[1].description.is_none());
    }

    #[test]
    fn parse_live_list_empty_when_missing_or_empty() {
        assert!(parse_finder_live_list(&serde_json::json!({"errcode": 0})).is_empty());
        assert!(parse_finder_live_list(&serde_json::json!({"live_list": []})).is_empty());
    }

    #[test]
    fn wrap_finder_error_keeps_errcode_text_with_java_prefix() {
        let e = wrap_finder_error(Error::biz(
            1001,
            "微信接口错误(48001): api unauthorized",
        ));
        match e {
            Error::Biz { code, msg } => {
                assert_eq!(code, 1001);
                assert!(msg.starts_with("获取视频号直播记录失败："));
                assert!(msg.contains("48001"));
            }
            other => panic!("期望业务错误，得到 {other:?}"),
        }
    }

    #[test]
    fn finder_path_matches_official_doc() {
        assert_eq!(
            FINDER_LIVE_RECORD_PATH,
            "/channels/ec/finderlive/getfinderliverecordlist"
        );
    }
}
