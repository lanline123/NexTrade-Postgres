//! 商城前台秒杀路由。
//!
//! **重要：本模块路径为 Rust 版新增，原 Java 契约中不存在。**
//! 原 Java 前台只有 `GET /api/home/home/getSeckill`（首页秒杀模块数据），
//! 秒杀价与参与资格在结算流程内部计算，没有独立 HTTP 端点。
//! 本模块把该能力拆成独立端点以便结算页与活动页复用，
//! 因此在覆盖率报告中会被归入 `extra_routes`（需人工确认是否保留）。
//!
//! | 本模块路由 | 说明 |
//! |-----------|------|
//! | `GET /promotion/seckill/list` | 秒杀专区（进行中且有余量） |
//! | `GET /promotion/seckill/price` | 结算页秒杀价查询 |
//! | `GET /promotion/seckill/canJoin` | 参与资格判定（需登录） |

use axum::{
    extract::{Query, State},
    routing::get,
    Extension, Router,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use nextrade_core::models::marketing::seckill::SeckillPromotionVO;
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::PublicState;

/// 秒杀域**公开**路由（无需登录）。
pub fn public_routes() -> Router<PublicState> {
    Router::new()
        .route("/promotion/seckill/list", get(list))
        .route("/promotion/seckill/price", get(price))
}

/// 秒杀域**需登录**路由（外层已套 `require_auth`）。
pub fn authorized_routes() -> Router<PublicState> {
    Router::new().route("/promotion/seckill/canJoin", get(can_join))
}

/// 本域已实现路由。
///
/// 返回空列表：上述三个端点均为 Rust 版新增，原 Java 契约中无对应项，
/// 故不计入覆盖率分子（会在 `extra_routes` 中列出以待确认）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    Vec::new()
}

/// 秒杀专区列表查询。
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ListQuery {
    page: Option<u64>,
    page_size: Option<u64>,
}

/// `GET /api/promotion/seckill/list`：进行中的秒杀商品。
async fn list(
    State(state): State<PublicState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<PageResult<SeckillPromotionVO>> {
    ok(state.marketing.seckill.active_list(q.page, q.page_size).await?)
}

/// 秒杀价查询参数。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PriceQuery {
    product_id: i32,
    sku_id: Option<i32>,
}

/// `GET /api/promotion/seckill/price`：查询当前秒杀价。
///
/// 返回 `{"seckillPrice": "9.90"}`；无进行中活动时 `seckillPrice` 为 `null`。
async fn price(
    State(state): State<PublicState>,
    Query(q): Query<PriceQuery>,
) -> ApiResult<serde_json::Value> {
    let price: Option<Decimal> = state.marketing.seckill
        .current_seckill_price(q.product_id, q.sku_id)
        .await?;
    ok(serde_json::json!({ "seckillPrice": price }))
}

/// `GET /api/promotion/seckill/canJoin`：能否参与该商品秒杀。
async fn can_join(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<PriceQuery>,
) -> ApiResult<bool> {
    ok(state.marketing.seckill
        .can_user_join(q.product_id, q.sku_id, auth.id)
        .await?)
}