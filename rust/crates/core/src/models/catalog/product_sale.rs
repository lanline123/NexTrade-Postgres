//! 商品可售/禁售状态（v5.8.61 `ProductSaleState` / `ProductSaleParam`）。

use serde::{Deserialize, Serialize};

/// 商品可售状态。
///
/// 对应 Java `com.nextrade.bean.vo.product.ProductSaleState`：
/// `saleStatus` 为最终可售状态，`manualSaleStatus` 为当前作用域的设置值，
/// 定时禁售/恢复由读时结算。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductSaleState {
    pub sale_status: i32,
    pub manual_sale_status: i32,
    pub sale_ban_time: i64,
    pub sale_recover_time: i64,
    pub sale_reason: Option<String>,
}

/// 平台设置商品禁售参数。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductSaleParam {
    pub product_id: i32,
    pub shop_id: Option<i32>,
    pub sale_status: i32,
    pub sale_ban_time: Option<i64>,
    pub sale_recover_time: Option<i64>,
}
