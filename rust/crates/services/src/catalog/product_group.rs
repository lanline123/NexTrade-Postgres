//! 商品分组域服务：分组 CRUD / 字段更新 / 批量操作 / 前台选择器列表。
//! 对应原 Java `ProductGroupServiceImpl`
//! （`com.nextrade.service.product.impl`，管理端 8 个端点背后的 8 个方法）。
//!
//! 迁移要点（逐条对照 Java 源）：
//! - `list`：**恒定** `shop_id` 归属过滤（`eq(shopId, getShopId())`）+
//!   `keyword` 名称模糊 + `productGroupIds` 逗号串 `IN` + 排序；
//!   VO 把 `product_ids` JSON 文本还原为数组、`add_time` 格式化为字符串。
//! - `detail`：`getById`；Java 对不存在的 ID 会在
//!   `BeanUtils.copyProperties(null, ...)` 处抛 `IllegalArgumentException`
//!   （全局 500，无业务文案）。本实现返回业务 500 文案「商品分组不存在」。
//! - `create`：`BeanUtils.copyProperties(createDTO, entity)` + `setProductIds`
//!   （JSON 文本）+ `setAddTime(now)` + `setShopId(请求头值)` → `save`。
//! - `update`：`BeanUtils.copyProperties(updateDTO, entity)` +
//!   `setProductIds`（**空列表也写 `"[]"`**）→ `updateById`（NOT_NULL 策略：
//!   未传字段不进 `SET`；`shopId` 若传了则覆盖）。
//! - `del`：`removeById`；Java `OperateDTO.id` 为 null 时 `BaseServiceImpl#del`
//!   直接返回 `false`（不发 SQL）——本实现同样静默成功。
//! - `updateField`：见下。
//! - `batch`：`BaseServiceImpl#batch` —— 仅 `type == "del"` 时 `removeByIds`，
//!   其它/缺失类型返回 `false`；`ids` 为空时 `removeByIds` 也返回 `false`。
//! - `clientList`：`lambdaQuery().like(name).list()`，**无店铺过滤、无分页**。
//!
//! # `updateField` 为什么总是失败（重要）
//!
//! `ProductGroupController#updateField` 传入
//! `CheckFieldConstants.GENERAL_FIELDS = {"is_show","sort_order","status"}`，
//! 但 `product_group` 表**没有**这三列。Java `getColumnByPropertyName`
//! 找不到属性时回退主键列 `product_group_id`，白名单校验必然失败，
//! 于是抛 `GlobalException("field 的值有误")`。
//! 这是现网 Java 的真实行为，本实现原样复刻（不做「善意修正」）。
//!
//! # 未迁移 / 不确定点
//!
//! - Java 控制器类级 `@PreAuthorize("@pms.hasPermission('productGroupManage')")`
//!   在 Rust 侧尚未实现（只校验登录态）。
//! - `create` / `update` 的 DTO 校验注解（`@Size` 等）因控制器**未加 `@Valid`**
//!   在 Java 中并不生效，本实现同样不校验名称非空/长度。
//! - `shop_id`：Java 用 `HeaderUtils.getShopId()`（`admin` ⇒ 0，
//!   `shop`/`pos` ⇒ `x-shop-id`，缺失 ⇒ `null`）；Rust 路由层解析后传入，
//!   缺失/未知类型统一按 `0` 处理（与 attributes_tpl 等域一致）。
//! - `updateField` 的错误码：Java `GlobalException` → 业务 500；本实现
//!   `Error::biz(500, "field 的值有误")` 保持同样业务码。
//! - 列表分页外壳用项目统一的 `PageResult`（Java 为 MyBatis `Page`）。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::models::catalog::product_group::{
    group_to_vo, product_ids_to_json, BatchDTO, OperateDTO, ProductGroupCreateDTO,
    ProductGroupListQuery, ProductGroupUpdateDTO, ProductGroupVO, UpdateFieldDTO,
    GROUP_ID_REQUIRED, GROUP_NOT_FOUND, INVALID_FIELD_VALUE,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::catalog::product_group::{
    ProductGroupInsert, ProductGroupPatch,
};
use nextrade_infrastructure::db::Repositories;

/// 商品分组域服务。
pub struct ProductGroupService {
    repos: Arc<Repositories>,
}

impl ProductGroupService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 查询 —————————————————————

    /// 分组分页列表（对应 Java `ProductGroupServiceImpl#list`）。
    ///
    /// `shop_id` 由路由层按请求头解析（Java `getShopId()`），**恒定过滤**。
    pub async fn list(
        &self,
        q: &ProductGroupListQuery,
        shop_id: i32,
    ) -> Result<PageResult<ProductGroupVO>> {
        let page = q.page_query();
        let (total, rows) = self
            .repos.catalog.product_group
            .paginate(q, shop_id, &page)
            .await?;
        let list: Vec<ProductGroupVO> = rows.iter().map(group_to_vo).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 分组详情（对应 Java `ProductGroupServiceImpl#detail`）。
    pub async fn detail(&self, id: i32) -> Result<ProductGroupVO> {
        let entity = self
            .repos.catalog.product_group
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, GROUP_NOT_FOUND))?;
        Ok(group_to_vo(&entity))
    }

    /// 前台选择器分组列表（对应 Java `ProductGroupController#client`）。
    ///
    /// Java 直接返回**实体**（`productIds` 是 JSON 字符串、`addTime` 是数字），
    /// 本实现统一返回 `ProductGroupVO`（`productIds` 数组、`addTime` 字符串）：
    /// 与 `list` / `detail` 同构，且与前端
    /// `types/product/productGroup.d.ts#ProductGroupFilterState`
    /// （`productIds?: number[]`、`addTime?: string`）一致。前端选择器只读
    /// `productGroupId` / `productGroupName`，不受影响（见 `uncertain_points`）。
    pub async fn client_list(&self, q: &ProductGroupListQuery) -> Result<Vec<ProductGroupVO>> {
        let rows = self
            .repos.catalog.product_group
            .client_list(q.effective_group_name())
            .await?;
        Ok(rows.iter().map(group_to_vo).collect())
    }

    // ————————————————————— 写操作 —————————————————————

    /// 新增分组（对应 Java `ProductGroupServiceImpl#create`），返回自增主键。
    ///
    /// Java 控制器返回 `void`；本实现返回主键便于日志/调试（不影响响应结构，
    /// 路由层丢弃该值）。
    pub async fn create(
        &self,
        dto: &ProductGroupCreateDTO,
        shop_id: i32,
    ) -> Result<i32> {
        let row = ProductGroupInsert {
            product_group_name: dto.product_group_name.clone(),
            product_group_sn: dto.product_group_sn.clone(),
            product_group_description: dto.product_group_description.clone(),
            // Java `JSONUtil.toJsonStr(createDTO.getProductIds())`；
            // null 时 Java 写字符串 "null"，本实现写 "[]"（JSON 数组语义，
            // 详见 models 的 `parse_product_ids` 容错约定）
            product_ids: product_ids_to_json(dto.product_ids.as_deref().unwrap_or_default()),
            add_time: now_seconds() as i32,
            shop_id,
        };
        let id = self.repos.catalog.product_group.insert(&row).await?;
        tracing::info!("创建商品分组成功: id={id}, shop_id={shop_id}");
        Ok(id)
    }

    /// 更新分组（对应 Java `ProductGroupServiceImpl#update`）。
    ///
    /// NOT_NULL 语义：DTO 未传的列保持原值；`product_ids` 总是写
    /// （空列表 ⇒ `"[]"`）；`shopId` 传了才覆盖。
    pub async fn update(&self, dto: &ProductGroupUpdateDTO) -> Result<()> {
        let (id, patch) = build_update_patch(dto)?;
        let affected = self
            .repos.catalog.product_group
            .update(id, &patch)
            .await?;
        // Java `updateById` 影响 0 行返回 false，控制器忽略 ⇒ 不报错
        tracing::info!("更新商品分组: id={id}, affected={affected}");
        Ok(())
    }

    /// 删除分组（对应 Java `ProductGroupController#del` → `removeById`）。
    ///
    /// Java `OperateDTO.id` 为 null 时 `BaseServiceImpl#del` 直接返回 `false`
    /// （不发 SQL），控制器返回 `void` ⇒ 静默成功。本实现保持一致。
    pub async fn delete(&self, dto: &OperateDTO) -> Result<()> {
        if let Some(id) = dto.id {
            let deleted = self.repos.catalog.product_group.delete(id).await?;
            tracing::info!("删除商品分组: id={id}, deleted={deleted}");
        }
        Ok(())
    }

    /// 更新单个字段（对应 Java `ProductGroupController#updateField`）。
    ///
    /// ⚠️ **恒返回业务错误**「field 的值有误」——原因见模块注释：
    /// `product_group` 没有任何 `GENERAL_FIELDS` 列，Java 归一后回退主键列，
    /// 白名单必然不命中。这里原样复刻现网行为，不做「善意修正」。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<()> {
        reject_update_field(dto)
    }

    /// 批量操作（对应 Java `BaseServiceImpl#batch`）。
    ///
    /// - `type == "del"` 且 `ids` 非空 ⇒ 批量删除，返回是否有删除
    /// - 其它情况（类型缺失/非 del、ids 缺失/为空）⇒ `false`
    ///   （Java：类型不是 del 返回 false；`removeByIds` 对空集合也返回 false）
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        let Some(ids) = batch_delete_ids(dto) else {
            return Ok(false);
        };
        let n = self.repos.catalog.product_group.delete_batch(ids).await?;
        tracing::info!("批量删除商品分组: {n} 条");
        Ok(n > 0)
    }
}

/// 由更新入参构造仓储补丁（对应 Java `copyProperties` + `setProductIds`）。
///
/// 唯一校验：`productGroupId` 必填（Java 侧由
/// `ProductGroupConstants.PRODUCT_GROUP_ID_NOT_NULL` 表达；控制器未加
/// `@Valid`，但 `updateById` 没有主键也无法定位记录，故显式校验）。
fn build_update_patch(dto: &ProductGroupUpdateDTO) -> Result<(i32, ProductGroupPatch)> {
    let id = dto
        .product_group_id
        .ok_or_else(|| Error::Validation(GROUP_ID_REQUIRED.into()))?;
    let patch = ProductGroupPatch {
        product_group_name: dto.product_group_name.clone(),
        product_group_sn: dto.product_group_sn.clone(),
        product_group_description: dto.product_group_description.clone(),
        // Java：`CollUtil.isNotEmpty(productIds) ? toJsonStr(ids) : "[]"`
        product_ids: product_ids_to_json(dto.product_ids.as_deref().unwrap_or_default()),
        shop_id: dto.shop_id,
    };
    Ok((id, patch))
}

/// 复刻 Java `updateField` 的现网结果：**必然**报「field 的值有误」。
///
/// 独立成函数便于单测（不需要构造仓储）。
fn reject_update_field(_dto: &UpdateFieldDTO) -> Result<()> {
    Err(Error::biz(500, INVALID_FIELD_VALUE))
}

/// `batch` 的实际删除目标（`None` ⇒ 不删除、返回 false）。
///
/// 对应 Java `if ("del".equals(batchDTO.getType())) return removeByIds(...)`；
/// `removeByIds` 对空集合返回 `false`。
fn batch_delete_ids(dto: &BatchDTO) -> Option<&[i32]> {
    if dto.r#type.as_deref() != Some("del") {
        return None;
    }
    let ids = dto.ids.as_deref().unwrap_or_default();
    if ids.is_empty() {
        None
    } else {
        Some(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ———————————————— update 补丁 ————————————————

    #[test]
    fn update_requires_product_group_id() {
        let err = build_update_patch(&ProductGroupUpdateDTO::default()).unwrap_err();
        assert!(
            err.to_string().contains("商品分组id不能为空"),
            "实际: {err}"
        );
    }

    #[test]
    fn update_patch_keeps_absent_fields_untouched() {
        let dto = ProductGroupUpdateDTO {
            product_group_id: Some(8),
            product_group_name: Some("新名".into()),
            product_ids: Some(vec![1, 2]),
            ..Default::default()
        };
        let (id, patch) = build_update_patch(&dto).unwrap();
        assert_eq!(id, 8);
        // DTO 未传 ⇒ 不进 SET（Java NOT_NULL 策略）
        assert!(patch.product_group_sn.is_none());
        assert!(patch.product_group_description.is_none());
        assert!(patch.shop_id.is_none());
        assert_eq!(patch.product_group_name.as_deref(), Some("新名"));
        // Java 一定写 product_ids
        assert_eq!(patch.product_ids, "[1,2]");
    }

    #[test]
    fn update_patch_encodes_empty_product_ids_as_empty_array() {
        // Java `CollUtil.isNotEmpty(updateDTO.getProductIds())
        //       ? toJsonStr(ids) : toJsonStr(new ArrayList<>())`
        for ids in [None, Some(Vec::<i32>::new())] {
            let dto = ProductGroupUpdateDTO {
                product_group_id: Some(1),
                product_ids: ids,
                ..Default::default()
            };
            let (_, patch) = build_update_patch(&dto).unwrap();
            assert_eq!(patch.product_ids, "[]");
        }
    }

    #[test]
    fn update_patch_passes_shop_id_through() {
        // Java `BeanUtils.copyProperties` 会把 shopId 拷进实体，updateById 写入
        let dto = ProductGroupUpdateDTO {
            product_group_id: Some(2),
            shop_id: Some(9),
            ..Default::default()
        };
        let (_, patch) = build_update_patch(&dto).unwrap();
        assert_eq!(patch.shop_id, Some(9));
    }

    // ———————————————— batch ————————————————

    #[test]
    fn batch_only_deletes_for_del_type_with_ids() {
        let del = BatchDTO {
            ids: Some(vec![1, 2]),
            r#type: Some("del".into()),
            ..Default::default()
        };
        assert_eq!(batch_delete_ids(&del), Some(&[1, 2][..]));

        // 非 del ⇒ false（Java 静默）
        for op in [Some("delete"), Some("update"), None] {
            let dto = BatchDTO {
                ids: Some(vec![1]),
                r#type: op.map(ToString::to_string),
                ..Default::default()
            };
            assert_eq!(batch_delete_ids(&dto), None, "type={op:?}");
        }

        // del 但 ids 空/缺失 ⇒ removeByIds 也返回 false
        for ids in [None, Some(Vec::<i32>::new())] {
            let dto = BatchDTO {
                ids,
                r#type: Some("del".into()),
                ..Default::default()
            };
            assert_eq!(batch_delete_ids(&dto), None);
        }
    }

    // ———————————————— updateField（恒失败，现网行为） ————————————————

    #[test]
    fn update_field_is_rejected_like_java() {
        // `product_group` 没有任何 GENERAL_FIELDS 列 ⇒ Java 归一后回退主键列，
        // 白名单恒不命中 ⇒ GlobalException("field 的值有误")。
        let err = reject_update_field(&UpdateFieldDTO {
            id: Some(1),
            field: Some("isShow".into()),
            val: Some("1".into()),
        })
        .unwrap_err();
        assert!(err.to_string().contains("field 的值有误"), "实际: {err}");
        // 即使字段就是 GENERAL_FIELDS 里的名字也一样失败（现网 Java 行为）
        for field in ["is_show", "sort_order", "status"] {
            let err = reject_update_field(&UpdateFieldDTO {
                id: Some(1),
                field: Some(field.into()),
                val: Some("1".into()),
            })
            .unwrap_err();
            assert!(err.to_string().contains("field 的值有误"), "{field}");
        }
    }
}
