//! vendor 低频接口（vendor::lowfreq） —— 一次性低频接口（b2b2b 5.8.57 新版前端调用，无 Java 参考；
//! 列名取自 overseas pure.sql，简化口径见各 handler 注释）。
use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;
use sqlx::{FromRow, Postgres, QueryBuilder};

use nextrade_core::models::catalog::product_manage_extra::{
    audit_result_product_status, parse_audit_shelf_type, CFG_VENDOR_PRODUCT_AUDIT_SHELF_TYPE,
    CHECK_STATUS_APPROVED,
};
use nextrade_core::models::catalog::product_sale::ProductSaleState;
use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult};

use crate::AdminState;

fn pool(state: &AdminState) -> sqlx::PgPool {
    state.system.config_basic.repo().pool_clone()
}
fn db(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Pg {
    pub page: Option<u64>,
    pub size: Option<u64>,
    pub keyword: Option<String>,
    pub id: Option<i64>,
    pub ids: Option<Vec<i64>>,
    pub field: Option<String>,
    pub value: Option<Value>,
    pub status: Option<i32>,
    pub vendor_id: Option<i32>,
    pub shop_id: Option<i64>,
    pub product_id: Option<i64>,
    pub order_id: Option<i32>,
    pub parent_id: Option<i32>,
}

fn page_of(q: &Pg) -> (i64, i64, i64) {
    let p = q.page.unwrap_or(1).max(1) as i64;
    let s = q.size.unwrap_or(10).clamp(1, 100) as i64;
    (p, s, (p - 1) * s)
}

async fn read_cfg(state: &AdminState, code: &str) -> Result<Value, Error> {
    let raw = state.system.config_basic.repo().find_by_code(code).await?;
    Ok(raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({})))
}

macro_rules! list_pg {
    ($fn_name:ident, $table:literal, $key:literal, $jo:expr, $like:literal) => {
        async fn $fn_name(State(state): State<AdminState>, Query(q): Query<Pg>) -> ApiResult<Value> {
            let (p, s, off) = page_of(&q);
            let kw = format!("%{}%", q.keyword.clone().unwrap_or_default().trim());
            // PG 里 `ILIKE` 只能作用于 text：`vendor_id` / `add_time` / `admin_id` /
            // `merchant_apply_id` / `type` 等列在 MySQL 时代可以隐式转字符串，PG 直接报
            // `operator does not exist: integer ~~ text`。统一 CAST 成 text（对 text 列是零成本直通）。
            let (total,): (i64,) = sqlx::query_as(
                &format!("SELECT COUNT(*) FROM \"{}\" WHERE CAST(\"{}\" AS TEXT) ILIKE $1", $table, $like))
                .bind(&kw).fetch_one(&pool(&state)).await.map_err(db)?;
            let rows: Vec<(Value,)> = sqlx::query_as(&format!(
                "SELECT json_build_object({}) FROM \"{}\" WHERE CAST(\"{}\" AS TEXT) ILIKE $1 ORDER BY \"{}\" DESC LIMIT $2 OFFSET $3",
                $jo, $table, $like, $key))
                .bind(&kw).bind(s).bind(off)
                .fetch_all(&pool(&state)).await.map_err(db)?;
            ok(serde_json::json!({"records": rows.into_iter().map(|(v,)| v).collect::<Vec<_>>(), "total": total, "page": p, "size": s}))
        }
    };
}

macro_rules! detail_one {
    ($fn_name:ident, $table:literal, $key:literal, $jo:expr) => {
        async fn $fn_name(
            State(state): State<AdminState>,
            Query(q): Query<Pg>,
        ) -> ApiResult<Value> {
            let row: Option<(Value,)> = sqlx::query_as(&format!(
                "SELECT json_build_object({}) FROM \"{}\" WHERE \"{}\" = $1 LIMIT 1",
                $jo, $table, $key
            ))
            .bind(q.id.unwrap_or(0))
            .fetch_optional(&pool(&state))
            .await
            .map_err(db)?;
            ok(row.map(|(v,)| v).unwrap_or(Value::Null))
        }
    };
}

macro_rules! upd_field {
    ($fn_name:ident, $table:literal, $key:literal, [$($fld:literal),+]) => {
        async fn $fn_name(State(state): State<AdminState>, Json(q): Json<Pg>) -> ApiResult<()> {
            let f = q.field.clone().unwrap_or_default();
            if ![$($fld),+].contains(&f.as_str()) {
                return Err(nextrade_security::web::AppError(Error::Validation("不支持的字段".into())));
            }
            let v = match q.value.clone().unwrap_or(Value::Null) {
                Value::String(sv) => sv,
                other => other.to_string(),
            };
            sqlx::query(&format!("UPDATE \"{}\" SET \"{}\" = $1 WHERE \"{}\" = $2", $table, f, $key))
                .bind(v).bind(q.id.unwrap_or(0))
                .execute(&pool(&state)).await.map_err(db)?;
            ok(())
        }
    };
}

macro_rules! del_ids {
    ($fn_name:ident, $table:literal, $key:literal) => {
        async fn $fn_name(State(state): State<AdminState>, Json(q): Json<Pg>) -> ApiResult<()> {
            let ids = q
                .ids
                .clone()
                .or_else(|| q.id.map(|v| vec![v]))
                .unwrap_or_default();
            if ids.is_empty() {
                return Err(nextrade_security::web::AppError(Error::Validation(
                    "缺少 id".into(),
                )));
            }
            let list = ids
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",");
            sqlx::query(&format!(
                "DELETE FROM \"{}\" WHERE \"{}\" IN ({})",
                $table, $key, list
            ))
            .execute(&pool(&state))
            .await
            .map_err(db)?;
            ok(())
        }
    };
}

macro_rules! cfg_get {
    ($fn_name:ident, $code:literal) => {
        async fn $fn_name(State(state): State<AdminState>) -> ApiResult<Value> {
            ok(read_cfg(&state, $code).await?)
        }
    };
}

macro_rules! cfg_set {
    ($fn_name:ident, $code:literal) => {
        async fn $fn_name(
            State(state): State<AdminState>,
            Json(body): Json<serde_json::Map<String, Value>>,
        ) -> ApiResult<()> {
            state
                .system
                .config_basic
                .repo()
                .save_code($code, &Value::Object(body).to_string())
                .await?;
            ok(())
        }
    };
}

/// 通用状态更新：body {id 或 ids, status?}（缺省 0=撤回待处理）。
async fn set_status(
    state: &AdminState,
    table: &str,
    key: &str,
    q: &Pg,
    status: i32,
) -> Result<(), Error> {
    let ids = q
        .ids
        .clone()
        .or_else(|| q.id.map(|v| vec![v]))
        .unwrap_or_default();
    if ids.is_empty() {
        return Err(Error::Validation("缺少 id".into()));
    }
    let list = ids
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(",");
    sqlx::query(&format!(
        "UPDATE \"{}\" SET status = $1 WHERE \"{}\" IN ({})",
        table, key, list
    ))
    .bind(status)
    .execute(&pool(state))
    .await
    .map_err(db)?;
    Ok(())
}

// post /vendor/vendorWithdraw/list
list_pg!(h_0, "vendor_withdraw", "vendor_withdraw_log_id", "'vendor_withdraw_log_id', vendor_withdraw_log_id, 'vendor_id', vendor_id, 'amount', amount, 'handling_fee', handling_fee, 'handling_rate', handling_rate, 'status', status, 'add_time', add_time, 'remark', remark, 'vendor_account_id', vendor_account_id, 'account_data', account_data, 'audit_remark', audit_remark, 'payment_voucher', payment_voucher, 'withdraw_sn', withdraw_sn", "vendor_id");

// get /vendor/vendorWithdraw/detail
detail_one!(h_1, "vendor_withdraw", "vendor_withdraw_log_id", "'vendor_withdraw_log_id', vendor_withdraw_log_id, 'vendor_id', vendor_id, 'amount', amount, 'handling_fee', handling_fee, 'handling_rate', handling_rate, 'status', status, 'add_time', add_time, 'remark', remark, 'vendor_account_id', vendor_account_id, 'account_data', account_data, 'audit_remark', audit_remark, 'payment_voucher', payment_voucher, 'withdraw_sn', withdraw_sn");

// post /vendor/vendorWithdraw/create
async fn h_2(State(state): State<AdminState>, Json(q): Json<Pg>) -> ApiResult<i64> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let r: i32 = sqlx::query_scalar("INSERT INTO vendor_withdraw (vendor_id, amount, status, add_time, withdraw_sn) VALUES ($1, 0, 0, $2, $3) RETURNING vendor_withdraw_log_id")
        .bind(q.vendor_id.unwrap_or(0)).bind(now).bind(format!("WD{now}"))
        .fetch_one(&pool(&state)).await.map_err(db)?;
    ok(i64::from(r))
}

// post /vendor/vendorWithdraw/audit
async fn h_3(State(state): State<AdminState>, Json(q): Json<Pg>) -> ApiResult<()> {
    sqlx::query("UPDATE vendor_withdraw SET status = $1 WHERE vendor_withdraw_log_id = $2")
        .bind(q.status.unwrap_or(1))
        .bind(q.id.unwrap_or(0))
        .execute(&pool(&state))
        .await
        .map_err(db)?;
    ok(())
}

// get /vendor/vendorWithdraw/config
cfg_get!(h_4, "vendorWithdrawConfig");

// get /vendor/vendorWithdraw/summaryList
list_pg!(h_5, "vendor_withdraw", "vendor_withdraw_log_id", "'vendor_withdraw_log_id', vendor_withdraw_log_id, 'vendor_id', vendor_id, 'amount', amount, 'handling_fee', handling_fee, 'handling_rate', handling_rate, 'status', status, 'add_time', add_time, 'remark', remark, 'vendor_account_id', vendor_account_id, 'account_data', account_data, 'audit_remark', audit_remark, 'payment_voucher', payment_voucher, 'withdraw_sn', withdraw_sn", "vendor_id");

// post /vendor/vendorWithdraw/uploadPayVoucher
async fn h_6(State(state): State<AdminState>, Json(q): Json<Pg>) -> ApiResult<()> {
    sqlx::query("UPDATE vendor_withdraw SET payment_voucher = $1 WHERE vendor_withdraw_log_id = $2")
        .bind(q.keyword.clone().unwrap_or_default())
        .bind(q.id.unwrap_or(0))
        .execute(&pool(&state))
        .await
        .map_err(db)?;
    ok(())
}

// post /vendor/vendorShopBind/list
list_pg!(
    h_7,
    "vendor_shop_bind",
    "id",
    "'id', id, 'vendor_id', vendor_id, 'shop_id', shop_id, 'add_time', add_time",
    "vendor_id"
);

// get /vendor/vendorShopBind/merchantList
list_pg!(h_8, "merchant", "merchant_id", "'merchant_id', merchant_id, 'merchant_apply_id', merchant_apply_id, 'user_id', user_id, 'add_time', add_time, 'base_data', base_data, 'shop_data', shop_data, 'merchant_data', merchant_data, 'status', status, 'is_delete', is_delete, 'type', type, 'company_name', company_name, 'corporate_name', corporate_name, 'settlement_cycle', settlement_cycle", "merchant_apply_id");

/// 供应商订单列表查询（GET /vendor/vendorShopBind/orderList 专用）。
/// vendorId / shopId 在前端是文本输入框：纯数字按 ID 等值过滤，非数字按名称模糊。
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VendorOrderListQuery {
    pub page: Option<u64>,
    pub size: Option<u64>,
    pub keyword: Option<String>,
    pub transaction_id: Option<String>,
    pub vendor_id: Option<String>,
    pub shop_id: Option<String>,
    pub order_status: Option<i32>,
    /// 前端筛选名为 payStatus，但选项是支付方式（在线支付/货到付款/线下支付），
    /// 按 pay_type_id 消费（pay_type：1 在线 / 2 货到付款 / 3 线下）。
    pub pay_status: Option<i32>,
    pub shipping_status: Option<i32>,
}

/// epoch 秒 → `yyyy-MM-dd HH:mm:ss`（与平台 OrderVO 的 fmt_time 同口径；<=0 为空串）。
fn fmt_epoch_vendor(secs: i64) -> String {
    if secs <= 0 {
        return String::new();
    }
    use chrono::TimeZone;
    match chrono::Local.timestamp_opt(secs, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => String::new(),
    }
}

/// 供应商订单列表的 WHERE（count 与 rows 共用同一函数，保证口径一致）。
///
/// 范围口径（平台视角）：订单头 `vendor_id > 0` 的供应商订单——原版 Java
/// `VendorShopBindMapper.pageOrderList` 为供应商本人视角（vendor_id = 当前供应商），
/// 平台页放宽为全部供应商 + 可选 vendorId 过滤；订单头 vendor_id 由下单/拆单的
/// `header_vendor_id` 口径写入（同店铺同供应商分组），与供应商订单判定一致。
/// 软删/有效口径对齐平台订单列表（is_del=0、is_valid=1，拆后无效父单不出现）。
fn vendor_order_where(qb: &mut QueryBuilder<Postgres>, q: &VendorOrderListQuery) {
    qb.push(
        " WHERE o.is_del = 0 AND o.is_valid = 1 AND o.vendor_id IS NOT NULL AND o.vendor_id > 0",
    );
    if let Some(st) = q.order_status.filter(|v| *v != -1) {
        qb.push(" AND o.order_status = ").push_bind(st);
    }
    // 关键词：订单号 / 收货人姓名 / 商品名称（页面搜索框规范，商品名走明细 EXISTS）
    if let Some(kw) = q.keyword.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        let like = format!("%{kw}%");
        qb.push(" AND (o.order_sn ILIKE ")
            .push_bind(like.clone())
            .push(" OR o.consignee ILIKE ")
            .push_bind(like.clone())
            .push(
                " OR EXISTS (SELECT 1 FROM order_item oi WHERE oi.order_id = o.order_id AND oi.product_name ILIKE ",
            )
            .push_bind(like)
            .push("))");
    }
    if let Some(tx) = q
        .transaction_id
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        qb.push(" AND o.out_trade_no ILIKE ")
            .push_bind(format!("%{tx}%"));
    }
    if let Some(v) = q.vendor_id.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        if let Ok(n) = v.parse::<i32>() {
            qb.push(" AND o.vendor_id = ").push_bind(n);
        } else {
            qb.push(" AND v.vendor_name ILIKE ")
                .push_bind(format!("%{v}%"));
        }
    }
    if let Some(v) = q.shop_id.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        if let Ok(n) = v.parse::<i32>() {
            qb.push(" AND o.shop_id = ").push_bind(n);
        } else {
            qb.push(" AND s.shop_title ILIKE ")
                .push_bind(format!("%{v}%"));
        }
    }
    if let Some(ps) = q.pay_status {
        qb.push(" AND o.pay_type_id = ").push_bind(ps);
    }
    if let Some(ss) = q.shipping_status {
        qb.push(" AND o.shipping_status = ").push_bind(ss);
    }
}

/// 供应商订单列表的明细行（金额仅展示用，SQL 侧 cast float8，避免 Decimal 解码口径问题）。
#[derive(Debug, FromRow)]
struct VendorOrderItemRow {
    item_id: i32,
    order_id: i32,
    product_id: i32,
    product_name: String,
    product_sn: String,
    pic_thumb: String,
    sku_id: i32,
    sku_data: String,
    price: f64,
    quantity: i32,
    vendor_id: Option<i32>,
    vendor_product_id: Option<i32>,
    vendor_product_supply_price: Option<f64>,
}

/// 明细行 → 前端 JSON（字段对齐平台 OrderItemVO + 供货价 vendorProductSupplyPrice）。
fn vendor_item_json(it: &VendorOrderItemRow) -> Value {
    let sku_arr: Vec<Value> = serde_json::from_str(&it.sku_data).unwrap_or_default();
    // 规格文案 `名称:值|名称:值`（与服务层 sku_value_of 同口径，取下单快照 sku_data）
    let sku_text = sku_arr
        .iter()
        .filter_map(|v| {
            let name = v.get("name")?.as_str()?;
            let raw = v.get("value")?;
            let value = raw
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| raw.to_string());
            Some(format!("{name}:{value}"))
        })
        .collect::<Vec<_>>()
        .join("|");
    let sku_value = if sku_text.is_empty() {
        Value::Null
    } else {
        Value::String(sku_text)
    };
    serde_json::json!({
        "itemId": it.item_id,
        "productId": it.product_id,
        "productName": it.product_name,
        "productSn": it.product_sn,
        "picThumb": it.pic_thumb,
        "skuId": it.sku_id,
        "skuData": sku_arr,
        "skuValue": sku_value,
        "price": it.price,
        "quantity": it.quantity,
        "vendorId": it.vendor_id,
        "vendorProductId": it.vendor_product_id,
        "vendorProductSupplyPrice": it.vendor_product_supply_price,
    })
}

// get /vendor/vendorShopBind/orderList
// 原版有·我方移植残桩补齐（2026-11-21）：原版 Java `VendorShopBindServiceImpl#orderList`
// 返回完整 OrderVO——订单头全字段回填 + 会员/店铺批量组装 + 明细 items 批量组装
// （getOrderItemList 按 orderIds 一次取回）。旧实现只下发 orderId/orderSn/userId/
// totalAmount 四个字段、不消费任何过滤参数、连 total 都缺，前端大半列无数据可渲染。
async fn h_9(
    State(state): State<AdminState>,
    Query(q): Query<VendorOrderListQuery>,
) -> ApiResult<Value> {
    let p = q.page.unwrap_or(1).max(1) as i64;
    let s = q.size.unwrap_or(10).clamp(1, 100) as i64;
    let off = (p - 1) * s;
    let pool = pool(&state);
    const JOINS: &str = " FROM \"order\" o LEFT JOIN \"user\" u ON u.user_id = o.user_id LEFT JOIN shop s ON s.shop_id = o.shop_id LEFT JOIN vendor v ON v.vendor_id = o.vendor_id";

    let mut cq = QueryBuilder::<Postgres>::new("SELECT COUNT(*)".to_string() + JOINS);
    vendor_order_where(&mut cq, &q);
    let (total,): (i64,) = cq.build_query_as().fetch_one(&pool).await.map_err(db)?;

    let mut rq = QueryBuilder::<Postgres>::new(
        "SELECT json_build_object('orderId', o.order_id, 'orderSn', o.order_sn, 'userId', o.user_id, 'username', COALESCE(u.username, ''), 'consignee', o.consignee, 'mobile', o.mobile, 'address', o.address, 'regionNames', o.region_names, 'addTime', o.add_time, 'totalAmount', o.total_amount, 'payStatus', o.pay_status, 'payTypeId', o.pay_type_id, 'payName', CASE o.pay_type_id WHEN 1 THEN '在线支付' WHEN 2 THEN '货到付款' WHEN 3 THEN '线下支付' ELSE '' END, 'shippingStatus', o.shipping_status, 'shippingName', COALESCE(NULLIF(o.logistics_name, ''), o.shipping_type_name), 'orderStatus', o.order_status, 'shopId', o.shop_id, 'shopName', COALESCE(s.shop_title, ''), 'vendorId', o.vendor_id, 'vendorName', COALESCE(v.vendor_name, ''), 'isSplit', CASE WHEN o.parent_order_id IS NOT NULL AND o.parent_order_id > 0 THEN 1 ELSE 0 END, 'parentId', o.parent_order_id, 'orderType', o.order_type, 'mark', o.mark, 'outTradeNo', o.out_trade_no)".to_string()
            + JOINS,
    );
    vendor_order_where(&mut rq, &q);
    rq.push(" ORDER BY o.order_id DESC LIMIT ")
        .push_bind(s)
        .push(" OFFSET ")
        .push_bind(off);
    let rows: Vec<(Value,)> = rq.build_query_as().fetch_all(&pool).await.map_err(db)?;
    let mut records: Vec<Value> = rows.into_iter().map(|(v,)| v).collect();

    // 明细批量组装（一次 ANY 查询取回本页全部明细，防 N+1；与平台订单列表同口径）
    let order_ids: Vec<i32> = records
        .iter()
        .filter_map(|r| r.get("orderId").and_then(Value::as_i64).map(|v| v as i32))
        .collect();
    let mut items_map: std::collections::HashMap<i32, Vec<Value>> =
        std::collections::HashMap::new();
    if !order_ids.is_empty() {
        let items: Vec<VendorOrderItemRow> = sqlx::query_as(
            "SELECT item_id, order_id, product_id, product_name, product_sn, pic_thumb, sku_id, sku_data, price::float8 AS price, quantity, vendor_id, vendor_product_id, vendor_product_supply_price::float8 AS vendor_product_supply_price FROM order_item WHERE order_id = ANY($1) ORDER BY order_id ASC, item_id ASC",
        )
        .bind(order_ids.as_slice())
        .fetch_all(&pool)
        .await
        .map_err(db)?;
        for it in &items {
            items_map
                .entry(it.order_id)
                .or_default()
                .push(vendor_item_json(it));
        }
    }

    for rec in &mut records {
        let Some(obj) = rec.as_object_mut() else {
            continue;
        };
        let oid = obj.get("orderId").and_then(Value::as_i64).unwrap_or(0) as i32;
        // 下单时间：epoch 秒 → 文本（前端直接渲染，与平台 OrderVO.addTime 同口径）
        let epoch = obj.get("addTime").and_then(Value::as_i64).unwrap_or(0);
        obj.insert(
            "addTime".to_string(),
            Value::String(fmt_epoch_vendor(epoch)),
        );
        // 收货地址：region_names（JSON 数组文本）拼接详细地址（平台 OrderVO.userAddress 口径）
        let region_text = obj
            .remove("regionNames")
            .as_ref()
            .and_then(Value::as_str)
            .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok())
            .map(|parts| parts.concat())
            .unwrap_or_default();
        if !region_text.is_empty() {
            let detail = obj
                .get("address")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            obj.insert(
                "address".to_string(),
                Value::String(format!("{region_text}{detail}")),
            );
        }
        obj.insert(
            "items".to_string(),
            Value::Array(items_map.remove(&oid).unwrap_or_default()),
        );
    }

    ok(serde_json::json!({"records": records, "total": total, "page": p, "size": s}))
}

// get /vendor/vendorShopBind/productList
async fn h_10(State(state): State<AdminState>, Query(q): Query<Pg>) -> ApiResult<Value> {
    let (p, s, off) = page_of(&q);
    let rows: Vec<(Value,)> = sqlx::query_as("SELECT json_build_object('productId', product_id, 'productName', product_name, 'picThumb', pic_thumb, 'productPrice', product_price) FROM product WHERE shop_id = $1 ORDER BY product_id DESC LIMIT $2 OFFSET $3")
        .bind(q.shop_id.unwrap_or(0)).bind(s).bind(off).fetch_all(&pool(&state)).await.map_err(db)?;
    ok(
        serde_json::json!({"records": rows.into_iter().map(|(v,)| v).collect::<Vec<_>>(), "page": p, "size": s}),
    )
}

// get /vendor/vendorShopBind/shopList
list_pg!(
    h_11,
    "vendor_shop_bind",
    "id",
    "'id', id, 'vendor_id', vendor_id, 'shop_id', shop_id, 'add_time', add_time",
    "vendor_id"
);

// post /vendor/adminVendorUser/list
list_pg!(h_12, "admin_user_vendor", "id", "'id', id, 'admin_id', admin_id, 'user_id', user_id, 'vendor_id', vendor_id, 'username', username, 'email', email, 'avatar', avatar, 'auth_list', auth_list, 'is_using', is_using, 'is_admin', is_admin, 'add_time', add_time, 'role_id', role_id", "admin_id");

// post /vendor/adminVendorUser/batch
del_ids!(h_13, "admin_user_vendor", "id");

// post /vendor/adminVendorUser/del
del_ids!(h_14, "admin_user_vendor", "id");

// get /vendor/adminVendorAccount/index
list_pg!(h_15, "vendor_account", "account_id", "'account_id', account_id, 'vendor_id', vendor_id, 'account_type', account_type, 'account_name', account_name, 'account_no', account_no, 'bank_name', bank_name, 'add_time', add_time, 'bank_branch', bank_branch", "vendor_id");

// get /vendor/adminVendorAccount/listFund
list_pg!(h_16, "vendor_account", "account_id", "'account_id', account_id, 'vendor_id', vendor_id, 'account_type', account_type, 'account_name', account_name, 'account_no', account_no, 'bank_name', bank_name, 'add_time', add_time, 'bank_branch', bank_branch", "vendor_id");

// get /vendor/adminVendorAccount/logList
list_pg!(h_17, "vendor_account_log", "vendor_account_log_id", "'vendor_account_log_id', vendor_account_log_id, 'add_time', add_time, 'vendor_money', vendor_money, 'frozen_money', frozen_money, 'type', type, 'remarks', remarks, 'new_vendor_money', new_vendor_money, 'new_frozen_money', new_frozen_money, 'vendor_id', vendor_id", "add_time");

// get /vendor/vendorAccount/index
list_pg!(h_18, "vendor_account", "account_id", "'account_id', account_id, 'vendor_id', vendor_id, 'account_type', account_type, 'account_name', account_name, 'account_no', account_no, 'bank_name', bank_name, 'add_time', add_time, 'bank_branch', bank_branch", "vendor_id");

// get /vendor/vendorAccount/list
list_pg!(h_19, "vendor_account", "account_id", "'account_id', account_id, 'vendor_id', vendor_id, 'account_type', account_type, 'account_name', account_name, 'account_no', account_no, 'bank_name', bank_name, 'add_time', add_time, 'bank_branch', bank_branch", "vendor_id");

// get /vendor/vendorAccount/logList
list_pg!(h_20, "vendor_account_log", "vendor_account_log_id", "'vendor_account_log_id', vendor_account_log_id, 'add_time', add_time, 'vendor_money', vendor_money, 'frozen_money', frozen_money, 'type', type, 'remarks', remarks, 'new_vendor_money', new_vendor_money, 'new_frozen_money', new_frozen_money, 'vendor_id', vendor_id", "add_time");

// post /vendor/vendorAccount/del
del_ids!(h_21, "vendor_account", "account_id");

// get /vendor/vendorAccount/config
cfg_get!(h_22, "vendorAccountConfig");

// get /vendor/vendorAccount/applyShopAgreement
cfg_get!(h_23, "vendorApplyShopAgreement");

// post /vendor/vendor/delVendor
del_ids!(h_24, "vendor", "vendor_id");

// get /vendor/vendor/staffShow
list_pg!(h_25, "admin_user_vendor", "id", "'id', id, 'admin_id', admin_id, 'user_id', user_id, 'vendor_id', vendor_id, 'username', username, 'email', email, 'avatar', avatar, 'auth_list', auth_list, 'is_using', is_using, 'is_admin', is_admin, 'add_time', add_time, 'role_id', role_id", "admin_id");

// post /vendor/vendor/updateField
upd_field!(
    h_26,
    "vendor",
    "vendor_id",
    ["vendor_name", "status", "contact_mobile"]
);

// post /vendor/vendor/updateInfo
upd_field!(
    h_27,
    "vendor",
    "vendor_id",
    ["vendor_name", "contact_mobile", "description", "kefu_phone"]
);

// post /vendor/product/auditAgain
async fn h_28(State(state): State<AdminState>, Json(q): Json<Pg>) -> ApiResult<()> {
    sqlx::query("UPDATE vendor_product SET audit_state = 0 WHERE id = $1")
        .bind(q.id.unwrap_or(0))
        .execute(&pool(&state))
        .await
        .map_err(db)?;
    ok(())
}

// post /vendor/product/batch
del_ids!(h_29, "vendor_product", "id");

// post /vendor/product/changeState
async fn h_30(State(state): State<AdminState>, Json(q): Json<Pg>) -> ApiResult<()> {
    sqlx::query("UPDATE vendor_product SET product_state = $1 WHERE id = $2")
        .bind(q.status.unwrap_or(1))
        .bind(q.id.unwrap_or(0))
        .execute(&pool(&state))
        .await
        .map_err(db)?;
    ok(())
}

// get /vendor/product/config
cfg_get!(h_31, "vendorProductConfig");

// post /vendor/product/copy
async fn h_32(State(state): State<AdminState>, Json(q): Json<Pg>) -> ApiResult<i64> {
    let r: i64 = sqlx::query_scalar("INSERT INTO vendor_product (product_name, product_category_id, product_state, sku_type, audit_state, pic_url, pic_thumb, pic_original, pic_large, is_recycle, vendor_id, create_time, update_time, create_by_id, update_by_id, is_del, product_sn_generate_type, product_brief, product_desc) SELECT CONCAT(product_name, ' 副本'), product_category_id, product_state, sku_type, 0, pic_url, pic_thumb, pic_original, pic_large, is_recycle, vendor_id, CAST(EXTRACT(EPOCH FROM NOW()) AS BIGINT), CAST(EXTRACT(EPOCH FROM NOW()) AS BIGINT), create_by_id, update_by_id, is_del, product_sn_generate_type, product_brief, product_desc FROM vendor_product WHERE id = $1 RETURNING id")
        .bind(q.id.unwrap_or(0)).fetch_one(&pool(&state)).await.map_err(db)?;
    ok(r)
}

// post /vendor/product/del
del_ids!(h_33, "vendor_product", "id");

// get /vendor/product/getParticiple
async fn h_34(_state: State<AdminState>, Query(q): Query<Pg>) -> ApiResult<Value> {
    // 分词服务未迁移：按空格/2字切分
    let kw = q.keyword.clone().unwrap_or_default();
    let parts: Vec<String> = kw
        .chars()
        .collect::<Vec<_>>()
        .chunks(2)
        .map(|c| c.iter().collect())
        .collect();
    ok(serde_json::json!({ "words": parts }))
}

// post /vendor/product/platform/audit
async fn h_35(State(state): State<AdminState>, Json(q): Json<Pg>) -> ApiResult<()> {
    let status = q.status.unwrap_or(1);
    if status == CHECK_STATUS_APPROVED {
        // 审核通过后的上架状态按平台设置 `vendorProductAuditShelfType` 决定
        // （原版有，Tigshop v5.8.65）：自动上架（默认/未配置）→ product_state=1，
        // 立即上架供平台或门店选品；手动上架 → product_state=0，只置审核通过，
        // 由供应商用既有 changeState 自行上架。判断时点为审核时的当前设置
        // （无快照）；配置读取失败按未配置处理（回落自动），不阻断审核。
        let raw = state
            .repos
            .system
            .config_client
            .find_config_val(CFG_VENDOR_PRODUCT_AUDIT_SHELF_TYPE, true)
            .await
            .ok()
            .flatten();
        let product_state = audit_result_product_status(
            CHECK_STATUS_APPROVED,
            parse_audit_shelf_type(raw.as_deref()),
        );
        sqlx::query(
            "UPDATE vendor_product SET audit_state = $1, product_state = $2 WHERE id = $3",
        )
        .bind(status)
        .bind(product_state)
        .bind(q.id.unwrap_or(0))
        .execute(&pool(&state))
        .await
        .map_err(db)?;
    } else {
        // 不通过等其它状态：只改审核状态，上架状态保持旧行为不动
        sqlx::query("UPDATE vendor_product SET audit_state = $1 WHERE id = $2")
            .bind(status)
            .bind(q.id.unwrap_or(0))
            .execute(&pool(&state))
            .await
            .map_err(db)?;
    }
    ok(())
}

// get /vendor/product/platform/getWaitingCheckedVendorCount
async fn h_36(State(state): State<AdminState>) -> ApiResult<Value> {
    let (c,): (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT vendor_id) FROM vendor_product WHERE audit_state = 0",
    )
    .fetch_one(&pool(&state))
    .await
    .map_err(db)?;
    ok(serde_json::json!({ "count": c }))
}

// post /vendor/product/platform/pageList
async fn h_37(State(state): State<AdminState>, Query(q): Query<Pg>) -> ApiResult<Value> {
    let (p, s, off) = page_of(&q);
    let kw = format!("%{}%", q.keyword.clone().unwrap_or_default().trim());
    let (total,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM vendor_product WHERE audit_state = 0 AND product_name ILIKE $1",
    )
    .bind(&kw)
    .fetch_one(&pool(&state))
    .await
    .map_err(db)?;
    let rows: Vec<(Value,)> = sqlx::query_as("SELECT json_build_object('id', id, 'productName', product_name, 'vendorId', vendor_id, 'auditState', audit_state, 'picThumb', pic_thumb) FROM vendor_product WHERE audit_state = 0 AND product_name ILIKE $1 ORDER BY id DESC LIMIT $2 OFFSET $3")
        .bind(&kw).bind(s).bind(off).fetch_all(&pool(&state)).await.map_err(db)?;
    ok(
        serde_json::json!({"records": rows.into_iter().map(|(v,)| v).collect::<Vec<_>>(), "total": total, "page": p, "size": s}),
    )
}

// get /vendor/product/platform/saleStatus
async fn h_vendor_sale_status(
    State(state): State<AdminState>,
    Query(q): Query<Pg>,
) -> ApiResult<ProductSaleState> {
    let vendor_product_id =
        q.id.or(q.product_id)
            .ok_or_else(|| Error::biz(400, "供应商商品不存在".to_string()))? as i32;
    ok(state
        .repos
        .catalog
        .product_sale
        .vendor_setting(vendor_product_id)
        .await?)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VendorSaleParam {
    vendor_product_id: i32,
    sale_status: i32,
    sale_ban_time: Option<i64>,
    sale_recover_time: Option<i64>,
}

// post /vendor/product/platform/saleStatus
async fn h_vendor_sale_save(
    State(state): State<AdminState>,
    Json(q): Json<VendorSaleParam>,
) -> ApiResult<()> {
    state
        .repos
        .catalog
        .product_sale
        .save_vendor(
            q.vendor_product_id,
            q.sale_status,
            q.sale_ban_time,
            q.sale_recover_time,
        )
        .await?;
    ok(())
}

// post /vendor/product/recycle
async fn h_38(State(state): State<AdminState>, Json(q): Json<Pg>) -> ApiResult<()> {
    sqlx::query("UPDATE vendor_product SET is_recycle = 1 WHERE id = $1")
        .bind(q.id.unwrap_or(0))
        .execute(&pool(&state))
        .await
        .map_err(db)?;
    ok(())
}

// get /vendor/product/shippingTplList
async fn h_39(State(state): State<AdminState>) -> ApiResult<Value> {
    let rows: Vec<(Value,)> = sqlx::query_as("SELECT json_build_object('shippingTplId', shipping_tpl_id, 'tplName', shipping_tpl_name) FROM shipping_tpl").fetch_all(&pool(&state)).await.map_err(db)?;
    ok(serde_json::json!({"records": rows.into_iter().map(|(v,)| v).collect::<Vec<_>>()}))
}

// post /vendor/product/updateField
upd_field!(
    h_40,
    "vendor_product",
    "id",
    [
        "product_name",
        "product_state",
        "audit_state",
        "product_brief"
    ]
);

// get /vendor/productInventoryLog/list
list_pg!(h_41, "vendor_product_stock_record", "product_stock_id", "'product_stock_id', product_stock_id, 'stock_code', stock_code, 'in_or_out', in_or_out, 'type', type, 'add_time', add_time, 'add_by', add_by, 'comment', comment, 'vendor_id', vendor_id", "stock_code");

// get /vendor/stock/adjustDetail
detail_one!(h_42, "vendor_product_stock_record_detail", "record_detail_id", "'record_detail_id', record_detail_id, 'stock_record_id', stock_record_id, 'vendor_product_id', vendor_product_id, 'vendor_product_sku_id', vendor_product_sku_id, 'quantity', quantity");

// get /vendor/stock/getRecordDetail
detail_one!(h_43, "vendor_product_stock_record", "product_stock_id", "'product_stock_id', product_stock_id, 'stock_code', stock_code, 'in_or_out', in_or_out, 'type', type, 'add_time', add_time, 'add_by', add_by, 'comment', comment, 'vendor_id', vendor_id");

// get /vendor/stock/page
list_pg!(h_44, "vendor_product_stock_record", "product_stock_id", "'product_stock_id', product_stock_id, 'stock_code', stock_code, 'in_or_out', in_or_out, 'type', type, 'add_time', add_time, 'add_by', add_by, 'comment', comment, 'vendor_id', vendor_id", "stock_code");

// get /vendor/stock/record/page
list_pg!(h_45, "vendor_product_stock_record", "product_stock_id", "'product_stock_id', product_stock_id, 'stock_code', stock_code, 'in_or_out', in_or_out, 'type', type, 'add_time', add_time, 'add_by', add_by, 'comment', comment, 'vendor_id', vendor_id", "stock_code");

// get /vendor/stock/stockDetail
detail_one!(h_46, "vendor_product", "id", "'id', id, 'product_name', product_name, 'product_brand_id', product_brand_id, 'product_category_id', product_category_id, 'product_sn_generate_type', product_sn_generate_type, 'product_brief', product_brief, 'product_state', product_state, 'sku_type', sku_type, 'product_desc', product_desc, 'audit_state', audit_state, 'pic_url', pic_url, 'pic_thumb', pic_thumb, 'pic_original', pic_original, 'pic_large', pic_large, 'is_recycle', is_recycle, 'vendor_id', vendor_id");

// get /vendor/stock/getAddByList
list_pg!(h_47, "admin_user_vendor", "id", "'id', id, 'admin_id', admin_id, 'user_id', user_id, 'vendor_id', vendor_id, 'username', username, 'email', email, 'avatar', avatar, 'auth_list', auth_list, 'is_using', is_using, 'is_admin', is_admin, 'add_time', add_time, 'role_id', role_id", "admin_id");

// get /vendor/stock/downloadTemplate
async fn h_48() -> ApiResult<&'static str> {
    // 简化：CSV 表头模板
    ok("skuSn,stock,price\n")
}

// get /vendor/stock/getImportErrorData
async fn h_49() -> ApiResult<Value> {
    ok(serde_json::json!({"records": []}))
}

// post /vendor/stock/batchImport
async fn h_50(_state: State<AdminState>) -> ApiResult<()> {
    Err(nextrade_security::web::AppError(Error::Validation(
        "批量导入未迁移（简化实现）".into(),
    )))
}

// post /vendor/stock/stockIn
async fn h_51(State(state): State<AdminState>, Json(q): Json<Pg>) -> ApiResult<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    sqlx::query("INSERT INTO vendor_product_stock_record (stock_code, in_or_out, type, add_time, add_by, comment, vendor_id) VALUES ('MANUAL', 1, 0, $1, 'admin', 'POS/后台入库', $2)")
        .bind(now).bind(q.vendor_id.unwrap_or(0)).execute(&pool(&state)).await.map_err(db)?;
    ok(())
}

// post /vendor/stock/stockOut
async fn h_52(State(state): State<AdminState>, Json(q): Json<Pg>) -> ApiResult<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    sqlx::query("INSERT INTO vendor_product_stock_record (stock_code, in_or_out, type, add_time, add_by, comment, vendor_id) VALUES ('MANUAL', 2, 0, $1, 'admin', '后台出库', $2)")
        .bind(now).bind(q.vendor_id.unwrap_or(0)).execute(&pool(&state)).await.map_err(db)?;
    ok(())
}

// post /vendor/stock/stockAdjust
async fn h_53(State(state): State<AdminState>, Json(q): Json<Pg>) -> ApiResult<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    sqlx::query("INSERT INTO vendor_product_stock_record (stock_code, in_or_out, type, add_time, add_by, comment, vendor_id) VALUES ('ADJ', 1, 1, $1, 'admin', '库存调整', $2)")
        .bind(now).bind(q.vendor_id.unwrap_or(0)).execute(&pool(&state)).await.map_err(db)?;
    ok(())
}

// ————————————————————— 供应商员工 / 资金账户 / 商品 —————————————————————

fn json_i64(body: &Value, key: &str) -> i64 {
    body.get(key).and_then(Value::as_i64).unwrap_or(0)
}
fn json_str(body: &Value, key: &str) -> String {
    body.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

async fn h_vendor_user_create(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<i64> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let r: i32 = sqlx::query_scalar(
        "INSERT INTO admin_user_vendor (admin_id, user_id, vendor_id, username, email, avatar, auth_list, is_using, is_admin, add_time, role_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id",
    )
    .bind(json_i64(&body, "adminId")).bind(json_i64(&body, "userId")).bind(json_i64(&body, "vendorId"))
    .bind(json_str(&body, "username")).bind(json_str(&body, "email")).bind(json_str(&body, "avatar"))
    .bind(json_str(&body, "authList")).bind(json_i64(&body, "isUsing")).bind(json_i64(&body, "isAdmin"))
    .bind(now).bind(json_i64(&body, "roleId"))
    .fetch_one(&pool(&state)).await.map_err(db)?;
    ok(i64::from(r))
}

async fn h_vendor_user_update(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<()> {
    sqlx::query(
        "UPDATE admin_user_vendor SET admin_id = $1, user_id = $2, vendor_id = $3, username = $4, email = $5, avatar = $6, auth_list = $7, is_using = $8, is_admin = $9, role_id = $10 WHERE id = $11",
    )
    .bind(json_i64(&body, "adminId")).bind(json_i64(&body, "userId")).bind(json_i64(&body, "vendorId"))
    .bind(json_str(&body, "username")).bind(json_str(&body, "email")).bind(json_str(&body, "avatar"))
    .bind(json_str(&body, "authList")).bind(json_i64(&body, "isUsing")).bind(json_i64(&body, "isAdmin"))
    .bind(json_i64(&body, "roleId")).bind(json_i64(&body, "id"))
    .execute(&pool(&state)).await.map_err(db)?;
    ok(())
}

async fn h_vendor_user_detail(
    State(state): State<AdminState>,
    Query(q): Query<Pg>,
) -> ApiResult<Value> {
    let row: Option<(Value,)> = sqlx::query_as(
        "SELECT json_build_object('id', id, 'adminId', admin_id, 'userId', user_id, 'vendorId', vendor_id, 'username', username, 'email', email, 'avatar', avatar, 'authList', auth_list, 'isUsing', is_using, 'isAdmin', is_admin, 'addTime', add_time, 'roleId', role_id) FROM admin_user_vendor WHERE id = $1",
    ).bind(q.id.unwrap_or(0)).fetch_optional(&pool(&state)).await.map_err(db)?;
    ok(row.map(|(v,)| v).unwrap_or(Value::Null))
}

async fn h_vendor_account_create(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<i64> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let r: i32 = sqlx::query_scalar(
        "INSERT INTO vendor_account (vendor_id, account_type, account_name, account_no, bank_name, bank_branch, add_time) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING account_id",
    )
    .bind(json_i64(&body, "vendorId")).bind(json_i64(&body, "accountType"))
    .bind(json_str(&body, "accountName")).bind(json_str(&body, "accountNo"))
    .bind(json_str(&body, "bankName")).bind(json_str(&body, "bankBranch")).bind(now)
    .fetch_one(&pool(&state)).await.map_err(db)?;
    ok(i64::from(r))
}

async fn h_vendor_account_update(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<()> {
    sqlx::query(
        "UPDATE vendor_account SET vendor_id = $1, account_type = $2, account_name = $3, account_no = $4, bank_name = $5, bank_branch = $6 WHERE account_id = $7",
    )
    .bind(json_i64(&body, "vendorId")).bind(json_i64(&body, "accountType"))
    .bind(json_str(&body, "accountName")).bind(json_str(&body, "accountNo"))
    .bind(json_str(&body, "bankName")).bind(json_str(&body, "bankBranch"))
    .bind(json_i64(&body, "accountId"))
    .execute(&pool(&state)).await.map_err(db)?;
    ok(())
}

async fn h_vendor_account_detail(
    State(state): State<AdminState>,
    Query(q): Query<Pg>,
) -> ApiResult<Value> {
    let row: Option<(Value,)> = sqlx::query_as(
        "SELECT json_build_object('accountId', account_id, 'vendorId', vendor_id, 'accountType', account_type, 'accountName', account_name, 'accountNo', account_no, 'bankName', bank_name, 'bankBranch', bank_branch, 'addTime', add_time) FROM vendor_account WHERE account_id = $1",
    ).bind(q.id.unwrap_or(0)).fetch_optional(&pool(&state)).await.map_err(db)?;
    ok(row.map(|(v,)| v).unwrap_or(Value::Null))
}

async fn h_vendor_product_create(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<i64> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let pid = json_i64(&body, "productId");
    if pid > 0 {
        sqlx::query("UPDATE product SET product_name = $1, product_price = $2 WHERE product_id = $3")
            .bind(json_str(&body, "productName"))
            .bind(json_str(&body, "productPrice"))
            .bind(pid)
            .execute(&pool(&state))
            .await
            .map_err(db)?;
        return ok(pid);
    }
    let r: i32 = sqlx::query_scalar(
        "INSERT INTO product (product_name, product_price, shop_id, vendor_id, add_time) VALUES ($1, $2, $3, $4, $5) RETURNING product_id",
    )
    .bind(json_str(&body, "productName")).bind(json_str(&body, "productPrice"))
    .bind(json_i64(&body, "shopId")).bind(json_i64(&body, "vendorId")).bind(now)
    .fetch_one(&pool(&state)).await.map_err(db)?;
    ok(i64::from(r))
}

async fn h_vendor_product_detail(
    State(state): State<AdminState>,
    Query(q): Query<Pg>,
) -> ApiResult<Value> {
    let row: Option<(Value,)> = sqlx::query_as(
        "SELECT json_build_object('productId', product_id, 'productName', product_name, 'productPrice', product_price, 'picThumb', pic_thumb, 'shopId', shop_id, 'vendorId', vendor_id, 'addTime', add_time) FROM product WHERE product_id = $1",
    ).bind(q.product_id.or(q.id).unwrap_or(0)).fetch_optional(&pool(&state)).await.map_err(db)?;
    ok(row.map(|(v,)| v).unwrap_or(Value::Null))
}

async fn h_vendor_update(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<()> {
    let id = body
        .get("vendorId")
        .or_else(|| body.get("id"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    sqlx::query("UPDATE vendor SET vendor_name=COALESCE(NULLIF($1,''),vendor_name), contact_mobile=COALESCE(NULLIF($2,''),contact_mobile), status=COALESCE($3,status) WHERE vendor_id=$4")
        .bind(body.get("vendorName").and_then(Value::as_str).unwrap_or(""))
        .bind(body.get("contactMobile").and_then(Value::as_str).unwrap_or(""))
        .bind(body.get("status").and_then(Value::as_i64).map(|v| v as i32))
        .bind(id).execute(&pool(&state)).await.map_err(db)?;
    ok(())
}

async fn h_vendor_product_update(
    State(state): State<AdminState>,
    Json(body): Json<Value>,
) -> ApiResult<()> {
    let id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    sqlx::query("UPDATE vendor_product SET product_name=COALESCE(NULLIF($1,''),product_name), product_state=COALESCE($2,product_state), product_brief=COALESCE(NULLIF($3,''),product_brief) WHERE id=$4")
        .bind(body.get("productName").and_then(Value::as_str).unwrap_or(""))
        .bind(body.get("productState").and_then(Value::as_i64).map(|v| v as i32))
        .bind(body.get("productBrief").and_then(Value::as_str).unwrap_or(""))
        .bind(id).execute(&pool(&state)).await.map_err(db)?;
    ok(())
}

/// 域路由（挂载于 /adminapi 下）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/vendor/vendorWithdraw/list", get(h_0).post(h_0))
        .route("/vendor/vendorWithdraw/detail", get(h_1))
        .route("/vendor/vendorWithdraw/create", post(h_2))
        .route("/vendor/vendorWithdraw/audit", post(h_3))
        .route("/vendor/vendorWithdraw/config", get(h_4))
        .route("/vendor/vendorWithdraw/summaryList", get(h_5))
        .route("/vendor/vendorWithdraw/uploadPayVoucher", post(h_6))
        .route("/vendor/vendorShopBind/list", get(h_7).post(h_7))
        .route("/vendor/vendorShopBind/merchantList", get(h_8))
        .route("/vendor/vendorShopBind/orderList", get(h_9))
        .route("/vendor/vendorShopBind/productList", get(h_10))
        .route("/vendor/vendorShopBind/shopList", get(h_11))
        .route("/vendor/adminVendorUser/list", get(h_12).post(h_12))
        .route("/vendor/adminVendorUser/create", post(h_vendor_user_create))
        .route("/vendor/adminVendorUser/update", post(h_vendor_user_update))
        .route(
            "/vendor/adminVendorUser/modifyUser",
            post(h_vendor_user_update),
        )
        .route("/vendor/adminVendorUser/info", get(h_vendor_user_detail))
        .route("/vendor/adminVendorUser/detail", get(h_vendor_user_detail))
        .route("/vendor/adminVendorUser/batch", post(h_13))
        .route("/vendor/adminVendorUser/del", post(h_14))
        .route("/vendor/adminVendorAccount/index", get(h_15))
        .route("/vendor/adminVendorAccount/listFund", get(h_16))
        .route("/vendor/adminVendorAccount/logList", get(h_17))
        .route("/vendor/vendorAccount/index", get(h_18))
        .route("/vendor/vendorAccount/list", get(h_19))
        .route("/vendor/vendorAccount/logList", get(h_20))
        .route("/vendor/vendorAccount/del", post(h_21))
        .route(
            "/vendor/vendorAccount/create",
            post(h_vendor_account_create),
        )
        .route(
            "/vendor/vendorAccount/update",
            post(h_vendor_account_update),
        )
        .route("/vendor/vendorAccount/detail", get(h_vendor_account_detail))
        .route("/vendor/product/create", post(h_vendor_product_create))
        .route("/vendor/product/detail", get(h_vendor_product_detail))
        .route(
            "/vendor/product/platform/detail",
            get(h_vendor_product_detail),
        )
        .route("/vendor/vendorAccount/config", get(h_22))
        .route("/vendor/vendorAccount/applyShopAgreement", get(h_23))
        .route("/vendor/vendor/delVendor", post(h_24))
        .route("/vendor/vendor/staffShow", get(h_25))
        .route("/vendor/vendor/updateField", post(h_26))
        .route("/vendor/vendor/updateInfo", post(h_27))
        .route("/vendor/vendor/update", post(h_vendor_update))
        .route("/vendor/product/auditAgain", post(h_28))
        .route("/vendor/product/batch", post(h_29))
        .route("/vendor/product/changeState", post(h_30))
        .route("/vendor/product/config", get(h_31))
        .route("/vendor/product/copy", post(h_32))
        .route("/vendor/product/del", post(h_33))
        .route("/vendor/product/getParticiple", get(h_34))
        .route("/vendor/product/platform/audit", post(h_35))
        .route(
            "/vendor/product/platform/getWaitingCheckedVendorCount",
            get(h_36),
        )
        .route("/vendor/product/platform/pageList", get(h_37).post(h_37))
        .route(
            "/vendor/product/platform/saleStatus",
            get(h_vendor_sale_status).post(h_vendor_sale_save),
        )
        .route("/vendor/product/recycle", post(h_38))
        .route("/vendor/product/shippingTplList", get(h_39))
        .route("/vendor/product/updateField", post(h_40))
        .route("/vendor/product/update", post(h_vendor_product_update))
        .route("/vendor/productInventoryLog/list", get(h_41))
        .route("/vendor/stock/adjustDetail", get(h_42))
        .route("/vendor/stock/getRecordDetail", get(h_43))
        .route("/vendor/stock/page", get(h_44))
        .route("/vendor/stock/record/page", get(h_45))
        .route("/vendor/stock/stockDetail", get(h_46))
        .route("/vendor/stock/getAddByList", get(h_47))
        .route("/vendor/stock/downloadTemplate", get(h_48))
        .route("/vendor/stock/getImportErrorData", get(h_49).post(h_49))
        .route("/vendor/stock/batchImport", post(h_50))
        .route("/vendor/stock/stockIn", post(h_51))
        .route("/vendor/stock/stockOut", post(h_52))
        .route("/vendor/stock/stockAdjust", post(h_53))
}
