//! 订单管理域（trade::order_admin）仓储：`order` / `order_item` / `logistics_company` /
//! `product` / `user` / `shop` / `aftersales` / `aftersales_item` / `aftersales_log`。
//!
//! 列名严格对齐 `java/database/pure.sql`（`python tools/schema_parser.py --table <表名>`）：
//! - `order`：`order_id` / `order_sn` / `user_id` / `parent_order_id` / `order_status` /
//!   `shipping_status` / `pay_status` / `is_del` / `is_valid` / `shop_id` / `vendor_id` /
//!   `is_cycle_order` / `is_card_order` / `shipping_method` / `logistics_id` /
//!   `logistics_provider` / `logistics_name` / `tracking_no` / `shipping_time` 等
//! - `order_item`：`item_id` / `order_id` / `product_id` / `product_name` / `quantity` /
//!   `delivery_quantity` / `product_type`
//! - `aftersales`：`aftersale_id` / `aftersale_type` / `status` / `pics` / `description` /
//!   `reply` / `add_time` / `tracking_no` / `logistics_name` / `refund_amount` / `shop_id` /
//!   `audit_time` / `deal_time` / `final_time` / `return_goods_tip` / `close_type` / …
//! - `aftersales_item`：`aftersales_item_id` / `order_item_id` / `number` / `aftersale_id`
//!
//! 注意：`order` 是 MySQL 保留字，SQL 中必须使用反引号。
//! 查询统一使用运行时 `sqlx::query_as`（不使用 `query!` 宏）。

use std::collections::HashMap;

use sqlx::PgPool;
use nextrade_core::entities::{Aftersales, AftersalesItem, AftersalesLog, Order, OrderItem, Shop, User};
use nextrade_core::models::trade::order_admin::OrderListQuery;
use nextrade_core::{Error, Result};

/// 待绑定参数（按 `conds` 顺序 1:1 对应）。
#[derive(Debug, Clone)]
enum Bind {
    I32(i32),
    I64(i64),
    Str(String),
    Decimal(rust_decimal::Decimal),
    /// `NULL`（用于 `Option` 列为空的场景）
    Null,
}

impl Bind {
    /// `Option<i32>` → `Bind`。
    fn opt_i32(v: Option<i32>) -> Self {
        v.map_or(Self::Null, Self::I32)
    }

    /// `Option<String>` → `Bind`。
    fn opt_str<S: Into<String>>(v: Option<S>) -> Self {
        v.map_or(Self::Null, |s| Self::Str(s.into()))
    }

    /// `Option<Decimal>` → `Bind`。
    fn opt_decimal(v: Option<rust_decimal::Decimal>) -> Self {
        v.map_or(Self::Null, Self::Decimal)
    }
}

/// 把 `Bind` 列表按顺序绑定到查询上（宏展开以便同时适配不同返回行类型）。
macro_rules! bind_all {
    ($q:expr, $binds:expr) => {{
        let mut q = $q;
        for b in $binds.iter() {
            q = match b {
                Bind::I32(v) => q.bind(*v),
                Bind::I64(v) => q.bind(*v),
                Bind::Str(v) => q.bind(v.as_str()),
                Bind::Decimal(v) => q.bind(*v),
                Bind::Null => q.bind(Option::<i32>::None),
            };
        }
        q
    }};
}

/// 订单动态更新片段：列名来自代码内白名单（绝不来自请求），值一律走占位符绑定。
#[derive(Debug, Default, Clone)]
pub struct OrderPatch {
    sets: Vec<(String, Bind)>,
}

impl OrderPatch {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn i32_val(mut self, col: &str, v: i32) -> Self {
        self.sets.push((col.to_string(), Bind::I32(v)));
        self
    }

    #[must_use]
    pub fn opt_i32(mut self, col: &str, v: Option<i32>) -> Self {
        if let Some(v) = v {
            self.sets.push((col.to_string(), Bind::I32(v)));
        }
        self
    }

    #[must_use]
    pub fn str_val(mut self, col: &str, v: &str) -> Self {
        self.sets.push((col.to_string(), Bind::Str(v.to_string())));
        self
    }

    #[must_use]
    pub fn decimal(mut self, col: &str, v: rust_decimal::Decimal) -> Self {
        self.sets.push((col.to_string(), Bind::Decimal(v)));
        self
    }

    #[must_use]
    pub fn opt_decimal(mut self, col: &str, v: Option<rust_decimal::Decimal>) -> Self {
        if let Some(v) = v {
            self.sets.push((col.to_string(), Bind::Decimal(v)));
        }
        self
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    /// `SET` 子句内容（不含 `SET` 关键字）。
    fn sets_sql(&self) -> String {
        self.sets
            .iter()
            .map(|(c, _)| format!("{c} = ?"))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn binds(&self) -> Vec<Bind> {
        self.sets.iter().map(|(_, b)| b.clone()).collect()
    }
}

#[derive(Clone)]
pub struct OrderAdminRepository {
    pool: PgPool,
}

impl OrderAdminRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 订单 —————————————————————

    /// 按订单ID查询。
    pub async fn find_order(&self, order_id: i32) -> Result<Option<Order>> {
        sqlx::query_as::<_, Order>("SELECT * FROM \"order\" WHERE order_id = $1 LIMIT 1")
            .bind(order_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 子订单数量（对应 Java `detail` 中 `parent_order_id = id` 的 count，
    /// 用于判断失效父单是否已被拆单）。
    pub async fn count_children(&self, parent_order_id: i32) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM \"order\" WHERE parent_order_id = $1")
                .bind(parent_order_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0)
    }

    /// 订单分页（对应 Java `OrderServiceImpl#list` + `buildQueryWrapper`）。
    ///
    /// `platform_view` 对应 Java 的「非店铺/非供应商视角」
    /// （`getShopId() <= 0 && getVendorId() <= 0`）；平台视角会额外排除
    /// 未支付的拆单子单与已失效父单。
    pub async fn paginate_orders(
        &self,
        q: &OrderListQuery,
        platform_view: bool,
    ) -> Result<(u64, Vec<Order>)> {
        let (where_sql, binds) = build_filter(q, platform_view)?;
        let (limit, offset) = (q.page_size(), (q.page_no() - 1) * q.page_size());

        let total_sql = format!("SELECT COUNT(*) FROM \"order\"{where_sql}");
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&total_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows_sql = format!(
            "SELECT * FROM \"order\"{where_sql}{} LIMIT ? OFFSET ?",
            order_by_clause(q)
        );
        let rows = bind_all!(sqlx::query_as::<_, Order>(&rows_sql), binds)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 批量查询订单明细（列表页按当前页 orderId 一次取回，消除 N+1）。
    pub async fn find_items_by_order_ids(&self, order_ids: &[i32]) -> Result<Vec<OrderItem>> {
        if order_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; order_ids.len()].join(", ");
        let sql = format!(
            "SELECT * FROM order_item WHERE order_id IN ({placeholders}) ORDER BY order_id ASC, item_id ASC"
        );
        let mut q = sqlx::query_as::<_, OrderItem>(&sql);
        for id in order_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 查询单个订单的明细。
    pub async fn find_items(&self, order_id: i32) -> Result<Vec<OrderItem>> {
        sqlx::query_as::<_, OrderItem>(
            "SELECT * FROM order_item WHERE order_id = $1 ORDER BY item_id ASC",
        )
        .bind(order_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 删除订单的商品明细数量（发货拆单时 Java 会删除原订单明细，这里只统计）。
    pub async fn count_items(&self, order_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM order_item WHERE order_id = $1")
            .bind(order_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0)
    }

    // ————————————————————— 发货 —————————————————————

    /// 发货落库：**原子条件更新**（仅当订单仍为「已确认，待发货」时才写入），
    /// 返回是否更新成功。等价 Java `@Transactional` + `updateById` 的
    /// 「状态门 + 写入」组合，且并发重复发货只有一次能成功。
    #[allow(clippy::too_many_arguments)]
    pub async fn update_deliver(
        &self,
        order_id: i32,
        shipping_method: i32,
        shipping_status: i32,
        shipping_time: i32,
        logistics_id: i32,
        logistics_provider: i32,
        logistics_name: &str,
        tracking_no: &str,
    ) -> Result<bool> {
        let affected = sqlx::query(
            "UPDATE \"order\" SET
               order_status = 2,
               shipping_method = $1,
               shipping_status = $2,
               shipping_time = $3,
               logistics_id = $4,
               logistics_provider = $5,
               logistics_name = $6,
               tracking_no = $7
             WHERE order_id = $8 AND order_status = 1",
        )
        .bind(shipping_method)
        .bind(shipping_status)
        .bind(shipping_time)
        .bind(logistics_id)
        .bind(logistics_provider)
        .bind(logistics_name)
        .bind(tracking_no)
        .bind(order_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected > 0)
    }

    /// 物流公司名称（对应 Java `logisticsCompanyService.getById(id).getLogisticsName()`）。
    pub async fn find_company_name(&self, logistics_id: i32) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT logistics_name FROM logistics_company WHERE logistics_id = $1 LIMIT 1",
        )
        .bind(logistics_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|r| r.0))
    }

    /// 物流公司编码 + 名称（快递鸟查询需要 `logistics_code`）。
    pub async fn find_company_code_name(
        &self,
        logistics_id: i32,
    ) -> Result<Option<(String, String)>> {
        let row: Option<(String, String)> = sqlx::query_as(
            "SELECT logistics_code, logistics_name FROM logistics_company WHERE logistics_id = $1 LIMIT 1",
        )
        .bind(logistics_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row)
    }

    /// 按物流公司名称精确匹配 ID（批量发货导入：Java `eq(logisticsName).last("LIMIT 1")`）。
    pub async fn find_logistics_company_id_by_name(&self, name: &str) -> Result<Option<i32>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT logistics_id FROM logistics_company WHERE logistics_name = $1 LIMIT 1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|r| r.0))
    }

    // ————————————————————— 售后占用数量 —————————————————————

    /// 订单「售后中/已完成」占用数量合计（对应 Java
    /// `AftersalesServiceImpl#isOrderItemsFullyCovered` 的求和部分）。
    pub async fn aftersales_covered_quantity(&self, order_id: i32, statuses: &[i32]) -> Result<i64> {
        if statuses.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; statuses.len()].join(", ");
        let sql = format!(
            "SELECT CAST(COALESCE(SUM(ai.number), 0) AS BIGINT)
               FROM aftersales_item ai
               JOIN aftersales a ON a.aftersale_id = ai.aftersale_id
              WHERE a.order_id = $1
                AND a.status IN ({placeholders})
                AND ai.order_item_id IN (SELECT item_id FROM order_item WHERE order_id = $2)"
        );
        let mut q = sqlx::query_as::<_, (i64,)>(&sql).bind(order_id);
        for s in statuses {
            q = q.bind(*s);
        }
        let row = q
            .bind(order_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0)
    }

    /// 订单内所有明细数量合计（`isOrderItemsFullyCovered` 的对比基准）。
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

    /// 批量判定「整单售后进行中」的订单（对应 Java
    /// `AftersalesServiceImpl#isOrderItemsFullyCovered` 的按订单一次查询版本）。
    ///
    /// 语义：订单内售后明细数量合计 ≥ 订单内全部明细数量合计。
    pub async fn fully_covered_order_ids(
        &self,
        order_ids: &[i32],
        statuses: &[i32],
    ) -> Result<Vec<i32>> {
        if order_ids.is_empty() || statuses.is_empty() {
            return Ok(Vec::new());
        }
        let order_ph = vec!["?"; order_ids.len()].join(", ");
        let status_ph = vec!["?"; statuses.len()].join(", ");
        let sql = format!(
            "SELECT a.order_id
               FROM aftersales_item ai
               JOIN aftersales a ON a.aftersale_id = ai.aftersale_id
              WHERE a.order_id IN ({order_ph})
                AND a.status IN ({status_ph})
                AND ai.order_item_id IN (
                    SELECT item_id FROM order_item WHERE order_id = a.order_id
                )
              GROUP BY a.order_id
             HAVING CAST(COALESCE(SUM(ai.number), 0) AS BIGINT) >= (
                    SELECT CAST(COALESCE(SUM(quantity), 0) AS BIGINT)
                      FROM order_item WHERE order_id = a.order_id
                )"
        );
        let mut q = sqlx::query_as::<_, (i32,)>(&sql);
        for id in order_ids {
            q = q.bind(*id);
        }
        for s in statuses {
            q = q.bind(*s);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 仍在售（`product_status = 1`）的商品ID集合（对应 Java `hasAvailableRebuyItems`）。
    pub async fn on_sale_product_ids(&self, product_ids: &[i32]) -> Result<Vec<i32>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; product_ids.len()].join(", ");
        let sql = format!(
            "SELECT product_id FROM product WHERE product_status = 1 AND product_id IN ({placeholders})"
        );
        let mut q = sqlx::query_as::<_, (i32,)>(&sql);
        for id in product_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 批量查询拼团记录：`(order_id, groupon_promotion_record_id, join_status)`。
    ///
    /// 对应 Java `grouponPromotionRecordService`：同一订单存在多条时取第一条
    /// （Java `Collectors.toMap` 的 `(a, b) -> a` 语义）。
    pub async fn find_groupon_records(&self, order_ids: &[i32]) -> Result<Vec<(i32, i64, i32)>> {
        if order_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; order_ids.len()].join(", ");
        let sql = format!(
            "SELECT order_id, groupon_promotion_record_id, join_status
               FROM groupon_promotion_record
              WHERE order_id IN ({placeholders})
              ORDER BY groupon_promotion_record_id ASC"
        );
        let mut q = sqlx::query_as::<_, (i64, i64, i32)>(&sql);
        for id in order_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        let mut out: Vec<(i32, i64, i32)> = Vec::new();
        for (order_id, rec_id, join_status) in rows {
            let oid = i32::try_from(order_id).unwrap_or(i32::MAX);
            if out.iter().any(|(id, _, _)| *id == oid) {
                continue; // 保留首条，与 Java toMap 冲突合并策略一致
            }
            out.push((oid, rec_id, join_status));
        }
        Ok(out)
    }

    /// 按订单项统计有效售后占用数量（`order_item_id -> sum(number)`），
    /// 用于发货数量校验（对应 Java `getAftersalesQuantityMap`）。
    pub async fn aftersales_quantity_by_item(
        &self,
        order_id: i32,
        statuses: &[i32],
    ) -> Result<HashMap<i32, i32>> {
        if statuses.is_empty() {
            return Ok(HashMap::new());
        }
        let placeholders = vec!["?"; statuses.len()].join(", ");
        let sql = format!(
            "SELECT ai.order_item_id, CAST(COALESCE(SUM(ai.number), 0) AS BIGINT) AS n
               FROM aftersales_item ai
               JOIN aftersales a ON a.aftersale_id = ai.aftersale_id
              WHERE a.order_id = $1
                AND a.status IN ({placeholders})
                AND ai.order_item_id IS NOT NULL
              GROUP BY ai.order_item_id"
        );
        let mut q = sqlx::query_as::<_, (i32, i64)>(&sql).bind(order_id);
        for s in statuses {
            q = q.bind(*s);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(item_id, n)| (item_id, i32::try_from(n).unwrap_or(i32::MAX)))
            .collect())
    }

    /// 该订单项是否已有「售后处理中」的申请（对应 Java `getApplyAftersalesCount > 0`）。
    pub async fn has_applied_aftersales(
        &self,
        order_id: i32,
        order_item_id: i32,
        statuses: &[i32],
    ) -> Result<bool> {
        if statuses.is_empty() {
            return Ok(false);
        }
        let placeholders = vec!["?"; statuses.len()].join(", ");
        let sql = format!(
            "SELECT COUNT(*)
               FROM aftersales_item ai
               JOIN aftersales a ON a.aftersale_id = ai.aftersale_id
              WHERE a.order_id = $1
                AND a.status IN ({placeholders})
                AND ai.order_item_id = $2"
        );
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

    /// 写入售后申请单，返回 `aftersale_id`。
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

    /// 在同一事务内写入「售后申请单 + 申请明细 + 售后日志」，返回 `aftersale_id`。
    ///
    /// 对应 Java `AftersalesServiceImpl#afterSalesApply` 的
    /// `this.save(aftersales)` + `aftersalesItemService.saveBatch(...)` +
    /// `aftersalesLogMapper.insert(...)`（整体 `@Transactional`）。
    /// 明细与日志的 `aftersale_id` 由本方法写入实际自增值，入参中的该字段被忽略。
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
                "INSERT INTO aftersales_item (order_item_id, number, aftersale_id)
                 VALUES ($1, $2, $3)",
            )
            .bind(it.order_item_id)
            .bind(it.number)
            .bind(aftersale_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        }

        sqlx::query(
            "INSERT INTO aftersales_log
             (aftersale_id, log_info, add_time, admin_name, refund_money, refund_type,
              refund_desc, user_name, return_pic, shop_id, vendor_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
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
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;

        tx.commit().await.map_err(map_err)?;
        Ok(aftersale_id)
    }

    // ————————————————————— 关联数据 —————————————————————

    /// 批量取用户（订单列表/详情的用户信息）。
    pub async fn find_users(&self, user_ids: &[i32]) -> Result<Vec<User>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; user_ids.len()].join(", ");
        let sql = format!("SELECT * FROM user WHERE user_id IN ({placeholders})");
        let mut q = sqlx::query_as::<_, User>(&sql);
        for id in user_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量取店铺。
    pub async fn find_shops(&self, shop_ids: &[i32]) -> Result<Vec<Shop>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; shop_ids.len()].join(", ");
        let sql = format!("SELECT * FROM shop WHERE shop_id IN ({placeholders})");
        let mut q = sqlx::query_as::<_, Shop>(&sql);
        for id in shop_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 是否存在仍在售的商品（对应 Java `hasAvailableRebuyItems`：
    /// `product_status = 1` 即 Constants.YES）。
    pub async fn any_product_on_sale(&self, product_ids: &[i32]) -> Result<bool> {
        let on_sale = self.on_sale_product_ids(product_ids).await?;
        Ok(!on_sale.is_empty())
    }

    // ————————————————————— 管理端操作（wave3 扩展） —————————————————————

    /// 通用订单字段更新（列名由 [`OrderPatch`] 白名单保证，值走占位符绑定）。
    pub async fn update_order(&self, order_id: i32, patch: &OrderPatch) -> Result<u64> {
        if patch.is_empty() {
            return Ok(0);
        }
        let sql = format!("UPDATE \"order\" SET {} WHERE order_id = $1", patch.sets_sql());
        Ok(update_with_patch(&self.pool, &sql, patch, order_id, None)
            .await?
            .rows_affected())
    }

    /// 带状态门的订单更新（并发保护）：仅当当前 `order_status = expected` 时生效。
    pub async fn update_order_if_status(
        &self,
        order_id: i32,
        expected_status: i32,
        patch: &OrderPatch,
    ) -> Result<u64> {
        if patch.is_empty() {
            return Ok(0);
        }
        let sql = format!(
            "UPDATE \"order\" SET {} WHERE order_id = $1 AND order_status = $2",
            patch.sets_sql()
        );
        Ok(update_with_patch(&self.pool, &sql, patch, order_id, Some(expected_status))
            .await?
            .rows_affected())
    }

    /// 写订单操作日志（`order_log`；Java `OrderLogService#quickCreate` / `save`）。
    pub async fn insert_order_log(
        &self,
        order_id: i32,
        order_sn: &str,
        description: &str,
        admin_id: i32,
        shop_id: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO order_log (order_id, order_sn, admin_id, user_id, description, log_time, shop_id) \
             VALUES (?, ?, ?, 0, ?, ?, ?)",
        )
        .bind(order_id)
        .bind(order_sn)
        .bind(admin_id)
        .bind(description)
        .bind(now_secs())
        .bind(shop_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 写「无操作人」的订单日志（Java \"quickCreate\" 的 adminId/userId 缺省值）。
    pub async fn quick_order_log(
        &self,
        order_id: i32,
        order_sn: &str,
        description: &str,
    ) -> Result<()> {
        self.insert_order_log(order_id, order_sn, description, 0, 0)
            .await
    }

    /// 按订单号查询（批量发货导入 / 换单号唯一性校验）。
    pub async fn find_order_by_sn(&self, order_sn: &str) -> Result<Option<Order>> {
        sqlx::query_as::<_, Order>("SELECT * FROM \"order\" WHERE order_sn = ? LIMIT 1")
            .bind(order_sn)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 子订单列表（`parent_order_id = ?`，Java `lambdaQuery().eq(parentOrderId, id)`）。
    pub async fn find_children(&self, parent_order_id: i32) -> Result<Vec<Order>> {
        sqlx::query_as::<_, Order>(
            "SELECT * FROM \"order\" WHERE parent_order_id = $1 ORDER BY order_id ASC",
        )
        .bind(parent_order_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 子订单 ID 列表（`parentDetail` 的 `subOrderIds`）。
    pub async fn find_sub_order_ids(&self, parent_order_id: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> =
            sqlx::query_as("SELECT order_id FROM \"order\" WHERE parent_order_id = $1 ORDER BY order_id ASC")
                .bind(parent_order_id)
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 管理员导出列配置（`admin_user.order_export`，JSON 字符串数组）。
    pub async fn find_admin_order_export(&self, admin_id: i32) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT order_export FROM admin_user WHERE admin_id = $1 LIMIT 1")
                .bind(admin_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }

    /// 保存管理员导出列配置。
    pub async fn update_admin_order_export(&self, admin_id: i32, raw: &str) -> Result<u64> {
        Ok(sqlx::query("UPDATE admin_user SET order_export = $1 WHERE admin_id = $2")
            .bind(raw)
            .bind(admin_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected())
    }

    /// 读取配置项原始值（`config.biz_code`，Java `ConfigService#getConfigVal`）。
    pub async fn find_config_val(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1")
                .bind(biz_code)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }

    /// 电子面单日志（`orderPrintBill`，取最新一条）。
    pub async fn find_logistics_api_log(
        &self,
        order_id: i32,
    ) -> Result<Option<nextrade_core::entities::LogisticsApiLog>> {
        sqlx::query_as::<_, nextrade_core::entities::LogisticsApiLog>(
            "SELECT * FROM logistics_api_log WHERE order_id = $1 ORDER BY id DESC LIMIT 1",
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 写入电子面单日志（Java `logisticsApiLogMapper.insert`）。
    pub async fn insert_logistics_api_log(
        &self,
        order_id: i32,
        provider_type: i32,
        order_code: &str,
        logistic_code: &str,
        print_template: &str,
        add_time: i32,
    ) -> Result<u64> {
        let res: i64 = sqlx::query_scalar(
            "INSERT INTO logistics_api_log \
             (order_id, provider_type, order_code, logistic_code, add_time, print_template) \
             VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(order_id)
        .bind(provider_type)
        .bind(order_code)
        .bind(logistic_code)
        .bind(add_time)
        .bind(print_template)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res as u64)
    }

    /// 批量取订单支付方式（`paylog.pay_code`，导出用）。
    pub async fn find_pay_codes(&self, order_ids: &[i32]) -> Result<Vec<(i32, String)>> {
        if order_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; order_ids.len()].join(", ");
        let sql = format!(
            "SELECT order_id, pay_code FROM paylog WHERE order_id IN ({placeholders}) ORDER BY paylog_id ASC"
        );
        let mut q = sqlx::query_as::<_, (i32, String)>(&sql);
        for id in order_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量取商品（`getAddProductInfo` / 导出重量）。
    pub async fn find_products(&self, product_ids: &[i32]) -> Result<Vec<nextrade_core::entities::Product>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; product_ids.len()].join(", ");
        let sql = format!("SELECT * FROM product WHERE product_id IN ({placeholders})");
        let mut q = sqlx::query_as::<_, nextrade_core::entities::Product>(&sql);
        for id in product_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 地区名（`modifyConsignee`：按 `region_ids` 回填 `region_names`）。
    pub async fn find_region_names(&self, region_ids: &[i32]) -> Result<Vec<String>> {
        if region_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; region_ids.len()].join(", ");
        let fields = region_ids
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT region_id, region_name FROM region WHERE region_id IN ({placeholders}) \
             ORDER BY FIELD(region_id, {fields})"
        );
        let mut q = sqlx::query_as::<_, (i32, String)>(&sql);
        for id in region_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.1).collect())
    }

    /// 取消订单：返还 SKU / 商品库存，并扣回虚拟销量（Java `cancelOrder` 的库存部分）。
    pub async fn restore_stock(&self, product_id: i32, sku_id: i32, quantity: i32) -> Result<()> {
        if quantity <= 0 {
            return Ok(());
        }
        if sku_id > 0 {
            sqlx::query("UPDATE product_sku SET sku_stock = sku_stock + ? WHERE sku_id = ?")
                .bind(quantity)
                .bind(sku_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
        }
        sqlx::query("UPDATE product SET product_stock = product_stock + ? WHERE product_id = ?")
            .bind(quantity)
            .bind(product_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        sqlx::query(
            "UPDATE product SET virtual_sales = CASE WHEN virtual_sales >= $1 THEN virtual_sales - $2 ELSE 0 END WHERE product_id = $3",
        )
        .bind(quantity)
        .bind(quantity)
        .bind(product_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 拆单日志（`order_split_log`，取最新一条：`(parent_order_id, parent_order_data)`）。
    pub async fn find_order_split_log(
        &self,
        order_id: i32,
    ) -> Result<Option<(i32, Option<String>)>> {
        let row: Option<(i32, Option<String>)> = sqlx::query_as(
            "SELECT parent_order_id, parent_order_data FROM order_split_log \
             WHERE order_id = ? ORDER BY log_id DESC LIMIT 1",
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row)
    }

    /// 按订单状态聚合数量（\"statusCount\" 的单条 GROUP BY 版本）。
    pub async fn status_counts(
        &self,
        q: &OrderListQuery,
        platform_view: bool,
    ) -> Result<Vec<(i32, i64)>> {
        let mut plain = q.clone();
        plain.order_status = None;
        let (where_sql, binds) = build_filter(&plain, platform_view)?;
        let sql = format!(
            "SELECT order_status, COUNT(*) FROM \"order\"{where_sql} GROUP BY order_status"
        );
        let rows: Vec<(i32, i64)> = bind_all!(sqlx::query_as::<_, (i32, i64)>(&sql), binds)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows)
    }

    /// 复用列表筛选条件，按指定 `order_status`（`-2` = 已删除）计数。
    pub async fn count_with_status_filter(
        &self,
        q: &OrderListQuery,
        platform_view: bool,
        order_status: Option<&str>,
    ) -> Result<i64> {
        let mut copy = q.clone();
        copy.order_status = order_status.map(str::to_string);
        let (where_sql, binds) = build_filter(&copy, platform_view)?;
        let sql = format!("SELECT COUNT(*) FROM \"order\"{where_sql}");
        let row: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    /// 指定页/页大小的订单分页（`orderExport` 分批导出用，不受 BasePage 100 限制）。
    pub async fn paginate_orders_with(
        &self,
        q: &OrderListQuery,
        platform_view: bool,
        page: u64,
        size: u64,
    ) -> Result<(u64, Vec<Order>)> {
        let (where_sql, binds) = build_filter(q, platform_view)?;
        let limit = size.max(1);
        let offset = page.saturating_sub(1) * limit;
        let count_binds = binds.clone();
        let total_sql = format!("SELECT COUNT(*) FROM \"order\"{where_sql}");
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&total_sql), count_binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let rows_sql = format!(
            "SELECT * FROM \"order\"{where_sql}{} LIMIT ? OFFSET ?",
            order_by_clause(q)
        );
        let rows = bind_all!(sqlx::query_as::<_, Order>(&rows_sql), binds)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 查询订单店铺金额明细（`order_amount_detail`）。
    pub async fn find_amount_details(
        &self,
        order_id: i32,
    ) -> Result<Vec<nextrade_core::entities::OrderAmountDetail>> {
        sqlx::query_as::<_, nextrade_core::entities::OrderAmountDetail>(
            "SELECT * FROM order_amount_detail WHERE order_id = $1 ORDER BY order_discount_detail_id ASC",
        )
        .bind(order_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 更新店铺金额明细的优惠金额。
    pub async fn update_amount_detail_discount(
        &self,
        order_discount_detail_id: i32,
        discount_amount: rust_decimal::Decimal,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE order_amount_detail SET discount_amount = $1 WHERE order_discount_detail_id = $2",
        )
        .bind(discount_amount)
        .bind(order_discount_detail_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 批量更新订单明细价格（改价后按比例分摊折扣，Java `orderItemService.updateBatchById`）。
    pub async fn update_item_prices(
        &self,
        updates: &[(i32, rust_decimal::Decimal, Option<rust_decimal::Decimal>)],
    ) -> Result<()> {
        for (item_id, price, admin_pre) in updates {
            sqlx::query("UPDATE order_item SET price = $1, admin_change_pre_price = $2 WHERE item_id = $3")
                .bind(*price)
                .bind(*admin_pre)
                .bind(*item_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
        }
        Ok(())
    }

    /// 删除订单全部明细（`modifyProduct` 整批替换）。
    pub async fn delete_items(&self, order_id: i32) -> Result<u64> {
        Ok(sqlx::query("DELETE FROM order_item WHERE order_id = $1")
            .bind(order_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected())
    }

    /// 新增订单明细（`modifyProduct` 整批写入）。
    pub async fn insert_order_item(&self, it: &OrderItem) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO order_item (
                order_id, order_sn, user_id, price, quantity, product_id, product_name,
                product_sn, pic_thumb, sku_id, sku_data, delivery_quantity, product_type,
                is_gift, shop_id, is_pin, prepay_price, commission, origin_price,
                promotion_data, is_seckill, extra_sku_data, suppliers_id, card_group_name,
                vendor_product_id, vendor_product_sku_id, vendor_id, vendor_product_supply_price,
                admin_change_pre_price, delivery_service_fee, server_service_fee,
                is_wholesale_line, wholesale_grade_id, wholesale_base_price
             ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30,$31,$32,$33,$34) RETURNING item_id",
        )
        .bind(it.order_id)
        .bind(&it.order_sn)
        .bind(it.user_id)
        .bind(it.price)
        .bind(it.quantity)
        .bind(it.product_id)
        .bind(&it.product_name)
        .bind(&it.product_sn)
        .bind(&it.pic_thumb)
        .bind(it.sku_id)
        .bind(&it.sku_data)
        .bind(it.delivery_quantity)
        .bind(it.product_type)
        .bind(it.is_gift)
        .bind(it.shop_id)
        .bind(it.is_pin)
        .bind(it.prepay_price)
        .bind(&it.commission)
        .bind(it.origin_price)
        .bind(&it.promotion_data)
        .bind(it.is_seckill)
        .bind(&it.extra_sku_data)
        .bind(it.suppliers_id)
        .bind(&it.card_group_name)
        .bind(it.vendor_product_id)
        .bind(it.vendor_product_sku_id)
        .bind(it.vendor_id)
        .bind(it.vendor_product_supply_price)
        .bind(it.admin_change_pre_price)
        .bind(it.delivery_service_fee)
        .bind(it.server_service_fee)
        .bind(it.is_wholesale_line)
        .bind(it.wholesale_grade_id)
        .bind(it.wholesale_base_price)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 插入拆单记录（`order_split_log`）。
    pub async fn insert_order_split_log(
        &self,
        parent_order_id: i32,
        order_id: i32,
        split_time: i32,
        parent_order_data: &str,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO order_split_log (parent_order_id, order_id, split_time, parent_order_data)
             VALUES ($1, $2, $3, $4) RETURNING log_id",
        )
        .bind(parent_order_id)
        .bind(order_id)
        .bind(split_time)
        .bind(parent_order_data)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 迁移订单项所属订单 ID。
    pub async fn update_item_order_id(&self, item_id: i32, new_order_id: i32) -> Result<()> {
        sqlx::query("UPDATE order_item SET order_id = $1 WHERE item_id = $2")
            .bind(new_order_id)
            .bind(item_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 更新订单项数量。
    pub async fn update_item_quantity(&self, item_id: i32, quantity: i32) -> Result<()> {
        sqlx::query("UPDATE order_item SET quantity = $1 WHERE item_id = $2")
            .bind(quantity)
            .bind(item_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

/// 执行「订单字段更新」SQL（`update_order` / `update_order_if_status` 共用）。
async fn update_with_patch(
    pool: &PgPool,
    sql: &str,
    patch: &OrderPatch,
    order_id: i32,
    expect_status: Option<i32>,
) -> Result<sqlx::postgres::PgQueryResult> {
    let mut q = sqlx::query(sql);
    for b in patch.binds() {
        q = match b {
            Bind::I32(v) => q.bind(v),
            Bind::I64(v) => q.bind(v),
            Bind::Str(v) => q.bind(v),
            Bind::Decimal(v) => q.bind(v),
            Bind::Null => q.bind(Option::<i32>::None),
        };
    }
    q = q.bind(order_id);
    if let Some(status) = expect_status {
        q = q.bind(status);
    }
    q.execute(pool).await.map_err(map_err)
}

/// 当前秒级时间戳。
fn now_secs() -> i32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i32)
        .unwrap_or(0)
}

/// 售后申请单插入语句（`insert_aftersales` 与事务版共用，避免两处漂移）。
const AFTERSALES_INSERT: &str = "INSERT INTO aftersales
     (aftersale_type, status, pics, description, reply, add_time, tracking_no,
      logistics_name, return_address, aftersale_reason, aftersales_sn, order_id,
      user_id, refund_amount, shop_id, audit_time, deal_time, final_time,
      return_goods_tip, vendor_id, close_type)
     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21) RETURNING aftersale_id";

/// 构造 WHERE 子句（含前导空格）与绑定参数列表。
///
/// 对照 Java `OrderServiceImpl#buildQueryWrapper`；所有条件与绑定严格同序。
/// `platform_view`：平台视角（Java `HeaderUtils.getShopId() <= 0`）时附加
/// 「排除未支付拆单子单 / 已失效父单」条件；店铺、供应商、门店视角不附加。
fn build_filter(q: &OrderListQuery, platform_view: bool) -> Result<(String, Vec<Bind>)> {
    let mut conds: Vec<String> = Vec::new();
    let mut binds: Vec<Bind> = Vec::new();

    // 软删除：orderStatus=-2 查已删除，否则查未删除
    if q.only_deleted() {
        conds.push("is_del = 1".into());
    } else {
        conds.push("is_del = 0".into());
    }

    if let Some(uid) = q.user_id {
        conds.push("user_id = ?".into());
        binds.push(Bind::I32(uid));
    }

    if let Some(ref uids) = q.tag_user_ids {
        if uids.is_empty() {
            conds.push("user_id = -1".into());
        } else {
            let placeholders = vec!["?"; uids.len()].join(", ");
            conds.push(format!("user_id IN ({placeholders})"));
            for id in uids {
                binds.push(Bind::I32(*id));
            }
        }
    }

    let statuses = q.order_status_codes();
    match statuses.len() {
        0 => {}
        1 => {
            conds.push("order_status = ?".into());
            binds.push(Bind::I32(statuses[0]));
        }
        n => {
            conds.push(format!("order_status IN ({})", vec!["?"; n].join(", ")));
            for s in &statuses {
                binds.push(Bind::I32(*s));
            }
        }
    }

    if q.invoice_scope == Some(true) {
        conds.push(
            "(order_status = 5 OR (invoice_data IS NOT NULL AND invoice_data <> '' AND invoice_data <> '[]'))"
                .into(),
        );
    }

    if let Some(ids) = q.order_ids.as_ref().filter(|v| !v.is_empty()) {
        conds.push(format!("order_id IN ({})", vec!["?"; ids.len()].join(", ")));
        for id in ids {
            binds.push(Bind::I32(*id));
        }
    }

    if q.include_invalid != Some(true) {
        conds.push("is_valid = 1".into());
    }

    // 平台视角（Java `getShopId() <= 0 && getVendorId() <= 0`）：排除未支付的拆单子单
    if platform_view {
        conds.push(
            "(parent_order_id <= 0 OR parent_order_id IS NULL OR pay_status <> 0 \
             OR is_cycle_order = 1 OR is_card_order = 1)"
                .into(),
        );
    }

    if let Some(shop_id) = q.shop_id.filter(|v| *v > 0) {
        conds.push("shop_id = ?".into());
        binds.push(Bind::I32(shop_id));
    }

    match q.vendor_id {
        Some(-2) => conds.push("vendor_id > 0".into()),
        Some(v) if v > 0 => {
            conds.push("vendor_id = ?".into());
            binds.push(Bind::I32(v));
        }
        _ => {}
    }

    if let Some(is_settlement) = q.is_settlement.filter(|v| *v > -1) {
        conds.push("is_settlement = ?".into());
        binds.push(Bind::I32(is_settlement));
        if is_settlement == 0 {
            conds.push(
                "order_id NOT IN (SELECT order_id FROM groupon_promotion_record WHERE join_status = 1)"
                    .into(),
            );
        }
    }

    if let Some(v) = q.is_exchange_order.filter(|v| *v > -1) {
        conds.push("is_exchange_order = ?".into());
        binds.push(Bind::I32(v));
    }

    if let Some(v) = q.is_cycle_order.filter(|v| *v > -1) {
        conds.push("is_cycle_order = ?".into());
        binds.push(Bind::I32(v));
    }

    if let Some(v) = q.pay_status.filter(|v| *v != -1) {
        conds.push("pay_status = ?".into());
        binds.push(Bind::I32(v));
    }

    if let Some(v) = q.shipping_status.filter(|v| *v != -1) {
        conds.push("shipping_status = ?".into());
        binds.push(Bind::I32(v));
    }

    if let Some(v) = q.comment_status.filter(|v| *v != -1) {
        if v > 0 {
            conds.push("comment_status = ?".into());
            binds.push(Bind::I32(v));
        } else {
            // Java：comment_status = 0 且订单已完成
            conds.push("comment_status = 0 AND order_status = 5".into());
        }
    }

    if let Some(addr) = non_empty(&q.address) {
        let like = format!("%{addr}%");
        conds.push("(address LIKE ? OR region_names LIKE ?)".into());
        binds.push(Bind::Str(like.clone()));
        binds.push(Bind::Str(like));
    }

    if let Some(email) = non_empty(&q.email) {
        conds.push("email LIKE ?".into());
        binds.push(Bind::Str(format!("%{email}%")));
    }

    if let Some(mobile) = non_empty(&q.mobile) {
        conds.push("mobile LIKE ?".into());
        binds.push(Bind::Str(format!("%{mobile}%")));
    }

    if let Some(logistics_id) = q.logistics_id.filter(|v| *v > 0) {
        conds.push("logistics_id = ?".into());
        binds.push(Bind::I64(logistics_id));
    }

    if let Some(start) = non_empty(&q.add_start_time) {
        conds.push("add_time >= ?".into());
        binds.push(Bind::I64(day_start_seconds(start)?));
    }

    if let Some(end) = non_empty(&q.add_end_time) {
        conds.push("add_time <= ?".into());
        binds.push(Bind::I64(day_end_seconds(end)?));
    }

    if let Some(pay_time) = non_empty(&q.pay_time) {
        let (start, end) = parse_range_seconds(pay_time)?;
        conds.push("pay_time >= ?".into());
        binds.push(Bind::I64(start));
        conds.push("pay_time <= ?".into());
        binds.push(Bind::I64(end));
    }

    if let Some(data_type) = q.data_type {
        // PC 端日期检索：1 近30天 2 近60天 3 近365天 4 近1天（其他值不限）
        let days = match data_type {
            1 => Some(30i64),
            2 => Some(60),
            3 => Some(365),
            4 => Some(1),
            _ => None,
        };
        if let Some(days) = days {
            let now = nextrade_common::utils::now_seconds();
            conds.push("add_time >= ? AND add_time <= ?".into());
            binds.push(Bind::I64(now - days * 86_400));
            binds.push(Bind::I64(now));
        }
    }

    if let Some(mark) = q.mark.filter(|v| *v != -1) {
        conds.push("mark = ?".into());
        binds.push(Bind::I32(mark));
    }

    if let Some(pay_type_id) = q.pay_type_id {
        conds.push("pay_type_id = ?".into());
        binds.push(Bind::I32(pay_type_id));
    }

    if let Some(pay_code) = non_empty(&q.pay_code) {
        if pay_code == "balance" {
            conds.push("balance > 0".into());
        } else {
            // 按支付日志筛选已支付订单（Java 用 leftJoin + groupBy 实现同一语义）
            conds.push(
                "EXISTS (SELECT 1 FROM paylog p WHERE p.order_id = \"order\".order_id \
                 AND p.order_type = 0 AND p.pay_code = ? AND p.pay_status = 2)"
                    .into(),
            );
            binds.push(Bind::Str(pay_code.to_string()));
        }
    }

    if let Some(source) = non_empty(&q.order_source) {
        let items: Vec<&str> = source
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if !items.is_empty() {
            conds.push(format!("order_source IN ({})", vec!["?"; items.len()].join(", ")));
            for s in items {
                binds.push(Bind::Str(s.to_string()));
            }
        }
    }

    // 微信直播归因筛选（Java 走 wechat_live_order_rel）
    let live_filter = non_empty(&q.order_sub_source).is_some()
        || q.wechat_live_id.is_some()
        || q.wechat_live_room_id.is_some()
        || q.wechat_goods_id.is_some()
        || non_empty(&q.live_share_open_id).is_some()
        || q.live_sync_status.is_some();
    if live_filter {
        let mut sub = vec!["r.order_id = \"order\".order_id".to_string()];
        if let Some(v) = non_empty(&q.order_sub_source) {
            sub.push("r.sub_source = ?".into());
            binds.push(Bind::Str(v.to_string()));
        }
        if let Some(v) = q.wechat_live_id {
            sub.push("r.wechat_live_id = ?".into());
            binds.push(Bind::I32(v));
        }
        if let Some(v) = q.wechat_live_room_id {
            sub.push("r.room_id = ?".into());
            binds.push(Bind::I32(v));
        }
        if let Some(v) = q.wechat_goods_id {
            sub.push("r.wechat_goods_id = ?".into());
            binds.push(Bind::I32(v));
        }
        if let Some(v) = non_empty(&q.live_share_open_id) {
            sub.push("r.share_open_id = ?".into());
            binds.push(Bind::Str(v.to_string()));
        }
        if let Some(v) = q.live_sync_status {
            sub.push("r.sync_status = ?".into());
            binds.push(Bind::I32(v));
        }
        conds.push(format!(
            "EXISTS (SELECT 1 FROM wechat_live_order_rel r WHERE {})",
            sub.join(" AND ")
        ));
    }

    if let Some(pid) = q.parent_order_id.filter(|v| *v > 0) {
        conds.push("parent_order_id = ?".into());
        binds.push(Bind::I32(pid));
    }

    if let Some(sn) = non_empty(&q.out_trade_no) {
        conds.push("out_trade_no LIKE ?".into());
        binds.push(Bind::Str(format!("%{sn}%")));
    }

    // 待晒单：Java 先按 (user_id, is_showed) 取评论订单ID，再 IN 过滤；
    // 无 user_id 时 `user_id = NULL` 匹配不到任何行 → 结果为空。
    if let Some(is_showed) = q.is_showed.filter(|v| *v > -1) {
        match q.user_id {
            None => conds.push("1 = 0".into()),
            Some(uid) => {
                conds.push(
                    "EXISTS (SELECT 1 FROM comment c WHERE c.order_id = \"order\".order_id \
                     AND c.user_id = ? AND c.is_showed = ?)"
                        .into(),
                );
                binds.push(Bind::I32(uid));
                binds.push(Bind::I32(is_showed));
            }
        }
    }

    // 关键词：订单号 / 收货人 / 商品名 / 订单ID
    if let Some(kw) = non_empty(&q.keyword) {
        let like = format!("%{kw}%");
        let mut parts = vec![
            "order_sn LIKE ?".to_string(),
            "consignee LIKE ?".to_string(),
            "EXISTS (SELECT 1 FROM order_item oi WHERE oi.order_id = \"order\".order_id \
             AND oi.product_name LIKE ?)"
                .to_string(),
        ];
        binds.push(Bind::Str(like.clone()));
        binds.push(Bind::Str(like.clone()));
        binds.push(Bind::Str(like));
        // Java `eq(Order::getOrderId, keyword)`：订单ID精确匹配（非数字时跳过）
        if let Ok(id) = kw.parse::<i32>() {
            parts.push("order_id = ?".into());
            binds.push(Bind::I32(id));
        }
        conds.push(format!("({})", parts.join(" OR ")));
    }

    if conds.is_empty() {
        return Ok((String::new(), binds));
    }
    Ok((format!(" WHERE {}", conds.join(" AND ")), binds))
}

/// ORDER BY 子句（含前导空格）。
///
/// Java `BaseServiceImpl#buildSortOrder`：无排序字段时按主键 `order_id` 降序；
/// 字段名经白名单映射为真实列名，防止 SQL 注入（Java 亦在找不到属性时回退主键）。
fn order_by_clause(q: &OrderListQuery) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(items) = q.sort_items.as_ref().filter(|v| !v.is_empty()) {
        for it in items {
            let Some(field) = it.field.as_deref() else { continue };
            let Some(col) = sort_column(field) else { continue };
            let dir = if it.order.as_deref().is_some_and(|o| o.eq_ignore_ascii_case("asc")) {
                "ASC"
            } else {
                "DESC"
            };
            parts.push(format!("{col} {dir}"));
        }
    }
    if parts.is_empty() {
        let col = q
            .sort_field
            .as_deref()
            .and_then(sort_column)
            .unwrap_or("order_id");
        let dir = if q
            .sort_order
            .as_deref()
            .is_some_and(|o| o.eq_ignore_ascii_case("asc"))
        {
            "ASC"
        } else {
            "DESC"
        };
        parts.push(format!("{col} {dir}"));
    }
    format!(" ORDER BY {}", parts.join(", "))
}

/// 排序字段白名单：Java 属性名（驼峰）或真实列名 → 列名。
fn sort_column(field: &str) -> Option<&'static str> {
    match field {
        "orderId" | "order_id" => Some("order_id"),
        "orderSn" | "order_sn" => Some("order_sn"),
        "userId" | "user_id" => Some("user_id"),
        "orderStatus" | "order_status" => Some("order_status"),
        "shippingStatus" | "shipping_status" => Some("shipping_status"),
        "payStatus" | "pay_status" => Some("pay_status"),
        "addTime" | "add_time" => Some("add_time"),
        "payTime" | "pay_time" => Some("pay_time"),
        "shippingTime" | "shipping_time" => Some("shipping_time"),
        "receivedTime" | "received_time" => Some("received_time"),
        "totalAmount" | "total_amount" => Some("total_amount"),
        "paidAmount" | "paid_amount" => Some("paid_amount"),
        "shopId" | "shop_id" => Some("shop_id"),
        "mark" => Some("mark"),
        "parentOrderId" | "parent_order_id" => Some("parent_order_id"),
        "commentStatus" | "comment_status" => Some("comment_status"),
        _ => None,
    }
}

/// 取非空的去空白字符串。
fn non_empty(v: &Option<String>) -> Option<&str> {
    v.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

/// `yyyy-MM-dd` → 当日 00:00:00 秒级时间戳（服务器本地时区，对齐 Java
/// `LocalDate.atStartOfDay().atZone(ZoneId.systemDefault())`）。
fn day_start_seconds(date: &str) -> Result<i64> {
    let (y, m, d) = parse_ymd(date)?;
    let naive = chrono::NaiveDate::from_ymd_opt(y, m, d)
        .ok_or_else(|| Error::Validation(format!("日期不合法: {date}")))?
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| Error::Validation(format!("日期不合法: {date}")))?;
    Ok(local_seconds(naive))
}

/// `yyyy-MM-dd` → 当日 23:59:59 秒级时间戳（对齐 Java `atTime(LocalTime.MAX)`）。
fn day_end_seconds(date: &str) -> Result<i64> {
    let (y, m, d) = parse_ymd(date)?;
    let naive = chrono::NaiveDate::from_ymd_opt(y, m, d)
        .ok_or_else(|| Error::Validation(format!("日期不合法: {date}")))?
        .and_hms_opt(23, 59, 59)
        .ok_or_else(|| Error::Validation(format!("日期不合法: {date}")))?;
    Ok(local_seconds(naive))
}

/// 解析 `开始,结束` 日期区间为秒级时间戳（结束日含当日 23:59:59）。
fn parse_range_seconds(range: &str) -> Result<(i64, i64)> {
    let mut it = range.split(',');
    let start = it
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Validation("支付时间区间格式错误".into()))?;
    let end = it.next().map(str::trim).filter(|s| !s.is_empty());
    let start_s = day_start_seconds(start)?;
    let end_s = match end {
        Some(e) => day_end_seconds(e)?,
        None => day_end_seconds(start)?,
    };
    Ok((start_s, end_s))
}

fn parse_ymd(date: &str) -> Result<(i32, u32, u32)> {
    let mut it = date.split('-');
    let y = it.next().and_then(|v| v.trim().parse::<i32>().ok());
    let m = it.next().and_then(|v| v.trim().parse::<u32>().ok());
    let d = it.next().and_then(|v| v.trim().parse::<u32>().ok());
    match (y, m, d) {
        (Some(y), Some(m), Some(d)) => Ok((y, m, d)),
        _ => Err(Error::Validation(format!("日期格式应为 yyyy-MM-dd: {date}"))),
    }
}

/// 本地时区的 naive 时间 → 秒级时间戳。
fn local_seconds(naive: chrono::NaiveDateTime) -> i64 {
    use chrono::TimeZone;
    match chrono::Local.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => dt.timestamp(),
        chrono::LocalResult::Ambiguous(dt, _) => dt.timestamp(),
        chrono::LocalResult::None => naive.and_utc().timestamp(),
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("订单不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_defaults_to_not_deleted_and_valid() {
        let q = OrderListQuery::default();
        let (sql, binds) = build_filter(&q, true).unwrap();
        assert!(sql.contains("is_del = 0"));
        assert!(sql.contains("is_valid = 1"));
        // 平台视角会带排除未支付拆单子单的条件
        assert!(sql.contains("parent_order_id <= 0"));
        assert!(binds.is_empty());

        // 店铺/供应商视角不附加该条件
        let (sql, _) = build_filter(&q, false).unwrap();
        assert!(!sql.contains("parent_order_id <= 0"));
    }

    #[test]
    fn filter_deleted_semantics_and_status_list() {
        let q: OrderListQuery =
            serde_json::from_str(r#"{"orderStatus":"1,2","shopId":5}"#).unwrap();
        let (sql, binds) = build_filter(&q, false).unwrap();
        assert!(sql.contains("order_status IN (?, ?)"));
        assert!(sql.contains("shop_id = ?"));
        assert_eq!(binds.len(), 3);
    }

    #[test]
    fn filter_binds_are_in_same_order_as_placeholders() {
        let q: OrderListQuery = serde_json::from_str(
            r#"{"userId":9,"keyword":"TIG","payCode":"wechat","vendorId":-2,
                "addStartTime":"2026-01-01","addEndTime":"2026-01-31"}"#,
        )
        .unwrap();
        let (sql, binds) = build_filter(&q, true).unwrap();
        let placeholders = sql.matches('?').count();
        assert_eq!(placeholders, binds.len(), "占位符与绑定数量必须一致: {sql}");
    }

    #[test]
    fn keyword_without_numeric_id_has_no_id_condition() {
        let q: OrderListQuery = serde_json::from_str(r#"{"keyword":"TIG"}"#).unwrap();
        let (sql, _) = build_filter(&q, true).unwrap();
        assert!(sql.contains("order_sn LIKE ?"));
        assert!(sql.contains("oi.product_name LIKE ?"));
        assert!(!sql.contains("order_id = ?"));

        let q: OrderListQuery = serde_json::from_str(r#"{"keyword":"10086"}"#).unwrap();
        let (sql, binds) = build_filter(&q, true).unwrap();
        assert!(sql.contains("order_id = ?"));
        assert_eq!(sql.matches('?').count(), binds.len());
    }

    #[test]
    fn nested_live_and_showed_filters_keep_bind_order() {
        let q: OrderListQuery = serde_json::from_str(
            r#"{"orderSubSource":"live","wechatLiveId":3,"wechatLiveRoomId":4,"wechatGoodsId":5,
                "liveShareOpenId":"open","liveSyncStatus":1,"userId":2,"isShowed":0}"#,
        )
        .unwrap();
        let (sql, binds) = build_filter(&q, true).unwrap();
        assert!(sql.contains("wechat_live_order_rel r"));
        assert!(sql.contains("comment c"));
        assert_eq!(sql.matches('?').count(), binds.len());
    }

    #[test]
    fn order_by_defaults_to_primary_key_desc() {
        let q = OrderListQuery::default();
        assert_eq!(order_by_clause(&q), " ORDER BY order_id DESC");
    }

    #[test]
    fn order_by_uses_whitelist_and_ignores_unknown() {
        let q: OrderListQuery =
            serde_json::from_str(r#"{"sortField":"addTime","sortOrder":"asc"}"#).unwrap();
        assert_eq!(order_by_clause(&q), " ORDER BY add_time ASC");

        // 未在白名单内的字段被忽略 → 回退主键降序（防注入）
        let q: OrderListQuery =
            serde_json::from_str(r#"{"sortField":"1;DROP TABLE \"order\"","sortOrder":"asc"}"#)
                .unwrap();
        assert_eq!(order_by_clause(&q), " ORDER BY order_id ASC");

        let q: OrderListQuery = serde_json::from_str(
            r#"{"sortItems":[{"field":"totalAmount","order":"desc"},{"field":"bad","order":"asc"}]}"#,
        )
        .unwrap();
        assert_eq!(order_by_clause(&q), " ORDER BY total_amount DESC");
    }

    #[test]
    fn date_parsing_matches_java_day_bounds() {
        // 2026-01-01 00:00:00 → 2026-01-31 23:59:59（本地时区；容差 ±2h 以兼容夏令时地区）
        let s = day_start_seconds("2026-01-01").unwrap();
        let e = day_end_seconds("2026-01-31").unwrap();
        let expected = 30 * 86_400 + 86_399;
        assert!(
            (expected - 7_200..=expected + 7_200).contains(&(e - s)),
            "实际跨度 {}",
            e - s
        );

        let (rs, re) = parse_range_seconds("2026-01-01,2026-01-02").unwrap();
        let expected = 86_400 + 86_399;
        assert!((expected - 7_200..=expected + 7_200).contains(&(re - rs)));

        // 只有开始日期时，结束取当日 23:59:59
        let (rs2, re2) = parse_range_seconds("2026-01-01").unwrap();
        assert_eq!(rs2, rs);
        assert_eq!(re2, day_end_seconds("2026-01-01").unwrap());

        assert!(day_start_seconds("2026-13-01").is_err());
        assert!(day_start_seconds("bad").is_err());
    }
}
