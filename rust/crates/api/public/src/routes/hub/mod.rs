//! 平台联邦面对外路由（hub 上下文）。
//!
//! 来源标注：**我方新增**（设计 `~/workspace/hub-federation-design/proposal.md`
//! §3/§4/§5/§8）。挂载于 `/api` 前缀下（`routes::merge_public` 的 anonymous
//! 通道，外层不套会员 JWT），鉴权走联邦凭证双请求头
//! `X-Fed-Access-Key` + `X-Fed-Secret`；配对三件套以 claim token 为凭据。
//! 总开关关闭时全部接口 404（服务层 gate）。

pub mod federation;

use axum::Router;

use super::super::PublicState;

/// 匿名可访问路由（自带凭证鉴权）。
pub fn anonymous_routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new().merge(federation::routes(state))
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    federation::implemented_routes()
}
