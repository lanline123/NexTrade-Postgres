//! 外卖聚合管理端路由（takeout 上下文，我方新增）。
//!
//! 论坛 C 类第 ③ 项一期（设计 `~/workspace/takeout-aggregation/DESIGN.md`）。
//! 页面接口统一挂 `/adminapi/setting/takeout/*`；租户作用域走
//! `tenant_scope_layer`（与 content / marketing 域同层），服务层以
//! `TenantScope` 判定平台/门店可见范围，不信任客户端租户头。

pub mod manage;

use axum::Router;

use crate::routes::marketing::tenant_layer::tenant_scope_layer;

use super::super::AdminState;

/// 需登录的域路由（外层统一套 require_auth）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .merge(manage::routes(state))
        .layer(axum::middleware::from_fn(tenant_scope_layer))
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    manage::implemented_routes()
}
