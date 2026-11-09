//! 商城前台分类/品牌路由。
//! 对应原 Java `com.nextrade.api.controller.product.CategoryController`
//! （`/api/category/category`）。
//!
//! | 路由 | 原 Java |
//! |------|---------|
//! | `GET /api/category/category/all` | `CategoryController.getAllCategory` |
//!
//! 原 Java 没有 `/api/category/tree`；前台分类树由 `all` 提供。

use axum::{
    extract::State,
    routing::get,
    Router,
};
use nextrade_core::models::CategoryTreeNode;
use nextrade_security::web::{ok, ApiResult};

use crate::PublicState;

/// 公开路由：前台分类树。
pub fn public_routes() -> Router<PublicState> {
    Router::new().route("/category/category/all", get(category_tree))
}

/// 需登录路由（暂无）。
pub fn authorized_routes() -> Router<PublicState> {
    Router::new()
}

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    [(
        "GET",
        "/api/category/category/all",
        "CategoryController.getAllCategory",
    )]
    .iter()
    .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, p, j))
    .collect()
}

/// `GET /api/category/category/all`：前台分类树（过滤未显示分类）。
async fn category_tree(
    State(state): State<PublicState>,
) -> ApiResult<Vec<CategoryTreeNode>> {
    let tree = state.catalog.category_all.category_tree(false).await?;
    ok(tree)
}