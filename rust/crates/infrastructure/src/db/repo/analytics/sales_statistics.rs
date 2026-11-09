//! 销售统计 域仓储（analytics::sales_statistics）—— wave9。
//!
//! 聚合源表：`order`（order_status/pay_status/total_amount/add_time/shop_id）与
//! `order_item`（product_id/product_name/product_sn/quantity/price/shop_id）。
//! 运行时 sqlx；金额 Decimal。

use rust_decimal::Decimal;
use sqlx::PgPool;
use sqlx::FromRow;

use nextrade_core::{Error, Result};

/// 区间总计（订单数 / 销售额）。
#[derive(Debug, Clone, Copy)]
pub struct SalesTotals {
    /// 订单数
    pub order_count: i64,
    /// 销售额
    pub total_amount: Decimal,
}

/// 按天聚合行。
#[derive(Debug, Clone, FromRow)]
pub struct DailySalesSqlRow {
    /// 日期
    pub date: String,
    /// 订单数
    pub order_count: i64,
    /// 销售额
    pub total_amount: Option<Decimal>,
}

/// 商品聚合行。
#[derive(Debug, Clone, FromRow)]
pub struct ProductSalesSqlRow {
    /// 商品 ID
    pub product_id: i32,
    /// 商品名称
    pub product_name: String,
    /// 商品货号
    pub product_sn: String,
    /// 销量
    pub quantity: Option<i64>,
    /// 销售额
    pub amount: Option<Decimal>,
}

/// 销售统计仓储。
#[derive(Clone)]
pub struct SalesStatisticsRepository {
    pool: PgPool,
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

impl SalesStatisticsRepository {
    /// 构造仓储。
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 区间总计（已支付订单）。
    pub async fn totals(
        &self,
        start: i64,
        end: i64,
        shop_id: Option<i32>,
    ) -> Result<SalesTotals> {
        let mut sql = String::from(
            "SELECT COUNT(*), COALESCE(SUM(total_amount), 0) FROM \"order\" \
             WHERE is_del = 0 AND pay_status > 0 AND add_time >= $1 AND add_time <= $2",
        );
        if shop_id.filter(|v| *v > 0).is_some() {
            sql.push_str(" AND shop_id = $3");
        }
        let mut q = sqlx::query_as::<_, (i64, Option<Decimal>)>(&sql)
            .bind(start)
            .bind(end);
        if let Some(sid) = shop_id.filter(|v| *v > 0) {
            q = q.bind(sid);
        }
        let (order_count, total_amount) = q.fetch_one(&self.pool).await.map_err(map_err)?;
        Ok(SalesTotals {
            order_count,
            total_amount: total_amount.unwrap_or_default(),
        })
    }

    /// 按天聚合（\"dateType\" 未参与 SQL；固定按天）。
    pub async fn daily(
        &self,
        start: i64,
        end: i64,
        shop_id: Option<i32>,
    ) -> Result<Vec<DailySalesSqlRow>> {
        let mut sql = String::from(
            "SELECT TO_CHAR(TO_TIMESTAMP(add_time), 'YYYY-MM-DD') AS date, COUNT(*) AS order_count, \
             COALESCE(SUM(total_amount), 0) AS total_amount FROM \"order\" \
             WHERE is_del = 0 AND pay_status > 0 AND add_time >= $1 AND add_time <= $2",
        );
        if shop_id.filter(|v| *v > 0).is_some() {
            sql.push_str(" AND shop_id = $3");
        }
        sql.push_str(" GROUP BY date ORDER BY date ASC");
        let mut q = sqlx::query_as::<_, DailySalesSqlRow>(&sql)
            .bind(start)
            .bind(end);
        if let Some(sid) = shop_id.filter(|v| *v > 0) {
            q = q.bind(sid);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 商品维度聚合分页（销量/销售额）。
    pub async fn product_page(
        &self,
        start: i64,
        end: i64,
        shop_id: Option<i32>,
        order_by_amount: bool,
        ascending: bool,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<ProductSalesSqlRow>)> {
        let mut where_sql = String::from(
            " FROM order_item i JOIN \"order\" o ON o.order_id = i.order_id \
             WHERE o.is_del = 0 AND o.pay_status > 0 AND o.add_time >= $1 AND o.add_time <= $2",
        );
        if shop_id.filter(|v| *v > 0).is_some() {
            where_sql.push_str(" AND o.shop_id = ?");
        }
        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(DISTINCT i.product_id){where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql).bind(start).bind(end);
        if let Some(sid) = shop_id.filter(|v| *v > 0) {
            cq = cq.bind(sid);
        }
        let total = cq.fetch_one(&self.pool).await.map_err(map_err)?.0;
        let col = if order_by_amount { "amount" } else { "quantity" };
        let dir = if ascending { "ASC" } else { "DESC" };
        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT i.product_id, MAX(i.product_name) AS product_name, MAX(i.product_sn) AS product_sn, \
             CAST(SUM(i.quantity) AS BIGINT) AS quantity, SUM(i.price * i.quantity) AS amount{where_sql} \
             GROUP BY i.product_id ORDER BY {col} {dir} LIMIT ? OFFSET ?"
        ));
        let mut rq = sqlx::query_as::<_, ProductSalesSqlRow>(&rows_sql)
            .bind(start)
            .bind(end);
        if let Some(sid) = shop_id.filter(|v| *v > 0) {
            rq = rq.bind(sid);
        }
        let rows = rq
            .bind(i64::try_from(limit).unwrap_or(20))
            .bind(i64::try_from(offset).unwrap_or(0))
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total).unwrap_or(0), rows))
    }
}
