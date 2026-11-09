//! 首页面板域（analytics::panel）DTO / 纯函数。
//!
//! 对应原 Java `com.nextrade.adminapi.controller.panel.PanelController`
//! （`@RequestMapping("/adminapi/panel/panel")`，7 个端点）与
//! `SalesStatisticsServiceImpl#getPanelIndex/getPanelVendorIndex/
//! getPanelStatisticalData/getVendorPanelStatisticalData`。
//!
//! # 与 Java 的差异（有意降级，逐条记录）
//! - `index` / `vendorIndex` 返回**动态 JSON**：字段名对齐 Java `PanelIndexVO` /
//!   `PanelVendorIndexVO` 的一级键（`awaitPay/awaitShip/.../todayOrderAmount/...`），
//!   但 `consoleData`/`realTimeData` 等深度嵌套聚合、访问量/浏览量（需埋点表）、
//!   供应商结算域指标未迁移，对应值为 0 / 空对象。
//! - `statisticalData`：按天聚合订单数与支付金额（真实 SQL）；访问量/浏览量曲线恒 0。
//! - `searchMenu`：真实查询 `authority`（LIKE 名称），返回 `authorityId/authorityName/routeLink` 子集。
//! - `shortcutMenu` / `saveShortcutMenu`：读写 `panel_shortcut_menu`（`admin_id + shop_id` 维度），
//!   与 Java `PanelShortcutMenuServiceImpl` 的去空/去重/保序/上限 20 一致。

use serde::{Deserialize, Serialize};

/// Java `HttpStatus.SERVICE_DATA_ERROR`。
pub const SERVICE_DATA_ERROR: i32 = 1001;

/// Java `PanelShortcutMenuServiceImpl.MAX_MENU_SIZE`。
pub const MAX_MENU_SIZE: usize = 20;

/// 默认趋势天数（Java `@RequestParam(defaultValue = "7")`）。
pub const DEFAULT_DAYS: i64 = 7;

/// `GET .../statisticalData?days=` 查询。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DaysQuery {
    pub days: Option<i64>,
}

impl DaysQuery {
    /// 归一化天数：缺省 7，范围 1..=365（防全表扫描）。
    #[must_use]
    pub fn resolved_days(&self) -> i64 {
        self.days.unwrap_or(DEFAULT_DAYS).clamp(1, 365)
    }
}

/// `GET .../searchMenu?keyword=` 查询。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchMenuQuery {
    pub keyword: Option<String>,
}

/// 快捷菜单保存入参（Java `ShortcutMenuSaveDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShortcutMenuSaveDTO {
    pub list: Option<Vec<String>>,
}

/// 快捷菜单（Java `ShortcutMenuVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShortcutMenuVO {
    pub list: Vec<String>,
}

/// Java `saveShortcutMenu` 的归一化：去空、去首尾空白、去重、保序、上限 20。
#[must_use]
pub fn normalize_menu_keys(keys: Option<&[String]>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for key in keys.unwrap_or(&[]) {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !out.iter().any(|k| k == trimmed) {
            out.push(trimmed.to_string());
        }
        if out.len() >= MAX_MENU_SIZE {
            break;
        }
    }
    out
}

/// 增长率（Java `BigDecimal` 口径：基数为 0 时：本期为 0 → 0，否则 100）。
#[must_use]
pub fn growth_rate(today: f64, yesterday: f64) -> f64 {
    if yesterday.abs() < f64::EPSILON {
        if today.abs() < f64::EPSILON {
            0.0
        } else {
            100.0
        }
    } else {
        ((today - yesterday) / yesterday * 10000.0).round() / 100.0
    }
}

/// 秒级时间戳 → 当地日期 `YYYY-MM-DD`（趋势横轴）。
#[must_use]
pub fn day_key(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| {
            dt.with_timezone(
                &chrono::FixedOffset::east_opt(8 * 3600)
                    .unwrap_or_else(|| chrono::FixedOffset::east_opt(0).expect("utc")),
            )
            .format("%Y-%m-%d")
            .to_string()
        })
        .unwrap_or_default()
}

/// 今日/昨日时间窗口 `(today_start, now, yesterday_start, yesterday_end)`（秒）。
#[must_use]
pub fn day_windows(now: i64) -> (i64, i64, i64, i64) {
    let day = 86_400;
    // 东八区（UTC+8）当地日起点：先平移时区再按天取整，最后平移回 UTC 时间戳
    let shifted = now + 8 * 3600;
    let today_start = shifted - shifted % day - 8 * 3600;
    let yesterday_start = today_start - day;
    (today_start, now, yesterday_start, today_start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_query_is_clamped() {
        assert_eq!(DaysQuery::default().resolved_days(), 7);
        assert_eq!(DaysQuery { days: Some(30) }.resolved_days(), 30);
        assert_eq!(DaysQuery { days: Some(0) }.resolved_days(), 1);
        assert_eq!(DaysQuery { days: Some(9999) }.resolved_days(), 365);
    }

    #[test]
    fn menu_normalization_matches_java() {
        let keys = vec![
            "a".to_string(),
            " ".to_string(),
            " a ".to_string(),
            "b".to_string(),
        ];
        assert_eq!(normalize_menu_keys(Some(&keys)), vec!["a", "b"]);
        let many: Vec<String> = (0..30).map(|i| format!("k{i}")).collect();
        assert_eq!(normalize_menu_keys(Some(&many)).len(), MAX_MENU_SIZE);
        assert!(normalize_menu_keys(None).is_empty());
    }

    #[test]
    fn growth_rate_matches_java_bigdecimal_rounding() {
        assert_eq!(growth_rate(0.0, 0.0), 0.0);
        assert_eq!(growth_rate(10.0, 0.0), 100.0);
        assert_eq!(growth_rate(120.0, 100.0), 20.0);
        assert_eq!(growth_rate(80.0, 100.0), -20.0);
    }

    #[test]
    fn day_windows_are_ordered() {
        let (t0, now, y0, y1) = day_windows(1_700_000_000);
        assert!(y1 <= t0);
        assert_eq!(y1 - y0, 86_400);
        assert!(now >= t0);
        assert!(!day_key(1_700_000_000).is_empty());
    }
}
