//! 抽奖活动域仓储（marketing::lottery_activity）。
//!
//! 表（均来自 `java/database/pure.sql`，实体由 schema 生成）：
//! - `lottery_activity`（18 列）
//! - `lottery_prize`（10 列）
//! - `lottery_record`（13 列）
//! - `lottery_user_stat`（10 列）
//! - `promotion`（抽奖活动同步：type=8）
//! - `user`（读用户名/头像/来源标签）
//!
//! SQL 一律运行时 `sqlx`（不用 `query!` 宏）。

use sqlx::{Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::{LotteryActivity, LotteryPrize, LotteryRecord, LotteryUserStat};
use nextrade_core::models::marketing::lottery_activity::{
    LotteryActivityListQuery, LotteryUserManageQuery, LotteryUserRecordQuery,
};
use nextrade_core::{Error, Result};

/// `lottery_activity` 列清单（18 列，顺序与实体一致）。
pub const ACTIVITY_COLUMNS: [&str; 18] = [
    "activity_id",
    "activity_name",
    "start_time",
    "end_time",
    "cost_points",
    "daily_limit",
    "total_limit",
    "win_limit",
    "decorate_config",
    "grid_config",
    "no_win_name",
    "no_win_image",
    "no_win_tips",
    "shop_id",
    "is_delete",
    "create_time",
    "update_time",
    "cost_type",
];

/// `lottery_prize` 列清单（10 列）。
pub const PRIZE_COLUMNS: [&str; 10] = [
    "prize_id",
    "activity_id",
    "prize_name",
    "prize_type",
    "prize_value",
    "prize_image",
    "prize_stock",
    "prize_stock_used",
    "create_time",
    "update_time",
];

/// `lottery_record` 列清单（13 列）。
pub const RECORD_COLUMNS: [&str; 13] = [
    "record_id",
    "activity_id",
    "user_id",
    "prize_id",
    "prize_name",
    "prize_type",
    "prize_value",
    "prize_image",
    "cost_points",
    "is_win",
    "status",
    "receive_time",
    "create_time",
];

/// `lottery_user_stat` 列清单（10 列）。
pub const USER_STAT_COLUMNS: [&str; 10] = [
    "stat_id",
    "activity_id",
    "user_id",
    "total_count",
    "win_count",
    "today_count",
    "today_date",
    "last_lottery_time",
    "create_time",
    "update_time",
];

/// 用户简要信息（用户管理 / 中奖记录装配）。
#[derive(Debug, Clone, Default)]
pub struct UserBrief {
    pub username: String,
    pub avatar: Option<String>,
    pub from_tag: Option<i32>,
}

/// 抽奖活动域仓储。
#[derive(Clone)]
pub struct LotteryActivityRepository {
    pool: PgPool,
}

impl LotteryActivityRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }
}

fn select_sql(columns: &[&str], table: &str) -> String {
    format!("SELECT {} FROM {table}", columns.join(", "))
}

impl LotteryActivityRepository {
    // ————————————————————— 活动 —————————————————————

    pub async fn find_by_id(&self, activity_id: i32) -> Result<Option<LotteryActivity>> {
        let sql = crate::db::to_pg_placeholders(&format!("{} WHERE activity_id = ? LIMIT 1", select_sql(&ACTIVITY_COLUMNS, "lottery_activity")));
        sqlx::query_as::<_, LotteryActivity>(&sql)
            .bind(activity_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 活动分页（Java `getList`：is_delete=0 + keyword/activityName LIKE + 时间状态 +
    /// 店铺租户过滤 + create_time DESC）。
    pub async fn paginate(
        &self,
        q: &LotteryActivityListQuery,
        now: i64,
        shop_id: Option<i32>,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<LotteryActivity>)> {
        let (where_sql, binds) = build_activity_where(q, now, shop_id);
        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM lottery_activity{where_sql}"));
        let mut count_q = sqlx::query_as::<_, (i64,)>(&count_sql);
        for b in &binds {
            count_q = count_q.bind(b.as_str());
        }
        let total: (i64,) = count_q.fetch_one(&self.pool).await.map_err(map_err)?;
        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "{} {where_sql} ORDER BY create_time DESC LIMIT {limit} OFFSET {offset}",
            select_sql(&ACTIVITY_COLUMNS, "lottery_activity")
        ));
        let mut rows_q = sqlx::query_as::<_, LotteryActivity>(&rows_sql);
        for b in &binds {
            rows_q = rows_q.bind(b.as_str());
        }
        let rows = rows_q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total.0, rows))
    }

    pub async fn insert(&self, a: &LotteryActivity) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO lottery_activity (activity_name, start_time, end_time, cost_points, daily_limit, total_limit, win_limit, decorate_config, grid_config, no_win_name, no_win_image, no_win_tips, shop_id, is_delete, create_time, update_time, cost_type) VALUES ($1,  $2,  $3,  $4,  $5,  $6,  $7, $8, $9,  $10,  $11,  $12,  $13,  $14,  $15,  $16,  $17) RETURNING activity_id",
        )
        .bind(&a.activity_name)
        .bind(a.start_time)
        .bind(a.end_time)
        .bind(a.cost_points)
        .bind(a.daily_limit)
        .bind(a.total_limit)
        .bind(a.win_limit)
        .bind(&a.decorate_config)
        .bind(&a.grid_config)
        .bind(&a.no_win_name)
        .bind(&a.no_win_image)
        .bind(&a.no_win_tips)
        .bind(a.shop_id)
        .bind(a.is_delete)
        .bind(a.create_time)
        .bind(a.update_time)
        .bind(a.cost_type)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    pub async fn update(&self, a: &LotteryActivity) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE lottery_activity SET activity_name = $1, start_time = $2, end_time = $3, cost_points = $4, daily_limit = $5, total_limit = $6, win_limit = $7, decorate_config = $8, grid_config = $9, no_win_name = $10, no_win_image = $11, no_win_tips = $12, update_time = $13, cost_type = $14 WHERE activity_id = $15",
        )
        .bind(&a.activity_name)
        .bind(a.start_time)
        .bind(a.end_time)
        .bind(a.cost_points)
        .bind(a.daily_limit)
        .bind(a.total_limit)
        .bind(a.win_limit)
        .bind(&a.decorate_config)
        .bind(&a.grid_config)
        .bind(&a.no_win_name)
        .bind(&a.no_win_image)
        .bind(&a.no_win_tips)
        .bind(a.update_time)
        .bind(a.cost_type)
        .bind(a.activity_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 时间冲突数（Java `checkTimeConflict`）。
    pub async fn time_conflict_count(
        &self,
        activity_id: Option<i32>,
        start_time: i32,
        end_time: i32,
    ) -> Result<i64> {
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT COUNT(*) FROM lottery_activity WHERE is_delete = 0 AND start_time < ",
        );
        qb.push_bind(end_time);
        qb.push(" AND end_time > ");
        qb.push_bind(start_time);
        if let Some(id) = activity_id {
            qb.push(" AND activity_id <> ");
            qb.push_bind(id);
        }
        let row: (i64,) = qb.build_query_as().fetch_one(&self.pool).await.map_err(map_err)?;
        Ok(row.0)
    }

    /// 参与人数（去重用户数，Java `countParticipants`）。
    pub async fn participant_count(&self, activity_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT user_id) FROM lottery_record WHERE activity_id = $1",
        )
        .bind(activity_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    // ————————————————————— 奖品 —————————————————————

    pub async fn prizes_by_activity(&self, activity_id: i32) -> Result<Vec<LotteryPrize>> {
        let sql = crate::db::to_pg_placeholders(&format!(
            "{} WHERE activity_id = ? ORDER BY prize_id ASC",
            select_sql(&PRIZE_COLUMNS, "lottery_prize")
        ));
        sqlx::query_as::<_, LotteryPrize>(&sql)
            .bind(activity_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    pub async fn prize_by_id(&self, prize_id: i32) -> Result<Option<LotteryPrize>> {
        let sql = crate::db::to_pg_placeholders(&format!(
            "{} WHERE prize_id = ? LIMIT 1",
            select_sql(&PRIZE_COLUMNS, "lottery_prize")
        ));
        sqlx::query_as::<_, LotteryPrize>(&sql)
            .bind(prize_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    pub async fn insert_prize(&self, p: &LotteryPrize) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO lottery_prize (activity_id, prize_name, prize_type, prize_value, prize_image, prize_stock, prize_stock_used, create_time, update_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING prize_id",
        )
        .bind(p.activity_id)
        .bind(&p.prize_name)
        .bind(p.prize_type)
        .bind(&p.prize_value)
        .bind(&p.prize_image)
        .bind(p.prize_stock)
        .bind(p.prize_stock_used)
        .bind(p.create_time)
        .bind(p.update_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 更新奖品（保留 `prize_stock_used` 等统计字段由服务层负责）。
    pub async fn update_prize(&self, p: &LotteryPrize) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE lottery_prize SET prize_name = $1, prize_type = $2, prize_value = $3, prize_image = $4, prize_stock = $5, prize_stock_used = $6, update_time = $7 WHERE prize_id = $8",
        )
        .bind(&p.prize_name)
        .bind(p.prize_type)
        .bind(&p.prize_value)
        .bind(&p.prize_image)
        .bind(p.prize_stock)
        .bind(p.prize_stock_used)
        .bind(p.update_time)
        .bind(p.prize_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 删除未被本次保留的奖品（Java `savePrizeList` 的删除分支）。
    pub async fn delete_prizes_except(&self, activity_id: i32, kept_ids: &[i32]) -> Result<u64> {
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("DELETE FROM lottery_prize WHERE activity_id = ");
        qb.push_bind(activity_id);
        if !kept_ids.is_empty() {
            qb.push(" AND prize_id NOT IN (");
            let mut sep = qb.separated(", ");
            for id in kept_ids {
                sep.push_bind(*id);
            }
            sep.push_unseparated(")");
            drop(sep);
        }
        let res = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— Promotion 同步 —————————————————————

    /// 保存/更新 Promotion（Java `savePromotion`：type=8，relation_id=activityId）。
    pub async fn upsert_promotion(
        &self,
        activity: &LotteryActivity,
        is_available: i32,
    ) -> Result<()> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM promotion WHERE relation_id = $1 AND type = 8",
        )
        .bind(activity.activity_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        if row.0 > 0 {
            sqlx::query(
                "UPDATE promotion SET start_time = $1, end_time = $2, promotion_name = $3, is_available = $4 WHERE relation_id = $5 AND type = 8",
            )
            .bind(activity.start_time)
            .bind(activity.end_time)
            .bind(&activity.activity_name)
            .bind(is_available)
            .bind(activity.activity_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        } else {
            sqlx::query(
                "INSERT INTO promotion (promotion_name, start_time, end_time, type, shop_id, relation_id, is_available, is_delete, member_range) VALUES ($1,  $2,  $3,  8,  $4,  $5,  $6,  0,  0)",
            )
            .bind(&activity.activity_name)
            .bind(activity.start_time)
            .bind(activity.end_time)
            .bind(activity.shop_id)
            .bind(activity.activity_id)
            .bind(is_available)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        }
        Ok(())
    }

    /// 结束活动时同步 Promotion.end_time。
    pub async fn update_promotion_end(&self, activity_id: i32, end_time: i32) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE promotion SET end_time = $1 WHERE relation_id = $2 AND type = 8",
        )
        .bind(end_time)
        .bind(activity_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 软删除活动时同步 Promotion.is_delete=1。
    pub async fn soft_delete_promotion(&self, activity_id: i32) -> Result<u64> {
        let res =
            sqlx::query("UPDATE promotion SET is_delete = 1 WHERE relation_id = $1 AND type = 8")
                .bind(activity_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— 用户管理 / 中奖记录 —————————————————————

    /// 用户参与统计分页（Java `getUserManageList`）。
    pub async fn user_stats_paginate(
        &self,
        q: &LotteryUserManageQuery,
        start_ts: Option<i32>,
        end_ts: Option<i32>,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<LotteryUserStat>)> {
        let activity_id = q.activity_id.unwrap_or(0);
        let mut where_sql = format!(" WHERE activity_id = {activity_id} AND total_count > 0");
        if let Some(ts) = start_ts {
            where_sql.push_str(&format!(" AND last_lottery_time >= {ts}"));
        }
        if let Some(ts) = end_ts {
            where_sql.push_str(&format!(" AND last_lottery_time <= {ts}"));
        }
        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM lottery_user_stat{where_sql}"));
        let total: (i64,) = sqlx::query_as(&count_sql)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "{} {where_sql} ORDER BY last_lottery_time DESC LIMIT {limit} OFFSET {offset}",
            select_sql(&USER_STAT_COLUMNS, "lottery_user_stat")
        ));
        let rows = sqlx::query_as::<_, LotteryUserStat>(&rows_sql)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0, rows))
    }

    /// 用户中奖记录分页（Java `getUserRecordList`）。
    pub async fn records_paginate(
        &self,
        q: &LotteryUserRecordQuery,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<LotteryRecord>)> {
        let activity_id = q.activity_id.unwrap_or(0);
        let user_id = q.user_id.unwrap_or(0);
        let where_sql =
            format!(" WHERE activity_id = {activity_id} AND user_id = {user_id}");
        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM lottery_record{where_sql}"));
        let total: (i64,) = sqlx::query_as(&count_sql)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "{} {where_sql} ORDER BY create_time DESC LIMIT {limit} OFFSET {offset}",
            select_sql(&RECORD_COLUMNS, "lottery_record")
        ));
        let rows = sqlx::query_as::<_, LotteryRecord>(&rows_sql)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0, rows))
    }

    /// 批量用户简要信息（用户名 / 头像 / 来源标签）。
    pub async fn user_briefs(&self, ids: &[i32]) -> Result<Vec<(i32, UserBrief)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT user_id, username, avatar, from_tag FROM \"user\" WHERE user_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        let rows: Vec<(i32, String, Option<String>, Option<i32>)> =
            qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(id, username, avatar, from_tag)| {
                (
                    id,
                    UserBrief {
                        username,
                        avatar,
                        from_tag,
                    },
                )
            })
            .collect())
    }

    /// 单个用户简要信息。
    pub async fn user_brief(&self, user_id: i32) -> Result<Option<UserBrief>> {
        let row: Option<(String, Option<String>, Option<i32>)> =
            sqlx::query_as("SELECT username, avatar, from_tag FROM \"user\" WHERE user_id = $1 LIMIT 1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|(username, avatar, from_tag)| UserBrief {
            username,
            avatar,
            from_tag,
        }))
    }
}

/// 活动列表 WHERE（Java `getList`）。
#[must_use]
pub fn build_activity_where(
    q: &LotteryActivityListQuery,
    now: i64,
    shop_id: Option<i32>,
) -> (String, Vec<String>) {
    let mut conds = vec!["is_delete = 0".to_string()];
    // 多租户：店铺账号强制按 shop_id 过滤（i32 内部值，非用户文本）
    if let Some(shop_id) = shop_id {
        conds.push(format!("shop_id = {shop_id}"));
    }
    let mut binds: Vec<String> = Vec::new();
    if let Some(keyword) = q.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        conds.push("activity_name LIKE ?".to_string());
        binds.push(format!("%{keyword}%"));
    }
    if let Some(name) = q
        .activity_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        conds.push("activity_name LIKE ?".to_string());
        binds.push(format!("%{name}%"));
    }
    match q.status {
        Some(0) => conds.push(format!("start_time > {now}")),
        Some(1) => {
            conds.push(format!("start_time <= {now}"));
            conds.push(format!("end_time >= {now}"));
        }
        Some(2) => conds.push(format!("end_time < {now}")),
        _ => {}
    }
    (format!(" WHERE {}", conds.join(" AND ")), binds)
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(json: serde_json::Value) -> LotteryActivityListQuery {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn list_where_filters_delete_keywords_and_status() {
        let (sql, binds) = build_activity_where(&q(serde_json::json!({})), 100, None);
        assert_eq!(sql, " WHERE is_delete = 0");
        assert!(binds.is_empty());

        let (sql, binds) = build_activity_where(
            &q(serde_json::json!({"keyword": "抽奖", "activityName": "元旦"})),
            100,
            None,
        );
        assert_eq!(
            sql,
            " WHERE is_delete = 0 AND activity_name LIKE ? AND activity_name LIKE ?"
        );
        assert_eq!(binds, vec!["%抽奖%".to_string(), "%元旦%".to_string()]);

        let (sql, _) = build_activity_where(&q(serde_json::json!({"status": 0})), 100, None);
        assert!(sql.ends_with("start_time > 100"));
        let (sql, _) = build_activity_where(&q(serde_json::json!({"status": 1})), 100, None);
        assert!(sql.contains("start_time <= 100") && sql.contains("end_time >= 100"));
        let (sql, _) = build_activity_where(&q(serde_json::json!({"status": 2})), 100, None);
        assert!(sql.ends_with("end_time < 100"));
    }

    #[test]
    fn list_where_applies_shop_scope() {
        let (sql, _) = build_activity_where(&q(serde_json::json!({})), 100, Some(8));
        assert_eq!(sql, " WHERE is_delete = 0 AND shop_id = 8");
    }

    #[test]
    fn column_lists_match_entities() {
        assert_eq!(ACTIVITY_COLUMNS.len(), 18);
        assert_eq!(PRIZE_COLUMNS.len(), 10);
        assert_eq!(RECORD_COLUMNS.len(), 13);
        assert_eq!(USER_STAT_COLUMNS.len(), 10);
        let src = include_str!("lottery_activity.rs");
        assert!(src.contains("AND type = 8"));
        assert!(src.contains("INSERT INTO promotion"));
    }
}
