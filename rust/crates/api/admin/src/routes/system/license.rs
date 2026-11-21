//! 管理端 授权状态路由（system::license，1 条）。
//!
//! 来源标注：**我方新增**（对接独立授权中心 auth.recy.cc 的客户端状态口；
//! 与原版 `setting/licensed` 授权码桩无关，那是原 Tigshop 授权模块的残桩，
//! 两者不要混淆）。
//!
//! `GET /adminapi/license/status`：返回本实例缓存的授权判定（数据来自
//! 最近一次验签通过的授权中心 checkin 响应，见
//! `services::system::license`）。从未成功上报过 → `status = "unknown"`。
//! 本接口只读缓存、永不外呼授权中心、永不失败（读不到缓存即 unknown），
//! 管理端徽标与菜单版本控制消费它；fail-soft 口径下它不参与任何拦截。

use axum::{extract::State, routing::get, Router};
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult};
use serde_json::Value;

use crate::AdminState;

/// 本域 1 条路由（`/adminapi` 前缀之后的路径）。
const ROUTE_TABLE: [(&str, &str, &str); 1] = [(
    "GET",
    "/license/status",
    "LicenseClientController.getStatus",
)];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new().route("/license/status", get(get_status))
}

/// 本域已实现路由（完整契约路径，用于覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/adminapi{path}"), handler))
        .collect()
}

/// `GET /adminapi/license/status`：缓存的授权判定（unknown 时前端显示
/// 灰色「授权状态待确认」，不隐藏任何菜单之外的功能、不拦截）。
async fn get_status(State(state): State<AdminState>) -> ApiResult<Value> {
    ok(state.system.license.status().await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract_exactly() {
        let expected = [("GET", "/adminapi/license/status")];
        let docs = implemented_routes();
        assert_eq!(docs.len(), 1);
        for (doc, (method, path)) in docs.iter().zip(expected.iter()) {
            assert_eq!(doc.method, *method);
            assert_eq!(doc.path, *path);
        }
    }
}
