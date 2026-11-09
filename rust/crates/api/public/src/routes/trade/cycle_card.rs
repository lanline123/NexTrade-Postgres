//! 前台次卡路由（trade::cycle_card）。
//!
//! 对应契约 `/api/cycleCard/*`（5 个端点）。
//! 需登录：无有效令牌返回 401；仅允许查询本人次卡。

use axum::{
    extract::{Path, Query, Request, State},
    http::header,
    middleware::{self, Next},
    response::Response,
    routing::get,
    Extension, Router,
};
use serde::Deserialize;
use serde_json::Value;
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult, AppError};
use nextrade_security::{AuthUser, JwtService};

use crate::PublicState;

const ROUTE_TABLE: [(&str, &str); 5] = [
    ("GET", "/cycleCard/list"),
    ("GET", "/cycleCard/detail/{cardId}"),
    ("GET", "/cycleCard/checkUsable/{cardId}"),
    ("GET", "/cycleCard/refundInfo/{cardId}"),
    ("GET", "/cycleCard/checkAftersales/{orderId}"),
];

pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/cycleCard/list", get(list))
        .route("/cycleCard/detail/{cardId}", get(detail))
        .route("/cycleCard/checkUsable/{cardId}", get(check_usable))
        .route("/cycleCard/refundInfo/{cardId}", get(refund_info))
        .route("/cycleCard/checkAftersales/{orderId}", get(check_aftersales))
        .layer(middleware::from_fn_with_state(state.jwt.clone(), optional_auth))
}

#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path)| RouteDoc::new(method, &format!("/api{path}"), "CycleCardController"))
        .collect()
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct PageQuery {
    page: Option<u64>,
    size: Option<u64>,
}

fn uid(auth: Option<Extension<AuthUser>>) -> Result<i32, AppError> {
    auth.map(|Extension(u)| u.id)
        .filter(|id| *id > 0)
        .ok_or_else(|| AppError(nextrade_core::Error::Unauthorized("请先登录".into())))
}

async fn list(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<PageQuery>,
) -> ApiResult<Value> {
    let page = q.page.unwrap_or(1).max(1);
    let size = q.size.unwrap_or(10).clamp(1, 100);
    let (total, records) = state
        .trade
        .user_order
        .cycle_cards(uid(auth)?, page, size)
        .await?;
    ok(serde_json::json!({"records": records, "total": total, "page": page, "size": size}))
}

async fn detail(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Path(card_id): Path<i32>,
) -> ApiResult<Value> {
    ok(state
        .trade
        .user_order
        .cycle_card(uid(auth)?, card_id)
        .await?
        .unwrap_or(Value::Null))
}

async fn check_usable(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Path(card_id): Path<i32>,
) -> ApiResult<Value> {
    let card = state
        .trade
        .user_order
        .cycle_card(uid(auth)?, card_id)
        .await?;
    let Some(card) = card else {
        return ok(serde_json::json!({"usable": false, "reason": "次卡不存在"}));
    };
    let status = card.get("status").and_then(Value::as_i64).unwrap_or(0);
    let remaining = card.get("remainingTimes").and_then(Value::as_i64).unwrap_or(0);
    let expire = card.get("expireTime").and_then(Value::as_i64).unwrap_or(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let usable = status == 1 && remaining > 0 && (expire == 0 || expire > now);
    ok(serde_json::json!({"usable": usable, "card": card}))
}

async fn refund_info(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Path(card_id): Path<i32>,
) -> ApiResult<Value> {
    ok(state
        .trade
        .user_order
        .cycle_refund_info(uid(auth)?, card_id)
        .await?
        .unwrap_or(Value::Null))
}

async fn check_aftersales(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Path(order_id): Path<i32>,
) -> ApiResult<Value> {
    ok(state
        .trade
        .user_order
        .cycle_aftersales_check(uid(auth)?, order_id)
        .await?)
}

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
