//! 管理端拍卖路由（Rust 版新增域，原 Java 契约中不存在）。
//!
//! | 路由 | 说明 |
//! |------|------|
//! | `GET  /promotion/auction/session/list` | 场次列表 |
//! | `POST /promotion/auction/session/save` | 场次新增/编辑 |
//! | `POST /promotion/auction/session/del` | 场次删除 |
//! | `GET  /promotion/auction/item/list` | 拍品列表 |
//! | `GET  /promotion/auction/item/detail` | 拍品详情 |
//! | `POST /promotion/auction/item/save` | 拍品新增/编辑 |
//! | `POST /promotion/auction/item/cancel` | 撤拍 |
//! | `POST /promotion/auction/item/forceEnd` | 强制结束 |
//! | `POST /promotion/auction/item/del` | 拍品删除（草稿） |
//! | `GET  /promotion/auction/bid/list` | 出价记录 |
//! | `POST /promotion/auction/bid/void` | 作废出价 |
//! | `GET  /promotion/auction/deposit/list` | 保证金流水 |
//! | `GET  /promotion/auction/breach/list` | 违约记录 |
//! | `POST /promotion/auction/breach/waivePenalty` | 扣罚改判豁免 |
//! | `GET  /promotion/auction/ban/list` | 封禁记录 |
//! | `POST /promotion/auction/ban/save` | 手动封禁 |
//! | `POST /promotion/auction/ban/unban` | 解封 |
//! | `GET  /promotion/auction/settings` | 拍卖设置（读） |
//! | `POST /promotion/auction/settings/save` | 拍卖设置（存） |
//! | `GET  /promotion/auction/stats/overview` | 运营概览 |

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;

use nextrade_core::models::marketing::auction::{
    AuctionAdminBidVO, AuctionBanSaveDTO, AuctionBanVO, AuctionBidListQuery, AuctionBreachListQuery,
    AuctionBreachVO, AuctionDepositListQuery, AuctionDepositVO, AuctionIdDTO,
    AuctionItemAdminDetailVO, AuctionItemListQuery, AuctionItemSaveDTO, AuctionItemVO,
    AuctionSessionSaveDTO, AuctionSessionVO, AuctionStatsVO,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::AdminState;

/// 拍卖域路由（挂载于 `/adminapi` 下）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/promotion/auction/session/list", get(session_list))
        .route("/promotion/auction/session/save", post(session_save))
        .route("/promotion/auction/session/del", post(session_del))
        .route("/promotion/auction/item/list", get(item_list))
        .route("/promotion/auction/item/detail", get(item_detail))
        .route("/promotion/auction/item/save", post(item_save))
        .route("/promotion/auction/item/cancel", post(item_cancel))
        .route("/promotion/auction/item/forceEnd", post(item_force_end))
        .route("/promotion/auction/item/del", post(item_del))
        .route("/promotion/auction/bid/list", get(bid_list))
        .route("/promotion/auction/bid/void", post(bid_void))
        .route("/promotion/auction/deposit/list", get(deposit_list))
        .route("/promotion/auction/breach/list", get(breach_list))
        .route("/promotion/auction/breach/waivePenalty", post(waive_penalty))
        .route("/promotion/auction/ban/list", get(ban_list))
        .route("/promotion/auction/ban/save", post(ban_save))
        .route("/promotion/auction/ban/unban", post(unban))
        .route("/promotion/auction/settings", get(settings_get))
        .route("/promotion/auction/settings/save", post(settings_save))
        .route("/promotion/auction/stats/overview", get(stats))
}

/// 本域已实现路由：Rust 版新增域，返回空列表（同前台口径）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    Vec::new()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdQuery {
    id: i32,
}

// ————————————————————— 场次 —————————————————————

async fn session_list(State(state): State<AdminState>) -> ApiResult<Vec<AuctionSessionVO>> {
    ok(state.marketing.auction.session_list().await?)
}

async fn session_save(
    State(state): State<AdminState>,
    Json(dto): Json<AuctionSessionSaveDTO>,
) -> ApiResult<i32> {
    ok(state.marketing.auction.session_save(&dto).await?)
}

async fn session_del(
    State(state): State<AdminState>,
    Json(dto): Json<AuctionIdDTO>,
) -> ApiResult<()> {
    state
        .marketing
        .auction
        .session_delete(dto.session_id.unwrap_or(0))
        .await?;
    ok(())
}

// ————————————————————— 拍品 —————————————————————

async fn item_list(
    State(state): State<AdminState>,
    Query(q): Query<AuctionItemListQuery>,
) -> ApiResult<PageResult<AuctionItemVO>> {
    ok(state.marketing.auction.item_list_admin(&q).await?)
}

async fn item_detail(
    State(state): State<AdminState>,
    Query(q): Query<IdQuery>,
) -> ApiResult<AuctionItemAdminDetailVO> {
    ok(state.marketing.auction.item_detail_admin(q.id).await?)
}

async fn item_save(
    State(state): State<AdminState>,
    Extension(user): Extension<AuthUser>,
    Json(dto): Json<AuctionItemSaveDTO>,
) -> ApiResult<i32> {
    ok(state.marketing.auction.item_save(&dto, user.id).await?)
}

async fn item_cancel(
    State(state): State<AdminState>,
    Json(dto): Json<AuctionIdDTO>,
) -> ApiResult<()> {
    state.marketing.auction.item_cancel(dto.item_id.unwrap_or(0)).await?;
    ok(())
}

async fn item_force_end(
    State(state): State<AdminState>,
    Json(dto): Json<AuctionIdDTO>,
) -> ApiResult<()> {
    state.marketing.auction.item_force_end(dto.item_id.unwrap_or(0)).await?;
    ok(())
}

async fn item_del(State(state): State<AdminState>, Json(dto): Json<AuctionIdDTO>) -> ApiResult<()> {
    state.marketing.auction.item_delete(dto.item_id.unwrap_or(0)).await?;
    ok(())
}

// ————————————————————— 出价 / 保证金 —————————————————————

async fn bid_list(
    State(state): State<AdminState>,
    Query(q): Query<AuctionBidListQuery>,
) -> ApiResult<PageResult<AuctionAdminBidVO>> {
    ok(state.marketing.auction.bid_list_admin(&q).await?)
}

async fn bid_void(State(state): State<AdminState>, Json(dto): Json<AuctionIdDTO>) -> ApiResult<()> {
    state.marketing.auction.bid_void(dto.bid_id.unwrap_or(0)).await?;
    ok(())
}

async fn deposit_list(
    State(state): State<AdminState>,
    Query(q): Query<AuctionDepositListQuery>,
) -> ApiResult<PageResult<AuctionDepositVO>> {
    ok(state.marketing.auction.deposit_list_admin(&q).await?)
}

// ————————————————————— 违约 / 封禁 —————————————————————

async fn breach_list(
    State(state): State<AdminState>,
    Query(q): Query<AuctionBreachListQuery>,
) -> ApiResult<PageResult<AuctionBreachVO>> {
    ok(state.marketing.auction.breach_list(&q).await?)
}

async fn waive_penalty(
    State(state): State<AdminState>,
    Extension(user): Extension<AuthUser>,
    Json(dto): Json<AuctionIdDTO>,
) -> ApiResult<()> {
    state
        .marketing
        .auction
        .waive_penalty(dto.breach_id.unwrap_or(0), dto.reason.as_deref().unwrap_or(""), user.id)
        .await?;
    ok(())
}

async fn ban_list(
    State(state): State<AdminState>,
    Query(q): Query<AuctionBreachListQuery>,
) -> ApiResult<PageResult<AuctionBanVO>> {
    ok(state.marketing.auction.ban_list(&q).await?)
}

async fn ban_save(
    State(state): State<AdminState>,
    Extension(user): Extension<AuthUser>,
    Json(dto): Json<AuctionBanSaveDTO>,
) -> ApiResult<i32> {
    ok(state.marketing.auction.ban_save(&dto, user.id).await?)
}

async fn unban(State(state): State<AdminState>, Json(dto): Json<AuctionIdDTO>) -> ApiResult<()> {
    state.marketing.auction.unban(dto.ban_id.unwrap_or(0)).await?;
    ok(())
}

// ————————————————————— 设置 / 统计 —————————————————————

async fn settings_get(State(state): State<AdminState>) -> ApiResult<serde_json::Value> {
    ok(state.marketing.auction.get_settings_json().await?)
}

async fn settings_save(
    State(state): State<AdminState>,
    Json(payload): Json<serde_json::Value>,
) -> ApiResult<serde_json::Value> {
    ok(state.marketing.auction.save_settings(&payload).await?)
}

async fn stats(State(state): State<AdminState>) -> ApiResult<AuctionStatsVO> {
    ok(state.marketing.auction.stats().await?)
}
