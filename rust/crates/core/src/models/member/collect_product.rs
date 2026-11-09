//! 商品收藏域（member::collect_product）DTO / VO / 纯函数。
//!
//! Java：`com.nextrade.api.controller.user.CollectProductController`
//! （`@RequestMapping("/api/user/collectProduct")`，list/save/cancel 3 端点，均需登录）。
//! 表 `collect_product`（collect_id/user_id/product_id/add_time/shop_id）。

use serde::{Deserialize, Serialize};

pub const SERVICE_DATA_ERROR: i32 = 1001;
/// 商品 ID 必填。
pub const PRODUCT_ID_REQUIRED: &str = "商品ID不能为空";
/// 商品不存在。
pub const PRODUCT_NOT_EXIST: &str = "商品不存在";
/// 已收藏。
pub const PRODUCT_ALREADY_COLLECTED: &str = "商品已收藏";
/// 取消收藏时未找到记录。
pub const COLLECT_NOT_EXIST: &str = "收藏记录不存在";

/// 列表查询（`page/size`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CollectListQuery {
    pub page: Option<u64>,
    pub size: Option<u64>,
}

impl CollectListQuery {
    #[must_use]
    pub fn page_params(&self) -> (u64, u64) {
        (
            self.page.unwrap_or(1).max(1),
            self.size.unwrap_or(15).clamp(1, 100),
        )
    }
}

/// 收藏入参（save / cancel）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CollectProductDTO {
    pub product_id: Option<i32>,
}

/// 收藏列表项（含商品摘要）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CollectProductVO {
    pub collect_id: i32,
    pub product_id: i32,
    pub shop_id: i32,
    pub add_time: i32,
    pub product_name: Option<String>,
    pub pic_thumb: Option<String>,
}

/// 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`。
#[must_use]
pub fn format_time(ts: Option<i32>) -> String {
    let Some(ts) = ts.filter(|t| *t > 0) else {
        return String::new();
    };
    chrono::DateTime::from_timestamp(i64::from(ts), 0)
        .map(|dt| {
            dt.with_timezone(
                &chrono::FixedOffset::east_opt(8 * 3600)
                    .unwrap_or_else(|| chrono::FixedOffset::east_opt(0).expect("utc")),
            )
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_are_java_verbatim() {
        assert_eq!(PRODUCT_ID_REQUIRED, "商品ID不能为空");
        assert_eq!(PRODUCT_NOT_EXIST, "商品不存在");
        assert_eq!(PRODUCT_ALREADY_COLLECTED, "商品已收藏");
        assert_eq!(COLLECT_NOT_EXIST, "收藏记录不存在");
    }

    #[test]
    fn paging_is_clamped() {
        assert_eq!(CollectListQuery::default().page_params(), (1, 15));
        let q = CollectListQuery {
            page: Some(0),
            size: Some(0),
            ..Default::default()
        };
        assert_eq!(q.page_params(), (1, 1));
        let q = CollectListQuery {
            page: Some(2),
            size: Some(500),
            ..Default::default()
        };
        assert_eq!(q.page_params(), (2, 100));
    }

    #[test]
    fn time_and_dto_shapes() {
        assert_eq!(format_time(None), "");
        assert_eq!(format_time(Some(1_700_000_000)), "2023-11-15 06:13:20");
        let dto: CollectProductDTO = serde_json::from_str(r#"{"productId":5}"#).unwrap();
        assert_eq!(dto.product_id, Some(5));
        let vo = CollectProductVO {
            collect_id: 1,
            product_id: 5,
            ..Default::default()
        };
        let v = serde_json::to_value(&vo).unwrap();
        assert_eq!(v["collectId"], 1);
        assert_eq!(v["productId"], 5);
    }
}
