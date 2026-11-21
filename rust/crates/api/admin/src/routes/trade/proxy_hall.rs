//! 代客下单接单大厅路由（trade::proxy_hall，我方新增，二期）。
//!
//! | 本模块路由（`/adminapi` 前缀在 `lib.rs` 统一嵌套） | 用途 |
//! |--------------------------------------------------|------|
//! | `GET  /proxyShopping/hall/pool` | 待接单池（同体系下单店的在池单） |
//! | `GET  /proxyShopping/hall/mine` | 我的接单（本店履约的已接单/已备货） |
//! | `POST /proxyShopping/hall/accept` | 抢单（原子认领） |
//! | `POST /proxyShopping/hall/assign` | 派单（下单店/平台指派履约门店） |
//! | `POST /proxyShopping/hall/reject` | 拒单回池（当前履约店退单） |
//! | `POST /proxyShopping/hall/ready` | 备货完成标记（当前履约店） |
//!
//! > 身份口径与同域 `proxy_shopping_order_check` 一致：仅信任 JWT
//! > （`AuthUser::scoped_shop_id()`），店铺作用域在服务层强制。
//! > 服务按需构造（`ProxyHallService::new(state.repos)`，与调度器
//! > 内 GiftOrderService 的构造先例一致），不进 trade 服务集合。

use axum::{
    extract::{Extension, Query, State},
    routing::{get, post},
    Json, Router,
};
use nextrade_core::models::trade::proxy_hall::{
    HallActionVO, HallAssignParam, HallOrderActionParam, HallOrderVO, HallRejectParam,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult, AuthUser};
use nextrade_services::trade::proxy_hall::ProxyHallService;
use nextrade_services::trade::proxy_shopping_order_check::RequestContext;

use crate::AdminState;

/// 本域 6 条路由（去 `/adminapi` 前缀的路径 + 处理器名，均为我方新增）。
const ROUTE_TABLE: [(&str, &str, &str); 6] = [
    ("GET", "/proxyShopping/hall/pool", "ProxyHallController.pool"),
    ("GET", "/proxyShopping/hall/mine", "ProxyHallController.mine"),
    ("POST", "/proxyShopping/hall/accept", "ProxyHallController.accept"),
    ("POST", "/proxyShopping/hall/assign", "ProxyHallController.assign"),
    ("POST", "/proxyShopping/hall/reject", "ProxyHallController.reject"),
    ("POST", "/proxyShopping/hall/ready", "ProxyHallController.ready"),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/proxyShopping/hall/pool", get(pool))
        .route("/proxyShopping/hall/mine", get(mine))
        .route("/proxyShopping/hall/accept", post(accept))
        .route("/proxyShopping/hall/assign", post(assign))
        .route("/proxyShopping/hall/reject", post(reject))
        .route("/proxyShopping/hall/ready", post(ready))
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/adminapi{path}"), handler))
        .collect()
}

fn ctx_of(auth: &AuthUser) -> RequestContext {
    RequestContext::new(auth.scoped_shop_id(), auth.admin_type.clone())
}

fn service(state: &AdminState) -> ProxyHallService {
    ProxyHallService::new(state.repos.clone())
}

/// `GET /adminapi/proxyShopping/hall/pool`：待接单池。
async fn pool(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<PageQuery>,
) -> ApiResult<PageResult<HallOrderVO>> {
    ok(service(&state).pool(&query, &ctx_of(&auth)).await?)
}

/// `GET /adminapi/proxyShopping/hall/mine`：我的接单。
async fn mine(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<PageQuery>,
) -> ApiResult<PageResult<HallOrderVO>> {
    ok(service(&state).mine(&query, &ctx_of(&auth)).await?)
}

/// `POST /adminapi/proxyShopping/hall/accept`：抢单。
async fn accept(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(param): Json<HallOrderActionParam>,
) -> ApiResult<HallActionVO> {
    ok(service(&state)
        .accept(&param, &ctx_of(&auth), auth.id)
        .await?)
}

/// `POST /adminapi/proxyShopping/hall/assign`：派单。
async fn assign(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(param): Json<HallAssignParam>,
) -> ApiResult<HallActionVO> {
    ok(service(&state)
        .assign(&param, &ctx_of(&auth), auth.id)
        .await?)
}

/// `POST /adminapi/proxyShopping/hall/reject`：拒单回池。
async fn reject(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(param): Json<HallRejectParam>,
) -> ApiResult<HallActionVO> {
    ok(service(&state)
        .reject(&param, &ctx_of(&auth), auth.id)
        .await?)
}

/// `POST /adminapi/proxyShopping/hall/ready`：备货完成。
async fn ready(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(param): Json<HallOrderActionParam>,
) -> ApiResult<HallActionVO> {
    ok(service(&state).ready(&param, &ctx_of(&auth)).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract_exactly() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), ROUTE_TABLE.len());
        for (doc, (method, path, handler)) in docs.iter().zip(ROUTE_TABLE.iter()) {
            assert_eq!(doc.method, *method);
            assert_eq!(doc.path, format!("/adminapi{path}"));
            assert_eq!(doc.java_ref, *handler);
        }
    }
}
