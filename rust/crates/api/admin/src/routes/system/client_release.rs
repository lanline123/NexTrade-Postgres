//! 管理端客户端发版登记路由（system::client_release，我方新增，无原版对应）。
//!
//! admin 自助热更新 P4（设计见 hot-update-design/REPORT.md §5.4）：小程序 /
//! App 不能热更，只能提醒；本域是提醒的「已发布登记」一侧事实源。
//!
//! | 路由 | 说明 |
//! |------|------|
//! | `GET  /adminapi/setting/clientRelease/overview` | 两渠道登记概览（`?targetVersion=` 透传最新产品版本算覆盖） |
//! | `POST /adminapi/setting/clientRelease/register` | 登记一条已发布记录 |
//! | `POST /adminapi/setting/clientRelease/remove` | 删除一条登记（纠错） |
//!
//! 权限口径：整域挂 [`platform_only`]——与 systemUpdate 同口径，仅平台管理员
//! 可见可达（店铺/供应商账号无入口）。提醒的存否是派生态：前端组合系统更新
//! 状态的清单标志（`requiresMiniProgramUpdate` / `requiresAppUpdate`，P1 口径）
//! 与本域概览的 `covered` 即得，不落提醒行。

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;

use nextrade_core::models::system::client_release::{
    ClientReleaseOverviewVO, ClientReleaseVO, RegisterClientReleaseParam,
    RemoveClientReleaseParam,
};
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::support::tenant::{platform_only, require_platform};

use crate::AdminState;

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/setting/clientRelease/overview", get(overview))
        .route("/setting/clientRelease/register", post(register))
        .route("/setting/clientRelease/remove", post(remove))
        .layer(axum::middleware::from_fn(platform_only))
}

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计；我方新增，无 Java 对照）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    [
        ("GET", "/adminapi/setting/clientRelease/overview", "我方新增·客户端发版登记概览"),
        ("POST", "/adminapi/setting/clientRelease/register", "我方新增·登记客户端已发布版本"),
        ("POST", "/adminapi/setting/clientRelease/remove", "我方新增·删除客户端发版登记"),
    ]
    .iter()
    .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, p, j))
    .collect()
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct OverviewQuery {
    target_version: Option<String>,
}

/// `GET /adminapi/setting/clientRelease/overview`：两渠道登记概览。
async fn overview(
    State(state): State<AdminState>,
    Query(q): Query<OverviewQuery>,
) -> ApiResult<ClientReleaseOverviewVO> {
    ok(state
        .system
        .client_release
        .overview(q.target_version.as_deref())
        .await?)
}

/// `POST /adminapi/setting/clientRelease/register`：登记已发布。
async fn register(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(param): Json<RegisterClientReleaseParam>,
) -> ApiResult<ClientReleaseVO> {
    ok(state
        .system
        .client_release
        .register(&param, auth.id, &auth.username)
        .await?)
}

/// `POST /adminapi/setting/clientRelease/remove`：删除登记（纠错）。
async fn remove(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(param): Json<RemoveClientReleaseParam>,
) -> ApiResult<String> {
    state
        .system
        .client_release
        .remove(param.id, &auth.username)
        .await?;
    ok("删除成功".to_string())
}

#[cfg(test)]
mod tenant_tests {
    use super::*;

    #[test]
    fn shop_account_is_rejected_by_platform_guard() {
        let admin = AuthUser::new(1, "a".into(), None, Some("admin".into()), Some(0));
        assert!(require_platform(&admin).is_ok());
        let shop = AuthUser::new(2, "s".into(), None, Some("shop".into()), Some(7));
        assert!(require_platform(&shop).is_err());
    }
}
