//! 商城前台商品分类路由（catalog::category）。
//!
//! 对应原 Java `com.nextrade.api.controller.product.CategoryController`
//! （`@RequestMapping("/api/category/category")`）。
//!
//! | 本模块路由（挂载于 `/api`） | 原 Java | 契约 auth |
//! |-----------------------------|---------|-----------|
//! | `GET /category/category/list` | `CategoryController.getChildrenCategoryTreeById` | optional |
//! | `GET /category/category/relateLookAlso` | `CategoryController.getRelateLookAlso` | optional |
//! | `GET /category/category/hot` | `CategoryController.getHotCategory` | optional |
//! | `GET /category/category/relateRank` | `CategoryController.getRelateRank` | optional |
//! | `GET /category/category/relateArticle` | `CategoryController.getRelateArticle` | optional |
//! | `GET /category/category/relateBrand` | `CategoryController.getRelateBrand` | optional |
//! | `GET /category/category/relateCategory` | `CategoryController.getRelateCategory` | optional |
//! | `GET /category/category/parentTree` | `CategoryController.parentTree` | optional |
//!
//! > **路径说明**：注意控制器前缀**重复一次**（`/api/category/category/...`），
//! > 这不是笔误，必须逐字符照抄。
//! >
//! > **auth=optional**：原契约允许游客访问，登录后部分字段（批发价、会员价）
//! > 会变化。本实现挂在**公开**路由组，未强制登录。
//! >
//! > **未迁移**：批发价回填（`wholesalerService` 未迁移，
//! > `firstWholesalePrice` 等字段恒为 `null`）；最终成交价
//! > （Java `getProductSkuDetail` 的会员折扣/促销/秒杀/门店覆盖链路未迁移，
//! > 这里只取 SKU 原始价 / 商品零售价，见 `CategoryService` 模块注释）；
//! > 多语言翻译（`translatePackage`）。
//! >
//! > **注意**：原 Java 另有 `GET /api/category/category/all`
//! > （`CategoryController.getAllCategory`），该端点**不属本切片**，
//! > 已由 `crates/api/public/src/routes/catalog.rs` 实现，两者**路径不重叠**，
//! > 可安全 merge（无需像 admin 侧那样先去重）。
//! > **`relateBrand` 的默认值**：Java Controller 里 `size` / `rankNum`
//! > 在被置为 10 / 5 后**从未使用**（真正生效的只有 `ProductRelateDTO`
//! > 构造器的默认值），本实现照抄该行为。

use axum::{
    extract::{Query, State},
    routing::get,
    Router,
};
use serde::Deserialize;
use nextrade_core::entities::Category;
use nextrade_core::models::catalog::category::{
    CategoryParentTreeVO, CategoryTreeVO, ProductLookAlsoVO, ProductRelateQuery, RelateArticleVO,
    RelateBrandVO, RelateCategoryVO, RelateRankVO,
};
use nextrade_security::web::{ok, ApiResult};

use crate::PublicState;

/// 商品分类域前台路由（挂载于 `/api` 下）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .route("/category/category/list", get(children_category_tree))
        .route("/category/category/relateLookAlso", get(relate_look_also))
        .route("/category/category/hot", get(hot_category))
        .route("/category/category/relateRank", get(relate_rank))
        .route("/category/category/relateArticle", get(relate_article))
        .route("/category/category/relateBrand", get(relate_brand))
        .route("/category/category/relateCategory", get(relate_category))
        .route("/category/category/parentTree", get(parent_tree))
}

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    [
        (
            "GET",
            "/api/category/category/list",
            "CategoryController.getChildrenCategoryTreeById",
        ),
        (
            "GET",
            "/api/category/category/relateLookAlso",
            "CategoryController.getRelateLookAlso",
        ),
        (
            "GET",
            "/api/category/category/hot",
            "CategoryController.getHotCategory",
        ),
        (
            "GET",
            "/api/category/category/relateRank",
            "CategoryController.getRelateRank",
        ),
        (
            "GET",
            "/api/category/category/relateArticle",
            "CategoryController.getRelateArticle",
        ),
        (
            "GET",
            "/api/category/category/relateBrand",
            "CategoryController.getRelateBrand",
        ),
        (
            "GET",
            "/api/category/category/relateCategory",
            "CategoryController.getRelateCategory",
        ),
        (
            "GET",
            "/api/category/category/parentTree",
            "CategoryController.parentTree",
        ),
    ]
    .iter()
    .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, p, j))
    .collect()
}

/// `/category/category/list` 查询参数。
///
/// Java：`getChildrenCategoryTreeById(@RequestParam("id") Integer categoryId,
/// @RequestParam(value = "shopId", required = false) Integer shopId)`。
/// `shopId` 在原实现中**完全未使用**，这里保留字段以兼容前端但忽略其值。
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ChildrenCategoryQuery {
    id: Option<i32>,
    shop_id: Option<i32>,
}

/// `GET /api/category/category/list?id=&shopId=`：指定分类下的子分类树。
///
/// 对应 Java `CategoryController.getChildrenCategoryTreeById` →
/// `CategoryServiceImpl.getChildrenCategoryTreeById`（可见树快照切片）。
async fn children_category_tree(
    State(state): State<PublicState>,
    Query(q): Query<ChildrenCategoryQuery>,
) -> ApiResult<Vec<CategoryTreeVO>> {
    ok(state.catalog.category.children_category_tree(q.id).await?)
}

/// `GET /api/category/category/relateLookAlso`：商品详情页「看了还看」。
///
/// 对应 Java `CategoryController.getRelateLookAlso(ProductRelateDTO dto)`
/// → `ProductServiceImpl.getLookAlso`。
async fn relate_look_also(
    State(state): State<PublicState>,
    Query(q): Query<ProductRelateQuery>,
) -> ApiResult<Vec<ProductLookAlsoVO>> {
    ok(state.catalog.category.relate_look_also(&q).await?)
}

/// `GET /api/category/category/hot`：热门分类（`is_hot = 1`，最多 20 条）。
///
/// 对应 Java `CategoryController.getHotCategory` → `CategoryServiceImpl.getHotCategory`。
async fn hot_category(State(state): State<PublicState>) -> ApiResult<Vec<Category>> {
    ok(state.catalog.category.hot_categories().await?)
}

/// `GET /api/category/category/relateRank`：相关排行（相同价位 / 品牌 / 类别）。
///
/// 对应 Java `CategoryController.getRelateRank(ProductRelateDTO dto)`
/// → `ProductServiceImpl.getRelateRank`。
async fn relate_rank(
    State(state): State<PublicState>,
    Query(q): Query<ProductRelateQuery>,
) -> ApiResult<RelateRankVO> {
    ok(state.catalog.category.relate_rank(&q).await?)
}

/// `GET /api/category/category/relateArticle`：相关文章。
///
/// 对应 Java `CategoryController.getRelateArticle(ProductRelateDTO dto)`
/// → `ProductServiceImpl.getArticleList`。
async fn relate_article(
    State(state): State<PublicState>,
    Query(q): Query<ProductRelateQuery>,
) -> ApiResult<Vec<RelateArticleVO>> {
    ok(state.catalog.category.relate_article(&q).await?)
}

/// `GET /api/category/category/relateBrand`：相关品牌。
///
/// 对应 Java `CategoryController.getRelateBrand(ProductRelateDTO dto)`
/// → `ProductServiceImpl.getRelateBrand`。
async fn relate_brand(
    State(state): State<PublicState>,
    Query(q): Query<ProductRelateQuery>,
) -> ApiResult<Vec<RelateBrandVO>> {
    ok(state.catalog.category.relate_brand(&q).await?)
}

/// `GET /api/category/category/relateCategory`：相关分类。
///
/// 对应 Java `CategoryController.getRelateCategory(ProductRelateDTO dto)`
/// → `ProductServiceImpl.getRelateCategory`。
async fn relate_category(
    State(state): State<PublicState>,
    Query(q): Query<ProductRelateQuery>,
) -> ApiResult<Vec<RelateCategoryVO>> {
    ok(state.catalog.category.relate_category(&q).await?)
}

/// `GET /api/category/category/parentTree?id=`：父级分类链。
///
/// 对应 Java `CategoryController.parentTree(@RequestParam(value = "id",
/// required = false) Integer categoryId)` → `ProductServiceImpl.parentTree`。
/// `id` 缺失时返回空列表（Java `categoryId == null → List.of()`）。
async fn parent_tree(
    State(state): State<PublicState>,
    Query(q): Query<ChildrenCategoryQuery>,
) -> ApiResult<Vec<CategoryParentTreeVO>> {
    ok(state.catalog.category.parent_tree(q.id).await?)
}