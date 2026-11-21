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

/// 分享得次数未开启（我方新增）。
pub const MSG_SHARE_REWARD_DISABLED: &str = "该活动未开启分享得次数";
/// 分享事件标识缺失（我方新增）。
pub const MSG_SHARE_EVENT_REQUIRED: &str = "分享事件标识不能为空";
/// 今日分享奖励已达上限（我方新增）。
pub const MSG_SHARE_DAILY_LIMIT: &str = "今日分享奖励已达上限";
/// 活动期分享奖励已达上限（我方新增）。
pub const MSG_SHARE_TOTAL_LIMIT: &str = "分享奖励已达活动上限";
/// 两次分享奖励间隔过短（我方新增）。
pub const MSG_SHARE_TOO_FREQUENT: &str = "分享操作太频繁，请稍后再试";

/// 两次分享奖励的最小间隔（秒，我方新增防刷口径）：
/// 同一用户同一活动两次入账至少间隔 60 秒，挡住连续点分享面板刷次数。
pub const SHARE_REWARD_MIN_INTERVAL_SECS: i64 = 60;

/// 分享渠道：微信小程序分享面板（我方新增）。
pub const SHARE_CHANNEL_WEAPP: &str = "weapp";
/// 分享渠道：H5 复制链接（我方新增）。
pub const SHARE_CHANNEL_H5_COPY: &str = "h5_copy";
/// 分享渠道：系统分享（navigator.share，我方新增）。
pub const SHARE_CHANNEL_WEB_SHARE: &str = "web_share";

/// 每次分享奖励次数归一化（我方新增）：配置缺失/非正数时回落 1。
#[must_use]
pub fn normalize_share_reward_count(raw: i32) -> i32 {
    if raw > 0 {
        raw
    } else {
        1
    }
}

/// 含分享奖励加成的有效上限（我方新增）：基础上限为 0（不限制）时奖励不改变不限制语义。
#[must_use]
pub fn effective_limit(limit: i32, bonus: i32) -> i32 {
    if limit > 0 {
        limit + bonus.max(0)
    } else {
        0
    }
}

/// 还可获奖励的分享次数（我方新增）：上限为 0（不限制）时返回 -1，与 `remaining` 同口径。
#[must_use]
pub fn share_events_remaining(limit: i32, used_events: i32) -> i32 {
    if limit > 0 {
        (limit - used_events).max(0)
    } else {
        -1
    }
}

/// 分享事件标识校验（我方新增）：非空且不超过 64 字符（与流水表 event_id 列宽一致）。
#[must_use]
pub fn valid_share_event_id(raw: Option<&str>) -> Option<String> {
    let id = raw.map(str::trim).filter(|s| !s.is_empty())?;
    if id.chars().count() > 64 {
        return None;
    }
    Some(id.to_string())
}

/// 分享渠道归一化（我方新增）：去空白、截断 32 字符（与流水表 channel 列宽一致），空回落空串。
#[must_use]
pub fn normalize_share_channel(raw: Option<&str>) -> String {
    raw.map(str::trim)
        .unwrap_or_default()
        .chars()
        .take(32)
        .collect()
}

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
    /// 分享得次数开关（我方新增）：0 关闭 / 1 开启
    pub share_reward_enabled: i32,
    /// 每次有效分享奖励的抽奖次数（我方新增，已归一化）
    pub share_reward_count: i32,
    /// 今日已通过分享入账的奖励次数合计（我方新增）
    pub share_today_earned_count: i32,
    /// 今日还可获奖励的分享次数（我方新增）：-1 表示不限制
    pub share_today_reward_remaining: i32,
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

/// 分享得次数上报入参（我方新增）。
///
/// 有效分享判定口径：客户端在分享动作完成点上报（小程序分享面板选定目标 /
/// H5 复制链接成功 / 系统分享完成），服务端不核验第三方平台真实送达，
/// 以事件幂等 + 每日/活动期上限 + 最小间隔三重限额防刷（见服务层）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LotteryShareRewardParam {
    /// 活动 ID（可空：空则取当前进行中的活动，与 info/draw 同口径）
    pub activity_id: Option<i32>,
    /// 客户端生成的分享事件标识（幂等键，同一事件重复上报不重复发放）
    pub event_id: Option<String>,
    /// 分享渠道：weapp / h5_copy / web_share
    pub channel: Option<String>,
}

/// 分享得次数结果 VO（我方新增）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LotteryShareRewardVO {
    /// 本次是否新入账
    pub granted: bool,
    /// 该事件此前已入账（幂等重放，未重复发放）
    pub already_rewarded: bool,
    /// 本次入账的奖励次数（重放时为 0）
    pub reward_count: i32,
    /// 今日剩余抽奖次数（含奖励加成，-1 不限制）
    pub today_remaining: i32,
    /// 总剩余抽奖次数（含奖励加成，-1 不限制）
    pub total_remaining: i32,
    /// 今日已通过分享入账的奖励次数合计
    pub share_today_earned_count: i32,
    /// 今日还可获奖励的分享次数：-1 表示不限制
    pub share_today_reward_remaining: i32,
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
    fn share_reward_helpers() {
        assert_eq!(normalize_share_reward_count(3), 3);
        assert_eq!(normalize_share_reward_count(0), 1);
        assert_eq!(normalize_share_reward_count(-2), 1);
        // 基础不限制（0）时奖励不改变不限制语义；有限时叠加奖励
        assert_eq!(effective_limit(0, 5), 0);
        assert_eq!(effective_limit(10, 5), 15);
        assert_eq!(effective_limit(10, -3), 10);
        assert_eq!(share_events_remaining(0, 9), -1);
        assert_eq!(share_events_remaining(3, 1), 2);
        assert_eq!(share_events_remaining(3, 5), 0);
        assert_eq!(valid_share_event_id(Some(" evt-1 ")).as_deref(), Some("evt-1"));
        assert_eq!(valid_share_event_id(Some("  ")), None);
        assert_eq!(valid_share_event_id(None), None);
        assert_eq!(valid_share_event_id(Some(&"x".repeat(65))), None);
        assert_eq!(normalize_share_channel(Some(" weapp ")), "weapp");
        assert_eq!(normalize_share_channel(None), "");
        assert_eq!(normalize_share_channel(Some(&"c".repeat(40))).chars().count(), 32);
        assert_eq!(SHARE_REWARD_MIN_INTERVAL_SECS, 60);
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
