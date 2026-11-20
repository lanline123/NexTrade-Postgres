//! 管理端 平台联邦设置路由（system::federation_config，2 条）。
//!
//! 来源标注：**我方新增**（设计 `~/workspace/hub-federation-design/proposal.md`
//! §2/§6：设置页常驻「设置 > 平台联邦」，不受联邦总开关门控——总开关关闭时
//! 本页正是重新打开的入口）。存储：config 表 `federationSettings` 一行 JSON，
//! 口径同 systemMode。平台级配置，`platform_only` 层与 systemMode 一致。

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
        "/setting/config/federationSettings",
        "FederationConfigController.getFederationSettings",
    ),
    (
        "POST",
        "/setting/config/saveFederationSettings",
        "FederationConfigController.saveFederationSettings",
    ),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/setting/config/federationSettings", get(get_settings))
        .route("/setting/config/saveFederationSettings", post(save_settings))
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

/// `GET /adminapi/setting/config/federationSettings`：读取生效中的联邦设置
/// （未保存过时返回全关默认；总开关关闭时本接口照常可用）。
async fn get_settings(State(state): State<AdminState>) -> ApiResult<Value> {
    let settings = state.hub.federation.get_settings().await?;
    let value = serde_json::to_value(settings)
        .map_err(|e| nextrade_core::Error::Internal(format!("联邦设置序列化失败: {e}")))?;
    ok(value)
}

/// `POST /adminapi/setting/config/saveFederationSettings`：保存联邦设置
/// （整块覆盖，服务层清洗；首次开启时生成并持久化实例 UUID）。
async fn save_settings(
    State(state): State<AdminState>,
    Json(payload): Json<Value>,
) -> ApiResult<Value> {
    let saved = state.hub.federation.save_settings(&payload).await?;
    let value = serde_json::to_value(saved)
        .map_err(|e| nextrade_core::Error::Internal(format!("联邦设置序列化失败: {e}")))?;
    ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract_exactly() {
        let expected = [
            ("GET", "/adminapi/setting/config/federationSettings"),
            ("POST", "/adminapi/setting/config/saveFederationSettings"),
        ];
        let docs = implemented_routes();
        assert_eq!(docs.len(), 2);
        for (doc, (method, path)) in docs.iter().zip(expected.iter()) {
            assert_eq!(doc.method, *method);
            assert_eq!(doc.path, *path);
        }
    }
}
