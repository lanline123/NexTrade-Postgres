//! 商城端表单令牌路由：`GET /api/common/csrf/create`。
//!
//! 对应原 Java `com.nextrade.api.controller.common.CSRFController#create`。
//!
//! ⚠️ 该路径在 Java `WhiteListConfig` 中（`/api/common/csrf/**`），即**免登录**。
//! 因此本模块注册在 `anonymous_routes`（需要 state、但不套 `require_auth`）。

use axum::{extract::State, routing::get, Router};
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult};

use crate::PublicState;

const ROUTE_TABLE: [(&str, &str, &str); 1] = [(
    "GET",
    "/common/csrf/create",
    "CSRFController.create",
)];

/// 匿名路由（需要 state，但免登录）。
pub fn anonymous_routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new().route("/common/csrf/create", get(create))
}

/// 本模块已实现路由（覆盖率统计用，完整契约路径）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/api{path}"), handler))
        .collect()
}

/// `GET /api/common/csrf/create`：生成表单令牌。
async fn create(State(state): State<PublicState>) -> ApiResult<String> {
    ok(state.system.csrf.create().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_table_matches_implemented_routes() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), ROUTE_TABLE.len());
        for (idx, (method, path, handler)) in ROUTE_TABLE.iter().enumerate() {
            assert_eq!(docs[idx].method, *method);
            assert_eq!(docs[idx].path, format!("/api{path}"));
            assert_eq!(docs[idx].java_ref, *handler);
        }
    }
}
