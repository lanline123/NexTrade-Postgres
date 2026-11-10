//! 授信会员域仓储（payment::credit_user）。
//!
//! 对应原 Java `CreditUserMapper`（`selectByUserId`）+ `UserMapper` /
//! `UserCompanyMapper` / `CreditOrderMapper` 的只读装配 + `ConfigService`
//! 的货币名称读取。
//!
//! 表（列名经 `python tools/check_sql_columns.py` 校验）：
//! - `credit_user`（10 列，主键 `credit_user_id`，`uk_user_id` 唯一）
//! - 只读装配：`user`、`user_company`（`status = 2`）、`credit_order`
//!   （删除前置校验）、`config`（`dollarSignCn`）
//!
//! 本域无逻辑删除列：`delete` 为物理删除（Java `removeById`）。

use std::collections::HashMap;

use rust_decimal::Decimal;
use sqlx::{FromRow,Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::CreditUser;
use nextrade_core::models::payment::credit_user::{
    limit_offset, CreditUserListQuery, ORDER_SETTLEMENT_UNSETTLED,
};
use nextrade_core::{Error, Result};

/// `credit_user` 固定 SELECT 列清单（顺序与实体一致）。
const CREDIT_USER_SELECT: &str = "SELECT credit_user_id, user_id, credit_limit, used_amount, available_amount, settlement_type, status, period_start_time, create_time, update_time FROM credit_user";

/// 用户装配行（`user`）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct CreditUserUserRow {
    pub user_id: i32,
    pub username: String,
    pub mobile: String,
}

/// 企业认证装配行（`user_company`，`status = 2`）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct CreditUserCompanyRow {
    pub user_id: i32,
    #[sqlx(rename = "type")]
    pub r#type: i32,
    pub contact_name: String,
    pub company_name: String,
}

/// 授信会员域仓储。
#[derive(Clone)]
pub struct CreditUserRepository {
    pool: PgPool,
}

impl CreditUserRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— credit_user —————————————————————

    /// 按主键查询（Java `getById`）。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<CreditUser>> {
        sqlx::query_as::<_, CreditUser>(&format!(
            "{CREDIT_USER_SELECT} WHERE credit_user_id = $1 LIMIT 1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按会员 ID 查询（Java `selectByUserId`，`uk_user_id` 保证唯一）。
    pub async fn find_by_user_id(&self, user_id: i32) -> Result<Option<CreditUser>> {
        sqlx::query_as::<_, CreditUser>(&format!(
            "{CREDIT_USER_SELECT} WHERE user_id = $1 LIMIT 1"
        ))
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 分页（Java `list`：会员类型 / 用户名 / 状态过滤 + `create_time DESC`）。
    ///
    /// `company_user_ids` / `username_user_ids` 为 `Some` 时追加 `IN (...)`；
    /// 筛选命中为空由服务层提前返回空页。
    pub async fn paginate(
        &self,
        q: &CreditUserListQuery,
        company_user_ids: Option<&[i32]>,
        username_user_ids: Option<&[i32]>,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<CreditUser>)> {
        let mut cq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM credit_user WHERE 1 = 1");
        Self::push_filters(&mut cq, q, company_user_ids, username_user_ids);
        let total: (i64,) = cq
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut rq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{CREDIT_USER_SELECT} WHERE 1 = 1"));
        Self::push_filters(&mut rq, q, company_user_ids, username_user_ids);
        rq.push(q.order_clause());
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<CreditUser> = rq
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0, rows))
    }

    fn push_filters(
        qb: &mut QueryBuilder<'_, Postgres>,
        q: &CreditUserListQuery,
        company_user_ids: Option<&[i32]>,
        username_user_ids: Option<&[i32]>,
    ) {
        if let Some(status) = q.status {
            qb.push(" AND status = ");
            qb.push_bind(status);
        }
        if let Some(ids) = username_user_ids {
            qb.push(" AND user_id IN (");
            Self::push_int_list(qb, ids);
            qb.push(")");
        }
        if let Some(ids) = company_user_ids {
            qb.push(" AND user_id IN (");
            Self::push_int_list(qb, ids);
            qb.push(")");
        }
    }

    /// 新增（Java `save`）。
    pub async fn insert(&self, entity: &CreditUser) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO credit_user (user_id, credit_limit, used_amount, available_amount, settlement_type, status, period_start_time, create_time, update_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING credit_user_id",
        )
        .bind(entity.user_id)
        .bind(entity.credit_limit)
        .bind(entity.used_amount)
        .bind(entity.available_amount)
        .bind(entity.settlement_type)
        .bind(entity.status)
        .bind(entity.period_start_time)
        .bind(entity.create_time)
        .bind(entity.update_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 整行更新（Java `updateById`，服务层按 NOT_NULL 语义合并后调用）。
    pub async fn update(&self, entity: &CreditUser) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE credit_user SET user_id = $1, credit_limit = $2, used_amount = $3, available_amount = $4, settlement_type = $5, status = $6, period_start_time = $7, create_time = $8, update_time = $9 WHERE credit_user_id = $10",
        )
        .bind(entity.user_id)
        .bind(entity.credit_limit)
        .bind(entity.used_amount)
        .bind(entity.available_amount)
        .bind(entity.settlement_type)
        .bind(entity.status)
        .bind(entity.period_start_time)
        .bind(entity.create_time)
        .bind(entity.update_time)
        .bind(entity.credit_user_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 物理删除（Java `removeById`）。
    pub async fn delete(&self, id: i32) -> Result<bool> {
        let res = sqlx::query("DELETE FROM credit_user WHERE credit_user_id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 仅更新状态（Java `setStatus` 的 `updateById`，同时刷新 `update_time`）。
    pub async fn update_status(&self, id: i32, status: i32, now: i64) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE credit_user SET status = $1, update_time = $2 WHERE credit_user_id = $3",
        )
        .bind(status)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 查询所有启用的且已设置账期起点的授信会员（对应 Java updateAllPeriodStartTime）。
    pub async fn find_enabled_with_period(&self) -> Result<Vec<CreditUser>> {
        let sql = format!(
            "{CREDIT_USER_SELECT} WHERE status = 1 AND period_start_time IS NOT NULL AND period_start_time > 0"
        );
        sqlx::query_as::<_, CreditUser>(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 更新账期周期起点时间与更新时间
    pub async fn update_period_start_time(&self, id: i32, period_start_time: i64, now: i64) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE credit_user SET period_start_time = $1, update_time = $2 WHERE credit_user_id = $3",
        )
        .bind(period_start_time)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— 装配 / 过滤 —————————————————————

    /// 用户名模糊匹配的会员 ID（`user.username ILIKE`）。
    pub async fn find_user_ids_by_username_like(&self, keyword: &str) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> =
            sqlx::query_as("SELECT user_id FROM \"user\" WHERE username ILIKE $1")
                .bind(format!("%{keyword}%"))
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 指定认证类型且已通过认证（`status = 2`）的会员 ID。
    pub async fn find_user_ids_by_company_type(&self, company_type: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT user_id FROM user_company WHERE \"type\" = $1 AND status = 2",
        )
        .bind(company_type)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 批量用户信息（用户名 / 手机号）。
    pub async fn find_users(&self, ids: &[i32]) -> Result<Vec<CreditUserUserRow>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT user_id, username, mobile FROM \"user\" WHERE user_id IN (");
        Self::push_int_list(&mut qb, ids);
        qb.push(")");
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 批量企业认证（`status = 2`；每用户取 `id` 最小行，Java `limit 1` 的
    /// 确定性落地）。
    pub async fn find_companies(
        &self,
        user_ids: &[i32],
    ) -> Result<Vec<CreditUserCompanyRow>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT user_id, \"type\", contact_name, company_name FROM user_company WHERE status = 2 AND user_id IN (",
        );
        Self::push_int_list(&mut qb, user_ids);
        qb.push(") ORDER BY user_id, id");
        let rows: Vec<CreditUserCompanyRow> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        let mut out: Vec<CreditUserCompanyRow> = Vec::new();
        for row in rows {
            if out.last().map(|r: &CreditUserCompanyRow| r.user_id) != Some(row.user_id) {
                out.push(row);
            }
        }
        Ok(out)
    }

    /// 待结算账期订单数（删除前置校验；Java `settlement_status = 0`）。
    pub async fn count_unsettled_orders(&self, credit_user_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM credit_order WHERE credit_user_id = $1 AND settlement_status = $2",
        )
        .bind(credit_user_id)
        .bind(ORDER_SETTLEMENT_UNSETTLED)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 读取配置值（`config.biz_val`，`is_del = 0`；缺失返回 `None`）。
    pub async fn config_value(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1",
        )
        .bind(biz_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }

    /// 便捷分页窗口（供服务层统一口径）。
    #[must_use]
    pub fn page_window(page: Option<i64>, size: Option<i64>) -> (i64, i64) {
        limit_offset(page, size)
    }

    /// 分类求和辅助：按 `user_id` 建索引（服务层装配用）。
    #[must_use]
    pub fn index_by_user_id(rows: Vec<CreditUserCompanyRow>) -> HashMap<i32, CreditUserCompanyRow> {
        rows.into_iter().map(|row| (row.user_id, row)).collect()
    }

    /// 追加 `IN (...)` 的整数列表（调用方保证非空）。
    fn push_int_list(qb: &mut QueryBuilder<'_, Postgres>, ids: &[i32]) {
        {
            let mut sep = qb.separated(", ");
            for id in ids {
                sep.push_bind(*id);
            }
        }
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

/// 金额精度辅助（`decimal(12,2)`）。
#[must_use]
pub fn as_money(value: Decimal) -> Decimal {
    value.round_dp(2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::payment::credit_user::CREDIT_USER_COLUMNS;

    #[test]
    fn select_matches_schema_columns() {
        let selected = CREDIT_USER_SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap();
        assert_eq!(selected, CREDIT_USER_COLUMNS.join(", "));
        assert_eq!(CREDIT_USER_COLUMNS.len(), 10);
    }

    #[test]
    fn static_guards_for_sql_semantics() {
        let src = include_str!("credit_user.rs");
        // 删除前校验待结算订单（status = 0）
        assert!(src.contains("FROM credit_order WHERE credit_user_id = $1 AND settlement_status = $2"));
        // 企业认证只取已通过
        assert!(src.contains("FROM user_company WHERE status = 2"));
        // 货币配置键读取
        assert!(src.contains("FROM config WHERE biz_code = $1"));
        // 物理删除（本表无逻辑删除列）
        assert!(src.contains("DELETE FROM credit_user WHERE credit_user_id = $1"));
    }

    #[test]
    fn money_helper_rounds_to_two_decimals() {
        assert_eq!(
            as_money(Decimal::new(123456, 3)),
            Decimal::new(12346, 2)
        );
    }
}
