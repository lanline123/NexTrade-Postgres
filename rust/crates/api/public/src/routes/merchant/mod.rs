//! 商城端 merchant 路由。

pub mod merchant_client;
pub mod shop_account_log;
pub mod shop_client;
pub mod shop_client_category;

use axum::Router;

use super::super::PublicState;

/// 自带鉴权中间件的域路由（并入 merge_public，外层不再套 require_auth）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .merge(merchant_client::routes(state))
        .merge(shop_account_log::routes(state))
        .merge(shop_client::routes(state))
        .merge(shop_client_category::routes(state))
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
    out.extend(merchant_client::implemented_routes());
    out.extend(shop_account_log::implemented_routes());
    out.extend(shop_client::implemented_routes());
    out.extend(shop_client_category::implemented_routes());
    out
}
