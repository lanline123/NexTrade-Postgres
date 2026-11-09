//! 库存日历仓储：`product_reservation_stock` / `product_reservation_time`。
//!
//! 对应原 Java（overseas）：
//! - `com.nextrade.mapper.o2o.ProductReservationStockMapper` /
//!   `ProductReservationTimeMapper`（BaseMapper + `StockCalendarUtil` 日期编排）
//! - `StockCalendarListVO`（分页按「门店×商品×SKU」分组 + 日期区间汇总）
//!
//! 列名对齐 `crates/core/src/entities/o2o.rs` 对应实体。

use sqlx::{PgPool, QueryBuilder};

use nextrade_core::entities::o2o::{ProductReservationStock, ProductReservationTime};
use nextrade_core::{Error, Result};

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct StockCalendarRepository {
    pool: PgPool,
}

/// 库存分组键（门店×商品×SKU）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StockGroupKey {
    pub shop_id: i32,
    pub product_id: i32,
    pub sku_id: i32,
}

impl StockCalendarRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ————————————————————— 时间段 —————————————————————

    /// 商品的时间段配置。
    pub async fn find_times(&self, product_id: i32) -> Result<Vec<ProductReservationTime>> {
        sqlx::query_as::<_, ProductReservationTime>(
            "SELECT product_reservation_time_id, product_id, start_time, end_time, add_time FROM product_reservation_time WHERE product_id = $1 ORDER BY product_reservation_time_id ASC",
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— 分组（pageList） —————————————————————

    /// 分组分页：日期区间内出现过库存的「门店×商品×SKU」组合，
    /// 可按商品名称 / 规格值 / 门店集合过滤。
    ///
    /// 返回 `(总数, 当前页分组)`；附带商品名称（排序稳定用）。
    #[allow(clippy::too_many_lines)]
    pub async fn paginate_groups(
        &self,
        start_date: &str,
        end_date: &str,
        product_name: Option<&str>,
        spec_value: Option<&str>,
        shop_ids: &[i32],
        shop_scope: Option<i32>,
        limit: i64,
        offset: i64,
    ) -> Result<(u64, Vec<(StockGroupKey, String)>)> {
        // —— 过滤片段（count 与取行共用）——
        let push_common = |qb: &mut QueryBuilder<'_, sqlx::Postgres>| -> Result<()> {
            qb.push(" WHERE s.stock_date BETWEEN ");
            qb.push_bind(start_date.to_string());
            qb.push(" AND ");
            qb.push_bind(end_date.to_string());
            if let Some(scope) = shop_scope {
                qb.push(" AND s.shop_id = ");
                qb.push_bind(scope);
            } else if !shop_ids.is_empty() {
                qb.push(" AND s.shop_id IN (");
                let mut sep = qb.separated(", ");
                for id in shop_ids {
                    sep.push_bind(*id);
                }
                drop(sep);
                qb.push(")");
            }
            if let Some(name) = product_name.filter(|s| !s.trim().is_empty()) {
                qb.push(" AND EXISTS (SELECT 1 FROM product p WHERE p.product_id = s.product_id AND p.product_name LIKE ");
                qb.push_bind(format!("%{}%", name.trim()));
                qb.push(")");
            }
            if let Some(spec) = spec_value.filter(|s| !s.trim().is_empty()) {
                qb.push(" AND EXISTS (SELECT 1 FROM product_sku k WHERE k.sku_id = s.sku_id AND k.sku_value LIKE ");
                qb.push_bind(format!("%{}%", spec.trim()));
                qb.push(")");
            }
            Ok(())
        };

        let mut count_qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
            "SELECT COUNT(*) FROM (SELECT DISTINCT s.shop_id, s.product_id, s.sku_id FROM product_reservation_stock s",
        );
        push_common(&mut count_qb)?;
        count_qb.push(") t");
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut row_qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
            "SELECT s.shop_id, s.product_id, s.sku_id, COALESCE(MAX(p.product_name), '') FROM product_reservation_stock s LEFT JOIN product p ON p.product_id = s.product_id",
        );
        push_common(&mut row_qb)?;
        row_qb.push(" GROUP BY s.shop_id, s.product_id, s.sku_id ORDER BY s.product_id ASC, s.shop_id ASC, s.sku_id ASC LIMIT ");
        row_qb.push(limit);
        row_qb.push(" OFFSET ");
        row_qb.push(offset);
        let rows: Vec<(i32, i32, i32, String)> = row_qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        let groups = rows
            .into_iter()
            .map(|(shop_id, product_id, sku_id, name)| {
                (
                    StockGroupKey {
                        shop_id,
                        product_id,
                        sku_id,
                    },
                    name,
                )
            })
            .collect();
        Ok((u64::try_from(total.0).unwrap_or(0), groups))
    }

    /// 日期区间内、给定分组的库存行。
    pub async fn find_stocks_for_groups(
        &self,
        start_date: &str,
        end_date: &str,
        groups: &[StockGroupKey],
    ) -> Result<Vec<ProductReservationStock>> {
        if groups.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
            "SELECT product_reservation_stock_id, product_id, shop_id, sku_id, product_reservation_time_id, stock_date, total_stock, used_stock, add_time FROM product_reservation_stock WHERE stock_date BETWEEN ",
        );
        qb.push_bind(start_date.to_string());
        qb.push(" AND ");
        qb.push_bind(end_date.to_string());
        qb.push(" AND (");
        for (i, g) in groups.iter().enumerate() {
            if i > 0 {
                qb.push(" OR ");
            }
            qb.push("(shop_id = ");
            qb.push_bind(g.shop_id);
            qb.push(" AND product_id = ");
            qb.push_bind(g.product_id);
            qb.push(" AND sku_id = ");
            qb.push_bind(g.sku_id);
            qb.push(")");
        }
        qb.push(")");
        let rows: Vec<ProductReservationStock> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows)
    }

    // ————————————————————— 单日库存 —————————————————————

    /// 单日（某商品某店铺某 sku）各时间段库存。
    pub async fn find_daily_stocks(
        &self,
        product_id: i32,
        shop_id: i32,
        sku_id: i32,
        stock_date: &str,
    ) -> Result<Vec<ProductReservationStock>> {
        sqlx::query_as::<_, ProductReservationStock>(
            "SELECT product_reservation_stock_id, product_id, shop_id, sku_id, product_reservation_time_id, stock_date, total_stock, used_stock, add_time FROM product_reservation_stock WHERE product_id = $1 AND shop_id = $2 AND sku_id = $3 AND stock_date = $4 ORDER BY product_reservation_time_id ASC",
        )
        .bind(product_id)
        .bind(shop_id)
        .bind(sku_id)
        .bind(stock_date)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 单条库存行（按唯一键定位）。
    pub async fn find_stock_row(
        &self,
        product_id: i32,
        shop_id: i32,
        sku_id: i32,
        stock_date: &str,
        time_id: i32,
    ) -> Result<Option<ProductReservationStock>> {
        sqlx::query_as::<_, ProductReservationStock>(
            "SELECT product_reservation_stock_id, product_id, shop_id, sku_id, product_reservation_time_id, stock_date, total_stock, used_stock, add_time FROM product_reservation_stock WHERE product_id = $1 AND shop_id = $2 AND sku_id = $3 AND stock_date = $4 AND product_reservation_time_id = $5 LIMIT 1",
        )
        .bind(product_id)
        .bind(shop_id)
        .bind(sku_id)
        .bind(stock_date)
        .bind(time_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 插入或覆盖一行库存（唯一键
    /// `(shop_id, product_id, sku_id, stock_date, product_reservation_time_id)`）。
    pub async fn upsert_stock(
        &self,
        product_id: i32,
        shop_id: i32,
        sku_id: i32,
        stock_date: &str,
        time_id: i32,
        total_stock: i32,
        used_stock: i32,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO product_reservation_stock (product_id, shop_id, sku_id, product_reservation_time_id, stock_date, total_stock, used_stock, add_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT (shop_id, product_id, sku_id, stock_date, product_reservation_time_id) DO UPDATE SET total_stock = EXCLUDED.total_stock, used_stock = EXCLUDED.used_stock",
        )
        .bind(product_id)
        .bind(shop_id)
        .bind(sku_id)
        .bind(time_id)
        .bind(stock_date)
        .bind(total_stock)
        .bind(used_stock)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    // ————————————————————— 附属信息 —————————————————————

    /// 商品基础信息（列表 / 时间段接口用）。
    pub async fn find_product_infos(
        &self,
        product_ids: &[i32],
    ) -> Result<Vec<(i32, String, String, String, String)>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
            "SELECT product_id, product_name, pic_url, pic_thumb, pic_original FROM product WHERE product_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in product_ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        let rows: Vec<(i32, String, String, String, String)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows)
    }

    /// 商品某 SKU 的规格值（`product_sku.sku_value`）。
    pub async fn find_sku_value(&self, sku_id: i32) -> Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT sku_value FROM product_sku WHERE sku_id = $1 LIMIT 1")
                .bind(sku_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|r| r.0))
    }

    /// 店铺名称（列表门店信息列）。
    pub async fn find_shop_names(&self, shop_ids: &[i32]) -> Result<Vec<(i32, String)>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT shop_id, shop_title FROM shop WHERE shop_id IN (");
        let mut sep = qb.separated(", ");
        for id in shop_ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        let rows: Vec<(i32, Option<String>)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(id, name)| (id, name.unwrap_or_default()))
            .collect())
    }
}

/// SQLx 错误 → 统一错误类型。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("预约库存数据不存在".into()),
        other => Error::Database(other.to_string()),
    }
}
