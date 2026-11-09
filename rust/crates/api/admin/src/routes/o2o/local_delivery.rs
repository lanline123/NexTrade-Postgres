//! 同城配送配置路由（o2o::local_delivery）。
//!
//! Java 来源（overseas）：`ShopLocalDeliveryConfigMapper` +
//! `bean/model/o2o/ShopLocalDeliveryConfig.java`；overseas 无对应控制器，
//! 路由以线上前端契约为准（`java/view/admin/src/api/setting/intraCityDelivery.ts`）：
//! `GET o2o/localDelivery/getConfig?shopId=`。
//!
//! 语义同 `home_service`：读 `shop_local_delivery_config`；无行返回空对象。
//! 平台级总控（`o2o/intraCityDelivery/platformConfig/*`）为 setting/config 域，
//! **不在**本任务范围（见任务说明：不实现 setting/config）。

use axum::{
    extract::{Query, State},
    routing::get,
    Extension, Json, Router,
};
use serde_json::Value;
use nextrade_core::models::o2o::config::ShopConfigQuery;
use nextrade_security::web::{ApiResult, TenantContext};

use crate::AdminState;

/// 本域路由。
const ROUTE_TABLE: [(&str, &str, &str); 1] = [(
    "GET",
    "/o2o/localDelivery/getConfig",
    "ShopLocalDeliveryController.getConfig",
)];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new().route("/o2o/localDelivery/getConfig", get(get_config))
}

/// 本域已实现路由（覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, java_ref)| {
            nextrade_openapi::RouteDoc::new(method, &format!("/adminapi{path}"), java_ref)
        })
        .collect()
}

/// `GET /adminapi/o2o/localDelivery/getConfig`。
async fn get_config(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Query(q): Query<ShopConfigQuery>,
) -> ApiResult<Value> {
    let shop_id =
        nextrade_services::o2o::config::O2oConfigService::resolve_shop_id(&q, tenant.shop_scope());
    let vo = state.o2o.config.local_delivery_config(shop_id).await?;
    Ok(Json(nextrade_core::error::ApiResponse::success(
        vo.map(|v| serde_json::to_value(v).unwrap_or(Value::Null))
            .unwrap_or(Value::Object(serde_json::Map::new())),
    )))
}
