//! 商品对接 API 服务（catalog::product_feed）。
//!
//! 功能规划：`~/workspace/product-feed-api-design/proposal.md`（P1）。
//!
//! 两副面孔：
//! - **管理面**（adminapi，管理员 JWT）：密钥 CRUD、重置 secret、
//!   单品价格覆盖、密钥可见商品预览。secret 明文只在新建/重置时
//!   随响应返回一次，库里只存 SHA-256 摘要。
//! - **对接面**（`/api/feed/v1`，密钥鉴权）：meta / 分类树 / 商品
//!   列表 / 商品详情。只出上架未删且授权内的商品；价格全部为
//!   规则后价格，响应不含原价/成本字段（规划 §3.6）。
//!
//! # 金额：`Decimal`；时间：Unix 秒（i32，与 `product.last_update` 同口径）。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use rust_decimal::Decimal;

use nextrade_common::utils;
use nextrade_core::entities::{Category, Product, ProductSku};
use nextrade_core::models::catalog::product_feed::{
    content_hash_canonical, expand_categories, is_inquiry_product, market_price_out, resolve_rule,
    validate_key_fields, validate_rule, ContentHashParts, FeedCategoryPathVO, FeedCategoryVO,
    FeedIdDTO, FeedImageVO, FeedKey, FeedKeyCreateDTO, FeedKeyDetailVO, FeedKeyListQuery,
    FeedKeySecretVO, FeedKeyUpdateDTO, FeedKeyVO, FeedMetaVO, FeedPriceOverride,
    FeedPriceOverrideSaveDTO, FeedPriceOverrideVO, FeedProductDetailVO, FeedProductListQuery,
    FeedAudience, FeedProductRow, FeedProductVO, FeedScopeProductVO, FeedSkuVO, PriceRule,
    FEED_MAX_PAGE_SIZE, MSG_FEED_DISABLED, MSG_FEED_EXPIRED, MSG_FEED_PRODUCT_NOT_FOUND,
    MSG_FEED_UNAUTHORIZED, MSG_KEY_NOT_FOUND, RULE_NONE, SCOPE_CATEGORIES, SCOPE_PRODUCTS,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::catalog::product_feed::{
    FeedProductFilter, ProductFeedRepository,
};
use nextrade_infrastructure::db::Repositories;

/// 商品对接服务（挂载于 `AdminState.catalog.product_feed` /
/// `PublicState.catalog.product_feed`）。
pub struct ProductFeedService {
    repos: Arc<Repositories>,
}

impl ProductFeedService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本域仓储（NESTED：`repos.catalog.product_feed`）。
    #[must_use]
    pub fn repo(&self) -> &ProductFeedRepository {
        &self.repos.catalog.product_feed
    }

    fn now(&self) -> i32 {
        i32::try_from(utils::now_seconds()).unwrap_or(i32::MAX)
    }

    fn default_rule(&self, key: &FeedKey) -> PriceRule {
        PriceRule::new(&key.price_rule_type, key.price_rule_value)
    }

    // ———————————————————— 管理面：密钥 CRUD ————————————————————

    /// 密钥分页列表（带每把密钥当前可见商品数）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn list_keys(&self, q: &FeedKeyListQuery) -> Result<PageResult<FeedKeyVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20).min(100),
        };
        let (limit, offset) = page.limit_offset();
        let (total, keys) = self
            .repo()
            .list_keys(q.keyword.as_deref(), q.status, limit, offset)
            .await?;
        let mut out = Vec::with_capacity(keys.len());
        for key in &keys {
            let count = self.count_visible(key).await?;
            out.push(key_to_vo(key, count));
        }
        Ok(PageResult::new(total, out, &page))
    }

    /// 密钥详情（主表 + 范围清单 + 单品覆盖回显）。
    ///
    /// # Errors
    /// 密钥不存在 → `NotFound`；数据库错误 → `Database`。
    pub async fn detail(&self, feed_key_id: i32) -> Result<FeedKeyDetailVO> {
        let key = self.load_key(feed_key_id).await?;
        let product_ids = self.repo().list_key_product_ids(feed_key_id).await?;
        let category_ids = self.repo().list_key_category_ids(feed_key_id).await?;
        let overrides = self.repo().list_overrides(feed_key_id).await?;
        let count = self.count_visible(&key).await?;

        // 覆盖回显：补商品名/原价/规则后价
        let ov_product_ids: Vec<i32> = overrides.iter().map(|o| o.product_id).collect();
        let products = self
            .repos
            .catalog
            .product
            .find_by_ids(&ov_product_ids)
            .await?;
        let by_id: HashMap<i32, &Product> =
            products.iter().map(|p| (p.product_id, p)).collect();
        let price_overrides = overrides
            .iter()
            .map(|o| {
                let p = by_id.get(&o.product_id).copied();
                FeedPriceOverrideVO {
                    product_id: o.product_id,
                    product_name: p.map(|p| p.product_name.clone()),
                    base_price: p.map(|p| p.product_price),
                    rule_type: o.rule_type.clone(),
                    rule_value: o.rule_value,
                    final_price: p.map(|p| {
                        PriceRule::new(&o.rule_type, o.rule_value).apply(p.product_price)
                    }),
                }
            })
            .collect();

        Ok(FeedKeyDetailVO {
            key: key_to_vo(&key, count),
            product_ids,
            category_ids,
            price_overrides,
        })
    }

    /// 新建密钥：生成 access_key/secret，secret 明文仅此一次返回。
    ///
    /// # Errors
    /// 入参校验失败 → `Validation`；数据库错误 → `Database`。
    pub async fn create(&self, dto: &FeedKeyCreateDTO) -> Result<FeedKeySecretVO> {
        let now = self.now();
        let scope_mode = dto.scope_mode.unwrap_or(1);
        let rule_type = dto
            .price_rule_type
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(RULE_NONE);
        let rule_value = dto.price_rule_value.unwrap_or(Decimal::ZERO);
        let expire_time = dto.expire_time.unwrap_or(0);
        let name = validate_key_fields(
            dto.key_name.as_deref(),
            scope_mode,
            &dto.product_ids,
            &dto.category_ids,
            rule_type,
            rule_value,
            expire_time,
            now,
        )
        .map_err(Error::Validation)?;

        let access_key = gen_access_key();
        let secret = gen_secret();
        let secret_hash = utils::sha256_hex(&secret);
        let id = self
            .repo()
            .insert_key(
                &name,
                &access_key,
                &secret_hash,
                scope_mode,
                i32::from(dto.include_inquiry.unwrap_or(false)),
                rule_type,
                rule_value,
                expire_time,
                dto.remark.as_deref().unwrap_or_default(),
                now,
            )
            .await?;
        self.repo()
            .replace_key_products(id, &dto.product_ids)
            .await?;
        self.repo()
            .replace_key_categories(id, &dto.category_ids)
            .await?;
        tracing::info!("新建对接密钥: id={id}, access_key={access_key}");
        Ok(FeedKeySecretVO {
            feed_key_id: id,
            access_key,
            secret,
        })
    }

    /// 编辑密钥（范围清单整体替换，入参即全量）。
    ///
    /// # Errors
    /// 密钥不存在 → `NotFound`；校验失败 → `Validation`。
    pub async fn update(&self, dto: &FeedKeyUpdateDTO) -> Result<()> {
        let existing = self.load_key(dto.feed_key_id).await?;
        let now = self.now();
        let scope_mode = dto.scope_mode.unwrap_or(i32::from(existing.scope_mode));
        let rule_type = dto
            .price_rule_type
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(&existing.price_rule_type)
            .to_string();
        let rule_value = dto.price_rule_value.unwrap_or(existing.price_rule_value);
        let expire_time = dto.expire_time.unwrap_or(existing.expire_time);
        let name = validate_key_fields(
            dto.key_name.as_deref().or(Some(&existing.key_name)),
            scope_mode,
            &dto.product_ids,
            &dto.category_ids,
            &rule_type,
            rule_value,
            expire_time,
            now,
        )
        .map_err(Error::Validation)?;
        self.repo()
            .update_key(
                dto.feed_key_id,
                &name,
                dto.status.unwrap_or(i32::from(existing.status)),
                scope_mode,
                dto.include_inquiry
                    .map(i32::from)
                    .unwrap_or(i32::from(existing.include_inquiry)),
                &rule_type,
                rule_value,
                expire_time,
                dto.remark.as_deref().unwrap_or(&existing.remark),
                now,
            )
            .await?;
        self.repo()
            .replace_key_products(dto.feed_key_id, &dto.product_ids)
            .await?;
        self.repo()
            .replace_key_categories(dto.feed_key_id, &dto.category_ids)
            .await?;
        Ok(())
    }

    /// 重置 secret：旧 secret 立即失效，新明文仅此一次返回。
    ///
    /// # Errors
    /// 密钥不存在 → `NotFound`。
    pub async fn reset_secret(&self, feed_key_id: i32) -> Result<FeedKeySecretVO> {
        let key = self.load_key(feed_key_id).await?;
        let secret = gen_secret();
        let secret_hash = utils::sha256_hex(&secret);
        self.repo()
            .update_secret_hash(feed_key_id, &secret_hash, self.now())
            .await?;
        tracing::info!("重置对接密钥 secret: id={feed_key_id}");
        Ok(FeedKeySecretVO {
            feed_key_id,
            access_key: key.access_key,
            secret,
        })
    }

    /// 删除密钥（连带范围与覆盖）。
    ///
    /// # Errors
    /// 密钥不存在 → `NotFound`。
    pub async fn delete(&self, feed_key_id: i32) -> Result<()> {
        self.load_key(feed_key_id).await?;
        self.repo().delete_key_scope(feed_key_id).await?;
        self.repo().delete_key(feed_key_id).await?;
        Ok(())
    }

    /// 保存单品价格覆盖（upsert）。
    ///
    /// # Errors
    /// 密钥/商品不存在 → `NotFound`；规则非法 → `Validation`。
    pub async fn save_override(&self, dto: &FeedPriceOverrideSaveDTO) -> Result<()> {
        self.load_key(dto.feed_key_id).await?;
        let product = self
            .repos
            .catalog
            .product
            .find_by_id(dto.product_id)
            .await?
            .ok_or_else(|| Error::NotFound("商品不存在".into()))?;
        let _ = product;
        let rule_type = dto
            .rule_type
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(RULE_NONE);
        let rule_value = dto.rule_value.unwrap_or(Decimal::ZERO);
        validate_rule(rule_type, rule_value).map_err(Error::Validation)?;
        self.repo()
            .upsert_override(
                dto.feed_key_id,
                dto.product_id,
                rule_type,
                rule_value,
                self.now(),
            )
            .await
    }

    /// 删除单品价格覆盖（幂等）。
    ///
    /// # Errors
    /// 数据库错误 → `Database`。
    pub async fn delete_override(&self, dto: &FeedIdDTO) -> Result<()> {
        let product_id = dto.product_id.unwrap_or(0);
        self.repo()
            .delete_override(dto.feed_key_id, product_id)
            .await?;
        Ok(())
    }

    /// 密钥可见商品预览（原价 vs 规则后价格，供后台选品/设覆盖）。
    ///
    /// # Errors
    /// 密钥不存在 → `NotFound`。
    pub async fn scope_products(
        &self,
        feed_key_id: i32,
        page: u64,
        size: u64,
        keyword: Option<&str>,
    ) -> Result<PageResult<FeedScopeProductVO>> {
        let key = self.load_key(feed_key_id).await?;
        let page_q = PageQuery {
            page: page.max(1),
            page_size: size.clamp(1, 100),
        };
        let (limit, offset) = page_q.limit_offset();
        let mut filter = self.scope_filter(&key).await?;
        filter.keyword = keyword.map(str::to_string);
        let total = self.repo().count_products(&filter).await?;
        let rows = self.repo().list_products(&filter, limit, offset).await?;
        let overrides = self.overrides_map(feed_key_id, &rows).await?;
        let default = self.default_rule(&key);
        let out = rows
            .iter()
            .map(|row| {
                let ov = overrides.get(&row.product_id);
                let rule = resolve_rule(&default, ov);
                let final_price = rule.apply(row.product_price);
                FeedScopeProductVO {
                    product_id: row.product_id,
                    product_name: row.product_name.clone(),
                    product_sn: row.product_sn.clone(),
                    pic_url: row.pic_url.clone(),
                    category_id: row.category_id,
                    base_price: row.product_price,
                    base_market_price: row.market_price,
                    final_price,
                    final_market_price: market_price_out(row.market_price, final_price),
                    stock: row.product_stock,
                    inquiry: is_inquiry_product(row.is_inquiry, row.sku_inquiry_max),
                    has_override: ov.is_some(),
                }
            })
            .collect();
        Ok(PageResult::new(total.max(0) as u64, out, &page_q))
    }

    // ———————————————————— 对接面：鉴权 ————————————————————

    /// 密钥鉴权：access_key 查钥 → secret 哈希恒定时间比对 →
    /// 状态/过期校验。成功后节流回写调用统计（>60 秒才写）。
    ///
    /// # Errors
    /// 密钥不存在/secret 错 → `Unauthorized`（统一文案防枚举）；
    /// 停用/过期 → `Forbidden`。
    pub async fn authenticate(&self, access_key: &str, secret: &str) -> Result<FeedKey> {
        let key = self
            .repo()
            .find_key_by_access_key(access_key.trim())
            .await?
            .ok_or_else(|| Error::Unauthorized(MSG_FEED_UNAUTHORIZED.into()))?;
        let presented = utils::sha256_hex(secret);
        if !constant_time_eq(presented.as_bytes(), key.secret_hash.as_bytes()) {
            return Err(Error::Unauthorized(MSG_FEED_UNAUTHORIZED.into()));
        }
        if key.status != 1 {
            return Err(Error::Forbidden(MSG_FEED_DISABLED.into()));
        }
        let now = self.now();
        if key.expire_time != 0 && key.expire_time < now {
            return Err(Error::Forbidden(MSG_FEED_EXPIRED.into()));
        }
        if now - key.last_used_time > 60 {
            let _ = self.repo().touch_usage(key.feed_key_id, now).await;
        }
        Ok(key)
    }

    // ———————————————————— 对接面：查询 ————————————————————

    /// `GET /api/feed/v1/meta`。
    ///
    /// # Errors
    /// 数据库错误 → `Database`。
    pub async fn meta(&self, key: &FeedKey) -> Result<FeedMetaVO> {
        Ok(FeedMetaVO {
            key_name: key.key_name.clone(),
            scope_mode: i32::from(key.scope_mode),
            include_inquiry: key.include_inquiry == 1,
            product_count: self.count_visible(key).await?,
            server_time: utils::now_seconds(),
            api_version: "v1".to_string(),
        })
    }

    // ———————————————————— 受众口径（feed 密钥与联邦凭证共用） ————————————————————

    /// 把一把 feed 密钥解析成受众（范围集合 + 默认规则 + 全部覆盖）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn audience_for_key(&self, key: &FeedKey) -> Result<FeedAudience> {
        let mut aud = FeedAudience {
            scope_mode: i32::from(key.scope_mode),
            include_inquiry: key.include_inquiry == 1,
            default_rule: self.default_rule(key),
            ..Default::default()
        };
        match i32::from(key.scope_mode) {
            SCOPE_PRODUCTS => {
                aud.product_ids = Some(
                    self.repo()
                        .list_key_product_ids(key.feed_key_id)
                        .await?
                        .into_iter()
                        .collect(),
                );
            }
            SCOPE_CATEGORIES => {
                let selected = self.repo().list_key_category_ids(key.feed_key_id).await?;
                let all = self.repo().list_all_categories().await?;
                let pairs: Vec<(i32, i32)> =
                    all.iter().map(|c| (c.category_id, c.parent_id)).collect();
                aud.category_ids = Some(expand_categories(&selected, &pairs));
            }
            _ => {}
        }
        aud.overrides = self
            .repo()
            .list_all_overrides(key.feed_key_id)
            .await?
            .into_iter()
            .map(|o| (o.product_id, PriceRule::new(&o.rule_type, o.rule_value)))
            .collect();
        Ok(aud)
    }

    /// 受众口径的可见商品数。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn count_for(&self, aud: &FeedAudience) -> Result<i64> {
        let filter = audience_filter(aud);
        self.repo().count_products(&filter).await
    }

    /// 受众口径的授权分类树。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn categories_for(&self, aud: &FeedAudience) -> Result<Vec<FeedCategoryVO>> {
        let filter = audience_filter(aud);
        let counts: HashMap<i32, i64> = self
            .repo()
            .count_products_by_category(&filter)
            .await?
            .into_iter()
            .collect();
        let all = self.repo().list_all_categories().await?;
        Ok(build_category_tree(&all, &counts))
    }

    /// 受众口径的商品分页（规则后价格 + 内容哈希）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn products_for(
        &self,
        aud: &FeedAudience,
        q: &FeedProductListQuery,
    ) -> Result<PageResult<FeedProductVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20).min(FEED_MAX_PAGE_SIZE),
        };
        let (limit, offset) = page.limit_offset();
        let mut filter = audience_filter(aud);
        filter.updated_since = q.updated_since.filter(|s| *s > 0);
        filter.keyword = q.keyword.clone();
        let all_categories = self.repo().list_all_categories().await?;
        if let Some(cat_id) = q.category_id.filter(|id| *id > 0) {
            let pairs: Vec<(i32, i32)> = all_categories
                .iter()
                .map(|c| (c.category_id, c.parent_id))
                .collect();
            filter.filter_category_ids =
                Some(expand_categories(&[cat_id], &pairs).into_iter().collect());
        }
        let total = self.repo().count_products(&filter).await?;
        let rows = self.repo().list_products(&filter, limit, offset).await?;
        let out = self.assemble_list(aud, &rows, &all_categories).await?;
        Ok(PageResult::new(total.max(0) as u64, out, &page))
    }

    /// `GET /api/feed/v1/categories`：只含授权内有商品的分类及其祖先链，
    /// `product_count` 为子树合计。
    ///
    /// # Errors
    /// 数据库错误 → `Database`。
    pub async fn categories(&self, key: &FeedKey) -> Result<Vec<FeedCategoryVO>> {
        let aud = self.audience_for_key(key).await?;
        self.categories_for(&aud).await
    }

    /// `GET /api/feed/v1/products`：授权内商品分页（规则后价格 + 内容哈希）。
    ///
    /// # Errors
    /// 数据库错误 → `Database`。
    pub async fn products(
        &self,
        key: &FeedKey,
        q: &FeedProductListQuery,
    ) -> Result<PageResult<FeedProductVO>> {
        let aud = self.audience_for_key(key).await?;
        self.products_for(&aud, q).await
    }

    /// `GET /api/feed/v1/products/detail`：越权/下架/不存在统一 404。
    ///
    /// # Errors
    /// 不可见 → `NotFound`（不区分不存在与无权，防探测）。
    pub async fn product_detail(
        &self,
        key: &FeedKey,
        product_id: i32,
    ) -> Result<FeedProductDetailVO> {
        let aud = self.audience_for_key(key).await?;
        self.product_detail_for(&aud, product_id).await
    }

    /// 受众口径的商品详情：越权/下架/不存在统一 404。
    ///
    /// # Errors
    /// 不可见 → `NotFound`（不区分不存在与无权，防探测）。
    pub async fn product_detail_for(
        &self,
        aud: &FeedAudience,
        product_id: i32,
    ) -> Result<FeedProductDetailVO> {
        let not_found = || Error::NotFound(MSG_FEED_PRODUCT_NOT_FOUND.into());
        let product = self
            .repos
            .catalog
            .product
            .find_by_id(product_id)
            .await?
            .ok_or_else(not_found)?;
        if product.product_status != 1 || product.is_delete != 0 {
            return Err(not_found());
        }
        // 范围校验（商品/分类/店铺三维，与列表过滤同口径）
        if !aud.contains(product_id, product.category_id, product.shop_id) {
            return Err(not_found());
        }
        let all_skus = self.repos.catalog.product.find_skus(product_id).await?;
        let enabled: Vec<&ProductSku> = all_skus.iter().filter(|s| s.is_enabled == 1).collect();
        let inquiry = product.is_inquiry == 1 || enabled.iter().any(|s| s.is_inquiry == 1);
        if !aud.include_inquiry && inquiry {
            return Err(not_found());
        }

        let rule = aud.rule_for(product_id);
        let price = rule.apply(product.product_price);
        let market = market_price_out(product.market_price, price);

        let gallery = self.repo().find_gallery(product_id).await?;
        let images: Vec<FeedImageVO> = if gallery.is_empty() {
            vec![FeedImageVO {
                pic_url: product.pic_url.clone(),
                pic_thumb: product.pic_thumb.clone(),
                pic_large: product.pic_large.clone(),
                pic_original: product.pic_original.clone(),
                sort_order: 0,
            }]
        } else {
            gallery
                .iter()
                .map(|g| FeedImageVO {
                    pic_url: g.pic_url.clone(),
                    pic_thumb: g.pic_thumb.clone(),
                    pic_large: g.pic_large.clone(),
                    pic_original: g.pic_original.clone(),
                    sort_order: g.sort_order,
                })
                .collect()
        };
        let image_urls: Vec<String> = if gallery.is_empty() {
            vec![product.pic_url.clone()]
        } else {
            gallery.iter().map(|g| g.pic_url.clone()).collect()
        };

        let skus: Vec<FeedSkuVO> = enabled
            .iter()
            .map(|s| {
                let sp = rule.apply(s.sku_price.unwrap_or(product.product_price));
                FeedSkuVO {
                    sku_id: s.sku_id,
                    sku_value: s.sku_value.clone(),
                    sku_data: s.sku_data.clone(),
                    sku_sn: s.sku_sn.clone(),
                    price: sp,
                    market_price: market_price_out(
                        s.market_price.unwrap_or(product.market_price),
                        sp,
                    ),
                    stock: s.sku_stock.unwrap_or(0),
                    is_enabled: s.is_enabled,
                    inquiry: s.is_inquiry == 1 || product.is_inquiry == 1,
                }
            })
            .collect();
        let sku_parts: Vec<(i32, String, Decimal, i32)> = enabled
            .iter()
            .map(|s| {
                (
                    s.sku_id,
                    s.sku_value.clone(),
                    rule.apply(s.sku_price.unwrap_or(product.product_price)),
                    s.sku_stock.unwrap_or(0),
                )
            })
            .collect();
        let content_hash = hash_of(&ContentHashParts {
            product_name: product.product_name.clone(),
            product_brief: product.product_brief.clone(),
            product_desc: product.product_desc.clone().unwrap_or_default(),
            category_id: product.category_id,
            brand_id: product.brand_id,
            price,
            stock: product.product_stock,
            inquiry,
            images: image_urls,
            skus: sku_parts,
        });

        let all_categories = self.repo().list_all_categories().await?;
        let cat_name = all_categories
            .iter()
            .find(|c| c.category_id == product.category_id)
            .map(|c| c.category_name.clone());
        let brand_name = self
            .repo()
            .brand_names(&[product.brand_id])
            .await?
            .into_iter()
            .next()
            .map(|(_, n)| n);

        Ok(FeedProductDetailVO {
            base: FeedProductVO {
                product_id: product.product_id,
                product_name: product.product_name.clone(),
                product_sn: product.product_sn.clone(),
                category_id: product.category_id,
                category_name: cat_name,
                shop_id: product.shop_id,
                shop_name: self
                    .repo()
                    .shop_names(&[product.shop_id])
                    .await?
                    .into_iter()
                    .next()
                    .map(|(_, n)| n),
                brand_id: product.brand_id,
                brand_name,
                pic_url: product.pic_url.clone(),
                price,
                market_price: market,
                stock: product.product_stock,
                sku_count: enabled.len() as i64,
                inquiry,
                content_hash,
                last_update: product.last_update,
            },
            product_brief: product.product_brief.clone(),
            product_desc: product.product_desc.clone(),
            keywords: product.keywords.clone(),
            product_weight: product.product_weight,
            product_unit: product.product_unit.clone(),
            free_shipping: product.free_shipping,
            product_type: product.product_type,
            images,
            skus,
            category_path: category_path(&all_categories, product.category_id),
        })
    }

    // ———————————————————— 内部 ————————————————————

    async fn load_key(&self, feed_key_id: i32) -> Result<FeedKey> {
        self.repo()
            .find_key_by_id(feed_key_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_KEY_NOT_FOUND.into()))
    }

    /// 密钥口径下的基础过滤（范围 + 询价开关）。
    async fn scope_filter(&self, key: &FeedKey) -> Result<FeedProductFilter> {
        let mut filter = FeedProductFilter {
            include_inquiry: key.include_inquiry == 1,
            ..Default::default()
        };
        match i32::from(key.scope_mode) {
            SCOPE_PRODUCTS => {
                filter.product_ids =
                    Some(self.repo().list_key_product_ids(key.feed_key_id).await?);
            }
            SCOPE_CATEGORIES => {
                let selected = self.repo().list_key_category_ids(key.feed_key_id).await?;
                let all = self.repo().list_all_categories().await?;
                let pairs: Vec<(i32, i32)> =
                    all.iter().map(|c| (c.category_id, c.parent_id)).collect();
                filter.category_ids =
                    Some(expand_categories(&selected, &pairs).into_iter().collect());
            }
            _ => {}
        }
        Ok(filter)
    }

    async fn count_visible(&self, key: &FeedKey) -> Result<i64> {
        let aud = self.audience_for_key(key).await?;
        self.count_for(&aud).await
    }

    async fn overrides_map(
        &self,
        feed_key_id: i32,
        rows: &[FeedProductRow],
    ) -> Result<HashMap<i32, FeedPriceOverride>> {
        let ids: Vec<i32> = rows.iter().map(|r| r.product_id).collect();
        let list = self
            .repo()
            .list_overrides_for_products(feed_key_id, &ids)
            .await?;
        Ok(list.into_iter().map(|o| (o.product_id, o)).collect())
    }

    /// 列表行 → VO（批量回填覆盖/SKU/相册/品牌/分类名，避免 N+1）。
    async fn assemble_list(
        &self,
        aud: &FeedAudience,
        rows: &[FeedProductRow],
        all_categories: &[Category],
    ) -> Result<Vec<FeedProductVO>> {
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<i32> = rows.iter().map(|r| r.product_id).collect();
        let skus = self.repo().find_skus_for_products(&ids).await?;
        let mut skus_by_product: HashMap<i32, Vec<&ProductSku>> = HashMap::new();
        for s in &skus {
            if s.is_enabled == 1 {
                skus_by_product.entry(s.product_id).or_default().push(s);
            }
        }
        let gallery = self.repo().find_gallery_for_products(&ids).await?;
        let mut images_by_product: HashMap<i32, Vec<String>> = HashMap::new();
        for g in &gallery {
            images_by_product
                .entry(g.product_id)
                .or_default()
                .push(g.pic_url.clone());
        }
        let brand_ids: Vec<i32> = rows.iter().map(|r| r.brand_id).collect();
        let brands: HashMap<i32, String> = self
            .repo()
            .brand_names(&brand_ids)
            .await?
            .into_iter()
            .collect();
        let cat_names: HashMap<i32, String> = all_categories
            .iter()
            .map(|c| (c.category_id, c.category_name.clone()))
            .collect();
        let shop_ids: Vec<i32> = rows.iter().map(|r| r.shop_id).collect();
        let shop_names: HashMap<i32, String> = self
            .repo()
            .shop_names(&shop_ids)
            .await?
            .into_iter()
            .collect();
        Ok(rows
            .iter()
            .map(|row| {
                let rule = aud.rule_for(row.product_id);
                let price = rule.apply(row.product_price);
                let inquiry = is_inquiry_product(row.is_inquiry, row.sku_inquiry_max);
                let product_skus = skus_by_product
                    .get(&row.product_id)
                    .cloned()
                    .unwrap_or_default();
                let sku_parts: Vec<(i32, String, Decimal, i32)> = product_skus
                    .iter()
                    .map(|s| {
                        (
                            s.sku_id,
                            s.sku_value.clone(),
                            rule.apply(s.sku_price.unwrap_or(row.product_price)),
                            s.sku_stock.unwrap_or(0),
                        )
                    })
                    .collect();
                let images = images_by_product
                    .get(&row.product_id)
                    .cloned()
                    .unwrap_or_else(|| vec![row.pic_url.clone()]);
                let content_hash = hash_of(&ContentHashParts {
                    product_name: row.product_name.clone(),
                    product_brief: row.product_brief.clone(),
                    product_desc: row.product_desc.clone().unwrap_or_default(),
                    category_id: row.category_id,
                    brand_id: row.brand_id,
                    price,
                    stock: row.product_stock,
                    inquiry,
                    images,
                    skus: sku_parts,
                });
                FeedProductVO {
                    product_id: row.product_id,
                    product_name: row.product_name.clone(),
                    product_sn: row.product_sn.clone(),
                    category_id: row.category_id,
                    category_name: cat_names.get(&row.category_id).cloned(),
                    shop_id: row.shop_id,
                    shop_name: shop_names.get(&row.shop_id).cloned(),
                    brand_id: row.brand_id,
                    brand_name: brands.get(&row.brand_id).cloned(),
                    pic_url: row.pic_url.clone(),
                    price,
                    market_price: market_price_out(row.market_price, price),
                    stock: row.product_stock,
                    sku_count: row.sku_count,
                    inquiry,
                    content_hash,
                    last_update: row.last_update,
                }
            })
            .collect())
    }
}

/// 受众 → 仓储过滤（集合展开为排序后的 IN 列表）。
fn audience_filter(aud: &FeedAudience) -> FeedProductFilter {
    let sorted = |set: &Option<std::collections::HashSet<i32>>| {
        set.as_ref().map(|s| {
            let mut v: Vec<i32> = s.iter().copied().collect();
            v.sort_unstable();
            v
        })
    };
    FeedProductFilter {
        product_ids: sorted(&aud.product_ids),
        category_ids: sorted(&aud.category_ids),
        shop_ids: sorted(&aud.shop_ids),
        include_inquiry: aud.include_inquiry,
        ..Default::default()
    }
}

/// 密钥 VO（永不含 secret/哈希）。
fn key_to_vo(key: &FeedKey, product_count: i64) -> FeedKeyVO {
    FeedKeyVO {
        feed_key_id: key.feed_key_id,
        key_name: key.key_name.clone(),
        access_key: key.access_key.clone(),
        status: i32::from(key.status),
        scope_mode: i32::from(key.scope_mode),
        include_inquiry: key.include_inquiry == 1,
        price_rule_type: key.price_rule_type.clone(),
        price_rule_value: key.price_rule_value,
        expire_time: key.expire_time,
        remark: key.remark.clone(),
        last_used_time: key.last_used_time,
        call_count: key.call_count,
        add_time: key.add_time,
        update_time: key.update_time,
        product_count,
    }
}

/// 生成 access_key：`fk_` + 20 位 hex。
fn gen_access_key() -> String {
    format!("fk_{}", utils::random_hex(20))
}

/// 生成 secret：48 位 hex（192 bit）。
fn gen_secret() -> String {
    utils::random_hex(48)
}

/// 内容哈希：规范化拼接串的 SHA-256（规划 §4.6）。
fn hash_of(parts: &ContentHashParts) -> String {
    utils::sha256_hex(content_hash_canonical(parts))
}

/// 恒定时间字符串比较（secret 哈希比对用，避免时序泄漏）。
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// 分类树构建：只保留子树内有授权商品的节点，count 为子树合计。
/// `categories` 已按 (sort_order, category_id) 排序，children 保持该序。
fn build_category_tree(
    categories: &[Category],
    counts: &HashMap<i32, i64>,
) -> Vec<FeedCategoryVO> {
    let id_set: HashSet<i32> = categories.iter().map(|c| c.category_id).collect();
    let mut children: HashMap<i32, Vec<i32>> = HashMap::new();
    let mut roots: Vec<i32> = Vec::new();
    for c in categories {
        if c.parent_id == 0 || !id_set.contains(&c.parent_id) {
            roots.push(c.category_id);
        } else {
            children.entry(c.parent_id).or_default().push(c.category_id);
        }
    }
    let by_id: HashMap<i32, &Category> =
        categories.iter().map(|c| (c.category_id, c)).collect();

    fn subtree_total(
        id: i32,
        children: &HashMap<i32, Vec<i32>>,
        counts: &HashMap<i32, i64>,
        memo: &mut HashMap<i32, i64>,
    ) -> i64 {
        if let Some(v) = memo.get(&id) {
            return *v;
        }
        let mut total = counts.get(&id).copied().unwrap_or(0);
        if let Some(kids) = children.get(&id) {
            for kid in kids {
                total += subtree_total(*kid, children, counts, memo);
            }
        }
        memo.insert(id, total);
        total
    }

    fn build(
        id: i32,
        by_id: &HashMap<i32, &Category>,
        children: &HashMap<i32, Vec<i32>>,
        counts: &HashMap<i32, i64>,
        memo: &mut HashMap<i32, i64>,
    ) -> Option<FeedCategoryVO> {
        let total = subtree_total(id, children, counts, memo);
        if total <= 0 {
            return None;
        }
        let c = by_id.get(&id)?;
        let kids = children
            .get(&id)
            .map(|list| {
                list.iter()
                    .filter_map(|kid| build(*kid, by_id, children, counts, memo))
                    .collect()
            })
            .unwrap_or_default();
        Some(FeedCategoryVO {
            category_id: c.category_id,
            category_name: c.category_name.clone(),
            parent_id: c.parent_id,
            sort_order: c.sort_order,
            category_pic: c.category_pic.clone(),
            product_count: total,
            children: kids,
        })
    }

    let mut memo: HashMap<i32, i64> = HashMap::new();
    roots
        .iter()
        .filter_map(|id| build(*id, &by_id, &children, counts, &mut memo))
        .collect()
}

/// 分类路径（根 → 叶），父链断裂/成环时安全截断。
fn category_path(categories: &[Category], category_id: i32) -> Vec<FeedCategoryPathVO> {
    let by_id: HashMap<i32, &Category> =
        categories.iter().map(|c| (c.category_id, c)).collect();
    let mut path = Vec::new();
    let mut cur = category_id;
    let mut seen = HashSet::new();
    while cur != 0 && seen.insert(cur) {
        let Some(c) = by_id.get(&cur) else { break };
        path.push(FeedCategoryPathVO {
            category_id: c.category_id,
            category_name: c.category_name.clone(),
        });
        cur = c.parent_id;
    }
    path.reverse();
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_cases() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn generated_credentials_shape() {
        let ak = gen_access_key();
        assert!(ak.starts_with("fk_"));
        assert_eq!(ak.len(), 23);
        let secret = gen_secret();
        assert_eq!(secret.len(), 48);
        assert!(secret.chars().all(|c| c.is_ascii_hexdigit()));
        // 两次生成不应相同（随机性冒烟）
        assert_ne!(gen_secret(), gen_secret());
    }

    #[test]
    fn category_tree_keeps_only_branches_with_products() {
        let mk = |id: i32, parent: i32, name: &str| Category {
            category_id: id,
            parent_id: parent,
            category_name: name.to_string(),
            sort_order: id,
            ..Default::default()
        };
        let cats = vec![mk(1, 0, "根"), mk(2, 1, "有货"), mk(3, 1, "无货"), mk(4, 2, "叶子")];
        let counts = HashMap::from([(4, 5i64)]);
        let tree = build_category_tree(&cats, &counts);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].category_id, 1);
        assert_eq!(tree[0].product_count, 5);
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].category_id, 2);
        assert_eq!(tree[0].children[0].children[0].category_id, 4);
    }

    #[test]
    fn category_path_root_to_leaf() {
        let mk = |id: i32, parent: i32, name: &str| Category {
            category_id: id,
            parent_id: parent,
            category_name: name.to_string(),
            ..Default::default()
        };
        let cats = vec![mk(1, 0, "根"), mk(2, 1, "中"), mk(3, 2, "叶")];
        let path = category_path(&cats, 3);
        assert_eq!(
            path.iter().map(|p| p.category_id).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        // 不存在的分类 → 空路径
        assert!(category_path(&cats, 99).is_empty());
    }
}
