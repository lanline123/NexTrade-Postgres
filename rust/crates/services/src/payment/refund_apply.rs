//! 退款申请服务：创建、审核（通过/退回/拒绝）、线下转账确认、可退额度校验。
//! 对应原 Java `RefundApplyServiceImpl`（1356 行）。
//!
//! 迁移要点（逐条对照 Java 源）：
//! - `assertRefundable`：**可退额度 = 实付 - 已占用**，
//!   已占用只统计待处理/处理中/已处理（`occupies_quota` 判定）
//! - `audit` 拒绝分支：拒绝原因必填且 ≤ 255 字；仅当状态为「待处理」时才可拒绝
//!   （用条件更新防并发重复处理）
//! - `refundRecordTx`：Java 用 REQUIRES_NEW 独立事务保证流水不随外层回滚 ——
//!   Rust 侧对应为「流水先落库，失败时显式置为失败态」
//! - `markRefundRecordFailed`：渠道未扣款时把流水置为失败，释放已占用额度
//!   （否则该笔支付永久无法再退款）
//!
//! 未迁移（依赖其他域）：
//! - 售后单状态同步（`aftersales` 域）
//! - 信用订单退款（`credit_order`）
//! - 积分退还（`returnUsePoints`）
//! - 渠道实际扣款（依赖各支付渠道实现）
//!
//! 已补齐（2026-11-20）：分销佣金回退——Java `handleSalesmanOrderOnRefund`
//! 照迁为 [`RefundApplyService::handle_salesman_order_on_refund`]，触发点
//! 与 Java 一致为财务审核通过（本服务 `audit` 的 Processing 流转成功后；
//! 售后自动执行路径在 `aftersales` 域的流转成功点同口径补挂，见该文件）。

use std::sync::Arc;

use rust_decimal::{Decimal, RoundingStrategy};
use nextrade_common::utils;
use nextrade_core::entities::{RefundApply, RefundLog};
use nextrade_core::models::payment::refund_apply::{
    total_refund, validate_reject_reason, OfflineAuditDTO, ProactiveRefundPreviewVO,
    RefundApplyAuditDTO, RefundApplyCreateDTO, RefundApplyDetailVO, RefundApplyListQuery,
    RefundApplyStatus, RefundApplyUpdateDTO, RefundApplyVO, RefundInitiator, RefundLogVO,
    RefundType,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::payment::paylog::ORDER_TYPE_PRODUCT;
use nextrade_infrastructure::db::Repositories;

/// 退款资金执行结果（`RefundApplyService::execute_refund_money`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoneyOutcome {
    /// 现金部分已同步执行完毕（余额已回补 / 渠道同步退款成功 /
    /// 无现金部分），申请可置已处理。
    Completed,
    /// 渠道退款已受理待回调，或该支付方式无自动退款轨——申请
    /// 停在处理中等待收尾，不得置已处理。
    ChannelPending,
}

pub struct RefundApplyService {
    repos: Arc<Repositories>,
}

impl RefundApplyService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 查询 —————————————————————

    /// 退款申请分页。
    pub async fn list(&self, q: &RefundApplyListQuery) -> Result<PageResult<RefundApplyVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        let (total, rows) = self
            .repos.payment.refund_apply
            .paginate(q.order_sn.as_deref(), q.refund_status, q.shop_id, &page)
            .await?;
        let list = rows.iter().map(to_vo).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 退款申请详情（含操作日志）。
    /// 退款申请归属店铺（供管理端租户归属校验）。
    pub async fn shop_id_of(&self, refund_id: i32) -> Result<i32> {
        self.repos
            .payment
            .refund_apply
            .find_shop_id(refund_id)
            .await?
            .ok_or_else(|| Error::NotFound("退款申请不存在".into()))
    }

    pub async fn detail(&self, refund_id: i32) -> Result<RefundApplyDetailVO> {
        let r = self
            .repos.payment.refund_apply
            .find_by_id(refund_id)
            .await?
            .ok_or_else(|| Error::NotFound("退款申请不存在".into()))?;
        let logs = self.repos.payment.refund_apply.find_logs(refund_id).await?;
        Ok(RefundApplyDetailVO {
            base: to_vo(&r),
            logs: logs.iter().map(to_log_vo).collect(),
        })
    }

    /// 按退款流水反查申请。
    pub async fn by_paylog_refund_id(
        &self,
        paylog_refund_id: i32,
    ) -> Result<Option<RefundApplyVO>> {
        let r = self
            .repos.payment.refund_apply
            .find_by_paylog_refund_id(paylog_refund_id)
            .await?;
        Ok(r.as_ref().map(to_vo))
    }

    /// 审核状态配置（供前端下拉）。
    ///
    /// 关联函数（不依赖 `repos`），便于直接测试。
    pub fn status_config() -> Vec<(i32, &'static str)> {
        [
            RefundApplyStatus::Wait,
            RefundApplyStatus::Processing,
            RefundApplyStatus::Processed,
            RefundApplyStatus::Cancel,
            RefundApplyStatus::Rejected,
        ]
        .iter()
        .map(|s| (*s as i32, s.name()))
        .collect()
    }

    // ————————————————————— 创建 —————————————————————

    /// 创建退款申请。
    ///
    /// 校验链：
    /// 1. 退款类型合法（1/2）
    /// 2. 订单/用户/售后单 ID 合法
    /// 3. 线上+线下退款金额 > 0
    /// 4. **退款总额不得超过可退额度**（实付 - 已占用）
    pub async fn create(&self, dto: &RefundApplyCreateDTO) -> Result<i32> {
        let refund_type = match dto.refund_type {
            Some(t) => RefundType::try_from(t)
                .map_err(|_| Error::Validation("退款类型非法（应为 1/2）".into()))?,
            None => RefundType::Order,
        };

        let order_id = dto
            .order_id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::Validation("订单ID无效".into()))?;
        let user_id = dto.user_id.filter(|id| *id > 0).unwrap_or(0);
        let aftersale_id = dto.aftersale_id.unwrap_or(0);
        let shop_id = dto.shop_id.unwrap_or(0);

        let online = dto.online_balance.unwrap_or_default().max(Decimal::ZERO);
        let offline = dto.offline_balance.unwrap_or_default().max(Decimal::ZERO);
        let total = total_refund(online, offline);
        if total <= Decimal::ZERO {
            return Err(Error::Validation("退款金额必须大于0".into()));
        }

        // 可退额度校验
        self.assert_refundable(order_id, total).await?;

        let r = RefundApply {
            // 自增主键占位
            refund_id: 0,
            refund_type: refund_type as i32,
            order_id,
            user_id,
            aftersale_id,
            refund_status: RefundApplyStatus::Wait as i32,
            add_time: utils::now_seconds() as i32,
            refund_note: dto.refund_note.clone().unwrap_or_default(),
            online_balance: online,
            offline_balance: offline,
            refund_balance: total,
            // 线上/线下标记由金额推导
            is_online: i32::from(online > Decimal::ZERO),
            is_offline: i32::from(offline > Decimal::ZERO),
            is_receive: 0,
            paylog_refund_id: 0,
            shop_id,
            payment_voucher: None,
            // 通用创建（财务手工）发起方恒为 0；主动退款走 create_proactive
            initiator: 0,
        };
        let id = self.repos.payment.refund_apply.insert(&r).await?;
        tracing::info!(
            "创建退款申请: id={id} order_id={order_id} amount={total}（线上{online}/线下{offline}）"
        );
        Ok(id)
    }

    // ————————————————————— 主动退款（我方新增） —————————————————————

    /// 主动退款预检：作用域 + 订单状态门校验后，返回实付与可退余额
    /// （后台弹窗打开时拉取，金额默认 = 可退余额）。
    pub async fn proactive_preview(
        &self,
        order_id: i32,
        scope_shop_id: Option<i32>,
    ) -> Result<ProactiveRefundPreviewVO> {
        let order = self.load_proactive_order(order_id, scope_shop_id).await?;
        Ok(ProactiveRefundPreviewVO {
            order_id: order.order_id,
            order_sn: order.order_sn.clone(),
            paid_amount: self.paid_amount(order_id).await?,
            refundable_amount: self.refundable_amount(order_id).await?,
            is_settlement: order.is_settlement.unwrap_or(0),
        })
    }

    /// 发起主动退款（平台/商家对已支付订单协商退款：退差价等场景）。
    ///
    /// 口径（我方新增功能，评估 forum-post-877927 定版）：
    /// - 买家、店铺一律取订单本身，发起方 `initiator` 与作用域由
    ///   路由层按登录态推导（1 平台 / 2 商家），请求参数不采信；
    /// - 金额为**退款总额口径**（与售后来源申请一致）：资金执行时
    ///   先扣礼品卡抵扣部分走完成点返还钩子，现金部分走支付轨；
    /// - 落 Wait 态 refund_apply 后走既有财务审核出款链；
    ///   `financialReview != 1` 时与售后同口径自动执行；
    /// - 不回补库存（纯资金返还，无退货语义）；买家以站内信通知。
    pub async fn create_proactive(
        &self,
        order_id: i32,
        amount: Decimal,
        reason: &str,
        initiator: i32,
        scope_shop_id: Option<i32>,
    ) -> Result<i32> {
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(Error::Validation("退款原因不能为空".into()));
        }
        if reason.chars().count() > 255 {
            return Err(Error::Validation("退款原因不能超过255字".into()));
        }
        if amount <= Decimal::ZERO {
            return Err(Error::Validation("退款金额必须大于0".into()));
        }
        if initiator != RefundInitiator::Platform as i32
            && initiator != RefundInitiator::Merchant as i32
        {
            return Err(Error::Validation("发起方标记非法".into()));
        }
        let order = self.load_proactive_order(order_id, scope_shop_id).await?;
        // 可退额度校验（实付 - 已占用，与售后/手工申请同一防线）
        self.assert_refundable(order_id, amount).await?;

        let now = utils::now_seconds() as i32;
        let mut apply = RefundApply {
            refund_id: 0,
            refund_type: RefundType::Order as i32,
            order_id,
            user_id: order.user_id,
            aftersale_id: 0,
            refund_status: RefundApplyStatus::Wait as i32,
            add_time: now,
            refund_note: reason.to_string(),
            online_balance: amount,
            offline_balance: Decimal::ZERO,
            refund_balance: amount,
            is_online: 1,
            is_offline: 0,
            is_receive: 0,
            paylog_refund_id: 0,
            shop_id: order.shop_id,
            payment_voucher: None,
            initiator: i16::try_from(initiator).unwrap_or_default(),
        };
        let refund_id = self.repos.payment.refund_apply.insert(&apply).await?;
        apply.refund_id = refund_id;
        let who = if initiator == RefundInitiator::Merchant as i32 {
            "商家"
        } else {
            "平台"
        };
        // 发起日志（财务详情时间线可见，与售后建单日志同构；尽力而为）
        let _ = self.repos.payment.refund_apply.insert_log(&RefundLog {
            log_id: 0,
            refund_apply_id: refund_id,
            order_id,
            refund_type: RefundType::Order as i32,
            refund_pay_code: "online".into(),
            transaction_id: String::new(),
            refund_amount: amount,
            add_time: now,
            description: Some(format!("{who}主动退款发起：{reason}")),
            user_id: order.user_id,
        }).await;
        tracing::info!(
            "主动退款发起: id={refund_id} order_id={order_id} amount={amount} initiator={initiator}（{who}）"
        );

        // 买家站内信（best-effort：失败只告警，不回滚已创建的申请）
        self.notify_buyer_proactive(&order, amount, reason, initiator)
            .await;

        // 财务审核关闭（financialReview != 1）时与售后同口径自动执行
        let financial_review = self
            .repos
            .system
            .config_basic
            .find_by_code("financialReview")
            .await?
            .unwrap_or_default();
        if financial_review != "1" {
            self.auto_execute_proactive(&apply, refund_id).await;
        }
        Ok(refund_id)
    }

    /// 系统规则退款（我方新增，送礼物订单流使用）：与主动退款同一资金
    /// 执行链（`execute_refund_money` 原路退回 / 余额回补 / 礼品卡返还 /
    /// 结算扣回），区别只有两点：
    /// 1. 发起方固定记为平台（initiator=Platform），备注与日志写明系统
    ///    触发原因，财务侧可追溯；
    /// 2. **不经财务审核队列、无条件立即自动执行**：这类退款由明确的
    ///    业务规则触发（礼物超时未领取 / 买家领取前取消），买家无过错，
    ///    「到期自动退款」的承诺不能卡在人工队列里；financialReview
    ///    开关只管人工发起的主动退款，不覆盖系统规则退款。
    pub async fn create_system_refund(
        &self,
        order_id: i32,
        amount: Decimal,
        reason: &str,
    ) -> Result<i32> {
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(Error::Validation("退款原因不能为空".into()));
        }
        if reason.chars().count() > 255 {
            return Err(Error::Validation("退款原因不能超过255字".into()));
        }
        if amount <= Decimal::ZERO {
            return Err(Error::Validation("退款金额必须大于0".into()));
        }
        let order = self.load_proactive_order(order_id, None).await?;
        self.assert_refundable(order_id, amount).await?;

        let now = utils::now_seconds() as i32;
        let mut apply = RefundApply {
            refund_id: 0,
            refund_type: RefundType::Order as i32,
            order_id,
            user_id: order.user_id,
            aftersale_id: 0,
            refund_status: RefundApplyStatus::Wait as i32,
            add_time: now,
            refund_note: reason.to_string(),
            online_balance: amount,
            offline_balance: Decimal::ZERO,
            refund_balance: amount,
            is_online: 1,
            is_offline: 0,
            is_receive: 0,
            paylog_refund_id: 0,
            shop_id: order.shop_id,
            payment_voucher: None,
            initiator: RefundInitiator::Platform as i16,
        };
        let refund_id = self.repos.payment.refund_apply.insert(&apply).await?;
        apply.refund_id = refund_id;
        let _ = self.repos.payment.refund_apply.insert_log(&RefundLog {
            log_id: 0,
            refund_apply_id: refund_id,
            order_id,
            refund_type: RefundType::Order as i32,
            refund_pay_code: "online".into(),
            transaction_id: String::new(),
            refund_amount: amount,
            add_time: now,
            description: Some(format!("系统自动退款发起：{reason}")),
            user_id: order.user_id,
        }).await;
        tracing::info!(
            "系统自动退款发起: id={refund_id} order_id={order_id} amount={amount} reason={reason}"
        );
        self.notify_buyer_proactive(&order, amount, reason, RefundInitiator::Platform as i32)
            .await;
        self.auto_execute_proactive(&apply, refund_id).await;
        Ok(refund_id)
    }

    /// 主动退款的目标订单加载 + 强校验：
    /// - 作用域：商家账号（scope 有值）只能操作本店订单——店铺取
    ///   订单本身比对，**不采信请求参数**；平台账号不限制；
    /// - 状态门：见 [`proactive_gate_error`]（拆单父单/未支付/
    ///   已取消/已失效一律拒绝，退款单位是有效（子）订单）。
    async fn load_proactive_order(
        &self,
        order_id: i32,
        scope_shop_id: Option<i32>,
    ) -> Result<nextrade_core::entities::Order> {
        if order_id <= 0 {
            return Err(Error::Validation("订单ID无效".into()));
        }
        let order = self
            .repos
            .trade
            .order_admin
            .find_order(order_id)
            .await?
            .ok_or_else(|| Error::NotFound("订单不存在".into()))?;
        if let Some(scope) = scope_shop_id {
            if order.shop_id != scope {
                return Err(Error::Forbidden(format!(
                    "无权访问店铺 {} 的数据",
                    order.shop_id
                )));
            }
        }
        if let Some(msg) = proactive_gate_error(order.order_status, order.pay_status, order.is_valid)
        {
            return Err(Error::biz(400, msg));
        }
        Ok(order)
    }

    /// 财务审核关闭时的主动退款自动执行（与售后自动路径同构）：
    /// 资金执行 → Wait 流转到已处理/处理中 → 完成点钩子（礼品卡
    /// 返还 + 已结算扣回）与分销回退挂点（无退货项，自然空转）。
    async fn auto_execute_proactive(&self, apply: &RefundApply, refund_id: i32) {
        let (to_status, completed, card_part) =
            match self.execute_refund_money(apply, refund_id).await {
            Ok((MoneyOutcome::Completed, _, part)) => {
                (RefundApplyStatus::Processed as i32, true, part)
            }
            Ok((MoneyOutcome::ChannelPending, _, _)) => {
                (RefundApplyStatus::Processing as i32, false, Decimal::ZERO)
            }
            Err(e) => {
                tracing::warn!(
                    "主动退款自动执行 资金执行失败 refund_id={refund_id} order_id={}: {e}",
                    apply.order_id
                );
                (RefundApplyStatus::Processing as i32, false, Decimal::ZERO)
            }
        };
        let transitioned = self
            .repos
            .payment
            .refund_apply
            .transition_status(
                refund_id,
                &[RefundApplyStatus::Wait as i32],
                to_status,
                Some("财务审核关闭自动执行退款"),
            )
            .await
            .unwrap_or(false);
        if !transitioned {
            return;
        }
        // 分销佣金回退挂点与财务审核路径保持同点：主动退款无退货
        // 商品项，handler 按 aftersale_id=0 查不到退货项即空转返回
        // （不退货即不按件回退佣金，口径见 handler 文档）。
        if let Err(e) = self
            .handle_salesman_order_on_refund(apply.order_id, 0)
            .await
        {
            tracing::warn!(
                "主动退款自动执行 分销佣金回退失败 refund_id={refund_id} order_id={}: {e}",
                apply.order_id
            );
        }
        if completed {
            // 礼品卡返还（与 mark_processed 同为退款完成点）：按
            // 资金执行算出的本笔卡构成按额返还（子单/部分退款
            // 精确化口径），仓储层以归属份额剩余额度钳制防重。
            if apply.order_id > 0 {
                if let Err(e) = self
                    .repos
                    .marketing
                    .gift_card_issue
                    .return_stored_cards_amount(
                        apply.order_id,
                        card_part,
                        utils::now_seconds(),
                    )
                    .await
                {
                    tracing::warn!(
                        "主动退款自动执行 礼品卡返还失败 refund_id={refund_id} order_id={}: {e}",
                        apply.order_id
                    );
                }
            }
            self.settle_proactive_refund(refund_id, apply).await;
        }
    }

    /// 主动退款完成后的结算收尾（best-effort）：已结算订单由
    /// [`crate::trade::settlement::SettlementService`] 从商家结算款
    /// 扣回并落负向流水；未结算订单在日后店铺结算时净额扣除
    /// （store_settlement 已计入已完成主动退款）。售后来源申请的
    /// 同等收尾由售后域 `on_refund_completed` 承担，互不重复。
    async fn settle_proactive_refund(&self, refund_id: i32, apply: &RefundApply) {
        let svc = crate::trade::settlement::SettlementService::new(self.repos.clone());
        if let Err(e) = svc
            .on_proactive_refund_completed(refund_id, apply.order_id, apply.refund_balance)
            .await
        {
            tracing::warn!(
                "主动退款结算扣回失败 refund_id={refund_id} order_id={}: {e}",
                apply.order_id
            );
        }
    }

    /// 主动退款买家站内信（best-effort）。
    async fn notify_buyer_proactive(
        &self,
        order: &nextrade_core::entities::Order,
        amount: Decimal,
        reason: &str,
        initiator: i32,
    ) {
        if order.user_id <= 0 {
            return;
        }
        let who = if initiator == RefundInitiator::Merchant as i32 {
            "商家"
        } else {
            "平台"
        };
        let content = format!(
            "您的订单 {} 已由{}发起退款，退款金额 ¥{}，原因：{}。退款将在审核通过后原路退回，请注意查收。",
            order.order_sn, who, amount, reason
        );
        if let Err(e) = self
            .repos
            .member
            .user_message
            .insert_user_message(order.user_id, "退款通知", &content, 1, None)
            .await
        {
            tracing::warn!(
                "主动退款买家通知失败 order_id={} user={}: {e}",
                order.order_id, order.user_id
            );
        }
    }

    // ————————————————————— 买家取消直退（我方新增·按用户口径） —————————————————————

    /// 读取买家取消直退设置（生效值）。
    async fn paid_cancel_settings(
        &self,
    ) -> Result<crate::system::paid_cancel_refund::PaidCancelRefundSettings> {
        let raw = self
            .repos
            .system
            .config_basic
            .find_by_code(crate::system::paid_cancel_refund::PAID_CANCEL_REFUND_CODE)
            .await?;
        Ok(crate::system::paid_cancel_refund::effective_paid_cancel_refund(
            raw.as_deref(),
        ))
    }

    /// 买家取消已付未发货订单并发起直退（论坛帖 347748，用户
    /// 2026-11-21 20:45–20:46 拍板口径，我方新增）：
    /// - 平台级设置主开关默认关：关时端点直接拒绝，买家侧无入口，
    ///   全站零行为变化；
    /// - 仅「已支付 + 未发货」的（子）订单可走本通道：整单实付
    ///   （paylog 口径）全额退款，已有售后/退款占用时拒绝并引导
    ///   售后通道；拆单以子单为单位判定，已发货子单不走本通道；
    /// - 落 Wait 态 `refund_apply`（来源 `initiator = 3` 买家取消），
    ///   按设置矩阵分流：小额免审命中 → 自动通过直达出款；进财务
    ///   审核队列开 → 留在队列等财务审核；否则直接出款；
    /// - 出款一律走既有财务退款链的资金执行（`execute_refund_money`）
    ///   与完成点钩子（礼品卡按额返还、结算扣回、佣金回冲），
    ///   不新造出款通道；退款完成即关闭订单（见
    ///   [`Self::close_order_for_buyer_cancel`]）。
    pub async fn create_buyer_cancel(
        &self,
        order_id: i32,
        user_id: i32,
        reason: Option<&str>,
    ) -> Result<i32> {
        let settings = self.paid_cancel_settings().await?;
        if !settings.enabled {
            return Err(Error::biz(400, "买家取消退款功能未开启"));
        }
        if order_id <= 0 {
            return Err(Error::Validation("订单ID无效".into()));
        }
        let order = self
            .repos
            .trade
            .order_admin
            .find_order(order_id)
            .await?
            .ok_or_else(|| Error::NotFound("订单不存在".into()))?;
        if order.user_id != user_id {
            return Err(Error::Forbidden("无权操作该订单".into()));
        }
        if let Some(msg) = buyer_cancel_gate_error(
            order.order_status,
            order.pay_status,
            order.shipping_status,
            order.is_valid,
            order.order_type,
        ) {
            return Err(Error::biz(400, msg));
        }
        // 定金预售：已付定金、待付尾款阶段不可取消（与后台取消同口径）
        if order.presale_type == Some(2) {
            if let Some(presale) = self
                .repos
                .payment
                .pay
                .find_presale_by_order_id(order_id)
                .await?
            {
                if presale.presale_status == 1 {
                    return Err(Error::biz(
                        400,
                        "预售订单已支付定金，待付尾款阶段无法取消",
                    ));
                }
            }
        }
        // 防重复提交：已有进行中的买家取消申请
        if self
            .repos
            .payment
            .refund_apply
            .exists_active_by_order_initiator(
                order_id,
                RefundInitiator::BuyerCancel as i32,
            )
            .await?
        {
            return Err(Error::biz(400, "已提交取消退款申请，请勿重复提交"));
        }
        // 金额 = 整单实付；必须全额可退（部分占用说明已有售后/退款
        // 在途，整单关闭语义不成立，引导回售后通道）
        let amount = self.paid_amount(order_id).await?;
        if amount <= Decimal::ZERO {
            return Err(Error::biz(400, "订单无可退金额"));
        }
        if self.refundable_amount(order_id).await? < amount {
            return Err(Error::biz(
                400,
                "订单已有退款或售后处理中，请通过售后通道处理",
            ));
        }

        let reason = reason
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("买家取消订单");
        if reason.chars().count() > 255 {
            return Err(Error::Validation("取消原因不能超过255字".into()));
        }
        let now = utils::now_seconds() as i32;
        let mut apply = RefundApply {
            refund_id: 0,
            refund_type: RefundType::Order as i32,
            order_id,
            user_id,
            aftersale_id: 0,
            refund_status: RefundApplyStatus::Wait as i32,
            add_time: now,
            refund_note: reason.to_string(),
            online_balance: amount,
            offline_balance: Decimal::ZERO,
            refund_balance: amount,
            is_online: 1,
            is_offline: 0,
            is_receive: 0,
            paylog_refund_id: 0,
            shop_id: order.shop_id,
            payment_voucher: None,
            initiator: i16::try_from(RefundInitiator::BuyerCancel as i32)
                .unwrap_or_default(),
        };
        let refund_id = self.repos.payment.refund_apply.insert(&apply).await?;
        apply.refund_id = refund_id;
        let _ = self
            .add_log(
                refund_id,
                order_id,
                RefundType::Order as i32,
                "",
                "",
                amount,
                format!("买家取消订单发起退款：{reason}"),
                user_id,
            )
            .await;
        // 支付状态置「退款中」（条件更新，仅当仍为已支付）
        self.set_order_pay_status(order_id, 2, 3).await;
        tracing::info!(
            "买家取消直退发起: id={refund_id} order_id={order_id} amount={amount}"
        );
        self.notify_buyer_cancel_submitted(&order, amount).await;

        match settings.route_for(amount) {
            crate::system::paid_cancel_refund::PaidCancelRoute::FinanceAudit => {
                tracing::info!("买家取消直退进财务审核队列: id={refund_id}");
            }
            route => {
                let exempt =
                    route == crate::system::paid_cancel_refund::PaidCancelRoute::Exempt;
                self.execute_buyer_cancel_now(&apply, refund_id, exempt)
                    .await;
            }
        }
        Ok(refund_id)
    }

    /// 买家取消直退的立即执行（小额免审 / 不进审核队列两条直达
    /// 路径，与主动退款自动执行 `auto_execute_proactive` 同构）：
    /// 资金执行 → Wait 流转 → 分销佣金回冲 → 完成点钩子
    /// （礼品卡按额返还 + 已结算扣回 + 订单关闭）。
    async fn execute_buyer_cancel_now(
        &self,
        apply: &RefundApply,
        refund_id: i32,
        exempt: bool,
    ) {
        if exempt {
            let _ = self
                .add_log(
                    refund_id,
                    apply.order_id,
                    apply.refund_type,
                    "",
                    "",
                    apply.refund_balance,
                    format!(
                        "小额免审自动通过（退款额 ¥{} 在免审上限内，系统免审）",
                        apply.refund_balance
                    ),
                    apply.user_id,
                )
                .await;
        }
        let note = if exempt {
            "小额免审自动执行退款"
        } else {
            "买家取消退款自动执行"
        };
        let (to_status, completed, card_part) =
            match self.execute_refund_money(apply, refund_id).await {
                Ok((MoneyOutcome::Completed, _, part)) => {
                    (RefundApplyStatus::Processed as i32, true, part)
                }
                Ok((MoneyOutcome::ChannelPending, _, _)) => {
                    (RefundApplyStatus::Processing as i32, false, Decimal::ZERO)
                }
                Err(e) => {
                    tracing::warn!(
                        "买家取消直退 资金执行失败 refund_id={refund_id} order_id={}: {e}",
                        apply.order_id
                    );
                    (RefundApplyStatus::Processing as i32, false, Decimal::ZERO)
                }
            };
        let transitioned = self
            .repos
            .payment
            .refund_apply
            .transition_status(
                refund_id,
                &[RefundApplyStatus::Wait as i32],
                to_status,
                Some(note),
            )
            .await
            .unwrap_or(false);
        if !transitioned {
            return;
        }
        // 分销佣金回冲（与财务审核路径同点，尽力而为）：买家取消无
        // 退货商品项，handler 按 aftersale_id=0 整单口径回冲。
        if let Err(e) = self
            .handle_salesman_order_on_refund(apply.order_id, 0)
            .await
        {
            tracing::warn!(
                "买家取消直退 分销佣金回退失败 refund_id={refund_id} order_id={}: {e}",
                apply.order_id
            );
        }
        if completed {
            // 礼品卡返还（与 mark_processed 同为退款完成点）：按
            // 资金执行的本笔卡构成按额返还（子单精确化口径）。
            if apply.order_id > 0 {
                if let Err(e) = self
                    .repos
                    .marketing
                    .gift_card_issue
                    .return_stored_cards_amount(
                        apply.order_id,
                        card_part,
                        utils::now_seconds(),
                    )
                    .await
                {
                    tracing::warn!(
                        "买家取消直退 礼品卡返还失败 refund_id={refund_id} order_id={}: {e}",
                        apply.order_id
                    );
                }
            }
            self.settle_proactive_refund(refund_id, apply).await;
            self.close_order_for_buyer_cancel(apply.order_id).await;
        }
    }

    /// 买家取消退款完成后的订单关闭（完成点唯一收口：立即执行与
    /// 财务审核两条路径的完成都经此，含渠道受理后经 `mark_processed`
    /// 收尾的情形）：待发货 → 已取消（原子流转，成功才做后续收尾，
    /// 重复调用天然幂等）、支付状态置已退款、回补库存、释放优惠券
    /// 占用、买家站内信。
    ///
    /// 收尾口径与既有链对齐，不另定：库存回补同买家取消未支付单
    /// （`OrderService::cancel`）；优惠券释放同售后完成/后台取消
    /// （`release_by_order` + 父单兜底）；积分按已付退款既定口径
    /// 不返还（售后与主动退款完成链均无积分返还，本服务模块头注
    /// 亦载明积分退还未在退款链迁移）。
    pub async fn close_order_for_buyer_cancel(&self, order_id: i32) {
        if order_id <= 0 {
            return;
        }
        let ok = self
            .repos
            .trade
            .order
            .transition_status(
                order_id,
                &[crate::trade::order::status::PAID],
                crate::trade::order::status::CANCELED,
            )
            .await
            .unwrap_or(false);
        if !ok {
            return;
        }
        self.set_order_pay_status(order_id, 3, 4).await;
        // 取消备注（仅当为空时写，不覆盖已有备注）
        let _ = sqlx::query(
            "UPDATE \"order\" SET cancel_remark = '买家取消并退款' \
             WHERE order_id = $1 AND (cancel_remark IS NULL OR cancel_remark = '')",
        )
        .bind(order_id)
        .execute(&self.repos.pool)
        .await;
        // 回补库存（订单从未发货，库存与下单扣减对冲）
        if let Ok(items) = self.repos.trade.order.find_items(order_id).await {
            for item in items {
                let sku = if item.sku_id > 0 {
                    Some(item.sku_id)
                } else {
                    None
                };
                if let Err(e) = self
                    .repos
                    .catalog
                    .product
                    .restore_stock(item.product_id, sku, item.quantity)
                    .await
                {
                    tracing::warn!(
                        "买家取消关单 回补库存失败 order={order_id} product={}: {e}",
                        item.product_id
                    );
                }
            }
        }
        if let Ok(Some(order)) = self.repos.trade.order.find_by_id(order_id).await {
            // 优惠券释放（本单释放为空时兜底父单，同售后完成口径）
            if order.coupon_amount > Decimal::ZERO {
                match self
                    .repos
                    .marketing
                    .coupon
                    .release_by_order(order_id)
                    .await
                {
                    Ok(0) => {
                        if let Some(parent_id) =
                            order.parent_order_id.filter(|v| *v > 0)
                        {
                            let _ = self
                                .repos
                                .marketing
                                .coupon
                                .release_by_order(parent_id)
                                .await;
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!(
                            "买家取消关单 优惠券释放失败 order={order_id}: {e}"
                        );
                    }
                }
            }
            if order.user_id > 0 {
                let content = format!(
                    "您的订单 {} 已取消，退款已原路退回，请注意查收。",
                    order.order_sn
                );
                if let Err(e) = self
                    .repos
                    .member
                    .user_message
                    .insert_user_message(
                        order.user_id,
                        "订单取消退款完成",
                        &content,
                        1,
                        None,
                    )
                    .await
                {
                    tracing::warn!(
                        "买家取消关单 买家通知失败 order={order_id}: {e}"
                    );
                }
            }
        }
        tracing::info!("买家取消退款完成，订单已关闭: {order_id}");
    }

    /// 订单支付状态条件迁移（买家取消直退专用：2 已支付 → 3 退款中
    /// → 4 已退款；被拒/退回时 3 → 2 恢复）。失败只告警不阻断主流程。
    async fn set_order_pay_status(&self, order_id: i32, from: i32, to: i32) {
        let res = sqlx::query(
            "UPDATE \"order\" SET pay_status = $1 WHERE order_id = $2 AND pay_status = $3",
        )
        .bind(to)
        .bind(order_id)
        .bind(from)
        .execute(&self.repos.pool)
        .await;
        if let Err(e) = res {
            tracing::warn!(
                "买家取消直退 更新订单支付状态失败 order={order_id} {from}->{to}: {e}"
            );
        }
    }

    /// 买家取消申请提交后的站内信（best-effort）。
    async fn notify_buyer_cancel_submitted(
        &self,
        order: &nextrade_core::entities::Order,
        amount: Decimal,
    ) {
        if order.user_id <= 0 {
            return;
        }
        let content = format!(
            "您已申请取消订单 {} 并退款，退款金额 ¥{}。审核通过后将原路退回，请注意查收。",
            order.order_sn, amount
        );
        if let Err(e) = self
            .repos
            .member
            .user_message
            .insert_user_message(order.user_id, "取消退款申请已提交", &content, 1, None)
            .await
        {
            tracing::warn!(
                "买家取消直退 买家通知失败 order_id={} user={}: {e}",
                order.order_id, order.user_id
            );
        }
    }

    /// 更新退款申请金额/备注（仅待处理状态可改）。
    pub async fn update(&self, dto: &RefundApplyUpdateDTO) -> Result<()> {
        let existing = self
            .repos.payment.refund_apply
            .find_by_id(dto.refund_id)
            .await?
            .ok_or_else(|| Error::NotFound("退款申请不存在".into()))?;

        if RefundApplyStatus::try_from(existing.refund_status)
            .map(|s| s as i32)
            != Ok(RefundApplyStatus::Wait as i32)
        {
            return Err(Error::biz(400, "仅待处理的申请允许修改"));
        }

        let online = dto.online_balance.unwrap_or(existing.online_balance);
        let offline = dto.offline_balance.unwrap_or(existing.offline_balance);
        let total = total_refund(online, offline);
        if total <= Decimal::ZERO {
            return Err(Error::Validation("退款金额必须大于0".into()));
        }

        // 差额校验：新总额不能超过「可退额度 + 原申请占用」
        let occupied = self
            .repos.payment.refund_apply
            .sum_occupied_by_order(existing.order_id)
            .await?;
        let paid = self.paid_amount(existing.order_id).await?;
        let refundable = (paid - (occupied - existing.refund_balance)).max(Decimal::ZERO);
        if total > refundable {
            return Err(Error::biz(
                400,
                format!("退款金额超出可退余额，本笔最多可退 {refundable}"),
            ));
        }

        let mut updated = existing.clone();
        updated.online_balance = online;
        updated.offline_balance = offline;
        updated.refund_balance = total;
        updated.is_online = i32::from(online > Decimal::ZERO);
        updated.is_offline = i32::from(offline > Decimal::ZERO);
        if let Some(note) = &dto.refund_note {
            updated.refund_note = note.clone();
        }
        self.repos.payment.refund_apply.update_amounts(&updated).await
    }

    // ————————————————————— 审核 —————————————————————

    /// 财务审核。
    ///
    /// 对应 Java `audit`：
    /// - 目标状态必须是 1（处理中）/ 3（退回售后）/ 4（拒绝）
    /// - 拒绝(4)：原因必填（trim 后非空、≤255），且**仅当当前为待处理(0)**时可拒绝，
    ///   用条件更新防并发；成功后不产生任何资金副作用
    /// - 3（退回售后）：标记取消，释放占用额度
    /// - 1（处理中）：进入退款流程
    pub async fn audit(&self, dto: &RefundApplyAuditDTO) -> Result<()> {
        let target = RefundApplyStatus::try_from(dto.refund_status)
            .map_err(|_| Error::biz(400, "审核状态值错误"))?;

        // Java 只允许这三种目标状态
        if !matches!(
            target,
            RefundApplyStatus::Processing
                | RefundApplyStatus::Cancel
                | RefundApplyStatus::Rejected
        ) {
            return Err(Error::biz(400, "审核状态值错误"));
        }

        let existing = self
            .repos.payment.refund_apply
            .find_by_id(dto.refund_id)
            .await?
            .ok_or_else(|| Error::biz(400, "该申请不存在"))?;

        // 仅待处理状态可被审核（对应 Java Assert refundStatus == 0）
        if RefundApplyStatus::try_from(existing.refund_status)
            .map(|s| s as i32)
            != Ok(RefundApplyStatus::Wait as i32)
        {
            return Err(Error::biz(400, "申请状态值错误"));
        }

        match target {
            RefundApplyStatus::Rejected => {
                let reason = validate_reject_reason(dto.refund_note.as_deref())
                    .map_err(Error::Validation)?;
                let ok = self
                    .repos.payment.refund_apply
                    .transition_status(
                        dto.refund_id,
                        &[RefundApplyStatus::Wait as i32],
                        RefundApplyStatus::Rejected as i32,
                        Some(&reason),
                    )
                    .await?;
                if !ok {
                    return Err(Error::biz(400, "退款申请已处理，请刷新后重试"));
                }
                self.add_log(
                    dto.refund_id,
                    existing.order_id,
                    existing.refund_type,
                    "",
                    "",
                    Decimal::ZERO,
                    format!("财务审核拒绝：{reason}"),
                    existing.user_id,
                )
                .await?;
                // 买家取消来源被拒：退款不发生，订单继续履约——
                // 支付状态从「退款中」恢复「已支付」，发货拦截随
                // 申请终态自动解除。
                if existing.initiator == RefundInitiator::BuyerCancel as i16 {
                    self.set_order_pay_status(existing.order_id, 3, 2).await;
                }
                tracing::info!("退款申请已拒绝: id={} 原因={}", dto.refund_id, reason);
            }
            RefundApplyStatus::Cancel => {
                // 退回售后人员重新处理：释放占用额度（状态不在 occupies_quota 集合内）
                let ok = self
                    .repos.payment.refund_apply
                    .transition_status(
                        dto.refund_id,
                        &[RefundApplyStatus::Wait as i32],
                        RefundApplyStatus::Cancel as i32,
                        dto.refund_note.as_deref(),
                    )
                    .await?;
                if !ok {
                    return Err(Error::biz(400, "退款申请已处理，请刷新后重试"));
                }
                self.add_log(
                    dto.refund_id,
                    existing.order_id,
                    existing.refund_type,
                    "",
                    "",
                    Decimal::ZERO,
                    "财务审核退回售后人员重新处理".to_string(),
                    existing.user_id,
                )
                .await?;
                // 买家取消来源被退回：同拒绝口径，订单恢复「已支付」
                // 继续履约（买家可改走售后通道）。
                if existing.initiator == RefundInitiator::BuyerCancel as i16 {
                    self.set_order_pay_status(existing.order_id, 3, 2).await;
                }
                tracing::info!("退款申请退回售后: id={}", dto.refund_id);
            }
            RefundApplyStatus::Processing => {
                // 进入退款流程前再次校验额度（防止排队期间被其他申请占满）
                self.assert_refundable(existing.order_id, existing.refund_balance)
                    .await?;
                let ok = self
                    .repos.payment.refund_apply
                    .transition_status(
                        dto.refund_id,
                        &[RefundApplyStatus::Wait as i32],
                        RefundApplyStatus::Processing as i32,
                        dto.refund_note.as_deref(),
                    )
                    .await?;
                if !ok {
                    return Err(Error::biz(400, "退款申请已处理，请刷新后重试"));
                }
                self.add_log(
                    dto.refund_id,
                    existing.order_id,
                    existing.refund_type,
                    "",
                    "",
                    existing.refund_balance,
                    "财务审核通过，进入退款流程".to_string(),
                    existing.user_id,
                )
                .await?;
                tracing::info!("退款申请进入处理中: id={}", dto.refund_id);
                // 分销佣金回退（尽力而为）：财务审核通过即触发（Java 在
                // audit 审核通过段调用 handleSalesmanOrderOnRefund，与
                // 退款流水创建同点）；失败只告警，不阻断退款主流程
                // （状态流转已落库，申请已进入处理中不会再被重复审核）。
                if existing.order_id > 0 {
                    if let Err(e) = self
                        .handle_salesman_order_on_refund(
                            existing.order_id,
                            existing.aftersale_id,
                        )
                        .await
                    {
                        tracing::warn!(
                            "财务审核通过 分销佣金回退失败 refund_id={} order_id={}: {e}",
                            dto.refund_id, existing.order_id
                        );
                    }
                }
                // 资金执行（与售后自动执行共用同一核心
                // `execute_refund_money`）：现金部分按订单支付构成
                // 选轨——余额支付同步回补买家余额、渠道支付发起渠道
                // 退款；礼品卡抵扣部分由完成点钩子按本笔卡构成返还。
                // 同步执行完成 → mark_processed 置已处理（内含礼品卡
                // 返还钩子）；渠道受理中/无自动退款轨 → 停在处理中，
                // 等待渠道回调或人工收尾（钱未动前不置已处理）。
                if existing.online_balance > rust_decimal::Decimal::ZERO {
                    let (outcome, refund_sn, card_part) = self
                        .execute_refund_money(&existing, dto.refund_id)
                        .await?;
                    if outcome == MoneyOutcome::Completed {
                        self.mark_processed(
                            dto.refund_id,
                            refund_sn.as_deref().unwrap_or_default(),
                            card_part,
                        )
                        .await?;
                    }
                }

            }
            // 其余状态不可作为审核目标
            _ => return Err(Error::biz(400, "审核状态值错误")),
        }
        Ok(())
    }

    /// 确认线下转账（上传支付凭证 + 标记已收款）。
    ///
    /// 对应 Java `offlineAudit`：线下退款需人工确认到账并留痕凭证。
    pub async fn offline_audit(&self, dto: &OfflineAuditDTO) -> Result<()> {
        let existing = self
            .repos.payment.refund_apply
            .find_by_id(dto.refund_id)
            .await?
            .ok_or_else(|| Error::NotFound("退款申请不存在".into()))?;

        if existing.is_offline != 1 {
            return Err(Error::biz(400, "该申请非线下退款，无需确认转账"));
        }

        if let Some(v) = &dto.payment_voucher {
            self.repos.payment.refund_apply
                .update_voucher(dto.refund_id, v)
                .await?;
        }

        let ok = self.repos.payment.refund_apply.confirm_receive(dto.refund_id).await?;
        if !ok {
            return Err(Error::biz(400, "已确认收款，无需重复操作"));
        }

        self.add_log(
            dto.refund_id,
            existing.order_id,
            existing.refund_type,
            "offline",
            "",
            existing.offline_balance,
            "确认线下转账到账".to_string(),
            existing.user_id,
        )
        .await?;
        tracing::info!(
            "确认线下转账: refund_id={} amount={}",
            dto.refund_id,
            existing.offline_balance
        );
        // 礼品卡返还（尽力而为）：线下退款以确认到账为完成点，口径同
        // mark_processed；仓储层幂等，重复确认/重复调用不会重复返还。
        if existing.order_id > 0 {
            if let Err(e) = self
                .repos
                .marketing
                .gift_card_issue
                .return_stored_cards_for_order(existing.order_id, utils::now_seconds())
                .await
            {
                tracing::warn!(
                    "线下退款确认 礼品卡返还失败 refund_id={} order_id={}: {e}",
                    dto.refund_id, existing.order_id
                );
            }
        }
        Ok(())
    }

    /// 标记退款完成（渠道扣款成功后调用）。
    ///
    /// `card_part` = 本笔退款的礼品卡构成，必须由调用方从
    /// [`Self::execute_refund_money`] 的返回值贯穿传入（总额口径
    /// 申请按此额返还到卡）。手工申请（非总额口径）不经资金
    /// 执行拆分，完成点仍按归属剩余份额全额返还（历史口径）。
    /// 未来新增完成点调用方（如渠道回调收尾）若无法经 execute
    /// 取得 card_part，须按「含本笔的累计比例公式」自行补算
    /// （见 [`gift_card_refund_part`]），不得以 0 冒充——0 只对
    /// 「本笔卡构成确为 0」成立。
    pub async fn mark_processed(
        &self,
        refund_id: i32,
        transaction_id: &str,
        card_part: Decimal,
    ) -> Result<()> {
        let ok = self
            .repos.payment.refund_apply
            .transition_status(
                refund_id,
                &[RefundApplyStatus::Processing as i32],
                RefundApplyStatus::Processed as i32,
                None,
            )
            .await?;
        if !ok {
            return Err(Error::biz(400, "退款申请已处理，请刷新后重试"));
        }

        let existing = self
            .repos.payment.refund_apply
            .find_by_id(refund_id)
            .await?
            .unwrap_or_default();
        self.add_log(
            refund_id,
            existing.order_id,
            existing.refund_type,
            "",
            transaction_id,
            existing.refund_balance,
            format!("退款完成，交易号 {transaction_id}"),
            existing.user_id,
        )
        .await?;
        tracing::info!("退款完成: refund_id={refund_id} trade_no={transaction_id}");
        // 礼品卡返还（尽力而为）：退款完成後返还储值卡抵扣并写
        // type=2 流水，失败只告警、不回滚已完成的退款（与订单
        // 取消返还口径一致）。总额口径申请（售后/主动退款）按
        // 资金执行贯穿的本笔卡构成按额返还（子单/部分退款精确
        // 化口径，仓储层以归属份额剩余额度钳制防重）；手工申请
        // 沿用全额返还口径（其 online_balance 本就是财务填写的
        // 现金部分，卡腿不经拆分）。
        if existing.order_id > 0 {
            let returned = if existing.aftersale_id > 0 || existing.initiator > 0 {
                self.repos
                    .marketing
                    .gift_card_issue
                    .return_stored_cards_amount(
                        existing.order_id,
                        card_part,
                        utils::now_seconds(),
                    )
                    .await
            } else {
                self.repos
                    .marketing
                    .gift_card_issue
                    .return_stored_cards_for_order(existing.order_id, utils::now_seconds())
                    .await
            };
            if let Err(e) = returned {
                tracing::warn!(
                    "退款完成 礼品卡返还失败 refund_id={refund_id} order_id={}: {e}",
                    existing.order_id
                );
            }
        }
        // 主动退款（initiator > 0）完成收尾：已结算订单从商家结算
        // 款扣回（best-effort）。售后来源申请的同等收尾由售后域
        // on_refund_completed 承担，两条路径互不重复触发。
        if existing.initiator > 0 {
            self.settle_proactive_refund(refund_id, &existing).await;
        }
        // 买家取消来源（initiator = 3）：退款完成即关闭订单
        // （待发货 → 已取消 + 库存回补 + 券释放 + 置已退款），
        // 收口在 close_order_for_buyer_cancel（原子流转，幂等）。
        if existing.initiator == RefundInitiator::BuyerCancel as i16 {
            self.close_order_for_buyer_cancel(existing.order_id).await;
        }
        Ok(())
    }

    // ————————————————————— 资金执行 —————————————————————

    /// 执行一笔已审核通过申请的现金部分退款（财务审核与售后自动
    /// 执行共用；状态流转与完成点钩子由调用方负责）。
    ///
    /// 金额拆分（对齐 Java `createRefundFlow` 按订单支付构成执行）：
    /// - 礼品卡抵扣部分不走支付轨，由完成点按额返还到卡
    ///   （`return_stored_cards_amount`）。总额口径来源（售后 /
    ///   主动退款）的 `online_balance` 是退款总额（建单时写入
    ///   落定金额），现金部分须扣减**本笔退款的卡构成**——按
    ///   归属份额与累计退款比例算出（纯函数
    ///   [`gift_card_refund_part`]，子单/部分退款精确化口径；
    ///   单笔全额退款时恒等于整单抵扣额，与历史口径一致），
    ///   再经纯函数 [`cash_refund_amount`] 扣减；手工申请的
    ///   `online_balance` 本就是财务填写的线上部分，不扣减。
    /// - 现金部分按订单最后一笔成功支付流水的 `pay_code` 选轨：
    ///   `balance` → 同步回补买家余额（写 user_balance_log
    ///   「退款到账」，Java `incBalance` 同口径）并累加 paylog
    ///   退款额，执行即完成；已实现渠道 → 建退款流水 + 调渠道
    ///   退款，同步 SUCCESS 才算完成，受理/处理中为
    ///   `ChannelPending`；无支付流水或未支持的支付方式 →
    ///   `ChannelPending`（停处理中等待人工处理，不静默置已处理）。
    ///
    /// 返回 (结果, 退款单号, 本笔礼品卡构成)：渠道与余额两轨都
    /// 生成 `RF...` 单号，供完成日志引用；卡构成由完成点钩子
    /// 按额返还（仅同步完成时生效，渠道受理中不提前返还）。
    pub async fn execute_refund_money(
        &self,
        existing: &RefundApply,
        refund_id: i32,
    ) -> Result<(MoneyOutcome, Option<String>, Decimal)> {
        if existing.order_id <= 0 || existing.online_balance <= Decimal::ZERO {
            return Ok((MoneyOutcome::Completed, None, Decimal::ZERO));
        }
        let total_caliber = existing.aftersale_id > 0 || existing.initiator > 0;
        // 本笔退款的礼品卡构成（子单/部分退款精确化）：归属份额
        // （子单按拆单分摊、未拆单按流水总额）乘以累计退款比例，
        // 尾差归本单最后一笔；手工申请不走此口径（恒 0）。
        let mut card_part = Decimal::ZERO;
        if total_caliber {
            let share = self
                .repos
                .marketing
                .gift_card_issue
                .deducted_total_for_order(existing.order_id)
                .await?;
            if share > Decimal::ZERO {
                let already_returned = self
                    .repos
                    .marketing
                    .gift_card_issue
                    .returned_total_for_order(existing.order_id)
                    .await?;
                let base = self
                    .repos
                    .trade
                    .order_admin
                    .find_order(existing.order_id)
                    .await?
                    .map(|o| o.total_amount)
                    .unwrap_or(Decimal::ZERO);
                let prior_refunded = self
                    .repos
                    .payment
                    .refund_apply
                    .sum_completed_refund_total_by_order(existing.order_id, refund_id)
                    .await?;
                card_part = gift_card_refund_part(
                    share,
                    already_returned,
                    base,
                    prior_refunded,
                    existing.online_balance,
                );
            }
        }
        let cash = cash_refund_amount(existing.online_balance, card_part, total_caliber);
        if cash <= Decimal::ZERO {
            // 全额由礼品卡抵扣构成：无现金可退，完成点钩子返还卡即可
            return Ok((MoneyOutcome::Completed, None, card_part));
        }
        let Some(paylog) = self
            .repos
            .payment
            .pay
            .find_last_success_paylog(existing.order_id, 0)
            .await?
        else {
            tracing::warn!(
                "退款资金执行：订单 {} 无成功支付流水，申请 {refund_id} 停处理中等待人工处理",
                existing.order_id
            );
            return Ok((MoneyOutcome::ChannelPending, None, card_part));
        };
        let code = paylog.pay_code.as_str();
        if code == "balance" {
            self.credit_balance_refund(existing, &paylog, cash).await?;
            let refund_sn = format!("RF{}{}", utils::now_seconds(), refund_id);
            return Ok((MoneyOutcome::Completed, Some(refund_sn), card_part));
        }
        if matches!(code, "stripe" | "paypal" | "xendit" | "wechat" | "alipay" | "antom" | "huifu" | "lakala" | "yeepay" | "yabandpay" | "yunpay") {
            let refund_sn = format!("RF{}{}", utils::now_seconds(), refund_id);
            let paylog_service = crate::payment::paylog::PaylogService::new(self.repos.clone());
            let paylog_refund_id = paylog_service
                .record_refund(
                    paylog.paylog_id,
                    existing.order_id,
                    &refund_sn,
                    cash,
                    "退款审核通过",
                    0,
                    Some(code),
                )
                .await?;
            self.repos
                .payment
                .refund_apply
                .link_paylog_refund(refund_id, paylog_refund_id)
                .await?;
            let _ = sqlx::query("UPDATE refund_apply SET paylog_refund_id=$1 WHERE refund_id=$2")
                .bind(paylog_refund_id)
                .bind(refund_id)
                .execute(&self.repos.pool)
                .await;
            let pay = crate::payment::pay::PayService::new(self.repos.clone());
            // 拆单子单：渠道退款的订单总额取父单同 paySn 的 paylog
            // （原版 RefundApplyServiceImpl L965-975）；退款流水与占用
            // 仍记在子单自身 paylog 上（assertRefundable 口径不变）。
            let channel_paylog = match self
                .channel_total_paylog(existing.order_id, &paylog)
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!("退款资金执行：父单 paylog 解析失败 refund_id={refund_id}: {e}");
                    paylog.clone()
                }
            };
            match pay
                .refund_via_channel(&channel_paylog, &refund_sn, cash)
                .await
            {
                Ok(status) => {
                    let upper = status.to_ascii_uppercase();
                    if upper.contains("SUCCESS") || upper.contains("COMPLETED") {
                        return Ok((MoneyOutcome::Completed, Some(refund_sn), card_part));
                    }
                    tracing::info!("渠道退款已受理: refund_id={refund_id} status={status}");
                    return Ok((MoneyOutcome::ChannelPending, Some(refund_sn), card_part));
                }
                Err(e) => {
                    tracing::warn!("渠道退款失败: refund_id={refund_id} error={e}");
                    return Err(e);
                }
            }
        }
        tracing::warn!(
            "退款资金执行：支付方式 {code} 暂无自动退款轨，申请 {refund_id} 停处理中等待人工处理"
        );
        Ok((MoneyOutcome::ChannelPending, None, card_part))
    }

    /// 渠道退款的总额基准 paylog：拆单子单（parent_order_id > 0）时取
    /// 父单同 paySn 的 paylog（其 pay_amount 为整单实付）；非子单或父单
    /// 无对应记录时回退本单 paylog。对应原版 RefundApplyServiceImpl
    /// 在渠道退款时以 (parentOrderId, paySn) 查父单 paylog 的口径——
    /// 子单 paylog 只承载分摊额，不能作渠道总额。
    async fn channel_total_paylog(
        &self,
        order_id: i32,
        paylog: &nextrade_core::entities::Paylog,
    ) -> Result<nextrade_core::entities::Paylog> {
        let Some(order) = self.repos.trade.order_admin.find_order(order_id).await? else {
            return Ok(paylog.clone());
        };
        if let Some(parent_id) = order.parent_order_id.filter(|id| *id > 0) {
            if let Some(parent_log) = self
                .repos
                .payment
                .paylog
                .find_by_order_and_pay_sn(parent_id, &paylog.pay_sn)
                .await?
            {
                return Ok(parent_log);
            }
        }
        Ok(paylog.clone())
    }

    /// 余额支付部分的退款回补（Java `createRefundFlow` 余额分支 →
    /// `userBalanceService.incBalance(amount, userId, "退款到账")`）。
    ///
    /// 同一事务内：买家余额入账 + `user_balance_log`（复用充值域
    /// `inc_balance_in_tx`，即 Java incBalance 的等价实现）+ paylog
    /// 累加退款额。额度守卫与 `BalancePayService::refund` 同口径：
    /// 该笔余额支付流水的累计退款不得超过其实付额。
    async fn credit_balance_refund(
        &self,
        existing: &RefundApply,
        paylog: &nextrade_core::entities::Paylog,
        amount: Decimal,
    ) -> Result<()> {
        if paylog.refund_amount + amount > paylog.pay_amount {
            return Err(Error::biz(400, "退款金额超出余额支付金额"));
        }
        let now = utils::now_seconds() as i32;
        let mut tx = self
            .repos
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        let ok = self
            .repos
            .payment
            .user_recharge_order
            .inc_balance_in_tx(&mut tx, existing.user_id, amount, "退款到账", now)
            .await?;
        if !ok {
            return Err(Error::NotFound("退款用户不存在".into()));
        }
        sqlx::query(
            "UPDATE paylog SET refund_amount = refund_amount + $1 WHERE paylog_id = $2",
        )
        .bind(amount)
        .bind(paylog.paylog_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        tracing::info!(
            "余额退款到账: refund_id={} user={} amount={amount}",
            existing.refund_id,
            existing.user_id
        );
        Ok(())
    }

    // ————————————————————— 分销佣金回退 —————————————————————

    /// 分销佣金回退（Java `RefundApplyServiceImpl#handleSalesmanOrderOnRefund` 照迁）。
    ///
    /// 触发点与 Java 一致：财务审核通过（退款进入执行）。Java 在 `audit`
    /// 审核通过段与退款流水创建同事务调用；本仓为分步落库，挂点为：
    /// - 本服务 `audit` 的 Wait→Processing 流转成功后；
    /// - 售后域自动执行路径（`aftersales::create_refund_apply_and_handle_finance`
    ///   的流转成功后——Java 自动退款走同一 `audit` 方法，本仓自动路径
    ///   绕过服务层 `audit`，在该处同口径补挂）。
    ///
    /// 两处互斥（一笔退款申请只经其一），且都以状态流转成功为前提——
    /// **严禁再从其他完成点重复调用**：部分退款的金额按历史已退数量
    /// 反推原始金额，重复触发会把已扣减金额再次按比例扣减。
    ///
    /// 口径（逐条对照 Java）：
    /// - 按售后商品项精确匹配 `salesman_order`（order_id + item_id）；
    /// - 只处理 `status=0`（待结算）行；已结算（1）行原版**不动**——
    ///   不置失效、不扣钱包；已失效（2）行跳过；
    /// - 本次 + 历史已完成售后（`aftersales.status=6`，不含本次）的
    ///   累计退货数量 ≥ 购买数量 → 整行置 `status=2`（已失效）；
    /// - 未退满 → 按剩余数量占比扣减 `amount` / `order_amount`
    ///   （占比 4 位小数 HALF_UP，金额 2 位；状态保持 0）；
    /// - 不触碰 `salesman_customer` 与分销员钱包（Java 同）。
    ///
    /// 与 Java 的防御性差异（正常数据下为恒等）：本次售后同项多行先
    /// 按订单项聚合数量；购买数量缺失、历史占比反推分母为 0 时跳过
    /// 该项（Java 此处会除零异常）。
    pub async fn handle_salesman_order_on_refund(
        &self,
        order_id: i32,
        aftersale_id: i32,
    ) -> Result<()> {
        let pool = &self.repos.pool;
        // 本次售后商品项（按订单项聚合本次退货数量）
        let item_rows: Vec<(Option<i32>, Option<i32>)> = sqlx::query_as(
            "SELECT order_item_id, number FROM aftersales_item WHERE aftersale_id = $1",
        )
        .bind(aftersale_id)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        if item_rows.is_empty() {
            return Ok(());
        }
        let mut refund_qty_by_item: std::collections::HashMap<i32, i32> =
            std::collections::HashMap::new();
        for (order_item_id, number) in item_rows {
            if let Some(item_id) = order_item_id {
                *refund_qty_by_item.entry(item_id).or_insert(0) += number.unwrap_or(0);
            }
        }

        for (item_id, refund_qty) in refund_qty_by_item {
            // 该商品项的分销佣金行（一件商品可有两级分销两行）
            let rows: Vec<(i32, Option<i32>, Option<Decimal>, Option<Decimal>)> =
                sqlx::query_as(
                    "SELECT salesman_order_id, status, amount, order_amount \
                     FROM salesman_order WHERE order_id = $1 AND item_id = $2",
                )
                .bind(order_id)
                .bind(item_id)
                .fetch_all(pool)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
            if rows.is_empty() {
                continue;
            }
            // 原订单商品项购买数量（缺失则无从按比例处理，跳过该项）
            let buy_qty: Option<i32> =
                sqlx::query_scalar("SELECT quantity FROM order_item WHERE item_id = $1")
                    .bind(item_id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| Error::Database(e.to_string()))?;
            let Some(buy_qty) = buy_qty else {
                continue;
            };
            // 历史已完成售后（不含本次）对该商品项的累计退货数量
            let prev_qty: i64 = sqlx::query_scalar(
                "SELECT COALESCE(SUM(ai.number), 0) \
                 FROM aftersales_item ai \
                 JOIN aftersales a ON a.aftersale_id = ai.aftersale_id \
                 WHERE a.order_id = $1 AND a.status = 6 \
                   AND a.aftersale_id <> $2 AND ai.order_item_id = $3",
            )
            .bind(order_id)
            .bind(aftersale_id)
            .bind(item_id)
            .fetch_one(pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
            let prev_qty = i32::try_from(prev_qty).unwrap_or(i32::MAX);
            let total_qty = prev_qty.saturating_add(refund_qty);

            for (row_id, status, amount, order_amount) in rows {
                // 只处理待结算行（Java 同：status != 0 跳过，
                // 已结算行不置失效、不扣钱包）
                if status != Some(0) {
                    continue;
                }
                match salesman_clawback_decision(
                    buy_qty,
                    prev_qty,
                    total_qty,
                    amount.unwrap_or(Decimal::ZERO),
                    order_amount.unwrap_or(Decimal::ZERO),
                ) {
                    ClawbackDecision::Invalidate => {
                        let n = sqlx::query(
                            "UPDATE salesman_order SET status = 2 \
                             WHERE salesman_order_id = $1 AND status = 0",
                        )
                        .bind(row_id)
                        .execute(pool)
                        .await
                        .map_err(|e| Error::Database(e.to_string()))?
                        .rows_affected();
                        if n > 0 {
                            tracing::info!(
                                "分销佣金回退：订单 {order_id} 商品项 {item_id} 已全部退货，佣金行 {row_id} 置已失效"
                            );
                        }
                    }
                    ClawbackDecision::Rescale {
                        amount: new_amount,
                        order_amount: new_order_amount,
                    } => {
                        let n = sqlx::query(
                            "UPDATE salesman_order SET amount = $1, order_amount = $2 \
                             WHERE salesman_order_id = $3 AND status = 0",
                        )
                        .bind(new_amount)
                        .bind(new_order_amount)
                        .bind(row_id)
                        .execute(pool)
                        .await
                        .map_err(|e| Error::Database(e.to_string()))?
                        .rows_affected();
                        if n > 0 {
                            tracing::info!(
                                "分销佣金回退：订单 {order_id} 商品项 {item_id} 部分退货，佣金行 {row_id} 金额扣减为 {new_amount}（订单金额 {new_order_amount}）"
                            );
                        }
                    }
                    ClawbackDecision::Skip => {}
                }
            }
        }
        Ok(())
    }

    // ————————————————————— 额度校验 —————————————————————

    /// 校验订单在给定退款金额下是否可退。
    ///
    /// 对应 Java `assertRefundable`：
    /// - 可退额度 ≤ 0 → 「该笔支付已全额退款，无需重复发起」
    /// - 本次金额 > 可退额度 → 「退款金额超出可退余额，本笔最多可退 X」
    pub async fn assert_refundable(&self, order_id: i32, refund_amount: Decimal) -> Result<()> {
        let paid = self.paid_amount(order_id).await?;
        let occupied = self
            .repos.payment.refund_apply
            .sum_occupied_by_order(order_id)
            .await?;
        let refundable = (paid - occupied).max(Decimal::ZERO);

        if refundable <= Decimal::ZERO {
            tracing::warn!(
                "订单 {} 已无可退余额（实付 {}，已占用 {}）",
                order_id,
                paid,
                occupied
            );
            return Err(Error::biz(400, "该笔支付已全额退款，无需重复发起"));
        }
        if refund_amount > refundable {
            return Err(Error::biz(
                400,
                format!("退款金额超出可退余额，本笔最多可退 {refundable}"),
            ));
        }
        Ok(())
    }

    /// 订单可退余额。
    pub async fn refundable_amount(&self, order_id: i32) -> Result<Decimal> {
        let paid = self.paid_amount(order_id).await?;
        let occupied = self
            .repos.payment.refund_apply
            .sum_occupied_by_order(order_id)
            .await?;
        Ok((paid - occupied).max(Decimal::ZERO))
    }

    // ————————————————————— 内部 —————————————————————

    /// 取订单实付金额（从已支付的 `paylog` 取）。
    ///
    /// **必须限定 `order_type = 0`（商品订单）** —— `paylog` 三类业务共表，
    /// `order_id` 会撞号。
    async fn paid_amount(&self, order_id: i32) -> Result<Decimal> {
        match self
            .repos.payment.paylog
            .find_last_by_order(order_id, ORDER_TYPE_PRODUCT)
            .await?
        {
            Some(p) if p.pay_status == 1 => Ok(p.pay_amount),
            // 无已支付记录 → 可退额度为 0
            _ => Ok(Decimal::ZERO),
        }
    }

    /// 写退款操作日志。
    async fn add_log(
        &self,
        refund_apply_id: i32,
        order_id: i32,
        refund_type: i32,
        pay_code: &str,
        transaction_id: &str,
        amount: Decimal,
        description: String,
        user_id: i32,
    ) -> Result<()> {
        let log = RefundLog {
            log_id: 0,
            refund_apply_id,
            order_id,
            refund_type,
            refund_pay_code: pay_code.to_string(),
            transaction_id: transaction_id.to_string(),
            refund_amount: amount,
            add_time: utils::now_seconds() as i32,
            description: Some(description),
            user_id,
        };
        self.repos.payment.refund_apply.insert_log(&log).await?;
        Ok(())
    }
}

/// 现金退款部分拆分（纯函数，供单测）。
///
/// 总额口径来源（售后 / 主动退款）的 `online_balance` 为退款
/// 总额：扣减**本笔退款的礼品卡构成**（[`gift_card_refund_part`]
/// 按归属份额与累计比例算出）后才是走支付轨的现金部分（卡
/// 部分由完成点钩子按同一构成额返还到卡）。手工申请的
/// `online_balance` 本就是线上部分，不扣减。
pub(crate) fn cash_refund_amount(
    online_balance: Decimal,
    gift_card_part: Decimal,
    is_total_caliber: bool,
) -> Decimal {
    if is_total_caliber {
        (online_balance - gift_card_part).max(Decimal::ZERO)
    } else {
        online_balance.max(Decimal::ZERO)
    }
}

/// 本笔退款的礼品卡构成（纯函数，供单测）——子单/部分退款
/// 精确化口径：
///
/// ```text
/// 本笔卡构成 = round2(归属份额 × min(此前已完成退款累计 + 本笔退款额, 可退基数) / 可退基数) − 已返还卡额
/// ```
///
/// - `share`：归属份额（子单 = 拆单分摊额，未拆单 = 流水抵扣
///   总额，仓储 `deducted_total_for_order` 口径）；
/// - `base`：可退基数 = 订单 `total_amount`（退款额的度量基，
///   与原版「退款累计与订单总额比对」同口径）；
/// - 结果钳制于 [0, share − already_returned]：单笔全额退款
///   （累计达到基数）时尾差归一、恒等于剩余额度，与历史整单
///   口径逐分一致；分次退款时卡随退款进度同比例返还，Σ 恒
///   等于份额；基数 ≤ 0（异常数据）时返回 0，不产生卡返还。
pub(crate) fn gift_card_refund_part(
    share: Decimal,
    already_returned: Decimal,
    base: Decimal,
    prior_refunded: Decimal,
    this_refund: Decimal,
) -> Decimal {
    if share <= Decimal::ZERO || base <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let remaining = (share - already_returned).max(Decimal::ZERO);
    if remaining <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let cumulative = (prior_refunded + this_refund).min(base).max(Decimal::ZERO);
    let target = (share * cumulative / base)
        .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
    (target - already_returned).max(Decimal::ZERO).min(remaining)
}

/// 主动退款的订单状态门（纯函数，供单测）：返回 `Some(拒绝原因)`
/// 表示不可发起。退款单位必须是有效订单（拆单后父单为支付壳
/// `is_valid=0`，退款落到子单）；订单状态限待发货（1）/ 已发货
/// （2）/ 已完成（5，一期主场景）；支付状态限已支付（2）/ 退款中
/// （3，部分退后仍可继续退，额度由 assert_refundable 兜底）。
pub(crate) fn proactive_gate_error(
    order_status: i32,
    pay_status: i32,
    is_valid: i32,
) -> Option<&'static str> {
    if is_valid != 1 {
        return Some("该订单为拆单父单，请对子订单发起退款");
    }
    match order_status {
        1 | 2 | 5 => {}
        0 => return Some("订单尚未支付，无法发起退款"),
        3 => return Some("订单已取消，无法发起退款"),
        4 => return Some("订单已失效，无法发起退款"),
        _ => return Some("当前订单状态不支持主动退款"),
    }
    match pay_status {
        2 | 3 => None,
        _ => Some("订单未支付，无法发起退款"),
    }
}

/// 买家取消直退的状态门（纯函数，供单测）：仅「已支付 + 未发货」
/// 的有效（子）订单可走本通道。
///
/// - 拆单父单（`is_valid != 1`）拒绝，退款单位是（子）订单；
/// - 已部分发货（`shipping_status != 0`）拒绝——以（子）单为单位
///   判定「未发货」，部分发货单仍走售后；
/// - 拍卖单（11）与买家取消未支付单同口径拒绝；拼团单（2）的
///   成团/解散口径走售后与结算既有链，本通道不接；
/// - 支付状态须恰为 2（已支付）：3（退款中）说明已有退款在途，
///   与「整单全额退」语义冲突。
#[must_use]
pub(crate) fn buyer_cancel_gate_error(
    order_status: i32,
    pay_status: i32,
    shipping_status: i32,
    is_valid: i32,
    order_type: Option<i32>,
) -> Option<&'static str> {
    if is_valid != 1 {
        return Some("该订单为拆单父单，请对子订单操作");
    }
    match order_type {
        Some(11) => return Some("拍卖订单不可取消"),
        Some(2) => return Some("拼团订单请通过售后处理"),
        _ => {}
    }
    match order_status {
        1 => {}
        0 => return Some("订单尚未支付，请直接取消订单"),
        2 => return Some("订单已发货，请通过售后处理"),
        3 => return Some("订单已取消"),
        4 => return Some("订单已失效"),
        5 => return Some("订单已完成，请通过售后处理"),
        _ => return Some("当前订单状态不支持取消退款"),
    }
    if shipping_status != 0 {
        return Some("订单已部分发货，请通过售后处理");
    }
    if pay_status != 2 {
        return Some("订单支付状态不支持取消退款");
    }
    None
}

/// 分销佣金回退的单行决策（纯函数，供单测；金额数学对照 Java
/// `handleSalesmanOrderOnRefund`：占比 scale 4 HALF_UP、金额 2 位）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClawbackDecision {
    /// 累计退货数量已达购买数量：整行置已失效（status=2）。
    Invalidate,
    /// 部分退货：`amount` / `order_amount` 扣减后的新值（状态保持 0）。
    Rescale {
        amount: Decimal,
        order_amount: Decimal,
    },
    /// 不处理（购买数量异常等防御分支）。
    Skip,
}

/// 计算一行待结算佣金的回退决策。
///
/// - `buy_qty`：订单商品项购买数量；
/// - `prev_refund_qty`：历史已完成售后（不含本次）累计退货数量；
/// - `total_refund_qty`：prev + 本次退货数量；
/// - `amount` / `order_amount`：佣金行当前值（可能已被先前部分退款扣减）。
pub(crate) fn salesman_clawback_decision(
    buy_qty: i32,
    prev_refund_qty: i32,
    total_refund_qty: i32,
    amount: Decimal,
    order_amount: Decimal,
) -> ClawbackDecision {
    if buy_qty <= 0 {
        return ClawbackDecision::Skip;
    }
    if total_refund_qty >= buy_qty {
        return ClawbackDecision::Invalidate;
    }
    // 剩余有效数量占比（Java：divide(buyQuantity, 4, HALF_UP)）
    let ratio = (Decimal::from(buy_qty - total_refund_qty) / Decimal::from(buy_qty))
        .round_dp_with_strategy(4, RoundingStrategy::MidpointAwayFromZero);
    // 历史已部分退货时，行内金额已被扣减过：先按历史剩余占比反推
    // 原始金额（Java：divide(previousRatio, 4, HALF_UP)），再乘本次占比
    let (orig_amount, orig_order_amount) = if prev_refund_qty > 0 {
        let prev_ratio = (Decimal::from(buy_qty - prev_refund_qty) / Decimal::from(buy_qty))
            .round_dp_with_strategy(4, RoundingStrategy::MidpointAwayFromZero);
        if prev_ratio <= Decimal::ZERO {
            return ClawbackDecision::Skip;
        }
        (
            (amount / prev_ratio)
                .round_dp_with_strategy(4, RoundingStrategy::MidpointAwayFromZero),
            (order_amount / prev_ratio)
                .round_dp_with_strategy(4, RoundingStrategy::MidpointAwayFromZero),
        )
    } else {
        (amount, order_amount)
    };
    ClawbackDecision::Rescale {
        amount: (orig_amount * ratio)
            .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero),
        order_amount: (orig_order_amount * ratio)
            .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero),
    }
}

/// 实体 → VO。
fn to_vo(r: &RefundApply) -> RefundApplyVO {
    RefundApplyVO {
        refund_id: r.refund_id,
        refund_type: r.refund_type,
        refund_type_name: RefundType::try_from(r.refund_type)
            .map(|t| t.name().to_string())
            .unwrap_or_default(),
        order_id: r.order_id,
        user_id: r.user_id,
        aftersale_id: r.aftersale_id,
        refund_status: r.refund_status,
        refund_status_name: RefundApplyStatus::try_from(r.refund_status)
            .map(|s| s.name().to_string())
            .unwrap_or_default(),
        add_time: r.add_time,
        refund_note: r.refund_note.clone(),
        online_balance: r.online_balance,
        offline_balance: r.offline_balance,
        refund_balance: r.refund_balance,
        is_online: r.is_online,
        is_offline: r.is_offline,
        is_receive: r.is_receive,
        paylog_refund_id: r.paylog_refund_id,
        shop_id: r.shop_id,
        payment_voucher: r.payment_voucher.clone(),
        initiator: i32::from(r.initiator),
        initiator_name: RefundInitiator::name(i32::from(r.initiator), r.aftersale_id).to_string(),
    }
}

/// 日志实体 → VO。
fn to_log_vo(l: &RefundLog) -> RefundLogVO {
    RefundLogVO {
        log_id: l.log_id,
        refund_apply_id: l.refund_apply_id,
        refund_type: l.refund_type,
        refund_type_name: RefundType::try_from(l.refund_type)
            .map(|t| t.name().to_string())
            .unwrap_or_default(),
        refund_pay_code: l.refund_pay_code.clone(),
        transaction_id: l.transaction_id.clone(),
        refund_amount: l.refund_amount,
        add_time: l.add_time,
        description: l.description.clone(),
        user_id: l.user_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal as D;
    use std::str::FromStr;

    fn dec(s: &str) -> D {
        D::from_str(s).unwrap()
    }

    fn apply(status: i32, online: &str, offline: &str) -> RefundApply {
        let on = dec(online);
        let off = dec(offline);
        RefundApply {
            refund_id: 1,
            refund_type: 1,
            order_id: 100,
            user_id: 9,
            refund_status: status,
            online_balance: on,
            offline_balance: off,
            refund_balance: total_refund(on, off),
            is_online: i32::from(on > Decimal::ZERO),
            is_offline: i32::from(off > Decimal::ZERO),
            ..Default::default()
        }
    }

    #[test]
    fn vo_maps_status_and_type_names() {
        let vo = to_vo(&apply(0, "10.00", "0"));
        assert_eq!(vo.refund_status_name, "待处理");
        assert_eq!(vo.refund_type_name, "订单");
        assert_eq!(vo.refund_balance, dec("10.00"));
        assert_eq!(vo.is_online, 1);
        assert_eq!(vo.is_offline, 0);
    }

    #[test]
    fn vo_totals_online_plus_offline() {
        let vo = to_vo(&apply(1, "10.00", "5.50"));
        assert_eq!(vo.refund_balance, dec("15.50"));
        assert_eq!(vo.is_online, 1);
        assert_eq!(vo.is_offline, 1);
    }

    #[test]
    fn log_vo_maps_type_name() {
        let l = RefundLog {
            log_id: 3,
            refund_apply_id: 1,
            refund_type: 2,
            refund_amount: dec("20.00"),
            user_id: 9,
            ..Default::default()
        };
        let vo = to_log_vo(&l);
        assert_eq!(vo.refund_type_name, "商品");
        assert_eq!(vo.refund_amount, dec("20.00"));
    }

    #[test]
    fn status_config_lists_all_five() {
        let cfg = RefundApplyService::status_config();
        assert_eq!(cfg.len(), 5);
        assert_eq!(cfg[0], (0, "待处理"));
        assert_eq!(cfg[2], (2, "已处理"));
        assert_eq!(cfg[4], (4, "已拒绝"));
    }

    /// 复刻审核的目标状态白名单（纯逻辑，不依赖数据库）。
    fn audit_target_ok(status: i32) -> bool {
        matches!(
            RefundApplyStatus::try_from(status),
            Ok(RefundApplyStatus::Processing)
                | Ok(RefundApplyStatus::Cancel)
                | Ok(RefundApplyStatus::Rejected)
        )
    }

    #[test]
    fn audit_accepts_only_three_targets() {
        assert!(audit_target_ok(1)); // 处理中
        assert!(audit_target_ok(3)); // 退回售后
        assert!(audit_target_ok(4)); // 拒绝
        assert!(!audit_target_ok(0)); // 待处理不可作为审核目标
        assert!(!audit_target_ok(2)); // 已处理不可作为审核目标
        assert!(!audit_target_ok(9));
    }

    /// 礼品卡返还挂点不变量：两个退款完成点（mark_processed 置已处理、
    /// offline_audit 确认线下到账）都必须在完成动作之后挂返还钩子，
    /// 防止后续重构把返还钩子误删或误挂到完成之前。
    ///
    /// 2026-11-21 子单退款精确化 re-pin：mark_processed 的总额
    /// 口径分支已由整笔 `return_stored_cards_for_order` 改为按额
    /// `return_stored_cards_amount`（本笔卡构成贯穿），手工口径
    /// 分支保留全额函数——断言随之双钉两条调用形态，挂点位置
    /// （完成流转之后）与防重强度不变；offline_audit 仍为全额
    /// 口径，断言不变。
    #[test]
    fn gift_card_return_wired_after_refund_completion() {
        let src = include_str!("refund_apply.rs");

        // mark_processed：先流转到 Processed，再返还礼品卡
        let mp = src
            .find("pub async fn mark_processed")
            .expect("mark_processed 应存在");
        let tail = &src[mp..];
        let transition = tail
            .find("RefundApplyStatus::Processed as i32")
            .expect("mark_processed 应流转到已处理");
        let hook = tail
            .find("return_stored_cards_amount")
            .expect("mark_processed 应挂按额礼品卡返还");
        assert!(
            transition < hook,
            "礼品卡返还必须在退款完成流转之后执行"
        );
        let full_hook = tail
            .find("return_stored_cards_for_order")
            .expect("mark_processed 手工口径应保留全额礼品卡返还");
        assert!(
            transition < full_hook,
            "手工口径全额返还同样必须在退款完成流转之后执行"
        );

        // offline_audit：先 confirm_receive 成功，再返还礼品卡
        let oa = src
            .find("pub async fn offline_audit")
            .expect("offline_audit 应存在");
        let tail = &src[oa..];
        let confirm = tail
            .find("confirm_receive")
            .expect("offline_audit 应确认线下到账");
        let hook = tail
            .find("return_stored_cards_for_order")
            .expect("offline_audit 应挂礼品卡返还");
        assert!(confirm < hook, "礼品卡返还必须在确认到账之后执行");
    }

    /// 分销回退决策：全部退货 → 整行失效（含历史累计达到购买数量）。
    #[test]
    fn clawback_full_refund_invalidates() {
        assert_eq!(
            salesman_clawback_decision(3, 0, 3, dec("90.00"), dec("900.00")),
            ClawbackDecision::Invalidate
        );
        // 历史已退 2、本次再退 1，累计达到购买数量 3
        assert_eq!(
            salesman_clawback_decision(3, 2, 3, dec("30.00"), dec("300.00")),
            ClawbackDecision::Invalidate
        );
    }

    /// 分销回退决策：首次部分退货按剩余占比扣减。
    /// 买 3 退 1：占比 2/3（4 位 HALF_UP）=0.6667；
    /// 90.00×0.6667=60.003→60.00；900.00×0.6667=600.03。
    #[test]
    fn clawback_first_partial_rescales_by_remaining_ratio() {
        assert_eq!(
            salesman_clawback_decision(3, 0, 1, dec("90.00"), dec("900.00")),
            ClawbackDecision::Rescale {
                amount: dec("60.00"),
                order_amount: dec("600.03"),
            }
        );
    }

    /// 分销回退决策：二次部分退货先按历史占比反推原始金额再扣减。
    /// 买 3、首退 1 后金额为 60.00 / 600.03；再退 1：
    /// 历史占比 0.6667 → 反推 60.00/0.6667=89.9955、600.03/0.6667=900.0000；
    /// 本次占比 1/3=0.3333 → 89.9955×0.3333=29.9955→30.00、
    /// 900.0000×0.3333=299.97。
    #[test]
    fn clawback_second_partial_back_derives_original() {
        assert_eq!(
            salesman_clawback_decision(3, 1, 2, dec("60.00"), dec("600.03")),
            ClawbackDecision::Rescale {
                amount: dec("30.00"),
                order_amount: dec("299.97"),
            }
        );
    }

    /// 买家取消直退状态门：仅已支付 + 未发货 + 有效单可走。
    #[test]
    fn buyer_cancel_gate_allows_paid_unshipped() {
        // (order_status, pay_status, shipping_status, is_valid, order_type)
        assert_eq!(buyer_cancel_gate_error(1, 2, 0, 1, Some(1)), None);
        assert_eq!(buyer_cancel_gate_error(1, 2, 0, 1, None), None);
    }

    #[test]
    fn buyer_cancel_gate_rejects() {
        // 未支付
        assert!(buyer_cancel_gate_error(0, 0, 0, 1, Some(1)).is_some());
        // 已发货
        assert!(buyer_cancel_gate_error(2, 2, 2, 1, Some(1)).is_some());
        // 部分发货（待发货态但 shipping_status=1）
        assert!(buyer_cancel_gate_error(1, 2, 1, 1, Some(1)).is_some());
        // 已取消 / 已完成
        assert!(buyer_cancel_gate_error(3, 4, 0, 1, Some(1)).is_some());
        assert!(buyer_cancel_gate_error(5, 2, 2, 1, Some(1)).is_some());
        // 拆单父单
        assert!(buyer_cancel_gate_error(1, 2, 0, 0, Some(1)).is_some());
        // 退款中（已有退款在途）
        assert!(buyer_cancel_gate_error(1, 3, 0, 1, Some(1)).is_some());
        // 拍卖 / 拼团
        assert!(buyer_cancel_gate_error(1, 2, 0, 1, Some(11)).is_some());
        assert!(buyer_cancel_gate_error(1, 2, 0, 1, Some(2)).is_some());
    }

    /// 分销回退决策：购买数量异常时跳过（Java 此处会除零，我方防御）。
    #[test]
    fn clawback_skips_on_invalid_buy_qty() {
        assert_eq!(
            salesman_clawback_decision(0, 0, 0, dec("90.00"), dec("900.00")),
            ClawbackDecision::Skip
        );
    }

    /// 主动退款状态门：待发货/已发货/已完成 + 已支付可发起。
    #[test]
    fn proactive_gate_allows_paid_active_orders() {
        assert_eq!(proactive_gate_error(1, 2, 1), None);
        assert_eq!(proactive_gate_error(2, 2, 1), None);
        assert_eq!(proactive_gate_error(5, 2, 1), None);
        // 部分退款后 pay_status=3（退款中）仍可继续退
        assert_eq!(proactive_gate_error(5, 3, 1), None);
    }

    /// 主动退款状态门：父单壳/未支付/已取消/已失效一律拒绝。
    #[test]
    fn proactive_gate_rejects_shell_and_unpaid() {
        assert!(proactive_gate_error(5, 2, 0).is_some()); // 拆单父单
        assert!(proactive_gate_error(0, 0, 1).is_some()); // 待支付
        assert!(proactive_gate_error(1, 0, 1).is_some()); // 状态在途但未支付
        assert!(proactive_gate_error(3, 2, 1).is_some()); // 已取消
        assert!(proactive_gate_error(4, 2, 1).is_some()); // 已失效
    }

    /// 发起方来源名：主动（平台/商家）与售后/手工四分，VO 透出。
    #[test]
    fn vo_maps_initiator_names() {
        let mut a = apply(0, "10.00", "0");
        a.initiator = 1;
        assert_eq!(to_vo(&a).initiator_name, "平台主动");
        a.initiator = 2;
        assert_eq!(to_vo(&a).initiator_name, "商家主动");
        a.initiator = 0;
        a.aftersale_id = 9;
        assert_eq!(to_vo(&a).initiator_name, "买家售后");
        a.aftersale_id = 0;
        assert_eq!(to_vo(&a).initiator_name, "手工退款");
    }

    /// 现金拆分：售后来源申请扣减本笔礼品卡构成（E2E 订单 77
    /// 口径：退款总额 100 = 卡构成 50 + 现金 50；整单全额退款时
    /// 本笔卡构成恒等于整单抵扣额，见 gift_card_refund_part）。
    #[test]
    fn cash_split_aftersale_deducts_gift_card() {
        assert_eq!(
            cash_refund_amount(dec("100.00"), dec("50.00"), true),
            dec("50.00")
        );
        // 无卡抵扣：全额现金
        assert_eq!(
            cash_refund_amount(dec("66.60"), dec("0"), true),
            dec("66.60")
        );
        // 全额卡抵扣：现金为 0（完成点钩子返还卡即可）
        assert_eq!(cash_refund_amount(dec("50.00"), dec("50.00"), true), dec("0"));
        // 抵扣大于金额（数据异常）时下限为 0，不产生负数退款
        assert_eq!(cash_refund_amount(dec("40.00"), dec("50.00"), true), dec("0"));
    }

    /// 现金拆分：手工申请的 online_balance 本就是线上部分，不扣减。
    #[test]
    fn cash_split_manual_keeps_online_amount() {
        assert_eq!(
            cash_refund_amount(dec("50.00"), dec("50.00"), false),
            dec("50.00")
        );
    }

    /// 卡构成：单笔全额退款恒等于归属份额（与历史整单口径逐分
    /// 一致，E2E 订单 77：份额 50 / 基数 100 / 本笔 100 → 50）。
    #[test]
    fn card_part_full_refund_equals_share() {
        assert_eq!(
            gift_card_refund_part(dec("50.00"), dec("0"), dec("100.00"), dec("0"), dec("100.00")),
            dec("50.00")
        );
        // 子单份额同理（份额 20 / 子单基数 40 / 全额退 40 → 20）
        assert_eq!(
            gift_card_refund_part(dec("20.00"), dec("0"), dec("40.00"), dec("0"), dec("40.00")),
            dec("20.00")
        );
    }

    /// 卡构成：分次退款同比例返还且总额守恒（修复历史口径下
    /// 「首笔整笔回卡 + 后续现金被整单抵扣重复扣减」的少退）：
    /// 份额 50 / 基数 100，分两笔退 30 / 70 → 卡 15 + 35 = 50，
    /// 现金 (30−15) + (70−35) = 50，合计恰 100。
    #[test]
    fn card_part_partial_sequence_conserves_total() {
        let p1 = gift_card_refund_part(
            dec("50.00"),
            dec("0"),
            dec("100.00"),
            dec("0"),
            dec("30.00"),
        );
        assert_eq!(p1, dec("15.00"));
        let p2 = gift_card_refund_part(
            dec("50.00"),
            dec("15.00"),
            dec("100.00"),
            dec("30.00"),
            dec("70.00"),
        );
        assert_eq!(p2, dec("35.00"));
        assert_eq!(p1 + p2, dec("50.00"));
    }

    /// 卡构成：尾差归最后一笔——分币级拆分（如 33.33 分三笔）
    /// 累计返还恰等于份额，不多不少。
    #[test]
    fn card_part_tail_difference_lands_on_last_refund() {
        // 份额 10.00 / 基数 30.00，三笔各退 10.00
        let p1 = gift_card_refund_part(
            dec("10.00"),
            dec("0"),
            dec("30.00"),
            dec("0"),
            dec("10.00"),
        );
        assert_eq!(p1, dec("3.33"));
        let p2 = gift_card_refund_part(
            dec("10.00"),
            dec("3.33"),
            dec("30.00"),
            dec("10.00"),
            dec("10.00"),
        );
        assert_eq!(p2, dec("3.34"));
        let p3 = gift_card_refund_part(
            dec("10.00"),
            dec("6.67"),
            dec("30.00"),
            dec("20.00"),
            dec("10.00"),
        );
        assert_eq!(p3, dec("3.33"));
        assert_eq!(p1 + p2 + p3, dec("10.00"));
    }

    /// 卡构成：未退满基数时（已发货不退运费等）卡只按已退比例
    /// 返还，未退部分对应的卡份额不返还；已全额返还后再退恒 0；
    /// 基数异常（≤0）恒 0。
    #[test]
    fn card_part_partial_base_and_guards() {
        // 基数 100（运费 10 不退），只退 90 → 卡返还 50×90% = 45
        assert_eq!(
            gift_card_refund_part(dec("50.00"), dec("0"), dec("100.00"), dec("0"), dec("90.00")),
            dec("45.00")
        );
        // 已返还满额
        assert_eq!(
            gift_card_refund_part(
                dec("50.00"),
                dec("50.00"),
                dec("100.00"),
                dec("100.00"),
                dec("10.00")
            ),
            dec("0")
        );
        // 基数 ≤ 0
        assert_eq!(
            gift_card_refund_part(dec("50.00"), dec("0"), dec("0"), dec("0"), dec("10.00")),
            dec("0")
        );
        // 无卡份额
        assert_eq!(
            gift_card_refund_part(dec("0"), dec("0"), dec("100.00"), dec("0"), dec("10.00")),
            dec("0")
        );
    }

    /// 资金执行接线不变量：`audit` 审核通过段必须经
    /// `execute_refund_money` 执行资金（余额回补/渠道退款共用
    /// 核心），且仅在同步完成（Completed）时才 mark_processed——
    /// 防止后续重构退回「只有渠道分支、余额支付永不执行」的
    /// 缺陷 B 形态。
    ///
    /// 扫描口径（2026-11-20 re-pin）：限定 `audit` 函数体，且匹配
    /// 调用形态（`.execute_refund_money(` / `self.mark_processed(`）。
    /// 原裸标识符全文件尾扫描会被 `audit` 内说明注释污染——注释按
    /// 「同步执行完成 → mark_processed 置已处理」叙述，`mark_processed`
    /// 字样先于代码中的 `MoneyOutcome::Completed` 判定出现，顺序
    /// 断言误报（check-rust 首个完整跑实证；生产代码的实际调用
    /// 顺序为 执行→Completed 判定→mark_processed，无误）。改扫
    /// 调用形态后断言的是真实调用顺序，强度不降。
    #[test]
    fn audit_executes_money_via_shared_core() {
        let src = include_str!("refund_apply.rs");
        let a = src.find("pub async fn audit").expect("audit 应存在");
        let tail = &src[a..];
        let end = tail
            .find("\n    pub async fn ")
            .expect("audit 后应有下一个方法");
        let body = &tail[..end];
        let exec = body
            .find(".execute_refund_money(")
            .expect("audit 应调用统一资金执行");
        let done = body
            .find("MoneyOutcome::Completed")
            .expect("audit 应按执行结果分支");
        let mp = body
            .find("self.mark_processed(")
            .expect("audit 完成时应 mark_processed");
        assert!(exec < done && done < mp, "资金执行→完成判定→置已处理 顺序不可乱");
    }

    /// 分销佣金回退挂点不变量：`audit` 必须在 Wait→Processing 流转
    /// 之后调用 `handle_salesman_order_on_refund`（触发点=财务审核
    /// 通过，与 Java 一致），防止后续重构把回退钩子误删或误挂到
    /// 审核之前。部分退款金额数学非幂等，回退只许在此与售后自动
    /// 执行路径两处触发（见 handler 方法文档）。
    #[test]
    fn salesman_clawback_wired_after_audit_processing_transition() {
        let src = include_str!("refund_apply.rs");
        let a = src.find("pub async fn audit").expect("audit 应存在");
        let tail = &src[a..];
        let transition = tail
            .find("RefundApplyStatus::Processing as i32")
            .expect("audit 应流转到处理中");
        let hook = tail
            .find("handle_salesman_order_on_refund")
            .expect("audit 应挂分销佣金回退");
        assert!(transition < hook, "分销佣金回退必须在审核通过流转之后执行");
    }

    /// 改金额状态门不变量：`update` 的状态门必须以 `!= Wait` 拒绝
    /// 非待处理申请（与方法文档「仅待处理状态可改」、报错文案
    /// 「仅待处理的申请允许修改」及 `audit` 的 Java Assert 口径一致）。
    /// 2026-11-20 前该门误写 `== Wait`，待处理单恒被拒、已处理单
    /// 反被放行改金额——本测试防止该反转回归。作用域限定在
    /// `update` 函数体内（其后紧邻的 `audit` 用同一 `!=` 形态，
    /// 不得串扰）。
    #[test]
    fn update_gate_rejects_non_wait_status() {
        let src = include_str!("refund_apply.rs");
        let u = src.find("pub async fn update").expect("update 应存在");
        let tail = &src[u..];
        let end = tail
            .find("\n    pub async fn ")
            .expect("update 后应有下一个方法");
        let body = &tail[..end];

        let gate = body
            .find("!= Ok(RefundApplyStatus::Wait as i32)")
            .expect("update 状态门必须以 != Wait 形式拒绝非待处理申请");
        let err = body
            .find("仅待处理的申请允许修改")
            .expect("update 应保留状态门报错文案");
        assert!(gate < err, "状态门必须先于报错返回出现");
        assert!(
            !body.contains("== Ok(RefundApplyStatus::Wait as i32)"),
            "update 内不得再出现 == Wait 的反转状态门"
        );
    }

    /// 源码不变量（拆单子单退款）：渠道退款的总额基准必须经
    /// `channel_total_paylog` 解析（子单 → 父单同 paySn paylog），
    /// 不得直接以子单自身 paylog 的分摊额充当渠道总额。
    #[test]
    fn channel_refund_total_resolves_parent_paylog_for_children() {
        let src = include_str!("refund_apply.rs");
        let start = src
            .find("pub async fn execute_refund_money")
            .expect("execute_refund_money 应存在");
        let tail = &src[start..];
        let end = tail
            .find("\n    async fn credit_balance_refund")
            .expect("execute_refund_money 后应有 credit_balance_refund");
        let body = &tail[..end];
        assert!(
            body.contains(".channel_total_paylog(existing.order_id, &paylog)"),
            "渠道退款前必须解析总额基准 paylog"
        );
        assert!(
            body.contains(".refund_via_channel(&channel_paylog, &refund_sn, cash)"),
            "渠道退款必须以解析后的 paylog 执行"
        );
        // channel_total_paylog 本体：按 parent_order_id + pay_sn 查父单
        let helper = src
            .find("async fn channel_total_paylog")
            .expect("channel_total_paylog 应存在");
        let htail = &src[helper..];
        let hend = htail
            .find("\n    /// 余额支付部分的退款回补")
            .expect("channel_total_paylog 后应有 credit_balance_refund 注释");
        let hbody = &htail[..hend];
        assert!(
            hbody.contains("find_by_order_and_pay_sn(parent_id, &paylog.pay_sn)"),
            "父单 paylog 必须以 (parent_order_id, pay_sn) 解析"
        );
    }
}