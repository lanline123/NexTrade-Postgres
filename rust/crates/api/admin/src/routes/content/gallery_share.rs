//! 管理端素材上报审核路由（content::gallery_share，我方新增）。
//!
//! 论坛 B 类缓做项 #35（帖 687807 余子项）「平台与门店相册互通」的
//! 门店 → 平台方向：门店提交本店素材上报、平台审核（通过 = 复制入平台库）。
//! 平台 → 门店方向（下发）走 `gallery/setShared` 与 `galleryVideo/setShared`。
//!
//! | 本模块路由（挂载于 `/adminapi`） | 说明 |
//! |----------------------------------|------|
//! | `POST /setting/galleryShare/submit` | 门店提交上报（仅本店素材） |
//! | `GET  /setting/galleryShare/list` | 上报记录（门店看本店 / 平台看全部） |
//! | `POST /setting/galleryShare/review` | 平台审核（通过复制入平台库 / 驳回） |
//!
//! 身份判定全部走认证中间件写入的 `TenantScope`（服务层），
//! 不信任客户端可伪造的租户头。

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use nextrade_core::models::content::gallery_share::{
    GalleryShareListQuery, GalleryShareReviewDTO, GalleryShareSubmitDTO,
    GalleryShareSubmitResultVO, GalleryShareSubmitVO,
};
use nextrade_core::types::PageResult;
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::AdminState;

/// 本域全部 3 条路由（去掉 `/adminapi` 前缀后的路径 + 处理名）。
const ROUTE_TABLE: [(&str, &str, &str); 3] = [
    (
        "POST",
        "/setting/galleryShare/submit",
        "GalleryShareController.submit",
    ),
    (
        "GET",
        "/setting/galleryShare/list",
        "GalleryShareController.list",
    ),
    (
        "POST",
        "/setting/galleryShare/review",
        "GalleryShareController.review",
    ),
];

/// 素材上报审核路由（挂载于 `/adminapi` 下，外层统一套 `require_auth`）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/setting/galleryShare/submit", post(submit))
        .route("/setting/galleryShare/list", get(list))
        .route("/setting/galleryShare/review", post(review))
}

/// 本模块已实现路由（用于 `/openapi/coverage` 覆盖率统计，完整契约路径）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/adminapi{path}"), handler))
        .collect()
}

/// `POST /adminapi/setting/galleryShare/submit`：门店提交素材上报。
async fn submit(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<GalleryShareSubmitDTO>,
) -> ApiResult<GalleryShareSubmitResultVO> {
    ok(state.content.gallery_share.submit(&dto, auth.id).await?)
}

/// `GET /adminapi/setting/galleryShare/list`：上报记录列表。
async fn list(
    State(state): State<AdminState>,
    Query(q): Query<GalleryShareListQuery>,
) -> ApiResult<PageResult<GalleryShareSubmitVO>> {
    ok(state.content.gallery_share.list(&q).await?)
}

/// `POST /adminapi/setting/galleryShare/review`：平台审核上报。
async fn review(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<GalleryShareReviewDTO>,
) -> ApiResult<()> {
    state.content.gallery_share.review(&dto, auth.id).await?;
    ok(())
}
