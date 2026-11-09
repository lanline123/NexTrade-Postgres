//! WebSocket 握手与连接处理（`/ws?token=...&platform=...`，对齐 Java `WebSocketServerHandler`）。
//!
//! 协议行为：
//! - 握手参数：`token`（JWT access token）、`platform`（pc/admin/...，仅日志与欢迎语）；
//! - 连接成功：发送 `连接成功！你的 userId 是：{isAdmin}::{userId}，设备：{platform}`；
//! - 客户端消息：原样回显（前端每 15s 发 `{"type":"heartBeat","data":{}}`，Java 也是 echo）；
//! - 服务端推送：`ws_registry::send_to_user`（IM 发送接口复用）。
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::jwt::{JwtService, TokenType};
use crate::ws_registry;

/// 握手查询参数。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct WsQuery {
    pub token: Option<String>,
    pub platform: Option<String>,
}

/// axum 升级入口。
pub async fn handle_upgrade(ws: WebSocketUpgrade, jwt: JwtService, q: WsQuery) -> Response {
    ws.on_upgrade(move |socket| serve(socket, jwt, q))
}

async fn serve(socket: WebSocket, jwt: JwtService, q: WsQuery) {
    let token = q.token.as_deref().map(str::trim).filter(|t| !t.is_empty());
    let Some(token) = token else {
        close_with(socket, "缺少 token").await;
        return;
    };
    let claims = match jwt.decode(token) {
        Ok(claims) => claims,
        Err(_) => {
            close_with(socket, "token 无效或已过期").await;
            return;
        }
    };
    if claims.ttype != TokenType::Access {
        close_with(socket, "需要访问令牌").await;
        return;
    }

    // Java `isAdmin`：管理端 token 为 true，会员 token 为 false。
    let is_admin = !matches!(claims.typ.as_deref(), None | Some("member"));
    let key = ws_registry::user_key(is_admin, &claims.sub);
    let platform = q.platform.unwrap_or_else(|| "unknown".to_string());

    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let self_tx = tx.clone();
    let session_id = ws_registry::register(key.clone(), tx);

    let welcome = format!("连接成功！你的 userId 是：{key}，设备：{platform}");
    if sink.send(Message::Text(welcome.into())).await.is_err() {
        ws_registry::unregister(&key, session_id);
        return;
    }

    let writer = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if sink.send(Message::Text(message.into())).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(message)) = stream.next().await {
        match message {
            // 与 Java 一致：收到文本消息立即回显（心跳保活）
            Message::Text(text) => {
                let _ = self_tx.send(text.to_string());
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    ws_registry::unregister(&key, session_id);
    writer.abort();
}

async fn close_with(mut socket: WebSocket, reason: &str) {
    let _ = socket.send(Message::Text(reason.to_string().into())).await;
    let _ = socket.close().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_defaults() {
        let q = WsQuery::default();
        assert!(q.token.is_none());
        assert!(q.platform.is_none());
        let q: WsQuery =
            serde_json::from_str(r#"{"token":"t","platform":"admin"}"#).unwrap();
        assert_eq!(q.token.as_deref(), Some("t"));
        assert_eq!(q.platform.as_deref(), Some("admin"));
    }
}
