//! 前台送礼物订单流路由（trade::gift_order，我方新增，论坛帖 197742）。
//!
//! | 本模块路由（挂载于 `/api`） | 用途 | 登录 |
//! |------------------------------|------|------|
//! | `POST /order/gift/claimPreview` | 领取页预览（商品/赠言/状态，无价格） | 可选 |
//! | `POST /order/gift/claim` | 领取（填自己的地址落单） | 必须 |
//! | `GET  /user/order/giftInfo` | 买家订单礼物面板数据 | 必须 |
//! | `POST /user/order/giftShareToken` | 重新生成分享链接（旧链接作废） | 必须 |
//! | `POST /user/order/giftCancel` | 买家取消（未付款直取消/待领取退款关闭） | 必须 |
//! | `GET  /user/gift/received` | 我收到的礼物（领取人视角，无价格） | 必须 |
//!
//! > 契约 auth=optional：本文件自带 [`optional_auth`]，需登录的端点在
//! > handler 内校验，与 `trade/check.rs` 同口径。外层不要再套 require_auth。

use axum::{
    extract::{Query, Request, State},
    http::header,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;

use nextrade_core::models::trade::check::MSG_LOGIN_REQUIRED;
use nextrade_core::models::trade::gift_order::{
    GiftClaimDTO, GiftClaimPreviewDTO, GiftClaimPreviewVO, GiftInfoVO, GiftReceivedVO,
    GiftShareTokenVO,
};
use nextrade_core::Error;
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};
use nextrade_security::JwtService;

use crate::PublicState;

/// 本域全部 6 条路由（去掉 `/api` 前缀后的路径 + 说明）。
const ROUTE_TABLE: [(&str, &str, &str); 6] = [
    ("POST", "/order/gift/claimPreview", "GiftOrder.claimPreview"),
    ("POST", "/order/gift/claim", "GiftOrder.claim"),
    ("GET", "/user/order/giftInfo", "GiftOrder.giftInfo"),
    (
        "POST",
        "/user/order/giftShareToken",
        "GiftOrder.giftShareToken",
    ),
    ("POST", "/user/order/giftCancel", "GiftOrder.giftCancel"),
    ("GET", "/user/gift/received", "GiftOrder.received"),
];

/// 送礼物路由（挂载于 `/api` 下）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/order/gift/claimPreview", post(claim_preview))
        .route("/order/gift/claim", post(claim))
        .route("/user/order/giftInfo", get(gift_info))
        .route("/user/order/giftShareToken", post(gift_share_token))
        .route("/user/order/giftCancel", post(gift_cancel))
        .route("/user/gift/received", get(received))
        .layer(middleware::from_fn_with_state(
            state.jwt.clone(),
            optional_auth,
        ))
}

/// 本模块已实现路由（用于 `/openapi/coverage` 覆盖率统计，完整契约路径）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/api{path}"), handler))
        .collect()
}

/// 可选登录态中间件（与 `trade/check.rs` 逐行一致）。
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
        let Ok(claims) = jwt.from_authorization(token) else { return Ok(next.run(req).await); };
        let Ok(id) = claims.sub.parse::<i32>() else { return Ok(next.run(req).await); };
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

/// 从可选扩展中取当前会员 id，游客 → 401「请先登录」。
fn current_user_id(auth: Option<Extension<AuthUser>>) -> std::result::Result<i32, AppError> {
    auth.map(|Extension(user)| user.id)
        .filter(|id| *id > 0)
        .ok_or_else(|| AppError(Error::Unauthorized(MSG_LOGIN_REQUIRED.into())))
}

/// 订单 ID 入参（giftInfo / giftShareToken / giftCancel 共用）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrderIdParam {
    order_id: i32,
}

/// `POST /api/order/gift/claimPreview`：领取页预览（可匿名；无价格字段）。
async fn claim_preview(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(dto): Json<GiftClaimPreviewDTO>,
) -> ApiResult<GiftClaimPreviewVO> {
    let viewer = auth.map(|Extension(user)| user.id).filter(|id| *id > 0);
    ok(state.trade.gift_order.claim_preview(&dto.token, viewer).await?)
}

/// `POST /api/order/gift/claim`：领取（必须登录；地址取领取人地址簿）。
async fn claim(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(dto): Json<GiftClaimDTO>,
) -> ApiResult<GiftInfoVO> {
    let user_id = current_user_id(auth)?;
    ok(state
        .trade
        .gift_order
        .claim(&dto.token, user_id, dto.address_id)
        .await?)
}

/// `GET /api/user/order/giftInfo`：买家礼物面板（非礼物订单返回 null）。
async fn gift_info(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<OrderIdParam>,
) -> ApiResult<Option<GiftInfoVO>> {
    let user_id = current_user_id(auth)?;
    ok(state.trade.gift_order.buyer_info(q.order_id, user_id).await?)
}

/// `POST /api/user/order/giftShareToken`：重新生成分享链接（旧链接作废）。
async fn gift_share_token(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(dto): Json<OrderIdParam>,
) -> ApiResult<GiftShareTokenVO> {
    let user_id = current_user_id(auth)?;
    ok(state
        .trade
        .gift_order
        .regenerate_share_token(dto.order_id, user_id)
        .await?)
}

/// `POST /api/user/order/giftCancel`：买家取消礼物订单。
async fn gift_cancel(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(dto): Json<OrderIdParam>,
) -> ApiResult<bool> {
    let user_id = current_user_id(auth)?;
    state
        .trade
        .gift_order
        .buyer_cancel(dto.order_id, user_id, &state.trade.order)
        .await?;
    ok(true)
}

/// `GET /api/user/gift/received`：我收到的礼物（无价格字段）。
async fn received(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
) -> ApiResult<Vec<GiftReceivedVO>> {
    let user_id = current_user_id(auth)?;
    ok(state.trade.gift_order.received_list(user_id).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_table_complete() {
        assert_eq!(ROUTE_TABLE.len(), 6);
        let docs = implemented_routes();
        assert_eq!(docs.len(), 6);
        assert!(docs.iter().all(|d| d.path.starts_with("/api/")));
    }

    #[test]
    fn source_pins_no_price_in_claim_surface() {
        // 领取面铁律：路由层不得引入任何金额型 VO
        let src = include_str!("gift_order.rs");
        assert!(!src.contains("total_amount"));
        assert!(!src.contains("paid_amount"));
    }
}
