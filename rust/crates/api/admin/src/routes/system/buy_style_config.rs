//! 管理端 购买按钮样式设置路由（system::buy_style_config，2 条）。
//!
//! 平台级全站配置：PC 装修商品类模块购买按钮的统一枚举（0=隐藏、1–8 沿用
//! 旧版 ProductButton、9=圆圈购物车、10=自定义图片）之全局默认值，存
//! `config` 表 `buyStyle` 一行 JSON，前台经 `GET /api/common/config/buyStyle`
//! 消费（模块 buyBtnStyle=-1「跟随全局」时解析到这里）。契约为本平台新增
//! （非 Java 迁移端点），与价格显示设置的命名保持一致：
//! `GET /buyStyleSettings`、`POST /saveBuyStyle`。

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
        "/setting/config/buyStyleSettings",
        "BuyStyleConfigController.getBuyStyleSettings",
    ),
    (
        "POST",
        "/setting/config/saveBuyStyle",
        "BuyStyleConfigController.saveBuyStyle",
    ),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/setting/config/buyStyleSettings", get(get_settings))
        .route("/setting/config/saveBuyStyle", post(save_settings))
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

/// `GET /adminapi/setting/config/buyStyleSettings`：读取生效中的购买按钮
/// 样式配置（未保存过时返回内置默认）。
async fn get_settings(State(state): State<AdminState>) -> ApiResult<Value> {
    ok(state.system.config_basic.get_buy_style().await?)
}

/// `POST /adminapi/setting/config/saveBuyStyle`：保存购买按钮样式配置
/// （整块覆盖，保存前经服务层清洗校验）。
async fn save_settings(
    State(state): State<AdminState>,
    Json(payload): Json<Value>,
) -> ApiResult<()> {
    state.system.config_basic.save_buy_style(&payload).await?;
    ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract_exactly() {
        let expected = [
            ("GET", "/adminapi/setting/config/buyStyleSettings"),
            ("POST", "/adminapi/setting/config/saveBuyStyle"),
        ];
        let docs = implemented_routes();
        assert_eq!(docs.len(), 2);
        for (doc, (method, path)) in docs.iter().zip(expected.iter()) {
            assert_eq!(doc.method, *method);
            assert_eq!(doc.path, *path);
        }
    }
}
