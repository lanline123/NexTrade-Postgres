//! 商城前台页面装修域路由。
//! 对应原 Java `controller.decorate.discrete.DecorateDiscreteController`、
//! `controller.home.HomeController#index/#pcIndex`。
//!
//! > 注意：`/home/home/*` 路径在 Java 侧属 `HomeController`（装修数据出口），
//! > 本域只注册装修数据相关的 `index` / `pcIndex` 两个端点；`friendLinks`、
//! > `mobileNav`、`pcLogin` 等其余端点由 home 域实现者注册，避免路由重复。
//!
//! > 迁移记录：`GET /api/home/share/import`（`ShareController#import`）曾在此
//! > 作为引导期内联实现，现已由 **content::share** 正式实现
//! > （`crates/api/public/src/routes/share.rs`），故从本文件移除，
//! > 避免 axum `Router::merge` 因同一 `(method, path)` 重复注册而启动 panic。

use axum::{
    extract::{Query, State},
    routing::get,
    Router,
};
use serde::Deserialize;
use nextrade_core::models::content::decorate::{DecorateModuleListVO, DecorateVO, HomeDecorateQuery};
use nextrade_security::web::{ok, ApiResult};

use crate::PublicState;

/// 公开路由：装修组件、首页装修数据与分享导入（均免登录可访问）。
pub fn public_routes() -> Router<PublicState> {
    Router::new()
        .route(
            "/decorate/discrete/getOpenAdvertising",
            get(open_advertising),
        )
        .route("/home/home/index", get(home_index))
        .route("/home/home/pcIndex", get(pc_index))
}

/// 需登录路由：暂无（装修数据全部为公开读取）。
pub fn authorized_routes() -> Router<PublicState> {
    Router::new()
}

/// 本域已实现路由（用于覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    [
        (
            "GET",
            "/api/decorate/discrete/getOpenAdvertising",
            "DecorateDiscreteController.getOpenAdvertising",
        ),
        ("GET", "/api/home/home/index", "HomeController.index"),
        ("GET", "/api/home/home/pcIndex", "HomeController.pcIndex"),
    ]
    .iter()
    .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, p, j))
    .collect()
}

/// `GET /api/decorate/discrete/getOpenAdvertising`：开屏广告装修组件。
async fn open_advertising(
    State(state): State<PublicState>,
) -> ApiResult<serde_json::Value> {
    ok(state.content.decorate.open_advertising().await?)
}

/// `GET /api/home/home/index?previewId=`：H5 商城首页装修。
async fn home_index(
    State(state): State<PublicState>,
    Query(q): Query<HomeDecorateQuery>,
) -> ApiResult<nextrade_core::models::content::decorate::DecorateModuleListVO> {
    ok(state.content.decorate.home_index(&q).await?)
}

/// `GET /api/home/home/pcIndex?previewId&id&decorateType`：PC 商城首页装修。
async fn pc_index(
    State(state): State<PublicState>,
    Query(q): Query<HomeDecorateQuery>,
) -> ApiResult<nextrade_core::models::content::decorate::DecorateModuleListVO> {
    ok(state.content.decorate.pc_index(&q).await?)
}