//! 订单结算域仓储（trade::check）。
//!
//! 涉及表（列名严格对齐 `schema_parser`）：
//! - `order_invoice`（订单发票，`getInvoice` 最近一条）
//! - `config`（`useOffline` / `useSurplus` 支付开关）
//!
//! 查询统一使用运行时 `sqlx::query_as`（**不使用** `query!` 宏）。

use rust_decimal::Decimal;
use sqlx::PgPool;

use nextrade_core::models::trade::check::{invoice_filter, OrderInvoiceVO};
use nextrade_core::{Error, Result};

/// 订单结算域仓储（持有连接池）。
#[derive(Clone)]
pub struct CheckRepository {
    pool: PgPool,
}

impl CheckRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（与其它仓储一致）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 读取配置值（Java `ConfigService#getConfigVal`，缺失 → `None`）。
    pub async fn find_config_value(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1",
        )
        .bind(biz_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|(value,)| value))
    }

    /// 批量读取配置值（结算页积分/账期等配置，一次 `IN` 查询）。
    ///
    /// # Errors
    /// 数据库错误。
    pub async fn find_config_values(
        &self,
        biz_codes: &[&str],
    ) -> Result<std::collections::HashMap<String, String>> {
        if biz_codes.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let placeholders = vec!["?"; biz_codes.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT biz_code, biz_val FROM config WHERE biz_code IN ({placeholders}) AND is_del = 0"
        ));
        let mut q = sqlx::query_as::<_, (String, Option<String>)>(&sql);
        for code in biz_codes {
            q = q.bind(*code);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows
            .into_iter()
            .filter_map(|(code, value)| value.map(|v| (code, v)))
            .collect())
    }

    /// 最近一条发票（Java `getLastInvoice`：`invoice_type/title_type` 缺省 0，
    /// `user_id` + `ORDER BY id DESC LIMIT 1`）。
    pub async fn find_last_invoice(
        &self,
        user_id: i32,
        invoice_type: Option<i32>,
        title_type: Option<i32>,
    ) -> Result<Option<OrderInvoiceVO>> {
        let (invoice_type, title_type) = invoice_filter(invoice_type, title_type);
        let row = sqlx::query_as::<_, InvoiceRow>(
            "SELECT id, user_id, order_id, invoice_type, status, apply_reply, title_type,
                    company_code, company_name, company_address, company_phone, company_bank,
                    company_account, invoice_content, invoice_no, amount, mobile, email,
                    add_time, NULL AS invoice_attachment, NULL AS invoicing_time
             FROM order_invoice
             WHERE user_id = $1 AND invoice_type = $2 AND title_type = $3
             ORDER BY id DESC LIMIT 1",
        )
        .bind(user_id)
        .bind(invoice_type)
        .bind(title_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(InvoiceRow::into_vo))
    }
}

/// `order_invoice` 行映射。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
struct InvoiceRow {
    id: i32,
    user_id: i32,
    order_id: i32,
    invoice_type: i32,
    status: i32,
    apply_reply: String,
    title_type: i32,
    company_code: String,
    company_name: String,
    company_address: String,
    company_phone: String,
    company_bank: String,
    company_account: String,
    invoice_content: String,
    invoice_no: String,
    amount: Decimal,
    mobile: String,
    email: String,
    add_time: i32,
    invoice_attachment: Option<String>,
    invoicing_time: Option<i32>,
}

impl InvoiceRow {
    fn into_vo(self) -> OrderInvoiceVO {
        OrderInvoiceVO {
            id: self.id,
            user_id: self.user_id,
            order_id: self.order_id,
            invoice_type: self.invoice_type,
            status: self.status,
            apply_reply: self.apply_reply,
            title_type: self.title_type,
            company_code: self.company_code,
            company_name: self.company_name,
            company_address: self.company_address,
            company_phone: self.company_phone,
            company_bank: self.company_bank,
            company_account: self.company_account,
            invoice_content: self.invoice_content,
            invoice_no: self.invoice_no,
            amount: self.amount,
            mobile: self.mobile,
            email: self.email,
            add_time: self.add_time,
            invoice_attachment: self.invoice_attachment,
            invoicing_time: self.invoicing_time,
        }
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoice_row_maps_to_vo() {
        let row = InvoiceRow {
            id: 1,
            user_id: 2,
            order_id: 3,
            invoice_type: 1,
            status: 2,
            apply_reply: "已开".into(),
            title_type: 1,
            company_code: "913".into(),
            company_name: "公司".into(),
            company_address: "地址".into(),
            company_phone: "123".into(),
            company_bank: "银行".into(),
            company_account: "6222".into(),
            invoice_content: "商品明细".into(),
            invoice_no: "NO1".into(),
            amount: Decimal::new(1000, 2),
            mobile: "138".into(),
            email: "a@b.c".into(),
            add_time: 100,
            invoice_attachment: None,
            invoicing_time: Some(200),
        };
        let vo = row.into_vo();
        assert_eq!(vo.invoice_no, "NO1");
        assert_eq!(vo.amount, Decimal::new(1000, 2));
    }

    #[test]
    fn repository_type_is_constructible() {
        assert!(std::any::type_name::<CheckRepository>().contains("CheckRepository"));
    }
}
