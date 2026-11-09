//! 用户线下支付认款（v5.8.61 `/api/user/order/offlinePaymentClaim`）。

use axum::{
    extract::{Query, Request, State},
    http::header,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
use nextrade_core::models::trade::offline_claim::{
    OfflinePaymentClaimSubmitDTO, OfflinePaymentClaimVO,
};
use nextrade_core::Error;
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};
use nextrade_security::JwtService;

use crate::PublicState;

const MSG_LOGIN_REQUIRED: &str = "请先登录";

pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/user/order/offlinePaymentClaim/submit", post(submit))
        .route("/user/order/offlinePaymentClaim/detail", get(detail))
        .route("/user/order/offlinePaymentClaim/history", get(history))
        .layer(middleware::from_fn_with_state(state.jwt.clone(), optional_auth))
}

#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    [
        ("POST", "/api/user/order/offlinePaymentClaim/submit", "OfflinePaymentClaimController.submit"),
        ("GET", "/api/user/order/offlinePaymentClaim/detail", "OfflinePaymentClaimController.detail"),
        ("GET", "/api/user/order/offlinePaymentClaim/history", "OfflinePaymentClaimController.history"),
    ]
    .iter()
    .map(|(m, p, h)| RouteDoc::new(m, p, h))
    .collect()
}

async fn optional_auth(
    State(jwt): State<JwtService>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = req.headers().get(header::AUTHORIZATION).and_then(|v| v.to_str().ok());
    if let Some(token) = token {
        if let Ok(claims) = jwt.from_authorization(Some(token)) {
            if let Ok(id) = claims.sub.parse::<i32>() {
                req.extensions_mut().insert(AuthUser {
                    id,
                    username: claims.usr,
                    role_id: claims.role,
                    admin_type: claims.typ,
                    shop_id: claims.shop_id,
                });
            }
        }
    }
    Ok(next.run(req).await)
}

fn current_user_id(auth: Option<Extension<AuthUser>>) -> Result<i32, AppError> {
    auth.map(|Extension(user)| user.id)
        .filter(|id| *id > 0)
        .ok_or_else(|| AppError(Error::Unauthorized(MSG_LOGIN_REQUIRED.into())))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrderQuery {
    order_id: Option<i32>,
}

async fn submit(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(dto): Json<OfflinePaymentClaimSubmitDTO>,
) -> ApiResult<i32> {
    let user_id = current_user_id(auth)?;
    ok(state.trade.order.offline_claim_submit(user_id, dto).await?)
}

async fn detail(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<OrderQuery>,
) -> ApiResult<Option<OfflinePaymentClaimVO>> {
    let user_id = current_user_id(auth)?;
    let order_id = q.order_id.ok_or_else(|| AppError(Error::biz(400, "订单不存在")))?;
    ok(state.trade.order.offline_claim_latest(order_id, user_id).await?)
}

async fn history(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<OrderQuery>,
) -> ApiResult<Vec<OfflinePaymentClaimVO>> {
    let user_id = current_user_id(auth)?;
    let order_id = q.order_id.ok_or_else(|| AppError(Error::biz(400, "订单不存在")))?;
    ok(state.trade.order.offline_claim_history(order_id, user_id).await?)
}
