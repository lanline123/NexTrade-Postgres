//! 分销商品域仓储（marketing::salesman_product）。
//!
//! 表：`salesman_product`（8 列）、`product`、`product_gallery`、`salesman`、
//! `salesman_config`、`salesman_order`（分析聚合）。
//!
//! SQL 一律运行时 `sqlx`（不用 `query!` 宏）。

use rust_decimal::Decimal;
use sqlx::PgPool;

use nextrade_core::entities::{Product, ProductGallery, Salesman, SalesmanConfig, SalesmanProduct};
use nextrade_core::models::marketing::salesman_product::SalesmanProductPageQuery;
use nextrade_core::{Error, Result};

/// `salesman_product` 列清单（8 列）。
pub const SALESMAN_PRODUCT_COLUMNS: [&str; 8] = [
    "salesman_product_id",
    "product_id",
    "is_join",
    "commission_type",
    "commission_data",
    "add_time",
    "update_time",
    "shop_id",
];

/// 列表联表行（商品核心字段 + 分销配置）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SalesmanProductRow {
    pub product_id: i32,
    pub product_name: String,
    pub product_sn: String,
    pub product_tsn: Option<String>,
    pub product_stock: i32,
    pub product_price: Decimal,
    pub market_price: Decimal,
    pub cost_price: Option<Decimal>,
    pub product_status: i32,
    pub product_type: Option<i32>,
    pub pic_url: String,
    pub pic_thumb: String,
    pub is_delete: i32,
    pub add_time: i32,
    pub last_update: i32,
    pub salesman_product_id: Option<i32>,
    pub is_join: Option<i32>,
    pub commission_type: Option<i32>,
    pub commission_data: Option<String>,
    pub sp_add_time: Option<i32>,
    pub sp_update_time: Option<i32>,
}

/// 分销商品域仓储。
#[derive(Clone)]
pub struct SalesmanProductRepository {
    pool: PgPool,
}

impl SalesmanProductRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }
}

impl SalesmanProductRepository {
    /// 商品列表（Java `productService.salesmanProducts(pageQuery)` 的等价 SQL）。
    pub async fn paginate(
        &self,
        q: &SalesmanProductPageQuery,
        shop_id: i32,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<SalesmanProductRow>)> {
        let (from_where, binds) = build_from_where(q, shop_id);
        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM {from_where}"));
        let mut count_q = sqlx::query_as::<_, (i64,)>(&count_sql);
        for b in &binds {
            count_q = match b {
                Bind::I32(v) => count_q.bind(*v),
                Bind::Str(v) => count_q.bind(v.as_str()),
            };
        }
        let total: (i64,) = count_q.fetch_one(&self.pool).await.map_err(map_err)?;

        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT p.product_id, p.product_name, p.product_sn, p.product_tsn, p.product_stock, p.product_price, p.market_price, p.cost_price, p.product_status, p.product_type, p.pic_url, p.pic_thumb, p.is_delete, p.add_time, p.last_update, sp.salesman_product_id, sp.is_join, sp.commission_type, sp.commission_data, sp.add_time AS sp_add_time, sp.update_time AS sp_update_time FROM {from_where} ORDER BY p.product_id DESC LIMIT {limit} OFFSET {offset}"
        ));
        let mut rows_q = sqlx::query_as::<_, SalesmanProductRow>(&rows_sql);
        for b in &binds {
            rows_q = match b {
                Bind::I32(v) => rows_q.bind(*v),
                Bind::Str(v) => rows_q.bind(v.as_str()),
            };
        }
        let rows = rows_q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total.0, rows))
    }

    /// 按商品 ID 查分销配置（Java `detailByProductId`：`.eq(productId).one()`）。
    pub async fn find_by_product(&self, product_id: i32) -> Result<Option<SalesmanProduct>> {
        let sql = format!(
            "SELECT {} FROM salesman_product WHERE product_id = $1 LIMIT 1",
            SALESMAN_PRODUCT_COLUMNS.join(", ")
        );
        sqlx::query_as::<_, SalesmanProduct>(&sql)
            .bind(product_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    pub async fn find_by_id(&self, id: i32) -> Result<Option<SalesmanProduct>> {
        let sql = format!(
            "SELECT {} FROM salesman_product WHERE salesman_product_id = $1 LIMIT 1",
            SALESMAN_PRODUCT_COLUMNS.join(", ")
        );
        sqlx::query_as::<_, SalesmanProduct>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    pub async fn insert(&self, p: &SalesmanProduct) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO salesman_product (product_id, is_join, commission_type, commission_data, add_time, update_time, shop_id) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING salesman_product_id",
        )
        .bind(p.product_id)
        .bind(p.is_join)
        .bind(p.commission_type)
        .bind(&p.commission_data)
        .bind(p.add_time)
        .bind(p.update_time)
        .bind(p.shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    pub async fn update(&self, p: &SalesmanProduct) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE salesman_product SET product_id = $1, is_join = $2, commission_type = $3, commission_data = $4, add_time = $5, update_time = $6, shop_id = $7 WHERE salesman_product_id = $8",
        )
        .bind(p.product_id)
        .bind(p.is_join)
        .bind(p.commission_type)
        .bind(&p.commission_data)
        .bind(p.add_time)
        .bind(p.update_time)
        .bind(p.shop_id)
        .bind(p.salesman_product_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 按主键批量删除（Java `BaseServiceImpl#batch(type=del)` → `removeByIds`）。
    pub async fn delete_many(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "DELETE FROM salesman_product WHERE salesman_product_id IN ({placeholders})"
        ));
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        let res = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 商品详情（`SELECT *` 映射实体，用于客户端详情 / 分析）。
    pub async fn product_by_id(&self, product_id: i32) -> Result<Option<Product>> {
        sqlx::query_as::<_, Product>("SELECT * FROM product WHERE product_id = $1 LIMIT 1")
            .bind(product_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 商品相册（Java `productGalleryMapper.selectList(... orderByAsc(sortOrder))`）。
    pub async fn galleries(&self, product_id: i32) -> Result<Vec<ProductGallery>> {
        sqlx::query_as::<_, ProductGallery>(
            "SELECT * FROM product_gallery WHERE product_id = $1 ORDER BY sort_order ASC",
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 分销配置 JSON（Java `configService.detail("salesmanConfig")`）。
    pub async fn config_by_code(&self, code: &str) -> Result<Option<SalesmanConfig>> {
        sqlx::query_as::<_, SalesmanConfig>(
            "SELECT * FROM salesman_config WHERE code = $1 LIMIT 1",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 当前用户的分销员记录（Java `salesmanMapper.selectOne(userId=...)`）。
    pub async fn salesman_by_user(&self, user_id: i32) -> Result<Option<Salesman>> {
        sqlx::query_as::<_, Salesman>("SELECT * FROM salesman WHERE user_id = $1 LIMIT 1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 商品成交聚合（Java `analysis` 的 `salesman_order` 汇总）。
    ///
    /// 返回 `(成交金额合计, 已结算佣金合计)`。
    pub async fn order_stats(
        &self,
        product_id: i32,
        start_time: Option<i32>,
        end_time: Option<i32>,
    ) -> Result<(Decimal, Decimal)> {
        let mut sql = String::from(
            "SELECT COALESCE(SUM(order_amount), 0), COALESCE(SUM(CASE WHEN status = 1 THEN amount ELSE 0 END), 0) FROM salesman_order WHERE product_id = ",
        );
        sql.push_str(&product_id.to_string());
        if let Some(start) = start_time {
            sql.push_str(&format!(" AND add_time >= {start}"));
        }
        if let Some(end) = end_time {
            sql.push_str(&format!(" AND add_time <= {end}"));
        }
        let row: (Decimal, Decimal) = sqlx::query_as(&sql)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row)
    }

    /// 参与分销的商品分页（Java `analysis` 的 `is_join=1` + `shop_id`）。
    pub async fn joined_products(
        &self,
        shop_id: i32,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<SalesmanProduct>)> {
        let sql = format!(
            "SELECT {} FROM salesman_product WHERE shop_id = $1 AND is_join = 1",
            SALESMAN_PRODUCT_COLUMNS.join(", ")
        );
        let total: (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM ({sql}) t"))
            .bind(shop_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let rows = sqlx::query_as::<_, SalesmanProduct>(&format!(
            "{sql} ORDER BY salesman_product_id ASC LIMIT {limit} OFFSET {offset}"
        ))
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((total.0, rows))
    }
}

/// 待绑定参数。
#[derive(Debug, Clone)]
pub enum Bind {
    I32(i32),
    Str(String),
}

/// 构建 `FROM product p LEFT JOIN salesman_product sp` + WHERE（列表口径）。
#[must_use]
pub fn build_from_where(q: &SalesmanProductPageQuery, shop_id: i32) -> (String, Vec<Bind>) {
    let mut from = "product p LEFT JOIN salesman_product sp ON sp.product_id = p.product_id".to_string();
    let mut conds = vec!["p.is_delete = 0".to_string()];
    let mut binds: Vec<Bind> = Vec::new();
    // 店铺条件：Java 对 sp.shop_id 过滤（未配置分销时 sp 行为 NULL，仍展示商品）
    if shop_id > 0 {
        from = "product p LEFT JOIN salesman_product sp ON sp.product_id = p.product_id AND sp.shop_id = ?".to_string();
        binds.push(Bind::I32(shop_id));
    }
    if let Some(name) = q.effective_name() {
        conds.push("p.product_name ILIKE ?".to_string());
        binds.push(Bind::Str(format!("%{name}%")));
    }
    if let Some(status) = q.status {
        conds.push("p.product_status = ?".to_string());
        binds.push(Bind::I32(status));
    }
    if let Some(is_join) = q.is_join {
        conds.push("sp.is_join = ?".to_string());
        binds.push(Bind::I32(is_join));
    }
    match q.salesman_product_type {
        Some(1) => conds.push("(sp.is_join = 0 OR sp.is_join IS NULL)".to_string()),
        Some(2) => conds.push("sp.commission_type = 2".to_string()),
        Some(3) => conds.push("sp.is_join = 1".to_string()),
        Some(4) => conds.push("sp.commission_type = 3".to_string()),
        _ => {}
    }
    (format!("{from} WHERE {}", conds.join(" AND ")), binds)
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(json: serde_json::Value) -> SalesmanProductPageQuery {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn list_where_filters_and_join() {
        let (sql, binds) = build_from_where(&q(serde_json::json!({})), 0);
        assert!(sql.starts_with("product p LEFT JOIN salesman_product sp"));
        assert!(sql.contains("p.is_delete = 0"));
        assert!(binds.is_empty());

        let (sql, binds) = build_from_where(
            &q(serde_json::json!({"keyword": "手机", "status": 1, "isJoin": 1})),
            5,
        );
        assert!(sql.contains("AND sp.shop_id = ?"));
        assert_eq!(binds.len(), 4);
        assert!(matches!(&binds[1], Bind::Str(s) if s == "%手机%"));
        assert!(matches!(binds[2], Bind::I32(1)));
        assert!(matches!(binds[3], Bind::I32(1)));
    }

    #[test]
    fn salesman_product_type_branches() {
        let cases = [
            (1, "(sp.is_join = 0 OR sp.is_join IS NULL)"),
            (2, "sp.commission_type = 2"),
            (3, "sp.is_join = 1"),
            (4, "sp.commission_type = 3"),
        ];
        for (code, expected) in cases {
            let (sql, _) = build_from_where(&q(serde_json::json!({"salesmanProductType": code})), 0);
            assert!(sql.contains(expected), "{code}: {sql}");
        }
    }

    #[test]
    fn column_list_matches_entity() {
        assert_eq!(SALESMAN_PRODUCT_COLUMNS.len(), 8);
        assert_eq!(SALESMAN_PRODUCT_COLUMNS[0], "salesman_product_id");
        let src = include_str!("salesman_product.rs");
        assert!(src.contains("FROM salesman_order WHERE product_id = "));
        assert!(src.contains("SELECT * FROM salesman_config WHERE code = $1 LIMIT 1"));
    }
}
