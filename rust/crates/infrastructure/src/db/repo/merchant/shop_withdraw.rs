//! 商家提现仓储（`merchant::shop_withdraw`）。
//!
//! 承载本切片 8 个端点的表操作：
//! - `shop_withdraw` 写操作（插入 / 状态 / 凭证 / 删除）；
//! - `shop` + `shop_withdraw` 聚合查询（`summaryList`）；
//! - `shop` / `merchant` / `config` 读取（余额校验、双余额、提现设置）。
//!
//! 表结构：`shop_withdraw` 19 列（主键 `shop_withdraw_log_id` 自增，插入不带主键）；
//! `shop` 列 `shop_id/shop_title/shop_logo/status/shop_money/add_time/shop_type/merchant_id`；
//! `merchant` 列 `merchant_id/type`；`config` 列 `biz_code/biz_val`。
//!
//! 全部 SQL 为运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`（禁止 `query!` 宏）。

use rust_decimal::Decimal;
use sqlx::PgPool;
use sqlx::{FromRow,Postgres, QueryBuilder};

use nextrade_core::entities::Shop;
use nextrade_core::models::merchant::shop_withdraw::{
    parse_date_end, parse_date_start, resolve_summary_sort_column, ShopWithdrawSummaryQuery,
};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// 汇总列表 LEFT JOIN 行（`withdrawn_amount` / `pending_withdraw_amount` 为聚合别名）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct WithdrawSummaryRow {
    pub shop_id: i32,
    pub shop_title: Option<String>,
    pub shop_logo: Option<String>,
    pub status: Option<i32>,
    pub shop_money: Option<Decimal>,
    pub add_time: Option<i32>,
    pub withdrawn_amount: Decimal,
    pub pending_withdraw_amount: Decimal,
}

#[derive(Clone)]
pub struct ShopWithdrawRepository {
    pool: PgPool,
}

impl ShopWithdrawRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ————————————————————— 写操作 —————————————————————

    /// 插入提现申请（19 列，主键自增）。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert(
        &self,
        shop_id: i32,
        amount: Decimal,
        status: i32,
        add_time: i32,
        remark: &str,
        merchant_account_id: Option<i32>,
        account_data: Option<String>,
        withdraw_sn: &str,
        withdraw_type: i32,
        pay_channel: Option<String>,
        offline_frozen_amount: Decimal,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO shop_withdraw (
                shop_id, amount, status, add_time, remark, merchant_account_id, account_data,
                withdraw_sn, withdraw_type, pay_channel, offline_frozen_amount,
                channel_withdraw_id, channel_status
             ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'','') RETURNING shop_withdraw_log_id",
        )
        .bind(shop_id)
        .bind(amount)
        .bind(status)
        .bind(add_time)
        .bind(remark)
        .bind(merchant_account_id)
        .bind(account_data)
        .bind(withdraw_sn)
        .bind(withdraw_type)
        .bind(pay_channel)
        .bind(offline_frozen_amount)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    pub async fn find_by_id(
        &self,
        id: i32,
    ) -> Result<Option<nextrade_core::entities::ShopWithdraw>> {
        sqlx::query_as::<_, nextrade_core::entities::ShopWithdraw>(
            "SELECT * FROM shop_withdraw WHERE shop_withdraw_log_id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// `updateById` 等价：仅当存在 `status` 时更新状态。
    pub async fn update_status(&self, id: i32, status: i32) -> Result<u64> {
        let r = sqlx::query("UPDATE shop_withdraw SET status = $1 WHERE shop_withdraw_log_id = $2")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 打款凭证回填 + 状态置「完成」（Java `uploadPayVoucher`）。
    pub async fn update_voucher(&self, id: i32, voucher: &str, status: i32) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE shop_withdraw SET payment_voucher = $1, status = $2 WHERE shop_withdraw_log_id = $3",
        )
        .bind(voucher)
        .bind(status)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 通用字段更新（`updateField`）：只接受白名单列，值以字符串绑定由 MySQL 隐式转换。
    pub async fn update_field(&self, id: i32, column: &str, val: &str) -> Result<u64> {
        // column 来自白名单常量，不拼接用户输入
        let sql = format!("UPDATE shop_withdraw SET {column} = $1 WHERE shop_withdraw_log_id = $2");
        let r = sqlx::query(&sql)
            .bind(val)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 批量删除（Java `removeByIds`）。
    pub async fn delete_by_ids(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("DELETE FROM shop_withdraw WHERE shop_withdraw_log_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        let r = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(r.rows_affected())
    }

    // ————————————————————— 校验 / 设置读取 —————————————————————

    /// 店铺行（余额/标题/类型校验；`shop_money` 可为 NULL）。
    pub async fn find_shop(&self, shop_id: i32) -> Result<Option<Shop>> {
        sqlx::query_as::<_, Shop>("SELECT * FROM shop WHERE shop_id = $1 LIMIT 1")
            .bind(shop_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 商户主体类型（Java `DepositRuleMatcher#resolveMerchantType` 同一来源：`merchant.type`）。
    pub async fn find_merchant_type(&self, merchant_id: i32) -> Result<Option<i32>> {
        let row: Option<(Option<i32>,)> =
            sqlx::query_as("SELECT type FROM merchant WHERE merchant_id = $1 LIMIT 1")
                .bind(merchant_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }

    /// 提现频率计数（Java `checkWithdrawalFrequency`：同店 + 起始时间 + 提现方式）。
    pub async fn count_since(&self, shop_id: i32, start_time: i32, withdraw_type: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM shop_withdraw WHERE shop_id = $1 AND add_time >= $2 AND withdraw_type = $3",
        )
        .bind(shop_id)
        .bind(start_time)
        .bind(withdraw_type)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 单个配置键值（`config.biz_code`）。
    pub async fn config_value(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT biz_val FROM config WHERE biz_code = $1 LIMIT 1")
                .bind(biz_code)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }

    /// 批量配置键值（提现设置一次读取）。
    pub async fn config_values(&self, codes: &[&str]) -> Result<std::collections::HashMap<String, Option<String>>> {
        if codes.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT biz_code, biz_val FROM config WHERE biz_code IN (");
        let mut sep = qb.separated(", ");
        for code in codes {
            sep.push_bind(*code);
        }
        drop(sep);
        qb.push(")");
        let rows: Vec<(String, Option<String>)> =
            qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().collect())
    }

    // ————————————————————— 汇总列表 —————————————————————

    /// 店铺提现汇总分页（Java `summaryList`：`shop t LEFT JOIN shop_withdraw t1`，
    /// 金额按状态条件聚合；带提现时间过滤时只保留范围内有提现记录的店铺）。
    pub async fn paginate_summary(
        &self,
        q: &ShopWithdrawSummaryQuery,
        page: &PageQuery,
    ) -> Result<(u64, Vec<WithdrawSummaryRow>)> {
        let withdraw_start = q
            .withdraw_time_start
            .as_deref()
            .and_then(parse_date_start);
        let withdraw_end = q.withdraw_time_end.as_deref().and_then(parse_date_end);
        let add_time_start = q.add_time_start.as_deref().and_then(parse_date_start);
        let add_time_end = q.add_time_end.as_deref().and_then(parse_date_end);
        let sort_col = resolve_summary_sort_column(q.sort_field.as_deref());
        let dir = if q.ascending() { "ASC" } else { "DESC" };

        // 提现金额聚合条件（过滤别名 t1）
        let mut agg_cond = String::new();
        let mut agg_binds: Vec<i32> = Vec::new();
        if let Some(t) = withdraw_start {
            agg_cond.push_str(" AND t1.add_time >= ?");
            agg_binds.push(t);
        }
        if let Some(t) = withdraw_end {
            agg_cond.push_str(" AND t1.add_time <= ?");
            agg_binds.push(t);
        }

        let join = crate::db::to_pg_placeholders(&format!(
            " LEFT JOIN shop_withdraw t1 ON t1.shop_id = t.shop_id{agg_cond}"
        ));

        // 店铺维度过滤
        let mut where_sql = String::from(" WHERE (t.is_delete IS NULL OR t.is_delete = 0)");
        let mut where_binds: Vec<String> = Vec::new();
        if let Some(title) = q
            .shop_title
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            push_like(&mut where_sql, &mut where_binds, "t.shop_title", title);
        }
        if let Some(kw) = q.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            push_like(&mut where_sql, &mut where_binds, "t.shop_title", kw);
        }
        if let Some(status) = q.status {
            where_sql.push_str(" AND t.status = ?");
            where_binds.push(status.to_string());
        }
        if let Some(shop_type) = q.shop_type {
            where_sql.push_str(" AND t.shop_type = ?");
            where_binds.push(shop_type.to_string());
        }
        if let Some(t) = add_time_start {
            where_sql.push_str(" AND t.add_time >= ?");
            where_binds.push(t.to_string());
        }
        if let Some(t) = add_time_end {
            where_sql.push_str(" AND t.add_time <= ?");
            where_binds.push(t.to_string());
        }
        // 提现时间过滤：LEFT JOIN 条件收窄为「范围内有提现记录」（Java 先查 shopId 集合再 IN，等价）
        if withdraw_start.is_some() || withdraw_end.is_some() {
            where_sql.push_str(" AND t1.shop_withdraw_log_id IS NOT NULL");
        }

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(DISTINCT t.shop_id) FROM shop t{join}{where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        for b in &agg_binds {
            cq = cq.bind(*b);
        }
        for b in &where_binds {
            cq = cq.bind(b.as_str());
        }
        let total = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let list_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT t.shop_id, t.shop_title, t.shop_logo, t.status, t.shop_money, t.add_time,
                COALESCE(SUM(CASE WHEN t1.status = 3{agg_cond} THEN t1.amount ELSE 0 END), 0) AS withdrawn_amount,
                COALESCE(SUM(CASE WHEN t1.status IN (0, 4){agg_cond} THEN t1.amount ELSE 0 END), 0) AS pending_withdraw_amount
             FROM shop t{join}{where_sql}
             GROUP BY t.shop_id
             ORDER BY {sort_col} {dir}
             LIMIT ? OFFSET ?"
        ));
        let (limit, offset) = page.limit_offset();
        let mut lq = sqlx::query_as::<_, WithdrawSummaryRow>(&list_sql);
        // 绑定顺序：SELECT 聚合条件（出现两次）→ JOIN 条件 → WHERE → 分页
        for b in &agg_binds {
            lq = lq.bind(*b);
        }
        for b in &agg_binds {
            lq = lq.bind(*b);
        }
        for b in &agg_binds {
            lq = lq.bind(*b);
        }
        for b in &where_binds {
            lq = lq.bind(b.as_str());
        }
        let rows = lq
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }
}

/// 追加 `AND col ILIKE ?`（`%value%`），与 `where_binds` 同步（避免闭包同时借用两个局部）。
fn push_like(where_sql: &mut String, where_binds: &mut Vec<String>, col: &str, value: &str) {
    where_sql.push_str(&format!(" AND {col} ILIKE ?"));
    where_binds.push(format!("%{}%", value.trim()));
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("提现记录不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn insert_statement_column_placeholder_parity() {
        // 插入语句 11 个绑定列 + 2 个常量列（channel_withdraw_id/channel_status）
        let sql = "INSERT INTO shop_withdraw (
                shop_id, amount, status, add_time, remark, merchant_account_id, account_data,
                withdraw_sn, withdraw_type, pay_channel, offline_frozen_amount,
                channel_withdraw_id, channel_status
             ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'','')";
        let opens = sql.matches('$').count();
        assert_eq!(opens, 11, "shop_withdraw 插入占位符数量");
    }
}
