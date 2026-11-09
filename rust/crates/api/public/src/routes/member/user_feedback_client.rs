//! 商城前台留言路由（member::user_feedback_client）。2 端点，需登录。

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use nextrade_core::models::member::user_feedback_client::{
    FeedbackCreateDTO, FeedbackListQuery, UserFeedbackVO,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::PublicState;

const ROUTE_TABLE: [(&str, &str, &str); 2] = [
    ("GET", "/user/feedback/list", "UserFeedbackClientController.list"),
    ("POST", "/user/feedback/submit", "UserFeedbackClientController.submit"),
];

pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/user/feedback/list", get(list))
        .route("/user/feedback/submit", post(submit))
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
    Query(q): Query<FeedbackListQuery>,
) -> ApiResult<PageResult<UserFeedbackVO>> {
    ok(state.member.user_feedback_client.list(auth.id, &q).await?)
}

async fn submit(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<FeedbackCreateDTO>,
) -> ApiResult<i32> {
    ok(state.member.user_feedback_client.submit(auth.id, &dto).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_match_contract() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].path, "/api/user/feedback/list");
        assert_eq!(docs[0].method, "GET");
        assert_eq!(docs[1].path, "/api/user/feedback/submit");
        assert_eq!(docs[1].method, "POST");
    }
}
