//! 外卖聚合对外路由（takeout 上下文，我方新增）。
//!
//! 挂载于 `/api` 前缀下（`routes::merge_public` 的 anonymous 通道，
//! 外层不套会员 JWT）；鉴权走渠道签名（适配器内验签），未配置/验签
//! 失败一律诚实拒绝并落回调日志，不吞单不假成功。

pub mod callback;

use axum::Router;

use super::super::PublicState;

/// 匿名可访问路由（渠道回调，自带签名鉴权）。
pub fn anonymous_routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new().merge(callback::routes(state))
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    callback::implemented_routes()
}
