//! 微信开放平台第三方平台回调 —— 商城前台路由（content::wechat_component）。
//!
//! 店铺独立绑定小程序「模式②」（论坛帖 487946315732211118，我方新增）。
//! 两个免登录端点（调用方是微信服务器 / 商家浏览器，无商城登录态）：
//!
//! | 本模块路由（挂载于 `/api` 之下） | 说明 | HTTP |
//! |----------------------------------|------|------|
//! | `POST /content/wechatComponent/event` | 授权事件接收 URL：component_verify_ticket（每 10 分钟）、authorized / updateauthorized / unauthorized 事件 | POST |
//! | `GET /content/wechatComponent/authCallback` | 授权页 redirect_uri 落点：带 auth_code + state 回跳，完成授权落库后回简易结果页 | GET |
//!
//! > **事件通道口径**：第三方平台消息为安全模式（AES 加密 + msg_signature
//! > 校验），验签/解密复用公众号安全模式工具
//! > （`services::member::wechat_mp`）；解密后的事件应用（ticket 入缓存、
//! > 授权收敛、撤销置态）在
//! > `services::merchant::shop_mini_program_auth::apply_component_event`。
//! > 处理成功一律回纯文本 `success`（微信约定，否则会重推）；验签失败
//! > 回 403。第三方平台未配置时事件端点只记录日志并回 `success`
//! > （功能未启用，不产生任何副作用）。
//! >
//! > **作用域中间件**：两端点均无 `x-mini-app-id` 头，
//! > `mini_shop_scope_layer` 直接放行，不受店铺小程序作用域影响。

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::Deserialize;

use nextrade_services::content::wechat_component;
use nextrade_services::member::wechat_mp;
use nextrade_services::merchant::shop_mini_program_auth;

use crate::PublicState;

/// 本域路由表（完整契约路径，供覆盖率统计）。
const ROUTE_TABLE: [(&str, &str, &str); 2] = [
    ("POST", "/api/content/wechatComponent/event", "WechatComponentController.event (本平台新增)"),
    ("GET", "/api/content/wechatComponent/authCallback", "WechatComponentController.authCallback (本平台新增)"),
];

pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .route("/content/wechatComponent/event", post(event))
        .route("/content/wechatComponent/authCallback", get(auth_callback))
}

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, java_ref)| nextrade_openapi::RouteDoc::new(method, path, java_ref))
        .collect()
}

#[derive(Debug, Default, Deserialize)]
struct EventQuery {
    msg_signature: Option<String>,
    timestamp: Option<String>,
    nonce: Option<String>,
}

/// 授权事件接收：验签 → 解密 → 应用事件 → 回 `success`。
async fn event(
    State(state): State<PublicState>,
    Query(q): Query<EventQuery>,
    body: String,
) -> Response {
    let cfg = match wechat_component::load_component_config(&state.repos).await {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::error!("第三方平台配置读取失败: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "error").into_response();
        }
    };
    if !cfg.has_event_credentials() {
        // 未配置即未启用：不验签不解密（无凭据可验），仅记录后回 success，
        // 避免微信侧无意义重推；功能启用前该 URL 本不应被填入开放平台。
        tracing::warn!("收到第三方平台事件推送，但第三方平台未配置，忽略");
        return (StatusCode::OK, "success").into_response();
    }
    let Some(encrypt) = wechat_mp::xml_field(&body, "Encrypt") else {
        return (StatusCode::BAD_REQUEST, "invalid body").into_response();
    };
    let (Ok(timestamp), Ok(nonce), Ok(msg_signature)) = (
        q.timestamp.ok_or(()),
        q.nonce.ok_or(()),
        q.msg_signature.ok_or(()),
    ) else {
        return (StatusCode::BAD_REQUEST, "missing signature params").into_response();
    };
    if !wechat_mp::verify_msg_signature(&cfg.token, &timestamp, &nonce, &encrypt, &msg_signature) {
        tracing::warn!("第三方平台事件验签失败");
        return (StatusCode::FORBIDDEN, "signature error").into_response();
    }
    let plain = match wechat_mp::decrypt_message(&cfg.aes_key, &cfg.app_id, &encrypt) {
        Ok(plain) => plain,
        Err(e) => {
            tracing::warn!("第三方平台事件解密失败: {e}");
            return (StatusCode::FORBIDDEN, "decrypt error").into_response();
        }
    };
    if let Err(e) = shop_mini_program_auth::apply_component_event(&state.repos, &plain).await {
        // 应用失败（多为数据库瞬时错误）：仍回 success —— ticket 下个
        // 10 分钟会再推，授权类事件的丢失由后台「同步信息」与商家在
        // mp 后台的状态兜底，不靠微信重推（重推同一事件亦无法补回
        // 已过期的授权码）。
        tracing::error!("第三方平台事件应用失败: {e}");
    }
    (StatusCode::OK, "success").into_response()
}

#[derive(Debug, Default, Deserialize)]
struct AuthCallbackQuery {
    auth_code: Option<String>,
    state: Option<String>,
}

/// 授权完成回调：用 auth_code + state 完成授权落库，回简易结果页
/// （商家在浏览器里看到结果后自行返回后台，授权状态以后台列表为准）。
async fn auth_callback(
    State(state): State<PublicState>,
    Query(q): Query<AuthCallbackQuery>,
) -> Response {
    let Some(auth_code) = q
        .auth_code
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return Html(result_page("授权未完成", "回调缺少授权码，请返回后台重新发起授权。"))
            .into_response();
    };
    match shop_mini_program_auth::complete_authorization(
        &state.repos,
        auth_code,
        q.state.as_deref(),
    )
    .await
    {
        Ok(_) => Html(result_page(
            "授权成功",
            "小程序已授权给平台，请返回商城后台「店铺小程序」查看授权状态。",
        ))
        .into_response(),
        Err(e) => {
            tracing::warn!("授权回调完成失败: {e}");
            Html(result_page(
                "授权未完成",
                &format!("授权信息保存失败：{e}。请返回后台重新发起授权。"),
            ))
            .into_response()
        }
    }
}

fn result_page(title: &str, message: &str) -> String {
    // 静态文案页：title/message 均为服务端固定文案或内部错误串，做最小
    // HTML 转义防破碎（不引入模板引擎）。
    let escape = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    format!(
        "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>{}</title></head><body style=\"font-family:sans-serif;text-align:center;padding:64px 24px\">\
         <h2>{}</h2><p style=\"color:#666\">{}</p></body></html>",
        escape(title),
        escape(title),
        escape(message),
    )
}
