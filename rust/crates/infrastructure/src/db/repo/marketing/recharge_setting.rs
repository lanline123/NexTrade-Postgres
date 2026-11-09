//! 充值余额配置域仓储（marketing::recharge_setting）。
//! 表 `recharge_setting`（5 列，主键 recharge_id）。

use sqlx::{Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::promotion::RechargeSetting;
use nextrade_core::models::marketing::recharge_setting::{
    limit_offset, RechargeSettingListQuery,
};
use nextrade_core::{Error, Result};

const SELECT: &str = "SELECT recharge_id, money, discount_money, sort_order, is_show FROM recharge_setting";

/// 充值余额配置域仓储。
#[derive(Clone)]
pub struct RechargeSettingRepository {
    pool: PgPool,
}

impl RechargeSettingRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn paginate(
        &self,
        q: &RechargeSettingListQuery,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<RechargeSetting>)> {
        let mut cq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM recharge_setting WHERE 1 = 1");
        Self::push_filters(&mut cq, q);
        let total: (i64,) = cq.build_query_as().fetch_one(&self.pool).await.map_err(map_err)?;
        let mut rq: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!("{SELECT} WHERE 1 = 1"));
        Self::push_filters(&mut rq, q);
        rq.push(q.order_clause());
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<RechargeSetting> = rq.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total.0, rows))
    }

    fn push_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &RechargeSettingListQuery) {
        if let Some(is_show) = q.effective_is_show() {
            qb.push(" AND is_show = ");
            qb.push_bind(is_show);
        }
    }

    pub async fn find_by_id(&self, id: i32) -> Result<Option<RechargeSetting>> {
        sqlx::query_as::<_, RechargeSetting>(&format!("{SELECT} WHERE recharge_id = $1 LIMIT 1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    pub async fn insert(&self, entity: &RechargeSetting) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO recharge_setting (money, discount_money, sort_order, is_show) VALUES ($1, $2, $3, $4) RETURNING recharge_id",
        )
        .bind(entity.money)
        .bind(entity.discount_money)
        .bind(entity.sort_order)
        .bind(entity.is_show)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    pub async fn update(&self, entity: &RechargeSetting) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE recharge_setting SET money = $1, discount_money = $2, sort_order = $3, is_show = $4 WHERE recharge_id = $5",
        )
        .bind(entity.money)
        .bind(entity.discount_money)
        .bind(entity.sort_order)
        .bind(entity.is_show)
        .bind(entity.recharge_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    pub async fn delete(&self, id: i32) -> Result<bool> {
        let res = sqlx::query("DELETE FROM recharge_setting WHERE recharge_id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn delete_batch(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("DELETE FROM recharge_setting WHERE recharge_id IN (");
        {
            let mut sep = qb.separated(", ");
            for id in ids {
                sep.push_bind(*id);
            }
        }
        qb.push(")");
        let res = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    pub async fn update_field(&self, id: i32, column: &str, val: &str) -> Result<u64> {
        debug_assert!(
            nextrade_core::models::marketing::recharge_setting::UPDATE_FIELDS.contains(&column)
        );
        let sql = format!("UPDATE recharge_setting SET {column} = $1 WHERE recharge_id = $2");
        let res = sqlx::query(&sql).bind(val).bind(id).execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
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
    use nextrade_core::models::marketing::recharge_setting::COLUMNS;

    #[test]
    fn select_matches_columns() {
        let selected = SELECT.trim_start_matches("SELECT ").split(" FROM ").next().unwrap();
        assert_eq!(selected, COLUMNS.join(", "));
    }
}
