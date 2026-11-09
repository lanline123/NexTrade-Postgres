//! 商城前台行为验证码路由（member::captcha_api）。
//! Java `com.nextrade.api.controller.user.CaptchaApiController`（3 匿名端点）。
//! 匿名白名单：注册在 `anonymous_routes`（需要 state，但不套 require_auth）。
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use nextrade_core::models::member::captcha_api::{CaptchaDTO, CaptchaVO};
use nextrade_security::web::{ok, ApiResult};

use crate::PublicState;

const ROUTE_TABLE: [(&str, &str, &str); 3] = [
    ("POST", "/common/verification/captcha", "CaptchaApiController.captcha"),
    ("POST", "/common/verification/get", "CaptchaApiController.get"),
    ("POST", "/common/verification/check", "CaptchaApiController.check"),
];

/// 匿名路由（需 state）。
pub fn anonymous_routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .route(
            "/common/verification/captcha",
            get(captcha_get).post(captcha),
        )
        .route("/common/verification/get", get(captcha_get).post(get_compat))
        .route("/common/verification/check", post(check))
}

#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, &format!("/api{p}"), j))
        .collect()
}

async fn captcha(State(state): State<PublicState>) -> ApiResult<CaptchaVO> {
    ok(state.member.captcha_api.captcha()?)
}

/// 前端滑块组件使用 GET 获取验证码（契约只登记 POST，此处两种方法都支持）。
async fn captcha_get(State(state): State<PublicState>) -> ApiResult<CaptchaVO> {
    ok(state.member.captcha_api.get()?)
}

async fn get_compat(State(state): State<PublicState>) -> ApiResult<CaptchaVO> {
    ok(state.member.captcha_api.get()?)
}

async fn check(State(state): State<PublicState>, Json(dto): Json<CaptchaDTO>) -> ApiResult<bool> {
    ok(state.member.captcha_api.check(&dto)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_match_contract() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 3);
        assert_eq!(docs[0].path, "/api/common/verification/captcha");
        assert_eq!(docs[1].path, "/api/common/verification/get");
        assert_eq!(docs[2].path, "/api/common/verification/check");
        assert!(docs.iter().all(|d| d.method == "POST"));
    }
}
