//! 商城端用户收货地址路由（member::user_address_client）。
//!
//! 对应原 Java `com.nextrade.api.controller.user.UserAddressClientController`
//! （`@RequestMapping("/api/user/address")`，7 个端点）。
//!
//! | 本模块路由（挂载于 `/api`） | 原 Java |
//! |-----------------------------|---------|
//! | `GET  /user/address/tagList`     | `UserAddressClientController.tagList` |
//! | `GET  /user/address/list`        | `UserAddressClientController.list` |
//! | `GET  /user/address/detail`      | `UserAddressClientController.detail` |
//! | `POST /user/address/del`         | `UserAddressClientController.del` |
//! | `POST /user/address/create`      | `UserAddressClientController.create` |
//! | `POST /user/address/update`      | `UserAddressClientController.update` |
//! | `POST /user/address/setSelected` | `UserAddressClientController.setSelected` |
//!
//! ## 鉴权
//! 契约 7 条均为 `optional`。本文件自带 [`optional_auth`]（同
//! `routes/content/seo.rs`）。Java 控制器在 `detail` / `del` / `update` 内部
//! 校验登录态（匿名 →「用户未登录」），本实现同样在 handler 层用
//! [`UserAddressClientService::ensure_logged_in`] 拦截；`tagList` / `list` /
//! `create` / `setSelected` 不拦截（与原实现一致，游客按 `user_id = 0` 处理）。
//!
//! ## 合并注意
//! 必须并入 `crates/api/public/src/routes/member/mod.rs` 的 `merge_public` 链
//! （外层**不要**再套 `require_auth`）。

use axum::{
    extract::{Query, Request, State},
    http::header,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
use nextrade_core::models::member::user_address_client::{
    UserAddressCreateDTO, UserAddressIdDTO, UserAddressListQuery, UserAddressUpdateDTO,
    UserAddressVO,
};
use nextrade_core::types::PageResult;
use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};
use nextrade_security::JwtService;

use crate::PublicState;

/// 本域路由清单（完整路径 + 原 Java 方法名）。
const ROUTE_TABLE: [(&str, &str, &str); 7] = [
    (
        "GET",
        "/api/user/address/tagList",
        "UserAddressClientController.tagList",
    ),
    (
        "GET",
        "/api/user/address/list",
        "UserAddressClientController.list",
    ),
    (
        "GET",
        "/api/user/address/detail",
        "UserAddressClientController.detail",
    ),
    (
        "POST",
        "/api/user/address/del",
        "UserAddressClientController.del",
    ),
    (
        "POST",
        "/api/user/address/create",
        "UserAddressClientController.create",
    ),
    (
        "POST",
        "/api/user/address/update",
        "UserAddressClientController.update",
    ),
    (
        "POST",
        "/api/user/address/setSelected",
        "UserAddressClientController.setSelected",
    ),
];

/// 收货地址前台路由（挂载于 `/api` 下，自带可选登录态中间件）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/user/address/tagList", get(tag_list))
        .route("/user/address/list", get(list))
        .route("/user/address/detail", get(detail))
        .route("/user/address/del", post(del))
        .route("/user/address/create", post(create))
        .route("/user/address/update", post(update))
        .route("/user/address/setSelected", post(set_selected))
        .layer(middleware::from_fn_with_state(
            state.jwt.clone(),
            optional_auth,
        ))
}

/// 本域已实现路由（覆盖率统计用）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, java_ref)| nextrade_openapi::RouteDoc::new(method, path, java_ref))
        .collect()
}

/// 可选登录态中间件（Java `permitAll` + JWT 过滤器）。
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

/// 取登录会员 ID；游客返回 `0`（Java `SecurityUtils.getCurrentUserId()`）。
fn current_user_id(auth: &Option<Extension<AuthUser>>) -> i32 {
    auth.as_ref().map(|Extension(a)| a.id).unwrap_or(0)
}

/// `GET /api/user/address/detail?id=` 的查询参数。
#[derive(Debug, Deserialize)]
struct IdQuery {
    id: Option<i32>,
}

/// `GET /api/user/address/tagList`：常用标签列表。
async fn tag_list(State(state): State<PublicState>) -> ApiResult<Vec<&'static str>> {
    ok(state.member.user_address_client.tag_list())
}

/// `Json<T>` 提取失败 → 业务 JSON(带文案)。
///
/// axum 内置 rejection 是纯文本 422/400/415,前端 `error.message` 为空,
/// 只弹灰色空 toast;转成统一 `ApiResponse` 后用户能看到具体原因。
fn parse_body<T>(result: Result<Json<T>, axum::extract::rejection::JsonRejection>) -> Result<T, AppError> {
    result
        .map(|Json(v)| v)
        .map_err(|re| AppError(Error::biz_msg(format!("请求参数解析失败: {}", re.body_text()))))
}

/// `GET /api/user/address/list`：当前用户地址列表。
async fn list(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<UserAddressListQuery>,
) -> ApiResult<PageResult<UserAddressVO>> {
    let user_id = current_user_id(&auth);
    ok(state.member.user_address_client.list(user_id, &q).await?)
}

/// `GET /api/user/address/detail?id=`：地址详情（需登录）。
async fn detail(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<IdQuery>,
) -> ApiResult<UserAddressVO> {
    let user_id = current_user_id(&auth);
    ensure_login(user_id)?;
    ok(state.member.user_address_client.detail(user_id, q.id).await?)
}

/// `POST /api/user/address/del`：删除地址（需登录）。
async fn del(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    dto: Result<Json<UserAddressIdDTO>, axum::extract::rejection::JsonRejection>,
) -> ApiResult<()> {
    let dto = parse_body(dto)?;
    let user_id = current_user_id(&auth);
    ensure_login(user_id)?;
    state.member.user_address_client.delete(user_id, &dto).await?;
    ok(())
}

/// `POST /api/user/address/create`：创建地址。
async fn create(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    dto: Result<Json<UserAddressCreateDTO>, axum::extract::rejection::JsonRejection>,
) -> ApiResult<i32> {
    let dto = parse_body(dto)?;
    let user_id = current_user_id(&auth);
    ok(state.member.user_address_client.create(user_id, &dto).await?)
}

/// `POST /api/user/address/update`：修改地址（需登录）。
async fn update(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    dto: Result<Json<UserAddressUpdateDTO>, axum::extract::rejection::JsonRejection>,
) -> ApiResult<()> {
    let dto = parse_body(dto)?;
    let user_id = current_user_id(&auth);
    ensure_login(user_id)?;
    state.member.user_address_client.update(user_id, &dto).await?;
    ok(())
}

/// `POST /api/user/address/setSelected`：设置选中地址。
async fn set_selected(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    dto: Result<Json<UserAddressIdDTO>, axum::extract::rejection::JsonRejection>,
) -> ApiResult<()> {
    let dto = parse_body(dto)?;
    let user_id = current_user_id(&auth);
    state
        .member
        .user_address_client
        .set_selected(user_id, &dto)
        .await?;
    ok(())
}

/// 路由层登录校验（供本文件 handler 复用）。
fn ensure_login(user_id: i32) -> Result<(), AppError> {
    nextrade_services::member::user_address_client::UserAddressClientService::ensure_logged_in(
        user_id,
    )
    .map_err(AppError)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_java_contract() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 7);
        let expected = [
            ("GET", "/api/user/address/tagList"),
            ("GET", "/api/user/address/list"),
            ("GET", "/api/user/address/detail"),
            ("POST", "/api/user/address/del"),
            ("POST", "/api/user/address/create"),
            ("POST", "/api/user/address/update"),
            ("POST", "/api/user/address/setSelected"),
        ];
        for (method, path) in expected {
            assert!(
                docs.iter().any(|d| d.method == method && d.path == path),
                "缺少 {method} {path}"
            );
        }
        let mut unique: Vec<(String, String)> =
            docs.iter().map(|d| (d.method.clone(), d.path.clone())).collect();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), docs.len(), "implemented_routes() 存在重复项");
    }

    #[test]
    fn route_table_is_unique_and_prefixed() {
        assert_eq!(ROUTE_TABLE.len(), 7);
        for (i, (_, p, _)) in ROUTE_TABLE.iter().enumerate() {
            assert!(p.starts_with("/api/user/address/"), "路径前缀错误: {p}");
            for (_, p2, _) in ROUTE_TABLE.iter().skip(i + 1) {
                assert_ne!(p, p2, "重复路由声明会导致 merge panic");
            }
        }
    }

    #[test]
    fn current_user_id_matches_java_security_utils() {
        assert_eq!(current_user_id(&None), 0);
        let auth = Some(Extension(AuthUser {
            id: 12,
            username: "u".into(),
            role_id: None,
            admin_type: None,
            shop_id: None,
        }));
        assert_eq!(current_user_id(&auth), 12);
    }

    #[test]
    fn ensure_login_blocks_anonymous() {
        assert!(ensure_login(0).is_err());
        assert!(ensure_login(3).is_ok());
    }
}
