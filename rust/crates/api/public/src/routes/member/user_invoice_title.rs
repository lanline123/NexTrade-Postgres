//! 商城前台常用发票抬头路由（member::user_invoice_title）。
//! 5 端点，需登录。Tigshop v5.8.65「常用发票抬头管理」跟进（原版有，我方自研实现），
//! Java 原版无此控制器，路由风格对齐 `user_invoice_client`（`/user/invoice/*`）。

use axum::{
    extract::State,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
use nextrade_core::models::member::user_invoice_title::{
    UserInvoiceTitleDTO, UserInvoiceTitleVO,
};
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::PublicState;

const ROUTE_TABLE: [(&str, &str, &str); 5] = [
    ("GET", "/user/invoiceTitle/list", "UserInvoiceTitleClientController.list"),
    ("POST", "/user/invoiceTitle/create", "UserInvoiceTitleClientController.create"),
    ("POST", "/user/invoiceTitle/update", "UserInvoiceTitleClientController.update"),
    ("POST", "/user/invoiceTitle/del", "UserInvoiceTitleClientController.del"),
    ("POST", "/user/invoiceTitle/setDefault", "UserInvoiceTitleClientController.setDefault"),
];

pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/user/invoiceTitle/list", get(list))
        .route("/user/invoiceTitle/create", post(create))
        .route("/user/invoiceTitle/update", post(update))
        .route("/user/invoiceTitle/del", post(del))
        .route("/user/invoiceTitle/setDefault", post(set_default))
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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct TitleIdParam {
    id: Option<i32>,
}

async fn list(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<Vec<UserInvoiceTitleVO>> {
    ok(state.member.user_invoice_title.list(auth.id).await?)
}

async fn create(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<UserInvoiceTitleDTO>,
) -> ApiResult<i32> {
    ok(state.member.user_invoice_title.create(auth.id, &dto).await?)
}

async fn update(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<UserInvoiceTitleDTO>,
) -> ApiResult<()> {
    state.member.user_invoice_title.update(auth.id, &dto).await?;
    ok(())
}

async fn del(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(param): Json<TitleIdParam>,
) -> ApiResult<()> {
    state
        .member
        .user_invoice_title
        .del(auth.id, param.id.unwrap_or(0))
        .await?;
    ok(())
}

async fn set_default(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(param): Json<TitleIdParam>,
) -> ApiResult<()> {
    state
        .member
        .user_invoice_title
        .set_default(auth.id, param.id.unwrap_or(0))
        .await?;
    ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_match_contract() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 5);
        assert_eq!(docs[0].path, "/api/user/invoiceTitle/list");
        assert_eq!(docs[4].path, "/api/user/invoiceTitle/setDefault");
        assert_eq!(docs[1].method, "POST");
    }
}
