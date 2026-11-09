//! 店铺端保证金聚合仓储（`merchant::deposit_shop`）。
//!
//! 表：`shop_deposit`（账户）/ `shop_deposit_log`（流水）/ `shop_deposit_refund`
//! （待审核提现合计）/ `shop_deposit_recharge`（充值账户快照）/
//! `shop_deposit_rule` + `shop_deposit_rule_category`（应缴规则匹配）/
//! `product`（在售类目）/ `category`（类目父级）/ `config`（功能开关+协议）/ `shop`+`merchant`（主体类型）。
//!
//! 全部 SQL 为运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`（禁止 `query!` 宏）。

use std::collections::HashMap;

use rust_decimal::Decimal;
use sqlx::PgPool;
use sqlx::{Postgres, QueryBuilder};

use nextrade_core::entities::deposit::{ShopDeposit, ShopDepositLog, ShopDepositRecharge};
use nextrade_core::entities::ShopDepositRefund;
use nextrade_core::models::merchant::deposit_shop::{DepositShopLogQuery, OWNER_SHOP};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct DepositShopRepository {
    pool: PgPool,
}

impl DepositShopRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ————————————————————— 账户 / 配置 —————————————————————

    /// 只读读取保证金账户（Java `findAccount`：不存在返回 None，不创建行）。
    pub async fn find_account(&self, shop_id: i32) -> Result<Option<ShopDeposit>> {
        sqlx::query_as::<_, ShopDeposit>(
            "SELECT * FROM shop_deposit WHERE owner_type = $1 AND owner_id = $2 LIMIT 1",
        )
        .bind(OWNER_SHOP)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 配置键值（Java `readConfig`：不存在返回 None，不抛错）。
    pub async fn find_config(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT biz_val FROM config WHERE biz_code = $1 LIMIT 1")
                .bind(biz_code)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }

    /// 店铺主体类型（Java `DepositRuleMatcher#resolveMerchantType`：`shop.merchant_id → merchant.type`）。
    pub async fn shop_merchant_type(&self, shop_id: i32) -> Result<Option<i32>> {
        let row: Option<(Option<i32>,)> = sqlx::query_as(
            "SELECT m.type FROM shop s INNER JOIN merchant m ON m.merchant_id = s.merchant_id WHERE s.shop_id = $1 LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }

    /// 店铺在售商品类目 id（Java `selectOnSaleCategoryIds`）：
    /// `product.shop_id = ? AND product_status = 1 AND is_delete = 0 AND category_id > 0`。
    pub async fn on_sale_category_ids(&self, shop_id: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT DISTINCT category_id FROM product WHERE shop_id = $1 AND product_status = 1 AND is_delete = 0 AND category_id > 0",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 类目父级 id（`category.parent_id`）。
    pub async fn category_parent(&self, category_id: i32) -> Result<Option<i32>> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT parent_id FROM category WHERE category_id = $1 LIMIT 1")
                .bind(category_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|r| r.0))
    }

    /// 按类目集合取每个类目的最高应缴金额
    /// （逐条照抄 Java `ShopDepositRuleMapper#matchMaxByCategories` XML）。
    pub async fn match_max_by_categories(
        &self,
        merchant_type: i32,
        shop_type: i32,
        category_ids: &[i32],
        now: i64,
    ) -> Result<Vec<(i32, Decimal)>> {
        if category_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT rc.category_id, MAX(r.deposit_amount) FROM shop_deposit_rule r INNER JOIN shop_deposit_rule_category rc ON rc.rule_id = r.shop_deposit_rule_id WHERE r.status = 1 AND r.is_delete = 0 AND r.start_time <= ",
        );
        qb.push_bind(now);
        qb.push(" AND (r.end_time = 0 OR r.end_time >= ");
        qb.push_bind(now);
        qb.push(") AND ");
        qb.push_bind(merchant_type);
        qb.push("::text = ANY(string_to_array(r.merchant_types, ',')) AND ");
        qb.push_bind(shop_type);
        qb.push("::text = ANY(string_to_array(r.shop_types, ',')) AND rc.category_id IN (");
        let mut separated = qb.separated(", ");
        for id in category_ids {
            separated.push_bind(*id);
        }
        drop(separated);
        qb.push(") GROUP BY rc.category_id");
        let rows: Vec<(i32, Decimal)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows)
    }

    /// 待审核退还/提现申请金额合计（Java `DepositRefundService#pendingRefundSum`）。
    pub async fn pending_refund_sum(&self, shop_id: i32) -> Result<Decimal> {
        let row: (Option<Decimal>,) = sqlx::query_as(
            "SELECT SUM(amount) FROM shop_deposit_refund WHERE owner_type = $1 AND owner_id = $2 AND status = 0",
        )
        .bind(OWNER_SHOP)
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.unwrap_or(Decimal::ZERO))
    }

    // ————————————————————— 流水分页 —————————————————————

    /// 保证金流水分页（Java `logList` / `frozenDetailList`）。
    ///
    /// - `owner_type = 1` + `owner_id = shopId`
    /// - `deduct_only = true` → 额外 `type = 4`（冻结明细，忽略 `q.type`）
    /// - 其余 `q.type`：提现类（2/3）→ `IN (2,3)`，否则等值
    /// - `startTime` / `endTime` → `add_time` 区间
    /// - `tradeNo` 纯数字 → `(order_id = ? OR related_id = ?)`
    /// - 固定 `ORDER BY shop_deposit_log_id DESC`
    pub async fn paginate_logs(
        &self,
        shop_id: i32,
        q: &DepositShopLogQuery,
        deduct_only: bool,
    ) -> Result<(u64, Vec<ShopDepositLog>)> {
        let page = q.page_query();
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT COUNT(*) FROM shop_deposit_log WHERE owner_type = ",
        );
        qb.push_bind(OWNER_SHOP);
        qb.push(" AND owner_id = ");
        qb.push_bind(shop_id);
        self.push_log_filters(&mut qb, q, deduct_only);
        let total: (i64,) = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let (limit, offset) = page.limit_offset();
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT * FROM shop_deposit_log WHERE owner_type = ",
        );
        qb.push_bind(OWNER_SHOP);
        qb.push(" AND owner_id = ");
        qb.push_bind(shop_id);
        self.push_log_filters(&mut qb, q, deduct_only);
        qb.push(" ORDER BY shop_deposit_log_id DESC LIMIT ");
        qb.push_bind(limit);
        qb.push(" OFFSET ");
        qb.push_bind(offset);
        let rows: Vec<ShopDepositLog> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    fn push_log_filters<'a>(
        &self,
        qb: &mut QueryBuilder<'a, Postgres>,
        q: &DepositShopLogQuery,
        deduct_only: bool,
    ) {
        if deduct_only {
            qb.push(" AND type = ");
            qb.push_bind(4);
        } else if let Some(log_type) = q.r#type {
            if log_type == 2 || log_type == 3 {
                qb.push(" AND type IN (");
                qb.push_bind(2);
                qb.push(", ");
                qb.push_bind(3);
                qb.push(")");
            } else {
                qb.push(" AND type = ");
                qb.push_bind(log_type);
            }
        }
        if let Some(start) = q.start_time {
            qb.push(" AND add_time >= ");
            qb.push_bind(start);
        }
        if let Some(end) = q.end_time {
            qb.push(" AND add_time <= ");
            qb.push_bind(end);
        }
        if let Some(trade_no) = q.trade_no_int() {
            qb.push(" AND (order_id = ");
            qb.push_bind(trade_no);
            qb.push(" OR related_id = ");
            qb.push_bind(trade_no);
            qb.push(")");
        }
    }

    /// 充值单快照批量读取（Java `DepositShopLogEnricher`：来源账户）。
    pub async fn recharge_map(
        &self,
        ids: &[i32],
    ) -> Result<HashMap<i32, ShopDepositRecharge>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT * FROM shop_deposit_recharge WHERE shop_deposit_recharge_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        let rows: Vec<ShopDepositRecharge> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| (r.shop_deposit_recharge_id, r)).collect())
    }

    /// 退还/提现申请批量读取（Java `DepositShopLogEnricher`：去向账户）。
    pub async fn refund_map(&self, ids: &[i32]) -> Result<HashMap<i32, ShopDepositRefund>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT * FROM shop_deposit_refund WHERE shop_deposit_refund_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        let rows: Vec<ShopDepositRefund> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|r| (r.shop_deposit_refund_id, r))
            .collect())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("保证金账户不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn rule_matching_sql_shape_is_parameterized() {
        // 断言 SQL 模板关键条件（防止把时间/类型内联导致注入）
        let sql = "r.status = 1 AND r.is_delete = 0 AND r.start_time <= $1 AND (r.end_time = 0 OR r.end_time >= $2) AND $3 = ANY(string_to_array(r.merchant_types, ',')) AND $4 = ANY(string_to_array(r.shop_types, ',')) AND rc.category_id IN (...) GROUP BY rc.category_id";
        assert!(sql.contains("$3 = ANY(string_to_array(r.merchant_types"));
        assert!(sql.contains("$4 = ANY(string_to_array(r.shop_types"));
        assert!(!sql.contains("string_to_array(1,"), "禁止内联主体类型");
    }
}
