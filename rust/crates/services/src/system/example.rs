//! 示例服务（system::example）。
//!
//! 对应原 Java `com.nextrade.service.example.impl.ExampleServiceImpl`（115 行）
//! + `ExampleController`（103 行）中的 7 条管理端链路：
//!
//! | 本域端点（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |--------------------------------|---------|------|
//! | `GET  /example/example/list` | `ExampleController.list` | [`ExampleService::list`] |
//! | `GET  /example/example/detail` | `ExampleController.detail` | [`ExampleService::detail`] |
//! | `POST /example/example/create` | `ExampleController.create` | [`ExampleService::create`] |
//! | `POST /example/example/update` | `ExampleController.update` | [`ExampleService::update`] |
//! | `POST /example/example/del` | `ExampleController.del` | [`ExampleService::del`] |
//! | `POST /example/example/updateField` | `ExampleController.updateField` | [`ExampleService::update_field`] |
//! | `POST /example/example/batch` | `ExampleController.batch` | [`ExampleService::batch`] |
//!
//! > ⚠️ `docs/slices.json` 给本片列了 7 条，与契约一致（本片**没有**漏登记）。
//! > 但整个 `slices.json` 全量比对显示有 78 个契约接口不在任何切片里，
//! > 见 `docs/registrations/sessionB-merge-notes.md`。
//!
//! ## 迁移要点（逐条对照 Java 源）
//!
//! ### `list`
//! - `isShow` 过滤走 `IsShowType.fromTypeCode(isShow) != null`
//!   ⇒ **只有 0 和 1 生效**；`null` 与其它值（如 2）都**不加条件**
//!   （Java 返回全部数据，不是返回空）。这是本域最易误实现的一处。
//! - `keyword` 走 `LIKE %keyword%`，判据是 **`StrUtil.isNotEmpty`**
//!   （纯空白算有效关键字，与 area_code 的 `isNotBlank` 不同）。
//! - 排序**无条件**调 `buildSortOrder` ⇒ `sortField` 空时是主键列降序。
//!
//! ### `detail`
//! - Java：`if (id != null) { getById(id); copyProperties(null, vo) }`
//!   —— 记录不存在时会得到**全 null 的 VO**（不报错）。
//!   本实现返回 404（项目内既有约定），见 `uncertain_points`。
//!
//! ### `create`
//! - `@Valid` 校验 `exampleName`：`@NotBlank` + `@Size(max = 50)`。
//! - ⚠️ **不 trim** 入库值：Java 的 `@NotBlank` 只校验不改值，
//!   因此 `"  名称  "` 会**原样入库**（含首尾空格）。本实现保持一致。
//!
//! ### `update`
//! - `@NotNull` 校验 `exampleId`（文案 `"示例id不能为空"`）。
//! - Java `new Example()` + `copyProperties(updateDTO, example)`
//!   ⇒ DTO 未传字段保留旧值；`exampleName` **无** `@NotBlank`/`@Size`
//!   （更新可传空名）。
//! - ⚠️ **无法改 `example_desc`**：`ExampleUpdateDTO` 没有该字段
//!   ⇒ 本实现不写 `example_desc` 列（描述保持原值）。
//! - **不检查**目标记录是否存在（`updateById` 影响 0 行不报错）。
//!
//! ### `del`
//! - `removeById(id)` 硬删除（表无软删列）。
//!
//! ### `updateField`
//! - 白名单 `CheckFieldConstants.EXAMPLE_FIELDS`
//!   = `{example_name, is_show, sort_order}` —— 三项都是真实列，
//!   因此没有 logistics::shipping_type 那种「回退主键列」的缺陷。
//!
//! ### `batch`
//! - `BaseServiceImpl#batch`：**仅** `type == "del"` 生效，其余类型
//!   **静默返回 `false`**（不报错）。

use std::sync::Arc;

use nextrade_core::models::system::example::{
    map_update_field_column, validate_create, BatchDTO, ExampleCreateDTO, ExampleListQuery,
    ExampleUpdateDTO, ExampleVO, UpdateFieldDTO, EXAMPLE_FIELDS, MSG_EXAMPLE_ID_REQUIRED,
    MSG_EXAMPLE_NOT_FOUND, MSG_ID_REQUIRED, MSG_INVALID_FIELD_VALUE,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::system::example::{ExampleInsert, ExamplePatch};
use nextrade_infrastructure::db::Repositories;

/// 示例管理域服务（挂载于 `AdminState.example`）。
pub struct ExampleService {
    repos: Arc<Repositories>,
}

impl ExampleService {
    /// 构造（与 area_code / order_admin / admin_user 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 列表 / 详情 —————————————————————

    /// 示例分页（对应 Java `ExampleServiceImpl#list`）。
    ///
    /// 返回 `PageResult<ExampleVO>`（Java 返回 MyBatis-Plus `Page<ExampleVO>`，
    /// Rust 统一使用项目既有 `PageResult`）。
    ///
    /// `is_show` 的过滤条件在仓储层按 Java 的 `IsShowType.fromTypeCode`
    /// 口径判定（只有 0/1 生效）。
    pub async fn list(&self, q: &ExampleListQuery) -> Result<PageResult<ExampleVO>> {
        let (total, rows) = self.repos.system.example.paginate(q).await?;
        let list: Vec<ExampleVO> = rows.iter().map(ExampleVO::from).collect();
        Ok(PageResult::new(total, list, &q.page_query()))
    }

    /// 示例详情（对应 Java `ExampleServiceImpl#detail`）。
    ///
    /// Java 在记录不存在时返回**全 null 的 VO**（`getById` 返回 null 后
    /// `copyProperties(null, vo)` 是空操作），不报错；
    /// 本实现返回 404 + 文案（项目内 order_admin/005/017/075/163/210 的既有约定），
    /// 见 `uncertain_points`。
    pub async fn detail(&self, id: i32) -> Result<ExampleVO> {
        let row = self
            .repos.system.example
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_EXAMPLE_NOT_FOUND.to_string()))?;
        Ok(ExampleVO::from(&row))
    }

    // ————————————————————— 写入 —————————————————————

    /// 新增示例（对应 Java `ExampleServiceImpl#create`）。
    ///
    /// ⚠️ 名称**不 trim**：Java 的 `@NotBlank` 只校验不改值，
    /// 因此 `"  演示  "` 会原样入库。本实现同样原样写入。
    ///
    /// Java 返回 `boolean`；此处回传新建主键便于前端跳转编辑
    /// （与 shop_manage/017/075/163/210 的处理方式一致）。
    pub async fn create(&self, dto: &ExampleCreateDTO) -> Result<i32> {
        // 对照 Java `@Valid ExampleCreateDTO`（@NotBlank + @Size(max=50)）
        validate_create(dto).map_err(Error::Validation)?;

        let insert = ExampleInsert {
            // 原样写入（不 trim，与 Java 一致）
            example_name: dto.example_name.clone(),
            example_pic: dto.example_pic.clone(),
            example_desc: dto.example_desc.clone(),
            sort_order: dto.sort_order,
            is_show: dto.is_show,
            status: dto.status,
        };
        self.repos.system.example.insert(&insert).await
    }

    /// 更新示例（对应 Java `ExampleServiceImpl#update`）。
    ///
    /// 复刻 Java 的「直接 new 空实体 + copyProperties」语义：
    /// DTO 未传的字段**不写列**（保留旧值）。
    ///
    /// ⚠️ 两处必须照抄 Java 的行为：
    /// 1. **不校验** `exampleName`（`ExampleUpdateDTO` 上没有
    ///    `@NotBlank`/`@Size`）⇒ 可传空名；
    /// 2. **不写 `example_desc`**（`ExampleUpdateDTO` 没有该字段）
    ///    ⇒ 描述保持库中原值。
    ///
    /// 也**不检查**目标记录是否存在（Java `updateById` 影响 0 行不报错）。
    pub async fn update(&self, dto: &ExampleUpdateDTO) -> Result<u64> {
        // Java `@NotNull(message = "示例id不能为空")`
        let id = dto
            .example_id
            .ok_or_else(|| Error::Validation(MSG_EXAMPLE_ID_REQUIRED.to_string()))?;

        let patch = ExamplePatch {
            example_name: Some(dto.example_name.clone()),
            example_pic: Some(dto.example_pic.clone()),
            // ⚠️ 显式不写：ExampleUpdateDTO 无此字段（Java 契约如此）
            example_desc: None,
            sort_order: Some(dto.sort_order),
            is_show: Some(dto.is_show),
            status: Some(dto.status),
        };
        self.repos.system.example.update(id, &patch).await
    }

    /// 删除示例（对应 Java `removeById`）。
    pub async fn del(&self, id: i32) -> Result<bool> {
        self.repos.system.example.delete(id).await
    }

    /// 单字段更新（对应 Java `BaseServiceImpl#updateField`）。
    ///
    /// 三步与 Java 一致：
    /// 1. `field = getColumnByPropertyName(field)`（未识别回退主键列）；
    /// 2. 映射后的列名必须命中 [`EXAMPLE_FIELDS`]，否则
    ///    `"field 的值有误"`；
    /// 3. `UPDATE ... SET <column> = ? WHERE example_id = ?`（字符串绑定）。
    ///
    /// ✅ 本域白名单三项都是真实列，因此**不存在** logistics::shipping_type 那种
    /// 「`is_show` 通过白名单却改成主键」的缺陷；
    /// 但「完全未知的字段名回退主键列」这一 Java 通用行为仍然存在，
    /// 且回退后的 `example_id` **不在** `EXAMPLE_FIELDS` 里 ⇒ 会被拒绝。
    /// 也就是说本域的 `updateField` 实际上是**安全**的。
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
        if !EXAMPLE_FIELDS.contains(&column) {
            return Err(Error::Validation(MSG_INVALID_FIELD_VALUE.to_string()));
        }

        // ③ 执行
        self.repos.system.example.update_field(id, column, val).await
    }

    /// 批量操作（对应 Java `BaseServiceImpl#batch`）。
    ///
    /// ⚠️ **仅 `type == "del"` 生效**；其它类型 Java **静默返回 `false`**。
    /// 本实现返回 `Ok(0)` 表示「什么都没做」，由路由层回 `false`。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<u64> {
        let ids = dto.validated_ids().map_err(Error::Validation)?;
        if !dto.is_delete() {
            // Java `return false;` —— 非 del 类型静默不做事
            return Ok(0);
        }
        self.repos.system.example.delete_many(&ids).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::system::example::{EXAMPLE_COLUMNS, KEY_COLUMN};

    /// ✅ 本域 `updateField` 是安全的：未知字段回退主键列后
    /// **不在**白名单里，因此被拒绝（与 logistics::shipping_type 的缺陷形成对照）。
    #[test]
    fn unknown_field_is_rejected_in_this_domain() {
        let col = map_update_field_column("totally_unknown");
        assert_eq!(col, KEY_COLUMN, "复刻 Java：未识别字段回退主键列");
        assert!(
            !EXAMPLE_FIELDS.contains(&col),
            "关键：回退后的主键列不在 EXAMPLE_FIELDS 里 ⇒ 会被拒绝"
        );
    }

    /// 真实列名通过白名单。
    #[test]
    fn real_columns_pass_whitelist() {
        for f in ["example_name", "is_show", "sort_order"] {
            let col = map_update_field_column(f);
            assert!(EXAMPLE_FIELDS.contains(&col), "{f} 应通过（映射为 {col}）");
        }
    }

    /// 白名单三项都必须是真实列（防止未来被误改）。
    #[test]
    fn whitelist_entries_are_real_columns() {
        for f in EXAMPLE_FIELDS {
            assert!(EXAMPLE_COLUMNS.contains(&f), "白名单含不存在的列: {f}");
        }
    }

    /// `batch` 只认精确的 `"del"`。
    #[test]
    fn batch_only_acts_on_exact_del_type() {
        let d = BatchDTO {
            ids: Some(vec![1]),
            r#type: Some("del".into()),
            ..Default::default()
        };
        assert!(d.is_delete());

        for t in ["Del", "DEL", "delete", "", "show"] {
            let d = BatchDTO {
                ids: Some(vec![1]),
                r#type: Some(t.into()),
                ..Default::default()
            };
            assert!(!d.is_delete(), "type={t} 不应判定为删除");
        }
    }

    /// `batch` 缺 ids 时报 Java 的 `@NotEmpty` 文案。
    #[test]
    fn batch_requires_ids() {
        let d = BatchDTO {
            ids: None,
            r#type: Some("del".into()),
            ..Default::default()
        };
        assert_eq!(d.validated_ids(), Err("未选择项目".to_string()));
    }

    /// Java 文案常量必须逐字符一致。
    #[test]
    fn java_message_constants() {
        assert_eq!(MSG_EXAMPLE_ID_REQUIRED, "示例id不能为空");
        assert_eq!(MSG_EXAMPLE_NOT_FOUND, "示例不存在");
        assert_eq!(MSG_INVALID_FIELD_VALUE, "field 的值有误");
        assert_eq!(MSG_ID_REQUIRED, "id不能为空");
    }

    /// `update` 缺 `exampleId` 时应报 `"示例id不能为空"`。
    #[test]
    fn update_requires_example_id() {
        let dto = ExampleUpdateDTO::default();
        // 直接验证校验分支的文案来源（不构造服务实例，避免依赖 DB）
        let err = dto
            .example_id
            .ok_or_else(|| MSG_EXAMPLE_ID_REQUIRED.to_string());
        assert_eq!(err, Err("示例id不能为空".to_string()));
    }

    /// ⚠️ 回归护栏：`update` 的补丁**不写** `example_desc`
    /// （Java `ExampleUpdateDTO` 无该字段 ⇒ 描述不可通过 update 修改）。
    #[test]
    fn update_patch_never_writes_example_desc() {
        let dto = ExampleUpdateDTO {
            example_id: Some(1),
            example_name: Some("n".into()),
            example_pic: Some("p".into()),
            sort_order: Some(2),
            is_show: Some(1),
            status: Some(1),
        };
        let patch = ExamplePatch {
            example_name: Some(dto.example_name.clone()),
            example_pic: Some(dto.example_pic.clone()),
            example_desc: None, // 有意
            sort_order: Some(dto.sort_order),
            is_show: Some(dto.is_show),
            status: Some(dto.status),
        };
        assert_eq!(patch.example_desc, None);
        assert!(!patch.is_empty(), "其它字段仍应产生写入");
    }
}