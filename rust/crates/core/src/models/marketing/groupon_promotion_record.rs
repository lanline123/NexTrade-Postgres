//! 拼团活动记录（管理端）域（marketing::groupon_promotion_record）。
//! Java `GrouponPromotionRecordController`（5 端点，admin）。

use serde::{Deserialize, Serialize};

pub const MSG_NOT_EXISTS_OR_FAILED: &str = "拼团记录不存在或已失败";
pub const SELF_OPERATED_SHOP_NAME: &str = "商城自营";

/// 拼团状态文案。
#[must_use]
pub fn join_status_desc(status: i32) -> &'static str {
    match status {
        1 => "待成团",
        2 => "拼团成功",
        3 => "拼团失败",
        _ => "",
    }
}

/// 订单状态文案（Java `OrderStatusEnum.getStatusName` 的默认中文）。
#[must_use]
pub fn order_status_name(status: Option<i32>) -> String {
    match status {
        Some(0) => "待支付".into(),
        Some(1) => "已确认，待发货".into(),
        Some(2) => "处理中，已发货".into(),
        Some(3) => "已取消".into(),
        Some(4) => "无效".into(),
        Some(5) => "已完成".into(),
        Some(_) => String::new(),
        None => String::new(),
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

/// 开团记录查询（Java `GrouponRecordOpenJonPageQuery`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OpenJoinListQuery {
    pub product_name: Option<String>,
    pub join_status: Option<i32>,
    pub groupon_promotion_id: Option<i64>,
    pub page: Option<i64>,
    pub size: Option<i64>,
    pub sort_field: Option<String>,
    pub sort_order: Option<String>,
}

/// 订单记录查询（Java `GrouponRecordOrderPageQuery`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OrderListQuery {
    pub groupon_promotion_id: Option<i64>,
    pub keyword: Option<String>,
    pub page: Option<i64>,
    pub size: Option<i64>,
    pub sort_field: Option<String>,
    pub sort_order: Option<String>,
}

/// 团员列表查询（Java `GrouponRecordOpenJoinMemberQuery`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GroupMemberQuery {
    pub groupon_promotion_record_id: Option<i64>,
}

/// `manualGroup?grouponPromotionRecordId=`。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ManualGroupQuery {
    pub groupon_promotion_record_id: Option<i64>,
}

/// 分页窗口（`BasePage` 口径）。
#[must_use]
pub fn limit_offset(page: Option<i64>, size: Option<i64>) -> (i64, i64) {
    let page = page.unwrap_or(1).max(1);
    let size = match size {
        None => 15,
        Some(v) if v > 100 => 100,
        Some(v) if v < 1 => 1,
        Some(v) => v,
    };
    (size, (page - 1) * size)
}

/// 开团记录 VO（Java `GrouponRecordOpenJonPageVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrouponRecordOpenJonPageVO {
    pub groupon_promotion_record_id: i64,
    pub open_group_time: i64,
    pub product_name: String,
    pub join_status: i32,
    pub group_captain_name: String,
    pub join_expiration_time: i64,
    pub min_group_num: i32,
}

/// 订单记录 VO（Java `GrouponRecordOrderPageVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrouponRecordOrderPageVO {
    pub join_type: i32,
    pub group_captain_name: String,
    pub product_name: String,
    pub order_sn: String,
    pub add_time: String,
    pub join_status: i32,
    pub username: Option<String>,
    pub order_status: Option<i32>,
    pub order_status_name: String,
}

/// 团员 VO（Java `GrouponRecordOpenJoinMemberVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrouponRecordOpenJoinMemberVO {
    pub join_type: i32,
    pub shop_id: i32,
    pub shop_title: String,
    pub add_time: i64,
    pub username: Option<String>,
    pub avatar: Option<String>,
    pub user_id: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_and_order_texts() {
        assert_eq!(join_status_desc(1), "待成团");
        assert_eq!(join_status_desc(3), "拼团失败");
        assert_eq!(order_status_name(Some(5)), "已完成");
        assert_eq!(order_status_name(None), "");
        assert_eq!(order_status_name(Some(99)), "");
    }

    #[test]
    fn paging_and_time() {
        assert_eq!(limit_offset(None, None), (15, 0));
        assert_eq!(limit_offset(Some(3), Some(20)), (20, 40));
        assert_eq!(format_time(1_700_000_000), "2023-11-15 06:13:20");
    }

    #[test]
    fn message_and_shop_name() {
        assert_eq!(MSG_NOT_EXISTS_OR_FAILED, "拼团记录不存在或已失败");
        assert_eq!(SELF_OPERATED_SHOP_NAME, "商城自营");
    }
}
