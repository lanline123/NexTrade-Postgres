//! C 端批发域（wholesale::wholesale_client）仓储。
//!
//! 覆盖表：`config`（批发总开关）、`wholesaler` / `wholesaler_grade`（批发身份）、
//! `product_wholesale_config` / `product_wholesale_sku_config` /
//! `product_wholesale_price`（批发能力配置）、`cart`（批发行落库）。
//!
//! 列名严格对齐 `java/database/pure.sql`（用
//! `python tools/schema_parser.py --table <表名>` 核对）；查询统一使用运行时
//! `sqlx::query_as` / `sqlx::query`（**不使用** `query!` 宏）。
//!
//! > **SQL 写法约定**：本文件所有 SQL 都写成**单行字符串字面量**。
//! > `tools/check_sql_columns.py` 的 SQL 提取器要求一个字符串字面量的
//! > 起止引号出现在同一行，跨行 SQL 会被静默跳过（列名也就不会被校验）。
//! > 为保证每一列都真的过校验器，这里刻意不换行。
//!
//! 过滤口径逐条对齐原 Java：
//! - `wholesaler`：`WholesalerServiceImpl#queryActive` → `user_id = ? AND status = 1`
//!   （`is_delete` 由 `@TableLogic` 自动过滤 → SQL 显式写 `is_delete = 0`）。
//! - `wholesaler_grade`：`WholesaleClientController#me` 链路用 `getById`，**不校验** `status`。
//! - `product_wholesale_config`：`getActiveByProductId` → `product_id = ? AND status = 1`
//!   （`LIMIT 1`）。
//! - `product_wholesale_sku_config` / `product_wholesale_price`：Mapper XML
//!   `listByProductId` → 仅 `is_delete = 0 AND product_id = ?`（不校验 `status`）。
//! - `config`：`ConfigService#getConfigVal` → `biz_code = ? AND is_del = 0 LIMIT 1`。

use rust_decimal::Decimal;
use sqlx::PgPool;
use nextrade_core::entities::{
    ProductWholesaleConfig, ProductWholesalePrice, ProductWholesaleSkuConfig, Wholesaler,
};
use nextrade_core::{Error, Result};

/// 批发功能总开关的配置项 `biz_code`
/// （Java `SettingsEnum.WHOLESALE_ENABLED.getBizCode()`）。
pub const WHOLESALE_ENABLED_BIZ_CODE: &str = "wholesaleEnabled";

/// 购物车行类型：普通商品（Java `CartTypeEnum.TYPE_NORMAL` = 1）。
///
/// `WholesaleCartBatchServiceImpl#toAddToCartDTO` 不设置 `type`，
/// 由 `AddToCartDTO#initParam` 兜底为 `TYPE_NORMAL`。
pub const CART_TYPE_NORMAL: i32 = 1;

/// 新增购物车行的入参。
///
/// 未列入的列由本仓储按 Java `CartServiceImpl#addProductToCart` 的取值写死：
/// `is_checked = 1`、`type = 1`（`TYPE_NORMAL`）、`salesman_id = 0`、
/// `extra_sku_data = ''`、`extra_data = '{}'`。
#[derive(Debug, Clone)]
pub struct NewCartLine {
    /// `cart.user_id`
    pub user_id: i32,
    /// `cart.product_id`
    pub product_id: i32,
    /// `cart.product_sn`（取 `product.product_sn`）
    pub product_sn: String,
    /// `cart.pic_thumb`（取 `product.pic_thumb`）
    pub pic_thumb: String,
    /// `cart.market_price`（取 `product.market_price`）
    pub market_price: Decimal,
    /// `cart.original_price`（有 SKU 取 `sku_price`，无 SKU 取商品当前售价）
    pub original_price: Decimal,
    /// `cart.quantity`（本项数量 + 已存在同款行数量）
    pub quantity: i32,
    /// `cart.sku_id`（`0` = SPU 兜底，与 Java 归一化口径一致）
    pub sku_id: i32,
    /// `cart.sku_data`（Java 无 SKU 时写 `"[]"`）
    pub sku_data: String,
    /// `cart.product_type`（取 `product.product_type`）
    pub product_type: i32,
    /// `cart.shop_id`（非 O2O 取 `product.shop_id`）
    pub shop_id: i32,
    /// `cart.is_wholesale_line`（`1` = 批发行）
    pub is_wholesale_line: i32,
    /// `cart.wholesale_grade_id`（加车时的等级快照，`0` = 无）
    pub wholesale_grade_id: i32,
}

#[derive(Clone)]
pub struct WholesaleClientRepository {
    pool: PgPool,
}

impl WholesaleClientRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 批发总开关（`config`） —————————————————————

    /// 批发功能总开关是否启用（Java `WholesaleSettingServiceImpl#isEnabled`）。
    ///
    /// `readInt`：配置缺失 / 空串 / 无法解析整数时回退到默认值 `"0"`，
    /// 故只有解析结果恰为 `1` 才算启用。
    pub async fn wholesale_enabled(&self) -> Result<bool> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1")
                .bind(WHOLESALE_ENABLED_BIZ_CODE)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        let value = row
            .and_then(|(v,)| v)
            .and_then(|v| v.trim().parse::<i32>().ok())
            .unwrap_or(0);
        Ok(value == 1)
    }

    // ————————————————————— 批发商身份（`wholesaler` / `wholesaler_grade`） —————————————————————

    /// 启用中的批发商身份（Java `WholesalerServiceImpl#queryActive`：
    /// `user_id = ? AND status = 1` + `@TableLogic is_delete = 0`）。
    ///
    /// 返回 `None` 表示该会员不是（启用中的）批发商。
    pub async fn find_active_wholesaler(&self, user_id: i32) -> Result<Option<Wholesaler>> {
        sqlx::query_as::<_, Wholesaler>("SELECT * FROM wholesaler WHERE user_id = $1 AND status = 1 AND is_delete = 0 LIMIT 1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 批发等级名称（Java `WholesalerGradeService#getById` 仅取 `gradeName`）。
    ///
    /// **不校验** `status`：Java `WholesaleClientController#me` 只判空，
    /// 未启用的等级同样回填名称。
    pub async fn find_grade_name(&self, grade_id: i32) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as("SELECT grade_name FROM wholesaler_grade WHERE grade_id = $1 AND is_delete = 0 LIMIT 1")
            .bind(grade_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.map(|(name,)| name))
    }

    // ————————————————————— 批发商品配置 —————————————————————

    /// 生效中的商品级批发配置（Java
    /// `WholesaleProductConfigService#getActiveByProductId`）。
    pub async fn find_active_config(
        &self,
        product_id: i32,
    ) -> Result<Option<ProductWholesaleConfig>> {
        sqlx::query_as::<_, ProductWholesaleConfig>("SELECT * FROM product_wholesale_config WHERE product_id = $1 AND status = 1 AND is_delete = 0 LIMIT 1")
            .bind(product_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 商品的全部 SKU 级批发配置（Java
    /// `WholesaleSkuConfigService#listByProductId` → Mapper XML
    /// `is_delete = 0 AND product_id = ?`）。
    pub async fn list_sku_configs(
        &self,
        product_id: i32,
    ) -> Result<Vec<ProductWholesaleSkuConfig>> {
        sqlx::query_as::<_, ProductWholesaleSkuConfig>("SELECT * FROM product_wholesale_sku_config WHERE is_delete = 0 AND product_id = $1")
            .bind(product_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 商品的全部批发价候选行（Java `WholesalePriceService#listByProductId`
    /// → Mapper XML `is_delete = 0 AND product_id = ?`）。
    ///
    /// `status` / 维度匹配交由纯函数 `resolve_wholesale_price` 兜底过滤，
    /// 与 Java `WholesalePriceResolver.resolve` 的自过滤口径一致。
    pub async fn list_prices(&self, product_id: i32) -> Result<Vec<ProductWholesalePrice>> {
        sqlx::query_as::<_, ProductWholesalePrice>("SELECT * FROM product_wholesale_price WHERE is_delete = 0 AND product_id = $1")
            .bind(product_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    // ————————————————————— 购物车（`cart`） —————————————————————

    /// 已存在的同款购物车行 `(cart_id, quantity)`（Java
    /// `CartServiceImpl#addProductToCart` 的 `matchedCarts` 查询）。
    ///
    /// 口径：`user_id` + `product_id` + `type` 等值；`sku_id` 允许
    /// `NULL` 与 `0` 相互匹配（Java 把无规格归一为 `0`，但历史行可能是 `NULL`）；
    /// `shop_id` 仅在 O2O 形态下参与过滤（非 O2O 传 `None`）。
    pub async fn find_cart_lines(
        &self,
        user_id: i32,
        product_id: i32,
        cart_type: i32,
        sku_id: i32,
        shop_id: Option<i32>,
    ) -> Result<Vec<(i32, i32)>> {
        sqlx::query_as::<_, (i32, i32)>("SELECT cart_id, quantity FROM cart WHERE user_id = $1 AND product_id = $2 AND \"type\" = $3 AND (sku_id = $4 OR (sku_id IS NULL AND $5 = 0)) AND ($6 IS NULL OR shop_id = $7) ORDER BY cart_id ASC")
            .bind(user_id)
            .bind(product_id)
            .bind(cart_type)
            .bind(sku_id)
            .bind(sku_id)
            .bind(shop_id)
            .bind(shop_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 同商品「其它 SKU」在购物车中的数量合计（Java
    /// `CartWholesaleEnhancer#computeAddMixQtyTotal` 的 `others` 部分）。
    ///
    /// 对应 Java `qw.eq(userId).eq(productId).eq(type).eq(shopId)
    /// .notIn(skuId, 本批 skuIds)`：`sku_id <> ?`（`NULL` 行同 Java
    /// `NOT IN` 语义一并排除）。无行时返回 `0`。
    pub async fn sum_other_sku_quantity(
        &self,
        user_id: i32,
        product_id: i32,
        cart_type: i32,
        shop_id: i32,
        excluded_sku_id: i32,
    ) -> Result<i32> {
        let row: (i64,) = sqlx::query_as("SELECT CAST(COALESCE(SUM(quantity), 0) AS BIGINT) FROM cart WHERE user_id = $1 AND product_id = $2 AND \"type\" = $3 AND shop_id = $4 AND sku_id <> $5")
            .bind(user_id)
            .bind(product_id)
            .bind(cart_type)
            .bind(shop_id)
            .bind(excluded_sku_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(i32::try_from(row.0).unwrap_or(i32::MAX))
    }

    /// 删除同款旧行并插入合并后的新行（Java `CartServiceImpl#addProductToCart`：
    /// `removeByIds(matchedCarts)` + `saveBatch(addCarts)`，二者同处一个
    /// `@Transactional` 方法内）。
    ///
    /// 返回新行的 `cart_id`。
    pub async fn replace_cart_line(
        &self,
        user_id: i32,
        old_cart_ids: &[i32],
        line: &NewCartLine,
    ) -> Result<i32> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;

        for cart_id in old_cart_ids {
            sqlx::query("DELETE FROM cart WHERE cart_id = $1 AND user_id = $2")
                .bind(cart_id)
                .bind(user_id)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
        }

        let inserted: i32 = sqlx::query_scalar("INSERT INTO cart (user_id, product_id, product_sn, pic_thumb, market_price, original_price, quantity, sku_id, sku_data, product_type, is_checked, shop_id, \"type\", update_time, salesman_id, extra_sku_data, extra_data, is_wholesale_line, wholesale_grade_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 1, $11, 1, $12, 0, '', '{}', $13, $14) RETURNING cart_id")
            .bind(line.user_id)
            .bind(line.product_id)
            .bind(&line.product_sn)
            .bind(&line.pic_thumb)
            .bind(line.market_price)
            .bind(line.original_price)
            .bind(line.quantity)
            .bind(line.sku_id)
            .bind(&line.sku_data)
            .bind(line.product_type)
            .bind(line.shop_id)
            .bind(nextrade_common::utils::now_seconds() as i32)
            .bind(line.is_wholesale_line)
            .bind(line.wholesale_grade_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_err)?;

        tx.commit().await.map_err(map_err)?;
        Ok(inserted)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("批发相关记录不存在".into()),
        other => Error::Database(other.to_string()),
    }
}