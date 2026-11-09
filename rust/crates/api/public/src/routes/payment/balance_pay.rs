//! 商城前台余额支付路由。
//!
//! 首个真实落地的支付渠道（无需外部 SDK，站内余额直接扣减）。
//!
//! | 本模块路由 | 说明 |
//! |-----------|------|
//! | `GET  /balance/pay/info` | 查询余额与该订单可抵扣额（需登录） |
//! | `POST /balance/pay` | 用余额支付订单（需登录） |
//!
//! 路径为 Rust 新增渠道端点（原 Java 余额抵扣内联在下单流程中，
//! 无独立 HTTP 接口），会被覆盖率报告归入 `extra_routes`。

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::PublicState;

/// 余额支付域**需登录**路由（外层已套 `require_auth`）。
pub fn authorized_routes() -> Router<PublicState> {
    Router::new()
        .route("/balance/pay/info", get(info))
        .route("/balance/pay", post(pay))
}

/// 本域已实现路由（Rust 新增渠道，全部计入 extra_routes）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    // 原契约无对应端点（余额抵扣内联在下单流程），返回空列表。
    Vec::new()
}

/// 余额查询参数。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InfoQuery {
    order_id: i32,
}

/// `GET /api/balance/pay/info?orderId=`：余额与可抵扣额。
async fn info(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<InfoQuery>,
) -> ApiResult<serde_json::Value> {
    let order = state.payment.balance_pay
        .order_payment_context(q.order_id, auth.id)
        .await?;
    ok(serde_json::json!({
        "userBalance": order.user_balance,
        "unpaidAmount": order.unpaid_amount,
        "deductible": order.deductible,
        "fullyCoverable": order.fully_coverable,
        "alreadyPaid": order.already_paid,
    }))
}

/// 余额支付请求。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PayDTO {
    order_id: i32,
    /// 本次使用余额抵扣的金额；缺省表示「尽量抵扣」。
    amount: Option<Decimal>,
}

/// 余额支付响应。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PayVO {
    pay_sn: String,
    /// 实际扣除余额
    deducted: Decimal,
    /// 订单是否已全额付清
    order_fully_paid: bool,
}

/// `POST /api/balance/pay`：用余额支付订单。
async fn pay(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<PayDTO>,
) -> ApiResult<PayVO> {
    let result = state.payment.balance_pay
        .pay_order(auth.id, dto.order_id, dto.amount)
        .await?;
    ok(PayVO {
        pay_sn: result.pay_sn,
        deducted: result.deducted,
        order_fully_paid: result.order_fully_paid,
    })
}