//! 店铺会员域仓储（member::shop_member）。
//!
//! 对应原 Java `ShopMemberMapper` + `UserMapper` / `ShopMapper` / `ConfigService`
//! 的只读与写入。
//!
//! 表结构（`python tools/schema_parser.py --table shop_member` 可复核）：
//! - `shop_member`（10 列，主键 `id` BIGINT，逻辑删除列 `is_del`）
//! - `user`（只读 `user_id` / `username` / `nickname` / `mobile` / `email` /
//!   `avatar` / `user_type`）
//! - `shop`（只读 `shop_id` / `shop_title` / `shop_logo` / `status` / `shop_type`）
//! - `config`（读写 `biz_code` / `biz_val` / `is_del`）
//!
//! 查询统一使用运行时 sqlx（不用 `query!` 宏）；金额 `Decimal`。

use rust_decimal::Decimal;
use sqlx::{FromRow,Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::ShopMember;
use nextrade_core::models::member::shop_member::ShopMemberQueryPage;
use nextrade_core::{Error, Result};

/// `shop_member` 固定 SELECT 列清单（顺序与实体一致）。
const MEMBER_SELECT: &str = "SELECT id, user_id, shop_id, order_count, order_amount, join_time, last_consume_time, create_time, update_time, is_del FROM shop_member";

/// 会员列表行（`shop_member LEFT JOIN user`）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct ShopMemberRow {
    pub user_id: i32,
    pub order_count: i32,
    pub order_amount: Decimal,
    pub join_time: Option<i64>,
    pub last_consume_time: Option<i64>,
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub avatar: Option<String>,
}

/// 店铺列表行。
#[derive(Debug, Clone, Default, FromRow)]
pub struct ShopRow {
    pub shop_id: i32,
    pub shop_title: Option<String>,
    pub shop_logo: Option<String>,
    pub status: Option<i32>,
    pub shop_type: Option<i32>,
}

/// 用户简信息（新增客户 / 列表装配）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct MemberUserRow {
    pub user_id: i32,
    pub username: String,
    pub mobile: String,
    pub email: String,
    pub nickname: Option<String>,
    pub avatar: String,
    pub user_type: Option<i32>,
}

/// 店铺会员域仓储。
#[derive(Clone)]
pub struct ShopMemberRepository {
    pool: PgPool,
}

impl ShopMemberRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 配置读写 —————————————————————

    /// 读取 `config.biz_val`（`is_del = 0`）。
    pub async fn config_value(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1",
        )
        .bind(biz_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }

    /// 写入配置（Java `SettingsHelper#set` 的持久化部分）。
    ///
    /// 先 `UPDATE`；受影响 0 行则 `INSERT`（`biz_code` 不存在时）。
    pub async fn set_config_value(&self, biz_code: &str, value: &str, now: i64) -> Result<()> {
        let res = sqlx::query(
            "UPDATE config SET biz_val = $1, update_time = $2 WHERE biz_code = $3 AND is_del = 0",
        )
        .bind(value)
        .bind(now)
        .bind(biz_code)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        if res.rows_affected() == 0 {
            sqlx::query(
                "INSERT INTO config (biz_code, biz_val, create_time, update_time, is_del) VALUES ($1, $2, $3, $4, 0)",
            )
            .bind(biz_code)
            .bind(value)
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        }
        Ok(())
    }

    /// 会员默认头像配置（Java `getUserAvatar` → `DEFAULT_AVATAR`）。
    pub async fn default_avatar(&self) -> Result<Option<String>> {
        self.config_value("defaultAvatar").await
    }

    // ————————————————————— 店铺列表 —————————————————————

    /// 平台端店铺分页（Java `shopList`：`LIKE shop_title` + `shop_type` +
    /// `ORDER BY shop_id DESC`）。
    pub async fn paginate_shops(
        &self,
        q: &nextrade_core::models::member::shop_member::ShopMemberShopQueryPage,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<ShopRow>)> {
        let mut cq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM shop WHERE 1 = 1");
        Self::push_shop_filters(&mut cq, q);
        let total: (i64,) = cq
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut rq: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT shop_id, shop_title, shop_logo, status, shop_type FROM shop WHERE 1 = 1",
        );
        Self::push_shop_filters(&mut rq, q);
        rq.push(" ORDER BY shop_id DESC");
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<ShopRow> = rq
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0, rows))
    }

    fn push_shop_filters(
        qb: &mut QueryBuilder<'_, Postgres>,
        q: &nextrade_core::models::member::shop_member::ShopMemberShopQueryPage,
    ) {
        if let Some(title) = q.effective_shop_title() {
            qb.push(" AND shop_title LIKE ");
            qb.push_bind(format!("%{title}%"));
        }
        if let Some(shop_type) = q.shop_type {
            qb.push(" AND shop_type = ");
            qb.push_bind(shop_type);
        }
    }

    /// 批量统计店铺会员数（Java `countQw` 的 `GROUP BY shop_id`）。
    pub async fn count_members_by_shops(&self, shop_ids: &[i32]) -> Result<Vec<(i32, i64)>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT shop_id, COUNT(*) AS cnt FROM shop_member WHERE is_del = 0 AND shop_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in shop_ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.push(" AND user_id IN (SELECT user_id FROM \"user\" WHERE user_type = 0) GROUP BY shop_id");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 会员列表 / 详情 —————————————————————

    /// 会员分页（Java `memberList`：`LEFT JOIN user` + `user_type` +
    /// `ORDER BY join_time DESC`）。
    pub async fn paginate_members(
        &self,
        q: &ShopMemberQueryPage,
        shop_id: i32,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<ShopMemberRow>)> {
        let mut cq: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT COUNT(*) FROM shop_member m LEFT JOIN \"user\" u ON u.user_id = m.user_id WHERE 1 = 1",
        );
        Self::push_member_filters(&mut cq, q, shop_id);
        let total: (i64,) = cq
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut rq: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT m.user_id, m.order_count, m.order_amount, m.join_time, m.last_consume_time, u.username, u.nickname, u.mobile, u.email, u.avatar FROM shop_member m LEFT JOIN \"user\" u ON u.user_id = m.user_id WHERE 1 = 1",
        );
        Self::push_member_filters(&mut rq, q, shop_id);
        rq.push(" ORDER BY m.join_time DESC");
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<ShopMemberRow> = rq
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0, rows))
    }

    fn push_member_filters(
        qb: &mut QueryBuilder<'_, Postgres>,
        q: &ShopMemberQueryPage,
        shop_id: i32,
    ) {
        qb.push(" AND m.is_del = 0 AND m.shop_id = ");
        qb.push_bind(shop_id);
        qb.push(" AND u.user_type = ");
        qb.push_bind(q.effective_user_type());
        if let Some(user_id) = q.user_id {
            qb.push(" AND m.user_id = ");
            qb.push_bind(user_id);
        }
        if let Some(keyword) = q.effective_keyword() {
            let like = format!("%{keyword}%");
            qb.push(" AND (u.nickname LIKE ");
            qb.push_bind(like.clone());
            qb.push(" OR u.mobile LIKE ");
            qb.push_bind(like.clone());
            qb.push(" OR u.email LIKE ");
            qb.push_bind(like);
            qb.push(")");
        }
        if let Some(min) = q.order_amount_min {
            qb.push(" AND m.order_amount >= ");
            qb.push_bind(min);
        }
        if let Some(max) = q.order_amount_max {
            qb.push(" AND m.order_amount <= ");
            qb.push_bind(max);
        }
        if let Some(start) = q.join_time_start {
            qb.push(" AND m.join_time >= ");
            qb.push_bind(start);
        }
        if let Some(end) = q.join_time_end {
            qb.push(" AND m.join_time <= ");
            qb.push_bind(end);
        }
        if let Some(start) = q.last_consume_time_start {
            qb.push(" AND m.last_consume_time >= ");
            qb.push_bind(start);
        }
        if let Some(end) = q.last_consume_time_end {
            qb.push(" AND m.last_consume_time <= ");
            qb.push_bind(end);
        }
    }

    /// 按「店铺 + 用户」取会员绑定（Java `detail` 的 `one()`）。
    pub async fn find_member(&self, shop_id: i32, user_id: i32) -> Result<Option<ShopMember>> {
        sqlx::query_as::<_, ShopMember>(&format!(
            "{MEMBER_SELECT} WHERE shop_id = $1 AND user_id = $2 AND is_del = 0 LIMIT 1"
        ))
        .bind(shop_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 会员绑定是否已存在（Java `bindMember` 的 `count > 0`）。
    pub async fn count_member(&self, shop_id: i32, user_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM shop_member WHERE shop_id = $1 AND user_id = $2 AND is_del = 0",
        )
        .bind(shop_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 新建会员绑定（Java `bindMember` 的 `save`）。
    pub async fn insert_member(&self, shop_id: i32, user_id: i32, now: i64) -> Result<i64> {
        let res: i64 = sqlx::query_scalar(
            "INSERT INTO shop_member (user_id, shop_id, order_count, order_amount, join_time, last_consume_time, create_time, update_time, is_del) VALUES ($1, $2, 0, 0.00, $3, NULL, $4, $5, 0) RETURNING id",
        )
        .bind(user_id)
        .bind(shop_id)
        .bind(now)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    // ————————————————————— 用户查询 —————————————————————

    /// 按主键取用户（Java `userService.getById`）。
    pub async fn find_user_by_id(&self, user_id: i32) -> Result<Option<MemberUserRow>> {
        sqlx::query_as::<_, MemberUserRow>(
            "SELECT user_id, username, mobile, email, nickname, avatar, user_type FROM \"user\" WHERE user_id = $1 LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按手机号取会员（Java `userService.getUserByMobile`）。
    pub async fn find_user_by_mobile(&self, mobile: &str) -> Result<Option<MemberUserRow>> {
        sqlx::query_as::<_, MemberUserRow>(
            "SELECT user_id, username, mobile, email, nickname, avatar, user_type FROM \"user\" WHERE mobile = $1 AND user_type = 0 LIMIT 1",
        )
        .bind(mobile)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 批量取用户（Java `listByIds`）。
    pub async fn find_users_by_ids(&self, ids: &[i32]) -> Result<Vec<MemberUserRow>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT user_id, username, mobile, email, nickname, avatar, user_type FROM \"user\" WHERE user_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_lists_cover_schema_columns() {
        let selected = MEMBER_SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap();
        assert_eq!(selected, nextrade_core::models::member::shop_member::MEMBER_COLUMNS.join(", "));
    }

    #[test]
    fn row_defaults_are_null_friendly() {
        let row = ShopMemberRow::default();
        assert_eq!(row.order_amount, Decimal::ZERO);
        assert_eq!(row.username, None);
        let shop = ShopRow::default();
        assert_eq!(shop.shop_title, None);
    }

    #[test]
    fn user_row_mobile_not_null_like_java() {
        let u = MemberUserRow::default();
        assert!(u.mobile.is_empty());
        assert_eq!(u.user_type, None);
    }
}
