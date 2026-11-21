//! 渠道订单推送回调（takeout::callback，我方新增）。
//!
//! 美团按事件类型在应用后台配置各自的回调地址，本模块对应四个入口：
//! - `POST /api/takeout/callback/meituan/order`（已支付订单推送）
//! - `POST /api/takeout/callback/meituan/cancel`（取消推送）
//! - `POST /api/takeout/callback/meituan/refund`（退款推送）
//! - `POST /api/takeout/callback/meituan/status`（状态推送）
//!
//! 淘宝闪购（饿了么）在应用后台只配置**一个**推送地址，事件靠推送
//! type 与订单状态字符串在适配器内分类：
//! - `POST /api/takeout/callback/eleme`（全事件共用）
//!
//! 抖音生活服务在开放平台 Webhooks 配置中只配**一个**回调地址，
//! 签名在请求头 `X-Douyin-Signature`，事件靠 body 的 `event` 字段
//! 在适配器内分类：
//! - `POST /api/takeout/callback/douyin`（全事件共用）
//!
//! 响应口径（各渠道官方）：美团受理成功回 `{"data":"ok"}`；饿了么
//! 回 `{"message":"ok"}`；抖音 HTTP 200 即成功（响应体可空）。其余
//! 情形回非成功形态（未配置 503 / 验签失败 403 / 处理异常 500），
//! 平台会按其规则重试，重复推送由接入层幂等兜底 —— 任何情况下都
//! 不在未落库时假回成功。
//!
//! 抖音回调地址**验证请求**（配置回调地址时平台先行发送，content
//! 为对象且含 `challenge`）须原样回 challenge 才能配置成功；该
//! 请求发生在凭据配置之前，故在路由层接入处理链之前应答。

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde_json::json;

use nextrade_openapi::RouteDoc;
use nextrade_services::takeout::ingest::CallbackOutcome;

use crate::PublicState;

/// 本模块路由表（覆盖率统计用）。
const ROUTE_TABLE: [(&str, &str, &str); 6] = [
    ("POST", "/takeout/callback/meituan/order", "TakeoutCallbackController.meituanOrder"),
    ("POST", "/takeout/callback/meituan/cancel", "TakeoutCallbackController.meituanCancel"),
    ("POST", "/takeout/callback/meituan/refund", "TakeoutCallbackController.meituanRefund"),
    ("POST", "/takeout/callback/meituan/status", "TakeoutCallbackController.meituanStatus"),
    ("POST", "/takeout/callback/eleme", "TakeoutCallbackController.elemePush"),
    ("POST", "/takeout/callback/douyin", "TakeoutCallbackController.douyinPush"),
];

/// 回调路由汇总。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .route("/takeout/callback/meituan/order", post(meituan_order))
        .route("/takeout/callback/meituan/cancel", post(meituan_cancel))
        .route("/takeout/callback/meituan/refund", post(meituan_refund))
        .route("/takeout/callback/meituan/status", post(meituan_status))
        .route("/takeout/callback/eleme", post(eleme_push))
        .route("/takeout/callback/douyin", post(douyin_push))
}

/// 本模块已实现路由（完整契约路径）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/api{path}"), handler))
        .collect()
}

/// 美团统一处理并映射响应形态（受理回 `{"data":"ok"}`）。
async fn handle(state: &PublicState, hint: &str, body: Bytes) -> Response {
    let raw = String::from_utf8_lossy(&body).into_owned();
    match state
        .takeout
        .ingest
        .handle_callback("meituan", hint, &raw)
        .await
    {
        CallbackOutcome::Accepted => (StatusCode::OK, Json(json!({ "data": "ok" }))).into_response(),
        CallbackOutcome::NotConfigured => {
            (StatusCode::SERVICE_UNAVAILABLE, "channel not configured").into_response()
        }
        CallbackOutcome::Rejected(msg) => (StatusCode::FORBIDDEN, msg).into_response(),
        CallbackOutcome::ProcessingFailed(msg) => {
            (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
        }
    }
}

async fn meituan_order(State(state): State<PublicState>, body: Bytes) -> Response {
    handle(&state, "order", body).await
}

async fn meituan_cancel(State(state): State<PublicState>, body: Bytes) -> Response {
    handle(&state, "cancel", body).await
}

async fn meituan_refund(State(state): State<PublicState>, body: Bytes) -> Response {
    handle(&state, "refund", body).await
}

async fn meituan_status(State(state): State<PublicState>, body: Bytes) -> Response {
    handle(&state, "status", body).await
}

/// 淘宝闪购统一推送入口（受理回 `{"message":"ok"}`，平台口径）。
async fn eleme_push(State(state): State<PublicState>, body: Bytes) -> Response {
    let raw = String::from_utf8_lossy(&body).into_owned();
    match state
        .takeout
        .ingest
        .handle_callback("eleme", "push", &raw)
        .await
    {
        CallbackOutcome::Accepted => {
            (StatusCode::OK, Json(json!({ "message": "ok" }))).into_response()
        }
        CallbackOutcome::NotConfigured => {
            (StatusCode::SERVICE_UNAVAILABLE, "channel not configured").into_response()
        }
        CallbackOutcome::Rejected(msg) => (StatusCode::FORBIDDEN, msg).into_response(),
        CallbackOutcome::ProcessingFailed(msg) => {
            (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
        }
    }
}

/// 抖音 WebHooks 统一回调入口（HTTP 200 即成功，响应体可空）。
async fn douyin_push(
    State(state): State<PublicState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let raw = String::from_utf8_lossy(&body).into_owned();
    // 回调地址验证请求：content 为对象且含 challenge，原样回
    // challenge（发生在凭据配置之前，不走接入处理链、不验签）。
    let challenge = serde_json::from_str::<serde_json::Value>(&raw)
        .ok()
        .and_then(|env| {
            env.get("content")
                .and_then(|c| c.get("challenge"))
                .cloned()
        });
    if let Some(challenge) = challenge {
        return (StatusCode::OK, Json(json!({ "challenge": challenge }))).into_response();
    }
    let signature = headers
        .get("x-douyin-signature")
        .and_then(|v| v.to_str().ok());
    match state
        .takeout
        .ingest
        .handle_callback_signed("douyin", "push", &raw, signature)
        .await
    {
        CallbackOutcome::Accepted => StatusCode::OK.into_response(),
        CallbackOutcome::NotConfigured => {
            (StatusCode::SERVICE_UNAVAILABLE, "channel not configured").into_response()
        }
        CallbackOutcome::Rejected(msg) => (StatusCode::FORBIDDEN, msg).into_response(),
        CallbackOutcome::ProcessingFailed(msg) => {
            (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
        }
    }
}
