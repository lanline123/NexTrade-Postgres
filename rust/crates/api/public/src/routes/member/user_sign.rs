//! 商城前台签到路由（member::user_sign）。2 端点，需登录。

use axum::{
    extract::State,
    routing::get,
    Extension, Router,
};
use nextrade_core::models::member::user_sign::UserSignInfoVO;
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::PublicState;

const ROUTE_TABLE: [(&str, &str, &str); 2] = [
    ("GET", "/user/sign/index", "UserSignController.getUserSignInfoVo"),
    ("GET", "/user/sign/sign", "UserSignController.sign"),
];

pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/user/sign/index", get(index))
        .route("/user/sign/sign", get(sign))
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

async fn index(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<UserSignInfoVO> {
    ok(state.member.user_sign.index(auth.id).await?)
}

async fn sign(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<()> {
    state.member.user_sign.sign(auth.id).await?;
    ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_match_contract() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].path, "/api/user/sign/index");
        assert_eq!(docs[0].method, "GET");
        assert_eq!(docs[1].path, "/api/user/sign/sign");
    }
}
