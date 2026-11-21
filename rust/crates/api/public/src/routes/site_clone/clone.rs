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
//! | `GET` | `/clone/v1/decorate` | decorate | 装修页+离散组件+导航三表（P2） |
//! | `GET` | `/clone/v1/assets` | assets | 素材分组与条目 URL 清单（P2） |
//! | `GET` | `/clone/v1/marketing` | marketing | 营销模板全集（P3，不含已发放实例） |
//! | `GET` | `/clone/v1/content` | content | 文章分类/文章/关联/友链（P3） |
//! | `GET` | `/clone/v1/mail` | mail | 邮件模板+消息类型开关（P3） |
//! | `GET` | `/clone/v1/shops` | shops | 店铺壳+分销规则（P3） |
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
    CloneAssetsPackage, CloneCatalogVO, CloneContentPackage, CloneDecoratePackage,
    CloneMailPackage, CloneManifestVO, CloneMarketingPackage, CloneProduct, CloneSettingsPackage,
    CloneShopsPackage, SiteCloneKey, DOMAIN_ASSETS, DOMAIN_CONTENT, DOMAIN_DECORATE, DOMAIN_MAIL,
    DOMAIN_MARKETING, DOMAIN_PRODUCTS, DOMAIN_SETTINGS, DOMAIN_SHOPS, MSG_CLONE_UNAUTHORIZED,
};
use nextrade_core::types::PageResult;
use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, AppError};

use super::super::super::PublicState;

/// 本域路由（去掉 `/api` 前缀）。
const ROUTE_TABLE: [(&str, &str, &str); 10] = [
    ("GET", "/clone/v1/manifest", "CloneApi.manifest"),
    ("GET", "/clone/v1/catalog", "CloneApi.catalog"),
    ("GET", "/clone/v1/products", "CloneApi.products"),
    ("GET", "/clone/v1/settings", "CloneApi.settings"),
    ("GET", "/clone/v1/decorate", "CloneApi.decorate"),
    ("GET", "/clone/v1/assets", "CloneApi.assets"),
    ("GET", "/clone/v1/marketing", "CloneApi.marketing"),
    ("GET", "/clone/v1/content", "CloneApi.content"),
    ("GET", "/clone/v1/mail", "CloneApi.mail"),
    ("GET", "/clone/v1/shops", "CloneApi.shops"),
];

/// 搬家面路由（自带密钥鉴权）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .route("/clone/v1/manifest", get(manifest))
        .route("/clone/v1/catalog", get(catalog))
        .route("/clone/v1/products", get(products))
        .route("/clone/v1/settings", get(settings))
        .route("/clone/v1/decorate", get(decorate))
        .route("/clone/v1/assets", get(assets))
        .route("/clone/v1/marketing", get(marketing))
        .route("/clone/v1/content", get(content))
        .route("/clone/v1/mail", get(mail))
        .route("/clone/v1/shops", get(shops))
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

async fn decorate(
    State(state): State<PublicState>,
    headers: HeaderMap,
) -> ApiResult<CloneDecoratePackage> {
    let started = Instant::now();
    let key = auth_key(&state, &headers, DOMAIN_DECORATE).await?;
    let pkg = state.site_clone.clone.decorate_package(&key).await.map_err(AppError)?;
    let count = i32::try_from(
        pkg.pages.len() + pkg.discrete.len() + pkg.navigations.len() + pkg.cat_floors.len()
            + pkg.mobile_cat_navs.len(),
    )
    .unwrap_or(i32::MAX);
    state
        .site_clone
        .clone
        .record_call(&key, "/api/clone/v1/decorate", "decorate", 200, count, &client_ip(&headers), started)
        .await;
    ok(pkg)
}

async fn assets(
    State(state): State<PublicState>,
    headers: HeaderMap,
) -> ApiResult<CloneAssetsPackage> {
    let started = Instant::now();
    let key = auth_key(&state, &headers, DOMAIN_ASSETS).await?;
    let pkg = state.site_clone.clone.assets_package(&key).await.map_err(AppError)?;
    let count =
        i32::try_from(pkg.groups.len() + pkg.pics.len() + pkg.video_groups.len() + pkg.videos.len())
            .unwrap_or(i32::MAX);
    state
        .site_clone
        .clone
        .record_call(&key, "/api/clone/v1/assets", "assets", 200, count, &client_ip(&headers), started)
        .await;
    ok(pkg)
}

async fn marketing(
    State(state): State<PublicState>,
    headers: HeaderMap,
) -> ApiResult<CloneMarketingPackage> {
    let started = Instant::now();
    let key = auth_key(&state, &headers, DOMAIN_MARKETING).await?;
    let pkg = state.site_clone.clone.marketing_package(&key).await.map_err(AppError)?;
    let count = i32::try_from(
        pkg.coupons.len()
            + pkg.seckills.len()
            + pkg.groupons.len()
            + pkg.time_discounts.len()
            + pkg.product_promotions.len()
            + pkg.gift_card_groups.len()
            + pkg.lotteries.len(),
    )
    .unwrap_or(i32::MAX);
    state
        .site_clone
        .clone
        .record_call(&key, "/api/clone/v1/marketing", "marketing", 200, count, &client_ip(&headers), started)
        .await;
    ok(pkg)
}

async fn content(
    State(state): State<PublicState>,
    headers: HeaderMap,
) -> ApiResult<CloneContentPackage> {
    let started = Instant::now();
    let key = auth_key(&state, &headers, DOMAIN_CONTENT).await?;
    let pkg = state.site_clone.clone.content_package().await.map_err(AppError)?;
    let count = i32::try_from(pkg.articles.len() + pkg.friend_links.len()).unwrap_or(i32::MAX);
    state
        .site_clone
        .clone
        .record_call(&key, "/api/clone/v1/content", "content", 200, count, &client_ip(&headers), started)
        .await;
    ok(pkg)
}

async fn mail(
    State(state): State<PublicState>,
    headers: HeaderMap,
) -> ApiResult<CloneMailPackage> {
    let started = Instant::now();
    let key = auth_key(&state, &headers, DOMAIN_MAIL).await?;
    let pkg = state.site_clone.clone.mail_package().await.map_err(AppError)?;
    let count = i32::try_from(pkg.templates.len() + pkg.message_types.len()).unwrap_or(i32::MAX);
    state
        .site_clone
        .clone
        .record_call(&key, "/api/clone/v1/mail", "mail", 200, count, &client_ip(&headers), started)
        .await;
    ok(pkg)
}

async fn shops(
    State(state): State<PublicState>,
    headers: HeaderMap,
) -> ApiResult<CloneShopsPackage> {
    let started = Instant::now();
    let key = auth_key(&state, &headers, DOMAIN_SHOPS).await?;
    let pkg = state.site_clone.clone.shops_package().await.map_err(AppError)?;
    let count = i32::try_from(pkg.shops.len()).unwrap_or(i32::MAX);
    state
        .site_clone
        .clone
        .record_call(&key, "/api/clone/v1/shops", "shops", 200, count, &client_ip(&headers), started)
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
