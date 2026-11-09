//! 订单完成 / 退款结算级联（trade::settlement）。
//!
//! 对应原 Java：
//! - `OrderServiceImpl#confirmReceipt` 中的结算段（`autoSettlement` /
//!   分销佣金出账 / `storeSettlement` / 平台消息副本 / 分销员升级消息）
//! - `RabbitMqConsumer` 的 `storeSettlementMessage`（店铺结算）、
//!   `salesmanSettlementMessage` + `OrderServiceImpl#settlement`（分销结算）、
//!   `salesmanUpdateMessage`（分销员等级升级）、`refundSettlementMessage`（退款结算）
//!
//! ## 架构差异（Rust 无 MQ）
//!
//! Java 靠 RabbitMQ 延迟队列逐条触发；Rust 侧在订单完成（确认收货 / 自动收货 /
//! 门店核销）后**同步执行**级联。幂等性等价保证：
//! - 店铺结算：`UPDATE order SET is_settlement=1 WHERE is_settlement=0` 原子抢占；
//! - 分销结算：`UPDATE salesman_order SET status=1 WHERE status=0` 条件更新；
//! - 重复调用不会重复入账（与 Java MQ 重投的防护一致）。
//!
//! 已声明的口径差异（`uncertain_points`）：
//! 1. Java `autoSettlement` 的 `dateType=0`（延时结算）用 5 分钟延迟队列
//!    （源码注释「临时测试」，原始为 15 天）；Rust 立即结算。
//! 2. `hasInProgressFullOrderAftersales`（整单售后进行中跳过自动收货）以
//!    「存在审核处理中售后」近似（自动收货扫描处的防护，见 scheduler）。

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::{Decimal, RoundingStrategy};
use serde_json::Value;
use nextrade_common::utils;
use nextrade_core::entities::order::Order;
use nextrade_core::models::payment::statement::statement_type;
use nextrade_core::Result;
use nextrade_infrastructure::db::Repositories;

fn now_i32() -> i32 {
    i32::try_from(utils::now_seconds()).unwrap_or(0)
}

/// 售后状态（Java `AftersalesStatusEnum`）：1 审核处理中 / 6 已完成。
const AFTERSALES_IN_REVIEW: i32 = 1;
const AFTERSALES_COMPLETE: i32 = 6;
/// 分销记录状态：0 待结算 / 1 已结算 / 2 已失效（Java `SalesmanOrderStatusEnum.INVALID`）。
const SALESMAN_ORDER_INVALID: i32 = 2;
/// 分销配置 code（Java `SalesmanConfigTypeEnum`）。
const CFG_SALESMAN_SETTLEMENT: &str = "salesmanSettlement";
const CFG_SALESMAN_CONFIG: &str = "salesmanConfig";
/// 分账配置 code（Java `SettingsEnum`）。
const CFG_PROFIT_SHARING_ENABLED: &str = "profitSharingEnabled";
const CFG_SPLIT_PAYMENT_METHOD: &str = "splitPaymentMethod";
/// 分销员升级防死循环上限（Java 同款）。
const SALESMAN_MAX_UPGRADES: usize = 20;

/// 结算级联服务。
pub struct SettlementService {
    repos: Arc<Repositories>,
}

impl SettlementService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn pool(&self) -> &sqlx::PgPool {
        &self.repos.pool
    }

    /// 订单完成结算级联总入口（确认收货 / 自动收货 / 核销完成后调用）。
    ///
    /// 各步骤尽力而为：单步失败记 `warn` 不阻断订单完成主流程
    /// （幂等条件更新保证补跑 / 重放不会重复入账）。
    pub async fn on_order_completed(&self, order: &Order) {
        if let Err(e) = self.salesman_auto_settlement(order.order_id).await {
            tracing::warn!("结算级联：分销自动结算失败 order={} err={e}", order.order_id);
        }
        if let Err(e) = self.save_commission_output(order).await {
            tracing::warn!("结算级联：分销佣金出账失败 order={} err={e}", order.order_id);
        }
        if let Err(e) = self.store_settlement(order.order_id).await {
            tracing::warn!("结算级联：店铺结算失败 order={} err={e}", order.order_id);
        }
        if let Err(e) = self.notify_platform_copy(order).await {
            tracing::warn!("结算级联：平台消息副本失败 order={} err={e}", order.order_id);
        }
        if let Err(e) = self.salesman_upgrade_fanout(order).await {
            tracing::warn!("结算级联：分销员升级失败 order={} err={e}", order.order_id);
        }
    }

    /// 订单完成后的统一收尾（前台 / 管理端两条 `confirmReceipt` 路径、
    /// 自动收货与门店核销共用）：
    /// 积分（节点 3）→ 会员成长值 → 次卡主单推进 → 出账单置已结算 → 结算级联。
    ///
    /// 已声明的口径差异：成长值直接取订单总额整数部分
    /// （Java `getRankGrowth` 走 `grow_config` 配置规则，见 `uncertain_points`）。
    pub async fn after_order_completed(&self, order: &Order) {
        // 1. 积分赠送（已完成节点 Node 3）
        let order_service = crate::trade::order::OrderService::from_repos(self.repos.clone());
        let _ = order_service.order_success_point(order, 3).await;

        // 2. 会员成长值累加 + 升级日志
        let growth = order
            .total_amount
            .to_string()
            .parse::<f64>()
            .unwrap_or(0.0) as i32;
        if growth > 0 {
            let log = nextrade_core::entities::RankGrowthLog {
                id: 0,
                user_id: order.user_id,
                r#type: 1, // 完成下单
                growth_points: growth,
                change_type: 1, // 增加
                create_time: Some(now_i32()),
            };
            let _ = self
                .repos
                .member
                .user_center
                .insert_rank_growth_log(&log)
                .await;
            let _ = sqlx::query(
                "UPDATE \"user\" SET \"growth_points\" = \"growth_points\" + $1 WHERE \"user_id\" = $2",
            )
            .bind(growth)
            .bind(order.user_id)
            .execute(self.pool())
            .await;
        }

        // 3. 次卡子单完成时推进次卡主订单
        if let Some(parent_id) = order.parent_order_id.filter(|p| *p > 0) {
            let _ = sqlx::query(
                "UPDATE \"order\" SET \"order_status\" = 5, \"received_time\" = ? \
                 WHERE \"order_id\" = ? AND \"is_card_order\" = 1",
            )
            .bind(now_i32())
            .bind(parent_id)
            .execute(self.pool())
            .await;
        }

        // 4. 出账单结算状态更新（real_settlement = 1）
        let _ = self
            .repos
            .payment
            .statement
            .complete_statement_output_by_order(order.order_id, now_i32())
            .await;

        // 5. 结算级联（分销 / 佣金出账 / 店铺资金 / 平台消息 / 分销员升级）
        self.on_order_completed(order).await;
    }

    // ————————————————— 分销结算 —————————————————

    /// Java `OrderServiceImpl#autoSettlement`：按 `salesmanSettlement` 配置自动结算。
    ///
    /// `settlementType=1`（自动）时结算；`dateType` 的延时分支见模块头差异 1。
    async fn salesman_auto_settlement(&self, order_id: i32) -> Result<()> {
        let Some(data) = self.salesman_config_data(CFG_SALESMAN_SETTLEMENT).await? else {
            return Ok(());
        };
        let settlement_type = data.get("settlementType").and_then(Value::as_i64);
        if settlement_type == Some(1) {
            self.salesman_settlement(order_id, 1).await?;
        }
        // settlementType=2（人工结算）：保持待结算，由后台 manualSettle 处理
        Ok(())
    }

    /// Java `OrderServiceImpl#settlement(orderId, settlementType)`。
    pub async fn salesman_settlement(&self, order_id: i32, settlement_type: i32) -> Result<()> {
        let rows: Vec<(i32, Option<i32>, Option<Decimal>, Option<Decimal>)> = sqlx::query_as(
            "SELECT salesman_order_id, salesman_id, amount, order_amount \
             FROM salesman_order WHERE order_id = $1",
        )
        .bind(order_id)
        .fetch_all(self.pool())
        .await
        .map_err(db_err)?;
        let now = now_i32();
        for (sid, salesman_id, amount, order_amount) in rows {
            // 已结算（!=0）跳过；条件更新保证并发 / 重放安全
            let claimed = sqlx::query(
                "UPDATE salesman_order SET status = 1, settlement_time = $1 \
                 WHERE salesman_order_id = $2 AND status = 0",
            )
            .bind(now)
            .bind(sid)
            .execute(self.pool())
            .await
            .map_err(db_err)?
            .rows_affected()
                > 0;
            if !claimed {
                continue;
            }
            let Some(salesman_id) = salesman_id else { continue };
            // 累计推广金额
            let _ = sqlx::query(
                "UPDATE salesman SET sale_amount = COALESCE(sale_amount, 0) + $1 \
                 WHERE salesman_id = $2",
            )
            .bind(order_amount.unwrap_or(Decimal::ZERO))
            .bind(salesman_id)
            .execute(self.pool())
            .await;
            if settlement_type == 1 {
                if let Err(e) =
                    self.inc_commission_balance(salesman_id, order_id, amount.unwrap_or(Decimal::ZERO))
                        .await
                {
                    tracing::warn!("结算级联：佣金入账失败 salesman={salesman_id} err={e}");
                }
            }
            if let Err(e) = self.salesman_update(salesman_id).await {
                tracing::warn!("结算级联：分销员升级失败 salesman={salesman_id} err={e}");
            }
        }
        Ok(())
    }

    /// Java `UserBalanceServiceImpl#incCommissionBalance`：佣金账户入账 + 流水
    /// （`user_balance_log.wallet_type=1`，与余额钱包隔离，可单独提现）。
    async fn inc_commission_balance(
        &self,
        salesman_id: i32,
        order_id: i32,
        amount: Decimal,
    ) -> Result<()> {
        let user_id: Option<i32> = sqlx::query_scalar(
            "SELECT user_id FROM salesman WHERE salesman_id = $1",
        )
        .bind(salesman_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        let Some(user_id) = user_id.filter(|v| *v > 0) else {
            return Ok(());
        };
        if amount <= Decimal::ZERO {
            return Ok(());
        }
        let order_sn: String = sqlx::query_scalar("SELECT order_sn FROM \"order\" WHERE order_id = $1")
            .bind(order_id)
            .fetch_optional(self.pool())
            .await
            .map_err(db_err)?
            .unwrap_or_default();
        let desc = format!("订单{order_sn}结算佣金");

        let before: Option<Decimal> = sqlx::query_scalar(
            "SELECT commission_balance FROM \"user\" WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        let Some(before) = before else { return Ok(()) };
        let frozen: Decimal = sqlx::query_scalar(
            "SELECT frozen_commission_balance FROM \"user\" WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_one(self.pool())
        .await
        .map_err(db_err)?;
        let after = before + amount;

        let _ = sqlx::query(
            "UPDATE \"user\" SET commission_balance = commission_balance + $1 WHERE user_id = $2",
        )
        .bind(amount)
        .bind(user_id)
        .execute(self.pool())
        .await;
        let _ = sqlx::query(
            "INSERT INTO user_balance_log \
             (user_id, before_balance, balance, after_balance, frozen_balance, \
              before_frozen_balance, after_frozen_balance, change_time, change_desc, \
              change_type, wallet_type, new_balance, new_frozen_balance) \
             VALUES ($1, $2, $3, $4, 0, $5, $6, $7, $8, 1, 1, $9, $10)",
        )
        .bind(user_id)
        .bind(before)
        .bind(amount)
        .bind(after)
        .bind(frozen)
        .bind(frozen)
        .bind(now_i32())
        .bind(desc)
        .bind(after)
        .bind(frozen)
        .execute(self.pool())
        .await;
        Ok(())
    }

    /// Java `confirmReceipt` 的分销佣金出账：非自营店铺订单写
    /// `DISTRIBUTION_COMMISSION`（type=8）出账记录（店铺承担佣金）。
    async fn save_commission_output(&self, order: &Order) -> Result<()> {
        if order.shop_id <= 0 {
            return Ok(());
        }
        let total: Decimal = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount), 0) FROM salesman_order \
             WHERE order_id = $1 AND status <> $2",
        )
        .bind(order.order_id)
        .bind(SALESMAN_ORDER_INVALID)
        .fetch_one(self.pool())
        .await
        .map_err(db_err)?;
        if total <= Decimal::ZERO {
            return Ok(());
        }
        let _ = sqlx::query(
            "INSERT INTO statement_output \
             (shop_id, vendor_id, income, expenditure, gmt_create, record_sn, \
              record_type, record_id, settlement_status, real_settlement) \
             VALUES ($1, $2, 0, $3, $4, $5, $6, $7, 0, 0)",
        )
        .bind(order.shop_id)
        .bind(order.vendor_id)
        .bind(total)
        .bind(now_i32() as i64)
        .bind(&order.order_sn)
        .bind(statement_type::DISTRIBUTION_COMMISSION)
        .bind(order.order_id)
        .execute(self.pool())
        .await;
        Ok(())
    }

    // ————————————————— 店铺结算 —————————————————

    /// Java `RabbitMqConsumer#storeSettlementMessage`：店铺资金结算。
    pub async fn store_settlement(&self, order_id: i32) -> Result<()> {
        let Some(order) = self.repos.trade.order.find_by_id(order_id).await? else {
            tracing::error!("结算级联：订单不存在 order={order_id}");
            return Ok(());
        };
        // 未结算（is_settlement=0）才处理
        if order.is_settlement.unwrap_or(1) != 0 {
            return Ok(());
        }
        let Some((shop_money, frozen_money)) =
            sqlx::query_as::<_, (Option<Decimal>, Option<Decimal>)>(
                "SELECT shop_money, frozen_money FROM shop WHERE shop_id = $1",
            )
            .bind(order.shop_id)
            .fetch_optional(self.pool())
            .await
            .map_err(db_err)?
        else {
            tracing::error!("结算级联：店铺不存在 shop={}", order.shop_id);
            return Ok(());
        };
        // 原子抢占结算权（MQ 重投 / 多实例并发防护）
        let claimed = sqlx::query(
            "UPDATE \"order\" SET is_settlement = 1 WHERE order_id = $1 AND is_settlement = 0",
        )
        .bind(order_id)
        .execute(self.pool())
        .await
        .map_err(db_err)?
        .rows_affected()
            > 0;
        if !claimed {
            return Ok(());
        }

        // 已完成售后的退款额
        let aftersales: Vec<(i32, Option<Decimal>)> = sqlx::query_as(
            "SELECT aftersale_id, refund_amount FROM aftersales \
             WHERE order_id = $1 AND status = $2",
        )
        .bind(order_id)
        .bind(AFTERSALES_COMPLETE)
        .fetch_all(self.pool())
        .await
        .map_err(db_err)?;
        let mut refund_amount = Decimal::ZERO;
        let is_vendor_order = order.vendor_id.filter(|v| *v > 0).is_some();
        for (_, refund) in &aftersales {
            refund_amount += refund.unwrap_or(Decimal::ZERO);
            if is_vendor_order {
                refund_amount += self.vendor_refund_amount(order_id, None).await?;
            }
        }
        // 供应商应得（供应价 × 数量）
        let vendor_amount = if is_vendor_order {
            self.vendor_order_amount(order_id).await?
        } else {
            Decimal::ZERO
        };
        // 分销佣金由店铺承担
        let commission: Decimal = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount), 0) FROM salesman_order \
             WHERE order_id = $1 AND status <> $2",
        )
        .bind(order_id)
        .bind(SALESMAN_ORDER_INVALID)
        .fetch_one(self.pool())
        .await
        .map_err(db_err)?;

        let mut paid_amount = order.paid_amount - refund_amount - vendor_amount - commission;

        // 线上分账判定（渠道无关：该单自身支付渠道 + 二级户激活）
        let split = self.resolve_online_split(&order).await?;
        if let Some(online_amount) = split {
            let shop_net =
                shop_money_net_on_settle(order.paid_amount, online_amount, refund_amount);
            if shop_net > Decimal::ZERO {
                self.credit_shop(&order, shop_money, frozen_money, shop_net)
                    .await?;
            } else {
                tracing::info!("结算级联：order={order_id} 线上分账余额份额为 0，跳过 shopMoney");
            }
        } else {
            let mut fee = Decimal::ZERO;
            if paid_amount > Decimal::ZERO {
                let rate = self.shop_service_fee(order_id).await?;
                fee = (paid_amount * rate / Decimal::from(100)).round_dp(2);
            }
            paid_amount -= fee;
            self.credit_shop(&order, shop_money, frozen_money, paid_amount)
                .await?;
        }

        // 出账单 → 已结算（Java updateStatement2Settlement 逐类型）
        for record_type in [
            statement_type::ORDER,
            statement_type::SERVICE_FEE,
            statement_type::DISTRIBUTION_COMMISSION,
        ] {
            let _ = self.mark_output_settled(order_id, record_type).await;
        }
        for (aftersale_id, _) in &aftersales {
            let _ = self
                .mark_output_settled(*aftersale_id, statement_type::ORDER_REFUND)
                .await;
            let _ = self
                .mark_output_settled(*aftersale_id, statement_type::SERVICE_FEE)
                .await;
        }
        Ok(())
    }

    /// 店铺资金入账 + \"shop_account_log\" 流水（Java \"saveAccount\"，type='4'）。
    async fn credit_shop(
        &self,
        order: &Order,
        shop_money: Option<Decimal>,
        frozen_money: Option<Decimal>,
        delta: Decimal,
    ) -> Result<()> {
        if delta == Decimal::ZERO {
            return Ok(());
        }
        let old_shop = shop_money.unwrap_or(Decimal::ZERO);
        let frozen = frozen_money.unwrap_or(Decimal::ZERO);
        let _ = sqlx::query("UPDATE shop SET shop_money = shop_money + $1 WHERE shop_id = $2")
            .bind(delta)
            .bind(order.shop_id)
            .execute(self.pool())
            .await;
        let _ = sqlx::query(
            "INSERT INTO shop_account_log \
             (add_time, shop_money, frozen_money, \"type\", new_shop_money, \
              new_frozen_money, shop_id, is_new) \
             VALUES ($1, $2, $3, '4', $4, $5, $6, 1)",
        )
        .bind(now_i32())
        .bind(old_shop)
        .bind(frozen)
        .bind(old_shop + delta)
        .bind(frozen)
        .bind(order.shop_id)
        .execute(self.pool())
        .await;
        Ok(())
    }

    /// 店铺资金扣减 + 流水（Java `saveDescAccount`，退款结算用）。
    async fn debit_shop(
        &self,
        order: &Order,
        shop_money: Option<Decimal>,
        frozen_money: Option<Decimal>,
        delta: Decimal,
    ) -> Result<()> {
        if delta == Decimal::ZERO {
            return Ok(());
        }
        let old_shop = shop_money.unwrap_or(Decimal::ZERO);
        let frozen = frozen_money.unwrap_or(Decimal::ZERO);
        let _ = sqlx::query("UPDATE shop SET shop_money = shop_money - $1 WHERE shop_id = $2")
            .bind(delta)
            .bind(order.shop_id)
            .execute(self.pool())
            .await;
        let _ = sqlx::query(
            "INSERT INTO shop_account_log \
             (add_time, shop_money, frozen_money, \"type\", new_shop_money, \
              new_frozen_money, shop_id, is_new) \
             VALUES ($1, $2, $3, '4', $4, $5, $6, 1)",
        )
        .bind(now_i32())
        .bind(old_shop)
        .bind(frozen)
        .bind(old_shop - delta)
        .bind(frozen)
        .bind(order.shop_id)
        .execute(self.pool())
        .await;
        Ok(())
    }

    /// Java `getVendorOrderAmount`：供应商应得（供应价 × 数量，vendor_id 非空行）。
    async fn vendor_order_amount(&self, order_id: i32) -> Result<Decimal> {
        let rows: Vec<(Option<Decimal>, i32)> = sqlx::query_as(
            "SELECT vendor_product_supply_price, quantity FROM order_item \
             WHERE order_id = $1 AND vendor_id IS NOT NULL",
        )
        .bind(order_id)
        .fetch_all(self.pool())
        .await
        .map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|(price, qty)| price.unwrap_or(Decimal::ZERO) * Decimal::from(qty))
            .sum())
    }

    /// Java \"getVendorRefundAmount\"：供应商已退（售后行数量 × 供应价）。
    async fn vendor_refund_amount(&self, order_id: i32, aftersale_id: Option<i32>) -> Result<Decimal> {
        let ids: Vec<i32> = match aftersale_id {
            Some(id) => vec![id],
            None => sqlx::query_scalar(
                "SELECT aftersale_id FROM aftersales WHERE order_id = $1 AND status = $2",
            )
            .bind(order_id)
            .bind(AFTERSALES_COMPLETE)
            .fetch_all(self.pool())
            .await
            .map_err(db_err)?,
        };
        if ids.is_empty() {
            return Ok(Decimal::ZERO);
        }
        let ph = vec!["?"; ids.len()].join(", ");
        let sql = nextrade_infrastructure::db::to_pg_placeholders(&format!(
            "SELECT ai.order_item_id, SUM(ai.number) \
             FROM aftersales_item ai WHERE ai.aftersale_id IN ({ph}) GROUP BY ai.order_item_id"
        ));
        let mut q = sqlx::query_as::<_, (i32, i64)>(&sql);
        for id in &ids {
            q = q.bind(id);
        }
        let refunded: HashMap<i32, i64> = q
            .fetch_all(self.pool())
            .await
            .map_err(db_err)?
            .into_iter()
            .collect();
        if refunded.is_empty() {
            return Ok(Decimal::ZERO);
        }
        let item_ids: Vec<i32> = refunded.keys().copied().collect();
        let ph2 = vec!["?"; item_ids.len()].join(", ");
        let sql2 = nextrade_infrastructure::db::to_pg_placeholders(&format!(
            "SELECT item_id, vendor_product_supply_price FROM order_item \
             WHERE order_id = ? AND vendor_id IS NOT NULL AND item_id IN ({ph2})"
        ));
        let mut q2 = sqlx::query_as::<_, (i32, Option<Decimal>)>(&sql2);
        q2 = q2.bind(order_id);
        for id in &item_ids {
            q2 = q2.bind(id);
        }
        let items = q2.fetch_all(self.pool()).await.map_err(db_err)?;
        Ok(items
            .into_iter()
            .map(|(item_id, price)| {
                price.unwrap_or(Decimal::ZERO)
                    * Decimal::from(refunded.get(&item_id).copied().unwrap_or(0))
            })
            .sum())
    }

    /// 线上分账判定（Java \"resolveOnlineSplit\"）：返回 `Some(二级户实收)` 表示走线上分账。
    ///
    /// 门：分账总开关开 + 分账方式=线上 + 该单成功网关支付渠道的二级户已激活。
    /// 任一异常降级为平台代收（\"None\"）。
    async fn resolve_online_split(&self, order: &Order) -> Result<Option<Decimal>> {
        let cfgs = self
            .repos
            .system
            .config_basic
            .find_by_codes(&[CFG_PROFIT_SHARING_ENABLED, CFG_SPLIT_PAYMENT_METHOD])
            .await?;
        let map: HashMap<String, String> = cfgs.into_iter().collect();
        if map.get(CFG_PROFIT_SHARING_ENABLED).map(String::as_str) == Some("0") {
            return Ok(None);
        }
        if map.get(CFG_SPLIT_PAYMENT_METHOD).map(String::as_str) != Some("2") {
            return Ok(None);
        }
        if order.shop_id <= 0 {
            return Ok(None);
        }
        let paylogs: Vec<(String, Decimal)> = sqlx::query_as(
            "SELECT pay_code, pay_amount FROM paylog \
             WHERE order_id = $1 AND order_type = 0 AND pay_status = 1 AND pay_amount > 0 \
             ORDER BY paylog_id DESC",
        )
        .bind(order.order_id)
        .fetch_all(self.pool())
        .await
        .map_err(db_err)?;
        for (pay_code, pay_amount) in paylogs {
            let Some(channel) = resolve_pay_channel(&pay_code) else {
                continue;
            };
            let active: Option<i64> = sqlx::query_scalar(
                "SELECT id FROM sub_merchant \
                 WHERE shop_id = $1 AND pay_channel = $2 AND status = 1 AND is_del = 0 \
                 LIMIT 1",
            )
            .bind(order.shop_id)
            .bind(channel)
            .fetch_optional(self.pool())
            .await
            .map_err(db_err)?;
            if active.is_some() {
                return Ok(Some(pay_amount));
            }
        }
        Ok(None)
    }

    /// Java `StatementServiceImpl#getRecordRate` → `shopServiceFee`（缺行回 0）。
    async fn shop_service_fee(&self, order_id: i32) -> Result<Decimal> {
        let rate: Option<Decimal> = sqlx::query_scalar(
            "SELECT shop_service_fee FROM record_rate \
             WHERE record_id = $1 AND record_type = $2 ORDER BY record_rate_id DESC LIMIT 1",
        )
        .bind(order_id)
        .bind(statement_type::ORDER)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        Ok(rate.unwrap_or(Decimal::ZERO))
    }

    /// Java \"updateStatement2Settlement\"：出账单标记已结算。
    async fn mark_output_settled(&self, record_id: i32, record_type: i32) -> Result<()> {
        let n = sqlx::query(
            "UPDATE statement_output SET real_settlement = 1, real_settlement_time = $1 \
             WHERE record_id = $2 AND record_type = $3",
        )
        .bind(now_i32())
        .bind(record_id)
        .bind(record_type)
        .execute(self.pool())
        .await
        .map_err(db_err)?
        .rows_affected();
        if n > 0 {
            tracing::info!("结算级联：出账单 {record_id}/{record_type} 置已结算 {n} 行");
        }
        Ok(())
    }

    // ————————————————— 消息 / 分销员升级 —————————————————

    /// Java `confirmReceipt`：店铺订单再给平台（shopId=0）发一条完成通知。
    async fn notify_platform_copy(&self, order: &Order) -> Result<()> {
        if order.shop_id <= 0 {
            return Ok(());
        }
        let title = format!("订单已完成通知：{},金额：{}", order.order_sn, order.total_amount);
        let content = format!("您有订单【{}】已完成，请注意查看。", order.order_sn);
        let related = serde_json::json!({ "orderId": order.order_id }).to_string();
        let _ = self
            .repos
            .system
            .admin_msg
            .create_message(13, 0, order.order_id, &title, &content, &related)
            .await;
        Ok(())
    }

    /// Java `confirmReceipt` 末段：本单各分销记录 + 自购分销员的等级升级。
    async fn salesman_upgrade_fanout(&self, order: &Order) -> Result<()> {
        let salesman_ids: Vec<i32> = sqlx::query_scalar(
            "SELECT DISTINCT salesman_id FROM salesman_order WHERE order_id = $1",
        )
        .bind(order.order_id)
        .fetch_all(self.pool())
        .await
        .map_err(db_err)?;
        for salesman_id in salesman_ids {
            let _ = self.salesman_update(salesman_id).await;
        }
        let self_buyer: Option<i32> =
            sqlx::query_scalar("SELECT salesman_id FROM salesman WHERE user_id = $1")
                .bind(order.user_id)
                .fetch_optional(self.pool())
                .await
                .map_err(db_err)?;
        if let Some(salesman_id) = self_buyer {
            let _ = self.salesman_update(salesman_id).await;
        }
        Ok(())
    }

    /// Java `RabbitMqConsumer#salesmanUpdateMessage`：按 `salesmanConfig.level`
    /// 条件逐级升级（最多 20 次防死循环）。
    pub async fn salesman_update(&self, salesman_id: i32) -> Result<()> {
        let Some(cfg) = self.salesman_config_data(CFG_SALESMAN_CONFIG).await? else {
            return Ok(());
        };
        let Some(levels) = cfg.get("level").and_then(Value::as_array) else {
            return Ok(());
        };
        for _ in 0..=SALESMAN_MAX_UPGRADES {
            let Ok(Some((level, user_id))) = sqlx::query_as::<_, (Option<i32>, Option<i32>)>(
                "SELECT level, user_id FROM salesman WHERE salesman_id = $1",
            )
            .bind(salesman_id)
            .fetch_optional(self.pool())
            .await
            else {
                return Ok(());
            };
            let next_level = level.unwrap_or(0) + 1;
            let Some(next) = levels
                .iter()
                .find(|l| l.get("id").and_then(Value::as_i64) == Some(i64::from(next_level)))
            else {
                return Ok(());
            };
            let Some(condition) = next.get("condition") else {
                return Ok(());
            };
            // 三个可选门槛：发展客户数 / 自购金额 / 推广金额（已结算口径）
            if let Some(required) = condition_threshold(condition, "salesInviteUsers") {
                let invitees: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM salesman_customer WHERE salesman_id = $1",
                )
                .bind(salesman_id)
                .fetch_one(self.pool())
                .await
                .map_err(db_err)?;
                if Decimal::from(invitees) < required {
                    return Ok(());
                }
            }
            if let Some(required) = condition_threshold(condition, "selfBuyAmount") {
                let Some(uid) = user_id else { return Ok(()) };
                let self_buy: Decimal = sqlx::query_scalar(
                    "SELECT COALESCE(SUM(total_amount), 0) FROM \"order\" \
                     WHERE user_id = $1 AND order_status = 5",
                )
                .bind(uid)
                .fetch_one(self.pool())
                .await
                .map_err(db_err)?;
                if self_buy < required {
                    return Ok(());
                }
            }
            if let Some(required) = condition_threshold(condition, "salesAmount") {
                let sales: Decimal = sqlx::query_scalar(
                    "SELECT COALESCE(SUM(order_amount), 0) FROM salesman_order \
                     WHERE salesman_id = $1 AND status = 1",
                )
                .bind(salesman_id)
                .fetch_one(self.pool())
                .await
                .map_err(db_err)?;
                if sales < required {
                    return Ok(());
                }
            }
            let _ = sqlx::query("UPDATE salesman SET level = $1 WHERE salesman_id = $2")
                .bind(next_level)
                .bind(salesman_id)
                .execute(self.pool())
                .await;
        }
        tracing::error!("结算级联：分销员升级超过 {SALESMAN_MAX_UPGRADES} 次，疑似死循环 salesman={salesman_id}");
        Ok(())
    }

    /// 读取 `salesman_config.data` JSON（code 命中且可解析时）。
    async fn salesman_config_data(&self, code: &str) -> Result<Option<Value>> {
        let raw: Option<String> = sqlx::query_scalar(
            "SELECT data FROM salesman_config WHERE code = $1 LIMIT 1",
        )
        .bind(code)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        Ok(raw
            .as_deref()
            .and_then(|data| serde_json::from_str::<Value>(data).ok()))
    }

    // ————————————————— 退款结算 —————————————————

    /// 售后退款完成后的收尾（Java `AftersalesServiceImpl#update` 审核通过段 +
    /// \"completeReturnAftersales\"）：退款出账单 + 店铺退款结算 + 拼团整团解散。
    ///
    /// 账期支付订单（\"credit_order\" 存在）跳过店铺退款结算（Java 同款门）。
    pub async fn on_refund_completed(&self, aftersale_id: i32) {
        let row: Option<(i32, Option<i32>, Option<Decimal>)> = sqlx::query_as(
            "SELECT order_id, vendor_id, refund_amount FROM aftersales WHERE aftersale_id = $1",
        )
        .bind(aftersale_id)
        .fetch_optional(self.pool())
        .await
        .ok()
        .flatten();
        let Some((order_id, _vendor_id, refund_amount)) = row else {
            return;
        };
        let credit: Option<i32> = sqlx::query_scalar(
            "SELECT credit_order_id FROM credit_order WHERE order_id = $1 LIMIT 1",
        )
        .bind(order_id)
        .fetch_optional(self.pool())
        .await
        .ok()
        .flatten();
        let is_credit = credit.is_some();

        if !is_credit {
            if let Ok(Some(order)) = self.repos.trade.order.find_by_id(order_id).await {
                if order.is_settlement.unwrap_or(0) != 0 {
                    // Java generateStatement(aftersales)：退款出账单（type 7，record=售后）
                    let exists: i64 = sqlx::query_scalar(
                        "SELECT COUNT(*) FROM statement_output \
                         WHERE record_id = $1 AND record_type = $2",
                    )
                    .bind(aftersale_id)
                    .bind(statement_type::ORDER_REFUND)
                    .fetch_one(self.pool())
                    .await
                    .unwrap_or(0);
                    if exists == 0 {
                        let _ = sqlx::query(
                            "INSERT INTO statement_output \
                             (shop_id, vendor_id, income, expenditure, gmt_create, record_sn, \
                              record_type, record_id, settlement_status, real_settlement) \
                             VALUES ($1, $2, 0, $3, $4, '', $5, $6, 0, 0)",
                        )
                        .bind(order.shop_id)
                        .bind(order.vendor_id)
                        .bind(refund_amount.unwrap_or(Decimal::ZERO))
                        .bind(now_i32() as i64)
                        .bind(statement_type::ORDER_REFUND)
                        .bind(aftersale_id)
                        .execute(self.pool())
                        .await;
                    }
                    // Java：非账期 + 已结算订单 → REFUND_SETTLEMENT 队列
                    if let Err(e) = self.refund_settlement(aftersale_id).await {
                        tracing::warn!("结算级联：退款结算失败 aftersale={aftersale_id} err={e}");
                    }
                }
            }
        }

        // 拼团订单：团长仅退款审核通过 → 整团解散（Java dissolveGroupByCaptainRefund）
        if let Err(e) = self.dissolve_group_by_captain_refund(order_id).await {
            tracing::warn!("结算级联：拼团整团解散失败 order={order_id} err={e}");
        }
    }

    /// Java `GrouponPromotionRecordServiceImpl#dissolveGroupByCaptainRefund`：
    /// 团长（开团 + 待成团）退款 → 整团置失败并为其他真实团员自动创建退款售后。
    ///
    /// 门槛：2 人团（minGroupNum<3）不解散；无其他参团者不解散。
    pub async fn dissolve_group_by_captain_refund(&self, captain_order_id: i32) -> Result<()> {
        let captain: Option<(i32, i64, i32, i32)> = sqlx::query_as(
            "SELECT join_type, open_group_record_id, join_status, min_group_num \
             FROM groupon_promotion_record WHERE order_id = $1 LIMIT 1",
        )
        .bind(captain_order_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        let Some((join_type, open_group_id, join_status, min_group_num)) = captain else {
            return Ok(());
        };
        // 仅团长（开团）+ 待成团才触发；2 人团不解散
        const JOIN_TYPE_OPEN: i32 = 1;
        const JOIN_STATUS_WAIT: i32 = 1;
        if join_type != JOIN_TYPE_OPEN || join_status != JOIN_STATUS_WAIT || min_group_num < 3 {
            return Ok(());
        }
        let members: Vec<(i64, i64, Decimal)> = sqlx::query_as(
            "SELECT order_id, user_id, paid_amount FROM groupon_promotion_record \
             WHERE open_group_record_id = $1 AND join_status = $2 AND order_id <> $3 AND user_id <> 0",
        )
        .bind(open_group_id)
        .bind(JOIN_STATUS_WAIT)
        .bind(captain_order_id)
        .fetch_all(self.pool())
        .await
        .map_err(db_err)?;
        if members.is_empty() {
            return Ok(());
        }
        // 整团待成团记录置拼团失败
        let n = self
            .repos
            .marketing
            .groupon_promotion_record
            .mark_failed_by_open(open_group_id)
            .await?;
        tracing::info!("结算级联：团长退款整团解散 open_group={open_group_id} 置失败 {n} 行");
        // 为其他真实团员自动创建退款售后（复用 aftersales::create_groupon_refund，幂等）
        let aftersales = crate::trade::aftersales::AftersalesService::new(self.repos.clone());
        for (order_id, user_id, paid_amount) in members {
            if let Err(e) = aftersales
                .create_groupon_refund(
                    i32::try_from(order_id).unwrap_or(i32::MAX),
                    i32::try_from(user_id).unwrap_or(0),
                    paid_amount,
                )
                .await
            {
                tracing::warn!("结算级联：团员退款售后创建失败 order={order_id} err={e}");
            }
        }
        Ok(())
    }

    /// Java `RabbitMqConsumer#refundSettlementMessage`：售后完成后的店铺退款结算。
    pub async fn refund_settlement(&self, aftersale_id: i32) -> Result<()> {
        let Some((order_id, refund_amount)) = sqlx::query_as::<_, (i32, Option<Decimal>)>(
            "SELECT order_id, refund_amount FROM aftersales WHERE aftersale_id = $1",
        )
        .bind(aftersale_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?
        else {
            tracing::error!("结算级联：售后不存在 aftersale={aftersale_id}");
            return Ok(());
        };
        let Some(order) = self.repos.trade.order.find_by_id(order_id).await? else {
            tracing::error!("结算级联：退款结算订单不存在 order={order_id}");
            return Ok(());
        };
        // 订单尚未结算时不做退款结算（Java 同款：结算后退款才扣回）
        if order.is_settlement.unwrap_or(0) == 0 {
            return Ok(());
        }
        let shop_row = sqlx::query_as::<_, (Option<Decimal>, Option<Decimal>)>(
            "SELECT shop_money, frozen_money FROM shop WHERE shop_id = $1",
        )
        .bind(order.shop_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        let Some((shop_money, frozen_money)) = shop_row else {
            return Ok(());
        };

        let vendor_refund = self.vendor_refund_amount(order_id, Some(aftersale_id)).await?;
        let mut shop_refund = Decimal::ZERO;
        let rows: Vec<Option<Decimal>> = sqlx::query_scalar(
            "SELECT refund_amount FROM aftersales \
             WHERE order_id = $1 AND aftersale_id = $2 AND status = $3",
        )
        .bind(order_id)
        .bind(aftersale_id)
        .bind(AFTERSALES_COMPLETE)
        .fetch_all(self.pool())
        .await
        .map_err(db_err)?;
        for refund in rows {
            shop_refund += refund.unwrap_or(Decimal::ZERO);
            if vendor_refund > Decimal::ZERO {
                shop_refund -= vendor_refund;
            }
        }
        // 待结算佣金返还店铺（减少店铺扣减额）
        let pending: Decimal = sqlx::query_scalar(
            "SELECT COALESCE(SUM(expenditure), 0) FROM statement_output \
             WHERE record_id = $1 AND record_type = $2 AND real_settlement = 0",
        )
        .bind(order_id)
        .bind(statement_type::DISTRIBUTION_COMMISSION)
        .fetch_one(self.pool())
        .await
        .map_err(db_err)?;
        if pending > Decimal::ZERO {
            shop_refund -= pending;
        }

        if shop_refund == Decimal::ZERO {
            return Ok(());
        }
        if let Some(online_amount) = self.resolve_online_split(&order).await? {
            let decr = shop_money_decr_on_refund(shop_refund, order.paid_amount, online_amount);
            if decr != Decimal::ZERO {
                self.debit_shop(&order, shop_money, frozen_money, decr).await?;
            }
        } else {
            let fee = if shop_refund > Decimal::ZERO {
                let rate = self.shop_service_fee(order_id).await?;
                (shop_refund * rate / Decimal::from(100)).round_dp_with_strategy(
                    2,
                    RoundingStrategy::MidpointAwayFromZero,
                )
            } else {
                Decimal::ZERO
            };
            self.debit_shop(&order, shop_money, frozen_money, shop_refund - fee)
                .await?;
        }
        Ok(())
    }
}

// ————————————————————— 纯函数（Java 对照移植，可单测） —————————————————————

/// Java `ProfitSharingChannelResolver#resolvePayChannel`：payCode → 分账渠道。
/// 非二级户分账渠道（拉卡拉 / 未知）返回 \"None\"。
#[must_use]
pub fn resolve_pay_channel(pay_code: &str) -> Option<&'static str> {
    let code = pay_code.trim().to_ascii_lowercase();
    if code.starts_with("huifu") {
        Some("huifu")
    } else if code.starts_with("alipay") {
        Some("alipay")
    } else if code.starts_with("wechat") {
        Some("wechat")
    } else {
        None
    }
}

/// Java \"OnlineSplitSettlementMath\" 的 \"balancePortion\"：余额 portion =
/// max(0, paidAmount − 二级户实收)。
fn balance_portion(paid: Decimal, online: Decimal) -> Decimal {
    (paid - online).max(Decimal::ZERO)
}

/// Java \"shopMoneyNetOnSettle\"：结算时余额份额净入账，退款优先冲二级户实收。
#[must_use]
pub fn shop_money_net_on_settle(paid: Decimal, online: Decimal, total_refund: Decimal) -> Decimal {
    let balance = balance_portion(paid, online);
    if balance <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let balance_refund = (total_refund - online).max(Decimal::ZERO);
    (balance - balance_refund).max(Decimal::ZERO)
}

/// Java \"shopMoneyDecrOnRefund\"：退款中按余额占比分摊的 shopMoney 扣减额。
#[must_use]
pub fn shop_money_decr_on_refund(
    shop_refund: Decimal,
    paid: Decimal,
    online: Decimal,
) -> Decimal {
    if shop_refund == Decimal::ZERO {
        return Decimal::ZERO;
    }
    let balance = balance_portion(paid, online);
    if balance <= Decimal::ZERO || paid <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    (shop_refund * balance / paid).round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

/// Java 升级条件解析：`condition.<key>.checked == true` 时取 \"value\" 数值门槛。
fn condition_threshold(condition: &Value, key: &str) -> Option<Decimal> {
    let node = condition.get(key)?;
    if node.get("checked").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    node.get("value")
        .and_then(Value::as_f64)
        .and_then(|v| Decimal::try_from(v).ok())
}

fn db_err(e: sqlx::Error) -> nextrade_core::Error {
    nextrade_core::Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::FromPrimitive;

    fn d(v: impl Into<f64>) -> Decimal {
        Decimal::from_f64(v.into()).unwrap()
    }

    #[test]
    fn pay_channel_matches_java_resolver() {
        assert_eq!(resolve_pay_channel("wechatJsApi"), Some("wechat"));
        assert_eq!(resolve_pay_channel("WECHAT_H5"), Some("wechat"));
        assert_eq!(resolve_pay_channel("alipayApp"), Some("alipay"));
        assert_eq!(resolve_pay_channel("huifuPc"), Some("huifu"));
        // 拉卡拉一期平台统一收款、不分账
        assert_eq!(resolve_pay_channel("lakalaQr"), None);
        assert_eq!(resolve_pay_channel("balance"), None);
        assert_eq!(resolve_pay_channel(""), None);
    }

    #[test]
    fn net_on_settle_refund_offsets_online_first() {
        // 全额线上：余额份额 0 → 不动 shopMoney
        assert_eq!(shop_money_net_on_settle(d(100.0), d(100.0), d(0.0)), d(0));
        // 半线上：余额 50，退款 20 优先冲线上（online=50 ≥ 20）→ 余额净入 50
        assert_eq!(shop_money_net_on_settle(d(100.0), d(50.0), d(20.0)), d(50));
        // 退款 60 超出线上 50 → 余额再扣 10 → 净入 40
        assert_eq!(shop_money_net_on_settle(d(100.0), d(50.0), d(60.0)), d(40));
    }

    #[test]
    fn decr_on_refund_pro_rata_by_balance_share() {
        // 退款 30，线上 50/支付 100 → 余额分摊 15
        assert_eq!(shop_money_decr_on_refund(d(30.0), d(100.0), d(50.0)), d(15));
        assert_eq!(shop_money_decr_on_refund(d(0.0), d(100.0), d(50.0)), d(0));
        // 全线上 → 余额份额 0 不扣
        assert_eq!(shop_money_decr_on_refund(d(30.0), d(100.0), d(100.0)), d(0));
    }

    #[test]
    fn threshold_only_when_checked() {
        let cond: Value = serde_json::json!({
            "salesAmount": { "checked": true, "value": 1000 },
            "salesInviteUsers": { "checked": false, "value": 50 },
            "selfBuyAmount": { "checked": true, "value": 200.5 }
        });
        assert_eq!(condition_threshold(&cond, "salesAmount"), Some(d(1000.0)));
        assert_eq!(condition_threshold(&cond, "salesInviteUsers"), None);
        assert_eq!(
            condition_threshold(&cond, "selfBuyAmount"),
            Some(d(200.5))
        );
        assert_eq!(condition_threshold(&cond, "missing"), None);
    }
}
