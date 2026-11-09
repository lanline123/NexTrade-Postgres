//! 管理端分销员域路由。
//! 对应原 Java `controller.salesman.SalesmanController` /
//! `SalesmanGroupController` / `SalesmanCustomerTransactionController` /
//! `SalesmanOrderController` / `SalesmanMaterialController` 等。
//!
//! TODO(域实现者)：对照 Java 控制器补全分销员增删改、佣金订单、素材等接口。

use axum::{
    extract::{Query, State},
    routing::get,
    Router,
};
use nextrade_core::models::{SalesmanListQuery, SalesmanVO};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult};

use crate::AdminState;

/// 分销员域路由（挂载于 `/adminapi` 下）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new().route("/salesman/salesman/list", get(salesman_list))
}

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    [(
        "GET",
        "/adminapi/salesman/salesman/list",
        "SalesmanController.list",
    )]
    .iter()
    .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, p, j))
    .collect()
}

/// `GET /adminapi/salesman/salesman/list`：分销员分页。
async fn salesman_list(
    State(state): State<AdminState>,
    Query(q): Query<SalesmanListQuery>,
) -> ApiResult<PageResult<SalesmanVO>> {
    ok(state.marketing.salesman.list(&q).await?)
}
