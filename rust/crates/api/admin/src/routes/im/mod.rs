//! 管理端 im 路由。

pub mod admin_shop_kefu_config;
pub mod admin_user_kefu;

use axum::Router;

use super::super::AdminState;
use crate::routes::marketing::tenant_layer::tenant_scope_layer;

/// 需登录的域路由（外层统一套 require_auth）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .merge(admin_shop_kefu_config::routes(state))
        .merge(admin_user_kefu::routes(state))
        .layer(axum::middleware::from_fn(tenant_scope_layer))
}

/// 免登录的域路由（对应 Java `URL_WHITELIST`，外层不套 require_auth）。
pub fn public_routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    let mut out = Vec::new();
    out.extend(admin_shop_kefu_config::implemented_routes());
    out.extend(admin_user_kefu::implemented_routes());
    out
}
