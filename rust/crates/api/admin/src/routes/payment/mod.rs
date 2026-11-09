//! 管理端 payment 路由。

pub mod account_panel;
pub mod apple_pay_config;
pub mod category_fee_rule;
pub mod credit_order;
pub mod credit_settlement;
pub mod credit_user;
pub mod huifu_config;
pub mod lakala_config;
pub mod merchant_application;
pub mod order_invoice;
pub mod paylog;
pub mod paylog_batch;
pub mod profit_sharing;
pub mod refund_apply;
pub mod refund_apply_admin;
pub mod refund_log;
pub mod statement;
pub mod sub_merchant;
pub mod tax_rate;
pub mod user_balance_log;
pub mod user_invoice;
pub mod user_recharge_order;
pub mod user_withdraw_apply;
pub mod yeepay_config;

use axum::Router;

use super::super::AdminState;

/// 需登录的域路由（外层统一套 require_auth）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .merge(account_panel::routes(state))
        .merge(apple_pay_config::routes(state))
        .merge(category_fee_rule::routes(state))
        .merge(credit_order::routes(state))
        .merge(credit_settlement::routes(state))
        .merge(credit_user::routes(state))
        .merge(huifu_config::routes(state))
        .merge(lakala_config::routes(state))
        .merge(merchant_application::routes(state))
        .merge(order_invoice::routes(state))
        .merge(paylog::routes(state))
        .merge(paylog_batch::routes(state))
        .merge(profit_sharing::routes(state))
        .merge(refund_apply::routes(state))
        .merge(refund_apply_admin::routes(state))
        .merge(refund_log::routes(state))
        .merge(statement::routes(state))
        .merge(sub_merchant::routes(state))
        .merge(tax_rate::routes(state))
        .merge(user_balance_log::routes(state))
        .merge(user_invoice::routes(state))
        .merge(user_recharge_order::routes(state))
        .merge(user_withdraw_apply::routes(state))
        .merge(yeepay_config::routes(state))
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
    out.extend(account_panel::implemented_routes());
    out.extend(apple_pay_config::implemented_routes());
    out.extend(category_fee_rule::implemented_routes());
    out.extend(credit_order::implemented_routes());
    out.extend(credit_settlement::implemented_routes());
    out.extend(credit_user::implemented_routes());
    out.extend(huifu_config::implemented_routes());
    out.extend(lakala_config::implemented_routes());
    out.extend(merchant_application::implemented_routes());
    out.extend(order_invoice::implemented_routes());
    out.extend(paylog::implemented_routes());
    out.extend(paylog_batch::implemented_routes());
    out.extend(profit_sharing::implemented_routes());
    out.extend(refund_apply::implemented_routes());
    out.extend(refund_apply_admin::implemented_routes());
    out.extend(refund_log::implemented_routes());
    out.extend(statement::implemented_routes());
    out.extend(sub_merchant::implemented_routes());
    out.extend(tax_rate::implemented_routes());
    out.extend(user_balance_log::implemented_routes());
    out.extend(user_invoice::implemented_routes());
    out.extend(user_recharge_order::implemented_routes());
    out.extend(user_withdraw_apply::implemented_routes());
    out.extend(yeepay_config::implemented_routes());
    out
}
