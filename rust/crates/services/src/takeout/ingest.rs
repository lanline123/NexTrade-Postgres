//! 订单接入服务（takeout::ingest，我方新增）：渠道回调的统一处理
//! 链 —— 配置门禁 → 验签 → 落日志 → 幂等落外部单 → 绑定解析 →
//! 生成内部履约单 → 自动确认 → 状态镜像同步。
//!
//! # 兜底口径（设计 §1.4）
//! - 未配置/未启用：拒绝并落日志，**不吞单、不假成功**（回调返回
//!   非 ok，渠道侧会按其重试规则处理，管理员在后台可见日志）；
//! - 验签失败：拒绝 + 日志（sign_verified = 0）；
//! - 门店未绑定：外部单照常落库（status = pending_binding），不生成
//!   内部单；补绑后后台「重新处理」即可补单，不丢单；
//! - 商品未映射：按渠道原始商品名落单（product_id = 0），事后补映射
//!   只影响后续新单，存量单不回改（与设计 §1.4.4 一致）；
//! - 终态不回退：已完成/已取消/已退款的单不接受更早状态覆盖；
//! - 资金隔离：内部单 `order_type = 13` + `external_channel` 标记，
//!   `is_need_commisson = 0`；退款/取消只同步状态镜像，绝不触发
//!   我方出款链（设计 §1.4.2 / §1.4.3）。

use std::sync::Arc;

use serde_json::json;

use nextrade_common::utils::{generate_order_sn, now_seconds};
use nextrade_core::entities::order::{Order, OrderItem};
use nextrade_core::models::takeout::{
    channel_name, effective_takeout_settings, internal_order_status, internal_pay_status,
    ChannelConfig, ExternalOrder, TakeoutSettings, EXT_STATUS_CANCELLED, EXT_STATUS_COMPLETED,
    EXT_STATUS_CONFIRMED, EXT_STATUS_PAID, EXT_STATUS_PENDING_BINDING, EXT_STATUS_REFUNDED,
    ORDER_TYPE_EXTERNAL, TAKEOUT_SETTINGS_CODE,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::takeout::order::ExternalOrderRow;
use nextrade_infrastructure::db::Repositories;

use super::channel::{adapter_for, OrderChannelAdapter, PushEvent, EVENT_ORDER};

/// 回调处理结果（路由层据此决定响应形态）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackOutcome {
    /// 已受理（渠道期望的成功响应）。
    Accepted,
    /// 渠道未配置/未启用（诚实拒绝，落日志）。
    NotConfigured,
    /// 验签失败或渠道未接入（拒绝，落日志）。
    Rejected(String),
    /// 已验签但处理异常（不确认成功，等待渠道重试）。
    ProcessingFailed(String),
}

/// 订单接入服务。
pub struct TakeoutIngestService {
    repos: Arc<Repositories>,
}

impl TakeoutIngestService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 读取生效配置（后台服务共用）。
    pub async fn settings(&self) -> Result<TakeoutSettings> {
        let stored = self
            .repos
            .system
            .config_basic
            .find_by_code(TAKEOUT_SETTINGS_CODE)
            .await?;
        Ok(effective_takeout_settings(stored.as_deref()))
    }

    fn now() -> i32 {
        i32::try_from(now_seconds()).unwrap_or(0)
    }

    async fn log(
        &self,
        channel: &str,
        event_type: &str,
        external_order_id: &str,
        sign_verified: i32,
        process_status: i32,
        message: &str,
        raw_body: &str,
    ) {
        // 日志是审计兜底，写失败不反噬回调主流程。
        let _ = self
            .repos
            .takeout
            .order
            .insert_callback_log(
                channel,
                event_type,
                external_order_id,
                sign_verified,
                process_status,
                message,
                raw_body,
                Self::now(),
            )
            .await;
    }

    /// 处理一次渠道回调（路由层只透传渠道、事件入口与原文）。
    pub async fn handle_callback(
        &self,
        channel: &str,
        event_hint: &str,
        raw_body: &str,
    ) -> CallbackOutcome {
        let settings = match self.settings().await {
            Ok(s) => s,
            Err(e) => {
                return CallbackOutcome::ProcessingFailed(format!("配置读取失败：{e}"));
            }
        };
        let Some(cfg) = settings.channel(channel).cloned() else {
            self.log(channel, event_hint, "", 0, 2, "未知渠道", raw_body)
                .await;
            return CallbackOutcome::Rejected("未知渠道".to_string());
        };
        let Some(adapter) = adapter_for(channel) else {
            self.log(channel, event_hint, "", 0, 2, "渠道未接入", raw_body)
                .await;
            return CallbackOutcome::Rejected("渠道未接入".to_string());
        };
        if !cfg.active() {
            self.log(
                channel,
                event_hint,
                "",
                0,
                2,
                "渠道未启用或未配置凭据",
                raw_body,
            )
            .await;
            return CallbackOutcome::NotConfigured;
        }
        let event = match adapter.verify_and_parse(&cfg, event_hint, raw_body) {
            Ok(ev) => ev,
            Err(e) => {
                self.log(
                    channel,
                    event_hint,
                    "",
                    0,
                    2,
                    &format!("验签/解析失败：{e}"),
                    raw_body,
                )
                .await;
                return CallbackOutcome::Rejected(format!("{e}"));
            }
        };
        match self.apply_event(channel, &cfg, &adapter, &event).await {
            Ok(()) => {
                self.log(
                    channel,
                    &event.event_type,
                    &event.external_order_id,
                    1,
                    0,
                    "",
                    raw_body,
                )
                .await;
                CallbackOutcome::Accepted
            }
            Err(e) => {
                self.log(
                    channel,
                    &event.event_type,
                    &event.external_order_id,
                    1,
                    1,
                    &format!("处理异常：{e}"),
                    raw_body,
                )
                .await;
                CallbackOutcome::ProcessingFailed(format!("{e}"))
            }
        }
    }

    /// 把已验签的统一事件落库（回调与后台「重新处理」共用）。
    ///
    /// # Errors
    /// DB 错误直传（调用方据此决定是否向渠道确认成功）。
    pub async fn apply_event(
        &self,
        channel: &str,
        cfg: &ChannelConfig,
        adapter: &Arc<dyn OrderChannelAdapter>,
        event: &PushEvent,
    ) -> Result<()> {
        let Some(order) = &event.order else {
            return Ok(());
        };
        let now = Self::now();
        let repo = &self.repos.takeout.order;
        let existing = repo.find(channel, &order.external_order_id).await?;

        // 终态不回退：已完成/已取消/已退款后到达的更早状态只刷新载荷。
        if let Some(old) = &existing {
            if is_terminal(&old.status) && old.status != order.status {
                let mut row = old.clone();
                row.last_event = event.event_type.clone();
                row.update_time = now;
                repo.update(&row).await?;
                return Ok(());
            }
        }

        // 绑定解析：app_poi_code → 门店。
        let binding = self
            .repos
            .takeout
            .binding
            .find_by_poi(channel, &order.app_poi_code)
            .await?;

        let mut row = existing.clone().unwrap_or_default();
        row.channel = channel.to_string();
        row.external_order_id = order.external_order_id.clone();
        row.external_status = order.external_status;
        row.raw_payload = serde_json::to_string(order).unwrap_or_default();
        row.last_event = event.event_type.clone();
        row.update_time = now;
        if row.add_time == 0 {
            row.add_time = now;
        }

        let Some(binding) = binding else {
            // 未绑定兜底：落外部单、不建内部单、明确标记待处理。
            row.shop_id = 0;
            row.binding_id = 0;
            row.status = if order.status == EXT_STATUS_CANCELLED {
                EXT_STATUS_CANCELLED.to_string()
            } else {
                EXT_STATUS_PENDING_BINDING.to_string()
            };
            row.total_amount = order.total_amount;
            row.product_amount = order.product_amount;
            row.shipping_fee = order.shipping_fee;
            row.consignee = order.consignee.clone();
            row.mobile = order.mobile.clone();
            row.address = order.address.clone();
            row.last_error = format!("门店未绑定（app_poi_code={}）", order.app_poi_code);
            upsert_external(repo, &row, existing.is_some()).await?;
            return Ok(());
        };

        row.shop_id = binding.shop_id;
        row.binding_id = binding.id;
        row.status = order.status.clone();
        row.total_amount = order.total_amount;
        row.product_amount = order.product_amount;
        row.shipping_fee = order.shipping_fee;
        row.consignee = order.consignee.clone();
        row.mobile = order.mobile.clone();
        row.address = order.address.clone();
        if row.last_error.starts_with("门店未绑定") {
            row.last_error = String::new();
        }

        // 商品映射解析（未映射保持 product_id = 0 按原名落单）。
        let mut items = order.items.clone();
        for item in &mut items {
            if let Some(m) = self
                .repos
                .takeout
                .mapping
                .find(channel, binding.shop_id, &item.external_sku_id)
                .await?
            {
                item.product_id = m.product_id;
                item.sku_id = m.sku_id;
            }
        }

        // 生成内部履约单（仅一次；取消/退款终态首达不建单）。
        if row.internal_order_id == 0
            && !matches!(
                order.status.as_str(),
                EXT_STATUS_CANCELLED | EXT_STATUS_REFUNDED
            )
        {
            let internal_id =
                self.create_internal_order(channel, order, &items, binding.shop_id, now)
                    .await?;
            row.internal_order_id = internal_id;
        }
        upsert_external(repo, &row, existing.is_some()).await?;

        // 内部单状态镜像同步。
        if row.internal_order_id > 0 {
            let cancel_note = if order.status == EXT_STATUS_CANCELLED {
                Some(if event.note.is_empty() {
                    "渠道侧取消"
                } else {
                    event.note.as_str()
                })
            } else {
                None
            };
            repo.update_internal_order_status(
                row.internal_order_id,
                internal_order_status(&order.status),
                internal_pay_status(&order.status),
                cancel_note,
            )
            .await?;
        }

        // 自动确认：新单（paid）且绑定开启自动接单时调用渠道确认。
        // 失败只记 last_error 不回滚落单（5 分钟红线内可后台手动补确认）。
        if event.event_type == EVENT_ORDER
            && order.status == EXT_STATUS_PAID
            && binding.auto_accept == 1
        {
            match adapter
                .confirm_order(
                    cfg,
                    &binding.app_poi_code,
                    &binding.access_token,
                    &order.external_order_id,
                )
                .await
            {
                Ok(()) => {
                    row.status = EXT_STATUS_CONFIRMED.to_string();
                    row.last_error = String::new();
                    repo.update(&row).await?;
                    if row.internal_order_id > 0 {
                        repo.update_internal_order_status(
                            row.internal_order_id,
                            internal_order_status(EXT_STATUS_CONFIRMED),
                            internal_pay_status(EXT_STATUS_CONFIRMED),
                            None,
                        )
                        .await?;
                    }
                }
                Err(e) => {
                    row.last_error = format!("自动确认失败：{e}");
                    repo.update(&row).await?;
                }
            }
        }
        Ok(())
    }

    /// 后台「重新处理」：对已落库的外部单（典型为补绑后）重跑接入。
    ///
    /// # Errors
    /// 外部单不存在 / 渠道未启用 / 载荷损坏 → 业务错误（真实原因）。
    pub async fn reprocess(&self, external_order_row_id: i32) -> Result<()> {
        let repo = &self.repos.takeout.order;
        let row = repo
            .find_by_id(external_order_row_id)
            .await?
            .ok_or_else(|| Error::biz_msg("外部订单不存在"))?;
        let settings = self.settings().await?;
        let cfg = settings
            .channel(&row.channel)
            .cloned()
            .ok_or_else(|| Error::biz_msg("未知渠道"))?;
        if !cfg.active() {
            return Err(Error::biz_msg("渠道未启用或未配置凭据"));
        }
        let adapter = adapter_for(&row.channel)
            .ok_or_else(|| Error::biz_msg("渠道未接入"))?;
        let order: ExternalOrder = serde_json::from_str(&row.raw_payload)
            .map_err(|e| Error::biz_msg(format!("外部单载荷损坏：{e}")))?;
        let event = PushEvent {
            event_type: EVENT_ORDER.to_string(),
            external_order_id: row.external_order_id.clone(),
            order: Some(order),
            refund_id: String::new(),
            note: String::new(),
        };
        self.apply_event(&row.channel, &cfg, &adapter, &event).await
    }

    /// 组装并落内部履约单（金额以渠道推送原样，资金链隔离字段在此定死）。
    async fn create_internal_order(
        &self,
        channel: &str,
        order: &ExternalOrder,
        items: &[nextrade_core::models::takeout::ExternalOrderItem],
        shop_id: i32,
        now: i32,
    ) -> Result<i32> {
        let mut o = Order::default();
        o.order_sn = generate_order_sn();
        o.order_type = Some(ORDER_TYPE_EXTERNAL);
        o.user_id = 0;
        o.order_status = internal_order_status(&order.status);
        o.shipping_status = 0;
        o.pay_status = internal_pay_status(&order.status);
        o.add_time = now;
        o.consignee = order.consignee.chars().take(255).collect();
        o.address = order.address.clone();
        o.region_ids = "[]".to_string();
        o.region_names = String::new();
        o.mobile = order.mobile.clone();
        o.buyer_note = order.remark.clone();
        o.admin_note = format!(
            "外卖聚合·{}订单 {}",
            channel_name(channel),
            order.external_order_id
        );
        // 配送方式：1 物流配送（渠道配送/自配送均归此口径履约）。
        o.shipping_method = 1;
        o.shipping_type_name = format!("{}配送", channel_name(channel));
        o.pay_time = now;
        o.shop_id = shop_id;
        o.is_valid = 1;
        o.total_amount = order.total_amount;
        o.paid_amount = order.total_amount;
        o.unpaid_amount = rust_decimal::Decimal::new(0, 0);
        o.unrefund_amount = order.total_amount;
        o.product_amount = order.product_amount;
        o.shipping_fee = order.shipping_fee;
        o.online_paid_amount = order.total_amount;
        // 资金隔离铁律：不分佣、不挂推广人（设计 §1.4.2）。
        o.is_need_commisson = 0;
        o.referrer_user_id = 0;
        o.order_source = format!("外卖聚合-{}", channel_name(channel));
        o.order_extension = json!({
            "takeout": {
                "channel": channel,
                "externalOrderId": order.external_order_id,
                "externalStatus": order.external_status,
            }
        })
        .to_string();
        o.external_channel = channel.to_string();
        o.external_order_sn = order.external_order_id.clone();

        let order_items: Vec<OrderItem> = items
            .iter()
            .map(|item| {
                let mut it = OrderItem::default();
                it.user_id = 0;
                it.price = item.price;
                it.quantity = item.quantity;
                it.product_id = item.product_id;
                it.product_name = item.name.chars().take(255).collect();
                it.product_sn = item.external_sku_id.clone();
                it.sku_id = item.sku_id;
                it.sku_data = json!({
                    "externalSpuId": item.external_spu_id,
                    "externalSkuId": item.external_sku_id,
                })
                .to_string();
                it.product_type = 1;
                it.shop_id = shop_id;
                it.origin_price = Some(item.price);
                it
            })
            .collect();
        self.repos
            .takeout
            .order
            .create_internal_order(&o, &order_items)
            .await
    }
}

/// 终态集合（不接受状态回退）。
fn is_terminal(status: &str) -> bool {
    matches!(
        status,
        EXT_STATUS_COMPLETED | EXT_STATUS_CANCELLED | EXT_STATUS_REFUNDED
    )
}

/// 外部单 upsert（先查后插的写路径收敛）。
async fn upsert_external(
    repo: &nextrade_infrastructure::db::repo::takeout::order::ExternalOrderRepository,
    row: &ExternalOrderRow,
    existed: bool,
) -> Result<()> {
    if existed {
        repo.update(row).await
    } else {
        repo.insert(row).await.map(|_| ())
    }
}
