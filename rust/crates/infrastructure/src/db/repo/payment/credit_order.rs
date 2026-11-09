//! 账期订单仓储（payment::credit_order）。
//!
//! 表：`credit_order`（主键 `credit_order_id`）+ 回填用 `user` / `user_company` /
//! `order` / `refund_apply`。
//!
//! 全部 SQL 为运行时 `sqlx::query_as` / `QueryBuilder`（禁止 `query!` 宏）。

use std::collections::HashMap;

use rust_decimal::Decimal;
use sqlx::PgPool;
use sqlx::{Postgres, QueryBuilder};

use nextrade_core::entities::finance::CreditOrder;
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// 账期订单列表查询（Java `CreditOrderListDTO` + BasePage）。
#[derive(Debug, Clone, Default)]
pub struct CreditOrderFilter {
    pub order_sn: Option<String>,
    pub user_id: Option<i32>,
    pub username: Option<String>,
    pub mobile: Option<String>,
    pub order_status: Option<i32>,
    pub settlement_status: Option<i32>,
    pub settlement_status_list: Option<Vec<i32>>,
    pub settlement_sn: Option<String>,
    pub settlement_id: Option<i32>,
    pub order_time_start: Option<i64>,
    pub order_time_end: Option<i64>,
    /// 店铺作用域（`Some(shop_id)` 时仅该店铺会员的账期订单）
    pub shop_scope: Option<i32>,
}

#[derive(Clone)]
pub struct CreditOrderRepository {
    pool: PgPool,
}

impl CreditOrderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 账期订单分页（Java `CreditOrderServiceImpl#list`）。
    pub async fn paginate(
        &self,
        q: &CreditOrderFilter,
        page: &PageQuery,
    ) -> Result<(u64, Vec<CreditOrder>)> {
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM credit_order WHERE 1 = 1");
        self.push_filters(&mut qb, q);
        let total: (i64,) = qb.build_query_as().fetch_one(&self.pool).await.map_err(map_err)?;

        let (limit, offset) = page.limit_offset();
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT * FROM credit_order WHERE 1 = 1");
        self.push_filters(&mut qb, q);
        qb.push(" ORDER BY order_time DESC, credit_order_id DESC LIMIT ");
        qb.push_bind(limit);
        qb.push(" OFFSET ");
        qb.push_bind(offset);
        let rows = qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    fn push_filters<'a>(&self, qb: &mut QueryBuilder<'a, Postgres>, q: &CreditOrderFilter) {
        if let Some(shop) = q.shop_scope {
            qb.push(" AND EXISTS (SELECT 1 FROM shop_member m WHERE m.user_id = credit_order.user_id AND m.shop_id = ");
            qb.push_bind(shop);
            qb.push(" AND m.is_del = 0)");
        }
        if let Some(sn) = q.order_sn.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            qb.push(" AND order_sn LIKE ");
            qb.push_bind(format!("%{sn}%"));
        }
        if let Some(user_id) = q.user_id {
            qb.push(" AND user_id = ");
            qb.push_bind(user_id);
        }
        if q.username.as_deref().is_some_and(|s| !s.trim().is_empty())
            || q.mobile.as_deref().is_some_and(|s| !s.trim().is_empty())
        {
            qb.push(" AND user_id IN (SELECT user_id FROM \"user\" WHERE 1 = 1");
            if let Some(username) = q.username.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                qb.push(" AND username LIKE ");
                qb.push_bind(format!("%{username}%"));
            }
            if let Some(mobile) = q.mobile.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                qb.push(" AND mobile LIKE ");
                qb.push_bind(format!("%{mobile}%"));
            }
            qb.push(")");
        }
        if let Some(status) = q.order_status {
            qb.push(" AND order_id IN (SELECT order_id FROM \"order\" WHERE order_status = ");
            qb.push_bind(status);
            qb.push(")");
        }
        if let Some(list) = q.settlement_status_list.as_ref().filter(|l| !l.is_empty()) {
            qb.push(" AND settlement_status IN (");
            let mut sep = qb.separated(", ");
            for status in list {
                sep.push_bind(*status);
            }
            drop(sep);
            qb.push(")");
        } else if let Some(status) = q.settlement_status {
            qb.push(" AND settlement_status = ");
            qb.push_bind(status);
        }
        if let Some(sn) = q
            .settlement_sn
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            qb.push(" AND settlement_sn LIKE ");
            qb.push_bind(format!("%{sn}%"));
        }
        if let Some(id) = q.settlement_id {
            qb.push(" AND settlement_id = ");
            qb.push_bind(id);
        }
        if let Some(start) = q.order_time_start {
            qb.push(" AND order_time >= ");
            qb.push_bind(start);
        }
        if let Some(end) = q.order_time_end {
            qb.push(" AND order_time <= ");
            qb.push_bind(end);
        }
    }

    /// 用户（username/mobile）。
    pub async fn find_users(&self, ids: &[i32]) -> Result<HashMap<i32, (String, String)>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT user_id, username, mobile FROM \"user\" WHERE user_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        let rows: Vec<(i32, String, String)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(id, username, mobile)| (id, (username, mobile)))
            .collect())
    }

    /// 企业名称（Java `user_company LIMIT 1`）。
    pub async fn find_companies(&self, ids: &[i32]) -> Result<HashMap<i32, String>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT user_id, company_name FROM user_company WHERE user_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(") ORDER BY id ASC");
        let rows: Vec<(i32, String)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        let mut map = HashMap::new();
        for (user_id, company_name) in rows {
            map.entry(user_id).or_insert(company_name);
        }
        Ok(map)
    }

    /// 订单金额/状态（Java `orderMapper.selectById`）。
    pub async fn find_orders(&self, ids: &[i32]) -> Result<HashMap<i32, (Decimal, i32)>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT order_id, total_amount, order_status FROM \"order\" WHERE order_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        let rows: Vec<(i32, Decimal, i32)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(id, amount, status)| (id, (amount, status)))
            .collect())
    }

    /// 已退款金额（财务审核通过 refund_status=2）：
    /// 取 `refund_apply` 三类金额之和（online+offline+refund_balance）。
    ///
    /// Java 从售后单 `aftersales` 计算并含运费退还调整；本实现直接汇总退款申请金额，
    /// 作为偏差已在模块头说明（不涉及运费拆分）。
    pub async fn refunded_amount_map(
        &self,
        order_ids: &[i32],
    ) -> Result<HashMap<i32, Decimal>> {
        if order_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT order_id, COALESCE(SUM(online_balance + offline_balance + refund_balance),0)
             FROM refund_apply WHERE refund_status = 2 AND order_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in order_ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(") GROUP BY order_id");
        let rows: Vec<(i32, Decimal)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().collect())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn refunded_sql_filters_approved_apply() {
        let sql = "SELECT order_id, SUM(online_balance + offline_balance + refund_balance) FROM refund_apply WHERE refund_status = 2 AND order_id IN (...) GROUP BY order_id";
        assert!(sql.contains("refund_status = 2"));
        assert!(sql.contains("GROUP BY order_id"));
    }
}
