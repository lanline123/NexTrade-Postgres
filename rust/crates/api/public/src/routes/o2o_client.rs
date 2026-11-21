//! o2o 前台 —— 移动端本地生活路由（o2o_client）。
//!
//! b2b2b 5.8.57 移动端新增（**无 Java 控制器对应**），按表 + 前端消费实现：
//!
//! | 本模块路由 | 说明 |
//! |-----------|------|
//! | `GET /o2o/userPickup/getUserPickups` | 当前会员自提人列表 |
//! | `GET /o2o/userPickup/detail?id=` | 自提人详情 |
//! | `POST /o2o/userPickup/create` | 新增自提人 |
//! | `POST /o2o/userPickup/update` | 更新自提人 |
//! | `POST /o2o/userPickup/del` | 删除自提人 |
//! | `GET /o2o/orderPickup/getShopPickupTpl?shopId=` | 门店自提模板 |
//! | `GET /o2o/orderPickup/getOrderPickupShipment?orderId=` | 订单自提单 |
//! | `GET /o2o/orderPickup/getDeliveryOptions?orderId=` | 配送方式（自提单存在=自提） |
//! | `GET /o2o/OrderReservation/getStockCalendar?productId=&shopId=&days=` | 预约库存日历 |
//! | `GET /o2o/storeSearch/getStoreProduct` | 门店商品搜索 |
//!
//! 自提人接口需登录；模板/配送/日历/门店商品匿名。

use axum::{
    extract::{Query, Request, State},
    http::header,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
use serde_json::Value;
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult, AuthUser};
use nextrade_services::o2o::pickup_client::StockDayVO;

use crate::PublicState;

/// 本域路由（挂载于 `/api` 下）。
const ROUTE_TABLE: [(&str, &str, &str); 36] = [
    ("GET", "/o2o/userPickup/getUserPickups", "UserPickupController.getUserPickups"),
    ("GET", "/o2o/userPickup/detail", "UserPickupController.detail"),
    ("POST", "/o2o/userPickup/create", "UserPickupController.create"),
    ("POST", "/o2o/userPickup/update", "UserPickupController.update"),
    ("POST", "/o2o/userPickup/del", "UserPickupController.del"),
    ("GET", "/o2o/orderPickup/getShopPickupTpl", "OrderPickupController.getShopPickupTpl"),
    ("GET", "/o2o/orderPickup/getOrderPickupShipment", "OrderPickupController.getOrderPickupShipment"),
    ("GET", "/o2o/orderPickup/getDeliveryOptions", "OrderPickupController.getDeliveryOptions"),
    ("GET", "/o2o/OrderReservation/getStockCalendar", "OrderReservationController.getStockCalendar"),
    ("GET", "/o2o/storeSearch/getStoreProduct", "StoreSearchController.getStoreProduct"),
    ("POST", "/o2o/deliveryOrder/grab", "DeliveryOrderController.grab"),
    ("POST", "/o2o/deliveryOrder/confirmPickup", "DeliveryOrderController.confirmPickup"),
    ("POST", "/o2o/deliveryOrder/complete", "DeliveryOrderController.complete"),
    ("POST", "/o2o/deliveryOrder/cancel", "DeliveryOrderController.cancel"),
    ("POST", "/o2o/deliveryOrder/removeCancelled", "DeliveryOrderController.removeCancelled"),
    ("GET", "/o2o/deliveryOrder/statistics", "DeliveryOrderController.statistics"),
    ("GET", "/o2o/deliveryOrder/myList", "DeliveryOrderController.myList"),
    ("GET", "/o2o/deliveryOrder/myListTotalCount", "DeliveryOrderController.myListTotalCount"),
    ("GET", "/o2o/deliveryOrder/grabList", "DeliveryOrderController.grabList"),
    ("GET", "/o2o/deliveryOrder/detail", "DeliveryOrderController.detail"),
    ("GET", "/o2o/deliveryOrder/buyerProgress", "DeliveryOrderController.buyerProgress"),
    ("GET", "/o2o/deliveryOrder/myListTabCounts", "DeliveryOrderController.myListTabCounts"),
    ("GET", "/o2o/serviceStaff/myInfo", "ServiceStaffController.myInfo"),
    ("POST", "/o2o/serviceStaff/apply", "ServiceStaffController.apply"),
    ("GET", "/o2o/serviceStaff/myApplies", "ServiceStaffController.myApplies"),
    ("POST", "/o2o/serviceStaff/updateWorkStatus", "ServiceStaffController.updateWorkStatus"),
    ("GET", "/o2o/staffWallet/myWallet", "StaffWalletController.myWallet"),
    ("GET", "/o2o/staffWallet/myLogs", "StaffWalletController.myLogs"),
    ("GET", "/o2o/staffWallet/withdrawConfig", "StaffWalletController.withdrawConfig"),
    ("GET", "/o2o/staffWallet/myWithdrawList", "StaffWalletController.myWithdrawList"),
    ("GET", "/o2o/staffWallet/myWithdrawDetail", "StaffWalletController.myWithdrawDetail"),
    ("GET", "/o2o/staffAccount/list", "StaffAccountController.list"),
    ("POST", "/o2o/staffAccount/save", "StaffAccountController.save"),
    ("POST", "/o2o/staffAccount/del", "StaffAccountController.del"),
    ("POST", "/o2o/staffWallet/applyWithdraw", "StaffWalletController.applyWithdraw"),
    ("GET", "/o2o/config/localDeliveryConfig", "O2oConfigController.localDeliveryConfig"),
    ("GET", "/o2o/config/homeServiceConfig", "O2oConfigController.homeServiceConfig"),
];

/// o2o 前台路由（挂载于 `/api` 下）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let pickup = Router::new()
        .route("/o2o/userPickup/getUserPickups", get(get_user_pickups))
        .route("/o2o/userPickup/detail", get(pickup_detail))
        .route("/o2o/userPickup/create", post(pickup_create))
        .route("/o2o/userPickup/update", post(pickup_update))
        .route("/o2o/userPickup/del", post(pickup_del))
        .layer(middleware::from_fn_with_state(
            state.jwt.clone(),
            optional_auth,
        ));
    let delivery = Router::new()
        .route("/o2o/deliveryOrder/grab", post(delivery_grab))
        .route("/o2o/deliveryOrder/confirmPickup", post(delivery_confirm))
        .route("/o2o/deliveryOrder/complete", post(delivery_complete))
        .route("/o2o/deliveryOrder/cancel", post(delivery_cancel))
        .route("/o2o/deliveryOrder/removeCancelled", post(delivery_remove_cancelled))
        .route("/o2o/deliveryOrder/statistics", get(delivery_statistics))
        .route("/o2o/deliveryOrder/myList", get(delivery_my_list))
        .route("/o2o/deliveryOrder/myListTotalCount", get(delivery_total_count))
        .route("/o2o/deliveryOrder/grabList", get(delivery_grab_list))
        .route("/o2o/deliveryOrder/detail", get(delivery_detail))
        .route("/o2o/deliveryOrder/buyerProgress", get(buyer_progress))
        .route("/o2o/deliveryOrder/myListTabCounts", get(delivery_tab_counts))
        .route("/o2o/serviceStaff/myInfo", get(staff_my_info))
        .route("/o2o/serviceStaff/apply", post(staff_apply))
        .route("/o2o/serviceStaff/myApplies", get(staff_my_applies))
        .route("/o2o/serviceStaff/updateWorkStatus", post(staff_update_work_status))
        .route("/o2o/staffWallet/myWallet", get(staff_wallet))
        .route("/o2o/staffWallet/myLogs", get(staff_wallet_logs))
        .route("/o2o/staffWallet/withdrawConfig", get(staff_withdraw_config))
        .route("/o2o/staffWallet/myWithdrawList", get(staff_withdraw_list))
        .route("/o2o/staffWallet/myWithdrawDetail", get(staff_withdraw_detail))
        .route("/o2o/staffAccount/list", get(staff_accounts))
        .route("/o2o/staffAccount/save", post(staff_account_save))
        .route("/o2o/staffAccount/del", post(staff_account_del))
        .route("/o2o/staffWallet/applyWithdraw", post(staff_apply_withdraw))
        .route("/o2o/config/localDeliveryConfig", get(local_delivery_config))
        .route("/o2o/config/homeServiceConfig", get(home_service_config))
        .layer(middleware::from_fn_with_state(
            state.jwt.clone(),
            optional_auth,
        ));
    Router::new()
        .merge(pickup)
        .merge(delivery)
        .route(
            "/o2o/orderPickup/getShopPickupTpl",
            get(shop_pickup_tpl),
        )
        .route(
            "/o2o/orderPickup/getOrderPickupShipment",
            get(order_pickup_shipment),
        )
        .route("/o2o/orderPickup/getDeliveryOptions", get(delivery_options))
        .route(
            "/o2o/OrderReservation/getStockCalendar",
            get(stock_calendar),
        )
        .route("/o2o/storeSearch/getStoreProduct", get(store_product))
}

/// 已实现路由（契约完整路径）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/api{path}"), handler))
        .collect()
}

// ————————————————————— 自提人 —————————————————————

async fn get_user_pickups(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
) -> ApiResult<Vec<nextrade_infrastructure::db::repo::o2o::pickup_client::UserPickupVO>> {
    ok(state.o2o.pickup_client.list_pickups(current_user_id(auth)?).await?)
}

#[derive(Debug, Default, Deserialize)]
pub struct PickupIdQuery {
    pub id: Option<i32>,
}

async fn pickup_detail(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<PickupIdQuery>,
) -> ApiResult<Value> {
    let vo = state
        .o2o
        .pickup_client
        .pickup_detail(current_user_id(auth)?, q.id.unwrap_or(0))
        .await?;
    ok(serde_json::to_value(vo).unwrap_or(Value::Null))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PickupSaveDTO {
    pub user_pickup_id: Option<i32>,
    pub consignee: Option<String>,
    pub mobile: Option<String>,
    pub mobile_area_code: Option<String>,
    pub is_default: Option<bool>,
}

async fn pickup_create(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(dto): Json<PickupSaveDTO>,
) -> ApiResult<i32> {
    let id = state
        .o2o
        .pickup_client
        .create_pickup(
            current_user_id(auth)?,
            dto.consignee.as_deref().unwrap_or(""),
            dto.mobile.as_deref().unwrap_or(""),
            dto.mobile_area_code.as_deref().unwrap_or(""),
            dto.is_default.unwrap_or(false),
        )
        .await?;
    ok(id)
}

async fn pickup_update(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(dto): Json<PickupSaveDTO>,
) -> ApiResult<()> {
    state
        .o2o
        .pickup_client
        .update_pickup(
            current_user_id(auth)?,
            dto.user_pickup_id.unwrap_or(0),
            dto.consignee.as_deref(),
            dto.mobile.as_deref(),
            dto.mobile_area_code.as_deref(),
            dto.is_default,
        )
        .await?;
    ok(())
}

async fn pickup_del(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(dto): Json<PickupIdQuery>,
) -> ApiResult<()> {
    state
        .o2o
        .pickup_client
        .delete_pickup(current_user_id(auth)?, dto.id.unwrap_or(0))
        .await?;
    ok(())
}

/// 当前会员 id（未登录 → 401「请先登录」，与 cart_summary 同口径）。
// ————————————————————— C 端派送/服务订单 —————————————————————

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct DeliveryListQuery {
    page: Option<u64>,
    size: Option<u64>,
    tab: Option<String>,
    r#type: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct DeliveryIdQuery {
    id: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct DeliveryAction {
    delivery_order_id: Option<i32>,
    #[serde(default)]
    photo: Vec<String>,
    cancel_reason: Option<String>,
    /// 状态动作点定位（阶段三：H5 端动作时上报当前位置，可空）。
    longitude: Option<f64>,
    latitude: Option<f64>,
}

impl DeliveryAction {
    fn coords(&self) -> (Option<rust_decimal::Decimal>, Option<rust_decimal::Decimal>) {
        (
            self.longitude.and_then(rust_decimal::Decimal::from_f64_retain),
            self.latitude.and_then(rust_decimal::Decimal::from_f64_retain),
        )
    }
}

/// 动作成功后 best-effort 打点（定位失败/缺失不影响业务结果）。
async fn track(
    state: &PublicState,
    staff_id: i32,
    dto: &DeliveryAction,
    action: &str,
) {
    let (lng, lat) = dto.coords();
    let _ = state
        .o2o
        .pickup_client
        .record_track_point(staff_id, dto.delivery_order_id.unwrap_or(0), action, lng, lat)
        .await;
}

async fn delivery_staff_of(
    state: &PublicState,
    uid: i32,
) -> std::result::Result<(i32, String, String), nextrade_security::web::AppError> {
    state
        .o2o
        .pickup_client
        .delivery_staff(uid)
        .await?
        .map(|(id, name, phone, _)| (id, name, phone))
        .ok_or_else(|| nextrade_security::web::AppError(nextrade_core::Error::biz(1001, "当前账号未绑定服务人员")))
}

/// 骑手 MVP：解析当前会员的服务人员完整上下文（大厅/抢单的作用域与状态校验）。
async fn delivery_staff_ctx(
    state: &PublicState,
    uid: i32,
) -> std::result::Result<nextrade_services::o2o::pickup_client::StaffContext, nextrade_security::web::AppError> {
    state
        .o2o
        .pickup_client
        .staff_context(uid)
        .await?
        .ok_or_else(|| nextrade_security::web::AppError(nextrade_core::Error::biz(1001, "当前账号未绑定服务人员")))
}

async fn delivery_my_list(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<DeliveryListQuery>,
) -> ApiResult<Value> {
    let (staff_id, _, _) = delivery_staff_of(&state, current_user_id(auth)?).await?;
    let page = q.page.unwrap_or(1).max(1);
    let size = q.size.unwrap_or(10).clamp(1, 100);
    let (total, records) = state
        .o2o
        .pickup_client
        .delivery_list(staff_id, q.tab.as_deref(), page, size)
        .await?;
    ok(serde_json::json!({"records": records, "total": total, "size": size, "current": page, "pages": (total as u64 + size - 1) / size}))
}

async fn delivery_total_count(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
) -> ApiResult<i64> {
    let (staff_id, _, _) = delivery_staff_of(&state, current_user_id(auth)?).await?;
    let (total, _) = state
        .o2o
        .pickup_client
        .delivery_list(staff_id, None, 1, 1)
        .await?;
    ok(total)
}

/// 抢单大厅（骑手 MVP 修复：原实现**不验身份**、全平台一个大厅；
/// 现强制登录 + 人员绑定，并按人员所属店铺 + 类型作用域过滤；
/// 已下班/被禁用人员返回空大厅并在 records 外提示状态不可接单）。
async fn delivery_grab_list(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<DeliveryListQuery>,
) -> ApiResult<Value> {
    let staff = delivery_staff_ctx(&state, current_user_id(auth)?).await?;
    let page = q.page.unwrap_or(1).max(1);
    let size = q.size.unwrap_or(10).clamp(1, 100);
    if !staff.can_take_orders() {
        return ok(serde_json::json!({"records": [], "total": 0, "size": size, "current": page, "pages": 0, "canTakeOrders": false}));
    }
    let (total, records) = state
        .o2o
        .pickup_client
        .grab_list(&staff, page, size, q.r#type)
        .await?;
    ok(serde_json::json!({"records": records, "total": total, "size": size, "current": page, "pages": (total as u64 + size - 1) / size, "canTakeOrders": true}))
}

async fn delivery_detail(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<DeliveryIdQuery>,
) -> ApiResult<Value> {
    let (staff_id, _, _) = delivery_staff_of(&state, current_user_id(auth)?).await?;
    ok(state
        .o2o
        .pickup_client
        .delivery_detail(staff_id, q.id.unwrap_or(0))
        .await?
        .unwrap_or(Value::Null))
}

async fn delivery_grab(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(dto): Json<DeliveryAction>,
) -> ApiResult<()> {
    let staff = delivery_staff_ctx(&state, current_user_id(auth)?).await?;
    if !staff.can_take_orders() {
        return Err(nextrade_security::web::AppError(nextrade_core::Error::biz(1001, "当前状态不可接单（已下班或被禁用）")));
    }
    let ok_flag = state
        .o2o
        .pickup_client
        .grab_delivery(&staff, dto.delivery_order_id.unwrap_or(0))
        .await?;
    if !ok_flag {
        return Err(nextrade_security::web::AppError(nextrade_core::Error::biz(1001, "该订单已被抢或不可抢")));
    }
    track(&state, staff.id, &dto, "grab").await;
    ok(())
}

async fn delivery_confirm(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(dto): Json<DeliveryAction>,
) -> ApiResult<()> {
    let (staff_id, _, _) = delivery_staff_of(&state, current_user_id(auth)?).await?;
    let photos = serde_json::to_string(&dto.photo).unwrap_or_else(|_| "[]".into());
    let ok_flag = state
        .o2o
        .pickup_client
        .confirm_pickup(staff_id, dto.delivery_order_id.unwrap_or(0), &photos)
        .await?;
    if !ok_flag {
        return Err(nextrade_security::web::AppError(nextrade_core::Error::biz(1001, "订单状态不允许确认取货")));
    }
    track(&state, staff_id, &dto, "pickup").await;
    ok(())
}

/// 完成配送/服务：状态翻转 + 服务费入账（骑手 MVP 激活）。回执带本次入账金额。
async fn delivery_complete(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(dto): Json<DeliveryAction>,
) -> ApiResult<Value> {
    let (staff_id, _, _) = delivery_staff_of(&state, current_user_id(auth)?).await?;
    let photos = serde_json::to_string(&dto.photo).unwrap_or_else(|_| "[]".into());
    let fee = state
        .o2o
        .pickup_client
        .complete_delivery(staff_id, dto.delivery_order_id.unwrap_or(0), &photos)
        .await?;
    match fee {
        Some(fee) => {
            track(&state, staff_id, &dto, "complete").await;
            ok(serde_json::json!({"serviceFee": fee}))
        }
        None => Err(nextrade_security::web::AppError(nextrade_core::Error::biz(1001, "订单状态不允许完成"))),
    }
}

async fn delivery_cancel(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(dto): Json<DeliveryAction>,
) -> ApiResult<()> {
    let (staff_id, _, _) = delivery_staff_of(&state, current_user_id(auth)?).await?;
    let ok_flag = state
        .o2o
        .pickup_client
        .cancel_delivery(
            staff_id,
            dto.delivery_order_id.unwrap_or(0),
            dto.cancel_reason.as_deref().unwrap_or_default(),
        )
        .await?;
    if !ok_flag {
        return Err(nextrade_security::web::AppError(nextrade_core::Error::biz(1001, "订单状态不允许取消")));
    }
    ok(())
}

/// 配送员移除已取消单（论坛 417866，我方新增）：把指派给自己的已取消
/// 派送单从本人列表移除（隐藏标记），数据不删、后台仍可查。非本人 /
/// 非已取消 / 已移除过 → 业务错误，不静默成功。
async fn delivery_remove_cancelled(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(dto): Json<DeliveryAction>,
) -> ApiResult<()> {
    let (staff_id, _, _) = delivery_staff_of(&state, current_user_id(auth)?).await?;
    let ok_flag = state
        .o2o
        .pickup_client
        .hide_cancelled_delivery(staff_id, dto.delivery_order_id.unwrap_or(0))
        .await?;
    if !ok_flag {
        return Err(nextrade_security::web::AppError(nextrade_core::Error::biz(
            1001,
            "仅可移除本人已取消的派送单",
        )));
    }
    ok(())
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct BuyerProgressQuery {
    order_id: Option<i32>,
}

/// 买家侧派送进度（骑手 MVP 阶段三）：按主订单归属校验后返回派送单状态与
/// 步骤流（待接单 → 已接单 → 已取货/开始服务 → 已送达/完成；取消单附取消步）。
/// 非本人订单/无派送单 → data 为 null（不泄露他人派送信息）。
async fn buyer_progress(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<BuyerProgressQuery>,
) -> ApiResult<Value> {
    let uid = current_user_id(auth)?;
    let order_id = q.order_id.unwrap_or(0);
    let owner = state.o2o.pickup_client.order_owner(order_id).await?;
    if owner != Some(uid) {
        return ok(Value::Null);
    }
    let Some(mut progress) = state.o2o.pickup_client.buyer_delivery(order_id).await? else {
        return ok(Value::Null);
    };
    let status = progress.get("status").and_then(Value::as_i64).unwrap_or(0);
    let dtype = progress.get("type").and_then(Value::as_i64).unwrap_or(1);
    let ts = |k: &str| progress.get(k).cloned().unwrap_or(Value::Null);
    let accepted_time = {
        let g = ts("grabTime");
        if g.is_null() { ts("dispatchTime") } else { g }
    };
    let mut steps = vec![
        serde_json::json!({"key": "created", "label": "已下单，等待接单", "time": ts("createTime"), "done": true}),
        serde_json::json!({"key": "accepted", "label": if dtype == 2 { "服务人员已接单" } else { "骑手已接单" }, "time": accepted_time, "done": status >= 20 && status != 50}),
        serde_json::json!({"key": "picked", "label": if dtype == 2 { "已开始服务" } else { "已取货，配送中" }, "time": ts("pickupTime"), "done": status >= 30 && status != 50}),
        serde_json::json!({"key": "delivered", "label": if dtype == 2 { "服务完成" } else { "已送达" }, "time": ts("completeTime"), "done": status == 40}),
    ];
    if status == 50 {
        steps.push(serde_json::json!({"key": "cancelled", "label": "已取消", "time": ts("cancelTime"), "done": true}));
    }
    progress["steps"] = Value::Array(steps);
    ok(progress)
}

async fn delivery_statistics(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
) -> ApiResult<Value> {
    let (staff_id, _, _) = delivery_staff_of(&state, current_user_id(auth)?).await?;
    let (total, _) = state.o2o.pickup_client.delivery_list(staff_id, None, 1, 1).await?;
    let (pending, _) = state.o2o.pickup_client.delivery_list(staff_id, Some("pending"), 1, 1).await?;
    let (processing, _) = state.o2o.pickup_client.delivery_list(staff_id, Some("processing"), 1, 1).await?;
    let (completed, _) = state.o2o.pickup_client.delivery_list(staff_id, Some("completed"), 1, 1).await?;
    let (cancelled, _) = state.o2o.pickup_client.delivery_list(staff_id, Some("cancelled"), 1, 1).await?;
    ok(serde_json::json!({"total": total, "pending": pending, "processing": processing, "completed": completed, "cancelled": cancelled}))
}

async fn delivery_tab_counts(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
) -> ApiResult<Value> {
    delivery_statistics(State(state), auth).await
}

// ————————————————————— C 端服务人员资料 / 钱包 —————————————————————

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct WorkStatusBody {
    work_status: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct StaffApplyDto {
    shop_id: Option<i32>,
    r#type: Option<i32>,
    name: Option<String>,
    phone: Option<String>,
    id_card: Option<String>,
    remark: Option<String>,
}

/// 入驻申请（骑手 MVP 阶段二）：会员申请成为某店铺的配送员/服务员，
/// 审核在 admin「入驻审核」页，通过后生成 service_staff 记录。
async fn staff_apply(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(dto): Json<StaffApplyDto>,
) -> ApiResult<i32> {
    let uid = current_user_id(auth)?;
    let apply_id = state
        .o2o
        .pickup_client
        .staff_apply(
            uid,
            dto.shop_id.unwrap_or(0),
            dto.r#type.unwrap_or(1),
            dto.name.as_deref().unwrap_or_default(),
            dto.phone.as_deref().unwrap_or_default(),
            dto.id_card.as_deref().unwrap_or_default(),
            dto.remark.as_deref().unwrap_or_default(),
        )
        .await?;
    ok(apply_id)
}

/// 我的入驻申请（含审核状态）。
async fn staff_my_applies(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
) -> ApiResult<Value> {
    let uid = current_user_id(auth)?;
    let rows = state.o2o.pickup_client.my_applies(uid).await?;
    ok(serde_json::json!({"records": rows}))
}

async fn staff_my_info(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
) -> ApiResult<Value> {
    let uid = current_user_id(auth)?;
    ok(state
        .o2o
        .pickup_client
        .staff_info(uid)
        .await?
        .unwrap_or(Value::Null))
}

async fn staff_update_work_status(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(body): Json<WorkStatusBody>,
) -> ApiResult<()> {
    let uid = current_user_id(auth)?;
    let ok_flag = state
        .o2o
        .pickup_client
        .update_work_status(uid, body.work_status.unwrap_or(0))
        .await?;
    if !ok_flag {
        return Err(nextrade_security::web::AppError(nextrade_core::Error::biz(1001, "当前账号未绑定服务人员")));
    }
    ok(())
}

async fn staff_wallet(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
) -> ApiResult<Value> {
    let (staff_id, _, _) = delivery_staff_of(&state, current_user_id(auth)?).await?;
    ok(state.o2o.pickup_client.my_wallet(staff_id).await?)
}

async fn staff_wallet_logs(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<DeliveryListQuery>,
) -> ApiResult<Value> {
    let (staff_id, _, _) = delivery_staff_of(&state, current_user_id(auth)?).await?;
    let page = q.page.unwrap_or(1).max(1);
    let size = q.size.unwrap_or(10).clamp(1, 100);
    let (total, records) = state
        .o2o
        .pickup_client
        .my_wallet_logs(staff_id, page, size)
        .await?;
    ok(serde_json::json!({"records": records, "total": total, "size": size, "current": page, "pages": (total as u64 + size - 1) / size}))
}

// ————————————————————— C 端收款账号 / 提现 —————————————————————

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct StaffAccountBody {
    account_id: Option<i32>,
    account_type: Option<i32>,
    account_name: Option<String>,
    account_no: Option<String>,
    bank_name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct WithdrawListQuery {
    status: Option<i32>,
    page: Option<u64>,
    size: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct WithdrawDetailQuery {
    withdraw_id: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct AccountIdQuery {
    account_id: Option<i32>,
}

async fn staff_accounts(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
) -> ApiResult<Value> {
    let uid = current_user_id(auth)?;
    ok(Value::Array(state.o2o.pickup_client.accounts(uid).await?))
}

async fn staff_account_save(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(body): Json<StaffAccountBody>,
) -> ApiResult<i32> {
    let uid = current_user_id(auth)?;
    ok(state
        .o2o
        .pickup_client
        .save_account(
            uid,
            body.account_id.unwrap_or(0),
            body.account_type.unwrap_or(1),
            body.account_name.as_deref().unwrap_or_default(),
            body.account_no.as_deref().unwrap_or_default(),
            body.bank_name.as_deref().unwrap_or_default(),
        )
        .await?)
}

async fn staff_account_del(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<AccountIdQuery>,
) -> ApiResult<()> {
    let uid = current_user_id(auth)?;
    state
        .o2o
        .pickup_client
        .delete_account(uid, q.account_id.unwrap_or(0))
        .await?;
    ok(())
}

async fn staff_withdraw_config(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
) -> ApiResult<Value> {
    let uid = current_user_id(auth)?;
    let (_, _, _, staff_type) = state
        .o2o
        .pickup_client
        .delivery_staff(uid)
        .await?
        .ok_or_else(|| nextrade_security::web::AppError(nextrade_core::Error::biz(1001, "当前账号未绑定服务人员")))?;
    let code = if staff_type == 2 {
        "o2oServerWithdrawConfig"
    } else {
        "o2oDeliveryWithdrawConfig"
    };
    let raw = state.system.config_basic.repo().find_by_code(code).await?;
    let value = raw
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    ok(value)
}

async fn staff_withdraw_list(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<WithdrawListQuery>,
) -> ApiResult<Value> {
    let (staff_id, _, _) = delivery_staff_of(&state, current_user_id(auth)?).await?;
    let page = q.page.unwrap_or(1).max(1);
    let size = q.size.unwrap_or(10).clamp(1, 100);
    let (total, records) = state
        .o2o
        .pickup_client
        .withdraw_list(staff_id, q.status, page, size)
        .await?;
    ok(serde_json::json!({"records": records, "total": total, "size": size, "current": page, "pages": (total as u64 + size - 1) / size}))
}

async fn staff_withdraw_detail(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<WithdrawDetailQuery>,
) -> ApiResult<Value> {
    let (staff_id, _, _) = delivery_staff_of(&state, current_user_id(auth)?).await?;
    ok(state
        .o2o
        .pickup_client
        .withdraw_detail(staff_id, q.withdraw_id.unwrap_or(0))
        .await?
        .unwrap_or(Value::Null))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ApplyWithdrawBody {
    amount: Option<Value>,
    account_id: Option<i32>,
    remark: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ShopConfigQuery {
    shop_id: Option<i32>,
}

async fn staff_apply_withdraw(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(body): Json<ApplyWithdrawBody>,
) -> ApiResult<i32> {
    use std::str::FromStr;
    let (staff_id, _, _, staff_type) = state
        .o2o
        .pickup_client
        .delivery_staff(current_user_id(auth)?)
        .await?
        .ok_or_else(|| nextrade_security::web::AppError(nextrade_core::Error::biz(1001, "当前账号未绑定服务人员")))?;
    let amount_text = match body.amount.as_ref() {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        _ => return Err(nextrade_security::web::AppError(nextrade_core::Error::Validation("提现金额不能为空".into()))),
    };
    let amount = rust_decimal::Decimal::from_str(&amount_text)
        .map_err(|_| nextrade_security::web::AppError(nextrade_core::Error::Validation("提现金额格式错误".into())))?;
    let business_type = if staff_type == 2 { 2 } else { 1 };
    ok(state
        .o2o
        .pickup_client
        .apply_withdraw(
            staff_id,
            business_type,
            amount,
            body.account_id.unwrap_or(0),
            body.remark.as_deref().unwrap_or_default(),
        )
        .await?)
}

async fn local_delivery_config(
    State(state): State<PublicState>,
    Query(q): Query<ShopConfigQuery>,
) -> ApiResult<Value> {
    ok(state
        .o2o
        .pickup_client
        .local_delivery_config(q.shop_id.unwrap_or(0))
        .await?)
}

async fn home_service_config(
    State(state): State<PublicState>,
    Query(q): Query<ShopConfigQuery>,
) -> ApiResult<Value> {
    ok(state
        .o2o
        .pickup_client
        .home_service_config(q.shop_id.unwrap_or(0))
        .await?)
}

fn current_user_id(auth: Option<Extension<AuthUser>>) -> std::result::Result<i32, nextrade_security::web::AppError> {
    auth.map(|Extension(user)| user.id)
        .filter(|id| *id > 0)
        .ok_or_else(|| nextrade_security::web::AppError(nextrade_core::Error::Unauthorized("请先登录".into())))
}

/// 可选登录态中间件（cart_summary.rs 同款）。
async fn optional_auth(
    State(jwt): State<nextrade_security::JwtService>,
    mut req: Request,
    next: Next,
) -> Result<Response, nextrade_security::web::AppError> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .filter(|t| !t.trim().eq_ignore_ascii_case("bearer null") && !t.trim().is_empty())
        .map(str::to_string);
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

// ————————————————————— 自提模板 / 配送单 —————————————————————

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PickupTplQuery {
    pub shop_id: Option<i64>,
}

async fn shop_pickup_tpl(
    State(state): State<PublicState>,
    Query(q): Query<PickupTplQuery>,
) -> ApiResult<Value> {
    let vo = state
        .o2o
        .pickup_client
        .shop_pickup_tpl(q.shop_id.unwrap_or(0))
        .await?;
    ok(serde_json::to_value(vo).unwrap_or(Value::Null))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OrderPickupQuery {
    pub order_id: Option<i32>,
}

async fn order_pickup_shipment(
    State(state): State<PublicState>,
    Query(q): Query<OrderPickupQuery>,
) -> ApiResult<Value> {
    let vo = state
        .o2o
        .pickup_client
        .order_shipment(q.order_id.unwrap_or(0))
        .await?;
    ok(serde_json::to_value(vo).unwrap_or(Value::Null))
}

/// 配送方式：存在自提单 → 自提（pickup），否则为空列表（物流引擎未迁移）。
async fn delivery_options(
    State(state): State<PublicState>,
    Query(q): Query<OrderPickupQuery>,
) -> ApiResult<Value> {
    let shipment = state
        .o2o
        .pickup_client
        .order_shipment(q.order_id.unwrap_or(0))
        .await?;
    let options = match shipment {
        Some(_) => serde_json::json!([{
            "type": "pickup",
            "name": "到店自提",
        }]),
        None => serde_json::json!([]),
    };
    ok(options)
}

// ————————————————————— 库存日历 / 门店商品 —————————————————————

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StockCalendarQuery {
    pub product_id: Option<i32>,
    pub shop_id: Option<i64>,
    pub days: Option<u32>,
}

async fn stock_calendar(
    State(state): State<PublicState>,
    Query(q): Query<StockCalendarQuery>,
) -> ApiResult<Vec<StockDayVO>> {
    ok(state
        .o2o
        .pickup_client
        .stock_calendar(
            q.product_id.unwrap_or(0),
            q.shop_id,
            q.days.unwrap_or(14).clamp(1, 31),
        )
        .await?)
}

/// 门店商品搜索：按 shop_id 过滤上架商品（名称前缀/模糊匹配），分页。
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StoreProductQuery {
    pub shop_id: Option<i64>,
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub size: Option<u64>,
}

async fn store_product(
    State(state): State<PublicState>,
    Query(q): Query<StoreProductQuery>,
) -> ApiResult<Value> {
    let page = q.page.unwrap_or(1).max(1);
    let size = q.size.unwrap_or(10).clamp(1, 50);
    let (total, rows) = state
        .o2o
        .pickup_client
        .store_product_search(
            q.shop_id.unwrap_or(0),
            q.keyword.as_deref().unwrap_or_default(),
            page,
            size,
        )
        .await?;
    ok(serde_json::json!({ "records": rows, "total": total, "page": page, "size": size }))
}
