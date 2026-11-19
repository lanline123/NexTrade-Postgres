//! 收货后自动评价（Tigshop v5.8.65 功能①）域模型 / DTO / VO / 纯决策函数。
//!
//! 来源：**原版有·我方改口径**（原版 v5.8.65 新增「收货后自动评价」；
//! 设计见 `~/workspace/auto-review/DESIGN.md`）。
//!
//! 口径要点：
//! - 店铺级设置（开关默认关 / 收货后 N 天 / 多条文案），落
//!   `shop_auto_comment_setting` 表；无行 = 未配置 = 关闭。
//! - 确认收货时把到期时间与文案**快照**落 `order_auto_comment`（一单一行，
//!   `order_id` 唯一作幂等锚）；之后改设置不影响已快照订单。
//! - 生成的评价复用 `comment` 表原版既有列 `is_default`（「系统默认评价」）
//!   作来源标记，前台展示「系统自动评价」——我方改口径的合规底线。
//!
//! 本文件只放 DTO/VO/纯函数（无 SQL / IO），决策逻辑可单测
//! （CI 测试套件无数据库，与仓库既有口径一致）。

use serde::{Deserialize, Serialize};

/// 待生成（快照已落，等待到期）。
pub const TASK_STATUS_PENDING: i32 = 0;
/// 已生成（至少生成 1 件评价，或整单在生成时已全部有评价但本轮补齐）。
pub const TASK_STATUS_DONE: i32 = 1;
/// 已跳过（到期时整单订单项均已有真实评价，本轮生成 0 件）。
pub const TASK_STATUS_SKIPPED: i32 = 2;
/// 生成中（已被某轮扫描抢占，防重复扫描重复生成）。
pub const TASK_STATUS_CLAIMED: i32 = 3;

/// 抢占僵尸回收阈值（秒）：`status = 3` 且 `update_time` 早于此时长视为
/// 上轮进程中断遗留，可被重新抢占。
pub const CLAIM_STALE_SECONDS: i64 = 3600;

/// 自动评价星级（原版口径：自动好评 5 星）。
pub const AUTO_COMMENT_RANK: i32 = 5;

/// 文案兜底（店铺开启自动评价但未填写任何文案时使用）。
pub const DEFAULT_AUTO_COMMENT_CONTENT: &str = "商品收到了，质量不错，好评！";

/// 默认收货后天数（店铺未配置时设置页回显）。
pub const DEFAULT_AUTO_COMMENT_DAYS: i32 = 7;

/// 天数合法上限（含）。
pub const MAX_AUTO_COMMENT_DAYS: i32 = 90;
/// 文案条数上限。
pub const MAX_AUTO_COMMENT_CONTENTS: usize = 10;
/// 单条文案字数上限（按字符数计）。
pub const MAX_AUTO_COMMENT_CONTENT_CHARS: usize = 200;

// ————————————————————— 实体（与迁移 005 两表逐列对齐） —————————————————————

/// 店铺自动评价设置（`shop_auto_comment_setting`）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AutoCommentSetting {
    pub shop_id: i32,
    /// 0 关 / 1 开。
    pub enabled: i32,
    /// 收货后多少天生成。
    pub days: i32,
    /// 文案列表（落库时序列化为 JSON 数组字符串存 `contents` 列）。
    pub contents: Vec<String>,
    pub update_time: i32,
}

impl AutoCommentSetting {
    /// 未配置店铺的默认设置（关 / 7 天 / 无文案）。
    #[must_use]
    pub fn defaults(shop_id: i32) -> Self {
        Self {
            shop_id,
            enabled: 0,
            days: DEFAULT_AUTO_COMMENT_DAYS,
            contents: Vec::new(),
            update_time: 0,
        }
    }

    /// 是否开启。
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled == 1
    }
}

/// 收货快照 / 待生成任务（`order_auto_comment`）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AutoCommentTask {
    pub id: i32,
    pub order_id: i32,
    pub shop_id: i32,
    pub user_id: i32,
    /// 确认收货时间（秒）。
    pub received_time: i32,
    /// 快照的收货后天数。
    pub days: i32,
    /// 快照的到期时间（秒）= `received_time + days * 86400`。
    pub due_time: i32,
    /// 快照文案（收货时按 `order_id` 定选一条）。
    pub content: String,
    pub status: i32,
    pub generated_time: i32,
    pub add_time: i32,
    pub update_time: i32,
}

// ————————————————————— DTO / VO —————————————————————

/// 设置读取 VO / 保存入参共用形状（JSON camelCase）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AutoCommentSettingDTO {
    pub shop_id: Option<i32>,
    /// 0 关 / 1 开。
    pub enabled: Option<i32>,
    /// 收货后天数（0..=90）。
    pub days: Option<i32>,
    /// 文案列表（≤10 条，单条 ≤200 字）。
    pub contents: Option<Vec<String>>,
}

// ————————————————————— 纯函数 —————————————————————

/// 解析落库的文案列（JSON 字符串数组）→ 文案列表。
///
/// 非 JSON / 非数组 → 空列表（不把脏数据当文案用）；逐条 trim、丢空串。
#[must_use]
pub fn parse_contents(raw: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Vec::new();
    };
    let Some(arr) = value.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// 文案列表 → 落库字符串（JSON 数组）。
#[must_use]
pub fn serialize_contents(contents: &[String]) -> String {
    serde_json::to_string(&contents).unwrap_or_else(|_| "[]".to_string())
}

/// 清洗商家提交的文案列表：trim、丢空串、去重（保序）。
#[must_use]
pub fn normalize_contents(contents: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in contents {
        let item = raw.trim().to_string();
        if !item.is_empty() && !out.contains(&item) {
            out.push(item);
        }
    }
    out
}

/// 校验商家提交的设置（保存前调用）：返回第一处错误文案。
///
/// 口径：天数 0..=90；文案 ≤10 条、单条 ≤200 字（超限报错，不静默截断）。
pub fn validate_setting(days: i32, contents: &[String]) -> Result<(), String> {
    if !(0..=MAX_AUTO_COMMENT_DAYS).contains(&days) {
        return Err(format!("自动评价天数需在 0 到 {MAX_AUTO_COMMENT_DAYS} 天之间"));
    }
    if contents.len() > MAX_AUTO_COMMENT_CONTENTS {
        return Err(format!("自动评价文案最多 {MAX_AUTO_COMMENT_CONTENTS} 条"));
    }
    if contents
        .iter()
        .any(|c| c.chars().count() > MAX_AUTO_COMMENT_CONTENT_CHARS)
    {
        return Err(format!(
            "单条自动评价文案不能超过 {MAX_AUTO_COMMENT_CONTENT_CHARS} 字"
        ));
    }
    Ok(())
}

/// 按 `order_id` 确定性选文案（同单恒同文案、跨单分散轮换）。
///
/// 文案为空 → 平台兜底文案。
#[must_use]
pub fn pick_content(contents: &[String], order_id: i32) -> String {
    if contents.is_empty() {
        return DEFAULT_AUTO_COMMENT_CONTENT.to_string();
    }
    let idx = (order_id.unsigned_abs() as usize) % contents.len();
    contents[idx].clone()
}

/// 快照决策：确认收货时按店铺**当时**设置决定是否落快照、落什么。
///
/// - 未开启（含无设置行）→ `None`（开关修改只对之后收货的订单生效）；
/// - 开启 → 任务行（status=待生成，到期与文案均为此刻快照，
///   此后改天数/文案/关开关都不影响本行）。
#[must_use]
pub fn plan_snapshot(
    setting: &AutoCommentSetting,
    order_id: i32,
    shop_id: i32,
    user_id: i32,
    received_time: i32,
) -> Option<AutoCommentTask> {
    if !setting.is_enabled() {
        return None;
    }
    let days = setting.days.clamp(0, MAX_AUTO_COMMENT_DAYS);
    let due_time = received_time.saturating_add(days.saturating_mul(86_400));
    Some(AutoCommentTask {
        id: 0,
        order_id,
        shop_id,
        user_id,
        received_time,
        days,
        due_time,
        content: pick_content(&setting.contents, order_id),
        status: TASK_STATUS_PENDING,
        generated_time: 0,
        add_time: received_time,
        update_time: received_time,
    })
}

/// 单件订单项的生成决策。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemAction {
    /// 该订单项已有评价（真实评价或此前已生成的自动评价）→ 跳过。
    Skip,
    /// 生成一条自动评价。
    Generate,
}

/// 整单生成计划。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationPlan {
    /// 与输入订单项一一对应的决策。
    pub actions: Vec<ItemAction>,
    /// 本轮实际生成件数。
    pub generate_count: usize,
    /// 任务终态：生成 ≥1 件 → 已生成；0 件（全部已有评价）→ 已跳过。
    pub final_status: i32,
    /// 生成后整单是否全部订单项都有评价（→ 置 `order.comment_status = 1`）。
    pub order_fully_commented: bool,
}

/// 生成决策：逐件比对已有评价集合，买家已真实评价的跳过；
/// 此前已生成自动评价的件同样在已有集合里（幂等：重复扫描不重复生成）。
#[must_use]
pub fn plan_generation(
    item_ids: &[i32],
    commented_item_ids: &std::collections::HashSet<i32>,
) -> GenerationPlan {
    let actions: Vec<ItemAction> = item_ids
        .iter()
        .map(|id| {
            if commented_item_ids.contains(id) {
                ItemAction::Skip
            } else {
                ItemAction::Generate
            }
        })
        .collect();
    let generate_count = actions.iter().filter(|a| **a == ItemAction::Generate).count();
    // 执行后整单覆盖度：跳过件本就有评价、生成件本轮补齐 → 全部订单项均有评价。
    // （item_ids 为该订单全部订单项时恒成立；保留显式计算以防调用方传子集。）
    let already = item_ids
        .iter()
        .filter(|id| commented_item_ids.contains(id))
        .count();
    GenerationPlan {
        order_fully_commented: already + generate_count == item_ids.len(),
        final_status: if generate_count > 0 {
            TASK_STATUS_DONE
        } else {
            TASK_STATUS_SKIPPED
        },
        actions,
        generate_count,
    }
}

/// 抢占决策（纯状态机，SQL 侧以同条件 UPDATE 落地，双保险防重复生成）：
///
/// - 待生成（0）→ 可抢占；
/// - 生成中（3）且 `update_time` 距今超过僵尸阈值 → 可回收抢占；
/// - 其余（已生成/已跳过/新鲜生成中）→ 不可抢占。
#[must_use]
pub fn can_claim(status: i32, update_time: i32, now: i32) -> bool {
    match status {
        TASK_STATUS_PENDING => true,
        TASK_STATUS_CLAIMED => {
            i64::from(now) - i64::from(update_time) > CLAIM_STALE_SECONDS
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn setting(enabled: i32, days: i32, contents: Vec<&str>) -> AutoCommentSetting {
        AutoCommentSetting {
            shop_id: 42,
            enabled,
            days,
            contents: contents.into_iter().map(ToString::to_string).collect(),
            update_time: 0,
        }
    }

    // ————— 开关关不生成 —————

    #[test]
    fn disabled_shop_plans_no_snapshot() {
        // 真实语义：店铺从未配置（默认关）
        let s = AutoCommentSetting::defaults(42);
        assert!(plan_snapshot(&s, 1001, 42, 7, 1_700_000_000).is_none());
        // 店铺显式关闭
        let s = setting(0, 7, vec!["好评"]);
        assert!(plan_snapshot(&s, 1001, 42, 7, 1_700_000_000).is_none());
    }

    // ————— 快照后改设置不影响已收货订单 —————

    #[test]
    fn snapshot_is_self_contained_against_later_setting_changes() {
        let received = 1_700_000_000;
        let before = setting(1, 7, vec!["文案甲", "文案乙"]);
        let task = plan_snapshot(&before, 1002, 42, 7, received).expect("开启时应落快照");
        assert_eq!(task.due_time, received + 7 * 86_400);
        assert_eq!(task.days, 7);
        assert_eq!(task.content, pick_content(&before.contents, 1002));

        // 收货后商家改设置：天数改 1、文案全换、甚至关开关——
        // 已落快照的行字段一字不变（快照自包含，不回读设置）。
        let after = setting(0, 1, vec!["完全不同的文案"]);
        assert_ne!(task.due_time, received + after.days * 86_400);
        assert_eq!(task.content, "文案甲");
        assert!(!after.contents.contains(&task.content));
        assert_eq!(task.status, TASK_STATUS_PENDING);
    }

    #[test]
    fn snapshot_due_time_and_content_follow_receipt_time_setting() {
        let received = 1_700_100_000;
        let s = setting(1, 3, vec!["三天文案"]);
        let task = plan_snapshot(&s, 55, 42, 9, received).expect("应落快照");
        assert_eq!(task.received_time, received);
        assert_eq!(task.due_time, received + 3 * 86_400);
        assert_eq!(task.content, "三天文案");
        assert_eq!(task.shop_id, 42);
        assert_eq!(task.user_id, 9);
    }

    // ————— 文案轮换 / 兜底 —————

    #[test]
    fn pick_content_rotates_by_order_and_falls_back() {
        let contents = vec!["甲".to_string(), "乙".to_string(), "丙".to_string()];
        // 同单恒同文案
        assert_eq!(pick_content(&contents, 10), pick_content(&contents, 10));
        // 跨单分散：连续订单号覆盖全部文案
        let picked: HashSet<String> = (0..3).map(|i| pick_content(&contents, i)).collect();
        assert_eq!(picked.len(), 3);
        // 无文案 → 兜底文案
        assert_eq!(pick_content(&[], 10), DEFAULT_AUTO_COMMENT_CONTENT);
    }

    // ————— 已评价跳过 / 幂等 —————

    #[test]
    fn plan_generation_skips_already_commented_items() {
        // 订单 3 件商品：item 12 买家已真实评价
        let items = vec![11, 12, 13];
        let commented: HashSet<i32> = HashSet::from([12]);
        let plan = plan_generation(&items, &commented);
        assert_eq!(
            plan.actions,
            vec![ItemAction::Generate, ItemAction::Skip, ItemAction::Generate]
        );
        assert_eq!(plan.generate_count, 2);
        assert_eq!(plan.final_status, TASK_STATUS_DONE);
        assert!(plan.order_fully_commented);
    }

    #[test]
    fn plan_generation_all_commented_marks_task_skipped() {
        let items = vec![11, 12];
        let commented: HashSet<i32> = HashSet::from([11, 12]);
        let plan = plan_generation(&items, &commented);
        assert_eq!(plan.generate_count, 0);
        assert_eq!(plan.final_status, TASK_STATUS_SKIPPED);
        assert!(plan.order_fully_commented);
    }

    #[test]
    fn plan_generation_is_idempotent_on_rescan() {
        // 第一轮：2 件都生成
        let items = vec![21, 22];
        let plan1 = plan_generation(&items, &HashSet::new());
        assert_eq!(plan1.generate_count, 2);
        // 第二轮重复扫描：已生成件进入已有评价集合 → 0 生成、任务跳过，
        // 绝不重复生成（配合 order_id 唯一行 + 抢占状态机构成三重幂等）。
        let commented: HashSet<i32> = HashSet::from([21, 22]);
        let plan2 = plan_generation(&items, &commented);
        assert_eq!(plan2.generate_count, 0);
        assert_eq!(plan2.final_status, TASK_STATUS_SKIPPED);
    }

    // ————— 抢占状态机 —————

    #[test]
    fn claim_state_machine() {
        let now = 1_700_000_000;
        assert!(can_claim(TASK_STATUS_PENDING, now, now));
        // 新鲜的生成中不可重抢（防并发重复生成）
        assert!(!can_claim(TASK_STATUS_CLAIMED, now - 60, now));
        // 僵尸抢占（进程中断遗留）可回收
        assert!(can_claim(
            TASK_STATUS_CLAIMED,
            now - (CLAIM_STALE_SECONDS as i32) - 1,
            now
        ));
        // 终态不可抢
        assert!(!can_claim(TASK_STATUS_DONE, 0, now));
        assert!(!can_claim(TASK_STATUS_SKIPPED, 0, now));
    }

    // ————— 文案解析 / 清洗 / 校验 —————

    #[test]
    fn contents_roundtrip_and_dirty_data() {
        let list = vec![" 好评 ".to_string(), "".to_string(), "物流快".to_string()];
        let normalized = normalize_contents(&list);
        assert_eq!(normalized, vec!["好评".to_string(), "物流快".to_string()]);
        let raw = serialize_contents(&normalized);
        assert_eq!(parse_contents(&raw), normalized);
        // 脏数据不 panic、不当文案
        assert!(parse_contents("not-json").is_empty());
        assert!(parse_contents("{}").is_empty());
        assert!(parse_contents("").is_empty());
    }

    #[test]
    fn validate_setting_bounds() {
        assert!(validate_setting(7, &["好".to_string()]).is_ok());
        assert!(validate_setting(0, &[]).is_ok());
        assert!(validate_setting(90, &[]).is_ok());
        assert!(validate_setting(91, &[]).is_err());
        assert!(validate_setting(-1, &[]).is_err());
        let too_many: Vec<String> = (0..11).map(|i| format!("文案{i}")).collect();
        assert!(validate_setting(7, &too_many).is_err());
        let too_long = vec!["好".repeat(201)];
        assert!(validate_setting(7, &too_long).is_err());
    }
}
