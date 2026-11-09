//! content 域多租户作用域助手（`TenantScope` → content 语义）。
//!
//! 由域路由中间件（`api/admin/src/routes/content/mod.rs` 的 `tenant_scope_layer`）
//! 写入请求级 task-local；Service 层通过这些助手读取，不必修改 Handler 签名。
//!
//! # 平台素材规则（content 素材类 feature）
//!
//! - `shop_id = 0`：平台素材，**所有账号可读**；
//! - 店铺素材：仅本店账号可读写；
//! - 平台账号：不做店铺过滤（可读写任意店铺）。
//!
//! 对应列表条件：店铺账号 `(shop_id = 0 OR shop_id = ?)`；平台账号不加条件。
//! 本模块提供：
//! - [`media_guard`]：列表过滤值（`None` = 平台不过滤）；
//! - [`ensure_media_access`]：详情/更新/删除归属校验（平台素材放行）；
//! - [`media_write_shop_id`]：create 强制落库店铺（店铺→本店；平台→请求值或 0）。

use nextrade_core::Result;
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;

/// 当前请求的素材列表过滤值（`None` = 平台账号/无上下文，不加店铺条件）。
#[must_use]
pub fn media_list_filter() -> Option<i32> {
    TenantScope::current_list_filter()
}

/// 素材归属校验：平台素材（`shop_id = 0`）放行；店铺素材仅本店可访问。
///
/// # Errors
/// 店铺账号访问其他店铺素材 → `Forbidden(无权访问店铺 {id} 的数据)`。
pub fn ensure_media_access(shop_id: i32) -> Result<()> {
    let scope = TenantScope::current();
    if scope.is_platform() || shop_id == 0 {
        return Ok(());
    }
    scope.ensure_access(shop_id)
}

/// 素材**写**归属校验：平台账号放行；店铺账号仅本店素材可写
/// （平台素材 `shop_id = 0` 对店铺账号只读）。
///
/// # Errors
/// 店铺账号修改/删除其他店铺或平台素材 → `Forbidden(无权访问店铺 {id} 的数据)`。
pub fn ensure_media_write(shop_id: i32) -> Result<()> {
    let scope = TenantScope::current();
    if scope.is_platform() {
        return Ok(());
    }
    scope.ensure_access(shop_id)
}

/// create 的落库店铺：店铺账号强制本店；平台账号用请求值，缺省落平台（0）。
///
/// # Errors
/// 店铺账号显式传入其他店铺 → `Forbidden(无权写入店铺 {id} 的数据)`。
pub fn media_write_shop_id(requested: Option<i32>) -> Result<i32> {
    TenantScope::current().write_shop_id(requested)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_infrastructure::db::repo::marketing::tenant::with_tenant_scope;

    #[tokio::test]
    async fn platform_context_sees_all_materials() {
        let scope = TenantScope::platform();
        let (filter, access_other, write_shop) = with_tenant_scope(scope, async {
            (
                media_list_filter(),
                ensure_media_access(9).is_ok(),
                media_write_shop_id(None).unwrap(),
            )
        })
        .await;
        assert_eq!(filter, None);
        assert!(access_other);
        assert_eq!(write_shop, 0);
    }

    #[tokio::test]
    async fn shop_context_sees_platform_and_own_but_not_other() {
        let scope = TenantScope::for_shop(8);
        let result = with_tenant_scope(scope, async {
            (
                media_list_filter(),
                ensure_media_access(0).is_ok(),
                ensure_media_access(8).is_ok(),
                ensure_media_access(9).is_err(),
                media_write_shop_id(None).unwrap(),
                media_write_shop_id(Some(9)).is_err(),
            )
        })
        .await;
        let (filter, platform_ok, own_ok, other_err, write_own, write_other_err) = result;
        assert_eq!(filter, Some(8));
        assert!(platform_ok);
        assert!(own_ok);
        assert!(other_err);
        assert_eq!(write_own, 8);
        // 店铺账号显式传其他店铺 → 403
        assert!(write_other_err);
    }
}
