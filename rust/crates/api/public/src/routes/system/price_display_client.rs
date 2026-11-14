//! 前台 价格显示配置路由（system::price_display_client，1 条）。
//!
//! 本平台新增端点（非 Java 契约）：`GET /api/common/config/priceDisplay`
//! 返回后台「价格显示设置」的生效配置（大额展示价单位缩写规则），
//! 供 PC 前台 layout 预取并注入价格格式化。未配置时返回内置默认。
//! 与 `config_client` 的 Java 契约文件分开，避免污染其 10 条契约测试。

use axum::{extract::State, routing::get, Router};
use serde_json::Value;
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult};

use crate::PublicState;

/// 本域 1 条路由（`/api` 前缀之后的路径）。
const ROUTE_TABLE: [(&str, &str, &str); 1] = [(
    "GET",
    "/common/config/priceDisplay",
    "PriceDisplayController.getPriceDisplay",
)];

pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new().route("/common/config/priceDisplay", get(get_price_display))
}

/// 本域已实现路由（完整契约路径，用于覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/api{path}"), handler))
        .collect()
}

/// `GET /api/common/config/priceDisplay`：价格显示配置（公开只读）。
async fn get_price_display(State(state): State<PublicState>) -> ApiResult<Value> {
    ok(state.system.config_client.price_display().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract_exactly() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].method, "GET");
        assert_eq!(docs[0].path, "/api/common/config/priceDisplay");
    }
}
