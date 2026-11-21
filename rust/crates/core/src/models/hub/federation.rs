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
