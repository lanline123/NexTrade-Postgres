//! 会员消息管理仓储：`user_message_log`，兼写 `user_message`、读 `user`。
//!
//! 对应原 Java `UserMessageLogMapper` / `UserMessageMapper` / `UserMapper`
//! （仅本域用到的部分）。
//!
//! 列名严格对齐 `java/database/pure.sql`：
//! - `user_message_log`：`message_log_id` / `message_type` / `send_user_type` /
//!   `user_ids` / `user_rank` / `message_title` / `message_content` /
//!   `message_link` / `send_time` / `is_recall`
//! - `user_message`：发送时批量插入（`message_log_id` / `user_id` / `user_rank` /
//!   `is_read` / `title` / `content` / `link` / `add_time`；`type` 走 DB 默认 0）
//!
//! 事务：`create`（写日志 + 批量写站内信）与 `recall`（改日志 + 删站内信）在
//! Java 是 `@Transactional`，本实现在仓储层用 `pool.begin()` 保证原子性。

use sqlx::{Postgres,PgPool, QueryBuilder};
use nextrade_core::entities::UserMessageLog;
use nextrade_core::models::member::user_message_log::{resolve_sort, UserMessageLogListQuery};
use nextrade_core::{Error, Result};

/// `user_message_log` 表全部列（顺序与 [`UserMessageLog`] 字段一致）。
const LOG_COLUMNS: &str = "message_log_id, message_type, send_user_type, user_ids, user_rank, \
     message_title, message_content, message_link, send_time, is_recall";

#[derive(Clone)]
pub struct UserMessageLogRepository {
    pool: PgPool,
}

impl UserMessageLogRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 读 —————————————————————

    /// 日志分页（Java `UserMessageLogServiceImpl#list`：
    /// `message_title LIKE keyword` + `buildSortOrder` 分页）。
    pub async fn paginate(
        &self,
        q: &UserMessageLogListQuery,
    ) -> Result<(u64, Vec<UserMessageLog>)> {
        let page = q.page_query();
        let (limit, offset) = page.limit_offset();
        let (column, desc) = resolve_sort(q.sort_field.as_deref(), q.sort_order.as_deref());
        let order = order_clause(column, desc);
        let like = q
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(|k| format!("%{k}%"));

        let count_sql =
            "SELECT COUNT(*) FROM user_message_log WHERE ($1 IS NULL OR message_title LIKE $2)";
        let total: (i64,) = sqlx::query_as(count_sql)
            .bind(&like)
            .bind(&like)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let list_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {LOG_COLUMNS} FROM user_message_log \
             WHERE (? IS NULL OR message_title LIKE ?){order} LIMIT ? OFFSET ?"
        ));
        let rows = sqlx::query_as::<_, UserMessageLog>(&list_sql)
            .bind(&like)
            .bind(&like)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 按主键查（Java `getOne(eq message_log_id)`）。
    pub async fn find_by_id(&self, log_id: i32) -> Result<Option<UserMessageLog>> {
        let sql = crate::db::to_pg_placeholders(&format!("SELECT {LOG_COLUMNS} FROM user_message_log WHERE message_log_id = ?"));
        sqlx::query_as::<_, UserMessageLog>(&sql)
            .bind(log_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    // ————————————————————— 人群解析（sendUserType 0/1/2/3） —————————————————————

    /// 全部会员 ID（`sendUserType = 0`）。
    pub async fn all_user_ids(&self) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as("SELECT user_id FROM \"user\"")
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 按会员等级取会员 ID（`sendUserType = 2`）。
    pub async fn user_ids_by_rank(&self, rank_id: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as("SELECT user_id FROM \"user\" WHERE rank_id = $1")
            .bind(rank_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 按会员名批量取会员 ID（`sendUserType = 3`）。
    pub async fn user_ids_by_usernames(&self, usernames: &[String]) -> Result<Vec<i32>> {
        if usernames.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT user_id FROM \"user\" WHERE username IN (");
        let mut sep = qb.separated(", ");
        for name in usernames {
            sep.push_bind(name.clone());
        }
        sep.push_unseparated(")");
        drop(sep);
        let rows: Vec<(i32,)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    // ————————————————————— 写 —————————————————————

    /// 写日志 + 批量写站内信（Java `#create`，`@Transactional`）。
    ///
    /// 返回新日志主键；`user_ids` 为空时不写站内信（Java `saveBatch(empty)` 无操作）。
    pub async fn create_log_with_messages(
        &self,
        log: &UserMessageLog,
        user_ids: &[i32],
        add_time: i32,
    ) -> Result<i32> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO user_message_log \
             (message_type, send_user_type, user_ids, user_rank, message_title, \
              message_content, message_link, send_time, is_recall) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING message_log_id",
        )
        .bind(log.message_type)
        .bind(log.send_user_type)
        .bind(&log.user_ids)
        .bind(log.user_rank)
        .bind(&log.message_title)
        .bind(&log.message_content)
        .bind(&log.message_link)
        .bind(log.send_time)
        .bind(log.is_recall)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_err)?;
        let log_id = r;

        for user_id in user_ids {
            sqlx::query(
                "INSERT INTO user_message \
                 (message_log_id, user_id, user_rank, is_read, title, content, link, add_time) \
                 VALUES ($1, $2, 0, 0, $3, $4, $5, $6)",
            )
            .bind(log_id)
            .bind(*user_id)
            .bind(&log.message_title)
            .bind(&log.message_content)
            .bind(&log.message_link)
            .bind(add_time)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        }
        tx.commit().await.map_err(map_err)?;
        Ok(log_id)
    }

    /// 撤回（Java `recall`，`@Transactional`）：日志 `is_recall = 1`，
    /// 并删除对应 `user_message`。
    pub async fn recall(&self, log_id: i32) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        sqlx::query("UPDATE user_message_log SET is_recall = 1 WHERE message_log_id = $1")
            .bind(log_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        sqlx::query("DELETE FROM user_message WHERE message_log_id = $1")
            .bind(log_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        tx.commit().await.map_err(map_err)?;
        Ok(())
    }

    /// 删除单条日志（Java `BaseServiceImpl#del`，不级联）。
    pub async fn delete(&self, log_id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM user_message_log WHERE message_log_id = $1")
            .bind(log_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 批量删除日志（Java `BaseServiceImpl#batch`）。
    pub async fn delete_many(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("DELETE FROM user_message_log WHERE message_log_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        let r = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(r.rows_affected())
    }
}

/// `ORDER BY` 子句（列名已由 models 白名单归一）。
fn order_clause(column: &'static str, desc: Option<bool>) -> String {
    match desc {
        Some(true) => format!(" ORDER BY {column} DESC"),
        Some(false) => format!(" ORDER BY {column} ASC"),
        None => String::new(),
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("会员消息不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_clause_matches_build_sort_order() {
        assert_eq!(
            order_clause("message_log_id", Some(true)),
            " ORDER BY message_log_id DESC"
        );
        assert_eq!(order_clause("send_time", Some(false)), " ORDER BY send_time ASC");
        assert_eq!(order_clause("message_title", None), "");
    }

    #[test]
    fn order_clause_only_uses_whitelisted_columns() {
        for column in nextrade_core::models::member::user_message_log::SORTABLE_COLUMNS {
            assert!(order_clause(column, Some(true)).contains(column));
        }
    }
}
