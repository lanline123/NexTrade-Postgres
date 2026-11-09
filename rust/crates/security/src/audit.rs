//! 请求级审计上下文（管理端操作日志用）。
//!
//! `require_auth` 中间件在请求进入时把当前管理员 id 与客户端 IP 放入
//! `tokio::task_local`；业务服务写 `admin_log` 时通过 [`current`] 读取，
//! 避免在每层方法签名里透传 actor，也避免把 user_id/IP 写死。

use std::future::Future;

/// 当前请求的操作者信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditActor {
    /// 管理员/会员 ID（JWT `sub`）。
    pub admin_id: i32,
    /// 客户端 IP（`X-Forwarded-For` / `X-Real-IP` / `unknown`）。
    pub ip: String,
}

tokio::task_local! {
    static CURRENT_ACTOR: AuditActor;
}

/// 在指定操作者上下文中执行 future。
pub async fn with_actor<F: Future>(actor: AuditActor, future: F) -> F::Output {
    CURRENT_ACTOR.scope(actor, future).await
}

/// 读取当前请求的操作者（不在请求上下文时返回 None）。
#[must_use]
pub fn current() -> Option<AuditActor> {
    CURRENT_ACTOR.try_with(Clone::clone).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn actor_is_visible_inside_scope_only() {
        assert!(current().is_none());
        let actor = AuditActor {
            admin_id: 7,
            ip: "1.2.3.4".into(),
        };
        let seen = with_actor(actor.clone(), async { current() }).await;
        assert_eq!(seen, Some(actor));
        assert!(current().is_none());
    }
}
