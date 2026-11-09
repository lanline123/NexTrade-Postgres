//! 管理端 identity 路由。

pub mod admin_account;
pub mod admin_log;
pub mod admin_role;
pub mod admin_user;
pub mod authority;

use axum::Router;

use super::super::AdminState;

/// 需登录的域路由（外层统一套 require_auth）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .merge(admin_account::routes(state))
        .merge(admin_log::routes(state))
        .merge(admin_role::routes(state))
        .merge(admin_user::routes(state))
        .merge(authority::routes(state))
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
    out.extend(admin_account::implemented_routes());
    out.extend(admin_log::implemented_routes());
    out.extend(admin_role::implemented_routes());
    out.extend(admin_user::implemented_routes());
    out.extend(authority::implemented_routes());
    out
}
