//! 前台 系统模式配置路由（system::system_mode_client，1 条）。
//!
//! 来源标注：**我方新增**（非 Java 契约）：`GET /api/common/config/systemMode`
//! 返回后台「系统模式」设置的生效配置，供 PC / H5 / admin 做入口级渲染
//! 门控。未配置时返回默认 `{mode:"all"}`。后端不按模式拦截业务接口：
//! 已发礼品卡的兑换 / 抵扣与已有订单在任何模式下可用（存量兜底）。

use axum::{extract::State, routing::get, Router};
use serde_json::Value;
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult};

use crate::PublicState;

/// 本域 1 条路由（`/api` 前缀之后的路径）。
const ROUTE_TABLE: [(&str, &str, &str); 1] = [(
    "GET",
    "/common/config/systemMode",
    "SystemModeController.getSystemMode",
)];

pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new().route("/common/config/systemMode", get(get_system_mode))
}

/// 本域已实现路由（完整契约路径，用于覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/api{path}"), handler))
        .collect()
}

/// `GET /api/common/config/systemMode`：系统模式配置（公开只读）。
async fn get_system_mode(State(state): State<PublicState>) -> ApiResult<Value> {
    ok(state.system.config_client.system_mode().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract_exactly() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].method, "GET");
        assert_eq!(docs[0].path, "/api/common/config/systemMode");
    }
}
