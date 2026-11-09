//! 到店自提配置路由（o2o::shop_pickup）。
//!
//! Java 来源（overseas）：`bean/model/o2o/ShopPickupTpl.java` /
//! `ShopPickupConfig.java` + 对应 Mapper；overseas 无控制器，路由以线上前端
//! 契约为准（`java/view/admin/src/api/setting/pickupSetting.ts`）：
//! `GET o2o/shopPickupConfig/getShopPickupTpl?shopId=`。
//!
//! 语义：门店级模板 → 平台默认模板（`shop_id=0, is_default=1`）回退；
//! 无任何模板行时返回空对象。

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
    "/o2o/shopPickupConfig/getShopPickupTpl",
    "ShopPickupConfigController.getShopPickupTpl",
)];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new().route(
        "/o2o/shopPickupConfig/getShopPickupTpl",
        get(get_shop_pickup_tpl),
    )
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

/// `GET /adminapi/o2o/shopPickupConfig/getShopPickupTpl`。
async fn get_shop_pickup_tpl(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Query(q): Query<ShopConfigQuery>,
) -> ApiResult<Value> {
    let shop_id =
        nextrade_services::o2o::config::O2oConfigService::resolve_shop_id(&q, tenant.shop_scope());
    let vo = state.o2o.config.shop_pickup_tpl(shop_id).await?;
    Ok(Json(nextrade_core::error::ApiResponse::success(
        vo.map(|v| serde_json::to_value(v).unwrap_or(Value::Null))
            .unwrap_or(Value::Object(serde_json::Map::new())),
    )))
}
