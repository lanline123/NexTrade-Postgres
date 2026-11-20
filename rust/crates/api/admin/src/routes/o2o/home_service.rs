//! 预约上门服务配置路由（o2o::home_service）。
//!
//! Java 来源（overseas）：`ShopHomeServiceConfigMapper` +
//! `bean/model/o2o/ShopHomeServiceConfig.java`；overseas 无对应控制器，
//! 路由以线上前端契约为准（`java/view/admin/src/api/setting/homeService.ts`）：
//! `GET o2o/homeService/getConfig?shopId=`。
//!
//! 语义：读 `shop_home_service_config`（按 shop_id）；无配置行返回**空对象**
//! （前端按默认值渲染）。`saveConfig` 写接口由骑手 MVP 配置双轨统一补齐
//! （2026-11-20，此前由 lowfreq_extra 的 cfg_set 占位接管、写读两轨）。

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use serde_json::Value;
use nextrade_core::models::o2o::config::{ShopConfigQuery, ShopHomeServiceSaveParam};
use nextrade_security::web::{ApiResult, AppError, TenantContext};

use crate::AdminState;

/// 本域路由。
const ROUTE_TABLE: [(&str, &str, &str); 2] = [
    (
        "GET",
        "/o2o/homeService/getConfig",
        "ShopHomeServiceConfigController.getConfig",
    ),
    (
        "POST",
        "/o2o/homeService/saveConfig",
        "ShopHomeServiceConfigController.saveConfig",
    ),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/o2o/homeService/getConfig", get(get_config))
        .route("/o2o/homeService/saveConfig", post(save_config))
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
        nextrade_services::o2o::config::O2oConfigService::resolve_shop_id(&q, tenant.scoped_shop_id());
    let vo = state.o2o.config.home_service_config(shop_id).await?;
    Ok(Json(nextrade_core::error::ApiResponse::success(
        vo.map(|v| serde_json::to_value(v).unwrap_or(Value::Null))
            .unwrap_or(Value::Object(serde_json::Map::new())),
    )))
}

/// `POST /adminapi/o2o/homeService/saveConfig`（骑手 MVP 配置双轨统一新增；
/// 作用域口径同 localDelivery.saveConfig：门店只存本店，平台用 body.shopId）。
async fn save_config(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<ShopHomeServiceSaveParam>,
) -> ApiResult<Value> {
    let shop_id = if tenant.is_platform() {
        body.shop_id.unwrap_or(0)
    } else {
        tenant.require_shop().map_err(AppError)?
    };
    let now = chrono::Utc::now().timestamp() as i32;
    state.o2o.config.save_home_service(shop_id, &body, now).await?;
    Ok(Json(nextrade_core::error::ApiResponse::success(
        Value::Null,
    )))
}
