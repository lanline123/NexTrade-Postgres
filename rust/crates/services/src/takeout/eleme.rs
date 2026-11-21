//! 淘宝闪购（饿了么）适配器（takeout::eleme，我方新增）：出站 NOP
//! 签名与客户端、推送验签与解析、轻推送回查补全。
//!
//! # 依据（官方公开文档 / 官方 SDK，2026-11-21 核）
//! - 出站协议（NOP）：POST `{gateway}`（默认
//!   `https://open-api.shop.ele.me/api/v1/`），JSON 信封
//!   `{id, action, token, metas{app_key, timestamp}, params, signature}`；
//!   签名规则（开放平台《签名规则》+ 官方 Go SDK `ApiBody.Sign` 同构）：
//!   metas 与 params 全部属性以 `key=json_encode(value)` 序列化
//!   （类型敏感：数字 123 ≠ 字符串 "123"）、字典序排序、无分隔拼接，
//!   待签串 = `action + token + 拼接串 + app_secret`，MD5 大写。
//!   官方未公开完整示例向量 —— [`sign_request`] 单测向量为按上述
//!   规则以独立工具（python3 hashlib）计算的自洽向量，联调期须以
//!   沙箱实调复核（复核点 E-2：非 ASCII 参数值的 JSON 转义差异
//!   可能影响签名，如驳回退款 reason 含中文）。
//! - 推送：JSON 信封 `{appId, requestId, type, message, shopId,
//!   timestamp, userId, signature}`，成功响应 `{"message":"ok"}`。
//!   验签按官方 Go SDK demo（server.go）算法：七字段 `key=value`
//!   字典序拼接后 MD5 大写比对。**该算法按官方 demo 不含
//!   app_secret**（仅完整性校验、无密钥防伪）——与出站签名口径
//!   不一致，列为联调复核点 E-1：凭据到位后以真实推送复核，若
//!   平台实际算法含 secret 只需改 [`push_signature`] 一处。
//! - 推送为**轻通知**：message 只保证带 orderId / shopId，完整订单
//!   经 `eleme.order.getOrder` 回查（官方接入通例）；message 内嵌
//!   完整订单时（官方 demo 口径）亦可直接解析。本适配器经
//!   `enrich_event` 钩子在落库前回查补全（绑定令牌为空 = 未绑定，
//!   跳过补全走待绑定兜底）。
//! - 金额：饿了么 API 金额为**元（浮点）**，与美团的分不同，归一
//!   时经 `yuan_amount` 换算（两位小数格式化后解析，无浮点尾差）。
//! - 订单状态为字符串枚举（官方 SDK 订单实体 OOrder.status /
//!   refundStatus），归一见 `eleme_status_normalize`。
//! - 推送 type：10 = 订单生成；取消族 {14, 15, 17, 23, 25, 35}
//!   （官方消息口径的生产接入代码归类）。**不臆造完整 type 表**：
//!   其余 type 的状态一律以回查订单的 status / refundStatus 字符串
//!   为准归一（退款进程即由此识别，不依赖未证实的 type 码）。
//! - 出站动作（官方 SDK 动作名）：确认接单 `confirmOrderLite`、
//!   退款同意/驳回 `agreeRefundLite` / `disagreeRefundLite`、开始
//!   自配送 `deliveryBySelfLite`、确认送达 `receivedOrderLite`。

use async_trait::async_trait;
use md5::{Digest, Md5};
use rust_decimal::Decimal;
use serde_json::{json, Value};

use nextrade_common::utils::now_seconds;
use nextrade_core::models::takeout::{
    eleme_status_normalize, yuan_amount, ChannelConfig, ExternalOrder, ExternalOrderItem,
    EXT_STATUS_CANCELLED, EXT_STATUS_PAID, EXT_STATUS_REFUNDED, EXT_STATUS_REFUNDING,
};
use nextrade_core::{Error, Result};

use super::channel::{
    OrderChannelAdapter, PushEvent, EVENT_CANCEL, EVENT_ORDER, EVENT_REFUND, EVENT_STATUS,
    EVENT_UNKNOWN,
};

/// 默认网关（NOP 单端点；配置 `gatewayBase` 可覆盖，如沙箱
/// `https://open-api-sandbox.shop.ele.me/api/v1/`）。
pub const DEFAULT_GATEWAY: &str = "https://open-api.shop.ele.me/api/v1/";
/// 查询订单详情动作。
pub const ACTION_GET_ORDER: &str = "eleme.order.getOrder";
/// 确认接单动作（Lite 版，官方推荐）。
pub const ACTION_CONFIRM_ORDER: &str = "eleme.order.confirmOrderLite";
/// 同意退款动作。
pub const ACTION_AGREE_REFUND: &str = "eleme.order.agreeRefundLite";
/// 驳回退款动作。
pub const ACTION_DISAGREE_REFUND: &str = "eleme.order.disagreeRefundLite";
/// 开始自配送动作。
pub const ACTION_DELIVERY_BY_SELF: &str = "eleme.order.deliveryBySelfLite";
/// 确认送达动作。
pub const ACTION_RECEIVED_ORDER: &str = "eleme.order.receivedOrderLite";

/// 推送 type：订单生成（新订单）。
pub const PUSH_TYPE_ORDER_CREATED: i64 = 10;
/// 推送 type 取消族（官方消息口径归类）。
pub const PUSH_TYPES_CANCEL: [i64; 6] = [14, 15, 17, 23, 25, 35];
/// 物流同步口径码：开始自配送（后台手填，与美团同入口口径）。
pub const LOGISTICS_DELIVERY_START: i32 = 10;
/// 物流同步口径码：确认送达。
pub const LOGISTICS_DELIVERED: i32 = 40;

/// 出站签名（官方规则）：metas + params 全部属性 `key=json(value)`
/// 字典序拼接，`action + token + 拼接串 + app_secret` 做 MD5 大写。
#[must_use]
pub fn sign_request(
    action: &str,
    token: &str,
    pairs: &[(String, Value)],
    app_secret: &str,
) -> String {
    let mut strs: Vec<String> = pairs
        .iter()
        .map(|(k, v)| {
            format!(
                "{k}={}",
                serde_json::to_string(v).unwrap_or_else(|_| "null".to_string())
            )
        })
        .collect();
    strs.sort();
    let raw = format!("{action}{token}{}{app_secret}", strs.concat());
    hex::encode_upper(Md5::digest(raw.as_bytes()))
}

/// 推送验签签名（官方 Go SDK demo 算法）：七字段 `key=value`
/// 字典序拼接后 MD5 大写。注意：按 demo 口径**不含 app_secret**
/// （复核点 E-1，见文件头）。
#[must_use]
pub fn push_signature(
    app_id: i64,
    request_id: &str,
    msg_type: i64,
    message: &str,
    shop_id: i64,
    timestamp: i64,
    user_id: i64,
) -> String {
    let mut strs = vec![
        format!("appId={app_id}"),
        format!("message={message}"),
        format!("requestId={request_id}"),
        format!("shopId={shop_id}"),
        format!("timestamp={timestamp}"),
        format!("type={msg_type}"),
        format!("userId={user_id}"),
    ];
    strs.sort();
    hex::encode_upper(Md5::digest(strs.concat().as_bytes()))
}

/// 淘宝闪购适配器（无状态，凭据每次从配置传入）。
#[derive(Debug, Default)]
pub struct ElemeAdapter;

impl ElemeAdapter {
    fn client(&self, cfg: &ChannelConfig) -> ElemeClient {
        ElemeClient::new(cfg)
    }
}

/// 淘宝闪购出站 API 客户端（NOP 协议）。
pub struct ElemeClient {
    http: reqwest::Client,
    gateway_base: String,
    app_key: String,
    app_secret: String,
}

impl ElemeClient {
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
                cfg.gateway_base.trim().to_string()
            },
            app_key: cfg.app_id.clone(),
            app_secret: cfg.app_secret.clone(),
        }
    }

    /// 签名并调用一个 NOP 动作。成功判据：HTTP 2xx 且响应信封
    /// `error` 为空；返回 `result` 载荷。失败 Err 带渠道错误码与
    /// 原文摘要 —— 不把失败包装成成功。
    async fn call(&self, action: &str, access_token: &str, params: Value) -> Result<Value> {
        if access_token.trim().is_empty() {
            return Err(Error::biz_msg(
                "淘宝闪购门店授权令牌未配置（绑定 access_token 为空）",
            ));
        }
        let timestamp = now_seconds();
        let mut pairs: Vec<(String, Value)> = vec![
            ("app_key".to_string(), Value::String(self.app_key.clone())),
            ("timestamp".to_string(), json!(timestamp)),
        ];
        if let Some(obj) = params.as_object() {
            for (k, v) in obj {
                pairs.push((k.clone(), v.clone()));
            }
        }
        let signature = sign_request(action, access_token, &pairs, &self.app_secret);
        let body = json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "action": action,
            "token": access_token,
            "metas": { "app_key": self.app_key, "timestamp": timestamp },
            "params": params,
            "signature": signature,
        });
        let resp = self
            .http
            .post(&self.gateway_base)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::biz_msg(format!("淘宝闪购接口网络错误：{e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| Error::biz_msg(format!("淘宝闪购接口响应读取失败：{e}")))?;
        if !status.is_success() {
            return Err(Error::biz_msg(format!(
                "淘宝闪购接口 HTTP {status}：{}",
                text.chars().take(200).collect::<String>()
            )));
        }
        let value: Value = serde_json::from_str(&text).map_err(|e| {
            Error::biz_msg(format!("淘宝闪购接口响应不可解析：{e}"))
        })?;
        if let Some(err) = value.get("error").filter(|e| !e.is_null()) {
            let code = err
                .get("code")
                .map(|c| match c {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .unwrap_or_default();
            let message = err.get("message").and_then(Value::as_str).unwrap_or("");
            return Err(Error::biz_msg(format!(
                "淘宝闪购接口返回错误 {code}：{message}（原文：{}）",
                text.chars().take(200).collect::<String>()
            )));
        }
        Ok(value.get("result").cloned().unwrap_or(Value::Null))
    }

    /// 查询订单详情（`eleme.order.getOrder`，返回订单对象原文）。
    pub async fn get_order(&self, access_token: &str, external_order_id: &str) -> Result<Value> {
        self.call(
            ACTION_GET_ORDER,
            access_token,
            json!({ "orderId": external_order_id }),
        )
        .await
    }

    /// 确认接单（`eleme.order.confirmOrderLite`）。
    pub async fn confirm_order(&self, access_token: &str, external_order_id: &str) -> Result<()> {
        self.call(
            ACTION_CONFIRM_ORDER,
            access_token,
            json!({ "orderId": external_order_id }),
        )
        .await?;
        Ok(())
    }

    /// 退款应答（同意 `agreeRefundLite` / 驳回 `disagreeRefundLite`）。
    pub async fn respond_refund(
        &self,
        access_token: &str,
        external_order_id: &str,
        agree: bool,
        reason: &str,
    ) -> Result<()> {
        if agree {
            self.call(
                ACTION_AGREE_REFUND,
                access_token,
                json!({ "orderId": external_order_id }),
            )
            .await?;
        } else {
            self.call(
                ACTION_DISAGREE_REFUND,
                access_token,
                json!({ "orderId": external_order_id, "reason": reason }),
            )
            .await?;
        }
        Ok(())
    }

    /// 自配送状态同步：开始自配送 / 确认送达。其余口径码无对应
    /// 官方动作，明确 Err（不臆造映射）。
    pub async fn sync_logistics(
        &self,
        access_token: &str,
        external_order_id: &str,
        logistics_status: i32,
    ) -> Result<()> {
        let action = match logistics_status {
            LOGISTICS_DELIVERY_START => ACTION_DELIVERY_BY_SELF,
            LOGISTICS_DELIVERED => ACTION_RECEIVED_ORDER,
            other => {
                return Err(Error::biz_msg(format!(
                    "淘宝闪购物流同步仅支持 {LOGISTICS_DELIVERY_START}=开始自配送 / \
                     {LOGISTICS_DELIVERED}=确认送达，收到未支持的状态码 {other}"
                )));
            }
        };
        self.call(action, access_token, json!({ "orderId": external_order_id }))
            .await?;
        Ok(())
    }
}

/// 取值助手：字符串或数字 → 字符串（缺失为空串）。
fn v_str(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// 取值助手：数字或数字字符串 → f64（缺失为 0）。
fn v_f64(v: &Value, key: &str) -> f64 {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.trim().parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

/// 取值助手：数字或数字字符串 → i64（缺失为 0）。
fn v_i64(v: &Value, key: &str) -> i64 {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0),
        Some(Value::String(s)) => s.trim().parse::<i64>().unwrap_or(0),
        _ => 0,
    }
}

/// 解析饿了么订单对象（推送 message 内嵌或 getOrder 返回，字段同
/// 构）为归一订单。状态由 status / refundStatus 字符串归一，未知
/// 状态回落 paid（已支付订单才会进入推送/查询口径）；取消等事件
/// 覆盖由调用方处理。金额为元（浮点）。
fn parse_order(order: &Value, app_poi_code: &str) -> ExternalOrder {
    let mut items: Vec<ExternalOrderItem> = Vec::new();
    if let Some(groups) = order.get("groups").and_then(Value::as_array) {
        for group in groups {
            if let Some(arr) = group.get("items").and_then(Value::as_array) {
                for it in arr {
                    items.push(ExternalOrderItem {
                        external_spu_id: v_str(it, "id"),
                        external_sku_id: v_str(it, "skuId"),
                        name: v_str(it, "name"),
                        quantity: i32::try_from(v_i64(it, "quantity"))
                            .unwrap_or(1)
                            .max(1),
                        price: yuan_amount(v_f64(it, "price")),
                        product_id: 0,
                        sku_id: 0,
                    });
                }
            }
        }
    }
    let mut product_amount = yuan_amount(v_f64(order, "originalPrice"));
    if product_amount.is_zero() && !items.is_empty() {
        product_amount = items
            .iter()
            .map(|i| i.price * Decimal::from(i.quantity))
            .sum();
    }
    let mobile = order
        .get("phoneList")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| v_str(order, "phone"));
    ExternalOrder {
        external_order_id: v_str(order, "orderId"),
        app_poi_code: if app_poi_code.is_empty() {
            v_str(order, "shopId")
        } else {
            app_poi_code.to_string()
        },
        external_status: 0,
        status: eleme_status_normalize(&v_str(order, "status"), &v_str(order, "refundStatus"))
            .unwrap_or(EXT_STATUS_PAID)
            .to_string(),
        items,
        total_amount: yuan_amount(v_f64(order, "totalPrice")),
        product_amount,
        shipping_fee: yuan_amount(v_f64(order, "deliverFee")),
        consignee: v_str(order, "consignee"),
        mobile,
        address: v_str(order, "address"),
        remark: v_str(order, "description"),
    }
}

#[async_trait]
impl OrderChannelAdapter for ElemeAdapter {
    fn channel(&self) -> &'static str {
        "eleme"
    }

    fn verify_and_parse(
        &self,
        cfg: &ChannelConfig,
        _event_hint: &str,
        raw_body: &str,
    ) -> Result<PushEvent> {
        if !cfg.configured() {
            return Err(Error::biz_msg("淘宝闪购渠道未配置凭据"));
        }
        // 饿了么应用后台只配一个推送 URL，事件分类靠推送 type 与
        // 订单状态字符串（见文件头），不依赖 event_hint 分入口。
        let env: Value = serde_json::from_str(raw_body)
            .map_err(|e| Error::biz_msg(format!("推送不是合法 JSON：{e}")))?;
        let signature = env
            .get("signature")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if signature.is_empty() {
            return Err(Error::biz_msg("推送缺少 signature"));
        }
        let message = env
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let expect = push_signature(
            v_i64(&env, "appId"),
            &v_str(&env, "requestId"),
            v_i64(&env, "type"),
            &message,
            v_i64(&env, "shopId"),
            v_i64(&env, "timestamp"),
            v_i64(&env, "userId"),
        );
        if !signature.eq_ignore_ascii_case(&expect) {
            return Err(Error::biz_msg("推送验签失败"));
        }
        let msg: Value = serde_json::from_str(&message).unwrap_or(Value::Null);
        let msg_type = v_i64(&env, "type");
        let shop_id = v_i64(&env, "shopId");
        let poi = if shop_id != 0 {
            shop_id.to_string()
        } else {
            v_str(&msg, "shopId")
        };
        let external_order_id = v_str(&msg, "orderId");
        let mut event = PushEvent {
            event_type: EVENT_UNKNOWN.to_string(),
            external_order_id: external_order_id.clone(),
            order: None,
            refund_id: String::new(),
            note: String::new(),
        };
        if external_order_id.is_empty() {
            // 无订单号的推送（如店铺类消息）只落日志，不建单。
            return Ok(event);
        }
        // 未支付订单不落履约单（推送口径内订单均应已支付，此为
        // 防御：状态字符串明确 unpaid 时只记录事件）。
        if v_str(&msg, "status").eq_ignore_ascii_case("unpaid") {
            event.event_type = EVENT_STATUS.to_string();
            event.note = "未支付订单推送，不落履约单".to_string();
            return Ok(event);
        }
        let mut order = parse_order(&msg, &poi);
        if PUSH_TYPES_CANCEL.contains(&msg_type) {
            order.status = EXT_STATUS_CANCELLED.to_string();
            event.event_type = EVENT_CANCEL.to_string();
            event.note = v_str(&msg, "cancelReason");
        } else if msg_type == PUSH_TYPE_ORDER_CREATED {
            event.event_type = EVENT_ORDER.to_string();
        } else if order.status == EXT_STATUS_REFUNDING || order.status == EXT_STATUS_REFUNDED {
            // 退款进程由订单 refundStatus 识别（不依赖未证实的
            // 退款 type 码），事件标 refund 便于日志检索。
            event.event_type = EVENT_REFUND.to_string();
        } else {
            event.event_type = EVENT_STATUS.to_string();
        }
        event.order = Some(order);
        Ok(event)
    }

    async fn enrich_event(
        &self,
        cfg: &ChannelConfig,
        access_token: &str,
        mut event: PushEvent,
    ) -> Result<PushEvent> {
        // 未绑定（令牌为空）：保留轻量订单走 pending_binding 兜底。
        if access_token.trim().is_empty() {
            return Ok(event);
        }
        let Some(order) = &event.order else {
            return Ok(event);
        };
        if order.external_order_id.is_empty() {
            return Ok(event);
        }
        let was_cancelled = order.status == EXT_STATUS_CANCELLED;
        let poi = order.app_poi_code.clone();
        let ext_id = order.external_order_id.clone();
        // 已绑定却回查失败 → Err：接入层不向平台确认成功、等重推，
        // 不以残缺轻量订单冒充完整订单落库。
        let full = self.client(cfg).get_order(access_token, &ext_id).await?;
        let full_order = if full.get("orderId").is_some() {
            full.clone()
        } else if let Some(inner) = full.get("order") {
            inner.clone()
        } else {
            full
        };
        let mut fresh = parse_order(&full_order, &poi);
        if fresh.external_order_id.is_empty() {
            return Err(Error::biz_msg("淘宝闪购 getOrder 未返回订单号"));
        }
        if was_cancelled {
            // 取消事件已达时以取消为准（终态口径），回查只补全字段。
            fresh.status = EXT_STATUS_CANCELLED.to_string();
        }
        event.order = Some(fresh);
        Ok(event)
    }

    async fn confirm_order(
        &self,
        cfg: &ChannelConfig,
        _app_poi_code: &str,
        access_token: &str,
        external_order_id: &str,
    ) -> Result<()> {
        if !cfg.active() {
            return Err(Error::biz_msg("淘宝闪购渠道未启用或未配置凭据"));
        }
        self.client(cfg)
            .confirm_order(access_token, external_order_id)
            .await
    }

    async fn sync_logistics(
        &self,
        cfg: &ChannelConfig,
        _app_poi_code: &str,
        access_token: &str,
        external_order_id: &str,
        logistics_status: i32,
    ) -> Result<()> {
        if !cfg.active() {
            return Err(Error::biz_msg("淘宝闪购渠道未启用或未配置凭据"));
        }
        self.client(cfg)
            .sync_logistics(access_token, external_order_id, logistics_status)
            .await
    }

    async fn respond_refund(
        &self,
        cfg: &ChannelConfig,
        _app_poi_code: &str,
        access_token: &str,
        external_order_id: &str,
        _refund_id: &str,
        agree: bool,
        reason: &str,
    ) -> Result<()> {
        if !cfg.active() {
            return Err(Error::biz_msg("淘宝闪购渠道未启用或未配置凭据"));
        }
        // 饿了么退款应答按订单维度（Lite 接口无 refund_id 参数）。
        self.client(cfg)
            .respond_refund(access_token, external_order_id, agree, reason)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cfg() -> ChannelConfig {
        ChannelConfig {
            enabled: true,
            app_id: "test_app_key".into(),
            app_secret: "test_secret_456".into(),
            gateway_base: String::new(),
        }
    }

    /// 出站签名自洽向量：按官方规则以独立工具（python3 hashlib）
    /// 计算 —— action=eleme.order.confirmOrderLite、
    /// token=test_token_123、app_key="test_app_key"、
    /// orderId="123456789"、timestamp=1700000000、
    /// secret=test_secret_456 → 57DC3021…2498。官方未公开示例
    /// 向量，联调期以沙箱实调复核（复核点 E-2）。
    #[test]
    fn sign_request_matches_independent_vector() {
        let pairs = vec![
            ("app_key".to_string(), json!("test_app_key")),
            ("timestamp".to_string(), json!(1_700_000_000_i64)),
            ("orderId".to_string(), json!("123456789")),
        ];
        let sig = sign_request(
            "eleme.order.confirmOrderLite",
            "test_token_123",
            &pairs,
            "test_secret_456",
        );
        assert_eq!(sig, "57DC3021F30F621F283256C9434E2498");
    }

    /// 推送验签向量（官方 Go SDK demo 算法，独立工具计算）：
    /// appId=123456 / requestId=req-abc / type=10 /
    /// message={"orderId":"998877"} / shopId=777 /
    /// timestamp=1700000000 / userId=555 → 35FBB99B…B674。
    #[test]
    fn push_signature_matches_independent_vector() {
        let sig = push_signature(
            123456,
            "req-abc",
            10,
            "{\"orderId\":\"998877\"}",
            777,
            1_700_000_000,
            555,
        );
        assert_eq!(sig, "35FBB99BDE0F8D119CF4D7B9B896B674");
    }

    #[test]
    fn push_verify_accepts_good_and_rejects_bad() {
        let adapter = ElemeAdapter;
        let cfg = test_cfg();
        let body = json!({
            "appId": 123456,
            "requestId": "req-abc",
            "type": 10,
            "message": "{\"orderId\":\"998877\"}",
            "shopId": 777,
            "timestamp": 1_700_000_000_i64,
            "userId": 555,
            "signature": "35FBB99BDE0F8D119CF4D7B9B896B674",
        })
        .to_string();
        let event = adapter
            .verify_and_parse(&cfg, "push", &body)
            .expect("valid push sign should parse");
        assert_eq!(event.event_type, EVENT_ORDER);
        assert_eq!(event.external_order_id, "998877");
        let order = event.order.expect("light order body");
        assert_eq!(order.app_poi_code, "777");
        assert_eq!(order.status, EXT_STATUS_PAID);
        assert!(order.items.is_empty());

        let bad = body.replace("35FBB99B", "00000000");
        assert!(adapter.verify_and_parse(&cfg, "push", &bad).is_err());
    }

    #[test]
    fn push_parse_full_order_message() {
        let adapter = ElemeAdapter;
        let cfg = test_cfg();
        let message = json!({
            "orderId": "E2001",
            "shopId": 777,
            "status": "valid",
            "refundStatus": "noRefund",
            "totalPrice": 25.9,
            "originalPrice": 28.0,
            "deliverFee": 3.0,
            "consignee": "张三",
            "phoneList": ["13800000000"],
            "address": "上海市黄浦区南京东路 1 号",
            "description": "少放辣",
            "groups": [{
                "name": "分组1",
                "items": [
                    {"id": "SPU1", "skuId": "SKU1", "name": "电池 A", "price": 12.5, "quantity": 2},
                    {"id": "SPU2", "skuId": "SKU2", "name": "电池 B", "price": 3.0, "quantity": 1}
                ]
            }],
        })
        .to_string();
        let sig = push_signature(123456, "req-full", 10, &message, 777, 1_700_000_000, 555);
        let body = json!({
            "appId": 123456,
            "requestId": "req-full",
            "type": 10,
            "message": message,
            "shopId": 777,
            "timestamp": 1_700_000_000_i64,
            "userId": 555,
            "signature": sig,
        })
        .to_string();
        let event = adapter
            .verify_and_parse(&cfg, "push", &body)
            .expect("full order push should parse");
        let order = event.order.expect("order body");
        assert_eq!(order.external_order_id, "E2001");
        assert_eq!(order.status, "confirmed");
        assert_eq!(order.total_amount, Decimal::new(2590, 2));
        assert_eq!(order.product_amount, Decimal::new(2800, 2));
        assert_eq!(order.shipping_fee, Decimal::new(300, 2));
        assert_eq!(order.items.len(), 2);
        assert_eq!(order.items[0].external_sku_id, "SKU1");
        assert_eq!(order.items[0].external_spu_id, "SPU1");
        assert_eq!(order.items[0].quantity, 2);
        assert_eq!(order.items[0].price, Decimal::new(1250, 2));
        assert_eq!(order.mobile, "13800000000");
        assert_eq!(order.remark, "少放辣");
    }

    #[test]
    fn push_cancel_type_forces_cancelled() {
        let adapter = ElemeAdapter;
        let cfg = test_cfg();
        let message = "{\"orderId\":\"E3001\",\"shopId\":777,\"status\":\"valid\"}";
        let sig = push_signature(123456, "req-cancel", 14, message, 777, 1_700_000_000, 555);
        let body = json!({
            "appId": 123456,
            "requestId": "req-cancel",
            "type": 14,
            "message": message,
            "shopId": 777,
            "timestamp": 1_700_000_000_i64,
            "userId": 555,
            "signature": sig,
        })
        .to_string();
        let event = adapter
            .verify_and_parse(&cfg, "push", &body)
            .expect("cancel push should parse");
        assert_eq!(event.event_type, EVENT_CANCEL);
        assert_eq!(event.order.expect("order").status, EXT_STATUS_CANCELLED);
    }
}
