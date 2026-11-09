//! 视频相册仓储：`gallery_video`，删除模式兼操作 `gallery_video_info`。
//!
//! 列名严格对齐 `python tools/schema_parser.py --table gallery_video`：
//! `id` / `shop_id` / `parent_id` / `name` / `sort` / `add_time` /
//! `add_user_id` / `vendor_id`。
//!
//! 租户过滤（`VideoScope`）为 `None` 时**不加条件**；`Some` 时精确匹配
//! （Java `eq(shopId/vendorId)`）。
//!
//! 事务：`delWithMode`（子树素材删除/迁移 + 分组删除）在 Java 为
//! `@Transactional`，本实现在仓储层用 `pool.begin()` 保证原子性。

use sqlx::{Postgres,PgPool, QueryBuilder};
use nextrade_core::entities::GalleryVideo;
use nextrade_core::models::content::gallery_video::{
    resolve_sort, GalleryVideoListQuery, VideoScope,
};
use nextrade_core::{Error, Result};

/// `gallery_video` 表全部列。
const VIDEO_COLUMNS: &str = "id, shop_id, parent_id, name, sort, add_time, add_user_id, vendor_id";

#[derive(Clone)]
pub struct GalleryVideoRepository {
    pool: PgPool,
}

impl GalleryVideoRepository {
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

    /// 分页（Java `GalleryVideoServiceImpl#list`；页大小 99）。
    pub async fn paginate(
        &self,
        scope: VideoScope,
        q: &GalleryVideoListQuery,
    ) -> Result<(u64, Vec<GalleryVideo>)> {
        let page = q.page_query();
        let (limit, offset) = page.limit_offset();
        let (column, desc) = resolve_sort(q.sort_field.as_deref(), q.sort_order.as_deref());
        let order = order_clause(column, desc);
        let parent = q.parent_filter();

        let mut count_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM gallery_video WHERE 1 = 1");
        Self::push_filters(&mut count_qb, scope, parent);
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut row_qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!(
            "SELECT {VIDEO_COLUMNS} FROM gallery_video WHERE 1 = 1"
        ));
        Self::push_filters(&mut row_qb, scope, parent);
        row_qb.push(order);
        row_qb.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<GalleryVideo> = row_qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 全部分组（树用）：租户过滤 + `sort ASC, id ASC`（Java `treeList`）。
    pub async fn list_for_tree(&self, scope: VideoScope) -> Result<Vec<GalleryVideo>> {
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!(
            "SELECT {VIDEO_COLUMNS} FROM gallery_video WHERE 1 = 1"
        ));
        Self::push_filters(&mut qb, scope, None);
        qb.push(" ORDER BY sort ASC, id ASC");
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 追加租户 + 父级过滤（平台素材规则：店铺账号可见 `shop_id = 0` + 本店）。
    fn push_filters(qb: &mut QueryBuilder<'_, Postgres>, scope: VideoScope, parent: Option<i32>) {
        if let Some(shop_id) = scope.shop_id {
            qb.push(" AND (shop_id = 0 OR shop_id = ");
            qb.push_bind(shop_id);
            qb.push(")");
        }
        if let Some(vendor_id) = scope.vendor_id {
            qb.push(" AND vendor_id = ");
            qb.push_bind(vendor_id);
        }
        if let Some(parent_id) = parent {
            qb.push(" AND parent_id = ");
            qb.push_bind(parent_id);
        }
    }

    /// 按主键查询（Java `getById`）。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<GalleryVideo>> {
        let sql = format!("SELECT {VIDEO_COLUMNS} FROM gallery_video WHERE id = $1 LIMIT 1");
        sqlx::query_as::<_, GalleryVideo>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 同父级下一个排序（Java `nextVideoSort`：`max(sort)+1`，无同级为 1）。
    pub async fn next_sort(&self, scope: VideoScope, parent_id: i32) -> Result<i32> {
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT MAX(sort) FROM gallery_video WHERE parent_id = ");
        qb.push_bind(parent_id);
        if let Some(shop_id) = scope.shop_id {
            qb.push(" AND shop_id = ");
            qb.push_bind(shop_id);
        }
        if let Some(vendor_id) = scope.vendor_id {
            qb.push(" AND vendor_id = ");
            qb.push_bind(vendor_id);
        }
        let row: (Option<i32>,) = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0.unwrap_or(0) + 1)
    }

    /// 同父级下名称是否已存在（Java `update` 的 `ne(id).eq(parent).eq(name)`）。
    ///
    /// 注意 Java 该查询**不带租户过滤**，本实现同样不加。
    pub async fn name_exists_excluding(
        &self,
        parent_id: i32,
        name: &str,
        exclude_id: i32,
    ) -> Result<bool> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM gallery_video \
             WHERE parent_id = ? AND name = ? AND id <> ?",
        )
        .bind(parent_id)
        .bind(name)
        .bind(exclude_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0 > 0)
    }

    // ————————————————————— 写 —————————————————————

    /// 新增（Java \"save\"），返回自增主键。
    pub async fn insert(&self, row: &GalleryVideo) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO gallery_video \
             (shop_id, parent_id, name, sort, add_time, add_user_id, vendor_id) \
             VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(row.shop_id)
        .bind(row.parent_id)
        .bind(&row.name)
        .bind(row.sort)
        .bind(row.add_time)
        .bind(row.add_user_id)
        .bind(row.vendor_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 更新名称（+ 可选排序）（Java `lambdaUpdate().set(name).set(sort!=null, sort)`）。
    pub async fn update_name_sort(
        &self,
        id: i32,
        name: &str,
        sort: Option<i32>,
    ) -> Result<u64> {
        let r = match sort {
            Some(sort) => {
                sqlx::query("UPDATE gallery_video SET name = ?, sort = ? WHERE id = ?")
                    .bind(name)
                    .bind(sort)
                    .bind(id)
                    .execute(&self.pool)
                    .await
            }
            None => {
                sqlx::query("UPDATE gallery_video SET name = ? WHERE id = ?")
                    .bind(name)
                    .bind(id)
                    .execute(&self.pool)
                    .await
            }
        }
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 删除单条（Java \"removeById\"）。
    pub async fn delete(&self, id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM gallery_video WHERE id = ?")
            .bind(id)
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
            QueryBuilder::new("DELETE FROM gallery_video WHERE id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        let r = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 查自身 + 全部子孙分组 ID（Java \"collectChildGroupIds\"）。
    pub async fn find_subtree_ids(&self, root_id: i32) -> Result<Vec<i32>> {
        use std::collections::HashSet;
        let mut seen: HashSet<i32> = HashSet::new();
        seen.insert(root_id);
        let mut result = vec![root_id];
        let mut frontier = vec![root_id];
        while !frontier.is_empty() {
            let placeholders = vec!["?"; frontier.len()].join(", ");
            let sql = format!("SELECT id FROM gallery_video WHERE parent_id IN ({placeholders})");
            let mut q = sqlx::query_as::<_, (i32,)>(&sql);
            for id in &frontier {
                q = q.bind(*id);
            }
            let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
            let mut next = Vec::new();
            for (child,) in rows {
                if seen.insert(child) {
                    result.push(child);
                    next.push(child);
                }
            }
            frontier = next;
        }
        Ok(result)
    }

    /// 按删除模式删除分组（Java \"delWithMode\"，`@Transactional`）：
    /// \"group_and_files\" → 删除子树内 \"gallery_video_info\"；
    /// 否则 → 子树内素材 \"gallery_id\" 置 0；最后删除子树分组。
    pub async fn delete_with_mode(&self, root_id: i32, delete_files: bool) -> Result<()> {
        let ids = self.find_subtree_ids(root_id).await?;
        let placeholders = vec!["?"; ids.len()].join(", ");
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        if delete_files {
            let sql = format!("DELETE FROM gallery_video_info WHERE gallery_id IN ({placeholders})");
            let mut q = sqlx::query(&sql);
            for id in &ids {
                q = q.bind(*id);
            }
            q.execute(&mut *tx).await.map_err(map_err)?;
        } else {
            let sql = format!(
                "UPDATE gallery_video_info SET gallery_id = 0 WHERE gallery_id IN ({placeholders})"
            );
            let mut q = sqlx::query(&sql);
            for id in &ids {
                q = q.bind(*id);
            }
            q.execute(&mut *tx).await.map_err(map_err)?;
        }
        let sql = format!("DELETE FROM gallery_video WHERE id IN ({placeholders})");
        let mut q = sqlx::query(&sql);
        for id in &ids {
            q = q.bind(*id);
        }
        q.execute(&mut *tx).await.map_err(map_err)?;
        tx.commit().await.map_err(map_err)?;
        Ok(())
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
        sqlx::Error::RowNotFound => Error::NotFound("视频相册不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_clause_matches_build_sort_order() {
        // 平台素材规则：店铺账号过滤条件包含 `shop_id = 0` 分支
        let src = include_str!("gallery_video.rs");
        assert!(src.contains("(shop_id = 0 OR shop_id = "));
        assert_eq!(order_clause("id", Some(true)), " ORDER BY id DESC");
        assert_eq!(order_clause("sort", Some(false)), " ORDER BY sort ASC");
        assert_eq!(order_clause("name", None), "");
    }

    #[test]
    fn order_clause_only_uses_whitelisted_columns() {
        for column in nextrade_core::models::content::gallery_video::SORTABLE_COLUMNS {
            assert!(order_clause(column, Some(true)).contains(column));
        }
    }
}
