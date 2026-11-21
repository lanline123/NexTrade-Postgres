//! 管理端 送礼物设置路由（system::gift_order_config，2 条，我方新增）。
//!
//! 平台级配置：送礼物订单的领取有效期（天），存 `config` 表
//! `giftOrderSettings` 一行 JSON，默认 7 天（合法区间 1..=90，
//! 清洗在 core `GiftOrderSettings::sanitized`）。命名与
//! `setting/config` 域既有口径一致：`GET /giftOrderSettings`、
//! `POST /saveGiftOrderSettings`。仅平台账号可读写（platform_only）。

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
        "/setting/config/giftOrderSettings",
        "GiftOrderConfigController.getGiftOrderSettings",
    ),
    (
        "POST",
        "/setting/config/saveGiftOrderSettings",
        "GiftOrderConfigController.saveGiftOrderSettings",
    ),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/setting/config/giftOrderSettings", get(get_settings))
        .route("/setting/config/saveGiftOrderSettings", post(save_settings))
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

/// `GET /adminapi/setting/config/giftOrderSettings`：读取礼物设置
/// （未保存过时返回默认：领取有效期 7 天）。
async fn get_settings(State(state): State<AdminState>) -> ApiResult<Value> {
    let settings = state.trade.gift_order.settings().await;
    ok(serde_json::json!({
        "claimExpireDays": settings.claim_expire_days,
    }))
}

/// `POST /adminapi/setting/config/saveGiftOrderSettings`：保存礼物设置
/// （天数经服务层清洗钳制后落库，返回落库后的生效值）。
async fn save_settings(
    State(state): State<AdminState>,
    Json(payload): Json<Value>,
) -> ApiResult<Value> {
    let settings = state.trade.gift_order.save_settings(&payload).await?;
    ok(serde_json::json!({
        "claimExpireDays": settings.claim_expire_days,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract_exactly() {
        let expected = [
            ("GET", "/adminapi/setting/config/giftOrderSettings"),
            ("POST", "/adminapi/setting/config/saveGiftOrderSettings"),
        ];
        let docs = implemented_routes();
        assert_eq!(docs.len(), 2);
        for (doc, (method, path)) in docs.iter().zip(expected.iter()) {
            assert_eq!(doc.method, *method);
            assert_eq!(doc.path, *path);
        }
    }
}
