//! 站点搬家服务（site_clone::clone）：搬家密钥管理、`/clone/v1` 鉴权与
//! 导出（manifest / catalog / products / settings）、拉取审计、导入前
//! 快照与一键回滚。导入执行在同模块 `importer`。
//!
//! 安全口径（设计报告第四/五节）：
//! - secret 高熵随机、只在生成/重置时明文返回一次，落库只存 SHA-256 哈希；
//! - 设置导出只走 `whitelist` 注册表（白名单 + 字段级过滤），未登记不导出；
//! - 每次带钥调用落 `site_clone_call_log` 审计，并受分钟级限流与
//!   总次数上限双重约束。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::{json, Map, Value};

use nextrade_common::utils;
use nextrade_core::models::catalog::product_manage::{
    GalleryPicDTO, ProductAttrGroupVO, ProductAttributeVO, ProductAttrsVO, ProductSaveDTO,
    ProductSkuSaveDTO, ProductVideoVO,
};
use nextrade_core::models::site_clone::clone::{
    CloneCallLogListQuery, CloneCatalogVO, CloneIdDTO, CloneJobListQuery,
    CloneKeyCreateDTO, CloneKeyListQuery, CloneKeySecretVO, CloneKeyUpdateDTO, CloneKeyVO,
    CloneManifestVO, CloneProduct, CloneSettingsBlock,
    CloneSettingsGroup, CloneSettingsGroupInfo, CloneSettingsPackage, CloneShippingTpl,
    CloneSourceCred, SiteCloneCallLog, SiteCloneImportJob, SiteCloneKey, SiteCloneSnapshot,
    ALL_DOMAINS, DOMAIN_ASSETS, DOMAIN_DECORATE, DOMAIN_SETTINGS, EXPORTER_VERSION,
    MSG_CLONE_CALLS_EXHAUSTED,
    MSG_CLONE_DISABLED, MSG_CLONE_EXPIRED, MSG_CLONE_RATE_LIMITED, MSG_CLONE_SCOPE_DENIED,
    MSG_CLONE_UNAUTHORIZED, MSG_KEY_NAME_REQUIRED, MSG_KEY_NOT_FOUND,
};
use nextrade_core::models::system::config_basic::find_endpoint;
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

use crate::catalog::product_manage::ProductManageService;
use crate::system::config_basic::ConfigBasicService;

use super::whitelist;

/// 每把密钥每分钟调用上限（分钟窗口内存计数，与联邦限流同法）。
const RATE_LIMIT_PER_MIN: u32 = 120;
/// 商品导出分页大小上限。
const PRODUCT_PAGE_MAX: u64 = 100;

/// 站点搬家服务。
pub struct SiteCloneService {
    pub(crate) repos: Arc<Repositories>,
    pub(crate) config_basic: Arc<ConfigBasicService>,
    pub(crate) product_manage: Arc<ProductManageService>,
    /// 密钥分钟级限流窗口：key_code → (窗口起始秒, 计数)。
    rate_windows: Mutex<HashMap<String, (i64, u32)>>,
}

impl SiteCloneService {
    #[must_use]
    pub fn new(
        repos: Arc<Repositories>,
        config_basic: Arc<ConfigBasicService>,
        product_manage: Arc<ProductManageService>,
    ) -> Self {
        Self { repos, config_basic, product_manage, rate_windows: Mutex::new(HashMap::new()) }
    }

    pub(crate) fn repo(&self) -> &nextrade_infrastructure::db::repo::site_clone::clone::SiteCloneRepository {
        &self.repos.site_clone.clone
    }

    pub(crate) fn catalog_repo(&self) -> &nextrade_infrastructure::db::repo::site_clone::catalog_data::SiteCloneCatalogRepository {
        &self.repos.site_clone.catalog
    }

    pub(crate) fn decorate_repo(&self) -> &nextrade_infrastructure::db::repo::site_clone::decorate_data::SiteCloneDecorateRepository {
        &self.repos.site_clone.decorate
    }

    pub(crate) fn config_repo(&self) -> &nextrade_infrastructure::db::repo::system::config_basic::ConfigBasicRepository {
        &self.repos.system.config_basic
    }

    // ———————————————————— 密钥管理（管理端） ————————————————————

    /// 生成搬家密钥：明文 secret 与整段口令只在本次返回出现一次。
    pub async fn create_key(
        &self,
        dto: &CloneKeyCreateDTO,
        operator_id: i32,
        operator_name: &str,
    ) -> Result<CloneKeySecretVO> {
        let name = dto.key_name.trim();
        if name.is_empty() {
            return Err(Error::biz(400, MSG_KEY_NAME_REQUIRED));
        }
        let domains = normalize_domains(&dto.domains);
        if domains.is_empty() {
            return Err(Error::biz(400, "至少勾选一个分享域（商品/设置/装修/素材）"));
        }
        let plain = utils::random_hex(48);
        let key_code = format!("mv_{}", utils::random_hex(16));
        let now = utils::now_seconds() as i32;
        let expire_time = match dto.expire_days.unwrap_or(0) {
            d if d > 0 => now + d * 86_400,
            _ => 0,
        };
        let key_id = self
            .repo()
            .insert_key(
                &key_code,
                name,
                &utils::sha256_hex(&plain),
                &domains.join(","),
                dto.source_shop_id.unwrap_or(0),
                expire_time,
                dto.max_calls.unwrap_or(0).max(0),
                operator_id,
                operator_name,
                now,
            )
            .await?;
        Ok(CloneKeySecretVO {
            key_id,
            key_code: key_code.clone(),
            secret: plain.clone(),
            token: build_token(dto.origin.as_deref().unwrap_or(""), &key_code, &plain),
        })
    }

    pub async fn list_keys(&self, q: &CloneKeyListQuery) -> Result<PageResult<CloneKeyVO>> {
        let page = q.page.unwrap_or(1).max(1);
        let size = q.page_size.unwrap_or(20).clamp(1, 100);
        let (rows, total) = self.repo().list_keys(page, size).await?;
        Ok(page_result(total, rows.iter().map(CloneKeyVO::from).collect(), page, size))
    }

    pub async fn update_key(&self, dto: &CloneKeyUpdateDTO) -> Result<()> {
        let key = self
            .repo()
            .find_key_by_id(dto.key_id)
            .await?
            .ok_or_else(|| Error::biz(404, MSG_KEY_NOT_FOUND))?;
        if key.status == -1 {
            return Err(Error::biz(400, "密钥已吊销，不能编辑（可重置口令后重新启用）"));
        }
        let name = dto.key_name.as_deref().map(str::trim).filter(|s| !s.is_empty())
            .map_or(key.key_name.clone(), str::to_string);
        let scopes = dto.domains.as_ref().map(|d| normalize_domains(d).join(","))
            .filter(|s| !s.is_empty())
            .unwrap_or(key.scopes.clone());
        let status = dto.status.filter(|s| *s == 0 || *s == 1).unwrap_or(key.status);
        let now = utils::now_seconds() as i32;
        self.repo()
            .update_key(dto.key_id, &name, &scopes, dto.source_shop_id.unwrap_or(key.source_shop_id), status, now)
            .await
    }

    /// 吊销密钥：即刻失效（status = -1），不可恢复（重置走 reset 并重新启用）。
    pub async fn revoke_key(&self, key_id: i32) -> Result<()> {
        let key = self
            .repo()
            .find_key_by_id(key_id)
            .await?
            .ok_or_else(|| Error::biz(404, MSG_KEY_NOT_FOUND))?;
        let _ = key;
        let now = utils::now_seconds() as i32;
        self.repo().set_key_status(key_id, -1, now).await
    }

    /// 重置 secret：旧 secret 立即作废；吊销状态一并恢复为启用
    /// （重置即重新签发，页面文案明示）。
    pub async fn reset_secret(&self, dto: &CloneIdDTO) -> Result<CloneKeySecretVO> {
        let key_id = dto.target().ok_or_else(|| Error::biz(400, "缺少密钥 id"))?;
        let key = self
            .repo()
            .find_key_by_id(key_id)
            .await?
            .ok_or_else(|| Error::biz(404, MSG_KEY_NOT_FOUND))?;
        let plain = utils::random_hex(48);
        let now = utils::now_seconds() as i32;
        self.repo().update_secret_hash(key_id, &utils::sha256_hex(&plain), now).await?;
        if key.status != 1 {
            self.repo().set_key_status(key_id, 1, now).await?;
        }
        Ok(CloneKeySecretVO {
            key_id,
            key_code: key.key_code.clone(),
            secret: plain.clone(),
            token: build_token(dto.origin.as_deref().unwrap_or(""), &key.key_code, &plain),
        })
    }

    pub async fn list_call_logs(
        &self,
        q: &CloneCallLogListQuery,
    ) -> Result<PageResult<SiteCloneCallLog>> {
        let page = q.page.unwrap_or(1).max(1);
        let size = q.page_size.unwrap_or(20).clamp(1, 100);
        let (rows, total) = self.repo().list_call_logs(q.key_id, page, size).await?;
        Ok(page_result(total, rows, page, size))
    }

    // ———————————————————— 鉴权与审计（公开面） ————————————————————

    /// `/clone/v1` 鉴权：哈希比对 + 状态/有效期/域权限/限流/次数上限。
    /// 成功后计数 +1（用量口径与联邦一致：鉴权通过即计一次调用）。
    pub async fn authenticate(
        &self,
        key_code: &str,
        secret: &str,
        domain: &str,
    ) -> Result<SiteCloneKey> {
        let key = self
            .repo()
            .find_key_by_code(key_code.trim())
            .await?
            .ok_or_else(|| Error::Unauthorized(MSG_CLONE_UNAUTHORIZED.into()))?;
        let provided = utils::sha256_hex(secret.trim());
        if !constant_time_eq(provided.as_bytes(), key.secret_hash.as_bytes()) {
            return Err(Error::Unauthorized(MSG_CLONE_UNAUTHORIZED.into()));
        }
        if key.status != 1 {
            return Err(Error::Forbidden(MSG_CLONE_DISABLED.into()));
        }
        let now = utils::now_seconds() as i32;
        if key.expire_time > 0 && key.expire_time < now {
            return Err(Error::Forbidden(MSG_CLONE_EXPIRED.into()));
        }
        if !domain.is_empty() && !key.has_domain(domain) {
            return Err(Error::Forbidden(MSG_CLONE_SCOPE_DENIED.into()));
        }
        if key.max_calls > 0 && key.used_calls >= key.max_calls {
            return Err(Error::Forbidden(MSG_CLONE_CALLS_EXHAUSTED.into()));
        }
        self.check_rate(&key.key_code)?;
        self.repo().touch_key_usage(key.key_id, now).await?;
        Ok(key)
    }

    fn check_rate(&self, key_code: &str) -> Result<()> {
        let now = utils::now_seconds();
        let mut windows = self.rate_windows.lock().unwrap_or_else(|p| p.into_inner());
        let entry = windows.entry(key_code.to_string()).or_insert((now, 0));
        if now - entry.0 >= 60 {
            *entry = (now, 0);
        }
        entry.1 += 1;
        if entry.1 > RATE_LIMIT_PER_MIN {
            return Err(Error::Forbidden(MSG_CLONE_RATE_LIMITED.into()));
        }
        Ok(())
    }

    /// 拉取审计（失败不影响主流程，与 hub record_call 同法）。
    pub async fn record_call(
        &self,
        key: &SiteCloneKey,
        endpoint: &str,
        domain: &str,
        http_status: i32,
        result_count: i32,
        client_ip: &str,
        started: std::time::Instant,
    ) {
        let duration_ms = i32::try_from(started.elapsed().as_millis()).unwrap_or(i32::MAX);
        let now = utils::now_seconds() as i32;
        if let Err(e) = self
            .repo()
            .insert_call_log(key.key_id, &key.key_code, endpoint, domain, http_status, result_count, client_ip, duration_ms, now)
            .await
        {
            tracing::warn!("站点搬家审计写入失败: {e}");
        }
    }

    // ———————————————————— 导出（公开面） ————————————————————

    pub async fn manifest(&self, key: &SiteCloneKey) -> Result<CloneManifestVO> {
        let site_name = self
            .config_repo()
            .find_by_code("shopName")
            .await?
            .unwrap_or_default();
        let shop = key.source_shop_id;
        let deco = self.decorate_repo();
        let (decorate_total, discrete_total, nav_total) = if key.has_domain(DOMAIN_DECORATE) {
            (
                deco.count_decorate_pages(shop).await?,
                deco.count_discrete(shop).await?,
                deco.count_nav_total().await?,
            )
        } else {
            (0, 0, 0)
        };
        let (asset_pic_total, asset_video_total) = if key.has_domain(DOMAIN_ASSETS) {
            (deco.count_asset_pics(shop).await?, deco.count_asset_videos(shop).await?)
        } else {
            (0, 0)
        };
        Ok(CloneManifestVO {
            site_name,
            exporter_version: EXPORTER_VERSION,
            key_name: key.key_name.clone(),
            domains: key.domain_list(),
            source_shop_id: shop,
            product_total: self.catalog_repo().count_products(shop).await?,
            category_total: self.catalog_repo().count_categories().await?,
            brand_total: self.catalog_repo().count_brands(shop).await?,
            shipping_tpl_total: self.catalog_repo().count_shipping_tpls(shop).await?,
            settings_group_total: if key.has_domain(DOMAIN_SETTINGS) {
                i32::try_from(whitelist::GROUP_RULES.len()).unwrap_or(0)
            } else {
                0
            },
            decorate_total,
            discrete_total,
            nav_total,
            asset_pic_total,
            asset_video_total,
            exempt_note: "支付渠道、各类 API 密钥与凭据、站点身份、会员/订单/财务数据不在分享范围内（白名单导出）".into(),
        })
    }

    /// 装修包导出（P2）：源店铺全部装修页/离散组件 + 平台级导航三表。
    pub async fn decorate_package(
        &self,
        key: &SiteCloneKey,
    ) -> Result<nextrade_core::models::site_clone::clone::CloneDecoratePackage> {
        let shop = key.source_shop_id;
        let repo = self.decorate_repo();
        Ok(nextrade_core::models::site_clone::clone::CloneDecoratePackage {
            pages: repo.list_decorate_pages(shop).await?,
            discrete: repo.list_discrete(shop).await?,
            navigations: repo.list_navigations().await?,
            cat_floors: repo.list_cat_floors().await?,
            mobile_cat_navs: repo.list_mobile_cat_navs().await?,
        })
    }

    /// 素材包导出（P2）：源店铺 + 平台（shop_id=0）的素材分组与条目。
    /// 只给源 URL 清单，转存由导入端服务端下载（OSS 公开读，不需凭证）。
    pub async fn assets_package(
        &self,
        key: &SiteCloneKey,
    ) -> Result<nextrade_core::models::site_clone::clone::CloneAssetsPackage> {
        let shop = key.source_shop_id;
        let repo = self.decorate_repo();
        Ok(nextrade_core::models::site_clone::clone::CloneAssetsPackage {
            groups: repo.list_asset_groups(shop).await?,
            pics: repo.list_asset_pics(shop).await?,
            video_groups: repo.list_asset_video_groups(shop).await?,
            videos: repo.list_asset_videos(shop).await?,
        })
    }

    pub async fn catalog(&self, key: &SiteCloneKey) -> Result<CloneCatalogVO> {
        let shop = key.source_shop_id;
        let tpl_rows = self.catalog_repo().list_shipping_tpl_rows(shop).await?;
        let mut tpls = Vec::with_capacity(tpl_rows.len());
        for row in tpl_rows {
            let infos = self.catalog_repo().list_tpl_infos(row.shipping_tpl_id).await?;
            tpls.push(CloneShippingTpl {
                shipping_tpl_id: row.shipping_tpl_id,
                shipping_tpl_name: row.shipping_tpl_name,
                shipping_time: row.shipping_time,
                is_free: row.is_free,
                pricing_type: row.pricing_type,
                is_default: row.is_default,
                shop_id: row.shop_id,
                infos,
            });
        }
        Ok(CloneCatalogVO {
            categories: self.catalog_repo().list_categories().await?,
            brands: self.catalog_repo().list_brands(shop).await?,
            shipping_types: self.catalog_repo().list_shipping_types(shop).await?,
            shipping_tpls: tpls,
        })
    }

    pub async fn products_page(
        &self,
        key: &SiteCloneKey,
        page: u64,
        size: u64,
    ) -> Result<PageResult<CloneProduct>> {
        let page = page.max(1);
        let size = size.clamp(1, PRODUCT_PAGE_MAX);
        let total = self.catalog_repo().count_products(key.source_shop_id).await?;
        let ids = self
            .catalog_repo()
            .list_product_ids(key.source_shop_id, ((page - 1) * size) as i64, size as i64)
            .await?;
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(p) = self.assemble_product(id).await? {
                out.push(p);
            }
        }
        Ok(page_result(total, out, page, size))
    }

    /// 组装单品全保真包（子表 + 分类路径/品牌名/模板名 + 服务承诺）。
    pub async fn assemble_product(&self, product_id: i32) -> Result<Option<CloneProduct>> {
        let Some(row) = self.catalog_repo().load_product_row(product_id).await? else {
            return Ok(None);
        };
        let repo = self.catalog_repo();
        let category_path = self.category_path(row.category_id).await?;
        let brand_name = repo
            .find_brand_name(row.brand_id)
            .await?
            .unwrap_or_default();
        let shipping_tpl_name = repo
            .find_shipping_tpl_name(row.shipping_tpl_id)
            .await?
            .unwrap_or_default();
        let service_ids = parse_id_list(row.product_service_ids.as_deref().unwrap_or(""));
        let services = repo.load_services_by_ids(&service_ids).await?;
        Ok(Some(CloneProduct {
            product_id: row.product_id,
            product_name: row.product_name,
            product_sn: row.product_sn,
            product_tsn: row.product_tsn,
            seo_identification: row.seo_identification,
            product_type: row.product_type,
            sale_mode: row.sale_mode,
            delivery_mode: row.delivery_mode,
            product_status: row.product_status,
            product_price: row.product_price,
            market_price: row.market_price,
            cost_price: row.cost_price,
            product_stock: row.product_stock,
            product_weight: row.product_weight,
            product_volume: row.product_volume,
            category_id: row.category_id,
            category_path,
            brand_id: row.brand_id,
            brand_name,
            shipping_tpl_id: row.shipping_tpl_id,
            shipping_tpl_name,
            free_shipping: row.free_shipping,
            fixed_shipping_type: row.fixed_shipping_type,
            fixed_shipping_fee: row.fixed_shipping_fee,
            no_shipping: row.no_shipping,
            keywords: row.keywords,
            product_brief: row.product_brief,
            product_desc: row.product_desc,
            pic_url: row.pic_url,
            pic_thumb: row.pic_thumb,
            pic_original: row.pic_original,
            pic_large: row.pic_large,
            is_best: row.is_best,
            is_new: row.is_new,
            is_hot: row.is_hot,
            is_promote: row.is_promote,
            promote_price: row.promote_price,
            sort_order: row.sort_order,
            store_sort_order: row.store_sort_order,
            remark: row.remark,
            give_integral: row.give_integral,
            rank_integral: row.rank_integral,
            integral: row.integral,
            virtual_sales: row.virtual_sales,
            limit_number: row.limit_number,
            min_buy_count: row.min_buy_count,
            product_care: row.product_care,
            is_support_return: row.is_support_return,
            is_support_cod: row.is_support_cod,
            product_video: row.product_video,
            prepay_price: row.prepay_price,
            virtual_sample: row.virtual_sample,
            paid_content: row.paid_content,
            product_unit: row.product_unit,
            is_inquiry: row.is_inquiry,
            is_shop_pickup: row.is_shop_pickup,
            is_logistics: row.is_logistics,
            is_shop_delivery: row.is_shop_delivery,
            delivery_service_fee_type: row.delivery_service_fee_type,
            delivery_service_fee_value: row.delivery_service_fee_value,
            server_service_fee_type: row.server_service_fee_type,
            server_service_fee_value: row.server_service_fee_value,
            services,
            skus: repo.load_skus(product_id).await?,
            attributes: repo.load_attributes(product_id).await?,
            gallery: repo.load_gallery(product_id).await?,
            videos: repo.load_videos(product_id).await?,
        }))
    }

    /// 分类名称路径（`一级/二级/三级`），向上回溯至多 10 层防环。
    pub(crate) async fn category_path(&self, category_id: i32) -> Result<String> {
        if category_id <= 0 {
            return Ok(String::new());
        }
        let mut names = Vec::new();
        let mut cur = category_id;
        for _ in 0..10 {
            match self.catalog_repo().find_category_name(cur).await? {
                Some((name, parent)) => {
                    names.push(name);
                    if parent <= 0 {
                        break;
                    }
                    cur = parent;
                }
                None => break,
            }
        }
        names.reverse();
        Ok(names.join("/"))
    }

    /// 设置包导出：仅白名单组（经规格表 GET 组装后字段级过滤）+ 白名单块。
    pub async fn settings_package(&self) -> Result<CloneSettingsPackage> {
        let mut groups = Vec::with_capacity(whitelist::GROUP_RULES.len());
        for rule in whitelist::GROUP_RULES {
            let spec = find_endpoint(rule.get_path)
                .ok_or_else(|| Error::Internal(format!("设置规格缺失: {}", rule.get_path)))?;
            let raw = self.config_basic.get(spec, None).await?;
            groups.push(CloneSettingsGroup {
                key: rule.key.to_string(),
                data: whitelist::filter_group_value(&raw, rule.deny_fields),
            });
        }
        let mut blocks = Vec::new();
        for code in whitelist::JSON_BLOCK_CODES {
            if let Some(raw) = self.config_repo().find_by_code(code).await? {
                let value = serde_json::from_str::<Value>(&raw).unwrap_or(Value::String(raw));
                blocks.push(CloneSettingsBlock { code: (*code).to_string(), value });
            }
        }
        for code in whitelist::RAW_BLOCK_CODES {
            if let Some(raw) = self.config_repo().find_by_code(code).await? {
                blocks.push(CloneSettingsBlock { code: (*code).to_string(), value: Value::String(raw) });
            }
        }
        Ok(CloneSettingsPackage { groups, blocks })
    }

    // ———————————————————— 快照与回滚（管理端） ————————————————————

    pub async fn list_snapshots(
        &self,
        q: &CloneJobListQuery,
    ) -> Result<PageResult<SiteCloneSnapshot>> {
        let page = q.page.unwrap_or(1).max(1);
        let size = q.page_size.unwrap_or(20).clamp(1, 100);
        let (rows, total) = self.repo().list_snapshots(page, size).await?;
        Ok(page_result(total, rows, page, size))
    }

    /// 一键回滚：按快照原值整包还原设置（组走规格表保存语义，块走原始行
    /// 写回；导入前不存在的块删除该行）。回滚本身不再产生新快照。
    pub async fn rollback_snapshot(&self, snapshot_id: i32) -> Result<()> {
        let snap = self
            .repo()
            .find_snapshot(snapshot_id)
            .await?
            .ok_or_else(|| Error::biz(404, "快照不存在"))?;
        if snap.status == 2 {
            return Err(Error::biz(400, "该快照已回滚过"));
        }
        let payload: Value = serde_json::from_str(&snap.payload)
            .map_err(|e| Error::Internal(format!("快照数据损坏: {e}")))?;
        if let Some(groups) = payload.get("groups").and_then(Value::as_object) {
            for (key, data) in groups {
                let Some(rule) = whitelist::group_rule(key) else { continue };
                let Some(spec) = find_endpoint(rule.save_path) else { continue };
                let body: Map<String, Value> = match data {
                    Value::Object(m) => m.clone(),
                    _ => Map::new(),
                };
                self.config_basic.save(spec, &body).await?;
            }
        }
        if let Some(blocks) = payload.get("blocks").and_then(Value::as_object) {
            for (code, raw) in blocks {
                if !whitelist::is_block_code(code) {
                    continue;
                }
                match raw {
                    Value::Null => self.repo().delete_config_code(code).await?,
                    Value::String(s) => self.config_repo().save_code(code, s).await?,
                    _ => {}
                }
            }
        }
        let now = utils::now_seconds() as i32;
        self.repo().mark_snapshot_rolled_back(snapshot_id, now).await
    }

    pub async fn list_jobs(&self, q: &CloneJobListQuery) -> Result<PageResult<SiteCloneImportJob>> {
        let page = q.page.unwrap_or(1).max(1);
        let size = q.page_size.unwrap_or(20).clamp(1, 100);
        let (rows, total) = self.repo().list_jobs(page, size).await?;
        Ok(page_result(total, rows, page, size))
    }

    pub async fn job_detail(&self, job_id: i32) -> Result<SiteCloneImportJob> {
        self.repo()
            .find_job(job_id)
            .await?
            .ok_or_else(|| Error::biz(404, "导入任务不存在"))
    }
}

// ———————————————————— 自由函数 ————————————————————

/// 归一化授权域：只保留已开放域（P1 商品/设置 + P2 装修/素材）、去重保序。
pub(crate) fn normalize_domains(input: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for d in input {
        let d = d.trim();
        if ALL_DOMAINS.contains(&d) && !out.iter().any(|x| x == d) {
            out.push(d.to_string());
        }
    }
    out
}

/// 拼搬家口令：`MV1.` + base64url(JSON{origin,keyId,secret})。
/// origin 为空时不生成口令（前端改走三栏手动填）。
pub(crate) fn build_token(origin: &str, key_code: &str, secret: &str) -> String {
    use base64::Engine;
    let origin = origin.trim().trim_end_matches('/');
    if origin.is_empty() {
        return String::new();
    }
    let payload = json!({ "v": 1, "origin": origin, "keyId": key_code, "secret": secret });
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string());
    format!("MV1.{encoded}")
}

/// 解析搬家口令（与手动三栏共用归一化出口）。
pub(crate) fn parse_token(token: &str) -> Result<(String, String, String)> {
    use base64::Engine;
    let raw = token.trim();
    let Some(b64) = raw.strip_prefix("MV1.") else {
        return Err(Error::biz(400, "搬家口令格式不正确（应以 MV1. 开头）"));
    };
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(b64.trim())
        .map_err(|_| Error::biz(400, "搬家口令无法解码"))?;
    let v: Value = serde_json::from_slice(&bytes)
        .map_err(|_| Error::biz(400, "搬家口令内容损坏"))?;
    let origin = v.get("origin").and_then(Value::as_str).unwrap_or("").trim().trim_end_matches('/').to_string();
    let key_id = v.get("keyId").and_then(Value::as_str).unwrap_or("").trim().to_string();
    let secret = v.get("secret").and_then(Value::as_str).unwrap_or("").trim().to_string();
    if origin.is_empty() || key_id.is_empty() || secret.is_empty() {
        return Err(Error::biz(400, "搬家口令缺少来源地址/密钥信息"));
    }
    Ok((origin, key_id, secret))
}

/// 来源凭据归一化：口令优先，否则三栏手动填。
pub(crate) fn resolve_source(
    source: &nextrade_core::models::site_clone::clone::CloneSourceDTO,
) -> Result<CloneSourceCred> {
    let (origin, key_code, secret) = if let Some(token) = source.token.as_deref().filter(|s| !s.trim().is_empty()) {
        parse_token(token)?
    } else {
        let origin = source.origin.as_deref().unwrap_or("").trim().trim_end_matches('/').to_string();
        let key_code = source.key_id.as_deref().unwrap_or("").trim().to_string();
        let secret = source.secret.trim().to_string();
        if origin.is_empty() || key_code.is_empty() || secret.is_empty() {
            return Err(Error::biz(400, "请粘贴搬家口令，或完整填写来源地址/密钥 ID/密钥"));
        }
        (origin, key_code, secret)
    };
    let fingerprint = utils::sha256_hex(format!("{key_code}:{secret}"));
    Ok(CloneSourceCred { origin, key_code, secret, fingerprint })
}

/// 常量时间字符串比较（防哈希比对时序侧信道，与联邦同要求）。
pub(crate) fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// 宽容解析 id 列表存储串（`[1,2]` / `1,2` / `1` 均可）。
pub(crate) fn parse_id_list(raw: &str) -> Vec<i32> {
    raw.chars()
        .filter(|c| c.is_ascii_digit() || *c == ',' || *c == '-')
        .collect::<String>()
        .split(',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .filter(|id| *id > 0)
        .collect()
}

/// 统一分页结果组装（PageResult 字段口径）。
pub(crate) fn page_result<T>(total: i64, list: Vec<T>, page: u64, size: u64) -> PageResult<T> {
    let total_u = u64::try_from(total.max(0)).unwrap_or(0);
    PageResult {
        total: total_u,
        total_page: total_u.div_ceil(size.max(1)),
        list,
        page_size: size,
        page,
    }
}

/// 预览/执行共用：设置组信息（key/名/风险/字段数）。
pub(crate) fn settings_group_infos(pkg: &CloneSettingsPackage) -> Vec<CloneSettingsGroupInfo> {
    pkg.groups
        .iter()
        .filter_map(|g| {
            let rule = whitelist::group_rule(&g.key)?;
            let field_count = g.data.as_object().map_or(0, |m| i32::try_from(m.len()).unwrap_or(0));
            Some(CloneSettingsGroupInfo {
                key: g.key.clone(),
                label: rule.label.to_string(),
                risk: rule.risk,
                field_count,
            })
        })
        .collect()
}

/// 把导出的商品组装成 `ProductSaveDTO`（导入映射在 importer 中补齐
/// category/brand/shipping/service 的本站 id 后调用 create）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_save_dto(
    p: &CloneProduct,
    target_shop_id: i32,
    category_id: i32,
    brand_id: i32,
    shipping_tpl_id: i64,
    service_ids: Vec<i32>,
    keep_status: bool,
) -> ProductSaveDTO {
    let img_list: Vec<GalleryPicDTO> = if p.gallery.is_empty() {
        if p.pic_url.is_empty() {
            Vec::new()
        } else {
            vec![GalleryPicDTO {
                pic_id: None,
                pic_url: Some(p.pic_url.clone()),
                pic_name: None,
                pic_thumb: Some(p.pic_thumb.clone()),
                pic_sort: Some(0),
            }]
        }
    } else {
        p.gallery
            .iter()
            .map(|g| GalleryPicDTO {
                pic_id: None,
                pic_url: Some(g.pic_url.clone()),
                pic_name: Some(g.pic_desc.clone()),
                pic_thumb: Some(g.pic_thumb.clone()),
                pic_sort: Some(g.sort_order),
            })
            .collect()
    };
    let product_list: Option<Vec<ProductSkuSaveDTO>> = if p.skus.is_empty() {
        None
    } else {
        Some(
            p.skus
                .iter()
                .map(|s| ProductSkuSaveDTO {
                    sku_id: None,
                    product_id: None,
                    sku_value: Some(s.sku_value.clone()),
                    sku_data: s.sku_data.as_deref().and_then(|raw| serde_json::from_str::<Value>(raw).ok()),
                    sku_sn: s.sku_sn.clone(),
                    sku_stock: s.sku_stock,
                    sku_tsn: s.sku_tsn.clone(),
                    sku_weight: s.sku_weight,
                    sku_volume: s.sku_volume,
                    sku_price: s.sku_price,
                    is_inquiry: Some(s.is_inquiry),
                    is_default: Some(s.is_default),
                    is_enabled: Some(s.is_enabled),
                    market_price: s.market_price,
                    cost_price: s.cost_price,
                    delivery_service_fee_type: Some(s.delivery_service_fee_type),
                    delivery_service_fee_value: Some(s.delivery_service_fee_value),
                    server_service_fee_type: Some(s.server_service_fee_type),
                    server_service_fee_value: Some(s.server_service_fee_value),
                    vendor_product_sku_id: None,
                    attr_names: None,
                    attr_values: None,
                    attrs: None,
                })
                .collect(),
        )
    };
    let sku_type = match p.skus.len() {
        0 => None,
        1 => Some(1),
        _ => Some(2),
    };
    let mut normal: Vec<ProductAttrGroupVO> = Vec::new();
    let mut spe: Vec<ProductAttrGroupVO> = Vec::new();
    let mut extra: Vec<ProductAttrGroupVO> = Vec::new();
    for a in &p.attributes {
        let bucket = match a.attr_type {
            1 => &mut spe,
            2 => &mut extra,
            _ => &mut normal,
        };
        let vo = ProductAttributeVO {
            attributes_id: None,
            product_id: None,
            attr_type: a.attr_type,
            attr_name: a.attr_name.clone(),
            attr_value: a.attr_value.clone(),
            attr_price: a.attr_price,
            attr_color: a.attr_color.clone().filter(|s| !s.is_empty()),
            attr_pic: a.attr_pic.clone().filter(|s| !s.is_empty()),
            attr_pic_thumb: a.attr_pic_thumb.clone().filter(|s| !s.is_empty()),
            sort_order: Some(a.sort_order),
        };
        if let Some(group) = bucket.iter_mut().find(|g| g.attr_name == a.attr_name) {
            group.attr_list.push(vo);
        } else {
            bucket.push(ProductAttrGroupVO { attr_name: a.attr_name.clone(), attr_list: vec![vo] });
        }
    }
    ProductSaveDTO {
        product_id: None,
        product_type: p.product_type,
        sale_mode: Some(p.sale_mode),
        delivery_mode: Some(p.delivery_mode),
        product_stock: Some(p.product_stock),
        free_shipping: Some(p.free_shipping),
        shop_category_id: None,
        product_status: Some(if keep_status { p.product_status } else { 0 }),
        give_integral: Some(p.give_integral),
        rank_integral: Some(p.rank_integral),
        integral: Some(p.integral),
        product_weight: Some(p.product_weight),
        product_volume: p.product_volume,
        product_desc_arr: None,
        product_related: None,
        product_service_ids: Some(service_ids),
        product_name: Some(p.product_name.clone()),
        seo_identification: p.seo_identification.clone(),
        product_sn: if p.product_sn.is_empty() { None } else { Some(p.product_sn.clone()) },
        product_tsn: p.product_tsn.clone(),
        product_price: Some(p.product_price),
        market_price: Some(p.market_price),
        cost_price: p.cost_price,
        is_inquiry: Some(p.is_inquiry),
        shipping_tpl_id: Some(shipping_tpl_id),
        category_id: Some(category_id),
        brand_id: Some(brand_id),
        shop_id: Some(target_shop_id),
        keywords: p.keywords.clone(),
        check_status: Some(1),
        check_reason: None,
        click_count: None,
        is_promote: Some(p.is_promote != 0),
        is_promote_activity: None,
        promote_price: Some(p.promote_price),
        promote_start_date: None,
        promote_end_date: None,
        seckill_max_num: None,
        product_brief: Some(p.product_brief.clone()),
        product_desc: p.product_desc.clone(),
        pic_url: Some(p.pic_url.clone()),
        pic_thumb: Some(p.pic_thumb.clone()),
        pic_original: Some(p.pic_original.clone()),
        comment_tag: None,
        add_time: None,
        sort_order: Some(p.sort_order),
        store_sort_order: Some(p.store_sort_order),
        is_delete: None,
        is_best: Some(p.is_best),
        is_new: Some(p.is_new),
        is_hot: Some(p.is_hot),
        fixed_shipping_type: p.fixed_shipping_type,
        fixed_shipping_fee: p.fixed_shipping_fee,
        remark: Some(p.remark.clone()),
        suppliers_id: None,
        virtual_sales: Some(p.virtual_sales),
        limit_number: Some(p.limit_number),
        min_buy_count: Some(p.min_buy_count),
        product_care: p.product_care.clone(),
        is_support_return: Some(p.is_support_return),
        is_support_cod: Some(p.is_support_cod),
        product_video: p.product_video.clone(),
        prepay_price: p.prepay_price,
        card_group_id: None,
        virtual_sample: p.virtual_sample.clone(),
        paid_content: p.paid_content.as_deref().and_then(|raw| serde_json::from_str::<Value>(raw).ok()),
        no_shipping: p.no_shipping,
        img_list: Some(img_list),
        product_video_info: Some(
            p.videos
                .iter()
                .map(|v| ProductVideoVO {
                    id: None,
                    video_id: Some(v.video_id),
                    product_id: None,
                    video_url: Some(v.video_url.clone()),
                    video_cover: Some(v.video_cover.clone()),
                    format: Some(v.format.clone()),
                })
                .collect(),
        ),
        product_article_list: None,
        attr_list: Some(ProductAttrsVO { normal, spe, extra }),
        product_list,
        sku_type,
        is_shop_pickup: Some(p.is_shop_pickup),
        is_logistics: Some(p.is_logistics),
        is_shop_delivery: Some(p.is_shop_delivery),
        shop_pickup_tpl_id: None,
        reservation_tpl_id: None,
        gift_card_groups_id: None,
        product_unit: p.product_unit.clone(),
        attributes_tpl_id: None,
        delivery_service_fee_type: Some(p.delivery_service_fee_type),
        delivery_service_fee_value: Some(p.delivery_service_fee_value),
        server_service_fee_type: Some(p.server_service_fee_type),
        server_service_fee_value: Some(p.server_service_fee_value),
        auto_online_time: None,
        auto_offline_time: None,
        sale_channel: None,
        is_weighing: None,
        weigh_unit_grams: None,
    }
}

