//! 账期结算域服务（payment::credit_settlement）。
//!
//! 对应原 Java `CreditSettlementController` 的 6 个端点 + 其依赖的
//! `CreditSettlementServiceImpl` / `CreditOrderServiceImpl#list` /
//! `CreditUserServiceImpl{restoreAmount,deductAmount,resetPeriodStartIfNoPending}`。
//!
//! # 已完整实现
//!
//! - `userList`：授信会员分页 + 用户/企业认证装配 + 待结算统计 + 账期窗口
//!   （`settlementDays` / `remainingDays`，含逾期判定）。
//! - `pendingOrders`：强制 `settlement_status IN (0, 3)` 的账期订单分页。
//! - `submit`：流水号唯一、逐单校验（存在/归属/仅待结算）、汇总金额、写结算
//!   记录 → 批量置已结算 → 恢复授信额度 → 无待结算时初始化周期起点，**同一事务**。
//! - `history`：历史结算分页（会员/流水号/状态/结算时间范围）+ 用户名/状态文案/
//!   时间格式化。
//! - `historyOrders`：按 `settlementId` 查订单明细（缺失时 500「结算记录ID不能为空」）。
//! - `cancel`：状态校验 → 当账期校验（月结同年月 / 季结同年同季）→ 撤销结算记录
//!   → 订单回待结算 → 扣减授信额度，**同一事务**。
//! - `refundedAmount`：复刻 Java 退款金额装配（财务审核通过 + 售后类型过滤 +
//!   运费退还规则：待发货仅退款、待发货全部商品售后、已完成未发货全部售后）。
//!
//! # 偏差（如实标注）
//!
//! 1. **日期参数**：非法日期返回 400 校验错误（Java 抛 `DateTimeParseException`）。
//! 2. **`@Sensitive(PHONE)`**：仅 `nextrade.is-demo=1` 脱敏，非 demo 原样返回；
//!    Rust 侧无 demo 开关，统一返回原文（与默认生产行为一致）。
//! 3. **排序**：Java 两层 ORDER BY（`buildSortOrder` + 时间列 DESC）；本实现以
//!   时间列 DESC 为主序、`sortField` 为次序。
//! 4. **`userList` 关键字**：按 Java 实现只匹配 `user.username ILIKE`（DTO 注释
//!    提到手机号但代码未实现手机号匹配）。
//! 5. **企业认证 `limit 1`**：Java 无 ORDER BY（不确定行）；本实现固定取
//!    `id` 最小的一行（`ORDER BY user_id, id`）以保证分页稳定。
//! 6. `operatorName`：按 Java 查 `admin_user.username`，查不到回退「管理员」。
//! 7. `submit` 的前置校验（流水号重复/订单归属/状态）在事务外读取，写入在事务内；
//!    Java 整体在事务内，等价性仅受并发窗口影响（同一流水号并发提交由唯一索引
//!    兜底，本切片未新增索引）。
//! 8. 退款金额的运费判定只读 `order/order_item/aftersales/aftersales_item`，
//!    与 Java 一致；`refund_apply` 仅取 `refund_status = 2`。

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Datelike;
use rust_decimal::Decimal;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::{CreditOrder, CreditSettlement, CreditUser};
use nextrade_core::models::payment::credit_settlement::{
    base_start_date, date_start_epoch, epoch_to_date, format_time, order_not_settleable_message,
    order_settlement_status_text, order_status_text, period_start_date, remaining_days,
    settlement_status_text, settlement_type_days, settlement_type_text, today_local,
    user_company_type_text, CreditOrderListQuery, CreditOrderVO, CreditSettlementCancelDTO,
    CreditSettlementHistoryQuery, CreditSettlementSubmitDTO, CreditSettlementUserListQuery,
    CreditSettlementUserVO, CreditSettlementVO, AFTERSALES_TYPE_PAYRETURN,
    FALLBACK_OPERATOR_NAME, MSG_CANCEL_NOT_CURRENT_PERIOD, MSG_CREDIT_AMOUNT_INSUFFICIENT,
    MSG_CREDIT_USER_NOT_FOUND, MSG_ORDER_NOT_BELONG, MSG_ORDER_NOT_CREDIT,
    MSG_RESTORE_AMOUNT_FAILED, MSG_SETTLEMENT_ALREADY_CANCELLED, MSG_SETTLEMENT_ID_REQUIRED,
    MSG_SETTLEMENT_NOT_FOUND, MSG_SETTLEMENT_SN_EXISTS, ORDER_MAIN_STATUS_COMPLETED,
    ORDER_STATUS_SETTLED, ORDER_STATUS_UNSETTLED, SETTLEMENT_STATUS_SETTLED, SETTLEMENT_TYPE_MONTHLY, SHIPPING_STATUS_PENDING,
    SHIPPING_STATUS_SHIPPED,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::payment::credit_settlement::{
    AftersalesItemRow, CompanyRow, CreditSettlementRepository, OrderBriefRow, OrderItemRow, UserRow,
};
use nextrade_infrastructure::db::Repositories;

/// 账期结算域服务。
pub struct CreditSettlementService {
    repos: Arc<Repositories>,
}

impl CreditSettlementService {
    /// 构造服务（组合根唯一装配点）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &CreditSettlementRepository {
        &self.repos.payment.credit_settlement
    }

    // ————————————————————— 1) userList —————————————————————

    /// 结算会员列表（Java `getUserList`）。
    pub async fn user_list(
        &self,
        q: &CreditSettlementUserListQuery,
    ) -> Result<PageResult<CreditSettlementUserVO>> {
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let user_ids = match q.effective_keyword() {
            Some(keyword) => Some(self.repo().find_user_ids_by_username_like(keyword).await?),
            None => None,
        };
        if let Some(ids) = &user_ids {
            if ids.is_empty() {
                return Ok(PageResult::empty(&page));
            }
        }
        let (total, rows) = self
            .repo()
            .paginate_credit_users(user_ids.as_deref(), limit, offset)
            .await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let uids: Vec<i32> = distinct(rows.iter().map(|u| u.user_id));
        let users: HashMap<i32, UserRow> = self
            .repo()
            .find_users(&uids)
            .await?
            .into_iter()
            .map(|u| (u.user_id, u))
            .collect();
        let companies: HashMap<i32, CompanyRow> = self
            .repo()
            .find_companies(&uids)
            .await?
            .into_iter()
            .map(|c| (c.user_id, c))
            .collect();
        let stats: HashMap<i32, (i64, Decimal)> = self
            .repo()
            .pending_stats(&uids)
            .await?
            .into_iter()
            .map(|(user_id, count, amount)| (user_id, (count, amount)))
            .collect();
        let min_times: HashMap<i32, Option<i64>> = self
            .repo()
            .min_pending_order_time(&uids)
            .await?
            .into_iter()
            .collect();
        let today = today_local();
        let list: Vec<CreditSettlementUserVO> = rows
            .iter()
            .map(|credit_user| {
                to_user_vo(
                    credit_user,
                    &users,
                    &companies,
                    &stats,
                    &min_times,
                    today,
                )
            })
            .collect();
        Ok(PageResult::new(
            u64::try_from(total).unwrap_or(0),
            list,
            &page,
        ))
    }

    // ————————————————————— 2) pendingOrders —————————————————————

    /// 待结算订单列表（Java controller 强制 `[0, 3]` 后走 `CreditOrderService#list`）。
    pub async fn pending_orders(
        &self,
        q: &CreditOrderListQuery,
    ) -> Result<PageResult<CreditOrderVO>> {
        let q = q.clone().for_pending_orders();
        self.order_page(&q).await
    }

    // ————————————————————— 3) submit —————————————————————

    /// 确认结算（Java `CreditSettlementServiceImpl#submit`）。
    pub async fn submit(
        &self,
        dto: &CreditSettlementSubmitDTO,
        admin_id: i32,
    ) -> Result<()> {
        let input = dto.validate().map_err(Error::Validation)?;

        // 1) 流水单号重复校验
        if self
            .repo()
            .find_settlement_by_sn(&input.settlement_sn)
            .await?
            .is_some()
        {
            return Err(Error::biz(500, MSG_SETTLEMENT_SN_EXISTS));
        }

        // 2) 逐单校验并汇总金额
        let mut total_amount = Decimal::ZERO;
        for order_id in &input.order_ids {
            let credit_order = self
                .repo()
                .find_credit_order_by_order_id(*order_id)
                .await?
                .ok_or_else(|| Error::biz(500, MSG_ORDER_NOT_CREDIT))?;
            if credit_order.user_id != input.user_id {
                return Err(Error::biz(500, MSG_ORDER_NOT_BELONG));
            }
            if credit_order.settlement_status != ORDER_STATUS_UNSETTLED {
                return Err(Error::biz(
                    500,
                    order_not_settleable_message(
                        &credit_order.order_sn,
                        credit_order.settlement_status,
                    ),
                ));
            }
            total_amount += credit_order.credit_amount;
        }

        // 3) 授信会员必须存在（Java `restoreAmount` 首行校验）
        let credit_user = self
            .repo()
            .find_credit_user_by_user_id(input.user_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CREDIT_USER_NOT_FOUND))?;

        let now = now_seconds();
        let operator_name = self
            .repo()
            .find_admin_username(admin_id)
            .await?
            .unwrap_or_else(|| FALLBACK_OPERATOR_NAME.to_string());
        let entity = CreditSettlement {
            settlement_id: 0,
            settlement_sn: input.settlement_sn.clone(),
            user_id: input.user_id,
            credit_user_id: credit_user.credit_user_id,
            settlement_amount: total_amount,
            order_count: i32::try_from(input.order_ids.len()).unwrap_or(i32::MAX),
            remark: input.remark.clone(),
            settlement_time: Some(input.settlement_time),
            settlement_status: SETTLEMENT_STATUS_SETTLED,
            cancel_time: None,
            cancel_reason: None,
            operator_id: Some(admin_id),
            operator_name: Some(operator_name),
            create_time: Some(now),
            update_time: Some(now),
        };

        // 4) 事务：写结算记录 → 订单置已结算 → 恢复额度 → 周期起点
        let mut tx = self.repo().begin().await?;
        let settlement_id = self.repo().insert_settlement_in_tx(&mut tx, &entity).await?;
        self.repo()
            .update_orders_settled_in_tx(
                &mut tx,
                &input.order_ids,
                settlement_id,
                &input.settlement_sn,
                now,
            )
            .await?;
        let restored = self
            .repo()
            .restore_amount_in_tx(&mut tx, input.user_id, total_amount, now)
            .await?;
        if restored == 0 {
            return Err(Error::biz(500, MSG_RESTORE_AMOUNT_FAILED));
        }
        // Java `resetPeriodStartIfNoPending`：无待结算/退款中订单时初始化周期起点
        if self.repo().count_pending_in_tx(&mut tx, input.user_id).await? == 0 {
            self.repo()
                .reset_period_start_in_tx(
                    &mut tx,
                    input.user_id,
                    date_start_epoch(today_local()),
                    now,
                )
                .await?;
        }
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        tracing::info!(
            "账期结算成功: settlement_id={settlement_id}, user_id={}, orders={}, amount={}",
            input.user_id,
            input.order_ids.len(),
            total_amount
        );
        Ok(())
    }

    // ————————————————————— 4) history —————————————————————

    /// 历史结算列表（Java `history`）。
    pub async fn history(
        &self,
        q: &CreditSettlementHistoryQuery,
    ) -> Result<PageResult<CreditSettlementVO>> {
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let start = q.time_start_epoch().map_err(Error::Validation)?;
        let end = q.time_end_epoch().map_err(Error::Validation)?;
        let (total, rows) = self
            .repo()
            .paginate_history(q, start, end, limit, offset)
            .await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let user_ids = distinct(rows.iter().map(|s| s.user_id));
        let usernames = self.repo().find_usernames(&user_ids).await?;
        let list: Vec<CreditSettlementVO> = rows
            .iter()
            .map(|settlement| to_settlement_vo(settlement, &usernames))
            .collect();
        Ok(PageResult::new(
            u64::try_from(total).unwrap_or(0),
            list,
            &page,
        ))
    }

    // ————————————————————— 5) historyOrders —————————————————————

    /// 历史结算订单明细（Java controller：先要求 `settlementId` 非空）。
    pub async fn history_orders(
        &self,
        q: &CreditOrderListQuery,
    ) -> Result<PageResult<CreditOrderVO>> {
        if q.settlement_id.is_none() {
            return Err(Error::biz(500, MSG_SETTLEMENT_ID_REQUIRED));
        }
        self.order_page(q).await
    }

    // ————————————————————— 6) cancel —————————————————————

    /// 撤销结算（Java `CreditSettlementServiceImpl#cancel`）。
    pub async fn cancel(&self, dto: &CreditSettlementCancelDTO) -> Result<()> {
        let (settlement_id, cancel_reason) = dto.validate().map_err(Error::Validation)?;
        let settlement = self
            .repo()
            .find_settlement_by_id(settlement_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_SETTLEMENT_NOT_FOUND))?;
        if settlement.settlement_status != SETTLEMENT_STATUS_SETTLED {
            return Err(Error::biz(500, MSG_SETTLEMENT_ALREADY_CANCELLED));
        }
        let credit_user = self
            .repo()
            .find_credit_user_by_id(settlement.credit_user_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CREDIT_USER_NOT_FOUND))?;
        // 校验是否在当前账期内（月结：同年同月；季结：同年同季）
        let settlement_date = settlement.settlement_time.map(epoch_to_date);
        let today = today_local();
        let in_current_period = match settlement_date {
            None => false,
            Some(date) => {
                if credit_user.settlement_type == SETTLEMENT_TYPE_MONTHLY {
                    date.year() == today.year() && date.month() == today.month()
                } else {
                    let quarter = (date.month() - 1) / 3;
                    let today_quarter = (today.month() - 1) / 3;
                    date.year() == today.year() && quarter == today_quarter
                }
            }
        };
        if !in_current_period {
            return Err(Error::biz(500, MSG_CANCEL_NOT_CURRENT_PERIOD));
        }

        let now = now_seconds();
        let mut tx = self.repo().begin().await?;
        self.repo()
            .cancel_settlement_in_tx(
                &mut tx,
                settlement_id,
                cancel_reason.as_deref(),
                now,
                now,
            )
            .await?;
        self.repo()
            .cancel_orders_in_tx(&mut tx, settlement_id, now)
            .await?;
        let deducted = self
            .repo()
            .deduct_amount_in_tx(&mut tx, settlement.user_id, settlement.settlement_amount, now)
            .await?;
        if deducted == 0 {
            return Err(Error::biz(500, MSG_CREDIT_AMOUNT_INSUFFICIENT));
        }
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        tracing::info!(
            "撤销账期结算: settlement_id={settlement_id}, user_id={}, amount={}",
            settlement.user_id,
            settlement.settlement_amount
        );
        Ok(())
    }

    // ————————————————————— 内部：账期订单分页 —————————————————————

    async fn order_page(&self, q: &CreditOrderListQuery) -> Result<PageResult<CreditOrderVO>> {
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let start = q.time_start_epoch().map_err(Error::Validation)?;
        let end = q.time_end_epoch().map_err(Error::Validation)?;

        // 用户名 / 手机号 → 会员 ID（两者同时存在时为 AND，Java 同一 wrapper）
        let username = q.effective(&q.username);
        let mobile = q.effective(&q.mobile);
        let user_ids = if username.is_some() || mobile.is_some() {
            Some(
                self.repo()
                    .find_user_ids_by_name_or_mobile(username, mobile)
                    .await?,
            )
        } else {
            None
        };
        if let Some(ids) = &user_ids {
            if ids.is_empty() {
                return Ok(PageResult::empty(&page));
            }
        }
        // 订单状态 → 订单 ID
        let order_ids = match q.order_status {
            Some(status) => {
                let ids = self.repo().find_order_ids_by_status(status).await?;
                if ids.is_empty() {
                    return Ok(PageResult::empty(&page));
                }
                Some(ids)
            }
            None => None,
        };

        let (total, rows) = self
            .repo()
            .paginate_credit_orders(
                q,
                user_ids.as_deref(),
                order_ids.as_deref(),
                start,
                end,
                limit,
                offset,
            )
            .await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let list = self.enrich_credit_orders(&rows).await?;
        Ok(PageResult::new(
            u64::try_from(total).unwrap_or(0),
            list,
            &page,
        ))
    }

    // ————————————————————— 内部：订单 VO 装配 —————————————————————

    async fn enrich_credit_orders(&self, rows: &[CreditOrder]) -> Result<Vec<CreditOrderVO>> {
        let order_ids = distinct(rows.iter().map(|o| o.order_id));
        let user_ids = distinct(rows.iter().map(|o| o.user_id));
        let users: HashMap<i32, UserRow> = self
            .repo()
            .find_users(&user_ids)
            .await?
            .into_iter()
            .map(|u| (u.user_id, u))
            .collect();
        // Java `convertToVO` 的企业名称查询**不带 status 过滤**
        let companies: HashMap<i32, CompanyRow> = self
            .repo()
            .find_companies_any(&user_ids)
            .await?
            .into_iter()
            .map(|c| (c.user_id, c))
            .collect();
        let orders: HashMap<i32, OrderBriefRow> = self
            .repo()
            .find_order_briefs(&order_ids)
            .await?
            .into_iter()
            .map(|o| (o.order_id, o))
            .collect();
        let refunded = self.refunded_amount_map(&order_ids).await?;
        Ok(rows
            .iter()
            .map(|credit_order| {
                let user = users.get(&credit_order.user_id);
                let company = companies.get(&credit_order.user_id);
                let order = orders.get(&credit_order.order_id);
                CreditOrderVO {
                    credit_order_id: credit_order.credit_order_id,
                    order_id: credit_order.order_id,
                    order_sn: credit_order.order_sn.clone(),
                    user_id: credit_order.user_id,
                    username: user.map(|u| u.username.clone()),
                    mobile: user.map(|u| u.mobile.clone()),
                    company_name: company.map(|c| c.company_name.clone()),
                    order_amount: order.map(|o| o.total_amount),
                    credit_amount: credit_order.credit_amount,
                    refunded_amount: refunded
                        .get(&credit_order.order_id)
                        .copied()
                        .unwrap_or(Decimal::ZERO),
                    order_status: order.map(|o| o.order_status),
                    order_status_text: order.map(|o| order_status_text(o.order_status).to_string()),
                    settlement_status: credit_order.settlement_status,
                    settlement_status_text: order_settlement_status_text(
                        credit_order.settlement_status,
                    )
                    .to_string(),
                    settlement_sn: credit_order.settlement_sn.clone(),
                    order_time: format_time(credit_order.order_time),
                    settlement_time: credit_order.settlement_time.map(|t| format_time(Some(t))),
                }
            })
            .collect())
    }

    /// 退款金额映射（Java `getRefundedAmountMap`，含运费退还规则）。
    async fn refunded_amount_map(&self, order_ids: &[i32]) -> Result<HashMap<i32, Decimal>> {
        let mut result: HashMap<i32, Decimal> = HashMap::new();
        let refunds = self.repo().find_approved_refunds(order_ids).await?;
        if refunds.is_empty() {
            return Ok(result);
        }
        let aftersale_ids = distinct(refunds.iter().map(|r| r.aftersale_id));
        let aftersales: HashMap<i32, _> = self
            .repo()
            .find_aftersales_by_ids(&aftersale_ids)
            .await?
            .into_iter()
            .map(|a| (a.aftersale_id, a))
            .collect();
        let orders: HashMap<i32, OrderBriefRow> = self
            .repo()
            .find_order_briefs(order_ids)
            .await?
            .into_iter()
            .map(|o| (o.order_id, o))
            .collect();
        let order_items: HashMap<i32, Vec<OrderItemRow>> =
            group_by(self.repo().find_order_items(order_ids).await?, |i| i.order_id);
        let completed = self.repo().find_completed_aftersales(order_ids).await?;
        let completed_ids: Vec<i32> = completed.iter().map(|(_, id)| *id).collect();
        let completed_items = self.repo().find_aftersales_items(&completed_ids).await?;
        // order_item_id → order_id
        let item_to_order: HashMap<i32, i32> = order_items
            .values()
            .flatten()
            .map(|i| (i.item_id, i.order_id))
            .collect();
        let mut items_by_order: HashMap<i32, Vec<AftersalesItemRow>> = HashMap::new();
        for item in completed_items {
            if let Some(order_id) = item
                .order_item_id
                .and_then(|item_id| item_to_order.get(&item_id).copied())
            {
                items_by_order.entry(order_id).or_default().push(item);
            }
        }

        // 按订单分组
        let mut refunds_by_order: HashMap<i32, Vec<i32>> = HashMap::new();
        for refund in &refunds {
            if aftersales.contains_key(&refund.aftersale_id) {
                refunds_by_order
                    .entry(refund.order_id)
                    .or_default()
                    .push(refund.aftersale_id);
            }
        }
        for (order_id, aftersale_ids) in refunds_by_order {
            let Some(order) = orders.get(&order_id) else {
                continue;
            };
            let empty_items: Vec<OrderItemRow> = Vec::new();
            let order_items = order_items.get(&order_id).unwrap_or(&empty_items);
            let empty_completed: Vec<AftersalesItemRow> = Vec::new();
            let completed_items = items_by_order.get(&order_id).unwrap_or(&empty_completed);
            let mut total = Decimal::ZERO;
            let mut shipping_refunded = false;
            for aftersale_id in aftersale_ids {
                let Some(aftersale) = aftersales.get(&aftersale_id) else {
                    continue;
                };
                total += aftersale.refund_amount;
                if !shipping_refunded && order.shipping_fee > Decimal::ZERO {
                    if should_refund_shipping_fee(
                        aftersale.aftersale_type,
                        order,
                        order_items,
                        completed_items,
                    ) {
                        total += order.shipping_fee;
                        shipping_refunded = true;
                    }
                }
            }
            result.insert(order_id, total);
        }
        Ok(result)
    }
}

// ——————————————————————————————————————————————
// 纯函数（单测覆盖）
// ——————————————————————————————————————————————

/// 是否退还运费（Java `shouldRefundShippingFee`）。
#[must_use]
pub fn should_refund_shipping_fee(
    aftersale_type: i32,
    order: &OrderBriefRow,
    order_items: &[OrderItemRow],
    completed_items: &[AftersalesItemRow],
) -> bool {
    if order_items.is_empty() {
        return false;
    }
    let is_pending = order.shipping_status == SHIPPING_STATUS_PENDING;
    let is_pay_return = aftersale_type == AFTERSALES_TYPE_PAYRETURN;
    // 待发货 + 仅退款：退运费
    if is_pending && is_pay_return {
        return true;
    }
    // 待发货 + 非仅退款：全部商品都售后才退
    if is_pending {
        return is_all_items_aftersales(order_items, completed_items);
    }
    // 已完成 + 已收货（未发过货，如虚拟商品）：全部商品都售后才退
    if order.order_status == ORDER_MAIN_STATUS_COMPLETED
        && order.shipping_status == SHIPPING_STATUS_SHIPPED
        && order.shipping_time <= 0
    {
        return is_all_items_aftersales(order_items, completed_items);
    }
    false
}

/// 是否全部商品都已完成售后（Java `isAllItemsAftersales`）。
#[must_use]
pub fn is_all_items_aftersales(
    order_items: &[OrderItemRow],
    completed_items: &[AftersalesItemRow],
) -> bool {
    let mut by_item: HashMap<i32, i32> = HashMap::new();
    for item in completed_items {
        if let Some(item_id) = item.order_item_id {
            *by_item.entry(item_id).or_insert(0) += item.number.unwrap_or(0);
        }
    }
    order_items
        .iter()
        .all(|item| by_item.get(&item.item_id).copied().unwrap_or(0) == item.quantity)
}

/// 实体 → 结算会员 VO。
fn to_user_vo(
    credit_user: &CreditUser,
    users: &HashMap<i32, UserRow>,
    companies: &HashMap<i32, CompanyRow>,
    stats: &HashMap<i32, (i64, Decimal)>,
    min_times: &HashMap<i32, Option<i64>>,
    today: chrono::NaiveDate,
) -> CreditSettlementUserVO {
    let user = users.get(&credit_user.user_id);
    let company = companies.get(&credit_user.user_id);
    let (pending_count, pending_amount) = stats
        .get(&credit_user.user_id)
        .copied()
        .unwrap_or((0, Decimal::ZERO));
    let days = settlement_type_days(credit_user.settlement_type);
    let base = base_start_date(credit_user.period_start_time, credit_user.create_time);
    let period_start = period_start_date(base, today, days);
    let has_pending_before = min_times
        .get(&credit_user.user_id)
        .copied()
        .flatten()
        .map(|earliest| earliest < date_start_epoch(period_start))
        .unwrap_or(false);
    CreditSettlementUserVO {
        credit_user_id: credit_user.credit_user_id,
        user_id: credit_user.user_id,
        username: user.map(|u| u.username.clone()),
        mobile: user.map(|u| u.mobile.clone()),
        r#type: company.map(|c| c.r#type),
        type_text: company.map(|c| user_company_type_text(c.r#type).to_string()),
        contact_name: company.map(|c| c.contact_name.clone()),
        company_name: company.map(|c| c.company_name.clone()),
        credit_limit: credit_user.credit_limit,
        used_amount: credit_user.used_amount,
        available_amount: credit_user.available_amount,
        status: credit_user.status,
        status_text: settlement_status_text_for_credit_user(credit_user.status),
        settlement_type: credit_user.settlement_type,
        settlement_type_text: settlement_type_text(credit_user.settlement_type).to_string(),
        settlement_days: days,
        remaining_days: remaining_days(base, today, days, has_pending_before),
        pending_order_count: i32::try_from(pending_count).unwrap_or(i32::MAX),
        pending_amount,
    }
}

/// 授信会员状态文案（避免与结算状态文案函数重名混淆）。
fn settlement_status_text_for_credit_user(status: i32) -> String {
    nextrade_core::models::payment::credit_settlement::credit_user_status_text(status).to_string()
}

/// 实体 → 历史结算 VO（Java `BeanUtils.copyProperties` + 时间格式化）。
fn to_settlement_vo(
    settlement: &CreditSettlement,
    usernames: &HashMap<i32, String>,
) -> CreditSettlementVO {
    CreditSettlementVO {
        settlement_id: settlement.settlement_id,
        settlement_sn: settlement.settlement_sn.clone(),
        user_id: settlement.user_id,
        username: usernames.get(&settlement.user_id).cloned(),
        settlement_amount: settlement.settlement_amount,
        order_count: settlement.order_count,
        remark: settlement.remark.clone(),
        settlement_time: format_time(settlement.settlement_time),
        settlement_status: settlement.settlement_status,
        settlement_status_text: settlement_status_text(settlement.settlement_status).to_string(),
        cancel_time: settlement.cancel_time.map(|t| format_time(Some(t))),
        cancel_reason: settlement.cancel_reason.clone(),
        operator_name: settlement.operator_name.clone(),
        create_time: format_time(settlement.create_time),
    }
}

/// 去重（保持出现顺序）。
fn distinct(ids: impl Iterator<Item = i32>) -> Vec<i32> {
    let mut out: Vec<i32> = Vec::new();
    for id in ids {
        if !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// 分组。
fn group_by<T, K: std::hash::Hash + Eq>(items: Vec<T>, key: impl Fn(&T) -> K) -> HashMap<K, Vec<T>> {
    let mut map: HashMap<K, Vec<T>> = HashMap::new();
    for item in items {
        map.entry(key(&item)).or_default().push(item);
    }
    map
}

/// `BasePage` → `PageQuery`。
fn to_page_query(page: Option<i64>, size: Option<i64>) -> nextrade_core::types::PageQuery {
    let (size, offset) = nextrade_core::models::payment::credit_settlement::limit_offset(page, size);
    let page_no = if size > 0 { offset / size + 1 } else { 1 };
    nextrade_core::types::PageQuery {
        page: u64::try_from(page_no.max(1)).unwrap_or(1),
        page_size: u64::try_from(size.max(1)).unwrap_or(15),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::payment::credit_settlement::{
        format_time, MSG_CANCEL_ID_REQUIRED, MSG_DATE_INVALID,
    };

    fn order_brief(status: i32, shipping_status: i32, shipping_time: i32, fee: Decimal) -> OrderBriefRow {
        OrderBriefRow {
            order_id: 1,
            total_amount: Decimal::new(10000, 2),
            order_status: status,
            shipping_status,
            shipping_time,
            shipping_fee: fee,
        }
    }

    fn item(item_id: i32, quantity: i32) -> OrderItemRow {
        OrderItemRow {
            item_id,
            order_id: 1,
            quantity,
        }
    }

    fn completed(item_id: i32, number: i32) -> AftersalesItemRow {
        AftersalesItemRow {
            aftersale_id: Some(9),
            order_item_id: Some(item_id),
            number: Some(number),
        }
    }

    #[test]
    fn pending_orders_forced_filter_keeps_other_filters() {
        let q = CreditOrderListQuery {
            settlement_status: Some(ORDER_STATUS_SETTLED),
            settlement_id: Some(5),
            ..Default::default()
        }
        .for_pending_orders();
        assert_eq!(q.settlement_status_list, Some(vec![0, 3]));
        assert_eq!(q.settlement_status, Some(ORDER_STATUS_SETTLED));
        assert_eq!(q.settlement_id, Some(5));
    }

    #[test]
    fn shipping_refund_pending_and_pay_return() {
        let order = order_brief(1, SHIPPING_STATUS_PENDING, 0, Decimal::new(1000, 2));
        let items = vec![item(1, 2)];
        // 待发货 + 仅退款 ⇒ 退
        assert!(should_refund_shipping_fee(
            AFTERSALES_TYPE_PAYRETURN,
            &order,
            &items,
            &[]
        ));
        // 待发货 + 退货退款，未全部售后 ⇒ 不退
        assert!(!should_refund_shipping_fee(1, &order, &items, &[]));
        // 全部商品售后 ⇒ 退
        assert!(should_refund_shipping_fee(
            1,
            &order,
            &items,
            &[completed(1, 2)]
        ));
        // 无商品 ⇒ 不退
        assert!(!should_refund_shipping_fee(
            AFTERSALES_TYPE_PAYRETURN,
            &order,
            &[],
            &[]
        ));
    }

    #[test]
    fn shipping_refund_completed_virtual_goods() {
        let items = vec![item(1, 3), item(2, 1)];
        // 已完成 + 已收货 + shipping_time<=0 + 全部售后 ⇒ 退
        let order = order_brief(
            ORDER_MAIN_STATUS_COMPLETED,
            SHIPPING_STATUS_SHIPPED,
            0,
            Decimal::new(500, 2),
        );
        assert!(should_refund_shipping_fee(
            1,
            &order,
            &items,
            &[completed(1, 3), completed(2, 1)]
        ));
        // 部分售后 ⇒ 不退
        assert!(!should_refund_shipping_fee(
            1,
            &order,
            &items,
            &[completed(1, 3)]
        ));
        // 有发货时间 ⇒ 不退
        let shipped = order_brief(
            ORDER_MAIN_STATUS_COMPLETED,
            SHIPPING_STATUS_SHIPPED,
            1_700_000_000,
            Decimal::new(500, 2),
        );
        assert!(!should_refund_shipping_fee(
            1,
            &shipped,
            &items,
            &[completed(1, 3), completed(2, 1)]
        ));
    }

    #[test]
    fn all_items_aftersales_quantity_comparison() {
        let items = vec![item(1, 2), item(2, 1)];
        assert!(is_all_items_aftersales(
            &items,
            &[completed(1, 1), completed(1, 1), completed(2, 1)]
        ));
        assert!(!is_all_items_aftersales(&items, &[completed(1, 2)]));
        // 数量超出不算「全部」
        assert!(!is_all_items_aftersales(&items, &[completed(1, 3), completed(2, 1)]));
    }

    #[test]
    fn distinct_and_group_helpers() {
        assert_eq!(distinct([3, 1, 3, 2].into_iter()), vec![3, 1, 2]);
        let grouped = group_by(vec![(1, "a"), (1, "b"), (2, "c")], |pair| pair.0);
        assert_eq!(grouped.get(&1).map(Vec::len), Some(2));
        assert_eq!(grouped.get(&2).map(Vec::len), Some(1));
    }

    #[test]
    fn page_conversion_matches_base_page() {
        let p = to_page_query(None, None);
        assert_eq!((p.page, p.page_size), (1, 15));
        let p = to_page_query(Some(3), Some(20));
        assert_eq!((p.page, p.page_size), (3, 20));
        let p = to_page_query(Some(1), Some(500));
        assert_eq!((p.page, p.page_size), (1, 100));
    }

    #[test]
    fn validation_error_texts_are_verbatim() {
        let cancel =
            CreditSettlementCancelDTO::default().validate().unwrap_err();
        assert_eq!(cancel, MSG_CANCEL_ID_REQUIRED);
        let history = CreditSettlementHistoryQuery {
            settlement_time_end: Some("2026-13-01".into()),
            ..Default::default()
        };
        assert_eq!(history.time_end_epoch().unwrap_err(), MSG_DATE_INVALID);
        assert_eq!(format_time(Some(0)), "");
    }
}
