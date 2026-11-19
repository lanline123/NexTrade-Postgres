//! 管理端 广告投流路由（merchant::ad_campaign，8 条，本平台新增契约）。
//!
//! 商家自助投流 + 平台审核/置顶/下架，共用 `ad_campaign` 表与状态机：
//! - 商家（店铺账号）：新建投放（强制本店，`TenantContext::require_shop()`）、
//!   我的投放、暂停、恢复；付款在服务层从店铺余额扣款。
//! - 平台账号：审核列表、审核（通过/驳回）、置顶（自建 pin 单）、强制下架。
//!
//! | 路由（挂载于 `/adminapi`） | 权限 |
//! |---|---|
//! | `POST /merchant/adCampaign/create` | 店铺账号（本店） |
//! | `GET  /merchant/adCampaign/myList` | 店铺账号（本店） |
//! | `POST /merchant/adCampaign/pause` | 店铺账号（本店） |
//! | `POST /merchant/adCampaign/resume` | 店铺账号（本店） |
//! | `GET  /merchant/adCampaign/auditList` | 平台 |
//! | `POST /merchant/adCampaign/audit` | 平台 |
//! | `POST /merchant/adCampaign/pin` | 平台 |
//! | `POST /merchant/adCampaign/takeDown` | 平台 |

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
use nextrade_core::models::merchant::ad_campaign::{
    AdCampaign, AdCampaignAuditParam, AdCampaignCreateParam, AdCampaignListQuery, AdCampaignPinParam,
    AdCampaignVO,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser, TenantContext};

use crate::AdminState;

/// 本域 8 条路由。
const ROUTE_TABLE: [(&str, &str, &str); 8] = [
    ("POST", "/merchant/adCampaign/create", "AdCampaignController.create"),
    ("GET", "/merchant/adCampaign/myList", "AdCampaignController.myList"),
    ("POST", "/merchant/adCampaign/pause", "AdCampaignController.pause"),
    ("POST", "/merchant/adCampaign/resume", "AdCampaignController.resume"),
    ("GET", "/merchant/adCampaign/auditList", "AdCampaignController.auditList"),
    ("POST", "/merchant/adCampaign/audit", "AdCampaignController.audit"),
    ("POST", "/merchant/adCampaign/pin", "AdCampaignController.pin"),
    ("POST", "/merchant/adCampaign/takeDown", "AdCampaignController.takeDown"),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/merchant/adCampaign/create", post(create))
        .route("/merchant/adCampaign/myList", get(my_list))
        .route("/merchant/adCampaign/pause", post(pause))
        .route("/merchant/adCampaign/resume", post(resume))
        .route("/merchant/adCampaign/auditList", get(audit_list))
        .route("/merchant/adCampaign/audit", post(audit))
        .route("/merchant/adCampaign/pin", post(pin))
        .route("/merchant/adCampaign/takeDown", post(take_down))
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(m, p, h)| nextrade_openapi::RouteDoc::new(m, &format!("/adminapi{p}"), h))
        .collect()
}

/// 仅 id 的请求体（暂停/恢复/下架）。
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct IdParam {
    id: Option<i32>,
}

fn require_platform(tenant: &TenantContext) -> Result<(), AppError> {
    if tenant.is_platform() {
        Ok(())
    } else {
        Err(nextrade_core::Error::Forbidden("仅平台账号可访问该接口".into()).into())
    }
}

fn param_id(p: &IdParam) -> Result<i32, AppError> {
    p.id
        .filter(|v| *v > 0)
        .ok_or_else(|| AppError(nextrade_core::Error::biz(1001, "id不能为空")))
}

/// `POST /merchant/adCampaign/create`：商家新建投放（强制本店 + 余额扣款）。
async fn create(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(param): Json<AdCampaignCreateParam>,
) -> ApiResult<AdCampaign> {
    let shop_id = tenant.require_shop().map_err(AppError)?;
    ok(state.merchant.ad_campaign.create_for_shop(shop_id, &param).await?)
}

/// `GET /merchant/adCampaign/myList`：商家「我的投放」。
async fn my_list(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Query(q): Query<AdCampaignListQuery>,
) -> ApiResult<PageResult<AdCampaignVO>> {
    let shop_id = tenant.require_shop().map_err(AppError)?;
    ok(state.merchant.ad_campaign.list_for_shop(shop_id, &q).await?)
}

/// `POST /merchant/adCampaign/pause`：商家暂停投放。
async fn pause(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(param): Json<IdParam>,
) -> ApiResult<AdCampaign> {
    let shop_id = tenant.require_shop().map_err(AppError)?;
    ok(state.merchant.ad_campaign.pause(shop_id, param_id(&param)?).await?)
}

/// `POST /merchant/adCampaign/resume`：商家恢复投放。
async fn resume(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(param): Json<IdParam>,
) -> ApiResult<AdCampaign> {
    let shop_id = tenant.require_shop().map_err(AppError)?;
    ok(state.merchant.ad_campaign.resume(shop_id, param_id(&param)?).await?)
}

/// `GET /merchant/adCampaign/auditList`：平台投放审核列表。
async fn audit_list(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Query(q): Query<AdCampaignListQuery>,
) -> ApiResult<PageResult<AdCampaignVO>> {
    require_platform(&tenant)?;
    ok(state.merchant.ad_campaign.list_for_platform(&q).await?)
}

/// `POST /merchant/adCampaign/audit`：平台审核（通过/驳回）。
async fn audit(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Extension(tenant): Extension<TenantContext>,
    Json(param): Json<AdCampaignAuditParam>,
) -> ApiResult<AdCampaign> {
    require_platform(&tenant)?;
    ok(state.merchant.ad_campaign.audit(auth.id, &param).await?)
}

/// `POST /merchant/adCampaign/pin`：平台置顶（自建 pin 投放单）。
async fn pin(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(param): Json<AdCampaignPinParam>,
) -> ApiResult<AdCampaign> {
    require_platform(&tenant)?;
    ok(state.merchant.ad_campaign.create_pin(&param).await?)
}

/// `POST /merchant/adCampaign/takeDown`：平台强制下架。
async fn take_down(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(param): Json<IdParam>,
) -> ApiResult<AdCampaign> {
    require_platform(&tenant)?;
    ok(state.merchant.ad_campaign.take_down(param_id(&param)?).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract_exactly() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 8);
        assert_eq!(docs[0].path, "/adminapi/merchant/adCampaign/create");
        assert_eq!(docs[7].path, "/adminapi/merchant/adCampaign/takeDown");
    }

    #[test]
    fn platform_guard_rejects_shop_account() {
        let shop = TenantContext::new(Some("shop".into()), Some(7), None);
        assert!(require_platform(&shop).is_err());
        let platform = TenantContext::new(Some("admin".into()), None, None);
        assert!(require_platform(&platform).is_ok());
    }
}
