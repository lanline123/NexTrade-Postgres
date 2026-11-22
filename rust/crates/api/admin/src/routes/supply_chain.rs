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
        .route("/supplyChain/supplierProduct/selection", get(product_selection))
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
        // payable（采购结算：应付 + 付款登记台账，我方新增）
        .route("/supplyChain/purchasePayable/list", get(payable_list))
        .route("/supplyChain/purchasePayable/detail", get(payable_detail))
        .route(
            "/supplyChain/purchasePayable/payment/create",
            post(payment_create),
        )
        .route(
            "/supplyChain/purchasePayable/payment/delete",
            post(payment_delete),
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
        ("GET", "/api/supplyChain/purchasePayable/list"),
        ("GET", "/api/supplyChain/purchasePayable/detail"),
        ("POST", "/api/supplyChain/purchasePayable/payment/create"),
        ("POST", "/api/supplyChain/purchasePayable/payment/delete"),
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
    /// 采购订单物流单号搜索（C-12 后端侧，site-audit-backend2）。
    tracking_no: Option<String>,
    /// 对账单号搜索（C-12 后端侧；与 keyword 等效，兼容旧前端参数名）。
    reconciliation_sn: Option<String>,
}

fn page_of(q: &Pg) -> (u64, u64, i64) {
    let page = q.page.unwrap_or(1).max(1);
    let size = q.size.unwrap_or(15).clamp(1, 200);
    (page, size, ((page - 1) * size) as i64)
}

/// 列表过滤列按表分派（模式实证，00-pure.sql + PG 对齐脚本）：各表列名
/// 并不一致 —— `vendor_product` 的状态列是 `product_state`、无 `status`
/// 与 `category_name`；采购单据四表有 `status` 但无 `category_name`。
/// 历史实现对一切非 vendor 表硬编码 `status` / `category_name`，带
/// status 或 keyword 的列表请求会拼出不存在的列直接 500（选品弹窗恒
/// 传 status=1、退货/对账列表带 keyword 搜索即触发）。现按表白名单
/// 返回 (状态列, 关键词列集合)，列名均为表内实证存在的列；关键词列
/// 与前端各列表搜索框文案对齐（单号/名称类）。
fn list_filter_columns(table: &str) -> (&'static str, &'static [&'static str]) {
    match table {
        "vendor" => ("status", &["vendor_name"]),
        "purchase_supplier_category" => ("status", &["category_name"]),
        "vendor_product" => ("product_state", &["product_name", "product_sn"]),
        "purchase_order" => ("status", &["order_sn", "vendor_name"]),
        "purchase_receipt" => ("status", &["receipt_sn"]),
        "purchase_return" => ("status", &["return_sn"]),
        "purchase_reconciliation" => ("status", &["reconciliation_sn"]),
        _ => ("status", &[]),
    }
}

async fn list_table(
    state: &AdminState,
    table: &str,
    q: &Pg,
    supplier_only: bool,
) -> Result<Value, AppError> {
    let (page, size, offset) = page_of(q);
    let (status_col, keyword_cols) = list_filter_columns(table);
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
    // 供应商过滤（C-12 后端侧，site-audit-backend2）：此前 Pg 有 vendorId
    // 字段但 list_table 从不消费，供应商商品/采购单据按供应商筛选无效。
    // 仅对确有 vendor_id 列的表生效。
    if matches!(
        table,
        "vendor_product"
            | "purchase_order"
            | "purchase_receipt"
            | "purchase_return"
            | "purchase_reconciliation"
    ) {
        if let Some(vendor_id) = q.vendor_id {
            cq.push(" AND vendor_id = ").push_bind(vendor_id);
            rq.push(" AND vendor_id = ").push_bind(vendor_id);
        }
    }
    if let Some(status) = q.status {
        cq.push(format!(" AND {status_col} = ")).push_bind(status);
        rq.push(format!(" AND {status_col} = ")).push_bind(status);
    }
    // 采购订单物流单号搜索（C-12 后端侧，site-audit-backend2）。
    if table == "purchase_order" {
        if let Some(tn) = q
            .tracking_no
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let like = format!("%{tn}%");
            for qb in [&mut cq, &mut rq] {
                qb.push(" AND tracking_no ILIKE ").push_bind(like.clone());
            }
        }
    }
    // 对账列表兼容 reconciliationSn 参数名（与 keyword 等效）。
    let kw_owned: Option<String> = q
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            if table == "purchase_reconciliation" {
                q.reconciliation_sn
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(ToString::to_string)
            } else {
                None
            }
        });
    if let Some(kw) = kw_owned.as_deref() {
        // 关键词子句按表定制（C-12 后端侧，site-audit-backend2）：入库单
        // 可按采购单号/供应商名搜（本表无此二列，走子查询）；退货单可按
        // 入库单号搜；其余表走 list_filter_columns 的本表列集合。
        let clauses: Vec<String> = match table {
            "purchase_receipt" => vec![
                "receipt_sn ILIKE ".to_string(),
                "order_id IN (SELECT id FROM purchase_order WHERE order_sn ILIKE ".to_string(),
                "vendor_id IN (SELECT vendor_id FROM vendor WHERE vendor_name ILIKE ".to_string(),
            ],
            "purchase_return" => vec![
                "return_sn ILIKE ".to_string(),
                "receipt_id IN (SELECT id FROM purchase_receipt WHERE receipt_sn ILIKE ".to_string(),
            ],
            _ => keyword_cols
                .iter()
                .map(|col| format!("{col} ILIKE "))
                .collect(),
        };
        if !clauses.is_empty() {
            let like = format!("%{kw}%");
            for qb in [&mut cq, &mut rq] {
                qb.push(" AND (");
                for (i, clause) in clauses.iter().enumerate() {
                    if i > 0 {
                        qb.push(" OR ");
                    }
                    qb.push(clause.clone()).push_bind(like.clone());
                    if clause.contains(" IN (SELECT") {
                        qb.push(")");
                    }
                }
                qb.push(")");
            }
        }
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
    let mut records: Vec<Value> = rows.iter().map(row_to_json).collect();
    // 采购四单据补驼峰别名层（C-19，site-audit-backend2）。
    if PURCHASE_DOC_TABLES.contains(&table) {
        for record in records.iter_mut() {
            add_camel_aliases(record);
        }
    }
    Ok(json!({"records": records, "total": total, "page": page, "size": size}))
}

/// 采购四单据（C-19）：list/detail 出参在蛇形原键之外补驼峰别名，
/// 照 supplier_list 的手工别名范式；蛇形键保留不动，前端旧读法不受影响。
const PURCHASE_DOC_TABLES: [&str; 4] = [
    "purchase_order",
    "purchase_receipt",
    "purchase_return",
    "purchase_reconciliation",
];

/// 值为 Unix 秒的列：驼峰别名按 supplier createTime 范式格式化为
/// `yyyy-MM-dd HH:mm:ss` 字符串（蛇形原键仍保留原始数值）。
const EPOCH_SECOND_COLS: [&str; 7] = [
    "create_time",
    "update_time",
    "confirm_time",
    "finish_time",
    "business_date",
    "review_time",
    "expected_delivery_date",
];

fn snake_to_camel(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut upper_next = false;
    for ch in name.chars() {
        if ch == '_' {
            upper_next = true;
        } else if upper_next {
            out.extend(ch.to_uppercase());
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// 给一行 JSON 补驼峰别名（已存在同名键不覆盖）。
fn add_camel_aliases(v: &mut Value) {
    let Some(obj) = v.as_object_mut() else {
        return;
    };
    let keys: Vec<String> = obj.keys().cloned().collect();
    for key in keys {
        if !key.contains('_') {
            continue;
        }
        let camel = snake_to_camel(&key);
        if camel == key || obj.contains_key(&camel) {
            continue;
        }
        let mut val = obj.get(&key).cloned().unwrap_or(Value::Null);
        if EPOCH_SECOND_COLS.contains(&key.as_str()) {
            if let Some(secs) = val.as_i64().filter(|s| *s > 0) {
                val = json!(format_unix_seconds(secs));
            }
        }
        obj.insert(camel, val);
    }
}

/// 给 detail 的主行与明细数组逐行补驼峰别名（C-19）。
fn alias_detail_with_items(v: &mut Value, item_keys: &[&str]) {
    add_camel_aliases(v);
    for key in item_keys {
        if let Some(arr) = v.get_mut(*key).and_then(Value::as_array_mut) {
            for row in arr.iter_mut() {
                add_camel_aliases(row);
            }
        }
    }
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

/// 供应商表单（admin SupplierFormDrawer 既有契约）扩展字段的落库口径：
/// 分类落 `purchase_category_id` 列（表单字段名 `categoryId`，兼容旧名
/// `purchaseCategoryId`）；联系人/电话组/资质/地址/备注/结算账户进
/// `vendor_data` JSON——与商家域 vendor_data 的 `contactName` 键同义共存，
/// 合并写入不覆盖既有键；主电话（primary，缺省取首个）同步落
/// `contact_mobile` 列（采购单快照等存量消费方仍读该列）。
/// `person_data` 为个人主体信息，供应链表单无对应入参，不写入。
struct SupplierFormExtract {
    /// 主电话（contact_mobile 落值）；表单未传电话时为 None（更新时不覆盖）。
    primary_mobile: Option<String>,
    /// 待合并进 vendor_data 的键（仅含表单实际传来的键）。
    data_patch: Map<String, Value>,
}

fn extract_supplier_form(body: &Value) -> SupplierFormExtract {
    let mut patch = Map::new();
    let mut primary_mobile: Option<String> = None;
    if let Some(phones) = body.get("contactPhones").and_then(Value::as_array) {
        let normalized: Vec<Value> = phones
            .iter()
            .filter_map(|p| {
                let number = p
                    .get("number")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim();
                if number.is_empty() {
                    return None;
                }
                let mut row = Map::new();
                row.insert(
                    "type".into(),
                    Value::String(
                        p.get("type")
                            .and_then(Value::as_str)
                            .unwrap_or("mobile")
                            .to_string(),
                    ),
                );
                row.insert("number".into(), Value::String(number.to_string()));
                if let Some(area) = p
                    .get("areaCode")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    row.insert("areaCode".into(), Value::String(area.to_string()));
                }
                if p.get("primary").and_then(Value::as_bool).unwrap_or(false) {
                    row.insert("primary".into(), Value::Bool(true));
                }
                Some(Value::Object(row))
            })
            .collect();
        let primary = normalized
            .iter()
            .find(|p| {
                p.get("primary")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            })
            .or_else(|| normalized.first());
        primary_mobile = primary
            .and_then(|p| p.get("number"))
            .and_then(Value::as_str)
            .map(ToString::to_string);
        patch.insert("contactPhones".into(), Value::Array(normalized));
    } else if let Some(mobile) = body
        .get("contactMobile")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        primary_mobile = Some(mobile.to_string());
    }
    for key in [
        "contactName",
        "detailedAddress",
        "remark",
        "address",
        "qualificationImages",
        "account",
    ] {
        if let Some(v) = body.get(key) {
            patch.insert(key.to_string(), v.clone());
        }
    }
    SupplierFormExtract {
        primary_mobile,
        data_patch: patch,
    }
}

/// 把 patch 合并进既有 vendor_data JSON 文本（非对象/非法 JSON 按空对象处理）。
fn merge_vendor_data(existing: Option<&str>, patch: &Map<String, Value>) -> String {
    let mut base = parse_vendor_data(existing.map(|s| Value::String(s.to_string())).as_ref());
    for (k, v) in patch {
        base.insert(k.clone(), v.clone());
    }
    Value::Object(base).to_string()
}

/// 解析 vendor_data（行内为 JSON 文本字符串，兼容已解析对象）为对象。
fn parse_vendor_data(raw: Option<&Value>) -> Map<String, Value> {
    match raw {
        Some(Value::Object(m)) => m.clone(),
        Some(Value::String(s)) => serde_json::from_str::<Value>(s)
            .ok()
            .and_then(|v| match v {
                Value::Object(m) => Some(m),
                _ => None,
            })
            .unwrap_or_default(),
        _ => Map::new(),
    }
}

/// Unix 秒 → `yyyy-MM-dd HH:mm:ss`（UTC；列表 createTime 展示口径）。
fn format_unix_seconds(secs: i64) -> String {
    if secs <= 0 {
        return String::new();
    }
    sqlx::types::chrono::DateTime::<sqlx::types::chrono::Utc>::from_timestamp(secs, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

async fn supplier_list(State(state): State<AdminState>, Query(q): Query<Pg>) -> ApiResult<Value> {
    let mut result = list_table(&state, "vendor", &q, true).await?;
    // 列表行补驼峰别名与展示字段（前端列契约 vendorName / categoryName /
    // contactName / createTime）：分类名按行内分类 ID 一次批量查，
    // 联系人自 vendor_data 解析；原始蛇形键保留不动。
    if let Some(records) = result
        .get_mut("records")
        .and_then(Value::as_array_mut)
    {
        let cat_ids: Vec<i64> = records
            .iter()
            .filter_map(|r| r.get("purchase_category_id").and_then(Value::as_i64))
            .filter(|v| *v > 0)
            .collect();
        let mut cat_names: std::collections::HashMap<i64, String> =
            std::collections::HashMap::new();
        if !cat_ids.is_empty() {
            if let Ok(rows) =
                sqlx::query("SELECT id, category_name FROM purchase_supplier_category WHERE id = ANY($1)")
                    .bind(cat_ids.as_slice())
                    .fetch_all(&state.repos.pool)
                    .await
            {
                for row in &rows {
                    cat_names.insert(
                        row.try_get::<i64, _>("id").unwrap_or(0),
                        row.try_get::<String, _>("category_name")
                            .unwrap_or_default(),
                    );
                }
            }
        }
        for record in records.iter_mut() {
            let Some(obj) = record.as_object_mut() else {
                continue;
            };
            let vendor_id = obj.get("vendor_id").cloned().unwrap_or(Value::Null);
            let vendor_name = obj.get("vendor_name").cloned().unwrap_or(Value::Null);
            let vendor_code = obj.get("vendor_code").cloned().unwrap_or(Value::Null);
            let cat_id = obj
                .get("purchase_category_id")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let data = parse_vendor_data(obj.get("vendor_data"));
            let contact_name = data
                .get("contactName")
                .cloned()
                .unwrap_or_else(|| json!(""));
            let add_time = obj.get("add_time").and_then(Value::as_i64).unwrap_or(0);
            obj.insert("vendorId".into(), vendor_id);
            obj.insert("vendorName".into(), vendor_name);
            obj.insert("vendorCode".into(), vendor_code);
            obj.insert(
                "categoryName".into(),
                json!(cat_names.get(&cat_id).cloned().unwrap_or_default()),
            );
            obj.insert("contactName".into(), contact_name);
            obj.insert("createTime".into(), json!(format_unix_seconds(add_time)));
        }
    }
    ok(result)
}
async fn supplier_detail(State(state): State<AdminState>, Query(q): Query<Pg>) -> ApiResult<Value> {
    let mut detail = detail_table(&state, "vendor", q.id.unwrap_or(0)).await?;
    // 编辑回显组装：原始蛇形键保留，叠加表单契约的驼峰键——分类自
    // purchase_category_id 列，其余扩展字段自 vendor_data JSON 回读。
    if let Some(obj) = detail.as_object_mut() {
        let data = parse_vendor_data(obj.get("vendor_data"));
        let vendor_id = obj.get("vendor_id").cloned().unwrap_or(Value::Null);
        let shop_id = obj.get("shop_id").cloned().unwrap_or(Value::Null);
        let vendor_name = obj.get("vendor_name").cloned().unwrap_or(Value::Null);
        let vendor_code = obj.get("vendor_code").cloned().unwrap_or(Value::Null);
        let category_id = obj
            .get("purchase_category_id")
            .cloned()
            .unwrap_or(Value::Null);
        let contact_mobile = obj.get("contact_mobile").cloned().unwrap_or(Value::Null);
        let vendor_logo = obj.get("vendor_logo").cloned().unwrap_or(Value::Null);
        obj.insert("vendorId".into(), vendor_id);
        obj.insert("shopId".into(), shop_id);
        obj.insert("vendorName".into(), vendor_name);
        obj.insert("vendorCode".into(), vendor_code);
        obj.insert("categoryId".into(), category_id);
        obj.insert("contactMobile".into(), contact_mobile);
        obj.insert("vendorLogo".into(), vendor_logo);
        for key in [
            "contactName",
            "contactPhones",
            "qualificationImages",
            "address",
            "detailedAddress",
            "remark",
            "account",
        ] {
            if let Some(v) = data.get(key) {
                obj.insert(key.to_string(), v.clone());
            }
        }
        // 存量行（旧实现只落 contact_mobile）合成单条主电话供表单回显。
        let phones_empty = obj
            .get("contactPhones")
            .and_then(Value::as_array)
            .map_or(true, |a| a.is_empty());
        if phones_empty {
            if let Some(mobile) = obj
                .get("contact_mobile")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
            {
                obj.insert(
                    "contactPhones".into(),
                    json!([{"type": "mobile", "number": mobile, "primary": true}]),
                );
            }
        }
        // 表单必读键缺省兜底，免前端逐个判空。
        obj.entry("contactName".to_string())
            .or_insert_with(|| json!(""));
        obj.entry("contactPhones".to_string())
            .or_insert_with(|| json!([]));
        obj.entry("qualificationImages".to_string())
            .or_insert_with(|| json!([]));
        obj.entry("address".to_string())
            .or_insert_with(|| json!([]));
        obj.entry("detailedAddress".to_string())
            .or_insert_with(|| json!(""));
        obj.entry("remark".to_string()).or_insert_with(|| json!(""));
        obj.entry("account".to_string())
            .or_insert_with(|| json!({}));
    }
    ok(detail)
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
    let extract = extract_supplier_form(&body);
    let vendor_data = merge_vendor_data(None, &extract.data_patch);
    let now = nextrade_common::utils::now_seconds() as i64;
    let r: i32 = sqlx::query_scalar(
        "INSERT INTO vendor(source_type, shop_id, vendor_code, vendor_name, contact_mobile, purchase_category_id, vendor_logo, description, status, add_time, vendor_data) VALUES(2,$1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING vendor_id",
    )
    .bind(body.get("shopId").and_then(Value::as_i64).unwrap_or(0) as i32)
    .bind(body.get("vendorCode").and_then(Value::as_str).unwrap_or(""))
    .bind(name)
    .bind(extract.primary_mobile.as_deref().unwrap_or(""))
    .bind(body.get("categoryId").or_else(|| body.get("purchaseCategoryId")).and_then(Value::as_i64).unwrap_or(0))
    .bind(body.get("vendorLogo").and_then(Value::as_str).unwrap_or(""))
    .bind(body.get("description").and_then(Value::as_str).unwrap_or(""))
    .bind(body.get("status").and_then(Value::as_i64).unwrap_or(1) as i32)
    .bind(now)
    .bind(vendor_data)
    .fetch_one(&state.repos.pool).await.map_err(db)?;
    ok(i64::from(r))
}
async fn supplier_update(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<()> {
    // 表单契约传 id（兼容 vendorId）；未传的字段一律不覆盖（COALESCE 保旧值），
    // 扩展字段合并进既有 vendor_data，不冲掉商家域写入的键。
    let id = body
        .get("id")
        .or_else(|| body.get("vendorId"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let existing_row = sqlx::query(
        "SELECT vendor_data FROM vendor WHERE vendor_id=$1 AND source_type=2",
    )
    .bind(id)
    .fetch_optional(&state.repos.pool)
    .await
    .map_err(db)?;
    let existing: Option<String> = existing_row
        .and_then(|row| row.try_get::<Option<String>, _>("vendor_data").ok())
        .flatten();
    let extract = extract_supplier_form(&body);
    let vendor_data = if extract.data_patch.is_empty() {
        None
    } else {
        Some(merge_vendor_data(existing.as_deref(), &extract.data_patch))
    };
    let text_field = |key: &str| {
        body.get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
    };
    let result = sqlx::query("UPDATE vendor SET vendor_name=COALESCE($1,vendor_name), vendor_code=COALESCE($2,vendor_code), purchase_category_id=COALESCE($3,purchase_category_id), contact_mobile=COALESCE($4,contact_mobile), status=COALESCE($5,status), vendor_logo=COALESCE($6,vendor_logo), vendor_data=COALESCE($7,vendor_data) WHERE vendor_id=$8 AND source_type=2")
        .bind(text_field("vendorName"))
        .bind(text_field("vendorCode"))
        .bind(body.get("categoryId").or_else(|| body.get("purchaseCategoryId")).and_then(Value::as_i64))
        .bind(extract.primary_mobile.as_deref())
        .bind(body.get("status").and_then(Value::as_i64).map(|v| v as i32))
        .bind(text_field("vendorLogo"))
        .bind(vendor_data.as_deref())
        .bind(id)
        .execute(&state.repos.pool).await.map_err(db)?;
    if result.rows_affected() == 0 {
        return Err(AppError(Error::biz(400, "供应商不存在")));
    }
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
/// 建单选品专用查询（采购单「添加商品」弹窗契约）：按 vendor 过滤、
/// SKU 级行、驼峰字段。历史实现复用通用列表（vendor_product 关系行
/// 原始蛇形列、无 vendor 过滤、无 SKU 展开），前端勾选后建单必被拒。
/// status 按 vendor_product.product_state 过滤；shopId 给定时再按
/// 供应商所属组织收口；keyword 按商品名模糊。
async fn product_selection(State(state): State<AdminState>, Query(q): Query<Pg>) -> ApiResult<Value> {
    let (page, size, offset) = page_of(&q);
    let vendor_id = q.vendor_id.unwrap_or(0);
    if vendor_id <= 0 {
        return ok(json!({"records": [], "total": 0, "page": page, "size": size}));
    }
    let mut cq: QueryBuilder<'_, Postgres> = QueryBuilder::new(
        "SELECT COUNT(*) FROM vendor_product_sku s JOIN vendor_product vp ON vp.id = s.vendor_product_id WHERE s.is_del = 0 AND vp.is_del = 0 AND vp.vendor_id = ",
    );
    cq.push_bind(vendor_id);
    let mut rq: QueryBuilder<'_, Postgres> = QueryBuilder::new(
        "SELECT s.id AS sku_row_id, s.source_sku_id, vp.source_product_id, vp.product_name, vp.pic_thumb, s.sku_attr_val, s.sku_stock, s.min_purchase_qty, s.supply_price, c.category_name FROM vendor_product_sku s JOIN vendor_product vp ON vp.id = s.vendor_product_id LEFT JOIN category c ON c.category_id = vp.product_category_id WHERE s.is_del = 0 AND vp.is_del = 0 AND vp.vendor_id = ",
    );
    rq.push_bind(vendor_id);
    if let Some(status) = q.status {
        cq.push(" AND vp.product_state = ").push_bind(status);
        rq.push(" AND vp.product_state = ").push_bind(status);
    }
    if let Some(shop_id) = q.shop_id.filter(|v| *v > 0) {
        cq.push(" AND vp.vendor_id IN (SELECT vendor_id FROM vendor WHERE shop_id = ")
            .push_bind(shop_id)
            .push(" AND source_type = 2)");
        rq.push(" AND vp.vendor_id IN (SELECT vendor_id FROM vendor WHERE shop_id = ")
            .push_bind(shop_id)
            .push(" AND source_type = 2)");
    }
    if let Some(kw) = q
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let like = format!("%{kw}%");
        cq.push(" AND vp.product_name ILIKE ").push_bind(like.clone());
        rq.push(" AND vp.product_name ILIKE ").push_bind(like);
    }
    rq.push(" ORDER BY s.id DESC LIMIT ")
        .push_bind(size as i64)
        .push(" OFFSET ")
        .push_bind(offset);
    let (total,): (i64,) = cq
        .build_query_as()
        .fetch_one(&state.repos.pool)
        .await
        .map_err(db)?;
    let rows = rq.build().fetch_all(&state.repos.pool).await.map_err(db)?;
    let records: Vec<Value> = rows
        .iter()
        .map(|row| {
            json!({
                "vendorProductSkuId": row.try_get::<i64, _>("sku_row_id").unwrap_or(0),
                "skuId": row.try_get::<Option<i32>, _>("source_sku_id").ok().flatten().unwrap_or(0),
                "productId": row.try_get::<i32, _>("source_product_id").unwrap_or(0),
                "productName": row.try_get::<String, _>("product_name").unwrap_or_default(),
                "skuName": row.try_get::<String, _>("sku_attr_val").unwrap_or_default(),
                "categoryName": row.try_get::<Option<String>, _>("category_name").ok().flatten().unwrap_or_default(),
                "picThumb": row.try_get::<Option<String>, _>("pic_thumb").ok().flatten().unwrap_or_default(),
                "supplierStock": row.try_get::<Option<i32>, _>("sku_stock").ok().flatten().unwrap_or(0),
                "minPurchaseQty": row.try_get::<i64, _>("min_purchase_qty").unwrap_or(1),
                "supplyPrice": row.try_get::<rust_decimal::Decimal, _>("supply_price").map(|d| d.to_string()).unwrap_or_else(|_| "0".to_string()),
            })
        })
        .collect();
    ok(json!({"records": records, "total": total, "page": page, "size": size}))
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
        // 供应价走双口径解析：前端传 JSON 数字，历史实现只认字符串、
        // 取不到便恒落 0——与采购价（parse_decimal_flex）同类根因的漏网点。
        let price = parse_decimal_flex(sku.get("supplyPrice"));
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
    alias_detail_with_items(&mut order, &["items"]);
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
    status: i32,
}

async fn load_vendor_snapshot(
    state: &AdminState,
    vendor_id: i32,
) -> Result<VendorSnapshot, AppError> {
    let vendor = sqlx::query("SELECT vendor_name, vendor_code, contact_mobile, shop_id, status FROM vendor WHERE vendor_id=$1 AND source_type=2 LIMIT 1")
        .bind(vendor_id)
        .fetch_optional(&state.repos.pool).await.map_err(db)?
        .ok_or_else(|| AppError(Error::biz(400, "供应商不存在")))?;
    Ok(VendorSnapshot {
        vendor_name: vendor.try_get("vendor_name").unwrap_or_default(),
        vendor_code: vendor.try_get("vendor_code").unwrap_or_default(),
        vendor_contact: vendor.try_get("contact_mobile").unwrap_or_default(),
        shop_id: vendor.try_get("shop_id").unwrap_or(0),
        status: vendor.try_get("status").unwrap_or(0),
    })
}

/// 建单启用校验：历史实现只校验供应商存在，停用供应商经 API/旧页面
/// 数据仍可建单（save_supplier_product 有 status=1 校验、建单侧漏了）。
/// 正式建单/草稿转正/草稿提交三处共用；草稿暂存不拦（放宽校验口径）。
fn ensure_vendor_enabled(snapshot: &VendorSnapshot) -> Result<(), AppError> {
    if snapshot.status != 1 {
        return Err(AppError(Error::biz(400, "供应商不存在或已停用")));
    }
    Ok(())
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
    if !draft {
        if let Some(s) = &snapshot {
            ensure_vendor_enabled(s)?;
        }
    }
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
    let order_sn = unique_doc_sn(&state, "purchase_order", "order_sn", "PO", now).await?;
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
    if !target_draft {
        if let Some(s) = &snapshot {
            ensure_vendor_enabled(s)?;
        }
    }
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
    ensure_vendor_enabled(&snapshot)?;
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
    // 未命中不再静默成功（与 return_confirm 同口径）：单据不存在或状态
    // 不在可结束集合时明确报错，避免 UI 显示成功但状态未流转。
    let row = sqlx::query("SELECT status FROM purchase_order WHERE id=$1 AND is_del=0 LIMIT 1")
        .bind(id)
        .fetch_optional(&state.repos.pool)
        .await
        .map_err(db)?
        .ok_or_else(|| AppError(Error::biz(400, "采购订单不存在")))?;
    let status: i32 = row.try_get("status").unwrap_or(-1);
    if !matches!(status, 1 | 2 | 5) {
        return Err(AppError(Error::biz(
            400,
            "只有待入库、部分入库或已发货的采购单可以结束",
        )));
    }
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

/// 单据号生成（防同秒碰撞）：单号 = 前缀 + 秒级时间戳 + 4 位随机后缀，
/// 同秒同后缀概率约 1/10000，而四类单号列均有唯一索引，撞号即整单
/// 创建失败且历史实现无重试。现生成前先查存量占用，撞号换后缀重试
/// （最多 10 次），仍冲突则叠加一段后缀兜底；唯一索引仍是最终防线。
/// table / column 只接受本模块内部字面量，不拼入参。
async fn unique_doc_sn(
    state: &AdminState,
    table: &str,
    column: &str,
    prefix: &str,
    now: i64,
) -> Result<String, AppError> {
    for _ in 0..10 {
        let sn = format!("{prefix}{now}{:04}", rand_suffix());
        let (n,): (i64,) = sqlx::query_as(&format!(
            "SELECT COUNT(*) FROM {table} WHERE {column} = $1"
        ))
        .bind(&sn)
        .fetch_one(&state.repos.pool)
        .await
        .map_err(db)?;
        if n == 0 {
            return Ok(sn);
        }
    }
    Ok(format!(
        "{prefix}{now}{:04}{:04}",
        rand_suffix(),
        rand_suffix()
    ))
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
    alias_detail_with_items(&mut receipt, &["items"]);
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
    let now = nextrade_common::utils::now_seconds() as i64;
    let receipt_sn = unique_doc_sn(&state, "purchase_receipt", "receipt_sn", "PR", now).await?;

    // —— 单据事务（库存与单据同事务）：先锁采购单行，再校验与落库 ——
    // 历史结构：库存委托服务在单据事务之前逐行落库（自动提交），可收量
    // 预检在事务外走连接池无锁 —— ① 多行入库第 N 行落点报错时前 N-1 行
    // 库存已增、无入库单、无出入库记录，重试再入则前行重复计入；
    // ② 并发双发同一订单时双双通过预检，received_quantity 可超
    // ordered（超收关闭时亦然）、库存双记。现：订单行 FOR UPDATE 把
    // 同单收货串行化，校验在锁内按已提交状态复核；库存动作走
    // stock_in_tx 落在同一事务，任一步失败整体回滚、不留半成品。
    let mut tx = state.repos.pool.begin().await.map_err(db)?;
    let order = sqlx::query(
        "SELECT * FROM purchase_order WHERE id=$1 AND is_del=0 LIMIT 1 FOR UPDATE",
    )
    .bind(order_id)
    .fetch_optional(&mut *tx)
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
    let allow_over = load_allow_over_receive(&state, shop_id).await?;

    // —— 校验（锁内、无写入）：逐行核对明细归属与可收数量，收齐落库快照 ——
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
        .fetch_optional(&mut *tx)
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
    // 事务口径：走 stock_in_tx 落在本函数已开启的单据事务内（订单行锁
    // 已持有），库存增减/出入库记录/库存日志与入库单、订单进度同提交
    // 同回滚；落点在中途报错时整个收货回滚，不再留无单据的库存增量。
    let stock_record_id = state
        .catalog
        .product_stock
        .stock_in_tx(
            &mut *tx,
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

    // —— 单据落库（同事务）：入库单 + 明细 + 订单已收量/状态推进 ——
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
    alias_detail_with_items(&mut rec, &["sources", "items"]);
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

/// 对账导出（C-13 后端侧，site-audit-backend2）：此前返回 JSON 信封，
/// 前端存成无扩展名文件冒充 Excel。现真生成 xlsx 并带
/// Content-Disposition；过滤口径与列表一致（shopId/status/keyword，
/// keyword 与 reconciliationSn 均匹配对账单号）。
async fn reconciliation_export(
    State(state): State<AdminState>,
    Query(q): Query<Pg>,
) -> std::result::Result<axum::response::Response, AppError> {
    let mut qb: QueryBuilder<'_, Postgres> =
        QueryBuilder::new("SELECT * FROM purchase_reconciliation WHERE is_del=0");
    if let Some(shop_id) = q.shop_id {
        qb.push(" AND shop_id=").push_bind(shop_id);
    }
    if let Some(status) = q.status {
        qb.push(" AND status=").push_bind(status);
    }
    let kw = q
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            q.reconciliation_sn
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
        });
    if let Some(kw) = kw {
        qb.push(" AND reconciliation_sn ILIKE ")
            .push_bind(format!("%{kw}%"));
    }
    qb.push(" ORDER BY id DESC LIMIT 5000");
    let rows = qb.build().fetch_all(&state.repos.pool).await.map_err(db)?;
    let cell = |row: &Value, key: &str| -> String {
        match row.get(key) {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            _ => String::new(),
        }
    };
    let data: Vec<Vec<String>> = rows
        .iter()
        .map(row_to_json)
        .map(|row| {
            let status_name = match cell(&row, "status").as_str() {
                "1" => "待审核",
                "2" => "已通过",
                "3" => "已驳回",
                "4" => "已作废",
                _ => "",
            };
            let create_time = row
                .get("create_time")
                .and_then(Value::as_i64)
                .filter(|s| *s > 0)
                .map(format_unix_seconds)
                .unwrap_or_default();
            vec![
                cell(&row, "reconciliation_sn"),
                cell(&row, "vendor_id"),
                status_name.to_string(),
                cell(&row, "receipt_amount"),
                cell(&row, "return_amount"),
                cell(&row, "total_amount"),
                cell(&row, "remark"),
                cell(&row, "review_remark"),
                create_time,
            ]
        })
        .collect();
    let bytes = nextrade_services::system::excel::build_xlsx(
        &[
            "对账单号",
            "供应商ID",
            "状态",
            "入库金额",
            "退货金额",
            "对账金额",
            "备注",
            "审核备注",
            "创建时间",
        ],
        &data,
        "采购对账单",
    )
    .map_err(AppError)?;
    Ok(crate::support::excel_response::xlsx_response(
        bytes,
        "purchase_reconciliation.xlsx",
    ))
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
        let sn = unique_doc_sn(state, "purchase_reconciliation", "reconciliation_sn", "PC", now).await?;
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
        // 来源行加 FOR UPDATE 锁：占用校验在锁内执行，并发以同一来源
        // 建两张对账单时后到事务阻塞至先行提交，随后占用校验命中拒绝，
        // 不再双占同一来源（与 receipt/return 的行锁口径一致）。
        let row = if source_type == 1 {
            sqlx::query("SELECT id, receipt_sn, business_date, total_amount FROM purchase_receipt WHERE id=$1 AND vendor_id=$2 AND status=2 AND is_del=0 LIMIT 1 FOR UPDATE")
                .bind(source_id).bind(vendor_id).fetch_optional(&mut *tx).await.map_err(db)?
        } else if source_type == 2 {
            sqlx::query("SELECT id, return_sn AS receipt_sn, business_date, total_amount FROM purchase_return WHERE id=$1 AND vendor_id=$2 AND status=2 AND is_del=0 LIMIT 1 FOR UPDATE")
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
/// 对账状态流转前置校验（与 return_confirm 同口径）：单据必须存在，
/// 否则明确报错。历史实现（approve/reject/resubmit/void、order_finish）
/// 只跑条件 UPDATE、未命中行也返回成功，UI 显示操作成功但状态实际
/// 未流转，验收与审计易误判；现统一先查状态再做条件流转。
async fn load_reconciliation_status(state: &AdminState, id: i64) -> Result<i32, AppError> {
    let row = sqlx::query(
        "SELECT status FROM purchase_reconciliation WHERE id=$1 AND is_del=0 LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&state.repos.pool)
    .await
    .map_err(db)?
    .ok_or_else(|| AppError(Error::biz(400, "采购对账单不存在")))?;
    Ok(row.try_get("status").unwrap_or(-1))
}

async fn reconciliation_approve(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<()> {
    let id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    if load_reconciliation_status(&state, id).await? != 1 {
        return Err(AppError(Error::biz(
            400,
            "只有待审核的采购对账单可以审核通过",
        )));
    }
    sqlx::query("UPDATE purchase_reconciliation SET status=2, review_remark=$1, review_time=$2, update_time=$3 WHERE id=$4 AND status=1")
        .bind(body.get("remark").and_then(Value::as_str).unwrap_or(""))
        .bind(nextrade_common::utils::now_seconds() as i64)
        .bind(nextrade_common::utils::now_seconds() as i64)
        .bind(id).execute(&state.repos.pool).await.map_err(db)?;
    // 结算环节（我方新增）：对账通过即生成应付单（与对账单 1:1、幂等）。
    // ensure 内部按对账单 status=2 条件生成——本次更新未命中（单据本
    // 非待审核）时，已通过的存量单在此自愈补生成，其余状态不误生成。
    ensure_payable_for(&state, id).await?;
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
    if load_reconciliation_status(&state, id).await? != 1 {
        return Err(AppError(Error::biz(
            400,
            "只有待审核的采购对账单可以驳回",
        )));
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
    if load_reconciliation_status(&state, id).await? != 3 {
        return Err(AppError(Error::biz(
            400,
            "只有已驳回的采购对账单可以重新提交",
        )));
    }
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
    if load_reconciliation_status(&state, id).await? != 3 {
        return Err(AppError(Error::biz(
            400,
            "只有已驳回的采购对账单可以作废",
        )));
    }
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

// ————————————————— 采购结算：应付 + 付款登记（我方新增） —————————————————
// 背景：采购链此前（与原版一致）止于对账通过，无应付/付款单据
// （演练报告 D13）；用户 2026-11-21 拍板在对账后新增结算环。
// 红线：只做登记台账——不接任何真实支付通道、不动钱；不改变对账
// 及此前各环的任何既有口径。
// 口径：应付与对账单 1:1（迁移 038，UNIQUE reconciliation_id），
// 对账通过时生成（approve 钩子），存量已通过对账单由列表回填补齐；
// 应付金额 = 对账单 total_amount 快照；付款登记支持分次付款，
// 应付状态只由 derive_payable_status 按已付汇总推导落列，
// 任何接口都不接受外部传入 status / paidAmount。

/// 应付状态推导（纯函数，单测覆盖）：1 待付款 / 2 部分付款 / 3 已结清。
/// 应付额 ≤ 0（退货净额抵平或反超对账总额）时无付款义务，直接结清。
fn derive_payable_status(
    payable_amount: rust_decimal::Decimal,
    paid_amount: rust_decimal::Decimal,
) -> i32 {
    if payable_amount <= rust_decimal::Decimal::ZERO || paid_amount >= payable_amount {
        3
    } else if paid_amount <= rust_decimal::Decimal::ZERO {
        1
    } else {
        2
    }
}

/// 应付生成 SQL（单条与批量回填共用）：从已通过（status=2）对账单
/// 派生，应付号 = PS + 生成秒级时间戳 + 对账单 id 补零 6 位——
/// 同秒批量回填也不会撞号（uk_purchase_payable_sn 兜底）。
fn payable_insert_select(where_sql: &str) -> String {
    format!(
        "INSERT INTO purchase_payable(payable_sn, reconciliation_id, reconciliation_sn, shop_id, vendor_id, supplier_snapshot, payable_amount, paid_amount, status, create_time, update_time) \
         SELECT 'PS' || $1::text || LPAD(r.id::text, 6, '0'), r.id, r.reconciliation_sn, r.shop_id, r.vendor_id, r.supplier_snapshot, r.total_amount, 0, \
         CASE WHEN r.total_amount <= 0 THEN 3 ELSE 1 END, $1, $1 \
         FROM purchase_reconciliation r WHERE r.is_del = 0 AND r.status = 2 AND {where_sql} \
         ON CONFLICT (reconciliation_id) DO NOTHING"
    )
}

/// 对账通过 → 生成应付（幂等，已生成则无操作）。
async fn ensure_payable_for(state: &AdminState, reconciliation_id: i64) -> Result<(), AppError> {
    let now = nextrade_common::utils::now_seconds() as i64;
    sqlx::query(&payable_insert_select("r.id = $2"))
        .bind(now)
        .bind(reconciliation_id)
        .execute(&state.repos.pool)
        .await
        .map_err(db)?;
    Ok(())
}

/// 存量回填：本功能上线前已通过的对账单在应付列表打开时补生成。
async fn backfill_payables(state: &AdminState) -> Result<(), AppError> {
    let now = nextrade_common::utils::now_seconds() as i64;
    sqlx::query(&payable_insert_select("1 = 1"))
        .bind(now)
        .execute(&state.repos.pool)
        .await
        .map_err(db)?;
    Ok(())
}

fn payable_row_json(row: &PgRow) -> Value {
    let payable_amount: rust_decimal::Decimal =
        row.try_get("payable_amount").unwrap_or_default();
    let paid_amount: rust_decimal::Decimal = row.try_get("paid_amount").unwrap_or_default();
    json!({
        "id": row.try_get::<i64, _>("id").unwrap_or(0),
        "payableSn": row.try_get::<String, _>("payable_sn").unwrap_or_default(),
        "reconciliationId": row.try_get::<i64, _>("reconciliation_id").unwrap_or(0),
        "reconciliationSn": row.try_get::<String, _>("reconciliation_sn").unwrap_or_default(),
        "shopId": row.try_get::<i32, _>("shop_id").unwrap_or(0),
        "vendorId": row.try_get::<i32, _>("vendor_id").unwrap_or(0),
        "vendorName": row.try_get::<Option<String>, _>("vendor_name").ok().flatten().unwrap_or_default(),
        "payableAmount": payable_amount.to_string(),
        "paidAmount": paid_amount.to_string(),
        "unpaidAmount": (payable_amount - paid_amount).to_string(),
        // 展示状态按金额现算，与落列 status 同一推导函数，双保险防漂移
        "status": derive_payable_status(payable_amount, paid_amount),
        "remark": row.try_get::<Option<String>, _>("remark").ok().flatten().unwrap_or_default(),
        "createTime": row.try_get::<i64, _>("create_time").unwrap_or(0),
    })
}

fn payment_row_json(row: &PgRow) -> Value {
    json!({
        "id": row.try_get::<i64, _>("id").unwrap_or(0),
        "payableId": row.try_get::<i64, _>("payable_id").unwrap_or(0),
        "amount": row.try_get::<rust_decimal::Decimal, _>("amount").unwrap_or_default().to_string(),
        "payDate": row.try_get::<i64, _>("pay_date").unwrap_or(0),
        "payMethod": row.try_get::<String, _>("pay_method").unwrap_or_default(),
        "serialNo": row.try_get::<String, _>("serial_no").unwrap_or_default(),
        "voucherRemark": row.try_get::<String, _>("voucher_remark").unwrap_or_default(),
        "remark": row.try_get::<Option<String>, _>("remark").ok().flatten().unwrap_or_default(),
        "createTime": row.try_get::<i64, _>("create_time").unwrap_or(0),
        "createByName": row.try_get::<String, _>("create_by_name").unwrap_or_default(),
    })
}

async fn payable_list(
    State(state): State<AdminState>,
    Query(q): Query<Pg>,
) -> ApiResult<Value> {
    backfill_payables(&state).await?;
    let (page, size, offset) = page_of(&q);
    let mut cq: QueryBuilder<'_, Postgres> = QueryBuilder::new(
        "SELECT COUNT(*) FROM purchase_payable p LEFT JOIN vendor v ON v.vendor_id = p.vendor_id WHERE p.is_del = 0",
    );
    let mut rq: QueryBuilder<'_, Postgres> = QueryBuilder::new(
        "SELECT p.*, v.vendor_name AS vendor_name FROM purchase_payable p LEFT JOIN vendor v ON v.vendor_id = p.vendor_id WHERE p.is_del = 0",
    );
    if let Some(vendor_id) = q.vendor_id {
        cq.push(" AND p.vendor_id = ").push_bind(vendor_id);
        rq.push(" AND p.vendor_id = ").push_bind(vendor_id);
    }
    if let Some(shop_id) = q.shop_id {
        cq.push(" AND p.shop_id = ").push_bind(shop_id);
        rq.push(" AND p.shop_id = ").push_bind(shop_id);
    }
    if let Some(status) = q.status {
        cq.push(" AND p.status = ").push_bind(status);
        rq.push(" AND p.status = ").push_bind(status);
    }
    if let Some(kw) = q
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let like = format!("%{kw}%");
        for qb in [&mut cq, &mut rq] {
            qb.push(" AND (p.payable_sn ILIKE ")
                .push_bind(like.clone())
                .push(" OR p.reconciliation_sn ILIKE ")
                .push_bind(like.clone())
                .push(" OR v.vendor_name ILIKE ")
                .push_bind(like.clone())
                .push(")");
        }
    }
    rq.push(" ORDER BY p.id DESC LIMIT ")
        .push_bind(size as i64)
        .push(" OFFSET ")
        .push_bind(offset);
    let (total,): (i64,) = cq
        .build_query_as()
        .fetch_one(&state.repos.pool)
        .await
        .map_err(db)?;
    let rows = rq.build().fetch_all(&state.repos.pool).await.map_err(db)?;
    let records: Vec<Value> = rows.iter().map(payable_row_json).collect();
    ok(json!({"records": records, "total": total, "page": page, "size": size}))
}

async fn payable_detail(
    State(state): State<AdminState>,
    Query(q): Query<Pg>,
) -> ApiResult<Value> {
    let id = q.id.unwrap_or(0);
    let row = sqlx::query(
        "SELECT p.*, v.vendor_name AS vendor_name FROM purchase_payable p LEFT JOIN vendor v ON v.vendor_id = p.vendor_id WHERE p.id = $1 AND p.is_del = 0 LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&state.repos.pool)
    .await
    .map_err(db)?;
    let Some(row) = row else {
        return ok(Value::Null);
    };
    let mut obj = payable_row_json(&row);
    let reconciliation_id: i64 = row.try_get("reconciliation_id").unwrap_or(0);
    let payments = sqlx::query(
        "SELECT * FROM purchase_payment WHERE payable_id = $1 AND is_del = 0 ORDER BY pay_date DESC, id DESC",
    )
    .bind(id)
    .fetch_all(&state.repos.pool)
    .await
    .map_err(db)?;
    let rec = sqlx::query(
        "SELECT id, reconciliation_sn, receipt_amount, return_amount, total_amount, create_time FROM purchase_reconciliation WHERE id = $1 LIMIT 1",
    )
    .bind(reconciliation_id)
    .fetch_optional(&state.repos.pool)
    .await
    .map_err(db)?;
    if let Some(map) = obj.as_object_mut() {
        map.insert(
            "payments".into(),
            Value::Array(payments.iter().map(payment_row_json).collect()),
        );
        map.insert(
            "reconciliation".into(),
            rec.as_ref().map(|r| {
                json!({
                    "id": r.try_get::<i64, _>("id").unwrap_or(0),
                    "reconciliationSn": r.try_get::<String, _>("reconciliation_sn").unwrap_or_default(),
                    "receiptAmount": r.try_get::<rust_decimal::Decimal, _>("receipt_amount").unwrap_or_default().to_string(),
                    "returnAmount": r.try_get::<rust_decimal::Decimal, _>("return_amount").unwrap_or_default().to_string(),
                    "totalAmount": r.try_get::<rust_decimal::Decimal, _>("total_amount").unwrap_or_default().to_string(),
                    "createTime": r.try_get::<i64, _>("create_time").unwrap_or(0),
                })
            }).unwrap_or(Value::Null),
        );
    }
    ok(obj)
}

/// 付款日期入参：unix 秒（数字/数字串）或 yyyy-MM-dd 日期串。
fn parse_pay_date(v: Option<&Value>) -> i64 {
    match v {
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0),
        Some(Value::String(s)) => {
            let s = s.trim();
            if let Ok(ts) = s.parse::<i64>() {
                ts
            } else if let Ok(d) =
                sqlx::types::chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            {
                d.and_hms_opt(0, 0, 0)
                    .map(|dt| dt.and_utc().timestamp())
                    .unwrap_or(0)
            } else {
                0
            }
        }
        _ => 0,
    }
}

async fn payment_create(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<Value>,
) -> ApiResult<i64> {
    let payable_id = body.get("payableId").and_then(Value::as_i64).unwrap_or(0);
    if payable_id <= 0 {
        return Err(AppError(Error::biz(400, "请选择应付单")));
    }
    // 金额走双口径解析（数字/字符串均可），与采购价同口径
    let amount = parse_decimal_flex(body.get("amount"));
    if amount <= rust_decimal::Decimal::ZERO {
        return Err(AppError(Error::biz(400, "付款金额必须大于0")));
    }
    let pay_date = parse_pay_date(body.get("payDate"));
    if pay_date <= 0 {
        return Err(AppError(Error::biz(400, "请选择付款日期")));
    }
    let pay_method = body
        .get("payMethod")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if pay_method.is_empty() || pay_method.chars().count() > 32 {
        return Err(AppError(Error::biz(400, "请选择付款方式")));
    }
    let serial_no: String = body
        .get("serialNo")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .chars()
        .take(128)
        .collect();
    let voucher_remark = body
        .get("voucherRemark")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let remark = body
        .get("remark")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let now = nextrade_common::utils::now_seconds() as i64;
    let mut tx = state.repos.pool.begin().await.map_err(db)?;
    // 锁应付行后校验未付额：并发登记不能超付（与收货/退货 D6 的
    // 单据行锁同口径），登记与应付汇总更新同事务。
    let row = sqlx::query(
        "SELECT payable_amount, paid_amount FROM purchase_payable WHERE id = $1 AND is_del = 0 FOR UPDATE",
    )
    .bind(payable_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db)?
    .ok_or_else(|| AppError(Error::biz(400, "应付单不存在")))?;
    let payable_amount: rust_decimal::Decimal =
        row.try_get("payable_amount").unwrap_or_default();
    let paid_amount: rust_decimal::Decimal = row.try_get("paid_amount").unwrap_or_default();
    if paid_amount + amount > payable_amount {
        return Err(AppError(Error::biz(400, "付款金额超过未付金额")));
    }
    let payment_id: i64 = sqlx::query_scalar(
        "INSERT INTO purchase_payment(payable_id, amount, pay_date, pay_method, serial_no, voucher_remark, remark, create_time, create_by_id, create_by_name, update_time) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING id",
    )
    .bind(payable_id)
    .bind(amount)
    .bind(pay_date)
    .bind(&pay_method)
    .bind(&serial_no)
    .bind(&voucher_remark)
    .bind(&remark)
    .bind(now)
    .bind(i64::from(auth.id))
    .bind(&auth.username)
    .bind(now)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    let new_paid = paid_amount + amount;
    sqlx::query(
        "UPDATE purchase_payable SET paid_amount = $1, status = $2, update_time = $3, update_by_id = $4, update_by_name = $5 WHERE id = $6",
    )
    .bind(new_paid)
    .bind(derive_payable_status(payable_amount, new_paid))
    .bind(now)
    .bind(i64::from(auth.id))
    .bind(&auth.username)
    .bind(payable_id)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    tx.commit().await.map_err(db)?;
    ok(payment_id)
}

async fn payment_delete(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<Value>,
) -> ApiResult<()> {
    let payment_id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    if payment_id <= 0 {
        return Err(AppError(Error::biz(400, "请选择付款登记")));
    }
    let now = nextrade_common::utils::now_seconds() as i64;
    let mut tx = state.repos.pool.begin().await.map_err(db)?;
    let prow = sqlx::query(
        "SELECT payable_id FROM purchase_payment WHERE id = $1 AND is_del = 0 LIMIT 1",
    )
    .bind(payment_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db)?
    .ok_or_else(|| AppError(Error::biz(400, "付款登记不存在")))?;
    let payable_id: i64 = prow.try_get("payable_id").unwrap_or(0);
    let arow = sqlx::query(
        "SELECT payable_amount FROM purchase_payable WHERE id = $1 AND is_del = 0 FOR UPDATE",
    )
    .bind(payable_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db)?
    .ok_or_else(|| AppError(Error::biz(400, "应付单不存在")))?;
    let payable_amount: rust_decimal::Decimal =
        arow.try_get("payable_amount").unwrap_or_default();
    sqlx::query("UPDATE purchase_payment SET is_del = 1, update_time = $1 WHERE id = $2")
        .bind(now)
        .bind(payment_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    // 作废后按有效登记汇总重算（不做增减推算，防历史漂移）
    let (paid,): (rust_decimal::Decimal,) = sqlx::query_as(
        "SELECT COALESCE(SUM(amount), 0) FROM purchase_payment WHERE payable_id = $1 AND is_del = 0",
    )
    .bind(payable_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    sqlx::query(
        "UPDATE purchase_payable SET paid_amount = $1, status = $2, update_time = $3, update_by_id = $4, update_by_name = $5 WHERE id = $6",
    )
    .bind(paid)
    .bind(derive_payable_status(payable_amount, paid))
    .bind(now)
    .bind(i64::from(auth.id))
    .bind(&auth.username)
    .bind(payable_id)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    tx.commit().await.map_err(db)?;
    ok(())
}

async fn return_list(State(state): State<AdminState>, Query(q): Query<Pg>) -> ApiResult<Value> {
    ok(list_table(&state, "purchase_return", &q, false).await?)
}
async fn return_detail(State(state): State<AdminState>, Query(q): Query<Pg>) -> ApiResult<Value> {
    let mut detail = detail_table(&state, "purchase_return", q.id.unwrap_or(0)).await?;
    add_camel_aliases(&mut detail);
    ok(detail)
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
    let inputs = body
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if inputs.is_empty() {
        return Err(AppError(Error::biz(400, "请选择退货商品")));
    }
    let now = nextrade_common::utils::now_seconds() as i64;
    let return_sn = unique_doc_sn(&state, "purchase_return", "return_sn", "PT", now).await?;

    // —— 单据事务（库存与单据同事务）：先锁入库单行，再校验与落库 ——
    // 与 receipt_save 同口径：历史结构下可退量预检在事务外无锁、库存
    // 扣减先于单据落库自动提交，并发双发可双扣库存/超量退货，部分
    // 失败留无单据的库存扣减。现入库单行 FOR UPDATE 串行化同单退货，
    // 校验在锁内复核，库存走 stock_out_tx 与单据同事务。
    let mut tx = state.repos.pool.begin().await.map_err(db)?;
    let receipt = sqlx::query(
        "SELECT * FROM purchase_receipt WHERE id=$1 AND is_del=0 LIMIT 1 FOR UPDATE",
    )
    .bind(receipt_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db)?
    .ok_or_else(|| AppError(Error::biz(400, "采购入库单不存在或未确认")))?;
    if receipt.try_get::<i32, _>("status").unwrap_or(0) != 2 {
        return Err(AppError(Error::biz(400, "采购入库单不存在或未确认")));
    }
    let shop_id: i32 = receipt.try_get("shop_id").unwrap_or(0);
    let vendor_id: i32 = receipt.try_get("vendor_id").unwrap_or(0);
    let order_id: i64 = receipt.try_get("order_id").unwrap_or(0);

    // —— 校验（锁内、无写入）——
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
        .fetch_optional(&mut *tx)
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
    // 事务口径：走 stock_out_tx 落在本函数已开启的单据事务内（入库单行
    // 锁已持有），与退货单、入库行已退量同提交同回滚。
    let stock_record_id = state
        .catalog
        .product_stock
        .stock_out_tx(
            &mut *tx,
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

    // —— 单据落库（同事务）：退货单 + 明细 + 入库行已退量 ——
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
async fn setting_detail(
    State(state): State<AdminState>,
    Query(q): Query<Pg>,
) -> ApiResult<Value> {
    // 按组织解析（组织行优先、平台行兜底，与 load_allow_over_receive /
    // 调度器自动结束同一口径）：历史实现不读入参、无 shop 过滤取全表
    // 最新一行，多组织保存过设置后设置页会串显他组织的值。生效行来自
    // 平台兜底时回 inherited=true（前端设置页契约），并补驼峰别名键。
    let shop_id = q.shop_id.unwrap_or(0);
    for sid in [shop_id, 0] {
        let row = sqlx::query(
            "SELECT * FROM purchase_setting WHERE shop_id=$1 AND is_del=0 ORDER BY id DESC LIMIT 1",
        )
        .bind(sid)
        .fetch_optional(&state.repos.pool)
        .await
        .map_err(db)?;
        if let Some(row) = row {
            let mut v = row_to_json(&row);
            if let Some(obj) = v.as_object_mut() {
                let auto_days = obj
                    .get("auto_finish_days")
                    .cloned()
                    .unwrap_or(Value::Null);
                let over = obj
                    .get("allow_over_receive")
                    .cloned()
                    .unwrap_or(Value::Null);
                obj.insert("autoFinishDays".into(), auto_days);
                obj.insert("allowOverReceive".into(), over);
                obj.insert("inherited".into(), json!(sid != shop_id));
            }
            return ok(v);
        }
    }
    ok(Value::Null)
}
async fn setting_save(State(state): State<AdminState>, Json(body): Json<Value>) -> ApiResult<()> {
    let now = nextrade_common::utils::now_seconds() as i64;
    // 字段口径：显式传 null = 清空为「继承」（两列可空、NULL 即继承，
    // 与 load_allow_over_receive / 调度器解析一致；前端继承态正是发
    // null）；字段缺省沿用历史默认 0。历史实现对 null 一律落 0，
    // 「继承」会被静默写成「组织级关闭/0 天」，与解析口径脱节。
    let auto_days: Option<i32> = match body.get("autoFinishDays") {
        Some(Value::Null) => None,
        Some(v) => Some(v.as_i64().unwrap_or(0) as i32),
        None => Some(0),
    };
    let allow_over: Option<i32> = match body.get("allowOverReceive") {
        Some(Value::Null) => None,
        Some(v) => Some(v.as_i64().unwrap_or(0) as i32),
        None => Some(0),
    };
    sqlx::query("INSERT INTO purchase_setting(shop_id,auto_finish_days,allow_over_receive,create_time,update_time) VALUES($1,$2,$3,$4,$5) ON CONFLICT (shop_id) DO UPDATE SET auto_finish_days=EXCLUDED.auto_finish_days,allow_over_receive=EXCLUDED.allow_over_receive,update_time=EXCLUDED.update_time")
        .bind(body.get("shopId").and_then(Value::as_i64).unwrap_or(0) as i32)
        .bind(auto_days)
        .bind(allow_over)
        .bind(now).bind(now)
        .execute(&state.repos.pool).await.map_err(db)?;
    ok(())
}

// ————————————————— 采购单草稿（我方新增）口径测试 —————————————————
#[cfg(test)]
mod camel_alias_tests {
    use super::*;

    #[test]
    fn snake_to_camel_converts_doc_keys() {
        assert_eq!(snake_to_camel("order_sn"), "orderSn");
        assert_eq!(snake_to_camel("product_amount"), "productAmount");
        assert_eq!(snake_to_camel("create_time"), "createTime");
        assert_eq!(snake_to_camel("id"), "id");
        assert_eq!(snake_to_camel("is_del"), "isDel");
    }

    #[test]
    fn add_camel_aliases_keeps_snake_and_formats_epoch() {
        let mut v = json!({
            "order_sn": "PO1",
            "product_amount": "12.30",
            "create_time": 1_700_000_000,
            "status": 1
        });
        add_camel_aliases(&mut v);
        // 蛇形原键保留
        assert_eq!(v["order_sn"], json!("PO1"));
        assert_eq!(v["create_time"], json!(1_700_000_000));
        // 驼峰别名补齐；时间列按 supplier 范式格式化为字符串
        assert_eq!(v["orderSn"], json!("PO1"));
        assert_eq!(v["productAmount"], json!("12.30"));
        let ct = v["createTime"].as_str().expect("createTime 应为字符串");
        assert_eq!(ct.len(), 19);
        assert!(ct.contains('-') && ct.contains(':'));
        // 无下划线键不产生别名
        assert!(v.get("status").is_some());
        assert!(v.get("Status").is_none());
    }
}

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

    // ——— 供应商主数据与选品契约修复（D1/D2/D3）口径测试 ———

    #[test]
    fn supplier_form_extract_primary_phone_and_patch() {
        let body = json!({
            "contactName": "张三",
            "contactPhones": [
                {"type": "landline", "number": "021-555", "areaCode": "021"},
                {"type": "mobile", "number": "13800000000", "primary": true},
                {"type": "mobile", "number": "  "}
            ],
            "remark": "备注"
        });
        let ex = extract_supplier_form(&body);
        // 主号取显式 primary；空白号码行被剔除
        assert_eq!(ex.primary_mobile.as_deref(), Some("13800000000"));
        let phones = ex
            .data_patch
            .get("contactPhones")
            .and_then(Value::as_array)
            .unwrap();
        assert_eq!(phones.len(), 2);
        assert_eq!(ex.data_patch.get("contactName"), Some(&json!("张三")));
        assert_eq!(ex.data_patch.get("remark"), Some(&json!("备注")));
        // 无 primary 时取首个有效号码
        let body2 = json!({"contactPhones": [{"type": "mobile", "number": "111"}]});
        assert_eq!(
            extract_supplier_form(&body2).primary_mobile.as_deref(),
            Some("111")
        );
        // 存量 contactMobile 单值兼容
        let body3 = json!({"contactMobile": " 139 "});
        assert_eq!(
            extract_supplier_form(&body3).primary_mobile.as_deref(),
            Some("139")
        );
        // 未传任何扩展字段时 patch 为空（更新不覆盖）
        assert!(extract_supplier_form(&json!({})).data_patch.is_empty());
    }

    #[test]
    fn vendor_data_merge_keeps_existing_keys() {
        let mut patch = Map::new();
        patch.insert("contactName".to_string(), json!("李四"));
        let merged = merge_vendor_data(
            Some("{\"contactPhone\":\"021-1\",\"contactName\":\"旧\"}"),
            &patch,
        );
        let v: Value = serde_json::from_str(&merged).unwrap();
        // 商家域既有键不被冲掉，同义键按新值更新
        assert_eq!(v.get("contactPhone"), Some(&json!("021-1")));
        assert_eq!(v.get("contactName"), Some(&json!("李四")));
        // 非法存量 JSON 按空对象合并，不 panic
        let merged2 = merge_vendor_data(Some("not-json"), &patch);
        assert!(serde_json::from_str::<Value>(&merged2).is_ok());
    }

    #[test]
    fn vendor_master_fix_invariants_hold_in_source() {
        // 源码级不变量（同 draft_isolation 手法，concat! 拼针防自证）。
        let src = include_str!("supply_chain.rs");
        // D1) 供应价走双口径解析，不再只认字符串
        assert!(src.contains(concat!(
            "parse_decimal_flex(sku.get(",
            "\"supplyPrice\"))"
        )));
        // D2) 建档 INSERT 落 vendor_data；更新带未命中校验
        assert!(src.contains(concat!("add_time, vendor", "_data) VALUES(2,")));
        assert!(src.contains(concat!("rows_affected() ", "== 0")));
        // D3) selection 路由指向专用查询而非通用列表
        assert!(src.contains(concat!(
            "supplierProduct/selection\", get(product",
            "_selection))"
        )));
    }

    // ——— 供应商/采购链缺陷批（D4/D6/D7/D9/D10）口径测试 ———

    #[test]
    fn list_filter_columns_match_real_schema() {
        // 模式实证（00-pure.sql）：vendor_product 无 status/category_name
        // 列，状态列为 product_state；采购单据四表无 category_name。
        // 映射里出现的每一列都必须在对应表内真实存在，否则列表 500。
        let known: &[(&str, &[&str])] = &[
            ("vendor", &["status", "vendor_name"]),
            (
                "purchase_supplier_category",
                &["status", "category_name"],
            ),
            (
                "vendor_product",
                &["product_state", "product_name", "product_sn"],
            ),
            ("purchase_order", &["status", "order_sn", "vendor_name"]),
            ("purchase_receipt", &["status", "receipt_sn"]),
            ("purchase_return", &["status", "return_sn"]),
            ("purchase_reconciliation", &["status", "reconciliation_sn"]),
        ];
        for (table, cols) in known {
            let (status_col, kw_cols) = list_filter_columns(table);
            assert!(
                cols.contains(&status_col),
                "{table} 状态列 {status_col} 不在实证列集内"
            );
            for c in kw_cols {
                assert!(cols.contains(c), "{table} 关键词列 {c} 不在实证列集内");
            }
        }
        // 回归针：vendor_product 的状态过滤必须落到 product_state
        assert_eq!(list_filter_columns("vendor_product").0, "product_state");
        // 回归针：除分类表外，任何表都不许再把 category_name 当关键词列
        for (table, _) in known {
            if *table != "purchase_supplier_category" {
                assert!(
                    !list_filter_columns(table).1.contains(&"category_name"),
                    "{table} 不应映射 category_name"
                );
            }
        }
    }

    #[test]
    fn vendor_chain_fix_invariants_hold_in_source() {
        // 本批守卫不变量（源码级断言，同既有手法；针串 concat! 拼装
        // 避免断言行自身字面量让 contains 恒真）。
        let src = include_str!("supply_chain.rs");
        // D6) 收货/退货的库存动作走事务变体、且父单行锁在位
        assert!(src.contains(concat!("stock_in", "_tx")));
        assert!(src.contains(concat!("stock_out", "_tx")));
        assert!(src.contains(concat!(
            "FROM purchase_order WHERE id=$1 AND is_del=0 LIMIT 1 FOR",
            " UPDATE"
        )));
        assert!(src.contains(concat!(
            "FROM purchase_receipt WHERE id=$1 AND is_del=0 LIMIT 1 FOR",
            " UPDATE"
        )));
        // D7) 对账状态流转先查状态（未命中明确报错）
        assert!(src.contains(concat!("load_reconciliation", "_status")));
        assert!(src.contains(concat!("只有待审核的采购对账单", "可以审核通过")));
        // D9) 建单供应商启用校验在位
        assert!(src.contains(concat!("ensure_vendor", "_enabled")));
        // D10) 四类单号都经防撞号生成（针串拆段避免断言行自匹配）
        assert!(src.contains(concat!("unique_doc", "_sn")));
        assert_eq!(src.matches(concat!("unique_doc", "_sn(&state")).count(), 3);
        assert_eq!(src.matches(concat!("unique_doc", "_sn(state")).count(), 1);
    }

    // ——— 采购结算（应付/付款登记，我方新增）口径测试 ———

    #[test]
    fn payable_status_is_derived_from_amounts() {
        use rust_decimal::Decimal;
        let d = |s: &str| Decimal::from_str(s).unwrap();
        // 未付 → 待付款
        assert_eq!(derive_payable_status(d("100.00"), d("0")), 1);
        // 部分付 → 部分付款
        assert_eq!(derive_payable_status(d("100.00"), d("40.50")), 2);
        // 付清 / 超付（守卫上不会发生，推导仍收敛为结清）
        assert_eq!(derive_payable_status(d("100.00"), d("100.00")), 3);
        assert_eq!(derive_payable_status(d("100.00"), d("120.00")), 3);
        // 应付额为 0 或负数（退货净额抵平/反超）→ 无付款义务，直接结清
        assert_eq!(derive_payable_status(d("0"), d("0")), 3);
        assert_eq!(derive_payable_status(d("-20.00"), d("0")), 3);
    }

    #[test]
    fn pay_date_accepts_unix_and_date_string() {
        assert_eq!(parse_pay_date(Some(&json!(1763845600))), 1763845600);
        assert_eq!(parse_pay_date(Some(&json!("1763845600"))), 1763845600);
        // 日期串按 UTC 零点落秒（只记日期口径）
        assert_eq!(parse_pay_date(Some(&json!("2026-11-21"))), 1795219200);
        assert_eq!(parse_pay_date(Some(&json!(""))), 0);
        assert_eq!(parse_pay_date(Some(&json!("abc"))), 0);
        assert_eq!(parse_pay_date(None), 0);
    }

    #[test]
    fn payable_settlement_invariants_hold_in_source() {
        // 源码级不变量（同 draft_isolation 手法，concat! 拼针防自证）。
        let src = include_str!("supply_chain.rs");
        // 1) 应付四端点已注册
        assert!(src.contains(concat!("purchasePayable", "/list")));
        assert!(src.contains(concat!("purchasePayable/payment", "/create")));
        assert!(src.contains(concat!("purchasePayable/payment", "/delete")));
        // 2) 对账通过钩子在位：approve 后生成应付
        assert!(src.contains(concat!("ensure_payable", "_for(&state, id)")));
        // 3) 生成与对账单 1:1 幂等：ON CONFLICT 不重复生成
        assert!(src.contains(concat!(
            "ON CONFLICT (reconciliation_id)",
            " DO NOTHING"
        )));
        // 4) 登记防超付 + 应付行锁（并发口径）
        assert!(src.contains(concat!("付款金额超过", "未付金额")));
        assert!(src.contains(concat!(
            "FROM purchase_payable WHERE id = $1 AND is_del = 0",
            " FOR UPDATE"
        )));
        // 5) 状态只由推导函数产生：入参里不存在可写 status 的字段名
        assert!(!src.contains(concat!("body.get(\"payable", "Status\")")));
        assert!(!src.contains(concat!("body.get(\"paid", "Amount\")")));
    }
}
