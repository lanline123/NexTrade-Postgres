//! admin AI 助手（我方新增，DESIGN.md）：对话页 + 渠道/密钥 + 用量。
//!
//! - 工具层进程内复用 `ai_mcp::call_tool`，注入登录会话的 `ToolCtx`（租户作用域）。
//! - 隔离红线（DESIGN §3）：作用域只来自登录身份；商家会话工具注册裁剪、
//!   分发强制注入、实体零泄露校验、跨店尝试审计，四层缺一不可。
//! - 密钥只走 AES-256-GCM 加密库（vault），任何读接口只回打码。

pub mod channels;
pub mod chat;
pub mod guide_config;
pub mod usage;
/// 密钥库已下沉 services 共享层（2026-11-21 导购立项），此处再导出保持路径不变。
pub use nextrade_services::ai::vault;

use axum::{routing::{delete, get, post, put}, Router};
use nextrade_core::Error;
use nextrade_security::web::{AppError, AuthUser};

use crate::AdminState;

pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn err_validation(msg: impl Into<String>) -> AppError {
    AppError(Error::Validation(msg.into()))
}

pub fn err_forbidden(msg: impl Into<String>) -> AppError {
    AppError(Error::Forbidden(msg.into()))
}

/// AI 功能的租户上下文：作用域的唯一可信源 = 登录身份。
#[derive(Debug, Clone)]
pub struct AiTenant {
    /// 0 = 平台；>0 = 商家本店。
    pub shop_id: i32,
    pub admin_id: i32,
    pub username: String,
}

impl AiTenant {
    pub fn from_auth(auth: &AuthUser) -> Result<Self, AppError> {
        if auth.is_platform() {
            return Ok(Self {
                shop_id: 0,
                admin_id: auth.id,
                username: auth.username.clone(),
            });
        }
        // 供应商账号无店铺维度，且不是平台管理员：明确拒绝，避免落入平台口径。
        if auth.is_vendor() {
            return Err(err_forbidden("供应商账号不支持 AI 助手"));
        }
        match auth.shop_id.filter(|id| *id > 0) {
            Some(shop_id) => Ok(Self {
                shop_id,
                admin_id: auth.id,
                username: auth.username.clone(),
            }),
            None => Err(err_forbidden("当前账号未绑定店铺，无法使用 AI 助手")),
        }
    }

    pub fn is_merchant(&self) -> bool {
        self.shop_id > 0
    }

    pub fn tool_ctx(&self, conv_id: i32) -> crate::routes::ai_mcp::ToolCtx {
        crate::routes::ai_mcp::ToolCtx {
            source: crate::routes::ai_mcp::ToolSource::Chat,
            actor_id: self.admin_id,
            actor_name: self.username.clone(),
            tenant_shop_id: if self.shop_id > 0 {
                Some(self.shop_id)
            } else {
                None
            },
            conv_id,
        }
    }

    /// 是否为本店店主（admin_user.parent_id 为空/0 的店铺主账号）。
    pub async fn is_shop_owner(&self, pool: &sqlx::PgPool) -> bool {
        if !self.is_merchant() {
            return false;
        }
        let row: Option<(Option<i32>,)> =
            sqlx::query_as("SELECT parent_id FROM admin_user WHERE admin_id = $1")
                .bind(self.admin_id)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten();
        matches!(row, Some((None,)) | Some((Some(0),)))
    }
}

pub fn routes(_state: &AdminState) -> Router<AdminState> {
    Router::new()
        // 渠道与密钥
        .route("/ai/channels/presets", get(channels::presets))
        .route("/ai/channels", get(channels::list).post(channels::create))
        .route(
            "/ai/channels/{id}",
            put(channels::update).delete(channels::remove),
        )
        .route("/ai/channels/{id}/default", post(channels::set_default))
        .route("/ai/channels/{id}/test", post(channels::test_connection))
        .route(
            "/ai/channels/{id}/models",
            get(channels::list_models).post(channels::create_model),
        )
        .route(
            "/ai/channels/{id}/models/{model_id}",
            put(channels::update_model).delete(channels::delete_model),
        )
        // 会话与对话
        .route(
            "/ai/conversations",
            get(chat::list_conversations).post(chat::create_conversation),
        )
        .route(
            "/ai/conversations/{id}/messages",
            get(chat::list_messages),
        )
        .route("/ai/conversations/{id}/rename", post(chat::rename_conversation))
        .route(
            "/ai/conversations/{id}",
            delete(chat::delete_conversation),
        )
        .route("/ai/conversations/{id}/model", post(chat::switch_model))
        .route("/ai/chat/stream", post(chat::chat_stream))
        .route("/ai/chat/confirm", post(chat::chat_confirm))
        .route("/ai/chat/stop", post(chat::chat_stop))
        // 用量
        .route("/ai/usage", get(usage::list_usage))
        // 导购配置（平台面独立配置区，platform_only 在子路由内自带）
        .merge(guide_config::routes())
}
