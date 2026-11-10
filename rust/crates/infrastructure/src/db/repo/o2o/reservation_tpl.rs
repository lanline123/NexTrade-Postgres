//! 预约模板（`reservation_tpl`）仓储。
//!
//! 对应原 Java（overseas）：`com.nextrade.mapper.o2o.ReservationTplMapper`
//! （BaseMapper 通用查询/写入；参考工程无 XML 自定义语句）。
//! 列名对齐 `crates/core/src/entities/o2o.rs::ReservationTpl`。

use sqlx::{PgPool, QueryBuilder};

use nextrade_core::entities::o2o::ReservationTpl;
use nextrade_core::models::o2o::reservation_tpl::{ReservationTplListQuery, ReservationTplWritePayload};
use nextrade_core::{Error, Result};

/// `reservation_tpl` 固定 SELECT 列。
const RESERVATION_TPL_SELECT: &str = "SELECT reservation_tpl_id, template_name, reservation_type, advance_number, advance_unit, max_days, shop_id, is_default FROM reservation_tpl";

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct ReservationTplRepository {
    pool: PgPool,
}

impl ReservationTplRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 分页（`shop_scope = Some(id)` 时按店铺隔离；平台全量；名称模糊）。
    pub async fn paginate(
        &self,
        q: &ReservationTplListQuery,
        shop_scope: Option<i32>,
    ) -> Result<(u64, Vec<ReservationTpl>)> {
        let (limit, offset) = q.limit_offset();

        let mut count_qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM reservation_tpl WHERE 1=1");
        Self::push_filters(&mut count_qb, q, shop_scope);
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut row_qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new(format!("{RESERVATION_TPL_SELECT} WHERE 1=1"));
        Self::push_filters(&mut row_qb, q, shop_scope);
        row_qb.push(format!(
            " ORDER BY reservation_tpl_id DESC LIMIT {limit} OFFSET {offset}"
        ));
        let rows: Vec<ReservationTpl> = row_qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 全量（`GET o2o/reservationTpl/list`，下拉用）。
    pub async fn list_all(&self, shop_scope: Option<i32>) -> Result<Vec<ReservationTpl>> {
        let q = ReservationTplListQuery::default();
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new(format!("{RESERVATION_TPL_SELECT} WHERE 1=1"));
        Self::push_filters(&mut qb, &q, shop_scope);
        qb.push(" ORDER BY reservation_tpl_id DESC LIMIT 500");
        let rows: Vec<ReservationTpl> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows)
    }

    fn push_filters(
        qb: &mut QueryBuilder<'_, sqlx::Postgres>,
        q: &ReservationTplListQuery,
        shop_scope: Option<i32>,
    ) {
        if let Some(shop_id) = shop_scope {
            qb.push(" AND shop_id = ");
            qb.push_bind(shop_id);
        }
        if let Some(keyword) = q.keyword.as_deref().filter(|s| !s.trim().is_empty()) {
            qb.push(" AND template_name ILIKE ");
            qb.push_bind(format!("%{}%", keyword.trim()));
        }
    }

    /// 按主键取模板。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<ReservationTpl>> {
        sqlx::query_as::<_, ReservationTpl>(&format!(
            "{RESERVATION_TPL_SELECT} WHERE reservation_tpl_id = $1 LIMIT 1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 创建；返回自增 id。
    pub async fn insert(&self, p: &ReservationTplWritePayload, shop_id: i32) -> Result<i32> {
        let out: i32 = sqlx::query_scalar(
            "INSERT INTO reservation_tpl (template_name, reservation_type, advance_number, advance_unit, max_days, shop_id, is_default) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING reservation_tpl_id",
        )
        .bind(p.template_name.as_deref().unwrap_or(""))
        .bind(p.reservation_type.unwrap_or(0))
        .bind(p.advance_number.unwrap_or(0))
        .bind(p.advance_unit.as_deref().unwrap_or("d"))
        .bind(p.max_days.unwrap_or(0))
        .bind(shop_id)
        .bind(p.is_default.unwrap_or(0))
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(out)
    }

    /// 更新（提交的 `Some` 字段才更新）。
    pub async fn update(&self, p: &ReservationTplWritePayload) -> Result<u64> {
        let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
            "UPDATE reservation_tpl SET reservation_tpl_id = reservation_tpl_id",
        );
        if let Some(v) = &p.template_name {
            qb.push(", template_name = ");
            qb.push_bind(v.clone());
        }
        if let Some(v) = p.reservation_type {
            qb.push(", reservation_type = ");
            qb.push_bind(v);
        }
        if let Some(v) = p.advance_number {
            qb.push(", advance_number = ");
            qb.push_bind(v);
        }
        if let Some(v) = &p.advance_unit {
            qb.push(", advance_unit = ");
            qb.push_bind(v.clone());
        }
        if let Some(v) = p.max_days {
            qb.push(", max_days = ");
            qb.push_bind(v);
        }
        if let Some(v) = p.is_default {
            qb.push(", is_default = ");
            qb.push_bind(v);
        }
        qb.push(" WHERE reservation_tpl_id = ");
        qb.push_bind(p.reservation_tpl_id.unwrap_or(0));
        let out = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(out.rows_affected())
    }

    /// 物理删除（表无 is_del 列）。
    pub async fn delete(&self, id: i32) -> Result<u64> {
        let out = sqlx::query("DELETE FROM reservation_tpl WHERE reservation_tpl_id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(out.rows_affected())
    }
}

/// SQLx 错误 → 统一错误类型。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("预约模板不存在".into()),
        other => Error::Database(other.to_string()),
    }
}
