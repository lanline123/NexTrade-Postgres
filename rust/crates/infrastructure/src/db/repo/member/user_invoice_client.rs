//! 用户增票资质仓储（member::user_invoice_client）。表 `user_invoice`。

use sqlx::PgPool;
use nextrade_core::{Error, Result};

/// 行类型（15 列子集，按 VO 顺序）。
pub type InvoiceRow = (
    i32, i32, i32, String, i32, String, String, String, String, String, String, String, String, String, i32,
);

#[derive(Clone)]
pub struct UserInvoiceClientRepository {
    pool: PgPool,
}

impl UserInvoiceClientRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    const COLUMNS: &'static str = "invoice_id, user_id, status, apply_reply, title_type, company_name, company_code, company_address, company_phone, company_bank, company_account, mobile, email, invoice_content, add_time";

    /// 最新一条资质（getStatus/detail 共用）。
    pub async fn latest(&self, user_id: i32) -> Result<Option<InvoiceRow>> {
        let sql = format!(
            "SELECT {} FROM user_invoice WHERE user_id = $1 ORDER BY invoice_id DESC LIMIT 1",
            Self::COLUMNS
        );
        sqlx::query_as::<_, InvoiceRow>(&sql)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按主键 + 用户查询（update 归属校验）。
    pub async fn find_own(&self, invoice_id: i32, user_id: i32) -> Result<Option<InvoiceRow>> {
        let sql = format!(
            "SELECT {} FROM user_invoice WHERE invoice_id = $1 AND user_id = $2 LIMIT 1",
            Self::COLUMNS
        );
        sqlx::query_as::<_, InvoiceRow>(&sql)
            .bind(invoice_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 新建资质（status=0）。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert(
        &self,
        user_id: i32,
        title_type: i32,
        company_name: &str,
        company_code: &str,
        company_address: &str,
        company_phone: &str,
        company_bank: &str,
        company_account: &str,
        mobile: &str,
        email: &str,
        invoice_content: &str,
        now: i32,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO user_invoice (user_id, status, apply_reply, add_time, title_type, company_name, company_code, company_address, company_phone, company_bank, company_account, mobile, email, invoice_content) VALUES ($1, 0, '', $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING invoice_id",
        )
        .bind(user_id)
        .bind(now)
        .bind(title_type)
        .bind(company_name)
        .bind(company_code)
        .bind(company_address)
        .bind(company_phone)
        .bind(company_bank)
        .bind(company_account)
        .bind(mobile)
        .bind(email)
        .bind(invoice_content)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 更新资质（重置为待审核）。
    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        &self,
        invoice_id: i32,
        user_id: i32,
        title_type: i32,
        company_name: &str,
        company_code: &str,
        company_address: &str,
        company_phone: &str,
        company_bank: &str,
        company_account: &str,
        mobile: &str,
        email: &str,
        invoice_content: &str,
    ) -> Result<bool> {
        let r = sqlx::query(
            "UPDATE user_invoice SET status = 0, title_type = $1, company_name = $2, company_code = $3, company_address = $4, company_phone = $5, company_bank = $6, company_account = $7, mobile = $8, email = $9, invoice_content = $10 WHERE invoice_id = $11 AND user_id = $12",
        )
        .bind(title_type)
        .bind(company_name)
        .bind(company_code)
        .bind(company_address)
        .bind(company_phone)
        .bind(company_bank)
        .bind(company_account)
        .bind(mobile)
        .bind(email)
        .bind(invoice_content)
        .bind(invoice_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}
