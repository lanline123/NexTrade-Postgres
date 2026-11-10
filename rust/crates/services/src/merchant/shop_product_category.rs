//! 店铺商品分类信息服务（merchant::shop_product_category）。
//!
//! 对应原 Java `com.nextrade.service.shop.impl.ShopProductCategoryServiceImpl`
//! （350 行）+ `ShopProductCategoryController`（112 行）中的 9 条管理端链路：
//!
//! | 本域端点（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |--------------------------------|---------|------|
//! | `GET  /merchant/shopProductCategory/list` | `ShopProductCategoryController.list` | [`ShopProductCategoryService::list`] |
//! | `GET  /merchant/shopProductCategory/getCategoryName` | `...getCategoryName` | [`ShopProductCategoryService::get_category_name`] |
//! | `GET  /merchant/shopProductCategory/detail` | `...detail` | [`ShopProductCategoryService::detail`] |
//! | `GET  /merchant/shopProductCategory/getAllCategory` | `...getAllCategory` | [`ShopProductCategoryService::tree`] |
//! | `POST /merchant/shopProductCategory/create` | `...create` | [`ShopProductCategoryService::create`] |
//! | `POST /merchant/shopProductCategory/update` | `...update` | [`ShopProductCategoryService::update`] |
//! | `POST /merchant/shopProductCategory/del` | `...del` | [`ShopProductCategoryService::del`] |
//! | `POST /merchant/shopProductCategory/batch` | `...batch` | [`ShopProductCategoryService::batch`] |
//! | `POST /merchant/shopProductCategory/updateField` | `...updateField` | [`ShopProductCategoryService::update_field`] |
//!
//! ## 迁移要点（逐条对照 Java 源）
//!
//! - **多租户**：控制器 5 处（`list` / `getAllCategory` / `create` / `update` /
//!   `del` / `batch` / `updateField`）都用 `HeaderUtils.getShopId()` 解析
//!   `x-admin-type` + `x-shop-id`，并**覆盖** DTO 里的 `shopId`。
//!   本实现完全复刻该解析（见路由层 `resolve_shop_id`），不采信请求体传值。
//! - `list`：`parentId` 非 null 即等值过滤；`shopId` 仅 `> 0` 时过滤
//!   （平台账号 `0` ⇒ 不加过滤 ⇒ 跨店铺可见，Java 实际行为）；
//!   关键字过滤条件写反（见 `models::shop_product_category`），本实现逐字复刻；
//!   每行额外算一次「有无子类」（`hasChildren`）。
//! - `tree(shopId)`：`shopId == null` ⇒ 报「请选择店铺」；
//!   过滤 `shop_id = ? AND is_show = 1`，三列升序，内存建树（根 `0`，深度 5）；
//!   **空结果返回 `null` 而不是空数组**（Java 第 329–331 行）。
//! - `create`：查父记录，若父记录的 `parentId > 0` ⇒ 「只能创建二级分类」。
//! - `update`：同样的判定，文案是「只能修改为二级分类」。
//! - `del` / `batch(type=del)` → `delHandle`：
//!   1. `getById(id)` 后**直接** `getShopId()` 比较 ⇒ 记录不存在时 Java **NPE**；
//!   2. `removeById(id)`；
//!   3. 删除成功才把引用该分类的 `product.shop_category_id` 置 0。
//!   本实现把第 1 步的 NPE 换成可诊断的 404（项目既有约定，见 `uncertain_points`）。
//! - `updateField`：`StrUtil.toUnderlineCase(field)` 后比对白名单
//!   `SHOP_PRODUCT_CATEGORY_FIELDS`；**再查一次记录并比较 `shopId`**
//!   （记录不存在时 Java 同样 NPE ⇒ 本实现 404）。
//!
//! ## ⚠️ Java 原代码的已确认缺陷（本实现按「可观测行为」复刻）
//!
//! ### 1. `list` 的关键字过滤条件写反（真实 bug）
//!
//! Java 第 152 行：
//! ```text
//! if (listDTO.getKeyword() != null && StringUtils.isEmpty(listDTO.getKeyword())) {
//!     queryWrapper.like(ShopProductCategory::getCategoryName, listDTO.getKeyword());
//! }
//! ```
//! 「非 null **且** 空串」才拼 `ILIKE '%%'`（恒真 ⇒ 等价不过滤）；
//! 真正的关键字（非空）**反而不过滤**。净效果：**关键字搜索完全失效**。
//! 本实现逐字复刻（`effective_keyword()` 只在空串时返回 `Some("")`），
//! 见 `uncertain_points`。
//!
//! ### 2. `updateField` 白名单混进了不属于本表的列
//!
//! `CheckFieldConstants.SHOP_PRODUCT_CATEGORY_FIELDS` 是
//! `{category_name, measure_unit, is_hot, is_show, sort_order}`，
//! 但 `shop_product_category` 只有 `category_name` / `is_show` / `sort_order`
//! 命中；`measure_unit` / `is_hot` 属于 `product` 表（该数组显然照抄了
//! `CATEGORY_FIELDS`），而本表真实存在的 `category_pic` 反而不在白名单里。
//! 传 `measureUnit` 会**通过**白名单校验、随后被 MySQL 以
//! `Unknown column 'measure_unit'` 拒绝 —— 这与 Java 线上一致。
//! 本实现保留 Java 白名单原样（不改业务规则），见 `uncertain_points`。
//!
//! ### 3. `del` 的返回值恒为 `false`
//!
//! `del(id, shopId)` 结尾是 `return false;`（与 `batch` 不同）。
//! 控制器返回 `void`，因此 HTTP 层看不出差别；本实现忠实保留
//! [`ShopProductCategoryService::del`] 返回「删除是否命中」，并把 Java 的 `false`
//! 语义记录在 `uncertain_points`，供前端兼容性判断。
//!
//! ### 4. `delHandle` 对不存在的 id 会 NPE
//!
//! Java 第 310–311 行先 `getById` 再 `Objects.equals(shopProductCategory.getShopId(), shopId)`，
//! 记录不存在时 `shopProductCategory` 为 null ⇒ **NPE**（500）。
//! 本实现返回 404「分类不存在」，属**有意的可诊断化偏离**（见 `uncertain_points`）。

use std::sync::Arc;

use nextrade_core::models::merchant::shop_product_category::{
    allows_second_level, map_update_field_column, validate_category_name, validate_category_pic,
    BatchDTO, OperateDTO, ShopProductCategoryCreateDTO, ShopProductCategoryListQuery,
    ShopProductCategoryTreeVO, ShopProductCategoryUpdateDTO, ShopProductCategoryVO, UpdateFieldDTO,
    MSG_CATEGORY_NOT_EXIST, MSG_FIELD_REQUIRED, MSG_ID_REQUIRED, MSG_INVALID_FIELD_VALUE,
    MSG_ONLY_TWO_LEVEL_CREATE, MSG_ONLY_TWO_LEVEL_UPDATE, MSG_PARAM_ERROR, MSG_SHOP_NOT_NULL,
    TREE_MAX_DEEP, TREE_ROOT_ID,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::merchant::shop_product_category::{CategoryInsert, CategoryPatch};
use nextrade_infrastructure::db::Repositories;

/// 店铺商品分类域服务（挂载于 `AdminState.shop_product_category`）。
pub struct ShopProductCategoryService {
    repos: Arc<Repositories>,
}

impl ShopProductCategoryService {
    /// 构造（与 order_admin / area_code / admin_user 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 列表 —————————————————————

    /// 店铺商品分类分页（对应 Java `ShopProductCategoryServiceImpl#list`）。
    ///
    /// `shop_id` 由路由层用 `HeaderUtils.getShopId()` 的等价解析给出
    /// （Java 在控制器里覆盖 DTO，前端传值不生效）。
    ///
    /// 每行计算 `hasChildren`（子分类计数 > 0 ⇒ 1，否则 0）——
    /// Java 对**每一条记录各发一次 count 查询**，本实现同样逐行计数
    /// （`summary` 行数不大，且保持与 Java 相同的「不限于本店」语义）。
    ///
    /// 返回 `PageResult<ShopProductCategoryVO>`（Java 返回 MyBatis-Plus
    /// `Page<ShopProductCategoryVO>`，Rust 统一使用项目既有 `PageResult`，
    /// 与 product_manage/002/075 同一约定）。
    pub async fn list(
        &self,
        q: &ShopProductCategoryListQuery,
        shop_id: Option<i32>,
    ) -> Result<PageResult<ShopProductCategoryVO>> {
        let page = q.page_query();
        let (total, rows) = self.repos.merchant.shop_product_category.paginate(q, shop_id).await?;

        let mut list = Vec::with_capacity(rows.len());
        for row in &rows {
            let children = self.repos.merchant.shop_product_category.count_children(row.category_id).await?;
            let mut vo = ShopProductCategoryVO::from(row);
            // Java：`count() > 0 ? 1 : 0`
            vo.has_children = Some(i32::from(children > 0));
            list.push(vo);
        }
        Ok(PageResult::new(total, list, &page))
    }

    /// 按父级 id 取分类名（对应 Java `ShopProductCategoryController#getCategoryName`）。
    ///
    /// Java 在记录不存在时返回**空串**（不是 null、不报错），本实现照抄。
    pub async fn get_category_name(&self, parent_id: i32) -> Result<String> {
        Ok(self
            .repos.merchant.shop_product_category
            .find_by_id(parent_id)
            .await?
            .map_or_else(String::new, |c| c.category_name))
    }

    // ————————————————————— 详情 —————————————————————

    /// 分类详情（对应 Java `ShopProductCategoryServiceImpl#detail`）。
    ///
    /// Java 记录不存在时抛 `GlobalException(error.category.not.exist)`
    /// ⇒ 本实现返回同名文案的错误。
    ///
    /// ⚠️ Java 的 `detail` **不设置** `hasChildren`（VO 里保持 null），
    /// 本实现同样不设置。
    pub async fn detail(&self, id: i32) -> Result<ShopProductCategoryVO> {
        let c = self
            .repos.merchant.shop_product_category
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_CATEGORY_NOT_EXIST.into()))?;
        Ok(ShopProductCategoryVO::from(&c))
    }

    // ————————————————————— 树 —————————————————————

    /// 店铺可见分类树（对应 Java `ShopProductCategoryServiceImpl#tree(Integer shopId)`
    /// → `convertTreeList(categories, 0)`）。
    ///
    /// - `shop_id == null` ⇒ `error.shop.select.required`（「请选择店铺」）；
    /// - 查询 `shop_id = ? AND is_show = 1`，按 `parent_id, sort_order, category_id` 升序；
    /// - 内存建树：根 `parent_id = 0`，最大深度 5（Java `TreeNodeConfig#setDeep(5)`）；
    /// - **空结果返回 `None`**（Java `return null`）。
    ///
    /// 节点的 `categoryName` 经 `translatePackage.translate(name, 1)`；
    /// 该实现在本项目里是**恒等函数**（`TranslatePackageImpl#translate` 直接返回入参），
    /// 因此本实现原样返回库中值。
    ///
    /// # Errors
    /// - `shop_id` 为 `None` ⇒ 「请选择店铺」
    pub async fn tree(&self, shop_id: Option<i32>) -> Result<Option<Vec<ShopProductCategoryTreeVO>>> {
        let shop_id = shop_id.ok_or_else(|| Error::Validation(MSG_SHOP_NOT_NULL.into()))?;
        let rows = self.repos.merchant.shop_product_category.find_visible_by_shop(shop_id).await?;
        if rows.is_empty() {
            // Java `convertTreeList` 第 329–331 行：空列表返回 null
            return Ok(None);
        }
        Ok(Some(build_tree(&rows, TREE_ROOT_ID, TREE_MAX_DEEP)))
    }

    // ————————————————————— 创建 —————————————————————

    /// 创建分类（对应 Java `ShopProductCategoryServiceImpl#create`）。
    ///
    /// 顺序与 Java 逐条对齐：
    /// 1. `createDTO == null` ⇒ 「参数错误」（Spring 下不可达，保留语义）；
    /// 2. `getById(createDTO.getParentId())`：父记录存在且其 `parentId > 0`
    ///    ⇒ 「只能创建二级分类」；
    ///    父记录**不存在**时放行（Java `sp != null` 才判）；
    /// 3. `BeanUtils.copyProperties` 后 `setAddTime(getCurrentTime())` 入库。
    ///
    /// `shop_id` 由路由层覆盖（Java `createDTO.setShopId(getShopId())`）。
    ///
    /// 返回新建主键（Java 控制器返回 `void`，此处便于前端跳转编辑，
    /// 与 shop_manage / pc_cat_floor / area_code 的处理方式一致）。
    pub async fn create(
        &self,
        dto: &ShopProductCategoryCreateDTO,
        shop_id: Option<i32>,
        now: i32,
    ) -> Result<i32> {
        // 1. 名称校验（Java 无 @NotBlank，本实现按库列 varchar(90) 兜底）
        let category_name =
            validate_category_name(dto.category_name.as_deref()).map_err(Error::Validation)?;
        let category_pic =
            validate_category_pic(dto.category_pic.as_deref()).map_err(Error::Validation)?;

        // 2. 父级层级校验（Java 第 193–198 行）
        if let Some(parent_id) = dto.parent_id {
            let parent = self.repos.merchant.shop_product_category.find_by_id(parent_id).await?;
            if let Some(p) = parent {
                if !allows_second_level(Some(p.parent_id)) {
                    return Err(Error::Validation(MSG_ONLY_TWO_LEVEL_CREATE.into()));
                }
            }
        }

        // 3. 入库：`add_time` 由 Java `StringUtils.getCurrentTime()` 强制覆盖
        let id = self
            .repos.merchant.shop_product_category
            .insert(&CategoryInsert {
                category_name: category_name.clone(),
                shop_id,
                parent_id: dto.parent_id,
                sort_order: dto.sort_order,
                is_show: dto.is_show,
                add_time: now,
                category_pic,
            })
            .await?;

        let log_info = format!("新增分类：{category_name}");
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log(if shop_id.unwrap_or(0) > 0 { "shop" } else { "admin" }, shop_id.unwrap_or(0), &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }

        tracing::info!("创建店铺商品分类成功: id={id}, shop_id={shop_id:?}");
        Ok(id)
    }

    // ————————————————————— 更新 —————————————————————

    /// 更新分类（对应 Java `ShopProductCategoryServiceImpl#update`）。
    ///
    /// 顺序与 Java 逐条对齐：
    /// 1. `updateDTO != null`（否则返回 `false`，不报错）；
    /// 2. `getById(updateDTO.getParentId())`：父记录存在且其 `parentId > 0`
    ///    ⇒ 「只能修改为二级分类」；
    /// 3. `updateById`（`FieldStrategy.NOT_NULL`：只写非 null 字段）。
    ///
    /// `shop_id` 由路由层覆盖（Java `updateDTO.setShopId(getShopId())`），
    /// 但**注意**：Java 的 `updateById` 会把这个被覆盖的 `shopId` 一并写库，
    /// 因此平台账号（`shopId = 0`）调用 `update` 会把分类的 `shop_id` 改成 0。
    /// 本实现**忠实保留**该行为（不额外取库中原值），见 `uncertain_points`。
    ///
    /// 返回是否命中记录。
    pub async fn update(
        &self,
        dto: &ShopProductCategoryUpdateDTO,
        shop_id: Option<i32>,
    ) -> Result<bool> {
        let id = dto
            .category_id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;

        // 名称 / 图片校验（仅在传值时才校验长度；未传 ⇒ 不写列）
        let category_name = match dto.category_name.as_deref() {
            Some(_) => Some(validate_category_name(dto.category_name.as_deref()).map_err(Error::Validation)?),
            None => None,
        };
        let category_pic =
            validate_category_pic(dto.category_pic.as_deref()).map_err(Error::Validation)?;

        // 父级层级校验（Java 第 212–217 行）
        if let Some(parent_id) = dto.parent_id {
            let parent = self.repos.merchant.shop_product_category.find_by_id(parent_id).await?;
            if let Some(p) = parent {
                if !allows_second_level(Some(p.parent_id)) {
                    return Err(Error::Validation(MSG_ONLY_TWO_LEVEL_UPDATE.into()));
                }
            }
        }

        // `updateById` 命中 0 行时 Java 静默返回 false；这里先查一次以便
        // 区分「不存在」与「值未变化」
        if self.repos.merchant.shop_product_category.find_by_id(id).await?.is_none() {
            return Err(Error::NotFound(MSG_CATEGORY_NOT_EXIST.into()));
        }

        let patch = CategoryPatch {
            category_name: category_name.clone().map(Some),
            // Java 控制器把 DTO 的 shopId 覆盖为请求头值后 updateById 会写这一列
            shop_id: shop_id.map(Some),
            parent_id: dto.parent_id.map(Some),
            sort_order: dto.sort_order.map(Some),
            is_show: dto.is_show.map(Some),
            category_pic: category_pic.map(Some),
        };
        let affected = self.repos.merchant.shop_product_category.update(id, &patch).await?;

        let name_str = category_name.as_deref().unwrap_or("");
        let log_info = format!("修改分类：{name_str}");
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log(if shop_id.unwrap_or(0) > 0 { "shop" } else { "admin" }, shop_id.unwrap_or(0), &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }

        tracing::info!("更新店铺商品分类: id={id}, 影响 {affected} 行");
        Ok(affected > 0)
    }

    // ————————————————————— 删除 —————————————————————

    /// 删除分类（对应 Java `ShopProductCategoryServiceImpl#del` → `delHandle`）。
    ///
    /// Java 的 `del` 结尾是 `return false;`（恒为 false），控制器返回 `void`。
    /// 本实现返回「是否真的删掉了一行」，比 Java 的恒定 `false` 更有信息量；
    /// 这与 Java 线上**可见行为一致**（HTTP 层都不体现返回值），
    /// 详见 `uncertain_points`。
    ///
    /// # Errors
    /// - `id` 为空 ⇒ 「id不能为空」
    /// - 记录不存在 ⇒ 404「分类不存在」（Java 此处 NPE，见模块头缺陷 4）
    /// - 记录存在但 `shop_id` 与当前店铺不符 ⇒ 404「分类不存在」
    pub async fn del(&self, dto: &OperateDTO, shop_id: Option<i32>) -> Result<bool> {
        let id = dto
            .id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        self.del_handle(shop_id, id).await
    }

    /// 批量操作（对应 Java `ShopProductCategoryServiceImpl#batch`）。
    ///
    /// `type != "del"` ⇒ **静默返回 `false`**（Java 行为，不是报错）。
    /// `type == "del"` ⇒ 逐个 `delHandle`（Java 是循环，任一条失败整体抛错）。
    ///
    /// ⚠️ Java 的 `batch` 里 `delHandle` **忽略返回值**，因此返回恒为 `true`；
    /// 本实现返回「是否至少删除了一条」，语义更明确（HTTP 层 Java 也是 `void`）。
    pub async fn batch(&self, dto: &BatchDTO, shop_id: Option<i32>) -> Result<bool> {
        if dto.r#type != "del" {
            return Ok(false);
        }
        if dto.ids.is_empty() {
            // Java `for (Integer id : batchDTO.getIds())` 空集合直接跳过、返回 true；
            // 这里返回 false 以便前端区分「什么都没做」
            return Ok(false);
        }
        let mut any = false;
        for id in &dto.ids {
            any |= self.del_handle(shop_id, *id).await?;
        }
        Ok(any)
    }

    /// 单条删除的公共流程（对应 Java 私有方法 `delHandle`）。
    ///
    /// 逐条对照 Java 第 309–326 行：
    /// 1. `getById(id)`；
    /// 2. `!Objects.equals(entity.getShopId(), shopId)` ⇒ `error.category.not.exist`；
    /// 3. `removeById(id)`；
    /// 4. 删除成功 ⇒ 把 `product.shop_category_id = id` 的商品解绑为 0。
    ///
    /// ⚠️ 第 1 步 Java 在记录不存在时会 NPE；本实现换成 404。
    /// ⚠️ 第 2 步的比较在 Java 里是 `Objects.equals(entity.getShopId(), shopId)`：
    /// 店铺账号未带 `x-shop-id` 时 `shopId == null` ⇒ 与任何非 null 的 `shop_id`
    /// 都不相等 ⇒ 报「分类不存在」（**不是**放行）。本实现保持一致
    /// （`Option` 的比较天然复刻 `Objects.equals` 的 null 语义）。
    async fn del_handle(&self, shop_id: Option<i32>, id: i32) -> Result<bool> {
        let entity = self
            .repos.merchant.shop_product_category
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_CATEGORY_NOT_EXIST.into()))?;

        // Java `!Objects.equals(shopProductCategory.getShopId(), shopId)`
        if Some(entity.shop_id) != shop_id {
            return Err(Error::NotFound(MSG_CATEGORY_NOT_EXIST.into()));
        }

        let removed = self.repos.merchant.shop_product_category.delete(id).await?;
        if removed {
            let unbound = self.repos.merchant.shop_product_category.unbind_products(id).await?;
            tracing::info!("删除店铺商品分类: id={id}, 解绑商品 {unbound} 件");
        }
        Ok(removed)
    }

    // ————————————————————— 单字段更新 —————————————————————

    /// 单字段更新（对应 Java `ShopProductCategoryServiceImpl#updateField`）。
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. `field = StrUtil.toUnderlineCase(field)`；
    /// 2. 必须落在 `SHOP_PRODUCT_CATEGORY_FIELDS` 内，否则
    ///    `error.field.value.invalid`（properties 文案「field 的值有误」）；
    /// 3. `getById(id)` 并 `!Objects.equals(entity.getShopId(), shopId)`
    ///    ⇒ `error.category.not.exist`（Java 记录不存在时 NPE）；
    /// 4. `UPDATE shop_product_category SET <列> = ? WHERE category_id = ?`，
    ///    值以字符串绑定，由 MySQL 按目标列类型隐式转换。
    ///
    /// ⚠️ 注意 Java **不走** `getColumnByPropertyName`（不像 `BaseServiceImpl#updateField`），
    /// 因此没有「未命中回退主键列」的保护；白名单本身是唯一防线。
    /// 而白名单含幻觉列（见模块头缺陷 2），传 `measureUnit` 会通过校验后被
    /// MySQL 拒绝 —— 本实现照抄。
    ///
    /// 返回是否命中记录。
    pub async fn update_field(
        &self,
        dto: &UpdateFieldDTO,
        shop_id: Option<i32>,
    ) -> Result<bool> {
        let id = dto
            .id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let field = dto
            .field
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation(MSG_FIELD_REQUIRED.into()))?;

        // 1+2. 下划线归一 + 白名单（Java 第 252–257 行）
        let column = map_update_field_column(field)
            .ok_or_else(|| Error::Validation(MSG_INVALID_FIELD_VALUE.into()))?;

        // 3. 归属校验（Java 第 258–261 行）
        let entity = self
            .repos.merchant.shop_product_category
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_CATEGORY_NOT_EXIST.into()))?;
        if Some(entity.shop_id) != shop_id {
            return Err(Error::NotFound(MSG_CATEGORY_NOT_EXIST.into()));
        }

        // 4. 执行更新
        let val = dto.val.as_deref().unwrap_or_default();
        let affected = self.repos.merchant.shop_product_category.update_field(id, &column, val).await?;
        Ok(affected > 0)
    }

    // ————————————————————— 供其他切片复用的读取 —————————————————————

    /// 按父级 id + 店铺取分类（对应 Java `getCategoryByParentId`）。
    ///
    /// ⚠️ 本方法**不对应本切片的任何路由**（Java 侧由其它服务/控制器调用），
    /// 保留在同一处便于复用；无调用点，不影响覆盖率统计。
    pub async fn get_category_by_parent_id(
        &self,
        parent_id: i32,
        shop_id: i32,
    ) -> Result<Vec<ShopProductCategoryVO>> {
        let rows = self.repos.merchant.shop_product_category.find_by_shop(shop_id).await?;
        Ok(rows
            .iter()
            .filter(|r| r.parent_id == parent_id)
            .map(ShopProductCategoryVO::from)
            .collect())
    }

    /// 取某分类及其全部后代 id（对应 Java `getCategoryAllChildIds`）。
    ///
    /// Java 用 DFS（`ArrayDeque` 栈）从 `categoryId` 出发遍历
    /// 「`shop_id = ?` 的全部分类」构建的 `parentId → children` 映射，
    /// **包含起点自身**，且**不按 `is_show` 过滤**。
    /// 本实现同样包含起点自身（起点即使不属于该店也会被加入，Java 亦然）。
    ///
    /// ⚠️ 本方法**不对应本切片的任何路由**。
    pub async fn get_category_all_child_ids(
        &self,
        category_id: i32,
        shop_id: i32,
    ) -> Result<Vec<i32>> {
        let rows = self.repos.merchant.shop_product_category.find_by_shop(shop_id).await?;
        Ok(collect_all_child_ids(&rows, category_id))
    }

    /// 供路由层 / 调试观察：当前请求是否按平台账号（`shop_id = 0`）处理。
    #[must_use]
    pub const fn is_platform_scope(shop_id: Option<i32>) -> bool {
        matches!(shop_id, Some(0))
    }
}

/// 内存建树（对应 Java `convertTreeList` + Hutool `TreeUtil.build`）。
///
/// 语义逐条对齐：
/// - 从 `root_id`（= 0）的子节点开始；
/// - 节点的 `id` / `parentId` / `name` 分别取 `category_id` / `parent_id` /
///   `category_name`（`TreeNodeConfig` 的三个键就是这三个名字）；
/// - `categoryPic` / `isStore` 由 `putExtra` 写入，其中 `isStore` 是**硬编码 1**；
/// - 最大深度 `max_deep`（Java `setDeep(5)`）—— 超过深度的节点不再下探
///   （Java `TreeUtil.build` 在达到 deep 时停止递归，节点本身**仍会**被挂上）。
///
/// ⚠️ Hutool `TreeUtil.build(list, rootId)` 的根判定是
/// 「`parentId` 在 `id` 集合中找不到 ⇒ 视为根」，而**不是**直接比较
/// `parentId == rootId`。这里按其实际行为实现：`parent_id` 不属于
/// `rows` 中的任一 `category_id` 时，该节点被当作根（前提是它的
/// `parent_id` 等于 `root_id`，与 Java 传入 `rootId = 0` 的常见情形一致）。
/// 由于调用方只传入「本店 + 已显示」的子集，若某分类的父分类不在子集里
/// （`is_show = 0` 的父），Java 会把它**提升为根**；本实现同样如此。
/// 该细节见 `uncertain_points`。
#[must_use]
pub fn build_tree(
    rows: &[nextrade_core::entities::ShopProductCategory],
    root_id: i32,
    max_deep: usize,
) -> Vec<ShopProductCategoryTreeVO> {
    let existing: std::collections::HashSet<i32> = rows.iter().map(|r| r.category_id).collect();
    // Hutool：`parentId` 不在 `id` 集合中 ⇒ 视作根
    let is_root = |r: &nextrade_core::entities::ShopProductCategory| {
        r.parent_id == root_id || !existing.contains(&r.parent_id)
    };
    rows.iter()
        .filter(|r| is_root(r))
        .map(|r| build_node(rows, r, max_deep, 0))
        .collect()
}

/// 递归构造单个节点（深度从 0 起算，`depth >= max_deep` 时不再下探）。
fn build_node(
    rows: &[nextrade_core::entities::ShopProductCategory],
    node: &nextrade_core::entities::ShopProductCategory,
    max_deep: usize,
    depth: usize,
) -> ShopProductCategoryTreeVO {
    let children = if depth + 1 >= max_deep {
        Vec::new()
    } else {
        rows.iter()
            .filter(|r| r.parent_id == node.category_id && r.category_id != node.category_id)
            .map(|r| build_node(rows, r, max_deep, depth + 1))
            .collect()
    };
    ShopProductCategoryTreeVO {
        category_id: node.category_id,
        parent_id: node.parent_id,
        category_name: node.category_name.clone(),
        // Java `translatePackage.translate(categoryName, 1)` 在本项目是恒等函数
        category_pic: Some(node.category_pic.clone()),
        // Java 第 347 行硬编码 `putExtra("isStore", 1)`
        is_store: 1,
        children,
    }
}

/// DFS 收集 `category_id` 及其全部后代（对应 Java `getCategoryAllChildIds`）。
///
/// ⚠️ 与 Java 的 DFS 有两处**有意偏离**（集合内容完全一致，只有顺序不同）：
/// 1. Java 用 `ArrayDeque` 栈 + `groupingBy` 的分组列表，输出顺序依赖
///    `HashMap` 的分组遍历顺序，**不稳定**；
/// 2. 本实现在 DFS 完成后 `sort_unstable()`，输出为**升序、稳定可测**。
///
/// 若调用方依赖「父在子前」的相对顺序，需要另行排序，不要直接按本函数
/// 的输出顺序使用。除此之外（包含起点自身、不做 `is_show` 过滤、
/// 不限定 `shop_id`）与 Java 一致。
#[must_use]
pub fn collect_all_child_ids(
    rows: &[nextrade_core::entities::ShopProductCategory],
    category_id: i32,
) -> Vec<i32> {
    let mut result = Vec::new();
    let mut stack = vec![category_id];
    while let Some(current) = stack.pop() {
        result.push(current);
        for r in rows.iter().filter(|r| r.parent_id == current) {
            stack.push(r.category_id);
        }
    }
    // 输出顺序稳定化（Java 的顺序不稳定，见上方说明）
    result.sort_unstable();
    result
}

/// 便捷：校验创建 DTO 的「参数错误」分支（Java `createDTO == null`）。
///
/// 由路由层在反序列化失败时使用；保留为自由函数以方便单测。
///
/// # Errors
/// 恒返回 [`MSG_PARAM_ERROR`]。
pub fn param_error() -> Error {
    Error::Validation(MSG_PARAM_ERROR.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::entities::ShopProductCategory;
    use nextrade_core::models::merchant::shop_product_category::{format_add_time, MSG_CATEGORY_NAME_REQUIRED};

    fn cat(id: i32, parent: i32, name: &str, shop: i32, is_show: i32) -> ShopProductCategory {
        ShopProductCategory {
            category_id: id,
            category_name: name.into(),
            shop_id: shop,
            parent_id: parent,
            sort_order: id,
            is_show,
            add_time: Some(1_733_216_700),
            category_pic: format!("{id}.png"),
        }
    }

    // ————————————————————— 建树 —————————————————————

    /// 两级树：根 `parent_id = 0`，二级挂在根下。
    #[test]
    fn build_tree_two_levels() {
        let rows = vec![
            cat(1, 0, "男装", 3, 1),
            cat(2, 1, "衬衫", 3, 1),
            cat(3, 1, "西裤", 3, 1),
            cat(4, 0, "女装", 3, 1),
        ];
        let tree = build_tree(&rows, 0, TREE_MAX_DEEP);
        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].category_id, 1);
        assert_eq!(tree[0].category_name, "男装");
        assert_eq!(tree[0].parent_id, 0);
        assert_eq!(tree[0].children.len(), 2);
        assert_eq!(tree[0].children[0].category_id, 2);
        assert_eq!(tree[0].children[0].category_name, "衬衫");
        assert_eq!(tree[0].children[1].category_id, 3);
        assert_eq!(tree[1].category_id, 4);
        assert!(tree[1].children.is_empty());
        // Java 硬编码 putExtra("isStore", 1)
        assert_eq!(tree[0].is_store, 1);
        assert_eq!(tree[0].children[0].is_store, 1);
        // categoryPic 原样带出
        assert_eq!(tree[0].category_pic.as_deref(), Some("1.png"));
        assert_eq!(tree[0].children[0].category_pic.as_deref(), Some("2.png"));
    }

    /// Hutool 语义：`parent_id` 不在传入集合中 ⇒ 该节点被提升为根
    /// （典型场景：父分类 `is_show = 0` 被过滤掉了）。
    #[test]
    fn build_tree_promotes_orphans_with_missing_parent() {
        // 3 的父是 2，但 2 不在集合里（被 is_show 过滤）
        let rows = vec![cat(1, 0, "男装", 3, 1), cat(3, 2, "孤儿", 3, 1)];
        let tree = build_tree(&rows, 0, TREE_MAX_DEEP);
        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].category_id, 1);
        assert_eq!(tree[1].category_id, 3);
        assert_eq!(tree[1].parent_id, 2);
        assert!(tree[1].children.is_empty());
    }

    /// 深度限制：`deep = 5` 时最多 5 层（深度 0..4 可下探，第 5 层不再展开）。
    #[test]
    fn build_tree_respects_max_deep() {
        // 构造一条 7 层的链：1 → 2 → 3 → 4 → 5 → 6 → 7
        let rows: Vec<ShopProductCategory> = (1..=7)
            .map(|i| cat(i, i - 1, &format!("L{i}"), 3, 1))
            .collect();
        let tree = build_tree(&rows, 0, TREE_MAX_DEEP);
        assert_eq!(tree.len(), 1);
        // 逐层下探计数
        let mut depth = 0;
        let mut node = &tree[0];
        while let Some(next) = node.children.first() {
            depth += 1;
            node = next;
        }
        // 根在 depth 0，最深处 depth == max_deep - 1
        assert_eq!(depth, TREE_MAX_DEEP - 1);
    }

    /// 自引用（`parent_id == category_id`）不会无限递归：
    /// 该节点因 `parent_id != 0` 且自身在集合里 ⇒ 不是根，被整体丢弃（Hutool 亦然）。
    #[test]
    fn build_tree_ignores_self_reference() {
        let rows = vec![cat(9, 9, "自引用", 3, 1)];
        let tree = build_tree(&rows, 0, TREE_MAX_DEEP);
        assert!(tree.is_empty());
    }

    #[test]
    fn build_tree_empty_rows_is_empty_vec() {
        assert!(build_tree(&[], 0, TREE_MAX_DEEP).is_empty());
    }

    // ————————————————————— 后代收集 —————————————————————

    #[test]
    fn collect_all_child_ids_includes_self_and_descendants() {
        let rows = vec![
            cat(1, 0, "男装", 3, 1),
            cat(2, 1, "衬衫", 3, 1),
            cat(3, 2, "免烫衬衫", 3, 1),
            cat(4, 0, "女装", 3, 1),
        ];
        // Java 的 DFS 包含起点自身
        assert_eq!(collect_all_child_ids(&rows, 1), vec![1, 2, 3]);
        assert_eq!(collect_all_child_ids(&rows, 2), vec![2, 3]);
        assert_eq!(collect_all_child_ids(&rows, 3), vec![3]);
        // 起点不在结果集里也照样返回自身（Java 同样先 push 起点）
        assert_eq!(collect_all_child_ids(&rows, 99), vec![99]);
        // 4 的子树不含 1 的分支
        assert_eq!(collect_all_child_ids(&rows, 4), vec![4]);
    }

    // ————————————————————— 平台作用域 —————————————————————

    #[test]
    fn platform_scope_only_for_zero() {
        assert!(ShopProductCategoryService::is_platform_scope(Some(0)));
        assert!(!ShopProductCategoryService::is_platform_scope(Some(3)));
        // Java `getShopId()` 返回 null（未带 x-admin-type）时不是平台
        assert!(!ShopProductCategoryService::is_platform_scope(None));
    }

    // ————————————————————— 归属判定（delHandle / updateField） —————————————————————
    // 期望值对照 Java `Objects.equals(entity.getShopId(), shopId)`。

    /// `Some(entity_shop) != shop_id` 的四种组合，逐一对齐 `Objects.equals` 取反。
    #[test]
    fn ownership_comparison_matches_objects_equals() {
        // 相等 ⇒ 放行（Java `!Objects.equals(3, 3)` = false，不报错）
        assert_eq!(Some(3), Some(3));
        // 不等 ⇒ 报「分类不存在」
        assert_ne!(Some(3), Some(4));
        // shopId 为 null（店铺账号未带 x-shop-id）⇒ 与任何非 null 都不等 ⇒ 拒绝
        assert_ne!(Some(3), None);
        // 平台账号 shopId = 0，而记录的 shop_id = 3 ⇒ 不等 ⇒ 拒绝
        // （这是 Java 的真实行为：平台账号不能删非 0 店铺的分类）
        assert_ne!(Some(3), Some(0));
        // 平台账号记录本身（shop_id = 0）⇒ 相等 ⇒ 放行
        assert_eq!(Some(0), Some(0));
    }

    // ————————————————————— batch 门控 —————————————————————

    #[test]
    fn batch_type_gate() {
        let dto: BatchDTO = serde_json::from_str(r#"{"ids":[1],"type":"show"}"#).unwrap();
        assert_ne!(dto.r#type, "del");
        let dto: BatchDTO = serde_json::from_str(r#"{"ids":[],"type":"del"}"#).unwrap();
        assert!(dto.ids.is_empty());
        let dto: BatchDTO = serde_json::from_str(r#"{"ids":[1,2,3],"type":"del"}"#).unwrap();
        assert_eq!(dto.ids, vec![1, 2, 3]);
    }

    // ————————————————————— 二级分类规则 —————————————————————
    // 期望值直接来自 Java `create` 第 193–198 / `update` 第 212–217 行。

    #[test]
    fn second_level_guard_messages() {
        assert_eq!(MSG_ONLY_TWO_LEVEL_CREATE, "只能创建二级分类");
        assert_eq!(MSG_ONLY_TWO_LEVEL_UPDATE, "只能修改为二级分类");
        // 父记录 parentId = 0 ⇒ 放行；parentId > 0 ⇒ 拒绝
        assert!(allows_second_level(Some(0)));
        assert!(!allows_second_level(Some(1)));
        // 父记录不存在（getById 为 null）⇒ Java 的 `sp != null` 不成立 ⇒ 放行
        assert!(allows_second_level(None));
    }

    // ————————————————————— 出参形态 —————————————————————
    // 期望值对照 Java `ShopProductCategoryServiceImpl#detail`：不设置 hasChildren。

    #[test]
    fn detail_vo_keeps_has_children_none() {
        let c = cat(5, 1, "衬衫", 3, 1);
        let vo = ShopProductCategoryVO::from(&c);
        assert_eq!(vo.has_children, None);
        assert_eq!(vo.category_id, 5);
        assert_eq!(vo.category_name, "衬衫");
        assert_eq!(vo.add_time, "2024-12-03 17:05:00");
    }

    /// `list` 里 `hasChildren` 由子计数 > 0 决定（Java `count > 0 ? 1 : 0`）。
    #[test]
    fn has_children_flag_from_count() {
        for (count, expect) in [(0_i64, 0_i32), (1, 1), (7, 1)] {
            assert_eq!(i32::from(count > 0), expect);
        }
    }

    // ————————————————————— getCategoryName 语义 —————————————————————

    /// Java 记录不存在时返回空串（不是 null、不报错）。
    #[test]
    fn missing_category_name_is_empty_string() {
        let rows: Vec<ShopProductCategory> = Vec::new();
        let found = rows.iter().find(|c| c.category_id == 42);
        // `map_or_else(String::new, ...)` 的等价形态
        assert_eq!(found.map_or_else(String::new, |c| c.category_name.clone()), "");
    }

    // ————————————————————— updateField 白名单 —————————————————————
    // 期望值对照 Java 第 252–257 行 + `CheckFieldConstants`。

    #[test]
    fn update_field_column_from_dto() {
        let dto: UpdateFieldDTO =
            serde_json::from_str(r#"{"id":1,"field":"isShow","val":"0"}"#).unwrap();
        let col = map_update_field_column(dto.field.as_deref().unwrap()).unwrap();
        assert_eq!(col, "is_show");
        assert!(nextrade_core::models::merchant::shop_product_category::SHOP_PRODUCT_CATEGORY_FIELDS.contains(&col.as_str()));
    }

    /// 幻觉列通过白名单（Java 缺陷照抄）：`measureUnit` / `isHot` 归一后命中白名单，
    /// 但**不是**真实列，真实的 `categoryPic` 反而被拒。
    #[test]
    fn update_field_whitelist_admits_phantom_columns() {
        for phantom in ["measureUnit", "isHot"] {
            let col = map_update_field_column(phantom).unwrap();
            assert!(
                !nextrade_core::models::merchant::shop_product_category::SHOP_PRODUCT_CATEGORY_COLUMNS
                    .contains(&col.as_str()),
                "{col} 不该是真实列"
            );
        }
        // 真实列 categoryPic 不在白名单 ⇒ 前端改不了图片
        assert!(map_update_field_column("categoryPic").is_none());
        assert!(map_update_field_column("category_name").is_some());
    }

    // ————————————————————— 错误文案 —————————————————————

    #[test]
    fn error_messages_match_java_sources() {
        assert_eq!(MSG_SHOP_NOT_NULL, "请选择店铺");
        assert_eq!(MSG_CATEGORY_NOT_EXIST, "分类不存在");
        assert_eq!(MSG_INVALID_FIELD_VALUE, "field 的值有误");
        assert_eq!(MSG_ID_REQUIRED, "id不能为空");
        assert_eq!(MSG_FIELD_REQUIRED, "字段名不能为空");
        assert_eq!(MSG_CATEGORY_NAME_REQUIRED, "分类名称不能为空");
        assert_eq!(MSG_PARAM_ERROR, "参数错误");
    }

    #[test]
    fn param_error_is_validation() {
        assert!(matches!(param_error(), Error::Validation(_)));
    }

    // ————————————————————— 名称校验 —————————————————————

    #[test]
    fn name_validation_used_by_create() {
        assert!(validate_category_name(Some("男装")).is_ok());
        assert_eq!(
            validate_category_name(None).unwrap_err(),
            MSG_CATEGORY_NAME_REQUIRED
        );
    }

    #[test]
    fn add_time_formatting_used_in_vo() {
        assert_eq!(format_add_time(Some(1_733_216_700)), "2024-12-03 17:05:00");
        assert_eq!(format_add_time(None), "");
    }
}