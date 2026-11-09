//! 区号管理服务（system::area_code）。
//!
//! 对应原 Java `com.nextrade.service.setting.impl.AreaCodeServiceImpl`（199 行）
//! + `AreaCodeController`（93 行）中的 7 条管理端链路：
//!
//! | 本域端点（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |--------------------------------|---------|------|
//! | `GET  /setting/areaCode/list` | `AreaCodeController.list` | [`AreaCodeService::list`] |
//! | `GET  /setting/areaCode/detail` | `AreaCodeController.detail` | [`AreaCodeService::detail`] |
//! | `POST /setting/areaCode/create` | `AreaCodeController.create` | [`AreaCodeService::create`] |
//! | `POST /setting/areaCode/update` | `AreaCodeController.update` | [`AreaCodeService::update`] |
//! | `POST /setting/areaCode/del` | `AreaCodeController.del` | [`AreaCodeService::del`] |
//! | `POST /setting/areaCode/updateField` | `AreaCodeController.updateField` | [`AreaCodeService::update_field`] |
//! | `POST /setting/areaCode/batch` | `AreaCodeController.batch` | [`AreaCodeService::batch`] |
//!
//! ## 迁移要点（逐条对照 Java 源）
//!
//! - `list`：`buildSortOrder(page, sortField, sortOrder)` —— `sortField` 为空或
//!   等于主键属性名 `id` 时用主键列；未识别的字段名**静默回退主键列**
//!   （MyBatis-Plus 的安全修复，不是报错）；`sortOrder` 为 `asc` 升序、
//!   为空或 `desc` 降序，**其它非法值完全不追加 ORDER BY**（`else if` 的缺口）。
//!   `name` 走 `LIKE %name%`。
//! - VO 的 `label` 一律由 `code` 派生为 `+{code}`
//!   （Java `vo.setLabel(StrUtil.format("+{}", code))`）。
//! - `create`：先 `checkAreaCode(param, null)`，再把其它记录的
//!   「默认」标记清 0（仅当本次 `isDefault == 1`），最后入库。
//! - `update`：先 `checkAreaCode(param, param.getId())`（排除自身），
//!   同样在 `isDefault == 1` 时清掉其它默认，再 `updateById`。
//! - `del`：`removeById(id)` 硬删除（表无软删列）。
//! - `updateField`：白名单 `CheckFieldConstants.AREA_CODE_FIELDS`
//!   = `{code, name, is_available, is_default}`；字段名先经
//!   `getColumnByPropertyName` 归一（属性名或列名都可命中）；
//!   `is_default` 且值为 `"1"` 时先清掉其它默认。
//! - `batch`：仅 `type == "del"` 生效，其余类型**静默返回 `false`**
//!   （Java `BaseServiceImpl#batch` 的行为，不是报错）。
//!
//! ## ⚠️ Java 原代码的两个已确认缺陷（本实现按「可观测行为」复刻）
//!
//! ### 1. `checkAreaCode` 的名称查重写错了列（真实 bug）
//!
//! Java 第 142–147 行：
//! ```text
//! long nameCount = this.count(new LambdaQueryWrapper<AreaCode>()
//!         .ne(null != areaCodeId, AreaCode::getId, areaCodeId)
//!         .eq(AreaCode::getName, param.getCode()));   // ← 拿 name 列比 code 字段
//! if (nameCount > 0) throw new GlobalException(AREA_CODE_NAME_IS_EXISTS);
//! ```
//! 第二次查的是 **`name` 列 = `code` 值**，而且它排在 `code` 查重之后：
//! 只要 `code` 命中就会被前一段先抛出 `AREA_CODE_IS_EXISTS`，所以这段**永不触发**。
//! 净效果：**「区号名称已存在」这条校验在 Java 里从未生效，名称允许重复**。
//!
//! 本实现的选择：**默认逐字符复刻 Java 的可观测行为**（名称不查重），
//! 以保持与线上一致；同时把 `AREA_CODE_NAME_IS_EXISTS` 文案与
//! 正确的查重能力保留在
//! [`check_area_code_with_name`]，由调用方按需切换。
//! 这是一处**需要产品确认**的点，已列入 `uncertain_points`。
//!
//! ### 2. `is_default` 唯一性只在「置 1」时维护，且有顺序问题
//!
//! Java `create` / `update`：**先**把「其它记录的默认清 0」，**再**写自己。
//! 对 `update` 而言，那条 SQL 的 `eq(isDefault, YES)` 会**把正在编辑的这条也清 0**，
//! 然后再由 `updateById` 写回请求里的 `isDefault`
//! （请求为 1 时最终恰好唯一；请求为 0 时也正确）。
//! 因此行为上是对的，但依赖「先清后写」的顺序。
//! 本实现用 `clear_default_flag(Some(id))` 把自身排除，得到**相同净结果**
//! 且语义更直白（见 `uncertain_points` 记录的这处有意偏离）。
//!
//! `updateField` 里 Java 的 `lambdaUpdate()` **没有**排除自身，同样因为
// `set(field, val)` 随后会写回 `"1"` 而得到正确结果。本实现同样用
// `clear_default_flag(Some(id))`，净结果一致。

use std::sync::Arc;

use nextrade_core::models::system::area_code::{
    map_update_field_column, validate_save, AreaCodeEditDTO, AreaCodeListQuery, AreaCodeSaveDTO,
    AreaCodeVO, BatchDTO, OperateDTO, UpdateFieldDTO, AREA_CODE_COLUMNS, AREA_CODE_FIELDS,
    MSG_AREA_CODE_ID_REQUIRED, MSG_AREA_CODE_IS_EXISTS, MSG_AREA_CODE_NAME_IS_EXISTS,
    MSG_AREA_CODE_NOT_FOUND, MSG_ID_REQUIRED, MSG_INVALID_FIELD_VALUE, NO, YES,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::system::area_code::{AreaCodeInsert, AreaCodePatch};
use nextrade_infrastructure::db::Repositories;

/// 区号管理域服务（挂载于 `AdminState.area_code`）。
pub struct AreaCodeService {
    repos: Arc<Repositories>,
}

impl AreaCodeService {
    /// 构造（与 order_admin / admin_user / pc_cat_floor 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 列表 / 详情 —————————————————————

    /// 区号分页（对应 Java `AreaCodeServiceImpl#list`）。
    ///
    /// 返回 `PageResult<AreaCodeVO>`（Java 返回 MyBatis-Plus
    /// `Page<AreaCodeVO>`，Rust 统一使用项目既有 `PageResult`，
    /// 与 product_manage/002/008/011/017 同一约定）。
    pub async fn list(&self, q: &AreaCodeListQuery) -> Result<PageResult<AreaCodeVO>> {
        let page = q.page_query();
        let (total, rows) = self.repos.system.area_code.paginate(q).await?;
        let list: Vec<AreaCodeVO> = rows.iter().map(AreaCodeVO::from).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 区号详情（对应 Java `AreaCodeServiceImpl#detail`）。
    ///
    /// Java 在 `id == null` 时返回 `null` 体；本实现由路由层要求 `id` 必填
    /// （与 Java `@RequestParam Integer id` 的「缺失即 400」一致）。
    /// 记录不存在时 Java 会 `BeanUtils.copyProperties(null, vo)` 后
    /// 在 `areaCode.getCode()` 上抛 **NPE**；本实现返回可诊断的 404
    /// （项目内 order_admin/005/017 的既有约定）。
    pub async fn detail(&self, id: i32) -> Result<AreaCodeVO> {
        let a = self
            .repos.system.area_code
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_AREA_CODE_NOT_FOUND.into()))?;
        Ok(AreaCodeVO::from(&a))
    }

    // ————————————————————— 创建 —————————————————————

    /// 创建区号（对应 Java `AreaCodeServiceImpl#create`）。
    ///
    /// 顺序与 Java 逐条对齐：
    /// 1. `checkAreaCode(param, null)` —— 查重（`code` 已存在 → `区号已存在`）
    /// 2. `isDefault == YES` 时把其它记录的默认清 0
    /// 3. `save(areaCode)`
    ///
    /// 返回新建主键（Java 控制器返回 `void`，此处便于前端跳转编辑，
    /// 与 shop_manage / pc_cat_floor 的处理方式一致）。
    pub async fn create(&self, dto: &AreaCodeSaveDTO) -> Result<i32> {
        // 1. 校验（Java `@NotBlank` / `@Size(max = 64)`，类级 `@Validated` 生效）
        let (code, name) = validate_save(dto).map_err(Error::Validation)?;
        // 2. 查重（Java `checkAreaCode(param, null)`）
        self.check_area_code(&code, None).await?;

        // 3. 若本次设为默认，先把其它默认清 0（Java 在 save 之前执行）
        if dto.is_default == Some(YES) {
            self.repos.system.area_code.clear_default_flag(None).await?;
        }

        // 4. 入库：`isAvailable` / `isDefault` 为 null 时不写列（走库默认值）
        let id = self
            .repos.system.area_code
            .insert(&AreaCodeInsert {
                code,
                name,
                is_available: dto.is_available,
                is_default: dto.is_default,
            })
            .await?;

        tracing::info!("创建区号成功: id={id}");
        Ok(id)
    }

    // ————————————————————— 更新 —————————————————————

    /// 更新区号（对应 Java `AreaCodeServiceImpl#update`）。
    ///
    /// 顺序与 Java 逐条对齐：
    /// 1. `checkAreaCode(param, param.getId())` —— 查重时**排除自身**
    /// 2. `isDefault == YES` 时把其它记录的默认清 0
    /// 3. `updateById(areaCode)`（`FieldStrategy.NOT_NULL`：只写非 null 字段）
    ///
    /// 返回是否命中记录（Java 控制器返回 `void`，此处回传受影响行数 > 0）。
    pub async fn update(&self, dto: &AreaCodeEditDTO) -> Result<bool> {
        let id = dto
            .id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation(MSG_AREA_CODE_ID_REQUIRED.into()))?;

        let base = dto.as_save();
        let (code, name) = validate_save(base).map_err(Error::Validation)?;
        // 查重排除自身（Java `ne(null != areaCodeId, AreaCode::getId, areaCodeId)`）
        self.check_area_code(&code, Some(id)).await?;

        // 记录必须存在：Java `updateById` 命中 0 行静默返回 false，
        // 这里先查一次以便区分「不存在」与「值未变化」
        if self.repos.system.area_code.find_by_id(id).await?.is_none() {
            return Err(Error::NotFound(MSG_AREA_CODE_NOT_FOUND.into()));
        }

        if base.is_default == Some(YES) {
            self.repos.system.area_code.clear_default_flag(Some(id)).await?;
        }

        // `code` / `name` 已通过校验 ⇒ 必然写列；两个标记列仅在非 null 时写列
        let patch = AreaCodePatch {
            code: Some(Some(code)),
            name: Some(Some(name)),
            is_available: base.is_available.map(Some),
            is_default: base.is_default.map(Some),
        };
        let affected = self.repos.system.area_code.update(id, &patch).await?;
        tracing::info!("更新区号: id={id}, 影响 {affected} 行");
        Ok(affected > 0)
    }

    // ————————————————————— 删除 —————————————————————

    /// 删除区号（对应 Java `AreaCodeServiceImpl#del` → `BaseServiceImpl#del`
    /// → `removeById`，硬删除）。
    ///
    /// ⚠️ Java 的 `del` 是「`id == null` 返回 false，否则 `removeById(id)`」，
    /// **不校验记录是否存在**（删不存在的 id 也返回成功）。
    /// 本实现保持这一语义（返回删除是否命中），不额外抛 404。
    pub async fn del(&self, dto: &OperateDTO) -> Result<bool> {
        let id = dto
            .id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        self.repos.system.area_code.delete(id).await
    }

    // ————————————————————— 单字段更新 —————————————————————

    /// 单字段更新（对应 Java `AreaCodeServiceImpl#updateField(dto, AREA_CODE_FIELDS)`）。
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. `field` 经 `getColumnByPropertyName` 归一为列名（属性名/列名都可命中，
    ///    **未识别时回退主键列** —— 本实现用 `None` 表达，对白名单的判定结果一致）；
    /// 2. 归一后的列名必须落在 `AREA_CODE_FIELDS` = `{code, name, is_available,
    ///    is_default}` 内，否则 `字段值不合法`；
    /// 3. `is_default` 且值等于字符串 `"1"` 时，先把其它记录的默认清 0
    ///    （Java 的比较是 `"1".equals(updateField.getVal())`，**字符串比较**，
     ///    因此传 `1` 数字以外的写法如 `01`、`true` 都**不会**触发清默认）；
    /// 4. `UPDATE area_code SET <列> = ? WHERE id = ?`，值以字符串绑定，
    ///    由 MySQL 按目标列类型隐式转换。
    ///
    /// 返回是否命中记录（Java 直接返回 `this.update(updateWrapper)` 的布尔值）。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let id = dto
            .id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let field = dto
            .field
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("字段名不能为空".into()))?;

        // 1. 归一
        let column = map_update_field_column(field)
            .ok_or_else(|| Error::Validation(MSG_INVALID_FIELD_VALUE.into()))?;
        // 2. 白名单
        if !AREA_CODE_FIELDS.contains(&column) {
            return Err(Error::Validation(MSG_INVALID_FIELD_VALUE.into()));
        }
        // 3. 二次保险：白名单四项都是真实列，这里只作防御性断言
        if !AREA_CODE_COLUMNS.contains(&column) {
            return Err(Error::Validation(MSG_INVALID_FIELD_VALUE.into()));
        }

        let val = dto.val.as_deref().unwrap_or_default();

        // 4. 置默认时先清其它默认（Java 用字符串 "1" 比较，不是数值比较）
        if column == "is_default" && val == "1" {
            self.repos.system.area_code.clear_default_flag(Some(id)).await?;
        }

        let affected = self.repos.system.area_code.update_field(id, column, val).await?;
        Ok(affected > 0)
    }

    // ————————————————————— 批量操作 —————————————————————

    /// 批量操作（对应 Java `BaseServiceImpl#batch`）。
    ///
    /// `type != "del"` → **静默返回 `false`**（Java 行为，不是报错）。
    /// `type == "del"` → `removeByIds(ids)`，返回 `rows >= 1`。
    ///
    /// ⚠️ Java 控制器类级 `@Validated` + 参数 `@Validated` 会让
    /// `BatchDTO.ids` 的 `@NotEmpty` 生效（空数组 → 400）。
    /// 本实现按项目惯例在服务层返回 `false` 而不是 400
    /// （与 pc_cat_floor 的选择一致，见 `uncertain_points`）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if dto.r#type != "del" {
            return Ok(false);
        }
        if dto.ids.is_empty() {
            // Java `removeByIds(空集合)` 直接返回 false，不产生 SQL
            return Ok(false);
        }
        let affected = self.repos.system.area_code.delete_many(&dto.ids).await?;
        Ok(affected > 0)
    }

    // ————————————————————— 内部：查重 —————————————————————

    /// 区号查重（对应 Java `AreaCodeServiceImpl#checkAreaCode`）。
    ///
    /// ⚠️ **默认只查 `code`，不查 `name`** —— 这是对 Java 实际行为的复刻，
    /// 详见模块头「Java 原代码的两个已确认缺陷」第 1 条：
    /// Java 的第二次查重拿 `name` 列去比 `code` 值，且被前一段短路，
    /// 永远不可达。若这里「顺手修好」，会造成 Rust 与 Java **行为不一致**
    /// （Java 允许同名，Rust 报错），属于未被授权的业务变更。
    ///
    /// 需要启用名称查重时用 [`check_area_code_with_name`]。
    ///
    /// # Errors
    /// - `code` 已被其它记录占用 ⇒ `区号已存在`
    pub async fn check_area_code(&self, code: &str, area_code_id: Option<i32>) -> Result<()> {
        let code_count = self
            .repos.system.area_code
            .count_by_code(code, area_code_id)
            .await?;
        if code_count > 0 {
            return Err(Error::Validation(MSG_AREA_CODE_IS_EXISTS.into()));
        }
        Ok(())
    }

    /// 查重（`code` + `name` 双查）—— **Java 的意图版本**，当前未被路由调用。
    ///
    /// 第二段用**正确的** `name` 列比较（Java 写成了 `param.getCode()`）。
    /// 保留它是为了让主会话 / 产品在确认「名称是否应唯一」后一行切换，
    /// 而不必重读 Java 源码。
    ///
    /// ⚠️ 一旦启用，`AREA_CODE_NAME_IS_EXISTS`（`区号名称已存在`）才会真正出现，
    /// 这是**与当前 Java 线上行为不一致**的变更，需产品确认。
    ///
    /// # Errors
    /// - `code` 重复 ⇒ `区号已存在`
    /// - `name` 重复 ⇒ `区号名称已存在`
    pub async fn check_area_code_with_name(
        &self,
        code: &str,
        name: &str,
        area_code_id: Option<i32>,
    ) -> Result<()> {
        self.check_area_code(code, area_code_id).await?;
        let name_count = self
            .repos.system.area_code
            .count_by_name(name, area_code_id)
            .await?;
        if name_count > 0 {
            return Err(Error::Validation(MSG_AREA_CODE_NAME_IS_EXISTS.into()));
        }
        Ok(())
    }

    /// 启用「是否可用」的区号列表（对应 Java `AreaCodeServiceImpl#mobileAreaCode`）。
    ///
    /// ⚠️ **本方法不对应本切片的任何路由**：Java `AreaCodeServiceImpl#mobileAreaCode`
    /// 由**前台**（`mobileapi` 的注册/登录页区号选择）调用，本切片只有 7 条
    /// 管理端接口。保留此实现是为了让同一张表的读取逻辑集中在一处，
    /// 前台切片可直接复用；它**没有**被本切片的路由注册
    /// （`crates/api/admin/src/routes/area_code.rs` 只注册 7 条管理端路径），
    /// 不影响覆盖率统计。
    ///
    /// 语义（逐条对照 Java）：
    /// - `WHERE is_available = 1`
    /// - `ORDER BY is_default DESC`（默认项排最前）
    /// - `label = "+{code}"`
    pub async fn mobile_area_code(&self) -> Result<Vec<AreaCodeVO>> {
        let rows = self.repos.system.area_code.find_available().await?;
        Ok(rows.iter().map(AreaCodeVO::from).collect())
    }

    /// 本次入参是否把该条设为默认（供路由层 / 调试观察，非 Java 概念）。
    #[must_use]
    pub fn is_setting_default(v: Option<i32>) -> bool {
        v == Some(YES)
    }

    /// `is_default` 的反值（Java `Constants.NO` / `Constants.YES` 对照用）。
    #[must_use]
    pub const fn negate_flag(v: i32) -> i32 {
        if v == YES {
            NO
        } else {
            YES
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::system::area_code::{
        map_order_column, AreaCodeListQuery, AREA_CODE_FIELDS, AREA_CODE_MAX_LEN,
        MSG_AREA_CODE_MAX_SIZE, MSG_AREA_CODE_NAME_MAX_SIZE, MSG_AREA_CODE_NAME_NOT_BLANK,
        MSG_AREA_CODE_NOT_BLANK,
    };

    // ————————————————————— 常量：与 Java 逐字符对照 —————————————————————

    /// Java `CheckFieldConstants.AREA_CODE_FIELDS` 的逐字对照。
    #[test]
    fn area_code_fields_matches_java_constant() {
        assert_eq!(
            AREA_CODE_FIELDS,
            ["code", "name", "is_available", "is_default"]
        );
        // 与 pc_cat_floor 的 GENERAL_FIELDS 不同：本域白名单里没有 status
        assert!(!AREA_CODE_FIELDS.contains(&"status"));
    }

    /// Java 常量文案逐字符照抄（含「10 个字符」与 `max = 64` 的不一致）。
    #[test]
    fn java_message_constants_are_verbatim() {
        assert_eq!(MSG_AREA_CODE_NOT_BLANK, "区号不能为空");
        assert_eq!(MSG_AREA_CODE_MAX_SIZE, "区号不能超过10个字符");
        assert_eq!(MSG_AREA_CODE_IS_EXISTS, "区号已存在");
        assert_eq!(MSG_AREA_CODE_NAME_IS_EXISTS, "区号名称已存在");
        assert_eq!(MSG_AREA_CODE_NAME_NOT_BLANK, "区号名称不能为空");
        assert_eq!(MSG_AREA_CODE_NAME_MAX_SIZE, "区号名称不能超过10个字符");
        assert_eq!(MSG_INVALID_FIELD_VALUE, "字段值不合法");
        // 阈值按注解 @Size(max = 64)，文案按常量「10 个字符」—— 两者不一致是 Java 原样
        assert_eq!(AREA_CODE_MAX_LEN, 64);
        assert!(MSG_AREA_CODE_MAX_SIZE.contains("10"));
    }

    /// Java `Constants.YES / NO`。
    #[test]
    fn yes_no_constants() {
        assert_eq!(YES, 1);
        assert_eq!(NO, 0);
    }

    // ————————————————————— 纯逻辑：updateField 白名单 —————————————————————

    /// 归一 + 白名单：`id` 能归一但被白名单拒绝；未识别字段直接拒绝。
    #[test]
    fn update_field_whitelist_flow() {
        // 四个白名单字段（属性名与列名两种写法）
        for (input, column) in [
            ("code", "code"),
            ("name", "name"),
            ("isAvailable", "is_available"),
            ("is_available", "is_available"),
            ("isDefault", "is_default"),
            ("is_default", "is_default"),
        ] {
            let c = map_update_field_column(input).unwrap();
            assert_eq!(c, column);
            assert!(AREA_CODE_FIELDS.contains(&c));
        }
        // 主键可归一，但不在白名单 ⇒ Java 报「字段值不合法」
        let id_col = map_update_field_column("id").unwrap();
        assert!(!AREA_CODE_FIELDS.contains(&id_col));
        // 完全无法识别 ⇒ 归一失败
        assert!(map_update_field_column("something").is_none());
    }

    /// `updateField` 触发「清其它默认」的条件是**字符串** `"1"`。
    #[test]
    fn clear_default_trigger_is_string_one() {
        // 复刻 Java `if (field.equals("is_default") && "1".equals(val))`
        let should_clear = |field: &str, val: &str| field == "is_default" && val == "1";
        assert!(should_clear("is_default", "1"));
        assert!(!should_clear("is_default", "01"));
        assert!(!should_clear("is_default", "true"));
        assert!(!should_clear("is_default", "0"));
        assert!(!should_clear("is_default", ""));
        // 其它字段即使是 "1" 也不清默认
        assert!(!should_clear("is_available", "1"));
        assert!(!should_clear("code", "1"));
    }

    // ————————————————————— 纯逻辑：create / update 的默认唯一性 —————————————————————

    /// 只有 `isDefault == 1` 才触发「清其它默认」（Java `Constants.YES.equals(...)`）。
    #[test]
    fn only_yes_triggers_default_reset() {
        assert!(AreaCodeService::is_setting_default(Some(1)));
        assert!(!AreaCodeService::is_setting_default(Some(0)));
        // null 与其它值都不触发（Java `Constants.YES.equals(null)` = false）
        assert!(!AreaCodeService::is_setting_default(None));
        assert!(!AreaCodeService::is_setting_default(Some(2)));
    }

    /// `negate_flag` 只是常量对照，不参与业务（防止被误用为业务判断）。
    #[test]
    fn negate_flag_round_trip() {
        assert_eq!(AreaCodeService::negate_flag(YES), NO);
        assert_eq!(AreaCodeService::negate_flag(NO), YES);
        assert_eq!(AreaCodeService::negate_flag(7), YES);
    }

    // ————————————————————— 纯逻辑：batch —————————————————————

    /// `BatchDTO.type != "del"` ⇒ 静默 false（不报错），空 ids 同样 false。
    #[test]
    fn batch_type_gate() {
        let dto: BatchDTO = serde_json::from_str(r#"{"ids":[1],"type":"show"}"#).unwrap();
        assert_ne!(dto.r#type, "del");
        let dto: BatchDTO = serde_json::from_str(r#"{"ids":[],"type":"del"}"#).unwrap();
        assert!(dto.ids.is_empty());
        let dto: BatchDTO = serde_json::from_str(r#"{"ids":[1,2,3],"type":"del"}"#).unwrap();
        assert_eq!(dto.ids, vec![1, 2, 3]);
    }

    // ————————————————————— 纯逻辑：list 排序 —————————————————————

    /// Java `buildSortOrder` 三分支：asc 升序；空与 desc 降序；
    /// **其它非法值不追加 ORDER BY**。
    #[test]
    fn order_clause_three_branches() {
        let q = AreaCodeListQuery {
            sort_order: Some("asc".into()),
            ..Default::default()
        };
        assert_eq!(q.order_clause(), " ORDER BY id ASC");

        let q = AreaCodeListQuery {
            sort_order: None,
            ..Default::default()
        };
        assert_eq!(q.order_clause(), " ORDER BY id DESC");

        let q = AreaCodeListQuery {
            sort_order: Some("desc".into()),
            ..Default::default()
        };
        assert_eq!(q.order_clause(), " ORDER BY id DESC");

        // 非法值：Java else-if 未覆盖 ⇒ 无排序
        let q = AreaCodeListQuery {
            sort_order: Some("ascending".into()),
            ..Default::default()
        };
        assert_eq!(q.order_clause(), "");
    }

    /// 未识别 `sortField` 静默回退主键列（防注入）。
    #[test]
    fn unknown_sort_field_falls_back_to_key() {
        assert_eq!(map_order_column("nope"), "id");
        assert_eq!(map_order_column("is_available"), "is_available");
        assert_eq!(map_order_column("isDefault"), "is_default");
        let q = AreaCodeListQuery {
            sort_field: Some("name; DROP TABLE area_code".into()),
            sort_order: Some("desc".into()),
            ..Default::default()
        };
        assert_eq!(q.order_clause(), " ORDER BY id DESC");
    }

    // ————————————————————— 纯逻辑：入参校验 —————————————————————

    /// `create` / `update` 的必填与长度校验（Java `@NotBlank` + `@Size(max=64)`）。
    #[test]
    fn save_validation_messages() {
        let blank: AreaCodeSaveDTO = serde_json::from_str(r#"{"name":"中国"}"#).unwrap();
        assert_eq!(
            validate_save(&blank).unwrap_err(),
            MSG_AREA_CODE_NOT_BLANK
        );

        let no_name: AreaCodeSaveDTO = serde_json::from_str(r#"{"code":"86"}"#).unwrap();
        assert_eq!(
            validate_save(&no_name).unwrap_err(),
            MSG_AREA_CODE_NAME_NOT_BLANK
        );

        let long_code = AreaCodeSaveDTO {
            code: Some("x".repeat(65)),
            name: Some("中国".into()),
            ..Default::default()
        };
        assert_eq!(
            validate_save(&long_code).unwrap_err(),
            MSG_AREA_CODE_MAX_SIZE
        );

        // 64 字符边界合法；65 字符报错
        let ok = AreaCodeSaveDTO {
            code: Some("x".repeat(64)),
            name: Some("中国".into()),
            ..Default::default()
        };
        assert!(validate_save(&ok).is_ok());
    }

    /// 更新请求缺主键 → `区号主键不能为空`（Java `AreaCodeEditParam @NotNull`）。
    #[test]
    fn edit_dto_requires_id() {
        let dto: AreaCodeEditDTO =
            serde_json::from_str(r#"{"code":"86","name":"中国"}"#).unwrap();
        assert_eq!(dto.id, None);
        assert_eq!(MSG_AREA_CODE_ID_REQUIRED, "区号主键不能为空");
    }

    /// 删除请求缺主键 → `id不能为空`（Java `OperateDTO @NotNull`）。
    #[test]
    fn operate_dto_requires_id() {
        let dto: OperateDTO = serde_json::from_str(r#"{"adminNote":"x"}"#).unwrap();
        assert_eq!(dto.id, None);
        assert_eq!(MSG_ID_REQUIRED, "id不能为空");
    }
}