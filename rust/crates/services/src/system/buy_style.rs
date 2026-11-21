//! 全站购买按钮样式配置（后台「购买按钮样式」，2026-11-21 由
//! 「购买按钮样式设置」改名）。
//!
//! PC 装修各商品类模块（商品 / 商品分组 / 拼团 / 秒杀 / 搜索结果等）的购买
//! 按钮已统一为数字枚举 `buyBtnStyle`（见 pc-semi `BuyButton` 共享组件）：
//! 0=不显示、1=购物车描边、2=购物车实心、3=加号描边、4=加号实心、
//! 5="购买"描边、6="购买"圆角实心、7="马上抢"描边、8="马上抢"圆角实心、
//! 9=圆圈购物车、10=自定义图片；2026-11-21 扩充 11=购物袋描边、12=购物袋
//! 实心、13=圆圈购物袋、14=圆圈加号、15=加购购物车、16=闪电、
//! 17="立即购买"描边、18="立即购买"圆角实心（11–16 图标源自 IconPark
//! 电商类目，Apache-2.0）。模块值 `-1`（或缺省）表示「跟随全局」，
//! 渲染时取本配置的 `style`；样式 10 的图标优先用模块自带图标，缺省用
//! 本配置的 `customIcon`。
//!
//! `hiddenStyles`：设置页「样式管理」隐藏掉的样式 id 列表。仅约束各编辑器
//! 选择器的可选项（隐藏项不再出现），渲染端一律不消费此字段——已被页面 /
//! 全局配置选用的样式即使被隐藏，存量渲染照常。清洗时强制剔除当前全局
//! `style` 本身（正在全局使用的样式不可隐藏，防呆兜底）。
//!
//! 存储：`config` 表单行 `biz_code = "buyStyle"`，`biz_val` 为整个配置的
//! JSON。与 `priceDisplay` 同属「单 code 整块 JSON」型配置。未配置时返回
//! 内置默认（style=1，与「商品」模块出厂默认一致）。

use serde_json::{json, Map, Value};

/// config 表中的业务编码。
pub const BUY_STYLE_CODE: &str = "buyStyle";

/// 统一枚举的合法范围（0=隐藏 … 18="立即购买"实心）。
pub const BUY_STYLE_MIN: i64 = 0;
pub const BUY_STYLE_MAX: i64 = 18;
/// 内置默认样式（购物车描边，与 pc-product 模块默认 buyBtnStyle=1 一致）。
pub const BUY_STYLE_DEFAULT: i64 = 1;

/// 内置默认配置（未保存过设置时生效；与 PC 前台 DEFAULT_BUY_STYLE 一致）。
#[must_use]
pub fn default_buy_style() -> Value {
    json!({
        "style": BUY_STYLE_DEFAULT,
        "customIcon": { "picUrl": "", "picThumb": "" },
        "iconColor": "",
        "hiddenStyles": [],
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

/// 清洗隐藏样式列表：只保留 0..=MAX 的整数、去重升序，并强制剔除当前
/// 全局样式（正在全局使用的样式不可隐藏）。
fn clean_hidden_styles(input: Option<&Value>, current_style: i64) -> Vec<i64> {
    let mut out: Vec<i64> = input
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_f64)
                .filter(|s| s.fract() == 0.0 && *s >= BUY_STYLE_MIN as f64 && *s <= BUY_STYLE_MAX as f64)
                .map(|s| s as i64)
                .filter(|s| *s != current_style)
                .collect()
        })
        .unwrap_or_default();
    out.sort_unstable();
    out.dedup();
    out
}

/// 校验/清洗一份购买按钮样式配置（保存与读取共用）。
///
/// - `style` 必须是 0..=18 的整数，否则回落默认 1；
/// - `customIcon` 只保留 `picUrl` / `picThumb` 两个字符串（各截断 1024 字符）；
/// - `iconColor` 截断 32 字符，空串表示跟随模块/主题色；
/// - `hiddenStyles` 按 [`clean_hidden_styles`] 清洗（仅约束选择器，渲染不消费）。
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
        "hiddenStyles": clean_hidden_styles(obj.get("hiddenStyles"), style),
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
        assert_eq!(d["hiddenStyles"], json!([]));
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
        assert_eq!(sanitize_buy_style(&json!({ "style": 19 }))["style"], json!(1));
        assert_eq!(sanitize_buy_style(&json!({ "style": -1 }))["style"], json!(1));
        assert_eq!(sanitize_buy_style(&json!({ "style": 2.5 }))["style"], json!(1));
        // 合法值原样保留（含 0=隐藏、10=自定义图片 与扩充段 11–18）
        assert_eq!(sanitize_buy_style(&json!({ "style": 0 }))["style"], json!(0));
        assert_eq!(sanitize_buy_style(&json!({ "style": 9 }))["style"], json!(9));
        assert_eq!(sanitize_buy_style(&json!({ "style": 11 }))["style"], json!(11));
        assert_eq!(sanitize_buy_style(&json!({ "style": 18 }))["style"], json!(18));
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

    #[test]
    fn sanitize_cleans_hidden_styles() {
        // 去重升序、剔除越界/非整数，并强制剔除当前全局样式本身
        let ok = sanitize_buy_style(&json!({
            "style": 3,
            "hiddenStyles": [12, 5, 12, 19, -1, 2.5, 3, 0]
        }));
        assert_eq!(ok["hiddenStyles"], json!([0, 5, 12]));
        // 非数组输入回落空列表
        assert_eq!(
            sanitize_buy_style(&json!({ "style": 1, "hiddenStyles": "x" }))["hiddenStyles"],
            json!([])
        );
        // 缺省（存量配置无此字段）回落空列表，存量渲染零影响
        assert_eq!(sanitize_buy_style(&json!({ "style": 2 }))["hiddenStyles"], json!([]));
    }
}
