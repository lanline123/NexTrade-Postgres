//! 多租户隔离辅助（catalog 管理端 handler 统一入口）。
//!
//! 身份只来自鉴权中间件写入的 [`AuthUser`]（JWT 声明）；禁止把 query/body 的
//! `shopId` 当作身份，也不直接信任客户端 `x-shop-id`。
//!
//! 语义与 Java `HeaderUtils.getShopId()` 对齐：
//! - 平台账号：列表不过滤（`None`）/ 可用请求 `shopId` 筛选；写操作需显式
//!   `shopId`；详情/更新/删除放行；
//! - 店铺账号：列表强制本店；写操作固定本店；跨店访问 403。

use nextrade_core::Result;
use nextrade_security::web::AuthUser;

/// 列表/查询的店铺过滤（见模块头语义）。
pub fn list_scope(auth: &AuthUser, requested: Option<i32>) -> Result<Option<i32>> {
    auth.resolve_shop_scope(requested)
}

/// 写操作的店铺归属（平台必须显式；店铺账号固定本店）。
pub fn write_scope(auth: &AuthUser, requested: Option<i32>) -> Result<i32> {
    auth.require_shop_scope(requested)
}

/// 以 `i32` 店铺维度服务调用：平台 `0`、店铺本店。
pub fn platform_or_shop_id(auth: &AuthUser) -> i32 {
    auth.scoped_shop_id().unwrap_or(0)
}

/// 已取回行的店铺归属校验（平台放行）。
pub fn ensure_row_shop(auth: &AuthUser, row_shop_id: i32) -> Result<()> {
    auth.ensure_shop_access(row_shop_id)
}

/// [`ensure_row_shop`] 的 `Option` 版本。
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
    fn list_scope_forces_shop_filter() {
        let shop = auth(Some("shop"), Some(7));
        assert_eq!(list_scope(&shop, None).unwrap(), Some(7));
        assert!(list_scope(&shop, Some(9)).is_err());
        let platform = auth(Some("admin"), Some(0));
        assert_eq!(list_scope(&platform, None).unwrap(), None);
        assert_eq!(list_scope(&platform, Some(9)).unwrap(), Some(9));
    }

    #[test]
    fn write_scope_prevents_cross_shop_writes() {
        let shop = auth(Some("merchant"), Some(4));
        assert_eq!(write_scope(&shop, None).unwrap(), 4);
        assert!(write_scope(&shop, Some(6)).is_err());
        let platform = auth(Some("admin"), Some(0));
        assert!(write_scope(&platform, None).is_err());
        assert_eq!(write_scope(&platform, Some(6)).unwrap(), 6);
    }

    #[test]
    fn row_ownership_checks() {
        let shop = auth(Some("shop"), Some(4));
        assert!(ensure_row_shop(&shop, 4).is_ok());
        assert!(ensure_row_shop(&shop, 5).is_err());
        assert!(ensure_row_shop_opt(&shop, None).is_err());
        assert_eq!(platform_or_shop_id(&shop), 4);
        let platform = auth(Some("admin"), Some(0));
        assert_eq!(platform_or_shop_id(&platform), 0);
        assert!(ensure_row_shop(&platform, 5).is_ok());
    }
}
