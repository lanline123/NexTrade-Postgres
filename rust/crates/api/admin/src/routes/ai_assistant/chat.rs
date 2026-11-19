//! 对话引擎（DESIGN §1.1 / §2.3 / §3 / §4）。
//!
//! - SSE 流式；工具调用循环（读直行、写进确认状态机、流在待确认处结束，
//!   confirm 接口返回新流续跑——上下文全部由 ai_message 表重建，无状态续接）。
//! - 工具注册表是隔离第 1 层：商家会话不注册 platform_only 工具。
//! - 全部工具执行走 ai_mcp::call_tool + 会话 ToolCtx（分发注入 + 实体校验在工具层）。

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use axum::{
    extract::{Extension, Path, State},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    Json,
};
use futures::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{json, Value};

use nextrade_security::web::{ok, ApiResult, AuthUser};

use super::channels::load_channel_secret;
use super::usage::record_usage;
use super::{err_validation, now_secs, AiTenant};
use crate::routes::ai_mcp::{self, McpSettings, ToolCtx};
use crate::AdminState;

// ————————————————————— 工具注册表（隔离第 1 层 + 确认分级） —————————————————————

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolRisk {
    Read,
    WriteNormal,
    WriteHigh,
}

#[derive(Debug, Clone, Copy)]
pub struct ToolMeta {
    pub name: &'static str,
    pub title: &'static str,
    pub risk: ToolRisk,
    /// 平台专属：商家会话不注册（模型看不见、调不到）。
    pub platform_only: bool,
}

const fn meta(name: &'static str, title: &'static str, risk: ToolRisk) -> ToolMeta {
    ToolMeta { name, title, risk, platform_only: false }
}

const fn meta_platform(name: &'static str, title: &'static str, risk: ToolRisk) -> ToolMeta {
    ToolMeta { name, title, risk, platform_only: true }
}

pub fn tool_registry() -> Vec<ToolMeta> {
    use ToolRisk::*;
    vec![
        meta("product_search", "商品搜索", Read),
        meta("product_get", "商品详情", Read),
        meta("product_list_categories", "分类列表", Read),
        meta("product_list_brands", "品牌列表", Read),
        meta("product_list_skus", "SKU 列表", Read),
        meta("product_update_fields", "修改商品信息", WriteNormal),
        meta("product_update_price", "修改商品价格", WriteNormal), // 超阈值动态升 High
        meta("product_update_stock", "修改商品库存", WriteNormal), // 超上限动态升 High
        meta("product_publish", "商品上架", WriteNormal),
        meta("product_unpublish", "商品下架", WriteNormal),
        meta("product_batch_preview", "批量操作预览", Read),
        meta("product_batch_execute", "批量操作", WriteHigh),
        meta("product_import", "批量导入商品", WriteHigh),
        meta("product_import_validate", "导入校验", Read),
        meta("product_copy", "复制商品", WriteNormal),
        meta("product_analysis", "商品分析", Read),
        meta("product_context", "商品样本统计", Read),
        meta("product_category_create", "创建商品分类", WriteNormal),
        meta("media_upload", "上传素材", WriteNormal),
        meta("media_manage", "素材管理", WriteNormal),
        meta("decorate_list_pages", "装修页列表", Read),
        meta("decorate_get_page", "装修页详情", Read),
        meta("decorate_save_draft", "保存装修草稿", WriteNormal),
        meta("decorate_publish", "发布装修页", WriteHigh),
        meta("analytics_sales_summary", "销售汇总", Read),
        meta("analytics_top_products", "热销商品", Read),
        meta("analytics_low_stock", "低库存预警", Read),
        meta("order_search", "订单搜索", Read),
        meta("order_get", "订单详情", Read),
        meta("order_update_remark", "修改订单备注", WriteNormal),
        meta("order_deliver", "订单发货", WriteNormal),
        meta_platform("member_search", "会员搜索", Read),
        meta_platform("member_get", "会员详情", Read),
        meta_platform("content_article_list", "文章列表", Read),
        meta_platform("content_article_get", "文章详情", Read),
    ]
}

pub fn tool_meta(name: &str) -> Option<ToolMeta> {
    tool_registry().into_iter().find(|m| m.name == name)
}

/// 聊天路径的工具设置：域全开（商家靠注册表裁剪），阈值沿用 MCP 设置，
/// 审批中心流不进聊天（require_approval_* 全关 + 免审核标记，确认由对话卡承担）。
async fn chat_settings(pool: &sqlx::PgPool, tenant: &AiTenant) -> McpSettings {
    let mut s = ai_mcp::load_settings(pool).await;
    s.enabled = true;
    s.product_enabled = true;
    s.decorate_enabled = true;
    s.analytics_enabled = true;
    s.order_enabled = true;
    s.member_enabled = !tenant.is_merchant();
    s.content_enabled = !tenant.is_merchant();
    s.require_approval_price = false;
    s.require_approval_publish = false;
    s.require_approval_batch = false;
    s.require_approval_import = false;
    s.approval_exempt = true;
    s
}

/// 下发给模型的 tools 定义（OpenAI 格式），按租户裁剪注册表。
fn chat_tools(tenant: &AiTenant, settings: &McpSettings) -> Vec<Value> {
    ai_mcp::tools_for(settings)
        .into_iter()
        .filter(|t| {
            let name = t.get("name").and_then(Value::as_str).unwrap_or("");
            match tool_meta(name) {
                Some(m) => !(tenant.is_merchant() && m.platform_only),
                None => false,
            }
        })
        .map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.get("name").cloned().unwrap_or(Value::Null),
                    "description": t.get("description").cloned().unwrap_or(Value::Null),
                    "parameters": t.get("inputSchema").cloned().unwrap_or_else(|| json!({"type": "object"})),
                }
            })
        })
        .collect()
}

// ————————————————————— 审计 —————————————————————

/// result：1 成功 / 2 拒绝-越权 / 3 拒绝-未注册 / 4 失败 / 5 待确认取消（拒绝）。
pub async fn write_audit(
    pool: &sqlx::PgPool,
    tenant: &AiTenant,
    conv_id: i32,
    tool: &str,
    args: &Value,
    result: i32,
) {
    let (target_type, target_id) = target_of(tool, args);
    let digest: String = serde_json::to_string(args)
        .unwrap_or_default()
        .chars()
        .take(500)
        .collect();
    let _ = sqlx::query(
        "INSERT INTO ai_action_audit (shop_id, admin_id, conv_id, tool, target_type, target_id, args_digest, result, create_time)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(tenant.shop_id)
    .bind(tenant.admin_id)
    .bind(conv_id)
    .bind(tool)
    .bind(target_type)
    .bind(target_id)
    .bind(digest)
    .bind(result)
    .bind(now_secs())
    .execute(pool)
    .await;
}

fn target_of(tool: &str, args: &Value) -> (&'static str, i32) {
    let get = |k: &str| args.get(k).and_then(Value::as_i64).unwrap_or(0) as i32;
    if tool.starts_with("product_") {
        ("product", get("productId"))
    } else if tool.starts_with("order_") {
        ("order", get("orderId"))
    } else if tool.starts_with("decorate_") {
        ("decorate", get("decorateId"))
    } else if tool.starts_with("media_") {
        ("media", get("picId"))
    } else if tool.starts_with("member_") {
        ("member", get("userId"))
    } else if tool.starts_with("content_") {
        ("article", get("articleId"))
    } else {
        ("", 0)
    }
}

// ————————————————————— 会话 CRUD —————————————————————

#[derive(sqlx::FromRow, Clone)]
pub struct ConvRow {
    pub conv_id: i32,
    pub shop_id: i32,
    pub admin_id: i32,
    pub title: String,
    pub channel_id: i32,
    pub model_code: String,
    pub status: i32,
    pub create_time: i64,
    pub update_time: i64,
}

fn conv_vo(c: &ConvRow) -> Value {
    json!({
        "convId": c.conv_id,
        "shopId": c.shop_id,
        "adminId": c.admin_id,
        "title": c.title,
        "channelId": c.channel_id,
        "modelCode": c.model_code,
        "createTime": c.create_time,
        "updateTime": c.update_time,
    })
}

async fn load_conv(state: &AdminState, conv_id: i32) -> Result<ConvRow, nextrade_security::web::AppError> {
    sqlx::query_as(
        "SELECT conv_id, shop_id, admin_id, title, channel_id, model_code, status, create_time, update_time
         FROM ai_conversation WHERE conv_id = $1 AND status = 1",
    )
    .bind(conv_id)
    .fetch_optional(&state.repos.pool)
    .await
    .map_err(|e| err_validation(e.to_string()))?
    .ok_or_else(|| err_validation("会话不存在"))
}

/// 可见性：本人会话；店主可看本店全部会话（DESIGN §1.6 注 + 拍板 8）。
async fn conv_visible(state: &AdminState, tenant: &AiTenant, conv: &ConvRow) -> bool {
    if conv.shop_id != tenant.shop_id {
        return false;
    }
    if conv.admin_id == tenant.admin_id {
        return true;
    }
    tenant.is_shop_owner(&state.repos.pool).await
}

pub async fn list_conversations(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<Value> {
    let tenant = AiTenant::from_auth(&auth)?;
    let owner = tenant.is_shop_owner(&state.repos.pool).await;
    let rows: Vec<ConvRow> = if owner {
        sqlx::query_as(
            "SELECT conv_id, shop_id, admin_id, title, channel_id, model_code, status, create_time, update_time
             FROM ai_conversation WHERE shop_id = $1 AND status = 1 ORDER BY update_time DESC LIMIT 100",
        )
        .bind(tenant.shop_id)
        .fetch_all(&state.repos.pool)
        .await
    } else {
        sqlx::query_as(
            "SELECT conv_id, shop_id, admin_id, title, channel_id, model_code, status, create_time, update_time
             FROM ai_conversation WHERE shop_id = $1 AND admin_id = $2 AND status = 1 ORDER BY update_time DESC LIMIT 100",
        )
        .bind(tenant.shop_id)
        .bind(tenant.admin_id)
        .fetch_all(&state.repos.pool)
        .await
    }
    .map_err(|e| err_validation(e.to_string()))?;
    ok(json!({ "conversations": rows.iter().map(conv_vo).collect::<Vec<_>>() }))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CreateConvBody {
    pub channel_id: Option<i32>,
    pub model_code: Option<String>,
    pub title: Option<String>,
}

/// 解析默认渠道 + 模型：指定 → 本租户默认渠道 → 第一个启用渠道 → 官方渠道。
async fn resolve_channel_model(
    state: &AdminState,
    tenant: &AiTenant,
    channel_id: Option<i32>,
    model_code: Option<String>,
) -> Result<(i32, String), String> {
    let pool = &state.repos.pool;
    let channel: Option<(i32, i32)> = if let Some(id) = channel_id {
        sqlx::query_as(
            "SELECT channel_id, kind FROM ai_channel
             WHERE channel_id = $1 AND is_del = 0 AND is_enabled = 1
               AND (shop_id = $2 OR (kind = 1 AND shop_id = 0))",
        )
        .bind(id)
        .bind(tenant.shop_id)
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
        .bind(tenant.shop_id)
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

async fn insert_conversation(
    state: &AdminState,
    tenant: &AiTenant,
    channel_id: i32,
    model_code: &str,
    title: &str,
) -> Result<ConvRow, String> {
    let now = now_secs();
    sqlx::query_as(
        "INSERT INTO ai_conversation (shop_id, admin_id, title, channel_id, model_code, status, create_time, update_time)
         VALUES ($1, $2, $3, $4, $5, 1, $6, $6) RETURNING conv_id, shop_id, admin_id, title, channel_id, model_code, status, create_time, update_time",
    )
    .bind(tenant.shop_id)
    .bind(tenant.admin_id)
    .bind(title)
    .bind(channel_id)
    .bind(model_code)
    .bind(now)
    .fetch_one(&state.repos.pool)
    .await
    .map_err(|e| e.to_string())
}

pub async fn create_conversation(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<CreateConvBody>,
) -> ApiResult<Value> {
    let tenant = AiTenant::from_auth(&auth)?;
    let (cid, model) = resolve_channel_model(&state, &tenant, body.channel_id, body.model_code)
        .await
        .map_err(err_validation)?;
    let conv = insert_conversation(
        &state,
        &tenant,
        cid,
        &model,
        body.title.as_deref().unwrap_or(""),
    )
    .await
    .map_err(err_validation)?;
    ok(conv_vo(&conv))
}

pub async fn list_messages(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<i32>,
) -> ApiResult<Value> {
    let tenant = AiTenant::from_auth(&auth)?;
    let conv = load_conv(&state, id).await?;
    if !conv_visible(&state, &tenant, &conv).await {
        return Err(err_validation("会话不存在"));
    }
    let rows = load_message_rows(&state, id).await?;
    ok(json!({
        "conversation": conv_vo(&conv),
        "messages": rows.iter().map(msg_vo).collect::<Vec<_>>(),
    }))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RenameBody {
    pub title: String,
}

pub async fn rename_conversation(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<i32>,
    Json(body): Json<RenameBody>,
) -> ApiResult<Value> {
    let tenant = AiTenant::from_auth(&auth)?;
    let conv = load_conv(&state, id).await?;
    if !conv_visible(&state, &tenant, &conv).await {
        return Err(err_validation("会话不存在"));
    }
    sqlx::query("UPDATE ai_conversation SET title = $1, update_time = $2 WHERE conv_id = $3")
        .bind(body.title.chars().take(200).collect::<String>())
        .bind(now_secs())
        .bind(id)
        .execute(&state.repos.pool)
        .await
        .map_err(|e| err_validation(e.to_string()))?;
    ok(json!({ "renamed": true }))
}

pub async fn delete_conversation(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<i32>,
) -> ApiResult<Value> {
    let tenant = AiTenant::from_auth(&auth)?;
    let conv = load_conv(&state, id).await?;
    if !conv_visible(&state, &tenant, &conv).await {
        return Err(err_validation("会话不存在"));
    }
    sqlx::query("UPDATE ai_conversation SET status = 0, update_time = $1 WHERE conv_id = $2")
        .bind(now_secs())
        .bind(id)
        .execute(&state.repos.pool)
        .await
        .map_err(|e| err_validation(e.to_string()))?;
    ok(json!({ "deleted": true }))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SwitchModelBody {
    pub channel_id: i32,
    pub model_code: String,
}

pub async fn switch_model(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<i32>,
    Json(body): Json<SwitchModelBody>,
) -> ApiResult<Value> {
    let tenant = AiTenant::from_auth(&auth)?;
    let conv = load_conv(&state, id).await?;
    if conv.admin_id != tenant.admin_id || conv.shop_id != tenant.shop_id {
        return Err(err_validation("会话不存在"));
    }
    let (cid, model) = resolve_channel_model(
        &state,
        &tenant,
        Some(body.channel_id),
        Some(body.model_code.clone()),
    )
    .await
    .map_err(err_validation)?;
    sqlx::query(
        "UPDATE ai_conversation SET channel_id = $1, model_code = $2, update_time = $3 WHERE conv_id = $4",
    )
    .bind(cid)
    .bind(&model)
    .bind(now_secs())
    .bind(id)
    .execute(&state.repos.pool)
    .await
    .map_err(|e| err_validation(e.to_string()))?;
    ok(json!({ "channelId": cid, "modelCode": model }))
}

// ————————————————————— 消息持久化 —————————————————————

#[derive(sqlx::FromRow, Clone)]
pub struct MsgRow {
    pub msg_id: i64,
    pub conv_id: i32,
    pub role: i32,
    pub content: String,
    pub tool_calls: Option<Value>,
    pub tool_name: String,
    pub tool_status: i32,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub create_time: i64,
}

fn msg_vo(m: &MsgRow) -> Value {
    json!({
        "msgId": m.msg_id,
        "convId": m.conv_id,
        "role": m.role,
        "content": m.content,
        "toolCalls": m.tool_calls,
        "toolName": m.tool_name,
        "toolStatus": m.tool_status,
        "promptTokens": m.prompt_tokens,
        "completionTokens": m.completion_tokens,
        "createTime": m.create_time,
    })
}

async fn load_message_rows(state: &AdminState, conv_id: i32) -> Result<Vec<MsgRow>, nextrade_security::web::AppError> {
    sqlx::query_as(
        "SELECT msg_id, conv_id, role, content, tool_calls, tool_name, tool_status,
                prompt_tokens, completion_tokens, create_time
         FROM ai_message WHERE conv_id = $1 ORDER BY msg_id",
    )
    .bind(conv_id)
    .fetch_all(&state.repos.pool)
    .await
    .map_err(|e| err_validation(e.to_string()))
}

async fn insert_message(
    pool: &sqlx::PgPool,
    conv_id: i32,
    role: i32,
    content: &str,
    tool_calls: Option<Value>,
    tool_name: &str,
    tool_status: i32,
) -> Result<i64, String> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO ai_message (conv_id, role, content, tool_calls, tool_name, tool_status, create_time)
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING msg_id",
    )
    .bind(conv_id)
    .bind(role)
    .bind(content)
    .bind(tool_calls)
    .bind(tool_name)
    .bind(tool_status)
    .bind(now_secs())
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(row.0)
}

async fn touch_conv(pool: &sqlx::PgPool, conv_id: i32) {
    let _ = sqlx::query("UPDATE ai_conversation SET update_time = $1 WHERE conv_id = $2")
        .bind(now_secs())
        .bind(conv_id)
        .execute(pool)
        .await;
}

// ————————————————————— 历史重建（OpenAI 消息格式） —————————————————————

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: Value,
}

fn system_prompt(tenant: &AiTenant, shop_title: &str) -> String {
    let today = chrono::Local::now().format("%Y-%m-%d");
    if tenant.is_merchant() {
        format!(
            "你是 NexTrade 电商平台后台的 AI 运营助手，通过工具帮助商家管理本店业务。\n\
             当前身份：{user}（店铺「{shop}」）。今天是 {today}。\n\
             规则：查询类操作直接调用工具完成；涉及修改数据的操作会先由用户在界面上确认，你只管发起调用，不要在回复里重复询问确认。\n\
             你只能操作本店的数据（工具已按本店范围限定）；用户要求查看或修改其他店铺、平台全局数据时，直接说明没有这个权限，不要尝试调用工具绕过。\n\
             回答用简体中文，简洁准确；金额、库存、订单号等数字一律以工具返回为准，不要编造。工具返回为空或失败时如实说明。",
            user = tenant.username,
            shop = shop_title,
            today = today,
        )
    } else {
        format!(
            "你是 NexTrade 电商平台后台的 AI 运营助手，通过工具帮助平台管理员管理全平台业务。\n\
             当前身份：{user}（平台管理员）。今天是 {today}。\n\
             规则：查询类操作直接调用工具完成；涉及修改数据的操作会先由用户在界面上确认，你只管发起调用，不要在回复里重复询问确认。\n\
             回答用简体中文，简洁准确；金额、库存、订单号等数字一律以工具返回为准，不要编造。工具返回为空或失败时如实说明。",
            user = tenant.username,
            today = today,
        )
    }
}

/// 从消息行重建上下文；同时返回「最后一个 assistant 消息里尚未处理的 tool_calls」。
fn rebuild_history(
    tenant: &AiTenant,
    shop_title: &str,
    rows: &[MsgRow],
) -> (Vec<Value>, VecDeque<ToolCall>) {
    // tool 行按 callId 索引
    let mut tool_rows: HashMap<String, &MsgRow> = HashMap::new();
    for r in rows.iter().filter(|r| r.role == 3) {
        if let Some(call_id) = r
            .tool_calls
            .as_ref()
            .and_then(|tc| tc.get("callId"))
            .and_then(Value::as_str)
        {
            tool_rows.insert(call_id.to_string(), r);
        }
    }
    let mut messages: Vec<Value> = vec![json!({
        "role": "system",
        "content": system_prompt(tenant, shop_title),
    })];
    let mut pending: VecDeque<ToolCall> = VecDeque::new();
    for r in rows {
        match r.role {
            1 => messages.push(json!({ "role": "user", "content": r.content })),
            2 => {
                let mut msg = json!({ "role": "assistant", "content": r.content });
                if let Some(calls) = r.tool_calls.as_ref().and_then(Value::as_array) {
                    let oai_calls: Vec<Value> = calls
                        .iter()
                        .map(|c| {
                            json!({
                                "id": c.get("id").cloned().unwrap_or(Value::Null),
                                "type": "function",
                                "function": {
                                    "name": c.get("name").cloned().unwrap_or(Value::Null),
                                    "arguments": c.get("args").map(|a| a.to_string()).unwrap_or_else(|| "{}".to_string()),
                                }
                            })
                        })
                        .collect();
                    msg["tool_calls"] = Value::Array(oai_calls);
                    // 紧随其后的 tool 消息在下面的 role=3 分支按 callId 顺序补齐；
                    // 这里先记录哪些 call 还没有结果行（待处理）。
                    for c in calls {
                        let cid = c.get("id").and_then(Value::as_str).unwrap_or("");
                        if !tool_rows.contains_key(cid) {
                            pending.push_back(ToolCall {
                                id: cid.to_string(),
                                name: c.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
                                args: c.get("args").cloned().unwrap_or_else(|| json!({})),
                            });
                        }
                    }
                }
                messages.push(msg);
            }
            3 => {
                let call_id = r
                    .tool_calls
                    .as_ref()
                    .and_then(|tc| tc.get("callId"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let content = match r.tool_status {
                    2 => truncate_chars(&r.content, 6000),
                    3 => "用户拒绝了此操作，未执行。".to_string(),
                    4 => format!("执行失败：{}", truncate_chars(&r.content, 500)),
                    _ => "该操作未执行（已取消）。".to_string(),
                };
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": call_id,
                    "content": content,
                }));
            }
            _ => {}
        }
    }
    // pending 只保留「最后一个 assistant 消息」的未处理调用（更早的视为已放弃，清掉防重复执行）。
    let last_assistant_calls: HashSet<String> = rows
        .iter()
        .rev()
        .find(|r| r.role == 2)
        .and_then(|r| r.tool_calls.as_ref())
        .and_then(Value::as_array)
        .map(|calls| {
            calls
                .iter()
                .filter_map(|c| c.get("id").and_then(Value::as_str).map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    pending.retain(|c| last_assistant_calls.contains(&c.id));
    // 上下文预算：粗略按字符数截断最旧的几轮（保留 system + 最近消息）。
    let budget = 120_000usize;
    let total: usize = messages.iter().map(|m| m.to_string().len()).sum();
    if total > budget {
        let mut kept: Vec<Value> = vec![messages[0].clone()];
        let mut acc = kept[0].to_string().len();
        let mut tail: Vec<Value> = Vec::new();
        for m in messages.iter().skip(1).rev() {
            let len = m.to_string().len();
            if acc + len > budget && !tail.is_empty() {
                break;
            }
            acc += len;
            tail.push(m.clone());
        }
        tail.reverse();
        // 避免以 tool 消息开头（上游会报错）：丢弃开头的 tool 消息。
        while tail.first().and_then(|m| m.get("role")).and_then(Value::as_str) == Some("tool") {
            tail.remove(0);
        }
        kept.extend(tail);
        messages = kept;
    }
    (messages, pending)
}

fn truncate_chars(s: &str, max: usize) -> String {
    let mut out: String = s.chars().take(max).collect();
    if s.chars().count() > max {
        out.push_str("…（已截断）");
    }
    out
}

// ————————————————————— 风险分级与确认摘要 —————————————————————

pub struct Classified {
    pub risk: ToolRisk,
    pub summary: String,
}

async fn classify(
    state: &AdminState,
    settings: &McpSettings,
    name: &str,
    args: &Value,
) -> Classified {
    let m = tool_meta(name);
    let title = m.map(|m| m.title).unwrap_or(name);
    let base_risk = m.map(|m| m.risk).unwrap_or(ToolRisk::WriteHigh);
    let mut summary = summarize(name, title, args);
    if base_risk != ToolRisk::WriteNormal {
        return Classified { risk: base_risk, summary };
    }
    // 动态升格：改价/改库存超阈值（口径与工具内审批门一致）。
    if name == "product_update_price" {
        if let (Some(id), Some(price)) = (
            args.get("productId").and_then(Value::as_i64).map(|v| v as i32),
            args.get("productPrice").and_then(|v| {
                v.as_f64()
                    .map(|f| Decimal::from_f64_retain(f).unwrap_or_default())
                    .or_else(|| v.as_str().and_then(|s| s.parse::<Decimal>().ok()))
            }),
        ) {
            if let Ok(Some((old, _))) = state
                .repos
                .catalog
                .product_manage
                .ai_product_price(id)
                .await
            {
                let delta = (price - old).abs();
                let pct_limit = Decimal::from_f64_retain(settings.price_change_pct).unwrap_or_default();
                let amt_limit = Decimal::from_f64_retain(settings.price_change_amount).unwrap_or_default();
                let over_pct = pct_limit > Decimal::ZERO
                    && old > Decimal::ZERO
                    && delta > old * pct_limit / Decimal::from(100i64);
                let over_amt = amt_limit > Decimal::ZERO && delta > amt_limit;
                summary = format!("将商品 #{id} 的售价从 ¥{old} 改为 ¥{price}");
                if over_pct || over_amt {
                    return Classified { risk: ToolRisk::WriteHigh, summary };
                }
            }
        }
    }
    if name == "product_update_stock" {
        if let (Some(id), Some(stock)) = (
            args.get("productId").and_then(Value::as_i64).map(|v| v as i32),
            args.get("productStock").and_then(Value::as_i64),
        ) {
            if let Ok(Some(old)) = state
                .repos
                .catalog
                .product_manage
                .ai_product_stock(id)
                .await
            {
                summary = format!("将商品 #{id} 的库存从 {old} 改为 {stock}");
                if settings.stock_change_cap > 0
                    && (stock - i64::from(old)).abs() > settings.stock_change_cap
                {
                    return Classified { risk: ToolRisk::WriteHigh, summary };
                }
            }
        }
    }
    Classified { risk: ToolRisk::WriteNormal, summary }
}

fn summarize(name: &str, title: &str, args: &Value) -> String {
    let get_i = |k: &str| args.get(k).and_then(Value::as_i64);
    let get_s = |k: &str| args.get(k).and_then(Value::as_str).unwrap_or("");
    match name {
        "product_update_fields" => format!("修改商品 #{} 的基本信息（{}）", get_i("productId").unwrap_or(0), {
            let mut fields = Vec::new();
            if args.get("productName").is_some() { fields.push(format!("名称→{}", get_s("productName"))); }
            if args.get("productBrief").is_some() { fields.push("简介".to_string()); }
            if args.get("categoryId").is_some() { fields.push(format!("分类→#{}", get_i("categoryId").unwrap_or(0))); }
            if args.get("brandId").is_some() { fields.push(format!("品牌→#{}", get_i("brandId").unwrap_or(0))); }
            if args.get("productUnit").is_some() { fields.push(format!("单位→{}", get_s("productUnit"))); }
            fields.join("、")
        }),
        "product_update_price" => format!("将商品 #{} 的售价改为 ¥{}", get_i("productId").unwrap_or(0), get_s("productPrice")),
        "product_update_stock" => format!("将商品 #{} 的库存改为 {}", get_i("productId").unwrap_or(0), get_i("productStock").unwrap_or(0)),
        "product_publish" => format!("上架商品 #{}", get_i("productId").unwrap_or(0)),
        "product_unpublish" => format!("下架商品 #{}", get_i("productId").unwrap_or(0)),
        "product_batch_execute" => {
            let n = args.get("productIds").and_then(Value::as_array).map_or(0, Vec::len);
            format!("对 {n} 个商品执行批量操作「{}」（高风险，请仔细核对）", get_s("operation"))
        }
        "product_import" => {
            let n = args.get("products").and_then(Value::as_array).map_or(0, Vec::len);
            format!("批量导入 {n} 个新商品（高风险，请仔细核对）")
        }
        "product_copy" => format!("复制商品 #{} 为新商品（下架状态、库存 0）", get_i("productId").unwrap_or(0)),
        "decorate_save_draft" => format!("保存装修页 #{} 的草稿", get_i("decorateId").unwrap_or(0)),
        "decorate_publish" => format!(
            "发布装修页 #{}（高风险）。注意：若你正在装修编辑器里打开这个页面，编辑器里保存会整体覆盖本次发布的内容。",
            get_i("decorateId").unwrap_or(0)
        ),
        "order_update_remark" => format!("修改订单 #{} 的后台备注为「{}」", get_i("orderId").unwrap_or(0), get_s("remark")),
        "order_deliver" => format!(
            "订单 #{} 发货（方式 {}，物流单号 {}）",
            get_i("orderId").unwrap_or(0),
            get_i("shippingMethod").unwrap_or(0),
            if get_s("trackingNo").is_empty() { "无" } else { get_s("trackingNo") }
        ),
        "media_upload" => "上传一张图片到素材库".to_string(),
        "media_manage" => format!("素材管理操作「{}」", get_s("operation")),
        "product_category_create" => {
            let n = args.get("paths").and_then(Value::as_array).map_or(0, Vec::len);
            format!("创建 {n} 条商品分类路径")
        }
        _ => {
            let brief: String = serde_json::to_string(args).unwrap_or_default().chars().take(200).collect();
            format!("{title}：{brief}")
        }
    }
}

// ————————————————————— 工具执行（分发 + 审计 + 落库） —————————————————————

pub struct ExecOutcome {
    /// 给模型看的结果文本（已截断）。
    pub model_text: String,
    /// 工具行状态：2 已执行 / 4 失败。
    pub status: i32,
}

/// 执行一个工具调用并落 tool 消息行。
/// `existing_msg` 为确认续跑时已存在的待确认行（更新它而不是新建）。
async fn execute_tool(
    state: &AdminState,
    tenant: &AiTenant,
    settings: &McpSettings,
    conv_id: i32,
    call: &ToolCall,
    existing_msg: Option<i64>,
) -> ExecOutcome {
    let pool = &state.repos.pool;
    let ctx = tenant.tool_ctx(conv_id);

    let save_row = |status: i32, content: String| {
        let pool = pool.clone();
        let call = call.clone();
        async move {
            if let Some(mid) = existing_msg {
                let _ = sqlx::query(
                    "UPDATE ai_message SET tool_status = $1, content = $2 WHERE msg_id = $3",
                )
                .bind(status)
                .bind(&content)
                .bind(mid)
                .execute(&pool)
                .await;
            } else {
                let _ = insert_message(
                    &pool,
                    conv_id,
                    3,
                    &content,
                    Some(json!({ "callId": call.id, "name": call.name, "args": call.args })),
                    &call.name,
                    status,
                )
                .await;
            }
        }
    };

    // 注册层兜底：未注册 / 商家调平台工具 → 拒绝 + 审计（结果 3）。
    let m = tool_meta(&call.name);
    if m.is_none() || (tenant.is_merchant() && m.is_some_and(|x| x.platform_only)) {
        write_audit(pool, tenant, conv_id, &call.name, &call.args, 3).await;
        let text = "该操作不可用。".to_string();
        save_row(4, text.clone()).await;
        return ExecOutcome { model_text: text, status: 4 };
    }
    // 参数级 shop 校验（DESIGN §3.1）：不一致 → 整次拒绝 + 审计（结果 2）。
    let sanitized = match ai_mcp::sanitize_chat_args(&ctx, &call.args) {
        Ok(v) => v,
        Err(_) => {
            write_audit(pool, tenant, conv_id, &call.name, &call.args, 2).await;
            let text = "未找到该对象或无权访问。".to_string();
            save_row(4, text.clone()).await;
            return ExecOutcome { model_text: text, status: 4 };
        }
    };
    // 批量/导入的二次确认标记由分发层注入（用户已在确认卡上批准）。
    let mut args = sanitized;
    if matches!(call.name.as_str(), "product_batch_execute" | "product_import") {
        if let Some(obj) = args.as_object_mut() {
            obj.insert("confirmation".to_string(), Value::Bool(true));
        }
    }
    match ai_mcp::call_tool(state, settings, &call.name, &args, true, &ctx).await {
        Ok(value) => {
            let full = value.to_string();
            save_row(2, truncate_chars(&full, 100_000)).await;
            write_audit(pool, tenant, conv_id, &call.name, &call.args, 1).await;
            ExecOutcome {
                model_text: truncate_chars(&full, 6000),
                status: 2,
            }
        }
        Err(e) => {
            // 工具层的作用域拒绝与「不存在」同文案，审计记为越权拒绝（结果 2）；
            // 其余失败记结果 4。判定靠文案口径：以「不存在」结尾的实体错误。
            let is_scope_deny = e.ends_with("不存在");
            write_audit(
                pool,
                tenant,
                conv_id,
                &call.name,
                &call.args,
                if is_scope_deny { 2 } else { 4 },
            )
            .await;
            save_row(4, e.clone()).await;
            ExecOutcome {
                model_text: format!("执行失败：{}", truncate_chars(&e, 500)),
                status: 4,
            }
        }
    }
}

// ————————————————————— 在途流注册（停止标记 + 并发上限） —————————————————————

fn stop_flags() -> &'static Mutex<HashMap<i32, Arc<AtomicBool>>> {
    static FLAGS: OnceLock<Mutex<HashMap<i32, Arc<AtomicBool>>>> = OnceLock::new();
    FLAGS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn tenant_active() -> &'static Mutex<HashMap<i32, usize>> {
    static ACTIVE: OnceLock<Mutex<HashMap<i32, usize>>> = OnceLock::new();
    ACTIVE.get_or_init(|| Mutex::new(HashMap::new()))
}

struct InflightGuard {
    conv_id: i32,
    shop_id: i32,
}

impl Drop for InflightGuard {
    fn drop(&mut self) {
        if let Ok(mut map) = stop_flags().lock() {
            map.remove(&self.conv_id);
        }
        if let Ok(mut map) = tenant_active().lock() {
            if let Some(n) = map.get_mut(&self.shop_id) {
                *n = n.saturating_sub(1);
                if *n == 0 {
                    map.remove(&self.shop_id);
                }
            }
        }
    }
}

fn acquire_inflight(shop_id: i32, conv_id: i32) -> Result<(Arc<AtomicBool>, InflightGuard), String> {
    {
        let mut map = tenant_active().lock().map_err(|_| "内部状态异常".to_string())?;
        let n = map.get(&shop_id).copied().unwrap_or(0);
        if n >= 2 {
            return Err("当前店铺已有对话正在生成，请等其完成后再发送".to_string());
        }
        *map.entry(shop_id).or_insert(0) += 1;
    }
    let flag = Arc::new(AtomicBool::new(false));
    {
        let mut map = stop_flags().lock().map_err(|_| "内部状态异常".to_string())?;
        if map.contains_key(&conv_id) {
            // 同一会话已有在途流：回滚计数并拒绝。
            if let Ok(mut active) = tenant_active().lock() {
                if let Some(n) = active.get_mut(&shop_id) {
                    *n = n.saturating_sub(1);
                }
            }
            return Err("该会话正在生成中".to_string());
        }
        map.insert(conv_id, flag.clone());
    }
    Ok((flag, InflightGuard { conv_id, shop_id }))
}

// ————————————————————— 上游（OpenAI 兼容）流式调用 —————————————————————

enum RoundKind {
    Text,
    Calls(Vec<ToolCall>),
    Failed,
    Stopped,
}

struct EngineJob {
    state: AdminState,
    tenant: AiTenant,
    conv: ConvRow,
    skip_confirm: HashSet<String>,
}

async fn send_event(tx: &mut futures::channel::mpsc::Sender<Event>, event: &str, data: Value) {
    let _ = tx
        .send(Event::default().event(event).data(data.to_string()))
        .await;
}

/// 执行一轮上游调用：流式消费、落 assistant 消息行、记用量。
/// 返回本轮结果类型与（若有）工具调用列表。
#[allow(clippy::too_many_arguments)]
async fn upstream_round(
    job: &EngineJob,
    messages: &[Value],
    tools: &[Value],
    tx: &mut futures::channel::mpsc::Sender<Event>,
    stop_flag: &AtomicBool,
) -> (RoundKind, Vec<ToolCall>) {
    let state = &job.state;
    let tenant = &job.tenant;
    let conv = &job.conv;
    let pool = &state.repos.pool;

    // 解析渠道（自有或官方）与密钥。
    let channel: Option<(String, i32, i32)> = sqlx::query_as(
        "SELECT base_url, kind, is_enabled FROM ai_channel
         WHERE channel_id = $1 AND is_del = 0 AND (shop_id = $2 OR (kind = 1 AND shop_id = 0))",
    )
    .bind(conv.channel_id)
    .bind(tenant.shop_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    let Some((base_url, kind, is_enabled)) = channel else {
        send_event(tx, "error", json!({ "message": "会话绑定的渠道不存在或已被删除，请重新选择渠道" })).await;
        return (RoundKind::Failed, Vec::new());
    };
    if is_enabled != 1 {
        send_event(tx, "error", json!({ "message": "会话绑定的渠道已被停用，请重新选择渠道" })).await;
        return (RoundKind::Failed, Vec::new());
    }
    if kind != 2 {
        send_event(tx, "error", json!({ "message": "该渠道类型暂不支持对话（当前仅支持 OpenAI 兼容渠道）" })).await;
        return (RoundKind::Failed, Vec::new());
    }
    let (api_key, _) = match load_channel_secret(pool, conv.channel_id).await {
        Ok(pair) if !pair.0.is_empty() => pair,
        Ok(_) => {
            send_event(tx, "error", json!({ "message": "渠道未配置密钥，请先在「渠道配置」中填写 API 密钥" })).await;
            return (RoundKind::Failed, Vec::new());
        }
        Err(e) => {
            send_event(tx, "error", json!({ "message": e })).await;
            return (RoundKind::Failed, Vec::new());
        }
    };

    // 先落一条空的 assistant 行，流式过程中增量更新（断线可续读）。
    let assistant_msg_id = match insert_message(pool, conv.conv_id, 2, "", None, "", 0).await {
        Ok(id) => id,
        Err(e) => {
            send_event(tx, "error", json!({ "message": format!("保存消息失败：{e}") })).await;
            return (RoundKind::Failed, Vec::new());
        }
    };
    send_event(tx, "assistant", json!({ "msgId": assistant_msg_id, "convId": conv.conv_id })).await;

    let payload = json!({
        "model": conv.model_code,
        "messages": messages,
        "tools": tools,
        "stream": true,
        "stream_options": { "include_usage": true },
        "temperature": 0.3,
    });
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .connect_timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            send_event(tx, "error", json!({ "message": format!("HTTP 客户端初始化失败：{e}") })).await;
            return (RoundKind::Failed, Vec::new());
        }
    };
    // 连接阶段重试（429/5xx/网络错误，最多 3 次）；一旦开始出流不再重试。
    let mut attempt = 0;
    let mut resp = loop {
        attempt += 1;
        match client.post(&url).bearer_auth(&api_key).json(&payload).send().await {
            Ok(r) if r.status().is_success() => break r,
            Ok(r) => {
                let status = r.status();
                if (status.as_u16() == 429 || status.is_server_error()) && attempt < 3 {
                    tokio::time::sleep(std::time::Duration::from_millis(600 * attempt)).await;
                    continue;
                }
                let body = r.text().await.unwrap_or_default();
                let brief: String = body.chars().take(300).collect();
                let msg = format!("上游返回 {status}：{brief}");
                record_usage(pool, tenant, conv.conv_id, conv.channel_id, &conv.model_code, 0, 0, 2, &msg).await;
                let _ = sqlx::query("DELETE FROM ai_message WHERE msg_id = $1 AND content = ''")
                    .bind(assistant_msg_id)
                    .execute(pool)
                    .await;
                send_event(tx, "error", json!({ "message": msg })).await;
                return (RoundKind::Failed, Vec::new());
            }
            Err(e) => {
                if attempt < 3 {
                    tokio::time::sleep(std::time::Duration::from_millis(600 * attempt)).await;
                    continue;
                }
                let msg = format!("连接上游失败：{e}");
                record_usage(pool, tenant, conv.conv_id, conv.channel_id, &conv.model_code, 0, 0, 2, &msg).await;
                let _ = sqlx::query("DELETE FROM ai_message WHERE msg_id = $1 AND content = ''")
                    .bind(assistant_msg_id)
                    .execute(pool)
                    .await;
                send_event(tx, "error", json!({ "message": msg })).await;
                return (RoundKind::Failed, Vec::new());
            }
        }
    };

    let mut buf: Vec<u8> = Vec::new();
    let mut text = String::new();
    let mut calls_map: std::collections::BTreeMap<i64, (String, String, String)> =
        std::collections::BTreeMap::new();
    let mut prompt_tokens: i32 = 0;
    let mut completion_tokens: i32 = 0;
    let mut delta_count: u32 = 0;
    let mut stream_broken = false;

    'read: loop {
        if stop_flag.load(Ordering::Relaxed) {
            break 'read;
        }
        let chunk = match resp.chunk().await {
            Ok(Some(c)) => c,
            Ok(None) => break 'read,
            Err(_) => {
                stream_broken = true;
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
                prompt_tokens = u.get("prompt_tokens").and_then(Value::as_i64).unwrap_or(0) as i32;
                completion_tokens = u
                    .get("completion_tokens")
                    .and_then(Value::as_i64)
                    .unwrap_or(0) as i32;
            }
            let delta = v.pointer("/choices/0/delta").cloned().unwrap_or(Value::Null);
            if let Some(content) = delta.get("content").and_then(Value::as_str) {
                if !content.is_empty() {
                    text.push_str(content);
                    delta_count += 1;
                    send_event(tx, "delta", json!({ "content": content })).await;
                    if delta_count % 40 == 0 {
                        let _ = sqlx::query("UPDATE ai_message SET content = $1 WHERE msg_id = $2")
                            .bind(&text)
                            .bind(assistant_msg_id)
                            .execute(pool)
                            .await;
                    }
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

    let stopped = stop_flag.load(Ordering::Relaxed);
    let mut calls: Vec<ToolCall> = Vec::new();
    for (idx, (id, name, args_buf)) in &calls_map {
        if name.is_empty() {
            continue;
        }
        let args: Value = serde_json::from_str(args_buf).unwrap_or_else(|_| json!({}));
        calls.push(ToolCall {
            id: if id.is_empty() {
                format!("call_{assistant_msg_id}_{idx}")
            } else {
                id.clone()
            },
            name: name.clone(),
            args,
        });
    }
    let tool_calls_json = if calls.is_empty() {
        None
    } else {
        Some(Value::Array(
            calls
                .iter()
                .map(|c| json!({ "id": c.id, "name": c.name, "args": c.args }))
                .collect(),
        ))
    };
    let _ = sqlx::query(
        "UPDATE ai_message SET content = $1, tool_calls = $2, prompt_tokens = $3, completion_tokens = $4
         WHERE msg_id = $5",
    )
    .bind(&text)
    .bind(tool_calls_json)
    .bind(prompt_tokens)
    .bind(completion_tokens)
    .bind(assistant_msg_id)
    .execute(pool)
    .await;
    let usage_status = if stopped || stream_broken { 3 } else { 1 };
    record_usage(
        pool,
        tenant,
        conv.conv_id,
        conv.channel_id,
        &conv.model_code,
        prompt_tokens,
        completion_tokens,
        usage_status,
        if stream_broken { "上游流中断" } else { "" },
    )
    .await;
    send_event(
        tx,
        "usage",
        json!({ "promptTokens": prompt_tokens, "completionTokens": completion_tokens }),
    )
    .await;

    if stopped {
        return (RoundKind::Stopped, calls);
    }
    if calls.is_empty() {
        (RoundKind::Text, calls)
    } else {
        (RoundKind::Calls(calls.clone()), calls)
    }
}

// ————————————————————— 引擎主循环 —————————————————————

async fn run_engine(job: EngineJob, mut tx: futures::channel::mpsc::Sender<Event>) {
    let state = job.state.clone();
    let tenant = job.tenant.clone();
    let conv = job.conv.clone();
    let pool = state.repos.pool.clone();

    let (stop_flag, _guard) = match acquire_inflight(tenant.shop_id, conv.conv_id) {
        Ok(pair) => pair,
        Err(e) => {
            send_event(&mut tx, "error", json!({ "message": e })).await;
            send_event(&mut tx, "done", json!({})).await;
            return;
        }
    };

    let settings = chat_settings(&pool, &tenant).await;
    let tools = chat_tools(&tenant, &settings);
    let shop_title: String = if tenant.is_merchant() {
        sqlx::query_as::<_, (String,)>("SELECT shop_title FROM shop WHERE shop_id = $1")
            .bind(tenant.shop_id)
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten()
            .map(|r| r.0)
            .unwrap_or_else(|| "本店".to_string())
    } else {
        String::new()
    };

    let rows = match load_message_rows(&state, conv.conv_id).await {
        Ok(r) => r,
        Err(e) => {
            send_event(&mut tx, "error", json!({ "message": e.0.to_string() })).await;
            send_event(&mut tx, "done", json!({})).await;
            return;
        }
    };
    let (mut messages, mut pending) = rebuild_history(&tenant, &shop_title, &rows);

    let mut rounds = 0;
    loop {
        // 先处理未完成的工具调用（确认续跑 / 同轮多调用）。
        while let Some(call) = pending.pop_front() {
            let m = tool_meta(&call.name);
            let is_write = m.is_some_and(|x| x.risk != ToolRisk::Read);
            if is_write {
                let classified = classify(&state, &settings, &call.name, &call.args).await;
                let needs_confirm = classified.risk == ToolRisk::WriteHigh
                    || !job.skip_confirm.contains(&call.name);
                if needs_confirm {
                    let tool_row = insert_message(
                        &pool,
                        conv.conv_id,
                        3,
                        "",
                        Some(json!({ "callId": call.id, "name": call.name, "args": call.args })),
                        &call.name,
                        1,
                    )
                    .await;
                    match tool_row {
                        Ok(msg_id) => {
                            send_event(
                                &mut tx,
                                "confirm",
                                json!({
                                    "msgId": msg_id,
                                    "callId": call.id,
                                    "tool": call.name,
                                    "title": m.map(|x| x.title).unwrap_or(call.name.as_str()),
                                    "risk": if classified.risk == ToolRisk::WriteHigh { "high" } else { "normal" },
                                    "summary": classified.summary,
                                    "args": call.args,
                                }),
                            )
                            .await;
                        }
                        Err(e) => {
                            send_event(&mut tx, "error", json!({ "message": format!("保存确认请求失败：{e}") })).await;
                        }
                    }
                    send_event(&mut tx, "done", json!({ "awaitingConfirm": true })).await;
                    touch_conv(&pool, conv.conv_id).await;
                    return;
                }
            }
            // 读工具 / 已免确认的普通写：直接执行。
            send_event(
                &mut tx,
                "tool",
                json!({ "callId": call.id, "tool": call.name, "status": "running" }),
            )
            .await;
            let outcome = execute_tool(&state, &tenant, &settings, conv.conv_id, &call, None).await;
            send_event(
                &mut tx,
                "tool",
                json!({
                    "callId": call.id,
                    "tool": call.name,
                    "status": if outcome.status == 2 { "executed" } else { "failed" },
                    "result": truncate_chars(&outcome.model_text, 2000),
                }),
            )
            .await;
            messages.push(json!({
                "role": "tool",
                "tool_call_id": call.id,
                "content": outcome.model_text,
            }));
        }

        rounds += 1;
        if rounds > 8 {
            send_event(&mut tx, "error", json!({ "message": "本轮工具调用次数过多，已停止" })).await;
            send_event(&mut tx, "done", json!({})).await;
            touch_conv(&pool, conv.conv_id).await;
            return;
        }
        let (kind, calls) = upstream_round(&job, &messages, &tools, &mut tx, &stop_flag).await;
        match kind {
            RoundKind::Text => {
                send_event(&mut tx, "done", json!({})).await;
                touch_conv(&pool, conv.conv_id).await;
                return;
            }
            RoundKind::Failed => {
                send_event(&mut tx, "done", json!({})).await;
                touch_conv(&pool, conv.conv_id).await;
                return;
            }
            RoundKind::Stopped => {
                send_event(&mut tx, "done", json!({ "stopped": true })).await;
                touch_conv(&pool, conv.conv_id).await;
                return;
            }
            RoundKind::Calls(_) => {
                // 把 assistant 的 tool_calls 补进上下文，下一圈处理这些调用。
                let oai_calls: Vec<Value> = calls
                    .iter()
                    .map(|c| {
                        json!({
                            "id": c.id,
                            "type": "function",
                            "function": { "name": c.name, "arguments": c.args.to_string() }
                        })
                    })
                    .collect();
                messages.push(json!({
                    "role": "assistant",
                    "content": "",
                    "tool_calls": oai_calls,
                }));
                pending = calls.into_iter().collect();
            }
        }
    }
}

fn sse_response(job: EngineJob) -> Response {
    let (tx, rx) = futures::channel::mpsc::channel::<Event>(128);
    tokio::spawn(async move {
        run_engine(job, tx).await;
    });
    let stream = rx.map(|e| Ok::<Event, std::convert::Infallible>(e));
    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response()
}

// ————————————————————— 入口 handlers —————————————————————

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StreamBody {
    pub conv_id: Option<i32>,
    pub content: String,
    pub channel_id: Option<i32>,
    pub model_code: Option<String>,
    pub skip_confirm_tools: Option<Vec<String>>,
}

pub async fn chat_stream(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<StreamBody>,
) -> Result<Response, nextrade_security::web::AppError> {
    let tenant = AiTenant::from_auth(&auth)?;
    let content = body.content.trim().to_string();
    if content.is_empty() {
        return Err(err_validation("消息内容不能为空"));
    }
    if content.chars().count() > 8000 {
        return Err(err_validation("消息内容过长（上限 8000 字）"));
    }
    let pool = &state.repos.pool;
    let conv = if let Some(cid) = body.conv_id {
        let conv = load_conv(&state, cid).await?;
        if conv.admin_id != tenant.admin_id || conv.shop_id != tenant.shop_id {
            return Err(err_validation("会话不存在"));
        }
        // 切换渠道/模型（随消息指定时）。
        if body.channel_id.is_some() || body.model_code.is_some() {
            let (ch, model) = resolve_channel_model(
                &state,
                &tenant,
                body.channel_id.or(Some(conv.channel_id)),
                body.model_code.clone().or_else(|| Some(conv.model_code.clone())),
            )
            .await
            .map_err(err_validation)?;
            sqlx::query("UPDATE ai_conversation SET channel_id = $1, model_code = $2 WHERE conv_id = $3")
                .bind(ch)
                .bind(&model)
                .bind(cid)
                .execute(pool)
                .await
                .map_err(|e| err_validation(e.to_string()))?;
            ConvRow { channel_id: ch, model_code: model, ..conv }
        } else {
            conv
        }
    } else {
        let (ch, model) = resolve_channel_model(&state, &tenant, body.channel_id, body.model_code.clone())
            .await
            .map_err(err_validation)?;
        insert_conversation(&state, &tenant, ch, &model, "")
            .await
            .map_err(err_validation)?
    };
    // 新消息到来：取消遗留的待确认（视为放弃）。
    let _ = sqlx::query(
        "UPDATE ai_message SET tool_status = 3, content = '该操作未执行（已取消）'
         WHERE conv_id = $1 AND role = 3 AND tool_status = 1",
    )
    .bind(conv.conv_id)
    .execute(pool)
    .await;
    insert_message(pool, conv.conv_id, 1, &content, None, "", 0)
        .await
        .map_err(err_validation)?;
    if conv.title.is_empty() {
        let title: String = content.chars().take(40).collect();
        let _ = sqlx::query("UPDATE ai_conversation SET title = $1 WHERE conv_id = $2")
            .bind(title)
            .bind(conv.conv_id)
            .execute(pool)
            .await;
    }
    touch_conv(pool, conv.conv_id).await;
    let skip: HashSet<String> = body
        .skip_confirm_tools
        .unwrap_or_default()
        .into_iter()
        .collect();
    Ok(sse_response(EngineJob {
        state,
        tenant,
        conv,
        skip_confirm: skip,
    }))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ConfirmBody {
    pub msg_id: i64,
    pub decision: String,
    pub patched_args: Option<Value>,
    /// 批准时随带当前会话的免确认集合，续跑时一并生效（高风险仍被服务端强制拦截）。
    pub skip_confirm_tools: Vec<String>,
}

pub async fn chat_confirm(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<ConfirmBody>,
) -> Result<Response, nextrade_security::web::AppError> {
    let tenant = AiTenant::from_auth(&auth)?;
    let pool = &state.repos.pool;
    let row: Option<MsgRow> = sqlx::query_as(
        "SELECT msg_id, conv_id, role, content, tool_calls, tool_name, tool_status,
                prompt_tokens, completion_tokens, create_time
         FROM ai_message WHERE msg_id = $1 AND role = 3 AND tool_status = 1",
    )
    .bind(body.msg_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| err_validation(e.to_string()))?;
    let Some(row) = row else {
        return Err(err_validation("确认请求已失效或已处理"));
    };
    let conv = load_conv(&state, row.conv_id).await?;
    if conv.admin_id != tenant.admin_id || conv.shop_id != tenant.shop_id {
        return Err(err_validation("会话不存在"));
    }
    let tc = row.tool_calls.clone().unwrap_or_else(|| json!({}));
    let call = ToolCall {
        id: tc.get("callId").and_then(Value::as_str).unwrap_or("").to_string(),
        name: tc
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(row.tool_name.as_str())
            .to_string(),
        args: tc.get("args").cloned().unwrap_or_else(|| json!({})),
    };
    match body.decision.as_str() {
        "reject" => {
            sqlx::query(
                "UPDATE ai_message SET tool_status = 3, content = '用户拒绝了此操作，未执行' WHERE msg_id = $1",
            )
            .bind(body.msg_id)
            .execute(pool)
            .await
            .map_err(|e| err_validation(e.to_string()))?;
            write_audit(pool, &tenant, conv.conv_id, &call.name, &call.args, 5).await;
        }
        "approve" => {
            let mut args = call.args.clone();
            if let Some(patch) = body.patched_args.as_ref() {
                if let (Some(obj), Some(pobj)) = (args.as_object_mut(), patch.as_object()) {
                    for (k, v) in pobj {
                        obj.insert(k.clone(), v.clone());
                    }
                } else if patch.is_object() {
                    args = patch.clone();
                }
            }
            let exec_call = ToolCall { args, ..call.clone() };
            let settings = chat_settings(pool, &tenant).await;
            execute_tool(&state, &tenant, &settings, conv.conv_id, &exec_call, Some(body.msg_id)).await;
        }
        _ => return Err(err_validation("decision 只能是 approve 或 reject")),
    }
    Ok(sse_response(EngineJob {
        state,
        tenant,
        conv,
        skip_confirm: body
            .skip_confirm_tools
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
    }))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StopBody {
    pub conv_id: i32,
}

pub async fn chat_stop(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<StopBody>,
) -> ApiResult<Value> {
    let tenant = AiTenant::from_auth(&auth)?;
    let conv = load_conv(&state, body.conv_id).await?;
    if conv.admin_id != tenant.admin_id || conv.shop_id != tenant.shop_id {
        return Err(err_validation("会话不存在"));
    }
    if let Ok(map) = stop_flags().lock() {
        if let Some(flag) = map.get(&body.conv_id) {
            flag.store(true, Ordering::Relaxed);
        }
    }
    ok(json!({ "stopped": true }))
}

// ————————————————————— 单元测试 —————————————————————

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_covers_all_mcp_tools_plus_deliver() {
        // 注册表必须覆盖全部工具名（35 = MCP 34 + order_deliver），且名字唯一。
        let reg = tool_registry();
        assert_eq!(reg.len(), 35);
        let mut names: Vec<&str> = reg.iter().map(|m| m.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 35);
    }

    #[test]
    fn platform_only_tools_are_reads() {
        for m in tool_registry().into_iter().filter(|m| m.platform_only) {
            assert_eq!(m.risk, ToolRisk::Read, "{} 应为只读", m.name);
        }
    }

    #[test]
    fn high_risk_set_matches_design() {
        let high: Vec<&str> = tool_registry()
            .into_iter()
            .filter(|m| m.risk == ToolRisk::WriteHigh)
            .map(|m| m.name)
            .collect();
        assert_eq!(high, vec!["product_batch_execute", "product_import", "decorate_publish"]);
    }

    #[test]
    fn truncate_chars_respects_char_boundary() {
        assert_eq!(truncate_chars("你好世界", 2), "你好…（已截断）");
        assert_eq!(truncate_chars("abc", 10), "abc");
    }
}
