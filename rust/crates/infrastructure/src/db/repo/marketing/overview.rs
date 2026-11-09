//! 分销概览域仓储（marketing::overview）。
//! 表：`salesman` / `salesman_order` / `order`。

use rust_decimal::Decimal;
use sqlx::{Postgres,PgPool, QueryBuilder};

use nextrade_core::{Error, Result};

/// 分销概览域仓储。
#[derive(Clone)]
pub struct OverviewRepository {
    pool: PgPool,
}

impl OverviewRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 店铺范围内的分销员 ID（shopId>0 时过滤 `shop_id`）。
    pub async fn scoped_salesman_ids(&self, shop_id: Option<i32>) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = match shop_id.filter(|id| *id > 0) {
            Some(shop_id) => {
                sqlx::query_as("SELECT salesman_id FROM salesman WHERE shop_id = $1")
                    .bind(shop_id)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(map_err)?
            }
            None => sqlx::query_as("SELECT salesman_id FROM salesman")
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?,
        };
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 新增分销员数（today 时限定 add_time 范围）。
    pub async fn count_salesmen(
        &self,
        shop_id: Option<i32>,
        today_range: Option<(i64, i64)>,
    ) -> Result<i64> {
        let mut qb = String::from("SELECT COUNT(*) FROM salesman WHERE 1 = 1");
        let mut binds: Vec<i32> = Vec::new();
        if let Some(shop_id) = shop_id.filter(|id| *id > 0) {
            qb.push_str(" AND shop_id = ?");
            binds.push(shop_id);
        }
        if let Some((start, end)) = today_range {
            qb.push_str(" AND add_time >= ? AND add_time <= ?");
            binds.push(i32::try_from(start).unwrap_or_default());
            binds.push(i32::try_from(end).unwrap_or_default());
        }
        let qb = crate::db::to_pg_placeholders(&qb);
        let mut query = sqlx::query_as::<_, (i64,)>(&qb);
        for bind in binds {
            query = query.bind(bind);
        }
        let row = query.fetch_one(&self.pool).await.map_err(map_err)?;
        Ok(row.0)
    }

    /// 累计分销额（`SUM(salesman.sale_amount)`）。
    pub async fn sum_sale_amount(&self, shop_id: Option<i32>) -> Result<Decimal> {
        let row: (Option<Decimal>,) = match shop_id.filter(|id| *id > 0) {
            Some(shop_id) => {
                sqlx::query_as("SELECT SUM(sale_amount) FROM salesman WHERE shop_id = $1")
                    .bind(shop_id)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(map_err)?
            }
            None => sqlx::query_as("SELECT SUM(sale_amount) FROM salesman")
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?,
        };
        Ok(row.0.unwrap_or_default())
    }

    /// 趋势原始行：`(add_time, order_amount)`。
    pub async fn trend_rows(
        &self,
        salesman_ids: &[i32],
        start: i64,
        end: i64,
    ) -> Result<Vec<(i32, Decimal)>> {
        if salesman_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT add_time, order_amount FROM salesman_order WHERE add_time >= ",
        );
        qb.push_bind(i32::try_from(start).unwrap_or_default());
        qb.push(" AND add_time <= ");
        qb.push_bind(i32::try_from(end).unwrap_or_default());
        qb.push(" AND salesman_id IN (");
        {
            let mut sep = qb.separated(", ");
            for id in salesman_ids {
                sep.push_bind(*id);
            }
        }
        qb.push(")");
        let rows: Vec<(Option<i32>, Option<Decimal>)> =
            qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(time, amount)| (time.unwrap_or_default(), amount.unwrap_or_default()))
            .collect())
    }

    /// 今日销售额（`SUM(salesman_order.order_amount)`）。
    pub async fn sum_order_amount(
        &self,
        salesman_ids: &[i32],
        start: i64,
        end: i64,
    ) -> Result<Decimal> {
        if salesman_ids.is_empty() {
            return Ok(Decimal::ZERO);
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT SUM(order_amount) FROM salesman_order WHERE add_time >= ",
        );
        qb.push_bind(i32::try_from(start).unwrap_or_default());
        qb.push(" AND add_time <= ");
        qb.push_bind(i32::try_from(end).unwrap_or_default());
        qb.push(" AND salesman_id IN (");
        {
            let mut sep = qb.separated(", ");
            for id in salesman_ids {
                sep.push_bind(*id);
            }
        }
        qb.push(")");
        let row: (Option<Decimal>,) = qb.build_query_as().fetch_one(&self.pool).await.map_err(map_err)?;
        Ok(row.0.unwrap_or_default())
    }

    /// 客户数（`COUNT(DISTINCT order.user_id)`）。
    pub async fn count_customers(
        &self,
        salesman_ids: &[i32],
        start: Option<i64>,
        end: Option<i64>,
    ) -> Result<i64> {
        if salesman_ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT COUNT(DISTINCT o.user_id) FROM salesman_order so LEFT JOIN \"order\" o ON o.order_id = so.order_id WHERE so.salesman_id IN (",
        );
        {
            let mut sep = qb.separated(", ");
            for id in salesman_ids {
                sep.push_bind(*id);
            }
        }
        qb.push(")");
        if let (Some(start), Some(end)) = (start, end) {
            qb.push(" AND so.add_time >= ");
            qb.push_bind(i32::try_from(start).unwrap_or_default());
            qb.push(" AND so.add_time <= ");
            qb.push_bind(i32::try_from(end).unwrap_or_default());
        }
        let row: (i64,) = qb.build_query_as().fetch_one(&self.pool).await.map_err(map_err)?;
        Ok(row.0)
    }

    /// 佣金（`SUM(salesman_order.amount)`，`status=1`；today 时限定 settlement_time）。
    pub async fn sum_commission(
        &self,
        salesman_ids: &[i32],
        start: Option<i64>,
        end: Option<i64>,
    ) -> Result<Decimal> {
        if salesman_ids.is_empty() {
            return Ok(Decimal::ZERO);
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT SUM(amount) FROM salesman_order WHERE status = 1 AND salesman_id IN (",
        );
        {
            let mut sep = qb.separated(", ");
            for id in salesman_ids {
                sep.push_bind(*id);
            }
        }
        qb.push(")");
        if let (Some(start), Some(end)) = (start, end) {
            qb.push(" AND settlement_time >= ");
            qb.push_bind(i32::try_from(start).unwrap_or_default());
            qb.push(" AND settlement_time <= ");
            qb.push_bind(i32::try_from(end).unwrap_or_default());
        }
        let row: (Option<Decimal>,) = qb.build_query_as().fetch_one(&self.pool).await.map_err(map_err)?;
        Ok(row.0.unwrap_or_default())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn repository_is_send_sync() {
        fn assert_send<T: Send>() {}
        assert_send::<super::OverviewRepository>();
    }
}
