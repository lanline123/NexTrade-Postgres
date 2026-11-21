//! 会员认证门控配置下发（第 0 期：认证启用）。
//!
//! 后台「会员认证设置」（`GET/POST /adminapi/setting/config/authSettings|saveAuth`）
//! 的三个开关原本只存不用——公众端没有任何代码读它们。本模块把它们以
//! 只读形式下发给前台，供 PC/H5 决定：是否展示认证入口、价格是否需要
//! 「登录 + 认证后可见」（门控执行在价格接口侧，见第 2 期）。
//!
//! 存储映射（与 `nextrade_core::models::system::config_basic` 的 EP44/EP45
//! 字段规格逐一对应）：`config` 表按字段各存一行，`biz_code` 即字段名
//! 本身、`biz_val` 为整数文本——
//! - `closeAuth`：认证功能开关。注意原版命名反直觉：后台页
//!   `authClosed = (closeAuth === 0)`，即 **1 = 认证开启、0 = 关闭**
//!   （规格表默认 `"0"`）。
//! - `isIdentity`：是否实名（认证通过）后才能查看价格，1 = 开启。
//! - `isLogin`：是否登录后才能查看价格，1 = 开启（我方新增，
//!   2026-11-21 用户拍板把价格门控由单开关拆为两个独立开关：
//!   `isLogin` 只管查看、不管购买；购买拦截只跟随 `isIdentity`，
//!   见 `system::price_gate`）。
//! - `type`：可申请的认证类型，1 = 仅企业认证、0 = 个人 + 企业。
//!
//! 读取语义与管理端 `parse_field`（Int）对齐：缺失/空白 ⇒ 0，
//! 非数字 ⇒ 0（管理端为 Null，门控下发取 0 更安全，不会误开/误关）。

use serde_json::{json, Value};

/// config 表中的业务编码（与认证设置规格表 EP44/EP45 的字段 code 相同）。
pub const CLOSE_AUTH_CODE: &str = "closeAuth";
/// config 表中的业务编码：是否实名后才能查看价格。
pub const IS_IDENTITY_CODE: &str = "isIdentity";
/// config 表中的业务编码：是否登录后才能查看价格（我方新增）。
pub const IS_LOGIN_CODE: &str = "isLogin";
/// config 表中的业务编码：认证类型（1 仅企业 / 0 个人+企业）。
pub const AUTH_TYPE_CODE: &str = "type";

/// 本端点读取的全部 biz_code（一次 IN 查询读齐）。
pub const AUTH_GATE_CODES: [&str; 4] = [
    CLOSE_AUTH_CODE,
    IS_IDENTITY_CODE,
    IS_LOGIN_CODE,
    AUTH_TYPE_CODE,
];

fn parse_flag(rows: &[(String, String)], code: &str) -> i64 {
    rows.iter()
        .find(|(k, _)| k == code)
        .and_then(|(_, v)| v.trim().parse::<i64>().ok())
        .unwrap_or(0)
}

/// 由 config 原始行组装下发 JSON（字段名与后台设置页表单一致）。
#[must_use]
pub fn effective_auth_gate(rows: &[(String, String)]) -> Value {
    json!({
        "closeAuth": parse_flag(rows, CLOSE_AUTH_CODE),
        "isIdentity": parse_flag(rows, IS_IDENTITY_CODE),
        "isLogin": parse_flag(rows, IS_LOGIN_CODE),
        "type": parse_flag(rows, AUTH_TYPE_CODE),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn defaults_all_zero_when_unconfigured() {
        let v = effective_auth_gate(&[]);
        assert_eq!(v["closeAuth"], json!(0));
        assert_eq!(v["isIdentity"], json!(0));
        assert_eq!(v["isLogin"], json!(0));
        assert_eq!(v["type"], json!(0));
    }

    #[test]
    fn reads_stored_integer_strings() {
        let v = effective_auth_gate(&rows(&[
            ("closeAuth", "1"),
            ("isIdentity", "1"),
            ("isLogin", "1"),
            ("type", "1"),
            ("smsNote", "1"),
        ]));
        assert_eq!(v["closeAuth"], json!(1));
        assert_eq!(v["isIdentity"], json!(1));
        assert_eq!(v["isLogin"], json!(1));
        assert_eq!(v["type"], json!(1));
        // 只下发四项，其余认证设置字段（smsNote/tips）不外泄
        assert!(v.get("smsNote").is_none());
        assert!(v.get("tips").is_none());
    }

    #[test]
    fn blank_or_garbage_falls_back_to_zero() {
        let v = effective_auth_gate(&rows(&[("closeAuth", " "), ("isIdentity", "yes")]));
        assert_eq!(v["closeAuth"], json!(0));
        assert_eq!(v["isIdentity"], json!(0));
    }
}
