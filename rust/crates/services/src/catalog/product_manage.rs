//! 商品域服务（切片 catalog::product_manage）：管理端商品 CRUD、批量修改、导入模板下载，
//! 前台商品详情。
//!
//! 对应原 Java：
//! - `com.nextrade.service.product.impl.ProductServiceImpl`
//!   （`list` 289-711、`detail` 1112-1260、`create` 1646-1792、
//!   `update` 2176-2306、`buildProduct` 1797-1861、`saveProductSku` 1459-1611、
//!   `saveProductAttr` 1954-1998、`saveProductGallery` 2056-2089、
//!   `saveProductArticle` 2130-2154、`clientDetail` 3520-）
//! - `com.nextrade.service.product.impl.ProductBatchServiceImpl`
//!   （`downloadTemplate` 102-119、`productBatchModify` 235-360、
//!   `productBatchEdit` 1181-1200）
//!
//! 迁移要点（逐条对照 Java）：
//! - `create` / `update` 的校验顺序与提示语原文一致（商品名称/价格/货号、
//!   SKU 启用状态、SKU 汇总）
//! - `update` 采用「先按 ID 读库 → 用入参非空字段覆盖 → 全列回写」，
//!   等价于 Java `BeanUtils.copyProperties` + `updateById` 的
//!   `FieldStrategy.NOT_NULL` 语义（入参未传的字段保持库中原值）
//! - SKU：新增（`skuId` 为空）/ 删除（库中启用但未提交）/ 编辑三分支，
//!   空 SKU 列表等价「删除全部启用规格」；新增规格 `skuSn` 为空时
//!   自动生成 `productSn + "-" + skuId`
//! - 相册：`imgList` 非空时「先删（shop_id=0）后插」；为空时**不动**库
//!   （Java `saveProductGallery` 的 early return）
//!
//! **未迁移（刻意不编造，依赖其它域就绪后补齐）**：
//! - 预售配置 `presaleConfig` / `product_presale` 读写
//! - 供应商、多门店 O2O（`store_product` 覆盖价/库存、`nearestShopId`）
//! - 秒杀/拼团/限时折扣/批发/积分兑换/周期购/礼品卡/预约/付费内容等
//!   营销与会员域字段（VO 中保留字段但恒为 `None`）
//! - 商品自动上下架任务 `product_auto_status`、库存调整记录、
//!   保证金校验、ES 同步、审核消息、多语言翻译
//! - 图片尺寸后缀（Java `galleryPicService.getImageSize()` 配置）
//! - `ProductBatchServiceImpl#productBatchModify` 的 Excel 解析
//!   （本 crate 未启用 `calamine`，见 [`ProductManageService::batch_modify`]）

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_common::utils;
use nextrade_core::entities::{
    Brand, Category, Product, ProductArticle, ProductAttributes, ProductGallery,
    ProductInventoryLog, ProductSku, ProductVideo,
};
use nextrade_core::models::catalog::product_manage::{
    AdminProductDetailVO, AdminProductListQuery, BrandOptionVO, CatAndBrandVO, CategoryOptionVO,
    ClientProductDetailVO, ClientProductInfoVO, GalleryPicDTO, ProductAttrGroupVO,
    ProductAttributeVO, ProductAttrsVO, ProductBatchEditDTO, ProductBatchModifyVO, ProductDescVO,
    ProductGalleryVO, ProductListItemVO, ProductSaveDTO, ProductSkuSaveDTO, ProductSkuVO,
    ProductVideoVO, ServicesVO,
};
use nextrade_core::models::catalog::product_sale::ProductSaleState;
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::catalog::product_manage::ProductListFilter;
use nextrade_infrastructure::db::Repositories;

// ————————————————————— 提示语（与原 Java 常量逐字一致） —————————————————————

/// `ProductConstants.PRODUCT_NAME_IS_NULL`
const ERR_NAME_EMPTY: &str = "商品名称不能为空";
/// `ProductConstants.PRODUCT_NAME_OVER_LENGTH`
const ERR_NAME_TOO_LONG: &str = "商品名称不能超过200个字符";
/// `ProductConstants.PRODUCT_SN_EXIST`
const ERR_SN_EXIST: &str = "商品货号已存在";
/// `ProductConstants.PRODUCT_PRICE_LE_0`
const ERR_PRICE: &str = "商品售价不能小于0";
/// `applyEnabledSkuSummary` 中价格缺失时的提示
const ERR_PRICE_EMPTY: &str = "商品价格不能为空";
/// `ProductConstants.PRODUCT_NOT_EXIST`
const ERR_NOT_EXIST: &str = "商品不存在";
/// `normalizeSkuStatus`
const ERR_SKU_STATUS: &str = "规格状态不正确";
/// `normalizeAndValidateSkuStatus`
const ERR_SKU_AT_LEAST_ONE: &str = "请至少启用一个SKU";
/// `saveProductSku` 编辑分支
const ERR_SKU_NOT_EXIST: &str = "商品规格不存在";
/// `saveProductSku` 新增分支
const ERR_SKU_VALUE_EXISTS: &str = "该规格已存在，请重新启用";
/// `productBatchModify` 入参校验
const ERR_FILE_EMPTY: &str = "请上传文件";
/// `downloadTemplate` 读取失败
const ERR_TEMPLATE: &str = "商品导入模板文件不存在或读取失败";
/// `create` 成功文案（平台账号分支）
const MSG_CREATED: &str = "商品已添加";
/// `update` 成功文案（平台账号分支）
const MSG_UPDATED: &str = "商品管理更新成功";
/// `update` 失败文案
const MSG_UPDATE_FAIL: &str = "商品管理更新失败";

/// 导入模板（多商户）磁盘路径候选：Java 侧为
/// `classpath:template/多商户商品导入模板.xlsx`。
const TEMPLATE_MERCHANT: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../../java/nextrade-adminapi/src/main/resources/template/多商户商品导入模板.xlsx");
/// 导入模板（O2O）磁盘路径候选。
const TEMPLATE_O2O: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../../java/nextrade-adminapi/src/main/resources/template/o2o商品导入模板.xlsx");
/// 下载文件名（多商户），对应 Java `DOWNLOAD_NAME_MERCHANT`。
const DOWNLOAD_NAME_MERCHANT: &str = "商品导入模板.xlsx";
/// 下载文件名（O2O），对应 Java `DOWNLOAD_NAME_O2O`。
const DOWNLOAD_NAME_O2O: &str = "o2o商品导入模板.xlsx";

/// 商品域服务。
pub struct ProductManageService {
    repos: Arc<Repositories>,
}

impl ProductManageService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 管理端：列表 —————————————————————

    /// 管理端商品分页（对应 `ProductServiceImpl#list`）。
    ///
    /// 已实现过滤项见 [`ProductListFilter`]；N+1 优化：规格列表按整页
    /// 商品 ID 批量查询后在内存分组（Java 侧为逐商品查询）。
    pub async fn admin_list(
        &self,
        q: &AdminProductListQuery,
    ) -> Result<PageResult<ProductListItemVO>> {
        let page = q.page_query();
        let filter = self.build_list_filter(q, &page).await?;
        let (total, rows) = self.repos.catalog.product_manage.paginate_products(&filter).await?;

        let ids: Vec<i32> = rows.iter().map(|p| p.product_id).collect();
        let skus = self
            .repos.catalog.product_manage
            .find_skus_by_product_ids(&ids, false)
            .await?;
        let mut sku_map: HashMap<i32, Vec<ProductSku>> = HashMap::new();
        for s in skus {
            sku_map.entry(s.product_id).or_default().push(s);
        }

        // v5.8.61 列表返回最终可售状态与手动设置，后台据此展示禁售标签。
        let mut sale_map: HashMap<i32, ProductSaleState> = HashMap::new();
        for p in &rows {
            let state = self
                .repos
                .catalog
                .product_sale
                .setting(p.product_id, None)
                .await?;
            sale_map.insert(p.product_id, state);
        }

        let list = rows
            .iter()
            .map(|p| {
                let skus = sku_map.get(&p.product_id).cloned().unwrap_or_default();
                to_list_item(p, &skus, sale_map.get(&p.product_id))
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 归一化列表查询条件（对应 Java `buildQueryWrapper` 的可迁移部分）。
    async fn build_list_filter(
        &self,
        q: &AdminProductListQuery,
        page: &PageQuery,
    ) -> Result<ProductListFilter> {
        let keyword_like = q
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|k| format!("%{k}%"));

        // Java：categoryId > 0 时按「本分类 + 全部子孙分类」过滤
        let category_ids = match q.category_id.filter(|v| *v > 0) {
            Some(root) => {
                let cats = self.repos.catalog.product_manage.find_categories().await?;
                let pairs: Vec<(i32, i32)> =
                    cats.iter().map(|c| (c.category_id, c.parent_id)).collect();
                descendant_category_ids(&pairs, root)
            }
            None => Vec::new(),
        };

        // Java：productGroupId > 0 时用 product_group.product_ids 过滤，
        // 分组内无商品则用 `product_id = -1` 让结果为空
        let group_product_ids = match q.product_group_id.filter(|v| *v > 0) {
            Some(gid) => {
                let raw = self
                    .repos.catalog.product_manage
                    .find_group_product_ids(gid)
                    .await?
                    .unwrap_or_default();
                Some(parse_id_json_lossy(&raw))
            }
            None => None,
        };

        let sort_field = if q.sort_by_total_sales == Some(true)
            || q.sort_by_thirty_day_sales == Some(true)
        {
            Some("virtualSales")
        } else {
            q.sort_field.as_deref()
        };

        let (limit, offset) = page.limit_offset();
        let mut id_list = q.id_list.clone().unwrap_or_default();
        if id_list.is_empty() {
            if let Some(raw) = q.ids.as_deref() {
                id_list = raw
                    .split(',')
                    .filter_map(|s| s.trim().parse::<i32>().ok())
                    .collect();
            }
        }
        if id_list.is_empty() {
            if let Some(ids) = &q.product_ids {
                id_list = ids.clone();
            }
        }
        Ok(ProductListFilter {
            keyword_like,
            id_list,
            // Java：isDelete 合法值（0/1）用之，否则强制 0
            is_delete: match q.is_delete {
                Some(v) if v == 0 || v == 1 => v,
                _ => 0,
            },
            // Java：shopId == -1 表示全部店铺（跳过该条件）
            shop_id: q.shop_id.filter(|v| *v != -1),
            product_status: q.product_status,
            check_status: q.check_status,
            product_id: q.product_id,
            // Java：productType > 0 才生效
            product_type: q.product_type.filter(|v| *v > 0),
            product_types: q.product_types.clone().unwrap_or_default(),
            exclude_product_types: q.exclude_product_types.clone().unwrap_or_default(),
            category_ids,
            // Java：brandId >= 0 生效（0 表示「无品牌」也要匹配）
            brand_id: q.brand_id.filter(|v| *v >= 0),
            brand_ids: q
                .parsed_brand_ids()
                .map_err(|e| Error::biz(400, e))?
                .unwrap_or_default(),
            shop_category_id: q.shop_category_id.filter(|v| *v > 0),
            extra_product_ids: q.extra_product_ids.clone().unwrap_or_default(),
            group_product_ids,
            min_price: q.min_price,
            max_price: q.max_price,
            min_sales: q.min_sales,
            max_sales: q.max_sales,
            added_start_time: q.added_start_time,
            added_end_time: q.added_end_time,
            min_stock: q.min_stock,
            max_stock: q.max_stock,
            // Java：isNew > -1 才生效
            is_new: q.is_new.filter(|v| *v > -1),
            suppliers_id: q.suppliers_id.filter(|v| *v > 0),
            sale_mode: q.sale_mode,
            field_order_ids: q.parsed_ids().map_err(|e| Error::biz(400, e))?.unwrap_or_default(),
            order_column: resolve_order_column(sort_field),
            order_asc: q
                .sort_order
                .as_deref()
                .is_some_and(|s| s.eq_ignore_ascii_case("asc")),
            limit,
            offset,
        })
    }

    // ————————————————————— 管理端：详情 —————————————————————

    /// 管理端商品详情（对应 `ProductServiceImpl#detail`）。
    ///
    /// Java 在商品不存在时返回 `null`（HTTP 200 + `data: null`），
    /// 因此这里返回 `Option`。
    pub async fn admin_detail(&self, product_id: i32) -> Result<Option<AdminProductDetailVO>> {
        let Some(product) = self.repos.catalog.product_manage.find_product(product_id).await? else {
            return Ok(None);
        };
        // Java：detail 每次访问 clickCount + 1 并回写
        self.repos.catalog.product_manage.inc_click_count(product_id).await?;

        let gallery = self.repos.catalog.product_manage.find_gallery(product_id).await?;
        let videos = self.repos.catalog.product_manage.find_videos(product_id).await?;
        let attrs = self.repos.catalog.product_manage.find_attributes(product_id).await?;
        let article_ids = self.repos.catalog.product_manage.find_article_ids(product_id).await?;
        // Java：`getProductSkusByPid(productId, true)` —— 详情包含禁用规格
        let skus = self.repos.catalog.product_manage.find_skus(product_id, true).await?;

        Ok(Some(to_admin_detail(
            &product,
            &skus,
            &gallery,
            &videos,
            &attrs,
            &article_ids,
        )))
    }

    // ————————————————————— 管理端：分类 + 品牌下拉 —————————————————————

    /// 分类 + 品牌下拉（`GET /adminapi/product/product/catAndBrandList`）。
    ///
    /// **不确定点**：该方法在现存 Java 源码中不存在（全仓无
    /// `catAndBrandList`），这里按「商品编辑表单所需下拉数据」实现：
    /// 全量分类（id/名称/父级/排序/显示）+ 显示中的品牌。
    pub async fn cat_and_brand_list(&self) -> Result<CatAndBrandVO> {
        let cats = self.repos.catalog.product_manage.find_categories().await?;
        let brands = self.repos.catalog.product_manage.find_brands(Some(1)).await?;
        Ok(CatAndBrandVO {
            cat_list: cats.iter().map(to_category_option).collect(),
            brand_list: brands.iter().map(to_brand_option).collect(),
        })
    }

    // ————————————————————— 管理端：创建 —————————————————————

    /// 创建商品（对应 `ProductServiceImpl#create`）。
    ///
    /// 返回提示语文本（Java 直接返回 `String`，Spring 包装为 `data`）。
    pub async fn create(&self, dto: &ProductSaveDTO) -> Result<String> {
        // 1. 名称 / 价格 / 货号校验（与 Java 同序同文案）
        validate_product_name(dto.product_name.as_deref())?;
        validate_product_price(dto.product_price)?;
        if let Some(sn) = non_empty(dto.product_sn.as_deref()) {
            if self.repos.catalog.product_manage.count_product_sn(sn, None).await? > 0 {
                return Err(Error::biz(400, ERR_SN_EXIST));
            }
        }

        // 2. SKU 校验 + 汇总（Java `normalizeAndValidateSkuStatus` +
        //    `applyEnabledSkuSummary`）
        let (stock, price) = resolve_save_summary(dto)?;

        let now = utils::now_seconds() as i32;
        let mut product = Product::default();
        apply_dto_to_product(&mut product, dto.clone(), true);
        product.product_stock = stock;
        product.product_price = price;
        product.add_time = now;
        product.last_update = now;
        sync_free_shipping_with_shipping_type(&mut product);

        let product_id = self.repos.catalog.product_manage.insert_product(&product).await?;
        product.product_id = product_id;

        // 3. 子表（SKU / 相册 / 属性 / 视频 / 关联文章）
        self.sync_skus(&product, dto.product_list.as_deref()).await?;
        self.sync_gallery(&product, dto.img_list.as_deref()).await?;
        self.sync_attributes(&product, dto.attr_list.as_ref())
            .await?;
        self.sync_videos(&product, dto.product_video_info.as_deref())
            .await?;
        self.sync_articles(&product, dto.product_article_list.as_deref())
            .await?;

        // 4. 单规格商品写一条库存日志（Java `create` 中
        //    `CollUtil.isEmpty(param.getProductList())` 分支）
        if dto
            .product_list
            .as_ref()
            .is_none_or(|l| l.is_empty())
        {
            let log = ProductInventoryLog {
                product_id: Some(product_id),
                spec_id: Some(0),
                number: Some(product.product_stock),
                add_time: Some(now),
                old_number: Some(0),
                r#type: Some(1),
                change_number: Some(product.product_stock),
                desc: Some("库存新增".into()),
                shop_id: product.shop_id,
                ..Default::default()
            };
            self.repos.catalog.product_manage.insert_inventory_log(&log).await?;
        }

        let log_info = format!("创建商品:{}", product.product_name);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        let sid = product.shop_id;
        if let Err(e) = admin_log
            .record_current_log(if sid > 0 { "shop" } else { "admin" }, sid, &log_info)
            .await
        {
            tracing::warn!("写管理操作日志失败（创建商品）: {e}");
        }

        tracing::info!("创建商品成功: product_id={product_id}, sn={:?}", product.product_sn);
        Ok(MSG_CREATED.to_string())
    }

    // ————————————————————— 管理端：更新 —————————————————————

    /// 更新商品（对应 `ProductServiceImpl#update`）。
    pub async fn update(&self, dto: &ProductSaveDTO) -> Result<String> {
        let product_id = dto
            .product_id
            .ok_or_else(|| Error::biz(400, "商品ID不能为空"))?;
        let mut db = self
            .repos.catalog.product_manage
            .find_product(product_id)
            .await?
            .ok_or_else(|| Error::biz(400, ERR_NOT_EXIST))?;

        // 多租户隔离：商户账号禁止越权修改其他商户商品
        if let Some(target_shop_id) = dto.shop_id {
            if target_shop_id > 0 && db.shop_id != target_shop_id && db.shop_id != 0 {
                return Err(Error::biz(403, "无权操作其他商户商品"));
            }
        }

        // 1. 名称 / 价格校验（入参未传时保持库中原值，不触发校验）
        if dto.product_name.is_some() {
            validate_product_name(dto.product_name.as_deref())?;
        }
        if dto.product_price.is_some() {
            validate_product_price(dto.product_price)?;
        }

        // 2. 货号唯一（Java：自身已占用同一货号时跳过校验）
        if let Some(sn) = non_empty(dto.product_sn.as_deref()) {
            if db.product_sn != sn && self.repos.catalog.product_manage.count_product_sn(sn, None).await? > 0 {
                return Err(Error::biz(400, ERR_SN_EXIST));
            }
        }

        // 3. 入参非空字段覆盖（等价 MyBatis-Plus NOT_NULL 更新策略），
        //    随后按 SKU 汇总改写价格/库存（Java：先
        //    `applyEnabledSkuSummary(param)` 再 copy，等价于覆盖在最后）
        let old_stock = db.product_stock;
        let single_spec = dto
            .product_list
            .as_ref()
            .is_none_or(|l| l.is_empty());
        apply_dto_to_product(&mut db, dto.clone(), false);
        if single_spec {
            if let Some(stock) = dto.product_stock {
                db.product_stock = stock;
            }
        } else {
            let (stock, price) = resolve_save_summary(dto)?;
            db.product_stock = stock;
            db.product_price = price;
        }
        db.last_update = utils::now_seconds() as i32;
        sync_free_shipping_with_shipping_type(&mut db);

        let affected = self.repos.catalog.product_manage.update_product(&db).await?;
        if affected == 0 {
            return Ok(MSG_UPDATE_FAIL.to_string());
        }

        // 5. 子表
        self.sync_skus(&db, dto.product_list.as_deref()).await?;
        self.sync_gallery(&db, dto.img_list.as_deref()).await?;
        self.sync_attributes(&db, dto.attr_list.as_ref()).await?;
        self.sync_videos(&db, dto.product_video_info.as_deref())
            .await?;
        self.sync_articles(&db, dto.product_article_list.as_deref())
            .await?;

        // 6. 单规格库存变化日志（Java `update` 中的 diff 分支）
        if single_spec {
            let diff = db.product_stock - old_stock;
            if diff != 0 {
                let log = ProductInventoryLog {
                    product_id: Some(product_id),
                    spec_id: Some(0),
                    number: Some(db.product_stock),
                    add_time: Some(utils::now_seconds() as i32),
                    old_number: Some(old_stock),
                    r#type: Some(if diff > 0 { 1 } else { 2 }),
                    change_number: Some(diff),
                    desc: Some("库存编辑".into()),
                    shop_id: db.shop_id,
                    ..Default::default()
                };
                self.repos.catalog.product_manage.insert_inventory_log(&log).await?;
            }
        }

        let log_info = format!("更新商品:{}", db.product_name);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_current_log(
                if db.shop_id > 0 { "shop" } else { "admin" },
                db.shop_id,
                &log_info,
            )
            .await
        {
            tracing::warn!("写管理操作日志失败（更新商品）: {e}");
        }

        tracing::info!("更新商品成功: product_id={product_id}");
        Ok(MSG_UPDATED.to_string())
    }

    // ————————————————————— 管理端：删除 —————————————————————

    /// 删除商品（软删除 `is_delete = 1`）。
    ///
    /// **不确定点**：契约中的 `POST /adminapi/product/product/del` 在现存
    /// Java 源码中不存在（`ProductController` 只有 `/batch`、`/recycle`），
    /// 这里按「软删除」实现（与原系统回收站语义一致，不做物理删除）。
    pub async fn delete(&self, ids: &[i32]) -> Result<u64> {
        self.delete_scoped(ids, None).await
    }

    /// 删除商品（带租户店铺隔离）。
    pub async fn delete_scoped(&self, ids: &[i32], shop_id: Option<i32>) -> Result<u64> {
        if ids.is_empty() {
            return Err(Error::biz(400, "商品ID不能为空"));
        }
        let affected = self
            .repos
            .catalog
            .product_manage
            .soft_delete_products_scoped(ids, shop_id)
            .await?;
        tracing::info!("软删除商品 {} 个 (shop_id: {:?})", affected, shop_id);
        Ok(affected)
    }


    // ————————————————————— 管理端：批量修改 —————————————————————

    /// 商品批量修改（对应 `ProductBatchServiceImpl#productBatchEdit`）。
    ///
    /// 与原 Java 一致：逐行「货号重复校验」→ 逐行全列回写
    /// （`updateBatchById` 的非空字段更新语义），库中不存在的行静默跳过。
    pub async fn batch_edit(&self, dto: &ProductBatchEditDTO) -> Result<bool> {
        self.batch_edit_scoped(dto, None).await
    }

    /// 商品批量修改（支持多商户租户隔离）。
    pub async fn batch_edit_scoped(
        &self,
        dto: &ProductBatchEditDTO,
        scoped_shop_id: Option<i32>,
    ) -> Result<bool> {
        let items = dto.edit_items.clone().unwrap_or_default();
        if items.is_empty() {
            // Java：`CollUtil.isNotEmpty(dto.getEditItems())` 不成立时直接返回 true
            return Ok(true);
        }

        // 1. 货号重复校验（排除自身）
        for item in &items {
            if let Some(sn) = non_empty(item.product_sn.as_deref()) {
                let exclude = item.product_id;
                if self.repos.catalog.product_manage.count_product_sn(sn, exclude).await? > 0 {
                    return Err(Error::biz(400, format!("{ERR_SN_EXIST}：{sn}")));
                }
            }
        }

        // 2. 逐行回写
        for item in &items {
            let Some(product_id) = item.product_id else {
                return Err(Error::biz(400, "商品ID不能为空"));
            };
            let Some(mut product) = self.repos.catalog.product_manage.find_product(product_id).await? else {
                tracing::warn!("批量修改跳过不存在的商品: product_id={product_id}");
                continue;
            };
            if let Some(sid) = scoped_shop_id {
                if product.shop_id != sid {
                    return Err(Error::biz(403, "无权修改其他商家的商品"));
                }
            }
            apply_batch_item(&mut product, item);
            self.repos.catalog.product_manage.update_product(&product).await?;
        }

        tracing::info!("商品批量修改完成: {} 行", items.len());
        Ok(true)
    }

    // ————————————————————— 管理端：导入 —————————————————————

    /// 下载商品导入模板（对应 `ProductBatchServiceImpl#downloadTemplate`）。
    ///
    /// Java 直接流式输出 classpath 下的 xlsx（`application/vnd.
    /// openxmlformats-officedocument.spreadsheetml.sheet`）。
    ///
    /// **不确定点**：O2O 开关（Java `nextradeProperties.getIsO2o()`）在 Rust
    /// 侧尚未有配置项，这里读环境变量 `TIG_IS_O2O`（=1 走 O2O 模板），
    /// 默认多商户模板；模板文件按磁盘路径读取（运行时缺失则报
    /// 「商品导入模板文件不存在或读取失败」，与 Java 文案一致）。
    pub async fn download_template(&self) -> Result<(String, Vec<u8>)> {
        let o2o = std::env::var("TIG_IS_O2O").is_ok_and(|v| v.trim() == "1");
        let (path, name) = if o2o {
            (TEMPLATE_O2O, DOWNLOAD_NAME_O2O)
        } else {
            (TEMPLATE_MERCHANT, DOWNLOAD_NAME_MERCHANT)
        };
        // 候选路径：显式环境变量 → 相对路径 → 编译期基准路径
        let mut candidates: Vec<String> = Vec::new();
        if let Ok(p) = std::env::var("TIG_PRODUCT_IMPORT_TEMPLATE") {
            if !p.trim().is_empty() {
                candidates.push(p);
            }
        }
        let actual_filename = if o2o { "o2o商品导入模板.xlsx" } else { "多商户商品导入模板.xlsx" };
        candidates.push(format!("template/{actual_filename}"));
        candidates.push(format!("rust/template/{actual_filename}"));
        candidates.push(format!("../template/{actual_filename}"));
        candidates.push(format!("java/nextrade-adminapi/src/main/resources/template/{actual_filename}"));
        candidates.push(path.to_string());
        if o2o {
            candidates.push(TEMPLATE_MERCHANT.to_string());
        }

        for candidate in candidates {
            match tokio::fs::read(&candidate).await {
                Ok(bytes) if !bytes.is_empty() => {
                    tracing::info!("商品导入模板读取成功: {candidate} ({} 字节)", bytes.len());
                    return Ok((name.to_string(), bytes));
                }
                Ok(_) => tracing::warn!("商品导入模板为空文件: {candidate}"),
                Err(e) => tracing::debug!("商品导入模板候选路径不可用 {candidate}: {e}"),
            }
        }
        Err(Error::biz(500, ERR_TEMPLATE))
    }

    /// 商品导入（对应 `ProductBatchServiceImpl#productBatchModify`）。
    ///
    /// 通过 `calamine` 解析上传的 Excel xlsx 二进制数据并逐组按商品录入/更新。
    pub async fn batch_modify(
        &self,
        is_auto_cat: Option<i32>,
        is_auto_brand: Option<i32>,
        is_change_pic: Option<i32>,
        is_upload: Option<i32>,
        file_name: Option<&str>,
        file_bytes: &[u8],
    ) -> Result<ProductBatchModifyVO> {
        self.batch_modify_scoped(
            is_auto_cat,
            is_auto_brand,
            is_change_pic,
            is_upload,
            file_name,
            file_bytes,
            None,
        )
        .await
    }

    /// 商品导入（支持多商户租户隔离）。
    pub async fn batch_modify_scoped(
        &self,
        is_auto_cat: Option<i32>,
        _is_auto_brand: Option<i32>,
        is_change_pic: Option<i32>,
        _is_upload: Option<i32>,
        file_name: Option<&str>,
        file_bytes: &[u8],
        shop_id: Option<i32>,
    ) -> Result<ProductBatchModifyVO> {
        if file_bytes.is_empty() {
            return Err(Error::biz(400, ERR_FILE_EMPTY));
        }

        tracing::info!(
            "开始处理 Excel 商品导入: file={:?}, size={} 字节, shop_id={:?}",
            file_name,
            file_bytes.len(),
            shop_id
        );

        // 1. 调用系统公共模块读取 Excel 文件
        let raw_rows = match crate::system::excel::read_xlsx(file_bytes) {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!("Excel 解析失败: {e}");
                return Err(Error::biz(400, "读取Excel文件失败"));
            }
        };

        if raw_rows.is_empty() || raw_rows.len() <= 1 {
            return Ok(ProductBatchModifyVO {
                add_num: 0,
                update_num: 0,
                fail_num: 0,
                error_msg: Some("请上传有数据的文件".to_string()),
                error_list: Vec::new(),
            });
        }

        // 2. 表头位置解析
        let header = &raw_rows[0];
        let find_col = |name: &str| -> Option<usize> {
            header.iter().position(|h| h.trim() == name)
        };
        let c_sn = find_col("商品编码").unwrap_or(0);
        let c_name = find_col("商品名称").unwrap_or(1);
        let c_pic = find_col("商品图片").unwrap_or(2);
        let c_cat = find_col("商品分类").unwrap_or(3);
        let c_status = find_col("是否上架（默认1）")
            .or_else(|| find_col("是否上架"))
            .unwrap_or(6);
        let c_spec = find_col("商品规格").unwrap_or(7);
        let c_sku_sn = find_col("规格编码").unwrap_or(9);
        let c_sku_tsn = find_col("规格条码").unwrap_or(10);
        let c_price = find_col("商品售价").unwrap_or(12);
        let c_stock = find_col("库存").unwrap_or(13);
        let c_market = find_col("市场价").unwrap_or(14);
        let c_tsn = find_col("商品条码").unwrap_or(15);
        let c_keywords = find_col("搜索关键词").unwrap_or(16);
        let c_brief = find_col("商品描述")
            .or_else(|| find_col("商品简介"))
            .unwrap_or(17);
        let c_cost = find_col("成本价").unwrap_or(18);

        let cell = |r: &[String], idx: usize| -> String {
            r.get(idx).map(|s| s.trim()).unwrap_or("").to_string()
        };

        // 3. 数据行抽取与按商品分组
        let mut grouped: Vec<(String, Vec<ParsedRowData>)> = Vec::new();
        let mut group_map: HashMap<String, usize> = HashMap::new();
        let mut error_list: Vec<String> = Vec::new();
        let mut fail_num = 0;

        for (row_idx, row) in raw_rows.iter().enumerate().skip(1) {
            if row.iter().all(|c| c.trim().is_empty()) {
                continue;
            }
            let sn = cell(row, c_sn);
            let name = cell(row, c_name);
            if sn.is_empty() && name.is_empty() {
                fail_num += 1;
                error_list.push(format!("第 {} 行：商品编码和商品名称均为空", row_idx + 1));
                continue;
            }
            if sn.len() > 50 {
                fail_num += 1;
                error_list.push(format!("第 {} 行：商品编码超过50个字符", row_idx + 1));
                continue;
            }
            let key = if !sn.is_empty() {
                format!("SN:{sn}")
            } else {
                format!("NAME:{name}")
            };

            let row_data = ParsedRowData {
                product_sn: sn,
                product_name: name,
                pic_url: cell(row, c_pic),
                category_name: cell(row, c_cat),
                product_status: cell(row, c_status),
                spec_name: cell(row, c_spec),
                sku_sn: cell(row, c_sku_sn),
                sku_tsn: cell(row, c_sku_tsn),
                product_price: cell(row, c_price),
                product_stock: cell(row, c_stock),
                market_price: cell(row, c_market),
                product_tsn: cell(row, c_tsn),
                keywords: cell(row, c_keywords),
                product_brief: cell(row, c_brief),
                cost_price: cell(row, c_cost),
            };

            if let Some(&idx) = group_map.get(&key) {
                grouped[idx].1.push(row_data);
            } else {
                let idx = grouped.len();
                group_map.insert(key.clone(), idx);
                grouped.push((key, vec![row_data]));
            }
        }

        // 4. 加载基础字典与分类
        let categories = self.repos.catalog.product_manage.find_categories().await?;
        let now = utils::now_seconds() as i32;

        let mut add_num = 0;
        let mut update_num = 0;

        for (group_idx, (_key, rows)) in grouped.into_iter().enumerate() {
            let first = &rows[0];
            let product_sn = if !first.product_sn.is_empty() {
                first.product_sn.clone()
            } else {
                format!("SN{}", utils::now_millis())
            };

            if first.product_name.is_empty() {
                fail_num += 1;
                error_list.push(format!("第 {} 组({})：商品名称为空", group_idx + 1, product_sn));
                continue;
            }

            // 查询库中是否已存在
            let existing = self.repos.catalog.product_manage.find_product_by_sn(&product_sn).await?;
            if let Some(mut exist) = existing {
                // 多商户租户隔离
                if let Some(sid) = shop_id {
                    if sid > 0 && exist.shop_id != sid && exist.shop_id != 0 {
                        fail_num += 1;
                        error_list.push(format!("第 {} 组({})：商品属于其他店铺，无权修改", group_idx + 1, product_sn));
                        continue;
                    }
                }

                // 更新商品字段
                if !first.product_name.is_empty() {
                    exist.product_name = first.product_name.clone();
                }
                if let Ok(price) = first.product_price.parse::<Decimal>() {
                    exist.product_price = price;
                }
                if let Ok(stock) = first.product_stock.parse::<i32>() {
                    exist.product_stock = stock;
                }
                if is_change_pic == Some(1) && !first.pic_url.is_empty() {
                    exist.pic_url = first.pic_url.clone();
                    exist.pic_thumb = first.pic_url.clone();
                    exist.pic_original = first.pic_url.clone();
                }
                exist.last_update = now;

                if let Err(e) = self.repos.catalog.product_manage.update_product(&exist).await {
                    fail_num += 1;
                    error_list.push(format!("第 {} 组({})：更新失败: {e}", group_idx + 1, product_sn));
                    continue;
                }
                update_num += 1;
            } else {
                // 新建商品
                let cat_id = resolve_cat_id(&categories, &first.category_name);
                if cat_id == 0 && is_auto_cat != Some(1) {
                    fail_num += 1;
                    error_list.push(format!("第 {} 组({})：分类【{}】不存在", group_idx + 1, product_sn, first.category_name));
                    continue;
                }

                let price = first.product_price.parse::<Decimal>().unwrap_or(Decimal::ZERO);
                let stock = first.product_stock.parse::<i32>().unwrap_or(0);
                let market = first.market_price.parse::<Decimal>().unwrap_or(price * Decimal::from_str_exact("1.2").unwrap_or(Decimal::ONE));
                let cost = first.cost_price.parse::<Decimal>().ok();

                let opt_non_empty = |s: &str| {
                    let trimmed = s.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                };

                let mut product = Product {
                    product_sn: product_sn.clone(),
                    product_name: first.product_name.clone(),
                    product_tsn: opt_non_empty(&first.product_tsn),
                    category_id: cat_id,
                    brand_id: 0,
                    shop_id: shop_id.unwrap_or(0),
                    product_price: price,
                    market_price: market,
                    cost_price: cost,
                    product_stock: stock,
                    pic_url: first.pic_url.clone(),
                    pic_thumb: first.pic_url.clone(),
                    pic_original: first.pic_url.clone(),
                    product_brief: first.product_brief.clone(),
                    keywords: opt_non_empty(&first.keywords),
                    product_status: first.product_status.parse::<i32>().unwrap_or(1),
                    check_status: 1,
                    add_time: now,
                    last_update: now,
                    ..Default::default()
                };

                let product_id = match self.repos.catalog.product_manage.insert_product(&product).await {
                    Ok(id) => id,
                    Err(e) => {
                        fail_num += 1;
                        error_list.push(format!("第 {} 组({})：创建商品记录失败: {e}", group_idx + 1, product_sn));
                        continue;
                    }
                };
                product.product_id = product_id;

                // 写入商品相册
                if !first.pic_url.is_empty() {
                    let gallery = ProductGallery {
                        product_id,
                        pic_url: first.pic_url.clone(),
                        pic_thumb: first.pic_url.clone(),
                        pic_original: first.pic_url.clone(),
                        pic_large: first.pic_url.clone(),
                        sort_order: 0,
                        shop_id: product.shop_id as i64,
                        ..Default::default()
                    };
                    let _ = self.repos.catalog.product_manage.insert_gallery(&gallery).await;
                }

                // 写入规格 SKU
                let has_sku = rows.len() > 1 || !first.spec_name.is_empty() || !first.sku_sn.is_empty();
                if has_sku {
                    for r in &rows {
                        let sku_price = r.product_price.parse::<Decimal>().unwrap_or(price);
                        let sku_stock = r.product_stock.parse::<i32>().unwrap_or(stock);
                        let sku = ProductSku {
                            product_id,
                            sku_sn: opt_non_empty(&r.sku_sn),
                            sku_tsn: opt_non_empty(&r.sku_tsn),
                            sku_value: r.spec_name.clone(),
                            sku_price: Some(sku_price),
                            sku_stock: Some(sku_stock),
                            is_enabled: 1,
                            market_price: Some(r.market_price.parse::<Decimal>().unwrap_or(market)),
                            cost_price: r.cost_price.parse::<Decimal>().ok(),
                            ..Default::default()
                        };
                        let _ = self.repos.catalog.product_manage.insert_sku(&sku).await;
                    }
                }

                add_num += 1;
            }
        }

        let error_msg = if error_list.is_empty() {
            Some("上传完成".to_string())
        } else {
            Some(format!("上传完成，{} 组成功，{} 组失败", add_num + update_num, fail_num))
        };

        Ok(ProductBatchModifyVO {
            add_num,
            update_num,
            fail_num,
            error_msg,
            error_list,
        })
    }


    // ————————————————————— 前台：商品详情 —————————————————————

    /// 按商品 ID 反查货号（对应 Java `ProductServiceImpl#getProductSnById`，
    /// 供 `GET /api/product/product/detail?id=` 使用）。
    pub async fn product_sn_by_id(&self, product_id: i32) -> Result<Option<String>> {
        Ok(self
            .repos.catalog.product_manage
            .find_product(product_id)
            .await?
            .map(|p| p.product_sn))
    }

    /// 前台商品详情（对应 `ProductServiceImpl#clientDetail`）。
    ///
    /// `shop_id` / `nearest_shop_id` 为 O2O 门店上下文，本切片未迁移
    /// （仅记录日志），商品主数据一律取平台图与平台库存。
    pub async fn client_detail(
        &self,
        sn: &str,
        shop_id: Option<i32>,
        nearest_shop_id: Option<i32>,
    ) -> Result<ClientProductDetailVO> {
        if shop_id.is_some() || nearest_shop_id.is_some() {
            tracing::debug!(
                "前台商品详情携带门店上下文 shop_id={shop_id:?} nearest_shop_id={nearest_shop_id:?}（O2O 未迁移，按平台数据返回）"
            );
        }
        let sn = sn.trim();
        if sn.is_empty() {
            return Err(Error::biz(400, ERR_NOT_EXIST));
        }
        let product = self
            .repos.catalog.product_manage
            .find_product_by_sn(sn)
            .await?
            .ok_or_else(|| Error::biz(400, ERR_NOT_EXIST))?;

        // Java：`isDelete == 1 || productStatus != 1` → PRODUCT_NOT_EXIST
        if product.is_delete == 1 || product.product_status != 1 {
            return Err(Error::biz(400, ERR_NOT_EXIST));
        }

        let product_id = product.product_id;
        // v5.8.61 商品禁售：搜索/列表/详情使用同一套读时判定。
        let sale_state = self
            .repos
            .catalog
            .product_sale
            .setting(product_id, shop_id)
            .await?;
        if sale_state.sale_status == 0 {
            return Err(Error::biz(400, ERR_NOT_EXIST));
        }
        let mut product = product;
        let mut skus = self.repos.catalog.product_manage.find_skus(product_id, false).await?;

        // O2O：请求带门店（shopId 优先，其次 nearestShopId）时用 store_product /
        // store_product_sku 覆盖商品名、价格、库存、主图、详情和店铺归属。
        let store_shop_id = shop_id
            .filter(|v| *v > 0)
            .or_else(|| nearest_shop_id.filter(|v| *v > 0));
        if let Some(store_shop_id) = store_shop_id {
            if let Some(store) = self
                .repos
                .catalog
                .product_stock
                .find_store_product(product_id, store_shop_id)
                .await?
            {
                if let Some(name) = store.product_name.as_deref().filter(|s| !s.is_empty()) {
                    product.product_name = name.to_string();
                }
                if let Some(price) = store.product_price {
                    product.product_price = price;
                }
                if let Some(stock) = store.product_stock {
                    product.product_stock = stock;
                }
                product.product_status = store.product_status;
                product.shop_id = i32::try_from(store.shop_id).unwrap_or(product.shop_id);
                if let Some(pic) = store.pic_url.as_deref().filter(|s| !s.is_empty()) {
                    product.pic_url = pic.to_string();
                }
                if let Some(desc) = store.product_desc.as_deref().filter(|s| !s.is_empty()) {
                    product.product_desc = Some(desc.to_string());
                }
                for sku in &mut skus {
                    if let Some(store_sku) = self
                        .repos
                        .catalog
                        .product_stock
                        .find_store_product_sku(store.store_product_id, sku.sku_id)
                        .await?
                    {
                        if let Some(price) = store_sku.sku_price {
                            sku.sku_price = Some(price);
                        }
                        if let Some(stock) = store_sku.sku_stock {
                            sku.sku_stock = Some(stock);
                        }
                    }
                }
            }
        }
        // 门店商品下架时按商品不存在处理（避免把门店下架商品露出）
        if product.product_status != 1 {
            return Err(Error::biz(400, ERR_NOT_EXIST));
        }
        let skus = skus;
        let gallery = self.repos.catalog.product_manage.find_gallery(product_id).await?;
        let videos = self.repos.catalog.product_manage.find_videos(product_id).await?;
        let attrs = self.repos.catalog.product_manage.find_attributes(product_id).await?;
        let service_ids = parse_id_json_lossy(product.product_service_ids.as_deref().unwrap_or(""));
        let services = self
            .repos.catalog.product_manage
            .find_services_by_ids(&service_ids)
            .await?;

        let sku_list: Vec<ProductSkuVO> = skus.iter().map(to_sku_vo).collect();
        let attr_list = group_attributes(&attrs);
        let delivery_mode = product.delivery_mode;

        Ok(ClientProductDetailVO {
            shop_id: product.shop_id,
            item: to_client_info(&product),
            desc_arr: desc_arr_from_html(product.product_desc.as_deref().unwrap_or("")),
            sku_list,
            pic_list: gallery.iter().map(to_gallery_vo).collect(),
            video_list: videos.iter().map(to_video_vo).collect(),
            attr_list,
            // 以下字段依赖评论 / 秒杀 / 拼团 / 积分 / 批发域，未迁移，保持空
            rank_detail: None,
            seckill_detail: Vec::new(),
            groupon_detail: None,
            service_list: services.iter().map(to_services_vo).collect(),
            checked_value: Vec::new(),
            consultation_total: None,
            exchange_info: None,
            delivery_mode,
            delivery_mode_text: if delivery_mode == 2 {
                "周期购".to_string()
            } else {
                "普通商品".to_string()
            },
            wholesale_price_list: None,
            wholesale_price_mode: None,
        })
    }

    // ————————————————————— 子表同步（内部） —————————————————————

    /// SKU 同步（对应 Java `saveProductSku`）。
    ///
    /// - `None` / 空列表：删除该商品全部**启用**规格（Java 语义）
    /// - 新增：`skuId` 为空的行；`skuSn` 为空时生成 `productSn-skuId`
    /// - 删除：库中启用但未在提交列表中的行
    /// - 编辑：提交了 `skuId` 的行；库中原为禁用或状态发生变化时
    ///   **保留库中原值**只改启用状态（Java `prepareSkuForUpdate`）
    /// - 收尾：按启用规格重算商品价格/库存汇总
    async fn sync_skus(
        &self,
        product: &Product,
        submitted: Option<&[ProductSkuSaveDTO]>,
    ) -> Result<()> {
        let product_id = product.product_id;
        let Some(list) = submitted else {
            self.repos.catalog.product_manage.delete_enabled_skus(product_id).await?;
            return Ok(());
        };
        if list.is_empty() {
            self.repos.catalog.product_manage.delete_enabled_skus(product_id).await?;
            return Ok(());
        }
        validate_sku_list(list)?;

        let db_skus = self.repos.catalog.product_manage.find_skus(product_id, true).await?;
        let db_map: HashMap<i32, ProductSku> =
            db_skus.iter().map(|s| (s.sku_id, s.clone())).collect();
        let disabled_values: HashSet<String> = db_skus
            .iter()
            .filter(|s| s.is_enabled == 0)
            .filter_map(|s| Some(s.sku_value.clone()))
            .collect();
        let now = utils::now_seconds() as i32;

        // — 新增 —
        let adds: Vec<&ProductSkuSaveDTO> = list.iter().filter(|s| s.sku_id.is_none()).collect();
        if !adds.is_empty() {
            if adds.iter().any(|s| {
                s.sku_value
                    .as_deref()
                    .is_some_and(|v| disabled_values.contains(v))
            }) {
                return Err(Error::biz(400, ERR_SKU_VALUE_EXISTS));
            }
            for item in adds {
                let mut sku = build_sku_from_dto(item, product_id, None)?;
                let sku_id = self.repos.catalog.product_manage.insert_sku(&sku).await?;
                if sku.sku_sn.as_deref().is_none_or(str::is_empty) {
                    let generated = format!("{}-{sku_id}", product.product_sn);
                    self.repos.catalog.product_manage.update_sku_sn(sku_id, &generated).await?;
                    sku.sku_sn = Some(generated);
                }
                let log = ProductInventoryLog {
                    product_id: Some(product_id),
                    spec_id: Some(sku_id),
                    number: sku.sku_stock,
                    add_time: Some(now),
                    old_number: Some(0),
                    r#type: Some(1),
                    change_number: sku.sku_stock,
                    desc: Some("新增产品规格".into()),
                    shop_id: product.shop_id,
                    ..Default::default()
                };
                self.repos.catalog.product_manage.insert_inventory_log(&log).await?;
            }
        }

        // — 删除 —
        let keep: HashSet<i32> = list.iter().filter_map(|s| s.sku_id).collect();
        let dels: Vec<i32> = db_skus
            .iter()
            .filter(|s| s.is_enabled != 0 && !keep.contains(&s.sku_id))
            .map(|s| s.sku_id)
            .collect();
        if !dels.is_empty() {
            for sku_id in &dels {
                let old = db_map.get(sku_id).and_then(|s| s.sku_stock).unwrap_or(0);
                let log = ProductInventoryLog {
                    product_id: Some(product_id),
                    spec_id: Some(*sku_id),
                    number: Some(0),
                    add_time: Some(now),
                    old_number: Some(old),
                    r#type: Some(1),
                    change_number: Some(-old),
                    desc: Some("删除产品规格".into()),
                    shop_id: product.shop_id,
                    ..Default::default()
                };
                self.repos.catalog.product_manage.insert_inventory_log(&log).await?;
            }
            self.repos.catalog.product_manage.delete_skus(&dels).await?;
        }

        // — 编辑 —
        for item in list.iter().filter(|s| s.sku_id.is_some()) {
            let sku_id = item.sku_id.unwrap_or_default();
            let db_sku = db_map
                .get(&sku_id)
                .ok_or_else(|| Error::biz(400, ERR_SKU_NOT_EXIST))?;
            let target_status = normalize_sku_enabled(item.is_enabled)?;
            let merged = if db_sku.is_enabled == 0 || target_status != db_sku.is_enabled {
                // 原为禁用 / 状态变化：保留库中原值，只改启用状态
                let mut preserved = db_sku.clone();
                preserved.is_enabled = target_status;
                preserved
            } else {
                build_sku_from_dto(item, product_id, Some(db_sku))?
            };
            self.repos.catalog.product_manage.update_sku(&merged).await?;
            let old = db_sku.sku_stock.unwrap_or(0);
            let new = merged.sku_stock.unwrap_or(0);
            if old != new {
                let log = ProductInventoryLog {
                    product_id: Some(product_id),
                    spec_id: Some(sku_id),
                    number: Some(new),
                    add_time: Some(now),
                    old_number: Some(old),
                    r#type: Some(1),
                    change_number: Some(new - old),
                    // 注：原 Java 该分支文案写死为「删除产品规格」（疑为复制粘贴笔误），
                    // 这里按语义写「编辑产品规格」。
                    desc: Some("编辑产品规格".into()),
                    shop_id: product.shop_id,
                    ..Default::default()
                };
                self.repos.catalog.product_manage.insert_inventory_log(&log).await?;
            }
        }

        // — 汇总回写（Java `updateProductSkuSummary`）—
        let saved = self.repos.catalog.product_manage.find_skus(product_id, true).await?;
        if let Some((stock, price)) = summary_from_entities(&saved) {
            self.repos.catalog.product_manage
                .update_product_summary(product_id, stock, price)
                .await?;
        }
        Ok(())
    }

    /// 相册同步（对应 Java `saveProductGallery`）。
    ///
    /// 注意 Java 的 `CollUtil.isEmpty(imgList)` 会 **early return**，
    /// 即「不传图片列表」不会清空相册；只有传了非空列表才整体替换。
    async fn sync_gallery(&self, product: &Product, img_list: Option<&[GalleryPicDTO]>) -> Result<()> {
        let Some(list) = img_list else { return Ok(()) };
        if list.is_empty() {
            return Ok(());
        }
        self.repos.catalog.product_manage.delete_gallery(product.product_id).await?;
        for (idx, pic) in list.iter().enumerate() {
            let pic_url = pic.pic_url.clone().unwrap_or_default();
            let row = ProductGallery {
                product_id: product.product_id,
                pic_url: pic_url.clone(),
                pic_desc: pic.pic_name.clone().unwrap_or_default(),
                pic_thumb: pic.pic_thumb.clone().unwrap_or_else(|| pic_url.clone()),
                pic_original: pic_url.clone(),
                // 未迁移：Java 会拼图片尺寸后缀（galleryPicService 配置）
                pic_large: pic_url.clone(),
                sort_order: i32::try_from(idx).unwrap_or(i32::MAX),
                shop_id: 0,
                ..Default::default()
            };
            self.repos.catalog.product_manage.insert_gallery(&row).await?;
        }
        Ok(())
    }

    /// 属性同步（对应 Java `saveProductAttr`）。
    ///
    /// 展平顺序与 Java 一致：`extra` → `spe` → `normal`，`sortOrder` 取
    /// 展平后的下标；带 `attributesId` 的走更新、其余新增，
    /// 并删除不在提交集合中的历史属性。
    async fn sync_attributes(
        &self,
        product: &Product,
        attr_list: Option<&ProductAttrsVO>,
    ) -> Result<()> {
        let Some(attrs) = attr_list else { return Ok(()) };
        let product_id = product.product_id;
        let flattened: Vec<&ProductAttributeVO> = attrs
            .extra
            .iter()
            .chain(attrs.spe.iter())
            .chain(attrs.normal.iter())
            .flat_map(|g| g.attr_list.iter())
            .collect();

        if flattened.is_empty() {
            self.repos.catalog.product_manage
                .delete_attributes_except(product_id, &[])
                .await?;
            return Ok(());
        }

        let keep_ids: Vec<i32> = flattened
            .iter()
            .filter_map(|a| a.attributes_id)
            .collect();
        self.repos.catalog.product_manage
            .delete_attributes_except(product_id, &keep_ids)
            .await?;

        for (idx, a) in flattened.iter().enumerate() {
            let row = ProductAttributes {
                attributes_id: a.attributes_id.unwrap_or_default(),
                product_id,
                attr_type: a.attr_type,
                attr_name: a.attr_name.clone(),
                attr_value: a.attr_value.clone(),
                attr_price: a.attr_price,
                attr_color: a.attr_color.clone(),
                attr_pic: a.attr_pic.clone(),
                attr_pic_thumb: a.attr_pic_thumb.clone(),
                sort_order: Some(i32::try_from(idx).unwrap_or(i32::MAX)),
            };
            if a.attributes_id.is_some() {
                self.repos.catalog.product_manage.update_attribute(&row).await?;
            } else {
                self.repos.catalog.product_manage.insert_attribute(&row).await?;
            }
        }
        Ok(())
    }

    /// 视频同步（对应 Java `otherUpdates` 中的视频分支：
    /// 先清空该商品全部视频，再按入参写入；入参为空也清空）。
    async fn sync_videos(&self, product: &Product, videos: Option<&[ProductVideoVO]>) -> Result<()> {
        let product_id = product.product_id;
        self.repos.catalog.product_manage.delete_videos(product_id).await?;
        let Some(list) = videos else { return Ok(()) };
        for v in list {
            let row = ProductVideo {
                id: 0,
                // 前端把图库视频 ID 放在 id 字段，Java 转存到 video_id
                video_id: v.video_id.or(v.id).unwrap_or(0),
                product_id,
                video_url: v.video_url.clone().unwrap_or_default(),
                video_cover: v.video_cover.clone().unwrap_or_default(),
                format: v.format.clone().unwrap_or_default(),
            };
            self.repos.catalog.product_manage.insert_video(&row).await?;
        }
        Ok(())
    }

    /// 关联文章同步（对应 Java `saveProductArticle`：先删同名文章关联，
    /// 再批量插入；入参为空不做任何操作）。
    async fn sync_articles(&self, product: &Product, articles: Option<&[i32]>) -> Result<()> {
        let Some(list) = articles else { return Ok(()) };
        if list.is_empty() {
            return Ok(());
        }
        let product_id = product.product_id;
        self.repos.catalog.product_manage
            .delete_articles_in(product_id, list)
            .await?;
        for article_id in list {
            self.repos.catalog.product_manage
                .insert_article(&ProductArticle {
                    goods_id: product_id,
                    article_id: *article_id,
                })
                .await?;
        }
        Ok(())
    }
}

// ————————————————————— 纯逻辑（可单测） —————————————————————

/// 分类子孙展开（对应 Java `CategoryService#getCategoryAllChildIds`）。
///
/// 返回「自身 + 全部子孙」，按层级 BFS 顺序去重。
/// `all` 为 `(category_id, parent_id)` 全量分类。
pub fn descendant_category_ids(all: &[(i32, i32)], root: i32) -> Vec<i32> {
    let mut out = vec![root];
    let mut seen: HashSet<i32> = HashSet::from([root]);
    let mut frontier = vec![root];
    while let Some(current) = frontier.pop() {
        for (id, parent) in all {
            if *parent == current && !seen.contains(id) {
                seen.insert(*id);
                out.push(*id);
                frontier.push(*id);
            }
        }
    }
    out
}

/// 解析 `[1,2,3]` 形式的 JSON 数字数组；非法内容返回空列表。
///
/// 对应 Java `StringUtils.str2IntList` 与
/// `productGroupService.detail(id).getProductIds()` 的宽松解析。
pub fn parse_id_json_lossy(raw: &str) -> Vec<i32> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(arr) = v.as_array() {
            return arr.iter().filter_map(|x| x.as_i64()).filter_map(|n| i32::try_from(n).ok()).collect();
        }
        if let Some(n) = v.as_i64() {
            return i32::try_from(n).map(|v| vec![v]).unwrap_or_default();
        }
    }
    // 退化为逗号分隔
    trimmed
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<i32>().ok())
        .collect()
}

/// 图文详情 HTML → 片段数组（对应 Java `ProductDescDTO.getProductDescArr`）。
///
/// 以 `<div data-division=1></div>` 分割；含 `desc-video-item` 的片段
/// 取 `data-cover` / `data-video`，含 `desc-pic-item` 的取 `<img src>`，
/// 其余按 `text` 处理。
pub fn desc_arr_from_html(html: &str) -> Vec<ProductDescVO> {
    if html.trim().is_empty() {
        return Vec::new();
    }
    const DIVISION: &str = "<div data-division=1></div>";
    html.split(DIVISION)
        .map(|value| {
            if value.contains("desc-video-item") {
                ProductDescVO {
                    r#type: "video".into(),
                    cover: extract_quoted_attr(value, "data-cover"),
                    video_url: extract_quoted_attr(value, "data-video"),
                    html: Some(value.to_string()),
                    ..Default::default()
                }
            } else if value.contains("desc-pic-item") {
                ProductDescVO {
                    r#type: "pic".into(),
                    pic: extract_img_src(value),
                    html: Some(value.to_string()),
                    ..Default::default()
                }
            } else {
                ProductDescVO {
                    r#type: "text".into(),
                    html: Some(value.to_string()),
                    ..Default::default()
                }
            }
        })
        .collect()
}

/// 片段数组 → 图文详情 HTML（对应 Java `ProductDescDTO.getProductDesc`）。
pub fn desc_html_from_arr(arr: &[ProductDescVO]) -> String {
    let parts: Vec<String> = arr
        .iter()
        .filter_map(|item| match item.r#type.as_str() {
            "pic" => Some(format!(
                "<div class=\"desc-pic-item\"><img src=\"{}\"></div>",
                item.pic.clone().unwrap_or_default()
            )),
            "video" => Some(format!(
                "<div class=\"desc-video-item\" data-cover=\"{}\" data-video=\"{}\"></div>",
                item.cover.clone().unwrap_or_default(),
                item.video_url.clone().unwrap_or_default()
            )),
            "text" => item.html.clone(),
            _ => None,
        })
        .filter(|s| !s.is_empty())
        .collect();
    parts.join("<div data-division=1></div>")
}

/// 商品属性行 → 分组结构（对应 Java
/// `ProductAttributesServiceImpl#getProductAttributes`）。
///
/// `attr_type`：0 普通、1 规格、2 附加；组内按 `attr_name` 聚合，
/// 组顺序保持库中 `sort_order` 顺序。
pub fn group_attributes(rows: &[ProductAttributes]) -> ProductAttrsVO {
    let mut normal: Vec<&ProductAttributes> = Vec::new();
    let mut spe: Vec<&ProductAttributes> = Vec::new();
    let mut extra: Vec<&ProductAttributes> = Vec::new();
    for row in rows {
        match row.attr_type {
            1 => spe.push(row),
            2 => extra.push(row),
            _ => normal.push(row),
        }
    }
    ProductAttrsVO {
        normal: group_by_name(&normal),
        spe: group_by_name(&spe),
        extra: group_by_name(&extra),
    }
}

/// 按 `attr_name` 聚合（保持首次出现顺序）。
fn group_by_name(rows: &[&ProductAttributes]) -> Vec<ProductAttrGroupVO> {
    let mut order: Vec<String> = Vec::new();
    let mut map: HashMap<String, Vec<ProductAttributeVO>> = HashMap::new();
    for row in rows {
        let name = row.attr_name.clone();
        if !map.contains_key(&name) {
            order.push(name.clone());
        }
        map.entry(name).or_default().push(ProductAttributeVO {
            attributes_id: Some(row.attributes_id),
            product_id: Some(row.product_id),
            attr_type: row.attr_type,
            attr_name: row.attr_name.clone(),
            attr_value: row.attr_value.clone(),
            attr_price: row.attr_price,
            attr_color: row.attr_color.clone(),
            attr_pic: row.attr_pic.clone(),
            attr_pic_thumb: row.attr_pic_thumb.clone(),
            sort_order: row.sort_order,
        });
    }
    order
        .into_iter()
        .map(|name| {
            let attr_list = map.remove(&name).unwrap_or_default();
            ProductAttrGroupVO {
                attr_name: name,
                attr_list,
            }
        })
        .collect()
}

/// 规格类型判定（对应 Java `resolveSkuType`）：
/// 有规格属性或 SKU 多于 1 条 → 2（多规格），否则 1（统一规格）。
pub fn resolve_sku_type(attrs: &ProductAttrsVO, sku_count: usize) -> i32 {
    if !attrs.spe.is_empty() || sku_count > 1 {
        2
    } else {
        1
    }
}

/// 列表排序字段白名单（对应 Java `getColumnByPropertyName`：命中属性名或
/// 列名则用之，否则回落主键 `product_id`，防注入）。
///
/// 未列入的字段（依赖其它域或当前列表不展示的列）一律回落主键。
#[must_use]
pub fn resolve_order_column(sort_field: Option<&str>) -> &'static str {
    let Some(field) = sort_field.map(str::trim).filter(|s| !s.is_empty()) else {
        return "product_id";
    };
    match field {
        "productId" | "product_id" => "product_id",
        "productName" | "product_name" => "product_name",
        "productSn" | "product_sn" => "product_sn",
        "productPrice" | "product_price" => "product_price",
        "marketPrice" | "market_price" => "market_price",
        "productStock" | "product_stock" => "product_stock",
        "productStatus" | "product_status" => "product_status",
        "checkStatus" | "check_status" => "check_status",
        "categoryId" | "category_id" => "category_id",
        "brandId" | "brand_id" => "brand_id",
        "shopId" | "shop_id" => "shop_id",
        "addTime" | "add_time" => "add_time",
        "sortOrder" | "sort_order" => "sort_order",
        "storeSortOrder" | "store_sort_order" => "store_sort_order",
        "virtualSales" | "virtual_sales" => "virtual_sales",
        "clickCount" | "click_count" => "click_count",
        "isBest" | "is_best" => "is_best",
        "isNew" | "is_new" => "is_new",
        "isHot" | "is_hot" => "is_hot",
        "lastUpdate" | "last_update" => "last_update",
        "productWeight" | "product_weight" => "product_weight",
        "limitNumber" | "limit_number" => "limit_number",
        _ => "product_id",
    }
}

/// 规格启用状态归一化（对应 Java `normalizeSkuStatus`）：
/// `null` → 1（启用）；仅接受 0/1，其余报「规格状态不正确」。
pub fn normalize_sku_enabled(is_enabled: Option<i32>) -> Result<i32> {
    match is_enabled {
        None => Ok(1),
        Some(0) => Ok(0),
        Some(1) => Ok(1),
        Some(_) => Err(Error::biz(400, ERR_SKU_STATUS)),
    }
}

/// SKU 列表校验（对应 Java `normalizeAndValidateSkuStatus`）。
pub fn validate_sku_list(list: &[ProductSkuSaveDTO]) -> Result<()> {
    if list.is_empty() {
        return Ok(());
    }
    let mut has_enabled = false;
    for item in list {
        let status = normalize_sku_enabled(item.is_enabled)?;
        if status == 1 {
            has_enabled = true;
        }
    }
    if !has_enabled {
        return Err(Error::biz(400, ERR_SKU_AT_LEAST_ONE));
    }
    Ok(())
}

/// 启用规格汇总（对应 Java `applyEnabledSkuSummary`）：
/// 返回 `(库存合计, 最低价)`；无启用规格价格时返回「商品价格不能为空」。
pub fn enabled_sku_summary(list: &[ProductSkuSaveDTO]) -> Result<(i32, Decimal)> {
    let mut stock = 0_i32;
    let mut price: Option<Decimal> = None;
    for item in list {
        if normalize_sku_enabled(item.is_enabled)? != 1 {
            continue;
        }
        stock += item.sku_stock.unwrap_or(0);
        if let Some(p) = item.sku_price {
            price = Some(match price {
                Some(cur) if cur <= p => cur,
                _ => p,
            });
        }
    }
    price
        .map(|p| (stock, p))
        .ok_or_else(|| Error::biz(400, ERR_PRICE_EMPTY))
}

/// 创建 / 更新的最终 `(库存, 价格)`：
/// 多规格由启用规格汇总，单规格直接取入参。
pub fn resolve_save_summary(dto: &ProductSaveDTO) -> Result<(i32, Decimal)> {
    match dto.product_list.as_ref() {
        Some(list) if !list.is_empty() => {
            validate_sku_list(list)?;
            enabled_sku_summary(list)
        }
        _ => Ok((dto.product_stock.unwrap_or(0), dto.product_price.unwrap_or(Decimal::ZERO))),
    }
}

/// 已落库规格的汇总（对应 Java `updateProductSkuSummary`）：
/// 无启用规格时返回 `None`（Java 此时不更新商品汇总字段）。
pub fn summary_from_entities(skus: &[ProductSku]) -> Option<(i32, Decimal)> {
    let mut stock = 0_i32;
    let mut price: Option<Decimal> = None;
    for sku in skus.iter().filter(|s| s.is_enabled == 1) {
        stock += sku.sku_stock.unwrap_or(0);
        if let Some(p) = sku.sku_price {
            price = Some(match price {
                Some(cur) if cur <= p => cur,
                _ => p,
            });
        }
    }
    price.map(|p| (stock, p))
}

/// `free_shipping` 与运费方式对齐（对应 Java
/// `syncFreeShippingWithShippingType`）。
pub fn sync_free_shipping_with_shipping_type(product: &mut Product) {
    let Some(kind) = product.fixed_shipping_type else {
        return;
    };
    match kind {
        2 => product.free_shipping = 0,
        1 => {
            let is_free = product
                .fixed_shipping_fee
                .is_none_or(|fee| fee == Decimal::ZERO);
            product.free_shipping = if is_free { 1 } else { 0 };
        }
        _ => {}
    }
}

/// 商品名称校验（对应 `@NotBlank` + `@Size(max=200)`）。
pub fn validate_product_name(name: Option<&str>) -> Result<()> {
    let Some(name) = name.map(str::trim).filter(|s| !s.is_empty()) else {
        return Err(Error::biz(400, ERR_NAME_EMPTY));
    };
    if name.chars().count() > 200 {
        return Err(Error::biz(400, ERR_NAME_TOO_LONG));
    }
    Ok(())
}

/// 商品价格校验（对应 `checkProductPrice`：为空或 < 0 都报同一文案）。
pub fn validate_product_price(price: Option<Decimal>) -> Result<()> {
    match price {
        Some(p) if p >= Decimal::ZERO => Ok(()),
        _ => Err(Error::biz(400, ERR_PRICE)),
    }
}

/// URL 编码（对应 Java `URLEncoder.encode(name, UTF-8).replace("+", "%20")`，
/// 用于 `Content-Disposition` 的中文文件名）。
#[must_use]
pub fn percent_encode_utf8(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for byte in raw.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'*' => {
                out.push(*byte as char);
            }
            b' ' => out.push_str("%20"),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// 从 HTML 片段里取 `<img ... src="...">`。
fn extract_img_src(html: &str) -> Option<String> {
    extract_quoted_attr(html, "src")
}

/// 从 HTML 片段里取 `attr="value"`（同时兼容单引号）。
fn extract_quoted_attr(html: &str, attr: &str) -> Option<String> {
    let mut search_from = 0;
    while let Some(pos) = html[search_from..].find(attr) {
        let abs = search_from + pos;
        let rest = &html[abs + attr.len()..];
        let rest_trimmed = rest.trim_start();
        if let Some(after_eq) = rest_trimmed.strip_prefix('=') {
            let after_eq = after_eq.trim_start();
            let quote = after_eq.chars().next()?;
            if quote == '"' || quote == '\'' {
                let body = &after_eq[quote.len_utf8()..];
                if let Some(end) = body.find(quote) {
                    return Some(body[..end].to_string());
                }
            }
        }
        search_from = abs + attr.len();
    }
    None
}

/// 非空字符串（\"None\" / 空白 → \"None\"）。
fn non_empty(raw: Option<&str>) -> Option<&str> {
    raw.map(str::trim).filter(|s| !s.is_empty())
}

// ————————————————————— DTO → 实体 —————————————————————

/// 入参覆盖到商品实体。
///
/// `is_create = true` 时未传字段按 `pure.sql` 的 DDL 默认值补齐
/// （因为仓储层是全列 INSERT，不能依赖库默认值）；
/// `is_create = false` 时只覆盖入参**已传**的字段
/// （等价 MyBatis-Plus `FieldStrategy.NOT_NULL`）。
fn apply_dto_to_product(product: &mut Product, dto: ProductSaveDTO, is_create: bool) {
    /// 实体字段非 \"Option\" 时直接赋值。
    macro_rules! set_opt {
        ($field:ident, $value:expr) => {
            if let Some(v) = $value {
                product.$field = v;
            }
        };
    }
    /// 实体字段是 `Option<T>` 时包一层。
    macro_rules! set_some {
        ($field:ident, $value:expr) => {
            if let Some(v) = $value {
                product.$field = Some(v);
            }
        };
    }

    set_some!(product_type, dto.product_type);
    set_opt!(sale_mode, dto.sale_mode);
    set_opt!(delivery_mode, dto.delivery_mode);
    set_opt!(product_stock, dto.product_stock);
    set_opt!(free_shipping, dto.free_shipping);
    set_opt!(shop_category_id, dto.shop_category_id);
    set_opt!(product_status, dto.product_status);
    set_opt!(give_integral, dto.give_integral);
    set_opt!(rank_integral, dto.rank_integral);
    set_opt!(integral, dto.integral);
    set_opt!(product_weight, dto.product_weight);
    set_some!(product_volume, dto.product_volume);
    set_some!(seo_identification, dto.seo_identification);
    set_opt!(product_sn, dto.product_sn);
    set_some!(product_tsn, dto.product_tsn);
    set_opt!(product_price, dto.product_price);
    set_opt!(market_price, dto.market_price);
    set_some!(cost_price, dto.cost_price);
    // \"shipping_tpl_id\"：Java DTO 为 Long，库列为 mediumint（i32）
    if let Some(v) = dto.shipping_tpl_id {
        product.shipping_tpl_id = i32::try_from(v).unwrap_or(product.shipping_tpl_id);
    }
    set_opt!(category_id, dto.category_id);
    set_opt!(brand_id, dto.brand_id);
    set_opt!(shop_id, dto.shop_id);
    set_some!(keywords, dto.keywords);
    set_opt!(check_status, dto.check_status);
    set_opt!(check_reason, dto.check_reason);
    set_opt!(click_count, dto.click_count);
    set_opt!(promote_price, dto.promote_price);
    set_opt!(seckill_max_num, dto.seckill_max_num);
    set_opt!(product_brief, dto.product_brief);
    set_opt!(pic_url, dto.pic_url);
    set_opt!(pic_thumb, dto.pic_thumb);
    set_opt!(pic_original, dto.pic_original);
    set_some!(comment_tag, dto.comment_tag);
    set_opt!(sort_order, dto.sort_order);
    set_opt!(store_sort_order, dto.store_sort_order);
    set_opt!(is_delete, dto.is_delete);
    set_opt!(is_best, dto.is_best);
    set_opt!(is_new, dto.is_new);
    set_opt!(is_hot, dto.is_hot);
    set_some!(fixed_shipping_type, dto.fixed_shipping_type);
    set_some!(fixed_shipping_fee, dto.fixed_shipping_fee);
    set_opt!(remark, dto.remark);
    set_some!(suppliers_id, dto.suppliers_id);
    set_opt!(virtual_sales, dto.virtual_sales);
    set_opt!(limit_number, dto.limit_number);
    set_some!(product_care, dto.product_care);
    set_opt!(is_support_return, dto.is_support_return);
    set_opt!(is_support_cod, dto.is_support_cod);
    set_some!(product_video, dto.product_video);
    set_some!(prepay_price, dto.prepay_price);
    set_opt!(card_group_id, dto.card_group_id);
    set_some!(virtual_sample, dto.virtual_sample);
    set_some!(no_shipping, dto.no_shipping);
    set_opt!(is_shop_pickup, dto.is_shop_pickup);
    set_opt!(is_logistics, dto.is_logistics);
    set_opt!(is_shop_delivery, dto.is_shop_delivery);
    set_some!(shop_pickup_tpl_id, dto.shop_pickup_tpl_id);
    set_some!(reservation_tpl_id, dto.reservation_tpl_id);
    set_some!(gift_card_groups_id, dto.gift_card_groups_id);
    set_some!(product_unit, dto.product_unit);
    set_some!(attributes_tpl_id, dto.attributes_tpl_id);
    set_opt!(delivery_service_fee_type, dto.delivery_service_fee_type);
    set_opt!(delivery_service_fee_value, dto.delivery_service_fee_value);
    set_opt!(server_service_fee_type, dto.server_service_fee_type);
    set_opt!(server_service_fee_value, dto.server_service_fee_value);

    // 布尔入参 → 0/1
    if let Some(v) = dto.is_promote {
        product.is_promote = i32::from(v);
    }
    if let Some(v) = dto.is_promote_activity {
        product.is_promote_activity = i32::from(v);
    }

    // 名称
    if let Some(v) = &dto.product_name {
        product.product_name = v.clone();
    }

    // 商品描述：优先 \"productDescArr\"（Java `ProductDescDTO.getProductDesc`），
    // 其次直接用 \"productDesc\"
    if let Some(arr) = &dto.product_desc_arr {
        let html = desc_html_from_arr(arr);
        if !html.is_empty() {
            product.product_desc = Some(html);
        }
    } else if let Some(desc) = &dto.product_desc {
        product.product_desc = Some(desc.clone());
    }

    // 关联商品 / 服务项：Java 存 `List.toString()`（`[1, 2]`）格式
    if let Some(list) = &dto.product_related {
        if !list.is_empty() {
            product.product_related = Some(format_int_list(list));
        }
    }
    if let Some(list) = &dto.product_service_ids {
        if !list.is_empty() {
            product.product_service_ids = Some(format_int_list(list));
        }
    }

    // 付费内容（JSON 文本）
    if let Some(v) = &dto.paid_content {
        product.paid_content = serde_json::to_string(v).ok();
    }

    // 促销起止：Java 入参为 `yyyy-MM-dd HH:mm:ss` 字符串，库中为秒级时间戳
    if let Some(ts) = dto.promote_start_date.as_deref().and_then(parse_datetime_seconds) {
        product.promote_start_date = ts;
    }
    if let Some(ts) = dto.promote_end_date.as_deref().and_then(parse_datetime_seconds) {
        product.promote_end_date = ts;
    }

    // 主图：Java \"setImgList\" 取相册第一张
    if let Some(img) = dto.img_list.as_ref().and_then(|l| l.first()) {
        if let Some(url) = non_empty(img.pic_url.as_deref()) {
            product.pic_url = url.to_string();
            product.pic_original = url.to_string();
            product.pic_large = url.to_string();
            product.pic_thumb = img
                .pic_thumb
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| url.to_string());
        }
    }

    if is_create {
        // 以下默认值取自 `java/database/pure.sql` 的 \"product\" DDL，
        // 因为仓储层是全列 INSERT，必须显式补齐
        if dto.product_status.is_none() {
            product.product_status = 1;
        }
        if dto.check_status.is_none() {
            product.check_status = 1;
        }
        if dto.product_type.is_none() {
            product.product_type = Some(1);
        }
        if dto.delivery_mode.is_none() {
            product.delivery_mode = 1;
        }
        if dto.sort_order.is_none() {
            product.sort_order = 100;
        }
        if dto.store_sort_order.is_none() {
            product.store_sort_order = 100;
        }
        if dto.is_logistics.is_none() {
            product.is_logistics = 1;
        }
        if dto.is_support_cod.is_none() {
            product.is_support_cod = 1;
        }
        if dto.brand_id.is_none() {
            product.brand_id = 0;
        }
    }
}

/// 批量修改单行覆盖（对应 Java `ProductBatchEditDTO.buildProducts()`；
/// 后者未使用 \"costPrice\"，此处同样忽略）。
fn apply_batch_item(product: &mut Product, item: &nextrade_core::models::catalog::product_manage::ProductBatchEditItem) {
    if let Some(v) = &item.product_name {
        product.product_name = v.clone();
    }
    if let Some(v) = item.category_id {
        product.category_id = v;
    }
    if let Some(v) = item.brand_id {
        product.brand_id = v;
    }
    if let Some(v) = &item.product_sn {
        product.product_sn = v.clone();
    }
    if let Some(v) = &item.product_tsn {
        product.product_tsn = Some(v.clone());
    }
    if let Some(v) = item.product_price {
        product.product_price = v;
    }
    if let Some(v) = item.market_price {
        product.market_price = v;
    }
    if let Some(v) = item.shipping_tpl_id {
        product.shipping_tpl_id = i32::try_from(v).unwrap_or(product.shipping_tpl_id);
    }
    if let Some(v) = item.free_shipping {
        product.free_shipping = v;
    }
    if let Some(v) = item.is_new {
        product.is_new = v;
    }
    if let Some(v) = item.is_best {
        product.is_best = v;
    }
    if let Some(v) = item.is_hot {
        product.is_hot = v;
    }
    if let Some(v) = item.sort_order {
        product.sort_order = v;
    }
    if let Some(v) = item.product_status {
        product.product_status = v;
    }
    if let Some(ids) = &item.product_service_ids {
        if !ids.is_empty() {
            product.product_service_ids = Some(format_int_list(ids));
        }
    }
}

/// `[1, 2, 3]` 文本（与 Java `List.toString()` 等价）。
fn format_int_list(list: &[i32]) -> String {
    let body = list
        .iter()
        .map(i32::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{body}]")
}

/// `yyyy-MM-dd HH:mm:ss` / `yyyy-MM-dd` → 秒级时间戳。
fn parse_datetime_seconds(raw: &str) -> Option<i32> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let formats = ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M", "%Y-%m-%d"];
    for fmt in formats {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(trimmed, fmt) {
            return i32::try_from(dt.and_utc().timestamp()).ok();
        }
        if let Ok(d) = chrono::NaiveDate::parse_from_str(trimmed, fmt) {
            return i32::try_from(d.and_hms_opt(0, 0, 0)?.and_utc().timestamp()).ok();
        }
    }
    trimmed.parse::<i32>().ok()
}

// ————————————————————— 实体 → VO —————————————————————

/// 商品实体 + 规格 → 列表项（对应 Java \"ProductListResDTO\"）。
fn to_list_item(
    p: &Product,
    skus: &[ProductSku],
    sale: Option<&ProductSaleState>,
) -> ProductListItemVO {
    let sku_vo: Vec<ProductSkuVO> = skus.iter().map(to_sku_vo).collect();
    ProductListItemVO {
        category_id: p.category_id,
        brand_id: p.brand_id,
        product_tsn: p.product_tsn.clone(),
        market_price: p.market_price,
        shipping_tpl_id: i64::from(p.shipping_tpl_id),
        free_shipping: p.free_shipping,
        product_id: p.product_id,
        pic_url: p.pic_url.clone(),
        pic_large: p.pic_large.clone(),
        pic_thumb: p.pic_thumb.clone(),
        product_brief: p.product_brief.clone(),
        product_name: p.product_name.clone(),
        check_status: p.check_status,
        check_reason: p.check_reason.clone(),
        shop_id: p.shop_id,
        suppliers_id: p.suppliers_id,
        product_type: p.product_type,
        sale_mode: p.sale_mode,
        product_sale_mode: i32::from(p.sale_mode > 0),
        product_service_ids: parse_id_json_lossy(p.product_service_ids.as_deref().unwrap_or("")),
        product_sn: p.product_sn.clone(),
        product_price: p.product_price,
        product_status: p.product_status,
        sale_status: sale.map_or(1, |s| s.sale_status),
        manual_sale_status: sale.map_or(1, |s| s.manual_sale_status),
        sale_ban_time: sale.map_or(0, |s| s.sale_ban_time),
        sale_recover_time: sale.map_or(0, |s| s.sale_recover_time),
        sale_reason: sale.and_then(|s| s.sale_reason.clone()),
        is_best: p.is_best,
        is_new: p.is_new,
        is_hot: p.is_hot,
        product_stock: p.product_stock,
        sort_order: p.sort_order,
        sku_count: i32::try_from(sku_vo.len()).unwrap_or(i32::MAX),
        product_sku: sku_vo,
        // —— 以下字段依赖营销/批发/周期购/O2O 域，本切片未迁移 ——
        shop: None,
        distance: None,
        price: None,
        is_time_discount: None,
        vendor_product_id: p.vendor_product_id,
        org_product_price: None,
        is_seckill: None,
        is_groupon: None,
        is_need_re_import_vendor_product: p.is_need_re_import_vendor_product,
        attributes_tpl_id: p.attributes_tpl_id,
        limit_number: p.limit_number,
        delivery_mode: p.delivery_mode,
        group_type: None,
        is_cycle_product: None,
        cycle_price: None,
        cycle_original_price: None,
        cycle_period_name: None,
        virtual_sales: p.virtual_sales,
        delivery_type: None,
        is_card_product: None,
        redeem_total_times: None,
        effective_type_desc: None,
        validity_desc: None,
        unit_price: None,
        first_wholesale_price: None,
        wholesale_card_moq: None,
        wholesale_moq_dimension: None,
        wholesale_multiple: None,
    }
}

/// 商品实体 + 子表 → 管理端详情（对应 Java \"ProductDetailDTO\"）。
fn to_admin_detail(
    p: &Product,
    skus: &[ProductSku],
    gallery: &[ProductGallery],
    videos: &[ProductVideo],
    attrs: &[ProductAttributes],
    article_ids: &[i32],
) -> AdminProductDetailVO {
    let sku_vo: Vec<ProductSkuVO> = skus.iter().map(to_sku_vo).collect();
    let attr_list = group_attributes(attrs);
    let sku_type = resolve_sku_type(&attr_list, sku_vo.len());
    AdminProductDetailVO {
        product_id: p.product_id,
        product_name: p.product_name.clone(),
        seo_identification: p.seo_identification.clone(),
        product_sn: p.product_sn.clone(),
        product_tsn: p.product_tsn.clone(),
        product_stock: p.product_stock,
        product_price: p.product_price,
        market_price: p.market_price,
        cost_price: p.cost_price,
        shipping_tpl_id: i64::from(p.shipping_tpl_id),
        product_status: p.product_status,
        product_type: p.product_type,
        sale_mode: p.sale_mode,
        product_sale_mode: i32::from(p.sale_mode > 0),
        presale_info: None,
        category_id: p.category_id,
        brand_id: p.brand_id,
        shop_id: p.shop_id,
        keywords: p.keywords.clone(),
        shop_category_id: p.shop_category_id,
        check_status: p.check_status,
        check_reason: p.check_reason.clone(),
        // Java 在返回前已把 clickCount +1
        click_count: p.click_count + 1,
        product_weight: p.product_weight,
        product_volume: p.product_volume,
        is_promote: p.is_promote,
        is_promote_activity: p.is_promote_activity,
        promote_price: p.promote_price,
        promote_start_date: p.promote_start_date,
        promote_end_date: p.promote_end_date,
        seckill_max_num: p.seckill_max_num,
        product_brief: p.product_brief.clone(),
        product_desc: p.product_desc.clone(),
        pic_url: p.pic_url.clone(),
        pic_thumb: p.pic_thumb.clone(),
        pic_original: p.pic_original.clone(),
        pic_large: p.pic_large.clone(),
        product_service_ids: parse_id_json_lossy(p.product_service_ids.as_deref().unwrap_or("")),
        free_shipping: p.free_shipping,
        integral: p.integral,
        add_time: i64::from(p.add_time),
        sort_order: p.sort_order,
        store_sort_order: p.store_sort_order,
        is_delete: p.is_delete,
        is_best: p.is_best,
        is_new: p.is_new,
        is_hot: p.is_hot,
        last_update: p.last_update,
        remark: p.remark.clone(),
        give_integral: p.give_integral,
        rank_integral: p.rank_integral,
        suppliers_id: p.suppliers_id,
        virtual_sales: p.virtual_sales,
        limit_number: p.limit_number,
        product_care: p.product_care.clone(),
        product_related: parse_id_json_lossy(p.product_related.as_deref().unwrap_or("")),
        is_support_return: p.is_support_return,
        is_support_cod: p.is_support_cod,
        product_video: p.product_video.clone(),
        prepay_price: p.prepay_price,
        product_desc_arr: desc_arr_from_html(p.product_desc.as_deref().unwrap_or("")),
        img_list: gallery.iter().map(to_gallery_vo).collect(),
        product_video_info: videos.iter().map(to_video_vo).collect(),
        product_article_list: article_ids.to_vec(),
        attr_list,
        sku_count: i32::try_from(sku_vo.len()).unwrap_or(i32::MAX),
        product_list: sku_vo,
        sku_type,
        card_group_id: p.card_group_id,
        virtual_sample: p.virtual_sample.clone(),
        paid_content: p
            .paid_content
            .as_deref()
            .and_then(|raw| serde_json::from_str(raw).ok()),
        no_shipping: p.no_shipping,
        fixed_shipping_type: p.fixed_shipping_type,
        fixed_shipping_fee: p.fixed_shipping_fee,
        is_shop_pickup: p.is_shop_pickup,
        shop_pickup_tpl: None,
        vendor_product_id: p.vendor_product_id,
        vendor_id: p.vendor_id,
        is_logistics: p.is_logistics,
        is_shop_delivery: p.is_shop_delivery,
        reservation_tpl_id: p.reservation_tpl_id,
        reservation_times: Vec::new(),
        gift_card_groups_id: p.gift_card_groups_id,
        product_unit: p.product_unit.clone(),
        attributes_tpl_id: p.attributes_tpl_id,
        auto_online_time: None,
        auto_offline_time: None,
        auto_online_task_id: None,
        auto_offline_task_id: None,
        auto_online_task_status: None,
        auto_offline_task_status: None,
        auto_task_error_msg: None,
        timed_off_shelf: 0,
        delivery_mode: p.delivery_mode,
        delivery_service_fee_enabled: None,
        server_service_fee_enabled: None,
        store_assign_delivery_service_fee: None,
        store_assign_server_service_fee: None,
        delivery_fee_editable: None,
        server_fee_editable: None,
        platform_delivery_service_fee_type: Some(p.delivery_service_fee_type),
        platform_delivery_service_fee_value: Some(p.delivery_service_fee_value),
        platform_server_service_fee_type: Some(p.server_service_fee_type),
        platform_server_service_fee_value: Some(p.server_service_fee_value),
        store_delivery_service_fee_type: None,
        store_delivery_service_fee_value: None,
        store_server_service_fee_type: None,
        store_server_service_fee_value: None,
    }
}

/// 规格实体 → VO。
fn to_sku_vo(s: &ProductSku) -> ProductSkuVO {
    ProductSkuVO {
        sku_id: s.sku_id,
        product_id: s.product_id,
        sku_value: s.sku_value.clone(),
        sku_data: s
            .sku_data
            .as_deref()
            .and_then(|raw| serde_json::from_str(raw).ok()),
        sku_sn: s.sku_sn.clone(),
        sku_stock: s.sku_stock.unwrap_or(0),
        sku_tsn: s.sku_tsn.clone(),
        sku_weight: s.sku_weight,
        sku_volume: s.sku_volume,
        sku_price: s.sku_price,
        is_enabled: s.is_enabled,
        market_price: s.market_price,
        cost_price: s.cost_price,
        delivery_service_fee_type: s.delivery_service_fee_type,
        delivery_service_fee_value: s.delivery_service_fee_value,
        server_service_fee_type: s.server_service_fee_type,
        server_service_fee_value: s.server_service_fee_value,
        vendor_product_sku_id: s.vendor_product_sku_id,
    }
}

/// 入参 SKU → 实体（\"db\" 为 \"Some\" 时按「入参非空覆盖库值」合并）。
fn build_sku_from_dto(
    item: &ProductSkuSaveDTO,
    product_id: i32,
    db: Option<&ProductSku>,
) -> Result<ProductSku> {
    let base = db.cloned().unwrap_or_default();
    Ok(ProductSku {
        sku_id: item.sku_id.unwrap_or(base.sku_id),
        product_id,
        sku_value: item
            .sku_value
            .clone()
            .unwrap_or_else(|| base.sku_value.clone()),
        sku_data: item
            .sku_data
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .or_else(|| base.sku_data.clone()),
        sku_sn: item.sku_sn.clone().or_else(|| base.sku_sn.clone()),
        sku_stock: item.sku_stock.or(base.sku_stock),
        sku_tsn: item.sku_tsn.clone().or_else(|| base.sku_tsn.clone()),
        sku_price: item.sku_price.or(base.sku_price),
        sku_weight: item.sku_weight.or(base.sku_weight),
        sku_volume: item.sku_volume.or(base.sku_volume),
        is_enabled: normalize_sku_enabled(item.is_enabled)?,
        market_price: item.market_price.or(base.market_price),
        cost_price: item.cost_price.or(base.cost_price),
        delivery_service_fee_type: item
            .delivery_service_fee_type
            .unwrap_or(base.delivery_service_fee_type),
        delivery_service_fee_value: item
            .delivery_service_fee_value
            .unwrap_or(base.delivery_service_fee_value),
        server_service_fee_type: item
            .server_service_fee_type
            .unwrap_or(base.server_service_fee_type),
        server_service_fee_value: item
            .server_service_fee_value
            .unwrap_or(base.server_service_fee_value),
        vendor_product_sku_id: item.vendor_product_sku_id.or(base.vendor_product_sku_id),
    })
}

/// 相册实体 → VO。
fn to_gallery_vo(g: &ProductGallery) -> ProductGalleryVO {
    ProductGalleryVO {
        pic_id: g.pic_id,
        product_id: g.product_id,
        pic_url: g.pic_url.clone(),
        pic_desc: g.pic_desc.clone(),
        pic_thumb: g.pic_thumb.clone(),
        pic_original: g.pic_original.clone(),
        pic_large: g.pic_large.clone(),
        sort_order: g.sort_order,
        shop_id: g.shop_id,
    }
}

/// 视频实体 → VO。
fn to_video_vo(v: &ProductVideo) -> ProductVideoVO {
    ProductVideoVO {
        id: Some(v.id),
        video_id: Some(v.video_id),
        product_id: Some(v.product_id),
        video_url: Some(v.video_url.clone()),
        video_cover: Some(v.video_cover.clone()),
        format: Some(v.format.clone()),
    }
}

/// 商品服务实体 → VO。
fn to_services_vo(s: &nextrade_core::entities::ProductServices) -> ServicesVO {
    ServicesVO {
        product_service_id: s.product_service_id,
        product_service_name: s.product_service_name.clone(),
        product_service_desc: s.product_service_desc.clone(),
        ico_img: s.ico_img.clone(),
        sort_order: s.sort_order,
        default_on: s.default_on,
        shop_id: s.shop_id,
    }
}

/// 分类实体 → 下拉项。
fn to_category_option(c: &Category) -> CategoryOptionVO {
    CategoryOptionVO {
        category_id: c.category_id,
        category_name: c.category_name.clone(),
        parent_id: c.parent_id,
        sort_order: c.sort_order,
        is_show: c.is_show,
    }
}

/// 品牌实体 → 下拉项。
fn to_brand_option(b: &Brand) -> BrandOptionVO {
    BrandOptionVO {
        brand_id: b.brand_id,
        brand_name: b.brand_name.clone(),
        brand_logo: b.brand_logo.clone(),
        first_word: b.first_word.clone(),
        sort_order: b.sort_order,
        is_show: b.is_show,
    }
}

/// 商品实体 → 前台详情主体（对应 Java \"ProductInfoDTO\" 的可迁移部分）。
fn to_client_info(p: &Product) -> ClientProductInfoVO {
    // 虚拟类商品不管理库存，库里常为 0；原版前端用 productStock > 0 判断售罄。
    let product_stock = if p.is_virtual_product() {
        999_999
    } else {
        p.product_stock
    };
    ClientProductInfoVO {
        product_id: p.product_id,
        product_name: p.product_name.clone(),
        product_sn: p.product_sn.clone(),
        product_tsn: p.product_tsn.clone(),
        seo_identification: p.seo_identification.clone(),
        product_stock,
        product_price: p.product_price,
        market_price: p.market_price,
        cost_price: p.cost_price,
        product_brief: p.product_brief.clone(),
        product_desc: p.product_desc.clone(),
        pic_url: p.pic_url.clone(),
        pic_thumb: p.pic_thumb.clone(),
        pic_original: p.pic_original.clone(),
        pic_large: p.pic_large.clone(),
        keywords: p.keywords.clone(),
        category_id: p.category_id,
        brand_id: p.brand_id,
        shop_id: p.shop_id,
        product_status: p.product_status,
        product_type: p.product_type,
        sale_mode: p.sale_mode,
        product_sale_mode: i32::from(p.sale_mode > 0),
        delivery_mode: p.delivery_mode,
        free_shipping: p.free_shipping,
        fixed_shipping_type: p.fixed_shipping_type,
        fixed_shipping_fee: p.fixed_shipping_fee,
        no_shipping: p.no_shipping,
        integral: p.integral,
        give_integral: p.give_integral,
        rank_integral: p.rank_integral,
        virtual_sales: p.virtual_sales,
        limit_number: p.limit_number,
        product_weight: p.product_weight,
        product_volume: p.product_volume,
        product_unit: p.product_unit.clone(),
        product_care: p.product_care.clone(),
        product_service_ids: parse_id_json_lossy(p.product_service_ids.as_deref().unwrap_or("")),
        is_support_return: p.is_support_return,
        is_support_cod: p.is_support_cod,
        is_best: p.is_best,
        is_new: p.is_new,
        is_hot: p.is_hot,
        add_time: p.add_time,
        click_count: p.click_count,
        is_delete: p.is_delete,
        card_group_id: p.card_group_id,
        virtual_sample: p.virtual_sample.clone(),
        paid_content: p
            .paid_content
            .as_deref()
            .and_then(|raw| serde_json::from_str(raw).ok()),
        prepay_price: p.prepay_price,
        card_product: None,
        presale_info: None,
        sale_mode_text: None,
        is_distribution: None,
    }
}

#[derive(Debug, Clone, Default)]
struct ParsedRowData {
    product_sn: String,
    product_name: String,
    pic_url: String,
    category_name: String,
    product_status: String,
    spec_name: String,
    sku_sn: String,
    sku_tsn: String,
    product_price: String,
    product_stock: String,
    market_price: String,
    product_tsn: String,
    keywords: String,
    product_brief: String,
    cost_price: String,
}

fn resolve_cat_id(categories: &[Category], raw_name: &str) -> i32 {
    let name = raw_name.trim();
    if name.is_empty() {
        return 0;
    }
    // 1. 精确匹配分类名称
    if let Some(cat) = categories.iter().find(|c| c.category_name.trim() == name) {
        return cat.category_id;
    }
    // 2. 匹配路径末级分类，例如 "服装 > 女装 > 上衣" 或 "服装/女装/上衣"
    let leaf_name = name.split(['>', '/', '\\']).last().unwrap_or("").trim();
    if !leaf_name.is_empty() {
        if let Some(cat) = categories.iter().find(|c| c.category_name.trim() == leaf_name) {
            return cat.category_id;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::FromPrimitive;
    use std::str::FromStr;

    fn sku_save(id: Option<i32>, enabled: Option<i32>, stock: i32, price: &str) -> ProductSkuSaveDTO {
        ProductSkuSaveDTO {
            sku_id: id,
            is_enabled: enabled,
            sku_stock: Some(stock),
            sku_price: Some(Decimal::from_str(price).unwrap()),
            sku_value: Some(format!("值{stock}")),
            ..Default::default()
        }
    }

    #[test]
    fn descendant_category_ids_includes_self_and_children() {
        // 1 ─ 2 ─ 4 ; 1 ─ 3
        let all = vec![(1, 0), (2, 1), (3, 1), (4, 2), (9, 0)];
        let mut ids = descendant_category_ids(&all, 1);
        ids.sort_unstable();
        assert_eq!(ids, vec![1, 2, 3, 4]);
        // 叶子节点只返回自己
        assert_eq!(descendant_category_ids(&all, 4), vec![4]);
        // 未出现在表中的分类：仍按自身过滤（不放大为全表）
        assert_eq!(descendant_category_ids(&all, 7), vec![7]);
    }

    #[test]
    fn resolve_order_column_whitelists_and_falls_back() {
        assert_eq!(resolve_order_column(Some("productPrice")), "product_price");
        assert_eq!(resolve_order_column(Some("virtualSales")), "virtual_sales");
        assert_eq!(resolve_order_column(Some("product_price")), "product_price");
        // 注入尝试与未知字段一律回落主键
        assert_eq!(resolve_order_column(Some("product_price; DROP TABLE product")), "product_id");
        assert_eq!(resolve_order_column(Some("unknownField")), "product_id");
        assert_eq!(resolve_order_column(None), "product_id");
        assert_eq!(resolve_order_column(Some("  ")), "product_id");
    }

    #[test]
    fn sku_enabled_normalization_matches_java() {
        assert_eq!(normalize_sku_enabled(None).unwrap(), 1);
        assert_eq!(normalize_sku_enabled(Some(0)).unwrap(), 0);
        assert_eq!(normalize_sku_enabled(Some(1)).unwrap(), 1);
        let err = normalize_sku_enabled(Some(2)).unwrap_err();
        assert!(matches!(err, Error::Biz { msg, .. } if msg == ERR_SKU_STATUS));
    }

    #[test]
    fn sku_list_requires_at_least_one_enabled() {
        // 全禁用 → 报错
        let list = vec![sku_save(Some(1), Some(0), 1, "1.00")];
        let err = validate_sku_list(&list).unwrap_err();
        assert!(matches!(err, Error::Biz { msg, .. } if msg == ERR_SKU_AT_LEAST_ONE));

        // 未传 isEnabled 视为启用
        let list = vec![sku_save(None, None, 1, "1.00")];
        assert!(validate_sku_list(&list).is_ok());
    }

    #[test]
    fn enabled_sku_summary_sums_stock_and_takes_min_price() {
        let list = vec![
            sku_save(None, Some(1), 3, "9.90"),
            sku_save(None, Some(1), 2, "8.50"),
            // 禁用规格不计入
            sku_save(None, Some(0), 100, "1.00"),
        ];
        let (stock, price) = enabled_sku_summary(&list).unwrap();
        assert_eq!(stock, 5);
        assert_eq!(price, Decimal::from_str("8.50").unwrap());
    }

    #[test]
    fn enabled_sku_summary_requires_price() {
        let list = vec![ProductSkuSaveDTO {
            sku_id: None,
            is_enabled: Some(1),
            sku_stock: Some(1),
            sku_price: None,
            ..Default::default()
        }];
        let err = enabled_sku_summary(&list).unwrap_err();
        assert!(matches!(err, Error::Biz { msg, .. } if msg == ERR_PRICE_EMPTY));
    }

    #[test]
    fn save_summary_single_spec_uses_input_values() {
        let dto = ProductSaveDTO {
            product_stock: Some(7),
            product_price: Some(Decimal::from_str("12.30").unwrap()),
            product_list: Some(Vec::new()),
            ..Default::default()
        };
        let (stock, price) = resolve_save_summary(&dto).unwrap();
        assert_eq!(stock, 7);
        assert_eq!(price, Decimal::from_str("12.30").unwrap());
    }

    #[test]
    fn free_shipping_follows_fixed_shipping_type() {
        // 固定运费 0 元 → 包邮
        let mut p = Product {
            fixed_shipping_type: Some(1),
            fixed_shipping_fee: Some(Decimal::ZERO),
            free_shipping: 0,
            ..Default::default()
        };
        sync_free_shipping_with_shipping_type(&mut p);
        assert_eq!(p.free_shipping, 1);

        // 固定运费 > 0 → 不包邮
        let mut p = Product {
            fixed_shipping_type: Some(1),
            fixed_shipping_fee: Some(Decimal::from_str("5.00").unwrap()),
            free_shipping: 1,
            ..Default::default()
        };
        sync_free_shipping_with_shipping_type(&mut p);
        assert_eq!(p.free_shipping, 0);

        // 运费模板(2) → 不包邮
        let mut p = Product {
            fixed_shipping_type: Some(2),
            fixed_shipping_fee: None,
            free_shipping: 1,
            ..Default::default()
        };
        sync_free_shipping_with_shipping_type(&mut p);
        assert_eq!(p.free_shipping, 0);

        // 未传运费类型 → 不动
        let mut p = Product {
            fixed_shipping_type: None,
            free_shipping: 1,
            ..Default::default()
        };
        sync_free_shipping_with_shipping_type(&mut p);
        assert_eq!(p.free_shipping, 1);
    }

    #[test]
    fn name_and_price_validation_messages() {
        assert!(matches!(
            validate_product_name(None).unwrap_err(),
            Error::Biz { msg, .. } if msg == ERR_NAME_EMPTY
        ));
        assert!(matches!(
            validate_product_name(Some("   ")).unwrap_err(),
            Error::Biz { msg, .. } if msg == ERR_NAME_EMPTY
        ));
        let long = "商".repeat(201);
        assert!(matches!(
            validate_product_name(Some(&long)).unwrap_err(),
            Error::Biz { msg, .. } if msg == ERR_NAME_TOO_LONG
        ));
        assert!(validate_product_name(Some(&"商".repeat(200))).is_ok());

        assert!(matches!(
            validate_product_price(None).unwrap_err(),
            Error::Biz { msg, .. } if msg == ERR_PRICE
        ));
        assert!(matches!(
            validate_product_price(Some(Decimal::from_str("-0.01").unwrap())).unwrap_err(),
            Error::Biz { msg, .. } if msg == ERR_PRICE
        ));
        assert!(validate_product_price(Some(Decimal::ZERO)).is_ok());
    }

    #[test]
    fn desc_html_round_trip() {
        let arr = vec![
            ProductDescVO {
                r#type: "pic".into(),
                pic: Some("a.jpg".into()),
                ..Default::default()
            },
            ProductDescVO {
                r#type: "video".into(),
                cover: Some("c.jpg".into()),
                video_url: Some("v.mp4".into()),
                ..Default::default()
            },
            ProductDescVO {
                r#type: "text".into(),
                html: Some("<p>hi</p>".into()),
                ..Default::default()
            },
        ];
        let html = desc_html_from_arr(&arr);
        assert!(html.contains("desc-pic-item"));
        assert!(html.contains("desc-video-item"));
        assert!(html.contains("<p>hi</p>"));

        let back = desc_arr_from_html(&html);
        assert_eq!(back.len(), 3);
        assert_eq!(back[0].r#type, "pic");
        assert_eq!(back[0].pic.as_deref(), Some("a.jpg"));
        assert_eq!(back[1].r#type, "video");
        assert_eq!(back[1].cover.as_deref(), Some("c.jpg"));
        assert_eq!(back[1].video_url.as_deref(), Some("v.mp4"));
        assert_eq!(back[2].r#type, "text");
    }

    #[test]
    fn desc_arr_empty_html_yields_empty() {
        assert!(desc_arr_from_html("").is_empty());
        assert!(desc_arr_from_html("  ").is_empty());
    }

    #[test]
    fn group_attributes_splits_by_type_and_keeps_order() {
        let rows = vec![
            ProductAttributes {
                attributes_id: 1,
                product_id: 9,
                attr_type: 1,
                attr_name: "颜色".into(),
                attr_value: "红".into(),
                sort_order: Some(0),
                ..Default::default()
            },
            ProductAttributes {
                attributes_id: 2,
                product_id: 9,
                attr_type: 1,
                attr_name: "颜色".into(),
                attr_value: "蓝".into(),
                sort_order: Some(1),
                ..Default::default()
            },
            ProductAttributes {
                attributes_id: 3,
                product_id: 9,
                attr_type: 0,
                attr_name: "材质".into(),
                attr_value: "棉".into(),
                sort_order: Some(2),
                ..Default::default()
            },
            ProductAttributes {
                attributes_id: 4,
                product_id: 9,
                attr_type: 2,
                attr_name: "附赠".into(),
                attr_value: "说明书".into(),
                sort_order: Some(3),
                ..Default::default()
            },
        ];
        let attrs = group_attributes(&rows);
        assert_eq!(attrs.spe.len(), 1);
        assert_eq!(attrs.spe[0].attr_name, "颜色");
        assert_eq!(attrs.spe[0].attr_list.len(), 2);
        assert_eq!(attrs.spe[0].attr_list[1].attr_value, "蓝");
        assert_eq!(attrs.normal.len(), 1);
        assert_eq!(attrs.extra.len(), 1);

        // 有规格属性 → 多规格
        assert_eq!(resolve_sku_type(&attrs, 2), 2);
        // 无规格属性且仅 1 条 SKU → 统一规格
        let empty = ProductAttrsVO::default();
        assert_eq!(resolve_sku_type(&empty, 1), 1);
        // 无规格属性但 SKU 多于 1 条 → 多规格
        assert_eq!(resolve_sku_type(&empty, 2), 2);
    }

    #[test]
    fn parse_id_json_accepts_java_list_to_string() {
        // Java `List.toString()` 结果
        assert_eq!(parse_id_json_lossy("[3, 1, 2]"), vec![3, 1, 2]);
        assert_eq!(parse_id_json_lossy("[1,2]"), vec![1, 2]);
        assert_eq!(parse_id_json_lossy("1,2,3"), vec![1, 2, 3]);
        assert_eq!(parse_id_json_lossy("5"), vec![5]);
        assert!(parse_id_json_lossy("").is_empty());
        assert!(parse_id_json_lossy("abc").is_empty());
    }

    #[test]
    fn format_int_list_matches_java_to_string() {
        assert_eq!(format_int_list(&[1, 2, 3]), "[1, 2, 3]");
        assert_eq!(format_int_list(&[7]), "[7]");
    }

    #[test]
    fn summary_from_entities_ignores_disabled() {
        let skus = vec![
            ProductSku {
                sku_id: 1,
                is_enabled: 1,
                sku_stock: Some(4),
                sku_price: Some(Decimal::from_str("6.60").unwrap()),
                ..Default::default()
            },
            ProductSku {
                sku_id: 2,
                is_enabled: 1,
                sku_stock: Some(1),
                sku_price: Some(Decimal::from_str("5.50").unwrap()),
                ..Default::default()
            },
            ProductSku {
                sku_id: 3,
                is_enabled: 0,
                sku_stock: Some(999),
                sku_price: Some(Decimal::from_str("0.10").unwrap()),
                ..Default::default()
            },
        ];
        let (stock, price) = summary_from_entities(&skus).unwrap();
        assert_eq!(stock, 5);
        assert_eq!(price, Decimal::from_str("5.50").unwrap());

        // 全部禁用 → 不更新商品汇总
        let disabled = vec![ProductSku {
            sku_id: 3,
            is_enabled: 0,
            sku_stock: Some(1),
            sku_price: Some(Decimal::from_str("1.00").unwrap()),
            ..Default::default()
        }];
        assert!(summary_from_entities(&disabled).is_none());
    }

    #[test]
    fn percent_encode_handles_chinese_and_space() {
        // Java: URLEncoder.encode("商品导入模板.xlsx").replace("+","%20")
        let encoded = percent_encode_utf8(DOWNLOAD_NAME_MERCHANT);
        assert!(encoded.ends_with(".xlsx"));
        assert!(encoded.starts_with("%E5%95%86"));
        assert!(!encoded.contains('+'));
        assert_eq!(percent_encode_utf8("a b"), "a%20b");
    }

    #[test]
    fn parse_datetime_seconds_supports_java_formats() {
        // 2024-01-02 03:04:05 UTC
        let ts = parse_datetime_seconds("2024-01-02 03:04:05").unwrap();
        assert_eq!(ts, 1_704_164_645);
        // 纯时间戳原样通过
        assert_eq!(parse_datetime_seconds("1704164645"), Some(1_704_164_645));
        assert_eq!(parse_datetime_seconds(""), None);
    }

    #[test]
    fn apply_dto_fills_ddl_defaults_on_create() {
        let dto = ProductSaveDTO {
            product_name: Some("测试".into()),
            product_price: Some(Decimal::from_str("1.00").unwrap()),
            ..Default::default()
        };
        let mut p = Product::default();
        apply_dto_to_product(&mut p, dto.clone(), true);
        // 与 java/database/pure.sql 的 DDL 默认值一致
        assert_eq!(p.product_status, 1);
        assert_eq!(p.check_status, 1);
        assert_eq!(p.product_type, Some(1));
        assert_eq!(p.delivery_mode, 1);
        assert_eq!(p.sort_order, 100);
        assert_eq!(p.store_sort_order, 100);
        assert_eq!(p.is_logistics, 1);
        assert_eq!(p.is_support_cod, 1);
        assert_eq!(p.product_name, "测试");
    }

    #[test]
    fn apply_dto_keeps_db_values_when_field_absent() {
        let mut p = Product {
            product_name: "库里原有".into(),
            product_status: 0,
            brand_id: 5,
            check_status: 2,
            ..Default::default()
        };
        let dto = ProductSaveDTO {
            product_name: Some("新名字".into()),
            ..Default::default()
        };
        apply_dto_to_product(&mut p, dto.clone(), false);
        assert_eq!(p.product_name, "新名字");
        // 未传字段保持库中原值（NOT_NULL 更新策略）
        assert_eq!(p.product_status, 0);
        assert_eq!(p.brand_id, 5);
        assert_eq!(p.check_status, 2);
    }

    #[test]
    fn apply_dto_maps_related_and_services_as_java_list_to_string() {
        let dto = ProductSaveDTO {
            product_related: Some(vec![2, 4]),
            product_service_ids: Some(vec![9]),
            ..Default::default()
        };
        let mut p = Product::default();
        apply_dto_to_product(&mut p, dto.clone(), true);
        assert_eq!(p.product_related.as_deref(), Some("[2, 4]"));
        assert_eq!(p.product_service_ids.as_deref(), Some("[9]"));
    }

    #[test]
    fn apply_dto_takes_first_gallery_pic_as_main_image() {
        let dto = ProductSaveDTO {
            img_list: Some(vec![
                GalleryPicDTO {
                    pic_url: Some("a.jpg".into()),
                    pic_thumb: Some("a_thumb.jpg".into()),
                    ..Default::default()
                },
                GalleryPicDTO {
                    pic_url: Some("b.jpg".into()),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };
        let mut p = Product::default();
        apply_dto_to_product(&mut p, dto.clone(), true);
        assert_eq!(p.pic_url, "a.jpg");
        assert_eq!(p.pic_original, "a.jpg");
        assert_eq!(p.pic_thumb, "a_thumb.jpg");
    }

    #[test]
    fn batch_item_maps_only_provided_fields() {
        let item = nextrade_core::models::catalog::product_manage::ProductBatchEditItem {
            product_id: Some(1),
            product_name: Some("批量名".into()),
            product_price: Some(Decimal::from_str("3.30").unwrap()),
            product_service_ids: Some(vec![1, 2]),
            ..Default::default()
        };
        let mut p = Product {
            product_status: 1,
            free_shipping: 1,
            ..Default::default()
        };
        apply_batch_item(&mut p, &item);
        assert_eq!(p.product_name, "批量名");
        assert_eq!(p.product_price, Decimal::from_str("3.30").unwrap());
        assert_eq!(p.product_service_ids.as_deref(), Some("[1, 2]"));
        // 未传字段不动
        assert_eq!(p.product_status, 1);
        assert_eq!(p.free_shipping, 1);
    }

    #[test]
    fn decimal_helper_is_used_for_money_only() {
        // 金额统一 Decimal，避免 f64 精度问题
        let v = Decimal::from_f64(0.1 + 0.2).unwrap();
        assert_eq!(v.to_string(), "0.3");
    }

    #[test]
    fn test_resolve_cat_id() {
        let cats = vec![
            Category {
                category_id: 10,
                category_name: "服装".to_string(),
                ..Default::default()
            },
            Category {
                category_id: 101,
                category_name: "女装".to_string(),
                ..Default::default()
            },
            Category {
                category_id: 102,
                category_name: "连衣裙".to_string(),
                ..Default::default()
            },
        ];

        // 精确匹配
        assert_eq!(resolve_cat_id(&cats, "服装"), 10);
        assert_eq!(resolve_cat_id(&cats, "女装"), 101);

        // 路径末级匹配
        assert_eq!(resolve_cat_id(&cats, "服装 > 女装 > 连衣裙"), 102);
        assert_eq!(resolve_cat_id(&cats, "服装/女装/连衣裙"), 102);

        // 未匹配
        assert_eq!(resolve_cat_id(&cats, "数码3C"), 0);
        assert_eq!(resolve_cat_id(&cats, ""), 0);
    }

    #[test]
    fn test_excel_round_trip_for_import() {
        let headers = vec!["商品编码", "商品名称", "售价", "库存", "分类名称"];
        let rows = vec![
            vec![
                "SN001".to_string(),
                "测试商品1".to_string(),
                "99.00".to_string(),
                "100".to_string(),
                "服装".to_string(),
            ],
            vec![
                "SN002".to_string(),
                "测试商品2".to_string(),
                "199.00".to_string(),
                "50".to_string(),
                "女装".to_string(),
            ],
        ];

        let bytes = crate::system::excel::build_xlsx(&headers, &rows, "Sheet1").unwrap();
        assert!(!bytes.is_empty());

        let read_back = crate::system::excel::read_xlsx(&bytes).unwrap();
        assert_eq!(read_back.len(), 3);
        assert_eq!(read_back[0], vec!["商品编码", "商品名称", "售价", "库存", "分类名称"]);
        assert_eq!(read_back[1][0], "SN001");
        assert_eq!(read_back[1][1], "测试商品1");
        assert_eq!(read_back[2][0], "SN002");
    }
}"