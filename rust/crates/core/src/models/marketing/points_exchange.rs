//! 积分商品域（marketing::points_exchange）DTO / VO / 纯函数。
//! Java `PointsExchangeController`（7 端点，admin）+ `PointsExchangeServiceImpl`。
//! 表 `points_exchange`（9 列，主键 id；迁移 012 起含 exchange_type / coupon_id）。
//!
//! 兑换类型：1 = 商品（原版有），2 = 优惠券（**我方新增**，论坛帖 517907，
//! 用户 2026-11-21 拍板）。券型积分价口径：单品自设（exchange_integral > 0）
//! 优先；未自设（0）时按平台统一比 `integralCouponScale` 折算
//! （未配置则回落现行 `integralScale`，再无则默认 1，即 100 积分 = 1 元，
//! 与现行积分换算习惯一致）。折算纯函数见 [`resolve_coupon_points`]。

use std::collections::HashMap;

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub const COLUMNS: [&str; 9] = [
    "id",
    "product_id",
    "exchange_integral",
    "points_deducted_amount",
    "is_hot",
    "is_enabled",
    "sku_id",
    "exchange_type",
    "coupon_id",
];
pub const KEY_COLUMN: &str = "id";
pub const UPDATE_FIELDS: [&str; 2] = ["is_hot", "is_enabled"];

/// 兑换类型：商品（原版）。
pub const EXCHANGE_TYPE_PRODUCT: i32 = 1;
/// 兑换类型：优惠券（我方新增）。
pub const EXCHANGE_TYPE_COUPON: i32 = 2;

/// 积分价来源标记：单品自设。
pub const INTEGRAL_SOURCE_SELF: &str = "self";
/// 积分价来源标记：平台统一比折算。
pub const INTEGRAL_SOURCE_PLATFORM: &str = "platform";

/// 平台统一比配置键（积分兑券比例：每 100 积分可兑换的券面额，元；
/// 与 `integralScale` 同语义同量纲，落在商城设置「积分设置」组）。
pub const KEY_INTEGRAL_COUPON_SCALE: &str = "integralCouponScale";
/// 现行积分换算比配置键（兑券比未配置时的回落）。
pub const KEY_INTEGRAL_SCALE: &str = "integralScale";
/// 两级比都未配置时的默认比（100 积分 = 1 元）。
pub const DEFAULT_COUPON_SCALE: Decimal = Decimal::ONE;

pub const MSG_PRODUCT_REQUIRED: &str = "请选择积分商品";
pub const MSG_ID_REQUIRED: &str = "积分商品id不能为空";
pub const MSG_DUPLICATE: &str = "已存在相同的积分商品";
pub const MSG_INTEGRAL_INVALID: &str = "兑换积分不能小于或等于0";
pub const MSG_SKU_DISABLED: &str = "商品规格已禁用，请重新选择";
pub const MSG_NOT_EXISTS: &str = "积分商品不存在";
pub const MSG_COUPON_REQUIRED: &str = "请选择要兑换的优惠券";
pub const MSG_COUPON_DUPLICATE: &str = "已存在相同的积分兑换优惠券";
pub const MSG_COUPON_NOT_EXISTS: &str = "优惠券不存在";
pub const MSG_INTEGRAL_NEGATIVE: &str = "兑换积分不能小于0";
pub const MSG_COUPON_FACE_REQUIRED: &str = "该优惠券无固定面额，跟随平台比无法折算积分价，请为该兑换品单独设置兑换积分";

/// 解析配置比值（字符串 → 正 Decimal；空/非法/非正视为未配置）。
#[must_use]
pub fn parse_scale(raw: Option<&str>) -> Option<Decimal> {
    let value = raw?.trim().parse::<Decimal>().ok()?;
    (value > Decimal::ZERO).then_some(value)
}

/// 生效的积分兑券统一比：`integralCouponScale` → `integralScale` → 默认 1。
#[must_use]
pub fn resolve_effective_coupon_scale(configs: &HashMap<String, String>) -> Decimal {
    parse_scale(configs.get(KEY_INTEGRAL_COUPON_SCALE).map(String::as_str))
        .or_else(|| parse_scale(configs.get(KEY_INTEGRAL_SCALE).map(String::as_str)))
        .unwrap_or(DEFAULT_COUPON_SCALE)
}

/// 解析券型兑换品的实际积分价与来源。
///
/// - 单品自设（`stored_integral > 0`）→ 原值，来源 `self`；
/// - 未自设 → 按生效统一比折算：`ceil(券面额 × 100 ÷ scale)`，来源 `platform`；
///   券面额非正（纯折扣券无固定面额）时无法折算，返回 0 由调用方拦截。
#[must_use]
pub fn resolve_coupon_points(stored_integral: i32, coupon_money: Decimal, scale: Decimal) -> (i32, &'static str) {
    if stored_integral > 0 {
        return (stored_integral, INTEGRAL_SOURCE_SELF);
    }
    if coupon_money <= Decimal::ZERO || scale <= Decimal::ZERO {
        return (0, INTEGRAL_SOURCE_PLATFORM);
    }
    let points = (coupon_money * Decimal::from(100) / scale).ceil();
    let points = points.to_i32().unwrap_or(i32::MAX).max(1);
    (points, INTEGRAL_SOURCE_PLATFORM)
}

#[must_use]
pub fn map_order_column(field: &str) -> &'static str {
    match field {
        "id" => "id",
        "productId" | "product_id" => "product_id",
        "exchangeIntegral" | "exchange_integral" => "exchange_integral",
        "pointsDeductedAmount" | "points_deducted_amount" => "points_deducted_amount",
        "isHot" | "is_hot" => "is_hot",
        "isEnabled" | "is_enabled" => "is_enabled",
        "skuId" | "sku_id" => "sku_id",
        "exchangeType" | "exchange_type" => "exchange_type",
        "couponId" | "coupon_id" => "coupon_id",
        _ => KEY_COLUMN,
    }
}

#[must_use]
pub fn order_clause(sort_field: Option<&str>, sort_order: Option<&str>) -> String {
    let field = sort_field.map(str::trim).unwrap_or("");
    let column = if field.is_empty() {
        KEY_COLUMN
    } else {
        map_order_column(field)
    };
    let dir = if sort_order
        .map(|s| s.trim().eq_ignore_ascii_case("asc"))
        .unwrap_or(false)
    {
        "ASC"
    } else {
        "DESC"
    };
    format!(" ORDER BY {column} {dir}")
}

#[must_use]
pub fn limit_offset(page: Option<i64>, size: Option<i64>) -> (i64, i64) {
    let page = page.unwrap_or(1).max(1);
    let size = match size {
        None => 15,
        Some(v) if v > 100 => 100,
        Some(v) if v < 1 => 1,
        Some(v) => v,
    };
    (size, (page - 1) * size)
}

/// `updateField` 白名单（Java `POINTS_EXCHANGE_FIELDS`）。
#[must_use]
pub fn resolve_update_field(field: &str) -> Option<&'static str> {
    let column = map_order_column(field);
    UPDATE_FIELDS.contains(&column).then_some(column)
}

/// 列表查询（Java `PointsExchangeListDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PointsExchangeListQuery {
    pub is_hot: Option<i32>,
    pub is_enabled: Option<i32>,
    pub product_name: Option<String>,
    pub keyword: Option<String>,
    pub page: Option<i64>,
    pub size: Option<i64>,
    pub sort_field: Option<String>,
    pub sort_order: Option<String>,
}

impl PointsExchangeListQuery {
    #[must_use]
    pub fn limit_offset(&self) -> (i64, i64) {
        limit_offset(self.page, self.size)
    }
    #[must_use]
    pub fn order_clause(&self) -> String {
        order_clause(self.sort_field.as_deref(), self.sort_order.as_deref())
    }
    #[must_use]
    pub fn effective_keyword(&self) -> Option<&str> {
        self.keyword
            .as_deref()
            .or(self.product_name.as_deref())
            .filter(|s| !s.is_empty())
    }
}

/// 创建入参（Java `PointsExchangeCreateDTO`；券型字段为我方新增）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PointsExchangeCreateDTO {
    pub product_id: Option<i32>,
    pub exchange_integral: Option<i32>,
    pub points_deducted_amount: Option<Decimal>,
    pub is_hot: Option<i32>,
    pub is_enabled: Option<i32>,
    pub sku_id: Option<i32>,
    /// 兑换类型（缺省 1=商品；2=优惠券，我方新增）
    pub exchange_type: Option<i32>,
    /// 券模板 ID（券型必填）
    pub coupon_id: Option<i32>,
}

/// 校验后的创建参数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointsExchangeCreate {
    pub product_id: i32,
    pub exchange_integral: i32,
    pub points_deducted_amount: Decimal,
    pub is_hot: i32,
    pub is_enabled: i32,
    pub sku_id: Option<i32>,
    pub exchange_type: i32,
    pub coupon_id: Option<i32>,
}

impl PointsExchangeCreateDTO {
    pub fn validate(&self) -> Result<PointsExchangeCreate, String> {
        if self.exchange_type.unwrap_or(EXCHANGE_TYPE_PRODUCT) == EXCHANGE_TYPE_COUPON {
            // 券型（我方新增）：必须选券模板；积分价可空/0 = 跟随平台统一比，
            // 负数非法；商品侧字段一律归零（product_id 存哨兵 0）。
            let coupon_id = self
                .coupon_id
                .filter(|v| *v > 0)
                .ok_or_else(|| MSG_COUPON_REQUIRED.to_string())?;
            let exchange_integral = self.exchange_integral.unwrap_or(0);
            if exchange_integral < 0 {
                return Err(MSG_INTEGRAL_NEGATIVE.to_string());
            }
            return Ok(PointsExchangeCreate {
                product_id: 0,
                exchange_integral,
                points_deducted_amount: Decimal::ZERO,
                is_hot: self.is_hot.unwrap_or(0),
                is_enabled: self.is_enabled.unwrap_or(0),
                sku_id: None,
                exchange_type: EXCHANGE_TYPE_COUPON,
                coupon_id: Some(coupon_id),
            });
        }
        let product_id = self
            .product_id
            .ok_or_else(|| MSG_PRODUCT_REQUIRED.to_string())?;
        let exchange_integral = self.exchange_integral.unwrap_or(0);
        if exchange_integral <= 0 {
            return Err(MSG_INTEGRAL_INVALID.to_string());
        }
        Ok(PointsExchangeCreate {
            product_id,
            exchange_integral,
            points_deducted_amount: self.points_deducted_amount.unwrap_or(Decimal::ZERO),
            is_hot: self.is_hot.unwrap_or(0),
            is_enabled: self.is_enabled.unwrap_or(0),
            sku_id: self.sku_id,
            exchange_type: EXCHANGE_TYPE_PRODUCT,
            coupon_id: None,
        })
    }
}

/// 更新入参（Java `PointsExchangeUpdateDTO`；券型字段为我方新增）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PointsExchangeUpdateDTO {
    pub id: Option<i32>,
    pub product_id: Option<i32>,
    pub exchange_integral: Option<i32>,
    pub points_deducted_amount: Option<Decimal>,
    pub is_hot: Option<i32>,
    pub is_enabled: Option<i32>,
    pub sku_id: Option<i32>,
    /// 兑换类型（缺省 1=商品；2=优惠券，我方新增）
    pub exchange_type: Option<i32>,
    /// 券模板 ID（券型必填）
    pub coupon_id: Option<i32>,
}

/// 校验后的更新参数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointsExchangeUpdate {
    pub id: i32,
    pub product_id: i32,
    pub exchange_integral: i32,
    pub points_deducted_amount: Option<Decimal>,
    pub is_hot: Option<i32>,
    pub is_enabled: Option<i32>,
    pub sku_id: Option<i32>,
    pub exchange_type: i32,
    pub coupon_id: Option<i32>,
}

impl PointsExchangeUpdateDTO {
    pub fn validate(&self) -> Result<PointsExchangeUpdate, String> {
        let id = self.id.ok_or_else(|| MSG_ID_REQUIRED.to_string())?;
        if self.exchange_type.unwrap_or(EXCHANGE_TYPE_PRODUCT) == EXCHANGE_TYPE_COUPON {
            let coupon_id = self
                .coupon_id
                .filter(|v| *v > 0)
                .ok_or_else(|| MSG_COUPON_REQUIRED.to_string())?;
            let exchange_integral = self.exchange_integral.unwrap_or(0);
            if exchange_integral < 0 {
                return Err(MSG_INTEGRAL_NEGATIVE.to_string());
            }
            return Ok(PointsExchangeUpdate {
                id,
                product_id: 0,
                exchange_integral,
                points_deducted_amount: Some(Decimal::ZERO),
                is_hot: self.is_hot,
                is_enabled: self.is_enabled,
                sku_id: None,
                exchange_type: EXCHANGE_TYPE_COUPON,
                coupon_id: Some(coupon_id),
            });
        }
        let product_id = self
            .product_id
            .ok_or_else(|| MSG_PRODUCT_REQUIRED.to_string())?;
        let exchange_integral = self.exchange_integral.unwrap_or(0);
        if exchange_integral <= 0 {
            return Err(MSG_INTEGRAL_INVALID.to_string());
        }
        Ok(PointsExchangeUpdate {
            id,
            product_id,
            exchange_integral,
            points_deducted_amount: self.points_deducted_amount,
            is_hot: self.is_hot,
            is_enabled: self.is_enabled,
            sku_id: self.sku_id,
            exchange_type: EXCHANGE_TYPE_PRODUCT,
            coupon_id: None,
        })
    }
}

/// 删除入参（Java `OperateDTO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OperateDTO {
    pub id: Option<i32>,
    pub admin_note: Option<String>,
    pub page_type: Option<i32>,
}

impl OperateDTO {
    pub fn validate_id(&self) -> Result<i32, String> {
        self.id.ok_or_else(|| "id不能为空".to_string())
    }
}

/// 字段更新入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UpdateFieldDTO {
    pub id: Option<i32>,
    pub field: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::models::serde_helpers::deserialize_opt_stringish"
    )]
    pub val: Option<String>,
}

/// 校验后的字段更新。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldUpdate {
    pub id: i32,
    pub column: &'static str,
    pub val: String,
}

impl UpdateFieldDTO {
    pub fn validate(&self) -> Result<FieldUpdate, String> {
        let id = self.id.ok_or_else(|| "id不能为空".to_string())?;
        let field = self
            .field
            .as_deref()
            .ok_or_else(|| "字段名不能为空".to_string())?;
        let val = self
            .val
            .as_deref()
            .ok_or_else(|| "值不能为空".to_string())?;
        let column = resolve_update_field(field).ok_or_else(|| "field 的值有误".to_string())?;
        Ok(FieldUpdate {
            id,
            column,
            val: val.to_string(),
        })
    }
}

/// 批量操作入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BatchDTO {
    pub ids: Option<Vec<i32>>,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub value: Option<i32>,
}

/// 积分商品 VO（Java `PointsExchangeVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PointsExchangeVO {
    pub id: i32,
    pub product_id: i32,
    pub exchange_integral: i32,
    pub points_deducted_amount: Decimal,
    pub is_hot: i32,
    pub is_enabled: i32,
    pub sku_id: Option<i32>,
    pub product_name: Option<String>,
    pub pic_thumb: Option<String>,
    pub pic_url: Option<String>,
    pub market_price: Option<Decimal>,
    pub cost_price: Option<Decimal>,
    pub product_price: Option<Decimal>,
    pub discounts_price: Option<Decimal>,
    pub product_stock: Option<i32>,
    pub virtual_sales: Option<i32>,
    pub product_sn: Option<String>,
    /// 兑换类型（1=商品 2=优惠券，我方新增）
    pub exchange_type: i32,
    /// 券模板 ID（券型行）
    pub coupon_id: Option<i32>,
    /// 券模板名称（券型行回填）
    pub coupon_name: Option<String>,
    /// 券面额（券型行回填，coupon.coupon_money）
    pub coupon_money: Option<Decimal>,
    /// 折扣（券型行回填，coupon.coupon_discount；折扣券面额基准）
    pub coupon_discount: Option<Decimal>,
    /// 使用门槛（券型行回填，coupon.min_order_amount）
    pub min_order_amount: Option<Decimal>,
    /// 券归属店铺（券型行回填，coupon.shop_id；0=平台券）
    pub coupon_shop_id: Option<i32>,
    /// 实际生效积分价（券型 = 自设或平台比折算；商品型 = exchange_integral）
    pub effective_exchange_integral: i32,
    /// 积分价来源（券型行：self=单品自设 / platform=平台统一比折算）
    pub integral_source: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_validation_messages() {
        let dto = PointsExchangeCreateDTO::default();
        assert_eq!(dto.validate().unwrap_err(), MSG_PRODUCT_REQUIRED);
        let dto = PointsExchangeCreateDTO {
            product_id: Some(3),
            ..Default::default()
        };
        assert_eq!(dto.validate().unwrap_err(), MSG_INTEGRAL_INVALID);
        let dto = PointsExchangeCreateDTO {
            product_id: Some(3),
            exchange_integral: Some(10),
            points_deducted_amount: Some(Decimal::new(500, 2)),
            ..Default::default()
        };
        let input = dto.validate().unwrap();
        assert_eq!(input.product_id, 3);
        assert_eq!(input.exchange_integral, 10);
        assert_eq!(input.points_deducted_amount, Decimal::new(500, 2));
        assert_eq!(input.is_hot, 0);
    }

    #[test]
    fn update_validation_messages() {
        let dto = PointsExchangeUpdateDTO::default();
        assert_eq!(dto.validate().unwrap_err(), MSG_ID_REQUIRED);
        let dto = PointsExchangeUpdateDTO {
            id: Some(1),
            ..Default::default()
        };
        assert_eq!(dto.validate().unwrap_err(), MSG_PRODUCT_REQUIRED);
        let dto = PointsExchangeUpdateDTO {
            id: Some(1),
            product_id: Some(2),
            exchange_integral: Some(-1),
            ..Default::default()
        };
        assert_eq!(dto.validate().unwrap_err(), MSG_INTEGRAL_INVALID);
    }

    #[test]
    fn coupon_create_validation() {
        // 券型未选券模板 → 报错；选了券且积分留空 → 跟随平台（存 0）。
        let dto = PointsExchangeCreateDTO {
            exchange_type: Some(EXCHANGE_TYPE_COUPON),
            ..Default::default()
        };
        assert_eq!(dto.validate().unwrap_err(), MSG_COUPON_REQUIRED);
        let dto = PointsExchangeCreateDTO {
            exchange_type: Some(EXCHANGE_TYPE_COUPON),
            coupon_id: Some(9),
            exchange_integral: Some(-5),
            ..Default::default()
        };
        assert_eq!(dto.validate().unwrap_err(), MSG_INTEGRAL_NEGATIVE);
        let dto = PointsExchangeCreateDTO {
            exchange_type: Some(EXCHANGE_TYPE_COUPON),
            coupon_id: Some(9),
            ..Default::default()
        };
        let input = dto.validate().unwrap();
        assert_eq!(input.exchange_type, EXCHANGE_TYPE_COUPON);
        assert_eq!(input.coupon_id, Some(9));
        assert_eq!(input.exchange_integral, 0);
        assert_eq!(input.product_id, 0);
        assert_eq!(input.points_deducted_amount, Decimal::ZERO);
    }

    #[test]
    fn coupon_points_resolution() {
        // 自设优先。
        assert_eq!(
            resolve_coupon_points(500, Decimal::new(1000, 2), Decimal::ONE),
            (500, INTEGRAL_SOURCE_SELF)
        );
        // 平台比折算：面额 10 元、比 2（100 积分 = 2 元）→ 500 积分。
        assert_eq!(
            resolve_coupon_points(0, Decimal::new(1000, 2), Decimal::new(200, 2)),
            (500, INTEGRAL_SOURCE_PLATFORM)
        );
        // 向上取整：面额 10 元、比 3 → 333.33… → 334。
        assert_eq!(
            resolve_coupon_points(0, Decimal::new(1000, 2), Decimal::new(300, 2)),
            (334, INTEGRAL_SOURCE_PLATFORM)
        );
        // 无固定面额（纯折扣券）未自设 → 0（调用方拦截）。
        assert_eq!(
            resolve_coupon_points(0, Decimal::ZERO, Decimal::ONE),
            (0, INTEGRAL_SOURCE_PLATFORM)
        );
    }

    #[test]
    fn coupon_scale_fallback_chain() {
        let mut configs = HashMap::new();
        // 都未配置 → 默认 1。
        assert_eq!(resolve_effective_coupon_scale(&configs), Decimal::ONE);
        // 仅现行积分比 → 回落它。
        configs.insert(KEY_INTEGRAL_SCALE.to_string(), "2.5".to_string());
        assert_eq!(
            resolve_effective_coupon_scale(&configs),
            Decimal::new(250, 2)
        );
        // 兑券比优先于现行积分比。
        configs.insert(KEY_INTEGRAL_COUPON_SCALE.to_string(), "4".to_string());
        assert_eq!(resolve_effective_coupon_scale(&configs), Decimal::new(400, 2));
        // 非法/非正视为未配置。
        configs.insert(KEY_INTEGRAL_COUPON_SCALE.to_string(), "0".to_string());
        assert_eq!(
            resolve_effective_coupon_scale(&configs),
            Decimal::new(250, 2)
        );
    }

    #[test]
    fn field_update_whitelist_and_sorting() {
        let dto = UpdateFieldDTO {
            id: Some(1),
            field: Some("isHot".into()),
            val: Some("1".into()),
        };
        assert_eq!(dto.validate().unwrap().column, "is_hot");
        assert_eq!(resolve_update_field("is_enabled"), Some("is_enabled"));
        assert_eq!(resolve_update_field("exchangeIntegral"), None);
        assert_eq!(limit_offset(None, None), (15, 0));
        assert_eq!(limit_offset(Some(2), Some(10)), (10, 10));
        assert_eq!(order_clause(None, None), " ORDER BY id DESC");
        assert_eq!(
            order_clause(Some("x; DROP TABLE points_exchange"), None),
            " ORDER BY id DESC"
        );
    }
}
