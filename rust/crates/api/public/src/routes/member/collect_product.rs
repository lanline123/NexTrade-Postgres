//! 商城前台商品收藏路由（member::collect_product）。
//! Java `com.nextrade.api.controller.user.CollectProductController`（3 端点，需登录）。

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use nextrade_core::models::member::collect_product::{
    CollectListQuery, CollectProductDTO, CollectProductVO,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::PublicState;

const ROUTE_TABLE: [(&str, &str, &str); 3] = [
    ("GET", "/user/collectProduct/list", "CollectProductController.list"),
    ("POST", "/user/collectProduct/save", "CollectProductController.save"),
    ("POST", "/user/collectProduct/cancel", "CollectProductController.cancel"),
];

/// 需登录路由（自套 require_auth，合并进 merge_public）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/user/collectProduct/list", get(list))
        .route("/user/collectProduct/save", post(save))
        .route("/user/collectProduct/cancel", post(cancel))
        .layer(axum::middleware::from_fn_with_state(
            state.jwt.clone(),
            nextrade_security::web::require_auth,
        ))
}

#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, &format!("/api{p}"), j))
        .collect()
}

async fn list(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<CollectListQuery>,
) -> ApiResult<PageResult<CollectProductVO>> {
    let _ = state;
    ok(state.member.collect_product.list(auth.id, &q).await?)
}

async fn save(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<CollectProductDTO>,
) -> ApiResult<i32> {
    ok(state.member.collect_product.save(auth.id, &dto).await?)
}

async fn cancel(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<CollectProductDTO>,
) -> ApiResult<()> {
    state.member.collect_product.cancel(auth.id, &dto).await?;
    ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_match_contract() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 3);
        assert_eq!(docs[0].path, "/api/user/collectProduct/list");
        assert_eq!(docs[1].path, "/api/user/collectProduct/save");
        assert_eq!(docs[2].path, "/api/user/collectProduct/cancel");
        assert_eq!(docs[0].method, "GET");
        assert_eq!(docs[1].method, "POST");
    }
}
