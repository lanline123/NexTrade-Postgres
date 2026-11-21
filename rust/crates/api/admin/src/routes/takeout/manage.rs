//! 外卖聚合管理接口（takeout::manage，我方新增）。
//!
//! | 路由（挂载于 `/adminapi`） | 说明 |
//! |----------------------------------|------|
//! | `GET  /setting/takeout/overview` | 渠道状态概览（脱敏）+ 绑定/待绑定计数 |
//! | `POST /setting/takeout/config/save` | 保存单渠道配置（仅平台；密钥空 = 不改） |
//! | `GET  /setting/takeout/bindings` | 门店绑定列表（令牌脱敏） |
//! | `POST /setting/takeout/bindings/save` | 新增/更新绑定 |
//! | `POST /setting/takeout/bindings/delete` | 删除绑定 |
//! | `GET  /setting/takeout/mappings` | SKU 映射列表 |
//! | `POST /setting/takeout/mappings/save` | 新增/更新映射 |
//! | `POST /setting/takeout/mappings/delete` | 删除映射 |
//! | `GET  /setting/takeout/orders` | 外部订单分页列表 |
//! | `GET  /setting/takeout/orders/detail` | 外部订单详情（含归一载荷） |
//! | `POST /setting/takeout/orders/confirm` | 手动确认接单（真实调用渠道 API） |
//! | `POST /setting/takeout/orders/logistics` | 同步自配送物流状态 |
//! | `POST /setting/takeout/orders/refund` | 退款应答（同意/驳回，执行在渠道侧） |
//! | `POST /setting/takeout/orders/reprocess` | 重新处理（补绑/补映射后补单） |
//! | `POST /setting/takeout/douyin/verify` | 抖音团购券手工输码核销（走既有核销链） |

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;

use nextrade_core::models::takeout::{
    BindingSaveDTO, ChannelConfigSaveDTO, ExternalOrderListQuery, ExternalOrderVO, IdDTO,
    MappingSaveDTO,
};
use nextrade_core::types::PageResult;
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult};

use crate::AdminState;

/// 本模块路由表（路径 + 处理名，覆盖率统计用）。
const ROUTE_TABLE: [(&str, &str, &str); 15] = [
    ("GET", "/setting/takeout/overview", "TakeoutController.overview"),
    ("POST", "/setting/takeout/config/save", "TakeoutController.saveConfig"),
    ("GET", "/setting/takeout/bindings", "TakeoutController.bindings"),
    ("POST", "/setting/takeout/bindings/save", "TakeoutController.saveBinding"),
    ("POST", "/setting/takeout/bindings/delete", "TakeoutController.deleteBinding"),
    ("GET", "/setting/takeout/mappings", "TakeoutController.mappings"),
    ("POST", "/setting/takeout/mappings/save", "TakeoutController.saveMapping"),
    ("POST", "/setting/takeout/mappings/delete", "TakeoutController.deleteMapping"),
    ("GET", "/setting/takeout/orders", "TakeoutController.orders"),
    ("GET", "/setting/takeout/orders/detail", "TakeoutController.orderDetail"),
    ("POST", "/setting/takeout/orders/confirm", "TakeoutController.confirmOrder"),
    ("POST", "/setting/takeout/orders/logistics", "TakeoutController.syncLogistics"),
    ("POST", "/setting/takeout/orders/refund", "TakeoutController.respondRefund"),
    ("POST", "/setting/takeout/orders/reprocess", "TakeoutController.reprocess"),
    ("POST", "/setting/takeout/douyin/verify", "TakeoutController.verifyDouyin"),
];

/// 本域路由汇总。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/setting/takeout/overview", get(overview))
        .route("/setting/takeout/config/save", post(save_config))
        .route("/setting/takeout/bindings", get(bindings))
        .route("/setting/takeout/bindings/save", post(save_binding))
        .route("/setting/takeout/bindings/delete", post(delete_binding))
        .route("/setting/takeout/mappings", get(mappings))
        .route("/setting/takeout/mappings/save", post(save_mapping))
        .route("/setting/takeout/mappings/delete", post(delete_mapping))
        .route("/setting/takeout/orders", get(orders))
        .route("/setting/takeout/orders/detail", get(order_detail))
        .route("/setting/takeout/orders/confirm", post(confirm_order))
        .route("/setting/takeout/orders/logistics", post(sync_logistics))
        .route("/setting/takeout/orders/refund", post(respond_refund))
        .route("/setting/takeout/orders/reprocess", post(reprocess))
        .route("/setting/takeout/douyin/verify", post(verify_douyin))
}

/// 本模块已实现路由（用于 `/openapi/coverage` 覆盖率统计，完整契约路径）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/adminapi{path}"), handler))
        .collect()
}

/// 渠道过滤查询。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelQuery {
    /// 渠道 id（空 = 全部）。
    pub channel: Option<String>,
}

/// 详情查询。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailQuery {
    /// 外部单 id。
    pub id: i32,
}

/// 物流同步入参。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogisticsDTO {
    /// 外部单 id。
    pub id: i32,
    /// 渠道口径物流状态码。
    pub logistics_status: i32,
}

/// 退款应答入参。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundDTO {
    /// 外部单 id。
    pub id: i32,
    /// 渠道退款单号（可空）。
    pub refund_id: String,
    /// 是否同意。
    pub agree: bool,
    /// 驳回原因（驳回时必填口径在渠道侧校验）。
    pub reason: String,
}

/// 抖音券码核销入参。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DouyinVerifyDTO {
    /// 门店绑定 id（须为抖音渠道绑定）。
    pub binding_id: i32,
    /// 顾客出示的明文券码。
    pub code: String,
}

async fn overview(State(state): State<AdminState>) -> ApiResult<Value> {
    ok(state.takeout.admin.overview().await?)
}

async fn save_config(
    State(state): State<AdminState>,
    Json(dto): Json<ChannelConfigSaveDTO>,
) -> ApiResult<Value> {
    ok(state.takeout.admin.save_config(&dto).await?)
}

async fn bindings(
    State(state): State<AdminState>,
    Query(q): Query<ChannelQuery>,
) -> ApiResult<Value> {
    ok(state.takeout.admin.list_bindings(q.channel.as_deref()).await?)
}

async fn save_binding(
    State(state): State<AdminState>,
    Json(dto): Json<BindingSaveDTO>,
) -> ApiResult<i32> {
    ok(state.takeout.admin.save_binding(&dto).await?)
}

async fn delete_binding(
    State(state): State<AdminState>,
    Json(dto): Json<IdDTO>,
) -> ApiResult<()> {
    state.takeout.admin.delete_binding(&dto).await?;
    ok(())
}

async fn mappings(
    State(state): State<AdminState>,
    Query(q): Query<ChannelQuery>,
) -> ApiResult<Value> {
    ok(state.takeout.admin.list_mappings(q.channel.as_deref()).await?)
}

async fn save_mapping(
    State(state): State<AdminState>,
    Json(dto): Json<MappingSaveDTO>,
) -> ApiResult<i32> {
    ok(state.takeout.admin.save_mapping(&dto).await?)
}

async fn delete_mapping(
    State(state): State<AdminState>,
    Json(dto): Json<IdDTO>,
) -> ApiResult<()> {
    state.takeout.admin.delete_mapping(&dto).await?;
    ok(())
}

async fn orders(
    State(state): State<AdminState>,
    Query(q): Query<ExternalOrderListQuery>,
) -> ApiResult<PageResult<ExternalOrderVO>> {
    ok(state.takeout.admin.list_orders(&q).await?)
}

async fn order_detail(
    State(state): State<AdminState>,
    Query(q): Query<DetailQuery>,
) -> ApiResult<ExternalOrderVO> {
    ok(state.takeout.admin.order_detail(q.id).await?)
}

async fn confirm_order(
    State(state): State<AdminState>,
    Json(dto): Json<IdDTO>,
) -> ApiResult<()> {
    state.takeout.admin.confirm_order(dto.id).await?;
    ok(())
}

async fn sync_logistics(
    State(state): State<AdminState>,
    Json(dto): Json<LogisticsDTO>,
) -> ApiResult<()> {
    state
        .takeout
        .admin
        .sync_logistics(dto.id, dto.logistics_status)
        .await?;
    ok(())
}

async fn respond_refund(
    State(state): State<AdminState>,
    Json(dto): Json<RefundDTO>,
) -> ApiResult<()> {
    state
        .takeout
        .admin
        .respond_refund(dto.id, &dto.refund_id, dto.agree, &dto.reason)
        .await?;
    ok(())
}

async fn reprocess(
    State(state): State<AdminState>,
    Json(dto): Json<IdDTO>,
) -> ApiResult<()> {
    state.takeout.admin.reprocess(dto.id).await?;
    ok(())
}

async fn verify_douyin(
    State(state): State<AdminState>,
    Json(dto): Json<DouyinVerifyDTO>,
) -> ApiResult<Value> {
    ok(state
        .takeout
        .admin
        .verify_douyin_certificate(dto.binding_id, &dto.code)
        .await?)
}
