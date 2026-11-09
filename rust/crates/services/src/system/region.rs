//! 地区管理服务（system::region）。
//!
//! 对应原 Java
//! `java/nextrade-service/src/main/java/com/nextrade/service/setting/impl/RegionServiceImpl.java`
//! （413 行）中由 **管理端** `RegionController`
//! （`java/nextrade-adminapi/.../controller/setting/RegionController.java`）
//! 调用的 8 个方法：
//!
//! | 本域端点（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |--------------------------------|---------|------|
//! | `GET  /setting/region/getAllRegionTree` | `RegionController.getAllRegionTree` | [`RegionService::region_tree`] |
//! | `GET  /setting/region/list` | `RegionController.list` | [`RegionService::list`] |
//! | `GET  /setting/region/detail` | `RegionController.detail` | [`RegionService::detail`] |
//! | `POST /setting/region/updateField` | `RegionController.updateField` | [`RegionService::update_field`] |
//! | `POST /setting/region/create` | `RegionController.create` | [`RegionService::create`] |
//! | `POST /setting/region/update` | `RegionController.update` | [`RegionService::update`] |
//! | `POST /setting/region/del` | `RegionController.del` | [`RegionService::del`] |
//! | `POST /setting/region/batch` | `RegionController.batch` | [`RegionService::batch`] |
//!
//! ## 迁移要点（逐条对照 Java 源）
//!
//! ### `getAllRegionTree`
//!
//! `list(level >= 0 或 level > 0)` → `getRegionTree(...)` →
//! Hutool `TreeUtil.build(regionList, 0, config, consumer)`：
//! - 版本开关 `nextradeProperties.getIsOverseas()`：`== 1` 用 `level >= 0`，
//!   否则 `level > 0`。Rust 侧复用 identity::authority 的
//!   [`EditionFlags::default()`]（全功能部署 `is-overseas: 1` ⇒
//!   **国内版 ⇒ `level > 0`**），与 trade::proxy_shopping_user_address 的既有约定一致。
//! - `TreeNodeConfig`：`idKey = regionId`、`parentIdKey = parentId`、
//!   `nameKey = regionName`、`deep = 8`。
//! - 根 = `parent_id == 0` 的那些节点。
//!
//! ### `list`
//!
//! - `parentId != null && parentId >= 0` → `eq(parent_id)`
//! - `regionId != null && regionId >= 0` → `eq(region_id)`
//! - 排序**两个分支形状不同**（本域最容易写错的地方）：
//!   - `sortField` 为空白 ⇒ `ORDER BY sort_order ASC, region_id ASC`
//!     （Java `queryWrapper.orderByAsc(...).orderByAsc(...)`，**双列升序**）
//!   - `sortField` 非空 ⇒ `buildSortOrder(page, sortField, sortOrder)`
//!     （**单列**；`asc` 升序，空或 `desc` 降序，**其它非法值完全不追加
//!     ORDER BY**）
//!   - `RegionListVO` 是 `BeanUtils.copyProperties`，`standard_code`
//!     **不在 VO 里**。
//!
//! ### `detail`
//!
//! Java `getById(id)`，`null` ⇒ 抛 `GlobalException("地区不存在")`，
//! 否则 `BeanUtils.copyProperties` 到 `RegionListVO`。
//!
//! ### `updateField`
//!
//! `updateField(updateField, REGION_FIELDS)`，白名单
//! `CheckFieldConstants.REGION_FIELDS` = `{region_name, sort_order,
//! is_hot, first_word}`。字段名先经 `getColumnByPropertyName` 归一
//! （属性名或列名都可命中），再比对白名单，未命中 ⇒ 「字段值不合法」。
//! 注意白名单里**没有** `parent_id` / `level`。
//!
//! ### `create`
//!
//! Java 只有 `BeanUtils.copyProperties(dto, region)` + `save(region)`：
//! **不查父级、不推导 level、不推导首字母**，`null` 字段交给
//! MyBatis-Plus `FieldStrategy.NOT_NULL` 跳过。唯一的关卡是 Bean
//! Validation（`@NotNull regionName` / `@NotNull parentId` /
//! `@Size(max = 200)` / `@Size(max = 1)`）。
//!
//! ⚠️ 注意 `create` 与 `update` 的**规则不对称**：`update` 会重算
//! `level`、会推导 `first_word`、会给 `is_hot` / `sort_order` 补默认值；
//! `create` 一概不做。本实现逐字复刻，**不**把 `update` 的规则提到
//! `create`（那会造成与 Java 的行为不一致）。
//!
//! ### `update`
//!
//! `updateRegion(dto)` 逐条：
//! 1. `getById(dto.getId())`，`null` ⇒ 「地区不存在」
//! 2. `parentId == null` ⇒ 「上级地区不能为空」
//! 3. `parentId == dto.getId()` ⇒ 「上级地区不能选择自身」
//! 4. `level = 1`；`parentId != 0` 时查父级，父级 `null` **或** 其
//!    `level` 为 `null` ⇒ 「上级地区不存在」，否则 `level = 父级 level + 1`
//! 5. 覆盖 `regionName` / `parentId` / `level` /
//!    `isHot`（null ⇒ 0）/ `sortOrder`（null ⇒ 50）/ `firstWord`
//!    （空白 ⇒ 名称拼音首字母；否则取首字符，都要 `.toUpperCase()`）
//! 6. `updateById(region)`
//!
//! ⚠️ `standard_code` **不在** `set` 清单里，故更新时原值保持不动。
//!
//! ⚠️ 第 3 步只挡「自己当自己的父亲」，**不挡**级联成环
//! （把父级设成自己的子孙），Java 也不挡 —— 已记入 `uncertain_points`。
//!
//! ### `del`
//!
//! Java `del(id)` → `BaseServiceImpl#del`：`id == null` 返回 `false`，
//! 否则 `removeById(id)`。**硬删除**（`region` 表无软删标记列），
//! 且**不校验是否存在子节点** —— 删父级会留下孤儿行。
//!
//! ### `batch`
//!
//! Java `batch(batchDTO)` → `BaseServiceImpl#batch`：仅 `type == "del"`
//! 走 `removeByIds(ids)`，其余类型**静默返回 `false`**（HTTP 200，不报错）。
//!
//! ## 未迁移 / 有差异的点（详见 `uncertain_points`）
//!
//! - **拼音**：Java 用 Hutool `PinyinUtil` 取地区名全拼 / 首字母。
//!   Rust 工作区无拼音依赖（且不允许改 `Cargo.toml`），
//!   [`pinyin_first_letter`] 对非 ASCII 首字符返回**空串**，
//!   而 Java 会给出真实拼音首字母。`update` 的 `first_word` 推导因此
//!   在中文名 + 未传首字母时与 Java **不一致**（这是本切片最重要的差异）。
//! - **`first_word` 首字符**：`normalize_first_word` 用「字符」而非
//!   Java `substring(0, 1)` 的「UTF-16 code unit」，对 BMP 内字符一致，
//!   对 emoji 等代理对字符更安全（Java 会切出半个字符）。
//! - **`@JsonTranslate`**：Java `Region` 实体的 `regionName` 带跨境翻译注解，
//!   管理端 VO 会经翻译切面。Rust 侧无翻译切面，原样返回库中值。
//! - **无 Redis 缓存**：本域 8 个接口都不涉及缓存
//!   （走缓存的是前台 `getCityRegion`，属 system::region_client）。

use std::sync::Arc;

use nextrade_core::entities::Region;
use nextrade_core::models::system::region::{
    build_region_tree, map_update_field_column, normalize_first_word, resolve_level,
    validate_first_word, validate_parent_id, validate_region_name, BatchDTO, OperateDTO,
    RegionCreateDTO, RegionListQuery, RegionListVO, RegionLocationVO, RegionTreeNode,
    RegionUpdateDTO, UpdateFieldDTO,
    DEFAULT_SORT_ORDER, MSG_INVALID_FIELD_VALUE, MSG_ID_REQUIRED, MSG_PARENT_ID_REQUIRED,
    MSG_PARENT_IS_SELF, MSG_PARENT_NOT_FOUND, MSG_REGION_ID_REQUIRED, MSG_REGION_NAME_MAX_SIZE,
    MSG_REGION_NAME_REQUIRED, MSG_REGION_NOT_FOUND, NO, REGION_COLUMNS, REGION_FIELDS, YES,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::system::region::{RegionInsert, RegionPatch};
use nextrade_infrastructure::db::Repositories;

use crate::identity::authority::EditionFlags;

/// 本域树构建时的**根父级 id**（Java `TreeUtil.build(regionList, 0, ...)`
/// 的字面量 `0`）。
pub const TREE_ROOT_PARENT_ID: i32 = 0;

/// 版本开关：是否跨境版（Java `NexTradeProperties.getIsOverseas()`）。
///
/// Java 取自 `application.yaml` 的 `nextrade.is-overseas`（全功能部署默认 `1`）；
/// Rust 侧复用 identity::authority 的既有约定，不新建配置服务
/// （与 trade::proxy_shopping_user_address 的 [`crate::trade::proxy_shopping_user_address::default_is_overseas`] 同一做法）。
#[must_use]
pub fn default_is_overseas() -> bool {
    EditionFlags::default().is_overseas == 1
}

/// 地区管理域服务（挂载于 `AdminState.region`）。
pub struct RegionService {
    repos: Arc<Repositories>,
}

impl RegionService {
    /// 构造（与 product_manage / area_code / region_client 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 地区树 —————————————————————

    /// 获取所有地区（树形）（对应 Java `RegionServiceImpl#getAllRegionTree`）。
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. `isOverseas == 1` ⇒ `level >= 0`；否则 ⇒ `level > 0`
    /// 2. `TreeUtil.build(regionList, 0, config, ...)`：
    ///    根 = `parent_id == 0`，`deep = 8`
    ///
    /// ⚠️ 第 1 步的 `>= 0` / `> 0` 在 `level` 列为 `NULL` 时都会把该行
    /// **排除**（SQL 三值逻辑），与 Java 一致。
    pub async fn region_tree(&self) -> Result<Vec<RegionTreeNode>> {
        let min_level = if default_is_overseas() { 0 } else { 1 };
        let rows = self.repos.system.region.find_all_for_tree(min_level).await?;
        let nodes: Vec<RegionTreeNode> = rows.iter().map(node_from_region).collect();
        Ok(build_region_tree(nodes, TREE_ROOT_PARENT_ID))
    }

    /// 获取所有地区（树形），**显式指定**版本开关
    /// （供测试与「跨境版」部署按需切换）。
    ///
    /// 这是对 [`Self::region_tree`] 的参数化版本，语义完全相同；
    /// 路由层调用的是 `region_tree()`（走 `application.yaml` 的默认值）。
    pub async fn region_tree_with_edition(&self, is_overseas: bool) -> Result<Vec<RegionTreeNode>> {
        let min_level = if is_overseas { 0 } else { 1 };
        let rows = self.repos.system.region.find_all_for_tree(min_level).await?;
        let nodes: Vec<RegionTreeNode> = rows.iter().map(node_from_region).collect();
        Ok(build_region_tree(nodes, TREE_ROOT_PARENT_ID))
    }

    // ————————————————————— 列表 / 详情 —————————————————————

    /// 地区分页（对应 Java `RegionServiceImpl#list`）。
    ///
    /// 返回 `PageResult<RegionListVO>`（Java 返回 MyBatis-Plus
    /// `Page<RegionListVO>`，Rust 统一使用项目既有 `PageResult`，
    /// 与 product_manage/002/075 同一约定）。
    pub async fn list(&self, q: &RegionListQuery) -> Result<PageResult<RegionListVO>> {
        let page = q.page_query();
        let (total, rows) = self.repos.system.region.paginate(q).await?;
        let list: Vec<RegionListVO> = rows.iter().map(RegionListVO::from).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 地区详情（对应 Java `RegionController.detail`）。
    ///
    /// Java：`regionService.getById(id)`；`null` ⇒
    /// `throw new GlobalException("地区不存在")`；否则
    /// `BeanUtils.copyProperties(region, result)` 成 `RegionListVO`。
    ///
    /// 本实现用 `Error::NotFound("地区不存在")` 承载
    /// （文案与 Java `GlobalException` 逐字符一致）。
    pub async fn detail(&self, id: i32) -> Result<RegionListVO> {
        let region = self
            .repos.system.region
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_REGION_NOT_FOUND.into()))?;
        Ok(RegionListVO::from(&region))
    }

    /// 按 id 查地区定位信息（Java 来源：新版管理端前端
    /// `view/admin/src/api/setting/region.ts#getLocationByRegionId`，
    /// Java `RegionController` 未含此端点，按最小可用实现）。
    ///
    /// 返回 `{ regionId, regionName, parentId, level, centor: null }`；
    /// `centor`（中心点）在 `region` 表已无对应列，恒为 `null`。
    /// 地区不存在时返回 `Ok(None)`（调用方决定 404 文案）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn get_location_by_region_id(&self, region_id: i32) -> Result<Option<RegionLocationVO>> {
        let region = self.repos.system.region.find_by_id(region_id).await?;
        Ok(region.as_ref().map(RegionLocationVO::from))
    }

    // ————————————————————— 新增 —————————————————————

    /// 新增地区（对应 Java `RegionServiceImpl#create`）。
    ///
    /// Java 只有两步：
    /// 1. Bean Validation（`@NotNull regionName` / `@NotNull parentId` /
    ///    `@Size(max = 200)` / `@Size(max = 1)`）
    /// 2. `BeanUtils.copyProperties(dto, region)` + `save(region)`
    ///
    /// ⚠️ **不推导 `level`、不推导 `first_word`、不补 `is_hot` /
    /// `sort_order` 默认值** —— 这些都是 `update` 才有的规则。
    /// 本实现逐字复刻（前端若想拿到合理默认值，必须自己在 `create`
    /// 请求里带上）。见模块头「规则不对称」说明。
    ///
    /// 返回新建主键（Java 控制器返回 `void`，此处便于前端跳转编辑，
    /// 与 area_code / shop_manage / pc_cat_floor 的处理方式一致）。
    pub async fn create(&self, dto: &RegionCreateDTO) -> Result<i32> {
        // 1. Bean Validation（文案逐字符照抄 Java 注解的 message）
        let region_name = validate_region_name(dto.region_name.as_deref())
            .map_err(Error::Validation)?;
        let parent_id = validate_parent_id(dto.parent_id).map_err(Error::Validation)?;
        let first_word = validate_first_word(dto.first_word.as_deref())
            .map_err(Error::Validation)?;

        // 2. 入库：`null` 的列**不写**（MyBatis-Plus `FieldStrategy.NOT_NULL`），
        //    交给 MySQL 列默认值
        let id = self
            .repos.system.region
            .insert(&RegionInsert {
                region_id: dto.region_id,
                region_name,
                parent_id,
                level: dto.level,
                is_hot: dto.is_hot,
                sort_order: dto.sort_order,
                first_word,
                // `standard_code` 不在 Java `RegionCreateDTO` 里，不写该列
                standard_code: None,
            })
            .await?;

        tracing::info!("新增地区成功: region_id={id}, parent_id={parent_id}");
        Ok(id)
    }

    // ————————————————————— 更新 —————————————————————

    /// 更新地区（对应 Java `RegionServiceImpl#updateRegion`）。
    ///
    /// 顺序与 Java 逐条对齐（见模块头 `update` 小节），
    /// 关键点：
    /// - `id` 来自 `dto.getId()`（**不是** `dto.getRegionId()`）
    /// - `parentId == id` ⇒ 「上级地区不能选择自身」
    /// - `parentId == 0` ⇒ `level = 1`；否则 `level = 父级.level + 1`，
    ///   父级不存在 **或** 其 `level` 为 `null` ⇒ 「上级地区不存在」
    /// - `isHot` / `sortOrder` 为 `null` 时分别补 `0` / `50`
    /// - `firstWord` 空白 ⇒ 由 `regionName` 推导
    ///
    /// 返回是否命中记录（Java 控制器返回 `void`，此处回传受影响行数 > 0）。
    pub async fn update(&self, dto: &RegionUpdateDTO) -> Result<bool> {
        let base = dto.as_create();

        // 1. 主键（Java `RegionUpdateDTO.id` 上 `@NotNull`）
        let id = dto
            .id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation(MSG_REGION_ID_REQUIRED.into()))?;

        // 2. 记录必须存在（Java `this.getById(dto.getId())` + `null` 检查）
        //
        // ⚠️ Java 取出实体后只用于存在性判断，随后逐字段 `set` 覆盖；
        // 唯一「读过但不重写」的是 `standard_code`（不在 `set` 清单里），
        // 由仓储层 `RegionPatch.standard_code = None` 保证其原值不动。
        self.repos.system.region
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_REGION_NOT_FOUND.into()))?;

        // 3. 名称校验（Java `RegionUpdateDTO extends RegionCreateDTO`，
        //    父类的 `@NotNull` / `@Size(max = 200)` 同样生效）
        let region_name = validate_region_name(base.region_name.as_deref())
            .map_err(Error::Validation)?;
        // 首字母同样受父类 `@Size(max = 1)` 约束
        let raw_first_word = validate_first_word(base.first_word.as_deref())
            .map_err(Error::Validation)?;

        // 4. 上级地区不能为空（Java 第 236–238 行）
        let parent_id = base
            .parent_id
            .ok_or_else(|| Error::Validation(MSG_PARENT_ID_REQUIRED.into()))?;

        // 5. 上级地区不能选择自身（Java 第 239–241 行）
        if parent_id == id {
            return Err(Error::Validation(MSG_PARENT_IS_SELF.into()));
        }

        // 6. 层级推导（Java 第 242–249 行）
        let parent_level = if parent_id == 0 {
            // Java 的 `parentId != 0` 守卫：为 0 时**根本不查**父节点
            None
        } else {
            self.repos.system.region.find_level_by_id(parent_id).await?
        };
        let level = resolve_level(parent_id, parent_level).map_err(Error::Validation)?;

        // 7. 字段覆盖（Java 的 `set` 清单，逐条对齐）
        //    - isHot：`dto.getIsHot() == null ? 0 : ...`
        //    - sortOrder：`dto.getSortOrder() == null ? 50 : ...`
        //    - firstWord：空白 ⇒ 名称拼音首字母；否则取首字符；都大写
        let is_hot = base.is_hot.unwrap_or(NO);
        let sort_order = base.sort_order.unwrap_or(DEFAULT_SORT_ORDER);
        let first_word = normalize_first_word(raw_first_word.as_deref(), &region_name);

        let patch = RegionPatch {
            // 这 6 列在 Java 里都被 `set` 了 ⇒ 一定会出现在 SET 子句里
            region_name: Some(Some(region_name)),
            parent_id: Some(Some(parent_id)),
            level: Some(Some(level)),
            is_hot: Some(Some(is_hot)),
            sort_order: Some(Some(sort_order)),
            first_word: Some(Some(first_word)),
            // `standard_code` **不在** Java 的 `set` 清单里 ⇒ 保持原值
            standard_code: None,
        };

        let affected = self.repos.system.region.update(id, &patch).await?;
        tracing::info!("更新地区: region_id={id}, parent_id={parent_id}, level={level}, 影响 {affected} 行");
        Ok(affected > 0)
    }

    // ————————————————————— 删除 / 批量 —————————————————————

    /// 删除地区（对应 Java `RegionController.del`
    /// → `RegionServiceImpl#del` → `BaseServiceImpl#del` → `removeById`）。
    ///
    /// ⚠️ Java 的 `del` 是「`id == null` 返回 `false`，否则 `removeById(id)`」，
    /// **不校验记录是否存在**（删不存在的 id 也返回成功），
    /// 也**不校验子节点**（删父级会留下孤儿行）。
    /// 本实现保持这一语义（返回删除是否命中），不额外抛 404、不拦截。
    ///
    /// # Errors
    /// - `id` 缺失 ⇒ `id不能为空`（Java `OperateDTO.id` 上的 `@NotNull`）
    pub async fn del(&self, dto: &OperateDTO) -> Result<bool> {
        let id = dto
            .id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        self.repos.system.region.delete(id).await
    }

    /// 批量操作（对应 Java `BaseServiceImpl#batch`）。
    ///
    /// `type != "del"` ⇒ **静默返回 `false`**（Java 行为，不是报错）。
    /// `type == "del"` ⇒ `removeByIds(ids)`，返回 `rows >= 1`。
    ///
    /// ⚠️ Java 控制器类级 `@Validated` + 参数 `@Valid` 会让
    /// `BatchDTO.ids` 的 `@NotEmpty` 生效（空数组 → 400）。
    /// 本实现按项目惯例在服务层返回 `false` 而不是 400
    /// （与 area_code / pc_cat_floor 的选择一致，见 `uncertain_points`）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if dto.r#type != "del" {
            return Ok(false);
        }
        if dto.ids.is_empty() {
            // Java `removeByIds(空集合)` 直接返回 false，不产生 SQL
            return Ok(false);
        }
        let affected = self.repos.system.region.delete_many(&dto.ids).await?;
        Ok(affected > 0)
    }

    /// 统计某地区的直接子节点数（本域新增的防御性能力，**非 Java 概念**）。
    ///
    /// ⚠️ **Java 的 `del` 不做这个检查**，本方法当前**不被任何路由调用**。
    /// 保留它是为了让主会话 / 产品在确认「删除父级是否应拦截」后
    /// 一行接入，而不必重读 Java 源码。
    ///
    /// # Errors
    /// - `id` 缺失 ⇒ `id不能为空`
    pub async fn count_children(&self, dto: &OperateDTO) -> Result<i64> {
        let id = dto
            .id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        self.repos.system.region.count_children(id).await
    }

    // ————————————————————— 单字段更新 —————————————————————

    /// 单字段更新（对应 Java `RegionController.updateField`
    /// → `BaseServiceImpl#updateField(dto, REGION_FIELDS)`）。
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. `field` 经 `getColumnByPropertyName` 归一为列名
    ///    （属性名/列名都可命中；**未识别时回退主键列** ——
    ///    本实现用 `None` 表达，对白名单的判定结果一致）；
    /// 2. 归一后的列名必须落在 `REGION_FIELDS` = `{region_name,
    ///    sort_order, is_hot, first_word}` 内，否则 `字段值不合法`；
    /// 3. `UPDATE region SET <列> = ? WHERE region_id = ?`，
    ///    值以字符串绑定，由 MySQL 按目标列类型隐式转换。
    ///
    /// ⚠️ **没有任何联动逻辑**：改 `is_hot` 不会碰清仓字段，
    /// 改 `region_name` 也**不会**重算 `first_word`（Java 的
    /// `updateField` 是纯 `SET 单列`，不像 `update` 那样推导）。
    /// 本实现同样不做 —— 加上会与 Java 行为不一致。
    ///
    /// ⚠️ Java 的 `update(wrapper)` **不校验记录存在**，
    /// 影响 0 行也返回成功（`false`）；这里同样不抛 404。
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

        // 1. 归一（Java `getColumnByPropertyName`）
        let column = map_update_field_column(field)
            .ok_or_else(|| Error::Validation(MSG_INVALID_FIELD_VALUE.into()))?;
        // 2. 白名单（Java `Arrays.asList(REGION_FIELDS).contains(field)`）
        if !REGION_FIELDS.contains(&column) {
            return Err(Error::Validation(MSG_INVALID_FIELD_VALUE.into()));
        }
        // 3. 二次保险：白名单四项都是真实列，这里只作防御性断言
        if !REGION_COLUMNS.contains(&column) {
            return Err(Error::Validation(MSG_INVALID_FIELD_VALUE.into()));
        }

        let val = dto.val.as_deref().unwrap_or_default();
        let affected = self.repos.system.region.update_field(id, column, val).await?;
        Ok(affected > 0)
    }

    // ————————————————————— 纯逻辑转发（供路由层 / 测试直接调用） —————————————————————

    /// `is_hot` 的反值（Java `Constants.YES` / `NO` 对照用，
    /// 非 Java 概念，仅作常量自检）。
    #[must_use]
    pub const fn negate_flag(v: i32) -> i32 {
        if v == YES {
            NO
        } else {
            YES
        }
    }

    /// 某地区实体是否来自「跨境版可见」的层级
    /// （`is_overseas == 1` ⇒ `level >= 0`；否则 `level > 0`）。
    ///
    /// 这是 Java `getAllRegionTree` 的 `ge` / `gt` 判据的纯函数形式，
    /// 便于单测覆盖 `level == 0` 与 `level == NULL` 两个边界。
    #[must_use]
    pub fn visible_in_tree(level: Option<i32>, is_overseas: bool) -> bool {
        // SQL 三值逻辑：`level >= 0` 与 `level > 0` 在 level 为 NULL 时
        // 都得到 UNKNOWN ⇒ 该行被排除
        match level {
            None => false,
            Some(l) if is_overseas => l >= 0,
            Some(l) => l > 0,
        }
    }

    /// 地区名是否可进入「新地址选择」列表（对应 Java
    /// `RegionServiceImpl#filterSelectableRegions`）。
    ///
    /// ⚠️ **本切片的管理端 8 个接口都不调用它**（`filterSelectableRegions`
    /// 只被前台的 `getRegionByParentId` / `getRegionByIds` 使用，
    /// 属 system::region_client）。保留在此是为了让同一张表的规则集中可见，
    /// 且它是纯函数、可单测；system::region_client 有它自己的实现（独占文件约定，
    /// 两边不互相引用）。
    ///
    /// 语义：仅当 `parent_id == 500100`（重庆市）时生效，
    /// 从列表中剔除已废止的原江北区 `500105` / 原渝北区 `500112`；
    /// 但 `selected_region_id` 命中时仍回填（加载存量地址场景）。
    #[must_use]
    pub fn is_selectable_child(
        parent_id: i32,
        region_id: i32,
        selected_region_id: Option<i32>,
    ) -> bool {
        /// Java `RegionServiceImpl.CHONGQING_CITY_REGION_ID`。
        const CHONGQING_CITY_REGION_ID: i32 = 500100;
        /// Java `RegionServiceImpl.DEPRECATED_CHONGQING_DISTRICT_IDS`。
        const DEPRECATED: [i32; 2] = [500105, 500112];

        if parent_id != CHONGQING_CITY_REGION_ID {
            return true;
        }
        !DEPRECATED.contains(&region_id) || selected_region_id == Some(region_id)
    }
}

// `RegionTreeNode` 定义在 `nextrade-core`，`Region` 亦然；两者对本 crate 都是
// **外部类型**，故不能在此 `impl From<&Region> for RegionTreeNode`
// （E0117：orphan rule）。改用本 crate 内的自由函数，语义完全一致。
fn node_from_region(r: &Region) -> RegionTreeNode {
    RegionTreeNode {
        region_id: r.region_id,
        parent_id: r.parent_id,
        region_name: r.region_name.clone(),
        children: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::system::region::{
        map_order_column, pinyin_first_letter, KEY_COLUMN, MSG_FIRST_WORD_MAX_SIZE,
        MSG_PARAM_ERROR, TREE_MAX_DEEP,
    };

    // ————————————————————— 常量与 Java 对照 —————————————————————

    /// 树根父级 id 是 Java `TreeUtil.build(regionList, 0, ...)` 的字面量 `0`。
    #[test]
    fn tree_root_parent_id_is_literal_zero() {
        assert_eq!(TREE_ROOT_PARENT_ID, 0);
    }

    /// 版本开关默认取全功能部署（跨境/批发/O2O/供应商全开，is_overseas=1；
    /// Java `application.yaml` 的 `is-overseas` 在本部署形态下为 1）。
    #[test]
    fn default_edition_is_full_featured() {
        assert!(default_is_overseas());
        assert_eq!(EditionFlags::default().is_overseas, 1);
    }

    /// Java 文案逐字符照抄。
    #[test]
    fn java_messages_are_verbatim() {
        assert_eq!(MSG_REGION_NOT_FOUND, "地区不存在");
        assert_eq!(MSG_PARENT_IS_SELF, "上级地区不能选择自身");
        assert_eq!(MSG_PARENT_NOT_FOUND, "上级地区不存在");
        assert_eq!(MSG_PARENT_ID_REQUIRED, "上级地区不能为空");
        assert_eq!(MSG_REGION_ID_REQUIRED, "地区ID不能为空");
        assert_eq!(MSG_REGION_NAME_REQUIRED, "地区名称不能为空");
        assert_eq!(MSG_REGION_NAME_MAX_SIZE, "地区名称最多200个字符");
        assert_eq!(MSG_FIRST_WORD_MAX_SIZE, "首字母最多1个字符");
        assert_eq!(MSG_PARAM_ERROR, "参数错误");
        assert_eq!(DEFAULT_SORT_ORDER, 50); // updateRegion 的 `== null ? 50 :`
        assert_eq!(TREE_MAX_DEEP, 8); // TreeNodeConfig#setDeep(8)
        assert_eq!(YES, 1);
        assert_eq!(NO, 0);
    }

    /// 常量自检：`negate_flag` 只用于常量对照。
    #[test]
    fn negate_flag_round_trip() {
        assert_eq!(RegionService::negate_flag(YES), NO);
        assert_eq!(RegionService::negate_flag(NO), YES);
        assert_eq!(RegionService::negate_flag(7), YES);
    }

    // ————————————————————— 纯逻辑：树可见性 —————————————————————

    /// `getAllRegionTree` 的 `level >= 0` / `level > 0`，含 NULL 边界。
    #[test]
    fn tree_visibility_matches_sql_three_valued_logic() {
        // 国内版（默认）：level > 0
        assert!(!RegionService::visible_in_tree(Some(0), false));
        assert!(RegionService::visible_in_tree(Some(1), false));
        assert!(RegionService::visible_in_tree(Some(2), false));
        // 跨境版：level >= 0
        assert!(RegionService::visible_in_tree(Some(0), true));
        assert!(RegionService::visible_in_tree(Some(1), true));
        // level 为 NULL：两种比较都得 UNKNOWN ⇒ 排除（与 Java SQL 一致）
        assert!(!RegionService::visible_in_tree(None, false));
        assert!(!RegionService::visible_in_tree(None, true));
    }

    /// 可见性过滤 + 树构建的组合结果（模拟 `getAllRegionTree` 全链路）。
    #[test]
    fn tree_pipeline_filters_then_roots_at_zero() {
        let rows = vec![
            // 国内版：level 0 的国家被过滤掉
            Region {
                region_id: 1,
                level: Some(0),
                region_name: "中国".into(),
                parent_id: 0,
                ..Region::default()
            },
            Region {
                region_id: 110000,
                level: Some(2),
                region_name: "北京市".into(),
                parent_id: 1, // 父级 1 被过滤 ⇒ 成孤儿
                ..Region::default()
            },
            Region {
                region_id: 500000,
                level: Some(2),
                region_name: "重庆市".into(),
                parent_id: 0, // 直接以 0 为父 ⇒ 成根
                ..Region::default()
            },
        ];
        let kept: Vec<RegionTreeNode> = rows
            .iter()
            .filter(|r| RegionService::visible_in_tree(r.level, false))
            .map(node_from_region)
            .collect();
        assert_eq!(kept.len(), 2);
        let tree = build_region_tree(kept, TREE_ROOT_PARENT_ID);
        // 只有「重庆市」能成为根；「北京市」的父级被过滤 ⇒ 孤儿丢弃
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].region_name, "重庆市");
    }

    // ————————————————————— 纯逻辑：updateField 白名单 —————————————————————

    /// `updateField` 的白名单流转：白名单内四种写法都通过，
    /// `parent_id` / `level` / `region_id` 被拒。
    #[test]
    fn update_field_whitelist_flow() {
        for (input, column) in [
            ("region_name", "region_name"),
            ("regionName", "region_name"),
            ("sort_order", "sort_order"),
            ("sortOrder", "sort_order"),
            ("is_hot", "is_hot"),
            ("isHot", "is_hot"),
            ("first_word", "first_word"),
            ("firstWord", "first_word"),
        ] {
            let c = map_update_field_column(input).unwrap();
            assert_eq!(c, column);
            assert!(REGION_FIELDS.contains(&c));
            // 白名单四项都是真实列
            assert!(REGION_COLUMNS.contains(&c));
        }
        // 能归一但不在白名单 ⇒ Java 报「字段值不合法」
        for outside in ["region_id", "regionId", "parent_id", "parentId", "level"] {
            let c = map_update_field_column(outside).unwrap();
            assert!(!REGION_FIELDS.contains(&c), "{outside} 不应在白名单里");
        }
        // 完全无法识别 ⇒ 归一失败
        assert!(map_update_field_column("something").is_none());
        assert!(map_update_field_column("region_name; DROP TABLE region").is_none());
    }

    // ————————————————————— 纯逻辑：update 的 level / firstWord —————————————————————

    /// `update` 的 level 推导，含两条错误分支。
    #[test]
    fn update_level_rules() {
        assert_eq!(resolve_level(0, None).unwrap(), 1);
        assert_eq!(resolve_level(110000, Some(Some(2))).unwrap(), 3);
        assert_eq!(
            resolve_level(1, Some(None)).unwrap_err(),
            MSG_PARENT_NOT_FOUND
        );
        assert_eq!(resolve_level(1, None).unwrap_err(), MSG_PARENT_NOT_FOUND);
    }

    /// `update` 的 firstWord 推导（含与 Java 的拼音差异）。
    #[test]
    fn update_first_word_rules() {
        // 传了首字母 ⇒ 首字符大写
        assert_eq!(normalize_first_word(Some("beijing"), "北京市"), "B");
        assert_eq!(normalize_first_word(Some(" b "), "北京市"), "B");
        // 空白 ⇒ 退回名称推导；ASCII 名称可用
        assert_eq!(normalize_first_word(Some("  "), "Shanghai"), "S");
        assert_eq!(normalize_first_word(None, "Shanghai"), "S");
        // 中文名：Java 用 PinyinUtil 得 "B"，Rust 无拼音库 ⇒ 空串（已知差异）
        assert_eq!(pinyin_first_letter("北京市"), "");
        assert_eq!(normalize_first_word(None, "北京市"), "");
    }

    /// `isHot` / `sortOrder` 为 null 时的默认值（Java `== null ? 0 / 50 :`）。
    #[test]
    fn update_nullable_defaults() {
        let is_hot = None::<i32>.unwrap_or(NO);
        let sort_order = None::<i32>.unwrap_or(DEFAULT_SORT_ORDER);
        assert_eq!(is_hot, 0);
        assert_eq!(sort_order, 50);
        // 传了 0 就是 0（不是「空则默认」）
        assert_eq!(Some(0).unwrap_or(NO), 0);
        assert_eq!(Some(0).unwrap_or(DEFAULT_SORT_ORDER), 0);
    }

    // ————————————————————— 纯逻辑：list / batch —————————————————————

    /// `list` 的排序两分支（`sortField` 空 ⇒ 双列升序；非空 ⇒ 单列）。
    #[test]
    fn list_order_two_shapes() {
        let q = RegionListQuery::default();
        assert_eq!(q.order_clause(), " ORDER BY sort_order ASC, region_id ASC");

        let q = RegionListQuery {
            sort_field: Some("regionName".into()),
            sort_order: Some("desc".into()),
            ..Default::default()
        };
        assert_eq!(q.order_clause(), " ORDER BY region_name DESC");

        // 非法 sortOrder ⇒ 不追加 ORDER BY（Java else-if 缺口）
        let q = RegionListQuery {
            sort_field: Some("regionName".into()),
            sort_order: Some("nope".into()),
            ..Default::default()
        };
        assert_eq!(q.order_clause(), "");
        // 未识别 sortField ⇒ 回退主键列
        assert_eq!(map_order_column("bogus"), KEY_COLUMN);
    }

    /// `batch` 的 `type` 闸门：非 `del` 静默 `false`，空 ids 同样 `false`。
    #[test]
    fn batch_type_gate() {
        let dto: BatchDTO = serde_json::from_str(r#"{"ids":[1],"type":"show"}"#).unwrap();
        assert_ne!(dto.r#type, "del");
        let dto: BatchDTO = serde_json::from_str(r#"{"ids":[],"type":"del"}"#).unwrap();
        assert!(dto.ids.is_empty());
        let dto: BatchDTO = serde_json::from_str(r#"{"ids":[500105,500112],"type":"del"}"#).unwrap();
        assert_eq!(dto.ids, vec![500105, 500112]);
    }

    // ————————————————————— 纯逻辑：重庆已废止区 —————————————————————

    /// Java `filterSelectableRegions`：仅重庆（500100）下的
    /// 500105 / 500112 是已废止节点。
    ///
    /// ⚠️ 本切片的管理端接口**不调用**它，此测试只锁住纯函数语义
    /// （避免主会话误把它接进管理端 `list`）。
    #[test]
    fn deprecated_chongqing_districts_rules() {
        // 非重庆父级：一律可选
        assert!(RegionService::is_selectable_child(0, 500105, None));
        assert!(RegionService::is_selectable_child(500000, 500105, None));
        // 重庆（500100）下：两个已废止节点被剔除
        assert!(!RegionService::is_selectable_child(500100, 500105, None));
        assert!(!RegionService::is_selectable_child(500100, 500112, None));
        // 但加载存量地址（selected 命中）时仍回填
        assert!(RegionService::is_selectable_child(500100, 500105, Some(500105)));
        assert!(RegionService::is_selectable_child(500100, 500112, Some(500112)));
        // selected 是**别的**节点时不回填
        assert!(!RegionService::is_selectable_child(500100, 500105, Some(500112)));
        // 重庆下的其它区不受影响
        assert!(RegionService::is_selectable_child(500100, 500101, None));
    }

    // ————————————————————— 入参 DTO 语义 —————————————————————

    /// `update` 用 `id`（不是 `regionId`）定位记录，且 `id` 缺失时
    /// 报 Java 的「地区ID不能为空」。
    #[test]
    fn update_dto_uses_id_not_region_id() {
        let dto: RegionUpdateDTO = serde_json::from_str(
            r#"{"regionId":110000,"regionName":"北京市","parentId":0}"#,
        )
        .unwrap();
        assert_eq!(dto.id, None);
        assert_eq!(dto.as_create().region_id, Some(110000));
        assert_eq!(MSG_REGION_ID_REQUIRED, "地区ID不能为空");

        // 只有 id 才是定位键
        let dto: RegionUpdateDTO = serde_json::from_str(
            r#"{"id":110000,"regionId":999,"regionName":"北京市","parentId":0}"#,
        )
        .unwrap();
        assert_eq!(dto.id, Some(110000));
        assert_eq!(dto.as_create().region_id, Some(999));
    }

    /// `create` 必填字段缺失时的消息与 Java 注解一致。
    #[test]
    fn create_validation_messages() {
        // 名称缺失
        assert_eq!(
            validate_region_name(None).unwrap_err(),
            MSG_REGION_NAME_REQUIRED
        );
        // parentId 缺失
        assert_eq!(validate_parent_id(None).unwrap_err(), MSG_PARENT_ID_REQUIRED);
        // 首字母超 1 字符
        assert_eq!(
            validate_first_word(Some("BJ")).unwrap_err(),
            MSG_FIRST_WORD_MAX_SIZE
        );
        // 名称超 200 字符
        assert_eq!(
            validate_region_name(Some(&"名".repeat(201))).unwrap_err(),
            MSG_REGION_NAME_MAX_SIZE
        );
    }

    /// `create` 的最小可用入参（Java 只要求 name + parentId）。
    #[test]
    fn create_minimal_payload_is_valid() {
        let dto: RegionCreateDTO =
            serde_json::from_str(r#"{"regionName":"北京市","parentId":0}"#).unwrap();
        assert!(validate_region_name(dto.region_name.as_deref()).is_ok());
        assert!(validate_parent_id(dto.parent_id).is_ok());
        assert!(validate_first_word(dto.first_word.as_deref()).is_ok());
        // create 不推导 level / first_word / sort_order（Java 原样落库为 null）
        assert_eq!(dto.level, None);
        assert_eq!(dto.sort_order, None);
        assert_eq!(dto.first_word, None);
    }

    /// `del` 的 `id` 校验文案（Java `OperateDTO.id` 上 `@NotNull`）。
    #[test]
    fn del_requires_id() {
        let dto: OperateDTO = serde_json::from_str(r#"{"adminNote":"x"}"#).unwrap();
        assert_eq!(dto.id, None);
        assert_eq!(MSG_ID_REQUIRED, "id不能为空");
    }
}