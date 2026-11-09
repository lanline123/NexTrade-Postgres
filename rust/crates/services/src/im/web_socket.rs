//! 消息控制器（内部接口 / WebSocket 推送）域服务（im::web_socket）。
//!
//! 构造签名固定 `pub fn new(repos: Arc<Repositories>) -> Self`。
//! 详见 `rust/docs/prompts/im::web_socket.md`。
//!
//! ## WebSocket 推送（已迁移）
//!
//! Rust 侧提供 `/ws?token=...&platform=...` 握手端点
//! （`nextrade_security::ws::handle_upgrade`）与进程内会话注册表
//! `nextrade_security::ws_registry`。
//! [`WebSocketService::send_message`] 通过 `send_to_user` 向在线用户推送，
//! 用户不在线时与 Java 一样只记录日志并返回成功文案。
//!
//! ## 内部密钥门禁（已迁移，2026-10-25）
//!
//! Java `InternalApiFilter` 要求 `/api/socket/send` 携带 `X-Internal-Api-Key`
//! 请求头。Rust 侧已在路由层实现同等校验
//! （`nextrade_security::web::require_internal_api_key`，密钥取
//! `config.yaml` 的 `internal.api_key` / env `TIG_INTERNAL__API_KEY`）：
//! 未配置密钥 → 403；缺头/密钥不符 → 401。

use std::sync::Arc;

use nextrade_core::models::im::web_socket::validate_query;
use nextrade_core::models::im::web_socket::SocketSendQuery;
use nextrade_core::Result;
use nextrade_infrastructure::db::Repositories;

/// 消息控制器（内部接口）域服务。
pub struct WebSocketService {
    repos: Arc<Repositories>,
}

impl WebSocketService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储（当前实现不落库，保留以固定构造签名）。
    #[must_use]
    pub fn repo(&self) -> &nextrade_infrastructure::db::repo::im::web_socket::WebSocketRepository {
        &self.repos.im.web_socket
    }

    /// 服务器端主动发消息给指定用户（对应 Java `WebSocketController.sendMessage`）。
    ///
    /// 行为对照：
    /// - 参数缺失 → 业务码 `1001`「接口参数缺失」（Java
    ///   `MissingServletRequestParameterException` 分支）；
    /// - 参数齐备 → 返回 `"已发送消息给用户：" + targetUserId`（Java `:69`），
    ///   与 Java 一样**不区分在线/离线**（Java 离线也只是打印日志后同样返回成功文案）。
    ///
    /// ⚠ 推送本身未迁移，见模块头「WebSocket 推送通道**未迁移**」。
    pub fn send_message(&self, q: &SocketSendQuery) -> Result<String> {
        let (target_user_id, message) = validate_query(q)?;
        let sent = nextrade_security::ws_registry::send_to_user(&target_user_id, &message);
        if sent == 0 {
            tracing::warn!(target_user_id = %target_user_id, "WebSocket 推送：用户不在线（Java 同样只记日志）");
        } else {
            tracing::info!(target_user_id = %target_user_id, sent, "WebSocket 推送成功");
        }
        Ok(success_text(&target_user_id))
    }
}

/// Java `WebSocketServerHandler#sendMessageToUser` 里「用户不在线」的判定位置。
/// 本构建始终视为不在线——保留该纯函数以便单测锁定文案，将来接上会话表时
/// 只需让它返回真实的在线会话数。
#[must_use]
pub fn online_session_count(target_user_id: &str) -> usize {
    nextrade_security::ws_registry::online_session_count(target_user_id)
}

/// 成功应答文案（对应 Java `"已发送消息给用户：" + targetUserId`，`:69`）。
#[must_use]
pub fn success_text(target_user_id: &str) -> String {
    format!("已发送消息给用户：{target_user_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_text_is_the_java_literal() {
        assert_eq!(
            success_text("false::123"),
            "已发送消息给用户：false::123"
        );
        assert_eq!(success_text("true::1"), "已发送消息给用户：true::1");
        // 空串也要原样拼进文案（Java 无 @NotBlank）
        assert_eq!(success_text(""), "已发送消息给用户：");
    }

    #[test]
    fn online_session_count_is_always_zero_until_ws_migrated() {
        // 锁定「推送未迁移」这一事实：任何用户都视为离线
        assert_eq!(online_session_count("false::1"), 0);
        assert_eq!(online_session_count(""), 0);
    }

    #[test]
    fn missing_params_surface_java_error_code_and_msg() {
        let q = SocketSendQuery {
            target_user_id: Some("true::1".into()),
            message: None,
        };
        let err = validate_query(&q).unwrap_err();
        match err {
            nextrade_core::Error::Biz { code, msg } => {
                assert_eq!(code, 1001);
                assert_eq!(msg, "接口参数缺失");
            }
            other => panic!("期望 Biz，得到 {other:?}"),
        }
    }
}