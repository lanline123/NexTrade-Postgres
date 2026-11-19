//! 推荐 —— 商城端路由（system::recommend）—— wave9。
//!
//! 原 Java `RecommendController`（`@RequestMapping("/api/common/recommend")`），
//! 2 个 public GET（契约 `auth=optional`），另扩展 1 个协同过滤端点：
//!
//! | 本模块路由（挂载于 `/api`） | Java handler |
//! |-----------------------------|--------------|
//! | `GET /common/recommend/guessLike` | getProductList |
//! | `GET /common/recommend/getProductIds` | getProductIds |
//! | `GET /common/recommend/cohortBuys` | （扩展，无 Java 对应） |
//!
//! > 自带 optional_auth；必须并入 `merge_public`，外层不得再套 `require_auth`。

use axum::{
    extract::{Query, Request, State},
    http::header,
    middleware::{self, Next},
    response::Response,
    routing::get,
    Extension, Router,
};
use nextrade_core::models::system::recommend::{
    CohortBuysQuery, GuessLikeQuery, RecommendProductVO,
};
use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};
use nextrade_security::JwtService;

use crate::PublicState;

const ROUTE_TABLE: [(&str, &str, &str); 3] = [
    ("GET", "/common/recommend/guessLike", "RecommendController.getProductList"),
    ("GET", "/common/recommend/getProductIds", "RecommendController.getProductIds"),
    ("GET", "/common/recommend/cohortBuys", "RecommendController.cohortBuys"),
];

/// 推荐路由。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/common/recommend/guessLike", get(guess_like))
        .route("/common/recommend/getProductIds", get(get_product_ids))
        .route("/common/recommend/cohortBuys", get(cohort_buys))
        .layer(middleware::from_fn_with_state(state.jwt.clone(), optional_auth))
}

/// 已实现路由（覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, &format!("/api{p}"), j))
        .collect()
}

/// 价格门控（第 2 期补漏）：推荐商品 VO 带 `productPrice`，本域此前
/// 零接入；无看价权时逐项脱敏 + 打标记，与商品域相关推荐同一处理
/// （`services::system::price_gate`）。
fn gate_items(
    list: &[RecommendProductVO],
    access: nextrade_services::system::price_gate::PriceAccess,
) -> serde_json::Value {
    let mut v = serde_json::to_value(list).unwrap_or(serde_json::Value::Null);
    if let Some(items) = v.as_array_mut() {
        for item in items.iter_mut() {
            if !access.can_view() {
                nextrade_services::system::price_gate::mask_price_keys(item);
            }
            nextrade_services::system::price_gate::stamp_price_gate(item, access);
        }
    }
    v
}

/// 可选登录态 → 看价权解析（本域各带价读口共用）。
async fn price_access(
    state: &PublicState,
    auth: Option<Extension<AuthUser>>,
) -> Result<nextrade_services::system::price_gate::PriceAccess, AppError> {
    let user_id = auth
        .map(|Extension(a)| a.id)
        .filter(|id| *id > 0);
    Ok(state.system.config_client.price_access(user_id).await?)
}

/// `GET /api/common/recommend/guessLike`（带价出参，价格门控逐项脱敏）。
async fn guess_like(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<GuessLikeQuery>,
) -> ApiResult<serde_json::Value> {
    let access = price_access(&state, auth).await?;
    let list = state.system.recommend.guess_like(&q).await?;
    ok(gate_items(&list, access))
}

/// `GET /api/common/recommend/cohortBuys`
///
/// 相似用户近期购买（协同过滤）：与当前用户买过相同分类的其他用户，
/// 近 N 天（默认 90）买过的商品聚合榜；游客或本人无购买记录时回退为
/// 全体用户近窗口购买榜。只返回商品与聚合顺序，不含任何用户信息。
/// 出参同为带价推荐 VO，价格门控逐项脱敏（同 guessLike）。
async fn cohort_buys(
    State(state): State<PublicState>,
    Query(q): Query<CohortBuysQuery>,
    auth: Option<Extension<AuthUser>>,
) -> ApiResult<serde_json::Value> {
    let user_id = auth.as_ref().map(|Extension(a)| a.id).unwrap_or(0);
    let access = price_access(&state, auth).await?;
    let list = state.system.recommend.cohort_buys(user_id, &q).await?;
    ok(gate_items(&list, access))
}

/// `GET /api/common/recommend/getProductIds`
///
/// 原版 `getProductIds` 内部取 `SecurityUtils.getCurrentUserId()`，登录会员会额外掺入
/// 浏览记录段；游客按 0 处理。
async fn get_product_ids(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
) -> ApiResult<String> {
    let user_id = auth.as_ref().map(|Extension(a)| a.id).unwrap_or(0);
    ok(state.system.recommend.product_ids(user_id).await?)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_table_matches_contract() {
        assert_eq!(ROUTE_TABLE.len(), 3);
        let docs = implemented_routes();
        assert_eq!(docs[0].path, "/api/common/recommend/guessLike");
        assert_eq!(docs[1].path, "/api/common/recommend/getProductIds");
        assert_eq!(docs[2].path, "/api/common/recommend/cohortBuys");
        assert!(ROUTE_TABLE.iter().all(|(_, p, _)| !p.contains("/api")));
    }

    #[test]
    fn both_routes_are_get() {
        assert!(ROUTE_TABLE.iter().all(|(m, _, _)| *m == "GET"));
    }

    #[test]
    fn recommend_items_gate_masks_price() {
        use nextrade_services::system::price_gate::PriceAccess;
        // 真实数据形状：推荐 VO（Java ProductListResDTO 子集），带售价
        let list = vec![RecommendProductVO {
            product_id: 1576,
            product_name: "示例商品".to_string(),
            product_sn: "sn0001576".to_string(),
            product_price: rust_decimal::Decimal::new(45000, 0),
            pic_url: "https://example.com/a.jpg".to_string(),
            pic_thumb: "https://example.com/a_t.jpg".to_string(),
            shop_id: 0,
            product_type: Some(1),
            product_stock: 10,
            virtual_sales: 3,
        }];
        let masked = gate_items(&list, PriceAccess::NeedCertification);
        let item = &masked[0];
        assert!(item["productPrice"].is_null());
        assert_eq!(item["priceVisible"], false);
        assert_eq!(item["priceGate"], "certification");
        assert_eq!(item["productName"], "示例商品");
        assert_eq!(item["productStock"], 10);
        // 可见时价格原样、标记为 visible
        let visible = gate_items(&list, PriceAccess::Visible);
        assert_eq!(visible[0]["productPrice"], "45000");
        assert_eq!(visible[0]["priceVisible"], true);
        assert_eq!(visible[0]["priceGate"], "none");
    }
}
