//! 管理端租户辅助（共享）：平台专属中间件与判定。
//!
//! 身份只来自认证中间件注入的 [`AuthUser`]（JWT 声明），禁止 query/header 作为身份。
//! 适用于「源表没有 `shop_id` 列、无法按店铺隔离」的平台级配置/运维域：
//! 整域挂在 [`platform_only`] 上，非平台账号（店铺/门店/供应商）一律 403。

use axum::{
    extract::{Extension, Request},
    middleware::Next,
    response::Response,
};
use nextrade_security::web::{AppError, AuthUser};

/// 平台专属中间件：仅平台管理员可通过。
///
/// 用法（路由构建器末端）：
/// ```ignore
/// Router::new()
///     .route(...)
///     .layer(middleware::from_fn(platform_only))
/// ```
pub async fn platform_only(
    Extension(auth): Extension<AuthUser>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    require_platform(&auth)?;
    Ok(next.run(req).await)
}

/// 平台账号判定（供 handler 内的写操作与单测复用）。
pub fn require_platform(auth: &AuthUser) -> Result<(), AppError> {
    if auth.is_platform() {
        Ok(())
    } else {
        Err(AppError(nextrade_core::Error::Forbidden(
            "仅平台账号可访问该系统配置".into(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_guard_allows_admin_only() {
        let admin = AuthUser::new(1, "a".into(), None, Some("admin".into()), Some(0));
        assert!(require_platform(&admin).is_ok());
        for (t, shop) in [("shop", Some(7)), ("store", Some(7)), ("pos", Some(7))] {
            let auth = AuthUser::new(2, "x".into(), None, Some(t.into()), shop);
            assert!(require_platform(&auth).is_err(), "t={t}");
        }
        // 供应商同样不是平台
        let vendor = AuthUser::new(3, "v".into(), None, Some("vendor".into()), None);
        assert!(require_platform(&vendor).is_err());
    }
}
