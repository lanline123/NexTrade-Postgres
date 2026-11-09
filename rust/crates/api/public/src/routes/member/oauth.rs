//! 第三方 OAuth 登录（`/api/user/oauth`）。
//!
//! 对应 Java `RestAuthController`：`render/{source}`、`callback/{source}`。
//! 当前已迁移 Google / Facebook；Twitter 暂返回明确业务错误。

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use nextrade_core::models::member::login::LoginWechatEventVO;
use nextrade_security::web::{ok, ApiResult};

use crate::PublicState;

const ROUTE_TABLE: [(&str, &str, &str); 3] = [
    ("GET", "/user/oauth/render/{source}", "RestAuthController.renderAuth"),
    ("GET", "/user/oauth/callback/{source}", "RestAuthController.login"),
    ("POST", "/user/oauth/callback/{source}", "RestAuthController.login"),
];

pub fn anonymous_routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .route("/user/oauth/render/{source}", get(render))
        .route("/user/oauth/callback/{source}", get(callback_get).post(callback))
}

#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, &format!("/api{p}"), j))
        .collect()
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct RenderQuery {
    return_url: Option<String>,
    platform: Option<String>,
}

/// `GET /api/user/oauth/render/{source}`：生成第三方授权地址。
async fn render(
    State(state): State<PublicState>,
    Path(source): Path<String>,
    Query(q): Query<RenderQuery>,
) -> ApiResult<String> {
    ok(state
        .member
        .login
        .oauth_render(&source, q.return_url.as_deref(), q.platform.as_deref())
        .await?)
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct CallbackBody {
    code: Option<String>,
    state: Option<String>,
}

/// `POST /api/user/oauth/callback/{source}`：第三方回调登录。
async fn callback(
    State(state): State<PublicState>,
    Path(source): Path<String>,
    Json(body): Json<CallbackBody>,
) -> ApiResult<LoginWechatEventVO> {
    let code = body
        .code
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| nextrade_security::web::AppError(nextrade_core::Error::biz(1, "code不能为空")))?;
    let st = body.state.as_deref().unwrap_or_default();
    ok(state.member.login.oauth_callback(&source, code, st).await?)
}

/// `GET /api/user/oauth/callback/{source}`：测试版回调（query 传参）。
async fn callback_get(
    State(state): State<PublicState>,
    Path(source): Path<String>,
    Query(q): Query<CallbackQuery>,
) -> ApiResult<LoginWechatEventVO> {
    let code = q
        .code
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| nextrade_security::web::AppError(nextrade_core::Error::biz(1, "code不能为空")))?;
    ok(state
        .member
        .login
        .oauth_callback(&source, code, q.state.as_deref().unwrap_or_default())
        .await?)
}
