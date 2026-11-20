//! 商品文件型交付 —— 管理端路由（catalog::product_delivery，我方新增）。
//!
//! 本域为新功能，无 Java 对应控制器，路径自拟（挂载于 `/adminapi`）：
//!
//! | method | path | 说明 |
//! |--------|------|------|
//! | `GET`  | `/product/product/deliveryFiles?productId=` | 商品附件列表 |
//! | `POST` | `/product/product/uploadDeliveryFile` | 上传附件（multipart：productId + file） |
//! | `POST` | `/product/product/delDeliveryFile` | 删除附件（软删） |
//!
//! 权限：全局 `require_auth` 校验登录态；商品归属作用域在服务层强校验
//! （商家只能管本店商品，平台商品仅平台）。

use axum::{
    extract::{DefaultBodyLimit, Multipart, Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;

use nextrade_core::models::catalog::product_delivery::{
    DeliveryFileDeleteDTO, DeliveryFileVO, MAX_DELIVERY_FILE_SIZE,
};
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};

use crate::AdminState;

/// 本域路由（去掉 `/adminapi` 前缀）。
const ROUTE_TABLE: [(&str, &str, &str); 3] = [
    (
        "GET",
        "/product/product/deliveryFiles",
        "ProductDeliveryController.list",
    ),
    (
        "POST",
        "/product/product/uploadDeliveryFile",
        "ProductDeliveryController.upload",
    ),
    (
        "POST",
        "/product/product/delDeliveryFile",
        "ProductDeliveryController.del",
    ),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/product/product/deliveryFiles", get(list))
        .route(
            "/product/product/uploadDeliveryFile",
            post(upload).layer(DefaultBodyLimit::max(
                (MAX_DELIVERY_FILE_SIZE + 16 * 1024 * 1024) as usize,
            )),
        )
        .route("/product/product/delDeliveryFile", post(del))
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, java_ref)| {
            nextrade_openapi::RouteDoc::new(method, &format!("/adminapi{path}"), java_ref)
        })
        .collect()
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ProductIdQuery {
    #[serde(rename = "productId")]
    product_id: Option<i32>,
}

/// `GET /adminapi/product/product/deliveryFiles?productId=`：附件列表。
async fn list(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<ProductIdQuery>,
) -> ApiResult<Vec<DeliveryFileVO>> {
    let product_id = q
        .product_id
        .filter(|v| *v > 0)
        .ok_or_else(|| AppError(nextrade_core::Error::Validation("商品参数缺失".into())))?;
    ok(state
        .catalog
        .product_delivery
        .admin_list(product_id, auth.scoped_shop_id())
        .await?)
}

/// `POST /adminapi/product/product/uploadDeliveryFile`：上传附件
/// （multipart 字段 `productId` + `file`；大小/类型校验在服务层）。
async fn upload(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    mut multipart: Multipart,
) -> ApiResult<DeliveryFileVO> {
    let mut product_id: Option<i32> = None;
    let mut payload: Option<(String, Vec<u8>)> = None;
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError(nextrade_core::Error::Validation(format!(
            "上传解析失败: {e}"
        )))
    })? {
        match field.name() {
            Some("productId") => {
                let text = field.text().await.unwrap_or_default();
                product_id = text.trim().parse::<i32>().ok();
            }
            Some("file") => {
                let file_name = field.file_name().map(ToString::to_string).unwrap_or_default();
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| {
                        AppError(nextrade_core::Error::Validation(format!(
                            "文件读取失败: {e}"
                        )))
                    })?
                    .to_vec();
                payload = Some((file_name, bytes));
            }
            _ => {}
        }
    }
    let product_id = product_id
        .filter(|v| *v > 0)
        .ok_or_else(|| AppError(nextrade_core::Error::Validation("商品参数缺失".into())))?;
    let (file_name, bytes) = payload
        .filter(|(_, b)| !b.is_empty())
        .ok_or_else(|| AppError(nextrade_core::Error::Validation("文件不能为空".into())))?;
    ok(state
        .catalog
        .product_delivery
        .admin_upload(product_id, auth.scoped_shop_id(), &file_name, bytes)
        .await?)
}

/// `POST /adminapi/product/product/delDeliveryFile`：删除附件（软删）。
async fn del(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<DeliveryFileDeleteDTO>,
) -> ApiResult<()> {
    state
        .catalog
        .product_delivery
        .admin_delete(dto.id, auth.scoped_shop_id())
        .await?;
    ok(())
}
