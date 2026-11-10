//! 品类服务费率规则域（payment::category_fee_rule）DTO / VO / 查询参数 / 纯函数。
//!
//! 对应原 Java：
//! - `com.nextrade.bean.dto.finance.feerule.{CategoryFeeRulePageDTO,
//!   CategoryFeeRuleSaveDTO, CategoryFeeRuleUpdateStatusDTO,
//!   CategoryFeeRuleIdsDTO, CategoryFeeRuleConflictCheckDTO}`
//! - `com.nextrade.bean.vo.finance.feerule.{CategoryFeeRuleListVO,
//!   CategoryFeeRuleDetailVO, CategoryFeeRuleConflictVO}`
//! - `com.nextrade.bean.enums.finance.{RuleTargetTypeEnum, RuleScopeTypeEnum}`
//! - `com.nextrade.service.finance.impl.CategoryFeeRuleServiceImpl`
//!
//! 实体已由 `nextrade_core::entities::{CategoryServiceFeeRule,
//! CategoryServiceFeeRuleCategory, CategoryServiceFeeRuleTarget}` 自动生成，
//! 本文件不重复定义。金额 `service_fee_rate` 是 `decimal(5,2)` → `Decimal`。
//!
//! # 关键语义（逐条对照 Java）
//!
//! - `page`：`target_type` **必填**（`@NotNull`）；`status` / `keyword` 可选；
//!   `deleted = 0` 恒定；排序走三参 `buildSortOrder`（默认主键降序）。
//! - `save/update/updateStatus` 返回 `List<String>`（Java 恒为空列表）。
//! - 保存前先做 **冲突预检**（`assertNoConflict`）：存在同 `target_type` 且
//!   启用、类目相交、范围相交的规则即拒绝，文案列出前 5 个规则名。
//! - 冲突判定 `hasTargetOverlap` 的语义是纯函数，见 [`has_target_overlap`]。
//! - `detail` 规则不存在 ⇒「费率规则不存在」。
//! - `delete` 是**逻辑删除**（`deleted = 1`），同时物理删除类目/目标关联。
//! - `updateStatus` 仅在启用（`status = 1`）时做冲突预检。
//! - Redis 匹配缓存失效（`evictCacheByTargetType`）未迁移（见服务层说明）。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ——————————————————————————————————————————————
// 常量
// ——————————————————————————————————————————————

/// `category_service_fee_rule` 的 10 个列。
pub const RULE_COLUMNS: [&str; 10] = [
    "id",
    "rule_name",
    "target_type",
    "scope_type",
    "service_fee_rate",
    "remark",
    "status",
    "deleted",
    "create_time",
    "modify_time",
];

/// `category_service_fee_rule_category` 的 8 个列。
pub const CATEGORY_COLUMNS: [&str; 8] = [
    "id",
    "rule_id",
    "category_id",
    "category_name",
    "category_path",
    "deleted",
    "create_time",
    "modify_time",
];

/// `category_service_fee_rule_target` 的 6 个列。
pub const TARGET_COLUMNS: [&str; 6] = [
    "id",
    "rule_id",
    "target_id",
    "deleted",
    "create_time",
    "modify_time",
];

/// 主键列（Java `BaseServiceImpl#getKeyColumn` → `id`）。
pub const KEY_COLUMN: &str = "id";

/// 主键属性名（Java `getKeyProperty` → `id`）。
pub const KEY_PROPERTY: &str = "id";

/// 适用对象：店铺（Java `RuleTargetTypeEnum.SHOP`）。
pub const TARGET_TYPE_SHOP: i32 = 1;
/// 适用对象：供应商（Java `RuleTargetTypeEnum.VENDOR`）。
pub const TARGET_TYPE_VENDOR: i32 = 2;

/// 范围：全部（Java `RuleScopeTypeEnum.ALL`）。
pub const SCOPE_ALL: i32 = 1;
/// 范围：指定（Java `RuleScopeTypeEnum.SPECIFIED`）。
pub const SCOPE_SPECIFIED: i32 = 2;
/// 范围：不包含（Java `RuleScopeTypeEnum.EXCLUDE`）。
pub const SCOPE_EXCLUDE: i32 = 3;

/// 冲突文案里最多列出的规则名数量（Java `MAX_CONFLICT_NAMES_IN_MESSAGE`）。
pub const MAX_CONFLICT_NAMES_IN_MESSAGE: usize = 5;

// ————————————————————— 文案（Bean Validation / GlobalException 原文） —————————————————————

pub const MSG_TARGET_TYPE_REQUIRED: &str = "适用对象类型不能为空";
pub const MSG_SCOPE_TYPE_REQUIRED: &str = "范围类型不能为空";
pub const MSG_RATE_REQUIRED: &str = "服务费率不能为空";
pub const MSG_STATUS_REQUIRED: &str = "状态不能为空";
pub const MSG_CATEGORY_IDS_REQUIRED: &str = "关联类目不能为空";
pub const MSG_SPECIFIED_TARGET_REQUIRED: &str = "指定范围时关联对象不能为空";
pub const MSG_EXCLUDE_TARGET_REQUIRED: &str = "不包含范围时排除对象不能为空";
pub const MSG_RULE_NAME_REQUIRED: &str = "规则名称不能为空";
pub const MSG_RULE_NAME_TOO_LONG: &str = "规则名称最多50个字符";
pub const MSG_RATE_TOO_SMALL: &str = "服务费率不能小于0";
pub const MSG_RATE_TOO_LARGE: &str = "服务费率不能大于100";
pub const MSG_REMARK_TOO_LONG: &str = "备注最多200个字符";
pub const MSG_RULE_NOT_EXISTS: &str = "费率规则不存在";
pub const MSG_RULE_ID_REQUIRED: &str = "规则ID不能为空";
pub const MSG_DUPLICATE_NAME: &str = "同类型下规则名称已存在";
pub const MSG_IDS_REQUIRED: &str = "请选择要删除的规则";
pub const MSG_CONFLICT_PREFIX: &str = "存在冲突规则，无法保存：";

// ——————————————————————————————————————————————
// 枚举名 / 纯函数
// ——————————————————————————————————————————————

/// 适用对象类型文案（Java `RuleTargetTypeEnum.getLabel`；未知返回 `""`）。
#[must_use]
pub fn target_type_name(target_type: Option<i32>) -> &'static str {
    match target_type {
        Some(TARGET_TYPE_SHOP) => "店铺",
        Some(TARGET_TYPE_VENDOR) => "供应商",
        _ => "",
    }
}

/// 范围类型文案（Java `RuleScopeTypeEnum.getLabel`；未知返回 `""`）。
#[must_use]
pub fn scope_type_name(scope_type: Option<i32>) -> &'static str {
    match scope_type {
        Some(SCOPE_ALL) => "全部",
        Some(SCOPE_SPECIFIED) => "指定",
        Some(SCOPE_EXCLUDE) => "不包含",
        _ => "",
    }
}

/// 是否需要关联对象 ID（Java `RuleScopeTypeEnum.requiresTargetIds`）。
#[must_use]
pub fn requires_target_ids(scope_type: i32) -> bool {
    scope_type == SCOPE_SPECIFIED || scope_type == SCOPE_EXCLUDE
}

/// 两个范围是否相交（Java `CategoryFeeRuleServiceImpl#hasTargetOverlap` 逐分支照抄）。
#[must_use]
pub fn has_target_overlap(
    scope_a: i32,
    targets_a: &[i32],
    scope_b: i32,
    targets_b: &[i32],
) -> bool {
    if scope_a == SCOPE_ALL || scope_b == SCOPE_ALL {
        return true;
    }
    let (a_specified, b_specified) = (scope_a == SCOPE_SPECIFIED, scope_b == SCOPE_SPECIFIED);
    if a_specified && b_specified {
        return targets_a.iter().any(|id| targets_b.contains(id));
    }
    let (a_exclude, b_exclude) = (scope_a == SCOPE_EXCLUDE, scope_b == SCOPE_EXCLUDE);
    if a_exclude && b_exclude {
        return true;
    }
    if a_specified && b_exclude {
        return targets_a.iter().any(|id| !targets_b.contains(id));
    }
    if a_exclude && b_specified {
        return targets_b.iter().any(|id| !targets_a.contains(id));
    }
    false
}

/// 冲突拒绝文案（Java `assertNoConflict`）：去重规则名取前 5 个，`、` 连接，
/// 超出加「等共N条」。
#[must_use]
pub fn conflict_message<'a>(names: impl IntoIterator<Item = &'a str>) -> String {
    let mut distinct: Vec<&str> = Vec::new();
    for name in names {
        if !distinct.contains(&name) {
            distinct.push(name);
        }
    }
    let total = distinct.len();
    let preview = distinct
        .iter()
        .take(MAX_CONFLICT_NAMES_IN_MESSAGE)
        .copied()
        .collect::<Vec<_>>()
        .join("、");
    let suffix = if total > MAX_CONFLICT_NAMES_IN_MESSAGE {
        format!("等共{total}条")
    } else {
        String::new()
    };
    format!("{MSG_CONFLICT_PREFIX}{preview}{suffix}")
}

/// 排序字段 → 列名（Java `getColumnByPropertyName`：属性名/列名命中，否则回退主键）。
#[must_use]
pub fn map_order_column(field: &str) -> &'static str {
    match field {
        "id" => "id",
        "ruleName" | "rule_name" => "rule_name",
        "targetType" | "target_type" => "target_type",
        "scopeType" | "scope_type" => "scope_type",
        "serviceFeeRate" | "service_fee_rate" => "service_fee_rate",
        "remark" => "remark",
        "status" => "status",
        "deleted" => "deleted",
        "createTime" | "create_time" => "create_time",
        "modifyTime" | "modify_time" => "modify_time",
        _ => KEY_COLUMN,
    }
}

/// `LocalDateTime.now()` 等价文本（MySQL `DATETIME`，UTC+8，`yyyy-MM-dd HH:mm:ss`）。
#[must_use]
pub fn now_datetime() -> String {
    (chrono::Utc::now().naive_utc() + chrono::Duration::hours(8))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

/// `save/update` 入参的 Bean Validation（Java 注解逐条对齐）。
fn validate_rule_name(name: Option<&str>) -> Result<(), String> {
    let Some(name) = name else {
        return Err(MSG_RULE_NAME_REQUIRED.to_string());
    };
    if name.trim().is_empty() {
        return Err(MSG_RULE_NAME_REQUIRED.to_string());
    }
    if name.chars().count() > 50 {
        return Err(MSG_RULE_NAME_TOO_LONG.to_string());
    }
    Ok(())
}

fn validate_optional_len(value: Option<&str>, max: usize, msg: &str) -> Result<(), String> {
    if value.is_some_and(|v| v.chars().count() > max) {
        return Err(msg.to_string());
    }
    Ok(())
}

// ——————————————————————————————————————————————
// 入参
// ——————————————————————————————————————————————

/// 分页查询（Java `CategoryFeeRulePageDTO extends BasePage`）。
///
/// ⚠️ 原版 admin 页面（`views/finance/categoryServiceFeeRule/List.vue`）的
/// `defaultParams` 把 `status` 初始化成**空字符串** `""`，整包原样 POST：
/// ```json
/// {"page":1,"size":10,"sortField":"","sortOrder":"","keyword":"","status":"","targetType":1}
/// ```
/// Jackson 绑 `Integer` 时把 `""` 当 `null`；serde 严格解析则会让整个请求
/// **422**（2026-10-21 巡检 `/admin/finance/category-service-fee-rule` 的根因）。
/// 故可选数值字段统一走宽松解析——空串/非法串 → `None`。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CategoryFeeRulePageQuery {
    /// 适用对象类型：1 店铺 2 供应商（`@NotNull`）
    #[serde(default, deserialize_with = "de_opt_i32_lenient")]
    pub target_type: Option<i32>,
    /// 状态 1 启用 0 禁用（前端可能传 `""`）
    #[serde(default, deserialize_with = "de_opt_i32_lenient")]
    pub status: Option<i32>,
    /// 规则名称关键字（`ILIKE`）
    pub keyword: Option<String>,
    /// 页码
    #[serde(default, deserialize_with = "de_opt_i64_lenient")]
    pub page: Option<i64>,
    /// 每页条数（Java `size`）
    #[serde(default, deserialize_with = "de_opt_i64_lenient")]
    pub size: Option<i64>,
    /// 排序字段
    pub sort_field: Option<String>,
    /// 排序方向
    pub sort_order: Option<String>,
}

/// 宽松地把 JSON 值解析为 `Option<i32>`：`""` / 非数字串 / 其它类型 → `None`。
///
/// 与 Java Jackson 的 `ACCEPT_EMPTY_STRING_AS_NULL_OBJECT` 行为对齐
/// （见 `CategoryFeeRulePageQuery` 注释）。
fn de_opt_i32_lenient<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(serde_json::Value::Number(n)) => n.as_i64().and_then(|v| i32::try_from(v).ok()),
        Some(serde_json::Value::String(s)) => s.trim().parse::<i32>().ok(),
        _ => None,
    })
}

/// 宽松地把 JSON 值解析为 `Option<i64>`（同上，用于 `page` / `size`）。
fn de_opt_i64_lenient<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(serde_json::Value::Number(n)) => n.as_i64(),
        Some(serde_json::Value::String(s)) => s.trim().parse::<i64>().ok(),
        _ => None,
    })
}

impl CategoryFeeRulePageQuery {
    /// 校验 `targetType` 必填（`@NotNull(message = "适用对象类型不能为空")`）。
    pub fn target_type_required(&self) -> Result<i32, String> {
        self.target_type
            .ok_or_else(|| MSG_TARGET_TYPE_REQUIRED.to_string())
    }

    /// `(LIMIT, OFFSET)`（Java `BasePage` 口径：page null→1，size null→15、>100→100）。
    #[must_use]
    pub fn limit_offset(&self) -> (i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let size = match self.size {
            None => 15,
            Some(v) if v > 100 => 100,
            Some(v) if v < 1 => 1,
            Some(v) => v,
        };
        (size, (page - 1) * size)
    }

    /// ORDER BY（三参 `buildSortOrder`：默认主键降序，非法方向不追加）。
    #[must_use]
    pub fn order_clause(&self) -> String {
        let field = self.sort_field.as_deref().map(str::trim).unwrap_or("");
        let column = if field.is_empty() || field == KEY_PROPERTY || field == KEY_COLUMN {
            KEY_COLUMN
        } else {
            map_order_column(field)
        };
        match self.sort_order.as_deref().map(str::trim) {
            Some(s) if s.eq_ignore_ascii_case("asc") => format!(" ORDER BY {column} ASC"),
            None | Some("") => format!(" ORDER BY {column} DESC"),
            Some(s) if s.eq_ignore_ascii_case("desc") => format!(" ORDER BY {column} DESC"),
            Some(_) => String::new(),
        }
    }
}

/// 保存/编辑入参（Java `CategoryFeeRuleSaveDTO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CategoryFeeRuleSaveDTO {
    /// 规则 ID（编辑时传）
    pub id: Option<i32>,
    /// 规则名称（`@NotBlank` + `@Size(max = 50)`）
    pub rule_name: Option<String>,
    /// 适用对象类型（`@NotNull`）
    pub target_type: Option<i32>,
    /// 范围类型（`@NotNull`）
    pub scope_type: Option<i32>,
    /// 服务费率（`@NotNull` + `DecimalMin 0` + `DecimalMax 100`）
    pub service_fee_rate: Option<Decimal>,
    /// 备注（`@Size(max = 200)`）
    pub remark: Option<String>,
    /// 状态（`@NotNull`）
    pub status: Option<i32>,
    /// 关联类目（`@NotEmpty(message = "关联类目不能为空")`）
    pub category_ids: Option<Vec<i32>>,
    /// 关联对象 ID：指定 = 包含；不包含 = 排除名单
    pub target_ids: Option<Vec<i32>>,
}

impl CategoryFeeRuleSaveDTO {
    /// Java `validateSaveDto` + Bean Validation（顺序：注解先于服务层校验）。
    pub fn validate(&self) -> Result<(), String> {
        validate_rule_name(self.rule_name.as_deref())?;
        self.target_type
            .ok_or_else(|| MSG_TARGET_TYPE_REQUIRED.to_string())?;
        let scope_type = self
            .scope_type
            .ok_or_else(|| MSG_SCOPE_TYPE_REQUIRED.to_string())?;
        let rate = self
            .service_fee_rate
            .ok_or_else(|| MSG_RATE_REQUIRED.to_string())?;
        if rate < Decimal::ZERO {
            return Err(MSG_RATE_TOO_SMALL.to_string());
        }
        if rate > Decimal::from(100) {
            return Err(MSG_RATE_TOO_LARGE.to_string());
        }
        validate_optional_len(self.remark.as_deref(), 200, MSG_REMARK_TOO_LONG)?;
        self.status.ok_or_else(|| MSG_STATUS_REQUIRED.to_string())?;
        if self.category_ids.as_deref().is_none_or(|v| v.is_empty()) {
            return Err(MSG_CATEGORY_IDS_REQUIRED.to_string());
        }
        if scope_type == SCOPE_SPECIFIED && self.target_ids.as_deref().is_none_or(|v| v.is_empty())
        {
            return Err(MSG_SPECIFIED_TARGET_REQUIRED.to_string());
        }
        if scope_type == SCOPE_EXCLUDE && self.target_ids.as_deref().is_none_or(|v| v.is_empty()) {
            return Err(MSG_EXCLUDE_TARGET_REQUIRED.to_string());
        }
        Ok(())
    }

    /// 冲突预检入参视图。
    #[must_use]
    pub fn to_conflict_dto(&self) -> CategoryFeeRuleConflictCheckDTO {
        CategoryFeeRuleConflictCheckDTO {
            rule_id: self.id,
            target_type: self.target_type,
            scope_type: self.scope_type,
            category_ids: self.category_ids.clone(),
            target_ids: self.target_ids.clone(),
        }
    }
}

/// 启用/禁用入参（Java `CategoryFeeRuleUpdateStatusDTO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CategoryFeeRuleUpdateStatusDTO {
    /// 规则 ID（`@NotNull(message = "规则ID不能为空")`）
    pub id: Option<i32>,
    /// 状态（`@NotNull(message = "状态不能为空")`）
    pub status: Option<i32>,
}

impl CategoryFeeRuleUpdateStatusDTO {
    /// 注解校验。
    pub fn validate(&self) -> Result<(i32, i32), String> {
        let id = self.id.ok_or_else(|| MSG_RULE_ID_REQUIRED.to_string())?;
        let status = self.status.ok_or_else(|| MSG_STATUS_REQUIRED.to_string())?;
        Ok((id, status))
    }
}

/// 批量删除入参（Java `CategoryFeeRuleIdsDTO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CategoryFeeRuleIdsDTO {
    /// 规则 ID 列表（`@NotEmpty(message = "请选择要删除的规则")`）
    pub ids: Option<Vec<i32>>,
}

impl CategoryFeeRuleIdsDTO {
    /// 注解校验。
    pub fn validate(&self) -> Result<Vec<i32>, String> {
        match self.ids.clone() {
            Some(ids) if !ids.is_empty() => Ok(ids),
            _ => Err(MSG_IDS_REQUIRED.to_string()),
        }
    }
}

/// 冲突预检入参（Java `CategoryFeeRuleConflictCheckDTO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CategoryFeeRuleConflictCheckDTO {
    /// 当前规则 ID（更新时排除自身）
    pub rule_id: Option<i32>,
    /// 适用对象类型（`@NotNull`）
    pub target_type: Option<i32>,
    /// 范围类型（`@NotNull`）
    pub scope_type: Option<i32>,
    /// 关联类目（`@NotEmpty`）
    pub category_ids: Option<Vec<i32>>,
    /// 关联对象 ID
    pub target_ids: Option<Vec<i32>>,
}

impl CategoryFeeRuleConflictCheckDTO {
    /// 注解校验（注意：Java 冲突预检不校验「指定/不包含必须有 targetIds」，
    /// 该规则只在 save/update 的 `validateSaveDto` 里）。
    pub fn validate(&self) -> Result<(i32, i32, Vec<i32>), String> {
        let target_type = self
            .target_type
            .ok_or_else(|| MSG_TARGET_TYPE_REQUIRED.to_string())?;
        let scope_type = self
            .scope_type
            .ok_or_else(|| MSG_SCOPE_TYPE_REQUIRED.to_string())?;
        let category_ids = match self.category_ids.clone() {
            Some(ids) if !ids.is_empty() => ids,
            _ => return Err(MSG_CATEGORY_IDS_REQUIRED.to_string()),
        };
        Ok((target_type, scope_type, category_ids))
    }
}

// ——————————————————————————————————————————————
// 出参 VO
// ——————————————————————————————————————————————

/// 列表项（Java `CategoryFeeRuleListVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryFeeRuleListVO {
    pub id: i32,
    pub rule_name: String,
    pub target_type: i32,
    pub target_type_name: String,
    pub scope_type: i32,
    pub scope_type_name: String,
    /// Java 实体无该字段（未赋值）⇒ 恒为 null
    pub precision_layer: Option<i32>,
    pub service_fee_rate: Decimal,
    pub target_count: i32,
    pub category_count: i32,
    pub status: i32,
    pub create_time: Option<String>,
}

/// 详情（Java `CategoryFeeRuleDetailVO extends CategoryFeeRuleListVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryFeeRuleDetailVO {
    #[serde(flatten)]
    pub base: CategoryFeeRuleListVO,
    pub remark: String,
    pub categories: Vec<CategoryFeeRuleCategoryItem>,
    pub targets: Vec<CategoryFeeRuleTargetItem>,
    pub modify_time: Option<String>,
}

/// 详情内品类项（Java `CategoryFeeRuleDetailVO.CategoryItem`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryFeeRuleCategoryItem {
    pub category_id: i32,
    pub category_name: String,
    pub category_path: String,
}

/// 详情内目标项（Java `CategoryFeeRuleDetailVO.TargetItem`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryFeeRuleTargetItem {
    pub target_id: i32,
    /// 店铺名 / 供应商名（按 targetType 装配；缺失时为 `""`）
    pub target_name: String,
}

/// 冲突预检结果（Java `CategoryFeeRuleConflictVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryFeeRuleConflictVO {
    pub has_conflict: bool,
    pub conflicts: Vec<CategoryFeeRuleConflictItem>,
}

/// 冲突项（Java `CategoryFeeRuleConflictVO.ConflictItem`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryFeeRuleConflictItem {
    pub category_id: i32,
    pub category_name: String,
    pub existing_rule_id: i32,
    pub existing_rule_name: String,
    /// Java 未赋值 ⇒ null
    pub target_id: Option<i32>,
    /// Java 未赋值 ⇒ null
    pub target_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_names_and_requires_target_ids() {
        assert_eq!(target_type_name(Some(TARGET_TYPE_SHOP)), "店铺");
        assert_eq!(target_type_name(Some(TARGET_TYPE_VENDOR)), "供应商");
        assert_eq!(target_type_name(None), "");
        assert_eq!(target_type_name(Some(9)), "");
        assert_eq!(scope_type_name(Some(SCOPE_ALL)), "全部");
        assert_eq!(scope_type_name(Some(SCOPE_SPECIFIED)), "指定");
        assert_eq!(scope_type_name(Some(SCOPE_EXCLUDE)), "不包含");
        assert!(requires_target_ids(SCOPE_SPECIFIED));
        assert!(requires_target_ids(SCOPE_EXCLUDE));
        assert!(!requires_target_ids(SCOPE_ALL));
    }

    #[test]
    fn target_overlap_matches_java_branches() {
        // 任一为「全部」⇒ 相交
        assert!(has_target_overlap(SCOPE_ALL, &[], SCOPE_SPECIFIED, &[1]));
        assert!(has_target_overlap(SCOPE_EXCLUDE, &[1], SCOPE_ALL, &[]));
        // 指定 vs 指定 ⇒ 有交集才冲突
        assert!(has_target_overlap(
            SCOPE_SPECIFIED,
            &[1, 2],
            SCOPE_SPECIFIED,
            &[2, 3]
        ));
        assert!(!has_target_overlap(
            SCOPE_SPECIFIED,
            &[1],
            SCOPE_SPECIFIED,
            &[2]
        ));
        // 不包含 vs 不包含 ⇒ 恒相交
        assert!(has_target_overlap(SCOPE_EXCLUDE, &[1], SCOPE_EXCLUDE, &[2]));
        // 指定 vs 不包含 ⇒ 存在「不在排除名单里的指定对象」
        assert!(has_target_overlap(
            SCOPE_SPECIFIED,
            &[1, 2],
            SCOPE_EXCLUDE,
            &[2]
        ));
        assert!(!has_target_overlap(
            SCOPE_SPECIFIED,
            &[2],
            SCOPE_EXCLUDE,
            &[2]
        ));
        // 不包含 vs 指定 ⇒ 对称分支
        assert!(has_target_overlap(
            SCOPE_EXCLUDE,
            &[2],
            SCOPE_SPECIFIED,
            &[1, 2]
        ));
        assert!(!has_target_overlap(
            SCOPE_EXCLUDE,
            &[2],
            SCOPE_SPECIFIED,
            &[2]
        ));
    }

    #[test]
    fn conflict_message_limits_to_five_names() {
        assert_eq!(conflict_message(["规则A"]), "存在冲突规则，无法保存：规则A");
        assert_eq!(
            conflict_message(["A", "B", "A"]),
            "存在冲突规则，无法保存：A、B"
        );
        let names = ["A", "B", "C", "D", "E", "F", "G"];
        assert_eq!(
            conflict_message(names),
            "存在冲突规则，无法保存：A、B、C、D、E等共7条"
        );
    }

    #[test]
    fn save_dto_validation_messages() {
        let mut dto = CategoryFeeRuleSaveDTO {
            rule_name: Some("规则".into()),
            target_type: Some(TARGET_TYPE_SHOP),
            scope_type: Some(SCOPE_ALL),
            service_fee_rate: Some(Decimal::new(250, 2)),
            status: Some(1),
            category_ids: Some(vec![1]),
            ..Default::default()
        };
        // 2.50 合法
        assert!(dto.validate().is_ok());
        // 费率 > 100
        dto.service_fee_rate = Some(Decimal::from(101));
        assert_eq!(dto.validate().unwrap_err(), MSG_RATE_TOO_LARGE);
        dto.service_fee_rate = Some(Decimal::from(-1));
        assert_eq!(dto.validate().unwrap_err(), MSG_RATE_TOO_SMALL);
        dto.service_fee_rate = Some(Decimal::from(5));
        // 规则名空
        dto.rule_name = Some("   ".into());
        assert_eq!(dto.validate().unwrap_err(), MSG_RULE_NAME_REQUIRED);
        dto.rule_name = Some("a".repeat(51));
        assert_eq!(dto.validate().unwrap_err(), MSG_RULE_NAME_TOO_LONG);
        dto.rule_name = Some("规则".into());
        // 类目空
        dto.category_ids = Some(vec![]);
        assert_eq!(dto.validate().unwrap_err(), MSG_CATEGORY_IDS_REQUIRED);
        dto.category_ids = Some(vec![1]);
        // 指定范围必须有目标
        dto.scope_type = Some(SCOPE_SPECIFIED);
        dto.target_ids = None;
        assert_eq!(dto.validate().unwrap_err(), MSG_SPECIFIED_TARGET_REQUIRED);
        // 不包含范围必须有排除对象
        dto.scope_type = Some(SCOPE_EXCLUDE);
        assert_eq!(dto.validate().unwrap_err(), MSG_EXCLUDE_TARGET_REQUIRED);
        dto.target_ids = Some(vec![1]);
        assert!(dto.validate().is_ok());
    }

    #[test]
    fn page_query_order_and_paging() {
        let q = CategoryFeeRulePageQuery::default();
        assert_eq!(q.limit_offset(), (15, 0));
        assert_eq!(q.order_clause(), " ORDER BY id DESC");
        assert_eq!(
            q.target_type_required().unwrap_err(),
            MSG_TARGET_TYPE_REQUIRED
        );
        let q = CategoryFeeRulePageQuery {
            target_type: Some(1),
            sort_field: Some("createTime".into()),
            sort_order: Some("asc".into()),
            page: Some(3),
            size: Some(200),
            ..Default::default()
        };
        assert_eq!(q.order_clause(), " ORDER BY create_time ASC");
        assert_eq!(q.limit_offset(), (100, 200));
        // 非法方向不追加 ORDER BY
        let q = CategoryFeeRulePageQuery {
            sort_order: Some("ascending".into()),
            ..Default::default()
        };
        assert_eq!(q.order_clause(), "");
    }

    #[test]
    fn update_status_and_ids_validation() {
        assert_eq!(
            CategoryFeeRuleUpdateStatusDTO::default()
                .validate()
                .unwrap_err(),
            MSG_RULE_ID_REQUIRED
        );
        let dto = CategoryFeeRuleUpdateStatusDTO {
            id: Some(3),
            status: None,
        };
        assert_eq!(dto.validate().unwrap_err(), MSG_STATUS_REQUIRED);
        let dto = CategoryFeeRuleUpdateStatusDTO {
            id: Some(3),
            status: Some(1),
        };
        assert_eq!(dto.validate().unwrap(), (3, 1));
        assert_eq!(
            CategoryFeeRuleIdsDTO::default().validate().unwrap_err(),
            MSG_IDS_REQUIRED
        );
        assert_eq!(
            CategoryFeeRuleIdsDTO {
                ids: Some(vec![1, 2])
            }
            .validate()
            .unwrap(),
            vec![1, 2]
        );
    }

    #[test]
    fn now_datetime_format_is_mysql_style() {
        let now = now_datetime();
        assert_eq!(now.len(), 19, "yyyy-MM-dd HH:mm:ss");
        assert_eq!(&now[4..5], "-");
        assert_eq!(&now[13..14], ":");
    }
}
