//! 平台联邦 P1 服务（设计 `~/workspace/hub-federation-design/proposal.md`）。
//!
//! 来源标注：**我方新增**——原版 tigshop 无实例联邦概念。本服务实现：
//! 开关体系（§2，config 表 `federationSettings` 单行 JSON，口径同 systemMode）、
//! 通用凭证（§4，新表族 hub_credential*，不改 feed 旧表；商品范围/价格规则
//! 复用 product_feed 的 FeedAudience 纯口径）、三档连接与配对（§3）、
//! 商品上行聚合（§5，拉取为主）、心跳与注册表（§8）。
//!
//! 二期（迁移 028）续补「同步核」：会员同步（§6.1/§6.2/§6.4：全局 ID 映射、
//! 手机号匹配与冲突队列、资料快照后写优先、锚点核对）与订单头级上行
//! （§7.1：幂等 + 版本守卫）。推送类端点按 §9.3 加 HMAC 签名：签名密钥为
//! secret 的 SHA-256 摘要（中枢侧只存哈希、永不存明文，故以摘要为共享密钥，
//! 与设计「hmac(secret, …)」等价强度）。
//!
//! 三期（迁移 042 起，设计 `~/workspace/federation-phase3/DESIGN.md`）续补：
//! 登录互认（§6.3 授权码模式，042）、配置分发（§8.3，043）、跨店下单
//! （§7.2，044）、中央账本（§6.5，045）、跨站结算（§11，046）。账本与
//! 结算只记账与生成结算单，不做真实跨站资金划拨（资金红线）。
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
use hmac::{Hmac, Mac};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

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
            // 二期计数（迁移 028 未执行时回落 0，不拖垮概览）。
            "membersLinked": repo.count_member_links().await.unwrap_or(0),
            "memberConflictsPending": repo.count_pending_conflicts().await.unwrap_or(0),
            "ordersReported": repo.sum_order_summaries(None).await.map(|t| t.0).unwrap_or(0),
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
    ) -> Result<PageResult<HubCallLogVO>> {
        let (rows, total) = self.repo().list_call_logs(q).await?;
        // 行结构是 snake_case 的 DB 形态，一律经 VO 转 camelCase 出 API
        //（全仓 VO 口径统一；前端表格按 camelCase 取数）。
        let list: Vec<HubCallLogVO> = rows.iter().map(HubCallLogVO::from).collect();
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        Ok(PageResult::new(total.max(0) as u64, list, &page))
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

    /// 删除连接（双向凭证删除、已同步数据不自动删、调用日志保留）。
    /// 聚合映射口径变更（2026-11-21 验收观察项后定，偏离设计 §3.5 原口径）：
    /// §3.5 原定映射「置已断开（sync_state=4）保留」，但连接行删除后映射成
    /// 无主孤儿（验收实测 API 删连接后残 63 行、靠人工清扫）；改为随连接
    /// 一并删除——repo delete_link 在同一事务内先删映射、再删连接本体。
    pub async fn delete_link(&self, link_id: i32) -> Result<()> {
        self.gate().await?;
        let link = self.load_link(link_id).await?;
        if link.up_credential_id > 0 {
            self.repo().delete_credential(link.up_credential_id).await?;
        }
        if link.down_credential_id > 0 {
            self.repo()
                .delete_credential(link.down_credential_id)
                .await?;
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
        // 三期②点亮：该连接有高于其回执水位的生效配置包时为 true。
        let has_config_package = self
            .repo()
            .has_unacked_config_package(link.link_id, now)
            .await
            .unwrap_or(false);
        Ok(HeartbeatVO {
            server_time: i64::from(now),
            link_status: i32::from(link.status),
            has_config_package,
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

    // ———————————————————— 二期：模块闸与推送签名（设计 §2.3/§9.3） ————————————————————

    /// 管理端模块闸：模块开关关 ⇒ 403（联邦面由 authenticate 的模块交集把关）。
    async fn gate_module(&self, module: i64, msg: &str) -> Result<FederationSettings> {
        let settings = self.gate().await?;
        if settings.module_bits() & module == 0 {
            return Err(Error::Forbidden(msg.to_string()));
        }
        Ok(settings)
    }

    /// 校验推送类端点的 HMAC 签名（路由层在 authenticate 通过后调用）。
    /// 签名 = HMAC-SHA256(key = SHA-256(secret) 摘要, "{timestamp}.{body}")；
    /// 中枢侧只存 secret_hash（即摘要的 hex），解码即同一密钥。
    ///
    /// # Errors
    /// 时间戳超 ±5 分钟窗口 / 签名不符 → Forbidden 统一文案。
    pub fn verify_push_signature(
        &self,
        cred: &HubCredential,
        timestamp: i64,
        signature: &str,
        body: &str,
    ) -> Result<()> {
        verify_signature_with_hash(&cred.secret_hash, timestamp, signature, body)
    }

    /// 手机号密文（落映射/冲突快照用）：主密钥未配置时回落空串（展示全掩），
    /// 不因加密不可用拖垮整批同步——哈希匹配面不受影响。
    fn encrypt_mobile(&self, mobile: &str) -> String {
        if mobile.is_empty() {
            return String::new();
        }
        self.encrypt_secret(mobile).unwrap_or_default()
    }

    /// 映射行手机号脱敏展示（解密失败/无密文 → 全掩）。
    fn masked_mobile_of(&self, mobile_enc: &str) -> String {
        if mobile_enc.is_empty() {
            return "****".to_string();
        }
        match self.decrypt_secret(mobile_enc) {
            Ok(m) if !m.is_empty() => mask_mobile(&m),
            _ => "****".to_string(),
        }
    }

    // ———————————————————— 二期：会员同步（设计 §6.1/§6.2/§6.4） ————————————————————

    /// 中枢侧接收一批会员同步事件（member.sync 已由路由鉴权 + 验签）。
    pub async fn receive_member_sync(
        &self,
        cred: &HubCredential,
        dto: &MemberSyncDTO,
    ) -> Result<MemberSyncResultVO> {
        if cred.link_id <= 0 {
            return Err(Error::Validation("会员同步凭证未绑定连接".to_string()));
        }
        let link = self.load_link(cred.link_id).await?;
        let now = self.now();
        let mut results = Vec::with_capacity(dto.events.len());
        for ev in &dto.events {
            results.push(self.apply_member_event(&link, ev, now).await?);
        }
        Ok(MemberSyncResultVO {
            results,
            server_time: i64::from(now),
        })
    }

    /// 单条会员事件落账：已有映射走快照后写优先；新面孔走手机号匹配裁决。
    async fn apply_member_event(
        &self,
        link: &HubLink,
        ev: &MemberSyncEvent,
        now: i32,
    ) -> Result<MemberSyncItemVO> {
        let remote = ev.remote_member_id;
        let item = |gid: &str, outcome: &str| MemberSyncItemVO {
            remote_member_id: remote,
            global_member_id: gid.to_string(),
            outcome: outcome.to_string(),
        };
        if remote <= 0 {
            return Ok(item("", "skipped"));
        }
        let link_id = link.link_id;
        if ev.event_type.trim() == "unbind" {
            return match self.repo().find_member_link_by_remote(link_id, remote).await? {
                Some(row) => {
                    self.repo()
                        .set_member_link_status(link_id, remote, MEMBER_LINK_UNBOUND, now)
                        .await?;
                    Ok(item(&row.global_member_id, "unbound"))
                }
                None => Ok(item("", "skipped")),
            };
        }
        let mobile = ev.mobile.trim().to_string();
        let stamp = i32::try_from(ev.profile_updated_at).unwrap_or(i32::MAX);
        if let Some(row) = self.repo().find_member_link_by_remote(link_id, remote).await? {
            // 换号且新号已被别的全局 ID 占用 → 身份类变更不自动覆盖，进冲突队列。
            if !mobile.is_empty() {
                let new_hash = utils::sha256_hex(&mobile);
                if new_hash != row.mobile_hash {
                    let others = self.repo().list_member_links_by_mobile_hash(&new_hash).await?;
                    if let Some(other) = others
                        .iter()
                        .find(|r| r.global_member_id != row.global_member_id)
                    {
                        let gid = other.global_member_id.clone();
                        self.enqueue_conflict(link_id, ev, &new_hash, CONFLICT_MOBILE_CLAIMED, &gid, now)
                            .await?;
                        return Ok(item("", "conflict"));
                    }
                }
            }
            if stamp >= row.profile_updated_at {
                let (hash, enc) = if mobile.is_empty() {
                    (row.mobile_hash.clone(), row.mobile_enc.clone())
                } else {
                    (utils::sha256_hex(&mobile), self.encrypt_mobile(&mobile))
                };
                self.repo()
                    .update_member_link_snapshot(
                        row.id,
                        &hash,
                        &enc,
                        ev.nickname.trim(),
                        ev.avatar.trim(),
                        ev.birthday.trim(),
                        ev.email.trim(),
                        ev.real_name.trim(),
                        stamp,
                        now,
                    )
                    .await?;
            }
            return Ok(item(&row.global_member_id, "linked"));
        }
        // 新面孔：只有已验证手机号能进自动匹配（设计 §6.2）。
        if mobile.is_empty() {
            return Ok(item("", "skipped"));
        }
        let hash = utils::sha256_hex(&mobile);
        if !ev.mobile_validated {
            self.enqueue_conflict(link_id, ev, &hash, CONFLICT_UNVERIFIED_MOBILE, "", now)
                .await?;
            return Ok(item("", "conflict"));
        }
        let rows = self.repo().list_member_links_by_mobile_hash(&hash).await?;
        let match_rows: Vec<MobileMatchRow> = rows
            .iter()
            .map(|r| MobileMatchRow {
                global_member_id: r.global_member_id.clone(),
                link_id: r.link_id,
                remote_member_id: r.remote_member_id,
                real_name: r.real_name.clone(),
            })
            .collect();
        match decide_member_match(&match_rows, link_id, ev.real_name.trim()) {
            MemberMatchDecision::CreateNew => {
                let gid = format_global_member_id(&uuid::Uuid::new_v4().to_string());
                self.insert_member_link_from_event(&gid, link_id, ev, &hash, stamp, now)
                    .await?;
                Ok(item(&gid, "created"))
            }
            MemberMatchDecision::LinkTo(gid) => {
                self.insert_member_link_from_event(&gid, link_id, ev, &hash, stamp, now)
                    .await?;
                Ok(item(&gid, "linked"))
            }
            MemberMatchDecision::Conflict(kind, candidate) => {
                self.enqueue_conflict(link_id, ev, &hash, kind, &candidate, now)
                    .await?;
                Ok(item("", "conflict"))
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn insert_member_link_from_event(
        &self,
        gid: &str,
        link_id: i32,
        ev: &MemberSyncEvent,
        mobile_hash: &str,
        stamp: i32,
        now: i32,
    ) -> Result<()> {
        self.repo()
            .insert_member_link(
                gid,
                link_id,
                ev.remote_member_id,
                mobile_hash,
                &self.encrypt_mobile(ev.mobile.trim()),
                ev.nickname.trim(),
                ev.avatar.trim(),
                ev.birthday.trim(),
                ev.email.trim(),
                ev.real_name.trim(),
                stamp,
                now,
            )
            .await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn enqueue_conflict(
        &self,
        link_id: i32,
        ev: &MemberSyncEvent,
        mobile_hash: &str,
        conflict_type: &str,
        candidate: &str,
        now: i32,
    ) -> Result<()> {
        self.repo()
            .insert_member_conflict(
                link_id,
                ev.remote_member_id,
                mobile_hash,
                &self.encrypt_mobile(ev.mobile.trim()),
                ev.nickname.trim(),
                ev.avatar.trim(),
                ev.birthday.trim(),
                ev.email.trim(),
                ev.real_name.trim(),
                i32::try_from(ev.profile_updated_at).unwrap_or(i32::MAX),
                candidate,
                conflict_type,
                now,
            )
            .await
    }

    /// 成员侧应答中枢下钻（member.read）：按全局 ID（经本地映射）或手机号
    /// 回本地会员资料。查无此人 → 404（范围外与不存在不区分，设计 §9.4）。
    pub async fn hub_member_query(
        &self,
        cred: &HubCredential,
        global_member_id: &str,
        mobile: &str,
    ) -> Result<MemberQueryVO> {
        if cred.link_id <= 0 {
            return Err(Error::Validation("会员查询凭证未绑定连接".to_string()));
        }
        let gid = global_member_id.trim();
        let user = if !gid.is_empty() {
            let map = self
                .repo()
                .find_member_map_by_global(cred.link_id, gid)
                .await?
                .ok_or_else(|| Error::NotFound(MSG_MEMBER_NOT_FOUND.to_string()))?;
            self.repo()
                .find_local_member_by_id(map.local_user_id)
                .await?
        } else if !mobile.trim().is_empty() {
            self.repo().find_local_member_by_mobile(mobile.trim()).await?
        } else {
            return Err(Error::Validation("缺少 globalMemberId 或 mobile".to_string()));
        };
        let user = user.ok_or_else(|| Error::NotFound(MSG_MEMBER_NOT_FOUND.to_string()))?;
        let mapped_gid = self
            .repo()
            .find_member_map_by_user(cred.link_id, user.user_id)
            .await?
            .map(|m| m.global_member_id)
            .unwrap_or_else(|| gid.to_string());
        Ok(MemberQueryVO {
            global_member_id: mapped_gid,
            local_member_id: user.user_id,
            mobile_masked: mask_mobile(&user.mobile),
            nickname: user.nickname.clone().unwrap_or_default(),
            avatar: user.avatar.clone().unwrap_or_default(),
            birthday: user.birthday.clone().unwrap_or_default(),
            email: user.email.clone().unwrap_or_default(),
            account_status: user.status.unwrap_or(-1),
        })
    }

    /// 中枢侧应答成员锚点核对（member.sync 通道）：回该全局 ID 在调用方
    /// 连接名下的映射状态与资料版本戳（身份类以中枢为准，设计 §6.4）。
    pub async fn hub_member_anchor(
        &self,
        cred: &HubCredential,
        global_member_id: &str,
    ) -> Result<MemberAnchorVO> {
        if cred.link_id <= 0 {
            return Err(Error::Validation("锚点核对凭证未绑定连接".to_string()));
        }
        let gid = global_member_id.trim();
        let any = self.repo().find_member_link_by_global(gid).await?;
        let mine = self
            .repo()
            .find_member_link_by_link_global(cred.link_id, gid)
            .await?;
        Ok(MemberAnchorVO {
            global_member_id: gid.to_string(),
            exists: any.is_some(),
            link_status: mine
                .as_ref()
                .map(|r| i32::from(r.status))
                .unwrap_or(0),
            mobile_masked: mine
                .as_ref()
                .map(|r| self.masked_mobile_of(&r.mobile_enc))
                .unwrap_or_else(|| "****".to_string()),
            profile_updated_at: mine.map(|r| r.profile_updated_at).unwrap_or(0),
        })
    }

    /// 成员侧手动同步会员到中枢（自动调度同心跳口径留后续）：按 user_id
    /// 游标增量分批推送，回执的全局 ID 落本地映射 hub_member_map。
    pub async fn sync_members(&self, link_id: i32) -> Result<Value> {
        let settings = self.gate().await?;
        if !settings.modules.member {
            return Err(Error::Forbidden(MSG_MEMBER_MODULE_OFF.to_string()));
        }
        let link = self.load_link(link_id).await?;
        if i32::from(link.role) != LINK_ROLE_MEMBER || i32::from(link.status) != LINK_ACTIVE {
            return Err(Error::Validation("仅已激活的成员连接可同步会员".to_string()));
        }
        if link.approved_modules & MODULE_MEMBER == 0 {
            return Err(Error::Forbidden(MSG_MEMBER_MODULE_OFF.to_string()));
        }
        let cred = self.load_credential(link.up_credential_id).await?;
        let secret = self.decrypt_secret(&cred.secret_enc)?;
        let now = self.now();
        let mut cursor = link.member_sync_cursor;
        let (mut scanned, mut created, mut linked, mut conflict, mut skipped) =
            (0i64, 0i64, 0i64, 0i64, 0i64);
        for _ in 0..20 {
            let rows = self.repo().list_local_members_after(cursor, 100).await?;
            if rows.is_empty() {
                break;
            }
            let events: Vec<MemberSyncEvent> = rows
                .iter()
                .map(|u| MemberSyncEvent {
                    remote_member_id: u.user_id,
                    event_type: "upsert".to_string(),
                    mobile: u.mobile.clone(),
                    mobile_validated: u.mobile_validated == 1,
                    email: u.email.clone().unwrap_or_default(),
                    nickname: u.nickname.clone().unwrap_or_default(),
                    avatar: u.avatar.clone().unwrap_or_default(),
                    birthday: u.birthday.clone().unwrap_or_default(),
                    real_name: String::new(),
                    profile_updated_at: i64::from(u.reg_time.max(u.last_login)),
                })
                .collect();
            let resp = hub_post_json_signed(
                &link.peer_base_url,
                "/api/hub/v1/members/sync",
                &cred.access_key,
                &secret,
                &json!({ "events": events }),
            )
            .await?;
            let results = resp
                .get("results")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for (u, r) in rows.iter().zip(results.iter()) {
                let outcome = r.get("outcome").and_then(Value::as_str).unwrap_or("");
                let gid = r
                    .get("globalMemberId")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                match outcome {
                    "created" => created += 1,
                    "linked" => linked += 1,
                    "conflict" => conflict += 1,
                    _ => skipped += 1,
                }
                if !gid.is_empty() && matches!(outcome, "created" | "linked") {
                    self.repo()
                        .upsert_member_map(
                            link_id,
                            u.user_id,
                            gid,
                            u.reg_time.max(u.last_login),
                            now,
                        )
                        .await?;
                }
            }
            scanned += rows.len() as i64;
            cursor = rows.last().map(|u| u.user_id).unwrap_or(cursor);
            self.repo().update_link_member_cursor(link_id, cursor, now).await?;
            if rows.len() < 100 {
                break;
            }
        }
        tracing::info!(
            "联邦会员同步完成: link={link_id}, scanned={scanned}, created={created}, linked={linked}, conflict={conflict}"
        );
        Ok(json!({
            "scanned": scanned,
            "created": created,
            "linked": linked,
            "conflict": conflict,
            "skipped": skipped,
            "cursor": cursor,
        }))
    }

    /// 管理端：会员映射列表（中枢侧，会员模块闸）。
    pub async fn list_members(
        &self,
        q: &HubMemberListQuery,
    ) -> Result<PageResult<HubMemberLinkVO>> {
        self.gate_module(MODULE_MEMBER, MSG_MEMBER_MODULE_OFF).await?;
        let (rows, total) = self.repo().list_member_links(q).await?;
        let list = rows
            .iter()
            .map(|r| HubMemberLinkVO {
                id: r.id,
                global_member_id: r.global_member_id.clone(),
                link_id: r.link_id,
                remote_member_id: r.remote_member_id,
                mobile_masked: self.masked_mobile_of(&r.mobile_enc),
                nickname: r.nickname.clone(),
                avatar: r.avatar.clone(),
                birthday: r.birthday.clone(),
                email: r.email.clone(),
                profile_updated_at: r.profile_updated_at,
                status: i32::from(r.status),
                add_time: r.add_time,
                update_time: r.update_time,
            })
            .collect();
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        Ok(PageResult::new(total.max(0) as u64, list, &page))
    }

    /// 管理端：冲突队列列表。
    pub async fn list_member_conflicts(
        &self,
        q: &HubMemberConflictListQuery,
    ) -> Result<PageResult<HubMemberConflictVO>> {
        self.gate_module(MODULE_MEMBER, MSG_MEMBER_MODULE_OFF).await?;
        let (rows, total) = self.repo().list_member_conflicts(q).await?;
        let list = rows
            .iter()
            .map(|r| HubMemberConflictVO {
                id: r.id,
                link_id: r.link_id,
                remote_member_id: r.remote_member_id,
                mobile_masked: self.masked_mobile_of(&r.mobile_enc),
                nickname: r.nickname.clone(),
                email: r.email.clone(),
                candidate_global_member_id: r.candidate_global_member_id.clone(),
                conflict_type: r.conflict_type.clone(),
                status: i32::from(r.status),
                add_time: r.add_time,
                update_time: r.update_time,
            })
            .collect();
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        Ok(PageResult::new(total.max(0) as u64, list, &page))
    }

    /// 管理端：冲突裁决（link 关联候选 / separate 独立建号 / ignore 忽略）。
    /// 裁决落映射时复用冲突行内的上报快照，不回源再取。
    pub async fn resolve_member_conflict(&self, dto: &HubMemberResolveDTO) -> Result<()> {
        self.gate_module(MODULE_MEMBER, MSG_MEMBER_MODULE_OFF).await?;
        let conflict = self
            .repo()
            .find_member_conflict(dto.conflict_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_CONFLICT_NOT_FOUND.to_string()))?;
        if i32::from(conflict.status) != MEMBER_CONFLICT_PENDING {
            return Err(Error::Validation("该冲突已处理".to_string()));
        }
        let now = self.now();
        let (gid, resolved) = match dto.action.trim() {
            "link" => {
                if conflict.candidate_global_member_id.is_empty() {
                    return Err(Error::Validation(
                        "该冲突无候选全局会员，请改用独立建号".to_string(),
                    ));
                }
                (conflict.candidate_global_member_id.clone(), MEMBER_CONFLICT_LINKED)
            }
            "separate" => (
                format_global_member_id(&uuid::Uuid::new_v4().to_string()),
                MEMBER_CONFLICT_SEPARATE,
            ),
            "ignore" => (String::new(), MEMBER_CONFLICT_IGNORED),
            _ => return Err(Error::Validation("裁决动作非法".to_string())),
        };
        if !gid.is_empty()
            && self
                .repo()
                .find_member_link_by_remote(conflict.link_id, conflict.remote_member_id)
                .await?
                .is_none()
        {
            self.repo()
                .insert_member_link(
                    &gid,
                    conflict.link_id,
                    conflict.remote_member_id,
                    &conflict.mobile_hash,
                    &conflict.mobile_enc,
                    &conflict.nickname,
                    &conflict.avatar,
                    &conflict.birthday,
                    &conflict.email,
                    &conflict.real_name,
                    conflict.profile_updated_at,
                    now,
                )
                .await?;
        }
        self.repo()
            .update_member_conflict_status(conflict.id, resolved, now)
            .await
    }

    // ———————————————————— 二期：订单上行（设计 §7.1） ————————————————————

    /// 中枢侧接收一批订单头级汇总（order.report 已由路由鉴权 + 验签）。
    /// 幂等键（连接, 远端订单号）+ status_version 守卫：旧版本不覆盖新版本。
    pub async fn receive_order_report(
        &self,
        cred: &HubCredential,
        dto: &OrderReportDTO,
    ) -> Result<OrderReportResultVO> {
        if cred.link_id <= 0 {
            return Err(Error::Validation("订单上报凭证未绑定连接".to_string()));
        }
        let link = self.load_link(cred.link_id).await?;
        let now = self.now();
        let (mut created, mut updated, mut skipped) = (0i64, 0i64, 0i64);
        for item in &dto.orders {
            let sn = item.remote_order_sn.trim();
            if sn.is_empty() {
                skipped += 1;
                continue;
            }
            match self.repo().find_order_summary(cred.link_id, sn).await? {
                None => {
                    self.repo()
                        .upsert_order_summary(cred.link_id, &link.peer_instance_uuid, item, now)
                        .await?;
                    created += 1;
                }
                Some(old) if item.status_version >= old.status_version => {
                    self.repo()
                        .upsert_order_summary(cred.link_id, &link.peer_instance_uuid, item, now)
                        .await?;
                    updated += 1;
                }
                Some(_) => skipped += 1,
            }
        }
        Ok(OrderReportResultVO {
            created,
            updated,
            skipped,
            server_time: i64::from(now),
        })
    }

    /// 成员侧手动上报订单到中枢：新增按 order_id 游标增量、存量按
    /// 「版本戳 > 上次上报时间 - 1 天」重扫补状态推进，汇总计数回执。
    pub async fn report_orders(&self, link_id: i32) -> Result<Value> {
        let settings = self.gate().await?;
        if !settings.modules.order {
            return Err(Error::Forbidden(MSG_ORDER_MODULE_OFF.to_string()));
        }
        let link = self.load_link(link_id).await?;
        if i32::from(link.role) != LINK_ROLE_MEMBER || i32::from(link.status) != LINK_ACTIVE {
            return Err(Error::Validation("仅已激活的成员连接可上报订单".to_string()));
        }
        if link.approved_modules & MODULE_ORDER == 0 {
            return Err(Error::Forbidden(MSG_ORDER_MODULE_OFF.to_string()));
        }
        let cred = self.load_credential(link.up_credential_id).await?;
        let secret = self.decrypt_secret(&cred.secret_enc)?;
        let now = self.now();
        let mut cursor = link.order_sync_cursor;
        let (mut reported, mut created, mut updated, mut skipped) = (0i64, 0i64, 0i64, 0i64);
        for _ in 0..20 {
            let rows = self.repo().list_local_orders_after(cursor, 100).await?;
            if rows.is_empty() {
                break;
            }
            let items = self.order_items_for(link_id, &rows).await?;
            let resp = hub_post_json_signed(
                &link.peer_base_url,
                "/api/hub/v1/orders/report",
                &cred.access_key,
                &secret,
                &json!({ "orders": items }),
            )
            .await?;
            created += resp.get("created").and_then(Value::as_i64).unwrap_or(0);
            updated += resp.get("updated").and_then(Value::as_i64).unwrap_or(0);
            skipped += resp.get("skipped").and_then(Value::as_i64).unwrap_or(0);
            reported += rows.len() as i64;
            cursor = rows.last().map(|r| r.order_id).unwrap_or(cursor);
            if rows.len() < 100 {
                break;
            }
        }
        // 存量变更重扫：上次上报后 1 天窗口内有状态推进的老订单（版本守卫去重）。
        if link.order_sync_time > 0 {
            let changed = self
                .repo()
                .list_local_orders_changed_since(
                    link.order_sync_time - 86_400,
                    link.order_sync_cursor,
                    200,
                )
                .await?;
            if !changed.is_empty() {
                let items = self.order_items_for(link_id, &changed).await?;
                let resp = hub_post_json_signed(
                    &link.peer_base_url,
                    "/api/hub/v1/orders/report",
                    &cred.access_key,
                    &secret,
                    &json!({ "orders": items }),
                )
                .await?;
                created += resp.get("created").and_then(Value::as_i64).unwrap_or(0);
                updated += resp.get("updated").and_then(Value::as_i64).unwrap_or(0);
                skipped += resp.get("skipped").and_then(Value::as_i64).unwrap_or(0);
                reported += changed.len() as i64;
            }
        }
        self.repo()
            .update_link_order_cursor(link_id, cursor, now, now)
            .await?;
        tracing::info!(
            "联邦订单上报完成: link={link_id}, reported={reported}, created={created}, updated={updated}, skipped={skipped}"
        );
        Ok(json!({
            "reported": reported,
            "created": created,
            "updated": updated,
            "skipped": skipped,
            "cursor": cursor,
        }))
    }

    /// 本地订单扫描行 → 上行条目（买家全局 ID 经本地映射带上，未关联为空）。
    async fn order_items_for(
        &self,
        link_id: i32,
        rows: &[LocalOrderRow],
    ) -> Result<Vec<OrderReportItem>> {
        let mut items = Vec::with_capacity(rows.len());
        for r in rows {
            let gid = self
                .repo()
                .find_member_map_by_user(link_id, r.user_id)
                .await?
                .map(|m| m.global_member_id)
                .unwrap_or_default();
            items.push(OrderReportItem {
                remote_order_sn: r.order_sn.clone(),
                remote_shop_id: r.shop_id,
                remote_shop_name: r.shop_name.clone(),
                remote_user_id: r.user_id,
                global_member_id: gid,
                order_amount: r.total_amount,
                paid_amount: r.paid_amount,
                order_status: r.order_status,
                pay_status: r.pay_status,
                item_count: r.item_count,
                order_time: r.add_time,
                pay_time: r.pay_time,
                status_version: i64::from(r.version_time),
            });
        }
        Ok(items)
    }

    /// 管理端：订单上行汇总列表（中枢侧，订单模块闸）。
    pub async fn list_order_summaries(
        &self,
        q: &HubOrderSummaryListQuery,
    ) -> Result<PageResult<HubOrderSummaryVO>> {
        self.gate_module(MODULE_ORDER, MSG_ORDER_MODULE_OFF).await?;
        let (rows, total) = self.repo().list_order_summaries(q).await?;
        let list = rows.iter().map(HubOrderSummaryVO::from).collect();
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        Ok(PageResult::new(total.max(0) as u64, list, &page))
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

    // ———————————————————— 三期②：配置分发（迁移 043；DESIGN.md §2） ————————————————————
    //
    // 拉模式：中枢发布带版本包 → 成员拉取 → 下行钥派生签名 + 内容哈希双验
    // → 落快照 → 自动/手动应用 → 回执。应用语义边界（DESIGN §2.3）：
    // announcement 回写本地平台公告；其余类型只落快照并如实记录，不假装执行。

    /// 中枢侧：成员拉取配置包（路由层已过 config.pull 鉴权）。逐包以该连接
    /// 下行钥密钥派生签名（中枢解密 secret_enc 后取 SHA-256 摘要为 HMAC 钥；
    /// 成员以其入站下行钥的 secret_hash 解码同一摘要验签）。
    pub async fn config_packages(
        &self,
        cred: &HubCredential,
        since_version: i32,
    ) -> Result<ConfigPackagesVO> {
        if cred.link_id <= 0 {
            return Err(Error::Validation("配置拉取凭证未绑定连接".to_string()));
        }
        let link = self.load_link(cred.link_id).await?;
        if i32::from(link.role) != LINK_ROLE_HUB || i32::from(link.status) != LINK_ACTIVE {
            return Err(Error::Validation("仅已激活的中枢侧连接可拉取配置".to_string()));
        }
        let now = self.now();
        let rows = self
            .repo()
            .list_effective_config_packages(since_version.max(0), now, CONFIG_PULL_LIMIT)
            .await?;
        let down = self.load_credential(link.down_credential_id).await?;
        let secret = self.decrypt_secret(&down.secret_enc)?;
        let key = Sha256::digest(secret.as_bytes());
        let packages = rows
            .iter()
            .map(|p| ConfigPackageWireVO {
                package_id: p.id,
                package_type: p.package_type.clone(),
                title: p.title.clone(),
                version: p.version,
                payload: p.payload.clone(),
                content_hash: p.content_hash.clone(),
                apply_mode: i32::from(p.apply_mode),
                effective_from: p.effective_from,
                effective_to: p.effective_to,
                signature: config_package_signature(&key, p.id, p.version, &p.content_hash),
            })
            .collect();
        Ok(ConfigPackagesVO {
            packages,
            latest_version: self.repo().max_config_version().await?,
        })
    }

    /// 中枢侧：接收成员回执（路由层已过 config.pull 鉴权与签名）。
    pub async fn receive_config_ack(
        &self,
        cred: &HubCredential,
        dto: &ConfigAckDTO,
    ) -> Result<()> {
        if cred.link_id <= 0 {
            return Err(Error::Validation("配置回执凭证未绑定连接".to_string()));
        }
        if !(CONFIG_ACK_APPLIED..=CONFIG_ACK_FAILED).contains(&dto.status) {
            return Err(Error::Validation("回执状态不合法".to_string()));
        }
        let pkg = self
            .repo()
            .find_config_package(dto.package_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_CONFIG_PACKAGE_INVALID.to_string()))?;
        if pkg.version != dto.version {
            return Err(Error::Validation("回执版本与配置包不一致".to_string()));
        }
        self.repo()
            .upsert_config_ack(
                cred.link_id,
                dto.package_id,
                dto.version,
                dto.status,
                dto.message.trim(),
                self.now(),
            )
            .await
    }

    /// 管理端（中枢）：发布配置包（联邦能力闸）。version 取 max+1。
    pub async fn publish_config_package(
        &self,
        dto: &ConfigPublishDTO,
    ) -> Result<HubConfigPackageVO> {
        let settings = self.gate().await?;
        if !settings.capabilities.federation {
            return Err(Error::Forbidden(MSG_FEDERATION_DISABLED.to_string()));
        }
        let package_type = dto.package_type.trim();
        if !config_package_type_valid(package_type) {
            return Err(Error::Validation(MSG_CONFIG_TYPE_UNKNOWN.to_string()));
        }
        let title = dto.title.trim();
        if title.is_empty() {
            return Err(Error::Validation("配置包标题不能为空".to_string()));
        }
        let apply_mode = dto.apply_mode.unwrap_or(CONFIG_APPLY_MANUAL);
        if apply_mode != CONFIG_APPLY_MANUAL && apply_mode != CONFIG_APPLY_AUTO {
            return Err(Error::Validation("应用方式不合法".to_string()));
        }
        let payload = serde_json::to_string(&dto.payload)
            .map_err(|e| Error::Internal(format!("配置包内容序列化失败: {e}")))?;
        let content_hash = utils::sha256_hex(&payload);
        let now = self.now();
        let version = self.repo().max_config_version().await? + 1;
        let id = self
            .repo()
            .insert_config_package(
                package_type,
                title,
                version,
                &payload,
                &content_hash,
                apply_mode,
                dto.effective_from.unwrap_or(0),
                dto.effective_to.unwrap_or(0),
                now,
            )
            .await?;
        let row = self
            .repo()
            .find_config_package(id)
            .await?
            .ok_or_else(|| Error::Internal("配置包落库后回读失败".to_string()))?;
        Ok(HubConfigPackageVO::from(&row))
    }

    /// 管理端（中枢）：配置包列表（带回执覆盖统计）。
    pub async fn list_config_packages(
        &self,
        q: &HubConfigListQuery,
    ) -> Result<PageResult<HubConfigPackageVO>> {
        let settings = self.gate().await?;
        if !settings.capabilities.federation {
            return Err(Error::Forbidden(MSG_FEDERATION_DISABLED.to_string()));
        }
        let (rows, total) = self.repo().list_config_packages(q).await?;
        let mut list = Vec::with_capacity(rows.len());
        for r in &rows {
            let mut vo = HubConfigPackageVO::from(r);
            let (ack, applied) = self.repo().config_ack_counts(r.id).await?;
            vo.ack_count = ack;
            vo.applied_count = applied;
            list.push(vo);
        }
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        Ok(PageResult::new(total.max(0) as u64, list, &page))
    }

    /// 管理端（中枢）：撤销配置包（撤销后不再下发；已落地成员侧不受影响）。
    pub async fn revoke_config_package(&self, id: i32) -> Result<()> {
        let settings = self.gate().await?;
        if !settings.capabilities.federation {
            return Err(Error::Forbidden(MSG_FEDERATION_DISABLED.to_string()));
        }
        if !self.repo().revoke_config_package(id, self.now()).await? {
            return Err(Error::Validation("配置包不存在或已撤销".to_string()));
        }
        Ok(())
    }

    /// 管理端（中枢）：回执列表（覆盖率核查）。
    pub async fn list_config_acks(
        &self,
        q: &HubConfigAckListQuery,
    ) -> Result<PageResult<HubConfigAckVO>> {
        let settings = self.gate().await?;
        if !settings.capabilities.federation {
            return Err(Error::Forbidden(MSG_FEDERATION_DISABLED.to_string()));
        }
        let (rows, total) = self.repo().list_config_acks(q).await?;
        let mut list = Vec::with_capacity(rows.len());
        for r in &rows {
            let mut vo = HubConfigAckVO::from(r);
            if r.link_id > 0 {
                if let Ok(link) = self.load_link(r.link_id).await {
                    vo.peer_name = link.peer_name.clone();
                }
            }
            list.push(vo);
        }
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        Ok(PageResult::new(total.max(0) as u64, list, &page))
    }

    /// 成员侧：按连接拉取配置包并处理（管理端手动触发，口径同二期手动同步）。
    /// 逐包双验（content_hash + 下行钥派生签名）→ 落快照 → 自动包即时应用、
    /// 手动包待确认；终态包回执补发（upsert 幂等）。回处理计数摘要。
    pub async fn pull_config(&self, link_id: i32) -> Result<Value> {
        self.gate().await?;
        let link = self.load_link(link_id).await?;
        if i32::from(link.role) != LINK_ROLE_MEMBER || i32::from(link.status) != LINK_ACTIVE {
            return Err(Error::Validation("仅已激活的成员连接可拉取配置".to_string()));
        }
        let cred = self.load_credential(link.up_credential_id).await?;
        let secret = self.decrypt_secret(&cred.secret_enc)?;
        let down = self.load_credential(link.down_credential_id).await?;
        let verify_key = hex_decode(&down.secret_hash)
            .filter(|k| k.len() == 32)
            .ok_or_else(|| Error::Internal("入站下行钥缺少 secret 哈希，无法验签".to_string()))?;
        let since = self.repo().max_apply_version(link_id).await?;
        let resp = hub_get_json(
            &link.peer_base_url,
            &format!("/api/hub/v1/config/packages?sinceVersion={since}"),
            &cred.access_key,
            &secret,
        )
        .await?;
        let wires = resp
            .get("packages")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let now = self.now();
        let (mut applied, mut pending, mut failed, mut rejected) = (0i64, 0i64, 0i64, 0i64);
        for w in &wires {
            let package_id = w.get("packageId").and_then(Value::as_i64).unwrap_or(0) as i32;
            let version = w.get("version").and_then(Value::as_i64).unwrap_or(0) as i32;
            let payload = w.get("payload").and_then(Value::as_str).unwrap_or_default();
            let content_hash = w
                .get("contentHash")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let signature = w.get("signature").and_then(Value::as_str).unwrap_or_default();
            // 双验：内容哈希 + 下行钥派生签名，任一不过整包拒收（不落快照）。
            let hash_ok = utils::sha256_hex(payload) == content_hash;
            let expect = config_package_signature(&verify_key, package_id, version, content_hash);
            let sig_ok = constant_time_eq(
                expect.as_bytes(),
                signature.trim().to_ascii_lowercase().as_bytes(),
            );
            if !hash_ok || !sig_ok || package_id <= 0 {
                rejected += 1;
                tracing::warn!(
                    "联邦配置包验签失败已拒收: link={link_id}, package={package_id}, hash_ok={hash_ok}, sig_ok={sig_ok}"
                );
                continue;
            }
            let apply = self
                .repo()
                .upsert_config_apply(
                    link_id,
                    package_id,
                    version,
                    w.get("packageType").and_then(Value::as_str).unwrap_or("custom"),
                    w.get("title").and_then(Value::as_str).unwrap_or_default(),
                    payload,
                    content_hash,
                    w.get("applyMode").and_then(Value::as_i64).unwrap_or(1) as i32,
                    now,
                )
                .await?;
            if i32::from(apply.apply_status) != CONFIG_APPLY_PENDING {
                continue; // 已终态：不复活（回执补发在下方统一处理）
            }
            if i32::from(apply.apply_mode) == CONFIG_APPLY_AUTO {
                let (status, message) = self.apply_config_package(&apply).await;
                self.repo()
                    .set_config_apply_status(apply.id, status, &message, self.now())
                    .await?;
                let ack = if status == CONFIG_APPLY_APPLIED {
                    CONFIG_ACK_APPLIED
                } else {
                    CONFIG_ACK_FAILED
                };
                self.send_config_ack(&link, &apply, ack, &message).await;
                if status == CONFIG_APPLY_APPLIED {
                    applied += 1;
                } else {
                    failed += 1;
                }
            } else {
                pending += 1;
            }
        }
        // 回执补发：本连接全部终态落地包重发回执（中枢 upsert 幂等，防丢报）。
        let (terminal, _) = self
            .repo()
            .list_config_applies(&HubConfigApplyListQuery {
                link_id: Some(link_id),
                page: Some(1),
                page_size: Some(100),
                ..Default::default()
            })
            .await?;
        for a in &terminal {
            let st = i32::from(a.apply_status);
            if st == CONFIG_APPLY_APPLIED || st == CONFIG_APPLY_DENIED || st == CONFIG_APPLY_FAILED
            {
                let ack = if st == CONFIG_APPLY_APPLIED {
                    CONFIG_ACK_APPLIED
                } else if st == CONFIG_APPLY_DENIED {
                    CONFIG_ACK_DENIED
                } else {
                    CONFIG_ACK_FAILED
                };
                self.send_config_ack(&link, a, ack, &a.apply_message).await;
            }
        }
        Ok(json!({
            "fetched": wires.len(),
            "applied": applied,
            "pending": pending,
            "failed": failed,
            "rejected": rejected,
        }))
    }

    /// 配置包业务应用（DESIGN §2.3 边界）：announcement 回写本地平台公告；
    /// 其余类型只落快照。回 (落地状态, 说明)——说明如实记录落地形态。
    async fn apply_config_package(&self, apply: &HubConfigApply) -> (i32, String) {
        if apply.package_type == CONFIG_TYPE_ANNOUNCEMENT {
            let parsed: Value = serde_json::from_str(&apply.payload).unwrap_or(Value::Null);
            let title = parsed.get("title").and_then(Value::as_str).unwrap_or_default();
            let content = parsed
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if title.trim().is_empty() || content.trim().is_empty() {
                return (
                    CONFIG_APPLY_FAILED,
                    "公告包内容缺少 title/content，未回写".to_string(),
                );
            }
            let dto = nextrade_core::models::system::admin_msg::AnnouncementPublishDTO {
                title: Some(title.to_string()),
                content: Some(content.to_string()),
            };
            match crate::system::admin_msg::AdminMsgService::new(self.repos.clone())
                .publish_announcement(0, &dto)
                .await
            {
                Ok(vo) => (
                    CONFIG_APPLY_APPLIED,
                    format!("已回写本地平台公告（admin_msg 主行 #{})", vo.msg_id),
                ),
                Err(e) => (CONFIG_APPLY_FAILED, format!("公告回写失败: {e}")),
            }
        } else {
            (
                CONFIG_APPLY_APPLIED,
                format!(
                    "已落本地快照（类型 {} 业务回写未接入，仅归档）",
                    apply.package_type
                ),
            )
        }
    }

    /// 向中枢回执（失败只告警不回滚本地落地——pull 时会补发）。
    async fn send_config_ack(
        &self,
        link: &HubLink,
        apply: &HubConfigApply,
        ack_status: i32,
        message: &str,
    ) {
        let Ok(cred) = self.load_credential(link.up_credential_id).await else {
            return;
        };
        let Ok(secret) = self.decrypt_secret(&cred.secret_enc) else {
            return;
        };
        let body = json!({
            "packageId": apply.package_id,
            "version": apply.version,
            "status": ack_status,
            "message": message,
        });
        if let Err(e) = hub_post_json_signed(
            &link.peer_base_url,
            "/api/hub/v1/config/ack",
            &cred.access_key,
            &secret,
            &body,
        )
        .await
        {
            tracing::warn!(
                "联邦配置回执发送失败（下次拉取补发）: link={}, package={}, err={e}",
                link.link_id,
                apply.package_id
            );
        }
    }

    /// 管理端（成员）：手动包确认/拒绝。确认 → 应用并回执；拒绝 → 回执拒绝。
    pub async fn decide_config_apply(&self, dto: &ConfigDecideDTO) -> Result<()> {
        self.gate().await?;
        let apply = self
            .repo()
            .find_config_apply(dto.apply_id)
            .await?
            .ok_or_else(|| Error::NotFound("配置落地记录不存在".to_string()))?;
        if i32::from(apply.apply_status) != CONFIG_APPLY_PENDING {
            return Err(Error::Validation("该配置包已处理，不能重复确认".to_string()));
        }
        let link = self.load_link(apply.link_id).await?;
        let now = self.now();
        if dto.agree {
            let (status, message) = self.apply_config_package(&apply).await;
            if !self
                .repo()
                .set_config_apply_status(apply.id, status, &message, now)
                .await?
            {
                return Err(Error::Validation("该配置包已处理，不能重复确认".to_string()));
            }
            let ack = if status == CONFIG_APPLY_APPLIED {
                CONFIG_ACK_APPLIED
            } else {
                CONFIG_ACK_FAILED
            };
            self.send_config_ack(&link, &apply, ack, &message).await;
        } else {
            if !self
                .repo()
                .set_config_apply_status(apply.id, CONFIG_APPLY_DENIED, "成员侧人工拒绝", now)
                .await?
            {
                return Err(Error::Validation("该配置包已处理，不能重复确认".to_string()));
            }
            self.send_config_ack(&link, &apply, CONFIG_ACK_DENIED, "成员侧人工拒绝")
                .await;
        }
        Ok(())
    }

    /// 管理端（成员）：落地列表。
    pub async fn list_config_applies(
        &self,
        q: &HubConfigApplyListQuery,
    ) -> Result<PageResult<HubConfigApplyVO>> {
        self.gate().await?;
        let (rows, total) = self.repo().list_config_applies(q).await?;
        let list = rows.iter().map(HubConfigApplyVO::from).collect();
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        Ok(PageResult::new(total.max(0) as u64, list, &page))
    }

    // ———————————————————— 三期③：跨店下单（迁移 044；DESIGN.md §3） ————————————————————
    //
    // 中枢买家跨成员店铺下单：按 (连接, 远端店铺) 分组预占（价以成员确认价
    // 为准）→ 全组成功才建单 → 本站 paylog 付款（order_type=3）→ 支付回调
    // 路由落成员本地单（order_type=14）→ 成员回传状态聚合主单。
    // 资金：付款只走本站既有通道；分账只记账目（结算批 046），不执行渠道拆分。

    /// 中枢前台面：跨店下单（买家本人）。任一组预占失败 → 已预占组逐个
    /// 释放、本单不建；建单后落 paylog 并回主单 VO（含 paySn）。
    pub async fn xorder_create(
        &self,
        buyer_user_id: i32,
        dto: &XorderCreateDTO,
    ) -> Result<HubXorderVO> {
        self.gate_module(MODULE_ORDER, MSG_ORDER_MODULE_OFF).await?;
        if dto.items.is_empty() {
            return Err(Error::Validation("跨店下单明细不能为空".to_string()));
        }
        let addr = self
            .repos
            .member
            .user_address_client
            .find_by_id(dto.address_id)
            .await?
            .filter(|a| a.user_id == buyer_user_id)
            .ok_or_else(|| Error::Validation("收货地址不存在或不属于当前买家".to_string()))?;
        // 映射解析 + 分组（保持首次出现顺序）。
        let mut groups: Vec<(i32, i32, String, Vec<(HubProductMap, i32)>)> = Vec::new();
        for it in &dto.items {
            if it.quantity <= 0 {
                return Err(Error::Validation("购买数量必须大于 0".to_string()));
            }
            let map = self
                .repo()
                .find_product_map_by_id(it.map_id)
                .await?
                .ok_or_else(|| Error::Validation(format!("商品映射 {} 不存在", it.map_id)))?;
            if i32::from(map.sync_state) != 1 {
                return Err(Error::Validation(format!(
                    "商品「{}」当前不可跨店购买",
                    map.product_name
                )));
            }
            match groups.iter_mut().find(|g| g.0 == map.link_id && g.1 == map.remote_shop_id) {
                Some(g) => g.3.push((map, it.quantity)),
                None => groups.push((
                    map.link_id,
                    map.remote_shop_id,
                    map.remote_shop_name.clone(),
                    vec![(map, it.quantity)],
                )),
            }
        }
        let now = self.now();
        let xorder_sn = format!(
            "HX{}{}",
            utils::now_seconds(),
            utils::random_hex(6).to_uppercase()
        );
        // 逐组预占；失败释放已成功组。
        let mut reserved: Vec<XorderReservedGroup> = Vec::new();
        for (idx, (link_id, shop_id, shop_name, items)) in groups.iter().enumerate() {
            let sub_sn = format!("{xorder_sn}-{}", idx + 1);
            let link = self.load_link(*link_id).await?;
            if i32::from(link.role) != LINK_ROLE_HUB || i32::from(link.status) != LINK_ACTIVE {
                self.release_reserved_groups(&reserved).await;
                return Err(Error::Validation("目标成员连接未激活".to_string()));
            }
            if link.approved_modules & MODULE_ORDER == 0 {
                self.release_reserved_groups(&reserved).await;
                return Err(Error::Forbidden(MSG_ORDER_MODULE_OFF.to_string()));
            }
            let wire_items: Vec<XorderWireItem> = items
                .iter()
                .map(|(m, q)| XorderWireItem {
                    product_id: m.remote_product_id,
                    sku_id: 0,
                    quantity: *q,
                    price: None,
                    product_name: None,
                    pic_url: None,
                })
                .collect();
            let resp = match self
                .down_post_signed(
                    &link,
                    "/api/hub/v1/orders/reserve",
                    &json!({ "subSn": sub_sn, "items": wire_items }),
                )
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    self.release_reserved_groups(&reserved).await;
                    return Err(e);
                }
            };
            let confirmed = resp.get("confirmed").and_then(Value::as_bool).unwrap_or(false);
            if !confirmed {
                let reason = resp
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or("成员侧预占失败")
                    .to_string();
                self.release_reserved_groups(&reserved).await;
                return Err(Error::Validation(format!(
                    "{}（店铺 {}）",
                    reason, shop_name
                )));
            }
            let mut priced: Vec<(HubProductMap, i32, Decimal)> = Vec::new();
            let mut amount = Decimal::ZERO;
            for (m, q) in items {
                let price = resp
                    .get("items")
                    .and_then(Value::as_array)
                    .and_then(|arr| {
                        arr.iter().find(|w| {
                            w.get("productId").and_then(Value::as_i64)
                                == Some(i64::from(m.remote_product_id))
                        })
                    })
                    .map(|w| decimal_of(w.get("price")))
                    .filter(|p| *p > Decimal::ZERO)
                    .unwrap_or(m.price);
                amount += price * Decimal::from(*q);
                priced.push((m.clone(), *q, price));
            }
            let shipping_fee = decimal_of(resp.get("shippingFee"));
            reserved.push(XorderReservedGroup {
                link_id: *link_id,
                remote_shop_id: *shop_id,
                remote_shop_name: shop_name.clone(),
                sub_sn,
                items: priced,
                shipping_fee,
                amount,
            });
        }
        // 全组预占成功 → 建主单/子单/行（此后失败靠预占 TTL 兜底过期）。
        let product_amount: Decimal = reserved.iter().map(|g| g.amount).sum();
        let shipping_total: Decimal = reserved.iter().map(|g| g.shipping_fee).sum();
        let total = product_amount + shipping_total;
        let buyer_gid = self
            .repo()
            .find_member_map_by_user(0, buyer_user_id)
            .await?
            .map(|m| m.global_member_id)
            .unwrap_or_default();
        let full_address = format!(
            "{}{}{}",
            addr.region_names.replace(',', " "),
            addr.address,
            addr.address_line_two.as_deref().unwrap_or_default()
        );
        let master_id = self
            .repo()
            .insert_xorder_master(
                &xorder_sn,
                buyer_user_id,
                &buyer_gid,
                product_amount,
                shipping_total,
                total,
                &addr.consignee,
                &addr.mobile,
                full_address.trim(),
                now,
            )
            .await?;
        for g in &reserved {
            let sub_id = self
                .repo()
                .insert_xorder_sub(
                    master_id,
                    &g.sub_sn,
                    g.link_id,
                    g.remote_shop_id,
                    &g.remote_shop_name,
                    g.amount,
                    g.shipping_fee,
                    XSUB_RESERVED,
                    now,
                )
                .await?;
            for (m, q, price) in &g.items {
                self.repo()
                    .insert_xorder_item(
                        sub_id,
                        m.map_id,
                        m.remote_product_id,
                        &m.product_name,
                        &m.pic_url,
                        *price,
                        *q,
                        *price * Decimal::from(*q),
                    )
                    .await?;
            }
        }
        // 建 paylog（本站既有支付通道，业务类型 HubCrossOrder=3）。
        let paylog = crate::payment::paylog::PaylogService::new(self.repos.clone())
            .create_or_reuse(
                None,
                &crate::payment::paylog::PaylogOrderRef {
                    order_id: master_id,
                    order_sn: xorder_sn.clone(),
                    total_amount: total,
                    unpaid_amount: total,
                    presale_type: None,
                },
                "",
                nextrade_core::models::payment::paylog::PayOrderType::HubCrossOrder,
                None,
                None,
            )
            .await?;
        self.repo()
            .set_xorder_paylog(master_id, paylog.paylog_id, &paylog.pay_sn, now)
            .await?;
        let master = self
            .repo()
            .find_xorder(master_id)
            .await?
            .ok_or_else(|| Error::Internal("跨店主单落库后回读失败".to_string()))?;
        self.xorder_vo(&master).await
    }

    /// 释放已预占分组（下单失败回滚用；单组失败只告警，预占 TTL 兜底）。
    async fn release_reserved_groups(&self, groups: &[XorderReservedGroup]) {
        for g in groups {
            if let Ok(link) = self.load_link(g.link_id).await {
                let _ = self
                    .down_post_signed(
                        &link,
                        "/api/hub/v1/orders/release",
                        &json!({ "subSn": g.sub_sn, "reason": "下单失败回滚" }),
                    )
                    .await;
            }
        }
    }

    /// 向成员发下行签名请求（跨店/账本等中枢→成员通道共用）。
    async fn down_post_signed(&self, link: &HubLink, path: &str, body: &Value) -> Result<Value> {
        let cred = self.load_credential(link.down_credential_id).await?;
        let secret = self.decrypt_secret(&cred.secret_enc)?;
        hub_post_json_signed(&link.peer_base_url, path, &cred.access_key, &secret, body).await
    }

    /// 组装主单 VO（子单 + 行展开）。
    async fn xorder_vo(&self, master: &HubXorder) -> Result<HubXorderVO> {
        let mut vo = HubXorderVO::from(master);
        for s in self.repo().list_xorder_subs(master.id).await? {
            let items = self
                .repo()
                .list_xorder_items(s.id)
                .await?
                .iter()
                .map(HubXorderItemVO::from)
                .collect();
            vo.subs.push(HubXorderSubVO {
                id: s.id,
                sub_sn: s.sub_sn.clone(),
                link_id: s.link_id,
                remote_shop_id: s.remote_shop_id,
                remote_shop_name: s.remote_shop_name.clone(),
                amount: s.amount,
                shipping_fee: s.shipping_fee,
                status: i32::from(s.status),
                remote_order_id: s.remote_order_id,
                remote_order_sn: s.remote_order_sn.clone(),
                tracking_no: s.tracking_no.clone(),
                logistics_name: s.logistics_name.clone(),
                items,
            });
        }
        Ok(vo)
    }

    /// 中枢前台面：我的跨店单。
    pub async fn xorder_my_list(
        &self,
        buyer_user_id: i32,
        page: u64,
        size: u64,
    ) -> Result<PageResult<HubXorderVO>> {
        self.gate_module(MODULE_ORDER, MSG_ORDER_MODULE_OFF).await?;
        let (rows, total) = self
            .repo()
            .list_xorders_by_buyer(buyer_user_id, page, size)
            .await?;
        let mut list = Vec::with_capacity(rows.len());
        for r in &rows {
            list.push(self.xorder_vo(r).await?);
        }
        let pq = PageQuery {
            page,
            page_size: size,
        };
        Ok(PageResult::new(total.max(0) as u64, list, &pq))
    }

    /// 中枢前台面：跨店单详情（仅本人）。
    pub async fn xorder_detail(&self, buyer_user_id: i32, id: i32) -> Result<HubXorderVO> {
        self.gate_module(MODULE_ORDER, MSG_ORDER_MODULE_OFF).await?;
        let master = self
            .repo()
            .find_xorder(id)
            .await?
            .filter(|m| m.buyer_user_id == buyer_user_id)
            .ok_or_else(|| Error::NotFound(MSG_XORDER_NOT_FOUND.to_string()))?;
        self.xorder_vo(&master).await
    }

    /// 中枢前台面：取消未付跨店单（逐子单释放预占后关单）。
    pub async fn xorder_cancel(&self, buyer_user_id: i32, id: i32) -> Result<()> {
        self.gate_module(MODULE_ORDER, MSG_ORDER_MODULE_OFF).await?;
        let master = self
            .repo()
            .find_xorder(id)
            .await?
            .filter(|m| m.buyer_user_id == buyer_user_id)
            .ok_or_else(|| Error::NotFound(MSG_XORDER_NOT_FOUND.to_string()))?;
        if i32::from(master.status) != XORDER_PENDING_PAY || i32::from(master.pay_status) != 0 {
            return Err(Error::Validation("仅未支付的跨店单可取消".to_string()));
        }
        let now = self.now();
        for s in self.repo().list_xorder_subs(id).await? {
            if i32::from(s.status) == XSUB_RESERVED {
                if let Ok(link) = self.load_link(s.link_id).await {
                    let _ = self
                        .down_post_signed(
                            &link,
                            "/api/hub/v1/orders/release",
                            &json!({ "subSn": s.sub_sn, "reason": "买家取消" }),
                        )
                        .await;
                }
                self.repo()
                    .set_xorder_sub_status(s.id, XSUB_CLOSED, now)
                    .await?;
            }
        }
        self.repo().set_xorder_status(id, XORDER_CLOSED, now).await
    }

    /// 支付回调分发（paylog order_type=3）：置已付并逐子单路由。
    /// 幂等：已付主单重复回调时只补路由未路由子单（兼作路由重试入口）。
    pub async fn on_cross_order_paid(&self, master_id: i32) -> Result<()> {
        let master = self
            .repo()
            .find_xorder(master_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_XORDER_NOT_FOUND.to_string()))?;
        let now = self.now();
        if i32::from(master.pay_status) == 0 {
            self.repo()
                .mark_xorder_paid(master_id, 0, "", now)
                .await?;
        }
        for s in self.repo().list_xorder_subs(master_id).await? {
            if i32::from(s.status) != XSUB_RESERVED {
                continue;
            }
            let link = match self.load_link(s.link_id).await {
                Ok(l) => l,
                Err(e) => {
                    tracing::warn!("跨店子单路由跳过（连接读取失败）: sub={}, err={e}", s.sub_sn);
                    continue;
                }
            };
            let items = self.repo().list_xorder_items(s.id).await?;
            let wire_items: Vec<XorderWireItem> = items
                .iter()
                .map(|it| XorderWireItem {
                    product_id: it.remote_product_id,
                    sku_id: it.remote_sku_id,
                    quantity: it.quantity,
                    price: Some(it.price),
                    product_name: Some(it.product_name.clone()),
                    pic_url: Some(it.pic_url.clone()),
                })
                .collect();
            let body = json!({
                "subSn": s.sub_sn,
                "masterSn": master.xorder_sn,
                "items": wire_items,
                "buyerGlobalMemberId": master.global_member_id,
                "consignee": master.consignee,
                "mobile": master.mobile,
                "address": master.address,
                "paidAmount": s.amount + s.shipping_fee,
                "shippingFee": s.shipping_fee,
            });
            match self
                .down_post_signed(&link, "/api/hub/v1/orders/route", &body)
                .await
            {
                Ok(resp) => {
                    let rid = resp.get("localOrderId").and_then(Value::as_i64).unwrap_or(0) as i32;
                    let rsn = resp
                        .get("localOrderSn")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    self.repo()
                        .set_xorder_sub_remote_order(s.id, rid, rsn, self.now())
                        .await?;
                }
                Err(e) => {
                    tracing::warn!(
                        "跨店子单路由失败（子单保持已预占，可后台重试）: sub={}, err={e}",
                        s.sub_sn
                    );
                }
            }
        }
        self.refresh_xorder_aggregate(master_id).await
    }

    /// 按子单现状重算并落主单聚合状态。
    async fn refresh_xorder_aggregate(&self, master_id: i32) -> Result<()> {
        let master = self
            .repo()
            .find_xorder(master_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_XORDER_NOT_FOUND.to_string()))?;
        let subs = self.repo().list_xorder_subs(master_id).await?;
        let statuses: Vec<i32> = subs.iter().map(|s| i32::from(s.status)).collect();
        let agg = xorder_aggregate_status(&statuses, i32::from(master.status));
        if agg != i32::from(master.status) {
            self.repo()
                .set_xorder_status(master_id, agg, self.now())
                .await?;
        }
        // 全子单退款 → 主单支付状态置已退款（记账口径，渠道退款另行发起）。
        if !statuses.is_empty()
            && statuses.iter().all(|s| *s == XSUB_REFUNDED)
            && i32::from(master.pay_status) == 1
        {
            self.repo()
                .set_xorder_pay_status(master_id, 2, self.now())
                .await?;
        }
        Ok(())
    }

    /// 管理端（中枢）：路由重试（对已付主单重跑路由补落）。
    pub async fn xorder_route_retry(&self, master_id: i32) -> Result<()> {
        self.gate_module(MODULE_ORDER, MSG_ORDER_MODULE_OFF).await?;
        self.on_cross_order_paid(master_id).await
    }

    /// 中枢前台面：子单退款申请（已路由/已发货未完成时；成员确认未发货才关单）。
    pub async fn xorder_refund_sub(
        &self,
        buyer_user_id: i32,
        dto: &XorderRefundSubDTO,
    ) -> Result<()> {
        self.gate_module(MODULE_ORDER, MSG_ORDER_MODULE_OFF).await?;
        let sub = self
            .repo()
            .find_xorder_sub(dto.sub_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_XORDER_NOT_FOUND.to_string()))?;
        let master = self
            .repo()
            .find_xorder(sub.master_id)
            .await?
            .filter(|m| m.buyer_user_id == buyer_user_id)
            .ok_or_else(|| Error::NotFound(MSG_XORDER_NOT_FOUND.to_string()))?;
        let prev = i32::from(sub.status);
        if prev != XSUB_ROUTED && prev != XSUB_SHIPPED {
            return Err(Error::Validation("该子单当前状态不可申请退款".to_string()));
        }
        let now = self.now();
        self.repo()
            .set_xorder_sub_status(sub.id, XSUB_REFUNDING, now)
            .await?;
        let link = self.load_link(sub.link_id).await?;
        let reason = if dto.reason.trim().is_empty() {
            "买家申请退款"
        } else {
            dto.reason.trim()
        };
        match self
            .down_post_signed(
                &link,
                "/api/hub/v1/orders/refund",
                &json!({ "subSn": sub.sub_sn, "reason": reason }),
            )
            .await
        {
            Ok(_) => {
                self.repo()
                    .set_xorder_sub_status(sub.id, XSUB_REFUNDED, self.now())
                    .await?;
                // 三期⑤钩子：退款成立且此前已记分账行 → 生成负行冲抵。
                self.gen_xorder_refund_settle_line(&sub).await?;
                self.refresh_xorder_aggregate(master.id).await
            }
            Err(e) => {
                self.repo()
                    .set_xorder_sub_status(sub.id, prev, self.now())
                    .await?;
                Err(e)
            }
        }
    }

    /// 中枢侧：接收成员子单状态回传（路由层已过 order.report 鉴权与签名）。
    pub async fn receive_sub_status(
        &self,
        cred: &HubCredential,
        dto: &XorderSubStatusDTO,
    ) -> Result<()> {
        let sub = self
            .repo()
            .find_xorder_sub_by_sn(dto.sub_sn.trim())
            .await?
            .ok_or_else(|| Error::NotFound(MSG_XORDER_NOT_FOUND.to_string()))?;
        if sub.link_id != cred.link_id {
            return Err(Error::Forbidden("子单不属于调用连接".to_string()));
        }
        let from = i32::from(sub.status);
        if !xsub_transition_allowed(from, dto.status) {
            return Err(Error::Validation(format!(
                "子单状态不可从 {from} 推进到 {}",
                dto.status
            )));
        }
        self.repo()
            .set_xorder_sub_tracking(
                sub.id,
                dto.status,
                dto.tracking_no.trim(),
                dto.logistics_name.trim(),
                self.now(),
            )
            .await?;
        // 三期⑤钩子：子单完成 → 生成跨店分账账目行（只记账目，渠道未执行）。
        if dto.status == XSUB_COMPLETED {
            self.gen_xorder_settle_line(&sub).await?;
        }
        self.refresh_xorder_aggregate(sub.master_id).await
    }

    // ————— 成员侧：被调用端（下行钥 order.place，路由层鉴权） —————

    /// 成员侧：库存预占。逐商品验上架/库存（商品维度，扣有效预占），
    /// 全通过才落预占；确认价 = 本地当前售价（SKU 优先）。运费本批回 0
    /// （无统一运费模板可算，子单金额以商品额为准，页面如实标注）。
    pub async fn receive_order_reserve(
        &self,
        cred: &HubCredential,
        dto: &XorderReserveDTO,
    ) -> Result<XorderReserveVO> {
        let fail = |reason: &str| XorderReserveVO {
            confirmed: false,
            reason: reason.to_string(),
            items: Vec::new(),
            shipping_fee: Decimal::ZERO,
        };
        if cred.link_id <= 0 {
            return Ok(fail("预占凭证未绑定连接"));
        }
        if dto.sub_sn.trim().is_empty() || dto.items.is_empty() {
            return Ok(fail("预占参数不完整"));
        }
        let now = self.now();
        // 幂等：同子单号已有有效预占 → 直接回确认（价格以现价回带，不重复验库存，
        // 避免自身预占量把重试挤成假不足）。
        let existing = self.repo().list_holds_by_sn(cred.link_id, dto.sub_sn.trim()).await?;
        let already = existing
            .iter()
            .any(|h| i32::from(h.status) == HOLD_ACTIVE && h.expire_time > now);
        let mut priced: Vec<XorderWireItem> = Vec::new();
        for it in &dto.items {
            let Some((_, _, product_price, stock, _)) =
                self.repo().find_local_product_for_order(it.product_id).await?
            else {
                return Ok(fail("商品不存在或已下架"));
            };
            let price = if it.sku_id > 0 {
                self.repo()
                    .find_local_sku_price(it.product_id, it.sku_id)
                    .await?
                    .unwrap_or(product_price)
            } else {
                product_price
            };
            if !already {
                let held = self.repo().sum_active_holds(it.product_id, now).await?;
                if i64::from(stock) - held < i64::from(it.quantity) {
                    return Ok(fail("库存不足"));
                }
            }
            priced.push(XorderWireItem {
                product_id: it.product_id,
                sku_id: it.sku_id,
                quantity: it.quantity,
                price: Some(price),
                product_name: None,
                pic_url: None,
            });
        }
        if !already {
            let expire = i32::try_from(i64::from(now) + XORDER_HOLD_TTL_SECONDS)
                .unwrap_or(i32::MAX);
            for it in &dto.items {
                self.repo()
                    .insert_stock_hold(
                        cred.link_id,
                        dto.sub_sn.trim(),
                        it.product_id,
                        it.sku_id,
                        it.quantity,
                        expire,
                        now,
                    )
                    .await?;
            }
        }
        Ok(XorderReserveVO {
            confirmed: true,
            reason: String::new(),
            items: priced,
            shipping_fee: Decimal::ZERO,
        })
    }

    /// 成员侧：路由落单（子单号幂等）。消费预占 + 守卫扣库存 + 直插本地单
    /// （order_type=14，资金隔离口径同外卖聚合）同事务完成。
    pub async fn receive_order_route(
        &self,
        cred: &HubCredential,
        dto: &XorderRouteDTO,
    ) -> Result<XorderRouteVO> {
        if cred.link_id <= 0 {
            return Err(Error::Validation("路由凭证未绑定连接".to_string()));
        }
        if let Some(route) = self.repo().find_route_by_sub_sn(dto.sub_sn.trim()).await? {
            return Ok(XorderRouteVO {
                local_order_id: route.local_order_id,
                local_order_sn: route.local_order_sn.clone(),
            });
        }
        let now = self.now();
        let buyer_user_id = if dto.buyer_global_member_id.trim().is_empty() {
            0
        } else {
            self.repo()
                .find_member_map_by_global(cred.link_id, dto.buyer_global_member_id.trim())
                .await?
                .map(|m| m.local_user_id)
                .unwrap_or(0)
        };
        // 店铺归属：子单内商品同店（中枢按店铺分组），取首个商品的店铺。
        let first_shop = match dto.items.first() {
            Some(it) => self
                .repo()
                .find_local_product_for_order(it.product_id)
                .await?
                .map(|p| p.4)
                .unwrap_or(0),
            None => 0,
        };
        let mut order = nextrade_core::entities::order::Order::default();
        order.order_sn = utils::generate_order_sn();
        order.order_type = Some(XORDER_LOCAL_ORDER_TYPE);
        order.user_id = buyer_user_id;
        order.order_status = 1; // 待发货（已付路由单）
        order.shipping_status = 0;
        order.pay_status = 2; // 已支付
        order.add_time = now;
        order.consignee = dto.consignee.chars().take(255).collect();
        order.address = dto.address.clone();
        order.region_ids = "[]".to_string();
        order.region_names = String::new();
        order.mobile = dto.mobile.clone();
        order.shipping_method = 1;
        order.shop_id = first_shop;
        order.is_valid = 1;
        order.total_amount = dto.paid_amount;
        order.paid_amount = dto.paid_amount;
        order.unpaid_amount = Decimal::ZERO;
        order.product_amount = dto.paid_amount - dto.shipping_fee;
        order.shipping_fee = dto.shipping_fee;
        order.is_need_commisson = 0;
        order.referrer_user_id = 0;
        order.order_source = "联邦跨店".to_string();
        order.order_extension = json!({
            "hub": { "subSn": dto.sub_sn, "masterSn": dto.master_sn, "linkId": cred.link_id }
        })
        .to_string();
        order.external_channel = "hub".to_string();
        order.external_order_sn = dto.sub_sn.clone();
        order.admin_note = format!("联邦跨店单 {}（主单 {}）", dto.sub_sn, dto.master_sn);
        let mut order_items: Vec<nextrade_core::entities::order::OrderItem> = Vec::new();
        for it in &dto.items {
            let local = self.repo().find_local_product_for_order(it.product_id).await?;
            let mut item = nextrade_core::entities::order::OrderItem::default();
            item.user_id = buyer_user_id;
            item.price = it.price.unwrap_or(Decimal::ZERO);
            item.quantity = it.quantity;
            item.product_id = it.product_id;
            item.product_name = it
                .product_name
                .clone()
                .filter(|s| !s.is_empty())
                .or_else(|| local.as_ref().map(|p| p.0.clone()))
                .unwrap_or_default()
                .chars()
                .take(255)
                .collect();
            item.pic_thumb = it
                .pic_url
                .clone()
                .filter(|s| !s.is_empty())
                .or_else(|| local.as_ref().map(|p| p.1.clone()))
                .unwrap_or_default();
            item.sku_id = it.sku_id;
            item.sku_data = json!({ "hubSubSn": dto.sub_sn }).to_string();
            item.product_type = 1;
            item.shop_id = local.map(|p| p.4).unwrap_or(first_shop);
            item.origin_price = it.price;
            order_items.push(item);
        }
        let (order_id, order_sn) = self
            .repo()
            .land_route_order(
                cred.link_id,
                dto.sub_sn.trim(),
                &order,
                &order_items,
                dto.buyer_global_member_id.trim(),
                now,
            )
            .await?;
        Ok(XorderRouteVO {
            local_order_id: order_id,
            local_order_sn: order_sn,
        })
    }

    /// 成员侧：释放预占（已消费则拒绝）。
    pub async fn receive_order_release(
        &self,
        cred: &HubCredential,
        dto: &XorderSubSnDTO,
    ) -> Result<()> {
        if cred.link_id <= 0 {
            return Err(Error::Validation("释放凭证未绑定连接".to_string()));
        }
        if !self
            .repo()
            .release_holds(cred.link_id, dto.sub_sn.trim(), self.now())
            .await?
        {
            return Err(Error::Validation("预占已消费，不能释放".to_string()));
        }
        Ok(())
    }

    /// 成员侧：退款确认（本地单未发货才关单；已发货明确拒绝，中枢回滚子单状态）。
    pub async fn receive_order_refund(
        &self,
        cred: &HubCredential,
        dto: &XorderSubSnDTO,
    ) -> Result<()> {
        if cred.link_id <= 0 {
            return Err(Error::Validation("退款凭证未绑定连接".to_string()));
        }
        let route = self
            .repo()
            .find_route_by_sub_sn(dto.sub_sn.trim())
            .await?
            .ok_or_else(|| Error::NotFound("本地无此跨店路由单".to_string()))?;
        let state = self
            .repo()
            .find_local_order_state(route.local_order_id)
            .await?
            .ok_or_else(|| Error::NotFound("本地订单不存在".to_string()))?;
        if state.1 == 1 || state.3 > 0 || state.0 >= 2 {
            return Err(Error::Validation(
                "本地订单已发货，不能直接关单退款".to_string(),
            ));
        }
        self.repo().close_local_order(route.local_order_id).await?;
        self.repo()
            .set_route_status(route.id, XSUB_REFUNDED, self.now())
            .await?;
        Ok(())
    }

    /// 管理端（中枢）：跨店主单列表（子单展开）。
    pub async fn list_xorders_admin(
        &self,
        q: &HubXorderListQuery,
    ) -> Result<PageResult<HubXorderVO>> {
        self.gate_module(MODULE_ORDER, MSG_ORDER_MODULE_OFF).await?;
        let (rows, total) = self.repo().list_xorders(q).await?;
        let mut list = Vec::with_capacity(rows.len());
        for r in &rows {
            list.push(self.xorder_vo(r).await?);
        }
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        Ok(PageResult::new(total.max(0) as u64, list, &page))
    }

    /// 管理端（成员）：本机路由单列表（带本地单状态）。
    pub async fn list_xorder_routes_admin(
        &self,
        q: &HubXorderRouteListQuery,
    ) -> Result<PageResult<HubXorderRouteVO>> {
        self.gate_module(MODULE_ORDER, MSG_ORDER_MODULE_OFF).await?;
        let (rows, total) = self.repo().list_xorder_routes(q).await?;
        let mut list = Vec::with_capacity(rows.len());
        for r in &rows {
            let mut vo = HubXorderRouteVO::from(r);
            if let Some(st) = self.repo().find_local_order_state(r.local_order_id).await? {
                vo.local_order_status = st.0;
                vo.local_shipping_status = st.1;
            }
            list.push(vo);
        }
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        Ok(PageResult::new(total.max(0) as u64, list, &page))
    }

    /// 管理端（成员）：逐单同步状态到中枢（读本地单状态映射后上行回传，
    /// 与二期手动同步同口径，不侵入本地订单状态机）。
    pub async fn sync_route_status(&self, route_id: i32) -> Result<Value> {
        self.gate_module(MODULE_ORDER, MSG_ORDER_MODULE_OFF).await?;
        let route = self
            .repo()
            .find_xorder_route(route_id)
            .await?
            .ok_or_else(|| Error::NotFound("跨店路由单不存在".to_string()))?;
        let st = self
            .repo()
            .find_local_order_state(route.local_order_id)
            .await?
            .ok_or_else(|| Error::NotFound("本地订单不存在".to_string()))?;
        let mapped = xsub_from_local_order(st.0, st.1);
        if mapped == i32::from(route.status) {
            return Ok(json!({ "synced": false, "status": mapped }));
        }
        let link = self.load_link(route.link_id).await?;
        let cred = self.load_credential(link.up_credential_id).await?;
        let secret = self.decrypt_secret(&cred.secret_enc)?;
        hub_post_json_signed(
            &link.peer_base_url,
            "/api/hub/v1/orders/subStatus",
            &cred.access_key,
            &secret,
            &json!({
                "subSn": route.sub_sn,
                "status": mapped,
                "trackingNo": st.4,
                "logisticsName": st.5,
            }),
        )
        .await?;
        self.repo()
            .set_route_status(route.id, mapped, self.now())
            .await?;
        Ok(json!({ "synced": true, "status": mapped }))
    }

    // ———————————————————— 三期⑤：跨站结算（迁移 046；DESIGN.md §5） ————————————————————

    /// 跨店分账行生成（子单完成钩子；bizSn 幂等）。费率走 resolve_link_rate
    /// 单点收口：未定费率照常落行打标，生成结算单时拦截。
    pub async fn gen_xorder_settle_line(&self, sub: &HubXorderSub) -> Result<()> {
        let biz_sn = format!("xo-{}", sub.sub_sn);
        if self
            .repo()
            .find_settle_line_by_biz(sub.link_id, 0, &biz_sn)
            .await?
            .is_some()
        {
            return Ok(());
        }
        let link = self.load_link(sub.link_id).await?;
        let (rate, pending) = resolve_link_rate(link.rate_override);
        let (commission, peer) = xorder_split(sub.amount, sub.shipping_fee, rate);
        let items = self.repo().list_xorder_items(sub.id).await?;
        let line = HubSettleLine {
            link_id: sub.link_id,
            credential_id: 0,
            partner_user_id: link.partner_user_id,
            source_type: SETTLE_SRC_XORDER.to_string(),
            remote_order_sn: sub.remote_order_sn.clone(),
            item_name: items
                .first()
                .map(|i| i.product_name.clone())
                .unwrap_or_default(),
            quantity: items.iter().map(|i| i.quantity).sum(),
            deal_amount: sub.amount,
            supply_amount: Decimal::ZERO,
            rate,
            rate_pending: if pending { 1 } else { 0 },
            self_due: commission,
            peer_due: peer,
            line_status: SETTLE_LINE_NORMAL as i16,
            biz_sn,
            remark: "账目分账，渠道未执行".to_string(),
            add_time: self.now(),
            ..Default::default()
        };
        self.repo().insert_settle_line(&line).await?;
        Ok(())
    }

    /// 跨店退款冲抵行（退款成立钩子）：仅当正行已存在（子单曾完成记账）
    /// 才镜像负行；未完成过的子单退款不记负行（无佣金可冲）。
    pub async fn gen_xorder_refund_settle_line(&self, sub: &HubXorderSub) -> Result<()> {
        let Some(src) = self
            .repo()
            .find_settle_line_by_biz(sub.link_id, 0, &format!("xo-{}", sub.sub_sn))
            .await?
        else {
            return Ok(());
        };
        let line = HubSettleLine {
            link_id: src.link_id,
            credential_id: 0,
            partner_user_id: src.partner_user_id,
            source_type: SETTLE_SRC_XORDER_REFUND.to_string(),
            remote_order_sn: src.remote_order_sn.clone(),
            item_name: src.item_name.clone(),
            quantity: src.quantity,
            deal_amount: -src.deal_amount,
            supply_amount: Decimal::ZERO,
            rate: src.rate,
            rate_pending: src.rate_pending,
            self_due: -src.self_due,
            peer_due: -src.peer_due,
            line_status: SETTLE_LINE_NORMAL as i16,
            biz_sn: format!("xor-{}", sub.sub_sn),
            remark: "跨店退款冲抵（账目）".to_string(),
            add_time: self.now(),
            ..Default::default()
        };
        self.repo().insert_settle_line(&line).await?;
        Ok(())
    }

    /// 联邦面：供货成交回传接收（order.report）。以 bizSn 幂等；同单号重报
    /// 金额不一致拒收计差异。落行口径按连接结算模式：periodic（及未设/
    /// commission）→ 供货方应得入 peer_due；prepaid → 扣回传方预付账户
    /// （可扣负，扣负行标差异挂起）。feed 钥路径本批未落（见报告接缝）。
    pub async fn receive_settle_returns(
        &self,
        cred: &HubCredential,
        dto: &SettleReturnsDTO,
    ) -> Result<SettleReturnsVO> {
        if dto.returns.is_empty() || dto.returns.len() > 200 {
            return Err(Error::Validation("回传条目为空或超过 200 条".into()));
        }
        let link = self.load_link(cred.link_id).await?;
        let prepaid = link.settle_mode == "prepaid";
        let mut vo = SettleReturnsVO::default();
        for item in &dto.returns {
            let biz_sn = item.biz_sn.trim();
            if biz_sn.is_empty() || biz_sn.len() > 80 || item.quantity <= 0 {
                vo.diffs += 1;
                continue;
            }
            let amount = item.supply_price * Decimal::from(item.quantity);
            let deal = item.deal_price * Decimal::from(item.quantity);
            if let Some(exist) = self
                .repo()
                .find_settle_line_by_biz(cred.link_id, cred.credential_id, biz_sn)
                .await?
            {
                if exist.supply_amount.abs() != amount {
                    vo.diffs += 1;
                } else {
                    vo.duplicated += 1;
                }
                continue;
            }
            let mut line = HubSettleLine {
                link_id: cred.link_id,
                credential_id: cred.credential_id,
                partner_user_id: link.partner_user_id,
                remote_order_sn: item.remote_order_sn.trim().to_string(),
                item_name: item.item_name.trim().to_string(),
                quantity: item.quantity,
                line_status: SETTLE_LINE_NORMAL as i16,
                biz_sn: biz_sn.to_string(),
                add_time: self.now(),
                ..Default::default()
            };
            if prepaid {
                let Some(account) = self
                    .repo()
                    .find_settle_account_by_subject("link", cred.link_id)
                    .await?
                else {
                    // 无预付账户不可扣减：计差异待运营先开户。
                    vo.diffs += 1;
                    continue;
                };
                if item.refund {
                    let balance = self
                        .repo()
                        .adjust_settle_account(account.id, amount, self.now())
                        .await?
                        .unwrap_or(account.balance + amount);
                    line.source_type = SETTLE_SRC_SUPPLY_REFUND.to_string();
                    line.supply_amount = -amount;
                    line.remark = format!("预付退款回充，回充后余额 {balance}");
                } else {
                    let balance = self
                        .repo()
                        .adjust_settle_account(account.id, -amount, self.now())
                        .await?
                        .unwrap_or(account.balance - amount);
                    line.source_type = SETTLE_SRC_PREPAID_DEDUCT.to_string();
                    line.supply_amount = amount;
                    line.remark = format!("预付扣减，扣后余额 {balance}");
                    if balance < Decimal::ZERO {
                        line.line_status = SETTLE_LINE_SUSPENDED as i16;
                    }
                }
            } else if item.refund {
                line.source_type = SETTLE_SRC_SUPPLY_REFUND.to_string();
                line.deal_amount = -deal;
                line.supply_amount = -amount;
                line.peer_due = -amount;
                line.remark = "供货退款冲抵".to_string();
            } else {
                line.source_type = SETTLE_SRC_SUPPLY_RETURN.to_string();
                line.deal_amount = deal;
                line.supply_amount = amount;
                line.peer_due = amount;
                line.remark = "供货成交回传（周期结算）".to_string();
            }
            self.repo().insert_settle_line(&line).await?;
            vo.accepted += 1;
        }
        Ok(vo)
    }

    /// 联邦面：对端拉取本连接结算单（确认用）。
    pub async fn receive_settle_statements(
        &self,
        cred: &HubCredential,
    ) -> Result<Vec<HubSettleStatementVO>> {
        let rows = self.repo().list_link_statements(cred.link_id).await?;
        Ok(rows.iter().map(HubSettleStatementVO::from).collect())
    }

    /// 联邦面：对端确认结算单（双确认齐 → 已确认待结清）。
    pub async fn receive_settle_confirm(
        &self,
        cred: &HubCredential,
        dto: &SettleConfirmDTO,
    ) -> Result<HubSettleStatementVO> {
        let stmt = self
            .repo()
            .find_settle_statement_by_sn(dto.statement_sn.trim())
            .await?
            .filter(|s| s.link_id == cred.link_id)
            .ok_or_else(|| Error::NotFound(MSG_SETTLE_NOT_FOUND.into()))?;
        let updated = self
            .repo()
            .set_statement_peer_confirm(stmt.id, self.now())
            .await?
            .ok_or_else(|| Error::NotFound(MSG_SETTLE_NOT_FOUND.into()))?;
        if updated.self_confirm == 1 && i32::from(updated.exec_status) == SETTLE_EXEC_PENDING {
            self.repo()
                .set_statement_exec(stmt.id, SETTLE_EXEC_CONFIRMED, self.now())
                .await?;
        }
        let final_stmt = self
            .repo()
            .find_settle_statement(stmt.id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_SETTLE_NOT_FOUND.into()))?;
        Ok(HubSettleStatementVO::from(&final_stmt))
    }

    /// 管理端：生成结算单。先把上期结转额以 carry_in 行带入本期，再汇聚
    /// 周期内未入单正常行；未定费率行存在 → 拦截；net<0 → 整单结转下期。
    pub async fn generate_statement(
        &self,
        dto: &SettleGenerateDTO,
    ) -> Result<HubSettleStatementVO> {
        self.gate_module(MODULE_ORDER, MSG_ORDER_MODULE_OFF).await?;
        if dto.link_id <= 0 || dto.period_end < dto.period_start {
            return Err(Error::Validation("结算周期不合法".into()));
        }
        let link = self.load_link(dto.link_id).await?;
        for st in self.repo().list_carried_statements(dto.link_id).await? {
            let line = HubSettleLine {
                link_id: dto.link_id,
                credential_id: 0,
                partner_user_id: link.partner_user_id,
                source_type: SETTLE_SRC_CARRY_IN.to_string(),
                self_due: st.carry_amount,
                line_status: SETTLE_LINE_NORMAL as i16,
                biz_sn: format!("carry-{}", st.statement_sn),
                remark: format!("上期结算单 {} 结转入", st.statement_sn),
                add_time: self.now(),
                ..Default::default()
            };
            self.repo().insert_settle_line(&line).await?;
        }
        let lines = self
            .repo()
            .list_unstated_lines(dto.link_id, dto.period_start, dto.period_end)
            .await?;
        if lines.is_empty() {
            return Err(Error::Validation("周期内无可入单账目行".into()));
        }
        if lines.iter().any(|l| l.rate_pending == 1) {
            return Err(Error::Validation(MSG_SETTLE_RATE_PENDING.into()));
        }
        let mut total_deal = Decimal::ZERO;
        let mut self_total = Decimal::ZERO;
        let mut peer_total = Decimal::ZERO;
        for l in &lines {
            total_deal += l.deal_amount;
            self_total += l.self_due;
            peer_total += l.peer_due;
        }
        let net = peer_total - self_total;
        let (exec_status, carry_amount) = if net < Decimal::ZERO {
            (SETTLE_EXEC_CARRIED, -net)
        } else {
            (SETTLE_EXEC_PENDING, Decimal::ZERO)
        };
        let stmt = HubSettleStatement {
            statement_sn: format!("ST{}-{}", self.now(), utils::random_hex(4)),
            link_id: dto.link_id,
            credential_id: 0,
            partner_user_id: link.partner_user_id,
            period_start: dto.period_start,
            period_end: dto.period_end,
            total_deal,
            self_total,
            peer_total,
            net_amount: net,
            line_count: i32::try_from(lines.len()).unwrap_or(i32::MAX),
            diff_count: 0,
            statement_type: if self_total > Decimal::ZERO && peer_total > Decimal::ZERO {
                2
            } else {
                1
            },
            exec_status: exec_status as i16,
            carry_amount,
            add_time: self.now(),
            ..Default::default()
        };
        let stmt_id = self.repo().insert_settle_statement(&stmt).await?;
        let ids: Vec<i32> = lines.iter().map(|l| l.id).collect();
        self.repo()
            .assign_lines_to_statement(&ids, stmt_id, self.now())
            .await?;
        let saved = self
            .repo()
            .find_settle_statement(stmt_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_SETTLE_NOT_FOUND.into()))?;
        Ok(HubSettleStatementVO::from(&saved))
    }

    /// 管理端：本方确认结算单。
    pub async fn confirm_statement_admin(&self, id: i32) -> Result<HubSettleStatementVO> {
        self.gate_module(MODULE_ORDER, MSG_ORDER_MODULE_OFF).await?;
        let stmt = self
            .repo()
            .find_settle_statement(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_SETTLE_NOT_FOUND.into()))?;
        if i32::from(stmt.exec_status) == SETTLE_EXEC_CARRIED
            || i32::from(stmt.exec_status) == SETTLE_EXEC_SETTLED
        {
            return Ok(HubSettleStatementVO::from(&stmt));
        }
        let updated = self
            .repo()
            .set_statement_self_confirm(id, self.now())
            .await?
            .ok_or_else(|| Error::NotFound(MSG_SETTLE_NOT_FOUND.into()))?;
        if updated.peer_confirm == 1 && i32::from(updated.exec_status) == SETTLE_EXEC_PENDING {
            self.repo()
                .set_statement_exec(id, SETTLE_EXEC_CONFIRMED, self.now())
                .await?;
        }
        let final_stmt = self
            .repo()
            .find_settle_statement(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_SETTLE_NOT_FOUND.into()))?;
        Ok(HubSettleStatementVO::from(&final_stmt))
    }

    /// 管理端：结清登记（只改状态记账，不动资金）。
    pub async fn mark_statement_settled(&self, id: i32) -> Result<HubSettleStatementVO> {
        self.gate_module(MODULE_ORDER, MSG_ORDER_MODULE_OFF).await?;
        let stmt = self
            .repo()
            .find_settle_statement(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_SETTLE_NOT_FOUND.into()))?;
        if i32::from(stmt.exec_status) != SETTLE_EXEC_CONFIRMED {
            return Err(Error::Validation(
                "结算单未双确认，不能结清登记".into(),
            ));
        }
        self.repo()
            .set_statement_exec(id, SETTLE_EXEC_SETTLED, self.now())
            .await?;
        let saved = self
            .repo()
            .find_settle_statement(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_SETTLE_NOT_FOUND.into()))?;
        Ok(HubSettleStatementVO::from(&saved))
    }

    /// 管理端：账目行差异挂起/恢复（仅未入单行）。
    pub async fn set_settle_line_status(&self, dto: &SettleLineStatusDTO) -> Result<()> {
        self.gate_module(MODULE_ORDER, MSG_ORDER_MODULE_OFF).await?;
        if dto.line_status != SETTLE_LINE_NORMAL && dto.line_status != SETTLE_LINE_SUSPENDED {
            return Err(Error::Validation("行状态只可设为正常或差异挂起".into()));
        }
        let line = self
            .repo()
            .find_settle_line(dto.id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_SETTLE_NOT_FOUND.into()))?;
        if line.statement_id != 0 {
            return Err(Error::Validation("已入单的账目行不可变更状态".into()));
        }
        self.repo()
            .set_settle_line_status(dto.id, dto.line_status, self.now())
            .await?;
        Ok(())
    }

    /// 管理端：未定费率行重算（连接须已设覆盖费率）。
    pub async fn reprice_settle_lines(&self, link_id: i32) -> Result<Value> {
        self.gate_module(MODULE_ORDER, MSG_ORDER_MODULE_OFF).await?;
        let link = self.load_link(link_id).await?;
        let (rate, pending) = resolve_link_rate(link.rate_override);
        if pending {
            return Err(Error::Validation(
                "请先在连接审批中设置覆盖费率，再重算".into(),
            ));
        }
        let count = self
            .repo()
            .reprice_pending_lines(link_id, rate, self.now())
            .await?;
        Ok(json!({ "repriced": count, "rate": rate }))
    }

    /// 管理端：预付账户开户。
    pub async fn open_settle_account_admin(
        &self,
        dto: &SettleAccountOpenDTO,
    ) -> Result<HubSettleAccountVO> {
        self.gate_module(MODULE_ORDER, MSG_ORDER_MODULE_OFF).await?;
        if !matches!(dto.subject_type.as_str(), "link" | "credential") || dto.subject_id <= 0 {
            return Err(Error::Validation("结算账户主体不合法".into()));
        }
        let account = self
            .repo()
            .open_settle_account(
                &dto.subject_type,
                dto.subject_id,
                dto.warn_line,
                self.now(),
            )
            .await?;
        Ok(HubSettleAccountVO::from(&account))
    }

    /// 管理端：预付账户充值登记（只记账，不划拨真实资金）。
    pub async fn adjust_settle_account_admin(
        &self,
        dto: &SettleAccountAdjustDTO,
    ) -> Result<Value> {
        self.gate_module(MODULE_ORDER, MSG_ORDER_MODULE_OFF).await?;
        if dto.amount == Decimal::ZERO {
            return Err(Error::Validation("登记金额不可为 0".into()));
        }
        let balance = self
            .repo()
            .adjust_settle_account(dto.id, dto.amount, self.now())
            .await?
            .ok_or_else(|| Error::NotFound(MSG_SETTLE_NOT_FOUND.into()))?;
        Ok(json!({ "id": dto.id, "balance": balance }))
    }

    /// 管理端：预付账户列表。
    pub async fn list_settle_accounts_admin(
        &self,
    ) -> Result<PageResult<HubSettleAccountVO>> {
        self.gate_module(MODULE_ORDER, MSG_ORDER_MODULE_OFF).await?;
        let rows = self.repo().list_settle_accounts().await?;
        let total = rows.len() as u64;
        let list = rows.iter().map(HubSettleAccountVO::from).collect();
        let page = PageQuery {
            page: 1,
            page_size: u64::try_from(total.max(1)).unwrap_or(20),
        };
        Ok(PageResult::new(total, list, &page))
    }

    /// 管理端：账目行列表。
    pub async fn list_settle_lines_admin(
        &self,
        q: &HubSettleLineListQuery,
    ) -> Result<PageResult<HubSettleLineVO>> {
        self.gate_module(MODULE_ORDER, MSG_ORDER_MODULE_OFF).await?;
        let (rows, total) = self.repo().list_settle_lines(q).await?;
        let list = rows.iter().map(HubSettleLineVO::from).collect();
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        Ok(PageResult::new(total.max(0) as u64, list, &page))
    }

    /// 管理端：结算单列表。
    pub async fn list_settle_statements_admin(
        &self,
        q: &HubSettleStatementListQuery,
    ) -> Result<PageResult<HubSettleStatementVO>> {
        self.gate_module(MODULE_ORDER, MSG_ORDER_MODULE_OFF).await?;
        let (rows, total) = self.repo().list_settle_statements(q).await?;
        let list = rows.iter().map(HubSettleStatementVO::from).collect();
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        Ok(PageResult::new(total.max(0) as u64, list, &page))
    }

    // ———————————————————— 三期④：中央账本（迁移 045；DESIGN.md §4） ————————————————————

    /// 联邦面（中枢）：成员上行记账。记账机在仓储单事务内（锁账户行校验后
    /// 写行，balance_after 中枢生成）；本地永不直改联邦资产数字。
    pub async fn receive_wallet_post(
        &self,
        cred: &HubCredential,
        dto: &WalletPostDTO,
    ) -> Result<WalletPostVO> {
        let gid = dto.global_member_id.trim();
        let biz_sn = dto.biz_sn.trim();
        if gid.is_empty() || biz_sn.is_empty() || biz_sn.len() > 64 {
            return Err(Error::Validation(MSG_WALLET_BIZ_INVALID.into()));
        }
        if !wallet_biz_currency_valid(&dto.biz_type, &dto.currency) {
            return Err(Error::Validation(MSG_WALLET_BIZ_INVALID.into()));
        }
        let out = self
            .repo()
            .apply_wallet_post(
                cred.link_id,
                gid,
                &dto.currency,
                &dto.biz_type,
                dto.amount,
                biz_sn,
                dto.ref_biz_sn.trim(),
                dto.remark.trim(),
                self.now(),
            )
            .await?;
        Ok(WalletPostVO {
            line_id: out.line_id,
            global_member_id: gid.to_string(),
            currency: out.currency,
            biz_type: dto.biz_type.clone(),
            amount: out.amount,
            balance_after: out.balance_after,
            frozen_after: out.frozen_after,
            duplicated: out.duplicated,
        })
    }

    /// 联邦面（中枢）：余额查询（无账户回零值）。
    pub async fn receive_wallet_balance(
        &self,
        gid: &str,
    ) -> Result<WalletBalanceVO> {
        let gid = gid.trim();
        if gid.is_empty() {
            return Err(Error::Validation(MSG_WALLET_BIZ_INVALID.into()));
        }
        match self.repo().find_wallet_account(gid).await? {
            Some(a) => Ok(WalletBalanceVO::from(&a)),
            None => Ok(WalletBalanceVO {
                global_member_id: gid.to_string(),
                status: 1,
                ..Default::default()
            }),
        }
    }

    /// 管理端（中枢）：手工记账（发放/入账/冲正；bizSn 后台生成 admin- 前缀，
    /// link_id=0 表示中枢管理动作）。冻结类动作走成员上行或页面释放入口。
    pub async fn admin_wallet_post(&self, dto: &WalletAdminPostDTO) -> Result<WalletPostVO> {
        self.gate_module(MODULE_MEMBER, MSG_MEMBER_MODULE_OFF).await?;
        let gid = dto.global_member_id.trim();
        if gid.is_empty() || !wallet_biz_currency_valid(&dto.biz_type, &dto.currency) {
            return Err(Error::Validation(MSG_WALLET_BIZ_INVALID.into()));
        }
        if !matches!(
            dto.biz_type.as_str(),
            BIZ_POINTS_GRANT | BIZ_MONEY_CREDIT | BIZ_REVERSE
        ) {
            return Err(Error::Validation(
                "管理端手工记账仅支持积分发放、余额入账与冲正".into(),
            ));
        }
        let biz_sn = format!(
            "admin-{}-{}",
            utils::now_seconds(),
            &utils::random_hex(6)
        );
        let out = self
            .repo()
            .apply_wallet_post(
                0,
                gid,
                &dto.currency,
                &dto.biz_type,
                dto.amount,
                &biz_sn,
                dto.ref_biz_sn.trim(),
                if dto.remark.trim().is_empty() {
                    "中枢管理端手工记账"
                } else {
                    dto.remark.trim()
                },
                self.now(),
            )
            .await?;
        Ok(WalletPostVO {
            line_id: out.line_id,
            global_member_id: gid.to_string(),
            currency: out.currency,
            biz_type: dto.biz_type.clone(),
            amount: out.amount,
            balance_after: out.balance_after,
            frozen_after: out.frozen_after,
            duplicated: out.duplicated,
        })
    }

    /// 管理端（中枢）：账户列表。
    pub async fn list_wallet_accounts_admin(
        &self,
        q: &HubWalletAccountListQuery,
    ) -> Result<PageResult<WalletBalanceVO>> {
        self.gate_module(MODULE_MEMBER, MSG_MEMBER_MODULE_OFF).await?;
        let (rows, total) = self.repo().list_wallet_accounts(q).await?;
        let list = rows.iter().map(WalletBalanceVO::from).collect();
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        Ok(PageResult::new(total.max(0) as u64, list, &page))
    }

    /// 管理端（中枢）：账本行列表。
    pub async fn list_wallet_lines_admin(
        &self,
        q: &HubWalletLineListQuery,
    ) -> Result<PageResult<HubWalletLineVO>> {
        self.gate_module(MODULE_MEMBER, MSG_MEMBER_MODULE_OFF).await?;
        let (rows, total) = self.repo().list_wallet_lines(q).await?;
        let list = rows.iter().map(HubWalletLineVO::from).collect();
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        Ok(PageResult::new(total.max(0) as u64, list, &page))
    }

    /// 管理端（中枢）：释放过期冻结（>30 分钟未确认/释放；页面按钮触发，
    /// 本批不设调度器）。已确认/已释放的行由记账机拒绝重复处理并计跳过。
    pub async fn release_expired_freezes(&self) -> Result<Value> {
        self.gate_module(MODULE_MEMBER, MSG_MEMBER_MODULE_OFF).await?;
        let before = self.now() - WALLET_FREEZE_TTL_SECONDS as i32;
        let rows = self.repo().list_expired_freezes(before).await?;
        let mut released = 0;
        let mut skipped = 0;
        for line in &rows {
            let biz_sn = format!("expire-release-{}", line.id);
            match self
                .repo()
                .apply_wallet_post(
                    line.link_id,
                    &line.global_member_id,
                    WALLET_MONEY,
                    BIZ_MONEY_RELEASE,
                    Decimal::ZERO,
                    &biz_sn,
                    &line.biz_sn,
                    "冻结超时自动释放",
                    self.now(),
                )
                .await
            {
                Ok(_) => released += 1,
                Err(_) => skipped += 1,
            }
        }
        Ok(json!({ "scanned": rows.len(), "released": released, "skipped": skipped }))
    }

    /// 成员侧：向中枢记账并落本地镜像（中枢回执为准）。
    #[allow(clippy::too_many_arguments)]
    pub async fn wallet_post_to_hub(
        &self,
        link_id: i32,
        local_user_id: i32,
        gid: &str,
        currency: &str,
        biz_type: &str,
        amount: Decimal,
        biz_sn: &str,
        ref_biz_sn: &str,
        remark: &str,
    ) -> Result<WalletPostVO> {
        let link = self.load_link(link_id).await?;
        let cred = self.load_credential(link.up_credential_id).await?;
        let secret = self.decrypt_secret(&cred.secret_enc)?;
        let data = hub_post_json_signed(
            &link.peer_base_url,
            "/api/hub/v1/wallet/post",
            &cred.access_key,
            &secret,
            &json!({
                "bizSn": biz_sn,
                "bizType": biz_type,
                "globalMemberId": gid,
                "currency": currency,
                "amount": amount,
                "refBizSn": ref_biz_sn,
                "remark": remark,
            }),
        )
        .await?;
        let vo: WalletPostVO = serde_json::from_value(data)
            .map_err(|e| Error::Internal(format!("中枢记账回执解析失败: {e}")))?;
        self.repo()
            .insert_wallet_mirror(
                link_id,
                local_user_id,
                gid,
                biz_sn,
                &vo.currency,
                &vo.biz_type,
                vo.amount,
                vo.balance_after,
                self.now(),
            )
            .await?;
        Ok(vo)
    }

    /// 管理端（成员）：并账——把已映射会员的本地积分/余额一次性上行中枢
    /// （merge_in，bizSn 天然幂等），落并账状态；已并账会员跳过（一次性动作）。
    /// 本地列不清零：清零是资金动作，运营按本页快照核对后手工处理。
    pub async fn merge_wallet(&self, link_id: i32) -> Result<Value> {
        self.gate_module(MODULE_MEMBER, MSG_MEMBER_MODULE_OFF).await?;
        let link = self.load_link(link_id).await?;
        if link.up_credential_id <= 0 {
            return Err(Error::Validation("该连接没有上行凭证，无法并账".into()));
        }
        let maps = self.repo().list_member_maps(link_id).await?;
        let mut merged_count = 0;
        let mut skipped = 0;
        let mut total_points: i64 = 0;
        let mut total_money = Decimal::ZERO;
        for m in &maps {
            if m.local_user_id <= 0 || m.global_member_id.is_empty() {
                skipped += 1;
                continue;
            }
            if let Some(st) = self.repo().find_wallet_local_state(m.local_user_id).await? {
                if st.merged == 1 {
                    skipped += 1;
                    continue;
                }
            }
            let Some(user) = self.repos.member.user.find_by_id(m.local_user_id).await? else {
                skipped += 1;
                continue;
            };
            let points = i64::from(user.points.max(0));
            let money = user.available_balance();
            if points > 0 {
                self.wallet_post_to_hub(
                    link_id,
                    m.local_user_id,
                    &m.global_member_id,
                    WALLET_POINTS,
                    BIZ_MERGE_IN,
                    Decimal::from(points),
                    &wallet_merge_biz_sn(link_id, m.local_user_id, WALLET_POINTS),
                    "",
                    "存量并账：本地积分并入中枢账本",
                )
                .await?;
            }
            if money > Decimal::ZERO {
                self.wallet_post_to_hub(
                    link_id,
                    m.local_user_id,
                    &m.global_member_id,
                    WALLET_MONEY,
                    BIZ_MERGE_IN,
                    money,
                    &wallet_merge_biz_sn(link_id, m.local_user_id, WALLET_MONEY),
                    "",
                    "存量并账：本地余额并入中枢账本",
                )
                .await?;
            }
            self.repo()
                .upsert_wallet_local_state(
                    m.local_user_id,
                    link_id,
                    &m.global_member_id,
                    points,
                    money,
                    self.now(),
                )
                .await?;
            merged_count += 1;
            total_points += points;
            total_money += money;
        }
        Ok(json!({
            "merged": merged_count,
            "skipped": skipped,
            "pointsMerged": total_points,
            "moneyMerged": total_money,
        }))
    }

    /// 管理端（成员）：账本镜像列表。
    pub async fn list_wallet_mirrors_admin(
        &self,
        q: &HubWalletMirrorListQuery,
    ) -> Result<PageResult<HubWalletMirrorVO>> {
        self.gate_module(MODULE_MEMBER, MSG_MEMBER_MODULE_OFF).await?;
        let (rows, total) = self.repo().list_wallet_mirrors(q).await?;
        let list = rows.iter().map(HubWalletMirrorVO::from).collect();
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        Ok(PageResult::new(total.max(0) as u64, list, &page))
    }

    // ———————————————————— 三期①：登录互认（迁移 042；DESIGN.md §1） ————————————————————
    //
    // 授权码模式：成员登记请求 → 中枢会员本人同意 → 一次性码（5 分钟）→
    // 成员兑换并落号。红线：兑换只回映射与展示字段；未验证手机号不签发
    // 全局身份；影子账号密码为随机不可用值（只能经联邦登录或本地找回重置）。

    /// 中枢侧：成员登记授权请求（路由层已过 member.auth 鉴权与签名）。
    pub async fn sso_auth_request(
        &self,
        cred: &HubCredential,
        dto: &SsoAuthRequestDTO,
    ) -> Result<SsoAuthRequestVO> {
        let settings = self.gate().await?;
        if cred.link_id <= 0 {
            return Err(Error::Validation("登录互认凭证未绑定连接".to_string()));
        }
        let link = self.load_link(cred.link_id).await?;
        if i32::from(link.role) != LINK_ROLE_HUB || i32::from(link.status) != LINK_ACTIVE {
            return Err(Error::Validation("仅已激活的中枢侧连接可发起联邦登录".to_string()));
        }
        let state = dto.state.trim();
        if state.len() < 16 || state.len() > 128 {
            return Err(Error::Validation("state 长度不合法".to_string()));
        }
        if !sso_return_url_allowed(&dto.return_url, &link.peer_base_url) {
            return Err(Error::Validation(
                "returnUrl 不在连接登记地址白名单内".to_string(),
            ));
        }
        let now = self.now();
        let id = self
            .repo()
            .insert_sso_request(cred.link_id, state, dto.return_url.trim(), now)
            .await?;
        Ok(SsoAuthRequestVO {
            request_id: id,
            authorize_url: format!(
                "{}/federation/authorize?requestId={}",
                settings.instance_base_url.trim_end_matches('/'),
                id
            ),
        })
    }

    /// 中枢侧：兑换授权码。先校验（连接归属/已同意/未用/未过期）再条件烧码，
    /// 并发双兑只有一个成功；响应只含映射与展示字段（总设计 §6.3）。
    pub async fn sso_exchange(
        &self,
        cred: &HubCredential,
        dto: &SsoExchangeDTO,
    ) -> Result<SsoExchangeVO> {
        if cred.link_id <= 0 {
            return Err(Error::Validation("登录互认凭证未绑定连接".to_string()));
        }
        let code = dto.code.trim();
        if code.is_empty() {
            return Err(Error::Validation(MSG_SSO_CODE_INVALID.to_string()));
        }
        let row = self
            .repo()
            .find_sso_request_by_code_hash(&utils::sha256_hex(code))
            .await?
            .ok_or_else(|| Error::NotFound(MSG_SSO_CODE_INVALID.to_string()))?;
        let now = self.now();
        if row.link_id != cred.link_id
            || i32::from(row.status) != SSO_APPROVED
            || i32::from(row.code_used) != 0
            || row.code_expire_time <= now
        {
            return Err(Error::Validation(MSG_SSO_CODE_INVALID.to_string()));
        }
        if !self.repo().burn_sso_code(row.id, now).await? {
            return Err(Error::Validation(MSG_SSO_CODE_INVALID.to_string()));
        }
        let user = self
            .repo()
            .find_local_member_by_id(row.hub_user_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_SSO_CODE_INVALID.to_string()))?;
        Ok(SsoExchangeVO {
            global_member_id: row.global_member_id.clone(),
            mobile: user.mobile.clone(),
            nickname: user.nickname.clone().unwrap_or_default(),
            avatar: user.avatar.clone().unwrap_or_default(),
        })
    }

    /// 读授权请求并惰性流转过期（待同意超 30 分钟）。
    async fn load_sso_request_fresh(&self, request_id: i32) -> Result<HubSsoRequest> {
        let mut row = self
            .repo()
            .find_sso_request(request_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_SSO_REQUEST_INVALID.to_string()))?;
        if i32::from(row.status) == SSO_PENDING
            && i64::from(row.add_time) + SSO_REQUEST_TTL_SECONDS < i64::from(self.now())
        {
            self.repo().expire_sso_request(row.id, self.now()).await?;
            row.status = i16::try_from(SSO_EXPIRED).unwrap_or(3);
        }
        Ok(row)
    }

    /// 中枢会员面：授权请求信息（同意页展示申请方）。
    pub async fn sso_request_info(&self, request_id: i32) -> Result<SsoRequestInfoVO> {
        self.gate_module(MODULE_MEMBER, MSG_MEMBER_MODULE_OFF).await?;
        let row = self.load_sso_request_fresh(request_id).await?;
        let link = self.load_link(row.link_id).await?;
        Ok(SsoRequestInfoVO {
            request_id: row.id,
            peer_name: link.peer_name.clone(),
            peer_base_url: link.peer_base_url.clone(),
            status: i32::from(row.status),
            add_time: row.add_time,
        })
    }

    /// 中枢会员面：同意/拒绝。同意时解析（或新签）当前会员的全局 ID 并落码；
    /// 未验证手机号拒绝签发（与会员同步同口径，设计 §6.2/§6.3）。
    pub async fn sso_consent(
        &self,
        hub_user_id: i32,
        dto: &SsoConsentDTO,
    ) -> Result<SsoConsentVO> {
        self.gate_module(MODULE_MEMBER, MSG_MEMBER_MODULE_OFF).await?;
        let row = self.load_sso_request_fresh(dto.request_id).await?;
        if i32::from(row.status) != SSO_PENDING {
            return Err(Error::Validation(MSG_SSO_REQUEST_INVALID.to_string()));
        }
        let now = self.now();
        let sep = if row.return_url.contains('?') { '&' } else { '?' };
        let base = format!("{}{sep}", row.return_url);
        if !dto.agree {
            self.repo().deny_sso_request(row.id, now).await?;
            return Ok(SsoConsentVO {
                redirect_url: format!("{base}federationSso=denied"),
            });
        }
        let user = self
            .repo()
            .find_local_member_by_id(hub_user_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_MEMBER_NOT_FOUND.to_string()))?;
        if user.mobile_validated != 1 || user.mobile.trim().is_empty() {
            return Err(Error::Forbidden(MSG_SSO_MOBILE_UNVERIFIED.to_string()));
        }
        let gid = self.resolve_global_id_for_local_user(&user, now).await?;
        let code = utils::random_hex(32);
        let expire = i32::try_from(i64::from(now) + SSO_CODE_TTL_SECONDS).unwrap_or(i32::MAX);
        let ok = self
            .repo()
            .approve_sso_request(
                row.id,
                hub_user_id,
                &gid,
                &utils::sha256_hex(&code),
                expire,
                now,
            )
            .await?;
        if !ok {
            return Err(Error::Validation(MSG_SSO_REQUEST_INVALID.to_string()));
        }
        Ok(SsoConsentVO {
            redirect_url: format!("{base}code={code}&state={}", row.state),
        })
    }

    /// 中枢本地会员 → 全局 ID：先查自映射（link_id=0），再按已验证手机号
    /// 哈希命中既有全局 ID 挂靠，都没有则新签（与会员同步 created 同口径）。
    async fn resolve_global_id_for_local_user(
        &self,
        user: &LocalMemberRow,
        now: i32,
    ) -> Result<String> {
        if let Some(row) = self.repo().find_member_link_by_remote(0, user.user_id).await? {
            if i32::from(row.status) == 1 {
                return Ok(row.global_member_id);
            }
        }
        let hash = utils::sha256_hex(user.mobile.trim());
        let rows = self.repo().list_member_links_by_mobile_hash(&hash).await?;
        let mut gids: Vec<String> = rows.iter().map(|r| r.global_member_id.clone()).collect();
        gids.sort();
        gids.dedup();
        let gid = if gids.len() == 1 {
            gids[0].clone()
        } else {
            // 0 个 → 新签；多个（历史脏数据）→ 不猜，新签并留痕待人工核对。
            format_global_member_id(&uuid::Uuid::new_v4().to_string())
        };
        self.repo()
            .insert_member_link(
                &gid,
                0,
                user.user_id,
                &hash,
                &self.encrypt_mobile(user.mobile.trim()),
                user.nickname.as_deref().unwrap_or_default(),
                user.avatar.as_deref().unwrap_or_default(),
                user.birthday.as_deref().unwrap_or_default(),
                user.email.as_deref().unwrap_or_default(),
                "",
                user.reg_time.max(user.last_login),
                now,
            )
            .await?;
        self.repo()
            .upsert_member_map(0, user.user_id, &gid, user.reg_time.max(user.last_login), now)
            .await?;
        Ok(gid)
    }

    /// 成员面：解析发起联邦登录所用的成员连接（前台接缝补线）。
    /// 显式 link_id（>0）时按原口径取该连接；缺省（<=0，前台登录页
    /// 无从枚举连接）时取唯一符合条件的连接：成员角色 + 已激活 +
    /// 会员模块已批准。零条/多条都明确报错，不猜。
    async fn resolve_sso_link(&self, link_id: i32) -> Result<HubLink> {
        if link_id > 0 {
            return self.load_link(link_id).await;
        }
        let q = HubLinkListQuery {
            page: Some(1),
            page_size: Some(100),
            keyword: None,
            status: Some(LINK_ACTIVE),
            role: Some(LINK_ROLE_MEMBER),
        };
        let (rows, _) = self.repo().list_links(&q).await?;
        let mut eligible = rows
            .into_iter()
            .filter(|l| l.approved_modules & MODULE_MEMBER != 0);
        match (eligible.next(), eligible.next()) {
            (Some(link), None) => Ok(link),
            (None, _) => Err(Error::Validation("本站尚未接入联邦中枢".to_string())),
            (Some(_), Some(_)) => Err(Error::Validation(
                "本站存在多个中枢连接，暂不支持联邦登录".to_string(),
            )),
        }
    }

    /// 成员面：联邦登录可用性探针（前台接缝补线，登录页入口显隐）。
    /// 刻意不走 gate_module 硬闸：联邦未启用 / 会员模块未开 / 无可用
    /// 连接 / 配置读取失败一律回 available=false 而非报错——登录页
    /// 据此隐藏入口，fail-soft，绝不影响本站登录主链。只读无副作用
    /// （sso_start 会落 state 记录，不能兼作探针）。
    pub async fn sso_options(&self) -> Result<SsoOptionsVO> {
        let off = SsoOptionsVO {
            available: false,
            peer_name: String::new(),
        };
        let settings = match self.get_settings().await {
            Ok(s) => s,
            Err(_) => return Ok(off),
        };
        if !settings.enabled || settings.module_bits() & MODULE_MEMBER == 0 {
            return Ok(off);
        }
        match self.resolve_sso_link(0).await {
            Ok(link) => Ok(SsoOptionsVO {
                available: true,
                peer_name: link.peer_name.clone(),
            }),
            Err(_) => Ok(off),
        }
    }

    /// 成员面：发起联邦登录。后端生成 state（只存哈希），向上登记请求，
    /// 回中枢授权页地址供浏览器跳转。callback_url 必须以本机登记地址为
    /// 前缀（第一道闸；中枢侧再按连接登记地址复核）。
    pub async fn sso_start(&self, dto: &SsoStartDTO) -> Result<SsoStartVO> {
        let settings = self.gate_module(MODULE_MEMBER, MSG_MEMBER_MODULE_OFF).await?;
        let link = self.resolve_sso_link(dto.link_id).await?;
        if i32::from(link.role) != LINK_ROLE_MEMBER || i32::from(link.status) != LINK_ACTIVE {
            return Err(Error::Validation("仅已激活的成员连接可发起联邦登录".to_string()));
        }
        if link.approved_modules & MODULE_MEMBER == 0 {
            return Err(Error::Forbidden(MSG_MEMBER_MODULE_OFF.to_string()));
        }
        if !sso_return_url_allowed(&dto.callback_url, &settings.instance_base_url) {
            return Err(Error::Validation("回调地址必须为本站地址".to_string()));
        }
        let cred = self.load_credential(link.up_credential_id).await?;
        let secret = self.decrypt_secret(&cred.secret_enc)?;
        let state = utils::random_hex(24);
        let now = self.now();
        self.repo()
            .insert_sso_state(link.link_id, &utils::sha256_hex(&state), now)
            .await?;
        let resp = hub_post_json_signed(
            &link.peer_base_url,
            "/api/hub/v1/auth/request",
            &cred.access_key,
            &secret,
            &json!({ "state": state, "returnUrl": dto.callback_url.trim() }),
        )
        .await?;
        let authorize_url = resp
            .get("authorizeUrl")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Internal("中枢未返回授权页地址".to_string()))?;
        Ok(SsoStartVO {
            authorize_url: authorize_url.to_string(),
        })
    }

    /// 成员面：回调落号。校验 state → 向中枢兑换 → 按映射/手机号找号，
    /// 无号建影子账号（随机不可用密码）→ 落映射与发起记录完成态。
    /// 返回 (本地 user_id, username, 全局 ID, 是否新建) 供路由签发本地令牌。
    pub async fn sso_callback(
        &self,
        dto: &SsoCallbackDTO,
    ) -> Result<(i32, String, String, bool)> {
        self.gate_module(MODULE_MEMBER, MSG_MEMBER_MODULE_OFF).await?;
        let state = dto.state.trim();
        let code = dto.code.trim();
        if state.is_empty() || code.is_empty() {
            return Err(Error::Validation(MSG_SSO_STATE_INVALID.to_string()));
        }
        let state_hash = utils::sha256_hex(state);
        let srow = self
            .repo()
            .find_sso_state_by_hash(&state_hash)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_SSO_STATE_INVALID.to_string()))?;
        let now = self.now();
        if i32::from(srow.status) != 0
            || i64::from(srow.add_time) + SSO_REQUEST_TTL_SECONDS < i64::from(now)
        {
            return Err(Error::Validation(MSG_SSO_STATE_INVALID.to_string()));
        }
        let link = self.load_link(srow.link_id).await?;
        if i32::from(link.role) != LINK_ROLE_MEMBER || i32::from(link.status) != LINK_ACTIVE {
            return Err(Error::Validation(MSG_SSO_STATE_INVALID.to_string()));
        }
        let cred = self.load_credential(link.up_credential_id).await?;
        let secret = self.decrypt_secret(&cred.secret_enc)?;
        let resp = hub_post_json_signed(
            &link.peer_base_url,
            "/api/hub/v1/auth/exchange",
            &cred.access_key,
            &secret,
            &json!({ "code": code }),
        )
        .await?;
        let gid = resp
            .get("globalMemberId")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Internal("中枢兑换响应缺少全局会员 ID".to_string()))?
            .to_string();
        let mobile = resp
            .get("mobile")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        let nickname = resp
            .get("nickname")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let avatar = resp
            .get("avatar")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if mobile.is_empty() {
            return Err(Error::Internal("中枢兑换响应缺少手机号".to_string()));
        }
        // 落号三级：映射 → 手机号既有账号 → 新建影子账号。
        let mut created = false;
        let user_id = if let Some(map) = self
            .repo()
            .find_member_map_by_global(srow.link_id, &gid)
            .await?
        {
            map.local_user_id
        } else if let Some(existing) = self.repos.member.user.find_by_mobile(&mobile).await? {
            self.repo()
                .upsert_member_map(srow.link_id, existing.user_id, &gid, now, now)
                .await?;
            existing.user_id
        } else {
            let hash = nextrade_security::hash_password(&utils::random_hex(32))
                .map_err(|e| Error::Internal(format!("影子账号密码哈希失败: {e}")))?;
            let user = nextrade_core::entities::User {
                username: mobile.clone(),
                mobile: mobile.clone(),
                nickname: if nickname.is_empty() {
                    None
                } else {
                    Some(nickname.clone())
                },
                avatar: avatar.clone(),
                password: hash,
                status: Some(1),
                mobile_validated: 1,
                reg_time: now,
                ..Default::default()
            };
            match self.repos.member.user.insert(&user).await {
                Ok(id) => {
                    created = true;
                    self.repo()
                        .upsert_member_map(srow.link_id, id, &gid, now, now)
                        .await?;
                    id
                }
                Err(_) => {
                    // 并发落号撞手机号唯一约束：回查既有账号继续。
                    if let Some(existing) =
                        self.repos.member.user.find_by_mobile(&mobile).await?
                    {
                        self.repo()
                            .upsert_member_map(srow.link_id, existing.user_id, &gid, now, now)
                            .await?;
                        existing.user_id
                    } else {
                        // 手机号未被占而失败 = 用户名撞占：换 fed_ 前缀用户名重试
                        // （设计 §1.2：username 被占则 fed_ + 全局 ID 短码）。
                        let short: String = gid.chars().filter(|c| *c != '_').take(14).collect();
                        let retry = nextrade_core::entities::User {
                            username: format!("fed_{short}"),
                            ..user
                        };
                        let id = self.repos.member.user.insert(&retry).await?;
                        created = true;
                        self.repo()
                            .upsert_member_map(srow.link_id, id, &gid, now, now)
                            .await?;
                        id
                    }
                }
            }
        };
        self.repo()
            .complete_sso_state(&state_hash, user_id, now)
            .await?;
        let username = self
            .repos
            .member
            .user
            .find_by_id(user_id)
            .await?
            .map(|u| u.username)
            .unwrap_or_else(|| mobile.clone());
        Ok((user_id, username, gid, created))
    }

    /// 管理端：SSO 请求列表（授权审计，会员模块闸）。
    pub async fn list_sso_requests(
        &self,
        q: &HubSsoListQuery,
    ) -> Result<PageResult<HubSsoRequestVO>> {
        self.gate_module(MODULE_MEMBER, MSG_MEMBER_MODULE_OFF).await?;
        let (rows, total) = self.repo().list_sso_requests(q).await?;
        let mut list = Vec::with_capacity(rows.len());
        for r in &rows {
            let mut vo = HubSsoRequestVO::from(r);
            if r.link_id > 0 {
                if let Ok(link) = self.load_link(r.link_id).await {
                    vo.peer_name = link.peer_name.clone();
                }
            }
            list.push(vo);
        }
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        Ok(PageResult::new(total.max(0) as u64, list, &page))
    }

    /// 管理端：SSO 本机发起列表（成员侧视角，会员模块闸；设计 §1.3
    /// 「本机发起」页签，前台接缝补线）。与中枢侧请求列表同页分视角：
    /// 本表记本站前台发起的联邦登录（目标中枢、是否已回调落号），
    /// 中枢实例上本表为空属正常（发起记录只在成员侧产生）。
    pub async fn list_sso_states(
        &self,
        q: &HubSsoListQuery,
    ) -> Result<PageResult<HubSsoStateVO>> {
        self.gate_module(MODULE_MEMBER, MSG_MEMBER_MODULE_OFF).await?;
        let (rows, total) = self.repo().list_sso_states(q).await?;
        let mut list = Vec::with_capacity(rows.len());
        for r in &rows {
            let mut vo = HubSsoStateVO::from(r);
            if r.link_id > 0 {
                if let Ok(link) = self.load_link(r.link_id).await {
                    vo.peer_name = link.peer_name.clone();
                }
            }
            list.push(vo);
        }
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        Ok(PageResult::new(total.max(0) as u64, list, &page))
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
        scopes |= SCOPE_MEMBER_SYNC | SCOPE_MEMBER_AUTH | SCOPE_WALLET_POST;
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

/// 推送验签核心（与服务状态无关，便于单测）：窗口检查 + 常量时间比对。
fn verify_signature_with_hash(
    secret_hash_hex: &str,
    timestamp: i64,
    signature: &str,
    body: &str,
) -> Result<()> {
    let now = utils::now_seconds();
    if (now - timestamp).abs() > HUB_SIGNATURE_WINDOW_SECONDS {
        return Err(Error::Forbidden(MSG_HUB_SIGNATURE_INVALID.to_string()));
    }
    let key = hex_decode(secret_hash_hex)
        .filter(|k| k.len() == 32)
        .ok_or_else(|| Error::Internal("入站凭证缺少 secret 哈希，无法验签".to_string()))?;
    let expected = hub_signature(&key, timestamp, body);
    if !constant_time_eq(
        expected.as_bytes(),
        signature.trim().to_ascii_lowercase().as_bytes(),
    ) {
        return Err(Error::Forbidden(MSG_HUB_SIGNATURE_INVALID.to_string()));
    }
    Ok(())
}

/// hex 解码（签名密钥 / 哈希比对用）；非法字符 → None。
fn hex_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return None;
    }
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(s.len() / 2);
    let mut i = 0;
    while i < bytes.len() {
        let hi = (bytes[i] as char).to_digit(16)?;
        let lo = (bytes[i + 1] as char).to_digit(16)?;
        out.push(u8::try_from(hi * 16 + lo).ok()?);
        i += 2;
    }
    Some(out)
}

/// 推送签名（设计 §9.3）：HMAC-SHA256(key, "{timestamp}.{body}") 的 hex。
/// key = SHA-256(secret) 摘要字节——发送方由明文现算，接收方由 secret_hash
/// hex 解码，两侧同一密钥，中枢永不存明文。
/// 跨店下单预占分组（三期③下单过程中的中间态，模块级以便回滚释放共用）。
struct XorderReservedGroup {
    link_id: i32,
    remote_shop_id: i32,
    remote_shop_name: String,
    sub_sn: String,
    /// (聚合映射, 数量, 店主确认价)。
    items: Vec<(HubProductMap, i32, Decimal)>,
    shipping_fee: Decimal,
    amount: Decimal,
}

/// 配置包签名（三期②）：HMAC-SHA256(key, "{packageId}.{version}.{contentHash}")。
/// key = SHA-256(下行 secret) 摘要——中枢以密文解密值派生、成员以入站钥
/// secret_hash 解码，双方同值且都不需要对方明文（与推送验签同一 trick）。
fn config_package_signature(key: &[u8], package_id: i32, version: i32, content_hash: &str) -> String {
    let mut mac =
        <Hmac<Sha256> as Mac>::new_from_slice(key).expect("HMAC 接受任意长度密钥");
    mac.update(format!("{package_id}.{version}.{content_hash}").as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn hub_signature(key: &[u8], timestamp: i64, body: &str) -> String {
    let mut mac =
        <Hmac<Sha256> as Mac>::new_from_slice(key).expect("HMAC 接受任意长度密钥");
    mac.update(format!("{timestamp}.").as_bytes());
    mac.update(body.as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// 带签名的出站 POST（会员同步 / 订单上报）：body 只序列化一次，签名与
/// 发送用同一字节串，避免「签的和发的不是同一份」。
async fn hub_post_json_signed(
    base: &str,
    path: &str,
    access_key: &str,
    secret: &str,
    body: &Value,
) -> Result<Value> {
    let text = serde_json::to_string(body)
        .map_err(|e| Error::Internal(format!("联邦签名请求序列化失败: {e}")))?;
    let ts = utils::now_seconds();
    let key = Sha256::digest(secret.as_bytes());
    let signature = hub_signature(&key, ts, &text);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| Error::Internal(format!("联邦 HTTP 客户端构建失败: {e}")))?;
    let resp = client
        .post(format!("{}{}", base.trim_end_matches('/'), path))
        .header("Content-Type", "application/json")
        .header("X-Fed-Access-Key", access_key)
        .header("X-Fed-Secret", secret)
        .header("X-Fed-Timestamp", ts.to_string())
        .header("X-Fed-Signature", signature)
        .body(text)
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
    use super::*;
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

    /// 删连接清映射不变量（2026-11-21 验收观察项）：repo delete_link 必须在
    /// 同一事务内先删该连接的 hub_product_map、再删 hub_link 本体——删后该
    /// link 的映射为 0、无孤儿残留；且不得触碰审计流水 hub_call_log。
    /// 服务层不得再走「分页扫映射置已断开保留」的旧路径。
    #[test]
    fn delete_link_clears_product_maps_in_same_transaction() {
        let repo_src = include_str!("../../../infrastructure/src/db/repo/hub/federation.rs");
        let body = fn_body(repo_src, "pub async fn delete_link");
        let maps = body
            .find("DELETE FROM hub_product_map WHERE link_id")
            .expect("repo delete_link 应删除该连接的聚合映射");
        let link = body
            .find("DELETE FROM hub_link WHERE link_id")
            .expect("repo delete_link 应删除连接本体");
        assert!(maps < link, "必须先删映射、再删连接本体");
        assert!(
            body.contains("pool.begin()") && body.contains("commit()"),
            "映射删除与连接本体删除必须在同一事务内"
        );
        assert!(
            !body.contains("hub_call_log"),
            "调用日志属审计流水，删连接不得触碰"
        );
        let src = include_str!("federation.rs");
        let svc = fn_body(src, "pub async fn delete_link");
        assert!(
            !svc.contains("set_map_state"),
            "服务层 delete_link 不应再把映射置已断开保留"
        );
        assert!(
            svc.contains("delete_link(link_id)"),
            "服务层 delete_link 应收口到 repo 的事务删除"
        );
    }

    /// 删凭证清映射不变量（二期续补 delete_link 同款口径）：单向接入
    /// （type-② 出站凭证直连聚合）的映射行 link_id=0、只挂 credential_id，
    /// 删该凭证必须同事务清掉这些行，否则永久孤儿。
    #[test]
    fn delete_credential_clears_product_maps_too() {
        let repo_src = include_str!("../../../infrastructure/src/db/repo/hub/federation.rs");
        let body = fn_body(repo_src, "pub async fn delete_credential");
        assert!(
            body.contains("\"hub_product_map\""),
            "repo delete_credential 的级联表清单应含 hub_product_map"
        );
        assert!(
            body.contains("WHERE credential_id"),
            "映射清理应按 credential_id 定向删除"
        );
    }

    /// 推送签名往返：发送方用明文 secret 的 SHA-256 摘要签名，接收方用
    /// secret_hash（同一摘要的 hex）必须验得过；篡改 body/签名必须验不过；
    /// 超窗时间戳必须拒。
    #[test]
    fn push_signature_roundtrip_and_tamper_rejection() {
        let secret = "fed_test_secret_0123456789abcdef";
        let body = r#"{"events":[]}"#;
        let ts = utils::now_seconds();
        let key = Sha256::digest(secret.as_bytes());
        let sig = hub_signature(&key, ts, body);
        let hash = utils::sha256_hex(secret);
        verify_signature_with_hash(&hash, ts, &sig, body).expect("同密钥签名应验过");
        assert!(
            verify_signature_with_hash(&hash, ts, &sig, "{\"events\":[1]}").is_err(),
            "body 被篡改必须验签失败"
        );
        assert!(
            verify_signature_with_hash(&hash, ts, "deadbeef", body).is_err(),
            "伪造签名必须失败"
        );
        assert!(
            verify_signature_with_hash(&hash, ts - 3600, &sig, body).is_err(),
            "超窗时间戳必须拒"
        );
    }

    /// 会员同步守则：未验证手机号绝不自动匹配（进冲突队列）、换号撞号
    /// 进冲突、已有映射只做快照后写优先——三条都在 apply_member_event 内。
    #[test]
    fn member_sync_guards_verified_mobile_and_conflicts() {
        let src = include_str!("federation.rs");
        let body = fn_body(src, "async fn apply_member_event");
        assert!(
            body.contains("CONFLICT_UNVERIFIED_MOBILE"),
            "未验证手机号应入冲突队列"
        );
        assert!(
            body.contains("CONFLICT_MOBILE_CLAIMED"),
            "换号撞号应入冲突队列"
        );
        assert!(
            body.contains("decide_member_match("),
            "新面孔匹配应走纯函数裁决"
        );
        assert!(
            body.contains("stamp >= row.profile_updated_at"),
            "已有映射快照更新应有后写优先守卫"
        );
    }

    /// 订单上行守则：幂等 upsert + status_version 版本守卫，旧版本跳过。
    #[test]
    fn order_report_has_version_guard() {
        let src = include_str!("federation.rs");
        let body = fn_body(src, "pub async fn receive_order_report");
        assert!(
            body.contains("item.status_version >= old.status_version"),
            "订单上报应有版本守卫"
        );
        assert!(
            body.contains("upsert_order_summary("),
            "订单上报应走幂等 upsert"
        );
    }

    /// 三期④：上行 scope 派生——会员模块批复必须带 wallet.post（账本上行钥），
    /// 未批会员模块时不得派生（防越权记账）。
    #[test]
    fn up_scopes_include_wallet_post_with_member_module() {
        let src = include_str!("federation.rs");
        let body = fn_body(src, "fn up_scopes_for_modules");
        assert!(body.contains("SCOPE_WALLET_POST"), "会员模块派生缺 wallet.post");
        let member = up_scopes_for_modules(MODULE_MEMBER);
        assert_ne!(member & SCOPE_WALLET_POST, 0);
        let product_only = up_scopes_for_modules(MODULE_PRODUCT);
        assert_eq!(product_only & SCOPE_WALLET_POST, 0);
    }

    /// 三期②配置包签名：同钥同串稳定、任一字段篡改即变（成员验签的根基）。
    #[test]
    fn config_package_signature_is_stable_and_tamper_sensitive() {
        let key = [7u8; 32];
        let a = config_package_signature(&key, 3, 5, "abc123");
        let b = config_package_signature(&key, 3, 5, "abc123");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
        assert_ne!(a, config_package_signature(&key, 4, 5, "abc123"));
        assert_ne!(a, config_package_signature(&key, 3, 6, "abc123"));
        assert_ne!(a, config_package_signature(&key, 3, 5, "abc124"));
        assert_ne!(a, config_package_signature(&[8u8; 32], 3, 5, "abc123"));
    }
}
