//! 商品复刻任务仓储（`product_clone_task`，2026-11-18 新增，
//! 见 `deploy/pg/2026-11-18_product_clone.sql`）。
//!
//! 全部运行时 `sqlx::query` / `query_as`（不用宏，本机无 DATABASE_URL）。

use sqlx::{PgPool, Row};

use nextrade_core::models::catalog::product_clone::CloneTaskVO;
use nextrade_core::{Error, Result};

const SELECT_COLS: &str = "id, platform, channel, source_url, source_item_id, status, title, draft_json, attempts, cost_fen, product_id, error_msg, created_by, create_time, update_time";

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct CloneTaskRow {
    pub id: i64,
    pub platform: String,
    pub channel: String,
    pub source_url: String,
    pub source_item_id: String,
    pub status: String,
    pub title: String,
    pub draft_json: Option<serde_json::Value>,
    pub attempts: serde_json::Value,
    pub cost_fen: i32,
    pub product_id: Option<i32>,
    pub error_msg: String,
    pub created_by: String,
    pub create_time: i64,
    pub update_time: i64,
}

impl CloneTaskRow {
    pub fn into_vo(self, with_draft: bool) -> CloneTaskVO {
        CloneTaskVO {
            id: self.id,
            platform: self.platform,
            channel: self.channel,
            source_url: self.source_url,
            source_item_id: self.source_item_id,
            status: self.status,
            title: self.title,
            draft: if with_draft {
                self.draft_json
                    .and_then(|v| serde_json::from_value(v).ok())
            } else {
                None
            },
            attempts: self.attempts,
            cost_fen: self.cost_fen,
            product_id: self.product_id,
            error_msg: self.error_msg,
            created_by: self.created_by,
            create_time: self.create_time,
            update_time: self.update_time,
        }
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[derive(Clone)]
pub struct ProductCloneRepository {
    pool: PgPool,
}

impl ProductCloneRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_task(
        &self,
        platform: &str,
        channel: &str,
        source_url: &str,
        source_item_id: &str,
        created_by: &str,
        now: i64,
    ) -> Result<i64> {
        let row = sqlx::query(
            "INSERT INTO product_clone_task (platform, channel, source_url, source_item_id, status, created_by, create_time, update_time) VALUES ($1,$2,$3,$4,'parsing',$5,$6,$6) RETURNING id",
        )
        .bind(platform)
        .bind(channel)
        .bind(source_url)
        .bind(source_item_id)
        .bind(created_by)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.get::<i64, _>("id"))
    }

    /// 解析完成/失败后回写：状态、标题、草稿、原始存档、尝试留痕、费用、错误。
    #[allow(clippy::too_many_arguments)]
    pub async fn finish_attempt(
        &self,
        id: i64,
        status: &str,
        title: &str,
        draft_json: Option<&serde_json::Value>,
        raw_snapshot: Option<&serde_json::Value>,
        attempts: &serde_json::Value,
        cost_fen: i32,
        error_msg: &str,
        now: i64,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE product_clone_task SET status=$2, title=$3, draft_json=$4, raw_snapshot=$5, attempts=$6, cost_fen=$7, error_msg=$8, update_time=$9 WHERE id=$1",
        )
        .bind(id)
        .bind(status)
        .bind(title)
        .bind(draft_json)
        .bind(raw_snapshot)
        .bind(attempts)
        .bind(cost_fen)
        .bind(error_msg)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 换通道重试前把任务打回 parsing 并更新通道。
    pub async fn reopen_for_retry(&self, id: i64, channel: &str, now: i64) -> Result<()> {
        sqlx::query(
            "UPDATE product_clone_task SET status='parsing', channel=$2, error_msg='', update_time=$3 WHERE id=$1",
        )
        .bind(id)
        .bind(channel)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn mark_imported(&self, id: i64, product_id: i32, now: i64) -> Result<()> {
        sqlx::query(
            "UPDATE product_clone_task SET status='imported', product_id=$2, error_msg='', update_time=$3 WHERE id=$1",
        )
        .bind(id)
        .bind(product_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn find_task(&self, id: i64) -> Result<Option<CloneTaskRow>> {
        let row = sqlx::query_as::<_, CloneTaskRow>(&format!(
            "SELECT {SELECT_COLS} FROM product_clone_task WHERE id=$1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row)
    }

    /// 同来源是否已有任务（查重提示用）：返回最近一条任务 id + 状态。
    pub async fn find_by_source(
        &self,
        platform: &str,
        source_item_id: &str,
    ) -> Result<Option<(i64, String, Option<i32>)>> {
        if source_item_id.is_empty() {
            return Ok(None);
        }
        let row = sqlx::query(
            "SELECT id, status, product_id FROM product_clone_task WHERE platform=$1 AND source_item_id=$2 ORDER BY id DESC LIMIT 1",
        )
        .bind(platform)
        .bind(source_item_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|r| {
            (
                r.get::<i64, _>("id"),
                r.get::<String, _>("status"),
                r.get::<Option<i32>, _>("product_id"),
            )
        }))
    }

    pub async fn list_tasks(
        &self,
        status: Option<&str>,
        platform: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<CloneTaskRow>, i64)> {
        let mut where_sql = String::from(" WHERE 1=1");
        if status.is_some() {
            where_sql.push_str(" AND status=$1");
        }
        // 组合占位符较繁琐，统一走 QueryBuilder
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(format!(
            "SELECT {SELECT_COLS} FROM product_clone_task WHERE 1=1"
        ));
        if let Some(s) = status {
            qb.push(" AND status=").push_bind(s.to_string());
        }
        if let Some(p) = platform {
            qb.push(" AND platform=").push_bind(p.to_string());
        }
        qb.push(" ORDER BY id DESC LIMIT ").push_bind(limit);
        qb.push(" OFFSET ").push_bind(offset);
        let rows = qb
            .build_query_as::<CloneTaskRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        let mut cq = sqlx::QueryBuilder::<sqlx::Postgres>::new(
            "SELECT COUNT(*) FROM product_clone_task WHERE 1=1",
        );
        if let Some(s) = status {
            cq.push(" AND status=").push_bind(s.to_string());
        }
        if let Some(p) = platform {
            cq.push(" AND platform=").push_bind(p.to_string());
        }
        let total: i64 = cq
            .build_query_scalar()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let _ = where_sql;
        Ok((rows, total))
    }

    /// 今日通道 A 已累计费用（分），日预算校验用。
    pub async fn today_api_cost_fen(&self, day_start_epoch: i64) -> Result<i64> {
        let total: Option<i64> = sqlx::query_scalar(
            "SELECT COALESCE(SUM(cost_fen),0)::bigint FROM product_clone_task WHERE create_time >= $1",
        )
        .bind(day_start_epoch)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(total.unwrap_or(0))
    }

    /// 按 SN 查商品 ID（导入落库后回填任务用；create() 只返回提示文案不返回 ID）。
    pub async fn find_product_id_by_sn(&self, product_sn: &str) -> Result<Option<i32>> {
        let id: Option<i32> = sqlx::query_scalar("SELECT product_id FROM product WHERE product_sn=$1")
            .bind(product_sn)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(id)
    }
}
