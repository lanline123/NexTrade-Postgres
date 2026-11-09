//! 分类/品牌仓储：`category` / `brand`，替代 `CategoryMapper` / `BrandMapper`。
//!
//! 对应原 Java `CategoryServiceImpl` + `BaseServiceImpl` 的数据访问部分。
//! 所有 SQL 列名已用 `tools/check_sql_columns.py` 校验。

use sqlx::PgPool;
use nextrade_core::entities::{Brand, Category};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// `category` 表允许的排序字段（对应 `BaseServiceImpl.getColumnByPropertyName`
/// 的防注入兜底：未知字段回退到主键）。
const SORTABLE_COLUMNS: [&str; 5] = [
    "category_id",
    "category_name",
    "parent_id",
    "sort_order",
    "is_show",
];

/// `category` 表允许通过 `updateField` 更新的字段。
/// 对应 Java `CheckFieldConstants.CATEGORY_FIELDS`。
pub const CATEGORY_FIELDS: [&str; 5] = [
    "category_name",
    "measure_unit",
    "is_hot",
    "is_show",
    "sort_order",
];

#[derive(Clone)]
pub struct CatalogRepository {
    pool: PgPool,
}

impl CatalogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 按 ID 查询分类。
    pub async fn find_category(&self, category_id: i32) -> Result<Option<Category>> {
        sqlx::query_as::<_, Category>("SELECT * FROM category WHERE category_id = $1 LIMIT 1")
            .bind(category_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 全部分类（构建分类树用）。
    pub async fn find_all_categories(&self) -> Result<Vec<Category>> {
        sqlx::query_as::<_, Category>(
            "SELECT * FROM category ORDER BY sort_order ASC, category_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 仅前台可见（`is_show = 1`）的分类。
    ///
    /// 对应 `CategoryServiceImpl.loadVisibleTreeSnapshot`：
    /// `lambdaQuery().eq(Category::getIsShow, Constants.YES).list()`。
    pub async fn find_visible_categories(&self) -> Result<Vec<Category>> {
        sqlx::query_as::<_, Category>(
            "SELECT * FROM category WHERE is_show = 1 ORDER BY sort_order ASC, category_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 分类分页列表。
    ///
    /// 对应 `CategoryServiceImpl.list(CategoryListDTO)`：
    /// - `parentId != null && parentId >= 0` 时按父级过滤
    /// - 排序字段经白名单校验（Java 侧找不到属性则回退主键）
    pub async fn paginate_categories(
        &self,
        parent_id: Option<i32>,
        sort_field: Option<&str>,
        sort_order: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<Category>)> {
        let column = resolve_sort_column(sort_field);
        let desc = !sort_order.is_some_and(|o| o.eq_ignore_ascii_case("asc"));
        let offset = (page.page.max(1) - 1) * page.page_size;

        // parent_id 为 None 或负数时不过滤（与 Java `>= 0` 判定一致）
        let filter = parent_id.filter(|p| *p >= 0);

        let sql_count = "SELECT COUNT(*) FROM category WHERE ($1 IS NULL OR parent_id = $2)";
        let total: (i64,) = sqlx::query_as(sql_count)
            .bind(filter)
            .bind(filter)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        // 排序方向只能是 ASC/DESC 字面量，列名来自白名单，无注入风险。
        let dir = if desc { "DESC" } else { "ASC" };
        let sql_rows = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM category WHERE (? IS NULL OR parent_id = ?) \
             ORDER BY {column} {dir} LIMIT ? OFFSET ?"
        ));
        let rows = sqlx::query_as::<_, Category>(&sql_rows)
            .bind(filter)
            .bind(filter)
            .bind(page.page_size as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0 as u64, rows))
    }

    /// 批量统计各分类的**直接**子分类数量，避免列表分页 N+1。
    ///
    /// 对应 `CategoryServiceImpl.countChildrenByParentIds`。
    pub async fn count_children_by_parent_ids(
        &self,
        parent_ids: &[i32],
    ) -> Result<Vec<(i32, i64)>> {
        if parent_ids.is_empty() {
            return Ok(Vec::new());
        }
        // SQLx 不支持动态 IN 列表绑定，需内联占位符。i32 无法被注入利用。
        let placeholders = std::iter::repeat_n("?", parent_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT parent_id, COUNT(*) FROM category \
             WHERE parent_id IN ({placeholders}) GROUP BY parent_id"
        ));
        let mut q = sqlx::query_as::<_, (i32, i64)>(&sql);
        for id in parent_ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 判断是否存在直接子分类（删除前校验用，`LIMIT 1`）。
    ///
    /// 对应 `CategoryServiceImpl.del` 的
    /// `lambdaQuery().eq(parentId, id).last("limit 1").one()`。
    pub async fn has_child(&self, parent_id: i32) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT category_id FROM category WHERE parent_id = $1 LIMIT 1",
        )
        .bind(parent_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.is_some())
    }

    /// 判断是否存在**不在给定集合内**的子分类（批量删除校验用）。
    ///
    /// 对应 `CategoryServiceImpl.batchDel` 的
    /// `.eq(parentId, id).notIn(categoryId, ids).last("limit 1").one()`。
    pub async fn has_child_excluding(&self, parent_id: i32, excluded: &[i32]) -> Result<bool> {
        if excluded.is_empty() {
            return self.has_child(parent_id).await;
        }
        let placeholders = std::iter::repeat_n("?", excluded.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT category_id FROM category \
             WHERE parent_id = ? AND category_id NOT IN ({placeholders}) LIMIT 1"
        ));
        let mut q = sqlx::query_as::<_, (i32,)>(&sql).bind(parent_id);
        for id in excluded {
            q = q.bind(id);
        }
        Ok(q.fetch_optional(&self.pool).await.map_err(map_err)?.is_some())
    }

    /// 统计挂在该分类下的商品数量（删除前校验用）。
    ///
    /// 对应 `productService.lambdaQuery().eq(Product::getCategoryId, id).count()`。
    pub async fn count_products_in_category(&self, category_id: i32) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM product WHERE category_id = $1")
                .bind(category_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0)
    }

    /// 校验同一父级下分类名称唯一性。
    ///
    /// 对应 `checkCategoryNameUnique`：
    /// `parentId` 为空时按顶级（`0`）处理；`exclude_id` 用于更新时排除自身。
    pub async fn exists_same_name(
        &self,
        category_name: &str,
        parent_id: Option<i32>,
        exclude_id: Option<i32>,
    ) -> Result<bool> {
        let actual_parent = parent_id.unwrap_or(0);
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT category_id FROM category \
             WHERE category_name = $1 AND parent_id = $2 \
             AND ($3 IS NULL OR category_id <> $4) LIMIT 1",
        )
        .bind(category_name)
        .bind(actual_parent)
        .bind(exclude_id)
        .bind(exclude_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.is_some())
    }

    /// 新增分类，返回自增主键。
    pub async fn insert_category(&self, c: &Category) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO category (category_name, seo_identification, keywords, category_desc, \
             parent_id, sort_order, measure_unit, is_show, seo_title, short_name, \
             category_pic, category_ico, is_hot, search_keywords) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14) RETURNING category_id",
        )
        .bind(&c.category_name)
        .bind(&c.seo_identification)
        .bind(&c.keywords)
        .bind(&c.category_desc)
        .bind(c.parent_id)
        .bind(c.sort_order)
        .bind(&c.measure_unit)
        .bind(c.is_show)
        .bind(&c.seo_title)
        .bind(&c.short_name)
        .bind(&c.category_pic)
        .bind(&c.category_ico)
        .bind(c.is_hot)
        .bind(&c.search_keywords)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 全量更新分类（除主键外的所有列）。
    pub async fn update_category(&self, c: &Category) -> Result<()> {
        sqlx::query(
            "UPDATE category SET category_name=$1, seo_identification=$2, keywords=$3, \
             category_desc=$4, parent_id=$5, sort_order=$6, measure_unit=$7, is_show=$8, \
             seo_title=$9, short_name=$10, category_pic=$11, category_ico=$12, is_hot=$13, \
             search_keywords=$14 WHERE category_id=$15",
        )
        .bind(&c.category_name)
        .bind(&c.seo_identification)
        .bind(&c.keywords)
        .bind(&c.category_desc)
        .bind(c.parent_id)
        .bind(c.sort_order)
        .bind(&c.measure_unit)
        .bind(c.is_show)
        .bind(&c.seo_title)
        .bind(&c.short_name)
        .bind(&c.category_pic)
        .bind(&c.category_ico)
        .bind(c.is_hot)
        .bind(&c.search_keywords)
        .bind(c.category_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 删除分类。
    pub async fn delete_category(&self, category_id: i32) -> Result<bool> {
        let r = sqlx::query("DELETE FROM category WHERE category_id = $1")
            .bind(category_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 按 ID 批量加载分类。
    pub async fn find_categories_by_ids(&self, ids: &[i32]) -> Result<Vec<Category>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT * FROM category WHERE category_id IN ({placeholders})"));
        let mut q = sqlx::query_as::<_, Category>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 通用字段更新：只更新白名单内的列。
    ///
    /// 对应 `BaseServiceImpl.updateField` + \"CATEGORY_FIELDS\"。
    /// 未知字段直接拒绝（Java 是回退主键静默更新，这里更严格，避免静默出错）。
    pub async fn update_category_field(
        &self,
        category_id: i32,
        field: &str,
        val: &str,
    ) -> Result<bool> {
        if !CATEGORY_FIELDS.contains(&field) {
            return Err(Error::Validation(format!("不允许更新的字段: {field}")));
        }
        // 列名来自常量白名单，非用户输入拼接。
        let sql = crate::db::to_pg_placeholders(&format!("UPDATE category SET \"{field}\" = ? WHERE category_id = ?"));
        let r = sqlx::query(&sql)
            .bind(val)
            .bind(category_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 批量删除分类。
    pub async fn delete_categories(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = crate::db::to_pg_placeholders(&format!("DELETE FROM category WHERE category_id IN ({placeholders})"));
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(id);
        }
        let r = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 把某分类下的商品迁移到目标分类（移动分类）。
    ///
    /// 对应 `CategoryServiceImpl.moveCat` 的
    /// `productMapper.update(...).eq(categoryId, id).set(categoryId, targetCatId)`。
    pub async fn move_products_category(&self, from: i32, to: i32) -> Result<u64> {
        let r = sqlx::query("UPDATE product SET category_id = $1 WHERE category_id = $2")
            .bind(to)
            .bind(from)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 品牌分页。
    pub async fn paginate_brands(
        &self,
        keyword: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<Brand>)> {
        let like = keyword.map(|k| format!("%{k}%"));
        let offset = (page.page.max(1) - 1) * page.page_size;

        let total: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM brand WHERE ($1 IS NULL OR brand_name LIKE $2)")
                .bind(&like)
                .bind(&like)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;

        let rows = sqlx::query_as::<_, Brand>(
            "SELECT * FROM brand WHERE ($1 IS NULL OR brand_name LIKE $2)
             ORDER BY brand_id DESC LIMIT $3 OFFSET $4",
        )
        .bind(&like)
        .bind(&like)
        .bind(page.page_size as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok((total.0 as u64, rows))
    }
}

/// 排序字段白名单解析：非法或缺失一律回退 `category_id`。
fn resolve_sort_column(field: Option<&str>) -> &'static str {
    match field {
        Some(f) => {
            let snake = to_snake(f);
            SORTABLE_COLUMNS
                .iter()
                .find(|c| **c == snake)
                .copied()
                .unwrap_or("category_id")
        }
        None => "category_id",
    }
}

/// `sortOrder` -> `sort_order`；允许前端传驼峰或下划线。
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
        sqlx::Error::RowNotFound => Error::NotFound("分类或品牌不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_column_whitelist() {
        assert_eq!(resolve_sort_column(None), "category_id");
        assert_eq!(resolve_sort_column(Some("sortOrder")), "sort_order");
        assert_eq!(resolve_sort_column(Some("sort_order")), "sort_order");
        assert_eq!(resolve_sort_column(Some("categoryName")), "category_name");
        // 未知字段回退主键，防止 SQL 注入
        assert_eq!(resolve_sort_column(Some("1;DROP TABLE category")), "category_id");
    }

    #[test]
    fn category_fields_match_java_constant() {
        // 必须与 Java CheckFieldConstants.CATEGORY_FIELDS 一致
        assert_eq!(
            CATEGORY_FIELDS,
            ["category_name", "measure_unit", "is_hot", "is_show", "sort_order"]
        );
    }

    #[test]
    fn snake_conversion_is_idempotent_for_snake_input() {
        assert_eq!(to_snake("sort_order"), "sort_order");
        assert_eq!(to_snake("parentId"), "parent_id");
    }
}
