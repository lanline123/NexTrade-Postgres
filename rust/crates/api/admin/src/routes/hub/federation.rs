//! 平台联邦 —— 管理端路由（hub::federation）。
//!
//! 设计：`~/workspace/hub-federation-design/proposal.md` §2/§3/§4/§5/§6。
//! 全部接口先过联邦总闸（总开关关 ⇒ 404，管理端亦不例外；开关本身走
//! `setting/config/federationSettings`，不受此闸限制以便重新打开）。
//!
//! 角色守卫（全站体检线 6 登记 R1 修复）：联邦端点不在权限生成表内、权限
//! 中间件对无条目路由放行，故角色隔离靠路由层——中枢管理面（凭证/配对/
//! 连接审批/聚合/会员映射/订单汇总/SSO 审计/配置发布/跨店主单/中央账本/
//! 跨站结算）整域挂 `platform_only`，仅平台账号可达（与 systemUpdate、
//! siteClone、federationSettings 先例同口径）；成员侧自助面（myLinks/*、
//! 配置落地确认、跨店路由单、账本镜像）保持仅登录可达。`/hub/links/list`
//! 为两面共用读端点，handler 内按 role 分流（非平台仅 role=2 本机出站）。
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
//! | `GET`  | `/hub/sso/requests` | 登录互认授权审计（三期①，迁移 042，会员模块闸） |
//! | `GET`  | `/hub/config/packages` | 配置包列表（三期②，迁移 043） |
//! | `POST` | `/hub/config/publish` | 发布配置包（三期②） |
//! | `POST` | `/hub/config/revoke` | 撤销配置包（三期②） |
//! | `GET`  | `/hub/config/acks` | 配置回执列表（三期②） |
//! | `GET`  | `/hub/config/applies` | 本机配置落地列表（三期②，成员侧） |
//! | `POST` | `/hub/config/decide` | 手动包确认/拒绝（三期②，成员侧） |
//! | `POST` | `/hub/myLinks/pullConfig` | 按连接拉取配置（三期②，成员侧） |
//! | `GET`  | `/hub/xorder/list` | 跨店主单列表（三期③，迁移 044，订单模块闸） |
//! | `POST` | `/hub/xorder/routeRetry` | 跨店路由重试（三期③，中枢侧） |
//! | `GET`  | `/hub/xorder/routes` | 本机路由单列表（三期③，成员侧） |
//! | `POST` | `/hub/xorder/syncRouteStatus` | 路由单状态同步（三期③，成员侧） |
//! | `GET`  | `/hub/wallet/accounts` | 中央账本账户列表（三期④，迁移 045，会员模块闸） |
//! | `GET`  | `/hub/wallet/lines` | 中央账本行列表（三期④） |
//! | `POST` | `/hub/wallet/postLine` | 中央账本手工记账（三期④，中枢侧） |
//! | `POST` | `/hub/wallet/releaseExpired` | 释放过期冻结（三期④，中枢侧） |
//! | `GET`  | `/hub/wallet/mirror` | 本机账本镜像列表（三期④，成员侧） |
//! | `POST` | `/hub/myLinks/mergeWallet` | 存量并账（三期④，成员侧） |
//! | `GET`  | `/hub/settle/accounts` | 预付账户列表（三期⑤，迁移 046，订单模块闸） |
//! | `POST` | `/hub/settle/accountOpen` | 预付账户开户（三期⑤） |
//! | `POST` | `/hub/settle/accountAdjust` | 预付账户充值登记（三期⑤，只记账） |
//! | `GET`  | `/hub/settle/lines` | 账目行列表（三期⑤） |
//! | `POST` | `/hub/settle/setLineStatus` | 账目行差异挂起/恢复（三期⑤） |
//! | `POST` | `/hub/settle/reprice` | 未定费率行重算（三期⑤，body.id = 连接 ID） |
//! | `GET`  | `/hub/settle/statements` | 结算单列表（三期⑤） |
//! | `POST` | `/hub/settle/generate` | 生成结算单（三期⑤） |
//! | `POST` | `/hub/settle/confirm` | 本方确认结算单（三期⑤，body.id = 结算单 ID） |
//! | `POST` | `/hub/settle/markSettled` | 结清登记（三期⑤，只记账不动资金） |

use axum::{
    extract::{Extension, Query, State},
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
    ConfigDecideDTO, ConfigPublishDTO, HubConfigAckListQuery, HubConfigAckVO,
    HubConfigApplyListQuery, HubConfigApplyVO, HubConfigListQuery, HubConfigPackageVO,
    HubPairInitiateDTO, HubPairingCodeVO, HubPairingCreateDTO, HubPriceOverrideSaveDTO,
    HubSettleAccountVO, HubSettleLineListQuery, HubSettleLineVO, HubSettleStatementListQuery,
    HubSettleStatementVO, HubSsoListQuery, HubSsoRequestVO, HubWalletAccountListQuery,
    HubWalletLineListQuery, HubWalletLineVO, HubWalletMirrorListQuery, HubWalletMirrorVO,
    HubXorderListQuery, HubXorderRouteListQuery, HubXorderRouteVO, HubXorderVO,
    SettleAccountAdjustDTO, SettleAccountOpenDTO, SettleGenerateDTO, SettleLineStatusDTO,
    WalletAdminPostDTO, WalletBalanceVO, WalletPostVO,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};

use crate::support::tenant::platform_only;

use crate::AdminState;

/// 本域路由（去掉 `/adminapi` 前缀）。
const ROUTE_TABLE: [(&str, &str, &str); 59] = [
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
    ("GET", "/hub/sso/requests", "HubController.ssoRequests"),
    ("GET", "/hub/config/packages", "HubController.configPackages"),
    ("POST", "/hub/config/publish", "HubController.configPublish"),
    ("POST", "/hub/config/revoke", "HubController.configRevoke"),
    ("GET", "/hub/config/acks", "HubController.configAcks"),
    ("GET", "/hub/config/applies", "HubController.configApplies"),
    ("POST", "/hub/config/decide", "HubController.configDecide"),
    ("POST", "/hub/myLinks/pullConfig", "HubController.myLinkPullConfig"),
    ("GET", "/hub/xorder/list", "HubController.xorderList"),
    ("POST", "/hub/xorder/routeRetry", "HubController.xorderRouteRetry"),
    ("GET", "/hub/xorder/routes", "HubController.xorderRoutes"),
    ("POST", "/hub/xorder/syncRouteStatus", "HubController.xorderSyncRouteStatus"),
    ("GET", "/hub/wallet/accounts", "HubController.walletAccounts"),
    ("GET", "/hub/wallet/lines", "HubController.walletLines"),
    ("POST", "/hub/wallet/postLine", "HubController.walletPostLine"),
    ("POST", "/hub/wallet/releaseExpired", "HubController.walletReleaseExpired"),
    ("GET", "/hub/wallet/mirror", "HubController.walletMirror"),
    ("POST", "/hub/myLinks/mergeWallet", "HubController.myLinkMergeWallet"),
    ("GET", "/hub/settle/accounts", "HubController.settleAccounts"),
    ("POST", "/hub/settle/accountOpen", "HubController.settleAccountOpen"),
    ("POST", "/hub/settle/accountAdjust", "HubController.settleAccountAdjust"),
    ("GET", "/hub/settle/lines", "HubController.settleLines"),
    ("POST", "/hub/settle/setLineStatus", "HubController.settleSetLineStatus"),
    ("POST", "/hub/settle/reprice", "HubController.settleReprice"),
    ("GET", "/hub/settle/statements", "HubController.settleStatements"),
    ("POST", "/hub/settle/generate", "HubController.settleGenerate"),
    ("POST", "/hub/settle/confirm", "HubController.settleConfirm"),
    ("POST", "/hub/settle/markSettled", "HubController.settleMarkSettled"),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    // 中枢管理面（46 条）：整域挂 platform_only，仅平台账号可达。联邦端点
    // 不在权限生成表内（/hub/ 条目为 0），权限中间件对无条目路由放行，
    // 若不挂本层，任一店铺级管理账号在联邦开启时即可签发/重置凭证密钥、
    // 中央账本记账、确认跨站结算（体检线 6 登记 R1）。
    let hub_admin = Router::new()
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
        .route("/hub/links/approve", post(link_approve))
        .route("/hub/links/reject", post(link_reject))
        .route("/hub/links/setStatus", post(link_set_status))
        .route("/hub/links/del", post(link_del))
        .route("/hub/aggregation/list", get(aggregation_list))
        .route("/hub/aggregation/sync", post(aggregation_sync))
        .route("/hub/aggregation/setState", post(aggregation_set_state))
        .route("/hub/callLogs/list", get(call_log_list))
        .route("/hub/members/list", get(member_list))
        .route("/hub/members/conflicts", get(member_conflicts))
        .route("/hub/members/resolveConflict", post(member_resolve_conflict))
        .route("/hub/orders/list", get(order_summary_list))
        .route("/hub/sso/requests", get(sso_requests))
        .route("/hub/config/packages", get(config_packages))
        .route("/hub/config/publish", post(config_publish))
        .route("/hub/config/revoke", post(config_revoke))
        .route("/hub/config/acks", get(config_acks))
        .route("/hub/xorder/list", get(xorder_list))
        .route("/hub/xorder/routeRetry", post(xorder_route_retry))
        .route("/hub/wallet/accounts", get(wallet_accounts))
        .route("/hub/wallet/lines", get(wallet_lines))
        .route("/hub/wallet/postLine", post(wallet_post_line))
        .route("/hub/wallet/releaseExpired", post(wallet_release_expired))
        .route("/hub/settle/accounts", get(settle_accounts))
        .route("/hub/settle/accountOpen", post(settle_account_open))
        .route("/hub/settle/accountAdjust", post(settle_account_adjust))
        .route("/hub/settle/lines", get(settle_lines))
        .route("/hub/settle/setLineStatus", post(settle_set_line_status))
        .route("/hub/settle/reprice", post(settle_reprice))
        .route("/hub/settle/statements", get(settle_statements))
        .route("/hub/settle/generate", post(settle_generate))
        .route("/hub/settle/confirm", post(settle_confirm))
        .route("/hub/settle/markSettled", post(settle_mark_settled))
        .layer(axum::middleware::from_fn(platform_only));
    // 成员侧自助面（13 条）：本机作为成员站的视角（发起配对、同步会员/订单、
    // 配置落地确认、跨店路由单、账本镜像），保持仅登录可达、不挂平台守卫。
    // /hub/links/list 为两面共用读端点（中枢页传 role=1、我的连接页传
    // role=2），不整端点挂守卫，改在 link_list handler 内按 role 分流。
    let member_side = Router::new()
        .route("/hub/links/list", get(link_list))
        .route("/hub/myLinks/initiate", post(my_link_initiate))
        .route("/hub/myLinks/poll", post(my_link_poll))
        .route("/hub/myLinks/heartbeat", post(my_link_heartbeat))
        .route("/hub/myLinks/syncMembers", post(my_link_sync_members))
        .route("/hub/myLinks/reportOrders", post(my_link_report_orders))
        .route("/hub/myLinks/pullConfig", post(my_link_pull_config))
        .route("/hub/myLinks/mergeWallet", post(my_link_merge_wallet))
        .route("/hub/config/applies", get(config_applies))
        .route("/hub/config/decide", post(config_decide))
        .route("/hub/xorder/routes", get(xorder_routes))
        .route("/hub/xorder/syncRouteStatus", post(xorder_sync_route_status))
        .route("/hub/wallet/mirror", get(wallet_mirror));
    Router::new().merge(hub_admin).merge(member_side)
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
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<HubLinkListQuery>,
) -> ApiResult<PageResult<HubLinkVO>> {
    state.hub.federation.gate().await?;
    // 两面共用读端点：role=1 为中枢侧连接（对端是成员站），role=2 为本机
    // 作为成员站的出站连接（「我的连接」页口径）。非平台账号仅可查 role=2，
    // 查中枢侧或不带 role 的全量列表按平台守卫同文案拒绝（体检 R1）。
    if !auth.is_platform() && q.role != Some(2) {
        return Err(AppError(nextrade_core::Error::Forbidden(
            "仅平台账号可访问该系统配置".into(),
        )));
    }
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

/// 三期①：登录互认授权审计列表。
async fn sso_requests(
    State(state): State<AdminState>,
    Query(q): Query<HubSsoListQuery>,
) -> ApiResult<PageResult<HubSsoRequestVO>> {
    ok(state.hub.federation.list_sso_requests(&q).await?)
}

/// 三期②：配置包列表（中枢）。
async fn config_packages(
    State(state): State<AdminState>,
    Query(q): Query<HubConfigListQuery>,
) -> ApiResult<PageResult<HubConfigPackageVO>> {
    ok(state.hub.federation.list_config_packages(&q).await?)
}

/// 三期②：发布配置包（中枢）。
async fn config_publish(
    State(state): State<AdminState>,
    Json(dto): Json<ConfigPublishDTO>,
) -> ApiResult<HubConfigPackageVO> {
    ok(state.hub.federation.publish_config_package(&dto).await?)
}

/// 三期②：撤销配置包（中枢）。
async fn config_revoke(
    State(state): State<AdminState>,
    Json(dto): Json<HubIdDTO>,
) -> ApiResult<&'static str> {
    state.hub.federation.revoke_config_package(dto.id).await?;
    ok("已撤销")
}

/// 三期②：配置回执列表（中枢）。
async fn config_acks(
    State(state): State<AdminState>,
    Query(q): Query<HubConfigAckListQuery>,
) -> ApiResult<PageResult<HubConfigAckVO>> {
    ok(state.hub.federation.list_config_acks(&q).await?)
}

/// 三期②：本机配置落地列表（成员）。
async fn config_applies(
    State(state): State<AdminState>,
    Query(q): Query<HubConfigApplyListQuery>,
) -> ApiResult<PageResult<HubConfigApplyVO>> {
    ok(state.hub.federation.list_config_applies(&q).await?)
}

/// 三期②：手动包确认/拒绝（成员）。
async fn config_decide(
    State(state): State<AdminState>,
    Json(dto): Json<ConfigDecideDTO>,
) -> ApiResult<&'static str> {
    state.hub.federation.decide_config_apply(&dto).await?;
    ok("已处理")
}

/// 三期②：按连接拉取配置（成员）。
async fn my_link_pull_config(
    State(state): State<AdminState>,
    Json(dto): Json<HubIdDTO>,
) -> ApiResult<Value> {
    ok(state.hub.federation.pull_config(dto.id).await?)
}

/// 三期③：跨店主单列表（中枢）。
async fn xorder_list(
    State(state): State<AdminState>,
    Query(q): Query<HubXorderListQuery>,
) -> ApiResult<PageResult<HubXorderVO>> {
    ok(state.hub.federation.list_xorders_admin(&q).await?)
}

/// 三期③：跨店路由重试（中枢，对已付主单重跑路由补落）。
async fn xorder_route_retry(
    State(state): State<AdminState>,
    Json(dto): Json<HubIdDTO>,
) -> ApiResult<&'static str> {
    state.hub.federation.xorder_route_retry(dto.id).await?;
    ok("路由已重跑")
}

/// 三期③：本机路由单列表（成员）。
async fn xorder_routes(
    State(state): State<AdminState>,
    Query(q): Query<HubXorderRouteListQuery>,
) -> ApiResult<PageResult<HubXorderRouteVO>> {
    ok(state.hub.federation.list_xorder_routes_admin(&q).await?)
}

/// 三期③：路由单状态同步（成员，读本地单状态回传中枢）。
async fn xorder_sync_route_status(
    State(state): State<AdminState>,
    Json(dto): Json<HubIdDTO>,
) -> ApiResult<Value> {
    ok(state.hub.federation.sync_route_status(dto.id).await?)
}

/// 三期④：中央账本账户列表（中枢）。
async fn wallet_accounts(
    State(state): State<AdminState>,
    Query(q): Query<HubWalletAccountListQuery>,
) -> ApiResult<PageResult<WalletBalanceVO>> {
    ok(state.hub.federation.list_wallet_accounts_admin(&q).await?)
}

/// 三期④：中央账本行列表（中枢）。
async fn wallet_lines(
    State(state): State<AdminState>,
    Query(q): Query<HubWalletLineListQuery>,
) -> ApiResult<PageResult<HubWalletLineVO>> {
    ok(state.hub.federation.list_wallet_lines_admin(&q).await?)
}

/// 三期④：中央账本手工记账（中枢：积分发放/余额入账/冲正）。
async fn wallet_post_line(
    State(state): State<AdminState>,
    Json(dto): Json<WalletAdminPostDTO>,
) -> ApiResult<WalletPostVO> {
    ok(state.hub.federation.admin_wallet_post(&dto).await?)
}

/// 三期④：释放过期冻结（中枢）。
async fn wallet_release_expired(State(state): State<AdminState>) -> ApiResult<Value> {
    ok(state.hub.federation.release_expired_freezes().await?)
}

/// 三期④：本机账本镜像列表（成员）。
async fn wallet_mirror(
    State(state): State<AdminState>,
    Query(q): Query<HubWalletMirrorListQuery>,
) -> ApiResult<PageResult<HubWalletMirrorVO>> {
    ok(state.hub.federation.list_wallet_mirrors_admin(&q).await?)
}

/// 三期④：存量并账（成员，按连接把已映射会员本地积分/余额并入中枢账本）。
async fn my_link_merge_wallet(
    State(state): State<AdminState>,
    Json(dto): Json<HubIdDTO>,
) -> ApiResult<Value> {
    ok(state.hub.federation.merge_wallet(dto.id).await?)
}

/// 三期⑤：预付账户列表。
async fn settle_accounts(
    State(state): State<AdminState>,
) -> ApiResult<PageResult<HubSettleAccountVO>> {
    ok(state.hub.federation.list_settle_accounts_admin().await?)
}

/// 三期⑤：预付账户开户。
async fn settle_account_open(
    State(state): State<AdminState>,
    Json(dto): Json<SettleAccountOpenDTO>,
) -> ApiResult<HubSettleAccountVO> {
    ok(state.hub.federation.open_settle_account_admin(&dto).await?)
}

/// 三期⑤：预付账户充值登记（只记账）。
async fn settle_account_adjust(
    State(state): State<AdminState>,
    Json(dto): Json<SettleAccountAdjustDTO>,
) -> ApiResult<Value> {
    ok(state.hub.federation.adjust_settle_account_admin(&dto).await?)
}

/// 三期⑤：账目行列表。
async fn settle_lines(
    State(state): State<AdminState>,
    Query(q): Query<HubSettleLineListQuery>,
) -> ApiResult<PageResult<HubSettleLineVO>> {
    ok(state.hub.federation.list_settle_lines_admin(&q).await?)
}

/// 三期⑤：账目行差异挂起/恢复。
async fn settle_set_line_status(
    State(state): State<AdminState>,
    Json(dto): Json<SettleLineStatusDTO>,
) -> ApiResult<&'static str> {
    state.hub.federation.set_settle_line_status(&dto).await?;
    ok("行状态已更新")
}

/// 三期⑤：未定费率行重算（body.id = 连接 ID）。
async fn settle_reprice(
    State(state): State<AdminState>,
    Json(dto): Json<HubIdDTO>,
) -> ApiResult<Value> {
    ok(state.hub.federation.reprice_settle_lines(dto.id).await?)
}

/// 三期⑤：结算单列表。
async fn settle_statements(
    State(state): State<AdminState>,
    Query(q): Query<HubSettleStatementListQuery>,
) -> ApiResult<PageResult<HubSettleStatementVO>> {
    ok(state.hub.federation.list_settle_statements_admin(&q).await?)
}

/// 三期⑤：生成结算单。
async fn settle_generate(
    State(state): State<AdminState>,
    Json(dto): Json<SettleGenerateDTO>,
) -> ApiResult<HubSettleStatementVO> {
    ok(state.hub.federation.generate_statement(&dto).await?)
}

/// 三期⑤：本方确认结算单（body.id = 结算单 ID）。
async fn settle_confirm(
    State(state): State<AdminState>,
    Json(dto): Json<HubIdDTO>,
) -> ApiResult<HubSettleStatementVO> {
    ok(state.hub.federation.confirm_statement_admin(dto.id).await?)
}

/// 三期⑤：结清登记（只记账，不动资金）。
async fn settle_mark_settled(
    State(state): State<AdminState>,
    Json(dto): Json<HubIdDTO>,
) -> ApiResult<HubSettleStatementVO> {
    ok(state.hub.federation.mark_statement_settled(dto.id).await?)
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
