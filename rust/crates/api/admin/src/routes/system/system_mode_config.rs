//! 管理端 系统模式设置路由（system::system_mode_config，2 条）。
//!
//! 来源标注：**我方新增**（原版无此设置，见 services::system::system_mode
//! 文件头）。平台级全站配置：决定这套系统对外形态——`all` 全功能（默认）/
//! `giftcard` 礼品卡系统 / `mall` 商城系统，存 `config` 表 `systemMode`
//! 一行 JSON，前台经 `GET /api/common/config/systemMode` 消费做入口级
//! 渲染门控。命名与 buyStyle 一致：`GET /systemModeSettings`、
//! `POST /saveSystemMode`。

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
        "/setting/config/systemModeSettings",
        "SystemModeConfigController.getSystemModeSettings",
    ),
    (
        "POST",
        "/setting/config/saveSystemMode",
        "SystemModeConfigController.saveSystemMode",
    ),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/setting/config/systemModeSettings", get(get_settings))
        .route("/setting/config/saveSystemMode", post(save_settings))
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

/// `GET /adminapi/setting/config/systemModeSettings`：读取生效中的系统
/// 模式（未保存过时返回默认 `all`；非法存储值归一化为 `all`）。
async fn get_settings(State(state): State<AdminState>) -> ApiResult<Value> {
    ok(state.system.config_basic.get_system_mode().await?)
}

/// `POST /adminapi/setting/config/saveSystemMode`：保存系统模式
/// （整块覆盖，保存前经服务层归一化）。
async fn save_settings(
    State(state): State<AdminState>,
    Json(payload): Json<Value>,
) -> ApiResult<()> {
    state.system.config_basic.save_system_mode(&payload).await?;
    ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract_exactly() {
        let expected = [
            ("GET", "/adminapi/setting/config/systemModeSettings"),
            ("POST", "/adminapi/setting/config/saveSystemMode"),
        ];
        let docs = implemented_routes();
        assert_eq!(docs.len(), 2);
        for (doc, (method, path)) in docs.iter().zip(expected.iter()) {
            assert_eq!(doc.method, *method);
            assert_eq!(doc.path, *path);
        }
    }
}
