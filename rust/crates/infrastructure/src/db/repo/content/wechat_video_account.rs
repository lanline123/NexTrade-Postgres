//! 视频号账号仓储：`wechat_video_account`。
//!
//! 列名严格对齐 `python tools/schema_parser.py --table wechat_video_account`：
//! `id` / `finder_id` / `add_time` / `update_time` / `shop_id`。
//!
//! 列表（Java `buildQueryWrapper`）：`finder_id LIKE`、`add_time >= / <=`、
//! `shop_id`（仅当 `Some(>0)`；Java `getShopId()!=null && >0`），`id DESC`。

use sqlx::{Postgres,PgPool, QueryBuilder};
use nextrade_core::entities::WechatVideoAccount;
use nextrade_core::models::content::wechat_video_account::{
    parse_end_time, parse_start_time, resolve_sort, WechatVideoAccountListQuery,
};
use nextrade_core::{Error, Result};

/// 表全部列。
const ACCOUNT_COLUMNS: &str = "id, finder_id, add_time, update_time, shop_id";

#[derive(Clone)]
pub struct WechatVideoAccountRepository {
    pool: PgPool,
}

impl WechatVideoAccountRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 分页（Java `#list`）。
    pub async fn paginate(
        &self,
        shop_scope: Option<i32>,
        q: &WechatVideoAccountListQuery,
    ) -> Result<(u64, Vec<WechatVideoAccount>)> {
        let page = q.page_query();
        let (limit, offset) = page.limit_offset();
        let (column, desc) = resolve_sort(q.sort_field.as_deref(), q.sort_order.as_deref());
        let order = order_clause(column, desc);
        let finder = q
            .finder_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{s}%"));
        let start = q.relation_time_start.as_deref().and_then(parse_start_time);
        let end = q.relation_time_end.as_deref().and_then(parse_end_time);
        let shop = shop_scope.filter(|s| *s > 0);

        let mut count_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM wechat_video_account WHERE 1 = 1");
        Self::push_filters(&mut count_qb, shop, finder.as_deref(), start, end);
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut row_qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!(
            "SELECT {ACCOUNT_COLUMNS} FROM wechat_video_account WHERE 1 = 1"
        ));
        Self::push_filters(&mut row_qb, shop, finder.as_deref(), start, end);
        row_qb.push(order);
        row_qb.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<WechatVideoAccount> = row_qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    fn push_filters(
        qb: &mut QueryBuilder<'_, Postgres>,
        shop: Option<i32>,
        finder: Option<&str>,
        start: Option<i64>,
        end: Option<i64>,
    ) {
        if let Some(finder) = finder {
            qb.push(" AND finder_id LIKE ");
            qb.push_bind(finder.to_string());
        }
        if let Some(start) = start {
            qb.push(" AND add_time >= ");
            qb.push_bind(start);
        }
        if let Some(end) = end {
            qb.push(" AND add_time <= ");
            qb.push_bind(end);
        }
        if let Some(shop) = shop {
            qb.push(" AND shop_id = ");
            qb.push_bind(shop);
        }
    }

    /// 按主键查询。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<WechatVideoAccount>> {
        let sql = format!("SELECT {ACCOUNT_COLUMNS} FROM wechat_video_account WHERE id = $1 LIMIT 1");
        sqlx::query_as::<_, WechatVideoAccount>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 视频号 ID 是否已关联（Java `checkFinderIdUnique`，同店铺范围 + 排除自身）。
    pub async fn finder_exists(
        &self,
        shop_scope: Option<i32>,
        finder_id: &str,
        exclude_id: Option<i32>,
    ) -> Result<bool> {
        let shop = shop_scope.filter(|s| *s > 0);
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM wechat_video_account WHERE finder_id = ");
        qb.push_bind(finder_id.to_string());
        if let Some(shop) = shop {
            qb.push(" AND shop_id = ");
            qb.push_bind(shop);
        }
        if let Some(exclude_id) = exclude_id {
            qb.push(" AND id <> ");
            qb.push_bind(exclude_id);
        }
        let row: (i64,) = qb.build_query_as().fetch_one(&self.pool).await.map_err(map_err)?;
        Ok(row.0 > 0)
    }

    /// 新增（Java `save`），返回自增主键。
    pub async fn insert(&self, row: &WechatVideoAccount) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO wechat_video_account (finder_id, add_time, update_time, shop_id) \
             VALUES (?, ?, ?, ?) RETURNING id",
        )
        .bind(&row.finder_id)
        .bind(row.add_time)
        .bind(row.update_time)
        .bind(row.shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 更新视频号 ID 与更新时间（Java \"updateById\"）。
    pub async fn update_finder(&self, id: i32, finder_id: &str, update_time: i64) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE wechat_video_account SET finder_id = ?, update_time = ? WHERE id = ?",
        )
        .bind(finder_id)
        .bind(update_time)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 删除（Java \"removeById\"）。
    pub async fn delete(&self, id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM wechat_video_account WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
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
        sqlx::Error::RowNotFound => Error::NotFound("视频号账号不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_match_entity_fields() {
        let cols: Vec<&str> = ACCOUNT_COLUMNS.split(',').map(str::trim).collect();
        assert_eq!(cols, vec!["id", "finder_id", "add_time", "update_time", "shop_id"]);
    }

    #[test]
    fn order_clause_shape() {
        assert_eq!(order_clause("id", Some(true)), " ORDER BY id DESC");
        assert_eq!(order_clause("add_time", Some(false)), " ORDER BY add_time ASC");
        assert_eq!(order_clause("finder_id", None), "");
    }
}
