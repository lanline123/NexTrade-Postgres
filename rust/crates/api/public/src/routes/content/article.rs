//! 商城前台内容（文章）路由。
//! 对应原 Java `com.nextrade.api.controller.article.ArticleController`
//! 与 `ArticleCategoryClientController`。
//!
//! | 路由 | 原 Java |
//! |------|---------|
//! | `GET /api/article/article/list`            | `ArticleController.list`（仅已显示） |
//! | `GET /api/article/article/newsInfo`        | `ArticleController.newsInfo`（累加浏览量） |
//! | `GET /api/article/category/getArticleCategoryList` | `ArticleCategoryClientController.getArticleCategoryList` |
//! | `GET /api/article/category/list`           | `ArticleCategoryClientController.list` |
//!
//! 注意原 Java 没有 `/api/article/hot` 与文章分类 `tree` 端点；
//! `hot_articles` 目前复用 `list` 的排序实现，故不再单独注册路径。

use axum::{
    extract::{Query, State},
    routing::get,
    Router,
};
use serde::Deserialize;
use nextrade_core::models::{
    ArticleCategoryTreeNode, ArticleCategoryVO, ArticleDetailVO, ArticleListQuery, ArticleVO,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult};

use crate::PublicState;

/// 公开路由：文章列表/详情、文章分类。
pub fn public_routes() -> Router<PublicState> {
    Router::new()
        .route("/article/article/list", get(article_list))
        .route("/article/article/newsInfo", get(article_detail))
        .route("/article/category/getArticleCategoryList", get(category_list))
        .route("/article/category/list", get(category_tree))
}

/// 需登录路由（暂无）。
pub fn authorized_routes() -> Router<PublicState> {
    Router::new()
}

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    [
        ("GET", "/api/article/article/list", "ArticleController.list"),
        ("GET", "/api/article/article/newsInfo", "ArticleController.newsInfo"),
        ("GET", "/api/article/category/getArticleCategoryList", "ArticleCategoryClientController.getArticleCategoryList"),
        ("GET", "/api/article/category/list", "ArticleCategoryClientController.list"),
    ]
    .iter()
    .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, p, j))
    .collect()
}

/// `GET /api/article/list`：文章分页（前台仅返回已显示文章）。
async fn article_list(
    State(state): State<PublicState>,
    Query(q): Query<ArticleListQuery>,
) -> ApiResult<PageResult<ArticleVO>> {
    // 前台强制 is_show = 1，由服务层保证
    ok(state.content.article.list(&q, true).await?)
}

#[derive(Debug, Deserialize)]
struct IdQuery {
    id: i32,
}

/// `GET /api/article/detail?id=`：文章详情（含正文，累加浏览次数）。
async fn article_detail(
    State(state): State<PublicState>,
    Query(q): Query<IdQuery>,
) -> ApiResult<ArticleDetailVO> {
    ok(state.content.article.detail(q.id, true).await?)
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct HotQuery {
    limit: Option<u32>,
}

/// 热门文章：原 Java 无独立端点，保留服务方法供后续端点复用。
///
/// 标注 `allow(dead_code)`：当前无路由指向它，但服务层能力已具备，
/// 待对照原系统确认承载路径后即可挂载。
#[allow(dead_code)]
async fn hot_articles(
    State(state): State<PublicState>,
    Query(q): Query<HotQuery>,
) -> ApiResult<Vec<ArticleVO>> {
    ok(state.content.article.hot_articles(q.limit.unwrap_or(10)).await?)
}

/// `GET /api/article/category/getArticleCategoryList`：文章分类平铺列表。
async fn category_list(
    State(state): State<PublicState>,
) -> ApiResult<Vec<ArticleCategoryVO>> {
    ok(state.content.article.categories().await?)
}

/// `GET /api/article/category/list`：文章分类树。
async fn category_tree(
    State(state): State<PublicState>,
) -> ApiResult<Vec<ArticleCategoryTreeNode>> {
    ok(state.content.article.category_tree().await?)
}