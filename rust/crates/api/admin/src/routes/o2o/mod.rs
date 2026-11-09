//! 管理端 o2o 路由（挂载于 `/adminapi`，本地生活/到店）。

pub mod home_service;
pub mod local_delivery;
pub mod reservation_tpl;
pub mod shop_pickup;
pub mod stock_calendar;
pub mod store_product;

use axum::Router;

use super::super::AdminState;

/// 需登录的域路由（外层统一套 require_auth）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .merge(home_service::routes(state))
        .merge(local_delivery::routes(state))
        .merge(reservation_tpl::routes(state))
        .merge(shop_pickup::routes(state))
        .merge(stock_calendar::routes(state))
        .merge(store_product::routes(state))
}

/// 免登录的域路由（本域无）。
pub fn public_routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    let mut out = Vec::new();
    out.extend(home_service::implemented_routes());
    out.extend(local_delivery::implemented_routes());
    out.extend(reservation_tpl::implemented_routes());
    out.extend(shop_pickup::implemented_routes());
    out.extend(stock_calendar::implemented_routes());
    out.extend(store_product::implemented_routes());
    out
}
