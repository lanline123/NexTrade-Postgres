//! 平台联邦管理端路由（hub 上下文）。
//!
//! 来源标注：**我方新增**（设计 `~/workspace/hub-federation-design/proposal.md` §6）。
//! 一级菜单「组织」下的页面接口统一挂 `/adminapi/hub/*`；开关页接口在
//! `system::federation_config`（`/adminapi/setting/config/federationSettings`）。

pub mod federation;

use axum::Router;

use crate::AdminState;

/// 本域路由汇总。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new().merge(federation::routes(state))
}

/// 本域已实现路由（覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    federation::implemented_routes()
}
