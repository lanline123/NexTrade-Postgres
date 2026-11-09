//! Apple Pay 前台仓储（payment::apple_pay）。
//!
//! 仅用于本地归属校验：`paylog`（按 pay_sn 取最近一条）/ `user_recharge_order`
//! （order_type=1 充值单归属）/ `order`（订单归属）。
//!
//! 全部 SQL 为运行时 `sqlx::query_as` / `sqlx::query`（禁止 `query!` 宏）。

use sqlx::PgPool;

use nextrade_core::entities::finance::Paylog;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct ApplePayRepository {
    pool: PgPool,
}

impl ApplePayRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 按 pay_sn 取最近一条支付记录（Java `PaylogService#getLastByPaySn`）。
    pub async fn find_paylog_by_pay_sn(&self, pay_sn: &str) -> Result<Option<Paylog>> {
        sqlx::query_as::<_, Paylog>(
            "SELECT * FROM paylog WHERE pay_sn = $1 ORDER BY paylog_id DESC LIMIT 1",
        )
        .bind(pay_sn)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 充值单归属用户（`paylog.order_type = 1` 时 `order_id` 指向 user_recharge_order）。
    pub async fn recharge_owner(&self, order_id: i32) -> Result<Option<i32>> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT user_id FROM user_recharge_order WHERE order_id = $1 LIMIT 1")
                .bind(order_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|r| r.0))
    }

    /// 订单归属用户（`paylog.order_type = 0` 时 order_id 指向 `order` 表）。
    pub async fn order_owner(&self, order_id: i32) -> Result<Option<i32>> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT user_id FROM \"order\" WHERE order_id = $1 LIMIT 1")
                .bind(order_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|r| r.0))
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("找不到原支付记录".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn queries_are_parameterized() {
        let sql = "SELECT * FROM paylog WHERE pay_sn = $1 ORDER BY paylog_id DESC LIMIT 1";
        assert!(sql.contains("pay_sn = ?"));
        assert!(!sql.contains("pay_sn = '"));
    }
}
