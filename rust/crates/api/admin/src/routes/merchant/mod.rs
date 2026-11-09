//! 管理端 merchant 路由。

pub mod admin_user_shop;
pub mod deposit_account;
pub mod deposit_log;
pub mod deposit_payee_account;
pub mod deposit_recharge;
pub mod deposit_refund;
pub mod deposit_rule;
pub mod deposit_setting;
pub mod deposit_shop;
pub mod merchant;
pub mod merchant_account;
pub mod merchant_apply;
pub mod order_config;
pub mod shop;
pub mod shop_account_log;
pub mod shop_funds;
pub mod shop_manage;
pub mod shop_product_category;
pub mod shop_withdraw;
pub mod vendor_deposit_shop;

use axum::Router;

use super::super::AdminState;

/// 需登录的域路由（外层统一套 require_auth）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .merge(admin_user_shop::routes(state))
        .merge(deposit_account::routes(state))
        .merge(deposit_log::routes(state))
        .merge(deposit_payee_account::routes(state))
        .merge(deposit_recharge::routes(state))
        .merge(deposit_refund::routes(state))
        .merge(deposit_rule::routes(state))
        .merge(deposit_setting::routes(state))
        .merge(deposit_shop::routes(state))
        .merge(merchant::routes(state))
        .merge(merchant_account::routes(state))
        .merge(merchant_apply::routes(state))
        .merge(order_config::routes(state))
        .merge(shop::routes(state))
        .merge(shop_account_log::routes(state))
        .merge(shop_funds::routes(state))
        .merge(shop_manage::routes(state))
        .merge(shop_product_category::routes(state))
        .merge(shop_withdraw::routes(state))
        .merge(vendor_deposit_shop::routes(state))
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
    out.extend(admin_user_shop::implemented_routes());
    out.extend(deposit_account::implemented_routes());
    out.extend(deposit_log::implemented_routes());
    out.extend(deposit_payee_account::implemented_routes());
    out.extend(deposit_recharge::implemented_routes());
    out.extend(deposit_refund::implemented_routes());
    out.extend(deposit_rule::implemented_routes());
    out.extend(deposit_setting::implemented_routes());
    out.extend(deposit_shop::implemented_routes());
    out.extend(merchant::implemented_routes());
    out.extend(merchant_account::implemented_routes());
    out.extend(merchant_apply::implemented_routes());
    out.extend(order_config::implemented_routes());
    out.extend(shop::implemented_routes());
    out.extend(shop_account_log::implemented_routes());
    out.extend(shop_funds::implemented_routes());
    out.extend(shop_manage::implemented_routes());
    out.extend(shop_product_category::implemented_routes());
    out.extend(shop_withdraw::implemented_routes());
    out.extend(vendor_deposit_shop::implemented_routes());
    out
}
