//! 商城前台实名认证路由（member::user_company_client）。3 端点，需登录。

use axum::{
    extract::State,
    routing::{get, post},
    Extension, Json, Router,
};
use nextrade_core::models::member::user_company_client::{UserCompanyDTO, UserCompanyVO};
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::PublicState;

const ROUTE_TABLE: [(&str, &str, &str); 3] = [
    ("POST", "/user/company/apply", "UserCompanyClientController.apply"),
    ("GET", "/user/company/detail", "UserCompanyClientController.detail"),
    ("GET", "/user/company/myApply", "UserCompanyClientController.myApply"),
];

pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/user/company/apply", post(apply))
        .route("/user/company/detail", get(detail))
        .route("/user/company/myApply", get(my_apply))
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

async fn apply(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<UserCompanyDTO>,
) -> ApiResult<i32> {
    ok(state.member.user_company_client.apply(auth.id, &dto).await?)
}

async fn detail(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<Option<UserCompanyVO>> {
    ok(state.member.user_company_client.detail(auth.id).await?)
}

async fn my_apply(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<Vec<UserCompanyVO>> {
    ok(state.member.user_company_client.my_apply(auth.id).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_match_contract() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 3);
        assert_eq!(docs[0].path, "/api/user/company/apply");
        assert_eq!(docs[0].method, "POST");
        assert_eq!(docs[1].path, "/api/user/company/detail");
        assert_eq!(docs[2].path, "/api/user/company/myApply");
    }
}
