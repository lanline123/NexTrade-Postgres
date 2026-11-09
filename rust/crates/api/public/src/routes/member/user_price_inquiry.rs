//! 商城前台询价路由（member::user_price_inquiry）。4 端点，需登录。

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use nextrade_core::models::member::user_price_inquiry::{
    OperateDTO, PriceInquiryDetailVO, PriceInquiryListQuery, PriceInquiryReplyDTO, PriceInquiryVO,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::PublicState;

const ROUTE_TABLE: [(&str, &str, &str); 4] = [
    ("GET", "/user/priceInquiry/list", "UserPriceInquiryController.list"),
    ("GET", "/user/priceInquiry/detail", "UserPriceInquiryController.detail"),
    ("POST", "/user/priceInquiry/reply", "UserPriceInquiryController.reply"),
    ("POST", "/user/priceInquiry/del", "UserPriceInquiryController.del"),
];

pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/user/priceInquiry/list", get(list))
        .route("/user/priceInquiry/detail", get(detail))
        .route("/user/priceInquiry/reply", post(reply))
        .route("/user/priceInquiry/del", post(del))
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

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct IdQuery {
    id: Option<i32>,
}

async fn list(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<PriceInquiryListQuery>,
) -> ApiResult<PageResult<PriceInquiryVO>> {
    ok(state.member.user_price_inquiry.list(auth.id, &q).await?)
}

async fn detail(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<IdQuery>,
) -> ApiResult<PriceInquiryDetailVO> {
    ok(state.member.user_price_inquiry.detail(q.id, auth.id).await?)
}

async fn reply(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<PriceInquiryReplyDTO>,
) -> ApiResult<()> {
    state.member.user_price_inquiry.reply(auth.id, &dto).await?;
    ok(())
}

async fn del(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<OperateDTO>,
) -> ApiResult<()> {
    state.member.user_price_inquiry.del(auth.id, dto.id).await?;
    ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_match_contract() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 4);
        assert_eq!(docs[0].path, "/api/user/priceInquiry/list");
        assert_eq!(docs[2].path, "/api/user/priceInquiry/reply");
        assert_eq!(docs[3].method, "POST");
    }
}
