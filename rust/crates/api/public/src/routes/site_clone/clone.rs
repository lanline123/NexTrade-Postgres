//! 站点搬家面路由（site_clone::clone，挂载于 `/api/clone/v1`）。
//!
//! 设计：`~/workspace/site-clone-design/REPORT.md` §5.2。
//!
//! | method | path | 授权域 | 说明 |
//! |--------|------|--------|------|
//! | `GET` | `/clone/v1/manifest` | （任一） | 源站元信息/各域计数/豁免声明 |
//! | `GET` | `/clone/v1/catalog` | products | 分类+品牌+运费模板 |
//! | `GET` | `/clone/v1/products?page&size` | products | 商品全保真分页 |
//! | `GET` | `/clone/v1/settings` | settings | 设置包（仅白名单） |
//!
//! 每次带钥调用落 `site_clone_call_log`（密钥/端点/域/状态/条数/耗时）。

use std::time::Instant;

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::get,
    Router,
};
use serde::Deserialize;

use nextrade_core::models::site_clone::clone::{
    CloneCatalogVO, CloneManifestVO, CloneProduct, CloneSettingsPackage, SiteCloneKey,
    DOMAIN_PRODUCTS, DOMAIN_SETTINGS, MSG_CLONE_UNAUTHORIZED,
};
use nextrade_core::types::PageResult;
use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, AppError};

use super::super::super::PublicState;

/// 本域路由（去掉 `/api` 前缀）。
const ROUTE_TABLE: [(&str, &str, &str); 4] = [
    ("GET", "/clone/v1/manifest", "CloneApi.manifest"),
    ("GET", "/clone/v1/catalog", "CloneApi.catalog"),
    ("GET", "/clone/v1/products", "CloneApi.products"),
    ("GET", "/clone/v1/settings", "CloneApi.settings"),
];

/// 搬家面路由（自带密钥鉴权）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .route("/clone/v1/manifest", get(manifest))
        .route("/clone/v1/catalog", get(catalog))
        .route("/clone/v1/products", get(products))
        .route("/clone/v1/settings", get(settings))
}

/// 本域已实现路由（覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| {
            nextrade_openapi::RouteDoc::new(method, &format!("/api{path}"), handler)
        })
        .collect()
}

/// 从请求头取搬家密钥凭据（缺任一 → 401 统一文案）。
fn extract_creds(headers: &HeaderMap) -> Result<(String, String), AppError> {
    let get = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    match (get("x-clone-key-id"), get("x-clone-secret")) {
        (Some(k), Some(s)) => Ok((k, s)),
        _ => Err(AppError(Error::Unauthorized(MSG_CLONE_UNAUTHORIZED.into()))),
    }
}

/// 客户端 IP（取 X-Forwarded-For 首跳，仅日志用）。
fn client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// 鉴权（指定域，空域 = 只验密钥本身）并返回密钥行。
async fn auth_key(
    state: &PublicState,
    headers: &HeaderMap,
    domain: &str,
) -> Result<SiteCloneKey, AppError> {
    let (key_code, secret) = extract_creds(headers)?;
    state
        .site_clone
        .clone
        .authenticate(&key_code, &secret, domain)
        .await
        .map_err(AppError)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProductsQuery {
    page: Option<u64>,
    size: Option<u64>,
}

async fn manifest(
    State(state): State<PublicState>,
    headers: HeaderMap,
) -> ApiResult<CloneManifestVO> {
    let started = Instant::now();
    let key = auth_key(&state, &headers, "").await?;
    let vo = state.site_clone.clone.manifest(&key).await.map_err(AppError)?;
    state
        .site_clone
        .clone
        .record_call(&key, "/api/clone/v1/manifest", "manifest", 200, 1, &client_ip(&headers), started)
        .await;
    ok(vo)
}

async fn catalog(
    State(state): State<PublicState>,
    headers: HeaderMap,
) -> ApiResult<CloneCatalogVO> {
    let started = Instant::now();
    let key = auth_key(&state, &headers, DOMAIN_PRODUCTS).await?;
    let vo = state.site_clone.clone.catalog(&key).await.map_err(AppError)?;
    let count = i32::try_from(vo.categories.len() + vo.brands.len() + vo.shipping_tpls.len())
        .unwrap_or(i32::MAX);
    state
        .site_clone
        .clone
        .record_call(&key, "/api/clone/v1/catalog", "catalog", 200, count, &client_ip(&headers), started)
        .await;
    ok(vo)
}

async fn products(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Query(q): Query<ProductsQuery>,
) -> ApiResult<PageResult<CloneProduct>> {
    let started = Instant::now();
    let key = auth_key(&state, &headers, DOMAIN_PRODUCTS).await?;
    let page = state
        .site_clone
        .clone
        .products_page(&key, q.page.unwrap_or(1), q.size.unwrap_or(50))
        .await
        .map_err(AppError)?;
    let count = i32::try_from(page.total).unwrap_or(i32::MAX);
    state
        .site_clone
        .clone
        .record_call(&key, "/api/clone/v1/products", "products", 200, count, &client_ip(&headers), started)
        .await;
    ok(page)
}

async fn settings(
    State(state): State<PublicState>,
    headers: HeaderMap,
) -> ApiResult<CloneSettingsPackage> {
    let started = Instant::now();
    let key = auth_key(&state, &headers, DOMAIN_SETTINGS).await?;
    let pkg = state.site_clone.clone.settings_package().await.map_err(AppError)?;
    let count = i32::try_from(pkg.groups.len() + pkg.blocks.len()).unwrap_or(i32::MAX);
    state
        .site_clone
        .clone
        .record_call(&key, "/api/clone/v1/settings", "settings", 200, count, &client_ip(&headers), started)
        .await;
    ok(pkg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract_exactly() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), ROUTE_TABLE.len());
        for (doc, (method, path, _)) in docs.iter().zip(ROUTE_TABLE.iter()) {
            assert_eq!(doc.method, *method);
            assert_eq!(doc.path, format!("/api{path}"));
        }
    }
}
