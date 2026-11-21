//! 平台联邦面对外路由（hub 上下文）。
//!
//! 来源标注：**我方新增**（设计 `~/workspace/hub-federation-design/proposal.md`
//! §3/§4/§5/§8；三期 `~/workspace/federation-phase3/DESIGN.md`）。
//! 挂载于 `/api` 前缀下（`routes::merge_public`）：联邦面走 anonymous 通道
//! （外层不套会员 JWT），鉴权走联邦凭证双请求头
//! `X-Fed-Access-Key` + `X-Fed-Secret`；配对三件套以 claim token 为凭据。
//! 登录互认（三期①）的中枢同意面是例外：会员本人持本地 JWT 操作，
//! 经 `routes()` 的 require_auth 通道挂载。总开关关闭时全部接口 404
//! （服务层 gate）。

pub mod federation;
pub mod sso;
pub mod xorder;

use axum::Router;

use super::super::PublicState;

/// 匿名可访问路由（自带凭证鉴权；含成员侧 SSO 发起/回调）。
pub fn anonymous_routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .merge(federation::routes(state))
        .merge(sso::anonymous_routes(state))
}

/// 需会员登录路由（中枢侧 SSO 同意面 + 跨店下单前台面，自套 require_auth）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .merge(sso::routes(state))
        .merge(xorder::routes(state))
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    let mut out = federation::implemented_routes();
    out.extend(sso::implemented_routes());
    out.extend(xorder::implemented_routes());
    out
}
