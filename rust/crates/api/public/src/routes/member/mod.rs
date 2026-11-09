//! 商城端 member 路由。

pub mod captcha_api;
pub mod collect_product;
pub mod login;
pub mod member_lists;
pub mod oauth;
pub mod user_address_client;
pub mod user_center;
pub mod user_company_client;
pub mod user_feedback_client;
pub mod user_invoice_client;
pub mod user_message;
pub mod user_price_inquiry;
pub mod user_sign;

use axum::Router;

use super::super::PublicState;

/// 自带鉴权中间件的域路由（并入 merge_public，外层不再套 require_auth）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .merge(collect_product::routes(state))
        .merge(login::routes(state))
        .merge(user_address_client::routes(state))
        .merge(user_center::routes(state))
        .merge(user_company_client::routes(state))
        .merge(user_feedback_client::routes(state))
        .merge(user_invoice_client::routes(state))
        .merge(user_message::routes(state))
        .merge(user_price_inquiry::routes(state))
        .merge(user_sign::routes(state))
}

/// 匿名可访问路由（无状态）。
pub fn public_routes() -> Router<PublicState> {
    Router::new()
        .merge(member_lists::public_routes())
}

/// 匿名可访问路由（需要 state，外层不套 require_auth）。
pub fn anonymous_routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .merge(captcha_api::anonymous_routes(state))
        .merge(login::anonymous_routes(state))
        .merge(oauth::anonymous_routes(state))
}

/// 需登录路由（外层统一套 require_auth）。
pub fn authorized_routes() -> Router<PublicState> {
    Router::new()
        .merge(member_lists::authorized_routes())
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    let mut out = Vec::new();
    out.extend(captcha_api::implemented_routes());
    out.extend(collect_product::implemented_routes());
    out.extend(login::implemented_routes());
    out.extend(member_lists::implemented_routes());
    out.extend(oauth::implemented_routes());
    out.extend(user_address_client::implemented_routes());
    out.extend(user_center::implemented_routes());
    out.extend(user_company_client::implemented_routes());
    out.extend(user_feedback_client::implemented_routes());
    out.extend(user_invoice_client::implemented_routes());
    out.extend(user_message::implemented_routes());
    out.extend(user_price_inquiry::implemented_routes());
    out.extend(user_sign::implemented_routes());
    out
}
