//! 保证金充值域服务（merchant::deposit_recharge）。
//!
//! 对应原 Java `com.nextrade.service.deposit.impl.DepositRechargeServiceImpl`（595 行）
//! 与管理端 `DepositRechargeController` 的 10 个端点。
//!
//! # 迁移要点（逐条对照 Java 源）
//!
//! - `payeeAccountOptions`：启用的收款账户列表（`status = 1`，可选账户类型/关键词，
//!   排序 `sort_order ASC, id DESC`）。
//! - `apply` / `vendorApply`：店铺/供应商两个入口共用内部逻辑 ——
//!   持有者存在性 → 金额 > 0 → 收款账户必填/存在/启用 → 快照收款账户
//!   （`payee_snapshot` JSON，供详情页财务核对）→ 落库（状态 0 待审核、
//!   审核人/审核时间 0）。
//! - `shopPaymentAuditList` / `vendorPaymentAuditList`：当前 owner 的**充值 + 退还**
//!   合并列表，按提交时间倒序，**内存分页**（Java 先全量合并再 `subList`）。
//! - `shopPaymentAuditDetail` / `vendorPaymentAuditDetail`：按 `auditType`
//!   取充值/退还单并校验 owner 归属（1 充值 / 2 退还 / 其他 → 审核类型非法）。
//! - `list` / `detail`：平台端充值单列表/详情；列表支持 owner 分段
//!   （类型 + 组织名/主体类型 → SQL 子查询）、精确 ownerId、收款账户类型、
//!   状态、时间区间与白名单排序。
//! - `audit`（Java `@Transactional`）：仅待审核可审；通过 → 账户入账
//!   （`paid += amount`）+ 保证金流水 `type = 1 充值入账` + 单据置「已入账」；
//!   驳回 → 单据置「未通过」。三次写入在同一事务内。
//!
//! # 未迁移 / 偏差（见 `uncertain_points`）
//!
//! - 审核通过后的「缴足即时对账」`depositLifecycleService.restore(owner, owner)`
//!   （清补缴缓冲 / 解除处罚 / 恢复下架商品）未迁移 —— 该生命周期服务不存在，
//!   本实现只完成入账 + 流水 + 状态，restore 部分 `tracing::info` 记录后跳过。
//! - `DepositAccountServiceImpl#computeRequired` 的「按类目规则取最高应缴」
//!   （`DepositRuleMatcher`）未迁移，本域不涉及应缴计算（仅按实缴入账）。
//! - 金额出参为两位小数文本（Java 为 JSON number），见模型模块注释。
//! - Java 用 `SecurityUtils.getCurrentAdminId()` 取审核人，Rust 由路由层从
//!   JWT `AuthUser` 显式传入；`HeaderUtils.getShopId/getVendorId` 由路由层按
//!   请求头解析后传入。
//! - holder 名称/主体类型富化：Java 逐行查 `shop`/`vendor`/`merchant`，
//!   本实现同样逐行查询（列表页 Java 也是 N+1，保持一致）。

use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_common::utils::now_seconds;
use nextrade_core::entities::{ShopDepositLog, ShopDepositRecharge, ShopDepositRefund};
use nextrade_core::models::merchant::deposit_recharge::{
    audit_type_text, deposit_change, format_time, owner_type_text, page_query,
    payee_account_type_text, payee_snapshot_json, recharge_status_text, refund_status_text,
    refund_type_text, slice_page, subject_type_text, DepositPaymentAuditListQuery,
    DepositPayeeAccountSnapshot, DepositPayeeAccountVO, DepositPaymentAuditVO,
    DepositRechargeApplyDTO, DepositRechargeAuditDTO, DepositRechargeListQuery, DepositRechargeVO,
    AUDIT_TYPE_RECHARGE, AUDIT_TYPE_REFUND, DEPOSIT_AMOUNT_INVALID,
    DEPOSIT_RECHARGE_STATUS_INVALID, LOG_TYPE_RECHARGE, MSG_AMOUNT_INVALID,
    MSG_AUDIT_TYPE_INVALID, MSG_BILL_NOT_EXIST, MSG_OWNER_NOT_EXIST, MSG_PAYEE_DISABLED,
    MSG_PAYEE_NOT_EXIST, MSG_PAYEE_REQUIRED, MSG_RECHARGE_NOT_EXIST, MSG_RECHARGE_STATUS_INVALID,
    MSG_REFUND_NOT_EXIST, OWNER_SHOP, OWNER_SUPPLIER, RECHARGE_CREDITED, RECHARGE_PENDING,
    RECHARGE_REJECTED,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 保证金充值域服务。
pub struct DepositRechargeService {
    repos: Arc<Repositories>,
}

impl DepositRechargeService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 收款账户 —————————————————————

    /// 启用的收款账户列表（Java `depositPayeeAccountService.listEnabled`）。
    pub async fn payee_account_options(
        &self,
        account_type: Option<i32>,
        keyword: Option<&str>,
    ) -> Result<Vec<DepositPayeeAccountVO>> {
        Ok(self
            .repos
            .merchant
            .deposit_recharge
            .list_payee_accounts(account_type, keyword)
            .await?
            .iter()
            .map(payee_account_vo)
            .collect())
    }

    // ————————————————————— 申请 —————————————————————

    /// 店铺端发起充值申请（Java `apply(shopId, applyDTO)`）。
    pub async fn apply(
        &self,
        shop_id: Option<i32>,
        dto: &DepositRechargeApplyDTO,
    ) -> Result<i32> {
        self.apply_internal(OWNER_SHOP, shop_id, dto).await
    }

    /// 供应商端发起充值申请（Java `vendorApply(vendorId, applyDTO)`）。
    pub async fn vendor_apply(
        &self,
        vendor_id: Option<i32>,
        dto: &DepositRechargeApplyDTO,
    ) -> Result<i32> {
        self.apply_internal(OWNER_SUPPLIER, vendor_id, dto).await
    }

    /// 充值申请统一内部入口（Java `applyInternal`）。
    async fn apply_internal(
        &self,
        owner_type: i32,
        owner_id: Option<i32>,
        dto: &DepositRechargeApplyDTO,
    ) -> Result<i32> {
        let owner_id = owner_id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::biz_msg(MSG_OWNER_NOT_EXIST))?;
        let amount = dto
            .amount
            .filter(|amount| *amount > Decimal::ZERO)
            .ok_or_else(|| Error::biz(DEPOSIT_AMOUNT_INVALID, MSG_AMOUNT_INVALID))?;
        let payee_account_id = dto
            .payee_account_id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::biz_msg(MSG_PAYEE_REQUIRED))?;

        let repo = &self.repos.merchant.deposit_recharge;
        let payee = repo
            .find_payee_account(payee_account_id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_PAYEE_NOT_EXIST))?;
        if payee.status != 1 {
            return Err(Error::biz_msg(MSG_PAYEE_DISABLED));
        }
        let payee_vo = payee_account_vo(&payee);
        let snapshot = payee_snapshot_json(&payee_vo);

        let now = now_i32();
        let recharge = ShopDepositRecharge {
            owner_type,
            owner_id,
            amount,
            pay_voucher: dto.pay_voucher.clone(),
            payee_account_id,
            payee_account_type: payee.account_type,
            payee_snapshot: Some(snapshot),
            remark: dto.remark.clone(),
            status: RECHARGE_PENDING,
            audit_admin_id: 0,
            audit_time: 0,
            audit_remark: None,
            add_time: now,
            update_time: now,
            ..Default::default()
        };
        repo.insert_recharge(&recharge).await
    }

    // ————————————————————— 款项审核（店铺/供应商端） —————————————————————

    /// 店铺端款项审核合并列表（Java `paymentAuditList(SHOP, shopId, dto)`）。
    pub async fn shop_payment_audit_list(
        &self,
        shop_id: Option<i32>,
        q: &DepositPaymentAuditListQuery,
    ) -> Result<PageResult<DepositPaymentAuditVO>> {
        self.payment_audit_list(OWNER_SHOP, shop_id, q).await
    }

    /// 供应商端款项审核合并列表（Java `paymentAuditList(SUPPLIER, vendorId, dto)`）。
    pub async fn vendor_payment_audit_list(
        &self,
        vendor_id: Option<i32>,
        q: &DepositPaymentAuditListQuery,
    ) -> Result<PageResult<DepositPaymentAuditVO>> {
        self.payment_audit_list(OWNER_SUPPLIER, vendor_id, q).await
    }

    /// 合并列表内部实现（Java `paymentAuditList`）。
    async fn payment_audit_list(
        &self,
        owner_type: i32,
        owner_id: Option<i32>,
        q: &DepositPaymentAuditListQuery,
    ) -> Result<PageResult<DepositPaymentAuditVO>> {
        let page_query_value = page_query(q.page, q.size);
        // owner 未登录/非法：返回空分页，避免越权全量（Java 同）
        let Some(owner_id) = owner_id.filter(|id| *id > 0) else {
            return Ok(PageResult::empty(&page_query_value));
        };
        let repo = &self.repos.merchant.deposit_recharge;
        let audit_type = q.audit_type;
        let mut merged: Vec<DepositPaymentAuditVO> = Vec::new();
        if audit_type.is_none() || audit_type == Some(AUDIT_TYPE_RECHARGE) {
            for recharge in repo
                .list_owner_recharges(owner_type, owner_id, q.status, q.start_time, q.end_time)
                .await?
            {
                merged.push(audit_vo_recharge(&recharge));
            }
        }
        if audit_type.is_none() || audit_type == Some(AUDIT_TYPE_REFUND) {
            for refund in repo
                .list_owner_refunds(owner_type, owner_id, q.status, q.start_time, q.end_time)
                .await?
            {
                merged.push(audit_vo_refund(&refund));
            }
        }
        // Java `Comparator.comparing(addTime, nullsLast(reverseOrder()))`：时间倒序
        // （add_time 为 NOT NULL；相等时保持插入顺序 —— 充值行在前）。
        merged.sort_by(|a, b| b.add_time.cmp(&a.add_time));

        let total = merged.len();
        let (from, to) = slice_page(total, page_query_value.page, page_query_value.page_size);
        let list = merged[from..to].to_vec();
        Ok(PageResult::new(
            u64::try_from(total).unwrap_or(u64::MAX),
            list,
            &page_query_value,
        ))
    }

    /// 店铺端款项审核详情（Java `paymentAuditDetail(SHOP, shopId, auditType, bizId)`）。
    pub async fn shop_payment_audit_detail(
        &self,
        shop_id: Option<i32>,
        audit_type: i32,
        biz_id: i32,
    ) -> Result<DepositPaymentAuditVO> {
        self.payment_audit_detail(OWNER_SHOP, shop_id, audit_type, biz_id)
            .await
    }

    /// 供应商端款项审核详情。
    pub async fn vendor_payment_audit_detail(
        &self,
        vendor_id: Option<i32>,
        audit_type: i32,
        biz_id: i32,
    ) -> Result<DepositPaymentAuditVO> {
        self.payment_audit_detail(OWNER_SUPPLIER, vendor_id, audit_type, biz_id)
            .await
    }

    /// 款项审核详情内部实现（Java `paymentAuditDetail`）。
    async fn payment_audit_detail(
        &self,
        owner_type: i32,
        owner_id: Option<i32>,
        audit_type: i32,
        biz_id: i32,
    ) -> Result<DepositPaymentAuditVO> {
        let owner_id = owner_id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::biz_msg(MSG_OWNER_NOT_EXIST))?;
        if biz_id <= 0 {
            return Err(Error::biz_msg(MSG_BILL_NOT_EXIST));
        }
        let repo = &self.repos.merchant.deposit_recharge;
        if audit_type == AUDIT_TYPE_RECHARGE {
            let recharge = repo
                .find_recharge(biz_id)
                .await?
                .filter(|r| r.owner_type == owner_type && r.owner_id == owner_id)
                .ok_or_else(|| Error::biz_msg(MSG_RECHARGE_NOT_EXIST))?;
            return Ok(audit_vo_recharge(&recharge));
        }
        if audit_type == AUDIT_TYPE_REFUND {
            let refund = repo
                .find_refund(biz_id)
                .await?
                .filter(|r| r.owner_type == owner_type && r.owner_id == owner_id)
                .ok_or_else(|| Error::biz_msg(MSG_REFUND_NOT_EXIST))?;
            return Ok(audit_vo_refund(&refund));
        }
        Err(Error::biz_msg(MSG_AUDIT_TYPE_INVALID))
    }

    // ————————————————————— 平台端列表 / 详情 —————————————————————

    /// 平台端充值单分页（Java `list(DepositRechargeListDTO)`）。
    pub async fn list(&self, q: &DepositRechargeListQuery) -> Result<PageResult<DepositRechargeVO>> {
        let page_query_value = page_query(q.page, q.size);
        let (total, rows) = self
            .repos
            .merchant
            .deposit_recharge
            .paginate_recharges(q)
            .await?;
        let mut list = Vec::with_capacity(rows.len());
        for row in &rows {
            list.push(self.recharge_vo(row).await?);
        }
        Ok(PageResult::new(total, list, &page_query_value))
    }

    /// 平台端充值单详情（Java `detail(Integer id)`；id 为 null 时返回 null）。
    pub async fn detail(&self, id: Option<i32>) -> Result<Option<DepositRechargeVO>> {
        let Some(id) = id else {
            return Ok(None);
        };
        let recharge = self
            .repos
            .merchant
            .deposit_recharge
            .find_recharge(id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_RECHARGE_NOT_EXIST))?;
        Ok(Some(self.recharge_vo(&recharge).await?))
    }

    /// 平台端审核充值单（Java `audit`，**单事务**）。
    ///
    /// 通过：账户入账（`paid += amount`）+ 流水 `type = 1` + 单据已入账；
    /// 驳回：单据未通过。审核字段统一回填。
    pub async fn audit(&self, admin_id: Option<i32>, dto: &DepositRechargeAuditDTO) -> Result<()> {
        let Some(id) = dto.id else {
            return Err(Error::biz_msg(MSG_RECHARGE_NOT_EXIST));
        };
        let repo = &self.repos.merchant.deposit_recharge;
        let mut tx = repo.begin().await?;
        let Some(recharge) = repo.find_recharge_in_tx(&mut tx, id).await? else {
            return Err(Error::biz_msg(MSG_RECHARGE_NOT_EXIST));
        };
        if recharge.status != RECHARGE_PENDING {
            return Err(Error::biz(
                DEPOSIT_RECHARGE_STATUS_INVALID,
                MSG_RECHARGE_STATUS_INVALID,
            ));
        }
        let pass = dto.pass == Some(true);
        let now = now_i32();
        let mut updated = recharge.clone();
        if pass {
            // Java `depositAccountService.changeDeposit(..., RECHARGE, rechargeId, 0,
            // adminId, auditRemark)`：入账 + 流水。
            let account = repo
                .lock_or_create_account_in_tx(
                    &mut tx,
                    recharge.owner_type,
                    recharge.owner_id,
                    now,
                )
                .await?;
            let (change_amount, after_paid) =
                deposit_change(account.paid, recharge.amount, LOG_TYPE_RECHARGE)?;
            repo.update_account_paid_in_tx(&mut tx, account.shop_deposit_id, after_paid, now)
                .await?;
            let log = ShopDepositLog {
                owner_type: recharge.owner_type,
                owner_id: recharge.owner_id,
                r#type: LOG_TYPE_RECHARGE,
                change_amount,
                before_paid: account.paid,
                after_paid,
                related_id: recharge.shop_deposit_recharge_id,
                order_id: 0,
                admin_id: admin_id.unwrap_or(0),
                remark: dto.audit_remark.clone(),
                add_time: now,
                ..Default::default()
            };
            repo.insert_log_in_tx(&mut tx, &log).await?;
            updated.status = RECHARGE_CREDITED;
            tracing::info!(
                "保证金充值单 {} 已入账，paid {}→{}",
                recharge.shop_deposit_recharge_id,
                account.paid,
                after_paid
            );
        } else {
            updated.status = RECHARGE_REJECTED;
        }
        updated.audit_admin_id = admin_id.unwrap_or(0);
        updated.audit_time = now;
        updated.audit_remark = dto.audit_remark.clone();
        updated.update_time = now;
        repo.update_recharge_in_tx(&mut tx, &updated).await?;
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        if updated.status == RECHARGE_CREDITED {
            // 对齐 Java：充值审核通过入账后触发生命周期 restore，若已缴足则解除惩罚并恢复下架商品
            let account_svc = crate::merchant::deposit_account::DepositAccountService::new(self.repos.clone());
            if let Err(e) = account_svc.restore(recharge.owner_type, recharge.owner_id).await {
                tracing::warn!(
                    "保证金充值入账后触发 restore 异常(已忽略): owner_type={}, owner_id={}, {e}",
                    recharge.owner_type,
                    recharge.owner_id
                );
            }
        }
        Ok(())
    }

    // ————————————————————— VO 富化 —————————————————————

    /// 充值单实体 → VO（Java `toVO`）：持有者名称/主体类型 + 收款账户快照回填。
    async fn recharge_vo(&self, recharge: &ShopDepositRecharge) -> Result<DepositRechargeVO> {
        let (owner_name, subject_type) = self
            .owner_info(recharge.owner_type, recharge.owner_id)
            .await?;
        let mut vo = DepositRechargeVO {
            shop_deposit_recharge_id: recharge.shop_deposit_recharge_id,
            owner_type: recharge.owner_type,
            owner_type_text: owner_type_text(recharge.owner_type).into(),
            subject_type,
            subject_type_text: subject_type.map(|t| subject_type_text(t).to_string()),
            owner_id: recharge.owner_id,
            owner_name: owner_name.clone(),
            // 兼容字段：Java 无条件 setShopId(ownerId)
            shop_id: recharge.owner_id,
            shop_name: if recharge.owner_type == OWNER_SHOP {
                owner_name
            } else {
                None
            },
            amount: recharge.amount.to_string(),
            pay_voucher: recharge.pay_voucher.clone(),
            remark: recharge.remark.clone(),
            payee_account_id: recharge.payee_account_id,
            payee_account_type: recharge.payee_account_type,
            payee_account_type_text: Some(
                payee_account_type_text(recharge.payee_account_type).to_string(),
            ),
            status: recharge.status,
            status_text: recharge_status_text(recharge.status).into(),
            audit_admin_id: recharge.audit_admin_id,
            audit_time: i64::from(recharge.audit_time),
            audit_time_text: format_time(i64::from(recharge.audit_time)),
            audit_remark: recharge.audit_remark.clone(),
            add_time: i64::from(recharge.add_time),
            add_time_text: format_time(i64::from(recharge.add_time)),
            ..Default::default()
        };
        // Java `fillPayeeAccount`：从快照回填，历史值不随账户后续改动变化。
        if let Some(snapshot) = DepositPayeeAccountSnapshot::parse(recharge.payee_snapshot.as_deref())
        {
            if let Some(account_type) = snapshot.account_type {
                vo.payee_account_type = account_type;
                vo.payee_account_type_text =
                    Some(payee_account_type_text(account_type).to_string());
            }
            vo.payee_account_name = snapshot.account_name;
            vo.payee_account_no = snapshot.account_no;
            vo.payee_bank_name = snapshot.bank_name;
            vo.payee_bank_branch = snapshot.bank_branch;
        }
        Ok(vo)
    }

    /// 持有者名称 + 主体类型（Java `fillOwnerName`）：
    /// 店铺取 `shop.shop_title` + `merchant.type`；供应商取 `vendor.vendor_name` + `vendor.type`
    /// （仅 `source_type = 1` 入驻供应商）。
    async fn owner_info(
        &self,
        owner_type: i32,
        owner_id: i32,
    ) -> Result<(Option<String>, Option<i32>)> {
        if owner_id <= 0 {
            return Ok((None, None));
        }
        let repo = &self.repos.merchant.deposit_recharge;
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

/// 收款账户实体 → VO（Java `DepositPayeeAccountServiceImpl#toVO`）。
fn payee_account_vo(entity: &nextrade_core::entities::ShopDepositPayeeAccount) -> DepositPayeeAccountVO {
    DepositPayeeAccountVO {
        id: entity.shop_deposit_payee_account_id,
        account_type: entity.account_type,
        account_type_text: payee_account_type_text(entity.account_type).to_string(),
        account_name: entity.account_name.clone(),
        account_no: entity.account_no.clone(),
        bank_name: entity.bank_name.clone(),
        bank_branch: entity.bank_branch.clone(),
        status: entity.status,
        sort_order: entity.sort_order,
        add_time: i64::from(entity.add_time),
        update_time: i64::from(entity.update_time),
    }
}

/// 充值单 → 款项审核行（Java `toAuditVO(ShopDepositRecharge)`）。
fn audit_vo_recharge(r: &ShopDepositRecharge) -> DepositPaymentAuditVO {
    DepositPaymentAuditVO {
        audit_type: AUDIT_TYPE_RECHARGE,
        audit_type_text: audit_type_text(AUDIT_TYPE_RECHARGE).into(),
        biz_id: r.shop_deposit_recharge_id,
        amount: r.amount.to_string(),
        status: r.status,
        status_text: recharge_status_text(r.status).into(),
        refund_type: None,
        refund_type_text: None,
        remark: r.remark.clone(),
        pay_voucher: r.pay_voucher.clone(),
        audit_remark: r.audit_remark.clone(),
        audit_time: i64::from(r.audit_time),
        audit_time_text: format_time(i64::from(r.audit_time)),
        add_time: i64::from(r.add_time),
        add_time_text: format_time(i64::from(r.add_time)),
    }
}

/// 退还单 → 款项审核行（Java `toAuditVO(ShopDepositRefund)`）。
fn audit_vo_refund(r: &ShopDepositRefund) -> DepositPaymentAuditVO {
    DepositPaymentAuditVO {
        audit_type: AUDIT_TYPE_REFUND,
        audit_type_text: audit_type_text(AUDIT_TYPE_REFUND).into(),
        biz_id: r.shop_deposit_refund_id,
        amount: r.amount.to_string(),
        status: r.status,
        status_text: refund_status_text(r.status).into(),
        refund_type: Some(r.r#type),
        refund_type_text: Some(refund_type_text(r.r#type).to_string()),
        remark: r.reason.clone(),
        pay_voucher: r.pay_voucher.clone(),
        audit_remark: r.audit_remark.clone(),
        audit_time: i64::from(r.audit_time),
        audit_time_text: format_time(i64::from(r.audit_time)),
        add_time: i64::from(r.add_time),
        add_time_text: format_time(i64::from(r.add_time)),
    }
}

/// 当前秒级时间戳（i32，列类型 `int unsigned`）。
fn now_i32() -> i32 {
    i32::try_from(now_seconds()).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recharge_audit_vo_maps_all_display_fields() {
        let r = ShopDepositRecharge {
            shop_deposit_recharge_id: 7,
            owner_type: OWNER_SHOP,
            owner_id: 3,
            amount: Decimal::new(10050, 2),
            pay_voucher: Some("v".into()),
            remark: Some("remark".into()),
            status: RECHARGE_PENDING,
            audit_time: 0,
            add_time: 1_700_000_000,
            update_time: 1_700_000_000,
            ..Default::default()
        };
        let vo = audit_vo_recharge(&r);
        assert_eq!(vo.audit_type, AUDIT_TYPE_RECHARGE);
        assert_eq!(vo.audit_type_text, "保证金充值");
        assert_eq!(vo.biz_id, 7);
        assert_eq!(vo.amount, "100.50");
        assert_eq!(vo.status_text, "待审核");
        assert_eq!(vo.audit_time_text, "");
        assert_eq!(vo.add_time, 1_700_000_000);
        assert!(!vo.add_time_text.is_empty());
        assert_eq!(vo.refund_type, None);
    }

    #[test]
    fn refund_audit_vo_carries_refund_type() {
        let r = ShopDepositRefund {
            shop_deposit_refund_id: 9,
            owner_type: OWNER_SUPPLIER,
            owner_id: 2,
            r#type: 2,
            amount: Decimal::new(500, 0),
            reason: Some("退保证金".into()),
            status: 1,
            add_time: 10,
            audit_time: 20,
            ..Default::default()
        };
        let vo = audit_vo_refund(&r);
        assert_eq!(vo.audit_type, AUDIT_TYPE_REFUND);
        assert_eq!(vo.audit_type_text, "保证金退还");
        assert_eq!(vo.refund_type, Some(2));
        assert_eq!(vo.refund_type_text.as_deref(), Some("全部退还"));
        assert_eq!(vo.status_text, "已通过");
        assert_eq!(vo.remark.as_deref(), Some("退保证金"));
    }

    #[test]
    fn payee_vo_and_snapshot_are_consistent() {
        let entity = nextrade_core::entities::ShopDepositPayeeAccount {
            shop_deposit_payee_account_id: 3,
            account_type: 2,
            account_name: "支付宝".into(),
            account_no: "a@b.c".into(),
            bank_name: None,
            bank_branch: None,
            status: 1,
            sort_order: 0,
            add_time: 1,
            update_time: 2,
        };
        let vo = payee_account_vo(&entity);
        assert_eq!(vo.account_type_text, "支付宝");
        assert_eq!(vo.id, 3);
        assert_eq!(vo.add_time, 1);
        let snapshot = DepositPayeeAccountSnapshot::parse(Some(&payee_snapshot_json(&vo)))
            .expect("snapshot");
        assert_eq!(snapshot.account_type, Some(2));
        assert_eq!(snapshot.account_name.as_deref(), Some("支付宝"));
    }

    #[test]
    fn owner_type_constants_match_java_enum() {
        assert_eq!(OWNER_SHOP, 1);
        assert_eq!(OWNER_SUPPLIER, 2);
        assert_eq!(owner_type_text(OWNER_SHOP), "店铺");
        assert_eq!(owner_type_text(OWNER_SUPPLIER), "供应商");
    }
}
