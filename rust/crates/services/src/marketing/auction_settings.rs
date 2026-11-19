//! 拍卖全局设置（后台「拍卖设置」页可视化编辑，规划第 11 节 7 项拍板结论）。
//!
//! 存储：`config` 表单行 `biz_code = "auctionSettings"`，`biz_val` 为整块
//! JSON，与 `fontSettings` / `buyStyle` 同属「单 code 整块 JSON」型配置。
//! 2026-11-18 用户拍板：7 项策略全部做成设置项、本模块的出厂值即推荐值，
//! 代码里不写死策略数字，运行时一律从本设置读取。
//!
//! 本模块只含纯逻辑（默认/清洗/生效 + 保证金/阶梯/延时/扣罚/封禁的判定
//! 函数），便于单元测试；读写 DB 在 `auction.rs` 服务里经
//! `repos.system.config_basic` 完成。

use rust_decimal::Decimal;
use serde_json::{json, Value};

/// config 表中的业务编码。
pub const AUCTION_SETTINGS_CODE: &str = "auctionSettings";

/// 一档加价阶梯：当前价 < `max_price`（`None` = 无上限）时最小加价为 `step`。
#[derive(Debug, Clone, PartialEq)]
pub struct BidStep {
    pub max_price: Option<Decimal>,
    pub step: Decimal,
}

/// 封禁判定结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BanAction {
    /// 不封禁（仅警告）
    None,
    /// 临时封禁（天数）
    Temp(i64),
    /// 永久封禁
    Permanent,
}

/// 拍卖全局设置（生效值）。
#[derive(Debug, Clone, PartialEq)]
pub struct AuctionSettings {
    /// ① 参拍门槛：true = 须企业认证通过；false = 登录即可
    pub require_certification: bool,
    /// ② 保证金比例（占起拍价，百分数）
    pub deposit_rate_percent: Decimal,
    /// ② 保证金保底（元）
    pub deposit_min: Decimal,
    /// ② 保证金封顶（元）
    pub deposit_max: Decimal,
    /// ③ 中标违约后处置：一期只有 `reauction`（重新上拍）一个有效值。
    /// `roll_down`（顺延次高价）为二期能力、逻辑未实现，`sanitize_settings`
    /// 对任何输入（含存量 `roll_down`）一律归一为 `reauction`，违约流程
    /// 行为不随该值变化；二期落地顺延逻辑时再恢复透传与按值分支。
    pub breach_disposal: String,
    /// ④ 尾款支付期限（小时）
    pub final_pay_hours: i64,
    /// ④ 违约扣罚比例（占保证金，百分数）
    pub penalty_rate_percent: Decimal,
    /// ④ 扣罚上限（占中标价，百分数）
    pub penalty_cap_rate_percent: Decimal,
    /// ④ 第 2 次违约的封禁天数
    pub second_breach_ban_days: i64,
    /// ④ 达到该违约次数永久封禁
    pub permanent_breach_count: i64,
    /// ⑤ 商户上拍开关（二期预留：一期默认关，且全仓无读取点——有意为之）。
    /// 一期没有商户创建拍品/场次的入口，建品入口只在平台管理端
    /// （/adminapi），「仅平台自营」由结构保证而非本开关拦截；
    /// 待二期商户自助上拍入口落地时，在该入口接线校验本开关。
    /// 不要往平台建品入口（`auction.rs` 的 item_save）加校验：开关为关时
    /// 平台自营必须能建品，加了要么误伤平台建品、要么是死代码。
    pub merchant_auction_enabled: bool,
    /// ⑥ 保留价开关（关 = 建拍品不可设保留价）
    pub reserve_price_enabled: bool,
    /// ⑥ 一口价开关（开 = 卖家可自选设置一口价）
    pub buyout_enabled: bool,
    /// ⑦ 退换货口径：false = 仅质量问题可退；true = 与普通商品同口径
    pub no_reason_return_allowed: bool,
    /// 延时竞价：结束前多少秒内出价触发顺延
    pub extend_window_seconds: i64,
    /// 延时竞价：每次顺延秒数
    pub extend_by_seconds: i64,
    /// 延时竞价：累计顺延上限秒数（另与原时长 2 倍取小）
    pub extend_max_seconds: i64,
    /// 阶梯加价表
    pub bid_steps: Vec<BidStep>,
}

/// 出厂阶梯加价表（规划 3.2 节推荐值）。
#[must_use]
pub fn default_bid_steps() -> Vec<BidStep> {
    vec![
        BidStep { max_price: Some(Decimal::from(1_000)), step: Decimal::from(10) },
        BidStep { max_price: Some(Decimal::from(10_000)), step: Decimal::from(50) },
        BidStep { max_price: Some(Decimal::from(50_000)), step: Decimal::from(100) },
        BidStep { max_price: Some(Decimal::from(200_000)), step: Decimal::from(500) },
        BidStep { max_price: Some(Decimal::from(1_000_000)), step: Decimal::from(1_000) },
        BidStep { max_price: None, step: Decimal::from(5_000) },
    ]
}

/// 出厂默认设置（= 规划推荐值）。
#[must_use]
pub fn default_settings() -> AuctionSettings {
    AuctionSettings {
        require_certification: true,
        deposit_rate_percent: Decimal::from(10),
        deposit_min: Decimal::from(100),
        deposit_max: Decimal::from(50_000),
        breach_disposal: "reauction".to_string(),
        final_pay_hours: 48,
        penalty_rate_percent: Decimal::from(50),
        penalty_cap_rate_percent: Decimal::from(20),
        second_breach_ban_days: 30,
        permanent_breach_count: 3,
        merchant_auction_enabled: false,
        reserve_price_enabled: false,
        buyout_enabled: true,
        no_reason_return_allowed: false,
        extend_window_seconds: 300,
        extend_by_seconds: 300,
        extend_max_seconds: 86_400,
        bid_steps: default_bid_steps(),
    }
}

/// 设置 → JSON（存库与下发前端共用形状）。
#[must_use]
pub fn settings_to_json(s: &AuctionSettings) -> Value {
    json!({
        "version": 1,
        "requireCertification": s.require_certification,
        "depositRatePercent": dec_str(s.deposit_rate_percent),
        "depositMin": dec_str(s.deposit_min),
        "depositMax": dec_str(s.deposit_max),
        "breachDisposal": s.breach_disposal,
        "finalPayHours": s.final_pay_hours,
        "penaltyRatePercent": dec_str(s.penalty_rate_percent),
        "penaltyCapRatePercent": dec_str(s.penalty_cap_rate_percent),
        "secondBreachBanDays": s.second_breach_ban_days,
        "permanentBreachCount": s.permanent_breach_count,
        "merchantAuctionEnabled": s.merchant_auction_enabled,
        "reservePriceEnabled": s.reserve_price_enabled,
        "buyoutEnabled": s.buyout_enabled,
        "noReasonReturnAllowed": s.no_reason_return_allowed,
        "extendWindowSeconds": s.extend_window_seconds,
        "extendBySeconds": s.extend_by_seconds,
        "extendMaxSeconds": s.extend_max_seconds,
        "bidSteps": s.bid_steps.iter().map(|b| json!({
            "maxPrice": b.max_price.map(dec_str),
            "step": dec_str(b.step),
        })).collect::<Vec<_>>(),
        "returnPolicyText": if s.no_reason_return_allowed {
            "拍卖商品与普通商品同口径支持退换货"
        } else {
            "拍卖商品除质量问题/与描述严重不符外，不支持无理由退货"
        },
    })
}

fn dec_str(d: Decimal) -> String {
    d.normalize().to_string()
}

fn get_bool(obj: &serde_json::Map<String, Value>, key: &str, fallback: bool) -> bool {
    obj.get(key).and_then(Value::as_bool).unwrap_or(fallback)
}

fn get_dec(obj: &serde_json::Map<String, Value>, key: &str, fallback: Decimal) -> Decimal {
    match obj.get(key) {
        Some(Value::Number(n)) => n
            .as_f64()
            .and_then(|f| Decimal::try_from(f).ok())
            .unwrap_or(fallback),
        Some(Value::String(s)) => s.trim().parse::<Decimal>().unwrap_or(fallback),
        _ => fallback,
    }
}

fn get_i64(obj: &serde_json::Map<String, Value>, key: &str, fallback: i64) -> i64 {
    match obj.get(key) {
        Some(Value::Number(n)) => n.as_i64().unwrap_or(fallback),
        Some(Value::String(s)) => s.trim().parse::<i64>().unwrap_or(fallback),
        _ => fallback,
    }
}

fn dec_in_range(obj: &serde_json::Map<String, Value>, key: &str, fallback: Decimal, min: Decimal, max: Decimal) -> Decimal {
    get_dec(obj, key, fallback).clamp(min, max)
}

fn parse_steps(input: Option<&Value>) -> Vec<BidStep> {
    let Some(list) = input.and_then(Value::as_array) else {
        return default_bid_steps();
    };
    let mut steps: Vec<BidStep> = Vec::new();
    for item in list {
        let Some(obj) = item.as_object() else { continue };
        let step = get_dec(obj, "step", Decimal::ZERO);
        if step <= Decimal::ZERO {
            continue;
        }
        let max_price = match obj.get("maxPrice") {
            None | Some(Value::Null) => None,
            Some(v) => {
                let d = match v {
                    Value::Number(n) => n.as_f64().and_then(|f| Decimal::try_from(f).ok()),
                    Value::String(s) => s.trim().parse::<Decimal>().ok(),
                    _ => None,
                };
                match d {
                    Some(d) if d > Decimal::ZERO => Some(d),
                    _ => continue,
                }
            }
        };
        steps.push(BidStep { max_price, step });
    }
    if steps.is_empty() {
        return default_bid_steps();
    }
    // 按上限升序、无上限置末，保证 step_for 的顺序扫描语义
    steps.sort_by(|a, b| match (a.max_price, b.max_price) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    steps
}

/// 校验/清洗一份设置（保存与读取共用）。输入不是对象时返回默认。
#[must_use]
pub fn sanitize_settings(input: &Value) -> AuctionSettings {
    let d = default_settings();
    let Some(obj) = input.as_object() else {
        return d;
    };
    let deposit_min = dec_in_range(obj, "depositMin", d.deposit_min, Decimal::ZERO, Decimal::from(1_000_000));
    let mut deposit_max = dec_in_range(obj, "depositMax", d.deposit_max, Decimal::ZERO, Decimal::from(100_000_000));
    if deposit_max < deposit_min {
        deposit_max = deposit_min;
    }
    AuctionSettings {
        require_certification: get_bool(obj, "requireCertification", d.require_certification),
        deposit_rate_percent: dec_in_range(obj, "depositRatePercent", d.deposit_rate_percent, Decimal::ZERO, Decimal::from(100)),
        deposit_min,
        deposit_max,
        // ③ 一期收口：只保留 reauction 一个有效值。存量配置中的 roll_down
        // （或任何其他值）一律归一为 reauction —— 顺延次高价逻辑留二期，
        // 二期落地时在此恢复 roll_down 透传，并在违约流程按值分支。
        breach_disposal: "reauction".to_string(),
        final_pay_hours: get_i64(obj, "finalPayHours", d.final_pay_hours).clamp(1, 720),
        penalty_rate_percent: dec_in_range(obj, "penaltyRatePercent", d.penalty_rate_percent, Decimal::ZERO, Decimal::from(100)),
        penalty_cap_rate_percent: dec_in_range(obj, "penaltyCapRatePercent", d.penalty_cap_rate_percent, Decimal::ZERO, Decimal::from(100)),
        second_breach_ban_days: get_i64(obj, "secondBreachBanDays", d.second_breach_ban_days).clamp(0, 3650),
        permanent_breach_count: get_i64(obj, "permanentBreachCount", d.permanent_breach_count).clamp(1, 100),
        merchant_auction_enabled: get_bool(obj, "merchantAuctionEnabled", d.merchant_auction_enabled),
        reserve_price_enabled: get_bool(obj, "reservePriceEnabled", d.reserve_price_enabled),
        buyout_enabled: get_bool(obj, "buyoutEnabled", d.buyout_enabled),
        no_reason_return_allowed: get_bool(obj, "noReasonReturnAllowed", d.no_reason_return_allowed),
        extend_window_seconds: get_i64(obj, "extendWindowSeconds", d.extend_window_seconds).clamp(0, 3600),
        extend_by_seconds: get_i64(obj, "extendBySeconds", d.extend_by_seconds).clamp(0, 3600),
        extend_max_seconds: get_i64(obj, "extendMaxSeconds", d.extend_max_seconds).clamp(0, 7 * 86_400),
        bid_steps: parse_steps(obj.get("bidSteps")),
    }
}

/// 由存储的原始 JSON 文本得到生效设置：解析失败/非对象 ⇒ 默认设置。
#[must_use]
pub fn effective_settings(stored: Option<&str>) -> AuctionSettings {
    let Some(raw) = stored else {
        return default_settings();
    };
    match serde_json::from_str::<Value>(raw) {
        Ok(v) if v.is_object() => sanitize_settings(&v),
        _ => default_settings(),
    }
}

impl AuctionSettings {
    /// 按起拍价计算保证金：比例计价后保底/封顶，保留 2 位小数。
    #[must_use]
    pub fn deposit_for(&self, start_price: Decimal) -> Decimal {
        if start_price <= Decimal::ZERO {
            return Decimal::ZERO;
        }
        let raw = (start_price * self.deposit_rate_percent / Decimal::from(100)).round_dp(2);
        raw.clamp(self.deposit_min, self.deposit_max)
    }

    /// 当前价所处档位的最小加价幅度。
    #[must_use]
    pub fn step_for(&self, current_price: Decimal) -> Decimal {
        for s in &self.bid_steps {
            match s.max_price {
                Some(max) if current_price >= max => continue,
                _ => return s.step,
            }
        }
        self.bid_steps.last().map(|s| s.step).unwrap_or(Decimal::from(10))
    }

    /// 下一口最低可出价：无人出价时为起拍价，否则当前价 + 当前档幅度。
    #[must_use]
    pub fn min_next_bid(&self, current_price: Option<Decimal>, start_price: Decimal) -> Decimal {
        match current_price {
            None => start_price,
            Some(c) => c + self.step_for(c),
        }
    }

    /// 出价合法性（规划 3.2）：首价 ≥ 起拍价；后续出价须为「当前价 + N×档幅」。
    #[must_use]
    pub fn is_valid_bid(&self, current_price: Option<Decimal>, start_price: Decimal, amount: Decimal) -> bool {
        match current_price {
            None => amount >= start_price,
            Some(c) => {
                let step = self.step_for(c);
                if step <= Decimal::ZERO || amount <= c {
                    return false;
                }
                (amount - c) % step == Decimal::ZERO
            }
        }
    }

    /// 延时判定：若在结束前窗口内出价，返回顺延后的新结束时间（秒）。
    /// 累计顺延受「原时长 2 倍」与 `extend_max_seconds` 双上限（取小）约束。
    #[must_use]
    pub fn extended_end(&self, now: i64, start_time: i64, orig_end: i64, real_end: i64) -> Option<i64> {
        if self.extend_window_seconds <= 0 || self.extend_by_seconds <= 0 {
            return None;
        }
        if real_end <= now || real_end - now > self.extend_window_seconds {
            return None;
        }
        let duration = (orig_end - start_time).max(0);
        let cap = orig_end + duration.saturating_mul(2).min(self.extend_max_seconds);
        let candidate = (now + self.extend_by_seconds).min(cap);
        if candidate > real_end {
            Some(candidate)
        } else {
            None
        }
    }

    /// 违约扣罚：保证金比例与中标价上限比例取小（规划 5.4）。
    #[must_use]
    pub fn penalty_for(&self, deposit: Decimal, final_price: Decimal) -> Decimal {
        if deposit <= Decimal::ZERO {
            return Decimal::ZERO;
        }
        let by_rate = (deposit * self.penalty_rate_percent / Decimal::from(100)).round_dp(2);
        let by_cap = (final_price.max(Decimal::ZERO) * self.penalty_cap_rate_percent / Decimal::from(100)).round_dp(2);
        by_rate.min(by_cap).min(deposit).max(Decimal::ZERO)
    }

    /// 违约次数（本次之后累计）→ 封禁动作（规划 5.4 阶梯）。
    #[must_use]
    pub fn ban_action(&self, breach_count_after: i64) -> BanAction {
        if breach_count_after >= self.permanent_breach_count.max(1) {
            BanAction::Permanent
        } else if breach_count_after >= 2 && self.second_breach_ban_days > 0 {
            BanAction::Temp(self.second_breach_ban_days)
        } else {
            BanAction::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn defaults_match_recommendations() {
        let s = default_settings();
        assert!(s.require_certification);
        assert_eq!(s.deposit_rate_percent, Decimal::from(10));
        assert_eq!(s.deposit_min, Decimal::from(100));
        assert_eq!(s.deposit_max, Decimal::from(50_000));
        assert_eq!(s.breach_disposal, "reauction");
        assert_eq!(s.final_pay_hours, 48);
        assert_eq!(s.penalty_rate_percent, Decimal::from(50));
        assert_eq!(s.penalty_cap_rate_percent, Decimal::from(20));
        assert_eq!(s.second_breach_ban_days, 30);
        assert_eq!(s.permanent_breach_count, 3);
        assert!(!s.merchant_auction_enabled);
        assert!(!s.reserve_price_enabled);
        assert!(s.buyout_enabled);
        assert!(!s.no_reason_return_allowed);
    }

    #[test]
    fn deposit_calc_floor_cap_and_ratio() {
        let s = default_settings();
        // 保底：800 × 10% = 80 < 100 → 100
        assert_eq!(s.deposit_for(dec("800")), dec("100"));
        // 比例：30000 × 10% = 3000
        assert_eq!(s.deposit_for(dec("30000")), dec("3000"));
        // 封顶：2000000 × 10% = 200000 > 50000 → 50000
        assert_eq!(s.deposit_for(dec("2000000")), dec("50000"));
        // 边界与小数：12345 × 10% = 1234.50
        assert_eq!(s.deposit_for(dec("12345")), dec("1234.5"));
        assert_eq!(s.deposit_for(Decimal::ZERO), Decimal::ZERO);
    }

    #[test]
    fn step_tiers() {
        let s = default_settings();
        assert_eq!(s.step_for(dec("0")), dec("10"));
        assert_eq!(s.step_for(dec("999")), dec("10"));
        assert_eq!(s.step_for(dec("1000")), dec("50"));
        assert_eq!(s.step_for(dec("9999")), dec("50"));
        assert_eq!(s.step_for(dec("10000")), dec("100"));
        assert_eq!(s.step_for(dec("50000")), dec("500"));
        assert_eq!(s.step_for(dec("200000")), dec("1000"));
        assert_eq!(s.step_for(dec("1000000")), dec("5000"));
        assert_eq!(s.step_for(dec("99999999")), dec("5000"));
    }

    #[test]
    fn bid_validation() {
        let s = default_settings();
        let start = dec("1000");
        // 首价：等于起拍价可，大于可，小于不可
        assert!(s.is_valid_bid(None, start, dec("1000")));
        assert!(s.is_valid_bid(None, start, dec("1350")));
        assert!(!s.is_valid_bid(None, start, dec("999")));
        // 后续：当前 1000（档幅 50）→ 1050/1100 可，1025 不可，1000 不可
        let cur = Some(dec("1000"));
        assert!(s.is_valid_bid(cur, start, dec("1050")));
        assert!(s.is_valid_bid(cur, start, dec("1100")));
        assert!(!s.is_valid_bid(cur, start, dec("1025")));
        assert!(!s.is_valid_bid(cur, start, dec("1000")));
        assert_eq!(s.min_next_bid(cur, start), dec("1050"));
        assert_eq!(s.min_next_bid(None, start), dec("1000"));
    }

    #[test]
    fn extend_rules() {
        let s = default_settings();
        // 原时长 3600s，开始 1000，结束 4600；now=4400（剩 200s < 300 窗口）→ 顺延到 4700
        assert_eq!(s.extended_end(4400, 1000, 4600, 4600), Some(4700));
        // 剩 400s > 窗口 → 不顺延
        assert_eq!(s.extended_end(4200, 1000, 4600, 4600), None);
        // 已结束 → 不顺延
        assert_eq!(s.extended_end(4600, 1000, 4600, 4600), None);
        // 累计上限：原时长 3600 → cap = 4600 + min(7200, 86400) = 11800
        // real_end 已到 11750，now=11700 → candidate = 12000 被 cap 到 11800
        assert_eq!(s.extended_end(11700, 1000, 4600, 11750), Some(11800));
        // 已达 cap → 不再顺延
        assert_eq!(s.extended_end(11750, 1000, 4600, 11800), None);
    }

    #[test]
    fn penalty_rules() {
        let s = default_settings();
        // 保证金 3000，中标价 50000：50% = 1500，上限 20% = 10000 → 1500
        assert_eq!(s.penalty_for(dec("3000"), dec("50000")), dec("1500"));
        // 低价拍品：保证金 100（保底），中标价 90：50% = 50，上限 20% = 18 → 18
        assert_eq!(s.penalty_for(dec("100"), dec("90")), dec("18"));
        // 无保证金 → 0
        assert_eq!(s.penalty_for(Decimal::ZERO, dec("90")), Decimal::ZERO);
    }

    #[test]
    fn ban_ladder() {
        let s = default_settings();
        assert_eq!(s.ban_action(1), BanAction::None);
        assert_eq!(s.ban_action(2), BanAction::Temp(30));
        assert_eq!(s.ban_action(3), BanAction::Permanent);
        assert_eq!(s.ban_action(9), BanAction::Permanent);
    }

    #[test]
    fn sanitize_clamps_and_falls_back() {
        // 非对象 → 默认
        assert_eq!(effective_settings(Some("[1,2]")), default_settings());
        assert_eq!(effective_settings(None), default_settings());
        assert_eq!(effective_settings(Some("not json")), default_settings());
        // 越界值被钳制
        let out = sanitize_settings(&json!({
            "depositRatePercent": 250,
            "depositMin": 500,
            "depositMax": 100,
            "finalPayHours": 99999,
            "breachDisposal": "whatever",
            "bidSteps": [{"maxPrice": 100, "step": 5}, {"maxPrice": null, "step": 20}]
        }));
        assert_eq!(out.deposit_rate_percent, Decimal::from(100));
        assert_eq!(out.deposit_min, Decimal::from(500));
        assert_eq!(out.deposit_max, Decimal::from(500)); // max < min 时抬到 min
        assert_eq!(out.final_pay_hours, 720);
        assert_eq!(out.breach_disposal, "reauction");
        // ③ 一期收口：存量 roll_down 也必须归一为 reauction（二期才恢复透传）
        let legacy = sanitize_settings(&json!({ "breachDisposal": "roll_down" }));
        assert_eq!(legacy.breach_disposal, "reauction");
        assert_eq!(out.bid_steps.len(), 2);
        assert_eq!(out.step_for(dec("50")), dec("5"));
        assert_eq!(out.step_for(dec("5000")), dec("20"));
        // 往返：JSON 序列化后再清洗应等值
        let round = sanitize_settings(&settings_to_json(&out));
        assert_eq!(round, out);
    }
}
