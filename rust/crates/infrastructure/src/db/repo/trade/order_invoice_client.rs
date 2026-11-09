//! 前台订单发票域仓储（trade::order_invoice_client）。
//!
//! 表：`order_invoice`（21 列）+ `order`（订单快照回填）。
//! 查询统一使用运行时 `sqlx::query_as`（`order` 为保留字，全部加反引号）。

use rust_decimal::Decimal;
use sqlx::{Postgres,PgPool};

use nextrade_core::models::trade::order_invoice_client::INVOICE_STATUS_PENDING;
use nextrade_core::{Error, Result};

/// 发票行（可写字段子集）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct InvoiceRow {
    pub id: i32,
    pub user_id: i32,
    pub order_id: i32,
    pub invoice_type: i32,
    pub status: i32,
    pub apply_reply: String,
    pub title_type: i32,
    pub company_code: String,
    pub company_name: String,
    pub company_address: String,
    pub company_phone: String,
    pub company_bank: String,
    pub company_account: String,
    pub invoice_content: String,
    pub invoice_no: String,
    pub amount: Decimal,
    pub mobile: String,
    pub email: String,
    pub add_time: i32,
    pub invoice_attachment: Option<String>,
    pub invoicing_time: Option<i32>,
}

/// 发票更新补丁（只更新提供字段）。
#[derive(Debug, Clone, Default)]
pub struct InvoiceUpdatePatch {
    pub invoice_type: Option<i32>,
    pub title_type: Option<i32>,
    pub company_code: Option<String>,
    pub company_name: Option<String>,
    pub company_address: Option<String>,
    pub company_phone: Option<String>,
    pub company_bank: Option<String>,
    pub company_account: Option<String>,
    pub invoice_content: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub amount: Option<Decimal>,
}

/// 订单快照（发票详情回填）。
#[derive(Debug, Clone, Default)]
pub struct OrderBrief {
    pub order_sn: String,
    pub total_amount: Decimal,
    pub address: String,
    pub consignee: String,
    pub mobile: String,
}

/// 前台订单发票仓储。
#[derive(Clone)]
pub struct OrderInvoiceClientRepository {
    pool: PgPool,
}

impl OrderInvoiceClientRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 按 id + userId 查发票（Java `clientDetail` 归属校验）。
    ///
    /// 21 列超过 sqlx 元组解码上限（16），故用 `#[derive(sqlx::FromRow)]` 结构体接收。
    pub async fn find_invoice(&self, id: i32, user_id: i32) -> Result<Option<InvoiceRow>> {
        sqlx::query_as::<_, InvoiceRow>(
            "SELECT id, user_id, order_id, invoice_type, status, apply_reply, title_type,
                    company_code, company_name, company_address, company_phone, company_bank,
                    company_account, invoice_content, invoice_no, amount, mobile, email,
                    add_time, NULL AS invoice_attachment, NULL AS invoicing_time
             FROM order_invoice WHERE id = $1 AND user_id = $2 LIMIT 1",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 该订单是否已申请发票（Java 重复申请校验）。
    pub async fn exists_invoice_for_order(&self, user_id: i32, order_id: i32) -> Result<bool> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM order_invoice WHERE user_id = $1 AND order_id = $2",
        )
        .bind(user_id)
        .bind(order_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0 > 0)
    }

    /// 订单快照（归属校验 + 详情回填）。
    pub async fn find_order_brief(&self, order_id: i32, user_id: i32) -> Result<Option<OrderBrief>> {
        let row = sqlx::query_as::<_, (String, Decimal, String, String, String)>(
            "SELECT order_sn, total_amount, address, consignee, mobile
             FROM \"order\" WHERE order_id = $1 AND user_id = $2 LIMIT 1",
        )
        .bind(order_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|r| OrderBrief {
            order_sn: r.0,
            total_amount: r.1,
            address: r.2,
            consignee: r.3,
            mobile: r.4,
        }))
    }

    /// 插入发票申请（status = 0 待审核）。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_invoice(
        &self,
        user_id: i32,
        order_id: i32,
        invoice_type: i32,
        title_type: i32,
        company_name: &str,
        company_code: Option<&str>,
        company_address: Option<&str>,
        company_phone: Option<&str>,
        company_bank: Option<&str>,
        company_account: Option<&str>,
        invoice_content: Option<&str>,
        amount: Decimal,
        mobile: Option<&str>,
        email: Option<&str>,
        add_time: i32,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO order_invoice
               (user_id, order_id, invoice_type, status, apply_reply, title_type, company_code,
                company_name, company_address, company_phone, company_bank, company_account,
                invoice_content, invoice_no, amount, mobile, email, add_time, invoicing_time)
             VALUES ($1, $2, $3, $4, '', $5, $6, $7, $8, $9, $10, $11, $12, '', $13, $14, $15, $16, 0) RETURNING id",
        )
        .bind(user_id)
        .bind(order_id)
        .bind(invoice_type)
        .bind(INVOICE_STATUS_PENDING)
        .bind(title_type)
        .bind(company_code.unwrap_or(""))
        .bind(company_name)
        .bind(company_address.unwrap_or(""))
        .bind(company_phone.unwrap_or(""))
        .bind(company_bank.unwrap_or(""))
        .bind(company_account.unwrap_or(""))
        .bind(invoice_content.unwrap_or(""))
        .bind(amount)
        .bind(mobile.unwrap_or(""))
        .bind(email.unwrap_or(""))
        .bind(add_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 在已有事务内插入发票申请（随单落库使用）。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_invoice_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user_id: i32,
        order_id: i32,
        invoice_type: i32,
        title_type: i32,
        company_name: &str,
        company_code: Option<&str>,
        company_address: Option<&str>,
        company_phone: Option<&str>,
        company_bank: Option<&str>,
        company_account: Option<&str>,
        invoice_content: Option<&str>,
        amount: Decimal,
        mobile: Option<&str>,
        email: Option<&str>,
        add_time: i32,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO order_invoice
               (user_id, order_id, invoice_type, status, apply_reply, title_type, company_code,
                company_name, company_address, company_phone, company_bank, company_account,
                invoice_content, invoice_no, amount, mobile, email, add_time, invoicing_time)
             VALUES ($1, $2, $3, $4, '', $5, $6, $7, $8, $9, $10, $11, $12, '', $13, $14, $15, $16, 0) RETURNING id",
        )
        .bind(user_id)
        .bind(order_id)
        .bind(invoice_type)
        .bind(INVOICE_STATUS_PENDING)
        .bind(title_type)
        .bind(company_code.unwrap_or(""))
        .bind(company_name)
        .bind(company_address.unwrap_or(""))
        .bind(company_phone.unwrap_or(""))
        .bind(company_bank.unwrap_or(""))
        .bind(company_account.unwrap_or(""))
        .bind(invoice_content.unwrap_or(""))
        .bind(amount)
        .bind(mobile.unwrap_or(""))
        .bind(email.unwrap_or(""))
        .bind(add_time)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 更新发票申请字段（每字段可选）。
    pub async fn update_invoice(
        &self,
        id: i32,
        patch: &InvoiceUpdatePatch,
    ) -> Result<u64> {
        let mut qb: sqlx::QueryBuilder<'_, Postgres> =
            sqlx::QueryBuilder::new("UPDATE order_invoice SET invoice_type = invoice_type");
        if let Some(v) = patch.invoice_type {
            qb.push(", invoice_type = ").push_bind(v);
        }
        if let Some(v) = patch.title_type {
            qb.push(", title_type = ").push_bind(v);
        }
        if let Some(v) = &patch.company_code {
            qb.push(", company_code = ").push_bind(v.clone());
        }
        if let Some(v) = &patch.company_name {
            qb.push(", company_name = ").push_bind(v.clone());
        }
        if let Some(v) = &patch.company_address {
            qb.push(", company_address = ").push_bind(v.clone());
        }
        if let Some(v) = &patch.company_phone {
            qb.push(", company_phone = ").push_bind(v.clone());
        }
        if let Some(v) = &patch.company_bank {
            qb.push(", company_bank = ").push_bind(v.clone());
        }
        if let Some(v) = &patch.company_account {
            qb.push(", company_account = ").push_bind(v.clone());
        }
        if let Some(v) = &patch.invoice_content {
            qb.push(", invoice_content = ").push_bind(v.clone());
        }
        if let Some(v) = &patch.mobile {
            qb.push(", mobile = ").push_bind(v.clone());
        }
        if let Some(v) = &patch.email {
            qb.push(", email = ").push_bind(v.clone());
        }
        if let Some(v) = patch.amount {
            qb.push(", amount = ").push_bind(v);
        }
        qb.push(" WHERE id = ").push_bind(id);
        let affected = qb
            .build()
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_status_constant() {
        assert_eq!(INVOICE_STATUS_PENDING, 0);
    }

    #[test]
    fn repository_type_exists() {
        assert!(std::any::type_name::<OrderInvoiceClientRepository>().contains("OrderInvoiceClient"));
    }
}


