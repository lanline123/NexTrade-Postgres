//! 保证金退还域仓储（merchant::deposit_refund）。
//!
//! 涉及表（列名严格对齐 `schema_parser`）：
//! - `shop_deposit_refund`（退还单，本域主表）
//! - `shop_deposit`（账户：可退额度读取 + 审核出账行锁更新）
//! - `shop_deposit_log`（保证金流水）
//! - 全额退还 5 条件：`shop` / `vendor`（仅 `source_type = 1`）/
//!   `order` / `aftersales` / `config`
//! - 展示富化：`shop` / `vendor` / `merchant`
//!
//! 查询统一使用运行时 `sqlx::query_as` / `QueryBuilder`（**不使用** `query!` 宏）。
//!
//! # 事务
//!
//! Java `DepositRefundServiceImpl#audit` 是 `@Transactional`（一步式：账户出账
//! + 流水 + 退还单完成）。本仓储提供 [`DepositRefundRepository::begin`] +
//! `*_in_tx` 方法，由服务层把三次写入收进同一事务。
//!
//! # 组织名称/主体类型筛选
//!
//! 与 `deposit_recharge` 一致：用等价子查询代替 Java 的「先解析 id 集合再 IN」，
//! 同一类型内多约束 = `AND`，多类型之间 = `OR`。

use rust_decimal::Decimal;
use sqlx::{Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::{
    Merchant, Shop, ShopDeposit, ShopDepositLog, ShopDepositRefund, Vendor,
};
use nextrade_core::models::merchant::deposit_refund::{
    refund_sort_column, sort_is_asc, DepositRefundListQuery, ONGOING_ORDER_STATUS,
};
use nextrade_core::{Error, Result};

/// 保证金退还域仓储（持有连接池）。
#[derive(Clone)]
pub struct DepositRefundRepository {
    pool: PgPool,
}

impl DepositRefundRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（与其它仓储一致）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 开启事务（服务层编排 `audit` 的账户/流水/单据三次写入）。
    pub async fn begin(&self) -> Result<sqlx::Transaction<'static, Postgres>> {
        self.pool.begin().await.map_err(map_err)
    }

    // ————————————————————— 账户 / 配置 / 持有者 —————————————————————

    /// 只读按持有者查保证金账户（Java `findAccount`）。
    pub async fn find_account(
        &self,
        owner_type: i32,
        owner_id: i32,
    ) -> Result<Option<ShopDeposit>> {
        sqlx::query_as::<_, ShopDeposit>(
            "SELECT * FROM shop_deposit WHERE owner_type = $1 AND owner_id = $2 LIMIT 1",
        )
        .bind(owner_type)
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 待审核退还总额（Java `pendingRefundSum`：`SUM(amount) WHERE status = 0`）。
    pub async fn sum_pending_refund(&self, owner_type: i32, owner_id: i32) -> Result<Decimal> {
        let row: (Option<Decimal>,) = sqlx::query_as(
            "SELECT SUM(amount) FROM shop_deposit_refund \
             WHERE owner_type = $1 AND owner_id = $2 AND status = 0",
        )
        .bind(owner_type)
        .bind(owner_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.unwrap_or(Decimal::ZERO))
    }

    /// 待审核退还单数（Java `count(status = 0)`）。
    pub async fn count_pending_refund(&self, owner_type: i32, owner_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM shop_deposit_refund \
             WHERE owner_type = $1 AND owner_id = $2 AND status = 0",
        )
        .bind(owner_type)
        .bind(owner_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 进行中订单数（Java `ONGOING_ORDER_STATUS = [0,1,2]` + `is_del = 0`）。
    pub async fn count_ongoing_orders(&self, owner_type: i32, owner_id: i32) -> Result<i64> {
        let sql = if owner_type == 2 {
            "SELECT COUNT(*) FROM \"order\" WHERE vendor_id = $1 AND is_del = 0 \
             AND order_status IN (0, 1, 2)"
        } else {
            "SELECT COUNT(*) FROM \"order\" WHERE shop_id = $1 AND is_del = 0 \
             AND order_status IN (0, 1, 2)"
        };
        let row: (i64,) = sqlx::query_as(sql)
            .bind(owner_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0)
    }

    /// 未完结售后数（Java：`status NOT IN (6 已完成, 7 已取消)`）。
    pub async fn count_ongoing_aftersales(&self, owner_type: i32, owner_id: i32) -> Result<i64> {
        let sql = if owner_type == 2 {
            "SELECT COUNT(*) FROM aftersales WHERE vendor_id = $1 AND status NOT IN (6, 7)"
        } else {
            "SELECT COUNT(*) FROM aftersales WHERE shop_id = $1 AND status NOT IN (6, 7)"
        };
        let row: (i64,) = sqlx::query_as(sql)
            .bind(owner_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0)
    }

    /// 末单确认收货时间（Java `orderByDesc(receivedTime).last("limit 1")`）。
    pub async fn last_received_time(
        &self,
        owner_type: i32,
        owner_id: i32,
    ) -> Result<Option<i32>> {
        let sql = if owner_type == 2 {
            "SELECT received_time FROM \"order\" WHERE vendor_id = $1 AND is_del = 0 \
             AND received_time > 0 ORDER BY received_time DESC LIMIT 1"
        } else {
            "SELECT received_time FROM \"order\" WHERE shop_id = $1 AND is_del = 0 \
             AND received_time > 0 ORDER BY received_time DESC LIMIT 1"
        };
        let row: Option<(i32,)> = sqlx::query_as(sql)
            .bind(owner_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.map(|(t,)| t))
    }

    /// 读取配置项原始值（Java `getConfigVal` / `getConfigByCode` 的公共部分）：
    /// 行缺失或 `biz_val` 为 NULL 均返回 `None`；调用方按各自默认值兜底。
    pub async fn config_value(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1",
        )
        .bind(biz_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|(value,)| value))
    }

    /// 店铺（全额退还条件 1：`shop.status`）。
    pub async fn find_shop(&self, shop_id: i32) -> Result<Option<Shop>> {
        sqlx::query_as::<_, Shop>("SELECT * FROM shop WHERE shop_id = $1 LIMIT 1")
            .bind(shop_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 入驻供应商（Java `vendorMapper.selectOnboardedById`）。
    pub async fn find_vendor(&self, vendor_id: i32) -> Result<Option<Vendor>> {
        sqlx::query_as::<_, Vendor>(
            "SELECT * FROM vendor WHERE vendor_id = $1 AND source_type = 1 LIMIT 1",
        )
        .bind(vendor_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 商户（展示富化：`merchant.type`）。
    pub async fn find_merchant(&self, merchant_id: i32) -> Result<Option<Merchant>> {
        sqlx::query_as::<_, Merchant>("SELECT * FROM merchant WHERE merchant_id = $1 LIMIT 1")
            .bind(merchant_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    // ————————————————————— 申请 / 详情 —————————————————————

    /// 插入退还申请（Java `this.save(refund)`），返回自增 id。
    pub async fn insert_refund(&self, refund: &ShopDepositRefund) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO shop_deposit_refund
               (owner_type, owner_id, type, amount, payee_info, reason, status,
                pay_voucher, audit_admin_id, audit_time, audit_remark, add_time,
                update_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) RETURNING shop_deposit_refund_id",
        )
        .bind(refund.owner_type)
        .bind(refund.owner_id)
        .bind(refund.r#type)
        .bind(refund.amount)
        .bind(&refund.payee_info)
        .bind(&refund.reason)
        .bind(refund.status)
        .bind(&refund.pay_voucher)
        .bind(refund.audit_admin_id)
        .bind(refund.audit_time)
        .bind(&refund.audit_remark)
        .bind(refund.add_time)
        .bind(refund.update_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 平台端退还单分页（Java `DepositRefundServiceImpl#list`）。
    pub async fn paginate_refunds(
        &self,
        q: &DepositRefundListQuery,
    ) -> Result<(u64, Vec<ShopDepositRefund>)> {
        let mut count_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM shop_deposit_refund WHERE 1 = 1");
        Self::push_refund_filters(&mut count_qb, q);
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let (page, size) =
            nextrade_core::models::merchant::deposit_refund::page_params(q.page, q.size);
        let mut row_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT * FROM shop_deposit_refund WHERE 1 = 1");
        Self::push_refund_filters(&mut row_qb, q);
        row_qb
            .push(" ORDER BY ")
            .push(refund_sort_column(q.sort_field.as_deref()));
        row_qb.push(if sort_is_asc(q.sort_order.as_deref()) {
            " ASC"
        } else {
            " DESC"
        });
        row_qb.push(" LIMIT ").push_bind(size as i64);
        row_qb.push(" OFFSET ").push_bind(((page - 1) * size) as i64);
        let rows = row_qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 退还单列表 WHERE 片段（COUNT 与行查询共用）。
    fn push_refund_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &DepositRefundListQuery) {
        let name_like = q
            .owner_name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(|n| format!("%{n}%"));
        match q.owner_type {
            Some(t) => Self::push_owner_segment(qb, t, name_like.as_deref(), q.subject_type),
            None => {
                qb.push(" AND (");
                Self::push_owner_segment_inner(qb, 1, name_like.as_deref(), q.subject_type);
                qb.push(" OR ");
                Self::push_owner_segment_inner(qb, 2, name_like.as_deref(), q.subject_type);
                qb.push(")");
            }
        }
        if let Some(owner_id) = q.effective_owner_id() {
            qb.push(" AND owner_id = ").push_bind(owner_id);
        }
        if let Some(refund_type) = q.r#type {
            qb.push(" AND type = ").push_bind(refund_type);
        }
        if let Some(status) = q.status {
            qb.push(" AND status = ").push_bind(status);
        }
        if let Some(start_time) = q.start_time {
            qb.push(" AND add_time >= ").push_bind(start_time);
        }
        if let Some(end_time) = q.end_time {
            qb.push(" AND add_time <= ").push_bind(end_time);
        }
    }

    /// 单 owner 类型分段条件（外层已保证只有这一段）。
    fn push_owner_segment(
        qb: &mut QueryBuilder<'_, Postgres>,
        owner_type: i32,
        name_like: Option<&str>,
        subject_type: Option<i32>,
    ) {
        qb.push(" AND ");
        Self::push_owner_segment_inner(qb, owner_type, name_like, subject_type);
    }

    /// 单 owner 类型分段条件本体：`(owner_type = ? [AND owner_id IN (...)])`。
    fn push_owner_segment_inner(
        qb: &mut QueryBuilder<'_, Postgres>,
        owner_type: i32,
        name_like: Option<&str>,
        subject_type: Option<i32>,
    ) {
        qb.push("(owner_type = ").push_bind(owner_type);
        if let Some(like) = name_like {
            if owner_type == 1 {
                qb.push(" AND owner_id IN (SELECT shop_id FROM shop WHERE shop_title LIKE ")
                    .push_bind(like.to_string())
                    .push(")");
            } else {
                qb.push(
                    " AND owner_id IN (SELECT vendor_id FROM vendor WHERE source_type = 1 AND vendor_name LIKE ",
                )
                .push_bind(like.to_string())
                .push(")");
            }
        }
        if let Some(subject_type) = subject_type {
            if owner_type == 1 {
                qb.push(
                    " AND owner_id IN (SELECT shop_id FROM shop WHERE merchant_id IN \
                     (SELECT merchant_id FROM merchant WHERE \"type\" = ",
                )
                .push_bind(subject_type)
                .push("))");
            } else {
                qb.push(
                    " AND owner_id IN (SELECT vendor_id FROM vendor WHERE source_type = 1 AND \"type\" = ",
                )
                .push_bind(subject_type)
                .push(")");
            }
        }
        qb.push(")");
    }

    /// 按 id 查退还单（Java `this.getById`）。
    pub async fn find_refund(&self, id: i32) -> Result<Option<ShopDepositRefund>> {
        sqlx::query_as::<_, ShopDepositRefund>(
            "SELECT * FROM shop_deposit_refund WHERE shop_deposit_refund_id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— 审核（事务内） —————————————————————

    /// 事务内按 id 查退还单。
    pub async fn find_refund_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        id: i32,
    ) -> Result<Option<ShopDepositRefund>> {
        sqlx::query_as::<_, ShopDepositRefund>(
            "SELECT * FROM shop_deposit_refund WHERE shop_deposit_refund_id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_err)
    }

    /// 事务内取保证金账户（不存在则创建），并对行加锁
    /// （Java `getOrCreate`：`selectForUpdateByOwner`）。
    pub async fn lock_or_create_account_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        owner_type: i32,
        owner_id: i32,
        now: i32,
    ) -> Result<ShopDeposit> {
        if let Some(account) = Self::select_account_for_update(tx, owner_type, owner_id).await? {
            return Ok(account);
        }
        sqlx::query(
            "INSERT INTO shop_deposit
               (owner_type, owner_id, paid, required_manual, required_snapshot,
                collect_due_time, supplement_due_time, collect_activated,
                penalty_status, status, add_time, update_time)
             VALUES ($1, $2, 0, NULL, 0, 0, 0, 0, 0, 1, $3, $4)",
        )
        .bind(owner_type)
        .bind(owner_id)
        .bind(now)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Self::select_account_for_update(tx, owner_type, owner_id)
            .await?
            .ok_or_else(|| Error::Database("保证金账户创建失败".into()))
    }

    /// `SELECT ... FOR UPDATE` 取账户。
    async fn select_account_for_update(
        tx: &mut sqlx::Transaction<'_, Postgres>,
        owner_type: i32,
        owner_id: i32,
    ) -> Result<Option<ShopDeposit>> {
        sqlx::query_as::<_, ShopDeposit>(
            "SELECT * FROM shop_deposit WHERE owner_type = $1 AND owner_id = $2 FOR UPDATE",
        )
        .bind(owner_type)
        .bind(owner_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_err)
    }

    /// 事务内更新账户实缴余额（Java `shopDepositMapper.updateById(account)`）。
    pub async fn update_account_paid_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        shop_deposit_id: i32,
        paid: Decimal,
        now: i32,
    ) -> Result<bool> {
        let affected = sqlx::query(
            "UPDATE shop_deposit SET paid = $1, update_time = $2 WHERE shop_deposit_id = $3",
        )
        .bind(paid)
        .bind(now)
        .bind(shop_deposit_id)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected > 0)
    }

    /// 事务内插入保证金流水（Java `shopDepositLogMapper.insert`）。
    pub async fn insert_log_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        log: &ShopDepositLog,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO shop_deposit_log
               (owner_type, owner_id, type, change_amount, before_paid, after_paid,
                related_id, order_id, admin_id, remark, add_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING shop_deposit_log_id",
        )
        .bind(log.owner_type)
        .bind(log.owner_id)
        .bind(log.r#type)
        .bind(log.change_amount)
        .bind(log.before_paid)
        .bind(log.after_paid)
        .bind(log.related_id)
        .bind(log.order_id)
        .bind(log.admin_id)
        .bind(&log.remark)
        .bind(log.add_time)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 事务内更新退还单（Java `this.updateById(refund)`：仅写审核相关列）。
    pub async fn update_refund_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        refund: &ShopDepositRefund,
    ) -> Result<bool> {
        let affected = sqlx::query(
            "UPDATE shop_deposit_refund SET status = $1, pay_voucher = $2,
               audit_admin_id = $3, audit_time = $4, audit_remark = $5, update_time = $6
             WHERE shop_deposit_refund_id = $7",
        )
        .bind(refund.status)
        .bind(&refund.pay_voucher)
        .bind(refund.audit_admin_id)
        .bind(refund.audit_time)
        .bind(&refund.audit_remark)
        .bind(refund.update_time)
        .bind(refund.shop_deposit_refund_id)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected > 0)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("保证金单据不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 进行中订单状态口径与 Java `ONGOING_ORDER_STATUS` 一致。
    #[test]
    fn ongoing_order_status_matches_java() {
        assert_eq!(ONGOING_ORDER_STATUS, [0, 1, 2]);
    }

    /// ownerId 兼容口径（ownerId 优先，回退 shopId，非正数忽略）。
    #[test]
    fn query_effective_owner_id_prefers_owner_id() {
        let q = DepositRefundListQuery {
            owner_id: Some(5),
            shop_id: Some(9),
            ..Default::default()
        };
        assert_eq!(q.effective_owner_id(), Some(5));
        let q = DepositRefundListQuery {
            owner_id: None,
            shop_id: Some(9),
            ..Default::default()
        };
        assert_eq!(q.effective_owner_id(), Some(9));
        let q = DepositRefundListQuery {
            owner_id: Some(0),
            shop_id: Some(-1),
            ..Default::default()
        };
        assert_eq!(q.effective_owner_id(), None);
    }
}
