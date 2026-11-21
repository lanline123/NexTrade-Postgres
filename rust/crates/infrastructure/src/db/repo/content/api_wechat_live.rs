//! 微信视频号直播记录域仓储（content::api_wechat_live）。
//!
//! 表 `finder_live_record`（迁移 028，我方新增落库——原版 Java 该接口
//! 只代理微信开放接口、不落库）：每次拉取到当前直播列表后整批刷新——
//! 本批记录按 `export_id` upsert 并置 `is_current = 1`，此前标当前但
//! 本批未再出现的置 `is_current = 0`（已结束/不再当前），供管理端落库
//! 查询与历史追溯。

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

use nextrade_core::models::content::api_wechat_live::FinderLiveRecordVO;
use nextrade_core::{Error, Result};

/// `finder_live_record` 行（与迁移 028 的表结构逐字对齐）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct FinderLiveRecordRow {
    /// 主键。
    pub id: i32,
    /// 微信直播 id（export_id），唯一键。
    pub export_id: String,
    /// 直播描述。
    pub description: String,
    /// 直播封面。
    pub cover_url: String,
    /// 开播视频号昵称。
    pub nickname: String,
    /// 开播视频号头像。
    pub head_url: String,
    /// 1 = 最近一次拉取仍在当前直播列表中。
    pub is_current: i32,
    /// 首次拉取到（秒级时间戳）。
    pub first_seen_time: i32,
    /// 最近一次拉取到（秒级时间戳）。
    pub last_seen_time: i32,
    /// 记录创建时间（秒级时间戳）。
    pub add_time: i32,
    /// 记录更新时间（秒级时间戳）。
    pub update_time: i32,
}

impl FinderLiveRecordRow {
    /// 行 → VO（空串归一为 None，与微信响应缺字段口径一致）。
    #[must_use]
    pub fn to_vo(&self) -> FinderLiveRecordVO {
        fn nz(s: &str) -> Option<String> {
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        }
        FinderLiveRecordVO {
            export_id: nz(&self.export_id),
            description: nz(&self.description),
            cover_url: nz(&self.cover_url),
            nickname: nz(&self.nickname),
            head_url: nz(&self.head_url),
        }
    }
}

/// 微信视频号直播记录仓储。
#[derive(Clone)]
pub struct ApiWechatLiveRepository {
    pool: PgPool,
}

impl ApiWechatLiveRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（与其它仓储一致）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 以一次拉取结果刷新落库（单事务）：
    /// 1. 现有当前记录全部置 `is_current = 0`；
    /// 2. 本批逐条按 `export_id` upsert（字段覆盖、置当前、刷新
    ///    last_seen；`first_seen_time` 保留首见值）。`export_id` 为空的
    ///    条目跳过——无稳定键不可落库（微信正常响应恒有 export_id）。
    ///
    /// # Errors
    /// 数据库错误。
    pub async fn replace_current(&self, records: &[FinderLiveRecordVO], now: i32) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        sqlx::query(
            "UPDATE finder_live_record SET is_current = 0, update_time = $1 WHERE is_current = 1",
        )
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        for r in records {
            let Some(export_id) = r
                .export_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                continue;
            };
            sqlx::query(
                "INSERT INTO finder_live_record
                   (export_id, description, cover_url, nickname, head_url,
                    is_current, first_seen_time, last_seen_time, add_time, update_time)
                 VALUES ($1, $2, $3, $4, $5, 1, $6, $6, $6, $6)
                 ON CONFLICT (export_id) DO UPDATE SET
                   description = EXCLUDED.description,
                   cover_url = EXCLUDED.cover_url,
                   nickname = EXCLUDED.nickname,
                   head_url = EXCLUDED.head_url,
                   is_current = 1,
                   last_seen_time = EXCLUDED.last_seen_time,
                   update_time = EXCLUDED.update_time",
            )
            .bind(export_id)
            .bind(r.description.as_deref().unwrap_or(""))
            .bind(r.cover_url.as_deref().unwrap_or(""))
            .bind(r.nickname.as_deref().unwrap_or(""))
            .bind(r.head_url.as_deref().unwrap_or(""))
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        }
        tx.commit().await.map_err(map_err)?;
        Ok(())
    }

    /// 落库分页查询：当前记录优先、最近拉取在前。
    ///
    /// # Errors
    /// 数据库错误。
    pub async fn paginate(
        &self,
        only_current: bool,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<FinderLiveRecordRow>)> {
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM finder_live_record
             WHERE ($1::bool = false OR is_current = 1)",
        )
        .bind(only_current)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let rows = sqlx::query_as::<_, FinderLiveRecordRow>(
            "SELECT * FROM finder_live_record
             WHERE ($1::bool = false OR is_current = 1)
             ORDER BY is_current DESC, last_seen_time DESC, id DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(only_current)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_type_exists() {
        assert!(std::any::type_name::<ApiWechatLiveRepository>().contains("ApiWechatLive"));
    }

    #[test]
    fn row_to_vo_normalizes_empty_strings() {
        let row = FinderLiveRecordRow {
            export_id: "exp-1".into(),
            nickname: "昵称".into(),
            ..Default::default()
        };
        let vo = row.to_vo();
        assert_eq!(vo.export_id.as_deref(), Some("exp-1"));
        assert_eq!(vo.nickname.as_deref(), Some("昵称"));
        assert!(vo.description.is_none());
        assert!(vo.cover_url.is_none());
        assert!(vo.head_url.is_none());
    }
}
