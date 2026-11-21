//! 同城配送送达时间选择 DTO 与纯逻辑（我方新增，论坛帖 467744 +
//! 预约配送子项 297743）。
//!
//! 口径（2026-11-21 定版，见迁移 025 与服务层 `trade::delivery_slot`）：
//! - 时段定义挂门店同城配送配置 `shop_local_delivery_config.time_slots`
//!   （JSON 数组文本）：`[{"start":"09:00","end":"11:00","capacity":20}]`，
//!   `capacity = 0` 表示不限量；
//! - 买家在结算页选「日期 + 时段」：日期范围为今日起 `booking_days` 天
//!   （含今日，1 = 仅今日，上限 7）；
//! - 截单口径：今日时段在时段开始前 `cutoff_minutes` 分钟停止接单，即
//!   `slot_start_ts - now >= cutoff_minutes * 60` 才可选（未来日期不受此限）；
//! - 容量口径：按 (门店, 时段开始时间戳) 统计未取消（order_status 非
//!   3/4）、未删除、有效的根订单数，已约满的时段下发 `full = true` 且
//!   下单时再次校验（防并发超卖）；
//! - 整单一个时段：一次结算只选一个时段，作用于订单内全部开通时段
//!   选择的同城店铺（多店校验逐店执行）；
//! - 时间一律按服务器本地时区（`chrono::Local`，与订单时间格式化
//!   `fmt_time` 同口径）换算为 Unix 秒存 `order_delivery_slot`。

use serde::{Deserialize, Serialize};

/// 门店时段定义（配置 JSON 的一行；管理端保存/下发共用）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DeliverySlotDef {
    /// 开始时刻 `HH:mm`（24 小时制）。
    pub start: String,
    /// 结束时刻 `HH:mm`，必须晚于 start（不跨夜）。
    pub end: String,
    /// 该时段单店可约订单数上限；0 = 不限量。
    pub capacity: i32,
}

/// 买家提交的送达时段选择（`OrderCreateDTO.deliveryTimeSlot`）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DeliverySlotSelection {
    /// 送达日期 `yyyy-MM-dd`（服务器本地时区）。
    pub date: String,
    /// 时段开始 `HH:mm`，须与门店时段定义逐字一致。
    pub start: String,
    /// 时段结束 `HH:mm`。
    pub end: String,
}

/// 校验通过后的时段（时间戳形态，落库/下发共用）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSlot {
    /// 时段开始 Unix 秒（本地时区换算）。
    pub slot_start: i32,
    /// 时段结束 Unix 秒。
    pub slot_end: i32,
    /// 展示文案：`yyyy-MM-dd HH:mm-HH:mm`。
    pub slot_text: String,
}

/// 结算页下发的单时段选项。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliverySlotOption {
    /// `HH:mm`。
    pub start: String,
    /// `HH:mm`。
    pub end: String,
    /// 展示文案 `HH:mm-HH:mm`。
    pub label: String,
    /// 时段开始 Unix 秒。
    pub slot_start: i32,
    /// 剩余可约数；不限量时为 null。
    pub remaining: Option<i64>,
    /// 是否已约满（满则不可选）。
    pub full: bool,
    /// 是否可选（截单口径未过时为 false，与 full 区分原因）。
    pub available: bool,
}

/// 结算页下发的单日选项。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryDayOption {
    /// `yyyy-MM-dd`。
    pub date: String,
    /// 展示文案：今日 / 明日 / `M月d日`。
    pub date_label: String,
    pub slots: Vec<DeliverySlotOption>,
}

/// 解析配置 JSON 文本为时段定义（空/非法 → 空列表；逐行做格式过滤，
/// 非法行直接丢弃，保证读路径永不因脏配置报错）。
#[must_use]
pub fn parse_slot_defs(raw: &str) -> Vec<DeliverySlotDef> {
    let text = raw.trim();
    if text.is_empty() {
        return Vec::new();
    }
    let Ok(mut defs) = serde_json::from_str::<Vec<DeliverySlotDef>>(text) else {
        return Vec::new();
    };
    defs.retain(|d| valid_def(d));
    defs.sort_by(|a, b| a.start.cmp(&b.start));
    defs.dedup_by(|a, b| a.start == b.start && a.end == b.end);
    defs
}

/// 时段定义序列化为配置 JSON 文本（保存路径用；先过滤 + 排序）。
#[must_use]
pub fn slot_defs_json(defs: &[DeliverySlotDef]) -> String {
    let mut clean: Vec<DeliverySlotDef> = defs.iter().filter(|d| valid_def(d)).cloned().collect();
    clean.sort_by(|a, b| a.start.cmp(&b.start));
    clean.dedup_by(|a, b| a.start == b.start && a.end == b.end);
    serde_json::to_string(&clean).unwrap_or_default()
}

/// 单行时段定义是否合法：`HH:mm` 格式、start < end、capacity >= 0。
#[must_use]
pub fn valid_def(d: &DeliverySlotDef) -> bool {
    let (Some(s), Some(e)) = (parse_hm(&d.start), parse_hm(&d.end)) else {
        return false;
    };
    s < e && d.capacity >= 0
}

/// `HH:mm` → 当日分钟数（0..=1439）；非法格式返回 None。
#[must_use]
pub fn parse_hm(s: &str) -> Option<i32> {
    let t = s.trim();
    let (h, m) = t.split_once(':')?;
    if h.len() > 2 || m.len() != 2 {
        return None;
    }
    let h: i32 = h.parse().ok()?;
    let m: i32 = m.parse().ok()?;
    if !(0..24).contains(&h) || !(0..60).contains(&m) {
        return None;
    }
    Some(h * 60 + m)
}

/// 本地日期 + `HH:mm` → Unix 秒（本地时区无对应时刻时返回 None）。
fn local_ts(date: chrono::NaiveDate, hm: &str) -> Option<i32> {
    let mins = parse_hm(hm)?;
    let naive = date.and_hms_opt((mins / 60) as u32, (mins % 60) as u32, 0)?;
    match chrono::TimeZone::from_local_datetime(&chrono::Local, &naive) {
        chrono::LocalResult::Single(dt) => i32::try_from(dt.timestamp()).ok(),
        _ => None,
    }
}

/// 生成结算页时段选项（纯函数，容量已约数由调用方按 (日期, 开始分钟) 注入）。
///
/// `booked`：key = 时段开始 Unix 秒，value = 已约订单数。
#[must_use]
pub fn build_day_options(
    defs: &[DeliverySlotDef],
    booking_days: i32,
    cutoff_minutes: i32,
    now: chrono::DateTime<chrono::Local>,
    booked: &std::collections::HashMap<i32, i64>,
) -> Vec<DeliveryDayOption> {
    let days = booking_days.clamp(1, 7);
    let today = now.date_naive();
    let now_ts = now.timestamp();
    let mut out = Vec::new();
    for offset in 0..days {
        let date = today + chrono::Duration::days(i64::from(offset));
        let date_label = match offset {
            0 => "今日".to_string(),
            1 => "明日".to_string(),
            _ => date.format("%m月%d日").to_string(),
        };
        let mut slots = Vec::new();
        for def in defs {
            let (Some(slot_start), Some(slot_end)) =
                (local_ts(date, &def.start), local_ts(date, &def.end))
            else {
                continue;
            };
            // 截单口径：仅今日受截单提前量约束（未来日期整日可选）
            let available =
                offset > 0 || i64::from(slot_start) - now_ts >= i64::from(cutoff_minutes.max(0)) * 60;
            let booked_n = booked.get(&slot_start).copied().unwrap_or(0);
            let (remaining, full) = if def.capacity > 0 {
                let left = i64::from(def.capacity) - booked_n;
                (Some(left.max(0)), left <= 0)
            } else {
                (None, false)
            };
            slots.push(DeliverySlotOption {
                start: def.start.clone(),
                end: def.end.clone(),
                label: format!("{}-{}", def.start, def.end),
                slot_start,
                remaining,
                full,
                available,
            });
        }
        out.push(DeliveryDayOption {
            date: date.format("%Y-%m-%d").to_string(),
            date_label,
            slots,
        });
    }
    out
}

/// 校验买家选择（纯口径部分：日期范围/时段命中/截单；容量由服务层
/// 叠加已约数判断）。成功返回时间戳形态。
pub fn validate_selection(
    defs: &[DeliverySlotDef],
    booking_days: i32,
    cutoff_minutes: i32,
    now: chrono::DateTime<chrono::Local>,
    sel: &DeliverySlotSelection,
) -> Result<ResolvedSlot, String> {
    let days = booking_days.clamp(1, 7);
    let date = chrono::NaiveDate::parse_from_str(sel.date.trim(), "%Y-%m-%d")
        .map_err(|_| "送达日期格式不正确".to_string())?;
    let today = now.date_naive();
    let offset = (date - today).num_days();
    if offset < 0 || offset >= i64::from(days) {
        return Err("所选送达日期超出可预约范围".to_string());
    }
    let def = defs
        .iter()
        .find(|d| d.start == sel.start.trim() && d.end == sel.end.trim())
        .ok_or_else(|| "所选送达时段已变更，请重新选择".to_string())?;
    let slot_start = local_ts(date, &def.start).ok_or_else(|| "送达时段不合法".to_string())?;
    let slot_end = local_ts(date, &def.end).ok_or_else(|| "送达时段不合法".to_string())?;
    if offset == 0
        && i64::from(slot_start) - now.timestamp() < i64::from(cutoff_minutes.max(0)) * 60
    {
        return Err("所选时段已停止接单，请选择其他时段".to_string());
    }
    Ok(ResolvedSlot {
        slot_start,
        slot_end,
        slot_text: format!("{} {}-{}", sel.date.trim(), def.start, def.end),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn defs() -> Vec<DeliverySlotDef> {
        vec![
            DeliverySlotDef { start: "09:00".into(), end: "11:00".into(), capacity: 2 },
            DeliverySlotDef { start: "14:00".into(), end: "16:00".into(), capacity: 0 },
            DeliverySlotDef { start: "18:00".into(), end: "20:00".into(), capacity: 1 },
        ]
    }

    fn now_at(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> chrono::DateTime<chrono::Local> {
        chrono::Local
            .with_ymd_and_hms(y, mo, d, h, mi, 0)
            .single()
            .expect("测试时刻应合法")
    }

    #[test]
    fn parse_defs_filters_invalid_and_sorts() {
        let raw = r#"[
            {"start":"14:00","end":"16:00","capacity":0},
            {"start":"25:00","end":"26:00","capacity":1},
            {"start":"11:00","end":"09:00","capacity":1},
            {"start":"09:00","end":"11:00","capacity":-1},
            {"start":"09:00","end":"11:00","capacity":3}
        ]"#;
        let got = parse_slot_defs(raw);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].start, "09:00");
        assert_eq!(got[1].start, "14:00");
        assert!(parse_slot_defs("").is_empty());
        assert!(parse_slot_defs("not-json").is_empty());
    }

    #[test]
    fn hm_parsing() {
        assert_eq!(parse_hm("09:30"), Some(570));
        assert_eq!(parse_hm("9:05"), Some(545));
        assert_eq!(parse_hm("24:00"), None);
        assert_eq!(parse_hm("12:60"), None);
        assert_eq!(parse_hm("1200"), None);
    }

    #[test]
    fn options_respect_cutoff_and_capacity() {
        // 2026-11-21 08:45 本地时间，截单 30 分钟：09:00 时段仅剩 15 分钟 < 30 → 不可选
        let now = now_at(2026, 11, 21, 8, 45);
        let mut booked = std::collections::HashMap::new();
        // 今日 18:00 时段已约 1 单（容量 1）→ 约满
        let evening = build_day_options(&defs(), 2, 30, now, &booked);
        let evening_start = evening[0].slots[2].slot_start;
        booked.insert(evening_start, 1);
        let days = build_day_options(&defs(), 2, 30, now, &booked);
        assert_eq!(days.len(), 2);
        assert_eq!(days[0].date_label, "今日");
        assert_eq!(days[1].date_label, "明日");
        assert!(!days[0].slots[0].available, "09:00 应被截单口径拦下");
        assert!(days[0].slots[1].available);
        assert!(days[0].slots[2].full, "18:00 容量 1 已约 1 应约满");
        assert_eq!(days[0].slots[2].remaining, Some(0));
        // 明日不受截单约束且容量独立
        assert!(days[1].slots[0].available);
        assert!(!days[1].slots[2].full);
        assert_eq!(days[1].slots[0].remaining, Some(2));
        // 不限量时段 remaining 为 null
        assert_eq!(days[0].slots[1].remaining, None);
    }

    #[test]
    fn validate_selection_paths() {
        let now = now_at(2026, 11, 21, 8, 0);
        let ok = DeliverySlotSelection { date: "2026-11-22".into(), start: "09:00".into(), end: "11:00".into() };
        let resolved = validate_selection(&defs(), 2, 30, now, &ok).expect("明日时段应合法");
        assert!(resolved.slot_end > resolved.slot_start);
        assert_eq!(resolved.slot_text, "2026-11-22 09:00-11:00");

        // 超出可约天数（booking_days = 2 仅今日+明日）
        let far = DeliverySlotSelection { date: "2026-11-25".into(), ..ok.clone() };
        assert!(validate_selection(&defs(), 2, 30, now, &far).is_err());
        // 时段未命中定义
        let miss = DeliverySlotSelection { start: "10:00".into(), ..ok.clone() };
        assert!(validate_selection(&defs(), 2, 30, now, &miss).is_err());
        // 今日 09:00 距 08:50 仅 10 分钟 < 截单 30 分钟
        let late_now = now_at(2026, 11, 21, 8, 50);
        let today_sel = DeliverySlotSelection { date: "2026-11-21".into(), ..ok };
        assert!(validate_selection(&defs(), 2, 30, late_now, &today_sel).is_err());
    }

    #[test]
    fn defs_json_roundtrip() {
        let json = slot_defs_json(&defs());
        assert_eq!(parse_slot_defs(&json), defs());
    }
}
