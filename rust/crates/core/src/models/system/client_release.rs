//! 客户端已发布版本登记 DTO / VO（system::client_release，我方新增）。
//!
//! admin 自助热更新 P4（设计见 `~/workspace/hot-update-design/REPORT.md` §5.4）：
//! 小程序与 App 无法由服务器热更，只能提醒。提醒的事实源分工：
//! - 「最新版本是否需要客户端发版」来自统一产品 Release 清单的兼容标志
//!   （`ManifestCompat::requires_mini_program_update` / `requires_app_update`，
//!   由 P1 的 `SystemUpdateStatusVO` 透出，本模块**只读不改**该契约）；
//! - 「运营是否已完成发版」来自本模块的人工登记表 `sys_client_release`。
//!
//! 消警口径（派生态）：某渠道存在 `product_version >= 目标产品版本` 的登记
//! 即视为已覆盖（[`is_covered`]），提醒自动消失；不落提醒行、无已读状态。
//!
//! JSON 字段全部 camelCase。

use serde::{Deserialize, Serialize};

/// 渠道：小程序。
pub const CHANNEL_MINI_PROGRAM: &str = "mini_program";
/// 渠道：App。
pub const CHANNEL_APP: &str = "app";

/// 渠道是否合法。
#[must_use]
pub fn is_valid_channel(channel: &str) -> bool {
    matches!(channel, CHANNEL_MINI_PROGRAM | CHANNEL_APP)
}

/// 渠道展示名。
#[must_use]
pub fn channel_label(channel: &str) -> &'static str {
    match channel {
        CHANNEL_MINI_PROGRAM => "小程序",
        CHANNEL_APP => "App",
        _ => "未知渠道",
    }
}

/// 点分版本号比较：`a >= b` 是否成立。
///
/// 按段数值比较（`1.10.0 > 1.9.0`）；任一段非纯数字时退化为字符串相等
/// 比较（相等才算 `>=`），避免对非标准版本号给出错误的大小结论。
#[must_use]
pub fn version_ge(a: &str, b: &str) -> bool {
    let parse = |s: &str| -> Option<Vec<u64>> {
        s.trim()
            .trim_start_matches('v')
            .split('.')
            .map(|seg| {
                if seg.is_empty() || !seg.bytes().all(|c| c.is_ascii_digit()) {
                    None
                } else {
                    seg.parse::<u64>().ok()
                }
            })
            .collect()
    };
    match (parse(a), parse(b)) {
        (Some(x), Some(y)) => {
            let len = x.len().max(y.len());
            for i in 0..len {
                let xi = x.get(i).copied().unwrap_or(0);
                let yi = y.get(i).copied().unwrap_or(0);
                if xi != yi {
                    return xi > yi;
                }
            }
            true
        }
        _ => a.trim() == b.trim(),
    }
}

/// 某渠道的最新登记是否覆盖目标产品版本。
#[must_use]
pub fn is_covered(latest_registered_product_version: Option<&str>, target: &str) -> bool {
    if target.trim().is_empty() {
        return false;
    }
    match latest_registered_product_version {
        Some(v) => version_ge(v, target),
        None => false,
    }
}

/// 登记行（仓储层用）。
#[derive(Debug, Clone, Default)]
pub struct ClientReleaseRow {
    pub id: i64,
    pub channel: String,
    pub product_version: String,
    pub client_version: String,
    pub note: String,
    pub operator_id: i64,
    pub operator_name: String,
    pub created_at: i64,
}

/// 登记行 VO。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ClientReleaseVO {
    pub id: i64,
    /// `mini_program` / `app`。
    pub channel: String,
    /// 展示名：小程序 / App。
    pub channel_label: String,
    pub product_version: String,
    pub client_version: String,
    pub note: String,
    pub operator_name: String,
    pub created_at: i64,
}

impl From<&ClientReleaseRow> for ClientReleaseVO {
    fn from(row: &ClientReleaseRow) -> Self {
        Self {
            id: row.id,
            channel: row.channel.clone(),
            channel_label: channel_label(&row.channel).to_string(),
            product_version: row.product_version.clone(),
            client_version: row.client_version.clone(),
            note: row.note.clone(),
            operator_name: row.operator_name.clone(),
            created_at: row.created_at,
        }
    }
}

/// 单渠道登记概览。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ChannelReleaseVO {
    /// `mini_program` / `app`。
    pub channel: String,
    /// 展示名：小程序 / App。
    pub label: String,
    /// 该渠道最近一条登记（无则 null）。
    pub latest: Option<ClientReleaseVO>,
    /// 是否已覆盖目标产品版本（未传目标版本时恒 false，前端据此与清单标志组合出提醒）。
    pub covered: bool,
    /// App 渠道附带：APP版本页现行配置的 Android 版本号（原版有 appVersion 面，只读展示）。
    pub configured_android_version: Option<String>,
    /// App 渠道附带：APP版本页现行配置的 iOS 版本号。
    pub configured_ios_version: Option<String>,
}

/// 登记总览（登记页 + 提醒组件共用）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ClientReleaseOverviewVO {
    /// 本次比对的目标产品版本（前端从系统更新状态透传；未传为 null）。
    pub target_product_version: Option<String>,
    /// 两渠道概览（恒两行，顺序 mini_program / app）。
    pub channels: Vec<ChannelReleaseVO>,
    /// 最近登记记录（跨渠道，时间倒序）。
    pub recent: Vec<ClientReleaseVO>,
}

/// 登记入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RegisterClientReleaseParam {
    /// `mini_program` / `app`。
    pub channel: String,
    /// 本次发版对应的产品版本（x.y.z）。
    pub product_version: String,
    /// 已发布的客户端版本（小程序版本号 / App 版本号）。
    pub client_version: String,
    /// 备注（可空，截断 500 字）。
    pub note: String,
}

/// 删除登记入参（纠错用，删除后可重登）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RemoveClientReleaseParam {
    pub id: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_ge_numeric_segments() {
        assert!(version_ge("1.10.0", "1.9.0"));
        assert!(version_ge("0.0.3", "0.0.3"));
        assert!(version_ge("0.0.4", "0.0.3"));
        assert!(!version_ge("0.0.2", "0.0.3"));
        assert!(version_ge("v1.2", "1.2.0"));
        assert!(!version_ge("1.2", "1.2.1"));
    }

    #[test]
    fn version_ge_non_numeric_falls_back_to_equality() {
        assert!(version_ge("release-a", "release-a"));
        assert!(!version_ge("release-a", "release-b"));
    }

    #[test]
    fn covered_requires_registration_at_or_above_target() {
        assert!(is_covered(Some("0.0.3"), "0.0.3"));
        assert!(is_covered(Some("0.0.4"), "0.0.3"));
        assert!(!is_covered(Some("0.0.2"), "0.0.3"));
        assert!(!is_covered(None, "0.0.3"));
        assert!(!is_covered(Some("0.0.3"), ""));
    }

    #[test]
    fn channel_validation() {
        assert!(is_valid_channel(CHANNEL_MINI_PROGRAM));
        assert!(is_valid_channel(CHANNEL_APP));
        assert!(!is_valid_channel("h5"));
    }
}
