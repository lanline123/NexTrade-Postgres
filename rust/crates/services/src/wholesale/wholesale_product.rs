//! 批发商品域服务（wholesale::wholesale_product）。
//!
//! 对应原 Java `WholesaleProductController` + `WholesaleProductConfigServiceImpl`
//! / `WholesaleProductQueryServiceImpl` / `WholesalePriceServiceImpl` /
//! `WholesaleSkuConfigServiceImpl` 的管理端 13 个端点。
//!
//! # 端点 → 方法
//!
//! | 端点（挂载于 `/adminapi`） | 方法 |
//! |---------------------------|------|
//! | `GET  /wholesale/product/page` | [`WholesaleProductService::page`] |
//! | `GET  /wholesale/product/selectable/page` | [`WholesaleProductService::selectable_page`] |
//! | `POST /wholesale/product/add` | [`WholesaleProductService::add`] |
//! | `POST /wholesale/product/batchAdd` | [`WholesaleProductService::batch_add`] |
//! | `POST /wholesale/product/batchDelete` | [`WholesaleProductService::batch_delete`] |
//! | `GET  /wholesale/product/price/detail` | [`WholesaleProductService::price_detail`] |
//! | `POST /wholesale/product/price/save` | [`WholesaleProductService::price_save`] |
//! | `POST /wholesale/product/price/batchSetCommon` | [`WholesaleProductService::batch_set_common`] |
//! | `POST /wholesale/product/price/batchSetByGrade` | [`WholesaleProductService::batch_set_grade`] |
//! | `GET  /wholesale/product/buyRule/detail` | [`WholesaleProductService::buy_rule_detail`] |
//! | `POST /wholesale/product/buyRule/save` | [`WholesaleProductService::buy_rule_save`] |
//! | `POST /wholesale/product/buyRule/batchSave` | [`WholesaleProductService::buy_rule_batch_save`] |
//! | `POST /wholesale/product/multiple/save` | [`WholesaleProductService::multiple_save`] |
//!
//! # 语义要点（逐条对照 Java）
//!
//! - `page`：以 config 为分页主表（`shop_id` 隔离）→ 回填商品 / SKU /
//!   SKU 配置 / 批发价范围（逐规格「有等级价取等级价、否则取通用价」再汇总）。
//! - `add`：商品存在（5310）→ 归属（5314）→ 类型限制（5315：仅普通实物、
//!   非预售、非周期购）→ 唯一配置（5311）→ 写入默认
//!   `multiSpecPriceMode/moqDimension` 与 `moq=1`。
//! - `batchAdd`：去重保序；已配置 → skipped；跨店 / 类型不支持 → failedItems；
//!   其余异常整批抛错。
//! - `batchDelete`：先批量归属校验 → 物理删 price → 软删 sku_config →
//!   物理删 config。
//! - `price/detail`：归属校验 → config 不存在 5310 → SKU × 价格行装配
//!   （`sku_id=0` 行归属所有 SKU；无 SKU 行时合成 SPU 兜底行）。
//! - `price/save`：归属校验 → 校验行（5312 含「第 N 行：」）→ 等级启用校验
//!   → 全量替换 → 可选同步 `multiSpecPriceMode`。
//! - `batchSetCommon` / `batchSetByGrade`：读-合并-整表替换（保留非目标行）。
//! - `buyRule/*`：详情读 config + sku_config（未配置返回 configured=false）；
//!   保存仅改 `dimension + moq`；批量保存整批校验、缺失即整批 5310。
//! - `multiple/save`：逐 SKU upsert 倍数（`multiple < 1` 拒绝）。
//!
//! # 差异
//!
//! 多租户 `shop_id` 由路由层传入（默认 0 = 平台）；价格区间文案读取
//! `config.dollarSign`（缺省 `¥`）。其余详见 models 模块头。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::entities::{
    ProductWholesaleConfig, ProductWholesalePrice, ProductWholesaleSkuConfig,
};
use nextrade_core::models::wholesale::wholesale_product::{
    belongs_to_sku, build_hint, format_discount, format_money, format_retail_range,
    is_discount_in_range, normalize_moq_dimension, normalize_multi_spec_mode, require_price_mode,
    resolve_target_skus, validate_price_rows, validate_tiers, WholesaleBatchCommonDTO,
    WholesaleBatchDeleteDTO, WholesaleBatchGradeDTO, WholesaleBuyRuleBatchSaveDTO,
    WholesaleBuyRuleSaveDTO, WholesaleMultipleSaveDTO, WholesalePriceDetailVO, WholesalePriceRowVO,
    WholesalePriceSaveDTO, WholesaleProductAddDTO, WholesaleProductBatchAddDTO,
    WholesaleProductBatchAddFailedItem, WholesaleProductBatchAddVO, WholesaleProductBuyRuleVO,
    WholesaleProductPageQuery, WholesaleProductVO, WholesaleSelectableProductPageQuery,
    WholesaleSelectableProductVO, WholesaleSkuPriceVO, WholesaleSkuRuleVO, CODE_BAD_REQUEST,
    CODE_GRADE_DISABLED, CODE_GRADE_NOT_FOUND, CODE_NON_PLATFORM_PRODUCT,
    CODE_PRICE_INVALID, CODE_PRODUCT_ALREADY_CONFIGURED, CODE_PRODUCT_NOT_FOUND,
    CODE_PRODUCT_TYPE_NOT_SUPPORTED, DEFAULT_MOQ, DEFAULT_MOQ_DIMENSION,
    DEFAULT_MULTI_SPEC_PRICE_MODE, DEFAULT_PAGE_SIZE, EMPTY_DISPLAY, GRADE_UNIVERSAL,
    GRADE_UNIVERSAL_NAME, MAX_PAGE_SIZE, MAX_TIERS_PER_GROUP, MSG_GRADE_ID_INVALID,
    MSG_GRADE_ROWS_REQUIRED, MSG_NON_PLATFORM_PRODUCT, MSG_PRESALE_NOT_SUPPORTED,
    MSG_CYCLE_NOT_SUPPORTED, MSG_PRODUCT_ALREADY_CONFIGURED, MSG_PRODUCT_ID_REQUIRED,
    MSG_PRODUCT_IDS_REQUIRED, MSG_PRODUCT_NOT_CONFIGURED, MSG_PRODUCT_NOT_FOUND,
    MSG_PRODUCT_NOT_FOUND_OR_CONFIG, MSG_PRODUCT_TYPE_NOT_SUPPORTED, MSG_PRICE_ROWS_REQUIRED,
    MSG_MOQ_INVALID, MSG_TIERS_REQUIRED, SKU_SPU_FALLBACK, SINGLE_SPEC_SPEC_TEXT,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::wholesale::wholesale_product::WholesaleProductRepository;
use nextrade_infrastructure::db::Repositories;

/// 批发商品域服务（挂载于 `AdminState.wholesale.wholesale_product`）。
pub struct WholesaleProductService {
    repos: Arc<Repositories>,
}

impl WholesaleProductService {
    /// 构造（与同域其它 feature 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &WholesaleProductRepository {
        &self.repos.wholesale.wholesale_product
    }

    // ————————————————————— 列表 / 选择 —————————————————————

    /// 批发商品分页（Java `WholesaleProductQueryServiceImpl#page`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn page(
        &self,
        q: &WholesaleProductPageQuery,
        shop_id: i32,
    ) -> Result<PageResult<WholesaleProductVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
        };
        let keyword = q.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let keyword_ids = match keyword {
            Some(k) => Some(self.repo().product_ids_by_keyword(k).await?),
            None => None,
        };
        let (limit, offset) = page.limit_offset();
        let (total, configs) = self
            .repo()
            .paginate_configs(
                shop_id,
                q.moq_dimension,
                keyword_ids.as_deref(),
                limit,
                offset,
            )
            .await?;
        if configs.is_empty() {
            return Ok(PageResult::new(total, Vec::new(), &page));
        }
        let product_ids: Vec<i32> = configs.iter().map(|c| c.product_id).collect();
        let mut products = HashMap::new();
        for pid in &product_ids {
            if let Some(p) = self.repo().find_product(*pid).await? {
                products.insert(*pid, p);
            }
        }
        let skus = self.repo().find_enabled_skus_by_products(&product_ids).await?;
        let sku_configs = self.repo().list_sku_configs_by_products(&product_ids).await?;
        let currency = self.repo().currency_symbol().await?;

        let mut list = Vec::with_capacity(configs.len());
        for c in &configs {
            let empty_skus = Vec::new();
            let product_skus: Vec<_> = skus.iter().filter(|s| s.product_id == c.product_id).collect();
            let prices = self.repo().list_prices(c.product_id).await?;
            list.push(self
                .to_product_vo(c, products.get(&c.product_id), &product_skus, &empty_skus, &sku_configs, &prices, &currency)
                .await?);
        }
        Ok(PageResult::new(total, list, &page))
    }

    /// 可选批发商品分页（Java `selectablePage`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn selectable_page(
        &self,
        q: &WholesaleSelectableProductPageQuery,
        shop_id: i32,
    ) -> Result<PageResult<WholesaleSelectableProductVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
        };
        let category_ids = match q.category_id.filter(|id| *id > 0) {
            Some(id) => Some(self.repo().category_descendant_ids(id).await?),
            None => None,
        };
        let excluded = self.repo().all_configured_product_ids().await?;
        let (limit, offset) = page.limit_offset();
        let (total, products) = self
            .repo()
            .paginate_selectable_products(
                shop_id,
                q.product_status.unwrap_or(1),
                q.keyword.as_deref(),
                category_ids.as_deref(),
                q.brand_id.filter(|id| *id > 0),
                &excluded,
                limit,
                offset,
            )
            .await?;
        let list = products
            .iter()
            .map(|p| WholesaleSelectableProductVO {
                product_id: p.product_id,
                product_name: p.product_name.clone(),
                product_sn: p.product_sn.clone(),
                product_tsn: p.product_tsn.clone(),
                pic_url: p.pic_url.clone(),
                pic_thumb: p.pic_thumb.clone(),
                product_price: p.product_price,
                market_price: p.market_price,
                product_status: p.product_status,
                category_id: p.category_id,
                brand_id: p.brand_id,
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    // ————————————————————— 添加 / 删除 —————————————————————

    /// 添加批发商品（Java `WholesaleProductConfigServiceImpl#create`）。
    ///
    /// # Errors
    /// 5310 / 5314 / 5315 / 5311 与 400 参数错误。
    pub async fn add(&self, dto: &WholesaleProductAddDTO, shop_id: i32) -> Result<i32> {
        let product_id = dto
            .product_id
            .ok_or_else(|| Error::biz(CODE_BAD_REQUEST, MSG_PRODUCT_ID_REQUIRED))?;
        let mode = normalize_multi_spec_mode(Some(
            dto.multi_spec_price_mode
                .unwrap_or(DEFAULT_MULTI_SPEC_PRICE_MODE),
        ))
        .map_err(|m| Error::biz(CODE_BAD_REQUEST, m))?;
        let dimension = normalize_moq_dimension(Some(
            dto.moq_dimension.unwrap_or(DEFAULT_MOQ_DIMENSION),
        ))
        .map_err(|m| Error::biz(CODE_BAD_REQUEST, m))?;
        self.create_config(product_id, mode, dimension, shop_id).await?;
        Ok(product_id)
    }

    /// 批量添加批发商品（Java `batchAdd`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`（单商品 5310 / 5311 整批抛错）。
    pub async fn batch_add(
        &self,
        dto: &WholesaleProductBatchAddDTO,
        shop_id: i32,
    ) -> Result<WholesaleProductBatchAddVO> {
        let raw = dto.product_ids.as_deref().unwrap_or(&[]);
        if raw.is_empty() {
            return Err(Error::biz(CODE_BAD_REQUEST, MSG_PRODUCT_IDS_REQUIRED));
        }
        if raw.len() > 50 {
            return Err(Error::biz(
                CODE_BAD_REQUEST,
                "批量数量必须在 1-50 之间",
            ));
        }
        let distinct: Vec<i32> = nextrade_core::models::wholesale::wholesaler::distinct_ids(raw);
        let existing: Vec<i32> = self
            .repo()
            .find_active_configs(&distinct)
            .await?
            .into_iter()
            .map(|c| c.product_id)
            .collect();
        let mut result = WholesaleProductBatchAddVO::default();
        for pid in distinct {
            if existing.contains(&pid) {
                result.skipped_ids.push(pid);
                continue;
            }
            match self
                .create_config(
                    pid,
                    DEFAULT_MULTI_SPEC_PRICE_MODE,
                    DEFAULT_MOQ_DIMENSION,
                    shop_id,
                )
                .await
            {
                Ok(_) => result.success_count += 1,
                Err(Error::Biz { code, msg }) if code == CODE_NON_PLATFORM_PRODUCT || code == CODE_PRODUCT_TYPE_NOT_SUPPORTED => {
                    result.failed_items.push(WholesaleProductBatchAddFailedItem {
                        product_id: pid,
                        error_code: code,
                        error_message: msg,
                    });
                }
                Err(e) => return Err(e),
            }
        }
        Ok(result)
    }

    /// 批量删除批发商品（Java `batchDelete`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn batch_delete(&self, dto: &WholesaleBatchDeleteDTO, shop_id: i32) -> Result<i32> {
        let ids: Vec<i32> = dto
            .product_ids
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .copied()
            .filter(|id| *id > 0)
            .collect();
        if ids.is_empty() {
            return Ok(0);
        }
        let ids = nextrade_core::models::wholesale::wholesaler::distinct_ids(&ids);
        self.require_ownership(&ids, shop_id).await?;
        self.repo().delete_prices_by_products(&ids).await?;
        self.repo().soft_delete_sku_configs(&ids, now_secs()).await?;
        let deleted = self.repo().hard_delete_configs(&ids).await?;
        Ok(i32::try_from(deleted).unwrap_or(i32::MAX))
    }

    // ————————————————————— 批发价 —————————————————————

    /// 批发价详情（Java `WholesaleProductController#buildPriceDetail`）。
    ///
    /// # Errors
    /// 5310（配置不存在）/ 5314（跨店）。
    pub async fn price_detail(&self, product_id: i32, shop_id: i32) -> Result<WholesalePriceDetailVO> {
        self.require_ownership(&[product_id], shop_id).await?;
        let config = self
            .repo()
            .find_active_config(product_id)
            .await?
            .ok_or_else(|| Error::biz(CODE_PRODUCT_NOT_FOUND, MSG_PRODUCT_NOT_FOUND_OR_CONFIG))?;
        let skus = self.repo().find_enabled_skus(product_id).await?;
        let prices = self.repo().list_prices(product_id).await?;
        let grade_names = self.grade_name_map(&prices).await?;
        let currency = self.repo().currency_symbol().await?;
        let _ = currency;

        let mut sku_vos: Vec<WholesaleSkuPriceVO> = Vec::new();
        for sku in &skus {
            let rows = prices
                .iter()
                .filter(|r| belongs_to_sku(r.sku_id, sku.sku_id))
                .map(|r| to_price_row_vo(r, &grade_names))
                .collect();
            sku_vos.push(WholesaleSkuPriceVO {
                sku_id: sku.sku_id,
                sku_spec_text: Some(sku.sku_value.clone()),
                retail_price: sku.sku_price,
                price_rows: sorted_rows(rows),
            });
        }
        if sku_vos.is_empty() {
            let product = self.repo().find_product(product_id).await?;
            sku_vos.push(WholesaleSkuPriceVO {
                sku_id: SKU_SPU_FALLBACK,
                sku_spec_text: Some(SINGLE_SPEC_SPEC_TEXT.to_string()),
                retail_price: product.map(|p| p.product_price),
                price_rows: sorted_rows(
                    prices
                        .iter()
                        .filter(|r| belongs_to_sku(r.sku_id, SKU_SPU_FALLBACK))
                        .map(|r| to_price_row_vo(r, &grade_names))
                        .collect(),
                ),
            });
        }
        Ok(WholesalePriceDetailVO {
            product_id,
            is_multi_spec: skus.len() > 1,
            multi_spec_price_mode: Some(config.multi_spec_price_mode),
            price_mode: prices.first().map(|r| r.price_mode),
            skus: sku_vos,
        })
    }

    /// 批发价保存（Java `WholesalePriceServiceImpl#save`，全量替换）。
    ///
    /// # Errors
    /// 400 / 5310 / 5303 / 5304 / 5312。
    pub async fn price_save(&self, dto: &WholesalePriceSaveDTO, shop_id: i32) -> Result<()> {
        let product_id = dto
            .product_id
            .ok_or_else(|| Error::biz(CODE_BAD_REQUEST, MSG_PRODUCT_ID_REQUIRED))?;
        self.require_ownership(&[product_id], shop_id).await?;
        if self.repo().find_active_config(product_id).await?.is_none() {
            return Err(Error::biz(CODE_PRODUCT_NOT_FOUND, MSG_PRODUCT_NOT_CONFIGURED));
        }
        let rows = dto.price_rows.as_deref().unwrap_or(&[]);
        let validated =
            validate_price_rows(rows).map_err(|m| Error::biz(CODE_PRICE_INVALID, m))?;
        for row in &validated {
            if row.grade_id > 0 {
                self.require_grade_enabled(row.grade_id).await?;
            }
        }
        self.replace_prices(product_id, &validated, shop_id).await?;
        if let Some(mode) = dto.multi_spec_price_mode {
            let mode = normalize_multi_spec_mode(Some(mode))
                .map_err(|m| Error::biz(CODE_BAD_REQUEST, m))?;
            if let Some(mut config) = self.repo().find_config(product_id).await? {
                config.multi_spec_price_mode = mode;
                config.update_time = Some(now_secs());
                self.repo().update_config(&config).await?;
            }
        }
        Ok(())
    }

    /// 批量设置通用售价（Java `batchSetCommon`），返回写入行数。
    ///
    /// # Errors
    /// 400 / 5310 / 5312。
    pub async fn batch_set_common(&self, dto: &WholesaleBatchCommonDTO, shop_id: i32) -> Result<i32> {
        let product_id = dto
            .product_id
            .ok_or_else(|| Error::biz(CODE_BAD_REQUEST, MSG_PRODUCT_ID_REQUIRED))?;
        self.require_ownership(&[product_id], shop_id).await?;
        if self.repo().find_active_config(product_id).await?.is_none() {
            return Err(Error::biz(CODE_PRODUCT_NOT_FOUND, MSG_PRODUCT_NOT_CONFIGURED));
        }
        let mode = require_price_mode(dto.price_mode).map_err(|m| Error::biz(CODE_PRICE_INVALID, m))?;
        let target_skus = resolve_target_skus(dto.sku_ids.as_deref())
            .map_err(|m| Error::biz(CODE_PRICE_INVALID, m))?;
        let all_sku = dto.sku_ids.as_deref().is_none_or(<[i32]>::is_empty);
        let tiers = dto.tiers.as_deref().unwrap_or(&[]);
        let validated_tiers =
            validate_tiers(tiers, mode).map_err(|m| Error::biz(CODE_PRICE_INVALID, m))?;
        let now = now_secs();
        let new_rows = build_batch_rows(product_id, shop_id, &target_skus, GRADE_UNIVERSAL, mode, &validated_tiers, now);
        self.merge_and_replace(product_id, shop_id, new_rows, &target_skus, all_sku, &[GRADE_UNIVERSAL], now).await
    }

    /// 批量设置类型售价（Java `batchSetByGrade`），返回写入行数。
    ///
    /// # Errors
    /// 400 / 5310 / 5303 / 5304 / 5312。
    pub async fn batch_set_grade(&self, dto: &WholesaleBatchGradeDTO, shop_id: i32) -> Result<i32> {
        let product_id = dto
            .product_id
            .ok_or_else(|| Error::biz(CODE_BAD_REQUEST, MSG_PRODUCT_ID_REQUIRED))?;
        self.require_ownership(&[product_id], shop_id).await?;
        if self.repo().find_active_config(product_id).await?.is_none() {
            return Err(Error::biz(CODE_PRODUCT_NOT_FOUND, MSG_PRODUCT_NOT_CONFIGURED));
        }
        let mode = require_price_mode(dto.price_mode).map_err(|m| Error::biz(CODE_PRICE_INVALID, m))?;
        let grade_rows = dto.grade_rows.as_deref().unwrap_or(&[]);
        if grade_rows.is_empty() {
            return Err(Error::biz(CODE_BAD_REQUEST, MSG_GRADE_ROWS_REQUIRED));
        }
        let target_skus = resolve_target_skus(dto.sku_ids.as_deref())
            .map_err(|m| Error::biz(CODE_PRICE_INVALID, m))?;
        let all_sku = dto.sku_ids.as_deref().is_none_or(<[i32]>::is_empty);
        let now = now_secs();
        let mut grade_ids: Vec<i32> = Vec::new();
        let mut new_rows: Vec<ProductWholesalePrice> = Vec::new();
        for gr in grade_rows {
            let grade_id = gr.grade_id.ok_or_else(|| Error::biz(CODE_PRICE_INVALID, MSG_GRADE_ID_INVALID))?;
            if grade_id <= 0 {
                return Err(Error::biz(CODE_PRICE_INVALID, MSG_GRADE_ID_INVALID));
            }
            if grade_ids.contains(&grade_id) {
                return Err(Error::biz(
                    CODE_PRICE_INVALID,
                    format!("gradeRows 中 gradeId 重复：{grade_id}"),
                ));
            }
            grade_ids.push(grade_id);
            self.require_grade_enabled(grade_id).await?;
            let tiers = gr.tiers.as_deref().unwrap_or(&[]);
            let validated = validate_tiers(tiers, mode)
                .map_err(|m| Error::biz(CODE_PRICE_INVALID, m))?;
            new_rows.extend(build_batch_rows(product_id, shop_id, &target_skus, grade_id, mode, &validated, now));
        }
        let written = new_rows.len();
        self.merge_and_replace(product_id, shop_id, new_rows, &target_skus, all_sku, &grade_ids, now).await?;
        Ok(i32::try_from(written).unwrap_or(i32::MAX))
    }

    // ————————————————————— 起批规则 / 倍数 —————————————————————

    /// 起批规则详情（Java `buildBuyRuleDetail`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn buy_rule_detail(
        &self,
        product_id: i32,
        shop_id: i32,
    ) -> Result<WholesaleProductBuyRuleVO> {
        self.require_ownership(&[product_id], shop_id).await?;
        let Some(config) = self.repo().find_active_config(product_id).await? else {
            return Ok(WholesaleProductBuyRuleVO {
                product_id,
                configured: false,
                sku_rules: Vec::new(),
                ..Default::default()
            });
        };
        let sku_configs = self.repo().list_sku_configs(product_id).await?;
        let mut sku_rules: Vec<WholesaleSkuRuleVO> = sku_configs
            .iter()
            .map(|c| WholesaleSkuRuleVO {
                sku_id: c.sku_id,
                multiple: Some(c.multiple),
            })
            .collect();
        // 单规格 / 无 SKU 行商品：保证至少一条 sku_id=0 规则供前端录入
        let skus = self.repo().find_enabled_skus(product_id).await?;
        let has_spu_rule = sku_rules.iter().any(|r| r.sku_id == SKU_SPU_FALLBACK);
        if skus.is_empty() && !has_spu_rule {
            sku_rules.push(WholesaleSkuRuleVO {
                sku_id: SKU_SPU_FALLBACK,
                multiple: None,
            });
        }
        let hint = build_hint(
            config.moq_dimension,
            config.moq,
            &sku_rules.iter().map(|r| r.multiple).collect::<Vec<_>>(),
        );
        Ok(WholesaleProductBuyRuleVO {
            product_id,
            configured: true,
            dimension: Some(config.moq_dimension),
            moq: Some(config.moq),
            sku_rules,
            hint: Some(hint),
        })
    }

    /// 起批量设置（Java `updateMoq`）。
    ///
    /// # Errors
    /// 400 / 5310 / 5314。
    pub async fn buy_rule_save(&self, dto: &WholesaleBuyRuleSaveDTO) -> Result<()> {
        let product_id = dto
            .product_id
            .ok_or_else(|| Error::biz(CODE_BAD_REQUEST, MSG_PRODUCT_ID_REQUIRED))?;
        let dimension = normalize_moq_dimension(dto.dimension)
            .map_err(|m| Error::biz(CODE_BAD_REQUEST, m))?;
        let moq = match dto.moq {
            Some(v) if v >= 1 => v,
            _ => return Err(Error::biz(CODE_BAD_REQUEST, MSG_MOQ_INVALID)),
        };
        let config = self
            .repo()
            .find_config(product_id)
            .await?
            .ok_or_else(|| Error::biz(CODE_PRODUCT_NOT_FOUND, MSG_PRODUCT_NOT_CONFIGURED))?;
        self.assert_owned(config.shop_id, self.default_shop_hint())?;
        let mut config = config;
        config.moq_dimension = dimension;
        config.moq = moq;
        config.update_time = Some(now_secs());
        self.repo().update_config(&config).await?;
        Ok(())
    }

    /// 起批量批量设置（Java `batchUpdateMoq`），返回更新行数。
    ///
    /// # Errors
    /// 400 / 5310（任一商品未配置即整批报错）/ 5314。
    pub async fn buy_rule_batch_save(&self, dto: &WholesaleBuyRuleBatchSaveDTO) -> Result<i32> {
        let raw = dto.product_ids.as_deref().unwrap_or(&[]);
        if raw.is_empty() {
            return Err(Error::biz(CODE_BAD_REQUEST, MSG_PRODUCT_IDS_REQUIRED));
        }
        if raw.len() > 50 {
            return Err(Error::biz(CODE_BAD_REQUEST, "批量数量必须在 1-50 之间"));
        }
        let dimension = normalize_moq_dimension(dto.dimension)
            .map_err(|m| Error::biz(CODE_BAD_REQUEST, m))?;
        let moq = match dto.moq {
            Some(v) if v >= 1 => v,
            _ => return Err(Error::biz(CODE_BAD_REQUEST, MSG_MOQ_INVALID)),
        };
        let distinct = nextrade_core::models::wholesale::wholesaler::distinct_ids(raw);
        let configs = self.repo().find_configs_by_product_ids(&distinct).await?;
        let by_pid: HashMap<i32, ProductWholesaleConfig> =
            configs.into_iter().map(|c| (c.product_id, c)).collect();
        let missing: Vec<String> = distinct
            .iter()
            .filter(|pid| !by_pid.contains_key(pid))
            .map(|pid| pid.to_string())
            .collect();
        if !missing.is_empty() {
            return Err(Error::biz(
                CODE_PRODUCT_NOT_FOUND,
                format!("以下商品未配置批发能力：{}", missing.join(", ")),
            ));
        }
        for c in by_pid.values() {
            self.assert_owned(c.shop_id, self.default_shop_hint())?;
        }
        let now = now_secs();
        let mut updated = 0;
        for pid in &distinct {
            if let Some(mut config) = by_pid.get(pid).cloned() {
                config.moq_dimension = dimension;
                config.moq = moq;
                config.update_time = Some(now);
                if self.repo().update_config(&config).await? {
                    updated += 1;
                }
            }
        }
        Ok(updated)
    }

    /// 倍数购买保存（Java `multipleSave` → `batchUpsertMultiple`）。
    ///
    /// # Errors
    /// 400（缺字段 / skuId 负数 / multiple < 1）/ 5310 / 5314。
    pub async fn multiple_save(&self, dto: &WholesaleMultipleSaveDTO, shop_id: i32) -> Result<()> {
        let product_id = dto
            .product_id
            .ok_or_else(|| Error::biz(CODE_BAD_REQUEST, MSG_PRODUCT_ID_REQUIRED))?;
        self.require_ownership(&[product_id], shop_id).await?;
        let rows = dto.sku_multiples.as_deref().unwrap_or(&[]);
        if rows.is_empty() {
            return Err(Error::biz(CODE_BAD_REQUEST, "skuMultiples 不能为空"));
        }
        let now = now_secs();
        for row in rows {
            let sku_id = row.sku_id.ok_or_else(|| Error::biz(CODE_BAD_REQUEST, "skuId 不能为空"))?;
            if sku_id < 0 {
                return Err(Error::biz(CODE_BAD_REQUEST, "skuId 不能为负数"));
            }
            let multiple = row.multiple.ok_or_else(|| Error::biz(CODE_BAD_REQUEST, "multiple 不能为空"))?;
            if multiple < 1 {
                return Err(Error::biz(CODE_BAD_REQUEST, "multiple 必须 ≥ 1"));
            }
            self.upsert_multiple(product_id, shop_id, sku_id, multiple, now).await?;
        }
        Ok(())
    }

    // ————————————————————— 内部 —————————————————————

    /// 当前操作者 shopId（本任务控制台默认平台 0；Java 取请求头）。
    fn default_shop_hint(&self) -> i32 {
        0
    }

    /// 归属断言：`config.shop_id` 必须与当前操作者一致（否则 5314）。
    fn assert_owned(&self, owner: i32, shop_id: i32) -> Result<()> {
        if owner != shop_id {
            return Err(Error::biz(CODE_NON_PLATFORM_PRODUCT, MSG_NON_PLATFORM_PRODUCT));
        }
        Ok(())
    }

    /// 批量归属校验（Java `requireOwnership(List)`：无配置行不报错，
    /// 交给业务方法的 5310）。
    async fn require_ownership(&self, product_ids: &[i32], shop_id: i32) -> Result<()> {
        let configs = self.repo().find_configs_by_product_ids(product_ids).await?;
        for c in configs {
            self.assert_owned(c.shop_id, shop_id)?;
        }
        Ok(())
    }

    /// 等级必须存在且启用（Java `requireEnabled`）。
    async fn require_grade_enabled(&self, grade_id: i32) -> Result<()> {
        match self.repo().grade_enabled(grade_id).await? {
            Some(true) => Ok(()),
            Some(false) => Err(Error::biz(CODE_GRADE_DISABLED, "批发等级已禁用")),
            None => Err(Error::biz(CODE_GRADE_NOT_FOUND, "批发等级不存在")),
        }
    }

    /// 创建商品级批发配置（Java `create`，含 5310/5314/5315/5311 校验）。
    async fn create_config(
        &self,
        product_id: i32,
        multi_spec_price_mode: i32,
        moq_dimension: i32,
        shop_id: i32,
    ) -> Result<i32> {
        let Some(product) = self.repo().find_product(product_id).await? else {
            return Err(Error::biz(CODE_PRODUCT_NOT_FOUND, MSG_PRODUCT_NOT_FOUND));
        };
        self.assert_owned(product.shop_id, shop_id)?;
        if product.product_type.is_some_and(|t| t != 1) {
            return Err(Error::biz(CODE_PRODUCT_TYPE_NOT_SUPPORTED, MSG_PRODUCT_TYPE_NOT_SUPPORTED));
        }
        if product.sale_mode > 0 {
            return Err(Error::biz(CODE_PRODUCT_TYPE_NOT_SUPPORTED, MSG_PRESALE_NOT_SUPPORTED));
        }
        if product.delivery_mode != 1 {
            return Err(Error::biz(CODE_PRODUCT_TYPE_NOT_SUPPORTED, MSG_CYCLE_NOT_SUPPORTED));
        }
        if self.repo().exists_any_config(product_id).await? {
            return Err(Error::biz(CODE_PRODUCT_ALREADY_CONFIGURED, MSG_PRODUCT_ALREADY_CONFIGURED));
        }
        let now = now_secs();
        let config = ProductWholesaleConfig {
            id: 0,
            product_id,
            shop_id: product.shop_id,
            multi_spec_price_mode,
            moq_dimension,
            moq: DEFAULT_MOQ,
            status: 1,
            is_delete: 0,
            create_time: Some(now),
            update_time: Some(now),
        };
        self.repo().insert_config(&config).await
    }

    /// 列表行装配（Java `assemble`）。
    #[allow(clippy::too_many_arguments)]
    async fn to_product_vo(
        &self,
        config: &ProductWholesaleConfig,
        product: Option<&nextrade_core::entities::Product>,
        _skus: &[&nextrade_core::entities::ProductSku],
        _unused: &[i32],
        sku_configs: &[ProductWholesaleSkuConfig],
        prices: &[ProductWholesalePrice],
        currency: &str,
    ) -> Result<WholesaleProductVO> {
        let skus = self.repo().find_enabled_skus(config.product_id).await?;
        let sku_prices: Vec<rust_decimal::Decimal> = skus.iter().filter_map(|s| s.sku_price).collect();
        let retail = format_retail_range(&sku_prices, product.map(|p| p.product_price), currency);
        let wholesale = self.format_price_range(prices, currency);
        let multiple_set = sku_configs
            .iter()
            .any(|c| c.product_id == config.product_id && c.multiple > 1);
        Ok(WholesaleProductVO {
            product_id: config.product_id,
            product_name: product.map(|p| p.product_name.clone()),
            product_sn: product.map(|p| p.product_sn.clone()),
            product_no: product.and_then(|p| p.product_tsn.clone()),
            pic_url: product.map(|p| p.pic_url.clone()),
            pic_thumb: product.map(|p| p.pic_thumb.clone()),
            retail_price_range: retail,
            wholesale_price_display: wholesale,
            moq_dimension: Some(config.moq_dimension),
            moq_dimension_name: if config.moq_dimension == DEFAULT_MOQ_DIMENSION {
                "SPU".to_string()
            } else {
                "SKU".to_string()
            },
            moq_display: Some(config.moq),
            multiple_set,
            stock: product.map(|p| p.product_stock),
            shelf_status: product.map(|p| p.product_status),
            config_status: Some(config.status),
        })
    }

    /// 批发价区间预览（Java `formatPriceRange`：逐 SKU 等级价优先，再汇总）。
    fn format_price_range(&self, rows: &[ProductWholesalePrice], currency: &str) -> String {
        let usable: Vec<&ProductWholesalePrice> = rows
            .iter()
            .filter(|r| {
                if r.price_mode == 1 {
                    is_discount_in_range(Some(r.discount))
                } else {
                    r.wholesale_price > rust_decimal::Decimal::ZERO
                }
            })
            .collect();
        if usable.is_empty() {
            return EMPTY_DISPLAY.to_string();
        }
        let mut by_sku: HashMap<i32, Vec<&ProductWholesalePrice>> = HashMap::new();
        for r in &usable {
            by_sku.entry(r.sku_id).or_default().push(r);
        }
        let mut source: Vec<&ProductWholesalePrice> = Vec::new();
        for rows_of_sku in by_sku.values() {
            let graded: Vec<&ProductWholesalePrice> =
                rows_of_sku.iter().copied().filter(|r| r.grade_id > GRADE_UNIVERSAL).collect();
            if graded.is_empty() {
                source.extend(rows_of_sku.iter().copied().filter(|r| r.grade_id <= GRADE_UNIVERSAL));
            } else {
                source.extend(graded);
            }
        }
        let mut discounts: Vec<rust_decimal::Decimal> = Vec::new();
        let mut amounts: Vec<rust_decimal::Decimal> = Vec::new();
        for r in source {
            if r.price_mode == 1 {
                discounts.push(r.discount);
            } else {
                amounts.push(r.wholesale_price);
            }
        }
        let mut segments: Vec<String> = Vec::new();
        if !discounts.is_empty() {
            let min = discounts.iter().copied().min().unwrap_or_default();
            let max = discounts.iter().copied().max().unwrap_or_default();
            segments.push(if min == max {
                format_discount(min)
            } else {
                format!("{}-{}", format_discount(min), format_discount(max))
            });
        }
        if !amounts.is_empty() {
            let min = amounts.iter().copied().min().unwrap_or_default();
            let max = amounts.iter().copied().max().unwrap_or_default();
            segments.push(if min == max {
                format_money(currency, min)
            } else {
                format!("{}-{}", format_money(currency, min), format_money(currency, max))
            });
        }
        if segments.is_empty() {
            EMPTY_DISPLAY.to_string()
        } else {
            segments.join(" / ")
        }
    }

    /// 等级名称映射（`grade_id=0` → 「通用价」）。
    async fn grade_name_map(&self, prices: &[ProductWholesalePrice]) -> Result<HashMap<i32, String>> {
        let ids: Vec<i32> = prices.iter().map(|p| p.grade_id).filter(|id| *id > 0).collect();
        let mut map: HashMap<i32, String> = self.repo().grade_names(&ids).await?.into_iter().collect();
        map.insert(GRADE_UNIVERSAL, GRADE_UNIVERSAL_NAME.to_string());
        Ok(map)
    }

    /// 全量替换价格行（Java `persistFullReplace`）。
    async fn replace_prices(
        &self,
        product_id: i32,
        rows: &[nextrade_core::models::wholesale::wholesale_product::ValidatedPriceRow],
        _shop_id: i32,
    ) -> Result<()> {
        self.repo().delete_prices(product_id).await?;
        let now = now_secs();
        for r in rows {
            let p = ProductWholesalePrice {
                id: 0,
                product_id,
                shop_id: 0,
                sku_id: r.sku_id,
                grade_id: r.grade_id,
                tier_min_qty: r.tier_min_qty,
                price_mode: r.price_mode,
                discount: r.discount,
                wholesale_price: r.wholesale_price,
                status: 1,
                is_delete: 0,
                create_time: Some(now),
                update_time: Some(now),
            };
            self.repo().insert_price(&p).await?;
        }
        Ok(())
    }

    /// 读-合并-整表替换（Java `batchSetCommon` / `batchSetByGrade`）。
    #[allow(clippy::too_many_arguments)]
    async fn merge_and_replace(
        &self,
        product_id: i32,
        _shop_id: i32,
        new_rows: Vec<ProductWholesalePrice>,
        target_skus: &[i32],
        all_sku: bool,
        target_grade_ids: &[i32],
        _now: i64,
    ) -> Result<i32> {
        let kept: Vec<ProductWholesalePrice> = self
            .repo()
            .list_prices(product_id)
            .await?
            .into_iter()
            .filter(|p| {
                let sku_hit = all_sku || target_skus.contains(&p.sku_id);
                let grade_hit = target_grade_ids.contains(&p.grade_id);
                !(grade_hit && sku_hit)
            })
            .collect();
        let count = i32::try_from(new_rows.len()).unwrap_or(i32::MAX);
        self.repo().delete_prices(product_id).await?;
        let now = now_secs();
        for p in kept.iter().chain(new_rows.iter()) {
            let mut row = p.clone();
            row.id = 0;
            row.product_id = product_id;
            row.create_time = Some(now);
            row.update_time = Some(now);
            self.repo().insert_price(&row).await?;
        }
        Ok(count)
    }

    /// SKU 倍数 upsert（Java `upsert`，存在则更新、不存在则默认 multiple=1）。
    async fn upsert_multiple(
        &self,
        product_id: i32,
        shop_id: i32,
        sku_id: i32,
        multiple: i32,
        now: i64,
    ) -> Result<()> {
        if let Some(existing) = self.repo().find_sku_config(product_id, sku_id).await? {
            self.repo().update_sku_multiple(existing.id, multiple, now).await?;
            return Ok(());
        }
        let entity = ProductWholesaleSkuConfig {
            id: 0,
            product_id,
            shop_id,
            sku_id,
            multiple,
            is_delete: 0,
            create_time: Some(now),
            update_time: Some(now),
        };
        self.repo().insert_sku_config(&entity).await?;
        Ok(())
    }
}

/// 价格行实体 → 行 VO（Java `toPriceRowVO`）。
fn to_price_row_vo(
    r: &ProductWholesalePrice,
    grade_names: &HashMap<i32, String>,
) -> WholesalePriceRowVO {
    WholesalePriceRowVO {
        grade_id: Some(r.grade_id),
        grade_name: if r.grade_id == GRADE_UNIVERSAL {
            Some(GRADE_UNIVERSAL_NAME.to_string())
        } else {
            grade_names.get(&r.grade_id).cloned()
        },
        tier_min_qty: Some(r.tier_min_qty),
        price_mode: Some(r.price_mode),
        discount: Some(r.discount),
        wholesale_price: Some(r.wholesale_price),
    }
}

/// 按 (gradeId, tierMinQty) 排序（Java `Comparator.comparing(...)`）。
fn sorted_rows(mut rows: Vec<WholesalePriceRowVO>) -> Vec<WholesalePriceRowVO> {
    rows.sort_by(|a, b| {
        a.grade_id
            .cmp(&b.grade_id)
            .then(a.tier_min_qty.cmp(&b.tier_min_qty))
    });
    rows
}

/// 构建批量阶梯行（Java `buildBatchRows`）。
fn build_batch_rows(
    product_id: i32,
    shop_id: i32,
    skus: &[i32],
    grade_id: i32,
    mode: i32,
    tiers: &[(i32, rust_decimal::Decimal, rust_decimal::Decimal)],
    now: i64,
) -> Vec<ProductWholesalePrice> {
    let mut out = Vec::with_capacity(skus.len() * tiers.len());
    for sku_id in skus {
        for (min_qty, discount, wholesale_price) in tiers {
            out.push(ProductWholesalePrice {
                id: 0,
                product_id,
                shop_id,
                sku_id: *sku_id,
                grade_id,
                tier_min_qty: *min_qty,
                price_mode: mode,
                discount: *discount,
                wholesale_price: *wholesale_price,
                status: 1,
                is_delete: 0,
                create_time: Some(now),
                update_time: Some(now),
            });
        }
    }
    out
}

/// 当前 Unix 秒（批发表时间为 bigint 秒级）。
fn now_secs() -> i64 {
    i64::try_from(nextrade_common::utils::now_seconds()).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 批量阶梯行 = SKU × tier 笛卡尔积（Java `buildBatchRows`）。
    #[test]
    fn batch_rows_build_cartesian_product() {
        let tiers = vec![(10, rust_decimal::Decimal::from(5), rust_decimal::Decimal::ZERO)];
        let rows = build_batch_rows(1, 0, &[0, 11], 0, 1, &tiers, 1_700_000_000);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].sku_id, 0);
        assert_eq!(rows[1].sku_id, 11);
        assert_eq!(rows[0].grade_id, 0);
        assert_eq!(rows[0].price_mode, 1);
        assert_eq!(rows[0].status, 1);
    }

    /// 价格行 VO 排序：先 gradeId 再 tierMinQty（Java Comparator）。
    #[test]
    fn price_rows_sort_matches_java_comparator() {
        let names = HashMap::new();
        let row = |grade: i32, qty: i32| WholesalePriceRowVO {
            grade_id: Some(grade),
            grade_name: None,
            tier_min_qty: Some(qty),
            price_mode: Some(2),
            discount: None,
            wholesale_price: Some(rust_decimal::Decimal::from(qty)),
        };
        let sorted = sorted_rows(vec![row(2, 5), row(0, 20), row(0, 10)]);
        assert_eq!(
            sorted
                .iter()
                .map(|r| (r.grade_id.unwrap(), r.tier_min_qty.unwrap()))
                .collect::<Vec<_>>(),
            vec![(0, 10), (0, 20), (2, 5)]
        );
        let price = ProductWholesalePrice {
            grade_id: 0,
            price_mode: 2,
            wholesale_price: rust_decimal::Decimal::from(10),
            ..Default::default()
        };
        let vo = to_price_row_vo(&price, &names);
        assert_eq!(vo.grade_name.as_deref(), Some(GRADE_UNIVERSAL_NAME));
    }
}
