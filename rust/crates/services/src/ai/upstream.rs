//! OpenAI 兼容上游流式调用（共享层，2026-11-21 自 admin 引擎下沉）。
//!
//! 只负责「把一轮对话发给上游并收全」：连接阶段重试（429/5xx/网络错误最多
//! 3 次、一旦出流不再重试）、SSE 增量解析、工具调用分片拼装、usage 捕获。
//! 消息落库、SSE 下发、用量记账由调用方按各自场景处理；文本增量经 `deltas`
//! 通道实时透出，调用方自行转发/节流。错误文案与下沉前逐字一致
//! （`上游返回 {status}：{brief}` / `连接上游失败：{e}`）。

use std::sync::atomic::{AtomicBool, Ordering};

use futures::SinkExt;
use serde_json::{json, Value};

/// 一次工具调用（上游分片拼装后的完整形态）。
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: Value,
}

/// 一轮上游调用的完整结果。
#[derive(Debug, Clone, Default)]
pub struct RoundOutcome {
    /// 助手文本（可能为空，如纯工具调用轮）。
    pub text: String,
    /// 本轮工具调用（为空 = 纯文本轮）。
    pub calls: Vec<ToolCall>,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    /// 读取上游流时中断（网络层）。
    pub stream_broken: bool,
    /// 被 stop_flag 叫停。
    pub stopped: bool,
}

/// 执行一轮流式对话补全。
///
/// `tools` 为 OpenAI 格式的 function 列表（可为空数组 = 不给工具）。
/// `stop_flag` 置位后在下一个网络分片处停读并以 `stopped` 收口。
/// `id_seed` 仅用于上游未给工具调用 id 时的兜底 id 拼装
/// （`call_{id_seed}_{index}`，调用方传本轮 assistant 消息 id，与下沉前一致）。
///
/// # Errors
/// - 连接/上游错误（重试耗尽）⇒ 与下沉前一致的中文文案。
pub async fn stream_completion(
    base_url: &str,
    api_key: &str,
    model: &str,
    messages: &[Value],
    tools: &[Value],
    stop_flag: &AtomicBool,
    id_seed: i64,
    deltas: &mut futures::channel::mpsc::Sender<String>,
) -> Result<RoundOutcome, String> {
    let payload = json!({
        "model": model,
        "messages": messages,
        "tools": tools,
        "stream": true,
        "stream_options": { "include_usage": true },
        "temperature": 0.3,
    });
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .connect_timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP 客户端初始化失败：{e}"))?;
    // 连接阶段重试（429/5xx/网络错误，最多 3 次）；一旦开始出流不再重试。
    let mut attempt = 0;
    let mut resp = loop {
        attempt += 1;
        match client.post(&url).bearer_auth(api_key).json(&payload).send().await {
            Ok(r) if r.status().is_success() => break r,
            Ok(r) => {
                let status = r.status();
                if (status.as_u16() == 429 || status.is_server_error()) && attempt < 3 {
                    tokio::time::sleep(std::time::Duration::from_millis(600 * attempt)).await;
                    continue;
                }
                let body = r.text().await.unwrap_or_default();
                let brief: String = body.chars().take(300).collect();
                return Err(format!("上游返回 {status}：{brief}"));
            }
            Err(e) => {
                if attempt < 3 {
                    tokio::time::sleep(std::time::Duration::from_millis(600 * attempt)).await;
                    continue;
                }
                return Err(format!("连接上游失败：{e}"));
            }
        }
    };

    let mut buf: Vec<u8> = Vec::new();
    let mut out = RoundOutcome::default();
    let mut calls_map: std::collections::BTreeMap<i64, (String, String, String)> =
        std::collections::BTreeMap::new();
    'read: loop {
        if stop_flag.load(Ordering::Relaxed) {
            out.stopped = true;
            break 'read;
        }
        let chunk = match resp.chunk().await {
            Ok(Some(c)) => c,
            Ok(None) => break 'read,
            Err(_) => {
                out.stream_broken = true;
                break 'read;
            }
        };
        buf.extend_from_slice(&chunk);
        while let Some(pos) = buf.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line);
            let line = line.trim();
            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data == "[DONE]" {
                break 'read;
            }
            let Ok(v) = serde_json::from_str::<Value>(data) else {
                continue;
            };
            if let Some(u) = v.get("usage").filter(|u| !u.is_null()) {
                out.prompt_tokens = u.get("prompt_tokens").and_then(Value::as_i64).unwrap_or(0) as i32;
                out.completion_tokens = u
                    .get("completion_tokens")
                    .and_then(Value::as_i64)
                    .unwrap_or(0) as i32;
            }
            let delta = v.pointer("/choices/0/delta").cloned().unwrap_or(Value::Null);
            if let Some(content) = delta.get("content").and_then(Value::as_str) {
                if !content.is_empty() {
                    out.text.push_str(content);
                    let _ = deltas.send(content.to_string()).await;
                }
            }
            if let Some(tcs) = delta.get("tool_calls").and_then(Value::as_array) {
                for tc in tcs {
                    let idx = tc.get("index").and_then(Value::as_i64).unwrap_or(0);
                    let entry = calls_map.entry(idx).or_default();
                    if let Some(id) = tc.get("id").and_then(Value::as_str) {
                        if entry.0.is_empty() {
                            entry.0 = id.to_string();
                        }
                    }
                    if let Some(name) = tc.pointer("/function/name").and_then(Value::as_str) {
                        entry.1.push_str(name);
                    }
                    if let Some(args) = tc.pointer("/function/arguments").and_then(Value::as_str) {
                        entry.2.push_str(args);
                    }
                }
            }
        }
    }
    if stop_flag.load(Ordering::Relaxed) {
        out.stopped = true;
    }
    // 收口：按 index 序拼装，丢弃无名分片、args 解析失败回 {}（口径同下沉前）。
    for (idx, (id, name, args_buf)) in &calls_map {
        if name.is_empty() {
            continue;
        }
        let args: Value = serde_json::from_str(args_buf).unwrap_or_else(|_| json!({}));
        out.calls.push(ToolCall {
            id: if id.is_empty() {
                format!("call_{id_seed}_{idx}")
            } else {
                id.clone()
            },
            name: name.clone(),
            args,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_default_is_empty() {
        let o = RoundOutcome::default();
        assert!(o.text.is_empty());
        assert!(o.calls.is_empty());
        assert_eq!(o.prompt_tokens, 0);
        assert!(!o.stream_broken && !o.stopped);
    }
}
