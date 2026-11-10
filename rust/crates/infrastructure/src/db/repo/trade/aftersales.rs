//! 售后管理域（trade::aftersales）仓储：`aftersales` / `aftersales_item` /
//! `aftersales_log` / `refund_apply` / `order` / `order_item` / `user` / `shop`。
//!
//! 列名严格对齐权威 schema（`python tools/schema_parser.py --table <表名>`）：
//! - `aftersales`：`aftersale_id` / `aftersale_type` / `status` / `pics` /
//!   `description` / `reply` / `add_time` / `tracking_no` / `logistics_name` /
//!   `return_address` / `aftersale_reason` / `aftersales_sn` / `order_id` /
//!   `user_id` / `refund_amount` / `shop_id` / `audit_time` / `deal_time` /
//!   `final_time` / `return_goods_tip` / `vendor_id` / `close_type`
//! - `aftersales_item`：`aftersales_item_id` / `order_item_id` / `number` / `aftersale_id`
//! - `aftersales_log`：`log_id` / `aftersale_id` / `log_info` / `add_time` /
//!   `admin_name` / `refund_money` / `refund_type` / `refund_desc` / `user_name` /
//!   `return_pic` / `shop_id` / `vendor_id`
//! - `refund_apply`：`refund_id` / `aftersale_id` / `refund_status` /
//!   `refund_note` / `online_balance` / `offline_balance` / `refund_balance`
//! - `order`：`order_id` / `order_sn` / `out_trade_no` / `order_status` /
//!   `shipping_status` / `total_amount` / `shipping_fee` / `shipping_time` /
//!   `coupon_amount` / `discount_amount` / `tax_rate` / `tax_rate_amount` /
//!   `parent_order_id` / `mark` / `admin_note` / `vendor_id` / `shop_id` / `user_id`
//! - `order_item`：`item_id` / `order_id` / `price` / `quantity` / `extra_sku_data` /
//!   `product_name` / `product_sn` / `pic_thumb` / `product_id` / `sku_data` / `order_sn`
//! - `user`：`user_id` / `username` / `nickname` / `avatar` / `mobile` / `email`
//! - `shop`：`shop_id` / `shop_title`
//!
//! 注意：`order` 是 MySQL 保留字，SQL 中必须使用反引号。
//! 查询统一使用运行时 `sqlx::query_as`（不使用 `query!` 宏）。

use sqlx::PgPool;
use nextrade_core::entities::{Aftersales, AftersalesItem, AftersalesLog, Order, OrderItem, RefundApply, Shop, User};
use nextrade_core::models::trade::aftersales::AftersalesListQuery;
use nextrade_core::{Error, Result};

/// 待绑定参数（按 `conds` / `SET` 顺序 1:1 对应）。
#[derive(Debug, Clone)]
enum Bind {
    I32(i32),
    Str(String),
    Decimal(rust_decimal::Decimal),
}

/// 把 `Bind` 列表按顺序绑定到查询上（宏展开以便同时适配不同返回行类型）。
macro_rules! bind_all {
    ($q:expr, $binds:expr) => {{
        let mut q = $q;
        for b in $binds.iter() {
            q = match b {
                Bind::I32(v) => q.bind(*v),
                Bind::Str(v) => q.bind(v.as_str()),
                Bind::Decimal(v) => q.bind(*v),
            };
        }
        q
    }};
}

/// 售后单 + 关联订单 + 下单用户的列表行（对应 Java `list` 循环里的 N+1 消除改造）。
#[derive(Debug, Clone)]
pub struct AftersalesRow {
    /// 售后主单
    pub aftersales: Aftersales,
    /// 关联订单（查不到时 `None`，Java 把 `orderSn` 置空串）
    pub order: Option<Order>,
}

/// 售后明细 + 其对应订单明细（一次联表取回，对应 Java `getItemByIds`）。
#[derive(Debug, Clone)]
pub struct AftersalesItemRow {
    /// 售后明细
    pub item: AftersalesItem,
    /// 订单明细（查不到时 `None`，Java 的 `orderItem == null` 分支）
    pub order_item: Option<OrderItem>,
}

/// 仓储错误映射：SQLx 错误 → [`nextrade_core::Error`]。
///
/// 与 `repo::admin_user` / `repo::cart` 等既有仓储保持同一口径
/// （`nextrade_core::Error` 未提供 `From<sqlx::Error>`）。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("记录不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

/// 构造 `aftersales_id IN (?, ?, ...)` 的占位串。
fn placeholders(n: usize) -> String {
    vec!["?"; n].join(", ")
}

#[derive(Clone)]
pub struct AftersalesRepository {
    pool: PgPool,
}

impl AftersalesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— aftersales —————————————————————

    /// 按主键查询售后单。
    pub async fn find_aftersales(&self, aftersale_id: i32) -> Result<Option<Aftersales>> {
        sqlx::query_as::<_, Aftersales>("SELECT * FROM aftersales WHERE aftersale_id = $1 LIMIT 1")
            .bind(aftersale_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 到期应自动确认收货的售后（Java `AftersalesServiceImpl#autoReceive` 延迟队列的轮询替代）。
    pub async fn find_due_auto_receive(
        &self,
        cutoff: i32,
        limit: i64,
    ) -> Result<Vec<Aftersales>> {
        sqlx::query_as::<_, Aftersales>(
            "SELECT * FROM aftersales \
             WHERE status IN (5, 22) AND deal_time > 0 AND deal_time <= $1 \
               AND tracking_no <> '' \
             ORDER BY deal_time ASC LIMIT $2",
        )
        .bind(cutoff)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 插入售后主单（对应 Java `AftersalesServiceImpl#create` 的 `this.save(aftersales)`），
    /// 返回自增主键。
    pub async fn insert_aftersales(&self, a: &Aftersales) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO aftersales \
             (aftersale_type, status, pics, description, reply, add_time, tracking_no, \
              logistics_name, return_address, aftersale_reason, aftersales_sn, order_id, \
              user_id, refund_amount, shop_id, audit_time, deal_time, final_time, \
              return_goods_tip, vendor_id, close_type) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21) RETURNING aftersale_id",
        )
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

    /// 批量查询售后单（财务退款标记的批量口径）。
    pub async fn find_aftersales_by_ids(&self, ids: &[i32]) -> Result<Vec<Aftersales>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM aftersales WHERE aftersale_id IN ({}) ORDER BY aftersale_id DESC",
            placeholders(ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, Aftersales>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 售后分页（对应 Java `AftersalesServiceImpl#list` 的查询构造）。
    ///
    /// 逐条对照 Java：
    /// - `aftersaleType ∈ {1, 2}` 时才按类型过滤（\"aftersalesTypeFlag\"）；
    /// - \"shopFlag\" / \"vendorFlag\" 在 `shop_id > 0` / `vendor_id > 0` 时才生效；
    /// - \"searchOrderIds\" 非空时才按订单ID集合过滤；
    /// - 普通后台按 \"status\" 过滤；供应商后台按 `{21, 4, 22, 6}` 白名单过滤，
    ///   传了非法状态时按 `status = -999` 必然查空（与 Java 完全一致）；
    /// - 默认排序 `aftersale_id DESC`（Java \"buildSortOrder\" 缺省主键降序）。
    #[allow(clippy::too_many_arguments)]
    pub async fn paginate_aftersales(
        &self,
        q: &AftersalesListQuery,
        shop_id: Option<i32>,
        vendor_id: Option<i32>,
        is_vendor_admin: bool,
        search_order_ids: Option<&[i32]>,
    ) -> Result<(u64, Vec<Aftersales>)> {
        let (where_sql, binds) = build_filter(q, shop_id, vendor_id, is_vendor_admin, search_order_ids);
        let page_size = q.page_size();
        let offset = (q.page_no() - 1) * page_size;

        let total_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM aftersales{where_sql}"));
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&total_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM aftersales{where_sql} ORDER BY aftersale_id DESC LIMIT ? OFFSET ?"
        ));
        let rows = bind_all!(sqlx::query_as::<_, Aftersales>(&rows_sql), binds)
            .bind(page_size as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 按售后ID批量取回「售后单 + 关联订单」。
    pub async fn find_aftersales_with_orders(&self, ids: &[i32]) -> Result<Vec<AftersalesRow>> {
        let list = self.find_aftersales_by_ids(ids).await?;
        if list.is_empty() {
            return Ok(Vec::new());
        }
        let order_ids: Vec<i32> = {
            let mut seen = std::collections::HashSet::new();
            list.iter()
                .filter_map(|a| a.order_id)
                .filter(|id| seen.insert(*id))
                .collect()
        };
        let orders = self.find_orders(&order_ids).await?;
        let map: std::collections::HashMap<i32, Order> =
            orders.into_iter().map(|o| (o.order_id, o)).collect();
        Ok(list
            .into_iter()
            .map(|a| AftersalesRow {
                order: a.order_id.and_then(|id| map.get(&id).cloned()),
                aftersales: a,
            })
            .collect())
    }

    /// 按售后ID查询协商记录（Java `AftersalesLogService#getDetailLog` 的取数口径：
    /// \"add_time\" 升序）。
    pub async fn find_logs(&self, aftersale_id: i32) -> Result<Vec<AftersalesLog>> {
        sqlx::query_as::<_, AftersalesLog>(
            "SELECT * FROM aftersales_log WHERE aftersale_id = $1 \
             ORDER BY add_time ASC, log_id ASC",
        )
        .bind(aftersale_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 查询该订单下指定状态的售后主单（对应 Java \"isOrderItemsFullyCovered\" 里的
    /// `SELECT aftersale_id FROM aftersales WHERE order_id = $1 AND status IN (...)`）。
    pub async fn find_aftersales_by_order_and_status(
        &self,
        order_id: i32,
        statuses: &[i32],
    ) -> Result<Vec<Aftersales>> {
        if statuses.is_empty() {
            return Ok(Vec::new());
        }
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM aftersales WHERE order_id = ? AND status IN ({}) \
             ORDER BY aftersale_id ASC",
            placeholders(statuses.len())
        ));
        let mut q = sqlx::query_as::<_, Aftersales>(&sql).bind(order_id);
        for s in statuses {
            q = q.bind(*s);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 该订单下、主键小于当前售后且未取消/未驳回的售后主单
    /// （对应 Java `isCompletingLastItems` 里的 `lt(aftersale_id).ne(CANCEL).ne(REFUSE)`）。
    pub async fn find_earlier_active_aftersales(
        &self,
        order_id: i32,
        before_aftersale_id: i32,
    ) -> Result<Vec<Aftersales>> {
        sqlx::query_as::<_, Aftersales>(
            "SELECT * FROM aftersales WHERE order_id = $1 AND aftersale_id < $2 \
             AND status NOT IN ($3, $4) ORDER BY aftersale_id ASC",
        )
        .bind(order_id)
        .bind(before_aftersale_id)
        .bind(nextrade_core::models::trade::aftersales::aftersales_status::CANCEL)
        .bind(nextrade_core::models::trade::aftersales::aftersales_status::REFUSE)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按订单查询全部售后主单（幂等判断用）。
    pub async fn find_by_order(&self, order_id: i32) -> Result<Vec<Aftersales>> {
        sqlx::query_as::<_, Aftersales>("SELECT * FROM aftersales WHERE order_id = $1")
            .bind(order_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 批量插入售后商品明细。
    pub async fn insert_items(&self, items: &[nextrade_core::entities::AftersalesItem]) -> Result<()> {
        for item in items {
            sqlx::query(
                "INSERT INTO aftersales_item (order_item_id, number, aftersale_id) VALUES ($1, $2, $3)",
            )
            .bind(item.order_item_id)
            .bind(item.number)
            .bind(item.aftersale_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        }
        Ok(())
    }

    // ————————————————————— aftersales_item —————————————————————

    /// 按售后ID批量查询明细（含对应订单明细，消除 N+1）。
    pub async fn find_items_by_aftersale_ids(
        &self,
        aftersale_ids: &[i32],
    ) -> Result<Vec<AftersalesItemRow>> {
        if aftersale_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM aftersales_item WHERE aftersale_id IN ({}) \
             ORDER BY aftersale_id ASC, aftersales_item_id ASC",
            placeholders(aftersale_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, AftersalesItem>(&sql);
        for id in aftersale_ids {
            q = q.bind(*id);
        }
        let items = q.fetch_all(&self.pool).await.map_err(map_err)?;
        self.attach_order_items(items).await
    }

    /// 按售后ID查询明细（含对应订单明细）。
    pub async fn find_items_by_aftersale_id(
        &self,
        aftersale_id: i32,
    ) -> Result<Vec<AftersalesItemRow>> {
        self.find_items_by_aftersale_ids(&[aftersale_id]).await
    }

    /// 补齐每条售后明细对应的订单明细（一批 \"IN\" 查询）。
    async fn attach_order_items(&self, items: Vec<AftersalesItem>) -> Result<Vec<AftersalesItemRow>> {
        if items.is_empty() {
            return Ok(Vec::new());
        }
        let order_item_ids: Vec<i32> = {
            let mut seen = std::collections::HashSet::new();
            items
                .iter()
                .filter_map(|i| i.order_item_id)
                .filter(|id| seen.insert(*id))
                .collect()
        };
        let order_items = self.find_order_items(&order_item_ids).await?;
        let map: std::collections::HashMap<i32, OrderItem> =
            order_items.into_iter().map(|i| (i.item_id, i)).collect();
        Ok(items
            .into_iter()
            .map(|item| AftersalesItemRow {
                order_item: item.order_item_id.and_then(|id| map.get(&id).cloned()),
                item,
            })
            .collect())
    }

    /// 按主键批量查询订单明细。
    pub async fn find_order_items(&self, item_ids: &[i32]) -> Result<Vec<OrderItem>> {
        if item_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM order_item WHERE item_id IN ({}) ORDER BY item_id ASC",
            placeholders(item_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, OrderItem>(&sql);
        for id in item_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— aftersales_log —————————————————————

    /// 写入协商记录（对应 Java `AftersalesLogService#addAftersaleLog` /
    /// \"addSystemAftersaleLog\"），返回自增主键。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_log(&self, log: &AftersalesLog) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO aftersales_log \
             (aftersale_id, log_info, add_time, admin_name, refund_money, refund_type, \
              refund_desc, user_name, return_pic, shop_id, vendor_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING log_id",
        )
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

    // ————————————————————— refund_apply —————————————————————

    /// 按售后ID取最新一条退款申请（对应 Java `orderByDesc("refund_id").last("LIMIT 1")`）。
    pub async fn find_latest_refund_apply(
        &self,
        aftersale_id: i32,
    ) -> Result<Option<RefundApply>> {
        sqlx::query_as::<_, RefundApply>(
            "SELECT * FROM refund_apply WHERE aftersale_id = $1 ORDER BY refund_id DESC LIMIT 1",
        )
        .bind(aftersale_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 批量取回这批售后单的「最新退款申请」（`aftersale_id` → 最新一笔）。
    ///
    /// 对应 Java `findPendingFinanceRefundIds` / `findFinanceRejectedRefundIds`
    /// 的批量口径（Java 在内存里按 `refund_id` 取最大值）。
    pub async fn find_latest_refund_applies(
        &self,
        aftersale_ids: &[i32],
    ) -> Result<std::collections::HashMap<i32, RefundApply>> {
        if aftersale_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let sql = format!(
            "SELECT * FROM refund_apply WHERE aftersale_id IN ({}) ORDER BY refund_id ASC",
            placeholders(aftersale_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, RefundApply>(&sql);
        for id in aftersale_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        // 升序遍历 → 后写覆盖前写，最终保留 refund_id 最大的一条
        let mut map = std::collections::HashMap::new();
        for r in rows {
            map.insert(r.aftersale_id, r);
        }
        Ok(map)
    }

    /// 订单已完成且未计入总额的售后退款明细之和（对应 Java
    /// \"isShippingAlreadyRefunded\" 里的 `sum(online_balance + refund_balance + offline_balance)`，
    /// 排除当前售后单本身，且只统计处理中/已处理）。
    pub async fn sum_other_processed_refunds(
        &self,
        order_id: i32,
        exclude_aftersale_id: i32,
    ) -> Result<rust_decimal::Decimal> {
        let row: (Option<rust_decimal::Decimal>,) = sqlx::query_as(
            "SELECT SUM(online_balance + refund_balance + offline_balance) FROM refund_apply \
             WHERE order_id = $1 AND aftersale_id <> $2 AND refund_status IN (1, 2)",
        )
        .bind(order_id)
        .bind(exclude_aftersale_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.unwrap_or(rust_decimal::Decimal::ZERO))
    }

    // ————————————————————— order / order_item —————————————————————

    /// 按订单号模糊匹配订单ID（对应 Java `like(Order::getOrderSn, keyword)`）。
    pub async fn find_order_ids_by_sn(&self, keyword: &str) -> Result<Vec<i32>> {
        let pat = format!("%{keyword}%");
        let rows: Vec<(i32,)> =
            sqlx::query_as("SELECT order_id FROM \"order\" WHERE order_sn LIKE $1 ORDER BY order_id ASC")
                .bind(pat)
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 按交易单号模糊匹配订单ID（对应 Java `like(Order::getOutTradeNo, outTradeNo)`）。
    pub async fn find_order_ids_by_out_trade_no(&self, out_trade_no: &str) -> Result<Vec<i32>> {
        let pat = format!("%{out_trade_no}%");
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT order_id FROM \"order\" WHERE out_trade_no LIKE $1 ORDER BY order_id ASC",
        )
        .bind(pat)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 按主键批量查询订单。
    pub async fn find_orders(&self, order_ids: &[i32]) -> Result<Vec<Order>> {
        if order_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT \"order_id\", \"order_sn\", \"user_id\", \"parent_order_id\", \"parent_order_sn\", \"order_status\", \"shipping_status\", \"pay_status\", \"add_time\", \"consignee\", \"address\", \"region_ids\", \"region_names\", \"address_data\", \"mobile\", \"email\", \"buyer_note\", \"admin_note\", \"shipping_method\", \"logistics_id\", \"logistics_provider\", \"logistics_name\", \"shipping_type_id\", \"shipping_type_name\", \"tracking_no\", \"shipping_time\", \"received_time\", \"pay_type_id\", \"pay_time\", \"use_points\", \"is_need_commisson\", \"distribution_status\", \"referrer_user_id\", \"is_del\", \"shop_id\", \"is_store_splited\", \"is_valid\", \"comment_status\", \"total_amount\", \"paid_amount\", \"unpaid_amount\", \"unrefund_amount\", \"product_amount\", \"coupon_amount\", \"points_amount\", \"discount_amount\", \"balance\", \"credit_amount\", \"is_credit_order\", \"online_paid_amount\", \"offline_paid_amount\", \"service_fee\", \"shipping_fee\", \"invoice_fee\", \"order_extension\", \"order_source\", \"invoice_data\", \"out_trade_no\", \"is_settlement\", \"is_exchange_order\", \"order_type\", \"is_cycle_order\", \"is_card_order\", \"is_presale\", \"presale_type\", \"mark\", \"vendor_id\", \"cancel_remark\", \"cash_amount\", \"cash_change_amount\", \"round_off_dime_amount\", \"round_off_cents_amount\", \"pos_user_id\", \"device_code\", \"discount_change_price\", \"tax_rate\", \"tax_rate_amount\", \"change_price_data\"::text AS \"change_price_data\", \"gift_card_amount\", \"delivery_date\"::text AS \"delivery_date\", \"delivery_time_slot\", \"period_index\", \"time_index\", \"table_id\", \"table_no\", \"table_area_name\", \"dining_count\" FROM \"order\" WHERE order_id IN ({}) ORDER BY order_id ASC",
            placeholders(order_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, Order>(&sql);
        for id in order_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 订单全部明细（对应 Java `orderItemMapper.selectList(eq("order_id", ...))`）。
    pub async fn find_order_items_by_order(&self, order_id: i32) -> Result<Vec<OrderItem>> {
        sqlx::query_as::<_, OrderItem>(
            "SELECT \"item_id\", \"order_id\", \"order_sn\", \"user_id\", \"price\", \"quantity\", \"product_id\", \"product_name\", \"product_sn\", \"pic_thumb\", \"sku_id\", \"sku_data\", \"delivery_quantity\", \"product_type\", \"is_gift\", \"shop_id\", \"is_pin\", \"prepay_price\", \"commission\", \"origin_price\", \"promotion_data\", \"is_seckill\", \"extra_sku_data\", \"suppliers_id\", \"card_group_name\", \"vendor_product_id\", \"vendor_product_sku_id\", \"vendor_id\", \"vendor_product_supply_price\", \"change_price_data\"::text AS \"change_price_data\", \"admin_change_pre_price\", \"gift_card_id\", \"delivery_service_fee\", \"server_service_fee\", \"is_wholesale_line\", \"wholesale_grade_id\", \"wholesale_base_price\" FROM order_item WHERE order_id = $1 ORDER BY item_id ASC",
        )
        .bind(order_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— user / shop —————————————————————

    /// 按主键批量查询用户。
    pub async fn find_users(&self, user_ids: &[i32]) -> Result<Vec<User>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM \"user\" WHERE user_id IN ({}) ORDER BY user_id ASC",
            placeholders(user_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, User>(&sql);
        for id in user_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 按主键批量查询店铺。
    pub async fn find_shops(&self, shop_ids: &[i32]) -> Result<Vec<Shop>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM shop WHERE shop_id IN ({}) ORDER BY shop_id ASC",
            placeholders(shop_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, Shop>(&sql);
        for id in shop_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 状态写回 —————————————————————

    /// 更新售后主单（对应 Java `updateById` 的部分字段更新）。
    ///
    /// 每个可空参数为 `None` 时保持原值不变（等价 MyBatis-Plus 的
    /// 「非空字段才进 SET」策略）。
    #[allow(clippy::too_many_arguments)]
    pub async fn update_aftersales(
        &self,
        aftersale_id: i32,
        status: Option<i32>,
        reply: Option<&str>,
        refund_amount: Option<rust_decimal::Decimal>,
        return_address: Option<&str>,
        return_goods_tip: Option<&str>,
        close_type: Option<i32>,
        audit_time: Option<i32>,
        deal_time: Option<i32>,
        final_time: Option<i32>,
        tracking_no: Option<&str>,
        logistics_name: Option<&str>,
    ) -> Result<bool> {
        let mut sets: Vec<&str> = Vec::new();
        let mut binds: Vec<Bind> = Vec::new();
        if let Some(v) = status {
            sets.push("status = ?");
            binds.push(Bind::I32(v));
        }
        if let Some(v) = reply {
            sets.push("reply = ?");
            binds.push(Bind::Str(v.to_string()));
        }
        if let Some(v) = refund_amount {
            // 走具名 Decimal 绑定，避免把金额降级成字符串中转
            sets.push("refund_amount = ?");
            binds.push(Bind::Decimal(v));
        }
        if let Some(v) = return_address {
            sets.push("return_address = ?");
            binds.push(Bind::Str(v.to_string()));
        }
        if let Some(v) = return_goods_tip {
            sets.push("return_goods_tip = ?");
            binds.push(Bind::Str(v.to_string()));
        }
        if let Some(v) = close_type {
            sets.push("close_type = ?");
            binds.push(Bind::I32(v));
        }
        if let Some(v) = audit_time {
            sets.push("audit_time = ?");
            binds.push(Bind::I32(v));
        }
        if let Some(v) = deal_time {
            sets.push("deal_time = ?");
            binds.push(Bind::I32(v));
        }
        if let Some(v) = final_time {
            sets.push("final_time = ?");
            binds.push(Bind::I32(v));
        }
        if let Some(v) = tracking_no {
            sets.push("tracking_no = ?");
            binds.push(Bind::Str(v.to_string()));
        }
        if let Some(v) = logistics_name {
            sets.push("logistics_name = ?");
            binds.push(Bind::Str(v.to_string()));
        }
        if sets.is_empty() {
            return Ok(false);
        }
        binds.push(Bind::I32(aftersale_id));
        let sql = format!(
            "UPDATE aftersales SET {} WHERE aftersale_id = $1",
            sets.join(", ")
        );
        let r = bind_all!(sqlx::query(&sql), binds)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 仅回写退款金额（对应 Java `lambdaUpdate().set(Aftersales::getRefundAmount, ...)`）。
    pub async fn update_refund_amount(
        &self,
        aftersale_id: i32,
        refund_amount: rust_decimal::Decimal,
    ) -> Result<bool> {
        let r = sqlx::query("UPDATE aftersales SET refund_amount = $1 WHERE aftersale_id = $2")
            .bind(refund_amount)
            .bind(aftersale_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }
}

/// 构造售后列表 WHERE 子句（含前导空格）与绑定参数。
///
/// 逐条对照 Java `AftersalesServiceImpl#list` 的 `lambdaQuery()` 链：
/// 条件顺序与 Java 的 `.eq(...)` 出现顺序一致，便于人工比对。
fn build_filter(
    q: &AftersalesListQuery,
    shop_id: Option<i32>,
    vendor_id: Option<i32>,
    is_vendor_admin: bool,
    search_order_ids: Option<&[i32]>,
) -> (String, Vec<Bind>) {
    let mut conds: Vec<&str> = Vec::new();
    let mut binds: Vec<Bind> = Vec::new();

    // 售后类型：仅 1/2 参与过滤（Java `aftersalesTypeFlag`）
    let aftersale_type = q
        .aftersale_type
        .filter(|t| *t == 1 || *t == 2);
    if let Some(t) = aftersale_type {
        conds.push("aftersale_type = ?");
        binds.push(Bind::I32(t));
    }
    // 店铺：Java `shopFlag = HeaderUtils.getShopId() > 0`
    if let Some(sid) = shop_id.filter(|v| *v > 0) {
        conds.push("shop_id = ?");
        binds.push(Bind::I32(sid));
    }
    if let Some(uid) = q.user_id {
        conds.push("user_id = ?");
        binds.push(Bind::I32(uid));
    }
    // 供应商：Java `vendorFlag = getVendorId() > 0`
    if let Some(vid) = vendor_id.filter(|v| *v > 0) {
        conds.push("vendor_id = ?");
        binds.push(Bind::I32(vid));
    }
    if let Some(sn) = q.aftersales_sn.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        conds.push("aftersales_sn LIKE ?");
        binds.push(Bind::Str(format!("%{sn}%")));
    }
    // 供应商后台的状态白名单（Java `List.of(21, 4, 22, 6)`）
    const VENDOR_STATUSES: [i32; 4] = [21, 4, 22, 6];

    // 订单ID集合过滤（Java `in(CollUtil.isNotEmpty(searchOrderIds), ...)`）
    if let Some(ids) = search_order_ids.filter(|v| !v.is_empty()) {
        conds.push("order_id IN (__ORDER_IDS__)");
        for id in ids {
            binds.push(Bind::I32(*id));
        }
    }

    let status_flag = q.status.is_some_and(|s| s > 0);
    let status = q.status;
    if !is_vendor_admin {
        // 普通后台按 status 查询
        if status_flag {
            conds.push("status = ?");
            binds.push(Bind::I32(status.unwrap_or_default()));
        }
    } else if status.is_none() {
        // 供应商后台没传 status → 默认白名单
        conds.push("status IN (__VENDOR_STATUSES__)");
        for s in VENDOR_STATUSES {
            binds.push(Bind::I32(s));
        }
    } else if status_flag && VENDOR_STATUSES.contains(&status.unwrap_or_default()) {
        // 供应商后台传了 status → 必须在白名单内
        conds.push("status = ?");
        binds.push(Bind::I32(status.unwrap_or_default()));
    } else if status_flag {
        // 供应商后台传了非法 status → 查不到数据（Java 用 `-999` 占位）
        conds.push("status = ?");
        binds.push(Bind::I32(-999));
    }

    // 占位符必须与绑定参数个数一致，逐个子句展开（避免出现 `IN ()` 语法错误）
    let mut expanded = String::new();
    for (idx, cond) in conds.iter().enumerate() {
        if idx > 0 {
            expanded.push_str(" AND ");
        }
        match *cond {
            "order_id IN (__ORDER_IDS__)" => {
                let n = search_order_ids.map_or(0, <[i32]>::len);
                expanded.push_str(&format!("order_id IN ({})", placeholders(n)));
            }
            "status IN (__VENDOR_STATUSES__)" => {
                expanded.push_str(&format!(
                    "status IN ({})",
                    placeholders(VENDOR_STATUSES.len())
                ));
            }
            other => expanded.push_str(other),
        }
    }

    let where_sql = if expanded.is_empty() {
        String::new()
    } else {
        format!(" WHERE {expanded}")
    };
    (where_sql, binds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::trade::aftersales::AftersalesListQuery;

    fn sql_and_binds(
        q: &AftersalesListQuery,
        shop_id: Option<i32>,
        vendor_id: Option<i32>,
        is_vendor_admin: bool,
        order_ids: Option<&[i32]>,
    ) -> (String, Vec<String>) {
        let (sql, binds) = build_filter(q, shop_id, vendor_id, is_vendor_admin, order_ids);
        let vals = binds
            .iter()
            .map(|b| match b {
                Bind::I32(v) => v.to_string(),
                Bind::Str(v) => v.clone(),
                Bind::Decimal(v) => v.to_string(),
            })
            .collect();
        (sql, vals)
    }

    #[test]
    fn empty_query_has_no_where() {
        let q = AftersalesListQuery::default();
        let (sql, binds) = sql_and_binds(&q, None, None, false, None);
        assert_eq!(sql, "");
        assert!(binds.is_empty());
    }

    #[test]
    fn invalid_aftersale_type_is_ignored() {
        // Java：0 / 3 / 9 都不参与类型过滤（aftersalesTypeFlag = false）
        for t in [0, 3, 9] {
            let q = AftersalesListQuery {
                aftersale_type: Some(t),
                ..Default::default()
            };
            let (sql, binds) = sql_and_binds(&q, None, None, false, None);
            assert_eq!(sql, "", "type={t} 不应产生过滤条件");
            assert!(binds.is_empty());
        }
        // 1 / 2 参与过滤
        for t in [1, 2] {
            let q = AftersalesListQuery {
                aftersale_type: Some(t),
                ..Default::default()
            };
            let (sql, binds) = sql_and_binds(&q, None, None, false, None);
            assert_eq!(sql, " WHERE aftersale_type = ?");
            assert_eq!(binds, vec![t.to_string()]);
        }
    }

    #[test]
    fn shop_and_vendor_only_filter_when_positive() {
        let q = AftersalesListQuery::default();
        // 0 / 负数视为「非店铺、非供应商视角」
        let (sql, _) = sql_and_binds(&q, Some(0), Some(-1), false, None);
        assert_eq!(sql, "");
        let (sql, binds) = sql_and_binds(&q, Some(3), Some(7), false, None);
        assert_eq!(sql, " WHERE shop_id = ? AND vendor_id = ?");
        assert_eq!(binds, vec!["3".to_string(), "7".to_string()]);
    }

    #[test]
    fn normal_admin_status_filter_matches_java() {
        // 普通后台：status > 0 才过滤；status = 0 / 负数不过滤
        let q = AftersalesListQuery {
            status: Some(2),
            ..Default::default()
        };
        let (sql, binds) = sql_and_binds(&q, None, None, false, None);
        assert_eq!(sql, " WHERE status = ?");
        assert_eq!(binds, vec!["2".to_string()]);

        let q = AftersalesListQuery {
            status: Some(0),
            ..Default::default()
        };
        let (sql, _) = sql_and_binds(&q, None, None, false, None);
        assert_eq!(sql, "");
    }

    #[test]
    fn vendor_admin_status_whitelist_matches_java() {
        // 未传 status → 默认白名单 21/4/22/6
        let q = AftersalesListQuery::default();
        let (sql, binds) = sql_and_binds(&q, None, None, true, None);
        assert_eq!(sql, " WHERE status IN (?, ?, ?, ?)");
        assert_eq!(binds, vec!["21", "4", "22", "6"]);

        // 传了白名单内的状态 → 精确匹配
        let q = AftersalesListQuery {
            status: Some(6),
            ..Default::default()
        };
        let (sql, binds) = sql_and_binds(&q, None, None, true, None);
        assert_eq!(sql, " WHERE status = ?");
        assert_eq!(binds, vec!["6".to_string()]);

        // 传了白名单外的状态 → -999（必然查空）
        let q = AftersalesListQuery {
            status: Some(1),
            ..Default::default()
        };
        let (sql, binds) = sql_and_binds(&q, None, None, true, None);
        assert_eq!(sql, " WHERE status = ?");
        assert_eq!(binds, vec!["-999".to_string()]);
    }

    #[test]
    fn sn_and_order_ids_are_bound_in_order() {
        let q = AftersalesListQuery {
            aftersales_sn: Some("  SN1  ".into()),
            ..Default::default()
        };
        let (sql, binds) = sql_and_binds(&q, None, None, false, Some(&[5, 6]));
        assert_eq!(sql, " WHERE aftersales_sn LIKE ? AND order_id IN (?, ?)");
        assert_eq!(binds, vec!["%SN1%".to_string(), "5".to_string(), "6".to_string()]);

        // 空串不作为过滤条件
        let q = AftersalesListQuery {
            aftersales_sn: Some("   ".into()),
            ..Default::default()
        };
        let (sql, _) = sql_and_binds(&q, None, None, false, None);
        assert_eq!(sql, "");
    }

    #[test]
    fn user_id_filter_applies() {
        let q = AftersalesListQuery {
            user_id: Some(9),
            ..Default::default()
        };
        let (sql, binds) = sql_and_binds(&q, None, None, false, None);
        assert_eq!(sql, " WHERE user_id = ?");
        assert_eq!(binds, vec!["9".to_string()]);
    }

    #[test]
    fn placeholder_count_always_matches_bind_count() {
        // 回归：`IN` 子句的占位符个数必须与绑定个数一致，
        // 否则 MySQL 会因 `IN ()` / 参数不匹配报语法错误。
        let cases: Vec<(AftersalesListQuery, Option<i32>, Option<i32>, bool, Option<Vec<i32>>)> = vec![
            (AftersalesListQuery::default(), None, None, true, None),
            (AftersalesListQuery::default(), Some(3), Some(7), false, Some(vec![1, 2, 3])),
            (
                AftersalesListQuery {
                    status: Some(2),
                    aftersales_sn: Some("SN".into()),
                    ..Default::default()
                },
                Some(1),
                None,
                false,
                Some(vec![9]),
            ),
        ];
        for (q, shop, vendor, vendor_admin, ids) in cases {
            let (sql, binds) = sql_and_binds(&q, shop, vendor, vendor_admin, ids.as_deref());
            assert_eq!(
                sql.matches('?').count(),
                binds.len(),
                "占位符与绑定参数个数不一致: {sql} vs {binds:?}"
            );
        }
    }
}
