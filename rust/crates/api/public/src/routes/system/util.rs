//! 通用工具路由（system::util，**商城端**）。
//!
//! 原 Java `UtilController`
//! （`java/nextrade-api/.../controller/common/UtilController.java`，
//! `@RequestMapping("/api/common/util")`），3 个 GET，契约 `auth=optional`，
//! 另加我方新增 `shareLink`（分享直达小程序，论坛帖 907763）：
//!
//! | 本模块路由（挂载于 `/api`） | 契约 auth | Java 实现 |
//! |-----------------------------|-----------|-----------|
//! | `GET /common/util/qrCode?url=` | optional | `QrCodeUtil.generateAsBase64` |
//! | `GET /common/util/miniCode?path=&id=&salesmanId=&shopId=` | optional | `WechatOAuthService.getMiniCode` |
//! | `GET /common/util/miniScene?scene=` | optional | `WechatOAuthService.resolveMiniScene` |
//! | `GET /common/util/shareLink?path=&id=&salesmanId=&shopId=` | optional | 无（我方新增） |
//!
//! > 小程序分享口径：`miniCode`/`shareLink` 以小程序凭据真调微信；
//! > 未配置/失败时如实降级 H5（`shareLink` 以 `degraded/notice` 明示，
//! > `miniCode` 回退 H5 链接二维码），不伪造成功。详见服务层模块头。
//!
//! > **路由归属**：spec 路径为 `/api/...`，故本文件位于
//! > `api/public/src/routes/system/util.rs`（而非 admin 路由树）；这是与
//! > write-scope 中 admin 路径的**有意偏差**，已在任务报告登记。
//!
//! > **auth=optional 落地**：自带 [`optional_auth`]（逐行对齐
//! > `content/seo.rs`），必须并入 `merge_public`，外层不得再套 `require_auth`。

use axum::{
    extract::{Query, Request, State},
    http::header,
    middleware::{self, Next},
    response::Response,
    routing::get,
    Router,
};
use nextrade_core::models::system::util::{
    MiniCodeQuery, MiniSceneQuery, QrCodeQuery, ShareLinkVO,
};
use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};
use nextrade_security::JwtService;

use crate::PublicState;

/// 本域 4 条路由（去 `/api` 前缀的路径 + 原 Java handler；shareLink 为我方新增）。
const ROUTE_TABLE: [(&str, &str, &str); 4] = [
    ("GET", "/common/util/qrCode", "UtilController.qrCode"),
    ("GET", "/common/util/miniCode", "UtilController.miniCode"),
    ("GET", "/common/util/miniScene", "UtilController.miniScene"),
    ("GET", "/common/util/shareLink", "UtilController.shareLink(我方新增)"),
];

/// 通用工具路由（挂载于 `/api` 下，自带可选登录态中间件）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/common/util/qrCode", get(qr_code))
        .route("/common/util/miniCode", get(mini_code))
        .route("/common/util/miniScene", get(mini_scene))
        .route("/common/util/shareLink", get(share_link))
        .layer(middleware::from_fn_with_state(state.jwt.clone(), optional_auth))
}

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, java_ref)| {
            nextrade_openapi::RouteDoc::new(method, &format!("/api{path}"), java_ref)
        })
        .collect()
}

/// `GET /api/common/util/qrCode?url=`：二维码（本地生成）。
async fn qr_code(
    State(state): State<PublicState>,
    Query(q): Query<QrCodeQuery>,
) -> ApiResult<String> {
    ok(state.system.util.qr_code(q.url.as_deref()).await?)
}

/// `GET /api/common/util/miniCode?path=&id=`：小程序码图片 data URL。
///
/// 说明：Java 该方法直接返回 `String`（非 `AjaxResult` 包装）；Rust 统一走
/// [`ApiResult`] 信封（与其余已迁移端点一致的平台口径），偏差已登记。
/// 未配置小程序/微信失败时降级为 H5 链接二维码（标注走 `shareLink`）。
async fn mini_code(
    State(state): State<PublicState>,
    Query(q): Query<MiniCodeQuery>,
) -> ApiResult<String> {
    ok(state
        .system
        .util
        .mini_code(q.path.as_deref(), q.id, q.salesman_id, q.shop_id)
        .await?)
}

/// `GET /api/common/util/shareLink?path=&id=`：分享直达小程序（我方新增）。
///
/// 结构化返回 URL Link + 小程序码 + H5 链接；降级时 `channel=h5`、
/// `degraded=true`、`notice` 写明原因，不伪造成功。
async fn share_link(
    State(state): State<PublicState>,
    Query(q): Query<MiniCodeQuery>,
) -> ApiResult<ShareLinkVO> {
    ok(state
        .system
        .util
        .share_link(q.path.as_deref(), q.id, q.salesman_id, q.shop_id)
        .await?)
}

/// `GET /api/common/util/miniScene?scene=`：解析小程序码 scene（query 直解 + 短码查映射）。
async fn mini_scene(
    State(state): State<PublicState>,
    Query(q): Query<MiniSceneQuery>,
) -> ApiResult<serde_json::Value> {
    ok(state.system.util.mini_scene(q.scene.as_deref()).await?)
}

/// 可选登录态中间件（对应 Java `permitAll` + JWT 过滤器）。
///
/// 无令牌 → 游客；非法/过期令牌 → 401；合法令牌 → 写入 [`AuthUser`]。
/// 与 `content/seo.rs` / `shop_client.rs` / `wholesale_client.rs` 一致。
async fn optional_auth(
    State(jwt): State<JwtService>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    if token.is_some() {
        let Ok(claims) = jwt.from_authorization(token) else { return Ok(next.run(req).await); };
        let Ok(id) = claims.sub.parse::<i32>() else { return Ok(next.run(req).await); };
        req.extensions_mut().insert(AuthUser {
            id,
            username: claims.usr,
            role_id: claims.role,
            admin_type: claims.typ,
            shop_id: claims.shop_id,
        });
    }
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_table_matches_contract_and_has_api_prefix_in_docs() {
        assert_eq!(ROUTE_TABLE.len(), 4);
        let docs = implemented_routes();
        assert_eq!(docs[0].path, "/api/common/util/qrCode");
        assert_eq!(docs[2].path, "/api/common/util/miniScene");
        assert_eq!(docs[3].path, "/api/common/util/shareLink");
        assert!(ROUTE_TABLE.iter().all(|(m, _, _)| *m == "GET"));
        assert!(ROUTE_TABLE.iter().all(|(_, p, _)| !p.contains("/api")));
    }

    #[test]
    fn all_paths_live_under_common_util() {
        for (_, path, _) in ROUTE_TABLE {
            assert!(path.starts_with("/common/util/"), "{path}");
        }
    }
}
