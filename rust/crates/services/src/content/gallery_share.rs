//! 上报审核服务（content::gallery_share，我方新增）：平台与门店相册互通。
//!
//! 论坛 B 类缓做项 #35（帖 687807 余子项）。口径（见本线 REPORT 第二节）：
//! - **提交**：仅门店账号，且只能提交 **本店** 素材（图片 `gallery_pic` /
//!   视频 `gallery_video_info`）；任一素材不属于本店 → 整批拒绝
//!   （`只能上报本店素材`），不做部分成功掩盖越权。同一素材已有待审核
//!   记录 → 该条跳过并计入 `skipped`（DB 部分唯一索引兜底）。
//! - **列表**：门店只看本店记录；平台看全部；供应商/未判定账号看空集
//!   （严格权限边界，不泄漏他店上报情况）。
//! - **审核**：仅平台账号。通过 = 把素材 **复制** 入平台库（新行
//!   `shop_id = 0`，落入指定平台分组，共用同一存储文件不重新上传），
//!   门店原件不动；驳回只回写状态与备注。已审记录不可重复审核
//!   （仓储 `status = 0` 条件更新兜底防并发）。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::models::content::gallery_share::{
    status_name, validate_review, validate_submit, GalleryShareListQuery, GalleryShareReviewDTO,
    GalleryShareSubmitDTO, GalleryShareSubmitResultVO, GalleryShareSubmitVO, ERR_ALREADY_PENDING,
    ERR_ALREADY_REVIEWED, ERR_ITEM_NOT_FOUND, ERR_NOT_OWN_ITEM, ERR_PLATFORM_CANNOT_SUBMIT,
    ERR_REVIEW_PLATFORM_ONLY, ERR_SUBMIT_NOT_FOUND, ERR_SUBMIT_SHOP_ONLY, ERR_TARGET_NOT_PLATFORM,
    MEDIA_IMAGE, MEDIA_VIDEO, STATUS_APPROVED, STATUS_PENDING, STATUS_REJECTED,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::content::gallery_share::{
    GalleryShareRepository, GalleryShareSubmitRow,
};
use nextrade_infrastructure::db::Repositories;

use crate::content::tenant_scope::TenantScope;

/// 上报审核服务。
pub struct GalleryShareService {
    repos: Arc<Repositories>,
}

impl GalleryShareService {
    /// 构造（与 content 域其余服务同签名）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &GalleryShareRepository {
        &self.repos.content.gallery_share
    }

    /// 门店提交素材上报（`POST /setting/galleryShare/submit`）。
    ///
    /// # Errors
    /// - 平台账号 → 「平台账号无需上报素材」；非门店账号 → 「仅门店账号可上报素材」
    /// - 入参非法 → models 校验文案
    /// - 素材不存在 → 「素材不存在」；非本店素材 → 「只能上报本店素材」（整批拒绝）
    pub async fn submit(
        &self,
        dto: &GalleryShareSubmitDTO,
        admin_id: i32,
    ) -> Result<GalleryShareSubmitResultVO> {
        let scope = TenantScope::current();
        if scope.is_platform() {
            return Err(Error::biz_msg(ERR_PLATFORM_CANNOT_SUBMIT));
        }
        let shop_id = scope
            .shop_id()
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::biz_msg(ERR_SUBMIT_SHOP_ONLY))?;
        let media_type = validate_submit(dto).map_err(Error::biz_msg)?;
        let now = now_seconds() as i32;
        let mut result = GalleryShareSubmitResultVO::default();
        for item_id in &dto.ids {
            let row = if media_type == MEDIA_IMAGE {
                let pic = self
                    .repos
                    .content
                    .gallery_pic
                    .find_pic(*item_id)
                    .await?
                    .ok_or_else(|| Error::biz_msg(ERR_ITEM_NOT_FOUND))?;
                if pic.shop_id != shop_id {
                    return Err(Error::biz_msg(ERR_NOT_OWN_ITEM));
                }
                GalleryShareSubmitRow {
                    shop_id,
                    media_type: media_type.to_string(),
                    source_gallery_id: pic.gallery_id,
                    source_item_id: pic.pic_id,
                    item_name: pic.pic_name.clone(),
                    item_url: pic.pic_url.clone(),
                    item_thumb: pic.pic_thumb.clone(),
                    status: STATUS_PENDING,
                    submit_admin_id: admin_id,
                    submit_time: now,
                    add_time: now,
                    ..Default::default()
                }
            } else {
                let info = self
                    .repos
                    .content
                    .gallery_video_info
                    .find_by_id(*item_id)
                    .await?
                    .ok_or_else(|| Error::biz_msg(ERR_ITEM_NOT_FOUND))?;
                if info.shop_id != shop_id {
                    return Err(Error::biz_msg(ERR_NOT_OWN_ITEM));
                }
                GalleryShareSubmitRow {
                    shop_id,
                    media_type: media_type.to_string(),
                    source_gallery_id: info.gallery_id,
                    source_item_id: info.id,
                    item_name: info.video_name.clone(),
                    item_url: info.video_url.clone(),
                    item_thumb: info
                        .video_cover
                        .clone()
                        .unwrap_or_else(|| info.video_first_frame.clone().unwrap_or_default()),
                    status: STATUS_PENDING,
                    submit_admin_id: admin_id,
                    submit_time: now,
                    add_time: now,
                    ..Default::default()
                }
            };
            match self.repo().insert(&row).await {
                Ok(_) => result.submitted += 1,
                Err(e) if e.to_string().contains(ERR_ALREADY_PENDING) => {
                    // 已有待审核记录：跳过该条，不中断整批
                    result.skipped += 1;
                }
                Err(e) => return Err(e),
            }
        }
        tracing::info!(
            "素材上报提交: shop={} type={} submitted={} skipped={}",
            shop_id,
            media_type,
            result.submitted,
            result.skipped
        );
        Ok(result)
    }

    /// 上报记录列表（`GET /setting/galleryShare/list`）。
    ///
    /// 平台看全部、门店只看本店、其余账号空集。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn list(
        &self,
        q: &GalleryShareListQuery,
    ) -> Result<PageResult<GalleryShareSubmitVO>> {
        let (page, size) = q.page_params();
        let scope = TenantScope::current();
        let shop_filter = if scope.is_platform() {
            None
        } else {
            match scope.shop_id().filter(|id| *id > 0) {
                Some(id) => Some(id),
                // 非平台且无门店身份：严格边界，返回空集
                None => {
                    return Ok(PageResult::new(
                        0,
                        Vec::new(),
                        &nextrade_core::types::PageQuery { page, page_size: size },
                    ));
                }
            }
        };
        let offset = (page - 1) * size;
        let (total, rows) = self
            .repo()
            .paginate(shop_filter, q.status, size, offset)
            .await?;
        let list = rows.iter().map(row_to_vo).collect();
        Ok(PageResult::new(
            total,
            list,
            &nextrade_core::types::PageQuery { page, page_size: size },
        ))
    }

    /// 平台审核上报（`POST /setting/galleryShare/review`）。
    ///
    /// 通过时复制素材入平台库并回填 `result_item_id`；复制在状态回写之前
    /// 完成，若回写时发现已被并发审核（受影响 0 行）则整次报错——复制出的
    /// 平台素材是一条可独立删除的普通平台素材，不产生脏关联。
    ///
    /// # Errors
    /// - 非平台账号 → 「仅平台账号可审核上报素材」
    /// - 记录不存在 → 「上报记录不存在」；已审核 → 「该记录已审核，不能重复审核」
    /// - 目标分组非平台分组 → 「目标分组必须是平台素材分组」
    /// - 来源素材已被删除 → 「素材不存在」
    pub async fn review(&self, dto: &GalleryShareReviewDTO, admin_id: i32) -> Result<()> {
        let scope = TenantScope::current();
        if !scope.is_platform() {
            return Err(Error::biz_msg(ERR_REVIEW_PLATFORM_ONLY));
        }
        let (id, approve, target_gallery_id, remark) =
            validate_review(dto).map_err(Error::biz_msg)?;
        let row = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz_msg(ERR_SUBMIT_NOT_FOUND))?;
        if row.status != STATUS_PENDING {
            return Err(Error::biz_msg(ERR_ALREADY_REVIEWED));
        }
        let now = now_seconds() as i32;
        let (status, result_item_id, target) = if approve {
            let new_id = self.copy_into_platform(&row, target_gallery_id).await?;
            (STATUS_APPROVED, new_id, target_gallery_id)
        } else {
            (STATUS_REJECTED, 0, 0)
        };
        let affected = self
            .repo()
            .update_review(id, status, admin_id, now, &remark, target, result_item_id)
            .await?;
        if affected == 0 {
            return Err(Error::biz_msg(ERR_ALREADY_REVIEWED));
        }
        tracing::info!(
            "素材上报审核: id={} status={} result_item={}",
            id,
            status,
            result_item_id
        );
        Ok(())
    }

    /// 审核通过的复制动作：按快照的来源素材现行行复制一份平台素材。
    /// 目标分组 > 0 时必须存在且为平台分组；= 0 表示平台未分组。
    async fn copy_into_platform(
        &self,
        row: &GalleryShareSubmitRow,
        target_gallery_id: i32,
    ) -> Result<i32> {
        if row.media_type == MEDIA_IMAGE {
            if target_gallery_id > 0 {
                let group = self
                    .repos
                    .content
                    .gallery
                    .find_by_id(target_gallery_id)
                    .await?
                    .ok_or_else(|| Error::biz_msg(ERR_TARGET_NOT_PLATFORM))?;
                if group.shop_id != 0 {
                    return Err(Error::biz_msg(ERR_TARGET_NOT_PLATFORM));
                }
            }
            let pic = self
                .repos
                .content
                .gallery_pic
                .find_pic(row.source_item_id)
                .await?
                .ok_or_else(|| Error::biz_msg(ERR_ITEM_NOT_FOUND))?;
            let mut copy = pic.clone();
            copy.pic_id = 0;
            copy.shop_id = 0;
            copy.gallery_id = target_gallery_id;
            copy.vendor_id = 0;
            copy.pic_ower_id = 0;
            self.repos.content.gallery_pic.insert_pic(&copy).await
        } else if row.media_type == MEDIA_VIDEO {
            if target_gallery_id > 0 {
                let group = self
                    .repos
                    .content
                    .gallery_video
                    .find_by_id(target_gallery_id)
                    .await?
                    .ok_or_else(|| Error::biz_msg(ERR_TARGET_NOT_PLATFORM))?;
                if group.shop_id != 0 {
                    return Err(Error::biz_msg(ERR_TARGET_NOT_PLATFORM));
                }
            }
            let info = self
                .repos
                .content
                .gallery_video_info
                .find_by_id(row.source_item_id)
                .await?
                .ok_or_else(|| Error::biz_msg(ERR_ITEM_NOT_FOUND))?;
            let mut copy = info.clone();
            copy.id = 0;
            copy.shop_id = 0;
            copy.gallery_id = target_gallery_id;
            copy.vendor_id = 0;
            copy.add_user_id = 0;
            self.repos.content.gallery_video_info.insert(&copy).await
        } else {
            Err(Error::biz_msg(ERR_ITEM_NOT_FOUND))
        }
    }
}

/// 仓储行 → VO（含状态文案）。
fn row_to_vo(row: &GalleryShareSubmitRow) -> GalleryShareSubmitVO {
    GalleryShareSubmitVO {
        id: row.id,
        shop_id: row.shop_id,
        media_type: row.media_type.clone(),
        source_gallery_id: row.source_gallery_id,
        source_item_id: row.source_item_id,
        item_name: row.item_name.clone(),
        item_url: row.item_url.clone(),
        item_thumb: row.item_thumb.clone(),
        status: row.status,
        status_name: status_name(row.status).to_string(),
        submit_time: row.submit_time,
        review_time: row.review_time,
        review_remark: row.review_remark.clone(),
        target_gallery_id: row.target_gallery_id,
        result_item_id: row.result_item_id,
    }
}
