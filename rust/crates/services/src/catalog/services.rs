//! 商品服务域服务（catalog::services）。
//!
//! 对应原 Java `com.nextrade.service.product.impl.ServicesServiceImpl`
//! （96 行，继承 `BaseServiceImpl<ServicesMapper, Services>`）
//! 与 `com.nextrade.adminapi.controller.product.ServicesController`。
//!
//! # 端点 → 方法
//!
//! | 端点（挂载于 `/adminapi`） | 原 Java | 方法 |
//! |---------------------------|---------|------|
//! | `GET  /product/productServices/list` | `ServicesController.list` | [`ServicesService::list`] |
//! | `GET  /product/productServices/detail` | `ServicesController.detail` | [`ServicesService::detail`] |
//! | `POST /product/productServices/create` | `ServicesController.create` | [`ServicesService::create`] |
//! | `POST /product/productServices/update` | `ServicesController.update` | [`ServicesService::update`] |
//! | `POST /product/productServices/del` | `ServicesController.del` | [`ServicesService::delete`] |
//! | `POST /product/productServices/batch` | `ServicesController.batch` | [`ServicesService::batch`] |
//! | `POST /product/productServices/updateField` | `ServicesController.updateField` | [`ServicesService::update_field`] |
//!
//! # 语义（逐条对照 Java 源）
//!
//! - `list`：`keyword != null` 时 `LIKE product_service_name`；排序走
//!   `BaseServiceImpl#buildSortOrder` → 空 `sortField` 视为主键、
//!   `desc` / 空 `sortOrder` 一律降序（默认 `product_service_id DESC`）。
//! - `detail`：`getById`，不存在抛 `商品服务不存在`。
//! - `create`：`save(convertToBean(dto))` —— DTO 为 `null` 的列不写入，
//!   由 `product_services` 的 DDL 默认值兜底（`sort_order=50`、
//!   `default_on=0`、`shop_id=0`、文本列 `''`）。
//! - `update`：`updateById(convertToBean(dto))` —— `NOT_NULL` 策略，
//!   `null` 列不进入 SET（保留库中原值）；`id` 不存在时 Java 静默返回
//!   `false`（本实现返回 `Ok(false)`，不抛错）。
//! - `del`：`BaseServiceImpl#del` → `removeById`（物理删除，无软删列）。
//! - `batch`：`BaseServiceImpl#batch` —— 仅 `type == "del"` 时
//!   `removeByIds`，其余类型返回 `false`。
//! - `updateField`：`BaseServiceImpl#updateField` —— 属性名归一后按
//!   [`SERVICES_FIELDS`] 白名单校验，非法字段抛 `field 的值有误`。
//!
//! # 与 Java 的差异（`uncertain_points`）
//!
//! 1. **校验注解在 Java 里实际未生效**（控制器 `@RequestBody` 缺 `@Valid`），
//!    本实现按注解本意校验名称必填且 ≤100 字，文案逐字符照抄
//!    `ServicesConstants`；详见 `nextrade_core::models::catalog::services`
//!    模块头差异第 1 条。
//! 2. **`del` / `batch` 的缺失参数**：Java 只返回 `false`（控制器 `void`
//!    → HTTP 200 空 `data`），本实现同样不抛业务错误。
//! 3. **分页返回结构**：Java `Page<ServicesDTO>` → Rust `PageResult`。
//! 4. `updateField` 不按店铺过滤（与 Java 通用实现一致）。
//!
//! # 金额
//!
//! 本域不涉及任何金额列，故无 `rust_decimal::Decimal`。

use std::sync::Arc;

use nextrade_core::entities::ProductServices;
use nextrade_core::models::catalog::services::{
    normalize_field, validate_service_name, BatchDTO, OperateDTO, ServicesDTO, ServicesListQuery,
    UpdateFieldDTO, DEFAULT_ON, DEFAULT_SHOP_ID, DEFAULT_SORT_ORDER, MSG_FIELD_NAME_REQUIRED,
    MSG_FIELD_VALUE_REQUIRED, MSG_ID_REQUIRED, MSG_INVALID_FIELD_VALUE, MSG_SERVICE_NOT_FOUND,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::catalog::services::ServicesRepository;
use nextrade_infrastructure::db::Repositories;

/// 管理端每页条数缺省值（Java `BasePage#getSize()` 默认 15）。
pub const DEFAULT_PAGE_SIZE: u64 = 15;

/// 管理端每页条数上限（Java `BasePage#getSize()` 上限 100）。
pub const MAX_PAGE_SIZE: u64 = 100;

/// 商品服务域服务（挂载于 `AdminState.catalog.services`）。
pub struct ServicesService {
    repos: Arc<Repositories>,
}

impl ServicesService {
    /// 构造（与同域其它 feature 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &ServicesRepository {
        &self.repos.catalog.services
    }

    // ————————————————————— 查询 —————————————————————

    /// 商品服务分页列表（对应 Java `ServicesServiceImpl#list`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn list(&self, q: &ServicesListQuery) -> Result<PageResult<ServicesDTO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
        };
        let (total, rows) = self
            .repo()
            .paginate(
                q.keyword.as_deref(),
                q.sort_field.as_deref(),
                q.sort_order.as_deref(),
                &page,
            )
            .await?;
        let list: Vec<ServicesDTO> = rows.iter().map(to_dto).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 商品服务详情（对应 Java `ServicesServiceImpl#detail`）。
    ///
    /// 不存在 → `商品服务不存在`（Java `GlobalException`）。
    ///
    /// # Errors
    /// - 不存在 → `Error::biz(500, 商品服务不存在)`
    /// - 数据库执行失败 → `Error::Database`
    pub async fn detail(&self, id: i32) -> Result<ServicesDTO> {
        let s = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_SERVICE_NOT_FOUND))?;
        Ok(to_dto(&s))
    }

    // ————————————————————— 写操作 —————————————————————

    /// 新增商品服务（对应 Java `ServicesServiceImpl#create`）。
    ///
    /// Java 返回 `save(...)` 的布尔值；本实现同样返回 `bool`。
    ///
    /// # Errors
    /// - 名称校验失败 → `Error::Validation(<Java 注解文案>)`
    /// - 数据库执行失败 → `Error::Database`
    pub async fn create(&self, dto: &ServicesDTO) -> Result<bool> {
        let name = validate_service_name(dto.product_service_name.as_deref())
            .map_err(Error::Validation)?;
        let entity = create_entity(dto, name);
        let id = self.repo().insert(&entity).await?;
        tracing::info!("新增商品服务: id={id}, name={}", entity.product_service_name);
        Ok(true)
    }

    /// 编辑商品服务（对应 Java `ServicesServiceImpl#update`）。
    ///
    /// Java `updateById` 的 `NOT_NULL` 语义：DTO 中为 `null` 的列保留库中
    /// 原值。`id` 缺失或记录不存在时 Java 静默返回 `false`（不抛错）。
    ///
    /// # Errors
    /// - 名称校验失败 → `Error::Validation(<Java 注解文案>)`
    /// - 数据库执行失败 → `Error::Database`
    pub async fn update(&self, dto: &ServicesDTO) -> Result<bool> {
        let Some(id) = dto.product_service_id else {
            // Java `updateById(null id)` → false，控制器 void → 200
            return Ok(false);
        };
        let name = validate_service_name(dto.product_service_name.as_deref())
            .map_err(Error::Validation)?;
        let Some(existing) = self.repo().find_by_id(id).await? else {
            // Java `updateById` 影响 0 行 → false，不抛异常
            return Ok(false);
        };
        let entity = merge_update(dto, name, &existing);
        let updated = self.repo().update(&entity).await?;
        tracing::info!("编辑商品服务: id={id}, updated={updated}");
        Ok(updated)
    }

    /// 删除商品服务（对应 Java `ServicesController.del`
    /// → `BaseServiceImpl#del` → `removeById`）。
    ///
    /// `id` 缺失时 Java 返回 `false`（控制器 `void` → 200），本实现同。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn delete(&self, dto: &OperateDTO) -> Result<bool> {
        let Some(id) = dto.id else {
            return Ok(false);
        };
        let deleted = self.repo().delete(id).await?;
        tracing::info!("删除商品服务: id={id}, deleted={deleted}");
        Ok(deleted)
    }

    /// 批量操作（对应 Java `BaseServiceImpl#batch`）。
    ///
    /// 仅 `type == "del"` 时物理删除；其余类型返回 `false`（Java 行为）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if !dto.r#type.as_deref().is_some_and(|t| t == "del") {
            // Java：非 del 类型直接 return false
            return Ok(false);
        }
        let Some(ids) = dto.ids.as_deref() else {
            return Ok(false);
        };
        let affected = self.repo().delete_batch(ids).await?;
        tracing::info!("批量删除商品服务: {} 条", affected);
        Ok(affected > 0)
    }

    /// 更新单个字段（对应 Java `ServicesController.updateField`
    /// → `ServicesServiceImpl#updateField(dto, SERVICES_FIELDS)`
    /// → `BaseServiceImpl#updateField`）。
    ///
    /// # Errors
    /// - 缺 `id` / `field` / `val` → `Error::Validation(<Java 注解文案>)`
    /// - 字段不在白名单 → `Error::Validation(field 的值有误)`
    /// - 数据库执行失败 → `Error::Database`
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let field = dto
            .field
            .as_deref()
            .ok_or_else(|| Error::Validation(MSG_FIELD_NAME_REQUIRED.into()))?;
        let val = dto
            .val
            .as_deref()
            .ok_or_else(|| Error::Validation(MSG_FIELD_VALUE_REQUIRED.into()))?;
        let column = normalize_field(field)
            .ok_or_else(|| Error::Validation(MSG_INVALID_FIELD_VALUE.into()))?;
        let updated = self.repo().update_field(id, column, val).await?;
        tracing::info!("更新商品服务字段: id={id}, {column}={val}, updated={updated}");
        Ok(updated)
    }
}

/// 实体 → DTO（Java `BeanUtils.copyProperties(services, dto)`）。
#[must_use]
fn to_dto(s: &ProductServices) -> ServicesDTO {
    ServicesDTO {
        product_service_id: Some(s.product_service_id),
        product_service_name: Some(s.product_service_name.clone()),
        product_service_desc: Some(s.product_service_desc.clone()),
        ico_img: Some(s.ico_img.clone()),
        sort_order: Some(s.sort_order),
        default_on: Some(s.default_on),
        shop_id: Some(s.shop_id),
    }
}

/// `create` 落库实体：DTO 未提供的列按 DDL 默认值补齐
/// （Java `save` + MyBatis-Plus `NOT_NULL` 策略的等价物）。
#[must_use]
fn create_entity(dto: &ServicesDTO, name: String) -> ProductServices {
    ProductServices {
        product_service_id: 0,
        product_service_name: name,
        // DDL: varchar NOT NULL DEFAULT ''
        product_service_desc: dto.product_service_desc.clone().unwrap_or_default(),
        ico_img: dto.ico_img.clone().unwrap_or_default(),
        // DDL: smallint unsigned NOT NULL DEFAULT '50'
        sort_order: dto.sort_order.unwrap_or(DEFAULT_SORT_ORDER),
        // DDL: tinyint unsigned NOT NULL DEFAULT '0'
        default_on: dto.default_on.unwrap_or(DEFAULT_ON),
        // DDL: int unsigned NOT NULL DEFAULT '0'
        shop_id: dto.shop_id.unwrap_or(DEFAULT_SHOP_ID),
    }
}

/// `update` 落库实体：DTO 传了就用 DTO，没传就保留旧值
/// （Java `updateById` 的 `FieldStrategy.NOT_NULL` 语义）。
#[must_use]
fn merge_update(dto: &ServicesDTO, name: String, existing: &ProductServices) -> ProductServices {
    ProductServices {
        product_service_id: existing.product_service_id,
        product_service_name: name,
        product_service_desc: dto
            .product_service_desc
            .clone()
            .unwrap_or_else(|| existing.product_service_desc.clone()),
        ico_img: dto.ico_img.clone().unwrap_or_else(|| existing.ico_img.clone()),
        sort_order: dto.sort_order.unwrap_or(existing.sort_order),
        default_on: dto.default_on.unwrap_or(existing.default_on),
        shop_id: dto.shop_id.unwrap_or(existing.shop_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity() -> ProductServices {
        ProductServices {
            product_service_id: 7,
            product_service_name: "包邮".into(),
            product_service_desc: "全国包邮".into(),
            ico_img: "https://oss.example/i.png".into(),
            sort_order: 50,
            default_on: 1,
            shop_id: 0,
        }
    }

    /// 实体 → DTO 逐字段映射（含 camelCase 序列化）。
    #[test]
    fn to_dto_maps_all_fields() {
        let dto = to_dto(&entity());
        assert_eq!(dto.product_service_id, Some(7));
        assert_eq!(dto.product_service_name.as_deref(), Some("包邮"));
        assert_eq!(dto.product_service_desc.as_deref(), Some("全国包邮"));
        assert_eq!(dto.ico_img.as_deref(), Some("https://oss.example/i.png"));
        assert_eq!(dto.sort_order, Some(50));
        assert_eq!(dto.default_on, Some(1));
        assert_eq!(dto.shop_id, Some(0));

        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["productServiceId"], 7);
        assert_eq!(json["defaultOn"], 1);
    }

    /// `create` 的默认值对齐 `product_services` DDL（未传字段不能落 0）。
    #[test]
    fn create_entity_defaults_match_ddl() {
        let dto = ServicesDTO {
            product_service_name: Some(" 包邮 ".into()),
            ..Default::default()
        };
        let name = validate_service_name(dto.product_service_name.as_deref()).unwrap();
        let e = create_entity(&dto, name);
        // 文本列 DDL DEFAULT ''
        assert_eq!(e.product_service_desc, "");
        assert_eq!(e.ico_img, "");
        // 数值列 DDL 默认值
        assert_eq!(e.sort_order, 50);
        assert_eq!(e.default_on, 0);
        assert_eq!(e.shop_id, 0);
        // 名称去首尾空白
        assert_eq!(e.product_service_name, "包邮");
        assert_eq!(e.product_service_id, 0);
    }

    /// `update` 的 null 跳过语义（MyBatis-Plus NOT_NULL）：
    /// 未传列必须保留原值，不能清空。
    #[test]
    fn merge_update_keeps_existing_when_dto_field_absent() {
        let existing = entity();
        let dto = ServicesDTO {
            product_service_id: Some(7),
            product_service_name: Some("新名字".into()),
            ..Default::default()
        };
        let name = validate_service_name(dto.product_service_name.as_deref()).unwrap();
        let merged = merge_update(&dto, name, &existing);
        assert_eq!(merged.product_service_name, "新名字");
        // 未传的列全部保留旧值
        assert_eq!(merged.product_service_desc, "全国包邮");
        assert_eq!(merged.ico_img, "https://oss.example/i.png");
        assert_eq!(merged.sort_order, 50);
        assert_eq!(merged.default_on, 1);
        assert_eq!(merged.shop_id, 0);
        assert_eq!(merged.product_service_id, 7);

        // 显式传值则覆盖（含 0）
        let dto2 = ServicesDTO {
            product_service_id: Some(7),
            product_service_name: Some("包邮".into()),
            sort_order: Some(0),
            default_on: Some(0),
            product_service_desc: Some(String::new()),
            ..Default::default()
        };
        let name2 = validate_service_name(dto2.product_service_name.as_deref()).unwrap();
        let merged2 = merge_update(&dto2, name2, &existing);
        assert_eq!(merged2.sort_order, 0);
        assert_eq!(merged2.default_on, 0);
        assert_eq!(merged2.product_service_desc, "");
    }

    /// 分页默认值与 Java `BasePage` 一致（15 / 100）。
    #[test]
    fn page_defaults_match_base_page() {
        assert_eq!(DEFAULT_PAGE_SIZE, 15);
        assert_eq!(MAX_PAGE_SIZE, 100);
        let q = ServicesListQuery::default();
        assert_eq!(q.page, None);
        assert_eq!(q.page_size, None);
        assert_eq!(q.page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE), 15);
        assert_eq!(Some(500_u64).unwrap_or(15).min(MAX_PAGE_SIZE), 100);
    }
}
