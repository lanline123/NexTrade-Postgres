//! AI 渠道解析（共享层，2026-11-21 自 admin crate 下沉）。
//!
//! 渠道注册表 `ai_channel / ai_channel_key / ai_model` 是平台与商家、运营助手
//! 与买家导购共用的同一份数据（导购评估 §2.1）。本模块收口三件与身份无关的
//! 读路径：密钥解密、渠道+模型解析、渠道端点读取。作用域口径由调用方以
//! `owner_shop_id` 传入（admin 助手传会话租户 shop_id；买家导购恒传 0 = 平台），
//! SQL 与下沉前 admin 实现逐字一致：
//! `shop_id = owner OR (kind = 1 AND shop_id = 0)`（自有渠道 + 平台官方渠道）。

use super::vault;

/// 取渠道现行密钥明文（仅服务端内部调用，永不下发）。
///
/// # Errors
/// - 渠道未配置密钥 / 解密失败 ⇒ 错误文案（与下沉前一致）。
pub async fn load_channel_secret(
    pool: &sqlx::PgPool,
    channel_id: i32,
) -> Result<(String, String), String> {
    let row: Option<(String, String, String, String)> = sqlx::query_as(
        "SELECT key_cipher, key_nonce, extra_cipher, extra_nonce FROM ai_channel_key
         WHERE channel_id = $1 AND status = 1 ORDER BY key_id DESC LIMIT 1",
    )
    .bind(channel_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;
    let Some((kc, kn, ec, en)) = row else {
        return Err("渠道未配置密钥".to_string());
    };
    let key = if kc.is_empty() {
        String::new()
    } else {
        vault::decrypt(&kc, &kn)?
    };
    let extra = if ec.is_empty() {
        String::new()
    } else {
        vault::decrypt(&ec, &en)?
    };
    Ok((key, extra))
}

/// 解析渠道 + 模型：指定 → owner 默认渠道 → 第一个启用渠道 → 官方渠道；
/// 指定模型须在该渠道下存在且启用，未指定取第一个启用的 chat 模型。
///
/// # Errors
/// - 无可用渠道 / 模型不存在 ⇒ 错误文案（与下沉前一致）。
pub async fn resolve_channel_model(
    pool: &sqlx::PgPool,
    owner_shop_id: i32,
    channel_id: Option<i32>,
    model_code: Option<String>,
) -> Result<(i32, String), String> {
    let channel: Option<(i32, i32)> = if let Some(id) = channel_id {
        sqlx::query_as(
            "SELECT channel_id, kind FROM ai_channel
             WHERE channel_id = $1 AND is_del = 0 AND is_enabled = 1
               AND (shop_id = $2 OR (kind = 1 AND shop_id = 0))",
        )
        .bind(id)
        .bind(owner_shop_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
    } else {
        sqlx::query_as(
            "SELECT channel_id, kind FROM ai_channel
             WHERE is_del = 0 AND is_enabled = 1
               AND (shop_id = $1 OR (kind = 1 AND shop_id = 0))
             ORDER BY (shop_id = $1) DESC, is_default DESC, sort, channel_id LIMIT 1",
        )
        .bind(owner_shop_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
    };
    let Some((cid, _kind)) = channel else {
        return Err("没有可用的渠道，请先在「渠道配置」中添加渠道与密钥".to_string());
    };
    if let Some(m) = model_code.filter(|s| !s.trim().is_empty()) {
        let exists: Option<(i32,)> = sqlx::query_as(
            "SELECT model_id FROM ai_model WHERE channel_id = $1 AND model_code = $2 AND is_del = 0 AND is_enabled = 1",
        )
        .bind(cid)
        .bind(&m)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;
        if exists.is_some() {
            return Ok((cid, m));
        }
        return Err("所选模型在该渠道下不存在或未启用".to_string());
    }
    let model: Option<(String,)> = sqlx::query_as(
        "SELECT model_code FROM ai_model WHERE channel_id = $1 AND is_del = 0 AND is_enabled = 1
           AND capabilities LIKE '%chat%' ORDER BY sort, model_id LIMIT 1",
    )
    .bind(cid)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;
    match model {
        Some((m,)) => Ok((cid, m)),
        None => Err("该渠道下没有可用的对话模型，请先在「渠道配置」中添加".to_string()),
    }
}

/// 读取会话绑定渠道的端点信息（base_url / kind / is_enabled），作用域口径同上。
pub async fn load_channel_endpoint(
    pool: &sqlx::PgPool,
    channel_id: i32,
    owner_shop_id: i32,
) -> Option<(String, i32, i32)> {
    sqlx::query_as(
        "SELECT base_url, kind, is_enabled FROM ai_channel
         WHERE channel_id = $1 AND is_del = 0 AND (shop_id = $2 OR (kind = 1 AND shop_id = 0))",
    )
    .bind(channel_id)
    .bind(owner_shop_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

/// 蜂鸟凭据双读（AI 助手 DESIGN §8 P1）：注册表 kind=3 平台渠道优先，
/// 老 config 兜底由调用方（admin `ai_image.rs`）在本函数返回 None 时继续走。
pub async fn fengniao_credentials(pool: &sqlx::PgPool) -> Option<(String, String)> {
    let row: Option<(i32,)> = sqlx::query_as(
        "SELECT channel_id FROM ai_channel
         WHERE kind = 3 AND shop_id = 0 AND is_del = 0 AND is_enabled = 1
         ORDER BY channel_id LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    let (channel_id,) = row?;
    let (authorization, project_id) = load_channel_secret(pool, channel_id).await.ok()?;
    if authorization.is_empty() {
        None
    } else {
        Some((authorization, project_id))
    }
}
