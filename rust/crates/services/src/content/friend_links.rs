//! 友情链接服务（content::friend_links）。
//!
//! 对应原 Java `com.nextrade.service.setting.impl.FriendLinksServiceImpl`
//! 与继承的 `BaseServiceImpl`（`updateField` / `batch` / `del`）。
//!
//! 构造签名固定：`pub fn new(repos: Arc<Repositories>) -> Self`。
//!
//! ## 已迁移（7）
//! `list`（keyword + 动态排序 + 分页）、`detail`、`create`、`update`（选择性更新）、
//! `del`、`updateField`（白名单 `link_title` / `sort_order`）、`batch`（仅 del）。
//!
//! ## 与 Java 的差异
//! 见 [`crate::models::content::friend_links`] 模块头（detail 缺行 → NotFound；
//! 业务错误码沿用项目惯例 500，消息逐字）。

use std::sync::Arc;

use nextrade_core::entities::FriendLinks;
use nextrade_core::models::content::friend_links::{
    is_delete_batch, normalize_field, validate_create, BatchDTO, FriendLinksCreateDTO,
    FriendLinksIdDTO, FriendLinksListQuery, FriendLinksUpdateDTO, FriendLinksVO, UpdateFieldDTO,
    MSG_ID_REQUIRED, MSG_INVALID_FIELD_VALUE, MSG_LINK_ID_REQUIRED, MSG_LINK_NOT_FOUND,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 友情链接服务。
pub struct FriendLinksService {
    repos: Arc<Repositories>,
}

impl FriendLinksService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储（NESTED 访问：`self.repos.content.friend_links`）。
    #[must_use]
    pub fn repo(
        &self,
    ) -> &nextrade_infrastructure::db::repo::content::friend_links::FriendLinksRepository {
        &self.repos.content.friend_links
    }

    /// 列表（Java `#list`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn list(
        &self,
        q: &FriendLinksListQuery,
    ) -> Result<PageResult<FriendLinksVO>> {
        let page = q.page_query();
        let (total, rows) = self.repo().paginate(q).await?;
        let list = rows.iter().map(to_vo).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 详情（Java `#detail`）。
    ///
    /// # Errors
    /// - 记录不存在 → `NotFound("友情链接不存在")`（Java 为 500，见模块头差异 2）
    pub async fn detail(&self, id: Option<i32>) -> Result<FriendLinksVO> {
        let id = id.ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let row = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_LINK_NOT_FOUND.into()))?;
        Ok(to_vo(&row))
    }

    /// 创建（Java `#create`）。
    ///
    /// # Errors
    /// - 标题为空 → `Validation("友情链接名称不能为空")`
    /// - 数据库失败 → `Error::Database`
    pub async fn create(&self, dto: &FriendLinksCreateDTO) -> Result<i32> {
        let title = validate_create(dto).map_err(Error::Validation)?;
        let row = FriendLinks {
            link_id: 0,
            link_title: title,
            link_logo: dto.link_logo.clone().unwrap_or_default(),
            link_url: dto.link_url.clone().unwrap_or_default(),
            sort_order: dto.sort_order.unwrap_or(0),
        };
        let id = self.repo().insert(&row).await?;
        tracing::info!("创建友情链接: id={id}");
        Ok(id)
    }

    /// 更新（Java `#update`，`updateById` 选择性更新）。
    ///
    /// Java `updateById` 使用 `linkId` 定位；`id` 缺失时无法定位，返回参数错误。
    ///
    /// # Errors
    /// - 主键缺失 → `Validation("友情链接id不能为空")`
    /// - 数据库失败 → `Error::Database`
    pub async fn update(&self, dto: &FriendLinksUpdateDTO) -> Result<()> {
        let id = dto
            .link_id
            .or(dto.id)
            .ok_or_else(|| Error::Validation(MSG_LINK_ID_REQUIRED.into()))?;
        let existing = match self.repo().find_by_id(id).await? {
            Some(row) => row,
            // Java：主键不存在 → updateById 影响 0 行，不报错
            None => return Ok(()),
        };
        let merged = FriendLinks {
            link_id: existing.link_id,
            link_title: dto.link_title.clone().unwrap_or(existing.link_title),
            link_logo: dto.link_logo.clone().unwrap_or(existing.link_logo),
            link_url: dto.link_url.clone().unwrap_or(existing.link_url),
            sort_order: dto.sort_order.unwrap_or(existing.sort_order),
        };
        self.repo().update(&merged).await?;
        tracing::info!("更新友情链接: id={id}");
        Ok(())
    }

    /// 删除（Java `#del` → `removeById`）。
    ///
    /// # Errors
    /// - `id` 缺失 → `Validation("id不能为空")`
    pub async fn delete(&self, dto: &FriendLinksIdDTO) -> Result<()> {
        let id = dto.id.ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        self.repo().delete(id).await?;
        tracing::info!("删除友情链接: id={id}");
        Ok(())
    }

    /// 更新单字段（Java `BaseServiceImpl#updateField` + 白名单）。
    ///
    /// # Errors
    /// - `id` 缺失 → `Validation("id不能为空")`
    /// - 非白名单字段 → `Validation("field 的值有误")`
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<()> {
        let id = dto.id.ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let raw = dto
            .field
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation(MSG_INVALID_FIELD_VALUE.into()))?;
        let column = normalize_field(raw)
            .ok_or_else(|| Error::Validation(MSG_INVALID_FIELD_VALUE.into()))?;
        let val = dto
            .val
            .as_deref()
            .ok_or_else(|| Error::Validation("值不能为空".into()))?;
        self.repo().update_field(id, column, Some(val)).await?;
        Ok(())
    }

    /// 批量操作（Java `BaseServiceImpl#batch`：仅 `type == "del"`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if !is_delete_batch(dto) {
            return Ok(false);
        }
        let affected = self.repo().delete_many(&dto.ids).await?;
        Ok(affected > 0)
    }
}

/// 实体 → VO（Java `BeanUtils.copyProperties`）。
#[must_use]
fn to_vo(e: &FriendLinks) -> FriendLinksVO {
    FriendLinksVO {
        link_id: e.link_id,
        link_logo: e.link_logo.clone(),
        link_title: e.link_title.clone(),
        link_url: e.link_url.clone(),
        sort_order: e.sort_order,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::content::friend_links::{
        MSG_LINK_TITLE_REQUIRED, MSG_UNSELECTED_ITEM,
    };

    fn link() -> FriendLinks {
        FriendLinks {
            link_id: 2,
            link_title: "官网".into(),
            link_logo: "logo.png".into(),
            link_url: "https://nextrade.com".into(),
            sort_order: 3,
        }
    }

    #[test]
    fn to_vo_maps_all_fields() {
        let vo = to_vo(&link());
        assert_eq!(vo.link_id, 2);
        assert_eq!(vo.link_title, "官网");
        assert_eq!(vo.link_logo, "logo.png");
        assert_eq!(vo.link_url, "https://nextrade.com");
        assert_eq!(vo.sort_order, 3);
    }

    #[test]
    fn create_validation_uses_java_message() {
        let err = validate_create(&FriendLinksCreateDTO::default()).unwrap_err();
        assert_eq!(err, MSG_LINK_TITLE_REQUIRED);
    }

    #[test]
    fn verbatim_messages_are_kept() {
        assert_eq!(MSG_ID_REQUIRED, "id不能为空");
        assert_eq!(MSG_LINK_ID_REQUIRED, "友情链接id不能为空");
        assert_eq!(MSG_INVALID_FIELD_VALUE, "field 的值有误");
        assert_eq!(MSG_UNSELECTED_ITEM, "未选择项目");
        assert_eq!(MSG_LINK_NOT_FOUND, "友情链接不存在");
    }
}
