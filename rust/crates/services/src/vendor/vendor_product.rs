//! 供应商商品域服务（vendor::vendor_product）。
//!
//! 对应原 Java（overseas）：
//! - `VendorProductController`（`/adminapi/vendor/product`）：
//!   `pageList` / `clientPage` → `VendorProductServiceImpl`
//! - `VendorProductBatchController`（`/adminapi/vendor/productBatch`）：
//!   `downloadTemplate` / `vendorProductBatchDeal`（Excel 导出）/
//!   `vendorProductBatchModify`（Excel 导入改价改库存）/
//!   `vendorProductBatchEdit`（批量编辑字段）
//! - `ProductController`（`/adminapi/product/product`）：
//!   `getVendorMaxPrice` / `vendorProductImport` / `vendorProductReImport`
//!   → `ProductServiceImpl`（`:6279` / `:6396` / `:6514`）
//!
//! ## 简化点（相对 Java）
//! 1. `vendorProductBatchModify`（Excel 导入）只支持按 **规格编码 `skuSn`**
//!    定位 `vendor_product_sku`，更新 `supply_price` / `sku_stock` /
//!    `supply_price_limit_type` / `supply_price_limit_val`；不支持新建商品
//!    （Java 会按模板整行建 vendor_product + sku）。
//! 2. `vendorProductBatchDeal` 导出的「分类」列只取本级类目名（Java 拼多级 `|`）。
//! 3. `vendorProductImport` 只落 `product` 最小字段集 + 首个 SKU 供货价为售价，
//!    **不迁移**视频/相册/属性/规格表（Java `transferInit` 全量迁移）；
//!    不做店铺加价策略（`vendorSetPriceType`）。
//! 4. `vendorProductReImport` 更新既有商品主行字段并清
//!    `is_need_re_import_vendor_product` 标记，同样不迁移子表。
//! 5. `clientPage` 的 `isCanImport=1` 简化为「审核通过 + 在售」附加条件。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::models::vendor::vendor_product::{
    vendor_sku_max_price, VendorProductBatchDealDTO, VendorProductBatchEditDTO,
    VendorProductBatchEditItem, VendorProductClientPageQuery, VendorProductExportRow,
    VendorProductImportParam, VendorProductPageQuery, VendorProductPageVO,
    VendorProductReImportParam, IMPORT_EXCEL_HEADERS,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 供应商商品域服务（挂载于 `AdminState.vendor.vendor_product`）。
pub struct VendorProductService {
    repos: Arc<Repositories>,
}

impl VendorProductService {
    /// 构造。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn product_repo(
        &self,
    ) -> &nextrade_infrastructure::db::repo::vendor::vendor_product::VendorProductRepository {
        &self.repos.vendor.vendor_product
    }

    fn vendor_repo(
        &self,
    ) -> &nextrade_infrastructure::db::repo::vendor::vendor::VendorRepository {
        &self.repos.vendor.vendor
    }

    // ————————————————————— 分页 —————————————————————

    /// `GET /vendor/product/pageList`（Java `VendorProductServiceImpl#pageList`）。
    pub async fn page_list(
        &self,
        q: &VendorProductPageQuery,
        vendor_scope: Option<i32>,
    ) -> Result<PageResult<VendorProductPageVO>> {
        let (total, rows) = self.product_repo().paginate(q, vendor_scope).await?;
        let vos = self.to_vos(&rows).await?;
        let query = PageQuery {
            page: q.page,
            page_size: q.size,
        };
        Ok(PageResult::new(total, vos, &query))
    }

    /// `GET /vendor/product/clientPage`（Java `#clientPage`）。
    pub async fn client_page(
        &self,
        q: &VendorProductClientPageQuery,
        vendor_scope: Option<i32>,
    ) -> Result<PageResult<VendorProductPageVO>> {
        let (total, rows) = self
            .product_repo()
            .paginate_client(q, vendor_scope)
            .await?;
        let vos = self.to_vos(&rows).await?;
        let query = PageQuery {
            page: q.page,
            page_size: q.size,
        };
        Ok(PageResult::new(total, vos, &query))
    }

    /// 行 → VO（附加 vendorName）。
    async fn to_vos(
        &self,
        rows: &[nextrade_core::entities::vendor::VendorProduct],
    ) -> Result<Vec<VendorProductPageVO>> {
        let vendor_ids: Vec<i32> = rows.iter().map(|r| r.vendor_id).collect();
        let names = self.vendor_repo().find_vendor_names(&vendor_ids).await?;
        let name_map: HashMap<i32, String> = names.into_iter().collect();
        Ok(rows
            .iter()
            .map(|r| VendorProductPageVO {
                id: r.id,
                product_name: Some(r.product_name.clone()),
                product_brand_id: r.product_brand_id,
                product_category_id: Some(r.product_category_id),
                product_brief: r.product_brief.clone(),
                product_state: Some(r.product_state),
                sku_type: Some(r.sku_type),
                product_desc: r.product_desc.clone(),
                audit_state: Some(r.audit_state),
                pic_url: Some(r.pic_url.clone()),
                pic_thumb: Some(r.pic_thumb.clone()),
                pic_original: Some(r.pic_original.clone()),
                pic_large: Some(r.pic_large.clone()),
                is_recycle: Some(r.is_recycle),
                vendor_id: Some(r.vendor_id),
                vendor_name: name_map.get(&r.vendor_id).cloned(),
                source_product_id: r.source_product_id,
                create_time: Some(r.create_time),
                create_by_name: r.create_by_name.clone(),
                update_time: Some(r.update_time),
                update_by_name: Some(r.update_by_name.clone()),
            })
            .collect())
    }

    // ————————————————————— Excel 模板 / 导出 —————————————————————

    /// `GET /vendor/productBatch/downloadTemplate`：含表头的空模板。
    pub fn download_template(&self) -> Result<Vec<u8>> {
        crate::system::excel::build_xlsx(
            &IMPORT_EXCEL_HEADERS,
            &[],
            "供应商商品导入模板",
        )
    }

    /// `GET|POST /vendor/productBatch/vendorProductBatchDeal`：按范围导出 xlsx
    /// （Java `vendorProductBatchDeal`，`dealRange=3` 供应商商品 + `rangeIds`）。
    pub async fn batch_deal_export(
        &self,
        dto: &VendorProductBatchDealDTO,
        vendor_scope: Option<i32>,
    ) -> Result<Vec<u8>> {
        let mut ids = dto.id_list();
        if ids.is_empty() {
            return Err(Error::Validation("请选择要导出的数据".into()));
        }
        // 平台 + 供应商范围兜底：导出时过滤掉越权 vendor 的行
        let products = self.product_repo().find_by_ids(&ids).await?;
        let products: Vec<_> = match vendor_scope {
            Some(scope) => products
                .into_iter()
                .filter(|p| p.vendor_id == scope)
                .collect(),
            None => products,
        };
        if products.is_empty() {
            return Err(Error::Biz {
                code: 500,
                msg: "选中的供应商商品不存在".into(),
            });
        }
        ids = products.iter().map(|p| p.id).collect();

        let skus = self.product_repo().find_skus_by_product_ids(&ids).await?;
        let mut first_sku: HashMap<i32, &nextrade_core::entities::vendor::VendorProductSku> =
            HashMap::new();
        for sku in &skus {
            first_sku.entry(sku.vendor_product_id).or_insert(sku);
        }

        let category_ids: Vec<i32> = products.iter().map(|p| p.product_category_id).collect();
        let brand_ids: Vec<i32> = products.iter().filter_map(|p| p.product_brand_id).collect();
        let categories = self.vendor_repo().find_category_names(&category_ids).await?;
        let brands = self.vendor_repo().find_brand_names(&brand_ids).await?;
        let cat_map: HashMap<i32, String> = categories.into_iter().collect();
        let brand_map: HashMap<i32, String> = brands.into_iter().collect();

        let rows: Vec<Vec<String>> = products
            .iter()
            .map(|p| {
                let sku = first_sku.get(&(p.id as i32)).copied();
                VendorProductExportRow {
                    pic_url: p.pic_url.clone(),
                    product_name: p.product_name.clone(),
                    category_name: cat_map
                        .get(&p.product_category_id)
                        .cloned()
                        .unwrap_or_default(),
                    brand_name: p
                        .product_brand_id
                        .and_then(|b| brand_map.get(&b).cloned())
                        .unwrap_or_default(),
                    product_brief: p.product_brief.clone().unwrap_or_default(),
                    product_state: p.product_state.to_string(),
                    sku_sn: sku.and_then(|s| s.sku_sn.clone()).unwrap_or_default(),
                    supply_price: sku.map(|s| s.supply_price.to_string()).unwrap_or_default(),
                    supply_price_limit_type: sku
                        .map(|s| s.supply_price_limit_type.to_string())
                        .unwrap_or_else(|| "1".into()),
                    supply_price_limit_val: sku
                        .map(|s| s.supply_price_limit_val.to_string())
                        .unwrap_or_else(|| "0".into()),
                    sku_stock: sku
                        .and_then(|s| s.sku_stock)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "0".into()),
                }
                .to_cells()
            })
            .collect();
        crate::system::excel::build_xlsx(&IMPORT_EXCEL_HEADERS, &rows, "供应商商品")
    }

    // ————————————————————— 批量编辑 / 导入 —————————————————————

    /// `POST /vendor/productBatch/vendorProductBatchEdit`：批量编辑字段
    /// （Java `VendorProductBatchEditDTO.buildVendorProducts` → 按非空字段 update）。
    pub async fn batch_edit(&self, dto: &VendorProductBatchEditDTO) -> Result<u64> {
        if dto.edit_items.is_empty() {
            return Err(Error::Validation("editItems 不能为空".into()));
        }
        let now = nextrade_common::utils::now_millis();
        self.product_repo().batch_edit(dto, now).await
    }

    /// `POST /vendor/productBatch/vendorProductBatchModify`：Excel 导入改价改库存。
    ///
    /// 返回 `(成功条数, 失败原因列表)`（对齐 Java `VendorProductBatchModifyVO`
    /// 的 successCount/failCount 字段口径）。
    pub async fn batch_modify_excel(
        &self,
        bytes: &[u8],
    ) -> Result<(u64, Vec<String>)> {
        let rows = crate::system::excel::read_xlsx(bytes)?;
        if rows.len() <= 1 {
            return Err(Error::Biz {
                code: 500,
                msg: "表格数据为空".into(),
            });
        }
        // 表头（第 0 行）列序对齐 IMPORT_EXCEL_HEADERS
        let headers: Vec<String> = rows[0].clone();
        let col = |name: &str| headers.iter().position(|h| h.trim() == name);

        let c_name = col("商品名称（必须）");
        let c_sku_sn = col("规格编码");
        let c_price = col("供货价（必须）");
        let c_limit_type = col("限售金额类型（默认1）");
        let c_limit_val = col("限售金额值（默认0）");
        let c_stock = col("库存（默认0）");

        let mut success: u64 = 0;
        let mut errors: Vec<String> = Vec::new();
        let now = nextrade_common::utils::now_millis();

        for (idx, row) in rows.iter().enumerate().skip(1) {
            let get = |c: Option<usize>| c.and_then(|i| row.get(i)).map(|s| s.trim().to_string());
            let sku_sn = get(c_sku_sn).unwrap_or_default();
            let product_name = get(c_name).unwrap_or_default();
            if sku_sn.is_empty() {
                errors.push(format!("第{}行：规格编码为空", idx + 1));
                continue;
            }
            let supply_price = get(c_price).and_then(|v| v.parse::<rust_decimal::Decimal>().ok());
            let sku_stock = get(c_stock).and_then(|v| v.parse::<i32>().ok());
            let limit_type = get(c_limit_type).and_then(|v| v.parse::<i32>().ok());
            let limit_val =
                get(c_limit_val).and_then(|v| v.parse::<rust_decimal::Decimal>().ok());
            let Some(sku) = self.product_repo().find_sku_by_sn(&sku_sn).await? else {
                errors.push(format!("第{}行：规格编码 {sku_sn} 不存在", idx + 1));
                continue;
            };
            let _ = &product_name;
            self.product_repo()
                .update_sku_price_stock(
                    sku.id,
                    supply_price,
                    sku_stock,
                    limit_type,
                    limit_val,
                    now,
                )
                .await?;
            success += 1;
        }
        Ok((success, errors))
    }

    // ————————————————————— 商品导入三件套 —————————————————————

    /// `GET /product/product/getVendorMaxPrice`：SKU 最高可售价
    /// （Java `ProductServiceImpl#getVendorMaxPrice` → `VendorProductSku#getMaxPrice`）。
    pub async fn vendor_max_price(&self, vendor_product_sku_id: i64) -> Result<rust_decimal::Decimal> {
        let sku = self
            .product_repo()
            .find_sku_by_id(vendor_product_sku_id)
            .await?
            .ok_or_else(|| Error::Biz {
                code: 500,
                msg: "商品规格不存在".into(),
            })?;
        Ok(vendor_sku_max_price(
            sku.supply_price,
            sku.supply_price_limit_type,
            sku.supply_price_limit_val,
        ))
    }

    /// `POST /product/product/vendorProductImport`：把选中的 vendor_product
    /// 转入正式商品表 `product`（Java `ProductServiceImpl#vendorProductImport` 最小化）。
    pub async fn vendor_product_import(
        &self,
        p: &VendorProductImportParam,
        shop_id: i32,
    ) -> Result<()> {
        if p.vendor_product_ids.is_empty() {
            return Err(Error::Validation("供应商商品ID不能为空".into()));
        }
        let imported = self
            .product_repo()
            .find_imported_vendor_product_ids(shop_id)
            .await?;
        let imported: std::collections::HashSet<i32> = imported.into_iter().collect();
        let products = self
            .product_repo()
            .find_by_ids(&p.vendor_product_ids)
            .await?;
        if products.iter().any(|vp| imported.contains(&(vp.id as i32))) {
            return Err(Error::Biz {
                code: 500,
                msg: "商品已导入过，请勿重复导入".into(),
            });
        }
        if products.is_empty() {
            return Err(Error::Biz {
                code: 500,
                msg: "供应商商品不存在".into(),
            });
        }

        let skus = self
            .product_repo()
            .find_skus_by_product_ids(&products.iter().map(|p| p.id).collect::<Vec<_>>())
            .await?;
        let now = nextrade_common::utils::now_millis() as i32;

        for vp in &products {
            let vp_skus: Vec<_> = skus
                .iter()
                .filter(|s| s.vendor_product_id == vp.id as i32)
                .collect();
            if vp_skus.is_empty() {
                return Err(Error::Biz {
                    code: 500,
                    msg: format!("供应商商品 {} 缺少规格数据", vp.id),
                });
            }
            let product_stock: i32 = vp_skus.iter().filter_map(|s| s.sku_stock).sum();
            let product_price = vp_skus[0].supply_price;
            let product_sn = format!("SP{}{}", nextrade_common::utils::now_millis(), vp.id);
            let product_id = self
                .product_repo()
                .insert_product_from_vendor(vp, shop_id, product_stock, product_price, &product_sn, now)
                .await?;
            // Java：导入后按最小 SKU 价回填商品价（此处 sku 数 ≥1，最小价即首个）
            self.product_repo()
                .update_product_price(product_id, product_price, now)
                .await?;
            // Java：导入成功建立 vendor_shop_bind（不存在才插）
            self.product_repo()
                .ensure_vendor_shop_bind(vp.vendor_id, shop_id, now)
                .await?;
        }
        Ok(())
    }

    /// `POST /product/product/vendorProductReImport`：重导（更新既有商品）。
    pub async fn vendor_product_re_import(
        &self,
        p: &VendorProductReImportParam,
    ) -> Result<()> {
        let vendor_product_id = p.vendor_product_id.ok_or_else(|| {
            Error::Validation("vendorProductId 不能为空".into())
        })?;
        let product_id = p
            .product_id
            .filter(|x| *x > 0)
            .ok_or_else(|| Error::Validation("productId 不能为空".into()))?;

        let exists: bool = sqlx::query_as::<_, (i32,)>(
            "SELECT product_id FROM product WHERE product_id = $1 LIMIT 1",
        )
        .bind(product_id)
        .fetch_optional(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?
        .is_some();
        if !exists {
            return Err(Error::Biz {
                code: 500,
                msg: "需要重新导入的商品不存在".into(),
            });
        }

        let vp = self
            .product_repo()
            .find_by_id(vendor_product_id)
            .await?
            .ok_or_else(|| Error::Biz {
                code: 500,
                msg: "供应商商品不存在".into(),
            })?;
        let skus = self
            .product_repo()
            .find_skus_by_product_ids(&[vendor_product_id])
            .await?;
        let product_stock: i32 = skus.iter().filter_map(|s| s.sku_stock).sum();
        let product_price = skus
            .first()
            .map(|s| s.supply_price)
            .unwrap_or(rust_decimal::Decimal::ZERO);
        let now = nextrade_common::utils::now_millis() as i32;
        self.product_repo()
            .update_product_from_vendor(product_id, &vp, product_stock, product_price, now)
            .await?;
        Ok(())
    }
}

