//! 前台 投流配置路由（system::ad_settings_client，1 条）。
//!
//! 本平台新增端点（非 Java 契约）：`GET /api/common/config/adSettings`
//! 返回平台「投流设置」的生效配置，供商家端/前台展示单价与开关。未配置时返回内置默认。

use axum::{extract::State, routing::get, Router};
use serde_json::Value;
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult};

use crate::PublicState;

/// 本域 1 条路由（`/api` 前缀之后的路径）。
const ROUTE_TABLE: [(&str, &str, &str); 1] = [(
    "GET",
    "/common/config/adSettings",
    "AdSettingsController.getAdSettings",
)];

pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new().route("/common/config/adSettings", get(get_ad_settings))
}

/// 本域已实现路由（完整契约路径，用于覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/api{path}"), handler))
        .collect()
}

/// `GET /api/common/config/adSettings`：投流配置（公开只读）。
async fn get_ad_settings(State(state): State<PublicState>) -> ApiResult<Value> {
    ok(state.system.config_client.ad_settings().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract_exactly() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].method, "GET");
        assert_eq!(docs[0].path, "/api/common/config/adSettings");
    }
}
