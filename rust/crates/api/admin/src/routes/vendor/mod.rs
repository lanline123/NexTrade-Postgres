//! 管理端 vendor 路由（挂载于 `/adminapi`）。

pub mod lowfreq;
pub mod vendor;
pub mod vendor_product;

use axum::Router;

use super::super::AdminState;

/// 需登录的域路由（外层统一套 require_auth）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .merge(lowfreq::routes(state))
        .merge(vendor::routes(state))
        .merge(vendor_product::routes(state))
}

/// 免登录的域路由（本域无）。
pub fn public_routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    let mut out = Vec::new();
    out.extend(vendor::implemented_routes());
    out.extend(vendor_product::implemented_routes());
    out
}
