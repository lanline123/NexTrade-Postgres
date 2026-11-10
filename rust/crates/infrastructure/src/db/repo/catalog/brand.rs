//! 品牌域仓储：对应原 Java `BrandMapper`。
//!
//! 列名严格对齐权威 schema（`python tools/schema_parser.py --table brand`）：
//! - `brand`：`brand_id` / `brand_name` / `brand_logo` / `brand_desc` /
//!   `site_url` / `sort_order` / `is_show` / `brand_type` / `brand_is_hot` /
//!   `first_word` / `is_store_brand` / `check_status` / `brand_en_name` /
//!   `shop_id` / `status` / `reject_remark`
//! - `shop`（本仓储只读两列）：`shop_id` / `shop_title`
//!
//! 所有列表 SQL 显式列出列名（与实体字段顺序一致），不用 `SELECT *`。

use sqlx::PgPool;
use nextrade_core::entities::{Brand, Shop};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// `brand` 表全部列（顺序与 [`Brand`] 实体字段一致）。
const BRAND_COLUMNS: &str = "brand_id, brand_name, brand_logo, brand_desc, site_url, sort_order, is_show, brand_type, brand_is_hot, first_word, is_store_brand, check_status, brand_en_name, shop_id, status, reject_remark";

/// 排序字段白名单（对应 `BaseServiceImpl.getColumnByPropertyName` 的防注入兜底：
/// 未知道段回退主键；`brand` 表默认主键降序）。
const SORTABLE_COLUMNS: [&str; 7] = [
    "brand_id",
    "brand_name",
    "sort_order",
    "is_show",
    "brand_is_hot",
    "first_word",
    "check_status",
];

/// `updateField` 允许更新的列白名单（对应 Java `CheckFieldConstants.BRAND_FIELDS`）。
///
/// 本地定义，避免仓储层反向依赖 `nextrade_services`（infra 不许依赖 services）。
/// 与服务层 `nextrade_core::models::catalog::brand::BRAND_FIELDS` 同源，务必保持一致。
const BRAND_FIELDS: [&str; 5] = [
    "brand_name",
    "first_word",
    "brand_is_hot",
    "is_show",
    "sort_order",
];

#[derive(Clone)]
pub struct BrandRepository {
    pool: PgPool,
}

impl BrandRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 按主键查询品牌。
    pub async fn find_by_id(&self, brand_id: i32) -> Result<Option<Brand>> {
        let sql = format!("SELECT {BRAND_COLUMNS} FROM brand WHERE brand_id = $1 LIMIT 1");
        sqlx::query_as::<_, Brand>(&sql)
            .bind(brand_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 全部品牌（供 `updateFirstWord` 批量重算用）。
    pub async fn find_all(&self) -> Result<Vec<Brand>> {
        let sql = format!("SELECT {BRAND_COLUMNS} FROM brand ORDER BY brand_id ASC");
        sqlx::query_as::<_, Brand>(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 品牌分页（管理端列表 / 审核列表共用）。
    ///
    /// 对应 Java `getBrandQueryWrapper` + `buildSortOrder`：
    /// - `keyword` -> `brand_name ILIKE`；`first_word` -> `ILIKE`
    /// - `status` 为 `-1` / `None` 不过滤
    /// - `shop_id`：`Some(0)` 视为 `None`（平台账号语义）；`shopId>0` 强制本店；
    ///   `audit_only = true` 时强制 `shop_id <> 0`（对应 Java `isAudit==1` -> `.ne(shopId, 0)`）
    #[allow(clippy::too_many_arguments)]
    pub async fn paginate(
        &self,
        keyword: Option<&str>,
        is_show: Option<i32>,
        brand_is_hot: Option<i32>,
        first_word: Option<&str>,
        status: Option<i32>,
        shop_id: Option<i32>,
        audit_only: bool,
        sort_field: Option<&str>,
        sort_order: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<Brand>)> {
        let like = keyword
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(|k| format!("%{k}%"));
        let fw = first_word
            .map(str::trim)
            .filter(|w| !w.is_empty())
            .map(|w| format!("%{w}%"));

        let column = resolve_sort_column(sort_field);
        // Java `buildSortOrder`：仅显式 asc 升序，desc/空一律降序。
        let dir = if sort_order.is_some_and(|o| o.eq_ignore_ascii_case("asc")) {
            "ASC"
        } else {
            "DESC"
        };

        let mut where_parts: Vec<String> = vec![];
        if like.is_some() {
            where_parts.push("brand_name ILIKE ?".into());
        }
        if fw.is_some() {
            where_parts.push("first_word ILIKE ?".into());
        }
        if status.is_some() {
            where_parts.push("status = ?".into());
        }
        if is_show.is_some() {
            where_parts.push("is_show = ?".into());
        }
        if brand_is_hot.is_some() {
            where_parts.push("brand_is_hot = ?".into());
        }
        if audit_only {
            where_parts.push("shop_id <> 0".into());
        } else if shop_id.is_some_and(|s| s > 0) {
            where_parts.push("shop_id = ?".into());
        }
        let where_sql = if where_parts.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_parts.join(" AND "))
        };
        let (limit, offset) = page.limit_offset();

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM brand {where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        cq = bind_filters(
            cq,
            like.as_deref(),
            fw.as_deref(),
            status,
            is_show,
            brand_is_hot,
            shop_id.filter(|_| !audit_only),
        );
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let list_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {BRAND_COLUMNS} FROM brand {where_sql} ORDER BY {column} {dir} LIMIT ? OFFSET ?"
        ));
        let mut lq = sqlx::query_as::<_, Brand>(&list_sql);
        lq = bind_filters(
            lq,
            like.as_deref(),
            fw.as_deref(),
            status,
            is_show,
            brand_is_hot,
            shop_id.filter(|_| !audit_only),
        );
        let rows = lq
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 前台可搜索品牌（`status = 1` 审核通过且 `is_show = 1`）。
    ///
    /// 对应 Java `BrandServiceImpl#search` 的第一个 lambdaQuery。
    pub async fn find_searchable(&self) -> Result<Vec<Brand>> {
        let sql = format!(
            "SELECT {BRAND_COLUMNS} FROM brand WHERE status = 1 AND is_show = 1 ORDER BY sort_order ASC, brand_id ASC"
        );
        sqlx::query_as::<_, Brand>(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 去重首字母列（对应 Java search 的 `groupBy(first_word)`）。
    pub async fn distinct_first_words(&self) -> Result<Vec<String>> {
        let rows: Vec<(Option<String>,)> = sqlx::query_as(
            "SELECT first_word FROM brand WHERE first_word IS NOT NULL AND first_word <> '' GROUP BY first_word ORDER BY first_word ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().filter_map(|(w,)| w).collect())
    }

    /// 按状态统计品牌数（`auditWaitNum`）。
    pub async fn count_by_status(&self, status: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM brand WHERE status = $1")
            .bind(status)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0)
    }

    /// 批量加载店铺（列表页展示店铺名，Java `shopMapper.selectBatchIds`）。
    pub async fn find_shops(&self, shop_ids: &[i32]) -> Result<Vec<Shop>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; shop_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT * FROM shop WHERE shop_id IN ({placeholders})"));
        let mut q = sqlx::query_as::<_, Shop>(&sql);
        for id in shop_ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 判断同店铺内是否已存在同名品牌（更新时排除自身）。
    ///
    /// 对应 Java `checkBrand`：`eq(brandName).eq(shopId).ne(brandId, 自身)`。
    pub async fn exists_same_name(
        &self,
        brand_name: &str,
        shop_id: i32,
        exclude_id: Option<i32>,
    ) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT brand_id FROM brand WHERE brand_name = $1 AND shop_id = $2 AND ($3 IS NULL OR brand_id <> $4) LIMIT 1",
        )
        .bind(brand_name)
        .bind(shop_id)
        .bind(exclude_id)
        .bind(exclude_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.is_some())
    }

    // ———————————————— 写操作 ————————————————

    /// 新增品牌，返回自增主键。
    pub async fn insert(&self, b: &Brand) -> Result<i32> {
        let r: i32 = sqlx::query_scalar("INSERT INTO brand (brand_name, brand_logo, brand_desc, site_url, sort_order, is_show, brand_type, brand_is_hot, first_word, is_store_brand, check_status, brand_en_name, shop_id, status, reject_remark) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15) RETURNING brand_id")
            .bind(&b.brand_name)
            .bind(&b.brand_logo)
            .bind(&b.brand_desc)
            .bind(&b.site_url)
            .bind(b.sort_order)
            .bind(b.is_show)
            .bind(&b.brand_type)
            .bind(b.brand_is_hot)
            .bind(&b.first_word)
            .bind(b.is_store_brand)
            .bind(b.check_status)
            .bind(&b.brand_en_name)
            .bind(b.shop_id)
            .bind(b.status)
            .bind(&b.reject_remark)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r)
    }

    /// 更新品牌（整行，除主键外的全部列）。
    pub async fn update(&self, b: &Brand) -> Result<()> {
        let affected = sqlx::query("UPDATE brand SET brand_name = $1, brand_logo = $2, brand_desc = $3, site_url = $4, sort_order = $5, is_show = $6, brand_type = $7, brand_is_hot = $8, first_word = $9, is_store_brand = $10, check_status = $11, brand_en_name = $12, shop_id = $13, status = $14, reject_remark = $15 WHERE brand_id = $16")
            .bind(&b.brand_name)
            .bind(&b.brand_logo)
            .bind(&b.brand_desc)
            .bind(&b.site_url)
            .bind(b.sort_order)
            .bind(b.is_show)
            .bind(&b.brand_type)
            .bind(b.brand_is_hot)
            .bind(&b.first_word)
            .bind(b.is_store_brand)
            .bind(b.check_status)
            .bind(&b.brand_en_name)
            .bind(b.shop_id)
            .bind(b.status)
            .bind(&b.reject_remark)
            .bind(b.brand_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        if affected == 0 {
            return Err(Error::NotFound("品牌不存在".into()));
        }
        Ok(())
    }

    /// 删除单个品牌（`brand` 无删除标记列，直接 DELETE）。
    pub async fn delete(&self, brand_id: i32) -> Result<bool> {
        let r = sqlx::query("DELETE FROM brand WHERE brand_id = $1")
            .bind(brand_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 批量删除品牌（对应 Java `removeByIds`）。
    pub async fn delete_batch(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; ids.len()].join(",");
        let sql = crate::db::to_pg_placeholders(&format!("DELETE FROM brand WHERE brand_id IN ({placeholders})"));
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(id);
        }
        let r = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 更新单个字段（列名来自白名单，禁止用户任意输入列名防注入）。
    pub async fn update_field(&self, brand_id: i32, field: &str, val: &str) -> Result<bool> {
        if !BRAND_FIELDS.contains(&field) {
            return Err(Error::Validation(format!("不允许更新的字段: {field}")));
        }
        let sql = format!("UPDATE brand SET \"{field}\" = $1 WHERE brand_id = $2");
        let r = sqlx::query(&sql)
            .bind(val)
            .bind(brand_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 设置审核状态（店铺品牌 `updateField` 后重新进入待审核）。
    pub async fn set_status(&self, brand_id: i32, status: i32) -> Result<()> {
        sqlx::query("UPDATE brand SET status = $1 WHERE brand_id = $2")
            .bind(status)
            .bind(brand_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 审核驳回：更新状态，仅在给出理由时同步 `reject_remark`。
    pub async fn audit(
        &self,
        brand_id: i32,
        status: i32,
        reject_remark: Option<&str>,
    ) -> Result<()> {
        if let Some(remark) = reject_remark {
            sqlx::query("UPDATE brand SET status = $1, reject_remark = $2 WHERE brand_id = $3")
                .bind(status)
                .bind(remark)
                .bind(brand_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
        } else {
            sqlx::query("UPDATE brand SET status = $1 WHERE brand_id = $2")
                .bind(status)
                .bind(brand_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
        }
        Ok(())
    }

    /// 重算单个品牌的首字母。
    pub async fn update_first_word(&self, brand_id: i32, first_word: &str) -> Result<u64> {
        let r = sqlx::query("UPDATE brand SET first_word = $1 WHERE brand_id = $2")
            .bind(first_word)
            .bind(brand_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }
}

/// 把列过滤条件按顺序绑定到查询上。
///
/// 顺序与 [`BrandRepository::paginate`] 构造 WHERE 片段的顺序一一对应：
/// keyword -> first_word -> status -> is_show -> brand_is_hot -> shop_id。
/// `audit_only` 片段（`shop_id <> 0`）是常量，不产生绑定参数。
fn bind_filters<'q, O>(
    q: sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments>,
    like: Option<&'q str>,
    fw: Option<&'q str>,
    status: Option<i32>,
    is_show: Option<i32>,
    brand_is_hot: Option<i32>,
    shop_id: Option<i32>,
) -> sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments> {
    let mut q = q;
    if like.is_some() {
        q = q.bind(like);
    }
    if fw.is_some() {
        q = q.bind(fw);
    }
    if status.is_some() {
        q = q.bind(status);
    }
    if is_show.is_some() {
        q = q.bind(is_show);
    }
    if brand_is_hot.is_some() {
        q = q.bind(brand_is_hot);
    }
    if shop_id.is_some() {
        q = q.bind(shop_id);
    }
    q
}

/// 排序字段白名单解析（未知或缺失回退主键，防 SQL 注入）。
fn resolve_sort_column(field: Option<&str>) -> &'static str {
    match field {
        Some(f) => {
            let snake = to_snake(f);
            SORTABLE_COLUMNS
                .iter()
                .find(|c| **c == snake)
                .copied()
                .unwrap_or("brand_id")
        }
        None => "brand_id",
    }
}

/// 驼峰 -> 下划线（允许前端传驼峰或数据库列名）。
fn to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for ch in s.chars() {
        if ch.is_ascii_uppercase() {
            out.push('_');
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("品牌不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_column_whitelist() {
        assert_eq!(resolve_sort_column(None), "brand_id");
        assert_eq!(resolve_sort_column(Some("sortOrder")), "sort_order");
        assert_eq!(resolve_sort_column(Some("sort_order")), "sort_order");
        assert_eq!(resolve_sort_column(Some("brandName")), "brand_name");
        assert_eq!(resolve_sort_column(Some("firstWord")), "first_word");
        // 未知字段回退主键，防 SQL 注入
        assert_eq!(resolve_sort_column(Some("1;DROP TABLE brand")), "brand_id");
    }

    #[test]
    fn brand_columns_match_entity_field_order() {
        assert_eq!(
            BRAND_COLUMNS,
            "brand_id, brand_name, brand_logo, brand_desc, site_url, sort_order, is_show, brand_type, brand_is_hot, first_word, is_store_brand, check_status, brand_en_name, shop_id, status, reject_remark"
        );
        assert_eq!(BRAND_COLUMNS.split(',').count(), 16);
    }

    #[test]
    fn snake_conversion_is_idempotent_for_snake_input() {
        assert_eq!(to_snake("brand_name"), "brand_name");
        assert_eq!(to_snake("brandName"), "brand_name");
    }
}