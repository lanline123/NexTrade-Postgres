//! 平台联邦（hub::federation）DTO / VO / 纯逻辑。
//!
//! 设计：`~/workspace/hub-federation-design/proposal.md`（§2 开关、§3 连接与
//! 配对、§4 通用凭证、§5 商品上行聚合、§8 心跳与注册表）。
//!
//! # 口径要点（写死）
//! - 开关存 `config` 表一行 `biz_code='federationSettings'`，JSON 结构见
//!   [`FederationSettings`]；总开关默认关，关时联邦接口一律 404、后台
//!   「组织」菜单整体不出现。
//! - 凭证 = 身份 + scopes 位集 + 店铺范围 + 商品范围 + 价格规则 + 生命周期；
//!   价格规则/分类展开/contentHash 逐字复用 `catalog::product_feed` 纯函数。
//! - secret 只存 SHA-256（入站钥）或 AES-GCM 密文（出站钥，服务层加解密），
//!   明文仅签发/重置/配对领取时出现一次。
//! - 连接类型③的每把钥都由「被调用方」签发；配对码只存哈希、一码一连接、
//!   TTL 24h、claim 失败 5 次作废。

use std::collections::HashSet;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ———————————————————— 常量与文案 ————————————————————

/// config 表业务编码。
pub const FEDERATION_SETTINGS_CODE: &str = "federationSettings";

/// 连接类型：单向供货（我签发钥、下级拉货）。
pub const CONN_SUPPLY_OUT: i32 = 1;
/// 连接类型：单向接入（我填上级钥拉货）。
pub const CONN_SUPPLY_IN: i32 = 2;
/// 连接类型：全联邦（双向配对）。
pub const CONN_FEDERATION: i32 = 3;

/// 凭证方向：入站（我签发，对方持明文调我，我只存哈希）。
pub const DIRECTION_INBOUND: i32 = 1;
/// 凭证方向：出站（对方签发，我持明文调对方，我存加密可用形态）。
pub const DIRECTION_OUTBOUND: i32 = 2;

/// 连接状态：待审批。
pub const LINK_PENDING: i32 = 1;
/// 连接状态：已激活。
pub const LINK_ACTIVE: i32 = 2;
/// 连接状态：已停用。
pub const LINK_DISABLED: i32 = 3;
/// 连接状态：已拒绝。
pub const LINK_REJECTED: i32 = 4;
/// 连接状态：已过期（批准后超时未领取上行钥）。
pub const LINK_EXPIRED: i32 = 5;

/// 连接角色：我是中枢、对端是成员。
pub const LINK_ROLE_HUB: i32 = 1;
/// 连接角色：我是成员、对端是中枢。
pub const LINK_ROLE_MEMBER: i32 = 2;

/// 批准模块位：商品。
pub const MODULE_PRODUCT: i64 = 1;
/// 批准模块位：会员。
pub const MODULE_MEMBER: i64 = 2;
/// 批准模块位：订单。
pub const MODULE_ORDER: i64 = 4;

/// 配对码状态：未用。
pub const PAIRING_UNUSED: i32 = 0;
/// 配对码状态：已用。
pub const PAIRING_USED: i32 = 1;
/// 配对码状态：作废。
pub const PAIRING_VOID: i32 = 2;

/// 配对码 TTL（秒）：24 小时。
pub const PAIRING_TTL_SECONDS: i64 = 24 * 3600;
/// claim 失败次数上限，超限作废。
pub const PAIRING_MAX_FAILS: i32 = 5;
/// 批准后领取上行钥窗口（秒）：7 天。
pub const CLAIM_WINDOW_SECONDS: i64 = 7 * 24 * 3600;
/// 心跳失联阈值（秒）：15 分钟无心跳标失联。
pub const HEARTBEAT_STALE_SECONDS: i64 = 15 * 60;

pub const MSG_FEDERATION_DISABLED: &str = "平台联邦未开启";
pub const MSG_HUB_UNAUTHORIZED: &str = "联邦凭证无效";
pub const MSG_HUB_DISABLED: &str = "联邦凭证已停用";
pub const MSG_HUB_EXPIRED: &str = "联邦凭证已过期";
pub const MSG_HUB_SCOPE_DENIED: &str = "联邦凭证无此能力权限";
pub const MSG_HUB_LINK_INACTIVE: &str = "联邦连接未激活或已停用";
pub const MSG_HUB_RATE_LIMITED: &str = "联邦凭证调用超限";
pub const MSG_CRED_NOT_FOUND: &str = "联邦凭证不存在";
pub const MSG_CRED_NAME_REQUIRED: &str = "凭证名称不能为空";
pub const MSG_LINK_NOT_FOUND: &str = "联邦连接不存在";
pub const MSG_PAIRING_INVALID: &str = "配对码无效";
pub const MSG_PAIRING_EXPIRED: &str = "配对码已过期或已被使用";
pub const MSG_PAIRING_FORBIDDEN: &str = "配对码已被作废";
pub const MSG_CLAIM_TOKEN_INVALID: &str = "配对会话凭证无效";

// ———————————————————— scopes 位集（设计 §4.3） ————————————————————

/// scope 位：实例元信息/健康自检（所有连接凭证默认授予）。
pub const SCOPE_META_READ: i64 = 1 << 0;
/// scope 位：拉取被调方商品（= feed 全部能力）。
pub const SCOPE_PRODUCT_READ: i64 = 1 << 1;
/// scope 位：向中枢推送商品变更信号（P4 信号面，P1 只落位）。
pub const SCOPE_PRODUCT_PUSH: i64 = 1 << 2;
/// scope 位：向中枢上行订单汇总（P3）。
pub const SCOPE_ORDER_REPORT: i64 = 1 << 3;
/// scope 位：中枢拉成员订单（P3）。
pub const SCOPE_ORDER_READ: i64 = 1 << 4;
/// scope 位：向中枢同步会员（P2）。
pub const SCOPE_MEMBER_SYNC: i64 = 1 << 5;
/// scope 位：中枢查成员侧会员（P2）。
pub const SCOPE_MEMBER_READ: i64 = 1 << 6;
/// scope 位：授权码换取/令牌校验（P2）。
pub const SCOPE_MEMBER_AUTH: i64 = 1 << 7;
/// scope 位：拉取中枢运营配置/活动包（P4）。
pub const SCOPE_CONFIG_PULL: i64 = 1 << 8;
/// scope 位：心跳与统计上报。
pub const SCOPE_STATS_REPORT: i64 = 1 << 9;
/// scope 位：中枢向成员路由落单（P3）。
pub const SCOPE_ORDER_PLACE: i64 = 1 << 11;
/// scope 位：向中枢中央账本记账（P2）。
pub const SCOPE_WALLET_POST: i64 = 1 << 12;

/// 全部已定义 scope 位（校验签发时不得越界）。
pub const SCOPE_ALL_KNOWN: i64 = SCOPE_META_READ
    | SCOPE_PRODUCT_READ
    | SCOPE_PRODUCT_PUSH
    | SCOPE_ORDER_REPORT
    | SCOPE_ORDER_READ
    | SCOPE_MEMBER_SYNC
    | SCOPE_MEMBER_READ
    | SCOPE_MEMBER_AUTH
    | SCOPE_CONFIG_PULL
    | SCOPE_STATS_REPORT
    | SCOPE_ORDER_PLACE
    | SCOPE_WALLET_POST;

/// scope 位 → 名称（日志与后台人话文案共用）。
#[must_use]
pub fn scope_name(bit: i64) -> Option<&'static str> {
    Some(match bit {
        SCOPE_META_READ => "meta.read",
        SCOPE_PRODUCT_READ => "product.read",
        SCOPE_PRODUCT_PUSH => "product.push",
        SCOPE_ORDER_REPORT => "order.report",
        SCOPE_ORDER_READ => "order.read",
        SCOPE_MEMBER_SYNC => "member.sync",
        SCOPE_MEMBER_READ => "member.read",
        SCOPE_MEMBER_AUTH => "member.auth",
        SCOPE_CONFIG_PULL => "config.pull",
        SCOPE_STATS_REPORT => "stats.report",
        SCOPE_ORDER_PLACE => "order.place",
        SCOPE_WALLET_POST => "wallet.post",
        _ => return None,
    })
}

/// 展开 scopes 位集为名称清单（按位升序）。
#[must_use]
pub fn scope_names(scopes: i64) -> Vec<&'static str> {
    let mut out = Vec::new();
    let mut bit = 1i64;
    while bit <= SCOPE_WALLET_POST {
        if scopes & bit != 0 {
            if let Some(name) = scope_name(bit) {
                out.push(name);
            }
        }
        bit <<= 1;
    }
    out
}

/// scope 所属能力域模块位（签发约束：模块关则对应 bit 拒签）。
#[must_use]
pub fn scope_module(bit: i64) -> Option<i64> {
    match bit {
        SCOPE_PRODUCT_READ | SCOPE_PRODUCT_PUSH => Some(MODULE_PRODUCT),
        SCOPE_ORDER_REPORT | SCOPE_ORDER_READ | SCOPE_ORDER_PLACE => Some(MODULE_ORDER),
        SCOPE_MEMBER_SYNC | SCOPE_MEMBER_READ | SCOPE_MEMBER_AUTH | SCOPE_WALLET_POST => {
            Some(MODULE_MEMBER)
        }
        _ => None,
    }
}

// ———————————————————— 开关配置（设计 §2） ————————————————————

/// 能力开关（三项可组合）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct FederationCapabilities {
    /// 开放供货：我建钥供下级拉货。
    pub supply_out: bool,
    /// 接入供货：我填上级 API 拉货。
    pub supply_in: bool,
    /// 全联邦：配对连接（可作中枢收成员，亦可作成员接中枢）。
    pub federation: bool,
}

/// 能力域模块开关。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct FederationModules {
    pub product: bool,
    /// 会员联邦：默认关，不随其他模块联动打开。
    pub member: bool,
    pub order: bool,
}

/// 平台联邦设置（config 表 `federationSettings` 一行 JSON 的结构）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct FederationSettings {
    /// 第一层：系统级总开关（默认关）。
    pub enabled: bool,
    pub capabilities: FederationCapabilities,
    pub modules: FederationModules,
    /// 本实例联邦身份（首次开启角色时生成并持久化，终身不变）。
    pub instance_uuid: String,
    /// 本实例对外名称（配对/注册表展示）。
    pub instance_name: String,
    /// 本实例对外地址（根 URL，地址变更报备的本方值）。
    pub instance_base_url: String,
}

impl FederationSettings {
    /// 已启用的模块位集（与连接批准上限、凭证 scopes 取交集用）。
    #[must_use]
    pub fn module_bits(&self) -> i64 {
        let mut bits = 0i64;
        if self.modules.product {
            bits |= MODULE_PRODUCT;
        }
        if self.modules.member {
            bits |= MODULE_MEMBER;
        }
        if self.modules.order {
            bits |= MODULE_ORDER;
        }
        bits
    }

    /// 校验一份待签发 scopes 是否落在已启用模块内（`meta.read` /
    /// `stats.report` / `config.pull` 不属任何模块，恒可签）。
    ///
    /// # Errors
    /// 含未知位、或含未启用模块的位 → 错误文案。
    pub fn validate_scopes(&self, scopes: i64) -> Result<(), String> {
        if scopes & !SCOPE_ALL_KNOWN != 0 {
            return Err("包含未定义的权限位".to_string());
        }
        let mut bit = 1i64;
        while bit <= SCOPE_WALLET_POST {
            if scopes & bit != 0 {
                if let Some(module) = scope_module(bit) {
                    if self.module_bits() & module == 0 {
                        return Err(format!(
                            "能力 {} 所属模块未开启，不能签发",
                            scope_name(bit).unwrap_or("?")
                        ));
                    }
                }
            }
            bit <<= 1;
        }
        Ok(())
    }
}

/// 清洗一份设置（保存与读取共用）：只保留已知字段与类型，
/// 字符串去空白；非法输入回落全关默认。
#[must_use]
pub fn sanitize_federation_settings(input: &serde_json::Value) -> FederationSettings {
    let parsed: FederationSettings =
        serde_json::from_value(input.clone()).unwrap_or_default();
    FederationSettings {
        enabled: parsed.enabled,
        capabilities: parsed.capabilities,
        modules: parsed.modules,
        instance_uuid: parsed.instance_uuid.trim().to_string(),
        instance_name: parsed.instance_name.trim().chars().take(100).collect(),
        instance_base_url: parsed
            .instance_base_url
            .trim()
            .trim_end_matches('/')
            .to_string(),
    }
}

/// 由存储的原始 JSON 文本得到生效配置：解析失败/非对象 ⇒ 全关默认。
#[must_use]
pub fn effective_federation_settings(stored: Option<&str>) -> FederationSettings {
    let Some(raw) = stored else {
        return FederationSettings::default();
    };
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(v) if v.is_object() => sanitize_federation_settings(&v),
        _ => FederationSettings::default(),
    }
}

// ———————————————————— 行结构（FromRow） ————————————————————

/// `hub_credential` 行。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubCredential {
    pub credential_id: i32,
    pub conn_type: i16,
    pub link_id: i32,
    pub direction: i16,
    pub peer_base_url: String,
    pub cred_name: String,
    pub access_key: String,
    pub secret_hash: String,
    pub secret_enc: String,
    pub scopes: i64,
    pub scope_all_shops: i16,
    pub scope_mode: i16,
    pub include_inquiry: i16,
    pub price_rule_type: String,
    pub price_rule_value: Decimal,
    pub status: i16,
    pub expire_time: i32,
    pub rate_limit_per_min: i32,
    pub daily_quota: i64,
    pub ip_whitelist: String,
    pub last_used_time: i32,
    pub call_count: i64,
    pub add_time: i32,
    pub update_time: i32,
}

/// `hub_credential_price_override` 行。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubPriceOverride {
    pub id: i32,
    pub credential_id: i32,
    pub product_id: i32,
    pub rule_type: String,
    pub rule_value: Decimal,
    pub add_time: i32,
    pub update_time: i32,
}

/// `hub_link` 行（仅类型③全联邦连接）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubLink {
    pub link_id: i32,
    pub link_type: i16,
    pub role: i16,
    /// 对端库中的连接 id（仅成员侧用：向中枢调 pair/status 时报对端 id）。
    pub peer_link_id: i32,
    pub peer_instance_uuid: String,
    pub peer_name: String,
    pub peer_base_url: String,
    pub peer_version: String,
    pub status: i16,
    pub approved_modules: i64,
    pub up_credential_id: i32,
    pub down_credential_id: i32,
    /// 中枢侧伙伴账户（= 分销员 user_id；P1 只落结构与绑定，结算 P3）。
    pub partner_user_id: i32,
    /// 结算四件套（P1 只落结构，执行在 P3）。
    pub settle_mode: String,
    pub settle_cycle: String,
    pub fund_channel: String,
    pub rate_source: String,
    pub rate_override: Decimal,
    /// 成员侧配对会话凭证的哈希（中枢侧为空）。
    pub claim_token_hash: String,
    /// 成员侧保存的 claimToken 加密形态（轮询/领取用，中枢侧为空）。
    pub claim_token_enc: String,
    /// 领取截止时间（批准时间 + 7 天），0 = 不适用。
    pub claim_expire_time: i32,
    pub reject_reason: String,
    pub last_heartbeat_time: i32,
    pub heartbeat_payload: String,
    /// 成员侧会员同步游标：已成功推送的最大本地 user_id（迁移 028；中枢侧恒 0）。
    pub member_sync_cursor: i32,
    /// 成员侧订单上报游标：已成功上报的最大本地 order_id（迁移 028）。
    pub order_sync_cursor: i32,
    /// 成员侧订单最近一次上报完成时间（变更重扫窗口基准，迁移 028）。
    pub order_sync_time: i32,
    pub add_time: i32,
    pub update_time: i32,
}

/// `hub_pairing_code` 行（仅中枢侧）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubPairingCode {
    pub id: i32,
    pub code_hash: String,
    pub expect_name: String,
    pub status: i16,
    pub fail_count: i32,
    pub expire_time: i32,
    pub used_link_id: i32,
    pub add_time: i32,
}

/// `hub_call_log` 行。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubCallLogRow {
    pub log_id: i32,
    pub link_id: i32,
    pub credential_id: i32,
    pub access_key: String,
    pub endpoint: String,
    pub scope: String,
    pub direction: i16,
    pub http_status: i32,
    pub result_count: i32,
    pub client_ip: String,
    pub duration_ms: i32,
    pub add_time: i32,
}

/// `hub_product_map` 行（中枢侧聚合映射）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubProductMap {
    pub map_id: i32,
    pub link_id: i32,
    pub credential_id: i32,
    pub remote_instance_uuid: String,
    pub remote_shop_id: i32,
    pub remote_shop_name: String,
    pub remote_product_id: i32,
    pub hub_product_id: i32,
    pub product_name: String,
    pub pic_url: String,
    pub price: Decimal,
    pub market_price: Decimal,
    pub stock: i32,
    pub content_hash: String,
    pub snapshot: String,
    pub sync_state: i16,
    pub last_sync_time: i32,
    pub add_time: i32,
    pub update_time: i32,
}

// ———————————————————— 管理端 DTO / VO ————————————————————

/// 凭证列表查询。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubCredentialListQuery {
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    pub keyword: Option<String>,
    pub status: Option<i32>,
    pub conn_type: Option<i32>,
}

/// 凭证列表/详情 VO（永不含 secret 明文与哈希；出站钥只回脱敏）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubCredentialVO {
    pub credential_id: i32,
    pub conn_type: i32,
    pub link_id: i32,
    pub direction: i32,
    pub peer_base_url: String,
    pub cred_name: String,
    pub access_key: String,
    /// 出站钥 secret 脱敏展示（前后各 4 位）；入站钥为空串。
    pub secret_masked: String,
    pub scopes: i64,
    pub scope_names: Vec<String>,
    pub scope_all_shops: bool,
    pub shop_ids: Vec<i32>,
    pub scope_mode: i32,
    pub include_inquiry: bool,
    pub price_rule_type: String,
    pub price_rule_value: Decimal,
    pub status: i32,
    pub expire_time: i32,
    pub rate_limit_per_min: i32,
    pub daily_quota: i64,
    pub ip_whitelist: String,
    pub last_used_time: i32,
    pub call_count: i64,
    pub add_time: i32,
    pub update_time: i32,
    /// 当前口径下可见商品数（入站钥回填；出站钥为 -1 不适用）。
    pub product_count: i64,
}

/// 凭证详情 VO（补范围与覆盖）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubCredentialDetailVO {
    #[serde(flatten)]
    pub cred: HubCredentialVO,
    pub product_ids: Vec<i32>,
    pub category_ids: Vec<i32>,
    pub price_overrides: Vec<HubPriceOverrideVO>,
}

/// 单品覆盖 VO。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubPriceOverrideVO {
    pub product_id: i32,
    pub product_name: Option<String>,
    pub base_price: Option<Decimal>,
    pub rule_type: String,
    pub rule_value: Decimal,
    pub final_price: Option<Decimal>,
}

/// 新建/编辑凭证入参（编辑时范围清单整体替换；secret 不可经编辑改）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubCredentialSaveDTO {
    pub credential_id: Option<i32>,
    pub conn_type: Option<i32>,
    pub direction: Option<i32>,
    pub peer_base_url: Option<String>,
    /// 出站钥专用：对方签发的 secret 明文（加密落库；编辑时为空表示不改）。
    pub peer_secret: Option<String>,
    /// 出站钥专用：对方签发的 accessKey（编辑出站钥时可改）。
    pub peer_access_key: Option<String>,
    pub cred_name: Option<String>,
    pub scopes: Option<i64>,
    pub scope_all_shops: Option<bool>,
    pub shop_ids: Vec<i32>,
    pub scope_mode: Option<i32>,
    pub product_ids: Vec<i32>,
    pub category_ids: Vec<i32>,
    pub include_inquiry: Option<bool>,
    pub price_rule_type: Option<String>,
    pub price_rule_value: Option<Decimal>,
    pub status: Option<i32>,
    pub expire_time: Option<i32>,
    pub rate_limit_per_min: Option<i32>,
    pub daily_quota: Option<i64>,
    pub ip_whitelist: Option<String>,
}

/// 新建/重置入站钥的响应：**唯一一次**返回 secret 明文。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubCredentialSecretVO {
    pub credential_id: i32,
    pub access_key: String,
    pub secret: String,
}

/// 保存单品覆盖入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubPriceOverrideSaveDTO {
    pub credential_id: i32,
    pub product_id: i32,
    pub rule_type: Option<String>,
    pub rule_value: Option<Decimal>,
}

/// 通用 id 入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubIdDTO {
    pub id: i32,
    pub credential_id: Option<i32>,
    pub link_id: Option<i32>,
    pub product_id: Option<i32>,
    pub feed_key_id: Option<i32>,
    pub status: Option<i32>,
}

/// 连接列表查询。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubLinkListQuery {
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    pub keyword: Option<String>,
    pub status: Option<i32>,
    pub role: Option<i32>,
}

/// 连接 VO。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubLinkVO {
    pub link_id: i32,
    pub role: i32,
    pub peer_instance_uuid: String,
    pub peer_name: String,
    pub peer_base_url: String,
    pub peer_version: String,
    pub status: i32,
    /// 在线状态：`online` / `stale`（超 15 分钟无心跳）/ `unknown`（从未心跳）。
    pub heartbeat_state: String,
    pub approved_modules: i64,
    pub up_credential_id: i32,
    pub down_credential_id: i32,
    pub partner_user_id: i32,
    pub settle_mode: String,
    pub settle_cycle: String,
    pub fund_channel: String,
    pub rate_source: String,
    pub rate_override: Decimal,
    pub reject_reason: String,
    pub last_heartbeat_time: i32,
    pub claim_expire_time: i32,
    pub add_time: i32,
    pub update_time: i32,
}

/// 生成配对码入参/响应（明文只出现一次）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubPairingCreateDTO {
    pub expect_name: Option<String>,
}

/// 配对码响应（**唯一一次**返回明文）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubPairingCodeVO {
    pub id: i32,
    pub code: String,
    pub expect_name: String,
    pub expire_time: i32,
}

/// 审批入参（批准时设定模块上限/店铺与结算结构/伙伴账户绑定）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubLinkApproveDTO {
    pub link_id: i32,
    pub approved_modules: Option<i64>,
    pub partner_user_id: Option<i32>,
    pub settle_mode: Option<String>,
    pub settle_cycle: Option<String>,
    pub fund_channel: Option<String>,
    pub rate_source: Option<String>,
    pub rate_override: Option<Decimal>,
}

/// 拒绝入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubLinkRejectDTO {
    pub link_id: i32,
    pub reason: Option<String>,
}

/// 成员侧发起配对入参（我的连接页）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubPairInitiateDTO {
    pub hub_base_url: Option<String>,
    pub pairing_code: Option<String>,
}

/// 聚合列表查询。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubAggregationListQuery {
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    pub link_id: Option<i32>,
    pub keyword: Option<String>,
    pub sync_state: Option<i32>,
}

/// 聚合商品 VO（带来源标记）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubAggregationVO {
    pub map_id: i32,
    pub link_id: i32,
    pub credential_id: i32,
    pub remote_instance_uuid: String,
    pub remote_shop_id: i32,
    pub remote_shop_name: String,
    pub remote_product_id: i32,
    pub hub_product_id: i32,
    pub product_name: String,
    pub pic_url: String,
    pub price: Decimal,
    pub market_price: Decimal,
    pub stock: i32,
    pub content_hash: String,
    pub sync_state: i32,
    pub last_sync_time: i32,
}

/// 调用日志查询。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubCallLogListQuery {
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    pub credential_id: Option<i32>,
    pub link_id: Option<i32>,
}

// ———————————————————— 联邦面（/api/hub/v1）DTO ————————————————————

/// `POST /api/hub/v1/pair/claim` 入参（成员后端 → 中枢后端）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PairClaimDTO {
    pub pairing_code: String,
    pub instance_uuid: String,
    pub instance_name: String,
    pub base_url: String,
    pub version: String,
    /// 成员签发的下行钥（中枢持有调成员）：accessKey + secret 明文（HTTPS 一次）。
    pub down_access_key: String,
    pub down_secret: String,
    /// 成员为下行钥申请的 scopes（中枢审批时可收紧，实际以凭证行为准）。
    pub down_scopes: Option<i64>,
}

/// claim 响应。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairClaimVO {
    pub link_id: i32,
    pub claim_token: String,
    pub status: i32,
}

/// `POST /api/hub/v1/pair/status` 入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PairStatusDTO {
    pub link_id: i32,
    pub claim_token: String,
}

/// status 响应（激活前只回状态；activate 才给上行钥明文）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairStatusVO {
    pub link_id: i32,
    pub status: i32,
    pub reject_reason: String,
    pub approved_modules: i64,
}

/// activate 响应：上行钥明文**唯一一次**。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairActivateVO {
    pub link_id: i32,
    pub status: i32,
    pub up_access_key: String,
    pub up_secret: String,
}

/// `GET /api/hub/v1/meta` 响应。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubMetaVO {
    pub instance_uuid: String,
    pub instance_name: String,
    pub version: String,
    pub server_time: i64,
    pub api_version: String,
    pub credential_name: String,
    pub scopes: i64,
    pub product_count: i64,
}

/// `POST /api/hub/v1/heartbeat` 入参（成员 → 中枢）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HeartbeatDTO {
    pub instance_uuid: String,
    pub version: String,
    pub modules: Option<i64>,
    pub today_orders: Option<i64>,
    pub product_count: Option<i64>,
    pub pending_sync: Option<i64>,
}

/// heartbeat 响应。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatVO {
    pub server_time: i64,
    pub link_status: i32,
    /// 是否有新配置包（P1 恒 false，P4 配置下发启用）。
    pub has_config_package: bool,
}

/// `POST /api/hub/v1/link/updateEndpoint` 入参（成员 → 中枢报新地址）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UpdateEndpointDTO {
    pub instance_uuid: String,
    pub new_base_url: String,
}

/// 纯逻辑：由心跳时间推导在线状态。
#[must_use]
pub fn heartbeat_state(last_heartbeat_time: i32, now: i64) -> &'static str {
    if last_heartbeat_time <= 0 {
        return "unknown";
    }
    if now - i64::from(last_heartbeat_time) <= HEARTBEAT_STALE_SECONDS {
        "online"
    } else {
        "stale"
    }
}

/// 纯逻辑：secret 脱敏展示（前后各 4 位，短串全掩）。
#[must_use]
pub fn mask_secret(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= 8 {
        return "****".to_string();
    }
    let head: String = chars.iter().take(4).collect();
    let tail: String = chars.iter().skip(chars.len() - 4).collect();
    format!("{head}****{tail}")
}

/// 纯逻辑：校验对方地址（HTTPS 强制；本机回环仅开发放行由服务层按
/// 环境判断，此处只做形态校验）。
///
/// # Errors
/// 空/非 http(s)/含空白 → 错误文案。
pub fn validate_base_url(raw: &str) -> Result<String, String> {
    let url = raw.trim().trim_end_matches('/');
    if url.is_empty() {
        return Err("对方地址不能为空".to_string());
    }
    if url.chars().any(char::is_whitespace) {
        return Err("对方地址不能含空白字符".to_string());
    }
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err("对方地址必须以 https:// 开头".to_string());
    }
    Ok(url.to_string())
}

/// 校验连接级结算四件套取值（P1 只落结构，取值域先行写死）。
///
/// # Errors
/// 任一字段不在枚举内 → 错误文案。
pub fn validate_settle_fields(
    settle_mode: &str,
    settle_cycle: &str,
    fund_channel: &str,
    rate_source: &str,
) -> Result<(), String> {
    if !settle_mode.is_empty() && !matches!(settle_mode, "prepaid" | "periodic" | "commission") {
        return Err("结算模式不合法".to_string());
    }
    if !settle_cycle.is_empty()
        && !matches!(settle_cycle, "weekly" | "monthly")
        && !settle_cycle.starts_with("t_plus_")
    {
        return Err("账期周期不合法".to_string());
    }
    if !fund_channel.is_empty() && !matches!(fund_channel, "own" | "channel_split") {
        return Err("资金通道不合法".to_string());
    }
    if !rate_source.is_empty() && !matches!(rate_source, "level" | "link_override") {
        return Err("费率来源不合法".to_string());
    }
    Ok(())
}

/// 一批店铺范围是否合法（店铺 id 均为正数）。
#[must_use]
pub fn shops_scope_valid(shop_ids: &HashSet<i32>) -> bool {
    shop_ids.iter().all(|id| *id > 0)
}

// ———————————————————— 二期：会员同步 / 订单上行（迁移 028） ————————————————————
//
// 设计 §6（会员面）与 §7.1（订单头级上行）。本批落「同步核」：
// 全局 ID 映射 + 手机号匹配/冲突队列 + 资料快照后写优先 + 订单汇总幂等上行。
// 登录互认（§6.3）与中央账本（§6.5）、跨店下单（§7.2）、结算（§11）不在本批。

/// 会员映射状态：已关联。
pub const MEMBER_LINK_LINKED: i32 = 1;
/// 会员映射状态：已解绑。
pub const MEMBER_LINK_UNBOUND: i32 = 3;

/// 冲突状态：待处理。
pub const MEMBER_CONFLICT_PENDING: i32 = 0;
/// 冲突状态：已裁决关联到既有全局会员。
pub const MEMBER_CONFLICT_LINKED: i32 = 1;
/// 冲突状态：已裁决独立建号。
pub const MEMBER_CONFLICT_SEPARATE: i32 = 2;
/// 冲突状态：已忽略。
pub const MEMBER_CONFLICT_IGNORED: i32 = 3;

/// 冲突类型：同一连接内同一手机号对应多个远端会员（一对多）。
pub const CONFLICT_MOBILE_MULTI_REMOTE: &str = "mobile_multi_remote";
/// 冲突类型：同一手机号已对应多个全局 ID（多对一/历史多号）。
pub const CONFLICT_MOBILE_CLAIMED: &str = "mobile_claimed";
/// 冲突类型：手机号相同但实名姓名不一致。
pub const CONFLICT_IDENTITY_MISMATCH: &str = "identity_mismatch";
/// 冲突类型：手机号未经验证，不满足自动关联前提。
pub const CONFLICT_UNVERIFIED_MOBILE: &str = "unverified_mobile";

pub const MSG_MEMBER_MODULE_OFF: &str = "会员联邦模块未开启";
pub const MSG_ORDER_MODULE_OFF: &str = "订单模块未开启";
pub const MSG_MEMBER_NOT_FOUND: &str = "联邦会员不存在";
pub const MSG_CONFLICT_NOT_FOUND: &str = "会员冲突记录不存在";
pub const MSG_HUB_SIGNATURE_INVALID: &str = "联邦推送签名无效或已过期";

/// 推送类端点签名时间戳窗口（秒，设计 §9.3 ±5 分钟）。
pub const HUB_SIGNATURE_WINDOW_SECONDS: i64 = 300;

/// 全局会员 ID 形态：`gm_` + UUID（设计 §6.1，终身不变、不复用）。
#[must_use]
pub fn format_global_member_id(uuid: &str) -> String {
    format!("gm_{}", uuid.trim())
}

/// 纯逻辑：手机号脱敏（前 3 后 4；短串全掩）。日志/看板/VO 共用（设计 §6.6）。
#[must_use]
pub fn mask_mobile(mobile: &str) -> String {
    let chars: Vec<char> = mobile.chars().collect();
    if chars.len() < 7 {
        return "****".to_string();
    }
    let head: String = chars.iter().take(3).collect();
    let tail: String = chars.iter().skip(chars.len() - 4).collect();
    format!("{head}****{tail}")
}

/// 同手机号既有映射的精简视图（匹配纯逻辑入参）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MobileMatchRow {
    pub global_member_id: String,
    pub link_id: i32,
    pub remote_member_id: i32,
    pub real_name: String,
}

/// 手机号匹配裁决（设计 §6.2）：系统永不凭弱信号自动合并。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberMatchDecision {
    /// 新建全局 ID 并关联。
    CreateNew,
    /// 关联到既有全局 ID。
    LinkTo(String),
    /// 进冲突队列人工裁决（冲突类型 + 候选全局 ID，可空）。
    Conflict(&'static str, String),
}

/// 纯逻辑：给定同手机号的全部已关联映射行，为（link_id, remote_member_id）
/// 的一次新上报裁决去向。调用方保证该（连接, 远端会员）尚无映射行。
#[must_use]
pub fn decide_member_match(
    rows: &[MobileMatchRow],
    link_id: i32,
    real_name: &str,
) -> MemberMatchDecision {
    let mut globals: Vec<&str> = Vec::new();
    for row in rows {
        if !globals.contains(&row.global_member_id.as_str()) {
            globals.push(row.global_member_id.as_str());
        }
    }
    match globals.len() {
        0 => MemberMatchDecision::CreateNew,
        1 => {
            let gid = globals[0].to_string();
            // 同一连接内已有别的远端会员占住这个全局 ID：一对多，人工裁决。
            if rows.iter().any(|r| r.link_id == link_id) {
                return MemberMatchDecision::Conflict(CONFLICT_MOBILE_MULTI_REMOTE, gid);
            }
            // 实名不一致是强冲突信号：两侧都给了实名且不同 → 人工裁决。
            let known_name = rows
                .iter()
                .map(|r| r.real_name.trim())
                .find(|n| !n.is_empty())
                .unwrap_or("");
            if !real_name.trim().is_empty() && !known_name.is_empty() && real_name.trim() != known_name {
                return MemberMatchDecision::Conflict(CONFLICT_IDENTITY_MISMATCH, gid);
            }
            MemberMatchDecision::LinkTo(gid)
        }
        _ => MemberMatchDecision::Conflict(
            CONFLICT_MOBILE_CLAIMED,
            globals[0].to_string(),
        ),
    }
}

/// `hub_member_link` 行（中枢侧映射）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubMemberLink {
    pub id: i32,
    pub global_member_id: String,
    pub link_id: i32,
    pub remote_member_id: i32,
    pub mobile_hash: String,
    pub mobile_enc: String,
    pub nickname: String,
    pub avatar: String,
    pub birthday: String,
    pub email: String,
    pub real_name: String,
    pub profile_updated_at: i32,
    pub status: i16,
    pub add_time: i32,
    pub update_time: i32,
}

/// `hub_member_map` 行（成员侧映射）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubMemberMap {
    pub id: i32,
    pub link_id: i32,
    pub local_user_id: i32,
    pub global_member_id: String,
    pub profile_updated_at: i32,
    pub add_time: i32,
    pub update_time: i32,
}

/// `hub_member_conflict` 行（中枢侧冲突队列，带上报快照）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubMemberConflict {
    pub id: i32,
    pub link_id: i32,
    pub remote_member_id: i32,
    pub mobile_hash: String,
    pub mobile_enc: String,
    pub nickname: String,
    pub avatar: String,
    pub birthday: String,
    pub email: String,
    pub real_name: String,
    pub profile_updated_at: i32,
    pub candidate_global_member_id: String,
    pub conflict_type: String,
    pub status: i16,
    pub add_time: i32,
    pub update_time: i32,
}

/// `hub_order_summary` 行（中枢侧订单头级汇总）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubOrderSummary {
    pub id: i32,
    pub link_id: i32,
    pub remote_instance_uuid: String,
    pub remote_order_sn: String,
    pub remote_shop_id: i32,
    pub remote_shop_name: String,
    pub remote_user_id: i32,
    pub global_member_id: String,
    pub order_amount: Decimal,
    pub paid_amount: Decimal,
    pub order_status: i32,
    pub pay_status: i32,
    pub item_count: i32,
    pub order_time: i32,
    pub pay_time: i32,
    pub status_version: i64,
    pub add_time: i32,
    pub update_time: i32,
}

/// 调用日志 VO（管理端；全仓统一 camelCase——行结构 HubCallLogRow 是
/// snake_case 的 DB 形态，不得直出 API，前端表格按 camelCase 取数）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubCallLogVO {
    pub log_id: i32,
    pub link_id: i32,
    pub credential_id: i32,
    pub access_key: String,
    pub endpoint: String,
    pub scope: String,
    pub direction: i32,
    pub http_status: i32,
    pub result_count: i32,
    pub client_ip: String,
    pub duration_ms: i32,
    pub add_time: i32,
}

impl From<&HubCallLogRow> for HubCallLogVO {
    fn from(r: &HubCallLogRow) -> Self {
        Self {
            log_id: r.log_id,
            link_id: r.link_id,
            credential_id: r.credential_id,
            access_key: r.access_key.clone(),
            endpoint: r.endpoint.clone(),
            scope: r.scope.clone(),
            direction: i32::from(r.direction),
            http_status: r.http_status,
            result_count: r.result_count,
            client_ip: r.client_ip.clone(),
            duration_ms: r.duration_ms,
            add_time: r.add_time,
        }
    }
}

// ———————— 联邦面：会员同步（member.sync，推送类、HMAC 签名） ————————

/// 会员同步事件（成员 → 中枢，一批多条）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MemberSyncEvent {
    pub remote_member_id: i32,
    /// `upsert`（默认）/ `unbind`。
    pub event_type: String,
    pub mobile: String,
    pub mobile_validated: bool,
    pub email: String,
    pub nickname: String,
    pub avatar: String,
    pub birthday: String,
    pub real_name: String,
    /// 远端资料版本戳（秒；后写优先比较用）。
    pub profile_updated_at: i64,
}

/// `POST /api/hub/v1/members/sync` 入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MemberSyncDTO {
    pub events: Vec<MemberSyncEvent>,
}

/// 单条同步结果。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberSyncItemVO {
    pub remote_member_id: i32,
    /// 已裁决的全局 ID；冲突/跳过时为空（冲突裁决后经 anchor/轮询回补）。
    pub global_member_id: String,
    /// `created` / `linked` / `conflict` / `unbound` / `skipped`。
    pub outcome: String,
}

/// `POST /api/hub/v1/members/sync` 响应。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberSyncResultVO {
    pub results: Vec<MemberSyncItemVO>,
    pub server_time: i64,
}

/// `GET /api/hub/v1/members/query` 响应（中枢按全局 ID/手机号下钻成员侧会员）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberQueryVO {
    pub global_member_id: String,
    pub local_member_id: i32,
    /// 脱敏手机号（全号只在映射密文里，接口不回明文）。
    pub mobile_masked: String,
    pub nickname: String,
    pub avatar: String,
    pub birthday: String,
    pub email: String,
    /// 本地账号状态（user.status：1 启用 / 0 禁用，可能为空回 -1）。
    pub account_status: i32,
}

/// `GET /api/hub/v1/members/anchor` 响应（成员向中枢核对身份锚点状态）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberAnchorVO {
    pub global_member_id: String,
    pub exists: bool,
    /// 本连接名下的映射状态（1 已关联 / 3 已解绑 / 0 无映射）。
    pub link_status: i32,
    pub mobile_masked: String,
    pub profile_updated_at: i32,
}

// ———————— 联邦面：订单上行（order.report，推送类、HMAC 签名） ————————

/// 订单头级汇总条目（成员 → 中枢；设计 §7.1 白名单字段，无地址/明细）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OrderReportItem {
    pub remote_order_sn: String,
    pub remote_shop_id: i32,
    pub remote_shop_name: String,
    pub remote_user_id: i32,
    pub global_member_id: String,
    pub order_amount: Decimal,
    pub paid_amount: Decimal,
    pub order_status: i32,
    pub pay_status: i32,
    pub item_count: i32,
    pub order_time: i32,
    pub pay_time: i32,
    /// 远端事件时间戳（add/pay/ship/receive 取大），旧版本不覆盖新版本。
    pub status_version: i64,
}

/// `POST /api/hub/v1/orders/report` 入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OrderReportDTO {
    pub orders: Vec<OrderReportItem>,
}

/// `POST /api/hub/v1/orders/report` 响应（幂等计数）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderReportResultVO {
    pub created: i64,
    pub updated: i64,
    /// 旧版本/非法条目被跳过数。
    pub skipped: i64,
    pub server_time: i64,
}

// ———————— 管理端：会员联邦 / 订单上行 ————————

/// 会员映射列表查询（中枢侧）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubMemberListQuery {
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    pub link_id: Option<i32>,
    pub status: Option<i32>,
    /// 全局 ID / 昵称模糊。
    pub keyword: Option<String>,
}

/// 会员映射 VO（手机号只出脱敏）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubMemberLinkVO {
    pub id: i32,
    pub global_member_id: String,
    pub link_id: i32,
    pub remote_member_id: i32,
    pub mobile_masked: String,
    pub nickname: String,
    pub avatar: String,
    pub birthday: String,
    pub email: String,
    pub profile_updated_at: i32,
    pub status: i32,
    pub add_time: i32,
    pub update_time: i32,
}

/// 冲突队列查询。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubMemberConflictListQuery {
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    pub link_id: Option<i32>,
    pub status: Option<i32>,
}

/// 冲突 VO（手机号只出脱敏）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubMemberConflictVO {
    pub id: i32,
    pub link_id: i32,
    pub remote_member_id: i32,
    pub mobile_masked: String,
    pub nickname: String,
    pub email: String,
    pub candidate_global_member_id: String,
    pub conflict_type: String,
    pub status: i32,
    pub add_time: i32,
    pub update_time: i32,
}

/// 冲突裁决入参：`link` 关联候选全局 ID / `separate` 独立建号 / `ignore` 忽略。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubMemberResolveDTO {
    pub conflict_id: i32,
    pub action: String,
}

/// 订单上行列表查询（中枢侧）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubOrderSummaryListQuery {
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    pub link_id: Option<i32>,
    /// 远端订单号模糊。
    pub keyword: Option<String>,
}

/// 订单汇总 VO。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubOrderSummaryVO {
    pub id: i32,
    pub link_id: i32,
    pub remote_instance_uuid: String,
    pub remote_order_sn: String,
    pub remote_shop_id: i32,
    pub remote_shop_name: String,
    pub remote_user_id: i32,
    pub global_member_id: String,
    pub order_amount: Decimal,
    pub paid_amount: Decimal,
    pub order_status: i32,
    pub pay_status: i32,
    pub item_count: i32,
    pub order_time: i32,
    pub pay_time: i32,
    pub update_time: i32,
}

impl From<&HubOrderSummary> for HubOrderSummaryVO {
    fn from(r: &HubOrderSummary) -> Self {
        Self {
            id: r.id,
            link_id: r.link_id,
            remote_instance_uuid: r.remote_instance_uuid.clone(),
            remote_order_sn: r.remote_order_sn.clone(),
            remote_shop_id: r.remote_shop_id,
            remote_shop_name: r.remote_shop_name.clone(),
            remote_user_id: r.remote_user_id,
            global_member_id: r.global_member_id.clone(),
            order_amount: r.order_amount,
            paid_amount: r.paid_amount,
            order_status: r.order_status,
            pay_status: r.pay_status,
            item_count: r.item_count,
            order_time: r.order_time,
            pay_time: r.pay_time,
            update_time: r.update_time,
        }
    }
}

/// 本地会员扫描行（成员侧发送端读 `"user"` 表的窄列视图）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct LocalMemberRow {
    pub user_id: i32,
    pub mobile: String,
    pub mobile_validated: i32,
    pub email: Option<String>,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub birthday: Option<String>,
    pub status: Option<i32>,
    pub reg_time: i32,
    pub last_login: i32,
}

/// 本地订单扫描行（成员侧上报端读 `"order"` 头 + 件数聚合的窄列视图）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct LocalOrderRow {
    pub order_id: i32,
    pub order_sn: String,
    pub user_id: i32,
    pub shop_id: i32,
    pub shop_name: String,
    pub order_status: i32,
    pub pay_status: i32,
    pub total_amount: Decimal,
    pub paid_amount: Decimal,
    pub item_count: i32,
    pub add_time: i32,
    pub pay_time: i32,
    /// GREATEST(add/pay/shipping/received) —— 状态版本戳。
    pub version_time: i32,
}

// ———————————————————— 三期①：登录互认（迁移 042） ————————————————————
//
// 设计：~/workspace/federation-phase3/DESIGN.md §1（总设计 §6.3 授权码模式）。
// 四步：成员后端登记请求（member.auth）→ 中枢会员本人同意（会员 JWT）→
// 生成一次性授权码（5 分钟、只存哈希）→ 成员后端兑换（member.auth）→
// 成员侧按映射找/建影子账号并签发本地会员令牌。

/// SSO 请求状态：待同意。
pub const SSO_PENDING: i32 = 0;
/// SSO 请求状态：已同意（码已生成）。
pub const SSO_APPROVED: i32 = 1;
/// SSO 请求状态：已拒绝。
pub const SSO_DENIED: i32 = 2;
/// SSO 请求状态：已过期。
pub const SSO_EXPIRED: i32 = 3;

/// 授权码有效期（秒）：5 分钟（总设计 §6.3 写死）。
pub const SSO_CODE_TTL_SECONDS: i64 = 300;
/// 授权请求待同意有效期（秒）：30 分钟（超时未同意按过期处理）。
pub const SSO_REQUEST_TTL_SECONDS: i64 = 30 * 60;

pub const MSG_SSO_REQUEST_INVALID: &str = "联邦登录请求无效或已过期";
pub const MSG_SSO_CODE_INVALID: &str = "联邦授权码无效或已被使用";
pub const MSG_SSO_MOBILE_UNVERIFIED: &str = "当前账号手机号未验证，不能用于联邦登录";
pub const MSG_SSO_STATE_INVALID: &str = "联邦登录会话无效或已过期";

/// `hub_sso_request` 行（中枢侧授权请求）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubSsoRequest {
    pub id: i32,
    pub link_id: i32,
    /// 成员生成的防 CSRF state（非凭据，同意后回跳原样回带）。
    pub state: String,
    pub return_url: String,
    pub status: i16,
    pub hub_user_id: i32,
    pub global_member_id: String,
    pub code_hash: String,
    pub code_expire_time: i32,
    pub code_used: i16,
    pub add_time: i32,
    pub update_time: i32,
}

/// `hub_sso_state` 行（成员侧发起记录）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubSsoState {
    pub id: i32,
    pub link_id: i32,
    pub state_hash: String,
    pub status: i16,
    pub local_user_id: i32,
    pub add_time: i32,
    pub update_time: i32,
}

/// `POST /api/hub/v1/auth/request` 入参（成员后端 → 中枢）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SsoAuthRequestDTO {
    /// 成员生成的防 CSRF state（明文只过境，中枢只存哈希）。
    pub state: String,
    /// 同意后浏览器回跳地址（必须以连接登记的成员地址为前缀）。
    pub return_url: String,
}

/// `POST /api/hub/v1/auth/request` 响应。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SsoAuthRequestVO {
    pub request_id: i32,
    /// 中枢授权页地址（前端约定路径 /federation/authorize?requestId=）。
    pub authorize_url: String,
}

/// `POST /api/hub/v1/auth/exchange` 入参（成员后端 → 中枢，兑换授权码）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SsoExchangeDTO {
    pub code: String,
}

/// `POST /api/hub/v1/auth/exchange` 响应：只给映射与展示必需字段，
/// 绝不含密码哈希等任何凭据类字段（总设计 §6.3 写死）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SsoExchangeVO {
    pub global_member_id: String,
    pub mobile: String,
    pub nickname: String,
    pub avatar: String,
}

/// 中枢会员面：授权请求信息（同意页展示）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SsoRequestInfoVO {
    pub request_id: i32,
    /// 申请方实例名（连接对端名）。
    pub peer_name: String,
    pub peer_base_url: String,
    pub status: i32,
    pub add_time: i32,
}

/// 中枢会员面：同意/拒绝入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SsoConsentDTO {
    pub request_id: i32,
    pub agree: bool,
}

/// 中枢会员面：同意结果（浏览器据此回跳成员站）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SsoConsentVO {
    pub redirect_url: String,
}

/// 成员面：发起联邦登录入参（前台登录页调本机后端）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SsoStartDTO {
    pub link_id: i32,
    /// 本机回调页地址（前端拼好回传，须以本机 instanceBaseUrl 为前缀）。
    pub callback_url: String,
}

/// 成员面：发起结果（浏览器跳中枢授权页）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SsoStartVO {
    pub authorize_url: String,
}

/// 成员面：回调落号入参（中枢回跳带 code + state）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SsoCallbackDTO {
    pub code: String,
    pub state: String,
}

/// 成员面：回调落号结果（本地会员令牌，与登录 VO 同字段口径 + 联邦标记）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SsoCallbackVO {
    #[serde(rename = "token")]
    pub token: String,
    #[serde(rename = "accessToken")]
    pub access_token: <redacted>
    #[serde(rename = "refreshToken")]
    pub refresh_token: <redacted>
    pub expires_in: i64,
    pub user_id: i32,
    pub global_member_id: String,
    /// 本次是否新建了影子账号。
    pub created: bool,
}

/// 管理端：SSO 请求列表查询。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubSsoListQuery {
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    pub link_id: Option<i32>,
    pub status: Option<i32>,
}

/// 管理端：SSO 请求 VO（授权审计）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubSsoRequestVO {
    pub id: i32,
    pub link_id: i32,
    pub peer_name: String,
    pub return_url: String,
    pub status: i32,
    pub hub_user_id: i32,
    pub global_member_id: String,
    pub code_used: i32,
    pub add_time: i32,
    pub update_time: i32,
}

impl From<&HubSsoRequest> for HubSsoRequestVO {
    fn from(r: &HubSsoRequest) -> Self {
        Self {
            id: r.id,
            link_id: r.link_id,
            peer_name: String::new(),
            return_url: r.return_url.clone(),
            status: i32::from(r.status),
            hub_user_id: r.hub_user_id,
            global_member_id: r.global_member_id.clone(),
            code_used: i32::from(r.code_used),
            add_time: r.add_time,
            update_time: r.update_time,
        }
    }
}

/// 纯逻辑：returnUrl 白名单校验——必须以连接登记的成员地址为前缀
/// （配对登记地址即白名单，总设计 §6.3 redirect_uri 白名单的实现形态）。
#[must_use]
pub fn sso_return_url_allowed(return_url: &str, peer_base_url: &str) -> bool {
    let base = peer_base_url.trim().trim_end_matches('/');
    if base.is_empty() || return_url.trim().is_empty() {
        return false;
    }
    return_url.trim().starts_with(base)
}

// ———————————————————— 三期②：配置分发（迁移 043） ————————————————————
//
// 设计：~/workspace/federation-phase3/DESIGN.md §2（总设计 §8.3 拉模式）。
// 中枢发布带版本配置包 → 成员凭 config.pull 拉取 → 以下行钥派生签名验签
// + content_hash 双验 → 落本地快照 → 按 apply_mode 应用并回执。

/// 配置包类型：平台公告（唯一做业务回写的类型）。
pub const CONFIG_TYPE_ANNOUNCEMENT: &str = "announcement";
/// 配置包应用方式：成员侧手动确认。
pub const CONFIG_APPLY_MANUAL: i32 = 1;
/// 配置包应用方式：拉取后自动应用。
pub const CONFIG_APPLY_AUTO: i32 = 2;

/// 回执状态：已应用。
pub const CONFIG_ACK_APPLIED: i32 = 1;
/// 回执状态：已拒绝。
pub const CONFIG_ACK_DENIED: i32 = 2;
/// 回执状态：应用失败。
pub const CONFIG_ACK_FAILED: i32 = 3;

/// 落地状态：待处理。
pub const CONFIG_APPLY_PENDING: i32 = 0;
/// 落地状态：已应用。
pub const CONFIG_APPLY_APPLIED: i32 = 1;
/// 落地状态：已拒绝。
pub const CONFIG_APPLY_DENIED: i32 = 2;
/// 落地状态：失败。
pub const CONFIG_APPLY_FAILED: i32 = 3;

/// 单次拉取下发包数上限。
pub const CONFIG_PULL_LIMIT: i64 = 50;

pub const MSG_CONFIG_PACKAGE_INVALID: &str = "联邦配置包无效或验签失败";
pub const MSG_CONFIG_TYPE_UNKNOWN: &str = "未知配置包类型";

/// `hub_config_package` 行（中枢侧配置包）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubConfigPackage {
    pub id: i32,
    pub package_type: String,
    pub title: String,
    pub version: i32,
    pub payload: String,
    pub content_hash: String,
    pub apply_mode: i16,
    pub effective_from: i32,
    pub effective_to: i32,
    pub status: i16,
    pub add_time: i32,
    pub update_time: i32,
}

/// `hub_config_ack` 行（中枢侧回执）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubConfigAck {
    pub id: i32,
    pub link_id: i32,
    pub package_id: i32,
    pub version: i32,
    pub ack_status: i16,
    pub message: String,
    pub add_time: i32,
    pub update_time: i32,
}

/// `hub_config_apply` 行（成员侧落地）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubConfigApply {
    pub id: i32,
    pub link_id: i32,
    pub package_id: i32,
    pub version: i32,
    pub package_type: String,
    pub title: String,
    pub payload: String,
    pub content_hash: String,
    pub apply_mode: i16,
    pub apply_status: i16,
    pub apply_message: String,
    pub add_time: i32,
    pub update_time: i32,
}

/// 联邦面：单个配置包的线上形态（payload 为规范化 JSON 文本原样下发，
/// 成员以原文验 content_hash，避免重序列化漂移）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigPackageWireVO {
    pub package_id: i32,
    pub package_type: String,
    pub title: String,
    pub version: i32,
    pub payload: String,
    pub content_hash: String,
    pub apply_mode: i32,
    pub effective_from: i32,
    pub effective_to: i32,
    /// 下行钥派生签名：HMAC-SHA256(key=SHA256(下行 secret), "{packageId}.{version}.{contentHash}")。
    pub signature: String,
}

/// `GET /api/hub/v1/config/packages` 响应。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigPackagesVO {
    pub packages: Vec<ConfigPackageWireVO>,
    /// 中枢当前最大版本号（成员据此推进本地水位）。
    pub latest_version: i32,
}

/// `POST /api/hub/v1/config/ack` 入参（成员 → 中枢）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ConfigAckDTO {
    pub package_id: i32,
    pub version: i32,
    /// 1 已应用 / 2 已拒绝 / 3 应用失败。
    pub status: i32,
    pub message: String,
}

/// 管理端（中枢）：发布配置包入参。payload 为任意 JSON，落库时规范化为文本。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ConfigPublishDTO {
    pub package_type: String,
    pub title: String,
    pub payload: serde_json::Value,
    pub apply_mode: Option<i32>,
    pub effective_from: Option<i32>,
    pub effective_to: Option<i32>,
}

/// 管理端：配置包列表查询。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubConfigListQuery {
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    pub package_type: Option<String>,
    pub status: Option<i32>,
}

/// 管理端：配置包 VO（带回执覆盖统计）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubConfigPackageVO {
    pub id: i32,
    pub package_type: String,
    pub title: String,
    pub version: i32,
    pub payload: String,
    pub content_hash: String,
    pub apply_mode: i32,
    pub effective_from: i32,
    pub effective_to: i32,
    pub status: i32,
    /// 已回执连接数 / 其中已应用数（覆盖率展示）。
    pub ack_count: i64,
    pub applied_count: i64,
    pub add_time: i32,
    pub update_time: i32,
}

impl From<&HubConfigPackage> for HubConfigPackageVO {
    fn from(r: &HubConfigPackage) -> Self {
        Self {
            id: r.id,
            package_type: r.package_type.clone(),
            title: r.title.clone(),
            version: r.version,
            payload: r.payload.clone(),
            content_hash: r.content_hash.clone(),
            apply_mode: i32::from(r.apply_mode),
            effective_from: r.effective_from,
            effective_to: r.effective_to,
            status: i32::from(r.status),
            ack_count: 0,
            applied_count: 0,
            add_time: r.add_time,
            update_time: r.update_time,
        }
    }
}

/// 管理端：回执列表查询。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubConfigAckListQuery {
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    pub package_id: Option<i32>,
    pub link_id: Option<i32>,
}

/// 管理端：回执 VO。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubConfigAckVO {
    pub id: i32,
    pub link_id: i32,
    pub peer_name: String,
    pub package_id: i32,
    pub version: i32,
    pub ack_status: i32,
    pub message: String,
    pub add_time: i32,
    pub update_time: i32,
}

impl From<&HubConfigAck> for HubConfigAckVO {
    fn from(r: &HubConfigAck) -> Self {
        Self {
            id: r.id,
            link_id: r.link_id,
            peer_name: String::new(),
            package_id: r.package_id,
            version: r.version,
            ack_status: i32::from(r.ack_status),
            message: r.message.clone(),
            add_time: r.add_time,
            update_time: r.update_time,
        }
    }
}

/// 管理端（成员）：落地列表查询。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubConfigApplyListQuery {
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    pub link_id: Option<i32>,
    pub apply_status: Option<i32>,
}

/// 管理端（成员）：落地 VO。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubConfigApplyVO {
    pub id: i32,
    pub link_id: i32,
    pub package_id: i32,
    pub version: i32,
    pub package_type: String,
    pub title: String,
    pub payload: String,
    pub apply_mode: i32,
    pub apply_status: i32,
    pub apply_message: String,
    pub add_time: i32,
    pub update_time: i32,
}

impl From<&HubConfigApply> for HubConfigApplyVO {
    fn from(r: &HubConfigApply) -> Self {
        Self {
            id: r.id,
            link_id: r.link_id,
            package_id: r.package_id,
            version: r.version,
            package_type: r.package_type.clone(),
            title: r.title.clone(),
            payload: r.payload.clone(),
            apply_mode: i32::from(r.apply_mode),
            apply_status: i32::from(r.apply_status),
            apply_message: r.apply_message.clone(),
            add_time: r.add_time,
            update_time: r.update_time,
        }
    }
}

/// 管理端（成员）：手动包确认/拒绝入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ConfigDecideDTO {
    pub apply_id: i32,
    pub agree: bool,
}

/// 纯逻辑：配置包类型白名单。
#[must_use]
pub fn config_package_type_valid(package_type: &str) -> bool {
    matches!(package_type, "announcement" | "activity" | "channel" | "custom")
}

/// 纯逻辑：包在给定时刻是否处于生效窗口（status 与窗口由调用方另行判断状态位）。
#[must_use]
pub fn config_package_in_window(effective_from: i32, effective_to: i32, now: i32) -> bool {
    (effective_from == 0 || effective_from <= now) && (effective_to == 0 || effective_to >= now)
}

// ———————————————————— 三期③：跨店下单（迁移 044） ————————————————————
//
// 设计：~/workspace/federation-phase3/DESIGN.md §3（总设计 §7.2）。
// 中枢买家跨成员店铺下单：预占（店主确认价为准）→ 本站 paylog 付款 →
// 路由落成员本地单（order_type=14）→ 状态回传聚合。分账只记账目（046）。

/// 跨店单在成员侧落本地单的 order_type（迁移 036 外卖单为 13，本类为 14）。
pub const XORDER_LOCAL_ORDER_TYPE: i32 = 14;
/// 预占有效期（秒）：15 分钟。
pub const XORDER_HOLD_TTL_SECONDS: i64 = 15 * 60;

/// 主单状态。
pub const XORDER_PENDING_PAY: i32 = 0;
pub const XORDER_PAID: i32 = 1;
pub const XORDER_ROUTED: i32 = 2;
pub const XORDER_COMPLETED: i32 = 3;
pub const XORDER_CLOSED: i32 = 4;
pub const XORDER_AFTERSALE: i32 = 5;

/// 子单状态。
pub const XSUB_PENDING: i32 = 0;
pub const XSUB_RESERVED: i32 = 1;
pub const XSUB_ROUTED: i32 = 2;
pub const XSUB_SHIPPED: i32 = 3;
pub const XSUB_COMPLETED: i32 = 4;
pub const XSUB_REJECTED: i32 = 5;
pub const XSUB_RESERVE_FAILED: i32 = 6;
pub const XSUB_REFUNDED: i32 = 7;
pub const XSUB_REFUNDING: i32 = 8;
/// 子单状态：已关闭（买家取消/超时释放）——设计枚举外补值，注释与迁移同步。
pub const XSUB_CLOSED: i32 = 9;

/// 预占状态。
pub const HOLD_ACTIVE: i32 = 0;
pub const HOLD_CONSUMED: i32 = 1;
pub const HOLD_RELEASED: i32 = 2;
pub const HOLD_EXPIRED: i32 = 3;

pub const MSG_XORDER_NOT_FOUND: &str = "联邦跨店订单不存在";
pub const MSG_XORDER_RESERVE_FAILED: &str = "跨店预占失败";

/// `hub_xorder` 行（中枢侧主单）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubXorder {
    pub id: i32,
    pub xorder_sn: String,
    pub buyer_user_id: i32,
    pub global_member_id: String,
    pub product_amount: Decimal,
    pub shipping_fee: Decimal,
    pub total_amount: Decimal,
    pub pay_status: i16,
    pub status: i16,
    pub paylog_id: i32,
    pub pay_sn: String,
    pub consignee: String,
    pub mobile: String,
    pub address: String,
    pub add_time: i32,
    pub pay_time: i32,
    pub update_time: i32,
}

/// `hub_xorder_sub` 行（中枢侧子单）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubXorderSub {
    pub id: i32,
    pub master_id: i32,
    pub sub_sn: String,
    pub link_id: i32,
    pub remote_shop_id: i32,
    pub remote_shop_name: String,
    pub amount: Decimal,
    pub shipping_fee: Decimal,
    pub status: i16,
    pub remote_order_id: i32,
    pub remote_order_sn: String,
    pub tracking_no: String,
    pub logistics_name: String,
    pub add_time: i32,
    pub update_time: i32,
}

/// `hub_xorder_item` 行（中枢侧单行）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubXorderItem {
    pub id: i32,
    pub sub_id: i32,
    pub map_id: i32,
    pub remote_product_id: i32,
    pub remote_sku_id: i32,
    pub product_name: String,
    pub pic_url: String,
    pub price: Decimal,
    pub quantity: i32,
    pub amount: Decimal,
}

/// `hub_stock_hold` 行（成员侧预占）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubStockHold {
    pub id: i32,
    pub link_id: i32,
    pub hold_sn: String,
    pub product_id: i32,
    pub sku_id: i32,
    pub quantity: i32,
    pub status: i16,
    pub expire_time: i32,
    pub remote_order_id: i32,
    pub add_time: i32,
    pub update_time: i32,
}

/// `hub_xorder_route` 行（成员侧路由登记）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubXorderRoute {
    pub id: i32,
    pub link_id: i32,
    pub sub_sn: String,
    pub local_order_id: i32,
    pub local_order_sn: String,
    pub buyer_global_member_id: String,
    pub status: i16,
    pub add_time: i32,
    pub update_time: i32,
}

/// 联邦面：预占/路由共用的单行（商品级口径，skuId 本批恒 0）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct XorderWireItem {
    pub product_id: i32,
    pub sku_id: i32,
    pub quantity: i32,
    /// 预占响应回带：成员本地当前售价（店主确认价）。
    pub price: Option<Decimal>,
    /// 路由请求回带：商品名/图快照（成员落单展示用）。
    pub product_name: Option<String>,
    pub pic_url: Option<String>,
}

/// `POST /api/hub/v1/orders/reserve` 入参（中枢 → 成员）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct XorderReserveDTO {
    pub sub_sn: String,
    pub items: Vec<XorderWireItem>,
}

/// `POST /api/hub/v1/orders/reserve` 响应。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XorderReserveVO {
    pub confirmed: bool,
    pub reason: String,
    pub items: Vec<XorderWireItem>,
    pub shipping_fee: Decimal,
}

/// `POST /api/hub/v1/orders/route` 入参（中枢 → 成员）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct XorderRouteDTO {
    pub sub_sn: String,
    pub master_sn: String,
    pub items: Vec<XorderWireItem>,
    pub buyer_global_member_id: String,
    pub consignee: String,
    pub mobile: String,
    pub address: String,
    /// 子单实付（商品额 + 运费），成员落单金额以此为准。
    pub paid_amount: Decimal,
    pub shipping_fee: Decimal,
}

/// `POST /api/hub/v1/orders/route` 响应。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XorderRouteVO {
    pub local_order_id: i32,
    pub local_order_sn: String,
}

/// `POST /api/hub/v1/orders/release` / `orders/refund` 入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct XorderSubSnDTO {
    pub sub_sn: String,
    pub reason: String,
}

/// `POST /api/hub/v1/orders/subStatus` 入参（成员 → 中枢）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct XorderSubStatusDTO {
    pub sub_sn: String,
    pub status: i32,
    pub tracking_no: String,
    pub logistics_name: String,
}

/// 中枢前台面：跨店下单入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct XorderCreateDTO {
    pub items: Vec<XorderCreateItem>,
    pub address_id: i32,
}

/// 中枢前台面：跨店下单单行入参（聚合映射 + 数量）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct XorderCreateItem {
    pub map_id: i32,
    pub quantity: i32,
}

/// 中枢前台面：按 id 操作入参（取消）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct XorderIdDTO {
    pub id: i32,
}

/// 中枢前台面：子单退款入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct XorderRefundSubDTO {
    pub sub_id: i32,
    pub reason: String,
}

/// 跨店子单 VO（前台/管理端共用）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubXorderSubVO {
    pub id: i32,
    pub sub_sn: String,
    pub link_id: i32,
    pub remote_shop_id: i32,
    pub remote_shop_name: String,
    pub amount: Decimal,
    pub shipping_fee: Decimal,
    pub status: i32,
    pub remote_order_id: i32,
    pub remote_order_sn: String,
    pub tracking_no: String,
    pub logistics_name: String,
    pub items: Vec<HubXorderItemVO>,
}

/// 跨店单行 VO。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubXorderItemVO {
    pub id: i32,
    pub map_id: i32,
    pub remote_product_id: i32,
    pub product_name: String,
    pub pic_url: String,
    pub price: Decimal,
    pub quantity: i32,
    pub amount: Decimal,
}

impl From<&HubXorderItem> for HubXorderItemVO {
    fn from(r: &HubXorderItem) -> Self {
        Self {
            id: r.id,
            map_id: r.map_id,
            remote_product_id: r.remote_product_id,
            product_name: r.product_name.clone(),
            pic_url: r.pic_url.clone(),
            price: r.price,
            quantity: r.quantity,
            amount: r.amount,
        }
    }
}

/// 跨店主单 VO（前台/管理端共用，带子单展开）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubXorderVO {
    pub id: i32,
    pub xorder_sn: String,
    pub buyer_user_id: i32,
    pub global_member_id: String,
    pub product_amount: Decimal,
    pub shipping_fee: Decimal,
    pub total_amount: Decimal,
    pub pay_status: i32,
    pub status: i32,
    pub pay_sn: String,
    pub consignee: String,
    pub mobile: String,
    pub address: String,
    pub add_time: i32,
    pub pay_time: i32,
    pub subs: Vec<HubXorderSubVO>,
}

impl From<&HubXorder> for HubXorderVO {
    fn from(r: &HubXorder) -> Self {
        Self {
            id: r.id,
            xorder_sn: r.xorder_sn.clone(),
            buyer_user_id: r.buyer_user_id,
            global_member_id: r.global_member_id.clone(),
            product_amount: r.product_amount,
            shipping_fee: r.shipping_fee,
            total_amount: r.total_amount,
            pay_status: i32::from(r.pay_status),
            status: i32::from(r.status),
            pay_sn: r.pay_sn.clone(),
            consignee: r.consignee.clone(),
            mobile: r.mobile.clone(),
            address: r.address.clone(),
            add_time: r.add_time,
            pay_time: r.pay_time,
            subs: Vec::new(),
        }
    }
}

/// 管理端（中枢）：跨店主单列表查询。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubXorderListQuery {
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    pub keyword: Option<String>,
    pub status: Option<i32>,
}

/// 管理端（成员）：路由单列表查询。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubXorderRouteListQuery {
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    pub link_id: Option<i32>,
    pub status: Option<i32>,
}

/// 管理端（成员）：路由单 VO（带本地单状态供同步判定）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubXorderRouteVO {
    pub id: i32,
    pub link_id: i32,
    pub sub_sn: String,
    pub local_order_id: i32,
    pub local_order_sn: String,
    pub buyer_global_member_id: String,
    pub status: i32,
    /// 本地订单当前状态（order_status 原值，同步映射在服务层）。
    pub local_order_status: i32,
    pub local_shipping_status: i32,
    pub add_time: i32,
    pub update_time: i32,
}

impl From<&HubXorderRoute> for HubXorderRouteVO {
    fn from(r: &HubXorderRoute) -> Self {
        Self {
            id: r.id,
            link_id: r.link_id,
            sub_sn: r.sub_sn.clone(),
            local_order_id: r.local_order_id,
            local_order_sn: r.local_order_sn.clone(),
            buyer_global_member_id: r.buyer_global_member_id.clone(),
            status: i32::from(r.status),
            local_order_status: -1,
            local_shipping_status: -1,
            add_time: r.add_time,
            update_time: r.update_time,
        }
    }
}

/// 纯逻辑：子单状态合法推进（只进不退；退款/拒单/关闭为旁路终态）。
#[must_use]
pub fn xsub_transition_allowed(from: i32, to: i32) -> bool {
    if from == to {
        return true;
    }
    matches!(
        (from, to),
        (XSUB_RESERVED, XSUB_ROUTED)
            | (XSUB_ROUTED, XSUB_SHIPPED)
            | (XSUB_ROUTED, XSUB_COMPLETED)
            | (XSUB_SHIPPED, XSUB_COMPLETED)
            | (XSUB_ROUTED, XSUB_REJECTED)
            | (XSUB_RESERVED, XSUB_CLOSED)
            | (XSUB_ROUTED, XSUB_REFUNDING)
            | (XSUB_SHIPPED, XSUB_REFUNDING)
            | (XSUB_REFUNDING, XSUB_REFUNDED)
    )
}

/// 纯逻辑：主单状态聚合（全完成→完成；任一售后/拒单/退款→售后中；
/// 全路由→已全部路由；其余保持已付待路由）。
#[must_use]
pub fn xorder_aggregate_status(sub_statuses: &[i32], current: i32) -> i32 {
    if sub_statuses.is_empty() || current == XORDER_PENDING_PAY || current == XORDER_CLOSED {
        return current;
    }
    if sub_statuses.iter().all(|s| *s == XSUB_COMPLETED || *s == XSUB_REFUNDED) {
        return XORDER_COMPLETED;
    }
    if sub_statuses
        .iter()
        .any(|s| matches!(s, XSUB_REJECTED | XSUB_REFUNDING | XSUB_REFUNDED))
    {
        return XORDER_AFTERSALE;
    }
    if sub_statuses
        .iter()
        .all(|s| matches!(s, XSUB_ROUTED | XSUB_SHIPPED | XSUB_COMPLETED))
    {
        return XORDER_ROUTED;
    }
    XORDER_PAID
}

/// 纯逻辑：成员本地订单状态 → 子单状态映射（同步状态用）。
/// 本地 order_status 口径（与订单上行页一致）：0 待付款 / 1 待发货 /
/// 2 待收货 / 3 已完成 / 4 已取消 / 5 已关闭。映射：取消/关闭→拒单；
/// 完成→完成；待收货或 shipping_status=1→已发货；其余→已路由。
#[must_use]
pub fn xsub_from_local_order(order_status: i32, shipping_status: i32) -> i32 {
    if order_status == 4 || order_status == 5 {
        return XSUB_REJECTED;
    }
    if order_status == 3 {
        return XSUB_COMPLETED;
    }
    if shipping_status == 1 || order_status == 2 {
        return XSUB_SHIPPED;
    }
    XSUB_ROUTED
}

// ———————————————————— 三期④：中央账本（迁移 045） ————————————————————
//
// 设计：~/workspace/federation-phase3/DESIGN.md §4（总设计 §6.5）。
// 中枢唯一记账：成员以 wallet.post 上行记账（bizSn 幂等），中枢同事务
// 锁账户行校验后写行（balance_after 由中枢生成）。只记账不划拨真实资金。

/// 币种：积分。
pub const WALLET_POINTS: &str = "points";
/// 币种：余额。
pub const WALLET_MONEY: &str = "money";

/// 业务类型枚举（写死，扩展须同步服务层记账机）。
pub const BIZ_POINTS_GRANT: &str = "points_grant";
pub const BIZ_POINTS_SPEND: &str = "points_spend";
pub const BIZ_MONEY_CREDIT: &str = "money_credit";
pub const BIZ_MONEY_DEBIT: &str = "money_debit";
pub const BIZ_MONEY_FREEZE: &str = "money_freeze";
pub const BIZ_MONEY_CONFIRM: &str = "money_confirm";
pub const BIZ_MONEY_RELEASE: &str = "money_release";
pub const BIZ_REVERSE: &str = "reverse";
pub const BIZ_MERGE_IN: &str = "merge_in";

/// 冻结自动释放阈值（秒）：30 分钟未确认/释放由管理动作释放。
pub const WALLET_FREEZE_TTL_SECONDS: i64 = 30 * 60;

pub const MSG_WALLET_INSUFFICIENT: &str = "联邦账户余额不足";
pub const MSG_WALLET_BIZ_INVALID: &str = "联邦记账参数不合法";

/// `hub_wallet_account` 行（中枢侧联邦账户）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubWalletAccount {
    pub global_member_id: String,
    pub points_balance: i64,
    pub money_balance: Decimal,
    pub frozen_money: Decimal,
    pub version: i32,
    pub status: i16,
    pub add_time: i32,
    pub update_time: i32,
}

/// `hub_wallet_line` 行（中枢侧账本行）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubWalletLine {
    pub id: i32,
    pub global_member_id: String,
    pub link_id: i32,
    pub currency: String,
    pub biz_type: String,
    pub amount: Decimal,
    pub balance_after: Decimal,
    pub frozen_after: Decimal,
    pub biz_sn: String,
    pub ref_biz_sn: String,
    pub ref_line_id: i32,
    pub status: i16,
    pub remark: String,
    pub add_time: i32,
}

/// `hub_wallet_mirror` 行（成员侧镜像）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubWalletMirror {
    pub id: i32,
    pub link_id: i32,
    pub local_user_id: i32,
    pub global_member_id: String,
    pub biz_sn: String,
    pub currency: String,
    pub biz_type: String,
    pub amount: Decimal,
    pub balance_after: Decimal,
    pub add_time: i32,
}

/// `hub_wallet_local_state` 行（成员侧并账状态）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubWalletLocalState {
    pub local_user_id: i32,
    pub link_id: i32,
    pub global_member_id: String,
    pub merged: i16,
    pub points_merged: i64,
    pub money_merged: Decimal,
    pub add_time: i32,
    pub update_time: i32,
}

/// `POST /api/hub/v1/wallet/post` 入参（成员 → 中枢）。
/// amount 为正数幅值，方向由 bizType 决定；money_confirm / money_release /
/// reverse 的金额由被引用行推导，amount 忽略。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WalletPostDTO {
    pub biz_sn: String,
    pub biz_type: String,
    pub global_member_id: String,
    pub currency: String,
    pub amount: Decimal,
    pub ref_biz_sn: String,
    pub remark: String,
}

/// `POST /api/hub/v1/wallet/post` 响应（balance_after 一律中枢生成）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletPostVO {
    pub line_id: i32,
    pub global_member_id: String,
    pub currency: String,
    pub biz_type: String,
    /// 实际落行金额（带符号）。
    pub amount: Decimal,
    pub balance_after: Decimal,
    pub frozen_after: Decimal,
    /// 幂等重发命中既有行（未重记）。
    pub duplicated: bool,
}

/// `GET /api/hub/v1/wallet/balance` 响应。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletBalanceVO {
    pub global_member_id: String,
    pub points_balance: i64,
    pub money_balance: Decimal,
    pub frozen_money: Decimal,
    pub version: i32,
    pub status: i32,
}

impl From<&HubWalletAccount> for WalletBalanceVO {
    fn from(r: &HubWalletAccount) -> Self {
        Self {
            global_member_id: r.global_member_id.clone(),
            points_balance: r.points_balance,
            money_balance: r.money_balance,
            frozen_money: r.frozen_money,
            version: r.version,
            status: i32::from(r.status),
        }
    }
}

/// 管理端：账本行 VO（= 行原样驼峰）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubWalletLineVO {
    pub id: i32,
    pub global_member_id: String,
    pub link_id: i32,
    pub currency: String,
    pub biz_type: String,
    pub amount: Decimal,
    pub balance_after: Decimal,
    pub frozen_after: Decimal,
    pub biz_sn: String,
    pub ref_biz_sn: String,
    pub status: i32,
    pub remark: String,
    pub add_time: i32,
}

impl From<&HubWalletLine> for HubWalletLineVO {
    fn from(r: &HubWalletLine) -> Self {
        Self {
            id: r.id,
            global_member_id: r.global_member_id.clone(),
            link_id: r.link_id,
            currency: r.currency.clone(),
            biz_type: r.biz_type.clone(),
            amount: r.amount,
            balance_after: r.balance_after,
            frozen_after: r.frozen_after,
            biz_sn: r.biz_sn.clone(),
            ref_biz_sn: r.ref_biz_sn.clone(),
            status: i32::from(r.status),
            remark: r.remark.clone(),
            add_time: r.add_time,
        }
    }
}

/// 管理端：账户列表查询。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubWalletAccountListQuery {
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    pub keyword: Option<String>,
}

/// 管理端：账本行列表查询。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubWalletLineListQuery {
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    pub global_member_id: Option<String>,
    pub link_id: Option<i32>,
    pub biz_type: Option<String>,
    pub currency: Option<String>,
}

/// 管理端（中枢）：手工记账入参（bizSn 后台生成 admin- 前缀）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WalletAdminPostDTO {
    pub global_member_id: String,
    pub biz_type: String,
    pub currency: String,
    pub amount: Decimal,
    pub ref_biz_sn: String,
    pub remark: String,
}

/// 管理端（成员）：镜像列表查询。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubWalletMirrorListQuery {
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    pub local_user_id: Option<i32>,
    pub link_id: Option<i32>,
}

/// 管理端（成员）：镜像 VO。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubWalletMirrorVO {
    pub id: i32,
    pub link_id: i32,
    pub local_user_id: i32,
    pub global_member_id: String,
    pub biz_sn: String,
    pub currency: String,
    pub biz_type: String,
    pub amount: Decimal,
    pub balance_after: Decimal,
    pub add_time: i32,
}

impl From<&HubWalletMirror> for HubWalletMirrorVO {
    fn from(r: &HubWalletMirror) -> Self {
        Self {
            id: r.id,
            link_id: r.link_id,
            local_user_id: r.local_user_id,
            global_member_id: r.global_member_id.clone(),
            biz_sn: r.biz_sn.clone(),
            currency: r.currency.clone(),
            biz_type: r.biz_type.clone(),
            amount: r.amount,
            balance_after: r.balance_after,
            add_time: r.add_time,
        }
    }
}

/// 纯逻辑：bizType 与币种的合法组合（记账机前置校验）。
#[must_use]
pub fn wallet_biz_currency_valid(biz_type: &str, currency: &str) -> bool {
    match biz_type {
        BIZ_POINTS_GRANT | BIZ_POINTS_SPEND => currency == WALLET_POINTS,
        BIZ_MONEY_CREDIT | BIZ_MONEY_DEBIT | BIZ_MONEY_FREEZE | BIZ_MONEY_CONFIRM
        | BIZ_MONEY_RELEASE => currency == WALLET_MONEY,
        BIZ_MERGE_IN => currency == WALLET_POINTS || currency == WALLET_MONEY,
        BIZ_REVERSE => currency == WALLET_POINTS || currency == WALLET_MONEY,
        _ => false,
    }
}

/// 纯逻辑：并账业务单号（天然幂等：一人一连接一币种只一行）。
#[must_use]
pub fn wallet_merge_biz_sn(link_id: i32, local_user_id: i32, currency: &str) -> String {
    format!("merge-{link_id}-{local_user_id}-{currency}")
}

// ———————————————————— 三期⑤：跨站结算（迁移 046） ————————————————————
//
// 设计：~/workspace/federation-phase3/DESIGN.md §5（总设计 §11）。
// 只记账、只生成结算单，不做真实跨站资金划拨；分账只记账目、渠道未执行。
// 主体键恒为 link_id / credential_id；partner_user_id 只是快照（§11.6）。

/// 账目来源：跨店分账。
pub const SETTLE_SRC_XORDER: &str = "xorder";
/// 账目来源：跨店退款冲抵。
pub const SETTLE_SRC_XORDER_REFUND: &str = "xorder_refund";
/// 账目来源：供货成交回传。
pub const SETTLE_SRC_SUPPLY_RETURN: &str = "supply_return";
/// 账目来源：供货退款冲抵。
pub const SETTLE_SRC_SUPPLY_REFUND: &str = "supply_refund";
/// 账目来源：预付扣减。
pub const SETTLE_SRC_PREPAID_DEDUCT: &str = "prepaid_deduct";
/// 账目来源：结转入。
pub const SETTLE_SRC_CARRY_IN: &str = "carry_in";

pub const SETTLE_LINE_NORMAL: i32 = 1;
pub const SETTLE_LINE_SUSPENDED: i32 = 2;

pub const SETTLE_EXEC_PENDING: i32 = 0;
pub const SETTLE_EXEC_CONFIRMED: i32 = 1;
pub const SETTLE_EXEC_SETTLED: i32 = 2;
pub const SETTLE_EXEC_CARRIED: i32 = 3;

pub const MSG_SETTLE_RATE_PENDING: &str = "周期内存在未定费率账目行，请先设置连接覆盖费率并重算费率";
pub const MSG_SETTLE_NOT_FOUND: &str = "结算记录不存在";

/// `hub_settle_account` 行（预付账户）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubSettleAccount {
    pub id: i32,
    pub subject_type: String,
    pub subject_id: i32,
    pub balance: Decimal,
    pub warn_line: Decimal,
    pub status: i16,
    pub add_time: i32,
    pub update_time: i32,
}

/// `hub_settle_line` 行（账目行）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubSettleLine {
    pub id: i32,
    pub link_id: i32,
    pub credential_id: i32,
    pub partner_user_id: i32,
    pub source_type: String,
    pub remote_order_sn: String,
    pub item_name: String,
    pub quantity: i32,
    pub deal_amount: Decimal,
    pub supply_amount: Decimal,
    pub rate: Decimal,
    pub rate_pending: i16,
    pub self_due: Decimal,
    pub peer_due: Decimal,
    pub statement_id: i32,
    pub line_status: i16,
    pub biz_sn: String,
    pub remark: String,
    pub add_time: i32,
    pub update_time: i32,
}

/// `hub_settle_statement` 行（结算单）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct HubSettleStatement {
    pub id: i32,
    pub statement_sn: String,
    pub link_id: i32,
    pub credential_id: i32,
    pub partner_user_id: i32,
    pub period_start: i32,
    pub period_end: i32,
    pub total_deal: Decimal,
    pub self_total: Decimal,
    pub peer_total: Decimal,
    pub net_amount: Decimal,
    pub line_count: i32,
    pub diff_count: i32,
    pub statement_type: i16,
    pub self_confirm: i16,
    pub peer_confirm: i16,
    pub exec_status: i16,
    pub carry_amount: Decimal,
    pub add_time: i32,
    pub update_time: i32,
}

/// 管理端：预付账户 VO（带预警标记）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubSettleAccountVO {
    pub id: i32,
    pub subject_type: String,
    pub subject_id: i32,
    pub balance: Decimal,
    pub warn_line: Decimal,
    /// 余额低于预警线（含负数）。
    pub warning: bool,
    pub status: i32,
    pub update_time: i32,
}

impl From<&HubSettleAccount> for HubSettleAccountVO {
    fn from(r: &HubSettleAccount) -> Self {
        Self {
            id: r.id,
            subject_type: r.subject_type.clone(),
            subject_id: r.subject_id,
            balance: r.balance,
            warn_line: r.warn_line,
            warning: r.balance < r.warn_line,
            status: i32::from(r.status),
            update_time: r.update_time,
        }
    }
}

/// 管理端：账目行 VO。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubSettleLineVO {
    pub id: i32,
    pub link_id: i32,
    pub credential_id: i32,
    pub partner_user_id: i32,
    pub source_type: String,
    pub remote_order_sn: String,
    pub item_name: String,
    pub quantity: i32,
    pub deal_amount: Decimal,
    pub supply_amount: Decimal,
    pub rate: Decimal,
    pub rate_pending: i32,
    pub self_due: Decimal,
    pub peer_due: Decimal,
    pub statement_id: i32,
    pub line_status: i32,
    pub biz_sn: String,
    pub remark: String,
    pub add_time: i32,
}

impl From<&HubSettleLine> for HubSettleLineVO {
    fn from(r: &HubSettleLine) -> Self {
        Self {
            id: r.id,
            link_id: r.link_id,
            credential_id: r.credential_id,
            partner_user_id: r.partner_user_id,
            source_type: r.source_type.clone(),
            remote_order_sn: r.remote_order_sn.clone(),
            item_name: r.item_name.clone(),
            quantity: r.quantity,
            deal_amount: r.deal_amount,
            supply_amount: r.supply_amount,
            rate: r.rate,
            rate_pending: i32::from(r.rate_pending),
            self_due: r.self_due,
            peer_due: r.peer_due,
            statement_id: r.statement_id,
            line_status: i32::from(r.line_status),
            biz_sn: r.biz_sn.clone(),
            remark: r.remark.clone(),
            add_time: r.add_time,
        }
    }
}

/// 结算单 VO（管理端与联邦面共用）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubSettleStatementVO {
    pub id: i32,
    pub statement_sn: String,
    pub link_id: i32,
    pub credential_id: i32,
    pub partner_user_id: i32,
    pub period_start: i32,
    pub period_end: i32,
    pub total_deal: Decimal,
    pub self_total: Decimal,
    pub peer_total: Decimal,
    pub net_amount: Decimal,
    pub line_count: i32,
    pub diff_count: i32,
    pub statement_type: i32,
    pub self_confirm: i32,
    pub peer_confirm: i32,
    pub exec_status: i32,
    pub carry_amount: Decimal,
    pub add_time: i32,
}

impl From<&HubSettleStatement> for HubSettleStatementVO {
    fn from(r: &HubSettleStatement) -> Self {
        Self {
            id: r.id,
            statement_sn: r.statement_sn.clone(),
            link_id: r.link_id,
            credential_id: r.credential_id,
            partner_user_id: r.partner_user_id,
            period_start: r.period_start,
            period_end: r.period_end,
            total_deal: r.total_deal,
            self_total: r.self_total,
            peer_total: r.peer_total,
            net_amount: r.net_amount,
            line_count: r.line_count,
            diff_count: r.diff_count,
            statement_type: i32::from(r.statement_type),
            self_confirm: i32::from(r.self_confirm),
            peer_confirm: i32::from(r.peer_confirm),
            exec_status: i32::from(r.exec_status),
            carry_amount: r.carry_amount,
            add_time: r.add_time,
        }
    }
}

/// `POST /api/hub/v1/settle/returns` 单条回传。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettleReturnItemDTO {
    pub biz_sn: String,
    pub remote_order_sn: String,
    pub item_name: String,
    pub quantity: i32,
    pub deal_price: Decimal,
    pub supply_price: Decimal,
    pub refund: bool,
}

/// `POST /api/hub/v1/settle/returns` 入参（批量）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettleReturnsDTO {
    pub returns: Vec<SettleReturnItemDTO>,
}

/// 供货回传处理回执。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettleReturnsVO {
    pub accepted: i32,
    pub duplicated: i32,
    /// 同 bizSn 重报金额不一致被拒收的条数（差异计数）。
    pub diffs: i32,
}

/// `POST /api/hub/v1/settle/confirm` 入参（对端按单号确认）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettleConfirmDTO {
    pub statement_sn: String,
}

/// 管理端：生成结算单入参（周期为 Unix 秒）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettleGenerateDTO {
    pub link_id: i32,
    pub period_start: i32,
    pub period_end: i32,
}

/// 管理端：预付账户开户入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettleAccountOpenDTO {
    pub subject_type: String,
    pub subject_id: i32,
    pub warn_line: Decimal,
}

/// 管理端：预付账户充值登记入参（amount 可正可负，只记账不划拨）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettleAccountAdjustDTO {
    pub id: i32,
    pub amount: Decimal,
}

/// 管理端：账目行状态入参（1 恢复正常 / 2 差异挂起）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettleLineStatusDTO {
    pub id: i32,
    pub line_status: i32,
}

/// 管理端：账目行列表查询。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubSettleLineListQuery {
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    pub link_id: Option<i32>,
    pub source_type: Option<String>,
    pub line_status: Option<i32>,
    pub statement_id: Option<i32>,
}

/// 管理端：结算单列表查询。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HubSettleStatementListQuery {
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    pub link_id: Option<i32>,
    pub exec_status: Option<i32>,
}

/// 纯逻辑：连接费率解析（DESIGN §0 改口径 2 的单点收口）。
/// 连接覆盖费率 > 0 → (费率， false)；未设 → (0, true) 未定费率。
/// 后续接分销等级查表只改本函数内部，不影响账目模型。
#[must_use]
pub fn resolve_link_rate(rate_override: Decimal) -> (Decimal, bool) {
    if rate_override > Decimal::ZERO {
        (rate_override, false)
    } else {
        (Decimal::ZERO, true)
    }
}

/// 纯逻辑：跨店分账拆分（成交额 × 费率% = 中枢佣金；店主应得 = 成交 − 佣金 + 运费）。
#[must_use]
pub fn xorder_split(deal: Decimal, shipping: Decimal, rate: Decimal) -> (Decimal, Decimal) {
    let commission = deal * rate / Decimal::from(100);
    (commission, deal - commission + shipping)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn settings_default_all_off() {
        let s = effective_federation_settings(None);
        assert!(!s.enabled);
        assert!(!s.capabilities.supply_out);
        assert!(!s.modules.member);
        assert_eq!(s.module_bits(), 0);
        assert_eq!(effective_federation_settings(Some("junk")), s);
    }

    #[test]
    fn settings_sanitize_trims_and_keeps() {
        let v = json!({
            "enabled": true,
            "capabilities": {"supplyOut": true, "federation": true},
            "modules": {"product": true},
            "instanceName": "  华东总仓  ",
            "instanceBaseUrl": "https://hub.example.com/",
            "evil": 1
        });
        let s = sanitize_federation_settings(&v);
        assert!(s.enabled);
        assert!(s.capabilities.supply_out && s.capabilities.federation);
        assert!(s.modules.product && !s.modules.member);
        assert_eq!(s.instance_name, "华东总仓");
        assert_eq!(s.instance_base_url, "https://hub.example.com");
        assert_eq!(s.module_bits(), MODULE_PRODUCT);
    }

    #[test]
    fn scopes_validation_respects_modules() {
        let mut s = FederationSettings::default();
        // meta/stats 不属模块，未开模块也可签
        assert!(s.validate_scopes(SCOPE_META_READ | SCOPE_STATS_REPORT).is_ok());
        // 商品模块未开 → product.read 拒签
        assert!(s.validate_scopes(SCOPE_PRODUCT_READ).is_err());
        s.modules.product = true;
        assert!(s.validate_scopes(SCOPE_PRODUCT_READ).is_ok());
        // 会员模块未开 → member.sync 拒签
        assert!(s.validate_scopes(SCOPE_MEMBER_SYNC).is_err());
        // 未知位拒签
        assert!(s.validate_scopes(1 << 20).is_err());
    }

    #[test]
    fn scope_names_expand_in_bit_order() {
        let names = scope_names(SCOPE_PRODUCT_READ | SCOPE_META_READ | SCOPE_STATS_REPORT);
        assert_eq!(names, vec!["meta.read", "product.read", "stats.report"]);
        assert!(scope_names(0).is_empty());
    }

    #[test]
    fn heartbeat_state_thresholds() {
        assert_eq!(heartbeat_state(0, 1_000_000), "unknown");
        assert_eq!(heartbeat_state(999_500, 1_000_000), "online");
        assert_eq!(heartbeat_state(998_000, 1_000_000), "stale");
    }

    #[test]
    fn mask_secret_cases() {
        assert_eq!(mask_secret("short"), "****");
        assert_eq!(mask_secret("fk_1234567890abcdef"), "fk_1****cdef");
    }

    #[test]
    fn base_url_validation() {
        assert_eq!(
            validate_base_url(" https://a.com/ ").unwrap(),
            "https://a.com"
        );
        assert!(validate_base_url("").is_err());
        assert!(validate_base_url("a.com").is_err());
        assert!(validate_base_url("https://a .com").is_err());
    }

    #[test]
    fn settle_fields_validation() {
        assert!(validate_settle_fields("prepaid", "monthly", "own", "level").is_ok());
        assert!(validate_settle_fields("", "", "", "").is_ok());
        assert!(validate_settle_fields("bogus", "", "", "").is_err());
        assert!(validate_settle_fields("", "t_plus_7", "channel_split", "link_override").is_ok());
        assert!(validate_settle_fields("", "daily", "", "").is_err());
    }
}

#[cfg(test)]
mod member_order_tests {
    use super::*;

    fn row(gid: &str, link_id: i32, remote: i32, name: &str) -> MobileMatchRow {
        MobileMatchRow {
            global_member_id: gid.to_string(),
            link_id,
            remote_member_id: remote,
            real_name: name.to_string(),
        }
    }

    #[test]
    fn match_creates_new_when_no_existing() {
        assert_eq!(decide_member_match(&[], 7, ""), MemberMatchDecision::CreateNew);
    }

    #[test]
    fn match_links_single_global_from_other_link() {
        let rows = vec![row("gm_a", 3, 11, "")];
        assert_eq!(
            decide_member_match(&rows, 7, ""),
            MemberMatchDecision::LinkTo("gm_a".to_string())
        );
    }

    #[test]
    fn match_conflicts_when_same_link_other_remote() {
        // 同一成员实例内两个本地账号共用一个手机号：一对多，永不自动合并。
        let rows = vec![row("gm_a", 7, 11, "")];
        assert_eq!(
            decide_member_match(&rows, 7, ""),
            MemberMatchDecision::Conflict(CONFLICT_MOBILE_MULTI_REMOTE, "gm_a".to_string())
        );
    }

    #[test]
    fn match_conflicts_when_mobile_claimed_by_two_globals() {
        let rows = vec![row("gm_a", 3, 11, ""), row("gm_b", 4, 12, "")];
        assert_eq!(
            decide_member_match(&rows, 7, ""),
            MemberMatchDecision::Conflict(CONFLICT_MOBILE_CLAIMED, "gm_a".to_string())
        );
    }

    #[test]
    fn match_conflicts_on_real_name_mismatch_only() {
        let rows = vec![row("gm_a", 3, 11, "张三")];
        assert_eq!(
            decide_member_match(&rows, 7, "李四"),
            MemberMatchDecision::Conflict(CONFLICT_IDENTITY_MISMATCH, "gm_a".to_string())
        );
        // 同名或任一侧未给实名 → 正常关联
        assert_eq!(
            decide_member_match(&rows, 7, "张三"),
            MemberMatchDecision::LinkTo("gm_a".to_string())
        );
        assert_eq!(
            decide_member_match(&rows, 7, ""),
            MemberMatchDecision::LinkTo("gm_a".to_string())
        );
    }

    #[test]
    fn mask_mobile_cases() {
        assert_eq!(mask_mobile("13812345678"), "138****5678");
        assert_eq!(mask_mobile("12345"), "****");
        assert_eq!(mask_mobile(""), "****");
    }

    #[test]
    fn global_member_id_format() {
        assert_eq!(
            format_global_member_id("329e736b-90b0-4d8b"),
            "gm_329e736b-90b0-4d8b"
        );
    }

    #[test]
    fn call_log_vo_uses_camel_case_keys() {
        let row = HubCallLogRow {
            log_id: 5,
            http_status: 200,
            client_ip: "1.2.3.4".to_string(),
            ..Default::default()
        };
        let vo = HubCallLogVO::from(&row);
        let v = serde_json::to_value(&vo).expect("serialize");
        assert_eq!(v["logId"], 5);
        assert_eq!(v["httpStatus"], 200);
        assert_eq!(v["clientIp"], "1.2.3.4");
        assert!(v.get("log_id").is_none(), "VO 不得再出 snake_case 键");
    }

    #[test]
    fn sso_return_url_must_stay_on_peer_origin() {
        assert!(sso_return_url_allowed(
            "https://shop.example.com/federation/callback",
            "https://shop.example.com"
        ));
        // 尾斜杠登记不影响前缀判定
        assert!(sso_return_url_allowed(
            "https://shop.example.com/cb",
            "https://shop.example.com/"
        ));
        // 跨源回跳一律拒绝（开放重定向防线）
        assert!(!sso_return_url_allowed(
            "https://evil.example.com/cb",
            "https://shop.example.com"
        ));
        assert!(!sso_return_url_allowed("", "https://shop.example.com"));
        assert!(!sso_return_url_allowed("https://shop.example.com/cb", ""));
    }

    #[test]
    fn settle_rate_resolution_and_split() {
        let (rate, pending) = resolve_link_rate(Decimal::new(5, 0));
        assert_eq!(rate, Decimal::new(5, 0));
        assert!(!pending);
        let (rate, pending) = resolve_link_rate(Decimal::ZERO);
        assert_eq!(rate, Decimal::ZERO);
        assert!(pending);
        // 成交 200、运费 10、费率 5% → 佣金 10、店主应得 200。
        let (commission, peer) = xorder_split(
            Decimal::new(200, 0),
            Decimal::new(10, 0),
            Decimal::new(5, 0),
        );
        assert_eq!(commission, Decimal::new(10, 0));
        assert_eq!(peer, Decimal::new(200, 0));
    }

    #[test]
    fn wallet_biz_currency_matrix() {
        assert!(wallet_biz_currency_valid("points_grant", "points"));
        assert!(!wallet_biz_currency_valid("points_grant", "money"));
        assert!(wallet_biz_currency_valid("money_freeze", "money"));
        assert!(!wallet_biz_currency_valid("money_freeze", "points"));
        assert!(wallet_biz_currency_valid("merge_in", "points"));
        assert!(wallet_biz_currency_valid("merge_in", "money"));
        assert!(wallet_biz_currency_valid("reverse", "money"));
        assert!(!wallet_biz_currency_valid("unknown_biz", "money"));
        assert_eq!(wallet_merge_biz_sn(3, 42, "points"), "merge-3-42-points");
    }

    #[test]
    fn xorder_status_machines() {
        // 子单只进不退
        assert!(xsub_transition_allowed(XSUB_RESERVED, XSUB_ROUTED));
        assert!(xsub_transition_allowed(XSUB_ROUTED, XSUB_SHIPPED));
        assert!(xsub_transition_allowed(XSUB_SHIPPED, XSUB_COMPLETED));
        assert!(!xsub_transition_allowed(XSUB_SHIPPED, XSUB_ROUTED));
        assert!(!xsub_transition_allowed(XSUB_COMPLETED, XSUB_SHIPPED));
        assert!(xsub_transition_allowed(XSUB_ROUTED, XSUB_REFUNDING));
        assert!(xsub_transition_allowed(XSUB_REFUNDING, XSUB_REFUNDED));
        // 主单聚合
        assert_eq!(
            xorder_aggregate_status(&[XSUB_COMPLETED, XSUB_COMPLETED], XORDER_PAID),
            XORDER_COMPLETED
        );
        assert_eq!(
            xorder_aggregate_status(&[XSUB_ROUTED, XSUB_SHIPPED], XORDER_PAID),
            XORDER_ROUTED
        );
        assert_eq!(
            xorder_aggregate_status(&[XSUB_ROUTED, XSUB_REFUNDED], XORDER_PAID),
            XORDER_AFTERSALE
        );
        assert_eq!(
            xorder_aggregate_status(&[XSUB_RESERVED], XORDER_PAID),
            XORDER_PAID
        );
        // 未付/已关闭主单不被聚合改写
        assert_eq!(
            xorder_aggregate_status(&[XSUB_COMPLETED], XORDER_PENDING_PAY),
            XORDER_PENDING_PAY
        );
        // 本地状态映射（4=已取消 5=已关闭 3=已完成 2=待收货）
        assert_eq!(xsub_from_local_order(4, 0), XSUB_REJECTED);
        assert_eq!(xsub_from_local_order(5, 0), XSUB_REJECTED);
        assert_eq!(xsub_from_local_order(3, 1), XSUB_COMPLETED);
        assert_eq!(xsub_from_local_order(2, 1), XSUB_SHIPPED);
        assert_eq!(xsub_from_local_order(1, 0), XSUB_ROUTED);
    }

    #[test]
    fn config_package_window_and_type_rules() {
        assert!(config_package_in_window(0, 0, 100));
        assert!(config_package_in_window(50, 150, 100));
        assert!(!config_package_in_window(101, 0, 100));
        assert!(!config_package_in_window(0, 99, 100));
        assert!(config_package_type_valid("announcement"));
        assert!(config_package_type_valid("custom"));
        assert!(!config_package_type_valid("script"));
    }

    #[test]
    fn sso_exchange_vo_has_no_credential_fields() {
        let vo = SsoExchangeVO {
            global_member_id: "gm_x".to_string(),
            mobile: "13800000000".to_string(),
            nickname: "n".to_string(),
            avatar: String::new(),
        };
        let v = serde_json::to_value(&vo).expect("serialize");
        let obj = v.as_object().expect("object");
        assert_eq!(obj.len(), 4, "兑换响应只给映射与展示必需的 4 个字段");
        assert!(obj.get("password").is_none());
        assert!(obj.get("passwordHash").is_none());
    }
}
