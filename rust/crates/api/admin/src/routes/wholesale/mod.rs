//! 管理端 wholesale 路由。

pub mod wholesale_product;
pub mod wholesale_setting;
pub mod wholesaler;
pub mod wholesaler_grade;

use axum::Router;

use super::super::AdminState;

/// 需登录的域路由（外层统一套 require_auth）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .merge(wholesale_product::routes(state))
        .merge(wholesale_setting::routes(state))
        .merge(wholesaler::routes(state))
        .merge(wholesaler_grade::routes(state))
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
    out.extend(wholesale_product::implemented_routes());
    out.extend(wholesale_setting::implemented_routes());
    out.extend(wholesaler::implemented_routes());
    out.extend(wholesaler_grade::implemented_routes());
    out
}
