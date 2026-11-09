//! 保证金账户域服务（merchant::deposit_account）。
//!
//! 对应 Java `DepositAccountAdminServiceImpl`（账户总览/详情/流水/调整记录/手动调整/扣除/预警）。
//!
//! # 已实现
//! `list`（分页）/`detail`/`logList`/`adjustLogList`/`adjust`（写调整记录 + 应缴快照）/
//! `deduct`（写平台扣除流水 + 扣减实缴）。
//!
//! # 未迁移
//! - `sendWarning`：站内信/短信通道（message 域）未迁移 → 明确 1001 业务错误。
//! - `runDueCheck`：`DepositDueCheckTask` + `DepositLifecycleService`（到期扫描/惩罚下架/
//!   恢复）未迁移 → 空操作成功（运维端点）。
//! - `depositStatus` 的「冻结(3)」依赖 `penalty_status`；本实现按该字段推断，
//!   列表的 `depositStatus` 过滤在内存中完成（Java 为 SQL 条件）。

use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_common::utils::now_seconds;
use nextrade_core::models::merchant::deposit_account::{
    available_amount, compute_deposit_status, deposit_status_text, log_type_text, money,
    owner_type_text, pending_amount, subject_type_text, DepositAccountDetailVO, DepositAccountListQuery,
    DepositAccountVO, DepositAdjustDTO, DepositDeductDTO, DepositLogVO, DepositSendWarningDTO,
    ADJUST_TYPE_INVALID, DEDUCT_AMOUNT_INVALID, OWNER_ID_INVALID, OWNER_SHOP, PARAM_INVALID,
    REQUIRED_MANUAL_INVALID, SERVICE_DATA_ERROR, SHOP_NOT_EXIST, VENDOR_NOT_EXIST,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::merchant::deposit_account::{
    AccountRow, DepositAccountRepository,
};
use nextrade_infrastructure::db::Repositories;

/// 保证金账户域服务。
pub struct DepositAccountService {
    repos: Arc<Repositories>,
}

impl DepositAccountService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    #[must_use]
    pub fn repo(&self) -> &DepositAccountRepository {
        &self.repos.merchant.deposit_account
    }

    /// `GET /adminapi/deposit/account/list`。
    pub async fn list(&self, q: &DepositAccountListQuery) -> Result<PageResult<DepositAccountVO>> {
        let (page, size) = q.page_params();
        let (total, rows) = self.repo().paginate(q).await?;
        let mut list = Vec::with_capacity(rows.len());
        for row in &rows {
            let mut vo = self.account_vo(row).await?;
            let status = compute_deposit_status(required(row), row.paid, row.penalty_status);
            vo.deposit_status = status;
            vo.deposit_status_text = deposit_status_text(status).to_string();
            if let Some(filter) = q.deposit_status {
                if filter != status {
                    continue;
                }
            }
            list.push(vo);
        }
        let pq = PageQuery {
            page,
            page_size: size,
        };
        Ok(PageResult::new(total, list, &pq))
    }

    /// `GET /adminapi/deposit/account/detail?ownerType=&ownerId=|=shopId`。
    pub async fn detail(
        &self,
        owner_type: Option<i32>,
        owner_id: Option<i32>,
    ) -> Result<DepositAccountDetailVO> {
        let (owner_type, owner_id) = self.resolve_owner(owner_type, owner_id, false).await?;
        let row = self
            .repo()
            .find_by_owner(owner_type, owner_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, PARAM_INVALID))?;
        let required = required(&row);
        let status = compute_deposit_status(required, row.paid, row.penalty_status);
        let frozen = std::cmp::min(row.paid, required);
        Ok(DepositAccountDetailVO {
            owner_type,
            owner_type_text: owner_type_text(owner_type).into(),
            owner_id,
            owner_name: self.owner_name(owner_type, owner_id).await?,
            shop_id: (owner_type == OWNER_SHOP).then_some(owner_id),
            shop_title: if owner_type == OWNER_SHOP {
                self.repo().owner_name(1, owner_id).await?
            } else {
                None
            },
            subject_type: 0,
            subject_type_text: subject_type_text(0).into(),
            deposit_status: status,
            deposit_status_text: deposit_status_text(status).into(),
            required: required.to_string(),
            paid: row.paid.to_string(),
            frozen: frozen.to_string(),
            available: available_amount(row.paid, required).to_string(),
            pending: pending_amount(row.paid, required).to_string(),
        })
    }

    /// `GET /adminapi/deposit/account/logList`。
    pub async fn log_list(
        &self,
        owner_type: Option<i32>,
        owner_id: Option<i32>,
        page: Option<u64>,
        size: Option<u64>,
    ) -> Result<PageResult<DepositLogVO>> {
        let (owner_type, owner_id) = self.resolve_owner(owner_type, owner_id, false).await?;
        let page = page.unwrap_or(1).max(1);
        let size = size.unwrap_or(15).clamp(1, 100);
        let (total, rows) = self
            .repo()
            .paginate_logs(owner_type, owner_id, page, size)
            .await?;
        let owner_name = self.owner_name(owner_type, owner_id).await?;
        let list: Vec<DepositLogVO> = rows
            .iter()
            .map(|r| DepositLogVO {
                shop_deposit_log_id: r.shop_deposit_log_id,
                owner_type: r.owner_type,
                owner_type_text: owner_type_text(r.owner_type).into(),
                owner_id: r.owner_id,
                owner_name: owner_name.clone(),
                r#type: r.r#type,
                type_text: log_type_text(r.r#type).into(),
                change_amount: r.change_amount.to_string(),
                before_paid: r.before_paid.to_string(),
                after_paid: r.after_paid.to_string(),
                related_id: r.related_id,
                order_id: r.order_id,
                admin_id: r.admin_id,
                remark: r.remark.clone(),
                add_time: r.add_time.to_string(),
            })
            .collect();
        Ok(PageResult::new(
            total,
            list,
            &PageQuery {
                page,
                page_size: size,
            },
        ))
    }

    /// `GET /adminapi/deposit/account/adjustLogList`。
    pub async fn adjust_log_list(
        &self,
        owner_type: Option<i32>,
        owner_id: Option<i32>,
        page: Option<u64>,
        size: Option<u64>,
    ) -> Result<PageResult<nextrade_core::models::merchant::deposit_account::DepositAdjustLogVO>> {
        use nextrade_core::models::merchant::deposit_account::{adjust_source_text, DepositAdjustLogVO};
        let (owner_type, owner_id) = self.resolve_owner(owner_type, owner_id, false).await?;
        let page = page.unwrap_or(1).max(1);
        let size = size.unwrap_or(15).clamp(1, 100);
        let (total, rows) = self
            .repo()
            .paginate_adjust_logs(owner_type, owner_id, page, size)
            .await?;
        let owner_name = self.owner_name(owner_type, owner_id).await?;
        let mut list = Vec::with_capacity(rows.len());
        for r in &rows {
            list.push(DepositAdjustLogVO {
                shop_deposit_adjust_id: r.shop_deposit_adjust_id,
                owner_type: r.owner_type,
                owner_type_text: owner_type_text(r.owner_type).into(),
                owner_id: r.owner_id,
                owner_name: owner_name.clone(),
                shop_id: (r.owner_type == OWNER_SHOP).then_some(r.owner_id),
                before_required: r.before_required.to_string(),
                after_required: r.after_required.to_string(),
                source: r.source,
                source_text: adjust_source_text(r.source).into(),
                admin_id: r.admin_id,
                admin_name: self.repo().admin_name(r.admin_id).await?,
                reason: r.reason.clone(),
                add_time: r.add_time.to_string(),
            });
        }
        Ok(PageResult::new(
            total,
            list,
            &PageQuery {
                page,
                page_size: size,
            },
        ))
    }

    /// `POST /adminapi/deposit/account/adjust`：手动调整应缴。
    pub async fn adjust(&self, dto: &DepositAdjustDTO, admin_id: i32) -> Result<()> {
        let (owner_type, owner_id) = self.resolve_owner(dto.owner_type, dto.resolved_owner_id(), true).await?;
        let adjust_type = dto.adjust_type.unwrap_or(2);
        if adjust_type != 1 && adjust_type != 2 {
            return Err(Error::biz(SERVICE_DATA_ERROR, ADJUST_TYPE_INVALID));
        }
        if adjust_type == 2 {
            match dto.required_manual {
                Some(v) if v >= Decimal::ZERO => {}
                _ => return Err(Error::biz(SERVICE_DATA_ERROR, REQUIRED_MANUAL_INVALID)),
            }
        }
        let now = i32::try_from(now_seconds()).unwrap_or(0);
        self.repo()
            .apply_adjust(dto, owner_type, owner_id, admin_id, now)
            .await?;
        Ok(())
    }

    /// `POST /adminapi/deposit/account/deduct`：赔付扣除。
    pub async fn deduct(&self, dto: &DepositDeductDTO, admin_id: i32) -> Result<()> {
        let (owner_type, owner_id) = self.resolve_owner(dto.owner_type, dto.resolved_owner_id(), true).await?;
        match dto.amount {
            Some(v) if v > Decimal::ZERO => {}
            _ => return Err(Error::biz(SERVICE_DATA_ERROR, DEDUCT_AMOUNT_INVALID)),
        }
        let now = i32::try_from(now_seconds()).unwrap_or(0);
        if !self
            .repo()
            .apply_deduct(dto, owner_type, owner_id, admin_id, now)
            .await?
        {
            return Err(Error::biz(
                SERVICE_DATA_ERROR,
                if owner_type == OWNER_SHOP {
                    SHOP_NOT_EXIST
                } else {
                    VENDOR_NOT_EXIST
                },
            ));
        }
        Ok(())
    }

    /// `POST /adminapi/deposit/account/sendWarning`：发送保证金预警站内信通知（对齐 Java DepositAccountAdminServiceImpl#sendWarning）。
    pub async fn send_warning(&self, dto: &DepositSendWarningDTO) -> Result<()> {
        let owner_type = dto.owner_type.unwrap_or(1);
        let owner_id = dto
            .resolved_owner_id()
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::biz_msg("持有者id非法"))?;

        let is_supplier = owner_type == 2;
        let owner_label = if is_supplier { "供应商" } else { "店铺" };

        let account = self.repo().find_by_owner(owner_type, owner_id).await?;
        let pending = account
            .as_ref()
            .map(|a| pending_amount(a.paid, required(a)))
            .unwrap_or(Decimal::ZERO);

        let title = dto
            .title
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("保证金预警通知");
        let content = if let Some(c) = dto.content.as_deref().filter(|s| !s.trim().is_empty()) {
            c.to_string()
        } else if pending > Decimal::ZERO {
            format!("您的{owner_label}保证金待补缴 {pending} 元，请尽快补足，以免影响正常经营。")
        } else {
            format!("您的{owner_label}保证金存在异常，请前往财务中心-保证金查看并处理。")
        };

        let related_data = serde_json::json!({
            "ownerType": owner_type,
            "ownerId": owner_id,
            "shopId": if is_supplier { 0 } else { owner_id }
        })
        .to_string();

        let target_shop_id = if is_supplier { 0 } else { owner_id };

        self.repos
            .system
            .admin_msg
            .create_message(
                44, // DEPOSIT_WARNING
                target_shop_id,
                0,
                title,
                &content,
                &related_data,
            )
            .await?;

        tracing::info!(
            "保证金预警发送成功: owner_type={owner_type}, owner_id={owner_id}, title={title}"
        );
        Ok(())
    }

    /// 保证金双缓冲到期扫描（Java `DepositLifecycleService#runDueCheck`）。
    ///
    /// 基础实现：功能关闭 → 跳过；待补缴 ≤0 且已处罚 → 恢复为正常；
    /// 待补缴 >0 且缓冲到期 → 按配置动作首次处罚（已处罚不升级）；
    /// 全店下架动作会下架店铺在售商品。供应商维度暂不物理下架。
    pub async fn run_due_check(&self) -> Result<()> {
        use chrono::TimeZone as _;
        use nextrade_core::models::merchant::deposit_setting::DepositSetting;
        let setting = crate::merchant::deposit_setting::DepositSettingService::new(
            self.repos.clone(),
        )
        .detail()
        .await?;
        if setting.deposit_enable != Some(1) {
            tracing::info!("保证金功能未开启，跳过到期扫描");
            return Ok(());
        }
        let collect_action = setting.deposit_collect_buffer_action.unwrap_or(1);
        let supplement_action = setting.deposit_supplement_buffer_action.unwrap_or(1);
        let accounts = self.repo().list_all_accounts().await?;
        let now = nextrade_common::utils::now_seconds() as i32;
        let mut penalized = 0u32;
        let mut restored = 0u32;
        for account in accounts {
            let required = account
                .required_manual
                .unwrap_or(account.required_snapshot);
            let pending = nextrade_core::models::merchant::deposit_account::pending_amount(
                account.paid,
                required,
            );
            if pending <= rust_decimal::Decimal::ZERO {
                if account.penalty_status != 0 {
                    self.repo()
                        .restore_account_penalty(account.owner_type, account.owner_id, now)
                        .await?;
                    restored += 1;
                }
                continue;
            }
            if account.penalty_status != 0 {
                // 首次处罚后冻结，不随后到期升级
                continue;
            }
            let collect_expired =
                account.collect_due_time > 0 && now >= account.collect_due_time;
            let supplement_expired =
                account.supplement_due_time > 0 && now >= account.supplement_due_time;
            let action = match (collect_expired, supplement_expired) {
                (false, false) => 0,
                (true, false) => collect_action,
                (false, true) => supplement_action,
                (true, true) => {
                    if account.collect_due_time < account.supplement_due_time {
                        collect_action
                    } else if account.supplement_due_time < account.collect_due_time {
                        supplement_action
                    } else {
                        collect_action.max(supplement_action)
                    }
                }
            };
            if action == 0 {
                continue;
            }
            self.repo()
                .update_penalty_status(account.shop_deposit_id, action, now)
                .await?;
            penalized += 1;
            if action == 2 {
                match self
                    .repo()
                    .record_and_off_shelf_products(account.owner_type, account.owner_id, now)
                    .await
                {
                    Ok(n) => tracing::info!(
                        "保证金全店下架: owner_type={}, owner_id={}, 商品 {n} 行",
                        account.owner_type,
                        account.owner_id
                    ),
                    Err(e) => tracing::warn!(
                        "保证金全店下架失败: owner_type={}, owner_id={}, {e}",
                        account.owner_type,
                        account.owner_id
                    ),
                }
            }
        }
        tracing::info!(
            "保证金到期扫描完成: 处罚 {penalized}, 恢复 {restored}"
        );
        Ok(())
    }

    /// `restore`：实缴足额后清除惩罚状态与到期时间，并将被处罚下架的商品恢复上架（对齐 Java DepositLifecycleServiceImpl#restore）。
    pub async fn restore(&self, owner_type: i32, owner_id: i32) -> Result<()> {
        let account = self.repo().find_by_owner(owner_type, owner_id).await?;
        let Some(account) = account else {
            return Ok(());
        };
        let req = required(&account);
        let pending = pending_amount(account.paid, req);
        if pending <= Decimal::ZERO {
            let now = nextrade_common::utils::now_seconds() as i32;
            self.repo()
                .restore_account_penalty(owner_type, owner_id, now)
                .await?;
            tracing::info!(
                "保证金已足额，恢复处罚状态与下架商品: owner_type={}, owner_id={}",
                owner_type,
                owner_id
            );
        }
        Ok(())
    }

    async fn resolve_owner(
        &self,
        owner_type: Option<i32>,
        owner_id: Option<i32>,
        require_exists: bool,
    ) -> Result<(i32, i32)> {
        let owner_type = owner_type.unwrap_or(OWNER_SHOP);
        if owner_type != OWNER_SHOP && owner_type != 2 {
            return Err(Error::biz(SERVICE_DATA_ERROR, PARAM_INVALID));
        }
        let owner_id = owner_id.ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, OWNER_ID_INVALID))?;
        if owner_id <= 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, OWNER_ID_INVALID));
        }
        if require_exists && !self.repo().owner_exists(owner_type, owner_id).await? {
            return Err(Error::biz(
                SERVICE_DATA_ERROR,
                if owner_type == OWNER_SHOP {
                    SHOP_NOT_EXIST
                } else {
                    VENDOR_NOT_EXIST
                },
            ));
        }
        Ok((owner_type, owner_id))
    }

    async fn owner_name(&self, owner_type: i32, owner_id: i32) -> Result<String> {
        Ok(self
            .repo()
            .owner_name(owner_type, owner_id)
            .await?
            .unwrap_or_default())
    }

    async fn account_vo(&self, row: &AccountRow) -> Result<DepositAccountVO> {
        let required = required(row);
        let status = compute_deposit_status(required, row.paid, row.penalty_status);
        Ok(DepositAccountVO {
            owner_type: row.owner_type,
            owner_type_text: owner_type_text(row.owner_type).into(),
            subject_type: 0,
            subject_type_text: subject_type_text(0).into(),
            owner_id: row.owner_id,
            owner_name: self.owner_name(row.owner_type, row.owner_id).await?,
            shop_id: (row.owner_type == OWNER_SHOP).then_some(row.owner_id),
            shop_title: None,
            merchant_name: None,
            business_category: None,
            required: required.to_string(),
            paid: money(row.paid).to_string(),
            pending: pending_amount(row.paid, required).to_string(),
            deposit_status: status,
            deposit_status_text: deposit_status_text(status).into(),
        })
    }
}

/// Java `required` 口径：手动值优先，否则取应缴快照。
fn required(row: &AccountRow) -> Decimal {
    row.required_manual.unwrap_or(row.required_snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_prefers_manual_value() {
        let row = AccountRow {
            shop_deposit_id: 1,
            owner_type: 1,
            owner_id: 2,
            paid: Decimal::ZERO,
            required_manual: Some(Decimal::new(500, 0)),
            required_snapshot: Decimal::new(100, 0),
            penalty_status: 0,
            add_time: 0,
        };
        assert_eq!(required(&row), Decimal::new(500, 0));
        let row = AccountRow {
            required_manual: None,
            ..row
        };
        assert_eq!(required(&row), Decimal::new(100, 0));
    }

    #[test]
    fn adjust_and_deduct_validations_match_java_messages() {
        // 这里只校验常量文案（服务方法需要 DB，纯逻辑在 models 中已测）
        assert_eq!(ADJUST_TYPE_INVALID, "调整方式非法");
        assert_eq!(REQUIRED_MANUAL_INVALID, "手动应缴金额非法");
        assert_eq!(DEDUCT_AMOUNT_INVALID, "扣除金额非法");
        assert_eq!(OWNER_ID_INVALID, "持有者id非法");
    }
}
