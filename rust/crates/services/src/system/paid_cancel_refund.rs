//! 已付未发货买家取消直退设置（平台级，我方新增·按用户口径）。
//!
//! 论坛帖 347748「未完成订单取消极速退款」深评后用户 2026-11-21
//! 20:45–20:46 拍板口径：
//! - 做成设置项，**主开关默认关闭**（关闭时全站零行为变化，买家侧
//!   不出现入口，端点拒绝）；
//! - 需要财务审核：子开关「取消后进财务审核队列」**默认开**；
//! - 子开关「小额免审」默认关 + 免审金额上限：开启且退款额 ≤ 上限
//!   时申请自动通过、直达出款执行，记录标明系统免审；
//! - 两子开关都不命中时（免审关/超上限 且 审核队列关）直接出款。
//!
//! 存储：`config` 表单行 `biz_code = "paidCancelRefund"`，`biz_val`
//! 为整个配置的 JSON（与 `priceDisplay` / `buyStyle` 同属「单 code
//! 整块 JSON」型配置）。未配置时返回内置默认（全关 + 审核开）。
//!
//! 资金执行一律复用财务退款审核链（`refund_apply` 申请 → 财务审核
//! → `execute_refund_money` 出款），本模块只负责设置与路由决策。

use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::str::FromStr;

/// config 表中的业务编码。
pub const PAID_CANCEL_REFUND_CODE: &str = "paidCancelRefund";

/// 买家取消退款的执行路由（设置矩阵的决策结果）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaidCancelRoute {
    /// 小额免审：自动通过，直达出款执行（记录标明系统免审）。
    Exempt,
    /// 进财务审核队列：落 Wait 态，等财务审核通过后出款。
    FinanceAudit,
    /// 直接出款：不进队列、非免审，直达出款执行。
    Direct,
}

/// 已付未发货买家取消直退设置（生效值）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaidCancelRefundSettings {
    /// 主开关「已付未发货买家可取消退款」，默认关。
    pub enabled: bool,
    /// 子开关「小额免审」，默认关。
    pub small_exempt_enabled: bool,
    /// 免审金额上限（退款额 ≤ 上限才免审）；≤ 0 视为不设上限命中。
    pub small_exempt_limit: Decimal,
    /// 子开关「取消后进财务审核队列」，默认开。
    pub finance_audit_enabled: bool,
}

impl Default for PaidCancelRefundSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            small_exempt_enabled: false,
            small_exempt_limit: Decimal::ZERO,
            finance_audit_enabled: true,
        }
    }
}

impl PaidCancelRefundSettings {
    /// 按退款金额决策执行路由（主开关关闭时调用方应先行拒绝，
    /// 本函数只在 `enabled == true` 的前提下有意义）。
    ///
    /// 优先级：小额免审命中 > 财务审核队列 > 直接出款。
    #[must_use]
    pub fn route_for(&self, amount: Decimal) -> PaidCancelRoute {
        if self.small_exempt_enabled
            && self.small_exempt_limit > Decimal::ZERO
            && amount <= self.small_exempt_limit
        {
            return PaidCancelRoute::Exempt;
        }
        if self.finance_audit_enabled {
            return PaidCancelRoute::FinanceAudit;
        }
        PaidCancelRoute::Direct
    }

    /// 转回 JSON（与落库结构一致；金额以字符串保存避免浮点精度）。
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "enabled": self.enabled,
            "smallExemptEnabled": self.small_exempt_enabled,
            "smallExemptLimit": self.small_exempt_limit.to_string(),
            "financeAuditEnabled": self.finance_audit_enabled,
        })
    }
}

/// 内置默认配置（未保存过设置时生效）：主开关关、免审关、审核开。
#[must_use]
pub fn default_paid_cancel_refund() -> Value {
    PaidCancelRefundSettings::default().to_json()
}

fn parse_limit(v: Option<&Value>) -> Decimal {
    match v {
        Some(Value::String(s)) => Decimal::from_str(s.trim()).unwrap_or(Decimal::ZERO),
        Some(Value::Number(n)) => n
            .as_f64()
            .and_then(|f| Decimal::from_str(&format!("{f:.2}")).ok())
            .unwrap_or(Decimal::ZERO),
        _ => Decimal::ZERO,
    }
}

/// 校验/清洗一份配置（保存与读取共用）。
///
/// - 三个开关缺省取默认值（主开关/免审 = false，审核 = true）；
/// - 免审上限钳制到 [0, 1_000_000]，负数归 0（= 不命中免审）；
/// - 输入不是对象时返回默认配置。
#[must_use]
pub fn sanitize_paid_cancel_refund(input: &Value) -> Value {
    let Some(obj) = input.as_object() else {
        return default_paid_cancel_refund();
    };
    let d = PaidCancelRefundSettings::default();
    let limit = parse_limit(obj.get("smallExemptLimit"))
        .max(Decimal::ZERO)
        .min(Decimal::from(1_000_000));
    PaidCancelRefundSettings {
        enabled: obj.get("enabled").and_then(Value::as_bool).unwrap_or(d.enabled),
        small_exempt_enabled: obj
            .get("smallExemptEnabled")
            .and_then(Value::as_bool)
            .unwrap_or(d.small_exempt_enabled),
        small_exempt_limit: limit,
        finance_audit_enabled: obj
            .get("financeAuditEnabled")
            .and_then(Value::as_bool)
            .unwrap_or(d.finance_audit_enabled),
    }
    .to_json()
}

/// 由存储的原始 JSON 文本得到生效配置：解析失败/非对象 ⇒ 默认配置。
#[must_use]
pub fn effective_paid_cancel_refund(stored: Option<&str>) -> PaidCancelRefundSettings {
    let raw = stored.and_then(|s| serde_json::from_str::<Value>(s).ok());
    let clean = match raw {
        Some(v) if v.is_object() => sanitize_paid_cancel_refund(&v),
        _ => default_paid_cancel_refund(),
    };
    let limit = parse_limit(clean.get("smallExemptLimit"));
    PaidCancelRefundSettings {
        enabled: clean
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        small_exempt_enabled: clean
            .get("smallExemptEnabled")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        small_exempt_limit: limit,
        finance_audit_enabled: clean
            .get("financeAuditEnabled")
            .and_then(Value::as_bool)
            .unwrap_or(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn settings(
        small_exempt: bool,
        limit: &str,
        finance_audit: bool,
    ) -> PaidCancelRefundSettings {
        PaidCancelRefundSettings {
            enabled: true,
            small_exempt_enabled: small_exempt,
            small_exempt_limit: dec(limit),
            finance_audit_enabled: finance_audit,
        }
    }

    #[test]
    fn defaults_are_off_with_audit_on() {
        let d = PaidCancelRefundSettings::default();
        assert!(!d.enabled, "主开关默认关");
        assert!(!d.small_exempt_enabled, "小额免审默认关");
        assert!(d.finance_audit_enabled, "进审核队列默认开");
        assert_eq!(effective_paid_cancel_refund(None), d);
        assert_eq!(effective_paid_cancel_refund(Some("not json")), d);
        assert_eq!(effective_paid_cancel_refund(Some("[1,2]")), d);
    }

    /// 开关矩阵逐组合（主开关开为前提；免审 × 审核 × 金额三维）。
    #[test]
    fn route_matrix_all_combinations() {
        let amt_small = dec("50.00");
        let amt_big = dec("500.00");
        // 免审关 + 审核开 → 全部进队列
        let s = settings(false, "100.00", true);
        assert_eq!(s.route_for(amt_small), PaidCancelRoute::FinanceAudit);
        assert_eq!(s.route_for(amt_big), PaidCancelRoute::FinanceAudit);
        // 免审关 + 审核关 → 全部直达
        let s = settings(false, "100.00", false);
        assert_eq!(s.route_for(amt_small), PaidCancelRoute::Direct);
        assert_eq!(s.route_for(amt_big), PaidCancelRoute::Direct);
        // 免审开 + 审核开 → ≤上限免审，>上限进队列
        let s = settings(true, "100.00", true);
        assert_eq!(s.route_for(amt_small), PaidCancelRoute::Exempt);
        assert_eq!(s.route_for(dec("100.00")), PaidCancelRoute::Exempt, "等于上限命中免审");
        assert_eq!(s.route_for(amt_big), PaidCancelRoute::FinanceAudit);
        // 免审开 + 审核关 → ≤上限免审，>上限直达
        let s = settings(true, "100.00", false);
        assert_eq!(s.route_for(amt_small), PaidCancelRoute::Exempt);
        assert_eq!(s.route_for(amt_big), PaidCancelRoute::Direct);
        // 免审开但上限为 0 → 永不命中免审
        let s = settings(true, "0", true);
        assert_eq!(s.route_for(amt_small), PaidCancelRoute::FinanceAudit);
        let s = settings(true, "0", false);
        assert_eq!(s.route_for(amt_small), PaidCancelRoute::Direct);
    }

    #[test]
    fn sanitize_clamps_limit_and_keeps_switch_defaults() {
        let out = sanitize_paid_cancel_refund(&json!({
            "enabled": true,
            "smallExemptLimit": "-5"
        }));
        assert_eq!(out["enabled"], json!(true));
        assert_eq!(out["smallExemptEnabled"], json!(false), "缺省关");
        assert_eq!(out["financeAuditEnabled"], json!(true), "缺省开");
        assert_eq!(out["smallExemptLimit"], json!("0"), "负上限归 0");
        let out = sanitize_paid_cancel_refund(&json!({ "smallExemptLimit": 88.5 }));
        assert_eq!(out["smallExemptLimit"], json!("88.50"), "数字上限转金额字符串");
    }

    #[test]
    fn effective_round_trips_stored_json() {
        let stored = json!({
            "enabled": true,
            "smallExemptEnabled": true,
            "smallExemptLimit": "200.00",
            "financeAuditEnabled": false
        })
        .to_string();
        let s = effective_paid_cancel_refund(Some(&stored));
        assert!(s.enabled && s.small_exempt_enabled && !s.finance_audit_enabled);
        assert_eq!(s.small_exempt_limit, dec("200.00"));
        assert_eq!(s.route_for(dec("150.00")), PaidCancelRoute::Exempt);
        assert_eq!(s.route_for(dec("250.00")), PaidCancelRoute::Direct);
    }
}
