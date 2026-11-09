//! Apple Pay 前台服务（payment::apple_pay，4 个 `optional` 端点）。
//!
//! 对应原 Java `com.nextrade.api.controller.order.ApplePayController`。
//! 本地完成 paylog 归属校验后，走可配置收单适配器：
//! - `validateMerchant`：用商户证书向 Apple 申请 merchant session（双向 TLS）；
//! - `confirm` / `confirmByPaymentMethod`：把 token 提交到配置的收单地址。
//!
//! 真实联调依赖：Apple 商户证书/私钥、收单机构地址与密钥。

use std::sync::Arc;

use serde_json::Value;

use nextrade_core::models::payment::apple_pay::{
    body_is_empty, pay_sn_from_body, MSG_ORDER_NOT_FOUND, MSG_PARAM_MISSING, MSG_PAYLOG_NOT_FOUND,
    MSG_USER_NOT_LOGIN,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

pub struct ApplePayService {
    repos: Arc<Repositories>,
}

impl ApplePayService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    async fn config_value(&self, code: &str) -> Result<Option<String>> {
        let v: Option<String> = sqlx::query_scalar("SELECT biz_val FROM config WHERE biz_code=$1 LIMIT 1")
            .bind(code)
            .fetch_optional(&self.repos.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(v.filter(|s| !s.trim().is_empty()))
    }

    /// `POST /api/order/pay/applepay/validateMerchant`：商户校验。
    pub async fn validate_merchant(&self, user_id: Option<i32>, body: &Value) -> Result<Value> {
        let paylog = self.find_owned_paylog(user_id, body).await?;
        if paylog.pay_status == 1 {
            return Err(Error::biz(1001, "订单已支付"));
        }
        let validation_url = body
            .get("validationUrl")
            .or_else(|| body.get("validationURL"))
            .and_then(Value::as_str)
            .unwrap_or("");
        if !validation_url.contains("apple.com") {
            return Err(Error::biz(1001, "Apple Pay 商户校验地址非法"));
        }
        let cert = self
            .config_value("applePayMerchantCert")
            .await?
            .ok_or_else(|| Error::biz(1001, "Apple Pay 商户证书未配置"))?;
        let key = self
            .config_value("applePayMerchantKey")
            .await?
            .ok_or_else(|| Error::biz(1001, "Apple Pay 商户私钥未配置"))?;
        let identity = reqwest::Identity::from_pem(format!("{cert}\n{key}").as_bytes())
            .map_err(|e| Error::biz(1001, format!("Apple Pay 商户证书加载失败: {e}")))?;
        let client = reqwest::Client::builder()
            .identity(identity)
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let merchant_id = self
            .config_value("applePayMerchantId")
            .await?
            .unwrap_or_default();
        let display_name = self
            .config_value("applePayDisplayName")
            .await?
            .unwrap_or_else(|| "NexTrade".into());
        let domain = match body.get("displayDomain").and_then(Value::as_str) {
            Some(v) => v.to_string(),
            None => self.config_value("pcDomain").await?.unwrap_or_default(),
        };
        let req = serde_json::json!({
            "merchantIdentifier": merchant_id,
            "displayName": display_name,
            "initiative": "web",
            "initiativeContext": domain
        });
        let resp = client
            .post(validation_url)
            .header("Content-Type", "application/json")
            .body(req.to_string())
            .send()
            .await
            .map_err(|e| Error::biz(1001, format!("Apple Pay 商户校验网络异常: {e}")))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(Error::biz(1001, format!("Apple Pay 商户校验失败: {text}")));
        }
        serde_json::from_str(&text)
            .map_err(|e| Error::biz(1001, format!("Apple Pay 商户校验响应非法: {e}")))
    }

    /// `POST /api/order/pay/applepay/confirm`：支付确认。
    pub async fn confirm(&self, user_id: Option<i32>, body: &Value) -> Result<Value> {
        self.confirm_common(user_id, body, None).await
    }

    /// `POST /api/order/pay/applepay/confirmByPaymentMethod`：按支付方式确认。
    pub async fn confirm_by_payment_method(&self, user_id: Option<i32>, body: &Value) -> Result<Value> {
        let pm = body
            .get("paymentMethodId")
            .and_then(Value::as_str)
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| Error::biz_msg(MSG_PARAM_MISSING))?;
        self.confirm_common(user_id, body, Some(pm)).await
    }

    async fn confirm_common(
        &self,
        user_id: Option<i32>,
        body: &Value,
        payment_method_id: Option<&str>,
    ) -> Result<Value> {
        let paylog = self.find_owned_paylog(user_id, body).await?;
        if paylog.pay_status == 1 {
            return Ok(serde_json::json!({
                "status": "success",
                "transactionId": paylog.transaction_id,
                "message": "订单已支付"
            }));
        }
        let endpoint = match self.config_value("applePayAcquirerUrl").await? {
            Some(v) => v,
            None => self
                .config_value("applePayGateway")
                .await?
                .ok_or_else(|| Error::biz(1001, "Apple Pay 收单地址未配置：applePayAcquirerUrl"))?,
        };
        let key = self.config_value("applePayAcquirerKey").await?.unwrap_or_default();
        let currency = self
            .config_value("applePayCurrency")
            .await?
            .unwrap_or_else(|| "CNY".into());
        let payment_token = body
            .get("paymentToken")
            .cloned()
            .or_else(|| body.get("token").cloned())
            .unwrap_or(Value::Null);
        let req = serde_json::json!({
            "paySn": paylog.pay_sn,
            "amount": paylog.pay_amount.to_string(),
            "currency": currency,
            "paymentToken": payment_token,
            "paymentMethodId": payment_method_id
        });
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(25))
            .build()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let mut builder = client.post(&endpoint).header("Content-Type", "application/json");
        if !key.trim().is_empty() {
            builder = builder.header("Authorization", format!("Bearer {}", key.trim()));
        }
        let resp = builder
            .body(req.to_string())
            .send()
            .await
            .map_err(|e| Error::biz(1001, format!("Apple Pay 收单网络异常: {e}")))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        if !status.is_success() {
            return Err(Error::biz(1001, format!("Apple Pay 收单失败: {text}")));
        }
        let tx = v
            .get("transactionId")
            .or_else(|| v.get("id"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let ok = v
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|s| matches!(s.to_ascii_lowercase().as_str(), "success" | "succeeded" | "completed" | "captured"));
        if !ok || tx.is_empty() {
            return Ok(serde_json::json!({
                "status": "processing",
                "message": v.get("message").and_then(Value::as_str).unwrap_or("收单处理中")
            }));
        }
        let tx = tx.to_string();
        self.repos
            .payment
            .paylog
            .mark_paid_by_pay_sn(&paylog.pay_sn, &tx, &text)
            .await?;
        if let Err(e) = crate::payment::paylog::PaylogService::new(self.repos.clone()).settle_order_by_pay_sn(&paylog.pay_sn, &tx).await {
            tracing::warn!("Apple Pay 推进订单失败: pay_sn={}, err={e}", paylog.pay_sn);
        }
        Ok(serde_json::json!({"status": "success", "transactionId": tx}))
    }

    /// `POST /api/order/pay/notify/ums`：银联商务异步通知。
    pub async fn notify_ums(&self, body: &Value) -> Result<()> {
        if body_is_empty(body) {
            return Err(Error::biz_msg(MSG_PARAM_MISSING));
        }
        let pay_sn = body
            .get("paySn")
            .or_else(|| body.get("outTradeNo"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let tx = body
            .get("transactionId")
            .or_else(|| body.get("tradeNo"))
            .and_then(Value::as_str)
            .unwrap_or("");
        if pay_sn.is_empty() || tx.is_empty() {
            return Err(Error::biz_msg(MSG_PARAM_MISSING));
        }
        self.repos
            .payment
            .paylog
            .mark_paid_by_pay_sn(pay_sn, tx, &body.to_string())
            .await?;
        if let Err(e) = crate::payment::paylog::PaylogService::new(self.repos.clone()).settle_order_by_pay_sn(pay_sn, tx).await {
            tracing::warn!("UMS 通知推进订单失败: pay_sn={pay_sn}, err={e}");
        }
        Ok(())
    }

    async fn find_owned_paylog(
        &self,
        user_id: Option<i32>,
        body: &Value,
    ) -> Result<nextrade_core::entities::Paylog> {
        let user_id = user_id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::biz_msg(MSG_USER_NOT_LOGIN))?;
        let pay_sn =
            pay_sn_from_body(body).ok_or_else(|| Error::biz_msg(MSG_PARAM_MISSING))?;
        let paylog = self
            .repos
            .payment
            .apple_pay
            .find_paylog_by_pay_sn(&pay_sn)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_PAYLOG_NOT_FOUND))?;
        let owner = if paylog.order_type == 1 {
            self.repos
                .payment
                .apple_pay
                .recharge_owner(paylog.order_id)
                .await?
        } else {
            self.repos
                .payment
                .apple_pay
                .order_owner(paylog.order_id)
                .await?
        };
        match owner {
            Some(id) if id == user_id => Ok(paylog),
            _ => Err(Error::biz_msg(MSG_ORDER_NOT_FOUND)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_messages_are_stable() {
        assert!(MSG_PARAM_MISSING.contains("参数"));
        assert!(MSG_PAYLOG_NOT_FOUND.contains("支付"));
    }
}
