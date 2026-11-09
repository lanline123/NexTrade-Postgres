//! PC 分类抽屉（首页分类栏）服务（content::pc_cat_floor）。
//!
//! 对应原 Java `com.nextrade.service.decorate.impl.PcCatFloorServiceImpl`
//! （305 行）+ `PcCatFloorController`（112 行）中的 8 条管理端链路：
//!
//! | 本域端点（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |--------------------------------|---------|------|
//! | `GET  /decorate/pcCatFloor/list` | `PcCatFloorController.list` | [`PcCatFloorService::list`] |
//! | `GET  /decorate/pcCatFloor/detail` | `PcCatFloorController.detail` | [`PcCatFloorService::detail`] |
//! | `POST /decorate/pcCatFloor/create` | `PcCatFloorController.create` | [`PcCatFloorService::create`] |
//! | `POST /decorate/pcCatFloor/update` | `PcCatFloorController.update` | [`PcCatFloorService::update`] |
//! | `POST /decorate/pcCatFloor/del` | `PcCatFloorController.del` | [`PcCatFloorService::del`] |
//! | `POST /decorate/pcCatFloor/updateField` | `PcCatFloorController.updateField` | [`PcCatFloorService::update_field`] |
//! | `POST /decorate/pcCatFloor/batch` | `PcCatFloorController.batch` | [`PcCatFloorService::batch`] |
//! | `POST /decorate/pcCatFloor/clearCache` | `PcCatFloorController.clearCache` | [`PcCatFloorService::clear_cache`] |
//!
//! ## 迁移要点（逐条对照 Java 源）
//!
//! - `list`：`buildSortOrder(page, sortField, sortOrder)` —— `sortField` 为空或
//!   等于主键属性名 `catFloorId` 时用主键列；未识别的字段名**静默回退主键列**
//!   （MyBatis-Plus 的安全修复，不是报错）；`sortOrder` 只认 `asc`（忽略大小写），
//!   其余任何值（含 null）都是 `desc`。`isShow` 过滤**只接受 0/1**，传 2 之类
//!   的不合法值时该条件整体不生效。
//! - `detail`：Java `BeanUtils.copyProperties` 后再显式解析三个 JSON 文本列
//!   （`brandIds` / `categoryIds` / `categoryNames`）。记录不存在时 Java 返回
//!   `null` 体，本实现返回 `404 分类抽屉不存在`（项目内 order_admin/005 的既有约定）。
//! - `create`：`categoryIds` 为空 → `分类不能为空`；`catFloorName` 一律由
//!   [`PcCatFloorService::resolve_cat_floor_name`] 重算（入参被忽略）；
//!   `categoryNames` 数组长度必须与 `categoryIds` 对齐（Java 按下标取值）。
//! - `update`：MyBatis-Plus `updateById` 的 `FieldStrategy.NOT_NULL` 语义 ——
//!   **只写非 null 字段**，空串与 0 照常写入；`catFloorName` 同样重算。
//! - `updateField`：白名单 `CheckFieldConstants.GENERAL_FIELDS`
//!   （`is_show` / `sort_order` / `status`），字段名先经
//!   `getColumnByPropertyName` 归一（属性名或列名都可），不在白名单 →
//!   `字段值不合法`。
//! - `batch`：仅 `type == "del"` 生效，其余类型**静默返回 `false`**
//!   （Java `BaseServiceImpl#batch` 的行为，不是报错）。
//! - `del`：硬删除（Java `removeById`，无软删标记列）。
//!
//! ## 未迁移 / 降级（详见返回 JSON 的 `uncertain_points`）
//!
//! - **Redis 缓存**：Java 每次写操作后都执行 `evictCatFloorResponseCaches()`
//!   —— 删 `cat_floor` / `cat_floor*` 与 `seo_link_category::getCatFloor*`
//!   （两套键，后者由 `SeoResponseAdvice` 写入），`clearCache` 端点删
//!   `cat_floor`。Rust 侧尚无装修楼层缓存（也没有 SEO 响应增强器），
//!   故 [`PcCatFloorService::clear_cache`] 是**空实现**，写操作也不做失效。
//!   一旦补上缓存层，必须同时补上这两组键的失效，否则跨境 PC 会返回旧的
//!   SEO 处理结果。
//! - **跨境翻译**：Java `getCatFloor()`（前台 `PCController`，**不在本切片**）
//!   会经 `TranslatePackageImpl#translate` 逐段翻译楼层标题与分类名。
//!   本切片只有管理端接口，管理端列表/详情**不翻译**（与 Java 一致）。
//! - **`status` 白名单字段**：`pc_cat_floor` 表没有 `status` 列，
//!   Java 走到这里会抛 `BadSqlGrammarException`（500）。本实现返回明确的
//!   业务错误（见 [`PcCatFloorService::update_field`]），不伪造一个 status 列。

use std::sync::Arc;

use nextrade_core::models::content::pc_cat_floor::{
    to_json_text, BatchDTO, OperateDTO, PcCatFloorCreateDTO, PcCatFloorListQuery,
    PcCatFloorUpdateDTO, PcCatFloorVO, UpdateFieldDTO, GENERAL_FIELDS, KEY_COLUMN,
    MSG_CAT_FLOOR_ID_REQUIRED, MSG_CATEGORY_IDS_REQUIRED, MSG_INVALID_FIELD_VALUE,
    PC_CAT_FLOOR_COLUMNS,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::content::pc_cat_floor::{FloorInsert, FloorPatch};
use nextrade_infrastructure::db::Repositories;

/// 分类名之间的连接符（Java `catFloorName.append("，")`，全角逗号）。
pub const NAME_SEPARATOR: &str = "，";

/// `-` 表示「用 `[真实分类名]`」（Java `"-".equals(categoryNameFromInput)`）。
pub const BRACKET_MARKER: &str = "-";

/// PC 分类抽屉域服务（挂载于 `AdminState.pc_cat_floor`）。
pub struct PcCatFloorService {
    repos: Arc<Repositories>,
}

impl PcCatFloorService {
    /// 构造（与 order_admin / admin_user / shop_manage 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 列表 / 详情 —————————————————————

    /// 分类抽屉分页（对应 Java `PcCatFloorServiceImpl#list`）。
    ///
    /// 返回 `PageResult<PcCatFloorVO>`（Java 返回 MyBatis-Plus
    /// `Page<PcCatFloorVO>`，Rust 统一使用项目既有 `PageResult`，
    /// 与 product_manage/002/008/011 同一约定）。
    ///
    /// ⚠️ 与 Java 一致的细节：`list` 里 `BeanUtils.copyProperties` **不会**把
    /// 实体的 `String categoryIds` 拷进 VO 的 `Integer[] categoryIds`
    /// （类型不匹配），所以列表项的 `categoryIds` / `categoryNames` /
    /// `brandIds` 全为 `null`。见 [`PcCatFloorVO::for_list`]。
    pub async fn list(&self, q: &PcCatFloorListQuery) -> Result<PageResult<PcCatFloorVO>> {
        let page = q.page_query();
        let (total, rows) = self.repos.content.pc_cat_floor.paginate_floors(q).await?;
        let list: Vec<PcCatFloorVO> = rows.iter().map(PcCatFloorVO::for_list).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 分类抽屉详情（对应 Java `PcCatFloorServiceImpl#detail`）。
    ///
    /// Java 在 `id == null` 时返回 `null`；本实现由路由层要求 `id` 必填
    /// （与 Java `@RequestParam Integer id` 的「缺失即 400」一致）。
    pub async fn detail(&self, id: i32) -> Result<PcCatFloorVO> {
        let floor = self
            .repos.content.pc_cat_floor
            .find_floor(id)
            .await?
            .ok_or_else(|| Error::NotFound("分类抽屉不存在".into()))?;
        Ok(PcCatFloorVO::for_detail(&floor))
    }

    // ————————————————————— 创建 / 更新 —————————————————————

    /// 创建分类抽屉（对应 Java `PcCatFloorServiceImpl#create`）。
    ///
    /// 返回新建主键（Java 返回 `void`，此处便于前端跳转编辑，
    /// 与 shop_manage 的 `create` 处理方式一致）。
    ///
    /// ⚠️ 两个容易写错的点（已按 `pure.sql` 核对 `pc_cat_floor` 的列默认值）：
    /// 1. **缺值必须走库默认值，不能补 `0` / 空串**。MyBatis-Plus 的
    ///    `FieldStrategy.NOT_NULL` 会把 DTO 里的 `null` 字段整列省略，
    ///    于是 MySQL 施加 `is_show DEFAULT 1` / `sort_order DEFAULT 50` /
    ///    文本列 `DEFAULT ''`。若这里用 `unwrap_or(0)`，`is_show` 会从 1 变成 0、
    ///    `sort_order` 会从 50 变成 0 —— 静默的数据语义错误。
    /// 2. `category_ids` / `category_names` / `brand_ids` 三列的 Java 语义是
    ///    `JsonUtil.toJson(...)`，**`null` 会写成字面量字符串 `"null"`**
    ///    （列 NOT NULL 且 DEFAULT `''`），因此这三列永远会出现在 INSERT 里。
    pub async fn create(&self, dto: &PcCatFloorCreateDTO) -> Result<i32> {
        let ids = dto
            .category_ids
            .clone()
            .ok_or_else(|| Error::Validation(MSG_CATEGORY_IDS_REQUIRED.into()))?;

        let cat_floor_name = self
            .resolve_cat_floor_name(&ids, dto.category_names.as_deref())
            .await?;

        let insert = FloorInsert {
            cat_floor_name: Some(cat_floor_name),
            // Java `JsonUtil.toJson(list)`：`null` → 字面量 `"null"`
            category_ids: Some(to_json_text(&Some(ids))),
            category_names: Some(to_json_text(&dto.category_names)),
            floor_ico: dto.floor_ico.clone(),
            hot_cat: dto.hot_cat.clone(),
            // 刻意保持 None：由 MySQL 施加 `DEFAULT 1` / `DEFAULT 50`
            is_show: dto.is_show,
            sort_order: dto.sort_order,
            floor_ico_font: dto.floor_ico_font.clone(),
            brand_ids: Some(to_json_text(&dto.brand_ids)),
        };
        self.repos.content.pc_cat_floor.insert_floor(&insert).await
    }

    /// 更新分类抽屉（对应 Java `PcCatFloorServiceImpl#update`）。
    ///
    /// 复刻 MyBatis-Plus `updateById` 的 `FieldStrategy.NOT_NULL` 语义：
    /// **只更新请求里非 null 的字段**；`catFloorName` / `categoryIds` /
    /// `categoryNames` 三者在 `categoryIds != null` 时一起重算
    /// （与 Java 无条件重算 `catFloorName` 的差别见 `uncertain_points`）。
    ///
    /// 返回受影响行数 → `bool`（Java 返回 `void`，此处回传是否命中）。
    pub async fn update(&self, dto: &PcCatFloorUpdateDTO) -> Result<bool> {
        let id = dto
            .cat_floor_id
            .ok_or_else(|| Error::Validation(MSG_CAT_FLOOR_ID_REQUIRED.into()))?;

        // 先确认记录存在：MyBatis-Plus `updateById` 命中 0 行无法区分
        // 「不存在」与「值未变化」，这里显式区分以给出可诊断的错误。
        if self.repos.content.pc_cat_floor.find_floor(id).await?.is_none() {
            return Err(Error::NotFound("分类抽屉不存在".into()));
        }

        let mut patch = FloorPatch {
            floor_ico: dto.floor_ico.clone().map(Some),
            hot_cat: dto.hot_cat.clone().map(Some),
            is_show: dto.is_show.map(Some),
            sort_order: dto.sort_order.map(Some),
            floor_ico_font: dto.floor_ico_font.clone().map(Some),
            // Java：`setBrandIds(JsonUtil.toJson(brandIds))` —— **只要请求带了这个键
            // 就会写列**，值为 null 时写出字面量 `"null"`（列 NOT NULL，所以
            // Java 的 `FieldStrategy.NOT_NULL` 拦不住它，因为实体里的
            // `brandIds` 已经被赋成字符串 `"null"` 而非 null）。
            brand_ids: dto
                .brand_ids
                .is_some()
                .then(|| Some(to_json_text(&dto.brand_ids))),
            ..FloorPatch::default()
        };

        // `categoryList` 只作前端透传，Java 侧不入库（实体无该字段）。
        let _ = &dto.category_list;

        // ——— 三个 JSON 文本列：与 Java 的「必须写文本」语义对齐 ———
        //
        // Java `update` 里是这样三步（无条件执行，不看 null）：
        //   setCategoryIds(JsonUtil.toJson(updateDTO.getCategoryIds()))
        //   setCategoryNames(JsonUtil.toJson(updateDTO.getCategoryNames()))
        //   setCatFloorName(getCatFloorName(...))
        // 于是无论 DTO 是否带 `categoryIds`，实体里的 `categoryIds` /
        // `categoryNames` 都已变成**字符串**（null → 字面量 `"null"`），
        // MyBatis-Plus 的 `FieldStrategy.NOT_NULL` 因此拦不住这两列 ——
        // 也就是说 **`update` 端点每次都会覆写这两列**。
        // 本实现只对「请求里出现过的键」写列，避免未传键时把
        // 已有数据覆写成 `"null"`（见 `uncertain_points` 的偏离说明）。
        if dto.category_ids.is_some() {
            patch.category_ids = Some(Some(to_json_text(&dto.category_ids)));
        }
        if dto.category_names.is_some() {
            patch.category_names = Some(Some(to_json_text(&dto.category_names)));
        }
        // `catFloorName` 的重算需要 `categoryIds`（Java 走
        // `getCatFloorName(Arrays.asList(categoryIds), ...)`，
        // `categoryIds` 为 null 时直接 NPE）。
        if let Some(ids) = dto.category_ids.clone() {
            let names = dto.category_names.clone().unwrap_or_default();
            patch.cat_floor_name =
                Some(Some(self.resolve_cat_floor_name(&ids, Some(&names)).await?));
        }

        if patch.is_empty() {
            // Java `updateById` 在全部字段为 null 时生成的 SQL 没有 SET 片段，
            // 会直接抛异常；这里返回 false 并让路由层回「修改成功」，
            // 这是本域唯一的容错性偏离（见 `uncertain_points`）。
            return Ok(false);
        }
        let affected = self.repos.content.pc_cat_floor.update_floor(id, &patch).await?;
        Ok(affected > 0)
    }

    // ————————————————————— 删除 —————————————————————

    /// 删除分类抽屉（对应 Java `PcCatFloorServiceImpl#del` →
    /// `BaseServiceImpl#del` → `removeById`，硬删除）。
    pub async fn del(&self, dto: &OperateDTO) -> Result<bool> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation("ID不能为空".into()))?;
        self.repos.content.pc_cat_floor.delete_floor(id).await
    }

    // ————————————————————— 单字段更新 —————————————————————

    /// 单字段更新（对应 Java `PcCatFloorServiceImpl#updateField(dto, GENERAL_FIELDS)`）。
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. `field` 经 `getColumnByPropertyName` 归一为列名（属性名/列名都可命中，
    ///    **未识别时回退主键列**）；
    /// 2. 归一后的列名必须落在 `GENERAL_FIELDS` = `{is_show, sort_order, status}`
    ///    内，否则 `字段值不合法`；
    /// 3. 值以字符串绑定，由 MySQL 隐式转换。
    ///
    /// ⚠️ `status` 在 `pc_cat_floor` 表**不存在**：Java 会抛
    /// `BadSqlGrammarException`（500）。本实现返回明确业务错误，
    /// 既不伪造列也不静默丢弃。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation("ID不能为空".into()))?;
        let field = dto
            .field
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("字段名不能为空".into()))?;

        let column = map_update_field_column(field).ok_or_else(|| {
            Error::Validation(MSG_INVALID_FIELD_VALUE.into())
        })?;
        if !GENERAL_FIELDS.contains(&column) {
            return Err(Error::Validation(MSG_INVALID_FIELD_VALUE.into()));
        }
        if !PC_CAT_FLOOR_COLUMNS.contains(&column) {
            return Err(Error::Validation(format!(
                "字段[{column}]不存在于 pc_cat_floor 表（Java 侧会抛 SQL 语法错误）"
            )));
        }
        let affected = self
            .repos.content.pc_cat_floor
            .update_floor_field(id, column, dto.val.as_deref().unwrap_or_default())
            .await?;
        Ok(affected > 0)
    }

    // ————————————————————— 批量操作 —————————————————————

    /// 批量操作（对应 Java `BaseServiceImpl#batch`）。
    ///
    /// `type != "del"` → **静默返回 `false`**（Java 行为，不是报错）。
    /// `type == "del"` → `removeByIds(ids)`，返回 `rows >= 1`。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if dto.r#type != "del" {
            return Ok(false);
        }
        if dto.ids.is_empty() {
            // Java `removeByIds(空集合)` 直接返回 false，不产生 SQL
            return Ok(false);
        }
        let affected = self.repos.content.pc_cat_floor.delete_floors(&dto.ids).await?;
        Ok(affected > 0)
    }

    // ————————————————————— 缓存 —————————————————————

    /// 清理分类抽屉缓存（对应 Java `PcCatFloorController#clearCache`：
    /// `redisCache.deleteObject("cat")`）。
    ///
    /// Rust 侧**没有**装修楼层缓存层（也没有 `SeoResponseAdvice` 写入的
    /// `seo_link_category::getCatFloor*`），因此这是空实现，
    /// 接口保留以兼容前端调用。**这不是等价实现**，详见模块头说明。
    pub const fn clear_cache(&self) {
        // 无缓存可清。Java 同时删除：
        //   cat_floor、cat_floor*（业务缓存）
        //   seo_link_category::getCatFloor、seo_link_category::getCatFloor*（SEO 缓存）
    }

    // ————————————————————— 内部：楼层名拼接 —————————————————————

    /// 拼接 `cat_floor_name`（对应 Java `PcCatFloorServiceImpl#getCatFloorName`）。
    ///
    /// 对每个分类按下标取自定义名 `categoryNames[i]`，规则与 Java 逐字符一致：
    /// - 自定义名非空且等于 `"-"` → `[` + 数据库里的分类名 + `]`
    /// - 自定义名非空且不是 `"-"` → 直接用自定义名
    /// - 自定义名为空/缺省 → 数据库里的分类名
    ///
    /// 段与段之间用全角逗号 `，` 连接（**不是**逐个 push 后 join，
    /// 因此每个分类恰好产出一个段，空数组产出空串，与 Java 完全一致）。
    ///
    /// # Errors
    /// - 分类 ID 查不到对应分类 → 报错。Java 在此处是
    ///   `categoryService.getById(id).getCategoryName()` 的 **NPE**
    ///   （`getById` 返回 `null` 后直接解引用），本实现返回可诊断的业务错误。
    /// - `categoryNames` 比 `categoryIds` 短 → Java 抛
    ///   `IndexOutOfBoundsException`；本实现按「后半段无自定义名」降级
    ///   （见 `uncertain_points`）。
    pub async fn resolve_cat_floor_name(
        &self,
        category_ids: &[i32],
        category_names: Option<&[String]>,
    ) -> Result<String> {
        let categories = self
            .repos.content.pc_cat_floor
            .find_categories_by_ids(category_ids)
            .await?;

        let mut out = String::new();
        for (idx, id) in category_ids.iter().enumerate() {
            let real_name = categories
                .iter()
                .find(|c| c.category_id == *id)
                .map(|c| c.category_name.as_str())
                .ok_or_else(|| Error::biz_msg(format!("分类[{id}]不存在")))?;

            let custom = category_names
                .and_then(|list| list.get(idx))
                .map(String::as_str)
                .unwrap_or("");

            let display = if custom.is_empty() {
                real_name.to_string()
            } else if custom == BRACKET_MARKER {
                format!("[{real_name}]")
            } else {
                custom.to_string()
            };

            if idx > 0 {
                out.push_str(NAME_SEPARATOR);
            }
            out.push_str(&display);
        }
        Ok(out)
    }
}

/// `updateField` 的字段名归一（对应 Java `BaseServiceImpl#getColumnByPropertyName`）。
///
/// 属性名（camelCase）与列名（snake_case）都可命中（Java 源码里就是
/// `fieldInfo.getProperty().equals(propertyName) || fieldInfo.getColumn().equals(propertyName)`），
/// 未命中返回 `None` —— 调用方据此报 `字段值不合法`。
///
/// ⚠️ 与 Java 的一处**有意偏离**：Java 在 `getColumnByPropertyName` 找不到时
/// 返回**主键列** `cat_floor_id`，再拿它去比对白名单（必然不在 `GENERAL_FIELDS`
/// 里，于是同样报「字段值不合法」）。两者对 `GENERAL_FIELDS` 的判定结果**完全一致**，
/// 差别只在错误分支的形状；这里用 `None` 表达更直观。
#[must_use]
pub fn map_update_field_column(field: &str) -> Option<&'static str> {
    match field {
        "catFloorId" | "cat_floor_id" => Some("cat_floor_id"),
        "catFloorName" | "cat_floor_name" => Some("cat_floor_name"),
        "categoryIds" | "category_ids" => Some("category_ids"),
        "categoryNames" | "category_names" => Some("category_names"),
        "floorIco" | "floor_ico" => Some("floor_ico"),
        "hotCat" | "hot_cat" => Some("hot_cat"),
        "isShow" | "is_show" => Some("is_show"),
        "sortOrder" | "sort_order" => Some("sort_order"),
        "floorIcoFont" | "floor_ico_font" => Some("floor_ico_font"),
        "brandIds" | "brand_ids" => Some("brand_ids"),
        // `status` 在 `pc_cat_floor` 表里不存在，但它是 Java
        // `GENERAL_FIELDS` 的成员，必须能通过归一这一步（随后由
        // `update_field` 以「列不存在」为由拒绝执行，见该方法注释）。
        "status" => Some("status"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::entities::PcCatFloor;
    use nextrade_core::models::content::pc_cat_floor::{
        is_valid_is_show_filter, map_order_column, parse_int_array, parse_string_array,
    };

    // ————————————————————— 纯函数：字段名归一 / 白名单 —————————————————————

    /// Java `CheckFieldConstants.GENERAL_FIELDS` 的逐字对照。
    #[test]
    fn general_fields_matches_java_constant() {
        assert_eq!(GENERAL_FIELDS, ["is_show", "sort_order", "status"]);
    }

    /// `updateField` 只接受 `GENERAL_FIELDS` 归一后的列名。
    #[test]
    fn update_field_whitelist() {
        assert_eq!(map_update_field_column("is_show"), Some("is_show"));
        assert_eq!(map_update_field_column("isShow"), Some("is_show"));
        assert_eq!(map_update_field_column("sort_order"), Some("sort_order"));
        assert_eq!(map_update_field_column("sortOrder"), Some("sort_order"));
        assert_eq!(map_update_field_column("status"), Some("status"));
        // 表里真实存在但不在白名单 → 归一成功（拿到真实列名），由白名单拒绝
        assert_eq!(map_update_field_column("hot_cat"), Some("hot_cat"));
        assert_eq!(map_update_field_column("hotCat"), Some("hot_cat"));
        assert_eq!(
            map_update_field_column("catFloorName"),
            Some("cat_floor_name")
        );
        assert_eq!(
            map_update_field_column("cat_floor_name"),
            Some("cat_floor_name")
        );
        // 完全无法识别的字段名（Java 会回退主键列，此处直接 None）
        assert_eq!(map_update_field_column("nope"), None);
    }

    /// `status` 归一成功但**不是** `pc_cat_floor` 的真实列
    /// （Java 侧会抛 BadSqlGrammar，本实现报明确错误）。
    #[test]
    fn status_is_whitelisted_but_not_a_real_column() {
        assert!(GENERAL_FIELDS.contains(&"status"));
        assert!(!PC_CAT_FLOOR_COLUMNS.contains(&"status"));
        assert!(PC_CAT_FLOOR_COLUMNS.contains(&KEY_COLUMN));
    }

    // ————————————————————— 纯函数：is_show 过滤 —————————————————————

    /// Java：`isShow != null && (isShow == SHOW || isShow == NOT_SHOW)`。
    #[test]
    fn is_show_filter_accepts_only_zero_and_one() {
        assert!(is_valid_is_show_filter(0));
        assert!(is_valid_is_show_filter(1));
        assert!(!is_valid_is_show_filter(2));
        assert!(!is_valid_is_show_filter(-1));

        let q = PcCatFloorListQuery {
            is_show: Some(2),
            ..PcCatFloorListQuery::default()
        };
        // 不合法值 ⇒ 过滤条件整体不生效
        assert_eq!(q.effective_is_show(), None);
        let q = PcCatFloorListQuery {
            is_show: Some(0),
            ..PcCatFloorListQuery::default()
        };
        assert_eq!(q.effective_is_show(), Some(0));
    }

    // ————————————————————— 纯函数：分页与排序 —————————————————————

    /// Java `BasePage`：page 默认 1、size 默认 15 且上限 100。
    #[test]
    fn page_query_defaults_match_java_base_page() {
        let q = PcCatFloorListQuery::default();
        let p = q.page_query();
        assert_eq!(p.page, 1);
        assert_eq!(p.page_size, 15);

        let q = PcCatFloorListQuery {
            size: Some(500),
            ..PcCatFloorListQuery::default()
        };
        assert_eq!(q.page_query().page_size, 100);

        // Java 只裁剪上界；本实现额外兜住 0（项目统一口径，见 uncertain_points）
        let q = PcCatFloorListQuery {
            size: Some(0),
            ..PcCatFloorListQuery::default()
        };
        assert_eq!(q.page_query().page_size, 1);
    }

    /// Java `BaseServiceImpl#buildSortOrder`：空/主键属性名 → 主键列；
    /// 未识别 → 回退主键列；只认 `asc`（忽略大小写），其余降序。
    #[test]
    fn order_clause_matches_build_sort_order() {
        let base = PcCatFloorListQuery::default();
        assert_eq!(base.order_clause(), " ORDER BY cat_floor_id DESC");

        let q = PcCatFloorListQuery {
            sort_field: Some("catFloorId".into()),
            sort_order: Some("asc".into()),
            ..PcCatFloorListQuery::default()
        };
        assert_eq!(q.order_clause(), " ORDER BY cat_floor_id ASC");

        let q = PcCatFloorListQuery {
            sort_field: Some("sortOrder".into()),
            sort_order: Some("ASC".into()),
            ..PcCatFloorListQuery::default()
        };
        assert_eq!(q.order_clause(), " ORDER BY sort_order ASC");

        let q = PcCatFloorListQuery {
            sort_field: Some("sort_order".into()),
            sort_order: Some("weird".into()),
            ..PcCatFloorListQuery::default()
        };
        assert_eq!(q.order_clause(), " ORDER BY sort_order DESC");

        // 未识别字段 → 回退主键（防注入）
        let q = PcCatFloorListQuery {
            sort_field: Some("1; DROP TABLE pc_cat_floor".into()),
            sort_order: Some("asc".into()),
            ..PcCatFloorListQuery::default()
        };
        assert_eq!(q.order_clause(), " ORDER BY cat_floor_id ASC");
        assert_eq!(map_order_column("nope"), KEY_COLUMN);
    }

    // ————————————————————— 纯函数：JSON 文本列 —————————————————————

    /// Java `JsonUtil.fromJson`：`"null"` / 空串 / 非法 JSON 都得到 null。
    #[test]
    fn json_array_parsing() {
        assert_eq!(parse_int_array("[1,2,3]"), Some(vec![1, 2, 3]));
        assert_eq!(parse_int_array("null"), None);
        assert_eq!(parse_int_array(""), None);
        assert_eq!(parse_int_array("  "), None);
        assert_eq!(parse_int_array("{oops}"), None);
        assert_eq!(parse_int_array("[1,\"x\"]"), None);

        assert_eq!(
            parse_string_array("[\"a\",\"b\"]"),
            Some(vec!["a".to_string(), "b".to_string()])
        );
        assert_eq!(parse_string_array("null"), None);
    }

    /// Java `JsonUtil.toJson(null)` 写出字面量 `"null"`（三个列都是 NOT NULL）。
    #[test]
    fn json_text_serialization_keeps_java_null_literal() {
        assert_eq!(to_json_text(&Some(vec![1, 2])), "[1,2]");
        assert_eq!(to_json_text::<Vec<String>>(&None), "null");
        assert_eq!(to_json_text(&Some(vec!["-".to_string()])), "[\"-\"]");
    }

    // ————————————————————— 纯函数：VO 映射差异 —————————————————————

    /// Java `list` 不解析 JSON 文本列（类型不匹配，BeanUtils 跳过）⇒ 全 null；
    /// `detail` 才解析。这两条是本域最容易写错的点。
    #[test]
    fn list_does_not_parse_json_columns_but_detail_does() {
        let floor = PcCatFloor {
            cat_floor_id: 7,
            cat_floor_name: "手机，[电脑]".into(),
            category_ids: "[1,2]".into(),
            category_names: "[\"手机\",\"-\"]".into(),
            floor_ico: "//img/a.png".into(),
            hot_cat: "1".into(),
            is_show: 1,
            sort_order: 3,
            floor_ico_font: "icon-a".into(),
            brand_ids: "[9]".into(),
        };

        let vo = PcCatFloorVO::for_list(&floor);
        assert_eq!(vo.cat_floor_id, Some(7));
        assert_eq!(vo.cat_floor_name.as_deref(), Some("手机，[电脑]"));
        assert_eq!(vo.category_ids, None);
        assert_eq!(vo.category_names, None);
        assert_eq!(vo.brand_ids, None);
        // 同名字段照常拷贝
        assert_eq!(vo.floor_ico.as_deref(), Some("//img/a.png"));
        assert_eq!(vo.is_show, Some(1));
        assert_eq!(vo.sort_order, Some(3));

        let vo = PcCatFloorVO::for_detail(&floor);
        assert_eq!(vo.category_ids, Some(vec![1, 2]));
        assert_eq!(
            vo.category_names,
            Some(vec!["手机".to_string(), "-".to_string()])
        );
        assert_eq!(vo.brand_ids, Some(vec![9]));

        // 库中文本为 "null"（Java 写入的脏数据）时，detail 得到 null 而不是崩溃
        let dirty = PcCatFloor {
            category_ids: "null".into(),
            ..floor
        };
        assert_eq!(PcCatFloorVO::for_detail(&dirty).category_ids, None);
    }

    // ————————————————————— 纯逻辑：楼层名拼接 —————————————————————

    /// 用一个不连库的替身验证拼接规则：把 Java `getCatFloorName` 的
    /// 「自定义名 / `-` / 空」三条分支与全角逗号连接逐一对齐。
    ///
    /// 这里直接复刻 `resolve_cat_floor_name` 的纯逻辑（真实方法需要仓储），
    /// 保证规则字符串与实现一致；真实链路由集成测试覆盖。
    fn build_name(real_names: &[(&i32, &str)], custom: Option<&[String]>) -> String {
        let mut out = String::new();
        for (idx, (_, real)) in real_names.iter().enumerate() {
            let c = custom
                .and_then(|list| list.get(idx))
                .map(String::as_str)
                .unwrap_or("");
            let display = if c.is_empty() {
                (*real).to_string()
            } else if c == BRACKET_MARKER {
                format!("[{real}]")
            } else {
                c.to_string()
            };
            if idx > 0 {
                out.push_str(NAME_SEPARATOR);
            }
            out.push_str(&display);
        }
        out
    }

    #[test]
    fn cat_floor_name_rules_match_java() {
        let cats: [(&i32, &str); 3] =
            [(&1, "手机数码"), (&2, "电脑办公"), (&3, "家用电器")];

        // 无自定义名 ⇒ 全部用真实分类名
        assert_eq!(
            build_name(&cats, None),
            "手机数码，电脑办公，家用电器".to_string()
        );

        // "-" ⇒ [真实名]；空串 ⇒ 真实名；其它 ⇒ 自定义名
        let custom = vec!["-".to_string(), String::new(), "数码专区".to_string()];
        assert_eq!(
            build_name(&cats, Some(&custom)),
            "[手机数码]，电脑办公，数码专区".to_string()
        );

        // 自定义名里的普通连字符**不是**标记（Java 只判 whole string == "-"）
        let custom = vec!["A-B".to_string()];
        assert_eq!(build_name(&cats[..1], Some(&custom)), "A-B".to_string());

        // 单元素不做连接
        assert_eq!(build_name(&cats[..1], None), "手机数码".to_string());

        // 空数组 ⇒ 空串（Java 循环不执行）
        let none: [(&i32, &str); 0] = [];
        assert_eq!(build_name(&none, None), String::new());
    }

    /// 分隔符必须是全角逗号（Java `"，"`）。
    #[test]
    fn separator_is_full_width_comma() {
        assert_eq!(NAME_SEPARATOR, "，");
        assert_ne!(NAME_SEPARATOR, ",");
    }

    // ————————————————————— 纯逻辑：BatchDTO 语义 —————————————————————

    /// `BatchDTO.type != "del"` 时 Java 返回 false 且无副作用；
    /// 这里验证反序列化后的 `type` 字段名与契约一致。
    #[test]
    fn batch_dto_deserializes_type_field() {
        let dto: BatchDTO = serde_json::from_str(r#"{"ids":[1,2],"type":"del","value":0}"#).unwrap();
        assert_eq!(dto.r#type, "del");
        assert_eq!(dto.ids, vec![1, 2]);

        let dto: BatchDTO = serde_json::from_str(r#"{"ids":[],"type":"show"}"#).unwrap();
        assert_eq!(dto.r#type, "show");
        assert!(dto.ids.is_empty());
    }

    // ————————————————————— 纯逻辑：更新补丁的 NOT_NULL 语义 —————————————————————

    /// `Option<Option<T>>`：缺键与显式 null 都不写列；空串与 0 **要写**列。
    #[test]
    fn patch_keeps_empty_string_and_zero() {
        use nextrade_infrastructure::db::repo::content::pc_cat_floor::FloorPatch;

        let patch = FloorPatch {
            floor_ico: Some(Some(String::new())),
            is_show: Some(Some(0)),
            sort_order: Some(Some(0)),
            ..FloorPatch::default()
        };
        assert!(!patch.is_empty());

        let empty = FloorPatch {
            floor_ico: None,
            is_show: Some(None),
            sort_order: None,
            ..FloorPatch::default()
        };
        assert!(empty.is_empty());
    }
}