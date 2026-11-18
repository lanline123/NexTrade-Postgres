//! 商品复刻（跨平台商品搬家）领域模型（2026-11-18 新增，本平台自有契约）。
//!
//! 设计：`~/workspace/product-clone-design/proposal.md` + `channel-design.md`。
//! 双通道（A 第三方数据接口 / B 自建抓取边车）的原始返回都归一到
//! [`CloneDraft`]，下游（预览修正、价格规则、图片转存、落库）不感知通道。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 来源平台。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Taobao,
    Tmall,
    Jd,
    Pdd,
    #[serde(rename = "1688")]
    Ali1688,
    Amazon,
    Meituan,
}

impl Platform {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::Taobao => "taobao",
            Platform::Tmall => "tmall",
            Platform::Jd => "jd",
            Platform::Pdd => "pdd",
            Platform::Ali1688 => "1688",
            Platform::Amazon => "amazon",
            Platform::Meituan => "meituan",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "taobao" => Some(Platform::Taobao),
            "tmall" => Some(Platform::Tmall),
            "jd" => Some(Platform::Jd),
            "pdd" => Some(Platform::Pdd),
            "1688" => Some(Platform::Ali1688),
            "amazon" => Some(Platform::Amazon),
            "meituan" => Some(Platform::Meituan),
            _ => None,
        }
    }

    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Platform::Taobao => "淘宝",
            Platform::Tmall => "天猫",
            Platform::Jd => "京东",
            Platform::Pdd => "拼多多",
            Platform::Ali1688 => "1688",
            Platform::Amazon => "亚马逊",
            Platform::Meituan => "美团",
        }
    }

    /// 全部平台（矩阵下发顺序）。
    #[must_use]
    pub fn all() -> [Platform; 7] {
        [
            Platform::Taobao,
            Platform::Tmall,
            Platform::Jd,
            Platform::Pdd,
            Platform::Ali1688,
            Platform::Amazon,
            Platform::Meituan,
        ]
    }
}

/// 数据通道。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelId {
    /// 通道 A：第三方商品数据 API（万邦 onebound），按次计费。
    Api,
    /// 通道 B：自建抓取边车。
    SelfHosted,
    /// 通道 C：官方开放平台（预留，一期不实现）。
    Official,
}

impl ChannelId {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ChannelId::Api => "api",
            ChannelId::SelfHosted => "selfhost",
            ChannelId::Official => "official",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "api" => Some(ChannelId::Api),
            "selfhost" => Some(ChannelId::SelfHosted),
            "official" => Some(ChannelId::Official),
            _ => None,
        }
    }
}

/// 平台 × 通道支持级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SupportLevel {
    Full,
    Partial,
    None,
}

/// 从商品链接识别平台与商品 ID（纯函数，可单测）。
///
/// 返回 `(平台, 商品ID)`；识别不了平台或抽不出 ID 返回 `None`。
/// 覆盖各平台常见链接形态（含短链域名只认平台、ID 抽不到时返回 None）：
/// - 淘宝/天猫：`item.taobao.com/item.htm?id=`、`detail.tmall.com/item.htm?id=`
/// - 京东：`item.jd.com/<id>.html`、`item.m.jd.com/product/<id>.html`
/// - 拼多多：`goods.html?goods_id=`、`mobile.yangkeduo.com/goods.html?goods_id=`
/// - 1688：`detail.1688.com/offer/<id>.html`
/// - 亚马逊：`/dp/<ASIN>`、`/gp/product/<ASIN>`
/// - 美团：仅识别平台（商品结构不同，一期不支持，ID 恒为空串 → 上层按 None 处理）
pub fn parse_source_url(raw: &str) -> Option<(Platform, String)> {
    let url = raw.trim();
    if url.is_empty() {
        return None;
    }
    let lower = url.to_ascii_lowercase();
    let query_param = |name: &str| -> Option<String> {
        let needle = format!("{name}=");
        let idx = lower.find(&needle)?;
        let rest = &url[idx + needle.len()..];
        let end = rest
            .find(|c| c == '&' || c == '#' || c == '"' || c == '\'')
            .unwrap_or(rest.len());
        let v = rest[..end].trim();
        if v.is_empty() {
            None
        } else {
            Some(v.to_string())
        }
    };
    let path_segment_after = |marker: &str| -> Option<String> {
        let idx = lower.find(marker)?;
        let rest = &url[idx + marker.len()..];
        let end = rest
            .find(|c| c == '.' || c == '/' || c == '?' || c == '#' || c == '&')
            .unwrap_or(rest.len());
        let v = rest[..end].trim();
        if v.is_empty() {
            None
        } else {
            Some(v.to_string())
        }
    };

    if lower.contains("tmall.com") {
        return query_param("id").map(|id| (Platform::Tmall, id));
    }
    if lower.contains("taobao.com") {
        return query_param("id").map(|id| (Platform::Taobao, id));
    }
    if lower.contains("jd.com") {
        if let Some(id) = path_segment_after("jd.com/") {
            if id.chars().all(|c| c.is_ascii_digit()) {
                return Some((Platform::Jd, id));
            }
        }
        if let Some(id) = path_segment_after("/product/") {
            return Some((Platform::Jd, id));
        }
        return query_param("sku").map(|id| (Platform::Jd, id));
    }
    if lower.contains("yangkeduo.com") || lower.contains("pinduoduo.com") {
        return query_param("goods_id").map(|id| (Platform::Pdd, id));
    }
    if lower.contains("1688.com") {
        return path_segment_after("/offer/").map(|id| (Platform::Ali1688, id));
    }
    if lower.contains("amazon.") {
        if let Some(id) = path_segment_after("/dp/") {
            return Some((Platform::Amazon, id));
        }
        if let Some(id) = path_segment_after("/gp/product/") {
            return Some((Platform::Amazon, id));
        }
        return None;
    }
    if lower.contains("meituan.com") {
        // 一期两通道均不支持美团：识别平台但无 ID，上层据此给「不支持」提示。
        return Some((Platform::Meituan, String::new()));
    }
    None
}

/// 归一后的 SKU 一行。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneSku {
    /// 规格组合（有序）。
    pub specs: Vec<CloneSpec>,
    /// 售价（元，字符串，两位小数）。
    pub price: String,
    /// 市场价/原价（元），取不到为空。
    pub original_price: Option<String>,
    /// 库存，取不到为 None。
    pub stock: Option<i64>,
    /// 规格图（源站地址，转存前）。
    pub image: Option<String>,
}

/// 规格键值。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneSpec {
    pub name: String,
    pub value: String,
}

/// 商品参数键值。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneProp {
    pub name: String,
    pub value: String,
}

/// 完整度：核心项（主图/详情/SKU）缺失清单。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneCompleteness {
    pub missing: Vec<String>,
}

/// 统一草稿（两通道归一目标，下游与通道无关）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneDraft {
    pub source_platform: String,
    pub source_channel: String,
    pub source_url: String,
    pub source_item_id: String,
    pub title: String,
    pub main_images: Vec<String>,
    /// 图文详情 HTML（图片仍是源站地址，导入时统一转存替换）。
    pub detail_html: String,
    /// 商品售价（元）：多 SKU 时为最低 SKU 价。
    pub price: String,
    /// 商品市场价（元），取不到为空。
    pub original_price: Option<String>,
    pub skus: Vec<CloneSku>,
    pub props: Vec<CloneProp>,
    /// 单品库存（多 SKU 时为汇总）。
    pub stock: Option<i64>,
    /// 来源店铺名（仅溯源展示，不落商品）。
    pub shop_name: Option<String>,
    pub completeness: CloneCompleteness,
}

impl CloneDraft {
    /// 按当前内容重算完整度（核心项：主图 mainImages / 详情 detail / SKU skus）。
    pub fn refresh_completeness(&mut self) {
        let mut missing = Vec::new();
        if self.main_images.is_empty() {
            missing.push("mainImages".to_string());
        }
        if self.detail_html.trim().is_empty() {
            missing.push("detail".to_string());
        }
        if self.skus.is_empty() {
            missing.push("skus".to_string());
        }
        self.completeness.missing = missing;
    }
}

/// 任务状态。
pub mod task_status {
    pub const PARSING: &str = "parsing";
    pub const READY: &str = "ready";
    pub const FAILED: &str = "failed";
    pub const IMPORTED: &str = "imported";
}

/// 复刻任务（列表/详情出参）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneTaskVO {
    pub id: i64,
    pub platform: String,
    pub channel: String,
    pub source_url: String,
    pub source_item_id: String,
    pub status: String,
    pub title: String,
    pub draft: Option<CloneDraft>,
    pub attempts: Value,
    pub cost_fen: i32,
    pub product_id: Option<i32>,
    pub error_msg: String,
    pub created_by: String,
    pub create_time: i64,
    pub update_time: i64,
}

/// 任务列表查询。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneTaskQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub status: Option<String>,
    pub platform: Option<String>,
}

/// 发起解析入参。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneFetchDTO {
    pub urls: Vec<String>,
    /// 通道：api / selfhost；不传走全局默认（含分平台覆盖）。
    pub channel: Option<String>,
}

/// 换通道重试入参。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneRetryDTO {
    pub task_id: i64,
    pub channel: String,
}

/// 价格规则（导入时应用）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ClonePriceRule {
    /// keep 直用 / coefficient 系数 / add 加减价（元，可负）。
    pub mode: String,
    pub value: Option<String>,
}

/// 确认导入入参（draft 为预览页修正后的完整草稿）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneImportDTO {
    pub task_id: i64,
    pub draft: CloneDraft,
    pub category_id: i32,
    pub brand_id: Option<i32>,
    pub price_rule: Option<ClonePriceRule>,
    /// 落地状态：0 下架（默认，即草稿态）/ 1 上架。
    pub target_status: Option<i32>,
    /// 图片转存（默认 true）。
    pub transfer_images: Option<bool>,
    /// 已存在同 SN 商品时仍新建（SN 加后缀）。
    pub force: Option<bool>,
}

/// 导入结果。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloneImportResult {
    pub product_id: i32,
    pub product_sn: String,
    /// 转存失败而保留源站地址的图片数（>0 时前端提示检查）。
    pub image_transfer_failed: i32,
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_taobao_and_tmall() {
        let (p, id) = parse_source_url("https://item.taobao.com/item.htm?id=123456789012&spm=a1z10").unwrap();
        assert_eq!(p, Platform::Taobao);
        assert_eq!(id, "123456789012");
        let (p, id) = parse_source_url("https://detail.tmall.com/item.htm?id=987654").unwrap();
        assert_eq!(p, Platform::Tmall);
        assert_eq!(id, "987654");
    }

    #[test]
    fn parse_jd_pdd_1688_amazon() {
        assert_eq!(
            parse_source_url("https://item.jd.com/100012043978.html"),
            Some((Platform::Jd, "100012043978".to_string()))
        );
        assert_eq!(
            parse_source_url("https://mobile.yangkeduo.com/goods.html?goods_id=234567"),
            Some((Platform::Pdd, "234567".to_string()))
        );
        assert_eq!(
            parse_source_url("https://detail.1688.com/offer/7654321.html"),
            Some((Platform::Ali1688, "7654321".to_string()))
        );
        assert_eq!(
            parse_source_url("https://www.amazon.com/dp/B0CXYZ1234/ref=sr_1_1"),
            Some((Platform::Amazon, "B0CXYZ1234".to_string()))
        );
    }

    #[test]
    fn parse_unknown_returns_none() {
        assert!(parse_source_url("https://example.com/item?id=1").is_none());
        assert!(parse_source_url("").is_none());
    }
}
