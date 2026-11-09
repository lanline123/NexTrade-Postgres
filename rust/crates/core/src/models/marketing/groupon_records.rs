//! 拼团记录（客户端）域（marketing::groupon_records）。
//! Java `GrouponRecordsController`：`/api/promotion/groupon/record/{list,details}`（optional）。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub const MSG_RECORD_NOT_EXISTS: &str = "拼团记录不存在";

/// 拼团状态文案（1 待成团 / 2 成功 / 3 失败）。
#[must_use]
pub fn join_status_desc(status: i32) -> &'static str {
    match status {
        1 => "待成团",
        2 => "拼团成功",
        3 => "拼团失败",
        _ => "",
    }
}

/// 秒级时间戳格式化。
#[must_use]
pub fn format_time(secs: i64) -> String {
    if secs == 0 {
        return String::new();
    }
    chrono::DateTime::from_timestamp(secs, 0)
        .map(|utc| {
            (utc.naive_utc() + chrono::Duration::hours(8))
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_default()
}

/// 列表查询（Java `GrouponRecordPageQuery`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GrouponRecordPageQuery {
    pub join_type: Option<i32>,
    pub is_mine: Option<bool>,
    pub join_status: Option<i32>,
    pub date_type: Option<i32>,
    pub product_id: Option<i32>,
    pub exclude_ended: Option<bool>,
    pub keyword: Option<String>,
    pub page: Option<i64>,
    pub size: Option<i64>,
}

impl GrouponRecordPageQuery {
    #[must_use]
    pub fn limit_offset(&self) -> (i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let size = match self.size {
            None => 15,
            Some(v) if v > 100 => 100,
            Some(v) if v < 1 => 1,
            Some(v) => v,
        };
        (size, (page - 1) * size)
    }
}

/// `details?grouponPromotionRecordId=`。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GrouponRecordDetailQuery {
    pub groupon_promotion_record_id: Option<i64>,
}

/// 记录列表行（Java `GrouponRecordVO`，部分字段）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrouponRecordVO {
    pub groupon_promotion_record_id: i64,
    pub groupon_promotion_item_id: i64,
    pub groupon_promotion_id: i64,
    pub product_name: String,
    pub join_status: i32,
    pub join_status_desc: String,
    pub min_group_num: i32,
    pub min_groupon_price: Option<Decimal>,
    pub groupon_price: Option<Decimal>,
    pub product_price: Option<Decimal>,
    pub product_sn: Option<String>,
    pub pic_thumb: Option<String>,
    pub join_num_count: i64,
    pub start_time: String,
    pub end_time: String,
    pub order_sn: Option<String>,
    pub order_id: Option<i64>,
    pub order_add_time: Option<String>,
    pub pay_type_id: Option<i32>,
    pub join_user: Option<JoinUserVO>,
}

/// 详情 VO（Java `GrouponRecordDetailVO`，部分字段）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrouponRecordDetailVO {
    pub groupon_promotion_record_id: i64,
    pub groupon_promotion_id: i64,
    pub groupon_promotion_item_id: i64,
    pub product_name: String,
    pub product_id: i32,
    pub join_type: i32,
    pub join_status: i32,
    pub join_status_desc: String,
    pub min_group_num: i32,
    pub shop_id: i32,
    pub product_sn: Option<String>,
    pub shop_title: Option<String>,
    pub groupon_image: Option<String>,
    pub pc_groupon_image: Option<String>,
    pub min_groupon_price: Option<Decimal>,
    pub groupon_price: Option<Decimal>,
    pub product_price: Option<Decimal>,
    pub pic_thumb: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub join_num_count: i64,
    pub order_id: Option<i64>,
    pub join_users: Vec<JoinUserVO>,
}

/// 团员信息。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinUserVO {
    pub user_id: i64,
    pub username: Option<String>,
    pub avatar: Option<String>,
    pub leader: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_texts() {
        assert_eq!(join_status_desc(1), "待成团");
        assert_eq!(join_status_desc(2), "拼团成功");
        assert_eq!(join_status_desc(3), "拼团失败");
        assert_eq!(join_status_desc(9), "");
    }

    #[test]
    fn paging_and_time() {
        let q = GrouponRecordPageQuery::default();
        assert_eq!(q.limit_offset(), (15, 0));
        let q = GrouponRecordPageQuery {
            page: Some(2),
            size: Some(10),
            ..Default::default()
        };
        assert_eq!(q.limit_offset(), (10, 10));
        assert_eq!(format_time(1_700_000_000), "2023-11-15 06:13:20");
    }
}
