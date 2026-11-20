//! AI 共享层（2026-11-21 随买家导购立项建立）。
//!
//! 渠道注册表、密钥库、上游调用与用量台账原先只存在于 admin 二进制的运营
//! 助手引擎里（以管理员身份 `AiTenant` 为上下文）。买家导购（public 二进制）
//! 需要同一套底座，故下沉到 services：
//! - `vault`：AES-256-GCM 密钥库（自 admin 整文件迁入，代码不变）；
//! - `channel`：渠道/模型解析与密钥读取（作用域以 owner_shop_id 参数化）；
//! - `upstream`：OpenAI 兼容上游流式调用（连接重试 + SSE 解析）；
//! - `usage`：用量台账写入（admin/guide 双场景维度）；
//! - `guide`：买家导购引擎（只读工具 + 价格红线 + 三级预算熔断）。
//!
//! admin 运营助手改为调用本层，行为不变（见各模块的口径注记）。

pub mod channel;
pub mod guide;
pub mod upstream;
pub mod usage;
pub mod vault;

/// 当前 Unix 秒（本地各 AI 表的 create_time/update_time 口径：epoch 秒）。
#[must_use]
pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
