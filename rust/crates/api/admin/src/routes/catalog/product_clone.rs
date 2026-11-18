//! 管理端商品复刻路由（catalog::product_clone，2026-11-18 新增，本平台自有契约）。
//!
//! 设计：`~/workspace/product-clone-design/proposal.md` + `channel-design.md`。
//! 双通道（A 万邦数据接口 / B 自建抓取边车）在服务层归一，前端只面对任务与草稿。
//!
//! | 路由 | 说明 |
//! |------|------|
//! | `POST /product/productClone/fetch` | 粘贴链接批量建解析任务（后台执行） |
//! | `GET  /product/productClone/tasks` | 复刻记录列表 |
//! | `GET  /product/productClone/taskDetail` | 任务详情（含归一草稿，供预览修正） |
//! | `POST /product/productClone/retry` | 换通道重试 |
//! | `POST /product/productClone/import` | 确认导入（转存+落库） |
//! | `GET  /product/productClone/channels` | 通道矩阵+健康+默认值 |
//! | `GET  /setting/config/productCloneSettings` | 复刻设置（密钥脱敏） |
//! | `POST /setting/config/saveProductCloneSettings` | 保存复刻设置 |

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
use serde_json::Value;

use nextrade_core::models::catalog::product_clone::{
    CloneFetchDTO, CloneImportDTO, CloneRetryDTO, CloneTaskQuery,
};
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::AdminState;

const ROUTE_TABLE: [(&str, &str, &str); 8] = [
    ("POST", "/product/productClone/fetch", "ProductCloneController.fetch"),
    ("GET", "/product/productClone/tasks", "ProductCloneController.tasks"),
    ("GET", "/product/productClone/taskDetail", "ProductCloneController.taskDetail"),
    ("POST", "/product/productClone/retry", "ProductCloneController.retry"),
    ("POST", "/product/productClone/import", "ProductCloneController.import"),
    ("GET", "/product/productClone/channels", "ProductCloneController.channels"),
    ("GET", "/setting/config/productCloneSettings", "ProductCloneController.getSettings"),
    ("POST", "/setting/config/saveProductCloneSettings", "ProductCloneController.saveSettings"),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/product/productClone/fetch", post(fetch))
        .route("/product/productClone/tasks", get(tasks))
        .route("/product/productClone/taskDetail", get(task_detail))
        .route("/product/productClone/retry", post(retry))
        .route("/product/productClone/import", post(import))
        .route("/product/productClone/channels", get(channels))
        .route("/setting/config/productCloneSettings", get(get_settings))
        .route("/setting/config/saveProductCloneSettings", post(save_settings))
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/adminapi{path}"), handler))
        .collect()
}

async fn fetch(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<CloneFetchDTO>,
) -> ApiResult<Vec<Value>> {
    ok(state
        .catalog
        .product_clone
        .create_fetch_tasks(&dto, &auth.username)
        .await?)
}

async fn tasks(
    State(state): State<AdminState>,
    Query(q): Query<CloneTaskQuery>,
) -> ApiResult<nextrade_core::types::PageResult<Value>> {
    ok(state.catalog.product_clone.list_tasks(&q).await?)
}

#[derive(Debug, Deserialize)]
struct IdQuery {
    id: i64,
}

async fn task_detail(
    State(state): State<AdminState>,
    Query(q): Query<IdQuery>,
) -> ApiResult<Value> {
    ok(state.catalog.product_clone.task_detail(q.id).await?)
}

async fn retry(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<CloneRetryDTO>,
) -> ApiResult<()> {
    state
        .catalog
        .product_clone
        .retry_task(&dto, &auth.username)
        .await?;
    ok(())
}

async fn import(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<CloneImportDTO>,
) -> ApiResult<nextrade_core::models::catalog::product_clone::CloneImportResult> {
    ok(state
        .catalog
        .product_clone
        .import_task(&dto, auth.shop_id)
        .await?)
}

async fn channels(State(state): State<AdminState>) -> ApiResult<Value> {
    ok(state.catalog.product_clone.channels_info().await?)
}

async fn get_settings(State(state): State<AdminState>) -> ApiResult<Value> {
    ok(state.catalog.product_clone.get_settings().await?)
}

async fn save_settings(
    State(state): State<AdminState>,
    Json(payload): Json<Value>,
) -> ApiResult<Value> {
    ok(state.catalog.product_clone.save_settings(&payload).await?)
}
