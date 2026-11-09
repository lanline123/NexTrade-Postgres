//! 视频相册服务（content::gallery_video）。
//!
//! 对应原 Java `com.nextrade.service.video.impl.GalleryVideoServiceImpl`
//! 与继承的 `BaseServiceImpl`（`batch` / `del`）。
//!
//! 构造签名固定：`pub fn new(repos: Arc<Repositories>) -> Self`。
//!
//! ## 已迁移（7）
//! - `getAllCategory`（treeList）：树形列表（租户过滤 + 同级 sort/id 排序）
//! - `list`：分页（页大小 99）+ parent 过滤 + 动态排序
//! - `detail`：`id < 1` → `null`；缺行 → NotFound
//! - `create`：parent 归一 + `max(sort)+1` + `shop_id` / `vendor_id` / `add_user_id` / `add_time`
//! - `update`：名称查重（同父级、排除自身）+ name / 可选 sort
//! - `del`：`deleteMode` 两分支（仅删分组 / 删分组和素材）
//! - `batch`：仅 `type == "del"`
//!
//! ## 与 Java 的差异
//! 见 [`crate::models::content::gallery_video`] 模块头（list 的 store/pickup
//! 无过滤原样保留；detail 缺行 NotFound；错误码 500 惯例）。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::GalleryVideo;
use nextrade_core::models::content::gallery_video::{
    build_video_tree, has_delete_mode, is_delete_batch, is_delete_files_mode, require_group_id,
    validate_create, validate_update, BatchDTO, GalleryDelDTO, GalleryVideoCreateDTO,
    GalleryVideoListQuery, GalleryVideoListVO, GalleryVideoUpdateDTO, GalleryVideoVO, VideoScope,
    MSG_ID_REQUIRED, MSG_NAME_EXISTS, MSG_VIDEO_NOT_FOUND,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

use crate::content::tenant_scope::{ensure_media_access, ensure_media_write, media_write_shop_id};

/// 视频相册服务。
pub struct GalleryVideoService {
    repos: Arc<Repositories>,
}

impl GalleryVideoService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储（NESTED 访问：`self.repos.content.gallery_video`）。
    #[must_use]
    pub fn repo(
        &self,
    ) -> &nextrade_infrastructure::db::repo::content::gallery_video::GalleryVideoRepository {
        &self.repos.content.gallery_video
    }

    /// 树形列表（Java `treeList` / `GET getAllCategory`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn tree_list(&self, scope: VideoScope) -> Result<Vec<GalleryVideoListVO>> {
        let rows = self.repo().list_for_tree(scope).await?;
        Ok(build_video_tree(&rows))
    }

    /// 分页列表（Java `#list`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn list(
        &self,
        scope: VideoScope,
        q: &GalleryVideoListQuery,
    ) -> Result<PageResult<GalleryVideoVO>> {
        let page = q.page_query();
        let (total, rows) = self.repo().paginate(scope, q).await?;
        let list = rows.iter().map(to_vo).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 详情（Java `#detail`）。
    ///
    /// # Errors
    /// - `id` 缺失 → `Validation("id不能为空")`
    /// - 记录不存在 → `NotFound("视频相册不存在")`
    pub async fn detail(&self, id: Option<i32>) -> Result<Option<GalleryVideoVO>> {
        let Some(id) = id else {
            return Err(Error::Validation(MSG_ID_REQUIRED.into()));
        };
        if id < 1 {
            return Ok(None);
        }
        let row = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_VIDEO_NOT_FOUND.into()))?;
        // 多租户：平台素材可读；店铺素材仅本店可读
        ensure_media_access(row.shop_id)?;
        Ok(Some(to_vo(&row)))
    }

    /// 创建（Java `#create`）。
    ///
    /// `scope` 使用 treeList 同款规则（admin → shop/vendor=0；shop/store/pickup →
    /// shop；vendor → vendor）；`admin_id` 为当前管理员 ID（`AuthUser.id`）。
    ///
    /// # Errors
    /// - 参数缺失 → `Validation`（Java `@Valid` 文案）
    /// - 数据库失败 → `Error::Database`
    pub async fn create(
        &self,
        scope: VideoScope,
        admin_id: i32,
        dto: &GalleryVideoCreateDTO,
    ) -> Result<i32> {
        let (parent_id, name) = validate_create(dto).map_err(Error::Validation)?;
        let sort = self.repo().next_sort(scope, parent_id).await?;
        let row = GalleryVideo {
            id: 0,
            // 多租户：店铺账号强制落本店；平台账号请求值缺失 → 平台素材（0）
            shop_id: media_write_shop_id(scope.shop_id)?,
            parent_id,
            name,
            sort,
            add_time: now_seconds() as i32,
            add_user_id: admin_id,
            vendor_id: scope.vendor_id.unwrap_or(0),
        };
        let id = self.repo().insert(&row).await?;
        tracing::info!("创建视频相册: id={id}, parent_id={parent_id}");
        Ok(id)
    }

    /// 更新（Java `#update`）。
    ///
    /// # Errors
    /// - 参数缺失 → `Validation`
    /// - 记录不存在 → 业务错误「视频相册不存在」
    /// - 同父级重名 → 业务错误「视频相册名称已存在」
    pub async fn update(&self, dto: &GalleryVideoUpdateDTO) -> Result<i32> {
        let (id, name) = validate_update(dto).map_err(Error::Validation)?;
        let existing = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_VIDEO_NOT_FOUND))?;
        // 多租户：仅本店素材可改（平台素材对店铺账号只读）
        ensure_media_write(existing.shop_id)?;
        if self
            .repo()
            .name_exists_excluding(existing.parent_id, &name, id)
            .await?
        {
            return Err(Error::biz_msg(MSG_NAME_EXISTS));
        }
        self.repo().update_name_sort(id, &name, dto.sort).await?;
        tracing::info!("更新视频相册: id={id}");
        Ok(id)
    }

    /// 删除（Java `#del`）。
    ///
    /// # Errors
    /// - `id` 缺失 → `Validation("分组ID不能为空")`
    /// - 数据库失败 → `Error::Database`
    pub async fn delete(&self, dto: &GalleryDelDTO) -> Result<()> {
        let id = require_group_id(dto.id).map_err(Error::Validation)?;
        // 多租户：删除前校验归属（记录不存在时保持静默 0 行语义）
        if let Some(row) = self.repo().find_by_id(id).await? {
            ensure_media_write(row.shop_id)?;
        }
        if has_delete_mode(dto.delete_mode.as_deref()) {
            let delete_files = is_delete_files_mode(dto.delete_mode.as_deref().unwrap_or(""));
            self.repo().delete_with_mode(id, delete_files).await?;
            tracing::info!("按模式删除视频相册分组: id={id}, delete_files={delete_files}");
        } else {
            self.repo().delete(id).await?;
            tracing::info!("删除视频相册: id={id}");
        }
        Ok(())
    }

    /// 批量操作（Java `BaseServiceImpl#batch`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if !is_delete_batch(dto) {
            return Ok(false);
        }
        // 多租户：先整体校验归属（任一越权 → 403，不做部分删除）
        for id in &dto.ids {
            if let Some(row) = self.repo().find_by_id(*id).await? {
                ensure_media_write(row.shop_id)?;
            }
        }
        let affected = self.repo().delete_many(&dto.ids).await?;
        Ok(affected > 0)
    }
}

/// 实体 → VO（Java `convertToDTO`）。
#[must_use]
fn to_vo(e: &GalleryVideo) -> GalleryVideoVO {
    GalleryVideoVO {
        id: e.id,
        name: e.name.clone(),
        sort: e.sort,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::content::gallery_video::{MSG_NAME_REQUIRED, MSG_PARENT_ID_REQUIRED};

    #[test]
    fn to_vo_maps_fields() {
        let row = GalleryVideo {
            id: 3,
            shop_id: 0,
            parent_id: 1,
            name: "宣传视频".into(),
            sort: 2,
            add_time: 1_700_000_000,
            add_user_id: 9,
            vendor_id: 0,
        };
        let vo = to_vo(&row);
        assert_eq!(vo.id, 3);
        assert_eq!(vo.name, "宣传视频");
        assert_eq!(vo.sort, 2);
    }

    #[test]
    fn create_validation_messages_are_verbatim() {
        assert_eq!(
            validate_create(&GalleryVideoCreateDTO::default()).unwrap_err(),
            MSG_PARENT_ID_REQUIRED
        );
        assert_eq!(
            validate_create(&GalleryVideoCreateDTO {
                parent_id: Some(0),
                name: Some("".into()),
                sort: None,
            })
            .unwrap_err(),
            MSG_NAME_REQUIRED
        );
    }

    #[test]
    fn delete_and_batch_rules_match_java() {
        assert!(has_delete_mode(Some("group_and_files")));
        assert!(is_delete_files_mode("group_and_files"));
        assert!(!is_delete_files_mode("only_group"));
        assert!(is_delete_batch(&BatchDTO {
            ids: vec![1, 2],
            r#type: Some("del".into()),
            value: 0
        }));
        assert!(!is_delete_batch(&BatchDTO {
            ids: vec![],
            r#type: Some("del".into()),
            value: 0
        }));
    }

    #[test]
    fn scope_helpers_match_admin_type_rules() {
        assert_eq!(
            VideoScope::admin(),
            VideoScope {
                shop_id: Some(0),
                vendor_id: Some(0)
            }
        );
        assert_eq!(VideoScope::shop(5).shop_id, Some(5));
        assert_eq!(VideoScope::shop(5).vendor_id, None);
        assert_eq!(VideoScope::vendor(7).vendor_id, Some(7));
        assert_eq!(VideoScope::unscoped(), VideoScope::default());
    }
}
