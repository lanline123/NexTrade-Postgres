//! 物流相关实体：`logistics_company` / `logistics_company_config` /
//! `shipping_tpl` / `shipping_tpl_info` / `shipping_type`。
//! 对应 `com.nextrade.bean.model.setting.*`。
//!
//! TODO(域实现者)：对照 `java/database/pure.sql` 中各表的完整列补全字段。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 物流公司。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct LogisticsCompany {
    pub company_id: i32,
    pub company_name: Option<String>,
    pub is_show: Option<i32>,
}

impl LogisticsCompany {
    pub const TABLE: &'static str = "logistics_company";
}

/// 运费模板。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShippingTpl {
    pub tpl_id: i32,
    pub tpl_name: Option<String>,
    /// 计费方式：1 按重量 2 按件数
    pub calc_type: Option<i32>,
    pub add_time: Option<i64>,
}

impl ShippingTpl {
    pub const TABLE: &'static str = "shipping_tpl";

    pub const CALC_BY_WEIGHT: i32 = 1;
    pub const CALC_BY_COUNT: i32 = 2;
}

/// 运费模板明细（区域运费）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShippingTplInfo {
    pub tpl_info_id: i32,
    pub tpl_id: Option<i32>,
    /// 首件/首重费用
    pub first_price: Option<Decimal>,
    /// 续件/续重费用
    pub next_price: Option<Decimal>,
}

impl ShippingTplInfo {
    pub const TABLE: &'static str = "shipping_tpl_info";
}

/// 配送方式。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShippingType {
    pub type_id: i32,
    pub type_name: Option<String>,
    pub is_show: Option<i32>,
}

impl ShippingType {
    pub const TABLE: &'static str = "shipping_type";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calc_type_consts() {
        assert_eq!(ShippingTpl::CALC_BY_WEIGHT, 1);
        assert_eq!(ShippingTpl::CALC_BY_COUNT, 2);
    }
}