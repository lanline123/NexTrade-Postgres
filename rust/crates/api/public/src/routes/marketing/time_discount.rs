//! 商城前台限时折扣路由。
//!
//! **重要：本模块路径为 Rust 版新增，原 Java 契约中不存在。**
//! 原 Java 前台没有限时折扣独立端点（折扣在结算流程内部计算），
//! 因此这些路由在覆盖率报告中会被归入 `extra_routes`（需人工确认）。
//!
//! | 本模块路由 | 说明 |
//! |-----------|------|
//! | `GET /promotion/timeDiscount/list` | 折扣专区（进行中） |
//! | `GET /promotion/timeDiscount/price` | 结算页折扣价查询 |
//! | `GET /promotion/timeDiscount/canJoin` | 参与资格判定（需登录） |

use axum::{
    extract::{Query, State},
    routing::get,
    Extension, Router,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use nextrade_core::models::marketing::time_discount::TimeDiscountPromotionVO;
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::PublicState;

/// 限时折扣域**公开**路由（无需登录）。
pub fn public_routes() -> Router<PublicState> {
    Router::new()
        .route("/promotion/timeDiscount/list", get(list))
        .route("/promotion/timeDiscount/price", get(price))
}

/// 限时折扣域**需登录**路由（外层已套 `require_auth`）。
pub fn authorized_routes() -> Router<PublicState> {
    Router::new().route("/promotion/timeDiscount/canJoin", get(can_join))
}

/// 本域已实现路由。
///
/// 返回空列表：本模块三个端点均为 Rust 版新增，原 Java 前台没有限时折扣
/// 独立端点（`/adminapi/promotion/timeDiscount/*` 是管理端契约，与前台无关），
/// 故不计入覆盖率分子，而是在 `extra_routes` 中列出以待确认。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    Vec::new()
}

/// 折扣专区列表查询。
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ListQuery {
    page: Option<u64>,
    page_size: Option<u64>,
}

/// `GET /api/promotion/timeDiscount/list`：进行中的折扣商品。
async fn list(
    State(state): State<PublicState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<PageResult<TimeDiscountPromotionVO>> {
    ok(state.marketing.time_discount.active_list(q.page, q.page_size).await?)
}

/// 折扣价查询参数。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PriceQuery {
    product_id: i32,
    sku_id: Option<i32>,
    /// 商品原价（用于计算折后价）
    price: Decimal,
}

/// `GET /api/promotion/timeDiscount/price`：查询折后价。
///
/// 返回 `{"discountPrice": "85.00", "hasDiscount": true}`；
/// 无适用折扣时 `discountPrice` 为 `null`。
async fn price(
    State(state): State<PublicState>,
    Query(q): Query<PriceQuery>,
) -> ApiResult<serde_json::Value> {
    let p: Option<Decimal> = state.marketing.time_discount
        .current_discount_price(q.product_id, q.sku_id, q.price)
        .await?;
    ok(serde_json::json!({
        "discountPrice": p,
        "hasDiscount": p.is_some(),
    }))
}

/// `GET /api/promotion/timeDiscount/canJoin`：能否参与该商品折扣。
async fn can_join(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<PriceQuery>,
) -> ApiResult<bool> {
    ok(state.marketing.time_discount
        .can_user_join(q.product_id, q.sku_id, auth.id)
        .await?)
}