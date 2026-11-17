//! 全站字体配置（后台「网站字体设置」，设置 → 商城设置）。
//!
//! 前台可按语言（PC locale code：zh-CN / en-US / fr / ko / ja …）分别指定
//! 字体，后台（admin）单独一个字体选择。全站默认 = 阿里巴巴普惠体 3.0
//! （自托管切片，见 pc-semi / admin-semi 的字体注册表）。
//!
//! 存储：`config` 表单行 `biz_code = "fontSettings"`，`biz_val` 为整个
//! 配置的 JSON。与 `buyStyle` / `priceDisplay` 同属「单 code 整块 JSON」
//! 型配置。配置里只存字体引用 `fontId`；字体注册表（fontId → family /
//! CSS 地址）是前后端代码里的常量表，本模块的注册表只做保存/读取时的
//! 白名单校验。另留 `customFonts` 逃生舱：管理员自备 CSS 地址的字体，
//! 其 id 同样可被各语言槽位引用。
//!
//! 未配置时返回内置默认：default 与 zh 语言 = 普惠体，拉丁语 = Inter，
//! 韩 = Noto Sans KR，日 = Noto Sans JP，俄 = Roboto，admin = 普惠体。

use serde_json::{json, Map, Value};

/// config 表中的业务编码。
pub const FONT_SETTINGS_CODE: &str = "fontSettings";

/// 预置字体注册表（仅 id 白名单；family / CSS 地址在前端注册表中）。
pub const PRESET_FONT_IDS: [&str; 8] = [
    "alibaba-puhuiti",
    "noto-sans-sc",
    "noto-sans-tc",
    "noto-sans-jp",
    "noto-sans-kr",
    "inter",
    "roboto",
    "system",
];

/// 全站默认字体（用户指定：阿里巴巴普惠体）。
pub const DEFAULT_FONT_ID: &str = "alibaba-puhuiti";

const MAX_LOCALES: usize = 32;
const MAX_CUSTOM_FONTS: usize = 8;

fn is_preset(font_id: &str) -> bool {
    PRESET_FONT_IDS.contains(&font_id)
}

/// 某语言槽位的出厂推荐字体（与设置页的「推荐」标注一致）。
#[must_use]
pub fn recommended_font_id(locale: &str) -> &'static str {
    match locale {
        "ko" => "noto-sans-kr",
        "ja" => "noto-sans-jp",
        "ru" => "roboto",
        "en-US" | "fr" | "de" | "es" | "it" | "pt" => "inter",
        _ => DEFAULT_FONT_ID,
    }
}

/// 内置默认配置（未保存过设置时生效）。
#[must_use]
pub fn default_font_settings() -> Value {
    json!({
        "version": 1,
        "storefront": {
            "default": { "fontId": DEFAULT_FONT_ID },
            "locales": {
                "zh-CN": { "fontId": "alibaba-puhuiti" },
                "zh-Hant": { "fontId": "alibaba-puhuiti" },
                "en-US": { "fontId": "inter" },
                "fr": { "fontId": "inter" },
                "ko": { "fontId": "noto-sans-kr" },
                "ja": { "fontId": "noto-sans-jp" },
                "ru": { "fontId": "roboto" },
            },
        },
        "admin": { "fontId": DEFAULT_FONT_ID },
        "customFonts": [],
    })
}

fn clean_text(v: Option<&Value>, max_chars: usize) -> String {
    v.and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .chars()
        .take(max_chars)
        .collect()
}

/// locale code 形态校验：`^[a-z]{2}(-[A-Za-z]{2,4})?$`（如 zh-CN、zh-Hant、fr）。
fn valid_locale_key(key: &str) -> bool {
    let mut parts = key.split('-');
    let lang = parts.next().unwrap_or("");
    if lang.len() != 2 || !lang.chars().all(|c| c.is_ascii_lowercase()) {
        return false;
    }
    match parts.next() {
        None => true,
        Some(region) => {
            (2..=4).contains(&region.len())
                && region.chars().all(|c| c.is_ascii_alphabetic())
                && parts.next().is_none()
        }
    }
}

/// customFonts 的 id 形态：`[a-z0-9-]{1,32}`。
fn valid_custom_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 32
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// 含 CSS 注入风险字符（`;{}<>`）的文本不可用作 family / cssUrl。
fn has_css_injection(text: &str) -> bool {
    text.chars().any(|c| matches!(c, ';' | '{' | '}' | '<' | '>'))
}

/// 清洗 customFonts 数组：逐条校验 id / name / family / cssUrl，非法条目
/// 整条丢弃；最多保留 8 条，id 重复时只留第一条。
fn sanitize_custom_fonts(input: Option<&Value>) -> Vec<Value> {
    let Some(list) = input.and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut out: Vec<Value> = Vec::new();
    for item in list {
        if out.len() >= MAX_CUSTOM_FONTS {
            break;
        }
        let Some(obj) = item.as_object() else { continue };
        let id = clean_text(obj.get("id"), 32);
        if !valid_custom_id(&id) || is_preset(&id) {
            continue;
        }
        if out.iter().any(|v| v["id"] == json!(id)) {
            continue;
        }
        let name = clean_text(obj.get("name"), 64);
        let family = clean_text(obj.get("family"), 64);
        let css_url = clean_text(obj.get("cssUrl"), 1024);
        if family.is_empty()
            || has_css_injection(&family)
            || !css_url.starts_with("https://")
            || has_css_injection(&css_url)
        {
            continue;
        }
        out.push(json!({
            "id": id,
            "name": if name.is_empty() { family.clone() } else { name },
            "family": family,
            "cssUrl": css_url,
        }));
    }
    out
}

/// 清洗一个字体槽位（`{ "fontId": … }`）：fontId 必须命中预置注册表或
/// customFonts 的 id，否则回落 `fallback`。
fn sanitize_slot(input: Option<&Value>, custom_ids: &[String], fallback: &str) -> Value {
    let font_id = input
        .and_then(Value::as_object)
        .and_then(|o| o.get("fontId"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if is_preset(font_id) || custom_ids.iter().any(|id| id == font_id) {
        json!({ "fontId": font_id })
    } else {
        json!({ "fontId": fallback })
    }
}

/// 校验/清洗一份字体配置（保存与读取共用）。输入不是对象时返回默认。
#[must_use]
pub fn sanitize_font_settings(input: &Value) -> Value {
    let Some(obj) = input.as_object() else {
        return default_font_settings();
    };
    let defaults = default_font_settings();

    let custom_fonts = sanitize_custom_fonts(obj.get("customFonts"));
    let custom_ids: Vec<String> = custom_fonts
        .iter()
        .filter_map(|v| v["id"].as_str().map(str::to_string))
        .collect();

    // storefront.default
    let sf = obj.get("storefront").and_then(Value::as_object);
    let default_slot = sanitize_slot(
        sf.and_then(|m| m.get("default")),
        &custom_ids,
        DEFAULT_FONT_ID,
    );

    // storefront.locales：以出厂推荐为底，逐条并入合法的用户配置
    let mut locales = defaults["storefront"]["locales"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    if let Some(user_locales) = sf.and_then(|m| m.get("locales")).and_then(Value::as_object) {
        for (key, slot) in user_locales {
            if locales.len() >= MAX_LOCALES {
                break;
            }
            if !valid_locale_key(key) {
                continue;
            }
            let fallback = recommended_font_id(key);
            locales.insert(
                key.clone(),
                sanitize_slot(Some(slot), &custom_ids, fallback),
            );
        }
    }

    let admin_slot = sanitize_slot(obj.get("admin"), &custom_ids, DEFAULT_FONT_ID);

    let mut storefront = Map::new();
    storefront.insert("default".to_string(), default_slot);
    storefront.insert("locales".to_string(), Value::Object(locales));
    json!({
        "version": 1,
        "storefront": Value::Object(storefront),
        "admin": admin_slot,
        "customFonts": custom_fonts,
    })
}

/// 由存储的原始 JSON 文本得到生效配置：解析失败/非对象 ⇒ 默认配置。
#[must_use]
pub fn effective_font_settings(stored: Option<&str>) -> Value {
    let Some(raw) = stored else {
        return default_font_settings();
    };
    match serde_json::from_str::<Value>(raw) {
        Ok(v) if v.is_object() => sanitize_font_settings(&v),
        _ => default_font_settings(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_puhuiti_with_locale_presets() {
        let d = default_font_settings();
        assert_eq!(d["storefront"]["default"]["fontId"], json!("alibaba-puhuiti"));
        assert_eq!(d["storefront"]["locales"]["zh-CN"]["fontId"], json!("alibaba-puhuiti"));
        assert_eq!(d["storefront"]["locales"]["en-US"]["fontId"], json!("inter"));
        assert_eq!(d["storefront"]["locales"]["ko"]["fontId"], json!("noto-sans-kr"));
        assert_eq!(d["admin"]["fontId"], json!("alibaba-puhuiti"));
        assert_eq!(d["customFonts"], json!([]));
    }

    #[test]
    fn effective_falls_back_to_defaults() {
        assert_eq!(effective_font_settings(None), default_font_settings());
        assert_eq!(effective_font_settings(Some("not json")), default_font_settings());
        assert_eq!(effective_font_settings(Some("[1,2]")), default_font_settings());
    }

    #[test]
    fn sanitize_rejects_unknown_font_and_locale() {
        let out = sanitize_font_settings(&json!({
            "storefront": {
                "default": { "fontId": "evil-font" },
                "locales": {
                    "fr": { "fontId": "roboto" },
                    "BAD KEY!": { "fontId": "inter" },
                    "de": { "fontId": "nope" }
                }
            },
            "admin": { "fontId": "noto-sans-sc" }
        }));
        // 未知 fontId 回落全站默认
        assert_eq!(out["storefront"]["default"]["fontId"], json!("alibaba-puhuiti"));
        // 合法 locale + 合法 fontId 保留
        assert_eq!(out["storefront"]["locales"]["fr"]["fontId"], json!("roboto"));
        // 非法 locale key 丢弃
        assert!(out["storefront"]["locales"].get("BAD KEY!").is_none());
        // 合法 locale + 未知 fontId 回落该语言推荐（de → inter）
        assert_eq!(out["storefront"]["locales"]["de"]["fontId"], json!("inter"));
        assert_eq!(out["admin"]["fontId"], json!("noto-sans-sc"));
    }

    #[test]
    fn sanitize_custom_fonts_escape_hatch() {
        let out = sanitize_font_settings(&json!({
            "storefront": { "default": { "fontId": "myfont1" } },
            "customFonts": [
                { "id": "myfont1", "name": "自备字体", "family": "My Font", "cssUrl": "https://cdn.example.com/f.css" },
                { "id": "bad id", "family": "X", "cssUrl": "https://x/f.css" },
                { "id": "inj", "family": "A; } body {", "cssUrl": "https://x/f.css" },
                { "id": "http1", "family": "Y", "cssUrl": "http://x/f.css" },
                { "id": "inter", "family": "Z", "cssUrl": "https://x/f.css" }
            ]
        }));
        // 逃生舱 id 可被槽位引用
        assert_eq!(out["storefront"]["default"]["fontId"], json!("myfont1"));
        let customs = out["customFonts"].as_array().unwrap();
        assert_eq!(customs.len(), 1);
        assert_eq!(customs[0]["id"], json!("myfont1"));
    }
}
