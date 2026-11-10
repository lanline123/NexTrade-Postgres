//! AI 运营 MCP（商品域 + 装修域）。
//!
//! 设计要点：
//! - **总开关 + 分域开关**：`enabled` 关闭时 MCP 端点直接 403；商品/装修各自独立开关。
//! - **专用 Token**：MCP 客户端用 `Authorization: Bearer <token>` 访问，不复用管理员 JWT。
//! - **只走业务服务**：工具调用复用现有 service/repo（校验、库存、软删除语义不变），不直连 SQL 改业务表。
//! - **审计与审批**：每次调用写 `ai_mcp_log`；改价超阈值、发布装修等高风险动作进入
//!   `ai_mcp_approval`，后台三级页审批后才真正执行。
//!
//! 管理端接口（需管理员登录）：
//! - `GET/POST /ai/mcp/settings`、`POST /ai/mcp/settings/token`
//! - `GET /ai/mcp/logs`、`GET /ai/mcp/approvals`、`POST /ai/mcp/approvals/approve|reject`
//!
//! MCP 端点（免管理员 JWT，Bearer Token 鉴权）：`POST /adminapi/ai/mcp`（JSON-RPC 2.0）。

use std::time::Instant;

use axum::{
    extract::State,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use axum::extract::Query;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use nextrade_core::models::catalog::product_manage::AdminProductListQuery;
use nextrade_core::models::content::decorate::{DecorateListQuery, DecorateSaveDTO};
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};

use crate::AdminState;

const SETTINGS_CODE: &str = "aiMcpSettings";

fn now_secs() -> i32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i32)
        .unwrap_or(0)
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ————————————————————— 设置 —————————————————————

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct McpSettings {
    /// MCP 总开关（关闭后端点返回 403）
    pub enabled: bool,
    /// 访问令牌（Bearer）
    pub token: String,
    /// 商品域权限
    pub product_enabled: bool,
    /// 装修域权限
    pub decorate_enabled: bool,
    /// 改价幅度阈值（百分比，0=不限制）
    pub price_change_pct: f64,
    /// 改价金额阈值（0=不限制）
    pub price_change_amount: f64,
    /// 单次库存变更上限（0=不限制）
    pub stock_change_cap: i64,
    /// 改价是否需要审批
    pub require_approval_price: bool,
    /// 发布装修是否需要审批
    pub require_approval_publish: bool,
}

impl Default for McpSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            token: String::new(),
            product_enabled: true,
            decorate_enabled: false,
            price_change_pct: 10.0,
            price_change_amount: 1000.0,
            stock_change_cap: 1000,
            require_approval_price: true,
            require_approval_publish: true,
        }
    }
}

async fn load_settings(pool: &sqlx::PgPool) -> McpSettings {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1",
    )
    .bind(SETTINGS_CODE)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    row.and_then(|(v,)| serde_json::from_str(&v).ok())
        .unwrap_or_default()
}

async fn save_settings(pool: &sqlx::PgPool, settings: &McpSettings) -> Result<(), String> {
    let value = serde_json::to_string(settings).map_err(|e| e.to_string())?;
    let now = now_millis();
    let r = sqlx::query(
        "UPDATE config SET biz_val = $1, update_time = $2 WHERE biz_code = $3 AND is_del = 0",
    )
    .bind(&value)
    .bind(now)
    .bind(SETTINGS_CODE)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    if r.rows_affected() == 0 {
        sqlx::query(
            "INSERT INTO config (biz_code, biz_val, create_time, create_by_id, create_by_name,
                                 update_time, update_by_id, update_by_name, is_del)
             VALUES ($1, $2, $3, 0, 'ai', $4, 0, 'ai', 0)",
        )
        .bind(SETTINGS_CODE)
        .bind(&value)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn generate_token() -> String {
    format!("mcp_{}", uuid::Uuid::new_v4().simple())
}

// ————————————————————— 路由 —————————————————————

/// 管理端接口（挂在需登录的 `/adminapi` 下）。
pub fn routes(_state: &AdminState) -> Router<AdminState> {
    Router::new()
        .route("/ai/mcp/settings", get(settings_get).post(settings_save))
        .route("/ai/mcp/settings/token", post(token_regenerate))
        .route("/ai/mcp/logs", get(logs_list))
        .route("/ai/mcp/approvals", get(approvals_list))
        .route("/ai/mcp/approvals/approve", post(approval_approve))
        .route("/ai/mcp/approvals/reject", post(approval_reject))
}

/// MCP 端点（免管理员 JWT，靠 Bearer Token）。
pub fn public_routes(_state: &AdminState) -> Router<AdminState> {
    Router::new().route("/ai/mcp", post(mcp_handler))
}

// ————————————————————— 管理端 handler —————————————————————

async fn settings_get(State(state): State<AdminState>) -> ApiResult<Value> {
    let settings = load_settings(&state.repos.pool).await;
    ok(json!({
        "enabled": settings.enabled,
        "token": settings.token,
        "productEnabled": settings.product_enabled,
        "decorateEnabled": settings.decorate_enabled,
        "priceChangePct": settings.price_change_pct,
        "priceChangeAmount": settings.price_change_amount,
        "stockChangeCap": settings.stock_change_cap,
        "requireApprovalPrice": settings.require_approval_price,
        "requireApprovalPublish": settings.require_approval_publish,
        "endpoint": "/adminapi/ai/mcp",
    }))
}

async fn settings_save(
    State(state): State<AdminState>,
    Json(mut body): Json<McpSettings>,
) -> ApiResult<Value> {
    if body.token.trim().is_empty() {
        body.token = generate_token();
    }
    save_settings(&state.repos.pool, &body)
        .await
        .map_err(|e| AppError(nextrade_core::Error::Validation(e)))?;
    ok(json!({ "saved": true, "token": body.token }))
}

async fn token_regenerate(State(state): State<AdminState>) -> ApiResult<Value> {
    let mut settings = load_settings(&state.repos.pool).await;
    settings.token = generate_token();
    save_settings(&state.repos.pool, &settings)
        .await
        .map_err(|e| AppError(nextrade_core::Error::Validation(e)))?;
    ok(json!({ "token": settings.token }))
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    #[serde(default)]
    page: Option<u64>,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    status: Option<String>,
}

async fn logs_list(
    State(state): State<AdminState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Value> {
    let page = q.page.unwrap_or(1).max(1);
    let size = q.size.unwrap_or(20).clamp(1, 200);
    let offset = (page - 1) * size;
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ai_mcp_log")
        .fetch_one(&state.repos.pool)
        .await
        .map_err(|e| AppError(nextrade_core::Error::Validation(e.to_string())))?;
    let rows: Vec<(i32, String, Option<String>, String, Option<String>, i32, i32)> =
        sqlx::query_as(
            "SELECT log_id, tool, args, status, error, duration_ms, add_time
             FROM ai_mcp_log ORDER BY log_id DESC LIMIT $1 OFFSET $2",
        )
        .bind(size as i64)
        .bind(offset as i64)
        .fetch_all(&state.repos.pool)
        .await
        .map_err(|e| AppError(nextrade_core::Error::Validation(e.to_string())))?;
    let records: Vec<Value> = rows
        .into_iter()
        .map(|(id, tool, args, status, error, duration_ms, add_time)| {
            json!({
                "logId": id, "tool": tool, "args": args, "status": status,
                "error": error, "durationMs": duration_ms, "addTime": add_time,
            })
        })
        .collect();
    ok(json!({ "records": records, "total": total.0, "page": page, "size": size }))
}

async fn approvals_list(
    State(state): State<AdminState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Value> {
    let page = q.page.unwrap_or(1).max(1);
    let size = q.size.unwrap_or(20).clamp(1, 200);
    let offset = (page - 1) * size;
    let status = q.status.unwrap_or_else(|| "pending".to_string());
    let total: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ai_mcp_approval WHERE status = $1")
            .bind(&status)
            .fetch_one(&state.repos.pool)
            .await
            .map_err(|e| AppError(nextrade_core::Error::Validation(e.to_string())))?;
    let rows: Vec<(i32, String, String, String, i32, Option<i32>, Option<i32>, Option<String>)> =
        sqlx::query_as(
            "SELECT approval_id, tool, args, status, add_time, approve_time, approve_by, result
             FROM ai_mcp_approval WHERE status = $1
             ORDER BY approval_id DESC LIMIT $2 OFFSET $3",
        )
        .bind(&status)
        .bind(size as i64)
        .bind(offset as i64)
        .fetch_all(&state.repos.pool)
        .await
        .map_err(|e| AppError(nextrade_core::Error::Validation(e.to_string())))?;
    let records: Vec<Value> = rows
        .into_iter()
        .map(|(id, tool, args, status, add_time, approve_time, approve_by, result)| {
            json!({
                "approvalId": id, "tool": tool, "args": args, "status": status,
                "addTime": add_time, "approveTime": approve_time,
                "approveBy": approve_by, "result": result,
            })
        })
        .collect();
    ok(json!({ "records": records, "total": total.0, "page": page, "size": size }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApprovalAction {
    approval_id: i32,
}

async fn approval_approve(
    State(state): State<AdminState>,
    Extension(user): Extension<AuthUser>,
    Json(body): Json<ApprovalAction>,
) -> ApiResult<Value> {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT tool, args FROM ai_mcp_approval WHERE approval_id = $1 AND status = 'pending'",
    )
    .bind(body.approval_id)
    .fetch_optional(&state.repos.pool)
    .await
    .map_err(|e| AppError(nextrade_core::Error::Validation(e.to_string())))?;
    let Some((tool, args_text)) = row else {
        return Err(AppError(nextrade_core::Error::Validation("审批单不存在或已处理".to_string())));
    };
    let args: Value = serde_json::from_str(&args_text).unwrap_or(Value::Null);
    let settings = load_settings(&state.repos.pool).await;
    let result = call_tool(&state, &settings, &tool, &args, true).await;
    let now = now_secs();
    match result {
        Ok(value) => {
            sqlx::query(
                "UPDATE ai_mcp_approval SET status = 'approved', approve_time = $1,
                    approve_by = $2, result = $3 WHERE approval_id = $4",
            )
            .bind(now)
            .bind(user.id)
            .bind(value.to_string())
            .bind(body.approval_id)
            .execute(&state.repos.pool)
            .await
            .map_err(|e| AppError(nextrade_core::Error::Validation(e.to_string())))?;
            ok(json!({ "approved": true, "result": value }))
        }
        Err(err) => {
            sqlx::query(
                "UPDATE ai_mcp_approval SET status = 'failed', approve_time = $1,
                    approve_by = $2, result = $3 WHERE approval_id = $4",
            )
            .bind(now)
            .bind(user.id)
            .bind(&err)
            .bind(body.approval_id)
            .execute(&state.repos.pool)
            .await
            .map_err(|e| AppError(nextrade_core::Error::Validation(e.to_string())))?;
            Err(AppError(nextrade_core::Error::Validation(err)))
        }
    }
}

async fn approval_reject(
    State(state): State<AdminState>,
    Extension(user): Extension<AuthUser>,
    Json(body): Json<ApprovalAction>,
) -> ApiResult<Value> {
    let now = now_secs();
    let r = sqlx::query(
        "UPDATE ai_mcp_approval SET status = 'rejected', approve_time = $1, approve_by = $2
         WHERE approval_id = $3 AND status = 'pending'",
    )
    .bind(now)
    .bind(user.id)
    .bind(body.approval_id)
    .execute(&state.repos.pool)
    .await
    .map_err(|e| AppError(nextrade_core::Error::Validation(e.to_string())))?;
    ok(json!({ "rejected": r.rows_affected() > 0 }))
}

// ————————————————————— MCP JSON-RPC —————————————————————

async fn mcp_handler(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let settings = load_settings(&state.repos.pool).await;
    if !settings.enabled {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": "MCP 未启用" }))).into_response();
    }
    let token = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    if settings.token.is_empty() || token != settings.token {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "invalid token" }))).into_response();
    }

    let id = body.get("id").cloned().unwrap_or(Value::Null);
    let method = body.get("method").and_then(Value::as_str).unwrap_or("");
    match method {
        "initialize" => rpc_ok(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "nextrade-admin-mcp", "version": "1.0.0" }
            }),
        ),
        "notifications/initialized" => StatusCode::ACCEPTED.into_response(),
        "ping" => rpc_ok(id, json!({})),
        "tools/list" => rpc_ok(id, json!({ "tools": tools_for(&settings) })),
        "tools/call" => {
            let name = body
                .pointer("/params/name")
                .and_then(Value::as_str)
                .unwrap_or("");
            let args = body
                .pointer("/params/arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let started = Instant::now();
            let result = call_tool(&state, &settings, name, &args, false).await;
            let duration = started.elapsed().as_millis() as i32;
            let (status, error) = match &result {
                Ok(_) => ("ok".to_string(), None),
                Err(err) => ("error".to_string(), Some(err.clone())),
            };
            let _ = sqlx::query(
                "INSERT INTO ai_mcp_log (tool, args, status, error, duration_ms, add_time)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(name)
            .bind(args.to_string())
            .bind(&status)
            .bind(error)
            .bind(duration)
            .bind(now_secs())
            .execute(&state.repos.pool)
            .await;
            match result {
                Ok(value) => rpc_ok(
                    id,
                    json!({ "content": [{ "type": "text", "text": value.to_string() }], "isError": false }),
                ),
                Err(err) => rpc_ok(
                    id,
                    json!({ "content": [{ "type": "text", "text": err }], "isError": true }),
                ),
            }
        }
        other => rpc_error(id, -32601, &format!("method not found: {other}")),
    }
}

fn rpc_ok(id: Value, result: Value) -> Response {
    (
        StatusCode::OK,
        Json(json!({ "jsonrpc": "2.0", "id": id, "result": result })),
    )
        .into_response()
}

fn rpc_error(id: Value, code: i64, message: &str) -> Response {
    (
        StatusCode::OK,
        Json(json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })),
    )
        .into_response()
}

// ————————————————————— 工具目录 —————————————————————

fn tools_for(settings: &McpSettings) -> Vec<Value> {
    let mut tools = Vec::new();
    if settings.product_enabled {
        tools.extend([
            json!({
                "name": "product_search",
                "description": "分页搜索商品（商品名/货号/关键词），只读。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "keyword": { "type": "string" },
                        "page": { "type": "integer" },
                        "size": { "type": "integer" },
                        "productStatus": { "type": "integer", "description": "1 上架 0 下架" }
                    },
                    "additionalProperties": false
                }
            }),
            json!({
                "name": "product_get",
                "description": "按商品 ID 读取详情（含 SKU/属性），只读。",
                "inputSchema": {
                    "type": "object",
                    "properties": { "productId": { "type": "integer" } },
                    "required": ["productId"],
                    "additionalProperties": false
                }
            }),
            json!({
                "name": "product_list_categories",
                "description": "读取分类下拉（含品牌），只读。",
                "inputSchema": { "type": "object", "properties": {} }
            }),
            json!({
                "name": "product_list_brands",
                "description": "读取品牌下拉，只读。",
                "inputSchema": { "type": "object", "properties": {} }
            }),
            json!({
                "name": "product_list_skus",
                "description": "读取商品 SKU 列表，只读。",
                "inputSchema": {
                    "type": "object",
                    "properties": { "productId": { "type": "integer" } },
                    "required": ["productId"]
                }
            }),
            json!({
                "name": "product_update_fields",
                "description": "更新商品基础字段（名称/简介/分类/品牌/单位），白名单字段。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "productId": { "type": "integer" },
                        "productName": { "type": "string" },
                        "productBrief": { "type": "string" },
                        "categoryId": { "type": "integer" },
                        "brandId": { "type": "integer" },
                        "productUnit": { "type": "string" }
                    },
                    "required": ["productId"],
                    "additionalProperties": false
                }
            }),
            json!({
                "name": "product_update_price",
                "description": "修改商品售价/市场价；超过幅度或金额阈值会转审批。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "productId": { "type": "integer" },
                        "productPrice": { "type": "number" },
                        "marketPrice": { "type": "number" }
                    },
                    "required": ["productId", "productPrice"],
                    "additionalProperties": false
                }
            }),
            json!({
                "name": "product_update_stock",
                "description": "修改商品库存；超过单次变更上限会转审批。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "productId": { "type": "integer" },
                        "productStock": { "type": "integer" }
                    },
                    "required": ["productId", "productStock"],
                    "additionalProperties": false
                }
            }),
            json!({
                "name": "product_publish",
                "description": "商品上架（product_status=1）。",
                "inputSchema": {
                    "type": "object",
                    "properties": { "productId": { "type": "integer" } },
                    "required": ["productId"]
                }
            }),
            json!({
                "name": "product_unpublish",
                "description": "商品下架（product_status=0），不改删除标记。",
                "inputSchema": {
                    "type": "object",
                    "properties": { "productId": { "type": "integer" } },
                    "required": ["productId"]
                }
            }),
        ]);
    }
    if settings.decorate_enabled {
        tools.extend([
            json!({
                "name": "decorate_list_pages",
                "description": "分页列出 PC 装修页，只读。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "keyword": { "type": "string" },
                        "page": { "type": "integer" },
                        "size": { "type": "integer" }
                    }
                }
            }),
            json!({
                "name": "decorate_get_page",
                "description": "读取装修页详情（含草稿与已发布数据），只读。",
                "inputSchema": {
                    "type": "object",
                    "properties": { "decorateId": { "type": "integer" } },
                    "required": ["decorateId"]
                }
            }),
            json!({
                "name": "decorate_save_draft",
                "description": "保存装修草稿（data 为页面模块 JSON），不发布。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "decorateId": { "type": "integer" },
                        "data": { "type": "object" }
                    },
                    "required": ["decorateId", "data"]
                }
            }),
            json!({
                "name": "decorate_publish",
                "description": "发布装修页（高风险，默认转审批）。",
                "inputSchema": {
                    "type": "object",
                    "properties": { "decorateId": { "type": "integer" } },
                    "required": ["decorateId"]
                }
            }),
        ]);
    }
    tools
}

// ————————————————————— 工具执行 —————————————————————

fn arg_i32(args: &Value, key: &str) -> Option<i32> {
    args.get(key).and_then(Value::as_i64).map(|v| v as i32)
}

fn arg_str(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn arg_dec(args: &Value, key: &str) -> Option<Decimal> {
    match args.get(key) {
        Some(Value::Number(n)) => n.as_f64().and_then(Decimal::from_f64_retain),
        Some(Value::String(s)) => s.trim().parse::<Decimal>().ok(),
        _ => None,
    }
}

fn require_i32(args: &Value, key: &str) -> Result<i32, String> {
    arg_i32(args, key).filter(|v| *v > 0).ok_or_else(|| format!("缺少参数 {key}"))
}

fn pending(approval_id: i32) -> Value {
    json!({ "status": "pending_approval", "approvalId": approval_id })
}

async fn create_approval(
    state: &AdminState,
    tool: &str,
    args: &Value,
) -> Result<i32, String> {
    let row: (i32,) = sqlx::query_as(
        "INSERT INTO ai_mcp_approval (tool, args, status, add_time)
         VALUES ($1, $2, 'pending', $3) RETURNING approval_id",
    )
    .bind(tool)
    .bind(args.to_string())
    .bind(now_secs())
    .fetch_one(&state.repos.pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(row.0)
}

async fn call_tool(
    state: &AdminState,
    settings: &McpSettings,
    name: &str,
    args: &Value,
    approved: bool,
) -> Result<Value, String> {
    match name {
        "product_search" => {
            if !settings.product_enabled {
                return Err("商品域未启用".into());
            }
            let q = AdminProductListQuery {
                keyword: arg_str(args, "keyword"),
                page: args.get("page").and_then(Value::as_u64),
                page_size: args.get("size").and_then(Value::as_u64),
                product_status: arg_i32(args, "productStatus"),
                ..Default::default()
            };
            let page = state
                .catalog
                .product_manage
                .admin_list(&q)
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_value(page).map_err(|e| e.to_string())
        }
        "product_get" => {
            if !settings.product_enabled {
                return Err("商品域未启用".into());
            }
            let id = require_i32(args, "productId")?;
            let detail = state
                .catalog
                .product_manage
                .admin_detail(id)
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_value(detail).map_err(|e| e.to_string())
        }
        "product_list_categories" | "product_list_brands" => {
            if !settings.product_enabled {
                return Err("商品域未启用".into());
            }
            let vo = state
                .catalog
                .product_manage
                .cat_and_brand_list()
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_value(vo).map_err(|e| e.to_string())
        }
        "product_list_skus" => {
            if !settings.product_enabled {
                return Err("商品域未启用".into());
            }
            let id = require_i32(args, "productId")?;
            let skus = state
                .repos
                .catalog
                .product_manage
                .find_skus_by_product_ids(&[id], false)
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_value(skus).map_err(|e| e.to_string())
        }
        "product_update_fields" => {
            if !settings.product_enabled {
                return Err("商品域未启用".into());
            }
            let id = require_i32(args, "productId")?;
            let name = arg_str(args, "productName");
            let brief = arg_str(args, "productBrief");
            let category = arg_i32(args, "categoryId");
            let brand = arg_i32(args, "brandId");
            let unit = arg_str(args, "productUnit");
            if name.is_none() && brief.is_none() && category.is_none() && brand.is_none() && unit.is_none() {
                return Err("没有可更新的字段".into());
            }
            let rows = state
                .repos
                .catalog
                .product_manage
                .ai_update_fields(id, name.as_deref(), brief.as_deref(), category, brand, unit.as_deref())
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({ "updatedRows": rows }))
        }
        "product_update_price" => {
            if !settings.product_enabled {
                return Err("商品域未启用".into());
            }
            let id = require_i32(args, "productId")?;
            let price = arg_dec(args, "productPrice").ok_or_else(|| "缺少参数 productPrice".to_string())?;
            let market = arg_dec(args, "marketPrice");
            let current = state
                .repos
                .catalog
                .product_manage
                .ai_product_price(id)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "商品不存在".to_string())?;
            let old = current.0;
            let delta = (price - old).abs();
            let pct_limit = Decimal::from_f64_retain(settings.price_change_pct).unwrap_or_default();
            let amt_limit = Decimal::from_f64_retain(settings.price_change_amount).unwrap_or_default();
            let over_pct = pct_limit > Decimal::ZERO && old > Decimal::ZERO && delta > old * pct_limit / Decimal::from(100i64);
            let over_amt = amt_limit > Decimal::ZERO && delta > amt_limit;
            if settings.require_approval_price && !approved && (over_pct || over_amt) {
                let approval_id = create_approval(state, name, args).await?;
                return Ok(pending(approval_id));
            }
            let rows = state
                .repos
                .catalog
                .product_manage
                .ai_update_price(id, price, market)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({ "updatedRows": rows, "oldPrice": old.to_string(), "newPrice": price.to_string() }))
        }
        "product_update_stock" => {
            if !settings.product_enabled {
                return Err("商品域未启用".into());
            }
            let id = require_i32(args, "productId")?;
            let stock = args
                .get("productStock")
                .and_then(Value::as_i64)
                .map(|v| v as i32)
                .ok_or_else(|| "缺少参数 productStock".to_string())?;
            if stock < 0 {
                return Err("库存不能为负数".into());
            }
            let old = state
                .repos
                .catalog
                .product_manage
                .ai_product_stock(id)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "商品不存在".to_string())?;
            let delta = (stock - old).abs() as i64;
            if settings.stock_change_cap > 0 && delta > settings.stock_change_cap && !approved {
                let approval_id = create_approval(state, name, args).await?;
                return Ok(pending(approval_id));
            }
            let rows = state
                .repos
                .catalog
                .product_manage
                .ai_update_stock(id, stock)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({ "updatedRows": rows, "oldStock": old, "newStock": stock }))
        }
        "product_publish" | "product_unpublish" => {
            if !settings.product_enabled {
                return Err("商品域未启用".into());
            }
            let id = require_i32(args, "productId")?;
            let status = if name == "product_publish" { 1 } else { 0 };
            let rows = state
                .repos
                .catalog
                .product_manage
                .set_product_flags(&[id], Some(status), None, None)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({ "updatedRows": rows, "productStatus": status }))
        }
        "decorate_list_pages" => {
            if !settings.decorate_enabled {
                return Err("装修域未启用".into());
            }
            let q = DecorateListQuery {
                keyword: arg_str(args, "keyword"),
                decorate_type: Some("2".to_string()),
                page: args.get("page").and_then(Value::as_u64),
                page_size: args.get("size").and_then(Value::as_i64),
                ..Default::default()
            };
            let page = state
                .content
                .decorate
                .list(&q)
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_value(page).map_err(|e| e.to_string())
        }
        "decorate_get_page" => {
            if !settings.decorate_enabled {
                return Err("装修域未启用".into());
            }
            let id = require_i32(args, "decorateId")?;
            let vo = state
                .content
                .decorate
                .detail(Some(id), None, None, Some(2))
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_value(vo).map_err(|e| e.to_string())
        }
        "decorate_save_draft" => {
            if !settings.decorate_enabled {
                return Err("装修域未启用".into());
            }
            let id = require_i32(args, "decorateId")?;
            let data = args
                .get("data")
                .filter(|v| v.is_object())
                .cloned()
                .ok_or_else(|| "缺少参数 data（对象）".to_string())?;
            let dto = DecorateSaveDTO {
                id: Some(id),
                data: Some(data),
                parent_id: None,
                locale_id: None,
            };
            let saved = state
                .content
                .decorate
                .save_draft(&dto)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({ "saved": saved }))
        }
        "decorate_publish" => {
            if !settings.decorate_enabled {
                return Err("装修域未启用".into());
            }
            let id = require_i32(args, "decorateId")?;
            if settings.require_approval_publish && !approved {
                let approval_id = create_approval(state, name, args).await?;
                return Ok(pending(approval_id));
            }
            let vo = state
                .content
                .decorate
                .detail(Some(id), None, None, Some(2))
                .await
                .map_err(|e| e.to_string())?;
            let data = vo
                .draft_data
                .clone()
                .or_else(|| vo.data.clone())
                .ok_or_else(|| "装修页没有可发布的数据".to_string())?;
            let dto = DecorateSaveDTO {
                id: Some(id),
                data: Some(data),
                parent_id: None,
                locale_id: None,
            };
            let published = state
                .content
                .decorate
                .publish(&dto)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({ "published": published }))
        }
        other => Err(format!("未知工具: {other}")),
    }
}
