//! 会员域仓储：`user_rank` / `member_tag` / `member_tag_group` /
//! `user_points_log`。替代 `UserRankMapper` 等。
//!
//! TODO(域实现者)：对照 `java/nextrade-service` 中对应 ServiceImpl 补全方法。

use sqlx::PgPool;
use nextrade_core::entities::{MemberTag, MemberTagGroup, UserPointsLog, UserRank};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct MemberRepository {
    pool: PgPool,
}

impl MemberRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 全部会员等级（按升级门槛升序）。
    ///
    /// 注意真实列名为 `min_growth_points`（非 `upgrade_amount`）。
    pub async fn find_all_ranks(&self) -> Result<Vec<UserRank>> {
        sqlx::query_as::<_, UserRank>(
            "SELECT * FROM user_rank ORDER BY min_growth_points ASC, rank_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 全部标签组（含其下标签由调用方组装）。
    ///
    /// 注意真实列名为 `sort_order`（非 `sort`），主键为 `id`（非 `group_id`）。
    /// `shop_id`：平台 `0`，店铺强制本店（来自可信 JWT）。
    pub async fn find_all_tag_groups(&self, shop_id: i32) -> Result<Vec<MemberTagGroup>> {
        sqlx::query_as::<_, MemberTagGroup>(
            "SELECT * FROM member_tag_group WHERE shop_id = $1 ORDER BY is_top DESC, id ASC",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 全部会员标签。
    ///
    /// 注意主键为 `id`（非 `tag_id`）。
    pub async fn find_all_tags(&self) -> Result<Vec<MemberTag>> {
        sqlx::query_as::<_, MemberTag>("SELECT * FROM member_tag ORDER BY id ASC")
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 积分流水分页。
    pub async fn paginate_points_logs(
        &self,
        user_id: Option<i32>,
        shop_scope: Option<i32>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<UserPointsLog>)> {
        let offset = (page.page.max(1) - 1) * page.page_size;

        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM user_points_log l WHERE ($1 IS NULL OR l.user_id = $2) AND ($3 IS NULL OR EXISTS (SELECT 1 FROM shop_member m WHERE m.user_id = l.user_id AND m.shop_id = $4 AND m.is_del = 0))",
        )
        .bind(user_id)
        .bind(user_id)
        .bind(shop_scope)
        .bind(shop_scope)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        let rows = sqlx::query_as::<_, UserPointsLog>(
            "SELECT * FROM user_points_log l
             WHERE ($1 IS NULL OR l.user_id = $2) AND ($3 IS NULL OR EXISTS (SELECT 1 FROM shop_member m WHERE m.user_id = l.user_id AND m.shop_id = $4 AND m.is_del = 0))
             ORDER BY l.log_id DESC LIMIT $5 OFFSET $6",
        )
        .bind(user_id)
        .bind(user_id)
        .bind(shop_scope)
        .bind(shop_scope)
        .bind(page.page_size as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok((total.0 as u64, rows))
    }

    /// 原子增加/扣减积分并在事务中写流水记录。
    pub async fn adjust_points_with_log(
        &self,
        user_id: i32,
        delta: i32,
        reason: &str,
    ) -> Result<bool> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;

        let update_sql = if delta >= 0 {
            "UPDATE \"user\" SET points = points + $1 WHERE user_id = $2"
        } else {
            "UPDATE \"user\" SET points = points + $1 WHERE user_id = $2 AND points >= $3"
        };
        let r = if delta >= 0 {
            sqlx::query(update_sql).bind(delta).bind(user_id)
        } else {
            sqlx::query(update_sql).bind(delta).bind(user_id).bind(-delta)
        };
        let res = r.execute(&mut *tx).await.map_err(map_err)?;

        if res.rows_affected() == 0 {
            return Ok(false);
        }

        let change_type = if delta >= 0 { 1 } else { 2 };
        let now_sec = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()) as i32;

        sqlx::query(
            "INSERT INTO user_points_log (user_id, points, change_time, change_desc, change_type, relation_type, relation_id) \
             VALUES ($1, $2, $3, $4, $5, NULL, NULL)"
        )
        .bind(user_id)
        .bind(delta.abs())
        .bind(now_sec)
        .bind(reason)
        .bind(change_type)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;

        tx.commit().await.map_err(map_err)?;
        Ok(true)
    }

    /// 原子增加积分（余额不足扣减时返回 false，防负数并发）。
    pub async fn try_adjust_points(&self, user_id: i32, delta: i32) -> Result<bool> {
        let sql = if delta >= 0 {
            "UPDATE \"user\" SET points = points + $1 WHERE user_id = $2"
        } else {
            "UPDATE \"user\" SET points = points + $1 WHERE user_id = $2 AND points >= $3"
        };
        let r = if delta >= 0 {
            sqlx::query(sql).bind(delta).bind(user_id)
        } else {
            sqlx::query(sql).bind(delta).bind(user_id).bind(-delta)
        };
        let r = r.execute(&self.pool).await.map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("会员数据不存在".into()),
        other => Error::Database(other.to_string()),
    }
}
