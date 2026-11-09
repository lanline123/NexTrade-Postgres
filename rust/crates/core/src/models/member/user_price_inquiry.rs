//! 前台用户询价域（member::user_price_inquiry）DTO / VO。
//! Java `UserPriceInquiryController`（`/api/user/priceInquiry`，4 端点，需登录）。
//! 表 `price_inquiry` / `price_inquiry_reply_record`。

use serde::{Deserialize, Serialize};

pub const SERVICE_DATA_ERROR: i32 = 1001;
pub const USER_NOT_LOGIN: &str = "用户未登录";
pub const INQUIRY_NOT_EXIST: &str = "询价信息不存在";
pub const REMARK_REQUIRED: &str = "回复内容不能为空";

/// 列表查询（Java `PriceInquiryDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PriceInquiryListQuery {
    pub mobile: Option<String>,
    pub status: Option<i32>,
    pub keyword: Option<String>,
    pub shop_id: Option<i32>,
    pub page: Option<u64>,
    pub size: Option<u64>,
}

impl PriceInquiryListQuery {
    #[must_use]
    pub fn page_params(&self) -> (u64, u64) {
        (
            self.page.unwrap_or(1).max(1),
            self.size.unwrap_or(15).clamp(1, 100),
        )
    }
}

/// 回复入参（Java `PriceInquiryReplyDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PriceInquiryReplyDTO {
    pub id: Option<i32>,
    pub remark: Option<String>,
}

/// 删除入参（Java `OperateDTO.id`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OperateDTO {
    pub id: Option<i32>,
}

/// 列表项（Java `PriceInquiryVO` 子集）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PriceInquiryVO {
    pub id: i32,
    pub mobile: String,
    pub email: String,
    pub content: String,
    pub product_id: i32,
    pub remark: String,
    pub status: i32,
    pub shop_id: i32,
    pub user_id: i32,
    pub create_time: String,
    pub product: Option<InquiryProductVO>,
}

/// 商品摘要（Java `PriceInquiryVO.ProductVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InquiryProductVO {
    pub product_id: i32,
    pub product_name: String,
    pub product_sn: String,
    pub pic_thumb: String,
}

/// 详情（Java `PriceInquiryDetailVO` 子集）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PriceInquiryDetailVO {
    pub id: i32,
    pub content: String,
    pub create_time: String,
    pub contact_name: String,
    pub mobile: String,
    pub email: String,
    pub remark: String,
    pub status: i32,
    pub product: Option<InquiryProductVO>,
    pub reply_records: Vec<PriceInquiryReplyRecordVO>,
}

/// 回复记录（Java `PriceInquiryReplyRecordVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PriceInquiryReplyRecordVO {
    pub reply_name: String,
    pub create_time: String,
    pub content: String,
    pub reply_type: i32,
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
        assert_eq!(USER_NOT_LOGIN, "用户未登录");
        assert_eq!(INQUIRY_NOT_EXIST, "询价信息不存在");
        assert_eq!(REMARK_REQUIRED, "回复内容不能为空");
    }

    #[test]
    fn paging_clamped() {
        assert_eq!(PriceInquiryListQuery::default().page_params(), (1, 15));
        let q = PriceInquiryListQuery {
            page: Some(3),
            size: Some(999),
            ..Default::default()
        };
        assert_eq!(q.page_params(), (3, 100));
    }

    #[test]
    fn vo_serde_shapes() {
        let vo = PriceInquiryVO {
            id: 1,
            product: Some(InquiryProductVO {
                product_id: 2,
                product_name: "A".into(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let v = serde_json::to_value(&vo).unwrap();
        assert_eq!(v["product"]["productId"], 2);
        assert_eq!(v["product"]["productName"], "A");
        assert_eq!(format_time(Some(1_700_000_000)), "2023-11-15 06:13:20");
    }
}
