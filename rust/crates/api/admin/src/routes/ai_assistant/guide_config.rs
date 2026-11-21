//! 管理端 AI 导购配置（我方新增）：平台 AI 设置下导购「面」的独立
//! 配置区（与运营助手共用同一 AI 底座，配置按面分开）。
//!
//! 只暴露导购引擎真实消费的四个旋钮（`config.aiGuideSettings` JSON，
//! 引擎 `services::ai::guide` 每回合现读、无缓存）：总开关 `enabled` +
//! 三级 token 预算 `sessionTokens`（单会话）/ `userDailyTokens`（单用户
//! 日）/ `globalDailyTokens`（全局日）。引擎不消费的字段不在此发明：
//! - 欢迎语：引擎无下发机制（建会话不预置任何助手消息），不做；
//! - 渠道/模型：引擎恒走平台默认渠道解析（`resolve_channel_model`
//!   平台作用域 + 默认渠道），不按面指定，不做——在「渠道配置」里设
//!   平台默认渠道即对导购生效。
//!
//! 权限：导购是平台面（渠道恒平台作用域、预算是平台成本），整域挂
//! `platform_only`，与联邦设置同口径，非平台账号 403。
//! 生效时机：保存落行（config 表先删后插）后，买家下一回合即按新值
//! 执行，无需重启。

use axum::{
    extract::State,
    routing::get,
    Json, Router,
};
use serde_json::Value;

use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, AppError};
use nextrade_services::ai::guide::{
    load_guide_settings, store_guide_settings, validate_guide_settings,
};

use crate::support::tenant::platform_only;
use crate::AdminState;

/// 本域路由（`/adminapi` 前缀之后）：导购设置读/写。
pub fn routes() -> Router<AdminState> {
    Router::new()
        .route("/ai/guide/settings", get(get_settings).post(save_settings))
        .layer(axum::middleware::from_fn(platform_only))
}

/// `GET /adminapi/ai/guide/settings`：读取生效中的导购设置
/// （config 无行时返回引擎内置默认：开 + 60000 / 200000 / 5000000）。
async fn get_settings(State(state): State<AdminState>) -> ApiResult<Value> {
    let settings = load_guide_settings(&state.repos).await;
    ok(settings.to_value())
}

/// `POST /adminapi/ai/guide/settings`：保存导购设置（严格校验后整块
/// 落行；非法值以 Validation 错误明确指明字段）。
async fn save_settings(
    State(state): State<AdminState>,
    Json(payload): Json<Value>,
) -> ApiResult<Value> {
    let settings =
        validate_guide_settings(&payload).map_err(|e| AppError(Error::Validation(e)))?;
    store_guide_settings(&state.repos, &settings)
        .await
        .map_err(|e| AppError(Error::Database(e)))?;
    ok(settings.to_value())
}
