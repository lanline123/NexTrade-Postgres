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
    /// 门店默认服务费（骑手 MVP 我方新增）：0=固定（元/件），1=按比例（%）。
    pub default_service_fee_type: i32,
    pub default_service_fee_value: rust_decimal::Decimal,
    /// 距离溢价规则行（此前 GET 不回此字段、前端恒空数组；现补齐往返保真）。
    pub distance_pricing_list: Vec<DistancePricingVO>,
    pub add_time: i32,
}

/// 距离溢价规则行（GET/SAVE 共用）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistancePricingVO {
    /// 超过多少公里起算。
    pub distance_threshold: rust_decimal::Decimal,
    /// 每增加多少公里（数值；历史上前端以字符串提交，服务层兼容解析）。
    pub distance_unit: rust_decimal::Decimal,
    /// 每单位增加费用（元）。
    pub additional_fee: rust_decimal::Decimal,
    pub sort_order: i32,
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
    /// 门店默认服务费（骑手 MVP 我方新增）：0=固定（元/件），1=按比例（%）。
    pub default_service_fee_type: i32,
    pub default_service_fee_value: rust_decimal::Decimal,
    /// 距离溢价规则行（此前 GET 不回此字段、前端恒空数组；现补齐往返保真）。
    pub distance_pricing_list: Vec<DistancePricingVO>,
    /// 是否开启送达时间选择（我方新增，论坛帖 467744，迁移 025）：0-关闭，1-开启。
    pub time_slot_enabled: i32,
    /// 送达时段定义（开启时段选择时生效；capacity=0 不限量）。
    pub time_slots: Vec<crate::models::trade::delivery_slot::DeliverySlotDef>,
    /// 可预约天数（含今日，1=仅今日，上限 7）。
    pub booking_days: i32,
    /// 截单提前量（分钟）：今日时段在开始前该分钟数停止接单。
    pub cutoff_minutes: i32,
    pub add_time: i32,
}

/// `POST /adminapi/o2o/localDelivery/saveConfig` 入参（前端整表提交）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopLocalDeliverySaveParam {
    /// 平台账号代运营时可传；门店账号忽略（以租户作用域为准）。
    pub shop_id: Option<i32>,
    pub enabled: Option<i32>,
    pub delivery_radius: Option<rust_decimal::Decimal>,
    pub min_order_amount: Option<rust_decimal::Decimal>,
    pub base_delivery_fee: Option<rust_decimal::Decimal>,
    pub distance_pricing_enabled: Option<i32>,
    pub order_dispatch_mode: Option<i32>,
    pub pickup_photo_enabled: Option<i32>,
    pub delivery_complete_photo_enabled: Option<i32>,
    pub default_service_fee_type: Option<i32>,
    pub default_service_fee_value: Option<rust_decimal::Decimal>,
    pub distance_pricing_list: Option<Vec<DistancePricingSaveRow>>,
    /// 送达时间选择开关（我方新增，迁移 025）。
    pub time_slot_enabled: Option<i32>,
    /// 送达时段定义全量替换（提交时给定则以提交为准）。
    pub time_slots: Option<Vec<crate::models::trade::delivery_slot::DeliverySlotDef>>,
    /// 可预约天数（含今日，上限 7）。
    pub booking_days: Option<i32>,
    /// 截单提前量（分钟，>= 0）。
    pub cutoff_minutes: Option<i32>,
}

/// `POST /adminapi/o2o/homeService/saveConfig` 入参（前端整表提交）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopHomeServiceSaveParam {
    /// 平台账号代运营时可传；门店账号忽略（以租户作用域为准）。
    pub shop_id: Option<i32>,
    pub enabled: Option<i32>,
    pub service_radius: Option<rust_decimal::Decimal>,
    pub home_fee: Option<rust_decimal::Decimal>,
    pub distance_pricing_enabled: Option<i32>,
    pub order_dispatch_mode: Option<i32>,
    pub before_service_photo_enabled: Option<i32>,
    pub after_service_photo_enabled: Option<i32>,
    pub default_service_fee_type: Option<i32>,
    pub default_service_fee_value: Option<rust_decimal::Decimal>,
    pub distance_pricing_list: Option<Vec<DistancePricingSaveRow>>,
}

/// 保存入参的距离溢价行：`distanceUnit` 历史上前端提交字符串（如 "1"），
/// 服务层兼容字符串/数值两种形态。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistancePricingSaveRow {
    pub distance_threshold: Option<rust_decimal::Decimal>,
    #[serde(default, deserialize_with = "de_decimal_flex")]
    pub distance_unit: Option<rust_decimal::Decimal>,
    pub additional_fee: Option<rust_decimal::Decimal>,
    pub sort_order: Option<i32>,
}

/// Decimal 兼容反序列化：数值或字符串均可。
fn de_decimal_flex<'de, D>(deserializer: D) -> Result<Option<rust_decimal::Decimal>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error as _;
    let v = Option::<serde_json::Value>::deserialize(deserializer)?;
    match v {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(n)) => n
            .to_string()
            .parse::<rust_decimal::Decimal>()
            .map(Some)
            .map_err(D::Error::custom),
        Some(serde_json::Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() {
                Ok(None)
            } else {
                t.parse::<rust_decimal::Decimal>()
                    .map(Some)
                    .map_err(D::Error::custom)
            }
        }
        Some(_) => Err(D::Error::custom("distanceUnit 须为数值或字符串")),
    }
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
