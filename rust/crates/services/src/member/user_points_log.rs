//! 会员积分日志 域服务（member::user_points_log）。
//!
//! 构造签名固定 `pub fn new(repos: Arc<Repositories>) -> Self`。
//! 详见 `rust/docs/prompts/member::user_points_log.md`。
//!
//! # 唯一端点
//!
//! | 路由 | 原 Java | 本文件 |
//! |------|---------|--------|
//! | `GET /adminapi/user/userPointsLog/getPoints` | `UserPointsLogController.getPoints` → `UserPointsLogServiceImpl#getPoints`（`:59-66`） | [`UserPointsLogService::get_points`] |
//!
//! # 与 Java 的差异（有意为之）
//!
//! 1. **身份来源**：Java 用 `SecurityUtils.getCurrentAdminId()`（会话 / JWT），
//!    再把它当作 `user.user_id` 去查会员行 —— 注意这**不是**「当前会员」，
//!    而是「与该后台账号 ID 同号的会员记录」的积分。Rust 侧同样把
//!    JWT 的 `AuthUser.id` 传给 `WHERE user_id = ?`，保持这个（略显奇怪的）语义。
//! 2. **查不到行**：Java 不判空 → `currentUser.getPoints()` NPE →
//!    `GlobalExceptionHandler` 返回 `code=500, msg=<NPE message，常为 null>`。
//!    Rust 改为显式业务错误（HTTP 200 + `code:500` + 可读文案），
//!    见 [`USER_POINTS_NOT_FOUND_MSG`]。
//! 3. **权限矩阵未迁移**：Java 类级
//!    `@PreAuthorize("@pms.hasPermission('integralLogManage')")`；Rust 侧
//!    `/adminapi` 子树只套 `require_auth`（登录态），与 product_manage/002/005/012/027
//!    的既有缺口一致。
//! 4. **返回裸标量**：Java 返回 `Integer`，故 `data` 是整数而不是对象，
//!    服务层返回 `i32`，路由层直接 `ok(i32)`。

use std::sync::Arc;

use nextrade_core::models::member::user_points_log::USER_POINTS_NOT_FOUND_MSG;
use nextrade_core::Result;
use nextrade_infrastructure::db::repo::member::user_points_log::UserPointsLogRepository;
use nextrade_infrastructure::db::Repositories;

/// 会员积分日志 域服务。
pub struct UserPointsLogService {
    repos: Arc<Repositories>,
}

impl UserPointsLogService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &UserPointsLogRepository {
        &self.repos.member.user_points_log
    }

    /// 当前登录管理员所绑定会员的积分（`getPoints`）。
    ///
    /// `admin_user_id` = Java `SecurityUtils.getCurrentAdminId()`
    /// = Rust `AuthUser.id`（JWT `sub`）。
    ///
    /// # Errors
    /// - 该后台账号在 `user` 表无对应会员行：`Error::Biz{code:500}`
    ///   （Java 在该处 NPE，见模块头差异 2）；
    /// - 数据库错误原样向上抛出。
    pub async fn get_points(&self, admin_user_id: i32) -> Result<i32> {
        self.repo()
            .points_of_user(admin_user_id)
            .await?
            .ok_or_else(|| nextrade_core::Error::biz_msg(USER_POINTS_NOT_FOUND_MSG))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_user_row_maps_to_java_global_exception_shape() {
        // Java：NPE → AjaxResult.error(SERVICE_DATA_ERROR=500, e.getMessage())
        // Rust：Biz{code:500} + 可读中文文案（HTTP 仍 200，body code 500）
        let err = nextrade_core::Error::biz_msg(USER_POINTS_NOT_FOUND_MSG);
        // 按引用匹配，避免把 `err` 移走后第 89 行继续使用（E0382）
        match &err {
            nextrade_core::Error::Biz { code, msg } => {
                assert_eq!(*code, 500);
                assert_eq!(msg, USER_POINTS_NOT_FOUND_MSG);
            }
            other => panic!("应为 Biz，实际 {other:?}"),
        }
        // `Error::Biz` 的非 4xx/5xx 业务码走 HTTP 200（`status_code()` 的
        // `_ => StatusCode::OK` 分支），这里用 `business_code()`（无需依赖
        // axum 的 StatusCode 类型）核对错误体里的 code
        assert_eq!(err.business_code(), 500);
        assert_eq!(
            err.to_string(),
            format!("业务错误 [500]: {USER_POINTS_NOT_FOUND_MSG}"),
            "错误文案必须原样透出（对齐 Java 的 msg 字段）"
        );
    }

    #[test]
    fn service_and_repo_handles_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<UserPointsLogService>();
        assert_send_sync::<UserPointsLogRepository>();
    }
}