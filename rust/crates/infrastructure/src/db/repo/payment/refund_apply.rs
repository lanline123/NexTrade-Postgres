//! 退款申请域仓储：`refund_apply` / `refund_log` /
//! `refund_apply_paylog_refund`（关联表）。
//! 对应原 Java `RefundApplyMapper` / `RefundLogMapper`。
//!
//! 列名严格对齐 `java/database/pure.sql`：
//! - `refund_apply`：`refund_id` / `refund_type` / `order_id` / `user_id` /
//!   `aftersale_id` / `refund_status` / `add_time` / `refund_note` /
//!   `online_balance` / `offline_balance` / `refund_balance` /
//!   `is_online` / `is_offline` / `is_receive` / `paylog_refund_id` /
//!   `shop_id` / `payment_voucher`
//! - `refund_log`：`log_id` / `refund_apply_id` / `refund_type` /
//!   `refund_pay_code` / `transaction_id` / `refund_amount` / `add_time` /
//!   `description` / `user_id`
//! - `refund_apply_paylog_refund`：`refund_apply_paylog_refund_id` /
//!   `refund_apply_id` / `paylog_refund_id`

use rust_decimal::Decimal;
use sqlx::PgPool;
use nextrade_core::entities::{RefundApply, RefundLog};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct RefundApplyRepository {
    pool: PgPool,
}

impl RefundApplyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 退款申请 —————————————————————

    /// 按主键查询。
    /// 退款申请归属店铺（`refund_apply.shop_id`）。
    pub async fn find_shop_id(&self, refund_id: i32) -> Result<Option<i32>> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT shop_id FROM refund_apply WHERE refund_id = $1 LIMIT 1")
                .bind(refund_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|(v,)| v))
    }

    pub async fn find_by_id(&self, refund_id: i32) -> Result<Option<RefundApply>> {
        sqlx::query_as::<_, RefundApply>("SELECT * FROM refund_apply WHERE refund_id = $1 LIMIT 1")
            .bind(refund_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按关联的退款流水 ID 反查申请（`getByPaylogRefundId`）。
    pub async fn find_by_paylog_refund_id(
        &self,
        paylog_refund_id: i32,
    ) -> Result<Option<RefundApply>> {
        sqlx::query_as::<_, RefundApply>(
            "SELECT * FROM refund_apply WHERE paylog_refund_id = $1 LIMIT 1",
        )
        .bind(paylog_refund_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按售后单查询申请。
    pub async fn find_by_aftersale(&self, aftersale_id: i32) -> Result<Option<RefundApply>> {
        sqlx::query_as::<_, RefundApply>(
            "SELECT * FROM refund_apply WHERE aftersale_id = $1 LIMIT 1",
        )
        .bind(aftersale_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 退款申请分页（管理端）。
    pub async fn paginate(
        &self,
        order_sn: Option<&str>,
        refund_status: Option<i32>,
        shop_id: Option<i32>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<RefundApply>)> {
        let like = order_sn
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{s}%"));
        let (limit, offset) = page.limit_offset();

        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM refund_apply
             WHERE ($1 IS NULL OR order_id IN (SELECT order_id FROM \"order\" WHERE order_sn LIKE $2))
               AND ($3 IS NULL OR refund_status = $4)
               AND ($5 IS NULL OR shop_id = $6)",
        )
        .bind(&like)
        .bind(&like)
        .bind(refund_status)
        .bind(refund_status)
        .bind(shop_id)
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        let rows = sqlx::query_as::<_, RefundApply>(
            "SELECT * FROM refund_apply
             WHERE ($1 IS NULL OR order_id IN (SELECT order_id FROM \"order\" WHERE order_sn LIKE $2))
               AND ($3 IS NULL OR refund_status = $4)
               AND ($5 IS NULL OR shop_id = $6)
             ORDER BY refund_id DESC
             LIMIT $7 OFFSET $8",
        )
        .bind(&like)
        .bind(&like)
        .bind(refund_status)
        .bind(refund_status)
        .bind(shop_id)
        .bind(shop_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 新增退款申请，返回 `refund_id`。
    pub async fn insert(&self, r: &RefundApply) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO refund_apply
             (refund_type, order_id, user_id, aftersale_id, refund_status,
              add_time, refund_note, online_balance, offline_balance,
              refund_balance, is_online, is_offline, is_receive,
              paylog_refund_id, shop_id, payment_voucher)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16) RETURNING refund_id",
        )
        .bind(r.refund_type)
        .bind(r.order_id)
        .bind(r.user_id)
        .bind(r.aftersale_id)
        .bind(r.refund_status)
        .bind(r.add_time)
        .bind(&r.refund_note)
        .bind(r.online_balance)
        .bind(r.offline_balance)
        .bind(r.refund_balance)
        .bind(r.is_online)
        .bind(r.is_offline)
        .bind(r.is_receive)
        .bind(r.paylog_refund_id)
        .bind(r.shop_id)
        .bind(&r.payment_voucher)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 原子状态流转：仅当当前状态在允许集合内时更新为目标状态。
    ///
    /// 对应 Java `audit` 里 `.eq("refund_status", REFUND_STATUS_WAIT).set(...)` 的
    /// 条件更新语义，防止并发重复处理。
    pub async fn transition_status(
        &self,
        refund_id: i32,
        from_statuses: &[i32],
        to_status: i32,
        note: Option<&str>,
    ) -> Result<bool> {
        if from_statuses.is_empty() {
            return Err(Error::Validation("缺少源状态".into()));
        }
        let placeholders = vec!["?"; from_statuses.len()].join(", ");
        // 仅在显式传入 note 时覆盖，避免误清空原备注
        let note_clause = if note.is_some() {
            ", refund_note = ?"
        } else {
            ""
        };
        let sql = crate::db::to_pg_placeholders(&format!(
            "UPDATE refund_apply SET refund_status = ?{note_clause}
             WHERE refund_id = ? AND refund_status IN ({placeholders})"
        ));
        let mut q = sqlx::query(&sql).bind(to_status);
        if let Some(n) = note {
            q = q.bind(n.to_string());
        }
        q = q.bind(refund_id);
        for s in from_statuses {
            q = q.bind(*s);
        }
        let affected = q.execute(&self.pool).await.map_err(map_err)?.rows_affected();
        Ok(affected > 0)
    }

    /// 更新退款金额与线上/线下标记（管理端修正待处理申请）。
    pub async fn update_amounts(&self, r: &RefundApply) -> Result<()> {
        let affected = sqlx::query(
            "UPDATE refund_apply SET
               online_balance = $1, offline_balance = $2, refund_balance = $3,
               is_online = $4, is_offline = $5, refund_note = $6
             WHERE refund_id = $7",
        )
        .bind(r.online_balance)
        .bind(r.offline_balance)
        .bind(r.refund_balance)
        .bind(r.is_online)
        .bind(r.is_offline)
        .bind(&r.refund_note)
        .bind(r.refund_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        if affected == 0 {
            return Err(Error::NotFound("退款申请不存在".into()));
        }
        Ok(())
    }

    /// 确认收货（线下退款流程）。
    pub async fn confirm_receive(&self, refund_id: i32) -> Result<bool> {        let affected =
            sqlx::query("UPDATE refund_apply SET is_receive = 1 WHERE refund_id = $1 AND is_receive = 0")
                .bind(refund_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?
                .rows_affected();
        Ok(affected > 0)
    }

    /// 更新支付凭证（线下转账凭据）。
    pub async fn update_voucher(&self, refund_id: i32, voucher: &str) -> Result<()> {
        sqlx::query("UPDATE refund_apply SET payment_voucher = $1 WHERE refund_id = $2")
            .bind(voucher)
            .bind(refund_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    // ————————————————————— 退款日志 —————————————————————

    /// 新增退款操作日志，返回 `log_id`。
    pub async fn insert_log(&self, l: &RefundLog) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO refund_log
             (refund_apply_id, refund_type, refund_pay_code, transaction_id,
              refund_amount, add_time, description, user_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING log_id",
        )
        .bind(l.refund_apply_id)
        .bind(l.refund_type)
        .bind(&l.refund_pay_code)
        .bind(&l.transaction_id)
        .bind(l.refund_amount)
        .bind(l.add_time)
        .bind(&l.description)
        .bind(l.user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 查询某申请的操作日志（按时间降序）。
    pub async fn find_logs(&self, refund_apply_id: i32) -> Result<Vec<RefundLog>> {
        sqlx::query_as::<_, RefundLog>(
            "SELECT * FROM refund_log WHERE refund_apply_id = $1 ORDER BY log_id DESC",
        )
        .bind(refund_apply_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— 关联表 —————————————————————

    /// 绑定退款申请与退款流水的关联关系（一申请可对应多笔流水）。
    pub async fn link_paylog_refund(
        &self,
        refund_apply_id: i32,
        paylog_refund_id: i32,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO refund_apply_paylog_refund (refund_apply_id, paylog_refund_id)
             VALUES ($1, $2) RETURNING refund_apply_paylog_refund_id",
        )
        .bind(refund_apply_id)
        .bind(paylog_refund_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 查询某申请关联的全部退款流水 ID。
    pub async fn linked_paylog_refund_ids(
        &self,
        refund_apply_id: i32,
    ) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT paylog_refund_id FROM refund_apply_paylog_refund
             WHERE refund_apply_id = $1",
        )
        .bind(refund_apply_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 统计某订单已「占用」的退款申请金额（不含已取消/已拒绝）。
    ///
    /// 对应 Java `sumOccupiedRefundAmount` 的口径：
    /// 只有待处理/处理中/已处理状态的申请会计入已占用额度，
    /// 否则一条被拒绝的申请会永久占用可退额度。
    pub async fn sum_occupied_by_order(&self, order_id: i32) -> Result<Decimal> {
        let row: (Option<Decimal>,) = sqlx::query_as(
            "SELECT SUM(refund_balance) FROM refund_apply
             WHERE order_id = $1
               AND refund_status IN (0, 1, 2)",
        )
        .bind(order_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.unwrap_or(Decimal::ZERO))
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("退款申请不存在".into()),
        other => Error::Database(other.to_string()),
    }
}