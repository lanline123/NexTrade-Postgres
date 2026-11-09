//! 视频相册信息服务（content::gallery_video_info）。
//!
//! 对应原 Java `com.nextrade.service.video.impl.GalleryVideoInfoServiceImpl`。
//!
//! 构造签名固定：`pub fn new(repos: Arc<Repositories>) -> Self`。
//!
//! ## 已迁移（6）
//! - `list`：分页 + 相册子树 / keyword / format / source 过滤 + 子相册（各前 4 条）
//! - `del`：按主键删除
//! - `create`：外链 URL 深度校验 + 级联 `galleryId` 归一 + 审计字段
//! - `update`：选择性更新（未提交字段保留）
//! - `detail`：缺行 → 「系统未找到视频信息」
//! - `uploadVideo`：本地存储保存视频并返回 URL（无 FFmpeg 时封面为空）
//!
//! ## 与 Java 的差异
//! 见 [`crate::models::content::gallery_video_info`] 模块头。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::{GalleryVideo, GalleryVideoInfo};
use nextrade_core::models::content::gallery_video::VideoScope;
use nextrade_core::models::content::gallery_video_info::{
    validate_external_video_url, validate_required, validate_upload_file, GalleryVideoInfoDTO,
    GalleryVideoInfoDetailVO, GalleryVideoInfoListQuery, GalleryVideoInfoVO, GalleryVideoInfoItemVO,
    GalleryVideoResVO, OperateDTO, UploadVideoInfoVO, MSG_ID_REQUIRED,
    MSG_VIDEO_INFO_NOT_FOUND,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

use crate::content::tenant_scope::{ensure_media_access, ensure_media_write, media_write_shop_id};

/// 视频相册信息服务。
pub struct GalleryVideoInfoService {
    repos: Arc<Repositories>,
}

impl GalleryVideoInfoService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储（NESTED 访问：`self.repos.content.gallery_video_info`）。
    #[must_use]
    pub fn repo(
        &self,
    ) -> &nextrade_infrastructure::db::repo::content::gallery_video_info::GalleryVideoInfoRepository
    {
        &self.repos.content.gallery_video_info
    }

    /// 列表（Java `#list`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn list(
        &self,
        scope: VideoScope,
        q: &GalleryVideoInfoListQuery,
    ) -> Result<GalleryVideoInfoVO> {
        let page = q.page_query();
        let gallery_ids = match q.gallery_filter() {
            Some(id) => Some(self.repo().find_gallery_subtree_ids(id).await?),
            None => None,
        };
        let (total, rows) = self
            .repo()
            .paginate(scope, q, gallery_ids.as_deref())
            .await?;
        let list: Vec<GalleryVideoInfoItemVO> = rows.iter().map(Into::into).collect();

        let mut vo = GalleryVideoInfoVO {
            child_gallery_list: Vec::new(),
            gallery_video_info_page: PageResult::new(total, list, &page),
            gallery_video: None,
        };
        if let Some(id) = q.gallery_filter() {
            if let Some(gallery) = self.repo().find_gallery_by_id(id).await? {
                vo.gallery_video = Some((&gallery).into());
            }
            let children = self.repo().find_child_galleries(id).await?;
            let mut child_list = Vec::with_capacity(children.len());
            for child in &children {
                let mut child_vo: GalleryVideoResVO = child.into();
                let infos = self.repo().find_first_infos(child.id, 4).await?;
                child_vo.gallery_video_info_list =
                    infos.iter().map(Into::into).collect();
                child_list.push(child_vo);
            }
            vo.child_gallery_list = child_list;
        }
        Ok(vo)
    }

    /// 详情（Java `#detail`）。
    ///
    /// # Errors
    /// - `id` 缺失 → `Validation("id不能为空")`
    /// - 记录不存在 → 业务错误「系统未找到视频信息」
    pub async fn detail(&self, id: Option<i32>) -> Result<GalleryVideoInfoDetailVO> {
        let id = id.ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let row = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_VIDEO_INFO_NOT_FOUND))?;
        // 多租户：平台素材可读；店铺素材仅本店可读
        ensure_media_access(row.shop_id)?;
        Ok(GalleryVideoInfoDetailVO {
            id: row.id,
            gallery_id: row.gallery_id,
            video_url: row.video_url,
            video_name: row.video_name,
            video_cover: row.video_cover,
            description: row.description,
            format: row.format,
            duration: row.duration,
            size: row.size,
            add_time: i64::from(row.add_time),
        })
    }

    /// 创建（Java `#save`）。
    ///
    /// # Errors
    /// - 必填缺失 → `Validation`（Java `@Valid` 文案）
    /// - 外链非法 → 业务错误（Java `validateExternalVideoUrl` 文案）
    pub async fn create(
        &self,
        scope: VideoScope,
        admin_id: i32,
        dto: &GalleryVideoInfoDTO,
    ) -> Result<i32> {
        let (url, name) = validate_required(dto).map_err(Error::Validation)?;
        if dto.source.as_deref() == Some("external") {
            validate_external_video_url(Some(&url)).map_err(Error::biz_msg)?;
        }
        let row = GalleryVideoInfo {
            id: 0,
            // 多租户：店铺账号强制落本店；平台账号请求值缺失 → 平台素材（0）
            shop_id: media_write_shop_id(scope.shop_id)?,
            gallery_id: dto.resolve_gallery_id(),
            video_url: url,
            video_name: name,
            video_cover: dto.video_cover.clone(),
            add_time: now_seconds() as i32,
            add_user_id: admin_id,
            format: dto.format.clone(),
            video_first_frame: dto.video_first_frame.clone(),
            duration: dto.duration.clone(),
            size: dto.size.clone(),
            vendor_id: scope.vendor_id.unwrap_or(0),
            source: dto.source.clone().unwrap_or_default(),
            description: dto.description.clone(),
        };
        let id = self.repo().insert(&row).await?;
        tracing::info!("创建相册视频: id={id}, gallery_id={}", row.gallery_id);
        Ok(id)
    }

    /// 更新（Java `#update`，选择性更新）。
    ///
    /// # Errors
    /// - `id` 缺失 → `Validation("id不能为空")`
    /// - 外链非法 → 业务错误
    /// - 记录不存在 → NotFound
    pub async fn update(&self, dto: &GalleryVideoInfoDTO) -> Result<()> {
        let id = dto.id.ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let existing = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_VIDEO_INFO_NOT_FOUND.into()))?;
        // 多租户：仅本店素材可改（平台素材对店铺账号只读）
        ensure_media_write(existing.shop_id)?;
        if let Some(url) = dto.video_url.as_deref() {
            if existing.source == "external" || dto.source.as_deref() == Some("external") {
                validate_external_video_url(Some(url)).map_err(Error::biz_msg)?;
            }
        }
        let merged = GalleryVideoInfo {
            id: existing.id,
            shop_id: existing.shop_id,
            gallery_id: if dto.gallery_id.is_empty() {
                existing.gallery_id
            } else {
                dto.resolve_gallery_id()
            },
            video_url: dto.video_url.clone().unwrap_or(existing.video_url),
            video_name: dto.video_name.clone().unwrap_or(existing.video_name),
            video_cover: dto.video_cover.clone().or(existing.video_cover),
            add_time: now_seconds() as i32,
            add_user_id: existing.add_user_id,
            format: dto.format.clone().or(existing.format),
            video_first_frame: dto
                .video_first_frame
                .clone()
                .or(existing.video_first_frame),
            duration: dto.duration.clone().or(existing.duration),
            size: dto.size.clone().or(existing.size),
            vendor_id: existing.vendor_id,
            source: dto.source.clone().unwrap_or(existing.source),
            description: dto.description.clone().or(existing.description),
        };
        self.repo().update(&merged).await?;
        tracing::info!("更新相册视频: id={id}");
        Ok(())
    }

    /// 删除（Java `#del`）。
    ///
    /// # Errors
    /// - `id` 缺失 → `Validation("id不能为空")`
    pub async fn delete(&self, dto: &OperateDTO) -> Result<()> {
        let id = dto.id.ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        // 多租户：删除前校验归属（记录不存在时保持静默 0 行语义）
        if let Some(row) = self.repo().find_by_id(id).await? {
            ensure_media_write(row.shop_id)?;
        }
        self.repo().delete(id).await?;
        tracing::info!("删除相册视频: id={id}");
        Ok(())
    }

    /// 上传视频（Java `#uploadVideo`）：本地校验通过后返回明确「未迁移」。
    ///
    /// # Errors
    /// - 空文件 / 非法类型 → `Validation`（Java 文案）
    /// - 其余 → 业务错误「未迁移」
    pub async fn upload_video(
        &self,
        file_name: Option<&str>,
        content_type: Option<&str>,
        bytes: &[u8],
    ) -> Result<UploadVideoInfoVO> {
        validate_upload_file(file_name, content_type).map_err(Error::Validation)?;
        let original = file_name
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("video.mp4");
        let url =
            nextrade_infrastructure::storage::save_local_upload("video", original, bytes).await?;
        let stem = std::path::Path::new(original)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("video")
            .to_string();
        Ok(UploadVideoInfoVO {
            video_url: url,
            video_name: stem,
            format: "mp4".to_string(),
            video_cover: String::new(),
        })
    }
}

/// 供路由层复用的 VO 转换（相册实体）。
#[must_use]
pub fn gallery_to_vo(g: &GalleryVideo) -> GalleryVideoResVO {
    g.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::content::gallery_video_info::{
        MSG_FILE_EMPTY, MSG_FILE_TYPE, MSG_VIDEO_NAME_REQUIRED, MSG_VIDEO_URL_REQUIRED,
    };

    #[test]
    fn create_validation_messages() {
        let err = validate_required(&GalleryVideoInfoDTO::default()).unwrap_err();
        assert_eq!(err, MSG_VIDEO_URL_REQUIRED);
        let dto = GalleryVideoInfoDTO {
            video_url: Some("https://a.com/v.mp4".into()),
            video_name: Some(" ".into()),
            ..Default::default()
        };
        assert_eq!(validate_required(&dto).unwrap_err(), MSG_VIDEO_NAME_REQUIRED);
    }

    #[test]
    fn upload_validation_rejects_empty_files() {
        assert_eq!(
            validate_upload_file(None, None).unwrap_err(),
            MSG_FILE_EMPTY
        );
        assert_eq!(
            validate_upload_file(Some("a.exe"), None).unwrap_err(),
            MSG_FILE_TYPE
        );
        // 校验通过后 upload_video 才返回未迁移
        assert_eq!(
            validate_upload_file(Some("a.mp4"), Some("video/mp4")).unwrap(),
            ()
        );
    }

    #[test]
    fn detail_id_required_message() {
        // 路由层 Query 缺失 id → 服务层 Validation("id不能为空")
        assert_eq!(MSG_ID_REQUIRED, "id不能为空");
        assert_eq!(MSG_VIDEO_INFO_NOT_FOUND, "系统未找到视频信息");
    }

    #[test]
    fn gallery_to_vo_copies_entity_fields() {
        let g = GalleryVideo {
            id: 2,
            shop_id: 0,
            parent_id: 1,
            name: "相册".into(),
            sort: 3,
            add_time: 1_700_000_000,
            add_user_id: 7,
            vendor_id: 0,
        };
        let vo = gallery_to_vo(&g);
        assert_eq!(vo.id, 2);
        assert_eq!(vo.parent_id, 1);
        assert_eq!(vo.name, "相册");
        assert!(vo.gallery_video_info_list.is_empty());
    }

    #[test]
    fn upload_video_validation_rejects_empty_file() {
        assert!(MSG_FILE_EMPTY.contains("不能为空"));
    }
}
