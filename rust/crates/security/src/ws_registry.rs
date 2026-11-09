//! 进程内 WebSocket 会话注册表（对应 Java `WebSocketServerHandler.USER_SESSIONS`）。
//!
//! Java 侧用 `ConcurrentHashMap<String, Set<WebSocketSession>>` 维护 `userId` 维度的
//! 多端连接（PC + 移动端）。Rust 侧用 `mpsc::UnboundedSender<String>` 表示一个连接，
//! 由连接任务把消息写入 axum WebSocket sink。
//!
//! 注意：进程内状态，仅适用于单实例部署；多实例需换 Redis pub/sub（接口保持不变）。
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use tokio::sync::mpsc::UnboundedSender;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static SESSIONS: OnceLock<Mutex<HashMap<String, Vec<Session>>>> = OnceLock::new();

struct Session {
    id: u64,
    tx: UnboundedSender<String>,
}

fn sessions() -> &'static Mutex<HashMap<String, Vec<Session>>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 用户键：与 Java `{isAdmin}::{userId}` 完全一致。
#[must_use]
pub fn user_key(is_admin: bool, user_id: &str) -> String {
    format!("{is_admin}::{user_id}")
}

/// 注册一个连接，返回会话 ID（用于断开时精确移除）。
pub fn register(key: String, tx: UnboundedSender<String>) -> u64 {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut map) = sessions().lock() {
        map.entry(key).or_default().push(Session { id, tx });
    }
    id
}

/// 移除连接；该用户无剩余连接时删除 key。
pub fn unregister(key: &str, session_id: u64) {
    if let Ok(mut map) = sessions().lock() {
        let empty = if let Some(list) = map.get_mut(key) {
            list.retain(|s| s.id != session_id);
            list.is_empty()
        } else {
            false
        };
        if empty {
            map.remove(key);
        }
    }
}

/// 向指定用户的全部在线连接推送文本消息，返回成功投递的连接数。
///
/// 与 Java 一致：用户不在线时返回 0（调用方不报错）。
pub fn send_to_user(key: &str, message: &str) -> usize {
    let Ok(mut map) = sessions().lock() else {
        return 0;
    };
    let Some(list) = map.get_mut(key) else {
        return 0;
    };
    list.retain(|s| !s.tx.is_closed());
    let mut sent = 0;
    for session in list.iter() {
        if session.tx.send(message.to_string()).is_ok() {
            sent += 1;
        }
    }
    if list.is_empty() {
        map.remove(key);
    }
    sent
}

/// 向所有在线管理员连接广播（IM 用户发消息、未分配客服时的兜底投递）。
pub fn send_to_admins(message: &str) -> usize {
    let Ok(mut map) = sessions().lock() else {
        return 0;
    };
    let keys: Vec<String> = map
        .keys()
        .filter(|key| key.starts_with("true::"))
        .cloned()
        .collect();
    let mut sent = 0;
    for key in keys {
        let mut remove = false;
        if let Some(list) = map.get_mut(&key) {
            list.retain(|s| !s.tx.is_closed());
            for session in list.iter() {
                if session.tx.send(message.to_string()).is_ok() {
                    sent += 1;
                }
            }
            remove = list.is_empty();
        }
        if remove {
            map.remove(&key);
        }
    }
    sent
}

/// 指定用户在线连接数。
#[must_use]
pub fn online_session_count(key: &str) -> usize {
    sessions()
        .lock()
        .ok()
        .and_then(|map| map.get(key).map(Vec::len))
        .unwrap_or(0)
}

/// 当前在线用户数（去重后的 key 数量）。
#[must_use]
pub fn online_user_count() -> usize {
    sessions().lock().map(|m| m.len()).unwrap_or(0)
}

/// 当前在线连接总数。
#[must_use]
pub fn online_total_sessions() -> usize {
    sessions()
        .lock()
        .map(|m| m.values().map(Vec::len).sum())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 注册表是进程级共享；涉及登记/广播的用例串行执行，避免并行测试互相收到消息。
    fn registry_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn register_send_unregister_flow() {
        let _guard = registry_test_lock();
        let key = user_key(false, "900001");
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let id = register(key.clone(), tx);
        assert_eq!(online_session_count(&key), 1);
        assert_eq!(send_to_user(&key, "{\"type\":\"heartBeat\"}"), 1);
        assert_eq!(rx.try_recv().unwrap(), "{\"type\":\"heartBeat\"}");
        unregister(&key, id);
        assert_eq!(online_session_count(&key), 0);
        assert_eq!(send_to_user(&key, "x"), 0);
    }

    #[test]
    fn multi_device_delivery() {
        let _guard = registry_test_lock();
        let key = user_key(true, "1");
        let (tx1, mut rx1) = tokio::sync::mpsc::unbounded_channel::<String>();
        let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel::<String>();
        let id1 = register(key.clone(), tx1);
        let id2 = register(key.clone(), tx2);
        assert_eq!(online_session_count(&key), 2);
        assert_eq!(send_to_user(&key, "msg"), 2);
        assert_eq!(rx1.try_recv().unwrap(), "msg");
        assert_eq!(rx2.try_recv().unwrap(), "msg");
        unregister(&key, id1);
        unregister(&key, id2);
        assert_eq!(online_session_count(&key), 0);
    }

    #[test]
    fn broadcast_to_admins_only() {
        let _guard = registry_test_lock();
        let admin_key = user_key(true, "42");
        let member_key = user_key(false, "7");
        let (tx_admin, mut rx_admin) = tokio::sync::mpsc::unbounded_channel::<String>();
        let (tx_member, mut rx_member) = tokio::sync::mpsc::unbounded_channel::<String>();
        let id_a = register(admin_key.clone(), tx_admin);
        let id_m = register(member_key.clone(), tx_member);
        // 注册表是进程级共享：其它并行测试也可能注册管理员会话，故只断言「至少送达本测试的管理员」
        assert!(send_to_admins("hello") >= 1);
        assert_eq!(rx_admin.try_recv().unwrap(), "hello");
        assert!(rx_member.try_recv().is_err());
        unregister(&admin_key, id_a);
        unregister(&member_key, id_m);
    }

    #[test]
    fn user_key_matches_java_format() {
        assert_eq!(user_key(false, "123"), "false::123");
        assert_eq!(user_key(true, "1"), "true::1");
    }
}
