//! 分销员分组（marketing::salesman_group）仓储。
//!
//! 表 `salesman_group`：`group_id` / `group_name` / `describe` / `add_time` /
//! `shop_id`（`python tools/schema_parser.py --table salesman_group`）。
//! 仅依赖 `salesman.group_id` 做删除前占用检查；无逻辑删除列。
//!
//! 查询统一运行时 `sqlx::query_as`。

use sqlx::PgPool;

use nextrade_core::entities::SalesmanGroup;
use nextrade_core::models::marketing::salesman_group::MSG_GROUP_NOT_FOUND;
use nextrade_core::{Error, Result};

/// `salesman_group` 固定 SELECT 列（5 列）。
///
/// ⚠️ `describe` 是 **MySQL 保留字**，必须加反引号；否则
/// `SELECT ... FROM salesman_group` 直接 1064 语法错误
/// （2026-10-21 线上 `/adminapi/salesman/group/list` 500 的根因）。
pub const GROUP_COLUMNS: &str = "group_id, group_name, \"describe\", add_time, shop_id";

/// 分组仓储。
#[derive(Clone)]
pub struct SalesmanGroupRepository {
    pool: PgPool,
}

impl SalesmanGroupRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 分页（Java `list`：`groupName` LIKE + `add_time ASC`；`buildSortOrder`
    /// 若显式传 `sortField` 则以白名单列覆盖排序）。
    ///
    /// `shop_id` 为租户过滤值：`None`（平台账号）不过滤，`Some(id)` 强制
    /// `WHERE shop_id = ?`（多租户隔离，见 `marketing::tenant`）。
    pub async fn paginate(
        &self,
        group_name: Option<&str>,
        shop_id: Option<i32>,
        sort_field: Option<&str>,
        sort_order: Option<&str>,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<SalesmanGroup>)> {
        let like = group_name
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|k| format!("%{k}%"));
        let mut where_parts: Vec<&str> = Vec::new();
        if like.is_some() {
            where_parts.push("group_name LIKE ?");
        }
        if shop_id.is_some() {
            where_parts.push("shop_id = ?");
        }
        let where_sql = if where_parts.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_parts.join(" AND "))
        };
        let column = resolve_sort_column(sort_field);
        let dir = if sort_order.is_some_and(|o| o.eq_ignore_ascii_case("desc")) {
            "DESC"
        } else {
            "ASC"
        };
        let count_sql = format!("SELECT COUNT(*) FROM salesman_group {where_sql}");
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        if let Some(k) = like.as_deref() {
            cq = cq.bind(k);
        }
        if let Some(id) = shop_id {
            cq = cq.bind(id);
        }
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let list_sql = format!(
            "SELECT {GROUP_COLUMNS} FROM salesman_group {where_sql} ORDER BY {column} {dir} LIMIT ? OFFSET ?"
        );
        let mut lq = sqlx::query_as::<_, SalesmanGroup>(&list_sql);
        if let Some(k) = like.as_deref() {
            lq = lq.bind(k);
        }
        if let Some(id) = shop_id {
            lq = lq.bind(id);
        }
        let rows = lq
            .bind(limit.max(1) as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 按主键查询（Java `getById`）。
    pub async fn find_by_id(&self, group_id: i32) -> Result<Option<SalesmanGroup>> {
        let sql = format!("SELECT {GROUP_COLUMNS} FROM salesman_group WHERE group_id = $1 LIMIT 1");
        sqlx::query_as::<_, SalesmanGroup>(&sql)
            .bind(group_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 新增，返回主键。
    pub async fn insert(&self, g: &SalesmanGroup) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO salesman_group (group_name, \"describe\", add_time, shop_id) VALUES ($1, $2, $3, $4) RETURNING group_id",
        )
        .bind(&g.group_name)
        .bind(&g.describe)
        .bind(g.add_time)
        .bind(g.shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 整行更新。
    pub async fn update(&self, g: &SalesmanGroup) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE salesman_group SET group_name = $1, \"describe\" = $2, add_time = $3, shop_id = $4 WHERE group_id = $5",
        )
        .bind(&g.group_name)
        .bind(&g.describe)
        .bind(g.add_time)
        .bind(g.shop_id)
        .bind(g.group_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 分组下的分销员数（Java `del` 前置检查 `salesman.group_id`）。
    pub async fn count_salesmen_in_group(&self, group_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM salesman WHERE group_id = $1")
            .bind(group_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0)
    }

    /// 物理删除（Java `removeById`；表无 is_delete）。
    pub async fn delete(&self, group_id: i32) -> Result<bool> {
        let res = sqlx::query("DELETE FROM salesman_group WHERE group_id = $1")
            .bind(group_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }
}

/// 排序字段白名单（`salesman_group` 实存列；未知回退 `add_time`，与 Java 一致）。
#[must_use]
pub fn resolve_sort_column(field: Option<&str>) -> &'static str {
    match field {
        Some("groupId") | Some("group_id") => "group_id",
        Some("addTime") | Some("add_time") => "add_time",
        Some("groupName") | Some("group_name") => "group_name",
        _ => "add_time",
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound(MSG_GROUP_NOT_FOUND.into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_cover_entity() {
        assert_eq!(GROUP_COLUMNS.split(", ").count(), 5);
        assert!(!GROUP_COLUMNS.contains('*'));
    }

    #[test]
    fn sort_default_is_add_time_asc() {
        assert_eq!(resolve_sort_column(None), "add_time");
        assert_eq!(resolve_sort_column(Some("groupId")), "group_id");
        assert_eq!(
            resolve_sort_column(Some("1;DROP TABLE salesman_group")),
            "add_time"
        );
    }
}
