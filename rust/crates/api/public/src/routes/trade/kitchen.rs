//! 前台后厨催菜路由（trade::kitchen，我方新增，论坛帖 817927）。
//!
//! | 本模块路由（挂载于 `/api`） | 用途 | 登录 |
//! |------------------------------|------|------|
//! | `GET  /user/order/kitchenInfo` | 订单详情后厨面板（状态 + 可否催菜） | 必须 |
//! | `POST /user/order/urge` | 催菜（10 分钟冷却 + 每日 5 次上限） | 必须 |
//!
//! > 契约 auth=optional：本文件自带 [`optional_auth`]，端点在 handler
//! > 内校验登录，与 `trade/gift_order.rs` 同口径。外层不要再套 require_auth。

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
use nextrade_core::models::trade::kitchen::{KitchenInfoVO, KitchenUrgeDTO, KitchenUrgeResultVO};
use nextrade_core::Error;
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};
use nextrade_security::JwtService;

use crate::PublicState;

/// 本域全部 2 条路由（去掉 `/api` 前缀后的路径 + 说明）。
const ROUTE_TABLE: [(&str, &str, &str); 2] = [
    ("GET", "/user/order/kitchenInfo", "Kitchen.buyerInfo"),
    ("POST", "/user/order/urge", "Kitchen.urge"),
];

/// 后厨催菜路由（挂载于 `/api` 下）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/user/order/kitchenInfo", get(kitchen_info))
        .route("/user/order/urge", post(urge))
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

/// 可选登录态中间件（与 `trade/gift_order.rs` 逐行一致）。
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

/// 订单 ID 入参（kitchenInfo 用）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrderIdParam {
    order_id: i32,
}

/// `GET /api/user/order/kitchenInfo`：买家后厨面板（状态 + 催菜频控现状）。
async fn kitchen_info(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<OrderIdParam>,
) -> ApiResult<KitchenInfoVO> {
    let user_id = current_user_id(auth)?;
    ok(state.trade.kitchen.buyer_info(user_id, q.order_id).await?)
}

/// `POST /api/user/order/urge`：买家催菜（频控在服务层）。
async fn urge(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(dto): Json<KitchenUrgeDTO>,
) -> ApiResult<KitchenUrgeResultVO> {
    let user_id = current_user_id(auth)?;
    ok(state.trade.kitchen.urge(user_id, dto.order_id).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_table_complete() {
        assert_eq!(ROUTE_TABLE.len(), 2);
        let docs = implemented_routes();
        assert_eq!(docs.len(), 2);
        assert!(docs.iter().all(|d| d.path.starts_with("/api/")));
    }
}
