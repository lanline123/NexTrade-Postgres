//! 全站价格显示配置（后台「价格显示设置」）。
//!
//! 大额商品展示价按币种单位阶梯缩写：人民币 万/亿、美元 K/M/B、
//! 印度卢比 K/L/Cr、俄卢布 тыс./млн 等。每条规则 = 门槛 `at`（换算后
//! 金额 ≥ at 时启用）+ 除数 `div`（显示值 = 金额 / div）+ 自由后缀
//! `suffix`（万 / 百万 / K / M …）。
//!
//! 存储：`config` 表单行 `biz_code = "priceDisplay"`，`biz_val` 为
//! 整个配置的 JSON。与 `themeStyle` 同属「单 code 整块 JSON」型配置。
//! 未配置时返回内置默认（与 PC 前台、admin 设置页的默认表一致）。
//!
//! 仅影响商品展示价的格式化（前台 formatDisplayPrice 消费）；购物车 /
//! 结算 / 订单等对账金额不读本配置，始终精确显示。

use serde_json::{json, Map, Value};

/// config 表中的业务编码。
pub const PRICE_DISPLAY_CODE: &str = "priceDisplay";

fn rule(at: i64, div: i64, suffix: &str) -> Value {
    json!({ "at": at, "div": div, "suffix": suffix })
}

fn western() -> Value {
    json!([
        rule(10_000, 1_000, "K"),
        rule(1_000_000, 1_000_000, "M"),
        rule(1_000_000_000, 1_000_000_000, "B"),
    ])
}

/// 内置默认配置（未保存过设置时生效；与 PC 前台 DEFAULT_PRICE_DISPLAY 一致）。
#[must_use]
pub fn default_price_display() -> Value {
    let mut currencies = Map::new();
    let mut put = |code: &str, ladder: Value| {
        currencies.insert(code.to_string(), ladder);
    };
    put(
        "CNY",
        json!([rule(10_000, 10_000, "万"), rule(100_000_000, 100_000_000, "亿")]),
    );
    put(
        "HKD",
        json!([rule(10_000, 10_000, "万"), rule(100_000_000, 100_000_000, "億")]),
    );
    put(
        "TWD",
        json!([rule(10_000, 10_000, "萬"), rule(100_000_000, 100_000_000, "億")]),
    );
    put(
        "JPY",
        json!([rule(10_000, 10_000, "万"), rule(100_000_000, 100_000_000, "億")]),
    );
    put(
        "KRW",
        json!([rule(10_000, 10_000, "만"), rule(100_000_000, 100_000_000, "억")]),
    );
    put(
        "INR",
        json!([
            rule(10_000, 1_000, "K"),
            rule(100_000, 100_000, "L"),
            rule(10_000_000, 10_000_000, "Cr"),
        ]),
    );
    put(
        "RUB",
        json!([
            rule(10_000, 1_000, " тыс."),
            rule(1_000_000, 1_000_000, " млн"),
            rule(1_000_000_000, 1_000_000_000, " млрд"),
        ]),
    );
    put(
        "IDR",
        json!([
            rule(10_000, 1_000, "K"),
            rule(1_000_000, 1_000_000, "Jt"),
            rule(1_000_000_000, 1_000_000_000, "M"),
        ]),
    );
    put(
        "VND",
        json!([
            rule(100_000, 1_000, "K"),
            rule(1_000_000, 1_000_000, "Tr"),
            rule(1_000_000_000, 1_000_000_000, "Tỷ"),
        ]),
    );
    for code in [
        "USD", "EUR", "GBP", "AUD", "CAD", "SGD", "CHF", "NZD", "MYR", "PHP", "PLN", "SEK", "NOK",
        "DKK", "ZAR", "MXN", "SAR", "AED", "QAR", "KWD", "BRL", "THB", "TRY", "ILS", "CZK", "HUF",
    ] {
        put(code, western());
    }
    json!({
        "enabled": true,
        "decimals": 2,
        "trimZeros": true,
        "currencies": Value::Object(currencies),
    })
}

fn num_value(v: f64) -> Value {
    if v.fract() == 0.0 && v.abs() < 9_007_199_254_740_992.0 {
        json!(v as i64)
    } else {
        json!(v)
    }
}

/// 校验/清洗一份价格显示配置（保存与读取共用，保证落库与下发都是合法结构）。
///
/// - `enabled` 缺省 true；`decimals` 钳制 0..=4（缺省 2）；`trimZeros` 缺省 true。
/// - 币种代码大写、截断 8 字符，最多 64 个币种。
/// - 规则只保留 at>0 且 div>0 的条目，后缀截断 8 字符，按 at 升序，最多 6 条/币种。
/// - 输入不是对象时返回默认配置。
#[must_use]
pub fn sanitize_price_display(input: &Value) -> Value {
    let Some(obj) = input.as_object() else {
        return default_price_display();
    };
    let enabled = obj.get("enabled").and_then(Value::as_bool).unwrap_or(true);
    let decimals = obj
        .get("decimals")
        .and_then(Value::as_f64)
        .map_or(2, |d| d.floor().clamp(0.0, 4.0) as i64);
    let trim_zeros = obj.get("trimZeros").and_then(Value::as_bool).unwrap_or(true);
    let mut currencies = Map::new();
    if let Some(map) = obj.get("currencies").and_then(Value::as_object) {
        for (code, rules) in map {
            if currencies.len() >= 64 {
                break;
            }
            let Some(list) = rules.as_array() else {
                continue;
            };
            let mut clean: Vec<(f64, Value)> = Vec::new();
            for r in list {
                let Some(r) = r.as_object() else { continue };
                let at = r.get("at").and_then(Value::as_f64).unwrap_or(0.0);
                let div = r.get("div").and_then(Value::as_f64).unwrap_or(0.0);
                if !(at > 0.0 && div > 0.0 && at.is_finite() && div.is_finite()) {
                    continue;
                }
                let suffix: String = r
                    .get("suffix")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .chars()
                    .take(8)
                    .collect();
                clean.push((
                    at,
                    json!({ "at": num_value(at), "div": num_value(div), "suffix": suffix }),
                ));
            }
            clean.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            clean.truncate(6);
            let key: String = code.to_uppercase().chars().take(8).collect();
            currencies.insert(key, Value::Array(clean.into_iter().map(|(_, v)| v).collect()));
        }
    }
    json!({
        "enabled": enabled,
        "decimals": decimals,
        "trimZeros": trim_zeros,
        "currencies": Value::Object(currencies),
    })
}

/// 由存储的原始 JSON 文本得到生效配置：解析失败/非对象 ⇒ 默认配置。
#[must_use]
pub fn effective_price_display(stored: Option<&str>) -> Value {
    let Some(raw) = stored else {
        return default_price_display();
    };
    match serde_json::from_str::<Value>(raw) {
        Ok(v) if v.is_object() => sanitize_price_display(&v),
        _ => default_price_display(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_cover_major_currencies() {
        let d = default_price_display();
        assert_eq!(d["enabled"], json!(true));
        assert_eq!(d["decimals"], json!(2));
        let currencies = d["currencies"].as_object().expect("currencies");
        assert!(currencies.len() >= 30, "主要币种应齐全: {}", currencies.len());
        assert_eq!(currencies["CNY"][0]["suffix"], json!("万"));
        assert_eq!(currencies["CNY"][1]["suffix"], json!("亿"));
        assert_eq!(currencies["USD"][1]["suffix"], json!("M"));
        assert_eq!(currencies["INR"][1]["suffix"], json!("L"));
        assert_eq!(currencies["INR"][2]["suffix"], json!("Cr"));
        assert_eq!(currencies["KRW"][0]["suffix"], json!("만"));
    }

    #[test]
    fn effective_falls_back_to_defaults() {
        assert_eq!(effective_price_display(None), default_price_display());
        assert_eq!(effective_price_display(Some("not json")), default_price_display());
        assert_eq!(effective_price_display(Some("[1,2]")), default_price_display());
    }

    #[test]
    fn sanitize_clamps_and_sorts() {
        let input = json!({
            "enabled": false,
            "decimals": 9,
            "currencies": {
                "cny": [
                    { "at": 1000000, "div": 1000000, "suffix": "百万" },
                    { "at": 10000, "div": 10000, "suffix": "万" },
                    { "at": -5, "div": 10, "suffix": "bad" }
                ]
            }
        });
        let out = sanitize_price_display(&input);
        assert_eq!(out["enabled"], json!(false));
        assert_eq!(out["decimals"], json!(4));
        let cny = out["currencies"]["CNY"].as_array().expect("CNY rules");
        assert_eq!(cny.len(), 2);
        assert_eq!(cny[0]["suffix"], json!("万"));
        assert_eq!(cny[1]["suffix"], json!("百万"));
    }
}
