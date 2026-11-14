//! 商城端 system 路由。

pub mod app_version_admin;
pub mod config_client;
pub mod price_display_client;
pub mod csrf;
pub mod locales_client;
pub mod log;
pub mod pc;
pub mod recommend;
pub mod region_client;
pub mod sms_client;
pub mod translate_client;
pub mod util;

use axum::Router;

use super::super::PublicState;

/// 自带鉴权中间件的域路由（并入 merge_public，外层不再套 require_auth）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .merge(app_version_admin::routes(state))
        .merge(config_client::routes(state))
        .merge(price_display_client::routes(state))
        .merge(locales_client::routes(state))
        .merge(pc::routes(state))
        .merge(recommend::routes(state))
        .merge(region_client::routes(state))
        .merge(sms_client::routes(state))
        .merge(translate_client::routes(state))
        .merge(util::routes(state))
}

/// 匿名可访问路由（无状态）。
pub fn public_routes() -> Router<PublicState> {
    Router::new()
}

/// 匿名可访问路由（需要 state，外层不套 require_auth）。
pub fn anonymous_routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .merge(csrf::anonymous_routes(state))
        .merge(log::anonymous_routes(state))
}

/// 需登录路由（外层统一套 require_auth）。
pub fn authorized_routes() -> Router<PublicState> {
    Router::new()
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    let mut out = Vec::new();
    out.extend(app_version_admin::implemented_routes());
    out.extend(config_client::implemented_routes());
    out.extend(price_display_client::implemented_routes());
    out.extend(locales_client::implemented_routes());
    out.extend(csrf::implemented_routes());
    out.extend(log::implemented_routes());
    out.extend(pc::implemented_routes());
    out.extend(recommend::implemented_routes());
    out.extend(region_client::implemented_routes());
    out.extend(sms_client::implemented_routes());
    out.extend(translate_client::implemented_routes());
    out.extend(util::implemented_routes());
    out
}
