//! 管理端店铺小程序绑定路由（merchant::shop_mini_program）—— **本平台新增**
//! （论坛帖 487946315732211118「店铺能不能绑定独立小程序」）。
//!
//! | 本模块路由（挂载于 `/adminapi`） | 说明 | HTTP |
//! |----------------------------------|------|------|
//! | `GET  /merchant/shopMiniProgram/list?keyword=` | 绑定列表（平台全量 / 商家仅本店） | GET |
//! | `GET  /merchant/shopMiniProgram/myBinding` | 本店绑定详情（商家自查，无绑定回 null） | GET |
//! | `POST /merchant/shopMiniProgram/bind` | 绑定/换绑（凭据探活后加密落库） | POST |
//! | `POST /merchant/shopMiniProgram/setStatus` | 启停（停用即按未登记拒绝） | POST |
//! | `POST /merchant/shopMiniProgram/unbind` | 解绑（软删） | POST |
//!
//! > **权限口径**：平台账号可管理全部店铺；商家账号只能查看/操作本店
//! > （shopId 由 JWT 作用域强制，请求体里的 shopId 与本店不符直接 403）；
//! > 供应商账号无店铺维度，一律 403。AppSecret 只入不出：列表/详情只回
//! > 掩码，密文永不下发。
//! >
//! > 迁移：`rust/deploy/pg/2026-11-21_shop_mini_program.sql` 须先于本代码
//! > 部署执行（迁移先于二进制）。

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;

use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, AppError, TenantContext};
use nextrade_services::merchant::shop_mini_program::{
    ShopMiniProgramBind, ShopMiniProgramItem, ShopMiniProgramStatus,
};

use crate::AdminState;

/// 本域全部 5 条路由（去掉 `/adminapi` 前缀后的路径）。
const ROUTE_TABLE: [(&str, &str, &str); 5] = [
    ("GET", "/merchant/shopMiniProgram/list", "ShopMiniProgramController.list (本平台新增)"),
    ("GET", "/merchant/shopMiniProgram/myBinding", "ShopMiniProgramController.myBinding (本平台新增)"),
    ("POST", "/merchant/shopMiniProgram/bind", "ShopMiniProgramController.bind (本平台新增)"),
    ("POST", "/merchant/shopMiniProgram/setStatus", "ShopMiniProgramController.setStatus (本平台新增)"),
    ("POST", "/merchant/shopMiniProgram/unbind", "ShopMiniProgramController.unbind (本平台新增)"),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/merchant/shopMiniProgram/list", get(list))
        .route("/merchant/shopMiniProgram/myBinding", get(my_binding))
        .route("/merchant/shopMiniProgram/bind", post(bind))
        .route("/merchant/shopMiniProgram/setStatus", post(set_status))
        .route("/merchant/shopMiniProgram/unbind", post(unbind))
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

/// 解析写操作的目标店铺：平台用请求值；商家强制本店（请求值不符 → 403）；
/// 供应商/缺失作用域 → 403（fail-closed，绝不退化成平台不限）。
fn target_shop(tenant: &TenantContext, requested: Option<i32>) -> Result<i32, Error> {
    if tenant.is_platform() {
        return requested
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::biz(1001, "请选择要绑定的店铺"));
    }
    let own = tenant
        .shop_scope()
        .ok_or_else(|| Error::Forbidden("缺少店铺租户上下文".into()))?;
    if let Some(req) = requested.filter(|id| *id > 0) {
        if req != own {
            return Err(Error::Forbidden("无权操作其他店铺的小程序绑定".into()));
        }
    }
    Ok(own)
}

/// 校验某条绑定记录在当前租户作用域内（商家只能动本店记录）。
async fn ensure_binding_access(
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
    let item = state
        .merchant
        .shop_mini_program
        .detail_for_shop(own)
        .await?
        .ok_or_else(|| Error::biz(1001, "绑定记录不存在"))?;
    if item.id != id {
        return Err(Error::Forbidden("无权操作其他店铺的小程序绑定".into()));
    }
    Ok(())
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ListQuery {
    keyword: Option<String>,
}

async fn list(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Vec<ShopMiniProgramItem>> {
    let mut rows = state
        .merchant
        .shop_mini_program
        .list(q.keyword.as_deref())
        .await?;
    // 商家账号只看本店（服务层出全量，此处按作用域收口；平台不过滤）
    if let Some(own) = tenant.shop_scope() {
        rows.retain(|r| r.shop_id == own);
    } else if !tenant.is_platform() {
        return Err(AppError(Error::Forbidden("缺少店铺租户上下文".into())));
    }
    ok(rows)
}

async fn my_binding(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
) -> ApiResult<Option<ShopMiniProgramItem>> {
    let own = tenant
        .shop_scope()
        .ok_or_else(|| Error::Forbidden("仅商家账号可查看本店绑定".into()))?;
    ok(state
        .merchant
        .shop_mini_program
        .detail_for_shop(own)
        .await?)
}

async fn bind(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<ShopMiniProgramBind>,
) -> ApiResult<i32> {
    let shop_id = target_shop(&tenant, body.shop_id)?;
    ok(state
        .merchant
        .shop_mini_program
        .bind(shop_id, &body)
        .await?)
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct IdBody {
    id: Option<i32>,
}

async fn set_status(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<ShopMiniProgramStatus>,
) -> ApiResult<bool> {
    let id = body.id.filter(|v| *v > 0).ok_or_else(|| Error::biz(1001, "缺少绑定记录 id"))?;
    let status = body.status.ok_or_else(|| Error::biz(1001, "缺少状态值"))?;
    ensure_binding_access(&state, &tenant, id).await?;
    state
        .merchant
        .shop_mini_program
        .set_status(id, status)
        .await?;
    ok(true)
}

async fn unbind(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<IdBody>,
) -> ApiResult<bool> {
    let id = body.id.filter(|v| *v > 0).ok_or_else(|| Error::biz(1001, "缺少绑定记录 id"))?;
    ensure_binding_access(&state, &tenant, id).await?;
    state.merchant.shop_mini_program.unbind(id).await?;
    ok(true)
}
