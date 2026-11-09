//! 管理端 system 路由。

pub mod admin_msg;
pub mod ai_image;
pub mod app_version_admin;
pub mod area_code;
pub mod cache_manage;
pub mod config_basic;
pub mod config_extra;
pub mod csrf;
pub mod example;
pub mod image_search;
pub mod licensed;
pub mod lowfreq_extra;
pub mod mail_templates;
pub mod message_type;
pub mod recall_extra;
pub mod region;
pub mod tips_manage;
pub mod verification;

use axum::Router;

use super::super::AdminState;

/// 需登录的域路由（外层统一套 require_auth）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .merge(admin_msg::routes(state))
        .merge(ai_image::routes(state))
        .merge(app_version_admin::routes(state))
        .merge(area_code::routes(state))
        .merge(cache_manage::routes(state))
        .merge(config_basic::routes(state))
        .merge(config_extra::routes(state))
        .merge(example::routes(state))
        .merge(image_search::routes(state))
        .merge(licensed::routes(state))
        .merge(lowfreq_extra::routes(state))
        .merge(mail_templates::routes(state))
        .merge(message_type::routes(state))
        .merge(recall_extra::routes(state))
        .merge(region::routes(state))
        .merge(tips_manage::routes(state))
}

/// 免登录的域路由（对应 Java `URL_WHITELIST`，外层不套 require_auth）。
pub fn public_routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .merge(config_basic::public_routes(state))
        .merge(csrf::public_routes(state))
        .merge(verification::public_routes(state))
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    let mut out = Vec::new();
    out.extend(admin_msg::implemented_routes());
    out.extend(ai_image::implemented_routes());
    out.extend(app_version_admin::implemented_routes());
    out.extend(area_code::implemented_routes());
    out.extend(cache_manage::implemented_routes());
    out.extend(config_basic::implemented_routes());
    out.extend(config_extra::implemented_routes());
    out.extend(csrf::implemented_routes());
    out.extend(example::implemented_routes());
    out.extend(image_search::implemented_routes());
    out.extend(licensed::implemented_routes());
    out.extend(mail_templates::implemented_routes());
    out.extend(message_type::implemented_routes());
    out.extend(region::implemented_routes());
    out.extend(tips_manage::implemented_routes());
    out
}
