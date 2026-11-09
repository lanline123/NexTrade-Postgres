//! 拼团活动记录（管理端）域仓储（marketing::groupon_promotion_record）。

use std::collections::HashMap;

use sqlx::{FromRow,Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::promotion::GrouponPromotionRecord;
use nextrade_core::models::marketing::groupon_promotion_record::{OpenJoinListQuery, OrderListQuery};
use nextrade_core::{Error, Result};

const SELECT: &str = "SELECT groupon_promotion_record_id, groupon_promotion_id, groupon_promotion_item_id, product_id, product_name, join_type, join_status, avatar, username, user_id, order_id, order_sn, is_virtual, paid_amount, open_group_record_id, open_group_time, group_captain_name, join_expiration_time, min_group_num, shop_id, add_time FROM groupon_promotion_record";

/// 订单记录行（`orderList` SQL）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct OrderRecordRow {
    pub groupon_promotion_record_id: i64,
    pub groupon_promotion_id: i64,
    pub groupon_promotion_item_id: i64,
    pub product_id: i64,
    pub product_name: String,
    pub join_type: i32,
    pub join_status: i32,
    pub user_id: i64,
    pub order_id: i64,
    pub order_sn: String,
    pub is_virtual: i32,
    pub paid_amount: rust_decimal::Decimal,
    pub open_group_record_id: i64,
    pub open_group_time: i64,
    pub group_captain_name: String,
    pub join_expiration_time: i64,
    pub min_group_num: i32,
    pub shop_id: i32,
    pub add_time: i64,
    pub username: Option<String>,
    pub order_status: Option<i32>,
}

/// 用户行。
#[derive(Debug, Clone, Default, FromRow)]
pub struct GrouponRecordUserRow {
    pub user_id: i32,
    pub username: String,
    pub avatar: String,
}

/// 拼团活动记录（管理端）域仓储。
#[derive(Clone)]
pub struct GrouponPromotionRecordRepository {
    pool: PgPool,
}

impl GrouponPromotionRecordRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn begin(&self) -> Result<sqlx::Transaction<'static, Postgres>> {
        self.pool.begin().await.map_err(map_err)
    }

    /// 开团记录（Java `openJoinList`）。
    pub async fn open_join_list(
        &self,
        q: &OpenJoinListQuery,
        shop_id: Option<i32>,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<GrouponPromotionRecord>)> {
        let mut cq: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT COUNT(*) FROM groupon_promotion_record WHERE join_type = 1",
        );
        Self::push_open_filters(&mut cq, q, shop_id);
        let total: (i64,) = cq.build_query_as().fetch_one(&self.pool).await.map_err(map_err)?;
        let mut rq: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!(
            "{SELECT} WHERE join_type = 1"
        ));
        Self::push_open_filters(&mut rq, q, shop_id);
        rq.push(" ORDER BY open_group_time DESC");
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<GrouponPromotionRecord> =
            rq.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total.0, rows))
    }

    fn push_open_filters(
        qb: &mut QueryBuilder<'_, Postgres>,
        q: &OpenJoinListQuery,
        shop_id: Option<i32>,
    ) {
        // 多租户：店铺账号强制按 shop_id 过滤（i32 内部值）
        if let Some(shop_id) = shop_id {
            qb.push(format!(" AND shop_id = {shop_id}"));
        }
        if let Some(id) = q.groupon_promotion_id {
            qb.push(" AND groupon_promotion_id = ");
            qb.push_bind(id);
        }
        if let Some(status) = q.join_status {
            qb.push(" AND join_status = ");
            qb.push_bind(status);
        }
        if let Some(name) = q.product_name.as_deref().filter(|s| !s.is_empty()) {
            qb.push(" AND product_name LIKE ");
            qb.push_bind(format!("%{name}%"));
        }
    }

    /// 订单记录（Java mapper `orderList`）。
    pub async fn order_list(
        &self,
        q: &OrderListQuery,
        shop_id: Option<i32>,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<OrderRecordRow>)> {
        let mut cq: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT COUNT(*) FROM groupon_promotion_record gpr LEFT JOIN \"user\" u ON gpr.user_id = u.user_id LEFT JOIN \"order\" o ON gpr.order_id = o.order_id WHERE o.order_id != 0",
        );
        Self::push_order_filters(&mut cq, q, shop_id);
        let total: (i64,) = cq.build_query_as().fetch_one(&self.pool).await.map_err(map_err)?;
        let mut rq: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT gpr.groupon_promotion_record_id, gpr.groupon_promotion_id, gpr.groupon_promotion_item_id, gpr.product_id, gpr.product_name, gpr.join_type, gpr.join_status, gpr.user_id, gpr.order_id, gpr.order_sn, gpr.is_virtual, gpr.paid_amount, gpr.open_group_record_id, gpr.open_group_time, gpr.group_captain_name, gpr.join_expiration_time, gpr.min_group_num, gpr.shop_id, gpr.add_time, u.username, o.order_status FROM groupon_promotion_record gpr LEFT JOIN \"user\" u ON gpr.user_id = u.user_id LEFT JOIN \"order\" o ON gpr.order_id = o.order_id WHERE o.order_id != 0",
        );
        Self::push_order_filters(&mut rq, q, shop_id);
        rq.push(" ORDER BY gpr.add_time DESC");
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<OrderRecordRow> =
            rq.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total.0, rows))
    }

    fn push_order_filters(
        qb: &mut QueryBuilder<'_, Postgres>,
        q: &OrderListQuery,
        shop_id: Option<i32>,
    ) {
        // 多租户：店铺账号强制按 shop_id 过滤（i32 内部值）
        if let Some(shop_id) = shop_id {
            qb.push(format!(" AND gpr.shop_id = {shop_id}"));
        }
        if let Some(id) = q.groupon_promotion_id {
            qb.push(" AND gpr.groupon_promotion_id = ");
            qb.push_bind(id);
        }
        if let Some(keyword) = q.keyword.as_deref().filter(|s| !s.is_empty()) {
            let like = format!("%{keyword}%");
            qb.push(" AND (gpr.product_name LIKE ");
            qb.push_bind(like.clone());
            qb.push(" OR u.username LIKE ");
            qb.push_bind(like.clone());
            qb.push(" OR o.order_sn LIKE ");
            qb.push_bind(like);
            qb.push(")");
        }
    }

    /// 事务内写入拼团记录（开团/参团通用），返回自增 ID。
    ///
    /// # Errors
    /// 数据库错误。
    pub async fn insert_record_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        r: &GrouponPromotionRecord,
    ) -> Result<i64> {
        let result: i64 = sqlx::query_scalar(
            "INSERT INTO groupon_promotion_record
             (groupon_promotion_id, groupon_promotion_item_id, product_id, product_name,
              join_type, join_status, avatar, username, user_id, order_id, order_sn,
              is_virtual, paid_amount, open_group_record_id, open_group_time,
              group_captain_name, join_expiration_time, min_group_num, shop_id, add_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20) RETURNING groupon_promotion_record_id",
        )
        .bind(r.groupon_promotion_id)
        .bind(r.groupon_promotion_item_id)
        .bind(r.product_id)
        .bind(&r.product_name)
        .bind(r.join_type)
        .bind(r.join_status)
        .bind(&r.avatar)
        .bind(&r.username)
        .bind(r.user_id)
        .bind(r.order_id)
        .bind(&r.order_sn)
        .bind(r.is_virtual)
        .bind(r.paid_amount)
        .bind(r.open_group_record_id)
        .bind(r.open_group_time)
        .bind(&r.group_captain_name)
        .bind(r.join_expiration_time)
        .bind(r.min_group_num)
        .bind(r.shop_id)
        .bind(r.add_time)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(result)
    }

    /// 事务内回填开团记录的 `open_group_record_id`（Java `save` 后 `updateById`）。
    ///
    /// # Errors
    /// 数据库错误。
    pub async fn set_open_group_id_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        record_id: i64,
        open_group_id: i64,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE groupon_promotion_record SET open_group_record_id = $1              WHERE groupon_promotion_record_id = $2",
        )
        .bind(open_group_id)
        .bind(record_id)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 事务内统计某开团记录的参团人数（含开团记录本身）。
    ///
    /// # Errors
    /// 数据库错误。
    pub async fn count_members_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        open_group_id: i64,
    ) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM groupon_promotion_record WHERE open_group_record_id = $1",
        )
        .bind(open_group_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 到期待成团的开团记录（拼团过期定时任务）。
    ///
    /// # Errors
    /// 数据库错误。
    pub async fn find_expired_waiting(
        &self,
        now: i64,
        limit: u64,
    ) -> Result<Vec<GrouponPromotionRecord>> {
        sqlx::query_as::<_, GrouponPromotionRecord>(&format!(
            "{SELECT} WHERE join_type = 1 AND join_status = 1              AND join_expiration_time > 0 AND join_expiration_time <= $1              ORDER BY groupon_promotion_record_id ASC LIMIT $2"
        ))
        .bind(now)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 拼团过期：把整团待成团成员置为失败（3），返回影响行数。
    ///
    /// # Errors
    /// 数据库错误。
    pub async fn mark_failed_by_open(&self, open_group_id: i64) -> Result<u64> {
        let affected = sqlx::query(
            "UPDATE groupon_promotion_record SET join_status = 3              WHERE open_group_record_id = $1 AND join_status = 1",
        )
        .bind(open_group_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected)
    }

    /// 订单取消：把该订单待成团记录置为已取消（4）。
    pub async fn cancel_waiting_by_order(&self, order_id: i64) -> Result<u64> {
        Ok(sqlx::query(
            "UPDATE groupon_promotion_record SET join_status = 4 \
             WHERE order_id = ? AND join_status = 1",
        )
        .bind(order_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected())
    }

    /// 团员（Java `groupMemberList`：`open_group_record_id = id`）。
    pub async fn members(&self, open_record_id: i64) -> Result<Vec<GrouponPromotionRecord>> {
        sqlx::query_as::<_, GrouponPromotionRecord>(&format!(
            "{SELECT} WHERE open_group_record_id = ?"
        ))
        .bind(open_record_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<GrouponPromotionRecord>> {
        sqlx::query_as::<_, GrouponPromotionRecord>(&format!(
            "{SELECT} WHERE groupon_promotion_record_id = ? LIMIT 1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn count_members(&self, open_record_id: i64) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM groupon_promotion_record WHERE open_group_record_id = ?",
        )
        .bind(open_record_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 开团记录下的全部成员（含开团记录本身）。
    pub async fn find_members(
        &self,
        open_group_id: i64,
    ) -> Result<Vec<GrouponPromotionRecord>> {
        sqlx::query_as::<_, GrouponPromotionRecord>(&format!(
            "{SELECT} WHERE open_group_record_id = ? ORDER BY groupon_promotion_record_id ASC"
        ))
        .bind(open_group_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 成团：开团记录 + 同团成员状态置 2（`join_status` 成功）。
    pub async fn mark_success_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        record_id: i64,
    ) -> Result<u64> {
        let self_res = sqlx::query("UPDATE groupon_promotion_record SET join_status = 2 WHERE groupon_promotion_record_id = ?")
            .bind(record_id)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?;
        sqlx::query("UPDATE groupon_promotion_record SET join_status = 2 WHERE open_group_record_id = ?")
            .bind(record_id)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?;
        Ok(self_res.rows_affected())
    }

    /// 到期自动成团：标记成功并补齐虚拟团员（单事务）。
    pub async fn auto_complete_group(
        &self,
        base: &GrouponPromotionRecord,
        fill_count: usize,
        now: i64,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        self.mark_success_in_tx(&mut tx, base.groupon_promotion_record_id)
            .await?;
        for index in 0..fill_count {
            self.insert_virtual_in_tx(&mut tx, base, index, now).await?;
        }
        tx.commit().await.map_err(map_err)?;
        Ok(())
    }

    /// 插入虚拟团员（Java \"getVirtualUser\" 依赖配置；本实现用「虚拟用户N」）。
    pub async fn insert_virtual_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        base: &GrouponPromotionRecord,
        index: usize,
        now: i64,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO groupon_promotion_record (groupon_promotion_id, groupon_promotion_item_id, product_id, product_name, join_type, join_status, avatar, username, user_id, order_id, order_sn, is_virtual, paid_amount, open_group_record_id, open_group_time, group_captain_name, join_expiration_time, min_group_num, shop_id, add_time) VALUES (?, ?, ?, ?, 2, 2, '', ?, 0, 0, '0', 1, 0.00, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(base.groupon_promotion_id)
        .bind(base.groupon_promotion_item_id)
        .bind(base.product_id)
        .bind(&base.product_name)
        .bind(format!("虚拟用户{}", index + 1))
        .bind(base.groupon_promotion_record_id)
        .bind(base.open_group_time)
        .bind(&base.group_captain_name)
        .bind(base.join_expiration_time)
        .bind(base.min_group_num)
        .bind(base.shop_id)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn find_users(&self, ids: &[i64]) -> Result<HashMap<i64, GrouponRecordUserRow>> {
        let ids: Vec<i32> = ids
            .iter()
            .filter(|id| **id > 0 && **id <= i64::from(i32::MAX))
            .map(|id| *id as i32)
            .collect();
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT user_id, username, avatar FROM \"user\" WHERE user_id IN (");
        {
            let mut sep = qb.separated(", ");
            for id in &ids {
                sep.push_bind(*id);
            }
        }
        qb.push(")");
        let rows: Vec<GrouponRecordUserRow> =
            qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().map(|r| (i64::from(r.user_id), r)).collect())
    }

    pub async fn find_shop_title(&self, shop_id: i32) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT shop_title FROM shop WHERE shop_id = $1 LIMIT 1")
                .bind(shop_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }

    #[must_use]
    pub fn page_window(page: Option<i64>, size: Option<i64>) -> (i64, i64) {
        nextrade_core::models::marketing::groupon_promotion_record::limit_offset(page, size)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn repository_is_send_sync() {
        fn assert_send<T: Send>() {}
        assert_send::<super::GrouponPromotionRecordRepository>();
    }
}
