//! 保证金充值域仓储（merchant::deposit_recharge）。
//!
//! 涉及表（列名严格对齐 `schema_parser`）：
//! - `shop_deposit_recharge`（充值单，本域主表）
//! - `shop_deposit_refund`（合并审核列表 / 审核详情要读，**只读**）
//! - `shop_deposit`（账户，审核入账时加行锁更新）
//! - `shop_deposit_log`（保证金流水，审核入账写入）
//! - `shop_deposit_payee_account`（收款账户）
//! - 展示富化：`shop` / `vendor`（仅 `source_type = 1` 入驻供应商）/ `merchant`
//!
//! 查询统一使用运行时 `sqlx::query_as` / `QueryBuilder`（**不使用** `query!` 宏，
//! 本机无 `DATABASE_URL`）。
//!
//! # 事务
//!
//! Java `DepositRechargeServiceImpl#audit` 是 `@Transactional`：账户入账
//! （`getOrCreate` + `paid` 更新）+ 流水插入 + 充值单状态更新必须原子。
//! 本仓储提供 [`DepositRechargeRepository::begin`] + `*_in_tx` 方法，
//! 由服务层把三次写入收进同一事务（与 `catalog::price_inquiry` 同一做法）。
//!
//! # 组织名称/主体类型筛选
//!
//! Java 先按店铺/供应商名与商户性质解析出 ownerId 集合再拼 `IN`；本实现用
//! **等价子查询**（同一类型内多个约束 = `AND`，多类型之间 = `OR`），
//! 语义一致且避免 N 次额外查询：
//! - 店铺：`owner_id IN (SELECT shop_id FROM shop WHERE shop_title ILIKE ?)`
//! - 店铺主体：`owner_id IN (SELECT shop_id FROM shop WHERE merchant_id IN
//!   (SELECT merchant_id FROM merchant WHERE type = ?))`
//! - 供应商：`owner_id IN (SELECT vendor_id FROM vendor WHERE source_type = 1
//!   AND vendor_name ILIKE ? / type = ?)`

use rust_decimal::Decimal;
use sqlx::{Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::{
    Merchant, Shop, ShopDeposit, ShopDepositLog, ShopDepositPayeeAccount, ShopDepositRecharge,
    ShopDepositRefund, Vendor,
};
use nextrade_core::models::merchant::deposit_recharge::{
    recharge_sort_column, sort_is_asc, DepositRechargeListQuery,
};
use nextrade_core::{Error, Result};

/// 保证金充值域仓储（持有连接池）。
#[derive(Clone)]
pub struct DepositRechargeRepository {
    pool: PgPool,
}

impl DepositRechargeRepository {
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

    // ————————————————————— 收款账户 —————————————————————

    /// 启用的收款账户列表（Java `DepositPayeeAccountServiceImpl#listEnabled`）：
    /// `status = 1` + 可选账户类型 + 关键词（户名/账号模糊）+ 排序
    /// `sort_order ASC, shop_deposit_payee_account_id DESC`。
    pub async fn list_payee_accounts(
        &self,
        account_type: Option<i32>,
        keyword: Option<&str>,
    ) -> Result<Vec<ShopDepositPayeeAccount>> {
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT * FROM shop_deposit_payee_account WHERE status = 1",
        );
        if let Some(account_type) = account_type {
            qb.push(" AND account_type = ").push_bind(account_type);
        }
        if let Some(keyword) = keyword.map(str::trim).filter(|k| !k.is_empty()) {
            let like = format!("%{keyword}%");
            qb.push(" AND (account_name ILIKE ")
                .push_bind(like.clone())
                .push(" OR account_no ILIKE ")
                .push_bind(like)
                .push(")");
        }
        qb.push(" ORDER BY sort_order ASC, shop_deposit_payee_account_id DESC");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 按 id 查收款账户（Java `depositPayeeAccountService.detail`）。
    pub async fn find_payee_account(&self, id: i32) -> Result<Option<ShopDepositPayeeAccount>> {
        sqlx::query_as::<_, ShopDepositPayeeAccount>(
            "SELECT * FROM shop_deposit_payee_account WHERE shop_deposit_payee_account_id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— 充值申请 —————————————————————

    /// 插入充值申请（Java `this.save(recharge)`），返回自增 id。
    pub async fn insert_recharge(&self, recharge: &ShopDepositRecharge) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO shop_deposit_recharge
               (owner_type, owner_id, amount, pay_voucher, payee_account_id,
                payee_account_type, payee_snapshot, remark, status, audit_admin_id,
                audit_time, audit_remark, add_time, update_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) RETURNING shop_deposit_recharge_id",
        )
        .bind(recharge.owner_type)
        .bind(recharge.owner_id)
        .bind(recharge.amount)
        .bind(&recharge.pay_voucher)
        .bind(recharge.payee_account_id)
        .bind(recharge.payee_account_type)
        .bind(&recharge.payee_snapshot)
        .bind(&recharge.remark)
        .bind(recharge.status)
        .bind(recharge.audit_admin_id)
        .bind(recharge.audit_time)
        .bind(&recharge.audit_remark)
        .bind(recharge.add_time)
        .bind(recharge.update_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    // ————————————————————— 列表 / 详情 —————————————————————

    /// 平台端充值单分页（Java `DepositRechargeServiceImpl#list`）。
    ///
    /// 条件：owner 分段（类型 + 组织名/主体类型**子查询**约束）+ 精确 ownerId
    /// + 收款账户类型 + 状态 + 申请时间区间；排序按 `sortField/sortOrder`
    /// 白名单（缺省主键降序）。
    pub async fn paginate_recharges(
        &self,
        q: &DepositRechargeListQuery,
    ) -> Result<(u64, Vec<ShopDepositRecharge>)> {
        let mut count_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM shop_deposit_recharge WHERE 1 = 1");
        Self::push_recharge_filters(&mut count_qb, q);
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let (page, size) = nextrade_core::models::merchant::deposit_recharge::page_params(
            q.page, q.size,
        );
        let mut row_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT * FROM shop_deposit_recharge WHERE 1 = 1");
        Self::push_recharge_filters(&mut row_qb, q);
        row_qb
            .push(" ORDER BY ")
            .push(recharge_sort_column(q.sort_field.as_deref()));
        row_qb.push(if sort_is_asc(q.sort_order.as_deref()) {
            " ASC"
        } else {
            " DESC"
        });
        row_qb.push(" LIMIT ").push_bind(size as i64);
        row_qb
            .push(" OFFSET ")
            .push_bind(((page - 1) * size) as i64);
        let rows = row_qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 充值单列表的 WHERE 片段（COUNT 与行查询共用）。
    fn push_recharge_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &DepositRechargeListQuery) {
        let name_like = q
            .owner_name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(|n| format!("%{n}%"));
        // Java `resolveOwnerTypes`：null = 店铺 + 供应商
        let owner_types: [i32; 2] = match q.owner_type {
            Some(t) => [t, t],
            None => [1, 2],
        };
        match q.owner_type {
            Some(t) => Self::push_owner_segment(qb, t, name_like.as_deref(), q.subject_type),
            None => {
                qb.push(" AND (");
                Self::push_owner_segment_inner(
                    qb,
                    owner_types[0],
                    name_like.as_deref(),
                    q.subject_type,
                );
                qb.push(" OR ");
                Self::push_owner_segment_inner(
                    qb,
                    owner_types[1],
                    name_like.as_deref(),
                    q.subject_type,
                );
                qb.push(")");
            }
        }
        if let Some(owner_id) = q.effective_owner_id() {
            qb.push(" AND owner_id = ").push_bind(owner_id);
        }
        if let Some(payee_account_type) = q.payee_account_type {
            qb.push(" AND payee_account_type = ")
                .push_bind(payee_account_type);
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

    /// 单 owner 类型分段条件（外层已保证只有这一段，故无括号包裹需求）。
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
                qb.push(
                    " AND owner_id IN (SELECT shop_id FROM shop WHERE shop_title ILIKE ",
                )
                .push_bind(like.to_string())
                .push(")");
            } else {
                qb.push(
                    " AND owner_id IN (SELECT vendor_id FROM vendor WHERE source_type = 1 AND vendor_name ILIKE ",
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

    /// 按 id 查充值单（Java `this.getById`）。
    pub async fn find_recharge(&self, id: i32) -> Result<Option<ShopDepositRecharge>> {
        sqlx::query_as::<_, ShopDepositRecharge>(
            "SELECT * FROM shop_deposit_recharge WHERE shop_deposit_recharge_id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按 id 查退还单（Java `shopDepositRefundMapper.selectById`，合并审核用）。
    pub async fn find_refund(&self, id: i32) -> Result<Option<ShopDepositRefund>> {
        sqlx::query_as::<_, ShopDepositRefund>(
            "SELECT * FROM shop_deposit_refund WHERE shop_deposit_refund_id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— 展示富化 —————————————————————

    /// 店铺（回填店铺名/商户性质）。
    pub async fn find_shop(&self, shop_id: i32) -> Result<Option<Shop>> {
        sqlx::query_as::<_, Shop>("SELECT * FROM shop WHERE shop_id = $1 LIMIT 1")
            .bind(shop_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 入驻供应商（Java `vendorMapper.selectOnboardedById`：`source_type = 1`）。
    pub async fn find_vendor(&self, vendor_id: i32) -> Result<Option<Vendor>> {
        sqlx::query_as::<_, Vendor>(
            "SELECT * FROM vendor WHERE vendor_id = $1 AND source_type = 1 LIMIT 1",
        )
        .bind(vendor_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 商户（回填商户性质 `merchant.type`）。
    pub async fn find_merchant(&self, merchant_id: i32) -> Result<Option<Merchant>> {
        sqlx::query_as::<_, Merchant>(
            "SELECT * FROM merchant WHERE merchant_id = $1 LIMIT 1",
        )
        .bind(merchant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— 款项审核合并列表 —————————————————————

    /// 当前 owner 的充值单（Java `loadRechargeRows`：owner 锁定 + 状态/时间筛选）。
    pub async fn list_owner_recharges(
        &self,
        owner_type: i32,
        owner_id: i32,
        status: Option<i32>,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> Result<Vec<ShopDepositRecharge>> {
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT * FROM shop_deposit_recharge WHERE owner_type = ",
        );
        qb.push_bind(owner_type)
            .push(" AND owner_id = ")
            .push_bind(owner_id);
        if let Some(status) = status {
            qb.push(" AND status = ").push_bind(status);
        }
        if let Some(start_time) = start_time {
            qb.push(" AND add_time >= ").push_bind(start_time);
        }
        if let Some(end_time) = end_time {
            qb.push(" AND add_time <= ").push_bind(end_time);
        }
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 当前 owner 的退还单（Java `loadRefundRows`，本域只读）。
    pub async fn list_owner_refunds(
        &self,
        owner_type: i32,
        owner_id: i32,
        status: Option<i32>,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> Result<Vec<ShopDepositRefund>> {
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT * FROM shop_deposit_refund WHERE owner_type = ");
        qb.push_bind(owner_type)
            .push(" AND owner_id = ")
            .push_bind(owner_id);
        if let Some(status) = status {
            qb.push(" AND status = ").push_bind(status);
        }
        if let Some(start_time) = start_time {
            qb.push(" AND add_time >= ").push_bind(start_time);
        }
        if let Some(end_time) = end_time {
            qb.push(" AND add_time <= ").push_bind(end_time);
        }
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    // ————————————————————— 审核（事务内） —————————————————————

    /// 事务内按 id 查充值单。
    pub async fn find_recharge_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        id: i32,
    ) -> Result<Option<ShopDepositRecharge>> {
        sqlx::query_as::<_, ShopDepositRecharge>(
            "SELECT * FROM shop_deposit_recharge WHERE shop_deposit_recharge_id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_err)
    }

    /// 事务内取保证金账户（不存在则创建），并对行加锁
    /// （Java `DepositAccountServiceImpl#getOrCreate`：`selectForUpdateByOwner`）。
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
             VALUES ($1, $2, 0, NULL, 0, 0, 0, 0, 0, 1, $3, $4)
             ON CONFLICT (owner_type, owner_id) DO NOTHING",
        )
        .bind(owner_type)
        .bind(owner_id)
        .bind(now)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        // 并发下另一事务已插入时 ON CONFLICT 静默让路，这里回读同一条
        // （等价 Java 的 catch (DuplicateKeyException) 后回读）。
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

    /// 事务内更新充值单（Java `this.updateById(recharge)`：仅写审核相关列）。
    pub async fn update_recharge_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        recharge: &ShopDepositRecharge,
    ) -> Result<bool> {
        let affected = sqlx::query(
            "UPDATE shop_deposit_recharge SET status = $1, pay_voucher = $2,
               audit_admin_id = $3, audit_time = $4, audit_remark = $5, update_time = $6
             WHERE shop_deposit_recharge_id = $7",
        )
        .bind(recharge.status)
        .bind(&recharge.pay_voucher)
        .bind(recharge.audit_admin_id)
        .bind(recharge.audit_time)
        .bind(&recharge.audit_remark)
        .bind(recharge.update_time)
        .bind(recharge.shop_deposit_recharge_id)
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
    use nextrade_core::models::merchant::deposit_recharge::DepositRechargeListQuery;

    /// 分页参数换算（Java `BasePage`）在仓储与服务层共用。
    #[test]
    fn page_params_are_shared_with_model() {
        assert_eq!(nextrade_core::models::merchant::deposit_recharge::page_params(None, None), (1, 15));
        assert_eq!(
            nextrade_core::models::merchant::deposit_recharge::page_params(Some(3), Some(500)),
            (3, 100)
        );
    }

    /// 查询对象的 ownerId 兼容口径（ownerId 优先，回退 shopId，非正数忽略）。
    #[test]
    fn query_effective_owner_id_prefers_owner_id() {
        let q = DepositRechargeListQuery {
            owner_id: Some(5),
            shop_id: Some(9),
            ..Default::default()
        };
        assert_eq!(q.effective_owner_id(), Some(5));
        let q = DepositRechargeListQuery {
            owner_id: None,
            shop_id: Some(9),
            ..Default::default()
        };
        assert_eq!(q.effective_owner_id(), Some(9));
        let q = DepositRechargeListQuery {
            owner_id: Some(0),
            shop_id: Some(0),
            ..Default::default()
        };
        assert_eq!(q.effective_owner_id(), None);
    }
}
