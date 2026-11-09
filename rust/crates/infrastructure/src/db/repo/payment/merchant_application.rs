//! 商户进件域仓储（payment::merchant_application）。
//!
//! 对应原 Java `MerchantApplicationMapper` + `ShopMapper` 的只读查询。
//!
//! 列名严格对齐 `python tools/schema_parser.py --table merchant_application`
//! （24 列，主键 `id` BIGINT）：`id` / `shop_id` / `applicant_type` /
//! `applicant_id` / `applicant_name` / `pay_channel` / `business_code` /
//! `applyment_id` / `organization_type` / `merchant_name` / `contact_name` /
//! `contact_phone` / `id_card_number_enc` / `bank_account_enc` / `status` /
//! `sub_mch_id` / `sign_url` / `reject_reason` / `audit_admin_id` /
//! `audit_time` / `request_snapshot` / `is_del` / `create_time` / `modify_time`。
//!
//! 另读 `shop`（只读 `shop_id` / `shop_title`）用于列表回填店铺名。
//! 查询统一使用运行时 sqlx（不用 `query!` 宏）。

use sqlx::{Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::MerchantApplication;
use nextrade_core::{Error, Result};

/// `merchant_application` 固定 SELECT 列清单（顺序与实体一致）。
const APPLICATION_SELECT: &str = "SELECT id, shop_id, applicant_type, applicant_id, applicant_name, pay_channel, business_code, applyment_id, organization_type, merchant_name, contact_name, contact_phone, id_card_number_enc, bank_account_enc, status, sub_mch_id, sign_url, reject_reason, audit_admin_id, audit_time::text AS audit_time, request_snapshot, is_del, create_time::text AS create_time, modify_time::text AS modify_time FROM merchant_application";

/// `list` 的过滤条件（Java `listPage` 的 wrapper 集合）。
#[derive(Debug, Clone, Default)]
pub struct MerchantApplicationFilter {
    /// 申请主体类型（非平台账号时恒为 `SHOP`）
    pub applicant_type: Option<String>,
    /// 申请主体 ID（非平台账号时恒为当前店铺）
    pub applicant_id: Option<i64>,
    /// 平台端按店铺过滤
    pub shop_id: Option<i64>,
    /// 状态
    pub status: Option<String>,
    /// 状态集合（`statusList`）
    pub status_list: Vec<String>,
}

/// 店铺名（Java `fillShopNames` 用）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct ShopTitleRow {
    pub shop_id: i32,
    pub shop_title: Option<String>,
}

/// 商户进件域仓储。
#[derive(Clone)]
pub struct MerchantApplicationRepository {
    pool: PgPool,
}

impl MerchantApplicationRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 按主键查询未删除申请（Java `getById`）。
    pub async fn find_by_id(&self, id: i64) -> Result<Option<MerchantApplication>> {
        sqlx::query_as::<_, MerchantApplication>(&format!(
            "{APPLICATION_SELECT} WHERE id = $1 AND is_del = 0 LIMIT 1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 分页（Java `listPage`；排序恒为 `id DESC`，覆盖前端 `sortField`）。
    pub async fn paginate(
        &self,
        f: &MerchantApplicationFilter,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<MerchantApplication>)> {
        let mut cq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM merchant_application WHERE is_del = 0");
        Self::push_filters(&mut cq, f);
        let total: (i64,) = cq
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut rq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{APPLICATION_SELECT} WHERE is_del = 0"));
        Self::push_filters(&mut rq, f);
        rq.push(" ORDER BY id DESC");
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<MerchantApplication> = rq
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0, rows))
    }

    fn push_filters(qb: &mut QueryBuilder<'_, Postgres>, f: &MerchantApplicationFilter) {
        if let Some(applicant_type) = f.applicant_type.as_deref() {
            qb.push(" AND applicant_type = ");
            qb.push_bind(applicant_type.to_string());
        }
        if let Some(applicant_id) = f.applicant_id {
            qb.push(" AND applicant_id = ");
            qb.push_bind(applicant_id);
        }
        if let Some(shop_id) = f.shop_id {
            qb.push(" AND shop_id = ");
            qb.push_bind(shop_id);
        }
        if let Some(status) = f.status.as_deref().filter(|s| !s.is_empty()) {
            qb.push(" AND status = ");
            qb.push_bind(status.to_string());
        }
        if !f.status_list.is_empty() {
            qb.push(" AND status IN (");
            let mut sep = qb.separated(", ");
            for status in &f.status_list {
                sep.push_bind(status.clone());
            }
            sep.push_unseparated(")");
            drop(sep);
        }
    }

    /// 当前主体最新一条申请（Java `current`）。
    ///
    /// `pay_channel` 非空时额外限定渠道；`only_finished = true` 时限定
    /// `status = FINISH`（Java `payChannel 非空` 分支）。
    pub async fn find_current(
        &self,
        applicant_type: &str,
        applicant_id: i64,
        pay_channel: Option<&str>,
        only_finished: bool,
    ) -> Result<Option<MerchantApplication>> {
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!(
            "{APPLICATION_SELECT} WHERE is_del = 0 AND applicant_type = "
        ));
        qb.push_bind(applicant_type.to_string());
        qb.push(" AND applicant_id = ");
        qb.push_bind(applicant_id);
        if let Some(channel) = pay_channel {
            qb.push(" AND pay_channel = ");
            qb.push_bind(channel.to_string());
        }
        if only_finished {
            qb.push(" AND status = ");
            qb.push_bind("FINISH".to_string());
        }
        qb.push(" ORDER BY id DESC LIMIT 1");
        qb.build_query_as()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 审核拒绝（Java `reject` 的 `updateById` 列：status / rejectReason /
    /// auditAdminId / auditTime / modifyTime）。
    pub async fn reject(
        &self,
        id: i64,
        reject_reason: &str,
        audit_admin_id: i32,
        now: &str,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE merchant_application SET status = 'REJECTED', reject_reason = $1, audit_admin_id = $2, audit_time = NULLIF($3, ''), modify_time = NULLIF($4, '') WHERE id = $5 AND is_del = 0",
        )
        .bind(reject_reason)
        .bind(audit_admin_id)
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 批量取店铺名（`fillShopNames`）。
    pub async fn find_shop_titles(&self, ids: &[i32]) -> Result<Vec<ShopTitleRow>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT shop_id, shop_title FROM shop WHERE shop_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_covers_all_24_columns() {
        let selected = APPLICATION_SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap();
        assert_eq!(selected.split(", ").count(), 24);
        assert!(APPLICATION_SELECT.ends_with("FROM merchant_application"));
    }

    #[test]
    fn filter_default_has_no_conditions() {
        let f = MerchantApplicationFilter::default();
        assert!(f.applicant_type.is_none());
        assert!(f.shop_id.is_none());
        assert!(f.status_list.is_empty());
    }

    #[test]
    fn shop_row_is_optional_title() {
        let row = ShopTitleRow::default();
        assert_eq!(row.shop_title, None);
    }
}
