//! 相册图片域仓储（content::gallery_pic）。
//!
//! 涉及表（列名严格对齐 `schema_parser`）：
//! - `gallery_pic`（相册图片，本域主表）/ `gallery`（相册，子树与子相册查询）
//! - `gallery_video_info`（视频素材，`transfer`/`batchDel` 的 `type=video` 分支）
//! - `config`（图片上传大小上限）
//!
//! 查询统一使用运行时 `sqlx::query_as`（**不使用** `query!` 宏）。
//! 多租户条件用 `(? IS NULL OR shop_id = ?) AND (? IS NULL OR vendor_id = ?)`
//! 形态保留 Java `eq(shopId != null, ..)` 语义，避免动态拼接注入面。

use sqlx::PgPool;

use nextrade_core::entities::{Gallery, GalleryPic, GalleryVideoInfo};
use nextrade_core::models::content::gallery_pic::GalleryPicListQuery;
use nextrade_core::{Error, Result};

/// 多租户过滤条件（平台素材规则）：
/// 店铺账号 `(shop_id = 0 OR shop_id = ?)`（平台素材可读 + 本店）；
/// 平台账号 `None` → 不加店铺条件。`vendor_id` 仍按 `None` 不加条件。
/// 占位符顺序不变：`(shop_id, shop_id, vendor_id, vendor_id)`。
const SCOPE_WHERE: &str =
    "(? IS NULL OR shop_id = 0 OR shop_id = ?) AND (? IS NULL OR vendor_id = ?)";

/// 相册图片域仓储（持有连接池）。
#[derive(Clone)]
pub struct GalleryPicRepository {
    pool: PgPool,
}

impl GalleryPicRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（与其它仓储一致）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— gallery_pic 读 —————————————————————

    /// 按主键查图片（Java `getById`，**不过滤 hidden/scope**）。
    pub async fn find_pic(&self, id: i32) -> Result<Option<GalleryPic>> {
        sqlx::query_as::<_, GalleryPic>("SELECT * FROM gallery_pic WHERE pic_id = $1 LIMIT 1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 相册前 N 张图片（Java `getPicsByGid`：`new Page<>(1, 4)`，无 scope/hidden 过滤）。
    pub async fn find_pics_by_gallery(&self, gallery_id: i32, limit: u64) -> Result<Vec<GalleryPic>> {
        sqlx::query_as::<_, GalleryPic>(
            "SELECT * FROM gallery_pic WHERE gallery_id = $1 LIMIT $2",
        )
        .bind(gallery_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按 id 集合查图片（Java `listByIds`，批量下载校验用）。
    pub async fn find_pics_by_ids(&self, ids: &[i32]) -> Result<Vec<GalleryPic>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = placeholders(ids.len());
        let sql = crate::db::to_pg_placeholders(&format!("SELECT * FROM gallery_pic WHERE pic_id IN ({placeholders})"));
        let mut q = sqlx::query_as::<_, GalleryPic>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 相册图片分页（Java `GalleryPicServiceImpl#list`）。
    ///
    /// 过滤：scope（shop/vendor）+ `gallery_id IN 子树` + 关键词/格式/来源 +
    /// `hidden = 0 OR hidden IS NULL`；排序由 `column/desc` 决定。
    pub async fn paginate(
        &self,
        shop_id: Option<i32>,
        vendor_id: Option<i32>,
        gallery_ids: Option<&[i32]>,
        q: &GalleryPicListQuery,
        column: &'static str,
        desc: bool,
    ) -> Result<(u64, Vec<GalleryPic>)> {
        let page = q.page_query();
        let (limit, offset) = page.limit_offset();
        let keyword = q.effective_keyword();
        let format = q.effective_format();
        let source = nextrade_core::models::content::gallery_pic::normalize_source_filter(
            q.source.as_deref(),
        );
        let gallery_ids = gallery_ids.unwrap_or(&[]);
        let gallery_filter = if gallery_ids.is_empty() {
            String::new()
        } else {
            format!(" AND gallery_id IN ({})", placeholders(gallery_ids.len()))
        };
        let where_sql = crate::db::to_pg_placeholders(&format!(
            " WHERE {SCOPE_WHERE}{gallery_filter} \
             AND (? IS NULL OR pic_name LIKE ?) \
             AND (? IS NULL OR file_type = ?) \
             AND (? IS NULL OR source = ?) \
             AND (hidden = 0 OR hidden IS NULL)"
        ));
        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM gallery_pic{where_sql}"));
        let mut count_q = sqlx::query_as::<_, (i64,)>(&count_sql)
            .bind(shop_id)
            .bind(shop_id)
            .bind(vendor_id)
            .bind(vendor_id);
        for id in gallery_ids {
            count_q = count_q.bind(*id);
        }
        let like = keyword.map(|kw| format!("%{kw}%"));
        let total: (i64,) = count_q
            .bind(like.clone())
            .bind(like.clone())
            .bind(format)
            .bind(format)
            .bind(source)
            .bind(source)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let order = format!(" ORDER BY {column}{}", if desc { " DESC" } else { " ASC" });
        let list_sql = crate::db::to_pg_placeholders(&format!("SELECT * FROM gallery_pic{where_sql}{order} LIMIT ? OFFSET ?"));
        let mut q_rows = sqlx::query_as::<_, GalleryPic>(&list_sql)
            .bind(shop_id)
            .bind(shop_id)
            .bind(vendor_id)
            .bind(vendor_id);
        for id in gallery_ids {
            q_rows = q_rows.bind(*id);
        }
        let rows = q_rows
            .bind(like.clone())
            .bind(like)
            .bind(format)
            .bind(format)
            .bind(source)
            .bind(source)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    // ————————————————————— gallery 读 —————————————————————

    /// 按主键查相册（Java `galleryService.detail`）。
    pub async fn find_gallery(&self, gallery_id: i32) -> Result<Option<Gallery>> {
        sqlx::query_as::<_, Gallery>("SELECT * FROM gallery WHERE gallery_id = $1 LIMIT 1")
            .bind(gallery_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 直接子相册（Java `galleryService.getGalleryByPid`：仅 `parent_id = ?`，
    /// 不含 scope 过滤；按 `gallery_sort ASC, gallery_id ASC` 稳定输出）。
    pub async fn find_child_galleries(&self, parent_id: i32) -> Result<Vec<Gallery>> {
        sqlx::query_as::<_, Gallery>(
            "SELECT * FROM gallery WHERE parent_id = $1 ORDER BY gallery_sort ASC, gallery_id ASC",
        )
        .bind(parent_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 自身 + 全部子孙相册 id（Java `getSelfAndChildIds`，逐层递归）。
    pub async fn find_self_and_child_ids(&self, root_id: i32) -> Result<Vec<i32>> {
        let mut result = vec![root_id];
        let mut frontier = vec![root_id];
        while !frontier.is_empty() {
            let placeholders = placeholders(frontier.len());
            let sql = crate::db::to_pg_placeholders(&format!(
                "SELECT gallery_id FROM gallery WHERE parent_id IN ({placeholders})"
            ));
            let mut q = sqlx::query_as::<_, (i32,)>(&sql);
            for id in &frontier {
                q = q.bind(*id);
            }
            let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
            frontier = rows
                .into_iter()
                .map(|(id,)| id)
                .filter(|id| !result.contains(id))
                .collect();
            result.extend(frontier.iter().copied());
        }
        Ok(result)
    }

    // ————————————————————— 写操作 —————————————————————

    /// 插入图片（Java \"save\"），返回自增 id。
    pub async fn insert_pic(&self, pic: &GalleryPic) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO gallery_pic
               (shop_id, gallery_id, pic_ower_id, pic_url, pic_name, pic_thumb,
                add_time, vendor_id, hidden, pic_width, pic_height, pic_size,
                file_type, pic_sort, source)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15) RETURNING pic_id",
        )
        .bind(pic.shop_id)
        .bind(pic.gallery_id)
        .bind(pic.pic_ower_id)
        .bind(&pic.pic_url)
        .bind(&pic.pic_name)
        .bind(&pic.pic_thumb)
        .bind(&pic.add_time)
        .bind(pic.vendor_id)
        .bind(pic.hidden)
        .bind(pic.pic_width)
        .bind(pic.pic_height)
        .bind(pic.pic_size)
        .bind(&pic.file_type)
        .bind(pic.pic_sort)
        .bind(&pic.source)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 单字段更新（Java `BaseServiceImpl#updateField`：白名单列 + 字符串值）。
    pub async fn update_pic_field(&self, id: i32, column: &'static str, value: &str) -> Result<u64> {
        let sql = crate::db::to_pg_placeholders(&format!("UPDATE gallery_pic SET {column} = ? WHERE pic_id = ?"));
        let affected = sqlx::query(&sql)
            .bind(value)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected)
    }

    /// 按 id 删除图片（Java \"removeById\" / \"removeByIds\"）。
    pub async fn delete_pics(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let sql = format!(
            "DELETE FROM gallery_pic WHERE pic_id IN ({})",
            placeholders(ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.execute(&self.pool)
            .await
            .map(|r| r.rows_affected())
            .map_err(map_err)
    }

    /// 图片排序（Java `@Transactional sort`：按 ids 顺序写 `pic_sort = i + 1`）。
    pub async fn sort_pics(&self, ids: &[i32]) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        for (i, id) in ids.iter().enumerate() {
            sqlx::query("UPDATE gallery_pic SET pic_sort = $1 WHERE pic_id = $2")
                .bind(i32::try_from(i + 1).unwrap_or(i32::MAX))
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
        }
        tx.commit().await.map_err(map_err)?;
        Ok(())
    }

    /// 批量移动图片到目标相册（Java \"transfer\" 的 image 分支）。
    pub async fn transfer_pics(&self, ids: &[i32], target_gallery_id: i32) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        for id in ids {
            sqlx::query("UPDATE gallery_pic SET gallery_id = $1 WHERE pic_id = $2")
                .bind(target_gallery_id)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
        }
        tx.commit().await.map_err(map_err)?;
        Ok(())
    }

    /// 批量移动视频到目标相册（Java `transfer` 的 video 分支）。
    pub async fn transfer_videos(&self, ids: &[i32], target_gallery_id: i32) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        for id in ids {
            sqlx::query("UPDATE gallery_video_info SET gallery_id = $1 WHERE id = $2")
                .bind(target_gallery_id)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
        }
        tx.commit().await.map_err(map_err)?;
        Ok(())
    }

    /// 批量删除视频（Java `galleryVideoInfoMapper.deleteByIds`）。
    pub async fn delete_videos(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let sql = format!(
            "DELETE FROM gallery_video_info WHERE id IN ({})",
            placeholders(ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.execute(&self.pool)
            .await
            .map(|r| r.rows_affected())
            .map_err(map_err)
    }

    /// 批量查视频（Java \"selectBatchIds\"，批量下载校验用）。
    pub async fn find_videos_by_ids(&self, ids: &[i32]) -> Result<Vec<GalleryVideoInfo>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM gallery_video_info WHERE id IN ({})",
            placeholders(ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, GalleryVideoInfo>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 读取配置值（Java `ConfigService#getConfigVal`：缺失 → `None`）。
    pub async fn find_config_value(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1",
        )
        .bind(biz_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|(value,)| value))
    }
}

/// 生成 `?, ?, ?` 占位符串。
fn placeholders(count: usize) -> String {
    vec!["?"; count].join(", ")
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_shape() {
        assert_eq!(placeholders(1), "?");
        assert_eq!(placeholders(3), "?, ?, ?");
    }

    #[test]
    fn scope_where_has_four_placeholders() {
        assert_eq!(SCOPE_WHERE.matches('?').count(), 4);
        // 平台素材规则：店铺账号可见 `shop_id = 0` + 本店
        assert!(SCOPE_WHERE.contains("shop_id = 0"));
    }
}
