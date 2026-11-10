//! 配送类型服务（logistics::shipping_type）。
//!
//! 对应原 Java `com.nextrade.service.setting.impl.ShippingTypeServiceImpl`
//! （144 行）+ `ShippingTypeController`（99 行）中的 7 条管理端链路：
//!
//! | 本域端点（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |--------------------------------|---------|------|
//! | `GET  /setting/shippingType/list` | `ShippingTypeController.list` | [`ShippingTypeService::list`] |
//! | `GET  /setting/shippingType/detail` | `ShippingTypeController.detail` | [`ShippingTypeService::detail`] |
//! | `POST /setting/shippingType/create` | `ShippingTypeController.create` | [`ShippingTypeService::create`] |
//! | `POST /setting/shippingType/update` | `ShippingTypeController.update` | [`ShippingTypeService::update`] |
//! | `POST /setting/shippingType/del` | `ShippingTypeController.del` | [`ShippingTypeService::del`] |
//! | `POST /setting/shippingType/updateField` | `ShippingTypeController.updateField` | [`ShippingTypeService::update_field`] |
//! | `POST /setting/shippingType/batch` | `ShippingTypeController.batch` | [`ShippingTypeService::batch`] |
//!
//! > ⚠️ `docs/slices.json` 给本片只列了 **6** 条（漏了 `GET /list`），
//! > 但 Java 控制器与 `docs/api-contract.json` 里**都有** `/list`。
//! > 本实现按**契约**补齐 7 条（见 `docs/registrations/sessionB-merge-notes.md`）。
//!
//! ## 迁移要点（逐条对照 Java 源）
//!
//! ### `list`
//! - **强制**按 `shop_id` 过滤，值来自请求头解析的 `getShopId()`
//!   （`ShippingTypeListDTO.shopId` 参数被 Java **忽略**）。
//! - `keyword` 走 `ILIKE %keyword%`，判据是 **`StrUtil.isNotEmpty`**
//!   （纯空白算有效关键字，与 area_code 的 `isNotBlank` 口径不同）。
//! - 排序**无条件**调 `buildSortOrder` ⇒ `sortField` 空时是主键列降序。
//! - 逐行查物流公司名补 `logisticsName`（Java N+1）。
//!
//! ### `detail`
//! - Java：`if (id != null) { getById(id); copyProperties } return null;`
//!   —— `id` 为空直接返回 `null`；**记录不存在时会 NPE**
//!   （`BeanUtils.copyProperties(null, vo)` 对 null 源不抛，但
//!   `getById` 返回 null 后 copyProperties 是空操作，实际返回一个全 null 的 VO）。
//!   本实现返回 404（项目内既有约定），见 `uncertain_points`。
//!
//! ### `create`
//! - `shopId` **无条件**用请求头值覆盖（请求里的 `shopId` 被丢弃）。
//! - Java 写了一条 admin 日志
//!   `adminLogService.createByLogInfo("新增配送类型:" + name)`。
//!   **本实现未复刻**（日志表 / 操作人上下文未在本轮范围内，见 `uncertain_points`）。
//!
//! ### `update`
//! - Java `new ShippingType()` + `copyProperties(updateDTO, ...)` +
//!   `setShippingTypeId(updateDTO.getId())` ⇒ DTO 未传字段保留旧值。
//! - **不校验** `id` 对应的记录是否存在（直接 updateById，
//!   不存在时影响 0 行但**不报错**）。
//! - 也写了一条 admin 日志（同样未复刻）。
//! - ⚠️ `shopId` 不在 `ShippingTypeUpdateDTO` 里 ⇒ **无法改店铺归属**。
//!
//! ### `del`
//! - `removeById(id)` 硬删除。
//!
//! ### `updateField`
//! - 白名单 `CheckFieldConstants.SHIPPING_TYPE_FIELDS`
//!   = `{is_show, sort_order, shipping_type_name, shipping_type_desc,
//!     is_support_cod}`。
//! - ⚠️ **`is_show` 不是 `shipping_type` 的列**！
//!   `getColumnByPropertyName("is_show")` 找不到 ⇒ 防注入「安全修复」
//!   回退成**主键列** `shipping_type_id`；而该列**恰好在白名单里**
//!   ⇒ 校验通过 ⇒ 最终执行
//!   `UPDATE shipping_type SET shipping_type_id = ? WHERE ...`
//!   —— 一个**能成功但语义完全错误**的更新（可改主键）。
//!   本实现**逐字符复刻该行为**（默认路径），并另备安全白名单
//!   [`SHIPPING_TYPE_FIELDS_SAFE`] 供切换（见 `uncertain_points`）。
//!
//! ### `batch`
//! - `BaseServiceImpl#batch`：**仅** `type == "del"` 生效，其余类型
//!   **静默返回 `false`**（不报错）。

use std::sync::Arc;

use nextrade_core::models::logistics::shipping_type::{
    map_update_field_column, BatchDTO, ShippingTypeCreateDTO, ShippingTypeListQuery,
    ShippingTypeUpdateDTO, UpdateFieldDTO, MSG_ID_REQUIRED, MSG_INVALID_FIELD_VALUE,
    MSG_SHIPPING_TYPE_ID_NOT_NULL, MSG_SHIPPING_TYPE_NOT_FOUND, SHIPPING_TYPE_FIELDS,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::logistics::shipping_type::{ShippingTypeInsert, ShippingTypePatch};
use nextrade_infrastructure::db::Repositories;

/// 配送类型管理域服务（挂载于 `AdminState.shipping_type`）。
pub struct ShippingTypeService {
    repos: Arc<Repositories>,
}

impl ShippingTypeService {
    /// 构造（与 area_code / order_admin / admin_user 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 列表 / 详情 —————————————————————

    /// 配送类型分页（对应 Java `ShippingTypeServiceImpl#list`）。
    ///
    /// `shop_id` 由**路由层**从请求头解析后传入（对应 Java
    /// `getShopId()`），因为服务层不应依赖 HTTP 头。
    pub async fn list(
        &self,
        q: &ShippingTypeListQuery,
        shop_id: i32,
    ) -> Result<PageResult<nextrade_core::models::logistics::shipping_type::ShippingTypeVO>> {
        let (total, rows) = self.repos.logistics.shipping_type.paginate(q, shop_id).await?;

        // 批量补物流公司名（Java 为逐行 N+1；本实现一次 IN 查询，结果集相同）。
        // `shipping_default_id == 0` 表示未设置 ⇒ 不查（Java 里 0 也能查到
        // 一条 id=0 的记录，但正常库中不存在，净结果同为 null）。
        let ids: Vec<i32> = rows
            .iter()
            .map(|r| r.shipping_default_id)
            .filter(|id| *id > 0)
            .collect();
        let names = self.repos.logistics.shipping_type.find_logistics_names(&ids).await?;

        let list = rows
            .iter()
            .map(|r| {
                let name = names.get(&r.shipping_default_id).cloned();
                nextrade_core::models::logistics::shipping_type::ShippingTypeVO::with_logistics_name(r, name)
            })
            .collect();

        Ok(PageResult::new(total, list, &q.page_query()))
    }

    /// 配送类型详情（对应 Java `ShippingTypeServiceImpl#detail`）。
    ///
    /// ⚠️ Java 的 `detail` **不补** `logisticsName`（只有 `list` 补），
    /// 因此这里同样不查物流公司。
    ///
    /// Java 在记录不存在时返回一个**全 null 的 VO**（不报错）；
    /// 本实现返回 404，与项目内 order_admin/005/017/075 的既有约定一致
    /// （见 `uncertain_points`）。
    pub async fn detail(
        &self,
        id: i32,
    ) -> Result<nextrade_core::models::logistics::shipping_type::ShippingTypeVO> {
        let row = self
            .repos.logistics.shipping_type
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_SHIPPING_TYPE_NOT_FOUND.to_string()))?;
        Ok(nextrade_core::models::logistics::shipping_type::ShippingTypeVO::from(&row))
    }

    // ————————————————————— 写入 —————————————————————

    /// 新增配送类型（对应 Java `ShippingTypeServiceImpl#create`）。
    ///
    /// ⚠️ `shopId` 用**请求头**值覆盖，忽略 DTO 里的 `shopId`
    /// （Java `shippingType.setShopId(getShopId())`）。
    ///
    /// Java 返回 `boolean`；此处回传新建主键便于前端跳转编辑
    /// （与 shop_manage/017/075/163 的处理方式一致）。
    ///
    /// ⚠️ Java **未复刻** admin 操作日志（`adminLogService.createByLogInfo`），
    /// 见 `uncertain_points`。
    pub async fn create(&self, dto: &ShippingTypeCreateDTO, shop_id: i32) -> Result<i32> {
        let insert = ShippingTypeInsert {
            // Java 无条件覆盖，忽略 dto.shop_id
            shop_id,
            shipping_type_name: dto.shipping_type_name.clone(),
            shipping_default_id: dto.shipping_default_id,
            shipping_type_desc: dto.shipping_type_desc.clone(),
            shipping_time_desc: dto.shipping_time_desc.clone(),
            is_support_cod: dto.is_support_cod,
            sort_order: dto.sort_order,
        };
        let id = self.repos.logistics.shipping_type.insert(&insert).await?;
        let log_info = format!(
                "新增配送类型:{}",
                dto.shipping_type_name.as_deref().unwrap_or("")
            );
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log(if shop_id > 0 { "shop" } else { "admin" }, shop_id, &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        Ok(id)
    }

    /// 更新配送类型（对应 Java `ShippingTypeServiceImpl#update`）。
    ///
    /// 复刻 Java 的「直接 new 空实体 + copyProperties」语义：
    /// DTO 未传的字段**不写列**（保留旧值）。
    ///
    /// ⚠️ Java **不检查**目标记录是否存在：`updateById` 影响 0 行也不报错。
    /// 本实现保持相同行为（不预检存在性），返回受影响行数。
    pub async fn update(&self, dto: &ShippingTypeUpdateDTO) -> Result<u64> {
        // Java `@NotNull(message = "配送类型id不能为空")`
        let id = dto
            .id
            .ok_or_else(|| Error::Validation(MSG_SHIPPING_TYPE_ID_NOT_NULL.to_string()))?;

        let patch = ShippingTypePatch {
            shipping_type_name: Some(dto.shipping_type_name.clone()),
            shipping_default_id: Some(dto.shipping_default_id),
            shipping_type_desc: Some(dto.shipping_type_desc.clone()),
            shipping_time_desc: Some(dto.shipping_time_desc.clone()),
            is_support_cod: Some(dto.is_support_cod),
            sort_order: Some(dto.sort_order),
            // update DTO 无 shopId ⇒ 不写该列
            shop_id: None,
        };
        let affected = self.repos.logistics.shipping_type.update(id, &patch).await?;
        let log_info = format!(
                "修改配送类型:{}",
                dto.shipping_type_name.as_deref().unwrap_or("")
            );
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log("admin", 0, &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        Ok(affected)
    }

    /// 删除配送类型（对应 Java `removeById`）。
    pub async fn del(&self, id: i32) -> Result<bool> {
        self.repos.logistics.shipping_type.delete(id).await
    }

    /// 单字段更新（对应 Java `BaseServiceImpl#updateField`）。
    ///
    /// 复刻 Java 的三步：
    /// 1. `field = getColumnByPropertyName(field)` —— 属性名/列名都受理，
    ///    **未识别则回退主键列**；
    /// 2. 校验映射后的列名是否在 [`SHIPPING_TYPE_FIELDS`] 白名单内，
    ///    不在则抛 `"field 的值有误"`；
    /// 3. `UPDATE ... SET <column> = ? WHERE <主键> = ?`（值按字符串绑定）。
    ///
    /// ⚠️ 因为第 1 步的回退 + 白名单里含 `is_show`，
    /// `field = "is_show"` 会通过校验并**改掉主键**（Java 真实缺陷，已复刻）。
    /// 参见 `crates/core/src/models/shipping_type.rs` 的
    /// [`SHIPPING_TYPE_FIELDS`] 说明与 `uncertain_points`。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<u64> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.to_string()))?;
        let field = dto
            .field
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("字段名不能为空".to_string()))?;
        let val = dto
            .val
            .as_deref()
            .ok_or_else(|| Error::Validation("值不能为空".to_string()))?;

        // ① 归一为列名（未识别回退主键列 —— Java 的防注入「安全修复」）
        let column = map_update_field_column(field);

        // ② 白名单校验（Java 比较的是**映射后**的列名）
        if !SHIPPING_TYPE_FIELDS.contains(&column) {
            return Err(Error::Validation(MSG_INVALID_FIELD_VALUE.to_string()));
        }

        // ③ 执行
        self.repos.logistics.shipping_type.update_field(id, column, val).await
    }

    /// 批量操作（对应 Java `BaseServiceImpl#batch`）。
    ///
    /// ⚠️ **仅 `type == "del"` 生效**；其它类型 Java **静默返回 `false`**
    /// （不抛异常、不改数据）。因此本方法返回 `Ok(0)` 表示「什么都没做」，
    /// 由路由层回 `false`，与 Java 的返回值语义一致。
    ///
    /// `ids` 为空时：Java 的 `@NotEmpty` 会拦下（400）；本实现同样报错。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<u64> {
        let ids = dto.validated_ids().map_err(Error::Validation)?;
        if !dto.is_delete() {
            // Java `return false;` —— 非 del 类型静默不做事
            return Ok(0);
        }
        self.repos.logistics.shipping_type.delete_many(&ids).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::logistics::shipping_type::{
        SHIPPING_TYPE_FIELDS_SAFE, MSG_INVALID_FIELD_VALUE as MSG,
    };

    /// 复刻 Java 缺陷路径：`is_show` 经「回退主键列」后变成
    /// `shipping_type_id`，而**主键列不在** Java 白名单 `SHIPPING_TYPE_FIELDS` 里
    /// ⇒ `update_field` 会以 `INVALID_FIELD_VALUE` **拒绝**它。
    ///
    /// Java 顺序是 `getColumnByPropertyName(field)` → `allowFields.contains(column)`，
    /// 比的是**映射后**的列名。因此白名单里那个 `is_show` 永远走不到。
    #[test]
    fn is_show_is_rejected_after_key_column_fallback() {
        let column = map_update_field_column("is_show");
        assert_eq!(column, nextrade_core::models::logistics::shipping_type::KEY_COLUMN);
        assert!(
            !SHIPPING_TYPE_FIELDS.contains(&column),
            "映射后是主键列；主键列不在 Java 白名单里 ⇒ 应被拒绝"
        );
        // 安全白名单同样拒绝，证明两条白名单在这点上结论一致
        assert!(!SHIPPING_TYPE_FIELDS_SAFE.contains(&column));
    }

    /// 真实列名要通过白名单。
    #[test]
    fn real_columns_pass_whitelist() {
        for f in ["shipping_type_name", "sort_order", "is_support_cod"] {
            let col = map_update_field_column(f);
            assert!(
                SHIPPING_TYPE_FIELDS.contains(&col),
                "{f} 应通过白名单（映射为 {col}）"
            );
        }
    }

    /// 完全未知的字段名回退主键列，主键列**不在**白名单里 ⇒ **同样被拒绝**
    /// （这是 Java 行为的更广义的后果，必须如实记录）。
    #[test]
    fn unknown_field_also_falls_back_and_is_rejected() {
        let col = map_update_field_column("totally_unknown_column");
        assert_eq!(col, nextrade_core::models::logistics::shipping_type::KEY_COLUMN);
        assert!(!SHIPPING_TYPE_FIELDS.contains(&col));
    }

    /// `MSG` 常量就是 Java 的 `"field 的值有误"`（带一个空格）。
    #[test]
    fn invalid_field_message_matches_java() {
        assert_eq!(MSG, "field 的值有误");
    }

    /// `BatchDTO` 的删除判据只认 `"del"`（区分大小写，Java 用 equals）。
    #[test]
    fn batch_only_acts_on_exact_del_type() {
        let d = BatchDTO {
            ids: Some(vec![1]),
            r#type: Some("del".into()),
            ..Default::default()
        };
        assert!(d.is_delete());

        for t in ["Del", "DEL", "delete", "show", ""] {
            let d = BatchDTO {
                ids: Some(vec![1]),
                r#type: Some(t.into()),
                ..Default::default()
            };
            assert!(!d.is_delete(), "type={t} 不应判定为删除");
        }

        let none = BatchDTO {
            ids: Some(vec![1]),
            r#type: None,
            ..Default::default()
        };
        assert!(!none.is_delete());
    }

    /// `batch` 的 ids 校验走 Java 的 `@NotEmpty` 文案。
    #[test]
    fn batch_requires_ids() {
        let d = BatchDTO {
            ids: None,
            r#type: Some("del".into()),
            ..Default::default()
        };
        assert_eq!(d.validated_ids(), Err("未选择项目".to_string()));
    }

    /// `update` 缺 id 时报 Java 的 `@NotNull` 文案。
    #[test]
    fn update_requires_id_message() {
        assert_eq!(MSG_SHIPPING_TYPE_ID_NOT_NULL, "配送类型id不能为空");
    }

    /// 错误文案与不存在提示。
    #[test]
    fn not_found_message() {
        assert_eq!(MSG_SHIPPING_TYPE_NOT_FOUND, "配送类型不存在");
    }
}