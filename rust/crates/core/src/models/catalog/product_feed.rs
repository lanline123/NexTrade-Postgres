//! 商品对接 API（catalog::product_feed）DTO / VO / 纯逻辑。
//!
//! 功能规划：`~/workspace/product-feed-api-design/proposal.md`。
//!
//! # 概念
//! 一把「对接密钥」= 一个下游：`access_key` 公开标识 + `secret` 保密凭据
//! （库里只存 SHA-256 摘要，明文仅创建/重置时返回一次）。密钥同时承载
//! 授权范围（全部 / 指定商品 / 指定分类含全部后代）与价格规则
//! （默认规则 + 单品覆盖）。
//!
//! # 价格规则口径（规划 §3，写死）
//! - 类型：`none` 原价 / `markup_percent` 加价百分比 / `add_fixed` 加固定
//!   金额 / `fixed_price` 固定售价。
//! - 优先级：单品覆盖 > 密钥默认 > 原价。
//! - `fixed_price` 对多 SKU 商品：商品价与每个 SKU 价统一等于固定值。
//! - 市场价不套规则：输出 `max(原市场价, 最终售价)`。
//! - 精度：Decimal 全程计算，最终量化 2 位小数四舍五入，下限 0.01。
//! - feed 响应不含原价 / 成本价字段，下游拿到的即最终售价。
//!
//! # 询价商品口径（规划 §7.3）
//! 商品级 `is_inquiry=1` 或任一启用 SKU `is_inquiry=1` ⇒ 询价商品
//! （与前台 `inquiryState != none` 一致）。默认排除，密钥
//! `include_inquiry=1` 才包含且响应带 `inquiry` 标记。
//!
//! # 金额：价格字段为 `Decimal`（JSON 字符串）。

use std::collections::{HashMap, HashSet};

use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ———————————————————— 常量与文案 ————————————————————

/// 授权模式：全部上架商品。
pub const SCOPE_ALL: i32 = 1;
/// 授权模式：指定商品。
pub const SCOPE_PRODUCTS: i32 = 2;
/// 授权模式：指定分类（含全部后代）。
pub const SCOPE_CATEGORIES: i32 = 3;

/// 价格规则：原价。
pub const RULE_NONE: &str = "none";
/// 价格规则：加价百分比。
pub const RULE_MARKUP_PERCENT: &str = "markup_percent";
/// 价格规则：加固定金额。
pub const RULE_ADD_FIXED: &str = "add_fixed";
/// 价格规则：固定售价。
pub const RULE_FIXED_PRICE: &str = "fixed_price";

pub const MSG_KEY_NAME_REQUIRED: &str = "密钥名称不能为空";
pub const MSG_KEY_NAME_TOO_LONG: &str = "密钥名称过长";
pub const MSG_KEY_NOT_FOUND: &str = "对接密钥不存在";
pub const MSG_SCOPE_IDS_REQUIRED: &str = "指定范围时商品/分类清单不能为空";
pub const MSG_RULE_TYPE_INVALID: &str = "价格规则类型不合法";
pub const MSG_RULE_PERCENT_RANGE: &str = "加价百分比必须大于 -100 且不超过 1000";
pub const MSG_RULE_FIXED_RANGE: &str = "固定金额超出允许范围";
pub const MSG_RULE_FIXED_PRICE_POSITIVE: &str = "固定售价必须大于 0";
pub const MSG_EXPIRE_IN_PAST: &str = "过期时间不能早于当前时间";
pub const MSG_FEED_UNAUTHORIZED: &str = "对接密钥无效";
pub const MSG_FEED_DISABLED: &str = "对接密钥已停用";
pub const MSG_FEED_EXPIRED: &str = "对接密钥已过期";
pub const MSG_FEED_PRODUCT_NOT_FOUND: &str = "商品不存在或未授权";

/// feed 列表单页上限（规划 §4.5）。
pub const FEED_MAX_PAGE_SIZE: u64 = 100;

// ———————————————————— 行结构（FromRow） ————————————————————

/// `product_feed_key` 行。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct FeedKey {
    pub feed_key_id: i32,
    pub key_name: String,
    pub access_key: String,
    pub secret_hash: String,
    pub status: i16,
    pub scope_mode: i16,
    pub include_inquiry: i16,
    pub price_rule_type: String,
    pub price_rule_value: Decimal,
    pub expire_time: i32,
    pub remark: String,
    pub shop_id: i32,
    pub last_used_time: i32,
    pub call_count: i64,
    pub add_time: i32,
    pub update_time: i32,
}

/// `product_feed_price_override` 行。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct FeedPriceOverride {
    pub id: i32,
    pub feed_key_id: i32,
    pub product_id: i32,
    pub rule_type: String,
    pub rule_value: Decimal,
    pub add_time: i32,
    pub update_time: i32,
}

/// feed 商品查询行（`product` 主表 + 派生列）。
#[derive(Debug, Clone, Default, FromRow)]
#[sqlx(rename_all = "snake_case")]
pub struct FeedProductRow {
    pub product_id: i32,
    pub product_name: String,
    pub product_sn: String,
    pub product_price: Decimal,
    pub market_price: Decimal,
    pub product_stock: i32,
    pub category_id: i32,
    pub brand_id: i32,
    /// 所属店铺（`product.shop_id`；平台联邦店铺范围过滤与来源标记用，
    /// feed 自身口径不消费此列）。
    pub shop_id: i32,
    pub pic_url: String,
    pub product_brief: String,
    /// 详情 HTML（仅供 contentHash 计算，不直接出 feed）。
    pub product_desc: Option<String>,
    pub last_update: i32,
    /// 商品级询价标志（`product.is_inquiry`）。
    pub is_inquiry: i32,
    /// 启用 SKU 数。
    pub sku_count: i64,
    /// 启用 SKU 中询价标志的最大值（任一为 1 即整品询价）。
    pub sku_inquiry_max: i32,
}

// ———————————————————— 价格规则纯逻辑 ————————————————————

/// 一条已解析的价格规则（type + value）。
#[derive(Debug, Clone, PartialEq)]
pub struct PriceRule {
    pub rule_type: String,
    pub value: Decimal,
}

impl Default for PriceRule {
    fn default() -> Self {
        Self {
            rule_type: RULE_NONE.to_string(),
            value: Decimal::ZERO,
        }
    }
}

impl PriceRule {
    #[must_use]
    pub fn new(rule_type: &str, value: Decimal) -> Self {
        Self {
            rule_type: rule_type.to_string(),
            value,
        }
    }

    /// 套规则：输入原价，输出最终售价（已量化到 2 位小数、下限 0.01）。
    #[must_use]
    pub fn apply(&self, base: Decimal) -> Decimal {
        let raw = match self.rule_type.as_str() {
            RULE_MARKUP_PERCENT => base * (Decimal::from(100) + self.value) / Decimal::from(100),
            RULE_ADD_FIXED => base + self.value,
            RULE_FIXED_PRICE => self.value,
            _ => base,
        };
        quantize_price(raw)
    }
}

/// 价格量化：2 位小数、四舍五入（MidpointAwayFromZero）、下限 0.01。
#[must_use]
pub fn quantize_price(v: Decimal) -> Decimal {
    let q = v.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
    let floor = Decimal::new(1, 2); // 0.01
    if q < floor {
        floor
    } else {
        q
    }
}

/// 市场价输出口径：不套规则，取 `max(原市场价, 最终售价)`；
/// 原市场价为空/0 时即最终售价。
#[must_use]
pub fn market_price_out(market_base: Decimal, final_price: Decimal) -> Decimal {
    if market_base <= Decimal::ZERO {
        return final_price;
    }
    if market_base < final_price {
        final_price
    } else {
        quantize_price(market_base)
    }
}

/// 解析生效规则：单品覆盖优先于密钥默认（规划 §3.2）。
#[must_use]
pub fn resolve_rule(
    key_default: &PriceRule,
    product_override: Option<&FeedPriceOverride>,
) -> PriceRule {
    match product_override {
        Some(o) => PriceRule::new(&o.rule_type, o.rule_value),
        None => key_default.clone(),
    }
}

/// 解析生效规则（已归一的覆盖规则版本）：覆盖存在即覆盖优先。
#[must_use]
pub fn resolve_rule_from(key_default: &PriceRule, product_override: Option<&PriceRule>) -> PriceRule {
    match product_override {
        Some(r) => r.clone(),
        None => key_default.clone(),
    }
}

/// 已解析的商品受众（范围 + 规则）：feed 密钥与平台联邦凭证共用同一份
/// 商品查询口径——范围先解析成集合，查询层只认集合，不认凭据类型。
#[derive(Debug, Clone, Default)]
pub struct FeedAudience {
    /// 商品范围模式（`SCOPE_ALL` / `SCOPE_PRODUCTS` / `SCOPE_CATEGORIES`）。
    pub scope_mode: i32,
    /// 是否包含询价商品。
    pub include_inquiry: bool,
    /// 指定商品集合（`scope_mode = SCOPE_PRODUCTS` 时生效）。
    pub product_ids: Option<HashSet<i32>>,
    /// 指定分类集合（已展开全部后代，`scope_mode = SCOPE_CATEGORIES` 时生效）。
    pub category_ids: Option<HashSet<i32>>,
    /// 店铺范围集合（`None` = 不限店铺；平台联邦凭证的店铺范围用）。
    pub shop_ids: Option<HashSet<i32>>,
    /// 默认价格规则。
    pub default_rule: PriceRule,
    /// 单品覆盖规则（product_id → 已解析规则）。
    pub overrides: HashMap<i32, PriceRule>,
}

impl FeedAudience {
    /// 某商品的生效价格规则（覆盖优先于默认）。
    #[must_use]
    pub fn rule_for(&self, product_id: i32) -> PriceRule {
        resolve_rule_from(&self.default_rule, self.overrides.get(&product_id))
    }

    /// 某商品是否在受众范围内（详情越权校验用，与列表过滤同口径）。
    #[must_use]
    pub fn contains(&self, product_id: i32, category_id: i32, shop_id: i32) -> bool {
        if let Some(ids) = &self.shop_ids {
            if !ids.contains(&shop_id) {
                return false;
            }
        }
        match self.scope_mode {
            SCOPE_PRODUCTS => self
                .product_ids
                .as_ref()
                .is_some_and(|ids| ids.contains(&product_id)),
            SCOPE_CATEGORIES => self
                .category_ids
                .as_ref()
                .is_some_and(|ids| ids.contains(&category_id)),
            _ => true,
        }
    }
}

/// 校验一条价格规则（后台与建钥共用）。
///
/// # Errors
/// 类型非法 / 百分比越界 / 固定金额越界 / 固定售价非正 → 对应文案。
pub fn validate_rule(rule_type: &str, value: Decimal) -> Result<(), String> {
    match rule_type {
        RULE_NONE => Ok(()),
        RULE_MARKUP_PERCENT => {
            if value <= Decimal::from(-100) || value > Decimal::from(1000) {
                Err(MSG_RULE_PERCENT_RANGE.to_string())
            } else {
                Ok(())
            }
        }
        RULE_ADD_FIXED => {
            if value.abs() > Decimal::from(99_999_999) {
                Err(MSG_RULE_FIXED_RANGE.to_string())
            } else {
                Ok(())
            }
        }
        RULE_FIXED_PRICE => {
            if value <= Decimal::ZERO {
                Err(MSG_RULE_FIXED_PRICE_POSITIVE.to_string())
            } else {
                Ok(())
            }
        }
        _ => Err(MSG_RULE_TYPE_INVALID.to_string()),
    }
}

// ———————————————————— 范围纯逻辑 ————————————————————

/// 分类展开：给定勾选分类与全站 `(category_id, parent_id)` 清单，
/// 返回勾选分类 + 全部后代的集合（规划 §2.1 表 3 口径）。
#[must_use]
pub fn expand_categories(selected: &[i32], all: &[(i32, i32)]) -> HashSet<i32> {
    let mut children: HashMap<i32, Vec<i32>> = HashMap::new();
    for (id, parent) in all {
        children.entry(*parent).or_default().push(*id);
    }
    let mut out: HashSet<i32> = HashSet::new();
    let mut stack: Vec<i32> = selected.to_vec();
    while let Some(id) = stack.pop() {
        if out.insert(id) {
            if let Some(kids) = children.get(&id) {
                stack.extend(kids.iter().copied());
            }
        }
    }
    out
}

/// 询价商品判定：商品级标志或任一启用 SKU 标志（规划 §7.3）。
#[must_use]
pub fn is_inquiry_product(product_flag: i32, sku_inquiry_max: i32) -> bool {
    product_flag == 1 || sku_inquiry_max == 1
}

// ———————————————————— 内容哈希 ————————————————————

/// 内容哈希的原料（service 层组装，哈希计算在 service 层用 sha256）。
/// 价格用规则后价格：上游改价或改规则都会改变哈希（规划 §4.6）。
#[derive(Debug, Clone, Default)]
pub struct ContentHashParts {
    pub product_name: String,
    pub product_brief: String,
    pub product_desc: String,
    pub category_id: i32,
    pub brand_id: i32,
    pub price: Decimal,
    pub stock: i32,
    pub inquiry: bool,
    pub images: Vec<String>,
    /// `(sku_id, sku_value, 规则后价, 库存)`，调用方按 sku_id 升序传入。
    pub skus: Vec<(i32, String, Decimal, i32)>,
}

/// 规范化拼接串（字段以 `\n` 分隔、SKU 以 `;` 连接），供 sha256 摘要。
#[must_use]
pub fn content_hash_canonical(parts: &ContentHashParts) -> String {
    let sku_part = parts
        .skus
        .iter()
        .map(|(id, value, price, stock)| format!("{id}:{value}:{price}:{stock}"))
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        parts.product_name,
        parts.product_brief,
        parts.product_desc,
        parts.category_id,
        parts.brand_id,
        parts.price,
        parts.stock,
        if parts.inquiry { 1 } else { 0 },
        parts.images.join(","),
        sku_part,
    )
}

// ———————————————————— 管理端 DTO / VO ————————————————————

/// 密钥列表查询（`GET /adminapi/product/feed/list`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FeedKeyListQuery {
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    pub keyword: Option<String>,
    pub status: Option<i32>,
}

/// 密钥列表/详情 VO（永不含 secret 与哈希）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedKeyVO {
    pub feed_key_id: i32,
    pub key_name: String,
    pub access_key: String,
    pub status: i32,
    pub scope_mode: i32,
    pub include_inquiry: bool,
    pub price_rule_type: String,
    pub price_rule_value: Decimal,
    pub expire_time: i32,
    pub remark: String,
    pub last_used_time: i32,
    pub call_count: i64,
    pub add_time: i32,
    pub update_time: i32,
    /// 当前口径下可见商品数（列表/详情回填）。
    pub product_count: i64,
}

/// 密钥详情 VO（详情接口在列表 VO 基础上补范围与覆盖）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedKeyDetailVO {
    #[serde(flatten)]
    pub key: FeedKeyVO,
    pub product_ids: Vec<i32>,
    pub category_ids: Vec<i32>,
    pub price_overrides: Vec<FeedPriceOverrideVO>,
}

/// 单品覆盖 VO（带商品名与原价，供后台回显）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedPriceOverrideVO {
    pub product_id: i32,
    pub product_name: Option<String>,
    pub base_price: Option<Decimal>,
    pub rule_type: String,
    pub rule_value: Decimal,
    /// 规则后商品价（回填预览）。
    pub final_price: Option<Decimal>,
}

/// 新建密钥入参（`POST /adminapi/product/feed/create`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FeedKeyCreateDTO {
    pub key_name: Option<String>,
    pub scope_mode: Option<i32>,
    pub product_ids: Vec<i32>,
    pub category_ids: Vec<i32>,
    pub include_inquiry: Option<bool>,
    pub price_rule_type: Option<String>,
    pub price_rule_value: Option<Decimal>,
    pub expire_time: Option<i32>,
    pub remark: Option<String>,
}

/// 编辑密钥入参（`POST /adminapi/product/feed/update`）：范围清单整体替换。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FeedKeyUpdateDTO {
    pub feed_key_id: i32,
    pub key_name: Option<String>,
    pub status: Option<i32>,
    pub scope_mode: Option<i32>,
    pub product_ids: Vec<i32>,
    pub category_ids: Vec<i32>,
    pub include_inquiry: Option<bool>,
    pub price_rule_type: Option<String>,
    pub price_rule_value: Option<Decimal>,
    pub expire_time: Option<i32>,
    pub remark: Option<String>,
}

/// 新建/重置密钥的响应：**唯一一次**返回 secret 明文。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedKeySecretVO {
    pub feed_key_id: i32,
    pub access_key: String,
    pub secret: String,
}

/// 保存单品覆盖入参（`POST /adminapi/product/feed/priceOverride/save`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FeedPriceOverrideSaveDTO {
    pub feed_key_id: i32,
    pub product_id: i32,
    pub rule_type: Option<String>,
    pub rule_value: Option<Decimal>,
}

/// 删除单品覆盖 / 通用 id 入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FeedIdDTO {
    pub feed_key_id: i32,
    pub product_id: Option<i32>,
}

/// 校验建钥/改钥入参（名称/范围/规则/过期），返回归一后的关键字段。
///
/// # Errors
/// 任一口径不满足 → 对应文案（`MSG_*`）。
pub fn validate_key_fields(
    key_name: Option<&str>,
    scope_mode: i32,
    product_ids: &[i32],
    category_ids: &[i32],
    rule_type: &str,
    rule_value: Decimal,
    expire_time: i32,
    now: i32,
) -> Result<String, String> {
    let name = key_name.map(str::trim).unwrap_or_default();
    if name.is_empty() {
        return Err(MSG_KEY_NAME_REQUIRED.to_string());
    }
    if name.chars().count() > 100 {
        return Err(MSG_KEY_NAME_TOO_LONG.to_string());
    }
    match scope_mode {
        SCOPE_ALL => {}
        SCOPE_PRODUCTS => {
            if product_ids.is_empty() {
                return Err(MSG_SCOPE_IDS_REQUIRED.to_string());
            }
        }
        SCOPE_CATEGORIES => {
            if category_ids.is_empty() {
                return Err(MSG_SCOPE_IDS_REQUIRED.to_string());
            }
        }
        _ => return Err(MSG_SCOPE_IDS_REQUIRED.to_string()),
    }
    validate_rule(rule_type, rule_value)?;
    if expire_time != 0 && expire_time < now {
        return Err(MSG_EXPIRE_IN_PAST.to_string());
    }
    Ok(name.to_string())
}

// ———————————————————— Provider 面 VO ————————————————————

/// `GET /api/feed/v1/meta` 响应。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedMetaVO {
    pub key_name: String,
    pub scope_mode: i32,
    pub include_inquiry: bool,
    pub product_count: i64,
    pub server_time: i64,
    pub api_version: String,
}

/// 授权分类树节点（`GET /api/feed/v1/categories`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedCategoryVO {
    pub category_id: i32,
    pub category_name: String,
    pub parent_id: i32,
    pub sort_order: i32,
    pub category_pic: String,
    /// 本分类子树内授权商品数。
    pub product_count: i64,
    pub children: Vec<FeedCategoryVO>,
}

/// feed 商品列表项（`GET /api/feed/v1/products`）。
/// 价格均为规则后价格，不含原价/成本字段（规划 §3.6）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedProductVO {
    pub product_id: i32,
    pub product_name: String,
    pub product_sn: String,
    pub category_id: i32,
    pub category_name: Option<String>,
    /// 所属店铺（平台联邦来源标记用；普通下游可忽略）。
    pub shop_id: i32,
    /// 店铺名（联邦来源标记用；解析不到时为 None）。
    pub shop_name: Option<String>,
    pub brand_id: i32,
    pub brand_name: Option<String>,
    pub pic_url: String,
    pub price: Decimal,
    pub market_price: Decimal,
    pub stock: i32,
    pub sku_count: i64,
    pub inquiry: bool,
    pub content_hash: String,
    pub last_update: i32,
}

/// feed 商品详情（`GET /api/feed/v1/products/detail`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedProductDetailVO {
    #[serde(flatten)]
    pub base: FeedProductVO,
    pub product_brief: String,
    pub product_desc: Option<String>,
    pub keywords: Option<String>,
    pub product_weight: Decimal,
    pub product_unit: Option<String>,
    pub free_shipping: i32,
    pub product_type: Option<i32>,
    pub images: Vec<FeedImageVO>,
    pub skus: Vec<FeedSkuVO>,
    pub category_path: Vec<FeedCategoryPathVO>,
}

/// feed 图片项（`product_gallery` 一行）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedImageVO {
    pub pic_url: String,
    pub pic_thumb: String,
    pub pic_large: String,
    pub pic_original: String,
    pub sort_order: i32,
}

/// feed SKU 项（价格为规则后价格）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedSkuVO {
    pub sku_id: i32,
    pub sku_value: String,
    pub sku_data: Option<String>,
    pub sku_sn: Option<String>,
    pub price: Decimal,
    pub market_price: Decimal,
    pub stock: i32,
    pub is_enabled: i32,
    pub inquiry: bool,
}

/// 分类路径节点（根 → 叶）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedCategoryPathVO {
    pub category_id: i32,
    pub category_name: String,
}

/// 管理端「密钥可见商品」预览项（原价与规则后价格对照）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedScopeProductVO {
    pub product_id: i32,
    pub product_name: String,
    pub product_sn: String,
    pub pic_url: String,
    pub category_id: i32,
    pub base_price: Decimal,
    pub base_market_price: Decimal,
    pub final_price: Decimal,
    pub final_market_price: Decimal,
    pub stock: i32,
    pub inquiry: bool,
    pub has_override: bool,
}

/// feed 列表查询（`GET /api/feed/v1/products`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FeedProductListQuery {
    pub page: Option<u64>,
    #[serde(alias = "size")]
    pub page_size: Option<u64>,
    /// 增量同步：只返回 `last_update >= updated_since` 的商品。
    pub updated_since: Option<i32>,
    pub category_id: Option<i32>,
    pub keyword: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn d(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    #[test]
    fn rule_none_keeps_base() {
        let r = PriceRule::new(RULE_NONE, d("50"));
        assert_eq!(r.apply(d("129.99")), d("129.99"));
    }

    #[test]
    fn rule_markup_percent() {
        let r = PriceRule::new(RULE_MARKUP_PERCENT, d("10"));
        assert_eq!(r.apply(d("100")), d("110.00"));
        // 负百分比 = 降价
        let r = PriceRule::new(RULE_MARKUP_PERCENT, d("-20"));
        assert_eq!(r.apply(d("100")), d("80.00"));
        // 小数百分比四舍五入到分
        let r = PriceRule::new(RULE_MARKUP_PERCENT, d("2.5"));
        assert_eq!(r.apply(d("33.33")), d("34.16"));
    }

    #[test]
    fn rule_add_fixed_and_floor() {
        let r = PriceRule::new(RULE_ADD_FIXED, d("50"));
        assert_eq!(r.apply(d("100")), d("150.00"));
        // 减到负数 → 下限 0.01
        let r = PriceRule::new(RULE_ADD_FIXED, d("-200"));
        assert_eq!(r.apply(d("100")), d("0.01"));
    }

    #[test]
    fn rule_fixed_price_ignores_base() {
        let r = PriceRule::new(RULE_FIXED_PRICE, d("199"));
        assert_eq!(r.apply(d("100")), d("199.00"));
        assert_eq!(r.apply(d("99999")), d("199.00"));
    }

    #[test]
    fn market_price_out_rules() {
        // 正常：原样输出
        assert_eq!(market_price_out(d("200"), d("110")), d("200.00"));
        // 加价后售价反超市场价 → 抬到售价
        assert_eq!(market_price_out(d("100"), d("110")), d("110.00"));
        // 无市场价 → 等于售价
        assert_eq!(market_price_out(d("0"), d("110")), d("110.00"));
    }

    #[test]
    fn resolve_rule_prefers_override() {
        let default = PriceRule::new(RULE_MARKUP_PERCENT, d("10"));
        let ov = FeedPriceOverride {
            rule_type: RULE_FIXED_PRICE.to_string(),
            rule_value: d("88"),
            ..Default::default()
        };
        let r = resolve_rule(&default, Some(&ov));
        assert_eq!(r.apply(d("1000")), d("88.00"));
        let r = resolve_rule(&default, None);
        assert_eq!(r.apply(d("1000")), d("1100.00"));
    }

    #[test]
    fn validate_rule_bounds() {
        assert!(validate_rule(RULE_MARKUP_PERCENT, d("-100")).is_err());
        assert!(validate_rule(RULE_MARKUP_PERCENT, d("-99.99")).is_ok());
        assert!(validate_rule(RULE_MARKUP_PERCENT, d("1000")).is_ok());
        assert!(validate_rule(RULE_MARKUP_PERCENT, d("1000.01")).is_err());
        assert!(validate_rule(RULE_FIXED_PRICE, d("0")).is_err());
        assert!(validate_rule(RULE_FIXED_PRICE, d("0.01")).is_ok());
        assert!(validate_rule("bogus", d("1")).is_err());
        assert!(validate_rule(RULE_NONE, d("12345")).is_ok());
    }

    #[test]
    fn expand_categories_includes_descendants() {
        // 1 → 2 → 3；1 → 4；5 独立
        let all = vec![(1, 0), (2, 1), (3, 2), (4, 1), (5, 0)];
        let got = expand_categories(&[1], &all);
        assert_eq!(got, HashSet::from([1, 2, 3, 4]));
        let got = expand_categories(&[3], &all);
        assert_eq!(got, HashSet::from([3]));
        // 勾选不存在的分类 → 只含自身（查询时自然无商品）
        let got = expand_categories(&[99], &all);
        assert_eq!(got, HashSet::from([99]));
    }

    #[test]
    fn inquiry_detection() {
        assert!(is_inquiry_product(1, 0));
        assert!(is_inquiry_product(0, 1));
        assert!(!is_inquiry_product(0, 0));
    }

    #[test]
    fn validate_key_fields_cases() {
        let now = 1_795_000_000;
        assert!(validate_key_fields(
            Some("下游A"),
            SCOPE_ALL,
            &[],
            &[],
            RULE_NONE,
            d("0"),
            0,
            now
        )
        .is_ok());
        assert!(validate_key_fields(
            Some("  "),
            SCOPE_ALL,
            &[],
            &[],
            RULE_NONE,
            d("0"),
            0,
            now
        )
        .is_err());
        assert!(validate_key_fields(
            Some("x"),
            SCOPE_PRODUCTS,
            &[],
            &[],
            RULE_NONE,
            d("0"),
            0,
            now
        )
        .is_err());
        assert!(validate_key_fields(
            Some("x"),
            SCOPE_CATEGORIES,
            &[],
            &[1],
            RULE_NONE,
            d("0"),
            now - 1,
            now
        )
        .is_err());
    }

    #[test]
    fn content_hash_canonical_stable() {
        let parts = ContentHashParts {
            product_name: "设备".into(),
            price: d("110.00"),
            stock: 5,
            skus: vec![(1, "标准".into(), d("110.00"), 5)],
            ..Default::default()
        };
        let s = content_hash_canonical(&parts);
        assert!(s.contains("设备"));
        assert!(s.contains("1:标准:110.00:5"));
    }
}
