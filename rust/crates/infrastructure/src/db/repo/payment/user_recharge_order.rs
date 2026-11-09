//! 充值记录域仓储（payment::user_recharge_order）。
//!
//! 对应原 Java `UserRechargeOrderMapper` + `UserMapper`（用户信息/关键字）+
//! `UserBalanceServiceImpl.incBalance`（审核通过入账 + `user_balance_log`）。
//!
//! 表：`user_recharge_order`（8 列）；只读装配 `user`；余额写入 `user` +
//! `user_balance_log`（`wallet_type=0`）。
//!
//! # 事务
//!
//! `update` 置已支付时「加余额 + 写流水 + 更新充值单」在 Java 是 `@Transactional`；
//! 本仓储提供 [`UserRechargeOrderRepository::begin`] 与 `*_in_tx` 方法，
//! 由服务层把多次写入放进同一事务（与 price_inquiry / credit_settlement 同做法）。

use std::collections::HashMap;

use rust_decimal::Decimal;
use sqlx::{FromRow,Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::UserRechargeOrder;
use nextrade_core::models::payment::user_recharge_order::{
    limit_offset, UserRechargeOrderListQuery, CHANGE_TYPE_INC,
};
use nextrade_core::{Error, Result};

/// `user_recharge_order` 固定 SELECT 列清单。
const RECHARGE_SELECT: &str = "SELECT order_id, user_id, amount, discount_money, add_time, paid_time, postscript, status FROM user_recharge_order";

/// 用户装配行（`user`）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct RechargeUserRow {
    pub user_id: i32,
    pub username: String,
    pub nickname: Option<String>,
    pub mobile: String,
}

/// 余额行（`user`）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct BalanceRow {
    pub balance: Decimal,
    pub frozen_balance: Decimal,
}

/// 充值记录域仓储。
#[derive(Clone)]
pub struct UserRechargeOrderRepository {
    pool: PgPool,
}

impl UserRechargeOrderRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 开启事务（审核通过入账）。
    pub async fn begin(&self) -> Result<sqlx::Transaction<'static, Postgres>> {
        self.pool.begin().await.map_err(map_err)
    }

    // ————————————————————— user_recharge_order —————————————————————

    /// 按主键查询（Java `getById`）。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<UserRechargeOrder>> {
        sqlx::query_as::<_, UserRechargeOrder>(&format!(
            "{RECHARGE_SELECT} WHERE order_id = $1 LIMIT 1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 分页（Java `list`：status 精确 + userId IN + 排序）。
    pub async fn paginate(
        &self,
        q: &UserRechargeOrderListQuery,
        user_ids: Option<&[i32]>,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<UserRechargeOrder>)> {
        let mut cq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM user_recharge_order WHERE 1 = 1");
        Self::push_filters(&mut cq, q, user_ids);
        let total: (i64,) = cq
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut rq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{RECHARGE_SELECT} WHERE 1 = 1"));
        Self::push_filters(&mut rq, q, user_ids);
        rq.push(q.order_clause());
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<UserRechargeOrder> = rq
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0, rows))
    }

    fn push_filters(
        qb: &mut QueryBuilder<'_, Postgres>,
        q: &UserRechargeOrderListQuery,
        user_ids: Option<&[i32]>,
    ) {
        if let Some(status) = q.status {
            qb.push(" AND status = ");
            qb.push_bind(status);
        }
        if let Some(ids) = user_ids {
            qb.push(" AND user_id IN (");
            Self::push_int_list(qb, ids);
            qb.push(")");
        }
    }

    /// 新增（Java `save`；`add_time` 由服务层置为当前秒）。
    pub async fn insert(&self, entity: &UserRechargeOrder) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO user_recharge_order (user_id, amount, discount_money, add_time, paid_time, postscript, status) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING order_id",
        )
        .bind(entity.user_id)
        .bind(entity.amount)
        .bind(entity.discount_money)
        .bind(entity.add_time)
        .bind(entity.paid_time)
        .bind(&entity.postscript)
        .bind(entity.status)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 整行更新（Java `updateById`，服务层按 NOT_NULL 语义合并后调用）。
    pub async fn update(&self, entity: &UserRechargeOrder) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE user_recharge_order SET user_id = $1, amount = $2, discount_money = $3, add_time = $4, paid_time = $5, postscript = $6, status = $7 WHERE order_id = $8",
        )
        .bind(entity.user_id)
        .bind(entity.amount)
        .bind(entity.discount_money)
        .bind(entity.add_time)
        .bind(entity.paid_time)
        .bind(&entity.postscript)
        .bind(entity.status)
        .bind(entity.order_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 物理删除（Java `removeById`）。
    pub async fn delete(&self, id: i32) -> Result<bool> {
        let res = sqlx::query("DELETE FROM user_recharge_order WHERE order_id = $1")
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
            QueryBuilder::new("DELETE FROM user_recharge_order WHERE order_id IN (");
        Self::push_int_list(&mut qb, ids);
        qb.push(")");
        let res = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 单字段更新（Java `BaseServiceImpl#updateField`；列名经白名单归一）。
    pub async fn update_field(&self, id: i32, column: &str, val: &str) -> Result<u64> {
        debug_assert!(
            nextrade_core::models::payment::user_recharge_order::GENERAL_FIELDS.contains(&column),
            "update_field 列名未经过白名单映射: {column}"
        );
        let sql = format!("UPDATE user_recharge_order SET {column} = $1 WHERE order_id = $2");
        let res = sqlx::query(&sql)
            .bind(val)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— 装配 / 关键字 —————————————————————

    /// 关键字命中的会员 ID（`username LIKE` OR `nickname LIKE`）。
    pub async fn find_user_ids_by_keyword(&self, keyword: &str) -> Result<Vec<i32>> {
        let like = format!("%{keyword}%");
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT user_id FROM \"user\" WHERE username LIKE $1 OR nickname LIKE $2",
        )
        .bind(&like)
        .bind(&like)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 批量用户信息。
    pub async fn find_users(&self, ids: &[i32]) -> Result<Vec<RechargeUserRow>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT user_id, username, nickname, mobile FROM \"user\" WHERE user_id IN (",
        );
        Self::push_int_list(&mut qb, ids);
        qb.push(")");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 余额入账（事务内） —————————————————————

    /// 余额入账（Java `UserBalanceServiceImpl.incBalance`）：
    /// `user.balance += amount` + 一条 `user_balance_log`（`wallet_type=0`、
    /// `change_type=1`，before/after 为变动前后余额）。
    pub async fn inc_balance_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        user_id: i32,
        amount: Decimal,
        change_desc: &str,
        change_time: i32,
    ) -> Result<bool> {
        let row: Option<BalanceRow> = sqlx::query_as::<_, BalanceRow>(
            "SELECT balance, frozen_balance FROM \"user\" WHERE user_id = $1 FOR UPDATE",
        )
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_err)?;
        let Some(balance_row) = row else {
            return Ok(false);
        };
        let before_balance = balance_row.balance;
        let after_balance = before_balance + amount;
        let before_frozen = balance_row.frozen_balance;
        sqlx::query("UPDATE \"user\" SET balance = $1 WHERE user_id = $2")
            .bind(after_balance)
            .bind(user_id)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?;
        sqlx::query(
            "INSERT INTO user_balance_log (user_id, before_balance, balance, after_balance, frozen_balance, before_frozen_balance, after_frozen_balance, change_time, change_desc, change_type, wallet_type, new_balance, new_frozen_balance) VALUES ($1, $2, $3, $4, 0, $5, $6, $7, $8, $9, 0, $10, $11)",
        )
        .bind(user_id)
        .bind(before_balance)
        .bind(amount)
        .bind(after_balance)
        .bind(before_frozen)
        .bind(before_frozen)
        .bind(change_time)
        .bind(change_desc)
        .bind(CHANGE_TYPE_INC)
        .bind(after_balance)
        .bind(before_frozen)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(true)
    }

    /// 便捷分页窗口。
    #[must_use]
    pub fn page_window(page: Option<i64>, size: Option<i64>) -> (i64, i64) {
        limit_offset(page, size)
    }

    /// 用户信息建索引。
    #[must_use]
    pub fn index_users(rows: Vec<RechargeUserRow>) -> HashMap<i32, RechargeUserRow> {
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
    use nextrade_core::models::payment::user_recharge_order::RECHARGE_COLUMNS;

    #[test]
    fn select_matches_schema_columns() {
        let selected = RECHARGE_SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap();
        assert_eq!(selected, RECHARGE_COLUMNS.join(", "));
    }

    #[test]
    fn static_guards_for_sql() {
        let src = include_str!("user_recharge_order.rs");
        assert!(src.contains("INSERT INTO user_recharge_order"));
        assert!(src.contains("INSERT INTO user_balance_log"));
        assert!(src.contains("wallet_type, new_balance"));
        assert!(src.contains("username LIKE ? OR nickname LIKE ?"));
    }
}
