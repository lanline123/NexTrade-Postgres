//! 语言关联服务（lang::locales_relation）。
//!
//! Java 来源（overseas）：
//! - `com.nextrade.adminapi.controller.lang.LocalesRelationController`
//! - `com.nextrade.service.lang.impl.LocalesRelationServiceImpl`
//!
//! `config` 端点按 Java 返回 `locales_lang` 全表
//! （`localesLangService.list()`），不读 `config` 表。

use std::sync::Arc;

use nextrade_core::entities::LocalesRelation;
use nextrade_core::models::lang::common::{BatchDTO, OperateDTO};
use nextrade_core::models::lang::locales_relation::{LocalesLangVO, LocalesRelationDTO, LocalesRelationVO};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 语言关联服务（经 `AdminState.lang.locales_relation()` 组合提供）。
pub struct LocalesRelationService {
    repos: Arc<Repositories>,
}

impl LocalesRelationService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(
        &self,
    ) -> &nextrade_infrastructure::db::repo::lang::locales_relation::LocalesRelationRepository
    {
        &self.repos.lang.locales_relation
    }

    /// 分页列表（Java `list(BasePage)`，无业务过滤）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn list(
        &self,
        page: Option<u64>,
        size: Option<u64>,
    ) -> Result<PageResult<LocalesRelationVO>> {
        let page = page.unwrap_or(1).max(1);
        let size = size.unwrap_or(15).clamp(1, 100);
        let (total, rows) = self
            .repo()
            .paginate("id ASC", size, (page - 1) * size)
            .await?;
        Ok(PageResult::new(
            total,
            rows.iter().map(to_vo).collect(),
            &PageQuery {
                page,
                page_size: size,
            },
        ))
    }

    /// 详情（Java `detail`）。
    pub async fn detail(&self, id: i32) -> Result<Option<LocalesRelationVO>> {
        Ok(self.repo().find_by_id(id).await?.as_ref().map(to_vo))
    }

    /// 创建（Java `create`）。
    pub async fn create(&self, dto: &LocalesRelationDTO) -> Result<i32> {
        self.repo().insert(&dto_to_row(dto, 0)).await
    }

    /// 修改（Java `update`）。
    pub async fn update(&self, dto: &LocalesRelationDTO) -> Result<bool> {
        let id = dto
            .id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::Validation("id不能为空".into()))?;
        Ok(self.repo().update(&dto_to_row(dto, id)).await? > 0)
    }

    /// 删除（Java `del`）。
    pub async fn del(&self, dto: &OperateDTO) -> Result<bool> {
        let id = dto
            .id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::Validation("id不能为空".into()))?;
        Ok(self.repo().delete_by_ids(&[id]).await? > 0)
    }

    /// 批量操作（Java `batch`：`type=del` 删除）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        let ids = dto.ids.clone().unwrap_or_default();
        match dto.r#type.as_deref() {
            Some("del") => Ok(self.repo().delete_by_ids(&ids).await? > 0),
            _ => Ok(false),
        }
    }

    /// 语言关联配置（Java `config`：`locales_lang` 全表直出）。
    pub async fn config(&self) -> Result<Vec<LocalesLangVO>> {
        let rows = self.repos.lang.locales_lang.find_all().await?;
        Ok(rows
            .into_iter()
            .map(|r| LocalesLangVO {
                id: Some(r.id),
                code: r.code,
                name: r.name,
                name_en: r.name_en,
            })
            .collect())
    }
}

fn dto_to_row(dto: &LocalesRelationDTO, id: i32) -> LocalesRelation {
    LocalesRelation {
        id,
        name: dto.name.clone(),
        code: dto.code.clone(),
        locales_id: dto.locales_id,
    }
}

fn to_vo(r: &LocalesRelation) -> LocalesRelationVO {
    LocalesRelationVO {
        id: Some(r.id),
        name: r.name.clone(),
        code: r.code.clone(),
        locales_id: r.locales_id,
    }
}
