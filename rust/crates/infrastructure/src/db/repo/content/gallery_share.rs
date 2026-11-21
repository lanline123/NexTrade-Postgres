//! 上报审核仓储（content::gallery_share，我方新增）：`gallery_share_submit`。
//!
//! 门店素材上报平台审核的数据访问：提交落库（待审核唯一索引兜底防重复）、
//! 门店自查列表、平台审核列表、审核回写。审核通过后的「复制入平台库」由
//! 服务层调用 `gallery_pic` / `gallery_video_info` 既有仓储完成，本仓储不越界。

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

use nextrade_core::{Error, Result};

/// `gallery_share_submit` 行（与迁移 026 的表结构逐字对齐）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct GalleryShareSubmitRow {
    /// 主键。
    pub id: i32,
    /// 上报门店。
    pub shop_id: i32,
    /// 媒体类型：image / video。
    pub media_type: String,
    /// 来源分组 id（快照）。
    pub source_gallery_id: i32,
    /// 来源素材 id（快照）。
    pub source_item_id: i32,
    /// 素材名（快照）。
    pub item_name: String,
    /// 素材地址（快照）。
    pub item_url: String,
    /// 缩略图/封面（快照）。
    pub item_thumb: String,
    /// 状态：0 待审核 / 1 已通过 / 2 已驳回。
    pub status: i32,
    /// 提交人管理员 id。
    pub submit_admin_id: i32,
    /// 提交时间（秒级时间戳）。
    pub submit_time: i32,
    /// 审核人管理员 id。
    pub review_admin_id: i32,
    /// 审核时间（秒级时间戳）。
    pub review_time: i32,
    /// 审核备注。
    pub review_remark: String,
    /// 通过时落入的平台分组 id。
    pub target_gallery_id: i32,
    /// 通过后生成的平台素材 id。
    pub result_item_id: i32,
    /// 记录创建时间（秒级时间戳）。
    pub add_time: i32,
}

/// 上报审核仓储。
#[derive(Clone)]
pub struct GalleryShareRepository {
    pool: PgPool,
}

impl GalleryShareRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 插入一条待审核记录。
    ///
    /// # Errors
    /// 命中待审核唯一索引（同一素材已有待审核记录）→ `Error::UniqueViolation`
    /// 语义的业务错误由服务层转「已在审核中」跳过计数；其余 DB 错误直传。
    pub async fn insert(&self, row: &GalleryShareSubmitRow) -> Result<i32> {
        let res: std::result::Result<i32, sqlx::Error> = sqlx::query_scalar(
            "INSERT INTO gallery_share_submit
               (shop_id, media_type, source_gallery_id, source_item_id,
                item_name, item_url, item_thumb, status,
                submit_admin_id, submit_time, review_admin_id, review_time,
                review_remark, target_gallery_id, result_item_id, add_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
             RETURNING id",
        )
        .bind(row.shop_id)
        .bind(&row.media_type)
        .bind(row.source_gallery_id)
        .bind(row.source_item_id)
        .bind(&row.item_name)
        .bind(&row.item_url)
        .bind(&row.item_thumb)
        .bind(row.status)
        .bind(row.submit_admin_id)
        .bind(row.submit_time)
        .bind(row.review_admin_id)
        .bind(row.review_time)
        .bind(&row.review_remark)
        .bind(row.target_gallery_id)
        .bind(row.result_item_id)
        .bind(row.add_time)
        .fetch_one(&self.pool)
        .await;
        match res {
            Ok(id) => Ok(id),
            Err(e) => {
                if let sqlx::Error::Database(db) = &e {
                    if db.kind() == sqlx::error::ErrorKind::UniqueViolation {
                        return Err(Error::biz_msg(
                            nextrade_core::models::content::gallery_share::ERR_ALREADY_PENDING,
                        ));
                    }
                }
                Err(map_err(e))
            }
        }
    }

    /// 按主键查记录。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<GalleryShareSubmitRow>> {
        sqlx::query_as::<_, GalleryShareSubmitRow>(
            "SELECT * FROM gallery_share_submit WHERE id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 分页列表：`shop_id = Some(n)` 只看该店；`None` 平台看全部。
    /// `status` 缺省不过滤。按 id 倒序（最新在前）。
    pub async fn paginate(
        &self,
        shop_id: Option<i32>,
        status: Option<i32>,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<GalleryShareSubmitRow>)> {
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM gallery_share_submit
             WHERE ($1::int IS NULL OR shop_id = $1)
               AND ($2::int IS NULL OR status = $2)",
        )
        .bind(shop_id)
        .bind(status)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let rows = sqlx::query_as::<_, GalleryShareSubmitRow>(
            "SELECT * FROM gallery_share_submit
             WHERE ($1::int IS NULL OR shop_id = $1)
               AND ($2::int IS NULL OR status = $2)
             ORDER BY id DESC LIMIT $3 OFFSET $4",
        )
        .bind(shop_id)
        .bind(status)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 审核回写（仅待审核记录可被回写，`status = 0` 条件兜底防并发重复审核）。
    /// 返回受影响行数（0 = 已被他人审核）。
    #[allow(clippy::too_many_arguments)]
    pub async fn update_review(
        &self,
        id: i32,
        status: i32,
        review_admin_id: i32,
        review_time: i32,
        review_remark: &str,
        target_gallery_id: i32,
        result_item_id: i32,
    ) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE gallery_share_submit
             SET status = $1, review_admin_id = $2, review_time = $3,
                 review_remark = $4, target_gallery_id = $5, result_item_id = $6
             WHERE id = $7 AND status = 0",
        )
        .bind(status)
        .bind(review_admin_id)
        .bind(review_time)
        .bind(review_remark)
        .bind(target_gallery_id)
        .bind(result_item_id)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_defaults_are_pending() {
        let row = GalleryShareSubmitRow::default();
        assert_eq!(row.status, 0);
        assert_eq!(row.media_type, "");
    }
}
