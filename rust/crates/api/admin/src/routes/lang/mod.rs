//! 多语言（lang）路由域。

pub mod currency;
pub mod locales;
pub mod locales_relation;
pub mod translations;

use axum::Router;

use super::super::AdminState;

/// 需登录的域路由（外层统一套 require_auth）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .merge(translations::routes(state))
        .merge(locales::routes(state))
        .merge(currency::routes(state))
        .merge(locales_relation::routes(state))
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    let mut out = Vec::new();
    out.extend(translations::implemented_routes());
    out.extend(locales::implemented_routes());
    out.extend(currency::implemented_routes());
    out.extend(locales_relation::implemented_routes());
    out
}
