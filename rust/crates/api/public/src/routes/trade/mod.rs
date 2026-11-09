//! 商城端 trade 路由。

pub mod aftersales_client;
pub mod cart_summary;
pub mod check;
pub mod cycle_card;
pub mod offline_claim;
pub mod order_invoice_client;
pub mod user_order;

use axum::Router;

use super::super::PublicState;

/// 自带鉴权中间件的域路由（并入 merge_public，外层不再套 require_auth）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .merge(aftersales_client::routes(state))
        .merge(cart_summary::routes(state))
        .merge(check::routes(state))
        .merge(cycle_card::routes(state))
        .merge(offline_claim::routes(state))
        .merge(order_invoice_client::routes(state))
        .merge(user_order::routes(state))
}

/// 匿名可访问路由（无状态）。
pub fn public_routes() -> Router<PublicState> {
    Router::new()
}

/// 匿名可访问路由（需要 state，外层不套 require_auth）。
pub fn anonymous_routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
}

/// 需登录路由（外层统一套 require_auth）。
pub fn authorized_routes() -> Router<PublicState> {
    Router::new()
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    let mut out = Vec::new();
    out.extend(aftersales_client::implemented_routes());
    out.extend(cart_summary::implemented_routes());
    out.extend(check::implemented_routes());
    out.extend(cycle_card::implemented_routes());
    out.extend(offline_claim::implemented_routes());
    out.extend(order_invoice_client::implemented_routes());
    out.extend(user_order::implemented_routes());
    out
}
