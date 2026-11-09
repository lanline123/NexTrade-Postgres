//! 管理端账期结算路由（payment::credit_settlement）。
//! 对应原 Java `com.nextrade.adminapi.controller.finance.CreditSettlementController`
//! （`@RequestMapping("/adminapi/credit/settlement")`，6 个端点）。
//!
//! | 本模块路由（挂载于 `/adminapi`） | 原 Java 完整路径 | 原 Java 方法 | 权限 |
//! |----------------------------------|------------------|--------------|------|
//! | `GET  /credit/settlement/userList` | `GET /adminapi/credit/settlement/userList` | `userList` | `termPayManage` |
//! | `GET  /credit/settlement/pendingOrders` | `GET /adminapi/credit/settlement/pendingOrders` | `pendingOrders` | `termPayManage` |
//! | `POST /credit/settlement/submit` | `POST /adminapi/credit/settlement/submit` | `submit` | `termPayModifyManage` |
//! | `GET  /credit/settlement/history` | `GET /adminapi/credit/settlement/history` | `history` | `termPayManage` |
//! | `GET  /credit/settlement/historyOrders` | `GET /adminapi/credit/settlement/historyOrders` | `historyOrders` | `termPayManage` |
//! | `POST /credit/settlement/cancel` | `POST /adminapi/credit/settlement/cancel` | `cancel` | `termPayModifyManage` |
//!
//! > **权限**：Java `@PreAuthorize` 未迁移（Rust 只校验登录态，外层 `require_auth`）。
//! > **分页**：`userList` / `pendingOrders` / `history` / `historyOrders` 返回项目
//! > `PageResult`（Java 为 MyBatis-Plus `Page`，字段口径一致）。
//! > **操作人**：`submit` 从登录态取 `AuthUser.id` 写 `operator_id`，并按
//! > `admin_user.username` 回填 `operator_name`（缺失回退「管理员」）。

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use nextrade_core::models::payment::credit_settlement::{
    CreditOrderListQuery, CreditOrderVO, CreditSettlementCancelDTO, CreditSettlementHistoryQuery,
    CreditSettlementSubmitDTO, CreditSettlementUserListQuery, CreditSettlementUserVO,
    CreditSettlementVO,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::support::tenant::platform_only;

use crate::AdminState;

/// 本域 6 条路由（去掉 `/adminapi` 前缀后的路径 + 原 Java handler）。
const ROUTE_TABLE: [(&str, &str, &str); 6] = [
    (
        "GET",
        "/credit/settlement/userList",
        "CreditSettlementController.userList",
    ),
    (
        "GET",
        "/credit/settlement/pendingOrders",
        "CreditSettlementController.pendingOrders",
    ),
    (
        "POST",
        "/credit/settlement/submit",
        "CreditSettlementController.submit",
    ),
    (
        "GET",
        "/credit/settlement/history",
        "CreditSettlementController.history",
    ),
    (
        "GET",
        "/credit/settlement/historyOrders",
        "CreditSettlementController.historyOrders",
    ),
    (
        "POST",
        "/credit/settlement/cancel",
        "CreditSettlementController.cancel",
    ),
];

/// 账期结算域路由（挂载于 `/adminapi` 下，外层 `require_auth`）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/credit/settlement/userList", get(user_list))
        .route("/credit/settlement/pendingOrders", get(pending_orders))
        .route("/credit/settlement/submit", post(submit))
        .route("/credit/settlement/history", get(history))
        .route("/credit/settlement/historyOrders", get(history_orders))
        .route("/credit/settlement/cancel", post(cancel))
        .layer(axum::middleware::from_fn(platform_only))
}

/// 本模块已实现路由（覆盖率统计，完整契约路径）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, java_ref)| {
            nextrade_openapi::RouteDoc::new(method, &format!("/adminapi{path}"), java_ref)
        })
        .collect()
}

async fn user_list(
    State(state): State<AdminState>,
    Query(q): Query<CreditSettlementUserListQuery>,
) -> ApiResult<PageResult<CreditSettlementUserVO>> {
    ok(state.payment.credit_settlement.user_list(&q).await?)
}

async fn pending_orders(
    State(state): State<AdminState>,
    Query(q): Query<CreditOrderListQuery>,
) -> ApiResult<PageResult<CreditOrderVO>> {
    ok(state.payment.credit_settlement.pending_orders(&q).await?)
}

async fn submit(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<CreditSettlementSubmitDTO>,
) -> ApiResult<()> {
    state
        .payment
        .credit_settlement
        .submit(&dto, auth.id)
        .await?;
    ok(())
}

async fn history(
    State(state): State<AdminState>,
    Query(q): Query<CreditSettlementHistoryQuery>,
) -> ApiResult<PageResult<CreditSettlementVO>> {
    ok(state.payment.credit_settlement.history(&q).await?)
}

async fn history_orders(
    State(state): State<AdminState>,
    Query(q): Query<CreditOrderListQuery>,
) -> ApiResult<PageResult<CreditOrderVO>> {
    ok(state.payment.credit_settlement.history_orders(&q).await?)
}

async fn cancel(
    State(state): State<AdminState>,
    Json(dto): Json<CreditSettlementCancelDTO>,
) -> ApiResult<()> {
    state.payment.credit_settlement.cancel(&dto).await?;
    ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_java_contract() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 6);
        let expected = [
            ("GET", "/adminapi/credit/settlement/userList"),
            ("GET", "/adminapi/credit/settlement/pendingOrders"),
            ("POST", "/adminapi/credit/settlement/submit"),
            ("GET", "/adminapi/credit/settlement/history"),
            ("GET", "/adminapi/credit/settlement/historyOrders"),
            ("POST", "/adminapi/credit/settlement/cancel"),
        ];
        for (method, path) in expected {
            assert!(
                docs.iter().any(|d| d.method == method && d.path == path),
                "缺少 {method} {path}"
            );
        }
        let mut keys: Vec<String> = docs.iter().map(nextrade_openapi::RouteDoc::key).collect();
        let mut unique = keys.clone();
        unique.sort();
        unique.dedup();
        keys.sort();
        assert_eq!(unique, keys);
    }

    #[test]
    fn route_table_matches_registrations() {
        let paths = [
            ("GET", "/credit/settlement/userList"),
            ("GET", "/credit/settlement/pendingOrders"),
            ("POST", "/credit/settlement/submit"),
            ("GET", "/credit/settlement/history"),
            ("GET", "/credit/settlement/historyOrders"),
            ("POST", "/credit/settlement/cancel"),
        ];
        assert_eq!(paths.len(), ROUTE_TABLE.len());
        for (m, p) in paths {
            assert!(ROUTE_TABLE.iter().any(|(mm, pp, _)| *mm == m && *pp == p));
        }
        for (_, p, _) in ROUTE_TABLE.iter() {
            assert!(paths.iter().any(|(_, pp)| pp == p));
        }
    }

    #[test]
    fn handler_methods_are_distinct() {
        let mut refs: Vec<&str> = ROUTE_TABLE.iter().map(|(_, _, j)| *j).collect();
        refs.sort();
        refs.dedup();
        assert_eq!(refs.len(), ROUTE_TABLE.len());
    }
}
