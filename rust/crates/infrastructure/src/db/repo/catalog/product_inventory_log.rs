//! 库存日志仓储（`product_inventory_log`）。
//!
//! 对应原 Java `ProductInventoryLogMapper` 的 XML 查询
//! `productInventoryLog(page, pageQuery)`：
//!
//! ```sql
//! select *, product.product_name as productName
//! from product_inventory_log
//! left join product on product_inventory_log.product_id = product.product_id
//! <where>
//!   product_inventory_log.shop_id = #{pageQuery.shopId}
//!   [and product.product_name like ...]
//!   [and product_inventory_log.type = ...]
//! </where>
//! ```
//!
//! 列名对齐 `python tools/schema_parser.py --table product_inventory_log`（10 列）。
//! 全部 SQL 为运行时 `sqlx::query_as` / `sqlx::query`（禁止 `query!` 宏）。

use sqlx::PgPool;

use nextrade_core::entities::{Product, ProductSku};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// 动态 WHERE 的绑定值。
pub enum Bind {
    I32(i32),
    Str(String),
}

/// 把 `Bind` 列表按顺序绑定到查询上（宏展开以适配不同返回行类型）。
macro_rules! bind_all {
    ($q:expr, $binds:expr) => {{
        let mut q = $q;
        for b in $binds.iter() {
            q = match b {
                Bind::I32(v) => q.bind(*v),
                Bind::Str(v) => q.bind(v.as_str()),
            };
        }
        q
    }};
}

/// 库存日志 + 商品名的联表投影（`LEFT JOIN product`）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct InventoryLogRow {
    pub log_id: i32,
    pub product_id: Option<i32>,
    pub spec_id: Option<i32>,
    pub number: Option<i32>,
    pub add_time: Option<i32>,
    pub old_number: Option<i32>,
    /// 列名 `type` 是 MySQL 保留字，查询里用反引号
    pub r#type: Option<i32>,
    pub change_number: Option<i32>,
    /// 列名 `desc` 是 MySQL 保留字，查询里用反引号
    pub desc: Option<String>,
    pub shop_id: i32,
    /// `product.product_name`
    pub product_name: Option<String>,
}

/// 列表过滤条件（`shop_id` 对应 Java 请求头店铺；`None` 表示不限制）。
#[derive(Debug, Clone, Default)]
pub struct InventoryLogFilters<'a> {
    pub keyword: Option<&'a str>,
    /// Java：`type != null && type != 0` 才过滤
    pub r#type: Option<i32>,
    pub shop_id: Option<i32>,
    pub sort_field: Option<&'a str>,
    pub sort_order: Option<&'a str>,
}

/// 排序字段白名单（Java `getColumnByPropertyName`：未知/带表前缀的属性
/// 会回退主键 `log_id`，因此 `init()` 写入的
/// `product_inventory_log.add_time` 实际不生效，最终按 `log_id desc` 排序）。
const SORTABLE_COLUMNS: [&str; 9] = [
    "log_id",
    "product_id",
    "spec_id",
    "number",
    "old_number",
    "type",
    "change_number",
    "add_time",
    "shop_id",
];

#[derive(Clone)]
pub struct ProductInventoryLogRepository {
    pool: PgPool,
}

impl ProductInventoryLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 库存日志分页（Java `productInventoryLog` XML）。
    pub async fn paginate(
        &self,
        f: &InventoryLogFilters<'_>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<InventoryLogRow>)> {
        let kw = f
            .keyword
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(|k| format!("%{k}%"));

        let mut where_sql = String::from(" WHERE 1 = 1");
        if kw.is_some() {
            where_sql.push_str(" AND p.product_name LIKE ?");
        }
        if f.r#type.is_some_and(|t| t != 0) {
            where_sql.push_str(" AND l.\"type\" = ?");
        }
        if f.shop_id.is_some() {
            where_sql.push_str(" AND l.shop_id = ?");
        }

        let column = resolve_sort_column(f.sort_field);
        // `type` 是 MySQL 保留字，ORDER BY 里必须加反引号
        let column_sql = if column == "type" { "\"type\"" } else { column };
        let dir = if f.sort_order.is_some_and(|o| o.eq_ignore_ascii_case("asc")) {
            "ASC"
        } else {
            "DESC"
        };
        let order_sql = format!(" ORDER BY l.{column_sql} {dir}");

        let binds = build_binds(f, kw.clone());

        let from_sql = " FROM product_inventory_log l LEFT JOIN product p ON l.product_id = p.product_id";
        let count_sql = format!("SELECT COUNT(*){from_sql}{where_sql}");
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&count_sql), &binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let (limit, offset) = page.limit_offset();
        let list_sql = format!(
            "SELECT l.log_id, l.product_id, l.spec_id, l.number, l.add_time, l.old_number, \
             l.\"type\", l.change_number, l.\"desc\", l.shop_id, p.product_name AS product_name\
             {from_sql}{where_sql}{order_sql} LIMIT ? OFFSET ?"
        );
        let rows = bind_all!(
            sqlx::query_as::<_, InventoryLogRow>(&list_sql),
            &binds
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 批量取 SKU（日志行 `spec_id > 0` 时回填 `sku_sn` / `sku_data`）。
    pub async fn find_skus(&self, sku_ids: &[i32]) -> Result<Vec<ProductSku>> {
        if sku_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; sku_ids.len()].join(", ");
        let sql = format!("SELECT * FROM product_sku WHERE sku_id IN ({placeholders})");
        let mut q = sqlx::query_as::<_, ProductSku>(&sql);
        for id in sku_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量取商品（日志行 `spec_id = 0` 时回填 `product_sn`）。
    pub async fn find_products(&self, product_ids: &[i32]) -> Result<Vec<Product>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; product_ids.len()].join(", ");
        let sql = format!("SELECT * FROM product WHERE product_id IN ({placeholders})");
        let mut q = sqlx::query_as::<_, Product>(&sql);
        for id in product_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }
}

/// 按 WHERE 片段顺序生成绑定值：keyword → type → shop_id。
fn build_binds(f: &InventoryLogFilters<'_>, kw: Option<String>) -> Vec<Bind> {
    let mut binds = Vec::new();
    if let Some(k) = kw {
        binds.push(Bind::Str(k));
    }
    if let Some(t) = f.r#type.filter(|t| *t != 0) {
        binds.push(Bind::I32(t));
    }
    if let Some(s) = f.shop_id {
        binds.push(Bind::I32(s));
    }
    binds
}

/// 排序字段解析：驼峰/列名 → 白名单列；带表前缀或未知值回退 \"log_id\"
/// （与 Java \"getColumnByPropertyName\" 的兜底一致）。
fn resolve_sort_column(field: Option<&str>) -> &'static str {
    let Some(field) = field.map(str::trim).filter(|s| !s.is_empty()) else {
        return "log_id";
    };
    if field.contains('.') {
        return "log_id";
    }
    let snake = to_snake(field);
    SORTABLE_COLUMNS
        .iter()
        .find(|c| **c == snake)
        .copied()
        .unwrap_or("log_id")
}

/// 驼峰 → 下划线。
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
        sqlx::Error::RowNotFound => Error::NotFound("库存日志不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_column_falls_back_to_primary_key_like_java() {
        // Java `init()` 写入 "product_inventory_log.add_time"，但
        // getColumnByPropertyName 找不到该属性 → 回退主键 log_id
        assert_eq!(
            resolve_sort_column(Some("product_inventory_log.add_time")),
            "log_id"
        );
        assert_eq!(resolve_sort_column(None), "log_id");
        assert_eq!(resolve_sort_column(Some("")), "log_id");
        assert_eq!(resolve_sort_column(Some("addTime")), "add_time");
        assert_eq!(resolve_sort_column(Some("changeNumber")), "change_number");
        assert_eq!(resolve_sort_column(Some("type")), "type");
        // 未知字段防注入
        assert_eq!(resolve_sort_column(Some("1;DROP TABLE x")), "log_id");
    }

    #[test]
    fn snake_conversion_is_idempotent() {
        assert_eq!(to_snake("add_time"), "add_time");
        assert_eq!(to_snake("oldNumber"), "old_number");
    }
}
