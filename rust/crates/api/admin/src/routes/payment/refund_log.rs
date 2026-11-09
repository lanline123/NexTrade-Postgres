//! 管理端退款记录路由（payment::refund_log）。
//!
//! 原 Java：`com.nextrade.adminapi.controller.finance.RefundLogController`
//! （`java/nextrade-adminapi/src/main/java/com/nextrade/adminapi/controller/finance/RefundLogController.java`，
//! `@RequestMapping("/adminapi/finance/refundLog")`，7 个端点）。
//!
//! ## 原 Java 路径对照表（本模块挂载于 `/adminapi` 之下）
//!
//! | 本模块路由 | 原 Java 完整路径 | 原 Java 方法 | 契约 auth |
//! |-----------|------------------|----------------|-----------|
//! | `GET /finance/refundLog/list` | `GET /adminapi/finance/refundLog/list` | `RefundLogController.list` | jwt |
//! | `GET /finance/refundLog/detail` | `GET /adminapi/finance/refundLog/detail` | `RefundLogController.detail` | jwt |
//! | `POST /finance/refundLog/create` | `POST /adminapi/finance/refundLog/create` | `RefundLogController.create` | jwt |
//! | `POST /finance/refundLog/update` | `POST /adminapi/finance/refundLog/update` | `RefundLogController.update` | jwt |
//! | `POST /finance/refundLog/del` | `POST /adminapi/finance/refundLog/del` | `RefundLogController.del` | jwt |
//! | `POST /finance/refundLog/updateField` | `POST /adminapi/finance/refundLog/updateField` | `RefundLogController.updateField` | jwt |
//! | `POST /finance/refundLog/batch` | `POST /adminapi/finance/refundLog/batch` | `RefundLogController.batch` | jwt |
//!
//! > **路径说明**：`@RequestMapping("/adminapi/finance/refundLog")` +
//! > `@GetMapping("/list")` 拼出 `/adminapi/finance/refundLog/list` ——
//! > 控制器前缀里 `finance` 与 `refundLog` 各出现一次，`refundLog` 大小写
//! > 原样照抄，不是笔误；本文件挂在 `/adminapi` 之下，故 `routes()` 内只写
//! > `/finance/refundLog/...`。
//!
//! > **权限**：契约 `auth=jwt`，Java 该控制器**没有** `@PreAuthorize`，
//! > 与 Rust 侧 `require_auth`（`api/admin/src/lib.rs::router` 统一套在
//! > `/adminapi` 子树外层）语义一致，无需额外处理。
//!
//! > **返回结构**：Java `list` 返回 MyBatis-Plus `Page<RefundLogVO>`
//! > （`records/total/size/current/pages`），Rust 统一用 [`PageResult`]
//! > （`list/total/totalPage/pageSize/page`）；`create/update/del/updateField/batch`
//! > Java 返回 `void`，Rust 返回 `null` 数据信封。

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use nextrade_core::models::payment::refund_log::{
    BatchDTO, OperateDTO, RefundLogCreateDTO, RefundLogListQuery, RefundLogUpdateDTO, RefundLogVO,
    UpdateFieldDTO,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult};

use crate::support::tenant::platform_only;

use crate::AdminState;

/// 本域 7 条路由（去掉 `/adminapi` 前缀后的路径 + 原 Java handler）。
const ROUTE_TABLE: [(&str, &str, &str); 7] = [
    ("GET", "/finance/refundLog/list", "RefundLogController.list"),
    (
        "GET",
        "/finance/refundLog/detail",
        "RefundLogController.detail",
    ),
    (
        "POST",
        "/finance/refundLog/create",
        "RefundLogController.create",
    ),
    (
        "POST",
        "/finance/refundLog/update",
        "RefundLogController.update",
    ),
    ("POST", "/finance/refundLog/del", "RefundLogController.del"),
    (
        "POST",
        "/finance/refundLog/updateField",
        "RefundLogController.updateField",
    ),
    (
        "POST",
        "/finance/refundLog/batch",
        "RefundLogController.batch",
    ),
];

/// 退款记录域路由（挂载于 `/adminapi` 下，外层 `require_auth`）。
///
/// ⚠️ 路径必须写成**字符串字面量**（不从 [`ROUTE_TABLE`] 循环生成）：
/// `tools/check_api_coverage.py` 静态扫描 `.route("...")` 字面量，
/// 循环拼接会让覆盖率统计看不见这些路由。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/finance/refundLog/list", get(list))
        .route("/finance/refundLog/detail", get(detail))
        .route("/finance/refundLog/create", post(create))
        .route("/finance/refundLog/update", post(update))
        .route("/finance/refundLog/del", post(del))
        .route("/finance/refundLog/updateField", post(update_field))
        .route("/finance/refundLog/batch", post(batch))
        .layer(axum::middleware::from_fn(platform_only))
}

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计，完整契约路径）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, java_ref)| {
            nextrade_openapi::RouteDoc::new(method, &format!("/adminapi{path}"), java_ref)
        })
        .collect()
}

/// `detail?id=` 查询参数（Java `@RequestParam Integer id`）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdQuery {
    id: i32,
}

/// `GET /adminapi/finance/refundLog/list`：退款记录分页列表。
async fn list(
    State(state): State<AdminState>,
    Query(q): Query<RefundLogListQuery>,
) -> ApiResult<PageResult<RefundLogVO>> {
    ok(state.payment.refund_log.list(&q).await?)
}

/// `GET /adminapi/finance/refundLog/detail`：退款记录详情。
async fn detail(
    State(state): State<AdminState>,
    Query(q): Query<IdQuery>,
) -> ApiResult<RefundLogVO> {
    ok(state.payment.refund_log.detail(q.id).await?)
}

/// `POST /adminapi/finance/refundLog/create`：创建退款记录。
async fn create(
    State(state): State<AdminState>,
    Json(dto): Json<RefundLogCreateDTO>,
) -> ApiResult<()> {
    let _id = state.payment.refund_log.create(&dto).await?;
    ok(())
}

/// `POST /adminapi/finance/refundLog/update`：更新退款记录。
async fn update(
    State(state): State<AdminState>,
    Json(dto): Json<RefundLogUpdateDTO>,
) -> ApiResult<()> {
    state.payment.refund_log.update(&dto).await?;
    ok(())
}

/// `POST /adminapi/finance/refundLog/del`：删除退款记录。
async fn del(State(state): State<AdminState>, Json(dto): Json<OperateDTO>) -> ApiResult<()> {
    state.payment.refund_log.delete(&dto).await?;
    ok(())
}

/// `POST /adminapi/finance/refundLog/updateField`：更新单字段（本表白名单为空）。
async fn update_field(
    State(state): State<AdminState>,
    Json(dto): Json<UpdateFieldDTO>,
) -> ApiResult<()> {
    state.payment.refund_log.update_field(&dto).await?;
    ok(())
}

/// `POST /adminapi/finance/refundLog/batch`：批量操作（仅 `del` 生效）。
async fn batch(State(state): State<AdminState>, Json(dto): Json<BatchDTO>) -> ApiResult<()> {
    state.payment.refund_log.batch(&dto).await?;
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
            ("GET", "/adminapi/finance/refundLog/list"),
            ("GET", "/adminapi/finance/refundLog/detail"),
            ("POST", "/adminapi/finance/refundLog/create"),
            ("POST", "/adminapi/finance/refundLog/update"),
            ("POST", "/adminapi/finance/refundLog/del"),
            ("POST", "/adminapi/finance/refundLog/updateField"),
            ("POST", "/adminapi/finance/refundLog/batch"),
        ];
        for (method, path) in expected {
            assert!(
                docs.iter().any(|d| d.method == method && d.path == path),
                "缺少 {method} {path}"
            );
        }
        // 不允许重复注册（axum merge 重复路径会在启动时 panic）
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
            ("GET", "/finance/refundLog/list"),
            ("GET", "/finance/refundLog/detail"),
            ("POST", "/finance/refundLog/create"),
            ("POST", "/finance/refundLog/update"),
            ("POST", "/finance/refundLog/del"),
            ("POST", "/finance/refundLog/updateField"),
            ("POST", "/finance/refundLog/batch"),
        ];
        assert_eq!(paths.len(), ROUTE_TABLE.len());
        for (m, p) in paths {
            assert!(
                ROUTE_TABLE.iter().any(|(mm, pp, _)| *mm == m && *pp == p),
                "ROUTE_TABLE 缺少 {m} {p}"
            );
        }
        for (_, p, _) in ROUTE_TABLE.iter() {
            assert!(paths.iter().any(|(_, pp)| pp == p));
        }
    }

    #[test]
    fn module_paths_are_relative_to_adminapi() {
        // 本模块路径不能带 /adminapi（否则会挂成 /adminapi/adminapi/...）
        for (_, path, _) in ROUTE_TABLE.iter() {
            assert!(!path.contains("/adminapi"), "{path}");
            assert!(path.starts_with("/finance/refundLog/"), "{path}");
        }
        // implemented_routes 给出的却是完整契约路径
        assert_eq!(
            implemented_routes()[0].path,
            "/adminapi/finance/refundLog/list"
        );
    }
}
