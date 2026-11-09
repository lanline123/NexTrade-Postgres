//! 分销客户交易仓储（marketing::salesman_customer_transaction）。
//!
//! 数据源：`salesman_order` JOIN `order`（无独立交易表）：
//! `salesman_order.salesman_id` / `salesman_order.order_id` →
//! `order.order_sn` / `user_id` / `add_time` / `total_amount` / `paid_amount` /
//! `order_status`。
//!
//! 会员名回填：`salesman.user_id`（分销员名）与 `order.user_id`（买家名）
//! 批量查 `user.username`。
//!
//! 查询统一运行时 `sqlx::query_as`；时间过滤口径 `order.add_time`。

use sqlx::{FromRow,PgPool};

use nextrade_core::{Error, Result};

/// 交易聚合行。
#[derive(Debug, Clone, Default, FromRow)]
pub struct CustomerTransactionRow {
    pub salesman_id: i32,
    pub order_id: i32,
    pub order_sn: String,
    pub user_id: i32,
    pub add_time: Option<i32>,
    pub total_amount: Option<rust_decimal::Decimal>,
    pub paid_amount: Option<rust_decimal::Decimal>,
    pub order_status: Option<i32>,
}

/// 客户交易仓储。
#[derive(Clone)]
pub struct SalesmanCustomerTransactionRepository {
    pool: PgPool,
}

impl SalesmanCustomerTransactionRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 分页：`salesman_id` + `order.add_time` 区间（Java Mapper XML 口径）。
    ///
    /// `salesman_id = None` → 不加分销员过滤（Java `CustomerTransactionPageQuery`
    /// 的 `salesmanId` 非必填，为空即查全部成交记录）。
    pub async fn paginate(
        &self,
        salesman_id: Option<i32>,
        start: Option<i64>,
        end: Option<i64>,
        limit: i64,
        offset: i64,
    ) -> Result<(u64, Vec<CustomerTransactionRow>)> {
        let mut where_parts: Vec<String> = Vec::new();
        if salesman_id.is_some() {
            where_parts.push("so.salesman_id = ?".into());
        }
        if start.is_some() {
            where_parts.push("o.add_time >= ?".into());
        }
        if end.is_some() {
            where_parts.push("o.add_time <= ?".into());
        }
        let where_sql = if where_parts.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_parts.join(" AND "))
        };

        let count_sql =
            crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM salesman_order so JOIN \"order\" o ON o.order_id = so.order_id {where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        if let Some(v) = salesman_id {
            cq = cq.bind(v);
        }
        if let Some(v) = start {
            cq = cq.bind(v);
        }
        if let Some(v) = end {
            cq = cq.bind(v);
        }
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT so.salesman_id, o.order_id, o.order_sn, o.user_id, o.add_time, o.total_amount, o.paid_amount, o.order_status FROM salesman_order so JOIN \"order\" o ON o.order_id = so.order_id {where_sql} ORDER BY o.order_id DESC LIMIT ? OFFSET ?"
        ));
        let mut q = sqlx::query_as::<_, CustomerTransactionRow>(&sql);
        if let Some(v) = salesman_id {
            q = q.bind(v);
        }
        if let Some(v) = start {
            q = q.bind(v);
        }
        if let Some(v) = end {
            q = q.bind(v);
        }
        let rows = q
            .bind(limit.max(1) as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 分销员对应的会员名（`salesman.salesman_id → user.username`）。
    pub async fn salesman_names(&self, salesman_ids: &[i32]) -> Result<Vec<(i32, Option<String>)>> {
        let ids: Vec<i32> = salesman_ids.iter().copied().filter(|id| *id > 0).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT s.salesman_id, u.username FROM salesman s LEFT JOIN \"user\" u ON u.user_id = s.user_id WHERE s.salesman_id IN ({ph})"
        ));
        let mut q = sqlx::query_as::<_, (i32, Option<String>)>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 买家会员名（`user.user_id → username`）。
    pub async fn usernames(&self, user_ids: &[i32]) -> Result<Vec<(i32, Option<String>)>> {
        let ids: Vec<i32> = user_ids.iter().copied().filter(|id| *id > 0).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT user_id, username FROM \"user\" WHERE user_id IN ({ph})"));
        let mut q = sqlx::query_as::<_, (i32, Option<String>)>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_defaults_are_safe() {
        let row = CustomerTransactionRow::default();
        assert_eq!(row.salesman_id, 0);
        assert!(row.total_amount.is_none());
        assert!(row.paid_amount.is_none());
    }
}
