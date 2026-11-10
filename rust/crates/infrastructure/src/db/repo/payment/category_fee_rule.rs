//! 品类服务费率规则域仓储（payment::category_fee_rule）。
//!
//! 对应原 Java `CategoryServiceFeeRuleMapper` + `CategoryServiceFeeRuleCategoryMapper`
//! + `CategoryServiceFeeRuleTargetMapper`（`BaseMapper` + `BaseServiceImpl`），
//! 以及服务层只读使用的 `CategoryMapper` / `ShopMapper` / `VendorMapper`。
//!
//! 表结构（`python tools/schema_parser.py --table <name>` 可复核）：
//! - `category_service_fee_rule`（10 列）
//! - `category_service_fee_rule_category`（8 列）
//! - `category_service_fee_rule_target`（6 列）
//! - `category`（只读 `category_id` / `category_name` / `parent_id`）
//! - `shop`（只读 `shop_id` / `shop_title`）
//! - `vendor`（只读 `vendor_id` / `vendor_name`）
//!
//! 查询统一使用运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`
//! （**不使用** `query!` 宏）。金额 `service_fee_rate` / `order_amount` 等
//! 一律 `rust_decimal::Decimal`，不经过浮点。

use std::collections::HashMap;

use rust_decimal::Decimal;
use sqlx::{FromRow,Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::{
    CategoryServiceFeeRule, CategoryServiceFeeRuleCategory, CategoryServiceFeeRuleTarget,
};
use nextrade_core::models::payment::category_fee_rule::CategoryFeeRulePageQuery;
use nextrade_core::{Error, Result};

/// `category_service_fee_rule` 固定 SELECT 列清单（顺序与实体一致）。
const RULE_SELECT: &str = "SELECT id, rule_name, target_type, scope_type, service_fee_rate, remark, status, deleted, create_time::text AS create_time, modify_time::text AS modify_time FROM category_service_fee_rule";

/// `category_service_fee_rule_category` 固定 SELECT 列清单。
const RULE_CATEGORY_SELECT: &str = "SELECT id, rule_id, category_id, category_name, category_path, deleted, create_time::text AS create_time, modify_time::text AS modify_time FROM category_service_fee_rule_category";

/// `category_service_fee_rule_target` 固定 SELECT 列清单。
const RULE_TARGET_SELECT: &str = "SELECT id, rule_id, target_id, deleted, create_time::text AS create_time, modify_time::text AS modify_time FROM category_service_fee_rule_target";

/// 品类闭包需要的只读行（Java `CategoryMapper.selectBatchIds`）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct CategoryBrief {
    pub category_id: i32,
    pub category_name: String,
    pub parent_id: i32,
}

/// 店铺名（Java `ShopMapper.selectBatchIds` 后取 `shop_title`）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct ShopTitleBrief {
    pub shop_id: i32,
    pub shop_title: Option<String>,
}

/// 供应商名（Java `VendorMapper.selectOnboardedById` 后取 `vendor_name`）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct VendorNameBrief {
    pub vendor_id: i32,
    pub vendor_name: Option<String>,
}

/// 品类服务费率规则域仓储。
#[derive(Clone)]
pub struct CategoryFeeRuleRepository {
    pool: PgPool,
}

impl CategoryFeeRuleRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（服务层事务编排用）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 规则主表 —————————————————————

    /// 按主键查询未删除规则（Java `selectOne(eq(id).eq(deleted,0))`）。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<CategoryServiceFeeRule>> {
        sqlx::query_as::<_, CategoryServiceFeeRule>(&format!(
            "{RULE_SELECT} WHERE id = $1 AND deleted = 0 LIMIT 1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 分页（Java `CategoryFeeRuleServiceImpl#page`）。
    ///
    /// - `deleted = 0` 恒定
    /// - `target_type = ?` 恒定（`@NotNull` 已校验）
    /// - `status` / `keyword`（`rule_name ILIKE`）可选
    /// - ORDER BY 由 [`CategoryFeeRulePageQuery::order_clause`] 生成
    pub async fn paginate(
        &self,
        q: &CategoryFeeRulePageQuery,
        target_type: i32,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<CategoryServiceFeeRule>)> {
        let mut cq: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT COUNT(*) FROM category_service_fee_rule WHERE deleted = 0 AND target_type = ",
        );
        cq.push_bind(target_type);
        Self::push_filters(&mut cq, q);
        let total: (i64,) = cq
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut rq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{RULE_SELECT} WHERE deleted = 0 AND target_type = "));
        rq.push_bind(target_type);
        Self::push_filters(&mut rq, q);
        rq.push(q.order_clause());
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<CategoryServiceFeeRule> = rq
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0, rows))
    }

    fn push_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &CategoryFeeRulePageQuery) {
        if let Some(status) = q.status {
            qb.push(" AND status = ");
            qb.push_bind(status);
        }
        if let Some(keyword) = q
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            qb.push(" AND rule_name ILIKE ");
            qb.push_bind(format!("%{keyword}%"));
        }
    }

    /// 同 `target_type` 下同名规则数（`exclude_id` 用于编辑时排除自身）。
    pub async fn count_by_rule_name(
        &self,
        target_type: i32,
        rule_name: &str,
        exclude_id: Option<i32>,
    ) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM category_service_fee_rule WHERE target_type = $1 AND rule_name = $2 AND deleted = 0 AND ($3 IS NULL OR id <> $4)",
        )
        .bind(target_type)
        .bind(rule_name)
        .bind(exclude_id)
        .bind(exclude_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 冲突候选：同 `target_type` + 启用 + 未删除 + 排除自身（Java `findConflictingRules`）。
    pub async fn list_conflict_candidates(
        &self,
        target_type: i32,
        exclude_id: Option<i32>,
    ) -> Result<Vec<CategoryServiceFeeRule>> {
        sqlx::query_as::<_, CategoryServiceFeeRule>(&format!(
            "{RULE_SELECT} WHERE target_type = $1 AND status = 1 AND deleted = 0 AND ($2 IS NULL OR id <> $3)"
        ))
        .bind(target_type)
        .bind(exclude_id)
        .bind(exclude_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 逻辑删除（`deleted = 1` + `modify_time`）。
    pub async fn soft_delete(&self, id: i32, modify_time: &str) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE category_service_fee_rule SET deleted = 1, modify_time = NULLIF($1, '') WHERE id = $2",
        )
        .bind(modify_time)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 新增规则，返回自增主键。
    pub async fn insert(&self, rule: &CategoryServiceFeeRule) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO category_service_fee_rule (rule_name, target_type, scope_type, service_fee_rate, remark, status, deleted, create_time, modify_time) VALUES ($1,  $2,  $3,  $4,  $5,  $6,  $7, NULLIF($8, ''), NULLIF($9, '')) RETURNING id",
        )
        .bind(&rule.rule_name)
        .bind(rule.target_type)
        .bind(rule.scope_type)
        .bind(rule.service_fee_rate)
        .bind(&rule.remark)
        .bind(rule.status)
        .bind(rule.deleted)
        .bind(&rule.create_time)
        .bind(&rule.modify_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 整行更新（Java `updateById`）。
    pub async fn update(&self, rule: &CategoryServiceFeeRule) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE category_service_fee_rule SET rule_name = $1, target_type = $2, scope_type = $3, service_fee_rate = $4, remark = $5, status = $6, modify_time = NULLIF($7, '') WHERE id = $8",
        )
        .bind(&rule.rule_name)
        .bind(rule.target_type)
        .bind(rule.scope_type)
        .bind(rule.service_fee_rate)
        .bind(&rule.remark)
        .bind(rule.status)
        .bind(&rule.modify_time)
        .bind(rule.id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 更新状态（Java `updateById` 只写 status + modifyTime）。
    pub async fn update_status(&self, id: i32, status: i32, modify_time: &str) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE category_service_fee_rule SET status = $1, modify_time = NULLIF($2, '') WHERE id = $3",
        )
        .bind(status)
        .bind(modify_time)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— 类目关联 —————————————————————

    /// 一批规则的类目关联（未删除），按 `rule_id` 分组。
    pub async fn categories_by_rule_ids(
        &self,
        rule_ids: &[i32],
    ) -> Result<HashMap<i32, Vec<CategoryServiceFeeRuleCategory>>> {
        if rule_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!(
            "{RULE_CATEGORY_SELECT} WHERE deleted = 0 AND rule_id IN ("
        ));
        let mut sep = qb.separated(", ");
        for id in rule_ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        let rows: Vec<CategoryServiceFeeRuleCategory> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        let mut map: HashMap<i32, Vec<CategoryServiceFeeRuleCategory>> = HashMap::new();
        for row in rows {
            map.entry(row.rule_id).or_default().push(row);
        }
        Ok(map)
    }

    /// 一批规则的类目数（Java `countByRuleIds(..., true)`）。
    pub async fn category_counts(&self, rule_ids: &[i32]) -> Result<HashMap<i32, i64>> {
        self.counts_by_rule_ids(RULE_CATEGORY_TABLE, rule_ids).await
    }

    /// 物理删除某规则的类目关联（Java `ruleCategoryMapper.delete(eq(ruleId))`）。
    pub async fn delete_categories(&self, rule_id: i32) -> Result<u64> {
        let res = sqlx::query("DELETE FROM category_service_fee_rule_category WHERE rule_id = $1")
            .bind(rule_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 新增类目关联行。
    pub async fn insert_category(
        &self,
        rule_id: i32,
        category_id: i32,
        category_name: &str,
        category_path: &str,
        create_time: &str,
        modify_time: &str,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO category_service_fee_rule_category (rule_id, category_id, category_name, category_path, deleted, create_time, modify_time) VALUES ($1,  $2,  $3,  $4,  0, NULLIF($5, ''), NULLIF($6, '')) RETURNING id",
        )
        .bind(rule_id)
        .bind(category_id)
        .bind(category_name)
        .bind(category_path)
        .bind(create_time)
        .bind(modify_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    // ————————————————————— 目标关联 —————————————————————

    /// 一批规则的目标关联（未删除），按 `rule_id` 分组。
    pub async fn targets_by_rule_ids(
        &self,
        rule_ids: &[i32],
    ) -> Result<HashMap<i32, Vec<CategoryServiceFeeRuleTarget>>> {
        if rule_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!(
            "{RULE_TARGET_SELECT} WHERE deleted = 0 AND rule_id IN ("
        ));
        let mut sep = qb.separated(", ");
        for id in rule_ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        let rows: Vec<CategoryServiceFeeRuleTarget> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        let mut map: HashMap<i32, Vec<CategoryServiceFeeRuleTarget>> = HashMap::new();
        for row in rows {
            map.entry(row.rule_id).or_default().push(row);
        }
        Ok(map)
    }

    /// 一批规则的目标数（Java `countByRuleIds(..., false)`）。
    pub async fn target_counts(&self, rule_ids: &[i32]) -> Result<HashMap<i32, i64>> {
        self.counts_by_rule_ids(RULE_TARGET_TABLE, rule_ids).await
    }

    /// 物理删除某规则的目标关联。
    pub async fn delete_targets(&self, rule_id: i32) -> Result<u64> {
        let res = sqlx::query("DELETE FROM category_service_fee_rule_target WHERE rule_id = $1")
            .bind(rule_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 新增目标关联行。
    pub async fn insert_target(
        &self,
        rule_id: i32,
        target_id: i32,
        create_time: &str,
        modify_time: &str,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO category_service_fee_rule_target (rule_id, target_id, deleted, create_time, modify_time) VALUES ($1,  $2,  0, NULLIF($3, ''), NULLIF($4, '')) RETURNING id",
        )
        .bind(rule_id)
        .bind(target_id)
        .bind(create_time)
        .bind(modify_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 关联表按 `rule_id` 分组计数（表名由内部常量给出，无注入面）。
    async fn counts_by_rule_ids(
        &self,
        table: &'static str,
        rule_ids: &[i32],
    ) -> Result<HashMap<i32, i64>> {
        if rule_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!(
            "SELECT rule_id, COUNT(*) AS cnt FROM {table} WHERE deleted = 0 AND rule_id IN ("
        ));
        let mut sep = qb.separated(", ");
        for id in rule_ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(") GROUP BY rule_id");
        drop(sep);
        let rows: Vec<(i32, i64)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().collect())
    }

    // ————————————————————— 品类 / 店铺 / 供应商 —————————————————————

    /// 按 ID 批量取品类（Java `CategoryMapper.selectBatchIds`）。
    pub async fn find_categories(&self, ids: &[i32]) -> Result<Vec<CategoryBrief>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT category_id, category_name, parent_id FROM category WHERE category_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 按 ID 批量取店铺名（Java `ShopMapper.selectBatchIds`）。
    pub async fn find_shop_titles(&self, ids: &[i32]) -> Result<Vec<ShopTitleBrief>> {
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

    /// 按 ID 批量取供应商名。
    pub async fn find_vendor_names(&self, ids: &[i32]) -> Result<Vec<VendorNameBrief>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT vendor_id, vendor_name FROM vendor WHERE vendor_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }
}

/// 规则关联表名（内部常量，拼进 `COUNT GROUP BY` 的固定 SQL）。
const RULE_CATEGORY_TABLE: &str = "category_service_fee_rule_category";
/// 目标关联表名（同上）。
const RULE_TARGET_TABLE: &str = "category_service_fee_rule_target";

/// `sqlx::Error` → 项目错误。
fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

/// 供服务层复用的 `Decimal` 类型别名（保持与 Java `BigDecimal` 语义一致）。
pub type Rate = Decimal;

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::payment::category_fee_rule::{
        CATEGORY_COLUMNS, RULE_COLUMNS, TARGET_COLUMNS,
    };

    #[test]
    fn select_lists_cover_schema_columns() {
        fn selected(s: &str) -> String {
            s.trim_start_matches("SELECT ")
                .split(" FROM ")
                .next()
                .unwrap()
                .to_string()
        }
        // PG 里 text 化时间列写成 `create_time::text AS create_time`，
        // 断言只看「逻辑列是否覆盖齐全」，不绑死某一种 cast 写法。
        fn logical(s: &str) -> String {
            s.split(", ")
                .map(|c| c.rsplit(" AS ").next().unwrap().trim())
                .collect::<Vec<_>>()
                .join(", ")
        }
        assert_eq!(logical(&selected(RULE_SELECT)), RULE_COLUMNS.join(", "));
        assert_eq!(
            logical(&selected(RULE_CATEGORY_SELECT)),
            CATEGORY_COLUMNS.join(", ")
        );
        assert_eq!(
            logical(&selected(RULE_TARGET_SELECT)),
            TARGET_COLUMNS.join(", ")
        );
        assert_eq!(RULE_COLUMNS.len(), 10);
    }

    #[test]
    fn count_sql_uses_internal_table_names_only() {
        assert_eq!(RULE_CATEGORY_TABLE, "category_service_fee_rule_category");
        assert_eq!(RULE_TARGET_TABLE, "category_service_fee_rule_target");
    }

    #[test]
    fn brief_rows_default_like_java_nulls() {
        let c = CategoryBrief::default();
        assert_eq!(c.category_id, 0);
        assert!(c.category_name.is_empty());
        let s = ShopTitleBrief::default();
        assert_eq!(s.shop_title, None);
        let v = VendorNameBrief::default();
        assert_eq!(v.vendor_name, None);
    }
}
