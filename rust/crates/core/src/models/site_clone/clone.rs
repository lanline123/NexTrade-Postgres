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
pub const EXPORTER_VERSION: i32 = 2;

/// 授权域：商品。
pub const DOMAIN_PRODUCTS: &str = "products";
/// 授权域：设置。
pub const DOMAIN_SETTINGS: &str = "settings";
/// 授权域：装修（P2）。
pub const DOMAIN_DECORATE: &str = "decorate";
/// 授权域：素材（P2）。
pub const DOMAIN_ASSETS: &str = "assets";
/// 全部可授权域（P1 两域 + P2 两域）。
pub const ALL_DOMAINS: [&str; 4] = [DOMAIN_PRODUCTS, DOMAIN_SETTINGS, DOMAIN_DECORATE, DOMAIN_ASSETS];

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
    /// 快照 JSON：`{groups:{key:原值}, blocks:{code:原值或 null}}`。
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
