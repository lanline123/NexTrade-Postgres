//! 渠道适配抽象（takeout::channel，我方新增）：A 向订单汇入的渠道
//! 适配器契约 + 注册表，以及 B 向配送聚合的接口位。
//!
//! # A 向（一期）
//! 渠道差异（验签规则、推送载荷、回调用法）全部收敛在
//! [`OrderChannelAdapter`] 实现内，接入服务（`ingest`）只面对归一模型：
//! - [`PushEvent`]：一次推送解析后的统一事件；
//! - 出站动作：确认接单 / 自配送物流同步 / 退款应答，均为「调用渠道
//!   API」语义，失败必须以 Err 返回真实原因，禁止假成功。
//! 本期只有美团闪购有真实适配器（`meituan` 模块）；淘宝闪购 / 抖音
//! 在注册表中返回 `None`（未接入），后台如实展示。
//!
//! # B 向（只留接口位，设计 §二）
//! [`DeliveryAggregator`] 是配送聚合的出站契约。麦芽田的开放能力
//! 须商务洽谈取得（无自助注册、无公开 API 文档与价目，设计 §2.1
//! 标「未取得」），故 [`MaiyatianAggregator`] 只占位：全部方法返回
//! 「未启用」业务错误，**不写死任何接口地址/参数**，商务到位后按其
//! 正式文档实现同一 trait 即可接通。青云聚信（美团旗下）的发单方
//! 公开文档同样未取得（设计 §2.2），不单独建占位类，待文档到位后
//! 与麦芽田同式接入。

use async_trait::async_trait;
use serde_json::Value;

use nextrade_core::models::takeout::{ChannelConfig, ExternalOrder, CHANNEL_MEITUAN};
use nextrade_core::{Error, Result};

use super::meituan::MeituanAdapter;

/// 推送事件类型（回调日志 `event_type` 同口径）。
pub const EVENT_ORDER: &str = "order";
/// 取消推送。
pub const EVENT_CANCEL: &str = "cancel";
/// 退款推送（渠道侧发起，我方只同步状态）。
pub const EVENT_REFUND: &str = "refund";
/// 状态推送（确认/完成等后续状态变更）。
pub const EVENT_STATUS: &str = "status";
/// 无法归类的推送（仍落日志，人工排查）。
pub const EVENT_UNKNOWN: &str = "unknown";

/// 一次渠道推送解析后的统一事件。
#[derive(Debug, Clone)]
pub struct PushEvent {
    /// 事件类型（本模块 `EVENT_*`）。
    pub event_type: String,
    /// 渠道订单号（解析不到为空串，日志兜底）。
    pub external_order_id: String,
    /// 归一订单（状态类事件可只带状态字段；无订单体时为 None）。
    pub order: Option<ExternalOrder>,
    /// 退款事件的渠道退款单号（无则空串）。
    pub refund_id: String,
    /// 事件附带说明（取消原因 / 退款原因等，原文）。
    pub note: String,
}

/// A 向渠道适配器契约。
#[async_trait]
pub trait OrderChannelAdapter: Send + Sync {
    /// 渠道 id（`meituan` 等）。
    fn channel(&self) -> &'static str;

    /// 验证推送签名并解析为统一事件。
    ///
    /// `raw_body` 为回调原文（form-urlencoded）。验签失败必须返回
    /// Err（接入层据此拒绝并落日志），不得「解析优先、验签可选」。
    ///
    /// # Errors
    /// 验签失败 / 载荷不可解析 → Err（文案进回调日志）。
    fn verify_and_parse(
        &self,
        cfg: &ChannelConfig,
        event_hint: &str,
        raw_body: &str,
    ) -> Result<PushEvent>;

    /// 确认接单（出站调用渠道 API）。
    ///
    /// # Errors
    /// 未配置 / 渠道返回失败 / 网络错误 → Err（真实原因）。
    async fn confirm_order(
        &self,
        cfg: &ChannelConfig,
        app_poi_code: &str,
        access_token: &str,
        external_order_id: &str,
    ) -> Result<()>;

    /// 自配送物流状态同步（出站调用渠道 API）。
    ///
    /// `logistics_status` 为渠道口径状态码，由实现按其文档映射
    /// 我方配送节点；一期只提供手动触发入口（后台外部订单页）。
    ///
    /// # Errors
    /// 未配置 / 渠道返回失败 / 网络错误 → Err（真实原因）。
    async fn sync_logistics(
        &self,
        cfg: &ChannelConfig,
        app_poi_code: &str,
        access_token: &str,
        external_order_id: &str,
        logistics_status: i32,
    ) -> Result<()>;

    /// 退款应答（同意/驳回；退款执行在渠道侧，我方只回结果）。
    ///
    /// # Errors
    /// 未配置 / 渠道返回失败 / 网络错误 → Err（真实原因）。
    async fn respond_refund(
        &self,
        cfg: &ChannelConfig,
        app_poi_code: &str,
        access_token: &str,
        external_order_id: &str,
        refund_id: &str,
        agree: bool,
        reason: &str,
    ) -> Result<()>;
}

/// 按渠道 id 取适配器（注册表）。未接入的渠道返回 None —— 调用方
/// 必须据此给出「渠道未接入」而不是降级假处理。
#[must_use]
pub fn adapter_for(channel: &str) -> Option<std::sync::Arc<dyn OrderChannelAdapter>> {
    match channel {
        CHANNEL_MEITUAN => Some(std::sync::Arc::new(MeituanAdapter::default())),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// B 向：配送聚合接口位（不实现，商务前置）
// ---------------------------------------------------------------------------

/// B 向配送聚合出站契约（设计 §二）：我方订单发给聚合运力方
/// （麦芽田 / 青云聚信聚合送等）询价、发单、取消、状态回流。
/// 状态回流走对方回调进我方 `delivery_order` 状态链，接入时在本
/// trait 之外另建回调入口，不与 A 向回调混用。
#[async_trait]
pub trait DeliveryAggregator: Send + Sync {
    /// 聚合方 id（`maiyatian` 等）。
    fn name(&self) -> &'static str;

    /// 是否已取得商务凭据并完成配置（占位实现恒 false）。
    fn configured(&self) -> bool;

    /// 询价（返回聚合方原始报价 JSON，口径随其文档）。
    ///
    /// # Errors
    /// 未启用 → 业务错误（占位实现恒 Err）。
    async fn quote(&self, _order_payload: &Value) -> Result<Value>;

    /// 发单。
    ///
    /// # Errors
    /// 未启用 → 业务错误（占位实现恒 Err）。
    async fn create_order(&self, _order_payload: &Value) -> Result<Value>;

    /// 取消发单。
    ///
    /// # Errors
    /// 未启用 → 业务错误（占位实现恒 Err）。
    async fn cancel_order(&self, _aggregator_order_id: &str) -> Result<()>;
}

/// 麦芽田配送聚合占位（设计 §2.1）：其「聚合配送开放平台」无自助
/// 开发者注册、无公开 API 文档与价目，接入方式与计费须商务洽谈
/// 取得（用户已启动商务对接）。本类只固定接口位与诚实状态，
/// **不预置任何接口地址、参数名或签名方式**，防止按猜测写死后
/// 商务文档到位时整段返工。
pub struct MaiyatianAggregator;

#[async_trait]
impl DeliveryAggregator for MaiyatianAggregator {
    fn name(&self) -> &'static str {
        "maiyatian"
    }

    fn configured(&self) -> bool {
        false
    }

    async fn quote(&self, _order_payload: &Value) -> Result<Value> {
        Err(Error::biz_msg(
            "麦芽田配送聚合未启用：商务对接与接口文档未到位",
        ))
    }

    async fn create_order(&self, _order_payload: &Value) -> Result<Value> {
        Err(Error::biz_msg(
            "麦芽田配送聚合未启用：商务对接与接口文档未到位",
        ))
    }

    async fn cancel_order(&self, _aggregator_order_id: &str) -> Result<()> {
        Err(Error::biz_msg(
            "麦芽田配送聚合未启用：商务对接与接口文档未到位",
        ))
    }
}
