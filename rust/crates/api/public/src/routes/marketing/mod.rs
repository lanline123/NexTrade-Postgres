//! 商城端 marketing 路由。

pub mod buyer_show_client;
pub mod coupon;
pub mod gift_card_groups;
pub mod groupon_promotion;
pub mod groupon_records;
pub mod lottery;
pub mod product_style_admin;
pub mod salesman_admin;
pub mod salesman_content;
pub mod salesman_material;
pub mod salesman_order;
pub mod salesman_product;
pub mod seckill;
pub mod time_discount;

use axum::Router;

use super::super::PublicState;

/// 自带鉴权中间件的域路由（并入 merge_public，外层不再套 require_auth）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .merge(buyer_show_client::routes(state))
        .merge(gift_card_groups::routes(state))
        .merge(groupon_promotion::routes(state))
        .merge(groupon_records::routes(state))
        .merge(lottery::routes(state))
        .merge(product_style_admin::routes(state))
        .merge(salesman_admin::routes(state))
        .merge(salesman_content::routes(state))
        .merge(salesman_material::routes(state))
        .merge(salesman_order::routes(state))
        .merge(salesman_product::routes(state))
}

/// 匿名可访问路由（无状态）。
pub fn public_routes() -> Router<PublicState> {
    Router::new()
        .merge(coupon::public_routes())
        .merge(seckill::public_routes())
        .merge(time_discount::public_routes())
}

/// 匿名可访问路由（需要 state，外层不套 require_auth）。
pub fn anonymous_routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
}

/// 需登录路由（外层统一套 require_auth）。
pub fn authorized_routes() -> Router<PublicState> {
    Router::new()
        .merge(coupon::authorized_routes())
        .merge(seckill::authorized_routes())
        .merge(time_discount::authorized_routes())
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    let mut out = Vec::new();
    out.extend(buyer_show_client::implemented_routes());
    out.extend(coupon::implemented_routes());
    out.extend(gift_card_groups::implemented_routes());
    out.extend(groupon_promotion::implemented_routes());
    out.extend(groupon_records::implemented_routes());
    out.extend(lottery::implemented_routes());
    out.extend(product_style_admin::implemented_routes());
    out.extend(salesman_admin::implemented_routes());
    out.extend(salesman_content::implemented_routes());
    out.extend(salesman_material::implemented_routes());
    out.extend(salesman_order::implemented_routes());
    out.extend(salesman_product::implemented_routes());
    out.extend(seckill::implemented_routes());
    out.extend(time_discount::implemented_routes());
    out
}
