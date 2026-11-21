//! 前台 会员认证门控配置路由（system::auth_gate_client，1 条）。
//!
//! 本平台新增端点（非 Java 契约）：`GET /api/common/config/authGate`
//! 返回后台「会员认证设置」中与前台相关的四项开关（`closeAuth` 认证
//! 功能开关 / `isIdentity` 实名后看价 / `isLogin` 登录后看价（我方
//! 新增，2026-11-21 门控拆双开关）/ `type` 认证类型），供 PC/H5 前台
//! 决定认证入口显隐与价格门控渲染口径。未配置时全 0。与 `config_client`
//! 的 Java 契约文件分开，避免污染其契约测试（同 buyStyle 先例）。

use axum::{extract::State, routing::get, Router};
use serde_json::Value;
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult};

use crate::PublicState;

/// 本域 1 条路由（`/api` 前缀之后的路径）。
const ROUTE_TABLE: [(&str, &str, &str); 1] = [(
    "GET",
    "/common/config/authGate",
    "AuthGateController.getAuthGate",
)];

pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new().route("/common/config/authGate", get(get_auth_gate))
}

/// 本域已实现路由（完整契约路径，用于覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/api{path}"), handler))
        .collect()
}

/// `GET /api/common/config/authGate`：会员认证门控配置（公开只读）。
async fn get_auth_gate(State(state): State<PublicState>) -> ApiResult<Value> {
    ok(state.system.config_client.auth_gate().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract_exactly() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].method, "GET");
        assert_eq!(docs[0].path, "/api/common/config/authGate");
    }
}
