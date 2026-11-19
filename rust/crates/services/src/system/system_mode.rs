//! 系统模式配置（后台「设置 → 商城设置 → 系统模式」）。
//!
//! 来源标注：**我方新增** — giftmall 三端源码审计（~/workspace/giftmall-ref/REPORT.md
//! 第 4 节）确认原版无 systemMode / 礼品卡模式 / 商城模式配置键，原版礼品卡
//! 入口为硬编码常显。本设置按用户 2026-11-18 需求新增，设计见
//! `~/workspace/giftcard-welfare-design/system-mode-design.md`。
//!
//! 存储：`config` 表单行 `biz_code = "systemMode"`，`biz_val` 为
//! `{ "mode": "all" | "giftcard" | "mall" }`。缺失 / 非法值一律归一化为
//! `all`（全功能，默认），存量部署升级后行为零变化。
//!
//! 门控口径（用户拍板）：仅入口级门控 + 存量兜底——本配置只被前端用于
//! 菜单 / 导航 / 入口 / 区块的渲染显隐，后端不按模式拦截任何业务接口，
//! 已发出的礼品卡兑换 / 结算抵扣与已有订单在任何模式下照常可用。

use serde_json::{json, Value};

/// config 表中的业务编码。
pub const SYSTEM_MODE_CODE: &str = "systemMode";

/// 全功能（默认）：商城 + 礼品卡全开。
pub const MODE_ALL: &str = "all";
/// 礼品卡系统：以购卡 / 兑换 / 我的礼品卡 / 福利商品为一级入口。
pub const MODE_GIFT_CARD: &str = "giftcard";
/// 商城系统：礼品卡入口整体收起。
pub const MODE_MALL: &str = "mall";

/// 归一化模式值：仅接受 `all` / `giftcard` / `mall`（trim + 小写），
/// 其余（缺失 / 非法 / 非字符串）一律回落 `all`。
#[must_use]
pub fn normalize_mode(raw: Option<&str>) -> &'static str {
    match raw.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
        Some("giftcard") => MODE_GIFT_CARD,
        Some("mall") => MODE_MALL,
        _ => MODE_ALL,
    }
}

/// 内置默认配置（未保存过设置时生效）。
#[must_use]
pub fn default_system_mode() -> Value {
    json!({ "mode": MODE_ALL })
}

/// 校验/清洗一份系统模式配置（保存与读取共用）。
/// 输入不是对象、或 `mode` 非法时回落默认 `all`。
#[must_use]
pub fn sanitize_system_mode(input: &Value) -> Value {
    let mode = input
        .as_object()
        .and_then(|obj| obj.get("mode"))
        .and_then(Value::as_str);
    json!({ "mode": normalize_mode(mode) })
}

/// 由存储的原始 JSON 文本得到生效配置：解析失败/非对象 ⇒ 默认配置。
#[must_use]
pub fn effective_system_mode(stored: Option<&str>) -> Value {
    let Some(raw) = stored else {
        return default_system_mode();
    };
    match serde_json::from_str::<Value>(raw) {
        Ok(v) if v.is_object() => sanitize_system_mode(&v),
        _ => default_system_mode(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_all() {
        assert_eq!(default_system_mode()["mode"], json!("all"));
        assert_eq!(effective_system_mode(None), default_system_mode());
        assert_eq!(effective_system_mode(Some("not json")), default_system_mode());
        assert_eq!(effective_system_mode(Some("[1]")), default_system_mode());
    }

    #[test]
    fn normalize_accepts_only_three_modes() {
        assert_eq!(normalize_mode(Some(" giftcard ")), MODE_GIFT_CARD);
        assert_eq!(normalize_mode(Some("MALL")), MODE_MALL);
        assert_eq!(normalize_mode(Some("all")), MODE_ALL);
        assert_eq!(normalize_mode(Some("gift")), MODE_ALL);
        assert_eq!(normalize_mode(Some("")), MODE_ALL);
        assert_eq!(normalize_mode(None), MODE_ALL);
    }

    #[test]
    fn sanitize_drops_unknown_fields() {
        let v = sanitize_system_mode(&json!({ "mode": "mall", "evil": 1 }));
        assert_eq!(v, json!({ "mode": "mall" }));
        assert_eq!(sanitize_system_mode(&json!({ "mode": 3 })), default_system_mode());
    }
}
