//! 站点搬家目录域仓储（site_clone::catalog_data）：
//! - 导出侧：分类/品牌/运费模板/商品的全保真只读查询（按源店铺圈定）；
//! - 导入侧：分类按路径、品牌/运费/服务按名的对齐查询与新建落库，
//!   商品图片 external 素材登记，以及新建商品 id 的回收查询。
//!
//! 商品子表（SKU/属性/图库/视频）的**写入**不在这里：导入统一走
//! `ProductManageService::create`，本仓储只负责导出读取与结构域对齐。

use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};

use nextrade_core::models::site_clone::clone::{
    CloneAttribute, CloneBrand, CloneCategory, CloneGalleryPic, CloneProductService,
    CloneShippingTplInfo, CloneSku, CloneVideo,
};
use nextrade_core::{Error, Result};

/// 商品主表导出行（`CloneProduct` 的标量部分，子表与名称字段另行组装）。
#[derive(Debug, Clone, Default, FromRow)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneProductRow {
    pub product_id: i32,
    pub product_name: String,
    pub product_sn: String,
    pub product_tsn: Option<String>,
    pub seo_identification: Option<String>,
    pub product_type: Option<i32>,
    pub sale_mode: i32,
    pub delivery_mode: i32,
    pub product_status: i32,
    pub product_price: Decimal,
    pub market_price: Decimal,
    pub cost_price: Option<Decimal>,
    pub product_stock: i32,
    pub product_weight: Decimal,
    pub product_volume: Option<Decimal>,
    pub category_id: i32,
    pub brand_id: i32,
    pub shipping_tpl_id: i32,
    pub free_shipping: i32,
    pub fixed_shipping_type: Option<i32>,
    pub fixed_shipping_fee: Option<Decimal>,
    pub no_shipping: Option<i32>,
    pub keywords: Option<String>,
    pub product_brief: String,
    pub product_desc: Option<String>,
    pub pic_url: String,
    pub pic_thumb: String,
    pub pic_original: String,
    pub pic_large: String,
    pub is_best: i32,
    pub is_new: i32,
    pub is_hot: i32,
    pub is_promote: i32,
    pub promote_price: Decimal,
    pub sort_order: i32,
    pub store_sort_order: i32,
    pub remark: String,
    pub give_integral: i32,
    pub rank_integral: i32,
    pub integral: i32,
    pub virtual_sales: i32,
    pub limit_number: i32,
    pub min_buy_count: i32,
    pub product_care: Option<String>,
    pub is_support_return: i32,
    pub is_support_cod: i32,
    pub product_video: Option<String>,
    pub prepay_price: Option<Decimal>,
    pub virtual_sample: Option<String>,
    pub paid_content: Option<String>,
    pub product_unit: Option<String>,
    pub is_inquiry: i32,
    pub is_shop_pickup: i32,
    pub is_logistics: i32,
    pub is_shop_delivery: i32,
    pub delivery_service_fee_type: i32,
    pub delivery_service_fee_value: Decimal,
    pub server_service_fee_type: i32,
    pub server_service_fee_value: Decimal,
    /// `product_service_ids` 原始存储串（JSON 数字数组形态，宽容解析）。
    pub product_service_ids: Option<String>,
}

const PRODUCT_COLUMNS: &str = "product_id, product_name, product_sn, product_tsn, seo_identification,
    product_type, sale_mode, delivery_mode, product_status, product_price, market_price, cost_price,
    product_stock, product_weight, product_volume, category_id, brand_id, shipping_tpl_id,
    free_shipping, fixed_shipping_type, fixed_shipping_fee, no_shipping, keywords, product_brief,
    product_desc, pic_url, pic_thumb, pic_original, pic_large, is_best, is_new, is_hot, is_promote,
    promote_price, sort_order, store_sort_order, remark, give_integral, rank_integral, integral,
    virtual_sales, limit_number, min_buy_count, product_care, is_support_return, is_support_cod,
    product_video, prepay_price, virtual_sample, paid_content, product_unit, is_inquiry,
    is_shop_pickup, is_logistics, is_shop_delivery, delivery_service_fee_type,
    delivery_service_fee_value, server_service_fee_type, server_service_fee_value, product_service_ids";

/// 目录域仓储。
#[derive(Clone)]
pub struct SiteCloneCatalogRepository {
    pool: PgPool,
}

impl SiteCloneCatalogRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ———————————————————— 导出：计数与结构域 ————————————————————

    pub async fn count_products(&self, shop_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM product WHERE shop_id = $1 AND is_delete = 0",
        )
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    pub async fn count_categories(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM category")
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    pub async fn count_brands(&self, shop_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM brand WHERE shop_id = 0 OR shop_id = $1",
        )
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    pub async fn count_shipping_tpls(&self, shop_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM shipping_tpl WHERE shop_id = 0 OR shop_id = $1",
        )
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    pub async fn list_categories(&self) -> Result<Vec<CloneCategory>> {
        sqlx::query_as::<_, CloneCategory>(
            "SELECT category_id, category_name, parent_id, sort_order, is_show, keywords, category_desc, measure_unit, seo_title, short_name, category_pic, category_ico, is_hot, search_keywords, seo_identification
             FROM category ORDER BY parent_id ASC, sort_order ASC, category_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_brands(&self, shop_id: i32) -> Result<Vec<CloneBrand>> {
        sqlx::query_as::<_, CloneBrand>(
            "SELECT brand_id, brand_name, brand_logo, brand_desc, site_url, sort_order, is_show, first_word, brand_en_name
             FROM brand WHERE shop_id = 0 OR shop_id = $1 ORDER BY sort_order ASC, brand_id ASC",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_shipping_types(&self, shop_id: i32) -> Result<Vec<nextrade_core::models::site_clone::clone::CloneShippingType>> {
        sqlx::query_as::<_, nextrade_core::models::site_clone::clone::CloneShippingType>(
            "SELECT shipping_type_id, shipping_type_name, shipping_type_desc, shipping_time_desc, is_support_cod, sort_order, shop_id
             FROM shipping_type WHERE shop_id = 0 OR shop_id = $1 ORDER BY sort_order ASC, shipping_type_id ASC",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 运费模板主行（不含明细，明细按模板逐个装载）。
    pub async fn list_shipping_tpl_rows(
        &self,
        shop_id: i32,
    ) -> Result<Vec<ShippingTplRow>> {
        sqlx::query_as::<_, ShippingTplRow>(
            "SELECT shipping_tpl_id, shipping_tpl_name, shipping_time, is_free, pricing_type, is_default, shop_id
             FROM shipping_tpl WHERE shop_id = 0 OR shop_id = $1 ORDER BY shipping_tpl_id ASC",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_tpl_infos(&self, tpl_id: i32) -> Result<Vec<CloneShippingTplInfo>> {
        sqlx::query_as::<_, CloneShippingTplInfo>(
            "SELECT shipping_type_id, is_free, is_default, region_data, start_number, start_price, add_number, add_price, free_price, pricing_type
             FROM shipping_tpl_info WHERE shipping_tpl_id = $1 ORDER BY id ASC",
        )
        .bind(tpl_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    // ———————————————————— 导出：商品 ————————————————————

    pub async fn list_product_ids(
        &self,
        shop_id: i32,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT product_id FROM product WHERE shop_id = $1 AND is_delete = 0 ORDER BY product_id ASC LIMIT $2 OFFSET $3",
        )
        .bind(shop_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    pub async fn load_product_row(&self, product_id: i32) -> Result<Option<CloneProductRow>> {
        let sql = format!("SELECT {PRODUCT_COLUMNS} FROM product WHERE product_id = $1");
        sqlx::query_as::<_, CloneProductRow>(&sql)
            .bind(product_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    pub async fn load_skus(&self, product_id: i32) -> Result<Vec<CloneSku>> {
        sqlx::query_as::<_, CloneSku>(
            "SELECT sku_id, sku_value, sku_data, sku_sn, sku_stock, sku_tsn, sku_weight, sku_volume, sku_price, is_enabled, is_default, is_inquiry, market_price, cost_price, delivery_service_fee_type, delivery_service_fee_value, server_service_fee_type, server_service_fee_value
             FROM product_sku WHERE product_id = $1 ORDER BY sku_id ASC",
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn load_attributes(&self, product_id: i32) -> Result<Vec<CloneAttribute>> {
        sqlx::query_as::<_, CloneAttribute>(
            "SELECT attr_type, attr_name, attr_value, attr_price, attr_color, attr_pic, attr_pic_thumb, sort_order
             FROM product_attributes WHERE product_id = $1 ORDER BY attr_type ASC, sort_order ASC, attributes_id ASC",
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn load_gallery(&self, product_id: i32) -> Result<Vec<CloneGalleryPic>> {
        sqlx::query_as::<_, CloneGalleryPic>(
            "SELECT pic_url, pic_desc, pic_thumb, pic_original, pic_large, sort_order
             FROM product_gallery WHERE product_id = $1 ORDER BY sort_order ASC, pic_id ASC",
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn load_videos(&self, product_id: i32) -> Result<Vec<CloneVideo>> {
        sqlx::query_as::<_, CloneVideo>(
            "SELECT video_id, video_url, video_cover, format FROM product_video WHERE product_id = $1 ORDER BY id ASC",
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn load_services_by_ids(&self, ids: &[i32]) -> Result<Vec<CloneProductService>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = (1..=ids.len())
            .map(|i| format!("${i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT product_service_id, product_service_name, product_service_desc, ico_img, sort_order, default_on
             FROM product_services WHERE product_service_id IN ({placeholders}) ORDER BY sort_order ASC"
        );
        let mut q = sqlx::query_as::<_, CloneProductService>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    pub async fn find_category_name(&self, category_id: i32) -> Result<Option<(String, i32)>> {
        let row: Option<(String, i32)> = sqlx::query_as(
            "SELECT category_name, parent_id FROM category WHERE category_id = $1",
        )
        .bind(category_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row)
    }

    pub async fn find_brand_name(&self, brand_id: i32) -> Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT brand_name FROM brand WHERE brand_id = $1")
                .bind(brand_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|(n,)| n))
    }

    pub async fn find_shipping_tpl_name(&self, tpl_id: i32) -> Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT shipping_tpl_name FROM shipping_tpl WHERE shipping_tpl_id = $1")
                .bind(tpl_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|(n,)| n))
    }

    // ———————————————————— 导入：结构域对齐 ————————————————————

    pub async fn find_category_by_name_parent(
        &self,
        name: &str,
        parent_id: i32,
    ) -> Result<Option<i32>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT category_id FROM category WHERE category_name = $1 AND parent_id = $2 ORDER BY category_id ASC LIMIT 1",
        )
        .bind(name)
        .bind(parent_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(id,)| id))
    }

    pub async fn insert_category(&self, c: &CloneCategory, parent_id: i32) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO category (category_name, seo_identification, keywords, category_desc, parent_id, sort_order, measure_unit, is_show, seo_title, short_name, category_pic, category_ico, is_hot, search_keywords)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) RETURNING category_id",
        )
        .bind(&c.category_name)
        .bind(&c.seo_identification)
        .bind(&c.keywords)
        .bind(&c.category_desc)
        .bind(parent_id)
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
        Ok(row.0)
    }

    pub async fn find_brand_by_name(&self, name: &str) -> Result<Option<i32>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT brand_id FROM brand WHERE brand_name = $1 ORDER BY shop_id ASC, brand_id ASC LIMIT 1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(id,)| id))
    }

    pub async fn insert_brand(&self, b: &CloneBrand, shop_id: i32) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO brand (brand_name, brand_logo, brand_desc, site_url, sort_order, is_show, brand_type, brand_is_hot, first_word, is_store_brand, check_status, brand_en_name, shop_id, status, reject_remark)
             VALUES ($1, $2, $3, $4, $5, $6, '', 0, $7, 0, 1, $8, $9, 1, '') RETURNING brand_id",
        )
        .bind(&b.brand_name)
        .bind(&b.brand_logo)
        .bind(&b.brand_desc)
        .bind(&b.site_url)
        .bind(b.sort_order)
        .bind(b.is_show)
        .bind(&b.first_word)
        .bind(&b.brand_en_name)
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    pub async fn find_shipping_type_by_name(
        &self,
        name: &str,
        shop_id: i32,
    ) -> Result<Option<i32>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT shipping_type_id FROM shipping_type WHERE shipping_type_name = $1 AND (shop_id = 0 OR shop_id = $2) ORDER BY shop_id ASC, shipping_type_id ASC LIMIT 1",
        )
        .bind(name)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(id,)| id))
    }

    pub async fn insert_shipping_type(
        &self,
        t: &nextrade_core::models::site_clone::clone::CloneShippingType,
        shop_id: i32,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO shipping_type (shipping_type_name, shipping_default_id, shipping_type_desc, shipping_time_desc, is_support_cod, sort_order, shop_id)
             VALUES ($1, 0, $2, $3, $4, $5, $6) RETURNING shipping_type_id",
        )
        .bind(&t.shipping_type_name)
        .bind(&t.shipping_type_desc)
        .bind(&t.shipping_time_desc)
        .bind(t.is_support_cod)
        .bind(t.sort_order)
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    pub async fn find_shipping_tpl_by_name(
        &self,
        name: &str,
        shop_id: i32,
    ) -> Result<Option<i32>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT shipping_tpl_id FROM shipping_tpl WHERE shipping_tpl_name = $1 AND (shop_id = 0 OR shop_id = $2) ORDER BY shop_id ASC, shipping_tpl_id ASC LIMIT 1",
        )
        .bind(name)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(id,)| id))
    }

    pub async fn insert_shipping_tpl(&self, name: &str, shipping_time: &str, is_free: i32, pricing_type: i32, is_default: i32, shop_id: i32) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO shipping_tpl (shipping_time, shipping_tpl_name, is_free, pricing_type, is_default, shop_id)
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING shipping_tpl_id",
        )
        .bind(shipping_time)
        .bind(name)
        .bind(is_free)
        .bind(pricing_type)
        .bind(is_default)
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    pub async fn insert_shipping_tpl_info(
        &self,
        tpl_id: i32,
        shipping_type_id: i32,
        info: &CloneShippingTplInfo,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO shipping_tpl_info (shipping_type_id, shipping_tpl_id, is_free, is_default, region_data, start_number, start_price, add_number, add_price, free_price, pricing_type)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(shipping_type_id)
        .bind(tpl_id)
        .bind(info.is_free)
        .bind(info.is_default)
        .bind(&info.region_data)
        .bind(info.start_number)
        .bind(info.start_price)
        .bind(info.add_number)
        .bind(info.add_price)
        .bind(info.free_price)
        .bind(info.pricing_type)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn find_service_by_name(&self, name: &str) -> Result<Option<i32>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT product_service_id FROM product_services WHERE product_service_name = $1 ORDER BY product_service_id ASC LIMIT 1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(id,)| id))
    }

    pub async fn insert_service(&self, s: &CloneProductService, shop_id: i32) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO product_services (product_service_name, product_service_desc, ico_img, sort_order, default_on, shop_id)
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING product_service_id",
        )
        .bind(&s.product_service_name)
        .bind(&s.product_service_desc)
        .bind(&s.ico_img)
        .bind(s.sort_order)
        .bind(s.default_on)
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    // ———————————————————— 导入：素材登记与商品 id 回收 ————————————————————

    /// 商品图片登记为 external 素材（P1 外链直用；转存为 P2）。
    /// `gallery_id = 0` 表示未分组登记，仅作来源留痕，不进素材库分组视图。
    pub async fn register_external_pic(
        &self,
        shop_id: i32,
        pic_url: &str,
        pic_name: &str,
        pic_thumb: &str,
        pic_sort: i32,
        add_time: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO gallery_pic (shop_id, gallery_id, pic_ower_id, pic_url, pic_name, pic_thumb, add_time, vendor_id, pic_sort, source)
             VALUES ($1, 0, 0, $2, $3, $4, $5, 0, $6, 'external')",
        )
        .bind(shop_id)
        .bind(pic_url)
        .bind(pic_name)
        .bind(pic_thumb)
        .bind(add_time)
        .bind(pic_sort)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn find_product_id_by_sn(&self, sn: &str) -> Result<Option<i32>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT product_id FROM product WHERE product_sn = $1 AND is_delete = 0 ORDER BY product_id DESC LIMIT 1",
        )
        .bind(sn)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(id,)| id))
    }

    /// 无货号商品在 create 后回收新 id：同店铺同名最新一条（导入串行执行，
    /// 竞态面可忽略；仅作 SN 缺失时的兜底）。
    pub async fn find_latest_product_id(&self, shop_id: i32, name: &str) -> Result<Option<i32>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT product_id FROM product WHERE shop_id = $1 AND product_name = $2 AND is_delete = 0 ORDER BY product_id DESC LIMIT 1",
        )
        .bind(shop_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(id,)| id))
    }
}

/// 运费模板主行（仓储内部用）。
#[derive(Debug, Clone, Default, FromRow)]
#[sqlx(rename_all = "snake_case")]
pub struct ShippingTplRow {
    pub shipping_tpl_id: i32,
    pub shipping_tpl_name: String,
    pub shipping_time: String,
    pub is_free: i32,
    pub pricing_type: i32,
    pub is_default: i32,
    pub shop_id: i32,
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}
