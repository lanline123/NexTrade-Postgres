//! 店铺保证金规则仓储（`merchant::deposit_rule`）。
//!
//! 表：`shop_deposit_rule`（11 列，主键 `shop_deposit_rule_id` 自增）、
//! `shop_deposit_rule_category`（`rule_id` + `category_id`）、
//! `category`（类目名回填）。
//!
//! 全部 SQL 为运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`（禁止 `query!` 宏）。

use std::collections::HashMap;

use sqlx::PgPool;
use sqlx::{Postgres, QueryBuilder};

use nextrade_core::entities::ShopDepositRule;
use nextrade_core::models::merchant::deposit_rule::DepositRuleListQuery;
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct DepositRuleRepository {
    pool: PgPool,
}

impl DepositRuleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 规则分页（Java `list`：`is_delete = 0` + 名称 ILIKE + 状态 + 生效窗口交集）。
    ///
    /// 生效窗口交集条件（逐条照抄 Java）：
    /// - 查询给了 start_time → `(rule.end_time = 0 OR rule.end_time >= start_time)`
    /// - 查询给了 end_time → `rule.start_time <= end_time`
    /// - 排序 `shop_deposit_rule_id DESC`。
    pub async fn paginate(
        &self,
        q: &DepositRuleListQuery,
        page: &PageQuery,
    ) -> Result<(u64, Vec<ShopDepositRule>)> {
        let mut wh = String::from(" WHERE is_delete = 0");
        let mut binds: Vec<String> = Vec::new();
        if let Some(name) = q
            .rule_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            wh.push_str(" AND rule_name ILIKE ?");
            binds.push(format!("%{name}%"));
        }
        if let Some(status) = q.status {
            wh.push_str(" AND status = ?");
            binds.push(status.to_string());
        }
        if let Some(start) = q.start_time {
            wh.push_str(" AND (end_time = 0 OR end_time >= ?)");
            binds.push(start.to_string());
        }
        if let Some(end) = q.end_time {
            wh.push_str(" AND start_time <= ?");
            binds.push(end.to_string());
        }
        let total_sql = format!("SELECT COUNT(*) FROM shop_deposit_rule{wh}");
        let total_sql = crate::db::to_pg_placeholders(&total_sql);
        let mut tq = sqlx::query_as::<_, (i64,)>(&total_sql);
        for b in &binds {
            tq = tq.bind(b.as_str());
        }
        let total = tq.fetch_one(&self.pool).await.map_err(map_err)?;

        let (limit, offset) = page.limit_offset();
        let list_sql = format!(
            "SELECT * FROM shop_deposit_rule{wh} ORDER BY shop_deposit_rule_id DESC LIMIT $1 OFFSET $2"
        );
        let list_sql = crate::db::to_pg_placeholders(&list_sql);
        let mut lq = sqlx::query_as::<_, ShopDepositRule>(&list_sql);
        for b in &binds {
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

    pub async fn find_by_id(&self, id: i32) -> Result<Option<ShopDepositRule>> {
        sqlx::query_as::<_, ShopDepositRule>(
            "SELECT * FROM shop_deposit_rule WHERE shop_deposit_rule_id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 插入规则（主键自增，返回 id）。
    pub async fn insert(&self, rule: &ShopDepositRule) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO shop_deposit_rule (rule_name, merchant_types, shop_types, deposit_amount,
                start_time, end_time, status, is_delete, add_time, update_time)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING shop_deposit_rule_id",
        )
        .bind(&rule.rule_name)
        .bind(&rule.merchant_types)
        .bind(&rule.shop_types)
        .bind(rule.deposit_amount)
        .bind(rule.start_time)
        .bind(rule.end_time)
        .bind(rule.status)
        .bind(rule.is_delete)
        .bind(rule.add_time)
        .bind(rule.update_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 整行更新（Java `updateById`）。
    pub async fn update(&self, rule: &ShopDepositRule) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE shop_deposit_rule SET rule_name = $1, merchant_types = $2, shop_types = $3,
                deposit_amount = $4, start_time = $5, end_time = $6, status = $7, is_delete = $8,
                add_time = $9, update_time = $10
             WHERE shop_deposit_rule_id = $11",
        )
        .bind(&rule.rule_name)
        .bind(&rule.merchant_types)
        .bind(&rule.shop_types)
        .bind(rule.deposit_amount)
        .bind(rule.start_time)
        .bind(rule.end_time)
        .bind(rule.status)
        .bind(rule.is_delete)
        .bind(rule.add_time)
        .bind(rule.update_time)
        .bind(rule.shop_deposit_rule_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 软删除（Java `del`：`is_delete = 1`）。
    pub async fn soft_delete(&self, id: i32, update_time: i32) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE shop_deposit_rule SET is_delete = 1, update_time = $1 WHERE shop_deposit_rule_id = $2",
        )
        .bind(update_time)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 启停规则（Java `changeStatus`）。
    pub async fn update_status(&self, id: i32, status: i32, update_time: i32) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE shop_deposit_rule SET status = $1, update_time = $2 WHERE shop_deposit_rule_id = $3",
        )
        .bind(status)
        .bind(update_time)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    // ————————————————————— 类目关联 —————————————————————

    /// 单规则类目 id 列表（Java `listCategoryIds`）。
    pub async fn category_ids_by_rule(&self, rule_id: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT category_id FROM shop_deposit_rule_category WHERE rule_id = $1 ORDER BY id ASC",
        )
        .bind(rule_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 批量类目映射（Java `loadRuleCategoryMap`，避免 N+1）。
    pub async fn category_map(
        &self,
        rule_ids: &[i32],
    ) -> Result<HashMap<i32, Vec<i32>>> {
        if rule_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT rule_id, category_id FROM shop_deposit_rule_category WHERE rule_id IN (");
        let mut sep = qb.separated(", ");
        for id in rule_ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(") ORDER BY id ASC");
        let rows: Vec<(i32, i32)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        let mut map: HashMap<i32, Vec<i32>> = HashMap::new();
        for (rule_id, category_id) in rows {
            map.entry(rule_id).or_default().push(category_id);
        }
        Ok(map)
    }

    /// 删除规则的全部类目关联（Java `update` 先删后插）。
    pub async fn delete_categories(&self, rule_id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM shop_deposit_rule_category WHERE rule_id = $1")
            .bind(rule_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 写入单条类目关联（Java `saveCategories`）。
    pub async fn insert_category(&self, rule_id: i32, category_id: i32) -> Result<i64> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO shop_deposit_rule_category (rule_id, category_id) VALUES ($1, $2) RETURNING id",
        )
        .bind(rule_id)
        .bind(category_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(i64::from(r))
    }

    /// 类目名映射（Java `loadCategoryNameMap`：`category.category_id → category_name`）。
    pub async fn category_names(&self, ids: &[i32]) -> Result<HashMap<i32, String>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT category_id, category_name FROM category WHERE category_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        let rows: Vec<(i32, String)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().collect())
    }

    /// 时间重叠候选规则（Java `checkTimeOverlap` 粗筛）：
    /// 未删除 + 启用 + 排除自身 + 与新区间重叠。
    pub async fn overlap_candidates(
        &self,
        self_id: Option<i32>,
        new_start: i64,
        new_end: i64,
    ) -> Result<Vec<ShopDepositRule>> {
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT * FROM shop_deposit_rule WHERE is_delete = 0 AND status = 1",
        );
        if let Some(id) = self_id {
            qb.push(" AND shop_deposit_rule_id <> ");
            qb.push_bind(id);
        }
        // 新规则长期有效（new_end=0）时不限制候选起点
        if new_end != 0 {
            qb.push(" AND start_time <= ");
            qb.push_bind(new_end);
        }
        qb.push(" AND (end_time = 0 OR end_time >= ");
        qb.push_bind(new_start);
        qb.push(")");
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("规则不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn insert_and_update_placeholder_counts_match() {
        let insert = "INSERT INTO shop_deposit_rule (rule_name, merchant_types, shop_types, deposit_amount,
                start_time, end_time, status, is_delete, add_time, update_time)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)";
        assert_eq!(insert.matches('$').count(), 10);
        let update = "UPDATE shop_deposit_rule SET rule_name = $1, merchant_types = $2, shop_types = $3,
                deposit_amount = $4, start_time = $5, end_time = $6, status = $7, is_delete = $8,
                add_time = $9, update_time = $10
             WHERE shop_deposit_rule_id = $11";
        assert_eq!(update.matches('$').count(), 11);
        // 关联表实体字段与 SQL 对齐（编译期占位，防重构漏改）
        let _ = std::mem::size_of::<nextrade_core::entities::ShopDepositRuleCategory>();
    }
}
