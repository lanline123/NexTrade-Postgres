//! 管理端域路由的租户作用域中间件（marketing / content / im 共用）。
//!
//! 认证中间件已把 JWT 身份写入 request extensions
//! （`TenantContext` 优先，兜底 `AuthUser`）。本层将其转换为
//! [`TenantScope`] 并包进 task-local，使域内 Handler 无需改签名即可在
//! Service / Repository 读取：
//!
//! - 列表：`TenantScope::current_list_filter()`（`None` = 平台不过滤）
//! - 详情/更新/删除：`TenantScope::current_ensure_access(row.shop_id)?`
//! - 创建：`TenantScope::current().require_write_shop(requested)?`
//!
//! 另外在**入口**拒绝伪造的 `shopId` 查询参数（店铺账号传入其他店铺 →
//! 403，文案与 security 一致），避免 Handler 误把 query 当身份。
//!
//! 注意：本层只处理 query string 中的 `shopId`/`shop_id`；JSON body 内的
//! `shopId` 由各 Service 通过 [`TenantScope::require_write_shop`] 兜底校验。

use axum::{
    extract::{Extension, Request},
    middleware::Next,
    response::Response,
};
use nextrade_core::Error;
use nextrade_infrastructure::db::repo::marketing::tenant::{with_tenant_scope, TenantScope};
use nextrade_security::web::{AppError, AuthUser, TenantContext};

/// 从 query string 解析 `shopId` / `shop_id`（首个非零值）。
#[must_use]
fn query_shop_id(query: Option<&str>) -> Option<i32> {
    let mut found: Option<i32> = None;
    for pair in query.unwrap_or_default().split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or_default();
        if !matches!(key, "shopId" | "shop_id" | "shopid") {
            continue;
        }
        let value = parts.next().unwrap_or_default();
        if let Ok(id) = value.trim().parse::<i32>() {
            if id > 0 {
                found = Some(id);
            }
        }
    }
    found
}

/// 解析当前请求的租户作用域（平台 / 指定店铺）。
#[must_use]
fn resolve_scope(req: &Request) -> TenantScope {
    if let Some(ctx) = req.extensions().get::<TenantContext>() {
        return TenantScope::from_identity(ctx.admin_type.as_deref(), ctx.shop_id);
    }
    if let Some(auth) = req.extensions().get::<AuthUser>() {
        return TenantScope::from_identity(auth.admin_type.as_deref(), auth.shop_id);
    }
    TenantScope::platform()
}

/// 域路由租户作用域中间件：伪造 query shopId 拦截 + task-local 作用域。
///
/// # Errors
/// 店铺账号请求其他店铺 `shopId` → `Forbidden(无权访问店铺 {id} 的数据)`。
pub async fn tenant_scope_layer(req: Request, next: Next) -> Result<Response, AppError> {
    let scope = resolve_scope(&req);
    if let Some(scope_shop) = scope.shop_id() {
        if let Some(requested) = query_shop_id(req.uri().query()) {
            if requested != scope_shop {
                return Err(AppError(Error::Forbidden(format!(
                    "无权访问店铺 {requested} 的数据"
                ))));
            }
        }
    }
    Ok(with_tenant_scope(scope, next.run(req)).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_shop_id_parsing() {
        assert_eq!(query_shop_id(Some("page=1&shopId=9")), Some(9));
        assert_eq!(query_shop_id(Some("shop_id=12&x=1")), Some(12));
        assert_eq!(query_shop_id(Some("shopId=0")), None);
        assert_eq!(query_shop_id(Some("shopId=abc")), None);
        assert_eq!(query_shop_id(None), None);
        assert_eq!(query_shop_id(Some("other=5")), None);
    }

    #[test]
    fn shop_scope_identity_resolution() {
        // 店铺账号：list filter 强制、跨店访问拒绝
        let scope = TenantScope::from_identity(Some("shop"), Some(8));
        assert_eq!(scope.list_filter(), Some(8));
        assert!(scope.ensure_access(9).is_err());
        // 平台账号不过滤
        let platform = TenantScope::from_identity(Some("admin"), None);
        assert_eq!(platform.list_filter(), None);
    }
}
