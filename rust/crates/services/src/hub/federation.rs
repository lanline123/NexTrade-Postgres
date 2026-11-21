//! 平台联邦 P1 服务（设计 `~/workspace/hub-federation-design/proposal.md`）。
//!
//! 来源标注：**我方新增**——原版 tigshop 无实例联邦概念。本服务实现：
//! 开关体系（§2，config 表 `federationSettings` 单行 JSON，口径同 systemMode）、
//! 通用凭证（§4，新表族 hub_credential*，不改 feed 旧表；商品范围/价格规则
//! 复用 product_feed 的 FeedAudience 纯口径）、三档连接与配对（§3）、
//! 商品上行聚合（§5，拉取为主）、心跳与注册表（§8）。
//! 会员/订单/结算 P1 只落开关与结构，功能分别在 P2/P3。
//!
//! 安全口径：入站钥 secret 只存 SHA-256 哈希、明文仅签发时回一次；
//! 出站钥 secret 以 AES-GCM 加密落库（主密钥 env `NEXTTRADE_FED_SECRET_KEY`，
//! 32 字节 hex 或 base64，永不入库/日志）；配对码与 claim token 只存哈希。

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use rust_decimal::Decimal;
use serde_json::{json, Value};

use nextrade_common::utils;
use nextrade_core::models::catalog::product_feed::{
    expand_categories, validate_rule, FeedAudience, FeedProductListQuery, PriceRule,
    SCOPE_CATEGORIES, SCOPE_PRODUCTS,
};
use nextrade_core::models::hub::federation::*;
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

use crate::catalog::product_feed::ProductFeedService;

/// 本实例对外版本号（meta / 心跳上报用）。
const INSTANCE_VERSION: &str = env!("CARGO_PKG_VERSION");
/// 联邦 API 版本标识。
const API_VERSION: &str = "hub/v1";
/// 聚合拉取单页大小与页数上限（防对端异常数据打爆内存）。
const SYNC_PAGE_SIZE: u64 = 100;
const SYNC_MAX_PAGES: u64 = 50;
/// 聚合时拉详情快照的单次上限。
const SYNC_MAX_DETAILS: usize = 500;

/// 平台联邦服务。
pub struct FederationService {
    repos: Arc<Repositories>,
    feed: Arc<ProductFeedService>,
    /// 凭证分钟级限流窗口：credential_id → (窗口起始秒, 计数)。
    rate_windows: Mutex<HashMap<i32, (i64, u32)>>,
    /// 凭证用量节流回写：credential_id → 上次回写秒。
    last_touch: Mutex<HashMap<i32, i32>>,
}

impl FederationService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>, feed: Arc<ProductFeedService>) -> Self {
        Self {
            repos,
            feed,
            rate_windows: Mutex::new(HashMap::new()),
            last_touch: Mutex::new(HashMap::new()),
        }
    }

    fn repo(&self) -> &nextrade_infrastructure::db::repo::hub::federation::HubFederationRepository {
        &self.repos.hub.federation
    }

    fn now(&self) -> i32 {
        i32::try_from(utils::now_seconds()).unwrap_or(i32::MAX)
    }

    // ———————————————————— 开关体系（设计 §2） ————————————————————

    /// 读取生效中的联邦设置（未保存过 ⇒ 全关默认）。
    pub async fn get_settings(&self) -> Result<FederationSettings> {
        let stored = self
            .repos
            .system
            .config_basic
            .find_by_code(FEDERATION_SETTINGS_CODE)
            .await?;
        Ok(effective_federation_settings(stored.as_deref()))
    }

    /// 保存联邦设置（清洗后整块 JSON 落 config 表；首次生成实例 UUID 并持久化）。
    pub async fn save_settings(&self, payload: &Value) -> Result<FederationSettings> {
        let current = self.get_settings().await?;
        let mut next = sanitize_federation_settings(payload);
        if next.instance_uuid.is_empty() {
            next.instance_uuid = if current.instance_uuid.is_empty() {
                uuid::Uuid::new_v4().to_string()
            } else {
                current.instance_uuid.clone()
            };
        }
        let text = serde_json::to_string(&next)
            .map_err(|e| Error::Internal(format!("联邦设置序列化失败: {e}")))?;
        self.repos
            .system
            .config_basic
            .save_code(FEDERATION_SETTINGS_CODE, &text)
            .await?;
        Ok(next)
    }

    /// 联邦面总闸：总开关关 ⇒ NotFound（路由层据此一律 404，全站零行为变化）。
    pub async fn gate(&self) -> Result<FederationSettings> {
        let settings = self.get_settings().await?;
        if !settings.enabled {
            return Err(Error::NotFound(MSG_FEDERATION_DISABLED.to_string()));
        }
        Ok(settings)
    }

    /// 组织概览（设置摘要 + 计数）。
    pub async fn overview(&self) -> Result<Value> {
        let settings = self.get_settings().await?;
        let repo = self.repo();
        Ok(json!({
            "settings": settings,
            "linksPending": repo.count_links_by_status(LINK_PENDING).await?,
            "linksActive": repo.count_links_by_status(LINK_ACTIVE).await?,
            "linksDisabled": repo.count_links_by_status(LINK_DISABLED).await?,
            "credentials": repo.count_credentials().await?,
            "aggregationProducts": repo.count_maps().await?,
        }))
    }

    // ———————————————————— 凭证管理（设计 §4） ————————————————————

    /// 凭证 → VO（范围 id 列表按需回填；product_count 入站钥实时算，出站钥 -1）。
    async fn credential_vo(&self, cred: &HubCredential, _with_ids: bool) -> Result<HubCredentialVO> {
        let (shop_ids, product_count) = if i32::from(cred.direction) == DIRECTION_INBOUND {
            let shops = if cred.scope_all_shops == 1 {
                Vec::new()
            } else {
                self.repo()
                    .list_id_list("hub_credential_shop", "shop_id", cred.credential_id)
                    .await?
            };
            let aud = self.audience_for_credential(cred).await?;
            (shops, self.feed.count_for(&aud).await?)
        } else {
            (Vec::new(), -1)
        };
        Ok(HubCredentialVO {
            credential_id: cred.credential_id,
            conn_type: i32::from(cred.conn_type),
            link_id: cred.link_id,
            direction: i32::from(cred.direction),
            peer_base_url: cred.peer_base_url.clone(),
            cred_name: cred.cred_name.clone(),
            access_key: cred.access_key.clone(),
            secret_masked: if i32::from(cred.direction) == DIRECTION_OUTBOUND {
                mask_secret(&self.decrypt_secret(&cred.secret_enc).unwrap_or_default())
            } else {
                String::new()
            },
            scopes: cred.scopes,
            scope_names: scope_names(cred.scopes)
                .into_iter()
                .map(ToString::to_string)
                .collect(),
            scope_all_shops: cred.scope_all_shops == 1,
            shop_ids,
            scope_mode: i32::from(cred.scope_mode),
            include_inquiry: cred.include_inquiry == 1,
            price_rule_type: cred.price_rule_type.clone(),
            price_rule_value: cred.price_rule_value,
            status: i32::from(cred.status),
            expire_time: cred.expire_time,
            rate_limit_per_min: cred.rate_limit_per_min,
            daily_quota: cred.daily_quota,
            ip_whitelist: cred.ip_whitelist.clone(),
            last_used_time: cred.last_used_time,
            call_count: cred.call_count,
            add_time: cred.add_time,
            update_time: cred.update_time,
            product_count,
        })
    }

    pub async fn list_credentials(
        &self,
        q: &HubCredentialListQuery,
    ) -> Result<PageResult<HubCredentialVO>> {
        let (rows, total) = self.repo().list_credentials(q).await?;
        let mut list = Vec::with_capacity(rows.len());
        for cred in &rows {
            list.push(self.credential_vo(cred, false).await?);
        }
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        Ok(PageResult::new(total.max(0) as u64, list, &page))
    }

    async fn load_credential(&self, credential_id: i32) -> Result<HubCredential> {
        self.repo()
            .find_credential_by_id(credential_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_CRED_NOT_FOUND.to_string()))
    }

    pub async fn credential_detail(
        &self,
        credential_id: i32,
    ) -> Result<HubCredentialDetailVO> {
        let cred = self.load_credential(credential_id).await?;
        let vo = self.credential_vo(&cred, true).await?;
        let product_ids = self
            .repo()
            .list_id_list("hub_credential_product", "product_id", credential_id)
            .await?;
        let category_ids = self
            .repo()
            .list_id_list("hub_credential_category", "category_id", credential_id)
            .await?;
        let overrides = self.repo().list_all_overrides(credential_id).await?;
        let mut override_vos = Vec::with_capacity(overrides.len());
        for ov in &overrides {
            override_vos.push(HubPriceOverrideVO {
                product_id: ov.product_id,
                product_name: None,
                base_price: None,
                rule_type: ov.rule_type.clone(),
                rule_value: ov.rule_value,
                final_price: None,
            });
        }
        Ok(HubCredentialDetailVO {
            cred: vo,
            product_ids,
            category_ids,
            price_overrides: override_vos,
        })
    }

    /// 校验一份凭证保存入参（名称/scopes/范围/规则），返回清洗后的关键字段。
    #[allow(clippy::too_many_arguments)]
    fn validate_credential_fields(
        settings: &FederationSettings,
        name: &str,
        scopes: i64,
        scope_mode: i32,
        product_ids: &[i32],
        category_ids: &[i32],
        rule_type: &str,
        rule_value: Decimal,
        expire_time: i32,
        now: i32,
    ) -> Result<()> {
        if name.trim().is_empty() {
            return Err(Error::Validation(MSG_CRED_NAME_REQUIRED.to_string()));
        }
        settings
            .validate_scopes(scopes)
            .map_err(Error::Validation)?;
        if !(1..=3).contains(&scope_mode) {
            return Err(Error::Validation("商品范围模式非法".to_string()));
        }
        if scope_mode == SCOPE_PRODUCTS && product_ids.is_empty() {
            return Err(Error::Validation(
                "指定商品范围时商品清单不能为空".to_string(),
            ));
        }
        if scope_mode == SCOPE_CATEGORIES && category_ids.is_empty() {
            return Err(Error::Validation(
                "指定分类范围时分类清单不能为空".to_string(),
            ));
        }
        validate_rule(rule_type, rule_value).map_err(Error::Validation)?;
        if expire_time != 0 && expire_time <= now {
            return Err(Error::Validation("过期时间必须晚于当前时间".to_string()));
        }
        Ok(())
    }

    /// 新建凭证。入站钥：本地签发（secret 明文只回这一次）；
    /// 出站钥：登记对端签发的钥（secret 加密落库）。
    pub async fn create_credential(
        &self,
        dto: &HubCredentialSaveDTO,
    ) -> Result<HubCredentialSecretVO> {
        let settings = self.gate().await?;
        let now = self.now();
        let direction = dto.direction.unwrap_or(DIRECTION_INBOUND);
        let conn_type = dto.conn_type.unwrap_or(CONN_SUPPLY_OUT);
        let name = dto.cred_name.as_deref().unwrap_or_default().trim().to_string();
        let scopes = dto.scopes.unwrap_or(SCOPE_META_READ | SCOPE_PRODUCT_READ);
        let scope_mode = dto.scope_mode.unwrap_or(1);
        let rule_type = dto
            .price_rule_type
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("none")
            .to_string();
        let rule_value = dto.price_rule_value.unwrap_or(Decimal::ZERO);
        let expire_time = dto.expire_time.unwrap_or(0);
        Self::validate_credential_fields(
            &settings,
            &name,
            scopes,
            scope_mode,
            &dto.product_ids,
            &dto.category_ids,
            &rule_type,
            rule_value,
            expire_time,
            now,
        )?;

        let (access_key, secret_plain, secret_hash, secret_enc) = if direction
            == DIRECTION_OUTBOUND
        {
            let ak = dto
                .peer_access_key
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| Error::Validation("出站凭证必须填写对端 accessKey".to_string()))?;
            let secret = dto
                .peer_secret
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| Error::Validation("出站凭证必须填写对端 secret".to_string()))?;
            (
                ak.to_string(),
                String::new(),
                String::new(),
                self.encrypt_secret(secret)?,
            )
        } else {
            let secret = gen_secret();
            let hash = utils::sha256_hex(&secret);
            (gen_access_key(), secret, hash, String::new())
        };

        let peer_base_url = if direction == DIRECTION_OUTBOUND {
            let url = dto.peer_base_url.as_deref().unwrap_or_default().trim();
            validate_base_url(url).map_err(Error::Validation)?;
            url.trim_end_matches('/').to_string()
        } else {
            String::new()
        };

        let id = self
            .repo()
            .insert_credential(
                conn_type,
                0,
                direction,
                &peer_base_url,
                &name,
                &access_key,
                &secret_hash,
                &secret_enc,
                scopes,
                i32::from(dto.scope_all_shops.unwrap_or(true)),
                scope_mode,
                i32::from(dto.include_inquiry.unwrap_or(false)),
                &rule_type,
                rule_value,
                expire_time,
                dto.rate_limit_per_min.unwrap_or(600),
                dto.daily_quota.unwrap_or(0),
                dto.ip_whitelist.as_deref().unwrap_or_default(),
                now,
            )
            .await?;
        self.replace_credential_scopes(id, dto).await?;
        tracing::info!("新建联邦凭证: id={id}, access_key={access_key}, direction={direction}");
        Ok(HubCredentialSecretVO {
            credential_id: id,
            access_key,
            secret: secret_plain,
        })
    }

    async fn replace_credential_scopes(
        &self,
        credential_id: i32,
        dto: &HubCredentialSaveDTO,
    ) -> Result<()> {
        let repo = self.repo();
        if dto.scope_all_shops.unwrap_or(true) {
            repo.replace_id_list("hub_credential_shop", "shop_id", credential_id, &[])
                .await?;
        } else {
            repo.replace_id_list(
                "hub_credential_shop",
                "shop_id",
                credential_id,
                &dto.shop_ids,
            )
            .await?;
        }
        repo.replace_id_list(
            "hub_credential_product",
            "product_id",
            credential_id,
            &dto.product_ids,
        )
        .await?;
        repo.replace_id_list(
            "hub_credential_category",
            "category_id",
            credential_id,
            &dto.category_ids,
        )
        .await?;
        Ok(())
    }

    /// 编辑凭证（范围清单整体替换；secret 不可经编辑改，出站钥换钥走 peer 字段）。
    pub async fn update_credential(&self, dto: &HubCredentialSaveDTO) -> Result<()> {
        let settings = self.gate().await?;
        let credential_id = dto
            .credential_id
            .ok_or_else(|| Error::Validation("缺少 credentialId".to_string()))?;
        let existing = self.load_credential(credential_id).await?;
        let now = self.now();
        let name = dto
            .cred_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(&existing.cred_name)
            .to_string();
        let scopes = dto.scopes.unwrap_or(existing.scopes);
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
        Self::validate_credential_fields(
            &settings,
            &name,
            scopes,
            scope_mode,
            &dto.product_ids,
            &dto.category_ids,
            &rule_type,
            rule_value,
            expire_time,
            now,
        )?;
        let peer_base_url = if i32::from(existing.direction) == DIRECTION_OUTBOUND {
            match dto.peer_base_url.as_deref().map(str::trim) {
                Some(url) if !url.is_empty() => {
                    validate_base_url(url).map_err(Error::Validation)?;
                    url.trim_end_matches('/').to_string()
                }
                _ => existing.peer_base_url.clone(),
            }
        } else {
            existing.peer_base_url.clone()
        };
        self.repo()
            .update_credential_fields(
                credential_id,
                &name,
                scopes,
                i32::from(dto.scope_all_shops.unwrap_or(existing.scope_all_shops == 1)),
                scope_mode,
                i32::from(dto.include_inquiry.unwrap_or(existing.include_inquiry == 1)),
                &rule_type,
                rule_value,
                dto.status.unwrap_or(i32::from(existing.status)),
                expire_time,
                dto.rate_limit_per_min
                    .unwrap_or(existing.rate_limit_per_min),
                dto.daily_quota.unwrap_or(existing.daily_quota),
                dto.ip_whitelist
                    .as_deref()
                    .unwrap_or(&existing.ip_whitelist),
                &peer_base_url,
                now,
            )
            .await?;
        // 出站钥换钥：填了新 secret/accessKey 才更新。
        if i32::from(existing.direction) == DIRECTION_OUTBOUND {
            if let Some(secret) = dto
                .peer_secret
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                let enc = self.encrypt_secret(secret)?;
                self.repo()
                    .update_credential_secret_enc(credential_id, &enc, now)
                    .await?;
            }
            if let Some(ak) = dto
                .peer_access_key
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                self.repo()
                    .update_credential_access_key(credential_id, ak, now)
                    .await?;
            }
        }
        self.replace_credential_scopes(credential_id, dto).await?;
        Ok(())
    }

    /// 重置入站钥 secret（旧 secret 立即失效；明文只回这一次）。
    pub async fn reset_credential_secret(
        &self,
        credential_id: i32,
    ) -> Result<HubCredentialSecretVO> {
        self.gate().await?;
        let cred = self.load_credential(credential_id).await?;
        if i32::from(cred.direction) != DIRECTION_INBOUND {
            return Err(Error::Validation(
                "出站凭证的 secret 由对端签发，不能在此重置".to_string(),
            ));
        }
        let secret = gen_secret();
        self.repo()
            .update_credential_secret_hash(
                credential_id,
                &utils::sha256_hex(&secret),
                self.now(),
            )
            .await?;
        Ok(HubCredentialSecretVO {
            credential_id,
            access_key: cred.access_key,
            secret,
        })
    }

    /// 删除凭证（连带范围子表与覆盖；调用日志保留）。
    pub async fn delete_credential(&self, credential_id: i32) -> Result<()> {
        self.gate().await?;
        let _ = self.load_credential(credential_id).await?;
        self.repo().delete_credential(credential_id).await
    }

    /// 保存单品价格覆盖。
    pub async fn save_override(&self, dto: &HubPriceOverrideSaveDTO) -> Result<()> {
        self.gate().await?;
        let _ = self.load_credential(dto.credential_id).await?;
        let rule_type = dto
            .rule_type
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("none");
        let rule_value = dto.rule_value.unwrap_or(Decimal::ZERO);
        validate_rule(rule_type, rule_value).map_err(Error::Validation)?;
        self.repo()
            .upsert_override(
                dto.credential_id,
                dto.product_id,
                rule_type,
                rule_value,
                self.now(),
            )
            .await
    }

    pub async fn delete_override(&self, credential_id: i32, product_id: i32) -> Result<()> {
        self.gate().await?;
        self.repo().delete_override(credential_id, product_id).await
    }

    /// feed 钥「一键生成等价凭证」（设计 §4.5）：范围/规则/覆盖整表复制，
    /// scopes = meta.read + product.read，连接类型=单向供货。
    pub async fn from_feed_key(&self, feed_key_id: i32) -> Result<HubCredentialSecretVO> {
        let settings = self.gate().await?;
        let detail = self.feed.detail(feed_key_id).await?;
        let key = &detail.key;
        let scopes = SCOPE_META_READ | SCOPE_PRODUCT_READ;
        settings
            .validate_scopes(scopes)
            .map_err(Error::Validation)?;
        let now = self.now();
        let access_key = gen_access_key();
        let secret = gen_secret();
        let id = self
            .repo()
            .insert_credential(
                CONN_SUPPLY_OUT,
                0,
                DIRECTION_INBOUND,
                "",
                &format!("{}（feed 等价凭证）", key.key_name),
                &access_key,
                &utils::sha256_hex(&secret),
                "",
                scopes,
                1,
                i32::from(key.scope_mode),
                i32::from(key.include_inquiry),
                &key.price_rule_type,
                key.price_rule_value,
                key.expire_time,
                600,
                0,
                "",
                now,
            )
            .await?;
        let repo = self.repo();
        repo.replace_id_list(
            "hub_credential_product",
            "product_id",
            id,
            &detail.product_ids,
        )
        .await?;
        repo.replace_id_list(
            "hub_credential_category",
            "category_id",
            id,
            &detail.category_ids,
        )
        .await?;
        for ov in &detail.price_overrides {
            repo.upsert_override(id, ov.product_id, &ov.rule_type, ov.rule_value, now)
                .await?;
        }
        tracing::info!("feed 钥 {feed_key_id} 生成等价联邦凭证: id={id}");
        Ok(HubCredentialSecretVO {
            credential_id: id,
            access_key,
            secret,
        })
    }

    // ———————————————————— 受众解析（与 feed 同口径） ————————————————————

    /// 凭证 → 商品受众（范围集合 + 价格规则），与 feed 钥共用查询口径。
    pub async fn audience_for_credential(
        &self,
        cred: &HubCredential,
    ) -> Result<FeedAudience> {
        let repo = self.repo();
        let product_ids = if i32::from(cred.scope_mode) == SCOPE_PRODUCTS {
            Some(
                repo.list_id_list("hub_credential_product", "product_id", cred.credential_id)
                    .await?
                    .into_iter()
                    .collect::<HashSet<i32>>(),
            )
        } else {
            None
        };
        let category_ids = if i32::from(cred.scope_mode) == SCOPE_CATEGORIES {
            let selected = repo
                .list_id_list("hub_credential_category", "category_id", cred.credential_id)
                .await?;
            let all = self.repos.catalog.product_feed.list_all_categories().await?;
            let pairs: Vec<(i32, i32)> = all
                .iter()
                .map(|c| (c.category_id, c.parent_id))
                .collect();
            Some(expand_categories(&selected, &pairs))
        } else {
            None
        };
        let shop_ids = if cred.scope_all_shops == 1 {
            None
        } else {
            Some(
                repo.list_id_list("hub_credential_shop", "shop_id", cred.credential_id)
                    .await?
                    .into_iter()
                    .collect::<HashSet<i32>>(),
            )
        };
        let overrides = repo
            .list_all_overrides(cred.credential_id)
            .await?
            .into_iter()
            .map(|ov| {
                (
                    ov.product_id,
                    PriceRule {
                        rule_type: ov.rule_type,
                        value: ov.rule_value,
                    },
                )
            })
            .collect::<HashMap<i32, PriceRule>>();
        Ok(FeedAudience {
            scope_mode: i32::from(cred.scope_mode),
            include_inquiry: cred.include_inquiry == 1,
            product_ids,
            category_ids,
            shop_ids,
            default_rule: PriceRule {
                rule_type: cred.price_rule_type.clone(),
                value: cred.price_rule_value,
            },
            overrides,
        })
    }

    // ———————————————————— 联邦面鉴权（设计 §4.2/§8） ————————————————————

    /// 凭证鉴权：哈希恒定时间比对 → 状态/过期 → 连接状态 → 模块交集 →
    /// scope 位 → 限流/配额。通过返回 (凭证, 所属连接, 生效设置)。
    pub async fn authenticate(
        &self,
        access_key: &str,
        secret: &str,
        required_scope: i64,
    ) -> Result<(HubCredential, Option<HubLink>, FederationSettings)> {
        let settings = self.gate().await?;
        // 定向取入站行：同一把钥在持有方另有出站孪生行（同 access_key），
        // 回环同库时两行共存，按钥裸查取行不确定——鉴权只认入站行（存哈希方）。
        let cred = self
            .repo()
            .find_inbound_credential_by_access_key(access_key.trim())
            .await?
            .ok_or_else(|| Error::Unauthorized(MSG_HUB_UNAUTHORIZED.to_string()))?;
        let provided = utils::sha256_hex(secret.trim());
        if !constant_time_eq(provided.as_bytes(), cred.secret_hash.as_bytes()) {
            return Err(Error::Unauthorized(MSG_HUB_UNAUTHORIZED.to_string()));
        }
        if cred.status != 1 {
            return Err(Error::Forbidden(MSG_HUB_DISABLED.to_string()));
        }
        let now = self.now();
        if cred.expire_time != 0 && cred.expire_time <= now {
            return Err(Error::Forbidden(MSG_HUB_EXPIRED.to_string()));
        }
        // 能力门：单向供货钥要求「开放供货」开；联邦钥要求「全联邦」开。
        match i32::from(cred.conn_type) {
            CONN_SUPPLY_OUT => {
                if !settings.capabilities.supply_out && !settings.capabilities.federation {
                    return Err(Error::Forbidden(MSG_FEDERATION_DISABLED.to_string()));
                }
            }
            CONN_FEDERATION => {
                if !settings.capabilities.federation {
                    return Err(Error::Forbidden(MSG_FEDERATION_DISABLED.to_string()));
                }
            }
            _ => {}
        }
        // 连接联动：所属连接必须已激活（停用即断）。
        let link = if cred.link_id > 0 {
            let link = self
                .repo()
                .find_link_by_id(cred.link_id)
                .await?
                .ok_or_else(|| Error::Forbidden(MSG_HUB_LINK_INACTIVE.to_string()))?;
            if i32::from(link.status) != LINK_ACTIVE {
                return Err(Error::Forbidden(MSG_HUB_LINK_INACTIVE.to_string()));
            }
            Some(link)
        } else {
            None
        };
        // 模块交集：scope 所属模块须在本机模块开关内，且（联邦钥）须在连接批准上限内。
        if let Some(module) = scope_module(required_scope) {
            if settings.module_bits() & module == 0 {
                return Err(Error::Forbidden(MSG_HUB_SCOPE_DENIED.to_string()));
            }
            if let Some(link) = &link {
                if link.approved_modules & module == 0 {
                    return Err(Error::Forbidden(MSG_HUB_SCOPE_DENIED.to_string()));
                }
            }
        }
        if cred.scopes & required_scope == 0 {
            return Err(Error::Forbidden(MSG_HUB_SCOPE_DENIED.to_string()));
        }
        // 限流（分钟窗口，进程内）与日配额（调用日志计数）。
        if cred.rate_limit_per_min > 0 {
            let now_s = utils::now_seconds();
            let mut windows = self.rate_windows.lock().unwrap_or_else(|p| p.into_inner());
            let entry = windows.entry(cred.credential_id).or_insert((now_s, 0));
            if now_s - entry.0 >= 60 {
                *entry = (now_s, 0);
            }
            entry.1 += 1;
            if entry.1 > cred.rate_limit_per_min.max(0) as u32 {
                return Err(Error::Biz {
                    code: 429,
                    msg: MSG_HUB_RATE_LIMITED.to_string(),
                });
            }
        }
        if cred.daily_quota > 0 {
            let day_start = (utils::now_seconds() / 86_400 * 86_400) as i32;
            let used = self
                .repo()
                .count_calls_today(cred.credential_id, day_start)
                .await?;
            if used >= cred.daily_quota {
                return Err(Error::Biz {
                    code: 429,
                    msg: MSG_HUB_RATE_LIMITED.to_string(),
                });
            }
        }
        // 用量节流回写（60 秒最多一次）。
        {
            let mut touches = self.last_touch.lock().unwrap_or_else(|p| p.into_inner());
            let last = touches.get(&cred.credential_id).copied().unwrap_or(0);
            if now - last >= 60 {
                touches.insert(cred.credential_id, now);
                let repo = self.repo().clone();
                let cid = cred.credential_id;
                tokio::spawn(async move {
                    let _ = repo.touch_credential_usage(cid, now).await;
                });
            }
        }
        Ok((cred, link, settings))
    }

    /// 调用日志落表（路由层在 handler 完成时调用；失败不影响主流程）。
    #[allow(clippy::too_many_arguments)]
    pub async fn record_call(
        &self,
        cred: &HubCredential,
        endpoint: &str,
        scope: &str,
        http_status: i32,
        result_count: i32,
        client_ip: &str,
        started: Instant,
    ) {
        let duration_ms = i32::try_from(started.elapsed().as_millis()).unwrap_or(i32::MAX);
        let _ = self
            .repo()
            .insert_call_log(
                cred.link_id,
                cred.credential_id,
                &cred.access_key,
                endpoint,
                scope,
                i32::from(cred.direction),
                http_status,
                result_count,
                client_ip,
                duration_ms,
                self.now(),
            )
            .await;
    }

    pub async fn list_call_logs(
        &self,
        q: &HubCallLogListQuery,
    ) -> Result<PageResult<HubCallLogRow>> {
        let (rows, total) = self.repo().list_call_logs(q).await?;
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        Ok(PageResult::new(total.max(0) as u64, rows, &page))
    }

    // ———————————————————— 联邦面：商品与元信息 ————————————————————

    pub async fn hub_meta(
        &self,
        cred: &HubCredential,
        settings: &FederationSettings,
    ) -> Result<HubMetaVO> {
        let product_count = if cred.scopes & SCOPE_PRODUCT_READ != 0 {
            let aud = self.audience_for_credential(cred).await?;
            self.feed.count_for(&aud).await?
        } else {
            0
        };
        Ok(HubMetaVO {
            instance_uuid: settings.instance_uuid.clone(),
            instance_name: settings.instance_name.clone(),
            version: INSTANCE_VERSION.to_string(),
            server_time: utils::now_seconds(),
            api_version: API_VERSION.to_string(),
            credential_name: cred.cred_name.clone(),
            scopes: cred.scopes,
            product_count,
        })
    }

    pub async fn hub_categories(
        &self,
        cred: &HubCredential,
    ) -> Result<Vec<nextrade_core::models::catalog::product_feed::FeedCategoryVO>> {
        let aud = self.audience_for_credential(cred).await?;
        self.feed.categories_for(&aud).await
    }

    pub async fn hub_products(
        &self,
        cred: &HubCredential,
        q: &FeedProductListQuery,
    ) -> Result<PageResult<nextrade_core::models::catalog::product_feed::FeedProductVO>> {
        let aud = self.audience_for_credential(cred).await?;
        self.feed.products_for(&aud, q).await
    }

    pub async fn hub_product_detail(
        &self,
        cred: &HubCredential,
        product_id: i32,
    ) -> Result<nextrade_core::models::catalog::product_feed::FeedProductDetailVO> {
        let aud = self.audience_for_credential(cred).await?;
        self.feed.product_detail_for(&aud, product_id).await
    }

    // ———————————————————— 配对（设计 §3.3，中枢侧） ————————————————————

    /// 生成一次性配对码（只存哈希，明文只回一次；24h 过期）。
    pub async fn create_pairing_code(
        &self,
        dto: &HubPairingCreateDTO,
    ) -> Result<HubPairingCodeVO> {
        let settings = self.gate().await?;
        if !settings.capabilities.federation {
            return Err(Error::Forbidden(MSG_FEDERATION_DISABLED.to_string()));
        }
        let code = gen_pairing_code();
        let expire = self.now() + i32::try_from(PAIRING_TTL_SECONDS).unwrap_or(86_400);
        let id = self
            .repo()
            .insert_pairing_code(
                &utils::sha256_hex(normalize_pairing_code(&code)),
                dto.expect_name.as_deref().unwrap_or_default().trim(),
                expire,
                self.now(),
            )
            .await?;
        Ok(HubPairingCodeVO {
            id,
            code,
            expect_name: dto.expect_name.clone().unwrap_or_default(),
            expire_time: expire,
        })
    }

    /// 配对码近况（哈希不回前端）。
    pub async fn list_pairing_codes(&self) -> Result<Vec<Value>> {
        self.gate().await?;
        let rows = self.repo().list_pairing_codes().await?;
        Ok(rows
            .iter()
            .map(|r| {
                json!({
                    "id": r.id,
                    "expectName": r.expect_name,
                    "status": r.status,
                    "failCount": r.fail_count,
                    "expireTime": r.expire_time,
                    "usedLinkId": r.used_link_id,
                    "addTime": r.add_time,
                })
            })
            .collect())
    }

    /// 作废一张未用配对码。
    pub async fn revoke_pairing_code(&self, id: i32) -> Result<()> {
        self.gate().await?;
        self.repo().void_pairing_code(id).await
    }

    /// 成员发起 claim（中枢侧）：验码 → 建待审批连接 + 落下行出站凭证 → 发 claim token。
    pub async fn pair_claim(&self, dto: &PairClaimDTO) -> Result<PairClaimVO> {
        let settings = self.gate().await?;
        if !settings.capabilities.federation {
            return Err(Error::Forbidden(MSG_FEDERATION_DISABLED.to_string()));
        }
        if dto.instance_uuid.trim().is_empty() || dto.down_access_key.trim().is_empty() {
            return Err(Error::Validation("成员实例信息不完整".to_string()));
        }
        validate_base_url(dto.base_url.trim()).map_err(Error::Validation)?;
        let now = self.now();
        let hash = utils::sha256_hex(normalize_pairing_code(&dto.pairing_code));
        let pairing = self
            .repo()
            .find_pairing_by_hash(&hash)
            .await?
            .ok_or_else(|| Error::Forbidden(MSG_PAIRING_INVALID.to_string()))?;
        match i32::from(pairing.status) {
            PAIRING_USED => return Err(Error::Forbidden(MSG_PAIRING_EXPIRED.to_string())),
            PAIRING_VOID => return Err(Error::Forbidden(MSG_PAIRING_FORBIDDEN.to_string())),
            _ => {}
        }
        if pairing.expire_time != 0 && pairing.expire_time <= now {
            return Err(Error::Forbidden(MSG_PAIRING_EXPIRED.to_string()));
        }
        let claim_token = gen_claim_token();
        // 先加密后落库：加密失败（主密钥缺失）须在建连接行之前暴露，
        // 不许落进下方补偿区再用 ? 提前返回、绕过孤儿清理。
        let down_secret_enc = self.encrypt_secret(dto.down_secret.trim())?;
        let link_id = self
            .repo()
            .insert_link(
                LINK_ROLE_HUB,
                dto.instance_uuid.trim(),
                dto.instance_name.trim(),
                dto.base_url.trim().trim_end_matches('/'),
                dto.version.trim(),
                LINK_PENDING,
                &utils::sha256_hex(&claim_token),
                "",
                now,
            )
            .await?;
        // 下行钥：中枢持有调成员 → 本库记为出站凭证（secret 加密存）。
        let down_scopes = dto
            .down_scopes
            .unwrap_or(SCOPE_META_READ | SCOPE_PRODUCT_READ | SCOPE_ORDER_READ | SCOPE_ORDER_PLACE | SCOPE_MEMBER_READ | SCOPE_WALLET_POST | SCOPE_CONFIG_PULL);
        // 落钥三步任一失败 → 补偿删除已落的凭证行与连接行，不留孤儿：
        // 配对码尚未标记已用，成员可用同一码重新 claim（从零开始）。
        let mut down_cred_id = 0;
        let mut fail: Option<Error> = None;
        match self
            .repo()
            .insert_credential(
                CONN_FEDERATION,
                link_id,
                DIRECTION_OUTBOUND,
                "",
                &format!("下行钥·{}", dto.instance_name.trim()),
                dto.down_access_key.trim(),
                "",
                &down_secret_enc,
                down_scopes,
                1,
                1,
                0,
                "none",
                Decimal::ZERO,
                0,
                600,
                0,
                "",
                now,
            )
            .await
        {
            Ok(id) => down_cred_id = id,
            Err(e) => fail = Some(e),
        }
        if fail.is_none() {
            if let Err(e) = self
                .repo()
                .update_link_credentials(link_id, 0, down_cred_id, now)
                .await
            {
                fail = Some(e);
            }
        }
        if fail.is_none() {
            if let Err(e) = self.repo().mark_pairing_used(pairing.id, link_id).await {
                fail = Some(e);
            }
        }
        if let Some(e) = fail {
            if down_cred_id > 0 {
                let _ = self.repo().delete_credential(down_cred_id).await;
            }
            let _ = self.repo().delete_link(link_id).await;
            return Err(e);
        }
        tracing::info!("联邦配对 claim 成功: link={link_id}, member={}", dto.instance_uuid);
        Ok(PairClaimVO {
            link_id,
            claim_token,
            status: LINK_PENDING,
        })
    }

    /// 查配对状态（claim token 哈希比对）。
    pub async fn pair_status(&self, dto: &PairStatusDTO) -> Result<PairStatusVO> {
        self.gate().await?;
        let link = self.load_claim_link(dto).await?;
        Ok(PairStatusVO {
            link_id: link.link_id,
            status: i32::from(link.status),
            reject_reason: link.reject_reason.clone(),
            approved_modules: link.approved_modules,
        })
    }

    /// 领取上行钥（批准后 7 天窗口内、凭 claim token 一次性领取）。
    pub async fn pair_activate(&self, dto: &PairStatusDTO) -> Result<PairActivateVO> {
        let settings = self.gate().await?;
        let link = self.load_claim_link(dto).await?;
        let now = self.now();
        if i32::from(link.status) == LINK_ACTIVE && link.claim_expire_time != 0 && link.claim_expire_time <= now
        {
            self.repo()
                .update_link_status(link.link_id, LINK_EXPIRED, "", now)
                .await?;
            return Err(Error::Forbidden(MSG_PAIRING_EXPIRED.to_string()));
        }
        if i32::from(link.status) != LINK_ACTIVE {
            return Err(Error::Forbidden(MSG_HUB_LINK_INACTIVE.to_string()));
        }
        if link.up_credential_id > 0 {
            return Err(Error::Forbidden(MSG_CLAIM_TOKEN_INVALID.to_string()));
        }
        let scopes = up_scopes_for_modules(link.approved_modules);
        settings
            .validate_scopes(scopes)
            .map_err(Error::Validation)?;
        let secret = gen_secret();
        let access_key = gen_access_key();
        let up_id = self
            .repo()
            .insert_credential(
                CONN_FEDERATION,
                link.link_id,
                DIRECTION_INBOUND,
                "",
                &format!("上行钥·{}", link.peer_name),
                &access_key,
                &utils::sha256_hex(&secret),
                "",
                scopes,
                1,
                1,
                0,
                "none",
                Decimal::ZERO,
                0,
                600,
                0,
                "",
                now,
            )
            .await?;
        // 回填/清 token 任一失败 → 补偿：先把连接的上行指针归零，再删已落的上行钥，
        // 不留「连接不知情、却真实有效」的悬空入站钥；claim token 未清，
        // 成员可重新领取（届时重新生成一把新钥，本把已删干净）。
        let mut fail: Option<Error> = None;
        if let Err(e) = self
            .repo()
            .update_link_credentials(link.link_id, up_id, link.down_credential_id, now)
            .await
        {
            fail = Some(e);
        }
        if fail.is_none() {
            if let Err(e) = self.repo().clear_link_claim_token(link.link_id, now).await {
                fail = Some(e);
            }
        }
        if let Some(e) = fail {
            let _ = self
                .repo()
                .update_link_credentials(link.link_id, 0, link.down_credential_id, now)
                .await;
            let _ = self.repo().delete_credential(up_id).await;
            return Err(e);
        }
        tracing::info!("联邦配对领取上行钥: link={}", link.link_id);
        Ok(PairActivateVO {
            link_id: link.link_id,
            status: LINK_ACTIVE,
            up_access_key: access_key,
            up_secret: secret,
        })
    }

    async fn load_claim_link(&self, dto: &PairStatusDTO) -> Result<HubLink> {
        let link = self
            .repo()
            .find_link_by_id(dto.link_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_LINK_NOT_FOUND.to_string()))?;
        let provided = utils::sha256_hex(dto.claim_token.trim());
        if link.claim_token_hash.is_empty()
            || !constant_time_eq(provided.as_bytes(), link.claim_token_hash.as_bytes())
        {
            return Err(Error::Unauthorized(MSG_CLAIM_TOKEN_INVALID.to_string()));
        }
        Ok(link)
    }

    // ———————————————————— 连接管理（中枢侧管理端） ————————————————————

    pub async fn list_links(&self, q: &HubLinkListQuery) -> Result<PageResult<HubLinkVO>> {
        let (rows, total) = self.repo().list_links(q).await?;
        let now = utils::now_seconds();
        let list = rows.iter().map(|l| link_to_vo(l, now)).collect();
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        Ok(PageResult::new(total.max(0) as u64, list, &page))
    }

    async fn load_link(&self, link_id: i32) -> Result<HubLink> {
        self.repo()
            .find_link_by_id(link_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_LINK_NOT_FOUND.to_string()))
    }

    /// 批准连接（写批准模块上限/伙伴账户/结算结构 + 开 7 天领取窗口；不预签上行钥）。
    pub async fn approve_link(&self, dto: &HubLinkApproveDTO) -> Result<()> {
        let settings = self.gate().await?;
        let link = self.load_link(dto.link_id).await?;
        if i32::from(link.status) != LINK_PENDING {
            return Err(Error::Validation("仅待审批连接可批准".to_string()));
        }
        let modules = dto.approved_modules.unwrap_or(settings.module_bits());
        if modules & !settings.module_bits() != 0 {
            return Err(Error::Validation(
                "批准模块超出本机已开启的模块范围".to_string(),
            ));
        }
        let settle_mode = dto.settle_mode.as_deref().unwrap_or("");
        let settle_cycle = dto.settle_cycle.as_deref().unwrap_or("");
        let fund_channel = dto.fund_channel.as_deref().unwrap_or("");
        let rate_source = dto.rate_source.as_deref().unwrap_or("level");
        validate_settle_fields(settle_mode, settle_cycle, fund_channel, rate_source)
            .map_err(Error::Validation)?;
        // 伙伴账户绑定（P1 只落结构）：给了 user_id 必须真实存在。
        let partner_user_id = dto.partner_user_id.unwrap_or(0);
        if partner_user_id > 0
            && self
                .repos
                .member
                .user
                .find_by_id(partner_user_id)
                .await?
                .is_none()
        {
            return Err(Error::Validation("伙伴账户用户不存在".to_string()));
        }
        let now = self.now();
        let claim_expire = now + i32::try_from(CLAIM_WINDOW_SECONDS).unwrap_or(604_800);
        self.repo()
            .approve_link(
                dto.link_id,
                modules,
                partner_user_id,
                settle_mode,
                settle_cycle,
                fund_channel,
                rate_source,
                dto.rate_override.unwrap_or(Decimal::ZERO),
                claim_expire,
                now,
            )
            .await
    }

    /// 拒绝连接（成员轮询到拒绝后自清下行钥）。
    pub async fn reject_link(&self, dto: &HubLinkRejectDTO) -> Result<()> {
        self.gate().await?;
        let link = self.load_link(dto.link_id).await?;
        if i32::from(link.status) != LINK_PENDING {
            return Err(Error::Validation("仅待审批连接可拒绝".to_string()));
        }
        self.repo()
            .update_link_status(
                dto.link_id,
                LINK_REJECTED,
                dto.reason.as_deref().unwrap_or_default().trim(),
                self.now(),
            )
            .await
    }

    /// 连接启停（已激活 ⇄ 已停用；停用后其全部凭证运行时即刻失效）。
    pub async fn set_link_status(&self, link_id: i32, status: i32) -> Result<()> {
        self.gate().await?;
        let link = self.load_link(link_id).await?;
        if !matches!(status, LINK_ACTIVE | LINK_DISABLED) {
            return Err(Error::Validation("仅支持启用/停用两种状态".to_string()));
        }
        if i32::from(link.status) != LINK_ACTIVE && i32::from(link.status) != LINK_DISABLED {
            return Err(Error::Validation("该连接当前状态不可启停".to_string()));
        }
        self.repo()
            .update_link_status(link_id, status, &link.reject_reason, self.now())
            .await
    }

    /// 删除连接（设计 §3.5：双向凭证删除、聚合映射置「已断开」保留、
    /// 已同步数据不自动删、调用日志保留）。
    pub async fn delete_link(&self, link_id: i32) -> Result<()> {
        self.gate().await?;
        let link = self.load_link(link_id).await?;
        let now = self.now();
        if link.up_credential_id > 0 {
            self.repo().delete_credential(link.up_credential_id).await?;
        }
        if link.down_credential_id > 0 {
            self.repo()
                .delete_credential(link.down_credential_id)
                .await?;
        }
        // 聚合映射置已断开（sync_state=4），不删行（分页扫全）。
        for page in 1..=100u64 {
            let (maps, _) = self
                .repo()
                .list_product_maps(&HubAggregationListQuery {
                    link_id: Some(link_id),
                    page: Some(page),
                    page_size: Some(100),
                    ..Default::default()
                })
                .await?;
            if maps.is_empty() {
                break;
            }
            for m in &maps {
                self.repo().set_map_state(m.map_id, 4, now).await?;
            }
        }
        self.repo().delete_link(link_id).await
    }

    // ———————————————————— 配对（成员侧，我的连接） ————————————————————

    /// 发起配对：本地签发下行钥 → 调中枢 claim → 建本地待审批连接。
    pub async fn initiate_pairing(&self, dto: &HubPairInitiateDTO) -> Result<HubLinkVO> {
        let settings = self.gate().await?;
        if !settings.capabilities.federation {
            return Err(Error::Forbidden(MSG_FEDERATION_DISABLED.to_string()));
        }
        if settings.instance_uuid.is_empty() || settings.instance_base_url.is_empty() {
            return Err(Error::Validation(
                "请先在「设置 > 平台联邦」中完善本实例身份与对外地址".to_string(),
            ));
        }
        let hub_url = validate_base_url(dto.hub_base_url.as_deref().unwrap_or_default())
            .map_err(Error::Validation)?;
        let code = dto.pairing_code.as_deref().unwrap_or_default().trim();
        if code.is_empty() {
            return Err(Error::Validation("配对码不能为空".to_string()));
        }
        let down_access_key = gen_access_key();
        let down_secret = gen_secret();
        let down_scopes = SCOPE_META_READ | SCOPE_PRODUCT_READ | SCOPE_ORDER_READ
            | SCOPE_ORDER_PLACE | SCOPE_MEMBER_READ | SCOPE_WALLET_POST | SCOPE_CONFIG_PULL;
        let claim = hub_post_json(
            &hub_url,
            "/api/hub/v1/pair/claim",
            "",
            "",
            &json!({
                "pairingCode": code,
                "instanceUuid": settings.instance_uuid,
                "instanceName": settings.instance_name,
                "baseUrl": settings.instance_base_url,
                "version": INSTANCE_VERSION,
                "downAccessKey": down_access_key,
                "downSecret": down_secret,
                "downScopes": down_scopes,
            }),
        )
        .await?;
        let hub_link_id = claim.get("linkId").and_then(Value::as_i64).unwrap_or(0) as i32;
        let claim_token = claim
            .get("claimToken")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if hub_link_id <= 0 || claim_token.is_empty() {
            return Err(Error::Internal("中枢 claim 响应不完整".to_string()));
        }
        let now = self.now();
        let link_id = self
            .repo()
            .insert_link(
                LINK_ROLE_MEMBER,
                "",
                "",
                &hub_url,
                "",
                LINK_PENDING,
                &utils::sha256_hex(&claim_token),
                &self.encrypt_secret(&claim_token)?,
                now,
            )
            .await?;
        // 本地落行三步任一失败 → 补偿删除已落的下行钥行与连接行，不留孤儿连接
        // （§3 实测：失败后留下 status=待审批、down_credential_id=0 的悬空行）。
        // 中枢侧连接已建、配对码已消耗，补偿只清本侧；成员须向中枢索取新码重试，
        // 中枢侧那条待审批连接由中枢管理员拒绝/过期处置（设计 §3.5）。
        let mut down_cred_id = 0;
        let mut fail: Option<Error> = None;
        if let Err(e) = self
            .repo()
            .update_link_peer_link_id(link_id, hub_link_id, now)
            .await
        {
            fail = Some(e);
        }
        if fail.is_none() {
            match self
                .repo()
                .insert_credential(
                    CONN_FEDERATION,
                    link_id,
                    DIRECTION_INBOUND,
                    "",
                    "下行钥·中枢",
                    &down_access_key,
                    &utils::sha256_hex(&down_secret),
                    "",
                    down_scopes,
                    1,
                    1,
                    0,
                    "none",
                    Decimal::ZERO,
                    0,
                    600,
                    0,
                    "",
                    now,
                )
                .await
            {
                Ok(id) => down_cred_id = id,
                Err(e) => fail = Some(e),
            }
        }
        if fail.is_none() {
            if let Err(e) = self
                .repo()
                .update_link_credentials(link_id, 0, down_cred_id, now)
                .await
            {
                fail = Some(e);
            }
        }
        if let Some(e) = fail {
            if down_cred_id > 0 {
                let _ = self.repo().delete_credential(down_cred_id).await;
            }
            let _ = self.repo().delete_link(link_id).await;
            return Err(e);
        }
        let link = self.load_link(link_id).await?;
        Ok(link_to_vo(&link, utils::now_seconds()))
    }

    /// 轮询配对状态：批准后自动领取上行钥并激活本地连接；被拒则自清下行钥。
    pub async fn poll_link(&self, link_id: i32) -> Result<HubLinkVO> {
        self.gate().await?;
        let link = self.load_link(link_id).await?;
        if i32::from(link.role) != LINK_ROLE_MEMBER || link.claim_token_enc.is_empty() {
            return Ok(link_to_vo(&link, utils::now_seconds()));
        }
        let now = self.now();
        // 角色感知复用（先于一切远程调用）：本连接已有出站行（= 上行钥本地副本）时，
        // 说明上一轮 poll 已领取并落库、只是回填连接中途失败。此刻中枢侧 claim
        // token 已在领取时烧毁，重走 pair/status 只会被以 token 失效拒绝——而本地
        // 这一行是上行 secret 的唯一副本，绝不可删、也无从再领。直接用它补完
        // 回填（批准模块已在领取前先行落库，见 ACTIVE 分支），全程幂等。
        if let Some(existing) = self
            .repo()
            .find_link_credential(link_id, DIRECTION_OUTBOUND)
            .await?
        {
            self.repo()
                .update_link_credentials(link_id, existing.credential_id, link.down_credential_id, now)
                .await?;
            self.repo()
                .update_link_activation(link_id, link.approved_modules, now)
                .await?;
            self.repo().clear_link_claim_token(link_id, now).await?;
            let link = self.load_link(link_id).await?;
            return Ok(link_to_vo(&link, utils::now_seconds()));
        }
        let token = self.decrypt_secret(&link.claim_token_enc)?;
        let body = json!({ "linkId": link.peer_link_id, "claimToken": token });
        let status = hub_post_json(&link.peer_base_url, "/api/hub/v1/pair/status", "", "", &body)
            .await?;
        let remote_status = status.get("status").and_then(Value::as_i64).unwrap_or(0) as i32;
        match remote_status {
            LINK_PENDING => Ok(link_to_vo(&link, utils::now_seconds())),
            LINK_REJECTED => {
                let reason = status
                    .get("rejectReason")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                if link.down_credential_id > 0 {
                    self.repo().delete_credential(link.down_credential_id).await?;
                }
                self.repo()
                    .update_link_status(link_id, LINK_REJECTED, &reason, now)
                    .await?;
                self.repo().clear_link_claim_token(link_id, now).await?;
                let link = self.load_link(link_id).await?;
                Ok(link_to_vo(&link, utils::now_seconds()))
            }
            LINK_EXPIRED => {
                self.repo()
                    .update_link_status(link_id, LINK_EXPIRED, "", now)
                    .await?;
                self.repo().clear_link_claim_token(link_id, now).await?;
                let link = self.load_link(link_id).await?;
                Ok(link_to_vo(&link, utils::now_seconds()))
            }
            LINK_ACTIVE => {
                let approved_modules = status
                    .get("approvedModules")
                    .and_then(Value::as_i64)
                    .unwrap_or(0);
                // 先把批准模块落本地再领取：领取是一次性动作（中枢随即烧 token），
                // 此后任何一步失败都靠 poll 头部的复用路径补完回填，
                // 而复用路径只能从本地行读批准模块——故必须先行落库。
                self.repo()
                    .update_link_approved_modules(link_id, approved_modules, now)
                    .await?;
                let act = hub_post_json(
                    &link.peer_base_url,
                    "/api/hub/v1/pair/activate",
                    "",
                    "",
                    &body,
                )
                .await?;
                let up_access_key = act
                    .get("upAccessKey")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let up_secret = act.get("upSecret").and_then(Value::as_str).unwrap_or_default();
                if up_access_key.is_empty() || up_secret.is_empty() {
                    return Err(Error::Internal("中枢 activate 响应不完整".to_string()));
                }
                // 落钥带复用守卫：同（钥, 方向, 连接）已有行则复用，绝不重复插；
                // 插入报错时再复查一次（连接中断类错误下行可能已落库），查到即复用。
                // 注意失败时不删已落行：它已是上行 secret 的唯一副本（见 poll 头部复用）。
                let up_id = match self
                    .repo()
                    .find_credential_by_key_dir_link(
                        up_access_key,
                        DIRECTION_OUTBOUND,
                        link_id,
                    )
                    .await?
                {
                    Some(existing) => existing.credential_id,
                    None => {
                        let up_secret_enc = self.encrypt_secret(up_secret)?;
                        match self
                            .repo()
                            .insert_credential(
                                CONN_FEDERATION,
                                link_id,
                                DIRECTION_OUTBOUND,
                                "",
                                "上行钥·中枢",
                                up_access_key,
                                "",
                                &up_secret_enc,
                                up_scopes_for_modules(approved_modules),
                                1,
                                1,
                                0,
                                "none",
                                Decimal::ZERO,
                                0,
                                600,
                                0,
                                "",
                                now,
                            )
                            .await
                        {
                            Ok(id) => id,
                            Err(e) => match self
                                .repo()
                                .find_credential_by_key_dir_link(
                                    up_access_key,
                                    DIRECTION_OUTBOUND,
                                    link_id,
                                )
                                .await
                            {
                                Ok(Some(existing)) => existing.credential_id,
                                _ => return Err(e),
                            },
                        }
                    }
                };
                self.repo()
                    .update_link_credentials(link_id, up_id, link.down_credential_id, now)
                    .await?;
                self.repo()
                    .update_link_activation(link_id, approved_modules, now)
                    .await?;
                self.repo().clear_link_claim_token(link_id, now).await?;
                let link = self.load_link(link_id).await?;
                Ok(link_to_vo(&link, utils::now_seconds()))
            }
            _ => Ok(link_to_vo(&link, utils::now_seconds())),
        }
    }

    /// 成员侧手动发一次心跳（自动调度在 P4 随配置下发一并接入）。
    pub async fn send_heartbeat(&self, link_id: i32) -> Result<()> {
        let settings = self.gate().await?;
        let link = self.load_link(link_id).await?;
        if i32::from(link.role) != LINK_ROLE_MEMBER || i32::from(link.status) != LINK_ACTIVE {
            return Err(Error::Validation("仅已激活的成员连接可发送心跳".to_string()));
        }
        let cred = self.load_credential(link.up_credential_id).await?;
        let secret = self.decrypt_secret(&cred.secret_enc)?;
        hub_post_json(
            &link.peer_base_url,
            "/api/hub/v1/heartbeat",
            &cred.access_key,
            &secret,
            &json!({
                "instanceUuid": settings.instance_uuid,
                "version": INSTANCE_VERSION,
                "modules": settings.module_bits(),
            }),
        )
        .await?;
        Ok(())
    }

    // ———————————————————— 心跳与地址报备（被调用侧） ————————————————————

    /// 接收心跳：更新连接注册表（最后心跳时间 + 负载快照）。
    pub async fn receive_heartbeat(
        &self,
        cred: &HubCredential,
        dto: &HeartbeatDTO,
    ) -> Result<HeartbeatVO> {
        if cred.link_id <= 0 {
            return Err(Error::Validation("心跳凭证未绑定连接".to_string()));
        }
        let link = self.load_link(cred.link_id).await?;
        if !link.peer_instance_uuid.is_empty()
            && dto.instance_uuid.trim() != link.peer_instance_uuid
        {
            return Err(Error::Forbidden("心跳实例身份与连接不一致".to_string()));
        }
        let payload = serde_json::to_string(dto).unwrap_or_default();
        let now = self.now();
        self.repo().update_link_heartbeat(link.link_id, &payload, now).await?;
        Ok(HeartbeatVO {
            server_time: i64::from(now),
            link_status: i32::from(link.status),
            has_config_package: false,
        })
    }

    /// 地址变更报备：用下行钥回拨新地址验明实例身份后才切换。
    pub async fn update_endpoint(
        &self,
        cred: &HubCredential,
        dto: &UpdateEndpointDTO,
    ) -> Result<()> {
        if cred.link_id <= 0 {
            return Err(Error::Validation("凭证未绑定连接".to_string()));
        }
        let link = self.load_link(cred.link_id).await?;
        if dto.instance_uuid.trim() != link.peer_instance_uuid {
            return Err(Error::Forbidden("实例身份与连接不一致".to_string()));
        }
        let new_url = validate_base_url(&dto.new_base_url).map_err(Error::Validation)?;
        let down = self.load_credential(link.down_credential_id).await?;
        let secret = self.decrypt_secret(&down.secret_enc)?;
        let meta = hub_get_json(&new_url, "/api/hub/v1/meta", &down.access_key, &secret).await?;
        let remote_uuid = meta
            .get("instanceUuid")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if remote_uuid != link.peer_instance_uuid {
            return Err(Error::Forbidden("新地址实例身份不一致，拒绝切换".to_string()));
        }
        self.repo()
            .update_link_peer_url(link.link_id, &new_url, self.now())
            .await
    }

    // ———————————————————— 商品上行聚合（设计 §5） ————————————————————

    pub async fn list_aggregation(
        &self,
        q: &HubAggregationListQuery,
    ) -> Result<PageResult<HubAggregationVO>> {
        let (rows, total) = self.repo().list_product_maps(q).await?;
        let list = rows
            .iter()
            .map(|m| HubAggregationVO {
                map_id: m.map_id,
                link_id: m.link_id,
                credential_id: m.credential_id,
                remote_instance_uuid: m.remote_instance_uuid.clone(),
                remote_shop_id: m.remote_shop_id,
                remote_shop_name: m.remote_shop_name.clone(),
                remote_product_id: m.remote_product_id,
                hub_product_id: m.hub_product_id,
                product_name: m.product_name.clone(),
                pic_url: m.pic_url.clone(),
                price: m.price,
                market_price: m.market_price,
                stock: m.stock,
                content_hash: m.content_hash.clone(),
                sync_state: i32::from(m.sync_state),
                last_sync_time: m.last_sync_time,
            })
            .collect();
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        Ok(PageResult::new(total.max(0) as u64, list, &page))
    }

    pub async fn set_map_state(&self, map_id: i32, sync_state: i32) -> Result<()> {
        self.gate().await?;
        if !(1..=4).contains(&sync_state) {
            return Err(Error::Validation("同步状态非法".to_string()));
        }
        self.repo().set_map_state(map_id, sync_state, self.now()).await
    }

    /// 执行一次聚合拉取：meta 验身份 → 分页拉列表 → 变更项拉详情快照 →
    /// upsert 映射 → 未见商品置「源不可见」。来源标记取自对端实例/店铺。
    pub async fn sync_aggregation(&self, link_id: i32, credential_id: i32) -> Result<Value> {
        self.gate().await?;
        let (eff_link_id, cred, base_url) = if link_id > 0 {
            let link = self.load_link(link_id).await?;
            if i32::from(link.role) != LINK_ROLE_HUB || i32::from(link.status) != LINK_ACTIVE {
                return Err(Error::Validation("仅已激活的中枢侧连接可同步".to_string()));
            }
            let cred = self.load_credential(link.down_credential_id).await?;
            (link_id, cred, link.peer_base_url.clone())
        } else if credential_id > 0 {
            let cred = self.load_credential(credential_id).await?;
            if i32::from(cred.direction) != DIRECTION_OUTBOUND {
                return Err(Error::Validation("仅出站凭证（单向接入）可同步".to_string()));
            }
            let base_url = cred.peer_base_url.clone();
            (0, cred, base_url)
        } else {
            return Err(Error::Validation("缺少 linkId 或 credentialId".to_string()));
        };
        let eff_cred_id = cred.credential_id;
        let secret = self.decrypt_secret(&cred.secret_enc)?;
        let meta = hub_get_json(&base_url, "/api/hub/v1/meta", &cred.access_key, &secret).await?;
        let remote_uuid = meta
            .get("instanceUuid")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let remote_name = meta
            .get("instanceName")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let now = self.now();
        let mut seen: Vec<i32> = Vec::new();
        let (mut created, mut updated, mut unchanged, mut details) = (0u64, 0u64, 0u64, 0usize);
        let mut page_no = 1u64;
        loop {
            let path = format!(
                "/api/hub/v1/products?page={page_no}&size={SYNC_PAGE_SIZE}"
            );
            let data = hub_get_json(&base_url, &path, &cred.access_key, &secret).await?;
            let items = data
                .get("list")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if items.is_empty() {
                break;
            }
            for item in &items {
                let remote_pid = item.get("productId").and_then(Value::as_i64).unwrap_or(0) as i32;
                if remote_pid <= 0 {
                    continue;
                }
                seen.push(remote_pid);
                let hash = item
                    .get("contentHash")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let existing = self
                    .repo()
                    .find_map_by_key(eff_link_id, eff_cred_id, remote_pid)
                    .await?;
                let changed = match &existing {
                    None => true,
                    Some(m) => m.content_hash != hash || m.sync_state != 1,
                };
                let snapshot = if changed && details < SYNC_MAX_DETAILS {
                    details += 1;
                    let detail_path =
                        format!("/api/hub/v1/products/detail?productId={remote_pid}");
                    match hub_get_json(&base_url, &detail_path, &cred.access_key, &secret).await {
                        Ok(d) => d.to_string(),
                        Err(_) => item.to_string(),
                    }
                } else if let Some(m) = &existing {
                    m.snapshot.clone()
                } else {
                    item.to_string()
                };
                self.repo()
                    .upsert_product_map(
                        eff_link_id,
                        eff_cred_id,
                        &remote_uuid,
                        item.get("shopId").and_then(Value::as_i64).unwrap_or(0) as i32,
                        item.get("shopName")
                            .and_then(Value::as_str)
                            .unwrap_or(&remote_name),
                        remote_pid,
                        item.get("productName")
                            .and_then(Value::as_str)
                            .unwrap_or_default(),
                        item.get("picUrl").and_then(Value::as_str).unwrap_or_default(),
                        decimal_of(item.get("price")),
                        decimal_of(item.get("marketPrice")),
                        item.get("stock").and_then(Value::as_i64).unwrap_or(0) as i32,
                        &hash,
                        &snapshot,
                        now,
                    )
                    .await?;
                match &existing {
                    None => created += 1,
                    Some(_) if changed => updated += 1,
                    Some(_) => unchanged += 1,
                }
            }
            if (items.len() as u64) < SYNC_PAGE_SIZE || page_no >= SYNC_MAX_PAGES {
                break;
            }
            page_no += 1;
        }
        self.repo()
            .mark_map_missing(eff_link_id, eff_cred_id, &seen, now)
            .await?;
        tracing::info!(
            "联邦聚合同步完成: link={eff_link_id}, cred={eff_cred_id}, fetched={}, created={created}, updated={updated}",
            seen.len()
        );
        Ok(json!({
            "fetched": seen.len(),
            "created": created,
            "updated": updated,
            "unchanged": unchanged,
            "remoteInstanceUuid": remote_uuid,
            "remoteInstanceName": remote_name,
        }))
    }

    // ———————————————————— 出站钥加解密（AES-GCM，主密钥走 env） ————————————————————

    fn encrypt_secret(&self, plain: &str) -> Result<String> {
        if plain.is_empty() {
            return Ok(String::new());
        }
        let key = fed_master_key()?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| Error::Internal(format!("AES-GCM 初始化失败: {e}")))?;
        let nonce_hex = utils::random_hex(24);
        let mut nonce_bytes = [0u8; 12];
        for (i, b) in nonce_bytes.iter_mut().enumerate() {
            *b = u8::from_str_radix(&nonce_hex[i * 2..i * 2 + 2], 16).unwrap_or(0);
        }
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), plain.as_bytes())
            .map_err(|_| Error::Internal("出站凭证加密失败".to_string()))?;
        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);
        Ok(base64::engine::general_purpose::STANDARD.encode(combined))
    }

    fn decrypt_secret(&self, enc: &str) -> Result<String> {
        if enc.is_empty() {
            return Ok(String::new());
        }
        let key = fed_master_key()?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| Error::Internal(format!("AES-GCM 初始化失败: {e}")))?;
        let combined = base64::engine::general_purpose::STANDARD
            .decode(enc)
            .map_err(|_| Error::Internal("出站凭证密文格式非法".to_string()))?;
        if combined.len() <= 12 {
            return Err(Error::Internal("出站凭证密文过短".to_string()));
        }
        let (nonce, ciphertext) = combined.split_at(12);
        let plain = cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|_| Error::Internal("出站凭证解密失败（主密钥不匹配）".to_string()))?;
        String::from_utf8(plain).map_err(|_| Error::Internal("出站凭证明文非 UTF-8".to_string()))
    }
}

// ———————————————————— 自由函数 ————————————————————

/// 生成 accessKey：`hk_` + 20 位 hex。
fn gen_access_key() -> String {
    format!("hk_{}", utils::random_hex(20))
}

/// 生成 secret：48 位 hex（192 bit）。
fn gen_secret() -> String {
    utils::random_hex(48)
}

/// 生成 claim token：48 位 hex。
fn gen_claim_token() -> String {
    utils::random_hex(48)
}

/// 生成配对码：24 位 hex，分组展示 `XXXXXX-XXXXXX-XXXXXX-XXXXXX`。
fn gen_pairing_code() -> String {
    let raw = utils::random_hex(24).to_uppercase();
    raw.as_bytes()
        .chunks(6)
        .map(|c| std::str::from_utf8(c).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("-")
}

/// 配对码归一化（去分隔符/空白、转大写）后取哈希——展示形态不影响校验。
fn normalize_pairing_code(code: &str) -> String {
    code.chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect::<String>()
        .to_uppercase()
}

/// 恒定时间比较（哈希串等长比较，防时序侧信道）。
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

/// 由批准模块推导上行钥 scopes（设计 §4.3 上行集合与模块取交集）。
fn up_scopes_for_modules(modules: i64) -> i64 {
    let mut scopes = SCOPE_META_READ | SCOPE_STATS_REPORT | SCOPE_CONFIG_PULL;
    if modules & MODULE_PRODUCT != 0 {
        scopes |= SCOPE_PRODUCT_PUSH;
    }
    if modules & MODULE_ORDER != 0 {
        scopes |= SCOPE_ORDER_REPORT;
    }
    if modules & MODULE_MEMBER != 0 {
        scopes |= SCOPE_MEMBER_SYNC | SCOPE_MEMBER_AUTH;
    }
    scopes
}

/// 连接行 → VO（带心跳在线状态）。
fn link_to_vo(link: &HubLink, now: i64) -> HubLinkVO {
    HubLinkVO {
        link_id: link.link_id,
        role: i32::from(link.role),
        peer_instance_uuid: link.peer_instance_uuid.clone(),
        peer_name: link.peer_name.clone(),
        peer_base_url: link.peer_base_url.clone(),
        peer_version: link.peer_version.clone(),
        status: i32::from(link.status),
        heartbeat_state: heartbeat_state(link.last_heartbeat_time, now).to_string(),
        approved_modules: link.approved_modules,
        up_credential_id: link.up_credential_id,
        down_credential_id: link.down_credential_id,
        partner_user_id: link.partner_user_id,
        settle_mode: link.settle_mode.clone(),
        settle_cycle: link.settle_cycle.clone(),
        fund_channel: link.fund_channel.clone(),
        rate_source: link.rate_source.clone(),
        rate_override: link.rate_override,
        reject_reason: link.reject_reason.clone(),
        last_heartbeat_time: link.last_heartbeat_time,
        claim_expire_time: link.claim_expire_time,
        add_time: link.add_time,
        update_time: link.update_time,
    }
}

/// JSON 数值/字符串 → Decimal（聚合价格解析容错）。
fn decimal_of(v: Option<&Value>) -> Decimal {
    match v {
        Some(Value::Number(n)) => n
            .to_string()
            .parse::<Decimal>()
            .unwrap_or(Decimal::ZERO),
        Some(Value::String(s)) => s.parse::<Decimal>().unwrap_or(Decimal::ZERO),
        _ => Decimal::ZERO,
    }
}

/// 出站调用主密钥：env `NEXTTRADE_FED_SECRET_KEY`（32 字节，hex 64 位或 base64）。
fn fed_master_key() -> Result<[u8; 32]> {
    let raw = std::env::var("NEXTTRADE_FED_SECRET_KEY").map_err(|_| {
        Error::Internal(
            "未配置 NEXTTRADE_FED_SECRET_KEY（出站凭证加密主密钥），无法处理出站凭证".to_string(),
        )
    })?;
    let raw = raw.trim();
    if raw.len() == 64 && raw.bytes().all(|b| b.is_ascii_hexdigit()) {
        let mut key = [0u8; 32];
        for (i, b) in key.iter_mut().enumerate() {
            *b = u8::from_str_radix(&raw[i * 2..i * 2 + 2], 16).unwrap_or(0);
        }
        return Ok(key);
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|_| Error::Internal("NEXTTRADE_FED_SECRET_KEY 格式非法".to_string()))?;
    if decoded.len() != 32 {
        return Err(Error::Internal(
            "NEXTTRADE_FED_SECRET_KEY 必须为 32 字节".to_string(),
        ));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&decoded);
    Ok(key)
}

/// 对端联邦面 GET（带钥头），解开统一信封回 `data`。
async fn hub_get_json(base: &str, path: &str, access_key: &str, secret: &str) -> Result<Value> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| Error::Internal(format!("HTTP 客户端初始化失败: {e}")))?;
    let resp = client
        .get(format!("{}{}", base.trim_end_matches('/'), path))
        .header("X-Fed-Access-Key", access_key)
        .header("X-Fed-Secret", secret)
        .send()
        .await
        .map_err(|e| Error::Validation(format!("连接对端失败: {e}")))?;
    parse_hub_response(resp).await
}

/// 对端联邦面 POST（带钥头），解开统一信封回 `data`。
async fn hub_post_json(
    base: &str,
    path: &str,
    access_key: &str,
    secret: &str,
    body: &Value,
) -> Result<Value> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| Error::Internal(format!("HTTP 客户端初始化失败: {e}")))?;
    let resp = client
        .post(format!("{}{}", base.trim_end_matches('/'), path))
        .header("X-Fed-Access-Key", access_key)
        .header("X-Fed-Secret", secret)
        .json(body)
        .send()
        .await
        .map_err(|e| Error::Validation(format!("连接对端失败: {e}")))?;
    parse_hub_response(resp).await
}

/// 统一信封解析：HTTP 非 2xx 或业务 code 非 0 → 错误（带对端 msg）。
async fn parse_hub_response(resp: reqwest::Response) -> Result<Value> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| Error::Internal(format!("读取对端响应失败: {e}")))?;
    let parsed: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if !status.is_success() {
        let msg = parsed
            .get("msg")
            .and_then(Value::as_str)
            .unwrap_or("对端返回错误");
        return Err(Error::Validation(format!("对端拒绝（HTTP {status}）：{msg}")));
    }
    if let Some(code) = parsed.get("code").and_then(Value::as_i64) {
        if code != 0 {
            let msg = parsed
                .get("msg")
                .and_then(Value::as_str)
                .unwrap_or("对端业务错误");
            return Err(Error::Validation(format!("对端拒绝：{msg}")));
        }
    }
    Ok(parsed.get("data").cloned().unwrap_or(Value::Null))
}

#[cfg(test)]
mod tests {
    /// 截取某个 `pub async fn` 的函数体（到下一个同级函数签名为止），
    /// 结构断言一律限定在函数体内，避免被文件其它位置的同名串误命中
    /// （含本测试模块自己的断言字面量——它们都在被截函数体之后）。
    fn fn_body<'a>(src: &'a str, sig: &str) -> &'a str {
        let start = src.find(sig).unwrap_or_else(|| panic!("应存在函数 {sig}"));
        let rest = &src[start + sig.len()..];
        let end = rest
            .find("\n    pub async fn ")
            .or_else(|| rest.find("\n    async fn "))
            .unwrap_or(rest.len());
        &rest[..end]
    }

    /// 回环双写修复的模式口径：同一把钥的入站行（签发方）与出站行（持有方）
    /// 必须能在同库共存——唯一索引是 (access_key, direction) 复合，
    /// 而非单列 access_key；存量迁移按名拆旧建新。
    #[test]
    fn credential_unique_index_is_key_plus_direction() {
        let migration = include_str!(
            "../../../../deploy/pg/2026-11-21_hub_credential_access_key_dir.sql"
        );
        assert!(
            migration.contains("DROP INDEX IF EXISTS uk_hub_credential_access_key"),
            "存量迁移应先拆旧单列唯一索引"
        );
        assert!(
            migration.contains("ON hub_credential (access_key, direction)"),
            "存量迁移应建 (access_key, direction) 复合唯一索引"
        );
        for create_sql in [
            include_str!("../../../../deploy/pg/2026-11-20_hub_federation.sql"),
            include_str!("../../../../migrations/2026-11-20_hub_federation_tables.sql"),
        ] {
            assert!(
                create_sql.contains("ON hub_credential (access_key, direction)"),
                "新建表 SQL 应直接建复合唯一索引"
            );
            assert!(
                !create_sql.contains("ON hub_credential (access_key);"),
                "新建表 SQL 不应再有单列唯一索引"
            );
        }
    }

    /// 鉴权必须定向取入站行：回环时同一 access_key 有入站/出站两行，
    /// 按钥裸查取行不确定，出站行（secret_enc、无哈希）绝不能进鉴权。
    #[test]
    fn authenticate_resolves_inbound_twin_only() {
        let src = include_str!("federation.rs");
        let body = fn_body(src, "pub async fn authenticate");
        assert!(
            body.contains("find_inbound_credential_by_access_key("),
            "authenticate 应走入站定向查找"
        );
        let repo_src = include_str!("../../../infrastructure/src/db/repo/hub/federation.rs");
        let finder = fn_body(repo_src, "pub async fn find_inbound_credential_by_access_key");
        assert!(
            finder.contains("direction = 1"),
            "入站定向查找的 SQL 必须带 direction = 1 条件"
        );
    }

    /// poll 复用不变量：已落库的上行钥复用检查必须排在一切远程调用之前
    /// （中枢 token 已烧毁，远程状态查询必失败），且批准模块先于领取落本地。
    #[test]
    fn poll_link_reuses_stored_up_credential_before_remote_calls() {
        let src = include_str!("federation.rs");
        let body = fn_body(src, "pub async fn poll_link");
        let reuse = body
            .find("find_link_credential(")
            .expect("poll_link 应有按连接查已落库出站行的复用检查");
        let status_call = body
            .find("\"/api/hub/v1/pair/status\"")
            .expect("poll_link 应调 pair/status");
        assert!(
            reuse < status_call,
            "复用检查必须在 pair/status 远程调用之前"
        );
        let persist_modules = body
            .find("update_link_approved_modules(")
            .expect("领取前应先把批准模块落本地");
        let activate_call = body
            .find("\"/api/hub/v1/pair/activate\"")
            .expect("poll_link 应调 pair/activate");
        assert!(
            persist_modules < activate_call,
            "批准模块落库必须在 pair/activate 领取之前"
        );
        assert!(
            body.contains("find_credential_by_key_dir_link("),
            "落钥前应按（钥, 方向, 连接）做复用守卫"
        );
    }

    /// 孤儿补偿不变量：配对三处落行流程失败时必须补偿删除已落的行——
    /// claim/initiate 删凭证+连接；activate 先归零连接指针再删上行钥。
    #[test]
    fn pairing_flows_compensate_orphans_on_failure() {
        let src = include_str!("federation.rs");
        let claim = fn_body(src, "pub async fn pair_claim");
        assert!(
            claim.contains("delete_credential(") && claim.contains("delete_link("),
            "pair_claim 失败须补偿删除已落的出站凭证与连接行"
        );
        let initiate = fn_body(src, "pub async fn initiate_pairing");
        assert!(
            initiate.contains("delete_credential(") && initiate.contains("delete_link("),
            "initiate_pairing 失败须补偿删除已落的下行钥与本地连接行"
        );
        let activate = fn_body(src, "pub async fn pair_activate");
        assert!(
            activate.contains("delete_credential("),
            "pair_activate 回填失败须删除已落的上行钥"
        );
        assert!(
            activate.contains("update_link_credentials(link.link_id, 0"),
            "pair_activate 补偿须先把连接的上行指针归零再删钥"
        );
    }
}
