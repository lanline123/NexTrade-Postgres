//! 友情链接仓储：`friend_links`。
//!
//! 对应原 Java `FriendLinksMapper`。列名严格对齐
//! `python tools/schema_parser.py --table friend_links`：
//! `link_id` / `link_title` / `link_logo` / `link_url` / `sort_order`。
//!
//! 列表：keyword 作用于 `link_title ILIKE`；排序由 Java `buildSortOrder` 归一
//! （缺省主键降序），列名来自 models 白名单，无注入面。

use sqlx::{Postgres,PgPool, QueryBuilder};
use nextrade_core::entities::FriendLinks;
use nextrade_core::models::content::friend_links::{
    resolve_sort, FriendLinksListQuery, FRIEND_LINKS_FIELDS,
};
use nextrade_core::{Error, Result};

/// `friend_links` 表全部列。
const LINK_COLUMNS: &str = "link_id, link_title, link_logo, link_url, sort_order";

#[derive(Clone)]
pub struct FriendLinksRepository {
    pool: PgPool,
}

impl FriendLinksRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 分页（Java `FriendLinksServiceImpl#list`）。
    pub async fn paginate(
        &self,
        q: &FriendLinksListQuery,
    ) -> Result<(u64, Vec<FriendLinks>)> {
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
            "SELECT COUNT(*) FROM friend_links WHERE ($1 IS NULL OR link_title ILIKE $2)";
        let total: (i64,) = sqlx::query_as(count_sql)
            .bind(&like)
            .bind(&like)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let list_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {LINK_COLUMNS} FROM friend_links \
             WHERE (? IS NULL OR link_title ILIKE ?){order} LIMIT ? OFFSET ?"
        ));
        let rows = sqlx::query_as::<_, FriendLinks>(&list_sql)
            .bind(&like)
            .bind(&like)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 按主键查询（Java `getById`）。
    pub async fn find_by_id(&self, link_id: i32) -> Result<Option<FriendLinks>> {
        let sql = crate::db::to_pg_placeholders(&format!("SELECT {LINK_COLUMNS} FROM friend_links WHERE link_id = ? LIMIT 1"));
        sqlx::query_as::<_, FriendLinks>(&sql)
            .bind(link_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 新增（Java \"save\"），返回自增主键。
    pub async fn insert(&self, row: &FriendLinks) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO friend_links (link_title, link_logo, link_url, sort_order) \
             VALUES ($1, $2, $3, $4) RETURNING link_id",
        )
        .bind(&row.link_title)
        .bind(&row.link_logo)
        .bind(&row.link_url)
        .bind(row.sort_order)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 整行更新（Java \"updateById\"；未提交字段由服务层合并原值）。
    pub async fn update(&self, row: &FriendLinks) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE friend_links SET link_title = $1, link_logo = $2, link_url = $3, sort_order = $4 \
             WHERE link_id = $5",
        )
        .bind(&row.link_title)
        .bind(&row.link_logo)
        .bind(&row.link_url)
        .bind(row.sort_order)
        .bind(row.link_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 更新单字段（Java `BaseServiceImpl#updateField`；列白名单兜底）。
    pub async fn update_field(
        &self,
        id: i32,
        column: &'static str,
        val: Option<&str>,
    ) -> Result<u64> {
        let safe = match column {
            "link_title" => "link_title",
            "sort_order" => "sort_order",
            _ => return Err(Error::Validation("field 的值有误".into())),
        };
        debug_assert!(FRIEND_LINKS_FIELDS.contains(&safe));
        let sql = crate::db::to_pg_placeholders(&format!("UPDATE friend_links SET {safe} = ? WHERE link_id = ?"));
        let r = sqlx::query(&sql)
            .bind(val)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 删除（Java \"removeById\"）。
    pub async fn delete(&self, link_id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM friend_links WHERE link_id = $1")
            .bind(link_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 批量删除（Java `BaseServiceImpl#batch`）。
    pub async fn delete_many(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("DELETE FROM friend_links WHERE link_id IN (");
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

/// `ORDER BY` 子句（列名来自 models 白名单）。
fn order_clause(column: &'static str, desc: Option<bool>) -> String {
    match desc {
        Some(true) => format!(" ORDER BY {column} DESC"),
        Some(false) => format!(" ORDER BY {column} ASC"),
        None => String::new(),
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("友情链接不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_clause_matches_build_sort_order() {
        assert_eq!(order_clause("link_id", Some(true)), " ORDER BY link_id DESC");
        assert_eq!(order_clause("sort_order", Some(false)), " ORDER BY sort_order ASC");
        assert_eq!(order_clause("link_title", None), "");
    }

    #[test]
    fn order_clause_only_uses_whitelisted_columns() {
        for column in nextrade_core::models::content::friend_links::SORTABLE_COLUMNS {
            assert!(order_clause(column, Some(true)).contains(column));
        }
    }
}
