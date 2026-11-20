//! 管理端退款申请路由。
//! 对应原 Java `controller.finance.RefundApplyController`
//! （`@RequestMapping("/adminapi/finance/refundApply")`）。
//!
//! | 本模块路由 | 原 Java |
//! |-----------|--------|
//! | `GET  /finance/refundApply/config` | `RefundApplyController.config` |
//! | `GET  /finance/refundApply/list` | `RefundApplyController.list` |
//! | `GET  /finance/refundApply/detail` | `RefundApplyController.detail` |
//! | `POST /finance/refundApply/audit` | `RefundApplyController.audit` |
//! | `POST /finance/refundApply/offlineAudit` | `RefundApplyController.offlineAudit` |
//! | `POST /finance/refundApply/create` | `RefundApplyController.create` |
//! | `POST /finance/refundApply/update` | `RefundApplyController.update` |
//! | `GET  /finance/refundApply/proactivePreview` | 我方新增（无 Java 对应） |
//! | `POST /finance/refundApply/proactiveCreate` | 我方新增（无 Java 对应） |
//!
//! 未迁移：`del` / `updateField` / `batch`（退款申请为审计数据，
//! 原系统同样对删除有严格限制；待确认业务口径后补齐）。

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
use nextrade_core::models::payment::refund_apply::{
    OfflineAuditDTO, ProactiveRefundCreateDTO, ProactiveRefundPreviewVO, RefundApplyAuditDTO,
    RefundApplyCreateDTO, RefundApplyDetailVO, RefundApplyListQuery, RefundApplyUpdateDTO,
    RefundApplyVO, RefundInitiator,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};

use crate::AdminState;

/// 退款申请域路由（挂载于 `/adminapi` 下）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/finance/refundApply/config", get(config))
        .route("/finance/refundApply/list", get(list))
        .route("/finance/refundApply/detail", get(detail))
        .route("/finance/refundApply/audit", post(audit))
        .route("/finance/refundApply/offlineAudit", post(offline_audit))
        .route("/finance/refundApply/create", post(create))
        .route("/finance/refundApply/update", post(update))
        .route(
            "/finance/refundApply/proactivePreview",
            get(proactive_preview),
        )
        .route("/finance/refundApply/proactiveCreate", post(proactive_create))
}

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    [
        (
            "GET",
            "/adminapi/finance/refundApply/config",
            "RefundApplyController.config",
        ),
        (
            "GET",
            "/adminapi/finance/refundApply/list",
            "RefundApplyController.list",
        ),
        (
            "GET",
            "/adminapi/finance/refundApply/detail",
            "RefundApplyController.detail",
        ),
        (
            "POST",
            "/adminapi/finance/refundApply/audit",
            "RefundApplyController.audit",
        ),
        (
            "POST",
            "/adminapi/finance/refundApply/offlineAudit",
            "RefundApplyController.offlineAudit",
        ),
        (
            "POST",
            "/adminapi/finance/refundApply/create",
            "RefundApplyController.create",
        ),
        (
            "POST",
            "/adminapi/finance/refundApply/update",
            "RefundApplyController.update",
        ),
        (
            "GET",
            "/adminapi/finance/refundApply/proactivePreview",
            "我方新增 proactiveRefund.preview（无 Java 对应）",
        ),
        (
            "POST",
            "/adminapi/finance/refundApply/proactiveCreate",
            "我方新增 proactiveRefund.create（无 Java 对应）",
        ),
    ]
    .iter()
    .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, p, j))
    .collect()
}

/// `GET /adminapi/finance/refundApply/config`：审核状态配置。
///
/// 返回 `[{status: 0, name: "待处理"}, ...]`。
async fn config() -> ApiResult<Vec<serde_json::Value>> {
    let cfg: Vec<serde_json::Value> =
        nextrade_services::payment::refund_apply::RefundApplyService::status_config()
            .into_iter()
            .map(|(status, name)| serde_json::json!({ "status": status, "name": name }))
            .collect();
    ok(cfg)
}

/// `GET /adminapi/finance/refundApply/list`：退款申请分页。
async fn list(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Query(mut q): Query<RefundApplyListQuery>,
) -> ApiResult<PageResult<RefundApplyVO>> {
    q.shop_id = auth.resolve_shop_scope(q.shop_id)?;
    ok(state.payment.refund_apply.list(&q).await?)
}

/// `GET /adminapi/finance/refundApply/detail?id=`：申请详情（含日志）。
#[derive(Debug, Deserialize)]
struct IdQuery {
    id: i32,
}

/// 退款申请归属校验：店铺账号只能访问本店退款单（平台账号跳过）。
async fn ensure_refund_access(
    state: &AdminState,
    auth: &AuthUser,
    refund_id: i32,
) -> Result<(), AppError> {
    if auth.scoped_shop_id().is_some() {
        state
            .payment
            .refund_apply
            .shop_id_of(refund_id)
            .await
            .map_err(AppError)?;
        let shop_id = state
            .payment
            .refund_apply
            .shop_id_of(refund_id)
            .await
            .map_err(AppError)?;
        auth.ensure_shop_access(shop_id).map_err(AppError)?;
    }
    Ok(())
}

async fn detail(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<IdQuery>,
) -> ApiResult<RefundApplyDetailVO> {
    ensure_refund_access(&state, &auth, q.id).await?;
    ok(state.payment.refund_apply.detail(q.id).await?)
}

/// `POST /adminapi/finance/refundApply/audit`：财务审核。
async fn audit(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<RefundApplyAuditDTO>,
) -> ApiResult<&'static str> {
    ensure_refund_access(&state, &auth, dto.refund_id).await?;
    state.payment.refund_apply.audit(&dto).await?;
    ok("审核完成")
}

/// `POST /adminapi/finance/refundApply/offlineAudit`：确认线下转账。
async fn offline_audit(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<OfflineAuditDTO>,
) -> ApiResult<&'static str> {
    ensure_refund_access(&state, &auth, dto.refund_id).await?;
    state.payment.refund_apply.offline_audit(&dto).await?;
    ok("已确认线下转账")
}

/// `POST /adminapi/finance/refundApply/create`：创建退款申请。
async fn create(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(mut dto): Json<RefundApplyCreateDTO>,
) -> ApiResult<i32> {
    dto.shop_id = Some(auth.require_shop_scope(dto.shop_id)?);
    ok(state.payment.refund_apply.create(&dto).await?)
}

/// 主动退款预检入参。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrderIdQuery {
    order_id: i32,
}

/// 主动退款的发起方与作用域一律按登录态推导（我方新增）：
/// 平台账号 → initiator=平台、不限店铺；店铺账号 → initiator=
/// 商家、作用域 = 登录店铺。请求体不携带也不采信 shopId/userId。
fn proactive_actor(auth: &AuthUser) -> (i32, Option<i32>) {
    match auth.scoped_shop_id() {
        None => (RefundInitiator::Platform as i32, None),
        scope => (RefundInitiator::Merchant as i32, scope),
    }
}

/// `GET /adminapi/finance/refundApply/proactivePreview?orderId=`
/// （我方新增）：主动退款弹窗预检——订单状态门 + 作用域校验后
/// 返回实付与可退余额。
async fn proactive_preview(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<OrderIdQuery>,
) -> ApiResult<ProactiveRefundPreviewVO> {
    let (_, scope) = proactive_actor(&auth);
    ok(state
        .payment
        .refund_apply
        .proactive_preview(q.order_id, scope)
        .await?)
}

/// `POST /adminapi/finance/refundApply/proactiveCreate`（我方新增）：
/// 平台/商家对已支付订单发起主动退款，生成 Wait 态退款申请，
/// 后续走既有财务审核出款链。
async fn proactive_create(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<ProactiveRefundCreateDTO>,
) -> ApiResult<i32> {
    let (initiator, scope) = proactive_actor(&auth);
    let order_id = dto.order_id.ok_or_else(|| {
        AppError(nextrade_core::Error::Validation("订单ID不能为空".into()))
    })?;
    let amount = dto.amount.ok_or_else(|| {
        AppError(nextrade_core::Error::Validation("退款金额不能为空".into()))
    })?;
    let reason = dto.reason.as_deref().unwrap_or("");
    ok(state
        .payment
        .refund_apply
        .create_proactive(order_id, amount, reason, initiator, scope)
        .await?)
}

/// `POST /adminapi/finance/refundApply/update`：更新申请（仅待处理）。
async fn update(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<RefundApplyUpdateDTO>,
) -> ApiResult<&'static str> {
    ensure_refund_access(&state, &auth, dto.refund_id).await?;
    state.payment.refund_apply.update(&dto).await?;
    ok("更新成功")
}
#[cfg(test)]
mod tenant_tests {
    use super::*;

    #[test]
    fn shop_scope_forced_for_refund_access() {
        use nextrade_core::Error;
        let shop = AuthUser::new(1, "s".into(), None, Some("shop".into()), Some(7));
        assert!(shop.ensure_shop_access(7).is_ok());
        assert!(shop.ensure_shop_access(9).is_err());
        assert_eq!(
            shop.resolve_shop_scope(Some(9))
                .unwrap_err()
                .to_string()
                .contains("无权访问店铺 9"),
            true
        );
    }
}
