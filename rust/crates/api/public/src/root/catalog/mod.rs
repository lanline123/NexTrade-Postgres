//! catalog 根级路由（不带模块前缀）。

pub mod short_url;

use axum::Router;

use super::super::PublicState;

/// 需登录的根级路由。
pub fn merge_protected(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()

}

/// 免登录的根级路由。
pub fn merge_public(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .merge(short_url::routes(state))

}

/// 汇总本域根级已实现路由。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    let mut out = Vec::new();
    out.extend(short_url::implemented_routes());
    out
}
