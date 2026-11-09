//! 静态资源文件（FilesController） 域仓储（system::files）骨架。
//!
//! 本文件由协调者预注册（保证 `cargo check` 绿），具体查询由域实现者按
//! 权威 schema `java/database/pure.sql` 补齐：
//! `cd rust && python tools/schema_parser.py --table <表名>` 查权威列。
//!
//! 约定：一律用**运行时** `sqlx::query_as` / `sqlx::query`，**不用** `query!` 宏
//! （本机无 `DATABASE_URL`）；每条 SQL 写成单行字面量，便于
//! `python tools/check_sql_columns.py --strict` 静态校验。

use sqlx::PgPool;

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct FilesRepository {
    pool: PgPool,
}

impl FilesRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }
}