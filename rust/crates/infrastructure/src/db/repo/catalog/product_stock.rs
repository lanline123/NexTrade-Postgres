//! 商品库存管理域（catalog::product_stock）仓储。
//!
//! 表：`product` / `product_sku` / `store_product` / `store_product_sku` /
//! `product_stock_record` / `product_stock_record_detail` /
//! `product_stock_adjust_record` / `product_inventory_log` /
//! `product_stock_allocate_record` / `product_stock_allocate_record_detail` /
//! `product_stock_allocate_operation_record` / `shop` / `e_card` / `e_card_group`。
//!
//! 列名严格对齐 `java/database/pure.sql`
//! （用 `python tools/schema_parser.py --table <表名>` 复核）：
//! - `product`：`product_id` / `product_name` / `product_sn` / `product_tsn` /
//!   `product_stock` / `product_unit` / `pic_thumb` / `product_type` / `cost_price` /
//!   `card_group_id` / `shop_id` / `vendor_id` / `is_delete` / `add_time` / `product_status`
//! - `product_sku`：`sku_id` / `product_id` / `sku_value` / `sku_stock` / `cost_price`
//! - `store_product`：`store_product_id` / `product_id` / `product_stock` / `shop_id` /
//!   `is_delete` / `add_time`
//! - `store_product_sku`：`store_product_sku_id` / `store_product_id` / `product_id` /
//!   `product_sku_id` / `sku_stock`
//! - `product_stock_record`：`product_stock_id` / `stock_code` / `in_or_out` / `type` /
//!   `add_time` / `add_by` / `comment` / `shop_id`
//! - `product_stock_record_detail`：`record_detail_id` / `stock_record_id` / `product_id` /
//!   `sku_id` / `quantity`
//! - `product_stock_adjust_record`：`adjust_record_id` / `sku_id` / `adjust_type` /
//!   `quantity` / `add_by` / `add_time` / `product_id` / `shop_id` /
//!   `change_before_quantity` / `change_after_quantity`
//! - `product_inventory_log`：`log_id` / `product_id` / `spec_id` / `number` / `add_time` /
//!   `old_number` / `type` / `change_number` / `desc` / `shop_id`
//! - `product_stock_allocate_record`：`allocate_id` / `allocate_code` /
//!   `outbound_organization_id` / `warehousing_organization_id` / `status` / `add_by` /
//!   `add_time` / `stock_in_comment` / `stock_out_comment` / `stock_allocate_comment`
//! - `product_stock_allocate_record_detail`：`allocate_detail_id` / `allocate_id` /
//!   `product_id` / `sku_id` / `quantity` / `stock_in_quantity` / `stock_out_quantity`
//! - `product_stock_allocate_operation_record`：`allocate_operation_id` / `allocate_id` /
//!   `operation_type` / `add_time` / `review_rejected_reason` / `add_by`
//!
//! 保留字注意：`product_stock_record.comment`、`product_inventory_log.desc`、
//! `product_stock_record.type` 是 MySQL 保留字，原始 SQL 必须反引号包裹。
//!
//! 查询统一使用运行时 `sqlx::query_as`（不使用 `query!` 宏，本机无 DATABASE_URL）。
//! 库存扣减一律 `UPDATE ... WHERE <stock 列> >= ?`，保证原子防超卖。

use std::collections::HashMap;

use sqlx::PgPool;
use nextrade_core::entities::{
    Product, ProductInventoryLog, ProductSku, StoreProduct, StoreProductSku,
    ProductStockAdjustRecord, ProductStockAllocateOperationRecord, ProductStockAllocateRecord,
    ProductStockAllocateRecordDetail, ProductStockRecord, ProductStockRecordDetail,
};
use nextrade_core::models::catalog::product_stock::{
    AllocateDetailItem, AllocateExportRow, AllocatePageQuery, ProductAllocateDetailVO, StockPageQuery,
    StockPageRow, StockRecordPageQuery,
};
use nextrade_core::models::catalog::product_stock::SHOP_TYPE_STORE;
use nextrade_core::{Error, Result};

/// `product` 表全部列（顺序与 [`Product`] 实体字段一致）。
const PRODUCT_COLUMNS: &str = "product_id, product_name, seo_identification, product_sn, product_tsn, product_stock, product_price, market_price, shipping_tpl_id, product_status, product_type, delivery_mode, sale_mode, category_id, brand_id, shop_id, keywords, shop_category_id, check_status, check_reason, click_count, product_weight, is_promote, is_promote_activity, promote_price, promote_start_date, promote_end_date, seckill_max_num, product_brief, product_desc, pic_url, pic_thumb, pic_original, pic_large, comment_tag, free_shipping, integral, add_time, sort_order, store_sort_order, is_delete, is_best, is_new, is_hot, last_update, remark, give_integral, rank_integral, suppliers_id, virtual_sales, limit_number, product_care, product_related, product_service_ids, is_support_return, is_support_cod, product_video, prepay_price, card_group_id, virtual_sample, paid_content, no_shipping, fixed_shipping_type, fixed_shipping_fee, vendor_product_id, vendor_id, shop_pickup_tpl_id, is_shop_pickup, is_logistics, is_shop_delivery, reservation_tpl_id, cost_price, product_unit, is_need_re_import_vendor_product, gift_card_groups_id, attributes_tpl_id, share_info, delivery_service_fee_type, delivery_service_fee_value, server_service_fee_type, server_service_fee_value";

/// `product_sku` 表全部列（顺序与 [`ProductSku`] 实体字段一致）。
const SKU_COLUMNS: &str = "sku_id, product_id, sku_value, sku_data, sku_sn, sku_stock, sku_tsn, sku_price, is_enabled, market_price, cost_price, delivery_service_fee_type, delivery_service_fee_value, server_service_fee_type, server_service_fee_value, vendor_product_sku_id";

/// `product_stock_record` 表全部列（顺序与 [`ProductStockRecord`] 实体一致）。
const STOCK_RECORD_COLUMNS: &str =
    "product_stock_id, stock_code, in_or_out, \"type\", add_time, add_by, \"comment\", shop_id";

/// `product_stock_record_detail` 表全部列。
const STOCK_RECORD_DETAIL_COLUMNS: &str =
    "record_detail_id, stock_record_id, product_id, sku_id, quantity";

/// `product_stock_adjust_record` 表全部列。
const ADJUST_COLUMNS: &str = "adjust_record_id, sku_id, adjust_type, quantity, add_by, add_time, product_id, shop_id, change_before_quantity, change_after_quantity";

/// `product_stock_allocate_record` 表全部列。
const ALLOCATE_COLUMNS: &str = "allocate_id, allocate_code, outbound_organization_id, warehousing_organization_id, status, add_by, add_time, stock_in_comment, stock_out_comment, stock_allocate_comment";

/// `product_stock_allocate_record_detail` 表全部列。
const ALLOCATE_DETAIL_COLUMNS: &str =
    "allocate_detail_id, allocate_id, product_id, sku_id, quantity, stock_in_quantity, stock_out_quantity";

/// `product_stock_allocate_operation_record` 表全部列。
const ALLOCATE_OP_COLUMNS: &str =
    "allocate_operation_id, allocate_id, operation_type, add_time, review_rejected_reason, add_by";

/// `store_product` 表全部列（顺序与 [`StoreProduct`] 实体一致）。
const STORE_PRODUCT_COLUMNS: &str = "store_product_id, product_id, product_name, product_price, product_status, product_stock, shop_id, add_time, card_group_id, is_delete, shop_category_id, reservation_tpl_id, delivery_service_fee_type, delivery_service_fee_value, server_service_fee_type, server_service_fee_value, pic_url, product_desc";

/// `store_product_sku` 表全部列（顺序与 [`StoreProductSku`] 实体一致）。
const STORE_SKU_COLUMNS: &str = "store_product_sku_id, product_id, product_sku_id, sku_price, sku_stock, add_time, store_product_id, delivery_service_fee_type, delivery_service_fee_value, server_service_fee_type, server_service_fee_value";

/// `product_inventory_log` 表全部列（`desc` 为保留字）。
const INVENTORY_LOG_COLUMNS: &str =
    "log_id, product_id, spec_id, number, add_time, old_number, type, change_number, \"desc\", shop_id";

/// 待绑定参数（按出现顺序 1:1 对应）。
#[derive(Debug, Clone)]
enum Bind {
    I32(i32),
    I64(i64),
    Str(String),
}

/// 把 `Bind` 列表按顺序绑定到查询上（宏展开以适配不同返回行类型）。
macro_rules! bind_all {
    ($q:expr, $binds:expr) => {{
        let mut q = $q;
        for b in $binds.iter() {
            q = match b {
                Bind::I32(v) => q.bind(*v),
                Bind::I64(v) => q.bind(*v),
                Bind::Str(v) => q.bind(v.as_str()),
            };
        }
        q
    }};
}

#[derive(Clone)]
pub struct ProductStockRepository {
    pool: PgPool,
}

impl ProductStockRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（服务层事务编排用）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 商品 / SKU —————————————————————

    /// 按商品 ID 查询（Java `productService.getById`）。
    pub async fn find_product(&self, product_id: i32) -> Result<Option<Product>> {
        let sql = format!("SELECT {PRODUCT_COLUMNS} FROM product WHERE product_id = $1 LIMIT 1");
        sqlx::query_as::<_, Product>(&sql)
            .bind(product_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按商品编码查询（Java `productService.getOne(eq(productSn))`）。
    ///
    /// `with_shop` 为 `Some(shop_id)` 时额外按 `shop_id` 过滤
    /// （对齐 `checkImportData` 中带 `HeaderUtils.getShopId()` 的查询）。
    pub async fn find_product_by_sn(
        &self,
        product_sn: &str,
        with_shop: Option<i32>,
    ) -> Result<Option<Product>> {
        let id: Option<(i32,)> = match with_shop {
            Some(shop_id) => sqlx::query_as(
                "SELECT product_id FROM product WHERE product_sn = $1 AND shop_id = $2 LIMIT 1",
            )
            .bind(product_sn)
            .bind(shop_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?,
            None => sqlx::query_as("SELECT product_id FROM product WHERE product_sn = $1 LIMIT 1")
                .bind(product_sn)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?,
        };
        match id {
            Some((pid,)) => self.find_product(pid).await,
            None => Ok(None),
        }
    }

    /// 按 SKU ID 查询（Java `productSkuService.getById`）。
    pub async fn find_sku(&self, sku_id: i32) -> Result<Option<ProductSku>> {
        let sql = format!("SELECT {SKU_COLUMNS} FROM product_sku WHERE sku_id = $1 LIMIT 1");
        sqlx::query_as::<_, ProductSku>(&sql)
            .bind(sku_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按「商品 + 规格值」查询 SKU（Java
    /// `productSkuService.getOne(eq(productId).eq(skuValue))`）。
    pub async fn find_sku_by_value(
        &self,
        product_id: i32,
        sku_value: &str,
    ) -> Result<Option<ProductSku>> {
        let sql = format!(
            "SELECT {SKU_COLUMNS} FROM product_sku WHERE product_id = $1 AND sku_value = $2 LIMIT 1"
        );
        sqlx::query_as::<_, ProductSku>(&sql)
            .bind(product_id)
            .bind(sku_value)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 商品下的 SKU 数量（Java `productSkuService.count(eq(productId))`）。
    pub async fn count_skus(&self, product_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM product_sku WHERE product_id = $1")
            .bind(product_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0)
    }

    /// 批量取商品名 / 单位 / 缩略图（`getRecordDetail` / `stockDetail` 翻译用）。
    pub async fn find_product_briefs(
        &self,
        product_ids: &[i32],
    ) -> Result<HashMap<i32, (String, Option<String>, String)>> {
        let mut out: HashMap<i32, (String, Option<String>, String)> = HashMap::new();
        for id in product_ids {
            if out.contains_key(id) {
                continue;
            }
            let row: Option<(String, Option<String>, String)> = sqlx::query_as(
                "SELECT product_name, product_unit, pic_thumb FROM product WHERE product_id = $1 LIMIT 1",
            )
            .bind(*id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?;
            if let Some(r) = row {
                out.insert(*id, r);
            }
        }
        Ok(out)
    }

    /// 批量取 SKU 规格值。
    pub async fn find_sku_values(&self, sku_ids: &[i32]) -> Result<HashMap<i32, String>> {
        let mut out: HashMap<i32, String> = HashMap::new();
        for id in sku_ids {
            if out.contains_key(id) {
                continue;
            }
            let row: Option<(String,)> =
                sqlx::query_as("SELECT sku_value FROM product_sku WHERE sku_id = $1 LIMIT 1")
                    .bind(*id)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(map_err)?;
            if let Some(r) = row {
                out.insert(*id, r.0);
            }
        }
        Ok(out)
    }

    // ————————————————————— 库存分页（ProductSkuMapper.getProductStockPage）—————————————

    /// 库存分页（对应 Java `ProductSkuMapper.getProductStockPage`）。
    ///
    /// SQL 与 Java 完全同构：
    /// `product p LEFT JOIN product_sku ps`，
    /// `p.shop_id = ? AND p.vendor_id IS NULL`，可选 `p.product_type` 与关键词
    /// （商品名 / 规格值 模糊，规格条码 / 商品条码 精确），`ORDER BY p.add_time`。
    pub async fn paginate_stock_page(
        &self,
        q: &StockPageQuery,
        shop_id: i32,
    ) -> Result<(u64, Vec<StockPageRow>)> {
        let (where_sql, binds) = build_stock_page_filter(q, shop_id);
        let page = q.page_query();

        let count_sql =
            crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM product p LEFT JOIN product_sku ps ON ps.product_id = p.product_id{where_sql}"));
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&count_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT p.product_id, p.product_name, p.product_stock, p.product_unit, \
                    p.pic_thumb AS product_pic_thumb, p.product_sn, p.product_tsn, \
                    p.product_type, p.cost_price AS product_cost, p.card_group_id, \
                    ps.sku_id, ps.sku_value AS product_sku_name, \
                    ps.sku_stock AS product_sku_stock, ps.cost_price AS product_sku_cost \
               FROM product p LEFT JOIN product_sku ps ON ps.product_id = p.product_id\
               {where_sql} ORDER BY p.add_time LIMIT ? OFFSET ?"
        ));
        let rows = bind_all!(sqlx::query_as::<_, StockPageRow>(&rows_sql), binds)
            .bind(page.page_size as i64)
            .bind(((page.page - 1) * page.page_size) as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 卡券分组是否有效（Java `eCardGroupMapper.selectOne(eq(groupId).eq(isUse, 1))`）。
    pub async fn find_card_group(&self, group_id: i32) -> Result<Option<i32>> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT group_id FROM e_card_group WHERE group_id = $1 AND is_use = 1 LIMIT 1")
                .bind(group_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|r| r.0))
    }

    /// 分组下未使用卡券数量（Java `eCardService.count(eq(groupId).eq(isUse, 0))`）。
    pub async fn count_unused_cards(&self, group_id: i32) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM e_card WHERE group_id = $1 AND is_use = 0")
                .bind(group_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0)
    }

    // ————————————————————— 出入库明细（product_inventory_log）—————————————

    /// 出入库明细分页（Java `ProductInventoryLogServiceImpl#getDetail`）。
    ///
    /// 过滤条件：`product_id = ? AND shop_id = ? AND spec_id = ?`
    /// （Java 中 `skuId == null ? 0 : skuId`），按 `add_time DESC`。
    pub async fn paginate_inventory_log(
        &self,
        product_id: i32,
        sku_id: Option<i32>,
        shop_id: i32,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<ProductInventoryLog>)> {
        let spec_id = sku_id.unwrap_or(0);
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM product_inventory_log \
              WHERE product_id = $1 AND shop_id = $2 AND spec_id = $3",
        )
        .bind(product_id)
        .bind(shop_id)
        .bind(spec_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {INVENTORY_LOG_COLUMNS} FROM product_inventory_log \
              WHERE product_id = ? AND shop_id = ? AND spec_id = ? \
              ORDER BY add_time DESC LIMIT ? OFFSET ?"
        ));
        let rows = sqlx::query_as::<_, ProductInventoryLog>(&sql)
            .bind(product_id)
            .bind(shop_id)
            .bind(spec_id)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 写入库存日志（Java `productInventoryLogService.save` / \"insert\"）。
    pub async fn insert_inventory_log(&self, log: &ProductInventoryLog) -> Result<()> {
        sqlx::query(
            "INSERT INTO product_inventory_log \
               (product_id, spec_id, number, add_time, old_number, type, change_number, \"desc\", shop_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(log.product_id)
        .bind(log.spec_id)
        .bind(log.number)
        .bind(log.add_time)
        .bind(log.old_number)
        .bind(log.r#type)
        .bind(log.change_number)
        .bind(&log.desc)
        .bind(log.shop_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    // ————————————————————— 出入库记录 —————————————————————

    /// 出入库记录分页（对应 `ProductStockRecordMapper.productStockRecordPage`）。
    ///
    /// Java：`LEFT JOIN` 明细 / 商品 / 规格，`GROUP BY pr.product_stock_id`，
    /// `ORDER BY pr.add_time DESC`，关键词匹配库存编码 / 商品名 / 商品编码 / 规格值。
    #[allow(clippy::too_many_arguments)]
    pub async fn paginate_stock_records(
        &self,
        q: &StockRecordPageQuery,
        shop_id: i32,
        begin_time: Option<i64>,
        end_time: Option<i64>,
    ) -> Result<(u64, Vec<ProductStockRecord>)> {
        let (where_sql, binds) = build_record_filter(q, shop_id, begin_time, end_time);
        let page = q.page_query();

        let from_sql = " FROM product_stock_record pr \
             LEFT JOIN product_stock_record_detail psrd ON psrd.stock_record_id = pr.product_stock_id \
             LEFT JOIN product p ON p.product_id = psrd.product_id \
             LEFT JOIN product_sku ps ON ps.sku_id = psrd.sku_id";

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(DISTINCT pr.product_stock_id){from_sql}{where_sql}"));
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&count_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT pr.product_stock_id, pr.stock_code, pr.in_or_out, pr.\"type\", pr.add_time, \
                    pr.add_by, pr.\"comment\", pr.shop_id{from_sql}{where_sql} \
              GROUP BY pr.product_stock_id ORDER BY pr.add_time DESC LIMIT ? OFFSET ?"
        ));
        let rows = bind_all!(sqlx::query_as::<_, ProductStockRecord>(&rows_sql), binds)
            .bind(page.page_size as i64)
            .bind(((page.page - 1) * page.page_size) as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 单条出入库记录（Java `this.getById(id)`）。
    pub async fn find_stock_record(&self, id: i32) -> Result<Option<ProductStockRecord>> {
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {STOCK_RECORD_COLUMNS} FROM product_stock_record \
              WHERE product_stock_id = ? LIMIT 1"
        ));
        sqlx::query_as::<_, ProductStockRecord>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 记录的明细行（Java `list(eq(stockRecordId, id))`）。
    pub async fn find_stock_record_details(
        &self,
        stock_record_id: i32,
    ) -> Result<Vec<ProductStockRecordDetail>> {
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {STOCK_RECORD_DETAIL_COLUMNS} FROM product_stock_record_detail \
              WHERE stock_record_id = ?"
        ));
        sqlx::query_as::<_, ProductStockRecordDetail>(&sql)
            .bind(stock_record_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 写入出入库记录，返回自增主键。
    pub async fn insert_stock_record(&self, r: &ProductStockRecord) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO product_stock_record \
               (stock_code, in_or_out, \"type\", add_time, add_by, \"comment\", shop_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING product_stock_id",
        )
        .bind(&r.stock_code)
        .bind(r.in_or_out)
        .bind(r.r#type)
        .bind(r.add_time)
        .bind(&r.add_by)
        .bind(&r.comment)
        .bind(r.shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 批量写入出入库明细（Java \"saveBatch\"）。
    pub async fn insert_stock_record_details(
        &self,
        details: &[ProductStockRecordDetail],
    ) -> Result<bool> {
        if details.is_empty() {
            return Ok(true);
        }
        for d in details {
            sqlx::query(
                "INSERT INTO product_stock_record_detail \
                   (stock_record_id, product_id, sku_id, quantity) VALUES ($1, $2, $3, $4)",
            )
            .bind(d.stock_record_id)
            .bind(d.product_id)
            .bind(d.sku_id)
            .bind(d.quantity)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        }
        Ok(true)
    }

    // ————————————————————— 库存增减（原子防超卖）—————————————————————

    /// 商品库存增加（Java `productService.incStock` 的 UPDATE 部分）。
    pub async fn inc_product_stock(&self, product_id: i32, quantity: i32) -> Result<u64> {
        let res =
            sqlx::query("UPDATE product SET product_stock = product_stock + $1 WHERE product_id = $2")
                .bind(quantity)
                .bind(product_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 商品库存扣减：**原子条件更新**（`WHERE product_stock >= ?`），
    /// 返回受影响行数（0 = 库存不足）。
    pub async fn dec_product_stock(&self, product_id: i32, quantity: i32) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE product SET product_stock = product_stock - $1 \
              WHERE product_id = $2 AND product_stock >= $3",
        )
        .bind(quantity)
        .bind(product_id)
        .bind(quantity)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// SKU 库存增加（Java `productSkuService.incStock`）。
    pub async fn inc_sku_stock(&self, sku_id: i32, quantity: i32) -> Result<u64> {
        let res = sqlx::query("UPDATE product_sku SET sku_stock = sku_stock + $1 WHERE sku_id = $2")
            .bind(quantity)
            .bind(sku_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// SKU 库存扣减：**原子条件更新**（`WHERE sku_stock >= ?`）。
    pub async fn dec_sku_stock(&self, sku_id: i32, quantity: i32) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE product_sku SET sku_stock = sku_stock - $1 \
              WHERE sku_id = $2 AND sku_stock >= $3",
        )
        .bind(quantity)
        .bind(sku_id)
        .bind(quantity)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— 门店分配商品（O2O）—————————————————————

    /// 查询平台商品分配给某门店的记录（Java `storeProductMapper.selectOne`：
    /// \"product_id\" + \"shop_id\" + `is_delete = 0`）。
    pub async fn find_store_product(
        &self,
        product_id: i32,
        shop_id: i32,
    ) -> Result<Option<StoreProduct>> {
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {STORE_PRODUCT_COLUMNS} FROM store_product \
              WHERE product_id = ? AND shop_id = ? AND is_delete = 0 LIMIT 1"
        ));
        sqlx::query_as::<_, StoreProduct>(&sql)
            .bind(product_id)
            .bind(shop_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 查询门店商品规格（Java `storeSkuMapper.selectOne`）。
    pub async fn find_store_product_sku(
        &self,
        store_product_id: i64,
        product_sku_id: i32,
    ) -> Result<Option<StoreProductSku>> {
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {STORE_SKU_COLUMNS} FROM store_product_sku \
              WHERE store_product_id = ? AND product_sku_id = ? LIMIT 1"
        ));
        sqlx::query_as::<_, StoreProductSku>(&sql)
            .bind(store_product_id)
            .bind(product_sku_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 门店商品规格数量（`checkAllocateImportData` 判断门店商品是否多规格）。
    pub async fn count_store_skus(&self, store_product_id: i64) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM store_product_sku WHERE store_product_id = $1")
                .bind(store_product_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0)
    }

    /// 门店商品库存增加。
    pub async fn inc_store_product_stock(
        &self,
        store_product_id: i64,
        quantity: i32,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE store_product SET product_stock = product_stock + $1 \
              WHERE store_product_id = $2 AND is_delete = 0",
        )
        .bind(quantity)
        .bind(store_product_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 门店商品库存扣减：**原子条件更新**（`WHERE product_stock >= ?`）。
    pub async fn dec_store_product_stock(
        &self,
        store_product_id: i64,
        quantity: i32,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE store_product SET product_stock = product_stock - $1 \
              WHERE store_product_id = $2 AND is_delete = 0 AND product_stock >= $3",
        )
        .bind(quantity)
        .bind(store_product_id)
        .bind(quantity)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 门店商品规格库存增加。
    pub async fn inc_store_sku_stock(
        &self,
        store_product_sku_id: i64,
        quantity: i32,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE store_product_sku SET sku_stock = sku_stock + $1 \
              WHERE store_product_sku_id = $2",
        )
        .bind(quantity)
        .bind(store_product_sku_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 门店商品规格库存扣减：**原子条件更新**（`WHERE sku_stock >= ?`）。
    pub async fn dec_store_sku_stock(
        &self,
        store_product_sku_id: i64,
        quantity: i32,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE store_product_sku SET sku_stock = sku_stock - $1 \
              WHERE store_product_sku_id = $2 AND sku_stock >= $3",
        )
        .bind(quantity)
        .bind(store_product_sku_id)
        .bind(quantity)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— 库存调整 —————————————————————

    /// 写入库存调整记录，返回自增主键。
    pub async fn insert_adjust_record(&self, r: &ProductStockAdjustRecord) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO product_stock_adjust_record \
               (sku_id, adjust_type, quantity, add_by, add_time, product_id, shop_id, \
                change_before_quantity, change_after_quantity) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING adjust_record_id",
        )
        .bind(r.sku_id)
        .bind(r.adjust_type)
        .bind(r.quantity)
        .bind(&r.add_by)
        .bind(r.add_time)
        .bind(r.product_id)
        .bind(r.shop_id)
        .bind(r.change_before_quantity)
        .bind(r.change_after_quantity)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 调整记录列表（Java \"adjustDetail\"：`eq(product_id).eq(sku_id$2).eq(shop_id)`）。
    pub async fn find_adjust_records(
        &self,
        product_id: i32,
        sku_id: Option<i32>,
        shop_id: i32,
    ) -> Result<Vec<ProductStockAdjustRecord>> {
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {ADJUST_COLUMNS} FROM product_stock_adjust_record WHERE product_id = ? AND shop_id = ?{} ORDER BY adjust_record_id DESC",
            if sku_id.is_some() { " AND sku_id = ?" } else { "" }
        ));

        let mut q = sqlx::query_as::<_, ProductStockAdjustRecord>(&sql)
            .bind(product_id)
            .bind(shop_id);
        if let Some(sku) = sku_id {
            q = q.bind(sku);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 调拨单 —————————————————————

    /// 调拨单分页（对应 `ProductStockAllocateRecordMapper.getPage`）。
    ///
    /// WHERE 与 Java 同构：
    /// `(outbound_organization_id = ? OR warehousing_organization_id = ?)`
    /// + 可选 `allocate_code LIKE` / `status =` / `add_by =`，`ORDER BY add_time DESC`。
    pub async fn paginate_allocate_records(
        &self,
        q: &AllocatePageQuery,
        shop_id: i32,
    ) -> Result<(u64, Vec<ProductStockAllocateRecord>)> {
        let mut conds: Vec<String> =
            vec!["(outbound_organization_id = ? OR warehousing_organization_id = ?)".into()];
        let mut binds: Vec<Bind> = vec![Bind::I32(shop_id), Bind::I32(shop_id)];

        if let Some(kw) = non_empty(&q.keyword) {
            conds.push("allocate_code LIKE ?".into());
            binds.push(Bind::Str(format!("%{kw}%")));
        }
        if let Some(s) = q.status {
            conds.push("status = ?".into());
            binds.push(Bind::I32(s));
        }
        if let Some(by) = non_empty(&q.add_by) {
            conds.push("add_by = ?".into());
            binds.push(Bind::Str(by.to_string()));
        }

        let where_sql = format!(" WHERE {}", conds.join(" AND "));
        let page = q.page_query();

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM product_stock_allocate_record{where_sql}"));
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&count_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {ALLOCATE_COLUMNS} FROM product_stock_allocate_record{where_sql} \
              ORDER BY add_time DESC LIMIT ? OFFSET ?"
        ));
        let rows = bind_all!(sqlx::query_as::<_, ProductStockAllocateRecord>(&rows_sql), binds)
            .bind(page.page_size as i64)
            .bind(((page.page - 1) * page.page_size) as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 单条调拨单（Java `this.getById(id)`）。
    pub async fn find_allocate_record(&self, id: i32) -> Result<Option<ProductStockAllocateRecord>> {
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {ALLOCATE_COLUMNS} FROM product_stock_allocate_record \
              WHERE allocate_id = ? LIMIT 1"
        ));
        sqlx::query_as::<_, ProductStockAllocateRecord>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 调拨明细 + 商品信息（Java `productStockAllocateRecordDetail`）。
    pub async fn find_allocate_details_with_product(
        &self,
        allocate_id: i32,
    ) -> Result<Vec<ProductAllocateDetailVO>> {
        let rows = sqlx::query_as::<
            _,
            (
                i32,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
            ),
        >(
            "SELECT d.allocate_detail_id, d.allocate_id, d.product_id, d.sku_id, d.quantity, \
                    d.stock_in_quantity, d.stock_out_quantity, p.product_name, ps.sku_value, \
                    p.product_unit, p.pic_thumb \
               FROM product_stock_allocate_record_detail d \
               LEFT JOIN product p ON p.product_id = d.product_id \
               LEFT JOIN product_sku ps ON ps.sku_id = d.sku_id \
              WHERE d.allocate_id = $1",
        )
        .bind(allocate_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    allocate_detail_id,
                    allocate_id,
                    product_id,
                    sku_id,
                    quantity,
                    stock_in_quantity,
                    stock_out_quantity,
                    product_name,
                    sku_name,
                    product_unit,
                    product_pic_thumb,
                )| ProductAllocateDetailVO {
                    allocate_detail_id: Some(allocate_detail_id),
                    allocate_id,
                    product_id,
                    sku_id,
                    quantity,
                    stock_in_quantity,
                    stock_out_quantity,
                    product_name,
                    sku_name,
                    product_unit,
                    product_pic_thumb,
                },
            )
            .collect())
    }

    /// 调拨明细实体行（调出/调入数量更新校验用）。
    pub async fn find_allocate_detail_rows(
        &self,
        allocate_id: i32,
    ) -> Result<Vec<ProductStockAllocateRecordDetail>> {
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {ALLOCATE_DETAIL_COLUMNS} FROM product_stock_allocate_record_detail \
              WHERE allocate_id = ?"
        ));
        sqlx::query_as::<_, ProductStockAllocateRecordDetail>(&sql)
            .bind(allocate_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 更新调拨明细的实际出入库数量
    /// （Java `productStockAllocateRecordDetailService.updateBatchById`）。
    pub async fn update_allocate_detail_quantities(
        &self,
        detail: &AllocateDetailItem,
    ) -> Result<u64> {
        let Some(detail_id) = detail.allocate_detail_id else {
            return Ok(0);
        };
        let res = sqlx::query(
            "UPDATE product_stock_allocate_record_detail \
                SET stock_out_quantity = $1, stock_in_quantity = $2 \
              WHERE allocate_detail_id = $3",
        )
        .bind(detail.stock_out_quantity.unwrap_or(0))
        .bind(detail.stock_in_quantity.unwrap_or(0))
        .bind(detail_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 新建调拨单，返回自增主键。
    pub async fn insert_allocate_record(&self, r: &ProductStockAllocateRecord) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO product_stock_allocate_record \
               (allocate_code, outbound_organization_id, warehousing_organization_id, status, \
                add_by, add_time, stock_in_comment, stock_out_comment, stock_allocate_comment) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING allocate_id",
        )
        .bind(&r.allocate_code)
        .bind(r.outbound_organization_id)
        .bind(r.warehousing_organization_id)
        .bind(r.status)
        .bind(&r.add_by)
        .bind(r.add_time)
        .bind(&r.stock_in_comment)
        .bind(&r.stock_out_comment)
        .bind(&r.stock_allocate_comment)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 批量写入调拨明细（Java \"saveBatch\"；
    /// \"allocate_id\" 由入参指定，\"stock_out_quantity\" 被置 0）。
    pub async fn insert_allocate_details(
        &self,
        allocate_id: i32,
        details: &[AllocateDetailItem],
    ) -> Result<bool> {
        if details.is_empty() {
            return Ok(false);
        }
        for d in details {
            sqlx::query(
                "INSERT INTO product_stock_allocate_record_detail \
                   (allocate_id, product_id, sku_id, quantity, stock_in_quantity, stock_out_quantity) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(allocate_id)
            .bind(d.product_id)
            .bind(d.sku_id)
            .bind(d.quantity)
            .bind(0_i32)
            .bind(0_i32)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        }
        Ok(true)
    }

    /// 更新调拨单状态（Java \"updateById\"）。
    ///
    /// \"stock_out_comment\" / \"stock_in_comment\" 仅在调用方显式给出时更新，
    /// 对齐 Java「在实体上赋值再 updateById」的字段合并语义。
    pub async fn update_allocate_status(
        &self,
        allocate_id: i32,
        status: i32,
        stock_out_comment: Option<&str>,
        stock_in_comment: Option<&str>,
    ) -> Result<u64> {
        let mut sets: Vec<&str> = vec!["status = ?"];
        if stock_out_comment.is_some() {
            sets.push("stock_out_comment = ?");
        }
        if stock_in_comment.is_some() {
            sets.push("stock_in_comment = ?");
        }
        let sql = crate::db::to_pg_placeholders(&format!(
            "UPDATE product_stock_allocate_record SET {} WHERE allocate_id = ?",
            sets.join(", ")
        ));
        let mut q = sqlx::query(&sql).bind(status);
        if let Some(c) = stock_out_comment {
            q = q.bind(c);
        }
        if let Some(c) = stock_in_comment {
            q = q.bind(c);
        }
        let res = q
            .bind(allocate_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 写入调拨操作记录（Java `productStockAllocateOperationRecordService.save`）。
    pub async fn insert_allocate_operation(
        &self,
        allocate_id: i32,
        operation_type: &str,
        review_rejected_reason: Option<&str>,
        add_by: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO product_stock_allocate_operation_record \
               (allocate_id, operation_type, add_time, review_rejected_reason, add_by) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(allocate_id)
        .bind(operation_type)
        .bind(nextrade_common::utils::now_seconds())
        .bind(review_rejected_reason)
        .bind(add_by)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 调拨操作记录列表（Java `list(eq(allocateId))`）。
    pub async fn find_allocate_operations(
        &self,
        allocate_id: i32,
    ) -> Result<Vec<ProductStockAllocateOperationRecord>> {
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {ALLOCATE_OP_COLUMNS} FROM product_stock_allocate_operation_record \
              WHERE allocate_id = ? ORDER BY allocate_operation_id ASC"
        ));
        sqlx::query_as::<_, ProductStockAllocateOperationRecord>(&sql)
            .bind(allocate_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 调拨单经办人去重集合（Java `getAddBy`，保持首次出现顺序）。
    pub async fn find_distinct_add_by(&self) -> Result<Vec<String>> {
        let rows: Vec<(Option<String>,)> =
            sqlx::query_as("SELECT add_by FROM product_stock_allocate_record GROUP BY add_by")
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        let mut out: Vec<String> = Vec::new();
        for (v,) in rows {
            if let Some(s) = v.filter(|s| !s.is_empty()) {
                if !out.contains(&s) {
                    out.push(s);
                }
            }
        }
        Ok(out)
    }

    /// 导出用调拨单列表（对应 \"getListByProductIds\"）。
    pub async fn find_allocate_records_for_export(
        &self,
        shop_id: i32,
        allocate_ids: Option<&[i32]>,
    ) -> Result<Vec<AllocateExportRow>> {
        let mut sql = String::from(
            "SELECT allocate_code, outbound_organization_id, warehousing_organization_id, \
                    status, add_by, add_time \
               FROM product_stock_allocate_record \
              WHERE (warehousing_organization_id = $1 OR outbound_organization_id = $2)",
        );
        let ids = allocate_ids.filter(|v| !v.is_empty());
        if let Some(list) = ids {
            let ph = vec!["?"; list.len()].join(", ");
            sql.push_str(&format!(" AND allocate_id IN ({ph})"));
        }
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, AllocateExportRow>(&sql)
            .bind(shop_id)
            .bind(shop_id);
        if let Some(list) = ids {
            for id in list {
                q = q.bind(*id);
            }
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 调拨选品（StoreProductMapper）—————————————————————

    /// 场景1：平台 → 门店（平台商品中已分配给目标门店的商品）。
    pub async fn paginate_allocate_platform_to_store(
        &self,
        target_shop_id: i32,
        keyword: Option<&str>,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<StockPageRow>)> {
        let (from_sql, where_sql, binds) = allocate_select_sql(
            "FROM product p \
               LEFT JOIN product_sku ps ON ps.product_id = p.product_id \
               INNER JOIN store_product sp ON sp.product_id = p.product_id \
                    AND sp.shop_id = $1 AND sp.is_delete = 0",
            "p.shop_id = 0 AND p.is_delete = 0 AND p.vendor_id IS NULL AND p.product_type = 1",
            keyword,
            target_shop_id,
        );
        self.run_allocate_page(&from_sql, &where_sql, binds, "p.add_time", limit, offset)
            .await
    }

    /// 场景2：门店 → 平台（出库门店已分配的商品）。
    pub async fn paginate_allocate_store_to_platform(
        &self,
        source_shop_id: i32,
        keyword: Option<&str>,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<StockPageRow>)> {
        let (from_sql, where_sql, binds) = allocate_select_sql(
            "FROM store_product sp \
               LEFT JOIN product p ON p.product_id = sp.product_id \
               LEFT JOIN store_product_sku sps ON sps.store_product_id = sp.store_product_id \
               LEFT JOIN product_sku ps ON ps.sku_id = sps.product_sku_id",
            "sp.shop_id = ? AND sp.is_delete = 0 AND p.product_id IS NOT NULL \
             AND p.is_delete = 0 AND p.product_type = 1",
            keyword,
            source_shop_id,
        );
        self.run_allocate_page(&from_sql, &where_sql, binds, "sp.add_time", limit, offset)
            .await
    }

    /// 场景3：门店 → 门店（两个门店都有分配的商品）。
    pub async fn paginate_allocate_store_to_store(
        &self,
        source_shop_id: i32,
        target_shop_id: i32,
        keyword: Option<&str>,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<StockPageRow>)> {
        let mut binds: Vec<Bind> = vec![Bind::I32(target_shop_id), Bind::I32(source_shop_id)];
        let mut conds: Vec<String> = vec![
            "sp_target.shop_id = ?".into(),
            "sp_target.is_delete = 0".into(),
            "sp_source.shop_id = ?".into(),
            "sp_source.is_delete = 0".into(),
            "p.product_id IS NOT NULL".into(),
            "p.is_delete = 0".into(),
            "p.product_type = 1".into(),
        ];
        if let Some(kw) = keyword.map(str::trim).filter(|s| !s.is_empty()) {
            conds.push("(p.product_name LIKE ? OR ps.sku_value LIKE ?)".into());
            let like = format!("%{kw}%");
            binds.push(Bind::Str(like.clone()));
            binds.push(Bind::Str(like));
        }
        let from_sql = "FROM store_product sp_source \
               INNER JOIN store_product sp_target ON sp_source.product_id = sp_target.product_id \
               LEFT JOIN product p ON p.product_id = sp_source.product_id \
               LEFT JOIN store_product_sku sps_source \
                      ON sps_source.store_product_id = sp_source.store_product_id \
               LEFT JOIN product_sku ps ON ps.sku_id = sps_source.product_sku_id";
        let where_sql = format!(" WHERE {}", conds.join(" AND "));
        self.run_allocate_page(
            from_sql,
            &where_sql,
            binds,
            "sp_source.add_time",
            limit,
            offset,
        )
        .await
    }

    /// 调拨选品分页执行（计数 + 明细）。
    async fn run_allocate_page(
        &self,
        from_sql: &str,
        where_sql: &str,
        binds: Vec<Bind>,
        order_col: &str,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<StockPageRow>)> {
        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) {from_sql}{where_sql}"));
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&count_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        // 列顺序必须与 [\"StockPageRow\"] 的字段顺序逐一对齐（\"FromRow\" 按位置取值）。
        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT p.product_id, p.product_name, p.product_stock, p.product_unit, \
                    p.pic_thumb AS product_pic_thumb, p.product_sn, p.product_tsn, \
                    p.product_type, p.cost_price AS product_cost, p.card_group_id, \
                    ps.sku_id AS product_sku_id, ps.sku_value AS product_sku_name, \
                    ps.sku_stock AS product_sku_stock, ps.cost_price AS product_sku_cost \
               {from_sql}{where_sql} ORDER BY {order_col} DESC LIMIT ? OFFSET ?"
        ));
        let rows = bind_all!(sqlx::query_as::<_, StockPageRow>(&rows_sql), binds)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    // ————————————————————— 组织 / 店铺 —————————————————————

    /// 组织名称（Java `resolveOrganizationName`：非 0 → `shop.shop_title`）。
    ///
    /// Java 对 `organizationId == 0` 返回配置项 `SHOP_NAME`（配置服务未迁移），
    /// 此处返回 `None`，由服务层按同一语义处理（见 `uncertain_points`）。
    pub async fn find_shop_title(&self, shop_id: i32) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT shop_title FROM shop WHERE shop_id = $1 LIMIT 1")
                .bind(shop_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }

    /// 可调拨组织（Java `getOrgList`：门店类型 + 开业 + 未挂父门店）。
    pub async fn find_allocate_orgs(&self) -> Result<Vec<(i32, Option<String>)>> {
        sqlx::query_as::<_, (i32, Option<String>)>(
            "SELECT shop_id, shop_title FROM shop \
              WHERE shop_type = $1 AND status = 1 AND store_parent_id IS NULL",
        )
        .bind(SHOP_TYPE_STORE)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按门店名查询开业门店（Java `getShopId`：
    /// `shop_title` + `status = 1` + `shop_type = 2`）。
    pub async fn find_shop_by_title(&self, title: &str) -> Result<Option<i32>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT shop_id FROM shop \
              WHERE shop_title = $1 AND status = 1 AND shop_type = $2 LIMIT 1",
        )
        .bind(title)
        .bind(SHOP_TYPE_STORE)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|r| r.0))
    }

    /// 批量组织名称（调拨单分页翻译）。
    pub async fn find_shop_titles(&self, shop_ids: &[i32]) -> Result<HashMap<i32, Option<String>>> {
        let mut out: HashMap<i32, Option<String>> = HashMap::new();
        for id in shop_ids {
            if out.contains_key(id) {
                continue;
            }
            out.insert(*id, self.find_shop_title(*id).await?);
        }
        Ok(out)
    }

    /// 商品所在门店 ID（库存调整记录落 \"shop_id\" 时用商品自身组织兜底）。
    pub async fn find_product_shop_id(&self, product_id: i32) -> Result<Option<i32>> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT shop_id FROM product WHERE product_id = $1 LIMIT 1")
                .bind(product_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|r| r.0))
    }
}

/// 调拨选品三场景共用的 FROM + WHERE 构造。
///
/// 返回 `(from_sql, where_sql, binds)`；\"shop_id\" 绑定在 \"from_sql\" 的 JOIN 条件里
/// （占位符出现在 WHERE 之前），因此绑定顺序为「组织 ID → 关键词」。
fn allocate_select_sql(
    from_sql: &str,
    base_where: &str,
    keyword: Option<&str>,
    shop_id: i32,
) -> (String, String, Vec<Bind>) {
    let mut conds: Vec<String> = base_where
        .split(" AND ")
        .map(|s| s.trim().to_string())
        .collect();
    let mut binds: Vec<Bind> = vec![Bind::I32(shop_id)];
    if let Some(kw) = keyword.map(str::trim).filter(|s| !s.is_empty()) {
        conds.push("(p.product_name LIKE ? OR ps.sku_value LIKE ?)".into());
        let like = format!("%{kw}%");
        binds.push(Bind::Str(like.clone()));
        binds.push(Bind::Str(like));
    }
    (
        from_sql.to_string(),
        format!(" WHERE {}", conds.join(" AND ")),
        binds,
    )
}

/// 库存分页 WHERE（对应 Java `ProductSkuMapper.getProductStockPage`）。
fn build_stock_page_filter(q: &StockPageQuery, shop_id: i32) -> (String, Vec<Bind>) {
    let mut conds: Vec<String> = vec!["p.shop_id = ?".into(), "p.vendor_id IS NULL".into()];
    let mut binds: Vec<Bind> = vec![Bind::I32(shop_id)];

    if let Some(t) = q.product_type {
        conds.push("p.product_type = ?".into());
        binds.push(Bind::I32(t));
    }
    if let Some(kw) = non_empty(&q.keyword) {
        conds.push(
            "(p.product_name LIKE ? OR ps.sku_value LIKE ? OR ps.sku_tsn = ? OR p.product_tsn = ?)"
                .into(),
        );
        let like = format!("%{kw}%");
        binds.push(Bind::Str(like.clone()));
        binds.push(Bind::Str(like));
        binds.push(Bind::Str(kw.to_string()));
        binds.push(Bind::Str(kw.to_string()));
    }
    (format!(" WHERE {}", conds.join(" AND ")), binds)
}

/// 出入库记录分页 WHERE（对应 Java `ProductStockRecordMapper.productStockRecordPage`）。
fn build_record_filter(
    q: &StockRecordPageQuery,
    shop_id: i32,
    begin_time: Option<i64>,
    end_time: Option<i64>,
) -> (String, Vec<Bind>) {
    let mut conds: Vec<String> = vec!["pr.shop_id = ?".into()];
    let mut binds: Vec<Bind> = vec![Bind::I32(shop_id)];

    if let Some(v) = q.in_or_out {
        conds.push("pr.in_or_out = ?".into());
        binds.push(Bind::I32(v));
    }
    if let Some(v) = q.r#type {
        conds.push("pr.type = ?".into());
        binds.push(Bind::I32(v));
    }
    if let Some(t) = begin_time {
        conds.push("pr.add_time >= ?".into());
        binds.push(Bind::I64(t));
    }
    if let Some(t) = end_time {
        conds.push("pr.add_time <= ?".into());
        binds.push(Bind::I64(t));
    }
    if let Some(kw) = non_empty(&q.keyword) {
        conds.push(
            "(pr.stock_code LIKE ? OR p.product_name LIKE ? OR p.product_sn LIKE ? OR ps.sku_value LIKE ?)"
                .into(),
        );
        let like = format!("%{kw}%");
        for _ in 0..4 {
            binds.push(Bind::Str(like.clone()));
        }
    }
    (format!(" WHERE {}", conds.join(" AND ")), binds)
}

/// 取非空的去空白字符串。
fn non_empty(v: &Option<String>) -> Option<&str> {
    v.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("库存数据不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stock_page_filter_scopes_shop_and_excludes_vendor() {
        let q = StockPageQuery::default();
        let (sql, binds) = build_stock_page_filter(&q, 7);
        assert!(sql.contains("p.shop_id = ?"));
        assert!(sql.contains("p.vendor_id IS NULL"));
        assert_eq!(binds.len(), 1);

        let q: StockPageQuery =
            serde_json::from_str(r#"{"productType":1,"keyword":"手机"}"#).unwrap();
        let (sql, binds) = build_stock_page_filter(&q, 7);
        assert!(sql.contains("p.product_type = ?"));
        assert!(sql.contains("p.product_name LIKE ?"));
        assert!(sql.contains("ps.sku_tsn = ?"));
        assert_eq!(binds.len(), 6); // shop + productType + 4 关键词
        assert_eq!(sql.matches('?').count(), binds.len());
    }

    #[test]
    fn record_filter_binds_match_placeholders() {
        let q = nextrade_core::models::catalog::product_stock::StockRecordPageQuery {
            in_or_out: Some(1),
            r#type: Some(11),
            keyword: Some("SN".into()),
            ..Default::default()
        };
        let (sql, binds) = build_record_filter(&q, 0, Some(100), Some(200));
        // shop + inOrOut + type + begin + end + 4 个 LIKE = 9
        assert_eq!(binds.len(), 9);
        assert_eq!(sql.matches('?').count(), binds.len());
        assert!(sql.contains("pr.stock_code LIKE ?"));
    }

    #[test]
    fn allocate_select_binds_org_before_keyword() {
        let (from_sql, where_sql, binds) = allocate_select_sql(
            "FROM product p INNER JOIN store_product sp ON sp.shop_id = $1",
            "p.shop_id = 0",
            Some("手机"),
            3,
        );
        assert!(from_sql.contains("sp.shop_id = $1"));
        assert!(where_sql.contains("p.product_name LIKE ?"));
        assert_eq!(binds.len(), 3); // 组织 + 两个 LIKE
        assert!(matches!(binds[0], Bind::I32(3)));
    }

    #[test]
    fn column_constants_match_entity_field_counts() {
        assert_eq!(PRODUCT_COLUMNS.split(',').count(), 81);
        assert_eq!(SKU_COLUMNS.split(',').count(), 16);
        assert_eq!(STOCK_RECORD_COLUMNS.split(',').count(), 8);
        assert_eq!(STOCK_RECORD_DETAIL_COLUMNS.split(',').count(), 5);
        assert_eq!(ADJUST_COLUMNS.split(',').count(), 10);
        assert_eq!(ALLOCATE_COLUMNS.split(',').count(), 10);
        assert_eq!(ALLOCATE_DETAIL_COLUMNS.split(',').count(), 7);
        assert_eq!(ALLOCATE_OP_COLUMNS.split(',').count(), 6);
        assert_eq!(STORE_PRODUCT_COLUMNS.split(',').count(), 18);
        assert_eq!(STORE_SKU_COLUMNS.split(',').count(), 11);
        assert_eq!(INVENTORY_LOG_COLUMNS.split(',').count(), 10);
    }

    #[test]
    fn reserved_words_are_backticked() {
        assert!(STOCK_RECORD_COLUMNS.contains("\"type\""));
        assert!(STOCK_RECORD_COLUMNS.contains("\"comment\""));
        assert!(INVENTORY_LOG_COLUMNS.contains("\"desc\""));
    }

    #[test]
    fn non_empty_trims_whitespace() {
        assert_eq!(non_empty(&Some("  x ".into())), Some("x"));
        assert_eq!(non_empty(&Some("   ".into())), None);
        assert_eq!(non_empty(&None), None);
    }
}
