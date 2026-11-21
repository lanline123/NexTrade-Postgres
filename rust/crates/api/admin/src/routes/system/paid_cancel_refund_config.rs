//! 管理端 已付未发货买家取消直退设置路由（system::paid_cancel_refund_config，2 条）。
//!
//! 平台级配置（我方新增·按用户口径，论坛帖 347748 深评后 2026-11-21
//! 用户拍板）：存 `config` 表 `paidCancelRefund` 一行 JSON——主开关
//! （默认关）+ 小额免审开关与上限 + 进财务审核队列开关（默认开）。
//! 契约与 `setting/config` 域既有命名保持一致：
//! `GET /paidCancelRefundSettings`、`POST /savePaidCancelRefund`。

use axum::{extract::State, routing::{get, post}, Json, Router};
use serde_json::Value;
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult};

use crate::support::tenant::platform_only;

use crate::AdminState;

/// 本域 2 条路由（`/adminapi` 前缀之后的路径）。
const ROUTE_TABLE: [(&str, &str, &str); 2] = [
    (
        "GET",
        "/setting/config/paidCancelRefundSettings",
        "PaidCancelRefundConfigController.getPaidCancelRefundSettings",
    ),
    (
        "POST",
        "/setting/config/savePaidCancelRefund",
        "PaidCancelRefundConfigController.savePaidCancelRefund",
    ),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route(
            "/setting/config/paidCancelRefundSettings",
            get(get_settings),
        )
        .route(
            "/setting/config/savePaidCancelRefund",
            post(save_settings),
        )
        .layer(axum::middleware::from_fn(platform_only))
}

/// 本域已实现路由（完整契约路径，用于覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/adminapi{path}"), handler))
        .collect()
}

/// `GET /adminapi/setting/config/paidCancelRefundSettings`：读取生效中的
/// 买家取消直退设置（未保存过时返回内置默认：主开关关、审核开）。
async fn get_settings(State(state): State<AdminState>) -> ApiResult<Value> {
    ok(state.system.config_basic.get_paid_cancel_refund().await?)
}

/// `POST /adminapi/setting/config/savePaidCancelRefund`：保存设置
/// （整块覆盖，保存前经服务层清洗校验）。
async fn save_settings(
    State(state): State<AdminState>,
    Json(payload): Json<Value>,
) -> ApiResult<()> {
    state
        .system
        .config_basic
        .save_paid_cancel_refund(&payload)
        .await?;
    ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract_exactly() {
        let expected = [
            ("GET", "/adminapi/setting/config/paidCancelRefundSettings"),
            ("POST", "/adminapi/setting/config/savePaidCancelRefund"),
        ];
        let docs = implemented_routes();
        assert_eq!(docs.len(), 2);
        for (doc, (method, path)) in docs.iter().zip(expected.iter()) {
            assert_eq!(doc.method, *method);
            assert_eq!(doc.path, *path);
        }
    }
}
