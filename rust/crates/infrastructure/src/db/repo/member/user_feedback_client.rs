//! 用户留言仓储（member::user_feedback_client）。表 `feedback`。

use sqlx::PgPool;
use nextrade_core::models::member::user_feedback_client::{
    FeedbackListQuery, FeedbackReplyVO, UserFeedbackVO,
};
use nextrade_core::{Error, Result};

pub type FeedbackRow = (
    i32, i32, Option<String>, String, i32, i32, i32, i32, Option<i32>, Option<i32>, i32,
);

const COLUMNS: &str = "id, type, title, content, status, add_time, parent_id, user_id, product_id, order_id, shop_id";

#[derive(Clone)]
pub struct UserFeedbackClientRepository {
    pool: PgPool,
}

impl UserFeedbackClientRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 当前用户留言分页（parent_id=0，含回复子记录）。
    pub async fn paginate(
        &self,
        user_id: i32,
        q: &FeedbackListQuery,
    ) -> Result<(u64, Vec<UserFeedbackVO>)> {
        let mut where_sql = String::from(" WHERE user_id = $1 AND parent_id = 0");
        if q.status.is_some() {
            where_sql.push_str(" AND status = ?");
        }
        if q.product_id.is_some() {
            where_sql.push_str(" AND product_id = ?");
        }
        let total_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM feedback{where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&total_sql).bind(user_id);
        if let Some(s) = q.status {
            cq = cq.bind(s);
        }
        if let Some(p) = q.product_id {
            cq = cq.bind(p);
        }
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let (page, size) = q.page_params();
        let sql =
            crate::db::to_pg_placeholders(&format!("SELECT {COLUMNS} FROM feedback{where_sql} ORDER BY id DESC LIMIT ? OFFSET ?"));
        let mut lq = sqlx::query_as::<_, FeedbackRow>(&sql).bind(user_id);
        if let Some(s) = q.status {
            lq = lq.bind(s);
        }
        if let Some(p) = q.product_id {
            lq = lq.bind(p);
        }
        let rows = lq
            .bind(i64::try_from(size).unwrap_or(15))
            .bind(i64::try_from((page - 1) * size).unwrap_or(0))
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        let replies = self.replies(&rows).await?;
        let list = rows
            .into_iter()
            .map(|r| UserFeedbackVO {
                id: r.0,
                title: r.2.unwrap_or_default(),
                content: r.3,
                r#type: r.1,
                type_name: nextrade_core::models::member::user_feedback_client::type_name(r.1)
                    .to_string(),
                status: r.4,
                status_text:
                    nextrade_core::models::member::user_feedback_client::status_text(r.4).to_string(),
                add_time: nextrade_core::models::member::user_feedback_client::format_time(Some(
                    r.5,
                )),
                product_id: r.8,
                order_id: r.9,
                reply: replies.get(&r.0).cloned(),
            })
            .collect();
        Ok((u64::try_from(total.0).unwrap_or(0), list))
    }

    /// 用户是否存在（Java `error.member.not.found`）。
    pub async fn user_exists(&self, user_id: i32) -> Result<Option<(String, String)>> {
        sqlx::query_as("SELECT username, mobile FROM \"user\" WHERE user_id = $1 LIMIT 1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 订单归属校验（Java：订单不存在 / 非本人订单）。
    pub async fn order_owner(&self, order_id: i32) -> Result<Option<i32>> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT user_id FROM \"order\" WHERE order_id = $1 LIMIT 1")
                .bind(order_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|(u,)| u))
    }

    /// 提交留言（parent_id=0，status=0）。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert(
        &self,
        writer: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user_id: i32,
        username: &str,
        r#type: i32,
        title: &str,
        content: &str,
        product_id: Option<i32>,
        order_id: Option<i32>,
        shop_id: Option<i32>,
        complaint_info: Option<&str>,
        email: Option<&str>,
        mobile: Option<&str>,
        feedback_pics: &str,
        now: i32,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO feedback (parent_id, user_id, username, email, mobile, title, type, status, content, add_time, feedback_pics, product_id, order_id, complaint_info, shop_id) VALUES (0, $1, $2, $3, $4, $5, $6, 0, $7, $8, $9, $10, $11, $12, $13) RETURNING id",
        )
        .bind(user_id)
        .bind(username)
        .bind(email)
        .bind(mobile)
        .bind(title)
        .bind(r#type)
        .bind(content)
        .bind(now)
        .bind(feedback_pics)
        .bind(product_id)
        .bind(order_id)
        .bind(complaint_info)
        .bind(shop_id)
        .fetch_one(&mut **writer)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 留言配置（敏感词 `msgHackWord`）。
    pub async fn config_value(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = $1 AND (is_del IS NULL OR is_del = 0) LIMIT 1",
        )
        .bind(biz_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }

    async fn replies(
        &self,
        rows: &[FeedbackRow],
    ) -> Result<std::collections::HashMap<i32, FeedbackReplyVO>> {
        let ids: Vec<i32> = rows.iter().map(|r| r.0).collect();
        if ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let ph = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT id, parent_id, username, content, add_time FROM feedback WHERE parent_id IN ({ph}) ORDER BY id ASC"
        ));
        let mut q = sqlx::query_as::<_, (i32, i32, String, String, i32)>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        let mut map = std::collections::HashMap::new();
        for (id, parent_id, username, content, add_time) in
            q.fetch_all(&self.pool).await.map_err(map_err)?
        {
            map.entry(parent_id).or_insert_with(|| FeedbackReplyVO {
                id,
                parent_id,
                username,
                content,
                add_time: nextrade_core::models::member::user_feedback_client::format_time(Some(
                    add_time,
                )),
            });
        }
        Ok(map)
    }

    /// 事务包装提交（保存 + 可扩展后台消息写入）。
    pub async fn begin(&self) -> Result<sqlx::Transaction<'static, sqlx::Postgres>> {
        self.pool.begin().await.map_err(map_err)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}
