//! 商品批量导入/导出仓储（`catalog/product_batch`）。
//!
//! 对应原 Java `ProductBatchServiceImpl#productBatchDeal` 中直接用到的 Mapper：
//! `Product`（按 分类/品牌/商品/排除 范围导出）、`ProductSku`
//! （`productSkuService.getProductSkusByProductIds`）、`ProductAttribute`
//! （`loadSpecPicDict`，`attr_type = 1` 的规格图）、`Category`
//! （`categoryService.lambdaQuery().list()` 构建分类路径），以及
//! `ServicesService#list` 的 `product_services` 分页。
//!
//! 全部 SQL 为运行时 `sqlx::query_as` / `QueryBuilder`（禁止 `query!` 宏）。

use sqlx::PgPool;
use sqlx::{Postgres, QueryBuilder};

use nextrade_core::entities::{Category, Product, ProductServices, ProductSku};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// 规格图字典行（`product_attributes` 的 `attr_type = 1` 子集）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct SpecAttrRow {
    pub product_id: i32,
    pub attr_name: String,
    pub attr_value: String,
    pub attr_pic: Option<String>,
}

#[derive(Clone)]
pub struct ProductBatchRepository {
    pool: PgPool,
}

impl ProductBatchRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 全部商品分类（Java `categoryService.lambdaQuery().list()`）。
    pub async fn find_categories(&self) -> Result<Vec<Category>> {
        sqlx::query_as::<_, Category>("SELECT * FROM category ORDER BY category_id ASC")
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按导出范围查询商品（Java `productBatchDeal` 的四段 lambdaQuery）。
    ///
    /// 固定条件：`is_delete = 0 AND check_status = 1 AND product_status = 1`
    /// （审核通过 + 上架 + 未删除）。`deal_range`：
    /// - `1` 分类：`category_id IN (展开后的子孙分类)`
    /// - `2` 品牌：`brand_id IN range_ids`
    /// - `3` 商品：`product_id IN range_ids`
    /// - 其它（4/缺省）：`range_ids` 非空时 `product_id NOT IN range_ids`
    ///
    /// ⚠️ Java 对店铺账号（请求头 `shopId > 0`）额外加 `shop_id = 本店`；
    /// Rust 无登录态店铺来源，当前不做店铺过滤（见服务层不确定点）。
    pub async fn find_products_for_export(
        &self,
        deal_range: Option<i32>,
        range_ids: &[i32],
        category_ids: &[i32],
    ) -> Result<Vec<Product>> {
        let mut qb = QueryBuilder::<Postgres>::new(
            "SELECT * FROM product WHERE is_delete = 0 AND check_status = 1 AND product_status = 1",
        );
        match deal_range {
            Some(1) => {
                if category_ids.is_empty() {
                    // Java：分类展开为空时不会执行查询（products 保持空列表）
                    return Ok(Vec::new());
                }
                push_in_i32(&mut qb, "category_id", category_ids);
            }
            Some(2) => {
                if !range_ids.is_empty() {
                    push_in_i32(&mut qb, "brand_id", range_ids);
                }
            }
            Some(3) => {
                if !range_ids.is_empty() {
                    push_in_i32(&mut qb, "product_id", range_ids);
                }
            }
            _ => {
                if !range_ids.is_empty() {
                    qb.push(" AND product_id NOT IN (");
                    for (i, id) in range_ids.iter().enumerate() {
                        if i > 0 {
                            qb.push(", ");
                        }
                        qb.push_bind(*id);
                    }
                    qb.push(")");
                }
            }
        }
        qb.push(" ORDER BY product_id ASC");
        qb.build_query_as::<Product>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 批量取商品 SKU（Java `productSkuService.getProductSkusByProductIds`）。
    pub async fn find_skus(&self, product_ids: &[i32]) -> Result<Vec<ProductSku>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; product_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM product_sku WHERE product_id IN ({placeholders}) ORDER BY sku_id ASC"
        ));
        let mut q = sqlx::query_as::<_, ProductSku>(&sql);
        for id in product_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 规格图字典（Java `loadSpecPicDict`：`attr_type = 1` 且有 `attr_pic`）。
    pub async fn find_spec_attrs(&self, product_ids: &[i32]) -> Result<Vec<SpecAttrRow>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; product_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT product_id, attr_name, attr_value, attr_pic FROM product_attributes \
             WHERE attr_type = 1 AND attr_pic IS NOT NULL AND attr_pic <> '' \
             AND product_id IN ({placeholders}) ORDER BY attributes_id ASC"
        ));
        let mut q = sqlx::query_as::<_, SpecAttrRow>(&sql);
        for id in product_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 商品服务分页（Java `ServicesServiceImpl#list(BasePage)`，表
    /// `product_services`）：`keyword` 模糊匹配服务名；排序缺省主键降序
    /// （Java `buildSortOrder` 空 sortField → 主键、空 sortOrder → desc）。
    pub async fn paginate_services(
        &self,
        keyword: Option<&str>,
        sort_field: Option<&str>,
        sort_order: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<ProductServices>)> {
        let kw = keyword
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(|k| format!("%{k}%"));
        let where_sql = if kw.is_some() {
            " WHERE product_service_name LIKE $1"
        } else {
            ""
        };
        let total: (i64,) = if let Some(k) = &kw {
            sqlx::query_as::<_, (i64,)>(&format!("SELECT COUNT(*) FROM product_services{where_sql}"))
                .bind(k)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?
        } else {
            sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM product_services")
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?
        };

        let column = resolve_service_sort_column(sort_field);
        let dir = if sort_order.is_some_and(|o| o.eq_ignore_ascii_case("asc")) {
            "ASC"
        } else {
            "DESC"
        };
        let (limit, offset) = page.limit_offset();
        let list_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM product_services{where_sql} ORDER BY {column} {dir} LIMIT ? OFFSET ?"
        ));
        let rows = if let Some(k) = &kw {
            sqlx::query_as::<_, ProductServices>(&list_sql)
                .bind(k)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?
        } else {
            sqlx::query_as::<_, ProductServices>(&list_sql)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?
        };
        Ok((total.0.max(0) as u64, rows))
    }
}

/// 商品服务排序白名单（Java `getColumnByPropertyName`：未知字段回退主键）。
fn resolve_service_sort_column(field: Option<&str>) -> &'static str {
    match field.map(str::trim).filter(|s| !s.is_empty()) {
        Some("sortOrder") | Some("sort_order") => "sort_order",
        Some("productServiceName") | Some("product_service_name") => "product_service_name",
        Some("defaultOn") | Some("default_on") => "default_on",
        Some("productServiceId") | Some("product_service_id") | None => "product_service_id",
        Some(_) => "product_service_id",
    }
}

/// 追加 `AND <column> IN (...)`（值由 `QueryBuilder` 绑定，防注入）。
fn push_in_i32(qb: &mut QueryBuilder<'_, Postgres>, column: &str, ids: &[i32]) {
    qb.push(" AND ").push(column).push(" IN (");
    for (i, id) in ids.iter().enumerate() {
        if i > 0 {
            qb.push(", ");
        }
        qb.push_bind(*id);
    }
    qb.push(")");
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("商品不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_in_uses_placeholders_not_literals() {
        let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM product WHERE 1 = 1");
        push_in_i32(&mut qb, "category_id", &[1, 2, 3]);
        let sql = qb.sql().to_string();
        qb.build();
        assert!(sql.contains("category_id IN ("));
        assert_eq!(sql.matches('$').count(), 3);
        assert!(!sql.contains("IN (1"), "值必须走绑定参数，不能拼进 SQL");
    }
}
