//! Apple Pay 前台（payment::apple_pay）入参 / 纯函数。
//!
//! 对应原 Java `com.nextrade.api.controller.order.ApplePayController`
//! （`/api/order/pay` 下 4 个端点，均 `optional` 鉴权）：
//! `POST applepay/validateMerchant`、`POST applepay/confirm`、
//! `POST applepay/confirmByPaymentMethod`、`POST notify/ums`。
//!
//! Java `ApplePayService` 依赖银联/Stripe/Adyen 等 PSP 渠道 SDK（验签、商户校验、
//! 支付确认、异步通知验签），Rust 未迁移；本模块在完成**本地归属校验**后返回
//! 明确的「未迁移」业务错误。

use serde_json::Value;

/// 未登录。
pub const MSG_USER_NOT_LOGIN: &str = "用户未登录";
/// 参数缺失。
pub const MSG_PARAM_MISSING: &str = "参数缺失";
/// 找不到原支付记录。
pub const MSG_PAYLOG_NOT_FOUND: &str = "找不到原支付记录";
/// 订单归属校验失败。
pub const MSG_ORDER_NOT_FOUND: &str = "订单不存在";
/// 渠道 SDK 未迁移。
pub const MSG_CHANNEL_UNMIGRATED: &str =
    "Apple Pay 验签与支付处理依赖渠道 SDK（银联商务/Stripe/Adyen 等），Rust 侧尚未迁移，暂无法完成";

/// 从请求体提取 `paySn`（兼容 `paySn` / `pay_sn` / `outTradeNo`）。
#[must_use]
pub fn pay_sn_from_body(body: &Value) -> Option<String> {
    ["paySn", "pay_sn", "outTradeNo", "out_trade_no"]
        .iter()
        .find_map(|key| body.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// 请求体是否为空（`null` / `{}` / 空字符串）。
#[must_use]
pub fn body_is_empty(body: &Value) -> bool {
    match body {
        Value::Null => true,
        Value::String(s) => s.trim().is_empty(),
        Value::Object(map) => map.is_empty(),
        Value::Array(arr) => arr.is_empty(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pay_sn_extraction_supports_camel_and_snake() {
        assert_eq!(
            pay_sn_from_body(&serde_json::json!({"paySn": " PS123 "})).as_deref(),
            Some("PS123")
        );
        assert_eq!(
            pay_sn_from_body(&serde_json::json!({"pay_sn": "PS2"})).as_deref(),
            Some("PS2")
        );
        assert_eq!(
            pay_sn_from_body(&serde_json::json!({"outTradeNo": "PS3"})).as_deref(),
            Some("PS3")
        );
        assert!(pay_sn_from_body(&serde_json::json!({})).is_none());
        assert!(pay_sn_from_body(&serde_json::json!({"paySn": "  "})).is_none());
    }

    #[test]
    fn empty_body_detection() {
        assert!(body_is_empty(&Value::Null));
        assert!(body_is_empty(&serde_json::json!({})));
        assert!(body_is_empty(&serde_json::json!("")));
        assert!(!body_is_empty(&serde_json::json!({"paySn": "x"})));
    }

    #[test]
    fn messages_are_verbatim() {
        assert_eq!(MSG_USER_NOT_LOGIN, "用户未登录");
        assert_eq!(MSG_PAYLOG_NOT_FOUND, "找不到原支付记录");
        assert!(MSG_CHANNEL_UNMIGRATED.contains("未迁移"));
    }
}
