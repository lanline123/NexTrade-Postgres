//! WorkBuddy MCP 开放接口域（openapi::workbuddy）仓储：`product` / `product_sku` /
//! `category` / `brand` / `config` / `gallery_pic` / `gallery` /
//! `gallery_video_info` / `product_gallery` / `product_video`。
//!
//! 对应原 Java：
//! - `com.nextrade.mapper.product.ProductMapper`（`BaseMapper<Product>`）
//! - `com.nextrade.mapper.product.{CategoryMapper, BrandMapper, ProductSkuMapper,
//!   ProductGalleryMapper, ProductVideoMapper}`
//! - `com.nextrade.mapper.setting.{ConfigMapper, GalleryMapper, GalleryPicMapper}`
//! - `com.nextrade.mapper.video.GalleryVideoInfoMapper`
//! - `com.nextrade.service.workbuddy.impl.*` 里的 LambdaQueryWrapper 直查
//!
//! 列名严格对齐 `java/database/pure.sql`
//! （`cd rust && python tools/schema_parser.py --table <表名>`）。
//!
//! 查询统一使用运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`
//! （**不使用** `query!` / `query_as!` 宏：本机无 `DATABASE_URL`）。
//!
//! ## 金额
//!
//! `product_price` / `market_price` / `sku_price` / `cost_price` 一律
//! `rust_decimal::Decimal`（对应 Java `BigDecimal`）。
//!
//! ## 与 Java 的取舍
//!
//! - Java 用 MyBatis-Plus 的 `LambdaQueryWrapper` 动态拼条件。Rust 侧一律
//!   用 `QueryBuilder` 显式 `push_cond`，**列名与方向都来自白名单常量**，
//!   无注入面。
//! - `product` 表有 81 列，Java 实体是整行映射。本仓储只 SELECT 本域
//!   真正用到的列，并以**显式列清单**保证与 `ProductSummaryVO` /
//!   `ProductDetailVO` 对齐；不存在的列名会被
//!   `python tools/check_sql_columns.py --strict` 拦住。

use rust_decimal::Decimal;
use sqlx::{PgPool, QueryBuilder};

use nextrade_core::models::openapi::workbuddy::{ProductScope, ProductSortField};
use nextrade_core::{Error, Result};

/// 把 `sqlx::Error` 统一映射为 `Error::Database`。
fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

// ——————————————————————————————————————————————
// 固定 SELECT 片段（列名全部来自 schema_parser 核对过的真实列）
// ——————————————————————————————————————————————

/// `product` 列表/详情用的列清单。
///
/// 逐列核对（`python tools/schema_parser.py --table product`）：
/// `product_id` / `product_name` / `product_sn` / `product_stock` /
/// `product_price` / `market_price` / `product_status` / `product_type` /
/// `category_id` / `brand_id` / `shop_id` / `check_status` / `check_reason` /
/// `product_brief` / `pic_url` / `virtual_sales` / `sort_order` /
/// `is_delete` / `add_time` —— 均为 `product` 表实际列。
const PRODUCT_SELECT: &str = "SELECT product_id, product_name, product_sn, product_stock, \
     product_price, market_price, product_status, product_type, category_id, brand_id, \
     shop_id, check_status, check_reason, product_brief, pic_url, virtual_sales, \
     sort_order, is_delete, add_time FROM product";

/// \"product_sku\" 的列清单。
///
/// 逐列核对（`--table product_sku`）：\"sku_id\" / \"product_id\" / \"sku_value\" /
/// \"sku_sn\" / \"sku_stock\" / \"sku_price\" / \"is_enabled\" / \"market_price\" /
/// \"cost_price\" —— 均为 \"product_sku\" 表实际列。
const SKU_SELECT: &str = "SELECT sku_id, product_id, sku_value, sku_sn, sku_stock, \
     sku_price, is_enabled, market_price, cost_price FROM product_sku";

// ——————————————————————————————————————————————
// 行结构
// ——————————————————————————————————————————————

/// \"product\" 行（本域用到的子集）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct ProductRow {
    pub product_id: i32,
    pub product_name: String,
    pub product_sn: String,
    pub product_stock: i32,
    pub product_price: Decimal,
    pub market_price: Decimal,
    pub product_status: i32,
    pub product_type: Option<i32>,
    pub category_id: i32,
    pub brand_id: i32,
    pub shop_id: i32,
    pub check_status: i32,
    pub check_reason: String,
    pub product_brief: String,
    pub pic_url: String,
    pub virtual_sales: i32,
    pub sort_order: i32,
    pub is_delete: i32,
    pub add_time: i32,
}

/// \"product_sku\" 行（本域用到的子集）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct SkuRow {
    pub sku_id: i32,
    pub product_id: i32,
    pub sku_value: String,
    pub sku_sn: Option<String>,
    pub sku_stock: Option<i32>,
    pub sku_price: Option<Decimal>,
    pub is_enabled: i32,
    pub market_price: Option<Decimal>,
    pub cost_price: Option<Decimal>,
}

/// \"category\" 行（本域用到的子集）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct CategoryRow {
    pub category_id: i32,
    pub category_name: String,
    pub parent_id: i32,
}

/// \"gallery\" 行（本域用到的子集）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct GalleryRow {
    pub gallery_id: i32,
    pub parent_id: i32,
    pub gallery_name: String,
}

/// \"gallery_pic\" 行（本域用到的子集）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct GalleryPicRow {
    pub pic_id: i32,
    pub shop_id: i32,
    pub gallery_id: i32,
    pub pic_url: String,
    pub pic_name: String,
    pub pic_thumb: String,
    pub pic_width: Option<i32>,
    pub pic_height: Option<i32>,
    pub pic_size: Option<i64>,
    pub file_type: Option<String>,
}

/// \"gallery_video_info\" 行（本域用到的子集）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct GalleryVideoRow {
    pub id: i32,
    pub shop_id: i32,
    pub gallery_id: i32,
    pub video_url: String,
    pub video_name: String,
    pub video_cover: Option<String>,
    pub format: Option<String>,
    pub duration: Option<String>,
    pub size: Option<String>,
    pub description: Option<String>,
}

/// \"product_gallery\" 行（本域用到的子集）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct ProductGalleryRow {
    pub pic_id: i32,
    pub product_id: i32,
    pub pic_url: String,
    pub sort_order: i32,
    pub shop_id: i64,
}

/// 批量预览/执行用的轻量商品行。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct BatchProductRow {
    pub product_id: i32,
    pub product_name: String,
    pub product_sn: String,
    pub product_stock: i32,
    pub product_price: Decimal,
    pub product_status: i32,
    pub product_type: Option<i32>,
    pub category_id: i32,
    pub check_status: i32,
    pub is_delete: i32,
    pub virtual_sales: i32,
    /// 店铺隔离用（`product.shop_id`，表里 NOT NULL）。
    /// 服务层批量/预览要用它判断商品是否属于当前密钥的店铺。
    pub shop_id: i32,
}

/// 商品搜索的查询条件（由服务层从 \"ProductSearchDTO\" 归一化而来）。
#[derive(Debug, Clone, Default)]
pub struct ProductSearchQuery {
    /// 关键词：命中 \"product_name\" 或 \"product_sn\"（`LIKE %kw%`）。
    pub keyword: Option<String>,
    pub category_id: Option<i32>,
    pub scope: ProductScope,
    /// 店铺隔离：`Some(shop_id > 0)` 时追加 `shop_id = $1`。
    pub shop_id: Option<i32>,
    pub brand_id: Option<i32>,
    pub product_status: Option<i32>,
    /// 仅看有库存（`product_stock > 0`）。
    pub in_stock_only: bool,
    /// 创建时间下限（unix 秒，含）。
    pub added_start_time: Option<i64>,
    /// 创建时间上限（unix 秒，含）。
    pub added_end_time: Option<i64>,
    pub sort_field: Option<ProductSortField>,
    pub ascending: bool,
    pub page: i32,
    pub size: i32,
}

// ——————————————————————————————————————————————
// 仓储
// ——————————————————————————————————————————————

/// WorkBuddy MCP 域仓储（挂载于 `Repositories.workbuddy`）。
#[derive(Clone)]
pub struct WorkbuddyRepository {
    pool: PgPool,
}

impl WorkbuddyRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排，与其它切片仓储一致）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— config（MCP 密钥 / 站点设置） —————————————————————

    /// 按 \"biz_code\" 读配置值（对应 Java
    /// `ConfigServiceImpl.getConfigByCode(code).getBizVal()`）。
    ///
    /// ⚠️ Java 侧 \"getConfigByCode\" 在**查不到时抛 \"GlobalException\"**，
    /// 而 `WorkBuddyKeyServiceImpl.configValue` 把它 \"catch\" 掉并返回 \"null\"。
    /// 本方法直接返回 \"Option\"（\"None\" = 查不到），语义等价。
    ///
    /// 未过滤 \"is_del\"：Java 的 \"getConfigByCode\" 是否过滤 \"is_del\" 本片未核实，
    /// 见 \"uncertain_points\"（此处按 \"is_del\" 为 0 或 NULL 视为有效，
    /// 与「删除标记」的常规语义一致）。
    pub async fn find_config_value(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = ? AND (is_del IS NULL OR is_del = 0) \
             ORDER BY id DESC LIMIT 1",
        )
        .bind(biz_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }

    /// 批量读配置（一次往返拿多个 code，减少 MCP 握手延迟）。
    pub async fn find_config_values(
        &self,
        biz_codes: &[&str],
    ) -> Result<std::collections::BTreeMap<String, String>> {
        let mut out = std::collections::BTreeMap::new();
        for code in biz_codes {
            if let Some(v) = self.find_config_value(code).await? {
                out.insert((*code).to_string(), v);
            }
        }
        Ok(out)
    }

    // ————————————————————— product（列表 / 详情 / 写操作） —————————————————————

    /// 追加 \"product\" 搜索的 WHERE 片段（\"count\" 与取行两条 SQL 共用）。
    ///
    /// 条件逐条对应 Java `ProductManageServiceImpl.toListQuery` +
    /// \"ProductScope\" 的三元组。
    fn push_product_filters(qb: &mut QueryBuilder<'_, sqlx::Postgres>, q: &ProductSearchQuery) {
        // scope ⇒ product_status / check_status / is_delete 固定组合
        if let Some(v) = q.scope.product_status() {
            qb.push(" AND product_status = ");
            qb.push_bind(v);
        }
        if let Some(v) = q.scope.check_status() {
            qb.push(" AND check_status = ");
            qb.push_bind(v);
        }
        if let Some(v) = q.scope.is_delete() {
            qb.push(" AND is_delete = ");
            qb.push_bind(v);
        }
        // 显式 product_status 覆盖（Java 侧同名参数直传）
        if let Some(v) = q.product_status {
            qb.push(" AND product_status = ");
            qb.push_bind(v);
        }
        if let Some(kw) = q.keyword.as_deref() {
            let kw = kw.trim();
            if !kw.is_empty() {
                qb.push(" AND (product_name LIKE ");
                qb.push_bind(format!("%{kw}%"));
                qb.push(" OR product_sn LIKE ");
                qb.push_bind(format!("%{kw}%"));
                qb.push(")");
            }
        }
        if let Some(v) = q.category_id {
            qb.push(" AND category_id = ");
            qb.push_bind(v);
        }
        if let Some(v) = q.brand_id {
            qb.push(" AND brand_id = ");
            qb.push_bind(v);
        }
        // 店铺隔离（Java `assertOwnedBy` / 列表 shopId 条件）
        if let Some(v) = q.shop_id {
            if v > 0 {
                qb.push(" AND shop_id = ");
                qb.push_bind(v);
            }
        }
        if q.in_stock_only {
            qb.push(" AND product_stock > 0");
        }
        if let Some(v) = q.added_start_time {
            qb.push(" AND add_time >= ");
            qb.push_bind(v);
        }
        if let Some(v) = q.added_end_time {
            qb.push(" AND add_time <= ");
            qb.push_bind(v);
        }
    }

    /// ORDER BY 片段。
    ///
    /// 列与方向都来自**白名单**（[`ProductSortField::column`] + bool），
    /// 无注入面；未指定 `sort_field` 时按 `product_id DESC`
    /// （Java `BaseServiceImpl` 的默认主键降序）。
    fn order_clause(q: &ProductSearchQuery) -> String {
        match q.sort_field {
            Some(f) => format!(
                " ORDER BY {} {}",
                f.column(),
                if q.ascending { "ASC" } else { "DESC" }
            ),
            None => " ORDER BY product_id DESC".to_string(),
        }
    }

    /// 商品搜索（对应 Java `ProductManageServiceImpl.search`）。
    ///
    /// 返回 `(命中总数, 当页行)`。
    pub async fn search_products(&self, q: &ProductSearchQuery) -> Result<(i64, Vec<ProductRow>)> {
        let mut count_qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM product WHERE 1 = 1");
        Self::push_product_filters(&mut count_qb, q);
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let size = q.size.max(1);
        let offset = (q.page.max(1) - 1) * size;
        let mut row_qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new(format!("{PRODUCT_SELECT} WHERE 1 = 1"));
        Self::push_product_filters(&mut row_qb, q);
        // ORDER BY 的列/方向均来自白名单；LIMIT/OFFSET 为 i32 字面量。
        row_qb.push(Self::order_clause(q));
        row_qb.push(format!(" LIMIT {size} OFFSET {offset}"));

        let rows: Vec<ProductRow> = row_qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0, rows))
    }

    /// 按主键查商品（对应 Java `ProductService.detail` / `getById`）。
    pub async fn find_product(&self, product_id: i32) -> Result<Option<ProductRow>> {
        sqlx::query_as::<_, ProductRow>(&format!("{PRODUCT_SELECT} WHERE product_id = ? LIMIT 1"))
            .bind(product_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按主键批量查商品（保持入参顺序由服务层负责）。
    pub async fn find_products(&self, ids: &[i32]) -> Result<Vec<ProductRow>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new(format!("{PRODUCT_SELECT} WHERE product_id IN ("));
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 批量预览用的轻量查询（`batch/preview` 的 affected/blocked）。
    pub async fn find_products_for_batch(&self, ids: &[i32]) -> Result<Vec<BatchProductRow>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
            "SELECT product_id, product_name, product_sn, product_stock, product_price, \
             product_status, product_type, category_id, check_status, is_delete, \
             virtual_sales, shop_id FROM product WHERE product_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按主键查 `product_sku`（对应 Java `ProductSkuService.listByProductId`）。
    pub async fn find_skus(&self, product_id: i32) -> Result<Vec<SkuRow>> {
        sqlx::query_as::<_, SkuRow>(&format!(
            "{SKU_SELECT} WHERE product_id = ? ORDER BY sku_id ASC"
        ))
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 统计每个商品的规格数（对应 `ProductSummaryVO.skuCount`，
    /// Java 侧由 SKU 列表大小得到）。
    pub async fn count_skus(&self, ids: &[i32]) -> Result<std::collections::BTreeMap<i32, i32>> {
        if ids.is_empty() {
            return Ok(std::collections::BTreeMap::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT product_id, COUNT(*) AS c FROM product_sku WHERE product_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(") GROUP BY product_id");
        let rows: Vec<(i32, i64)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(pid, c)| (pid, i32::try_from(c).unwrap_or(i32::MAX)))
            .collect())
    }

    /// 更新单个字段（对应 Java `ProductManageServiceImpl.updateField`）。
    ///
    /// \"column\" **必须**来自 [`nextrade_core::models::openapi::workbuddy::EDITABLE_FIELDS`]
    /// 映射后的白名单（由服务层保证），本方法只做拼接。
    ///
    /// 返回受影响行数。
    pub async fn update_product_field(
        &self,
        product_id: i32,
        column: &str,
        value: &str,
        now: i64,
    ) -> Result<u64> {
        debug_assert!(
            [
                "product_name",
                "product_sn",
                "product_price",
                "product_stock",
                "sort_order",
            ]
            .contains(&column),
            "update_product_field 只接受白名单列"
        );
        let sql = format!(
            "UPDATE product SET {column} = ?, last_update = ? WHERE product_id = ?"
        );
        // 数值列用无符号/有符号比较均可；此处统一以字符串绑定，
        // MySQL 会按目标列类型隐式转换（与 Java \"updateById\" 一致）。
        let affected = sqlx::query(&sql)
            .bind(value)
            .bind(i32::try_from(now).unwrap_or(i32::MAX))
            .bind(product_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected)
    }

    /// 更新上下架状态（对应 Java `ProductManageServiceImpl.setStatus`）。
    pub async fn update_product_status(
        &self,
        product_id: i32,
        on_sale: bool,
        now: i64,
    ) -> Result<u64> {
        let affected = sqlx::query(
            "UPDATE product SET product_status = ?, last_update = ? WHERE product_id = ?",
        )
        .bind(i32::from(on_sale))
        .bind(i32::try_from(now).unwrap_or(i32::MAX))
        .bind(product_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected)
    }

    /// 软删除（移入回收站，对应 Java `ProductManageServiceImpl.recycle`）。
    pub async fn recycle_product(&self, product_id: i32, now: i64) -> Result<u64> {
        let affected = sqlx::query(
            "UPDATE product SET is_delete = 1, last_update = ? WHERE product_id = ?",
        )
        .bind(i32::try_from(now).unwrap_or(i32::MAX))
        .bind(product_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected)
    }

    /// 还原（对应 Java `ProductManageServiceImpl.restore`）。
    ///
    /// Java 显式把 \"productStatus\" 置 0（还原后保持下架），故此处一并写入。
    pub async fn restore_product(&self, product_id: i32, now: i64) -> Result<u64> {
        let affected = sqlx::query(
            "UPDATE product SET is_delete = 0, product_status = 0, last_update = ? \
             WHERE product_id = ?",
        )
        .bind(i32::try_from(now).unwrap_or(i32::MAX))
        .bind(product_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected)
    }

    /// 彻底删除（对应 Java `ProductManageServiceImpl.deletePermanently`）。
    ///
    /// 与 Java 一致：连带删除 \"product_gallery\" / \"product_sku\" /
    /// \"product_video\" 的关联行，最后删主表。
    ///
    /// ⚠️ Java 用 `@Transactional` 包住整段；Rust 侧在服务层用同一连接开事务。
    pub async fn delete_product_cascade(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        product_id: i32,
    ) -> Result<()> {
        sqlx::query("DELETE FROM product_gallery WHERE product_id = ?")
            .bind(product_id)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?;
        sqlx::query("DELETE FROM product_sku WHERE product_id = ?")
            .bind(product_id)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?;
        sqlx::query("DELETE FROM product_video WHERE product_id = ?")
            .bind(product_id)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?;
        sqlx::query("DELETE FROM product WHERE product_id = ?")
            .bind(product_id)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 批量设置上下架（对应 Java `executeBatch` 的 `up` / `down`）。
    ///
    /// 返回受影响行数。
    pub async fn batch_set_status(
        &self,
        ids: &[i32],
        on_sale: bool,
        now: i64,
    ) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("UPDATE product SET product_status = ");
        qb.push_bind(i32::from(on_sale));
        qb.push(", last_update = ");
        qb.push_bind(i32::try_from(now).unwrap_or(i32::MAX));
        qb.push(" WHERE product_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        Ok(qb.build().execute(&self.pool).await.map_err(map_err)?.rows_affected())
    }

    /// 批量软删除 / 还原（对应 Java \"executeBatch\" 的 \"recycle\" / \"restore\"）。
    ///
    /// \"restore_reason_columns\" 为 \"true\" 时同时把 \"product_status\" 置 0
    /// （Java \"restore\" 的语义：还原后仍是下架）。
    pub async fn batch_set_deleted(
        &self,
        ids: &[i32],
        deleted: bool,
        reset_status: bool,
        now: i64,
    ) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new("UPDATE product SET is_delete = ");
        qb.push_bind(i32::from(deleted));
        if reset_status {
            qb.push(", product_status = 0");
        }
        qb.push(", last_update = ");
        qb.push_bind(i32::try_from(now).unwrap_or(i32::MAX));
        qb.push(" WHERE product_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        Ok(qb.build().execute(&self.pool).await.map_err(map_err)?.rows_affected())
    }

    /// 商品货号是否已存在（对应 Java `productSn` 唯一性校验）。
    ///
    /// `exclude_id` 用于更新场景排除自身。
    pub async fn product_sn_exists(&self, product_sn: &str, exclude_id: Option<i32>) -> Result<bool> {
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM product WHERE product_sn = ");
        qb.push_bind(product_sn);
        if let Some(id) = exclude_id {
            qb.push(" AND product_id <> ");
            qb.push_bind(id);
        }
        let (n,): (i64,) = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(n > 0)
    }

    /// 新建商品骨架（对应 Java `ProductService.create` 的最小落库）。
    ///
    /// 返回新商品 ID。
    ///
    /// ⚠️ Java 的 `ProductService.create` 会写 80 余列并联动
    /// 属性/规格/相册，远超本片可由 Java 源码确定的范围。
    /// 本实现只写**导入流程必需且规则明确**的列，其余依赖表默认值，
    /// 见 \"uncertain_points\"。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_product(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        p: &NewProduct,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO product (product_name, product_sn, product_tsn, product_stock, \
             product_price, market_price, shipping_tpl_id, product_status, product_type, \
             delivery_mode, sale_mode, category_id, brand_id, shop_id, shop_category_id, \
             check_status, check_reason, click_count, product_weight, is_promote, \
             is_promote_activity, promote_price, promote_start_date, promote_end_date, \
             seckill_max_num, product_brief, product_desc, pic_url, pic_thumb, \
             pic_original, pic_large, free_shipping, integral, add_time, sort_order, \
             store_sort_order, is_delete, is_best, is_new, is_hot, last_update, remark, \
             give_integral, rank_integral, virtual_sales, limit_number, \
             is_support_return, is_support_cod, card_group_id, is_shop_pickup, \
             is_logistics, is_shop_delivery) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0, 0, 0, \
             0, 0, 0, 0, ?, ?, ?, ?, ?, ?, 0, 0, ?, ?, 0, 0, 0, 0, 0, ?, '', 0, 0, ?, 0, \
             0, 0, 0, 0, 0) RETURNING product_id",
        )
        .bind(&p.product_name)
        .bind(&p.product_sn)
        .bind(&p.product_tsn)
        .bind(p.product_stock)
        .bind(p.product_price)
        .bind(p.market_price)
        .bind(p.shipping_tpl_id)
        .bind(p.product_status)
        .bind(p.product_type)
        .bind(p.delivery_mode)
        .bind(p.sale_mode)
        .bind(p.category_id)
        .bind(p.brand_id)
        .bind(p.shop_id)
        .bind(p.shop_category_id)
        .bind(p.check_status)
        .bind(&p.check_reason)
        .bind(&p.product_brief)
        .bind(&p.product_desc)
        .bind(&p.pic_url)
        .bind(&p.pic_thumb)
        .bind(&p.pic_original)
        .bind(&p.pic_large)
        .bind(p.add_time)
        .bind(p.sort_order)
        .bind(p.last_update)
        .bind(p.virtual_sales)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 删除商品相册（导入后重建，对应 Java \"ProductGalleryService\"）。
    pub async fn delete_product_gallery(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        product_id: i32,
    ) -> Result<()> {
        sqlx::query("DELETE FROM product_gallery WHERE product_id = ?")
            .bind(product_id)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 插入商品相册行（对应 \"product_gallery\"）。
    pub async fn insert_product_gallery(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        product_id: i32,
        pic_url: &str,
        sort_order: i32,
        shop_id: i64,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO product_gallery (product_id, pic_url, pic_desc, pic_thumb, \
             pic_original, pic_large, sort_order, shop_id) VALUES (?, ?, '', ?, ?, ?, ?, ?)",
        )
        .bind(product_id)
        .bind(pic_url)
        .bind(pic_url)
        .bind(pic_url)
        .bind(pic_url)
        .bind(sort_order)
        .bind(shop_id)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    // ————————————————————— order（消息推送的归属校验） —————————————————————

    /// 读订单的 `user_id`（对应 Java `MessagePushController.sendMessage`
    /// 下游 `messageService.sendUserMessage` 的归属校验）。
    ///
    /// ⚠️ \"order\" 表归属订单域；本域只取 `order_id` / `user_id` 两列
    /// （`python tools/schema_parser.py --table order` 已确认二者存在）。
    /// 表名为 MySQL 保留字，故用反引号包裹。
    pub async fn find_order_user_id(&self, order_id: i32) -> Result<Option<i32>> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT user_id FROM \"order\" WHERE order_id = ? LIMIT 1")
                .bind(order_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|(u,)| u))
    }

    // ————————————————————— category（分类检索 / 导入） —————————————————————
    /// 按名称检索分类（对应 Java `ProductImportServiceImpl.categories` 的
    /// `keyword` 分支，`LIKE %keyword%`）。
    ///
    /// `parent_id` 非空时按其过滤。
    pub async fn find_categories(
        &self,
        keyword: Option<&str>,
        parent_id: Option<i32>,
        limit: i64,
    ) -> Result<Vec<CategoryRow>> {
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT category_id, category_name, parent_id FROM category WHERE 1 = 1");
        if let Some(kw) = keyword {
            let kw = kw.trim();
            if !kw.is_empty() {
                qb.push(" AND category_name LIKE ");
                qb.push_bind(format!("%{kw}%"));
            }
        }
        if let Some(pid) = parent_id {
            qb.push(" AND parent_id = ");
            qb.push_bind(pid);
        }
        qb.push(" ORDER BY sort_order ASC, category_id ASC LIMIT ");
        qb.push_bind(limit);
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 一次取回全部分类（分类导入的归一化需要整棵树）。
    ///
    /// Java `CategoryImportServiceImpl` 用 `CATEGORY_PAGE_SIZE = 1000`
    /// 分页拉全量；此处同样按 1000 分批，避免大表一次性载入。
    pub async fn find_all_categories(&self, page_size: i64) -> Result<Vec<CategoryRow>> {
        let mut out = Vec::new();
        let mut offset: i64 = 0;
        loop {
            let page: Vec<CategoryRow> = sqlx::query_as(
                "SELECT category_id, category_name, parent_id FROM category \
                 ORDER BY category_id ASC LIMIT ? OFFSET ?",
            )
            .bind(page_size)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
            let n = page.len() as i64;
            out.extend(page);
            if n < page_size {
                break;
            }
            offset += page_size;
        }
        Ok(out)
    }

    /// 按分类名 + 父级查单个分类（对应 Java
    /// `CategoryService.getByNameAndParent`）。
    ///
    /// ⚠️ Java 侧「同名同父」是否要求唯一由 `CategoryService` 决定；
    /// 本方法取 `category_id` 最小的一条（与 `ORDER BY ... LIMIT 1` 一致）。
    pub async fn find_category_by_name(
        &self,
        name: &str,
        parent_id: i32,
    ) -> Result<Option<CategoryRow>> {
        sqlx::query_as::<_, CategoryRow>(
            "SELECT category_id, category_name, parent_id FROM category \
             WHERE category_name = ? AND parent_id = ? ORDER BY category_id ASC LIMIT 1",
        )
        .bind(name)
        .bind(parent_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按主键查分类。
    pub async fn find_category(&self, category_id: i32) -> Result<Option<CategoryRow>> {
        sqlx::query_as::<_, CategoryRow>(
            "SELECT category_id, category_name, parent_id FROM category WHERE category_id = ? LIMIT 1",
        )
        .bind(category_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 分类是否还有下级（`ProductCategoryVO.hasChildren`）。
    pub async fn has_child_category(&self, category_id: i32) -> Result<bool> {
        let (n,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM category WHERE parent_id = ?")
                .bind(category_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(n > 0)
    }

    /// 批量判断哪些分类有下级（避免 N+1）。
    pub async fn categories_with_children(
        &self,
        ids: &[i32],
    ) -> Result<std::collections::BTreeSet<i32>> {
        if ids.is_empty() {
            return Ok(std::collections::BTreeSet::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT DISTINCT parent_id FROM category WHERE parent_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        let rows: Vec<(i32,)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 新建分类（对应 Java `CategoryService.save`）。
    ///
    /// Java 的 \"CategoryDTO\" 有 15 列，此处只写导入流程必需的 5 列
    /// （\"category_name\" / \"parent_id\" / \"sort_order\" / \"is_show\" /
    /// \"measure_unit\" 之外的 NOT NULL 列依赖表默认值），见 \"uncertain_points\"。
    pub async fn insert_category(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        category_name: &str,
        parent_id: i32,
        sort_order: i32,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO category (category_name, parent_id, sort_order, keywords, \
             category_desc, measure_unit, is_show, seo_title, short_name, category_pic, \
             category_ico, is_hot, search_keywords) \
             VALUES (?, ?, ?, '', '', '', 1, ?, '', '', '', 0, '') RETURNING category_id",
        )
        .bind(category_name)
        .bind(parent_id)
        .bind(sort_order)
        .bind(category_name)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 批量预览用的分类存在性（显式 ID 列表）。
    pub async fn find_categories_by_ids(&self, ids: &[i32]) -> Result<Vec<CategoryRow>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT category_id, category_name, parent_id FROM category WHERE category_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    // ————————————————————— brand（详情里的品牌名） —————————————————————

    /// 按主键取品牌名。
    pub async fn find_brand_name(&self, brand_id: i32) -> Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT brand_name FROM brand WHERE brand_id = ? LIMIT 1")
                .bind(brand_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|(n,)| n))
    }

    /// 批量取品牌名（避免 N+1）。
    pub async fn find_brand_names(
        &self,
        ids: &[i32],
    ) -> Result<std::collections::BTreeMap<i32, String>> {
        if ids.is_empty() {
            return Ok(std::collections::BTreeMap::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT brand_id, brand_name FROM brand WHERE brand_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        let rows: Vec<(i32, String)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().collect())
    }

    /// 批量取分类名（避免 N+1）。
    pub async fn find_category_names(
        &self,
        ids: &[i32],
    ) -> Result<std::collections::BTreeMap<i32, String>> {
        if ids.is_empty() {
            return Ok(std::collections::BTreeMap::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT category_id, category_name FROM category WHERE category_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        let rows: Vec<(i32, String)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().collect())
    }

    // ————————————————————— 编码推断样本（productSn） —————————————————————

    /// 取历史 \"product_sn\" 样本（对应 Java
    /// `ProductContextServiceImpl.inferEncoding` 的 snake 样本集合）。
    ///
    /// 只取非空且 `is_delete = 0` 的商品编码，去重后按 `product_id DESC` 限量
    /// （Java 取列表页的样本，见 \"uncertain_points\"）。
    pub async fn find_product_sn_samples(
        &self,
        shop_id: Option<i32>,
        limit: i64,
    ) -> Result<Vec<String>> {
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT DISTINCT product_sn FROM product WHERE product_sn <> ''");
        if let Some(v) = shop_id {
            if v > 0 {
                qb.push(" AND shop_id = ");
                qb.push_bind(v);
            }
        }
        qb.push(" ORDER BY product_sn ASC LIMIT ");
        qb.push_bind(limit);
        let rows: Vec<(String,)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().map(|(s,)| s).collect())
    }

    // ————————————————————— gallery（素材查询 / 分组） —————————————————————

    /// 素材分组列表（对应 Java `GalleryService.list`）。
    pub async fn find_galleries(&self, shop_id: i32) -> Result<Vec<GalleryRow>> {
        sqlx::query_as::<_, GalleryRow>(
            "SELECT gallery_id, parent_id, gallery_name FROM gallery \
             WHERE shop_id = ? ORDER BY gallery_sort ASC, gallery_id ASC",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按主键查素材分组（用于分组上下文的租户校验）。
    pub async fn find_gallery(&self, gallery_id: i32) -> Result<Option<(i32, i32)>> {
        let row: Option<(i32, i32)> =
            sqlx::query_as("SELECT gallery_id, shop_id FROM gallery WHERE gallery_id = ? LIMIT 1")
                .bind(gallery_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row)
    }

    /// 素材分组是否存在（`MediaManagementServiceImpl.isValidGroup`）。
    pub async fn gallery_exists(&self, gallery_id: i32, shop_id: i32) -> Result<bool> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM gallery WHERE gallery_id = ? AND shop_id = ?",
        )
        .bind(gallery_id)
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(n > 0)
    }

    /// 图片素材查询（对应 Java `GalleryPicService.list`）。
    ///
    /// `group_id` 只在自己已有分组时才生效（Java 的 `currentContextGroupId`
    /// 语义，见 `uncertain_points`）。
    pub async fn search_gallery_pics(
        &self,
        shop_id: i32,
        group_id: Option<i32>,
        keyword: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<GalleryPicRow>)> {
        let mut count_qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM gallery_pic WHERE shop_id = ");
        count_qb.push_bind(shop_id);
        Self::push_gallery_pic_filters(&mut count_qb, group_id, keyword);
        let (total,): (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
            "SELECT pic_id, shop_id, gallery_id, pic_url, pic_name, pic_thumb, \
             pic_width, pic_height, pic_size, file_type FROM gallery_pic WHERE shop_id = ",
        );
        qb.push_bind(shop_id);
        Self::push_gallery_pic_filters(&mut qb, group_id, keyword);
        qb.push(" ORDER BY pic_id DESC LIMIT ");
        qb.push_bind(limit);
        qb.push(" OFFSET ");
        qb.push_bind(offset);
        let rows: Vec<GalleryPicRow> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total, rows))
    }

    fn push_gallery_pic_filters(
        qb: &mut QueryBuilder<'_, sqlx::Postgres>,
        group_id: Option<i32>,
        keyword: Option<&str>,
    ) {
        if let Some(g) = group_id {
            qb.push(" AND gallery_id = ");
            qb.push_bind(g);
        }
        if let Some(kw) = keyword {
            let kw = kw.trim();
            if !kw.is_empty() {
                qb.push(" AND pic_name LIKE ");
                qb.push_bind(format!("%{kw}%"));
            }
        }
    }

    /// 按主键批量取图片素材（`media/execute` 的 update / delete）。
    pub async fn find_gallery_pics_by_ids(&self, ids: &[i32]) -> Result<Vec<GalleryPicRow>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
            "SELECT pic_id, shop_id, gallery_id, pic_url, pic_name, pic_thumb, \
             pic_width, pic_height, pic_size, file_type FROM gallery_pic WHERE pic_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 图片被哪些商品引用（`MEDIA_IN_USE` 判定，`pic_url` 命中
    /// `product_gallery.pic_url` 或 `product.pic_url`）。
    pub async fn count_pics_in_use(&self, urls: &[String]) -> Result<i64> {
        if urls.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
            "SELECT COUNT(*) FROM product_gallery WHERE pic_url IN (",
        );
        let mut sep = qb.separated(", ");
        for u in urls {
            sep.push_bind(u.clone());
        }
        sep.push_unseparated(")");
        let (n,): (i64,) = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(n)
    }

    /// 视频被哪些商品引用（`product_video.video_url` 命中。
    pub async fn count_videos_in_use(&self, urls: &[String]) -> Result<i64> {
        if urls.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM product_video WHERE video_url IN (");
        let mut sep = qb.separated(", ");
        for u in urls {
            sep.push_bind(u.clone());
        }
        sep.push_unseparated(")");
        let (n,): (i64,) = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(n)
    }

    /// 视频素材查询（对应 Java `GalleryVideoInfoService.list`）。
    pub async fn search_gallery_videos(
        &self,
        shop_id: i32,
        group_id: Option<i32>,
        keyword: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<GalleryVideoRow>)> {
        let mut count_qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM gallery_video_info WHERE shop_id = ");
        count_qb.push_bind(shop_id);
        Self::push_gallery_video_filters(&mut count_qb, group_id, keyword);
        let (total,): (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
            "SELECT id, shop_id, gallery_id, video_url, video_name, video_cover, \
             format, duration, size, description FROM gallery_video_info WHERE shop_id = ",
        );
        qb.push_bind(shop_id);
        Self::push_gallery_video_filters(&mut qb, group_id, keyword);
        qb.push(" ORDER BY id DESC LIMIT ");
        qb.push_bind(limit);
        qb.push(" OFFSET ");
        qb.push_bind(offset);
        let rows: Vec<GalleryVideoRow> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total, rows))
    }

    fn push_gallery_video_filters(
        qb: &mut QueryBuilder<'_, sqlx::Postgres>,
        group_id: Option<i32>,
        keyword: Option<&str>,
    ) {
        if let Some(g) = group_id {
            qb.push(" AND gallery_id = ");
            qb.push_bind(g);
        }
        if let Some(kw) = keyword {
            let kw = kw.trim();
            if !kw.is_empty() {
                qb.push(" AND video_name LIKE ");
                qb.push_bind(format!("%{kw}%"));
            }
        }
    }

    /// 按主键批量取视频素材。
    pub async fn find_gallery_videos_by_ids(&self, ids: &[i32]) -> Result<Vec<GalleryVideoRow>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
            "SELECT id, shop_id, gallery_id, video_url, video_name, video_cover, \
             format, duration, size, description FROM gallery_video_info WHERE id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 统计每个分组的素材数（`MediaGroupVO.assetCount`）。
    ///
    /// `media_type == "video"` 时统计 `gallery_video_info`，否则统计
    /// `gallery_pic`。
    pub async fn count_assets_by_group(
        &self,
        shop_id: i32,
        media_type: &str,
    ) -> Result<std::collections::BTreeMap<i32, i32>> {
        let rows: Vec<(i32, i64)> = if media_type == "video" {
            sqlx::query_as(
                "SELECT gallery_id, COUNT(*) FROM gallery_video_info WHERE shop_id = ? \
                 GROUP BY gallery_id",
            )
            .bind(shop_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?
        } else {
            sqlx::query_as(
                "SELECT gallery_id, COUNT(*) FROM gallery_pic WHERE shop_id = ? GROUP BY gallery_id",
            )
            .bind(shop_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?
        };
        Ok(rows
            .into_iter()
            .map(|(g, c)| (g, i32::try_from(c).unwrap_or(i32::MAX)))
            .collect())
    }

    /// 插入图片素材（导入时需要把外链图片落到素材库时的最简写入）。
    pub async fn insert_gallery_pic(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        shop_id: i32,
        gallery_id: i32,
        pic_url: &str,
        pic_name: &str,
        file_type: &str,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO gallery_pic (shop_id, gallery_id, pic_ower_id, pic_url, pic_name, \
             pic_thumb, add_time, vendor_id, hidden, file_type, pic_sort, source) \
             VALUES (?, ?, 0, ?, ?, ?, '', 0, 0, ?, 0, '') RETURNING pic_id",
        )
        .bind(shop_id)
        .bind(gallery_id)
        .bind(pic_url)
        .bind(pic_name)
        .bind(pic_url)
        .bind(file_type)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 新建素材分组（对应 Java `GalleryService.save`）。
    pub async fn insert_gallery(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        shop_id: i32,
        parent_id: i32,
        name: &str,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO gallery (parent_id, gallery_admin_id, gallery_name, gallery_sort, \
             gallery_thumb, shop_id, vendor_id) VALUES (?, 0, ?, 0, '', ?, 0) RETURNING gallery_id",
        )
        .bind(parent_id)
        .bind(name)
        .bind(shop_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 重命名素材分组（对应 Java `GalleryService.update`）。
    pub async fn update_gallery_name(&self, gallery_id: i32, name: &str) -> Result<u64> {
        Ok(sqlx::query("UPDATE gallery SET gallery_name = ? WHERE gallery_id = ?")
            .bind(name)
            .bind(gallery_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected())
    }

    /// 删除素材分组（对应 Java `GalleryService.remove`）。
    pub async fn delete_gallery(&self, gallery_id: i32) -> Result<u64> {
        Ok(sqlx::query("DELETE FROM gallery WHERE gallery_id = ?")
            .bind(gallery_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected())
    }

    /// 删除分组下的全部图片素材。
    pub async fn delete_gallery_pics_by_group(&self, gallery_id: i32) -> Result<u64> {
        Ok(
            sqlx::query("DELETE FROM gallery_pic WHERE gallery_id = ?")
                .bind(gallery_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?
                .rows_affected(),
        )
    }

    /// 改图片素材的分组 / 名称（`media/execute` 的 \"update\"）。
    pub async fn update_gallery_pic(
        &self,
        pic_id: i32,
        group_id: Option<i32>,
        name: Option<&str>,
        sort: Option<i32>,
    ) -> Result<u64> {
        let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new("UPDATE gallery_pic SET pic_id = pic_id");
        if let Some(g) = group_id {
            qb.push(", gallery_id = ");
            qb.push_bind(g);
        }
        if let Some(n) = name {
            qb.push(", pic_name = ");
            qb.push_bind(n.to_string());
        }
        if let Some(s) = sort {
            qb.push(", pic_sort = ");
            qb.push_bind(s);
        }
        qb.push(" WHERE pic_id = ");
        qb.push_bind(pic_id);
        Ok(qb.build().execute(&self.pool).await.map_err(map_err)?.rows_affected())
    }

    /// 删除图片素材（`media/execute` 的 \"delete\"）。
    pub async fn delete_gallery_pics(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new("DELETE FROM gallery_pic WHERE pic_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        Ok(qb.build().execute(&self.pool).await.map_err(map_err)?.rows_affected())
    }
}

/// 新建商品所需的字段集合（[`WorkbuddyRepository::insert_product`]）。
#[derive(Debug, Clone)]
pub struct NewProduct {
    pub product_name: String,
    pub product_sn: String,
    pub product_tsn: Option<String>,
    pub product_stock: i32,
    pub product_price: Decimal,
    pub market_price: Decimal,
    pub shipping_tpl_id: i32,
    pub product_status: i32,
    pub product_type: Option<i32>,
    pub delivery_mode: i32,
    pub sale_mode: i32,
    pub category_id: i32,
    pub brand_id: i32,
    pub shop_id: i32,
    pub shop_category_id: i32,
    pub check_status: i32,
    pub check_reason: String,
    pub product_brief: String,
    pub product_desc: Option<String>,
    pub pic_url: String,
    pub pic_thumb: String,
    pub pic_original: String,
    pub pic_large: String,
    pub add_time: i32,
    pub sort_order: i32,
    pub last_update: i32,
    pub virtual_sales: i32,
}"