//! 配置管理 域仓储（system::cache_manage）：Redis 全量键清理。
//!
//! 对应原 Java `CacheManageServiceImpl#cleanup`：
//!
//! ```java
//! Set<String> allKeys = redisTemplate.keys("*");
//! Set<String> keysToDelete = allKeys.stream()
//!         .filter(key -> NO_CLEANUP_TAGS.stream().noneMatch(key::startsWith))
//!         .collect(Collectors.toSet());
//! redisTemplate.delete(keysToDelete);
//! ```
//!
//! ## 为什么仓储层直连 Redis
//!
//! 本片契约要求 `CacheManageRepository` 只持有 `PgPool`（公开签名固定），
//! 而「清理缓存」是纯 Redis 操作，与 MySQL 无关。基础设施层是**唯一**
//! 可直接依赖 `redis` crate 的位置（`services` / `api` 均未声明该依赖），
//! 且 `nextrade_infrastructure::cache::CacheService` 只提供
//! `get/set/del/incr`，**没有** `keys` / `scan` 键枚举能力。
//! 因此这里按需（每次调用）建立一次性连接，用 `SCAN` 游标替代 Java 的
//! `KEYS *`（`KEYS` 在大库上会阻塞 Redis 主线程，`SCAN` 为官方推荐替代）。
//!
//! 说明：本文件不涉及任何 MySQL 表/列，故无需 schema 核对。

use redis::aio::ConnectionManager;
use sqlx::PgPool;
use nextrade_common::AppConfig;
use nextrade_core::{Error, Result};

/// 单次 `SCAN` 的 `COUNT` 提示值（仅提示，Redis 可能多返回）。
const SCAN_COUNT: usize = 500;

/// 单次 `DEL` 的最大 key 数（避免单条命令参数过长）。
const DEL_BATCH: usize = 500;

/// 仓储（持有 MySQL 连接池）。
///
/// 注意：本片所有操作走 Redis，`pool` 仅为满足统一的仓储装配约定而保留。
#[derive(Clone)]
pub struct CacheManageRepository {
    pool: PgPool,
}

impl CacheManageRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 缓存清理 —————————————————————

    /// 扫描 Redis 全量 key（`SCAN 0 COUNT n` 游标循环）。
    ///
    /// 对应 Java `redisTemplate.keys("*")` 的结果集；改用 `SCAN` 是为了
    /// 避免 `KEYS` 在大 key 空间下阻塞 Redis 主线程。
    ///
    /// # Errors
    /// - Redis 不可用 / URL 非法 / 配置加载失败 ⇒ [`Error::Cache`]。
    #[allow(clippy::unused_self)] // 与其它仓储保持 `&self` 统一调用形态
    pub async fn scan_all_keys(&self) -> Result<Vec<String>> {
        let mut conn = redis_conn().await?;
        let mut cursor: u64 = 0;
        let mut keys: Vec<String> = Vec::new();
        loop {
            let (next, batch): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("COUNT")
                .arg(SCAN_COUNT)
                .query_async(&mut conn)
                .await
                .map_err(|e| Error::Cache(format!("Redis SCAN 失败: {e}")))?;
            keys.extend(batch);
            if next == 0 {
                break;
            }
            cursor = next;
        }
        tracing::debug!("缓存清理：SCAN 命中 {} 个 key", keys.len());
        Ok(keys)
    }

    /// 批量删除 Redis key，返回 `DEL` 实际删除的数量。
    ///
    /// 对应 Java `redisTemplate.delete(Collection<String>)`。
    ///
    /// # Errors
    /// Redis 不可用 / URL 非法 / 命令失败 ⇒ [`Error::Cache`]。
    #[allow(clippy::unused_self)] // 与其它仓储保持 `&self` 统一调用形态
    pub async fn delete_keys(&self, keys: &[String]) -> Result<u64> {
        if keys.is_empty() {
            return Ok(0);
        }
        let mut conn = redis_conn().await?;
        let mut deleted: u64 = 0;
        for chunk in keys.chunks(DEL_BATCH) {
            let mut cmd = redis::cmd("DEL");
            for key in chunk {
                cmd.arg(key.as_str());
            }
            let n: u64 = cmd
                .query_async(&mut conn)
                .await
                .map_err(|e| Error::Cache(format!("Redis DEL 失败: {e}")))?;
            deleted += n;
        }
        Ok(deleted)
    }
}

/// 按 `AppConfig` 的 redis 配置建立一次性连接。
///
/// 复用与 `bin/src/main.rs` 中 `CacheService::init` 完全相同的连接串来源
/// （`config.yaml` + `TIG_*` 环境变量），保证指向同一实例。
async fn redis_conn() -> Result<ConnectionManager> {
    let cfg = AppConfig::load().map_err(|e| Error::Cache(format!("加载 Redis 配置失败: {e}")))?;
    let client = redis::Client::open(cfg.redis.url())
        .map_err(|e| Error::Cache(format!("Redis URL 无效: {e}")))?;
    ConnectionManager::new(client)
        .await
        .map_err(|e| Error::Cache(format!("Redis 连接失败: {e}")))
}

#[cfg(test)]
mod tests {
    // 需要真实 Redis 的集成测试见 tests/ 目录。
    // 本文件的纯逻辑（key 前缀过滤）在服务层 `cache_manage` 单测覆盖。
}