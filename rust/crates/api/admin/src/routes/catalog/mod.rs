//! 管理端 catalog 路由。

pub mod attributes_tpl;
pub mod brand;
pub mod category;
pub mod category_all;
pub mod comment;
pub mod e_card;
pub mod e_card_group;
pub mod price_inquiry;
pub mod product_batch;
pub mod product_group;
pub mod product_inventory_log;
pub mod product_manage;
pub mod product_manage_client;
pub mod product_manage_extra;
pub mod product_stock;
pub mod search_admin;
pub mod services;
pub mod stock_warning;

pub(crate) mod tenant;

use axum::Router;

use super::super::AdminState;

/// 需登录的域路由（外层统一套 require_auth）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .merge(attributes_tpl::routes(state))
        .merge(brand::routes(state))
        .merge(category::routes(state))
        .merge(category_all::routes(state))
        .merge(comment::routes(state))
        .merge(e_card::routes(state))
        .merge(e_card_group::routes(state))
        .merge(price_inquiry::routes(state))
        .merge(product_batch::routes(state))
        .merge(product_group::routes(state))
        .merge(product_inventory_log::routes(state))
        .merge(product_manage::routes(state))
        .merge(product_manage_client::routes(state))
        .merge(product_manage_extra::routes(state))
        .merge(product_stock::routes(state))
        .merge(search_admin::routes(state))
        .merge(services::routes(state))
        .merge(stock_warning::routes(state))
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
    out.extend(attributes_tpl::implemented_routes());
    out.extend(brand::implemented_routes());
    out.extend(category::implemented_routes());
    out.extend(category_all::implemented_routes());
    out.extend(comment::implemented_routes());
    out.extend(e_card::implemented_routes());
    out.extend(e_card_group::implemented_routes());
    out.extend(price_inquiry::implemented_routes());
    out.extend(product_batch::implemented_routes());
    out.extend(product_group::implemented_routes());
    out.extend(product_inventory_log::implemented_routes());
    out.extend(product_manage::implemented_routes());
    out.extend(product_manage_client::implemented_routes());
    out.extend(product_manage_extra::implemented_routes());
    out.extend(product_stock::implemented_routes());
    out.extend(search_admin::implemented_routes());
    out.extend(services::implemented_routes());
    out.extend(stock_warning::implemented_routes());
    out
}
