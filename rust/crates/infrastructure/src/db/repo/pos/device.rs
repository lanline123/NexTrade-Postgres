//! 收银台设备（`pos_device`）仓储。
//!
//! Java 来源：参考工程（overseas / b2b2b-pro）**无** `PosDeviceController`
//! （grep `pos/device`、`PosDevice` 仅命中打印模块 `PrintPosDevice`，非同一表）。
//! 本仓储按 `pos_device` 真实 schema（`java/database/pure.sql`）+ 前端契约实现
//! MyBatis-Plus BaseMapper 风格的通用 CRUD。
//!
//! 列名对齐 `crates/core/src/entities/o2o.rs::PosDevice`。

use sqlx::{PgPool, QueryBuilder};

use nextrade_core::entities::o2o::PosDevice;
use nextrade_core::models::pos::device::PosDeviceListQuery;
use nextrade_core::{Error, Result};

/// `pos_device` 固定 SELECT 列（与 `PosDevice` 实体一一对应）。
const POS_DEVICE_SELECT: &str =
    "SELECT id, shop_id, device_name, device_code, status, last_used_time, add_time, update_time FROM pos_device";

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct PosDeviceRepository {
    pool: PgPool,
}

impl PosDeviceRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ————————————————————— 查询 —————————————————————

    /// 分页（`shop_scope = Some(id)` 时按店铺隔离；`None` 平台全量）。
    pub async fn paginate(
        &self,
        q: &PosDeviceListQuery,
        shop_scope: Option<i32>,
    ) -> Result<(u64, Vec<PosDevice>)> {
        let (limit, offset) = q.limit_offset();

        let mut count_qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM pos_device WHERE 1=1");
        Self::push_filters(&mut count_qb, q, shop_scope);
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut row_qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new(format!("{POS_DEVICE_SELECT} WHERE 1=1"));
        Self::push_filters(&mut row_qb, q, shop_scope);
        row_qb.push(format!(" ORDER BY id DESC LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<PosDevice> = row_qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 全量（`GET pos/device/all`，下拉用，最多 500 行）。
    pub async fn list_all(&self, shop_scope: Option<i32>) -> Result<Vec<PosDevice>> {
        let empty = PosDeviceListQuery::default();
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new(format!("{POS_DEVICE_SELECT} WHERE 1=1"));
        Self::push_filters(&mut qb, &empty, shop_scope);
        qb.push(" ORDER BY id DESC LIMIT 500");
        let rows: Vec<PosDevice> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows)
    }

    /// 按主键取设备。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<PosDevice>> {
        sqlx::query_as::<_, PosDevice>(&format!("{POS_DEVICE_SELECT} WHERE id = $1 LIMIT 1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    fn push_filters(
        qb: &mut QueryBuilder<'_, sqlx::Postgres>,
        q: &PosDeviceListQuery,
        shop_scope: Option<i32>,
    ) {
        if let Some(shop_id) = shop_scope {
            qb.push(" AND shop_id = ");
            qb.push_bind(shop_id);
        }
        if let Some(status) = q.status.filter(|s| *s > 0) {
            qb.push(" AND status = ");
            qb.push_bind(status);
        }
        if let Some(keyword) = q.keyword.as_deref().filter(|s| !s.trim().is_empty()) {
            let like = format!("%{}%", keyword.trim());
            qb.push(" AND (device_name LIKE ");
            qb.push_bind(like.clone());
            qb.push(" OR device_code LIKE ");
            qb.push_bind(like);
            qb.push(")");
        }
    }

    // ————————————————————— 写入 —————————————————————

    /// 创建设备；`(shop_id, device_code)` 唯一键冲突 → `Error::Conflict`。
    pub async fn insert(
        &self,
        shop_id: i32,
        device_name: &str,
        device_code: &str,
        status: i32,
        now: i64,
    ) -> Result<i32> {
        let out: i32 = sqlx::query_scalar(
            "INSERT INTO pos_device (shop_id, device_name, device_code, status, add_time, update_time) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        )
        .bind(shop_id)
        .bind(device_name)
        .bind(device_code)
        .bind(status)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db) = &e {
                if db.kind() == sqlx::error::ErrorKind::UniqueViolation {
                    return Error::Conflict("该店铺下设备码已存在".into());
                }
            }
            map_err(e)
        })?;
        Ok(out)
    }

    /// 更新设备基础字段（提交的 `Some` 才更新）。
    pub async fn update_fields(
        &self,
        id: i32,
        device_name: Option<&str>,
        device_code: Option<&str>,
        status: Option<i32>,
        now: i64,
    ) -> Result<u64> {
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("UPDATE pos_device SET update_time = ");
        qb.push_bind(now);
        if let Some(v) = device_name {
            qb.push(", device_name = ");
            qb.push_bind(v);
        }
        if let Some(v) = device_code {
            qb.push(", device_code = ");
            qb.push_bind(v);
        }
        if let Some(v) = status {
            qb.push(", status = ");
            qb.push_bind(v);
        }
        qb.push(" WHERE id = ");
        qb.push_bind(id);
        let out = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(out.rows_affected())
    }

    /// 白名单单字段更新（`pos/device/updateField`）。
    ///
    /// `field` 为蛇形列名，且必须落在 `POS_DEVICE_UPDATE_FIELDS` 白名单内
    /// （由服务层校验后传入）；`val` 直接绑定字符串，数值列由 MySQL 隐式转换。
    pub async fn update_single_field(
        &self,
        id: i32,
        field: &str,
        val: &str,
        now: i64,
    ) -> Result<u64> {
        // field 来自编译期白名单校验后的常量集合，此处仍只允许字母/下划线兜底
        debug_assert!(field.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
        let sql = format!(
            "UPDATE pos_device SET {field} = $1, update_time = $2 WHERE id = $3"
        );
        let out = sqlx::query(&sql)
            .bind(val)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(out.rows_affected())
    }

    /// 删除（物理删除；表无 is_del 列）。
    pub async fn delete_by_ids(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("DELETE FROM pos_device WHERE id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        let out = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(out.rows_affected())
    }
}

/// SQLx 错误 → 统一错误类型。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("收银设备不存在".into()),
        other => Error::Database(other.to_string()),
    }
}
