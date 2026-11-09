//! 账期结算域仓储（payment::credit_settlement）。
//!
//! 对应原 Java `CreditSettlementMapper` / `CreditOrderMapper` / `CreditUserMapper`
//! 以及 `CreditSettlementServiceImpl`、`CreditOrderServiceImpl`、
//! `CreditUserServiceImpl` 中本控制器用到的读写。
//!
//! 涉及表（列名均经 `python tools/check_sql_columns.py` 校验）：
//! - `credit_settlement`（15 列）/ `credit_order`（13 列）/ `credit_user`（10 列）
//! - 只读装配：`user`、`user_company`、`order`、`order_item`、`aftersales`、
//!   `aftersales_item`、`refund_apply`、`admin_user`
//!
//! # 事务
//!
//! `submit` / `cancel` 在 Java 里是 `@Transactional`（写结算记录 + 批量更新账期
//! 订单 + 调整授信额度 [+ 周期起点]）。本仓储提供 [`CreditSettlementRepository::begin`]
//! 与 `*_in_tx` 方法，让服务层把多次写入放进同一事务（与 price_inquiry /
//! feedback 同一做法）。

use std::collections::HashMap;

use rust_decimal::Decimal;
use sqlx::{FromRow,Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::{CreditOrder, CreditSettlement, CreditUser};
use nextrade_core::models::payment::credit_settlement::{
    format_time, limit_offset, CreditOrderListQuery, CreditSettlementHistoryQuery,
    ORDER_STATUS_REFUNDING, ORDER_STATUS_SETTLED, ORDER_STATUS_UNSETTLED, REFUND_STATUS_APPROVED,
};
use nextrade_core::{Error, Result};

/// `credit_settlement` 固定 SELECT 列清单。
const SETTLEMENT_SELECT: &str = "SELECT settlement_id, settlement_sn, user_id, credit_user_id, settlement_amount, order_count, remark, settlement_time, settlement_status, cancel_time, cancel_reason, operator_id, operator_name, create_time, update_time FROM credit_settlement";

/// `credit_order` 固定 SELECT 列清单。
const ORDER_SELECT: &str = "SELECT credit_order_id, order_id, order_sn, user_id, credit_user_id, credit_amount, settlement_status, settlement_id, settlement_sn, order_time, settlement_time, create_time, update_time FROM credit_order";

/// `credit_user` 固定 SELECT 列清单。
const CREDIT_USER_SELECT: &str = "SELECT credit_user_id, user_id, credit_limit, used_amount, available_amount, settlement_type, status, period_start_time, create_time, update_time FROM credit_user";

/// 用户装配行（`user`）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct UserRow {
    pub user_id: i32,
    pub username: String,
    pub mobile: String,
}

/// 企业认证装配行（`user_company`，`status = 2`）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct CompanyRow {
    pub user_id: i32,
    #[sqlx(rename = "type")]
    pub r#type: i32,
    pub contact_name: String,
    pub company_name: String,
}

/// 退款申请装配行（`refund_apply`，`refund_status = 2`）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct RefundApplyRow {
    pub refund_id: i32,
    pub order_id: i32,
    pub aftersale_id: i32,
}

/// 售后装配行（`aftersales`）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct AftersalesRow {
    pub aftersale_id: i32,
    pub order_id: Option<i32>,
    pub aftersale_type: i32,
    pub refund_amount: Decimal,
}

/// 订单装配行（`order`）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct OrderBriefRow {
    pub order_id: i32,
    pub total_amount: Decimal,
    pub order_status: i32,
    pub shipping_status: i32,
    pub shipping_time: i32,
    pub shipping_fee: Decimal,
}

/// 订单商品装配行（`order_item`）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct OrderItemRow {
    pub item_id: i32,
    pub order_id: i32,
    pub quantity: i32,
}

/// 售后商品装配行（`aftersales_item`）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct AftersalesItemRow {
    pub aftersale_id: Option<i32>,
    pub order_item_id: Option<i32>,
    pub number: Option<i32>,
}

/// 账期结算域仓储。
#[derive(Clone)]
pub struct CreditSettlementRepository {
    pool: PgPool,
}

impl CreditSettlementRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 开启事务（`submit` / `cancel` 的多次写入）。
    pub async fn begin(&self) -> Result<sqlx::Transaction<'static, Postgres>> {
        self.pool.begin().await.map_err(map_err)
    }

    // ————————————————————— credit_settlement —————————————————————

    /// 按主键查询结算记录（Java `getById`）。
    pub async fn find_settlement_by_id(&self, id: i32) -> Result<Option<CreditSettlement>> {
        sqlx::query_as::<_, CreditSettlement>(&format!(
            "{SETTLEMENT_SELECT} WHERE settlement_id = $1 LIMIT 1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按回款流水单号精确查询（Java `selectBySettlementSn`）。
    pub async fn find_settlement_by_sn(&self, sn: &str) -> Result<Option<CreditSettlement>> {
        sqlx::query_as::<_, CreditSettlement>(&format!(
            "{SETTLEMENT_SELECT} WHERE settlement_sn = $1 LIMIT 1"
        ))
        .bind(sn)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 历史结算分页（Java `history`：可选过滤 + `settlement_time DESC`）。
    pub async fn paginate_history(
        &self,
        q: &CreditSettlementHistoryQuery,
        start_epoch: Option<i64>,
        end_epoch: Option<i64>,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<CreditSettlement>)> {
        let mut cq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM credit_settlement WHERE 1 = 1");
        Self::push_history_filters(&mut cq, q, start_epoch, end_epoch);
        let total: (i64,) = cq
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut rq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{SETTLEMENT_SELECT} WHERE 1 = 1"));
        Self::push_history_filters(&mut rq, q, start_epoch, end_epoch);
        rq.push(q.order_clause());
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<CreditSettlement> = rq
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0, rows))
    }

    fn push_history_filters(
        qb: &mut QueryBuilder<'_, Postgres>,
        q: &CreditSettlementHistoryQuery,
        start_epoch: Option<i64>,
        end_epoch: Option<i64>,
    ) {
        if let Some(user_id) = q.user_id {
            qb.push(" AND user_id = ");
            qb.push_bind(user_id);
        }
        if let Some(sn) = q.effective_settlement_sn() {
            qb.push(" AND settlement_sn LIKE ");
            qb.push_bind(format!("%{sn}%"));
        }
        if let Some(status) = q.settlement_status {
            qb.push(" AND settlement_status = ");
            qb.push_bind(status);
        }
        if let Some(start) = start_epoch {
            qb.push(" AND settlement_time >= ");
            qb.push_bind(start);
        }
        if let Some(end) = end_epoch {
            qb.push(" AND settlement_time <= ");
            qb.push_bind(end);
        }
    }

    /// 新增结算记录（Java `save`）。
    pub async fn insert_settlement_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        entity: &CreditSettlement,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO credit_settlement (settlement_sn, user_id, credit_user_id, settlement_amount, order_count, remark, settlement_time, settlement_status, cancel_time, cancel_reason, operator_id, operator_name, create_time, update_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) RETURNING settlement_id",
        )
        .bind(&entity.settlement_sn)
        .bind(entity.user_id)
        .bind(entity.credit_user_id)
        .bind(entity.settlement_amount)
        .bind(entity.order_count)
        .bind(&entity.remark)
        .bind(entity.settlement_time)
        .bind(entity.settlement_status)
        .bind(entity.cancel_time)
        .bind(&entity.cancel_reason)
        .bind(entity.operator_id)
        .bind(&entity.operator_name)
        .bind(entity.create_time)
        .bind(entity.update_time)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 撤销结算记录（Java `updateById` 的 status/cancelTime/cancelReason/updateTime）。
    pub async fn cancel_settlement_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        settlement_id: i32,
        cancel_reason: Option<&str>,
        cancel_time: i64,
        update_time: i64,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE credit_settlement SET settlement_status = 2, cancel_time = $1, cancel_reason = $2, update_time = $3 WHERE settlement_id = $4",
        )
        .bind(cancel_time)
        .bind(cancel_reason)
        .bind(update_time)
        .bind(settlement_id)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 批量取用户名。
    pub async fn find_usernames(&self, ids: &[i32]) -> Result<HashMap<i32, String>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT user_id, username FROM \"user\" WHERE user_id IN (");
        Self::push_int_list(&mut qb, ids);
        qb.push(")");
        let rows: Vec<(i32, String)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().collect())
    }

    /// 操作人用户名（Java `adminUserMapper.selectById(adminId).getUsername()`）。
    pub async fn find_admin_username(&self, admin_id: i32) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT username FROM admin_user WHERE admin_id = $1 LIMIT 1",
        )
        .bind(admin_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(v,)| v))
    }

    // ————————————————————— credit_user —————————————————————

    /// 按主键查授信会员（Java `getById`）。
    pub async fn find_credit_user_by_id(&self, id: i32) -> Result<Option<CreditUser>> {
        sqlx::query_as::<_, CreditUser>(&format!(
            "{CREDIT_USER_SELECT} WHERE credit_user_id = $1 LIMIT 1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按会员 ID 查授信会员（Java `selectByUserId`）。
    pub async fn find_credit_user_by_user_id(&self, user_id: i32) -> Result<Option<CreditUser>> {
        sqlx::query_as::<_, CreditUser>(&format!(
            "{CREDIT_USER_SELECT} WHERE user_id = $1 LIMIT 1"
        ))
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 授信会员分页（Java `getUserList`：可选 `user_id IN (...)` + `create_time DESC`）。
    pub async fn paginate_credit_users(
        &self,
        user_ids: Option<&[i32]>,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<CreditUser>)> {
        let mut cq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM credit_user WHERE 1 = 1");
        Self::push_credit_user_filter(&mut cq, user_ids);
        let total: (i64,) = cq
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut rq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{CREDIT_USER_SELECT} WHERE 1 = 1"));
        Self::push_credit_user_filter(&mut rq, user_ids);
        rq.push(" ORDER BY create_time DESC");
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<CreditUser> = rq
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0, rows))
    }

    fn push_credit_user_filter(qb: &mut QueryBuilder<'_, Postgres>, user_ids: Option<&[i32]>) {
        if let Some(ids) = user_ids {
            qb.push(" AND user_id IN (");
            Self::push_int_list(qb, ids);
            qb.push(")");
        }
    }

    /// 批量用户信息（用户名 / 手机号）。
    pub async fn find_users(&self, ids: &[i32]) -> Result<Vec<UserRow>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT user_id, username, mobile FROM \"user\" WHERE user_id IN (");
        Self::push_int_list(&mut qb, ids);
        qb.push(")");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量企业认证（`status = 2`；每用户取 `id` 最小一行，Java `limit 1` 的
    /// 确定性落地）。
    pub async fn find_companies(&self, user_ids: &[i32]) -> Result<Vec<CompanyRow>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT user_id, \"type\", contact_name, company_name FROM user_company WHERE status = 2 AND user_id IN (",
        );
        Self::push_int_list(&mut qb, user_ids);
        qb.push(") ORDER BY user_id, id");
        let rows: Vec<CompanyRow> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        // 每用户仅保留第一行（ORDER BY user_id, id）
        let mut out: Vec<CompanyRow> = Vec::new();
        for row in rows {
            if out.last().map(|r: &CompanyRow| r.user_id) != Some(row.user_id) {
                out.push(row);
            }
        }
        Ok(out)
    }

    /// 批量企业认证（**不带 status 过滤**，Java `CreditOrderServiceImpl#convertToVO`
    /// 的 `userCompanyMapper.selectOne(userId LIMIT 1)` 口径；每用户取 `id` 最小行）。
    pub async fn find_companies_any(&self, user_ids: &[i32]) -> Result<Vec<CompanyRow>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT user_id, \"type\", contact_name, company_name FROM user_company WHERE user_id IN (",
        );
        Self::push_int_list(&mut qb, user_ids);
        qb.push(") ORDER BY user_id, id");
        let rows: Vec<CompanyRow> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        let mut out: Vec<CompanyRow> = Vec::new();
        for row in rows {
            if out.last().map(|r: &CompanyRow| r.user_id) != Some(row.user_id) {
                out.push(row);
            }
        }
        Ok(out)
    }

    /// 待结算统计（`settlement_status = 0` 的订单数 + 账期金额合计）。
    pub async fn pending_stats(&self, user_ids: &[i32]) -> Result<Vec<(i32, i64, Decimal)>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT user_id, COUNT(*), CAST(COALESCE(SUM(credit_amount), 0) AS DECIMAL(12,2)) FROM credit_order WHERE settlement_status = 0 AND user_id IN (",
        );
        Self::push_int_list(&mut qb, user_ids);
        qb.push(") GROUP BY user_id");
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 各用户最早的一笔「未结算/退款中」订单时间（用于逾期判断）。
    pub async fn min_pending_order_time(&self, user_ids: &[i32]) -> Result<Vec<(i32, Option<i64>)>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT user_id, MIN(order_time) FROM credit_order WHERE settlement_status IN (0, 3) AND user_id IN (",
        );
        Self::push_int_list(&mut qb, user_ids);
        qb.push(") GROUP BY user_id");
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 是否存在上期未结算订单（`order_time < 周期起点` 且状态 ∈ {0,3}）。
    pub async fn count_pending_before_period(
        &self,
        user_id: i32,
        period_start_epoch: i64,
    ) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM credit_order WHERE user_id = $1 AND settlement_status IN ($2, $3) AND order_time < $4",
        )
        .bind(user_id)
        .bind(ORDER_STATUS_UNSETTLED)
        .bind(ORDER_STATUS_REFUNDING)
        .bind(period_start_epoch)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 待结算/退款中订单数（事务内版本：与刚写入的订单状态保持一致）。
    pub async fn count_pending_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        user_id: i32,
    ) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM credit_order WHERE user_id = $1 AND settlement_status IN ($2, $3)",
        )
        .bind(user_id)
        .bind(ORDER_STATUS_UNSETTLED)
        .bind(ORDER_STATUS_REFUNDING)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 待结算/退款中订单数（Java `resetPeriodStartIfNoPending` / `hasPending`）。
    pub async fn count_pending(&self, user_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM credit_order WHERE user_id = $1 AND settlement_status IN ($2, $3)",
        )
        .bind(user_id)
        .bind(ORDER_STATUS_UNSETTLED)
        .bind(ORDER_STATUS_REFUNDING)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    // ————————————————————— credit_order —————————————————————

    /// 账期订单分页（Java `CreditOrderServiceImpl#list`）。
    ///
    /// `user_ids = Some([])` / `order_ids = Some([])` 表示「筛选命中为空」，
    /// 由服务层直接返回空页；此处空切片会生成 `IN ()`，因此调用方须先短路。
    pub async fn paginate_credit_orders(
        &self,
        q: &CreditOrderListQuery,
        user_ids: Option<&[i32]>,
        order_ids: Option<&[i32]>,
        start_epoch: Option<i64>,
        end_epoch: Option<i64>,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<CreditOrder>)> {
        let mut cq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM credit_order WHERE 1 = 1");
        Self::push_order_filters(&mut cq, q, user_ids, order_ids, start_epoch, end_epoch);
        let total: (i64,) = cq
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut rq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{ORDER_SELECT} WHERE 1 = 1"));
        Self::push_order_filters(&mut rq, q, user_ids, order_ids, start_epoch, end_epoch);
        rq.push(q.order_clause());
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<CreditOrder> = rq
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0, rows))
    }

    fn push_order_filters(
        qb: &mut QueryBuilder<'_, Postgres>,
        q: &CreditOrderListQuery,
        user_ids: Option<&[i32]>,
        order_ids: Option<&[i32]>,
        start_epoch: Option<i64>,
        end_epoch: Option<i64>,
    ) {
        if let Some(sn) = q.effective(&q.order_sn) {
            qb.push(" AND order_sn LIKE ");
            qb.push_bind(format!("%{sn}%"));
        }
        if let Some(user_id) = q.user_id {
            qb.push(" AND user_id = ");
            qb.push_bind(user_id);
        }
        if let Some(ids) = user_ids {
            qb.push(" AND user_id IN (");
            Self::push_int_list(qb, ids);
            qb.push(")");
        }
        if let Some(ids) = order_ids {
            qb.push(" AND order_id IN (");
            Self::push_int_list(qb, ids);
            qb.push(")");
        }
        if let Some(list) = q.settlement_status_list.as_deref().filter(|v| !v.is_empty()) {
            qb.push(" AND settlement_status IN (");
            Self::push_int_list(qb, list);
            qb.push(")");
        } else if let Some(status) = q.settlement_status {
            qb.push(" AND settlement_status = ");
            qb.push_bind(status);
        }
        if let Some(sn) = q.effective(&q.settlement_sn) {
            qb.push(" AND settlement_sn LIKE ");
            qb.push_bind(format!("%{sn}%"));
        }
        if let Some(settlement_id) = q.settlement_id {
            qb.push(" AND settlement_id = ");
            qb.push_bind(settlement_id);
        }
        if let Some(start) = start_epoch {
            qb.push(" AND order_time >= ");
            qb.push_bind(start);
        }
        if let Some(end) = end_epoch {
            qb.push(" AND order_time <= ");
            qb.push_bind(end);
        }
    }

    /// 按订单 ID 查账期订单（Java `selectByOrderId`）。
    pub async fn find_credit_order_by_order_id(
        &self,
        order_id: i32,
    ) -> Result<Option<CreditOrder>> {
        sqlx::query_as::<_, CreditOrder>(&format!(
            "{ORDER_SELECT} WHERE order_id = $1 LIMIT 1"
        ))
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 批量更新为已结算（Java `updateSettlementStatus`）。
    pub async fn update_orders_settled_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        order_ids: &[i32],
        settlement_id: i32,
        settlement_sn: &str,
        now: i64,
    ) -> Result<u64> {
        if order_ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "UPDATE credit_order SET settlement_status = ",
        );
        qb.push_bind(ORDER_STATUS_SETTLED);
        qb.push(", settlement_id = ");
        qb.push_bind(settlement_id);
        qb.push(", settlement_sn = ");
        qb.push_bind(settlement_sn.to_string());
        qb.push(", settlement_time = ");
        qb.push_bind(now);
        qb.push(", update_time = ");
        qb.push_bind(now);
        qb.push(" WHERE order_id IN (");
        Self::push_int_list(&mut qb, order_ids);
        qb.push(")");
        let res = qb.build().execute(&mut **tx).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 批量撤销结算状态（Java `cancelSettlementStatus`）。
    pub async fn cancel_orders_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        settlement_id: i32,
        now: i64,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE credit_order SET settlement_status = 0, update_time = $1 WHERE settlement_id = $2",
        )
        .bind(now)
        .bind(settlement_id)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 恢复可用额度（Java `restoreAmount`：`used_amount >= amount` 才更新）。
    pub async fn restore_amount_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        user_id: i32,
        amount: Decimal,
        now: i64,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE credit_user SET used_amount = used_amount - $1, available_amount = available_amount + $2, update_time = $3 WHERE user_id = $4 AND used_amount >= $5",
        )
        .bind(amount)
        .bind(amount)
        .bind(now)
        .bind(user_id)
        .bind(amount)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 扣减可用额度（Java `deductAmount`：`available_amount >= amount` 才更新）。
    pub async fn deduct_amount_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        user_id: i32,
        amount: Decimal,
        now: i64,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE credit_user SET used_amount = used_amount + $1, available_amount = available_amount - $2, update_time = $3 WHERE user_id = $4 AND available_amount >= $5",
        )
        .bind(amount)
        .bind(amount)
        .bind(now)
        .bind(user_id)
        .bind(amount)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 无待结算订单时初始化周期起点（Java `resetPeriodStartIfNoPending`：
    /// 仅当 `period_start_time` 为空才写当天 0 点）。
    pub async fn reset_period_start_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        user_id: i32,
        today_start_epoch: i64,
        now: i64,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE credit_user SET period_start_time = $1, update_time = $2 WHERE user_id = $3 AND (period_start_time IS NULL OR period_start_time = 0)",
        )
        .bind(today_start_epoch)
        .bind(now)
        .bind(user_id)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— 退款金额装配 —————————————————————

    /// 财务审核通过的退款申请（`refund_status = 2`）。
    pub async fn find_approved_refunds(
        &self,
        order_ids: &[i32],
    ) -> Result<Vec<RefundApplyRow>> {
        if order_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT refund_id, order_id, aftersale_id FROM refund_apply WHERE refund_status = ",
        );
        qb.push_bind(REFUND_STATUS_APPROVED);
        qb.push(" AND order_id IN (");
        Self::push_int_list(&mut qb, order_ids);
        qb.push(")");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 售后记录（限「退货/退款 + 仅退款」，Java 用于退款金额）。
    pub async fn find_aftersales_by_ids(&self, ids: &[i32]) -> Result<Vec<AftersalesRow>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT aftersale_id, order_id, aftersale_type, refund_amount FROM aftersales WHERE aftersale_type IN (1, 2) AND aftersale_id IN (",
        );
        Self::push_int_list(&mut qb, ids);
        qb.push(")");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 订单装配（金额 / 状态 / 运费）。
    pub async fn find_order_briefs(&self, order_ids: &[i32]) -> Result<Vec<OrderBriefRow>> {
        if order_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT order_id, total_amount, order_status, shipping_status, shipping_time, shipping_fee FROM \"order\" WHERE order_id IN (",
        );
        Self::push_int_list(&mut qb, order_ids);
        qb.push(")");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 订单商品（退款运费判定用）。
    pub async fn find_order_items(&self, order_ids: &[i32]) -> Result<Vec<OrderItemRow>> {
        if order_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT item_id, order_id, quantity FROM order_item WHERE order_id IN (",
        );
        Self::push_int_list(&mut qb, order_ids);
        qb.push(")");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 已完成售后（`status = 6`）：`(order_id, aftersale_id)`。
    pub async fn find_completed_aftersales(
        &self,
        order_ids: &[i32],
    ) -> Result<Vec<(Option<i32>, i32)>> {
        if order_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT order_id, aftersale_id FROM aftersales WHERE status = 6 AND order_id IN (",
        );
        Self::push_int_list(&mut qb, order_ids);
        qb.push(")");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 售后商品明细（已完成售后）。
    pub async fn find_aftersales_items(
        &self,
        aftersale_ids: &[i32],
    ) -> Result<Vec<AftersalesItemRow>> {
        if aftersale_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT aftersale_id, order_item_id, number FROM aftersales_item WHERE aftersale_id IN (",
        );
        Self::push_int_list(&mut qb, aftersale_ids);
        qb.push(")");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 用户 / 订单筛选辅助 —————————————————————

    /// 用户名模糊匹配的会员 ID（`user.username LIKE`）。
    pub async fn find_user_ids_by_username_like(&self, keyword: &str) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> =
            sqlx::query_as("SELECT user_id FROM \"user\" WHERE username LIKE $1")
                .bind(format!("%{keyword}%"))
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 用户名 / 手机号模糊匹配的会员 ID（`CreditOrderListDTO` 的两个条件为 AND）。
    pub async fn find_user_ids_by_name_or_mobile(
        &self,
        username: Option<&str>,
        mobile: Option<&str>,
    ) -> Result<Vec<i32>> {
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new("SELECT user_id FROM \"user\" WHERE 1 = 1");
        if let Some(username) = username {
            qb.push(" AND username LIKE ");
            qb.push_bind(format!("%{username}%"));
        }
        if let Some(mobile) = mobile {
            qb.push(" AND mobile LIKE ");
            qb.push_bind(format!("%{mobile}%"));
        }
        let rows: Vec<(i32,)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 指定订单状态的订单 ID（`CreditOrderListDTO.orderStatus` 过滤）。
    pub async fn find_order_ids_by_status(&self, order_status: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> =
            sqlx::query_as("SELECT order_id FROM \"order\" WHERE order_status = $1")
                .bind(order_status)
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 追加 `IN (...)` 的整数列表（调用方保证非空）。
    fn push_int_list(qb: &mut QueryBuilder<'_, Postgres>, ids: &[i32]) {
        {
            let mut sep = qb.separated(", ");
            for id in ids {
                sep.push_bind(*id);
            }
        }
    }

    /// 便捷时间格式化（供服务层复用，避免重复 chrono 逻辑）。
    #[must_use]
    pub fn format_secs(secs: Option<i64>) -> String {
        format_time(secs)
    }

    /// `(LIMIT, OFFSET)` 透传（供服务层统一口径）。
    #[must_use]
    pub fn page_window(page: Option<i64>, size: Option<i64>) -> (i64, i64) {
        limit_offset(page, size)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::payment::credit_settlement::{
        ORDER_COLUMNS, SETTLEMENT_COLUMNS,
    };

    #[test]
    fn settlement_select_matches_schema_columns() {
        let selected = SETTLEMENT_SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap();
        assert_eq!(selected, SETTLEMENT_COLUMNS.join(", "));
    }

    #[test]
    fn order_select_matches_schema_columns() {
        let selected = ORDER_SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap();
        assert_eq!(selected, ORDER_COLUMNS.join(", "));
    }

    #[test]
    fn credit_user_select_has_ten_columns() {
        let selected = CREDIT_USER_SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap();
        assert_eq!(selected.split(", ").count(), 10);
    }

    #[test]
    fn static_guards_for_sensitive_sql() {
        let src = include_str!("credit_settlement.rs");
        // 事务内三写：结算记录 / 账期订单 / 授信额度
        assert!(src.contains("INSERT INTO credit_settlement"));
        assert!(src.contains("UPDATE credit_order SET settlement_status = 0"));
        assert!(src.contains("used_amount = used_amount - ?"));
        assert!(src.contains("available_amount = available_amount - ?"));
        // 退款金额装配的审核通过过滤
        assert!(src.contains("WHERE refund_status = "));
    }
}
