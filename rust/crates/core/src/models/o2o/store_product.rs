//! 门店商品分配（`/adminapi/o2o/storeProduct`）DTO。
//!
//! Java 来源（overseas）：`com.nextrade.bean.model.o2o.StoreProduct`
//! （`@TableName store_product`）；参考工程未提供 `StoreProductController`，
//! 路由与入参以线上管理端前端契约为准
//! （`java/view/admin/src/views/product/product/store/BatchAllocationPop.vue`：
//! `{type: "SHOPS"|"ALL"|"AREA", productIds: number[], shopIds?: number[]}`）。

use serde::{Deserialize, Serialize};

/// `POST /adminapi/o2o/storeProduct/allocationProduct` 入参。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllocationProductDTO {
    /// 分配类型：`SHOPS` 指定门店 / `ALL` 全部门店 / `AREA` 按区域。
    pub r#type: Option<String>,
    #[serde(default)]
    pub product_ids: Vec<i32>,
    /// `type=SHOPS` 时的目标门店。
    #[serde(default)]
    pub shop_ids: Option<Vec<i32>>,
}

impl AllocationProductDTO {
    /// 归一化分配类型（大写）。
    #[must_use]
    pub fn alloc_type(&self) -> String {
        self.r#type
            .as_deref()
            .unwrap_or("SHOPS")
            .trim()
            .to_ascii_uppercase()
    }
}

/// 分配结果（最小 VO：成功分配的门店商品行数）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AllocationResultVO {
    pub allocated: u64,
}

/// 待插入 `store_product` 的行（服务层由 `product` 行构造）。
#[derive(Debug, Clone, Default)]
pub struct AllocationRow {
    pub product_id: i32,
    pub product_name: Option<String>,
    pub product_price: Option<rust_decimal::Decimal>,
    /// 1-上架，0-下架。
    pub product_status: i32,
    pub product_stock: Option<i32>,
    pub shop_id: i64,
    pub card_group_id: Option<i32>,
    pub shop_category_id: Option<i32>,
    pub reservation_tpl_id: Option<i32>,
    pub pic_url: Option<String>,
    pub product_desc: Option<String>,
}
