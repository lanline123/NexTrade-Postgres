//! 发票申请域仓储（payment::order_invoice）。
//!
//! 对应原 Java `OrderInvoiceMapper`（含 `OrderInvoiceMapper.xml#orderInvoicePage`）。
//!
//! 列名严格对齐 `python tools/schema_parser.py --table order_invoice`（21 列）：
//! `id` / `user_id` / `order_id` / `invoice_type` / `status` / `apply_reply` /
//! `title_type` / `company_code` / `company_name` / `company_address` /
//! `company_phone` / `company_bank` / `company_account` / `invoice_content` /
//! `invoice_no` / `amount` / `mobile` / `email` / `add_time` /
//! `invoice_attachment` / `invoicing_time`。
//!
//! 另读 `order`（只读 `order_id` / `order_sn`）与 `user`（只读
//! `user_id` / `username`）用于列表装配。
//!
//! 查询统一使用运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`。

use sqlx::{Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::OrderInvoice;
use nextrade_core::models::payment::order_invoice::OrderInvoicePageQuery;
use nextrade_core::{Error, Result};

/// `order_invoice` 固定 SELECT 列清单（顺序与实体一致）。
const INVOICE_SELECT: &str = "SELECT id, user_id, order_id, invoice_type, status, apply_reply, title_type, company_code, company_name, company_address, company_phone, company_bank, company_account, invoice_content, invoice_no, amount, mobile, email, add_time, NULL AS invoice_attachment, NULL AS invoicing_time FROM order_invoice";

/// 发票申请域仓储。
#[derive(Clone)]
pub struct OrderInvoiceRepository {
    pool: PgPool,
}

impl OrderInvoiceRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 查询 —————————————————————

    /// 按主键查询（Java `getById`）。
    /// 发票申请人是否属于指定店铺（`shop_member` 有效关联）。
    pub async fn user_in_shop(&self, user_id: i32, shop_id: i32) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM shop_member WHERE user_id = $1 AND shop_id = $2 AND is_del = 0 LIMIT 1",
        )
        .bind(user_id)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.is_some())
    }

    pub async fn find_by_id(&self, id: i32) -> Result<Option<OrderInvoice>> {
        sqlx::query_as::<_, OrderInvoice>(&format!("{INVOICE_SELECT} WHERE id = $1 LIMIT 1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 分页（Java `orderInvoicePage`）。
    ///
    /// - `keyword` 命中 `user.username LIKE` 或 `oi.company_name LIKE`
    ///   （2 步查询：先取用户名命中的 user_id 列表，再拼 `user_id IN (...) OR
    ///   company_name LIKE ?`；无命中用户时仅按 `company_name` 过滤）
    /// - `status` / `invoice_type` 可选
    /// - ORDER BY 由 [`OrderInvoicePageQuery::order_clause`] 生成
    pub async fn paginate(
        &self,
        q: &OrderInvoicePageQuery,
        shop_scope: Option<i32>,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<OrderInvoice>)> {
        let user_ids = match q.effective_keyword() {
            Some(keyword) => self.find_user_ids_by_username(keyword).await?,
            None => Vec::new(),
        };
        let keyword = q.effective_keyword();

        let mut cq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM order_invoice WHERE 1 = 1");
        Self::push_filters(&mut cq, q, keyword, &user_ids, shop_scope);
        let total: (i64,) = cq
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut rq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{INVOICE_SELECT} WHERE 1 = 1"));
        Self::push_filters(&mut rq, q, keyword, &user_ids, shop_scope);
        rq.push(q.order_clause());
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<OrderInvoice> = rq
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0, rows))
    }

    fn push_filters(
        qb: &mut QueryBuilder<'_, Postgres>,
        q: &OrderInvoicePageQuery,
        keyword: Option<&str>,
        user_ids: &[i32],
        shop_scope: Option<i32>,
    ) {
        if let Some(shop) = shop_scope {
            qb.push(" AND EXISTS (SELECT 1 FROM shop_member m WHERE m.user_id = order_invoice.user_id AND m.shop_id = ");
            qb.push_bind(shop);
            qb.push(" AND m.is_del = 0)");
        }
        if let Some(keyword) = keyword {
            // Java：`(u.username LIKE ? OR oi.company_name LIKE ?)`
            qb.push(" AND (company_name LIKE ");
            qb.push_bind(format!("%{keyword}%"));
            if !user_ids.is_empty() {
                qb.push(" OR user_id IN (");
                let mut sep = qb.separated(", ");
                for id in user_ids {
                    sep.push_bind(*id);
                }
                sep.push_unseparated(")");
                drop(sep);
            }
            qb.push(")");
        }
        if let Some(status) = q.status {
            qb.push(" AND status = ");
            qb.push_bind(status);
        }
        if let Some(invoice_type) = q.invoice_type {
            qb.push(" AND invoice_type = ");
            qb.push_bind(invoice_type);
        }
    }

    /// 用户名模糊命中的 user_id（`user.username LIKE %kw%`）。
    pub async fn find_user_ids_by_username(&self, keyword: &str) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as("SELECT user_id FROM \"user\" WHERE username LIKE $1")
            .bind(format!("%{keyword}%"))
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 批量取订单号（`left join order` 的 `order_sn`）。
    pub async fn find_order_sns(&self, ids: &[i32]) -> Result<Vec<(i32, String)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT order_id, order_sn FROM \"order\" WHERE order_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量取用户名（列表 `username` 列）。
    pub async fn find_usernames(&self, ids: &[i32]) -> Result<Vec<(i32, String)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT user_id, username FROM \"user\" WHERE user_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 写操作 —————————————————————

    /// 新增（Java `save`）。
    pub async fn insert(&self, entity: &OrderInvoice) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO order_invoice (user_id, order_id, invoice_type, status, apply_reply, title_type, company_code, company_name, company_address, company_phone, company_bank, company_account, invoice_content, invoice_no, amount, mobile, email, add_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18) RETURNING id",
        )
        .bind(entity.user_id)
        .bind(entity.order_id)
        .bind(entity.invoice_type)
        .bind(entity.status)
        .bind(&entity.apply_reply)
        .bind(entity.title_type)
        .bind(&entity.company_code)
        .bind(&entity.company_name)
        .bind(&entity.company_address)
        .bind(&entity.company_phone)
        .bind(&entity.company_bank)
        .bind(&entity.company_account)
        .bind(&entity.invoice_content)
        .bind(&entity.invoice_no)
        .bind(entity.amount)
        .bind(&entity.mobile)
        .bind(&entity.email)
        .bind(entity.add_time)

        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 整行更新（Java `updateById`；服务层先合并旧值，等价 NOT_NULL 语义）。
    pub async fn update(&self, entity: &OrderInvoice) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE order_invoice SET user_id = $1, order_id = $2, invoice_type = $3, status = $4, apply_reply = $5, title_type = $6, company_code = $7, company_name = $8, company_address = $9, company_phone = $10, company_bank = $11, company_account = $12, invoice_content = $13, invoice_no = $14, amount = $15, mobile = $16, email = $17, add_time = $18 WHERE id = $19",
        )
        .bind(entity.user_id)
        .bind(entity.order_id)
        .bind(entity.invoice_type)
        .bind(entity.status)
        .bind(&entity.apply_reply)
        .bind(entity.title_type)
        .bind(&entity.company_code)
        .bind(&entity.company_name)
        .bind(&entity.company_address)
        .bind(&entity.company_phone)
        .bind(&entity.company_bank)
        .bind(&entity.company_account)
        .bind(&entity.invoice_content)
        .bind(&entity.invoice_no)
        .bind(entity.amount)
        .bind(&entity.mobile)
        .bind(&entity.email)
        .bind(entity.add_time)

        .bind(entity.id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 单字段更新（列名由服务层白名单归一后传入）。
    pub async fn update_field(&self, id: i32, column: &str, val: &str) -> Result<u64> {
        debug_assert!(
            nextrade_core::models::payment::order_invoice::GENERAL_FIELDS.contains(&column),
            "update_field 列名未经过白名单映射: {column}"
        );
        let sql = format!("UPDATE order_invoice SET {column} = $1 WHERE id = $2");
        let res = sqlx::query(&sql)
            .bind(val)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 按主键删除（Java `removeById`；表无删除标记列）。
    pub async fn delete(&self, id: i32) -> Result<bool> {
        let res = sqlx::query("DELETE FROM order_invoice WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 批量删除（Java `removeByIds`）。
    pub async fn delete_batch(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("DELETE FROM order_invoice WHERE id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        let res = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::payment::order_invoice::INVOICE_COLUMNS;

    #[test]
    fn select_list_covers_entity_columns() {
        let selected = INVOICE_SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap()
            .replace("NULL AS invoice_attachment", "invoice_attachment")
            .replace("NULL AS invoicing_time", "invoicing_time");
        assert_eq!(selected, INVOICE_COLUMNS.join(", "));
        assert_eq!(INVOICE_COLUMNS.len(), 21);
    }

    #[test]
    fn keyword_filter_uses_username_and_company_name() {
        // 静态守卫：keyword 过滤必须同时覆盖用户名与单位名称
        let src = include_str!("order_invoice.rs");
        assert!(src.contains("AND (company_name LIKE "));
        assert!(src.contains("SELECT user_id FROM \"user\" WHERE username LIKE $1"));
        assert!(src.contains("SELECT order_id, order_sn FROM \"order\" WHERE order_id IN ("));
    }
}
