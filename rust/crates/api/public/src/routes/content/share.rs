//! 装修分享模板 —— 商城前台路由（content::share）。
//!
//! 原 Java：`ShareController`
//! （`nextrade-api/src/main/java/com/nextrade/api/controller/home/ShareController.java`，
//! `@RequestMapping("/api/home/share")`、`@Tag(name = "装修分享模板")`）。
//!
//! ## 原 Java 路径对照表（本模块挂载于 `/api` 之下）
//!
//! | 本模块路由 | 原 Java 完整路径 | 原 Java 方法 | 契约 auth |
//! |-----------|------------------|----------------|-----------|
//! | `GET /home/share/import` | `GET /api/home/share/import` | `ShareController.getImport` | optional |
//!
//! > **路径说明**：本控制器前缀为 `/api/home/share`，无控制器名重复现象；
//! > 本文件挂在 `/api` 之下，故 `routes()` 内只写 `/home/share/import`
//! > （`RouteDoc` 里写完整契约路径 `/api/home/share/import`）。
//!
//! > **⚠️ 合并注意（必须由主会话处理）**：`crates/api/public/src/routes/decorate.rs`
//! > 的 `public_routes()` 已存在一条**引导阶段的内联实现**
//! > `.route("/home/share/import", get(share_import))`，且
//! > `decorate::implemented_routes()` 里也有对应的
//! > `("GET", "/api/home/share/import", "ShareController.import")` 记录。
//! > 合并本模块时必须**同时删除**那条内联路由与它的记录，否则 axum 的
//! > `Router::merge` 会因同一路径重复注册在启动时 panic
//! > （与 merchant::shop_client 对 `shop.rs` 内联 `/shop/shop/detail` 的处理同因）。
//! >
//! > **auth=optional 的落地口径**：本接口是**免登录**的公开分享页入口
//! > （Java 侧 `permitAll`），签名参数是 `sn` + `token` 而非用户身份。
//! > 本文件不引入 `shop_client` 那套 `optional_auth` 中间件——该接口不读取
//! > 任何登录态，套上中间件只会让「带非法 token 的匿名访客」被 401 拦截，
//! > 与免登录语义冲突。故本模块**无中间件**，直接并入
//! > `routes::merge_public(state)`。

use axum::{
    extract::{Query, State},
    routing::get,
    Router,
};
use nextrade_core::models::content::decorate::DecorateVO;
use nextrade_core::models::content::share::DecorateShareImportQuery;
use nextrade_security::web::{ok, ApiResult};

use crate::PublicState;

/// 装修分享模板域路由（挂载于 `/api` 下，免登录）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new().route("/home/share/import", get(share_import))
}

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    [(
        "GET",
        "/api/home/share/import",
        "ShareController.getImport",
    )]
    .iter()
    .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, p, j))
    .collect()
}

/// `GET /api/home/share/import?sn=&token=`：装修分享导入。
///
/// 对应 Java `ShareController#getImport(@RequestParam("sn") String sn,
///                                     @RequestParam("token") String token)`
/// → `DecorateShareServiceImpl#getInfoBySn(sn, token)`：
/// 校验参数 → 取分享记录 → 返回被分享的装修页详情。
///
/// 原接口返回 `Object`（实际为 `DecorateVO`），本实现显式落地为
/// [`DecorateVO`]，JSON 字段与原 Java 完全一致（camelCase）。
async fn share_import(
    State(state): State<PublicState>,
    Query(q): Query<DecorateShareImportQuery>,
) -> ApiResult<DecorateVO> {
    ok(state.content.share
        .import_by_sn(
            q.sn.as_deref().unwrap_or(""),
            q.token.as_deref().unwrap_or(""),
        )
        .await?)
}