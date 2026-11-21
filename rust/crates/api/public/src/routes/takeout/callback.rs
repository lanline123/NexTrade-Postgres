//! 渠道订单推送回调（takeout::callback，我方新增）。
//!
//! 美团按事件类型在应用后台配置各自的回调地址，本模块对应四个入口：
//! - `POST /api/takeout/callback/meituan/order`（已支付订单推送）
//! - `POST /api/takeout/callback/meituan/cancel`（取消推送）
//! - `POST /api/takeout/callback/meituan/refund`（退款推送）
//! - `POST /api/takeout/callback/meituan/status`（状态推送）
//!
//! 响应口径（美团官方）：受理成功必须回 `{"data":"ok"}`；其余情形
//! 回非 ok（未配置 503 / 验签失败 403 / 处理异常 500），平台会按
//! 其规则重试，重复推送由接入层幂等兜底 —— 任何情况下都不在
//! 未落库时假回 ok。

use axum::{
    body::Bytes,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde_json::json;

use nextrade_openapi::RouteDoc;
use nextrade_services::takeout::ingest::CallbackOutcome;

use crate::PublicState;

/// 本模块路由表（覆盖率统计用）。
const ROUTE_TABLE: [(&str, &str, &str); 4] = [
    ("POST", "/takeout/callback/meituan/order", "TakeoutCallbackController.meituanOrder"),
    ("POST", "/takeout/callback/meituan/cancel", "TakeoutCallbackController.meituanCancel"),
    ("POST", "/takeout/callback/meituan/refund", "TakeoutCallbackController.meituanRefund"),
    ("POST", "/takeout/callback/meituan/status", "TakeoutCallbackController.meituanStatus"),
];

/// 回调路由汇总。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .route("/takeout/callback/meituan/order", post(meituan_order))
        .route("/takeout/callback/meituan/cancel", post(meituan_cancel))
        .route("/takeout/callback/meituan/refund", post(meituan_refund))
        .route("/takeout/callback/meituan/status", post(meituan_status))
}

/// 本模块已实现路由（完整契约路径）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/api{path}"), handler))
        .collect()
}

/// 统一处理并映射响应形态。
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
