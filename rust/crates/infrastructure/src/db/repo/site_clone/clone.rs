//! 站点搬家仓储（site_clone::clone）：密钥 / 拉取审计 / 导入快照 /
//! 导入任务 / 去重映射五行表。
//!
//! SQL 一律运行时 `sqlx::query` / `query_as`，`$n` 占位（与 hub 仓储同法）。

use sqlx::PgPool;

use nextrade_core::models::site_clone::clone::{
    SiteCloneCallLog, SiteCloneImportJob, SiteCloneImportMap, SiteCloneKey, SiteCloneSnapshot,
};
use nextrade_core::{Error, Result};

/// 站点搬家仓储。
#[derive(Clone)]
pub struct SiteCloneRepository {
    pool: PgPool,
}

impl SiteCloneRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ———————————————————— 密钥 ————————————————————

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_key(
        &self,
        key_code: &str,
        key_name: &str,
        secret_hash: &str,
        scopes: &str,
        source_shop_id: i32,
        expire_time: i32,
        max_calls: i32,
        created_by: i32,
        created_by_name: &str,
        now: i32,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO site_clone_key (key_code, key_name, secret_hash, scopes, source_shop_id, expire_time, max_calls, used_calls, status, last_used_time, created_by, created_by_name, add_time, update_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, 0, 1, 0, $8, $9, $10, $10) RETURNING key_id",
        )
        .bind(key_code)
        .bind(key_name)
        .bind(secret_hash)
        .bind(scopes)
        .bind(source_shop_id)
        .bind(expire_time)
        .bind(max_calls)
        .bind(created_by)
        .bind(created_by_name)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    pub async fn list_keys(&self, page: u64, size: u64) -> Result<(Vec<SiteCloneKey>, i64)> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM site_clone_key")
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let offset = ((page - 1) * size) as i64;
        let rows = sqlx::query_as::<_, SiteCloneKey>(
            "SELECT * FROM site_clone_key ORDER BY key_id DESC LIMIT $1 OFFSET $2",
        )
        .bind(size as i64)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((rows, total.0.max(0)))
    }

    pub async fn find_key_by_id(&self, key_id: i32) -> Result<Option<SiteCloneKey>> {
        sqlx::query_as::<_, SiteCloneKey>("SELECT * FROM site_clone_key WHERE key_id = $1")
            .bind(key_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    pub async fn find_key_by_code(&self, key_code: &str) -> Result<Option<SiteCloneKey>> {
        sqlx::query_as::<_, SiteCloneKey>("SELECT * FROM site_clone_key WHERE key_code = $1")
            .bind(key_code)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_key(
        &self,
        key_id: i32,
        key_name: &str,
        scopes: &str,
        source_shop_id: i32,
        status: i16,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE site_clone_key SET key_name = $2, scopes = $3, source_shop_id = $4, status = $5, update_time = $6 WHERE key_id = $1",
        )
        .bind(key_id)
        .bind(key_name)
        .bind(scopes)
        .bind(source_shop_id)
        .bind(status)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn update_secret_hash(&self, key_id: i32, secret_hash: &str, now: i32) -> Result<()> {
        sqlx::query(
            "UPDATE site_clone_key SET secret_hash = $2, update_time = $3 WHERE key_id = $1",
        )
        .bind(key_id)
        .bind(secret_hash)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn set_key_status(&self, key_id: i32, status: i16, now: i32) -> Result<()> {
        sqlx::query("UPDATE site_clone_key SET status = $2, update_time = $3 WHERE key_id = $1")
            .bind(key_id)
            .bind(status)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 一次成功鉴权后计数 +1 并刷新最近使用时间。
    pub async fn touch_key_usage(&self, key_id: i32, now: i32) -> Result<()> {
        sqlx::query(
            "UPDATE site_clone_key SET used_calls = used_calls + 1, last_used_time = $2, update_time = $2 WHERE key_id = $1",
        )
        .bind(key_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    // ———————————————————— 拉取审计 ————————————————————

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_call_log(
        &self,
        key_id: i32,
        key_code: &str,
        endpoint: &str,
        domain: &str,
        http_status: i32,
        result_count: i32,
        client_ip: &str,
        duration_ms: i32,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO site_clone_call_log (key_id, key_code, endpoint, domain, http_status, result_count, client_ip, duration_ms, add_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(key_id)
        .bind(key_code)
        .bind(endpoint)
        .bind(domain)
        .bind(http_status)
        .bind(result_count)
        .bind(client_ip)
        .bind(duration_ms)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn list_call_logs(
        &self,
        key_id: Option<i32>,
        page: u64,
        size: u64,
    ) -> Result<(Vec<SiteCloneCallLog>, i64)> {
        let offset = ((page - 1) * size) as i64;
        let (total, rows) = if let Some(kid) = key_id {
            let total: (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM site_clone_call_log WHERE key_id = $1")
                    .bind(kid)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(map_err)?;
            let rows = sqlx::query_as::<_, SiteCloneCallLog>(
                "SELECT * FROM site_clone_call_log WHERE key_id = $1 ORDER BY log_id DESC LIMIT $2 OFFSET $3",
            )
            .bind(kid)
            .bind(size as i64)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
            (total.0, rows)
        } else {
            let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM site_clone_call_log")
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
            let rows = sqlx::query_as::<_, SiteCloneCallLog>(
                "SELECT * FROM site_clone_call_log ORDER BY log_id DESC LIMIT $1 OFFSET $2",
            )
            .bind(size as i64)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
            (total.0, rows)
        };
        Ok((rows, total.max(0)))
    }

    // ———————————————————— 导入快照 ————————————————————

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_snapshot(
        &self,
        job_id: i32,
        operator_id: i32,
        operator_name: &str,
        source_label: &str,
        fingerprint: &str,
        payload: &str,
        group_keys: &str,
        now: i32,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO site_clone_snapshot (job_id, operator_id, operator_name, source_label, fingerprint, payload, group_keys, status, rollback_time, add_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, 1, 0, $8) RETURNING snapshot_id",
        )
        .bind(job_id)
        .bind(operator_id)
        .bind(operator_name)
        .bind(source_label)
        .bind(fingerprint)
        .bind(payload)
        .bind(group_keys)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 快照列表（不取 payload 大字段，以空串占位）。
    pub async fn list_snapshots(
        &self,
        page: u64,
        size: u64,
    ) -> Result<(Vec<SiteCloneSnapshot>, i64)> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM site_clone_snapshot")
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let offset = ((page - 1) * size) as i64;
        let rows = sqlx::query_as::<_, SiteCloneSnapshot>(
            "SELECT snapshot_id, job_id, operator_id, operator_name, source_label, fingerprint, '' AS payload, group_keys, status, rollback_time, add_time
             FROM site_clone_snapshot ORDER BY snapshot_id DESC LIMIT $1 OFFSET $2",
        )
        .bind(size as i64)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((rows, total.0.max(0)))
    }

    pub async fn find_snapshot(&self, snapshot_id: i32) -> Result<Option<SiteCloneSnapshot>> {
        sqlx::query_as::<_, SiteCloneSnapshot>(
            "SELECT * FROM site_clone_snapshot WHERE snapshot_id = $1",
        )
        .bind(snapshot_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn mark_snapshot_rolled_back(&self, snapshot_id: i32, now: i32) -> Result<()> {
        sqlx::query(
            "UPDATE site_clone_snapshot SET status = 2, rollback_time = $2 WHERE snapshot_id = $1",
        )
        .bind(snapshot_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 只保留最近 `keep` 份快照（设计口径：最近 10 份）。
    pub async fn prune_snapshots(&self, keep: i32) -> Result<()> {
        sqlx::query(
            "DELETE FROM site_clone_snapshot WHERE snapshot_id NOT IN
             (SELECT snapshot_id FROM site_clone_snapshot ORDER BY snapshot_id DESC LIMIT $1)",
        )
        .bind(keep)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    // ———————————————————— 导入任务 ————————————————————

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_job(
        &self,
        fingerprint: &str,
        source_origin: &str,
        target_shop_id: i32,
        domains: &str,
        options: &str,
        operator_id: i32,
        operator_name: &str,
        now: i32,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO site_clone_import_job (fingerprint, source_origin, source_label, target_shop_id, domains, options, status, snapshot_id, counts, failures, operator_id, operator_name, undone, add_time, finish_time)
             VALUES ($1, $2, '', $3, $4, $5, 'running', 0, '', '', $6, $7, 0, $8, 0) RETURNING job_id",
        )
        .bind(fingerprint)
        .bind(source_origin)
        .bind(target_shop_id)
        .bind(domains)
        .bind(options)
        .bind(operator_id)
        .bind(operator_name)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn finish_job(
        &self,
        job_id: i32,
        status: &str,
        source_label: &str,
        snapshot_id: i32,
        counts: &str,
        failures: &str,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE site_clone_import_job SET status = $2, source_label = $3, snapshot_id = $4, counts = $5, failures = $6, finish_time = $7 WHERE job_id = $1",
        )
        .bind(job_id)
        .bind(status)
        .bind(source_label)
        .bind(snapshot_id)
        .bind(counts)
        .bind(failures)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn list_jobs(&self, page: u64, size: u64) -> Result<(Vec<SiteCloneImportJob>, i64)> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM site_clone_import_job")
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let offset = ((page - 1) * size) as i64;
        let rows = sqlx::query_as::<_, SiteCloneImportJob>(
            "SELECT * FROM site_clone_import_job ORDER BY job_id DESC LIMIT $1 OFFSET $2",
        )
        .bind(size as i64)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((rows, total.0.max(0)))
    }

    pub async fn find_job(&self, job_id: i32) -> Result<Option<SiteCloneImportJob>> {
        sqlx::query_as::<_, SiteCloneImportJob>(
            "SELECT * FROM site_clone_import_job WHERE job_id = $1",
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn mark_job_undone(&self, job_id: i32) -> Result<()> {
        sqlx::query("UPDATE site_clone_import_job SET undone = 1 WHERE job_id = $1")
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    // ———————————————————— 去重映射 ————————————————————

    pub async fn find_map(
        &self,
        fingerprint: &str,
        domain: &str,
        source_id: i32,
    ) -> Result<Option<SiteCloneImportMap>> {
        sqlx::query_as::<_, SiteCloneImportMap>(
            "SELECT * FROM site_clone_import_map WHERE fingerprint = $1 AND domain = $2 AND source_id = $3",
        )
        .bind(fingerprint)
        .bind(domain)
        .bind(source_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_map(
        &self,
        fingerprint: &str,
        domain: &str,
        source_id: i32,
        source_key: &str,
        target_id: i32,
        job_id: i32,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO site_clone_import_map (fingerprint, domain, source_id, source_key, target_id, job_id, add_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (fingerprint, domain, source_id) DO NOTHING",
        )
        .bind(fingerprint)
        .bind(domain)
        .bind(source_id)
        .bind(source_key)
        .bind(target_id)
        .bind(job_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn list_maps_by_job(&self, job_id: i32) -> Result<Vec<SiteCloneImportMap>> {
        sqlx::query_as::<_, SiteCloneImportMap>(
            "SELECT * FROM site_clone_import_map WHERE job_id = $1 ORDER BY map_id ASC",
        )
        .bind(job_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn count_maps(&self, fingerprint: &str, domain: &str) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM site_clone_import_map WHERE fingerprint = $1 AND domain = $2",
        )
        .bind(fingerprint)
        .bind(domain)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    /// 撤销导入后清掉该任务的商品映射（商品已删，映射失效；
    /// 结构域映射保留——承接对象仍在，见服务层撤销口径）。
    pub async fn delete_product_maps_by_job(&self, job_id: i32) -> Result<()> {
        sqlx::query(
            "DELETE FROM site_clone_import_map WHERE job_id = $1 AND domain = 'product'",
        )
        .bind(job_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    // ———————————————————— config 原始行（快照回滚删除用） ————————————————————

    /// 删除一个 config 行（回滚时源站/本站原先不存在该块的还原动作）。
    pub async fn delete_config_code(&self, code: &str) -> Result<()> {
        sqlx::query("DELETE FROM config WHERE biz_code = $1")
            .bind(code)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}
