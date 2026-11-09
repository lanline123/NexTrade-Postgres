//! 相册域仓储：`gallery`（兼查 `gallery_pic` 做素材计数/迁移/级联删除）。
//!
//! 对应原 Java `GalleryMapper` / `GalleryPicMapper`（仅用到的部分）。
//!
//! 列名严格对齐 `java/database/pure.sql`（已用
//! `python tools/schema_parser.py --table gallery` 核对）：
//! - `gallery`：`gallery_id` / `parent_id` / `gallery_admin_id` / `gallery_name` /
//!   `gallery_sort` / `gallery_thumb` / `shop_id` / `vendor_id`
//! - `gallery_pic`：本域只用 `gallery_id`（计数、迁移、级联删除）
//!
//! 事务边界：Java `GalleryServiceImpl` 在 `sort` / `move` / `delWithMode` 上标了
//! `@Transactional`。本实现把这三个多语句操作收敛在仓储层、用 `pool.begin()`
//! 显式开事务（服务层只做校验与编排），且**不**在事务中做跨表业务分支。

use std::collections::{HashMap, HashSet};

use sqlx::PgPool;
use nextrade_core::entities::Gallery;
use nextrade_core::models::content::gallery::{resolve_sort, GalleryListQuery, GalleryScope};
use nextrade_core::{Error, Result};

/// `gallery` 表全部列（顺序与 [`Gallery`] 字段一致，避免 `SELECT *` 映射错位）。
const GALLERY_COLUMNS: &str = "gallery_id, parent_id, gallery_admin_id, gallery_name, \
     gallery_sort, gallery_thumb, shop_id, vendor_id";

/// 分页查询的公共过滤条件（多租户 + 分组）。
///
/// **平台素材规则**（content 素材）：`shop_id = 0` 的平台素材所有账号可读；
/// 店铺素材仅本店可见；平台账号不过滤（\"None\"）。
/// 即：店铺账号 → `(shop_id = 0 OR shop_id = ?)`；平台账号 → `? IS NULL` 恒真。
/// 绑定顺序仍为 `(shop_id, shop_id, vendor_id, vendor_id)`。
const SCOPE_WHERE: &str =
    "(? IS NULL OR shop_id = 0 OR shop_id = ?) AND (? IS NULL OR vendor_id = ?)";

#[derive(Clone)]
pub struct GalleryRepository {
    pool: PgPool,
}

impl GalleryRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层需要时复用；本域事务均在仓储内部完成）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 相册（gallery） —————————————————————

    /// 相册分页（Java `GalleryServiceImpl#list`）。
    ///
    /// 过滤顺序逐字对齐 Java：
    /// 1. \"shop_id\" / `vendor_id`（`None` 不加条件）；
    /// 2. `galleryId >= 0` → `parent_id = galleryId`（缺省 0，即根分组）；
    /// 3. 排序：`sortField` 缺省 `gallerySort`、`sortOrder` 缺省 `asc`；
    /// 4. 分页：页大小固定 99（Java `new Page<>(page, 99)`）。
    pub async fn paginate(
        &self,
        scope: GalleryScope,
        q: &GalleryListQuery,
    ) -> Result<(u64, Vec<Gallery>)> {
        let page = q.page_query();
        let (limit, offset) = page.limit_offset();
        let parent = q.parent_filter();
        let (column, desc) = resolve_sort(q.sort_field.as_deref(), q.sort_order.as_deref());
        let order_sql = order_by(column, desc);

        let count_sql = format!("SELECT COUNT(*) FROM gallery WHERE {SCOPE_WHERE} AND (? IS NULL OR parent_id = ?)");
        let total: (i64,) = sqlx::query_as(&count_sql)
            .bind(scope.shop_id)
            .bind(scope.shop_id)
            .bind(scope.vendor_id)
            .bind(scope.vendor_id)
            .bind(parent)
            .bind(parent)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let list_sql = format!(
            "SELECT {GALLERY_COLUMNS} FROM gallery
             WHERE {SCOPE_WHERE} AND (? IS NULL OR parent_id = ?){order_sql}
             LIMIT ? OFFSET ?"
        );
        let rows = sqlx::query_as::<_, Gallery>(&list_sql)
            .bind(scope.shop_id)
            .bind(scope.shop_id)
            .bind(scope.vendor_id)
            .bind(scope.vendor_id)
            .bind(parent)
            .bind(parent)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 按主键查询相册（Java `getById(id)`）。
    pub async fn find_by_id(&self, gallery_id: i32) -> Result<Option<Gallery>> {
        let sql = format!("SELECT {GALLERY_COLUMNS} FROM gallery WHERE gallery_id = ? LIMIT 1");
        sqlx::query_as::<_, Gallery>(&sql)
            .bind(gallery_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 全部分组（树用）：多租户过滤 + `gallery_sort ASC, gallery_id ASC`
    /// （Java `GalleryServiceImpl#tree` 的排序逐字一致）。
    pub async fn list_for_tree(&self, scope: GalleryScope) -> Result<Vec<Gallery>> {
        let sql = format!(
            "SELECT {GALLERY_COLUMNS} FROM gallery
             WHERE {SCOPE_WHERE}
             ORDER BY gallery_sort ASC, gallery_id ASC"
        );
        sqlx::query_as::<_, Gallery>(&sql)
            .bind(scope.shop_id)
            .bind(scope.shop_id)
            .bind(scope.vendor_id)
            .bind(scope.vendor_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 各分组的素材数量（\"gallery_pic\" 按 \"gallery_id\" 分组计数）。
    ///
    /// Java 对每个分组单独 `selectCount(eq(galleryId, ..))` 且**不**加店铺过滤；
    /// 这里合并成一条 `GROUP BY` 查询，数值语义一致。
    pub async fn count_pics_by_gallery(&self) -> Result<HashMap<i32, i64>> {
        let rows: Vec<(i32, i64)> =
            sqlx::query_as("SELECT gallery_id, COUNT(*) FROM gallery_pic GROUP BY gallery_id")
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows.into_iter().collect())
    }

    /// 同父级下下一个排序值（Java `nextGallerySort`：`max(gallery_sort) + 1`，无同级为 1）。
    pub async fn next_gallery_sort(&self, scope: GalleryScope, parent_id: i32) -> Result<i32> {
        let sql = format!(
            "SELECT MAX(gallery_sort) FROM gallery
             WHERE parent_id = ? AND {SCOPE_WHERE}"
        );
        let row: (Option<i32>,) = sqlx::query_as(&sql)
            .bind(parent_id)
            .bind(scope.shop_id)
            .bind(scope.shop_id)
            .bind(scope.vendor_id)
            .bind(scope.vendor_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0.unwrap_or(0) + 1)
    }

    /// 新增相册（Java \"save\"），返回自增主键。
    pub async fn insert(&self, g: &Gallery) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO gallery
               (parent_id, gallery_admin_id, gallery_name, gallery_sort, gallery_thumb, shop_id, vendor_id)
             VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING gallery_id",
        )
        .bind(g.parent_id)
        .bind(g.gallery_admin_id)
        .bind(&g.gallery_name)
        .bind(g.gallery_sort)
        .bind(&g.gallery_thumb)
        .bind(g.shop_id)
        .bind(g.vendor_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 按主键整行更新相册（Java \"updateById\"；非提交字段由服务层合并原值保证不变）。
    pub async fn update(&self, g: &Gallery) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE gallery SET parent_id = ?, gallery_admin_id = ?, gallery_name = ?,
                gallery_sort = ?, gallery_thumb = ?, shop_id = ?, vendor_id = ?
             WHERE gallery_id = ?",
        )
        .bind(g.parent_id)
        .bind(g.gallery_admin_id)
        .bind(&g.gallery_name)
        .bind(g.gallery_sort)
        .bind(&g.gallery_thumb)
        .bind(g.shop_id)
        .bind(g.vendor_id)
        .bind(g.gallery_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 按主键删除单个相册（Java `BaseServiceImpl#del` → \"removeById\"）。
    pub async fn delete(&self, gallery_id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM gallery WHERE gallery_id = ?")
            .bind(gallery_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 批量删除相册（Java `BaseServiceImpl#batch` → \"removeByIds\"）。
    pub async fn delete_many(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let sql = format!("DELETE FROM gallery WHERE gallery_id IN ({})", placeholders(ids.len()));
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(id);
        }
        let r = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 查自身 + 全部子孙分组 ID（Java \"getSelfAndChildIds\" / \"collectChildGroupIds\"）。
    ///
    /// 逐层 BFS（每层一条 \"IN\" 查询），并用 \"visited\" 防御脏数据成环。
    pub async fn find_subtree_ids(&self, root_id: i32) -> Result<Vec<i32>> {
        let mut seen: HashSet<i32> = HashSet::new();
        seen.insert(root_id);
        let mut result: Vec<i32> = vec![root_id];
        let mut frontier: Vec<i32> = vec![root_id];
        while !frontier.is_empty() {
            let sql = format!(
                "SELECT gallery_id FROM gallery WHERE parent_id IN ({})",
                placeholders(frontier.len())
            );
            let mut q = sqlx::query_as::<_, (i32,)>(&sql);
            for id in &frontier {
                q = q.bind(id);
            }
            let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
            let mut next = Vec::new();
            for (child_id,) in rows {
                if seen.insert(child_id) {
                    result.push(child_id);
                    next.push(child_id);
                }
            }
            frontier = next;
        }
        Ok(result)
    }

    // ————————————————————— 事务写操作 —————————————————————

    /// 视频数量按分组统计（\"gallery_video_info\"）。
    pub async fn count_videos_by_gallery(&self) -> Result<HashMap<i32, i64>> {
        let rows: Vec<(i32, i64)> = sqlx::query_as(
            "SELECT gallery_id, COUNT(*) FROM gallery_video_info GROUP BY gallery_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().collect())
    }

    /// 分组排序（Java `@Transactional sort`）：按 `ids` 顺序写 `gallery_sort = i + 1`。
    pub async fn sort_groups(&self, ids: &[i32]) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        for (i, id) in ids.iter().enumerate() {
            sqlx::query("UPDATE gallery SET gallery_sort = ? WHERE gallery_id = ?")
                .bind((i + 1) as i32)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
        }
        tx.commit().await.map_err(map_err)?;
        Ok(())
    }

    /// 分组转移（Java `@Transactional move`）：
    /// 1. 把来源分组子树内所有素材的 \"gallery_id\" 改为目标分组；
    /// 2. `delete_source == true`（`mode = move_and_delete`）时删除来源分组子树。
    ///
    /// 注意：Java 的子树快照在事务开启**之前**由服务层取得（\"collectChildGroupIds\"），
    /// 这里同样先取快照再开事务，避免事务内递归查询拉长锁持有时间。
    pub async fn move_groups(
        &self,
        source_id: i32,
        target_id: i32,
        delete_source: bool,
        video: bool,
    ) -> Result<()> {
        let ids = self.find_subtree_ids(source_id).await?;
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        let ph = placeholders(ids.len());

        let table = if video { "gallery_video_info" } else { "gallery_pic" };
        let move_sql = format!(
            "UPDATE {table} SET gallery_id = ? WHERE gallery_id IN ({ph})"
        );
        let mut mv = sqlx::query(&move_sql).bind(target_id);
        for id in &ids {
            mv = mv.bind(id);
        }
        mv.execute(&mut *tx).await.map_err(map_err)?;

        if delete_source {
            let del_sql = format!("DELETE FROM gallery WHERE gallery_id IN ({ph})");
            let mut del = sqlx::query(&del_sql);
            for id in &ids {
                del = del.bind(id);
            }
            del.execute(&mut *tx).await.map_err(map_err)?;
        }

        tx.commit().await.map_err(map_err)?;
        Ok(())
    }

    /// 按删除模式删除分组（Java `@Transactional delWithMode`）：
    /// - `delete_files = true`（\"group_and_files\"）→ 删除子树内全部素材行；
    /// - 否则（\"only_group\" 及其它值）→ 子树内素材 \"gallery_id\" 置 0（未分组）；
    /// - 最后删除子树全部分组。
    pub async fn delete_groups_with_mode(&self, root_id: i32, delete_files: bool) -> Result<()> {
        let ids = self.find_subtree_ids(root_id).await?;
        let ph = placeholders(ids.len());
        let mut tx = self.pool.begin().await.map_err(map_err)?;

        if delete_files {
            let sql = format!("DELETE FROM gallery_pic WHERE gallery_id IN ({ph})");
            let mut q = sqlx::query(&sql);
            for id in &ids {
                q = q.bind(id);
            }
            q.execute(&mut *tx).await.map_err(map_err)?;
        } else {
            let sql = format!("UPDATE gallery_pic SET gallery_id = 0 WHERE gallery_id IN ({ph})");
            let mut q = sqlx::query(&sql);
            for id in &ids {
                q = q.bind(id);
            }
            q.execute(&mut *tx).await.map_err(map_err)?;
        }

        let sql = format!("DELETE FROM gallery WHERE gallery_id IN ({ph})");
        let mut q = sqlx::query(&sql);
        for id in &ids {
            q = q.bind(id);
        }
        q.execute(&mut *tx).await.map_err(map_err)?;

        tx.commit().await.map_err(map_err)?;
        Ok(())
    }
}

/// 生成 `?, ?, ?` 占位符串（列名/表名均来自常量，不参与拼接）。
fn placeholders(n: usize) -> String {
    vec!["?"; n].join(", ")
}

/// 生成 `ORDER BY` 子句（`column` 已由 [`resolve_sort`] 白名单归一）。
fn order_by(column: &'static str, desc: Option<bool>) -> String {
    match desc {
        Some(true) => format!(" ORDER BY {column} DESC"),
        Some(false) => format!(" ORDER BY {column} ASC"),
        // Java：既非 asc 也非 desc 时不设置排序
        None => String::new(),
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("相册不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_renders_in_clause() {
        assert_eq!(placeholders(1), "?");
        assert_eq!(placeholders(3), "?, ?, ?");
    }

    #[test]
    fn scope_where_has_platform_material_rule() {
        // 平台素材规则：店铺账号 `(shop_id = 0 OR shop_id = ?)`；占位符仍为 4
        assert!(SCOPE_WHERE.contains("shop_id = 0"));
        assert_eq!(SCOPE_WHERE.matches('?').count(), 4);
    }

    #[test]
    fn order_by_matches_java_build_sort_order() {
        // 缺省 asc（Java sortOrder 为空 → asc）
        assert_eq!(order_by("gallery_sort", Some(false)), " ORDER BY gallery_sort ASC");
        assert_eq!(order_by("gallery_sort", Some(true)), " ORDER BY gallery_sort DESC");
        // 非 asc/desc → Java 不设置 ORDER BY
        assert_eq!(order_by("gallery_sort", None), "");
        // 列名只可能来自 resolve_sort 的白名单
        assert!(order_by("gallery_id", Some(true)).contains("gallery_id"));
    }
}
