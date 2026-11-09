//! 商城前台优惠券路由。
//! 对应原 Java：
//! - `com.nextrade.api.controller.user.UserCouponController`（`/api/user/coupon`）
//! - `com.nextrade.api.controller.promotion.CouponClientController`（`/api/promotion/coupon`）
//!
//! 路径与原 Java 控制器保持一致：
//! | 路由 | 原 Java |
//! |------|---------|
//! | `GET  /api/user/coupon/getList` | `UserCouponController.getList`（公开可领取列表） |
//! | `GET  /api/user/coupon/list`    | `UserCouponController.list`（我的优惠券） |
//! | `GET  /api/user/coupon/detail`  | `UserCouponController.detail` |
//! | `POST /api/user/coupon/claim`   | `UserCouponController.claim` |
//! | `POST /api/user/coupon/del`     | `UserCouponController.del` |
//! | `GET  /api/promotion/coupon/listByIds` | `CouponClientController.listByIds` |

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
use nextrade_core::models::{CouponListQuery, UserCouponClaimDTO, UserCouponIdDTO, UserCouponListQuery};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::PublicState;

/// 公开路由：可领取优惠券列表、按 ID 批量查询（游客可访问）。
pub fn public_routes() -> Router<PublicState> {
    Router::new()
        .route("/user/coupon/getList", get(available_list))
        .route("/promotion/coupon/listByIds", get(list_by_ids))
}

/// 需登录路由：领取、我的优惠券（外层已套 `require_auth`）。
///
/// 原 Java 没有 `/api/user/coupon/my`，故不注册该别名。
pub fn authorized_routes() -> Router<PublicState> {
    Router::new()
        .route("/user/coupon/list", get(my_coupons))
        .route("/user/coupon/detail", get(my_coupon_detail))
        .route("/user/coupon/claim", post(claim))
        .route("/user/coupon/del", post(delete_my_coupon))
}

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    [
        ("GET", "/api/user/coupon/getList", "UserCouponController.getList"),
        ("GET", "/api/promotion/coupon/listByIds", "CouponClientController.listByIds"),
        ("GET", "/api/user/coupon/list", "UserCouponController.list"),
        ("GET", "/api/user/coupon/detail", "UserCouponController.detail"),
        ("POST", "/api/user/coupon/claim", "UserCouponController.claim"),
        ("POST", "/api/user/coupon/del", "UserCouponController.del"),
    ]
    .iter()
    .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, p, j))
    .collect()
}

/// `GET /api/user/coupon/getList`：可领取的优惠券列表。
///
/// 携带有效登录态时返回 `isReceive` 标记并过滤会员等级不匹配的券；
/// 游客则隐藏会员限定券。
async fn available_list(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<CouponListQuery>,
) -> ApiResult<PageResult<nextrade_core::models::CouponClientVO>> {
    // 公开路由未挂 require_auth，登录态为可选
    let page = match auth.map(|Extension(a)| a).filter(|a| a.id > 0) {
        Some(user) => state.marketing.coupon.claimable_list_for_user(user.id, &q).await?,
        None => state.marketing.coupon.claimable_list(&q).await?,
    };
    ok(page)
}

#[derive(Debug, Deserialize)]
struct IdsQuery {
    /// 逗号分隔的券 ID，如 `1,2,3`
    ids: String,
}

/// `GET /api/promotion/coupon/listByIds`：按 ID 批量查询券。
async fn list_by_ids(
    State(state): State<PublicState>,
    Query(q): Query<IdsQuery>,
) -> ApiResult<Vec<nextrade_core::models::CouponClientVO>> {
    let ids: Vec<i32> = q
        .ids
        .split(',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .collect();
    ok(state.marketing.coupon.list_by_ids(&ids).await?)
}

/// `GET /api/user/coupon/list`：我的优惠券（需登录）。
async fn my_coupons(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<UserCouponListQuery>,
) -> ApiResult<PageResult<nextrade_core::models::UserCouponVO>> {
    ok(state.marketing.coupon.my_coupons(auth.id, &q).await?)
}

/// `GET /api/user/coupon/detail?id=`：券详情（含当前用户领取情况）。
async fn my_coupon_detail(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<IdQuery>,
) -> ApiResult<serde_json::Value> {
    // 原版前端进入 /coupon 时可能没有 query.id；此时返回空详情，不抛 400。
    let Some(coupon_id) = q.id else {
        return ok(serde_json::Value::Null);
    };
    let coupon = state.marketing.coupon.detail(coupon_id).await?;
    let usable = state.marketing.coupon.my_usable_coupons(auth.id).await?;
    let is_receive = usable.iter().any(|uc| uc.coupon_id == coupon_id);
    ok(serde_json::json!({
        "couponId": coupon.coupon_id,
        "couponName": coupon.coupon_name,
        "couponMoney": coupon.coupon_money,
        "couponDiscount": coupon.coupon_discount,
        "couponDesc": coupon.coupon_desc,
        "couponType": coupon.coupon_type,
        "minOrderAmount": coupon.min_order_amount,
        "sendType": coupon.send_type,
        "useDay": coupon.use_day,
        "useStartDate": coupon.use_start_date,
        "useEndDate": coupon.use_end_date,
        "isNewUser": coupon.is_new_user,
        "isReceive": i32::from(is_receive),
    }))
}

#[derive(Debug, Deserialize)]
struct IdQuery {
    id: Option<i32>,
}

/// `POST /api/user/coupon/claim`：领取优惠券（需登录）。
async fn claim(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<UserCouponClaimDTO>,
) -> ApiResult<i32> {
    ok(state.marketing.coupon.claim(auth.id, dto.coupon_id).await?)
}

/// `POST /api/user/coupon/del`：删除我的优惠券（需登录）。
async fn delete_my_coupon(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<UserCouponIdDTO>,
) -> ApiResult<bool> {
    state.marketing.coupon.delete_my_coupon(auth.id, dto.id).await?;
    ok(true)
}