//! 装修分享模板域仓储（content::share）：`decorate_share`。
//!
//! 对应原 Java `DecorateShareMapper` / `DecorateShareServiceImpl#getInfoBySn`
//! 中「按 token 查分享记录」的一段查询。
//!
//! 列名严格对齐权威 schema `java/database/pure.sql`（校验方式：
//! `cd rust && python tools/schema_parser.py --table decorate_share`）：
//!
//! | 列 | 类型 | 说明 |
//! |----|------|------|
//! | `share_id` | int unsigned | 自增主键 |
//! | `share_sn` | char(6) | 分享 sn 码 |
//! | `share_token` | char(5) | 分享验证 token |
//! | `decorate_id` | int | 装修页面 id |
//! | `valid_time` | int | token 有效时间（秒级时间戳） |
//! | `create_time` | int | 添加时间 |
//! | `update_time` | int | 更新时间 |
//!
//! SQL 一律使用运行时 `sqlx::query_as::<_, T>(...)`，**不使用** `query!` 宏
//! （本机无 `DATABASE_URL`，宏会编译失败）；每条 SQL 写成单行字面量，
//! 便于 `python tools/check_sql_columns.py --strict` 静态校验列名。

use sqlx::PgPool;
use nextrade_core::entities::DecorateShare;
use nextrade_core::{Error, Result};

/// 装修分享模板域仓储（持有连接池）。
#[derive(Clone)]
pub struct ShareRepository {
    pool: PgPool,
}

impl ShareRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— decorate_share —————————————————————

    /// 按 `share_token` 查分享记录（不限有效期）。
    ///
    /// 对应 Java `DecorateShareServiceImpl#getInfoBySn` 中缓存未命中分支的
    /// `getOne(new LambdaQueryWrapper<DecorateShare>()
    ///          .eq(DecorateShare::getShareToken, token))`。
    ///
    /// 注意：Java 侧**只用 token 过滤**（`share_sn` 仅在缓存 key 中参与），
    /// 且未校验 `valid_time`，本方法如实复刻该语义；若同一 token 命中多条，
    /// 按 `share_id` 升序取第一条（对 MyBatis Plus `getOne` 的确定性替代，
    /// 避免多结果时抛 `TooManyResultsException` 导致接口 500）。
    pub async fn find_share_by_token(&self, token: &str) -> Result<Option<DecorateShare>> {
        sqlx::query_as::<_, DecorateShare>("SELECT * FROM decorate_share WHERE share_token = $1 ORDER BY share_id ASC LIMIT 1")
            .bind(token)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按 `share_sn` + `share_token` 查分享记录（不限有效期）。
    ///
    /// 用于 `sn`/`token` 组合精确匹配（缓存中 `sn + token` 为联合 key，
    /// 数据库侧给出等价的确定性查询，避免仅凭 token 命中他人分享）。
    pub async fn find_share_by_sn_token(
        &self,
        sn: &str,
        token: &str,
    ) -> Result<Option<DecorateShare>> {
        sqlx::query_as::<_, DecorateShare>("SELECT * FROM decorate_share WHERE share_sn = $1 AND share_token = $2 ORDER BY share_id ASC LIMIT 1")
            .bind(sn)
            .bind(token)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按 `share_sn` + `share_token` 查**仍在有效期内**的分享记录。
    ///
    /// `valid_time` 为秒级时间戳，语义为「该时刻之前有效」，
    /// 故用 `valid_time > ?`（与既有
    /// `nextrade_infrastructure::db::repo::content::decorate::DecorateRepository::find_valid_share`
    /// 的判据一致）。
    pub async fn find_valid_share_by_sn_token(
        &self,
        sn: &str,
        token: &str,
        now: i32,
    ) -> Result<Option<DecorateShare>> {
        sqlx::query_as::<_, DecorateShare>("SELECT * FROM decorate_share WHERE share_sn = $1 AND share_token = $2 AND valid_time > $3 ORDER BY share_id ASC LIMIT 1")
            .bind(sn)
            .bind(token)
            .bind(now)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按主键查分享记录。
    pub async fn find_share_by_id(&self, share_id: i32) -> Result<Option<DecorateShare>> {
        sqlx::query_as::<_, DecorateShare>("SELECT * FROM decorate_share WHERE share_id = $1 LIMIT 1")
            .bind(share_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }
}

/// SQL 错误 → 领域错误。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("分享链接不存在或已失效!".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    // 本文件全部为真实 MySQL 往返，纯逻辑（参数清洗/有效期判据）由
    // service 层 `#[cfg(test)]` 覆盖，此处不再重复。
}