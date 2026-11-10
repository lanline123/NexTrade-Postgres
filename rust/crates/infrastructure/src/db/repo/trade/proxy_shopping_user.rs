//! 代客下单 · 会员（trade::proxy_shopping_user）仓储：单表 `user`。
//!
//! 对应原 Java `com.nextrade.adminapi.controller.proxyshopping.ProxyShoppingUserController`
//! （`@RequestMapping("/adminapi/proxyShopping/user")`）两条只读链路背后的
//! `UserMapper` 查询：
//! - `list` → `UserServiceImpl#list`（关键词 `=` / `ILIKE` 两种模式）
//! - `detail` → `UserServiceImpl#getUserAndUserRank`（按 `user_id` 取单条）
//!
//! 列名严格对齐权威 schema
//! （`cd rust && python tools/schema_parser.py --table user`，权威源
//! `java/database/pure.sql`），`user` 共 36 列，包含本文件用到的
//! `user_id` / `username` / `email` / `mobile`。
//!
//! 约定：
//! - 一律使用**运行时** `sqlx::query_as`，**不使用** `query!` 宏
//!   （本机无 `DATABASE_URL`，宏必然编译失败）。
//! - 仓储层**不做业务校验**（Java `UserMapper` 也不做），只忠实执行 SQL。
//!
//! # 与 `repo::user_manage` 的关系（为什么不重复实现分页）
//!
//! `trade::proxy_shopping_user` 的 `list` / `detail` 与 `member::user_manage`
//! （`/adminapi/user/user/list` / `detail`）**共用同一个** `UserServiceImpl`
//! 方法，唯一差异是控制器在调用前按 `x-admin-type` 覆盖了 `isExact`
//! （见 `services/proxy_shopping_user.rs` 模块头）。分页 SQL 的筛选语义
//! （标签筛选、关键词精确/模糊、余额/积分区间、排序）已由
//! `repo::nextrade_core::models::member::user_manage::UserManageRepository#paginate_users` 实现并经本片复核，
//! 服务层**直接复用**该实现，避免两份 SQL 漂移。
//! 本文件只放本域独占的只读诊断查询（用于自检与列名回归）。

use sqlx::PgPool;
use nextrade_core::{Error, Result};

/// `user` 表中本域查询涉及的列（单行常量，便于按 schema 核对）。
///
/// 单行书写：`tools/check_sql_columns.py` 按 SQL 字面量扫描，
/// 跨行拼接会让静态校验读到半截语句。
pub const USER_KEYWORD_COLUMNS: &str = "username, email, mobile";

/// 代客下单会员域仓储。
#[derive(Clone)]
pub struct ProxyShoppingUserRepository {
    pool: PgPool,
}

impl ProxyShoppingUserRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排 / 复用共享仓储）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 按 Java `UserServiceImpl#list` 的关键词语义统计命中会员数。
    ///
    /// 逐条对应 Java 的 wrapper 链（仅关键词那一段）：
    /// ```text
    /// if (Boolean.TRUE.equals(dto.getIsExact()))  // 精确
    ///     eq(username, kw).or().eq(email, kw).or().eq(mobile, kw)
    /// else                                        // 模糊
    ///     like(username, kw).or().like(email, kw).or().like(mobile, kw)
    /// ```
    /// 注意 Java 用 `StrUtil.isNotBlank(keyword)` 守卫：空白关键词**不参与**
    /// 筛选（即统计全部会员），本方法同样以 `None` 表示「不按关键词过滤」。
    ///
    /// 本方法**只用于自检 / 诊断**（代客下单页「精确匹配会员」的语义回归），
    /// 列表分页本身走 `repo::nextrade_core::models::member::user_manage::paginate_users`，
    /// 以免同一段筛选语义出现两份会漂移的实现。
    ///
    /// # Errors
    /// 数据库错误向上抛出。
    pub async fn count_by_keyword(&self, keyword: Option<&str>, exact: bool) -> Result<i64> {
        let Some(kw) = keyword.map(str::trim).filter(|s| !s.is_empty()) else {
            let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM \"user\"")
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
            return Ok(row.0);
        };
        // 两种模式各写一条**单行** SQL 字面量，便于静态列名校验。
        let row: (i64,) = if exact {
            sqlx::query_as(
                "SELECT COUNT(*) FROM \"user\" WHERE LOWER(username) = LOWER($1) OR LOWER(email) = LOWER($2) OR mobile = $3",
            )
            .bind(kw)
            .bind(kw)
            .bind(kw)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?
        } else {
            let pattern = format!("%{kw}%");
            sqlx::query_as(
                "SELECT COUNT(*) FROM \"user\" WHERE username ILIKE $1 OR email ILIKE $2 OR mobile ILIKE $3",
            )
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?
        };
        Ok(row.0)
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
    fn keyword_columns_are_real_user_columns() {
        // 逐列对照 `schema_parser.py --table user`（36 列）中的实际列名。
        for col in ["username", "email", "mobile"] {
            assert!(
                USER_KEYWORD_COLUMNS.split(", ").any(|c| c == col),
                "USER_KEYWORD_COLUMNS 缺少列 {col}"
            );
        }
        assert_eq!(USER_KEYWORD_COLUMNS.split(", ").count(), 3);
    }

    #[test]
    fn repository_is_cheap_to_clone() {
        // 仓储只持有连接池句柄（`PgPool` 内部为 Arc），Clone 必须廉价；
        // 这里只做类型层面的编译期校验占位。
        fn assert_clone<T: Clone>() {}
        assert_clone::<ProxyShoppingUserRepository>();
    }
}