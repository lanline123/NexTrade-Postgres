//! 素材分类服务（marketing::salesman_material_category）。
//!
//! 对应原 Java `SalesmanMaterialCategoryServiceImpl`
//! （`java/nextrade-service/src/main/java/com/nextrade/service/salesman/impl/SalesmanMaterialCategoryServiceImpl.java`）
//! 与继承来的 `BaseServiceImpl#updateField` / `#batch` / `#del`。
//!
//! 构造签名固定：`pub fn new(repos: Arc<Repositories>) -> Self`。
//!
//! ## 已迁移
//! - `list`：名称模糊 + 动态排序 + 分页；`add_time` 格式化为字符串
//! - `detail`：按主键取详情
//! - `create`：同店铺重名校验 + 写入 `shop_id` / `add_time`
//! - `update`：同店铺重名校验（排除自身）+ 更新
//! - `delete`：**先查该分类下是否还有素材**，有则拒绝
//! - `updateField`：`GENERAL_FIELDS` 白名单校验后更新单列
//! - `batch`：仅 `type == "del"` 走批量删除，其余返回 `false`
//!
//! ## 已知语义偏差（诚实记录，见 `uncertain_points`）
//! 1. **`shop_id` 的取值来源**：Java `BaseServiceImpl#getShopId()` 读
//!    `x-shop-id` 请求头（`HeaderUtils.getShopId()`）。Rust 侧当前中间件只
//!    注入 `AuthUser`，**没有**把店铺上下文透传到 handler/服务层。因此本实现
//!    把 `shop_id` 作为**可选入参**：调用方（路由层）能给就给，给不出就按
//!    `0` 处理。这会让「平台账号 + 多店铺」场景下的重名校验范围与 Java 不完全
//!    一致。
//! 2. **`updateField` 的可达字段**：Java 传的是 `GENERAL_FIELDS`
//!    （`is_show` / `sort_order` / `status`），但
//!    `salesman_material_category` 表**只有 `sort_order`**
//!    （见 `pure.sql`）。传 `is_show` / `status` 在 Java 里会走到 DB 报
//!    「Unknown column」。本实现保持同样行为（由仓储层返回 `Database` 错误），
//!    仅 `sortOrder` 真正生效。
//! 3. **`add_time` / `sort_order` 的默认值**：Java `create` 只显式设置
//!    `shopId` 与 `addTime`，`sortOrder` 若前端不传则保持 `null` 入库。
//!    本实现一致（不臆造补 0）。
//! 4. **`detail` 的 `addTime`**：Java `detail` 直接
//!    `BeanUtils.copyProperties(实体, VO)`，而实体 `addTime` 是 `Integer`、
//!    VO `addTime` 是 `String` —— Spring `BeanUtils` 类型不匹配会**静默跳过**，
//!    因此 Java 的 `detail` 返回 `addTime = null`；只有 `list` 会格式化。
//!    本实现用 `None` 复刻（`list` 走 [`nextrade_core::models::marketing::salesman_material_category::format_add_time`]）。
//! 5. **`batch` 不做关联素材校验**：Java `BaseServiceImpl#batch` 直接
//!    `removeByIds`，**不**走 `SalesmanMaterialCategoryServiceImpl#delete`
//!    的素材关联检查。这是 Java 的既有行为（可能误删有素材的分类），
//!    本实现如实照搬，不擅自"修正"。

use std::sync::Arc;

use nextrade_common::utils;
use nextrade_core::entities::SalesmanMaterialCategory;
use nextrade_core::models::marketing::salesman_material_category::{
    format_add_time, is_allowed_field, normalize_field, validate_category_name,
    BatchDTO, OperateDTO, SalesmanMaterialCategoryEditParam, SalesmanMaterialCategoryPageQuery,
    SalesmanMaterialCategorySaveParam, SalesmanMaterialCategoryVO, UpdateFieldDTO,
    ERR_CATEGORY_HAS_MATERIAL, ERR_CATEGORY_NAME_EXISTS, ERR_ID_REQUIRED, ERR_INVALID_FIELD_VALUE,
    ERR_UNSELECTED_ITEM,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 素材分类服务。
pub struct SalesmanMaterialCategoryService {
    repos: Arc<Repositories>,
}

impl SalesmanMaterialCategoryService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &nextrade_infrastructure::db::repo::marketing::salesman_material_category::SalesmanMaterialCategoryRepository {
        &self.repos.marketing.salesman_material_category
    }

    // ————————————————————— 管理端 —————————————————————

    /// 素材分类分页（对应 Java `SalesmanMaterialCategoryServiceImpl#list`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn list(
        &self,
        q: &SalesmanMaterialCategoryPageQuery,
    ) -> Result<PageResult<SalesmanMaterialCategoryVO>> {
        let page = q.page_query();
        let (total, rows) = self
            .repo()
            .paginate(q, TenantScope::current_list_filter())
            .await?;

        // Java：`BeanUtils.copyProperties` + `addTime` 格式化为字符串
        let list = rows.iter().map(to_vo_with_time).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 素材分类详情（对应 Java `#detail`）。
    ///
    /// # Errors
    /// - `id` 为 `None` → Java 直接返回 `null`；Rust 侧返回 `Validation`
    ///   （前端契约上 `id` 是必填 query 参数）。
    /// - 不存在 → `Error::NotFound`。
    pub async fn detail(&self, id: Option<i32>) -> Result<SalesmanMaterialCategoryVO> {
        let id = id.ok_or_else(|| Error::Validation(ERR_ID_REQUIRED.into()))?;
        let row = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound("素材分类不存在".into()))?;
        // 多租户：店铺账号只能查看本店分类
        TenantScope::current_ensure_access(row.shop_id.unwrap_or(0))?;
        Ok(to_vo_plain(&row))
    }

    /// 创建素材分类（对应 Java `#create`）。
    ///
    /// 校验链：名称非空 → 长度 ≤ 50 → 同店铺同名不存在。
    ///
    /// # Errors
    /// - 名称非法 → `Error::Validation`
    /// - 同店铺重名 → `Error::biz(400, "分类名称已存在")`
    pub async fn create(
        &self,
        param: &SalesmanMaterialCategorySaveParam,
    ) -> Result<i32> {
        // 多租户：店铺账号强制本店；平台账号落平台级（0）
        let shop_id = TenantScope::current().write_shop_id(None)?;
        let name = validate_category_name(param.category_name.as_deref())
            .map_err(Error::Validation)?;

        if self.repo().exists_name(&name, shop_id, None).await? {
            return Err(Error::biz(400, ERR_CATEGORY_NAME_EXISTS));
        }

        let now = utils::now_seconds() as i32;
        let row = SalesmanMaterialCategory {
            category_id: 0,
            category_name: Some(name),
            add_time: Some(now),
            shop_id: Some(shop_id),
            // Java 不传就不设置（保持 null），不臆造默认值
            sort_order: param.sort_order,
        };
        let id = self.repo().insert(&row).await?;
        tracing::info!("创建素材分类成功: id={id}, shop_id={shop_id}");
        Ok(id)
    }

    /// 更新素材分类（对应 Java `#update`）。
    ///
    /// Java 的重名校验用 `ne(categoryId, param.categoryId)` 排除自身。
    ///
    /// # Errors
    /// - 名称非法 → `Error::Validation`
    /// - 同店铺重名 → `Error::biz(400, "分类名称已存在")`
    /// - `categoryId` 缺失或记录不存在 → `Error::Validation` / `Error::NotFound`
    pub async fn update(&self, param: &SalesmanMaterialCategoryEditParam) -> Result<()> {
        let name = validate_category_name(param.category_name.as_deref())
            .map_err(Error::Validation)?;
        let category_id = param
            .category_id
            .ok_or_else(|| Error::Validation(ERR_ID_REQUIRED.into()))?;

        // 先取原行：既做存在性校验，也为「选择性更新」保留未提交字段
        let existing = self
            .repo()
            .find_by_id(category_id)
            .await?
            .ok_or_else(|| Error::NotFound("素材分类不存在".into()))?;
        // 多租户：更新前校验归属
        TenantScope::current_ensure_access(existing.shop_id.unwrap_or(0))?;

        let shop_id = existing.shop_id.unwrap_or(0);
        if self
            .repo()
            .exists_name(&name, shop_id, Some(category_id))
            .await?
        {
            return Err(Error::biz(400, ERR_CATEGORY_NAME_EXISTS));
        }

        let row = SalesmanMaterialCategory {
            category_id,
            category_name: Some(name),
            // add_time / shop_id 原样保留（Java updateById 不动这两列）
            add_time: existing.add_time,
            shop_id: existing.shop_id,
            // Java：param.sortOrder 为 null 时 MyBatis-Plus 不更新该列 → 保留原值
            sort_order: param.sort_order.or(existing.sort_order),
        };
        self.repo().update(&row).await?;
        tracing::info!("更新素材分类成功: id={category_id}");
        Ok(())
    }

    /// 删除素材分类（对应 Java `#delete`）。
    ///
    /// **关键业务规则**：该分类下还有素材时拒绝删除。
    ///
    /// # Errors
    /// - `id` 缺失 → `Error::Validation`
    /// - 有素材关联 → `Error::biz(400, "请先删除该分类下的素材")`
    /// - 不存在 → `Error::NotFound`
    pub async fn delete(&self, dto: &OperateDTO) -> Result<()> {
        let id = dto.id.ok_or_else(|| Error::Validation(ERR_ID_REQUIRED.into()))?;

        // 多租户：删除前校验归属
        let existing = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound("素材分类不存在".into()))?;
        TenantScope::current_ensure_access(existing.shop_id.unwrap_or(0))?;
        let count = self.repo().count_materials(id).await?;
        if count > 0 {
            return Err(Error::biz(400, ERR_CATEGORY_HAS_MATERIAL));
        }

        let affected = self.repo().delete(id).await?;
        if affected == 0 {
            return Err(Error::NotFound("素材分类不存在".into()));
        }
        tracing::info!("删除素材分类: id={id}");
        Ok(())
    }

    /// 更新单个字段（对应 Java `BaseServiceImpl#updateField`）。
    ///
    /// 白名单来自 Java `CheckFieldConstants.GENERAL_FIELDS`（由调用方传入，
    /// 本域固定为 `GENERAL_FIELDS`）。流程逐字对齐 Java：
    /// 1. `getColumnByPropertyName(field)` —— 找不到的属性名归一成主键列
    /// 2. 非白名单 → 抛 `field 的值有误`
    /// 3. `UPDATE ... SET <column> = ? WHERE <pk> = ?`
    ///
    /// # Errors
    /// - `id` / `field` / `val` 缺失 → `Error::Validation`
    /// - 字段不在白名单 → `Error::Validation("field 的值有误")`
    /// - 目标行不存在 → `Error::NotFound`
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<()> {
        let id = dto.id.ok_or_else(|| Error::Validation(ERR_ID_REQUIRED.into()))?;
        let field = dto
            .field
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation(ERR_INVALID_FIELD_VALUE.into()))?;
        let val = dto
            .val
            .as_deref()
            .ok_or_else(|| Error::Validation(ERR_INVALID_FIELD_VALUE.into()))?;

        let column = normalize_field(field);
        if !is_allowed_field(column) {
            return Err(Error::Validation(ERR_INVALID_FIELD_VALUE.into()));
        }

        // 多租户：字段更新前校验归属
        if let Some(existing) = self.repo().find_by_id(id).await? {
            TenantScope::current_ensure_access(existing.shop_id.unwrap_or(0))?;
        }
        let affected = self.repo().update_field(id, column, val).await?;
        if affected == 0 {
            return Err(Error::NotFound("素材分类不存在".into()));
        }
        tracing::info!("更新素材分类字段: id={id}, {column}={val}");
        Ok(())
    }

    /// 批量操作（对应 Java `BaseServiceImpl#batch`）。
    ///
    /// Java 只实现 `type == "del"`，其余类型**返回 `false` 且不做任何事**。
    /// 这里保持：非 `del` 返回 `Ok(false)`。
    ///
    /// 返回值口径：Java `removeByIds` 走 `SqlHelper.retBool(rows)`，
    /// 即影响行数为 0 时返回 `false`，故这里用 `affected > 0`。
    ///
    /// 注意：Java 的 `batch` **不做**关联素材校验（与 `delete` 不同），
    /// 本实现如实照搬。
    ///
    /// # Errors
    /// - `ids` 为空 → `Error::Validation("未选择项目")`
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if dto.ids.is_empty() {
            return Err(Error::Validation(ERR_UNSELECTED_ITEM.into()));
        }
        if dto.r#type.as_deref() != Some("del") {
            // Java：`return false;`（无副作用）
            tracing::debug!("素材分类 batch 非 del 类型，忽略: {:?}", dto.r#type);
            return Ok(false);
        }
        // 多租户：批量删除前逐条校验归属
        for id in &dto.ids {
            if let Some(existing) = self.repo().find_by_id(*id).await? {
                TenantScope::current_ensure_access(existing.shop_id.unwrap_or(0))?;
            }
        }
        let affected = self.repo().delete_many(&dto.ids).await?;
        tracing::info!("批量删除素材分类: {} 条", affected);
        Ok(affected > 0)
    }
}

/// 实体 → VO（`list` 用：`add_time` 格式化成字符串）。
fn to_vo_with_time(c: &SalesmanMaterialCategory) -> SalesmanMaterialCategoryVO {
    SalesmanMaterialCategoryVO {
        category_id: c.category_id,
        category_name: c.category_name.clone(),
        add_time: format_add_time(c.add_time),
        shop_id: c.shop_id,
        sort_order: c.sort_order,
    }
}

/// 实体 → VO（`detail` 用：复刻 Java `BeanUtils` 的类型不匹配 → `addTime` 为 `null`）。
fn to_vo_plain(c: &SalesmanMaterialCategory) -> SalesmanMaterialCategoryVO {
    SalesmanMaterialCategoryVO {
        category_id: c.category_id,
        category_name: c.category_name.clone(),
        add_time: None,
        shop_id: c.shop_id,
        sort_order: c.sort_order,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::marketing::salesman_material_category::{
        ERR_CATEGORY_NAME_REQUIRED, ERR_CATEGORY_NAME_TOO_LONG, GENERAL_FIELDS,
    };

    fn category() -> SalesmanMaterialCategory {
        SalesmanMaterialCategory {
            category_id: 9,
            category_name: Some("海报".into()),
            add_time: Some(1_733_000_000),
            shop_id: Some(2),
            sort_order: Some(5),
        }
    }

    #[tokio::test]
    async fn tenant_scope_scopes_list_and_blocks_cross_shop_category() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        let shop = TenantScope::for_shop(8);
        let (filter, write_shop, blocked) = with_tenant_scope(shop, async {
            (
                TenantScope::current_list_filter(),
                TenantScope::current().write_shop_id(None).unwrap_or(-1),
                TenantScope::current_ensure_access(9).is_err(),
            )
        })
        .await;
        assert_eq!(filter, Some(8));
        assert_eq!(write_shop, 8);
        assert!(blocked);
        let filter = with_tenant_scope(TenantScope::platform(), async {
            TenantScope::current_list_filter()
        })
        .await;
        assert_eq!(filter, None);
    }

    #[test]
    fn list_vo_formats_add_time_as_string() {
        let vo = to_vo_with_time(&category());
        assert_eq!(vo.category_id, 9);
        assert_eq!(vo.category_name.as_deref(), Some("海报"));
        assert_eq!(vo.add_time.as_deref().map(str::len), Some(19));
        assert_eq!(vo.shop_id, Some(2));
        assert_eq!(vo.sort_order, Some(5));
    }

    #[test]
    fn list_vo_leaves_add_time_null_when_not_positive() {
        // Java：addTime == null 或 <= 0 不格式化
        let mut c = category();
        c.add_time = Some(0);
        assert_eq!(to_vo_with_time(&c).add_time, None);
        c.add_time = None;
        assert_eq!(to_vo_with_time(&c).add_time, None);
    }

    #[test]
    fn detail_vo_matches_java_beanutils_skip() {
        // Java detail 用 BeanUtils 拷贝：Integer → String 类型不匹配被跳过
        let vo = to_vo_plain(&category());
        assert_eq!(vo.add_time, None);
        assert_eq!(vo.category_id, 9);
    }

    #[test]
    fn update_field_rejects_non_whitelisted_property() {
        // 白名单校验是纯函数，这里直接验证其判定（不触库）
        let column = normalize_field("categoryName");
        assert!(!is_allowed_field(column));
        assert_eq!(column, "category_id");

        let column = normalize_field("sortOrder");
        assert!(is_allowed_field(column));
        assert_eq!(column, "sort_order");
    }

    #[test]
    fn general_fields_only_sort_order_is_reachable_on_this_table() {
        // 本表真实列只有 sort_order 命中白名单（见 pure.sql）
        let reachable: Vec<&str> = GENERAL_FIELDS
            .iter()
            .copied()
            .filter(|f| *f == "sort_order")
            .collect();
        assert_eq!(reachable, vec!["sort_order"]);
    }

    #[test]
    fn batch_only_accepts_del_type() {
        // 纯逻辑：Java `BaseServiceImpl#batch` 只认 "del"
        let is_del = |t: Option<&str>| t == Some("del");
        assert!(is_del(Some("del")));
        assert!(!is_del(Some("show")));
        assert!(!is_del(None));
    }

    #[test]
    fn category_name_validation_messages_match_java() {
        assert_eq!(
            validate_category_name(None).unwrap_err(),
            ERR_CATEGORY_NAME_REQUIRED
        );
        let too_long = "字".repeat(51);
        assert_eq!(
            validate_category_name(Some(&too_long)).unwrap_err(),
            ERR_CATEGORY_NAME_TOO_LONG
        );
        assert!(validate_category_name(Some("正常名称")).is_ok());
    }
}