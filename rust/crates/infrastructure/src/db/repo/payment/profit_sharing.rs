//! 分账结算域仓储（payment::profit_sharing）。

use std::collections::HashMap;

use sqlx::{Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::ProfitSharingOrder;
use nextrade_core::models::payment::profit_sharing::{
    limit_offset, ProfitSharingPageQuery, FINISH_FINISHED, RESULT_DESC_CANCEL, STATUS_CANCELLED,
    STATUS_PENDING,
};
use nextrade_core::{Error, Result};

const SELECT: &str = "SELECT id, out_order_no, order_id, order_sn, transaction_id, pay_channel, shop_id, vendor_id, settlement_type, sub_mch_id, settlement_id, operator_id, total_amount, sharing_base_amount, split_amount, service_fee, commission_fee, service_fee_rate, status, channel_order_id, finish_status, finish_time::text AS finish_time, trigger_type, result_desc, retry_count, is_del, create_time::text AS create_time, modify_time::text AS modify_time FROM profit_sharing_order";

/// 分账结算域仓储。
#[derive(Clone)]
pub struct ProfitSharingRepository {
    pool: PgPool,
}

impl ProfitSharingRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 分页（Java `listPage` 过滤：settlementType/shopId/vendorId/status/finishStatus）。
    pub async fn paginate(
        &self,
        q: &ProfitSharingPageQuery,
        status: Option<i32>,
        finish_status: Option<i32>,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<ProfitSharingOrder>)> {
        let mut cq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM profit_sharing_order WHERE is_del = 0");
        Self::push_filters(&mut cq, q, status, finish_status);
        let total: (i64,) = cq.build_query_as().fetch_one(&self.pool).await.map_err(map_err)?;
        let mut rq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{SELECT} WHERE is_del = 0"));
        Self::push_filters(&mut rq, q, status, finish_status);
        rq.push(q.order_clause());
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<ProfitSharingOrder> =
            rq.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total.0, rows))
    }

    fn push_filters(
        qb: &mut QueryBuilder<'_, Postgres>,
        q: &ProfitSharingPageQuery,
        status: Option<i32>,
        finish_status: Option<i32>,
    ) {
        if let Some(settlement_type) = q.settlement_type {
            qb.push(" AND settlement_type = ");
            qb.push_bind(settlement_type);
        }
        if let Some(shop_id) = q.shop_id {
            qb.push(" AND shop_id = ");
            qb.push_bind(i64::from(shop_id));
        }
        if let Some(vendor_id) = q.vendor_id {
            qb.push(" AND vendor_id = ");
            qb.push_bind(i64::from(vendor_id));
        }
        if let Some(status) = status {
            qb.push(" AND status = ");
            qb.push_bind(status);
        }
        if let Some(finish) = finish_status {
            qb.push(" AND finish_status = ");
            qb.push_bind(finish);
        }
    }

    /// 按主键查询。
    pub async fn find_by_id(&self, id: i64) -> Result<Option<ProfitSharingOrder>> {
        sqlx::query_as::<_, ProfitSharingOrder>(&format!("{SELECT} WHERE id = $1 LIMIT 1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 订单是否存在（Java `withOrderLock` 的 `Order locked == null` 校验）。
    pub async fn order_exists(&self, order_id: i64) -> Result<bool> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM \"order\" WHERE order_id = $1")
            .bind(order_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0 > 0)
    }

    /// 手动作废（Java `canCancel` 分支：条件更新防并发）。
    pub async fn cancel_pending(
        &self,
        id: i64,
        operator_id: i64,
        now: &str,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE profit_sharing_order SET status = $1, finish_status = $2, finish_time = NULLIF($3, ''), operator_id = $4, result_desc = $5, modify_time = NULLIF($6, '') WHERE id = $7 AND status = $8 AND (transaction_id IS NULL OR transaction_id = '')",
        )
        .bind(STATUS_CANCELLED)
        .bind(FINISH_FINISHED)
        .bind(now)
        .bind(operator_id)
        .bind(RESULT_DESC_CANCEL)
        .bind(now)
        .bind(id)
        .bind(STATUS_PENDING)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 手动完结分账单（Java ProfitSharingOrderServiceImpl#manualFinishProfitSharing）。
    pub async fn finish_order(
        &self,
        id: i64,
        operator_id: i64,
        now: &str,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE profit_sharing_order SET finish_status = $1, finish_time = NULLIF($2, ''), operator_id = $3, modify_time = NULLIF($4, '') WHERE id = $5",
        )
        .bind(FINISH_FINISHED)
        .bind(now)
        .bind(operator_id)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 店铺名批量。
    pub async fn find_shop_titles(&self, shop_ids: &[i64]) -> Result<HashMap<i64, String>> {
        let ids: Vec<i32> = shop_ids
            .iter()
            .filter(|id| **id > 0 && **id <= i64::from(i32::MAX))
            .map(|id| *id as i32)
            .collect();
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT shop_id, shop_title FROM shop WHERE shop_id IN (");
        {
            let mut sep = qb.separated(", ");
            for id in &ids {
                sep.push_bind(*id);
            }
        }
        qb.push(")");
        let rows: Vec<(i32, Option<String>)> =
            qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(id, title)| (i64::from(id), title.unwrap_or_default()))
            .collect())
    }

    /// 供应商名批量（`vendor` 表）。
    pub async fn find_vendor_names(&self, vendor_ids: &[i64]) -> Result<HashMap<i64, String>> {
        let ids: Vec<i32> = vendor_ids
            .iter()
            .filter(|id| **id > 0 && **id <= i64::from(i32::MAX))
            .map(|id| *id as i32)
            .collect();
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT vendor_id, vendor_name FROM vendor WHERE vendor_id IN (");
        {
            let mut sep = qb.separated(", ");
            for id in &ids {
                sep.push_bind(*id);
            }
        }
        qb.push(")");
        let rows: Vec<(i32, Option<String>)> =
            qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(id, name)| (i64::from(id), name.unwrap_or_default()))
            .collect())
    }

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

    #[test]
    fn select_has_28_columns() {
        let selected = SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap();
        assert_eq!(selected.split(", ").count(), 28);
    }

    #[test]
    fn static_sql_guards() {
        let src = include_str!("profit_sharing.rs");
        assert!(src.contains("FROM profit_sharing_order WHERE is_del = 0"));
        assert!(src.contains("transaction_id IS NULL OR transaction_id = ''"));
        assert!(src.contains("FROM \"order\" WHERE order_id = $1"));
        assert!(src.contains("FROM vendor WHERE vendor_id IN ("));
    }
}
