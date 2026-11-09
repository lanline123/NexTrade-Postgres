//! 门店级 o2o 配置读接口 DTO（上门服务 / 同城配送 / 到店自提模板）。
//!
//! Java 来源（overseas）：
//! - `com.nextrade.bean.model.o2o.ShopHomeServiceConfig` / `ShopLocalDeliveryConfig`
//! - `com.nextrade.bean.model.o2o.ShopPickupTpl` / `ShopPickupConfig`
//! - 前端契约：`java/view/admin/src/types/setting/homeService.d.ts`、
//!   `intraCityDelivery.d.ts`、`pickupSetting.d.ts`

use serde::{Deserialize, Serialize};

/// `GET /adminapi/o2o/homeService/getConfig` 查询参数。
///
/// 平台管理员可传 `shopId`（0 = 平台级）；门店账号自动取自己。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopConfigQuery {
    pub shop_id: Option<i32>,
}

/// `GET /adminapi/o2o/homeService/getConfig` 响应。
///
/// 与 Java `ShopHomeServiceConfig` 实体字段一一对应（camelCase）；
/// 未配置时由服务层返回空对象（前端按默认值处理）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopHomeServiceConfigVO {
    pub shop_home_service_config_id: i32,
    pub shop_id: i32,
    /// 0-关闭，1-开启。
    pub enabled: i32,
    pub service_radius: Option<rust_decimal::Decimal>,
    pub home_fee: rust_decimal::Decimal,
    pub distance_pricing_enabled: i32,
    /// 1-抢单模式，2-手动派单。
    pub order_dispatch_mode: i32,
    pub before_service_photo_enabled: i32,
    pub after_service_photo_enabled: i32,
    pub add_time: i32,
}

/// `GET /adminapi/o2o/localDelivery/getConfig` 响应（Java `ShopLocalDeliveryConfig`）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopLocalDeliveryConfigVO {
    pub shop_local_delivery_config_id: Option<i64>,
    pub shop_id: i32,
    pub enabled: i32,
    pub delivery_radius: rust_decimal::Decimal,
    pub min_order_amount: rust_decimal::Decimal,
    pub base_delivery_fee: rust_decimal::Decimal,
    pub distance_pricing_enabled: i32,
    pub order_dispatch_mode: i32,
    pub pickup_photo_enabled: i32,
    pub delivery_complete_photo_enabled: i32,
    pub add_time: i32,
}

/// `GET /adminapi/o2o/shopPickupConfig/getShopPickupTpl` 响应。
///
/// 对齐前端 `pickupSettingData`：三个 JSON 子对象解析后透传（非法 JSON 为 null），
/// `status` 取 `shop_pickup_config.status`（无配置行时 1 启用）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopPickupTplVO {
    pub shop_pickup_tpl_id: Option<i64>,
    pub shop_id: i64,
    pub tpl_name: Option<String>,
    pub stocking_status: i32,
    pub stocking_json: Option<serde_json::Value>,
    pub pickup_time_status: i32,
    pub pickup_time_json: Option<serde_json::Value>,
    pub pickup_end_status: i32,
    pub pickup_end_json: Option<serde_json::Value>,
    pub is_default: i32,
    /// 来自 `shop_pickup_config.status`：0-禁用，1-启用。
    pub status: i32,
}

/// 解析 text 列里的 JSON；空 / 非法 → `None`（对齐 Java `parseDeliveryInfo` 吞异常语义）。
#[must_use]
pub fn parse_json_col(raw: Option<&str>) -> Option<serde_json::Value> {
    let s = raw?.trim();
    if s.is_empty() {
        return None;
    }
    serde_json::from_str(s).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_col_rejects_invalid() {
        assert!(parse_json_col(None).is_none());
        assert!(parse_json_col(Some("")).is_none());
        assert!(parse_json_col(Some("not-json")).is_none());
        assert!(parse_json_col(Some(r#"{"a":1}"#)).is_some());
    }
}
