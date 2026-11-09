//! 商品分类域（catalog::category）仓储。
//!
//! 涉及表：`category` / `product` / `product_sku` / `product_article` /
//! `article` / `brand`。
//!
//! 列名严格对齐 `java/database/pure.sql`
//! （`python tools/schema_parser.py --table <表名>` 可复核）：
//! - `category`：`category_id` / `category_name` / `seo_identification` /
//!   `keywords` / `category_desc` / `parent_id` / `sort_order` /
//!   `measure_unit` / `is_show` / `seo_title` / `short_name` /
//!   `category_pic` / `category_ico` / `is_hot` / `search_keywords`
//! - `product`：`product_id` / `product_name` / `product_sn` /
//!   `market_price` / `product_price` / `pic_thumb` / `pic_url` /
//!   `shop_id` / `category_id` / `brand_id` / `sort_order` /
//!   `is_delete` / `product_status` / `is_new` / `is_best` / `is_hot`
//! - `product_sku`：`sku_id` / `product_id` / `sku_price` / `is_enabled`
//! - `product_article`：主键 `(goods_id, article_id)`，**关联列是 `goods_id`**
//!   （不是 `product_id`）
//! - `article`：`article_id` / `article_title` / `article_category_id` /
//!   `article_thumb` / `article_author` / `description` / `is_show` /
//!   `is_hot` / `is_top` / `click_count` / `add_time`
//! - `brand`：`brand_id` / `brand_name` / `brand_logo` / `site_url` /
//!   `first_word` / `sort_order` / `is_show`
//!
//! 查询统一使用运行时 `sqlx::query_as`（**不使用** `query!` 宏）。

use rust_decimal::Decimal;
use sqlx::{FromRow,PgPool};
use nextrade_core::entities::{Article, Category, Product};
use nextrade_core::models::catalog::category::{RelateArticleVO, RelateBrandVO};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// 商品行（`getLookAlso` / `getProductRankData` 只取这几列）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct ProductRow {
    pub product_id: i32,
    pub product_name: String,
    pub product_sn: String,
    pub market_price: Decimal,
    pub pic_thumb: String,
    pub pic_url: String,
    pub shop_id: i32,
    /// `product.product_price`（零售价，Java 用于批发折扣模式兜底）
    pub product_price: Decimal,
    pub category_id: i32,
    pub brand_id: i32,
    pub sort_order: i32,
}

/// 商品 + 首个启用 SKU 的价格（用于填充 `ProductLookAlsoVO.price`）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct ProductSkuPriceRow {
    pub product_id: i32,
    /// 首个启用 SKU 的 `sku_price`（无 SKU 时为 NULL）
    pub sku_price: Option<Decimal>,
}

/// 相关品牌行（对应 Java `getRelateBrand` 的 `select` 列表）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct RelatedBrandRow {
    pub brand_id: i32,
    pub brand_name: String,
    pub brand_logo: Option<String>,
    pub site_url: String,
    pub first_word: Option<String>,
}

/// 分类树/同级列表使用的紧凑分类行。
#[derive(Debug, Clone, Default, FromRow)]
pub struct CategoryRow {
    pub category_id: i32,
    pub category_name: String,
    pub parent_id: i32,
    pub sort_order: i32,
    pub is_show: i32,
    pub category_pic: String,
    pub category_ico: String,
}



#[derive(Clone)]
pub struct CategoryRepository {
    pool: PgPool,
}

impl CategoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ————————————————————— category —————————————————————

    /// 全部分类（构建可见树快照用）。
    ///
    /// 对应 `CategoryServiceImpl.loadVisibleTreeSnapshot` 的
    /// `lambdaQuery().eq(Category::getIsShow, Constants.YES).list()`，
    /// `visible_only = false` 时不过滤 `is_show`。
    pub async fn find_categories(&self, visible_only: bool) -> Result<Vec<CategoryRow>> {
        let sql = if visible_only {
            "SELECT category_id, category_name, parent_id, sort_order, is_show, category_pic, \
             category_ico FROM category WHERE is_show = 1 \
             ORDER BY sort_order ASC, category_id ASC"
        } else {
            "SELECT category_id, category_name, parent_id, sort_order, is_show, category_pic, \
             category_ico FROM category ORDER BY sort_order ASC, category_id ASC"
        };
        sqlx::query_as::<_, CategoryRow>(sql)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按主键查询分类（`getParentName` / `detail` 用）。
    pub async fn find_category(&self, category_id: i32) -> Result<Option<Category>> {
        sqlx::query_as::<_, Category>("SELECT * FROM category WHERE category_id = $1 LIMIT 1")
            .bind(category_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 分类分页。
    ///
    /// 对应 `CategoryServiceImpl.list` + `BaseServiceImpl.buildSortOrder`：
    /// - `parent_id >= 0` 时按父级过滤（`None` / 负数不过滤）
    /// - `sort_field` 经白名单解析（未知字段回退 `category_id`）
    /// - 排序方向缺省为 `DESC`
    ///
    /// `column` / `dir` 只允许来自白名单常量与 `ASC|DESC` 字面量，无注入面。
    pub async fn paginate_categories(
        &self,
        parent_id: Option<i32>,
        column: &str,
        desc: bool,
        page: &PageQuery,
    ) -> Result<(u64, Vec<Category>)> {
        let filter = parent_id.filter(|p| *p >= 0);
        let (limit, offset) = page.limit_offset();

        let total: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM category WHERE ($1 IS NULL OR parent_id = $2)")
                .bind(filter)
                .bind(filter)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;

        let dir = if desc { "DESC" } else { "ASC" };
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM category WHERE (? IS NULL OR parent_id = ?) \
             ORDER BY {column} {dir} LIMIT ? OFFSET ?"
        ));
        let rows = sqlx::query_as::<_, Category>(&sql)
            .bind(filter)
            .bind(filter)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 批量统计各分类的**直接**子分类数量（`list` 消除 N+1）。
    ///
    /// 对应 `CategoryServiceImpl.countChildrenByParentIds`。
    pub async fn count_children_by_parent_ids(
        &self,
        parent_ids: &[i32],
    ) -> Result<Vec<(i32, i64)>> {
        if parent_ids.is_empty() {
            return Ok(Vec::new());
        }
        // i32 无法被注入利用；SQLx 不支持动态 IN 绑定，故内联占位符。
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

    /// 是否存在直接子分类（Controller `del` 守卫，`LIMIT 1`）。
    pub async fn has_child(&self, parent_id: i32) -> Result<bool> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT category_id FROM category WHERE parent_id = $1 LIMIT 1")
                .bind(parent_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.is_some())
    }

    /// 是否存在**不在本次删除集合内**的子分类。
    ///
    /// 对应 `batchDel` 的 `.eq(parentId, id).notIn(categoryId, ids).last("limit 1")`。
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

    /// 统计挂在该分类下的商品数量（删除守卫）。
    pub async fn count_products_in_category(&self, category_id: i32) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM product WHERE category_id = $1")
                .bind(category_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0)
    }

    /// 校验同一父级下分类名唯一（更新时排除自身）。
    ///
    /// 对应 `CategoryServiceImpl.checkCategoryNameUnique`：
    /// `parentId` 为空按顶级 `0` 处理。
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
    ///
    /// 逐列对齐 \"category\" 表（主键自增，不写入）。
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
    pub async fn update_category(&self, c: &Category) -> Result<bool> {
        let r = sqlx::query(
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
        Ok(r.rows_affected() > 0)
    }

    /// 删除单个分类。
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

    /// 按 ID 批量加载分类（紧凑行，用于 `parentTree` 的 `catList`）。
    pub async fn find_category_rows_by_ids(&self, ids: &[i32]) -> Result<Vec<CategoryRow>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT category_id, category_name, parent_id, sort_order, is_show, category_pic, \
             category_ico FROM category WHERE category_id IN ({placeholders})"
        ));
        let mut q = sqlx::query_as::<_, CategoryRow>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 可见分类中，按 ID 集合过滤后按 `sort_order` 升序取前 `limit` 条。
    /// 对应 `getRelateCategory` 的
    /// `.in(categoryId, ids).eq(isShow, 1).last("limit n").orderByAsc(sortOrder)`。
    pub async fn find_visible_categories_in(
        &self,
        ids: &[i32],
        limit: u32,
    ) -> Result<Vec<Category>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM category WHERE category_id IN ({placeholders}) AND is_show = 1 \
             ORDER BY sort_order ASC LIMIT ?"
        ));
        let mut q = sqlx::query_as::<_, Category>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 热门分类（`getHotCategory`：`is_hot = 1`，按 `sort_order` 降序，`limit 20`）。
    pub async fn find_hot_categories(&self, limit: u32) -> Result<Vec<Category>> {
        sqlx::query_as::<_, Category>(
            "SELECT * FROM category WHERE is_hot = 1 ORDER BY sort_order DESC LIMIT $1",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 通用字段更新（列名来自服务层白名单校验）。
    ///
    /// \"field\" 先在服务层对照 \"CATEGORY_FIELDS\" 校验，此处仅做兜底断言。
    pub async fn update_category_field(
        &self,
        category_id: i32,
        field: &str,
        val: &str,
    ) -> Result<bool> {
        if !nextrade_core::models::catalog::category::CATEGORY_FIELDS.contains(&field) {
            return Err(Error::Validation(format!("不允许更新的字段: {field}")));
        }
        let sql = crate::db::to_pg_placeholders(&format!("UPDATE category SET \"{field}\" = ? WHERE category_id = ?"));
        let r = sqlx::query(&sql)
            .bind(val)
            .bind(category_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    // ————————————————————— product —————————————————————

    /// 把某分类下的商品迁移到目标分类（`moveCat`）。
    ///
    /// 对应 `productMapper.update(new Product(), wrapper.eq(categoryId, id).set(categoryId, target))`。
    pub async fn move_products_category(&self, from: i32, to: i32) -> Result<u64> {
        let r = sqlx::query("UPDATE product SET category_id = $1 WHERE category_id = $2")
            .bind(to)
            .bind(from)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 按主键查询商品（`getRelate*` 系列的前置查询）。
    pub async fn find_product(&self, product_id: i32) -> Result<Option<Product>> {
        sqlx::query_as::<_, Product>("SELECT * FROM product WHERE product_id = $1 LIMIT 1")
            .bind(product_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 「看了还看」候选商品。
    ///
    /// 对应 `getLookAlso` 的 `select(...)` 列表 + 过滤条件：
    /// `category_id IN (...)` / `is_delete = 0` / `product_status = 1`，
    /// 再按 `intro` 决定 `is_new` / `is_best` / `is_hot`，
    /// `ORDER BY sort_order ASC LIMIT n`。
    ///
    /// `flag_column` 只允许 `is_new` / `is_best` / `is_hot`（服务层用
    /// [`nextrade_core::models::catalog::category::IntroKind`] 映射），无注入面。
    pub async fn find_look_also_products(
        &self,
        category_ids: &[i32],
        flag_column: &str,
        limit: u32,
    ) -> Result<Vec<ProductRow>> {
        if category_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", category_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT product_id, product_name, product_sn, market_price, pic_thumb, pic_url, \
             shop_id, product_price, category_id, brand_id, sort_order FROM product \
             WHERE category_id IN ({placeholders}) AND is_delete = 0 AND product_status = 1 \
             AND {flag_column} = 1 ORDER BY sort_order ASC LIMIT ?"
        ));
        let mut q = sqlx::query_as::<_, ProductRow>(&sql);
        for id in category_ids {
            q = q.bind(id);
        }
        q.bind(limit as i64).fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 相关排行「相同价位」候选：同分类集合内、排除自身，按
    /// `ABS(market_price - $1)` 升序。
    ///
    /// 对应 `getProductRankData(type = 1)`：
    /// `select(..., ABS(market_price - {price}))` + \"market_price\" 最近优先。
    /// Java 未显式写 `ORDER BY`（依赖 `ABS(...)` 取值列顺序，语义不明确），
    /// 这里按其**可见意图**补上显式升序，并在 \"uncertain_points\" 中标注。
    pub async fn find_rank_by_price(
        &self,
        category_ids: &[i32],
        exclude_product_id: i32,
        reference_price: Decimal,
        limit: u32,
    ) -> Result<Vec<ProductRow>> {
        if category_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", category_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT product_id, product_name, product_sn, market_price, pic_thumb, pic_url, \
             shop_id, product_price, category_id, brand_id, sort_order FROM product \
             WHERE product_id <> ? AND is_delete = 0 AND product_status = 1 \
             AND category_id IN ({placeholders}) \
             ORDER BY ABS(market_price - ?) ASC LIMIT ?"
        ));
        let mut q = sqlx::query_as::<_, ProductRow>(&sql)
            .bind(exclude_product_id)
            .bind(reference_price);
        for id in category_ids {
            q = q.bind(id);
        }
        q.bind(limit as i64).fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 相关排行「相同品牌」候选。
    ///
    /// 对应 `getProductRankData(type = 2)`：`brand_id = ?` + `sort_order ASC`。
    pub async fn find_rank_by_brand(
        &self,
        brand_id: i32,
        exclude_product_id: i32,
        limit: u32,
    ) -> Result<Vec<ProductRow>> {
        sqlx::query_as::<_, ProductRow>(
            "SELECT product_id, product_name, product_sn, market_price, pic_thumb, pic_url, \
             shop_id, product_price, category_id, brand_id, sort_order FROM product \
             WHERE product_id <> $1 AND is_delete = 0 AND product_status = 1 AND brand_id = $2 \
             ORDER BY sort_order ASC LIMIT $3",
        )
        .bind(exclude_product_id)
        .bind(brand_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 相关排行「相同类别」候选。
    ///
    /// 对应 `getProductRankData(type = 3)`：`category_id = ?` + `sort_order ASC`。
    pub async fn find_rank_by_category(
        &self,
        category_id: i32,
        exclude_product_id: i32,
        limit: u32,
    ) -> Result<Vec<ProductRow>> {
        sqlx::query_as::<_, ProductRow>(
            "SELECT product_id, product_name, product_sn, market_price, pic_thumb, pic_url, \
             shop_id, product_price, category_id, brand_id, sort_order FROM product \
             WHERE product_id <> $1 AND is_delete = 0 AND product_status = 1 AND category_id = $2 \
             ORDER BY sort_order ASC LIMIT $3",
        )
        .bind(exclude_product_id)
        .bind(category_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 相关品牌：按分类集合分组取品牌 ID。
    ///
    /// 对应 \"getRelateBrand\" 的
    /// `select(..., brand_id).in(category_id, ids).eq(is_delete,0).eq(product_status,1)
    ///  .groupBy(brand_id).last("limit n")`。Java 用 `MAX(product_id)` 等聚合列
    /// 只是为了满足 `ONLY_FULL_GROUP_BY`，实际只用 `brand_id`，这里照此只取
    /// 品牌 ID（并按 `MAX(product_id)` 降序保持结果稳定）。
    pub async fn find_brand_ids_in_categories(
        &self,
        category_ids: &[i32],
        limit: u32,
    ) -> Result<Vec<i32>> {
        if category_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", category_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT brand_id FROM product \
             WHERE category_id IN ({placeholders}) AND is_delete = 0 AND product_status = 1 \
             GROUP BY brand_id ORDER BY MAX(product_id) DESC LIMIT ?"
        ));
        let mut q = sqlx::query_as::<_, (i32,)>(&sql);
        for id in category_ids {
            q = q.bind(id);
        }
        Ok(q.bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?
            .into_iter()
            .map(|(b,)| b)
            .collect())
    }

    /// 品牌列表（`is_show = 1`，按 `sort_order` 升序，取前 `limit` 条）。
    ///
    /// Java 侧把 `sort_order ASC` 追加在 `limit` 之后（MyBatis-Plus 的
    /// `last()` + `orderByAsc` 合并顺序），语义上等价于「排序后取前 N」，
    /// 这里按等价语义书写。
    pub async fn find_related_brands(
        &self,
        brand_ids: &[i32],
        limit: u32,
    ) -> Result<Vec<RelateBrandVO>> {
        if brand_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", brand_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT brand_id, brand_name, brand_logo, site_url, first_word FROM brand \
             WHERE brand_id IN ({placeholders}) AND is_show = 1 ORDER BY sort_order ASC LIMIT ?"
        ));
        let mut q = sqlx::query_as::<_, RelatedBrandRow>(&sql);
        for id in brand_ids {
            q = q.bind(id);
        }
        Ok(q.bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?
            .into_iter()
            .map(|b| RelateBrandVO {
                brand_id: b.brand_id,
                brand_name: b.brand_name,
                brand_logo: b.brand_logo,
                site_url: b.site_url,
                first_word: b.first_word,
            })
            .collect())
    }

    // ————————————————————— product_sku —————————————————————

    /// 批量取每个商品**首个启用 SKU** 的价格。
    ///
    /// 对应 `getProductRelate` 的
    /// `productSkuService.lambdaQuery().in(productId, list).eq(isEnabled, YES).list()`
    /// 之后 `skus.getFirst()`（按主键升序取第一条）。
    ///
    /// **注意**：Java 随后调用 `getProductSkuDetail(productId, skuId, ...)` 计算
    /// 最终成交价（会员折扣 / 促销 / 秒杀 / 批发 / 门店覆盖），该价格引擎未迁移，
    /// 这里只暴露 SKU 原始价格，由服务层做**保守**兜底（见 `category.rs` 说明）。
    pub async fn find_first_sku_prices(
        &self,
        product_ids: &[i32],
    ) -> Result<Vec<ProductSkuPriceRow>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", product_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        // 子查询取每个 product_id 下 sku_id 最小的启用 SKU。
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT product_id, sku_price FROM product_sku \
             WHERE is_enabled = 1 AND sku_id IN ( \
               SELECT MIN(sku_id) FROM product_sku \
               WHERE is_enabled = 1 AND product_id IN ({placeholders}) GROUP BY product_id \
             )"
        ));
        let mut q = sqlx::query_as::<_, ProductSkuPriceRow>(&sql);
        for id in product_ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— product_article / article —————————————————————

    /// 查询商品关联的文章 ID（`product_article.goods_id` = 商品 ID）。
    ///
    /// 对应 `productArticleService.getProductArticleByProductId(productId)`。
    pub async fn find_article_ids_by_product(&self, product_id: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> =
            sqlx::query_as("SELECT article_id FROM product_article WHERE goods_id = $1")
                .bind(product_id)
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 按 ID 集合查询**已显示**的文章，取前 `limit` 条。
    ///
    /// 对应 `getArticleList`：先取关联 `articleId`，再走
    /// `articleService.list(ArticlePageQuery{articleIds, page=1, size, isShow=1})`。
    /// Java 未指定排序，Article 域默认按 `is_top DESC, add_time DESC` 排；
    /// 这里沿用同一默认序（见 `content` 仓储的既有约定）。
    pub async fn find_visible_articles_by_ids(
        &self,
        article_ids: &[i32],
        limit: u32,
    ) -> Result<Vec<Article>> {
        if article_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", article_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM article WHERE article_id IN ({placeholders}) AND is_show = 1 \
             ORDER BY is_top DESC, add_time DESC, article_id DESC LIMIT ?"
        ));
        let mut q = sqlx::query_as::<_, Article>(&sql);
        for id in article_ids {
            q = q.bind(id);
        }
        q.bind(limit as i64).fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 文章实体 → 本切片的相关文章 VO。
    #[must_use]
    pub fn to_relate_article(a: &Article) -> RelateArticleVO {
        RelateArticleVO {
            article_id: a.article_id,
            article_title: a.article_title.clone(),
            article_category_id: a.article_category_id,
            article_thumb: a.article_thumb.clone(),
            article_author: a.article_author.clone(),
            description: a.description.clone(),
            is_show: a.is_show,
            is_hot: a.is_hot,
            is_top: a.is_top,
            click_count: a.click_count,
            add_time: a.add_time,
        }
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("分类不存在".into()),
        other => Error::Database(other.to_string()),
    }
}
