//! 平台联邦 —— 管理端路由（hub::federation）。
//!
//! 设计：`~/workspace/hub-federation-design/proposal.md` §2/§3/§4/§5/§6。
//! 全部接口先过联邦总闸（总开关关 ⇒ 404，管理端亦不例外；开关本身走
//! `setting/config/federationSettings`，不受此闸限制以便重新打开）。
//!
//! | method | path | 说明 |
//! |--------|------|------|
//! | `GET`  | `/hub/overview` | 组织概览（设置摘要+计数） |
//! | `GET`  | `/hub/credentials/list` | 凭证分页列表 |
//! | `GET`  | `/hub/credentials/detail?id=` | 凭证详情 |
//! | `POST` | `/hub/credentials/create` | 新建凭证（入站钥一次性带 secret） |
//! | `POST` | `/hub/credentials/update` | 编辑凭证 |
//! | `POST` | `/hub/credentials/resetSecret` | 重置入站钥 secret |
//! | `POST` | `/hub/credentials/del` | 删除凭证 |
//! | `POST` | `/hub/credentials/fromFeedKey` | feed 钥一键生成等价凭证 |
//! | `POST` | `/hub/credentials/priceOverride/save` | 保存单品覆盖 |
//! | `POST` | `/hub/credentials/priceOverride/del` | 删除单品覆盖 |
//! | `GET`  | `/hub/pairing/list` | 配对码近况 |
//! | `POST` | `/hub/pairing/create` | 生成配对码（明文一次） |
//! | `POST` | `/hub/pairing/revoke` | 作废配对码 |
//! | `GET`  | `/hub/links/list` | 连接分页列表（role 过滤） |
//! | `POST` | `/hub/links/approve` | 批准连接 |
//! | `POST` | `/hub/links/reject` | 拒绝连接 |
//! | `POST` | `/hub/links/setStatus` | 连接启停 |
//! | `POST` | `/hub/links/del` | 删除连接 |
//! | `POST` | `/hub/myLinks/initiate` | 成员侧发起配对 |
//! | `POST` | `/hub/myLinks/poll` | 成员侧轮询/领取 |
//! | `POST` | `/hub/myLinks/heartbeat` | 成员侧手动心跳 |
//! | `GET`  | `/hub/aggregation/list` | 聚合商品分页 |
//! | `POST` | `/hub/aggregation/sync` | 执行一次聚合拉取 |
//! | `POST` | `/hub/aggregation/setState` | 聚合映射状态调整 |
//! | `GET`  | `/hub/callLogs/list` | 调用日志分页 |
//! | `GET`  | `/hub/members/list` | 会员映射分页（二期，会员模块闸） |
//! | `GET`  | `/hub/members/conflicts` | 会员冲突队列（二期） |
//! | `POST` | `/hub/members/resolveConflict` | 冲突裁决（二期） |
//! | `GET`  | `/hub/orders/list` | 订单上行汇总分页（二期，订单模块闸） |
//! | `POST` | `/hub/myLinks/syncMembers` | 成员侧手动同步会员（二期） |
//! | `POST` | `/hub/myLinks/reportOrders` | 成员侧手动上报订单（二期） |

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;

use nextrade_core::models::hub::federation::{
    HubAggregationListQuery, HubAggregationVO, HubCallLogListQuery, HubCallLogVO,
    HubCredentialDetailVO, HubCredentialListQuery, HubCredentialSaveDTO, HubCredentialSecretVO,
    HubCredentialVO, HubIdDTO, HubLinkApproveDTO, HubLinkListQuery, HubLinkRejectDTO,
    HubLinkVO, HubMemberConflictListQuery, HubMemberConflictVO, HubMemberLinkVO,
    HubMemberListQuery, HubMemberResolveDTO, HubOrderSummaryListQuery, HubOrderSummaryVO,
    HubPairInitiateDTO, HubPairingCodeVO, HubPairingCreateDTO, HubPriceOverrideSaveDTO,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult};

use crate::AdminState;

/// 本域路由（去掉 `/adminapi` 前缀）。
const ROUTE_TABLE: [(&str, &str, &str); 31] = [
    ("GET", "/hub/overview", "HubController.overview"),
    ("GET", "/hub/credentials/list", "HubController.credentialList"),
    ("GET", "/hub/credentials/detail", "HubController.credentialDetail"),
    ("POST", "/hub/credentials/create", "HubController.credentialCreate"),
    ("POST", "/hub/credentials/update", "HubController.credentialUpdate"),
    ("POST", "/hub/credentials/resetSecret", "HubController.credentialResetSecret"),
    ("POST", "/hub/credentials/del", "HubController.credentialDel"),
    ("POST", "/hub/credentials/fromFeedKey", "HubController.credentialFromFeedKey"),
    ("POST", "/hub/credentials/priceOverride/save", "HubController.savePriceOverride"),
    ("POST", "/hub/credentials/priceOverride/del", "HubController.deletePriceOverride"),
    ("GET", "/hub/pairing/list", "HubController.pairingList"),
    ("POST", "/hub/pairing/create", "HubController.pairingCreate"),
    ("POST", "/hub/pairing/revoke", "HubController.pairingRevoke"),
    ("GET", "/hub/links/list", "HubController.linkList"),
    ("POST", "/hub/links/approve", "HubController.linkApprove"),
    ("POST", "/hub/links/reject", "HubController.linkReject"),
    ("POST", "/hub/links/setStatus", "HubController.linkSetStatus"),
    ("POST", "/hub/links/del", "HubController.linkDel"),
    ("POST", "/hub/myLinks/initiate", "HubController.myLinkInitiate"),
    ("POST", "/hub/myLinks/poll", "HubController.myLinkPoll"),
    ("POST", "/hub/myLinks/heartbeat", "HubController.myLinkHeartbeat"),
    ("GET", "/hub/aggregation/list", "HubController.aggregationList"),
    ("POST", "/hub/aggregation/sync", "HubController.aggregationSync"),
    ("POST", "/hub/aggregation/setState", "HubController.aggregationSetState"),
    ("GET", "/hub/callLogs/list", "HubController.callLogList"),
    ("GET", "/hub/members/list", "HubController.memberList"),
    ("GET", "/hub/members/conflicts", "HubController.memberConflicts"),
    ("POST", "/hub/members/resolveConflict", "HubController.memberResolveConflict"),
    ("GET", "/hub/orders/list", "HubController.orderSummaryList"),
    ("POST", "/hub/myLinks/syncMembers", "HubController.myLinkSyncMembers"),
    ("POST", "/hub/myLinks/reportOrders", "HubController.myLinkReportOrders"),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/hub/overview", get(overview))
        .route("/hub/credentials/list", get(credential_list))
        .route("/hub/credentials/detail", get(credential_detail))
        .route("/hub/credentials/create", post(credential_create))
        .route("/hub/credentials/update", post(credential_update))
        .route("/hub/credentials/resetSecret", post(credential_reset_secret))
        .route("/hub/credentials/del", post(credential_del))
        .route("/hub/credentials/fromFeedKey", post(credential_from_feed_key))
        .route("/hub/credentials/priceOverride/save", post(save_override))
        .route("/hub/credentials/priceOverride/del", post(delete_override))
        .route("/hub/pairing/list", get(pairing_list))
        .route("/hub/pairing/create", post(pairing_create))
        .route("/hub/pairing/revoke", post(pairing_revoke))
        .route("/hub/links/list", get(link_list))
        .route("/hub/links/approve", post(link_approve))
        .route("/hub/links/reject", post(link_reject))
        .route("/hub/links/setStatus", post(link_set_status))
        .route("/hub/links/del", post(link_del))
        .route("/hub/myLinks/initiate", post(my_link_initiate))
        .route("/hub/myLinks/poll", post(my_link_poll))
        .route("/hub/myLinks/heartbeat", post(my_link_heartbeat))
        .route("/hub/aggregation/list", get(aggregation_list))
        .route("/hub/aggregation/sync", post(aggregation_sync))
        .route("/hub/aggregation/setState", post(aggregation_set_state))
        .route("/hub/callLogs/list", get(call_log_list))
        .route("/hub/members/list", get(member_list))
        .route("/hub/members/conflicts", get(member_conflicts))
        .route("/hub/members/resolveConflict", post(member_resolve_conflict))
        .route("/hub/orders/list", get(order_summary_list))
        .route("/hub/myLinks/syncMembers", post(my_link_sync_members))
        .route("/hub/myLinks/reportOrders", post(my_link_report_orders))
}

/// 本域已实现路由（覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| {
            nextrade_openapi::RouteDoc::new(method, &format!("/adminapi{path}"), handler)
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct IdQuery {
    id: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MapStateDTO {
    map_id: i32,
    sync_state: i32,
}

async fn overview(State(state): State<AdminState>) -> ApiResult<Value> {
    state.hub.federation.gate().await?;
    ok(state.hub.federation.overview().await?)
}

async fn credential_list(
    State(state): State<AdminState>,
    Query(q): Query<HubCredentialListQuery>,
) -> ApiResult<PageResult<HubCredentialVO>> {
    state.hub.federation.gate().await?;
    ok(state.hub.federation.list_credentials(&q).await?)
}

async fn credential_detail(
    State(state): State<AdminState>,
    Query(q): Query<IdQuery>,
) -> ApiResult<HubCredentialDetailVO> {
    state.hub.federation.gate().await?;
    ok(state.hub.federation.credential_detail(q.id).await?)
}

async fn credential_create(
    State(state): State<AdminState>,
    Json(dto): Json<HubCredentialSaveDTO>,
) -> ApiResult<HubCredentialSecretVO> {
    ok(state.hub.federation.create_credential(&dto).await?)
}

async fn credential_update(
    State(state): State<AdminState>,
    Json(dto): Json<HubCredentialSaveDTO>,
) -> ApiResult<&'static str> {
    state.hub.federation.update_credential(&dto).await?;
    ok("更新成功")
}

async fn credential_reset_secret(
    State(state): State<AdminState>,
    Json(dto): Json<HubIdDTO>,
) -> ApiResult<HubCredentialSecretVO> {
    ok(state.hub.federation.reset_credential_secret(dto.id).await?)
}

async fn credential_del(
    State(state): State<AdminState>,
    Json(dto): Json<HubIdDTO>,
) -> ApiResult<&'static str> {
    state.hub.federation.delete_credential(dto.id).await?;
    ok("删除成功")
}

async fn credential_from_feed_key(
    State(state): State<AdminState>,
    Json(dto): Json<HubIdDTO>,
) -> ApiResult<HubCredentialSecretVO> {
    let feed_key_id = dto
        .feed_key_id
        .ok_or_else(|| nextrade_core::Error::Validation("缺少 feedKeyId".to_string()))?;
    ok(state.hub.federation.from_feed_key(feed_key_id).await?)
}

async fn save_override(
    State(state): State<AdminState>,
    Json(dto): Json<HubPriceOverrideSaveDTO>,
) -> ApiResult<&'static str> {
    state.hub.federation.save_override(&dto).await?;
    ok("保存成功")
}

async fn delete_override(
    State(state): State<AdminState>,
    Json(dto): Json<HubIdDTO>,
) -> ApiResult<&'static str> {
    let credential_id = dto
        .credential_id
        .ok_or_else(|| nextrade_core::Error::Validation("缺少 credentialId".to_string()))?;
    let product_id = dto
        .product_id
        .ok_or_else(|| nextrade_core::Error::Validation("缺少 productId".to_string()))?;
    state
        .hub
        .federation
        .delete_override(credential_id, product_id)
        .await?;
    ok("删除成功")
}

async fn pairing_list(State(state): State<AdminState>) -> ApiResult<Vec<Value>> {
    ok(state.hub.federation.list_pairing_codes().await?)
}

async fn pairing_create(
    State(state): State<AdminState>,
    Json(dto): Json<HubPairingCreateDTO>,
) -> ApiResult<HubPairingCodeVO> {
    ok(state.hub.federation.create_pairing_code(&dto).await?)
}

async fn pairing_revoke(
    State(state): State<AdminState>,
    Json(dto): Json<HubIdDTO>,
) -> ApiResult<&'static str> {
    state.hub.federation.revoke_pairing_code(dto.id).await?;
    ok("已作废")
}

async fn link_list(
    State(state): State<AdminState>,
    Query(q): Query<HubLinkListQuery>,
) -> ApiResult<PageResult<HubLinkVO>> {
    state.hub.federation.gate().await?;
    ok(state.hub.federation.list_links(&q).await?)
}

async fn link_approve(
    State(state): State<AdminState>,
    Json(dto): Json<HubLinkApproveDTO>,
) -> ApiResult<&'static str> {
    state.hub.federation.approve_link(&dto).await?;
    ok("已批准")
}

async fn link_reject(
    State(state): State<AdminState>,
    Json(dto): Json<HubLinkRejectDTO>,
) -> ApiResult<&'static str> {
    state.hub.federation.reject_link(&dto).await?;
    ok("已拒绝")
}

async fn link_set_status(
    State(state): State<AdminState>,
    Json(dto): Json<HubIdDTO>,
) -> ApiResult<&'static str> {
    let status = dto
        .status
        .ok_or_else(|| nextrade_core::Error::Validation("缺少 status".to_string()))?;
    state.hub.federation.set_link_status(dto.id, status).await?;
    ok("状态已更新")
}

async fn link_del(
    State(state): State<AdminState>,
    Json(dto): Json<HubIdDTO>,
) -> ApiResult<&'static str> {
    state.hub.federation.delete_link(dto.id).await?;
    ok("删除成功")
}

async fn my_link_initiate(
    State(state): State<AdminState>,
    Json(dto): Json<HubPairInitiateDTO>,
) -> ApiResult<HubLinkVO> {
    ok(state.hub.federation.initiate_pairing(&dto).await?)
}

async fn my_link_poll(
    State(state): State<AdminState>,
    Json(dto): Json<HubIdDTO>,
) -> ApiResult<HubLinkVO> {
    ok(state.hub.federation.poll_link(dto.id).await?)
}

async fn my_link_heartbeat(
    State(state): State<AdminState>,
    Json(dto): Json<HubIdDTO>,
) -> ApiResult<&'static str> {
    state.hub.federation.send_heartbeat(dto.id).await?;
    ok("心跳已发送")
}

async fn aggregation_list(
    State(state): State<AdminState>,
    Query(q): Query<HubAggregationListQuery>,
) -> ApiResult<PageResult<HubAggregationVO>> {
    state.hub.federation.gate().await?;
    ok(state.hub.federation.list_aggregation(&q).await?)
}

async fn aggregation_sync(
    State(state): State<AdminState>,
    Json(dto): Json<HubIdDTO>,
) -> ApiResult<Value> {
    ok(state
        .hub
        .federation
        .sync_aggregation(dto.link_id.unwrap_or(0), dto.credential_id.unwrap_or(0))
        .await?)
}

async fn aggregation_set_state(
    State(state): State<AdminState>,
    Json(dto): Json<MapStateDTO>,
) -> ApiResult<&'static str> {
    state
        .hub
        .federation
        .set_map_state(dto.map_id, dto.sync_state)
        .await?;
    ok("状态已更新")
}

async fn call_log_list(
    State(state): State<AdminState>,
    Query(q): Query<HubCallLogListQuery>,
) -> ApiResult<PageResult<HubCallLogVO>> {
    state.hub.federation.gate().await?;
    ok(state.hub.federation.list_call_logs(&q).await?)
}

// ———————————————————— 二期：会员联邦 / 订单上行 ————————————————————

async fn member_list(
    State(state): State<AdminState>,
    Query(q): Query<HubMemberListQuery>,
) -> ApiResult<PageResult<HubMemberLinkVO>> {
    ok(state.hub.federation.list_members(&q).await?)
}

async fn member_conflicts(
    State(state): State<AdminState>,
    Query(q): Query<HubMemberConflictListQuery>,
) -> ApiResult<PageResult<HubMemberConflictVO>> {
    ok(state.hub.federation.list_member_conflicts(&q).await?)
}

async fn member_resolve_conflict(
    State(state): State<AdminState>,
    Json(dto): Json<HubMemberResolveDTO>,
) -> ApiResult<&'static str> {
    state.hub.federation.resolve_member_conflict(&dto).await?;
    ok("裁决已生效")
}

async fn order_summary_list(
    State(state): State<AdminState>,
    Query(q): Query<HubOrderSummaryListQuery>,
) -> ApiResult<PageResult<HubOrderSummaryVO>> {
    ok(state.hub.federation.list_order_summaries(&q).await?)
}

async fn my_link_sync_members(
    State(state): State<AdminState>,
    Json(dto): Json<HubIdDTO>,
) -> ApiResult<Value> {
    ok(state.hub.federation.sync_members(dto.id).await?)
}

async fn my_link_report_orders(
    State(state): State<AdminState>,
    Json(dto): Json<HubIdDTO>,
) -> ApiResult<Value> {
    ok(state.hub.federation.report_orders(dto.id).await?)
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
