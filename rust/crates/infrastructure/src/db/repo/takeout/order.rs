//! 外部订单仓储（takeout::order，我方新增）：`takeout_external_order` /
//! `takeout_callback_log`，以及外部单在我方 `order` / `order_item` 表的
//! 落库（列集与下单路径 `trade::order` 的 48 列子集逐字一致，仅追加
//! 迁移 030 的 `external_channel` / `external_order_sn` 两列）。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

use nextrade_core::entities::order::{Order, OrderItem};
use nextrade_core::Result;

use super::map_err;

/// `takeout_external_order` 行（与迁移 030 逐字对齐）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ExternalOrderRow {
    /// 主键。
    pub id: i32,
    /// 渠道 id。
    pub channel: String,
    /// 渠道订单号。
    pub external_order_id: String,
    /// 我方门店 id（0 = 未绑定兜底）。
    pub shop_id: i32,
    /// 绑定 id。
    pub binding_id: i32,
    /// 内部订单 id（0 = 未生成）。
    pub internal_order_id: i32,
    /// 渠道原始状态码。
    pub external_status: i32,
    /// 归一状态。
    pub status: String,
    /// 总额（元，渠道口径）。
    pub total_amount: Decimal,
    /// 商品总额（元）。
    pub product_amount: Decimal,
    /// 配送费（元）。
    pub shipping_fee: Decimal,
    /// 收货人。
    pub consignee: String,
    /// 手机号（渠道隐私号原样）。
    pub mobile: String,
    /// 地址全文。
    pub address: String,
    /// 最新一次推送的归一 JSON。
    pub raw_payload: String,
    /// 最近事件类型。
    pub last_event: String,
    /// 最近错误（兜底信息）。
    pub last_error: String,
    /// 创建时间（秒）。
    pub add_time: i32,
    /// 更新时间（秒）。
    pub update_time: i32,
}

/// 外部订单仓储。
#[derive(Clone)]
pub struct ExternalOrderRepository {
    pool: PgPool,
}

impl ExternalOrderRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 按（渠道，渠道订单号）查外部单（幂等判定用）。
    ///
    /// # Errors
    /// DB 错误直传。
    pub async fn find(
        &self,
        channel: &str,
        external_order_id: &str,
    ) -> Result<Option<ExternalOrderRow>> {
        sqlx::query_as::<_, ExternalOrderRow>(
            "SELECT id, channel, external_order_id, shop_id, binding_id, internal_order_id, \
             external_status, status, total_amount, product_amount, shipping_fee, \
             consignee, mobile, address, raw_payload, last_event, last_error, add_time, update_time \
             FROM takeout_external_order WHERE channel = $1 AND external_order_id = $2 LIMIT 1",
        )
        .bind(channel)
        .bind(external_order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按主键查外部单。
    ///
    /// # Errors
    /// DB 错误直传。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<ExternalOrderRow>> {
        sqlx::query_as::<_, ExternalOrderRow>(
            "SELECT id, channel, external_order_id, shop_id, binding_id, internal_order_id, \
             external_status, status, total_amount, product_amount, shipping_fee, \
             consignee, mobile, address, raw_payload, last_event, last_error, add_time, update_time \
             FROM takeout_external_order WHERE id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 插入外部单（唯一键冲突由调用方先查后插，本方法不吞冲突）。
    ///
    /// # Errors
    /// DB 错误直传（含唯一键冲突原文，服务层据此走幂等更新路径）。
    pub async fn insert(&self, row: &ExternalOrderRow) -> Result<i32> {
        sqlx::query_scalar::<_, i32>(
            "INSERT INTO takeout_external_order \
               (channel, external_order_id, shop_id, binding_id, internal_order_id, \
                external_status, status, total_amount, product_amount, shipping_fee, \
                consignee, mobile, address, raw_payload, last_event, last_error, \
                add_time, update_time) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18) \
             RETURNING id",
        )
        .bind(&row.channel)
        .bind(&row.external_order_id)
        .bind(row.shop_id)
        .bind(row.binding_id)
        .bind(row.internal_order_id)
        .bind(row.external_status)
        .bind(&row.status)
        .bind(row.total_amount)
        .bind(row.product_amount)
        .bind(row.shipping_fee)
        .bind(&row.consignee)
        .bind(&row.mobile)
        .bind(&row.address)
        .bind(&row.raw_payload)
        .bind(&row.last_event)
        .bind(&row.last_error)
        .bind(row.add_time)
        .bind(row.update_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 更新外部单的可变字段（状态 / 内部单关联 / 最近事件与错误 / 载荷）。
    ///
    /// # Errors
    /// DB 错误直传。
    pub async fn update(&self, row: &ExternalOrderRow) -> Result<()> {
        sqlx::query(
            "UPDATE takeout_external_order SET shop_id = $3, binding_id = $4, \
             internal_order_id = $5, external_status = $6, status = $7, \
             total_amount = $8, product_amount = $9, shipping_fee = $10, \
             consignee = $11, mobile = $12, address = $13, raw_payload = $14, \
             last_event = $15, last_error = $16, update_time = $17 \
             WHERE channel = $1 AND external_order_id = $2",
        )
        .bind(&row.channel)
        .bind(&row.external_order_id)
        .bind(row.shop_id)
        .bind(row.binding_id)
        .bind(row.internal_order_id)
        .bind(row.external_status)
        .bind(&row.status)
        .bind(row.total_amount)
        .bind(row.product_amount)
        .bind(row.shipping_fee)
        .bind(&row.consignee)
        .bind(&row.mobile)
        .bind(&row.address)
        .bind(&row.raw_payload)
        .bind(&row.last_event)
        .bind(&row.last_error)
        .bind(row.update_time)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 列表（门店过滤 + 渠道/状态/关键词过滤，分页）。返回 (rows, total)。
    ///
    /// # Errors
    /// DB 错误直传。
    pub async fn list(
        &self,
        shop_filter: Option<i32>,
        channel: Option<&str>,
        status: Option<&str>,
        keyword: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<ExternalOrderRow>, i64)> {
        let mut rows = sqlx::query_as::<_, ExternalOrderRow>(
            "SELECT id, channel, external_order_id, shop_id, binding_id, internal_order_id, \
             external_status, status, total_amount, product_amount, shipping_fee, \
             consignee, mobile, address, raw_payload, last_event, last_error, add_time, update_time \
             FROM takeout_external_order ORDER BY id DESC LIMIT 1000",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        if let Some(shop_id) = shop_filter {
            rows.retain(|r| r.shop_id == shop_id);
        }
        if let Some(ch) = channel {
            if !ch.is_empty() {
                rows.retain(|r| r.channel == ch);
            }
        }
        if let Some(st) = status {
            if !st.is_empty() {
                rows.retain(|r| r.status == st);
            }
        }
        if let Some(kw) = keyword {
            if !kw.is_empty() {
                rows.retain(|r| r.external_order_id.contains(kw) || r.mobile.contains(kw));
            }
        }
        let total = rows.len() as i64;
        let offset = ((page.max(1) - 1) * page_size.max(1)) as usize;
        let page_rows: Vec<ExternalOrderRow> = rows
            .into_iter()
            .skip(offset)
            .take(page_size.max(1) as usize)
            .collect();
        Ok((page_rows, total))
    }

    /// 写一条回调日志（审计 + 幂等排障）。日志写失败不反噬主流程，
    /// 由服务层决定是否忽略错误。
    ///
    /// # Errors
    /// DB 错误直传。
    pub async fn insert_callback_log(
        &self,
        channel: &str,
        event_type: &str,
        external_order_id: &str,
        sign_verified: i32,
        process_status: i32,
        message: &str,
        raw_body: &str,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO takeout_callback_log \
               (channel, event_type, external_order_id, sign_verified, process_status, \
                message, raw_body, add_time) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(channel)
        .bind(event_type)
        .bind(external_order_id)
        .bind(sign_verified)
        .bind(process_status)
        .bind(message.chars().take(500).collect::<String>())
        .bind(raw_body)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 在我方订单表生成外部单的履约单（单事务：order + order_item）。
    ///
    /// 列集与下单路径 `trade::order` 的插入子集一致（PG 对齐后其余
    /// NOT NULL 文本列在该子集内已覆盖），追加 `external_channel` /
    /// `external_order_sn`。`order` / `items` 由服务层组装（含
    /// `order_type = 13`、渠道金额原样、资金链隔离字段）。
    ///
    /// # Errors
    /// DB 错误直传（事务回滚，不留半单）。
    pub async fn create_internal_order(
        &self,
        order: &Order,
        items: &[OrderItem],
    ) -> Result<i32> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        let order_id: i32 = sqlx::query_scalar(
            "INSERT INTO \"order\"
             (order_type, order_sn, user_id, order_status, shipping_status, pay_status,
              add_time, consignee, address, region_ids, region_names, mobile,
              buyer_note, shipping_method, shipping_type_id, pay_type_id, shop_id, is_valid, is_del,
              total_amount, paid_amount, unpaid_amount, product_amount, coupon_amount,
              shipping_fee, referrer_user_id, order_source, vendor_id, order_extension,
              email, use_points, points_amount, balance, is_presale, presale_type,
              discount_amount, tax_rate, tax_rate_amount, parent_order_id, parent_order_sn,
              is_store_splited, pos_user_id, admin_note, logistics_name, shipping_type_name,
              tracking_no, table_no, table_area_name, external_channel, external_order_sn)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18,
                     $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36, $37, $38,
                     $39, $40, $41, $42, $43, $44, $45, $46, $47, $48, $49, $50) RETURNING order_id",
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
        .bind(order.shipping_type_id)
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
        .bind(order.parent_order_id.unwrap_or(0))
        .bind(order.parent_order_sn.as_deref().unwrap_or(""))
        .bind(order.is_store_splited)
        .bind(order.pos_user_id)
        .bind(&order.admin_note)
        .bind(&order.logistics_name)
        .bind(&order.shipping_type_name)
        .bind(&order.tracking_no)
        .bind("")
        .bind("")
        .bind(&order.external_channel)
        .bind(&order.external_order_sn)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_err)?;

        for item in items {
            sqlx::query(
                "INSERT INTO order_item
                 (order_id, order_sn, user_id, price, quantity, product_id,
                  product_name, product_sn, pic_thumb, sku_id, sku_data,
                  product_type, shop_id, origin_price, is_pin, promotion_data, card_group_name)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)",
            )
            .bind(order_id)
            .bind(&order.order_sn)
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
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        }
        tx.commit().await.map_err(map_err)?;
        Ok(order_id)
    }

    /// 同步外部状态到内部单（只动状态列；取消时写取消原因）。
    /// 外部单的资金状态只做镜像，**不触发任何出款/退款动作**。
    ///
    /// # Errors
    /// DB 错误直传。
    pub async fn update_internal_order_status(
        &self,
        order_id: i32,
        order_status: i32,
        pay_status: i32,
        cancel_remark: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE \"order\" SET order_status = $2, pay_status = $3, \
             cancel_remark = COALESCE($4, cancel_remark) WHERE order_id = $1",
        )
        .bind(order_id)
        .bind(order_status)
        .bind(pay_status)
        .bind(cancel_remark)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }
}
