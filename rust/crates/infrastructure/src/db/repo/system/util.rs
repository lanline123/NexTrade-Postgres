//! 通用工具 域仓储（system::util）。
//!
//! ⚠️ 本 feature **没有数据库访问**：Java `UtilController` 只依赖二维码库
//! （hutool `QrCodeUtil`）与 `WechatOAuthService`，不读写任何表。
//!
//! 保留仓储类型是为了与其余 feature 的四层结构/组合根装配保持一致；
//! [`UtilRepository::pool_clone`] 仅供组合根自检使用。
//!
//! 运行时 sqlx 约束只适用于有 SQL 的仓储（见 `tools/check_sql_columns.py`）；
//! 本文件不含 SQL。

use sqlx::PgPool;

/// 通用工具仓储（无表访问）。
#[derive(Clone)]
pub struct UtilRepository {
    /// 连接池（本 feature 未使用；保留以统一 `new(pool)` 装配签名）
    pool: PgPool,
}

impl UtilRepository {
    /// 构造仓储。
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（组合根自检/未来扩展用）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }
}
