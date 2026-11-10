//! 提现申请域仓储（payment::user_withdraw_apply）。
//!
//! 对应原 Java `UserWithdrawApplyMapper` + `UserMapper` + `UserBalanceServiceImpl`
//! 的余额/冻结余额/佣金账户增减（`user` 列 + `user_balance_log` 流水）。
//!
//! 表：`user_withdraw_apply`（9 列，主键 `id`）；写入 `user`（balance/
//! frozen_balance/commission_balance/frozen_commission_balance）与
//! `user_balance_log`（`wallet_type=0/1`）。

use std::collections::HashMap;

use rust_decimal::Decimal;
use sqlx::{FromRow,Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::UserWithdrawApply;
use nextrade_core::models::payment::user_withdraw_apply::{
    limit_offset, UserWithdrawApplyListQuery,
};
use nextrade_core::{Error, Result};

/// `user_withdraw_apply` 固定 SELECT 列清单。
const WITHDRAW_SELECT: &str = "SELECT id, user_id, wallet_type, amount, add_time, finished_time, postscript, status, account_data FROM user_withdraw_apply";

/// 用户装配行。
#[derive(Debug, Clone, Default, FromRow)]
pub struct WithdrawUserRow {
    pub user_id: i32,
    pub username: String,
}

/// 用户余额四列。
#[derive(Debug, Clone, Default, FromRow)]
pub struct WithdrawBalanceRow {
    pub balance: Decimal,
    pub frozen_balance: Decimal,
    pub commission_balance: Decimal,
    pub frozen_commission_balance: Decimal,
}

/// 一次余额变动（Java `addBalanceLog(userId, desc, changeType, balanceAmount,
/// frozenBalanceAmount)`：二者恰有一个为 `Some`）。
#[derive(Debug, Clone)]
pub struct BalanceChange {
    pub user_id: i32,
    pub balance_amount: Option<Decimal>,
    pub frozen_amount: Option<Decimal>,
    /// 1 增加 / 2 减少
    pub change_type: i32,
    pub change_desc: String,
    pub change_time: i32,
}

/// 提现申请域仓储。
#[derive(Clone)]
pub struct UserWithdrawApplyRepository {
    pool: PgPool,
}

impl UserWithdrawApplyRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 开启事务（创建/审核的多次写入）。
    pub async fn begin(&self) -> Result<sqlx::Transaction<'static, Postgres>> {
        self.pool.begin().await.map_err(map_err)
    }

    // ————————————————————— user_withdraw_apply —————————————————————

    /// 按主键查询。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<UserWithdrawApply>> {
        sqlx::query_as::<_, UserWithdrawApply>(&format!(
            "{WITHDRAW_SELECT} WHERE id = $1 LIMIT 1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 分页（status/wallet_type/userIds + 排序）。
    pub async fn paginate(
        &self,
        q: &UserWithdrawApplyListQuery,
        user_ids: Option<&[i32]>,
        shop_scope: Option<i32>,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<UserWithdrawApply>)> {
        let mut cq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM user_withdraw_apply WHERE 1 = 1");
        Self::push_filters(&mut cq, q, user_ids, shop_scope);
        let total: (i64,) = cq
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut rq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{WITHDRAW_SELECT} WHERE 1 = 1"));
        Self::push_filters(&mut rq, q, user_ids, shop_scope);
        rq.push(q.order_clause());
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<UserWithdrawApply> = rq
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0, rows))
    }

    fn push_filters(
        qb: &mut QueryBuilder<'_, Postgres>,
        q: &UserWithdrawApplyListQuery,
        user_ids: Option<&[i32]>,
        shop_scope: Option<i32>,
    ) {
        if let Some(status) = q.effective_status() {
            qb.push(" AND status = ");
            qb.push_bind(status);
        }
        if let Some(wallet_type) = q.wallet_type {
            qb.push(" AND wallet_type = ");
            qb.push_bind(wallet_type);
        }
        if let Some(ids) = user_ids {
            qb.push(" AND user_id IN (");
            Self::push_int_list(qb, ids);
            qb.push(")");
        }
        if let Some(shop) = shop_scope {
            qb.push(" AND EXISTS (SELECT 1 FROM shop_member m WHERE m.user_id = user_withdraw_apply.user_id AND m.shop_id = ");
            qb.push_bind(shop);
            qb.push(" AND m.is_del = 0)");
        }
    }

    /// 提现申请人是否属于指定店铺（`shop_member` 有效关联）。
    pub async fn user_in_shop(&self, user_id: i32, shop_id: i32) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM shop_member WHERE user_id = $1 AND shop_id = $2 AND is_del = 0 LIMIT 1",
        )
        .bind(user_id)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.is_some())
    }

    /// 新增（Java `save`）。
    pub async fn insert_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        entity: &UserWithdrawApply,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO user_withdraw_apply (user_id, wallet_type, amount, add_time, finished_time, postscript, status, account_data) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id",
        )
        .bind(entity.user_id)
        .bind(entity.wallet_type)
        .bind(entity.amount)
        .bind(entity.add_time)
        .bind(entity.finished_time)
        .bind(&entity.postscript)
        .bind(entity.status)
        .bind(&entity.account_data)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 整行更新（Java `updateById`，服务层按 NOT_NULL 语义合并后调用）。
    pub async fn update(&self, entity: &UserWithdrawApply) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE user_withdraw_apply SET user_id = $1, wallet_type = $2, amount = $3, add_time = $4, finished_time = $5, postscript = $6, status = $7, account_data = $8 WHERE id = $9",
        )
        .bind(entity.user_id)
        .bind(entity.wallet_type)
        .bind(entity.amount)
        .bind(entity.add_time)
        .bind(entity.finished_time)
        .bind(&entity.postscript)
        .bind(entity.status)
        .bind(&entity.account_data)
        .bind(entity.id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 物理删除（Java `removeById`）。
    pub async fn delete(&self, id: i32) -> Result<bool> {
        let res = sqlx::query("DELETE FROM user_withdraw_apply WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 批量删除（Java `removeByIds`）。
    pub async fn delete_batch(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("DELETE FROM user_withdraw_apply WHERE id IN (");
        Self::push_int_list(&mut qb, ids);
        qb.push(")");
        let res = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 选中项中待处理数量（`status = 0`；del / batch 前置校验）。
    pub async fn count_pending_by_ids(&self, ids: &[i32]) -> Result<i64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT COUNT(*) FROM user_withdraw_apply WHERE status = 0 AND id IN (",
        );
        Self::push_int_list(&mut qb, ids);
        qb.push(")");
        let row: (i64,) = qb.build_query_as().fetch_one(&self.pool).await.map_err(map_err)?;
        Ok(row.0)
    }

    /// 单字段更新（Java `BaseServiceImpl#updateField`；列名经白名单归一）。
    pub async fn update_field(&self, id: i32, column: &str, val: &str) -> Result<u64> {
        debug_assert!(
            nextrade_core::models::payment::user_withdraw_apply::GENERAL_FIELDS.contains(&column),
            "update_field 列名未经过白名单映射: {column}"
        );
        let sql = format!("UPDATE user_withdraw_apply SET {column} = $1 WHERE id = $2");
        let res = sqlx::query(&sql)
            .bind(val)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— 用户装配 / 余额 —————————————————————

    /// 用户名模糊匹配的会员 ID（`user.username LIKE`）。
    pub async fn find_user_ids_by_username_like(&self, keyword: &str) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> =
            sqlx::query_as("SELECT user_id FROM \"user\" WHERE username LIKE $1")
                .bind(format!("%{keyword}%"))
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 批量用户信息。
    pub async fn find_users(&self, ids: &[i32]) -> Result<Vec<WithdrawUserRow>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT user_id, username FROM \"user\" WHERE user_id IN (");
        Self::push_int_list(&mut qb, ids);
        qb.push(")");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 事务内查用户余额四列（`FOR UPDATE`，Java `selectForUpdateById`）。
    pub async fn find_balance_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        user_id: i32,
    ) -> Result<Option<WithdrawBalanceRow>> {
        sqlx::query_as::<_, WithdrawBalanceRow>(
            "SELECT balance, frozen_balance, commission_balance, frozen_commission_balance FROM \"user\" WHERE user_id = $1 FOR UPDATE",
        )
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_err)
    }

    /// 余额账户变动（`wallet_type=0`）：更新用户列 + 一条流水。
    pub async fn apply_balance_change_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        change: &BalanceChange,
    ) -> Result<()> {
        let Some(row) = self.find_balance_in_tx(tx, change.user_id).await? else {
            return Err(Error::biz(500, "用户不存在"));
        };
        let mut balance = row.balance;
        let mut frozen = row.frozen_balance;
        if let Some(amount) = change.balance_amount {
            balance = if change.change_type == 1 {
                balance + amount
            } else {
                balance - amount
            };
        }
        if let Some(amount) = change.frozen_amount {
            frozen = if change.change_type == 1 {
                frozen + amount
            } else {
                frozen - amount
            };
        }
        sqlx::query("UPDATE \"user\" SET balance = $1, frozen_balance = $2 WHERE user_id = $3")
            .bind(balance)
            .bind(frozen)
            .bind(change.user_id)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?;
        self.insert_balance_log_in_tx(tx, change, row.balance, row.frozen_balance, balance, frozen, 0)
            .await
    }

    /// 佣金账户变动（`wallet_type=1`）。
    pub async fn apply_commission_change_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        change: &BalanceChange,
    ) -> Result<()> {
        let Some(row) = self.find_balance_in_tx(tx, change.user_id).await? else {
            return Err(Error::biz(500, "用户不存在"));
        };
        let mut balance = row.commission_balance;
        let mut frozen = row.frozen_commission_balance;
        if let Some(amount) = change.balance_amount {
            balance = if change.change_type == 1 {
                balance + amount
            } else {
                balance - amount
            };
        }
        if let Some(amount) = change.frozen_amount {
            frozen = if change.change_type == 1 {
                frozen + amount
            } else {
                frozen - amount
            };
        }
        sqlx::query(
            "UPDATE \"user\" SET commission_balance = $1, frozen_commission_balance = $2 WHERE user_id = $3",
        )
        .bind(balance)
        .bind(frozen)
        .bind(change.user_id)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        self.insert_balance_log_in_tx(
            tx,
            change,
            row.commission_balance,
            row.frozen_commission_balance,
            balance,
            frozen,
            1,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn insert_balance_log_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        change: &BalanceChange,
        before_balance: Decimal,
        before_frozen: Decimal,
        after_balance: Decimal,
        after_frozen: Decimal,
        wallet_type: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO user_balance_log (user_id, before_balance, balance, after_balance, frozen_balance, before_frozen_balance, after_frozen_balance, change_time, change_desc, change_type, wallet_type, new_balance, new_frozen_balance) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(change.user_id)
        .bind(before_balance)
        .bind(change.balance_amount.unwrap_or(Decimal::ZERO))
        .bind(after_balance)
        .bind(change.frozen_amount.unwrap_or(Decimal::ZERO))
        .bind(before_frozen)
        .bind(after_frozen)
        .bind(change.change_time)
        .bind(&change.change_desc)
        .bind(change.change_type)
        .bind(wallet_type)
        .bind(after_balance)
        .bind(after_frozen)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 便捷分页窗口。
    #[must_use]
    pub fn page_window(page: Option<i64>, size: Option<i64>) -> (i64, i64) {
        limit_offset(page, size)
    }

    /// 用户信息建索引。
    #[must_use]
    pub fn index_users(rows: Vec<WithdrawUserRow>) -> HashMap<i32, WithdrawUserRow> {
        rows.into_iter().map(|row| (row.user_id, row)).collect()
    }

    fn push_int_list(qb: &mut QueryBuilder<'_, Postgres>, ids: &[i32]) {
        {
            let mut sep = qb.separated(", ");
            for id in ids {
                sep.push_bind(*id);
            }
        }
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::payment::user_withdraw_apply::WITHDRAW_COLUMNS;

    #[test]
    fn select_matches_schema_columns() {
        let selected = WITHDRAW_SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap();
        assert_eq!(selected, WITHDRAW_COLUMNS.join(", "));
    }

    #[test]
    fn static_guards_for_sql() {
        let src = include_str!("user_withdraw_apply.rs");
        assert!(src.contains("INSERT INTO user_withdraw_apply"));
        assert!(src.contains("INSERT INTO user_balance_log"));
        assert!(src.contains("SELECT COUNT(*) FROM user_withdraw_apply WHERE status = 0 AND id IN ("));
        assert!(src.contains(r#"frozen_commission_balance FROM \"user\""#));
    }
}
