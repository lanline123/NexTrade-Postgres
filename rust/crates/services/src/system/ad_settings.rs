//! 广告投流配置（平台「投流设置」）。
//!
//! 存储：`config` 表单行 `biz_code = "adSettings"`，`biz_val` 为整个配置的 JSON
//! （与 `priceDisplay` 同属「单 code 整块 JSON」型配置，链路照抄）。
//! 未配置时返回内置默认。

use serde_json::{json, Value};

/// config 表中的业务编码。
pub const AD_SETTINGS_CODE: &str = "adSettings";

/// 内置默认配置（未保存过设置时生效）。
#[must_use]
pub fn default_ad_settings() -> Value {
    json!({
        // 各投放位单价（元/天）
        "prices": {
            "shop_street": 30,
            "product_search": 20,
            "product_list": 15,
            "recommend_feed": 20
        },
        // 最短购买天数
        "minDays": 3,
        // 平台置顶位数量上限
        "pinLimit": 3,
        // 店铺街每页推广位上限
        "promotedPerPage": 2,
        // 店铺街智能推荐（千人千面）总开关
        "recommendEnabled": true,
        // 商家投放是否需要平台审核（默认开）
        "needAudit": true
    })
}

fn num(obj: &serde_json::Map<String, Value>, key: &str, default: f64, min: f64, max: f64) -> f64 {
    obj.get(key)
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite())
        .map_or(default, |v| v.clamp(min, max))
}

fn num_value(v: f64) -> Value {
    if v.fract() == 0.0 && v.abs() < 9_007_199_254_740_992.0 {
        json!(v as i64)
    } else {
        json!(v)
    }
}

/// 校验/清洗一份投流配置（保存与读取共用）。
///
/// - 单价钳制 0..=100000（元/天），只保留合法投放位；
/// - `minDays` 钳制 1..=30；`pinLimit` 0..=10；`promotedPerPage` 0..=10；
/// - 输入不是对象时返回默认配置。
#[must_use]
pub fn sanitize_ad_settings(input: &Value) -> Value {
    let Some(obj) = input.as_object() else {
        return default_ad_settings();
    };
    let defaults = default_ad_settings();
    let mut prices = serde_json::Map::new();
    let in_prices = obj.get("prices").and_then(Value::as_object);
    for pos in nextrade_core::models::merchant::ad_campaign::POSITIONS {
        let default = defaults["prices"][pos].as_f64().unwrap_or(0.0);
        let v = in_prices
            .and_then(|m| m.get(pos))
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
            .map_or(default, |v| v.clamp(0.0, 100_000.0));
        prices.insert(pos.to_string(), num_value(v));
    }
    json!({
        "prices": Value::Object(prices),
        "minDays": num(obj, "minDays", 3.0, 1.0, 30.0) as i64,
        "pinLimit": num(obj, "pinLimit", 3.0, 0.0, 10.0) as i64,
        "promotedPerPage": num(obj, "promotedPerPage", 2.0, 0.0, 10.0) as i64,
        "recommendEnabled": obj.get("recommendEnabled").and_then(Value::as_bool).unwrap_or(true),
        "needAudit": obj.get("needAudit").and_then(Value::as_bool).unwrap_or(true),
    })
}

/// 由存储的原始 JSON 文本得到生效配置：解析失败/非对象 ⇒ 默认配置。
#[must_use]
pub fn effective_ad_settings(stored: Option<&str>) -> Value {
    let Some(raw) = stored else {
        return default_ad_settings();
    };
    match serde_json::from_str::<Value>(raw) {
        Ok(v) if v.is_object() => sanitize_ad_settings(&v),
        _ => default_ad_settings(),
    }
}

/// 某投放位的单价（元/天，Decimal 精确计价用）。
#[must_use]
pub fn unit_price(settings: &Value, position: &str) -> rust_decimal::Decimal {
    use std::str::FromStr;
    let v = settings["prices"][position].as_f64().unwrap_or(0.0);
    rust_decimal::Decimal::from_str(&v.to_string()).unwrap_or(rust_decimal::Decimal::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_have_all_positions() {
        let d = default_ad_settings();
        for pos in nextrade_core::models::merchant::ad_campaign::POSITIONS {
            assert!(d["prices"][pos].is_number(), "缺 {pos} 单价");
        }
        assert_eq!(d["needAudit"], json!(true));
        assert_eq!(d["recommendEnabled"], json!(true));
    }

    #[test]
    fn effective_falls_back_to_defaults() {
        assert_eq!(effective_ad_settings(None), default_ad_settings());
        assert_eq!(effective_ad_settings(Some("junk")), default_ad_settings());
        assert_eq!(effective_ad_settings(Some("[1]")), default_ad_settings());
    }

    #[test]
    fn sanitize_clamps_values() {
        let input = json!({
            "prices": { "shop_street": -5, "product_search": 99999999, "bogus_pos": 1 },
            "minDays": 0,
            "pinLimit": 99,
            "promotedPerPage": 3,
            "needAudit": false
        });
        let out = sanitize_ad_settings(&input);
        assert_eq!(out["prices"]["shop_street"], json!(0));
        assert_eq!(out["prices"]["product_search"], json!(100000));
        assert!(out["prices"].get("bogus_pos").is_none());
        assert_eq!(out["minDays"], json!(1));
        assert_eq!(out["pinLimit"], json!(10));
        assert_eq!(out["promotedPerPage"], json!(3));
        assert_eq!(out["needAudit"], json!(false));
    }

    #[test]
    fn unit_price_reads_snapshot() {
        let s = sanitize_ad_settings(&json!({ "prices": { "shop_street": 12.5 } }));
        assert_eq!(unit_price(&s, "shop_street").to_string(), "12.5");
        assert_eq!(unit_price(&s, "recommend_feed").to_string(), "20");
    }
}
