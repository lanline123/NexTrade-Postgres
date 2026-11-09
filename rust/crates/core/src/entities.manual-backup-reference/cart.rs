//! 购物车实体 `cart`。
//! 对应 `com.nextrade.bean.model.cart.Cart`。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Cart {
    pub cart_id: Option<i32>,
    pub user_id: Option<i32>,
    pub product_id: Option<i32>,
    pub product_sn: Option<String>,
    pub pic_thumb: Option<String>,
    /// 市场价（加入购物车时价格快照）
    pub market_price: Option<Decimal>,
    /// 原价
    pub original_price: Option<Decimal>,
    /// 数量
    pub quantity: Option<i32>,
    pub sku_id: Option<i32>,
    /// SKU 规格 JSON
    pub sku_data: Option<String>,
    /// 改价数据 JSON
    pub change_price_data: Option<String>,
    pub product_type: Option<i32>,
    /// 是否选中结算：1 是 0 否
    pub is_checked: Option<i32>,
    pub shop_id: Option<i32>,
    /// 购物车行类型
    #[sqlx(rename = "type")]
    pub r#type: Option<i32>,
    pub update_time: Option<i64>,
    pub cycle_spec_id: Option<i32>,
    pub redeem_card_id: Option<i32>,
    pub salesman_id: Option<i32>,
    /// 扩展规格 JSON
    pub extra_sku_data: Option<String>,
    pub extra_data: Option<String>,
    pub min_order_amount: Option<Decimal>,
    /// 是否批发行：1 是 0 否
    pub is_wholesale_line: Option<i32>,
    pub wholesale_grade_id: Option<i32>,
}

impl Cart {
    pub const TABLE: &'static str = "cart";

    /// 行小计 = 单价 × 数量。
    pub fn subtotal(&self) -> Decimal {
        self.original_price
            .or(self.market_price)
            .unwrap_or(Decimal::ZERO)
            * Decimal::from(self.quantity.unwrap_or(0).max(0))
    }

    /// 是否已勾选。
    pub fn is_checked(&self) -> bool {
        self.is_checked.unwrap_or(0) == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn subtotal_calc() {
        let c = Cart {
            original_price: Some(Decimal::from_str("9.90").unwrap()),
            quantity: Some(3),
            ..Default::default()
        };
        assert_eq!(c.subtotal(), Decimal::from_str("29.70").unwrap());
    }
}