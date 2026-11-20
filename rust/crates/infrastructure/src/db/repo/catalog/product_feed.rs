//! 商品对接 API 仓储（catalog::product_feed）。
//!
//! 表（`rust/deploy/pg/2026-11-18_product_feed.sql`）：
//! - `product_feed_key`（密钥主表）
//! - `product_feed_key_product` / `product_feed_key_category`（授权范围）
//! - `product_feed_price_override`（单品价格覆盖）
//! 只读引用既有表：`product` / `product_sku` / `product_gallery` /
//! `category` / `brand`。
//!
//! SQL 一律运行时 `sqlx::query` / `query_as`，`$n` 占位。

use sqlx::{PgPool, Postgres, QueryBuilder};

use nextrade_core::entities::{Category, ProductGallery};
use nextrade_core::models::catalog::product_feed::{FeedKey, FeedPriceOverride, FeedProductRow};
use nextrade_core::{Error, Result};

/// feed 商品过滤条件（范围已由 service 按密钥口径展开完毕）。
#[derive(Debug, Clone, Default)]
pub struct FeedProductFilter {
    /// 指定商品模式：`Some(ids)` 时限定 `product_id IN ids`。
    pub product_ids: Option<Vec<i32>>,
    /// 指定分类模式（已展开后代）：`Some(ids)` 时限定 `category_id IN ids`。
    pub category_ids: Option<Vec<i32>>,
    /// 是否包含询价商品（false 时按规划 §7.3 排除）。
    pub include_inquiry: bool,
    /// 增量下界：`last_update >= updated_since`。
    pub updated_since: Option<i32>,
    /// 请求侧分类过滤（已展开后代），与范围条件取交集。
    pub filter_category_ids: Option<Vec<i32>>,
    /// 名称/货号模糊关键词。
    pub keyword: Option<String>,
    /// 店铺范围：`Some(ids)` 时限定 `shop_id IN ids`（平台联邦凭证用，
    /// feed 密钥恒为 `None` 不限店铺，行为不变）。
    pub shop_ids: Option<Vec<i32>>,
}

/// 商品对接仓储。
#[derive(Clone)]
pub struct ProductFeedRepository {
    pool: PgPool,
}

impl ProductFeedRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ———————————————————— 密钥主表 ————————————————————

    /// 按 access_key 查密钥（鉴权用）。
    pub async fn find_key_by_access_key(&self, access_key: &str) -> Result<Option<FeedKey>> {
        sqlx::query_as::<_, FeedKey>("SELECT * FROM product_feed_key WHERE access_key = $1")
            .bind(access_key)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按 id 查密钥。
    pub async fn find_key_by_id(&self, feed_key_id: i32) -> Result<Option<FeedKey>> {
        sqlx::query_as::<_, FeedKey>("SELECT * FROM product_feed_key WHERE feed_key_id = $1")
            .bind(feed_key_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 密钥分页列表（keyword 匹配名称/access_key；status 可选）。
    pub async fn list_keys(
        &self,
        keyword: Option<&str>,
        status: Option<i32>,
        limit: i64,
        offset: i64,
    ) -> Result<(u64, Vec<FeedKey>)> {
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM product_feed_key WHERE 1=1");
        push_key_filters(&mut qb, keyword, status);
        let total: (i64,) = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT * FROM product_feed_key WHERE 1=1");
        push_key_filters(&mut qb, keyword, status);
        qb.push(" ORDER BY feed_key_id DESC LIMIT ");
        qb.push_bind(limit.max(1));
        qb.push(" OFFSET ");
        qb.push_bind(offset.max(0));
        let rows = qb
            .build_query_as::<FeedKey>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 新建密钥，返回主键。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_key(
        &self,
        key_name: &str,
        access_key: &str,
        secret_hash: &str,
        scope_mode: i32,
        include_inquiry: i32,
        price_rule_type: &str,
        price_rule_value: rust_decimal::Decimal,
        expire_time: i32,
        remark: &str,
        now: i32,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO product_feed_key (key_name, access_key, secret_hash, status, scope_mode, include_inquiry, price_rule_type, price_rule_value, expire_time, remark, shop_id, last_used_time, call_count, add_time, update_time)
             VALUES ($1, $2, $3, 1, $4, $5, $6, $7, $8, $9, 0, 0, 0, $10, $10) RETURNING feed_key_id",
        )
        .bind(key_name)
        .bind(access_key)
        .bind(secret_hash)
        .bind(scope_mode)
        .bind(include_inquiry)
        .bind(price_rule_type)
        .bind(price_rule_value)
        .bind(expire_time)
        .bind(remark)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 更新密钥可编辑字段（范围清单走 replace_* 单独处理）。
    #[allow(clippy::too_many_arguments)]
    pub async fn update_key(
        &self,
        feed_key_id: i32,
        key_name: &str,
        status: i32,
        scope_mode: i32,
        include_inquiry: i32,
        price_rule_type: &str,
        price_rule_value: rust_decimal::Decimal,
        expire_time: i32,
        remark: &str,
        now: i32,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE product_feed_key SET key_name = $2, status = $3, scope_mode = $4, include_inquiry = $5, price_rule_type = $6, price_rule_value = $7, expire_time = $8, remark = $9, update_time = $10 WHERE feed_key_id = $1",
        )
        .bind(feed_key_id)
        .bind(key_name)
        .bind(status)
        .bind(scope_mode)
        .bind(include_inquiry)
        .bind(price_rule_type)
        .bind(price_rule_value)
        .bind(expire_time)
        .bind(remark)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 重置 secret 哈希。
    pub async fn update_secret_hash(
        &self,
        feed_key_id: i32,
        secret_hash: &str,
        now: i32,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE product_feed_key SET secret_hash = $2, update_time = $3 WHERE feed_key_id = $1",
        )
        .bind(feed_key_id)
        .bind(secret_hash)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 删除密钥（主表一行；范围/覆盖由 delete_key_scope 一并清）。
    pub async fn delete_key(&self, feed_key_id: i32) -> Result<u64> {
        let res = sqlx::query("DELETE FROM product_feed_key WHERE feed_key_id = $1")
            .bind(feed_key_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 鉴权成功后节流回写最近调用时间与累计次数（调用方已做 >60 秒判断）。
    pub async fn touch_usage(&self, feed_key_id: i32, now: i32) -> Result<()> {
        sqlx::query(
            "UPDATE product_feed_key SET last_used_time = $2, call_count = call_count + 1 WHERE feed_key_id = $1",
        )
        .bind(feed_key_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    // ———————————————————— 授权范围 ————————————————————

    pub async fn list_key_product_ids(&self, feed_key_id: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT product_id FROM product_feed_key_product WHERE feed_key_id = $1 ORDER BY product_id ASC",
        )
        .bind(feed_key_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    pub async fn list_key_category_ids(&self, feed_key_id: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT category_id FROM product_feed_key_category WHERE feed_key_id = $1 ORDER BY category_id ASC",
        )
        .bind(feed_key_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 整体替换指定商品清单。
    pub async fn replace_key_products(&self, feed_key_id: i32, ids: &[i32]) -> Result<()> {
        sqlx::query("DELETE FROM product_feed_key_product WHERE feed_key_id = $1")
            .bind(feed_key_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        for id in ids {
            sqlx::query(
                "INSERT INTO product_feed_key_product (feed_key_id, product_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(feed_key_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        }
        Ok(())
    }

    /// 整体替换指定分类清单。
    pub async fn replace_key_categories(&self, feed_key_id: i32, ids: &[i32]) -> Result<()> {
        sqlx::query("DELETE FROM product_feed_key_category WHERE feed_key_id = $1")
            .bind(feed_key_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        for id in ids {
            sqlx::query(
                "INSERT INTO product_feed_key_category (feed_key_id, category_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(feed_key_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        }
        Ok(())
    }

    /// 删除密钥时清范围与覆盖。
    pub async fn delete_key_scope(&self, feed_key_id: i32) -> Result<()> {
        for table in [
            "product_feed_key_product",
            "product_feed_key_category",
            "product_feed_price_override",
        ] {
            sqlx::query(&format!("DELETE FROM {table} WHERE feed_key_id = $1"))
                .bind(feed_key_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
        }
        Ok(())
    }

    // ———————————————————— 单品价格覆盖 ————————————————————

    pub async fn list_overrides(&self, feed_key_id: i32) -> Result<Vec<FeedPriceOverride>> {
        sqlx::query_as::<_, FeedPriceOverride>(
            "SELECT * FROM product_feed_price_override WHERE feed_key_id = $1 ORDER BY product_id ASC",
        )
        .bind(feed_key_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn find_override(
        &self,
        feed_key_id: i32,
        product_id: i32,
    ) -> Result<Option<FeedPriceOverride>> {
        sqlx::query_as::<_, FeedPriceOverride>(
            "SELECT * FROM product_feed_price_override WHERE feed_key_id = $1 AND product_id = $2",
        )
        .bind(feed_key_id)
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 某密钥的全部覆盖（受众解析用，运营量级下条数有限）。
    pub async fn list_all_overrides(
        &self,
        feed_key_id: i32,
    ) -> Result<Vec<FeedPriceOverride>> {
        sqlx::query_as::<_, FeedPriceOverride>(
            "SELECT * FROM product_feed_price_override WHERE feed_key_id = $1",
        )
        .bind(feed_key_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 多个商品的覆盖一次查回（feed 列表页用）：`product_id → override` 原始行。
    pub async fn list_overrides_for_products(
        &self,
        feed_key_id: i32,
        product_ids: &[i32],
    ) -> Result<Vec<FeedPriceOverride>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT * FROM product_feed_price_override WHERE feed_key_id = ",
        );
        qb.push_bind(feed_key_id);
        qb.push(" AND product_id IN (");
        let mut sep = qb.separated(", ");
        for id in product_ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.build_query_as::<FeedPriceOverride>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    pub async fn upsert_override(
        &self,
        feed_key_id: i32,
        product_id: i32,
        rule_type: &str,
        rule_value: rust_decimal::Decimal,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO product_feed_price_override (feed_key_id, product_id, rule_type, rule_value, add_time, update_time)
             VALUES ($1, $2, $3, $4, $5, $5)
             ON CONFLICT (feed_key_id, product_id) DO UPDATE SET rule_type = EXCLUDED.rule_type, rule_value = EXCLUDED.rule_value, update_time = EXCLUDED.update_time",
        )
        .bind(feed_key_id)
        .bind(product_id)
        .bind(rule_type)
        .bind(rule_value)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn delete_override(&self, feed_key_id: i32, product_id: i32) -> Result<u64> {
        let res = sqlx::query(
            "DELETE FROM product_feed_price_override WHERE feed_key_id = $1 AND product_id = $2",
        )
        .bind(feed_key_id)
        .bind(product_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ———————————————————— feed 商品查询 ————————————————————

    /// 可见商品计数（与 list_products 同过滤口径）。
    pub async fn count_products(&self, filter: &FeedProductFilter) -> Result<i64> {
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM product p WHERE ");
        push_product_conditions(&mut qb, filter);
        let row: (i64,) = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    /// 可见商品分页（`last_update DESC, product_id DESC`）。
    pub async fn list_products(
        &self,
        filter: &FeedProductFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<FeedProductRow>> {
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT p.product_id, p.product_name, p.product_sn, p.product_price, p.market_price, \
             p.product_stock, p.category_id, p.brand_id, p.shop_id, p.pic_url, p.product_brief, p.product_desc, p.last_update, p.is_inquiry, \
             (SELECT COUNT(*) FROM product_sku ps WHERE ps.product_id = p.product_id AND ps.is_enabled = 1) AS sku_count, \
             (SELECT COALESCE(MAX(ps.is_inquiry), 0) FROM product_sku ps WHERE ps.product_id = p.product_id AND ps.is_enabled = 1) AS sku_inquiry_max \
             FROM product p WHERE ",
        );
        push_product_conditions(&mut qb, filter);
        qb.push(" ORDER BY p.last_update DESC, p.product_id DESC LIMIT ");
        qb.push_bind(limit.max(1));
        qb.push(" OFFSET ");
        qb.push_bind(offset.max(0));
        qb.build_query_as::<FeedProductRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 店铺名批量查：`(shop_id, shop_title)`（联邦来源标记用）。
    pub async fn shop_names(&self, ids: &[i32]) -> Result<Vec<(i32, String)>> {
        let ids: Vec<i32> = ids.iter().copied().filter(|id| *id > 0).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT shop_id, shop_title FROM shop WHERE shop_id IN (");
        let mut sep = qb.separated(", ");
        for id in &ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.build_query_as::<(i32, String)>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 全站分类（树展开/路径/授权树共用）。
    pub async fn list_all_categories(&self) -> Result<Vec<Category>> {
        sqlx::query_as::<_, Category>(
            "SELECT * FROM category ORDER BY sort_order ASC, category_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 品牌名批量查：`(brand_id, brand_name)`。
    pub async fn brand_names(&self, ids: &[i32]) -> Result<Vec<(i32, String)>> {
        let ids: Vec<i32> = ids.iter().copied().filter(|id| *id > 0).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT brand_id, brand_name FROM brand WHERE brand_id IN (");
        let mut sep = qb.separated(", ");
        for id in &ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.build_query_as::<(i32, String)>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按分类计数（授权分类树用，与 list_products 同过滤口径）。
    pub async fn count_products_by_category(
        &self,
        filter: &FeedProductFilter,
    ) -> Result<Vec<(i32, i64)>> {
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT p.category_id, COUNT(*) FROM product p WHERE ");
        push_product_conditions(&mut qb, filter);
        qb.push(" GROUP BY p.category_id");
        qb.build_query_as::<(i32, i64)>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 批量查多个商品的全部 SKU（feed 列表算哈希/价格用，含禁用行，
    /// 由 service 按 is_enabled 过滤）。
    pub async fn find_skus_for_products(
        &self,
        product_ids: &[i32],
    ) -> Result<Vec<nextrade_core::entities::ProductSku>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT * FROM product_sku WHERE product_id IN (");
        let mut sep = qb.separated(", ");
        for id in product_ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(") ORDER BY product_id ASC, sku_id ASC");
        drop(sep);
        qb.build_query_as::<nextrade_core::entities::ProductSku>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 批量查多个商品的相册（feed 列表算哈希用）。
    pub async fn find_gallery_for_products(
        &self,
        product_ids: &[i32],
    ) -> Result<Vec<ProductGallery>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT * FROM product_gallery WHERE shop_id = 0 AND product_id IN (");
        let mut sep = qb.separated(", ");
        for id in product_ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(") ORDER BY product_id ASC, sort_order ASC, pic_id ASC");
        drop(sep);
        qb.build_query_as::<ProductGallery>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 商品相册（平台图库口径 `shop_id = 0`，与 product_manage 一致）。
    pub async fn find_gallery(&self, product_id: i32) -> Result<Vec<ProductGallery>> {
        sqlx::query_as::<_, ProductGallery>(
            "SELECT * FROM product_gallery WHERE product_id = $1 AND shop_id = 0 ORDER BY sort_order ASC, pic_id ASC",
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }
}

/// 密钥列表过滤条件（list_keys 的 count 与 list 共用）。
fn push_key_filters(qb: &mut QueryBuilder<'_, Postgres>, keyword: Option<&str>, status: Option<i32>) {
    if let Some(kw) = keyword.map(str::trim).filter(|s| !s.is_empty()) {
        qb.push(" AND (key_name ILIKE ");
        qb.push_bind(format!("%{kw}%"));
        qb.push(" OR access_key ILIKE ");
        qb.push_bind(format!("%{kw}%"));
        qb.push(")");
    }
    if let Some(st) = status {
        qb.push(" AND status = ");
        qb.push_bind(st);
    }
}

/// feed 商品可见性 + 过滤条件（规划 §7.3：恒定上架/未删；询价按开关）。
fn push_product_conditions(qb: &mut QueryBuilder<'_, Postgres>, f: &FeedProductFilter) {
    qb.push("p.product_status = 1 AND p.is_delete = 0");
    if let Some(ids) = &f.product_ids {
        qb.push(" AND p.product_id IN (");
        push_id_list(qb, ids);
        qb.push(")");
    }
    if let Some(ids) = &f.category_ids {
        qb.push(" AND p.category_id IN (");
        push_id_list(qb, ids);
        qb.push(")");
    }
    if let Some(ids) = &f.filter_category_ids {
        qb.push(" AND p.category_id IN (");
        push_id_list(qb, ids);
        qb.push(")");
    }
    if let Some(ids) = &f.shop_ids {
        qb.push(" AND p.shop_id IN (");
        push_id_list(qb, ids);
        qb.push(")");
    }
    if !f.include_inquiry {
        qb.push(
            " AND p.is_inquiry = 0 AND NOT EXISTS (SELECT 1 FROM product_sku ps WHERE ps.product_id = p.product_id AND ps.is_enabled = 1 AND ps.is_inquiry = 1)",
        );
    }
    if let Some(since) = f.updated_since {
        qb.push(" AND p.last_update >= ");
        qb.push_bind(since);
    }
    if let Some(kw) = f.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        qb.push(" AND (p.product_name ILIKE ");
        qb.push_bind(format!("%{kw}%"));
        qb.push(" OR p.product_sn ILIKE ");
        qb.push_bind(format!("%{kw}%"));
        qb.push(")");
    }
}

/// IN 列表绑定（空列表绑一个 -1 保证结果为空集而非语法错误）。
fn push_id_list(qb: &mut QueryBuilder<'_, Postgres>, ids: &[i32]) {
    if ids.is_empty() {
        qb.push_bind(-1);
        return;
    }
    let mut sep = qb.separated(", ");
    for id in ids {
        sep.push_bind(*id);
    }
    sep.push_unseparated("");
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("记录不存在".into()),
        other => Error::Database(other.to_string()),
    }
}
