//! 平台联邦·跨店下单前台面路由（三期③，迁移 044；DESIGN.md §3）。
//!
//! 来源标注：**我方新增**（总设计 §7.2）。中枢买家在本机商城跨成员店铺
//! 下单：全部端点需会员 JWT，订单只认本人（服务层按 buyer 过滤）。
//!
//! | method | path | 说明 |
//! |--------|------|------|
//! | `POST` | `/hub/xorder/create` | 跨店下单（预占→建单→paylog） |
//! | `GET`  | `/hub/xorder/myList` | 我的跨店单 |
//! | `GET`  | `/hub/xorder/detail?id=` | 跨店单详情 |
//! | `POST` | `/hub/xorder/cancel` | 取消未付跨店单 |
//! | `POST` | `/hub/xorder/refundSub` | 子单退款申请 |

use axum::{
    extract::{Query, State},
    middleware,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;

use nextrade_core::models::hub::federation::{
    HubXorderVO, XorderCreateDTO, XorderIdDTO, XorderRefundSubDTO,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, require_auth, ApiResult, AppError, AuthUser};

use super::super::super::PublicState;

/// 本域路由（去掉 `/api` 前缀）。
const ROUTE_TABLE: [(&str, &str, &str); 5] = [
    ("POST", "/hub/xorder/create", "HubXorderApi.create"),
    ("GET", "/hub/xorder/myList", "HubXorderApi.myList"),
    ("GET", "/hub/xorder/detail", "HubXorderApi.detail"),
    ("POST", "/hub/xorder/cancel", "HubXorderApi.cancel"),
    ("POST", "/hub/xorder/refundSub", "HubXorderApi.refundSub"),
];

/// 需会员登录路由（自套 require_auth）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/hub/xorder/create", post(create))
        .route("/hub/xorder/myList", get(my_list))
        .route("/hub/xorder/detail", get(detail))
        .route("/hub/xorder/cancel", post(cancel))
        .route("/hub/xorder/refundSub", post(refund_sub))
        .layer(middleware::from_fn_with_state(
            state.jwt.clone(),
            require_auth,
        ))
}

/// 本域已实现路由（覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| {
            nextrade_openapi::RouteDoc::new(method, &format!("/api{path}"), handler)
        })
        .collect()
}

async fn create(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<XorderCreateDTO>,
) -> ApiResult<HubXorderVO> {
    let vo = state
        .hub
        .federation
        .xorder_create(auth.id, &dto)
        .await
        .map_err(AppError)?;
    ok(vo)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MyListParams {
    page: Option<u64>,
    page_size: Option<u64>,
}

async fn my_list(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<MyListParams>,
) -> ApiResult<PageResult<HubXorderVO>> {
    let vo = state
        .hub
        .federation
        .xorder_my_list(auth.id, q.page.unwrap_or(1), q.page_size.unwrap_or(20))
        .await
        .map_err(AppError)?;
    ok(vo)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DetailParams {
    id: i32,
}

async fn detail(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<DetailParams>,
) -> ApiResult<HubXorderVO> {
    let vo = state
        .hub
        .federation
        .xorder_detail(auth.id, q.id)
        .await
        .map_err(AppError)?;
    ok(vo)
}

async fn cancel(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<XorderIdDTO>,
) -> ApiResult<&'static str> {
    state
        .hub
        .federation
        .xorder_cancel(auth.id, dto.id)
        .await
        .map_err(AppError)?;
    ok("已取消")
}

async fn refund_sub(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<XorderRefundSubDTO>,
) -> ApiResult<&'static str> {
    state
        .hub
        .federation
        .xorder_refund_sub(auth.id, &dto)
        .await
        .map_err(AppError)?;
    ok("退款已处理")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract_exactly() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), ROUTE_TABLE.len());
        for (doc, (method, path, _)) in docs.iter().zip(ROUTE_TABLE.iter()) {
            assert_eq!(doc.method, *method);
            assert_eq!(doc.path, format!("/api{path}"));
        }
    }
}
