//! 管理端后厨出单路由（trade::kitchen，我方新增，论坛帖 817927）。
//!
//! | 本模块路由（挂载于 `/adminapi`） | 用途 |
//! |----------------------------------|------|
//! | `GET  /order/kitchen/board` | 后厨看板（计数 + 订单卡，含催菜标记） |
//! | `POST /order/kitchen/transition` | 后厨状态流转（待制作→制作中→已出单，只进不退） |
//! | `POST /order/kitchen/ackUrge` | 确认已知晓催菜（清除未读标记） |
//!
//! > **多租户**：店铺账号按 JWT `scoped_shop_id()` 强制本店范围，
//! > 平台账号可看全部（可按 `shopId` 过滤）；流转/知晓在服务层
//! > 二次校验订单店铺归属。本模块注册在管理端需登录路由组。

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use nextrade_core::models::trade::kitchen::{
    KitchenAckUrgeDTO, KitchenBoardQuery, KitchenBoardVO, KitchenInfoVO, KitchenTransitionDTO,
};
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::AdminState;

/// 本域 3 条路由（`/adminapi` 前缀之后 + 处理器名）。
const ROUTE_TABLE: [(&str, &str, &str); 3] = [
    ("GET", "/order/kitchen/board", "KitchenController.board"),
    (
        "POST",
        "/order/kitchen/transition",
        "KitchenController.transition",
    ),
    ("POST", "/order/kitchen/ackUrge", "KitchenController.ackUrge"),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/order/kitchen/board", get(board))
        .route("/order/kitchen/transition", post(transition))
        .route("/order/kitchen/ackUrge", post(ack_urge))
}

/// 本域已实现路由（完整契约路径，用于覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/adminapi{path}"), handler))
        .collect()
}

/// `GET /adminapi/order/kitchen/board`：后厨看板。
async fn board(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<KitchenBoardQuery>,
) -> ApiResult<KitchenBoardVO> {
    ok(state
        .trade
        .kitchen
        .board(auth.scoped_shop_id(), &q)
        .await?)
}

/// `POST /adminapi/order/kitchen/transition`：后厨状态流转。
async fn transition(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<KitchenTransitionDTO>,
) -> ApiResult<KitchenInfoVO> {
    ok(state
        .trade
        .kitchen
        .transition(auth.scoped_shop_id(), dto.order_id, dto.target_status)
        .await?)
}

/// `POST /adminapi/order/kitchen/ackUrge`：确认已知晓催菜。
async fn ack_urge(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<KitchenAckUrgeDTO>,
) -> ApiResult<bool> {
    ok(state
        .trade
        .kitchen
        .ack_urge(auth.scoped_shop_id(), dto.order_id)
        .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract_exactly() {
        let expected = [
            ("GET", "/adminapi/order/kitchen/board"),
            ("POST", "/adminapi/order/kitchen/transition"),
            ("POST", "/adminapi/order/kitchen/ackUrge"),
        ];
        let docs = implemented_routes();
        assert_eq!(docs.len(), 3);
        for (doc, (method, path)) in docs.iter().zip(expected.iter()) {
            assert_eq!(doc.method, *method);
            assert_eq!(doc.path, *path);
        }
    }
}
