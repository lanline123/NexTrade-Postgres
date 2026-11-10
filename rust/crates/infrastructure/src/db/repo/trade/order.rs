//! `order` / `order_item` 仓储，替代 `OrderMapper`。
//!
//! 注意：`ORDER` 是 MySQL 保留字，SQL 中必须使用反引号。

use sqlx::PgPool;
use nextrade_core::entities::{Order, OrderItem};
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct OrderRepository {
    pool: PgPool,
}

impl OrderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层执行扩展更新语句，如发货信息）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 开启事务（供服务层把库存扣减与订单写入纳入同一事务）。
    pub async fn begin(
        &self,
    ) -> std::result::Result<sqlx::Transaction<'static, sqlx::Postgres>, sqlx::Error> {
        self.pool.begin().await
    }

    /// 写入发货信息（运单号/物流公司/发货时间）。
    pub async fn update_shipping(
        &self,
        order_id: i32,
        tracking_no: &str,
        logistics_name: &str,
        shipping_time: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE \"order\" SET tracking_no = $1, logistics_name = $2, shipping_time = $3, shipping_status = 1 WHERE order_id = $4",
        )
        .bind(tracking_no)
        .bind(logistics_name)
        .bind(shipping_time)
        .bind(order_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 卡密订单下发卡券后自动置为“待收货”（无需物流发货）。
    pub async fn mark_card_shipped(&self, order_id: i32, shipping_time: i32) -> Result<()> {
        sqlx::query(
            "UPDATE \"order\" SET order_status = 2, shipping_status = 1, shipping_time = $1 WHERE order_id = $2",
        )
        .bind(shipping_time)
        .bind(order_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 按订单ID查询。
    pub async fn find_by_id(&self, order_id: i32) -> Result<Option<Order>> {
        sqlx::query_as::<_, Order>("SELECT \"order_id\", \"order_sn\", \"user_id\", \"parent_order_id\", \"parent_order_sn\", \"order_status\", \"shipping_status\", \"pay_status\", \"add_time\", \"consignee\", \"address\", \"region_ids\", \"region_names\", \"address_data\", \"mobile\", \"email\", \"buyer_note\", \"admin_note\", \"shipping_method\", \"logistics_id\", \"logistics_provider\", \"logistics_name\", \"shipping_type_id\", \"shipping_type_name\", \"tracking_no\", \"shipping_time\", \"received_time\", \"pay_type_id\", \"pay_time\", \"use_points\", \"is_need_commisson\", \"distribution_status\", \"referrer_user_id\", \"is_del\", \"shop_id\", \"is_store_splited\", \"is_valid\", \"comment_status\", \"total_amount\", \"paid_amount\", \"unpaid_amount\", \"unrefund_amount\", \"product_amount\", \"coupon_amount\", \"points_amount\", \"discount_amount\", \"balance\", \"credit_amount\", \"is_credit_order\", \"online_paid_amount\", \"offline_paid_amount\", \"service_fee\", \"shipping_fee\", \"invoice_fee\", \"order_extension\", \"order_source\", \"invoice_data\", \"out_trade_no\", \"is_settlement\", \"is_exchange_order\", \"order_type\", \"is_cycle_order\", \"is_card_order\", \"is_presale\", \"presale_type\", \"mark\", \"vendor_id\", \"cancel_remark\", \"cash_amount\", \"cash_change_amount\", \"round_off_dime_amount\", \"round_off_cents_amount\", \"pos_user_id\", \"device_code\", \"discount_change_price\", \"tax_rate\", \"tax_rate_amount\", \"change_price_data\"::text AS \"change_price_data\", \"gift_card_amount\", \"delivery_date\"::text AS \"delivery_date\", \"delivery_time_slot\", \"period_index\", \"time_index\", \"table_id\", \"table_no\", \"table_area_name\", \"dining_count\" FROM \"order\" WHERE order_id = $1 LIMIT 1")
            .bind(order_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按订单编号查询。
    pub async fn find_by_sn(&self, order_sn: &str) -> Result<Option<Order>> {
        sqlx::query_as::<_, Order>("SELECT \"order_id\", \"order_sn\", \"user_id\", \"parent_order_id\", \"parent_order_sn\", \"order_status\", \"shipping_status\", \"pay_status\", \"add_time\", \"consignee\", \"address\", \"region_ids\", \"region_names\", \"address_data\", \"mobile\", \"email\", \"buyer_note\", \"admin_note\", \"shipping_method\", \"logistics_id\", \"logistics_provider\", \"logistics_name\", \"shipping_type_id\", \"shipping_type_name\", \"tracking_no\", \"shipping_time\", \"received_time\", \"pay_type_id\", \"pay_time\", \"use_points\", \"is_need_commisson\", \"distribution_status\", \"referrer_user_id\", \"is_del\", \"shop_id\", \"is_store_splited\", \"is_valid\", \"comment_status\", \"total_amount\", \"paid_amount\", \"unpaid_amount\", \"unrefund_amount\", \"product_amount\", \"coupon_amount\", \"points_amount\", \"discount_amount\", \"balance\", \"credit_amount\", \"is_credit_order\", \"online_paid_amount\", \"offline_paid_amount\", \"service_fee\", \"shipping_fee\", \"invoice_fee\", \"order_extension\", \"order_source\", \"invoice_data\", \"out_trade_no\", \"is_settlement\", \"is_exchange_order\", \"order_type\", \"is_cycle_order\", \"is_card_order\", \"is_presale\", \"presale_type\", \"mark\", \"vendor_id\", \"cancel_remark\", \"cash_amount\", \"cash_change_amount\", \"round_off_dime_amount\", \"round_off_cents_amount\", \"pos_user_id\", \"device_code\", \"discount_change_price\", \"tax_rate\", \"tax_rate_amount\", \"change_price_data\"::text AS \"change_price_data\", \"gift_card_amount\", \"delivery_date\"::text AS \"delivery_date\", \"delivery_time_slot\", \"period_index\", \"time_index\", \"table_id\", \"table_no\", \"table_area_name\", \"dining_count\" FROM \"order\" WHERE order_sn = $1 LIMIT 1")
            .bind(order_sn)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 订单编号是否已存在（下单防重）。
    pub async fn sn_exists(&self, order_sn: &str) -> Result<bool> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM \"order\" WHERE order_sn = $1",
        )
        .bind(order_sn)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0 > 0)
    }

    /// 插入订单（含明细，独立事务内执行）。
    ///
    /// 若调用方已有事务（如需把库存扣减一并纳入），请用 [`Self::insert_order_in_tx`]：
    /// 否则库存与订单不在同一事务，订单写失败时会泄漏已扣库存。
    pub async fn insert_order(&self, order: &Order, items: &[OrderItem]) -> Result<i32> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        let order_id = self.insert_order_in_tx(&mut tx, order, items).await?;
        tx.commit().await.map_err(map_err)?;
        Ok(order_id)
    }

    /// 在调用方给定的事务内插入订单（含明细），不负责提交。
    ///
    /// 与库存扣减共用同一事务，保证「扣库存 + 写订单」原子性：
    /// 任一步失败则整体回滚，不残留已扣库存。
    pub async fn insert_order_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        order: &Order,
        items: &[OrderItem],
    ) -> Result<i32> {
        let result: i32 = sqlx::query_scalar(
            "INSERT INTO \"order\"
             (order_type, order_sn, user_id, order_status, shipping_status, pay_status,
              add_time, consignee, address, region_ids, region_names, mobile,
              buyer_note, shipping_method, pay_type_id, shop_id, is_valid, is_del,
              total_amount, paid_amount, unpaid_amount, product_amount, coupon_amount,
              shipping_fee, referrer_user_id, order_source, vendor_id, order_extension,
              email, use_points, points_amount, balance, is_presale, presale_type,
              discount_amount, tax_rate, tax_rate_amount, parent_order_id, parent_order_sn,
              is_store_splited, pos_user_id, admin_note, logistics_name, shipping_type_name,
              tracking_no, table_no, table_area_name)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18,
                     $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36, $37, $38, $39, $40, $41,
                     $42, $43, $44, $45, $46, $47) RETURNING order_id",
        )
        .bind(order.order_type.unwrap_or(0))
        .bind(&order.order_sn)
        .bind(order.user_id)
        .bind(order.order_status)
        .bind(order.shipping_status)
        .bind(order.pay_status)
        .bind(order.add_time)
        .bind(&order.consignee)
        .bind(&order.address)
        .bind(&order.region_ids)
        .bind(&order.region_names)
        .bind(&order.mobile)
        .bind(&order.buyer_note)
        .bind(order.shipping_method)
        .bind(order.pay_type_id)
        .bind(order.shop_id)
        .bind(order.is_valid)
        .bind(order.is_del)
        .bind(order.total_amount)
        .bind(order.paid_amount)
        .bind(order.unpaid_amount)
        .bind(order.product_amount)
        .bind(order.coupon_amount)
        .bind(order.shipping_fee)
        .bind(order.referrer_user_id)
        .bind(&order.order_source)
        .bind(order.vendor_id)
        .bind(&order.order_extension)
        .bind(&order.email)
        .bind(order.use_points)
        .bind(order.points_amount)
        .bind(order.balance)
        .bind(order.is_presale)
        .bind(order.presale_type)
        .bind(order.discount_amount)
        .bind(order.tax_rate)
        .bind(order.tax_rate_amount)
        // 该库 `parent_order_id` / `parent_order_sn` 均为非空列，主订单没有父单时写默认值。
        .bind(order.parent_order_id.unwrap_or(0))
        .bind(order.parent_order_sn.as_deref().unwrap_or(""))
        .bind(order.is_store_splited)
        .bind(order.pos_user_id)
        // PG 对齐后这些文本列 NOT NULL 且无默认值，必须显式写空串。
        .bind(&order.admin_note)
        .bind(&order.logistics_name)
        .bind(&order.shipping_type_name)
        .bind(&order.tracking_no)
        .bind("")
        .bind("")
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        let order_id = result;

        for item in items {
            sqlx::query(
                "INSERT INTO order_item
                 (order_id, order_sn, user_id, price, quantity, product_id,
                  product_name, product_sn, pic_thumb, sku_id, sku_data,
                  product_type, shop_id, origin_price, is_pin, promotion_data, card_group_name)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)",
            )
            .bind(order_id)
            .bind(&item.order_sn)
            .bind(item.user_id)
            .bind(item.price)
            .bind(item.quantity)
            .bind(item.product_id)
            .bind(&item.product_name)
            .bind(&item.product_sn)
            .bind(&item.pic_thumb)
            .bind(item.sku_id)
            .bind(&item.sku_data)
            .bind(item.product_type)
            .bind(item.shop_id)
            .bind(item.origin_price)
            .bind(item.is_pin)
            .bind(&item.promotion_data)
            .bind(&item.card_group_name)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?;
        }

        Ok(order_id)
    }

    /// 更新订单发票快照数据（随单申请发票后回写）
    pub async fn update_invoice_data_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        order_id: i32,
        invoice_data: &str,
    ) -> Result<()> {
        sqlx::query("UPDATE \"order\" SET invoice_data = $1 WHERE order_id = $2")
            .bind(invoice_data)
            .bind(order_id)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 更新订单发票快照数据（审核或开票后回写）
    pub async fn update_invoice_data(&self, order_id: i32, invoice_data: &str) -> Result<()> {
        sqlx::query("UPDATE \"order\" SET invoice_data = $1 WHERE order_id = $2")
            .bind(invoice_data)
            .bind(order_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 待付款且下单时间早于 cutoff 的订单 ID（定时取消用）。
    pub async fn find_unpaid_before(
        &self,
        cutoff: i32,
        limit: u64,
    ) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT order_id FROM \"order\"
             WHERE is_del = 0 AND pay_status = 1 AND order_status = 0
               AND add_time > 0 AND add_time <= $1
             ORDER BY order_id ASC
             LIMIT $2",
        )
        .bind(cutoff)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 已发货且发货时间早于 cutoff 的订单 ID（自动收货用）。
    pub async fn find_shipped_before(
        &self,
        cutoff: i32,
        limit: u64,
    ) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT order_id FROM \"order\"
             WHERE is_del = 0 AND order_status = 2
               AND shipping_time > 0 AND shipping_time <= $1
             ORDER BY order_id ASC
             LIMIT $2",
        )
        .bind(cutoff)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 自动确认收货：已发货 → 已完成（条件更新，幂等）。
    pub async fn mark_received(&self, order_id: i32, received_time: i32) -> Result<bool> {
        let affected = sqlx::query(
            "UPDATE \"order\" SET order_status = 5, received_time = $1
             WHERE order_id = $2 AND order_status = 2",
        )
        .bind(received_time)
        .bind(order_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected > 0)
    }

    /// 用户订单分页（状态过滤 + 评价状态过滤 + 待晒单过滤 + 关键词）。
    ///
    /// 语义对齐 Java `OrderServiceImpl#list(OrderListPageQuery)`：
    /// - `order_status`：`-1`/`None` 不过滤（调用方已归一），否则等值；
    /// - `comment_status`：`None` 不过滤；`0` = 待评价（`comment_status = 0`
    ///   **且** `order_status = 5 已完成`，见 Java `buildQueryWrapper`）；
    ///   `> 0` = 等值（已评价）；
    /// - `is_showed`：`None`/`-1` 不过滤；`>= 0` 时按 Java「待晒单」逻辑先取
    ///   `comment(user_id, is_showed)` 的 `order_id` 集合再过滤（子查询等价于
    ///   Java 先 `selectList` 再 `in(orderIds)`；无命中时子查询自然返回空集）。
    /// - `keyword`：订单号 / 收货人模糊匹配。
    pub async fn paginate(
        &self,
        user_id: Option<i32>,
        order_status: Option<i32>,
        comment_status: Option<i32>,
        is_showed: Option<i32>,
        keyword: Option<&str>,
        page: u64,
        page_size: u64,
    ) -> Result<(u64, Vec<Order>)> {
        let offset = page.saturating_sub(1) * page_size;
        let kw = keyword.map(|s| format!("%{s}%"));
        // 待晒单：Java 先查 comment 表（user_id + is_showed）得到 orderId 集合
        let show_sql = if is_showed.is_some() && user_id.is_some() {
            " AND order_id IN (SELECT order_id FROM \"comment\" WHERE user_id = $1 AND is_showed = $2)"
        } else {
            ""
        };
        let count_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT COUNT(*) FROM \"order\"
             WHERE is_del = 0
               AND ($1 IS NULL OR user_id = $2)
               AND ($3 IS NULL OR order_status = $4)
               AND (
                     $5 IS NULL
                     OR ($6 = 0 AND comment_status = 0 AND order_status = 5)
                     OR ($7 > 0 AND comment_status = $8)
                   )
               AND ($9 IS NULL OR order_sn ILIKE $10 OR consignee ILIKE $11){show_sql}"
        ));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql)
            .bind(user_id).bind(user_id)
            .bind(order_status).bind(order_status)
            .bind(comment_status).bind(comment_status).bind(comment_status).bind(comment_status)
            .bind(&kw).bind(&kw).bind(&kw);
        if !show_sql.is_empty() {
            cq = cq.bind(user_id).bind(is_showed);
        }
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT \"order_id\", \"order_sn\", \"user_id\", \"parent_order_id\", \"parent_order_sn\", \"order_status\", \"shipping_status\", \"pay_status\", \"add_time\", \"consignee\", \"address\", \"region_ids\", \"region_names\", \"address_data\", \"mobile\", \"email\", \"buyer_note\", \"admin_note\", \"shipping_method\", \"logistics_id\", \"logistics_provider\", \"logistics_name\", \"shipping_type_id\", \"shipping_type_name\", \"tracking_no\", \"shipping_time\", \"received_time\", \"pay_type_id\", \"pay_time\", \"use_points\", \"is_need_commisson\", \"distribution_status\", \"referrer_user_id\", \"is_del\", \"shop_id\", \"is_store_splited\", \"is_valid\", \"comment_status\", \"total_amount\", \"paid_amount\", \"unpaid_amount\", \"unrefund_amount\", \"product_amount\", \"coupon_amount\", \"points_amount\", \"discount_amount\", \"balance\", \"credit_amount\", \"is_credit_order\", \"online_paid_amount\", \"offline_paid_amount\", \"service_fee\", \"shipping_fee\", \"invoice_fee\", \"order_extension\", \"order_source\", \"invoice_data\", \"out_trade_no\", \"is_settlement\", \"is_exchange_order\", \"order_type\", \"is_cycle_order\", \"is_card_order\", \"is_presale\", \"presale_type\", \"mark\", \"vendor_id\", \"cancel_remark\", \"cash_amount\", \"cash_change_amount\", \"round_off_dime_amount\", \"round_off_cents_amount\", \"pos_user_id\", \"device_code\", \"discount_change_price\", \"tax_rate\", \"tax_rate_amount\", \"change_price_data\"::text AS \"change_price_data\", \"gift_card_amount\", \"delivery_date\"::text AS \"delivery_date\", \"delivery_time_slot\", \"period_index\", \"time_index\", \"table_id\", \"table_no\", \"table_area_name\", \"dining_count\" FROM \"order\"
             WHERE is_del = 0
               AND ($1 IS NULL OR user_id = $2)
               AND ($3 IS NULL OR order_status = $4)
               AND (
                     $5 IS NULL
                     OR ($6 = 0 AND comment_status = 0 AND order_status = 5)
                     OR ($7 > 0 AND comment_status = $8)
                   )
               AND ($9 IS NULL OR order_sn ILIKE $10 OR consignee ILIKE $11){show_sql}
             ORDER BY order_id DESC
             LIMIT $12 OFFSET $13"
        ));
        let mut rq = sqlx::query_as::<_, Order>(&rows_sql)
            .bind(user_id).bind(user_id)
            .bind(order_status).bind(order_status)
            .bind(comment_status).bind(comment_status).bind(comment_status).bind(comment_status)
            .bind(&kw).bind(&kw).bind(&kw);
        if !show_sql.is_empty() {
            rq = rq.bind(user_id).bind(is_showed);
        }
        let rows = rq
            .bind(page_size as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0 as u64, rows))
    }

    /// 查询订单明细。
    pub async fn find_items(&self, order_id: i32) -> Result<Vec<OrderItem>> {
        sqlx::query_as::<_, OrderItem>(
            "SELECT \"item_id\", \"order_id\", \"order_sn\", \"user_id\", \"price\", \"quantity\", \"product_id\", \"product_name\", \"product_sn\", \"pic_thumb\", \"sku_id\", \"sku_data\", \"delivery_quantity\", \"product_type\", \"is_gift\", \"shop_id\", \"is_pin\", \"prepay_price\", \"commission\", \"origin_price\", \"promotion_data\", \"is_seckill\", \"extra_sku_data\", \"suppliers_id\", \"card_group_name\", \"vendor_product_id\", \"vendor_product_sku_id\", \"vendor_id\", \"vendor_product_supply_price\", \"change_price_data\"::text AS \"change_price_data\", \"admin_change_pre_price\", \"gift_card_id\", \"delivery_service_fee\", \"server_service_fee\", \"is_wholesale_line\", \"wholesale_grade_id\", \"wholesale_base_price\" FROM order_item WHERE order_id = $1 ORDER BY item_id ASC",
        )
        .bind(order_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 状态流转（原子条件更新，防并发重复流转）：
    /// 仅当当前状态在 `from_status` 集合内时更新为 `to_status`。
    pub async fn transition_status(
        &self,
        order_id: i32,
        from_statuses: &[i32],
        to_status: i32,
    ) -> Result<bool> {
        if from_statuses.is_empty() {
            return Err(Error::Validation("缺少源状态".into()));
        }
        let placeholders = vec!["?"; from_statuses.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "UPDATE \"order\" SET order_status = $1 WHERE order_id = $2 AND order_status IN ({placeholders})"
        ));
        let mut query = sqlx::query(&sql).bind(to_status).bind(order_id);
        for s in from_statuses {
            query = query.bind(s);
        }
        let affected = query
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected > 0)
    }

    /// 标记订单已支付（`pay_status=2`、`order_status=1` 待发货），返回是否更新成功。
    ///
    /// 状态值口径见 `java/database/pure.sql` 的 `order.order_status` 列注释
    /// （`1` = 已确定/待发货）。**不要改回 10**：库中不存在该状态。
    pub async fn mark_paid(&self, order_id: i32, pay_time: i32, paid_amount: rust_decimal::Decimal) -> Result<bool> {
        let affected = sqlx::query(
            "UPDATE \"order\" SET pay_status = 2, paid_amount = $1, pay_time = $2,
             unpaid_amount = 0, order_status = 1
             WHERE order_id = $3 AND pay_status <> 2",
        )
        .bind(paid_amount)
        .bind(pay_time)
        .bind(order_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected > 0)
    }

    /// 记录订单使用的余额金额（`order.balance`）。
    pub async fn set_balance_amount(
        &self,
        order_id: i32,
        amount: rust_decimal::Decimal,
    ) -> Result<()> {
        sqlx::query("UPDATE \"order\" SET balance = $1 WHERE order_id = $2")
            .bind(amount)
            .bind(order_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 应用部分支付（余额抵扣等站内渠道）：
    /// 累加已付金额、扣减未付金额，不改变支付状态。
    ///
    /// 条件更新（`pay_status <> 2`）防止与全款支付回调并发冲突。
    /// 返回是否更新成功。
    pub async fn apply_partial_payment(
        &self,
        order_id: i32,
        amount: rust_decimal::Decimal,
    ) -> Result<bool> {
        let affected = sqlx::query(
            "UPDATE \"order\" SET
               paid_amount = paid_amount + $1,
               unpaid_amount = GREATEST(unpaid_amount - $2, 0)
             WHERE order_id = $3 AND pay_status <> 2",
        )
        .bind(amount)
        .bind(amount)
        .bind(order_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected > 0)
    }

    /// 软删除订单。
    pub async fn soft_delete(&self, order_id: i32) -> Result<()> {
        sqlx::query("UPDATE \"order\" SET is_del = 1 WHERE order_id = $1")
            .bind(order_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 统计用户已支付订单数（新人专享券判定，`pay_status > 0`）。
    ///
    /// 对应 Java `UserCouponServiceImpl#claim` 中的
    /// `orderMapper.selectCount(... gt(Order::getPayStatus, 0))`。
    pub async fn count_paid_by_user(&self, user_id: i32) -> Result<i64> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM \"order\" WHERE user_id = $1 AND pay_status > 0",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(n)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("订单不存在".into()),
        other => Error::Database(other.to_string()),
    }
}
