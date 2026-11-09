//! 订单支付域仓储（payment::pay）。
//!
//! 只读访问：`config`（支付开关/排序/线下说明）、`order`（订单摘要/拆单子单）、
//! `paylog`（最后一笔成功支付）、`order_presale`、`order_reservations`。
//! 外部支付渠道 SDK 不在仓储层（见服务层「未迁移」）。

use std::collections::HashMap;

use rust_decimal::Decimal;
use sqlx::{FromRow,Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::{OrderPresale, OrderReservations, Paylog};
use nextrade_core::models::payment::pay::PAYLOG_STATUS_PAID;
use nextrade_core::{Error, Result};

/// 订单摘要行（仅本切片需要的列）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct PayOrderRow {
    pub order_id: i32,
    pub order_sn: String,
    pub user_id: i32,
    pub total_amount: Decimal,
    pub paid_amount: Decimal,
    pub unpaid_amount: Decimal,
    pub balance: Decimal,
    pub pay_status: i32,
    pub order_status: i32,
    pub pay_type_id: i32,
    pub is_presale: i32,
    pub presale_type: Option<i32>,
    pub order_type: Option<i32>,
    pub parent_order_id: Option<i32>,
    pub add_time: i32,
    pub shipping_type_name: String,
}

/// 订单支付域仓储。
#[derive(Clone)]
pub struct PayRepository {
    pool: PgPool,
}

impl PayRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 批量读取配置（缺失键不返回）。
    pub async fn config_values(
        &self,
        biz_codes: &[&str],
    ) -> Result<HashMap<String, String>> {
        if biz_codes.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT biz_code, biz_val FROM config WHERE is_del = 0 AND biz_code IN (");
        {
            let mut sep = qb.separated(", ");
            for code in biz_codes {
                sep.push_bind((*code).to_string());
            }
        }
        qb.push(")");
        let rows: Vec<(String, Option<String>)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .filter_map(|(code, val)| val.map(|v| (code, v)))
            .collect())
    }

    /// 按主键查订单摘要。
    pub async fn find_order_by_id(&self, order_id: i32) -> Result<Option<PayOrderRow>> {
        sqlx::query_as::<_, PayOrderRow>(
            "SELECT order_id, order_sn, user_id, total_amount, paid_amount, unpaid_amount, balance, pay_status, order_status, pay_type_id, is_presale, presale_type, order_type, parent_order_id, add_time, shipping_type_name FROM \"order\" WHERE order_id = $1 LIMIT 1",
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 拆单子订单（`parent_order_id = ?` 且属于当前会员）。
    pub async fn find_children_by_parent(
        &self,
        parent_order_id: i32,
        user_id: i32,
    ) -> Result<Vec<PayOrderRow>> {
        sqlx::query_as::<_, PayOrderRow>(
            "SELECT order_id, order_sn, user_id, total_amount, paid_amount, unpaid_amount, balance, pay_status, order_status, pay_type_id, is_presale, presale_type, order_type, parent_order_id, add_time, shipping_type_name FROM \"order\" WHERE parent_order_id = $1 AND user_id = $2",
        )
        .bind(parent_order_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 最后一笔成功支付日志（Java `paylogService.getLastByOrderId(orderId, orderType)`）。
    pub async fn find_last_success_paylog(
        &self,
        order_id: i32,
        order_type: i32,
    ) -> Result<Option<Paylog>> {
        sqlx::query_as::<_, Paylog>(
            "SELECT paylog_id, pay_sn, pay_name, order_id, order_sn, order_amount, order_type, pay_amount, pay_status, pay_code, profit_sharing_channel, add_time, transaction_id, notify_data, refund_amount, token_code FROM paylog WHERE order_id = $1 AND order_type = $2 AND pay_status = $3 ORDER BY paylog_id DESC LIMIT 1",
        )
        .bind(order_id)
        .bind(order_type)
        .bind(PAYLOG_STATUS_PAID)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按主键查支付日志（`checkStatus` 的 paylogId 分支）。
    pub async fn find_paylog_by_id(&self, paylog_id: i32) -> Result<Option<Paylog>> {
        sqlx::query_as::<_, Paylog>(
            "SELECT paylog_id, pay_sn, pay_name, order_id, order_sn, order_amount, order_type, pay_amount, pay_status, pay_code, profit_sharing_channel, add_time, transaction_id, notify_data, refund_amount, token_code FROM paylog WHERE paylog_id = $1 LIMIT 1",
        )
        .bind(paylog_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按订单查预售信息。
    pub async fn find_presale_by_order_id(&self, order_id: i32) -> Result<Option<OrderPresale>> {
        sqlx::query_as::<_, OrderPresale>(
            "SELECT presale_order_id, order_id, order_sn, product_id, sale_mode, deposit_amount, final_payment_amount, deposit_paid_time, final_payment_start_time, final_payment_paid_time, presale_status, delivery_days, delivery_date, create_time, update_time FROM order_presale WHERE order_id = $1 LIMIT 1",
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按订单查预约信息。
    /// 事务内写入预售订单信息（`order_presale`）。
    ///
    /// # Errors
    /// 数据库错误。
    pub async fn insert_presale_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        p: &OrderPresale,
    ) -> Result<i32> {
        let result: i32 = sqlx::query_scalar(
            "INSERT INTO order_presale
             (order_id, order_sn, product_id, sale_mode, deposit_amount, final_payment_amount,
              deposit_paid_time, final_payment_start_time, final_payment_paid_time,
              presale_status, delivery_days, delivery_date, create_time, update_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) RETURNING presale_order_id",
        )
        .bind(p.order_id)
        .bind(&p.order_sn)
        .bind(p.product_id)
        .bind(p.sale_mode)
        .bind(p.deposit_amount)
        .bind(p.final_payment_amount)
        .bind(p.deposit_paid_time)
        .bind(p.final_payment_start_time)
        .bind(p.final_payment_paid_time)
        .bind(p.presale_status)
        .bind(p.delivery_days)
        .bind(p.delivery_date)
        .bind(p.create_time)
        .bind(p.update_time)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(result)
    }

    /// 定金支付完成：更新预售状态为「待付尾款」并记录尾款开始时间。
    ///
    /// # Errors
    /// 数据库错误。
    pub async fn update_presale_deposit_paid(
        &self,
        order_id: i32,
        deposit_paid_time: i32,
        final_payment_start_time: Option<i32>,
    ) -> Result<u64> {
        let affected = sqlx::query(
            "UPDATE order_presale
             SET presale_status = 1, deposit_paid_time = $1, final_payment_start_time = $2,
                 update_time = $3
             WHERE order_id = $4 AND presale_status = 0",
        )
        .bind(deposit_paid_time)
        .bind(final_payment_start_time)
        .bind(deposit_paid_time)
        .bind(order_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected)
    }

    /// 尾款支付完成：更新预售状态为「已完成支付」。
    ///
    /// # Errors
    /// 数据库错误。
    pub async fn update_presale_final_paid(
        &self,
        order_id: i32,
        final_payment_paid_time: i32,
    ) -> Result<u64> {
        let affected = sqlx::query(
            "UPDATE order_presale
             SET presale_status = 2, final_payment_paid_time = $1, update_time = $2
             WHERE order_id = $3 AND presale_status = 1",
        )
        .bind(final_payment_paid_time)
        .bind(final_payment_paid_time)
        .bind(order_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected)
    }

    /// 定金支付后推进订单金额：已付累加、未付改为尾款。
    ///
    /// # Errors
    /// 数据库错误。
    pub async fn settle_presale_deposit(
        &self,
        order_id: i32,
        deposit_amount: Decimal,
    ) -> Result<u64> {
        let affected = sqlx::query(
            "UPDATE \"order\"
             SET paid_amount = paid_amount + $1,
                 unpaid_amount = GREATEST(total_amount - paid_amount, 0),
                 pay_time = $2
             WHERE order_id = $3",
        )
        .bind(deposit_amount)
        .bind(utils_now())
        .bind(order_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected)
    }

    pub async fn find_reservation_by_order_id(
        &self,
        order_id: i32,
    ) -> Result<Option<OrderReservations>> {
        sqlx::query_as::<_, OrderReservations>(
            "SELECT order_reservations_id, order_id, product_reservation_time_id, sku_id, reservation_date, start_time, end_time, time_slot_description, add_time FROM order_reservations WHERE order_id = $1 LIMIT 1",
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }
}

fn utils_now() -> i32 {
    nextrade_common::utils::now_seconds() as i32
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn repository_type_is_send_sync_placeholder() {
        fn assert_send<T: Send>() {}
        assert_send::<super::PayRepository>();
    }
}
