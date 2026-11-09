//! 根级路由（不带模块前缀），按限界上下文分组。

pub mod catalog;
pub mod im;

use axum::Router;

use super::PublicState;

/// 需登录的根级路由。
pub fn merge_protected(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .merge(catalog::merge_protected(state))
        .merge(im::merge_protected(state))
}

/// 免登录的根级路由。
pub fn merge_public(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .merge(catalog::merge_public(state))
        .merge(im::merge_public(state))
}

/// 汇总所有根级已实现路由。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    let mut out = Vec::new();
    out.extend(catalog::implemented_routes());
    out.extend(im::implemented_routes());
    out
}
