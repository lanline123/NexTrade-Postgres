//! 首页分类栏（H5）域服务（content::mobile_cat_nav）。
//!
//! 对应原 Java `com.nextrade.service.decorate.impl.MobileCatNavServiceImpl`
//! （207 行）+ `MobileCatNavController`（96 行）中的 7 条管理端链路：
//!
//! | 本域端点（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |--------------------------------|---------|------|
//! | `GET  /decorate/mobileCatNav/list` | `MobileCatNavController.list` | [`MobileCatNavService::list`] |
//! | `GET  /decorate/mobileCatNav/detail` | `MobileCatNavController.detail` | [`MobileCatNavService::detail`] |
//! | `POST /decorate/mobileCatNav/create` | `MobileCatNavController.create` | [`MobileCatNavService::create`] |
//! | `POST /decorate/mobileCatNav/update` | `MobileCatNavController.update` | [`MobileCatNavService::update`] |
//! | `POST /decorate/mobileCatNav/del` | `MobileCatNavController.del` | [`MobileCatNavService::del`] |
//! | `POST /decorate/mobileCatNav/updateField` | `MobileCatNavController.updateField` | [`MobileCatNavService::update_field`] |
//! | `POST /decorate/mobileCatNav/batch` | `MobileCatNavController.batch` | [`MobileCatNavService::batch`] |
//!
//! ## 迁移要点（逐条对照 Java 源）
//!
//! - `list`：`buildSortOrder(page, sortField, sortOrder)` —— `sortField` 为空或
//!   等于主键属性名 `mobileCatNavId` 时用主键列；未识别的字段名**静默回退
//!   主键列**（MyBatis-Plus 的安全修复，不是报错）；`sortOrder` 为 `asc` 升序、
//!   为空或 `desc` 降序，**其它非法值完全不追加 ORDER BY**（`else if` 的缺口）。
//!   `isShow` 只有 0 / 1 参与过滤；`keyword` 走 `category_id LIKE %keyword%`
//!   （对整型列做 LIKE，靠 MySQL 隐式转换）。查询后用
//!   `getCategoryByIds` 批量补 `categoryName`。
//! - `detail`：`getById(id)` 后 `BeanUtils.copyProperties`，再把
//!   `imgUrl` / `brandIds` / `childCatIds` 三列 `JsonUtil.fromJson` 成数组。
//!   ⚠️ **不补** `categoryName`（Java `detail` 没查 `category` 表）。
//! - `create` / `update`：三列 JSON 的 null 语义**不一致**（见
//!   [`nextrade_core::models::content::mobile_cat_nav::create_json_columns`]）：
//!   `img_url` / `child_cat_ids` 的 null ⇒ `"[]"`，而 `brand_ids` 的 null
//!   ⇒ `toJson(null)` = `null` ⇒ **不写该列**。
//! - `del`：`removeById(id)` 硬删除（表无软删列），不校验记录是否存在。
//! - `updateField`：白名单是控制器里**内联的数组字面量**
//!   `{"is_show", "sort_order"}`（不是 `CheckFieldConstants` 常量）；
//!   字段名先经 `getColumnByPropertyName` 归一（属性名或列名都可命中）。
//! - `batch`：仅 `type == "del"` 生效，其余类型**静默返回 `false`**
//!   （Java `BaseServiceImpl#batch` 的行为，不是报错）。
//!
//! ## ⚠️ 首页装修缓存失效：**本实现未接入**
//!
//! Java 的五条写链路（`create` / `update` / `del` / `batch` / `updateField`）
//! 都在事务提交后执行：
//! ```text
//! TransactionAfterCommitExecutor.execute(() -> homeDecorateCache.invalidateAndBroadcast());
//! ```
//! 即「清空本进程的首页装修快照 + 经 Redis Pub/Sub
//! （channel `home:decorate:invalidate`）通知 `nextrade-api` 等其它 JVM 一起失效」，
//! 保证后台改完分类栏后 C 端**立刻**生效（否则最多陈旧 5 分钟 TTL）。
//!
//! Rust 侧**尚无** `HomeDecorateCache` 的等价物：`grep -r home_decorate crates`
//! 显示全仓只有 shop_client 的 `find_shop_home_decorate`（一次 DB 查询），
//! 没有任何进程内首页装修快照缓存，也没有对应的 Pub/Sub 通道。因此本实现
//! **没有可失效的目标**，写链路只改库、不广播。
//!
//! ⚠️ 这不是「漏实现」，而是**前置依赖尚未迁移**。一旦主会话/其它切片引入
//! 首页装修缓存，本服务的五个写方法都必须补上提交后失效调用
//! （已列入 `uncertain_points`）。当前不引入的后果：如果将来有人加了缓存
//! 却忘了接这里，后台改分类栏后 C 端最长 5 分钟才生效。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::models::content::mobile_cat_nav::{
    child_cat_ids_for_create, create_json_columns, map_update_field_column, img_url_for_create,
    to_json_array, BatchDTO, MobileCatNavCreateDTO, MobileCatNavListQuery, MobileCatNavUpdateDTO,
    MobileCatNavVO, OperateDTO, UpdateFieldDTO, MOBILE_CAT_NAV_COLUMNS, MOBILE_CAT_NAV_FIELDS,
    MSG_ID_REQUIRED, MSG_INVALID_FIELD_VALUE, MSG_MOBILE_CAT_NAV_ID_NOT_NULL,
    MSG_MOBILE_CAT_NAV_NOT_FOUND,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::content::mobile_cat_nav::{MobileCatNavInsert, MobileCatNavPatch};
use nextrade_infrastructure::db::Repositories;

/// 首页分类栏域服务（挂载于 `AdminState.mobile_cat_nav`）。
pub struct MobileCatNavService {
    repos: Arc<Repositories>,
}

impl MobileCatNavService {
    /// 构造（与 order_admin / admin_user / pc_cat_floor / area_code 的
    /// `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 列表 / 详情 —————————————————————

    /// 首页分类栏分页（对应 Java `MobileCatNavServiceImpl#list`）。
    ///
    /// 返回 `PageResult<MobileCatNavVO>`（Java 返回 MyBatis-Plus
    /// `Page<MobileCatNavVO>`，Rust 统一使用项目既有 `PageResult`，
    /// 与 product_manage/002/008/011/017/075 同一约定）。
    ///
    /// Java 在记录为空时**提前返回** `PageUtil.convertPage(page, List.of())`
    /// （不做 `getCategoryByIds` 调用）；本实现同样短路，避免一次空 `IN` 查询。
    pub async fn list(&self, q: &MobileCatNavListQuery) -> Result<PageResult<MobileCatNavVO>> {
        let page = q.page_query();
        let (total, rows) = self.repos.content.mobile_cat_nav.paginate(q).await?;
        if rows.is_empty() {
            // Java `if (CollUtil.isEmpty(records)) return convertPage(page, List.of())`
            return Ok(PageResult::new(total, Vec::new(), &page));
        }

        // 收集去重后的 categoryId（Java `.map(getCategoryId).distinct()`）
        let mut category_ids: Vec<i32> = rows.iter().map(|r| r.category_id).collect();
        category_ids.sort_unstable();
        category_ids.dedup();

        // `categoryService.getCategoryByIds(categoryIds)` → `Map<categoryId, Category>`
        let categories = self
            .repos.content.mobile_cat_nav
            .find_categories_by_ids(&category_ids)
            .await?;
        let name_map: HashMap<i32, String> = categories
            .into_iter()
            .map(|c| (c.category_id, c.category_name))
            .collect();

        // Java 是逐个 `BeanUtils.copyProperties` + `if (category != null) setCategoryName`
        let list: Vec<MobileCatNavVO> = rows
            .iter()
            .map(|r| MobileCatNavVO::for_list(r, name_map.get(&r.category_id).cloned()))
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 首页分类栏详情（对应 Java `MobileCatNavServiceImpl#detail`）。
    ///
    /// Java：
    /// ```text
    /// if (id != null) { getById(id); copyProperties; 三次 JsonUtil.fromJson; return vo; }
    /// return null;
    /// ```
    /// ⚠️ **参数为 null 时 Java 返回 `null` 体**；本实现由路由层要求 `id` 必填
    /// （与 Java `@RequestParam Integer id` 的「缺失即 400」一致）。
    /// 记录不存在时 Java 会在 `mobileCatNav.getImgUrl()` 上抛 **NPE**；
    /// 本实现返回可诊断的 404（项目内 order_admin/005/017/075 的既有约定）。
    pub async fn detail(&self, id: i32) -> Result<MobileCatNavVO> {
        let m = self
            .repos.content.mobile_cat_nav
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_MOBILE_CAT_NAV_NOT_FOUND.into()))?;
        Ok(MobileCatNavVO::for_detail(&m))
    }

    // ————————————————————— 创建 —————————————————————

    /// 创建首页分类栏（对应 Java `MobileCatNavServiceImpl#create`）。
    ///
    /// Java：
    /// ```text
    /// if (createDTO != null) {
    ///     copyProperties(createDTO, mobileCatNav);
    ///     setImgUrl(toJson(imgUrl == null ? new GalleryPicDTO[0] : imgUrl));
    ///     setBrandIds(toJson(brandIds));                       // null ⇒ MyBatis 跳过该列
    ///     setChildCatIds(toJson(childCatIds == null ? new Integer[0] : childCatIds));
    ///     boolean result = this.save(mobileCatNav);
    ///     invalidateHomeDecorateAfterCommit();
    ///     return result;
    /// }
    /// return false;
    /// ```
    ///
    /// ⚠️ `MobileCatNavCreateDTO.categoryId` 上的 `@NotBlank` **不生效**：
    /// `MobileCatNavController` 类上只有 `@RestController` / `@RequestMapping`
    /// / `@Tag`，**没有** `@Validated`，方法参数上也没有 `@Valid`。
    /// 因此 Java 允许 `categoryId` 为 null（再由库的 `NOT NULL` 约束报错）。
    /// 本实现保持这一「不校验」行为（不编造校验规则），返回新建主键。
    ///
    /// 控制器返回 `void`，这里回传新建主键便于前端跳转编辑
    /// （与 shop_manage / pc_cat_floor / area_code 的处理方式一致）。
    pub async fn create(&self, dto: &MobileCatNavCreateDTO) -> Result<i32> {
        // 三列 JSON：img / child 的 null ⇒ "[]"，brand 的 null ⇒ 不写列
        let (img_url, child_cat_ids, brand_ids) = create_json_columns(
            dto.img_url.as_deref(),
            dto.child_cat_ids.as_deref(),
            dto.brand_ids.as_deref(),
        );

        let id = self
            .repos.content.mobile_cat_nav
            .insert(&MobileCatNavInsert {
                category_id: dto.category_id,
                cat_color: dto.cat_color.clone(),
                child_cat_ids: Some(child_cat_ids),
                brand_ids,
                is_show: dto.is_show,
                img_url: Some(img_url),
                sort_order: dto.sort_order,
                cat_name_alias: dto.cat_name_alias.clone(),
            })
            .await?;

        // ⚠️ Java 在此处 `invalidateHomeDecorateAfterCommit()`；
        // Rust 侧无首页装修缓存可失效，见模块头说明。
        tracing::info!("创建首页分类栏成功: id={id}");
        Ok(id)
    }

    // ————————————————————— 更新 —————————————————————

    /// 更新首页分类栏（对应 Java `MobileCatNavServiceImpl#update`）。
    ///
    /// Java：
    /// ```text
    /// if (updateDTO != null) {
    ///     copyProperties(updateDTO, mobileCatNav);
    ///     setImgUrl(toJson(updateDTO.getImgUrl()));           // 无条件，null ⇒ 不写列
    ///     setBrandIds(toJson(updateDTO.getBrandIds()));
    ///     setChildCatIds(toJson(updateDTO.getChildCatIds()));
    ///     boolean result = this.updateById(mobileCatNav);
    ///     invalidateHomeDecorateAfterCommit();
    ///     return result;
    /// }
    /// return false;
    /// ```
    ///
    /// ⚠️ 与 `create` 的关键差异：`update` 的三列 JSON 是**无条件** `toJson(...)`，
    /// 因此 `null` 一律变成 `null` ⇒ 三列**都不写**（保留库中旧值）；
    /// 而 `create` 会把 `imgUrl` / `childCatIds` 的 null 转成 `"[]"`。
    ///
    /// `mobileCatNavId` 缺失时 Java 会走 `updateById` 且主键为 null
    /// （`@NotNull` 因缺少 `@Validated` 不生效），MyBatis-Plus 抛异常。
    /// 本实现显式校验并复用 Java 那条 `@NotNull` 的**原文案**
    /// （见 `uncertain_points`）。
    pub async fn update(&self, dto: &MobileCatNavUpdateDTO) -> Result<bool> {
        let id = dto
            .mobile_cat_nav_id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation(MSG_MOBILE_CAT_NAV_ID_NOT_NULL.into()))?;

        // 记录必须存在：Java `updateById` 命中 0 行静默返回 false，
        // 这里先查一次以便区分「不存在」与「值未变化」（与 area_code 同一处理）。
        if self.repos.content.mobile_cat_nav.find_by_id(id).await?.is_none() {
            return Err(Error::NotFound(MSG_MOBILE_CAT_NAV_NOT_FOUND.into()));
        }

        // 三列 JSON：null ⇒ None ⇒ 不写列（Java `toJson(null)` = null）
        let patch = MobileCatNavPatch {
            category_id: dto.category_id.map(Some),
            cat_color: dto.cat_color.clone().map(Some),
            child_cat_ids: dto.child_cat_ids.as_deref().map(to_json_array).map(Some),
            brand_ids: dto.brand_ids.as_deref().map(to_json_array).map(Some),
            is_show: dto.is_show.map(Some),
            img_url: dto.img_url.as_deref().map(to_json_array).map(Some),
            sort_order: dto.sort_order.map(Some),
            cat_name_alias: dto.cat_name_alias.clone().map(Some),
        };

        if patch.is_empty() {
            // Java `updateById(全 null 实体)` 会生成非法 SQL 并抛异常；
            // 这里回 false（无字段可更新，等价于「未命中」）。
            return Ok(false);
        }

        let affected = self.repos.content.mobile_cat_nav.update(id, &patch).await?;
        // ⚠️ Java 在此处 `invalidateHomeDecorateAfterCommit()`；Rust 侧无缓存可失效。
        tracing::info!("更新首页分类栏: id={id}, 影响 {affected} 行");
        Ok(affected > 0)
    }

    // ————————————————————— 删除 —————————————————————

    /// 删除首页分类栏（对应 Java `MobileCatNavServiceImpl#del`
    /// → `BaseServiceImpl#del` → `removeById`，硬删除）。
    ///
    /// ⚠️ Java 的 `del` 是「`id == null` 返回 false，否则 `removeById(id)`」，
    /// **不校验记录是否存在**（删不存在的 id 也返回成功）。
    /// 本实现保持这一语义（返回删除是否命中），不额外抛 404。
    pub async fn del(&self, dto: &OperateDTO) -> Result<bool> {
        let id = dto
            .id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let removed = self.repos.content.mobile_cat_nav.delete(id).await?;
        // ⚠️ Java 仅在 `removed == true` 时 `invalidateHomeDecorateAfterCommit()`。
        Ok(removed)
    }

    // ————————————————————— 单字段更新 —————————————————————

    /// 单字段更新（对应 Java `MobileCatNavController.updateField` →
    /// `MobileCatNavServiceImpl#updateField(dto, new String[]{"is_show", "sort_order"})`）。
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. `field` 经 `getColumnByPropertyName` 归一为列名（属性名/列名都可命中，
    ///    **未识别时回退主键列** —— 本实现用 `None` 表达，对白名单的判定结果一致）；
    /// 2. 归一后的列名必须落在 `MOBILE_CAT_NAV_FIELDS` = `{is_show, sort_order}`
    ///    内，否则 `字段值不合法`；
    /// 3. `UPDATE mobile_cat_nav SET <列> = ? WHERE mobile_cat_nav_id = ?`，
    ///    值以字符串绑定，由 MySQL 按目标列类型隐式转换。
    ///
    /// 返回是否命中记录（Java 直接返回 `this.update(updateWrapper)` 的布尔值）。
    ///
    /// ⚠️ 与 area_code 的差异：本域白名单里**没有**任何需要「唯一性维护」的列
    /// （`is_show` / `sort_order` 都是普通标量），因此没有额外的副作用 SQL。
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

        // 1. 归一（Java `getColumnByPropertyName`）
        let column = map_update_field_column(field)
            .ok_or_else(|| Error::Validation(MSG_INVALID_FIELD_VALUE.into()))?;
        // 2. 白名单
        if !MOBILE_CAT_NAV_FIELDS.contains(&column) {
            return Err(Error::Validation(MSG_INVALID_FIELD_VALUE.into()));
        }
        // 3. 二次保险：白名单两项都是真实列，这里只作防御性断言
        if !MOBILE_CAT_NAV_COLUMNS.contains(&column) {
            return Err(Error::Validation(MSG_INVALID_FIELD_VALUE.into()));
        }

        let val = dto.val.as_deref().unwrap_or_default();
        let affected = self.repos.content.mobile_cat_nav.update_field(id, column, val).await?;
        // ⚠️ Java 仅在 `updated == true` 时 `invalidateHomeDecorateAfterCommit()`。
        Ok(affected > 0)
    }

    // ————————————————————— 批量操作 —————————————————————

    /// 批量操作（对应 Java `MobileCatNavServiceImpl#batch`
    /// → `BaseServiceImpl#batch`）。
    ///
    /// `type != "del"` → **静默返回 `false`**（Java 行为，不是报错）。
    /// `type == "del"` → `removeByIds(ids)`，返回 `rows >= 1`。
    ///
    /// ⚠️ Java 控制器类级**没有** `@Validated`，方法参数上也没有 `@Valid`，
    /// 因此 `BatchDTO.ids` 的 `@NotEmpty` **不生效**（空数组不会 400）。
    /// 本实现按 Java 服务层的可观测行为：空 ids ⇒ `removeByIds(空集合)` ⇒
    /// 不产生 SQL、返回 `false`（与 area_code 的选择一致，见 `uncertain_points`）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if dto.r#type != "del" {
            return Ok(false);
        }
        if dto.ids.is_empty() {
            return Ok(false);
        }
        let affected = self.repos.content.mobile_cat_nav.delete_many(&dto.ids).await?;
        // ⚠️ Java 仅在 `result == true` 时 `invalidateHomeDecorateAfterCommit()`。
        Ok(affected > 0)
    }

    // ————————————————————— 内部：供测试与路由复用的纯逻辑钩子 —————————————————————

    /// `create` 时 `img_url` 列的 JSON 文本（null ⇒ `"[]"`）。
    ///
    /// 暴露为方法是为了让单测能脱离数据库验证 null 语义；
    /// 业务链路内部直接用 [`img_url_for_create`]。
    #[must_use]
    pub fn img_url_json(v: Option<&[nextrade_core::models::content::mobile_cat_nav::GalleryPicVO]>) -> String {
        img_url_for_create(v)
    }

    /// `create` 时 `child_cat_ids` 列的 JSON 文本（null ⇒ `"[]"`）。
    #[must_use]
    pub fn child_cat_ids_json(v: Option<&[i32]>) -> String {
        child_cat_ids_for_create(v)
    }

    /// `update` 时某 JSON 列的取值：`None` = 不写列（Java `toJson(null)` = null）。
    #[must_use]
    pub fn update_json_column<T: serde::Serialize>(v: Option<&[T]>) -> Option<String> {
        v.map(to_json_array)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::content::mobile_cat_nav::{
        GalleryPicVO, MobileCatNavListQuery, IS_SHOW_NOT_SHOW, IS_SHOW_SHOW,
        MOBILE_CAT_NAV_FIELDS, MSG_CATEGORY_ID_NOT_NULL, MSG_INVALID_FIELD_VALUE,
        MSG_MOBILE_CAT_NAV_ID_NOT_NULL,
    };

    // ————————————————————— 常量：与 Java 逐字符对照 —————————————————————

    /// Java `MobileCatNavController.updateField` 的内联白名单逐字对照。
    #[test]
    fn update_field_whitelist_matches_java_literal() {
        assert_eq!(MOBILE_CAT_NAV_FIELDS, ["is_show", "sort_order"]);
        // 与 area_code 不同：本域白名单只有 2 项，且没有 is_default 之类的唯一性列
        assert_eq!(MOBILE_CAT_NAV_FIELDS.len(), 2);
        assert!(!MOBILE_CAT_NAV_FIELDS.contains(&"category_id"));
    }

    /// Java 常量文案逐字符照抄（含「名称」实指 `categoryId` 的笔误）。
    #[test]
    fn java_message_constants_are_verbatim() {
        assert_eq!(MSG_CATEGORY_ID_NOT_NULL, "首页分类栏名称不能为空");
        assert_eq!(MSG_MOBILE_CAT_NAV_ID_NOT_NULL, "首页分类栏id不能为空");
        assert_eq!(MSG_INVALID_FIELD_VALUE, "字段值不合法");
        assert_eq!(MSG_ID_REQUIRED, "id不能为空");
    }

    /// Java `IsShowType` 码值。
    #[test]
    fn is_show_codes_match_java_enum() {
        assert_eq!(IS_SHOW_SHOW, 1);
        assert_eq!(IS_SHOW_NOT_SHOW, 0);
    }

    // ————————————————————— 纯逻辑：updateField 白名单 —————————————————————

    /// 归一 + 白名单：主键能归一但被白名单拒绝；未识别字段直接拒绝。
    #[test]
    fn update_field_whitelist_flow() {
        for (input, column) in [
            ("isShow", "is_show"),
            ("is_show", "is_show"),
            ("sortOrder", "sort_order"),
            ("sort_order", "sort_order"),
        ] {
            let c = map_update_field_column(input).unwrap();
            assert_eq!(c, column);
            assert!(MOBILE_CAT_NAV_FIELDS.contains(&c));
        }
        // 真实列但不在白名单 ⇒ Java 报「字段值不合法」
        let col = map_update_field_column("catColor").unwrap();
        assert_eq!(col, "cat_color");
        assert!(!MOBILE_CAT_NAV_FIELDS.contains(&col));
        // 主键可归一，但不在白名单
        let id_col = map_update_field_column("mobileCatNavId").unwrap();
        assert!(!MOBILE_CAT_NAV_FIELDS.contains(&id_col));
        // 完全无法识别 ⇒ 归一失败
        assert!(map_update_field_column("something").is_none());
    }

    // ————————————————————— 纯逻辑：create / update 的 JSON null 语义 —————————————————————

    /// `create`：img / child 的 null ⇒ `"[]"`，brand 的 null ⇒ 不写列。
    #[test]
    fn create_null_semantics_differ_per_column() {
        assert_eq!(MobileCatNavService::img_url_json(None), "[]");
        assert_eq!(MobileCatNavService::child_cat_ids_json(None), "[]");
        // brand_ids：Java `toJson(null)` = null ⇒ 不写列
        assert_eq!(MobileCatNavService::update_json_column::<i32>(None), None);
        assert_eq!(
            MobileCatNavService::update_json_column(Some(&[9, 10])),
            Some("[9,10]".to_string())
        );
    }

    /// `update`：三列 JSON 的 null **一律**不写列（与 create 不同）。
    #[test]
    fn update_null_semantics_skip_all_json_columns() {
        assert_eq!(MobileCatNavService::update_json_column::<i32>(None), None);
        assert_eq!(MobileCatNavService::update_json_column::<GalleryPicVO>(None), None);
        // 有值时才写。注意本域的序列化器是 **Jackson**（项目 `JsonUtil.toJson`
        // → `ObjectMapper.writeValueAsString`），其默认 `Include.ALWAYS`：
        // null 字段**照常输出**（与 attributes_tpl 的 Hutool 行为相反）。
        // 故这里断言 JSON 语义而非字符串逐字节相等。
        let pics = vec![GalleryPicVO {
            pic_id: Some(1),
            ..Default::default()
        }];
        let json = MobileCatNavService::update_json_column(Some(&pics)).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 1);
        assert_eq!(v[0]["picId"], 1);
        assert!(v[0].as_object().unwrap().contains_key("picUrl"));
    }

    /// create 与 update 的差异是有意为之，用一个断言把两者钉住。
    #[test]
    fn create_and_update_json_null_handling_are_intentionally_different() {
        // create：imgUrl = null ⇒ "[]"
        assert_eq!(MobileCatNavService::img_url_json(None), "[]");
        // update：imgUrl = null ⇒ 不写列
        assert_eq!(MobileCatNavService::update_json_column::<GalleryPicVO>(None), None);
    }

    // ————————————————————— 纯逻辑：入参校验 —————————————————————

    /// `update` 缺主键 ⇒ Java 原文案（`@NotNull` 的 message）。
    #[test]
    fn update_requires_id_with_java_message() {
        let dto: MobileCatNavUpdateDTO =
            serde_json::from_str(r#"{"categoryId":42}"#).unwrap();
        assert_eq!(dto.mobile_cat_nav_id, None);
        assert_eq!(MSG_MOBILE_CAT_NAV_ID_NOT_NULL, "首页分类栏id不能为空");
        // id <= 0 同样视为缺失（filter(|v| *v > 0)）
        let dto: MobileCatNavUpdateDTO =
            serde_json::from_str(r#"{"mobileCatNavId":0}"#).unwrap();
        assert_eq!(dto.mobile_cat_nav_id.filter(|v| *v > 0), None);
    }

    /// `del` 缺主键 ⇒ `id不能为空`（Java `OperateDTO @NotNull`）。
    #[test]
    fn del_requires_id_with_java_message() {
        let dto: OperateDTO = serde_json::from_str(r#"{"adminNote":"x"}"#).unwrap();
        assert_eq!(dto.id, None);
        assert_eq!(MSG_ID_REQUIRED, "id不能为空");
    }

    /// `create` 的 `categoryId` **不**校验（Java 缺 `@Validated` ⇒ `@NotBlank` 不生效）。
    #[test]
    fn create_does_not_validate_category_id() {
        let dto: MobileCatNavCreateDTO = serde_json::from_str("{}").unwrap();
        assert_eq!(dto.category_id, None);
        // Java 有文案但没生效；这里只断言文案存在，不断言被使用
        assert_eq!(MSG_CATEGORY_ID_NOT_NULL, "首页分类栏名称不能为空");
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

    // ————————————————————— 纯逻辑：list 过滤 / 排序 —————————————————————

    /// `isShow` 只有 0 / 1 参与过滤（Java 双重判断）。
    #[test]
    fn list_is_show_gate() {
        let q = MobileCatNavListQuery {
            is_show: Some(IS_SHOW_SHOW),
            ..Default::default()
        };
        assert_eq!(q.effective_is_show(), Some(1));
        let q = MobileCatNavListQuery {
            is_show: Some(IS_SHOW_NOT_SHOW),
            ..Default::default()
        };
        assert_eq!(q.effective_is_show(), Some(0));
        let q = MobileCatNavListQuery {
            is_show: Some(2),
            ..Default::default()
        };
        assert_eq!(q.effective_is_show(), None);
    }

    /// 排序三分支（含非法值不追加 ORDER BY 的缺口）。
    #[test]
    fn list_order_clause() {
        let q = MobileCatNavListQuery {
            sort_order: Some("asc".into()),
            ..Default::default()
        };
        assert_eq!(q.order_clause(), " ORDER BY mobile_cat_nav_id ASC");
        let q = MobileCatNavListQuery::default();
        assert_eq!(q.order_clause(), " ORDER BY mobile_cat_nav_id DESC");
        let q = MobileCatNavListQuery {
            sort_order: Some("nope".into()),
            ..Default::default()
        };
        assert_eq!(q.order_clause(), "");
        // 未识别字段回退主键（防注入）
        let q = MobileCatNavListQuery {
            sort_field: Some("x; DROP TABLE mobile_cat_nav".into()),
            sort_order: Some("asc".into()),
            ..Default::default()
        };
        assert_eq!(q.order_clause(), " ORDER BY mobile_cat_nav_id ASC");
    }
}