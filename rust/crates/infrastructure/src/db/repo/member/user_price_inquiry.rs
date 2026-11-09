//! 用户询价仓储（member::user_price_inquiry）。表 `price_inquiry` /
//! `price_inquiry_reply_record` / `product` / `user`。

use sqlx::PgPool;
use nextrade_core::models::member::user_price_inquiry::{
    InquiryProductVO, PriceInquiryDetailVO, PriceInquiryListQuery, PriceInquiryReplyRecordVO,
    PriceInquiryVO,
};
use nextrade_core::{Error, Result};

pub type InquiryRow = (
    i32, String, String, String, i32, String, i32, i32, i32, Option<i32>,
);

const INQUIRY_COLUMNS: &str =
    "id, mobile, email, content, product_id, remark, status, shop_id, user_id, create_time";

#[derive(Clone)]
pub struct UserPriceInquiryRepository {
    pool: PgPool,
}

impl UserPriceInquiryRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 当前用户询价分页（user_id + user_is_del=0 + 可选 status/keyword(mobile/email/content)）。
    pub async fn paginate(
        &self,
        user_id: i32,
        q: &PriceInquiryListQuery,
    ) -> Result<(u64, Vec<PriceInquiryVO>)> {
        let mut where_sql = String::from(" WHERE user_id = ? AND user_is_del = 0");
        if q.status.is_some() {
            where_sql.push_str(" AND status = ?");
        }
        let like = q
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{s}%"));
        if like.is_some() {
            where_sql.push_str(" AND (content LIKE ? OR mobile LIKE ? OR email LIKE ?)");
        }
        let total_sql = format!("SELECT COUNT(*) FROM price_inquiry{where_sql}");
        let mut cq = sqlx::query_as::<_, (i64,)>(&total_sql).bind(user_id);
        if let Some(s) = q.status {
            cq = cq.bind(s);
        }
        if let Some(v) = like.as_deref() {
            cq = cq.bind(v).bind(v).bind(v);
        }
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let sql = format!(
            "SELECT {INQUIRY_COLUMNS} FROM price_inquiry{where_sql} ORDER BY id DESC LIMIT ? OFFSET ?"
        );
        let (page, size) = q.page_params();
        let mut lq = sqlx::query_as::<_, InquiryRow>(&sql).bind(user_id);
        if let Some(s) = q.status {
            lq = lq.bind(s);
        }
        if let Some(v) = like.as_deref() {
            lq = lq.bind(v).bind(v).bind(v);
        }
        let rows = lq
            .bind(i64::try_from(size).unwrap_or(15))
            .bind(i64::try_from((page - 1) * size).unwrap_or(0))
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        let products = self.products(&rows).await?;
        let list = rows
            .into_iter()
            .map(|r| PriceInquiryVO {
                id: r.0,
                mobile: r.1,
                email: r.2,
                content: r.3,
                product_id: r.4,
                remark: r.5,
                status: r.6,
                shop_id: r.7,
                user_id: r.8,
                create_time: nextrade_core::models::member::user_price_inquiry::format_time(r.9),
                product: products.get(&r.4).cloned(),
            })
            .collect();
        Ok((u64::try_from(total.0).unwrap_or(0), list))
    }

    /// 当前用户询价详情（含回复记录与商品/联系人）。
    pub async fn detail(&self, id: i32, user_id: i32) -> Result<Option<PriceInquiryDetailVO>> {
        let row: Option<InquiryRow> = sqlx::query_as(&format!(
            "SELECT {INQUIRY_COLUMNS} FROM price_inquiry WHERE id = $1 AND user_id = $2 AND user_is_del = 0 LIMIT 1"
        ))
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        let Some(r) = row else {
            return Ok(None);
        };
        let products = self.products(std::slice::from_ref(&r)).await?;
        let product = products.get(&r.4).cloned();
        let contact = self.contact_name(r.8).await?;
        let reply_records = self.reply_records(r.0).await?;
        Ok(Some(PriceInquiryDetailVO {
            id: r.0,
            content: r.3,
            create_time: nextrade_core::models::member::user_price_inquiry::format_time(r.9),
            contact_name: contact,
            mobile: r.1,
            email: r.2,
            remark: r.5,
            status: r.6,
            product,
            reply_records,
        }))
    }

    /// 用户回复：置 status=0 + 插入回复记录（reply_type=2）。
    pub async fn reply(&self, id: i32, user_id: i32, remark: &str, now: i32) -> Result<bool> {
        let r = sqlx::query(
            "UPDATE price_inquiry SET status = 0 WHERE id = $1 AND user_id = $2 AND user_is_del = 0",
        )
        .bind(id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        if r.rows_affected() == 0 {
            return Ok(false);
        }
        let reply_name = self.contact_name(user_id).await?;
        sqlx::query(
            "INSERT INTO price_inquiry_reply_record (inquiry_id, reply_type, reply_name, content, admin_id, user_id, create_time) VALUES ($1, 2, $2, $3, 0, $4, $5)",
        )
        .bind(id)
        .bind(reply_name)
        .bind(remark)
        .bind(user_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(true)
    }

    /// 用户侧软删（user_is_del=1）。
    pub async fn soft_delete(&self, id: i32, user_id: i32) -> Result<bool> {
        let r = sqlx::query(
            "UPDATE price_inquiry SET user_is_del = 1 WHERE id = $1 AND user_id = $2 AND user_is_del = 0",
        )
        .bind(id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    async fn products(
        &self,
        rows: &[InquiryRow],
    ) -> Result<std::collections::HashMap<i32, InquiryProductVO>> {
        let ids: Vec<i32> = rows.iter().map(|r| r.4).filter(|i| *i > 0).collect();
        if ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let ph = vec!["?"; ids.len()].join(", ");
        let sql = format!(
            "SELECT product_id, product_name, product_sn, pic_thumb FROM product WHERE product_id IN ({ph})"
        );
        let mut q = sqlx::query_as::<_, (i32, String, String, String)>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        Ok(q.fetch_all(&self.pool)
            .await
            .map_err(map_err)?
            .into_iter()
            .map(|(product_id, product_name, product_sn, pic_thumb)| {
                (
                    product_id,
                    InquiryProductVO {
                        product_id,
                        product_name,
                        product_sn,
                        pic_thumb,
                    },
                )
            })
            .collect())
    }

    async fn contact_name(&self, user_id: i32) -> Result<String> {
        let row: Option<(String, Option<String>)> =
            sqlx::query_as("SELECT username, nickname FROM user WHERE user_id = $1 LIMIT 1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row
            .map(|(username, nickname)| nickname.unwrap_or(username))
            .unwrap_or_default())
    }

    async fn reply_records(&self, inquiry_id: i32) -> Result<Vec<PriceInquiryReplyRecordVO>> {
        let rows: Vec<(String, String, i32, i32)> = sqlx::query_as(
            "SELECT reply_name, content, reply_type, create_time FROM price_inquiry_reply_record WHERE inquiry_id = $1 ORDER BY id ASC",
        )
        .bind(inquiry_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(reply_name, content, reply_type, create_time)| {
                PriceInquiryReplyRecordVO {
                    reply_name,
                    create_time:
                        nextrade_core::models::member::user_price_inquiry::format_time(Some(create_time)),
                    content,
                    reply_type,
                }
            })
            .collect())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}
