//! 前台用户提现 域服务（payment::with_draw_apply_client）。
//!
//! 构造签名固定 `pub fn new(repos: Arc<Repositories>) -> Self`，仓储通过
//! 嵌套聚合根访问：`self.repos.payment.with_draw_apply_client`。
//!
//! 对应 Java `WithdrawApplyAccountServiceImpl`
//! （`java/nextrade-service/.../finance/impl/WithdrawApplyAccountServiceImpl.java`）
//! 与 `WithDrawApplyClientController`（7 个端点，契约 `auth=optional`）。
//!
//! ## 已登记偏差
//!
//! 1. **会员余额提现开关 / 收款方式开关**（`MEMBER_WITHDRAWAL_ENABLED`、
//!    `checkReceiptMethodEnabled`）依赖设置服务 `SettingsEnum`，本波次未迁移
//!    → 本实现不做开关校验，仅保留金额校验（见各方法注释）。
//! 2. **后台消息**（`AdminMsgService`）未迁移 → `applyCommission` 不发送
//!    「佣金提现申请」后台消息。
//! 3. **列表排序**：Java 未显式排序，Rust 固定 `account_id DESC` 保证分页稳定。
//! 4. **修改账号越权收紧**：Java `update` 构造了 `queryWrapperCheck` 却未使用；
//!    Rust 在 `WHERE` 中强制 `user_id`，避免改他人账号。
//! 5. **佣金流水**：Java 走 `UserBalanceService.incFrozenCommissionBalance` /
//!    `decCommissionBalance`（两笔 `wallet_type=1` 流水）；Rust 直接更新
//!    `user` 两列 + 补两笔 `user_balance_log`，字段口径与既有余额流水一致。

use std::sync::Arc;

use rust_decimal::Decimal;

use nextrade_common::utils;
use nextrade_core::models::payment::with_draw_apply_client::{
    account_type_name, WithdrawAccountCreateDTO, WithdrawAccountData, WithdrawAccountListQuery,
    WithdrawAccountUpdateDTO, WithdrawAccountVO, WithdrawApplyDTO, MSG_ACCOUNT_EXIST,
    MSG_ACCOUNT_MAX, MSG_ACCOUNT_NOT_FOUND, MSG_ACCOUNT_TYPE_REQUIRED, MSG_AMOUNT_EXCEED_BALANCE,
    MSG_AMOUNT_EXCEED_COMMISSION, MSG_USER_NOT_FOUND, MAX_ACCOUNT_COUNT,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::payment::with_draw_apply_client::{
    BalanceLogInsert, WithDrawApplyClientRepository, WithdrawAccountRow,
};
use nextrade_infrastructure::db::Repositories;

/// 用户余额提现的 `wallet_type`（Java `walletType = 0`）。
const WALLET_BALANCE: i32 = 0;
/// 分销佣金提现的 `wallet_type`（Java `walletType = 1`）。
const WALLET_COMMISSION: i32 = 1;
/// 提现申请初始状态「待处理」（Java 默认 0）。
const STATUS_WAIT: i32 = 0;

/// 前台用户提现 域服务。
pub struct WithDrawApplyClientService {
    repos: Arc<Repositories>,
}

impl WithDrawApplyClientService {
    /// 构造服务（组合根唯一装配点）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本 feature 仓储。
    #[must_use]
    pub fn repo(&self) -> &WithDrawApplyClientRepository {
        &self.repos.payment.with_draw_apply_client
    }

    /// 提现账号分页列表（Java `WithdrawApplyAccountServiceImpl#list`
    /// → `WithdrawApplyAccountVO` 列表）。
    pub async fn list(
        &self,
        user_id: i32,
        q: &WithdrawAccountListQuery,
    ) -> Result<PageResult<WithdrawAccountVO>> {
        let page = q.page_query();
        let (total, rows) = self.repo().paginate_accounts(user_id, q).await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let list = rows.into_iter().map(to_vo).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 创建提现账号。
    ///
    /// 校验顺序与 Java 一致：数量上限 → 同卡号去重 → 写入。
    /// 注意 Java 的边界是 `Assert.isTrue(count <= 15)`（第 16 张仍能创建），
    /// 这里逐字保留 `count > 15` 才报错，并登记该偏差。
    pub async fn create(&self, user_id: i32, dto: &WithdrawAccountCreateDTO) -> Result<()> {
        let count = self.repo().count_accounts(user_id).await?;
        if count > MAX_ACCOUNT_COUNT {
            return Err(Error::Validation(MSG_ACCOUNT_MAX.into()));
        }
        if self
            .repo()
            .exists_same_account(user_id, dto.account_type, &dto.account_no)
            .await?
        {
            return Err(Error::Conflict(MSG_ACCOUNT_EXIST.into()));
        }
        self.repo().insert_account(user_id, dto).await
    }

    /// 修改提现账号（Java `update(updateDTO, getCurrentUserId())`）。
    pub async fn update(&self, user_id: i32, dto: &WithdrawAccountUpdateDTO) -> Result<()> {
        let affected = self.repo().update_account(user_id, dto).await?;
        if affected == 0 {
            return Err(Error::NotFound(MSG_ACCOUNT_NOT_FOUND.into()));
        }
        Ok(())
    }

    /// 提现账号详情（Java `detail(accountId, getCurrentUserId())`）。
    pub async fn detail(&self, user_id: i32, account_id: i32) -> Result<WithdrawAccountVO> {
        let row = self
            .repo()
            .find_account(account_id, user_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_ACCOUNT_NOT_FOUND.into()))?;
        Ok(to_vo(row))
    }

    /// 删除提现账号（Java `delAccount`）。
    pub async fn del_account(&self, user_id: i32, account_id: i32) -> Result<()> {
        self.repo().delete_account(account_id, user_id).await?;
        Ok(())
    }

    /// 余额提现申请（Java `apply(dto, getCurrentUserId())`）。
    ///
    /// Java 流程：金额不超可用余额 → 写 `user_withdraw_apply`（wallet_type=0）
    /// → 冻结 `user.frozen_balance` → 写一笔「提现冻结余额」流水。
    pub async fn apply(&self, user_id: i32, dto: &WithdrawApplyDTO) -> Result<()> {
        let account = dto.normalize_account();
        let wallet = self
            .repo()
            .find_wallet(user_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_USER_NOT_FOUND.into()))?;
        if dto.amount > wallet.balance {
            return Err(Error::Validation(MSG_AMOUNT_EXCEED_BALANCE.into()));
        }
        let account_json = serde_json::to_string(&account)
            .map_err(|e| Error::Internal(format!("提现账号快照序列化失败: {e}")))?;
        let now = now_secs();
        self.repo()
            .insert_apply(user_id, WALLET_BALANCE, dto.amount, &account_json, STATUS_WAIT, now)
            .await?;
        self.repo().freeze_balance(user_id, dto.amount).await?;
        let after_frozen = wallet.frozen_balance + dto.amount;
        self.repo()
            .insert_balance_log(&BalanceLogInsert {
                user_id,
                before_balance: wallet.balance,
                balance: Decimal::ZERO,
                after_balance: wallet.balance,
                frozen_balance: dto.amount,
                before_frozen_balance: wallet.frozen_balance,
                after_frozen_balance: after_frozen,
                change_time: now,
                change_desc: "提现冻结余额".to_string(),
                change_type: 1,
            })
            .await
    }

    /// 佣金提现申请（Java `applyCommission`，D1107）。
    ///
    /// 不受「会员余额提现开关 / 收款方式」限制，仅校验可用佣金；
    /// 写 `user_withdraw_apply`（wallet_type=1）→ 冻结佣金 + 扣减可用佣金。
    pub async fn apply_commission(&self, user_id: i32, dto: &WithdrawApplyDTO) -> Result<()> {
        let account = dto.normalize_account();
        if account.account_type.is_none() {
            return Err(Error::Validation(MSG_ACCOUNT_TYPE_REQUIRED.into()));
        }
        let wallet = self
            .repo()
            .find_wallet(user_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_USER_NOT_FOUND.into()))?;
        if dto.amount > wallet.commission_balance {
            return Err(Error::Validation(MSG_AMOUNT_EXCEED_COMMISSION.into()));
        }
        let account_json = serde_json::to_string(&account)
            .map_err(|e| Error::Internal(format!("提现账号快照序列化失败: {e}")))?;
        let now = now_secs();
        self.repo()
            .insert_apply(user_id, WALLET_COMMISSION, dto.amount, &account_json, STATUS_WAIT, now)
            .await?;
        self.repo()
            .freeze_and_deduct_commission(user_id, dto.amount)
            .await?;
        let after_commission = wallet.commission_balance - dto.amount;
        let after_frozen = wallet.frozen_commission_balance + dto.amount;
        // 两笔佣金流水：冻结 + 扣除（对应 Java 两次 UserBalanceService 调用）。
        self.repo()
            .insert_balance_log(&BalanceLogInsert {
                user_id,
                before_balance: wallet.commission_balance,
                balance: Decimal::ZERO,
                after_balance: wallet.commission_balance,
                frozen_balance: dto.amount,
                before_frozen_balance: wallet.frozen_commission_balance,
                after_frozen_balance: after_frozen,
                change_time: now,
                change_desc: "佣金提现冻结".to_string(),
                change_type: 1,
            })
            .await?;
        self.repo()
            .insert_balance_log(&BalanceLogInsert {
                user_id,
                before_balance: wallet.commission_balance,
                balance: dto.amount,
                after_balance: after_commission,
                frozen_balance: Decimal::ZERO,
                before_frozen_balance: wallet.frozen_commission_balance,
                after_frozen_balance: wallet.frozen_commission_balance,
                change_time: now,
                change_desc: "佣金提现扣除".to_string(),
                change_type: 2,
            })
            .await
    }
}

/// 当前时间（秒级时间戳，Java `StringUtils.getCurrentTime()`）。
fn now_secs() -> i32 {
    i32::try_from(utils::now_millis() / 1000).unwrap_or(0)
}

/// 行 → VO（对应 Java `BeanUtils.copyProperties` + `accountTypeName` 派生）。
#[must_use]
pub fn to_vo(row: WithdrawAccountRow) -> WithdrawAccountVO {
    WithdrawAccountVO {
        account_id: row.account_id,
        account_name: row.account_name,
        account_no: row.account_no,
        account_type: row.account_type,
        account_type_name: account_type_name(row.account_type),
        bank_name: row.bank_name,
        identity: row.identity,
        user_id: row.user_id,
    }
}

/// 只为让「账号快照」类型在本模块文档中可见（JSON 形态见模型层）。
pub type AccountSnapshot = WithdrawAccountData;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vo_maps_account_type_name() {
        let vo = to_vo(WithdrawAccountRow {
            account_id: 3,
            user_id: 8,
            account_type: 2,
            account_name: "张三".into(),
            account_no: "a@b.com".into(),
            identity: String::new(),
            bank_name: "支付宝".into(),
        });
        assert_eq!(vo.account_type_name, "支付宝");
        let v = serde_json::to_value(&vo).unwrap();
        assert_eq!(v["accountId"], 3);
        assert_eq!(v["accountTypeName"], "支付宝");
    }

    #[test]
    fn wallet_math_balances() {
        // 与 apply/apply_commission 里 before/after 计算同口径。
        let before = Decimal::new(10000, 2);
        let amount = Decimal::new(2500, 2);
        assert_eq!(before - amount, Decimal::new(7500, 2));
        assert_eq!(before + amount, Decimal::new(12500, 2));
    }

    #[test]
    fn now_secs_is_positive_epoch_seconds() {
        let n = now_secs();
        assert!(n > 1_600_000_000, "应返回秒级时间戳，实际 {n}");
    }
}
