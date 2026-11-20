//! 商品对接 API —— 管理端路由（catalog::product_feed）。
//!
//! 功能规划：`~/workspace/product-feed-api-design/proposal.md` §5。
//! 本域为新功能，无 Java 对应控制器，路径自拟（挂载于 `/adminapi`）：
//!
//! | method | path | 说明 |
//! |--------|------|------|
//! | `GET`  | `/product/feed/list` | 密钥分页列表（不含 secret） |
//! | `GET`  | `/product/feed/detail?id=` | 密钥详情（范围 + 覆盖） |
//! | `POST` | `/product/feed/create` | 新建密钥（响应一次性带 secret 明文） |
//! | `POST` | `/product/feed/update` | 编辑密钥（范围整体替换） |
//! | `POST` | `/product/feed/resetSecret` | 重置 secret（一次性带新明文） |
//! | `POST` | `/product/feed/del` | 删除密钥 |
//! | `GET`  | `/product/feed/scopeProducts` | 密钥可见商品预览（原价 vs 规则后价） |
//! | `POST` | `/product/feed/priceOverride/save` | 保存单品价格覆盖 |
//! | `POST` | `/product/feed/priceOverride/del` | 删除单品价格覆盖 |
//!
//! # 权限
//! 全局 `require_auth` 只校验管理员登录态（与全站口径一致）。

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use nextrade_core::models::catalog::product_feed::{
    FeedIdDTO, FeedKeyCreateDTO, FeedKeyDetailVO, FeedKeyListQuery, FeedKeySecretVO,
    FeedKeyUpdateDTO, FeedKeyVO, FeedPriceOverrideSaveDTO, FeedScopeProductVO,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult};

use crate::AdminState;

/// 本域路由（去掉 `/adminapi` 前缀）。
const ROUTE_TABLE: [(&str, &str, &str); 9] = [
    ("GET", "/product/feed/list", "ProductFeedController.list"),
    ("GET", "/product/feed/detail", "ProductFeedController.detail"),
    ("POST", "/product/feed/create", "ProductFeedController.create"),
    ("POST", "/product/feed/update", "ProductFeedController.update"),
    (
        "POST",
        "/product/feed/resetSecret",
        "ProductFeedController.resetSecret",
    ),
    ("POST", "/product/feed/del", "ProductFeedController.del"),
    (
        "GET",
        "/product/feed/scopeProducts",
        "ProductFeedController.scopeProducts",
    ),
    (
        "POST",
        "/product/feed/priceOverride/save",
        "ProductFeedController.savePriceOverride",
    ),
    (
        "POST",
        "/product/feed/priceOverride/del",
        "ProductFeedController.deletePriceOverride",
    ),
];

/// 商品对接管理路由（`/adminapi` 下，全局 `require_auth` 已生效）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/product/feed/list", get(list))
        .route("/product/feed/detail", get(detail))
        .route("/product/feed/create", post(create))
        .route("/product/feed/update", post(update))
        .route("/product/feed/resetSecret", post(reset_secret))
        .route("/product/feed/del", post(del))
        .route("/product/feed/scopeProducts", get(scope_products))
        .route("/product/feed/priceOverride/save", post(save_override))
        .route("/product/feed/priceOverride/del", post(delete_override))
}

/// 本域已实现路由（覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, java_ref)| {
            nextrade_openapi::RouteDoc::new(method, &format!("/adminapi{path}"), java_ref)
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct IdQuery {
    id: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScopeProductsQuery {
    feed_key_id: i32,
    page: Option<u64>,
    #[serde(alias = "size")]
    page_size: Option<u64>,
    keyword: Option<String>,
}

async fn list(
    State(state): State<AdminState>,
    Query(q): Query<FeedKeyListQuery>,
) -> ApiResult<PageResult<FeedKeyVO>> {
    ok(state.catalog.product_feed.list_keys(&q).await?)
}

async fn detail(
    State(state): State<AdminState>,
    Query(q): Query<IdQuery>,
) -> ApiResult<FeedKeyDetailVO> {
    ok(state.catalog.product_feed.detail(q.id).await?)
}

async fn create(
    State(state): State<AdminState>,
    Json(dto): Json<FeedKeyCreateDTO>,
) -> ApiResult<FeedKeySecretVO> {
    ok(state.catalog.product_feed.create(&dto).await?)
}

async fn update(
    State(state): State<AdminState>,
    Json(dto): Json<FeedKeyUpdateDTO>,
) -> ApiResult<&'static str> {
    state.catalog.product_feed.update(&dto).await?;
    ok("更新成功")
}

async fn reset_secret(
    State(state): State<AdminState>,
    Json(dto): Json<FeedIdDTO>,
) -> ApiResult<FeedKeySecretVO> {
    ok(state
        .catalog
        .product_feed
        .reset_secret(dto.feed_key_id)
        .await?)
}

async fn del(
    State(state): State<AdminState>,
    Json(dto): Json<FeedIdDTO>,
) -> ApiResult<&'static str> {
    state.catalog.product_feed.delete(dto.feed_key_id).await?;
    ok("删除成功")
}

async fn scope_products(
    State(state): State<AdminState>,
    Query(q): Query<ScopeProductsQuery>,
) -> ApiResult<PageResult<FeedScopeProductVO>> {
    ok(state
        .catalog
        .product_feed
        .scope_products(
            q.feed_key_id,
            q.page.unwrap_or(1),
            q.page_size.unwrap_or(20),
            q.keyword.as_deref(),
        )
        .await?)
}

async fn save_override(
    State(state): State<AdminState>,
    Json(dto): Json<FeedPriceOverrideSaveDTO>,
) -> ApiResult<&'static str> {
    state.catalog.product_feed.save_override(&dto).await?;
    ok("保存成功")
}

async fn delete_override(
    State(state): State<AdminState>,
    Json(dto): Json<FeedIdDTO>,
) -> ApiResult<&'static str> {
    state.catalog.product_feed.delete_override(&dto).await?;
    ok("删除成功")
}
