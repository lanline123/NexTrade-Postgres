//! 商城前台充值赠品路由（marketing::recharge_gift，2 条，`optional` 鉴权）。
//!
//! 【我方新增 2026-11-21，论坛帖 717721；原版无此控制器】
//!
//! | 本模块路由（挂载于 `/api`） | 说明 |
//! |-----------------------------|------|
//! | `GET /user/rechargeGift/list` | 我的赠品记录（待领取/已领取/已作废/异常） |
//! | `POST /user/rechargeGift/claim` | 领取：选收货地址 → 扣赠品池库存 → 生成履约订单 |
//!
//! 2 条均 `optional`（无令牌放行，服务层按 user_id 校验登录与归属），
//! 与 payment::recharge_order 同款。

use axum::{
    extract::{Query, Request, State},
    http::header,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};

use nextrade_core::models::marketing::recharge_gift::{
    RechargeGiftClaimDTO, RechargeGiftListQuery, RechargeGiftRecordVO,
};
use nextrade_core::types::PageResult;
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};
use nextrade_security::JwtService;

use crate::PublicState;

/// 本域 2 条路由（`/api` 前缀之后的路径 + 说明）。
const ROUTE_TABLE: [(&str, &str, &str); 2] = [
    (
        "GET",
        "/user/rechargeGift/list",
        "RechargeGiftClientController.list",
    ),
    (
        "POST",
        "/user/rechargeGift/claim",
        "RechargeGiftClientController.claim",
    ),
];

/// 前台充值赠品路由（挂载于 `/api`，模块自带可选登录态中间件）。
pub fn public_routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/user/rechargeGift/list", get(my_list))
        .route("/user/rechargeGift/claim", post(claim))
        .layer(middleware::from_fn_with_state(
            state.jwt.clone(),
            optional_auth,
        ))
}

/// 本域已实现路由（完整契约路径，用于覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/api{path}"), handler))
        .collect()
}

/// 可选登录态中间件（对应 Java `permitAll` + JWT 过滤器）。
async fn optional_auth(
    State(jwt): State<JwtService>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    if token.is_some() {
        let Ok(claims) = jwt.from_authorization(token) else {
            return Ok(next.run(req).await);
        };
        let Ok(id) = claims.sub.parse::<i32>() else {
            return Ok(next.run(req).await);
        };
        req.extensions_mut().insert(AuthUser {
            id,
            username: claims.usr,
            role_id: claims.role,
            admin_type: claims.typ,
            shop_id: claims.shop_id,
        });
    }
    Ok(next.run(req).await)
}

fn user_id(auth: Option<Extension<AuthUser>>) -> i32 {
    auth.map(|Extension(a)| a.id).unwrap_or(0)
}

/// `GET /api/user/rechargeGift/list`：我的赠品记录。
async fn my_list(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<RechargeGiftListQuery>,
) -> ApiResult<PageResult<RechargeGiftRecordVO>> {
    ok(state
        .marketing
        .recharge_gift
        .my_list(user_id(auth), &q)
        .await?)
}

/// `POST /api/user/rechargeGift/claim`：领取赠品（生成履约订单）。
async fn claim(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(dto): Json<RechargeGiftClaimDTO>,
) -> ApiResult<RechargeGiftRecordVO> {
    ok(state
        .marketing
        .recharge_gift
        .claim(user_id(auth), &dto)
        .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 2);
        for (m, p) in [
            ("GET", "/api/user/rechargeGift/list"),
            ("POST", "/api/user/rechargeGift/claim"),
        ] {
            assert!(docs.iter().any(|d| d.method == m && d.path == p), "{m} {p}");
        }
    }

    #[test]
    fn route_table_matches_registrations() {
        assert_eq!(ROUTE_TABLE.len(), 2);
    }
}
