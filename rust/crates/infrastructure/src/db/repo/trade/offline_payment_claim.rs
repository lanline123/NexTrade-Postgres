//! `offline_payment_claim` 仓储（v5.8.61 线下认款）。

use sqlx::PgPool;
use nextrade_core::entities::OfflinePaymentClaim;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct OfflinePaymentClaimRepository {
    pool: PgPool,
}

impl OfflinePaymentClaimRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, c: &OfflinePaymentClaim) -> Result<i32> {
        let r: i64 = sqlx::query_scalar(
            "INSERT INTO offline_payment_claim(order_id,order_sn,user_id,shop_id,claimed_amount,pay_stage,\
             transfer_time,payer_name,payer_account,bank_serial_no,voucher_urls,remark,status,\
             review_admin_id,review_time,paid_time,audit_remark,reject_reason,add_time,update_time) \
             VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20) RETURNING claim_id",
        )
        .bind(c.order_id)
        .bind(&c.order_sn)
        .bind(c.user_id)
        .bind(c.shop_id)
        .bind(c.claimed_amount)
        .bind(c.pay_stage)
        .bind(c.transfer_time)
        .bind(&c.payer_name)
        .bind(&c.payer_account)
        .bind(&c.bank_serial_no)
        .bind(&c.voucher_urls)
        .bind(&c.remark)
        .bind(c.status)
        .bind(c.review_admin_id)
        .bind(c.review_time)
        .bind(c.paid_time)
        .bind(&c.audit_remark)
        .bind(&c.reject_reason)
        .bind(c.add_time)
        .bind(c.update_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(i32::try_from(r)
            .map_err(|_| Error::Database(format!("offline_payment_claim.claim_id 超出 i32 范围: {r}")))?)
    }

    pub async fn find_by_id(&self, claim_id: i32) -> Result<Option<OfflinePaymentClaim>> {
        sqlx::query_as::<_, OfflinePaymentClaim>(
            "SELECT * FROM offline_payment_claim WHERE claim_id = $1 LIMIT 1",
        )
        .bind(claim_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn latest_for_order(&self, order_id: i32) -> Result<Option<OfflinePaymentClaim>> {
        sqlx::query_as::<_, OfflinePaymentClaim>(
            "SELECT * FROM offline_payment_claim WHERE order_id = $1 ORDER BY claim_id DESC LIMIT 1",
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn history_for_order(
        &self,
        order_id: i32,
        user_id: Option<i32>,
    ) -> Result<Vec<OfflinePaymentClaim>> {
        if let Some(user_id) = user_id {
            sqlx::query_as::<_, OfflinePaymentClaim>(
                "SELECT * FROM offline_payment_claim WHERE order_id = $1 AND user_id = $2 ORDER BY claim_id DESC",
            )
            .bind(order_id)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
        } else {
            sqlx::query_as::<_, OfflinePaymentClaim>(
                "SELECT * FROM offline_payment_claim WHERE order_id = $1 ORDER BY claim_id DESC",
            )
            .bind(order_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
        }
    }

    pub async fn audit(
        &self,
        claim_id: i32,
        pass: bool,
        admin_id: i32,
        audit_remark: &str,
        reject_reason: &str,
        now: i64,
    ) -> Result<bool> {
        let affected = if pass {
            sqlx::query(
                "UPDATE offline_payment_claim SET status=1, review_admin_id=$1, review_time=$2, \
                 audit_remark=$3, update_time=$4 WHERE claim_id=$5 AND status=0",
            )
            .bind(admin_id)
            .bind(now)
            .bind(audit_remark)
            .bind(now)
            .bind(claim_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected()
        } else {
            sqlx::query(
                "UPDATE offline_payment_claim SET status=2, review_admin_id=$1, review_time=$2, \
                 audit_remark=$3, reject_reason=$4, update_time=$5 WHERE claim_id=$6 AND status=0",
            )
            .bind(admin_id)
            .bind(now)
            .bind(audit_remark)
            .bind(reject_reason)
            .bind(now)
            .bind(claim_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected()
        };
        Ok(affected == 1)
    }

    pub async fn mark_paid(&self, claim_id: i32, now: i64) -> Result<()> {
        sqlx::query("UPDATE offline_payment_claim SET paid_time=$1, update_time=$2 WHERE claim_id=$3")
            .bind(now)
            .bind(now)
            .bind(claim_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

fn map_err(error: sqlx::Error) -> Error {
    Error::Database(error.to_string())
}
