//! 微信开放平台「第三方平台」客户端（店铺独立绑定小程序模式②，我方新增）。
//!
//! 模式②（设计稿 `~/workspace/shop-miniprogram-binding/DESIGN.md` 二期）：商家
//! 不再自填 AppSecret，而是把自有小程序**授权**给平台在微信开放平台创建的
//! 第三方平台，平台凭授权关系代调用微信接口（登录、取手机号，乃至后续的
//! 代提审/代发布）。本模块只负责与微信开放平台的协议层：
//!
//! - 配置位（`config` 表，后台「店铺小程序 → 第三方平台设置」维护）：
//!   `wechatComponentAppId` / `wechatComponentAppSecret` /
//!   `wechatComponentToken`（授权事件消息校验 Token）/
//!   `wechatComponentAesKey`（消息加解密 EncodingAESKey，43 位）。
//!   **未配置即未启用**：所有入口返回明确业务错误，绝不假成功。
//! - `component_verify_ticket`：微信每 10 分钟向授权事件接收 URL 推送，
//!   由公开回调端点写入全局缓存（Redis 优先），本模块读取换 token；
//! - `component_access_token`：`POST /cgi-bin/component/api_component_token`，
//!   缓存 + 单飞刷新（口径同 [`super::wechat_mini`] 的小程序 token 基建）；
//! - 预授权码：`POST /cgi-bin/component/api_create_preauthcode`
//!   （`pre_auth_code` 有效期 600 秒，官方口径）；
//! - 授权页链接（官方《授权流程技术说明》拼接规则）：PC 版
//!   `mp.weixin.qq.com/cgi-bin/componentloginpage`，H5 版
//!   `open.weixin.qq.com/wxaopen/safe/bindcomponent`；`auth_type=2`
//!   表示仅展示小程序（本平台只代管小程序，不捆绑公众号授权）；
//! - 授权码换授权信息：`POST /cgi-bin/component/api_query_auth` →
//!   `authorizer_appid` + `authorizer_access_token` +
//!   **`authorizer_refresh_token`（高价值凭据，落库必须加密，丢失只能
//!   让商家重新授权）** + 已授权权限集 `func_info`；
//! - 授权方 token 刷新：`POST /cgi-bin/component/api_authorizer_token`；
//! - 授权方基本信息：`POST /cgi-bin/component/api_get_authorizer_info`；
//! - 代登录：`GET /sns/component/jscode2session`（以 component 令牌为
//!   授权方小程序换 openid，与直连版 `jscode2session` 分工）。
//!
//! 授权关系的落库/解析/事件应用在
//! [`crate::merchant::shop_mini_program_auth`]；本模块不碰业务表。

use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::cache::get_cache;
use nextrade_infrastructure::db::Repositories;

use super::wechat_client::{parse_wechat_response, urlencode};

/// 第三方平台 AppID 配置位（`config` 表）。
pub const COMPONENT_APP_ID_CODE: &str = "wechatComponentAppId";
/// 第三方平台 AppSecret 配置位（`config` 表）。
pub const COMPONENT_APP_SECRET_CODE: &str = "wechatComponentAppSecret";
/// 授权事件消息校验 Token 配置位（`config` 表）。
pub const COMPONENT_TOKEN_CODE: &str = "wechatComponentToken";
/// 消息加解密 EncodingAESKey 配置位（`config` 表）。
pub const COMPONENT_AES_KEY_CODE: &str = "wechatComponentAesKey";
/// 授权回调域名配置位（`config` 表；为空时回落 `h5Domain`）。
pub const COMPONENT_REDIRECT_BASE_CODE: &str = "wechatComponentRedirectBase";
/// H5 域名配置位（回调域名回落源，与小程序链同口径）。
pub const H5_DOMAIN_CODE: &str = "h5Domain";

/// 授权事件接收路径（挂在商城前台 `/api` 下，须原样填入开放平台
/// 「授权事件接收 URL」；域名须与开放平台配置的授权回调域名一致）。
pub const EVENT_PATH: &str = "/api/content/wechatComponent/event";
/// 授权完成回调路径（授权页 redirect_uri 的落点，同上域名口径）。
pub const AUTH_CALLBACK_PATH: &str = "/api/content/wechatComponent/authCallback";

const API_BASE: &str = "https://api.weixin.qq.com";
/// 业务错误码（与同域服务口径一致）。
const SERVICE_DATA_ERROR: i32 = 1001;
/// component_verify_ticket 缓存键。
const TICKET_CACHE_KEY: &str = "wechat:component:verify_ticket";
/// component_access_token 缓存键。
const COMPONENT_TOKEN_CACHE_KEY: &str = "wechat:component:access_token";
/// ticket 缓存时长（微信每 10 分钟推送一次，留足冗余）。
const TICKET_TTL_SECS: u64 = 3_600;
/// token 提前过期边距（秒）。
const TOKEN_REFRESH_MARGIN_SECS: i64 = 300;
/// 授权类型：仅小程序（官方 auth_type 口径：1 公众号 / 2 小程序 / 3 两者）。
pub const AUTH_TYPE_MINI_PROGRAM: u8 = 2;

/// 第三方平台配置（`config` 表五位；凭据只入不出接口，管理端只回掩码）。
#[derive(Debug, Clone, Default)]
pub struct ComponentConfig {
    pub app_id: String,
    pub app_secret: String,
    pub token: String,
    pub aes_key: String,
    pub redirect_base: String,
}

impl ComponentConfig {
    /// 是否已配置到可换取 component 令牌（appid + secret）。
    #[must_use]
    pub fn has_token_credentials(&self) -> bool {
        !self.app_id.trim().is_empty() && !self.app_secret.trim().is_empty()
    }

    /// 是否已配置到可接收授权事件（appid + token + aeskey）。
    #[must_use]
    pub fn has_event_credentials(&self) -> bool {
        !self.app_id.trim().is_empty()
            && !self.token.trim().is_empty()
            && !self.aes_key.trim().is_empty()
    }

    /// 整体是否已启用（四项凭据齐全）。
    #[must_use]
    pub fn is_configured(&self) -> bool {
        self.has_token_credentials() && self.has_event_credentials()
    }

    /// 缺失的配置位（用于后台未启用提示）。
    #[must_use]
    pub fn missing(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.app_id.trim().is_empty() {
            missing.push(COMPONENT_APP_ID_CODE);
        }
        if self.app_secret.trim().is_empty() {
            missing.push(COMPONENT_APP_SECRET_CODE);
        }
        if self.token.trim().is_empty() {
            missing.push(COMPONENT_TOKEN_CODE);
        }
        if self.aes_key.trim().is_empty() {
            missing.push(COMPONENT_AES_KEY_CODE);
        }
        missing
    }
}

/// 从 `config` 表读取第三方平台配置（未配置 → 全空 Default，不报错）。
///
/// # Errors
/// 数据库读取失败。
pub async fn load_component_config(repos: &Repositories) -> Result<ComponentConfig> {
    let codes = [
        COMPONENT_APP_ID_CODE,
        COMPONENT_APP_SECRET_CODE,
        COMPONENT_TOKEN_CODE,
        COMPONENT_AES_KEY_CODE,
        COMPONENT_REDIRECT_BASE_CODE,
        H5_DOMAIN_CODE,
    ];
    let map: std::collections::HashMap<String, String> = repos
        .system
        .config_basic
        .find_by_codes(&codes)
        .await?
        .into_iter()
        .collect();
    let get = |key: &str| map.get(key).cloned().unwrap_or_default().trim().to_string();
    let mut redirect_base = get(COMPONENT_REDIRECT_BASE_CODE);
    if redirect_base.is_empty() {
        redirect_base = get(H5_DOMAIN_CODE);
    }
    Ok(ComponentConfig {
        app_id: get(COMPONENT_APP_ID_CODE),
        app_secret: get(COMPONENT_APP_SECRET_CODE),
        token: get(COMPONENT_TOKEN_CODE),
        aes_key: get(COMPONENT_AES_KEY_CODE),
        redirect_base: redirect_base.trim_end_matches('/').to_string(),
    })
}

fn biz(msg: impl Into<String>) -> Error {
    Error::biz(SERVICE_DATA_ERROR, msg.into())
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// 缓存中的 token 记录（绝对过期时间，秒；与 wechat_mini 同形态）。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedToken {
    token: String,
    expires_at: i64,
}

async fn cached_token(key: &str) -> Option<String> {
    let cached: Option<CachedToken> = get_cache().get_json(key).await.ok().flatten()?;
    if cached.expires_at - TOKEN_REFRESH_MARGIN_SECS <= now_secs() {
        return None;
    }
    Some(cached.token)
}

async fn put_cached_token(key: &str, token: &str, expires_in: u64) {
    let record = CachedToken {
        token: token.to_string(),
        expires_at: now_secs() + i64::try_from(expires_in).unwrap_or(7_200),
    };
    // 缓存写失败不阻断主链（下次调用重新换取即可）。
    let _ = get_cache()
        .set_json(key, &record, expires_in.max(60))
        .await;
}

async fn post_json(url: &str, body: &Value) -> Result<Value> {
    let resp = reqwest::Client::new()
        .post(url)
        .json(body)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| Error::Internal(format!("微信开放平台接口请求失败: {e}")))?;
    let status = resp.status();
    let raw = resp
        .text()
        .await
        .map_err(|e| Error::Internal(format!("微信开放平台响应读取失败: {e}")))?;
    if !status.is_success() {
        return Err(Error::Internal(format!(
            "微信开放平台接口 HTTP {status}: {raw}"
        )));
    }
    parse_wechat_response(&raw)
}

/// 写入最新 `component_verify_ticket`（授权事件回调调用；微信每 10 分钟
/// 推送一次，覆盖写即可）。
///
/// # Errors
/// 缓存写入失败。
pub async fn store_verify_ticket(ticket: &str) -> Result<()> {
    get_cache()
        .set_str(TICKET_CACHE_KEY, ticket, TICKET_TTL_SECS)
        .await
        .map_err(|e| Error::Internal(format!("verify_ticket 缓存失败: {e}")))
}

/// 是否已收到过 verify_ticket（后台「事件通道是否已通」的探活口径）。
pub async fn has_verify_ticket() -> bool {
    get_cache()
        .get_str(TICKET_CACHE_KEY)
        .await
        .ok()
        .flatten()
        .is_some_and(|t| !t.trim().is_empty())
}

async fn cached_verify_ticket() -> Result<String> {
    get_cache()
        .get_str(TICKET_CACHE_KEY)
        .await
        .ok()
        .flatten()
        .filter(|t| !t.trim().is_empty())
        .ok_or_else(|| {
            biz(
                "尚未收到微信推送的 component_verify_ticket：请先在微信开放平台把授权事件接收 URL 配置为本商城的 \
                 /api/content/wechatComponent/event 并等待约 10 分钟推送",
            )
        })
}

fn component_fetch_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

/// 获取 `component_access_token`（缓存 + 单飞；未配置 / 未收到 ticket →
/// 明确业务错误，诚实未启用）。
///
/// # Errors
/// 未配置、ticket 缺失、网络错误、微信 errcode。
pub async fn component_access_token(repos: &Repositories) -> Result<String> {
    if let Some(token) = cached_token(COMPONENT_TOKEN_CACHE_KEY).await {
        return Ok(token);
    }
    let _guard = component_fetch_lock().lock().await;
    if let Some(token) = cached_token(COMPONENT_TOKEN_CACHE_KEY).await {
        return Ok(token);
    }
    let cfg = load_component_config(repos).await?;
    if !cfg.has_token_credentials() {
        return Err(biz(format!(
            "微信第三方平台未配置：缺少 {}（后台「店铺小程序 → 第三方平台设置」）",
            cfg.missing().join("/")
        )));
    }
    let ticket = cached_verify_ticket().await?;
    let url = format!("{API_BASE}/cgi-bin/component/api_component_token");
    let value = post_json(
        &url,
        &serde_json::json!({
            "component_appid": cfg.app_id,
            "component_appsecret": cfg.app_secret,
            "component_verify_ticket": ticket,
        }),
    )
    .await?;
    let token = value
        .get("component_access_token")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Internal("微信 component token 响应缺少 component_access_token".into()))?
        .to_string();
    let expires_in = value.get("expires_in").and_then(Value::as_u64).unwrap_or(7_200);
    put_cached_token(COMPONENT_TOKEN_CACHE_KEY, &token, expires_in).await;
    Ok(token)
}

/// 清除 component token 缓存（凭据变更后由设置保存路径调用）。
pub async fn invalidate_component_token() {
    let _ = get_cache().del(COMPONENT_TOKEN_CACHE_KEY).await;
}

/// 获取预授权码（`pre_auth_code`，有效期 600 秒，官方口径）。
///
/// # Errors
/// 未配置 / 网络错误 / 微信 errcode。
pub async fn create_pre_auth_code(repos: &Repositories) -> Result<String> {
    let cfg = load_component_config(repos).await?;
    let token = component_access_token(repos).await?;
    let url = format!(
        "{API_BASE}/cgi-bin/component/api_create_preauthcode?component_access_token={}",
        urlencode(&token)
    );
    let value = post_json(&url, &serde_json::json!({ "component_appid": cfg.app_id })).await?;
    value
        .get("pre_auth_code")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| Error::Internal("微信预授权码响应缺少 pre_auth_code".into()))
}

/// 拼接授权页链接（官方《授权流程技术说明》）：返回 (PC 版, H5 版)。
/// `auth_type=2` 仅展示小程序。纯函数，便于测试。
#[must_use]
pub fn authorization_urls(
    component_app_id: &str,
    pre_auth_code: &str,
    redirect_uri: &str,
) -> (String, String) {
    let pc = format!(
        "https://mp.weixin.qq.com/cgi-bin/componentloginpage?component_appid={}&pre_auth_code={}&redirect_uri={}&auth_type={}",
        urlencode(component_app_id),
        urlencode(pre_auth_code),
        urlencode(redirect_uri),
        AUTH_TYPE_MINI_PROGRAM,
    );
    let h5 = format!(
        "https://open.weixin.qq.com/wxaopen/safe/bindcomponent?action=bindcomponent&no_scan=1&component_appid={}&pre_auth_code={}&redirect_uri={}&auth_type={}#wechat_redirect",
        urlencode(component_app_id),
        urlencode(pre_auth_code),
        urlencode(redirect_uri),
        AUTH_TYPE_MINI_PROGRAM,
    );
    (pc, h5)
}

/// 从 `func_info` 数组提取权限集 id 列表（`[{"funcscope_category":{"id":1}}]`
/// → `[1]`）。纯函数，便于测试。
#[must_use]
pub fn parse_func_ids(func_info: Option<&Value>) -> Vec<i64> {
    let Some(list) = func_info.and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|item| {
            item.get("funcscope_category")
                .and_then(|c| c.get("id"))
                .and_then(Value::as_i64)
        })
        .collect()
}

/// `api_query_auth` 的换取结果。
#[derive(Debug, Clone)]
pub struct QueryAuthOutcome {
    pub authorizer_app_id: String,
    pub authorizer_access_token: String,
    pub expires_in: u64,
    /// 授权方刷新令牌（高价值凭据，调用方必须加密落库）。
    pub refresh_token: String,
    pub func_ids: Vec<i64>,
}

/// 用授权码换取授权信息（授权页回调与 `authorized` 事件共用）。
///
/// # Errors
/// 未配置 / 授权码失效 / 网络错误 / 微信 errcode。
pub async fn query_auth(repos: &Repositories, authorization_code: &str) -> Result<QueryAuthOutcome> {
    let cfg = load_component_config(repos).await?;
    let token = component_access_token(repos).await?;
    let url = format!(
        "{API_BASE}/cgi-bin/component/api_query_auth?component_access_token={}",
        urlencode(&token)
    );
    let value = post_json(
        &url,
        &serde_json::json!({
            "component_appid": cfg.app_id,
            "authorization_code": authorization_code,
        }),
    )
    .await?;
    let info = value
        .get("authorization_info")
        .ok_or_else(|| Error::Internal("微信 query_auth 响应缺少 authorization_info".into()))?;
    let authorizer_app_id = info
        .get("authorizer_appid")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Internal("微信 query_auth 响应缺少 authorizer_appid".into()))?
        .to_string();
    let refresh_token = info
        .get("authorizer_refresh_token")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Internal("微信 query_auth 响应缺少 authorizer_refresh_token".into()))?
        .to_string();
    Ok(QueryAuthOutcome {
        authorizer_app_id,
        authorizer_access_token: info
            .get("authorizer_access_token")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        expires_in: info.get("expires_in").and_then(Value::as_u64).unwrap_or(7_200),
        refresh_token,
        func_ids: parse_func_ids(info.get("func_info")),
    })
}

/// 刷新授权方 token：返回 (新 access_token, 有效期秒, 响应带回的 refresh
/// token——官方口径下一般与原值相同，带回则以新值落库为准）。
///
/// # Errors
/// 未配置 / 授权已失效 / 网络错误 / 微信 errcode。
pub async fn refresh_authorizer_token(
    repos: &Repositories,
    authorizer_app_id: &str,
    refresh_token: &str,
) -> Result<(String, u64, Option<String>)> {
    let cfg = load_component_config(repos).await?;
    let token = component_access_token(repos).await?;
    let url = format!(
        "{API_BASE}/cgi-bin/component/api_authorizer_token?component_access_token={}",
        urlencode(&token)
    );
    let value = post_json(
        &url,
        &serde_json::json!({
            "component_appid": cfg.app_id,
            "authorizer_appid": authorizer_app_id,
            "authorizer_refresh_token": refresh_token,
        }),
    )
    .await?;
    let access = value
        .get("authorizer_access_token")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Internal("微信 authorizer token 响应缺少 authorizer_access_token".into()))?
        .to_string();
    let expires_in = value.get("expires_in").and_then(Value::as_u64).unwrap_or(7_200);
    let new_refresh = value
        .get("authorizer_refresh_token")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    Ok((access, expires_in, new_refresh))
}

/// 授权方基本信息（`api_get_authorizer_info` 的归一化结果）。
#[derive(Debug, Clone, Default)]
pub struct AuthorizerInfo {
    pub nick_name: String,
    pub head_img: String,
    /// 原始 ID（gh_ 开头）。
    pub user_name: String,
    pub principal_name: String,
    pub func_ids: Vec<i64>,
}

/// 拉取授权方基本信息（昵称/头像/主体/权限集），供后台回显与同步。
///
/// # Errors
/// 未配置 / 网络错误 / 微信 errcode。
pub async fn get_authorizer_info(
    repos: &Repositories,
    authorizer_app_id: &str,
) -> Result<AuthorizerInfo> {
    let cfg = load_component_config(repos).await?;
    let token = component_access_token(repos).await?;
    let url = format!(
        "{API_BASE}/cgi-bin/component/api_get_authorizer_info?component_access_token={}",
        urlencode(&token)
    );
    let value = post_json(
        &url,
        &serde_json::json!({
            "component_appid": cfg.app_id,
            "authorizer_appid": authorizer_app_id,
        }),
    )
    .await?;
    let info = value.get("authorizer_info").cloned().unwrap_or(Value::Null);
    let get = |key: &str| {
        info.get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    let func_ids = parse_func_ids(
        value
            .get("authorization_info")
            .and_then(|a| a.get("func_info")),
    );
    Ok(AuthorizerInfo {
        nick_name: get("nick_name"),
        head_img: get("head_img"),
        user_name: get("user_name"),
        principal_name: get("principal_name"),
        func_ids,
    })
}

/// 代授权方小程序 `jscode2session`（`GET /sns/component/jscode2session`）：
/// 模式②店铺小程序的买家登录换 openid，与直连版
/// [`super::wechat_client::code2session`] 分工——本路径不需要、也没有
/// 授权方 AppSecret，只凭 component 令牌代调用。
///
/// # Errors
/// 未配置 / 网络错误 / 微信 errcode。
pub async fn component_code2session(
    repos: &Repositories,
    authorizer_app_id: &str,
    code: &str,
) -> Result<Value> {
    let cfg = load_component_config(repos).await?;
    if !cfg.has_token_credentials() {
        return Err(biz("微信第三方平台未配置，无法代授权方小程序登录"));
    }
    let token = component_access_token(repos).await?;
    let url = format!(
        "{API_BASE}/sns/component/jscode2session?appid={}&code={}&grant_type=authorization_code&component_appid={}&component_access_token={}",
        urlencode(authorizer_app_id),
        urlencode(code),
        urlencode(&cfg.app_id),
        urlencode(&token),
    );
    let resp = reqwest::Client::new()
        .get(url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| Error::Internal(format!("微信 component code2session 请求失败: {e}")))?;
    let body = resp
        .text()
        .await
        .map_err(|e| Error::Internal(format!("微信响应读取失败: {e}")))?;
    parse_wechat_response(&body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_missing_lists_all_codes_when_empty() {
        let cfg = ComponentConfig::default();
        assert!(!cfg.is_configured());
        assert_eq!(cfg.missing().len(), 4);
    }

    #[test]
    fn authorization_urls_use_official_hosts_and_mini_only_auth_type() {
        let (pc, h5) = authorization_urls(
            "wxcomponent",
            "pre@code",
            "https://shop.example.com/api/content/wechatComponent/authCallback?state=s1",
        );
        assert!(pc.starts_with("https://mp.weixin.qq.com/cgi-bin/componentloginpage?"));
        assert!(pc.contains("component_appid=wxcomponent"));
        assert!(pc.contains("pre_auth_code=pre%40code"));
        assert!(pc.contains("auth_type=2"));
        assert!(pc.contains("redirect_uri=https%3A%2F%2Fshop.example.com"));
        assert!(h5.starts_with("https://open.weixin.qq.com/wxaopen/safe/bindcomponent?"));
        assert!(h5.contains("no_scan=1"));
        assert!(h5.ends_with("#wechat_redirect"));
    }

    #[test]
    fn parse_func_ids_extracts_category_ids() {
        let value = serde_json::json!([
            {"funcscope_category": {"id": 1}},
            {"funcscope_category": {"id": 17}},
            {"unexpected": true},
        ]);
        assert_eq!(parse_func_ids(Some(&value)), vec![1, 17]);
        assert!(parse_func_ids(None).is_empty());
    }
}
