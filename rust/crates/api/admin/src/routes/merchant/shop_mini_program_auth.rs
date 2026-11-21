//! 管理端店铺小程序「模式②」第三方平台授权路由（merchant::shop_mini_program_auth）
//! —— **本平台新增**（论坛帖 487946315732211118 二期）。
//!
//! | 本模块路由（挂载于 `/adminapi`） | 说明 | HTTP |
//! |----------------------------------|------|------|
//! | `GET  /merchant/shopMiniProgram/thirdparty/config` | 第三方平台配置视图（仅平台） | GET |
//! | `POST /merchant/shopMiniProgram/thirdparty/config` | 保存第三方平台配置（仅平台） | POST |
//! | `POST /merchant/shopMiniProgram/thirdparty/createAuthLink` | 发起授权（预授权码 + 授权链接） | POST |
//! | `GET  /merchant/shopMiniProgram/thirdparty/authStatus?state=` | 轮询待授权会话结果 | GET |
//! | `GET  /merchant/shopMiniProgram/thirdparty/authList?keyword=` | 授权列表（平台全量 / 商家仅本店） | GET |
//! | `POST /merchant/shopMiniProgram/thirdparty/syncInfo` | 同步授权方基本信息 | POST |
//! | `POST /merchant/shopMiniProgram/thirdparty/unbind` | 本地解绑（微信侧取消在 mp 后台） | POST |
//!
//! > **权限口径**：平台配置仅平台账号；发起授权/同步/解绑：平台可对
//! > 全部店铺，商家只能本店（shopId 由 JWT 作用域强制）；供应商一律
//! > 403。refresh token 只入不出：列表只回掩码，密文永不下发。
//! >
//! > 迁移：`rust/migrations/028_shop_mini_program_thirdparty.sql` 须先于
//! > 本代码部署执行（迁移先于二进制）。

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;

use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, TenantContext};
use nextrade_services::merchant::shop_mini_program_auth::{
    self, AuthLink, AuthorizationItem, ComponentConfigPatch, ComponentConfigView, PendingAuth,
};

use crate::AdminState;

/// 本域全部 7 条路由（去掉 `/adminapi` 前缀后的路径）。
const ROUTE_TABLE: [(&str, &str, &str); 7] = [
    ("GET", "/merchant/shopMiniProgram/thirdparty/config", "ShopMiniProgramAuthController.config (本平台新增)"),
    ("POST", "/merchant/shopMiniProgram/thirdparty/config", "ShopMiniProgramAuthController.saveConfig (本平台新增)"),
    ("POST", "/merchant/shopMiniProgram/thirdparty/createAuthLink", "ShopMiniProgramAuthController.createAuthLink (本平台新增)"),
    ("GET", "/merchant/shopMiniProgram/thirdparty/authStatus", "ShopMiniProgramAuthController.authStatus (本平台新增)"),
    ("GET", "/merchant/shopMiniProgram/thirdparty/authList", "ShopMiniProgramAuthController.authList (本平台新增)"),
    ("POST", "/merchant/shopMiniProgram/thirdparty/syncInfo", "ShopMiniProgramAuthController.syncInfo (本平台新增)"),
    ("POST", "/merchant/shopMiniProgram/thirdparty/unbind", "ShopMiniProgramAuthController.unbind (本平台新增)"),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/merchant/shopMiniProgram/thirdparty/config", get(config))
        .route("/merchant/shopMiniProgram/thirdparty/config", post(save_config))
        .route("/merchant/shopMiniProgram/thirdparty/createAuthLink", post(create_auth_link))
        .route("/merchant/shopMiniProgram/thirdparty/authStatus", get(auth_status))
        .route("/merchant/shopMiniProgram/thirdparty/authList", get(auth_list))
        .route("/merchant/shopMiniProgram/thirdparty/syncInfo", post(sync_info))
        .route("/merchant/shopMiniProgram/thirdparty/unbind", post(unbind))
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

/// 仅平台账号（第三方平台配置是平台级凭据，商家/供应商一律 403）。
fn ensure_platform(tenant: &TenantContext) -> Result<(), Error> {
    if tenant.is_platform() {
        Ok(())
    } else {
        Err(Error::Forbidden("仅平台账号可管理第三方平台配置".into()))
    }
}

/// 写操作的目标店铺：平台用请求值；商家强制本店（请求值不符 → 403）；
/// 供应商/缺失作用域 → 403（fail-closed，与模式①路由同口径）。
fn target_shop(tenant: &TenantContext, requested: Option<i32>) -> Result<i32, Error> {
    if tenant.is_platform() {
        return requested
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::biz(1001, "请选择要授权的店铺"));
    }
    let own = tenant
        .shop_scope()
        .ok_or_else(|| Error::Forbidden("缺少店铺租户上下文".into()))?;
    if let Some(req) = requested.filter(|id| *id > 0) {
        if req != own {
            return Err(Error::Forbidden("无权操作其他店铺的小程序授权".into()));
        }
    }
    Ok(own)
}

/// 校验某条授权记录在当前租户作用域内（商家只能动本店记录）。
async fn ensure_auth_access(
    state: &AdminState,
    tenant: &TenantContext,
    id: i32,
) -> Result<(), Error> {
    if tenant.is_platform() {
        return Ok(());
    }
    let own = tenant
        .shop_scope()
        .ok_or_else(|| Error::Forbidden("缺少店铺租户上下文".into()))?;
    let row = shop_mini_program_auth::find_authorization_by_id(&state.repos.pool, id)
        .await?
        .ok_or_else(|| Error::biz(1001, "授权记录不存在"))?;
    if row.shop_id != own {
        return Err(Error::Forbidden("无权操作其他店铺的小程序授权".into()));
    }
    Ok(())
}

async fn config(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
) -> ApiResult<ComponentConfigView> {
    ensure_platform(&tenant)?;
    ok(shop_mini_program_auth::component_config_view(&state.repos).await?)
}

async fn save_config(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<ComponentConfigPatch>,
) -> ApiResult<bool> {
    ensure_platform(&tenant)?;
    shop_mini_program_auth::save_component_config(&state.repos, &body).await?;
    ok(true)
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct CreateAuthLinkBody {
    shop_id: Option<i32>,
}

async fn create_auth_link(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<CreateAuthLinkBody>,
) -> ApiResult<AuthLink> {
    let shop_id = target_shop(&tenant, body.shop_id)?;
    ok(shop_mini_program_auth::create_auth_link(&state.repos, shop_id).await?)
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct AuthStatusQuery {
    state: Option<String>,
}

async fn auth_status(
    State(_state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Query(q): Query<AuthStatusQuery>,
) -> ApiResult<Option<PendingAuth>> {
    let state = q
        .state
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::biz(1001, "缺少 state"))?;
    let pending = shop_mini_program_auth::pending_auth_status(state).await;
    if let Some(p) = &pending {
        // 商家只能轮询本店会话（state 本身不可猜，仍按作用域收口）
        if let Some(own) = tenant.shop_scope() {
            if p.shop_id != own {
                return Err(
                    Error::Forbidden("无权查看其他店铺的授权会话".into()).into(),
                );
            }
        } else if !tenant.is_platform() {
            return Err(Error::Forbidden("缺少店铺租户上下文".into()).into());
        }
    }
    ok(pending)
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ListQuery {
    keyword: Option<String>,
}

async fn auth_list(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Vec<AuthorizationItem>> {
    let scope = if tenant.is_platform() {
        None
    } else if let Some(own) = tenant.shop_scope() {
        Some(own)
    } else {
        return Err(Error::Forbidden("缺少店铺租户上下文".into()).into());
    };
    ok(shop_mini_program_auth::list_authorizations(
        &state.repos.pool,
        q.keyword.as_deref(),
        scope,
    )
    .await?)
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct IdBody {
    id: Option<i32>,
}

async fn sync_info(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<IdBody>,
) -> ApiResult<bool> {
    let id = body.id.filter(|v| *v > 0).ok_or_else(|| Error::biz(1001, "缺少授权记录 id"))?;
    ensure_auth_access(&state, &tenant, id).await?;
    shop_mini_program_auth::sync_authorizer_info(&state.repos, id).await?;
    ok(true)
}

async fn unbind(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<IdBody>,
) -> ApiResult<bool> {
    let id = body.id.filter(|v| *v > 0).ok_or_else(|| Error::biz(1001, "缺少授权记录 id"))?;
    ensure_auth_access(&state, &tenant, id).await?;
    shop_mini_program_auth::detach_authorization(&state.repos.pool, id).await?;
    ok(true)
}
