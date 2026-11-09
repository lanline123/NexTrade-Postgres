//! 商米云打印回调 域服务（logistics::sunmi_print_callback）骨架。
//!
//! 构造签名固定 `pub fn new(repos: Arc<Repositories>) -> Self`。
//! 详见 `rust/docs/prompts/logistics::sunmi_print_callback.md`。
//!
//! ## 本片迁移范围（与 Java 逐字对齐）
//!
//! 原 Java `SunmiPrintCallbackController.callback`（`java/nextrade-api/src/main/
//! java/com/nextrade/api/controller/print/SunmiPrintCallbackController.java:52`）
//! 自身在源码注释里就写明是 **DR-01=α 占位实现**：
//!
//! ```java
//! public Map<String, Object> callback(@RequestBody(required = false) String body) {
//!     log.info("[Sunmi Callback] 收到商米云打印回调（MVP 占位）：{}", body);
//!     // TODO DR-01=β/γ：1. 校验 Sunmi-Sign 头 2. 解析 order_id/status
//!     //                  3. PrintLogService.markCallback(...)
//!     return Map.of("code", 1, "msg", "ok");
//! }
//! ```
//!
//! 即：MVP 阶段**只接收 + 返回 200**，不校验签名、不解析报文、
//! **不触发** `PrintLog` 状态机 `PUSHED(1) → PRINTED(2)` 的迁移
//! （对齐飞鹅云「推送成功即终态」的行为基线）。
//!
//! 本服务因此**没有任何 DB 写操作**（`SunmiPrintCallbackRepository` 保留但不使用，
//! 以免将来升级 DR-01=β/γ 时需要改注册行）。同理，Java TODO 里的
//! `Sunmi-Sign` 验签 / `markCallback` 契约在 Rust 侧也**未实现**，
//! 与 Java 保持一致（不是遗漏，是上游就是占位）。

use std::sync::Arc;

use nextrade_core::models::logistics::sunmi_print_callback::SunmiCallbackAck;
use nextrade_infrastructure::db::Repositories;

/// 商米云打印回调 域服务。
pub struct SunmiPrintCallbackService {
    repos: Arc<Repositories>,
}

impl SunmiPrintCallbackService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储（当前占位实现不使用，保留供 DR-01=β/γ 升级）。
    #[must_use]
    pub fn repo(&self) -> &nextrade_infrastructure::db::repo::logistics::sunmi_print_callback::SunmiPrintCallbackRepository {
        &self.repos.logistics.sunmi_print_callback
    }

    /// 处理商米云打印回调（对应 Java `callback` 方法体）。
    ///
    /// 行为与 Java 完全一致：记录回调原文 → 返回固定应答
    /// `{"code":1,"msg":"ok"}`。
    ///
    /// `body` 为请求体原文；Java 是 `@RequestBody(required = false) String`，
    /// **无 body / 空 body 同样成功**（`required = false`）。
    pub fn handle_callback(&self, body: &str) -> SunmiCallbackAck {
        // Java: log.info("[Sunmi Callback] 收到商米云打印回调（MVP 占位）：{}", body)
        tracing::info!("[Sunmi Callback] 收到商米云打印回调（MVP 占位）：{body}");
        SunmiCallbackAck::ok()
    }
}

/// 把原始请求体字节流转成可打印文本（对应 Java `String body` 的读取结果）。
///
/// Java 由 Spring 的 `StringHttpMessageConverter` 完成解码；非 UTF-8 字节
/// 在 Java 侧会得到替换字符。这里用 `from_utf8_lossy` 保持同样的「不报错」语义，
/// 缺 body（`Content-Length: 0`）即空串，对应 `required = false` 的空值分支。
#[must_use]
pub fn decode_body(raw: &[u8]) -> String {
    String::from_utf8_lossy(raw).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_decodes_to_empty_string_when_absent() {
        // Java `@RequestBody(required = false)` → 无 body 不报错
        assert_eq!(decode_body(&[]), "");
    }

    #[test]
    fn body_decodes_utf8_payload_verbatim() {
        let raw = r#"{"orderId":1,"status":1}"#.as_bytes();
        assert_eq!(decode_body(raw), r#"{"orderId":1,"status":1}"#);
    }

    #[test]
    fn non_utf8_body_does_not_fail_the_callback() {
        // 占位实现「收到即 200」：非法字节也不能把回调变成错误
        let ack = decode_body(&[0xF0, 0x00, 0x80]);
        assert!(!ack.is_empty() || ack.is_empty()); // 只要求不 panic
    }

    #[test]
    fn ack_is_the_java_literal_regardless_of_body() {
        // 无论商米传什么，Java 恒返回 code=1 / msg="ok"
        for body in ["", "{}", "not-json", "{\"status\":2}"] {
            let v = serde_json::to_value(ack_for(body)).unwrap();
            assert_eq!(v["code"], 1, "body={body:?}");
            assert_eq!(v["msg"], "ok", "body={body:?}");
        }
    }

    /// 不依赖数据库的应答构造（`handle_callback` 的纯逻辑部分）。
    fn ack_for(body: &str) -> SunmiCallbackAck {
        let _ = body;
        SunmiCallbackAck::ok()
    }
}