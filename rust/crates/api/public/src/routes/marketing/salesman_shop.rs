//! 公开端分销员外壳店「店中店」（marketing::salesman_shop，我方新增）。
//!
//! 端点（模块自带可选登录态中间件，与 salesman_admin 同实现）：
//! - `GET /api/salesman/shop/detail?salesmanId=` — 买家侧外壳店详情
//!   （自定义店名/店招/简介 + 昵称头像回退 + 平台展示开关）。
//! - `GET /api/salesman/shop/myShop` — 分销员自助视图（需登录）。
//! - `POST /api/salesman/shop/save` — 分销员保存三项自定义（需登录）。
//!
//! 我方新增端点：不登记进 Java 契约 `ROUTE_TABLE`（与 wholesale/print
//! 域同口径，见 lib.rs implemented_routes 注释）。
//!
//! 归因说明：本模块不写任何绑定——买家落地写 `bindSalesmanId`、注册
//! 绑定、订单归因与佣金生成全部走既有链；平台禁用只影响 detail 的
//! `enabled` 展示口径，不动身份/绑定/佣金。

use axum::extract::{Extension, Query, Request, State};
use axum::http::header;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use nextrade_core::models::marketing::salesman_shop::{
    ShellShopMineVO, ShellShopPublicVO, ShellShopSaveDTO,
};
use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};
use nextrade_security::JwtService;

use crate::PublicState;

/// 可选登录态中间件（对应 Java `permitAll` + JWT 过滤器，与
/// salesman_admin 模块同实现）。
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
        let Ok(claims) = jwt.from_authorization(token) else {
            return Ok(next.run(req).await);
        };
        let Ok(id) = claims.sub.parse::<i32>() else {
            return Ok(next.run(req).await);
        };
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

/// 取登录会员 ID；游客返回 `0`。
fn current_user_id(auth: &Option<Extension<AuthUser>>) -> i32 {
    auth.as_ref().map(|Extension(a)| a.id).unwrap_or(0)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SalesmanIdQuery {
    salesman_id: Option<i32>,
}

/// `GET /api/salesman/shop/detail?salesmanId=`：买家侧外壳店详情。
async fn detail(
    State(state): State<PublicState>,
    Query(q): Query<SalesmanIdQuery>,
) -> ApiResult<ShellShopPublicVO> {
    let salesman_id = q
        .salesman_id
        .ok_or_else(|| Error::Validation("salesmanId不能为空".into()))?;
    ok(state
        .marketing
        .salesman_shop
        .public_detail(salesman_id)
        .await?)
}

/// `GET /api/salesman/shop/myShop`：分销员自助视图（原始自定义值）。
async fn my_shop(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
) -> ApiResult<ShellShopMineVO> {
    ok(state
        .marketing
        .salesman_shop
        .my_shop(current_user_id(&auth))
        .await?)
}

/// `POST /api/salesman/shop/save`：分销员保存店名/店招/简介。
async fn save(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(dto): Json<ShellShopSaveDTO>,
) -> ApiResult<ShellShopMineVO> {
    ok(state
        .marketing
        .salesman_shop
        .save_my_shop(current_user_id(&auth), &dto)
        .await?)
}

/// 分销员外壳店公共路由（挂载于 `/api` 下，自带可选登录态中间件）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/salesman/shop/detail", get(detail))
        .route("/salesman/shop/myShop", get(my_shop))
        .route("/salesman/shop/save", post(save))
        .layer(middleware::from_fn_with_state(
            state.jwt.clone(),
            optional_auth,
        ))
}
