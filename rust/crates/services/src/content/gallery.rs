//! 相册域服务（content::gallery）。
//!
//! 对应原 Java `com.nextrade.service.setting.impl.GalleryServiceImpl`
//! （`java/nextrade-service/src/main/java/com/nextrade/service/setting/impl/GalleryServiceImpl.java`）
//! 与继承来的 `BaseServiceImpl#batch` / `#del`。
//!
//! 构造签名固定：`pub fn new(repos: Arc<Repositories>) -> Self`。
//!
//! ## 已迁移（9 条 admin 端点）
//! - `list`：多租户过滤 + `parent_id` 过滤 + 动态排序 + 固定页大小 99
//! - `detail`：按主键取详情（`id < 1` → `null`，与 Java 一致）
//! - `create`：名称校验 + 父级归一 + `max(sort)+1` 追加 + 落 `shop_id` / `vendor_id`
//! - `update`：按 `id` 定位、非提交字段保留原值（等价 MyBatis `updateById` 选择性更新）
//! - `del`：按 `deleteMode` 两分支（仅删分组 / 删分组和素材）
//! - `batch`：仅 `type == "del"` 生效（`BaseServiceImpl#batch`）
//! - `tree`：分组树（同级排序 + 子孙素材数累加）
//! - `sort`：同级 `gallery_sort = i + 1`（事务）
//! - `move`：分组子树素材转移，`mode = move_and_delete` 时删除来源子树（事务）
//!
//! ## 未迁移（诚实记录）
//! 1. **`type = "video"` 分支**：Java `tree` / `sort` / `move` 在 `type == "video"`
//!    时分流到 `GalleryVideoService`（`gallery_video*` 表，属 `content/gallery_video`
//!    特性，不在本批次范围）。本实现返回明确业务错误
//!    按类型分别统计/移动素材。
//! 2. **`gallery_pic` 其它端点**（上传/批量下载/详情/排序/转移/外部直链）属
//!    `content/gallery_pic` 特性，本服务只操作其 `gallery_id` 列（树计数、
//!    分组转移、级联删除），不越界实现其 HTTP 契约。
//!
//! ## 与 Java 的语义差异（均不影响正常前端请求）
//! - Java `@Valid` 校验以 DTO 注解声明为准（`sort` / `move` / `del` 控制器带
//!   `@Valid`）；本实现按同一批文案做服务层校验。
//! - Java `update` 在 `id` 不存在时静默影响 0 行；本实现同样 `Ok(())`，不报 404。
//! - 错误码：Java `GlobalException` 默认 `code = 1001`；本实现沿用项目既有映射
//!   （`Error::biz_msg` → 500），仅保证**消息文案逐字一致**（与 app_version_admin 同口径）。

use std::sync::Arc;

use nextrade_core::entities::Gallery;
use nextrade_core::models::content::gallery::{
    build_gallery_tree, has_delete_mode, is_delete_files_mode, validate_gallery_name,
    BatchDTO, GalleryDTO, GalleryDelDTO, GalleryListQuery, GalleryMoveDTO, GalleryScope,
    GallerySortDTO, GalleryTreeVO, GalleryUpdateDTO,
    ERR_GALLERY_NOT_FOUND, ERR_GROUP_ID_REQUIRED, ERR_ID_REQUIRED, ERR_MOVE_MODE_REQUIRED,
    ERR_SAME_GROUP, ERR_SORT_IDS_REQUIRED, ERR_SOURCE_ID_REQUIRED, ERR_TARGET_ID_REQUIRED,
    ERR_TARGET_IS_DESCENDANT, ERR_TYPE_REQUIRED,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

use crate::content::tenant_scope::{ensure_media_access, ensure_media_write, media_write_shop_id};

/// 相册服务。
pub struct GalleryService {
    repos: Arc<Repositories>,
}

impl GalleryService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储（NESTED 访问：`self.repos.content.gallery`）。
    #[must_use]
    pub fn repo(&self) -> &nextrade_infrastructure::db::repo::content::gallery::GalleryRepository {
        &self.repos.content.gallery
    }

    // ————————————————————— 管理端 —————————————————————

    /// 相册分页（Java `GalleryServiceImpl#list`）。
    ///
    /// 页大小固定 **99**（Java `new Page<>(dto.getPage(), 99)`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn list(
        &self,
        scope: GalleryScope,
        q: &GalleryListQuery,
    ) -> Result<PageResult<GalleryDTO>> {
        let page = q.page_query();
        let (total, rows) = self.repo().paginate(scope, q).await?;
        let list = rows.iter().map(GalleryDTO::from).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 相册详情（Java `#detail`）。
    ///
    /// # Errors
    /// - `id` 缺失 → `Validation("id不能为空")`（Java `@RequestParam Integer id` 必填）
    /// - `id >= 1` 但记录不存在 → `NotFound("相册不存在")`
    ///
    /// `id < 1` 时返回 `Ok(None)`，与 Java `detail` 返回 `null` 一致。
    pub async fn detail(&self, id: Option<i32>) -> Result<Option<GalleryDTO>> {
        let id = match resolve_detail_id(id)? {
            Some(v) => v,
            None => return Ok(None),
        };
        let row = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound(ERR_GALLERY_NOT_FOUND.into()))?;
        // 多租户：平台素材（shop_id=0）可读；店铺素材仅本店可读
        ensure_media_access(row.shop_id)?;
        Ok(Some(GalleryDTO::from(&row)))
    }

    /// 创建相册（Java `#create`）。
    ///
    /// 逐字对齐的执行顺序：
    /// 1. 名称校验（`相册名称不能为空` / `相册名称长度不能超过100`）
    /// 2. `parentId` 归一（缺省 0）
    /// 3. `gallerySort = 同父级 max(sort) + 1`（Java 覆盖前端传值）
    /// 4. `shop_id` 取登录上下文（`None` → 0）、`vendor_id` 取供应商上下文
    ///
    /// # Errors
    /// 名称非法 → `Validation`；数据库失败 → `Error::Database`。
    pub async fn create(&self, scope: GalleryScope, dto: &GalleryDTO) -> Result<i32> {
        let name = validate_gallery_name(dto.gallery_name.as_deref()).map_err(Error::Validation)?;
        let parent_id = dto.parent_id.unwrap_or(0);
        let sort = self.repo().next_gallery_sort(scope, parent_id).await?;
        let row = Gallery {
            gallery_id: 0,
            parent_id,
            // Java 从请求体取 galleryAdminId（缺省 null 撞 NOT NULL）；这里缺省 0
            gallery_admin_id: dto.gallery_admin_id.unwrap_or(0),
            gallery_name: name,
            gallery_sort: sort,
            // Java BeanUtils 拷贝 null 后落库同样失败；这里按列 NOT NULL 归一为空串
            gallery_thumb: dto.gallery_thumb.clone().unwrap_or_default(),
            // 多租户：店铺账号强制落本店；平台账号请求值缺失 → 平台素材（0）
            shop_id: media_write_shop_id(scope.shop_id)?,
            vendor_id: scope.vendor_id,
        };
        let id = self.repo().insert(&row).await?;
        tracing::info!("创建相册成功: id={id}, parent_id={parent_id}");
        Ok(id)
    }

    /// 更新相册（Java `#update`）。
    ///
    /// Java `update(entity, wrapper.eq(gallery_id, dto.id))` 是**选择性更新**：
    /// 请求体未提交的字段保持库中原值。本实现先读原行再合并，语义等价。
    /// `id` 对应记录不存在时 Java 静默影响 0 行，这里同样 `Ok(())`。
    ///
    /// # Errors
    /// - `id` 缺失 → `Validation("id不能为空")`
    /// - 名称非法 → `Validation`
    pub async fn update(&self, scope: GalleryScope, dto: &GalleryUpdateDTO) -> Result<()> {
        let id = dto.id.ok_or_else(|| Error::Validation(ERR_ID_REQUIRED.into()))?;
        let existing = match self.repo().find_by_id(id).await? {
            Some(row) => row,
            // Java：UPDATE ... WHERE gallery_id = 不存在的 id → 0 行，不报错
            None => return Ok(()),
        };
        // 多租户：仅本店素材可写（平台素材对店铺账号只读）
        ensure_media_write(existing.shop_id)?;
        let merged = merge_update(existing, dto, scope)?;
        self.repo().update(&merged).await?;
        tracing::info!("更新相册成功: id={id}");
        Ok(())
    }

    /// 删除相册（Java `#del`）。
    ///
    /// 有 `deleteMode` → `delWithMode`（连同子分组处理素材）；
    /// 无 `deleteMode` → `BaseServiceImpl#del`（只删单行）。
    ///
    /// # Errors
    /// - `id` 缺失 → `Validation("分组ID不能为空")`
    /// - 数据库失败 → `Error::Database`
    pub async fn delete(&self, dto: &GalleryDelDTO) -> Result<()> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation(ERR_GROUP_ID_REQUIRED.into()))?;
        // 多租户：删除前校验归属（记录不存在时保持 Java 的静默 0 行语义）
        if let Some(row) = self.repo().find_by_id(id).await? {
            ensure_media_write(row.shop_id)?;
        }
        if has_delete_mode(dto.delete_mode.as_deref()) {
            let delete_files = is_delete_files_mode(dto.delete_mode.as_deref().unwrap_or(""));
            self.repo().delete_groups_with_mode(id, delete_files).await?;
            tracing::info!("按模式删除相册分组: id={id}, delete_files={delete_files}");
        } else {
            self.repo().delete(id).await?;
            tracing::info!("删除相册: id={id}");
        }
        Ok(())
    }

    /// 批量操作（Java `BaseServiceImpl#batch`）。
    ///
    /// 仅 `type == "del"` 生效（批量删除相册行，**不**级联处理子分组/素材，
    /// 与 Java `BaseServiceImpl#batch` 逐字一致）；其它类型返回 `false` 且无副作用。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if batch_delete_ids(dto).is_none() {
            // Java：`return false;`（无副作用）
            return Ok(false);
        }
        let ids = &dto.ids;
        // 多租户：先整体校验归属（任一越权 → 403，不做部分删除）
        for id in ids {
            if let Some(row) = self.repo().find_by_id(*id).await? {
                ensure_media_write(row.shop_id)?;
            }
        }
        let affected = self.repo().delete_many(ids).await?;
        tracing::info!("批量删除相册: {} 条", affected);
        Ok(affected > 0)
    }

    /// 统一分组树（Java `#tree` + `GalleryServiceImpl#tree`）。
    ///
    /// # Errors
    /// - 数据库失败 → `Error::Database`
    pub async fn tree(&self, scope: GalleryScope, r#type: &str) -> Result<GalleryTreeVO> {
        let galleries = self.repo().list_for_tree(scope).await?;
        let counts = if is_video_type(Some(r#type)) {
            self.repo().count_videos_by_gallery().await?
        } else {
            self.repo().count_pics_by_gallery().await?
        };
        Ok(GalleryTreeVO {
            records: build_gallery_tree(&galleries, &counts),
        })
    }

    /// 分组排序（Java `#sort`，`@Transactional`）。
    ///
    /// # Errors
    /// - `type` 缺失/空 → `Validation("类型不能为空")`
    /// - `ids` 为空 → `Validation("排序ID列表不能为空")`
    pub async fn sort(&self, dto: &GallerySortDTO) -> Result<()> {
        validate_sort_request(dto)?;
        // 图片/视频共用 `gallery` 分组表，排序逻辑一致
        self.repo().sort_groups(&dto.ids).await?;
        tracing::info!("相册分组排序: {} 个", dto.ids.len());
        Ok(())
    }

    /// 分组转移（Java `#move`，`@Transactional`）。
    ///
    /// 校验顺序逐字对齐 Java：
    /// 1. `sourceId == targetId` → 「来源分组和目标分组不能相同」
    /// 2. 目标分组是来源分组的子孙 → 「目标分组不能是来源分组的子孙节点」
    /// 3. 子树内素材 `gallery_id` 改为目标分组；`move_and_delete` 时删除来源子树
    ///
    /// # Errors
    /// 入参缺失 → `Validation`；同组/子孙 → 业务错误；数据库失败 → `Error::Database`。
    pub async fn move_group(&self, dto: &GalleryMoveDTO) -> Result<()> {
        let (source_id, target_id, mode) = validate_move_request(dto)?;
        let video = is_video_type(dto.r#type.as_deref());
        if source_id == target_id {
            return Err(Error::biz_msg(ERR_SAME_GROUP));
        }
        // Java：先收集来源子树（含自身），再校验目标是否落在子树内
        let subtree = self.repo().find_subtree_ids(source_id).await?;
        if subtree.contains(&target_id) {
            return Err(Error::biz_msg(ERR_TARGET_IS_DESCENDANT));
        }
        let delete_source = mode == "move_and_delete";
        self.repo()
            .move_groups(source_id, target_id, delete_source, video)
            .await?;
        tracing::info!("相册分组转移: {source_id} -> {target_id}, delete_source={delete_source}");
        Ok(())
    }
}

/// 详情入参归一（Java `@RequestParam Integer id` 必填 + `id < 1` 返回 `null`）。
///
/// - `None` → `Validation("id不能为空")`
/// - `Some(id)` 且 `id < 1` → `Ok(None)`（调用方返回 JSON `null`）
/// - `Some(id)` 且 `id >= 1` → `Ok(Some(id))`
fn resolve_detail_id(id: Option<i32>) -> Result<Option<i32>> {
    match id {
        None => Err(Error::Validation(ERR_ID_REQUIRED.into())),
        Some(v) if v < 1 => Ok(None),
        Some(v) => Ok(Some(v)),
    }
}

/// 合并「选择性更新」（对应 Java `updateById` 的 `NOT_NULL` 字段策略）。
///
/// - `gallery_name`：提交则校验并覆盖；
/// - `parent_id` / `gallery_admin_id` / `gallery_sort`：提交则覆盖；
/// - `gallery_thumb`：提交则覆盖；
/// - `shop_id`：Java `convertToBean` 总是写 `HeaderUtils.getShopId()`，
///   非 `null` 才参与更新；这里同样是 `scope.shop_id` 优先、否则保留原值；
/// - `vendor_id`：Java `update` 不写该列，保留原值。
fn merge_update(existing: Gallery, dto: &GalleryUpdateDTO, scope: GalleryScope) -> Result<Gallery> {
    let name = match dto.gallery_name.as_deref() {
        Some(raw) => validate_gallery_name(Some(raw)).map_err(Error::Validation)?,
        None => existing.gallery_name,
    };
    Ok(Gallery {
        gallery_id: existing.gallery_id,
        parent_id: dto.parent_id.unwrap_or(existing.parent_id),
        gallery_admin_id: dto.gallery_admin_id.unwrap_or(existing.gallery_admin_id),
        gallery_name: name,
        gallery_sort: dto.gallery_sort.unwrap_or(existing.gallery_sort),
        gallery_thumb: dto
            .gallery_thumb
            .clone()
            .unwrap_or(existing.gallery_thumb),
        shop_id: scope.shop_id.unwrap_or(existing.shop_id),
        vendor_id: existing.vendor_id,
    })
}

/// 批量操作判定：仅 `type == "del"` 且 `ids` 非空时返回待删 ID。
///
/// Java `BaseServiceImpl#batch`：`"del".equals(type)` 才 `removeByIds(ids)`，
/// 其余返回 `false`；`removeByIds` 对空集合不落 SQL、返回 `false`。
fn batch_delete_ids(dto: &BatchDTO) -> Option<&[i32]> {
    if dto.r#type.as_deref() != Some("del") || dto.ids.is_empty() {
        return None;
    }
    Some(&dto.ids)
}

/// `type == "video"` 判定（Java `"video".equals(type)`）。
fn is_video_type(r#type: Option<&str>) -> bool {
    r#type == Some("video")
}

/// `sort` 入参校验（Java `@Valid`：`@NotNull type` + `@NotEmpty ids`）。
fn validate_sort_request(dto: &GallerySortDTO) -> Result<()> {
    if dto.r#type.as_deref().map(str::trim).filter(|s| !s.is_empty()).is_none() {
        return Err(Error::Validation(ERR_TYPE_REQUIRED.into()));
    }
    if dto.ids.is_empty() {
        return Err(Error::Validation(ERR_SORT_IDS_REQUIRED.into()));
    }
    Ok(())
}

/// `move` 入参校验（Java `@Valid`：四个 `@NotNull`）。
fn validate_move_request(dto: &GalleryMoveDTO) -> Result<(i32, i32, &str)> {
    let source_id = dto
        .source_id
        .ok_or_else(|| Error::Validation(ERR_SOURCE_ID_REQUIRED.into()))?;
    let target_id = dto
        .target_id
        .ok_or_else(|| Error::Validation(ERR_TARGET_ID_REQUIRED.into()))?;
    let mode = dto
        .mode
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Validation(ERR_MOVE_MODE_REQUIRED.into()))?;
    if dto.r#type.as_deref().map(str::trim).filter(|s| !s.is_empty()).is_none() {
        return Err(Error::Validation(ERR_TYPE_REQUIRED.into()));
    }
    Ok((source_id, target_id, mode))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::content::gallery::{
        ERR_GALLERY_NAME_IS_NULL, ERR_GALLERY_NAME_OVER_LENGTH,
    };

    fn gallery(id: i32, parent_id: i32, name: &str, sort: i32) -> Gallery {
        Gallery {
            gallery_id: id,
            parent_id,
            gallery_admin_id: 1,
            gallery_name: name.to_string(),
            gallery_sort: sort,
            gallery_thumb: "t.png".into(),
            shop_id: 5,
            vendor_id: Some(7),
        }
    }

    #[test]
    fn detail_id_resolution_matches_java() {
        assert!(resolve_detail_id(None).is_err());
        assert_eq!(resolve_detail_id(Some(0)).unwrap(), None);
        assert_eq!(resolve_detail_id(Some(-3)).unwrap(), None);
        assert_eq!(resolve_detail_id(Some(9)).unwrap(), Some(9));
        // 缺失 id 的文案
        assert_eq!(
            resolve_detail_id(None).unwrap_err().to_string(),
            "参数校验失败: id不能为空"
        );
    }

    #[test]
    fn merge_update_keeps_unsent_fields_and_applies_sent_ones() {
        let existing = gallery(3, 0, "旧名称", 4);
        // 只提交名字 → 其它字段（parent/排序/缩略图/shop/vendor）保持原值
        let dto = GalleryUpdateDTO {
            id: Some(3),
            gallery_name: Some("  新名称  ".into()),
            ..Default::default()
        };
        let merged = merge_update(existing.clone(), &dto, GalleryScope::unscoped()).unwrap();
        assert_eq!(merged.gallery_name, "新名称");
        assert_eq!(merged.parent_id, 0);
        assert_eq!(merged.gallery_sort, 4);
        assert_eq!(merged.gallery_thumb, "t.png");
        assert_eq!(merged.shop_id, 5);
        assert_eq!(merged.vendor_id, Some(7));

        // 提交 parentId / gallerySort / thumb / adminId → 覆盖
        let dto = GalleryUpdateDTO {
            id: Some(3),
            parent_id: Some(2),
            gallery_admin_id: Some(9),
            gallery_sort: Some(8),
            gallery_thumb: Some("n.png".into()),
            ..Default::default()
        };
        let merged = merge_update(existing.clone(), &dto, GalleryScope::unscoped()).unwrap();
        assert_eq!(merged.parent_id, 2);
        assert_eq!(merged.gallery_admin_id, 9);
        assert_eq!(merged.gallery_sort, 8);
        assert_eq!(merged.gallery_thumb, "n.png");

        // 平台账号（shop_id = Some(0)）→ 与 Java 一致，覆盖为 0
        let merged = merge_update(existing.clone(), &dto, GalleryScope::platform()).unwrap();
        assert_eq!(merged.shop_id, 0);
        // 无法判定账号（None）→ Java null 不更新 → 保留原值
        let merged = merge_update(existing, &dto, GalleryScope::unscoped()).unwrap();
        assert_eq!(merged.shop_id, 5);
    }

    #[test]
    fn merge_update_validates_submitted_name() {
        let existing = gallery(3, 0, "旧名称", 4);
        let empty = GalleryUpdateDTO {
            id: Some(3),
            gallery_name: Some("   ".into()),
            ..Default::default()
        };
        assert_eq!(
            merge_update(existing.clone(), &empty, GalleryScope::unscoped())
                .unwrap_err()
                .to_string(),
            format!("参数校验失败: {ERR_GALLERY_NAME_IS_NULL}")
        );
        let long = GalleryUpdateDTO {
            id: Some(3),
            gallery_name: Some("字".repeat(101)),
            ..Default::default()
        };
        assert_eq!(
            merge_update(existing, &long, GalleryScope::unscoped())
                .unwrap_err()
                .to_string(),
            format!("参数校验失败: {ERR_GALLERY_NAME_OVER_LENGTH}")
        );
    }

    #[test]
    fn batch_only_deletes_for_del_type_with_ids() {
        let dto = BatchDTO {
            ids: vec![1, 2],
            r#type: Some("del".into()),
            value: 0,
        };
        assert_eq!(batch_delete_ids(&dto), Some(&[1, 2][..]));
        // 非 del → 不做任何事（Java 返回 false）
        let other = BatchDTO {
            ids: vec![1],
            r#type: Some("show".into()),
            value: 0,
        };
        assert_eq!(batch_delete_ids(&other), None);
        // 空 ids → removeByIds 不落 SQL，等同 false
        let empty = BatchDTO {
            ids: vec![],
            r#type: Some("del".into()),
            value: 0,
        };
        assert_eq!(batch_delete_ids(&empty), None);
        let none = BatchDTO {
            ids: vec![1],
            r#type: None,
            value: 0,
        };
        assert_eq!(batch_delete_ids(&none), None);
    }

    #[test]
    fn sort_and_move_validations_use_java_messages() {
        let sort = GallerySortDTO {
            ids: vec![1],
            r#type: None,
        };
        assert_eq!(
            validate_sort_request(&sort).unwrap_err().to_string(),
            format!("参数校验失败: {ERR_TYPE_REQUIRED}")
        );
        let sort = GallerySortDTO {
            ids: vec![],
            r#type: Some("image".into()),
        };
        assert_eq!(
            validate_sort_request(&sort).unwrap_err().to_string(),
            format!("参数校验失败: {ERR_SORT_IDS_REQUIRED}")
        );
        let ok = GallerySortDTO {
            ids: vec![1, 2],
            r#type: Some("image".into()),
        };
        assert!(validate_sort_request(&ok).is_ok());

        let mv = GalleryMoveDTO {
            source_id: None,
            target_id: Some(0),
            mode: Some("move_and_keep".into()),
            r#type: Some("image".into()),
        };
        assert_eq!(
            validate_move_request(&mv).unwrap_err().to_string(),
            format!("参数校验失败: {ERR_SOURCE_ID_REQUIRED}")
        );
        let mv = GalleryMoveDTO {
            source_id: Some(1),
            target_id: None,
            mode: Some("move_and_keep".into()),
            r#type: Some("image".into()),
        };
        assert_eq!(
            validate_move_request(&mv).unwrap_err().to_string(),
            format!("参数校验失败: {ERR_TARGET_ID_REQUIRED}")
        );
        let mv = GalleryMoveDTO {
            source_id: Some(1),
            target_id: Some(0),
            mode: Some("  ".into()),
            r#type: Some("image".into()),
        };
        assert_eq!(
            validate_move_request(&mv).unwrap_err().to_string(),
            format!("参数校验失败: {ERR_MOVE_MODE_REQUIRED}")
        );
        let mv = GalleryMoveDTO {
            source_id: Some(1),
            target_id: Some(0),
            mode: Some("move_and_keep".into()),
            r#type: None,
        };
        assert_eq!(
            validate_move_request(&mv).unwrap_err().to_string(),
            format!("参数校验失败: {ERR_TYPE_REQUIRED}")
        );
        let ok = GalleryMoveDTO {
            source_id: Some(3),
            target_id: Some(0),
            mode: Some("move_and_delete".into()),
            r#type: Some("image".into()),
        };
        assert_eq!(validate_move_request(&ok).unwrap(), (3, 0, "move_and_delete"));
    }

    #[test]
    fn video_type_is_detected_and_other_types_are_gallery() {
        assert!(is_video_type(Some("video")));
        // Java `"video".equals(type)` 大小写敏感；image/空 都走相册分支
        assert!(!is_video_type(Some("image")));
        assert!(!is_video_type(Some("VIDEO")));
        assert!(!is_video_type(Some("")));
        assert!(!is_video_type(None));
    }

    #[test]
    fn move_request_accepts_video_type() {
        let mv = GalleryMoveDTO {
            source_id: Some(3),
            target_id: Some(4),
            mode: Some("move_and_keep".into()),
            r#type: Some("video".into()),
        };
        assert_eq!(validate_move_request(&mv).unwrap(), (3, 4, "move_and_keep"));
    }
}
