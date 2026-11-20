//! 平台联邦面路由（hub::federation，挂载于 `/api/hub/v1`）。
//!
//! 设计：`~/workspace/hub-federation-design/proposal.md` §3.3/§4/§5/§8。
//!
//! | method | path | 鉴权 | 说明 |
//! |--------|------|------|------|
//! | `POST` | `/hub/v1/pair/claim` | 配对码 | 成员发起配对 |
//! | `POST` | `/hub/v1/pair/status` | claim token | 查审批状态 |
//! | `POST` | `/hub/v1/pair/activate` | claim token | 领取上行钥（一次性） |
//! | `GET`  | `/hub/v1/meta` | meta.read | 实例元信息/凭证自检 |
//! | `GET`  | `/hub/v1/categories` | product.read | 授权分类树 |
//! | `GET`  | `/hub/v1/products` | product.read | 授权商品分页 |
//! | `GET`  | `/hub/v1/products/detail?productId=` | product.read | 商品详情（越权 404） |
//! | `POST` | `/hub/v1/heartbeat` | stats.report | 成员心跳上报 |
//! | `POST` | `/hub/v1/link/updateEndpoint` | meta.read | 地址变更报备 |
//!
//! 每次带钥调用落 `hub_call_log`（凭证/端点/状态/条数/耗时）。

use std::time::Instant;

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use nextrade_core::models::catalog::product_feed::{
    FeedCategoryVO, FeedProductDetailVO, FeedProductListQuery, FeedProductVO,
};
use nextrade_core::models::hub::federation::{
    HeartbeatDTO, HeartbeatVO, HubCredential, HubMetaVO, PairActivateVO, PairClaimDTO,
    PairClaimVO, PairStatusDTO, PairStatusVO, UpdateEndpointDTO, MSG_HUB_UNAUTHORIZED,
    SCOPE_META_READ, SCOPE_PRODUCT_READ, SCOPE_STATS_REPORT,
};
use nextrade_core::types::PageResult;
use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, AppError};

use super::super::super::PublicState;

/// 本域路由（去掉 `/api` 前缀）。
const ROUTE_TABLE: [(&str, &str, &str); 9] = [
    ("POST", "/hub/v1/pair/claim", "HubApi.pairClaim"),
    ("POST", "/hub/v1/pair/status", "HubApi.pairStatus"),
    ("POST", "/hub/v1/pair/activate", "HubApi.pairActivate"),
    ("GET", "/hub/v1/meta", "HubApi.meta"),
    ("GET", "/hub/v1/categories", "HubApi.categories"),
    ("GET", "/hub/v1/products", "HubApi.products"),
    ("GET", "/hub/v1/products/detail", "HubApi.productDetail"),
    ("POST", "/hub/v1/heartbeat", "HubApi.heartbeat"),
    ("POST", "/hub/v1/link/updateEndpoint", "HubApi.updateEndpoint"),
];

/// 联邦面路由（自带凭证鉴权）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .route("/hub/v1/pair/claim", post(pair_claim))
        .route("/hub/v1/pair/status", post(pair_status))
        .route("/hub/v1/pair/activate", post(pair_activate))
        .route("/hub/v1/meta", get(meta))
        .route("/hub/v1/categories", get(categories))
        .route("/hub/v1/products", get(products))
        .route("/hub/v1/products/detail", get(product_detail))
        .route("/hub/v1/heartbeat", post(heartbeat))
        .route("/hub/v1/link/updateEndpoint", post(update_endpoint))
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

/// 从请求头取联邦凭据（缺任一 → 401 统一文案）。
fn extract_creds(headers: &HeaderMap) -> Result<(String, String), AppError> {
    let get = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    match (get("x-fed-access-key"), get("x-fed-secret")) {
        (Some(ak), Some(sk)) => Ok((ak, sk)),
        _ => Err(AppError(Error::Unauthorized(MSG_HUB_UNAUTHORIZED.into()))),
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

/// 鉴权（指定 scope）并返回凭证行。
async fn auth_cred(
    state: &PublicState,
    headers: &HeaderMap,
    scope: i64,
) -> Result<HubCredential, AppError> {
    let (ak, sk) = extract_creds(headers)?;
    let (cred, _link, _settings) = state
        .hub
        .federation
        .authenticate(&ak, &sk, scope)
        .await
        .map_err(AppError)?;
    Ok(cred)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DetailQuery {
    product_id: i32,
}

// ———————————————————— 配对三件套（无钥，凭配对码/claim token） ————————————————————

async fn pair_claim(
    State(state): State<PublicState>,
    Json(dto): Json<PairClaimDTO>,
) -> ApiResult<PairClaimVO> {
    ok(state.hub.federation.pair_claim(&dto).await?)
}

async fn pair_status(
    State(state): State<PublicState>,
    Json(dto): Json<PairStatusDTO>,
) -> ApiResult<PairStatusVO> {
    ok(state.hub.federation.pair_status(&dto).await?)
}

async fn pair_activate(
    State(state): State<PublicState>,
    Json(dto): Json<PairStatusDTO>,
) -> ApiResult<PairActivateVO> {
    ok(state.hub.federation.pair_activate(&dto).await?)
}

// ———————————————————— 带钥接口 ————————————————————

async fn meta(
    State(state): State<PublicState>,
    headers: HeaderMap,
) -> ApiResult<HubMetaVO> {
    let started = Instant::now();
    let (ak, sk) = extract_creds(&headers)?;
    let (cred, _link, settings) = state
        .hub
        .federation
        .authenticate(&ak, &sk, SCOPE_META_READ)
        .await
        .map_err(AppError)?;
    let vo = state
        .hub
        .federation
        .hub_meta(&cred, &settings)
        .await
        .map_err(AppError)?;
    state
        .hub
        .federation
        .record_call(&cred, "/api/hub/v1/meta", "meta.read", 200, 1, &client_ip(&headers), started)
        .await;
    ok(vo)
}

async fn categories(
    State(state): State<PublicState>,
    headers: HeaderMap,
) -> ApiResult<Vec<FeedCategoryVO>> {
    let started = Instant::now();
    let cred = auth_cred(&state, &headers, SCOPE_PRODUCT_READ).await?;
    let list = state
        .hub
        .federation
        .hub_categories(&cred)
        .await
        .map_err(AppError)?;
    let count = i32::try_from(list.len()).unwrap_or(i32::MAX);
    state
        .hub
        .federation
        .record_call(&cred, "/api/hub/v1/categories", "product.read", 200, count, &client_ip(&headers), started)
        .await;
    ok(list)
}

async fn products(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Query(q): Query<FeedProductListQuery>,
) -> ApiResult<PageResult<FeedProductVO>> {
    let started = Instant::now();
    let cred = auth_cred(&state, &headers, SCOPE_PRODUCT_READ).await?;
    let page = state
        .hub
        .federation
        .hub_products(&cred, &q)
        .await
        .map_err(AppError)?;
    let count = i32::try_from(page.total).unwrap_or(i32::MAX);
    state
        .hub
        .federation
        .record_call(&cred, "/api/hub/v1/products", "product.read", 200, count, &client_ip(&headers), started)
        .await;
    ok(page)
}

async fn product_detail(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Query(q): Query<DetailQuery>,
) -> ApiResult<FeedProductDetailVO> {
    let started = Instant::now();
    let cred = auth_cred(&state, &headers, SCOPE_PRODUCT_READ).await?;
    let detail = state
        .hub
        .federation
        .hub_product_detail(&cred, q.product_id)
        .await
        .map_err(AppError)?;
    state
        .hub
        .federation
        .record_call(&cred, "/api/hub/v1/products/detail", "product.read", 200, 1, &client_ip(&headers), started)
        .await;
    ok(detail)
}

async fn heartbeat(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Json(dto): Json<HeartbeatDTO>,
) -> ApiResult<HeartbeatVO> {
    let started = Instant::now();
    let cred = auth_cred(&state, &headers, SCOPE_STATS_REPORT).await?;
    let vo = state
        .hub
        .federation
        .receive_heartbeat(&cred, &dto)
        .await
        .map_err(AppError)?;
    state
        .hub
        .federation
        .record_call(&cred, "/api/hub/v1/heartbeat", "stats.report", 200, 1, &client_ip(&headers), started)
        .await;
    ok(vo)
}

async fn update_endpoint(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Json(dto): Json<UpdateEndpointDTO>,
) -> ApiResult<&'static str> {
    let cred = auth_cred(&state, &headers, SCOPE_META_READ).await?;
    state
        .hub
        .federation
        .update_endpoint(&cred, &dto)
        .await
        .map_err(AppError)?;
    ok("地址已更新")
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
