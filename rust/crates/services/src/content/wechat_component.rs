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

// ---------------------------------------------------------------------------
// 小程序代码管理（代提审 / 代发布；第三方平台「代商家管理小程序」官方口径）
//
// 全部接口属权限集 18（小程序开发管理），以 `authorizer_access_token`
// 代授权方调用（模板库管理除外，用 `component_access_token`）。本节只做
// 协议层：入参组装、响应解析；状态落库与阶段推导在
// [`crate::merchant::shop_mini_program_release`]。微信 errcode 非 0 时
// `parse_wechat_response` 直接转业务错误——失败如实上抛，绝不假成功。
// ---------------------------------------------------------------------------

async fn get_json(url: &str) -> Result<Value> {
    let resp = reqwest::Client::new()
        .get(url)
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

fn str_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn i64_field(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(Value::as_i64).unwrap_or(0)
}

/// 提审类目（`GET /wxa/get_category` 的 `category_list` 一项）：商家小程序
/// 已申请通过的类目，提审 `item_list` 的类目字段必须取自此列表。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AuditCategory {
    pub first_class: String,
    pub second_class: String,
    pub first_id: i64,
    pub second_id: i64,
}

/// 解析 `category_list`（纯函数，便于测试）。
#[must_use]
pub fn parse_category_list(value: &Value) -> Vec<AuditCategory> {
    let Some(list) = value.get("category_list").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .map(|item| AuditCategory {
            first_class: str_field(item, "first_class"),
            second_class: str_field(item, "second_class"),
            first_id: i64_field(item, "first_id"),
            second_id: i64_field(item, "second_id"),
        })
        .collect()
}

/// 提审项（`submit_audit` 的 `item_list` 一项；官方要求 1–5 项）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AuditItemInput {
    /// 页面路径（须取自 `get_page` 的已提交代码页面列表）。
    pub address: String,
    /// 类目贴标签（页面所属服务类目对应的标签，如「电商」）。
    pub tag: String,
    pub first_class: String,
    pub second_class: String,
    pub first_id: i64,
    pub second_id: i64,
    /// 页面标题（在微信侧展示的页面名称）。
    pub title: String,
}

impl AuditItemInput {
    /// 转微信请求体的一项（字段名与官方一致，蛇形）。
    #[must_use]
    pub fn to_wechat_json(&self) -> Value {
        serde_json::json!({
            "address": self.address,
            "tag": self.tag,
            "first_class": self.first_class,
            "second_class": self.second_class,
            "first_id": self.first_id,
            "second_id": self.second_id,
            "title": self.title,
        })
    }
}

/// 审核状态归一化结果（`get_auditstatus` / `get_latest_auditstatus` 共用）。
/// `status` 为微信口径：0 审核成功 / 1 审核被拒绝 / 2 审核中 / 3 已撤回 /
/// 4 审核延后。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AuditStatusInfo {
    pub audit_id: i64,
    pub status: i64,
    pub reason: String,
    /// 拒绝时的截图 media_id 列表（竖线分隔；新版字段名 `screenshot`，
    /// 旧版为 `ScreenShot`，两者兼容读取）。
    pub screenshot: String,
    pub user_version: String,
    pub user_desc: String,
    pub submit_audit_time: i64,
}

/// 解析审核状态响应（纯函数，便于测试）。
#[must_use]
pub fn parse_audit_status(value: &Value) -> AuditStatusInfo {
    let screenshot = {
        let lower = str_field(value, "screenshot");
        if lower.is_empty() {
            str_field(value, "ScreenShot")
        } else {
            lower
        }
    };
    AuditStatusInfo {
        audit_id: i64_field(value, "auditid"),
        status: i64_field(value, "status"),
        reason: str_field(value, "reason"),
        screenshot,
        user_version: str_field(value, "user_version"),
        user_desc: str_field(value, "user_desc"),
        submit_audit_time: i64_field(value, "submit_audit_time"),
    }
}

/// 可回退历史版本（`revertcoderelease?action=get_history_version` 一项；
/// 微信最多保存最近发布或回退的 5 个版本）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HistoryVersion {
    pub app_version: i64,
    pub user_version: String,
    pub user_desc: String,
    pub commit_time: i64,
}

/// 解析 `version_list`（纯函数，便于测试）。
#[must_use]
pub fn parse_history_versions(value: &Value) -> Vec<HistoryVersion> {
    let Some(list) = value.get("version_list").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .map(|item| HistoryVersion {
            app_version: i64_field(item, "app_version"),
            user_version: str_field(item, "user_version"),
            user_desc: str_field(item, "user_desc"),
            commit_time: i64_field(item, "commit_time"),
        })
        .collect()
}

/// 解析 `page_list`（纯函数，便于测试）。
#[must_use]
pub fn parse_page_list(value: &Value) -> Vec<String> {
    value
        .get("page_list")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|p| p.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// 代码模板草稿（模板库草稿箱一项；草稿来自第三方平台的开发小程序）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TemplateDraft {
    pub draft_id: i64,
    pub user_version: String,
    pub user_desc: String,
    pub source_miniprogram_appid: String,
    pub source_miniprogram: String,
    pub create_time: i64,
}

/// 代码模板（模板库一项；`template_id` 是代提交代码的入参）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CodeTemplate {
    pub template_id: i64,
    pub user_version: String,
    pub user_desc: String,
    pub source_miniprogram_appid: String,
    pub source_miniprogram: String,
    pub create_time: i64,
}

fn parse_template_row(item: &Value) -> (String, String, String, String, i64) {
    (
        str_field(item, "user_version"),
        str_field(item, "user_desc"),
        str_field(item, "source_miniprogram_appid"),
        str_field(item, "source_miniprogram"),
        i64_field(item, "create_time"),
    )
}

/// 解析 `draft_list`（纯函数，便于测试）。
#[must_use]
pub fn parse_draft_list(value: &Value) -> Vec<TemplateDraft> {
    let Some(list) = value.get("draft_list").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .map(|item| {
            let (user_version, user_desc, appid, name, create_time) = parse_template_row(item);
            TemplateDraft {
                draft_id: i64_field(item, "draft_id"),
                user_version,
                user_desc,
                source_miniprogram_appid: appid,
                source_miniprogram: name,
                create_time,
            }
        })
        .collect()
}

/// 解析 `template_list`（纯函数，便于测试）。
#[must_use]
pub fn parse_template_list(value: &Value) -> Vec<CodeTemplate> {
    let Some(list) = value.get("template_list").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .map(|item| {
            let (user_version, user_desc, appid, name, create_time) = parse_template_row(item);
            CodeTemplate {
                template_id: i64_field(item, "template_id"),
                user_version,
                user_desc,
                source_miniprogram_appid: appid,
                source_miniprogram: name,
                create_time,
            }
        })
        .collect()
}

/// 代提交代码（`POST /wxa/commit`，权限集 18）：以模板 + 按店 `ext_json`
/// 为授权小程序上传代码并生成体验版。上传后需等微信代码检测任务结束
/// 方可提审（过早提审微信回 61039，错误如实上抛）。
///
/// # Errors
/// 网络错误 / 微信 errcode（如 61007 未授权该权限集）。
pub async fn code_commit(
    authorizer_access_token: &str,
    template_id: i64,
    ext_json: &str,
    user_version: &str,
    user_desc: &str,
) -> Result<()> {
    let url = format!(
        "{API_BASE}/wxa/commit?access_token={}",
        urlencode(authorizer_access_token)
    );
    post_json(
        &url,
        &serde_json::json!({
            "template_id": template_id,
            "ext_json": ext_json,
            "user_version": user_version,
            "user_desc": user_desc,
        }),
    )
    .await?;
    Ok(())
}

/// 获取已提交代码的页面列表（`GET /wxa/get_page`），提审项页面取自此列表。
///
/// # Errors
/// 网络错误 / 微信 errcode（86001 = 不存在已提交的代码）。
pub async fn code_get_page(authorizer_access_token: &str) -> Result<Vec<String>> {
    let url = format!(
        "{API_BASE}/wxa/get_page?access_token={}",
        urlencode(authorizer_access_token)
    );
    let value = get_json(&url).await?;
    Ok(parse_page_list(&value))
}

/// 获取授权小程序已申请通过的类目（`GET /wxa/get_category`），提审项
/// 类目必须取自此列表。
///
/// # Errors
/// 网络错误 / 微信 errcode。
pub async fn code_get_category(authorizer_access_token: &str) -> Result<Vec<AuditCategory>> {
    let url = format!(
        "{API_BASE}/wxa/get_category?access_token={}",
        urlencode(authorizer_access_token)
    );
    let value = get_json(&url).await?;
    Ok(parse_category_list(&value))
}

/// 提交审核（`POST /wxa/submit_audit`）：返回微信审核单 id（`auditid`）。
/// `feedback_info`（审核反馈说明）与 `version_desc`（版本描述）为空时
/// 不下发该字段。
///
/// # Errors
/// 网络错误 / 微信 errcode（61039 = 代码检测未结束、85012 等）。
pub async fn code_submit_audit(
    authorizer_access_token: &str,
    items: &[AuditItemInput],
    feedback_info: Option<&str>,
    version_desc: Option<&str>,
) -> Result<i64> {
    let url = format!(
        "{API_BASE}/wxa/submit_audit?access_token={}",
        urlencode(authorizer_access_token)
    );
    let mut body = serde_json::json!({
        "item_list": items.iter().map(AuditItemInput::to_wechat_json).collect::<Vec<_>>(),
    });
    if let Some(info) = feedback_info.map(str::trim).filter(|s| !s.is_empty()) {
        body["feedback_info"] = Value::String(info.to_string());
    }
    if let Some(desc) = version_desc.map(str::trim).filter(|s| !s.is_empty()) {
        body["version_desc"] = Value::String(desc.to_string());
    }
    let value = post_json(&url, &body).await?;
    let audit_id = i64_field(&value, "auditid");
    if audit_id <= 0 {
        return Err(Error::Internal("微信 submit_audit 响应缺少 auditid".into()));
    }
    Ok(audit_id)
}

/// 查询指定审核单状态（`POST /wxa/get_auditstatus`）。
///
/// # Errors
/// 网络错误 / 微信 errcode（85012 = 无效的审核 id）。
pub async fn code_get_audit_status(
    authorizer_access_token: &str,
    audit_id: i64,
) -> Result<AuditStatusInfo> {
    let url = format!(
        "{API_BASE}/wxa/get_auditstatus?access_token={}",
        urlencode(authorizer_access_token)
    );
    let value = post_json(&url, &serde_json::json!({ "auditid": audit_id })).await?;
    let mut info = parse_audit_status(&value);
    if info.audit_id <= 0 {
        info.audit_id = audit_id;
    }
    Ok(info)
}

/// 查询最新一次提审的审核状态（`GET /wxa/get_latest_auditstatus`）。
///
/// # Errors
/// 网络错误 / 微信 errcode（86001 = 不存在已提交的代码/审核单）。
pub async fn code_get_latest_audit_status(
    authorizer_access_token: &str,
) -> Result<AuditStatusInfo> {
    let url = format!(
        "{API_BASE}/wxa/get_latest_auditstatus?access_token={}",
        urlencode(authorizer_access_token)
    );
    let value = get_json(&url).await?;
    Ok(parse_audit_status(&value))
}

/// 发布已通过审核的版本（`POST /wxa/release`，请求体为空对象）。
///
/// # Errors
/// 网络错误 / 微信 errcode（85051 = 没有可发布的审核通过版本等）。
pub async fn code_release(authorizer_access_token: &str) -> Result<()> {
    let url = format!(
        "{API_BASE}/wxa/release?access_token={}",
        urlencode(authorizer_access_token)
    );
    post_json(&url, &serde_json::json!({})).await?;
    Ok(())
}

/// 撤回当前审核单（`GET /wxa/undocodeaudit`；每个版本周期撤回次数有限，
/// 微信侧超限会回错误码，如实上抛）。
///
/// # Errors
/// 网络错误 / 微信 errcode。
pub async fn code_undo_audit(authorizer_access_token: &str) -> Result<()> {
    let url = format!(
        "{API_BASE}/wxa/undocodeaudit?access_token={}",
        urlencode(authorizer_access_token)
    );
    get_json(&url).await?;
    Ok(())
}

/// 查询可回退的历史版本（`GET /wxa/revertcoderelease?action=
/// get_history_version`）。
///
/// # Errors
/// 网络错误 / 微信 errcode。
pub async fn code_history_versions(
    authorizer_access_token: &str,
) -> Result<Vec<HistoryVersion>> {
    let url = format!(
        "{API_BASE}/wxa/revertcoderelease?action=get_history_version&access_token={}",
        urlencode(authorizer_access_token)
    );
    let value = get_json(&url).await?;
    Ok(parse_history_versions(&value))
}

/// 线上版本回退（`GET /wxa/revertcoderelease`）：`app_version` 为 None 时
/// 回退到上一个版本；回退过的版本不能再次回退（微信 87012 如实上抛）。
///
/// # Errors
/// 网络错误 / 微信 errcode（87011 灰度中 / 87012 不可回退）。
pub async fn code_revert(
    authorizer_access_token: &str,
    app_version: Option<i64>,
) -> Result<()> {
    let mut url = format!(
        "{API_BASE}/wxa/revertcoderelease?access_token={}",
        urlencode(authorizer_access_token)
    );
    if let Some(v) = app_version {
        url.push_str(&format!("&app_version={v}"));
    }
    get_json(&url).await?;
    Ok(())
}

/// 模板库草稿箱列表（`GET /wxa/gettemplatedraftlist`，component 令牌）。
/// 草稿由第三方平台的开发小程序在开发者工具上传产生。
///
/// # Errors
/// 未配置 / 网络错误 / 微信 errcode。
pub async fn template_draft_list(repos: &Repositories) -> Result<Vec<TemplateDraft>> {
    let token = component_access_token(repos).await?;
    let url = format!(
        "{API_BASE}/wxa/gettemplatedraftlist?access_token={}",
        urlencode(&token)
    );
    let value = get_json(&url).await?;
    Ok(parse_draft_list(&value))
}

/// 草稿加入模板库（`POST /wxa/addtotemplate`，component 令牌）：
/// `template_type` 0 = 普通模板 / 1 = 标准模板。
///
/// # Errors
/// 未配置 / 网络错误 / 微信 errcode。
pub async fn template_add_to_library(
    repos: &Repositories,
    draft_id: i64,
    template_type: i64,
) -> Result<()> {
    let token = component_access_token(repos).await?;
    let url = format!(
        "{API_BASE}/wxa/addtotemplate?access_token={}",
        urlencode(&token)
    );
    post_json(
        &url,
        &serde_json::json!({ "draft_id": draft_id, "template_type": template_type }),
    )
    .await?;
    Ok(())
}

/// 模板库列表（`GET /wxa/gettemplatelist`，component 令牌）：
/// `template_type` 为 None 时查全部类型。
///
/// # Errors
/// 未配置 / 网络错误 / 微信 errcode。
pub async fn template_list(
    repos: &Repositories,
    template_type: Option<i64>,
) -> Result<Vec<CodeTemplate>> {
    let token = component_access_token(repos).await?;
    let mut url = format!(
        "{API_BASE}/wxa/gettemplatelist?access_token={}",
        urlencode(&token)
    );
    if let Some(t) = template_type {
        url.push_str(&format!("&template_type={t}"));
    }
    let value = get_json(&url).await?;
    Ok(parse_template_list(&value))
}

/// 删除模板（`POST /wxa/deletetemplate`，component 令牌）。
///
/// # Errors
/// 未配置 / 网络错误 / 微信 errcode。
pub async fn template_delete(repos: &Repositories, template_id: i64) -> Result<()> {
    let token = component_access_token(repos).await?;
    let url = format!(
        "{API_BASE}/wxa/deletetemplate?access_token={}",
        urlencode(&token)
    );
    post_json(&url, &serde_json::json!({ "template_id": template_id })).await?;
    Ok(())
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

    #[test]
    fn parse_audit_status_reads_official_fields_and_screenshot_alias() {
        let value = serde_json::json!({
            "errcode": 0,
            "auditid": 1234567,
            "status": 1,
            "reason": "帐号信息不合规范",
            "ScreenShot": "xx|yy",
            "user_version": "1.2.0",
            "user_desc": "修复",
            "submit_audit_time": 1700000000_i64,
        });
        let info = parse_audit_status(&value);
        assert_eq!(info.audit_id, 1234567);
        assert_eq!(info.status, 1);
        assert_eq!(info.reason, "帐号信息不合规范");
        assert_eq!(info.screenshot, "xx|yy");
        assert_eq!(info.user_version, "1.2.0");
        let lower = serde_json::json!({"status": 2, "screenshot": "zz"});
        assert_eq!(parse_audit_status(&lower).screenshot, "zz");
    }

    #[test]
    fn parse_category_page_and_history_lists() {
        let cats = parse_category_list(&serde_json::json!({
            "category_list": [
                {"first_class": "电商平台", "second_class": "百货", "first_id": 1, "second_id": 2}
            ]
        }));
        assert_eq!(cats.len(), 1);
        assert_eq!(cats[0].first_id, 1);
        assert_eq!(cats[0].second_class, "百货");
        let pages = parse_page_list(&serde_json::json!({
            "page_list": ["pages/index/index", "pages/shop/home"]
        }));
        assert_eq!(pages.len(), 2);
        let history = parse_history_versions(&serde_json::json!({
            "version_list": [
                {"app_version": 3, "user_version": "1.1.0", "user_desc": "d", "commit_time": 42}
            ]
        }));
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].app_version, 3);
        assert!(parse_history_versions(&serde_json::json!({})).is_empty());
    }

    #[test]
    fn parse_draft_and_template_lists() {
        let drafts = parse_draft_list(&serde_json::json!({
            "draft_list": [
                {"draft_id": 9, "user_version": "0.9", "user_desc": "草稿",
                 "source_miniprogram_appid": "wxdev", "source_miniprogram": "开发小程序",
                 "create_time": 7}
            ]
        }));
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].draft_id, 9);
        let templates = parse_template_list(&serde_json::json!({
            "template_list": [
                {"template_id": 1234567890123_i64, "user_version": "1.0", "user_desc": "正式",
                 "source_miniprogram_appid": "wxdev", "source_miniprogram": "开发小程序",
                 "create_time": 8}
            ]
        }));
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].template_id, 1234567890123_i64);
    }

    #[test]
    fn audit_item_to_wechat_json_uses_snake_case() {
        let item = AuditItemInput {
            address: "pages/index/index".into(),
            tag: "电商".into(),
            first_class: "电商平台".into(),
            second_class: "百货".into(),
            first_id: 1,
            second_id: 2,
            title: "首页".into(),
        };
        let json = item.to_wechat_json();
        assert_eq!(json["first_id"], 1);
        assert_eq!(json["address"], "pages/index/index");
        assert!(json.get("firstId").is_none());
    }
}
