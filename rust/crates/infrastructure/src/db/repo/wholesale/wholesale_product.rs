//! 批发商品域（wholesale::wholesale_product）仓储。
//!
//! 覆盖表（列名以 `python tools/schema_parser.py --table <表名>` 核对）：
//! - `product_wholesale_config`（10 列，主键 `id`）
//! - `product_wholesale_price`（13 列，主键 `id`）
//! - `product_wholesale_sku_config`（8 列，主键 `id`）
//! - `product` / `product_sku`（只读；实体列多，整行查询用 `SELECT *` 由
//!   `FromRow` 映射，与既有 `wholesale_client` 仓储同口径）
//! - `wholesaler_grade` / `category` / `config`（只读辅助）
//!
//! 写操作口径（逐条对齐 Java）：
//! - `product_wholesale_config` / `product_wholesale_sku_config` /
//!   `product_wholesale_price` 的删除是**物理删**（CD-033：唯一索引不含
//!   `is_delete`，软删会撞 1062）；仅 `product_wholesale_sku_config` 在
//!   Java `deleteByProductIds` 里是软删 —— 本实现按 Java 现状复刻：
//!   sku_config 软删（`is_delete=1`），config / price 物理删。
//! - `product_wholesale_price` 保存 = 物理删该商品全部旧行 + 批量插入。
//!
//! 查询统一使用运行时 `sqlx::query_as`（**不使用** `query!` 宏）。

use sqlx::PgPool;

use nextrade_core::entities::{
    Product, ProductSku, ProductWholesaleConfig, ProductWholesalePrice, ProductWholesaleSkuConfig,
};
use nextrade_core::models::wholesale::wholesale_product::MSG_PRODUCT_NOT_FOUND;
use nextrade_core::{Error, Result};

/// `product_wholesale_config` 的固定 SELECT 列清单（10 列）。
const CONFIG_SELECT: &str = "SELECT id, product_id, shop_id, multi_spec_price_mode, moq_dimension, moq, status, is_delete, create_time, update_time FROM product_wholesale_config";

/// `product_wholesale_price` 的固定 SELECT 列清单（13 列）。
const PRICE_SELECT: &str = "SELECT id, product_id, shop_id, sku_id, grade_id, tier_min_qty, price_mode, discount, wholesale_price, status, is_delete, create_time, update_time FROM product_wholesale_price";

/// `product_wholesale_sku_config` 的固定 SELECT 列清单（8 列）。
const SKU_CONFIG_SELECT: &str = "SELECT id, product_id, shop_id, sku_id, multiple, is_delete, create_time, update_time FROM product_wholesale_sku_config";

/// 批发商品仓储（持有连接池）。
#[derive(Clone)]
pub struct WholesaleProductRepository {
    pool: PgPool,
}

impl WholesaleProductRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— product_wholesale_config —————————————————————

    /// 生效中的商品级批发配置（Java `getActiveByProductId`：
    /// `product_id = ? AND status = 1 AND is_delete = 0 LIMIT 1`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_active_config(
        &self,
        product_id: i32,
    ) -> Result<Option<ProductWholesaleConfig>> {
        let sql = format!(
            "{CONFIG_SELECT} WHERE product_id = $1 AND status = 1 AND is_delete = 0 LIMIT 1"
        );
        sqlx::query_as::<_, ProductWholesaleConfig>(&sql)
            .bind(product_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 商品的任意未删配置（不限 status；Java `requireConfig` 口径）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_config(&self, product_id: i32) -> Result<Option<ProductWholesaleConfig>> {
        let sql = format!("{CONFIG_SELECT} WHERE product_id = $1 AND is_delete = 0 LIMIT 1");
        sqlx::query_as::<_, ProductWholesaleConfig>(&sql)
            .bind(product_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 批量取生效配置（Java `mapByProductIds`：`IN + status = 1`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_active_configs(&self, product_ids: &[i32]) -> Result<Vec<ProductWholesaleConfig>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; product_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "{CONFIG_SELECT} WHERE status = 1 AND is_delete = 0 AND product_id IN ({placeholders})"
        ));
        let mut q = sqlx::query_as::<_, ProductWholesaleConfig>(&sql);
        for id in product_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 商品是否已有**任意配置行（含软删）**（Java `existsAnyByProductId`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn exists_any_config(&self, product_id: i32) -> Result<bool> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT id FROM product_wholesale_config WHERE product_id = $1 LIMIT 1")
                .bind(product_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.is_some())
    }

    /// 全部已配置商品 ID（含软删；Java `selectAllConfiguredProductIds`，
    /// 可选列表剔除用）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn all_configured_product_ids(&self) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> =
            sqlx::query_as("SELECT DISTINCT product_id FROM product_wholesale_config")
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 批发商品分页（以 config 为分页主表；Java `WholesaleProductQueryServiceImpl#page`）。
    ///
    /// 过滤：`shop_id`、`moq_dimension`、`keyword`（先解析候选 productId）；
    /// `is_delete = 0`；`create_time DESC`。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn paginate_configs(
        &self,
        shop_id: i32,
        moq_dimension: Option<i32>,
        keyword_product_ids: Option<&[i32]>,
        limit: i64,
        offset: i64,
    ) -> Result<(u64, Vec<ProductWholesaleConfig>)> {
        let mut where_parts: Vec<String> = vec!["is_delete = 0".into(), "shop_id = ?".into()];
        if moq_dimension.is_some() {
            where_parts.push("moq_dimension = ?".into());
        }
        if let Some(ids) = keyword_product_ids {
            if ids.is_empty() {
                return Ok((0, Vec::new()));
            }
            where_parts.push(format!(
                "product_id IN ({})",
                vec!["?"; ids.len()].join(", ")
            ));
        }
        let where_sql = format!("WHERE {}", where_parts.join(" AND "));

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM product_wholesale_config {where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql).bind(shop_id);
        if let Some(v) = moq_dimension {
            cq = cq.bind(v);
        }
        if let Some(ids) = keyword_product_ids {
            for id in ids {
                cq = cq.bind(*id);
            }
        }
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let list_sql = crate::db::to_pg_placeholders(&format!(
            "{CONFIG_SELECT} {where_sql} ORDER BY create_time DESC LIMIT ? OFFSET ?"
        ));
        let mut lq = sqlx::query_as::<_, ProductWholesaleConfig>(&list_sql).bind(shop_id);
        if let Some(v) = moq_dimension {
            lq = lq.bind(v);
        }
        if let Some(ids) = keyword_product_ids {
            for id in ids {
                lq = lq.bind(*id);
            }
        }
        let rows = lq
            .bind(limit.max(1) as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 批量按 product_id 取未删配置（Java `selectByProductIds`；
    /// `batchUpdateMoq` 用，包含 status=0 行）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_configs_by_product_ids(&self, product_ids: &[i32]) -> Result<Vec<ProductWholesaleConfig>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; product_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("{CONFIG_SELECT} WHERE is_delete = 0 AND product_id IN ({placeholders})"));
        let mut q = sqlx::query_as::<_, ProductWholesaleConfig>(&sql);
        for id in product_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 新增商品级配置，返回自增主键。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn insert_config(&self, c: &ProductWholesaleConfig) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO product_wholesale_config (product_id, shop_id, multi_spec_price_mode, moq_dimension, moq, status, is_delete, create_time, update_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
        )
        .bind(c.product_id)
        .bind(c.shop_id)
        .bind(c.multi_spec_price_mode)
        .bind(c.moq_dimension)
        .bind(c.moq)
        .bind(c.status)
        .bind(c.is_delete)
        .bind(c.create_time)
        .bind(c.update_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 整行更新商品级配置（Java `updateById`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn update_config(&self, c: &ProductWholesaleConfig) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE product_wholesale_config SET multi_spec_price_mode = $1, moq_dimension = $2, moq = $3, status = $4, update_time = $5 WHERE id = $6",
        )
        .bind(c.multi_spec_price_mode)
        .bind(c.moq_dimension)
        .bind(c.moq)
        .bind(c.status)
        .bind(c.update_time)
        .bind(c.id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 物理删商品级配置（Java `hardDeleteByProductIds`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn hard_delete_configs(&self, product_ids: &[i32]) -> Result<u64> {
        if product_ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; product_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("DELETE FROM product_wholesale_config WHERE product_id IN ({placeholders})"));
        let mut q = sqlx::query(&sql);
        for id in product_ids {
            q = q.bind(*id);
        }
        let res = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— product_wholesale_sku_config —————————————————————

    /// 商品的 SKU 级配置（Java `listByProductId`：`is_delete = 0`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn list_sku_configs(&self, product_id: i32) -> Result<Vec<ProductWholesaleSkuConfig>> {
        let sql = format!("{SKU_CONFIG_SELECT} WHERE product_id = $1 AND is_delete = 0");
        sqlx::query_as::<_, ProductWholesaleSkuConfig>(&sql)
            .bind(product_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 批量 SKU 级配置（Java `mapByProductSkuKeys`：`IN` + `is_delete = 0`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn list_sku_configs_by_products(&self, product_ids: &[i32]) -> Result<Vec<ProductWholesaleSkuConfig>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; product_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "{SKU_CONFIG_SELECT} WHERE is_delete = 0 AND product_id IN ({placeholders})"
        ));
        let mut q = sqlx::query_as::<_, ProductWholesaleSkuConfig>(&sql);
        for id in product_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 单个 (product, sku) 的 SKU 配置（Java `upsert` 的 `selectOne`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_sku_config(
        &self,
        product_id: i32,
        sku_id: i32,
    ) -> Result<Option<ProductWholesaleSkuConfig>> {
        let sql = format!(
            "{SKU_CONFIG_SELECT} WHERE product_id = $1 AND sku_id = $2 AND is_delete = 0 LIMIT 1"
        );
        sqlx::query_as::<_, ProductWholesaleSkuConfig>(&sql)
            .bind(product_id)
            .bind(sku_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 新增 SKU 配置，返回自增主键。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn insert_sku_config(&self, c: &ProductWholesaleSkuConfig) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO product_wholesale_sku_config (product_id, shop_id, sku_id, multiple, is_delete, create_time, update_time) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
        )
        .bind(c.product_id)
        .bind(c.shop_id)
        .bind(c.sku_id)
        .bind(c.multiple)
        .bind(c.is_delete)
        .bind(c.create_time)
        .bind(c.update_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 更新 SKU 倍数（Java `upsert` 的 update 分支）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn update_sku_multiple(&self, id: i32, multiple: i32, now: i64) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE product_wholesale_sku_config SET multiple = $1, update_time = $2 WHERE id = $3 AND is_delete = 0",
        )
        .bind(multiple)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 软删商品下全部 SKU 配置（Java `deleteByProductIds`：`is_delete=1`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn soft_delete_sku_configs(&self, product_ids: &[i32], now: i64) -> Result<u64> {
        if product_ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; product_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "UPDATE product_wholesale_sku_config SET is_delete = 1, update_time = $1 WHERE is_delete = 0 AND product_id IN ({placeholders})"
        ));
        let mut q = sqlx::query(&sql).bind(now);
        for id in product_ids {
            q = q.bind(*id);
        }
        let res = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— product_wholesale_price —————————————————————

    /// 商品全部有效批发价行（Java `listByProductId`，Mapper XML：
    /// `product_id = ? AND is_delete = 0`，按 `id ASC` 隐含 admin 录入顺序）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn list_prices(&self, product_id: i32) -> Result<Vec<ProductWholesalePrice>> {
        let sql = format!("{PRICE_SELECT} WHERE product_id = $1 AND is_delete = 0 ORDER BY id ASC");
        sqlx::query_as::<_, ProductWholesalePrice>(&sql)
            .bind(product_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 物理删该商品全部批发价行（Java `deleteByProductId`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn delete_prices(&self, product_id: i32) -> Result<u64> {
        let res = sqlx::query("DELETE FROM product_wholesale_price WHERE product_id = $1")
            .bind(product_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 批量物理删多个商品的批发价行（Java `deleteByProductIds`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn delete_prices_by_products(&self, product_ids: &[i32]) -> Result<u64> {
        if product_ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; product_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("DELETE FROM product_wholesale_price WHERE product_id IN ({placeholders})"));
        let mut q = sqlx::query(&sql);
        for id in product_ids {
            q = q.bind(*id);
        }
        let res = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 插入一条批发价行。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn insert_price(&self, p: &ProductWholesalePrice) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO product_wholesale_price (product_id, shop_id, sku_id, grade_id, tier_min_qty, price_mode, discount, wholesale_price, status, is_delete, create_time, update_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING id",
        )
        .bind(p.product_id)
        .bind(p.shop_id)
        .bind(p.sku_id)
        .bind(p.grade_id)
        .bind(p.tier_min_qty)
        .bind(p.price_mode)
        .bind(p.discount)
        .bind(p.wholesale_price)
        .bind(p.status)
        .bind(p.is_delete)
        .bind(p.create_time)
        .bind(p.update_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    // ————————————————————— product / product_sku / category —————————————————————

    /// 按 ID 查询商品（Java `getById`，不过滤软删）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_product(&self, product_id: i32) -> Result<Option<Product>> {
        sqlx::query_as::<_, Product>("SELECT * FROM product WHERE product_id = $1 LIMIT 1")
            .bind(product_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按关键词（商品名 / 货号）解析候选 productId（Java
    /// `matchProductIdsByKeyword`：`is_delete = 0`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn product_ids_by_keyword(&self, keyword: &str) -> Result<Vec<i32>> {
        let like = format!("%{keyword}%");
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT product_id FROM product WHERE is_delete = 0 AND (product_name LIKE $1 OR product_sn LIKE $2)",
        )
        .bind(&like)
        .bind(&like)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 可选批发商品分页（Java `selectablePage`）。
    ///
    /// 过滤：`is_delete=0`、`shop_id`、`product_status`（缺省 1）、普通实物
    /// （`product_type IS NULL OR =1`）、非预售（`sale_mode IS NULL OR =0`）、
    /// 单次交付（`delivery_mode IS NULL OR =1`）、分类子树 / 品牌 / 关键词、
    /// 剔除已配置批发商品；`product_id DESC`。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    #[allow(clippy::too_many_arguments)]
    pub async fn paginate_selectable_products(
        &self,
        shop_id: i32,
        product_status: i32,
        keyword: Option<&str>,
        category_ids: Option<&[i32]>,
        brand_id: Option<i32>,
        excluded_product_ids: &[i32],
        limit: i64,
        offset: i64,
    ) -> Result<(u64, Vec<Product>)> {
        let like = keyword
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|k| format!("%{k}%"));
        let mut where_parts: Vec<String> = vec![
            "is_delete = 0".into(),
            "shop_id = ?".into(),
            "product_status = ?".into(),
            "(product_type IS NULL OR product_type = 1)".into(),
            "(sale_mode IS NULL OR sale_mode = 0)".into(),
            "(delivery_mode IS NULL OR delivery_mode = 1)".into(),
        ];
        if let Some(ids) = category_ids {
            if ids.is_empty() {
                return Ok((0, Vec::new()));
            }
            where_parts.push(format!("category_id IN ({})", vec!["?"; ids.len()].join(", ")));
        }
        if brand_id.is_some() {
            where_parts.push("brand_id = ?".into());
        }
        if like.is_some() {
            where_parts.push("(product_name LIKE ? OR product_sn LIKE ? OR product_tsn LIKE ?)".into());
        }
        if !excluded_product_ids.is_empty() {
            where_parts.push(format!(
                "product_id NOT IN ({})",
                vec!["?"; excluded_product_ids.len()].join(", ")
            ));
        }
        let where_sql = format!("WHERE {}", where_parts.join(" AND "));

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM product {where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql).bind(shop_id).bind(product_status);
        if let Some(ids) = category_ids {
            for id in ids {
                cq = cq.bind(*id);
            }
        }
        if let Some(v) = brand_id {
            cq = cq.bind(v);
        }
        if let Some(like) = like.as_deref() {
            cq = cq.bind(like).bind(like).bind(like);
        }
        for id in excluded_product_ids {
            cq = cq.bind(*id);
        }
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let list_sql =
            crate::db::to_pg_placeholders(&format!("SELECT * FROM product {where_sql} ORDER BY product_id DESC LIMIT ? OFFSET ?"));
        let mut lq = sqlx::query_as::<_, Product>(&list_sql).bind(shop_id).bind(product_status);
        if let Some(ids) = category_ids {
            for id in ids {
                lq = lq.bind(*id);
            }
        }
        if let Some(v) = brand_id {
            lq = lq.bind(v);
        }
        if let Some(like) = like.as_deref() {
            lq = lq.bind(like).bind(like).bind(like);
        }
        for id in excluded_product_ids {
            lq = lq.bind(*id);
        }
        let rows = lq
            .bind(limit.max(1) as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 分类子树 ID（含自身；Java `CategoryService#getCategoryAllChildIds`）。
    ///
    /// 一次性取全表 `(category_id, parent_id)` 后在内存 BFS；分类表规模小。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn category_descendant_ids(&self, category_id: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i32, i32)> =
            sqlx::query_as("SELECT category_id, parent_id FROM category")
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        let mut children: std::collections::HashMap<i32, Vec<i32>> = std::collections::HashMap::new();
        for (id, parent) in rows {
            children.entry(parent).or_default().push(id);
        }
        let mut out = vec![category_id];
        let mut idx = 0;
        while idx < out.len() {
            let current = out[idx];
            idx += 1;
            if let Some(kids) = children.get(&current) {
                for kid in kids {
                    if !out.contains(kid) {
                        out.push(*kid);
                    }
                }
            }
        }
        Ok(out)
    }

    /// 商品启用中的 SKU 列表（Java `getProductSkusByPid`：`is_enabled = 1`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_enabled_skus(&self, product_id: i32) -> Result<Vec<ProductSku>> {
        sqlx::query_as::<_, ProductSku>(
            "SELECT * FROM product_sku WHERE product_id = $1 AND is_enabled = 1 ORDER BY sku_id ASC",
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 批量商品启用中的 SKU 列表（Java `getProductSkusByProductIds`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_enabled_skus_by_products(&self, product_ids: &[i32]) -> Result<Vec<ProductSku>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; product_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM product_sku WHERE is_enabled = 1 AND product_id IN ({placeholders}) ORDER BY sku_id ASC"
        ));
        let mut q = sqlx::query_as::<_, ProductSku>(&sql);
        for id in product_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 等级 / 配置 —————————————————————

    /// 等级有效性（Java `WholesalerGradeService#requireEnabled` 只读版）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn grade_enabled(&self, grade_id: i32) -> Result<Option<bool>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT status FROM wholesaler_grade WHERE grade_id = $1 AND is_delete = 0 LIMIT 1",
        )
        .bind(grade_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(status,)| status == 1))
    }

    /// 批量等级名称（Java `gradeService.mapByIds`，`is_delete = 0`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn grade_names(&self, grade_ids: &[i32]) -> Result<Vec<(i32, String)>> {
        if grade_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; grade_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT grade_id, grade_name FROM wholesaler_grade WHERE is_delete = 0 AND grade_id IN ({placeholders})"
        ));
        let mut q = sqlx::query_as::<_, (i32, String)>(&sql);
        for id in grade_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 货币符号（Java `ConfigService#getCurrencySymbol`：`dollarSign`，
    /// 空白回退 `¥`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn currency_symbol(&self) -> Result<String> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = 'dollarSign' AND is_del = 0 LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row
            .and_then(|(v,)| v)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "¥".to_string()))
    }
}

/// 通用数据库错误映射。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound(MSG_PRODUCT_NOT_FOUND.into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 三张批发表的 SELECT 清单覆盖各自全部列。
    #[test]
    fn select_column_lists_cover_tables() {
        assert_eq!(CONFIG_SELECT.matches(',').count() + 1, 10);
        assert_eq!(PRICE_SELECT.matches(',').count() + 1, 13);
        assert_eq!(SKU_CONFIG_SELECT.matches(',').count() + 1, 8);
        assert!(!CONFIG_SELECT.contains('*'));
        assert!(!PRICE_SELECT.contains('*'));
        assert!(!SKU_CONFIG_SELECT.contains('*'));
    }

    /// 错误文案与 Java `WHOLESALE_PRODUCT_NOT_FOUND` 一致。
    #[test]
    fn not_found_message_is_verbatim() {
        assert_eq!(MSG_PRODUCT_NOT_FOUND, "商品不存在");
    }
}
