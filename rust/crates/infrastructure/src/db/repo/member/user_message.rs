//! 站内信（C 端）仓储：`user_message`，兼查 `im_message` 统计 IM 未读数。
//!
//! 对应原 Java `UserMessageMapper` / `ImMessageMapper`（仅 C 端用到的部分）。
//!
//! 列名严格对齐 `java/database/pure.sql`：
//! - `user_message`：`message_id` / `message_log_id` / `user_id` / `user_rank` /
//!   `is_read` / `title` / `content` / `message_template` / `link` / `add_time` / `type`
//! - `im_message`：仅 `user_id` / `type` / `is_read`（未读统计）
//!
//! 列表过滤逐字复刻 `UserMessageServiceImpl#list`：
//! 1. `user_id = ?`；
//! 2. `unread == 1` → `is_read = 0`；
//! 3. `type > 0` → `type = ?`；否则 `type <> 1`（未传类型时排除订单通知）。

use sqlx::{Postgres,PgPool, QueryBuilder};
use nextrade_core::entities::UserMessage;
use nextrade_core::models::member::user_message::{
    resolve_sort, UserMessageListQuery, TYPE_ORDER,
};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// `user_message` 表全部列（顺序与 [`UserMessage`] 字段一致）。
const MESSAGE_COLUMNS: &str = "message_id, message_log_id, user_id, user_rank, is_read, title, \
     content, message_template, link, add_time, type";

#[derive(Clone)]
pub struct UserMessageRepository {
    pool: PgPool,
}

impl UserMessageRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层需要时复用）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 站内信分页（对应 Java `UserMessageServiceImpl#list` 的查询部分）。
    pub async fn paginate(
        &self,
        user_id: i32,
        q: &UserMessageListQuery,
    ) -> Result<(u64, Vec<UserMessage>)> {
        let page = q.page_query();
        let (limit, offset) = page.limit_offset();
        let (column, desc) = resolve_sort(q.sort_field.as_deref(), q.sort_order.as_deref());
        let order = order_clause(column, desc);

        let mut count_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM user_message WHERE 1 = 1");
        Self::push_filters(&mut count_qb, user_id, q);
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut row_qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!(
            "SELECT {MESSAGE_COLUMNS} FROM user_message WHERE 1 = 1"
        ));
        Self::push_filters(&mut row_qb, user_id, q);
        row_qb.push(order);
        row_qb.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<UserMessage> = row_qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 追加列表 WHERE 片段（count 与取行共用）。
    fn push_filters(qb: &mut QueryBuilder<'_, Postgres>, user_id: i32, q: &UserMessageListQuery) {
        qb.push(" AND user_id = ");
        qb.push_bind(user_id);
        if q.only_unread() {
            qb.push(" AND is_read = 0");
        }
        match q.r#type {
            Some(t) if t > 0 => {
                qb.push(" AND type = ");
                qb.push_bind(t);
            }
            // Java：未传 type（或 <=0）时排除订单通知 type = 1
            _ => {
                qb.push(" AND type <> ");
                qb.push_bind(TYPE_ORDER);
            }
        }
    }

    /// 按主键查询（Java `getById`）。
    pub async fn find_by_id(&self, message_id: i32) -> Result<Option<UserMessage>> {
        let sql = crate::db::to_pg_placeholders(&format!("SELECT {MESSAGE_COLUMNS} FROM user_message WHERE message_id = ? LIMIT 1"));
        sqlx::query_as::<_, UserMessage>(&sql)
            .bind(message_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 全部置为已读（Java \"setAllMessageRead\"：`user_id = $1 AND is_read = 0`）。
    pub async fn update_all_read(&self, user_id: i32) -> Result<u64> {
        let r = sqlx::query("UPDATE user_message SET is_read = 1 WHERE user_id = $1 AND is_read = 0")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 订单消息置为已读（Java \"setOrderMessageRead\"：type = 1）。
    pub async fn update_order_read(&self, user_id: i32) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE user_message SET is_read = 1 \
             WHERE user_id = $1 AND is_read = 0 AND type = 1",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 单条消息置为已读（Java `setMessageRead`）。
    pub async fn set_read(&self, user_id: i32, message_id: i32) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE user_message SET is_read = 1 WHERE user_id = $1 AND message_id = $2",
        )
        .bind(user_id)
        .bind(message_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 插入单条站内信（消息通知，对应 Java sendUserMessage）
    pub async fn insert_user_message(
        &self,
        user_id: i32,
        title: &str,
        content: &str,
        msg_type: i32,
        link: Option<&str>,
    ) -> Result<i32> {
        let now = nextrade_common::utils::now_seconds() as i32;
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO user_message (message_log_id, user_id, user_rank, is_read, title, content, message_template, link, add_time, type) \
             VALUES (0, $1, 0, 0, $2, $3, '', $4, $5, $6) RETURNING message_id",
        )
        .bind(user_id)
        .bind(title)
        .bind(content)
        .bind(link.unwrap_or(""))
        .bind(now)
        .bind(msg_type)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 删除单条（Java \"removeById\"）。
    pub async fn delete(&self, message_id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM user_message WHERE message_id = $1")
            .bind(message_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 全部类型未读数（Java `getUnreadMessageCount` 第 1 个 count）。
    pub async fn count_unread_all(&self, user_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM user_message WHERE user_id = $1 AND is_read = 0",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 订单通知未读数（type = 1）。
    pub async fn count_unread_order(&self, user_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM user_message WHERE user_id = $1 AND type = 1 AND is_read = 0",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// IM 消息未读数（Java `ImMessageService#countUnreadServantMessagesByUserId`：
    /// `im_message.user_id = ? AND type = 2 AND is_read = 0`）。
    pub async fn count_unread_im(&self, user_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM im_message WHERE user_id = $1 AND type = 2 AND is_read = 0",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// IM 消息全部置读（Java `ImMessageService#setAllRead`：
    /// `im_message.user_id = $2 AND type = 2 AND is_read = 0` → `is_read = 1`）。
    pub async fn update_all_im_read(&self, user_id: i32) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE im_message SET is_read = 1 \
             WHERE user_id = $1 AND type = 2 AND is_read = 0",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }
}

/// `ORDER BY` 子句（列名已由 models 白名单归一）。
fn order_clause(column: &'static str, desc: Option<bool>) -> String {
    match desc {
        Some(true) => format!(" ORDER BY {column} DESC"),
        Some(false) => format!(" ORDER BY {column} ASC"),
        // Java：既非 asc 也非 desc → 不设置排序
        None => String::new(),
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("站内消息不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_clause_matches_base_sort_order() {
        assert_eq!(order_clause("message_id", Some(true)), " ORDER BY message_id DESC");
        assert_eq!(order_clause("add_time", Some(false)), " ORDER BY add_time ASC");
        assert_eq!(order_clause("title", None), "");
    }

    #[test]
    fn order_clause_uses_whitelisted_columns_only() {
        for column in nextrade_core::models::member::user_message::SORTABLE_COLUMNS {
            assert_eq!(
                order_clause(column, Some(true)),
                format!(" ORDER BY {column} DESC")
            );
        }
    }
}
