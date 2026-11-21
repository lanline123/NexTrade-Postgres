//! 站点搬家（site_clone）模型：密钥/审计/快照/任务/映射/素材转存映射表实体，
//! 管理端 DTO/VO 与 `/clone/v1` 跨站序列化包（P1 商品/设置 + P2 装修/素材）。
//!
//! 时间字段一律秒级时间戳（与 hub 联邦表口径一致）；金额/价格用 `Decimal`
//! （与 `ProductSaveDTO` 同 serde 口径，JSON 数字）。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ———————————————————— 常量与文案 ————————————————————

/// 导出端协议版本（manifest `exporterVersion`）。导入端拒绝高于自身的大版本。
/// v2 = P2（装修/素材两域端点上线）；v1 源站无此两端点，导入端按缺失处理。
/// v3 = P3（营销模板/文章友链/邮件模板/店铺壳四域端点上线）；v1/v2 源站
/// 无 P3 端点，导入端对 P3 域记说明跳过、既有域照常。
pub const EXPORTER_VERSION: i32 = 3;

/// 授权域：商品。
pub const DOMAIN_PRODUCTS: &str = "products";
/// 授权域：设置。
pub const DOMAIN_SETTINGS: &str = "settings";
/// 授权域：装修（P2）。
pub const DOMAIN_DECORATE: &str = "decorate";
/// 授权域：素材（P2）。
pub const DOMAIN_ASSETS: &str = "assets";
/// 授权域：营销模板（P3：活动/券/卡/抽奖等模板与规则，不含已发放实例）。
pub const DOMAIN_MARKETING: &str = "marketing";
/// 授权域：文章/友链（P3）。
pub const DOMAIN_CONTENT: &str = "content";
/// 授权域：邮件/消息模板（P3：文本模板与通知渠道开关，不含公众号模板 ID）。
pub const DOMAIN_MAIL: &str = "mail";
/// 授权域：店铺壳（P3：店铺基础资料重建，不含商户账号/资金/费率）。
pub const DOMAIN_SHOPS: &str = "shops";
/// 全部可授权域（P1 两域 + P2 两域 + P3 四域）。
pub const ALL_DOMAINS: [&str; 8] = [
    DOMAIN_PRODUCTS,
    DOMAIN_SETTINGS,
    DOMAIN_DECORATE,
    DOMAIN_ASSETS,
    DOMAIN_MARKETING,
    DOMAIN_CONTENT,
    DOMAIN_MAIL,
    DOMAIN_SHOPS,
];

pub const MSG_CLONE_UNAUTHORIZED: &str = "搬家密钥无效";
pub const MSG_CLONE_DISABLED: &str = "搬家密钥已停用或吊销";
pub const MSG_CLONE_EXPIRED: &str = "搬家密钥已过期";
pub const MSG_CLONE_SCOPE_DENIED: &str = "搬家密钥未授权该域";
pub const MSG_CLONE_RATE_LIMITED: &str = "搬家密钥调用超限";
pub const MSG_CLONE_CALLS_EXHAUSTED: &str = "搬家密钥拉取次数已用完";
pub const MSG_KEY_NOT_FOUND: &str = "搬家密钥不存在";
pub const MSG_KEY_NAME_REQUIRED: &str = "密钥名称不能为空";

// ———————————————————— 表实体 ————————————————————

/// `site_clone_key` 行（搬家密钥；secret 只存哈希）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct SiteCloneKey {
    pub key_id: i32,
    pub key_code: String,
    pub key_name: String,
    pub secret_hash: String,
    /// 授权域逗号分隔：`products,settings`。
    pub scopes: String,
    pub source_shop_id: i32,
    pub expire_time: i32,
    pub max_calls: i32,
    pub used_calls: i32,
    /// 1 启用 / 0 停用 / -1 已吊销。
    pub status: i16,
    pub last_used_time: i32,
    pub created_by: i32,
    pub created_by_name: String,
    pub add_time: i32,
    pub update_time: i32,
}

impl SiteCloneKey {
    /// 是否授权某域。
    #[must_use]
    pub fn has_domain(&self, domain: &str) -> bool {
        self.scopes.split(',').any(|s| s.trim() == domain)
    }

    /// 授权域列表。
    #[must_use]
    pub fn domain_list(&self) -> Vec<String> {
        self.scopes
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

/// `site_clone_call_log` 行（拉取审计）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct SiteCloneCallLog {
    pub log_id: i32,
    pub key_id: i32,
    pub key_code: String,
    pub endpoint: String,
    pub domain: String,
    pub http_status: i32,
    pub result_count: i32,
    pub client_ip: String,
    pub duration_ms: i32,
    pub add_time: i32,
}

/// `site_clone_snapshot` 行（导入前设置快照）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct SiteCloneSnapshot {
    pub snapshot_id: i32,
    pub job_id: i32,
    pub operator_id: i32,
    pub operator_name: String,
    pub source_label: String,
    pub fingerprint: String,
    /// 快照 JSON：`{groups:{key:原值}, blocks:{code:原值或 null}}`；
    /// P3 起可另含 `mail` / `salesman` 两节（覆盖前的模板与分销规则
    /// 原值，回滚同通道还原，见 rollback_snapshot）。
    pub payload: String,
    pub group_keys: String,
    /// 1 有效 / 2 已回滚。
    pub status: i16,
    pub rollback_time: i32,
    pub add_time: i32,
}

/// `site_clone_import_job` 行（导入任务）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct SiteCloneImportJob {
    pub job_id: i32,
    pub fingerprint: String,
    pub source_origin: String,
    pub source_label: String,
    pub target_shop_id: i32,
    pub domains: String,
    pub options: String,
    /// running / success / partial / failed。
    pub status: String,
    pub snapshot_id: i32,
    pub counts: String,
    pub failures: String,
    pub operator_id: i32,
    pub operator_name: String,
    pub undone: i16,
    pub add_time: i32,
    pub finish_time: i32,
}

/// `site_clone_asset_map` 行（P2 素材转存映射：源 URL → 本站 URL）。
/// 以口令指纹隔离：同一源文件在同一来源下只转存一次，跨任务复用。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct SiteCloneAssetMap {
    pub map_id: i32,
    pub fingerprint: String,
    /// 源站文件 URL（完整 http(s) 地址）。
    pub source_url: String,
    /// 转存后本站 URL（存储后端返回的访问地址）。
    pub local_url: String,
    /// 媒体类别：pic / video / file（按扩展名粗分，仅统计与展示用）。
    pub media_kind: String,
    /// 素材库导入时新建的 `gallery_pic.pic_id`（仅图片库条目有值，其余 0）。
    pub target_pic_id: i32,
    /// 首次转存的导入任务 id。
    pub job_id: i32,
    pub add_time: i32,
}

/// `site_clone_import_map` 行（去重映射）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct SiteCloneImportMap {
    pub map_id: i32,
    pub fingerprint: String,
    pub domain: String,
    pub source_id: i32,
    pub source_key: String,
    pub target_id: i32,
    pub job_id: i32,
    pub add_time: i32,
}

// ———————————————————— 管理端 DTO / VO ————————————————————

/// 密钥列表查询。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneKeyListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 新建密钥入参。`origin` 为本站对外地址（仅用于拼搬家口令，不落库）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneKeyCreateDTO {
    pub key_name: String,
    /// 授权域：`products` / `settings` 数组。
    pub domains: Vec<String>,
    pub source_shop_id: Option<i32>,
    /// 有效期天数（0/缺省 = 长期）。
    pub expire_days: Option<i32>,
    /// 拉取总次数上限（0/缺省 = 不限）。
    pub max_calls: Option<i32>,
    pub origin: Option<String>,
}

/// 编辑密钥入参（名称/权限/源店铺/状态启停；吊销走 revoke）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneKeyUpdateDTO {
    pub key_id: i32,
    pub key_name: Option<String>,
    pub domains: Option<Vec<String>>,
    pub source_shop_id: Option<i32>,
    /// 1 启用 / 0 停用。
    pub status: Option<i16>,
}

/// 单 id 入参（吊销/重置/详情等）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneIdDTO {
    pub id: Option<i32>,
    pub key_id: Option<i32>,
    pub job_id: Option<i32>,
    pub snapshot_id: Option<i32>,
    /// 重置时拼口令用的本站对外地址。
    pub origin: Option<String>,
}

impl CloneIdDTO {
    /// 归一化目标 id（多别名取第一个有值者）。
    #[must_use]
    pub fn target(&self) -> Option<i32> {
        self.id
            .or(self.key_id)
            .or(self.job_id)
            .or(self.snapshot_id)
    }
}

/// 密钥列表行 VO（不含 secret/哈希）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneKeyVO {
    pub key_id: i32,
    pub key_code: String,
    pub key_name: String,
    pub domains: Vec<String>,
    pub source_shop_id: i32,
    pub expire_time: i32,
    pub max_calls: i32,
    pub used_calls: i32,
    pub status: i16,
    pub last_used_time: i32,
    pub created_by_name: String,
    pub add_time: i32,
    pub update_time: i32,
}

impl From<&SiteCloneKey> for CloneKeyVO {
    fn from(k: &SiteCloneKey) -> Self {
        Self {
            key_id: k.key_id,
            key_code: k.key_code.clone(),
            key_name: k.key_name.clone(),
            domains: k.domain_list(),
            source_shop_id: k.source_shop_id,
            expire_time: k.expire_time,
            max_calls: k.max_calls,
            used_calls: k.used_calls,
            status: k.status,
            last_used_time: k.last_used_time,
            created_by_name: k.created_by_name.clone(),
            add_time: k.add_time,
            update_time: k.update_time,
        }
    }
}

/// 新建/重置密钥的一次性返回：明文 secret 与整段搬家口令只在此出现一次。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneKeySecretVO {
    pub key_id: i32,
    pub key_code: String,
    pub secret: String,
    /// `MV1.` + base64url(JSON{origin,keyId,secret})；origin 缺省时为空串。
    pub token: String,
}

/// 审计日志查询。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneCallLogListQuery {
    pub key_id: Option<i32>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 任务列表查询。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneJobListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 导入来源入参：口令整段粘贴，或 origin+keyId+secret 三栏手动填。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneSourceDTO {
    pub token: Option<String>,
    pub origin: Option<String>,
    pub key_id: Option<String>,
    pub secret: String,
}

/// 导入预览入参。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ClonePreviewDTO {
    pub source: CloneSourceDTO,
}

/// 导入执行入参。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneExecuteDTO {
    pub source: CloneSourceDTO,
    /// 承接店铺（缺省 0 = 平台自营）。
    pub target_shop_id: Option<i32>,
    pub import_products: Option<bool>,
    pub import_settings: Option<bool>,
    /// 导入装修（P2：页面/离散组件/导航/楼层/移动分类导航，新建为未发布页）。
    pub import_decorate: Option<bool>,
    /// 导入素材并转存（P2：素材库转存本站存储 + 全站 URL 重写，
    /// 含 P1 已导入商品/设置/外链登记素材的回溯转存）。
    pub import_assets: Option<bool>,
    /// 导入营销模板（P3：券/秒杀/拼团/限时折扣/促销/礼品卡/抽奖/积分
    /// 兑换/赠品/充值档位/签到规则 + promotion 镜像行；一律停用态落库，
    /// 已发放实例不搬）。
    pub import_marketing: Option<bool>,
    /// 导入文章/友链（P3：文章分类按名对齐、文章新建为不显示）。
    pub import_content: Option<bool>,
    /// 导入邮件/消息模板（P3：按 template_code / message_id 覆盖，
    /// 覆盖前原值进导入前快照，可一键回滚）。
    pub import_mail: Option<bool>,
    /// 导入店铺壳（P3：为源站其他店铺在本站新建暂停运营的壳店铺，
    /// 不含商户账号/资金/费率；含分销规则 salesman_config 按店对齐）。
    pub import_shops: Option<bool>,
    /// 离线包内容（P3 兜底通道）：粘贴/上传导出的离线包 JSON 时走离线
    /// 导入——逐文件验 sha + HMAC 验签通过后与在线导入共用同一编排，
    /// `source` 仍须提供对应密钥三元组（验签与指纹都依赖它）。
    pub package: Option<serde_json::Value>,
    /// 勾选导入的设置组 key（缺省 = 全部非风险组）；风险组须同时置
    /// `include_risk_groups = true` 才生效。
    pub settings_groups: Option<Vec<String>>,
    pub include_risk_groups: Option<bool>,
    /// 商品状态：true 保持源站状态，false（缺省）一律下架（防呆口径）。
    pub keep_product_status: Option<bool>,
    /// true 时已导入商品仍再新增一份（缺省 false = 跳过，防翻倍）。
    pub force_duplicate: Option<bool>,
}

/// 解析后的来源凭据（服务层内部 + 预览/执行共用）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneSourceCred {
    pub origin: String,
    pub key_code: String,
    pub secret: String,
    /// 口令指纹 SHA-256(keyCode + secret)，去重/快照/任务的关联键。
    pub fingerprint: String,
}

/// 导入预览 VO。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClonePreviewVO {
    pub source_label: String,
    pub source_origin: String,
    pub exporter_version: i32,
    pub key_domains: Vec<String>,
    pub source_shop_id: i32,
    pub product_total: i64,
    pub product_already_imported: i64,
    pub category_total: i64,
    pub brand_total: i64,
    pub shipping_tpl_total: i64,
    /// 装修页总数（P2；源站未授权装修域或版本过低时为 0）。
    pub decorate_total: i64,
    /// 离散组件总数（P2）。
    pub discrete_total: i64,
    /// 导航/楼层/移动分类导航合计条数（P2，随装修域一并搬）。
    pub nav_total: i64,
    /// 素材图片总数（P2）。
    pub asset_pic_total: i64,
    /// 素材视频总数（P2）。
    pub asset_video_total: i64,
    /// 已导入过的装修页数（默认跳过，防重复建页）。
    pub decorate_already_imported: i64,
    /// 营销模板活动总数（P3；未授权营销域或源站版本过低时为 0）。
    pub marketing_total: i64,
    /// 文章总数（P3）。
    pub article_total: i64,
    /// 友链总数（P3）。
    pub friend_link_total: i64,
    /// 邮件模板 + 消息类型合计条数（P3）。
    pub mail_template_total: i64,
    /// 店铺壳总数（P3，源站全部店铺）。
    pub shop_shell_total: i64,
    /// 已导入过的营销模板数（映射合计，默认跳过）。
    pub marketing_already_imported: i64,
    /// 已导入过的文章数（默认跳过）。
    pub content_already_imported: i64,
    /// 已建过的店铺壳数（默认跳过）。
    pub shops_already_imported: i64,
    /// 设置组清单（含风险标记与字段数）。
    pub settings_groups: Vec<CloneSettingsGroupInfo>,
    /// 将整块覆盖的独立 JSON 设置块 code 列表。
    pub settings_blocks: Vec<String>,
    pub warnings: Vec<String>,
}

/// 预览中的一个设置组。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneSettingsGroupInfo {
    pub key: String,
    pub label: String,
    /// 风险组（费率类）：默认不勾选，须显式纳入。
    pub risk: bool,
    pub field_count: i32,
}

// ———————————————————— /clone/v1 跨站序列化包 ————————————————————

/// `GET /clone/v1/manifest` 返回。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneManifestVO {
    pub site_name: String,
    pub exporter_version: i32,
    pub key_name: String,
    pub domains: Vec<String>,
    pub source_shop_id: i32,
    pub product_total: i64,
    pub category_total: i64,
    pub brand_total: i64,
    pub shipping_tpl_total: i64,
    pub settings_group_total: i32,
    /// 装修页总数（P2，源店铺口径；未授权装修域时为 0）。
    pub decorate_total: i64,
    /// 离散组件总数（P2）。
    pub discrete_total: i64,
    /// PC 导航 + 分类楼层 + 移动分类导航合计（P2，平台级结构、无店铺维）。
    pub nav_total: i64,
    /// 素材图片总数（P2，源店铺 + 平台素材）。
    pub asset_pic_total: i64,
    /// 素材视频总数（P2）。
    pub asset_video_total: i64,
    /// 营销模板活动总数（P3：券+秒杀+拼团+限时折扣+促销+礼品卡组+
    /// 抽奖+积分兑换，不含已发放实例；未授权营销域时为 0）。
    pub marketing_total: i64,
    /// 文章总数（P3；未授权文章域时为 0）。
    pub article_total: i64,
    /// 友链总数（P3）。
    pub friend_link_total: i64,
    /// 邮件模板 + 消息类型合计（P3；未授权邮件域时为 0）。
    pub mail_template_total: i64,
    /// 店铺壳总数（P3：源站全部未删除店铺；未授权店铺壳域时为 0）。
    pub shop_shell_total: i64,
    /// 豁免声明（明示不导出的域，供导入端预览展示）。
    pub exempt_note: String,
}

/// `GET /clone/v1/catalog`：分类 + 品牌 + 运费模板（结构域）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneCatalogVO {
    pub categories: Vec<CloneCategory>,
    pub brands: Vec<CloneBrand>,
    pub shipping_types: Vec<CloneShippingType>,
    pub shipping_tpls: Vec<CloneShippingTpl>,
}

/// 分类（全表平铺，parent_id 表达树形；导入按名称路径对齐）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneCategory {
    pub category_id: i32,
    pub category_name: String,
    pub parent_id: i32,
    pub sort_order: i32,
    pub is_show: i32,
    pub keywords: String,
    pub category_desc: String,
    pub measure_unit: String,
    pub seo_title: String,
    pub short_name: String,
    pub category_pic: String,
    pub category_ico: String,
    pub is_hot: i32,
    pub search_keywords: String,
    pub seo_identification: Option<String>,
}

/// 品牌（导入按名对齐）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneBrand {
    pub brand_id: i32,
    pub brand_name: String,
    pub brand_logo: Option<String>,
    pub brand_desc: Option<String>,
    pub site_url: String,
    pub sort_order: i32,
    pub is_show: i32,
    pub first_word: Option<String>,
    pub brand_en_name: Option<String>,
}

/// 配送方式（运费模板的父级）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneShippingType {
    pub shipping_type_id: i32,
    pub shipping_type_name: String,
    pub shipping_type_desc: String,
    pub shipping_time_desc: String,
    pub is_support_cod: i32,
    pub sort_order: i32,
    pub shop_id: i32,
}

/// 运费模板（含计费明细行）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneShippingTpl {
    pub shipping_tpl_id: i32,
    pub shipping_tpl_name: String,
    pub shipping_time: String,
    pub is_free: i32,
    pub pricing_type: i32,
    pub is_default: i32,
    pub shop_id: i32,
    pub infos: Vec<CloneShippingTplInfo>,
}

/// 运费模板计费明细行。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneShippingTplInfo {
    pub shipping_type_id: i32,
    pub is_free: i32,
    pub is_default: i32,
    pub region_data: Option<String>,
    pub start_number: Decimal,
    pub start_price: Decimal,
    pub add_number: Decimal,
    pub add_price: Decimal,
    pub free_price: Decimal,
    pub pricing_type: i32,
}

/// `GET /clone/v1/products` 单品全保真序列化（导入端据此组 `ProductSaveDTO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneProduct {
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
    /// 分类名称路径（如 `电池/锂电池`），导入对齐的主依据。
    pub category_path: String,
    pub brand_id: i32,
    pub brand_name: String,
    pub shipping_tpl_id: i32,
    pub shipping_tpl_name: String,
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
    /// 商品服务承诺（导入按名对齐到本站 product_services，缺失则新建）。
    pub services: Vec<CloneProductService>,
    pub skus: Vec<CloneSku>,
    pub attributes: Vec<CloneAttribute>,
    pub gallery: Vec<CloneGalleryPic>,
    pub videos: Vec<CloneVideo>,
}

/// 商品服务承诺序列化（product_services 行的子集）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneProductService {
    pub product_service_id: i32,
    pub product_service_name: String,
    pub product_service_desc: String,
    pub ico_img: String,
    pub sort_order: i32,
    pub default_on: i32,
}

/// SKU 序列化。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneSku {
    pub sku_id: i32,
    pub sku_value: String,
    pub sku_data: Option<String>,
    pub sku_sn: Option<String>,
    pub sku_stock: Option<i32>,
    pub sku_tsn: Option<String>,
    pub sku_weight: Option<Decimal>,
    pub sku_volume: Option<Decimal>,
    pub sku_price: Option<Decimal>,
    pub is_enabled: i32,
    pub is_default: i32,
    pub is_inquiry: i32,
    pub market_price: Option<Decimal>,
    pub cost_price: Option<Decimal>,
    pub delivery_service_fee_type: i32,
    pub delivery_service_fee_value: Decimal,
    pub server_service_fee_type: i32,
    pub server_service_fee_value: Decimal,
}

/// 商品属性行（普通/规格/附加，attr_type 区分）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneAttribute {
    pub attr_type: i32,
    pub attr_name: String,
    pub attr_value: String,
    pub attr_price: Option<Decimal>,
    pub attr_color: Option<String>,
    pub attr_pic: Option<String>,
    pub attr_pic_thumb: Option<String>,
    pub sort_order: Option<i32>,
}

/// 商品图库行（导入登记为 external 素材，URL 直用；转存为 P2）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneGalleryPic {
    pub pic_url: String,
    pub pic_desc: String,
    pub pic_thumb: String,
    pub pic_original: String,
    pub pic_large: String,
    pub sort_order: i32,
}

/// 商品视频行。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneVideo {
    pub video_id: i32,
    pub video_url: String,
    pub video_cover: String,
    pub format: String,
}

/// `GET /clone/v1/settings` 设置包（仅白名单组/块，导出与导入两侧同表过滤）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneSettingsPackage {
    pub groups: Vec<CloneSettingsGroup>,
    /// 独立 JSON 块 / 原始单行 code（含协议文本）：`{code, value}`，
    /// value 为 JSON 值或字符串。
    pub blocks: Vec<CloneSettingsBlock>,
}

/// 设置包中的一个规格组。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneSettingsGroup {
    /// 注册表 key（如 `themeStyle`）。
    pub key: String,
    pub data: serde_json::Value,
}

/// 设置包中的一个独立块。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneSettingsBlock {
    pub code: String,
    pub value: serde_json::Value,
}

/// 商品分页查询（/clone/v1/products）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneProductPageQuery {
    pub page: Option<u64>,
    pub size: Option<u64>,
}

// ———————————————————— P2：装修与素材序列化包 ————————————————————

/// `GET /clone/v1/decorate` 返回：装修页 + 离散组件 + 导航/楼层/移动分类导航。
/// 页面 `data`/`draft_data` 为装修 JSON 原文（字符串透传，变换在导入端做）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneDecoratePackage {
    pub pages: Vec<CloneDecoratePage>,
    pub discrete: Vec<CloneDiscrete>,
    pub navigations: Vec<ClonePcNavigation>,
    pub cat_floors: Vec<ClonePcCatFloor>,
    pub mobile_cat_navs: Vec<CloneMobileCatNav>,
}

/// 装修页序列化（`decorate` 行；导入端一律新建为未发布页、非首页）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneDecoratePage {
    pub decorate_id: i32,
    pub decorate_title: Option<String>,
    /// 已发布页面数据（装修 JSON 原文）。
    pub data: Option<String>,
    /// 草稿页面数据（装修 JSON 原文）。
    pub draft_data: Option<String>,
    /// 1 H5 / 2 PC / 3 移动端壳页。
    pub decorate_type: Option<i32>,
    pub page_type: i32,
    pub is_home: Option<i32>,
    pub status: Option<i32>,
    pub locale_id: Option<i32>,
    /// 父页 decorate_id（多语言子页），导入端按映射重建。
    pub parent_id: Option<i32>,
    pub sort_order: Option<i32>,
    /// 绑定的 H5 页 decorate_id（decorate_type=3），导入端按映射重建。
    pub binded_h5_id: Option<i32>,
    /// 分享信息 JSON（内含图片 URL，导入端做 URL 重写）。
    pub share_info: Option<String>,
}

/// 离散组件序列化（`decorate_discrete` 行，如开屏广告）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneDiscrete {
    pub id: i32,
    pub decorate_sn: String,
    pub decorate_name: Option<String>,
    /// 组件数据 JSON 原文。
    pub data: String,
}

/// PC 导航序列化（`pc_navigation` 行，平台级、无店铺维）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct ClonePcNavigation {
    pub id: i32,
    pub title: String,
    pub is_show: i32,
    pub sort_order: i32,
    pub is_blank: i32,
    pub link: String,
    pub r#type: i32,
    pub parent_id: i32,
    pub icon: String,
    pub entry_type: String,
    pub qr_type: String,
    pub qr_image: String,
}

/// PC 分类楼层序列化（`pc_cat_floor` 行；category_ids/brand_ids 为 JSON 数组原文）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct ClonePcCatFloor {
    pub cat_floor_id: i32,
    pub cat_floor_name: String,
    pub category_ids: String,
    pub category_names: String,
    pub floor_ico: String,
    pub hot_cat: String,
    pub is_show: i32,
    pub sort_order: i32,
    pub floor_ico_font: String,
    pub brand_ids: String,
}

/// 移动分类导航序列化（`mobile_cat_nav` 行；img_url 以全角「｜」分隔多图）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneMobileCatNav {
    pub mobile_cat_nav_id: i32,
    pub category_id: i32,
    pub cat_color: String,
    pub child_cat_ids: String,
    pub brand_ids: String,
    pub is_show: i32,
    pub img_url: String,
    pub sort_order: i32,
    pub cat_name_alias: Option<String>,
}

/// `GET /clone/v1/assets` 返回：素材库分组与条目（源 URL 直给，
/// 转存由导入端服务端下载后上传本站存储，不需源站额外凭证）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneAssetsPackage {
    pub groups: Vec<CloneAssetGroup>,
    pub pics: Vec<CloneAssetPic>,
    pub video_groups: Vec<CloneAssetVideoGroup>,
    pub videos: Vec<CloneAssetVideo>,
}

/// 图片分组序列化（`gallery` 行）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneAssetGroup {
    pub gallery_id: i32,
    pub parent_id: i32,
    pub gallery_name: String,
    pub gallery_sort: i32,
    pub gallery_thumb: String,
    pub shop_id: i32,
}

/// 图片素材序列化（`gallery_pic` 行子集）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneAssetPic {
    pub pic_id: i32,
    pub gallery_id: i32,
    pub shop_id: i32,
    pub pic_url: String,
    pub pic_name: String,
    pub pic_thumb: String,
    pub pic_width: Option<i32>,
    pub pic_height: Option<i32>,
    pub pic_size: Option<i64>,
    pub file_type: Option<String>,
    pub pic_sort: Option<i32>,
}

/// 视频分组序列化（`gallery_video` 行）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneAssetVideoGroup {
    pub id: i32,
    pub parent_id: i32,
    pub name: String,
    pub sort: i32,
    pub shop_id: i32,
}

/// 视频素材序列化（`gallery_video_info` 行子集）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneAssetVideo {
    pub id: i32,
    pub gallery_id: i32,
    pub shop_id: i32,
    pub video_url: String,
    pub video_name: String,
    pub video_cover: Option<String>,
    pub format: Option<String>,
    pub video_first_frame: Option<String>,
    pub duration: Option<String>,
    pub size: Option<String>,
    pub description: Option<String>,
}

// ———————————————————— P3：营销模板序列化包 ————————————————————

/// `GET /clone/v1/marketing` 返回：营销「模板/规则」全集（各活动的定义行
/// 与明细行），**不含任何已发放/已参与实例**（user_coupon、gift_cards、
/// groupon_promotion_record、lottery_record 等实例表不设端点、不导出）。
/// 引用口径：商品/分类/品牌引用在导入端按 import_map 重写；SKU 引用随
/// `sku_directory` 给出源 SKU 的商品归属与货号，导入端按货号在目标商品
/// 的 SKU 列表中对齐（对不齐的明细行跳过并在任务报告列明）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneMarketingPackage {
    pub coupons: Vec<CloneCoupon>,
    pub seckills: Vec<CloneSeckill>,
    pub seckill_items: Vec<CloneSeckillItem>,
    pub groupons: Vec<CloneGroupon>,
    pub groupon_items: Vec<CloneGrouponItem>,
    pub groupon_details: Vec<CloneGrouponDetail>,
    pub groupon_shops: Vec<CloneGrouponShops>,
    pub time_discounts: Vec<CloneTimeDiscount>,
    pub time_discount_items: Vec<CloneTimeDiscountItem>,
    pub product_promotions: Vec<CloneProductPromotion>,
    /// `promotion` 统一镜像行（relation_id 指向上面各活动，导入端按
    /// type 分发到对应活动映射重写；源站没有镜像行的活动不凭空造行）。
    pub promotions: Vec<ClonePromotion>,
    pub points_exchanges: Vec<ClonePointsExchange>,
    pub product_gifts: Vec<CloneProductGift>,
    pub recharge_settings: Vec<CloneRechargeSetting>,
    pub sign_in_settings: Vec<CloneSignInSetting>,
    pub gift_card_groups: Vec<CloneGiftCardGroup>,
    pub gift_card_products: Vec<CloneGiftCardProduct>,
    pub gift_scope_products: Vec<CloneGiftScopeRef>,
    pub gift_scope_categories: Vec<CloneGiftScopeRef>,
    pub lotteries: Vec<CloneLotteryActivity>,
    pub lottery_prizes: Vec<CloneLotteryPrize>,
    /// 被上述任一行引用的源 SKU 目录（sku_id → 商品 + 货号）。
    pub sku_directory: Vec<CloneSkuRef>,
}

/// 优惠券模板序列化（`coupon` 行，导出侧已过滤 is_delete=1）。
/// 导入防呆：`is_show` 强制 0（不公开领取）、`send_num` 归 0。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneCoupon {
    pub coupon_id: i32,
    pub coupon_name: String,
    pub coupon_money: Decimal,
    pub coupon_discount: Decimal,
    pub coupon_desc: String,
    pub coupon_type: i32,
    /// 使用范围：1 分类 / 2 品牌 / 3 指定商品 / 4 排除商品 / 其他 全场。
    pub send_range: i32,
    /// 范围 id 列表原文（Java List.toString，如 `[1, 2]`），按
    /// send_range 在导入端分发到分类/品牌/商品映射重写。
    pub send_range_data: String,
    pub min_order_amount: Decimal,
    pub send_start_date: i32,
    pub send_end_date: i32,
    pub send_type: i32,
    pub use_day: i32,
    pub use_start_date: i32,
    pub use_end_date: i32,
    pub is_show: i32,
    pub is_global: i32,
    pub is_new_user: i32,
    pub enabled_click_get: i32,
    /// 会员等级 id 列表原文（等级为两站同源体系字典，按原值搬运）。
    pub limit_user_rank: Option<String>,
    pub shop_id: Option<i32>,
    pub limit_num: Option<i32>,
    pub delay_day: i32,
    pub max_order_amount: Decimal,
    pub coupon_unit: i32,
    pub reduce_type: i32,
}

/// 秒杀活动序列化（`seckill` 行）。无启停标志，生效与否由时间窗决定，
/// 时间原样搬运（模板口径）；其 promotion 镜像行 is_available 强制 0。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneSeckill {
    pub seckill_id: i32,
    pub seckill_name: Option<String>,
    pub seckill_start_time: Option<i32>,
    pub seckill_end_time: Option<i32>,
    pub seckill_limit_num: Option<i32>,
    pub product_id: Option<i32>,
    pub shop_id: i32,
}

/// 秒杀明细序列化（`seckill_item` 行；导入时 seckill_sales 归 0）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneSeckillItem {
    pub rec_id: i32,
    pub seckill_id: Option<i32>,
    pub product_id: Option<i32>,
    pub sku_id: Option<i32>,
    pub seckill_price: Option<Decimal>,
    pub seckill_stock: Option<i32>,
    pub seckill_start_time: Option<i32>,
    pub seckill_end_time: Option<i32>,
}

/// 拼团活动序列化（`groupon_promotion` 行，i64 主键；旧 `groupon` 表
/// 无任何在役消费者，不导出）。时间原样搬运，镜像行停用（同秒杀口径）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneGroupon {
    pub groupon_promotion_id: i64,
    pub name: String,
    pub start_time: i64,
    pub end_time: i64,
    pub group_type: i32,
    pub expiration_time: i64,
    pub groupon_image: Option<String>,
    pub pc_groupon_image: Option<String>,
    pub share_image: Option<String>,
    pub groupon_explain: Option<String>,
    pub shop_id: Option<i32>,
}

/// 拼团商品行序列化（`groupon_promotion_item` 行）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneGrouponItem {
    pub groupon_promotion_item_id: i64,
    pub groupon_promotion_id: i64,
    pub product_id: i64,
    pub group_type: i32,
    /// 成团人数阶梯 JSON 原文（无外部引用，透传）。
    pub group_num: String,
    pub order_limit_num: i32,
    pub is_auto_group: i32,
    pub min_group_num: i32,
    pub sort: i32,
    pub min_group_price: Option<Decimal>,
}

/// 拼团规格价行序列化（`groupon_promotion_detail` 行，SKU 级价格）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneGrouponDetail {
    pub groupon_detail_id: i64,
    pub groupon_promotion_id: i64,
    pub groupon_promotion_item_id: i64,
    pub product_id: i64,
    pub sku_id: i64,
    pub group_price: String,
    pub limit_num: i32,
    pub is_join: i32,
}

/// 拼团适用店铺行序列化（`groupon_promotion_shops` 行；shop_ids 为
/// JSON 店铺 id 列表原文，导入端按店铺壳映射重写、对不齐剔除）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneGrouponShops {
    pub groupon_shop_id: i32,
    pub groupon_promotion_id: Option<i32>,
    pub shop_ids: Option<String>,
}

/// 限时折扣序列化（`time_discount` 行）。时间在明细行上，原样搬运。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneTimeDiscount {
    pub discount_id: i32,
    pub promotion_name: String,
    pub start_time: i32,
    pub end_time: i32,
    pub shop_id: i32,
}

/// 限时折扣明细序列化（`time_discount_item` 行；sku_ids 为 JSON 列表
/// 原文，空 = 全部规格；导入端逐个按货号对齐，对不齐剔除；源非空而
/// 对齐后全空时整行跳过——避免语义放大为全部规格）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneTimeDiscountItem {
    pub item_id: i32,
    pub discount_id: i32,
    pub start_time: i32,
    pub end_time: i32,
    pub product_id: i32,
    pub sku_ids: Option<String>,
    pub value: Decimal,
    pub discount_type: i32,
}

/// 满减/折扣/满赠序列化（`product_promotion` 行）。
/// 导入防呆：`is_available` 强制 0。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneProductPromotion {
    pub promotion_id: i32,
    pub promotion_name: String,
    pub start_time: i32,
    pub end_time: i32,
    /// 会员等级列表原文（同源字典，按原值搬运）。
    pub limit_user_rank: String,
    /// 范围：0 全部 / 3 指定商品 / 4 排除商品（本表仅此三值有定义）。
    pub range: i32,
    /// 范围商品 id 列表原文（JSON），按商品映射重写。
    pub range_data: String,
    pub min_order_amount: Decimal,
    pub max_order_amount: Decimal,
    /// 1 满减 / 2 折扣 / 3 满赠。
    pub promotion_type: i32,
    /// 阶梯规则 JSON 原文（数组，元素含 gift_id → 按赠品映射重写）。
    pub promotion_type_data: String,
    pub is_available: i32,
    pub sort_order: i32,
    pub shop_id: i32,
    pub rules_type: i32,
    pub unit: i32,
}

/// `promotion` 统一活动镜像行序列化。导入防呆：`is_available` 强制 0、
/// `is_delete` 强制 0；relation_id 按 type 重写（1 秒杀 / 2 优惠券 /
/// 3、4、5 满减折扣满赠（product_promotion）/ 6 限时折扣 / 8 抽奖），
/// 关系对不齐整行跳过；range_data / sku_ids 按同名字段口径重写，
/// member_* 会员人群字段为同源字典口径、按原值搬运。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct ClonePromotion {
    pub promotion_id: i32,
    pub promotion_name: String,
    pub start_time: i32,
    pub end_time: i32,
    pub r#type: i32,
    pub shop_id: i32,
    pub relation_id: Option<i32>,
    pub range: Option<i32>,
    pub range_data: Option<String>,
    pub sku_ids: Option<String>,
    pub member_range: i32,
    pub member_marks: Option<String>,
    pub member_scope: Option<String>,
}

/// 积分兑换规则序列化（`points_exchange` 行）。
/// 导入防呆：`is_enabled` 强制 0。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct ClonePointsExchange {
    pub id: i32,
    pub product_id: i32,
    pub exchange_integral: i32,
    pub points_deducted_amount: Decimal,
    pub is_hot: i32,
    pub is_enabled: i32,
    pub sku_id: Option<i32>,
    /// 1 兑商品 / 2 兑优惠券。
    pub exchange_type: i32,
    pub coupon_id: Option<i32>,
}

/// 赠品定义序列化（`product_gift` 行；被满赠阶梯与充值档位引用，
/// 营销域内先于引用方导入）。sku 对不齐时降为整品赠品（sku_id=0）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneProductGift {
    pub gift_id: i32,
    pub gift_name: String,
    pub gift_stock: i32,
    pub product_id: i32,
    pub sku_id: i32,
    pub shop_id: i32,
}

/// 充值档位序列化（`recharge_setting` 行；gift_id 按赠品映射重写，
/// 对不齐置 0 = 无赠品）。导入防呆：`is_show` 强制 0。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneRechargeSetting {
    pub recharge_id: i32,
    pub money: Option<Decimal>,
    pub discount_money: Option<Decimal>,
    pub sort_order: Option<i32>,
    pub is_show: Option<i32>,
    pub gift_id: i32,
    pub gift_quantity: i32,
}

/// 签到规则序列化（`sign_in_setting` 行，平台级、无店铺维）。
/// 导入按 day_num 对齐：本站已有同 day_num 规则时跳过（防翻倍），
/// 仍登记映射以便撤销口径统一（撤销时同理跳过删除本站原有行——
/// 映射的 target_id 记本站既有行 id、撤销按「非本任务新建」处理，
/// 见导入端实现注释）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneSignInSetting {
    pub id: i32,
    pub name: Option<String>,
    pub points: Option<i32>,
    pub day_num: Option<i32>,
    pub sort_order: i32,
}

/// 礼品卡组序列化（`gift_card_groups` 行 + 012 适用范围列）。
/// 导入防呆：`status` 强制 0（关闭）。已发行卡（gift_cards，含卡密）
/// 属实例资产，不导出。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneGiftCardGroup {
    pub card_groups_id: i32,
    pub group_type: i32,
    pub card_type: i32,
    pub card_group_name: String,
    pub image_url: Option<String>,
    pub validity_type: i32,
    pub validity_days: Option<i32>,
    pub expire_time: Option<i32>,
    pub face_amount: Option<Decimal>,
    pub status: i32,
    pub remark: Option<String>,
    pub shop_id: Option<i32>,
    pub availability_stock: Option<i32>,
    /// 适用范围：0 全部 / 1 指定商品 / 2 指定品类（迁移 012）。
    pub scope_type: i32,
}

/// 礼品卡兑换商品池行序列化（`gift_card_products` 行）。
/// sku 对不齐时降为整品绑定（sku_id=0）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneGiftCardProduct {
    pub gift_product_id: i32,
    pub card_groups_id: i32,
    pub product_id: i32,
    pub sku_id: i32,
}

/// 礼品卡适用范围引用行（商品或品类，结构相同、两表共用）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneGiftScopeRef {
    pub card_groups_id: i32,
    /// 商品表时为 product_id、品类表时为 category_id（按所在 Vec 区分）。
    pub ref_id: i32,
}

/// 抽奖活动序列化（`lottery_activity` 行）。无启停标志，时间原样
/// 搬运、镜像行停用（同秒杀口径）；decorate_config 内的商品/分类
/// 引用走装修同款变换重写。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneLotteryActivity {
    pub activity_id: i32,
    pub activity_name: String,
    pub start_time: i32,
    pub end_time: i32,
    pub cost_points: i32,
    pub daily_limit: i32,
    pub total_limit: i32,
    pub win_limit: i32,
    pub decorate_config: Option<String>,
    pub grid_config: Option<String>,
    pub no_win_name: String,
    pub no_win_image: String,
    pub no_win_tips: String,
    pub shop_id: i32,
    pub cost_type: i32,
    pub share_reward_enabled: i32,
    pub share_reward_count: i32,
    pub share_reward_daily_limit: i32,
    pub share_reward_total_limit: i32,
}

/// 抽奖奖品序列化（`lottery_prize` 行；导入时 prize_stock_used 归 0）。
/// prize_value 按 prize_type 解释：1 积分 = 积分数（原值）；2 商品 =
/// 商品 id（按商品映射重写）；3 优惠券 = 券 id（按券映射重写）；
/// 对不齐的奖品行跳过并在任务报告列明。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneLotteryPrize {
    pub prize_id: i32,
    pub activity_id: i32,
    pub prize_name: String,
    pub prize_type: i32,
    pub prize_value: String,
    pub prize_image: String,
    pub prize_stock: i32,
}

/// 源 SKU 目录项：营销明细的 SKU 引用靠它在导入端按货号对齐。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneSkuRef {
    pub sku_id: i32,
    pub product_id: i32,
    /// 货号（运营可控的跨站稳定标识；空串表示无货号，只能靠
    /// 「源/本站均为单 SKU」兜底对齐）。
    pub sku_sn: String,
}

// ———————————————————— P3：文章/友链序列化包 ————————————————————

/// `GET /clone/v1/content` 返回：文章分类 + 文章 + 文章-商品关联 + 友链。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneContentPackage {
    pub categories: Vec<CloneArticleCategory>,
    pub articles: Vec<CloneArticle>,
    pub article_products: Vec<CloneArticleProduct>,
    pub friend_links: Vec<CloneFriendLink>,
}

/// 文章分类序列化（导入按名称路径对齐，同商品分类口径；
/// 结构域对象撤销时保留）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneArticleCategory {
    pub article_category_id: i32,
    pub article_category_name: String,
    pub category_sn: String,
    pub category_type: i32,
    pub keywords: String,
    pub description: String,
    pub sort_order: i32,
    pub parent_id: i32,
}

/// 文章序列化。导入防呆：`is_show` 强制 0（不显示）、`click_count`
/// 归 0（运营数据不搬）；正文 HTML 内的源站素材地址按素材转存映射
/// 重写（素材域已导入时）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneArticle {
    pub article_id: i32,
    pub article_title: String,
    pub article_category_id: i32,
    pub article_sn: String,
    pub article_thumb: String,
    pub article_tag: String,
    pub article_author: String,
    pub article_type: i32,
    pub content: String,
    pub description: Option<String>,
    pub keywords: String,
    pub is_hot: i32,
    pub is_top: i32,
    pub link: String,
}

/// 文章-商品关联行（`product_article`：goods_id × article_id）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneArticleProduct {
    pub article_id: i32,
    pub goods_id: i32,
}

/// 友情链接序列化（导入按 标题+地址 去重）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneFriendLink {
    pub link_id: i32,
    pub link_title: String,
    pub link_logo: String,
    pub link_url: String,
    pub sort_order: i32,
}

// ———————————————————— P3：邮件/消息模板序列化包 ————————————————————

/// `GET /clone/v1/mail` 返回：邮件模板文本 + 消息类型渠道开关配置。
/// **豁免**：`message_template`（微信模板消息）不导出——其 template_id
/// 是源站公众号在微信侧签发的外部标识，复制到本站会指向错误公众号，
/// 属站点身份绑定信息；`mailSettings`（SMTP 凭据）在设置域已豁免，
/// 本域同样不碰。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneMailPackage {
    pub templates: Vec<CloneMailTemplate>,
    pub message_types: Vec<CloneMessageType>,
}

/// 邮件模板序列化（`mail_templates` 行；按 template_code 对齐覆盖，
/// `last_send` 为运行态统计不搬、导入端不动本站原值）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneMailTemplate {
    pub template_id: i32,
    pub template_code: String,
    pub is_html: i32,
    pub template_subject: String,
    pub template_content: String,
    pub r#type: String,
}

/// 消息类型配置序列化（`message_type` 行：通知类型的各渠道开关，
/// 按 message_id 对齐覆盖；name/describe 为两站同源字典文案一并搬运）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneMessageType {
    pub message_id: i32,
    pub name: Option<String>,
    pub describe: Option<String>,
    pub send_type: Option<i32>,
    pub is_wechat: Option<i32>,
    pub is_mini_program: Option<i32>,
    pub is_message: Option<i32>,
    pub is_msg: Option<i32>,
    pub is_app: Option<i32>,
    pub is_ding: Option<i32>,
    pub is_wx_work: i32,
    pub is_mail: i32,
}

// ———————————————————— P3：店铺壳序列化包 ————————————————————

/// `GET /clone/v1/shops` 返回：全部未删除店铺的「壳」字段 + 分销规则。
/// **豁免字段**（逐项）：merchant_id（商户账号绑定）、shop_money /
/// frozen_money（资金）、fee_rate / service_fee_rate（费率条款）、
/// vendor_set_price_type / vendor_set_price_auto_value（供应商定价
/// 条款）、click_count / shop_sales / shop_star / shop_total_star /
/// shop_star_num（运营数据）、last_login_time（运行态）。导入端：
/// 新建壳一律 status=4（暂停运营）、资金归 0、merchant_id=0，
/// 绑定商户账号后由平台开启；store_parent_id 按壳映射二次回填。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneShopsPackage {
    pub shops: Vec<CloneShop>,
    pub salesman_configs: Vec<CloneSalesmanConfig>,
}

/// 店铺壳序列化（`shop` 行的资料子集，见 CloneShopsPackage 豁免说明）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneShop {
    pub shop_id: i32,
    pub shop_title: Option<String>,
    pub shop_logo: Option<String>,
    pub description: Option<String>,
    pub contact_mobile: Option<String>,
    pub kefu_phone: Option<String>,
    pub kefu_weixin: Option<String>,
    pub kefu_link: Option<String>,
    pub is_contact_kefu: i32,
    pub kefu_inlet: Option<String>,
    /// 发货信息（店铺自身业务资料，随壳搬运）。
    pub ship_info: Option<String>,
    /// 收货信息（同上）。
    pub receive_info: Option<String>,
    /// 1 店铺 / 2 门店 / 3 自提点。
    pub shop_type: Option<i32>,
    /// 父级店铺 id（门店归属），导入端按壳映射回填。
    pub store_parent_id: Option<i32>,
    pub shop_cover_picture: Option<String>,
    pub shop_show_picture: Option<String>,
    pub shop_contact_json: Option<String>,
    pub shop_open_close_json: Option<String>,
    /// 区划 id（两站同源区划字典，按原值搬运）。
    pub shop_region_id: Option<String>,
    pub shop_detailed_address: Option<String>,
    pub shop_longitude: Option<Decimal>,
    pub shop_latitude: Option<Decimal>,
    pub use_shop_category: Option<i32>,
    pub shop_show_category: Option<String>,
    pub auto_erase_zero: i32,
    pub auto_deduction_points: i32,
    pub default_collection_method: i32,
    pub auto_add_cart_after_scan: i32,
    pub auto_print_online_order: i32,
    pub auto_print_offline_order: i32,
    pub shop_common_remarks: Option<String>,
    pub pos_product_source: Option<String>,
}

/// 分销规则序列化（`salesman_config` 行：仅 salesmanConfig /
/// salesmanSettlement 两 code 有业务语义，data 为规则 JSON 原文，
/// 不含任何凭据）。导入按（壳映射后 shop_id, code）upsert，
/// 覆盖前原值进 P3 快照可回滚。
#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase", default)]
#[sqlx(rename_all = "snake_case")]
pub struct CloneSalesmanConfig {
    pub id: i32,
    pub shop_id: Option<i32>,
    pub code: Option<String>,
    pub data: Option<String>,
}

// ———————————————————— P3：离线包 ————————————————————

/// 离线包文件格式（导出端生成、导入端验签后走同一导入编排）。
/// `files[].content` 为对应 `/clone/v1` 端点 `data` 载荷的 JSON 原文；
/// 每个文件带 sha256，整包带 HMAC-SHA256 签名：签名键为密钥
/// secret 的 SHA-256（即落库的 secret_hash——导出端用落库哈希签名、
/// 导入端用口令中的 secret 现算哈希验签，两侧都不需要明文落盘）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneOfflinePackage {
    /// 固定 `nextrade-clone-package`。
    pub format: String,
    pub package_version: i32,
    pub key_code: String,
    pub source_label: String,
    pub source_origin: String,
    pub source_shop_id: i32,
    pub exporter_version: i32,
    pub created_at: i32,
    pub files: Vec<CloneOfflineFile>,
    /// HMAC-SHA256(secret_hash, 规范串) 的 hex；规范串 = 文件按名
    /// 排序后逐个 `name\nsha256\n` 拼接。
    pub signature: String,
}

/// 离线包内的一个数据文件。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneOfflineFile {
    /// 端点名：manifest / catalog / products / settings / decorate /
    /// assets / marketing / content / mail / shops。
    pub name: String,
    /// content 的 SHA-256（hex）。
    pub sha256: String,
    /// 端点 data 载荷 JSON 原文。
    pub content: String,
}

/// 离线包格式标识。
pub const OFFLINE_PACKAGE_FORMAT: &str = "nextrade-clone-package";
/// 离线包格式版本（随 EXPORTER_VERSION v3 首发）。
pub const OFFLINE_PACKAGE_VERSION: i32 = 3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_domain_parsing() {
        let mut k = SiteCloneKey::default();
        k.scopes = "products,settings".into();
        assert!(k.has_domain(DOMAIN_PRODUCTS));
        assert!(k.has_domain(DOMAIN_SETTINGS));
        assert!(!k.has_domain(DOMAIN_DECORATE));
        assert_eq!(k.domain_list().len(), 2);
        k.scopes = "products,settings,decorate,assets".into();
        assert!(k.has_domain(DOMAIN_DECORATE));
        assert!(k.has_domain(DOMAIN_ASSETS));
        assert!(!k.has_domain("marketing"));
        assert_eq!(k.domain_list().len(), 4);
        k.scopes = String::new();
        assert!(!k.has_domain(DOMAIN_PRODUCTS));
    }

    #[test]
    fn id_dto_target_prefers_first_present() {
        let dto = CloneIdDTO { key_id: Some(7), job_id: Some(9), ..Default::default() };
        assert_eq!(dto.target(), Some(7));
        let dto2 = CloneIdDTO { snapshot_id: Some(3), ..Default::default() };
        assert_eq!(dto2.target(), Some(3));
    }
}
