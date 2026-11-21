//! 站点搬家对外路由（site_clone 上下文）。
//!
//! 来源标注：**我方新增**（设计 `~/workspace/site-clone-design/REPORT.md`
//! 第五节）。挂载于 `/api` 前缀下（`routes::merge_public` 的 anonymous
//! 通道，外层不套会员 JWT），鉴权走搬家密钥双请求头
//! `X-Clone-Key-Id` + `X-Clone-Secret`；全部端点只读。

pub mod clone;

use axum::Router;

use super::super::PublicState;

/// 匿名可访问路由（自带密钥鉴权）。
pub fn anonymous_routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new().merge(clone::routes(state))
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    clone::implemented_routes()
}
