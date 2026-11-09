//! 余额支付服务：余额扣减、支付流水、订单支付完成。
//!
//! 这是第一个真实落地的支付渠道（Java 侧 `CashPayServiceImpl` 是占位实现，
//! 真正的余额抵扣散落在 `OrderCheckServiceImpl#calcBalanceDeduction` 与
//! `OrderServiceImpl#setOrderPaid` 中）。
//!
//! 迁移要点（对照 Java 源）：
//! - **余额抵扣上限**：`min(应付金额, 用户余额)`（`OrderCheckServiceImpl` L2536-2542）
//! - **余额支付即付清**：全额抵扣时订单直接进入已支付（`OrderCheckServiceImpl` L1769）
//! - **扣款原子性**：`UPDATE user SET balance = balance - ? WHERE balance >= ?`
//!   （`UserRepository#try_debit_balance`，防并发透支）
//! - **支付完成幂等**：订单 `pay_status=2` 条件更新（防重复回调）
//! - **流水留痕**：写 `paylog`（`pay_code = "balance"`）
//!
//! 未迁移：
//! - 定金预售的余额抵扣上限（依赖 `product_presale` 域）
//! - 账期支付互斥校验（依赖 `credit_user` 域）
//! - 支付后积分赠送/分销佣金/拆单（依赖对应域）

use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_common::utils;
use nextrade_core::entities::Paylog;
use nextrade_core::models::payment::paylog::{generate_pay_sn, PayOrderType};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::payment::paylog::PAYLOG_STATUS_PAID;
use nextrade_infrastructure::db::Repositories;

/// 余额支付的支付方式编码（与 Java `paylog.pay_code = "balance"` 一致）。
pub const BALANCE_PAY_CODE: &str = "balance";

/// 余额支付结果。
#[derive(Debug, Clone)]
pub struct BalancePayResult {
    /// 支付单号
    pub pay_sn: String,
    /// 实际扣除的余额
    pub deducted: Decimal,
    /// 订单是否已全额付清（余额覆盖全部应付）
    pub order_fully_paid: bool,
}

/// 余额支付上下文（供前端展示可抵扣信息）。
#[derive(Debug, Clone)]
pub struct OrderPaymentContext {
    /// 用户可用余额
    pub user_balance: Decimal,
    /// 订单未付金额（已支付则为 0）
    pub unpaid_amount: Decimal,
    /// 本次可用余额抵扣的上限
    pub deductible: Decimal,
    /// 余额是否能全额付清本单
    pub fully_coverable: bool,
    /// 订单是否已支付
    pub already_paid: bool,
}

pub struct BalancePayService {
    repos: Arc<Repositories>,
}

impl BalancePayService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 查询用户可用余额。
    pub async fn balance_of(&self, user_id: i32) -> Result<Decimal> {
        let user = self
            .repos.member.user
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| Error::NotFound("用户不存在".into()))?;
        Ok(user.available_balance())
    }

    /// 计算余额可抵扣金额。
    ///
    /// 对应 Java `OrderCheckServiceImpl` L2536-2542：
    /// `balance = min(应付金额, 用户余额)`。
    pub fn deductible_of(payable: Decimal, user_balance: Decimal) -> Decimal {
        payable.min(user_balance).max(Decimal::ZERO)
    }

    /// 查询余额支付的上下文（供前端展示「可用余额 / 应付 / 可抵扣 / 是否可全额付清」）。
    ///
    /// 归属校验：非本人订单返回 `Forbidden`。订单已支付时 `already_paid=true`
    /// 且可抵扣为 0，便于前端置灰。
    pub async fn order_payment_context(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<OrderPaymentContext> {
        let order = self
            .repos.trade.order
            .find_by_id(order_id)
            .await?
            .ok_or_else(|| Error::NotFound("订单不存在".into()))?;
        if order.user_id != user_id {
            return Err(Error::Forbidden("无权查看该订单".into()));
        }
        let user_balance = self.balance_of(user_id).await?;
        let already_paid = order.is_paid();
        let unpaid = if already_paid {
            Decimal::ZERO
        } else {
            order.unpaid_amount.max(Decimal::ZERO)
        };
        let deductible = Self::deductible_of(unpaid, user_balance);
        Ok(OrderPaymentContext {
            user_balance,
            unpaid_amount: unpaid,
            deductible,
            fully_coverable: !already_paid && deductible >= unpaid && unpaid > Decimal::ZERO,
            already_paid,
        })
    }

    /// 余额支付订单。
    ///
    /// 流程（与 Java 语义一致）：
    /// 1. 校验用户与订单归属
    /// 2. 校验订单未支付（`pay_status != 2`）
    /// 3. 计算可抵扣金额：`min(未付金额, 用户余额)`；为 0 则拒绝
    /// 4. **原子扣减余额**（`WHERE balance >= ?`，失败即余额不足）
    /// 5. 写 `paylog` 流水（`pay_code = balance`）
    /// 6. 余额覆盖全部应付时，标记订单已支付（幂等）
    ///
    /// 部分抵扣场景：订单仍为待支付，剩余金额走其他渠道。
    pub async fn pay_order(
        &self,
        user_id: i32,
        order_id: i32,
        request_amount: Option<Decimal>,
    ) -> Result<BalancePayResult> {
        // 1. 订单校验
        let order = self
            .repos.trade.order
            .find_by_id(order_id)
            .await?
            .ok_or_else(|| Error::NotFound("订单不存在".into()))?;
        if order.user_id != user_id {
            return Err(Error::Forbidden("无权支付该订单".into()));
        }
        if order.is_paid() {
            return Err(Error::biz(400, "订单已支付"));
        }
        let unpaid = order.unpaid_amount.max(Decimal::ZERO);
        if unpaid <= Decimal::ZERO {
            return Err(Error::biz(400, "订单无待付金额"));
        }

        // 2. 余额校验与抵扣额计算
        let user = self
            .repos.member.user
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| Error::NotFound("用户不存在".into()))?;
        let available = user.available_balance();
        let requested = request_amount.unwrap_or(unpaid).max(Decimal::ZERO);
        let deduct = Self::deductible_of(requested.min(unpaid), available);
        if deduct <= Decimal::ZERO {
            return Err(Error::biz(400, "余额不足"));
        }

        // 3. 原子扣减余额（防并发透支）
        let debited = self
            .repos.member.user
            .try_debit_balance(user_id, deduct)
            .await?;
        if !debited {
            return Err(Error::biz(400, "余额不足"));
        }

        // 4. 写支付流水（余额支付直接是已支付状态）
        let pay_sn = generate_pay_sn();
        let pay_name = format!("余额支付-订单{}", order.order_sn);
        let paylog = Paylog {
            paylog_id: 0,
            pay_sn,
            pay_name,
            order_id,
            order_sn: order.order_sn.clone(),
            order_amount: order.total_amount,
            order_type: PayOrderType::Product as i32,
            pay_amount: deduct,
            pay_status: PAYLOG_STATUS_PAID,
            pay_code: BALANCE_PAY_CODE.to_string(),
            profit_sharing_channel: None,
            add_time: Some(utils::now_seconds() as i32),
            transaction_id: format!("BAL{}", utils::now_millis()),
            notify_data: String::new(),
            refund_amount: Decimal::ZERO,
            token_code: String::new(),
            appid: None,
        };
        let paylog_id = self.repos.payment.paylog.insert(&paylog).await?;

        // 5. 结算订单：余额抵扣计入已付；定金预售按「定金 → 尾款」两阶段推进
        let now = utils::now_seconds() as i32;
        let presale = if order.is_presale == 1 && order.presale_type == Some(2) {
            self.repos
                .payment
                .pay
                .find_presale_by_order_id(order_id)
                .await?
        } else {
            None
        };
        let mut order_fully_paid = false;
        if let Some(ps) = presale {
            match ps.presale_status {
                0 => {
                    if deduct >= order.unpaid_amount {
                        // 定金付清：已付=定金、未付=尾款；预售状态置「待付尾款」
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
                        order_fully_paid = false;
                        tracing::info!(
                            "定金余额支付完成: order_id={order_id} deposit={} paylog={paylog_id}",
                            ps.deposit_amount
                        );
                    } else {
                        let ok = self
                            .repos
                            .trade
                            .order
                            .apply_partial_payment(order_id, deduct)
                            .await?;
                        if !ok {
                            self.repos.member.user.adjust_balance(user_id, deduct).await?;
                            self.repos.payment.paylog.delete_paylog(paylog_id).await?;
                            return Err(Error::Conflict("订单支付状态已变更，请刷新后重试".into()));
                        }
                    }
                }
                1 => {
                    let new_paid = order.paid_amount + deduct;
                    order_fully_paid = new_paid >= order.total_amount;
                    if order_fully_paid {
                        let ok = self
                            .repos
                            .trade
                            .order
                            .mark_paid(order_id, now, new_paid)
                            .await?;
                        if !ok {
                            self.repos.member.user.adjust_balance(user_id, deduct).await?;
                            self.repos.payment.paylog.delete_paylog(paylog_id).await?;
                            return Err(Error::Conflict("订单已支付，请刷新后重试".into()));
                        }
                        self.repos
                            .payment
                            .pay
                            .update_presale_final_paid(order_id, now)
                            .await?;
                        tracing::info!(
                            "尾款余额支付完成: order_id={order_id} deducted={deduct} paylog={paylog_id}"
                        );
                    } else {
                        let ok = self
                            .repos
                            .trade
                            .order
                            .apply_partial_payment(order_id, deduct)
                            .await?;
                        if !ok {
                            self.repos.member.user.adjust_balance(user_id, deduct).await?;
                            self.repos.payment.paylog.delete_paylog(paylog_id).await?;
                            return Err(Error::Conflict("订单支付状态已变更，请刷新后重试".into()));
                        }
                    }
                }
                _ => {
                    self.repos.member.user.adjust_balance(user_id, deduct).await?;
                    self.repos.payment.paylog.delete_paylog(paylog_id).await?;
                    return Err(Error::Conflict("预售订单已完成支付".into()));
                }
            }
        } else {
            let new_paid = order.paid_amount + deduct;
            order_fully_paid = new_paid >= order.total_amount;
            if order_fully_paid {
                let ok = self
                    .repos.trade.order
                    .mark_paid(order_id, now, new_paid)
                    .await?;
                if !ok {
                    self.repos.member.user.adjust_balance(user_id, deduct).await?;
                    self.repos.payment.paylog.delete_paylog(paylog_id).await?;
                    return Err(Error::Conflict("订单已支付，请刷新后重试".into()));
                }
                tracing::info!(
                    "余额支付全额付清: order_id={order_id} deducted={deduct} paylog={paylog_id}"
                );
            } else {
                let ok = self
                    .repos.trade.order
                    .apply_partial_payment(order_id, deduct)
                    .await?;
                if !ok {
                    self.repos.member.user.adjust_balance(user_id, deduct).await?;
                    self.repos.payment.paylog.delete_paylog(paylog_id).await?;
                    return Err(Error::Conflict("订单支付状态已变更，请刷新后重试".into()));
                }
                tracing::info!(
                    "余额部分抵扣: order_id={order_id} deducted={deduct} 剩余应付={}",
                    unpaid - deduct
                );
            }
        }

        Ok(BalancePayResult {
            pay_sn: paylog.pay_sn,
            deducted: deduct,
            order_fully_paid,
        })
    }

    /// 余额退款（取消订单/售后退款时回补余额）。
    ///
    /// 幂等性由调用方保证（传入唯一 `refund_sn`）。
    pub async fn refund(
        &self,
        user_id: i32,
        order_id: i32,
        amount: Decimal,
        reason: &str,
    ) -> Result<()> {
        if amount <= Decimal::ZERO {
            return Err(Error::Validation("退款金额必须大于0".into()));
        }
        let order = self
            .repos.trade.order
            .find_by_id(order_id)
            .await?
            .ok_or_else(|| Error::NotFound("订单不存在".into()))?;
        // 归属校验：退款必须退回订单所属用户的余额，防止越权给他人账户加钱
        if order.user_id != user_id {
            return Err(Error::Forbidden("无权操作该订单的退款".into()));
        }

        // 找到该订单的余额支付流水（可能有部分抵扣多笔）
        let paylog = self
            .repos.payment.paylog
            .find_last_balance_paid(order_id, BALANCE_PAY_CODE)
            .await?
            .ok_or_else(|| Error::biz(400, "该订单无余额支付记录"))?;

        // 校验不可超额退款（以该笔余额流水实付额为限）
        let refunded = self.repos.payment.paylog.sum_refunded(order_id).await?;
        if refunded + amount > paylog.pay_amount {
            return Err(Error::biz(400, "退款金额超出余额支付金额"));
        }

        // 回补余额
        self.repos.member.user.adjust_balance(user_id, amount).await?;

        // 记录退款日志并累加 paylog 退款额
        self.repos.payment.paylog
            .add_refund_amount(paylog.paylog_id, amount)
            .await?;

        tracing::info!("余额退款: user={user_id} order={order_id} amount={amount} 原因={reason}");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod deductible {
        use super::*;
        use std::str::FromStr;

        fn d(s: &str) -> Decimal {
            Decimal::from_str(s).unwrap()
        }

        #[test]
        fn balance_covers_payable() {
            // 余额充足 → 抵扣全部应付
            assert_eq!(BalancePayService::deductible_of(d("100.00"), d("500.00")), d("100.00"));
        }

        #[test]
        fn balance_partial() {
            // 余额不足 → 抵扣全部余额
            assert_eq!(BalancePayService::deductible_of(d("100.00"), d("30.50")), d("30.50"));
        }

        #[test]
        fn zero_balance_means_no_deduction() {
            assert_eq!(BalancePayService::deductible_of(d("100.00"), Decimal::ZERO), Decimal::ZERO);
        }

        #[test]
        fn negative_inputs_clamped() {
            // 异常输入（负应付/负余额）不产生负抵扣
            assert_eq!(BalancePayService::deductible_of(d("-5"), d("10")), Decimal::ZERO);
            assert_eq!(BalancePayService::deductible_of(d("10"), d("-5")), Decimal::ZERO);
        }

        #[test]
        fn exact_boundary() {
            // 余额恰好等于应付 → 全额抵扣
            assert_eq!(BalancePayService::deductible_of(d("88.88"), d("88.88")), d("88.88"));
        }
    }

    mod pay_status {
        use nextrade_core::entities::Order;
        use nextrade_core::models::payment::paylog::PayStatus;
        use nextrade_infrastructure::db::repo::payment::paylog::{PAYLOG_STATUS_PAID, PAYLOG_STATUS_UNPAID};

        /// 锁定 paylog 表的状态编号（对应 Java `PAYLOG_STATUS_UNPAID=0 / PAID=1`）。
        #[test]
        fn paylog_status_values_match_java_constant() {
            assert_eq!(PayStatus::Unpaid as i32, PAYLOG_STATUS_UNPAID);
            assert_eq!(PayStatus::Paid as i32, PAYLOG_STATUS_PAID);
            assert_ne!(PayStatus::Unpaid as i32, PayStatus::Paid as i32);
        }

        /// ⚠️ 关键陷阱：`order.pay_status` 与 `paylog.pay_status` 编号**不同**。
        ///
        /// 余额支付同时写两张表：paylog 记 `pay_status=1`（已付），
        /// order 记 `pay_status=2`（已付）。混用会导致流水与订单状态不一致。
        /// 此测试把两套编号同时钉死，防止误改其一。
        #[test]
        fn order_and_paylog_paid_codes_differ() {
            // order 表：2 = 已支付（见 Order::is_paid）
            let paid = Order { pay_status: 2, ..Default::default() };
            let unpaid = Order { pay_status: 1, ..Default::default() };
            assert!(paid.is_paid());
            assert!(!unpaid.is_paid());
            // paylog 表：1 = 已支付，与 order 的 2 不同
            assert_eq!(PayStatus::Paid as i32, 1);
            assert_ne!(PayStatus::Paid as i32, 2);
        }
    }
}