//! 客户端发版登记仓储：`sys_client_release`（热更新 P4，我方新增）。
//!
//! 设计见 hot-update-design/REPORT.md §5.4。表由迁移 028 引入；**表可能尚未
//! 部署**：读方法在表缺失（PG 42P01）时返回空结果、写方法返回明确业务错误，
//! 与 system_update 仓储的降级口径一致（新后端先于迁移上线时页面可达不 500）。

use sqlx::PgPool;

use nextrade_core::models::system::client_release::ClientReleaseRow;
use nextrade_core::{Error, Result};

#[derive(Debug, Clone, sqlx::FromRow)]
struct ReleaseDbRow {
    id: i64,
    channel: String,
    product_version: String,
    client_version: String,
    note: String,
    operator_id: i64,
    operator_name: String,
    created_at: i64,
}

impl From<ReleaseDbRow> for ClientReleaseRow {
    fn from(r: ReleaseDbRow) -> Self {
        Self {
            id: r.id,
            channel: r.channel,
            product_version: r.product_version,
            client_version: r.client_version,
            note: r.note,
            operator_id: r.operator_id,
            operator_name: r.operator_name,
            created_at: r.created_at,
        }
    }
}

const COLUMNS: &str = "id, channel, product_version, client_version, note, \
    operator_id, operator_name, created_at";

/// 客户端发版登记仓储。
#[derive(Clone)]
pub struct ClientReleaseRepository {
    pool: PgPool,
}

impl ClientReleaseRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 插入一条登记，返回新行 id。表缺失时返回业务错误（不静默丢登记）。
    pub async fn insert(&self, row: &ClientReleaseRow) -> Result<i64> {
        let sql = format!(
            "INSERT INTO sys_client_release ({COLUMNS}) \
             VALUES (DEFAULT, $1, $2, $3, $4, $5, $6, $7) RETURNING id"
        );
        let id = sqlx::query_scalar::<_, i64>(&sql)
            .bind(&row.channel)
            .bind(&row.product_version)
            .bind(&row.client_version)
            .bind(&row.note)
            .bind(row.operator_id)
            .bind(&row.operator_name)
            .bind(row.created_at)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                if is_missing_table(&e) {
                    Error::biz_msg("发版登记表未就绪（迁移 028 未执行），请先完成部署迁移")
                } else {
                    map_err(e)
                }
            })?;
        Ok(id)
    }

    /// 某渠道最近一条登记（无则 None；表缺失降级 None）。
    pub async fn latest_by_channel(
        &self,
        channel: &str,
    ) -> Result<Option<ClientReleaseRow>> {
        let sql = format!(
            "SELECT {COLUMNS} FROM sys_client_release \
             WHERE channel = $1 ORDER BY created_at DESC, id DESC LIMIT 1"
        );
        let row = sqlx::query_as::<_, ReleaseDbRow>(&sql)
            .bind(channel)
            .fetch_optional(&self.pool)
            .await;
        match row {
            Ok(r) => Ok(r.map(ClientReleaseRow::from)),
            Err(e) if is_missing_table(&e) => Ok(None),
            Err(e) => Err(map_err(e)),
        }
    }

    /// 最近登记列表（跨渠道，时间倒序；表缺失降级空集）。
    pub async fn list_recent(&self, limit: i64) -> Result<Vec<ClientReleaseRow>> {
        let sql = format!(
            "SELECT {COLUMNS} FROM sys_client_release \
             ORDER BY created_at DESC, id DESC LIMIT $1"
        );
        let rows = sqlx::query_as::<_, ReleaseDbRow>(&sql)
            .bind(limit.clamp(1, 100))
            .fetch_all(&self.pool)
            .await;
        match rows {
            Ok(r) => Ok(r.into_iter().map(ClientReleaseRow::from).collect()),
            Err(e) if is_missing_table(&e) => Ok(Vec::new()),
            Err(e) => Err(map_err(e)),
        }
    }

    /// 按 id 删除一条登记（纠错用）。返回是否删到行；表缺失返回业务错误。
    pub async fn delete_by_id(&self, id: i64) -> Result<bool> {
        let affected = sqlx::query("DELETE FROM sys_client_release WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                if is_missing_table(&e) {
                    Error::biz_msg("发版登记表未就绪（迁移 028 未执行），请先完成部署迁移")
                } else {
                    map_err(e)
                }
            })?
            .rows_affected();
        Ok(affected > 0)
    }
}

/// PG `undefined_table`（42P01）：迁移 028 未部署时的降级判定。
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
