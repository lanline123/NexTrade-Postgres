//! 管理端系统更新路由（system::system_update，我方新增，无原版对应）。
//!
//! admin 自助热更新 P1/P2（设计见 hot-update-design/REPORT.md）：
//!
//! | 路由 | 说明 | 期 |
//! |------|------|----|
//! | `GET  /adminapi/setting/systemUpdate/status` | 版本感知总览（`?refresh=1` 强制重拉清单） | P1 |
//! | `POST /adminapi/setting/systemUpdate/refresh` | 手动检查更新（重拉清单后返回总览） | P1 |
//! | `GET  /adminapi/setting/systemUpdate/jobs` | 最近更新任务（审计/进度） | P2 |
//! | `GET  /adminapi/setting/systemUpdate/job/{jobId}` | 单个任务状态（前端轮询） | P2 |
//! | `POST /adminapi/setting/systemUpdate/createJob` | 发起静态三面更新（密码再认证在服务层） | P2 |
//!
//! 权限口径：整域挂 [`platform_only`]——仅平台管理员可见可达（设计 §5.3 的
//! `systemUpdate` 专属权限点在本仓权限矩阵生成表覆盖原版契约端点，我方新增
//! 端点沿用 siteClone 等既有先例，以平台守卫实现同等隔离，菜单仅对平台
//! 角色展示）。密码再认证、基线门、单 job 等业务校验在服务层。

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;

use nextrade_core::models::system::system_update::{
    CreateUpdateJobParam, SystemUpdateStatusVO, UpdateJobVO,
};
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::support::tenant::{platform_only, require_platform};

use crate::AdminState;

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/setting/systemUpdate/status", get(status))
        .route("/setting/systemUpdate/refresh", post(refresh))
        .route("/setting/systemUpdate/jobs", get(jobs))
        .route("/setting/systemUpdate/job/{jobId}", get(job_detail))
        .route("/setting/systemUpdate/createJob", post(create_job))
        .layer(axum::middleware::from_fn(platform_only))
}

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计；我方新增，无 Java 对照）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    [
        ("GET", "/adminapi/setting/systemUpdate/status", "我方新增·系统更新状态"),
        ("POST", "/adminapi/setting/systemUpdate/refresh", "我方新增·检查更新"),
        ("GET", "/adminapi/setting/systemUpdate/jobs", "我方新增·更新任务列表"),
        ("GET", "/adminapi/setting/systemUpdate/job/{jobId}", "我方新增·更新任务详情"),
        ("POST", "/adminapi/setting/systemUpdate/createJob", "我方新增·发起更新"),
    ]
    .iter()
    .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, p, j))
    .collect()
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct StatusQuery {
    refresh: Option<String>,
}

/// `GET /adminapi/setting/systemUpdate/status`：版本感知总览。
async fn status(
    State(state): State<AdminState>,
    Query(q): Query<StatusQuery>,
) -> ApiResult<SystemUpdateStatusVO> {
    let refresh = matches!(q.refresh.as_deref(), Some("1") | Some("true"));
    ok(state.system.system_update.status(refresh).await?)
}

/// `POST /adminapi/setting/systemUpdate/refresh`：手动检查更新。
async fn refresh(State(state): State<AdminState>) -> ApiResult<SystemUpdateStatusVO> {
    ok(state.system.system_update.status(true).await?)
}

/// `GET /adminapi/setting/systemUpdate/jobs`：最近 20 条更新任务。
async fn jobs(State(state): State<AdminState>) -> ApiResult<Vec<UpdateJobVO>> {
    ok(state.system.system_update.list_jobs(20).await?)
}

/// `GET /adminapi/setting/systemUpdate/job/{jobId}`：单个任务状态。
async fn job_detail(
    State(state): State<AdminState>,
    Path(job_id): Path<String>,
) -> ApiResult<UpdateJobVO> {
    ok(state.system.system_update.get_job(&job_id).await?)
}

/// `POST /adminapi/setting/systemUpdate/createJob`：发起更新（二次确认）。
async fn create_job(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(param): Json<CreateUpdateJobParam>,
) -> ApiResult<UpdateJobVO> {
    ok(state
        .system
        .system_update
        .create_job(&param, auth.id, &auth.username)
        .await?)
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
