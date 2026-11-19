//! 渠道注册表 + 模型管理 + 加密密钥写入（DESIGN §1.2 / §2.2）。
//!
//! 可见范围（硬性）：渠道只按 owner（会话租户）读写——商家只见自有渠道 +
//! 官方渠道只读条目；平台账号在列表里**看不到**商家渠道（含打码），
//! 商家看不到平台密钥的任何信息。密钥只写不读，读接口只回打码。

use axum::{
    extract::{Extension, Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use nextrade_security::web::{ok, ApiResult, AuthUser};

use super::{err_forbidden, err_validation, now_secs, vault, AiTenant};
use crate::AdminState;

// ————————————————————— 行映射 —————————————————————

#[derive(sqlx::FromRow)]
struct ChannelRow {
    channel_id: i32,
    shop_id: i32,
    name: String,
    kind: i32,
    base_url: String,
    is_enabled: i32,
    is_default: i32,
    sort: i32,
}

#[derive(sqlx::FromRow)]
struct ModelRow {
    model_id: i32,
    channel_id: i32,
    model_code: String,
    display_name: String,
    capabilities: String,
    context_tokens: i32,
    is_enabled: i32,
    sort: i32,
}

async fn load_key_masks(pool: &sqlx::PgPool, channel_id: i32) -> (String, String) {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT key_mask, extra_mask FROM ai_channel_key
         WHERE channel_id = $1 AND status = 1 ORDER BY key_id DESC LIMIT 1",
    )
    .bind(channel_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    row.unwrap_or_default()
}

fn model_vo(m: &ModelRow) -> Value {
    json!({
        "modelId": m.model_id,
        "channelId": m.channel_id,
        "modelCode": m.model_code,
        "displayName": m.display_name,
        "capabilities": m.capabilities,
        "contextTokens": m.context_tokens,
        "isEnabled": m.is_enabled == 1,
        "sort": m.sort,
    })
}

async fn channel_vo(pool: &sqlx::PgPool, ch: &ChannelRow, official_readonly: bool) -> Value {
    let models: Vec<ModelRow> = sqlx::query_as(
        "SELECT model_id, channel_id, model_code, display_name, capabilities, context_tokens, is_enabled, sort
         FROM ai_model WHERE channel_id = $1 AND is_del = 0 ORDER BY sort, model_id",
    )
    .bind(ch.channel_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    // 官方渠道对商家只下发名称与模型，密钥信息一律不下发（连打码都不给）。
    let (key_mask, extra_mask) = if official_readonly {
        (String::new(), String::new())
    } else {
        load_key_masks(pool, ch.channel_id).await
    };
    json!({
        "channelId": ch.channel_id,
        "shopId": ch.shop_id,
        "name": ch.name,
        "kind": ch.kind,
        "baseUrl": ch.base_url,
        "isEnabled": ch.is_enabled == 1,
        "isDefault": ch.is_default == 1,
        "sort": ch.sort,
        "keyMask": key_mask,
        "extraMask": extra_mask,
        "hasKey": !key_mask.is_empty() || !extra_mask.is_empty(),
        "official": official_readonly,
        "models": models.iter().map(model_vo).collect::<Vec<_>>(),
    })
}

/// 取当前租户自有渠道（owner 必须与会话租户一致），否则按不存在处理。
async fn load_own_channel(
    state: &AdminState,
    tenant: &AiTenant,
    channel_id: i32,
) -> Result<ChannelRow, nextrade_security::web::AppError> {
    let row: Option<ChannelRow> = sqlx::query_as(
        "SELECT channel_id, shop_id, name, kind, base_url, is_enabled, is_default, sort
         FROM ai_channel WHERE channel_id = $1 AND shop_id = $2 AND is_del = 0",
    )
    .bind(channel_id)
    .bind(tenant.shop_id)
    .fetch_optional(&state.repos.pool)
    .await
    .map_err(|e| err_validation(e.to_string()))?;
    row.ok_or_else(|| err_validation("渠道不存在"))
}

/// 取渠道现行密钥明文（仅服务端内部调用，永不下发）。
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

/// 保存/轮换密钥：旧现行行置为已轮换，新行加密落库。
async fn store_key(
    pool: &sqlx::PgPool,
    channel_id: i32,
    api_key: Option<&str>,
    extra_key: Option<&str>,
) -> Result<(), String> {
    let (existing_key, existing_extra) = load_channel_secret(pool, channel_id)
        .await
        .unwrap_or_default();
    let final_key = api_key
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or(existing_key);
    let final_extra = extra_key
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or(existing_extra);
    if final_key.is_empty() && final_extra.is_empty() {
        return Ok(());
    }
    let (kc, kn) = if final_key.is_empty() {
        (String::new(), String::new())
    } else {
        vault::encrypt(&final_key)?
    };
    let (ec, en) = if final_extra.is_empty() {
        (String::new(), String::new())
    } else {
        vault::encrypt(&final_extra)?
    };
    sqlx::query("UPDATE ai_channel_key SET status = 0 WHERE channel_id = $1 AND status = 1")
        .bind(channel_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query(
        "INSERT INTO ai_channel_key (channel_id, key_cipher, key_nonce, key_mask, extra_cipher, extra_nonce, extra_mask, status, create_time)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 1, $8)",
    )
    .bind(channel_id)
    .bind(kc)
    .bind(kn)
    .bind(vault::mask_of(&final_key))
    .bind(ec)
    .bind(en)
    .bind(vault::mask_of(&final_extra))
    .bind(now_secs())
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 蜂鸟凭据双读（DESIGN §8 P1）：注册表 kind=3 平台渠道优先，老 config 兜底
/// 由调用方（ai_image.rs）在本函数返回 None 时继续走。
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

// ————————————————————— 预设 —————————————————————

pub fn channel_presets() -> Value {
    json!([
        { "name": "DeepSeek", "kind": 2, "baseUrl": "https://api.deepseek.com", "models": [
            { "modelCode": "deepseek-chat", "displayName": "DeepSeek Chat (V3)", "capabilities": "chat", "contextTokens": 64000 },
            { "modelCode": "deepseek-reasoner", "displayName": "DeepSeek Reasoner (R1)", "capabilities": "chat", "contextTokens": 64000 }
        ]},
        { "name": "OpenAI", "kind": 2, "baseUrl": "https://api.openai.com/v1", "models": [
            { "modelCode": "gpt-4o", "displayName": "GPT-4o", "capabilities": "chat,vision", "contextTokens": 128000 },
            { "modelCode": "gpt-4o-mini", "displayName": "GPT-4o mini", "capabilities": "chat,vision", "contextTokens": 128000 }
        ]},
        { "name": "通义千问（DashScope 兼容）", "kind": 2, "baseUrl": "https://dashscope.aliyuncs.com/compatible-mode/v1", "models": [
            { "modelCode": "qwen-plus", "displayName": "Qwen Plus", "capabilities": "chat", "contextTokens": 128000 },
            { "modelCode": "qwen-max", "displayName": "Qwen Max", "capabilities": "chat", "contextTokens": 32000 }
        ]},
        { "name": "智谱 GLM", "kind": 2, "baseUrl": "https://open.bigmodel.cn/api/paas/v4", "models": [
            { "modelCode": "glm-4-plus", "displayName": "GLM-4 Plus", "capabilities": "chat", "contextTokens": 128000 },
            { "modelCode": "glm-4-flash", "displayName": "GLM-4 Flash", "capabilities": "chat", "contextTokens": 128000 }
        ]},
        { "name": "Moonshot（Kimi）", "kind": 2, "baseUrl": "https://api.moonshot.cn/v1", "models": [
            { "modelCode": "moonshot-v1-8k", "displayName": "Moonshot v1 8k", "capabilities": "chat", "contextTokens": 8000 },
            { "modelCode": "moonshot-v1-32k", "displayName": "Moonshot v1 32k", "capabilities": "chat", "contextTokens": 32000 }
        ]},
        { "name": "硅基流动", "kind": 2, "baseUrl": "https://api.siliconflow.cn/v1", "models": [
            { "modelCode": "deepseek-ai/DeepSeek-V3", "displayName": "DeepSeek V3（硅基流动）", "capabilities": "chat", "contextTokens": 64000 },
            { "modelCode": "Qwen/Qwen2.5-72B-Instruct", "displayName": "Qwen2.5 72B（硅基流动）", "capabilities": "chat", "contextTokens": 32000 }
        ]},
        { "name": "火山方舟", "kind": 2, "baseUrl": "https://ark.cn-beijing.volces.com/api/v3", "models": [
            { "modelCode": "doubao-seed-1-6-251015", "displayName": "豆包 Seed 1.6", "capabilities": "chat,vision", "contextTokens": 128000 }
        ]}
    ])
}

pub async fn presets(
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<Value> {
    AiTenant::from_auth(&auth)?;
    ok(json!({ "presets": channel_presets() }))
}

// ————————————————————— 渠道 CRUD —————————————————————

pub async fn list(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<Value> {
    let tenant = AiTenant::from_auth(&auth)?;
    let pool = &state.repos.pool;
    let own: Vec<ChannelRow> = sqlx::query_as(
        "SELECT channel_id, shop_id, name, kind, base_url, is_enabled, is_default, sort
         FROM ai_channel WHERE shop_id = $1 AND is_del = 0 ORDER BY sort, channel_id",
    )
    .bind(tenant.shop_id)
    .fetch_all(pool)
    .await
    .map_err(|e| err_validation(e.to_string()))?;
    let mut out = Vec::new();
    for ch in &own {
        out.push(channel_vo(pool, ch, false).await);
    }
    // 商家视角：官方渠道只读条目（名称 + 模型，无任何密钥信息）。
    if tenant.is_merchant() {
        let official: Vec<ChannelRow> = sqlx::query_as(
            "SELECT channel_id, shop_id, name, kind, base_url, is_enabled, is_default, sort
             FROM ai_channel WHERE shop_id = 0 AND kind = 1 AND is_del = 0 ORDER BY sort, channel_id",
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();
        for ch in &official {
            out.push(channel_vo(pool, ch, true).await);
        }
    }
    ok(json!({ "channels": out }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ChannelBody {
    pub name: String,
    pub kind: i32,
    pub base_url: String,
    pub api_key: Option<String>,
    pub extra_key: Option<String>,
    pub is_enabled: Option<bool>,
    pub sort: Option<i32>,
    pub models: Option<Vec<ModelBody>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ModelBody {
    pub model_id: Option<i32>,
    pub model_code: String,
    pub display_name: Option<String>,
    pub capabilities: Option<String>,
    pub context_tokens: Option<i32>,
    pub is_enabled: Option<bool>,
    pub sort: Option<i32>,
}

pub async fn create(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<ChannelBody>,
) -> ApiResult<Value> {
    let tenant = AiTenant::from_auth(&auth)?;
    if body.name.trim().is_empty() {
        return Err(err_validation("渠道名称不能为空"));
    }
    if body.kind == 1 && tenant.is_merchant() {
        return Err(err_forbidden("官方渠道只能由平台维护"));
    }
    if !matches!(body.kind, 1 | 2 | 3 | 4) {
        return Err(err_validation("渠道类型不合法"));
    }
    let pool = &state.repos.pool;
    let now = now_secs();
    let row: (i32,) = sqlx::query_as(
        "INSERT INTO ai_channel (shop_id, name, kind, base_url, is_enabled, is_default, sort, create_by, create_time, update_time, is_del)
         VALUES ($1, $2, $3, $4, $5, 0, $6, $7, $8, $8, 0) RETURNING channel_id",
    )
    .bind(tenant.shop_id)
    .bind(body.name.trim())
    .bind(body.kind)
    .bind(body.base_url.trim())
    .bind(i32::from(body.is_enabled.unwrap_or(true)))
    .bind(body.sort.unwrap_or(0))
    .bind(tenant.admin_id)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(|e| err_validation(e.to_string()))?;
    let channel_id = row.0;
    store_key(pool, channel_id, body.api_key.as_deref(), body.extra_key.as_deref())
        .await
        .map_err(err_validation)?;
    if let Some(models) = &body.models {
        for m in models {
            insert_model(pool, channel_id, m).await?;
        }
    }
    let ch = load_own_channel(&state, &tenant, channel_id).await?;
    ok(channel_vo(pool, &ch, false).await)
}

async fn insert_model(
    pool: &sqlx::PgPool,
    channel_id: i32,
    m: &ModelBody,
) -> Result<(), nextrade_security::web::AppError> {
    if m.model_code.trim().is_empty() {
        return Err(err_validation("模型代码不能为空"));
    }
    sqlx::query(
        "INSERT INTO ai_model (channel_id, model_code, display_name, capabilities, context_tokens, is_enabled, sort, is_del)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 0)",
    )
    .bind(channel_id)
    .bind(m.model_code.trim())
    .bind(m.display_name.clone().unwrap_or_default())
    .bind(m.capabilities.clone().unwrap_or_else(|| "chat".to_string()))
    .bind(m.context_tokens.unwrap_or(0))
    .bind(i32::from(m.is_enabled.unwrap_or(true)))
    .bind(m.sort.unwrap_or(0))
    .execute(pool)
    .await
    .map_err(|e| err_validation(e.to_string()))?;
    Ok(())
}

pub async fn update(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<i32>,
    Json(body): Json<ChannelBody>,
) -> ApiResult<Value> {
    let tenant = AiTenant::from_auth(&auth)?;
    let existing = load_own_channel(&state, &tenant, id).await?;
    if body.kind != 0 && body.kind != existing.kind {
        return Err(err_validation("渠道类型不可修改"));
    }
    let pool = &state.repos.pool;
    sqlx::query(
        "UPDATE ai_channel SET name = $1, base_url = $2, is_enabled = $3, sort = $4, update_time = $5
         WHERE channel_id = $6",
    )
    .bind(if body.name.trim().is_empty() { existing.name.clone() } else { body.name.trim().to_string() })
    .bind(if body.base_url.trim().is_empty() { existing.base_url.clone() } else { body.base_url.trim().to_string() })
    .bind(i32::from(body.is_enabled.unwrap_or(existing.is_enabled == 1)))
    .bind(body.sort.unwrap_or(existing.sort))
    .bind(now_secs())
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| err_validation(e.to_string()))?;
    // 密钥字段只写：传了非空才轮换，留空 = 不修改。
    if body.api_key.as_deref().is_some_and(|s| !s.trim().is_empty())
        || body.extra_key.as_deref().is_some_and(|s| !s.trim().is_empty())
    {
        store_key(pool, id, body.api_key.as_deref(), body.extra_key.as_deref())
            .await
            .map_err(err_validation)?;
    }
    let ch = load_own_channel(&state, &tenant, id).await?;
    ok(channel_vo(pool, &ch, false).await)
}

pub async fn remove(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<i32>,
) -> ApiResult<Value> {
    let tenant = AiTenant::from_auth(&auth)?;
    load_own_channel(&state, &tenant, id).await?;
    sqlx::query(
        "UPDATE ai_channel SET is_del = 1, is_default = 0, update_time = $1 WHERE channel_id = $2",
    )
    .bind(now_secs())
    .bind(id)
    .execute(&state.repos.pool)
    .await
    .map_err(|e| err_validation(e.to_string()))?;
    ok(json!({ "deleted": true }))
}

pub async fn set_default(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<i32>,
) -> ApiResult<Value> {
    let tenant = AiTenant::from_auth(&auth)?;
    load_own_channel(&state, &tenant, id).await?;
    let pool = &state.repos.pool;
    sqlx::query("UPDATE ai_channel SET is_default = 0 WHERE shop_id = $1 AND is_del = 0")
        .bind(tenant.shop_id)
        .execute(pool)
        .await
        .map_err(|e| err_validation(e.to_string()))?;
    sqlx::query("UPDATE ai_channel SET is_default = 1 WHERE channel_id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| err_validation(e.to_string()))?;
    ok(json!({ "default": true }))
}

// ————————————————————— 模型 CRUD —————————————————————

pub async fn list_models(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<i32>,
) -> ApiResult<Value> {
    let tenant = AiTenant::from_auth(&auth)?;
    load_own_channel(&state, &tenant, id).await?;
    let models: Vec<ModelRow> = sqlx::query_as(
        "SELECT model_id, channel_id, model_code, display_name, capabilities, context_tokens, is_enabled, sort
         FROM ai_model WHERE channel_id = $1 AND is_del = 0 ORDER BY sort, model_id",
    )
    .bind(id)
    .fetch_all(&state.repos.pool)
    .await
    .map_err(|e| err_validation(e.to_string()))?;
    ok(json!({ "models": models.iter().map(model_vo).collect::<Vec<_>>() }))
}

pub async fn create_model(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<i32>,
    Json(body): Json<ModelBody>,
) -> ApiResult<Value> {
    let tenant = AiTenant::from_auth(&auth)?;
    load_own_channel(&state, &tenant, id).await?;
    insert_model(&state.repos.pool, id, &body).await?;
    ok(json!({ "created": true }))
}

pub async fn update_model(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Path((id, model_id)): Path<(i32, i32)>,
    Json(body): Json<ModelBody>,
) -> ApiResult<Value> {
    let tenant = AiTenant::from_auth(&auth)?;
    load_own_channel(&state, &tenant, id).await?;
    if body.model_code.trim().is_empty() {
        return Err(err_validation("模型代码不能为空"));
    }
    let r = sqlx::query(
        "UPDATE ai_model SET model_code = $1, display_name = $2, capabilities = $3,
            context_tokens = $4, is_enabled = $5, sort = $6
         WHERE model_id = $7 AND channel_id = $8 AND is_del = 0",
    )
    .bind(body.model_code.trim())
    .bind(body.display_name.clone().unwrap_or_default())
    .bind(body.capabilities.clone().unwrap_or_else(|| "chat".to_string()))
    .bind(body.context_tokens.unwrap_or(0))
    .bind(i32::from(body.is_enabled.unwrap_or(true)))
    .bind(body.sort.unwrap_or(0))
    .bind(model_id)
    .bind(id)
    .execute(&state.repos.pool)
    .await
    .map_err(|e| err_validation(e.to_string()))?;
    if r.rows_affected() == 0 {
        return Err(err_validation("模型不存在"));
    }
    ok(json!({ "updated": true }))
}

pub async fn delete_model(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Path((id, model_id)): Path<(i32, i32)>,
) -> ApiResult<Value> {
    let tenant = AiTenant::from_auth(&auth)?;
    load_own_channel(&state, &tenant, id).await?;
    sqlx::query("UPDATE ai_model SET is_del = 1 WHERE model_id = $1 AND channel_id = $2")
        .bind(model_id)
        .bind(id)
        .execute(&state.repos.pool)
        .await
        .map_err(|e| err_validation(e.to_string()))?;
    ok(json!({ "deleted": true }))
}

// ————————————————————— 测试连接 —————————————————————

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TestBody {
    pub model_code: Option<String>,
    pub api_key: Option<String>,
}

pub async fn test_connection(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<i32>,
    Json(body): Json<TestBody>,
) -> ApiResult<Value> {
    let tenant = AiTenant::from_auth(&auth)?;
    let ch = load_own_channel(&state, &tenant, id).await?;
    if ch.kind != 2 {
        return ok(json!({
            "ok": false,
            "message": "该渠道类型不支持在线测试（仅 OpenAI 兼容渠道）"
        }));
    }
    let (stored_key, _) = load_channel_secret(&state.repos.pool, id)
        .await
        .unwrap_or_default();
    let api_key = body
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or(stored_key);
    if api_key.is_empty() {
        return ok(json!({ "ok": false, "message": "渠道未配置密钥" }));
    }
    let model_code = match body.model_code.as_deref().filter(|s| !s.is_empty()) {
        Some(m) => m.to_string(),
        None => {
            let row: Option<(String,)> = sqlx::query_as(
                "SELECT model_code FROM ai_model WHERE channel_id = $1 AND is_del = 0 AND is_enabled = 1
                 ORDER BY sort, model_id LIMIT 1",
            )
            .bind(id)
            .fetch_optional(&state.repos.pool)
            .await
            .ok()
            .flatten();
            match row {
                Some((m,)) => m,
                None => {
                    return ok(json!({
                        "ok": false,
                        "message": "渠道下没有可用模型，请先添加模型"
                    }));
                }
            }
        }
    };
    let base = ch.base_url.trim_end_matches('/');
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| err_validation(e.to_string()))?;
    let started = std::time::Instant::now();
    let resp = client
        .post(format!("{base}/chat/completions"))
        .bearer_auth(&api_key)
        .json(&json!({
            "model": model_code,
            "messages": [{ "role": "user", "content": "ping" }],
            "max_tokens": 1,
            "stream": false
        }))
        .send()
        .await;
    let latency_ms = started.elapsed().as_millis() as i64;
    match resp {
        Ok(r) if r.status().is_success() => ok(json!({
            "ok": true,
            "latencyMs": latency_ms,
            "modelCode": model_code,
            "message": "连接成功"
        })),
        Ok(r) => {
            let status = r.status().as_u16();
            let text = r.text().await.unwrap_or_default();
            let brief: String = text.chars().take(200).collect();
            ok(json!({
                "ok": false,
                "latencyMs": latency_ms,
                "modelCode": model_code,
                "message": format!("上游返回 {status}：{brief}")
            }))
        }
        Err(e) => ok(json!({
            "ok": false,
            "latencyMs": latency_ms,
            "modelCode": model_code,
            "message": format!("连接失败：{e}")
        })),
    }
}
