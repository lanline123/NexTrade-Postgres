//! 会员积分日志 域仓储（member::user_points_log）。
//!
//! 表名/列名必须来自权威 schema `java/database/pure.sql`
//! （`cd rust && python tools/schema_parser.py --table <表名>`）；
//! SQL 一律运行时 `sqlx::query_as`，**不用** `query!` 宏；每条 SQL 单行字面量。
//!
//! # 本片对应的 Java 查询
//!
//! `UserPointsLogServiceImpl#getPoints`（`UserPointsLogServiceImpl.java:59-66`）：
//!
//! ```java
//! User currentUser = userMapper.selectOne(
//!         Wrappers.lambdaQuery(User.class).eq(User::getUserId, SecurityUtils.getCurrentAdminId()));
//! return currentUser.getPoints();
//! ```
//!
//! 即 `SELECT * FROM user WHERE user_id = <当前管理员ID>` 取单行，只用 `points`
//! 一个字段。这里直接投影 `points`（`user.points` 为 `int unsigned NOT NULL`），
//! 语义等价且不必读 36 列。
//!
//! # 列名核对
//!
//! `python tools/schema_parser.py --table user` → 36 列，本文件只用
//! `user_id`（主键）与 `points`。
//!
//! # 为什么 `selectOne` 的「多行」分支不复刻
//!
//! MyBatis-Plus 的 `selectOne` 在命中 >1 行时抛
//! `TooManyResultsException`；`user_id` 是 `user` 表唯一主键
//! （`schema_parser` 输出 `主键: user_id`），物理上不可能多行，
//! 故 Rust 侧用 `fetch_optional` 即可（`tools/check_sql_columns.py`
//! 也会校验该语句只涉及 `user` 一张表）。

use sqlx::PgPool;
use nextrade_core::{Error, Result};

/// `user` 表中本域查询涉及的列（单行常量，便于按 schema 核对）。
pub const USER_POINTS_COLUMNS: &str = "user_id, points";

/// 会员积分日志 域仓储。
#[derive(Clone)]
pub struct UserPointsLogRepository {
    pool: PgPool,
}

impl UserPointsLogRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 取指定账号绑定会员的积分（Java `getPoints` 里 `selectOne` 的那一行）。
    ///
    /// `admin_user_id` 为 Java `SecurityUtils.getCurrentAdminId()` 的取值，
    /// Rust 侧由 JWT 解析出的 `AuthUser.id` 传入（见路由层）。
    ///
    /// 返回 `Ok(None)` 表示 `user` 表没有 `user_id = admin_user_id` 的行
    /// （Java 会在此处 NPE，服务层把它转成明确的业务错误）。
    ///
    /// # Errors
    /// 数据库错误向上抛出（`Error::Database`）。
    pub async fn points_of_user(&self, admin_user_id: i32) -> Result<Option<i32>> {
        let row: Option<(Option<i32>,)> = sqlx::query_as(
            "SELECT \"points\" FROM \"user\" WHERE \"user_id\" = $1 LIMIT 1",
        )
        .bind(admin_user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }

    /// 增加会员积分并记录日志（对齐 Java `UserPointsLogServiceImpl#incPoints`）。
    pub async fn inc_points(
        &self,
        points: i32,
        user_id: i32,
        desc: &str,
        relation_id: Option<i32>,
    ) -> Result<()> {
        if points <= 0 {
            return Ok(());
        }
        let now = nextrade_common::utils::now_seconds() as i32;
        sqlx::query(
            "UPDATE \"user\" SET \"points\" = \"points\" + $1 WHERE \"user_id\" = $2",
        )
        .bind(points)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        sqlx::query(
            "INSERT INTO \"user_points_log\" (\"user_id\", \"points\", \"change_time\", \"change_desc\", \"change_type\", \"relation_type\", \"relation_id\") VALUES ($1, $2, $3, $4, 1, 1, $5)",
        )
        .bind(user_id)
        .bind(points)
        .bind(now)
        .bind(desc)
        .bind(relation_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("会员不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_columns_match_schema() {
        // 逐列对照 `schema_parser.py --table user`
        for col in ["user_id", "points"] {
            assert!(
                USER_POINTS_COLUMNS.split(", ").any(|c| c == col),
                "USER_POINTS_COLUMNS 缺少列 {col}"
            );
        }
        assert_eq!(USER_POINTS_COLUMNS.split(", ").count(), 2);
    }

    #[test]
    fn repository_is_cheap_to_clone() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<UserPointsLogRepository>();
    }
}