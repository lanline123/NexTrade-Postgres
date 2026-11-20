//! 管理端店铺标签路由（merchant::shop_label）—— **本平台新增**（论坛帖
//! 207878955346177229），原版 tigshop 无对应控制器（官方称功能尚在设计中）。
//!
//! | 本模块路由（挂载于 `/adminapi`） | 说明 | HTTP |
//! |----------------------------------|------|------|
//! | `GET  /merchant/shopLabel/list` | 标签库全量（含停用） | GET |
//! | `POST /merchant/shopLabel/create` | 新建标签 | POST |
//! | `POST /merchant/shopLabel/update` | 编辑标签（名称/颜色/排序/启停） | POST |
//! | `POST /merchant/shopLabel/del` | 删除标签（连同分配关系） | POST |
//! | `GET  /merchant/shopLabel/assignments` | 全量店铺↔标签分配（店铺列表渲染徽章） | GET |
//! | `GET  /merchant/shopLabel/shopLabels?shopId=` | 单店已分配标签 id（回显） | GET |
//! | `POST /merchant/shopLabel/setShopLabels` | 整店替换式分配（平台打标） | POST |
//!
//! > **权限口径**：标签库与打标均为平台职能，全部端点要求平台账号
//! > （`TenantContext::is_platform`），商家/供应商账号调用返回 403。
//! > 与既有 `store_tip`（门店服务提示）是两套概念，互不影响。
//! >
//! > 迁移：`rust/deploy/pg/2026-11-21_shop_label.sql` 须先于本代码部署执行。

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;

use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, TenantContext};
use nextrade_services::merchant::shop_label::{
    ShopLabelAssignment, ShopLabelItem, ShopLabelWrite,
};

use crate::AdminState;

/// 本域全部 7 条路由（去掉 `/adminapi` 前缀后的路径）。
const ROUTE_TABLE: [(&str, &str, &str); 7] = [
    ("GET", "/merchant/shopLabel/list", "ShopLabelController.list (本平台新增)"),
    ("POST", "/merchant/shopLabel/create", "ShopLabelController.create (本平台新增)"),
    ("POST", "/merchant/shopLabel/update", "ShopLabelController.update (本平台新增)"),
    ("POST", "/merchant/shopLabel/del", "ShopLabelController.del (本平台新增)"),
    (
        "GET",
        "/merchant/shopLabel/assignments",
        "ShopLabelController.assignments (本平台新增)",
    ),
    (
        "GET",
        "/merchant/shopLabel/shopLabels",
        "ShopLabelController.shopLabels (本平台新增)",
    ),
    (
        "POST",
        "/merchant/shopLabel/setShopLabels",
        "ShopLabelController.setShopLabels (本平台新增)",
    ),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/merchant/shopLabel/list", get(list))
        .route("/merchant/shopLabel/create", post(create))
        .route("/merchant/shopLabel/update", post(update))
        .route("/merchant/shopLabel/del", post(del))
        .route("/merchant/shopLabel/assignments", get(assignments))
        .route("/merchant/shopLabel/shopLabels", get(shop_labels))
        .route("/merchant/shopLabel/setShopLabels", post(set_shop_labels))
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

/// 平台职能守卫：非平台账号（商家/供应商/缺失作用域）一律 403。
fn require_platform(tenant: &TenantContext) -> Result<(), Error> {
    if tenant.is_platform() {
        Ok(())
    } else {
        Err(Error::Forbidden("店铺标签仅平台管理员可管理".into()))
    }
}

async fn list(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
) -> ApiResult<Vec<ShopLabelItem>> {
    require_platform(&tenant)?;
    ok(state.merchant.shop_label.list_labels().await?)
}

async fn create(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<ShopLabelWrite>,
) -> ApiResult<i32> {
    require_platform(&tenant)?;
    ok(state.merchant.shop_label.create_label(&body).await?)
}

async fn update(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<ShopLabelWrite>,
) -> ApiResult<bool> {
    require_platform(&tenant)?;
    state.merchant.shop_label.update_label(&body).await?;
    ok(true)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LabelIdBody {
    label_id: Option<i32>,
}

async fn del(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<LabelIdBody>,
) -> ApiResult<bool> {
    require_platform(&tenant)?;
    let label_id = body.label_id.unwrap_or(0);
    state.merchant.shop_label.delete_label(label_id).await?;
    ok(true)
}

async fn assignments(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
) -> ApiResult<Vec<ShopLabelAssignment>> {
    require_platform(&tenant)?;
    ok(state.merchant.shop_label.all_assignments().await?)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShopIdQuery {
    shop_id: Option<i32>,
}

async fn shop_labels(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Query(q): Query<ShopIdQuery>,
) -> ApiResult<Vec<i32>> {
    require_platform(&tenant)?;
    let shop_id = q.shop_id.unwrap_or(0);
    ok(state.merchant.shop_label.shop_label_ids(shop_id).await?)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetShopLabelsBody {
    shop_id: Option<i32>,
    label_ids: Option<Vec<i32>>,
}

async fn set_shop_labels(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<SetShopLabelsBody>,
) -> ApiResult<bool> {
    require_platform(&tenant)?;
    let shop_id = body.shop_id.unwrap_or(0);
    let label_ids = body.label_ids.unwrap_or_default();
    state
        .merchant
        .shop_label
        .set_shop_labels(shop_id, &label_ids)
        .await?;
    ok(true)
}
