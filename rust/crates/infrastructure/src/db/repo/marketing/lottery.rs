//! 抽奖（客户端）域仓储（marketing::lottery）。

use std::collections::HashMap;

use sqlx::{FromRow,Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::promotion::{
    LotteryActivity, LotteryPrize, LotteryRecord, LotteryUserStat,
};
use nextrade_core::models::marketing::lottery::{LotteryRecordQuery, PRIZE_TYPE_PRODUCT};
use nextrade_core::{Error, Result};

const ACTIVITY_SELECT: &str = "SELECT activity_id, activity_name, start_time, end_time, cost_points, daily_limit, total_limit, win_limit, decorate_config::text AS decorate_config, grid_config::text AS grid_config, no_win_name, no_win_image, no_win_tips, shop_id, is_delete, create_time, update_time, cost_type FROM lottery_activity";
const PRIZE_SELECT: &str = "SELECT prize_id, activity_id, prize_name, prize_type, prize_value, prize_image, prize_stock, prize_stock_used, create_time, update_time FROM lottery_prize";
const RECORD_SELECT: &str = "SELECT record_id, activity_id, user_id, prize_id, prize_name, prize_type, prize_value, prize_image, cost_points, is_win, status, receive_time, create_time FROM lottery_record";

/// 用户积分/资料行。
#[derive(Debug, Clone, Default, FromRow)]
pub struct LotteryUserRow {
    pub user_id: i32,
    pub points: i32,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar: String,
}

/// 中奖记录 + 活动名。
#[derive(Debug, Clone, Default, FromRow)]
pub struct RecordWithActivity {
    pub record_id: i32,
    pub activity_name: String,
    pub prize_name: String,
    pub prize_type: i32,
    pub prize_image: Option<String>,
    pub prize_value: String,
    pub status: i32,
    pub create_time: i32,
    pub receive_time: Option<i32>,
}

/// 中奖名单行。
#[derive(Debug, Clone, Default, FromRow)]
pub struct WinnerRow {
    pub prize_name: String,
    pub prize_image: Option<String>,
    pub prize_type: i32,
    pub create_time: i32,
    pub avatar: String,
    pub nickname: Option<String>,
    pub username: String,
}

/// 抽奖域仓储。
#[derive(Clone)]
pub struct LotteryRepository {
    pool: PgPool,
}

impl LotteryRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn begin(&self) -> Result<sqlx::Transaction<'static, Postgres>> {
        self.pool.begin().await.map_err(map_err)
    }

    /// 当前进行中的活动。
    pub async fn find_ongoing(&self, now: i64) -> Result<Option<LotteryActivity>> {
        sqlx::query_as::<_, LotteryActivity>(&format!(
            "{ACTIVITY_SELECT} WHERE is_delete = 0 AND start_time <= $1 AND end_time >= $2 ORDER BY create_time DESC LIMIT 1"
        ))
        .bind(now)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn find_activity(&self, activity_id: i32) -> Result<Option<LotteryActivity>> {
        sqlx::query_as::<_, LotteryActivity>(&format!("{ACTIVITY_SELECT} WHERE activity_id = $1 LIMIT 1"))
            .bind(activity_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    pub async fn find_prizes(&self, activity_id: i32) -> Result<Vec<LotteryPrize>> {
        sqlx::query_as::<_, LotteryPrize>(&format!("{PRIZE_SELECT} WHERE activity_id = $1"))
            .bind(activity_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    pub async fn find_prizes_by_ids(&self, ids: &[i32]) -> Result<HashMap<i32, LotteryPrize>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!("{PRIZE_SELECT} WHERE prize_id IN ("));
        {
            let mut sep = qb.separated(", ");
            for id in ids {
                sep.push_bind(*id);
            }
        }
        qb.push(")");
        let rows: Vec<LotteryPrize> = qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().map(|p| (p.prize_id, p)).collect())
    }

    pub async fn find_stat(&self, activity_id: i32, user_id: i32) -> Result<Option<LotteryUserStat>> {
        sqlx::query_as::<_, LotteryUserStat>(
            "SELECT stat_id, activity_id, user_id, total_count, win_count, today_count, today_date, last_lottery_time, create_time, update_time FROM lottery_user_stat WHERE activity_id = $1 AND user_id = $2 LIMIT 1",
        )
        .bind(activity_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn find_user(&self, user_id: i32) -> Result<Option<LotteryUserRow>> {
        sqlx::query_as::<_, LotteryUserRow>(
            "SELECT user_id, points, username, nickname, avatar FROM \"user\" WHERE user_id = $1 LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 事务内扣积分 + 积分流水（Java `subPoints`，change_type=2）。
    pub async fn deduct_points_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        user_id: i32,
        points: i32,
        desc: &str,
        relation_id: i32,
        now: i32,
    ) -> Result<u64> {
        let res = sqlx::query("UPDATE \"user\" SET points = points - $1 WHERE user_id = $2 AND points >= $3")
            .bind(points)
            .bind(user_id)
            .bind(points)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?;
        if res.rows_affected() == 0 {
            return Ok(0);
        }
        sqlx::query(
            "INSERT INTO user_points_log (user_id, points, change_time, change_desc, change_type, relation_type, relation_id) VALUES ($1, $2, $3, $4, 2, NULL, $5)",
        )
        .bind(user_id)
        .bind(points)
        .bind(now)
        .bind(desc)
        .bind(relation_id)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(1)
    }

    /// 事务内扣奖品库存（条件更新）。
    pub async fn deduct_prize_stock_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        prize_id: i32,
        now: i32,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE lottery_prize SET prize_stock_used = prize_stock_used + 1, update_time = $1 WHERE prize_id = $2 AND prize_stock_used < prize_stock",
        )
        .bind(now)
        .bind(prize_id)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    pub async fn insert_record_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        record: &LotteryRecord,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO lottery_record (activity_id, user_id, prize_id, prize_name, prize_type, prize_value, prize_image, cost_points, is_win, status, receive_time, create_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING record_id",
        )
        .bind(record.activity_id)
        .bind(record.user_id)
        .bind(record.prize_id)
        .bind(&record.prize_name)
        .bind(record.prize_type)
        .bind(&record.prize_value)
        .bind(&record.prize_image)
        .bind(record.cost_points)
        .bind(record.is_win)
        .bind(record.status)
        .bind(record.receive_time)
        .bind(record.create_time)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 事务内更新用户统计（存在则累加，不存在则插入）。
    pub async fn upsert_stat_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        activity_id: i32,
        user_id: i32,
        is_win: bool,
        today: &str,
        now: i32,
    ) -> Result<()> {
        let existing = self.find_stat(activity_id, user_id).await?;
        match existing {
            Some(stat) => {
                let today_count = if stat.today_date == today { stat.today_count + 1 } else { 1 };
                sqlx::query(
                    "UPDATE lottery_user_stat SET total_count = $1, win_count = $2, today_count = $3, today_date = $4, last_lottery_time = $5, update_time = $6 WHERE stat_id = $7",
                )
                .bind(stat.total_count + 1)
                .bind(stat.win_count + i32::from(is_win))
                .bind(today_count)
                .bind(today)
                .bind(now)
                .bind(now)
                .bind(stat.stat_id)
                .execute(&mut **tx)
                .await
                .map_err(map_err)?;
            }
            None => {
                sqlx::query(
                    "INSERT INTO lottery_user_stat (activity_id, user_id, total_count, win_count, today_count, today_date, last_lottery_time, create_time, update_time) VALUES ($1, $2, 1, $3, 1, $4, $5, $6, $7)",
                )
                .bind(activity_id)
                .bind(user_id)
                .bind(i32::from(is_win))
                .bind(today)
                .bind(now)
                .bind(now)
                .bind(now)
                .execute(&mut **tx)
                .await
                .map_err(map_err)?;
            }
        }
        Ok(())
    }

    /// 我的中奖记录分页（仅 is_win=1）。
    pub async fn paginate_records(
        &self,
        user_id: i32,
        q: &LotteryRecordQuery,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<RecordWithActivity>)> {
        let mut cq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM lottery_record WHERE user_id = ");
        cq.push_bind(user_id);
        cq.push(" AND is_win = 1");
        if let Some(status) = q.status {
            cq.push(" AND status = ");
            cq.push_bind(status);
        }
        let total: (i64,) = cq.build_query_as().fetch_one(&self.pool).await.map_err(map_err)?;

        let mut rq: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT r.record_id, a.activity_name, r.prize_name, r.prize_type, r.prize_image, r.prize_value, r.status, r.create_time, r.receive_time FROM lottery_record r LEFT JOIN lottery_activity a ON a.activity_id = r.activity_id WHERE r.user_id = ",
        );
        rq.push_bind(user_id);
        rq.push(" AND r.is_win = 1");
        if let Some(status) = q.status {
            rq.push(" AND r.status = ");
            rq.push_bind(status);
        }
        rq.push(" ORDER BY r.create_time DESC");
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<RecordWithActivity> =
            rq.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total.0, rows))
    }

    pub async fn find_record(&self, record_id: i32) -> Result<Option<LotteryRecord>> {
        sqlx::query_as::<_, LotteryRecord>(&format!("{RECORD_SELECT} WHERE record_id = $1 LIMIT 1"))
            .bind(record_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    pub async fn receive_record(&self, record_id: i32, now: i32) -> Result<u64> {
        let res = sqlx::query("UPDATE lottery_record SET status = 1, receive_time = $1 WHERE record_id = $2")
            .bind(now)
            .bind(record_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 中奖名单。
    pub async fn winners(&self, activity_id: i32) -> Result<Vec<WinnerRow>> {
        sqlx::query_as::<_, WinnerRow>(
            "SELECT r.prize_name, r.prize_image, r.prize_type, r.create_time, u.avatar, u.nickname, u.username FROM lottery_record r LEFT JOIN \"user\" u ON u.user_id = r.user_id WHERE r.activity_id = $1 AND r.is_win = 1 ORDER BY r.create_time DESC",
        )
        .bind(activity_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 商品奖品（type=2）标记（供服务层文档/未来扩展，避免未使用常量告警）。
    #[must_use]
    pub fn product_prize_type() -> i32 {
        PRIZE_TYPE_PRODUCT
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
        assert_send::<super::LotteryRepository>();
    }
}
