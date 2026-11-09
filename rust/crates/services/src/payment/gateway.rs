//! 支付服务抽象。
//!
//! 对应原 Java `nextrade-service` 中的支付渠道实现
//! （微信 weixin-java-pay、支付宝 alipay-sdk、PayPal IJPay、Stripe、
//! 拉卡拉、易宝 YOP、余额支付等）。
//!
//! Rust 侧统一为 [`PayChannel`] trait，按配置注册实现。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use nextrade_core::{Error, Result};

/// 支付渠道标识（对应原 pay_type_id）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    /// 余额支付
    Balance,
    /// 微信支付
    Wechat,
    /// 支付宝
    Alipay,
    /// PayPal
    Paypal,
    /// Stripe
    Stripe,
    /// 线下转账
    Offline,
}

/// 统一下单请求。
#[derive(Debug, Clone)]
pub struct PayRequest {
    pub order_id: i32,
    pub out_trade_no: String,
    pub amount: Decimal,
    /// 商品标题
    pub subject: String,
    /// 回调地址
    pub notify_url: String,
    /// 客户端IP
    pub client_ip: String,
}

/// 统一下单响应。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayResponse {
    /// 渠道交易号
    pub trade_no: String,
    /// 支付参数（JSAPI 参数 / 表单 / 跳转 URL，按渠道不同）
    pub pay_params: serde_json::Value,
    /// 支付方式：qrcode-二维码 redirect-跳转 form-表单 jsapi
    pub pay_type: String,
}

/// 支付回调载荷（渠道回调解析后的统一结构）。
#[derive(Debug, Clone)]
pub struct PayNotify {
    pub out_trade_no: String,
    pub trade_no: String,
    pub paid_amount: Decimal,
    /// 渠道原始报文（验签留痕）
    pub raw: String,
}

/// 支付渠道 trait（异步对象安全）。
pub trait PayChannel: Send + Sync {
    /// 渠道标识。
    fn channel(&self) -> Channel;

    /// 统一下单。
    fn prepay(&self, req: PayRequest)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<PayResponse>> + Send + '_>>;

    /// 回调验签 + 解析。
    fn verify_notify<'a>(
        &'a self,
        body: &'a str,
        headers: &'a [(String, String)],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<PayNotify>> + Send + 'a>>;

    /// 退款。
    fn refund(
        &self,
        out_trade_no: &str,
        refund_amount: Decimal,
        reason: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send + '_>>;
}

/// 支付网关：渠道注册与分发，替代原 Java `PaymentService` 的 if/else 分发。
pub struct PaymentGateway {
    channels: Vec<Box<dyn PayChannel>>,
}

impl PaymentGateway {
    pub fn new() -> Self {
        Self { channels: Vec::new() }
    }

    /// 注册渠道。
    pub fn register(&mut self, channel: Box<dyn PayChannel>) {
        self.channels.push(channel);
    }

    /// 获取渠道。
    pub fn channel(&self, c: Channel) -> Result<&dyn PayChannel> {
        self.channels
            .iter()
            .find(|ch| ch.channel() == c)
            .map(|ch| ch.as_ref())
            .ok_or_else(|| Error::ThirdParty {
                service: format!("{c:?}"),
                reason: "支付渠道未启用".into(),
            })
    }

    /// 余额支付（无需外部渠道，直接调用订单服务入账）。
    pub fn supports_balance() -> bool {
        true
    }
}

impl Default for PaymentGateway {
    fn default() -> Self {
        Self::new()
    }
}

/// 零金额校验（所有渠道通用）。
pub fn validate_amount(amount: Decimal) -> Result<()> {
    if amount <= Decimal::ZERO {
        return Err(Error::Validation("支付金额必须大于0".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn amount_validation() {
        assert!(validate_amount(Decimal::ZERO).is_err());
        assert!(validate_amount(Decimal::from_str("0.01").unwrap()).is_ok());
    }

    #[test]
    fn gateway_missing_channel() {
        let gw = PaymentGateway::new();
        assert!(gw.channel(Channel::Wechat).is_err());
    }
}