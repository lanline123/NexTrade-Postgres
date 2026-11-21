//! 店铺小程序作用域（买家侧多小程序数据隔离上下文）。
//!
//! 背景：店铺独立绑定小程序（论坛帖 487946315732211118，我方新增）后，同一
//! 后端要同时服务平台小程序与各店铺自有小程序。客户端每请求带
//! `x-mini-app-id` 头上报自身 appid，前台中间件
//! （`api/public` 的 `mini_shop_scope_layer`）按绑定表解析：
//!
//! - 无 appid 头（H5 / PC / 平台外渠道）→ 平台语义，不过滤；
//! - 平台小程序 appid → 平台语义，不过滤；
//! - 已绑定且启用的店铺 appid → 店铺作用域，数据面强制收口到该店；
//! - 未登记 / 已停用 / 已解绑的 appid → 中间件直接拒绝（硬拒绝，防隔离击穿）。
//!
//! 解析结果经请求级 task-local 透传到 Service 层（与管理端
//! [`super::super::marketing::tenant::TenantScope`] 同一手法），Service 只读
//! 本模块，不信任客户端自报的 shopId。非请求上下文（后台任务 / 单测）默认
//! 平台语义，避免误拦截存量链路。

use std::future::Future;

use nextrade_core::{Error, Result};

tokio::task_local! {
    /// 当前请求的店铺小程序作用域（仅在前台中间件作用域内有效）。
    pub static CURRENT_MINI_SHOP: MiniShopScope;
}

/// 当前请求的小程序作用域。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MiniShopScope {
    /// 绑定的店铺（`None` = 平台模式，不做店铺过滤）。
    shop_id: Option<i32>,
}

impl MiniShopScope {
    /// 平台模式：平台小程序 / H5 / PC 等全平台渠道。
    #[must_use]
    pub fn platform() -> Self {
        Self { shop_id: None }
    }

    /// 店铺模式：请求来自绑定了 `shop_id` 的店铺自有小程序。
    #[must_use]
    pub fn for_shop(shop_id: i32) -> Self {
        if shop_id > 0 {
            Self {
                shop_id: Some(shop_id),
            }
        } else {
            Self::platform()
        }
    }

    /// 是否店铺模式。
    #[must_use]
    pub fn is_shop_mode(&self) -> bool {
        self.shop_id.is_some()
    }

    /// 绑定的店铺（平台模式为 `None`）。
    #[must_use]
    pub fn shop_id(&self) -> Option<i32> {
        self.shop_id
    }

    /// 列表查询的店铺过滤值（平台 → `None` 不加条件；店铺 → `Some(shop_id)`）。
    #[must_use]
    pub fn list_filter(&self) -> Option<i32> {
        self.shop_id
    }

    /// 校验指定店铺的数据在当前作用域内可见（平台放行）。
    ///
    /// # Errors
    /// 店铺模式下访问其他店铺数据 → `Forbidden`。
    pub fn ensure_access(&self, shop_id: i32) -> Result<()> {
        match self.shop_id {
            Some(scope) if scope != shop_id => Err(Error::Forbidden(format!(
                "当前小程序仅可访问店铺 {scope} 的数据"
            ))),
            _ => Ok(()),
        }
    }

    /// 当前请求作用域；无作用域（后台任务 / 单测未设置）→ 平台语义。
    #[must_use]
    pub fn current() -> Self {
        CURRENT_MINI_SHOP
            .try_with(|scope| *scope)
            .unwrap_or_else(|_| Self::platform())
    }

    /// 当前请求的列表过滤值（`None` = 平台不过滤）。
    #[must_use]
    pub fn current_list_filter() -> Option<i32> {
        Self::current().list_filter()
    }

    /// 在指定作用域内执行 Future（供前台中间件使用）。
    pub async fn scope<F>(self, fut: F) -> F::Output
    where
        F: Future,
    {
        CURRENT_MINI_SHOP.scope(self, fut).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_scope_passes_everything() {
        let scope = MiniShopScope::platform();
        assert!(!scope.is_shop_mode());
        assert_eq!(scope.list_filter(), None);
        assert!(scope.ensure_access(0).is_ok());
        assert!(scope.ensure_access(42).is_ok());
    }

    #[test]
    fn shop_scope_restricts_to_bound_shop() {
        let scope = MiniShopScope::for_shop(7);
        assert!(scope.is_shop_mode());
        assert_eq!(scope.shop_id(), Some(7));
        assert_eq!(scope.list_filter(), Some(7));
        assert!(scope.ensure_access(7).is_ok());
        assert!(scope.ensure_access(8).is_err());
        assert!(scope.ensure_access(0).is_err());
    }

    #[test]
    fn non_positive_shop_falls_back_to_platform() {
        assert!(!MiniShopScope::for_shop(0).is_shop_mode());
        assert!(!MiniShopScope::for_shop(-1).is_shop_mode());
    }

    #[tokio::test]
    async fn current_defaults_to_platform_outside_scope() {
        assert!(!MiniShopScope::current().is_shop_mode());
        let inside = MiniShopScope::for_shop(3)
            .scope(async { MiniShopScope::current().shop_id() })
            .await;
        assert_eq!(inside, Some(3));
        // 作用域退出后恢复默认
        assert!(!MiniShopScope::current().is_shop_mode());
    }
}
