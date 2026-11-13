//! 推荐（system::recommend）模型层 —— wave9。
//!
//! 对应 Java `com.nextrade.api.controller.common.RecommendController`
//! （`@RequestMapping("/api/common/recommend")`），原版 2 个 public GET：
//! `guessLike`（猜你喜欢，返回商品列表）与 `getProductIds`（返回推荐商品 ID 串）。
//! 另扩展 1 个：`cohortBuys`（相似用户近期购买聚合榜，协同过滤）。
//!
//! 说明：本实现从 `product` 表按「精品/点击量/上架」推荐；Java 侧推荐策略
//! （个性化/热度窗口）未迁移，属可解释的降级（见服务层偏差）。

use serde::{Deserialize, Serialize};

/// 猜你喜欢分页上限（避免一次拉全表）。
pub const MAX_GUESS_LIMIT: u32 = 50;
/// 推荐 ID 串目标数量。
///
/// 对齐 Java `ProductServiceImpl.getRecommendedProductIds` 的 `limit(40)`：
/// 精品段 + 热销段 + 浏览记录段不足时继续补足，最终截断到 40。
pub const DEFAULT_ID_COUNT: u32 = 40;
/// 精品段上限（Java `is_best = 1 … LIMIT 10`）。
pub const BEST_ID_COUNT: u32 = 10;
/// 热销段上限（Java `is_hot = 1 … LIMIT 10`）。
pub const HOT_ID_COUNT: u32 = 10;
/// 浏览记录段上限（Java `historyProductIds.limit(20)`）。
pub const HISTORY_ID_COUNT: u32 = 20;

/// 猜你喜欢查询（Java `ProductListDTO` 的安全子集）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuessLikeQuery {
    /// 页码
    pub page: Option<u32>,
    /// 每页条数
    pub size: Option<u32>,
    /// 店铺 ID
    pub shop_id: Option<i32>,
    /// 商品类型
    pub product_type: Option<i32>,
    /// 分类 ID
    pub category_id: Option<i32>,
    /// 排序字段（sales/click/new）
    pub sort_field: Option<String>,
    /// 排序方向
    pub sort_order: Option<String>,
}

impl GuessLikeQuery {
    /// 分页（默认 1/10，上限 50）。
    #[must_use]
    pub fn page_query(&self) -> crate::types::PageQuery {
        crate::types::PageQuery {
            page: self.page.unwrap_or(1).max(1) as u64,
            page_size: self.size.unwrap_or(10).min(MAX_GUESS_LIMIT).max(1) as u64,
        }
    }

    /// 排序方向：仅 `asc` 为升序，其余降序（默认热度）。
    #[must_use]
    pub fn ascending(&self) -> bool {
        self.sort_order
            .as_deref()
            .map(|s| s.eq_ignore_ascii_case("asc"))
            .unwrap_or(false)
    }
}

/// 相似用户购买查询（扩展接口 `cohortBuys`，原 Java 版无此端点）。
///
/// 协同过滤的轻量实现：与当前用户买过相同分类商品的用户（同画像群体）
/// 近期购买的商品聚合榜。只回商品与聚合排序，不暴露任何其他用户信息。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CohortBuysQuery {
    /// 页码
    pub page: Option<u32>,
    /// 每页条数
    pub size: Option<u32>,
    /// 店铺 ID
    pub shop_id: Option<i32>,
    /// 统计窗口（天）：只统计近 N 天的已支付订单，默认 90
    pub days: Option<u32>,
}

impl CohortBuysQuery {
    /// 分页（与猜你喜欢同口径：默认 1/10，上限 50）。
    #[must_use]
    pub fn page_query(&self) -> crate::types::PageQuery {
        crate::types::PageQuery {
            page: self.page.unwrap_or(1).max(1) as u64,
            page_size: self.size.unwrap_or(10).min(MAX_GUESS_LIMIT).max(1) as u64,
        }
    }

    /// 统计窗口天数：默认 90，夹紧到 7..=365。
    #[must_use]
    pub fn days(&self) -> u32 {
        self.days.unwrap_or(90).clamp(7, 365)
    }
}

/// 推荐商品列表项（Java `ProductListResDTO` 的前端常用子集）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendProductVO {
    /// 商品 ID
    pub product_id: i32,
    /// 商品名称
    pub product_name: String,
    /// 商品货号
    pub product_sn: String,
    /// 售价
    pub product_price: rust_decimal::Decimal,
    /// 主图
    pub pic_url: String,
    /// 缩略图
    pub pic_thumb: String,
    /// 店铺 ID
    pub shop_id: i32,
    /// 商品类型
    pub product_type: Option<i32>,
    /// 库存（原版 `ProductListResDTO.productStock`，前端据此判售罄）
    pub product_stock: i32,
    /// 销量（虚拟销量）
    pub virtual_sales: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_query_caps_at_fifty() {
        let q: GuessLikeQuery = serde_json::from_value(serde_json::json!({"size": 999})).unwrap();
        assert_eq!(q.page_query().page_size, u64::from(MAX_GUESS_LIMIT));
    }

    #[test]
    fn ascending_only_for_asc() {
        let q: GuessLikeQuery =
            serde_json::from_value(serde_json::json!({"sortOrder": "ASC"})).unwrap();
        assert!(q.ascending());
        let q: GuessLikeQuery = serde_json::from_value(serde_json::json!({})).unwrap();
        assert!(!q.ascending());
    }

    #[test]
    fn cohort_days_defaults_and_clamps() {
        let q: CohortBuysQuery = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(q.days(), 90);
        let q: CohortBuysQuery = serde_json::from_value(serde_json::json!({"days": 1})).unwrap();
        assert_eq!(q.days(), 7);
        let q: CohortBuysQuery = serde_json::from_value(serde_json::json!({"days": 9999})).unwrap();
        assert_eq!(q.days(), 365);
    }

    #[test]
    fn defaults_are_page_one_size_ten() {
        let q: GuessLikeQuery = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(q.page_query().page, 1);
        assert_eq!(q.page_query().page_size, 10);
    }
}
