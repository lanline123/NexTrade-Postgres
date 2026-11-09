//! 多租户隔离辅助（logistics 管理端 handler 统一入口）。
//!
//! 身份只来自鉴权中间件写入的 [`AuthUser`]（JWT 声明），不信任 query/body 里的
//! `shopId`，也不直接信任客户端 `x-shop-id`（中间件虽已覆写，但 handler 优先
//! Extension 以避免依赖中间件实现细节）。
//!
//! 语义与 Java `HeaderUtils.getShopId()` 对齐：
//! - 平台账号（`AuthUser::is_platform()`）→ 列表不过滤（`None`）、写操作需显式
//!   `shopId`（否则 403）、详情/更新/删除放行；
//! - 店铺账号 → 列表强制本店、写操作固定本店、访问他店数据 403。

use nextrade_core::Result;
use nextrade_security::web::AuthUser;

/// 列表/查询的店铺过滤：
/// - 店铺账号：忽略请求值并返回本店 `Some(scope)`（传入他店 → 403）；
/// - 平台账号：使用请求 `shopId`（`>0`）作为筛选，否则 `None`（不过滤）。
pub fn list_scope(auth: &AuthUser, requested: Option<i32>) -> Result<Option<i32>> {
    auth.resolve_shop_scope(requested)
}

/// 写操作的店铺归属：
/// - 店铺账号：固定本店（传入他店 → 403）；
/// - 平台账号：使用请求 `shopId`（必须显式且 `>0`，否则 403）。
pub fn write_scope(auth: &AuthUser, requested: Option<i32>) -> Result<i32> {
    auth.require_shop_scope(requested)
}

/// 打印/运费等以 `shop_id: i32` 为签名（平台 `0`）的服务调用值。
/// 平台账号 → 0（Java `HeaderUtils.getShopId()` 在平台下为 0）；店铺账号 → 本店。
pub fn platform_or_shop_id(auth: &AuthUser) -> i32 {
    auth.scoped_shop_id().unwrap_or(0)
}

/// 已取回行的店铺归属校验（平台放行；店铺账号仅本店）。
pub fn ensure_row_shop(auth: &AuthUser, row_shop_id: i32) -> Result<()> {
    auth.ensure_shop_access(row_shop_id)
}

/// [`ensure_row_shop`] 的 `Option` 版本（`None`/0 视为平台行）。
pub fn ensure_row_shop_opt(auth: &AuthUser, row_shop_id: Option<i32>) -> Result<()> {
    auth.ensure_shop_access(row_shop_id.unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth(admin_type: Option<&str>, shop_id: Option<i32>) -> AuthUser {
        AuthUser {
            id: 1,
            username: "tester".into(),
            role_id: Some(1),
            admin_type: admin_type.map(str::to_string),
            shop_id,
        }
    }

    #[test]
    fn list_scope_forces_shop_account_and_rejects_other_shop() {
        let shop = auth(Some("shop"), Some(7));
        assert_eq!(list_scope(&shop, None).unwrap(), Some(7));
        assert_eq!(list_scope(&shop, Some(7)).unwrap(), Some(7));
        // 店铺账号传他店 shopId → 403
        let err = list_scope(&shop, Some(9)).unwrap_err();
        assert!(format!("{err}").contains("无权访问店铺 9"));
    }

    #[test]
    fn list_scope_allows_platform_filter_or_none() {
        let platform = auth(Some("admin"), Some(0));
        assert_eq!(list_scope(&platform, None).unwrap(), None);
        assert_eq!(list_scope(&platform, Some(9)).unwrap(), Some(9));
    }

    #[test]
    fn write_scope_forces_own_shop_and_platform_requires_explicit() {
        let shop = auth(Some("pos"), Some(3));
        assert_eq!(write_scope(&shop, None).unwrap(), 3);
        assert_eq!(write_scope(&shop, Some(3)).unwrap(), 3);
        assert!(write_scope(&shop, Some(5)).is_err());

        let platform = auth(Some("admin"), Some(0));
        assert!(write_scope(&platform, None).is_err());
        assert_eq!(write_scope(&platform, Some(5)).unwrap(), 5);
    }

    #[test]
    fn platform_or_shop_id_returns_platform_zero() {
        assert_eq!(platform_or_shop_id(&auth(Some("admin"), Some(0))), 0);
        assert_eq!(platform_or_shop_id(&auth(Some("shop"), Some(8))), 8);
    }

    #[test]
    fn ensure_row_shop_blocks_cross_shop_reads() {
        let shop = auth(Some("shop"), Some(7));
        assert!(ensure_row_shop(&shop, 7).is_ok());
        assert!(ensure_row_shop(&shop, 8).is_err());
        assert!(ensure_row_shop_opt(&shop, None).is_err());
        let platform = auth(Some("admin"), Some(0));
        assert!(ensure_row_shop(&platform, 8).is_ok());
        assert!(ensure_row_shop_opt(&platform, None).is_ok());
    }
}
