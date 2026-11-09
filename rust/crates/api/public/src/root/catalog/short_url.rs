//! 短链接跳转 —— 根级路由（catalog::short_url，public 端）。
//!
//! **路径约定**：原 Java `ShortUrlController` 的映射是 `@GetMapping("/s/{code}")`，
//! **不含** `/api` 前缀，真实 URL 为 `<api 应用根>/s/{code}`
//! （见 `crate::root` 模块文档）。因此本文件的路由串直接写 `/s/{code}`，
//! `implemented_routes()` 里的 `RouteDoc.path` 也写 `/s/{code}`。
//!
//! | 本模块路由 | 原 Java | 登录 |
//! |-----------|--------|------|
//! | `GET /s/{code}` | `ShortUrlController.redirect` | 免登录（`/s/**` 在 Java `WhiteListConfig.URL_WHITELIST` 内） |
//!
//! 响应语义与 Java `response.sendRedirect(...)` 一致：**HTTP 302 + Location**，
//! 不走 `ResultWrapper` JSON 包装；仅当短链不存在/域名未配置时返回 JSON 业务错误。

use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};

use crate::PublicState;

/// 根级路由（挂在应用根路径，不在 `/api` 之下）。
pub fn routes(_state: &PublicState) -> Router<PublicState> {
    Router::new().route("/s/{code}", get(redirect))
}

/// 已实现路由清单（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    [
        ("GET", "/s/{code}", "ShortUrlController.redirect"),
    ]
    .into_iter()
    .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, p, j))
    .collect()
}

/// `GET /s/{code}`：短链 302 跳转。
async fn redirect(State(state): State<PublicState>, Path(code): Path<String>) -> Response {
    match state.catalog.short_url.resolve_redirect(&code).await {
        Ok(target) => Redirect::temporary(&target).into_response(),
        Err(e) => nextrade_security::web::AppError(e).into_response(),
    }
}