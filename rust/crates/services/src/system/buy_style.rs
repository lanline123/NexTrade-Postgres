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
//! `objStyle`（2026-11-21 字段补齐，我方新增）：H5 对象型一族（分组滑动 /
//! 商品分组 / 商品轮播 / 商品A1，共用 CommonBuyButton 的对象型
//! `buyBtnStyle`）的全局字段组。字段名与模块侧逐字一致，模块开启
//! `followGlobal` 后渲染层把本组 13 个字段浅合并覆盖模块值（读时解析、
//! 不回写模块数据）；`showBtn` / `wrap` / `wrapAlign` 是模块语境字段，
//! 永不进全局。两条铁律（评估报告 buybtn-global-fields-eval 第五节）：
//! ① 渐变色 `gradientColorA/B` 默认必须是空串（= 跟随主题色，与现行渲染
//!    的黑圈哨兵口径一致），严禁填模块数据出厂值 `#333333`——那会把哨兵
//!    语义带进全局、且设置页色板重现黑圈误导；
//! ② 本组内的 `iconColor` 与顶层 `iconColor` 同名但语义相反（顶层空串 =
//!    跟随主题色；对象型恒为显式色、出厂 `#ffffff`），必须分开存、严禁
//!    合并：本组 `iconColor` 清洗时空回落 `#ffffff`，不继承顶层空串语义。
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

/// 对象型字段组的内置默认（= 现行渲染行为：渐变空串跟随主题色，其余与
/// H5 编辑器 `defaultBuyButtonStyle` 出厂值一致）。
#[must_use]
pub fn default_obj_style() -> Value {
    json!({
        "btnType": 1,
        "iconImage": { "picUrl": "", "picThumb": "" },
        "iconSize": 14,
        "text": "",
        "textColor": "#ffffff",
        "textSize": 12,
        "iconColor": "#ffffff",
        "gradientType": "leftRight",
        "gradientColorA": "",
        "gradientColorB": "",
        "radius": 16,
        "paddingTopBottom": 0,
        "paddingLeftRight": 10,
    })
}

/// 内置默认配置（未保存过设置时生效；与 PC 前台 DEFAULT_BUY_STYLE 一致）。
#[must_use]
pub fn default_buy_style() -> Value {
    json!({
        "style": BUY_STYLE_DEFAULT,
        "customIcon": { "picUrl": "", "picThumb": "" },
        "iconColor": "",
        "hiddenStyles": [],
        "objStyle": default_obj_style(),
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

/// 数值字段清洗：有限数四舍五入后钳到 [min, max]，缺失/非法回落默认。
/// （对象型字段在编辑器里都是整数 Range，取整无损。）
fn clean_num_clamped(v: Option<&Value>, min: i64, max: i64, default: i64) -> i64 {
    v.and_then(Value::as_f64)
        .filter(|n| n.is_finite())
        .map_or(default, |n| (n.round() as i64).clamp(min, max))
}

/// 必填色值清洗：截断 32 字符，空回落给定默认（对象型的文字/图标色恒为
/// 显式色，空串在渲染层无意义；与渐变色的「空 = 跟随主题」语义不同）。
fn clean_required_color(v: Option<&Value>, default: &str) -> String {
    let s = clean_text(v, 32);
    if s.is_empty() {
        default.to_string()
    } else {
        s
    }
}

/// 清洗 `objStyle` 对象型字段组（字段名与 H5 对象型模块逐字一致）：
/// 非对象回落全默认；`btnType` 须为 1..=5 整数否则回落 1；数值字段按
/// 编辑器 Range 值域钳制；`gradientType` 走白名单；`iconColor` 空回落
/// `#ffffff`（铁律②）；`gradientColorA/B` 空串保留（= 跟随主题色，铁律①）。
fn sanitize_obj_style(input: Option<&Value>) -> Value {
    let Some(obj) = input.and_then(Value::as_object) else {
        return default_obj_style();
    };
    let btn_type = obj
        .get("btnType")
        .and_then(Value::as_f64)
        .filter(|n| n.fract() == 0.0 && *n >= 1.0 && *n <= 5.0)
        .map_or(1, |n| n as i64);
    let icon = obj.get("iconImage").and_then(Value::as_object);
    let mut icon_image = Map::new();
    icon_image.insert(
        "picUrl".to_string(),
        Value::String(clean_text(icon.and_then(|m| m.get("picUrl")), 1024)),
    );
    icon_image.insert(
        "picThumb".to_string(),
        Value::String(clean_text(icon.and_then(|m| m.get("picThumb")), 1024)),
    );
    let gradient_type = match obj.get("gradientType").and_then(Value::as_str) {
        Some(g @ ("upDown" | "leftRight" | "diagonal" | "purity")) => g,
        _ => "leftRight",
    };
    json!({
        "btnType": btn_type,
        "iconImage": Value::Object(icon_image),
        "iconSize": clean_num_clamped(obj.get("iconSize"), 12, 40, 14),
        "text": clean_text(obj.get("text"), 32),
        "textColor": clean_required_color(obj.get("textColor"), "#ffffff"),
        "textSize": clean_num_clamped(obj.get("textSize"), 10, 24, 12),
        "iconColor": clean_required_color(obj.get("iconColor"), "#ffffff"),
        "gradientType": gradient_type,
        "gradientColorA": clean_text(obj.get("gradientColorA"), 32),
        "gradientColorB": clean_text(obj.get("gradientColorB"), 32),
        "radius": clean_num_clamped(obj.get("radius"), 0, 20, 16),
        "paddingTopBottom": clean_num_clamped(obj.get("paddingTopBottom"), 0, 10, 0),
        "paddingLeftRight": clean_num_clamped(obj.get("paddingLeftRight"), 0, 10, 10),
    })
}

/// 校验/清洗一份购买按钮样式配置（保存与读取共用）。
///
/// - `style` 必须是 0..=18 的整数，否则回落默认 1；
/// - `customIcon` 只保留 `picUrl` / `picThumb` 两个字符串（各截断 1024 字符）；
/// - `iconColor` 截断 32 字符，空串表示跟随模块/主题色；
/// - `hiddenStyles` 按 [`clean_hidden_styles`] 清洗（仅约束选择器，渲染不消费）；
/// - `objStyle` 按 [`sanitize_obj_style`] 清洗（对象型字段组，缺失回落默认组）。
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
        "objStyle": sanitize_obj_style(obj.get("objStyle")),
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
        // objStyle 默认组：渐变空串 = 跟随主题色（铁律①，严禁 #333333），
        // 对象型 iconColor 恒显式 #ffffff（铁律②）
        assert_eq!(d["objStyle"], default_obj_style());
        assert_eq!(d["objStyle"]["gradientColorA"], json!(""));
        assert_eq!(d["objStyle"]["gradientColorB"], json!(""));
        assert_eq!(d["objStyle"]["iconColor"], json!("#ffffff"));
        assert_eq!(d["objStyle"]["btnType"], json!(1));
    }

    #[test]
    fn sanitize_obj_style_falls_back_and_clamps() {
        // 缺失 / 非对象回落全默认组（存量配置无 objStyle 时自愈）
        assert_eq!(sanitize_buy_style(&json!({ "style": 2 }))["objStyle"], default_obj_style());
        assert_eq!(
            sanitize_buy_style(&json!({ "style": 2, "objStyle": "x" }))["objStyle"],
            default_obj_style()
        );
        // 越界钳制 + 白名单 + 空色回落
        let ok = sanitize_buy_style(&json!({
            "style": 1,
            "objStyle": {
                "btnType": 9,
                "iconSize": 99,
                "textSize": 3,
                "radius": -3,
                "paddingTopBottom": 30,
                "paddingLeftRight": 7,
                "gradientType": "spiral",
                "iconColor": "",
                "textColor": " ",
                "gradientColorA": "#333333",
                "iconImage": { "picUrl": " https://x/b.svg ", "evil": 1 }
            }
        }));
        let o = &ok["objStyle"];
        assert_eq!(o["btnType"], json!(1));
        assert_eq!(o["iconSize"], json!(40));
        assert_eq!(o["textSize"], json!(10));
        assert_eq!(o["radius"], json!(0));
        assert_eq!(o["paddingTopBottom"], json!(10));
        assert_eq!(o["paddingLeftRight"], json!(7));
        assert_eq!(o["gradientType"], json!("leftRight"));
        // 铁律②：对象型 iconColor 空回落 #ffffff，不继承顶层空串语义
        assert_eq!(o["iconColor"], json!("#ffffff"));
        assert_eq!(o["textColor"], json!("#ffffff"));
        // 显式保存的渐变色原样保留（是否跟随主题由渲染层哨兵口径决定）
        assert_eq!(o["gradientColorA"], json!("#333333"));
        assert_eq!(o["iconImage"]["picUrl"], json!("https://x/b.svg"));
        assert!(o["iconImage"].get("evil").is_none());
        // 合法值原样保留
        let ok2 = sanitize_buy_style(&json!({
            "style": 1,
            "objStyle": { "btnType": 5, "gradientType": "purity", "text": "立即抢购", "gradientColorB": "#FF5E96" }
        }));
        assert_eq!(ok2["objStyle"]["btnType"], json!(5));
        assert_eq!(ok2["objStyle"]["gradientType"], json!("purity"));
        assert_eq!(ok2["objStyle"]["text"], json!("立即抢购"));
        assert_eq!(ok2["objStyle"]["gradientColorB"], json!("#FF5E96"));
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
