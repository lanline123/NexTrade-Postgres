//! 管理端 价格显示设置路由（system::price_display_config，2 条）。
//!
//! 平台级全站配置：大额商品展示价的单位缩写规则（万/亿、K/M/B、L/Cr 等），
//! 存 `config` 表 `priceDisplay` 一行 JSON，前台经
//! `GET /api/common/config/priceDisplay` 消费。契约为本平台新增
//! （非 Java 迁移端点），与 `setting/config` 域既有命名保持一致：
//! `GET /priceDisplaySettings`、`POST /savePriceDisplay`。

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
        "/setting/config/priceDisplaySettings",
        "PriceDisplayConfigController.getPriceDisplaySettings",
    ),
    (
        "POST",
        "/setting/config/savePriceDisplay",
        "PriceDisplayConfigController.savePriceDisplay",
    ),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/setting/config/priceDisplaySettings", get(get_settings))
        .route("/setting/config/savePriceDisplay", post(save_settings))
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

/// `GET /adminapi/setting/config/priceDisplaySettings`：读取生效中的价格显示配置
/// （未保存过时返回内置默认）。
async fn get_settings(State(state): State<AdminState>) -> ApiResult<Value> {
    ok(state.system.config_basic.get_price_display().await?)
}

/// `POST /adminapi/setting/config/savePriceDisplay`：保存价格显示配置（整块覆盖，
/// 保存前经服务层清洗校验）。
async fn save_settings(
    State(state): State<AdminState>,
    Json(payload): Json<Value>,
) -> ApiResult<()> {
    state.system.config_basic.save_price_display(&payload).await?;
    ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract_exactly() {
        let expected = [
            ("GET", "/adminapi/setting/config/priceDisplaySettings"),
            ("POST", "/adminapi/setting/config/savePriceDisplay"),
        ];
        let docs = implemented_routes();
        assert_eq!(docs.len(), 2);
        for (doc, (method, path)) in docs.iter().zip(expected.iter()) {
            assert_eq!(doc.method, *method);
            assert_eq!(doc.path, *path);
        }
    }
}
