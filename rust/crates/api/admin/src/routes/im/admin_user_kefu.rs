//! 客服人员管理 —— 管理端路由（im::admin_user_kefu）。
//!
//! 原 Java `AdminUserKefuController`
//! （`java/nextrade-adminapi/.../controller/im/AdminUserKefuController.java`，
//! `@RequestMapping("/adminapi/im/adminUserKefu")`，9 个端点）。
//!
//! | 本模块路由（挂载于 `/adminapi`） | 权限 |
//! |-----------------------------------|------|
//! | `GET  /im/adminUserKefu/list` | kefuManageList |
//! | `GET  /im/adminUserKefu/detail` | kefuManageList |
//! | `GET  /im/adminUserKefu/info` | kefuManageList |
//! | `GET  /im/adminUserKefu/roles` | kefuManageList |
//! | `POST /im/adminUserKefu/create` | kefuManageModify |
//! | `POST /im/adminUserKefu/update` | kefuManageModify |
//! | `POST /im/adminUserKefu/del` | kefuManageModify |
//! | `POST /im/adminUserKefu/batch` | kefuManageModify |
//! | `POST /im/adminUserKefu/updateField` | kefuManageModify |
//!
//! > 权限矩阵未实现（仅 `require_auth`）；上表权限码来自契约，用于后续 RBAC 接入。
//!
//! > **shopId**：Java 直接 `HeaderUtils.getShopId()`（`x-shop-id` 请求头）。

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;

use nextrade_core::models::im::admin_user_kefu::{
    AdminUserKefuCreateDTO, AdminUserKefuPageQuery, AdminUserKefuUpdateDTO, AdminUserKefuVO,
    KefuBatchDTO, KefuOperateDTO, KefuRoleSimpleVO, KefuUpdateFieldDTO,
};
use nextrade_core::types::PageResult;
use nextrade_core::Error;
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};

use crate::AdminState;

/// 本域 9 条路由（去 `/adminapi` 前缀的路径 + 原 Java handler）。
const ROUTE_TABLE: [(&str, &str, &str); 9] = [
    (
        "GET",
        "/im/adminUserKefu/list",
        "AdminUserKefuController.list",
    ),
    (
        "GET",
        "/im/adminUserKefu/detail",
        "AdminUserKefuController.detail",
    ),
    (
        "GET",
        "/im/adminUserKefu/info",
        "AdminUserKefuController.info",
    ),
    (
        "GET",
        "/im/adminUserKefu/roles",
        "AdminUserKefuController.roles",
    ),
    (
        "POST",
        "/im/adminUserKefu/create",
        "AdminUserKefuController.create",
    ),
    (
        "POST",
        "/im/adminUserKefu/update",
        "AdminUserKefuController.update",
    ),
    (
        "POST",
        "/im/adminUserKefu/del",
        "AdminUserKefuController.del",
    ),
    (
        "POST",
        "/im/adminUserKefu/batch",
        "AdminUserKefuController.batch",
    ),
    (
        "POST",
        "/im/adminUserKefu/updateField",
        "AdminUserKefuController.updateField",
    ),
];

/// 客服人员路由（挂载于 `/adminapi` 下，外层已统一 `require_auth`）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/im/adminUserKefu/list", get(list))
        .route("/im/adminUserKefu/detail", get(detail))
        .route("/im/adminUserKefu/info", get(info))
        .route("/im/adminUserKefu/roles", get(roles))
        .route("/im/adminUserKefu/create", post(create))
        .route("/im/adminUserKefu/update", post(update))
        .route("/im/adminUserKefu/del", post(del))
        .route("/im/adminUserKefu/batch", post(batch))
        .route("/im/adminUserKefu/updateField", post(update_field))
}

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, java_ref)| {
            nextrade_openapi::RouteDoc::new(method, &format!("/adminapi{path}"), java_ref)
        })
        .collect()
}

/// 当前店铺 ID（多租户：**身份只信任认证中间件**，忽略可伪造的 `x-shop-id`）。
///
/// ⚠️ 平台管理员必须映射成 `shop_id = 0`，**不能返回 400**。
///
/// Java 侧 `HeaderUtils.getShopId()`：
/// ```java
/// String adminType = getHeaderValue(X_ADMIN_TYPE);
/// if (ObjectUtil.equals(adminType, "admin")) return 0;   // ← 平台账号 → 0
/// if ("shop".equals(adminType) || "pos".equals(adminType))
///     return StrUtil.isEmpty(shopId) ? null : Integer.parseInt(shopId);
/// return null;
/// ```
/// 客服人员控制器直接把它传给 `listUser(pageQuery, shopId)`，
/// 平台后台因此查 `shop_id = 0` 的客服数据，**不会报错**。
///
/// Rust 早期实现把「平台账号无 shop_id」判成参数校验失败返回 400，
/// 于是平台管理员打开「客服人员」页必然 400 + 空列表
/// （2026-10-21 巡检 `/admin/customer-service/agents/list` 的根因）。
fn current_shop_id(_headers: &HeaderMap) -> Result<i32, AppError> {
    let scope = TenantScope::current();
    Ok(scope.shop_id().unwrap_or(0).max(0))
}

/// 详情/信息查询参数（`?id=`）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdQuery {
    /// 绑定 ID（detail）或管理员 ID（info）
    id: i32,
}

/// `GET /adminapi/im/adminUserKefu/list`：客服人员分页。
async fn list(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Query(q): Query<AdminUserKefuPageQuery>,
) -> ApiResult<PageResult<AdminUserKefuVO>> {
    let shop_id = current_shop_id(&headers)?;
    ok(state.im.admin_user_kefu.list(shop_id, &q).await?)
}

/// `GET /adminapi/im/adminUserKefu/detail?id=`：客服详情。
async fn detail(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Query(q): Query<IdQuery>,
) -> ApiResult<Option<AdminUserKefuVO>> {
    let shop_id = current_shop_id(&headers)?;
    ok(state.im.admin_user_kefu.detail(q.id, shop_id).await?)
}

/// `GET /adminapi/im/adminUserKefu/info?id=`：按管理员 ID 查客服信息。
async fn info(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Query(q): Query<IdQuery>,
) -> ApiResult<AdminUserKefuVO> {
    let shop_id = current_shop_id(&headers)?;
    ok(state.im.admin_user_kefu.info(q.id, shop_id).await?)
}

/// `GET /adminapi/im/adminUserKefu/roles`：客服角色列表。
async fn roles(
    State(state): State<AdminState>,
    headers: HeaderMap,
) -> ApiResult<Vec<KefuRoleSimpleVO>> {
    let shop_id = current_shop_id(&headers)?;
    ok(state.im.admin_user_kefu.roles(shop_id).await?)
}

/// `POST /adminapi/im/adminUserKefu/create`：创建客服。
async fn create(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Json(dto): Json<AdminUserKefuCreateDTO>,
) -> ApiResult<()> {
    let shop_id = current_shop_id(&headers)?;
    state.im.admin_user_kefu.create(shop_id, &dto).await?;
    ok(())
}

/// `POST /adminapi/im/adminUserKefu/update`：更新客服。
async fn update(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Json(dto): Json<AdminUserKefuUpdateDTO>,
) -> ApiResult<()> {
    let shop_id = current_shop_id(&headers)?;
    state.im.admin_user_kefu.update(shop_id, &dto).await?;
    ok(())
}

/// `POST /adminapi/im/adminUserKefu/del`：删除客服（禁止删除自己）。
async fn del(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<KefuOperateDTO>,
) -> ApiResult<()> {
    let shop_id = current_shop_id(&headers)?;
    state
        .im
        .admin_user_kefu
        .del(dto.id, auth.id, shop_id)
        .await?;
    ok(())
}

/// `POST /adminapi/im/adminUserKefu/batch`：批量删除（仅 `type='del'`）。
async fn batch(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<KefuBatchDTO>,
) -> ApiResult<()> {
    let shop_id = current_shop_id(&headers)?;
    state
        .im
        .admin_user_kefu
        .batch(shop_id, auth.id, &dto)
        .await?;
    ok(())
}

/// `POST /adminapi/im/adminUserKefu/updateField`：更新字段（仅 `isUsing`）。
async fn update_field(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Json(dto): Json<KefuUpdateFieldDTO>,
) -> ApiResult<()> {
    let shop_id = current_shop_id(&headers)?;
    state.im.admin_user_kefu.update_field(shop_id, &dto).await?;
    ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_table_has_all_nine_contract_paths() {
        assert_eq!(ROUTE_TABLE.len(), 9);
        let docs = implemented_routes();
        assert_eq!(docs.len(), 9);
        assert_eq!(docs[0].path, "/adminapi/im/adminUserKefu/list");
        assert_eq!(docs[8].method, "POST");
        assert_eq!(docs[8].path, "/adminapi/im/adminUserKefu/updateField");
        assert!(ROUTE_TABLE.iter().all(|(_, p, _)| !p.contains("/adminapi")));
    }

    #[test]
    fn route_methods_match_java_controller() {
        let gets = ROUTE_TABLE.iter().filter(|(m, _, _)| *m == "GET").count();
        assert_eq!(gets, 4);
        assert_eq!(ROUTE_TABLE.len() - gets, 5);
    }

    #[tokio::test]
    async fn shop_id_comes_from_trusted_identity_not_headers() {
        use nextrade_infrastructure::db::repo::marketing::tenant::with_tenant_scope;
        // 平台账号（无租户上下文）→ shop_id=0 平台视角（2026-10-21 修复：不再 400）
        assert_eq!(current_shop_id(&HeaderMap::new()).unwrap(), 0);
        // 店铺身份 → 本店（HeaderMap 内容被忽略）
        let shop = TenantScope::for_shop(6);
        let id =
            with_tenant_scope(shop, async { current_shop_id(&HeaderMap::new()).unwrap() }).await;
        assert_eq!(id, 6);
    }
}
