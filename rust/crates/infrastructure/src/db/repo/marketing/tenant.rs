//! 管理端多租户隔离上下文（marketing / content / im 域共用）。
//!
//! 背景：认证中间件（`nextrade_security::web::auth_middleware`）已把 JWT 中的
//! `shop_id` / `admin_type` 写入 request extensions 的
//! `TenantContext` / `AuthUser`，并**覆盖**客户端可伪造的
//! `x-shop-id` / `x-admin-type` 请求头。但 Rust Handler 无法在没有参数变更的
//! 情况下把身份透传到 service / repo 层，故本模块提供**请求级 task-local**：
//!
//! ```text
//! 认证中间件 (security)
//!   └─ 域路由聚合层 (api/admin routes/{marketing,content,im}/mod.rs)
//!        └─ tenant_scope_layer：Extension<TenantContext> → TenantScope
//!             └─ task_local CURRENT_TENANT 作用域内执行本域全部 Handler
//!                  └─ Service / Repository 调用 TenantScope::current()
//! ```
//!
//! 语义（与 `nextrade_security::web::AuthUser` 的
//! `is_platform` / `scoped_shop_id` / `resolve_shop_scope` /
//! `require_shop_scope` / `ensure_shop_access` 保持逐条一致）：
//! - **平台账号**：`list_filter() == None`（不加店铺条件），可读写任意店铺；
//! - **店铺账号**：`list_filter() == Some(shop_id)`（列表强制 `WHERE shop_id = ?`），
//!   访问/写入其他店铺返回 [`Error::Forbidden`]；
//! - 非请求上下文（后台任务 / 单测未作用域）默认平台语义，避免误拦截。
//!
//! 文案与 security 一致：`无权访问店铺 {id} 的数据` / `无权写入店铺 {id} 的数据`。

use std::future::Future;

use nextrade_core::{Error, Result};

tokio::task_local! {
    /// 当前请求的租户作用域（仅在本中间件作用域内有效）。
    pub static CURRENT_TENANT: TenantScope;
}

/// 当前请求的租户作用域。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TenantScope {
    platform: bool,
    shop_id: Option<i32>,
}

impl TenantScope {
    /// 平台（总后台）账号：不做店铺过滤。
    #[must_use]
    pub fn platform() -> Self {
        Self {
            platform: true,
            shop_id: None,
        }
    }

    /// 指定店铺账号。
    #[must_use]
    pub fn for_shop(shop_id: i32) -> Self {
        if shop_id > 0 {
            Self {
                platform: false,
                shop_id: Some(shop_id),
            }
        } else {
            Self::platform()
        }
    }

    /// 由 JWT 身份字段构造：
    /// 与 `AuthUser::is_platform` 一致 —— `admin_type` 为 `shop` / `pos`
    /// 或 `shop_id > 0` 视为店铺账号，其余为平台账号。
    #[must_use]
    pub fn from_identity(admin_type: Option<&str>, shop_id: Option<i32>) -> Self {
        let shop_id = shop_id.filter(|id| *id > 0);
        let shop_account = match admin_type {
            Some("shop" | "pos") => true,
            Some("member" | "vendor" | "suppliers" | "supplier") => false,
            _ => false,
        };
        if shop_account || shop_id.is_some() {
            shop_id.map_or_else(Self::platform, Self::for_shop)
        } else {
            Self::platform()
        }
    }

    /// 是否平台账号。
    #[must_use]
    pub fn is_platform(&self) -> bool {
        self.platform
    }

    /// 本账号绑定的店铺（平台为 `None`）。
    #[must_use]
    pub fn shop_id(&self) -> Option<i32> {
        self.shop_id
    }

    /// 列表查询的店铺过滤值：
    /// - 平台 → `None`（不加 `WHERE shop_id`）
    /// - 店铺 → `Some(shop_id)`
    #[must_use]
    pub fn list_filter(&self) -> Option<i32> {
        if self.platform {
            None
        } else {
            self.shop_id
        }
    }

    /// 校验能否访问指定店铺的数据（平台放行）。
    ///
    /// # Errors
    /// 店铺账号访问其他店铺 → `Forbidden(无权访问店铺 {id} 的数据)`。
    pub fn ensure_access(&self, shop_id: i32) -> Result<()> {
        match self.shop_id {
            Some(scope) if scope != shop_id => Err(Error::Forbidden(format!(
                "无权访问店铺 {shop_id} 的数据"
            ))),
            Some(_) => Ok(()),
            None if self.platform => Ok(()),
            None => Err(Error::Forbidden("缺少店铺租户上下文".into())),
        }
    }

    /// 写入落库店铺（**兼容平台级数据**）：
    /// - 店铺账号固定为自己的 `shop_id`（请求值不同 → 403）；
    /// - 平台账号：使用请求值，未传则落 `0`（平台级数据）。
    ///
    /// 与 [`TenantScope::require_write_shop`] 的区别：本方法允许平台账号
    /// 创建平台级（`shop_id = 0`）数据，用于营销活动/内容等平台兜底配置。
    ///
    /// # Errors
    /// 店铺账号越权 → `Forbidden(无权写入店铺 {id} 的数据)`。
    pub fn write_shop_id(&self, requested: Option<i32>) -> Result<i32> {
        if let Some(scope) = self.shop_id.filter(|_| !self.platform) {
            if let Some(requested) = requested.filter(|id| *id > 0 && *id != scope) {
                return Err(Error::Forbidden(format!(
                    "无权写入店铺 {requested} 的数据"
                )));
            }
            return Ok(scope);
        }
        Ok(requested.filter(|id| *id > 0).unwrap_or(0))
    }

    /// 写入落库店铺：
    /// - 店铺账号固定为自己的 `shop_id`（请求值不同 → 403）；
    /// - 平台账号必须显式传入 `requested`（否则 403）。
    ///
    /// # Errors
    /// 越权 / 缺省 → `Forbidden(无权写入店铺 {id} 的数据)` 或
    /// `Forbidden(平台账号执行该写操作时必须指定 shopId)`。
    pub fn require_write_shop(&self, requested: Option<i32>) -> Result<i32> {
        if let Some(scope) = self.shop_id.filter(|_| !self.platform) {
            if let Some(requested) = requested.filter(|id| *id > 0 && *id != scope) {
                return Err(Error::Forbidden(format!(
                    "无权写入店铺 {requested} 的数据"
                )));
            }
            return Ok(scope);
        }
        if self.platform {
            requested.filter(|id| *id > 0).ok_or_else(|| {
                Error::Forbidden("平台账号执行该写操作时必须指定 shopId".into())
            })
        } else {
            Err(Error::Forbidden("缺少店铺租户上下文".into()))
        }
    }

    /// 当前请求作用域；无作用域（后台任务 / 单测）→ 平台语义。
    #[must_use]
    pub fn current() -> Self {
        CURRENT_TENANT
            .try_with(|scope| *scope)
            .unwrap_or_else(|_| Self::platform())
    }

    /// 当前请求的列表过滤值（`None` = 平台不过滤）。
    #[must_use]
    pub fn current_list_filter() -> Option<i32> {
        Self::current().list_filter()
    }

    /// 当前请求校验指定店铺访问权限。
    ///
    /// # Errors
    /// 越权见 [`TenantScope::ensure_access`]。
    pub fn current_ensure_access(shop_id: i32) -> Result<()> {
        Self::current().ensure_access(shop_id)
    }

    /// 在指定作用域内执行 Future。
    pub async fn scope<F>(self, fut: F) -> F::Output
    where
        F: Future,
    {
        CURRENT_TENANT.scope(self, fut).await
    }
}

/// 在指定租户作用域内执行 Future（供域路由中间件使用）。
pub async fn with_tenant_scope<F, T>(scope: TenantScope, fut: F) -> T
where
    F: Future<Output = T>,
{
    CURRENT_TENANT.scope(scope, fut).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_identity_has_no_list_filter() {
        let scope = TenantScope::from_identity(Some("admin"), None);
        assert!(scope.is_platform());
        assert_eq!(scope.list_filter(), None);
        assert_eq!(scope.require_write_shop(Some(9)).unwrap(), 9);
        // 平台写操作未指定店铺 → 403
        let err = scope.require_write_shop(None).unwrap_err();
        assert!(matches!(err, Error::Forbidden(msg) if msg.contains("必须指定 shopId")));
        assert!(scope.ensure_access(123).is_ok());
    }

    #[test]
    fn shop_identity_forces_filter_and_blocks_cross_tenant() {
        let scope = TenantScope::from_identity(Some("shop"), Some(8));
        assert!(!scope.is_platform());
        assert_eq!(scope.shop_id(), Some(8));
        // 列表强制过滤
        assert_eq!(scope.list_filter(), Some(8));
        // 详情/写操作访问其他店铺 → 403（文案与 security 一致）
        let err = scope.ensure_access(9).unwrap_err();
        assert!(matches!(err, Error::Forbidden(msg) if msg == "无权访问店铺 9 的数据"));
        let err = scope.require_write_shop(Some(9)).unwrap_err();
        assert!(matches!(err, Error::Forbidden(msg) if msg == "无权写入店铺 9 的数据"));
        // 自己的店铺放行；请求 shopId 缺省 → 仍落自己的店铺
        assert_eq!(scope.require_write_shop(None).unwrap(), 8);
        assert_eq!(scope.require_write_shop(Some(8)).unwrap(), 8);
        assert!(scope.ensure_access(8).is_ok());
    }

    #[test]
    fn admin_type_shop_without_shop_id_stays_platform_but_write_requires_shop() {
        let scope = TenantScope::from_identity(Some("shop"), None);
        // shop_id 缺失时 scoped 过滤不可用：按平台语义访问只读数据，
        // 但写操作必须显式 shopId（与 security `scoped_shop_id` 行为一致）
        assert_eq!(scope.list_filter(), None);
        assert_eq!(scope.require_write_shop(Some(5)).unwrap(), 5);
    }

    #[tokio::test]
    async fn task_local_scope_is_read_by_current() {
        // 请求外默认平台语义
        assert!(TenantScope::current().is_platform());
        let shop = TenantScope::from_identity(Some("merchant"), Some(12));
        let seen = with_tenant_scope(shop, async {
            let current = TenantScope::current();
            assert_eq!(current.shop_id(), Some(12));
            assert_eq!(TenantScope::current_list_filter(), Some(12));
            TenantScope::current_ensure_access(13).is_err()
        })
        .await;
        assert!(seen);
        // 作用域结束后恢复平台语义
        assert!(TenantScope::current().is_platform());
    }
}
