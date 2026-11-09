//! SEO 域（content::seo）仓储：`category` / `product` / `article` / `brand`。
//!
//! 列名严格对齐权威 schema `java/database/pure.sql`
//! （`python tools/schema_parser.py --table <表名>` 核对）：
//! - `category`：`category_id` / `category_name` / `seo_identification` / `is_show`
//! - `product`：`product_id` / `product_sn` / `product_name` / `seo_identification` /
//!   `last_update` / `pic_url` / `product_status` / `is_delete` / `brand_id`
//! - `article`：`article_id` / `article_title` / `add_time` / `is_show`
//! - `brand`：`brand_id` / `brand_name` / `is_show`
//!
//! 查询统一使用运行时 `sqlx::query_as`（**不使用 `query!` 宏**，本机无
//! `DATABASE_URL`），并按 Java 的 `select(...)` 投影只取必要列
//! （不 `SELECT *`，避免为站点地图接口拉取 `product` 的 81 列 / `content` 大字段）。
//!
//! **排序口径**：Java 侧这些 `selectList` 都**没有 ORDER BY**（InnoDB 按主键
//! 顺序返回）。这里显式写成 `ORDER BY <主键> ASC` 使结果确定，语义与 InnoDB
//! 的实际返回一致（与 `repo/pc.rs` 同口径）；站点地图输出顺序不影响前端。
//!
//! 本仓储只做「查」，所有写入（SEO 标识批量生成）依赖未迁移的翻译服务，
//! 见 `crates/services/src/seo.rs` 的模块说明——**不提供伪造的写入方法**。

use sqlx::PgPool;
use nextrade_core::{Error, Result};

// ——————————————————————————————————————————————
// 行投影（对应 Java `LambdaQueryWrapper#select(...)`）
// ——————————————————————————————————————————————

/// `getSitemapProducts` 的行投影（Java `select(productSn, productName,
/// seoIdentification, lastUpdate, picUrl)`）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SitemapProductRow {
    /// `product.product_sn`
    pub product_sn: String,
    /// `product.product_name`
    pub product_name: String,
    /// `product.seo_identification`（可为 NULL）
    pub seo_identification: Option<String>,
    /// `product.last_update`（秒级时间戳，Java 侧 `Long` 原样回传）
    pub last_update: i32,
    /// `product.pic_url`
    pub pic_url: String,
}

/// `getSitemapCategories` 的行投影（Java `select(categoryId, categoryName,
/// seoIdentification)`）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SitemapCategoryRow {
    /// `category.category_id`
    pub category_id: i32,
    /// `category.category_name`
    pub category_name: String,
    /// `category.seo_identification`（可为 NULL）
    pub seo_identification: Option<String>,
}

/// `getSitemapArticles` 的行投影（Java `select(articleId, articleTitle, addTime)`）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SitemapArticleRow {
    /// `article.article_id`
    pub article_id: i32,
    /// `article.article_title`
    pub article_title: String,
    /// `article.add_time`（秒级时间戳）
    pub add_time: i32,
}

/// `getSitemapBrands` 第二步的行投影（Java `select(brandId, brandName)`）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SitemapBrandRow {
    /// `brand.brand_id`
    pub brand_id: i32,
    /// `brand.brand_name`
    pub brand_name: String,
}

// ——————————————————————————————————————————————
// 仓储
// ——————————————————————————————————————————————

/// SEO 域仓储。
#[derive(Clone)]
pub struct SeoRepository {
    pool: PgPool,
}

impl SeoRepository {
    /// 构造仓储。
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— category —————————————————————

    /// 按 `seo_identification` 查分类 ID（对应 Java
    /// `SeoServiceImpl#getCategoryIdBySeoLink`：
    /// `eq(Category::getSeoIdentification, seoIdentification).last("LIMIT 1")`）。
    ///
    /// 返回 `None` ⇔ Java 的 `selectOne(...) == null`（随后 Java 会走
    /// Redis 兜底查询，该兜底未迁移，见服务层）。
    pub async fn find_category_id_by_seo_identification(
        &self,
        seo_identification: &str,
    ) -> Result<Option<i32>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT category_id FROM category WHERE seo_identification = $1 LIMIT 1",
        )
        .bind(seo_identification)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(category_id,)| category_id))
    }

    /// 站点地图-分类（对应 Java `getSitemapCategories`：
    /// `eq(Category::getIsShow, 1)` + 投影三列）。
    pub async fn find_sitemap_categories(&self) -> Result<Vec<SitemapCategoryRow>> {
        sqlx::query_as::<_, SitemapCategoryRow>(
            "SELECT category_id, category_name, seo_identification FROM category WHERE is_show = 1 ORDER BY category_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// `seo_identification` 为 NULL 或空串且名称非空的分类（SEO 批量生成）。
    pub async fn find_categories_missing_seo(&self) -> Result<Vec<(i32, String)>> {
        sqlx::query_as(
            "SELECT category_id, category_name FROM category \
             WHERE (seo_identification IS NULL OR seo_identification = '') \
               AND category_name IS NOT NULL AND category_name <> '' \
             ORDER BY category_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 回写分类 SEO 标识。
    pub async fn update_category_seo(&self, category_id: i32, seo: &str) -> Result<u64> {
        let res = sqlx::query("UPDATE category SET seo_identification = ? WHERE category_id = ?")
            .bind(seo)
            .bind(category_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// `seo_identification` 为 NULL 或空串且名称非空的商品（SEO 批量生成）。
    pub async fn find_products_missing_seo(&self) -> Result<Vec<(i32, String)>> {
        sqlx::query_as(
            "SELECT product_id, product_name FROM product \
             WHERE (seo_identification IS NULL OR seo_identification = '') \
               AND product_name IS NOT NULL AND product_name <> '' \
             ORDER BY product_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 回写商品 SEO 标识。
    pub async fn update_product_seo(&self, product_id: i32, seo: &str) -> Result<u64> {
        let res = sqlx::query("UPDATE product SET seo_identification = ? WHERE product_id = ?")
            .bind(seo)
            .bind(product_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— product —————————————————————

    /// 站点地图-商品（对应 Java \"getSitemapProducts\"：
    /// `eq(Product::getProductStatus, 1).eq(Product::getIsDelete, 0)`
    /// + 投影五列）。
    pub async fn find_sitemap_products(&self) -> Result<Vec<SitemapProductRow>> {
        sqlx::query_as::<_, SitemapProductRow>(
            "SELECT product_sn, product_name, seo_identification, last_update, pic_url FROM product WHERE product_status = 1 AND is_delete = 0 ORDER BY product_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 有「上架且未删除」商品关联的品牌 ID（对应 Java \"getSitemapBrands\" 第一步：
    /// `eq(productStatus, 1).eq(isDelete, 0).isNotNull(brandId).ne(brandId, 0)`
    /// + `select(brandId).groupBy(brandId)` + `distinct()`）。
    ///
    /// \"brand_id\" 在 schema 中为 `NOT NULL`，Java 仍显式加了 \"isNotNull\"，
    /// 此处逐字保留（若有 \"NULL\" 也不会被 `brand_id <> 0` 意外命中）。
    /// Java 侧 `selectList(...).map(...).distinct()` 与 `GROUP BY` 等价，这里
    /// 只发 `GROUP BY` 查询并直接得到去重结果。
    pub async fn find_active_brand_ids(&self) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT brand_id FROM product WHERE product_status = 1 AND is_delete = 0 AND brand_id IS NOT NULL AND brand_id <> 0 GROUP BY brand_id ORDER BY brand_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|(brand_id,)| brand_id).collect())
    }

    // ————————————————————— article —————————————————————

    /// 站点地图-文章（对应 Java `getSitemapArticles`：
    /// `eq(Article::getIsShow, 1)` + 投影三列）。
    ///
    /// `is_show` 在 schema 中可为 `NULL`，SQL 的 `is_show = 1` 与 Java
    /// MyBatis-Plus 生成的 `is_show = 1` 一致（`NULL` 行不命中）。
    pub async fn find_sitemap_articles(&self) -> Result<Vec<SitemapArticleRow>> {
        sqlx::query_as::<_, SitemapArticleRow>(
            "SELECT article_id, article_title, add_time FROM article WHERE is_show = 1 ORDER BY article_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— brand —————————————————————

    /// 按品牌 ID 批量取「显示中」品牌（对应 Java \"getSitemapBrands\" 第二步：
    /// `in(Brand::getBrandId, activeBrandIds).eq(Brand::getIsShow, 1)`
    /// + `select(brandId, brandName)`）。
    ///
    /// \"ids\" 为空时 Java 不会下发 SQL（服务层已提前返回空表），这里同样返回空表。
    pub async fn find_shown_brands_by_ids(&self, ids: &[i32]) -> Result<Vec<SitemapBrandRow>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT brand_id, brand_name FROM brand WHERE brand_id IN ({placeholders}) AND is_show = 1 ORDER BY brand_id ASC"
        );
        let mut q = sqlx::query_as::<_, SitemapBrandRow>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }
}

/// SQL 错误 → `nextrade_core::Error`。
fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}"