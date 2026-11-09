//! 商城前台会员域路由（等级/积分/签到）。
//! 对应原 Java `controller.user.UserSignController`、`PointsLogController` 等。
//!
//! TODO(域实现者)：对照 Java 源补全签到（每日一次、连续奖励）、
//! 我的积分流水接口。

use axum::{
    extract::{Query, State},
    routing::get,
    Extension, Router,
};
use serde::Deserialize;
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::PublicState;

/// 公开路由：暂无（TODO 按 Java 源补全）。
pub fn public_routes() -> Router<PublicState> {
    Router::new()
}

/// 需登录路由：我的积分流水。
pub fn authorized_routes() -> Router<PublicState> {
    Router::new().route("/user/pointsLog/list", get(my_points_logs))
}

/// 本域已实现路由（用于覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    [("GET", "/api/user/pointsLog/list", "PointsLogController.list")]
        .iter()
        .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, p, j))
        .collect()
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct MyPointsQuery {
    page: Option<u64>,
    page_size: Option<u64>,
}

/// `GET /api/user/pointsLog/list`：我的积分流水。
async fn my_points_logs(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<MyPointsQuery>,
) -> ApiResult<PageResult<serde_json::Value>> {
    let query = nextrade_core::models::UserPointsLogQuery {
        user_id: Some(auth.id),
        page: q.page,
        page_size: q.page_size,
    };
    let page = state.member.member_lists.points_logs(&query, None).await?;
    let list: Vec<serde_json::Value> = page
        .list
        .iter()
        .map(|l| {
            // 库列：时间字段为 `change_time`（非 add_time），且无 balance 列。
            serde_json::json!({
                "logId": l.log_id,
                "points": l.points,
                "changeTime": l.change_time,
                "changeDesc": l.change_desc,
                "changeType": l.change_type,
            })
        })
        .collect();
    let pq = nextrade_core::types::PageQuery {
        page: query.page.unwrap_or(1),
        page_size: query.page_size.unwrap_or(20),
    };
    ok(PageResult::new(page.total, list, &pq))
}