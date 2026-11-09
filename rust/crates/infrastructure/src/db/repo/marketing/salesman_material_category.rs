//! 素材分类域仓储：`salesman_material_category`（兼查 `salesman_material` 做删除前校验）。
//! 对应原 Java `SalesmanMaterialCategoryMapper` / `SalesmanMaterialMapper`。
//!
//! 列名严格对齐 `java/database/pure.sql`：
//! - `salesman_material_category`：`category_id` / `category_name` / `add_time` /
//!   `shop_id` / `sort_order`
//! - `salesman_material`：`id` / `category_id`（仅用于关联计数）

use sqlx::PgPool;
use nextrade_core::entities::SalesmanMaterialCategory;
use nextrade_core::models::marketing::salesman_material_category::SalesmanMaterialCategoryPageQuery;
use nextrade_core::{Error, Result};

/// 允许参与 `ORDER BY` 的列白名单（防止 SQL 注入）。
///
/// 对应 Java `BaseServiceImpl#buildSortOrder` + `getColumnByPropertyName`：
/// 会把属性名归一成列名。这里的白名单是 `salesman_material_category` 的
/// **真实列**；归一化后仍不在此列的，退回主键 `category_id`
/// （与 Java `getColumnByPropertyName` 找不到时返回主键列的行为一致）。
const SORTABLE_COLUMNS: [&str; 5] = [
    "category_id",
    "category_name",
    "add_time",
    "shop_id",
    "sort_order",
];

/// 解析排序：返回 `(列名, 是否降序)`。
///
/// 复刻 Java `buildSortOrder(page, sortField, sortOrder)`：
/// - `sortField` 为空 → 主键 `category_id`
/// - `sortOrder == "asc"` → 升序；`desc` 或**为空** → 降序
#[must_use]
pub fn resolve_sort(sort_field: Option<&str>, sort_order: Option<&str>) -> (&'static str, bool) {
    // Java `getColumnByPropertyName`：先按属性名 / 列名归一，找不到则回退主键列。
    let requested = match sort_field.map(str::trim) {
        Some(s) if !s.is_empty() => s,
        _ => "category_id",
    };
    let normalized = match requested {
        "categoryId" => "category_id",
        "categoryName" => "category_name",
        "addTime" => "add_time",
        "shopId" => "shop_id",
        "sortOrder" => "sort_order",
        other => other,
    };
    let column = SORTABLE_COLUMNS
        .iter()
        .copied()
        .find(|c| *c == normalized)
        // Java 找不到列时回退主键，而不是报错
        .unwrap_or("category_id");

    let asc = sort_order
        .map(str::trim)
        .is_some_and(|o| o.eq_ignore_ascii_case("asc"));
    (column, !asc)
}

#[derive(Clone)]
pub struct SalesmanMaterialCategoryRepository {
    pool: PgPool,
}

impl SalesmanMaterialCategoryRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层在事务中复用）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 素材分类 —————————————————————

    /// 按 ID 查询素材分类。
    ///
    /// 对应 Java `SalesmanMaterialCategoryServiceImpl#detail` 里的 `getById(id)`。
    pub async fn find_by_id(&self, category_id: i32) -> Result<Option<SalesmanMaterialCategory>> {
        sqlx::query_as::<_, SalesmanMaterialCategory>(
            "SELECT * FROM salesman_material_category WHERE category_id = $1 LIMIT 1",
        )
        .bind(category_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 素材分类分页（按名称模糊搜索 + 动态排序）。
    ///
    /// 对应 Java `SalesmanMaterialCategoryServiceImpl#list`。
    /// ⚠️ Java 中「按 `shopId` 过滤」的那行代码是**注释掉的**，
    /// 所以这里**不做**店铺过滤，保持逐字一致。
    pub async fn paginate(
        &self,
        q: &SalesmanMaterialCategoryPageQuery,
        shop_id: Option<i32>,
    ) -> Result<(u64, Vec<SalesmanMaterialCategory>)> {
        let like = q
            .category_name
            .as_deref()
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(|k| format!("%{k}%"));

        let (limit, offset) = q.limit_offset();
        let (column, desc) = resolve_sort(q.sort_field.as_deref(), q.sort_order.as_deref());
        let direction = if desc { "DESC" } else { "ASC" };

        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM salesman_material_category
             WHERE ($1 IS NULL OR category_name LIKE $2)
               AND ($3 IS NULL OR shop_id = $4)",
        )
        .bind(&like)
        .bind(&like)
        .bind(shop_id)
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        // 列名来自 resolve_sort 的白名单，已消除注入面；
        // 不能用绑定参数占位（MySQL 不允许 ORDER BY 用占位符）。
        let sql = format!(
            "SELECT * FROM salesman_material_category
             WHERE ($1 IS NULL OR category_name LIKE $2)
               AND ($3 IS NULL OR shop_id = $4)
             ORDER BY {column} {direction}
             LIMIT $5 OFFSET $6"
        );
        let rows = sqlx::query_as::<_, SalesmanMaterialCategory>(&sql)
            .bind(&like)
            .bind(&like)
            .bind(shop_id)
            .bind(shop_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 同店铺下分类名是否已存在（排除指定 ID）。
    ///
    /// 对应 Java `create` / `update` 的
    /// `lambdaQuery().eq(categoryName, ..).eq(shopId, ..)[.ne(categoryId, ..)].count()`。
    /// `exclude_id` 为 `None` 时即「新建」场景。
    pub async fn exists_name(
        &self,
        category_name: &str,
        shop_id: i32,
        exclude_id: Option<i32>,
    ) -> Result<bool> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM salesman_material_category
             WHERE category_name = $1
               AND shop_id = $2
               AND ($3 IS NULL OR category_id <> $4)",
        )
        .bind(category_name)
        .bind(shop_id)
        .bind(exclude_id)
        .bind(exclude_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0 > 0)
    }

    /// 新增素材分类，返回 `category_id`。
    pub async fn insert(&self, c: &SalesmanMaterialCategory) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO salesman_material_category
             (category_name, add_time, shop_id, sort_order)
             VALUES ($1, $2, $3, $4) RETURNING category_id",
        )
        .bind(&c.category_name)
        .bind(c.add_time)
        .bind(c.shop_id)
        .bind(c.sort_order)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 更新素材分类名称与排序。
    ///
    /// 对应 Java `updateById`：Java 侧是「非 null 字段才更新」的
    /// 选择性更新。这里由服务层先读出原行再合并，语义等价。
    pub async fn update(&self, c: &SalesmanMaterialCategory) -> Result<()> {
        let affected = sqlx::query(
            "UPDATE salesman_material_category
             SET category_name = $1, sort_order = $2
             WHERE category_id = $3",
        )
        .bind(&c.category_name)
        .bind(c.sort_order)
        .bind(c.category_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        if affected == 0 {
            return Err(Error::NotFound("素材分类不存在".into()));
        }
        Ok(())
    }

    /// 按主键删除素材分类。
    ///
    /// 对应 Java `BaseServiceImpl#del` → `removeById(id)`。
    pub async fn delete(&self, category_id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM salesman_material_category WHERE category_id = $1")
            .bind(category_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 批量删除素材分类（`batch type=del`）。
    ///
    /// 对应 Java `BaseServiceImpl#batch` → `removeByIds(ids)`，
    /// 返回影响行数（Java 里被强转成 `boolean`）。
    pub async fn delete_many(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql =
            format!("DELETE FROM salesman_material_category WHERE category_id IN ({placeholders})");
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(id);
        }
        let r = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 更新单个字段（`updateField`）。
    ///
    /// `column` **必须**来自
    /// [`nextrade_core::models::marketing::salesman_material_category::GENERAL_FIELDS`] 白名单校验之后的值，
    /// 调用方负责保证；这里再做一次本地断言式白名单兜底。
    pub async fn update_field(&self, category_id: i32, column: &str, val: &str) -> Result<u64> {
        // 白名单兜底：只有本表真实存在的列可写
        let safe = match column {
            "sort_order" => "sort_order",
            "is_show" | "status" => {
                // 本表无此列；Java 会走到 DB 层报未知列。
                return Err(Error::Database(format!("Unknown column '{column}'")));
            }
            _ => return Err(Error::Validation("field 的值有误".into())),
        };
        let sql = format!(
            "UPDATE salesman_material_category SET {safe} = $1 WHERE category_id = $2"
        );
        let affected = sqlx::query(&sql)
            .bind(val)
            .bind(category_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected)
    }

    // ————————————————————— 关联素材计数 —————————————————————

    /// 统计该分类下的素材数量。
    ///
    /// 对应 Java `SalesmanMaterialCategoryServiceImpl#delete` 里的
    /// `salesmanMaterialMapper.selectCount(eq(SalesmanMaterial::getCategoryId, id))`。
    pub async fn count_materials(&self, category_id: i32) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM salesman_material WHERE category_id = $1")
                .bind(category_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("素材分类不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sort_is_primary_key_desc() {
        // Java buildSortOrder：sortField 为空 → 主键 desc
        assert_eq!(resolve_sort(None, None), ("category_id", true));
        assert_eq!(resolve_sort(Some(""), None), ("category_id", true));
    }

    #[test]
    fn asc_and_desc_are_recognized() {
        assert_eq!(resolve_sort(Some("sort_order"), Some("asc")), ("sort_order", false));
        assert_eq!(resolve_sort(Some("sort_order"), Some("DESC")), ("sort_order", true));
    }

    #[test]
    fn missing_sort_order_is_desc_per_java() {
        // Java：sortOrder 为空 → desc 分支
        assert_eq!(resolve_sort(Some("sort_order"), None), ("sort_order", true));
        // 非 asc/desc 的取值 Java 不设 ORDER BY；这里保守回退主键 desc
        assert_eq!(resolve_sort(Some("category_name"), Some("weird")), ("category_name", true));
    }

    #[test]
    fn property_names_are_normalized_to_columns() {
        assert_eq!(resolve_sort(Some("sortOrder"), Some("asc")), ("sort_order", false));
        assert_eq!(resolve_sort(Some("categoryName"), Some("asc")), ("category_name", false));
        assert_eq!(resolve_sort(Some("addTime"), None), ("add_time", true));
    }

    #[test]
    fn unknown_sort_column_falls_back_to_primary_key() {
        // 防注入：任意非法列名都退回主键
        assert_eq!(resolve_sort(Some("id; DROP TABLE x"), None), ("category_id", true));
        assert_eq!(resolve_sort(Some("password"), Some("asc")), ("category_id", false));
    }
}