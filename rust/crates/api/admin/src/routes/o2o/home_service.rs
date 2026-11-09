//! 预约上门服务配置路由（o2o::home_service）。
//!
//! Java 来源（overseas）：`ShopHomeServiceConfigMapper` +
//! `bean/model/o2o/ShopHomeServiceConfig.java`；overseas 无对应控制器，
//! 路由以线上前端契约为准（`java/view/admin/src/api/setting/homeService.ts`）：
//! `GET o2o/homeService/getConfig?shopId=`。
//!
//! 语义：读 `shop_home_service_config`（按 shop_id）；无配置行返回**空对象**
//! （前端按默认值渲染）。`saveConfig`（写接口）未在本最小实现范围内。

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
    "/o2o/homeService/getConfig",
    "ShopHomeServiceConfigController.getConfig",
)];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new().route("/o2o/homeService/getConfig", get(get_config))
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

/// `GET /adminapi/o2o/homeService/getConfig`。
async fn get_config(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Query(q): Query<ShopConfigQuery>,
) -> ApiResult<Value> {
    let shop_id =
        nextrade_services::o2o::config::O2oConfigService::resolve_shop_id(&q, tenant.shop_scope());
    let vo = state.o2o.config.home_service_config(shop_id).await?;
    Ok(Json(nextrade_core::error::ApiResponse::success(
        vo.map(|v| serde_json::to_value(v).unwrap_or(Value::Null))
            .unwrap_or(Value::Object(serde_json::Map::new())),
    )))
}
