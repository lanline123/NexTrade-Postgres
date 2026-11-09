//! 管理端 pos 路由（挂载于 `/adminapi`）。

pub mod cashier;
pub mod device;

use axum::Router;

use super::super::AdminState;

/// 需登录的域路由（外层统一套 require_auth）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .merge(cashier::routes(state))
        .merge(device::routes(state))
}

/// 免登录的域路由（本域无）。
pub fn public_routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    let mut out = cashier::implemented_routes();
    out.extend(device::implemented_routes());
    out
}
