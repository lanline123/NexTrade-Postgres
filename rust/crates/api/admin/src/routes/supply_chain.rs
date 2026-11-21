//! 供应链（v5.8.61 `/adminapi/supplyChain/*`）基础路由。
//!
//! 覆盖供应商、供应商分类、供应商商品、采购单/入库/对账/退货/设置。
//! 列表/详情/状态/删除使用通用表读取；复杂保存动作先返回明确未迁移错误。

use axum::{
    extract::{Extension, Multipart, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use sqlx::postgres::PgRow;
use sqlx::{Column,Postgres, QueryBuilder, Row, TypeInfo};
use std::str::FromStr;
use nextrade_core::models::catalog::product_stock::{
    record_type as stock_record_type, StockInDTO, StockRecordDetailItem,
};
use nextrade_core::Error;
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_services::catalog::product_stock::StockOperator;
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};

use crate::AdminState;

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        // supplier
        .route("/supplyChain/supplier/list", get(supplier_list))
        .route("/supplyChain/supplier/detail", get(supplier_detail))
        .route("/supplyChain/supplier/create", post(supplier_create))
        .route("/supplyChain/supplier/update", post(supplier_update))
        .route("/supplyChain/supplier/status", post(supplier_status))
        .route("/supplyChain/supplier/delete", post(supplier_delete))
        .route(
            "/supplyChain/supplier/qualification/upload",
            post(qualification_upload),
        )
        // category
        .route("/supplyChain/supplierCategory/list", get(category_list))
        .route("/supplyChain/supplierCategory/save", post(category_save))
        .route(
            "/supplyChain/supplierCategory/status",
            post(category_status),
        )
        .route(
            "/supplyChain/supplierCategory/delete",
            post(category_delete),
        )
        // supplier product
        .route("/supplyChain/supplierProduct/list", get(product_list))
        .route("/supplyChain/supplierProduct/summary", get(product_list))
        .route("/supplyChain/supplierProduct/selection", get(product_list))
        .route("/supplyChain/supplierProduct/detail", get(product_detail))
        .route("/supplyChain/supplierProduct/status", post(product_status))
        .route("/supplyChain/supplierProduct/delete", post(product_delete))
        .route(
            "/supplyChain/supplierProduct/batchDelete",
            post(product_batch_delete),
        )
        .route("/supplyChain/supplierProduct/save", post(product_save))
        .route(
            "/supplyChain/supplierProduct/batchSave",
            post(product_batch_save),
        )
        // purchase order
        .route("/supplyChain/purchaseOrder/list", get(order_list))
        .route("/supplyChain/purchaseOrder/detail", get(order_detail))
        .route("/supplyChain/purchaseOrder/create", post(order_create))
        .route("/supplyChain/purchaseOrder/update", post(order_update))
        .route("/supplyChain/purchaseOrder/submit", post(order_submit))
        .route("/supplyChain/purchaseOrder/delete", post(order_delete))
        .route("/supplyChain/purchaseOrder/finish", post(order_finish))
        .route("/supplyChain/purchaseOrder/ship", post(order_ship))
        // receipt
        .route("/supplyChain/purchaseReceipt/list", get(receipt_list))
        .route("/supplyChain/purchaseReceipt/detail", get(receipt_detail))
        .route("/supplyChain/purchaseReceipt/save", post(receipt_save))
        // reconciliation
        .route(
            "/supplyChain/purchaseReconciliation/list",
            get(reconciliation_list),
        )
        .route(
            "/supplyChain/purchaseReconciliation/detail",
            get(reconciliation_detail),
        )
        .route(
            "/supplyChain/purchaseReconciliation/candidates",
            get(reconciliation_candidates),
        )
        .route(
            "/supplyChain/purchaseReconciliation/export",
            get(reconciliation_export),
        )
        .route(
            "/supplyChain/purchaseReconciliation/create",
            post(reconciliation_create),
        )
        .route(
            "/supplyChain/purchaseReconciliation/update",
            post(reconciliation_update),
        )
        .route(
            "/supplyChain/purchaseReconciliation/approve",
            post(reconciliation_approve),
        )
        .route(
            "/supplyChain/purchaseReconciliation/reject",
            post(reconciliation_reject),
        )
        .route(
            "/supplyChain/purchaseReconciliation/resubmit",
            post(reconciliation_resubmit),
        )
        .route(
            "/supplyChain/purchaseReconciliation/void",
            post(reconciliation_void),
        )
        // return
        .route("/supplyChain/purchaseReturn/list", get(return_list))
        .route("/supplyChain/purchaseReturn/detail", get(return_detail))
        .route("/supplyChain/purchaseReturn/save", post(return_save))
        .route("/supplyChain/purchaseReturn/confirm", post(return_confirm))
        .route("/supplyChain/purchaseReturn/delete", post(return_delete))
        // setting
        .route("/supplyChain/purchaseSetting/detail", get(setting_detail))
        .route("/supplyChain/purchaseSetting/save", post(setting_save))
}

#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    let paths = [
        ("GET", "/api/supplyChain/supplier/list"),
        ("GET", "/api/supplyChain/supplier/detail"),
        ("POST", "/api/supplyChain/supplier/create"),
        ("POST", "/api/supplyChain/supplier/update"),
        ("POST", "/api/supplyChain/supplier/status"),
        ("POST", "/api/supplyChain/supplier/delete"),
        ("POST", "/api/supplyChain/supplier/qualification/upload"),
        ("GET", "/api/supplyChain/supplierCategory/list"),
        ("POST", "/api/supplyChain/supplierCategory/save"),
        ("POST", "/api/supplyChain/supplierCategory/status"),
        ("POST", "/api/supplyChain/supplierCategory/delete"),
        ("GET", "/api/supplyChain/supplierProduct/list"),
        ("GET", "/api/supplyChain/supplierProduct/summary"),
        ("GET", "/api/supplyChain/supplierProduct/selection"),
        ("GET", "/api/supplyChain/supplierProduct/detail"),
        ("POST", "/api/supplyChain/supplierProduct/status"),
        ("POST", "/api/supplyChain/supplierProduct/delete"),
        ("POST", "/api/supplyChain/supplierProduct/batchDelete"),
        ("POST", "/api/supplyChain/supplierProduct/save"),
        ("POST", "/api/supplyChain/supplierProduct/batchSave"),
        ("GET", "/api/supplyChain/purchaseOrder/list"),
        ("GET", "/api/supplyChain/purchaseOrder/detail"),
        ("POST", "/api/supplyChain/purchaseOrder/create"),
        ("POST", "/api/supplyChain/purchaseOrder/update"),
        ("POST", "/api/supplyChain/purchaseOrder/submit"),
        ("POST", "/api/supplyChain/purchaseOrder/delete"),
        ("POST", "/api/supplyChain/purchaseOrder/finish"),
        ("POST", "/api/supplyChain/purchaseOrder/ship"),
        ("GET", "/api/supplyChain/purchaseReceipt/list"),
        ("GET", "/api/supplyChain/purchaseReceipt/detail"),
        ("POST", "/api/supplyChain/purchaseReceipt/save"),
        ("GET", "/api/supplyChain/purchaseReconciliation/list"),
        ("GET", "/api/supplyChain/purchaseReconciliation/detail"),
        ("GET", "/api/supplyChain/purchaseReconciliation/candidates"),
        ("GET", "/api/supplyChain/purchaseReconciliation/export"),
        ("POST", "/api/supplyChain/purchaseReconciliation/create"),
        ("POST", "/api/supplyChain/purchaseReconciliation/update"),
        ("POST", "/api/supplyChain/purchaseReconciliation/approve"),
        ("POST", "/api/supplyChain/purchaseReconciliation/reject"),
        ("POST", "/api/supplyChain/purchaseReconciliation/resubmit"),
        ("POST", "/api/supplyChain/purchaseReconciliation/void"),
        ("GET", "/api/supplyChain/purchaseReturn/list"),
        ("GET", "/api/supplyChain/purchaseReturn/detail"),
        ("POST", "/api/supplyChain/purchaseReturn/save"),
        ("POST", "/api/supplyChain/purchaseReturn/confirm"),
        ("POST", "/api/supplyChain/purchaseReturn/delete"),
        ("GET", "/api/supplyChain/purchaseSetting/detail"),
        ("POST", "/api/supplyChain/purchaseSetting/save"),
    ];
    paths
        .iter()
        .map(|(m, p)| nextrade_openapi::RouteDoc::new(m, p, "SupplyChainController"))
        .collect()
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Pg {
    page: Option<u64>,
    size: Option<u64>,
    keyword: Option<String>,
    id: Option<i64>,
    ids: Option<Vec<i64>>,
    status: Option<i32>,
    shop_id: Option<i32>,
    vendor_id: Option<i32>,
}

fn page_of(q: &Pg) -> (u64, u64, i64) {
    let page = q.page.unwrap_or(1).max(1);
    let size = q.size.unwrap_or(15).clamp(1, 200);
    (page, size, ((page - 1) * size) as i64)
}

async fn list_table(
    state: &AdminState,
    table: &str,
    q: &Pg,
    supplier_only: bool,
) -> Result<Value, AppError> {
    let (page, size, offset) = page_of(q);
    let mut cq: QueryBuilder<'_, Postgres> =
        QueryBuilder::new(format!("SELECT COUNT(*) FROM {table} WHERE 1=1"));
    let mut rq: QueryBuilder<'_, Postgres> =
        QueryBuilder::new(format!("SELECT * FROM {table} WHERE 1=1"));
    if table != "vendor" {
        cq.push(" AND is_del = 0");
        rq.push(" AND is_del = 0");
    }
    if supplier_only {
        cq.push(" AND source_type = 2");
        rq.push(" AND source_type = 2");
    }
    if let Some(shop_id) = q.shop_id {
        cq.push(" AND shop_id = ").push_bind(shop_id);
        rq.push(" AND shop_id = ").push_bind(shop_id);
    }
    if let Some(status) = q.status {
        cq.push(" AND status = ").push_bind(status);
        rq.push(" AND status = ").push_bind(status);
    }
    if let Some(kw) = q
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let like = format!("%{kw}%");
        let col = if table == "vendor" {
            "vendor_name"
        } else {
            "category_name"
        };
        cq.push(format!(" AND {col} ILIKE ")).push_bind(like.clone());
        rq.push(format!(" AND {col} ILIKE ")).push_bind(like);
    }
    rq.push(" ORDER BY ")
        .push(if table == "vendor" { "vendor_id" } else { "id" })
        .push(" DESC LIMIT ")
        .push_bind(size as i64)
        .push(" OFFSET ")
        .push_bind(offset);
    let (total,): (i64,) = cq
        .build_query_as()
        .fetch_one(&state.repos.pool)
        .await
        .map_err(db)?;
    let rows = rq.build().fetch_all(&state.repos.pool).await.map_err(db)?;
    let records: Vec<Value> = rows.iter().map(row_to_json).collect();
    Ok(json!({"records": records, "total": total, "page": page, "size": size}))
}

async fn detail_table(state: &AdminState, table: &str, id: i64) -> Result<Value, AppError> {
    let key = if table == "vendor" { "vendor_id" } else { "id" };
    let row = sqlx::query(&format!("SELECT * FROM {table} WHERE {key} = $1 LIMIT 1"))
        .bind(id)
        .fetch_optional(&state.repos.pool)
        .await
        .map_err(db)?;
    Ok(row.as_ref().map(row_to_json).unwrap_or(Value::Null))
}

async fn set_status(state: &AdminState, table: &str, id: i64, status: i32) -> Result<(), AppError> {
    let key = if table == "vendor" { "vendor_id" } else { "id" };
    sqlx::query(&format!("UPDATE {table} SET status = $1 WHERE {key} = $2"))
        .bind(status)
        .bind(id)
        .execute(&state.repos.pool)
        .await
        .map_err(db)?;
    Ok(())
}

async fn soft_delete(state: &AdminState, table: &str, id: i64) -> Result<(), AppError> {
    let key = if table == "vendor" { "vendor_id" } else { "id" };
    let sql = if table == "vendor" {
        format!("DELETE FROM {table} WHERE {key} = $1")
    } else {
        format!("UPDATE {table} SET is_del = 1 WHERE {key} = $1")
    };
    sqlx::query(&sql)
        .bind(id)
        .execute(&state.repos.pool)
        .await
        .map_err(db)?;
    Ok(())
}

fn db(e: sqlx::Error) -> AppError {
    AppError(Error::Database(e.to_string()))
}

pub(crate) fn row_to_json(row: &PgRow) -> Value {
    let mut map = Map::new();
    for (idx, col) in row.columns().iter().enumerate() {
        let name = col.name();
        let type_name = col.type_info().name().to_ascii_uppercase();
        let value = if type_name.contains("INT") {
            row.try_get::<Option<i64>, _>(idx)
                .ok()
                .flatten()
                .map(Value::from)
        } else if type_name.contains("DECIMAL") || type_name.contains("NUMERIC") {
            row.try_get::<Option<rust_decimal::Decimal>, _>(idx)
                .ok()
                .flatten()
                .map(|v| Value::String(v.to_string()))
        } else if type_name.contains("FLOAT") || type_name.contains("DOUBLE") {
            row.try_get::<Option<f64>, _>(idx)
                .ok()
                .flatten()
                .map(Value::from)
        } else if type_name.contains("TIMESTAMP") {
            row.try_get::<Option<sqlx::types::chrono::NaiveDateTime>, _>(idx)
                .ok()
                .flatten()
                .map(|v| Value::String(v.format("%Y-%m-%d %H:%M:%S").to_string()))
        } else if type_name == "DATE" {
            row.try_get::<Option<sqlx::types::chrono::NaiveDate>, _>(idx)
                .ok()
                .flatten()
                .map(|v| Value::String(v.format("%Y-%m-%d").to_string()))
        } else if type_name.contains("JSON") {
            row.try_get::<Option<Value>, _>(idx).ok().flatten()
        } else {
            row.try_get::<Option<String>, _>(idx)
                .ok()
                .flatten()
                .map(Value::from)
        };
        map.insert(name.to_string(), value.unwrap_or(Value::Null));
    }
    Value::Object(map)
}

// ————————————————— supplier —————————————————
async fn supplier_list(State(state): State<AdminState>, Query(q): Query<Pg>) -> ApiResult<Value> {
    ok(list_table(&state, "vendor", &q, true).await?)
}
async fn supplier_detail(State(state): State<AdminState>, Query(q): Query<Pg>) -> ApiResult<Value> {
    ok(detail_table(&state, "vendor", q.id.unwrap_or(0)).await?)
}
async fn supplier_create(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<i64> {
    let name = body
        .get("vendorName")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if name.is_empty() {
        return Err(AppError(Error::biz(400, "供应商名称不能为空")));
    }
    let now = nextrade_common::utils::now_seconds() as i64;
    let r: i32 = sqlx::query_scalar(
        "INSERT INTO vendor(source_type, shop_id, vendor_code, vendor_name, contact_mobile, purchase_category_id, vendor_logo, description, status, add_time) VALUES(2,$1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING vendor_id",
    )
    .bind(body.get("shopId").and_then(Value::as_i64).unwrap_or(0) as i32)
    .bind(body.get("vendorCode").and_then(Value::as_str).unwrap_or(""))
    .bind(name)
    .bind(body.get("contactMobile").and_then(Value::as_str).unwrap_or(""))
    .bind(body.get("purchaseCategoryId").and_then(Value::as_i64).unwrap_or(0))
    .bind(body.get("vendorLogo").and_then(Value::as_str).unwrap_or(""))
    .bind(body.get("description").and_then(Value::as_str).unwrap_or(""))
    .bind(body.get("status").and_then(Value::as_i64).unwrap_or(1) as i32)
    .bind(now)
    .fetch_one(&state.repos.pool).await.map_err(db)?;
    ok(i64::from(r))
}
async fn supplier_update(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<()> {
    let id = body
        .get("vendorId")
        .or_else(|| body.get("id"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    sqlx::query("UPDATE vendor SET vendor_name=COALESCE(NULLIF($1,''),vendor_name), contact_mobile=COALESCE(NULLIF($2,''),contact_mobile) WHERE vendor_id=$3 AND source_type=2")
        .bind(body.get("vendorName").and_then(Value::as_str).unwrap_or(""))
        .bind(body.get("contactMobile").and_then(Value::as_str).unwrap_or(""))
        .bind(id)
        .execute(&state.repos.pool).await.map_err(db)?;
    ok(())
}
async fn supplier_status(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<()> {
    let id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    let status = body.get("status").and_then(Value::as_i64).unwrap_or(1) as i32;
    set_status(&state, "vendor", id, status).await?;
    ok(())
}
async fn supplier_delete(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<()> {
    soft_delete(
        &state,
        "vendor",
        body.get("id").and_then(Value::as_i64).unwrap_or(0),
    )
    .await?;
    ok(())
}
async fn qualification_upload(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    mut multipart: Multipart,
) -> ApiResult<String> {
    let mut payload: Option<(Option<String>, Option<String>, Vec<u8>)> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError(Error::Validation(format!("文件不能为空: {e}"))))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let content_type = field.content_type().map(ToString::to_string);
        let file_name = field.file_name().map(ToString::to_string);
        let bytes = field
            .bytes()
            .await
            .map_err(|e| AppError(Error::Validation(format!("文件不能为空: {e}"))))?
            .to_vec();
        payload = Some((content_type, file_name, bytes));
        break;
    }
    let Some((content_type, file_name, bytes)) = payload else {
        return Err(AppError(Error::Validation("资质图片不能为空".into())));
    };
    if bytes.len() > 10 * 1024 * 1024 {
        return Err(AppError(Error::Validation(
            "单张资质图片不能超过10MB".into(),
        )));
    }
    if !matches!(
        content_type.as_deref(),
        Some("image/png" | "image/jpeg" | "image/jpg")
    ) {
        return Err(AppError(Error::Validation(
            "资质图片仅支持png、jpg、jpeg格式".into(),
        )));
    }
    let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let shop_id = TenantScope::current().write_shop_id(None)?;
    let vo = state
        .content
        .gallery_pic
        .upload_img(
            None,
            content_type.as_deref(),
            size,
            file_name.as_deref(),
            &bytes,
            shop_id,
            auth.id,
        )
        .await?;
    ok(serde_json::to_value(vo)
        .ok()
        .and_then(|v| {
            v.get("url")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_default())
}

// ————————————————— category —————————————————
async fn category_list(State(state): State<AdminState>, Query(q): Query<Pg>) -> ApiResult<Value> {
    ok(list_table(&state, "purchase_supplier_category", &q, false).await?)
}
async fn category_save(State(state): State<AdminState>, Json(body): Json<Value>) -> ApiResult<i64> {
    let id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    let name = body
        .get("categoryName")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let now = nextrade_common::utils::now_seconds() as i64;
    if id > 0 {
        sqlx::query("UPDATE purchase_supplier_category SET category_name=$1, sort_order=$2, status=$3, remark=$4, update_time=$5 WHERE id=$6")
            .bind(name)
            .bind(body.get("sortOrder").and_then(Value::as_i64).unwrap_or(50) as i32)
            .bind(body.get("status").and_then(Value::as_i64).unwrap_or(1) as i32)
            .bind(body.get("remark").and_then(Value::as_str).unwrap_or(""))
            .bind(now).bind(id)
            .execute(&state.repos.pool).await.map_err(db)?;
        ok(id)
    } else {
        let r: i64 = sqlx::query_scalar("INSERT INTO purchase_supplier_category(shop_id,category_name,sort_order,status,remark,create_time,update_time) VALUES($1,$2,$3,$4,$5,$6,$7) RETURNING id")
            .bind(body.get("shopId").and_then(Value::as_i64).unwrap_or(0) as i32)
            .bind(name)
            .bind(body.get("sortOrder").and_then(Value::as_i64).unwrap_or(50) as i32)
            .bind(body.get("status").and_then(Value::as_i64).unwrap_or(1) as i32)
            .bind(body.get("remark").and_then(Value::as_str).unwrap_or(""))
            .bind(now).bind(now)
            .fetch_one(&state.repos.pool).await.map_err(db)?;
        ok(r)
    }
}
async fn category_status(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<()> {
    set_status(
        &state,
        "purchase_supplier_category",
        body.get("id").and_then(Value::as_i64).unwrap_or(0),
        body.get("status").and_then(Value::as_i64).unwrap_or(1) as i32,
    )
    .await?;
    ok(())
}
async fn category_delete(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<()> {
    soft_delete(
        &state,
        "purchase_supplier_category",
        body.get("id").and_then(Value::as_i64).unwrap_or(0),
    )
    .await?;
    ok(())
}

// ————————————————— supplier product —————————————————
async fn product_list(State(state): State<AdminState>, Query(q): Query<Pg>) -> ApiResult<Value> {
    ok(list_table(&state, "vendor_product", &q, false).await?)
}
async fn product_detail(State(state): State<AdminState>, Query(q): Query<Pg>) -> ApiResult<Value> {
    ok(detail_table(&state, "vendor_product", q.id.unwrap_or(0)).await?)
}
async fn product_status(State(state): State<AdminState>, Json(body): Json<Value>) -> ApiResult<()> {
    let id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    let status = body.get("status").and_then(Value::as_i64).unwrap_or(1) as i32;
    sqlx::query("UPDATE vendor_product SET product_state=$1 WHERE id=$2")
        .bind(status)
        .bind(id)
        .execute(&state.repos.pool)
        .await
        .map_err(db)?;
    ok(())
}
async fn product_delete(State(state): State<AdminState>, Json(body): Json<Value>) -> ApiResult<()> {
    sqlx::query("UPDATE vendor_product SET is_del=1 WHERE id=$1")
        .bind(body.get("id").and_then(Value::as_i64).unwrap_or(0))
        .execute(&state.repos.pool)
        .await
        .map_err(db)?;
    ok(())
}
async fn product_batch_delete(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<()> {
    let ids = body
        .get("ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for id in ids {
        let _ = sqlx::query("UPDATE vendor_product SET is_del=1 WHERE id=$1")
            .bind(id.as_i64().unwrap_or(0))
            .execute(&state.repos.pool)
            .await;
    }
    ok(())
}
async fn product_save(State(state): State<AdminState>, Json(body): Json<Value>) -> ApiResult<i64> {
    save_supplier_product(&state, &body).await
}

async fn product_batch_save(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<()> {
    let products = body
        .get("products")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for item in products {
        let mut single = item.clone();
        if let Some(obj) = single.as_object_mut() {
            obj.insert(
                "vendorId".into(),
                body.get("vendorId").cloned().unwrap_or(Value::Null),
            );
            obj.insert(
                "shopId".into(),
                body.get("shopId").cloned().unwrap_or(Value::Null),
            );
        }
        save_supplier_product(&state, &single).await?;
    }
    ok(())
}

async fn save_supplier_product(state: &AdminState, body: &Value) -> ApiResult<i64> {
    let vendor_id = body.get("vendorId").and_then(Value::as_i64).unwrap_or(0) as i32;
    let product_id = body.get("productId").and_then(Value::as_i64).unwrap_or(0) as i32;
    if vendor_id <= 0 || product_id <= 0 {
        return Err(AppError(Error::biz(400, "请选择供应商和商品")));
    }
    let vendor = sqlx::query(
        "SELECT vendor_id FROM vendor WHERE vendor_id=$1 AND source_type=2 AND status=1 LIMIT 1",
    )
    .bind(vendor_id)
    .fetch_optional(&state.repos.pool)
    .await
    .map_err(db)?
    .ok_or_else(|| AppError(Error::biz(400, "供应商不存在或已停用")))?;
    let _ = vendor;
    let product = sqlx::query("SELECT product_id, product_name, product_sn, category_id, pic_url, pic_thumb, pic_original, pic_large, product_brief FROM product WHERE product_id=$1 AND is_delete=0 LIMIT 1")
        .bind(product_id).fetch_optional(&state.repos.pool).await.map_err(db)?
        .ok_or_else(|| AppError(Error::biz(400, "商品不存在")))?;
    let skus = body
        .get("skus")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if skus.is_empty() {
        return Err(AppError(Error::biz(400, "请选择采购规格")));
    }
    let now = nextrade_common::utils::now_seconds() as i64;
    let existing_id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    let mut tx = state.repos.pool.begin().await.map_err(db)?;
    let relation_id = if existing_id > 0 {
        let row = sqlx::query("SELECT vendor_id, source_product_id FROM vendor_product WHERE id=$1 AND is_del=0 LIMIT 1")
            .bind(existing_id).fetch_optional(&mut *tx).await.map_err(db)?
            .ok_or_else(|| AppError(Error::biz(400, "供应商商品不存在")))?;
        if row.try_get::<i32, _>("vendor_id").unwrap_or(0) != vendor_id
            || row.try_get::<i32, _>("source_product_id").unwrap_or(0) != product_id
        {
            return Err(AppError(Error::biz(400, "供应商和来源商品不可变更")));
        }
        sqlx::query("UPDATE vendor_product SET product_name=$1, product_state=$2, sku_type=$3, update_time=$4 WHERE id=$5")
            .bind(product.try_get::<String, _>("product_name").unwrap_or_default())
            .bind(body.get("status").and_then(Value::as_i64).unwrap_or(1) as i32)
            .bind(if skus.len() > 1 { 2 } else { 1 })
            .bind(now).bind(existing_id)
            .execute(&mut *tx).await.map_err(db)?;
        sqlx::query("DELETE FROM vendor_product_sku WHERE vendor_product_id=$1")
            .bind(existing_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        existing_id
    } else {
        let duplicate: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM vendor_product WHERE vendor_id=$1 AND source_product_id=$2 AND is_del=0")
            .bind(vendor_id).bind(product_id).fetch_one(&mut *tx).await.map_err(db)?;
        if duplicate.0 > 0 {
            return Err(AppError(Error::biz(400, "该供应商已绑定此商品")));
        }
        sqlx::query_scalar("INSERT INTO vendor_product(product_name,product_sn,product_category_id,product_sn_generate_type,product_brief,product_state,sku_type,audit_state,pic_url,pic_thumb,pic_original,pic_large,is_recycle,vendor_id,source_product_id,create_time,create_by_id,update_time,update_by_id,update_by_name,is_del) VALUES($1,$2,$3,0,$4,$5,$6,1,$7,$8,$9,$10,0,$11,$12,$13,0,$14,0,'',0) RETURNING id")
            .bind(product.try_get::<String, _>("product_name").unwrap_or_default())
            .bind(product.try_get::<Option<String>, _>("product_sn").unwrap_or_default())
            .bind(product.try_get::<i32, _>("category_id").unwrap_or(0))
            .bind(product.try_get::<String, _>("product_brief").unwrap_or_default())
            .bind(body.get("status").and_then(Value::as_i64).unwrap_or(1) as i32)
            .bind(if skus.len() > 1 { 2 } else { 1 })
            .bind(product.try_get::<String, _>("pic_url").unwrap_or_default())
            .bind(product.try_get::<String, _>("pic_thumb").unwrap_or_default())
            .bind(product.try_get::<String, _>("pic_original").unwrap_or_default())
            .bind(product.try_get::<String, _>("pic_large").unwrap_or_default())
            .bind(vendor_id).bind(product_id)
            .bind(now).bind(now)
            .fetch_one(&mut *tx).await.map_err(db)?
    };
    for sku in skus {
        let source_sku_id = sku.get("skuId").and_then(Value::as_i64).filter(|v| *v > 0);
        let price = rust_decimal::Decimal::from_str(
            &sku.get("supplyPrice")
                .and_then(Value::as_str)
                .unwrap_or("0"),
        )
        .unwrap_or(rust_decimal::Decimal::ZERO);
        let min_qty = sku
            .get("minPurchaseQty")
            .and_then(Value::as_i64)
            .unwrap_or(1);
        let mut attr = "默认".to_string();
        let mut sn = product
            .try_get::<Option<String>, _>("product_sn")
            .unwrap_or_default();
        if let Some(source_sku_id) = source_sku_id {
            if let Some(row) = sqlx::query("SELECT sku_value, sku_sn, sku_tsn, sku_weight, sku_stock FROM product_sku WHERE sku_id=$1 LIMIT 1")
                .bind(source_sku_id).fetch_optional(&mut *tx).await.map_err(db)?
            {
                attr = row.try_get::<String, _>("sku_value").unwrap_or(attr);
                sn = row.try_get::<Option<String>, _>("sku_sn").unwrap_or(sn);
                sqlx::query("INSERT INTO vendor_product_sku(vendor_product_id,source_sku_id,min_purchase_qty,sku_attr_val,sku_sn,sku_tsn,sku_weight,sku_stock,supply_price,supply_price_limit_type,supply_price_limit_val,sales_volume,vendor_id,create_time,create_by_id,update_time,update_by_id,update_by_name,is_del) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,0,0,0,$10,$11,0,$12,0,'',0)")
                    .bind(relation_id).bind(source_sku_id).bind(min_qty).bind(&attr).bind(&sn)
                    .bind(row.try_get::<Option<String>, _>("sku_tsn").unwrap_or_default())
                    .bind(row.try_get::<Option<rust_decimal::Decimal>, _>("sku_weight").ok().flatten())
                    .bind(row.try_get::<Option<i32>, _>("sku_stock").ok().flatten())
                    .bind(price).bind(vendor_id).bind(now).bind(now)
                    .execute(&mut *tx).await.map_err(db)?;
                continue;
            }
        }
        sqlx::query("INSERT INTO vendor_product_sku(vendor_product_id,source_sku_id,min_purchase_qty,sku_attr_val,sku_sn,supply_price,supply_price_limit_type,supply_price_limit_val,sales_volume,vendor_id,create_time,create_by_id,update_time,update_by_id,update_by_name,is_del) VALUES($1,NULL,$2,$3,$4,$5,0,0,0,$6,$7,0,$8,0,'',0)")
            .bind(relation_id).bind(min_qty).bind(attr).bind(sn).bind(price).bind(vendor_id).bind(now).bind(now)
            .execute(&mut *tx).await.map_err(db)?;
    }
    tx.commit().await.map_err(db)?;
    ok(relation_id)
}

// ————————————————— purchase order —————————————————
async fn order_list(State(state): State<AdminState>, Query(q): Query<Pg>) -> ApiResult<Value> {
    ok(list_table(&state, "purchase_order", &q, false).await?)
}
async fn order_detail(State(state): State<AdminState>, Query(q): Query<Pg>) -> ApiResult<Value> {
    let id = q.id.unwrap_or(0);
    let mut order = detail_table(&state, "purchase_order", id).await?;
    let rows =
        sqlx::query("SELECT * FROM purchase_order_item WHERE order_id=$1 AND is_del=0 ORDER BY id")
            .bind(id)
            .fetch_all(&state.repos.pool)
            .await
            .map_err(db)?;
    let items: Vec<Value> = rows.iter().map(row_to_json).collect();
    if let Some(obj) = order.as_object_mut() {
        obj.insert("items".into(), Value::Array(items));
    }
    ok(order)
}
/// 金额入参双口径解析（我方修复，门店采购全流程批）：历史实现只认字符串，
/// 而线上抽屉提交的 purchasePrice / returnPrice 均为 JSON 数字 —— 按字符串
/// 取不到值便恒落 0，采购价/退货价与后续入库、对账金额全线失真。现字符串
/// 与数字均可解析，缺失/非法按 0（退货价缺失的回退口径在 return_save 单独处理）。
fn parse_decimal_flex(v: Option<&Value>) -> rust_decimal::Decimal {
    match v {
        Some(Value::String(s)) => {
            rust_decimal::Decimal::from_str(s.trim()).unwrap_or(rust_decimal::Decimal::ZERO)
        }
        Some(Value::Number(n)) => {
            rust_decimal::Decimal::from_str(&n.to_string()).unwrap_or(rust_decimal::Decimal::ZERO)
        }
        _ => rust_decimal::Decimal::ZERO,
    }
}

/// 采购单明细入参（create / update 共用解析口径；purchasePrice 走
/// [`parse_decimal_flex`] 双口径，历史为只认字符串、数字恒落 0）。
#[derive(Debug, Clone)]
struct OrderItemInput {
    sku_id: i64,
    qty: i64,
    price: rust_decimal::Decimal,
}

fn parse_order_items(body: &Value) -> Vec<OrderItemInput> {
    body.get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|item| OrderItemInput {
            sku_id: item
                .get("vendorProductSkuId")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            qty: item.get("quantity").and_then(Value::as_i64).unwrap_or(0),
            price: parse_decimal_flex(item.get("purchasePrice")),
        })
        .collect()
}

/// 正式单全量校验（与现 create 同口径）：至少一行、每行规格与数量均合法。
/// 入参为 (vendor_product_sku_id, quantity) 对，create 与 submit（校验已存明细）共用。
fn validate_formal_pairs(pairs: &[(i64, i64)]) -> Result<(), &'static str> {
    if pairs.is_empty() {
        return Err("请选择采购商品");
    }
    if pairs.iter().any(|(sku_id, qty)| *sku_id <= 0 || *qty <= 0) {
        return Err("采购商品或数量不合法");
    }
    Ok(())
}

/// 草稿可落库的明细：只保留已选规格的行（未选规格的空行不落库），
/// 数量未填按 0 暂存，待补全后提交时再走全量校验。
fn draft_items(items: &[OrderItemInput]) -> Vec<OrderItemInput> {
    items
        .iter()
        .filter(|item| item.sku_id > 0)
        .map(|item| OrderItemInput {
            sku_id: item.sku_id,
            qty: item.qty.max(0),
            price: item.price,
        })
        .collect()
}

/// 供应商快照（采购单落库口径与历史 create 一致：名称/编码/联系方式/所属店铺）。
struct VendorSnapshot {
    vendor_name: String,
    vendor_code: String,
    vendor_contact: String,
    shop_id: i32,
}

async fn load_vendor_snapshot(
    state: &AdminState,
    vendor_id: i32,
) -> Result<VendorSnapshot, AppError> {
    let vendor = sqlx::query("SELECT vendor_name, vendor_code, contact_mobile, shop_id FROM vendor WHERE vendor_id=$1 AND source_type=2 LIMIT 1")
        .bind(vendor_id)
        .fetch_optional(&state.repos.pool).await.map_err(db)?
        .ok_or_else(|| AppError(Error::biz(400, "供应商不存在")))?;
    Ok(VendorSnapshot {
        vendor_name: vendor.try_get("vendor_name").unwrap_or_default(),
        vendor_code: vendor.try_get("vendor_code").unwrap_or_default(),
        vendor_contact: vendor.try_get("contact_mobile").unwrap_or_default(),
        shop_id: vendor.try_get("shop_id").unwrap_or(0),
    })
}

/// 明细逐行落库（create / update 共用；行规格必须可解析，口径与历史 create 一致）。
/// 返回 (商品金额合计, 订购数量合计)。
async fn insert_order_items(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    order_id: i64,
    items: &[OrderItemInput],
    now: i64,
) -> Result<(rust_decimal::Decimal, i64), AppError> {
    let mut total_amount = rust_decimal::Decimal::ZERO;
    let mut total_qty = 0i64;
    for item in items {
        let row = sqlx::query(
            "SELECT s.id AS sku_id, s.vendor_product_id, s.source_sku_id, p.product_id, p.product_name, p.product_sn, p.pic_thumb, p.product_unit, s.sku_attr_val, s.sku_sn FROM vendor_product_sku s JOIN vendor_product vp ON vp.id=s.vendor_product_id JOIN product p ON p.product_id=vp.source_product_id WHERE s.id=$1 AND s.is_del=0 LIMIT 1",
        )
        .bind(item.sku_id)
        .fetch_optional(&mut **tx).await.map_err(db)?
        .ok_or_else(|| AppError(Error::biz(400, "供应商商品规格不存在")))?;
        let product_id: i32 = row.try_get("product_id").unwrap_or(0);
        let source_sku_id: Option<i32> = row.try_get("source_sku_id").ok().flatten();
        let amount = item.price * rust_decimal::Decimal::from(item.qty);
        sqlx::query(
            "INSERT INTO purchase_order_item(order_id,vendor_product_id,vendor_product_sku_id,product_id,sku_id,product_name,sku_name,product_sn,sku_sn,pic_thumb,product_unit,quantity,received_quantity,purchase_price,amount,create_time,update_time) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,0,$13,$14,$15,$16)",
        )
        .bind(order_id)
        .bind(row.try_get::<i64, _>("vendor_product_id").unwrap_or(0))
        .bind(item.sku_id)
        .bind(product_id)
        .bind(source_sku_id.unwrap_or(0))
        .bind(row.try_get::<String, _>("product_name").unwrap_or_default())
        .bind(row.try_get::<String, _>("sku_attr_val").unwrap_or_default())
        .bind(row.try_get::<String, _>("product_sn").unwrap_or_default())
        .bind(row.try_get::<String, _>("sku_sn").unwrap_or_default())
        .bind(row.try_get::<String, _>("pic_thumb").unwrap_or_default())
        .bind(row.try_get::<String, _>("product_unit").unwrap_or_default())
        .bind(item.qty)
        .bind(item.price)
        .bind(amount)
        .bind(now).bind(now)
        .execute(&mut **tx).await.map_err(db)?;
        total_amount += amount;
        total_qty += item.qty;
    }
    Ok((total_amount, total_qty))
}

async fn order_create(State(state): State<AdminState>, Json(body): Json<Value>) -> ApiResult<i64> {
    // 我方新增：draft=true 保存草稿（status=0），放宽必填校验——供应商/明细可暂缺，
    // 已填内容原样落库；正式提交（draft 缺省/false）沿用历史全量校验，status=1 待入库。
    let draft = body.get("draft").and_then(Value::as_bool).unwrap_or(false);
    let vendor_id = body.get("vendorId").and_then(Value::as_i64).unwrap_or(0) as i32;
    let inputs = parse_order_items(&body);
    let snapshot = if vendor_id > 0 {
        Some(load_vendor_snapshot(&state, vendor_id).await?)
    } else if draft {
        None
    } else {
        return Err(AppError(Error::biz(400, "请选择供应商")));
    };
    let items = if draft {
        draft_items(&inputs)
    } else {
        let pairs: Vec<(i64, i64)> = inputs.iter().map(|i| (i.sku_id, i.qty)).collect();
        validate_formal_pairs(&pairs).map_err(|msg| AppError(Error::biz(400, msg)))?;
        inputs
    };
    let (vendor_name, vendor_code, vendor_contact, shop_id) = match &snapshot {
        Some(s) => (
            s.vendor_name.clone(),
            s.vendor_code.clone(),
            s.vendor_contact.clone(),
            s.shop_id,
        ),
        None => (
            String::new(),
            String::new(),
            String::new(),
            body.get("shopId").and_then(Value::as_i64).unwrap_or(0) as i32,
        ),
    };
    let now = nextrade_common::utils::now_seconds() as i64;
    let order_sn = format!("PO{}{:04}", now, rand_suffix());
    let mut tx = state.repos.pool.begin().await.map_err(db)?;
    let order_id: i64 = sqlx::query_scalar(
        "INSERT INTO purchase_order(order_sn,type,shop_id,vendor_id,vendor_name,vendor_code,vendor_contact,status,expected_delivery_date,shipping_method,logistics_id,logistics_name,tracking_no,remark,product_amount,ordered_quantity,received_quantity,version,create_time,update_time) VALUES($1,1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,0,0,0,0,$14,$15) RETURNING id",
    )
    .bind(&order_sn)
    .bind(shop_id)
    .bind(vendor_id)
    .bind(&vendor_name)
    .bind(&vendor_code)
    .bind(&vendor_contact)
    .bind(if draft { 0 } else { 1 })
    .bind(body.get("expectedDeliveryDate").and_then(Value::as_i64).unwrap_or(0))
    .bind(body.get("shippingMethod").and_then(Value::as_i64).unwrap_or(1) as i32)
    .bind(body.get("logisticsId").and_then(Value::as_i64).unwrap_or(0) as i32)
    .bind(body.get("logisticsName").and_then(Value::as_str).unwrap_or(""))
    .bind(body.get("trackingNo").and_then(Value::as_str).unwrap_or(""))
    .bind(body.get("remark").and_then(Value::as_str).unwrap_or(""))
    .bind(now).bind(now)
    .fetch_one(&mut *tx).await.map_err(db)?;

    let (total_amount, total_qty) = insert_order_items(&mut tx, order_id, &items, now).await?;
    sqlx::query("UPDATE purchase_order SET product_amount=$1, ordered_quantity=$2 WHERE id=$3")
        .bind(total_amount)
        .bind(total_qty)
        .bind(order_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    tx.commit().await.map_err(db)?;
    ok(order_id)
}
async fn order_update(State(state): State<AdminState>, Json(body): Json<Value>) -> ApiResult<()> {
    let id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    let now = nextrade_common::utils::now_seconds() as i64;
    let row = sqlx::query("SELECT status, vendor_id, vendor_name, vendor_code, vendor_contact, shop_id FROM purchase_order WHERE id=$1 AND is_del=0 LIMIT 1")
        .bind(id)
        .fetch_optional(&state.repos.pool).await.map_err(db)?
        .ok_or_else(|| AppError(Error::biz(400, "采购订单不存在")))?;
    let status: i32 = row.try_get("status").unwrap_or(0);
    if status == 3 || status == 4 {
        return Err(AppError(Error::biz(400, "已完成或已关闭的采购单不可编辑")));
    }
    if status == 1 || status == 2 || status == 5 {
        // 正式单沿用历史口径：仅更新期望送达/发货方式/物流/备注等头部字段，
        // 不动供应商、明细与状态（历史 SQL 的 status NOT IN (3,4) 已由上方校验等价覆盖）。
        // status=5（已发货，我方新增）同属正式单头部可编辑范围；若落入下方草稿分支，
        // 其收尾 UPDATE 带 status=0 条件会静默 no-op 而明细却被替换，故必须在此收口。
        sqlx::query(
            "UPDATE purchase_order SET expected_delivery_date=COALESCE($1,expected_delivery_date), shipping_method=COALESCE($2,shipping_method), logistics_id=$3, logistics_name=$4, tracking_no=$5, remark=$6, update_time=$7 WHERE id=$8",
        )
        .bind(body.get("expectedDeliveryDate").and_then(Value::as_i64))
        .bind(body.get("shippingMethod").and_then(Value::as_i64).map(|v| v as i32))
        .bind(body.get("logisticsId").and_then(Value::as_i64).unwrap_or(0) as i32)
        .bind(body.get("logisticsName").and_then(Value::as_str).unwrap_or(""))
        .bind(body.get("trackingNo").and_then(Value::as_str).unwrap_or(""))
        .bind(body.get("remark").and_then(Value::as_str).unwrap_or(""))
        .bind(now).bind(id)
        .execute(&state.repos.pool).await.map_err(db)?;
        return ok(());
    }
    // 草稿（status=0，我方新增）：可改供应商/头部/明细。
    // draft=true 继续存草稿（放宽校验）；否则对最终内容执行全量校验后转正式单（status=1）。
    let target_draft = body.get("draft").and_then(Value::as_bool).unwrap_or(false);
    let cur_vendor_id: i32 = row.try_get("vendor_id").unwrap_or(0);
    let vendor_touched = body.get("vendorId").is_some();
    let vendor_id = if vendor_touched {
        body.get("vendorId").and_then(Value::as_i64).unwrap_or(0) as i32
    } else {
        cur_vendor_id
    };
    // 供应商快照：入参改供应商或转正式单时按当前供应商行刷新（兼校验存在），
    // 草稿未改供应商时沿用已存快照；清空供应商时快照置空、店铺取入参 shopId。
    let snapshot = if vendor_id > 0 && (vendor_touched || !target_draft) {
        Some(load_vendor_snapshot(&state, vendor_id).await?)
    } else {
        None
    };
    let (vendor_name, vendor_code, vendor_contact, shop_id) = if vendor_id <= 0 {
        (
            String::new(),
            String::new(),
            String::new(),
            body.get("shopId").and_then(Value::as_i64).unwrap_or(0) as i32,
        )
    } else if let Some(s) = &snapshot {
        (
            s.vendor_name.clone(),
            s.vendor_code.clone(),
            s.vendor_contact.clone(),
            s.shop_id,
        )
    } else {
        (
            row.try_get("vendor_name").unwrap_or_default(),
            row.try_get("vendor_code").unwrap_or_default(),
            row.try_get("vendor_contact").unwrap_or_default(),
            row.try_get("shop_id").unwrap_or(0),
        )
    };
    // 最终明细口径：入参带 items 时整体替换，否则沿用已存明细。
    let inputs = parse_order_items(&body);
    let has_items = body.get("items").and_then(Value::as_array).is_some();
    let final_items = if has_items {
        if target_draft {
            draft_items(&inputs)
        } else {
            inputs
        }
    } else {
        Vec::new()
    };
    let final_pairs: Vec<(i64, i64)> = if has_items {
        final_items.iter().map(|i| (i.sku_id, i.qty)).collect()
    } else {
        let stored = sqlx::query(
            "SELECT vendor_product_sku_id, quantity FROM purchase_order_item WHERE order_id=$1 AND is_del=0",
        )
        .bind(id)
        .fetch_all(&state.repos.pool)
        .await
        .map_err(db)?;
        stored
            .iter()
            .map(|r| {
                (
                    r.try_get("vendor_product_sku_id").unwrap_or(0),
                    i64::from(r.try_get::<i32, _>("quantity").unwrap_or(0)),
                )
            })
            .collect()
    };
    if !target_draft {
        if vendor_id <= 0 {
            return Err(AppError(Error::biz(400, "请选择供应商")));
        }
        validate_formal_pairs(&final_pairs).map_err(|msg| AppError(Error::biz(400, msg)))?;
    }
    let mut tx = state.repos.pool.begin().await.map_err(db)?;
    if has_items {
        sqlx::query("UPDATE purchase_order_item SET is_del=1 WHERE order_id=$1 AND is_del=0")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let (total_amount, total_qty) =
            insert_order_items(&mut tx, id, &final_items, now).await?;
        sqlx::query("UPDATE purchase_order SET product_amount=$1, ordered_quantity=$2 WHERE id=$3")
            .bind(total_amount)
            .bind(total_qty)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
    }
    sqlx::query(
        "UPDATE purchase_order SET vendor_id=$1, vendor_name=$2, vendor_code=$3, vendor_contact=$4, shop_id=$5, expected_delivery_date=COALESCE($6,expected_delivery_date), shipping_method=COALESCE($7,shipping_method), logistics_id=$8, logistics_name=$9, tracking_no=$10, remark=$11, status=$12, update_time=$13 WHERE id=$14 AND status=0",
    )
    .bind(vendor_id)
    .bind(&vendor_name)
    .bind(&vendor_code)
    .bind(&vendor_contact)
    .bind(shop_id)
    .bind(body.get("expectedDeliveryDate").and_then(Value::as_i64))
    .bind(body.get("shippingMethod").and_then(Value::as_i64).map(|v| v as i32))
    .bind(body.get("logisticsId").and_then(Value::as_i64).unwrap_or(0) as i32)
    .bind(body.get("logisticsName").and_then(Value::as_str).unwrap_or(""))
    .bind(body.get("trackingNo").and_then(Value::as_str).unwrap_or(""))
    .bind(body.get("remark").and_then(Value::as_str).unwrap_or(""))
    .bind(if target_draft { 0 } else { 1 })
    .bind(now).bind(id)
    .execute(&mut *tx).await.map_err(db)?;
    tx.commit().await.map_err(db)?;
    ok(())
}
/// 提交草稿（我方新增）：对已存内容执行与 create 相同的全量校验，
/// 通过后 status 0→1（待入库）并按当前供应商行刷新快照；校验不过不改状态。
async fn order_submit(State(state): State<AdminState>, Json(body): Json<Value>) -> ApiResult<()> {
    let id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    let row = sqlx::query("SELECT status, vendor_id FROM purchase_order WHERE id=$1 AND is_del=0 LIMIT 1")
        .bind(id)
        .fetch_optional(&state.repos.pool).await.map_err(db)?
        .ok_or_else(|| AppError(Error::biz(400, "采购订单不存在")))?;
    let status: i32 = row.try_get("status").unwrap_or(-1);
    if status != 0 {
        return Err(AppError(Error::biz(400, "只有草稿采购单可以提交")));
    }
    let vendor_id: i32 = row.try_get("vendor_id").unwrap_or(0);
    if vendor_id <= 0 {
        return Err(AppError(Error::biz(400, "请选择供应商")));
    }
    let snapshot = load_vendor_snapshot(&state, vendor_id).await?;
    let stored = sqlx::query(
        "SELECT vendor_product_sku_id, quantity FROM purchase_order_item WHERE order_id=$1 AND is_del=0",
    )
    .bind(id)
    .fetch_all(&state.repos.pool)
    .await
    .map_err(db)?;
    let pairs: Vec<(i64, i64)> = stored
        .iter()
        .map(|r| {
            (
                r.try_get("vendor_product_sku_id").unwrap_or(0),
                i64::from(r.try_get::<i32, _>("quantity").unwrap_or(0)),
            )
        })
        .collect();
    validate_formal_pairs(&pairs).map_err(|msg| AppError(Error::biz(400, msg)))?;
    let now = nextrade_common::utils::now_seconds() as i64;
    sqlx::query("UPDATE purchase_order SET status=1, vendor_name=$1, vendor_code=$2, vendor_contact=$3, shop_id=$4, update_time=$5 WHERE id=$6 AND status=0")
        .bind(&snapshot.vendor_name)
        .bind(&snapshot.vendor_code)
        .bind(&snapshot.vendor_contact)
        .bind(snapshot.shop_id)
        .bind(now).bind(id)
        .execute(&state.repos.pool).await.map_err(db)?;
    ok(())
}
/// 删除草稿（我方新增）：仅草稿（status=0）可删，订单与明细一并软删；
/// 正式单沿用既有口径（本模块原本无采购单删除端点，正式单仍不可删）。
async fn order_delete(State(state): State<AdminState>, Json(body): Json<Value>) -> ApiResult<()> {
    let id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    let row = sqlx::query("SELECT status FROM purchase_order WHERE id=$1 AND is_del=0 LIMIT 1")
        .bind(id)
        .fetch_optional(&state.repos.pool)
        .await
        .map_err(db)?
        .ok_or_else(|| AppError(Error::biz(400, "采购订单不存在")))?;
    let status: i32 = row.try_get("status").unwrap_or(-1);
    if status != 0 {
        return Err(AppError(Error::biz(400, "只有草稿采购单可以删除")));
    }
    let now = nextrade_common::utils::now_seconds() as i64;
    let mut tx = state.repos.pool.begin().await.map_err(db)?;
    sqlx::query("UPDATE purchase_order SET is_del=1, update_time=$1 WHERE id=$2 AND status=0")
        .bind(now)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    sqlx::query("UPDATE purchase_order_item SET is_del=1 WHERE order_id=$1 AND is_del=0")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    tx.commit().await.map_err(db)?;
    ok(())
}
async fn order_finish(State(state): State<AdminState>, Json(body): Json<Value>) -> ApiResult<()> {
    let id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    let now = nextrade_common::utils::now_seconds() as i64;
    // status=5（已发货）同样可完结：语义与从待入库完结一致（放弃剩余收货），
    // 否则已发货未收货的单除收货外无任何出口（自动结束循环亦覆盖 5，见 scheduler）。
    sqlx::query("UPDATE purchase_order SET status=3, finish_reason=$1, finish_time=$2, update_time=$3 WHERE id=$4 AND status IN (1,2,5)")
        .bind(body.get("remark").and_then(Value::as_str).unwrap_or(""))
        .bind(now).bind(now).bind(id)
        .execute(&state.repos.pool).await.map_err(db)?;
    ok(())
}

/// 供应商发货（我方新增，论坛帖 777863 门店采购流程的「发货」环）：
/// 采购单此前只有 待入库→(部分)入库 的状态流，供应商发货无落点 —— 物流
/// 字段只能在建单时由买方填写、且无任何状态表达。本端点把发货显式化：
/// 仅 status=1（待入库）可发货，入参物流公司/单号覆盖落库，状态 1→5
/// （已发货·待收货）；动作写入 purchase_operation_log（该表原版有、我方
/// 此前全仓零写入，本批起激活）。发货后仍走既有入库（收货）链确认收货。
async fn order_ship(State(state): State<AdminState>, Json(body): Json<Value>) -> ApiResult<()> {
    let id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    let row = sqlx::query(
        "SELECT status FROM purchase_order WHERE id=$1 AND is_del=0 LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&state.repos.pool)
    .await
    .map_err(db)?
    .ok_or_else(|| AppError(Error::biz(400, "采购订单不存在")))?;
    let status: i32 = row.try_get("status").unwrap_or(-1);
    if status != 1 {
        return Err(AppError(Error::biz(400, "只有待入库的采购单可以发货")));
    }
    let now = nextrade_common::utils::now_seconds() as i64;
    let logistics_name = body
        .get("logisticsName")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let tracking_no = body
        .get("trackingNo")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let mut tx = state.repos.pool.begin().await.map_err(db)?;
    sqlx::query(
        "UPDATE purchase_order SET status=5, logistics_id=COALESCE($1,logistics_id), logistics_name=COALESCE($2,logistics_name), tracking_no=COALESCE($3,tracking_no), update_time=$4 WHERE id=$5 AND status=1",
    )
    .bind(body.get("logisticsId").and_then(Value::as_i64).map(|v| v as i32))
    .bind(logistics_name)
    .bind(tracking_no)
    .bind(now)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    sqlx::query(
        "INSERT INTO purchase_operation_log(business_type,business_id,action,from_status,to_status,remark,create_time,create_by_id,create_by_name,update_time,update_by_id,update_by_name,is_del) VALUES('purchase_order',$1,'ship',1,5,$2,$3,0,'', $3,0,'',0)",
    )
    .bind(id)
    .bind(match (logistics_name, tracking_no) {
        (Some(name), Some(no)) => format!("供应商发货：{name} {no}"),
        (Some(name), None) => format!("供应商发货：{name}"),
        (None, Some(no)) => format!("供应商发货：单号 {no}"),
        (None, None) => "供应商发货".to_string(),
    })
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    tx.commit().await.map_err(db)?;
    ok(())
}

fn rand_suffix() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0))
        % 10000
}

// ————————————————— receipt / reconciliation / return —————————————————
async fn receipt_list(State(state): State<AdminState>, Query(q): Query<Pg>) -> ApiResult<Value> {
    ok(list_table(&state, "purchase_receipt", &q, false).await?)
}
async fn receipt_detail(State(state): State<AdminState>, Query(q): Query<Pg>) -> ApiResult<Value> {
    let id = q.id.unwrap_or(0);
    let mut receipt = detail_table(&state, "purchase_receipt", id).await?;
    let rows = sqlx::query(
        "SELECT * FROM purchase_receipt_item WHERE receipt_id=$1 AND is_del=0 ORDER BY id",
    )
    .bind(id)
    .fetch_all(&state.repos.pool)
    .await
    .map_err(db)?;
    if let Some(obj) = receipt.as_object_mut() {
        obj.insert(
            "items".into(),
            Value::Array(rows.iter().map(row_to_json).collect()),
        );
    }
    ok(receipt)
}
/// 采购设置 `allow_over_receive` 的生效值（组织行优先、平台行兜底、
/// 空 = 继承）：该设置原版有（purchase_setting），此前全仓无消费方 ——
/// 设置页可配但入库恒拒超收。本批起由 receipt_save 消费。
async fn load_allow_over_receive(state: &AdminState, shop_id: i32) -> Result<bool, AppError> {
    for sid in [shop_id, 0] {
        let row = sqlx::query(
            "SELECT allow_over_receive FROM purchase_setting WHERE shop_id=$1 AND is_del=0 ORDER BY id DESC LIMIT 1",
        )
        .bind(sid)
        .fetch_optional(&state.repos.pool)
        .await
        .map_err(db)?;
        if let Some(row) = row {
            if let Some(v) = row
                .try_get::<Option<i32>, _>("allow_over_receive")
                .ok()
                .flatten()
            {
                return Ok(v == 1);
            }
        }
    }
    Ok(false)
}

/// 入库校验前置的收齐行（receipt_save 专用）：归属/数量/超收口径全部
/// 校验通过后才进入库存落点与落库事务，避免半程失败留下脏单。
struct ReceiptLine {
    order_item_id: i64,
    qty: i64,
    price: rust_decimal::Decimal,
    amount: rust_decimal::Decimal,
    product_id: i32,
    sku_id: i32,
    product_name: String,
    sku_name: String,
    product_sn: String,
    sku_sn: String,
    pic_thumb: String,
    product_unit: String,
}

async fn receipt_save(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<Value>,
) -> ApiResult<i64> {
    let mut order_id = body.get("orderId").and_then(Value::as_i64).unwrap_or(0);
    let mut inputs = body
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if inputs.is_empty() {
        return Err(AppError(Error::biz(400, "请选择入库商品")));
    }
    if order_id <= 0 {
        // 直采入库：先按采购单创建逻辑生成采购单，再把供应商规格映射到采购明细。
        let order_body = serde_json::json!({
            "vendorId": body.get("vendorId"),
            "expectedDeliveryDate": body.get("expectedDeliveryDate"),
            "shippingMethod": body.get("shippingMethod"),
            "logisticsId": body.get("logisticsId"),
            "logisticsName": body.get("logisticsName"),
            "trackingNo": body.get("trackingNo"),
            "remark": body.get("remark"),
            "items": inputs.clone()
        });
        let created = order_create(State(state.clone()), Json(order_body)).await?;
        order_id = created
            .0
            .data
            .ok_or_else(|| AppError(Error::biz(400, "采购单创建失败")))?;
        let mut mapped = Vec::new();
        for input in inputs {
            let sku_id = input
                .get("vendorProductSkuId")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let qty = input.get("quantity").and_then(Value::as_i64).unwrap_or(0);
            let row = sqlx::query("SELECT id FROM purchase_order_item WHERE order_id=$1 AND vendor_product_sku_id=$2 AND is_del=0 ORDER BY id DESC LIMIT 1")
                .bind(order_id).bind(sku_id)
                .fetch_optional(&state.repos.pool).await.map_err(db)?
                .ok_or_else(|| AppError(Error::biz(400, "采购明细生成失败")))?;
            mapped.push(serde_json::json!({
                "orderItemId": row.try_get::<i64, _>("id").unwrap_or(0),
                "quantity": qty
            }));
        }
        inputs = mapped;
    }
    let order = sqlx::query("SELECT * FROM purchase_order WHERE id=$1 AND is_del=0 LIMIT 1")
        .bind(order_id)
        .fetch_optional(&state.repos.pool)
        .await
        .map_err(db)?
        .ok_or_else(|| AppError(Error::biz(400, "采购订单不存在")))?;
    let status: i32 = order.try_get("status").unwrap_or(0);
    // 隔离守卫（草稿功能红线）：草稿（status=0）不是正式采购单，绝不可被入库引用。
    if status == 0 {
        return Err(AppError(Error::biz(400, "草稿采购单不能入库，请先提交")));
    }
    // status=5（已发货，我方新增）与 1/2 同为可收货状态：发货环不改变收货口径。
    if status != 1 && status != 2 && status != 5 {
        return Err(AppError(Error::biz(400, "采购订单已结束或状态不允许入库")));
    }
    let shop_id: i32 = order.try_get("shop_id").unwrap_or(0);
    let vendor_id: i32 = order.try_get("vendor_id").unwrap_or(0);
    let now = nextrade_common::utils::now_seconds() as i64;
    let receipt_sn = format!("PR{}{:04}", now, rand_suffix());
    let allow_over = load_allow_over_receive(&state, shop_id).await?;

    // —— 校验前置（无写入）：逐行核对明细归属与可收数量，收齐落库快照 ——
    let mut lines: Vec<ReceiptLine> = Vec::new();
    let mut total_qty = 0i64;
    let mut total_amount = rust_decimal::Decimal::ZERO;
    for input in &inputs {
        let order_item_id = input
            .get("orderItemId")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let qty = input.get("quantity").and_then(Value::as_i64).unwrap_or(0);
        if order_item_id <= 0 || qty <= 0 {
            return Err(AppError(Error::biz(400, "入库商品或数量不合法")));
        }
        let item = sqlx::query(
            "SELECT * FROM purchase_order_item WHERE id=$1 AND order_id=$2 AND is_del=0 LIMIT 1",
        )
        .bind(order_item_id)
        .bind(order_id)
        .fetch_optional(&state.repos.pool)
        .await
        .map_err(db)?
        .ok_or_else(|| AppError(Error::biz(400, "入库商品不属于采购订单")))?;
        let ordered: i32 = item.try_get("quantity").unwrap_or(0);
        let received: i32 = item.try_get("received_quantity").unwrap_or(0);
        // 超收口径：采购设置 allow_over_receive 生效（组织行→平台行）时放行，
        // 否则与历史一致拒绝超收（完成判定 received>=quantity 天然兼容超收）。
        if !allow_over && received + qty as i32 > ordered {
            return Err(AppError(Error::biz(400, "入库数量超过剩余采购数量")));
        }
        let price = item
            .try_get::<rust_decimal::Decimal, _>("purchase_price")
            .unwrap_or_default();
        let amount = price * rust_decimal::Decimal::from(qty);
        lines.push(ReceiptLine {
            order_item_id,
            qty,
            price,
            amount,
            product_id: item.try_get("product_id").unwrap_or(0),
            sku_id: item.try_get("sku_id").unwrap_or(0),
            product_name: item.try_get::<String, _>("product_name").unwrap_or_default(),
            sku_name: item.try_get::<String, _>("sku_name").unwrap_or_default(),
            product_sn: item.try_get::<String, _>("product_sn").unwrap_or_default(),
            sku_sn: item.try_get::<String, _>("sku_sn").unwrap_or_default(),
            pic_thumb: item.try_get::<String, _>("pic_thumb").unwrap_or_default(),
            product_unit: item.try_get::<String, _>("product_unit").unwrap_or_default(),
        });
        total_qty += qty;
        total_amount += amount;
    }

    // —— 库存落点：委托库存体系出入库服务（原版 record_type 11 = 采购入库）——
    // 此前本链直接 UPDATE product/product_sku 的平台库存：门店采购（组织
    // shop_id>0）落不到门店分配库存、不生成出入库记录与库存流水、
    // purchase_receipt.stock_record_id 恒写 0。现按订单组织由库存服务统一
    // 裁决落点（组织自有商品直落商品库存；平台商品落门店分配库存、未分配
    // 明确报错），生成的出入库记录 id 回填 stock_record_id。
    // 顺序口径：上方校验已全部前置通过；库存服务本身为逐行落库（与平台
    // 自有出入库同口径、非单事务），其后再开单据事务落入库单与订单进度。
    let stock_record_id = state
        .catalog
        .product_stock
        .stock_in(
            &StockInDTO {
                r#type: Some(stock_record_type::PURCHASE_IN),
                comment: Some(format!("采购入库 {receipt_sn}")),
                product_stock_record_detail_list: lines
                    .iter()
                    .map(|l| StockRecordDetailItem {
                        record_detail_id: None,
                        stock_record_id: None,
                        product_id: Some(l.product_id),
                        sku_id: if l.sku_id > 0 { Some(l.sku_id) } else { None },
                        quantity: Some(l.qty as i32),
                    })
                    .collect(),
                product_total: Some(total_qty as i32),
            },
            &StockOperator::new(shop_id, Some(auth.username.clone())),
        )
        .await?;

    // —— 单据事务：入库单 + 明细 + 订单已收量/状态推进 ——
    let mut tx = state.repos.pool.begin().await.map_err(db)?;
    let receipt_id: i64 = sqlx::query_scalar(
        "INSERT INTO purchase_receipt(receipt_sn,order_id,shop_id,vendor_id,business_date,status,total_quantity,total_amount,stock_record_id,remark,confirm_time,version,create_time,update_time) VALUES($1,$2,$3,$4,$5,2,$6,$7,$8,$9,$10,0,$11,$12) RETURNING id",
    )
    .bind(&receipt_sn)
    .bind(order_id)
    .bind(shop_id)
    .bind(vendor_id)
    .bind(body.get("businessDate").and_then(Value::as_i64).unwrap_or(now))
    .bind(total_qty)
    .bind(total_amount)
    .bind(stock_record_id)
    .bind(body.get("remark").and_then(Value::as_str).unwrap_or(""))
    .bind(now).bind(now).bind(now)
    .fetch_one(&mut *tx).await.map_err(db)?;
    for line in &lines {
        sqlx::query(
            "UPDATE purchase_order_item SET received_quantity=received_quantity+$1 WHERE id=$2",
        )
        .bind(line.qty as i32)
        .bind(line.order_item_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO purchase_receipt_item(receipt_id,order_item_id,product_id,sku_id,product_name,sku_name,product_sn,sku_sn,pic_thumb,product_unit,quantity,returned_quantity,purchase_price,amount,create_time,update_time) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,0,$12,$13,$14,$15)",
        )
        .bind(receipt_id).bind(line.order_item_id).bind(line.product_id).bind(line.sku_id)
        .bind(&line.product_name)
        .bind(&line.sku_name)
        .bind(&line.product_sn)
        .bind(&line.sku_sn)
        .bind(&line.pic_thumb)
        .bind(&line.product_unit)
        .bind(line.qty as i32).bind(line.price).bind(line.amount).bind(now).bind(now)
        .execute(&mut *tx).await.map_err(db)?;
    }
    let (pending,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM purchase_order_item WHERE order_id=$1 AND is_del=0 AND received_quantity < quantity",
    ).bind(order_id).fetch_one(&mut *tx).await.map_err(db)?;
    let next_status = if pending == 0 { 3 } else { 2 };
    sqlx::query("UPDATE purchase_order SET status=$1, received_quantity=received_quantity+$2, update_time=$3 WHERE id=$4")
        .bind(next_status).bind(total_qty).bind(now).bind(order_id)
        .execute(&mut *tx).await.map_err(db)?;
    tx.commit().await.map_err(db)?;
    ok(receipt_id)
}

async fn reconciliation_list(
    State(state): State<AdminState>,
    Query(q): Query<Pg>,
) -> ApiResult<Value> {
    ok(list_table(&state, "purchase_reconciliation", &q, false).await?)
}
async fn reconciliation_detail(
    State(state): State<AdminState>,
    Query(q): Query<Pg>,
) -> ApiResult<Value> {
    let id = q.id.unwrap_or(0);
    let mut rec = detail_table(&state, "purchase_reconciliation", id).await?;
    let sources = sqlx::query(
        "SELECT s.*, CASE WHEN s.source_type=1 THEN (SELECT r.confirm_time FROM purchase_receipt r WHERE r.id=s.source_id LIMIT 1) ELSE (SELECT pr.confirm_time FROM purchase_return pr WHERE pr.id=s.source_id LIMIT 1) END AS confirm_time FROM purchase_reconciliation_source s WHERE s.reconciliation_id=$1 AND s.is_del=0 ORDER BY s.id",
    )
    .bind(id)
    .fetch_all(&state.repos.pool).await.map_err(db)?;
    let items = sqlx::query("SELECT * FROM purchase_reconciliation_item WHERE reconciliation_id=$1 AND is_del=0 ORDER BY id")
        .bind(id)
        .fetch_all(&state.repos.pool).await.map_err(db)?;
    if let Some(obj) = rec.as_object_mut() {
        obj.insert(
            "sources".into(),
            Value::Array(sources.iter().map(row_to_json).collect()),
        );
        obj.insert(
            "items".into(),
            Value::Array(items.iter().map(row_to_json).collect()),
        );
    }
    ok(rec)
}
/// 对账候选来源（我方重写，门店采购全流程批）：历史实现直接把
/// purchase_receipt 原表行当候选返回 —— 与前端选择器契约（sourceType/
/// sourceId/sn/amount/confirmTime/items）完全不符：前端拼出的 sourceId
/// 恒为空、退货单永不出现、vendorId 参数被 Pg 结构忽略（无此字段）。
/// 现口径：入库单（status=2 已确认）与退货单（status=2 已确认）的 UNION
/// 视图，按 vendorId/shopId 过滤，逐行附明细；且排除已被未作废对账单
/// 占用的来源（占用判定见 save_reconciliation 的来源占用校验）。
async fn reconciliation_candidates(
    State(state): State<AdminState>,
    Query(q): Query<Pg>,
) -> ApiResult<Value> {
    let (page, size, offset) = page_of(&q);
    // 两分支共用的占用排除片段（source_type 在各分支内固定）。
    let occupied = |source_type: i32, alias: &str| {
        format!(
            "NOT EXISTS (SELECT 1 FROM purchase_reconciliation_source s \
             JOIN purchase_reconciliation rec ON rec.id = s.reconciliation_id \
             WHERE s.is_del = 0 AND rec.is_del = 0 AND rec.status <> 4 \
             AND s.source_type = {source_type} AND s.source_id = {alias}.id)"
        )
    };
    let branch = |table: &str, sn_col: &str, source_type: i32| {
        format!(
            "SELECT {source_type} AS source_type, d.id AS source_id, d.{sn_col} AS sn, \
             d.business_date AS business_date, d.total_amount AS amount, \
             d.total_quantity AS total_quantity, d.vendor_id AS vendor_id, \
             d.shop_id AS shop_id, d.confirm_time AS confirm_time, v.vendor_name AS vendor_name \
             FROM {table} d LEFT JOIN vendor v ON v.vendor_id = d.vendor_id \
             WHERE d.is_del = 0 AND d.status = 2 AND {occupied}",
            occupied = occupied(source_type, "d")
        )
    };
    let union_sql = format!(
        "{} UNION ALL {}",
        branch("purchase_receipt", "receipt_sn", 1),
        branch("purchase_return", "return_sn", 2)
    );
    let mut cq: QueryBuilder<'_, Postgres> =
        QueryBuilder::new(format!("SELECT COUNT(*) FROM ({union_sql}) c WHERE 1=1"));
    let mut rq: QueryBuilder<'_, Postgres> =
        QueryBuilder::new(format!("SELECT * FROM ({union_sql}) c WHERE 1=1"));
    if let Some(vendor_id) = q.vendor_id {
        cq.push(" AND c.vendor_id = ").push_bind(vendor_id);
        rq.push(" AND c.vendor_id = ").push_bind(vendor_id);
    }
    if let Some(shop_id) = q.shop_id {
        cq.push(" AND c.shop_id = ").push_bind(shop_id);
        rq.push(" AND c.shop_id = ").push_bind(shop_id);
    }
    rq.push(" ORDER BY c.business_date DESC, c.source_id DESC LIMIT ")
        .push_bind(size as i64)
        .push(" OFFSET ")
        .push_bind(offset);
    let (total,): (i64,) = cq
        .build_query_as()
        .fetch_one(&state.repos.pool)
        .await
        .map_err(db)?;
    let rows = rq.build().fetch_all(&state.repos.pool).await.map_err(db)?;

    // 明细批量附挂（入库/退货各一次 ANY 查询，避免逐行往返）。
    let receipt_ids: Vec<i64> = rows
        .iter()
        .filter(|r| r.try_get::<i32, _>("source_type").unwrap_or(0) == 1)
        .map(|r| r.try_get::<i64, _>("source_id").unwrap_or(0))
        .collect();
    let return_ids: Vec<i64> = rows
        .iter()
        .filter(|r| r.try_get::<i32, _>("source_type").unwrap_or(0) == 2)
        .map(|r| r.try_get::<i64, _>("source_id").unwrap_or(0))
        .collect();
    let mut items_by_source: std::collections::HashMap<(i32, i64), Vec<Value>> =
        std::collections::HashMap::new();
    if !receipt_ids.is_empty() {
        let item_rows = sqlx::query(
            "SELECT receipt_id, product_id, sku_id, product_name, sku_name, product_sn, sku_sn, pic_thumb, product_unit, quantity, purchase_price, amount FROM purchase_receipt_item WHERE receipt_id = ANY($1) AND is_del=0 ORDER BY id",
        )
        .bind(&receipt_ids)
        .fetch_all(&state.repos.pool)
        .await
        .map_err(db)?;
        for r in &item_rows {
            let price = r
                .try_get::<rust_decimal::Decimal, _>("purchase_price")
                .unwrap_or_default();
            items_by_source
                .entry((1, r.try_get::<i64, _>("receipt_id").unwrap_or(0)))
                .or_default()
                .push(json!({
                    "productId": r.try_get::<i32, _>("product_id").unwrap_or(0),
                    "skuId": r.try_get::<i32, _>("sku_id").unwrap_or(0),
                    "productName": r.try_get::<String, _>("product_name").unwrap_or_default(),
                    "skuName": r.try_get::<String, _>("sku_name").unwrap_or_default(),
                    "productSn": r.try_get::<String, _>("product_sn").unwrap_or_default(),
                    "skuSn": r.try_get::<String, _>("sku_sn").unwrap_or_default(),
                    "picThumb": r.try_get::<String, _>("pic_thumb").unwrap_or_default(),
                    "productUnit": r.try_get::<String, _>("product_unit").unwrap_or_default(),
                    "quantity": r.try_get::<i32, _>("quantity").unwrap_or(0),
                    "price": price.to_string(),
                    "purchasePrice": price.to_string(),
                    "amount": r.try_get::<rust_decimal::Decimal, _>("amount").unwrap_or_default().to_string(),
                }));
        }
    }
    if !return_ids.is_empty() {
        let item_rows = sqlx::query(
            "SELECT return_id, product_id, sku_id, product_name, sku_name, product_sn, sku_sn, pic_thumb, product_unit, quantity, return_price, amount FROM purchase_return_item WHERE return_id = ANY($1) AND is_del=0 ORDER BY id",
        )
        .bind(&return_ids)
        .fetch_all(&state.repos.pool)
        .await
        .map_err(db)?;
        for r in &item_rows {
            let price = r
                .try_get::<rust_decimal::Decimal, _>("return_price")
                .unwrap_or_default();
            items_by_source
                .entry((2, r.try_get::<i64, _>("return_id").unwrap_or(0)))
                .or_default()
                .push(json!({
                    "productId": r.try_get::<i32, _>("product_id").unwrap_or(0),
                    "skuId": r.try_get::<i32, _>("sku_id").unwrap_or(0),
                    "productName": r.try_get::<String, _>("product_name").unwrap_or_default(),
                    "skuName": r.try_get::<String, _>("sku_name").unwrap_or_default(),
                    "productSn": r.try_get::<String, _>("product_sn").unwrap_or_default(),
                    "skuSn": r.try_get::<String, _>("sku_sn").unwrap_or_default(),
                    "picThumb": r.try_get::<String, _>("pic_thumb").unwrap_or_default(),
                    "productUnit": r.try_get::<String, _>("product_unit").unwrap_or_default(),
                    "quantity": r.try_get::<i32, _>("quantity").unwrap_or(0),
                    "price": price.to_string(),
                    "returnPrice": price.to_string(),
                    "amount": r.try_get::<rust_decimal::Decimal, _>("amount").unwrap_or_default().to_string(),
                }));
        }
    }

    let records: Vec<Value> = rows
        .iter()
        .map(|r| {
            let source_type = r.try_get::<i32, _>("source_type").unwrap_or(1);
            let source_id = r.try_get::<i64, _>("source_id").unwrap_or(0);
            json!({
                "sourceType": source_type,
                "sourceId": source_id,
                "sn": r.try_get::<String, _>("sn").unwrap_or_default(),
                "businessDate": r.try_get::<i64, _>("business_date").unwrap_or(0),
                "amount": r.try_get::<rust_decimal::Decimal, _>("amount").unwrap_or_default().to_string(),
                "totalQuantity": r.try_get::<i32, _>("total_quantity").unwrap_or(0),
                "vendorId": r.try_get::<i32, _>("vendor_id").unwrap_or(0),
                "vendorName": r.try_get::<Option<String>, _>("vendor_name").ok().flatten().unwrap_or_default(),
                "shopId": r.try_get::<i32, _>("shop_id").unwrap_or(0),
                "confirmTime": r.try_get::<Option<i64>, _>("confirm_time").ok().flatten(),
                "items": items_by_source.get(&(source_type, source_id)).cloned().unwrap_or_default(),
            })
        })
        .collect();
    ok(json!({"records": records, "total": total, "page": page, "size": size}))
}

async fn reconciliation_export(
    State(state): State<AdminState>,
    Query(q): Query<Pg>,
) -> ApiResult<Value> {
    let mut qb: QueryBuilder<'_, Postgres> =
        QueryBuilder::new("SELECT * FROM purchase_reconciliation WHERE is_del=0");
    if let Some(shop_id) = q.shop_id {
        qb.push(" AND shop_id=").push_bind(shop_id);
    }
    if let Some(status) = q.status {
        qb.push(" AND status=").push_bind(status);
    }
    qb.push(" ORDER BY id DESC LIMIT 5000");
    let rows = qb.build().fetch_all(&state.repos.pool).await.map_err(db)?;
    ok(json!(rows.iter().map(row_to_json).collect::<Vec<_>>()))
}
async fn reconciliation_create(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<i64> {
    save_reconciliation(&state, body, false).await
}
async fn reconciliation_update(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<()> {
    save_reconciliation(&state, body, true).await?;
    ok(())
}

async fn save_reconciliation(state: &AdminState, body: Value, is_update: bool) -> ApiResult<i64> {
    let vendor_id = body.get("vendorId").and_then(Value::as_i64).unwrap_or(0) as i32;
    if vendor_id <= 0 {
        return Err(AppError(Error::biz(400, "请选择供应商")));
    }
    let vendor = sqlx::query("SELECT vendor_id, vendor_name, vendor_code, contact_mobile, shop_id FROM vendor WHERE vendor_id=$1 AND source_type=2 LIMIT 1")
        .bind(vendor_id).fetch_optional(&state.repos.pool).await.map_err(db)?
        .ok_or_else(|| AppError(Error::biz(400, "供应商不存在")))?;
    let shop_id: i32 = vendor.try_get("shop_id").unwrap_or(0);
    let sources = body
        .get("sources")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if sources.is_empty() {
        return Err(AppError(Error::biz(400, "请选择对账来源单据")));
    }
    let now = nextrade_common::utils::now_seconds() as i64;
    let existing_id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    let mut tx = state.repos.pool.begin().await.map_err(db)?;
    let reconciliation_id = if is_update && existing_id > 0 {
        let row = sqlx::query(
            "SELECT status FROM purchase_reconciliation WHERE id=$1 AND is_del=0 LIMIT 1",
        )
        .bind(existing_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(|| AppError(Error::biz(400, "采购对账单不存在")))?;
        let status: i32 = row.try_get("status").unwrap_or(0);
        if status != 3 {
            return Err(AppError(Error::biz(400, "只有已驳回采购对账单可以修改")));
        }
        // 先释放旧来源占用（来源单据 reconciliation_id 回写），再删来源/快照重建。
        sqlx::query("UPDATE purchase_receipt r SET reconciliation_id=NULL FROM purchase_reconciliation_source s WHERE s.reconciliation_id=$1 AND s.source_type=1 AND s.source_id=r.id")
            .bind(existing_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query("UPDATE purchase_return pr SET reconciliation_id=NULL FROM purchase_reconciliation_source s WHERE s.reconciliation_id=$1 AND s.source_type=2 AND s.source_id=pr.id")
            .bind(existing_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query("DELETE FROM purchase_reconciliation_source WHERE reconciliation_id=$1")
            .bind(existing_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query("DELETE FROM purchase_reconciliation_item WHERE reconciliation_id=$1")
            .bind(existing_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query("UPDATE purchase_reconciliation SET vendor_id=$1, shop_id=$2, remark=$3, status=1, update_time=$4 WHERE id=$5")
            .bind(vendor_id).bind(shop_id).bind(body.get("remark").and_then(Value::as_str).unwrap_or("")).bind(now).bind(existing_id)
            .execute(&mut *tx).await.map_err(db)?;
        existing_id
    } else {
        let sn = format!("PC{}{:04}", now, rand_suffix());
        let snapshot = serde_json::json!({
            "vendorId": vendor_id,
            "vendorName": vendor.try_get::<String,_>("vendor_name").unwrap_or_default(),
            "vendorCode": vendor.try_get::<String,_>("vendor_code").unwrap_or_default(),
            "contactMobile": vendor.try_get::<String,_>("contact_mobile").unwrap_or_default()
        })
        .to_string();
        sqlx::query_scalar("INSERT INTO purchase_reconciliation(reconciliation_sn,shop_id,vendor_id,status,receipt_amount,return_amount,total_amount,supplier_snapshot,remark,version,create_time,update_time) VALUES($1,$2,$3,1,0,0,0,$4,$5,0,$6,$7) RETURNING id")
            .bind(&sn).bind(shop_id).bind(vendor_id).bind(snapshot)
            .bind(body.get("remark").and_then(Value::as_str).unwrap_or(""))
            .bind(now).bind(now)
            .fetch_one(&mut *tx).await.map_err(db)?
    };
    let mut receipt_amount = rust_decimal::Decimal::ZERO;
    let mut return_amount = rust_decimal::Decimal::ZERO;
    let mut seen_sources: std::collections::HashSet<(i32, i64)> =
        std::collections::HashSet::new();
    for source in sources {
        let source_type = source
            .get("sourceType")
            .and_then(Value::as_i64)
            .unwrap_or(0) as i32;
        let source_id = source.get("sourceId").and_then(Value::as_i64).unwrap_or(0);
        let row = if source_type == 1 {
            sqlx::query("SELECT id, receipt_sn, business_date, total_amount FROM purchase_receipt WHERE id=$1 AND vendor_id=$2 AND status=2 AND is_del=0 LIMIT 1")
                .bind(source_id).bind(vendor_id).fetch_optional(&mut *tx).await.map_err(db)?
        } else if source_type == 2 {
            sqlx::query("SELECT id, return_sn AS receipt_sn, business_date, total_amount FROM purchase_return WHERE id=$1 AND vendor_id=$2 AND status=2 AND is_del=0 LIMIT 1")
                .bind(source_id).bind(vendor_id).fetch_optional(&mut *tx).await.map_err(db)?
        } else {
            None
        };
        let Some(row) = row else {
            return Err(AppError(Error::biz(400, "对账来源单据不存在或状态不正确")));
        };
        // 来源占用校验（我方修复）：历史实现对来源占用零校验 —— 同一入库/
        // 退货单可被多张对账单重复纳入，形成重复应付。现口径：同一来源在
        // 未作废对账单中只能出现一次（更新自身时排除自身；同批重复亦拒）。
        if !seen_sources.insert((source_type, source_id)) {
            return Err(AppError(Error::biz(400, "对账来源单据重复")));
        }
        let occupied: Option<(i64,)> = sqlx::query_as(
            "SELECT rec.id FROM purchase_reconciliation_source s JOIN purchase_reconciliation rec ON rec.id = s.reconciliation_id WHERE s.is_del = 0 AND rec.is_del = 0 AND rec.status <> 4 AND s.source_type = $1 AND s.source_id = $2 AND rec.id <> $3 LIMIT 1",
        )
        .bind(source_type)
        .bind(source_id)
        .bind(reconciliation_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if occupied.is_some() {
            return Err(AppError(Error::biz(400, "对账来源单据已被其他对账单占用")));
        }
        let amount: rust_decimal::Decimal = row.try_get("total_amount").unwrap_or_default();
        sqlx::query("INSERT INTO purchase_reconciliation_source(reconciliation_id,source_type,source_id,source_sn,business_date,source_amount,create_time,update_time) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
            .bind(reconciliation_id).bind(source_type).bind(source_id)
            .bind(row.try_get::<String,_>("receipt_sn").unwrap_or_default())
            .bind(row.try_get::<i64,_>("business_date").unwrap_or(0))
            .bind(amount).bind(now).bind(now)
            .execute(&mut *tx).await.map_err(db)?;
        // 占用回写 + 商品快照（我方补齐）：来源单据 reconciliation_id 此前
        // 从不回写、purchase_reconciliation_item 快照从不生成（对账详情
        // items 恒空）。快照单价：入库取 purchase_price、退货取 return_price。
        if source_type == 1 {
            sqlx::query("UPDATE purchase_receipt SET reconciliation_id=$1 WHERE id=$2")
                .bind(reconciliation_id).bind(source_id)
                .execute(&mut *tx).await.map_err(db)?;
            sqlx::query("INSERT INTO purchase_reconciliation_item(reconciliation_id,source_type,source_id,source_item_id,product_id,sku_id,product_name,sku_name,product_sn,sku_sn,pic_thumb,product_unit,quantity,price,amount,create_time,update_time) SELECT $1,1,$2,id,product_id,sku_id,product_name,sku_name,product_sn,sku_sn,pic_thumb,product_unit,quantity,purchase_price,amount,$3,$3 FROM purchase_receipt_item WHERE receipt_id=$2 AND is_del=0")
                .bind(reconciliation_id).bind(source_id).bind(now)
                .execute(&mut *tx).await.map_err(db)?;
            receipt_amount += amount;
        } else {
            sqlx::query("UPDATE purchase_return SET reconciliation_id=$1 WHERE id=$2")
                .bind(reconciliation_id).bind(source_id)
                .execute(&mut *tx).await.map_err(db)?;
            sqlx::query("INSERT INTO purchase_reconciliation_item(reconciliation_id,source_type,source_id,source_item_id,product_id,sku_id,product_name,sku_name,product_sn,sku_sn,pic_thumb,product_unit,quantity,price,amount,create_time,update_time) SELECT $1,2,$2,id,product_id,sku_id,product_name,sku_name,product_sn,sku_sn,pic_thumb,product_unit,quantity,return_price,amount,$3,$3 FROM purchase_return_item WHERE return_id=$2 AND is_del=0")
                .bind(reconciliation_id).bind(source_id).bind(now)
                .execute(&mut *tx).await.map_err(db)?;
            return_amount += amount;
        }
    }
    let total = receipt_amount - return_amount;
    sqlx::query("UPDATE purchase_reconciliation SET receipt_amount=$1, return_amount=$2, total_amount=$3 WHERE id=$4")
        .bind(receipt_amount).bind(return_amount).bind(total).bind(reconciliation_id)
        .execute(&mut *tx).await.map_err(db)?;
    tx.commit().await.map_err(db)?;
    ok(reconciliation_id)
}
async fn reconciliation_approve(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<()> {
    let id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    sqlx::query("UPDATE purchase_reconciliation SET status=2, review_remark=$1, review_time=$2, update_time=$3 WHERE id=$4 AND status=1")
        .bind(body.get("remark").and_then(Value::as_str).unwrap_or(""))
        .bind(nextrade_common::utils::now_seconds() as i64)
        .bind(nextrade_common::utils::now_seconds() as i64)
        .bind(id).execute(&state.repos.pool).await.map_err(db)?;
    ok(())
}
async fn reconciliation_reject(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<()> {
    let id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    let remark = body
        .get("remark")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if remark.is_empty() {
        return Err(AppError(Error::biz(400, "请填写驳回原因")));
    }
    let now = nextrade_common::utils::now_seconds() as i64;
    sqlx::query("UPDATE purchase_reconciliation SET status=3, review_remark=$1, review_time=$2, update_time=$3 WHERE id=$4 AND status=1")
        .bind(remark).bind(now).bind(now).bind(id).execute(&state.repos.pool).await.map_err(db)?;
    ok(())
}
async fn reconciliation_resubmit(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<()> {
    let id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    sqlx::query("UPDATE purchase_reconciliation SET status=1, review_remark=NULL, review_time=NULL, update_time=$1 WHERE id=$2 AND status=3")
        .bind(nextrade_common::utils::now_seconds() as i64).bind(id)
        .execute(&state.repos.pool).await.map_err(db)?;
    ok(())
}
async fn reconciliation_void(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<()> {
    let id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    let now = nextrade_common::utils::now_seconds() as i64;
    // 作废须同事务释放来源占用（与 save_reconciliation 的占用回写成对），
    // 否则作废单据的来源永久不可再对账；状态未命中（非已驳回）时不释放。
    let mut tx = state.repos.pool.begin().await.map_err(db)?;
    let res = sqlx::query("UPDATE purchase_reconciliation SET status=4, review_remark=$1, review_time=$2, update_time=$3 WHERE id=$4 AND status=3")
        .bind(body.get("remark").and_then(Value::as_str).unwrap_or("")).bind(now).bind(now).bind(id)
        .execute(&mut *tx).await.map_err(db)?;
    if res.rows_affected() == 1 {
        sqlx::query("UPDATE purchase_receipt r SET reconciliation_id=NULL FROM purchase_reconciliation_source s WHERE s.reconciliation_id=$1 AND s.source_type=1 AND s.source_id=r.id")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query("UPDATE purchase_return pr SET reconciliation_id=NULL FROM purchase_reconciliation_source s WHERE s.reconciliation_id=$1 AND s.source_type=2 AND s.source_id=pr.id")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
    }
    tx.commit().await.map_err(db)?;
    ok(())
}

async fn return_list(State(state): State<AdminState>, Query(q): Query<Pg>) -> ApiResult<Value> {
    ok(list_table(&state, "purchase_return", &q, false).await?)
}
async fn return_detail(State(state): State<AdminState>, Query(q): Query<Pg>) -> ApiResult<Value> {
    ok(detail_table(&state, "purchase_return", q.id.unwrap_or(0)).await?)
}
/// 退货校验前置的收齐行（return_save 专用，口径同 ReceiptLine）。
struct ReturnLine {
    receipt_item_id: i64,
    qty: i64,
    original_price: rust_decimal::Decimal,
    return_price: rust_decimal::Decimal,
    amount: rust_decimal::Decimal,
    product_id: i32,
    sku_id: i32,
    product_name: String,
    sku_name: String,
    product_sn: String,
    sku_sn: String,
    pic_thumb: String,
    product_unit: String,
    /// 入库行总量 / 已退量（落 returnable 快照用）
    received_qty: i32,
    returned_qty: i32,
    reason: String,
}

async fn return_save(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<Value>,
) -> ApiResult<i64> {
    let receipt_id = body.get("receiptId").and_then(Value::as_i64).unwrap_or(0);
    if receipt_id <= 0 {
        return Err(AppError(Error::biz(400, "采购入库单不能为空")));
    }
    let receipt =
        sqlx::query("SELECT * FROM purchase_receipt WHERE id=$1 AND is_del=0 AND status=2 LIMIT 1")
            .bind(receipt_id)
            .fetch_optional(&state.repos.pool)
            .await
            .map_err(db)?
            .ok_or_else(|| AppError(Error::biz(400, "采购入库单不存在或未确认")))?;
    let shop_id: i32 = receipt.try_get("shop_id").unwrap_or(0);
    let vendor_id: i32 = receipt.try_get("vendor_id").unwrap_or(0);
    let order_id: i64 = receipt.try_get("order_id").unwrap_or(0);
    let inputs = body
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if inputs.is_empty() {
        return Err(AppError(Error::biz(400, "请选择退货商品")));
    }
    let now = nextrade_common::utils::now_seconds() as i64;
    let return_sn = format!("PT{}{:04}", now, rand_suffix());

    // —— 校验前置（无写入）——
    let mut lines: Vec<ReturnLine> = Vec::new();
    let mut total_qty = 0i64;
    let mut total_amount = rust_decimal::Decimal::ZERO;
    for input in &inputs {
        let item_id = input
            .get("receiptItemId")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let qty = input.get("quantity").and_then(Value::as_i64).unwrap_or(0);
        if item_id <= 0 || qty <= 0 {
            return Err(AppError(Error::biz(400, "退货商品或数量不合法")));
        }
        let item = sqlx::query(
            "SELECT * FROM purchase_receipt_item WHERE id=$1 AND receipt_id=$2 AND is_del=0 LIMIT 1",
        )
        .bind(item_id)
        .bind(receipt_id)
        .fetch_optional(&state.repos.pool)
        .await
        .map_err(db)?
        .ok_or_else(|| AppError(Error::biz(400, "退货明细不属于入库单")))?;
        let q: i32 = item.try_get("quantity").unwrap_or(0);
        let returned: i32 = item.try_get("returned_quantity").unwrap_or(0);
        if returned + qty as i32 > q {
            return Err(AppError(Error::biz(400, "退货数量超过可退数量")));
        }
        let original_price = item
            .try_get::<rust_decimal::Decimal, _>("purchase_price")
            .unwrap_or_default();
        // 退货价双口径解析（我方修复，同 purchasePrice）：线上前端传 JSON
        // 数字，历史只认字符串且缺失/非法回退不一致 —— 数字恒被解析成 0，
        // 退货金额全线失真。现：字符串/数字均可，非法回退原采购价，缺失
        // 亦按原采购价（与「按原价退」的业务直觉一致）。
        let return_price = match input.get("returnPrice") {
            Some(Value::String(s)) => rust_decimal::Decimal::from_str(s.trim())
                .unwrap_or(original_price),
            Some(Value::Number(n)) => rust_decimal::Decimal::from_str(&n.to_string())
                .unwrap_or(original_price),
            _ => original_price,
        };
        let amount = return_price * rust_decimal::Decimal::from(qty);
        lines.push(ReturnLine {
            receipt_item_id: item_id,
            qty,
            original_price,
            return_price,
            amount,
            product_id: item.try_get("product_id").unwrap_or(0),
            sku_id: item.try_get("sku_id").unwrap_or(0),
            product_name: item.try_get::<String, _>("product_name").unwrap_or_default(),
            sku_name: item.try_get::<String, _>("sku_name").unwrap_or_default(),
            product_sn: item.try_get::<String, _>("product_sn").unwrap_or_default(),
            sku_sn: item.try_get::<String, _>("sku_sn").unwrap_or_default(),
            pic_thumb: item.try_get::<String, _>("pic_thumb").unwrap_or_default(),
            product_unit: item.try_get::<String, _>("product_unit").unwrap_or_default(),
            received_qty: q,
            returned_qty: returned,
            reason: input
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        });
        total_qty += qty;
        total_amount += amount;
    }

    // —— 库存落点：委托库存体系出库服务（原版 record_type 25 = 采购退货出库）——
    // 此前本链直接扣 product 平台库存（SKU 侧 GREATEST 静默钳 0）：门店库存
    // 不动、库存不足被掩盖、无出入库记录。现与入库同口径由库存服务按组织
    // 落点裁决（不足明确报错），记录 id 回填 purchase_return.stock_record_id。
    let stock_record_id = state
        .catalog
        .product_stock
        .stock_out(
            &StockInDTO {
                r#type: Some(stock_record_type::PURCHASE_RETURN_OUT),
                comment: Some(format!("采购退货 {return_sn}")),
                product_stock_record_detail_list: lines
                    .iter()
                    .map(|l| StockRecordDetailItem {
                        record_detail_id: None,
                        stock_record_id: None,
                        product_id: Some(l.product_id),
                        sku_id: if l.sku_id > 0 { Some(l.sku_id) } else { None },
                        quantity: Some(l.qty as i32),
                    })
                    .collect(),
                product_total: Some(total_qty as i32),
            },
            &StockOperator::new(shop_id, Some(auth.username.clone())),
        )
        .await?;

    // —— 单据事务：退货单 + 明细 + 入库行已退量 ——
    let mut tx = state.repos.pool.begin().await.map_err(db)?;
    let return_id: i64 = sqlx::query_scalar(
        "INSERT INTO purchase_return(return_sn,order_id,receipt_id,shop_id,vendor_id,business_date,status,total_quantity,total_amount,stock_record_id,reason,remark,confirm_time,version,create_time,update_time) VALUES($1,$2,$3,$4,$5,$6,2,$7,$8,$9,$10,$11,$12,0,$13,$14) RETURNING id",
    )
    .bind(&return_sn).bind(order_id).bind(receipt_id).bind(shop_id).bind(vendor_id)
    .bind(body.get("businessDate").and_then(Value::as_i64).unwrap_or(now))
    .bind(total_qty).bind(total_amount).bind(stock_record_id)
    .bind(body.get("reason").and_then(Value::as_str).unwrap_or(""))
    .bind(body.get("remark").and_then(Value::as_str).unwrap_or(""))
    .bind(now).bind(now).bind(now)
    .fetch_one(&mut *tx).await.map_err(db)?;
    for line in &lines {
        sqlx::query(
            "INSERT INTO purchase_return_item(return_id,receipt_item_id,product_id,sku_id,product_name,sku_name,product_sn,sku_sn,pic_thumb,product_unit,quantity,returnable_quantity,available_quantity,returnable_quantity_after,original_price,return_price,amount,reason,create_time,update_time) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20)",
        )
        .bind(return_id).bind(line.receipt_item_id).bind(line.product_id).bind(line.sku_id)
        .bind(&line.product_name)
        .bind(&line.sku_name)
        .bind(&line.product_sn)
        .bind(&line.sku_sn)
        .bind(&line.pic_thumb)
        .bind(&line.product_unit)
        .bind(line.qty as i32)
        .bind(line.received_qty)
        .bind(line.received_qty - line.returned_qty)
        .bind(line.received_qty - line.returned_qty - line.qty as i32)
        .bind(line.original_price).bind(line.return_price).bind(line.amount)
        .bind(&line.reason)
        .bind(now).bind(now)
        .execute(&mut *tx).await.map_err(db)?;
        sqlx::query(
            "UPDATE purchase_receipt_item SET returned_quantity=returned_quantity+$1 WHERE id=$2",
        )
        .bind(line.qty as i32)
        .bind(line.receipt_item_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    }
    tx.commit().await.map_err(db)?;
    ok(return_id)
}

async fn return_confirm(State(state): State<AdminState>, Json(body): Json<Value>) -> ApiResult<()> {
    // 我方修复（方向性缺陷）：历史实现把退货单状态置为 1（待确认）——
    // 「确认」动作反而把已确认（2）的退货打回待确认，使其脱离对账候选
    // 且与已扣库存的事实不一致。现口径：仅 待确认(1) → 已确认(2) 的条件
    // 更新；本系统退货由 return_save 保存即确认（status=2），对已确认单
    // 调用本端点为无操作成功（幂等），不存在的单据明确报错。
    let id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    let row = sqlx::query(
        "SELECT status FROM purchase_return WHERE id=$1 AND is_del=0 LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&state.repos.pool)
    .await
    .map_err(db)?
    .ok_or_else(|| AppError(Error::biz(400, "采购退货单不存在")))?;
    let status: i32 = row.try_get("status").unwrap_or(0);
    if status == 1 {
        let now = nextrade_common::utils::now_seconds() as i64;
        sqlx::query(
            "UPDATE purchase_return SET status=2, confirm_time=COALESCE(confirm_time,$1), update_time=$1 WHERE id=$2 AND status=1",
        )
        .bind(now)
        .bind(id)
        .execute(&state.repos.pool)
        .await
        .map_err(db)?;
    }
    ok(())
}
async fn return_delete(State(state): State<AdminState>, Json(body): Json<Value>) -> ApiResult<()> {
    // 我方修复：历史实现对任意状态直接软删 —— 已确认退货（status=2）已扣
    // 库存、已增入库行 returned_quantity，删除后两处均不回滚，库存与可退
    // 量永久失真。现仅待确认（status=1，未产生库存事实）可删；已确认退货
    // 如需冲销应走新的退货/对账口径，不在此开洞。前端未调用本端点。
    let id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    let row = sqlx::query(
        "SELECT status FROM purchase_return WHERE id=$1 AND is_del=0 LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&state.repos.pool)
    .await
    .map_err(db)?
    .ok_or_else(|| AppError(Error::biz(400, "采购退货单不存在")))?;
    let status: i32 = row.try_get("status").unwrap_or(0);
    if status != 1 {
        return Err(AppError(Error::biz(400, "已确认的采购退货单不可删除")));
    }
    soft_delete(&state, "purchase_return", id).await?;
    ok(())
}

// ————————————————— setting —————————————————
async fn setting_detail(State(state): State<AdminState>) -> ApiResult<Value> {
    let row = sqlx::query("SELECT * FROM purchase_setting WHERE is_del=0 ORDER BY id DESC LIMIT 1")
        .fetch_optional(&state.repos.pool)
        .await
        .map_err(db)?;
    ok(row.as_ref().map(row_to_json).unwrap_or(Value::Null))
}
async fn setting_save(State(state): State<AdminState>, Json(body): Json<Value>) -> ApiResult<()> {
    let now = nextrade_common::utils::now_seconds() as i64;
    sqlx::query("INSERT INTO purchase_setting(shop_id,auto_finish_days,allow_over_receive,create_time,update_time) VALUES($1,$2,$3,$4,$5) ON CONFLICT (shop_id) DO UPDATE SET auto_finish_days=EXCLUDED.auto_finish_days,allow_over_receive=EXCLUDED.allow_over_receive,update_time=EXCLUDED.update_time")
        .bind(body.get("shopId").and_then(Value::as_i64).unwrap_or(0) as i32)
        .bind(body.get("autoFinishDays").and_then(Value::as_i64).unwrap_or(0) as i32)
        .bind(body.get("allowOverReceive").and_then(Value::as_i64).unwrap_or(0) as i32)
        .bind(now).bind(now)
        .execute(&state.repos.pool).await.map_err(db)?;
    ok(())
}

// ————————————————— 采购单草稿（我方新增）口径测试 —————————————————
#[cfg(test)]
mod purchase_draft_tests {
    use super::*;

    fn input(sku_id: i64, qty: i64) -> OrderItemInput {
        OrderItemInput {
            sku_id,
            qty,
            price: rust_decimal::Decimal::ZERO,
        }
    }

    #[test]
    fn formal_pairs_require_at_least_one_item() {
        assert_eq!(validate_formal_pairs(&[]), Err("请选择采购商品"));
    }

    #[test]
    fn formal_pairs_reject_bad_sku_or_qty() {
        assert_eq!(
            validate_formal_pairs(&[(0, 3)]),
            Err("采购商品或数量不合法")
        );
        assert_eq!(
            validate_formal_pairs(&[(9, 0)]),
            Err("采购商品或数量不合法")
        );
        assert_eq!(
            validate_formal_pairs(&[(9, -2)]),
            Err("采购商品或数量不合法")
        );
        // 草稿里按 0 暂存的数量行在提交时必须被全量校验拦下
        assert_eq!(
            validate_formal_pairs(&[(9, 5), (8, 0)]),
            Err("采购商品或数量不合法")
        );
    }

    #[test]
    fn formal_pairs_accept_valid_items() {
        assert_eq!(validate_formal_pairs(&[(9, 5)]), Ok(()));
        assert_eq!(validate_formal_pairs(&[(9, 5), (8, 1)]), Ok(()));
    }

    #[test]
    fn draft_items_drop_unselected_rows_and_clamp_qty() {
        let kept = draft_items(&[input(0, 3), input(9, 0), input(8, -4), input(7, 6)]);
        // 未选规格的空行（sku=0）不落库；数量未填（0）与负数都按 0 暂存
        let pairs: Vec<(i64, i64)> = kept.iter().map(|i| (i.sku_id, i.qty)).collect();
        assert_eq!(pairs, vec![(9, 0), (8, 0), (7, 6)]);
    }

    #[test]
    fn draft_isolation_invariants_hold_in_source() {
        // 隔离守卫不变量（源码级断言，防后续改动悄悄拆守卫）。
        // 针串用 concat! 拼装，避免断言行自身的字面量让 contains 恒真。
        let src = include_str!("supply_chain.rs");
        // 1) 入库 save 对草稿有显式拦截，且正式状态校验（仅 1/2 可入库）仍在
        assert!(src.contains(concat!("草稿采购单不能入库", "，请先提交")));
        assert!(src.contains(concat!("status != 1 &&", " status != 2")));
        // 2) 提交/删除都只允许作用于草稿
        assert!(src.contains(concat!("只有草稿采购单", "可以提交")));
        assert!(src.contains(concat!("只有草稿采购单", "可以删除")));
        // 3) create 的状态绑定：草稿落 0、正式落 1
        assert!(src.contains(concat!("if draft { 0 }", " else { 1 }")));
    }

    // ——— 门店采购全流程批（帖 777863）口径测试 ———

    #[test]
    fn decimal_flex_accepts_string_and_number() {
        use rust_decimal::Decimal;
        assert_eq!(
            parse_decimal_flex(Some(&json!("12.30"))),
            Decimal::from_str("12.30").unwrap()
        );
        assert_eq!(
            parse_decimal_flex(Some(&json!(12.3))),
            Decimal::from_str("12.3").unwrap()
        );
        assert_eq!(parse_decimal_flex(Some(&json!(7))), Decimal::from(7));
        assert_eq!(parse_decimal_flex(Some(&json!("abc"))), Decimal::ZERO);
        assert_eq!(parse_decimal_flex(Some(&json!(""))), Decimal::ZERO);
        assert_eq!(parse_decimal_flex(None), Decimal::ZERO);
        // parse_order_items：数字 purchasePrice 不再恒落 0（历史缺陷）
        let items = parse_order_items(&json!({
            "items": [{"vendorProductSkuId": 9, "quantity": 2, "purchasePrice": 3.5}]
        }));
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].price, Decimal::from_str("3.5").unwrap());
    }

    #[test]
    fn procure_flow_invariants_hold_in_source() {
        // 全流程守卫不变量（源码级断言，同 draft_isolation 手法）。
        let src = include_str!("supply_chain.rs");
        // 1) 发货环：端点已注册，且入库放行已发货状态（5）
        assert!(src.contains(concat!("purchaseOrder", "/ship")));
        assert!(src.contains(concat!("status != 1 && status != 2 &&", " status != 5")));
        // 2) 库存落点：入库/退货分别委托库存服务的采购入库/采购退货出库类型
        assert!(src.contains(concat!("stock_record_type::", "PURCHASE_IN")));
        assert!(src.contains(concat!("stock_record_type::", "PURCHASE_RETURN_OUT")));
        // 3) 对账占用：候选排除已占用 + 保存时占用校验文案在位
        assert!(src.contains(concat!(
            "NOT EXISTS (SELECT 1 FROM purchase_reconciliation",
            "_source s"
        )));
        assert!(src.contains(concat!("对账来源单据已被其他对账单", "占用")));
        // 4) 退货确认方向已纠正为 待确认(1) → 已确认(2) 的条件更新
        assert!(src.contains(concat!(
            "purchase_return SET status=2",
            ", confirm_time=COALESCE"
        )));
        // 5) 超收设置消费点在位
        assert!(src.contains(concat!("load_allow_over", "_receive")));
    }
}
