//! 全站购买按钮样式配置（后台「购买按钮样式设置」）。
//!
//! PC 装修各商品类模块（商品 / 商品分组 / 拼团 / 秒杀 / 搜索结果等）的购买
//! 按钮已统一为数字枚举 `buyBtnStyle`（见 pc-semi `BuyButton` 共享组件）：
//! 0=不显示、1=购物车描边、2=购物车实心、3=加号描边、4=加号实心、
//! 5="购买"描边、6="购买"圆角实心、7="马上抢"描边、8="马上抢"圆角实心、
//! 9=圆圈购物车、10=自定义图片。模块值 `-1`（或缺省）表示「跟随全局」，
//! 渲染时取本配置的 `style`；样式 10 的图标优先用模块自带图标，缺省用
//! 本配置的 `customIcon`。
//!
//! 存储：`config` 表单行 `biz_code = "buyStyle"`，`biz_val` 为整个配置的
//! JSON。与 `priceDisplay` 同属「单 code 整块 JSON」型配置。未配置时返回
//! 内置默认（style=1，与「商品」模块出厂默认一致）。

use serde_json::{json, Map, Value};

/// config 表中的业务编码。
pub const BUY_STYLE_CODE: &str = "buyStyle";

/// 统一枚举的合法范围（0=隐藏 … 10=自定义图片）。
pub const BUY_STYLE_MIN: i64 = 0;
pub const BUY_STYLE_MAX: i64 = 10;
/// 内置默认样式（购物车描边，与 pc-product 模块默认 buyBtnStyle=1 一致）。
pub const BUY_STYLE_DEFAULT: i64 = 1;

/// 内置默认配置（未保存过设置时生效；与 PC 前台 DEFAULT_BUY_STYLE 一致）。
#[must_use]
pub fn default_buy_style() -> Value {
    json!({
        "style": BUY_STYLE_DEFAULT,
        "customIcon": { "picUrl": "", "picThumb": "" },
        "iconColor": "",
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

/// 校验/清洗一份购买按钮样式配置（保存与读取共用）。
///
/// - `style` 必须是 0..=10 的整数，否则回落默认 1；
/// - `customIcon` 只保留 `picUrl` / `picThumb` 两个字符串（各截断 1024 字符）；
/// - `iconColor` 截断 32 字符，空串表示跟随模块/主题色。
/// - 输入不是对象时返回默认配置。
#[must_use]
pub fn sanitize_buy_style(input: &Value) -> Value {
    let Some(obj) = input.as_object() else {
        return default_buy_style();
    };
    let style = obj
        .get("style")
        .and_then(Value::as_f64)
        .filter(|s| s.fract() == 0.0 && *s >= BUY_STYLE_MIN as f64 && *s <= BUY_STYLE_MAX as f64)
        .map_or(BUY_STYLE_DEFAULT, |s| s as i64);
    let icon = obj.get("customIcon").and_then(Value::as_object);
    let mut custom_icon = Map::new();
    custom_icon.insert(
        "picUrl".to_string(),
        Value::String(clean_text(icon.and_then(|m| m.get("picUrl")), 1024)),
    );
    custom_icon.insert(
        "picThumb".to_string(),
        Value::String(clean_text(icon.and_then(|m| m.get("picThumb")), 1024)),
    );
    json!({
        "style": style,
        "customIcon": Value::Object(custom_icon),
        "iconColor": clean_text(obj.get("iconColor"), 32),
    })
}

/// 由存储的原始 JSON 文本得到生效配置：解析失败/非对象 ⇒ 默认配置。
#[must_use]
pub fn effective_buy_style(stored: Option<&str>) -> Value {
    let Some(raw) = stored else {
        return default_buy_style();
    };
    match serde_json::from_str::<Value>(raw) {
        Ok(v) if v.is_object() => sanitize_buy_style(&v),
        _ => default_buy_style(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_cart_outline() {
        let d = default_buy_style();
        assert_eq!(d["style"], json!(1));
        assert_eq!(d["customIcon"]["picUrl"], json!(""));
        assert_eq!(d["iconColor"], json!(""));
    }

    #[test]
    fn effective_falls_back_to_defaults() {
        assert_eq!(effective_buy_style(None), default_buy_style());
        assert_eq!(effective_buy_style(Some("not json")), default_buy_style());
        assert_eq!(effective_buy_style(Some("[1,2]")), default_buy_style());
    }

    #[test]
    fn sanitize_clamps_style_and_cleans_icon() {
        // 越界 / 非整数回落默认
        assert_eq!(sanitize_buy_style(&json!({ "style": 11 }))["style"], json!(1));
        assert_eq!(sanitize_buy_style(&json!({ "style": -1 }))["style"], json!(1));
        assert_eq!(sanitize_buy_style(&json!({ "style": 2.5 }))["style"], json!(1));
        // 合法值原样保留（含 0=隐藏 与 10=自定义图片）
        assert_eq!(sanitize_buy_style(&json!({ "style": 0 }))["style"], json!(0));
        assert_eq!(sanitize_buy_style(&json!({ "style": 9 }))["style"], json!(9));
        let ok = sanitize_buy_style(&json!({
            "style": 10,
            "customIcon": { "picUrl": " https://x/i.svg ", "picThumb": "https://x/i_t.svg", "evil": 1 },
            "iconColor": "#e93b3d"
        }));
        assert_eq!(ok["customIcon"]["picUrl"], json!("https://x/i.svg"));
        assert_eq!(ok["customIcon"]["picThumb"], json!("https://x/i_t.svg"));
        assert!(ok["customIcon"].get("evil").is_none());
        assert_eq!(ok["iconColor"], json!("#e93b3d"));
    }
}
