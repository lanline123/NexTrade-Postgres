//! 买家秀分组管理服务（marketing::buyer_show_group）：新建 / 列表 / 详情 / 更新 / 删除。
//!
//! 对应原 Java：
//! - `com.nextrade.adminapi.controller.promotion.BuyerShowGroupController`
//!   （`@RequestMapping("/adminapi/promotion/buyerShow/group")`，5 个端点）
//! - `com.nextrade.service.promotion.impl.BuyerShowGroupServiceImpl`
//! - `com.nextrade.service.common.impl.BaseServiceImpl`（`del` / `buildSortOrder` /
//!   `getShopId` 三个基类能力）
//! - `com.nextrade.common.utils.HeaderUtils#getShopId`（多租户，见
//!   [`BuyerShowGroupScope`]）
//!
//! 迁移要点（逐条对照 Java 源）：
//! - `create`：`verifyGroupName`（**不带 `shop_id` 条件**，全库唯一）→ 落库
//!   `group_name` / `product_group_id`(JSON 文本) / `shop_id`。
//! - `list`：`shop_id` 过滤 → 分页 → 批量取商品分组名 → 逐行 `JSON_CONTAINS`
//!   统计买家秀数量；分页为空时按 Java `return new Page<>()` 返回空页。
//! - `detail`：按主键取（**无 `shop_id` 过滤**）→ 反序列化 `group_ids` →
//!   批量取商品分组名。
//! - `update`：按主键取（不存在 → 「数据不存在」，code 1001）→
//!   `verifyGroupName`（排除自身）→ 更新名称与商品分组。
//! - `del`：`removeById` 语义，影响 0 行不报错。
//!
//! **未迁移 / 已简化（详见返回值的 `uncertain_points`）**：
//! 1. **多语言翻译**：Java `translatePackage.translate(...)` 用于
//!    `groupName`（`@JsonTranslate(dataType = 16)`）与 `productGroupName`
//!    （`dataType = 1`）；翻译服务未迁移，本实现按**原文**返回。
//! 2. **`product_group` 名称的 NULL 元素**：Java `List<String>` 可以携带
//!    `null` 元素（`product_group_name` 列可空），Rust `Vec<String>` 不能，
//!    故跳过 NULL 名称（对应 JSON 里少一个 `null`）。
//! 3. **参数校验文案的顺序**：Java `GlobalExceptionHandler` 把
//!    `BindingResult` 的多个消息塞进 `HashMap` 后用 `", "` 连接，顺序不确定；
//!    本实现固定按「字段声明顺序」输出。
//! 4. `product_group_id` 的 JSON 文本非法 / 为空时，Java `JSONUtil.toList`
//!    会抛异常（500）或生成语法错误的 `IN ()`；本实现容错为空列表。
//!
//! 错误码：Java `GlobalException(String)` 与参数校验异常
//! （`MethodArgumentNotValidException`）统一为 `HttpStatus.SERVICE_DATA_ERROR`
//! = 1001，故本文件所有业务错误都用 [`SERVICE_DATA_ERROR`]。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::models::marketing::buyer_show_group::{
    BuyerShowGroupPageQuery, BuyerShowGroupPageVO, BuyerShowGroupSaveParam, BuyerShowGroupScope,
    BuyerShowGroupUpdateParam, BuyerShowGroupVO,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::buyer_show_group::BuyerShowGroupRepository;
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// Java `HttpStatus.SERVICE_DATA_ERROR`（`GlobalException` 默认码，也是参数校验异常码）。
pub const SERVICE_DATA_ERROR: i32 = 1001;

/// Java `BuyerShowGroupSaveParam.groupName` / `BuyerShowGroupUpdateParam.groupName`
/// 的 `@NotBlank(message = ...)` 文案。
const ERR_GROUP_NAME_BLANK: &str = "分组名称不能为空";
/// Java `groupIds` 的 `@NotEmpty(message = ...)` 文案。
const ERR_GROUP_IDS_EMPTY: &str = "商品分组Ids不能为空";
/// Java `BuyerShowGroupUpdateParam.buyerShowGroupId` 的 `@NotNull(message = ...)` 文案。
const ERR_GROUP_ID_NULL: &str = "买家秀分组ID不能为空";
/// Java `OperateDTO.id` 的 `@NotNull(message = ResultTextConstants.ID_CANNOT_IS_NULL)` 文案。
const ERR_ID_NULL: &str = "id不能为空";
/// Java `Assert.notNull(..., () -> new GlobalException("数据不存在"))`。
const ERR_NOT_EXIST: &str = "数据不存在";
/// Java `Assert.isTrue(count == 0, () -> new GlobalException("分组名称已存在"))`。
const ERR_NAME_EXISTS: &str = "分组名称已存在";

/// 「属性名 → 数据库列名」白名单（Java `BaseServiceImpl#getColumnByPropertyName`
/// 同时接受属性名与列名，找不到时回退主键列）。
const SORTABLE: [(&str, &'static str); 8] = [
    ("buyerShowGroupId", "buyer_show_group_id"),
    ("buyer_show_group_id", "buyer_show_group_id"),
    ("groupName", "group_name"),
    ("group_name", "group_name"),
    ("productGroupId", "product_group_id"),
    ("product_group_id", "product_group_id"),
    ("shopId", "shop_id"),
    ("shop_id", "shop_id"),
];

/// 主键列（Java `getKeyColumn()`）。
const KEY_COLUMN: &str = "buyer_show_group_id";

/// 买家秀分组域服务。
pub struct BuyerShowGroupService {
    repos: Arc<Repositories>,
}

impl BuyerShowGroupService {
    /// 构造服务。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    fn repo(&self) -> &BuyerShowGroupRepository {
        &self.repos.marketing.buyer_show_group
    }

    // ————————————————————— 新建 —————————————————————

    /// `POST /adminapi/promotion/buyerShow/group/create`：新建买家秀分组。
    ///
    /// 对应 Java `BuyerShowGroupServiceImpl#create(BuyerShowGroupSaveParam)`：
    /// 1. `@Valid` 校验（`groupName` @NotBlank、`groupIds` @NotEmpty）；
    /// 2. `verifyGroupName(groupName, null)` —— 同名即「分组名称已存在」；
    /// 3. `setShopId(getShopId())`（`null` 时 MyBatis-Plus 跳过该列 → 列默认值 0）
    ///    与 `setProductGroupId(JSONUtil.toJsonStr(groupIds))` 后 `save`。
    ///
    /// 原接口返回 `void`；本方法返回新建主键，供路由层回传便于排查。
    ///
    /// # Errors
    /// - 参数校验失败 → `Error::biz(1001, ...)`（对应 Java 的 1001）；
    /// - 分组名重复 → `Error::biz(1001, "分组名称已存在")`；
    /// - 数据库失败 → `Error::Database`。
    pub async fn create(
        &self,
        param: &BuyerShowGroupSaveParam,
        scope: BuyerShowGroupScope,
    ) -> Result<i32> {
        validate_save_params(param.group_name.as_deref(), param.group_ids.as_deref())?;
        let group_name = param.group_name.clone().unwrap_or_default();
        let group_ids = param.group_ids.clone().unwrap_or_default();

        self.verify_group_name(&group_name, None).await?;

        let product_group_id = encode_group_ids(&group_ids);
        self.repo()
            .insert_group(&group_name, &product_group_id, scope.insert_shop_id())
            .await
    }

    // ————————————————————— 列表 —————————————————————

    /// `GET /adminapi/promotion/buyerShow/group/list`：分组分页。
    ///
    /// 对应 Java `BuyerShowGroupServiceImpl#list(BuyerShowGroupPageQuery)`：
    /// - `buildSortOrder(page, sortField, sortOrder)`（见 [`resolve_sort`]）；
    /// - `queryWrapper.eq(shopId, getShopId())` —— `getShopId()` 为 `null` 时
    ///   MyBatis-Plus 仍会生成 `shop_id = NULL`（恒不成立），故
    ///   [`BuyerShowGroupScope::Unscoped`] 直接返回**空页**（不查库）；
    /// - 记录为空时 Java `return new Page<>()`（`total = 0`），此处用
    ///   [`PageResult::empty`] 复刻；
    /// - 每条记录的 `productGroupName` 与 `buyerShowCount` 逐条派生。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn list(
        &self,
        query: &BuyerShowGroupPageQuery,
        scope: BuyerShowGroupScope,
    ) -> Result<PageResult<BuyerShowGroupPageVO>> {
        let page = query.page_query();

        // Java `eq(shop_id, null)` → `shop_id = NULL` → 查不到任何行
        let Some(shop_id) = scope.shop_id() else {
            return Ok(PageResult::empty(&page));
        };

        let (sort_column, ascending) =
            resolve_sort(query.sort_field.as_deref(), query.sort_order.as_deref());
        let (total, rows) = self
            .repo()
            .paginate_groups(shop_id, sort_column, ascending, &page)
            .await?;
        if rows.is_empty() {
            // Java：`if (CollUtil.isEmpty(buyerShowGroupList)) return new Page<>();`
            return Ok(PageResult::empty(&page));
        }

        // 一次性取回本页所有关联商品分组的名称（Java 也是 `in(...)` 批量查询）
        let mut all_ids: Vec<i32> = Vec::new();
        for row in &rows {
            all_ids.extend(decode_group_ids(&row.product_group_id));
        }
        let names = self.load_product_group_names(&all_ids).await?;

        let mut list = Vec::with_capacity(rows.len());
        for row in &rows {
            let group_ids = decode_group_ids(&row.product_group_id);
            // Java `buyerShowMapper.selectCount(... JSON_CONTAINS(group_data, {0}))`
            let buyer_show_count = self
                .repo()
                .count_buyer_shows_in_group(row.buyer_show_group_id)
                .await?;
            list.push(BuyerShowGroupPageVO {
                buyer_show_group_id: row.buyer_show_group_id,
                group_name: row.group_name.clone(),
                product_group_name: names_for(&group_ids, &names),
                buyer_show_count,
            });
        }
        Ok(PageResult::new(total, list, &page))
    }

    // ————————————————————— 详情 —————————————————————

    /// `GET /adminapi/promotion/buyerShow/group/detail?buyerShowGroupId=`：分组详情。
    ///
    /// 对应 Java `BuyerShowGroupServiceImpl#detail(Integer buyerShowGroupId)`：
    /// `getById` 为 `null` → 「数据不存在」（code 1001）；
    /// `groupIds = JSONUtil.toList(productGroupId, Integer.class)`；
    /// 再按 `groupIds` 查商品分组名。
    ///
    /// **注意**：Java 此处**没有** `shop_id` 归属校验，本实现保持一致。
    ///
    /// # Errors
    /// - 记录不存在 → `Error::biz(1001, "数据不存在")`；
    /// - 数据库失败 → `Error::Database`。
    pub async fn detail(&self, buyer_show_group_id: i32) -> Result<BuyerShowGroupVO> {
        let row = self
            .repo()
            .find_by_id(buyer_show_group_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, ERR_NOT_EXIST))?;
        // 多租户：店铺账号只能查看本店分组
        TenantScope::current_ensure_access(row.shop_id)?;

        let group_ids = decode_group_ids(&row.product_group_id);
        let names = self.load_product_group_names(&group_ids).await?;

        Ok(BuyerShowGroupVO {
            buyer_show_group_id: row.buyer_show_group_id,
            group_name: row.group_name,
            product_group_name: names_for(&group_ids, &names),
            group_ids,
        })
    }

    // ————————————————————— 更新 —————————————————————

    /// `POST /adminapi/promotion/buyerShow/group/update`：更新买家秀分组。
    ///
    /// 对应 Java `BuyerShowGroupServiceImpl#update(BuyerShowGroupUpdateParam)`：
    /// 1. `@Valid` 校验（`buyerShowGroupId` @NotNull、`groupName` @NotBlank、
    ///    `groupIds` @NotEmpty）；
    /// 2. `getById` 为 `null` → 「数据不存在」；
    /// 3. `verifyGroupName(groupName, buyerShowGroupId)`（排除自身）；
    /// 4. 更新 `groupName` / `productGroupId` 后 `updateById`。
    ///
    /// **注意**：与 Java 一致，更新**不校验** `shop_id` 归属，也**不修改**
    /// `shop_id`（`update_group` 只写两列）。
    ///
    /// # Errors
    /// - 参数校验失败 / 记录不存在 / 名称重复 → `Error::biz(1001, ...)`；
    /// - 数据库失败 → `Error::Database`。
    pub async fn update(&self, param: &BuyerShowGroupUpdateParam) -> Result<()> {
        if param.buyer_show_group_id.is_none() {
            // Java 的三个字段校验错误会被合并输出，故这里也走合并路径
            return Err(Error::biz(
                SERVICE_DATA_ERROR,
                collect_param_errors(
                    param.group_name.as_deref(),
                    param.group_ids.as_deref(),
                    true,
                ),
            ));
        }
        validate_save_params(param.group_name.as_deref(), param.group_ids.as_deref())?;

        let buyer_show_group_id = param.buyer_show_group_id.unwrap_or_default();
        let group_name = param.group_name.clone().unwrap_or_default();
        let group_ids = param.group_ids.clone().unwrap_or_default();

        let existing = self
            .repo()
            .find_by_id(buyer_show_group_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, ERR_NOT_EXIST))?;
        // 多租户：更新前校验归属
        TenantScope::current_ensure_access(existing.shop_id)?;

        self.verify_group_name(&group_name, Some(buyer_show_group_id))
            .await?;

        let product_group_id = encode_group_ids(&group_ids);
        // Java `updateById` 影响 0 行时静默返回 false，此处同样不报错
        self.repo()
            .update_group(
                existing.buyer_show_group_id,
                &group_name,
                &product_group_id,
            )
            .await?;
        Ok(())
    }

    // ————————————————————— 删除 —————————————————————

    /// `POST /adminapi/promotion/buyerShow/group/del`：删除买家秀分组。
    ///
    /// 对应 Java `BaseServiceImpl#del(Integer id)` → `removeById(id)`：
    /// 直接物理删除 `buyer_show_group` 行，**不**级联清理
    /// `buyer_show.group_data` 中的引用，影响 0 行也不报错。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn delete(&self, id: i32) -> Result<()> {
        // 多租户：记录存在时校验归属（Java 对不存在 id 静默成功）
        if let Some(row) = self.repo().find_by_id(id).await? {
            TenantScope::current_ensure_access(row.shop_id)?;
        }
        self.repo().delete_group(id).await?;
        Ok(())
    }

    // ————————————————————— 内部工具 —————————————————————

    /// 同名分组校验（Java `BuyerShowGroupServiceImpl#verifyGroupName`）。
    async fn verify_group_name(&self, group_name: &str, exclude_id: Option<i32>) -> Result<()> {
        let count = self
            .repo()
            .count_group_by_name(group_name, exclude_id)
            .await?;
        if count != 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, ERR_NAME_EXISTS));
        }
        Ok(())
    }

    /// 批量取商品分组名（`id → name`，跳过数据库中名称为 `NULL` 的行）。
    async fn load_product_group_names(&self, ids: &[i32]) -> Result<HashMap<i32, String>> {
        let rows = self.repo().find_product_group_names(ids).await?;
        Ok(rows
            .into_iter()
            .filter_map(|(id, name)| name.map(|n| (id, n)))
            .collect())
    }
}

// ————————————————————— 纯逻辑（可单测） —————————————————————

/// Java `BaseServiceImpl#buildSortOrder(Page, sortField, sortOrder)` 的等价实现。
///
/// 规则（逐条对照 Java）：
/// 1. `sortField` 为空，或等于主键属性名 `buyerShowGroupId` → 主键列；
/// 2. 否则按「属性名或列名」查表，**查不到时回退主键列**（Java 的防注入兜底）；
/// 3. `sortOrder` 忽略大小写等于 `asc` → 升序；等于 `desc` 或为空 → 降序；
///    **其它任意值 → 完全不设置 `ORDER BY`**（返回 `None`）。
#[must_use]
pub fn resolve_sort(
    sort_field: Option<&str>,
    sort_order: Option<&str>,
) -> (&'static str, Option<bool>) {
    let column = match sort_field {
        None => KEY_COLUMN,
        Some(raw) if raw.is_empty() => KEY_COLUMN,
        Some(raw) => SORTABLE
            .iter()
            .find(|(property, _)| *property == raw)
            .map_or(KEY_COLUMN, |(_, column)| *column),
    };

    let ascending = match sort_order {
        Some(order) if order.eq_ignore_ascii_case("asc") => Some(true),
        Some(order) if order.eq_ignore_ascii_case("desc") || order.is_empty() => Some(false),
        None => Some(false),
        // Java：既非 asc 也非 desc/空 → 不设置排序
        Some(_) => None,
    };

    (column, ascending)
}

/// `Vec<i32>` → JSON 数组文本（Java `JSONUtil.toJsonStr(groupIds)`）。
///
/// Java 产出紧凑格式（`[1,2]`），`serde_json` 一致（空数组为 `[]`）。
#[must_use]
pub fn encode_group_ids(group_ids: &[i32]) -> String {
    serde_json::to_string(group_ids).unwrap_or_else(|_| "[]".to_string())
}

/// JSON 数组文本 → `Vec<i32>`（Java `JSONUtil.toList(text, Integer.class)`）。
///
/// 容错：空白 / 非法 JSON / 类型不符一律返回空表（Java 在非法 JSON 时抛异常，
/// 差异已记入 `uncertain_points`）。
#[must_use]
pub fn decode_group_ids(raw: &str) -> Vec<i32> {
    serde_json::from_str::<Vec<i32>>(raw).unwrap_or_default()
}

/// 按 Java 的映射顺序输出商品分组名。
///
/// Java 是「遍历 `productGroup` 查询结果（主键升序）→ 若 `groupIds` 包含则取名」，
/// 因此输出顺序是**商品分组主键升序**，且重复 ID 只输出一次。
#[must_use]
pub fn names_for(group_ids: &[i32], names: &HashMap<i32, String>) -> Vec<String> {
    let mut ids: Vec<i32> = group_ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    ids.into_iter()
        .filter_map(|id| names.get(&id).cloned())
        .collect()
}

/// 合并 `groupName` / `groupIds` 两个字段的校验文案（Java `@Valid` 会一次报全部错误）。
fn collect_param_errors(
    group_name: Option<&str>,
    group_ids: Option<&[i32]>,
    include_id_error: bool,
) -> String {
    let mut errors: Vec<&'static str> = Vec::new();
    if include_id_error {
        errors.push(ERR_GROUP_ID_NULL);
    }
    if group_name.is_none_or(|name| name.trim().is_empty()) {
        errors.push(ERR_GROUP_NAME_BLANK);
    }
    if group_ids.is_none_or(|ids: &[i32]| ids.is_empty()) {
        errors.push(ERR_GROUP_IDS_EMPTY);
    }
    errors.join(", ")
}

/// `create` / `update` 共用的 `groupName` + `groupIds` 校验。
///
/// 注意：Java `@NotBlank` 只做校验、**不修改原值**，故带首尾空格的分组名
/// 会原样落库，这里同样返回原字符串。
fn validate_save_params(group_name: Option<&str>, group_ids: Option<&[i32]>) -> Result<()> {
    let message = collect_param_errors(group_name, group_ids, false);
    if message.is_empty() {
        Ok(())
    } else {
        Err(Error::biz(SERVICE_DATA_ERROR, message))
    }
}

/// 校验 `del` 入参的 `id`（Java `@NotNull(message = "id不能为空")`）。
///
/// # Errors
/// `id` 缺失 → `Error::biz(1001, "id不能为空")`。
pub fn require_operate_id(id: Option<i32>) -> Result<i32> {
    id.ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, ERR_ID_NULL))
}

/// 校验 `detail` 的 `buyerShowGroupId`。
///
/// Java 侧该参数是**必填** `@RequestParam`：缺失时 Spring 抛
/// `MissingServletRequestParameterException`，由 `GlobalExceptionHandler`
/// 统一返回 `SERVICE_DATA_ERROR(1001)` + `error.param.interface.missing`
/// （中文默认文案「接口参数缺失」）。
///
/// # Errors
/// 参数缺失 → `Error::biz(1001, "接口参数缺失")`。
pub fn require_group_id(id: Option<i32>) -> Result<i32> {
    id.ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "接口参数缺失"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(pairs: &[(i32, &str)]) -> HashMap<i32, String> {
        pairs
            .iter()
            .map(|(id, name)| (*id, (*name).to_string()))
            .collect()
    }

    // ————————————————— 排序 —————————————————

    #[test]
    fn sort_defaults_to_key_column_desc() {
        // Java `BasePage` 未传 sortField / sortOrder → 主键降序
        assert_eq!(resolve_sort(None, None), (KEY_COLUMN, Some(false)));
        assert_eq!(resolve_sort(Some(""), Some("")), (KEY_COLUMN, Some(false)));
        // 主键属性名 → 主键列
        assert_eq!(
            resolve_sort(Some("buyerShowGroupId"), Some("asc")),
            (KEY_COLUMN, Some(true))
        );
    }

    #[test]
    fn sort_maps_property_and_column_names() {
        assert_eq!(
            resolve_sort(Some("groupName"), Some("asc")),
            ("group_name", Some(true))
        );
        assert_eq!(
            resolve_sort(Some("group_name"), Some("ASC")),
            ("group_name", Some(true))
        );
        assert_eq!(
            resolve_sort(Some("productGroupId"), Some("desc")),
            ("product_group_id", Some(false))
        );
        assert_eq!(resolve_sort(Some("shop_id"), Some("desc")), ("shop_id", Some(false)));
    }

    #[test]
    fn sort_unknown_field_falls_back_and_unknown_order_skips_order_by() {
        // 未知属性 → 主键列（Java 的防注入兜底）
        assert_eq!(
            resolve_sort(Some("group_name; DROP TABLE x"), Some("asc")),
            (KEY_COLUMN, Some(true))
        );
        // 大小写敏感：Java 用 equals 精确匹配属性名
        assert_eq!(
            resolve_sort(Some("GROUPNAME"), Some("asc")),
            (KEY_COLUMN, Some(true))
        );
        // 既非 asc 也非 desc/空 → Java 不设置 ORDER BY
        assert_eq!(resolve_sort(Some("groupName"), Some("xyz")), ("group_name", None));
        assert_eq!(resolve_sort(None, Some("ascending")), (KEY_COLUMN, None));
    }

    // ————————————————— JSON 文本 —————————————————

    #[test]
    fn group_ids_json_round_trip() {
        assert_eq!(encode_group_ids(&[1, 2, 3]), "[1,2,3]");
        assert_eq!(encode_group_ids(&[]), "[]");
        assert_eq!(decode_group_ids("[1,2,3]"), vec![1, 2, 3]);
        // 容错：空白 / 非法 JSON / 类型不符 → 空表
        assert!(decode_group_ids("").is_empty());
        assert!(decode_group_ids("   ").is_empty());
        assert!(decode_group_ids("not json").is_empty());
        assert!(decode_group_ids(r#"["a"]"#).is_empty());
        assert!(decode_group_ids("null").is_empty());
    }

    #[test]
    fn names_follow_product_group_pk_order_and_dedup() {
        let map = names(&[(3, "C"), (5, "E"), (9, "I")]);
        // 乱序 + 重复 → 主键升序 + 去重
        assert_eq!(names_for(&[5, 3, 3, 9], &map), vec!["C", "E", "I"]);
        // 查不到的 ID 直接跳过（Java 中该 ID 不会出现在 product_group 结果里）
        assert_eq!(names_for(&[3, 404], &map), vec!["C"]);
        assert!(names_for(&[], &map).is_empty());
    }

    // ————————————————— 参数校验 —————————————————

    #[test]
    fn create_param_validation_matches_java_messages() {
        let empty: Vec<i32> = Vec::new();
        let ids: Vec<i32> = vec![1];
        // 合法
        assert!(validate_save_params(Some("夏季穿搭"), Some(ids.as_slice())).is_ok());
        // 名称缺失 / 空白
        let err = validate_save_params(None, Some(ids.as_slice())).unwrap_err();
        assert_eq!(err.business_code(), SERVICE_DATA_ERROR);
        assert!(err.to_string().contains(ERR_GROUP_NAME_BLANK));
        assert!(validate_save_params(Some("   "), Some(ids.as_slice())).is_err());
        // 商品分组为空
        assert!(validate_save_params(Some("x"), None).is_err());
        assert!(validate_save_params(Some("x"), Some(empty.as_slice())).is_err());
        // 两个都错 → Java 用 ", " 连接全部错误
        let err = validate_save_params(None, Some(empty.as_slice())).unwrap_err();
        let text = err.to_string();
        assert!(text.contains(ERR_GROUP_NAME_BLANK));
        assert!(text.contains(ERR_GROUP_IDS_EMPTY));
        assert!(text.contains(&format!("{ERR_GROUP_NAME_BLANK}, {ERR_GROUP_IDS_EMPTY}")));
    }

    #[test]
    fn not_blank_does_not_trim_stored_value() {
        let ids: Vec<i32> = vec![1];
        // Java `@NotBlank` 仅校验，不修改值：带空格的名称为合法输入且原样落库
        assert!(validate_save_params(Some(" 夏季 "), Some(ids.as_slice())).is_ok());
    }

    #[test]
    fn update_param_validation_reports_all_three_errors() {
        let text = collect_param_errors(None, None, true);
        assert_eq!(
            text,
            format!("{ERR_GROUP_ID_NULL}, {ERR_GROUP_NAME_BLANK}, {ERR_GROUP_IDS_EMPTY}")
        );
    }

    #[test]
    fn operate_and_detail_ids_are_required() {
        assert_eq!(require_operate_id(Some(7)).unwrap(), 7);
        let err = require_operate_id(None).unwrap_err();
        assert_eq!(err.business_code(), SERVICE_DATA_ERROR);
        assert!(err.to_string().contains(ERR_ID_NULL));

        assert_eq!(require_group_id(Some(8)).unwrap(), 8);
        let err = require_group_id(None).unwrap_err();
        assert_eq!(err.business_code(), SERVICE_DATA_ERROR);
        assert!(err.to_string().contains("接口参数缺失"));
    }

    // ————————————————— 错误文案 / 错误码 —————————————————

    #[test]
    fn business_errors_use_service_data_error_code() {
        let not_exist = Error::biz(SERVICE_DATA_ERROR, ERR_NOT_EXIST);
        assert_eq!(not_exist.business_code(), 1001);
        assert!(not_exist.to_string().contains(ERR_NOT_EXIST));

        let duplicated = Error::biz(SERVICE_DATA_ERROR, ERR_NAME_EXISTS);
        assert_eq!(duplicated.business_code(), 1001);
        assert!(duplicated.to_string().contains(ERR_NAME_EXISTS));
    }
}