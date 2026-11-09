//! 管理端授信会员路由（payment::credit_user）。
//! 对应原 Java `com.nextrade.adminapi.controller.finance.CreditUserController`
//! （`@RequestMapping("/adminapi/credit/user")`，6 个端点）。
//!
//! | 本模块路由（挂载于 `/adminapi`） | 原 Java 完整路径 | 原 Java 方法 | 权限 |
//! |----------------------------------|------------------|--------------|------|
//! | `GET  /credit/user/list` | `GET /adminapi/credit/user/list` | `list` | `termMemberManage` |
//! | `GET  /credit/user/detail` | `GET /adminapi/credit/user/detail` | `detail` | `termMemberManage` |
//! | `POST /credit/user/create` | `POST /adminapi/credit/user/create` | `create` | `termMemberModifyManage` |
//! | `POST /credit/user/update` | `POST /adminapi/credit/user/update` | `update` | `termMemberModifyManage` |
//! | `POST /credit/user/delete` | `POST /adminapi/credit/user/delete` | `delete` | `termMemberModifyManage` |
//! | `POST /credit/user/updateField` | `POST /adminapi/credit/user/updateField` | `updateField` | `termMemberModifyManage` |
//!
//! > **权限**：Java `@PreAuthorize` 未迁移（Rust 只校验登录态，外层 `require_auth`）。
//! > **分页**：`list` 返回项目 `PageResult`（Java 为 MyBatis-Plus `Page`，字段一致）。
//! > **`detail`**：`creditUserId` 为必填查询参数（缺失时由 `Query` 反序列化拒绝）。
//! > **`updateField`**：白名单 `CREDIT_USER_FIELDS = {"status"}`；`@Valid` 保证
//! > `id/field/val` 非空。

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use nextrade_core::models::payment::credit_user::{
    CreditUserCreateDTO, CreditUserListQuery, CreditUserListVO, CreditUserUpdateDTO, CreditUserVO,
    OperateDTO, UpdateFieldDTO,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult};

use crate::support::tenant::platform_only;

use crate::AdminState;

/// 本域 6 条路由（去掉 `/adminapi` 前缀后的路径 + 原 Java handler）。
const ROUTE_TABLE: [(&str, &str, &str); 6] = [
    ("GET", "/credit/user/list", "CreditUserController.list"),
    ("GET", "/credit/user/detail", "CreditUserController.detail"),
    ("POST", "/credit/user/create", "CreditUserController.create"),
    ("POST", "/credit/user/update", "CreditUserController.update"),
    ("POST", "/credit/user/delete", "CreditUserController.delete"),
    (
        "POST",
        "/credit/user/updateField",
        "CreditUserController.updateField",
    ),
];

/// 授信会员域路由（挂载于 `/adminapi` 下，外层 `require_auth`）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/credit/user/list", get(list))
        .route("/credit/user/detail", get(detail))
        .route("/credit/user/create", post(create))
        .route("/credit/user/update", post(update))
        .route("/credit/user/delete", post(delete))
        .route("/credit/user/updateField", post(update_field))
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

/// `detail?creditUserId=` 查询参数（Java `@RequestParam Integer creditUserId`）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DetailQuery {
    credit_user_id: i32,
}

async fn list(
    State(state): State<AdminState>,
    Query(q): Query<CreditUserListQuery>,
) -> ApiResult<PageResult<CreditUserListVO>> {
    ok(state.payment.credit_user.list(&q).await?)
}

async fn detail(
    State(state): State<AdminState>,
    Query(q): Query<DetailQuery>,
) -> ApiResult<CreditUserVO> {
    ok(state.payment.credit_user.detail(q.credit_user_id).await?)
}

async fn create(
    State(state): State<AdminState>,
    Json(dto): Json<CreditUserCreateDTO>,
) -> ApiResult<()> {
    let _id = state.payment.credit_user.create(&dto).await?;
    ok(())
}

async fn update(
    State(state): State<AdminState>,
    Json(dto): Json<CreditUserUpdateDTO>,
) -> ApiResult<()> {
    state.payment.credit_user.update(&dto).await?;
    ok(())
}

async fn delete(State(state): State<AdminState>, Json(dto): Json<OperateDTO>) -> ApiResult<()> {
    state.payment.credit_user.delete(&dto).await?;
    ok(())
}

async fn update_field(
    State(state): State<AdminState>,
    Json(dto): Json<UpdateFieldDTO>,
) -> ApiResult<()> {
    state.payment.credit_user.update_field(&dto).await?;
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
            ("GET", "/adminapi/credit/user/list"),
            ("GET", "/adminapi/credit/user/detail"),
            ("POST", "/adminapi/credit/user/create"),
            ("POST", "/adminapi/credit/user/update"),
            ("POST", "/adminapi/credit/user/delete"),
            ("POST", "/adminapi/credit/user/updateField"),
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
            ("GET", "/credit/user/list"),
            ("GET", "/credit/user/detail"),
            ("POST", "/credit/user/create"),
            ("POST", "/credit/user/update"),
            ("POST", "/credit/user/delete"),
            ("POST", "/credit/user/updateField"),
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
    fn detail_query_deserializes_camel_case() {
        let q: DetailQuery = serde_json::from_str(r#"{"creditUserId": 12}"#).unwrap();
        assert_eq!(q.credit_user_id, 12);
        assert!(serde_json::from_str::<DetailQuery>(r#"{"creditUserId": "x"}"#).is_err());
    }

    #[test]
    fn handler_refs_are_unique() {
        let mut refs: Vec<&str> = ROUTE_TABLE.iter().map(|(_, _, j)| *j).collect();
        refs.sort();
        refs.dedup();
        assert_eq!(refs.len(), ROUTE_TABLE.len());
    }
}
