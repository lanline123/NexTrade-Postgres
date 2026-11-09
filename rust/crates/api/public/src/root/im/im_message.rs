//! IM 消息（ImMessageController） —— 根级路由（im::im_message，public 端）。
//!
//! **路径约定**：原 Java `nextrade-api` 的 `ImMessageController`
//! `@RequestMapping("/im/conversation/message")` **不含** `/api` 前缀，
//! 真实 URL 挂在应用根路径（见 `crate::root` 模块文档）。故本文件路由串写
//! **完整契约路径**（以 `/im` 开头），`implemented_routes()` 亦写完整路径。
//!
//! | 本模块路由 | 原 Java | 登录 |
//! |-----------|--------|------|
//! | `POST /im/conversation/message/send` | `ImMessageController.send` | 会员登录态 |
//! | `GET  /im/conversation/message/list` | `ImMessageController.list` | 会员登录态 |
//! | `POST /im/conversation/message/setRead` | `ImMessageController.setRead` | 会员登录态 |
//!
//! > **认证**：由 `crate::router()` 在根级 `merge_protected` 通道统一附加
//! > （`/im/**` 不在 Java `WhiteListConfig.URL_WHITELIST` 内），本文件不自加中间件。
//! >
//! > **身份口径**（前台 = `getClientTypeByToken() == false`）：
//! > `userId` 用 `SecurityUtils.getCurrentUserId()`——Rust 侧即 JWT `sub`
//! > （[`AuthUser`] 的 `id`）；`send` 额外注入 `X-Client-Type` 请求头作为
//! > `userFrom`（Java `HeaderUtils.getHeaderValue(X_CLIENT_TYPE)`）。
//! >
//! > **未迁移**：`send` 的 WebSocket 推送 + RabbitMQ 自动回复、`setRead` 的
//! > `sendReadToClient` 推送均跳过（详见服务层模块头）。

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::{get, post},
    Extension, Json, Router,
};
use nextrade_core::models::im::im_message::{
    ImMessageListDTO, ImMessageListVO, ImMessageSendDTO, ImMessageSendVO, ImSetReadDTO,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::PublicState;

/// IM 消息域路由（挂载于应用**根路径**，不在 `/api` 下）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .route("/im/conversation/message/send", post(send))
        .route("/im/conversation/message/list", get(list))
        .route("/im/conversation/message/setRead", post(set_read))
}

/// 本域已实现路由（覆盖率统计，写**完整契约路径**）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    [
        ("POST", "/im/conversation/message/send", "ImMessageController.send"),
        ("GET", "/im/conversation/message/list", "ImMessageController.list"),
        (
            "POST",
            "/im/conversation/message/setRead",
            "ImMessageController.setRead",
        ),
    ]
    .iter()
    .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, p, j))
    .collect()
}

/// `POST /im/conversation/message/send`：发送消息（前台分支）。
async fn send(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    headers: HeaderMap,
    Json(dto): Json<ImMessageSendDTO>,
) -> ApiResult<ImMessageSendVO> {
    let user_from = header_str(&headers, "x-client-type");
    ok(state.im.im_message.send_as_user(&dto, auth.id, user_from).await?)
}

/// `GET /im/conversation/message/list`：消息列表（前台分支）。
async fn list(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Query(dto): Query<ImMessageListDTO>,
) -> ApiResult<PageResult<ImMessageListVO>> {
    ok(state.im.im_message.list_as_user(&dto, auth.id).await?)
}

/// `POST /im/conversation/message/setRead`：设置已读（前台分支）。
async fn set_read(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<ImSetReadDTO>,
) -> ApiResult<bool> {
    ok(state.im.im_message.set_read_as_user(&dto, auth.id).await?)
}

/// 读取请求头文本值（缺失 / 非 UTF-8 / 全空白 → `None`）。
fn header_str(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}