//! 用量记录（DESIGN §1.4）：只记不扣（P1）。商家强制本店；平台可按店铺筛选或看全平台。

use axum::{
    extract::{Extension, Query, State},
};
use serde::Deserialize;
use serde_json::{json, Value};
use nextrade_security::web::{ok, ApiResult, AuthUser};

use super::{err_validation, AiTenant};
use crate::AdminState;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UsageQuery {
    pub page: Option<i64>,
    pub size: Option<i64>,
    pub model_code: Option<String>,
    pub status: Option<i32>,
    /// 平台可指定商家；商家传任何值都会被强制改回本店（接口层隔离，DESIGN §3.5 用例 8）。
    pub shop_id: Option<i32>,
}

pub async fn list_usage(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<UsageQuery>,
) -> ApiResult<Value> {
    let tenant = AiTenant::from_auth(&auth)?;
    // 商家：忽略传入的 shopId，强制本店。平台：不传 = 全平台。
    let shop_filter: Option<i32> = if tenant.is_merchant() {
        Some(tenant.shop_id)
    } else {
        q.shop_id
    };
    let page = q.page.unwrap_or(1).max(1);
    let size = q.size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * size;
    let pool = &state.repos.pool;
    let rows: Vec<(i64, i32, i32, i32, String, i32, i32, i32, i64, i32, String, i64)> =
        sqlx::query_as(
            "SELECT usage_id, shop_id, admin_id, channel_id, model_code, kind,
                    prompt_tokens, completion_tokens, credits_charged, status, error, create_time
             FROM ai_usage
             WHERE ($1::int IS NULL OR shop_id = $1)
               AND ($2::text IS NULL OR model_code = $2)
               AND ($3::int IS NULL OR status = $3)
             ORDER BY usage_id DESC LIMIT $4 OFFSET $5",
        )
        .bind(shop_filter)
        .bind(q.model_code.clone())
        .bind(q.status)
        .bind(size)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| err_validation(e.to_string()))?;
    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM ai_usage
         WHERE ($1::int IS NULL OR shop_id = $1)
           AND ($2::text IS NULL OR model_code = $2)
           AND ($3::int IS NULL OR status = $3)",
    )
    .bind(shop_filter)
    .bind(q.model_code.clone())
    .bind(q.status)
    .fetch_one(pool)
    .await
    .map_err(|e| err_validation(e.to_string()))?;
    let summary: (i64, i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(prompt_tokens),0)::bigint, COALESCE(SUM(completion_tokens),0)::bigint,
                COALESCE(SUM(credits_charged),0)::bigint
         FROM ai_usage
         WHERE ($1::int IS NULL OR shop_id = $1)
           AND ($2::text IS NULL OR model_code = $2)
           AND ($3::int IS NULL OR status = $3)",
    )
    .bind(shop_filter)
    .bind(q.model_code.clone())
    .bind(q.status)
    .fetch_one(pool)
    .await
    .map_err(|e| err_validation(e.to_string()))?;
    let records: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "usageId": r.0,
                "shopId": r.1,
                "adminId": r.2,
                "channelId": r.3,
                "modelCode": r.4,
                "kind": r.5,
                "promptTokens": r.6,
                "completionTokens": r.7,
                "creditsCharged": r.8,
                "selfKey": true,
                "status": r.9,
                "error": r.10,
                "createTime": r.11,
            })
        })
        .collect();
    ok(json!({
        "records": records,
        "total": total.0,
        "page": page,
        "size": size,
        "summary": {
            "promptTokens": summary.0,
            "completionTokens": summary.1,
            "creditsCharged": summary.2,
        }
    }))
}

/// 供 chat 引擎记录用量（失败也记，status 见表注释）。
#[allow(clippy::too_many_arguments)]
pub async fn record_usage(
    pool: &sqlx::PgPool,
    tenant: &AiTenant,
    conv_id: i32,
    channel_id: i32,
    model_code: &str,
    prompt_tokens: i32,
    completion_tokens: i32,
    status: i32,
    error: &str,
) {
    let _ = sqlx::query(
        "INSERT INTO ai_usage (shop_id, admin_id, conv_id, channel_id, model_code, kind,
            prompt_tokens, completion_tokens, image_count, credits_charged, self_key, status, error, create_time)
         VALUES ($1, $2, $3, $4, $5, 1, $6, $7, 0, 0, 1, $8, $9, $10)",
    )
    .bind(tenant.shop_id)
    .bind(tenant.admin_id)
    .bind(conv_id)
    .bind(channel_id)
    .bind(model_code)
    .bind(prompt_tokens)
    .bind(completion_tokens)
    .bind(status)
    .bind(error.chars().take(500).collect::<String>())
    .bind(super::now_secs())
    .execute(pool)
    .await;
}
