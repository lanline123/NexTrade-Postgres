//! 站点搬家管理端路由（site_clone 上下文）。
//!
//! 来源标注：**我方新增**（设计 `~/workspace/site-clone-design/REPORT.md`
//! 第二节）。页面接口统一挂 `/adminapi/siteClone/*`，整域平台专属
//! （`platform_only`，设置域是平台级数据，店铺账号不开放）。

pub mod clone;

use axum::Router;

use crate::AdminState;

/// 本域路由汇总。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new().merge(clone::routes(state))
}

/// 本域已实现路由（覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    clone::implemented_routes()
}
