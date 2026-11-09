//! 优惠券仓储：`coupon` / `user_coupon`，替代 `CouponMapper` / `UserCouponMapper`。
//!
//! 列名严格对齐 `java/database/pure.sql`：
//! - `coupon`（L958）
//! - `user_coupon`（L6526，主键列名为 `id`，且**无 status 列**）
//!
//! 注意：不使用 `SELECT *`，显式列出列名以保证与实体字段一一对应，
//! 避免表结构演进或列顺序变化导致 `FromRow` 映射错位。

use sqlx::PgPool;
use nextrade_core::entities::{Coupon, UserCoupon};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// `coupon` 表全部列（顺序与 [`Coupon`] 字段一致）。
const COUPON_COLUMNS: &str = "coupon_id, coupon_name, coupon_money, coupon_discount, coupon_desc, \
     coupon_type, send_range, send_range_data, min_order_amount, send_start_date, send_end_date, \
     send_type, use_day, use_start_date, use_end_date, is_show, is_global, is_new_user, \
     enabled_click_get, limit_user_rank, shop_id, is_delete, limit_num, delay_day, send_num, \
     max_order_amount, coupon_unit, reduce_type, add_time";

/// \"user_coupon\" 表全部列（顺序与 [\"UserCoupon\"] 字段一致）。
const USER_COUPON_COLUMNS: &str = "id, coupon_id, coupon_sn, user_id, used_time, order_id, \
     start_date, end_date, source_type";

#[derive(Clone)]
pub struct CouponRepository {
    pool: PgPool,
}

impl CouponRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 按 ID 查询优惠券定义（含已软删除记录，由调用方判定）。
    pub async fn find_by_id(&self, coupon_id: i32) -> Result<Option<Coupon>> {
        let sql = format!("SELECT {COUPON_COLUMNS} FROM coupon WHERE coupon_id = ? LIMIT 1");
        sqlx::query_as::<_, Coupon>(&sql)
            .bind(coupon_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 批量按 ID 查询（前台 `listByIds`）。
    pub async fn find_by_ids(&self, ids: &[i32]) -> Result<Vec<Coupon>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; ids.len()].join(",");
        let sql = format!(
            "SELECT {COUPON_COLUMNS} FROM coupon WHERE coupon_id IN ({placeholders}) AND is_delete = 0"
        );
        let mut q = sqlx::query_as::<_, Coupon>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 管理端分页：关键字 + 状态（按用券时间派生）。
    ///
    /// `status` 语义与原 Java `applyStatusCondition` 一致：
    /// - 2 待生效：非按领取天数，且开始时间晚于当前
    /// - 3 已失效：非按领取天数，且结束时间早于当前
    /// - 1 进行中：按领取天数，或处于用券时间段内
    pub async fn paginate(
        &self,
        keyword: Option<&str>,
        status: Option<i32>,
        shop_id: Option<i32>,
        now: i64,
        page: &PageQuery,
    ) -> Result<(u64, Vec<Coupon>)> {
        let like = keyword.map(|k| format!("%{k}%"));
        let (limit, offset) = page.limit_offset();

        // 状态条件片段（与关键字/店铺条件组合）
        let status_clause = match status {
            Some(2) => " AND send_type <> 0 AND use_start_date > ?",
            Some(3) => " AND send_type <> 0 AND use_end_date > 0 AND use_end_date < ?",
            Some(1) => " AND (send_type = 0 OR (use_start_date <= ? AND use_end_date >= ?))",
            _ => "",
        };
        let shop_clause = if shop_id.filter(|v| *v > -1).is_some() {
            " AND shop_id = ?"
        } else {
            ""
        };
        let where_sql = format!(
            "WHERE is_delete = 0 AND (? IS NULL OR coupon_name LIKE ?){shop_clause}{status_clause}"
        );

        let count_sql = format!("SELECT COUNT(*) FROM coupon {where_sql}");
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql).bind(&like).bind(&like);
        if let Some(sid) = shop_id.filter(|v| *v > -1) {
            cq = cq.bind(sid);
        }
        match status {
            Some(1) => {
                cq = cq.bind(now).bind(now);
            }
            Some(2) | Some(3) => {
                cq = cq.bind(now);
            }
            _ => {}
        }
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let list_sql = format!(
            "SELECT {COUPON_COLUMNS} FROM coupon {where_sql} ORDER BY coupon_id DESC LIMIT ? OFFSET ?"
        );
        let mut lq = sqlx::query_as::<_, Coupon>(&list_sql).bind(&like).bind(&like);
        if let Some(sid) = shop_id.filter(|v| *v > -1) {
            lq = lq.bind(sid);
        }
        match status {
            Some(1) => {
                lq = lq.bind(now).bind(now);
            }
            Some(2) | Some(3) => {
                lq = lq.bind(now);
            }
            _ => {}
        }
        let rows = lq
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 管理端状态计数（全量，忽略分页与 status 入参）。
    pub async fn count_by_status(
        &self,
        shop_id: Option<i32>,
        now: i64,
    ) -> Result<(i64, i64, i64)> {
        let shop_clause = if shop_id.filter(|v| *v > -1).is_some() {
            " AND shop_id = ?"
        } else {
            ""
        };
        // 单次查询用条件聚合，避免三次往返
        let sql = format!(
            "SELECT
               SUM(CASE WHEN send_type = 0 OR (use_start_date <= ? AND use_end_date >= ?) THEN 1 ELSE 0 END),
               SUM(CASE WHEN send_type <> 0 AND use_start_date > ? THEN 1 ELSE 0 END),
               SUM(CASE WHEN send_type <> 0 AND use_end_date > 0 AND use_end_date < ? THEN 1 ELSE 0 END)
             FROM coupon WHERE is_delete = 0{shop_clause}"
        );
        let mut q = sqlx::query_as::<_, (Option<i64>, Option<i64>, Option<i64>)>(&sql)
            .bind(now)
            .bind(now)
            .bind(now)
            .bind(now);
        if let Some(sid) = shop_id.filter(|v| *v > -1) {
            q = q.bind(sid);
        }
        let (processing, pending, expired) = q.fetch_one(&self.pool).await.map_err(map_err)?;
        Ok((
            processing.unwrap_or(0),
            pending.unwrap_or(0),
            expired.unwrap_or(0),
        ))
    }

    /// 前台可领取列表：仅展示中、未删除、在有效期内的券。
    ///
    /// 对应 Java `CouponServiceImpl#getList` 中
    /// `receiveFlag && validDateFlag` 的有效期条件。
    pub async fn paginate_claimable(
        &self,
        now: i64,
        shop_id: Option<i32>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<Coupon>)> {
        let (limit, offset) = page.limit_offset();
        let shop_clause = if shop_id.filter(|v| *v > -1).is_some() {
            " AND shop_id = ?"
        } else {
            ""
        };
        // 有效期：固定时间段未结束，或按领取后天数（send_type = 0）
        let where_sql = format!(
            "WHERE is_delete = 0 AND is_show = 1 AND enabled_click_get = 1{shop_clause}
             AND (send_type = 0 OR use_end_date >= ?)"
        );

        let count_sql = format!("SELECT COUNT(*) FROM coupon {where_sql}");
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        if let Some(sid) = shop_id.filter(|v| *v > -1) {
            cq = cq.bind(sid);
        }
        let total: (i64,) = cq.bind(now).fetch_one(&self.pool).await.map_err(map_err)?;

        let list_sql = format!(
            "SELECT {COUPON_COLUMNS} FROM coupon {where_sql} ORDER BY add_time DESC LIMIT ? OFFSET ?"
        );
        let mut lq = sqlx::query_as::<_, Coupon>(&list_sql);
        if let Some(sid) = shop_id.filter(|v| *v > -1) {
            lq = lq.bind(sid);
        }
        let rows = lq
            .bind(now)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 统计某券的已领取（已发放）总量。
    pub async fn count_issued(&self, coupon_id: i32) -> Result<i64> {
        let (n,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM user_coupon WHERE coupon_id = ?")
                .bind(coupon_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(n)
    }

    /// 批量统计已领取数量，返回 `coupon_id -> count`。
    pub async fn count_issued_batch(
        &self,
        coupon_ids: &[i32],
    ) -> Result<std::collections::HashMap<i32, i64>> {
        let mut map = std::collections::HashMap::new();
        if coupon_ids.is_empty() {
            return Ok(map);
        }
        let placeholders = vec!["?"; coupon_ids.len()].join(",");
        let sql = format!(
            "SELECT coupon_id, COUNT(*) FROM user_coupon WHERE coupon_id IN ({placeholders}) GROUP BY coupon_id"
        );
        let mut q = sqlx::query_as::<_, (i32, i64)>(&sql);
        for id in coupon_ids {
            q = q.bind(id);
        }
        for (cid, cnt) in q.fetch_all(&self.pool).await.map_err(map_err)? {
            map.insert(cid, cnt);
        }
        Ok(map)
    }

    /// 统计某用户对某券的领取数量（限领校验）。
    pub async fn count_user_claimed(&self, user_id: i32, coupon_id: i32) -> Result<i64> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM user_coupon WHERE user_id = ? AND coupon_id = ?",
        )
        .bind(user_id)
        .bind(coupon_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(n)
    }

    /// 批量查询用户已领取的券 ID（前台 `isReceive` 标记）。
    pub async fn find_user_claimed_ids(
        &self,
        user_id: i32,
        coupon_ids: &[i32],
    ) -> Result<Vec<i32>> {
        if coupon_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; coupon_ids.len()].join(",");
        let sql = format!(
            "SELECT DISTINCT coupon_id FROM user_coupon WHERE user_id = ? AND coupon_id IN ({placeholders})"
        );
        let mut q = sqlx::query_as::<_, (i32,)>(&sql).bind(user_id);
        for id in coupon_ids {
            q = q.bind(id);
        }
        Ok(q.fetch_all(&self.pool)
            .await
            .map_err(map_err)?
            .into_iter()
            .map(|(id,)| id)
            .collect())
    }

    /// 查询用户名下优惠券。
    ///
    /// 排序与原 Java `UserCouponServiceImpl#list` 一致：
    /// 可用的排前面（`used_time = 0` 且在有效期内），其次按 `start_date` 升序。
    pub async fn find_user_coupons(
        &self,
        user_id: i32,
        used_time: Option<i64>,
        now: i64,
        page: &PageQuery,
    ) -> Result<(u64, Vec<UserCoupon>)> {
        let (limit, offset) = page.limit_offset();
        let time_clause = if used_time.is_some() {
            " AND used_time = ?"
        } else {
            ""
        };
        let where_sql = format!("WHERE user_id = ?{time_clause}");

        let count_sql = format!("SELECT COUNT(*) FROM user_coupon {where_sql}");
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql).bind(user_id);
        if let Some(t) = used_time {
            cq = cq.bind(t);
        }
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let list_sql = format!(
            "SELECT {USER_COUPON_COLUMNS} FROM user_coupon {where_sql}
             ORDER BY CASE WHEN used_time = 0 AND start_date <= ? AND end_date >= ? THEN 0 ELSE 1 END,
                      start_date ASC
             LIMIT ? OFFSET ?"
        );
        let mut lq = sqlx::query_as::<_, UserCoupon>(&list_sql).bind(user_id);
        if let Some(t) = used_time {
            lq = lq.bind(t);
        }
        let rows = lq
            .bind(now)
            .bind(now)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 查询用户可用的优惠券（未使用且在有效期内），供下单抵扣选择。
    pub async fn find_usable_user_coupons(
        &self,
        user_id: i32,
        now: i64,
    ) -> Result<Vec<UserCoupon>> {
        let sql = format!(
            "SELECT {USER_COUPON_COLUMNS} FROM user_coupon
             WHERE user_id = ? AND order_id = 0 AND used_time = 0
               AND start_date <= ? AND end_date >= ?
             ORDER BY end_date ASC"
        );
        sqlx::query_as::<_, UserCoupon>(&sql)
            .bind(user_id)
            .bind(now)
            .bind(now)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按主键查询用户券。
    pub async fn find_user_coupon_by_id(&self, id: i32) -> Result<Option<UserCoupon>> {
        let sql = format!("SELECT {USER_COUPON_COLUMNS} FROM user_coupon WHERE id = ? LIMIT 1");
        sqlx::query_as::<_, UserCoupon>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 发放一张优惠券，返回自增主键。
    pub async fn insert_user_coupon(&self, uc: &UserCoupon) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO user_coupon
               (coupon_id, coupon_sn, user_id, used_time, order_id, start_date, end_date, source_type)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(uc.coupon_id)
        .bind(&uc.coupon_sn)
        .bind(uc.user_id)
        .bind(uc.used_time)
        .bind(uc.order_id)
        .bind(uc.start_date)
        .bind(uc.end_date)
        .bind(uc.source_type.unwrap_or(0))
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 删除用户的某张优惠券（仅限本人、且未使用）。
    ///
    /// 对应 Java `UserCouponServiceImpl#deleteByUser`。
    pub async fn delete_user_coupon(&self, id: i32, user_id: i32) -> Result<bool> {
        let r = sqlx::query("DELETE FROM user_coupon WHERE id = ? AND user_id = ? AND order_id = 0")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 原子核销：把未使用且未过期的券绑定到订单上。
    ///
    /// 通过 `order_id = 0 AND used_time = 0` 作为条件实现乐观并发控制，
    /// 防止同一张券被并发重复使用（返回 `false` 表示已被占用）。
    pub async fn try_mark_used(
        &self,
        user_coupon_id: i32,
        user_id: i32,
        order_id: i32,
        now: i64,
    ) -> Result<bool> {
        let r = sqlx::query(
            "UPDATE user_coupon SET order_id = ?, used_time = ?
             WHERE id = ? AND user_id = ? AND order_id = 0 AND used_time = 0
               AND start_date <= ? AND end_date >= ?",
        )
        .bind(order_id)
        .bind(now as i32)
        .bind(user_coupon_id)
        .bind(user_id)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 订单取消/退款时回退优惠券占用。
    pub async fn release_by_order(&self, order_id: i32) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE user_coupon SET order_id = 0, used_time = 0 WHERE order_id = ?",
        )
        .bind(order_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 统计用户当前可用（未使用且在有效期内）的券数量。
    pub async fn count_user_usable(&self, user_id: i32, now: i64) -> Result<i64> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM user_coupon
             WHERE user_id = ? AND order_id = 0 AND used_time = 0
               AND start_date <= ? AND end_date >= ?",
        )
        .bind(user_id)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(n)
    }

    /// 按 ID 列表查询用户券（核销批量场景）。
    pub async fn find_user_coupons_by_ids(
        &self,
        user_id: i32,
        ids: &[i32],
    ) -> Result<Vec<UserCoupon>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; ids.len()].join(",");
        let sql = format!(
            "SELECT {USER_COUPON_COLUMNS} FROM user_coupon
             WHERE user_id = ? AND id IN ({placeholders})"
        );
        let mut q = sqlx::query_as::<_, UserCoupon>(&sql).bind(user_id);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("优惠券不存在".into()),
        other => Error::Database(other.to_string()),
    }
}