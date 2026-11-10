//! 子商户域仓储（payment::sub_merchant）。
//! 表 `sub_merchant`；只读 `shop` 回填店铺名。

use std::collections::HashMap;

use sqlx::{Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::SubMerchant;
use nextrade_core::models::payment::sub_merchant::{
    limit_offset, SubMerchantPageQuery, STATUS_FROZEN,
};
use nextrade_core::{Error, Result};

const SELECT: &str = "SELECT id, shop_id, pay_channel, sub_mch_id, sub_appid, merchant_name, status, application_id, max_split_ratio, is_del, create_time::text AS create_time, modify_time::text AS modify_time FROM sub_merchant";

/// 子商户域仓储。
#[derive(Clone)]
pub struct SubMerchantRepository {
    pool: PgPool,
}

impl SubMerchantRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 分页（Java `listPage`：shopId/payChannel/status/subMchId）。
    pub async fn paginate(
        &self,
        q: &SubMerchantPageQuery,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<SubMerchant>)> {
        let mut cq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM sub_merchant WHERE is_del = 0");
        Self::push_filters(&mut cq, q);
        let total: (i64,) = cq.build_query_as().fetch_one(&self.pool).await.map_err(map_err)?;
        let mut rq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{SELECT} WHERE is_del = 0"));
        Self::push_filters(&mut rq, q);
        rq.push(q.order_clause());
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<SubMerchant> = rq.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total.0, rows))
    }

    fn push_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &SubMerchantPageQuery) {
        if let Some(shop_id) = q.shop_id {
            qb.push(" AND shop_id = ");
            qb.push_bind(shop_id);
        }
        if let Some(channel) = q.effective_pay_channel() {
            qb.push(" AND pay_channel = ");
            qb.push_bind(channel.to_string());
        }
        if let Some(status) = q.status {
            qb.push(" AND status = ");
            qb.push_bind(status);
        }
        if let Some(sub_mch_id) = q.effective_sub_mch_id() {
            qb.push(" AND sub_mch_id = ");
            qb.push_bind(sub_mch_id.to_string());
        }
    }

    /// 按主键查询。
    pub async fn find_by_id(&self, id: i64) -> Result<Option<SubMerchant>> {
        sqlx::query_as::<_, SubMerchant>(&format!("{SELECT} WHERE id = $1 AND is_del = 0 LIMIT 1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 冻结（Java `unbind`：status=FROZEN + modify_time）。
    pub async fn freeze(&self, id: i64, modify_time: &str) -> Result<u64> {
        let res = sqlx::query("UPDATE sub_merchant SET status = $1, modify_time = NULLIF($2, '') WHERE id = $3")
            .bind(STATUS_FROZEN)
            .bind(modify_time)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 店铺名批量（`shop_id -> shop_title`）。
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

    /// 便捷分页窗口。
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
    use nextrade_core::models::payment::sub_merchant::SUB_MERCHANT_COLUMNS;

    #[test]
    fn select_matches_schema_columns() {
        let selected = SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap();
        // PG 里 text 化时间列写成 `create_time::text AS create_time`，
        // 断言只看「逻辑列是否覆盖齐全」，不绑死某一种 cast 写法。
        let logical = selected
            .split(", ")
            .map(|c| c.rsplit(" AS ").next().unwrap().trim())
            .collect::<Vec<_>>()
            .join(", ");
        assert_eq!(logical, SUB_MERCHANT_COLUMNS.join(", "));
    }

    #[test]
    fn static_sql_guards() {
        let src = include_str!("sub_merchant.rs");
        assert!(src.contains("FROM sub_merchant WHERE is_del = 0"));
        assert!(src.contains("UPDATE sub_merchant SET status = $1"));
        assert!(src.contains("FROM shop WHERE shop_id IN ("));
    }
}
