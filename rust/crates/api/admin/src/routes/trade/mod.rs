//! 管理端 trade 路由。

pub mod aftersales;
pub mod offline_claim;
pub mod order_admin;
pub mod order_log;
pub mod proxy_shopping_cart;
pub mod proxy_shopping_delivery;
pub mod proxy_shopping_order_check;
pub mod proxy_shopping_pickup_contact;
pub mod proxy_shopping_product;
pub mod proxy_shopping_user;
pub mod proxy_shopping_user_address;
pub mod proxy_shopping_user_coupon;
pub mod shop_order_config;

use axum::Router;

use super::super::AdminState;

/// 需登录的域路由（外层统一套 require_auth）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .merge(aftersales::routes(state))
        .merge(offline_claim::routes(state))
        .merge(order_admin::routes(state))
        .merge(order_log::routes(state))
        .merge(proxy_shopping_cart::routes(state))
        .merge(proxy_shopping_delivery::routes(state))
        .merge(proxy_shopping_order_check::routes(state))
        .merge(proxy_shopping_pickup_contact::routes(state))
        .merge(proxy_shopping_product::routes(state))
        .merge(proxy_shopping_user::routes(state))
        .merge(proxy_shopping_user_address::routes(state))
        .merge(proxy_shopping_user_coupon::routes(state))
        .merge(shop_order_config::routes(state))
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
    out.extend(aftersales::implemented_routes());
    out.extend(offline_claim::implemented_routes());
    out.extend(order_admin::implemented_routes());
    out.extend(order_log::implemented_routes());
    out.extend(proxy_shopping_cart::implemented_routes());
    out.extend(proxy_shopping_delivery::implemented_routes());
    out.extend(proxy_shopping_order_check::implemented_routes());
    out.extend(proxy_shopping_pickup_contact::implemented_routes());
    out.extend(proxy_shopping_product::implemented_routes());
    out.extend(proxy_shopping_user::implemented_routes());
    out.extend(proxy_shopping_user_address::implemented_routes());
    out.extend(proxy_shopping_user_coupon::implemented_routes());
    out.extend(shop_order_config::implemented_routes());
    out
}
