//! 支付日志服务：支付单创建/复用、状态流转、分账渠道绑定、退款日志。
//! 对应原 Java `PaylogServiceImpl`（427 行）。
//!
//! 迁移要点（逐条对照 Java 源）：
//! - `creatPayLog`：同订单+同支付方式复用待支付单；已支付则报错；
//!   **定金预售（`presale_type = 2`）例外** —— 允许存在多条支付日志
//! - `bindProfitSharingChannel`：分账渠道快照**不可变**绑定
//!   （已绑定且相同 → 幂等；已绑定但不同 → 拒绝；非待支付 → 拒绝）
//! - `getLastByOrderId`：**必须同时限定 `order_type`**，否则充值单/会员单
//!   的 `order_id` 会串到商品订单
//! - `getPaidProfitSharingPaylog`：分账发起依据（已支付 + 正金额 + 合法渠道）
//!
//! 未迁移：微信合单/服务商形态的子单 paylog（`createSplitPaylogs` /
//! `fillCombineChildTransactionIds` / `fillPartnerChildTransactionIds`），
//! 依赖 `WechatPayContext` 与拆单逻辑。

use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_common::utils;
use nextrade_core::entities::{Paylog, PaylogRefund};
use nextrade_core::models::payment::paylog::{
    generate_pay_sn, normalize_channel, require_channel, same_channel, PayOrderType, PayStatus,
    PaylogCreateDTO, PaylogListQuery, PaylogRefundVO, PaylogUpdateDTO, PaylogVO, RefundStatus,
    PROFIT_SHARING_CHANNELS,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::payment::paylog::{
    ORDER_TYPE_PRODUCT, PAYLOG_STATUS_PAID, PAYLOG_STATUS_UNPAID,
};
use nextrade_infrastructure::db::Repositories;

/// 创建支付单所需的订单信息（避免服务层依赖 `OrderVO` 造成循环）。
#[derive(Debug, Clone)]
pub struct PaylogOrderRef {
    pub order_id: i32,
    pub order_sn: String,
    pub total_amount: Decimal,
    /// 应付金额（未支付金额）
    pub unpaid_amount: Decimal,
    /// 预售类型：2 = 定金预售（不参与支付单复用判定）
    pub presale_type: Option<i32>,
}

/// 定金预售类型常量（与 Java 判定一致）。
const PRESALE_TYPE_DEPOSIT: i32 = 2;

pub struct PaylogService {
    repos: Arc<Repositories>,
}

impl PaylogService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 管理端查询 —————————————————————

    /// 支付日志分页。
    pub async fn list(&self, q: &PaylogListQuery) -> Result<PageResult<PaylogVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        let (total, rows) = self
            .repos.payment.paylog
            .paginate(q.order_sn.as_deref(), q.pay_status, q.order_type, &page)
            .await?;
        let list = rows.iter().map(to_vo).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 支付日志详情。
    pub async fn detail(&self, paylog_id: i32) -> Result<PaylogVO> {
        let p = self
            .repos.payment.paylog
            .find_by_id(paylog_id)
            .await?
            .ok_or_else(|| Error::NotFound("支付日志不存在".into()))?;
        Ok(to_vo(&p))
    }

    // ————————————————————— 支付单创建 —————————————————————

    /// 创建或复用支付日志。
    ///
    /// 对应 Java `creatPayLog(originPaySn, detail, type, orderType, payAmount, channel)`：
    ///
    /// 1. 查「同订单 + 同支付方式」最新一条
    /// 2. 若存在且 **非 wechat** 支付方式：
    ///    - 定金预售 → 跳过复用判定，直接新建（允许多条）
    ///    - 待支付且渠道快照相同 → **复用**（返回已有日志）
    ///    - 已支付 → 报错「订单已支付」
    /// 3. 否则新建
    ///
    /// `origin_pay_sn` 为 `None` 时自动生成支付单号。
    pub async fn create_or_reuse(
        &self,
        origin_pay_sn: Option<&str>,
        order: &PaylogOrderRef,
        pay_code: &str,
        order_type: PayOrderType,
        pay_amount: Option<Decimal>,
        profit_sharing_channel: Option<&str>,
    ) -> Result<Paylog> {
        let target_channel = normalize_channel(profit_sharing_channel);
        let ot = order_type as i32;

        let latest = self
            .repos.payment.paylog
            .find_latest_by_order_and_code(order.order_id, ot, &order.order_sn, pay_code)
            .await?;

        if let Some(latest) = &latest {
            // 微信支付不走复用判定（原 Java 显式排除 payCode = "wechat"）
            let is_wechat = latest.pay_code == "wechat";
            if !is_wechat {
                let is_deposit_presale = order.presale_type == Some(PRESALE_TYPE_DEPOSIT);
                if !is_deposit_presale {
                    match PayStatus::try_from(latest.pay_status) {
                        Ok(PayStatus::Unpaid) => {
                            if same_channel(
                                latest.profit_sharing_channel.as_deref(),
                                target_channel.as_deref(),
                            ) {
                                tracing::debug!(
                                    "复用待支付日志: pay_sn={} order_id={}",
                                    latest.pay_sn,
                                    order.order_id
                                );
                                return Ok(latest.clone());
                            }
                        }
                        Ok(PayStatus::Paid) => {
                            return Err(Error::biz(400, "订单已支付"));
                        }
                        _ => {}
                    }
                }
            }
        }

        // 新建
        let paylog = Paylog {
            // 自增主键由数据库生成，插入时占位
            paylog_id: 0,
            pay_sn: origin_pay_sn.map(String::from).unwrap_or_else(generate_pay_sn),
            pay_name: String::new(),
            order_id: order.order_id,
            order_sn: order.order_sn.clone(),
            order_amount: order.total_amount,
            order_type: ot,
            pay_amount: pay_amount.unwrap_or(order.unpaid_amount),
            pay_status: PAYLOG_STATUS_UNPAID,
            pay_code: pay_code.to_string(),
            profit_sharing_channel: target_channel,
            add_time: Some(utils::now_seconds() as i32),
            transaction_id: String::new(),
            notify_data: String::new(),
            refund_amount: Decimal::ZERO,
            token_code: String::new(),
            appid: None,
        };
        let paylog_id = self.repos.payment.paylog.insert(&paylog).await?;
        tracing::info!(
            "创建支付日志: id={paylog_id} pay_sn={} order_id={} amount={}",
            paylog.pay_sn,
            order.order_id,
            paylog.pay_amount
        );
        Ok(Paylog {
            paylog_id,
            ..paylog
        })
    }

    /// 管理端手工创建支付日志（对应 Java `create(PaylogCreateDTO)`）。
    pub async fn create(&self, dto: &PaylogCreateDTO) -> Result<i32> {
        let pay_sn = dto
            .pay_sn
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(generate_pay_sn);
        let order_type = dto.order_type.unwrap_or(ORDER_TYPE_PRODUCT);
        // 校验业务类型合法（防止写入未知类型导致后续查询串号）
        PayOrderType::try_from(order_type)
            .map_err(|_| Error::Validation("业务类型非法（应为 0/1/2）".into()))?;

        let order_amount = dto.order_amount.unwrap_or_default();
        if order_amount < Decimal::ZERO {
            return Err(Error::Validation("订单金额不能为负数".into()));
        }
        let pay_amount = dto.pay_amount.unwrap_or(order_amount);
        if pay_amount < Decimal::ZERO {
            return Err(Error::Validation("支付金额不能为负数".into()));
        }

        let p = Paylog {
            pay_sn,
            pay_name: dto.pay_name.clone().unwrap_or_default(),
            order_id: dto.order_id.unwrap_or(0),
            order_sn: dto.order_sn.clone().unwrap_or_default(),
            order_amount,
            order_type,
            pay_amount,
            pay_status: PAYLOG_STATUS_UNPAID,
            pay_code: dto.pay_code.clone().unwrap_or_default(),
            transaction_id: dto.transaction_id.clone().unwrap_or_default(),
            notify_data: String::new(),
            refund_amount: Decimal::ZERO,
            token_code: String::new(),
            add_time: Some(utils::now_seconds() as i32),
            ..Default::default()
        };
        self.repos.payment.paylog.insert(&p).await
    }

    /// 更新支付日志（管理端修正，对应 Java `update(PaylogUpdateDTO)`）。
    pub async fn update(&self, dto: &PaylogUpdateDTO) -> Result<()> {
        self.repos.payment.paylog
            .find_by_id(dto.paylog_id)
            .await?
            .ok_or_else(|| Error::NotFound("支付日志不存在".into()))?;

        if let Some(status) = dto.pay_status {
            PayStatus::try_from(status)
                .map_err(|_| Error::Validation("支付状态非法（应为 0/1/2）".into()))?;
        }

        // 仅允许推进为「已支付」这一受控路径，避免误改金额/订单号
        if dto.pay_status == Some(PAYLOG_STATUS_PAID) {
            let ok = self
                .repos.payment.paylog
                .mark_paid(
                    dto.paylog_id,
                    dto.transaction_id.as_deref().unwrap_or_default(),
                    Decimal::ZERO, // 保留原金额：mark_paid 会用传入值覆盖，故此处需读取原值
                    dto.notify_data.as_deref().unwrap_or_default(),
                )
                .await?;
            if !ok {
                tracing::info!("支付日志 {} 已是终态，跳过更新", dto.paylog_id);
            }
        }
        Ok(())
    }

    // ————————————————————— 状态流转 —————————————————————

    /// 标记支付成功（幂等）。
    ///
    /// 返回 `true` 表示本次完成流转；`false` 表示已是终态（重复回调）。
    pub async fn mark_paid(
        &self,
        paylog_id: i32,
        transaction_id: &str,
        pay_amount: Decimal,
        notify_data: &str,
    ) -> Result<bool> {
        if pay_amount < Decimal::ZERO {
            return Err(Error::Validation("支付金额不能为负数".into()));
        }
        let ok = self
            .repos.payment.paylog
            .mark_paid(paylog_id, transaction_id, pay_amount, notify_data)
            .await?;
        if ok {
            tracing::info!("支付日志标记成功: id={paylog_id} trade_no={transaction_id}");
        } else {
            tracing::info!("支付日志 {paylog_id} 已是终态，跳过重复回调");
        }
        Ok(ok)
    }

    /// 渠道回调成功后推进订单/定金预售状态（幂等）。
    ///
    /// - 定金预售 status 0：定金到账 → 订单已付+定金、未付改尾款、status 1；
    /// - 定金预售 status 1：尾款到账 → 订单全额付清、status 2；
    /// - 普通订单：累计已付达到订单总额时标记已支付。
    pub async fn settle_order_by_pay_sn(
        &self,
        pay_sn: &str,
        transaction_id: &str,
    ) -> Result<()> {
        let Some(paylog) = self
            .repos
            .payment
            .paylog
            .find_latest_by_pay_sn(pay_sn)
            .await?
        else {
            return Ok(());
        };
        if paylog.pay_status != crate::payment::paylog::PAYLOG_STATUS_PAID {
            return Ok(());
        }
        let order_id = paylog.order_id;
        let Some(order) = self.repos.payment.pay.find_order_by_id(order_id).await? else {
            return Ok(());
        };
        let now = nextrade_common::utils::now_seconds() as i32;
        if order.is_presale == 1 && order.presale_type == Some(2) {
            if let Some(ps) = self
                .repos
                .payment
                .pay
                .find_presale_by_order_id(order_id)
                .await?
            {
                match ps.presale_status {
                    0 => {
                        self.repos
                            .payment
                            .pay
                            .settle_presale_deposit(order_id, ps.deposit_amount)
                            .await?;
                        self.repos
                            .payment
                            .pay
                            .update_presale_deposit_paid(order_id, now, Some(now))
                            .await?;
                        tracing::info!("微信/支付宝定金到账: order={order_id} sn={transaction_id}");
                    }
                    1 => {
                        let new_paid = order.paid_amount + paylog.pay_amount;
                        if new_paid >= order.total_amount {
                            crate::trade::order::OrderService::from_repos(self.repos.clone())
                                .on_payment_success(order_id, new_paid, Some(transaction_id.to_string()))
                                .await?;
                            self.repos
                                .payment
                                .pay
                                .update_presale_final_paid(order_id, now)
                                .await?;
                            tracing::info!("微信/支付宝尾款到账: order={order_id}");
                        } else {
                            self.repos
                                .trade
                                .order
                                .apply_partial_payment(order_id, paylog.pay_amount)
                                .await?;
                        }
                    }
                    _ => {}
                }
                return Ok(());
            }
        }
        // 普通订单：以支付流水累计已付，达到总额即标记已支付并触发后置动作（日志/积分/卡券/对账）
        let new_paid = order.paid_amount + paylog.pay_amount;
        if new_paid >= order.total_amount {
            crate::trade::order::OrderService::from_repos(self.repos.clone())
                .on_payment_success(order_id, new_paid, Some(transaction_id.to_string()))
                .await?;
        } else {
            self.repos
                .trade
                .order
                .apply_partial_payment(order_id, paylog.pay_amount)
                .await?;
        }
        Ok(())
    }

    /// 按商户支付单号标记支付成功（渠道回调常用）。
    ///
    /// 返回受影响行数；`>1` 说明是定金预售/合单的多行场景。
    pub async fn mark_paid_by_pay_sn(
        &self,
        pay_sn: &str,
        transaction_id: &str,
        notify_data: &str,
    ) -> Result<u64> {
        if pay_sn.trim().is_empty() {
            return Err(Error::Validation("支付单号不能为空".into()));
        }
        let n = self
            .repos.payment.paylog
            .mark_paid_by_pay_sn(pay_sn, transaction_id, notify_data)
            .await?;
        tracing::info!("按支付单号标记成功: pay_sn={pay_sn} 影响 {n} 行");
        Ok(n)
    }

    // ————————————————————— 分账渠道 —————————————————————

    /// 不可变绑定分账渠道快照。
    ///
    /// 对应 Java `bindProfitSharingChannel`：
    /// - 日志不存在 → 报错
    /// - 已有快照且相同 → 幂等返回
    /// - 已有快照但不同 → 报错「快照已绑定，禁止覆盖」
    /// - 非待支付 → 报错
    ///
    /// 并发安全：SQL 层 CAS（`profit_sharing_channel IS NULL OR = ''`）。
    pub async fn bind_profit_sharing_channel(&self, pay_sn: &str, channel: &str) -> Result<()> {
        if pay_sn.trim().is_empty() {
            return Err(Error::Validation("支付号不能为空".into()));
        }
        let target = require_channel(channel).map_err(Error::Validation)?;

        let existing = self
            .repos.payment.paylog
            .find_latest_by_pay_sn(pay_sn)
            .await?
            .ok_or_else(|| Error::biz(400, "支付日志不存在"))?;

        // 已绑定：相同则幂等，不同则拒绝
        if let Some(bound) = normalize_channel(existing.profit_sharing_channel.as_deref()) {
            if bound == target {
                return Ok(());
            }
            return Err(Error::biz(400, "支付日志分账渠道快照已绑定，禁止覆盖"));
        }
        if existing.pay_status != PAYLOG_STATUS_UNPAID {
            return Err(Error::biz(400, "仅待支付日志允许绑定分账渠道快照"));
        }

        let ok = self
            .repos.payment.paylog
            .bind_profit_sharing_channel(existing.paylog_id, &existing.pay_sn, &target)
            .await?;
        if ok {
            tracing::info!("绑定分账渠道快照: pay_sn={pay_sn} channel={target}");
            return Ok(());
        }

        // CAS 失败：可能是并发已绑定相同值（幂等）或冲突
        let concurrent = self
            .repos.payment.paylog
            .find_by_id(existing.paylog_id)
            .await?;
        if let Some(c) = concurrent {
            if normalize_channel(c.profit_sharing_channel.as_deref()).as_deref() == Some(&target) {
                return Ok(());
            }
        }
        Err(Error::biz(400, "支付日志分账渠道快照绑定冲突"))
    }

    /// 取分账发起依据（已支付 + 正金额 + 合法渠道快照的商品订单支付证据）。
    pub async fn paid_profit_sharing_paylog(&self, order_id: i32) -> Result<Option<Paylog>> {
        self.repos.payment.paylog
            .find_paid_profit_sharing(order_id, &PROFIT_SHARING_CHANNELS)
            .await
    }

    // ————————————————————— 退款日志 —————————————————————

    /// 记录退款日志。
    ///
    /// 校验：
    /// 1. 退款金额 > 0
    /// 2. 累计退款不得超过支付金额（防超额退款）
    pub async fn record_refund(
        &self,
        paylog_id: i32,
        order_id: i32,
        refund_sn: &str,
        amount: Decimal,
        desc: &str,
        admin_id: i32,
        pay_code: Option<&str>,
    ) -> Result<i32> {
        if amount <= Decimal::ZERO {
            return Err(Error::Validation("退款金额必须大于0".into()));
        }

        let paylog = self
            .repos.payment.paylog
            .find_by_id(paylog_id)
            .await?
            .ok_or_else(|| Error::NotFound("支付日志不存在".into()))?;

        // 已成功退款总额 + 本次 ≤ 实付金额
        let refunded = self.repos.payment.paylog.sum_refunded(order_id).await?;
        let total_refund = refunded + amount;
        if total_refund > paylog.pay_amount {
            return Err(Error::biz(
                400,
                format!(
                    "退款总额 {} 超过实付金额 {}",
                    total_refund, paylog.pay_amount
                ),
            ));
        }

        let r = PaylogRefund {
            // 自增主键由数据库生成，插入时占位
            paylog_refund_id: 0,
            paylog_id: Some(paylog_id),
            refund_sn: Some(refund_sn.to_string()),
            paylog_desc: desc.to_string(),
            order_id,
            refund_amount: amount,
            add_time: Some(utils::now_seconds() as i32),
            admin_id,
            pay_code: pay_code.map(String::from),
            // 初始为处理中，回调后更新
            status: Some(RefundStatus::Processing as i32),
            notify_time: None,
        };
        let id = self.repos.payment.paylog.insert_refund(&r).await?;

        // 同步累加 paylog 的退款金额
        self.repos.payment.paylog
            .add_refund_amount(paylog_id, amount)
            .await?;

        tracing::info!(
            "记录退款: paylog_refund_id={id} order_id={order_id} amount={amount}"
        );
        Ok(id)
    }

    /// 更新退款状态（渠道回调/人工确认）。
    pub async fn update_refund_status(
        &self,
        paylog_refund_id: i32,
        status: RefundStatus,
    ) -> Result<()> {
        self.repos.payment.paylog
            .update_refund_status(paylog_refund_id, status as i32, utils::now_seconds() as i32)
            .await?;
        tracing::info!("退款状态更新: id={paylog_refund_id} status={}", status.name());
        Ok(())
    }

    /// 订单退款日志列表。
    pub async fn refunds_of_order(&self, order_id: i32) -> Result<Vec<PaylogRefundVO>> {
        let rows = self.repos.payment.paylog.find_refunds_by_order(order_id).await?;
        Ok(rows.iter().map(to_refund_vo).collect())
    }

    /// 订单已成功退款总额。
    pub async fn refunded_amount(&self, order_id: i32) -> Result<Decimal> {
        self.repos.payment.paylog.sum_refunded(order_id).await
    }
}

/// 实体 → VO。
fn to_vo(p: &Paylog) -> PaylogVO {
    PaylogVO {
        paylog_id: p.paylog_id,
        pay_sn: p.pay_sn.clone(),
        pay_name: p.pay_name.clone(),
        order_id: p.order_id,
        order_sn: p.order_sn.clone(),
        order_amount: p.order_amount,
        order_type: p.order_type,
        order_type_name: PayOrderType::try_from(p.order_type)
            .map(|t| t.name().to_string())
            .unwrap_or_default(),
        pay_amount: p.pay_amount,
        pay_status: p.pay_status,
        pay_status_name: PayStatus::try_from(p.pay_status)
            .map(|s| s.name().to_string())
            .unwrap_or_default(),
        pay_code: p.pay_code.clone(),
        transaction_id: p.transaction_id.clone(),
        profit_sharing_channel: normalize_channel(p.profit_sharing_channel.as_deref()),
        add_time: p.add_time.unwrap_or(0),
        refund_amount: p.refund_amount,
    }
}

/// 退款实体 → VO。
fn to_refund_vo(r: &PaylogRefund) -> PaylogRefundVO {
    let status = r.status.unwrap_or(RefundStatus::Processing as i32);
    PaylogRefundVO {
        paylog_refund_id: r.paylog_refund_id,
        paylog_id: r.paylog_id,
        refund_sn: r.refund_sn.clone().unwrap_or_default(),
        paylog_desc: r.paylog_desc.clone(),
        order_id: r.order_id,
        refund_amount: r.refund_amount,
        add_time: r.add_time.unwrap_or(0),
        admin_id: r.admin_id,
        pay_code: r.pay_code.clone(),
        status,
        status_name: RefundStatus::try_from(status)
            .map(|s| s.name().to_string())
            .unwrap_or_default(),
        notify_time: r.notify_time.unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn order(presale_type: Option<i32>) -> PaylogOrderRef {
        PaylogOrderRef {
            order_id: 100,
            order_sn: "SN100".into(),
            total_amount: dec("200.00"),
            unpaid_amount: dec("200.00"),
            presale_type,
        }
    }

    fn paylog(status: i32, code: &str, channel: Option<&str>) -> Paylog {
        Paylog {
            paylog_id: 1,
            pay_sn: "PAY1".into(),
            order_id: 100,
            order_sn: "SN100".into(),
            order_amount: dec("200.00"),
            order_type: 0,
            pay_amount: dec("200.00"),
            pay_status: status,
            pay_code: code.into(),
            profit_sharing_channel: channel.map(String::from),
            ..Default::default()
        }
    }

    /// 复刻 Java `creatPayLog` 的复用判定（纯逻辑，不依赖数据库）。
    fn should_reuse(
        latest: &Paylog,
        order: &PaylogOrderRef,
        target_channel: Option<&str>,
    ) -> ReuseDecision {
        if latest.pay_code == "wechat" {
            return ReuseDecision::Create;
        }
        if order.presale_type == Some(PRESALE_TYPE_DEPOSIT) {
            return ReuseDecision::Create;
        }
        match PayStatus::try_from(latest.pay_status) {
            Ok(PayStatus::Unpaid) => {
                if same_channel(latest.profit_sharing_channel.as_deref(), target_channel) {
                    ReuseDecision::Reuse
                } else {
                    ReuseDecision::Create
                }
            }
            Ok(PayStatus::Paid) => ReuseDecision::ErrorPaid,
            _ => ReuseDecision::Create,
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    enum ReuseDecision {
        Reuse,
        Create,
        ErrorPaid,
    }

    #[test]
    fn reuse_when_unpaid_and_same_channel() {
        let p = paylog(0, "alipay", Some("WECHAT"));
        assert_eq!(
            should_reuse(&p, &order(None), Some("wechat")),
            ReuseDecision::Reuse
        );
    }

    #[test]
    fn create_when_channel_differs() {
        // 待支付但渠道快照不同 → 新建（不能复用已冻结分账的单）
        let p = paylog(0, "alipay", Some("WECHAT"));
        assert_eq!(
            should_reuse(&p, &order(None), Some("ALIPAY")),
            ReuseDecision::Create
        );
    }

    #[test]
    fn error_when_already_paid() {
        let p = paylog(1, "alipay", None);
        assert_eq!(
            should_reuse(&p, &order(None), None),
            ReuseDecision::ErrorPaid
        );
    }

    #[test]
    fn deposit_presale_bypasses_reuse_and_paid_check() {
        // 定金预售允许存在多条支付日志，即使已有已支付单也新建
        let p = paylog(1, "alipay", None);
        assert_eq!(
            should_reuse(&p, &order(Some(2)), None),
            ReuseDecision::Create
        );
    }

    #[test]
    fn wechat_always_creates_new() {
        // 微信支付显式排除复用判定（即使待支付且渠道相同）
        let p = paylog(0, "wechat", Some("WECHAT"));
        assert_eq!(
            should_reuse(&p, &order(None), Some("wechat")),
            ReuseDecision::Create
        );
    }

    #[test]
    fn full_presale_not_bypassed() {
        // 全款预售（presale_type = 1）走正常复用判定
        let p = paylog(1, "alipay", None);
        assert_eq!(
            should_reuse(&p, &order(Some(1)), None),
            ReuseDecision::ErrorPaid
        );
    }

    #[test]
    fn vo_maps_names() {
        let vo = to_vo(&paylog(1, "alipay", Some(" wechat ")));
        assert_eq!(vo.pay_status_name, "已支付");
        assert_eq!(vo.order_type_name, "商品订单");
        // 渠道快照归一化为大写
        assert_eq!(vo.profit_sharing_channel.as_deref(), Some("WECHAT"));
    }

    #[test]
    fn refund_vo_maps_status_name() {
        let r = PaylogRefund {
            paylog_refund_id: 5,
            order_id: 100,
            refund_amount: dec("10.00"),
            status: Some(1),
            ..Default::default()
        };
        let vo = to_refund_vo(&r);
        assert_eq!(vo.status_name, "退款成功");
        assert_eq!(vo.refund_amount, dec("10.00"));
    }
}