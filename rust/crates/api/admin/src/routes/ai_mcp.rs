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

use std::net::IpAddr;
use std::time::{Duration, Instant};

use base64::Engine as _;

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

use nextrade_core::models::catalog::category::CategoryDTO;
use nextrade_core::models::catalog::product_manage::{AdminProductListQuery, GalleryPicDTO, ProductSaveDTO};
use nextrade_core::models::content::decorate::{DecorateListQuery, DecorateSaveDTO};
use nextrade_core::models::content::gallery::{
    GalleryDTO, GalleryDelDTO, GalleryScope, GalleryUpdateDTO,
};
use nextrade_core::models::content::gallery_pic::{
    GalleryPicBatchDTO, GalleryPicTransferDTO, UpdateFieldDTO,
};
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
    /// 报表域权限（只读）
    pub analytics_enabled: bool,
    /// 订单域权限（查询 + 后台备注）
    pub order_enabled: bool,
    /// 会员域权限（只读）
    pub member_enabled: bool,
    /// 内容域权限（只读）
    pub content_enabled: bool,
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
    /// 商品批量操作（回收站/彻底删除）是否需要审批
    pub require_approval_batch: bool,
    /// 商品批量导入是否需要审批
    pub require_approval_import: bool,
}

impl Default for McpSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            token: String::new(),
            product_enabled: true,
            decorate_enabled: false,
            analytics_enabled: true,
            order_enabled: false,
            member_enabled: false,
            content_enabled: false,
            price_change_pct: 10.0,
            price_change_amount: 1000.0,
            stock_change_cap: 1000,
            require_approval_price: true,
            require_approval_publish: true,
            require_approval_batch: true,
            require_approval_import: true,
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
        "analyticsEnabled": settings.analytics_enabled,
        "orderEnabled": settings.order_enabled,
        "memberEnabled": settings.member_enabled,
        "contentEnabled": settings.content_enabled,
        "priceChangePct": settings.price_change_pct,
        "priceChangeAmount": settings.price_change_amount,
        "stockChangeCap": settings.stock_change_cap,
        "requireApprovalPrice": settings.require_approval_price,
        "requireApprovalPublish": settings.require_approval_publish,
        "requireApprovalBatch": settings.require_approval_batch,
        "requireApprovalImport": settings.require_approval_import,
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
            json!({
                "name": "product_batch_preview",
                "description": "批量操作预览（只读）：返回可执行与不可执行商品清单。operation: up/down/recycle/restore/del，单次最多 50 个。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "productIds": {
                            "type": "array",
                            "items": { "type": "integer" },
                            "description": "商品 ID 列表"
                        },
                        "operation": {
                            "type": "string",
                            "enum": ["up", "down", "recycle", "restore", "del"]
                        }
                    },
                    "required": ["productIds", "operation"],
                    "additionalProperties": false
                }
            }),
            json!({
                "name": "product_batch_execute",
                "description": "执行批量操作（up/down/recycle/restore/del）。必须传 confirmation:true；移入回收站/彻底删除默认转审批。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "productIds": {
                            "type": "array",
                            "items": { "type": "integer" },
                            "description": "商品 ID 列表"
                        },
                        "operation": {
                            "type": "string",
                            "enum": ["up", "down", "recycle", "restore", "del"]
                        },
                        "confirmation": {
                            "type": "boolean",
                            "description": "确认执行，必须为 true"
                        }
                    },
                    "required": ["productIds", "operation", "confirmation"],
                    "additionalProperties": false
                }
            }),
            json!({
                "name": "product_import",
                "description": "批量创建商品（默认转审批，单次最多 50 个）。productName 与 productPrice 必填。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "products": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "productName": { "type": "string" },
                                    "productSn": { "type": "string", "description": "货号，留空自动生成" },
                                    "productPrice": { "type": "number" },
                                    "marketPrice": { "type": "number" },
                                    "productStock": { "type": "integer" },
                                    "categoryId": { "type": "integer" },
                                    "brandId": { "type": "integer" },
                                    "productBrief": { "type": "string" },
                                    "productStatus": { "type": "integer", "description": "1 上架 0 下架（默认 0）" },
                                    "picUrl": { "type": "string", "description": "主图地址" },
                                    "images": {
                                        "type": "array",
                                        "items": { "type": "string" },
                                        "description": "相册图片地址，首张作为主图"
                                    }
                                },
                                "required": ["productName", "productPrice"]
                            }
                        },
                        "confirmation": {
                            "type": "boolean",
                            "description": "确认导入，必须为 true"
                        }
                    },
                    "required": ["products", "confirmation"],
                    "additionalProperties": false
                }
            }),
            json!({
                "name": "product_import_validate",
                "description": "导入前校验商品草稿（只读）：检查名称/价格必填、货号是否重复，返回逐条问题与整体可导入结论。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "products": {
                            "type": "array",
                            "items": { "type": "object" },
                            "description": "与 product_import 的 products 同结构"
                        }
                    },
                    "required": ["products"],
                    "additionalProperties": false
                }
            }),
            json!({
                "name": "product_copy",
                "description": "复制商品为新商品（货号追加 -copy-时间戳，库存 0，默认下架）。",
                "inputSchema": {
                    "type": "object",
                    "properties": { "productId": { "type": "integer" } },
                    "required": ["productId"],
                    "additionalProperties": false
                }
            }),
            json!({
                "name": "product_analysis",
                "description": "商品经营分析（只读）：overview/top_sellers/slow_movers/inventory_risk/price_bands/category_performance/recommend。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "operation": {
                            "type": "string",
                            "enum": ["overview", "top_sellers", "slow_movers", "inventory_risk",
                                     "price_bands", "category_performance", "recommend"],
                            "description": "默认 overview"
                        },
                        "keyword": { "type": "string" },
                        "categoryId": { "type": "integer" },
                        "limit": { "type": "integer", "description": "默认 20，最大 100" }
                    },
                    "additionalProperties": false
                }
            }),
            json!({
                "name": "product_context",
                "description": "读取同类商品样本的价位/库存/销量统计（只读），用于导入前定价与选品参考。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "keyword": { "type": "string" },
                        "categoryId": { "type": "integer" },
                        "sampleSize": { "type": "integer", "description": "默认 100，最大 500" }
                    },
                    "additionalProperties": false
                }
            }),
            json!({
                "name": "product_category_create",
                "description": "按路径批量创建商品分类（如 \"电池/锂电/18650\"），已存在的层级直接复用。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "paths": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "斜杠分隔的分类路径，单次最多 50 条"
                        }
                    },
                    "required": ["paths"],
                    "additionalProperties": false
                }
            }),
        ]);
    }
    if settings.product_enabled || settings.decorate_enabled {
        tools.push(json!({
            "name": "media_upload",
            "description": "把公网图片 URL 或 base64 图片保存到素材中心，返回图片地址与 picId（jpg/png/gif/webp/tiff，≤20MB）。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "http/https 图片地址，与 base64 二选一" },
                    "base64": { "type": "string", "description": "图片 base64，可带 data:image/png;base64, 前缀" },
                    "fileName": { "type": "string", "description": "文件名，缺省从 URL/类型推断" },
                    "galleryId": { "type": "integer", "description": "素材分组 ID，缺省 0（未分组）" }
                },
                "additionalProperties": false
            }
        }));
        tools.push(json!({
            "name": "media_manage",
            "description": "素材管理：分组创建/重命名/删除，素材移动分组/重命名/删除。operation: group_create/group_update/group_delete/update/delete。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["group_create", "group_update", "group_delete", "update", "delete"]
                    },
                    "groupId": { "type": "integer", "description": "分组 ID（group_update/group_delete）" },
                    "groupName": { "type": "string", "description": "分组名称（group_create/group_update）" },
                    "deleteMode": {
                        "type": "string",
                        "enum": ["only_group", "group_and_files"],
                        "description": "group_delete 模式，默认 only_group"
                    },
                    "picIds": {
                        "type": "array",
                        "items": { "type": "integer" },
                        "description": "素材 ID（update/delete）"
                    },
                    "galleryId": { "type": "integer", "description": "目标分组（update 时移动）" },
                    "picName": { "type": "string", "description": "新素材名（update 时重命名）" }
                },
                "required": ["operation"],
                "additionalProperties": false
            }
        }));
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
    if settings.analytics_enabled {
        tools.extend([
            json!({
                "name": "analytics_sales_summary",
                "description": "统计近 N 天已支付订单数/金额，只读。",
                "inputSchema": {
                    "type": "object",
                    "properties": { "days": { "type": "integer", "description": "默认 7，最大 365" } }
                }
            }),
            json!({
                "name": "analytics_top_products",
                "description": "近 N 天销量 Top 商品（按件数），只读。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "days": { "type": "integer", "description": "默认 30" },
                        "limit": { "type": "integer", "description": "默认 10，最大 50" }
                    }
                }
            }),
            json!({
                "name": "analytics_low_stock",
                "description": "在售且库存低于阈值的商品，只读。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "threshold": { "type": "integer", "description": "默认 10" },
                        "limit": { "type": "integer", "description": "默认 20，最大 100" }
                    }
                }
            }),
        ]);
    }
    if settings.order_enabled {
        tools.extend([
            json!({
                "name": "order_search",
                "description": "分页查询订单（订单号/收货人关键词、订单状态），只读。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "keyword": { "type": "string" },
                        "orderStatus": { "type": "integer" },
                        "page": { "type": "integer" },
                        "size": { "type": "integer" }
                    }
                }
            }),
            json!({
                "name": "order_get",
                "description": "按订单 ID 读取订单与商品明细，只读。",
                "inputSchema": {
                    "type": "object",
                    "properties": { "orderId": { "type": "integer" } },
                    "required": ["orderId"]
                }
            }),
            json!({
                "name": "order_update_remark",
                "description": "修改订单后台备注（admin_note），不影响买家备注。",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "orderId": { "type": "integer" },
                        "remark": { "type": "string" }
                    },
                    "required": ["orderId", "remark"]
                }
            }),
        ]);
    }
    if settings.member_enabled {
        tools.extend([
            json!({
                "name": "member_search",
                "description": "分页查询会员（用户名/昵称/手机号关键词），只读。",
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
                "name": "member_get",
                "description": "按会员 ID 读取会员信息，只读。",
                "inputSchema": {
                    "type": "object",
                    "properties": { "userId": { "type": "integer" } },
                    "required": ["userId"]
                }
            }),
        ]);
    }
    if settings.content_enabled {
        tools.extend([
            json!({
                "name": "content_article_list",
                "description": "分页查询文章/公告（标题关键词），只读。",
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
                "name": "content_article_get",
                "description": "按文章 ID 读取详情，只读。",
                "inputSchema": {
                    "type": "object",
                    "properties": { "articleId": { "type": "integer" } },
                    "required": ["articleId"]
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

fn parse_i32_list(args: &Value, key: &str) -> Vec<i32> {
    args.get(key)
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_i64)
                .filter_map(|value| i32::try_from(value).ok())
                .filter(|value| *value > 0)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_product_ids(args: &Value) -> Result<Vec<i32>, String> {
    let list = args
        .get("productIds")
        .and_then(Value::as_array)
        .ok_or_else(|| "缺少参数 productIds（数组）".to_string())?;
    let mut seen = std::collections::HashSet::new();
    let mut ids = Vec::new();
    for value in list {
        if let Some(id) = value.as_i64().and_then(|n| i32::try_from(n).ok()) {
            if id > 0 && seen.insert(id) {
                ids.push(id);
            }
        }
    }
    if ids.is_empty() {
        return Err("productIds 不能为空".into());
    }
    Ok(ids)
}

const MAX_MEDIA_BYTES: usize = 20 * 1024 * 1024;

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || v4.is_documentation()
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_private_ip(IpAddr::V4(v4));
            }
            v6.is_loopback()
                || v6.is_unspecified()
                || (v6.segments()[0] & 0xfe00) == 0xfc00
                || (v6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

/// 只允许公网 http/https：解析域名并拒绝内网/回环/链路本地地址（SSRF 防护）。
async fn ensure_public_url(url: &reqwest::Url) -> Result<(), String> {
    if !matches!(url.scheme(), "http" | "https") {
        return Err("仅支持 http/https 图片地址".into());
    }
    let host = url.host_str().ok_or_else(|| "URL 缺少主机名".to_string())?;
    if host.eq_ignore_ascii_case("localhost") {
        return Err("不允许访问本机地址".into());
    }
    let port = url.port_or_known_default().unwrap_or(443);
    let mut resolved = false;
    for addr in tokio::net::lookup_host((host, port))
        .await
        .map_err(|e| format!("解析域名失败: {e}"))?
    {
        resolved = true;
        if is_private_ip(addr.ip()) {
            return Err("不允许访问内网地址".into());
        }
    }
    if !resolved {
        return Err("域名无可用解析结果".into());
    }
    Ok(())
}

async fn download_image(url: &str) -> Result<(Vec<u8>, Option<String>), String> {
    let mut current = reqwest::Url::parse(url).map_err(|e| format!("URL 不合法: {e}"))?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;
    for _ in 0..4 {
        ensure_public_url(&current).await?;
        let resp = client
            .get(current.clone())
            .send()
            .await
            .map_err(|e| format!("下载失败: {e}"))?;
        if resp.status().is_redirection() {
            let location = resp
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| "重定向缺少 Location".to_string())?;
            current = current
                .join(location)
                .map_err(|e| format!("重定向地址不合法: {e}"))?;
            continue;
        }
        if !resp.status().is_success() {
            return Err(format!("下载失败: HTTP {}", resp.status()));
        }
        let mime = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.split(';').next().unwrap_or(value).trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty());
        let bytes = resp.bytes().await.map_err(|e| format!("读取图片失败: {e}"))?;
        if bytes.is_empty() {
            return Err("图片内容为空".into());
        }
        if bytes.len() > MAX_MEDIA_BYTES {
            return Err("图片超过 20MB 限制".into());
        }
        return Ok((bytes.to_vec(), mime));
    }
    Err("重定向次数过多".into())
}

fn decode_base64_image(raw: &str) -> Result<(Vec<u8>, Option<String>), String> {
    let trimmed = raw.trim();
    let (mime, payload) = if let Some(rest) = trimmed.strip_prefix("data:") {
        let (meta, data) = rest
            .split_once(',')
            .ok_or_else(|| "base64 data URL 格式不正确".to_string())?;
        (
            meta.split(';')
                .next()
                .map(|value| value.trim().to_ascii_lowercase())
                .filter(|value| value.starts_with("image/")),
            data,
        )
    } else {
        (None, trimmed)
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload.trim())
        .map_err(|_| "base64 解码失败".to_string())?;
    if bytes.is_empty() {
        return Err("图片内容为空".into());
    }
    if bytes.len() > MAX_MEDIA_BYTES {
        return Err("图片超过 20MB 限制".into());
    }
    Ok((bytes, mime))
}

fn sanitize_file_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name).trim();
    let cleaned: String = base
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "ai-upload.jpg".to_string()
    } else {
        cleaned
    }
}

fn build_media_name(explicit: Option<&str>, url: Option<&str>, mime: Option<&str>) -> String {
    if let Some(name) = explicit {
        return sanitize_file_name(name);
    }
    if let Some(link) = url {
        if let Ok(parsed) = reqwest::Url::parse(link) {
            if let Some(last) = parsed.path_segments().and_then(|mut segments| segments.next_back()) {
                if !last.trim().is_empty() {
                    return sanitize_file_name(last);
                }
            }
        }
    }
    let ext = mime
        .and_then(|value| value.split('/').nth(1))
        .unwrap_or("jpg");
    let ext = if ext == "jpeg" { "jpg" } else { ext };
    format!("ai-upload.{ext}")
}

fn guess_image_mime(file_name: &str) -> Option<String> {
    let ext = file_name.rsplit('.').next()?.to_ascii_lowercase();
    let mime = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "tif" | "tiff" => "image/tiff",
        _ => return None,
    };
    Some(mime.to_string())
}

/// 批量操作可执行性判定（对齐旧版 batchBlockReason 的业务规则）。
const BATCH_MAX_IDS: usize = 50;
const BATCH_SORT_LIMIT: usize = 20;
const MAX_IMPORT_PRODUCTS: usize = 50;
const CHECK_STATUS_APPROVED: i32 = 0;

/// `(product_id, product_name, product_sn, product_stock, product_price,
///   product_status, check_status, is_delete, virtual_sales)`
type BatchProductRow = (i32, String, String, i32, Decimal, i32, i32, i32, i32);

fn normalize_batch_operation(operation: Option<&str>) -> Result<String, String> {
    let raw = operation.ok_or_else(|| "缺少批量操作类型".to_string())?;
    let op = raw.trim().to_lowercase();
    if ["up", "down", "recycle", "restore", "del"].contains(&op.as_str()) {
        Ok(op)
    } else {
        Err("仅支持 up、down、recycle、restore、del".into())
    }
}

fn batch_block_reason(op: &str, check_status: i32, is_delete: bool) -> Option<&'static str> {
    if op == "up" && check_status != CHECK_STATUS_APPROVED {
        return Some("审核未通过或尚未审核，审核通过后才能上架");
    }
    if op == "del" && !is_delete {
        return Some("需先移入回收站再删除");
    }
    if op == "restore" && !is_delete {
        return Some("商品不在回收站，无需还原");
    }
    if op == "recycle" && is_delete {
        return Some("商品已经在回收站");
    }
    None
}

fn status_label(code: i32) -> &'static str {
    if code == 1 { "出售中" } else { "已下架" }
}

fn audit_label(code: i32) -> &'static str {
    match code {
        0 => "审核通过",
        1 => "待审核",
        2 => "审核未通过",
        _ => "未知",
    }
}

async fn find_batch_rows(
    pool: &sqlx::PgPool,
    ids: &[i32],
) -> Result<Vec<BatchProductRow>, String> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut qb = sqlx::QueryBuilder::new(
        "SELECT product_id, product_name, product_sn, product_stock, product_price, \
         product_status, check_status, is_delete, virtual_sales FROM product WHERE product_id IN (",
    );
    let mut separated = qb.separated(", ");
    for id in ids {
        separated.push_bind(*id);
    }
    separated.push_unseparated(")");
    qb.build_query_as()
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())
}

async fn batch_set_status(pool: &sqlx::PgPool, ids: &[i32], on_sale: bool) -> Result<u64, String> {
    if ids.is_empty() {
        return Ok(0);
    }
    let mut qb = sqlx::QueryBuilder::new("UPDATE product SET product_status = ");
    qb.push_bind(i32::from(on_sale));
    qb.push(", last_update = ");
    qb.push_bind(now_secs());
    qb.push(" WHERE product_id IN (");
    let mut separated = qb.separated(", ");
    for id in ids {
        separated.push_bind(*id);
    }
    separated.push_unseparated(")");
    Ok(qb
        .build()
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?
        .rows_affected())
}

async fn batch_set_deleted(
    pool: &sqlx::PgPool,
    ids: &[i32],
    deleted: bool,
    reset_status: bool,
) -> Result<u64, String> {
    if ids.is_empty() {
        return Ok(0);
    }
    let mut qb = sqlx::QueryBuilder::new("UPDATE product SET is_delete = ");
    qb.push_bind(i32::from(deleted));
    if reset_status {
        qb.push(", product_status = 0");
    }
    qb.push(", last_update = ");
    qb.push_bind(now_secs());
    qb.push(" WHERE product_id IN (");
    let mut separated = qb.separated(", ");
    for id in ids {
        separated.push_bind(*id);
    }
    separated.push_unseparated(")");
    Ok(qb
        .build()
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?
        .rows_affected())
}

async fn delete_products_cascade(pool: &sqlx::PgPool, ids: &[i32]) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    for id in ids {
        for sql in [
            "DELETE FROM product_gallery WHERE product_id = $1",
            "DELETE FROM product_sku WHERE product_id = $1",
            "DELETE FROM product_video WHERE product_id = $1",
            "DELETE FROM product WHERE product_id = $1",
        ] {
            sqlx::query(sql)
                .bind(*id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    tx.commit().await.map_err(|e| e.to_string())
}

fn eval_batch(op: &str, ids: &[i32], rows: &[BatchProductRow]) -> (Vec<i32>, Vec<Value>) {
    let by_id: std::collections::BTreeMap<i32, &BatchProductRow> =
        rows.iter().map(|row| (row.0, row)).collect();
    let mut executable = Vec::new();
    let mut blocked = Vec::new();
    for id in ids {
        match by_id.get(id) {
            None => blocked.push(json!({
                "productId": id,
                "reason": "找不到该商品，可能已被彻底删除",
            })),
            Some(row) => match batch_block_reason(op, row.6, row.7 == 1) {
                Some(reason) => blocked.push(json!({
                    "productId": id,
                    "productName": row.1,
                    "reason": reason,
                })),
                None => executable.push(*id),
            },
        }
    }
    (executable, blocked)
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
        "product_batch_preview" => {
            if !settings.product_enabled {
                return Err("商品域未启用".into());
            }
            let op = normalize_batch_operation(arg_str(args, "operation").as_deref())?;
            let ids = parse_product_ids(args)?;
            if ids.len() > BATCH_MAX_IDS {
                return Err(format!("一次最多处理 {BATCH_MAX_IDS} 个商品"));
            }
            let rows = find_batch_rows(&state.repos.pool, &ids).await?;
            let (ok_ids, blocked) = eval_batch(&op, &ids, &rows);
            let by_id: std::collections::BTreeMap<i32, &BatchProductRow> =
                rows.iter().map(|row| (row.0, row)).collect();
            let affected_total = ok_ids.len();
            let affected: Vec<Value> = ok_ids
                .iter()
                .filter_map(|id| by_id.get(id))
                .take(BATCH_SORT_LIMIT)
                .map(|row| {
                    json!({
                        "productId": row.0,
                        "productName": row.1,
                        "productSn": row.2,
                        "price": row.4.to_string(),
                        "stock": row.3,
                        "status": status_label(row.5),
                        "auditStatus": audit_label(row.6),
                        "recycled": row.7 == 1,
                    })
                })
                .collect();
            Ok(json!({
                "operation": op,
                "affectedCount": affected_total,
                "affected": affected,
                "blocked": blocked,
                "irreversible": op == "del",
            }))
        }
        "product_batch_execute" => {
            if !settings.product_enabled {
                return Err("商品域未启用".into());
            }
            if args.get("confirmation") != Some(&Value::Bool(true)) {
                return Err("写入前必须传 confirmation: true".into());
            }
            let op = normalize_batch_operation(arg_str(args, "operation").as_deref())?;
            let ids = parse_product_ids(args)?;
            if ids.len() > BATCH_MAX_IDS {
                return Err(format!("一次最多处理 {BATCH_MAX_IDS} 个商品"));
            }
            if (op == "recycle" || op == "del") && settings.require_approval_batch && !approved {
                let approval_id = create_approval(state, name, args).await?;
                return Ok(pending(approval_id));
            }
            let rows = find_batch_rows(&state.repos.pool, &ids).await?;
            let (ok_ids, failed) = eval_batch(&op, &ids, &rows);
            if !ok_ids.is_empty() {
                match op.as_str() {
                    "up" => {
                        batch_set_status(&state.repos.pool, &ok_ids, true).await?;
                    }
                    "down" => {
                        batch_set_status(&state.repos.pool, &ok_ids, false).await?;
                    }
                    "recycle" => {
                        batch_set_deleted(&state.repos.pool, &ok_ids, true, false).await?;
                    }
                    "restore" => {
                        batch_set_deleted(&state.repos.pool, &ok_ids, false, true).await?;
                    }
                    "del" => {
                        delete_products_cascade(&state.repos.pool, &ok_ids).await?;
                    }
                    _ => return Err("不支持的批量操作".into()),
                }
            }
            let after = if op == "del" {
                Vec::new()
            } else {
                let refreshed = find_batch_rows(&state.repos.pool, &ok_ids).await?;
                refreshed
                    .iter()
                    .map(|row| {
                        json!({
                            "productId": row.0,
                            "productName": row.1,
                            "stock": row.3,
                            "status": status_label(row.5),
                            "recycled": row.7 == 1,
                        })
                    })
                    .collect()
            };
            Ok(json!({
                "operation": op,
                "successCount": ok_ids.len(),
                "failed": failed,
                "affected": after,
            }))
        }
        "product_import" => {
            if !settings.product_enabled {
                return Err("商品域未启用".into());
            }
            if args.get("confirmation") != Some(&Value::Bool(true)) {
                return Err("导入前必须传 confirmation: true".into());
            }
            let products = args
                .get("products")
                .and_then(Value::as_array)
                .ok_or_else(|| "缺少参数 products（数组）".to_string())?;
            if products.is_empty() {
                return Err("products 不能为空".into());
            }
            if products.len() > MAX_IMPORT_PRODUCTS {
                return Err(format!("一次最多导入 {MAX_IMPORT_PRODUCTS} 个商品"));
            }
            if settings.require_approval_import && !approved {
                let approval_id = create_approval(state, name, args).await?;
                return Ok(pending(approval_id));
            }
            let mut created = Vec::new();
            let mut failed = Vec::new();
            for (index, item) in products.iter().enumerate() {
                let name_value = arg_str(item, "productName");
                let price_value = arg_dec(item, "productPrice");
                if name_value.is_none() || price_value.is_none() {
                    failed.push(json!({
                        "index": index,
                        "error": "缺少 productName 或 productPrice",
                    }));
                    continue;
                }
                let mut images: Vec<String> = item
                    .get("images")
                    .and_then(Value::as_array)
                    .map(|list| {
                        list.iter()
                            .filter_map(Value::as_str)
                            .map(str::trim)
                            .filter(|url| !url.is_empty())
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default();
                if let Some(pic) = arg_str(item, "picUrl") {
                    images.insert(0, pic);
                }
                let mut seen = std::collections::HashSet::new();
                images.retain(|url| seen.insert(url.clone()));
                let img_list = if images.is_empty() {
                    None
                } else {
                    Some(
                        images
                            .iter()
                            .enumerate()
                            .map(|(sort, url)| GalleryPicDTO {
                                pic_url: Some(url.clone()),
                                pic_sort: Some(i32::try_from(sort).unwrap_or(i32::MAX)),
                                ..Default::default()
                            })
                            .collect::<Vec<_>>(),
                    )
                };
                let dto = ProductSaveDTO {
                    product_name: name_value,
                    product_sn: arg_str(item, "productSn"),
                    product_price: price_value,
                    market_price: arg_dec(item, "marketPrice"),
                    product_stock: arg_i32(item, "productStock"),
                    category_id: arg_i32(item, "categoryId"),
                    brand_id: arg_i32(item, "brandId"),
                    product_brief: arg_str(item, "productBrief"),
                    product_status: arg_i32(item, "productStatus"),
                    pic_url: images.first().cloned(),
                    img_list,
                    ..Default::default()
                };
                match state.catalog.product_manage.create(&dto).await {
                    Ok(_) => created.push(json!({
                        "index": index,
                        "productName": dto.product_name,
                    })),
                    Err(err) => failed.push(json!({
                        "index": index,
                        "productName": dto.product_name,
                        "error": err.to_string(),
                    })),
                }
            }
            Ok(json!({
                "total": products.len(),
                "createdCount": created.len(),
                "created": created,
                "failed": failed,
            }))
        }
        "product_import_validate" => {
            if !settings.product_enabled {
                return Err("商品域未启用".into());
            }
            let products = args
                .get("products")
                .and_then(Value::as_array)
                .ok_or_else(|| "缺少参数 products（数组）".to_string())?;
            if products.is_empty() {
                return Err("products 不能为空".into());
            }
            if products.len() > MAX_IMPORT_PRODUCTS {
                return Err(format!("一次最多校验 {MAX_IMPORT_PRODUCTS} 个商品"));
            }
            let mut seen_sn = std::collections::HashSet::new();
            let mut items = Vec::new();
            let mut importable = true;
            for (index, item) in products.iter().enumerate() {
                let name = arg_str(item, "productName");
                let mut errors: Vec<String> = Vec::new();
                if name.is_none() {
                    errors.push("缺少 productName".into());
                }
                if arg_dec(item, "productPrice").is_none() {
                    errors.push("缺少 productPrice".into());
                }
                if let Some(sn) = arg_str(item, "productSn") {
                    if !seen_sn.insert(sn.clone()) {
                        errors.push("本批次货号重复".into());
                    } else {
                        let count: (i64,) = sqlx::query_as(
                            "SELECT COUNT(*) FROM product WHERE product_sn = $1 AND is_delete = 0",
                        )
                        .bind(&sn)
                        .fetch_one(&state.repos.pool)
                        .await
                        .map_err(|e| e.to_string())?;
                        if count.0 > 0 {
                            errors.push("货号已存在".into());
                        }
                    }
                }
                if let Some(category_id) = arg_i32(item, "categoryId") {
                    let exists: Option<(i32,)> =
                        sqlx::query_as("SELECT category_id FROM category WHERE category_id = $1")
                            .bind(category_id)
                            .fetch_optional(&state.repos.pool)
                            .await
                            .map_err(|e| e.to_string())?;
                    if exists.is_none() {
                        errors.push("分类不存在".into());
                    }
                }
                if !errors.is_empty() {
                    importable = false;
                }
                items.push(json!({
                    "index": index,
                    "productName": name,
                    "valid": errors.is_empty(),
                    "errors": errors,
                }));
            }
            Ok(json!({
                "importable": importable,
                "total": products.len(),
                "items": items,
            }))
        }
        "product_copy" => {
            if !settings.product_enabled {
                return Err("商品域未启用".into());
            }
            let id = require_i32(args, "productId")?;
            let pool = &state.repos.pool;
            let row: Option<(String, String, Decimal, Decimal, i32, i32, String, String)> =
                sqlx::query_as(
                    "SELECT product_name, product_sn, product_price, market_price, category_id, \
                     brand_id, COALESCE(product_brief, ''), COALESCE(pic_url, '') \
                     FROM product WHERE product_id = $1 AND is_delete = 0",
                )
                .bind(id)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?;
            let (name, sn, price, market, category_id, brand_id, brief, pic) =
                row.ok_or_else(|| "商品不存在".to_string())?;
            let new_sn = format!("{sn}-copy-{}", now_secs());
            let dto = ProductSaveDTO {
                product_name: Some(format!("{name} 副本")),
                product_sn: Some(new_sn.clone()),
                product_price: Some(price),
                market_price: Some(market),
                category_id: Some(category_id),
                brand_id: Some(brand_id),
                product_brief: if brief.trim().is_empty() { None } else { Some(brief) },
                pic_url: if pic.trim().is_empty() { None } else { Some(pic) },
                product_stock: Some(0),
                product_status: Some(0),
                ..Default::default()
            };
            state
                .catalog
                .product_manage
                .create(&dto)
                .await
                .map_err(|e| e.to_string())?;
            let new_id: Option<(i32,)> = sqlx::query_as(
                "SELECT product_id FROM product WHERE product_sn = $1 LIMIT 1",
            )
            .bind(&new_sn)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;
            Ok(json!({
                "newProductId": new_id.map(|row| row.0),
                "productName": dto.product_name,
                "productSn": new_sn,
                "productStatus": 0,
                "productStock": 0,
            }))
        }
        "product_analysis" => {
            if !settings.product_enabled {
                return Err("商品域未启用".into());
            }
            let operation = arg_str(args, "operation")
                .unwrap_or_else(|| "overview".to_string())
                .to_lowercase();
            let keyword = arg_str(args, "keyword");
            let category = arg_i32(args, "categoryId");
            let limit = args
                .get("limit")
                .and_then(Value::as_i64)
                .unwrap_or(20)
                .clamp(1, 100);
            let pool = &state.repos.pool;
            let filter = "is_delete = 0 \
                AND ($1::text IS NULL OR product_name ILIKE '%' || $1 || '%') \
                AND ($2::int IS NULL OR category_id = $2)";
            let data = match operation.as_str() {
                "overview" => {
                    let row: (i64, Option<Decimal>, Option<i64>, Option<i64>) =
                        sqlx::query_as(&format!(
                            "SELECT COUNT(*), AVG(product_price), SUM(product_stock), \
                             SUM(virtual_sales) FROM product WHERE {filter}"
                        ))
                        .bind(keyword.clone())
                        .bind(category)
                        .fetch_one(pool)
                        .await
                        .map_err(|e| e.to_string())?;
                    json!({
                        "productCount": row.0,
                        "avgPrice": row.1.map(|value| value.to_string()),
                        "totalStock": row.2.unwrap_or(0),
                        "totalSales": row.3.unwrap_or(0),
                    })
                }
                "top_sellers" | "slow_movers" | "inventory_risk" | "recommend" => {
                    let (order, extra) = match operation.as_str() {
                        "top_sellers" => ("virtual_sales DESC", ""),
                        "slow_movers" => ("virtual_sales ASC, product_stock DESC", ""),
                        "inventory_risk" => ("product_stock DESC", "AND product_stock > 0 AND virtual_sales = 0"),
                        _ => ("virtual_sales DESC, product_stock ASC", "AND product_stock <= 20"),
                    };
                    let rows: Vec<(i32, String, Decimal, i32, i32)> = sqlx::query_as(&format!(
                        "SELECT product_id, product_name, product_price, product_stock, \
                         virtual_sales FROM product WHERE {filter} {extra} \
                         ORDER BY {order} LIMIT $3"
                    ))
                    .bind(keyword.clone())
                    .bind(category)
                    .bind(limit)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| e.to_string())?;
                    json!(
                        rows.iter()
                            .map(|row| json!({
                                "productId": row.0,
                                "productName": row.1,
                                "price": row.2.to_string(),
                                "stock": row.3,
                                "sales": row.4,
                            }))
                            .collect::<Vec<_>>()
                    )
                }
                "price_bands" => {
                    let rows: Vec<(String, i64)> = sqlx::query_as(&format!(
                        "SELECT CASE WHEN product_price < 50 THEN '0-50' \
                         WHEN product_price < 100 THEN '50-100' \
                         WHEN product_price < 500 THEN '100-500' \
                         WHEN product_price < 1000 THEN '500-1000' \
                         ELSE '1000+' END AS band, COUNT(*) \
                         FROM product WHERE {filter} GROUP BY band ORDER BY MIN(product_price) \
                         LIMIT $3"
                    ))
                    .bind(keyword.clone())
                    .bind(category)
                    .bind(limit)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| e.to_string())?;
                    json!(
                        rows.iter()
                            .map(|row| json!({ "band": row.0, "count": row.1 }))
                            .collect::<Vec<_>>()
                    )
                }
                "category_performance" => {
                    let rows: Vec<(i32, i64, Option<i64>, Option<Decimal>)> =
                        sqlx::query_as(&format!(
                            "SELECT category_id, COUNT(*), SUM(virtual_sales), \
                             AVG(product_price) FROM product WHERE {filter} \
                             GROUP BY category_id ORDER BY SUM(virtual_sales) DESC NULLS LAST \
                             LIMIT $3"
                        ))
                        .bind(keyword.clone())
                        .bind(category)
                        .bind(limit)
                        .fetch_all(pool)
                        .await
                        .map_err(|e| e.to_string())?;
                    json!(
                        rows.iter()
                            .map(|row| json!({
                                "categoryId": row.0,
                                "productCount": row.1,
                                "totalSales": row.2.unwrap_or(0),
                                "avgPrice": row.3.map(|value| value.to_string()),
                            }))
                            .collect::<Vec<_>>()
                    )
                }
                other => return Err(format!("不支持的分析操作: {other}")),
            };
            Ok(json!({ "operation": operation, "data": data }))
        }
        "product_context" => {
            if !settings.product_enabled {
                return Err("商品域未启用".into());
            }
            let keyword = arg_str(args, "keyword");
            let category = arg_i32(args, "categoryId");
            let size = args
                .get("sampleSize")
                .and_then(Value::as_i64)
                .unwrap_or(100)
                .clamp(1, 500);
            let filter = "is_delete = 0 \
                AND ($1::text IS NULL OR product_name ILIKE '%' || $1 || '%') \
                AND ($2::int IS NULL OR category_id = $2)";
            let rows: Vec<(i32, String, String, Decimal, i32, i32)> = sqlx::query_as(&format!(
                "SELECT product_id, product_name, product_sn, product_price, product_stock, \
                 virtual_sales FROM product WHERE {filter} ORDER BY product_id DESC LIMIT $3"
            ))
            .bind(keyword)
            .bind(category)
            .bind(size)
            .fetch_all(&state.repos.pool)
            .await
            .map_err(|e| e.to_string())?;
            let sample_size = rows.len();
            if sample_size == 0 {
                return Ok(json!({
                    "hasData": false,
                    "sampleSize": 0,
                    "metrics": null,
                    "samples": [],
                }));
            }
            let mut prices: Vec<Decimal> = rows.iter().map(|row| row.3).collect();
            prices.sort();
            let total_price: Decimal = prices.iter().copied().sum();
            let count_dec = Decimal::from(i64::try_from(sample_size).unwrap_or(i64::MAX));
            let avg_price = total_price / count_dec;
            let median_price = prices[sample_size / 2];
            let sales_avg = rows.iter().map(|row| i64::from(row.5)).sum::<i64>() as f64
                / sample_size as f64;
            let stock_avg = rows.iter().map(|row| i64::from(row.4)).sum::<i64>() as f64
                / sample_size as f64;
            let samples: Vec<Value> = rows
                .iter()
                .take(20)
                .map(|row| json!({
                    "productId": row.0,
                    "productName": row.1,
                    "productSn": row.2,
                    "price": row.3.to_string(),
                    "stock": row.4,
                    "sales": row.5,
                }))
                .collect();
            Ok(json!({
                "hasData": true,
                "sampleSize": sample_size,
                "metrics": {
                    "priceMin": prices.first().map(|value| value.to_string()),
                    "priceAvg": avg_price.to_string(),
                    "priceMedian": median_price.to_string(),
                    "priceMax": prices.last().map(|value| value.to_string()),
                    "stockAvg": stock_avg,
                    "salesAvg": sales_avg,
                },
                "samples": samples,
            }))
        }
        "product_category_create" => {
            if !settings.product_enabled {
                return Err("商品域未启用".into());
            }
            let paths = args
                .get("paths")
                .and_then(Value::as_array)
                .ok_or_else(|| "缺少参数 paths（数组）".to_string())?;
            if paths.is_empty() {
                return Err("paths 不能为空".into());
            }
            if paths.len() > 50 {
                return Err("一次最多创建 50 条分类路径".into());
            }
            let rows: Vec<(i32, String, i32)> =
                sqlx::query_as("SELECT category_id, category_name, parent_id FROM category")
                    .fetch_all(&state.repos.pool)
                    .await
                    .map_err(|e| e.to_string())?;
            let mut index: std::collections::HashMap<(i32, String), i32> = rows
                .iter()
                .map(|row| ((row.2, row.1.clone()), row.0))
                .collect();
            let mut created_count = 0;
            let mut items = Vec::new();
            for value in paths {
                let path = value
                    .as_str()
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                    .ok_or_else(|| "路径不能为空".to_string())?;
                let mut parent = 0;
                for segment in path.split('/').map(str::trim).filter(|part| !part.is_empty()) {
                    let key = (parent, segment.to_string());
                    parent = if let Some(id) = index.get(&key) {
                        *id
                    } else {
                        let dto = CategoryDTO {
                            category_name: Some(segment.to_string()),
                            parent_id: Some(parent),
                            is_show: Some(1),
                            ..Default::default()
                        };
                        let id = state
                            .catalog
                            .category
                            .create(&dto)
                            .await
                            .map_err(|e| e.to_string())?;
                        index.insert(key, id);
                        created_count += 1;
                        id
                    };
                }
                items.push(json!({ "path": path, "categoryId": parent }));
            }
            Ok(json!({ "createdCount": created_count, "items": items }))
        }
        "media_manage" => {
            if !settings.product_enabled && !settings.decorate_enabled {
                return Err("商品域与装修域均未启用，无法管理素材".into());
            }
            let operation = arg_str(args, "operation").ok_or_else(|| "缺少 operation".to_string())?;
            match operation.as_str() {
                "group_create" => {
                    let name =
                        arg_str(args, "groupName").ok_or_else(|| "缺少 groupName".to_string())?;
                    let dto = GalleryDTO {
                        gallery_name: Some(name.clone()),
                        shop_id: Some(0),
                        ..Default::default()
                    };
                    let group_id = state
                        .content
                        .gallery
                        .create(GalleryScope::platform(), &dto)
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(json!({ "groupId": group_id, "groupName": name }))
                }
                "group_update" => {
                    let group_id = require_i32(args, "groupId")?;
                    let name =
                        arg_str(args, "groupName").ok_or_else(|| "缺少 groupName".to_string())?;
                    let dto = GalleryUpdateDTO {
                        id: Some(group_id),
                        gallery_name: Some(name.clone()),
                        shop_id: Some(0),
                        ..Default::default()
                    };
                    state
                        .content
                        .gallery
                        .update(GalleryScope::platform(), &dto)
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(json!({ "groupId": group_id, "groupName": name }))
                }
                "group_delete" => {
                    let group_id = require_i32(args, "groupId")?;
                    let mode = arg_str(args, "deleteMode")
                        .unwrap_or_else(|| "only_group".to_string());
                    let dto = GalleryDelDTO {
                        id: Some(group_id),
                        delete_mode: Some(mode),
                    };
                    state
                        .content
                        .gallery
                        .delete(&dto)
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(json!({ "groupId": group_id, "deleted": true }))
                }
                "update" => {
                    let ids = parse_i32_list(args, "picIds");
                    if ids.is_empty() {
                        return Err("picIds 不能为空".into());
                    }
                    if let Some(gallery_id) = arg_i32(args, "galleryId") {
                        state
                            .content
                            .gallery_pic
                            .transfer(&GalleryPicTransferDTO {
                                ids: Some(ids.clone()),
                                target_gallery_id: Some(gallery_id),
                                r#type: Some("image".to_string()),
                            })
                            .await
                            .map_err(|e| e.to_string())?;
                    }
                    if let Some(name) = arg_str(args, "picName") {
                        for pic_id in &ids {
                            state
                                .content
                                .gallery_pic
                                .update_field(&UpdateFieldDTO {
                                    id: *pic_id,
                                    field: "pic_name".to_string(),
                                    val: name.clone(),
                                })
                                .await
                                .map_err(|e| e.to_string())?;
                        }
                    }
                    Ok(json!({ "updatedCount": ids.len() }))
                }
                "delete" => {
                    let ids = parse_i32_list(args, "picIds");
                    if ids.is_empty() {
                        return Err("picIds 不能为空".into());
                    }
                    state
                        .content
                        .gallery_pic
                        .batch_del(&GalleryPicBatchDTO {
                            ids: Some(ids.clone()),
                            r#type: Some("image".to_string()),
                        })
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(json!({ "deletedCount": ids.len() }))
                }
                other => Err(format!("不支持的素材操作: {other}")),
            }
        }
        "media_upload" => {
            if !settings.product_enabled && !settings.decorate_enabled {
                return Err("商品域与装修域均未启用，无法上传素材".into());
            }
            let url = arg_str(args, "url");
            let base64_raw = arg_str(args, "base64");
            let (bytes, mime) = match (url.as_deref(), base64_raw.as_deref()) {
                (Some(link), None) => download_image(link).await?,
                (None, Some(raw)) => decode_base64_image(raw)?,
                (Some(_), Some(_)) => return Err("url 与 base64 只能二选一".into()),
                (None, None) => return Err("缺少 url 或 base64".into()),
            };
            let file_name = build_media_name(
                arg_str(args, "fileName").as_deref(),
                url.as_deref(),
                mime.as_deref(),
            );
            let content_type = mime.or_else(|| guess_image_mime(&file_name));
            let gallery_id = arg_i32(args, "galleryId").filter(|id| *id > 0);
            let vo = state
                .content
                .gallery_pic
                .upload_img(
                    gallery_id,
                    content_type.as_deref(),
                    bytes.len() as u64,
                    Some(&file_name),
                    &bytes,
                    0,
                    0,
                )
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_value(vo).map_err(|e| e.to_string())
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
        "analytics_sales_summary" => {
            if !settings.analytics_enabled {
                return Err("报表域未启用".into());
            }
            let days = args.get("days").and_then(Value::as_i64).unwrap_or(7).clamp(1, 365);
            let since = now_secs() - (days as i32) * 86400;
            let row: (Value,) = sqlx::query_as(
                // pay_time=0 的历史/导入订单按 add_time 计入，避免整份报表静默为 0
                "SELECT to_jsonb(t) FROM (SELECT COUNT(*)::bigint AS orders,
                     COALESCE(SUM(total_amount), 0)::text AS amount
                     FROM \"order\"
                     WHERE pay_status = 1
                       AND COALESCE(NULLIF(pay_time, 0), add_time) >= $1) t",
            )
            .bind(since)
            .fetch_one(&state.repos.pool)
            .await
            .map_err(|e| e.to_string())?;
            Ok(json!({ "days": days, "summary": row.0 }))
        }
        "analytics_top_products" => {
            if !settings.analytics_enabled {
                return Err("报表域未启用".into());
            }
            let days = args.get("days").and_then(Value::as_i64).unwrap_or(30).clamp(1, 365);
            let limit = args.get("limit").and_then(Value::as_i64).unwrap_or(10).clamp(1, 50);
            let since = now_secs() - (days as i32) * 86400;
            let rows: Vec<(Value,)> = sqlx::query_as(
                "SELECT to_jsonb(t) FROM (
                     SELECT p.product_id, p.product_name, p.product_sn,
                            SUM(oi.quantity)::bigint AS qty,
                            COUNT(DISTINCT o.order_id)::bigint AS orders,
                            SUM(oi.price * oi.quantity)::text AS amount
                     FROM order_item oi
                     JOIN \"order\" o ON o.order_id = oi.order_id
                     JOIN product p ON p.product_id = oi.product_id
                     WHERE o.pay_status = 1
                       AND COALESCE(NULLIF(o.pay_time, 0), o.add_time) >= $1
                     GROUP BY p.product_id, p.product_name, p.product_sn
                     ORDER BY SUM(oi.quantity) DESC
                     LIMIT $2) t",
            )
            .bind(since)
            .bind(limit)
            .fetch_all(&state.repos.pool)
            .await
            .map_err(|e| e.to_string())?;
            Ok(json!({ "days": days, "records": rows.into_iter().map(|r| r.0).collect::<Vec<_>>() }))
        }
        "analytics_low_stock" => {
            if !settings.analytics_enabled {
                return Err("报表域未启用".into());
            }
            let threshold = args.get("threshold").and_then(Value::as_i64).unwrap_or(10).clamp(0, 100000);
            let limit = args.get("limit").and_then(Value::as_i64).unwrap_or(20).clamp(1, 100);
            let rows: Vec<(Value,)> = sqlx::query_as(
                "SELECT to_jsonb(p) FROM (
                     SELECT product_id, product_name, product_sn, product_stock, product_price, product_status
                     FROM product
                     WHERE is_delete = 0 AND product_status = 1 AND product_stock <= $1
                     ORDER BY product_stock ASC LIMIT $2) p",
            )
            .bind(threshold)
            .bind(limit)
            .fetch_all(&state.repos.pool)
            .await
            .map_err(|e| e.to_string())?;
            Ok(json!({ "threshold": threshold, "records": rows.into_iter().map(|r| r.0).collect::<Vec<_>>() }))
        }
        "order_search" => {
            if !settings.order_enabled {
                return Err("订单域未启用".into());
            }
            let kw = arg_str(args, "keyword").map(|s| format!("%{s}%"));
            let status = arg_i32(args, "orderStatus");
            let page = args.get("page").and_then(Value::as_i64).unwrap_or(1).max(1);
            let size = args.get("size").and_then(Value::as_i64).unwrap_or(20).clamp(1, 100);
            let offset = (page - 1) * size;
            let rows: Vec<(Value,)> = sqlx::query_as(
                "SELECT to_jsonb(o) FROM \"order\" o
                 WHERE ($1::text IS NULL OR o.order_sn ILIKE $1 OR o.consignee ILIKE $1)
                   AND ($2::int IS NULL OR o.order_status = $2)
                 ORDER BY o.order_id DESC LIMIT $3 OFFSET $4",
            )
            .bind(kw.clone())
            .bind(status)
            .bind(size)
            .bind(offset)
            .fetch_all(&state.repos.pool)
            .await
            .map_err(|e| e.to_string())?;
            let total: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM \"order\" o
                 WHERE ($1::text IS NULL OR o.order_sn ILIKE $1 OR o.consignee ILIKE $1)
                   AND ($2::int IS NULL OR o.order_status = $2)",
            )
            .bind(kw)
            .bind(status)
            .fetch_one(&state.repos.pool)
            .await
            .map_err(|e| e.to_string())?;
            Ok(json!({ "records": rows.into_iter().map(|r| r.0).collect::<Vec<_>>(),
                       "total": total.0, "page": page, "size": size }))
        }
        "order_get" => {
            if !settings.order_enabled {
                return Err("订单域未启用".into());
            }
            let id = require_i32(args, "orderId")?;
            let head: Option<(Value,)> = sqlx::query_as(
                "SELECT to_jsonb(o) FROM \"order\" o WHERE o.order_id = $1",
            )
            .bind(id)
            .fetch_optional(&state.repos.pool)
            .await
            .map_err(|e| e.to_string())?;
            let Some((head,)) = head else {
                return Err("订单不存在".into());
            };
            let items: Vec<(Value,)> = sqlx::query_as(
                "SELECT to_jsonb(i) FROM order_item i WHERE i.order_id = $1 ORDER BY i.item_id",
            )
            .bind(id)
            .fetch_all(&state.repos.pool)
            .await
            .map_err(|e| e.to_string())?;
            Ok(json!({ "order": head, "items": items.into_iter().map(|r| r.0).collect::<Vec<_>>() }))
        }
        "order_update_remark" => {
            if !settings.order_enabled {
                return Err("订单域未启用".into());
            }
            let id = require_i32(args, "orderId")?;
            let remark = args
                .get("remark")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let r = sqlx::query("UPDATE \"order\" SET admin_note = $1 WHERE order_id = $2")
                .bind(&remark)
                .bind(id)
                .execute(&state.repos.pool)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({ "updatedRows": r.rows_affected() }))
        }
        "member_search" => {
            if !settings.member_enabled {
                return Err("会员域未启用".into());
            }
            let kw = arg_str(args, "keyword").map(|s| format!("%{s}%"));
            let page = args.get("page").and_then(Value::as_i64).unwrap_or(1).max(1);
            let size = args.get("size").and_then(Value::as_i64).unwrap_or(20).clamp(1, 100);
            let offset = (page - 1) * size;
            let rows: Vec<(Value,)> = sqlx::query_as(
                "SELECT to_jsonb(u) - 'password' FROM \"user\" u
                 WHERE ($1::text IS NULL OR u.username ILIKE $1
                        OR u.nickname ILIKE $1 OR u.mobile ILIKE $1)
                 ORDER BY u.user_id DESC LIMIT $2 OFFSET $3",
            )
            .bind(kw.clone())
            .bind(size)
            .bind(offset)
            .fetch_all(&state.repos.pool)
            .await
            .map_err(|e| e.to_string())?;
            let total: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM \"user\" u
                 WHERE ($1::text IS NULL OR u.username ILIKE $1
                        OR u.nickname ILIKE $1 OR u.mobile ILIKE $1)",
            )
            .bind(kw)
            .fetch_one(&state.repos.pool)
            .await
            .map_err(|e| e.to_string())?;
            Ok(json!({ "records": rows.into_iter().map(|r| r.0).collect::<Vec<_>>(),
                       "total": total.0, "page": page, "size": size }))
        }
        "member_get" => {
            if !settings.member_enabled {
                return Err("会员域未启用".into());
            }
            let id = require_i32(args, "userId")?;
            let row: Option<(Value,)> = sqlx::query_as(
                "SELECT to_jsonb(u) - 'password' FROM \"user\" u WHERE u.user_id = $1",
            )
            .bind(id)
            .fetch_optional(&state.repos.pool)
            .await
            .map_err(|e| e.to_string())?;
            row.map(|r| r.0).ok_or_else(|| "会员不存在".to_string())
        }
        "content_article_list" => {
            if !settings.content_enabled {
                return Err("内容域未启用".into());
            }
            let kw = arg_str(args, "keyword").map(|s| format!("%{s}%"));
            let page = args.get("page").and_then(Value::as_i64).unwrap_or(1).max(1);
            let size = args.get("size").and_then(Value::as_i64).unwrap_or(20).clamp(1, 100);
            let offset = (page - 1) * size;
            let rows: Vec<(Value,)> = sqlx::query_as(
                "SELECT to_jsonb(a) FROM article a
                 WHERE ($1::text IS NULL OR a.article_title ILIKE $1)
                 ORDER BY a.article_id DESC LIMIT $2 OFFSET $3",
            )
            .bind(kw.clone())
            .bind(size)
            .bind(offset)
            .fetch_all(&state.repos.pool)
            .await
            .map_err(|e| e.to_string())?;
            let total: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM article a WHERE ($1::text IS NULL OR a.article_title ILIKE $1)",
            )
            .bind(kw)
            .fetch_one(&state.repos.pool)
            .await
            .map_err(|e| e.to_string())?;
            Ok(json!({ "records": rows.into_iter().map(|r| r.0).collect::<Vec<_>>(),
                       "total": total.0, "page": page, "size": size }))
        }
        "content_article_get" => {
            if !settings.content_enabled {
                return Err("内容域未启用".into());
            }
            let id = require_i32(args, "articleId")?;
            let row: Option<(Value,)> = sqlx::query_as(
                "SELECT to_jsonb(a) FROM article a WHERE a.article_id = $1",
            )
            .bind(id)
            .fetch_optional(&state.repos.pool)
            .await
            .map_err(|e| e.to_string())?;
            row.map(|r| r.0).ok_or_else(|| "文章不存在".to_string())
        }
        other => Err(format!("未知工具: {other}")),
    }
}
