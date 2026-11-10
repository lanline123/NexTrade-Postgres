//! 供应商商品（`vendor_product` / `vendor_product_sku`）仓储 + 商品导入落库。
//!
//! 对应原 Java（overseas）：
//! - `com.nextrade.mapper.vendor.product.VendorProductMapper` /
//!   `VendorProductSkuMapper`（MyBatis-Plus BaseMapper）
//! - `VendorProductServiceImpl#pageList` / `#clientPage`
//! - `VendorProductBatchServiceImpl#vendorProductBatchDeal`（导出取数）
//! - `ProductServiceImpl#vendorProductImport` / `#vendorProductReImport` /
//!   `#getVendorMaxPrice`（vendor_product → product 导入）
//!
//! 列名对齐真实 schema（`crates/core/src/entities/vendor.rs`）。
//! ⚠️ `vendor_product.active_source_product_key` 与
//! `vendor_product_sku.active_source_sku_key` 是 GENERATED ALWAYS 生成列：
//! **可 SELECT、禁止 INSERT/UPDATE**。

use sqlx::{PgPool, QueryBuilder};

use nextrade_core::entities::vendor::{VendorProduct, VendorProductSku};
use nextrade_core::models::vendor::vendor_product::{
    VendorProductBatchEditDTO, VendorProductClientPageQuery, VendorProductPageQuery,
};
use nextrade_core::{Error, Result};

/// `vendor_product` 固定 SELECT 列（与 `entities::vendor::VendorProduct` 一一对应，
/// 含生成列 `active_source_product_key`）。
const VENDOR_PRODUCT_SELECT: &str = "SELECT id, product_name, product_brand_id, product_category_id, product_sn_generate_type, product_brief, product_state, sku_type, product_desc, audit_state, pic_url, pic_thumb, pic_original, pic_large, is_recycle, vendor_id, source_product_id, create_time, create_by_id, create_by_name, update_time, update_by_id, update_by_name, is_del, active_source_product_key FROM vendor_product";

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct VendorProductRepository {
    pool: PgPool,
}

impl VendorProductRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 分页 —————————————————————

    /// 供应商商品分页（对应 Java `VendorProductServiceImpl#pageList` /
    /// `#clientPage`，两者过滤条件本仓储统一处理）。
    ///
    /// `vendor_scope`：供应商账号固定只看自己；`None` 平台看全部。
    #[allow(clippy::too_many_lines)]
    pub async fn paginate(
        &self,
        q: &VendorProductPageQuery,
        vendor_scope: Option<i32>,
    ) -> Result<(u64, Vec<VendorProduct>)> {
        let (limit, offset) = q.limit_offset();

        let mut count_qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM vendor_product WHERE is_del = 0");
        Self::push_filters(&mut count_qb, q, vendor_scope);
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut row_qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(format!(
            "{VENDOR_PRODUCT_SELECT} WHERE is_del = 0"
        ));
        Self::push_filters(&mut row_qb, q, vendor_scope);
        // ORDER BY 列名为硬编码字面量，LIMIT/OFFSET 为 u64 字面量，无注入面。
        row_qb.push(format!(" ORDER BY id DESC LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<VendorProduct> = row_qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// clientPage 分页（Java `VendorProductClientPageQuery` 语义）。
    pub async fn paginate_client(
        &self,
        q: &VendorProductClientPageQuery,
        vendor_scope: Option<i32>,
    ) -> Result<(u64, Vec<VendorProduct>)> {
        let pq = VendorProductPageQuery {
            page: q.page,
            size: q.size,
            brand_id: q.product_brand_id,
            category_id: q.product_category_id,
            vendor_id: q.vendor_id,
            // clientPage 语义：只给可导入/可售数据看
            audit_state: q.is_can_import.filter(|v| *v == 1).map(|_| 1),
            product_state: q.is_can_import.filter(|v| *v == 1).map(|_| 1),
            ..Default::default()
        };
        self.paginate(&pq, vendor_scope).await
    }

    /// 追加 pageList 过滤条件。
    fn push_filters(
        qb: &mut QueryBuilder<'_, sqlx::Postgres>,
        q: &VendorProductPageQuery,
        vendor_scope: Option<i32>,
    ) {
        if let Some(scope) = vendor_scope {
            qb.push(" AND vendor_id = ");
            qb.push_bind(scope);
        } else if let Some(vendor_id) = q.vendor_id.filter(|v| *v > 0) {
            qb.push(" AND vendor_id = ");
            qb.push_bind(vendor_id);
        }
        let ids = q.id_list();
        if !ids.is_empty() {
            qb.push(" AND id IN (");
            let mut sep = qb.separated(", ");
            for id in ids {
                sep.push_bind(id);
            }
            drop(sep);
            qb.push(")");
        }
        if let Some(brand_id) = q.brand_id.filter(|v| *v > 0) {
            qb.push(" AND product_brand_id = ");
            qb.push_bind(brand_id);
        }
        if let Some(category_id) = q.category_id.filter(|v| *v > 0) {
            qb.push(" AND product_category_id = ");
            qb.push_bind(category_id);
        }
        if let Some(state) = q.product_state {
            qb.push(" AND product_state = ");
            qb.push_bind(state);
        }
        if let Some(audit) = q.audit_state {
            qb.push(" AND audit_state = ");
            qb.push_bind(audit);
        }
        if let Some(recycle) = q.is_recycle {
            qb.push(" AND is_recycle = ");
            qb.push_bind(recycle);
        } else {
            // 列表默认排除回收站（Java pageList 由前端传 isRecycle=0，这里兜底）
            qb.push(" AND is_recycle = 0");
        }
        if let Some(keyword) = q.keyword.as_deref().filter(|s| !s.trim().is_empty()) {
            qb.push(" AND product_name ILIKE ");
            qb.push_bind(format!("%{}%", keyword.trim()));
        }
    }

    // ————————————————————— 单行 / 批量取数 —————————————————————

    /// 按主键取供应商商品。
    pub async fn find_by_id(&self, id: i64) -> Result<Option<VendorProduct>> {
        sqlx::query_as::<_, VendorProduct>(&format!(
            "{VENDOR_PRODUCT_SELECT} WHERE id = $1 LIMIT 1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 批量取供应商商品（导出 / 导入用；Java `selectVendorProducts` 主行段）。
    pub async fn find_by_ids(&self, ids: &[i64]) -> Result<Vec<VendorProduct>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new(format!("{VENDOR_PRODUCT_SELECT} WHERE is_del = 0 AND id IN ("));
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(") ORDER BY id DESC");
        let rows: Vec<VendorProduct> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows)
    }

    /// 取商品的全部 SKU（对应 Java `VendorProductSkuMapper` 按 vendor_product_id 查询）。
    pub async fn find_skus_by_product_ids(&self, ids: &[i64]) -> Result<Vec<VendorProductSku>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
            "SELECT id, vendor_product_id, source_sku_id, min_purchase_qty, sku_attr_val, sku_attr_json::text AS sku_attr_json, sku_sn, sku_tsn, sku_weight, sku_stock, supply_price, supply_price_limit_type, supply_price_limit_val, sales_volume, vendor_id, create_time, create_by_id, create_by_name, update_time, update_by_id, update_by_name, is_del, active_source_sku_key FROM vendor_product_sku WHERE is_del = 0 AND vendor_product_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(") ORDER BY id ASC");
        let rows: Vec<VendorProductSku> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows)
    }

    /// 按 sku 主键取单个 SKU（`getVendorMaxPrice` 用）。
    pub async fn find_sku_by_id(&self, sku_id: i64) -> Result<Option<VendorProductSku>> {
        sqlx::query_as::<_, VendorProductSku>(
            "SELECT id, vendor_product_id, source_sku_id, min_purchase_qty, sku_attr_val, sku_attr_json::text AS sku_attr_json, sku_sn, sku_tsn, sku_weight, sku_stock, supply_price, supply_price_limit_type, supply_price_limit_val, sales_volume, vendor_id, create_time, create_by_id, create_by_name, update_time, update_by_id, update_by_name, is_del, active_source_sku_key FROM vendor_product_sku WHERE id = $1 LIMIT 1",
        )
        .bind(sku_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按规格编码取单个 SKU（Excel 导入改价改库存用）。
    pub async fn find_sku_by_sn(&self, sku_sn: &str) -> Result<Option<VendorProductSku>> {
        sqlx::query_as::<_, VendorProductSku>(
            "SELECT id, vendor_product_id, source_sku_id, min_purchase_qty, sku_attr_val, sku_attr_json::text AS sku_attr_json, sku_sn, sku_tsn, sku_weight, sku_stock, supply_price, supply_price_limit_type, supply_price_limit_val, sales_volume, vendor_id, create_time, create_by_id, create_by_name, update_time, update_by_id, update_by_name, is_del, active_source_sku_key FROM vendor_product_sku WHERE sku_sn = $1 AND is_del = 0 LIMIT 1",
        )
        .bind(sku_sn)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 更新 SKU 价格 / 库存 / 限售额（Excel 导入改价改库存；
    /// 提交的 `Some` 才更新，同时同步 vendor_product.update_time）。
    pub async fn update_sku_price_stock(
        &self,
        sku_id: i64,
        supply_price: Option<rust_decimal::Decimal>,
        sku_stock: Option<i32>,
        limit_type: Option<i32>,
        limit_val: Option<rust_decimal::Decimal>,
        now_millis: i64,
    ) -> Result<()> {
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("UPDATE vendor_product_sku SET update_time = ");
        qb.push_bind(now_millis);
        if let Some(v) = supply_price {
            qb.push(", supply_price = ");
            qb.push_bind(v);
        }
        if let Some(v) = sku_stock {
            qb.push(", sku_stock = ");
            qb.push_bind(v);
        }
        if let Some(v) = limit_type {
            qb.push(", supply_price_limit_type = ");
            qb.push_bind(v);
        }
        if let Some(v) = limit_val {
            qb.push(", supply_price_limit_val = ");
            qb.push_bind(v);
        }
        qb.push(" WHERE id = ");
        qb.push_bind(sku_id);
        qb.build().execute(&self.pool).await.map_err(map_err)?;
        // 同步主行更新时间
        sqlx::query(
            "UPDATE vendor_product SET update_time = $1 WHERE id = (SELECT vendor_product_id FROM vendor_product_sku WHERE id = $2)",
        )
        .bind(now_millis)
        .bind(sku_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    // ————————————————————— 批量编辑 —————————————————————

    /// 批量编辑（对应 Java `VendorProductBatchServiceImpl#vendorProductBatchEdit`
    /// 的落库语义：仅更新提交的非空字段）。
    pub async fn batch_edit(
        &self,
        dto: &VendorProductBatchEditDTO,
        now_millis: i64,
    ) -> Result<u64> {
        let mut affected: u64 = 0;
        for item in &dto.edit_items {
            let Some(id) = item.id.filter(|x| *x > 0) else {
                continue;
            };
            let mut qb: QueryBuilder<'_, sqlx::Postgres> =
                QueryBuilder::new("UPDATE vendor_product SET update_time = ");
            qb.push_bind(now_millis);
            if let Some(v) = &item.product_name {
                qb.push(", product_name = ");
                qb.push_bind(v.clone());
            }
            if let Some(v) = item.product_brand_id {
                qb.push(", product_brand_id = ");
                qb.push_bind(v);
            }
            if let Some(v) = item.product_category_id {
                qb.push(", product_category_id = ");
                qb.push_bind(v);
            }
            if let Some(v) = &item.product_brief {
                qb.push(", product_brief = ");
                qb.push_bind(v.clone());
            }
            if let Some(v) = item.product_state {
                qb.push(", product_state = ");
                qb.push_bind(v);
            }
            qb.push(" WHERE id = ");
            qb.push_bind(id);
            let out = qb.build().execute(&self.pool).await.map_err(map_err)?;
            affected += out.rows_affected();
        }
        Ok(affected)
    }

    // ————————————————————— 导入（vendor_product → product） —————————————————————

    /// 该店铺已导入过的 vendor_product id（Java：查 product 表
    /// `vendor_product_id > 0 AND shop_id = ?`）。
    pub async fn find_imported_vendor_product_ids(&self, shop_id: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT vendor_product_id FROM product WHERE vendor_product_id > 0 AND shop_id = $1",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 按最小字段集插入正式商品（对齐 Java `VendorProductBO#toProduct` + `save`）。
    ///
    /// 返回新 `product_id`。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_product_from_vendor(
        &self,
        vp: &VendorProduct,
        shop_id: i32,
        product_stock: i32,
        product_price: rust_decimal::Decimal,
        product_sn: &str,
        now: i32,
    ) -> Result<i32> {
        let out: i32 = sqlx::query_scalar(
            "INSERT INTO product (product_name, product_sn, product_stock, product_price, product_status, product_type, category_id, brand_id, shop_id, check_status, product_brief, product_desc, pic_url, pic_thumb, pic_original, pic_large, add_time, last_update, is_delete, fixed_shipping_type, fixed_shipping_fee, vendor_product_id, vendor_id) VALUES ($1, $2, $3, $4, 0, 1, $5, $6, $7, 1, $8, $9, $10, $11, $12, $13, $14, $15, 0, 1, 0, $16, $17) RETURNING product_id",
        )
        .bind(vp.product_name.as_str())
        .bind(product_sn)
        .bind(product_stock)
        .bind(product_price)
        .bind(vp.product_category_id)
        .bind(vp.product_brand_id.unwrap_or(0))
        .bind(shop_id)
        .bind(vp.product_brief.as_deref().unwrap_or(""))
        .bind(&vp.product_desc)
        .bind(vp.pic_url.as_str())
        .bind(vp.pic_url.as_str())
        .bind(vp.pic_url.as_str())
        .bind(vp.pic_url.as_str())
        .bind(now)
        .bind(now)
        .bind(vp.id as i32)
        .bind(vp.vendor_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(out)
    }

    /// 更新商品价（导入后按最小 SKU 价回填，对齐 Java `updateById(product)`）。
    pub async fn update_product_price(
        &self,
        product_id: i32,
        price: rust_decimal::Decimal,
        now: i32,
    ) -> Result<()> {
        sqlx::query("UPDATE product SET product_price = $1, last_update = $2 WHERE product_id = $3")
            .bind(price)
            .bind(now)
            .bind(product_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 重新导入：用 vendor_product 数据覆盖既有商品（Java `vendorProductReImport`
    /// 最小化：更新名称/简介/图/类目/品牌/库存/价 + 清「需重导」标记）。
    pub async fn update_product_from_vendor(
        &self,
        product_id: i32,
        vp: &VendorProduct,
        product_stock: i32,
        product_price: rust_decimal::Decimal,
        now: i32,
    ) -> Result<u64> {
        let out = sqlx::query(
            "UPDATE product SET product_name = $1, product_brief = $2, product_desc = $3, pic_url = $4, pic_thumb = $5, pic_original = $6, pic_large = $7, category_id = $8, brand_id = $9, product_stock = $10, product_price = $11, last_update = $12, is_need_re_import_vendor_product = 0 WHERE product_id = $13",
        )
        .bind(vp.product_name.as_str())
        .bind(vp.product_brief.as_deref().unwrap_or(""))
        .bind(&vp.product_desc)
        .bind(vp.pic_url.as_str())
        .bind(vp.pic_url.as_str())
        .bind(vp.pic_url.as_str())
        .bind(vp.pic_url.as_str())
        .bind(vp.product_category_id)
        .bind(vp.product_brand_id.unwrap_or(0))
        .bind(product_stock)
        .bind(product_price)
        .bind(now)
        .bind(product_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(out.rows_affected())
    }

    /// 导入成功后建立供应商-店铺绑定（Java：`vendor_shop_bind` 不存在才插）。
    pub async fn ensure_vendor_shop_bind(&self, vendor_id: i32, shop_id: i32, now: i32) -> Result<()> {
        let exist: Option<(i32,)> = sqlx::query_as(
            "SELECT id FROM vendor_shop_bind WHERE vendor_id = $1 AND shop_id = $2 LIMIT 1",
        )
        .bind(vendor_id)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        if exist.is_none() {
            sqlx::query("INSERT INTO vendor_shop_bind (vendor_id, shop_id, add_time) VALUES ($1, $2, $3)")
                .bind(vendor_id)
                .bind(shop_id)
                .bind(now)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
        }
        Ok(())
    }
}

/// SQLx 错误 → 统一错误类型。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("供应商商品不存在".into()),
        other => Error::Database(other.to_string()),
    }
}
