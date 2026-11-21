//! 站点搬家 —— 管理端路由（site_clone::clone）。
//!
//! 设计：`~/workspace/site-clone-design/REPORT.md` 第二/五/六节。
//!
//! | method | path | 说明 |
//! |--------|------|------|
//! | `GET`  | `/siteClone/keys/list` | 搬家密钥分页列表 |
//! | `POST` | `/siteClone/keys/create` | 生成密钥（secret/口令只返回一次） |
//! | `POST` | `/siteClone/keys/update` | 编辑密钥（名称/权限/源店铺/启停） |
//! | `POST` | `/siteClone/keys/revoke` | 吊销密钥（即刻失效） |
//! | `POST` | `/siteClone/keys/resetSecret` | 重置 secret（旧的立即作废） |
//! | `GET`  | `/siteClone/callLogs/list` | 拉取审计分页 |
//! | `POST` | `/siteClone/import/preview` | 贴口令拉清单预览（不落数据） |
//! | `POST` | `/siteClone/import/execute` | 执行导入（快照→对齐→建品→覆盖） |
//! | `GET`  | `/siteClone/jobs/list` | 导入任务分页 |
//! | `GET`  | `/siteClone/jobs/detail?id=` | 任务详情（计数/失败明细） |
//! | `POST` | `/siteClone/jobs/undo` | 撤销本次导入（删新建商品） |
//! | `GET`  | `/siteClone/snapshots/list` | 导入前快照分页 |
//! | `POST` | `/siteClone/snapshots/rollback` | 一键回滚到导入前设置 |
//! | `POST` | `/siteClone/offline/export` | 导出离线包（P3：逐文件 sha + HMAC 签名的 JSON 包） |

use axum::{
    extract::{Extension, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;

use nextrade_core::models::site_clone::clone::{
    CloneCallLogListQuery, CloneExecuteDTO, CloneIdDTO, CloneJobListQuery, CloneKeyCreateDTO,
    CloneKeyListQuery, CloneKeySecretVO, CloneKeyUpdateDTO, CloneKeyVO, ClonePreviewDTO,
    ClonePreviewVO, SiteCloneCallLog, SiteCloneImportJob, SiteCloneSnapshot,
};
use nextrade_core::types::PageResult;
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::support::tenant::platform_only;
use crate::AdminState;

/// 本域路由（去掉 `/adminapi` 前缀）。
const ROUTE_TABLE: [(&str, &str, &str); 14] = [
    ("GET", "/siteClone/keys/list", "SiteCloneController.keyList"),
    ("POST", "/siteClone/keys/create", "SiteCloneController.keyCreate"),
    ("POST", "/siteClone/keys/update", "SiteCloneController.keyUpdate"),
    ("POST", "/siteClone/keys/revoke", "SiteCloneController.keyRevoke"),
    ("POST", "/siteClone/keys/resetSecret", "SiteCloneController.keyResetSecret"),
    ("GET", "/siteClone/callLogs/list", "SiteCloneController.callLogList"),
    ("POST", "/siteClone/import/preview", "SiteCloneController.importPreview"),
    ("POST", "/siteClone/import/execute", "SiteCloneController.importExecute"),
    ("GET", "/siteClone/jobs/list", "SiteCloneController.jobList"),
    ("GET", "/siteClone/jobs/detail", "SiteCloneController.jobDetail"),
    ("POST", "/siteClone/jobs/undo", "SiteCloneController.jobUndo"),
    ("GET", "/siteClone/snapshots/list", "SiteCloneController.snapshotList"),
    ("POST", "/siteClone/snapshots/rollback", "SiteCloneController.snapshotRollback"),
    ("POST", "/siteClone/offline/export", "SiteCloneController.offlineExport"),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/siteClone/keys/list", get(key_list))
        .route("/siteClone/keys/create", post(key_create))
        .route("/siteClone/keys/update", post(key_update))
        .route("/siteClone/keys/revoke", post(key_revoke))
        .route("/siteClone/keys/resetSecret", post(key_reset_secret))
        .route("/siteClone/callLogs/list", get(call_log_list))
        .route("/siteClone/import/preview", post(import_preview))
        .route("/siteClone/import/execute", post(import_execute))
        .route("/siteClone/jobs/list", get(job_list))
        .route("/siteClone/jobs/detail", get(job_detail))
        .route("/siteClone/jobs/undo", post(job_undo))
        .route("/siteClone/snapshots/list", get(snapshot_list))
        .route("/siteClone/snapshots/rollback", post(snapshot_rollback))
        .route("/siteClone/offline/export", post(offline_export))
        .layer(axum::middleware::from_fn(platform_only))
}

/// 本域已实现路由（完整契约路径，用于覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/adminapi{path}"), handler))
        .collect()
}

async fn key_list(
    State(state): State<AdminState>,
    Query(q): Query<CloneKeyListQuery>,
) -> ApiResult<PageResult<CloneKeyVO>> {
    ok(state.site_clone.clone.list_keys(&q).await?)
}

async fn key_create(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<CloneKeyCreateDTO>,
) -> ApiResult<CloneKeySecretVO> {
    ok(state.site_clone.clone.create_key(&dto, auth.id, &auth.username).await?)
}

async fn key_update(
    State(state): State<AdminState>,
    Json(dto): Json<CloneKeyUpdateDTO>,
) -> ApiResult<()> {
    state.site_clone.clone.update_key(&dto).await?;
    ok(())
}

async fn key_revoke(
    State(state): State<AdminState>,
    Json(dto): Json<CloneIdDTO>,
) -> ApiResult<()> {
    let id = dto.target().ok_or_else(|| {
        nextrade_security::web::AppError(nextrade_core::Error::biz(400, "缺少密钥 id"))
    })?;
    state.site_clone.clone.revoke_key(id).await?;
    ok(())
}

async fn key_reset_secret(
    State(state): State<AdminState>,
    Json(dto): Json<CloneIdDTO>,
) -> ApiResult<CloneKeySecretVO> {
    ok(state.site_clone.clone.reset_secret(&dto).await?)
}

async fn call_log_list(
    State(state): State<AdminState>,
    Query(q): Query<CloneCallLogListQuery>,
) -> ApiResult<PageResult<SiteCloneCallLog>> {
    ok(state.site_clone.clone.list_call_logs(&q).await?)
}

async fn import_preview(
    State(state): State<AdminState>,
    Json(dto): Json<ClonePreviewDTO>,
) -> ApiResult<ClonePreviewVO> {
    ok(state.site_clone.clone.preview(&dto).await?)
}

async fn import_execute(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<CloneExecuteDTO>,
) -> ApiResult<SiteCloneImportJob> {
    let job = state
        .site_clone
        .clone
        .execute(&dto, auth.id, &auth.username)
        .await?;
    // 导入商品可保留源站上架状态（keep_product_status），新品不在任何单品
    // 入队路径上，图搜索引随之全量对账（全量任务按 job_key 去重，
    // 图搜未启用时入队函数自跳过）。
    crate::routes::system::image_search::enqueue_full_sync(&state).await;
    ok(job)
}

async fn job_list(
    State(state): State<AdminState>,
    Query(q): Query<CloneJobListQuery>,
) -> ApiResult<PageResult<SiteCloneImportJob>> {
    ok(state.site_clone.clone.list_jobs(&q).await?)
}

async fn job_detail(
    State(state): State<AdminState>,
    Query(q): Query<CloneIdDTO>,
) -> ApiResult<SiteCloneImportJob> {
    let id = q.target().ok_or_else(|| {
        nextrade_security::web::AppError(nextrade_core::Error::biz(400, "缺少任务 id"))
    })?;
    ok(state.site_clone.clone.job_detail(id).await?)
}

async fn job_undo(
    State(state): State<AdminState>,
    Json(dto): Json<CloneIdDTO>,
) -> ApiResult<Value> {
    let id = dto.target().ok_or_else(|| {
        nextrade_security::web::AppError(nextrade_core::Error::biz(400, "缺少任务 id"))
    })?;
    ok(state.site_clone.clone.undo_job(id).await?)
}

async fn snapshot_list(
    State(state): State<AdminState>,
    Query(q): Query<CloneJobListQuery>,
) -> ApiResult<PageResult<SiteCloneSnapshot>> {
    ok(state.site_clone.clone.list_snapshots(&q).await?)
}

async fn snapshot_rollback(
    State(state): State<AdminState>,
    Json(dto): Json<CloneIdDTO>,
) -> ApiResult<()> {
    let id = dto.target().ok_or_else(|| {
        nextrade_security::web::AppError(nextrade_core::Error::biz(400, "缺少快照 id"))
    })?;
    state.site_clone.clone.rollback_snapshot(id).await?;
    ok(())
}

/// 导出离线包（P3 兜底通道）：按该密钥的授权域组装，前端落盘为
/// .json 文件；导入端在另一站粘贴口令 + 上传此包验签后导入。
async fn offline_export(
    State(state): State<AdminState>,
    Json(dto): Json<CloneIdDTO>,
) -> ApiResult<Value> {
    let id = dto.target().ok_or_else(|| {
        nextrade_security::web::AppError(nextrade_core::Error::biz(400, "缺少密钥 id"))
    })?;
    let origin = dto.origin.as_deref().unwrap_or("");
    ok(state.site_clone.clone.export_offline_package(id, origin).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract_exactly() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), ROUTE_TABLE.len());
        for (doc, (method, path, _)) in docs.iter().zip(ROUTE_TABLE.iter()) {
            assert_eq!(doc.method, *method);
            assert_eq!(doc.path, format!("/adminapi{path}"));
        }
    }
}
