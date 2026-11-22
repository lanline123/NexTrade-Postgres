//! 平台联邦·登录互认会员面路由（三期①，迁移 042；DESIGN.md §1）。
//!
//! 来源标注：**我方新增**（总设计 §6.3 授权码模式）。
//!
//! | method | path | 鉴权 | 说明 |
//! |--------|------|------|------|
//! | `GET`  | `/federation/sso/requestInfo` | 会员 JWT | 中枢侧授权请求信息（同意页） |
//! | `POST` | `/federation/sso/consent` | 会员 JWT | 中枢侧同意/拒绝（本人操作） |
//! | `POST` | `/federation/sso/start` | 匿名 | 成员侧发起联邦登录 |
//! | `POST` | `/federation/sso/callback` | 匿名 | 成员侧回调落号 + 签发本地令牌 |
//! | `GET`  | `/federation/sso/options` | 匿名 | 成员侧联邦登录可用性探针（前台接缝补线：登录页入口显隐，只读无副作用；不可用只回 available=false） |
//!
//! 同意必须由会员本人持本地 JWT 完成——联邦钥只登记/兑换，永不代签同意
//! （安全红线）。回调落号后签发的就是本站普通会员令牌（typ=member）。

use axum::{
    extract::{Query, State},
    middleware,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;

use nextrade_core::models::hub::federation::{
    SsoCallbackDTO, SsoCallbackVO, SsoConsentDTO, SsoConsentVO, SsoOptionsVO, SsoRequestInfoVO,
    SsoStartDTO, SsoStartVO,
};
use nextrade_security::web::{ok, require_auth, ApiResult, AppError, AuthUser};

use super::super::super::PublicState;

/// 本域路由（去掉 `/api` 前缀）。
const ROUTE_TABLE: [(&str, &str, &str); 5] = [
    ("GET", "/federation/sso/requestInfo", "HubSsoApi.requestInfo"),
    ("POST", "/federation/sso/consent", "HubSsoApi.consent"),
    ("POST", "/federation/sso/start", "HubSsoApi.start"),
    ("POST", "/federation/sso/callback", "HubSsoApi.callback"),
    ("GET", "/federation/sso/options", "HubSsoApi.options"),
];

/// 需会员登录路由（中枢侧同意面，自套 require_auth）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/federation/sso/requestInfo", get(request_info))
        .route("/federation/sso/consent", post(consent))
        .layer(middleware::from_fn_with_state(
            state.jwt.clone(),
            require_auth,
        ))
}

/// 匿名路由（成员侧发起/回调/可用性探针；回调成功即签发本地会员令牌）。
pub fn anonymous_routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .route("/federation/sso/start", post(start))
        .route("/federation/sso/callback", post(callback))
        .route("/federation/sso/options", get(options))
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestInfoParams {
    request_id: i32,
}

async fn request_info(
    State(state): State<PublicState>,
    Query(q): Query<RequestInfoParams>,
) -> ApiResult<SsoRequestInfoVO> {
    let vo = state
        .hub
        .federation
        .sso_request_info(q.request_id)
        .await
        .map_err(AppError)?;
    ok(vo)
}

async fn consent(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<SsoConsentDTO>,
) -> ApiResult<SsoConsentVO> {
    let vo = state
        .hub
        .federation
        .sso_consent(auth.id, &dto)
        .await
        .map_err(AppError)?;
    ok(vo)
}

async fn options(State(state): State<PublicState>) -> ApiResult<SsoOptionsVO> {
    let vo = state
        .hub
        .federation
        .sso_options()
        .await
        .map_err(AppError)?;
    ok(vo)
}

async fn start(
    State(state): State<PublicState>,
    Json(dto): Json<SsoStartDTO>,
) -> ApiResult<SsoStartVO> {
    let vo = state
        .hub
        .federation
        .sso_start(&dto)
        .await
        .map_err(AppError)?;
    ok(vo)
}

async fn callback(
    State(state): State<PublicState>,
    Json(dto): Json<SsoCallbackDTO>,
) -> ApiResult<SsoCallbackVO> {
    let (user_id, username, global_member_id, created) = state
        .hub
        .federation
        .sso_callback(&dto)
        .await
        .map_err(AppError)?;
    let (access, refresh, expires_in) =
        state
            .jwt
            .issue_pair(user_id.to_string(), username, None, Some("member".into()))?;
    ok(SsoCallbackVO {
        token: access.clone(),
        access_token: <redacted>
        refresh_token: <redacted>
        expires_in,
        user_id,
        global_member_id,
        created,
    })
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
