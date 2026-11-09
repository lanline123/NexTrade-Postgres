//! 商城前台增票资质路由（member::user_invoice_client）。4 端点，需登录。

use axum::{
    extract::State,
    routing::{get, post},
    Extension, Json, Router,
};
use nextrade_core::models::member::user_invoice_client::{UserInvoiceDTO, UserInvoiceVO};
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::PublicState;

const ROUTE_TABLE: [(&str, &str, &str); 4] = [
    ("GET", "/user/invoice/getStatus", "UserInvoiceClientController.getStatus"),
    ("GET", "/user/invoice/detail", "UserInvoiceClientController.detail"),
    ("POST", "/user/invoice/create", "UserInvoiceClientController.create"),
    ("POST", "/user/invoice/update", "UserInvoiceClientController.update"),
];

pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/user/invoice/getStatus", get(get_status))
        .route("/user/invoice/detail", get(detail))
        .route("/user/invoice/create", post(create))
        .route("/user/invoice/update", post(update))
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

async fn get_status(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<Option<i32>> {
    ok(state.member.user_invoice_client.get_status(auth.id).await?)
}

async fn detail(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<Option<UserInvoiceVO>> {
    ok(state.member.user_invoice_client.detail(auth.id).await?)
}

async fn create(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<UserInvoiceDTO>,
) -> ApiResult<i32> {
    ok(state.member.user_invoice_client.create(auth.id, &dto).await?)
}

async fn update(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<UserInvoiceDTO>,
) -> ApiResult<()> {
    state.member.user_invoice_client.update(auth.id, &dto).await?;
    ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_match_contract() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 4);
        assert_eq!(docs[0].path, "/api/user/invoice/getStatus");
        assert_eq!(docs[2].path, "/api/user/invoice/create");
        assert_eq!(docs[3].method, "POST");
    }
}
