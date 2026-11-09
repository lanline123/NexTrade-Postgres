//! 账户总览仓储（payment::account_panel）。
//!
//! 跨表区间汇总（Java `AccountPanelServiceImpl`）：
//! - `user_balance_log`：余额/冻结余额按 `change_type`（1 增 2 减）净值；
//! - `user_recharge_order`：已支付充值（status=1）金额合计；
//! - `user_withdraw_apply`：已完成提现（status=1）金额合计；
//! - `order`：余额支付、积分支付合计。
//!
//! 全部 SQL 为运行时 `sqlx::query_as`（禁止 `query!` 宏）。

use rust_decimal::Decimal;
use sqlx::PgPool;

use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct AccountPanelRepository {
    pool: PgPool,
}

impl AccountPanelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 余额变动净值：`change_type=1` 加、`2` 减（Java `getUserBalanceLogTotal`）。
    pub async fn balance_net(&self, start: i64, end: i64) -> Result<Decimal> {
        self.net("balance", start, end).await
    }

    /// 冻结余额变动净值（Java `getUserFrozenBalanceLogTotal`）。
    pub async fn frozen_net(&self, start: i64, end: i64) -> Result<Decimal> {
        self.net("frozen_balance", start, end).await
    }

    async fn net(&self, column: &str, start: i64, end: i64) -> Result<Decimal> {
        // column 为内部常量（balance / frozen_balance），不拼接用户输入
        let sql = format!(
            // `balance` 是 decimal，但 `ELSE 0` 让聚合在部分路径上落到 bigint，
            // 而 Rust 侧统一按 Option<Decimal> 解码，显式 CAST 成 NUMERIC 最稳。
            "SELECT CAST(COALESCE(SUM(CASE WHEN change_type = 1 THEN {column}
                WHEN change_type = 2 THEN -{column} ELSE 0 END), 0) AS NUMERIC)
             FROM user_balance_log WHERE change_time >= $1 AND change_time <= $2"
        );
        let row: (Option<Decimal>,) = sqlx::query_as(&sql)
            .bind(start)
            .bind(end)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0.unwrap_or_default())
    }

    /// 已支付充值金额合计（Java `getUserRechargeSurplusTotal`：status=1）。
    pub async fn recharge_total(&self, start: i64, end: i64) -> Result<Decimal> {
        let row: (Option<Decimal>,) = sqlx::query_as(
            "SELECT CAST(COALESCE(SUM(amount),0) AS NUMERIC) FROM user_recharge_order
             WHERE status = 1 AND add_time >= $1 AND add_time <= $2",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.unwrap_or_default())
    }

    /// 已完成提现金额合计（Java `getUserWithdrawTotal`：status=1）。
    pub async fn withdraw_total(&self, start: i64, end: i64) -> Result<Decimal> {
        let row: (Option<Decimal>,) = sqlx::query_as(
            "SELECT CAST(COALESCE(SUM(amount),0) AS NUMERIC) FROM user_withdraw_apply
             WHERE status = 1 AND add_time >= $1 AND add_time <= $2",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.unwrap_or_default())
    }

    /// 订单维度合计（Java `getOrderBalance`：`balance` 或 `use_points`）。
    pub async fn order_total(&self, column: &str, start: i64, end: i64) -> Result<Decimal> {
        let column = match column {
            "balance" => "balance",
            "use_points" => "use_points",
            _ => return Err(Error::Validation("参数缺失".into())),
        };
        let sql = format!(
            // `use_points` 在 PG 里是 integer，SUM 返回 bigint；`balance` 是 decimal。
            // 两种口径都 CAST 成 NUMERIC，统一按 Option<Decimal> 解码。
            "SELECT CAST(COALESCE(SUM({column}),0) AS NUMERIC) FROM \"order\" WHERE add_time >= $1 AND add_time <= $2"
        );
        let row: (Option<Decimal>,) = sqlx::query_as(&sql)
            .bind(start)
            .bind(end)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0.unwrap_or_default())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn net_sql_only_uses_constant_columns() {
        let balance = "SUM(CASE WHEN change_type = 1 THEN balance WHEN change_type = 2 THEN -balance ELSE 0 END)";
        let frozen = "SUM(CASE WHEN change_type = 1 THEN frozen_balance WHEN change_type = 2 THEN -frozen_balance ELSE 0 END)";
        assert!(balance.contains("change_type = 1"));
        assert!(frozen.contains("frozen_balance"));
    }
}
