//! 门店同城配送配置的计费口径（纯逻辑，我方新增接线）。
//!
//! 配置面（`shop_local_delivery_config` 的 `base_delivery_fee` /
//! `min_order_amount` / `distance_pricing_enabled` 与距离溢价行
//! `shop_local_delivery_distance_pricing`）为原版有，后台配置页文案
//! 早已宣称「未开启叠加溢价时统一基础运费」「最终费用 = 基础运费
//! + 叠加溢价」「订单商品优惠前金额低于起送价不能下单」，但结算/
//! 下单链此前从未消费这两个字段（同城运费全部来自运费模板）。本
//! 模块把文案口径落成纯函数，服务层（`services::trade::check`）在
//! 同城配送（delivery_type / shipping_method = 3）时消费：
//!
//! - 基础运费：`base_delivery_fee > 0` 才接管该店同城运费（替换
//!   运费模板结果）；为 0/未配时逐字沿用运费模板，存量行为不变。
//! - 叠加溢价：仅 `distance_pricing_enabled = 1` 时在基础运费上
//!   按买家地址→门店距离加收；距离未知（地址/门店无坐标）时溢价
//!   记 0，只收基础运费，不阻断下单。
//! - 起送价：`min_order_amount > 0` 且该店商品金额（活动价计价、
//!   优惠券/积分抵扣前，与结算页 `store_amounts` 及优惠券门槛同
//!   基数）低于起送价时给出差额；结算页据此下发
//!   `isNotReachMinOrderAmount` / `minOrderAmountDiff`（H5 已有
//!   「还差 ¥X 就能起送」提示与提交禁用），下单时硬拦截。

use rust_decimal::Decimal;

/// 距离溢价规则行：（起算门槛 km，计价单位 km，每单位加价 元），
/// 与 `shop_local_delivery_distance_pricing` 三列一一对应。
pub type DistancePricingRule = (Decimal, Decimal, Decimal);

/// 距离溢价：取「门槛被距离超过」的最高一档计价——超出门槛的
/// 距离按计价单位向上取整累加。无命中档位、单位或加价非正时
/// 溢价为 0（不猜测、不报错，与运费模板规则不完整时的处理同级）。
#[must_use]
pub fn distance_surcharge(rules: &[DistancePricingRule], distance_km: Decimal) -> Decimal {
    let mut best: Option<&DistancePricingRule> = None;
    for rule in rules {
        if distance_km > rule.0 && best.map_or(true, |b| rule.0 >= b.0) {
            best = Some(rule);
        }
    }
    let Some((threshold, unit, fee)) = best else {
        return Decimal::ZERO;
    };
    if *unit <= Decimal::ZERO || *fee <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let units = ((distance_km - *threshold) / *unit).ceil();
    (units * *fee).max(Decimal::ZERO)
}

/// 同城单店配送费 = 基础运费 +（开启叠加溢价时）距离溢价。
/// `distance_km` 为 None（坐标缺失）时溢价记 0。
#[must_use]
pub fn local_delivery_fee(
    base_fee: Decimal,
    distance_pricing_enabled: bool,
    rules: &[DistancePricingRule],
    distance_km: Option<Decimal>,
) -> Decimal {
    let mut fee = base_fee.max(Decimal::ZERO);
    if distance_pricing_enabled {
        if let Some(km) = distance_km {
            fee += distance_surcharge(rules, km);
        }
    }
    fee
}

/// 起送价差额：配置起送价为正且店铺商品金额未达时返回还差金额，
/// 其余情况（未配/已达）返回 None。
#[must_use]
pub fn min_order_shortfall(shop_amount: Decimal, min_order_amount: Decimal) -> Option<Decimal> {
    if min_order_amount > Decimal::ZERO && shop_amount < min_order_amount {
        Some(min_order_amount - shop_amount)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn surcharge_below_threshold_is_zero() {
        let rules = vec![(d("3"), d("1"), d("2"))];
        assert_eq!(distance_surcharge(&rules, d("2.9")), Decimal::ZERO);
        assert_eq!(distance_surcharge(&rules, d("3")), Decimal::ZERO);
        assert_eq!(distance_surcharge(&[], d("10")), Decimal::ZERO);
    }

    #[test]
    fn surcharge_rounds_up_partial_units() {
        let rules = vec![(d("3"), d("1"), d("2"))];
        // 超出 0.1km 按 1 个单位计
        assert_eq!(distance_surcharge(&rules, d("3.1")), d("2"));
        // 超出恰好 2km 按 2 个单位计
        assert_eq!(distance_surcharge(&rules, d("5")), d("4"));
        // 超出 2.01km 按 3 个单位计
        assert_eq!(distance_surcharge(&rules, d("5.01")), d("6"));
    }

    #[test]
    fn surcharge_picks_highest_matching_tier() {
        let rules = vec![(d("3"), d("1"), d("2")), (d("10"), d("2"), d("3"))];
        // 12km 命中 10km 档：超出 2km / 单位 2km = 1 单位 × 3 元
        assert_eq!(distance_surcharge(&rules, d("12")), d("3"));
        // 5km 只命中 3km 档
        assert_eq!(distance_surcharge(&rules, d("5")), d("4"));
    }

    #[test]
    fn surcharge_incomplete_rule_is_zero() {
        let rules = vec![(d("3"), d("0"), d("2"))];
        assert_eq!(distance_surcharge(&rules, d("9")), Decimal::ZERO);
        let rules = vec![(d("3"), d("1"), d("0"))];
        assert_eq!(distance_surcharge(&rules, d("9")), Decimal::ZERO);
    }

    #[test]
    fn fee_is_base_plus_surcharge_only_when_pricing_enabled() {
        let rules = vec![(d("3"), d("1"), d("2"))];
        assert_eq!(
            local_delivery_fee(d("5"), false, &rules, Some(d("9"))),
            d("5")
        );
        assert_eq!(
            local_delivery_fee(d("5"), true, &rules, Some(d("5"))),
            d("9")
        );
        // 距离未知：只收基础运费
        assert_eq!(local_delivery_fee(d("5"), true, &rules, None), d("5"));
        // 基础运费为 0 且未开溢价：0（服务层据此不接管，沿用模板）
        assert_eq!(
            local_delivery_fee(Decimal::ZERO, false, &rules, Some(d("9"))),
            Decimal::ZERO
        );
    }

    #[test]
    fn shortfall_only_when_below_positive_minimum() {
        assert_eq!(min_order_shortfall(d("30"), d("50")), Some(d("20")));
        assert_eq!(min_order_shortfall(d("50"), d("50")), None);
        assert_eq!(min_order_shortfall(d("60"), d("50")), None);
        assert_eq!(min_order_shortfall(d("0"), Decimal::ZERO), None);
    }
}
