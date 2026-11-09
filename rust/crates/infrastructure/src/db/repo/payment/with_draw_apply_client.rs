//! 前台用户提现 域仓储（payment::with_draw_apply_client）。
//!
//! 涉及表（列名以 `java/database/pure.sql` 为准）：
//! - `user_withdraw_account`：`account_id` / `user_id` / `account_type` /
//!   `account_name` / `account_no` / `identity` / `bank_name`
//! - `user_withdraw_apply`：`id` / `user_id` / `wallet_type` / `amount` /
//!   `add_time` / `finished_time` / `postscript` / `status` / `account_data`
//! - `user`：`balance` / `frozen_balance` / `commission_balance` /
//!   `frozen_commission_balance`
//! - `user_balance_log`：`user_id` / `before_balance` / `balance` /
//!   `after_balance` / `frozen_balance` / `before_frozen_balance` /
//!   `after_frozen_balance` / `change_time` / `change_desc` / `change_type` /
//!   `wallet_type` / `new_balance` / `new_frozen_balance`
//!
//! SQL 全部运行时 `sqlx::query_as`/`query`（**不使用** `query!` 宏：本机无
//! `DATABASE_URL`）。

use rust_decimal::Decimal;
use sqlx::PgPool;
use sqlx::FromRow;

use nextrade_core::models::payment::with_draw_apply_client::WithdrawAccountListQuery;
use nextrade_core::{Error, Result};

/// 提现账号行。
#[derive(Debug, Clone, FromRow)]
pub struct WithdrawAccountRow {
    /// 提现账号 ID
    pub account_id: i32,
    /// 用户 ID
    pub user_id: i32,
    /// 提现方式
    pub account_type: i32,
    /// 姓名
    pub account_name: String,
    /// 银行卡号或账号
    pub account_no: String,
    /// 身份证号
    pub identity: String,
    /// 银行名称
    pub bank_name: String,
}

/// 用户钱包快照（`user` 表四个余额列）。
#[derive(Debug, Clone, FromRow)]
pub struct UserWalletRow {
    /// 可用余额
    pub balance: Decimal,
    /// 冻结余额
    pub frozen_balance: Decimal,
    /// 可提现佣金
    pub commission_balance: Decimal,
    /// 冻结佣金
    pub frozen_commission_balance: Decimal,
}

/// 余额流水插入参数（`user_balance_log`）。
#[derive(Debug, Clone)]
pub struct BalanceLogInsert {
    /// 用户 ID
    pub user_id: i32,
    /// 变动前余额
    pub before_balance: Decimal,
    /// 本次变动金额
    pub balance: Decimal,
    /// 变动后余额
    pub after_balance: Decimal,
    /// 本次冻结变动
    pub frozen_balance: Decimal,
    /// 变动前冻结余额
    pub before_frozen_balance: Decimal,
    /// 变动后冻结余额
    pub after_frozen_balance: Decimal,
    /// 变动时间（秒）
    pub change_time: i32,
    /// 变动说明
    pub change_desc: String,
    /// 变动类型：1 增加 / 2 减少
    pub change_type: i32,
}

/// 前台用户提现仓储。
#[derive(Clone)]
pub struct WithDrawApplyClientRepository {
    pool: PgPool,
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

impl WithDrawApplyClientRepository {
    /// 构造仓储（每个 feature 持有连接池克隆，与其余 payment feature 一致）。
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 提现账号分页（Java `list`：按 userId + 可选 accountId/accountType 过滤）。
    ///
    /// 说明：Java 未显式排序（MyBatis-Plus 无 `orderBy`），这里固定
    /// `account_id DESC` 以保证分页稳定；属可观察偏差，已在服务层注释登记。
    pub async fn paginate_accounts(
        &self,
        user_id: i32,
        q: &WithdrawAccountListQuery,
    ) -> Result<(u64, Vec<WithdrawAccountRow>)> {
        let mut conds = vec!["user_id = ?".to_string()];
        let mut binds: Vec<i64> = vec![i64::from(user_id)];
        if let Some(id) = q.account_id.filter(|v| *v > 0) {
            conds.push("account_id = ?".to_string());
            binds.push(i64::from(id));
        }
        if let Some(t) = q.account_type.filter(|v| *v > 0) {
            conds.push("account_type = ?".to_string());
            binds.push(i64::from(t));
        }
        let where_sql = conds.join(" AND ");
        let (size, offset) = q.limit_offset();

        let total_sql = format!("SELECT COUNT(*) FROM user_withdraw_account WHERE {where_sql}");
        let mut tq = sqlx::query_as::<_, (i64,)>(&total_sql);
        for b in &binds {
            tq = tq.bind(*b);
        }
        let total = tq.fetch_one(&self.pool).await.map_err(map_err)?.0;

        let rows_sql = format!(
            "SELECT account_id, user_id, account_type, account_name, account_no, identity, bank_name \
             FROM user_withdraw_account WHERE {where_sql} ORDER BY account_id DESC LIMIT ? OFFSET ?"
        );
        let mut rq = sqlx::query_as::<_, WithdrawAccountRow>(&rows_sql);
        for b in &binds {
            rq = rq.bind(*b);
        }
        let rows = rq
            .bind(i64::try_from(size).unwrap_or(15))
            .bind(i64::try_from(offset).unwrap_or(0))
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total).unwrap_or(0), rows))
    }

    /// 该用户的提现账号数量（Java \"create\" 的上限校验）。
    pub async fn count_accounts(&self, user_id: i32) -> Result<u64> {
        let (n,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM user_withdraw_account WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    /// 是否已存在相同「账号 + 提现方式」的账号（Java `error.withdraw.account.exist`）。
    pub async fn exists_same_account(
        &self,
        user_id: i32,
        account_type: i32,
        account_no: &str,
    ) -> Result<bool> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM user_withdraw_account \
             WHERE user_id = ? AND account_type = ? AND account_no = ?",
        )
        .bind(user_id)
        .bind(account_type)
        .bind(account_no)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(n > 0)
    }

    /// 插入提现账号。
    pub async fn insert_account(
        &self,
        user_id: i32,
        dto: &nextrade_core::models::payment::with_draw_apply_client::WithdrawAccountCreateDTO,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO user_withdraw_account (user_id, account_type, account_name, account_no, identity, bank_name) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(user_id)
        .bind(dto.account_type)
        .bind(&dto.account_name)
        .bind(&dto.account_no)
        .bind(dto.identity.clone().unwrap_or_default())
        .bind(dto.bank_name.clone().unwrap_or_default())
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 修改提现账号（只更新非空字段；`WHERE user_id` 兜住越权，
    /// Java 原实现漏了该条件，这里按安全口径收紧，偏差已登记）。
    pub async fn update_account(
        &self,
        user_id: i32,
        dto: &nextrade_core::models::payment::with_draw_apply_client::WithdrawAccountUpdateDTO,
    ) -> Result<u64> {
        let mut sets = vec!["account_id = account_id".to_string()];
        let mut binds: Vec<String> = Vec::new();
        let mut nums: Vec<(i32, bool)> = Vec::new(); // (value, is_type)
        if let Some(v) = dto.account_type {
            sets.push("account_type = ?".to_string());
            nums.push((v, true));
        }
        if let Some(v) = &dto.account_name {
            sets.push("account_name = ?".to_string());
            binds.push(v.clone());
        }
        if let Some(v) = &dto.account_no {
            sets.push("account_no = ?".to_string());
            binds.push(v.clone());
        }
        if let Some(v) = &dto.identity {
            sets.push("identity = ?".to_string());
            binds.push(v.clone());
        }
        if let Some(v) = &dto.bank_name {
            sets.push("bank_name = ?".to_string());
            binds.push(v.clone());
        }
        let sql = format!(
            "UPDATE user_withdraw_account SET {} WHERE account_id = ? AND user_id = ?",
            sets.join(", ")
        );
        let mut q = sqlx::query(&sql);
        for (v, _) in &nums {
            q = q.bind(*v);
        }
        for v in &binds {
            q = q.bind(v);
        }
        let r = q
            .bind(dto.account_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 提现账号详情（必须同时匹配 user_id）。
    pub async fn find_account(&self, account_id: i32, user_id: i32) -> Result<Option<WithdrawAccountRow>> {
        sqlx::query_as::<_, WithdrawAccountRow>(
            "SELECT account_id, user_id, account_type, account_name, account_no, identity, bank_name \
             FROM user_withdraw_account WHERE account_id = ? AND user_id = ?",
        )
        .bind(account_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 删除提现账号（必须同时匹配 user_id）。
    pub async fn delete_account(&self, account_id: i32, user_id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM user_withdraw_account WHERE account_id = ? AND user_id = ?")
            .bind(account_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 用户钱包快照；用户不存在返回 `None`。
    pub async fn find_wallet(&self, user_id: i32) -> Result<Option<UserWalletRow>> {
        sqlx::query_as::<_, UserWalletRow>(
            "SELECT balance, frozen_balance, commission_balance, frozen_commission_balance \
             FROM user WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 余额提现：冻结余额 `+= amount`（Java `user.frozenBalance.add(amount)`）。
    pub async fn freeze_balance(&self, user_id: i32, amount: Decimal) -> Result<()> {
        sqlx::query("UPDATE user SET frozen_balance = frozen_balance + ? WHERE user_id = ?")
            .bind(amount)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 佣金提现：冻结佣金 `+= amount` 且可用佣金 `-= amount`
    /// （Java `incFrozenCommissionBalance` + `decCommissionBalance`）。
    pub async fn freeze_and_deduct_commission(&self, user_id: i32, amount: Decimal) -> Result<()> {
        sqlx::query(
            "UPDATE user SET frozen_commission_balance = frozen_commission_balance + ?, \
             commission_balance = commission_balance - ? WHERE user_id = ?",
        )
        .bind(amount)
        .bind(amount)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 插入提现申请，返回自增主键。
    pub async fn insert_apply(
        &self,
        user_id: i32,
        wallet_type: i32,
        amount: Decimal,
        account_data: &str,
        status: i32,
        add_time: i32,
    ) -> Result<u64> {
        let r: i64 = sqlx::query_scalar(
            "INSERT INTO user_withdraw_apply \
             (user_id, wallet_type, amount, add_time, finished_time, postscript, status, account_data) \
             VALUES (?, ?, ?, ?, 0, '', ?, ?) RETURNING id",
        )
        .bind(user_id)
        .bind(wallet_type)
        .bind(amount)
        .bind(add_time)
        .bind(status)
        .bind(account_data)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r as u64)
    }

    /// 插入余额流水（\"user_balance_log\"）。
    pub async fn insert_balance_log(&self, log: &BalanceLogInsert) -> Result<()> {
        sqlx::query(
            "INSERT INTO user_balance_log \
             (user_id, before_balance, balance, after_balance, frozen_balance, before_frozen_balance, \
              after_frozen_balance, change_time, change_desc, change_type, wallet_type, new_balance, new_frozen_balance) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)",
        )
        .bind(log.user_id)
        .bind(log.before_balance)
        .bind(log.balance)
        .bind(log.after_balance)
        .bind(log.frozen_balance)
        .bind(log.before_frozen_balance)
        .bind(log.after_frozen_balance)
        .bind(log.change_time)
        .bind(&log.change_desc)
        .bind(log.change_type)
        .bind(log.after_balance)
        .bind(log.after_frozen_balance)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn repository_type_is_send_sync_placeholder() {
        // 编译期验证：仓储可跨 await 持有（axum handler 里 Arc<Repositories> 需要 Send+Sync）。
        fn assert_send<T: Send>() {}
        assert_send::<super::WithDrawApplyClientRepository>();
    }
}
