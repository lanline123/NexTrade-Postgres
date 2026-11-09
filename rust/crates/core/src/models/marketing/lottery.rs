//! 抽奖（客户端）域（marketing::lottery）。
//! Java：`LotteryController` + `LotteryDrawServiceImpl` / `LotteryRecordServiceImpl`。
//! 表：`lottery_activity` / `lottery_prize` / `lottery_record` / `lottery_user_stat` / `user` / `user_points_log`。

use serde::{Deserialize, Serialize};

pub const MSG_NOT_LOGGED_IN: &str = "用户未登录";
pub const MSG_ACTIVITY_NOT_EXISTS: &str = "活动不存在";
pub const MSG_NOT_STARTED: &str = "活动未开始";
pub const MSG_ENDED: &str = "活动已结束";
pub const MSG_TODAY_LIMIT: &str = "今日抽奖次数已用完";
pub const MSG_TOTAL_LIMIT: &str = "抽奖次数已用完";
pub const MSG_WIN_LIMIT: &str = "中奖次数已达上限";
pub const MSG_POINTS_NOT_ENOUGH: &str = "积分不足,无法参与抽奖";
pub const MSG_RECORD_NOT_EXISTS: &str = "中奖记录不存在";
pub const MSG_NO_PERMISSION: &str = "无权操作此记录";
pub const MSG_ALREADY_RECEIVED: &str = "该奖品已领取";
pub const MSG_ONLY_PRODUCT_RECEIVE: &str = "只有商品奖品需要确认领取";
pub const MSG_POINTS_DEC_FAILED: &str = "积分减少失败";
pub const NO_WIN_NAME: &str = "未中奖";

/// 奖品类型：积分。
pub const PRIZE_TYPE_POINTS: i32 = 1;
/// 奖品类型：商品。
pub const PRIZE_TYPE_PRODUCT: i32 = 2;
/// 奖品类型：优惠券。
pub const PRIZE_TYPE_COUPON: i32 = 3;
/// 参与条件：消耗积分。
pub const COST_TYPE_POINTS: i32 = 1;

/// 记录状态文案。
#[must_use]
pub fn record_status_text(status: i32) -> &'static str {
    match status {
        0 => "未领取",
        1 => "已领取",
        2 => "已过期",
        _ => "未知",
    }
}

/// 昵称脱敏（Java `maskNickname`）。
#[must_use]
pub fn mask_nickname(name: Option<&str>) -> String {
    let Some(name) = name.filter(|v| !v.trim().is_empty()) else {
        return "用户***".to_string();
    };
    let chars: Vec<char> = name.chars().collect();
    if chars.len() == 11 && name.starts_with('1') && name.chars().all(|c| c.is_ascii_digit()) {
        let head: String = chars[..3].iter().collect();
        let tail: String = chars[7..].iter().collect();
        return format!("{head}****{tail}");
    }
    match chars.len() {
        1 => format!("{}**", chars[0]),
        2 => format!("{}*", chars[0]),
        len => {
            let mask_len = (len - 2).min(3);
            let mask = "*".repeat(mask_len);
            format!("{}{}{}", chars[0], mask, chars[len - 1])
        }
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

/// 九宫格位置（`grid_config` JSON 元素）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GridPosition {
    pub position: Option<i32>,
    pub prize_id: Option<i32>,
    #[serde(default)]
    pub probability: Option<rust_decimal::Decimal>,
}

/// 解析 `grid_config`。
#[must_use]
pub fn parse_grid(raw: Option<&str>) -> Vec<GridPosition> {
    raw.and_then(|s| serde_json::from_str::<Vec<GridPosition>>(s).ok())
        .unwrap_or_default()
}

/// 中奖记录查询（Java `LotteryRecordQuery extends BasePage`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LotteryRecordQuery {
    pub status: Option<i32>,
    pub page: Option<i64>,
    pub size: Option<i64>,
}

impl LotteryRecordQuery {
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

/// `winners?activityId=`。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LotteryWinnersQuery {
    pub activity_id: Option<i32>,
}

/// 确认领取入参（Java `LotteryConfirmReceiveParam`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LotteryConfirmReceiveParam {
    pub record_id: Option<i32>,
    pub order_id: Option<i32>,
}

/// 活动信息 VO（Java `LotteryActivityInfoVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LotteryActivityInfoVO {
    pub activity_id: i32,
    pub activity_name: String,
    pub start_time: String,
    pub end_time: String,
    pub cost_type: i32,
    pub cost_points: i32,
    pub decorate_config: Option<serde_json::Value>,
    pub no_win_name: String,
    pub no_win_image: String,
    pub no_win_tips: String,
    pub prize_entry_image: Option<String>,
    pub grid_config: Vec<GridPositionVO>,
    pub user_points: i32,
    pub today_remaining: i32,
    pub total_remaining: i32,
    pub win_remaining: i32,
    pub available_draw_count: i32,
}

/// 九宫格 VO（Java `GridPositionVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GridPositionVO {
    pub position: Option<i32>,
    pub prize_id: Option<i32>,
    pub probability: Option<rust_decimal::Decimal>,
    pub prize_name: Option<String>,
    pub prize_image: Option<String>,
    pub prize_type: Option<i32>,
    pub prize_value: Option<String>,
}

/// 抽奖结果 VO（Java `LotteryResultVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LotteryResultVO {
    pub is_win: bool,
    pub prize_id: Option<i32>,
    pub prize_name: Option<String>,
    pub prize_type: Option<i32>,
    pub prize_value: Option<String>,
    pub prize_image: Option<String>,
    pub position: i32,
    pub today_remaining: i32,
    pub total_remaining: i32,
    pub record_id: i32,
}

/// 中奖记录 VO（Java `LotteryRecordVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LotteryRecordVO {
    pub record_id: i32,
    pub activity_name: String,
    pub prize_name: String,
    pub prize_type: i32,
    pub prize_image: Option<String>,
    pub prize_value: String,
    pub status: i32,
    pub status_text: String,
    pub create_time: i64,
    pub receive_time: Option<i64>,
}

/// 中奖名单 VO（Java `LotteryWinnerVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LotteryWinnerVO {
    pub nickname: String,
    pub avatar: String,
    pub prize_name: String,
    pub prize_image: Option<String>,
    pub prize_type: i32,
    pub win_time: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nickname_masking_matches_java() {
        assert_eq!(mask_nickname(None), "用户***");
        assert_eq!(mask_nickname(Some("  ")), "用户***");
        assert_eq!(mask_nickname(Some("张")), "张**");
        assert_eq!(mask_nickname(Some("张三")), "张*");
        assert_eq!(mask_nickname(Some("张三丰")), "张*丰");
        assert_eq!(mask_nickname(Some("张三丰李")), "张**李");
        assert_eq!(mask_nickname(Some("13800138000")), "138****8000");
    }

    #[test]
    fn grid_parsing_and_status_text() {
        let grid = parse_grid(Some(
            r#"[{"position":1,"prizeId":3,"probability":10.5},{"position":2,"prizeId":null}]"#,
        ));
        assert_eq!(grid.len(), 2);
        assert_eq!(grid[0].prize_id, Some(3));
        assert_eq!(grid[1].prize_id, None);
        assert!(parse_grid(None).is_empty());
        assert!(parse_grid(Some("bad")).is_empty());
        assert_eq!(record_status_text(0), "未领取");
        assert_eq!(record_status_text(2), "已过期");
        assert_eq!(record_status_text(9), "未知");
    }

    #[test]
    fn record_query_paging() {
        let q = LotteryRecordQuery::default();
        assert_eq!(q.limit_offset(), (15, 0));
        let q = LotteryRecordQuery {
            page: Some(2),
            size: Some(10),
            status: Some(0),
        };
        assert_eq!(q.limit_offset(), (10, 10));
    }
}
