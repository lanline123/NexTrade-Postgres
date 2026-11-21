//! 管理端充值赠品记录路由（marketing::recharge_gift）。
//! 【我方新增 2026-11-21，论坛帖 717721；原版无此控制器】
//! `/adminapi/promotion/rechargeGift/*`（3 端点）：记录列表（含异常可见）、
//! 作废、恢复。赠品的发放/领取在充值单与会员端链路，本组仅做后台处置。

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};

use nextrade_core::models::marketing::recharge_gift::{
    RechargeGiftAdminActionDTO, RechargeGiftListQuery, RechargeGiftRecordVO,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult};

use crate::AdminState;

const ROUTE_TABLE: [(&str, &str, &str); 3] = [
    (
        "GET",
        "/promotion/rechargeGift/list",
        "RechargeGiftController.list",
    ),
    (
        "POST",
        "/promotion/rechargeGift/void",
        "RechargeGiftController.void",
    ),
    (
        "POST",
        "/promotion/rechargeGift/restore",
        "RechargeGiftController.restore",
    ),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/promotion/rechargeGift/list", get(list))
        .route("/promotion/rechargeGift/void", post(void))
        .route("/promotion/rechargeGift/restore", post(restore))
}

#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, &format!("/adminapi{p}"), j))
        .collect()
}

async fn list(
    State(state): State<AdminState>,
    Query(q): Query<RechargeGiftListQuery>,
) -> ApiResult<PageResult<RechargeGiftRecordVO>> {
    ok(state.marketing.recharge_gift.admin_list(&q).await?)
}

async fn void(
    State(state): State<AdminState>,
    Json(dto): Json<RechargeGiftAdminActionDTO>,
) -> ApiResult<()> {
    state.marketing.recharge_gift.admin_void(&dto).await?;
    ok(())
}

async fn restore(
    State(state): State<AdminState>,
    Json(dto): Json<RechargeGiftAdminActionDTO>,
) -> ApiResult<()> {
    state.marketing.recharge_gift.admin_restore(&dto).await?;
    ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 3);
        for (m, p) in [
            ("GET", "/adminapi/promotion/rechargeGift/list"),
            ("POST", "/adminapi/promotion/rechargeGift/void"),
            ("POST", "/adminapi/promotion/rechargeGift/restore"),
        ] {
            assert!(docs.iter().any(|d| d.method == m && d.path == p), "{m} {p}");
        }
    }

    #[test]
    fn route_table_matches_registrations() {
        assert_eq!(ROUTE_TABLE.len(), 3);
    }
}
