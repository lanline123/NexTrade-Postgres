//! 商品分类域服务（catalog::category）。
//!
//! 对应原 Java：
//! - `com.nextrade.service.product.impl.CategoryServiceImpl`
//! - `com.nextrade.service.product.impl.ProductServiceImpl` 中的
//!   `getLookAlso` / `getProductRelate` / `getRelateRank` / `getProductRankData`
//!   / `getRelateBrand` / `getArticleList` / `getRelateCategory` / `parentTree`
//!
//! 迁移要点（逐条对照 Java 源）：
//! - `list`：`parentId >= 0` 才过滤；排序字段白名单（未知回退主键）、
//!   方向缺省降序；子分类数量**批量统计**后回填 `hasChildren`。
//! - `detail`：Java 对不存在的分类**不报错**（`BeanUtils.copyProperties(null, dto)`
//!   产出全空 DTO），这里保持同一行为（返回 `None`）。
//! - `create` / `update`：名称非空 + 长度 ≤ 30 + SEO 标识正则 `^[a-z0-9\-]+$`
//!   + 同父级下名称唯一。
//! - `del`：有子分类 → 拒绝；分类下有商品 → 带数量的业务错误。
//! - `batch`：仅 `type = "del"` 有实现；其余 Java 返回 `false`（静默无操作）。
//! - `batchDel`：按层级深度倒序（先删子类）**先全部校验**再执行删除。
//! - `updateField`：`CATEGORY_FIELDS` 白名单。
//! - `moveCat`：把 `id` 分类下的**商品**改挂到 `targetCatId`（不动分类节点）。
//! - `getChildrenCategoryTreeById`：基于「可见分类树快照」切片；
//!   `id` 为空 / `0` 返回整片森林，节点不存在返回空列表。
//! - `getRelatedCategoryIds`：先向上找到祖先，再向下收集整棵子树 ID。
//! - `parentTree`：最多向上 4 层，带循环引用保护，最后按 `parentId` 升序。
//!
//! **未迁移（本切片明确不做，已在 `uncertain_points` 中列明）**：
//! - **价格引擎**：Java `getProductSkuDetail(productId, skuId, ...)` 会叠加会员
//!   等级折扣、促销/秒杀、批发阶梯、门店（O2O）覆盖、多店铺价差等规则。
//!   该链路未迁移，本切片对 `ProductLookAlsoVO.price` 采用**保守兜底**：
//!   首个启用 SKU 的 `sku_price`，无 SKU 时回退 `product.product_price`，
//!   并**不做**任何折扣/促销计算。结果通常**高于** Java 的最终成交价。
//! - **批发价回填**：Java `backfillFirstWholesalePriceForListGeneric` 依赖
//!   `wholesalerService`（批发商域未迁移），故 `firstWholesalePrice` /
//!   `wholesaleCardMoq` / `wholesaleMoqDimension` / `wholesaleMultiple`
//!   一律返回 `null`，而不是猜测数值。
//! - **多语言翻译**：Java `translatePackage.translate(...)`（`categoryName` /
//!   `productName`）未迁移，原样返回库中值。
//! - **缓存失效**：Java `invalidateCategoryCachesAfterCommit`（进程内分类树
//!   快照 + Redis `CAT_FLOOR` / SEO 链接缓存）未迁移；Rust 侧每次实时查库，
//!   结果正确性不受影响，仅少了缓存。
//! - **Redis SEO 缓存清理**：`redisCache.deleteObject(SEO_LINK_CATEGORY + id)`
//!   未迁移。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_core::entities::Category;
use nextrade_core::models::catalog::category::{
    intro_kind, resolve_sort_column, resolve_sort_desc, BatchDTO, CategoryDTO, CategoryListQuery,
    CategoryParentTreeVO, CategoryTreeVO, IntroKind, MoveCatDTO, OperateDTO, ProductLookAlsoVO,
    ProductRelateQuery, RelateArticleVO, RelateBrandVO, RelateCategoryVO, RelateRankVO,
    UpdateFieldDTO, CATEGORY_FIELDS,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::catalog::category::{
    CategoryRow as RepoCategoryRow, ProductRow as RepoProductRow, CategoryRepository,
};
use nextrade_infrastructure::db::Repositories;

/// 分类树最大深度，对应 Java `TreeNodeConfig.setDeep(4)`。
const TREE_MAX_DEPTH: u32 = 4;

/// 热门分类上限，对应 `getHotCategory` 的 `last("limit 20")`。
const HOT_CATEGORY_LIMIT: u32 = 20;

/// `parentTree` 向上遍历的最大循环次数，对应 Java `MAX_LOOPS = 4`。
const PARENT_TREE_MAX_LOOPS: u32 = 4;

/// 列表分页上限，对应 Java `BasePage.getSize()` 的 `> 100 → 100`。
const MAX_PAGE_SIZE: u64 = 100;

/// 列表分页默认值，对应 Java `BasePage.getSize()` 的缺省 15。
const DEFAULT_PAGE_SIZE: u64 = 15;

pub struct CategoryService {
    repos: Arc<Repositories>,
}

impl CategoryService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 便捷访问本切片独占仓储。
    fn repo(&self) -> &CategoryRepository {
        &self.repos.catalog.category
    }

    // ═══════════════════════ 管理端 ═══════════════════════

    /// 商品分类分页列表。
    ///
    /// 对应 `CategoryController.list` → `CategoryServiceImpl.list`。
    ///
    /// `BasePage` 语义：`page` 缺省 1；`size` 缺省 15、上限 100。
    pub async fn list(&self, q: &CategoryListQuery) -> Result<PageResult<CategoryDTO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1).max(1),
            page_size: q
                .page_size
                .unwrap_or(DEFAULT_PAGE_SIZE)
                .clamp(1, MAX_PAGE_SIZE),
        };
        let column = resolve_sort_column(q.sort_field.as_deref());
        let desc = resolve_sort_desc(q.sort_order.as_deref());

        let (total, rows) = self
            .repo()
            .paginate_categories(q.parent_id, column, desc, &page)
            .await?;

        // 批量统计直接子分类数量（Java 显式消除 N+1）
        let ids: Vec<i32> = rows.iter().map(|c| c.category_id).collect();
        let child_counts: HashMap<i32, i64> = self
            .repo()
            .count_children_by_parent_ids(&ids)
            .await?
            .into_iter()
            .collect();

        let list = rows
            .iter()
            .map(|c| {
                let mut dto = category_to_dto(c);
                dto.has_children = Some(child_counts.get(&c.category_id).copied().unwrap_or(0));
                dto
            })
            .collect();

        Ok(PageResult::new(total, list, &page))
    }

    /// 取父级分类名称，不存在返回 `None`（对应 `getParentName`，Java 返回 `null`）。
    pub async fn parent_name(&self, parent_id: Option<i32>) -> Result<Option<String>> {
        let Some(id) = parent_id else {
            return Ok(None);
        };
        Ok(self
            .repo()
            .find_category(id)
            .await?
            .map(|c| c.category_name))
    }

    /// 商品分类详情，对应 `CategoryServiceImpl.detail`。
    ///
    /// Java 对不存在的 ID 返回**全空 DTO**（不抛异常），故这里返回 `None`。
    pub async fn detail(&self, category_id: i32) -> Result<Option<CategoryDTO>> {
        Ok(self
            .repo()
            .find_category(category_id)
            .await?
            .map(|c| category_to_dto(&c)))
    }

    /// 创建商品分类，对应 `CategoryServiceImpl.create`。
    ///
    /// 同父级下名称唯一 → 参数校验 → 落库；返回自增主键。
    pub async fn create(&self, dto: &CategoryDTO) -> Result<i32> {
        self.check_name_unique(dto.category_name.as_deref(), dto.parent_id, None)
            .await?;
        let entity = to_entity(dto)?;
        self.repo().insert_category(&entity).await
    }

    /// 更新商品分类，对应 `CategoryServiceImpl.update`。
    pub async fn update(&self, dto: &CategoryDTO) -> Result<()> {
        let id = dto
            .category_id
            .ok_or_else(|| Error::Validation("分类ID不能为空".into()))?;
        self.check_name_unique(dto.category_name.as_deref(), dto.parent_id, Some(id))
            .await?;
        let mut entity = to_entity(dto)?;
        entity.category_id = id;
        let _ = self.repo().update_category(&entity).await?;
        Ok(())
    }

    /// 删除单个商品分类。
    ///
    /// 守卫逻辑在 Java 的 **Controller** 中（`CategoryController.del`）：
    /// 1. 有子分类 → `error.category.has.subcategory`
    /// 2. 分类下有商品 → `error.category.in.use.has.products`
    pub async fn del(&self, dto: &OperateDTO) -> Result<()> {
        let id = dto.id.ok_or_else(|| Error::Validation("ID不能为空".into()))?;

        if self.repo().has_child(id).await? {
            return Err(Error::biz_msg("当前分类下有子分类,无法删除"));
        }
        let count = self.repo().count_products_in_category(id).await?;
        if count > 0 {
            return Err(Error::biz_msg(format!(
                "无法删除使用中的分类,分类下包含{count}个商品,请先把绑定该分类的商品转移至其他类目下后在进行删除"
            )));
        }
        let _ = self.repo().delete_category(id).await?;
        Ok(())
    }

    /// 批量操作，对应 `CategoryController.batch`。
    ///
    /// Java：`"del".equals(type)` 走 `batchDel`，否则走 `BaseServiceImpl.batch`，
    /// 而基类对非 `del` 类型**直接返回 false**（不做任何事，也不报错）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<()> {
        if dto.r#type.as_deref() == Some("del") {
            return self.batch_del(&dto.ids.clone().unwrap_or_default()).await;
        }
        // 非 del：Java 为静默无操作
        Ok(())
    }

    /// 批量删除，对应 `CategoryServiceImpl.batchDel`。
    ///
    /// - `ids` 为空：Java 直接 return（无操作）
    /// - 按层级深度**倒序**（深度大的子类先删）
    /// - **先全部校验**是否有「不在删除列表内的子分类」与「分类下商品」，
    ///   任一不通过即抛错且不删任何数据
    pub async fn batch_del(&self, ids: &[i32]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let categories = self.repo().find_categories_by_ids(ids).await?;
        let by_id: HashMap<i32, Category> = categories
            .iter()
            .map(|c| (c.category_id, c.clone()))
            .collect();

        // 深度倒序：子类排前面
        let mut sorted = categories;
        sorted.sort_by(|a, b| {
            let da = depth_of(a, &by_id);
            let db = depth_of(b, &by_id);
            db.cmp(&da)
        });

        for c in &sorted {
            let name = c.category_name.as_str();
            if self.repo().has_child_excluding(c.category_id, ids).await? {
                return Err(Error::biz_msg(format!("分类[{name}]下有子分类,无法删除")));
            }
            let count = self.repo().count_products_in_category(c.category_id).await?;
            if count > 0 {
                return Err(Error::biz_msg(format!(
                    "无法删除分类[{name}],分类下包含{count}个商品,请先转移商品"
                )));
            }
        }

        for c in &sorted {
            let _ = self.repo().delete_category(c.category_id).await?;
        }
        Ok(())
    }

    /// 商品分类字段更新，对应 `CategoryController.updateField` +
    /// `BaseServiceImpl.updateField`（白名单 `CATEGORY_FIELDS`）。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let id = dto.id.ok_or_else(|| Error::Validation("ID不能为空".into()))?;
        let field = dto
            .field
            .as_deref()
            .ok_or_else(|| Error::Validation("字段名不能为空".into()))?;
        if !CATEGORY_FIELDS.contains(&field) {
            return Err(Error::Validation(format!("不允许更新的字段: {field}")));
        }
        let val = dto.val.clone().unwrap_or_default();
        self.repo().update_category_field(id, field, &val).await
    }

    /// 移动商品分类下的商品到目标分类，对应 `CategoryServiceImpl.moveCat`。
    ///
    /// Java：`id` 或 `targetCatId` 为空 → 「参数错误」；随后
    /// `UPDATE product SET category_id = target WHERE category_id = id`，
    /// 返回 `true`（无论影响行数）。
    pub async fn move_cat(&self, dto: &MoveCatDTO) -> Result<()> {
        let (from, to) = match (dto.id, dto.target_cat_id) {
            (Some(a), Some(b)) => (a, b),
            _ => return Err(Error::biz_msg("参数错误")),
        };
        let _ = self.repo().move_products_category(from, to).await?;
        Ok(())
    }

    // ═══════════════════════ 前台 ═══════════════════════

    /// 获取指定分类下的子分类树，对应
    /// `CategoryController.getChildrenCategoryTreeById` →
    /// `CategoryServiceImpl.getChildrenCategoryTreeById`。
    ///
    /// 语义等价于 `CategoryTreeCache.sliceChildren`：
    /// - 快照为空 → 空列表
    /// - `categoryId` 为空或 `0` → 返回整片森林（根分类列表）
    /// - 节点不在快照中 → 空列表
    /// - 否则 → 该节点的**直接子节点**森林
    ///
    /// `shopId` 参数在 Java 实现里**完全未使用**（只是签名占位），故忽略。
    pub async fn children_category_tree(
        &self,
        category_id: Option<i32>,
    ) -> Result<Vec<CategoryTreeVO>> {
        let forest = self.visible_forest().await?;
        if forest.is_empty() {
            return Ok(Vec::new());
        }
        match category_id {
            None | Some(0) => Ok(forest),
            Some(id) => Ok(find_node(&forest, id)
                .map(|n| n.children.clone())
                .unwrap_or_default()),
        }
    }

    /// 热门分类，对应 `CategoryServiceImpl.getHotCategory`。
    pub async fn hot_categories(&self) -> Result<Vec<Category>> {
        self.repo().find_hot_categories(HOT_CATEGORY_LIMIT).await
    }

    /// 看了还看，对应 `ProductServiceImpl.getLookAlso`。
    pub async fn relate_look_also(
        &self,
        q: &ProductRelateQuery,
    ) -> Result<Vec<ProductLookAlsoVO>> {
        let q = q.with_defaults();
        let product_id = q.product_id.unwrap_or(0);
        // Java 对不存在的商品会 NPE（→ 500）；这里返回明确的业务错误
        let product = self
            .repo()
            .find_product(product_id)
            .await?
            .ok_or_else(|| Error::biz_msg("商品不存在"))?;

        let category_ids = self.related_category_ids(product.category_id).await?;
        if category_ids.is_empty() {
            return Ok(Vec::new());
        }

        let flag = intro_flag_column(intro_kind(q.intro.as_deref()));
        let products = self
            .repo()
            .find_look_also_products(&category_ids, flag, q.size_or_default())
            .await?;
        self.to_look_also_list(&products).await
    }

    /// 相关排行，对应 `ProductServiceImpl.getRelateRank`。
    ///
    /// 商品不存在时 Java 抛 `PRODUCT_NOT_EXIST`，这里返回业务错误。
    pub async fn relate_rank(&self, q: &ProductRelateQuery) -> Result<RelateRankVO> {
        let q = q.with_defaults();
        let product_id = q.product_id.unwrap_or(0);
        let product = self
            .repo()
            .find_product(product_id)
            .await?
            .ok_or_else(|| Error::biz_msg("商品不存在"))?;

        let category_ids = self.related_category_ids(product.category_id).await?;
        let limit = q.rank_num_or_default();

        let price_rows = if category_ids.is_empty() {
            Vec::new()
        } else {
            self.repo()
                .find_rank_by_price(&category_ids, product_id, product.market_price, limit)
                .await?
        };
        let brand_rows = self
            .repo()
            .find_rank_by_brand(product.brand_id, product_id, limit)
            .await?;
        let cate_rows = self
            .repo()
            .find_rank_by_category(product.category_id, product_id, limit)
            .await?;

        Ok(RelateRankVO {
            price: self.to_look_also_list(&price_rows).await?,
            brand: self.to_look_also_list(&brand_rows).await?,
            cate: self.to_look_also_list(&cate_rows).await?,
        })
    }

    /// 相关品牌，对应 `ProductServiceImpl.getRelateBrand`。
    pub async fn relate_brand(&self, q: &ProductRelateQuery) -> Result<Vec<RelateBrandVO>> {
        let q = q.with_defaults();
        let product_id = q.product_id.unwrap_or(0);
        let product = self
            .repo()
            .find_product(product_id)
            .await?
            .ok_or_else(|| Error::biz_msg("商品不存在"))?;

        let size = q.size_or_default();
        let category_ids = self.related_category_ids(product.category_id).await?;
        if category_ids.is_empty() {
            return Ok(Vec::new());
        }
        let brand_ids = self
            .repo()
            .find_brand_ids_in_categories(&category_ids, size)
            .await?;
        if brand_ids.is_empty() {
            return Ok(Vec::new());
        }
        self.repo().find_related_brands(&brand_ids, size).await
    }

    /// 相关文章，对应 `ProductServiceImpl.getArticleList`。
    pub async fn relate_article(&self, q: &ProductRelateQuery) -> Result<Vec<RelateArticleVO>> {
        let q = q.with_defaults();
        let product_id = q.product_id.unwrap_or(0);
        let article_ids = self.repo().find_article_ids_by_product(product_id).await?;
        if article_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = self
            .repo()
            .find_visible_articles_by_ids(&article_ids, q.size_or_default())
            .await?;
        Ok(rows
            .iter()
            .map(CategoryRepository::to_relate_article)
            .collect())
    }

    /// 相关分类，对应 `ProductServiceImpl.getRelateCategory`。
    pub async fn relate_category(&self, q: &ProductRelateQuery) -> Result<Vec<RelateCategoryVO>> {
        let q = q.with_defaults();
        let product_id = q.product_id.unwrap_or(0);
        let product = self
            .repo()
            .find_product(product_id)
            .await?
            .ok_or_else(|| Error::biz_msg("商品不存在"))?;

        let category_ids = self.related_category_ids(product.category_id).await?;
        if category_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = self
            .repo()
            .find_visible_categories_in(&category_ids, q.size_or_default())
            .await?;
        Ok(rows.iter().map(to_relate_category).collect())
    }

    /// 父级分类，对应 `ProductServiceImpl.parentTree`。
    ///
    /// `categoryId` 为空 → 空列表；最多向上 4 层；带循环引用保护；
    /// 结果按 `parentId` 升序。
    pub async fn parent_tree(&self, category_id: Option<i32>) -> Result<Vec<CategoryParentTreeVO>> {
        let Some(start) = category_id else {
            return Ok(Vec::new());
        };
        // Java 每层各查一次库；这里一次载入后用内存映射做等价遍历
        let all = self.repo().find_categories(false).await?;
        let by_id: HashMap<i32, RepoCategoryRow> =
            all.into_iter().map(|r| (r.category_id, r)).collect();
        Ok(build_parent_tree(start, &by_id))
    }

    // ═══════════════════════ 内部 ═══════════════════════

    /// 可见分类树森林（`is_show = 1`，深度 ≤ 4）。
    async fn visible_forest(&self) -> Result<Vec<CategoryTreeVO>> {
        let rows = self.repo().find_categories(true).await?;
        Ok(build_tree(&rows, 0, 1))
    }

    /// 分类及其祖先整棵子树的 ID 集合。
    ///
    /// 对应 `CategoryServiceImpl.getRelatedCategoryIds`：
    /// 1. 在**可见树快照**中向上找到祖先；
    /// 2. 从祖先开始向下收集所有子节点 ID（含自身，带环保护）。
    ///
    /// ⚠️ 与 Java 一致：若入参分类**不在可见树中**（例如 `is_show = 0`），
    /// 直接返回空列表（而不是退化为「仅自身」）。
    async fn related_category_ids(&self, category_id: i32) -> Result<Vec<i32>> {
        let forest = self.visible_forest().await?;
        let Some(node) = find_node(&forest, category_id) else {
            return Ok(Vec::new());
        };

        // 向上找祖先：parent_id 为 0 或不在森林中即停（带环保护）
        let mut ancestor_id = node.category_id;
        let mut cursor = node;
        let mut visited: HashSet<i32> = HashSet::new();
        while cursor.parent_id != 0 {
            if !visited.insert(cursor.category_id) {
                break; // 环：强制终止
            }
            match find_node(&forest, cursor.parent_id) {
                Some(parent) => {
                    ancestor_id = parent.category_id;
                    cursor = parent;
                }
                None => break,
            }
        }

        let Some(ancestor) = find_node(&forest, ancestor_id) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        collect_ids(ancestor, &mut out, &mut HashSet::new());
        Ok(out)
    }

    /// 把商品行批量转成「看了还看」卡片（批量取 SKU 价，消除 N+1）。
    async fn to_look_also_list(
        &self,
        products: &[RepoProductRow],
    ) -> Result<Vec<ProductLookAlsoVO>> {
        if products.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<i32> = products.iter().map(|p| p.product_id).collect();
        let sku_map: HashMap<i32, Option<Decimal>> = self
            .repo()
            .find_first_sku_prices(&ids)
            .await?
            .into_iter()
            .map(|r| (r.product_id, r.sku_price))
            .collect();

        Ok(products
            .iter()
            .map(|p| {
                // ⚠️ 保守兜底：Java 走 getProductSkuDetail 计算最终成交价，
                // 价格引擎未迁移，这里仅取 SKU 原始价 / 商品零售价。
                let price = sku_map
                    .get(&p.product_id)
                    .copied()
                    .flatten()
                    .unwrap_or(p.product_price);
                ProductLookAlsoVO {
                    market_price: p.market_price.to_string(),
                    pic_thumb: p.pic_thumb.clone(),
                    pic_url: p.pic_url.clone(),
                    price: price.to_string(),
                    product_id: p.product_id,
                    product_name: p.product_name.clone(),
                    product_sn: p.product_sn.clone(),
                    shop_id: p.shop_id,
                    // 批发域未迁移：不猜测数值，保持 null
                    first_wholesale_price: None,
                    wholesale_card_moq: None,
                    wholesale_moq_dimension: None,
                    wholesale_multiple: None,
                }
            })
            .collect())
    }

    /// 同名校验，对应 `checkCategoryNameUnique`。
    async fn check_name_unique(
        &self,
        name: Option<&str>,
        parent_id: Option<i32>,
        exclude_id: Option<i32>,
    ) -> Result<()> {
        // Java `StrUtil.isBlank`：null / 空串 / 全空白都跳过校验
        let Some(name) = name.filter(|n| !n.trim().is_empty()) else {
            return Ok(());
        };
        if self
            .repo()
            .exists_same_name(name, parent_id, exclude_id)
            .await?
        {
            return Err(Error::biz_msg(format!(
                "同一级分类下已存在名称为【{name}】的分类"
            )));
        }
        Ok(())
    }
}

/// `IntroKind` → `product` 表筛选列（白名单，无注入面）。
#[must_use]
fn intro_flag_column(kind: IntroKind) -> &'static str {
    match kind {
        IntroKind::New => "is_new",
        IntroKind::Best => "is_best",
        IntroKind::Hot => "is_hot",
    }
}

/// `Category` 实体 → `CategoryDTO`（对应 `BeanUtils.copyProperties`）。
#[must_use]
fn category_to_dto(c: &Category) -> CategoryDTO {
    CategoryDTO {
        category_id: Some(c.category_id),
        category_name: Some(c.category_name.clone()),
        seo_identification: c.seo_identification.clone(),
        keywords: Some(c.keywords.clone()),
        category_desc: Some(c.category_desc.clone()),
        parent_id: Some(c.parent_id),
        sort_order: Some(c.sort_order),
        measure_unit: Some(c.measure_unit.clone()),
        is_show: Some(c.is_show),
        seo_title: Some(c.seo_title.clone()),
        short_name: Some(c.short_name.clone()),
        category_pic: Some(c.category_pic.clone()),
        category_ico: Some(c.category_ico.clone()),
        is_hot: Some(c.is_hot),
        search_keywords: Some(c.search_keywords.clone()),
        has_children: None,
    }
}

/// `CategoryDTO` → `Category` 实体（对应 `convertToEntity`）。
///
/// **业务校验顺序**（与 Java 一致）：
/// 1. `@NotNull` → 分类名称不能为空
/// 2. `@Size(max = 30)` → 分类名称长度不能超过30
/// 3. `SeoIdentificationUtil.isValidSeoIdentification` → SEO 标识正则
///
/// `BeanUtils.copyProperties` 对 DTO 中的 `null` 会保留实体默认值，故此处
/// 用空串 / `0` 兜底；`is_show` 的实体默认值为 `1`
/// （与 `category` 表 `is_show DEFAULT 1` 一致）。
fn to_entity(dto: &CategoryDTO) -> Result<Category> {
    let name = dto
        .category_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Validation("分类名称不能为空".into()))?;
    if name.chars().count() > 30 {
        return Err(Error::Validation("分类名称长度不能超过30".into()));
    }
    if let Some(seo) = dto.seo_identification.as_deref() {
        if !is_valid_seo_identification(seo) {
            return Err(Error::biz_msg("请输入正确的SEO标识"));
        }
    }
    Ok(Category {
        category_id: dto.category_id.unwrap_or(0),
        category_name: name.to_string(),
        seo_identification: dto.seo_identification.clone(),
        keywords: dto.keywords.clone().unwrap_or_default(),
        category_desc: dto.category_desc.clone().unwrap_or_default(),
        parent_id: dto.parent_id.unwrap_or(0),
        sort_order: dto.sort_order.unwrap_or(0),
        measure_unit: dto.measure_unit.clone().unwrap_or_default(),
        is_show: dto.is_show.unwrap_or(1),
        seo_title: dto.seo_title.clone().unwrap_or_default(),
        short_name: dto.short_name.clone().unwrap_or_default(),
        category_pic: dto.category_pic.clone().unwrap_or_default(),
        category_ico: dto.category_ico.clone().unwrap_or_default(),
        is_hot: dto.is_hot.unwrap_or(0),
        search_keywords: dto.search_keywords.clone().unwrap_or_default(),
    })
}

/// SEO 标识校验：`StrUtil.isBlank` 视为合法，否则须匹配 `^[a-z0-9\-]+$`。
#[must_use]
fn is_valid_seo_identification(s: &str) -> bool {
    if s.trim().is_empty() {
        return true;
    }
    s.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// `Category` → 相关分类 VO。
#[must_use]
fn to_relate_category(c: &Category) -> RelateCategoryVO {
    RelateCategoryVO {
        category_id: c.category_id,
        category_name: c.category_name.clone(),
        parent_id: c.parent_id,
        sort_order: c.sort_order,
        is_show: c.is_show,
        category_pic: c.category_pic.clone(),
        category_ico: c.category_ico.clone(),
    }
}

/// 递归构建分类树；`depth` 超过 [`TREE_MAX_DEPTH`] 即截断。
///
/// 对应 hutool `TreeUtil.build(list, 0, config, ...)`：按 `sort_order`
/// （hutool 的 `weight`）升序；未匹配到父节点的分类被丢弃。
#[must_use]
fn build_tree(rows: &[RepoCategoryRow], parent_id: i32, depth: u32) -> Vec<CategoryTreeVO> {
    if depth > TREE_MAX_DEPTH {
        return Vec::new();
    }
    let mut nodes: Vec<CategoryTreeVO> = rows
        .iter()
        .filter(|r| r.parent_id == parent_id)
        .map(|r| CategoryTreeVO {
            category_id: r.category_id,
            parent_id: r.parent_id,
            category_name: r.category_name.clone(),
            sort_order: r.sort_order,
            is_show: r.is_show,
            category_pic: r.category_pic.clone(),
            children: build_tree(rows, r.category_id, depth + 1),
        })
        .collect();
    nodes.sort_by_key(|n| (n.sort_order, n.category_id));
    nodes
}

/// 在森林中按 ID 查找节点（深度优先，覆盖全部层级）。
#[must_use]
fn find_node<'a>(forest: &'a [CategoryTreeVO], id: i32) -> Option<&'a CategoryTreeVO> {
    for node in forest {
        if node.category_id == id {
            return Some(node);
        }
        if let Some(found) = find_node(&node.children, id) {
            return Some(found);
        }
    }
    None
}

/// 收集节点及其全部后代的 ID（先序，带环保护，对应 `collectAllIdsSafe`）。
fn collect_ids(node: &CategoryTreeVO, out: &mut Vec<i32>, visited: &mut HashSet<i32>) {
    if !visited.insert(node.category_id) {
        return; // 环：终止
    }
    out.push(node.category_id);
    for child in &node.children {
        collect_ids(child, out, visited);
    }
}

/// 构建父级分类树，对应 `ProductServiceImpl.parentTree`。
///
/// - 从 `start` 开始，最多向上 [`PARENT_TREE_MAX_LOOPS`] 层
/// - 循环引用立即终止
/// - 每层收集「同一 `parent_id` 且 `is_show = 1`」的同级分类（**不排除自身**）
/// - 最终按 `parent_id` 升序排序（Java `Comparator.comparingInt`）
#[must_use]
fn build_parent_tree(
    start: i32,
    by_id: &HashMap<i32, RepoCategoryRow>,
) -> Vec<CategoryParentTreeVO> {
    let mut out = Vec::new();
    let mut visited: HashSet<i32> = HashSet::new();
    let mut current = start;
    let mut loops: u32 = 0;

    while current != 0 && loops < PARENT_TREE_MAX_LOOPS {
        if !visited.insert(current) {
            break; // 循环引用
        }
        let Some(category) = by_id.get(&current) else {
            break;
        };
        let parent_id = category.parent_id;

        // 同一父级下的可见同级分类（Java 不排除自身）
        let mut cat_list: Vec<CategoryTreeVO> = by_id
            .values()
            .filter(|c| c.parent_id == parent_id && c.is_show == 1)
            .map(|c| CategoryTreeVO {
                category_id: c.category_id,
                parent_id: c.parent_id,
                category_name: c.category_name.clone(),
                sort_order: c.sort_order,
                is_show: c.is_show,
                category_pic: c.category_pic.clone(),
                children: Vec::new(),
            })
            .collect();
        cat_list.sort_by_key(|c| (c.sort_order, c.category_id));

        out.push(CategoryParentTreeVO {
            category_id: category.category_id,
            category_name: category.category_name.clone(),
            parent_id,
            cat_list,
        });

        current = parent_id;
        loops += 1;
    }

    out.sort_by_key(|n| n.parent_id);
    out
}

/// 计算分类在**给定映射内**向上追溯的深度（对应 `calculateDepth`）。
#[must_use]
fn depth_of(category: &Category, by_id: &HashMap<i32, Category>) -> u32 {
    let mut depth = 0;
    let mut parent_id = category.parent_id;
    while parent_id != 0 {
        match by_id.get(&parent_id) {
            Some(p) => {
                depth += 1;
                parent_id = p.parent_id;
            }
            None => break,
        }
    }
    depth
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: i32, parent: i32, name: &str, sort: i32, show: i32) -> RepoCategoryRow {
        RepoCategoryRow {
            category_id: id,
            category_name: name.into(),
            parent_id: parent,
            sort_order: sort,
            is_show: show,
            category_pic: String::new(),
            category_ico: String::new(),
        }
    }

    fn cat(id: i32, parent: i32, name: &str) -> Category {
        Category {
            category_id: id,
            parent_id: parent,
            category_name: name.into(),
            ..Default::default()
        }
    }

    fn node(id: i32, parent: i32, children: Vec<CategoryTreeVO>) -> CategoryTreeVO {
        CategoryTreeVO {
            category_id: id,
            parent_id: parent,
            category_name: format!("n{id}"),
            sort_order: 0,
            is_show: 1,
            category_pic: String::new(),
            children,
        }
    }

    // ————————————————— 树构建 —————————————————

    #[test]
    fn build_tree_filters_by_parent_and_sorts_by_weight() {
        let rows = vec![
            row(1, 0, "根", 2, 1),
            row(2, 1, "子", 1, 1),
            row(3, 0, "另一个根", 1, 1),
        ];
        let tree = build_tree(&rows, 0, 1);
        assert_eq!(tree.len(), 2);
        // 按 sort_order 升序：另一个根(1) 在 根(2) 之前
        assert_eq!(tree[0].category_id, 3);
        assert_eq!(tree[1].category_id, 1);
        assert_eq!(tree[1].children.len(), 1);
        assert_eq!(tree[1].children[0].category_id, 2);
        // 叶子节点 children 为空数组（而非 panic / 缺失）
        assert!(tree[0].children.is_empty());
    }

    #[test]
    fn build_tree_depth_is_capped_at_four() {
        // 1 -> 2 -> 3 -> 4 -> 5 -> 6
        let rows: Vec<RepoCategoryRow> = (1..=6)
            .map(|i| row(i, i - 1, &format!("L{i}"), 0, 1))
            .collect();
        let tree = build_tree(&rows, 0, 1);
        fn depth(n: &CategoryTreeVO) -> u32 {
            1 + n.children.iter().map(depth).max().unwrap_or(0)
        }
        assert_eq!(depth(&tree[0]), TREE_MAX_DEPTH, "深度上限应为 4");
    }

    #[test]
    fn build_tree_drops_nodes_unreachable_from_root() {
        // 1 <-> 2 互指，均不可从 0 到达 → 森林为空（hutool 同样会丢弃）
        let rows = vec![row(1, 2, "a", 0, 1), row(2, 1, "b", 0, 1)];
        assert!(build_tree(&rows, 0, 1).is_empty());
    }

    #[test]
    fn find_node_searches_all_levels() {
        let rows = vec![
            row(1, 0, "a", 0, 1),
            row(2, 1, "b", 0, 1),
            row(3, 2, "c", 0, 1),
        ];
        let tree = build_tree(&rows, 0, 1);
        assert!(find_node(&tree, 3).is_some());
        assert!(find_node(&tree, 99).is_none());
    }

    #[test]
    fn collect_ids_includes_self_and_descendants_once() {
        let n = node(
            1,
            0,
            vec![node(2, 1, vec![node(3, 2, vec![])]), node(4, 1, vec![])],
        );
        let mut out = Vec::new();
        collect_ids(&n, &mut out, &mut HashSet::new());
        assert_eq!(out, vec![1, 2, 3, 4]);
    }

    #[test]
    fn collect_ids_terminates_on_cycle() {
        // 手工构造环形 children（正常数据不会出现，但必须有环保护）
        let mut a = node(1, 0, vec![]);
        let b = node(2, 1, vec![]);
        a.children = vec![b];
        let mut out = Vec::new();
        collect_ids(&a, &mut out, &mut HashSet::new());
        assert_eq!(out, vec![1, 2]);
    }

    // ————————————————— 参数校验 —————————————————

    #[test]
    fn seo_validation_matches_java_regex() {
        assert!(is_valid_seo_identification(""));
        assert!(is_valid_seo_identification("   "));
        assert!(is_valid_seo_identification("abc"));
        assert!(is_valid_seo_identification("a-b-c"));
        assert!(is_valid_seo_identification("a1b2"));
        assert!(!is_valid_seo_identification("Abc"));
        assert!(!is_valid_seo_identification("a_b"));
        assert!(!is_valid_seo_identification("a b"));
        assert!(!is_valid_seo_identification("手机"));
    }

    #[test]
    fn to_entity_requires_category_name() {
        let dto = CategoryDTO::default();
        let err = to_entity(&dto).unwrap_err();
        assert!(err.to_string().contains("分类名称不能为空"), "实际: {err}");

        let mut dto = CategoryDTO::default();
        dto.category_name = Some("   ".into());
        assert!(to_entity(&dto).is_err(), "全空白名称应被拒绝");
    }

    #[test]
    fn to_entity_rejects_overlength_name() {
        let mut dto = CategoryDTO::default();
        dto.category_name = Some("中".repeat(31));
        let err = to_entity(&dto).unwrap_err();
        assert!(err.to_string().contains("30"), "实际: {err}");

        // 恰好 30 个字符应通过
        dto.category_name = Some("中".repeat(30));
        assert!(to_entity(&dto).is_ok());
    }

    #[test]
    fn to_entity_rejects_bad_seo_and_applies_defaults() {
        let mut dto = CategoryDTO::default();
        dto.category_name = Some("手机".into());
        dto.seo_identification = Some("Bad_SEO".into());
        let err = to_entity(&dto).unwrap_err();
        assert!(err.to_string().contains("SEO"), "实际: {err}");

        dto.seo_identification = None;
        let e = to_entity(&dto).unwrap();
        // 实体默认值：is_show 1（前台显示），其余 0 / 空串
        assert_eq!(e.is_show, 1);
        assert_eq!(e.parent_id, 0);
        assert_eq!(e.sort_order, 0);
        assert_eq!(e.keywords, "");
        assert_eq!(e.category_name, "手机");
    }

    #[test]
    fn category_dto_mapping_round_trip() {
        let c = Category {
            category_id: 5,
            category_name: "数码".into(),
            parent_id: 2,
            sort_order: 7,
            is_show: 0,
            is_hot: 1,
            ..Default::default()
        };
        let dto = category_to_dto(&c);
        assert_eq!(dto.category_id, Some(5));
        assert_eq!(dto.category_name.as_deref(), Some("数码"));
        assert_eq!(dto.parent_id, Some(2));
        assert_eq!(dto.sort_order, Some(7));
        assert_eq!(dto.is_show, Some(0));
        assert_eq!(dto.is_hot, Some(1));
        assert_eq!(dto.has_children, None, "detail 不回填 hasChildren");
    }

    // ————————————————— batchDel 深度排序 —————————————————

    #[test]
    fn depth_ordering_deletes_children_first() {
        let cats = vec![cat(1, 0, "根"), cat(2, 1, "中"), cat(3, 2, "叶")];
        let by_id: HashMap<i32, Category> =
            cats.iter().map(|c| (c.category_id, c.clone())).collect();
        let mut sorted = cats.clone();
        sorted.sort_by(|a, b| depth_of(b, &by_id).cmp(&depth_of(a, &by_id)));
        let order: Vec<i32> = sorted.iter().map(|c| c.category_id).collect();
        assert_eq!(order, vec![3, 2, 1], "应按深度倒序：叶 -> 中 -> 根");

        assert_eq!(depth_of(&cats[0], &by_id), 0);
        assert_eq!(depth_of(&cats[1], &by_id), 1);
        assert_eq!(depth_of(&cats[2], &by_id), 2);
    }

    // ————————————————— parentTree —————————————————

    #[test]
    fn parent_tree_walks_up_and_groups_siblings() {
        // 0 ├─ 1 ── 2 ── 3
        //   └─ 9
        let rows = vec![
            row(1, 0, "一级", 0, 1),
            row(2, 1, "二级", 0, 1),
            row(3, 2, "三级", 0, 1),
            row(9, 0, "另一根", 0, 1),
        ];
        let by_id: HashMap<i32, RepoCategoryRow> =
            rows.into_iter().map(|r| (r.category_id, r)).collect();
        let tree = build_parent_tree(3, &by_id);

        // 向上 3 层（3 → 2 → 1），第 4 次循环 current 变成 0 即停
        assert_eq!(tree.len(), 3);
        // 结果按 parentId 升序
        assert_eq!(
            tree.iter().map(|n| n.parent_id).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        let leaf = tree.iter().find(|n| n.category_id == 3).unwrap();
        assert_eq!(leaf.parent_id, 2);
        assert_eq!(leaf.cat_list.len(), 1, "parent_id=2 下只有分类 3");
        // parent_id = 0 层有两个同级分类，且**未排除自身**
        let root = tree.iter().find(|n| n.parent_id == 0).unwrap();
        assert_eq!(root.category_id, 1);
        assert_eq!(root.cat_list.len(), 2);
    }

    #[test]
    fn parent_tree_filters_hidden_siblings() {
        let rows = vec![
            row(1, 0, "一级", 0, 1),
            row(2, 1, "二级", 0, 1),
            row(8, 0, "隐藏根", 0, 0),
        ];
        let by_id: HashMap<i32, RepoCategoryRow> =
            rows.into_iter().map(|r| (r.category_id, r)).collect();
        let tree = build_parent_tree(2, &by_id);
        let root = tree.iter().find(|n| n.parent_id == 0).unwrap();
        assert_eq!(root.cat_list.len(), 1, "is_show=0 的同级分类应被过滤");
        assert_eq!(root.cat_list[0].category_id, 1);
    }

    #[test]
    fn parent_tree_stops_on_cycle() {
        // 1 -> 2 -> 1 互指
        let rows = vec![row(1, 2, "a", 0, 1), row(2, 1, "b", 0, 1)];
        let by_id: HashMap<i32, RepoCategoryRow> =
            rows.into_iter().map(|r| (r.category_id, r)).collect();
        let tree = build_parent_tree(1, &by_id);
        // 1、2 各处理一次后再次遇到已访问 ID 即 break
        assert_eq!(tree.len(), 2);
    }

    #[test]
    fn parent_tree_is_empty_for_unknown_start() {
        let rows = vec![row(1, 0, "a", 0, 1)];
        let by_id: HashMap<i32, RepoCategoryRow> =
            rows.into_iter().map(|r| (r.category_id, r)).collect();
        assert!(build_parent_tree(999, &by_id).is_empty());
    }

    #[test]
    fn parent_tree_caps_at_four_levels() {
        // 1 <- 2 <- 3 <- 4 <- 5 <- 6（自下而上）
        let rows: Vec<RepoCategoryRow> = (1..=6)
            .map(|i| row(i, i - 1, &format!("L{i}"), 0, 1))
            .collect();
        let by_id: HashMap<i32, RepoCategoryRow> =
            rows.into_iter().map(|r| (r.category_id, r)).collect();
        let tree = build_parent_tree(6, &by_id);
        assert_eq!(tree.len(), PARENT_TREE_MAX_LOOPS as usize);
    }

    // ————————————————— intro 筛选列 —————————————————

    #[test]
    fn intro_flag_column_mapping() {
        assert_eq!(intro_flag_column(IntroKind::New), "is_new");
        assert_eq!(intro_flag_column(IntroKind::Best), "is_best");
        assert_eq!(intro_flag_column(IntroKind::Hot), "is_hot");
    }
}