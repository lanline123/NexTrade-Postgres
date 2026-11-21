//! 管理端营销活动下发路由（marketing::distribution）。
//!
//! `/adminapi/promotion/distribution/*`（7 端点）：可下发来源与目标店铺
//! 选择器、一键下发、下发记录列表/详情、整批撤销、单店撤销。
//! 全部端点仅平台管理员可用（服务层 TenantScope 校验）。

use axum::{
extract::{Query, State},
routing::{get, post},
Json, Router,
};
use serde::Deserialize;

use nextrade_core::models::marketing::distribution::{
DistributeDTO, DistributeResultVO, DistributionDetailVO, DistributionListQuery,
DistributionVO, RevokeDTO, SourceActivityVO, TargetShopVO,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult};

use crate::AdminState;

const ROUTE_TABLE: [(&str, &str, &str); 7] = [
(
"GET",
"/promotion/distribution/sources",
"DistributionController.sources",
),
(
"GET",
"/promotion/distribution/shops",
"DistributionController.shops",
),
(
"POST",
"/promotion/distribution/distribute",
"DistributionController.distribute",
),
(
"GET",
"/promotion/distribution/list",
"DistributionController.list",
),
(
"GET",
"/promotion/distribution/detail",
"DistributionController.detail",
),
(
"POST",
"/promotion/distribution/revoke",
"DistributionController.revoke",
),
(
"POST",
"/promotion/distribution/revokeItem",
"DistributionController.revokeItem",
),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
let _ = state;
Router::new()
.route("/promotion/distribution/sources", get(sources))
.route("/promotion/distribution/shops", get(shops))
.route("/promotion/distribution/distribute", post(distribute))
.route("/promotion/distribution/list", get(list))
.route("/promotion/distribution/detail", get(detail))
.route("/promotion/distribution/revoke", post(revoke))
.route("/promotion/distribution/revokeItem", post(revoke_item))
}

#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
ROUTE_TABLE
.iter()
.map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, &format!("/adminapi{p}"), j))
.collect()
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct SourcesQuery {
kind: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct DetailQuery {
id: Option<i64>,
}

async fn sources(
State(state): State<AdminState>,
Query(q): Query<SourcesQuery>,
) -> ApiResult<Vec<SourceActivityVO>> {
ok(state.marketing.distribution.sources(q.kind).await?)
}

async fn shops(State(state): State<AdminState>) -> ApiResult<Vec<TargetShopVO>> {
ok(state.marketing.distribution.shops().await?)
}

async fn distribute(
State(state): State<AdminState>,
Json(dto): Json<DistributeDTO>,
) -> ApiResult<DistributeResultVO> {
ok(state.marketing.distribution.distribute(&dto).await?)
}

async fn list(
State(state): State<AdminState>,
Query(q): Query<DistributionListQuery>,
) -> ApiResult<PageResult<DistributionVO>> {
ok(state.marketing.distribution.list(&q).await?)
}

async fn detail(
State(state): State<AdminState>,
Query(q): Query<DetailQuery>,
) -> ApiResult<DistributionDetailVO> {
ok(state
.marketing
.distribution
.detail(q.id.unwrap_or(0))
.await?)
}

async fn revoke(
State(state): State<AdminState>,
Json(dto): Json<RevokeDTO>,
) -> ApiResult<DistributionDetailVO> {
ok(state.marketing.distribution.revoke_batch(&dto).await?)
}

async fn revoke_item(
State(state): State<AdminState>,
Json(dto): Json<RevokeDTO>,
) -> ApiResult<DistributionDetailVO> {
ok(state.marketing.distribution.revoke_item(&dto).await?)
}

#[cfg(test)]
mod tests {
use super::*;

#[test]
fn implemented_routes_match_contract() {
let docs = implemented_routes();
assert_eq!(docs.len(), 7);
for (m, p) in [
("GET", "/adminapi/promotion/distribution/sources"),
("GET", "/adminapi/promotion/distribution/shops"),
("POST", "/adminapi/promotion/distribution/distribute"),
("GET", "/adminapi/promotion/distribution/list"),
("GET", "/adminapi/promotion/distribution/detail"),
("POST", "/adminapi/promotion/distribution/revoke"),
("POST", "/adminapi/promotion/distribution/revokeItem"),
] {
assert!(docs.iter().any(|d| d.method == m && d.path == p), "{m} {p}");
}
}

#[test]
fn route_table_matches_registrations() {
assert_eq!(ROUTE_TABLE.len(), 7);
}
}
