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
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PosCartUpdateDTO {
    pub cart_id: Option<i64>,
    pub item_id: Option<i64>,
    pub quantity: Option<i32>,
    pub is_checked: Option<bool>,
    pub user_id: Option<i32>,
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
