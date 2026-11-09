//! 分销素材仓储（marketing::salesman_material）。
//!
//! 主表 `salesman_material`（10 列，`python tools/schema_parser.py --table
//! salesman_material`）；只读关联：
//! - `salesman_material_category`（`category_id` / `category_name` /
//!   `add_time` / `shop_id` / `sort_order`）
//! - `product`（返回 `product_id` / `product_name` / `product_price` 精简对象）
//!
//! 查询统一运行时 `sqlx::query_as`。
//! 素材表**无逻辑删除列**：`del` / `batch(del)` 物理删除。

use rust_decimal::Decimal;
use sqlx::{FromRow,PgPool};

use nextrade_core::entities::{SalesmanMaterial, SalesmanMaterialCategory};
use nextrade_core::models::marketing::salesman_material::MSG_MATERIAL_NOT_FOUND;
use nextrade_core::{Error, Result};

/// `salesman_material` 固定 SELECT 列（10 列）。
pub const MATERIAL_COLUMNS: &str = "id, add_time, is_top, content, is_available, shop_id, pics, category_id, share_num, product_id";
/// `salesman_material_category` 固定 SELECT 列（5 列）。
pub const CATEGORY_COLUMNS: &str = "category_id, category_name, add_time, shop_id, sort_order";

/// 商品精简行。
#[derive(Debug, Clone, Default, FromRow)]
pub struct MaterialProductRow {
    pub product_id: i32,
    pub product_name: String,
    pub product_price: Option<Decimal>,
}

/// 素材仓储。
#[derive(Clone)]
pub struct SalesmanMaterialRepository {
    pool: PgPool,
}

impl SalesmanMaterialRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 管理端分页（Java `list`：店铺租户 + `categoryId` +
    /// `from=user → is_available=1`；排序白名单，缺省 `id DESC`）。
    ///
    /// `shop_id`：`None`（平台账号）不过滤；`Some(id)` 强制 `WHERE shop_id = ?`。
    pub async fn paginate(
        &self,
        shop_id: Option<i32>,
        category_id: Option<i32>,
        only_available: bool,
        sort_field: Option<&str>,
        sort_order: Option<&str>,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<SalesmanMaterial>)> {
        let mut where_parts: Vec<String> = Vec::new();
        if shop_id.is_some() {
            where_parts.push("shop_id = ?".to_string());
        }
        if category_id.is_some() {
            where_parts.push("category_id = ?".into());
        }
        if only_available {
            where_parts.push("is_available = 1".into());
        }
        let where_sql = if where_parts.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_parts.join(" AND "))
        };
        let column = resolve_sort_column(sort_field);
        let dir = if sort_order.is_some_and(|o| o.eq_ignore_ascii_case("asc")) {
            "ASC"
        } else {
            "DESC"
        };
        let count_sql = format!("SELECT COUNT(*) FROM salesman_material {where_sql}");
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        if let Some(shop_id) = shop_id {
            cq = cq.bind(shop_id);
        }
        if let Some(v) = category_id {
            cq = cq.bind(v);
        }
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let list_sql = format!(
            "SELECT {MATERIAL_COLUMNS} FROM salesman_material {where_sql} ORDER BY {column} {dir} LIMIT ? OFFSET ?"
        );
        let mut lq = sqlx::query_as::<_, SalesmanMaterial>(&list_sql);
        if let Some(shop_id) = shop_id {
            lq = lq.bind(shop_id);
        }
        if let Some(v) = category_id {
            lq = lq.bind(v);
        }
        let rows = lq
            .bind(limit.max(1) as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 按主键查询（Java `getById`）。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<SalesmanMaterial>> {
        let sql = format!("SELECT {MATERIAL_COLUMNS} FROM salesman_material WHERE id = $1 LIMIT 1");
        sqlx::query_as::<_, SalesmanMaterial>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 新增，返回主键。
    pub async fn insert(&self, m: &SalesmanMaterial) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO salesman_material (add_time, is_top, content, is_available, shop_id, pics, category_id, share_num, product_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
        )
        .bind(m.add_time)
        .bind(m.is_top)
        .bind(&m.content)
        .bind(m.is_available)
        .bind(m.shop_id)
        .bind(&m.pics)
        .bind(m.category_id)
        .bind(m.share_num)
        .bind(m.product_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 整行更新（`id` 之外 9 列）。
    pub async fn update(&self, m: &SalesmanMaterial) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE salesman_material SET add_time = $1, is_top = $2, content = $3, is_available = $4, shop_id = $5, pics = $6, category_id = $7, share_num = $8, product_id = $9 WHERE id = $10",
        )
        .bind(m.add_time)
        .bind(m.is_top)
        .bind(&m.content)
        .bind(m.is_available)
        .bind(m.shop_id)
        .bind(&m.pics)
        .bind(m.category_id)
        .bind(m.share_num)
        .bind(m.product_id)
        .bind(m.id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 物理删除（Java `removeById` / `removeByIds`）。
    pub async fn delete(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let ph = vec!["?"; ids.len()].join(", ");
        let sql = format!("DELETE FROM salesman_material WHERE id IN ({ph})");
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        let res = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 更新单个字段（白名单已在服务层归一，防注入）。
    pub async fn update_field(&self, id: i32, field: &str, val: &str) -> Result<bool> {
        if !matches!(field, "is_top" | "is_available") {
            return Err(Error::Validation(
                nextrade_core::models::marketing::salesman_material::MSG_INVALID_FIELD_VALUE.into(),
            ));
        }
        let sql = format!("UPDATE salesman_material SET \"{field}\" = $1 WHERE id = $2");
        let res = sqlx::query(&sql)
            .bind(val)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 批量取分类（`ids` 为空 → 全部，按 `sort_order` 升序）。
    pub async fn find_categories(
        &self,
        ids: Option<&[i32]>,
    ) -> Result<Vec<SalesmanMaterialCategory>> {
        match ids {
            Some(ids) if !ids.is_empty() => {
                let ph = vec!["?"; ids.len()].join(", ");
                let sql = format!(
                    "SELECT {CATEGORY_COLUMNS} FROM salesman_material_category WHERE category_id IN ({ph}) ORDER BY sort_order ASC"
                );
                let mut q = sqlx::query_as::<_, SalesmanMaterialCategory>(&sql);
                for id in ids {
                    q = q.bind(*id);
                }
                q.fetch_all(&self.pool).await.map_err(map_err)
            }
            Some(_) => Ok(Vec::new()),
            None => {
                let sql = format!(
                    "SELECT {CATEGORY_COLUMNS} FROM salesman_material_category ORDER BY sort_order ASC"
                );
                sqlx::query_as::<_, SalesmanMaterialCategory>(&sql)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(map_err)
            }
        }
    }

    /// 批量取商品精简信息（Java 内嵌完整 `Product`；见 models 差异 2）。
    pub async fn find_products(&self, ids: &[i32]) -> Result<Vec<MaterialProductRow>> {
        let ids: Vec<i32> = ids.iter().copied().filter(|id| *id > 0).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = vec!["?"; ids.len()].join(", ");
        let sql = format!(
            "SELECT product_id, product_name, product_price FROM product WHERE product_id IN ({ph})"
        );
        let mut q = sqlx::query_as::<_, MaterialProductRow>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }
}

/// 排序字段白名单（`salesman_material` 实存列；未知回退主键）。
#[must_use]
pub fn resolve_sort_column(field: Option<&str>) -> &'static str {
    match field {
        Some("id") => "id",
        Some("addTime") | Some("add_time") => "add_time",
        Some("isTop") | Some("is_top") => "is_top",
        Some("shareNum") | Some("share_num") => "share_num",
        Some("categoryId") | Some("category_id") => "category_id",
        _ => "id",
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound(MSG_MATERIAL_NOT_FOUND.into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_cover_entity() {
        assert_eq!(MATERIAL_COLUMNS.split(", ").count(), 10);
        assert_eq!(CATEGORY_COLUMNS.split(", ").count(), 5);
        assert!(!MATERIAL_COLUMNS.contains('*'));
    }

    #[test]
    fn sort_whitelist_falls_back_to_primary_key() {
        assert_eq!(resolve_sort_column(None), "id");
        assert_eq!(resolve_sort_column(Some("addTime")), "add_time");
        assert_eq!(resolve_sort_column(Some("isTop")), "is_top");
        assert_eq!(resolve_sort_column(Some("1;DROP TABLE salesman_material")), "id");
    }
}
