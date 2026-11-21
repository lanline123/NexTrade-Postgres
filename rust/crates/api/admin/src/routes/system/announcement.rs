//! 管理端平台公告路由（我方新增，论坛帖 587859 余子项）。
//!
//! 平台管理员发布公告 → 后端向全部店铺各分发一行 `admin_msg`
//! （msg_type = 59「平台公告」），商家在消息中心/公告页阅读，已读状态
//! 完全复用 `admin_msg` 既有口径。本文件只有平台管理三端点（均限平台
//! 管理员）；商家侧列表/已读走既有 `/msg/adminMsg/list?msgType=59`、
//! `/msg/adminMsg/setReaded`，不另设端点。

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
use nextrade_core::models::system::admin_msg::{AnnouncementPublishDTO, AnnouncementVO};
use nextrade_core::types::PageResult;
use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, AuthUser, TenantContext};

use crate::AdminState;

/// 本域 3 条路由（挂载路径 + 说明）。
const ROUTE_TABLE: [(&str, &str, &str); 3] = [
    (
        "POST",
        "/msg/announcement/publish",
        "PlatformAnnouncement.publish（我方新增）",
    ),
    (
        "GET",
        "/msg/announcement/list",
        "PlatformAnnouncement.list（我方新增）",
    ),
    (
        "POST",
        "/msg/announcement/delete",
        "PlatformAnnouncement.delete（我方新增）",
    ),
];

/// 平台公告管理路由（挂载于 `/adminapi` 下）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/msg/announcement/publish", post(publish))
        .route("/msg/announcement/list", get(list))
        .route("/msg/announcement/delete", post(delete))
}

/// 本域已实现路由（完整契约路径）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, note)| {
            nextrade_openapi::RouteDoc::new(method, &format!("/adminapi{path}"), note)
        })
        .collect()
}

/// 公告管理仅平台管理员可用（店铺/供应商账号一律拒绝）。
fn ensure_platform(tenant: &TenantContext) -> nextrade_core::Result<()> {
    if tenant.is_platform() {
        Ok(())
    } else {
        Err(Error::Forbidden("仅平台管理员可管理平台公告".into()))
    }
}

/// 公告列表分页入参。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct AnnouncementListQuery {
    page: Option<u64>,
    size: Option<u64>,
}

/// 删除入参（公告主行 `msgId`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct AnnouncementDeleteBody {
    msg_id: Option<i32>,
}

async fn publish(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Extension(tenant): Extension<TenantContext>,
    Json(dto): Json<AnnouncementPublishDTO>,
) -> ApiResult<AnnouncementVO> {
    ensure_platform(&tenant)?;
    ok(state
        .system
        .admin_msg
        .publish_announcement(auth.id, &dto)
        .await?)
}

async fn list(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Query(q): Query<AnnouncementListQuery>,
) -> ApiResult<PageResult<AnnouncementVO>> {
    ensure_platform(&tenant)?;
    ok(state
        .system
        .admin_msg
        .announcement_list(q.page.unwrap_or(1), q.size.unwrap_or(15))
        .await?)
}

async fn delete(
    State(state): State<AdminState>,
    Extension(tenant): Extension<TenantContext>,
    Json(body): Json<AnnouncementDeleteBody>,
) -> ApiResult<u64> {
    ensure_platform(&tenant)?;
    ok(state
        .system
        .admin_msg
        .delete_announcement(body.msg_id)
        .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 3);
        assert_eq!(docs[0].method, "POST");
        assert_eq!(docs[0].path, "/adminapi/msg/announcement/publish");
        assert_eq!(docs[1].method, "GET");
        assert_eq!(docs[1].path, "/adminapi/msg/announcement/list");
        assert_eq!(docs[2].method, "POST");
        assert_eq!(docs[2].path, "/adminapi/msg/announcement/delete");
    }

    #[test]
    fn platform_guard_rejects_shop_and_vendor() {
        let shop = TenantContext::new(Some("shop".into()), Some(5), None);
        assert!(ensure_platform(&shop).is_err());
        let vendor = TenantContext::new(Some("vendor".into()), None, Some(3));
        assert!(ensure_platform(&vendor).is_err());
        let platform = TenantContext::new(Some("admin".into()), None, None);
        assert!(ensure_platform(&platform).is_ok());
    }
}
