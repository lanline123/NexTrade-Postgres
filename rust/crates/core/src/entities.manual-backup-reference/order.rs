//! 订单实体 `order` 与订单项 `order_item`。
//! 对应 `com.nextrade.bean.model.order.Order` / `OrderItem`。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Order {
    pub order_id: Option<i32>,
    /// 订单类型
    pub order_type: Option<i32>,
    /// 是否预售
    pub is_presale: Option<i32>,
    /// 是否周期购订单
    pub is_cycle_order: Option<i32>,
    /// 是否卡订单
    pub is_card_order: Option<i32>,
    pub presale_type: Option<i32>,
    /// 订单编号
    pub order_sn: Option<String>,
    pub user_id: Option<i32>,
    pub parent_order_id: Option<i32>,
    pub parent_order_sn: Option<String>,
    /// 订单状态
    pub order_status: Option<i32>,
    /// 发货状态
    pub shipping_status: Option<i32>,
    /// 支付状态
    pub pay_status: Option<i32>,
    pub add_time: Option<i64>,
    /// 收货人
    pub consignee: Option<String>,
    /// 收货地址
    pub address: Option<String>,
    /// 地区ID串
    pub region_ids: Option<String>,
    /// 地区名称串
    pub region_names: Option<String>,
    /// 地址扩展 JSON
    pub address_data: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    /// 买家备注
    pub buyer_note: Option<String>,
    /// 后台备注
    pub admin_note: Option<String>,
    /// 配送方式
    pub shipping_method: Option<i32>,
    pub logistics_id: Option<i32>,
    pub logistics_provider: Option<i32>,
    pub logistics_name: Option<String>,
    pub shipping_type_id: Option<i32>,
    pub shipping_type_name: Option<String>,
    /// 运单号
    pub tracking_no: Option<String>,
    pub shipping_time: Option<i64>,
    pub received_time: Option<i64>,
    /// 支付方式ID
    pub pay_type_id: Option<i32>,
    pub pay_time: Option<i64>,
    /// 使用积分
    pub use_points: Option<i32>,
    /// 是否需要佣金
    pub is_need_commisson: Option<i32>,
    pub distribution_status: Option<i32>,
    pub referrer_user_id: Option<i32>,
    pub is_del: Option<i32>,
    pub shop_id: Option<i32>,
    pub is_store_splited: Option<i32>,
    /// 订单是否有效
    pub is_valid: Option<i32>,
    /// 评价状态
    pub comment_status: Option<i32>,
    /// 订单总额
    pub total_amount: Option<Decimal>,
    /// 已支付金额
    pub paid_amount: Option<Decimal>,
    /// 未支付金额
    pub unpaid_amount: Option<Decimal>,
    /// 未退款金额
    pub unrefund_amount: Option<Decimal>,
    /// 商品总额
    pub product_amount: Option<Decimal>,
    /// 优惠券抵扣
    pub coupon_amount: Option<Decimal>,
    /// 积分抵扣
    pub points_amount: Option<Decimal>,
    /// 礼品卡抵扣
    pub gift_card_amount: Option<Decimal>,
    /// 折扣金额
    pub discount_amount: Option<Decimal>,
    /// 余额支付
    pub balance: Option<Decimal>,
    /// 信用支付金额
    pub credit_amount: Option<Decimal>,
    pub is_credit_order: Option<i32>,
    /// 在线支付金额
    pub online_paid_amount: Option<Decimal>,
    /// 线下支付金额
    pub offline_paid_amount: Option<Decimal>,
    pub service_fee: Option<Decimal>,
    /// 运费
    pub shipping_fee: Option<Decimal>,
    pub invoice_fee: Option<Decimal>,
    /// 订单扩展 JSON
    pub order_extension: Option<String>,
    /// 订单来源
    pub order_source: Option<String>,
    pub invoice_data: Option<String>,
    /// 第三方交易号
    pub out_trade_no: Option<String>,
    /// 是否已结算
    pub is_settlement: Option<i32>,
    pub is_exchange_order: Option<i32>,
    pub mark: Option<i32>,
    pub vendor_id: Option<i32>,
    pub pos_user_id: Option<i32>,
    pub round_off_cents_amount: Option<Decimal>,
    pub round_off_dime_amount: Option<Decimal>,
    pub cash_amount: Option<Decimal>,
    pub cash_change_amount: Option<Decimal>,
    pub change_price_data: Option<String>,
    pub cancel_remark: Option<String>,
    pub discount_change_price: Option<Decimal>,
    pub tax_rate: Option<Decimal>,
    pub tax_rate_amount: Option<Decimal>,
}

impl Order {
    pub const TABLE: &'static str = "order";

    /// 是否已支付。
    pub fn is_paid(&self) -> bool {
        self.pay_status.unwrap_or(0) == 2
    }

    /// 是否可取消（未支付且有效）。
    pub fn is_cancellable(&self) -> bool {
        !self.is_paid()
            && self.is_valid.unwrap_or(1) == 1
            && self.is_del.unwrap_or(0) == 0
            && matches!(self.order_status, Some(0) | Some(10))
    }

    /// 展示用状态名（与原后台状态语义保持一致）。
    pub fn status_label(&self) -> &'static str {
        match self.order_status.unwrap_or(0) {
            0 => "待付款",
            10 => "待发货",
            20 => "已发货",
            30 => "已完成",
            40 => "已取消",
            50 => "已关闭",
            _ => "未知状态",
        }
    }
}

/// 订单项 `order_item`。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct OrderItem {
    pub item_id: Option<i32>,
    pub order_id: Option<i32>,
    pub order_sn: Option<String>,
    pub user_id: Option<i32>,
    /// 成交单价
    pub price: Option<Decimal>,
    pub quantity: Option<i32>,
    pub product_id: Option<i32>,
    pub product_name: Option<String>,
    pub product_sn: Option<String>,
    pub pic_thumb: Option<String>,
    pub sku_id: Option<i32>,
    /// SKU 规格 JSON
    pub sku_data: Option<String>,
    pub delivery_quantity: Option<i32>,
    pub product_type: Option<i32>,
    /// 是否赠品
    pub is_gift: Option<i32>,
    pub shop_id: Option<i32>,
    pub is_pin: Option<i32>,
    pub prepay_price: Option<Decimal>,
    /// 佣金 JSON
    pub commission: Option<String>,
    pub origin_price: Option<Decimal>,
    pub admin_change_pre_price: Option<Decimal>,
    /// 促销数据 JSON
    pub promotion_data: Option<String>,
    pub is_seckill: Option<i32>,
    pub extra_sku_data: Option<String>,
    pub suppliers_id: Option<i32>,
    pub card_group_name: Option<String>,
    pub vendor_id: Option<i32>,
    pub vendor_product_id: Option<i32>,
    pub vendor_product_sku_id: Option<i32>,
    pub vendor_product_supply_price: Option<Decimal>,
    pub change_price_data: Option<String>,
    pub gift_card_id: Option<i32>,
    pub delivery_service_fee: Option<Decimal>,
    pub server_service_fee: Option<Decimal>,
    pub is_wholesale_line: Option<i32>,
    pub wholesale_grade_id: Option<i32>,
    pub wholesale_base_price: Option<Decimal>,
}

impl OrderItem {
    pub const TABLE: &'static str = "order_item";

    /// 行小计。
    pub fn subtotal(&self) -> Decimal {
        self.price.unwrap_or(Decimal::ZERO)
            * Decimal::from(self.quantity.unwrap_or(0).max(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_rule() {
        let o = Order {
            pay_status: Some(1),
            is_valid: Some(1),
            is_del: Some(0),
            order_status: Some(0),
            ..Default::default()
        };
        assert!(o.is_cancellable());
    }

    #[test]
    fn status_labels() {
        let o = Order { order_status: Some(10), ..Default::default() };
        assert_eq!(o.status_label(), "待发货");
    }
}