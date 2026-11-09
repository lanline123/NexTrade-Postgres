//! 消息控制器（内部接口） 域仓储（im::web_socket）骨架。
//!
//! 表名/列名必须来自权威 schema `java/database/pure.sql`
//! （`cd rust && python tools/schema_parser.py --table <表名>`）；
//! SQL 一律运行时 `sqlx::query_as`，**不用** `query!` 宏；每条 SQL 单行字面量。

use sqlx::PgPool;

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct WebSocketRepository {
    pool: PgPool,
}

impl WebSocketRepository {
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