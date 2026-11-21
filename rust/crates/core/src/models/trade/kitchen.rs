//! 后厨出单与催菜 DTO / VO（我方新增，论坛帖 817927，用户 2026-11-21 拍板）。
//!
//! 形态：订单付款（`order_status = 1` 待发货）后进入后厨队列，
//! 后厨在管理端「后厨出单」看板做状态流转（待制作 → 制作中 →
//! 已出单）；买家在订单详情点「催菜」，催菜落商家消息中心
//! （新叶子类型 58「买家催菜」）并在后厨看板高亮提醒。
//! 后厨子状态独立落 `order_kitchen` 表（迁移 022），不侵入订单
//! 状态枚举；无后厨行时语义即「待制作」（懒物化，流转/催菜时才建行）。

use serde::{Deserialize, Serialize};

/// 后厨子状态（`order_kitchen.kitchen_status`）。
pub mod kitchen_status {
    /// 待制作（默认；含尚无后厨行的已付款订单）
    pub const PENDING: i32 = 0;
    /// 制作中
    pub const COOKING: i32 = 1;
    /// 已出单（后厨完工，订单回归正常发货/核销链）
    pub const DONE: i32 = 2;
}

/// 后厨状态文案。
#[must_use]
pub fn kitchen_status_name(status: i32) -> &'static str {
    match status {
        kitchen_status::PENDING => "待制作",
        kitchen_status::COOKING => "制作中",
        kitchen_status::DONE => "已出单",
        _ => "",
    }
}

/// 催菜在后台消息中心的叶子类型（`admin_msg.msg_type`，交易消息类目）。
pub const MSG_TYPE_URGE: i32 = 58;

/// 催菜冷却（秒）：同一买家对同一订单两次催菜的最小间隔。
pub const URGE_COOLDOWN_SECONDS: i32 = 600;

/// 催菜每日上限：同一买家对同一订单自然日内最多催菜次数。
pub const URGE_DAILY_LIMIT: i64 = 5;

/// 催菜频控判定结果（纯函数，供单测钉住）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrgeDecision {
    /// 允许催菜
    Allowed,
    /// 冷却中，`remain_seconds` 为还需等待的秒数
    Cooldown { remain_seconds: i32 },
    /// 当日次数已达上限
    DailyLimitReached,
}

/// 催菜频控判定：先冷却、后日限。
///
/// - `last_urge_time`：该买家对该订单上次催菜的秒级时间戳（无记录为 0）。
/// - `today_count`：该买家对该订单今日已催菜次数。
#[must_use]
pub fn urge_decision(now: i32, last_urge_time: i32, today_count: i64) -> UrgeDecision {
    if last_urge_time > 0 {
        let elapsed = now.saturating_sub(last_urge_time);
        if elapsed < URGE_COOLDOWN_SECONDS {
            return UrgeDecision::Cooldown {
                remain_seconds: URGE_COOLDOWN_SECONDS - elapsed,
            };
        }
    }
    if today_count >= URGE_DAILY_LIMIT {
        return UrgeDecision::DailyLimitReached;
    }
    UrgeDecision::Allowed
}

/// 后厨流转是否合法：只允许向前（待制作 → 制作中 → 已出单，
/// 允许待制作直达已出单），不允许回退或原地不动。
#[must_use]
pub fn transition_allowed(current: i32, target: i32) -> bool {
    (kitchen_status::PENDING..=kitchen_status::DONE).contains(&target) && target > current
}

/// 后厨看板查询（管理端；`shop_id` 由路由按登录态店铺范围覆写，
/// 前端传入值不可信）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KitchenBoardQuery {
    /// 后厨状态过滤：缺省/负数 = 全部
    pub status: Option<i32>,
    /// 订单号关键词
    pub keyword: Option<String>,
    /// 店铺过滤（仅平台视角有效，店铺视角被路由覆写为本店）
    pub shop_id: Option<i32>,
    pub page: Option<i64>,
    pub size: Option<i64>,
}

/// 后厨看板商品行。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KitchenItemVO {
    pub item_id: i32,
    pub product_name: String,
    pub pic_thumb: String,
    /// 规格原文（`order_item.sku_data`）
    pub sku_data: String,
    pub quantity: i32,
}

/// 后厨看板订单卡。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KitchenOrderVO {
    pub order_id: i32,
    pub order_sn: String,
    pub shop_id: i32,
    pub consignee: String,
    pub mobile: String,
    /// 买家备注（后厨需据此备餐，空串 = 无备注）
    pub buyer_note: String,
    pub shipping_type_name: String,
    /// 付款时间（`yyyy-MM-dd HH:mm:ss`）
    pub pay_time: String,
    /// 已等待分钟数（按下单付款时间起算，未付款兜底按下单时间）
    pub wait_minutes: i64,
    pub kitchen_status: i32,
    pub kitchen_status_name: String,
    /// 累计被催菜次数
    pub urge_count: i32,
    /// 最近一次催菜时间（格式化；空串 = 从未催菜）
    pub last_urge_time: String,
    /// 是否有未读催菜（后厨未点「已知晓」）
    pub urge_unread: bool,
    pub items: Vec<KitchenItemVO>,
}

/// 后厨看板（计数 + 当前页订单卡）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KitchenBoardVO {
    pub pending_count: i64,
    pub cooking_count: i64,
    pub done_count: i64,
    /// 未读催菜订单数（看板红点口径）
    pub urge_unread_count: i64,
    pub total: i64,
    pub orders: Vec<KitchenOrderVO>,
}

/// 后厨状态流转请求（管理端）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KitchenTransitionDTO {
    pub order_id: i32,
    pub target_status: i32,
}

/// 后厨「已知晓」催菜请求（管理端）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KitchenAckUrgeDTO {
    pub order_id: i32,
}

/// 催菜请求（买家端）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KitchenUrgeDTO {
    pub order_id: i32,
}

/// 催菜结果（买家端：催菜后的最新频控状态）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KitchenUrgeResultVO {
    pub order_id: i32,
    /// 本单累计催菜次数（含本次）
    pub urge_count: i32,
    /// 本次起算的冷却秒数（冷却期内不可再催）
    pub cooldown_seconds: i32,
}

/// 买家订单详情的后厨面板数据（买家端）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KitchenInfoVO {
    pub order_id: i32,
    /// 订单是否在后厨口径内（已付款待发货）
    pub in_kitchen: bool,
    pub kitchen_status: i32,
    pub kitchen_status_name: String,
    /// 本单累计催菜次数
    pub urge_count: i32,
    /// 当前是否可催菜（状态允许且不在冷却/未达日限）
    pub can_urge: bool,
    /// 冷却剩余秒数（0 = 不在冷却）
    pub cooldown_remain_seconds: i32,
    /// 今日还可催菜次数
    pub daily_urges_left: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urge_decision_first_time_allowed() {
        assert_eq!(urge_decision(1_000, 0, 0), UrgeDecision::Allowed);
    }

    #[test]
    fn urge_decision_cooldown_blocks_and_reports_remain() {
        // 上次催菜在 100 秒前，冷却 600 秒 → 还需等 500 秒
        assert_eq!(
            urge_decision(10_000, 9_900, 1),
            UrgeDecision::Cooldown {
                remain_seconds: 500
            }
        );
        // 恰好满 600 秒放行
        assert_eq!(urge_decision(10_000, 9_400, 1), UrgeDecision::Allowed);
    }

    #[test]
    fn urge_decision_daily_limit() {
        assert_eq!(
            urge_decision(10_000, 0, URGE_DAILY_LIMIT),
            UrgeDecision::DailyLimitReached
        );
        assert_eq!(
            urge_decision(10_000, 0, URGE_DAILY_LIMIT - 1),
            UrgeDecision::Allowed
        );
    }

    #[test]
    fn urge_decision_cooldown_takes_priority_over_daily_limit() {
        assert_eq!(
            urge_decision(10_000, 9_999, URGE_DAILY_LIMIT),
            UrgeDecision::Cooldown {
                remain_seconds: 599
            }
        );
    }

    #[test]
    fn transition_forward_only() {
        assert!(transition_allowed(0, 1));
        assert!(transition_allowed(0, 2));
        assert!(transition_allowed(1, 2));
        assert!(!transition_allowed(1, 0));
        assert!(!transition_allowed(2, 2));
        assert!(!transition_allowed(0, 0));
        assert!(!transition_allowed(0, 3));
    }

    #[test]
    fn status_names() {
        assert_eq!(kitchen_status_name(0), "待制作");
        assert_eq!(kitchen_status_name(1), "制作中");
        assert_eq!(kitchen_status_name(2), "已出单");
        assert_eq!(kitchen_status_name(9), "");
    }
}
