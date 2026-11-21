//! 美团闪购适配器（takeout::meituan，我方新增）：签名、推送验签与
//! 解析、出站 API 客户端（确认接单 / 自配送物流同步 / 退款应答）。
//!
//! # 依据（官方公开文档，2026-11-21 核）
//! - 签名规则：美团技术服务合作中心《签名规则》（tscc.meituan.com
//!   开发者文档，与闪购同属美团开放平台签名体系）：全部参数（sig
//!   除外）按英文字母逐个字符排序、`&` 连接，拼接
//!   `请求url + "?" + 排序串 + app_secret` 后 MD5（32 位小写）。
//!   本文件 [`sign_request`] 附官方文档完整示例向量单测。
//! - 推送验签：同文档注意事项 6 —— APP 方接收 **POST** 推送时参数
//!   需 **解码 2 次** 后再验签，空值参数也参与签名拼接；GET 推送
//!   解码 1 次。推送验签串按同规则以「排序参数串 + app_secret」
//!   计算（推送无固定请求 url 参与段）。**联调期须以开发者中心
//!   签名工具对真实推送复核一次**（凭据到位后第一项，见 IMPL-REPORT）。
//! - 回调响应：收到推送须快速响应 `{"data":"ok"}`，失败平台每分钟
//!   重试共 3 次（幂等由接入层以渠道订单号兜底）。
//! - 订单状态：1 已提交 / 2 已推送商家 / 4 已确认 / 8 已完成 /
//!   9 已取消（设计 §1.2）；新单须 5 分钟内处理（绑定 auto_accept）。
//! - 金额单位：推送与接口金额均为**分**，归一时换算为元。
//!
//! 网关与接口路径为可配常量：闪购网关 `open-shangou.meituan.com`，
//! 路径按美团开放平台公开接口命名；应用审核通过后以开发者中心
//! 该应用的接口列表为准复核（配置位 `gatewayBase` 可覆盖网关）。

use async_trait::async_trait;
use md5::{Digest, Md5};
use rust_decimal::Decimal;
use serde_json::{json, Value};

use nextrade_common::utils::now_seconds;
use nextrade_core::models::takeout::{
    fen_to_yuan, form_get, meituan_status_normalize, parse_form_pairs, ChannelConfig,
    ExternalOrder, ExternalOrderItem, EXT_STATUS_CANCELLED, EXT_STATUS_PAID, EXT_STATUS_REFUNDED,
    EXT_STATUS_REFUNDING,
};
use nextrade_core::{Error, Result};

use super::channel::{
    OrderChannelAdapter, PushEvent, EVENT_CANCEL, EVENT_ORDER, EVENT_REFUND, EVENT_STATUS,
    EVENT_UNKNOWN,
};

/// 闪购默认网关（配置 `gatewayBase` 可覆盖，如沙箱环境）。
pub const DEFAULT_GATEWAY: &str = "https://open-shangou.meituan.com";
/// 确认接单接口路径。
pub const PATH_ORDER_CONFIRM: &str = "/api/v1/order/confirm";
/// 自配送订单物流状态同步接口路径。
pub const PATH_LOGISTICS_SYNC: &str = "/api/v1/ecommerce/order/logistics/sync";
/// 退款同意接口路径。
pub const PATH_REFUND_AGREE: &str = "/api/v1/order/refund/agree";
/// 退款驳回接口路径。
pub const PATH_REFUND_REJECT: &str = "/api/v1/order/refund/reject";

/// 美团开放平台签名（官方规则）：参数（sig 除外）按 key 逐字符
/// 排序、`k=v` 以 `&` 连接，`url + "?" + 串 + app_secret` 做 MD5。
#[must_use]
pub fn sign_request(url: &str, params: &[(String, String)], app_secret: &str) -> String {
    let mut sorted: Vec<&(String, String)> = params.iter().filter(|(k, _)| k != "sig").collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let joined = sorted
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");
    let raw = format!("{url}?{joined}{app_secret}");
    let digest = Md5::digest(raw.as_bytes());
    format!("{digest:x}")
}

/// 推送验签签名：排序参数串 + app_secret 后 MD5（参数已按 POST
/// 解码 2 次 / GET 解码 1 次处理，空值参数保留参与拼接）。
#[must_use]
pub fn sign_push(params: &[(String, String)], app_secret: &str) -> String {
    let mut sorted: Vec<&(String, String)> = params.iter().filter(|(k, _)| k != "sig").collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let joined = sorted
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");
    let raw = format!("{joined}{app_secret}");
    let digest = Md5::digest(raw.as_bytes());
    format!("{digest:x}")
}

/// 美团闪购适配器（无状态，凭据每次从配置传入）。
#[derive(Debug, Default)]
pub struct MeituanAdapter;

impl MeituanAdapter {
    /// 组装出站客户端（网关取配置覆盖或默认）。
    fn client(&self, cfg: &ChannelConfig) -> MeituanClient {
        MeituanClient::new(cfg)
    }
}

/// 美团出站 API 客户端。
pub struct MeituanClient {
    http: reqwest::Client,
    gateway_base: String,
    app_id: String,
    app_secret: String,
}

impl MeituanClient {
    /// 由渠道配置构造（凭据不齐时调用方应已拦截，本构造不校验）。
    #[must_use]
    pub fn new(cfg: &ChannelConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self {
            http,
            gateway_base: if cfg.gateway_base.trim().is_empty() {
                DEFAULT_GATEWAY.to_string()
            } else {
                cfg.gateway_base.trim().trim_end_matches('/').to_string()
            },
            app_id: cfg.app_id.clone(),
            app_secret: cfg.app_secret.clone(),
        }
    }

    /// 签名并 POST 一个接口（表单编码）。成功判据：HTTP 200 且响应
    /// JSON 中 `data == "ok"` / `result == true` / `code == 0` 任一成立；
    /// 否则 Err 带渠道返回原文摘要 —— 不把失败包装成成功。
    async fn call(
        &self,
        path: &str,
        mut params: Vec<(String, String)>,
        access_token: &str,
    ) -> Result<Value> {
        let url = format!("{}{path}", self.gateway_base);
        params.push(("app_id".to_string(), self.app_id.clone()));
        params.push(("timestamp".to_string(), now_seconds().to_string()));
        if !access_token.is_empty() {
            // ISV 应用：被操作门店的 access_token 必须参与签名（官方口径）。
            params.push(("access_token".to_string(), access_token.to_string()));
        }
        let sig = sign_request(&url, &params, &self.app_secret);
        params.push(("sig".to_string(), sig));
        let resp = self
            .http
            .post(&url)
            .form(&params)
            .send()
            .await
            .map_err(|e| Error::biz_msg(format!("美团接口网络错误：{e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| Error::biz_msg(format!("美团接口响应读取失败：{e}")))?;
        if !status.is_success() {
            return Err(Error::biz_msg(format!(
                "美团接口 HTTP {status}：{}",
                text.chars().take(200).collect::<String>()
            )));
        }
        let value: Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({ "raw": text }));
        let ok = value.get("data").and_then(Value::as_str) == Some("ok")
            || value.get("result").and_then(Value::as_bool) == Some(true)
            || value.get("code").and_then(Value::as_i64) == Some(0);
        if ok {
            Ok(value)
        } else {
            Err(Error::biz_msg(format!(
                "美团接口返回失败：{}",
                text.chars().take(200).collect::<String>()
            )))
        }
    }

    /// 确认接单。
    pub async fn confirm_order(
        &self,
        app_poi_code: &str,
        access_token: &str,
        external_order_id: &str,
    ) -> Result<()> {
        self.call(
            PATH_ORDER_CONFIRM,
            vec![
                ("app_poi_code".to_string(), app_poi_code.to_string()),
                ("order_id".to_string(), external_order_id.to_string()),
            ],
            access_token,
        )
        .await?;
        Ok(())
    }

    /// 自配送物流状态同步。
    pub async fn sync_logistics(
        &self,
        app_poi_code: &str,
        access_token: &str,
        external_order_id: &str,
        logistics_status: i32,
    ) -> Result<()> {
        self.call(
            PATH_LOGISTICS_SYNC,
            vec![
                ("app_poi_code".to_string(), app_poi_code.to_string()),
                ("order_id".to_string(), external_order_id.to_string()),
                ("logistics_status".to_string(), logistics_status.to_string()),
            ],
            access_token,
        )
        .await?;
        Ok(())
    }

    /// 退款应答（同意走退款同意接口，驳回须带原因）。
    pub async fn respond_refund(
        &self,
        app_poi_code: &str,
        access_token: &str,
        external_order_id: &str,
        refund_id: &str,
        agree: bool,
        reason: &str,
    ) -> Result<()> {
        let path = if agree {
            PATH_REFUND_AGREE
        } else {
            PATH_REFUND_REJECT
        };
        let mut params = vec![
            ("app_poi_code".to_string(), app_poi_code.to_string()),
            ("order_id".to_string(), external_order_id.to_string()),
        ];
        if !refund_id.is_empty() {
            params.push(("refund_id".to_string(), refund_id.to_string()));
        }
        if !agree {
            params.push(("reason".to_string(), reason.to_string()));
        }
        self.call(path, params, access_token).await?;
        Ok(())
    }
}

/// 解析推送中的商品明细（`detail` 为 JSON 数组字符串；字段名在
/// 外卖/闪购两线间有差异，按多候选防御解析，金额为分）。
fn parse_items(detail: Option<&str>) -> Vec<ExternalOrderItem> {
    let Some(raw) = detail else {
        return Vec::new();
    };
    let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    let pick_str = |v: &Value, keys: &[&str]| -> String {
        for k in keys {
            if let Some(s) = v.get(*k).and_then(Value::as_str) {
                if !s.is_empty() {
                    return s.to_string();
                }
            }
            if let Some(n) = v.get(*k).and_then(Value::as_i64) {
                return n.to_string();
            }
        }
        String::new()
    };
    let pick_i64 = |v: &Value, keys: &[&str]| -> i64 {
        for k in keys {
            if let Some(n) = v.get(*k).and_then(Value::as_i64) {
                return n;
            }
            if let Some(s) = v.get(*k).and_then(Value::as_str) {
                if let Ok(n) = s.trim().parse::<i64>() {
                    return n;
                }
            }
        }
        0
    };
    arr.iter()
        .map(|v| ExternalOrderItem {
            external_spu_id: pick_str(v, &["app_spu_code", "spu_id", "app_food_code"]),
            external_sku_id: pick_str(v, &["app_food_code", "sku_id", "app_sku_code"]),
            name: pick_str(v, &["food_name", "name", "product_name"]),
            quantity: i32::try_from(pick_i64(v, &["quantity", "count", "num"]))
                .unwrap_or(1)
                .max(1),
            price: fen_to_yuan(pick_i64(v, &["price", "food_price"])),
            product_id: 0,
            sku_id: 0,
        })
        .collect()
}

fn form_i64(pairs: &[(String, String)], key: &str) -> i64 {
    form_get(pairs, key)
        .and_then(|s| s.trim().parse::<i64>().ok())
        .unwrap_or(0)
}

#[async_trait]
impl OrderChannelAdapter for MeituanAdapter {
    fn channel(&self) -> &'static str {
        "meituan"
    }

    fn verify_and_parse(
        &self,
        cfg: &ChannelConfig,
        event_hint: &str,
        raw_body: &str,
    ) -> Result<PushEvent> {
        if !cfg.configured() {
            return Err(Error::biz_msg("美团渠道未配置凭据"));
        }
        // POST 推送：参数解码 2 次后验签（官方注意事项 6）。
        let pairs = parse_form_pairs(raw_body, 2);
        let Some(sig) = form_get(&pairs, "sig") else {
            return Err(Error::biz_msg("推送缺少 sig 参数"));
        };
        let expect = sign_push(&pairs, &cfg.app_secret);
        if !sig.eq_ignore_ascii_case(&expect) {
            return Err(Error::biz_msg("推送验签失败"));
        }
        let external_order_id = form_get(&pairs, "order_id")
            .or_else(|| form_get(&pairs, "orderId"))
            .unwrap_or("")
            .to_string();
        if external_order_id.is_empty() {
            return Err(Error::biz_msg("推送缺少 order_id"));
        }
        let raw_status = i32::try_from(form_i64(&pairs, "status")).unwrap_or(0);
        let mut event = PushEvent {
            event_type: EVENT_UNKNOWN.to_string(),
            external_order_id: external_order_id.clone(),
            order: None,
            refund_id: String::new(),
            note: String::new(),
        };
        match event_hint {
            "cancel" => {
                event.event_type = EVENT_CANCEL.to_string();
                event.note = form_get(&pairs, "reason")
                    .or_else(|| form_get(&pairs, "cancel_reason"))
                    .unwrap_or("")
                    .to_string();
                let mut order = parse_order_base(&pairs, raw_status);
                order.status = EXT_STATUS_CANCELLED.to_string();
                event.order = Some(order);
            }
            "refund" => {
                event.event_type = EVENT_REFUND.to_string();
                event.refund_id = form_get(&pairs, "refund_id")
                    .or_else(|| form_get(&pairs, "refund_order_id"))
                    .unwrap_or("")
                    .to_string();
                event.note = form_get(&pairs, "reason")
                    .or_else(|| form_get(&pairs, "refund_reason"))
                    .unwrap_or("")
                    .to_string();
                let mut order = parse_order_base(&pairs, raw_status);
                // 退款完成信号字段在两线文档间不统一：带明确完成标记才落
                // refunded，其余一律 refunding（保守，不冒认退款已执行）。
                let done = form_get(&pairs, "refund_status")
                    .is_some_and(|s| s == "refunded" || s == "success")
                    || form_get(&pairs, "refund_valid").is_some_and(|s| s == "1");
                order.status = if done {
                    EXT_STATUS_REFUNDED.to_string()
                } else {
                    EXT_STATUS_REFUNDING.to_string()
                };
                event.order = Some(order);
            }
            "status" => {
                event.event_type = EVENT_STATUS.to_string();
                let mut order = parse_order_base(&pairs, raw_status);
                order.status = meituan_status_normalize(raw_status)
                    .unwrap_or(EXT_STATUS_PAID)
                    .to_string();
                event.order = Some(order);
            }
            _ => {
                // 已支付订单推送（默认入口）：完整订单体。
                event.event_type = EVENT_ORDER.to_string();
                let mut order = parse_order_base(&pairs, raw_status);
                order.items = parse_items(form_get(&pairs, "detail"));
                order.status = meituan_status_normalize(raw_status)
                    .unwrap_or(EXT_STATUS_PAID)
                    .to_string();
                if order.product_amount.is_zero() && !order.items.is_empty() {
                    order.product_amount = order
                        .items
                        .iter()
                        .map(|i| i.price * Decimal::from(i.quantity))
                        .sum();
                }
                event.order = Some(order);
            }
        }
        Ok(event)
    }

    async fn confirm_order(
        &self,
        cfg: &ChannelConfig,
        app_poi_code: &str,
        access_token: &str,
        external_order_id: &str,
    ) -> Result<()> {
        if !cfg.active() {
            return Err(Error::biz_msg("美团渠道未启用或未配置凭据"));
        }
        self.client(cfg)
            .confirm_order(app_poi_code, access_token, external_order_id)
            .await
    }

    async fn sync_logistics(
        &self,
        cfg: &ChannelConfig,
        app_poi_code: &str,
        access_token: &str,
        external_order_id: &str,
        logistics_status: i32,
    ) -> Result<()> {
        if !cfg.active() {
            return Err(Error::biz_msg("美团渠道未启用或未配置凭据"));
        }
        self.client(cfg)
            .sync_logistics(app_poi_code, access_token, external_order_id, logistics_status)
            .await
    }

    async fn respond_refund(
        &self,
        cfg: &ChannelConfig,
        app_poi_code: &str,
        access_token: &str,
        external_order_id: &str,
        refund_id: &str,
        agree: bool,
        reason: &str,
    ) -> Result<()> {
        if !cfg.active() {
            return Err(Error::biz_msg("美团渠道未启用或未配置凭据"));
        }
        self.client(cfg)
            .respond_refund(app_poi_code, access_token, external_order_id, refund_id, agree, reason)
            .await
    }
}

/// 推送的订单基础字段（不含商品明细）。
fn parse_order_base(pairs: &[(String, String)], raw_status: i32) -> ExternalOrder {
    ExternalOrder {
        external_order_id: form_get(pairs, "order_id")
            .or_else(|| form_get(pairs, "orderId"))
            .unwrap_or("")
            .to_string(),
        app_poi_code: form_get(pairs, "app_poi_code").unwrap_or("").to_string(),
        external_status: raw_status,
        status: String::new(),
        items: Vec::new(),
        total_amount: fen_to_yuan(form_i64(pairs, "total")),
        product_amount: fen_to_yuan(form_i64(pairs, "original_price")),
        shipping_fee: fen_to_yuan(form_i64(pairs, "shipping_fee")),
        consignee: form_get(pairs, "recipient_name").unwrap_or("").to_string(),
        mobile: form_get(pairs, "recipient_phone").unwrap_or("").to_string(),
        address: form_get(pairs, "recipient_address")
            .unwrap_or("")
            .to_string(),
        remark: form_get(pairs, "caution")
            .or_else(|| form_get(pairs, "remark"))
            .unwrap_or("")
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 官方文档完整示例向量（技术服务合作中心《签名规则》）：
    /// url = https://waimaiopen.meituan.com/api/v1/order/getOrderDaySeq
    /// app_id=0000, app_poi_code=31号测试店, timestamp=1389751221,
    /// secret = a00ba58f000001aa697ab000006d52d
    /// → sig = dbb4d444d68596d03dbcb79a4e4a4e3f
    #[test]
    fn sign_matches_official_doc_vector() {
        let params = vec![
            ("app_id".to_string(), "0000".to_string()),
            ("app_poi_code".to_string(), "31号测试店".to_string()),
            ("timestamp".to_string(), "1389751221".to_string()),
        ];
        let sig = sign_request(
            "https://waimaiopen.meituan.com/api/v1/order/getOrderDaySeq",
            &params,
            "a00ba58f000001aa697ab000006d52d",
        );
        assert_eq!(sig, "dbb4d444d68596d03dbcb79a4e4a4e3f");
    }

    #[test]
    fn push_verify_rejects_bad_sign_and_accepts_good() {
        let cfg = ChannelConfig {
            enabled: true,
            app_id: "100".into(),
            app_secret: "sekret".into(),
            gateway_base: String::new(),
        };
        let adapter = MeituanAdapter;
        // 手工组一份带正确签名的推送（值双重编码模拟平台 POST 口径）。
        let pairs = vec![
            ("app_id".to_string(), "100".to_string()),
            ("app_poi_code".to_string(), "POI1".to_string()),
            ("order_id".to_string(), "M123".to_string()),
            ("status".to_string(), "2".to_string()),
            ("total".to_string(), "25900".to_string()),
            ("shipping_fee".to_string(), "300".to_string()),
            (
                "detail".to_string(),
                "[{\"app_food_code\":\"F1\",\"food_name\":\"电池\",\"quantity\":2,\"price\":12800}]"
                    .to_string(),
            ),
        ];
        let sig = sign_push(&pairs, "sekret");
        let body = pairs
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .chain(std::iter::once(format!("sig={sig}")))
            .collect::<Vec<_>>()
            .join("&");
        let event = adapter
            .verify_and_parse(&cfg, "order", &body)
            .expect("valid sign should parse");
        assert_eq!(event.external_order_id, "M123");
        let order = event.order.expect("order body");
        assert_eq!(order.status, EXT_STATUS_PAID);
        assert_eq!(order.total_amount, fen_to_yuan(25900));
        assert_eq!(order.items.len(), 1);
        assert_eq!(order.items[0].external_sku_id, "F1");
        assert_eq!(order.items[0].quantity, 2);

        let bad = format!("{body}ff");
        assert!(adapter.verify_and_parse(&cfg, "order", &bad).is_err());
    }
}
