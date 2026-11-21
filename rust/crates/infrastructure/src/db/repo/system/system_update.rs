//! 热更新域（system::system_update）仓储：`sys_schema_migration` + `sys_update_job`。
//!
//! 我方新增（admin 自助热更新 P1/P2，设计见 hot-update-design/REPORT.md §4.1/§5）。
//! 两表由迁移 027 引入。**表可能尚未部署**：所有读方法在表缺失（PG 42P01）时
//! 返回空结果而非报错，由服务层据此判定「基线未就绪」并降级只读展示——
//! 这样新后端先于迁移 027 上线时页面可达、不 500。
//!
//! 查询统一使用运行时 `sqlx::query_as`（不使用 `query!` 宏）。

use sqlx::PgPool;
use nextrade_core::models::system::system_update::UpdateJobRow;
use nextrade_core::{Error, Result};

/// 进行中状态集合（与状态机一致；找活跃 job 用）。
const ACTIVE_STATUSES: [&str; 7] = [
    "queued",
    "downloading",
    "verifying",
    "backing_up",
    "swapping",
    "health_checking",
    "rolling_back",
];

#[derive(Debug, Clone, sqlx::FromRow)]
struct JobDbRow {
    job_id: String,
    product_version: String,
    manifest_sha256: String,
    manifest_snapshot: String,
    surfaces: String,
    status: String,
    stage: String,
    message: String,
    operator_id: i64,
    operator_name: String,
    destructive_ack: bool,
    created_at: i64,
    updated_at: i64,
}

impl From<JobDbRow> for UpdateJobRow {
    fn from(r: JobDbRow) -> Self {
        Self {
            job_id: r.job_id,
            product_version: r.product_version,
            manifest_sha256: r.manifest_sha256,
            manifest_snapshot: r.manifest_snapshot,
            surfaces: r.surfaces,
            status: r.status,
            stage: r.stage,
            message: r.message,
            operator_id: r.operator_id,
            operator_name: r.operator_name,
            destructive_ack: r.destructive_ack,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

const JOB_COLUMNS: &str = "job_id, product_version, manifest_sha256, manifest_snapshot, \
    surfaces, status, stage, message, operator_id, operator_name, destructive_ack, \
    created_at, updated_at";

/// 热更新仓储。
#[derive(Clone)]
pub struct SystemUpdateRepository {
    pool: PgPool,
}

impl SystemUpdateRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 已登记迁移（filename → sha256）。表缺失时返回空表（基线未就绪口径）。
    pub async fn list_applied_migrations(&self) -> Result<Vec<(String, String)>> {
        let rows = sqlx::query_as::<_, (String, String)>(
            "SELECT filename, sha256 FROM sys_schema_migration ORDER BY filename",
        )
        .fetch_all(&self.pool)
        .await;
        match rows {
            Ok(r) => Ok(r),
            Err(e) if is_missing_table(&e) => Ok(Vec::new()),
            Err(e) => Err(map_err(e)),
        }
    }

    /// 插入一条 job 审计行（状态恒 `queued`，由服务层组装后传入）。
    pub async fn insert_job(&self, row: &UpdateJobRow) -> Result<()> {
        sqlx::query(
            "INSERT INTO sys_update_job (job_id, product_version, manifest_sha256, manifest_snapshot, surfaces, status, stage, message, operator_id, operator_name, destructive_ack, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(&row.job_id)
        .bind(&row.product_version)
        .bind(&row.manifest_sha256)
        .bind(&row.manifest_snapshot)
        .bind(&row.surfaces)
        .bind(&row.status)
        .bind(&row.stage)
        .bind(&row.message)
        .bind(row.operator_id)
        .bind(&row.operator_name)
        .bind(row.destructive_ack)
        .bind(row.created_at)
        .bind(row.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 按 id 查 job（表缺失/不存在返回 None）。
    pub async fn find_job(&self, job_id: &str) -> Result<Option<UpdateJobRow>> {
        let sql = format!("SELECT {JOB_COLUMNS} FROM sys_update_job WHERE job_id = $1");
        let row = sqlx::query_as::<_, JobDbRow>(&sql)
            .bind(job_id)
            .fetch_optional(&self.pool)
            .await;
        match row {
            Ok(r) => Ok(r.map(UpdateJobRow::from)),
            Err(e) if is_missing_table(&e) => Ok(None),
            Err(e) => Err(map_err(e)),
        }
    }

    /// 最近的 job 列表（创建时间倒序）。
    pub async fn list_jobs(&self, limit: i64) -> Result<Vec<UpdateJobRow>> {
        let sql = format!(
            "SELECT {JOB_COLUMNS} FROM sys_update_job ORDER BY created_at DESC LIMIT $1"
        );
        let rows = sqlx::query_as::<_, JobDbRow>(&sql)
            .bind(limit.clamp(1, 100))
            .fetch_all(&self.pool)
            .await;
        match rows {
            Ok(r) => Ok(r.into_iter().map(UpdateJobRow::from).collect()),
            Err(e) if is_missing_table(&e) => Ok(Vec::new()),
            Err(e) => Err(map_err(e)),
        }
    }

    /// 当前进行中的 job（至多一条；全局单 job 由服务层 + updater flock 双保险）。
    pub async fn find_active_job(&self) -> Result<Option<UpdateJobRow>> {
        let sql = format!(
            "SELECT {JOB_COLUMNS} FROM sys_update_job \
             WHERE status IN ('queued','downloading','verifying','backing_up','swapping','health_checking','rolling_back') \
             ORDER BY created_at DESC LIMIT 1"
        );
        let row = sqlx::query_as::<_, JobDbRow>(&sql)
            .fetch_optional(&self.pool)
            .await;
        match row {
            Ok(r) => Ok(r.map(UpdateJobRow::from)),
            Err(e) if is_missing_table(&e) => Ok(None),
            Err(e) => Err(map_err(e)),
        }
    }

    /// 回写 job 进度（updater status.json 同步 + 失败标记共用）。
    /// 只前进不回退终态：已 `done` 的行不被覆盖（防迟到的状态文件把结果改掉）。
    pub async fn update_job_progress(
        &self,
        job_id: &str,
        status: &str,
        stage: &str,
        message: &str,
        updated_at: i64,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE sys_update_job SET status = $2, stage = $3, message = $4, updated_at = $5 WHERE job_id = $1 AND status <> 'done'",
        )
        .bind(job_id)
        .bind(status)
        .bind(stage)
        .bind(message)
        .bind(updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }
}

/// PG `undefined_table`（42P01）：迁移 027 未部署时的降级判定。
fn is_missing_table(e: &sqlx::Error) -> bool {
    match e {
        sqlx::Error::Database(db) => db.code().as_deref() == Some("42P01"),
        _ => false,
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("记录不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_statuses_cover_state_machine() {
        // 状态机进行中集合与 VO 的 is_active 口径一致（防两处漂移）
        for s in ACTIVE_STATUSES {
            let vo = nextrade_core::models::system::system_update::UpdateJobVO {
                status: s.to_string(),
                ..Default::default()
            };
            assert!(vo.is_active(), "状态 {s} 应为进行中");
        }
    }
}
