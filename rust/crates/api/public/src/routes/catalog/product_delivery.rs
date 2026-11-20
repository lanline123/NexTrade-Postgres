//! 商品文件型交付 —— 商城端路由（catalog::product_delivery，我方新增）。
//!
//! 本域为新功能，无 Java 对应控制器，路径自拟（挂载于 `/api`）：
//!
//! | method | path | 鉴权 | 说明 |
//! |--------|------|------|------|
//! | `POST` | `/product/delivery/requestDownload` | 登录 | 申领短时效下载令牌（强校验订单归属与交付门） |
//! | `GET`  | `/product/delivery/download?token=` | 令牌即凭证 | 核销令牌后放行下载（OSS 签名 URL 跳转 / 本地直读） |
//!
//! 下载端点匿名开放是刻意设计：令牌不可猜、绑订单 + 用户 + 文件、
//! 30 分钟过期且最多下载 5 次，申领环节已完成全部身份与资格校验。

use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;

use nextrade_core::models::catalog::product_delivery::{DownloadGrantVO, RequestDownloadDTO};
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};
use nextrade_services::catalog::product_delivery::DeliveryDownload;

use crate::PublicState;

/// 本域路由（去掉 `/api` 前缀）。
const ROUTE_TABLE: [(&str, &str, &str); 2] = [
    (
        "POST",
        "/product/delivery/requestDownload",
        "ProductDeliveryClientController.requestDownload",
    ),
    (
        "GET",
        "/product/delivery/download",
        "ProductDeliveryClientController.download",
    ),
];

/// 需登录路由（自带 require_auth，并入 merge_public）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/product/delivery/requestDownload", post(request_download))
        .layer(axum::middleware::from_fn_with_state(
            state.jwt.clone(),
            nextrade_security::web::require_auth,
        ))
}

/// 匿名路由（令牌即凭证）。
pub fn anonymous_routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new().route("/product/delivery/download", get(download))
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, java_ref)| {
            nextrade_openapi::RouteDoc::new(method, &format!("/api{path}"), java_ref)
        })
        .collect()
}

/// `POST /api/product/delivery/requestDownload`：申领下载令牌。
async fn request_download(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<RequestDownloadDTO>,
) -> ApiResult<DownloadGrantVO> {
    ok(state
        .catalog
        .product_delivery
        .request_download(auth.id, &dto)
        .await?)
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct DownloadQuery {
    token: Option<String>,
}

/// `GET /api/product/delivery/download?token=`：凭令牌下载。
async fn download(
    State(state): State<PublicState>,
    Query(q): Query<DownloadQuery>,
) -> Result<Response, AppError> {
    let token = q.token.unwrap_or_default();
    match state.catalog.product_delivery.download(&token).await? {
        DeliveryDownload::Redirect(url) => Ok(Redirect::temporary(&url).into_response()),
        DeliveryDownload::Bytes {
            bytes,
            file_name,
            content_type,
        } => {
            let disposition = format!(
                "attachment; filename*=UTF-8''{}",
                header_encode(&file_name)
            );
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CONTENT_DISPOSITION, disposition)
                .header(header::CONTENT_LENGTH, bytes.len())
                .body(axum::body::Body::from(bytes))
                .map_err(|e| {
                    AppError(nextrade_core::Error::Internal(format!(
                        "下载响应构建失败: {e}"
                    )))
                })?)
        }
    }
}

/// RFC 5987 文件名百分号编码（UTF-8 字节，非保留字符直通）。
fn header_encode(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for b in raw.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'!' | b'#'
            | b'$' | b'&' | b'+' | b'^' | b'`' | b'|' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
