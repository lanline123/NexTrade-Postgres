//! 买家侧 AI 导购（受限 MVP，我方新增）—— 商城端路由（system::ai_guide）。
//!
//! 挂在 `authorized_routes`（外层统一 `require_auth`）：仅登录买家可用。
//! 引擎在 services 共享层（`nextrade_services::ai::guide`）：平台维度会话、
//! 4 个只读工具、价格红线（工具走公开口径 + 当前买家身份脱敏）、三级预算熔断。
//!
//! | 路由（挂载于 `/api`） | 说明 |
//! |------------------------|------|
//! | `POST /ai/guide/conversations` | 新建导购会话 |
//! | `GET /ai/guide/conversations` | 我的导购会话列表 |
//! | `GET /ai/guide/conversations/{id}/messages` | 会话消息（tool 行带 cards 回显） |
//! | `DELETE /ai/guide/conversations/{id}` | 删除会话（逻辑删） |
//! | `POST /ai/guide/chat/stream` | 发送消息（SSE 流式回复） |
//! | `POST /ai/guide/chat/stop` | 停止进行中的回复 |

use axum::{
    extract::{Path, State},
    response::{sse::{Event, KeepAlive, Sse}, IntoResponse, Response},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use futures::StreamExt;
use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};
use nextrade_services::ai::guide::{GuideEvent, GuideService};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::PublicState;

const ROUTE_TABLE: [(&str, &str, &str); 6] = [
    ("POST", "/ai/guide/conversations", "AiGuide.createConversation（我方新增）"),
    ("GET", "/ai/guide/conversations", "AiGuide.listConversations（我方新增）"),
    ("GET", "/ai/guide/conversations/{id}/messages", "AiGuide.listMessages（我方新增）"),
    ("DELETE", "/ai/guide/conversations/{id}", "AiGuide.deleteConversation（我方新增）"),
    ("POST", "/ai/guide/chat/stream", "AiGuide.chatStream（我方新增）"),
    ("POST", "/ai/guide/chat/stop", "AiGuide.chatStop（我方新增）"),
];

/// 已实现路由（覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, &format!("/api{p}"), j))
        .collect()
}

/// 需登录路由（外层 `merge_authorized` 统一套 require_auth）。
pub fn authorized_routes() -> Router<PublicState> {
    Router::new()
        .route("/ai/guide/conversations", post(create_conversation).get(list_conversations))
        .route("/ai/guide/conversations/{id}/messages", get(list_messages))
        .route("/ai/guide/conversations/{id}", delete(delete_conversation))
        .route("/ai/guide/chat/stream", post(chat_stream))
        .route("/ai/guide/chat/stop", post(chat_stop))
}

fn guide_service(state: &PublicState) -> GuideService {
    GuideService::new(
        state.repos.clone(),
        state.catalog.product.clone(),
        state.trade.order.clone(),
        state.system.recommend.clone(),
        state.system.config_client.clone(),
    )
}

fn buyer_id(auth: &AuthUser) -> Result<i32, AppError> {
    if auth.id > 0 {
        Ok(auth.id)
    } else {
        Err(AppError(Error::Validation("请先登录".to_string())))
    }
}

async fn create_conversation(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<Value> {
    let user_id = buyer_id(&auth)?;
    let svc = guide_service(&state);
    let conv_id = svc
        .create_conversation(user_id)
        .await
        .map_err(|e| AppError(Error::Validation(e)))?;
    ok(json!({ "convId": conv_id }))
}

async fn list_conversations(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<Value> {
    let user_id = buyer_id(&auth)?;
    let svc = guide_service(&state);
    ok(json!({ "list": svc.list_conversations(user_id).await }))
}

async fn list_messages(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Path(conv_id): Path<i32>,
) -> ApiResult<Value> {
    let user_id = buyer_id(&auth)?;
    let svc = guide_service(&state);
    match svc.list_messages(user_id, conv_id).await {
        Some(list) => ok(json!({ "list": list })),
        None => Err(AppError(Error::Validation("会话不存在".to_string()))),
    }
}

async fn delete_conversation(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Path(conv_id): Path<i32>,
) -> ApiResult<Value> {
    let user_id = buyer_id(&auth)?;
    let svc = guide_service(&state);
    let deleted = svc.delete_conversation(user_id, conv_id).await;
    ok(json!({ "deleted": deleted }))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ChatBody {
    pub conv_id: Option<i32>,
    pub content: String,
}

async fn chat_stream(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<ChatBody>,
) -> Result<Response, AppError> {
    let user_id = buyer_id(&auth)?;
    let content = body.content.trim().to_string();
    if content.is_empty() {
        return Err(AppError(Error::Validation("消息内容不能为空".to_string())));
    }
    if content.chars().count() > 8000 {
        return Err(AppError(Error::Validation(
            "消息内容过长（上限 8000 字）".to_string(),
        )));
    }
    let svc = guide_service(&state);
    let conv_id = match body.conv_id {
        Some(id) => {
            if svc.get_conversation(user_id, id).await.is_none() {
                return Err(AppError(Error::Validation("会话不存在".to_string())));
            }
            id
        }
        None => svc
            .create_conversation(user_id)
            .await
            .map_err(|e| AppError(Error::Validation(e)))?,
    };

    let (mut tx, rx) = futures::channel::mpsc::channel::<GuideEvent>(128);
    tokio::spawn(async move {
        svc.run_turn(user_id, conv_id, &content, &mut tx).await;
    });
    let stream = rx.map(move |ev| {
        let (name, data) = match ev {
            GuideEvent::Assistant { msg_id } => {
                ("assistant", json!({ "msgId": msg_id, "convId": conv_id }))
            }
            GuideEvent::Delta(text) => ("delta", json!({ "content": text })),
            GuideEvent::ToolStart { name } => {
                ("tool", json!({ "name": name, "status": "running" }))
            }
            GuideEvent::ToolDone { name, ok, cards } => (
                "tool",
                json!({ "name": name, "status": if ok { "done" } else { "failed" }, "cards": cards }),
            ),
            GuideEvent::Usage { prompt, completion } => {
                ("usage", json!({ "promptTokens": prompt, "completionTokens": completion }))
            }
            GuideEvent::Error(msg) => ("error", json!({ "message": msg })),
            GuideEvent::Done { stopped } => ("done", json!({ "stopped": stopped })),
        };
        Ok::<Event, std::convert::Infallible>(Event::default().event(name).data(data.to_string()))
    });
    Ok(Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response())
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StopBody {
    pub conv_id: i32,
}

async fn chat_stop(
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<StopBody>,
) -> ApiResult<Value> {
    buyer_id(&auth)?;
    nextrade_services::ai::guide::request_stop(body.conv_id);
    ok(json!({ "stopped": true }))
}
