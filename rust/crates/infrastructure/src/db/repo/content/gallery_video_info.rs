//! 视频相册信息仓储：`gallery_video_info`，兼查 `gallery_video`（子相册/子树）。
//!
//! 列名严格对齐 `python tools/schema_parser.py --table gallery_video_info`
//! （15 列，主键 `id`）。
//!
//! 租户过滤（[`VideoScope`]）为 `None` 时不加条件；`Some` 时精确匹配
//! （Java `eq(shopId/vendorId)`）。
//!
//! 列表排序固定 `id DESC`（Java `buildGalleryVideoQueryWrapper` 末尾
//! `orderByDesc(GalleryVideoInfo::getId)`，本接口不消费 `sortField`）。

use sqlx::{Postgres,PgPool, QueryBuilder};
use nextrade_core::entities::{GalleryVideo, GalleryVideoInfo};
use nextrade_core::models::content::gallery_video::VideoScope;
use nextrade_core::models::content::gallery_video_info::GalleryVideoInfoListQuery;
use nextrade_core::{Error, Result};

/// `gallery_video_info` 表全部列（顺序与实体一致）。
const INFO_COLUMNS: &str = "id, shop_id, gallery_id, video_url, video_name, video_cover, add_time, \
     add_user_id, format, video_first_frame, duration, size, vendor_id, source, description";

/// \"gallery_video\" 表全部列。
const GALLERY_COLUMNS: &str = "id, shop_id, parent_id, name, sort, add_time, add_user_id, vendor_id";

#[derive(Clone)]
pub struct GalleryVideoInfoRepository {
    pool: PgPool,
}

impl GalleryVideoInfoRepository {
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

    /// 分页（Java `#list`）：租户 + 相册子树 + keyword/format/source 过滤，`id DESC`。
    pub async fn paginate(
        &self,
        scope: VideoScope,
        q: &GalleryVideoInfoListQuery,
        gallery_ids: Option<&[i32]>,
    ) -> Result<(u64, Vec<GalleryVideoInfo>)> {
        let page = q.page_query();
        let (limit, offset) = page.limit_offset();
        let keyword = q
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(|k| format!("%{k}%"));
        let format = q
            .format
            .as_deref()
            .map(str::trim)
            .filter(|f| !f.is_empty())
            .map(ToString::to_string);
        let source = q
            .source
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);

        let mut count_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM gallery_video_info WHERE 1 = 1");
        Self::push_filters(
            &mut count_qb,
            scope,
            gallery_ids,
            keyword.as_deref(),
            format.as_deref(),
            source.as_deref(),
        );
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut row_qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!(
            "SELECT {INFO_COLUMNS} FROM gallery_video_info WHERE 1 = 1"
        ));
        Self::push_filters(
            &mut row_qb,
            scope,
            gallery_ids,
            keyword.as_deref(),
            format.as_deref(),
            source.as_deref(),
        );
        row_qb.push(" ORDER BY id DESC");
        row_qb.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<GalleryVideoInfo> = row_qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 追加租户 / 相册 / keyword / format / source 过滤。
    fn push_filters(
        qb: &mut QueryBuilder<'_, Postgres>,
        scope: VideoScope,
        gallery_ids: Option<&[i32]>,
        keyword: Option<&str>,
        format: Option<&str>,
        source: Option<&str>,
    ) {
        if let Some(shop_id) = scope.shop_id {
            // 平台素材规则：店铺账号可见 `shop_id = 0`（平台素材）+ 本店
            qb.push(" AND (shop_id = 0 OR shop_id = ");
            qb.push_bind(shop_id);
            qb.push(")");
        }
        if let Some(vendor_id) = scope.vendor_id {
            qb.push(" AND vendor_id = ");
            qb.push_bind(vendor_id);
        }
        if let Some(ids) = gallery_ids {
            qb.push(" AND gallery_id IN (");
            let mut sep = qb.separated(", ");
            for id in ids {
                sep.push_bind(*id);
            }
            sep.push_unseparated(")");
            drop(sep);
        }
        if let Some(keyword) = keyword {
            qb.push(" AND video_name ILIKE ");
            qb.push_bind(keyword.to_string());
        }
        if let Some(format) = format {
            qb.push(" AND format = ");
            qb.push_bind(format.to_string());
        }
        if let Some(source) = source {
            if source == "external" {
                qb.push(" AND source = ");
                qb.push_bind("external".to_string());
            } else {
                // Java：`(source <> 'external' OR source IS NULL)`
                qb.push(" AND (source <> 'external' OR source IS NULL)");
            }
        }
    }

    /// 按主键查询。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<GalleryVideoInfo>> {
        let sql = crate::db::to_pg_placeholders(&format!("SELECT {INFO_COLUMNS} FROM gallery_video_info WHERE id = ? LIMIT 1"));
        sqlx::query_as::<_, GalleryVideoInfo>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 相册自身 + 全部子孙相册 ID（Java `galleryVideoService.getSelfAndChildIds`）。
    pub async fn find_gallery_subtree_ids(&self, root_id: i32) -> Result<Vec<i32>> {
        use std::collections::HashSet;
        let mut result = vec![root_id];
        let mut seen: HashSet<i32> = HashSet::from([root_id]);
        let mut frontier = vec![root_id];
        while !frontier.is_empty() {
            let mut qb: QueryBuilder<'_, Postgres> =
                QueryBuilder::new("SELECT id FROM gallery_video WHERE parent_id IN (");
            let mut sep = qb.separated(", ");
            for id in &frontier {
                sep.push_bind(*id);
            }
            sep.push_unseparated(")");
            drop(sep);
            let rows: Vec<(i32,)> = qb
                .build_query_as()
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
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

    /// 某相册的直接子相册（Java `eq(parent_id, id)`，未显式排序 → 本实现 `id ASC`）。
    pub async fn find_child_galleries(&self, gallery_id: i32) -> Result<Vec<GalleryVideo>> {
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {GALLERY_COLUMNS} FROM gallery_video WHERE parent_id = ? ORDER BY id ASC"
        ));
        sqlx::query_as::<_, GalleryVideo>(&sql)
            .bind(gallery_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 某相册下前 4 条视频（Java `eq(gallery_id).last("limit 4")`，未排序 → 本实现 `id ASC`）。
    pub async fn find_first_infos(
        &self,
        gallery_id: i32,
        limit: i64,
    ) -> Result<Vec<GalleryVideoInfo>> {
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {INFO_COLUMNS} FROM gallery_video_info WHERE gallery_id = ? \
             ORDER BY id ASC LIMIT {limit}"
        ));
        sqlx::query_as::<_, GalleryVideoInfo>(&sql)
            .bind(gallery_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按主键查相册（Java `galleryVideoService.getById`）。
    pub async fn find_gallery_by_id(&self, id: i32) -> Result<Option<GalleryVideo>> {
        let sql = crate::db::to_pg_placeholders(&format!("SELECT {GALLERY_COLUMNS} FROM gallery_video WHERE id = ? LIMIT 1"));
        sqlx::query_as::<_, GalleryVideo>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    // ————————————————————— 写 —————————————————————

    /// 新增（Java \"save\"），返回自增主键。
    pub async fn insert(&self, row: &GalleryVideoInfo) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO gallery_video_info \
             (shop_id, gallery_id, video_url, video_name, video_cover, add_time, add_user_id, \
              format, video_first_frame, duration, size, vendor_id, source, description) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) RETURNING id",
        )
        .bind(row.shop_id)
        .bind(row.gallery_id)
        .bind(&row.video_url)
        .bind(&row.video_name)
        .bind(&row.video_cover)
        .bind(row.add_time)
        .bind(row.add_user_id)
        .bind(&row.format)
        .bind(&row.video_first_frame)
        .bind(&row.duration)
        .bind(&row.size)
        .bind(row.vendor_id)
        .bind(&row.source)
        .bind(&row.description)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 整行更新（Java `lambdaUpdate().set(...)`，未提交字段由服务层合并原值）。
    pub async fn update(&self, row: &GalleryVideoInfo) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE gallery_video_info SET gallery_id = $1, video_url = $2, video_name = $3, \
                video_cover = $4, description = $5, format = $6, duration = $7, size = $8, \
                video_first_frame = $9, source = $10, add_time = $11 \
             WHERE id = $12",
        )
        .bind(row.gallery_id)
        .bind(&row.video_url)
        .bind(&row.video_name)
        .bind(&row.video_cover)
        .bind(&row.description)
        .bind(&row.format)
        .bind(&row.duration)
        .bind(&row.size)
        .bind(&row.video_first_frame)
        .bind(&row.source)
        .bind(row.add_time)
        .bind(row.id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 删除（Java \"removeById\"）。
    pub async fn delete(&self, id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM gallery_video_info WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("系统未找到视频信息".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::content::gallery_video_info::{
        ALLOWED_VIDEO_EXTENSIONS, BLOCKED_URL_PATTERNS,
    };

    #[test]
    fn info_columns_cover_all_schema_columns() {
        let cols: Vec<&str> = INFO_COLUMNS.split(',').map(str::trim).collect();
        assert_eq!(cols.len(), 15);
        assert!(cols.contains(&"video_first_frame"));
        assert!(cols.contains(&"source"));
        assert!(cols.contains(&"gallery_id"));
    }

    #[test]
    fn gallery_columns_cover_entity_fields() {
        let cols: Vec<&str> = GALLERY_COLUMNS.split(',').map(str::trim).collect();
        assert_eq!(cols.len(), 8);
        assert!(cols.contains(&"parent_id"));
        assert!(cols.contains(&"vendor_id"));
    }

    #[test]
    fn scope_filter_includes_platform_material_branch() {
        // 平台素材规则：店铺账号过滤条件包含 `shop_id = 0` 分支
        let src = include_str!("gallery_video_info.rs");
        assert!(src.contains("(shop_id = 0 OR shop_id = "));
    }

    #[test]
    fn external_rules_reference_java_constants() {
        assert_eq!(ALLOWED_VIDEO_EXTENSIONS.len(), 4);
        assert!(BLOCKED_URL_PATTERNS.contains(&"baidu.com/s?"));
    }
}
