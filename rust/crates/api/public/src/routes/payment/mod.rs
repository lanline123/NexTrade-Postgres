//! 商城端 payment 路由。

pub mod apple_pay;
pub mod balance_pay;
pub mod pay;
pub mod paylog;
pub mod recharge_order;
pub mod with_draw_apply_client;

use axum::Router;

use super::super::PublicState;

/// 自带鉴权中间件的域路由（并入 merge_public，外层不再套 require_auth）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .merge(apple_pay::routes(state))
        .merge(pay::routes(state))
        .merge(recharge_order::routes(state))
        .merge(with_draw_apply_client::routes(state))
}

/// 匿名可访问路由（无状态）。
pub fn public_routes() -> Router<PublicState> {
    Router::new()
        .merge(paylog::public_routes())
}

/// 匿名可访问路由（需要 state，外层不套 require_auth）。
pub fn anonymous_routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
}

/// 需登录路由（外层统一套 require_auth）。
pub fn authorized_routes() -> Router<PublicState> {
    Router::new()
        .merge(balance_pay::authorized_routes())
        .merge(paylog::authorized_routes())
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    let mut out = Vec::new();
    out.extend(apple_pay::implemented_routes());
    out.extend(balance_pay::implemented_routes());
    out.extend(pay::implemented_routes());
    out.extend(paylog::implemented_routes());
    out.extend(recharge_order::implemented_routes());
    out.extend(with_draw_apply_client::implemented_routes());
    out
}
