//! 抖音生活服务适配器（takeout::douyin，我方新增）：WebHooks 验签
//! 与订单通知解析、订单查询补全、团购券核销客户端（prepare →
//! verify）。抖音是**团购券核销模型**而非配送订单模型（设计
//! §1.1）：归一状态映射为 paid = 待核销 / completed = 已核销 /
//! refunded = 已退款 / cancelled = 已取消或过期，内部单仍走一期
//! `order_type = 13` 资金隔离口径，只作履约与展示镜像。
//!
//! # 依据（抖音开放平台官方文档，2026-11-21 核）
//! - WebHooks 接入（《WebHooks 接入》）：POST JSON
//!   `{event, client_key, content, log_id}`（content 为字符串化
//!   JSON）；请求头 `X-Douyin-Signature` =
//!   `sha1(app_secret + 消息体原文)` 小写 hex（官方 Go demo：
//!   按行读 body 拼接后签名，本实现兼容原文与去换行两种形态）；
//!   HTTP 200 且响应 < 2.5s 视为成功，非 200 平台重试；消息可能
//!   重复推送（接入层以渠道订单号幂等兜底）。回调地址配置时平台
//!   先发验证请求（content 为**对象**且含 `challenge`），须原样
//!   回 challenge —— 由路由层在接入前处理（见 public callback）。
//! - 订单通知事件 `life_trade_order_notify`：content =
//!   `{action, msg_time, order{order_id, pay_amount,
//!   original_amount, account_id, create_time, pay_time}}`
//!   （官方文档示例）。绑定键 `app_poi_code` := `account_id`
//!   （商户根账户）。金额按生活服务口径为**分**（验券接口文档
//!   对金额明确标注「单位分」；webhook 金额单位列为联调复核点
//!   D-1，凭据到位后以真实订单核对）。
//! - 订单查询（补全）：`GET /goodlife/v1/trade/order/query/`
//!   （《订单查询能力》：按订单 ID + 商户 id 查询，需申请「订单
//!   查询」能力）。响应字段结构未在公开文档完整给出，补全只做
//!   防御式提取（复核点 D-2），失败不阻断落单（webhook 已带金额）。
//! - 验券（履约本体）：`GET /goodlife/v1/fulfilment/certificate/
//!   prepare/`（验券准备：凭明文券码/加密数据换 verify_token 与
//!   加密券码，只查未使用券）→ `POST /goodlife/v1/fulfilment/
//!   certificate/verify/`（验券：verify_token + poi_id +
//!   encrypted_codes；`data.error_code = 0` 且逐张
//!   `verify_results[].result = 0` 为成功，result = 2 / 1208
//!   （已核销）在重试语义下亦代表核销成功，官方文档明示）。
//!   需「团购核销」能力审批 + 商家在抖音来客授权。本批实现完整
//!   客户端链（[`DouyinAdapter::verify_certificate`]），后台输码/
//!   扫码核销入口为后续小批（见二期报告登记）。
//! - **无对应接口的动作（不臆造）**：平台无「确认接单」（支付即
//!   生效）、无「配送物流同步」（到店核销无配送）、无商家侧
//!   「退款应答」（退款由平台主导，设计 §1.1）——三个 trait
//!   动作均返回说明性 Err，后台操作会看到真实原因。

use async_trait::async_trait;
use serde_json::{json, Value};
use sha1::{Digest, Sha1};

use nextrade_core::models::takeout::{
    douyin_action_normalize, fen_to_yuan, ChannelConfig, ExternalOrder, ExternalOrderItem,
    EXT_STATUS_CANCELLED, EXT_STATUS_COMPLETED, EXT_STATUS_PAID, EXT_STATUS_REFUNDED,
    EXT_STATUS_REFUNDING,
};
use nextrade_core::{Error, Result};

use super::channel::{
    OrderChannelAdapter, PushEvent, EVENT_CANCEL, EVENT_ORDER, EVENT_REFUND, EVENT_STATUS,
    EVENT_UNKNOWN,
};

/// 默认网关（配置 `gatewayBase` 可覆盖）。
pub const DEFAULT_GATEWAY: &str = "https://open.douyin.com";
/// 订单查询接口路径（goodlife 交易系统）。
pub const PATH_ORDER_QUERY: &str = "/goodlife/v1/trade/order/query/";
/// 验券准备接口路径。
pub const PATH_CERT_PREPARE: &str = "/goodlife/v1/fulfilment/certificate/prepare/";
/// 验券接口路径。
pub const PATH_CERT_VERIFY: &str = "/goodlife/v1/fulfilment/certificate/verify/";
/// 订单通知事件名（官方 WebHooks 文档示例）。
pub const EVENT_TRADE_ORDER_NOTIFY: &str = "life_trade_order_notify";

/// WebHooks 验签（官方规则）：`sha1(app_secret + 消息体)` 小写 hex。
#[must_use]
pub fn webhook_signature(app_secret: &str, raw_body: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(app_secret.as_bytes());
    hasher.update(raw_body.as_bytes());
    hex::encode(hasher.finalize())
}

/// 验签比对：官方 Go demo 按行读取 body 后拼接（去掉换行）再签名，
/// 故原文与去换行形态任一命中即通过（两者都是 secret + body 的
/// sha1，仅空白行处理不同）。
fn verify_webhook_signature(app_secret: &str, raw_body: &str, provided: &str) -> bool {
    if provided.eq_ignore_ascii_case(&webhook_signature(app_secret, raw_body)) {
        return true;
    }
    let stripped: String = raw_body
        .chars()
        .filter(|c| *c != '\n' && *c != '\r')
        .collect();
    stripped != raw_body
        && provided.eq_ignore_ascii_case(&webhook_signature(app_secret, &stripped))
}

/// 验券成功判据（官方文档）：`error_code = 0` 且每张券
/// `result = 0`；result = 2 / 1208（券码已核销）在幂等重试语义
/// 下亦代表核销成功。无逐张结果时以 error_code 为准。
#[must_use]
pub fn certificate_verify_succeeded(data: &Value) -> bool {
    if data.get("error_code").and_then(Value::as_i64) != Some(0) {
        return false;
    }
    match data.get("verify_results").and_then(Value::as_array) {
        Some(results) => results.iter().all(|r| {
            matches!(r.get("result").and_then(Value::as_i64), Some(0 | 2 | 1208))
        }),
        None => true,
    }
}

/// 抖音生活服务适配器（无状态，凭据每次从配置传入）。
#[derive(Debug, Default)]
pub struct DouyinAdapter;

impl DouyinAdapter {
    fn client(&self, cfg: &ChannelConfig) -> DouyinClient {
        DouyinClient::new(cfg)
    }

    /// 团购券核销（明文券码路径）：验券准备 → 验券确认完整链。
    /// 顾客到店出示券码时由门店侧调用；后台核销入口接线为后续
    /// 小批，本方法先行实装并保证口径与官方文档一致。
    ///
    /// # Errors
    /// 未启用 / 未授权 / 准备未返回可用券 / 验券未成功 → Err。
    pub async fn verify_certificate(
        &self,
        cfg: &ChannelConfig,
        access_token: &str,
        poi_id: &str,
        code: &str,
    ) -> Result<Value> {
        if !cfg.active() {
            return Err(Error::biz_msg("抖音渠道未启用或未配置凭据"));
        }
        let client = self.client(cfg);
        let prep = client
            .certificate_prepare(access_token, poi_id, Some(code), None)
            .await?;
        let verify_token = v_str(&prep, "verify_token");
        let mut encrypted_codes: Vec<String> = Vec::new();
        let mut order_id = v_str(&prep, "order_id");
        for key in ["certificates", "certificate_list"] {
            if let Some(arr) = prep.get(key).and_then(Value::as_array) {
                for cert in arr {
                    let ec = v_str(cert, "encrypted_code");
                    if !ec.is_empty() {
                        encrypted_codes.push(ec);
                    }
                    if order_id.is_empty() {
                        order_id = v_str(cert, "order_id");
                    }
                }
            }
        }
        if verify_token.is_empty() || encrypted_codes.is_empty() {
            return Err(Error::biz_msg(format!(
                "抖音验券准备未返回 verify_token / 加密券码（券码不可用或已核销）：{}",
                prep.to_string().chars().take(200).collect::<String>()
            )));
        }
        client
            .certificate_verify(
                access_token,
                poi_id,
                &verify_token,
                encrypted_codes,
                if order_id.is_empty() {
                    None
                } else {
                    Some(order_id.as_str())
                },
                None,
            )
            .await
    }
}

/// 抖音开放平台出站客户端（goodlife 信封：`{data, extra}`，
/// 成功判据 `error_code = 0`，失败带错误码与描述原文）。
pub struct DouyinClient {
    http: reqwest::Client,
    gateway_base: String,
}

impl DouyinClient {
    /// 由渠道配置构造。
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
        }
    }

    fn check_token(access_token: &str) -> Result<()> {
        if access_token.trim().is_empty() {
            return Err(Error::biz_msg(
                "抖音商家授权令牌未配置（绑定 access_token 为空）",
            ));
        }
        Ok(())
    }

    /// 解包 goodlife 信封：error_code 非 0 → Err（带描述）；成功
    /// 返回 `data` 载荷。
    fn unwrap(resp: &Value) -> Result<Value> {
        let data = resp.get("data").cloned().unwrap_or(Value::Null);
        let code = data
            .get("error_code")
            .and_then(Value::as_i64)
            .or_else(|| {
                resp.get("extra")
                    .and_then(|e| e.get("error_code"))
                    .and_then(Value::as_i64)
            })
            .unwrap_or(-1);
        if code != 0 {
            let desc = data
                .get("description")
                .and_then(Value::as_str)
                .or_else(|| {
                    resp.get("extra")
                        .and_then(|e| e.get("description"))
                        .and_then(Value::as_str)
                })
                .unwrap_or("");
            return Err(Error::biz_msg(format!(
                "抖音接口返回错误 {code}：{desc}（原文：{}）",
                resp.to_string().chars().take(200).collect::<String>()
            )));
        }
        Ok(data)
    }

    async fn get_json(
        &self,
        path: &str,
        access_token: &str,
        query: &[(&str, String)],
    ) -> Result<Value> {
        Self::check_token(access_token)?;
        let url = format!("{}{path}", self.gateway_base);
        let resp = self
            .http
            .get(&url)
            .header("access-token", access_token)
            .header("content-type", "application/json")
            .query(query)
            .send()
            .await
            .map_err(|e| Error::biz_msg(format!("抖音接口网络错误：{e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| Error::biz_msg(format!("抖音接口响应读取失败：{e}")))?;
        if !status.is_success() {
            return Err(Error::biz_msg(format!(
                "抖音接口 HTTP {status}：{}",
                text.chars().take(200).collect::<String>()
            )));
        }
        let value: Value = serde_json::from_str(&text)
            .map_err(|e| Error::biz_msg(format!("抖音接口响应不可解析：{e}")))?;
        Self::unwrap(&value)
    }

    async fn post_json(&self, path: &str, access_token: &str, body: Value) -> Result<Value> {
        Self::check_token(access_token)?;
        let url = format!("{}{path}", self.gateway_base);
        let resp = self
            .http
            .post(&url)
            .header("access-token", access_token)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::biz_msg(format!("抖音接口网络错误：{e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| Error::biz_msg(format!("抖音接口响应读取失败：{e}")))?;
        if !status.is_success() {
            return Err(Error::biz_msg(format!(
                "抖音接口 HTTP {status}：{}",
                text.chars().take(200).collect::<String>()
            )));
        }
        let value: Value = serde_json::from_str(&text)
            .map_err(|e| Error::biz_msg(format!("抖音接口响应不可解析：{e}")))?;
        Self::unwrap(&value)
    }

    /// 订单查询（按订单 ID + 商户根账户查询，返回 data 载荷原文，
    /// 字段提取由调用方防御式处理，见复核点 D-2）。
    pub async fn query_order(
        &self,
        access_token: &str,
        order_id: &str,
        account_id: &str,
    ) -> Result<Value> {
        self.get_json(
            PATH_ORDER_QUERY,
            access_token,
            &[
                ("order_id", order_id.to_string()),
                ("account_id", account_id.to_string()),
            ],
        )
        .await
    }

    /// 验券准备（明文券码 `code` 与扫码加密数据 `encrypted_data`
    /// 二选一；只返回未使用券的 verify_token 与加密券码）。
    pub async fn certificate_prepare(
        &self,
        access_token: &str,
        poi_id: &str,
        code: Option<&str>,
        encrypted_data: Option<&str>,
    ) -> Result<Value> {
        let mut query: Vec<(&str, String)> = vec![("poi_id", poi_id.to_string())];
        if let Some(c) = code {
            query.push(("code", c.to_string()));
        }
        if let Some(e) = encrypted_data {
            query.push(("encrypted_data", e.to_string()));
        }
        self.get_json(PATH_CERT_PREPARE, access_token, &query).await
    }

    /// 验券（核销）。成功判据见 [`certificate_verify_succeeded`]，
    /// 未成功 Err 带逐张结果原文。
    pub async fn certificate_verify(
        &self,
        access_token: &str,
        poi_id: &str,
        verify_token: &str,
        encrypted_codes: Vec<String>,
        order_id: Option<&str>,
        account_id: Option<&str>,
    ) -> Result<Value> {
        let mut body = json!({
            "verify_token": verify_token,
            "poi_id": poi_id,
            "encrypted_codes": encrypted_codes,
        });
        if let Some(oid) = order_id {
            body["order_id"] = json!(oid);
        }
        if let Some(acc) = account_id {
            body["account_id"] = json!(acc);
        }
        let data = self.post_json(PATH_CERT_VERIFY, access_token, body).await?;
        if certificate_verify_succeeded(&data) {
            Ok(data)
        } else {
            Err(Error::biz_msg(format!(
                "抖音验券未成功：{}",
                data.to_string().chars().take(200).collect::<String>()
            )))
        }
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

/// 取值助手：数字或数字字符串 → i64（缺失为 0）。
fn v_i64(v: &Value, key: &str) -> i64 {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0),
        Some(Value::String(s)) => s.trim().parse::<i64>().unwrap_or(0),
        _ => 0,
    }
}

/// 解析订单通知 content（官方示例结构）为统一事件。未知 action
/// 回落 paid（订单通知的基线语义是已支付订单触达）；无订单号时
/// 返回无订单体的未知事件（只落日志）。
fn parse_trade_content(content: &Value) -> PushEvent {
    let order_v = content.get("order").cloned().unwrap_or(Value::Null);
    let action = v_str(content, "action");
    let mut order_id = v_str(&order_v, "order_id");
    if order_id.is_empty() {
        order_id = v_str(content, "order_id");
    }
    let mut event = PushEvent {
        event_type: EVENT_UNKNOWN.to_string(),
        external_order_id: order_id.clone(),
        order: None,
        refund_id: String::new(),
        note: String::new(),
    };
    if order_id.is_empty() {
        return event;
    }
    let mut account_id = v_str(&order_v, "account_id");
    if account_id.is_empty() {
        account_id = v_str(content, "account_id");
    }
    let status = douyin_action_normalize(&action).unwrap_or(EXT_STATUS_PAID);
    event.event_type = match status {
        EXT_STATUS_REFUNDING | EXT_STATUS_REFUNDED => EVENT_REFUND.to_string(),
        EXT_STATUS_CANCELLED => EVENT_CANCEL.to_string(),
        EXT_STATUS_COMPLETED => EVENT_STATUS.to_string(),
        _ => EVENT_ORDER.to_string(),
    };
    event.order = Some(ExternalOrder {
        external_order_id: order_id,
        app_poi_code: account_id,
        external_status: 0,
        status: status.to_string(),
        items: Vec::new(),
        total_amount: fen_to_yuan(v_i64(&order_v, "pay_amount")),
        product_amount: fen_to_yuan(v_i64(&order_v, "original_amount")),
        shipping_fee: fen_to_yuan(0),
        consignee: String::new(),
        mobile: String::new(),
        address: String::new(),
        remark: String::new(),
    });
    event
}

/// 从订单查询响应中防御式提取商品行（复核点 D-2：响应结构以
/// 联调实测为准）。只在候选数组的行同时带 id 与名称、且带单价
/// 字段时才采信，避免按猜测的金额口径造行。
fn extract_items(data: &Value) -> Vec<ExternalOrderItem> {
    let root = if data.get("order").is_some() {
        data.get("order").cloned().unwrap_or(Value::Null)
    } else {
        data.clone()
    };
    for key in ["sku_list", "item_list", "goods_list", "items"] {
        let Some(arr) = root.get(key).and_then(Value::as_array) else {
            continue;
        };
        let mut out: Vec<ExternalOrderItem> = Vec::new();
        for it in arr {
            let sku_id = ["sku_id", "goods_id", "spu_id"]
                .iter()
                .map(|k| v_str(it, k))
                .find(|s| !s.is_empty())
                .unwrap_or_default();
            let name = ["sku_name", "goods_name", "title", "name"]
                .iter()
                .map(|k| v_str(it, k))
                .find(|s| !s.is_empty())
                .unwrap_or_default();
            let price_fen = ["price", "sale_price"]
                .iter()
                .map(|k| v_i64(it, k))
                .find(|n| *n > 0)
                .unwrap_or(0);
            if sku_id.is_empty() || name.is_empty() || price_fen == 0 {
                continue;
            }
            let qty = ["count", "num", "quantity", "buy_num"]
                .iter()
                .map(|k| v_i64(it, k))
                .find(|n| *n > 0)
                .unwrap_or(1);
            out.push(ExternalOrderItem {
                external_spu_id: sku_id.clone(),
                external_sku_id: sku_id,
                name,
                quantity: i32::try_from(qty).unwrap_or(1).max(1),
                price: fen_to_yuan(price_fen),
                product_id: 0,
                sku_id: 0,
            });
        }
        if !out.is_empty() {
            return out;
        }
    }
    Vec::new()
}

#[async_trait]
impl OrderChannelAdapter for DouyinAdapter {
    fn channel(&self) -> &'static str {
        "douyin"
    }

    fn verify_and_parse(
        &self,
        _cfg: &ChannelConfig,
        _event_hint: &str,
        _raw_body: &str,
    ) -> Result<PushEvent> {
        // 抖音签名在请求头内，无头签名的调用一律失败关闭。
        Err(Error::biz_msg(
            "抖音回调签名在请求头 X-Douyin-Signature 中，必须经带签名入口处理",
        ))
    }

    fn verify_and_parse_signed(
        &self,
        cfg: &ChannelConfig,
        _event_hint: &str,
        raw_body: &str,
        header_signature: Option<&str>,
    ) -> Result<PushEvent> {
        if !cfg.configured() {
            return Err(Error::biz_msg("抖音渠道未配置凭据"));
        }
        let Some(sig) = header_signature.filter(|s| !s.trim().is_empty()) else {
            return Err(Error::biz_msg("抖音回调缺少 X-Douyin-Signature 请求头"));
        };
        if !verify_webhook_signature(&cfg.app_secret, raw_body, sig) {
            return Err(Error::biz_msg("推送验签失败"));
        }
        let env: Value = serde_json::from_str(raw_body)
            .map_err(|e| Error::biz_msg(format!("推送不是合法 JSON：{e}")))?;
        let client_key = env.get("client_key").and_then(Value::as_str).unwrap_or("");
        if !client_key.is_empty() && client_key != cfg.app_id {
            return Err(Error::biz_msg("推送 client_key 与应用配置不一致"));
        }
        let content: Value = match env.get("content") {
            Some(Value::String(s)) => serde_json::from_str(s).unwrap_or(Value::Null),
            Some(other) => other.clone(),
            None => Value::Null,
        };
        // 回调地址验证请求（content 为对象且含 challenge）应由路由
        // 层在接入前回 challenge；走到这里说明未被拦截，防御性地
        // 只落日志不处理（不冒认业务事件）。
        if content.get("challenge").is_some() {
            return Ok(PushEvent {
                event_type: EVENT_UNKNOWN.to_string(),
                external_order_id: String::new(),
                order: None,
                refund_id: String::new(),
                note: "回调地址验证请求（未被路由层拦截）".to_string(),
            });
        }
        let event_name = env.get("event").and_then(Value::as_str).unwrap_or("");
        if event_name == EVENT_TRADE_ORDER_NOTIFY
            || content.get("order").is_some()
            || !v_str(&content, "order_id").is_empty()
        {
            Ok(parse_trade_content(&content))
        } else {
            // 其余事件（券消息等）本批不臆测结构：只落日志，人工
            // 排查后按官方事件文档增补解析。
            Ok(PushEvent {
                event_type: EVENT_UNKNOWN.to_string(),
                external_order_id: String::new(),
                order: None,
                refund_id: String::new(),
                note: format!("未订阅解析的抖音事件：{event_name}"),
            })
        }
    }

    async fn enrich_event(
        &self,
        cfg: &ChannelConfig,
        access_token: &str,
        mut event: PushEvent,
    ) -> Result<PushEvent> {
        // 未绑定（令牌为空）：webhook 已带金额，原样走待绑定兜底。
        if access_token.trim().is_empty() {
            return Ok(event);
        }
        let Some(order) = &event.order else {
            return Ok(event);
        };
        if order.external_order_id.is_empty() {
            return Ok(event);
        }
        let ext_id = order.external_order_id.clone();
        let account_id = order.app_poi_code.clone();
        // 订单查询为尽力补全（补商品行）：失败不阻断落单 —— 与
        // 饿了么轻推送不同，抖音 webhook 本身已带订单金额。
        if let Ok(data) = self
            .client(cfg)
            .query_order(access_token, &ext_id, &account_id)
            .await
        {
            let items = extract_items(&data);
            if !items.is_empty() {
                if let Some(order) = &mut event.order {
                    order.items = items;
                }
            }
        }
        Ok(event)
    }

    async fn confirm_order(
        &self,
        cfg: &ChannelConfig,
        _app_poi_code: &str,
        _access_token: &str,
        _external_order_id: &str,
    ) -> Result<()> {
        if !cfg.active() {
            return Err(Error::biz_msg("抖音渠道未启用或未配置凭据"));
        }
        Err(Error::biz_msg(
            "抖音团购无平台接单动作：订单支付即生效（待核销），履约为到店核销\
             （顾客出示券码后走验券准备 → 验券）；后台确认接单 / 自动接单对抖音\
             渠道不适用，请勿为抖音绑定开启自动接单",
        ))
    }

    async fn sync_logistics(
        &self,
        cfg: &ChannelConfig,
        _app_poi_code: &str,
        _access_token: &str,
        _external_order_id: &str,
        _logistics_status: i32,
    ) -> Result<()> {
        if !cfg.active() {
            return Err(Error::biz_msg("抖音渠道未启用或未配置凭据"));
        }
        Err(Error::biz_msg(
            "抖音生活服务团购为到店核销模型，无配送物流同步接口",
        ))
    }

    async fn respond_refund(
        &self,
        cfg: &ChannelConfig,
        _app_poi_code: &str,
        _access_token: &str,
        _external_order_id: &str,
        _refund_id: &str,
        _agree: bool,
        _reason: &str,
    ) -> Result<()> {
        if !cfg.active() {
            return Err(Error::biz_msg("抖音渠道未启用或未配置凭据"));
        }
        Err(Error::biz_msg(
            "抖音团购退款由平台主导（用户在抖音侧申请、按平台规则执行），\
             开放平台未提供商家退款应答接口；我方仅同步退款状态",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn test_cfg() -> ChannelConfig {
        ChannelConfig {
            enabled: true,
            app_id: "ck123".into(),
            app_secret: "douyin_secret".into(),
            gateway_base: String::new(),
        }
    }

    /// 官方验签向量（自洽）：secret = douyin_secret、body 为下方
    /// 字面量时 sha1 = 223ddce7…3717（以独立工具 python3 hashlib
    /// 计算；算法本身出自官方《WebHooks 接入》Go demo）。
    #[test]
    fn webhook_signature_matches_independent_vector() {
        let body = "{\"event\":\"life_trade_order_notify\",\"client_key\":\"ck123\",\"content\":\"{\\\"action\\\":\\\"pay_success\\\"}\",\"log_id\":\"x\"}";
        assert_eq!(
            webhook_signature("douyin_secret", body),
            "223ddce764c5fdb0e5b986242b4c99fb9d043717"
        );
    }

    #[test]
    fn signed_parse_pay_success() {
        let adapter = DouyinAdapter;
        let cfg = test_cfg();
        let content = json!({
            "action": "pay_success",
            "msg_time": 1_665_991_178_i64,
            "order": {
                "order_id": "D9001",
                "pay_amount": 19900,
                "original_amount": 25800,
                "account_id": "ACC42",
                "create_time": 1_665_991_178_i64,
                "pay_time": 1_665_991_178_i64
            }
        });
        let body = json!({
            "event": "life_trade_order_notify",
            "client_key": "ck123",
            "content": content.to_string(),
            "log_id": "log1"
        })
        .to_string();
        let sig = webhook_signature(&cfg.app_secret, &body);
        let event = adapter
            .verify_and_parse_signed(&cfg, "push", &body, Some(&sig))
            .expect("valid signature should parse");
        assert_eq!(event.event_type, EVENT_ORDER);
        let order = event.order.expect("order body");
        assert_eq!(order.external_order_id, "D9001");
        assert_eq!(order.app_poi_code, "ACC42");
        assert_eq!(order.status, EXT_STATUS_PAID);
        assert_eq!(order.total_amount, Decimal::new(19900, 2));
        assert_eq!(order.product_amount, Decimal::new(25800, 2));

        // 坏签名拒绝；无头签名拒绝；无签名入口（body 验签）失败关闭。
        assert!(adapter
            .verify_and_parse_signed(&cfg, "push", &body, Some("deadbeef"))
            .is_err());
        assert!(adapter.verify_and_parse_signed(&cfg, "push", &body, None).is_err());
        assert!(adapter.verify_and_parse(&cfg, "push", &body).is_err());
    }

    #[test]
    fn signed_parse_refund_action() {
        let adapter = DouyinAdapter;
        let cfg = test_cfg();
        let content = json!({
            "action": "refund_success",
            "order": { "order_id": "D9002", "pay_amount": 5000, "original_amount": 5000, "account_id": "ACC42" }
        });
        let body = json!({
            "event": "life_trade_order_notify",
            "client_key": "ck123",
            "content": content.to_string(),
            "log_id": "log2"
        })
        .to_string();
        let sig = webhook_signature(&cfg.app_secret, &body);
        let event = adapter
            .verify_and_parse_signed(&cfg, "push", &body, Some(&sig))
            .expect("refund notify should parse");
        assert_eq!(event.event_type, EVENT_REFUND);
        assert_eq!(event.order.expect("order").status, EXT_STATUS_REFUNDED);
    }

    #[test]
    fn verify_succeeded_judgement() {
        // 官方文档正常响应示例形态。
        let ok = json!({ "error_code": 0, "verify_results": [{ "result": 0 }] });
        assert!(certificate_verify_succeeded(&ok));
        // 已核销（幂等重试口径）亦代表成功。
        let idem = json!({ "error_code": 0, "verify_results": [{ "result": 1208 }] });
        assert!(certificate_verify_succeeded(&idem));
        // 券码已退款 → 未成功。
        let refunded = json!({ "error_code": 0, "verify_results": [{ "result": 1209 }] });
        assert!(!certificate_verify_succeeded(&refunded));
        // 接口级失败。
        let failed = json!({ "error_code": 3000001, "verify_results": [] });
        assert!(!certificate_verify_succeeded(&failed));
    }
}
