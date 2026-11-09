//! 语言地区维护服务（lang::locales）。
//!
//! Java 来源（overseas）：
//! - `com.nextrade.adminapi.controller.lang.LocalesController`
//! - `com.nextrade.service.lang.impl.LocalesServiceImpl`
//!   （`list` / `detail` / `create` / `update` / `updateField` / `del` /
//!   `batch` / `getLocalesLimit3`）
//!
//! # 未迁移 / 偏差
//!
//! - Java `getLocalesVO` 的 `lastUpdated` 为 `yyyy-MM-dd HH:mm:ss` 文本
//!   （空值取当前时间）；Rust 用 chrono 等价格式化。
//! - `updateField` 的 `isDefault` 联动：Java 先置 `is_default = 0` 再重放
//!   本次更新；Rust 在一个服务方法内顺序执行（无事务，与 Java 一致）。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::Locales;
use nextrade_core::models::lang::common::{BatchDTO, OperateDTO, UpdateFieldDTO, LANG_CRUD_FIELDS};
use nextrade_core::models::lang::locales::{CurrencyEmbedVO, LocalesDTO, LocalesListQuery, LocalesVO};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 语言地区服务（经 `AdminState.lang.locales()` 组合提供）。
pub struct LocalesService {
    repos: Arc<Repositories>,
}

/// `lastUpdated` 展示格式（Java `DateUtil.NORM_DATETIME_PATTERN`）。
const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

impl LocalesService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &nextrade_infrastructure::db::repo::lang::locales::LocalesRepository {
        &self.repos.lang.locales
    }

    /// 分页列表（Java `list`：`isEnabled` 过滤 + 每行补关联货币）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn list(&self, q: &LocalesListQuery) -> Result<PageResult<LocalesVO>> {
        let (page, size) = q.page_params();
        let (total, rows) = self
            .repo()
            .paginate(q.is_enabled, "id ASC", size, (page - 1) * size)
            .await?;
        let currency_ids: Vec<i32> = rows.iter().filter_map(|r| r.currency_id).collect();
        let currencies = self
            .repos
            .lang
            .currency
            .find_by_ids(&currency_ids)
            .await?;
        let currency_map: std::collections::HashMap<i32, _> = currencies
            .into_iter()
            .map(|c| (c.id, currency_embed(&c)))
            .collect();
        let list = rows
            .iter()
            .map(|r| {
                let mut vo = self.to_vo(r);
                if let Some(id) = r.currency_id {
                    vo.currency = currency_map.get(&id).cloned();
                }
                vo
            })
            .collect();
        Ok(PageResult::new(
            total,
            list,
            &PageQuery {
                page,
                page_size: size,
            },
        ))
    }

    /// 详情（Java `detail`：`getById` → VO；缺省返回 `None`）。
    pub async fn detail(&self, id: i32) -> Result<Option<LocalesVO>> {
        Ok(self.repo().find_by_id(id).await?.map(|r| self.to_vo(&r)))
    }

    /// 非中文启用语言前 3 条（Java `getLocalesLimit3`）。
    pub async fn get_locales_limit3(&self) -> Result<Vec<LocalesVO>> {
        let rows = self.repo().find_enabled_non_zh_limit(3).await?;
        Ok(rows.iter().map(|r| self.to_vo(r)).collect())
    }

    /// 创建（Java `create`；`last_updated` 取当前秒）。
    pub async fn create(&self, dto: &LocalesDTO) -> Result<i32> {
        let mut row = dto_to_row(dto);
        row.last_updated = Some(i32::try_from(now_seconds()).unwrap_or(0));
        self.repo().insert(&row).await
    }

    /// 修改（Java `update`：`isDefault=1` 时先清空全部默认）。
    pub async fn update(&self, dto: &LocalesDTO) -> Result<bool> {
        let id = dto
            .id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::Validation("id不能为空".into()))?;
        if dto.is_default == Some(1) {
            self.repo().clear_default().await?;
        }
        let mut row = dto_to_row(dto);
        row.id = id;
        row.last_updated = Some(i32::try_from(now_seconds()).unwrap_or(0));
        Ok(self.repo().update(&row).await? > 0)
    }

    /// 单字段更新（Java `updateField`；`isDefault=1` 联动清空其余默认）。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let id = dto
            .id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::Validation("id不能为空".into()))?;
        let field = dto
            .field
            .as_deref()
            .map(camel_to_column)
            .ok_or_else(|| Error::Validation("field不能为空".into()))?;
        if !LANG_CRUD_FIELDS.contains(&field.as_str()) {
            return Err(Error::Validation(format!("不允许修改字段: {field}")));
        }
        let Some(value) = dto.val.as_deref() else {
            return Err(Error::Validation("val不能为空".into()));
        };
        // `isDefault` 联动（Java：设为默认后全量置 0 再重放本条，
        // 净效果 = 仅本条为默认；这里先清空再设，等价且少一次写）。
        if field == "is_default" && value == "1" {
            self.repo().clear_default().await?;
        }
        Ok(self.repo().update_field(id, &field, value).await? > 0)
    }

    /// 删除（Java `del`）。
    pub async fn del(&self, dto: &OperateDTO) -> Result<bool> {
        let id = dto
            .id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::Validation("id不能为空".into()))?;
        Ok(self.repo().delete_by_ids(&[id]).await? > 0)
    }

    /// 批量操作（Java `batch`：`type=del` 删除，其余 no-op）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        let ids = dto.ids.clone().unwrap_or_default();
        match dto.r#type.as_deref() {
            Some("del") => Ok(self.repo().delete_by_ids(&ids).await? > 0),
            _ => Ok(false),
        }
    }

    /// `Locales` 行 → `LocalesVO`（Java `getLocalesVO`）。
    fn to_vo(&self, row: &Locales) -> LocalesVO {
        LocalesVO {
            id: Some(row.id),
            locale_code: row.locale_code.clone(),
            language: row.language.clone(),
            domain: Some(row.domain.clone()),
            flag_picture: row.flag_picture.clone(),
            last_updated: Some(format_last_updated(row.last_updated)),
            is_enabled: row.is_enabled,
            is_default: row.is_default,
            currency_id: row.currency_id,
            sort: row.sort,
            currency: None,
        }
    }
}

/// `last_updated` 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`（Java 空值取当前时间）。
fn format_last_updated(secs: Option<i32>) -> String {
    let secs = secs.unwrap_or_else(|| i32::try_from(now_seconds()).unwrap_or(0));
    chrono::DateTime::from_timestamp(i64::from(secs), 0)
        .map(|dt| dt.format(DATE_FORMAT).to_string())
        .unwrap_or_default()
}

/// `Currency` 实体 → 嵌入 VO。
fn currency_embed(c: &nextrade_core::entities::Currency) -> CurrencyEmbedVO {
    CurrencyEmbedVO {
        id: Some(c.id),
        name: Some(c.name.clone()),
        symbol: Some(c.symbol.clone()),
        is_default: Some(c.is_default),
        rate: Some(c.rate.clone()),
        code: Some(c.code.clone()),
    }
}

/// `LocalesDTO` → 实体行（`id` 保留入参值）。
fn dto_to_row(dto: &LocalesDTO) -> Locales {
    Locales {
        id: dto.id.unwrap_or(0),
        locale_code: dto.locale_code.clone(),
        language: dto.language.clone(),
        domain: dto.domain.clone().unwrap_or_default(),
        flag_picture: dto.flag_picture.clone(),
        last_updated: None,
        is_enabled: dto.is_enabled,
        is_default: dto.is_default,
        currency_id: dto.currency_id,
        sort: dto.sort,
    }
}

/// 驼峰字段名 → snake_case 列名（前端传 `isDefault`，白名单存列名）。
pub fn camel_to_column(field: &str) -> String {
    let mut out = String::with_capacity(field.len() + 4);
    for ch in field.chars() {
        if ch.is_ascii_uppercase() {
            out.push('_');
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camel_to_column_matches_whitelist_entries() {
        assert_eq!(camel_to_column("isDefault"), "is_default");
        assert_eq!(camel_to_column("isEnabled"), "is_enabled");
        assert_eq!(camel_to_column("sort"), "sort");
    }

    #[test]
    fn last_updated_formats_java_pattern() {
        // 2026-10-24 00:00:00 UTC
        assert_eq!(format_last_updated(Some(1_793_032_771 + 1)).len(), 19);
        assert!(format_last_updated(Some(0)).starts_with("1970-01-01"));
    }
}
