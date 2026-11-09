//! 保证金退还域服务（merchant::deposit_refund）。
//!
//! 对应原 Java `com.nextrade.service.deposit.impl.DepositRefundServiceImpl`（750 行）
//! 与管理端 `DepositRefundController` 的 7 个端点。
//!
//! # 迁移要点（逐条对照 Java 源）
//!
//! - `apply` / `vendorApply`（店铺/供应商共用内部逻辑）：
//!   持有者存在性 → 金额 > 0 → 退还类型 ∈ {1 部分退还, 2 全额退还} →
//!   全额退还且保证金功能开启时校验 5 个前置条件 → 可退上限
//!   （`有效可退 = max(0, 可提现 - 待审核退还总额)`）→ 落库（状态 0 待审核）。
//! - `fullRefundCheck` / `vendorFullRefundCheck`：只读预检，逐条收集未满足原因
//!   （与申请校验同口径，但条件 4 的天数取「售后限制天数」而非冷静期天数）。
//! - `list` / `detail`：平台端退还单列表/详情（owner 分段筛选 + 白名单排序）。
//! - `audit`（Java `@Transactional`，**一步式**）：仅待审核可审；通过时
//!   打款凭证必填 → 账户出账（`paid -= amount`）+ 保证金流水 `type = 2 提现出账`
//!   + 单据置「已通过」；驳回 → 单据置「未通过」。
//!
//! # 未迁移 / 偏差（见 `uncertain_points`）
//!
//! - `DepositAccountServiceImpl#computeRequired` 的「按类目规则取最高应缴」
//!   （`DepositRuleMatcher`）与「未激活收取 → 应缴 0」分支未迁移；
//!   本实现沿用同域 `deposit_account` 口径：`required_manual` 优先、
//!   否则 `required_snapshot`；功能关闭时应缴按 0。
//! - 审核通过的 `depositLifecycleService` 相关副作用：Java 退还审计只做
//!   `changeDeposit(WITHDRAW)`，无生命周期调用，故无需迁移。
//! - 金额出参为两位小数文本（Java 为 JSON number）。
//! - Java `SecurityUtils.getCurrentAdminId()` / `HeaderUtils` 由路由层注入。

use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_common::utils::now_seconds;
use nextrade_core::entities::{ShopDepositLog, ShopDepositRefund};
use nextrade_core::models::merchant::deposit_refund::{
    after_sales_limit_days, cooling_days, deposit_change, effective_refundable, format_time,
    is_deposit_enabled, owner_type_text, page_query, refund_status_text, refund_type_text,
    required_of, shop_is_closed, subject_type_text, vendor_is_closed, withdrawable,
    DepositFullRefundCheckVO, DepositRefundApplyDTO, DepositRefundAuditDTO, DepositRefundListQuery,
    DepositRefundVO, AFTERSALES_CANCEL, AFTERSALES_COMPLETE, CONFIG_AFTER_SALES_LIMIT_DAYS,
    CONFIG_DEPOSIT_ENABLE, CONFIG_REFUND_COOLING_DAYS, DEPOSIT_AMOUNT_INVALID,
    DEPOSIT_REFUND_COOLDOWN_NOT_REACHED, DEPOSIT_REFUND_HAS_ONGOING_AFTERSALE,
    DEPOSIT_REFUND_HAS_ONGOING_ORDER, DEPOSIT_REFUND_HAS_ONGOING_REFUND,
    DEPOSIT_REFUND_SHOP_NOT_CLOSED, DEPOSIT_REFUND_STATUS_INVALID, DEPOSIT_WITHDRAW_EXCEED_LIMIT,
    LOG_TYPE_WITHDRAW, MSG_AMOUNT_INVALID, MSG_BALANCE_NOT_ENOUGH, MSG_COOLDOWN_NOT_REACHED,
    MSG_EXCEED_LIMIT, MSG_HAS_ONGOING_AFTERSALE, MSG_HAS_ONGOING_ORDER, MSG_HAS_ONGOING_REFUND,
    MSG_OWNER_NOT_EXIST, MSG_PAY_VOUCHER_REQUIRED, MSG_REFUND_NOT_EXIST, MSG_REFUND_STATUS_INVALID,
    MSG_REFUND_TYPE_INVALID, MSG_SHOP_NOT_CLOSED, MSG_VENDOR_NOT_CLOSED, OWNER_SHOP, OWNER_SUPPLIER,
    REFUND_COMPLETED, REFUND_PENDING, REFUND_REJECTED, REFUND_TYPE_FULL, SECONDS_PER_DAY,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 「预检/申请共用」的条件 1 文案（Java 两处不同）：
/// - 申请校验：`全额退还需店铺停业` / `全额退还需供应商已停业关闭`
/// - 预检：`店铺未停业` / `供应商未停业关闭`
pub const MSG_SHOP_NOT_CLOSED_CHECK: &str = "店铺未停业";
/// 预检条件 1 文案（供应商）。
pub const MSG_VENDOR_NOT_CLOSED_CHECK: &str = "供应商未停业关闭";

/// 保证金退还域服务。
pub struct DepositRefundService {
    repos: Arc<Repositories>,
}

impl DepositRefundService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 申请 —————————————————————

    /// 店铺端发起退还/提现申请（Java `apply(shopId, applyDTO)`）。
    pub async fn apply(&self, shop_id: Option<i32>, dto: &DepositRefundApplyDTO) -> Result<i32> {
        self.apply_internal(OWNER_SHOP, shop_id, dto).await
    }

    /// 供应商端发起退还/提现申请（Java `vendorApply(vendorId, applyDTO)`）。
    pub async fn vendor_apply(
        &self,
        vendor_id: Option<i32>,
        dto: &DepositRefundApplyDTO,
    ) -> Result<i32> {
        self.apply_internal(OWNER_SUPPLIER, vendor_id, dto).await
    }

    /// 退还申请统一内部入口（Java `applyInternal`）。
    async fn apply_internal(
        &self,
        owner_type: i32,
        owner_id: Option<i32>,
        dto: &DepositRefundApplyDTO,
    ) -> Result<i32> {
        let owner_id = owner_id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::biz_msg(MSG_OWNER_NOT_EXIST))?;
        let amount = dto
            .amount
            .filter(|amount| *amount > Decimal::ZERO)
            .ok_or_else(|| Error::biz(DEPOSIT_AMOUNT_INVALID, MSG_AMOUNT_INVALID))?;
        // Java `DepositRefundType.getDescByCode(type).isEmpty()`：type 缺失或非 1/2 均非法。
        let refund_type = dto
            .r#type
            .filter(|t| *t == 1 || *t == REFUND_TYPE_FULL)
            .ok_or_else(|| Error::biz(DEPOSIT_AMOUNT_INVALID, MSG_REFUND_TYPE_INVALID))?;

        let enabled = self.is_deposit_enabled().await?;
        // type = 2 全额退还：功能开启时按 5 个前置条件拦截；部分提现不走。
        if refund_type == REFUND_TYPE_FULL && enabled {
            self.validate_full_refund_preconditions(owner_type, owner_id)
                .await?;
        }
        let refundable = self
            .effective_refundable(owner_type, owner_id, enabled)
            .await?;
        if amount > refundable {
            return Err(Error::biz(DEPOSIT_WITHDRAW_EXCEED_LIMIT, MSG_EXCEED_LIMIT));
        }

        let now = now_i32();
        let refund = ShopDepositRefund {
            owner_type,
            owner_id,
            r#type: refund_type,
            amount,
            payee_info: dto.payee_info.clone(),
            reason: dto.reason.clone(),
            status: REFUND_PENDING,
            pay_voucher: None,
            audit_admin_id: 0,
            audit_time: 0,
            audit_remark: None,
            add_time: now,
            update_time: now,
            ..Default::default()
        };
        self.repos
            .merchant
            .deposit_refund
            .insert_refund(&refund)
            .await
    }

    // ————————————————————— 全额退还预检 —————————————————————

    /// 店铺端查询能否全额退还保证金（Java `fullRefundCheck`，只读不抛业务错）。
    pub async fn full_refund_check(
        &self,
        shop_id: Option<i32>,
    ) -> Result<DepositFullRefundCheckVO> {
        self.full_refund_check_internal(OWNER_SHOP, shop_id).await
    }

    /// 供应商端查询能否全额退还保证金（Java `vendorFullRefundCheck`）。
    pub async fn vendor_full_refund_check(
        &self,
        vendor_id: Option<i32>,
    ) -> Result<DepositFullRefundCheckVO> {
        self.full_refund_check_internal(OWNER_SUPPLIER, vendor_id)
            .await
    }

    /// 预检内部实现（Java `fullRefundCheckInternal`）。
    async fn full_refund_check_internal(
        &self,
        owner_type: i32,
        owner_id: Option<i32>,
    ) -> Result<DepositFullRefundCheckVO> {
        let repo = &self.repos.merchant.deposit_refund;
        let limit_days = after_sales_limit_days(
            repo.config_value(CONFIG_AFTER_SALES_LIMIT_DAYS)
                .await?
                .as_deref(),
        );
        let mut vo = DepositFullRefundCheckVO {
            after_sales_limit_days: i32::try_from(limit_days).unwrap_or(i32::MAX),
            ..Default::default()
        };
        let Some(owner_id) = owner_id.filter(|id| *id > 0) else {
            vo.can_refund = false;
            vo.refundable_amount = Decimal::ZERO.to_string();
            vo.unmet_reasons = vec![MSG_OWNER_NOT_EXIST.to_string()];
            return Ok(vo);
        };
        let enabled = self.is_deposit_enabled().await?;
        if !enabled {
            // 保证金功能关闭：无条件放行，可退额度仍扣减待审核退还。
            vo.can_refund = true;
            vo.refundable_amount = self
                .effective_refundable(owner_type, owner_id, false)
                .await?
                .to_string();
            vo.unmet_reasons = Vec::new();
            return Ok(vo);
        }

        let mut reasons: Vec<String> = Vec::new();
        // 条件 1：处于停业/关店状态
        if owner_type == OWNER_SUPPLIER {
            let closed = repo
                .find_vendor(owner_id)
                .await?
                .map(|vendor| vendor_is_closed(vendor.status))
                .unwrap_or(false);
            if !closed {
                reasons.push(MSG_VENDOR_NOT_CLOSED_CHECK.to_string());
            }
        } else {
            let closed = repo
                .find_shop(owner_id)
                .await?
                .map(|shop| shop_is_closed(shop.status))
                .unwrap_or(false);
            if !closed {
                reasons.push(MSG_SHOP_NOT_CLOSED_CHECK.to_string());
            }
        }
        // 条件 2：无进行中订单
        if repo.count_ongoing_orders(owner_type, owner_id).await? > 0 {
            reasons.push(MSG_HAS_ONGOING_ORDER.to_string());
        }
        // 条件 3：无未完结售后
        if repo.count_ongoing_aftersales(owner_type, owner_id).await? > 0 {
            reasons.push(MSG_HAS_ONGOING_AFTERSALE.to_string());
        }
        // 条件 4：末单确认收货满 N 天（N = 售后限制天数）
        if limit_days > 0 {
            if let Some(received_time) = repo.last_received_time(owner_type, owner_id).await? {
                let deadline = i64::from(received_time).saturating_add(limit_days * SECONDS_PER_DAY);
                if now_seconds() < deadline {
                    reasons.push(format!("末单确认收货未满 {limit_days} 天"));
                }
            }
        }
        // 条件 5：无进行中退还单
        if repo.count_pending_refund(owner_type, owner_id).await? > 0 {
            reasons.push(MSG_HAS_ONGOING_REFUND.to_string());
        }

        let can_refund = reasons.is_empty();
        vo.can_refund = can_refund;
        vo.refundable_amount = if can_refund {
            self.effective_refundable(owner_type, owner_id, true)
                .await?
                .to_string()
        } else {
            Decimal::ZERO.to_string()
        };
        vo.unmet_reasons = reasons;
        Ok(vo)
    }

    /// 全额退还申请校验（Java `validateFullRefundPreconditions`，按顺序抛业务码 5407~5411）。
    async fn validate_full_refund_preconditions(
        &self,
        owner_type: i32,
        owner_id: i32,
    ) -> Result<()> {
        let repo = &self.repos.merchant.deposit_refund;
        // 条件 1：停业/关店（5407）
        if owner_type == OWNER_SUPPLIER {
            let closed = repo
                .find_vendor(owner_id)
                .await?
                .map(|vendor| vendor_is_closed(vendor.status))
                .unwrap_or(false);
            if !closed {
                return Err(Error::biz(
                    DEPOSIT_REFUND_SHOP_NOT_CLOSED,
                    MSG_VENDOR_NOT_CLOSED,
                ));
            }
        } else {
            let closed = repo
                .find_shop(owner_id)
                .await?
                .map(|shop| shop_is_closed(shop.status))
                .unwrap_or(false);
            if !closed {
                return Err(Error::biz(
                    DEPOSIT_REFUND_SHOP_NOT_CLOSED,
                    MSG_SHOP_NOT_CLOSED,
                ));
            }
        }
        // 条件 2：无进行中订单（5408）
        if repo.count_ongoing_orders(owner_type, owner_id).await? > 0 {
            return Err(Error::biz(
                DEPOSIT_REFUND_HAS_ONGOING_ORDER,
                MSG_HAS_ONGOING_ORDER,
            ));
        }
        // 条件 3：无未完结售后（5409）
        if repo.count_ongoing_aftersales(owner_type, owner_id).await? > 0 {
            return Err(Error::biz(
                DEPOSIT_REFUND_HAS_ONGOING_AFTERSALE,
                MSG_HAS_ONGOING_AFTERSALE,
            ));
        }
        // 条件 4：冷静期已过（5410）——天数取 deposit_refund_cooling_days（默认 7）
        let cooling = cooling_days(
            repo.config_value(CONFIG_REFUND_COOLING_DAYS)
                .await?
                .as_deref(),
        );
        if let Some(received_time) = repo.last_received_time(owner_type, owner_id).await? {
            let deadline = i64::from(received_time).saturating_add(cooling * SECONDS_PER_DAY);
            if now_seconds() < deadline {
                return Err(Error::biz(
                    DEPOSIT_REFUND_COOLDOWN_NOT_REACHED,
                    MSG_COOLDOWN_NOT_REACHED,
                ));
            }
        }
        // 条件 5：无进行中退还单（5411）——一步式下进行中仅含待审核
        if repo.count_pending_refund(owner_type, owner_id).await? > 0 {
            return Err(Error::biz(
                DEPOSIT_REFUND_HAS_ONGOING_REFUND,
                MSG_HAS_ONGOING_REFUND,
            ));
        }
        Ok(())
    }

    // ————————————————————— 平台端列表 / 详情 —————————————————————

    /// 平台端退还单分页（Java `list(DepositRefundListDTO)`）。
    pub async fn list(&self, q: &DepositRefundListQuery) -> Result<PageResult<DepositRefundVO>> {
        let page_query_value = page_query(q.page, q.size);
        let (total, rows) = self
            .repos
            .merchant
            .deposit_refund
            .paginate_refunds(q)
            .await?;
        let mut list = Vec::with_capacity(rows.len());
        for row in &rows {
            list.push(self.refund_vo(row).await?);
        }
        Ok(PageResult::new(total, list, &page_query_value))
    }

    /// 平台端退还单详情（Java `detail(Integer id)`；id 为 null 时返回 null）。
    pub async fn detail(&self, id: Option<i32>) -> Result<Option<DepositRefundVO>> {
        let Some(id) = id else {
            return Ok(None);
        };
        let refund = self
            .repos
            .merchant
            .deposit_refund
            .find_refund(id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_REFUND_NOT_EXIST))?;
        Ok(Some(self.refund_vo(&refund).await?))
    }

    /// 平台端一步式审核退还单（Java `audit`，**单事务**）。
    ///
    /// 通过：打款凭证必填 → 账户出账 + 流水 `type = 2 提现出账` + 单据已完成；
    /// 驳回：单据未通过。
    pub async fn audit(&self, admin_id: Option<i32>, dto: &DepositRefundAuditDTO) -> Result<()> {
        let Some(id) = dto.id else {
            return Err(Error::biz_msg(MSG_REFUND_NOT_EXIST));
        };
        let repo = &self.repos.merchant.deposit_refund;
        let mut tx = repo.begin().await?;
        let Some(refund) = repo.find_refund_in_tx(&mut tx, id).await? else {
            return Err(Error::biz_msg(MSG_REFUND_NOT_EXIST));
        };
        if refund.status != REFUND_PENDING {
            return Err(Error::biz(
                DEPOSIT_REFUND_STATUS_INVALID,
                MSG_REFUND_STATUS_INVALID,
            ));
        }
        let pass = dto.pass == Some(true);
        let now = now_i32();
        let mut updated = refund.clone();
        if pass {
            // 一步式：通过即代表已打款，打款凭证必填。
            let Some(pay_voucher) = dto
                .pay_voucher
                .as_deref()
                .filter(|voucher| !voucher.trim().is_empty())
            else {
                return Err(Error::biz_msg(MSG_PAY_VOUCHER_REQUIRED));
            };
            // Java 注释：退还金额沿用申请金额，统一按提现出账(WITHDRAW)记录，
            // 不再区分全额退还(REFUND)（余额不足由 changeDeposit 拦截）。
            let account = repo
                .lock_or_create_account_in_tx(
                    &mut tx,
                    refund.owner_type,
                    refund.owner_id,
                    now,
                )
                .await?;
            let (change_amount, after_paid) =
                deposit_change(account.paid, refund.amount, LOG_TYPE_WITHDRAW)?;
            repo.update_account_paid_in_tx(&mut tx, account.shop_deposit_id, after_paid, now)
                .await?;
            let log = ShopDepositLog {
                owner_type: refund.owner_type,
                owner_id: refund.owner_id,
                r#type: LOG_TYPE_WITHDRAW,
                change_amount,
                before_paid: account.paid,
                after_paid,
                related_id: refund.shop_deposit_refund_id,
                order_id: 0,
                admin_id: admin_id.unwrap_or(0),
                remark: dto.audit_remark.clone(),
                add_time: now,
                ..Default::default()
            };
            repo.insert_log_in_tx(&mut tx, &log).await?;
            updated.status = REFUND_COMPLETED;
            updated.pay_voucher = Some(pay_voucher.to_string());
            tracing::info!(
                "保证金退还单 {} 已出账，paid {}→{}",
                refund.shop_deposit_refund_id,
                account.paid,
                after_paid
            );
        } else {
            updated.status = REFUND_REJECTED;
        }
        updated.audit_admin_id = admin_id.unwrap_or(0);
        updated.audit_time = now;
        updated.audit_remark = dto.audit_remark.clone();
        updated.update_time = now;
        repo.update_refund_in_tx(&mut tx, &updated).await?;
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    // ————————————————————— 内部工具 —————————————————————

    /// 保证金功能是否开启（Java `isDepositEnabled`：`deposit_enable == 1`）。
    async fn is_deposit_enabled(&self) -> Result<bool> {
        Ok(is_deposit_enabled(
            self.repos
                .merchant
                .deposit_refund
                .config_value(CONFIG_DEPOSIT_ENABLE)
                .await?
                .as_deref(),
        ))
    }

    /// 有效可退额度 = `max(0, 可提现 - 待审核退还总额)`（Java `effectiveRefundable`）。
    async fn effective_refundable(
        &self,
        owner_type: i32,
        owner_id: i32,
        enabled: bool,
    ) -> Result<Decimal> {
        let repo = &self.repos.merchant.deposit_refund;
        let account = repo.find_account(owner_type, owner_id).await?;
        let paid = account
            .as_ref()
            .map_or(Decimal::ZERO, |account| account.paid);
        let required = account.as_ref().map_or(Decimal::ZERO, |account| {
            required_of(enabled, account.required_manual, account.required_snapshot)
        });
        let withdrawable = withdrawable(paid, required);
        let pending = repo.sum_pending_refund(owner_type, owner_id).await?;
        Ok(effective_refundable(withdrawable, pending))
    }

    /// 退还单实体 → VO（Java `toVO`）：持有者名称/主体类型回填。
    async fn refund_vo(&self, refund: &ShopDepositRefund) -> Result<DepositRefundVO> {
        let (owner_name, subject_type) = self
            .owner_info(refund.owner_type, refund.owner_id)
            .await?;
        Ok(DepositRefundVO {
            shop_deposit_refund_id: refund.shop_deposit_refund_id,
            owner_type: refund.owner_type,
            owner_type_text: owner_type_text(refund.owner_type).into(),
            subject_type,
            subject_type_text: subject_type.map(|t| subject_type_text(t).to_string()),
            owner_id: refund.owner_id,
            owner_name: owner_name.clone(),
            // 兼容字段：Java 无条件 setShopId(ownerId)
            shop_id: refund.owner_id,
            shop_name: if refund.owner_type == OWNER_SHOP {
                owner_name
            } else {
                None
            },
            r#type: refund.r#type,
            type_text: refund_type_text(refund.r#type).into(),
            amount: refund.amount.to_string(),
            payee_info: refund.payee_info.clone(),
            reason: refund.reason.clone(),
            status: refund.status,
            status_text: refund_status_text(refund.status).into(),
            pay_voucher: refund.pay_voucher.clone(),
            audit_admin_id: refund.audit_admin_id,
            audit_time: i64::from(refund.audit_time),
            audit_time_text: format_time(i64::from(refund.audit_time)),
            audit_remark: refund.audit_remark.clone(),
            add_time: i64::from(refund.add_time),
            add_time_text: format_time(i64::from(refund.add_time)),
        })
    }

    /// 持有者名称 + 主体类型（Java `fillOwnerName`）：店铺取
    /// `shop.shop_title` + `merchant.type`；供应商取 `vendor.vendor_name` + `vendor.type`。
    async fn owner_info(
        &self,
        owner_type: i32,
        owner_id: i32,
    ) -> Result<(Option<String>, Option<i32>)> {
        if owner_id <= 0 {
            return Ok((None, None));
        }
        let repo = &self.repos.merchant.deposit_refund;
        if owner_type == OWNER_SUPPLIER {
            return Ok(match repo.find_vendor(owner_id).await? {
                Some(vendor) => (vendor.vendor_name, vendor.r#type),
                None => (None, None),
            });
        }
        let Some(shop) = repo.find_shop(owner_id).await? else {
            return Ok((None, None));
        };
        let subject_type = match shop.merchant_id.filter(|id| *id > 0) {
            Some(merchant_id) => repo
                .find_merchant(merchant_id)
                .await?
                .and_then(|merchant| merchant.r#type),
            None => None,
        };
        Ok((shop.shop_title, subject_type))
    }
}

/// 当前秒级时间戳（i32，列类型 `int unsigned`）。
fn now_i32() -> i32 {
    i32::try_from(now_seconds()).unwrap_or(i32::MAX)
}

/// 进行中订单/售后口径常量（供单测断言，避免与 Java 枚举漂移）。
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_status_and_log_constants_match_java() {
        assert_eq!(REFUND_PENDING, 0);
        assert_eq!(REFUND_COMPLETED, 1);
        assert_eq!(REFUND_REJECTED, 2);
        assert_eq!(LOG_TYPE_WITHDRAW, 2);
        assert_eq!(DEPOSIT_REFUND_STATUS_INVALID, 5404);
        assert_eq!(DEPOSIT_REFUND_SHOP_NOT_CLOSED, 5407);
        assert_eq!(DEPOSIT_REFUND_HAS_ONGOING_ORDER, 5408);
        assert_eq!(DEPOSIT_REFUND_HAS_ONGOING_AFTERSALE, 5409);
        assert_eq!(DEPOSIT_REFUND_COOLDOWN_NOT_REACHED, 5410);
        assert_eq!(DEPOSIT_REFUND_HAS_ONGOING_REFUND, 5411);
        assert_eq!(DEPOSIT_WITHDRAW_EXCEED_LIMIT, 5412);
        assert_eq!(AFTERSALES_COMPLETE, 6);
        assert_eq!(AFTERSALES_CANCEL, 7);
    }

    #[test]
    fn error_messages_match_java_sources() {
        assert_eq!(MSG_OWNER_NOT_EXIST, "持有者不存在");
        assert_eq!(MSG_AMOUNT_INVALID, "保证金金额非法");
        assert_eq!(MSG_BALANCE_NOT_ENOUGH, "保证金余额不足");
        assert_eq!(MSG_REFUND_TYPE_INVALID, "退还类型非法");
        assert_eq!(MSG_EXCEED_LIMIT, "退还金额超过可退上限");
        assert_eq!(MSG_REFUND_NOT_EXIST, "退还单不存在");
        assert_eq!(MSG_REFUND_STATUS_INVALID, "退还单状态非法");
        assert_eq!(MSG_PAY_VOUCHER_REQUIRED, "打款凭证不能为空");
        assert_eq!(MSG_SHOP_NOT_CLOSED, "全额退还需店铺停业");
        assert_eq!(MSG_VENDOR_NOT_CLOSED, "全额退还需供应商已停业关闭");
        assert_eq!(MSG_HAS_ONGOING_ORDER, "存在进行中订单");
        assert_eq!(MSG_HAS_ONGOING_AFTERSALE, "存在进行中售后");
        assert_eq!(MSG_COOLDOWN_NOT_REACHED, "未过末单收货冷静期");
        assert_eq!(MSG_HAS_ONGOING_REFUND, "存在进行中退还单");
        assert_eq!(MSG_SHOP_NOT_CLOSED_CHECK, "店铺未停业");
        assert_eq!(MSG_VENDOR_NOT_CLOSED_CHECK, "供应商未停业关闭");
    }

    /// 全额退还条件 4 的 deadline 计算：末单收货时间 + N 天。
    #[test]
    fn cooldown_deadline_math() {
        let received = 1_700_000_000_i64;
        let cooling = 7_i64;
        let deadline = received.saturating_add(cooling * SECONDS_PER_DAY);
        assert_eq!(deadline, received + 604_800);
        // 冷静期 0 天 ⇒ deadline == 收货时间（一定 <= now）
        assert!(received <= received.saturating_add(0));
    }

    /// 「有效可退」与「应缴」口径（纯函数，不触库）。
    #[test]
    fn effective_refundable_and_required() {
        assert_eq!(
            required_of(true, Some(Decimal::new(500, 0)), Decimal::new(100, 0)),
            Decimal::new(500, 0)
        );
        assert_eq!(
            required_of(false, Some(Decimal::new(500, 0)), Decimal::new(100, 0)),
            Decimal::ZERO
        );
        assert_eq!(
            withdrawable(Decimal::new(120, 0), Decimal::new(100, 0)),
            Decimal::new(20, 0)
        );
        assert_eq!(
            effective_refundable(Decimal::new(20, 0), Decimal::new(5, 0)),
            Decimal::new(15, 0)
        );
    }
}
