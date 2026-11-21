//! POS 收银域入参/出参（pos::cashier）。
//!
//! ⚠️ 无 Java 参考（POS 仅存在于 o2o.recy.cc 编译前端）。字段契约来源：
//! 前端 bundle（o2ojs/pos-*.js 与相关页面 chunk）+ 真实表结构；
//! 挂单/桌台状态机响应语义为推断实现，需 POS 页面联调验证。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ————————————————————— 收银员管理（admin_user_pos） —————————————————————

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosAdminUserQuery {
    pub page: Option<u64>,
    pub size: Option<u64>,
    pub keyword: Option<String>,
    pub status: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosAdminUserVO {
    pub id: i32,
    pub admin_id: i32,
    pub shop_id: i32,
    pub username: String,
    pub email: String,
    pub avatar: Option<String>,
    pub is_using: i32,
    pub add_time: Option<i32>,
    pub role_id: i32,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosAdminUserWriteDTO {
    pub id: Option<i32>,
    pub admin_id: Option<i32>,
    pub shop_id: Option<i32>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub is_using: Option<i32>,
    pub role_id: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosAdminUserUpdateFieldDTO {
    pub id: i32,
    pub field: Option<String>,
    pub value: Option<serde_json::Value>,
}

// ————————————————————— 上下班 / 统计 —————————————————————

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosWorkStatusVO {
    pub working: bool,
    pub work_time: Option<i64>,
    pub off_time: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosStatisticsVO {
    pub order_count: i64,
    pub total_amount: Decimal,
    pub today: bool,
}

// ————————————————————— 会员 / 游客 —————————————————————

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosCreateMemberDTO {
    pub mobile: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosUserVO {
    pub user_id: i32,
    pub nickname: Option<String>,
    pub mobile: Option<String>,
    pub balance: Option<Decimal>,
    pub points: Option<i32>,
    pub add_time: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosUserQuery {
    pub page: Option<u64>,
    pub size: Option<u64>,
    pub keyword: Option<String>,
    pub user_id: Option<i32>,
}

// ————————————————————— 优惠券 / 充值 —————————————————————

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosCouponQuery {
    pub user_id: Option<i32>,
    pub product_ids: Option<Vec<i32>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosRechargeCreateDTO {
    pub user_id: i32,
    pub money: Decimal,
    pub pay_type: Option<String>,
}

// ————————————————————— 购物车 / 挂单 —————————————————————

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosCartAddDTO {
    pub product_id: i64,
    pub sku_id: Option<i64>,
    pub quantity: Option<i32>,
    pub user_id: Option<i32>,
    pub shop_id: Option<i32>,
    pub table_id: Option<i32>,
    /// 称重商品加行时的称重克数（我方新增）：称重商品必填且须 > 0，
    /// 非称重商品忽略。重量来源（蓝牙秤/手工录入）对后端无区别。
    pub weight_grams: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosCartUpdateDTO {
    pub cart_id: Option<i64>,
    pub item_id: Option<i64>,
    pub quantity: Option<i32>,
    pub is_checked: Option<bool>,
    pub user_id: Option<i32>,
    /// 称重行改重（我方新增）：给出时按行内快照单价/单位重算行金额，
    /// quantity 对称重行无意义、被忽略。
    pub weight_grams: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosCartRemoveDTO {
    pub cart_ids: Option<Vec<i64>>,
    pub item_ids: Option<Vec<i64>>,
    pub user_id: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosCartChangePriceDTO {
    pub cart_id: Option<i64>,
    pub item_id: Option<i64>,
    pub price: Option<Decimal>,
    /// 是否仅改展示价（changePriceShow：不改结算价）。
    pub show_only: Option<bool>,
    /// 整单改价方式：1=一口价（总额），2=折扣（百分比，如 90 表示九折）。
    #[serde(default)]
    pub total_change_price_type: Option<i32>,
    #[serde(default)]
    pub change_price_value: Option<Decimal>,
    #[serde(default)]
    pub user_id: Option<i32>,
    #[serde(default)]
    pub table_id: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosCheckoutDTO {
    pub table_id: Option<i32>,
    pub pay_type_id: Option<i32>,
    pub use_balance: Option<Decimal>,
    pub use_points: Option<i32>,
    pub use_coupon_ids: Option<Vec<i32>>,
    pub remark: Option<String>,
    /// 线下现金收银默认直接记已支付；线上支付传 false。
    pub mark_paid: Option<bool>,
    /// 下单幂等键（我方新增）：收银台每次打开结算弹窗生成一个，
    /// 本次结账的重复点击/超时重试共用，同键只建一单并返回原单。
    /// 缺省 = 不启用幂等（旧端行为不变）。口径见
    /// `models::trade::order::normalize_idempotency_key`。
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosPendingOrderDTO {
    pub remark: Option<String>,
    pub user_id: Option<i32>,
    pub table_id: Option<i32>,
    pub pending_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosCartItemVO {
    pub cart_id: i64,
    pub product_id: i64,
    pub sku_id: i64,
    pub product_name: String,
    pub pic_thumb: String,
    pub price: Decimal,
    pub origin_price: Decimal,
    pub quantity: i32,
    pub subtotal: Decimal,
    pub is_checked: bool,
    pub sku_data: Option<serde_json::Value>,
    /// 是否称重行（我方新增）：true 时 price/subtotal 为整行金额、
    /// quantity 恒 1，重量与计价快照见下三字段；普通行恒 false/None。
    pub is_weighing: bool,
    pub weight_grams: Option<i32>,
    pub weigh_unit_price: Option<Decimal>,
    pub weigh_unit_grams: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosCartVO {
    /// 与 Java 购物车契约同形：店铺分组（收银为单店）。
    pub cart_list: Vec<PosCartStoreVO>,
    pub total: PosCartTotalVO,
    pub pending_id: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosCartStoreVO {
    pub shop_id: i32,
    pub shop_title: String,
    pub carts: Vec<PosCartItemVO>,
    pub is_checked: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosCartTotalVO {
    pub total_count: i64,
    pub checked_count: i64,
    pub product_amount: Decimal,
    pub discount_after: Decimal,
}

// ————————————————————— 桌台 —————————————————————

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosTableQuery {
    pub area_id: Option<i32>,
    pub status: Option<i32>,
    pub shop_id: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosTableWriteDTO {
    pub table_id: Option<i32>,
    pub area_id: Option<i32>,
    pub shop_id: Option<i32>,
    pub table_name: Option<String>,
    pub table_no: Option<String>,
    pub capacity: Option<i32>,
    pub seat_count: Option<i32>,
    pub sort: Option<i32>,
    pub sort_order: Option<i32>,
    pub table_status: Option<i32>,
    pub dining_count: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosTableAreaWriteDTO {
    pub area_id: Option<i32>,
    pub shop_id: Option<i32>,
    pub area_name: Option<String>,
    pub sort: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosTableOperateDTO {
    pub table_id: i32,
    pub user_id: Option<i32>,
    pub target_table_id: Option<i32>,
    pub order_id: Option<i32>,
    pub remark: Option<String>,
    #[serde(default)]
    pub dining_count: Option<i32>,
}

// ————————————————————— 商品 / 订单 —————————————————————

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosProductQuery {
    pub keyword: Option<String>,
    pub category_id: Option<i32>,
    pub page: Option<u64>,
    pub size: Option<u64>,
    pub product_id: Option<i64>,
    pub shop_id: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosOrderPayCreateDTO {
    pub order_id: i32,
    pub pay_type: Option<String>,
}

pub const POS_ADMIN_USER_FIELDS: [&str; 3] = ["username", "email", "is_using"];

// ————————————————————— 称重计价（我方新增） —————————————————————

/// 称重计价单位缺省克数：500（元/500g，即俗称元/斤）。
pub const DEFAULT_WEIGH_UNIT_GRAMS: i32 = 500;

/// 称重行金额（纯函数）：行金额 = 计价单价 × 重量克数 ÷ 计价单位克数，
/// 四舍五入到分（MidpointAwayFromZero，与零售口径一致）。
/// 入参非法（重量 <= 0、单位 <= 0、单价为负）返回 None，调用方据此报错。
/// 前端预览必须与本函数同口径（整数分计算），保证预览与落库金额一致。
#[must_use]
pub fn weigh_line_amount(unit_price: Decimal, weight_grams: i32, unit_grams: i32) -> Option<Decimal> {
    if weight_grams <= 0 || unit_grams <= 0 || unit_price < Decimal::ZERO {
        return None;
    }
    let raw = unit_price * Decimal::from(weight_grams) / Decimal::from(unit_grams);
    Some(raw.round_dp_with_strategy(
        2,
        rust_decimal::RoundingStrategy::MidpointAwayFromZero,
    ))
}

/// 计价单位克数归一：非法值（<= 0）回落缺省 500。
#[must_use]
pub fn normalize_weigh_unit_grams(unit_grams: i32) -> i32 {
    if unit_grams > 0 {
        unit_grams
    } else {
        DEFAULT_WEIGH_UNIT_GRAMS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(text: &str) -> Decimal {
        Decimal::from_str(text).expect("decimal")
    }

    #[test]
    fn weigh_amount_per_500g() {
        // 12.80 元/500g，称 750g → 19.20
        assert_eq!(
            weigh_line_amount(dec("12.80"), 750, 500),
            Some(dec("19.20"))
        );
        // 恰好一个计价单位
        assert_eq!(
            weigh_line_amount(dec("12.80"), 500, 500),
            Some(dec("12.80"))
        );
    }

    #[test]
    fn weigh_amount_per_kg_and_rounding() {
        // 30.00 元/1000g，称 333g → 9.99
        assert_eq!(
            weigh_line_amount(dec("30.00"), 333, 1000),
            Some(dec("9.99"))
        );
        // 10.00 元/500g，称 25g → 0.50；称 24g → 0.48
        assert_eq!(
            weigh_line_amount(dec("10.00"), 25, 500),
            Some(dec("0.50"))
        );
        assert_eq!(
            weigh_line_amount(dec("10.00"), 24, 500),
            Some(dec("0.48"))
        );
        // 半分四舍五入（远离零）：10.05 元/500g 称 250g = 5.025 → 5.03
        assert_eq!(
            weigh_line_amount(dec("10.05"), 250, 500),
            Some(dec("5.03"))
        );
    }

    #[test]
    fn weigh_amount_rejects_invalid() {
        assert_eq!(weigh_line_amount(dec("10.00"), 0, 500), None);
        assert_eq!(weigh_line_amount(dec("10.00"), -5, 500), None);
        assert_eq!(weigh_line_amount(dec("10.00"), 100, 0), None);
        assert_eq!(weigh_line_amount(dec("-1.00"), 100, 500), None);
        // 免费称重（单价 0）合法，金额 0
        assert_eq!(
            weigh_line_amount(dec("0"), 100, 500),
            Some(dec("0.00"))
        );
    }

    #[test]
    fn weigh_unit_normalization() {
        assert_eq!(normalize_weigh_unit_grams(1000), 1000);
        assert_eq!(normalize_weigh_unit_grams(0), DEFAULT_WEIGH_UNIT_GRAMS);
        assert_eq!(normalize_weigh_unit_grams(-1), DEFAULT_WEIGH_UNIT_GRAMS);
    }
}
