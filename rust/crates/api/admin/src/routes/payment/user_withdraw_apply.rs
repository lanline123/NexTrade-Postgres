//! 管理端提现申请路由（payment::user_withdraw_apply）。
//! 对应原 Java `com.nextrade.adminapi.controller.finance.UserWithdrawApplyController`
//! （`@RequestMapping("/adminapi/finance/userWithdrawApply")`，7 个端点）。
//!
//! | 本模块路由（挂载于 `/adminapi`） | 原 Java | 权限 |
//! |----------------------------------|---------|------|
//! | `GET  /finance/userWithdrawApply/list` | `list` | jwt |
//! | `GET  /finance/userWithdrawApply/detail` | `detail` | jwt |
//! | `POST /finance/userWithdrawApply/create` | `create` | `userWithdrawApplyUpdateManage` |
//! | `POST /finance/userWithdrawApply/update` | `update` | `userWithdrawApplyUpdateManage` |
//! | `POST /finance/userWithdrawApply/del` | `del` | `userWithdrawApplyDelManage` |
//! | `POST /finance/userWithdrawApply/updateField` | `updateField` | `userWithdrawApplyUpdateManage` |
//! | `POST /finance/userWithdrawApply/batch` | `batch` | `userWithdrawApplyBatchManage` |
//!
//! > 权限矩阵未迁移（Rust 只校验登录态）；`list` 返回项目 `PageResult`。

use axum::{
    extract::Extension,
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use nextrade_core::models::payment::user_withdraw_apply::{
    BatchDTO, OperateDTO, UpdateFieldDTO, UserWithdrawApplyCreateDTO, UserWithdrawApplyListQuery,
    UserWithdrawApplyUpdateDTO, UserWithdrawApplyVO,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::AdminState;

/// 本域 7 条路由（去掉 `/adminapi` 前缀后的路径 + 原 Java handler）。
const ROUTE_TABLE: [(&str, &str, &str); 7] = [
    (
        "GET",
        "/finance/userWithdrawApply/list",
        "UserWithdrawApplyController.list",
    ),
    (
        "GET",
        "/finance/userWithdrawApply/detail",
        "UserWithdrawApplyController.detail",
    ),
    (
        "POST",
        "/finance/userWithdrawApply/create",
        "UserWithdrawApplyController.create",
    ),
    (
        "POST",
        "/finance/userWithdrawApply/update",
        "UserWithdrawApplyController.update",
    ),
    (
        "POST",
        "/finance/userWithdrawApply/del",
        "UserWithdrawApplyController.del",
    ),
    (
        "POST",
        "/finance/userWithdrawApply/updateField",
        "UserWithdrawApplyController.updateField",
    ),
    (
        "POST",
        "/finance/userWithdrawApply/batch",
        "UserWithdrawApplyController.batch",
    ),
];

/// 提现申请域路由（挂载于 `/adminapi` 下，外层 `require_auth`）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/finance/userWithdrawApply/list", get(list))
        .route("/finance/userWithdrawApply/detail", get(detail))
        .route("/finance/userWithdrawApply/create", post(create))
        .route("/finance/userWithdrawApply/update", post(update))
        .route("/finance/userWithdrawApply/del", post(del))
        .route("/finance/userWithdrawApply/updateField", post(update_field))
        .route("/finance/userWithdrawApply/batch", post(batch))
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

/// `detail?id=` 查询参数。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdQuery {
    id: i32,
}

async fn list(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<UserWithdrawApplyListQuery>,
) -> ApiResult<PageResult<UserWithdrawApplyVO>> {
    ok(state
        .payment
        .user_withdraw_apply
        .list(&q, auth.scoped_shop_id())
        .await?)
}

async fn detail(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<IdQuery>,
) -> ApiResult<UserWithdrawApplyVO> {
    ok(state
        .payment
        .user_withdraw_apply
        .detail(q.id, auth.scoped_shop_id())
        .await?)
}

async fn create(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<UserWithdrawApplyCreateDTO>,
) -> ApiResult<()> {
    let _id = state
        .payment
        .user_withdraw_apply
        .create(&dto, auth.scoped_shop_id())
        .await?;
    ok(())
}

async fn update(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<UserWithdrawApplyUpdateDTO>,
) -> ApiResult<()> {
    state
        .payment
        .user_withdraw_apply
        .update(&dto, auth.scoped_shop_id())
        .await?;
    ok(())
}

async fn del(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<OperateDTO>,
) -> ApiResult<()> {
    state
        .payment
        .user_withdraw_apply
        .delete(&dto, auth.scoped_shop_id())
        .await?;
    ok(())
}

async fn update_field(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<UpdateFieldDTO>,
) -> ApiResult<()> {
    state
        .payment
        .user_withdraw_apply
        .update_field(&dto, auth.scoped_shop_id())
        .await?;
    ok(())
}

async fn batch(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<BatchDTO>,
) -> ApiResult<()> {
    state
        .payment
        .user_withdraw_apply
        .batch(&dto, auth.scoped_shop_id())
        .await?;
    ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_java_contract() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 7);
        let expected = [
            ("GET", "/adminapi/finance/userWithdrawApply/list"),
            ("GET", "/adminapi/finance/userWithdrawApply/detail"),
            ("POST", "/adminapi/finance/userWithdrawApply/create"),
            ("POST", "/adminapi/finance/userWithdrawApply/update"),
            ("POST", "/adminapi/finance/userWithdrawApply/del"),
            ("POST", "/adminapi/finance/userWithdrawApply/updateField"),
            ("POST", "/adminapi/finance/userWithdrawApply/batch"),
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
            ("GET", "/finance/userWithdrawApply/list"),
            ("GET", "/finance/userWithdrawApply/detail"),
            ("POST", "/finance/userWithdrawApply/create"),
            ("POST", "/finance/userWithdrawApply/update"),
            ("POST", "/finance/userWithdrawApply/del"),
            ("POST", "/finance/userWithdrawApply/updateField"),
            ("POST", "/finance/userWithdrawApply/batch"),
        ];
        assert_eq!(paths.len(), ROUTE_TABLE.len());
        for (m, p) in paths {
            assert!(ROUTE_TABLE.iter().any(|(mm, pp, _)| *mm == m && *pp == p));
        }
        for (_, p, _) in ROUTE_TABLE.iter() {
            assert!(paths.iter().any(|(_, pp)| pp == p));
        }
    }
}
#[cfg(test)]
mod tenant_tests {
    use super::*;

    #[test]
    fn shop_scope_forced_and_cross_shop_rejected() {
        use nextrade_core::Error;
        let shop = AuthUser::new(1, "s".into(), None, Some("shop".into()), Some(7));
        assert_eq!(shop.scoped_shop_id(), Some(7));
        assert!(matches!(
            shop.resolve_shop_scope(Some(9)),
            Err(Error::Forbidden(_))
        ));
        let admin = AuthUser::new(2, "a".into(), None, Some("admin".into()), Some(0));
        assert_eq!(admin.scoped_shop_id(), None);
    }
}
