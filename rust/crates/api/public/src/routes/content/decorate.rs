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
    http::HeaderMap,
    routing::get,
    Router,
};
use serde::Deserialize;
use nextrade_core::models::content::decorate::HomeDecorateQuery;
use nextrade_security::web::{ok, ApiResult, AppError};

use crate::PublicState;

/// 价格门控（第 2 期补漏）：本域出参是存量装修 JSON，内嵌商品快照价
/// （如商品楼层保存的 `productPrice`）；无看价权时递归把价格键置空，
/// 与 home 域装修类 feed 同一口径（`services::system::price_gate`）。
/// 本域无登录态中间件，手动解析 `Authorization` 头（游客按未登录处理）。
async fn gate_stored(
    state: &PublicState,
    headers: &HeaderMap,
    mut v: serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    let access = state
        .system
        .config_client
        .price_access(crate::optional_user_id(state, headers))
        .await?;
    if !access.can_view() {
        nextrade_services::system::price_gate::mask_price_keys(&mut v);
    }
    Ok(v)
}

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

/// `GET /api/decorate/discrete/getOpenAdvertising`：开屏广告装修组件
/// （存量装修 JSON，价格门控同 index 口径递归脱敏）。
async fn open_advertising(
    State(state): State<PublicState>,
    headers: HeaderMap,
) -> ApiResult<serde_json::Value> {
    let v = state.content.decorate.open_advertising().await?;
    ok(gate_stored(&state, &headers, v).await?)
}

/// 请求头 `X-Locale-Code` → 语言代码（前台语言子页解析，原版
/// `HeaderUtils.getLocaleCode` 同源）；缺失/非 UTF-8 时为 None（默认语言）。
fn locale_code(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-locale-code")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

/// `GET /api/home/home/index?previewId=`：H5 商城首页装修
/// （存量装修 JSON 内嵌商品快照价，价格门控递归脱敏）。
async fn home_index(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Query(q): Query<HomeDecorateQuery>,
) -> ApiResult<serde_json::Value> {
    let vo = state
        .content
        .decorate
        .home_index(&q, locale_code(&headers).as_deref())
        .await?;
    let v = serde_json::to_value(&vo).unwrap_or(serde_json::Value::Null);
    ok(gate_stored(&state, &headers, v).await?)
}

/// `GET /api/home/home/pcIndex?previewId&id&decorateType`：PC 商城首页装修
/// （存量装修 JSON 内嵌商品快照价，价格门控递归脱敏）。
async fn pc_index(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Query(q): Query<HomeDecorateQuery>,
) -> ApiResult<serde_json::Value> {
    let vo = state
        .content
        .decorate
        .pc_index(&q, locale_code(&headers).as_deref())
        .await?;
    let v = serde_json::to_value(&vo).unwrap_or(serde_json::Value::Null);
    ok(gate_stored(&state, &headers, v).await?)
}