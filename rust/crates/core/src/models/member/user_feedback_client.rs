//! 前台用户留言/反馈域（member::user_feedback_client）DTO / VO。
//! Java `UserFeedbackClientController`（`/api/user/feedback/list|submit`，需登录）。
//! 表 `feedback`（16 列）；状态：0 待回复 / 1 已回复 / 2 无效。

use serde::{Deserialize, Serialize};

pub const SERVICE_DATA_ERROR: i32 = 1001;
pub const CONTENT_REQUIRED: &str = "留言内容不能为空";
pub const MEMBER_NOT_FOUND: &str = "会员不存在";
pub const SENSITIVE_WORD: &str = "留言内容包含敏感词";
pub const ORDER_NOT_EXIST: &str = "订单不存在";
pub const NOT_OWN_ORDER: &str = "非本人订单，无权提交留言";

/// 留言类型（Java `UserFeedbackType`）。
pub const FEEDBACK_TYPES: [(i32, &str); 7] = [
    (0, "建议"),
    (1, "投诉"),
    (2, "商品"),
    (3, "其他"),
    (4, "店铺投诉"),
    (5, "订单问题"),
    (6, "订单咨询"),
];

#[must_use]
pub fn type_name(r#type: i32) -> &'static str {
    FEEDBACK_TYPES
        .iter()
        .find(|(t, _)| *t == r#type)
        .map_or("", |(_, n)| *n)
}

#[must_use]
pub fn status_text(status: i32) -> &'static str {
    match status {
        0 => "待回复",
        1 => "已回复",
        2 => "无效",
        _ => "",
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FeedbackListQuery {
    pub r#type: Option<String>,
    pub is_order: Option<i32>,
    pub product_id: Option<i32>,
    pub status: Option<i32>,
    pub page: Option<u64>,
    pub size: Option<u64>,
}

impl FeedbackListQuery {
    #[must_use]
    pub fn page_params(&self) -> (u64, u64) {
        (
            self.page.unwrap_or(1).max(1),
            self.size.unwrap_or(15).clamp(1, 100),
        )
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FeedbackCreateDTO {
    pub r#type: Option<i32>,
    pub title: Option<String>,
    pub content: Option<String>,
    pub product_id: Option<i32>,
    pub order_id: Option<i32>,
    pub shop_id: Option<i32>,
    pub complaint_info: Option<String>,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub feedback_pics: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UserFeedbackVO {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub r#type: i32,
    pub type_name: String,
    pub status: i32,
    pub status_text: String,
    pub add_time: String,
    pub product_id: Option<i32>,
    pub order_id: Option<i32>,
    pub reply: Option<FeedbackReplyVO>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FeedbackReplyVO {
    pub id: i32,
    pub parent_id: i32,
    pub username: String,
    pub content: String,
    pub add_time: String,
}

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

/// 敏感词检查（Java：逗号分隔配置 + `contains`）。
#[must_use]
pub fn contains_sensitive(text: &str, config: Option<&str>) -> bool {
    config.is_some_and(|cfg| {
        cfg.split(',')
            .map(str::trim)
            .filter(|w| !w.is_empty())
            .any(|w| text.contains(w))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_and_status_texts() {
        assert_eq!(type_name(0), "建议");
        assert_eq!(type_name(6), "订单咨询");
        assert_eq!(type_name(9), "");
        assert_eq!(status_text(0), "待回复");
        assert_eq!(status_text(1), "已回复");
        assert_eq!(status_text(2), "无效");
    }

    #[test]
    fn sensitive_word_check() {
        assert!(contains_sensitive("包含违禁词内容", Some("违禁词,其他")));
        assert!(!contains_sensitive("正常内容", Some("违禁词,其他")));
        assert!(!contains_sensitive("正常内容", None));
        assert!(!contains_sensitive("正常内容", Some(" , ")));
    }

    #[test]
    fn dto_serde_and_time() {
        let dto: FeedbackCreateDTO =
            serde_json::from_str(r#"{"type":2,"content":"x","feedbackPics":["a"],"orderId":5}"#)
                .unwrap();
        assert_eq!(dto.r#type, Some(2));
        assert_eq!(dto.feedback_pics, Some(vec!["a".to_string()]));
        let vo = UserFeedbackVO {
            id: 1,
            type_name: type_name(3).into(),
            ..Default::default()
        };
        let v = serde_json::to_value(&vo).unwrap();
        assert_eq!(v["typeName"], "其他");
        assert_eq!(format_time(Some(1_700_000_000)), "2023-11-15 06:13:20");
    }
}
