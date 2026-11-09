//! 商品查询与展示 DTO，对应原 Java `dto/product`、`query/product`、`vo/product`。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 商品分页查询参数，对应原 Java `ProductPageQuery` / `ProductListDTO`。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProductListQuery {
    /// 关键词（商品名/编号）
    pub keyword: Option<String>,
    pub category_id: Option<i32>,
    /// 服务层展开后的分类 ID 集合（含自身与子分类）；不参与 JSON 反序列化。
    #[serde(skip)]
    pub category_ids: Vec<i32>,
    pub brand_id: Option<i32>,
    pub shop_id: Option<i32>,
    pub product_status: Option<i32>,
    /// 价格区间
    pub price_min: Option<Decimal>,
    pub price_max: Option<Decimal>,
    /// 排序字段：sales-销量 price-价格 add_time-上架时间
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    /// 商品 ID 组，**逗号分隔字符串**（Java `ProductListDTO.ids`）。
    ///
    /// 原版前端首页楼层 / 推荐位用 `?ids=322,321,320` 精确指定商品：
    /// Java 侧同时用它做两件事——
    /// 1. `queryWrapper.in(Product::getProductId, ids)` **过滤**；
    /// 2. `ORDER BY FIELD(product_id, ids)` **按给定顺序排序**。
    ///
    /// Rust 早期实现缺这个字段，serde 直接丢弃 → 查询退化成「全表按时间倒序」，
    /// 于是所有楼层显示同一批商品（2026-10-21 首页楼层商品不符的根因）。
    pub ids: Option<String>,
    /// 商品分组 ID（装修 pc-product-group 的 productGroup 数据源）。
    pub product_group_id: Option<i32>,
}

/// 商品列表 VO（含原 Java `@TableField(exist = false)` 的门店扩展字段）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProductVO {
    pub product_id: i32,
    pub product_name: String,
    pub product_sn: String,
    pub product_price: Decimal,
    pub market_price: Decimal,
    pub product_stock: i32,
    pub product_status: i32,
    pub category_id: i32,
    pub brand_id: i32,
    pub shop_id: i32,
    pub pic_url: String,
    pub pic_thumb: String,
    pub sales: i32,
    /// 推荐卡片兼容字段：前端 `ProductCard` 读取 `product.price`
    pub price: Option<Decimal>,
    /// 推荐卡片兼容字段：前端读取 `product.virtualSales`
    pub virtual_sales: i32,
    /// 上架时间（库中为 int unsigned 秒级时间戳）
    pub add_time: i32,
    // —— 原 Java exist=false 的门店扩展字段 ——
    pub store_product_name: Option<String>,
    pub store_product_price: Option<Decimal>,
    pub store_pic_url: Option<String>,
    pub store_product_status: Option<i32>,
    pub store_product_stock: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 原版前端首页楼层传 `?ids=322,321,320`：必须被解析进 `ids` 字段，
    /// 否则查询会退化成「全表按时间倒序」→ 所有楼层显示同一批商品。
    #[test]
    fn product_list_query_accepts_ids_from_original_frontend() {
        let q: ProductListQuery =
            serde_json::from_str(r#"{"ids":"322,321,320","page":1,"size":20}"#).unwrap();
        assert_eq!(q.ids.as_deref(), Some("322,321,320"));
        assert_eq!(q.page, Some(1));
        assert_eq!(q.page_size, Some(20));
        // 未传 ids 时保持 None（不改变默认语义）
        let q2: ProductListQuery = serde_json::from_str(r#"{"page":1}"#).unwrap();
        assert!(q2.ids.is_none());
    }

    #[test]
    fn product_list_query_accepts_product_group_id() {
        let q: ProductListQuery = serde_json::from_str(r#"{"productGroupId":25}"#).unwrap();
        assert_eq!(q.product_group_id, Some(25));
        let q2: ProductListQuery = serde_json::from_str(r#"{"page":1}"#).unwrap();
        assert!(q2.product_group_id.is_none());
    }
}
