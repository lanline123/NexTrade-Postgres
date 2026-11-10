//! 前台用户售后域（trade::aftersales_client）仓储：`aftersales` / `aftersales_item` /
//! `aftersales_log`，并读取 `order` / `order_item` / `user` / `product` /
//! `paylog` / `config` / `order_config` 的展示与校验所需字段。
//!
//! 列名严格对齐权威 schema（`python tools/schema_parser.py --table <表名>`）：
//! - `aftersales`：`aftersale_id` / `aftersale_type` / `status` / `pics` /
//!   `description` / `reply` / `add_time` / `tracking_no` / `logistics_name` /
//!   `return_address` / `aftersale_reason` / `aftersales_sn` / `order_id` /
//!   `user_id` / `refund_amount` / `shop_id` / `audit_time` / `deal_time` /
//!   `final_time` / `return_goods_tip` / `vendor_id` / `close_type`
//! - `aftersales_item`：`aftersales_item_id` / `order_item_id` / `number` / `aftersale_id`
//! - `aftersales_log`：`log_id` / `aftersale_id` / `log_info` / `add_time` /
//!   `admin_name` / `refund_money` / `refund_type` / `refund_desc` /
//!   `user_name` / `return_pic` / `shop_id` / `vendor_id`
//! - `config`：`biz_code` / `biz_val` / `is_del`（替代 Java `SettingsHelper`，
//!   Java 侧 `@TableLogic isDel` 等价于 `is_del = 0`）
//! - `order_config`：`shop_id` / `code` / `data`
//!
//! 注意：`order` 是 MySQL 保留字，SQL 中必须使用反引号。
//! 查询统一使用运行时 `sqlx::query_as`（不使用 `query!` 宏）。

use std::collections::HashMap;

use sqlx::PgPool;
use nextrade_core::entities::{
    Aftersales, AftersalesItem, AftersalesLog, Order, OrderItem, Product,
};
use nextrade_core::models::trade::aftersales_client::AftersalesListQuery;
use nextrade_core::{Error, Result};

/// `aftersales` 表全部列（顺序与 [`Aftersales`] 实体字段一致）。
const AFTERSALES_COLUMNS: &str = "aftersale_id, aftersale_type, status, pics, description, \
     reply, add_time, tracking_no, logistics_name, return_address, aftersale_reason, \
     aftersales_sn, order_id, user_id, refund_amount, shop_id, audit_time, deal_time, \
     final_time, return_goods_tip, vendor_id, close_type";

/// \"aftersales_log\" 表全部列（顺序与 [\"AftersalesLog\"] 实体字段一致）。
const AFTERSALES_LOG_COLUMNS: &str = "log_id, aftersale_id, log_info, add_time, admin_name, \
     refund_money, refund_type, refund_desc, user_name, return_pic, shop_id, vendor_id";

/// 仓储错误映射：SQLx 错误 → [`nextrade_core::Error`]。
///
/// 写法与同 crate 其他仓储（`shop.rs` / `aftersales.rs` 等）保持一致：
/// `RowNotFound` 转为 [`Error::NotFound`]，其余转为 [`Error::Database`]。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("售后单不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

/// 待绑定参数（按 \"conds\" 顺序 1:1 对应）。
#[derive(Debug, Clone)]
enum Bind {
    I32(i32),
    Str(String),
}

/// 把 \"Bind\" 列表按顺序绑定到查询上。
macro_rules! bind_all {
    ($q:expr, $binds:expr) => {{
        let mut q = $q;
        for b in $binds.iter() {
            q = match b {
                Bind::I32(v) => q.bind(*v),
                Bind::Str(v) => q.bind(v.as_str()),
            };
        }
        q
    }};
}

/// 去除首尾空白，空串视作 \"None\"（对应 Java `StrUtil.isNotBlank`）。
fn non_blank(v: &Option<String>) -> Option<String> {
    v.as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

/// Java `OrderType.PIN`：拼团（被排除）。
const EXCLUDED_ORDER_TYPE_PIN: i32 = 2;
/// Java `OrderType.PAID`：付费商品（被排除）。
const EXCLUDED_ORDER_TYPE_PAID: i32 = 7;
/// Java `OrderType.CARD`：卡密（被排除）。
const EXCLUDED_ORDER_TYPE_CARD: i32 = 8;
/// Java `OrderType.GIFT_CARD`：礼品卡（被排除）。
const EXCLUDED_ORDER_TYPE_GIFT_CARD: i32 = 10;

/// Java `OrderStatusEnum.CONFIRMED`：已确认，待发货。
const ORDER_STATUS_CONFIRMED: i32 = 1;
/// Java `OrderStatusEnum.PROCESSING`：处理中。
const ORDER_STATUS_PROCESSING: i32 = 2;
/// Java `OrderStatusEnum.COMPLETED`：已完成。
const ORDER_STATUS_COMPLETED: i32 = 5;

#[derive(Clone)]
pub struct AftersalesClientRepository {
    pool: PgPool,
}

/// \"aftersales_log\" 读取行。
///
/// 与自动生成实体 [\"AftersalesLog\"] 的唯一差别是 \"add_time\"：
/// schema 中该列为 `int unsigned`，故此处用 \"i32\" 解码；
/// 实体把 \"add_time\" 声明为 \"String\"（Java 侧写入 `getCurrentTime().toString()`），
/// 直接用实体做 \"query_as\" 会在运行时解码失败。
#[derive(Debug, Clone, sqlx::FromRow)]
#[sqlx(rename_all = "snake_case")]
pub struct AftersalesLogRow {
    pub log_id: i32,
    pub aftersale_id: i32,
    pub log_info: String,
    pub add_time: i32,
    pub admin_name: String,
    pub refund_money: rust_decimal::Decimal,
    pub refund_type: i32,
    pub refund_desc: String,
    pub user_name: Option<String>,
    pub return_pic: Option<String>,
    pub shop_id: Option<i32>,
    pub vendor_id: Option<i32>,
}

impl AftersalesClientRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 配置项（config） —————————————————————

    /// 读取系统配置原值（对应 Java `SettingsHelper#getRaw`：
    /// `config` 中按 `biz_code` 取一条未删除记录，取不到则由调用方用
    /// `SettingsEnum` 的 `defaultValue` 兜底）。
    pub async fn find_config_raw(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1",
        )
        .bind(biz_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }

    /// 店铺订单配置原始 JSON（对应 Java `orderConfigMapper.selectOne(shop_id, code)`）。
    pub async fn find_order_config(&self, shop_id: i32, code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT data FROM order_config WHERE shop_id = $1 AND code = $2 LIMIT 1",
        )
        .bind(shop_id)
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }

    // ————————————————————— 售后单 —————————————————————

    /// 按主键查询售后单。
    pub async fn find_aftersales(&self, aftersale_id: i32) -> Result<Option<Aftersales>> {
        let sql = crate::db::to_pg_placeholders(&format!("SELECT {AFTERSALES_COLUMNS} FROM aftersales WHERE aftersale_id = ? LIMIT 1"));
        sqlx::query_as::<_, Aftersales>(&sql)
            .bind(aftersale_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 售后申请记录分页（对应 Java `AftersalesServiceImpl#list` 的单表部分）。
    ///
    /// 与 Java \"buildSortOrder\" 一致：\"BasePage\" 未传 \"sortField\" 时按**主键降序**。
    ///
    /// \"keyword\"（订单号）/ \"outTradeNo\"（交易单号）在 Java 里是「先查 \"order\"
    /// 得到 orderId 列表，再用 \"IN\" 过滤」；这里等价地改写为
    /// `JOIN \`order\``（\"order_id\" 是主键，join 不会放大行数），
    /// 避免子查询出现在单表校验语料里造成列名误判。
    pub async fn paginate_aftersales(
        &self,
        q: &AftersalesListQuery,
        user_id: i32,
    ) -> Result<(u64, Vec<Aftersales>)> {
        let mut conds: Vec<String> = vec!["a.user_id = ?".into()];
        let mut binds: Vec<Bind> = vec![Bind::I32(user_id)];

        // Java：`aftersaleType` 仅在为 1/2 时参与过滤（`aftersalesTypeFlag`）
        if let Some(t) = q
            .aftersale_type
            .filter(|t| *t == AFTERSALES_TYPE_RETURN || *t == AFTERSALES_TYPE_PAY_RETURN)
        {
            conds.push("a.aftersale_type = ?".into());
            binds.push(Bind::I32(t));
        }
        // Java：`status != null && status > 0` 才参与过滤
        if let Some(s) = q.status.filter(|s| *s > 0) {
            conds.push("a.status = ?".into());
            binds.push(Bind::I32(s));
        }
        if let Some(sn) = non_blank(&q.aftersales_sn) {
            conds.push("a.aftersales_sn ILIKE ?".into());
            binds.push(Bind::Str(format!("%{sn}%")));
        }

        let keyword = non_blank(&q.keyword);
        let out_trade_no = non_blank(&q.out_trade_no);
        let join_order = keyword.is_some() || out_trade_no.is_some();
        if let Some(kw) = keyword {
            conds.push("o.order_sn ILIKE ?".into());
            binds.push(Bind::Str(format!("%{kw}%")));
        }
        // Java `outTradeNo`：按 out_trade_no 模糊过滤
        if let Some(no) = out_trade_no {
            conds.push("o.out_trade_no ILIKE ?".into());
            binds.push(Bind::Str(format!("%{no}%")));
        }

        let from_sql = if join_order {
            "aftersales a JOIN \"order\" o ON o.order_id = a.order_id"
        } else {
            "aftersales a"
        };
        let where_sql = format!(" WHERE {}", conds.join(" AND "));
        let (page_size, offset) = (q.page_size(), (q.page_no() - 1) * q.page_size());

        let total_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM {from_sql}{where_sql}"));
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&total_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let cols: Vec<String> = AFTERSALES_COLUMNS
            .split(',')
            .map(|c| format!("a.{}", c.trim()))
            .collect();
        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {} FROM {from_sql}{where_sql} ORDER BY a.aftersale_id DESC LIMIT ? OFFSET ?",
            cols.join(", ")
        ));
        let rows = bind_all!(sqlx::query_as::<_, Aftersales>(&rows_sql), binds)
            .bind(page_size as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 更新售后单（对应 Java `updateById`；整体覆盖除 `pics` 外的可写字段）。
    ///
    /// `pics` 单独更新，以对齐 Java `afterSalesApplyUpdate` 中
    /// 「`pics != null` 才覆盖」的语义（由服务层决定是否传入）。
    pub async fn update_aftersales(&self, a: &Aftersales) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE aftersales SET aftersale_type = $1, status = $2, pics = $3, description = $4, \
             reply = $5, tracking_no = $6, logistics_name = $7, return_address = $8, \
             aftersale_reason = $9, refund_amount = $10, audit_time = $11, deal_time = $12, \
             final_time = $13, return_goods_tip = $14, close_type = $15, vendor_id = $16 \
             WHERE aftersale_id = $17",
        )
        .bind(a.aftersale_type)
        .bind(a.status)
        .bind(&a.pics)
        .bind(&a.description)
        .bind(&a.reply)
        .bind(&a.tracking_no)
        .bind(&a.logistics_name)
        .bind(&a.return_address)
        .bind(&a.aftersale_reason)
        .bind(a.refund_amount)
        .bind(a.audit_time)
        .bind(a.deal_time)
        .bind(a.final_time)
        .bind(&a.return_goods_tip)
        .bind(a.close_type)
        .bind(a.vendor_id)
        .bind(a.aftersale_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 写出售后单（对应 Java \"afterSalesApply\" 的 `this.save(aftersales)`），
    /// 返回自增 \"aftersale_id\"。
    pub async fn insert_aftersales(&self, a: &Aftersales) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(AFTERSALES_INSERT)
            .bind(a.aftersale_type)
            .bind(a.status)
            .bind(&a.pics)
            .bind(&a.description)
            .bind(&a.reply)
            .bind(a.add_time)
            .bind(&a.tracking_no)
            .bind(&a.logistics_name)
            .bind(&a.return_address)
            .bind(&a.aftersale_reason)
            .bind(&a.aftersales_sn)
            .bind(a.order_id)
            .bind(a.user_id)
            .bind(a.refund_amount)
            .bind(a.shop_id)
            .bind(a.audit_time)
            .bind(a.deal_time)
            .bind(a.final_time)
            .bind(&a.return_goods_tip)
            .bind(a.vendor_id)
            .bind(a.close_type)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r)
    }

    /// 在同一事务内写入「售后单 + 售后明细 + 售后日志」，返回 \"aftersale_id\"。
    ///
    /// 对应 Java \"afterSalesApply\" 的 `@Transactional` 三步写入。
    pub async fn insert_aftersales_bundle(
        &self,
        a: &Aftersales,
        items: &[AftersalesItem],
        log: &AftersalesLog,
    ) -> Result<i32> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;

        let r: i32 = sqlx::query_scalar(AFTERSALES_INSERT)
            .bind(a.aftersale_type)
            .bind(a.status)
            .bind(&a.pics)
            .bind(&a.description)
            .bind(&a.reply)
            .bind(a.add_time)
            .bind(&a.tracking_no)
            .bind(&a.logistics_name)
            .bind(&a.return_address)
            .bind(&a.aftersale_reason)
            .bind(&a.aftersales_sn)
            .bind(a.order_id)
            .bind(a.user_id)
            .bind(a.refund_amount)
            .bind(a.shop_id)
            .bind(a.audit_time)
            .bind(a.deal_time)
            .bind(a.final_time)
            .bind(&a.return_goods_tip)
            .bind(a.vendor_id)
            .bind(a.close_type)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_err)?;
        let aftersale_id = r;

        for it in items {
            sqlx::query(
                "INSERT INTO aftersales_item (order_item_id, number, aftersale_id) VALUES ($1, $2, $3)",
            )
            .bind(it.order_item_id)
            .bind(it.number)
            .bind(aftersale_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        }

        insert_log(&mut tx, aftersale_id, log).await?;
        tx.commit().await.map_err(map_err)?;
        Ok(aftersale_id)
    }

    /// 同一事务内：整体覆盖售后单 + 删除旧明细 + 写新明细 + 追加日志。
    ///
    /// 对应 Java \"afterSalesApplyUpdate\"（`@Transactional`）。
    pub async fn update_aftersales_with_items_and_log(
        &self,
        a: &Aftersales,
        items: &[AftersalesItem],
        log: &AftersalesLog,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;

        sqlx::query(
            "UPDATE aftersales SET aftersale_type = $1, status = $2, pics = $3, description = $4, \
             reply = $5, tracking_no = $6, logistics_name = $7, return_address = $8, \
             aftersale_reason = $9, refund_amount = $10, audit_time = $11, deal_time = $12, \
             final_time = $13, return_goods_tip = $14, close_type = $15, vendor_id = $16 \
             WHERE aftersale_id = $17",
        )
        .bind(a.aftersale_type)
        .bind(a.status)
        .bind(&a.pics)
        .bind(&a.description)
        .bind(&a.reply)
        .bind(&a.tracking_no)
        .bind(&a.logistics_name)
        .bind(&a.return_address)
        .bind(&a.aftersale_reason)
        .bind(a.refund_amount)
        .bind(a.audit_time)
        .bind(a.deal_time)
        .bind(a.final_time)
        .bind(&a.return_goods_tip)
        .bind(a.close_type)
        .bind(a.vendor_id)
        .bind(a.aftersale_id)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;

        sqlx::query("DELETE FROM aftersales_item WHERE aftersale_id = $1")
            .bind(a.aftersale_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;

        for it in items {
            sqlx::query(
                "INSERT INTO aftersales_item (order_item_id, number, aftersale_id) VALUES ($1, $2, $3)",
            )
            .bind(it.order_item_id)
            .bind(it.number)
            .bind(a.aftersale_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        }

        insert_log(&mut tx, a.aftersale_id, log).await?;
        tx.commit().await.map_err(map_err)?;
        Ok(())
    }

    /// 写入一条售后日志（对应 Java `aftersalesLogMapper.insert` / \"save\"），
    /// 返回自增 \"log_id\"。
    pub async fn insert_aftersales_log(&self, log: &AftersalesLog) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(&format!("{} RETURNING log_id", AFTERSALES_LOG_INSERT))
            .bind(log.aftersale_id)
            .bind(&log.log_info)
            .bind(log.add_time)
            .bind(&log.admin_name)
            .bind(log.refund_money)
            .bind(log.refund_type)
            .bind(&log.refund_desc)
            .bind(&log.user_name)
            .bind(&log.return_pic)
            .bind(log.shop_id)
            .bind(log.vendor_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r)
    }

    // ————————————————————— 售后明细 —————————————————————

    /// 按售后ID查明细（对应 Java `getItemByIds(ids, isClientRequest)`）。
    ///
    /// \"order_item_id\" 升序，保证 \"sku_value\" 拼接顺序稳定。
    pub async fn find_items(&self, aftersale_id: i32) -> Result<Vec<AftersalesItem>> {
        sqlx::query_as::<_, AftersalesItem>(
            "SELECT aftersales_item_id, order_item_id, number, aftersale_id \
             FROM aftersales_item WHERE aftersale_id = $1 ORDER BY aftersales_item_id ASC",
        )
        .bind(aftersale_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 批量按售后ID查明细，`aftersale_id -> 明细列表`。
    pub async fn find_items_by_aftersale_ids(
        &self,
        aftersale_ids: &[i32],
    ) -> Result<HashMap<i32, Vec<AftersalesItem>>> {
        if aftersale_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let ph = vec!["?"; aftersale_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT aftersales_item_id, order_item_id, number, aftersale_id \
               FROM aftersales_item WHERE aftersale_id IN ({ph}) ORDER BY aftersales_item_id ASC"
        ));
        let mut q = sqlx::query_as::<_, AftersalesItem>(&sql);
        for id in aftersale_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        let mut out: HashMap<i32, Vec<AftersalesItem>> = HashMap::new();
        for it in rows {
            if let Some(aid) = it.aftersale_id {
                out.entry(aid).or_default().push(it);
            }
        }
        Ok(out)
    }

    /// 按订单项统计「有效状态」售后占用数量（`order_item_id -> sum(number)`）。
    ///
    /// 对应 Java `validateOrderQuantity` 中的 `getValidNumber`（仅订单维度求和）。
    pub async fn valid_quantity_by_item(
        &self,
        order_id: i32,
        statuses: &[i32],
        exclude_aftersale_id: Option<i32>,
    ) -> Result<HashMap<i32, i32>> {
        if statuses.is_empty() {
            return Ok(HashMap::new());
        }
        let status_ph = vec!["?"; statuses.len()].join(", ");
        let exclude_sql = if exclude_aftersale_id.is_some() {
            " AND a.aftersale_id <> ?"
        } else {
            ""
        };
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT ai.order_item_id, CAST(COALESCE(SUM(ai.number), 0) AS BIGINT) AS n \
               FROM aftersales_item ai \
               JOIN aftersales a ON a.aftersale_id = ai.aftersale_id \
              WHERE a.order_id = ? AND a.status IN ({status_ph}) AND ai.order_item_id IS NOT NULL{exclude_sql} \
              GROUP BY ai.order_item_id"
        ));
        let mut q = sqlx::query_as::<_, (i32, i64)>(&sql).bind(order_id);
        for s in statuses {
            q = q.bind(*s);
        }
        if let Some(ex) = exclude_aftersale_id {
            q = q.bind(ex);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(item_id, n)| (item_id, i32::try_from(n).unwrap_or(i32::MAX)))
            .collect())
    }

    /// 该订单项是否已有「售后进行中」的申请
    /// （对应 Java `getApplyAftersalesCount(orderId, itemId) > 0`）。
    pub async fn has_applied_aftersales(
        &self,
        order_id: i32,
        order_item_id: i32,
        statuses: &[i32],
    ) -> Result<bool> {
        if statuses.is_empty() {
            return Ok(false);
        }
        let status_ph = vec!["?"; statuses.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT COUNT(*) FROM aftersales_item ai \
               JOIN aftersales a ON a.aftersale_id = ai.aftersale_id \
              WHERE a.order_id = ? AND a.status IN ({status_ph}) AND ai.order_item_id = ?"
        ));
        let mut q = sqlx::query_as::<_, (i64,)>(&sql).bind(order_id);
        for s in statuses {
            q = q.bind(*s);
        }
        let row = q
            .bind(order_item_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0 > 0)
    }

    // ————————————————————— 售后日志 —————————————————————

    /// 售后日志列表（对应 Java `AftersalesLogServiceImpl#getDetailLog`：
    /// 按 `log_id` 升序）。
    ///
    /// ⚠️ 返回**仓储专用行结构**而非实体 [`AftersalesLog`]：
    /// 自动生成的实体把 `add_time` 声明为 `String`，而 schema 里
    /// `aftersales_log.add_time` 是 `int unsigned`，直接用实体
    /// `query_as` 会在运行时解码失败。
    pub async fn find_logs(&self, aftersale_id: i32) -> Result<Vec<AftersalesLogRow>> {
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {AFTERSALES_LOG_COLUMNS} FROM aftersales_log WHERE aftersale_id = ? ORDER BY log_id ASC"
        ));
        sqlx::query_as::<_, AftersalesLogRow>(&sql)
            .bind(aftersale_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    // ————————————————————— 订单 / 订单项 —————————————————————

    /// 按订单ID查询订单。
    pub async fn find_order(&self, order_id: i32) -> Result<Option<Order>> {
        sqlx::query_as::<_, Order>("SELECT * FROM \"order\" WHERE order_id = $1 LIMIT 1")
            .bind(order_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按订单ID查明细（可再按 `item_id` 收窄），`item_id` 升序。
    pub async fn find_order_items(
        &self,
        order_id: i32,
        item_id: Option<i32>,
    ) -> Result<Vec<OrderItem>> {
        let sql = if item_id.is_some() {
            "SELECT \"item_id\", \"order_id\", \"order_sn\", \"user_id\", \"price\", \"quantity\", \"product_id\", \"product_name\", \"product_sn\", \"pic_thumb\", \"sku_id\", \"sku_data\", \"delivery_quantity\", \"product_type\", \"is_gift\", \"shop_id\", \"is_pin\", \"prepay_price\", \"commission\", \"origin_price\", \"promotion_data\", \"is_seckill\", \"extra_sku_data\", \"suppliers_id\", \"card_group_name\", \"vendor_product_id\", \"vendor_product_sku_id\", \"vendor_id\", \"vendor_product_supply_price\", \"change_price_data\"::text AS \"change_price_data\", \"admin_change_pre_price\", \"gift_card_id\", \"delivery_service_fee\", \"server_service_fee\", \"is_wholesale_line\", \"wholesale_grade_id\", \"wholesale_base_price\" FROM order_item WHERE order_id = $1 AND item_id = $2 ORDER BY item_id ASC"
        } else {
            "SELECT \"item_id\", \"order_id\", \"order_sn\", \"user_id\", \"price\", \"quantity\", \"product_id\", \"product_name\", \"product_sn\", \"pic_thumb\", \"sku_id\", \"sku_data\", \"delivery_quantity\", \"product_type\", \"is_gift\", \"shop_id\", \"is_pin\", \"prepay_price\", \"commission\", \"origin_price\", \"promotion_data\", \"is_seckill\", \"extra_sku_data\", \"suppliers_id\", \"card_group_name\", \"vendor_product_id\", \"vendor_product_sku_id\", \"vendor_id\", \"vendor_product_supply_price\", \"change_price_data\"::text AS \"change_price_data\", \"admin_change_pre_price\", \"gift_card_id\", \"delivery_service_fee\", \"server_service_fee\", \"is_wholesale_line\", \"wholesale_grade_id\", \"wholesale_base_price\" FROM order_item WHERE order_id = $1 ORDER BY item_id ASC"
        };
        let mut q = sqlx::query_as::<_, OrderItem>(sql).bind(order_id);
        if let Some(id) = item_id {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量按 `item_id` 查订单明细。
    pub async fn find_order_items_by_ids(&self, item_ids: &[i32]) -> Result<Vec<OrderItem>> {
        if item_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = vec!["?"; item_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT \"item_id\", \"order_id\", \"order_sn\", \"user_id\", \"price\", \"quantity\", \"product_id\", \"product_name\", \"product_sn\", \"pic_thumb\", \"sku_id\", \"sku_data\", \"delivery_quantity\", \"product_type\", \"is_gift\", \"shop_id\", \"is_pin\", \"prepay_price\", \"commission\", \"origin_price\", \"promotion_data\", \"is_seckill\", \"extra_sku_data\", \"suppliers_id\", \"card_group_name\", \"vendor_product_id\", \"vendor_product_sku_id\", \"vendor_id\", \"vendor_product_supply_price\", \"change_price_data\"::text AS \"change_price_data\", \"admin_change_pre_price\", \"gift_card_id\", \"delivery_service_fee\", \"server_service_fee\", \"is_wholesale_line\", \"wholesale_grade_id\", \"wholesale_base_price\" FROM order_item WHERE item_id IN ({ph})"));
        let mut q = sqlx::query_as::<_, OrderItem>(&sql);
        for id in item_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量按订单ID查明细。
    pub async fn find_order_items_by_order_ids(
        &self,
        order_ids: &[i32],
    ) -> Result<Vec<OrderItem>> {
        if order_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = vec!["?"; order_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT \"item_id\", \"order_id\", \"order_sn\", \"user_id\", \"price\", \"quantity\", \"product_id\", \"product_name\", \"product_sn\", \"pic_thumb\", \"sku_id\", \"sku_data\", \"delivery_quantity\", \"product_type\", \"is_gift\", \"shop_id\", \"is_pin\", \"prepay_price\", \"commission\", \"origin_price\", \"promotion_data\", \"is_seckill\", \"extra_sku_data\", \"suppliers_id\", \"card_group_name\", \"vendor_product_id\", \"vendor_product_sku_id\", \"vendor_id\", \"vendor_product_supply_price\", \"change_price_data\"::text AS \"change_price_data\", \"admin_change_pre_price\", \"gift_card_id\", \"delivery_service_fee\", \"server_service_fee\", \"is_wholesale_line\", \"wholesale_grade_id\", \"wholesale_base_price\" FROM order_item WHERE order_id IN ({ph}) ORDER BY item_id ASC"));
        let mut q = sqlx::query_as::<_, OrderItem>(&sql);
        for id in order_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量按订单ID查订单。
    pub async fn find_orders_by_ids(&self, order_ids: &[i32]) -> Result<Vec<Order>> {
        if order_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = vec!["?"; order_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT \"order_id\", \"order_sn\", \"user_id\", \"parent_order_id\", \"parent_order_sn\", \"order_status\", \"shipping_status\", \"pay_status\", \"add_time\", \"consignee\", \"address\", \"region_ids\", \"region_names\", \"address_data\", \"mobile\", \"email\", \"buyer_note\", \"admin_note\", \"shipping_method\", \"logistics_id\", \"logistics_provider\", \"logistics_name\", \"shipping_type_id\", \"shipping_type_name\", \"tracking_no\", \"shipping_time\", \"received_time\", \"pay_type_id\", \"pay_time\", \"use_points\", \"is_need_commisson\", \"distribution_status\", \"referrer_user_id\", \"is_del\", \"shop_id\", \"is_store_splited\", \"is_valid\", \"comment_status\", \"total_amount\", \"paid_amount\", \"unpaid_amount\", \"unrefund_amount\", \"product_amount\", \"coupon_amount\", \"points_amount\", \"discount_amount\", \"balance\", \"credit_amount\", \"is_credit_order\", \"online_paid_amount\", \"offline_paid_amount\", \"service_fee\", \"shipping_fee\", \"invoice_fee\", \"order_extension\", \"order_source\", \"invoice_data\", \"out_trade_no\", \"is_settlement\", \"is_exchange_order\", \"order_type\", \"is_cycle_order\", \"is_card_order\", \"is_presale\", \"presale_type\", \"mark\", \"vendor_id\", \"cancel_remark\", \"cash_amount\", \"cash_change_amount\", \"round_off_dime_amount\", \"round_off_cents_amount\", \"pos_user_id\", \"device_code\", \"discount_change_price\", \"tax_rate\", \"tax_rate_amount\", \"change_price_data\"::text AS \"change_price_data\", \"gift_card_amount\", \"delivery_date\"::text AS \"delivery_date\", \"delivery_time_slot\", \"period_index\", \"time_index\", \"table_id\", \"table_no\", \"table_area_name\", \"dining_count\" FROM \"order\" WHERE order_id IN ({ph})"));
        let mut q = sqlx::query_as::<_, Order>(&sql);
        for id in order_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 「售后申请列表」候选订单分页（对应 Java
    /// `OrderServiceImpl#afterSalesList` 的 `lambdaQuery()`）。
    ///
    /// 过滤条件逐条对齐 Java：
    /// - `is_del = 0`
    /// - `user_id = 当前用户`
    /// - `order_type` 不属 `{2 拼团, 7 付费商品, 8 卡密, 10 礼品卡}`
    ///   （Java `OrderType.VIRTUAL/PAID/CARD/GIFT_CARD`）
    /// - `order_status IN (1 待发货, 2 处理中, 5 已完成)`
    /// - **排除**含礼品卡明细的订单（Java `ne(OrderItem::getGiftCardId, 0)`
    ///   在 MySQL 中 `NULL` 不参与匹配，故此处显式写 `IS NOT NULL AND <> 0`）
    pub async fn paginate_aftersales_orders(
        &self,
        q: &AftersalesListQuery,
        user_id: i32,
    ) -> Result<(u64, Vec<Order>)> {
        let where_sql = " WHERE is_del = 0 AND user_id = $1 \
             AND (order_type IS NULL OR order_type NOT IN ($2, $3, $4, $5)) \
             AND order_status IN ($6, $7, $8) \
             AND order_id NOT IN (SELECT order_id FROM order_item \
                                   WHERE user_id = $9 AND gift_card_id IS NOT NULL AND gift_card_id <> 0)"
            .to_string();

        let (page_size, offset) = (q.page_size(), (q.page_no() - 1) * q.page_size());

        let total_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM \"order\"{where_sql}"));
        let total: (i64,) = sqlx::query_as::<_, (i64,)>(&total_sql)
            .bind(user_id)
            .bind(EXCLUDED_ORDER_TYPE_PIN)
            .bind(EXCLUDED_ORDER_TYPE_PAID)
            .bind(EXCLUDED_ORDER_TYPE_CARD)
            .bind(EXCLUDED_ORDER_TYPE_GIFT_CARD)
            .bind(ORDER_STATUS_CONFIRMED)
            .bind(ORDER_STATUS_PROCESSING)
            .bind(ORDER_STATUS_COMPLETED)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT \"order_id\", \"order_sn\", \"user_id\", \"parent_order_id\", \"parent_order_sn\", \"order_status\", \"shipping_status\", \"pay_status\", \"add_time\", \"consignee\", \"address\", \"region_ids\", \"region_names\", \"address_data\", \"mobile\", \"email\", \"buyer_note\", \"admin_note\", \"shipping_method\", \"logistics_id\", \"logistics_provider\", \"logistics_name\", \"shipping_type_id\", \"shipping_type_name\", \"tracking_no\", \"shipping_time\", \"received_time\", \"pay_type_id\", \"pay_time\", \"use_points\", \"is_need_commisson\", \"distribution_status\", \"referrer_user_id\", \"is_del\", \"shop_id\", \"is_store_splited\", \"is_valid\", \"comment_status\", \"total_amount\", \"paid_amount\", \"unpaid_amount\", \"unrefund_amount\", \"product_amount\", \"coupon_amount\", \"points_amount\", \"discount_amount\", \"balance\", \"credit_amount\", \"is_credit_order\", \"online_paid_amount\", \"offline_paid_amount\", \"service_fee\", \"shipping_fee\", \"invoice_fee\", \"order_extension\", \"order_source\", \"invoice_data\", \"out_trade_no\", \"is_settlement\", \"is_exchange_order\", \"order_type\", \"is_cycle_order\", \"is_card_order\", \"is_presale\", \"presale_type\", \"mark\", \"vendor_id\", \"cancel_remark\", \"cash_amount\", \"cash_change_amount\", \"round_off_dime_amount\", \"round_off_cents_amount\", \"pos_user_id\", \"device_code\", \"discount_change_price\", \"tax_rate\", \"tax_rate_amount\", \"change_price_data\"::text AS \"change_price_data\", \"gift_card_amount\", \"delivery_date\"::text AS \"delivery_date\", \"delivery_time_slot\", \"period_index\", \"time_index\", \"table_id\", \"table_no\", \"table_area_name\", \"dining_count\" FROM \"order\"{where_sql} ORDER BY order_id DESC LIMIT ? OFFSET ?"
        ));
        let rows = sqlx::query_as::<_, Order>(&rows_sql)
            .bind(user_id)
            .bind(EXCLUDED_ORDER_TYPE_PIN)
            .bind(EXCLUDED_ORDER_TYPE_PAID)
            .bind(EXCLUDED_ORDER_TYPE_CARD)
            .bind(EXCLUDED_ORDER_TYPE_GIFT_CARD)
            .bind(ORDER_STATUS_CONFIRMED)
            .bind(ORDER_STATUS_PROCESSING)
            .bind(ORDER_STATUS_COMPLETED)
            .bind(user_id)
            .bind(page_size as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 订单下指定状态的售后单（`aftersale_id` 倒序）。
    ///
    /// 用于 Java `isFullRefundCoverage` / `coversAllOrderItems` 的
    /// 「有效状态（含已完成）」明细集合。
    pub async fn find_valid_aftersales(
        &self,
        order_id: i32,
        statuses: &[i32],
    ) -> Result<Vec<Aftersales>> {
        if statuses.is_empty() {
            return Ok(Vec::new());
        }
        let status_ph = vec!["?"; statuses.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {AFTERSALES_COLUMNS} FROM aftersales \
              WHERE order_id = ? AND status IN ({status_ph}) ORDER BY aftersale_id DESC"
        ));
        let mut q = sqlx::query_as::<_, Aftersales>(&sql).bind(order_id);
        for s in statuses {
            q = q.bind(*s);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 订单下「售后进行中」状态的售后单（用于 `aftersaleFlag` 判定）。
    pub async fn find_in_progress_aftersales(
        &self,
        order_id: i32,
        statuses: &[i32],
    ) -> Result<Vec<Aftersales>> {
        if statuses.is_empty() {
            return Ok(Vec::new());
        }
        let status_ph = vec!["?"; statuses.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {AFTERSALES_COLUMNS} FROM aftersales \
              WHERE order_id = ? AND status IN ({status_ph}) ORDER BY aftersale_id DESC"
        ));
        let mut q = sqlx::query_as::<_, Aftersales>(&sql).bind(order_id);
        for s in statuses {
            q = q.bind(*s);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 商品 / 用户 / 支付 —————————————————————

    /// 批量查商品。
    pub async fn find_products(&self, product_ids: &[i32]) -> Result<Vec<Product>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = vec!["?"; product_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT * FROM product WHERE product_id IN ({ph})"));
        let mut q = sqlx::query_as::<_, Product>(&sql);
        for id in product_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 按用户ID查用户名（Java `userMapper.selectById(userId).getUsername()`）。
    pub async fn find_username(&self, user_id: i32) -> Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT username FROM \"user\" WHERE user_id = $1 LIMIT 1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|r| r.0))
    }

    /// 是否存在指定支付渠道的成功支付记录（对应 Java
    /// `paylogMapper.selectList(order_id, order_type = 0, pay_status = 1)`
    /// 后用 \"payCodes\" 判断）。
    ///
    /// 云支付渠道下商品**只支持整单退款**（`error.order.item.full.refund.only`）。
    pub async fn has_success_paylog_with_codes(
        &self,
        order_id: i32,
        pay_codes: &[&str],
    ) -> Result<bool> {
        if pay_codes.is_empty() {
            return Ok(false);
        }
        let ph = vec!["?"; pay_codes.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT COUNT(*) FROM paylog \
              WHERE order_id = ? AND order_type = 0 AND pay_status = 1 AND pay_code IN ({ph})"
        ));
        let mut q = sqlx::query_as::<_, (i64,)>(&sql).bind(order_id);
        for c in pay_codes {
            q = q.bind(*c);
        }
        let row = q.fetch_one(&self.pool).await.map_err(map_err)?;
        Ok(row.0 > 0)
    }

    // ————————————————————— 售后金额（占用数量） —————————————————————

    /// 订单全部明细数量合计（Java `orderItems.stream().mapToInt(getQuantity).sum()`）。
    pub async fn order_items_total_quantity(&self, order_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT CAST(COALESCE(SUM(quantity), 0) AS BIGINT) FROM order_item WHERE order_id = $1",
        )
        .bind(order_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }
}

/// 售后单插入语句（与服务层、测试共用，避免多处漂移）。
const AFTERSALES_INSERT: &str = "INSERT INTO aftersales
     (aftersale_type, status, pics, description, reply, add_time, tracking_no,
      logistics_name, return_address, aftersale_reason, aftersales_sn, order_id,
      user_id, refund_amount, shop_id, audit_time, deal_time, final_time,
      return_goods_tip, vendor_id, close_type)
     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21) RETURNING aftersale_id";

/// 售后日志插入语句。
const AFTERSALES_LOG_INSERT: &str = "INSERT INTO aftersales_log
     (aftersale_id, log_info, add_time, admin_name, refund_money, refund_type,
      refund_desc, user_name, return_pic, shop_id, vendor_id)
     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)";

/// 事务内写入售后日志。
async fn insert_log(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    aftersale_id: i32,
    log: &AftersalesLog,
) -> Result<()> {
    sqlx::query(AFTERSALES_LOG_INSERT)
        .bind(aftersale_id)
        .bind(&log.log_info)
        .bind(log.add_time)
        .bind(&log.admin_name)
        .bind(log.refund_money)
        .bind(log.refund_type)
        .bind(&log.refund_desc)
        .bind(&log.user_name)
        .bind(&log.return_pic)
        .bind(log.shop_id)
        .bind(log.vendor_id)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
    Ok(())
}

// ————————————————————— 售后类型常量 —————————————————————
//
// 说明：\"Bind\" / `bind_all!` / \"non_blank\" / `EXCLUDED_ORDER_TYPE_*` /
// `ORDER_STATUS_*` 已在文件头部统一声明（见上方 const 与辅助函数区），
// 此处曾出现一份重复粘贴的同名定义，已删除以免 E0428 / E0119。

/// 售后类型「退货退款」（Java `AftersalesTypeEnum.RETURN`）。
const AFTERSALES_TYPE_RETURN: i32 = 1;
/// 售后类型「仅退款」（Java `AftersalesTypeEnum.PAYRETURN`）。
const AFTERSALES_TYPE_PAY_RETURN: i32 = 2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_blank_trims_and_drops_empty() {
        assert_eq!(non_blank(&None), None);
        assert_eq!(non_blank(&Some("   ".into())), None);
        assert_eq!(non_blank(&Some(" TIG ".into())), Some("TIG".into()));
    }

    #[test]
    fn bind_all_builds_query_for_all_variants() {
        // 只验证宏在两种 Bind 变体下都能展开（不连库）
        let binds = vec![Bind::I32(1), Bind::Str("x".into())];
        let q = bind_all!(sqlx::query_as::<sqlx::Postgres, (i32,)>("SELECT $1"), binds);
        drop(q);
    }
}
