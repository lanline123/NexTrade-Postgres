//! 分销员外壳店「店中店」管理端（marketing::salesman_shop，我方新增）。
//!
//! | method | path（挂载于 `/adminapi`） |
//! |--------|------------------------------|
//! | `GET`  | `/salesman/shop/list`        |
//! | `POST` | `/salesman/shop/setEnabled`  |
//!
//! 平台可查看全部外壳店并启停其**展示**；启停只翻 `salesman_shop.
//! is_enabled`，不动分销员身份、客户绑定与佣金（干净分离，见服务层）。
//! 租户作用域：`AuthUser::scoped_shop_id()` —— 平台 None 不限；商家
//! 账号按其 `salesman.shop_id` 过滤/校验；缺作用域哨兵 -1 fail-closed。
//!
//! 我方新增端点：不登记进 Java 契约 `ROUTE_TABLE`。

use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};

use nextrade_core::models::marketing::salesman_shop::{
    ShellShopAdminQuery, ShellShopAdminVO, ShellShopSetEnabledDTO,
};
use nextrade_core::types::PageResult;
use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::AdminState;

/// 外壳店管理路由（`/adminapi` 下，全局 `require_auth` 已生效）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/salesman/shop/list", get(list))
        .route("/salesman/shop/setEnabled", post(set_enabled))
}

/// `GET /adminapi/salesman/shop/list`：外壳店分页列表。
async fn list(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<ShellShopAdminQuery>,
) -> ApiResult<PageResult<ShellShopAdminVO>> {
    ok(state
        .marketing
        .salesman_shop
        .admin_list(auth.scoped_shop_id(), &q)
        .await?)
}

/// `POST /adminapi/salesman/shop/setEnabled`：启停外壳展示。
async fn set_enabled(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<ShellShopSetEnabledDTO>,
) -> ApiResult<()> {
    let salesman_id = dto
        .salesman_id
        .ok_or_else(|| Error::Validation("salesmanId不能为空".into()))?;
    let enabled = dto
        .enabled
        .ok_or_else(|| Error::Validation("enabled不能为空".into()))?;
    state
        .marketing
        .salesman_shop
        .admin_set_enabled(auth.scoped_shop_id(), salesman_id, enabled)
        .await?;
    ok(())
}
