//! 商城前台拍卖路由（Rust 版新增域，原 Java 契约中不存在，故不计入覆盖率）。
//!
//! | 路由 | 说明 |
//! |------|------|
//! | `GET  /promotion/auction/sessions` | 场次列表（公开） |
//! | `GET  /promotion/auction/items` | 拍品列表（公开，tab=all/preview/running/finished） |
//! | `GET  /promotion/auction/detail` | 拍品详情（可选登录：带 token 时含我的状态） |
//! | `GET  /promotion/auction/bids` | 出价墙（可选登录） |
//! | `GET  /promotion/auction/poll` | 轮询轻量状态（可选登录） |
//! | `POST /promotion/auction/freeze` | 冻结保证金（需登录） |
//! | `POST /promotion/auction/bid` | 出价 / 一口价（需登录） |
//! | `POST /promotion/auction/quit` | 退出参拍并解冻（需登录） |
//! | `GET  /promotion/auction/my/joins` | 我的拍卖（需登录） |
//! | `GET  /promotion/auction/my/deposits` | 我的保证金流水（需登录） |
//! | `POST /promotion/auction/remind` | 开拍提醒订阅/取消（需登录） |

use axum::{
    extract::{Query, Request, State},
    http::{header, HeaderMap},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;

use nextrade_core::models::marketing::auction::{
    AuctionBidDTO, AuctionBidVO, AuctionDepositListQuery, AuctionDepositVO, AuctionIdDTO,
    AuctionItemDetailVO, AuctionItemVO, AuctionMyJoinVO, AuctionPollVO, AuctionPublicListQuery,
    AuctionSessionVO,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};
use nextrade_security::JwtService;
use nextrade_services::marketing::auction::BidResultVO;

use crate::PublicState;

/// 公开路由（匿名可读，无我的状态）。
pub fn public_routes() -> Router<PublicState> {
    Router::new()
        .route("/promotion/auction/sessions", get(sessions))
        .route("/promotion/auction/items", get(items))
}

/// 自带可选登录态的路由（详情/出价墙/轮询：游客可看，登录后带我的状态）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/promotion/auction/detail", get(detail))
        .route("/promotion/auction/bids", get(bids))
        .route("/promotion/auction/poll", get(poll))
        .layer(middleware::from_fn_with_state(state.jwt.clone(), optional_auth))
}

/// 需登录路由（外层统一套 require_auth）。
pub fn authorized_routes() -> Router<PublicState> {
    Router::new()
        .route("/promotion/auction/freeze", post(freeze))
        .route("/promotion/auction/bid", post(bid))
        .route("/promotion/auction/quit", post(quit))
        .route("/promotion/auction/my/joins", get(my_joins))
        .route("/promotion/auction/my/deposits", get(my_deposits))
        .route("/promotion/auction/remind", post(remind))
}

/// 本域已实现路由：Rust 版新增域，返回空列表（同 seckill 口径）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    Vec::new()
}

/// 可选登录态中间件（与 catalog/comment.rs 同实现）：无令牌放行游客，
/// 令牌合法写入 AuthUser，非法则按游客处理（详情页不因过期 token 报错）。
async fn optional_auth(
    State(jwt): State<JwtService>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    if let Some(token) = token.as_deref() {
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

fn viewer_id(auth: &Option<Extension<AuthUser>>) -> i32 {
    auth.as_ref().map(|Extension(a)| a.id).unwrap_or(0)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdQuery {
    id: i32,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct BidsQuery {
    id: i32,
    page: Option<u64>,
    page_size: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct RemindDTO {
    item_id: i32,
    on: Option<bool>,
}

async fn sessions(State(state): State<PublicState>) -> ApiResult<Vec<AuctionSessionVO>> {
    ok(state.marketing.auction.sessions_public().await?)
}

async fn items(
    State(state): State<PublicState>,
    Query(q): Query<AuctionPublicListQuery>,
) -> ApiResult<PageResult<AuctionItemVO>> {
    ok(state.marketing.auction.items_public(&q).await?)
}

async fn detail(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<IdQuery>,
) -> ApiResult<AuctionItemDetailVO> {
    ok(state.marketing.auction.item_detail(q.id, viewer_id(&auth)).await?)
}

async fn bids(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<BidsQuery>,
) -> ApiResult<PageResult<AuctionBidVO>> {
    ok(state
        .marketing
        .auction
        .item_bids(q.id, q.page, q.page_size, viewer_id(&auth))
        .await?)
}

async fn poll(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<IdQuery>,
) -> ApiResult<AuctionPollVO> {
    ok(state.marketing.auction.item_poll(q.id, viewer_id(&auth)).await?)
}

async fn freeze(
    State(state): State<PublicState>,
    Extension(user): Extension<AuthUser>,
    Json(dto): Json<AuctionIdDTO>,
) -> ApiResult<AuctionDepositVO> {
    let item_id = dto.item_id.unwrap_or(0);
    ok(state.marketing.auction.freeze_deposit(user.id, item_id).await?)
}

async fn bid(
    State(state): State<PublicState>,
    Extension(user): Extension<AuthUser>,
    headers: HeaderMap,
    Json(dto): Json<AuctionBidDTO>,
) -> ApiResult<BidResultVO> {
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(str::trim)
        .unwrap_or("")
        .to_string();
    let ua = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    ok(state.marketing.auction.place_bid(user.id, &dto, &ip, &ua).await?)
}

async fn quit(
    State(state): State<PublicState>,
    Extension(user): Extension<AuthUser>,
    Json(dto): Json<AuctionIdDTO>,
) -> ApiResult<()> {
    state.marketing.auction.quit(user.id, dto.item_id.unwrap_or(0)).await?;
    ok(())
}

async fn my_joins(
    State(state): State<PublicState>,
    Extension(user): Extension<AuthUser>,
) -> ApiResult<Vec<AuctionMyJoinVO>> {
    ok(state.marketing.auction.my_joins(user.id).await?)
}

async fn my_deposits(
    State(state): State<PublicState>,
    Extension(user): Extension<AuthUser>,
    Query(q): Query<AuctionDepositListQuery>,
) -> ApiResult<PageResult<AuctionDepositVO>> {
    ok(state.marketing.auction.my_deposits(user.id, &q).await?)
}

async fn remind(
    State(state): State<PublicState>,
    Extension(user): Extension<AuthUser>,
    Json(dto): Json<RemindDTO>,
) -> ApiResult<()> {
    state
        .marketing
        .auction
        .remind(user.id, dto.item_id, dto.on.unwrap_or(true))
        .await?;
    ok(())
}
