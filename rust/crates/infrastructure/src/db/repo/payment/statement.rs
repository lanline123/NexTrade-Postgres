//! 对账单仓储（payment::statement）。
//!
//! 表：`statement` / `statement_download` / `shop` / `vendor`。
//!
//! 全部 SQL 为运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`（禁止 `query!` 宏）。

use rust_decimal::Decimal;
use sqlx::PgPool;
use sqlx::{Postgres, QueryBuilder};

use nextrade_core::entities::finance::{Statement, StatementDownload};
use nextrade_core::models::payment::statement::{parse_datetime, StatementListQuery};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct StatementRepository {
    pool: PgPool,
}

impl StatementRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 对账单分页（Java `getStatementList` 过滤条件）。
    pub async fn paginate(
        &self,
        q: &StatementListQuery,
        page: &PageQuery,
    ) -> Result<(u64, Vec<Statement>)> {
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new("SELECT COUNT(*) FROM statement WHERE 1 = 1");
        self.push_filters(&mut qb, q);
        let total: (i64,) = qb.build_query_as().fetch_one(&self.pool).await.map_err(map_err)?;

        let (limit, offset) = page.limit_offset();
        let dir = if q.ascending() { "ASC" } else { "DESC" };
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new("SELECT * FROM statement WHERE 1 = 1");
        self.push_filters(&mut qb, q);
        qb.push(format!(" ORDER BY statement_id {dir} LIMIT "));
        qb.push_bind(limit);
        qb.push(" OFFSET ");
        qb.push_bind(offset);
        let rows = qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    fn push_filters<'a>(&self, qb: &mut QueryBuilder<'a, Postgres>, q: &StatementListQuery) {
        if let Some(sn) = q.record_sn.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            qb.push(" AND record_sn ILIKE ");
            qb.push_bind(format!("%{sn}%"));
        }
        if let (Some(start), Some(end)) = (q.start_date_time.as_deref(), q.end_date_time.as_deref()) {
            let (start, end) = nextrade_core::models::payment::statement::normalize_range(
                parse_datetime(start),
                parse_datetime(end),
            );
            if start > 0 && end > 0 {
                match q.time_type {
                    Some(2) => {
                        qb.push(" AND record_time >= ");
                        qb.push_bind(start);
                        qb.push(" AND record_time <= ");
                        qb.push_bind(end);
                    }
                    _ => {
                        qb.push(" AND settlement_time >= ");
                        qb.push_bind(start);
                        qb.push(" AND settlement_time <= ");
                        qb.push_bind(end);
                    }
                }
            }
        }
        if let Some(account_type) = q.account_type {
            qb.push(" AND account_type = ");
            qb.push_bind(account_type);
        }
        if let Some(t) = q.r#type.filter(|v| *v != 0) {
            qb.push(" AND type = ");
            qb.push_bind(t);
        }
        if let Some(payment_type) = q.payment_type.as_deref().filter(|s| !s.is_empty()) {
            qb.push(" AND payment_type = ");
            qb.push_bind(payment_type.to_string());
        }
        if let Some(shop_id) = q.shop_id.filter(|v| *v > 0) {
            qb.push(" AND shop_id = ");
            qb.push_bind(shop_id);
        }
        if let Some(vendor_id) = q.vendor_id.filter(|v| *v > 0) {
            qb.push(" AND vendor_id = ");
            qb.push_bind(vendor_id);
        }
        match q.source.as_deref() {
            Some("shop") => {
                qb.push(" AND shop_id > 0");
            }
            Some("vendor") => {
                qb.push(" AND vendor_id > 0");
            }
            _ => {}
        }
    }

    /// 统计聚合（按 年/月/日 + 账户类型 + 来源维度）。
    ///
    /// 返回 `(income, expenditure, income_count, expenditure_count)`：
    /// 收入 = `amount > 0`，支出 = `amount < 0`（绝对值和）。
    pub async fn aggregate(
        &self,
        year: i32,
        month: i32,
        day: i32,
        account_type: Option<i32>,
        shop_id: Option<i32>,
        vendor_id: Option<i32>,
    ) -> Result<(Decimal, Decimal, i64, i64)> {
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT COALESCE(SUM(CASE WHEN amount > 0 THEN amount ELSE 0 END),0),
                    COALESCE(SUM(CASE WHEN amount < 0 THEN -amount ELSE 0 END),0),
                    CAST(COALESCE(SUM(CASE WHEN amount > 0 THEN 1 ELSE 0 END),0) AS BIGINT),
                    CAST(COALESCE(SUM(CASE WHEN amount < 0 THEN 1 ELSE 0 END),0) AS BIGINT)
             FROM statement WHERE statement_year = ",
        );
        qb.push_bind(year);
        if month > 0 {
            qb.push(" AND statement_month = ");
            qb.push_bind(month);
        }
        if day > 0 {
            qb.push(" AND statement_day = ");
            qb.push_bind(day);
        }
        if let Some(account_type) = account_type {
            qb.push(" AND account_type = ");
            qb.push_bind(account_type);
        }
        if let Some(shop_id) = shop_id.filter(|v| *v > 0) {
            qb.push(" AND shop_id = ");
            qb.push_bind(shop_id);
        }
        if let Some(vendor_id) = vendor_id.filter(|v| *v > 0) {
            qb.push(" AND vendor_id = ");
            qb.push_bind(vendor_id);
        }
        let row: (Option<Decimal>, Option<Decimal>, i64, i64) =
            qb.build_query_as().fetch_one(&self.pool).await.map_err(map_err)?;
        Ok((
            row.0.unwrap_or_default(),
            row.1.unwrap_or_default(),
            row.2,
            row.3,
        ))
    }

    /// 店铺名称（Java `shopMapper.selectById(shopId).shopTitle`）。
    pub async fn shop_title(&self, shop_id: i32) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT shop_title FROM shop WHERE shop_id = $1 LIMIT 1")
                .bind(shop_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }

    /// 供应商名称（Java `vendorMapper.selectOnboardedById(vendorId).vendorName`）。
    pub async fn vendor_name(&self, vendor_id: i32) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT vendor_name FROM vendor WHERE vendor_id = $1 LIMIT 1")
                .bind(vendor_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }

    /// 保存对账单下载信息（Java `saveStatementDownload`）。
    pub async fn insert_download(
        &self,
        start_time: i64,
        end_time: i64,
        shop_id: Option<i64>,
        vendor_id: Option<i32>,
        remark: Option<&str>,
        gmt_create: i64,
    ) -> Result<i64> {
        let r: i64 = sqlx::query_scalar(
            "INSERT INTO statement_download (vendor_id, shop_id, gmt_create, start_time, end_time, remark)
             VALUES ($1,$2,$3,$4,$5,$6) RETURNING statement_download_id",
        )
        .bind(vendor_id)
        .bind(shop_id.map(|v| v as i32))
        .bind(gmt_create)
        .bind(start_time)
        .bind(end_time)
        .bind(remark)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 下载记录（供调试/后续下载列表复用；本切片不注册端点）。
    pub async fn find_download(&self, id: i64) -> Result<Option<StatementDownload>> {
        sqlx::query_as::<_, StatementDownload>(
            "SELECT * FROM statement_download WHERE statement_download_id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 插入出账单明细（Java batchSaveStatementOutput）。
    pub async fn insert_statement_output(&self, row: &nextrade_core::entities::StatementOutput) -> Result<i64> {
        let r: i64 = sqlx::query_scalar(
            "INSERT INTO statement_output (shop_id, vendor_id, income, expenditure, gmt_create, record_sn, record_type, record_id, settlement_status, real_settlement, real_settlement_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING statement_output_id",
        )
        .bind(row.shop_id)
        .bind(row.vendor_id)
        .bind(row.income)
        .bind(row.expenditure)
        .bind(row.gmt_create)
        .bind(&row.record_sn)
        .bind(row.record_type)
        .bind(row.record_id)
        .bind(row.settlement_status)
        .bind(row.real_settlement)
        .bind(row.real_settlement_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 确认收货更新出账单结算状态。
    pub async fn complete_statement_output_by_order(&self, order_id: i32, now: i32) -> Result<()> {
        sqlx::query(
            "UPDATE statement_output SET real_settlement = 1, real_settlement_time = $1 WHERE record_id = $2 AND record_type = 1",
        )
        .bind(now)
        .bind(order_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("对账单不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn aggregate_query_is_parameterized() {
        // 聚合走 QueryBuilder.push_bind，占位符由 sqlx 生成（`$N`）。
        // 原断言在测试里自造字符串，恒假；改为断言真实实现形态。
        let src = include_str!("statement.rs");
        assert!(
            src.contains("FROM statement WHERE statement_year = "),
            "年份必须是 QueryBuilder 的绑定位"
        );
        // 同 apple_pay：否定断言的 needle 必须运行期拼接，避免自我命中。
        let hardcoded = format!("statement_year = {}", 2026);
        assert!(!src.contains(&hardcoded), "不许写死年份");
    }
}
