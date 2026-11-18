//! 商品复刻服务（2026-11-18 新增）：双通道取数 → 归一草稿 → 图片转存 → 落库。
//!
//! 设计：`~/workspace/product-clone-design/proposal.md`、`channel-design.md`。
//! - 通道 A：万邦 onebound REST（后端直连，key/secret 存配置表 `productCloneSettings`）。
//! - 通道 B：自建抓取边车（独立 Python 服务，本服务只调其 `/scrape`，归一在本侧）。
//! - 通道 C：官方开放平台预留枚举，不实现。
//!
//! 失败口径：默认不自动换通道（防不知情扣费/撞风控）；仅当设置开启
//! `fallbackBToA` 时 B 失败单向转 A 重试一次。每次尝试都记 attempts（含费用）。

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use rust_decimal::Decimal;
use serde_json::{json, Map, Value};

use nextrade_core::models::catalog::product_clone::{
    parse_source_url, task_status, ChannelId, CloneCompleteness, CloneDraft, CloneFetchDTO,
    CloneImportDTO, CloneImportResult, CloneProp, CloneRetryDTO, CloneSku, CloneSpec,
    CloneTaskQuery, Platform, SupportLevel,
};
use nextrade_core::models::catalog::product_manage::{
    GalleryPicDTO, ProductAttrGroupVO, ProductAttributeVO, ProductAttrsVO, ProductSaveDTO,
    ProductSkuSaveDTO, SkuAttrDTO,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

use super::product_manage::ProductManageService;

/// 配置表业务编码（整块 JSON，与 fontSettings 同型）。
pub const CLONE_SETTINGS_CODE: &str = "productCloneSettings";

// ————————————————————— 设置 —————————————————————

/// 复刻设置（内部全量，含密钥；出参必须走 `masked_settings`）。
#[derive(Debug, Clone)]
pub struct CloneSettings {
    pub default_channel: String,
    pub default_channel_by_platform: Map<String, Value>,
    pub api_enabled: bool,
    pub api_key: String,
    pub api_secret: String,
    pub api_pdd_recharged: bool,
    pub selfhost_enabled: bool,
    pub selfhost_endpoint: String,
    pub selfhost_token: String,
    pub fallback_b_to_a: bool,
    pub daily_api_budget_fen: Option<i64>,
}

impl Default for CloneSettings {
    fn default() -> Self {
        Self {
            default_channel: "api".to_string(),
            default_channel_by_platform: Map::new(),
            api_enabled: true,
            api_key: String::new(),
            api_secret: String::new(),
            api_pdd_recharged: false,
            selfhost_enabled: false,
            selfhost_endpoint: String::new(),
            selfhost_token: String::new(),
            fallback_b_to_a: false,
            daily_api_budget_fen: None,
        }
    }
}

impl CloneSettings {
    fn from_json(v: &Value) -> Self {
        let mut s = Self::default();
        if let Some(x) = v.get("defaultChannel").and_then(Value::as_str) {
            if ChannelId::from_str(x).is_some() {
                s.default_channel = x.to_string();
            }
        }
        if let Some(m) = v.get("defaultChannelByPlatform").and_then(Value::as_object) {
            s.default_channel_by_platform = m.clone();
        }
        if let Some(b) = v.get("apiEnabled").and_then(Value::as_bool) {
            s.api_enabled = b;
        }
        if let Some(x) = v.get("apiKey").and_then(Value::as_str) {
            s.api_key = x.to_string();
        }
        if let Some(x) = v.get("apiSecret").and_then(Value::as_str) {
            s.api_secret = x.to_string();
        }
        if let Some(b) = v.get("apiPddRecharged").and_then(Value::as_bool) {
            s.api_pdd_recharged = b;
        }
        if let Some(b) = v.get("selfhostEnabled").and_then(Value::as_bool) {
            s.selfhost_enabled = b;
        }
        if let Some(x) = v.get("selfhostEndpoint").and_then(Value::as_str) {
            s.selfhost_endpoint = x.trim_end_matches('/').to_string();
        }
        if let Some(x) = v.get("selfhostToken").and_then(Value::as_str) {
            s.selfhost_token = x.to_string();
        }
        if let Some(b) = v.get("fallbackBToA").and_then(Value::as_bool) {
            s.fallback_b_to_a = b;
        }
        if let Some(n) = v.get("dailyApiBudgetFen").and_then(Value::as_i64) {
            if n > 0 {
                s.daily_api_budget_fen = Some(n);
            }
        }
        s
    }

    fn to_json(&self) -> Value {
        json!({
            "defaultChannel": self.default_channel,
            "defaultChannelByPlatform": self.default_channel_by_platform,
            "apiEnabled": self.api_enabled,
            "apiKey": self.api_key,
            "apiSecret": self.api_secret,
            "apiPddRecharged": self.api_pdd_recharged,
            "selfhostEnabled": self.selfhost_enabled,
            "selfhostEndpoint": self.selfhost_endpoint,
            "selfhostToken": self.selfhost_token,
            "fallbackBToA": self.fallback_b_to_a,
            "dailyApiBudgetFen": self.daily_api_budget_fen,
        })
    }

    /// 某平台的默认通道（分平台覆盖优先于全局默认）。
    fn default_channel_for(&self, platform: Platform) -> ChannelId {
        if let Some(v) = self
            .default_channel_by_platform
            .get(platform.as_str())
            .and_then(Value::as_str)
        {
            if let Some(c) = ChannelId::from_str(v) {
                return c;
            }
        }
        ChannelId::from_str(&self.default_channel).unwrap_or(ChannelId::Api)
    }
}

fn tail4(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= 4 {
        return String::new();
    }
    chars[chars.len() - 4..].iter().collect()
}

/// 设置出参（密钥只回尾 4 位与是否已配置，永不回明文）。
fn masked_settings(s: &CloneSettings) -> Value {
    json!({
        "defaultChannel": s.default_channel,
        "defaultChannelByPlatform": s.default_channel_by_platform,
        "apiEnabled": s.api_enabled,
        "apiKeySet": !s.api_key.is_empty(),
        "apiKeyTail": tail4(&s.api_key),
        "apiSecretSet": !s.api_secret.is_empty(),
        "apiPddRecharged": s.api_pdd_recharged,
        "selfhostEnabled": s.selfhost_enabled,
        "selfhostEndpoint": s.selfhost_endpoint,
        "selfhostTokenSet": !s.selfhost_token.is_empty(),
        "fallbackBToA": s.fallback_b_to_a,
        "dailyApiBudgetFen": s.daily_api_budget_fen,
    })
}

/// 保存设置：密钥字段留空 = 保持原值（页面不回显明文，无法原样回传）。
fn merge_settings_save(old: &CloneSettings, payload: &Value) -> CloneSettings {
    let mut s = old.clone();
    if let Some(x) = payload.get("defaultChannel").and_then(Value::as_str) {
        if ChannelId::from_str(x).is_some() {
            s.default_channel = x.to_string();
        }
    }
    if let Some(m) = payload.get("defaultChannelByPlatform").and_then(Value::as_object) {
        s.default_channel_by_platform = m.clone();
    }
    if let Some(b) = payload.get("apiEnabled").and_then(Value::as_bool) {
        s.api_enabled = b;
    }
    if let Some(x) = payload.get("apiKey").and_then(Value::as_str) {
        if !x.trim().is_empty() {
            s.api_key = x.trim().to_string();
        }
    }
    if let Some(x) = payload.get("apiSecret").and_then(Value::as_str) {
        if !x.trim().is_empty() {
            s.api_secret = x.trim().to_string();
        }
    }
    if let Some(b) = payload.get("apiPddRecharged").and_then(Value::as_bool) {
        s.api_pdd_recharged = b;
    }
    if let Some(b) = payload.get("selfhostEnabled").and_then(Value::as_bool) {
        s.selfhost_enabled = b;
    }
    if let Some(x) = payload.get("selfhostEndpoint").and_then(Value::as_str) {
        s.selfhost_endpoint = x.trim().trim_end_matches('/').to_string();
    }
    if let Some(x) = payload.get("selfhostToken").and_then(Value::as_str) {
        if !x.trim().is_empty() {
            s.selfhost_token = x.trim().to_string();
        }
    }
    if let Some(b) = payload.get("fallbackBToA").and_then(Value::as_bool) {
        s.fallback_b_to_a = b;
    }
    if payload.get("dailyApiBudgetFen").is_some() {
        s.daily_api_budget_fen = payload
            .get("dailyApiBudgetFen")
            .and_then(Value::as_i64)
            .filter(|n| *n > 0);
    }
    s
}

// ————————————————————— 通道与错误 —————————————————————

/// 解析失败分类（与设计稿一致，前端据此给提示与「换通道重试」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneErrorKind {
    NotSupported,
    SourceGone,
    RiskBlocked,
    QuotaExceeded,
    Timeout,
    ParseIncomplete,
    Upstream,
}

impl CloneErrorKind {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            CloneErrorKind::NotSupported => "not_supported",
            CloneErrorKind::SourceGone => "source_gone",
            CloneErrorKind::RiskBlocked => "risk_blocked",
            CloneErrorKind::QuotaExceeded => "quota_exceeded",
            CloneErrorKind::Timeout => "timeout",
            CloneErrorKind::ParseIncomplete => "parse_incomplete",
            CloneErrorKind::Upstream => "upstream",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CloneError {
    pub kind: CloneErrorKind,
    pub message: String,
}

impl CloneError {
    fn new(kind: CloneErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// 平台 × 通道支持矩阵（与 `GET /productClone/channels` 下发同源）。
#[must_use]
pub fn support_level(channel: ChannelId, platform: Platform) -> SupportLevel {
    use ChannelId as C;
    use Platform as P;
    use SupportLevel as S;
    match (channel, platform) {
        (C::Api, P::Meituan) => S::None,
        (C::Api, _) => S::Full,
        (C::SelfHosted, P::Taobao) | (C::SelfHosted, P::Tmall) => S::Full,
        (C::SelfHosted, P::Jd) | (C::SelfHosted, P::Pdd) | (C::SelfHosted, P::Ali1688) => S::Partial,
        (C::SelfHosted, _) => S::None,
        (C::Official, _) => S::None,
    }
}

/// 通道 A 单次调用费用预估（分；按设计稿预算口径：淘宝/1688≈0.02 元、京东封顶 0.15 元）。
fn api_cost_fen(platform: Platform) -> i32 {
    match platform {
        Platform::Jd => 15,
        Platform::Pdd => 5,
        Platform::Amazon => 5,
        _ => 2,
    }
}

/// 万邦平台路径名。
fn onebound_platform(platform: Platform) -> &'static str {
    match platform {
        Platform::Taobao | Platform::Tmall => "taobao",
        Platform::Jd => "jd",
        Platform::Pdd => "pinduoduo",
        Platform::Ali1688 => "1688",
        Platform::Amazon => "amazon",
        Platform::Meituan => "meituan",
    }
}

fn http_client(timeout_secs: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36")
        .build()
        .unwrap_or_default()
}

/// 通道 A：万邦 onebound 薄客户端（普通 REST，照官方文档自写）。
struct OneboundChannel {
    key: String,
    secret: String,
}

impl OneboundChannel {
    async fn fetch_raw(&self, platform: Platform, item_id: &str) -> Result<Value, CloneError> {
        if self.key.is_empty() || self.secret.is_empty() {
            return Err(CloneError::new(
                CloneErrorKind::QuotaExceeded,
                "通道 A 未配置万邦 key/secret（请到 设置 → 商品复刻设置 填写）",
            ));
        }
        let url = format!(
            "https://api-gw.onebound.cn/{}/item_get/",
            onebound_platform(platform)
        );
        let resp = http_client(30)
            .get(&url)
            .query(&[
                ("key", self.key.as_str()),
                ("secret", self.secret.as_str()),
                ("num_iid", item_id),
                ("lang", "zh-CN"),
                ("cache", "no"),
            ])
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    CloneError::new(CloneErrorKind::Timeout, format!("万邦接口超时: {e}"))
                } else {
                    CloneError::new(CloneErrorKind::Upstream, format!("万邦接口请求失败: {e}"))
                }
            })?;
        let body: Value = resp.json().await.map_err(|e| {
            CloneError::new(CloneErrorKind::Upstream, format!("万邦返回不可解析: {e}"))
        })?;
        let code = body.get("error_code").and_then(|v| {
            v.as_str().map(str::to_string).or_else(|| v.as_i64().map(|n| n.to_string()))
        });
        if code.as_deref() != Some("0000") {
            let reason = body
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("未知错误")
                .to_string();
            let kind = if reason.contains("余额") || reason.contains("balance") {
                CloneErrorKind::QuotaExceeded
            } else if reason.contains("下架") || reason.contains("不存在") || reason.contains("删除") {
                CloneErrorKind::SourceGone
            } else {
                CloneErrorKind::Upstream
            };
            return Err(CloneError::new(kind, format!("万邦接口错误（{}）: {reason}", code.unwrap_or_default())));
        }
        Ok(body)
    }
}

/// 通道 B：抓取边车网关（边车只取数，归一在本服务）。
struct ScraperGateway {
    endpoint: String,
    token: String,
}

impl ScraperGateway {
    async fn fetch_raw(&self, platform: Platform, url: &str, item_id: &str) -> Result<Value, CloneError> {
        if self.endpoint.is_empty() {
            return Err(CloneError::new(
                CloneErrorKind::NotSupported,
                "通道 B 边车地址未配置（请到 设置 → 商品复刻设置 填写）",
            ));
        }
        // 一期边车只实现淘宝/天猫 MTOP；其余平台二期（浏览器路线/1688 适配）。
        if !matches!(platform, Platform::Taobao | Platform::Tmall) {
            return Err(CloneError::new(
                CloneErrorKind::NotSupported,
                format!("通道 B 的{}抓取二期上线（一期仅淘宝/天猫）", platform.label()),
            ));
        }
        let resp = http_client(60)
            .post(format!("{}/scrape", self.endpoint))
            .header("X-Sidecar-Token", &self.token)
            .json(&json!({"platform": platform.as_str(), "url": url, "item_id": item_id}))
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    CloneError::new(CloneErrorKind::Timeout, format!("抓取边车超时: {e}"))
                } else {
                    CloneError::new(CloneErrorKind::RiskBlocked, format!("抓取边车不可达: {e}"))
                }
            })?;
        let status = resp.status();
        let body: Value = resp.json().await.unwrap_or(Value::Null);
        if !status.is_success() {
            let kind = match body.get("kind").and_then(Value::as_str) {
                Some("risk_blocked") => CloneErrorKind::RiskBlocked,
                Some("source_gone") => CloneErrorKind::SourceGone,
                Some("timeout") => CloneErrorKind::Timeout,
                _ => CloneErrorKind::Upstream,
            };
            let msg = body
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("边车抓取失败")
                .to_string();
            return Err(CloneError::new(kind, msg));
        }
        Ok(body.get("raw").cloned().unwrap_or(Value::Null))
    }

    async fn health(&self) -> Value {
        if self.endpoint.is_empty() {
            return json!({"alive": false, "platforms": {}});
        }
        let resp = http_client(5)
            .get(format!("{}/healthz", self.endpoint))
            .header("X-Sidecar-Token", &self.token)
            .send()
            .await;
        match resp {
            Ok(r) => r.json().await.unwrap_or(json!({"alive": false, "platforms": {}})),
            Err(_) => json!({"alive": false, "platforms": {}}),
        }
    }
}

// ————————————————————— 归一层 —————————————————————

fn s(v: &Value) -> String {
match v {
Value::String(x) => x.trim().to_string(),
Value::Number(n) => n.to_string(),
_ => String::new(),
}
}

fn money_str(v: &Value) -> Option<String> {
let raw = s(v);
if raw.is_empty() {
return None;
}
Decimal::from_str(raw.split('-').next().unwrap_or(&raw).trim())
.ok()
.map(|d| format!("{d:.2}"))
}

/// 标题清洗：去首尾空白、折叠空白、剥掉开头的/[…] 营销前缀（最多 2 个）。
#[must_use]
pub fn clean_title(raw: &str) -> String {
let mut t = raw.split_whitespace().collect::<Vec<_>>().join(" ");
for _ in 0..2 {
let trimmed = t.trim_start();
if let Some(rest) = trimmed.strip_prefix('\u{3010}') {
if let Some(end) = rest.find('\u{3011}') {
t = rest[end + '\u{3011}'.len_utf8()..].trim_start().to_string();
continue;
}
}
if let Some(rest) = trimmed.strip_prefix('[') {
if let Some(end) = rest.find(']') {
t = rest[end + 1..].trim_start().to_string();
continue;
}
}
break;
}
t.chars().take(250).collect()
}

fn fix_url(u: &str) -> String {
let u = u.trim();
if let Some(rest) = u.strip_prefix("//") {
format!("https://{rest}")
} else {
u.to_string()
}
}

fn push_unique(list: &mut Vec<String>, u: String) {
let u = fix_url(&u);
if!u.is_empty() &&!list.contains(&u) {
list.push(u);
}
}

/// 解析万邦 `properties_name`（`pid:vid:名:值;pid:vid:名:值`）为规格组。
fn parse_properties_name(pn: &str) -> Vec<CloneSpec> {
pn.split(';')
.filter(|seg|!seg.trim().is_empty())
.filter_map(|seg| {
let parts: Vec<&str> = seg.splitn(4, ':').collect();
if parts.len() == 4 {
Some(CloneSpec {
name: parts[2].trim().to_string(),
value: parts[3].trim().to_string(),
})
} else {
None
}
})
.collect()
}

/// 万邦 item_get 的 item 对象 → CloneDraft（各平台字段大体同构，防御式读取）。
#[must_use]
pub fn normalize_onebound(platform: Platform, channel: ChannelId, url: &str, item_id: &str, item: &Value) -> CloneDraft {
let mut draft = CloneDraft {
source_platform: platform.as_str().to_string(),
source_channel: channel.as_str().to_string(),
source_url: url.to_string(),
source_item_id: item_id.to_string(),
..Default::default()
};
draft.title = clean_title(&s(&item["title"]));

// 主图：pic_url + item_imgs[]
let mut images: Vec<String> = Vec::new();
push_unique(&mut images, s(&item["pic_url"]));
if let Some(arr) = item.get("item_imgs").and_then(Value::as_array) {
for im in arr {
let u = if im.is_string() { s(im)} else { s(&im["url"])};
push_unique(&mut images, u);
}
}
draft.main_images = images;

// 规格图映射：props_img 键为 `pid:vid` 或完整 properties 串。
let mut props_img: HashMap<String, String> = HashMap::new();
if let Some(obj) = item.get("props_img").and_then(Value::as_object) {
for (k, v) in obj {
props_img.insert(k.clone(), s(v));
}
}

// SKU
let mut skus: Vec<CloneSku> = Vec::new();
if let Some(sku_arr) = item
.get("skus")
.and_then(|x| x.get("sku"))
.and_then(Value::as_array)
{
for row in sku_arr {
let pn = s(&row["properties_name"]);
let specs = parse_properties_name(&pn);
// 规格图：按每个规格段的 pid:vid 逐个查 props_img
let mut image: Option<String> = None;
for seg in pn.split(';') {
let parts: Vec<&str> = seg.splitn(4, ':').collect();
if parts.len() >= 2 {
let key = format!("{}:{}", parts[0], parts[1]);
if let Some(u) = props_img.get(&key) {
image = Some(fix_url(u));
break;
}
}
}
if image.is_none() {
if let Some(u) = props_img.get(&s(&row["properties"])) {
image = Some(fix_url(u));
}
}
skus.push(CloneSku {
specs,
price: money_str(&row["price"]).unwrap_or_default(),
original_price: money_str(&row["orginal_price"]).or_else(|| money_str(&row["original_price"])),
stock: row.get("quantity").and_then(|v| v.as_i64().or_else(|| s(v).parse().ok())),
image,
});
}
}
// 价格
let item_price = money_str(&item["promotion_price"])
.or_else(|| money_str(&item["price"]))
.unwrap_or_default();
let item_original = money_str(&item["orginal_price"])
.or_else(|| money_str(&item["original_price"]))
.or_else(|| money_str(&item["price"]));
if skus.is_empty() {
skus.push(CloneSku {
specs: Vec::new(),
price: item_price.clone(),
original_price: item_original.clone(),
stock: item.get("num").and_then(|v| v.as_i64().or_else(|| s(v).parse().ok())),
image: None,
});
}
draft.price = skus
.iter()
.filter_map(|k| Decimal::from_str(&k.price).ok())
.min()
.map(|d| format!("{d:.2}"))
.unwrap_or(item_price);
draft.original_price = item_original;
draft.stock = {
let total: i64 = skus.iter().filter_map(|k| k.stock).sum();
if skus.iter().all(|k| k.stock.is_some()) {
Some(total)
} else {
item.get("num").and_then(|v| v.as_i64().or_else(|| s(v).parse().ok()))
}
};
draft.skus = skus;

// 参数：props 数组 [{name,value}] 或 props_list 对象（值形如「品牌: xxx」）
let mut props: Vec<CloneProp> = Vec::new();
if let Some(arr) = item.get("props").and_then(Value::as_array) {
for p in arr {
let (n, v) = (s(&p["name"]), s(&p["value"]));
if!n.is_empty() &&!v.is_empty() {
props.push(CloneProp { name: n, value: v});
}
}
}
if props.is_empty() {
if let Some(obj) = item.get("props_list").and_then(Value::as_object) {
for v in obj.values() {
let text = s(v);
if let Some((n, val)) = text.split_once(':').or_else(|| text.split_once('：')) {
let (n, val) = (n.trim().to_string(), val.trim().to_string());
if!n.is_empty() &&!val.is_empty() {
props.push(CloneProp { name: n, value: val});
}
}
}
}
}
draft.props = props;

draft.detail_html = s(&item["desc"]);
draft.shop_name = {
let n = s(&item["shop_name"]);
if!n.is_empty() {
Some(n)
} else {
let n2 = s(&item["seller_info"]["shop_name"]);
if n2.is_empty() { None} else { Some(n2)}
}
};
draft.refresh_completeness();
draft
}

/// 淘宝 MTOP getdetail 数据 → CloneDraft（边车原始返回，防御式读取）。
#[must_use]
pub fn normalize_mtop_taobao(platform: Platform, channel: ChannelId, url: &str, item_id: &str, data: &Value) -> CloneDraft {
let mut draft = CloneDraft {
source_platform: platform.as_str().to_string(),
source_channel: channel.as_str().to_string(),
source_url: url.to_string(),
source_item_id: item_id.to_string(),
..Default::default()
};
let item = &data["item"];
draft.title = clean_title(&s(&item["title"]));
let mut images: Vec<String> = Vec::new();
if let Some(arr) = item.get("images").and_then(Value::as_array) {
for im in arr {
push_unique(&mut images, s(im));
}
}
draft.main_images = images;

// 规格维度：skuBase.props [{name, values:[{name, image}]}]
let mut dim_names: HashMap<String, String> = HashMap::new(); // pid -> name
let mut val_names: HashMap<String, (String, Option<String>)> = HashMap::new(); // "pid:vid" -> (value, image)
if let Some(props) = data.get("skuBase").and_then(|x| x.get("props")).and_then(Value::as_array) {
for p in props {
let pid = s(&p["pid"]);
dim_names.insert(pid.clone(), s(&p["name"]));
if let Some(vals) = p.get("values").and_then(Value::as_array) {
for v in vals {
let vid = s(&v["vid"]);
let img = {
let i = s(&v["image"]);
if i.is_empty() { None} else { Some(fix_url(&i))}
};
val_names.insert(format!("{pid}:{vid}"), (s(&v["name"]), img));
}
}
}
}
// SKU 行：skuBase.skus [{propPath, skuId}] + skuCore.sku2info[skuId] {price.priceText, quantity}
let sku2info = data.get("skuCore").and_then(|x| x.get("sku2info"));
let mut skus: Vec<CloneSku> = Vec::new();
if let Some(arr) = data.get("skuBase").and_then(|x| x.get("skus")).and_then(Value::as_array) {
for row in arr {
let sku_id = s(&row["skuId"]);
let mut specs: Vec<CloneSpec> = Vec::new();
let mut image: Option<String> = None;
for seg in s(&row["propPath"]).split(';') {
let seg = seg.trim();
if seg.is_empty() {
continue;
}
if let Some((val, img)) = val_names.get(seg) {
let pid = seg.split(':').next().unwrap_or("");
specs.push(CloneSpec {
name: dim_names.get(pid).cloned().unwrap_or_default(),
value: val.clone(),
});
if image.is_none() {
image = img.clone();
}
}
}
let info = sku2info.and_then(|m| m.get(&sku_id));
let price = info
.and_then(|i| money_str(&i["price"]["priceText"]).or_else(|| money_str(&i["price"]["price"])))
.unwrap_or_default();
let stock = info.and_then(|i| i.get("quantity")).and_then(|v| v.as_i64().or_else(|| s(v).parse().ok()));
skus.push(CloneSku {
specs,
price,
original_price: None,
stock,
image,
});
}
}
if skus.is_empty() {
let price = money_str(&data["price"]["priceText"]).unwrap_or_default();
skus.push(CloneSku {
specs: Vec::new(),
price: price.clone(),
original_price: None,
stock: None,
image: None,
});
draft.price = price;
} else {
draft.price = skus
.iter()
.filter_map(|k| Decimal::from_str(&k.price).ok())
.min()
.map(|d| format!("{d:.2}"))
.unwrap_or_default();
}
draft.stock = if skus.iter().all(|k| k.stock.is_some()) {
Some(skus.iter().filter_map(|k| k.stock).sum())
} else {
None
};
draft.skus = skus;
draft.detail_html = s(&data["desc"]).if_empty_then(s(&data["detailDesc"]));
draft.shop_name = {
let n = s(&data["seller"]["shopName"]);
if n.is_empty() { None} else { Some(n)}
};
draft.refresh_completeness();
draft
}

trait StrExt {
fn if_empty_then(self, other: String) -> String;
}
impl StrExt for String {
fn if_empty_then(self, other: String) -> String {
if self.is_empty() { other} else { self}
}
}

// ————————————————————— 服务主体 —————————————————————

pub struct ProductCloneService {
repos: Arc<Repositories>,
product_manage: Arc<ProductManageService>,
}

impl ProductCloneService {
#[must_use]
pub fn new(repos: Arc<Repositories>, product_manage: Arc<ProductManageService>) -> Self {
Self {
repos,
product_manage,
}
}

async fn load_settings(&self) -> Result<CloneSettings> {
let stored = self
.repos
.system
.config_basic
.find_by_code(CLONE_SETTINGS_CODE)
.await?;
Ok(match stored {
Some(text) => serde_json::from_str::<Value>(&text)
.map(|v| CloneSettings::from_json(&v))
.unwrap_or_default(),
None => CloneSettings::default(),
})
}

pub async fn get_settings(&self) -> Result<Value> {
Ok(masked_settings(&self.load_settings().await?))
}

pub async fn save_settings(&self, payload: &Value) -> Result<Value> {
let old = self.load_settings().await?;
let merged = merge_settings_save(&old, payload);
self.repos
.system
.config_basic
.save_code(CLONE_SETTINGS_CODE, &merged.to_json().to_string())
.await?;
Ok(masked_settings(&merged))
}

/// 通道矩阵 + 健康状态（前端通道选择与平台置灰同源）。
pub async fn channels_info(&self) -> Result<Value> {
let settings = self.load_settings().await?;
let gateway = ScraperGateway {
endpoint: settings.selfhost_endpoint.clone(),
token: settings.selfhost_token.clone(),
};
let b_health = if settings.selfhost_enabled {
gateway.health().await
} else {
json!({"alive": false, "platforms": {}})
};
let mut matrix = Vec::new();
for platform in Platform::all() {
matrix.push(json!({
"platform": platform.as_str(),
"label": platform.label(),
"api": support_level(ChannelId::Api, platform),
"selfhost": support_level(ChannelId::SelfHosted, platform),
"defaultChannel": settings.default_channel_for(platform).as_str(),
}));
}
Ok(json!({
"matrix": matrix,
"defaultChannel": settings.default_channel,
"api": {
"enabled": settings.api_enabled,
"configured":!settings.api_key.is_empty() &&!settings.api_secret.is_empty(),
"pddRecharged": settings.api_pdd_recharged,
},
"selfhost": {
"enabled": settings.selfhost_enabled,
"configured":!settings.selfhost_endpoint.is_empty(),
"health": b_health,
},
"fallbackBToA": settings.fallback_b_to_a,
}))
}

/// 发起解析：建任务后后台执行，立即返回任务列表（含即时失败的条目）。
pub async fn create_fetch_tasks(
self: &Arc<Self>,
dto: &CloneFetchDTO,
created_by: &str,
) -> Result<Vec<Value>> {
if dto.urls.is_empty() || dto.urls.len() > 20 {
return Err(Error::biz(400, "请提供 1–20 条商品链接"));
}
let settings = self.load_settings().await?;
let now = nextrade_common::utils::now_seconds();
let mut out: Vec<Value> = Vec::new();
for raw_url in &dto.urls {
let url = raw_url.trim().to_string();
if url.is_empty() {
continue;
}
let Some((platform, item_id)) = parse_source_url(&url) else {
out.push(json!({"url": url, "error": "无法识别的链接（不支持的平台或缺少商品ID）"}));
continue;
};
let channel = dto
.channel
.as_deref()
.and_then(ChannelId::from_str)
.unwrap_or_else(|| settings.default_channel_for(platform));
let task_id = self
.repos
.catalog
.product_clone
.insert_task(platform.as_str(), channel.as_str(), &url, &item_id, created_by, now)
.await?;
// 同源查重提示（不拦截，前端展示）
let duplicate = self
.repos
.catalog
.product_clone
.find_by_source(platform.as_str(), &item_id)
.await?
.filter(|(id, _, _)| *id!= task_id)
.map(|(id, status, pid)| json!({"taskId": id, "status": status, "productId": pid}));
out.push(json!({"taskId": task_id, "platform": platform.as_str(), "channel": channel.as_str(), "duplicate": duplicate}));

// 前置校验：不支持 / 通道未启用 / 日预算 → 直接落失败，不调通道
let pre_fail: Option<CloneError> = if item_id.is_empty() || support_level(channel, platform) == SupportLevel::None {
Some(CloneError::new(
CloneErrorKind::NotSupported,
format!("{} 在{}通道不支持复刻", platform.label(), if channel == ChannelId::Api { "数据接口"} else { "自建抓取"}),
))
} else if channel == ChannelId::Api &&!settings.api_enabled {
Some(CloneError::new(CloneErrorKind::NotSupported, "数据接口通道已被管理员关闭"))
} else if channel == ChannelId::SelfHosted &&!settings.selfhost_enabled {
Some(CloneError::new(CloneErrorKind::NotSupported, "自建抓取通道未启用（请到 设置 → 商品复刻设置 开启）"))
} else if channel == ChannelId::Api {
if let Some(budget) = settings.daily_api_budget_fen {
let day_start = now - (now % 86_400);
let used = self.repos.catalog.product_clone.today_api_cost_fen(day_start).await?;
if used + i64::from(api_cost_fen(platform)) > budget {
Some(CloneError::new(CloneErrorKind::QuotaExceeded, "已达通道 A 日费用上限"))
} else {
None
}
} else {
None
}
} else {
None
};
if let Some(err) = pre_fail {
let attempts = json!([{
"channel": channel.as_str(), "startedAt": now, "result": "failed",
"errorKind": err.kind.as_str(), "errorMsg": err.message, "costFen": 0
}]);
self.repos
.catalog
.product_clone
.finish_attempt(task_id, task_status::FAILED, "", None, None, &attempts, 0, &err.message, now)
.await?;
continue;
}
let svc = self.clone();
tokio::spawn(async move {
if let Err(e) = svc.run_task(task_id, platform, channel, url, item_id, Vec::new(), 0).await {
tracing::warn!("商品复刻任务 {task_id} 执行异常: {e}");
}
});
}
Ok(out)
}

/// 执行一个任务的取数+归一（含 B→A 单向自动转，当设置开启）。
async fn run_task(
&self,
task_id: i64,
platform: Platform,
channel: ChannelId,
url: String,
item_id: String,
init_attempts: Vec<Value>,
init_cost: i32,
) -> Result<()> {
let settings = self.load_settings().await?;
let mut attempts: Vec<Value> = init_attempts;
let mut total_cost: i32 = init_cost;
let mut current = channel;
loop {
let started = nextrade_common::utils::now_seconds();
let attempt_cost = if current == ChannelId::Api { api_cost_fen(platform)} else { 0};
let fetched = self.fetch_via(current, &settings, platform, &url, &item_id).await;
match fetched {
Ok((draft, raw)) => {
total_cost += attempt_cost;
attempts.push(json!({
"channel": current.as_str(), "startedAt": started, "result": "ok",
"errorKind": null, "errorMsg": null, "costFen": attempt_cost
}));
let now = nextrade_common::utils::now_seconds();
let draft_json = serde_json::to_value(&draft).unwrap_or(Value::Null);
self.repos
.catalog
.product_clone
.finish_attempt(
task_id,
task_status::READY,
&draft.title,
Some(&draft_json),
Some(&raw),
&Value::Array(attempts),
total_cost,
"",
now,
)
.await?;
return Ok(());
}
Err(err) => {
total_cost += attempt_cost;
attempts.push(json!({
"channel": current.as_str(), "startedAt": started, "result": "failed",
"errorKind": err.kind.as_str(), "errorMsg": err.message, "costFen": attempt_cost
}));
// B 失败且开启自动转 A：仅此一向、仅一次
if current == ChannelId::SelfHosted
&& settings.fallback_b_to_a
&& settings.api_enabled
&& support_level(ChannelId::Api, platform)!= SupportLevel::None
{
current = ChannelId::Api;
continue;
}
let now = nextrade_common::utils::now_seconds();
self.repos
.catalog
.product_clone
.finish_attempt(
task_id,
task_status::FAILED,
"",
None,
None,
&Value::Array(attempts),
total_cost,
&err.message,
now,
)
.await?;
return Ok(());
}
}
}
}

async fn fetch_via(
&self,
channel: ChannelId,
settings: &CloneSettings,
platform: Platform,
url: &str,
item_id: &str,
) -> Result<(CloneDraft, Value), CloneError> {
match channel {
ChannelId::Api => {
let client = OneboundChannel {
key: settings.api_key.clone(),
secret: settings.api_secret.clone(),
};
let body = client.fetch_raw(platform, item_id).await?;
let item = body.get("item").cloned().unwrap_or(Value::Null);
if item.is_null() {
return Err(CloneError::new(CloneErrorKind::ParseIncomplete, "万邦返回缺少 item 数据"));
}
let draft = normalize_onebound(platform, channel, url, item_id, &item);
if draft.title.is_empty() || draft.price.is_empty() || draft.price == "0.00" {
return Err(CloneError::new(CloneErrorKind::ParseIncomplete, "解析结果缺标题或价格，判失败"));
}
Ok((draft, body))
}
ChannelId::SelfHosted => {
let gateway = ScraperGateway {
endpoint: settings.selfhost_endpoint.clone(),
token: settings.selfhost_token.clone(),
};
let raw = gateway.fetch_raw(platform, url, item_id).await?;
let draft = normalize_mtop_taobao(platform, channel, url, item_id, &raw);
if draft.title.is_empty() || draft.price.is_empty() || draft.price == "0.00" {
return Err(CloneError::new(
CloneErrorKind::ParseIncomplete,
"边车返回数据缺标题或价格（可能触发风控/登录失效），判失败",
));
}
Ok((draft, raw))
}
ChannelId::Official => Err(CloneError::new(
CloneErrorKind::NotSupported,
"官方开放平台通道未实现（预留）",
)),
}
}

/// 换通道重试：把任务打回 parsing 后以指定通道重跑（attempts 追加留痕）。
pub async fn retry_task(self: &Arc<Self>, dto: &CloneRetryDTO, _created_by: &str) -> Result<()> {
let channel = ChannelId::from_str(&dto.channel)
.ok_or_else(|| Error::biz(400, "通道不合法"))?;
let row = self
.repos
.catalog
.product_clone
.find_task(dto.task_id)
.await?
.ok_or_else(|| Error::biz(404, "任务不存在"))?;
if row.status == task_status::IMPORTED {
return Err(Error::biz(400, "任务已导入，不能重试"));
}
let platform = Platform::from_str(&row.platform)
.ok_or_else(|| Error::biz(400, "任务平台不合法"))?;
if support_level(channel, platform) == SupportLevel::None {
return Err(Error::biz(400, "该平台在所选通道不支持复刻"));
}
let now = nextrade_common::utils::now_seconds();
self.repos
.catalog
.product_clone
.reopen_for_retry(dto.task_id, channel.as_str(), now)
.await?;
let svc = self.clone();
let (url, item_id) = (row.source_url.clone(), row.source_item_id.clone());
tokio::spawn(async move {
// 重跑时保留旧 attempts：run_task 会重写 attempts，这里先读旧值拼回
let _ = svc.run_task_keep_attempts(dto.task_id, platform, channel, url, item_id).await;
});
Ok(())
}

/// 重试入口：读旧 attempts/费用后交给 run_task 续写（留痕不丢）。
async fn run_task_keep_attempts(
&self,
task_id: i64,
platform: Platform,
channel: ChannelId,
url: String,
item_id: String,
) -> Result<()> {
let (prev_attempts, prev_cost) = self
.repos
.catalog
.product_clone
.find_task(task_id)
.await?
.map(|r| (r.attempts.as_array().cloned().unwrap_or_default(), r.cost_fen))
.unwrap_or_default();
self.run_task(task_id, platform, channel, url, item_id, prev_attempts, prev_cost).await
}

pub async fn list_tasks(&self, q: &CloneTaskQuery) -> Result<PageResult<Value>> {
let page = q.page.unwrap_or(1).max(1);
let page_size = q.page_size.unwrap_or(20).clamp(1, 100);
let (rows, total) = self
.repos
.catalog
.product_clone
.list_tasks(
q.status.as_deref().filter(|x|!x.is_empty()),
q.platform.as_deref().filter(|x|!x.is_empty()),
page_size,
(page - 1) * page_size,
)
.await?;
let records: Vec<Value> = rows
.into_iter()
.map(|r| serde_json::to_value(r.into_vo(false)).unwrap_or(Value::Null))
.collect();
Ok(PageResult::new(records, total, page, page_size))
}

pub async fn task_detail(&self, id: i64) -> Result<Value> {
let row = self
.repos
.catalog
.product_clone
.find_task(id)
.await?
.ok_or_else(|| Error::biz(404, "任务不存在"))?;
Ok(serde_json::to_value(row.into_vo(true)).unwrap_or(Value::Null))
}
}

// ————————————————————— 导入落库 —————————————————————

fn dec(s: &str) -> Option<Decimal> {
Decimal::from_str(s.trim()).ok()
}

fn apply_price_rule(money: &str, rule: Option<&nextrade_core::models::catalog::product_clone::ClonePriceRule>) -> String {
let Some(rule) = rule else { return money.to_string()};
let (Some(mut d), Some(v)) = (dec(money), rule.value.as_deref().and_then(dec)) else {
return money.to_string();
};
match rule.mode.as_str() {
"coefficient" => d *= v,
"add" => d += v,
_ => return money.to_string(),
}
if d < Decimal::ZERO {
d = Decimal::ZERO;
}
format!("{d:.2}")
}

/// 从详情 HTML 里抽出全部 img src（转存替换用，简单扫描即可）。
fn extract_img_srcs(html: &str) -> Vec<String> {
let mut out = Vec::new();
let bytes = html.as_bytes();
let mut i = 0;
while i + 5 < bytes.len() {
if &html[i..i + 4] == "src=" {
let q = html.as_bytes()[i + 4];
if q == b'"' || q == b'\'' {
let start = i + 5;
if let Some(end) = html[start..].find(q as char) {
let u = &html[start..start + end];
if u.starts_with("http") &&!out.contains(&u.to_string()) {
out.push(u.to_string());
}
i = start + end;
continue;
}
}
}
i += 1;
}
out
}

fn transfer_filename(url: &str, idx: usize) -> String {
let path = url.split(['?', '#']).next().unwrap_or("");
let last = path.rsplit('/').next().unwrap_or("");
let clean: String = last
.chars()
.filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
.collect();
if clean.contains('.') && clean.len() > 3 {
clean
} else {
format!("clone_{idx}.jpg")
}
}

impl ProductCloneService {
/// 图片转存：下载源站图片 → 现有存储管线（OSS/local）→ 返回 (原URL→新URL 映射, 失败数)。
async fn transfer_images(&self, urls: &[String]) -> (HashMap<String, String>, i32) {
let mut map: HashMap<String, String> = HashMap::new();
let mut failed = 0;
let client = http_client(20);
for (idx, url) in urls.iter().enumerate() {
if map.contains_key(url) {
continue;
}
let host = url
.split('/')
.nth(2)
.map(|h| format!("https://{h}/"))
.unwrap_or_default();
let result = async {
let resp = client.get(url).header("Referer", &host).send().await.map_err(|e| e.to_string())?;
if!resp.status().is_success() {
return Err(format!("HTTP {}", resp.status()));
}
let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
if bytes.len() < 100 || bytes.len() > 15 * 1024 * 1024 {
return Err("图片大小异常".to_string());
}
nextrade_infrastructure::storage::save_local_upload("gallery", &transfer_filename(url, idx), &bytes)
.await
.map_err(|e| e.to_string())
}
.await;
match result {
Ok(new_url) => {
map.insert(url.clone(), new_url);
}
Err(e) => {
failed += 1;
tracing::warn!("复刻图片转存失败 {url}: {e}");
}
}
}
(map, failed)
}

/// 确认导入：价格规则 → 图片转存 → 组装 ProductSaveDTO → 复用商品创建服务落库。
pub async fn import_task(&self, dto: &CloneImportDTO, shop_id: Option<i32>) -> Result<CloneImportResult> {
if dto.category_id <= 0 {
return Err(Error::biz(400, "请选择商品类目"));
}
let task = self
.repos
.catalog
.product_clone
.find_task(dto.task_id)
.await?
.ok_or_else(|| Error::biz(404, "任务不存在"))?;
if task.status == task_status::IMPORTED &&!dto.force.unwrap_or(false) {
return Err(Error::biz(400, "该任务已导入过商品，如需再建一份请勾选强制新建"));
}
if task.status!= task_status::READY && task.status!= task_status::IMPORTED {
return Err(Error::biz(400, "任务不在可导入状态（需解析成功）"));
}
let mut draft = dto.draft.clone();
if draft.title.trim().is_empty() {
return Err(Error::biz(400, "商品名称不能为空"));
}

// 价格规则
let rule = dto.price_rule.as_ref();
draft.price = apply_price_rule(&draft.price, rule);
if let Some(op) = draft.original_price.clone() {
draft.original_price = Some(apply_price_rule(&op, rule));
}
for sku in &mut draft.skus {
sku.price = apply_price_rule(&sku.price, rule);
if let Some(op) = sku.original_price.clone() {
sku.original_price = Some(apply_price_rule(&op, rule));
}
}

// 图片转存
let mut warnings: Vec<String> = Vec::new();
let mut transfer_failed = 0;
if dto.transfer_images.unwrap_or(true) {
let mut urls: Vec<String> = Vec::new();
for u in &draft.main_images {
if u.starts_with("http") &&!urls.contains(u) {
urls.push(u.clone());
}
}
for sku in &draft.skus {
if let Some(u) = &sku.image {
if u.starts_with("http") &&!urls.contains(u) {
urls.push(u.clone());
}
}
}
for u in extract_img_srcs(&draft.detail_html) {
if!urls.contains(&u) {
urls.push(u);
}
}
let (map, failed) = self.transfer_images(&urls).await;
transfer_failed = failed;
if failed > 0 {
warnings.push(format!("{failed} 张图片转存失败，已保留源站地址，请导入后检查"));
}
let swap = |u: &str| map.get(u).cloned().unwrap_or_else(|| u.to_string());
draft.main_images = draft.main_images.iter().map(|u| swap(u)).collect();
for sku in &mut draft.skus {
if let Some(u) = &sku.image {
sku.image = Some(swap(u));
}
}
// 详情图替换：长 URL 先替换，避免前缀误伤
let mut pairs: Vec<(&String, &String)> = map.iter().collect();
pairs.sort_by_key(|(k, _)| std::cmp::Reverse(k.len()));
for (old, new) in pairs {
draft.detail_html = draft.detail_html.replace(old.as_str(), new.as_str());
}
}

// SN：CLONE-<taskId>，冲突时强制模式下加序号
let base_sn = format!("CLONE-{}", dto.task_id);
let mut sn = base_sn.clone();
if self.repos.catalog.product_manage.count_product_sn(&sn, None).await? > 0 {
if!dto.force.unwrap_or(false) {
return Err(Error::biz(400, format!("已存在 SN 为 {sn} 的商品（该任务可能已导入），如需再建一份请勾选强制新建")));
}
let mut n = 2;
loop {
let cand = format!("{base_sn}-{n}");
if self.repos.catalog.product_manage.count_product_sn(&cand, None).await? == 0 {
sn = cand;
break;
}
n += 1;
if n > 99 {
return Err(Error::biz(400, "SN 冲突过多，请人工处理"));
}
}
}

// 组装 ProductSaveDTO
let multi = draft.skus.len() > 1
|| draft.skus.first().map(|k|!k.specs.is_empty()).unwrap_or(false);
let mut save = ProductSaveDTO {
product_name: Some(draft.title.clone()),
product_sn: Some(sn.clone()),
category_id: Some(dto.category_id),
brand_id: dto.brand_id,
shop_id: shop_id.filter(|x| *x > 0),
product_status: Some(dto.target_status.unwrap_or(0)),
check_status: Some(1),
product_price: dec(&draft.price),
market_price: draft.original_price.as_deref().and_then(dec),
product_desc: if draft.detail_html.trim().is_empty() {
None
} else {
Some(draft.detail_html.clone())
},
..Default::default()
};
if let Some(first) = draft.main_images.first() {
save.pic_url = Some(first.clone());
save.pic_thumb = Some(first.clone());
save.pic_original = Some(first.clone());
}
if!draft.main_images.is_empty() {
save.img_list = Some(
draft
.main_images
.iter()
.enumerate()
.map(|(i, u)| GalleryPicDTO {
pic_url: Some(u.clone()),
pic_thumb: Some(u.clone()),
pic_sort: Some(i as i32),
..Default::default()
})
.collect(),
);
}
if!draft.props.is_empty() {
save.attr_list = Some(ProductAttrsVO {
normal: draft
.props
.iter()
.map(|p| ProductAttrGroupVO {
attr_name: p.name.clone(),
attr_list: vec![ProductAttributeVO {
attr_type: 0,
attr_name: p.name.clone(),
attr_value: p.value.clone(),
..Default::default()
}],
})
.collect(),
spe: Vec::new(),
extra: Vec::new(),
});
}
if multi {
save.sku_type = Some(2);
save.product_list = Some(
draft
.skus
.iter()
.map(|sku| ProductSkuSaveDTO {
sku_data: Some(serde_json::json!(
sku.specs
.iter()
.map(|sp| serde_json::json!({
"name": sp.name,
"value": sp.value,
"attrPicThumb": sku.image.clone().unwrap_or_default(),
}))
.collect::<Vec<_>>()
)),
sku_value: Some(
sku.specs.iter().map(|sp| sp.value.clone()).collect::<Vec<_>>().join(" "),
),
attr_names: Some(sku.specs.iter().map(|sp| sp.name.clone()).collect()),
attr_values: Some(sku.specs.iter().map(|sp| sp.value.clone()).collect()),
attrs: Some(
sku.specs
.iter()
.map(|sp| SkuAttrDTO {
attr_name: Some(sp.name.clone()),
attr_value: Some(sp.value.clone()),
})
.collect(),
),
sku_price: dec(&sku.price),
market_price: sku.original_price.as_deref().and_then(dec),
sku_stock: sku.stock.map(|n| n.clamp(0, i64::from(i32::MAX)) as i32),
is_enabled: Some(1),
..Default::default()
})
.collect(),
);
} else {
save.sku_type = Some(1);
save.product_stock = draft
.stock
.or_else(|| draft.skus.first().and_then(|k| k.stock))
.map(|n| n.clamp(0, i64::from(i32::MAX)) as i32);
}

self.product_manage.create(&save).await?;
let product_id = self
.repos
.catalog
.product_clone
.find_product_id_by_sn(&sn)
.await?
.ok_or_else(|| Error::biz(500, "商品已创建但回查 ID 失败"))?;
let now = nextrade_common::utils::now_seconds();
self.repos
.catalog
.product_clone
.mark_imported(dto.task_id, product_id, now)
.await?;
Ok(CloneImportResult {
product_id,
product_sn: sn,
image_transfer_failed: transfer_failed,
warnings,
})
}
}

#[cfg(test)]
mod tests {
use super::*;

#[test]
fn clean_title_strips_marketing_prefix() {
assert_eq!(clean_title("\u{3010}2024\u{65b0}\u{6b3e}\u{3011}  \u{7537}\u{58eb}  \u{5916}\u{5957} "), "\u{7537}\u{58eb} \u{5916}\u{5957}");
assert_eq!(clean_title("[热卖]商品名"), "商品名");
assert_eq!(clean_title("普通标题"), "普通标题");
}

#[test]
fn normalize_onebound_basic() {
let item = serde_json::json!({
"title": "测试商品",
"pic_url": "//img.alicdn.com/main.jpg",
"item_imgs": [{"url": "//img.alicdn.com/2.jpg"}],
"price": "199.00",
"orginal_price": "299.00",
"num": 50,
"desc": "<p>详情</p>",
"props": [{"name": "品牌", "value": "测试牌"}],
"props_img": {"1627207:28341": "//img.alicdn.com/red.jpg"},
"skus": {"sku": [
{"price": "199.00", "quantity": 30, "properties": "1627207:28341", "properties_name": "1627207:28341:颜色:红色"},
{"price": "209.00", "quantity": 20, "properties": "1627207:28342", "properties_name": "1627207:28342:颜色:蓝色"}
]}
});
let d = normalize_onebound(Platform::Taobao, ChannelId::Api, "https://item.taobao.com/item.htm?id=1", "1", &item);
assert_eq!(d.title, "测试商品");
assert_eq!(d.main_images.len(), 2);
assert_eq!(d.main_images[0], "https://img.alicdn.com/main.jpg");
assert_eq!(d.price, "199.00");
assert_eq!(d.original_price.as_deref(), Some("299.00"));
assert_eq!(d.skus.len(), 2);
assert_eq!(d.skus[0].specs[0].name, "颜色");
assert_eq!(d.skus[0].specs[0].value, "红色");
assert_eq!(d.skus[0].image.as_deref(), Some("https://img.alicdn.com/red.jpg"));
assert_eq!(d.stock, Some(50));
assert!(d.completeness.missing.is_empty());
}

#[test]
fn extract_srcs_works() {
let html = r#"<p><img src="https://a.com/1.jpg"></p><img src='https://b.com/2.png'>"#;
assert_eq!(extract_img_srcs(html), vec!["https://a.com/1.jpg", "https://b.com/2.png"]);
}

#[test]
fn price_rule_applies() {
let rule = nextrade_core::models::catalog::product_clone::ClonePriceRule {
mode: "coefficient".to_string(),
value: Some("1.2".to_string()),
};
assert_eq!(apply_price_rule("100.00", Some(&rule)), "120.00");
let rule2 = nextrade_core::models::catalog::product_clone::ClonePriceRule {
mode: "add".to_string(),
value: Some("-5".to_string()),
};
assert_eq!(apply_price_rule("100.00", Some(&rule2)), "95.00");
assert_eq!(apply_price_rule("100.00", None), "100.00");
}
}
