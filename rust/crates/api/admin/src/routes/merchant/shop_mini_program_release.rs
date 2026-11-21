//! 管理端店铺小程序「模式②」代提审 / 代发布路由
//! （merchant::shop_mini_program_release）—— **本平台新增**（论坛帖
//! 487946315732211118 三期）。
//!
//! | 本模块路由（挂载于 `/adminapi`） | 说明 | HTTP |
//! |----------------------------------|------|------|
//! | `GET  /merchant/shopMiniProgram/release/overview` | 发布总览（平台全量 / 商家仅本店） | GET |
//! | `GET  /merchant/shopMiniProgram/release/state?authorizerId=` | 单个授权小程序的版本状态 | GET |
//! | `GET  /merchant/shopMiniProgram/release/logs?authorizerId=` | 动作流水 | GET |
//! | `GET  /merchant/shopMiniProgram/release/pages?authorizerId=` | 已提交代码页面（提审取数） | GET |
//! | `GET  /merchant/shopMiniProgram/release/categories?authorizerId=` | 已通过类目（提审取数） | GET |
//! | `GET  /merchant/shopMiniProgram/release/historyVersions?authorizerId=` | 可回退历史版本 | GET |
//! | `POST /merchant/shopMiniProgram/release/commit` | 代提交代码（模板 + 按店 ext_json） | POST |
//! | `POST /merchant/shopMiniProgram/release/submitAudit` | 提交审核（商家自助 = 提审申请） | POST |
//! | `POST /merchant/shopMiniProgram/release/syncAudit` | 同步最新审核状态 | POST |
//! | `POST /merchant/shopMiniProgram/release/release` | 发布审核通过的版本 | POST |
//! | `POST /merchant/shopMiniProgram/release/withdrawAudit` | 撤回审核 | POST |
//! | `POST /merchant/shopMiniProgram/release/revert` | 线上版本回退 | POST |
//! | `GET  /merchant/shopMiniProgram/release/templates/drafts` | 模板库草稿箱（仅平台） | GET |
//! | `GET  /merchant/shopMiniProgram/release/templates/list` | 模板库列表（平台/商家可读） | GET |
//! | `POST /merchant/shopMiniProgram/release/templates/add` | 草稿加入模板库（仅平台） | POST |
//! | `POST /merchant/shopMiniProgram/release/templates/delete` | 删除模板（仅平台） | POST |
//!
//! > **权限口径**（与模式②授权路由一致）：模板库的草稿/新增/删除是
//! > 平台资产操作，仅平台账号；模板列表只读开放给商家（自助提交代码
//! > 需选模板）。版本动作：平台可对全部店铺代操作，商家只能操作本店
//! > 授权（authorizerId 经授权行归属校验），供应商一律 403。动作流水
//! > 记录发起方（platform / merchant），商家自助提交即「提审申请」。
//! >
//! > 迁移：`rust/migrations/039_shop_mini_program_release.sql` 须先于
//! > 本代码部署执行（迁移先于二进制）。

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;

use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, TenantContext};
use nextrade_services::content::wechat_component::{
    AuditCategory, AuditItemInput, CodeTemplate, HistoryVersion, TemplateDraft,
};
use nextrade_services::merchant::shop_mini_program_auth;
use nextrade_services::merchant::shop_mini_program_release::{
    self, ReleaseLogItem, ReleaseOperator, VersionState,
};

use crate::AdminState;

/// 本域全部 16 条路由（去掉 `/adminapi` 前缀后的路径）。
const ROUTE_TABLE: [(&str, &str, &str); 16] = [
    ("GET", "/merchant/shopMiniProgram/release/overview", "ShopMiniProgramReleaseController.overview (本平台新增)"),
    ("GET", "/merchant/shopMiniProgram/release/state", "ShopMiniProgramReleaseController.state (本平台新增)"),
    ("GET", "/merchant/shopMiniProgram/release/logs", "ShopMiniProgramReleaseController.logs (本平台新增)"),
    ("GET", "/merchant/shopMiniProgram/release/pages", "ShopMiniProgramReleaseController.pages (本平台新增)"),
    ("GET", "/merchant/shopMiniProgram/release/categories", "ShopMiniProgramReleaseController.categories (本平台新增)"),
    ("GET", "/merchant/shopMiniProgram/release/historyVersions", "ShopMiniProgramReleaseController.historyVersions (本平台新增)"),
    ("POST", "/merchant/shopMiniProgram/release/commit", "ShopMiniProgramReleaseController.commit (本平台新增)"),
    ("POST", "/merchant/shopMiniProgram/release/submitAudit", "ShopMiniProgramReleaseController.submitAudit (本平台新增)"),
    ("POST", "/merchant/shopMiniProgram/release/syncAudit", "ShopMiniProgramReleaseController.syncAudit (本平台新增)"),
    ("POST", "/merchant/shopMiniProgram/release/release", "ShopMiniProgramReleaseController.release (本平台新增)"),
    ("POST", "/merchant/shopMiniProgram/release/withdrawAudit", "ShopMiniProgramReleaseController.withdrawAudit (本平台新增)"),
    ("POST", "/merchant/shopMiniProgram/release/revert", "ShopMiniProgramReleaseController.revert (本平台新增)"),
    ("GET", "/merchant/shopMiniProgram/release/templates/drafts", "ShopMiniProgramReleaseController.templateDrafts (本平台新增)"),
    ("GET", "/merchant/shopMiniProgram/release/templates/list", "ShopMiniProgramReleaseController.templateList (本平台新增)"),
    ("POST", "/merchant/shopMiniProgram/release/templates/add", "ShopMiniProgramReleaseController.templateAdd (本平台新增)"),
    ("POST", "/merchant/shopMiniProgram/release/templates/delete", "ShopMiniProgramReleaseController.templateDelete (本平台新增)"),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/merchant/shopMiniProgram/release/overview", get(overview))
        .route("/merchant/shopMiniProgram/release/state", get(state_detail))
        .route("/merchant/shopMiniProgram/release/logs", get(logs))
        .route("/merchant/shopMiniProgram/release/pages", get(pages))
        .route("/merchant/shopMiniProgram/release/categories", get(categories))
        .route(
            "/merchant/shopMiniProgram/release/historyVersions",
            get(history_versions),
        )
        .route("/merchant/shopMiniProgram/release/commit", post(commit))
        .route("/merchant/shopMiniProgram/release/submitAudit", post(submit_audit))
        .route("/merchant/shopMiniProgram/release/syncAudit", post(sync_audit))
        .route("/merchant/shopMiniProgram/release/release", post(release))
        .route(
            "/merchant/shopMiniProgram/release/withdrawAudit",
            post(withdraw_audit),
        )
        .route("/merchant/shopMiniProgram/release/revert", post(revert))
        .route(
            "/merchant/shopMiniProgram/release/templates/drafts",
            get(template_drafts),
        )
        .route(
            "/merchant/shopMiniProgram/release/templates/list",
            get(template_list),
        )
        .route(
            "/merchant/shopMiniProgram/release/templates/add",
            post(template_add),
        )
        .route(
            "/merchant/shopMiniProgram/release/templates/delete",
            post(template_delete),
        )
}

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, java_ref)| {
            let full = format!("/adminapi{path}");
            nextrade_openapi::RouteDoc::new(method, &full, java_ref)
        })
        .collect()
}

/// 仅平台账号（模板库是平台资产，商家/供应商一律 403）。
fn ensure_platform(tenant: &TenantContext) -> Result<(), Error> {
    if tenant.is_platform() {
        Ok(())
    } else {
        Err(Error::Forbidden("仅平台账号可管理代码模板库".into()))
    }
}

/// 校验目标授权记录在当前租户作用域内并构造发起方标识：平台任意；
/// 商家只能本店（授权行归属校验）；供应商/缺失作用域 → 403
/// （fail-closed，与模式②授权路由同口径）。
async fn ensure_release_access(
    state: &AdminState,
    tenant: &TenantContext,
    authorizer_id: i32,
) -> Result<ReleaseOperator, Error> {
    if tenant.is_platform() {
        return Ok(ReleaseOperator::platform());
    }
    let own = tenant
        .shop_scope()
        .ok_or_else(|| Error::Forbidden("缺少店铺租户上下文".into()))?;
    let row = shop_mini_program_auth::find_authorization_by_id(&state.repos.pool, authorizer_id)
        .await?
        .ok_or_else(|| Error::biz(1001, "授权记录不存在"))?;
    if row.shop_id != own {
        return Err(Error::Forbidden("无权操作其他店铺的小程序版本".into()));
    }
    Ok(ReleaseOperator::merchant(own))
}

/// 列表类作用域：平台全量 / 商家仅本店 / 其余 403。
fn list_scope(tenant: &TenantContext) -> Result<Option<i32>, Error> {
    if tenant.is_platform() {
        Ok(None)
    } else if let Some(own) = tenant.shop_scope() {
        Ok(Some(own))
    } else {
        Err(Error::Forbidden("缺少店铺租户上下文".into()))
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct AuthorizerQuery {
    authorizer_id: Option<i32>,
}

fn authorizer_id_of(q: &AuthorizerQuery) -> Result<i32, Error> {
    q.authorizer_id
        .filter(|v| *v > 0)
        .ok_or_else(|| Error::biz(1001, "缺少授权记录 id"))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct AuthorizerBody {
    authorizer_id: Option<i32>,
}

fn authorizer_id_of_body(body: &AuthorizerBody) -> Result<i32, Error> {
    body.authorizer_id
        .filter(|v| *v > 0)
        .ok_or_else(|| Error::biz(1001, "缺少授权记录 id"))
}

async fn overview(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
) -> ApiResult<Vec<VersionState>> {
    let scope = list_scope(&tenant)?;
    ok(shop_mini_program_release::list_overview(&state.repos.pool, scope).await?)
}

async fn state_detail(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Query(q): Query<AuthorizerQuery>,
) -> ApiResult<VersionState> {
    let authorizer_id = authorizer_id_of(&q)?;
    ensure_release_access(&state, &tenant, authorizer_id).await?;
    ok(shop_mini_program_release::get_state(&state.repos.pool, authorizer_id).await?)
}

async fn logs(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Query(q): Query<AuthorizerQuery>,
) -> ApiResult<Vec<ReleaseLogItem>> {
    let authorizer_id = authorizer_id_of(&q)?;
    ensure_release_access(&state, &tenant, authorizer_id).await?;
    ok(shop_mini_program_release::list_logs(&state.repos.pool, authorizer_id, 50).await?)
}

async fn pages(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Query(q): Query<AuthorizerQuery>,
) -> ApiResult<Vec<String>> {
    let authorizer_id = authorizer_id_of(&q)?;
    ensure_release_access(&state, &tenant, authorizer_id).await?;
    ok(shop_mini_program_release::page_list(&state.repos, authorizer_id).await?)
}

async fn categories(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Query(q): Query<AuthorizerQuery>,
) -> ApiResult<Vec<AuditCategory>> {
    let authorizer_id = authorizer_id_of(&q)?;
    ensure_release_access(&state, &tenant, authorizer_id).await?;
    ok(shop_mini_program_release::category_list(&state.repos, authorizer_id).await?)
}

async fn history_versions(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Query(q): Query<AuthorizerQuery>,
) -> ApiResult<Vec<HistoryVersion>> {
    let authorizer_id = authorizer_id_of(&q)?;
    ensure_release_access(&state, &tenant, authorizer_id).await?;
    ok(shop_mini_program_release::history_versions(&state.repos, authorizer_id).await?)
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct CommitBody {
    authorizer_id: Option<i32>,
    template_id: Option<i64>,
    user_version: Option<String>,
    user_desc: Option<String>,
}

async fn commit(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<CommitBody>,
) -> ApiResult<VersionState> {
    let authorizer_id = body
        .authorizer_id
        .filter(|v| *v > 0)
        .ok_or_else(|| Error::biz(1001, "缺少授权记录 id"))?;
    let operator = ensure_release_access(&state, &tenant, authorizer_id).await?;
    ok(shop_mini_program_release::commit_code(
        &state.repos,
        authorizer_id,
        body.template_id.unwrap_or(-1),
        body.user_version.as_deref().unwrap_or_default(),
        body.user_desc.as_deref().unwrap_or_default(),
        &operator,
    )
    .await?)
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct SubmitAuditBody {
    authorizer_id: Option<i32>,
    item_list: Vec<AuditItemInput>,
    feedback_info: Option<String>,
    version_desc: Option<String>,
}

async fn submit_audit(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<SubmitAuditBody>,
) -> ApiResult<VersionState> {
    let authorizer_id = body
        .authorizer_id
        .filter(|v| *v > 0)
        .ok_or_else(|| Error::biz(1001, "缺少授权记录 id"))?;
    let operator = ensure_release_access(&state, &tenant, authorizer_id).await?;
    ok(shop_mini_program_release::submit_audit(
        &state.repos,
        authorizer_id,
        &body.item_list,
        body.feedback_info.as_deref(),
        body.version_desc.as_deref(),
        &operator,
    )
    .await?)
}

async fn sync_audit(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<AuthorizerBody>,
) -> ApiResult<VersionState> {
    let authorizer_id = authorizer_id_of_body(&body)?;
    let operator = ensure_release_access(&state, &tenant, authorizer_id).await?;
    ok(shop_mini_program_release::sync_audit_status(&state.repos, authorizer_id, &operator)
        .await?)
}

async fn release(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<AuthorizerBody>,
) -> ApiResult<VersionState> {
    let authorizer_id = authorizer_id_of_body(&body)?;
    let operator = ensure_release_access(&state, &tenant, authorizer_id).await?;
    ok(shop_mini_program_release::release(&state.repos, authorizer_id, &operator).await?)
}

async fn withdraw_audit(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<AuthorizerBody>,
) -> ApiResult<VersionState> {
    let authorizer_id = authorizer_id_of_body(&body)?;
    let operator = ensure_release_access(&state, &tenant, authorizer_id).await?;
    ok(shop_mini_program_release::withdraw_audit(&state.repos, authorizer_id, &operator)
        .await?)
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct RevertBody {
    authorizer_id: Option<i32>,
    app_version: Option<i64>,
}

async fn revert(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<RevertBody>,
) -> ApiResult<VersionState> {
    let authorizer_id = body
        .authorizer_id
        .filter(|v| *v > 0)
        .ok_or_else(|| Error::biz(1001, "缺少授权记录 id"))?;
    let operator = ensure_release_access(&state, &tenant, authorizer_id).await?;
    ok(shop_mini_program_release::revert(
        &state.repos,
        authorizer_id,
        body.app_version,
        &operator,
    )
    .await?)
}

async fn template_drafts(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
) -> ApiResult<Vec<TemplateDraft>> {
    ensure_platform(&tenant)?;
    ok(shop_mini_program_release::list_template_drafts(&state.repos).await?)
}

async fn template_list(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
) -> ApiResult<Vec<CodeTemplate>> {
    // 模板列表只读开放给商家：商家自助代提交代码时需要选模板（模板的
    // 草稿/新增/删除仍仅平台）。供应商经 list_scope 校验被拒。
    list_scope(&tenant)?;
    ok(shop_mini_program_release::list_code_templates(&state.repos).await?)
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct TemplateAddBody {
    draft_id: Option<i64>,
    template_type: Option<i64>,
}

async fn template_add(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<TemplateAddBody>,
) -> ApiResult<bool> {
    ensure_platform(&tenant)?;
    shop_mini_program_release::add_draft_to_template(
        &state.repos,
        body.draft_id.unwrap_or(0),
        body.template_type.unwrap_or(0),
    )
    .await?;
    ok(true)
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct TemplateDeleteBody {
    template_id: Option<i64>,
}

async fn template_delete(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<TemplateDeleteBody>,
) -> ApiResult<bool> {
    ensure_platform(&tenant)?;
    shop_mini_program_release::delete_code_template(
        &state.repos,
        body.template_id.unwrap_or(-1),
    )
    .await?;
    ok(true)
}
