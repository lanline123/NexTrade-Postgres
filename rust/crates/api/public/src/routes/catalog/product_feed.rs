//! 商品对接 API —— 对接面路由（catalog::product_feed）。
//!
//! 功能规划：`~/workspace/product-feed-api-design/proposal.md` §4。
//! 挂载于 `/api` 前缀下（由 `routes::merge_public` 的 anonymous 通道并入，
//! 外层不套会员 JWT），鉴权走对接密钥双请求头：
//! `X-Feed-Access-Key` + `X-Feed-Secret`。
//!
//! | method | path | 说明 |
//! |--------|------|------|
//! | `GET` | `/feed/v1/meta` | 密钥自检/元信息 |
//! | `GET` | `/feed/v1/categories` | 授权分类树 |
//! | `GET` | `/feed/v1/products` | 授权商品分页（支持 updated_since 增量） |
//! | `GET` | `/feed/v1/products/detail?productId=` | 商品详情（越权 404） |

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::get,
    Router,
};
use serde::Deserialize;

use nextrade_core::models::catalog::product_feed::{
    FeedCategoryVO, FeedKey, FeedMetaVO, FeedProductDetailVO, FeedProductListQuery, FeedProductVO,
    MSG_FEED_UNAUTHORIZED,
};
use nextrade_core::types::PageResult;
use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, AppError};

use super::super::super::PublicState;

/// 本域路由（去掉 `/api` 前缀）。
const ROUTE_TABLE: [(&str, &str, &str); 4] = [
    ("GET", "/feed/v1/meta", "ProductFeedApi.meta"),
    ("GET", "/feed/v1/categories", "ProductFeedApi.categories"),
    ("GET", "/feed/v1/products", "ProductFeedApi.products"),
    ("GET", "/feed/v1/products/detail", "ProductFeedApi.productDetail"),
];

/// 对接面路由（自带密钥鉴权）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .route("/feed/v1/meta", get(meta))
        .route("/feed/v1/categories", get(categories))
        .route("/feed/v1/products", get(products))
        .route("/feed/v1/products/detail", get(product_detail))
}

/// 本域已实现路由（覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, java_ref)| {
            nextrade_openapi::RouteDoc::new(method, &format!("/api{path}"), java_ref)
        })
        .collect()
}

/// 从请求头取对接凭据（缺任一 → 401 统一文案）。
fn extract_creds(headers: &HeaderMap) -> Result<(String, String), AppError> {
    let get = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    match (get("x-feed-access-key"), get("x-feed-secret")) {
        (Some(ak), Some(sk)) => Ok((ak, sk)),
        _ => Err(AppError(Error::Unauthorized(MSG_FEED_UNAUTHORIZED.into()))),
    }
}

/// 鉴权并返回密钥行（各 handler 共用）。
async fn auth_key(state: &PublicState, headers: &HeaderMap) -> Result<FeedKey, AppError> {
    let (ak, sk) = extract_creds(headers)?;
    state
        .catalog
        .product_feed
        .authenticate(&ak, &sk)
        .await
        .map_err(AppError)
}

async fn meta(
    State(state): State<PublicState>,
    headers: HeaderMap,
) -> ApiResult<FeedMetaVO> {
    let key = auth_key(&state, &headers).await?;
    ok(state.catalog.product_feed.meta(&key).await?)
}

async fn categories(
    State(state): State<PublicState>,
    headers: HeaderMap,
) -> ApiResult<Vec<FeedCategoryVO>> {
    let key = auth_key(&state, &headers).await?;
    ok(state.catalog.product_feed.categories(&key).await?)
}

async fn products(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Query(q): Query<FeedProductListQuery>,
) -> ApiResult<PageResult<FeedProductVO>> {
    let key = auth_key(&state, &headers).await?;
    ok(state.catalog.product_feed.products(&key, &q).await?)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DetailQuery {
    #[serde(alias = "id")]
    product_id: Option<i32>,
}

async fn product_detail(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Query(q): Query<DetailQuery>,
) -> ApiResult<FeedProductDetailVO> {
    let key = auth_key(&state, &headers).await?;
    let product_id = q
        .product_id
        .filter(|id| *id > 0)
        .ok_or_else(|| AppError(Error::Validation("productId 不能为空".into())))?;
    ok(state
        .catalog
        .product_feed
        .product_detail(&key, product_id)
        .await?)
}
