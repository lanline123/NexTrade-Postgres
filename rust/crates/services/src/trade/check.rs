//! 订单结算域服务（trade::check）——结算页 v2。
//!
//! 对应原 Java `com.nextrade.service.order.impl.OrderCheckServiceImpl` 的
//! `getAvailablePaymentType` / `getLastInvoice` / `getOrderCheckData` /
//! `getStoreShippingType`，并补齐结算价格引擎的主要部分：
//!
//! - **运费模板**：按商品 `shipping_tpl_id`（缺省回退店铺默认模板）+ 地区规则
//!   计算店铺运费；包邮、满额包邮、按件/按重、续件/续重均支持（纯函数
//!   [`nextrade_core::models::trade::check::calc_shipping_fee`]）。
//! - **积分抵扣**：按 `integralScale` / `integralPercent` / `integralPercentBaseAmount`
//!   计算可用积分、实际积分与抵扣金额（[`nextrade_core::models::trade::check::calc_points`]）。
//! - **优惠券店铺范围与多店分摊**：`coupon.shop_id` 限定店铺，全局券按商品金额
//!   比例分摊到各店铺；不可用券进入 `disableCoupons`。
//! - **账期**：`useCredit` 配置 + `credit_user` 授信，返回 `creditPaymentInfo` 与
//!   `creditAmount`；余额/账期互斥。
//! - **预售**：读取 `product_presale`，返回 `productPresale` 与定金/尾款金额。
//! - **改价标记**：购物车存在改价数据时置 `hasChangePrice`。
//!
//! 仍未迁移（需要营销域活动引擎，另行排期）：秒杀/拼团/满减活动价格、O2O 配送
//! 选项/自提点、发票信息随单落库、代客下单整单改价金额、多店铺优惠券逐券明细落库。
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use nextrade_core::entities::{
    Cart, Coupon, GrouponPromotion, GrouponPromotionItem, Product, SeckillItem, ShippingTpl,
    ShippingTplInfo, UserAddress,
};
use nextrade_core::models::catalog::exchange::CART_TYPE_EXCHANGE;
use nextrade_core::models::trade::check::{
    allocate_discount, build_payment_types, calc_points, calc_shipping_fee, config_enabled,
    coupon_rule_discount, groupon_min_group_num, groupon_price_for, parse_groupon_data,
    parse_promotion_tiers, parse_sku_data, pick_promotion_tier, promotion_discount,
    gift_card_deduct_amount, resolve_gift_card_selection, AvailableGiftCardVO,
    CartStoreTotalVO, CheckCartByStoreVO, CheckCartVO, CheckCouponVO,
    CheckItemVO, CheckTotalVO, CouponListVO, GrouponCartData, OrderCheckParam, OrderCheckVO,
    OrderInvoiceVO, PaymentTypeVO, ShippingRule, ShippingTypeVO, StoreShippingTypeQuery,
    CONFIG_USE_OFFLINE, CONFIG_USE_SURPLUS, MSG_CART_EMPTY, PAYMENT_TYPE_ONLINE,
};
use nextrade_core::models::marketing::gift_card::{
    mask_middle_half, scope_type_name, scoped_deduct_base, GiftCardScope, SCOPE_TYPE_ALL,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::trade::check::CheckRepository;
use nextrade_infrastructure::db::Repositories;

use crate::marketing::coupon::CouponService;
use nextrade_core::models::marketing::coupon::UserCouponVO;

/// 结算相关配置键（积分/账期）。
const CFG_INTEGRAL_SCALE: &str = "integralScale";
const CFG_INTEGRAL_PERCENT: &str = "integralPercent";
const CFG_INTEGRAL_PERCENT_BASE: &str = "integralPercentBaseAmount";
const CFG_USE_POINTS: &str = "usePoints";
const CFG_USE_CREDIT: &str = "useCredit";

/// 按地区匹配适用税率（Java `OrderCheckServiceImpl#resolveOrderTaxRate`）。
///
/// 从末级区域到宏观区域依次查询启用的税率记录。
pub async fn resolve_tax_rate(
    repos: &Repositories,
    region_ids: &[i32],
) -> (Decimal, Option<nextrade_core::entities::setting::TaxRate>) {
    for &rid in region_ids.iter().rev() {
        if rid > 0 {
            if let Ok(Some(tr)) = repos.payment.tax_rate.find_active_by_region_id(rid).await {
                return (tr.rate, Some(tr));
            }
        }
    }
    for &rid in region_ids {
        if rid > 0 {
            if let Ok(Some(tr)) = repos.payment.tax_rate.find_active_by_region_id(rid).await {
                return (tr.rate, Some(tr));
            }
        }
    }
    (Decimal::ZERO, None)
}

/// 计算税率金额（Java `OrderCheckServiceImpl`:
/// `taxRateAmount = taxBaseAmount.multiply(taxRate / 100).setScale(2, RoundingMode.CEILING)`）。
pub fn calc_tax_amount(taxable_base: Decimal, tax_rate_percent: Decimal) -> Decimal {
    if tax_rate_percent <= Decimal::ZERO || taxable_base <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let rate = tax_rate_percent / Decimal::from(100);
    let raw = taxable_base * rate;
    raw.round_dp_with_strategy(2, rust_decimal::RoundingStrategy::AwayFromZero)
}

/// 订单结算域服务。
pub struct CheckService {
    repos: Arc<Repositories>,
}

/// 店铺运费计算结果。
pub(crate) struct ShippingCalc {
    pub total: Decimal,
    pub per_store: HashMap<i32, Decimal>,
}

/// 优惠券试算结果。
struct CouponCalc {
    total: Decimal,
    per_store: HashMap<i32, Decimal>,
    valid_selected: Vec<i32>,
    enable: Vec<CheckCouponVO>,
    disable: Vec<serde_json::Value>,
}

impl CheckService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本域仓储（NESTED 访问：`self.repos.trade.check`）。
    #[must_use]
    pub fn repo(&self) -> &CheckRepository {
        &self.repos.trade.check
    }

    /// 可用支付方式（Java `getAvailablePaymentType`）。
    pub async fn available_payment_types(
        &self,
        flow_type: Option<i32>,
    ) -> Result<Vec<PaymentTypeVO>> {
        let repo = self.repo();
        let use_offline =
            config_enabled(repo.find_config_value(CONFIG_USE_OFFLINE).await?.as_deref());
        let use_surplus =
            config_enabled(repo.find_config_value(CONFIG_USE_SURPLUS).await?.as_deref());
        Ok(build_payment_types(
            use_offline,
            use_surplus,
            false,
            flow_type,
        ))
    }

    /// 最近一条订单发票（Java `getLastInvoice`）。
    pub async fn last_invoice(
        &self,
        user_id: i32,
        invoice_type: Option<i32>,
        title_type: Option<i32>,
    ) -> Result<Option<OrderInvoiceVO>> {
        self.repo()
            .find_last_invoice(user_id, invoice_type, title_type)
            .await
    }

    /// 结算页 index。
    pub async fn order_check_index(
        &self,
        user_id: i32,
        param: &OrderCheckParam,
    ) -> Result<OrderCheckVO> {
        self.build_check(user_id, param).await
    }

    /// 结算页 update（地址/配送/优惠券变化后重算）。
    pub async fn order_check_update(
        &self,
        user_id: i32,
        param: &OrderCheckParam,
    ) -> Result<OrderCheckVO> {
        self.build_check(user_id, param).await
    }

    /// 结算页 updateCoupon（只关注优惠券选择，重算金额）。
    pub async fn order_check_update_coupon(
        &self,
        user_id: i32,
        param: &OrderCheckParam,
    ) -> Result<OrderCheckVO> {
        self.build_check(user_id, param).await
    }

    /// 店铺配送方式（Java `getStoreShippingType`）。
    pub async fn store_shipping_types(
        &self,
        query: &StoreShippingTypeQuery,
    ) -> Result<HashMap<i32, Vec<ShippingTypeVO>>> {
        let user_id = query.user_id.unwrap_or(0);
        let mut shop_ids: Vec<i32> = Vec::new();
        if user_id > 0 {
            let carts = self.repos.trade.cart.find_by_user(user_id, None).await?;
            for cart in carts.iter().filter(|c| c.is_checked()) {
                if !shop_ids.contains(&cart.shop_id) {
                    shop_ids.push(cart.shop_id);
                }
            }
        }
        let all = self.repos.logistics.shipping_type.list_all().await?;
        if shop_ids.is_empty() {
            let mut grouped: HashMap<i32, Vec<ShippingTypeVO>> = HashMap::new();
            for item in &all {
                grouped
                    .entry(item.shop_id)
                    .or_default()
                    .push(to_shipping_vo(item));
            }
            if grouped.is_empty() {
                grouped.insert(0, default_shipping_types(0));
            }
            return Ok(grouped);
        }
        let mut grouped: HashMap<i32, Vec<ShippingTypeVO>> = HashMap::new();
        for shop_id in shop_ids {
            let list: Vec<ShippingTypeVO> = all
                .iter()
                .filter(|s| s.shop_id == shop_id || s.shop_id == 0)
                .map(|s| to_shipping_vo(s))
                .collect();
            if list.is_empty() {
                grouped.insert(shop_id, default_shipping_types(shop_id));
            } else {
                grouped.insert(shop_id, list);
            }
        }
        Ok(grouped)
    }

    /// 结算页数据装配（v2）。
    async fn build_check(&self, user_id: i32, param: &OrderCheckParam) -> Result<OrderCheckVO> {
        let carts: Vec<Cart> = self
            .repos
            .trade
            .cart
            .find_by_user(user_id, None)
            .await?
            .into_iter()
            .filter(Cart::is_checked)
            .collect();
        if carts.is_empty() {
            return Err(Error::biz(1001, MSG_CART_EMPTY));
        }

        let mut product_ids: Vec<i32> = Vec::new();
        for cart in &carts {
            if !product_ids.contains(&cart.product_id) {
                product_ids.push(cart.product_id);
            }
        }
        let products = load_products(&self.repos, &product_ids).await?;
        let now = nextrade_common::utils::now_seconds() as i32;
        let skus = super::cart::load_cart_skus(&self.repos, &carts).await?;
        let pricing = price_cart_activities(
            &self.repos,
            user_id,
            &carts,
            &products,
            &skus,
            now,
            param.line_price_overrides.as_ref(),
        )
        .await?;
        let (seckill_total, groupon_total, time_discount_total, full_reduce_total) = (
            pricing.seckill.values().copied().sum::<Decimal>(),
            pricing.groupon.values().copied().sum::<Decimal>(),
            pricing.time.values().copied().sum::<Decimal>(),
            pricing.full_reduce.values().copied().sum::<Decimal>(),
        );
        let mut shop_groups: BTreeMap<i32, Vec<&Cart>> = BTreeMap::new();
        let mut store_amounts: BTreeMap<i32, Decimal> = BTreeMap::new();
        let mut product_amount = Decimal::ZERO;
        let mut total_quantity = 0;
        for cart in &carts {
            let price = pricing.prices[&cart.cart_id] + super::cart::cart_extra_price(cart);
            let qty = cart.quantity.max(1);
            let subtotal = price * Decimal::from(qty);
            product_amount += subtotal;
            total_quantity += qty;
            *store_amounts.entry(cart.shop_id).or_default() += subtotal;
            shop_groups.entry(cart.shop_id).or_default().push(cart);
        }

        let user = self.repos.member.user.find_by_id(user_id).await?;
        let balance = user.as_ref().map(|u| u.balance).unwrap_or_default();
        let user_points = user.as_ref().map(|u| u.points).unwrap_or(0);
        let default_address_id = user.as_ref().map(|u| u.address_id).unwrap_or(0);
        let address_id = param
            .address_id
            .filter(|id| *id > 0)
            .unwrap_or(default_address_id);
        let region_ids = self.address_region_ids(address_id).await?;

        let payments = self.available_payment_types(param.flow_type).await?;
        let pay_type_id = param
            .pay_type_id
            .filter(|id| *id > 0)
            .or_else(|| payments.first().map(|p| p.type_id))
            .unwrap_or(PAYMENT_TYPE_ONLINE);
        let flow_type = param.flow_type.unwrap_or(1);

        // —— 运费模板 ——
        let shipping = calculate_shipping(
            &self.repos,
            &carts,
            &products,
            &region_ids,
            param.delivery_type,
        )
        .await?;

        // —— 优惠券（店铺范围 + 多店分摊）——
        let coupon_service = CouponService::new(self.repos.clone());
        let usable = coupon_service.my_usable_coupons(user_id).await?;
        let selected: Vec<i32> = param
            .use_coupon_ids
            .clone()
            .or_else(|| param.select_user_coupon_ids.clone())
            .unwrap_or_default();
        let coupons = self
            .calc_coupons(&usable, &selected, &store_amounts, product_amount)
            .await?;

        // —— 积分抵扣 ——
        let cfg = self
            .repo()
            .find_config_values(&[
                CFG_INTEGRAL_SCALE,
                CFG_INTEGRAL_PERCENT,
                CFG_INTEGRAL_PERCENT_BASE,
                CFG_USE_POINTS,
                CFG_USE_CREDIT,
            ])
            .await?;
        let points_enabled = config_enabled(cfg.get(CFG_USE_POINTS).map(String::as_str));
        let mut points = if points_enabled {
            let base_after_coupon = (product_amount - coupons.total).max(Decimal::ZERO);
            calc_points(
                param.use_point.unwrap_or(0),
                user_points,
                base_after_coupon,
                decimal_config(&cfg, CFG_INTEGRAL_SCALE, Decimal::ZERO),
                decimal_config(&cfg, CFG_INTEGRAL_PERCENT, Decimal::ZERO),
                decimal_config(&cfg, CFG_INTEGRAL_PERCENT_BASE, Decimal::ONE_HUNDRED),
            )
        } else {
            Default::default()
        };

        // —— 积分兑换（Java OrderCheckServiceImpl flowType==3 分支）——
        // 兑换积分用量与抵扣金额由积分商品配置（exchange_integral / points_deducted_amount）决定，
        // 不受积分支付比例约束，也不进入普通订单封顶逻辑（Java flowType!=3 才 min 封顶）。
        let mut exchange_points = 0;
        if flow_type == CART_TYPE_EXCHANGE {
            let mut exchange_product_id = 0;
            let mut exchange_sku_id = 0;
            let mut exchange_number = 0i64;
            for cart in &carts {
                exchange_product_id = cart.product_id;
                exchange_sku_id = cart.sku_id.unwrap_or(0);
                exchange_number += i64::from(cart.quantity.max(1));
            }
            if exchange_product_id > 0 {
                let activity = self
                    .repos
                    .marketing
                    .points_exchange
                    .find_by_product(exchange_product_id, exchange_sku_id)
                    .await
                    .unwrap_or_default();
                if let Some(activity) = activity {
                    exchange_points = activity.exchange_integral * exchange_number as i32;
                    points.points_amount =
                        activity.points_deducted_amount * Decimal::from(exchange_number);
                    points.used_points = exchange_points;
                }
            }
        }

        // —— 地区税率计算（Java OrderCheckServiceImpl#resolveOrderTaxRate）——
        let (tax_rate, _) = resolve_tax_rate(&self.repos, &region_ids).await;
        let taxable_base = (product_amount
            - coupons.total
            - seckill_total
            - groupon_total
            - full_reduce_total
            - time_discount_total)
        .max(Decimal::ZERO);
        let tax_rate_amount = calc_tax_amount(taxable_base, tax_rate);

        // —— 礼品卡（标准对齐 D5/D6，原版结算口径）——
        // 普通流：单选一张储值卡抵扣，封顶 = 券/积分后应付（顺序 券→积分→礼品卡→余额）；
        // 可选卡由本响应 availableGiftCards 下发（不另调 myCards）。
        // flowType=12（兑换卡流）：已核销的兑换卡覆盖商品金额（运费/税另付），
        // 选择器锁定，不再叠加储值卡抵扣。
        let mut available_gift_cards: Vec<AvailableGiftCardVO> = Vec::new();
        let mut gift_card_amount = Decimal::ZERO;
        let mut selected_gift_card_id: i32 = 0;
        let mut selected_gift_card_sn: Option<String> = None;
        if flow_type == 12 {
            gift_card_amount =
                (product_amount - coupons.total - points.points_amount).max(Decimal::ZERO);
            // 锁定卡回显：取购物车行 extra_data 标记的兑换卡
            let locked_id = carts.iter().find_map(|c| {
                c.extra_data
                    .as_deref()
                    .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
                    .and_then(|v| v.get("giftCardsId").and_then(serde_json::Value::as_i64))
                    .and_then(|v| i32::try_from(v).ok())
            });
            if let Some(card_id) = locked_id {
                selected_gift_card_id = card_id;
                if let Ok(Some(card)) =
                    self.repos.marketing.gift_card.find_by_id(card_id).await
                {
                    selected_gift_card_sn = card.gift_card_sn.clone();
                    let group_name = self
                        .repos
                        .marketing
                        .gift_card
                        .find_group(card.card_groups_id)
                        .await
                        .ok()
                        .flatten()
                        .map(|g| g.card_group_name);
                    available_gift_cards.push(AvailableGiftCardVO {
                        gift_cards_id: card_id,
                        gift_card_sn: card.gift_card_sn.clone(),
                        card_group_name: group_name,
                        card_number: Some(mask_middle_half(&card.card_number)),
                        face_amount: None,
                        balance: None,
                        expired_time: card.expired_time,
                        // 兑换卡由绑定商品池天然限定，不走储值卡适用范围
                        scope_type: SCOPE_TYPE_ALL,
                        scope_desc: scope_type_name(SCOPE_TYPE_ALL).to_string(),
                    });
                }
            }
            if selected_gift_card_sn.is_none() {
                selected_gift_card_sn = param
                    .gift_card_sn
                    .clone()
                    .or_else(|| param.use_gift_card_sn.clone());
            }
        } else if flow_type != CART_TYPE_EXCHANGE {
            let stored = self
                .repos
                .marketing
                .gift_card_issue
                .list_available_stored_cards(user_id, now)
                .await
                .unwrap_or_default();
            for row in &stored {
                available_gift_cards.push(AvailableGiftCardVO {
                    gift_cards_id: row.gift_cards_id,
                    gift_card_sn: row.gift_card_sn.clone(),
                    card_group_name: row.card_group_name.clone(),
                    card_number: Some(mask_middle_half(&row.card_number)),
                    face_amount: row.face_amount,
                    balance: row.balance,
                    expired_time: row.expired_time,
                    scope_type: row.scope_type,
                    scope_desc: scope_type_name(row.scope_type).to_string(),
                });
            }
            let (sel_id, sel_sn) = resolve_gift_card_selection(
                param.use_gift_card_id,
                param.use_gift_card_sn.as_deref(),
            );
            if sel_id.is_some() || sel_sn.is_some() {
                let base = (product_amount + shipping.total + tax_rate_amount
                    - coupons.total
                    - points.points_amount)
                    .max(Decimal::ZERO);
                let found = stored.iter().find(|r| {
                    sel_id == Some(r.gift_cards_id)
                        || sel_sn.as_deref() == r.gift_card_sn.as_deref()
                });
                if let Some(row) = found {
                    // 适用范围（我方新增）：限定卡的抵扣基数只计范围内商品金额
                    // （运费/税/优惠按比例分摊，见 core `scoped_deduct_base`）；
                    // 范围外订单 gift_card_amount = 0，前端据 scopeType 给出明确提示。
                    let cap = if row.scope_type == SCOPE_TYPE_ALL {
                        base
                    } else {
                        let scope = self.load_gift_card_scope(row.card_groups_id).await;
                        let lines: Vec<(i32, i32, Decimal)> = carts
                            .iter()
                            .map(|cart| {
                                let price = pricing.prices[&cart.cart_id]
                                    + super::cart::cart_extra_price(cart);
                                let subtotal = price * Decimal::from(cart.quantity.max(1));
                                let category_id = products
                                    .get(&cart.product_id)
                                    .map(|p| p.category_id)
                                    .unwrap_or(0);
                                (cart.product_id, category_id, subtotal)
                            })
                            .collect();
                        scoped_deduct_base(&scope, &lines, product_amount, base)
                    };
                    gift_card_amount =
                        gift_card_deduct_amount(row.balance.unwrap_or(Decimal::ZERO), cap);
                    selected_gift_card_id = row.gift_cards_id;
                    selected_gift_card_sn = row.gift_card_sn.clone();
                }
            }
        }

        // —— 应付 / 余额 / 账期 ——
        let total_before_balance =
            (product_amount + shipping.total + tax_rate_amount - coupons.total - points.points_amount)
                .max(Decimal::ZERO);
        let balance_used = if param.use_balance.unwrap_or_default() > Decimal::ZERO {
            balance
                .min((total_before_balance - gift_card_amount).max(Decimal::ZERO))
                .max(Decimal::ZERO)
        } else {
            Decimal::ZERO
        };
        let use_credit_requested = param.use_credit == Some(1);
        if use_credit_requested && balance_used > Decimal::ZERO {
            return Err(Error::biz(1001, "账期支付与余额支付不能同时使用"));
        }
        let credit_enabled =
            config_enabled(cfg.get(CFG_USE_CREDIT).map(String::as_str));
        let credit_info = if credit_enabled {
            self.credit_payment_info(user_id, total_before_balance).await
        } else {
            None
        };
        let mut credit_amount = Decimal::ZERO;
        if use_credit_requested {
            if !credit_enabled {
                return Err(Error::biz(1001, "账期支付未启用"));
            }
            let can_use = credit_info
                .as_ref()
                .and_then(|v| v.get("canUseCredit"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            if !can_use {
                let reason = credit_info
                    .as_ref()
                    .and_then(|v| v.get("disableReason"))
                    .and_then(serde_json::Value::as_str)
                    .filter(|s| !s.is_empty())
                    .unwrap_or("当前不可使用账期支付");
                return Err(Error::biz(1001, reason.to_string()));
            }
            let available = credit_info
                .as_ref()
                .and_then(|v| v.get("availableAmount"))
                .and_then(|v| v.as_str().and_then(|s| s.parse::<Decimal>().ok()));
            credit_amount = available
                .unwrap_or(total_before_balance)
                .min((total_before_balance - gift_card_amount - balance_used).max(Decimal::ZERO))
                .max(Decimal::ZERO);
        }
        let unpaid_amount =
            (total_before_balance - gift_card_amount - balance_used - credit_amount)
                .max(Decimal::ZERO);
        let total_amount = total_before_balance;

        // —— 预售（定金/尾款展示）——
        let (presale_json, deposit_amount, final_payment_amount) =
            self.presale_info(&products, &shop_groups).await?;

        // —— 改价标记 ——
        let has_change_price = carts.iter().any(|c| {
            c.change_price_data
                .as_deref()
                .map(|s| !s.trim().is_empty() && s.trim() != "null")
                .unwrap_or(false)
        });

        // —— 组装店铺购物车 ——
        let mut cart_list: Vec<CheckCartByStoreVO> = Vec::new();
        for (shop_id, lines) in &shop_groups {
            let mut carts_vo: Vec<CheckCartVO> = Vec::new();
            for cart in lines {
                let product = products.get(&cart.product_id);
                let sku = super::cart::cart_line_sku(&skus, cart);
                let base_price = super::cart::cart_base_price(product, sku, cart);
                let price = pricing.prices[&cart.cart_id] + super::cart::cart_extra_price(cart);
                let qty = cart.quantity.max(1);
                carts_vo.push(CheckCartVO {
                    cart_id: cart.cart_id,
                    user_id: cart.user_id,
                    product_id: cart.product_id,
                    product_sn: cart.product_sn.clone(),
                    seo_link: product.and_then(|p| p.seo_identification.clone()),
                    pic_thumb: cart
                        .pic_thumb
                        .clone()
                        .or_else(|| product.map(|p| p.pic_thumb.clone()))
                        .unwrap_or_default(),
                    market_price: cart.market_price.or_else(|| product.map(|p| p.market_price)),
                    original_price: base_price,
                    quantity: qty,
                    sku_id: cart.sku_id.unwrap_or(0),
                    sku_data: parse_sku_data(sku.and_then(|s| s.sku_data.as_deref()).or(cart.sku_data.as_deref())),
                    extra_sku_data: parse_sku_data(cart.extra_sku_data.as_deref()),
                    product_type: product.and_then(|p| p.product_type).unwrap_or(cart.product_type),
                    is_checked: cart.is_checked(),
                    shop_id: cart.shop_id,
                    r#type: cart.r#type,
                    update_time: cart.update_time,
                    product_name: product.map(|p| p.product_name.clone()).unwrap_or_default(),
                    product_price: base_price,
                    category_id: product.map(|p| p.category_id).unwrap_or(0),
                    brand_id: product.map(|p| p.brand_id).unwrap_or(0),
                    product_stock: product.map(|p| p.product_stock).unwrap_or(0),
                    price,
                    stock: super::cart::cart_line_stock(product, sku, cart),
                    has_sku: cart.sku_id.unwrap_or(0) > 0,
                    subtotal: price * Decimal::from(qty),
                    origin_price: base_price,
                    is_disabled: product.map(|p| p.product_status != 1).unwrap_or(false),
                });
            }
            let store_coupon_amount = coupons.per_store.get(shop_id).copied().unwrap_or_default();
            let store_seckill_amount = pricing.seckill.get(shop_id).copied().unwrap_or_default();
            let store_groupon_amount = pricing.groupon.get(shop_id).copied().unwrap_or_default();
            let store_full_reduce = pricing.full_reduce.get(shop_id).copied().unwrap_or_default();
            let store_time_amount = pricing.time.get(shop_id).copied().unwrap_or_default();
            cart_list.push(CheckCartByStoreVO {
                shop_id: *shop_id,
                shop_title: if *shop_id == 0 {
                    "平台自营".to_string()
                } else {
                    format!("店铺 {shop_id}")
                },
                no_shipping: 0,
                min_order_amount_diff: "0".to_string(),
                is_not_reach_min_order_amount: false,
                carts: carts_vo,
                total: CartStoreTotalVO {
                    coupon_ids: coupons.valid_selected.clone(),
                    discount_coupon_amount: store_coupon_amount,
                    discount_seckill_amount: store_seckill_amount,
                    discount_product_promotion_amount: store_groupon_amount,
                    discount_discount_amount: store_full_reduce,
                    discount_time_discount_amount: store_time_amount,
                    discounts: store_coupon_amount
                        + store_seckill_amount
                        + store_groupon_amount
                        + store_full_reduce
                        + store_time_amount,
                    ..Default::default()
                },
                used_promotions: Vec::new(),
                enable_use_promotion: Vec::new(),
                gift: pricing.gifts
                    .iter()
                    .filter(|g| g.shop_id == *shop_id)
                    .map(|g| {
                        serde_json::json!({
                            "giftId": g.gift_id,
                            "productId": g.product_id,
                            "skuId": g.sku_id,
                            "productName": g.product_name,
                            "productSn": g.product_sn,
                            "picThumb": g.pic_thumb,
                            "productType": g.product_type,
                            "shopId": g.shop_id,
                            "type": 4,
                            "num": g.num,
                            "promotionId": g.promotion_id,
                        })
                    })
                    .collect(),
            });
        }

        let total = CheckTotalVO {
            product_amount,
            checked_count: total_quantity,
            discounts: coupons.total
                + seckill_total
                + groupon_total
                + full_reduce_total
                + time_discount_total,
            discount_after: Some((product_amount - coupons.total).max(Decimal::ZERO).to_string()),
            total_count: total_quantity,
            discount_coupon_amount: coupons.total,
            coupon_amount: coupons.total,
            discount_seckill_amount: seckill_total,
            discount_product_promotion_amount: groupon_total,
            discount_discount_amount: full_reduce_total,
            discount_time_discount_amount: time_discount_total,
            discount_amount: seckill_total + groupon_total + full_reduce_total + time_discount_total,
            shipping_fee: shipping.total,
            store_shipping_fee: shipping.per_store,
            total_amount,
            tax_rate,
            tax_rate_amount,
            gift_card_amount,
            balance: balance_used,
            unpaid_amount,
            credit_amount,
            points_amount: points.points_amount,
            exchange_points,
            deposit_amount,
            final_payment_amount,
            actual_pay_deposit_amount: deposit_amount,
            ..Default::default()
        };

        let shipping_type_value = param
            .shipping_type
            .clone()
            .unwrap_or_else(|| serde_json::json!({}));

        Ok(OrderCheckVO {
            cart_list,
            total,
            has_change_price,
            change_price_type: 0,
            price_change_type: 0,
            cart_change_price_type: 0,
            available_points: points.available_points,
            coupon_list: CouponListVO {
                enable_coupons: coupons.enable,
                disable_coupons: coupons.disable,
            },
            item: CheckItemVO {
                address_id,
                shipping_type: shipping_type_value,
                pay_type_id,
                use_point: points.used_points,
                use_balance: balance_used,
                use_credit: param.use_credit.unwrap_or(0),
                use_coupon_ids: Some(coupons.valid_selected.clone()),
                select_user_coupon_ids: Some(coupons.valid_selected),
                flow_type,
                delivery_type: param.delivery_type,
                use_gift_card_id: selected_gift_card_id,
                use_gift_card_sn: selected_gift_card_sn,
            },
            use_coupon_ids: param.use_coupon_ids.clone(),
            select_user_coupon_ids: param.select_user_coupon_ids.clone(),
            tmpl_ids: Vec::new(),
            flow_type,
            message: None,
            delivery_option: Some(
                self.build_delivery_option(
                    &carts,
                    products.values().collect::<Vec<_>>(),
                    address_id,
                    param.delivery_type,
                )
                .await?,
            ),
            product_presale: presale_json,
            credit_payment_info: credit_info,
            coupon_exclude_amount: "0".to_string(),
            points_exclude_amount: "0".to_string(),
            available_gift_cards,
        })
    }

    /// Java `ProductServiceImpl#getDeliveryOption`：按商品配送能力分组
    /// （商城配送 / 到店自提 / 同城配送），驱动 H5 确认订单页的配送区块与
    /// 收货地址编辑区。此前恒 `None`，H5 三个配送开关全灭导致地址区不渲染。
    ///
    /// 已声明差异：自提点按 `store_parent_id + status=1` 查询（Java 在带
    /// 经纬度时走 `pickupByLocation` 地理过滤）；平台自营门店坐标未配置时
    /// 距离记 0（与 Java 空坐标路径一致）。
    async fn build_delivery_option(
        &self,
        carts: &[Cart],
        products: Vec<&Product>,
        address_id: i32,
        delivery_type: Option<i32>,
    ) -> Result<serde_json::Value> {
        use serde_json::json;

        let product_json =
            |p: &Product| serde_json::to_value(p).unwrap_or(serde_json::Value::Null);

        if carts.is_empty() {
            return Ok(json!({}));
        }

        let address = if address_id > 0 {
            self.repos
                .member
                .user_address_client
                .find_by_id(address_id)
                .await?
        } else {
            None
        };
        let addr_lng = address.as_ref().and_then(|a| a.longitude);
        let addr_lat = address.as_ref().and_then(|a| a.latitude);

        let mut delivery_products: Vec<serde_json::Value> = Vec::new();
        let mut shop_delivery_products: Vec<serde_json::Value> = Vec::new();
        let mut no_shipping_products: Vec<serde_json::Value> = Vec::new();
        let mut pickup_groups: std::collections::BTreeMap<i32, (Vec<serde_json::Value>, Vec<serde_json::Value>)> =
            std::collections::BTreeMap::new();
        let mut shop_delivery_data: Vec<serde_json::Value> = Vec::new();
        let total = carts.len();

        for cart in carts {
            let Some(product) = products.iter().copied().find(|p| p.product_id == cart.product_id) else {
                continue;
            };
            let shop_id = cart.shop_id;

            let can_delivery = product.is_logistics == 1;
            let is_no_shipping = product.no_shipping.unwrap_or(0) == 1;
            let mut can_pickup = product.is_shop_pickup == 1;
            let mut is_shop_delivery = product.is_shop_delivery == 1;

            // 同城配送门：店铺开通自建同城配送（第三方运力切换会置 0）
            match self.repos.o2o.config.find_local_delivery(shop_id).await? {
                Some(cfg) if cfg.enabled == 1 => {
                    if cfg.distance_pricing_enabled == 0 {
                        let mut distance = Decimal::ZERO;
                        if let (Some(lng), Some(lat)) = (addr_lng, addr_lat) {
                            if shop_id > 0 {
                                let shop_coord: Option<(Option<Decimal>, Option<Decimal>)> =
                                    sqlx::query_as(
                                        "SELECT shop_longitude, shop_latitude FROM shop WHERE shop_id = $1",
                                    )
                                    .bind(shop_id)
                                    .fetch_optional(&self.repos.pool)
                                    .await
                                    .map_err(|e| Error::Database(e.to_string()))?;
                                if let Some((Some(sl), Some(sa))) = shop_coord {
                                    distance =
                                        crate::trade::proxy_shopping_product::haversine_meters(
                                            Some(lng),
                                            Some(lat),
                                            Some(sl),
                                            Some(sa),
                                        );
                                }
                            }
                        }
                        let out_of_range = distance > cfg.delivery_radius;
                        shop_delivery_data.push(json!({
                            "shopId": shop_id,
                            "distance": distance,
                            "isOutOfRange": out_of_range,
                        }));
                    }
                }
                _ => is_shop_delivery = false,
            }

            // 自提门：店铺自提开关开启
            let pickup_cfg = self
                .repos
                .o2o
                .config
                .find_pickup_config(i64::from(shop_id))
                .await?;
            if pickup_cfg.map(|c| c.status != 1).unwrap_or(true) {
                can_pickup = false;
            }

            // 无需物流（非卡券）：整单按配送处理提前返回
            if is_no_shipping && product.product_type != Some(1) {
                no_shipping_products.push(product_json(product));
                return Ok(json!({
                    "deliveryType": 3,
                    "deliveryProducts": no_shipping_products,
                    "pickupProducts": [],
                    "shopDeliveryProducts": [],
                    "shopDeliveryData": [],
                }));
            }

            if is_shop_delivery {
                shop_delivery_products.push(product_json(product));
            }
            if can_delivery {
                delivery_products.push(product_json(product));
            }
            if can_pickup {
                let shop: Option<nextrade_core::entities::Shop> =
                    sqlx::query_as::<_, nextrade_core::entities::Shop>(
                    "SELECT * FROM shop WHERE store_parent_id = $1 AND status = 1 ORDER BY shop_id LIMIT 1",
                )
                .bind(shop_id)
                .fetch_optional(&self.repos.pool)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
                if let Some(shop) = shop {
                    let entry = pickup_groups.entry(shop_id).or_default();
                    entry
                        .0
                        .push(serde_json::to_value(&shop).unwrap_or(serde_json::Value::Null));
                    entry.1.push(product_json(product));
                }
            }
        }

        let pickup_count: usize = pickup_groups.values().map(|(_, ps)| ps.len()).sum();
        let delivery_count = delivery_products.len();
        let shop_delivery_count = shop_delivery_products.len();

        // Java ShopDeliveryTypeEnum 决策矩阵（1 全可选 / 2 部分可选 /
        // 3 配送 / 4 自提 / 5 同城）
        let dt = if (pickup_count == total
            && delivery_count == total
            && shop_delivery_count == total)
            || (pickup_count == 0 && delivery_count == total && shop_delivery_count == total)
            || (pickup_count == total && delivery_count == 0 && shop_delivery_count == total)
            || (pickup_count == total && delivery_count == total && shop_delivery_count == 0)
        {
            1
        } else if pickup_count == total && delivery_count == 0 && shop_delivery_count == 0 {
            4
        } else if shop_delivery_count == total && pickup_count == 0 && delivery_count == 0 {
            5
        } else if delivery_count == total && pickup_count == 0 && shop_delivery_count == 0 {
            3
        } else {
            2
        };

        let pickup_products: Vec<serde_json::Value> = pickup_groups
            .into_iter()
            .map(|(shop_id, (pickups, products))| {
                json!({"shopId": shop_id, "pickups": pickups, "products": products})
            })
            .collect();

        let assemble =
            |dp: Vec<serde_json::Value>, pp: Vec<serde_json::Value>, sp: Vec<serde_json::Value>| {
                json!({
                    "deliveryType": dt,
                    "deliveryProducts": dp,
                    "pickupProducts": pp,
                    "shopDeliveryProducts": sp,
                    "shopDeliveryData": shop_delivery_data,
                })
            };

        // 指定配送方式时只回该组（Java 同款）
        if let Some(t) = delivery_type {
            return Ok(match t {
                1 => assemble(delivery_products, Vec::new(), Vec::new()),
                2 => assemble(Vec::new(), pickup_products, Vec::new()),
                3 => assemble(Vec::new(), Vec::new(), shop_delivery_products),
                _ => assemble(delivery_products, pickup_products, shop_delivery_products),
            });
        }
        Ok(assemble(
            delivery_products,
            pickup_products,
            shop_delivery_products,
        ))
    }

    /// 载入卡组适用范围（我方新增）：品类范围按 category 全量父子对展开子类目。
    /// 读取失败/卡组不存在时回落全部商品（不因范围读取失败阻断结算）。
    async fn load_gift_card_scope(&self, card_groups_id: i32) -> GiftCardScope {
        let Ok(Some((scope_type, product_ids, category_ids))) = self
            .repos
            .marketing
            .gift_card_groups
            .find_scope(card_groups_id)
            .await
        else {
            return GiftCardScope::default();
        };
        if scope_type == SCOPE_TYPE_ALL {
            return GiftCardScope::default();
        }
        let pairs = self
            .repos
            .marketing
            .gift_card_groups
            .list_category_pairs()
            .await
            .unwrap_or_default();
        GiftCardScope::from_parts(scope_type, product_ids, category_ids, &pairs)
    }

    /// 地址地区 ID（用于运费模板地区匹配；无地址返回空）。
    async fn address_region_ids(&self, address_id: i32) -> Result<Vec<i32>> {
        if address_id <= 0 {
            return Ok(Vec::new());
        }
        let address: Option<UserAddress> = self
            .repos
            .member
            .user_address_client
            .find_by_id(address_id)
            .await?;
        Ok(address
            .and_then(|a| serde_json::from_str::<Vec<i32>>(&a.region_ids).ok())
            .unwrap_or_default())
    }

    /// 优惠券试算：可用性（店铺范围/门槛）+ 多店分摊。
    async fn calc_coupons(
        &self,
        usable: &[UserCouponVO],
        selected: &[i32],
        store_amounts: &BTreeMap<i32, Decimal>,
        product_amount: Decimal,
    ) -> Result<CouponCalc> {
        let ids: Vec<i32> = usable.iter().map(|u| u.coupon_id).collect();
        let entities: HashMap<i32, Coupon> = self
            .repos
            .marketing
            .coupon
            .find_by_ids(&ids)
            .await?
            .into_iter()
            .map(|c| (c.coupon_id, c))
            .collect();

        let mut enable = Vec::new();
        let mut disable = Vec::new();
        let mut selected_discounts: Vec<(i32, Decimal, i32)> = Vec::new();
        let mut total = Decimal::ZERO;
        let mut per_store: HashMap<i32, Decimal> = HashMap::new();
        let mut valid_selected = Vec::new();

        for uc in usable {
            let Some(coupon) = entities.get(&uc.coupon_id) else {
                continue;
            };
            let shop_id = coupon.shop_id.unwrap_or(0);
            let base = if shop_id > 0 {
                store_amounts.get(&shop_id).copied().unwrap_or_default()
            } else {
                product_amount
            };
            let in_scope = shop_id == 0 || store_amounts.contains_key(&shop_id);
            let meets_min = base >= coupon.min_order_amount && base > Decimal::ZERO;
            let eligible = in_scope && meets_min && coupon.is_delete != Some(1);
            let vo = map_coupon(uc, coupon, eligible);
            let is_selected = selected.contains(&uc.id);
            if eligible {
                enable.push(CheckCouponVO {
                    selected: is_selected,
                    ..vo
                });
                if is_selected {
                    let discount = coupon_rule_discount(
                        coupon.coupon_type,
                        coupon.coupon_money,
                        coupon.coupon_discount,
                        coupon.max_order_amount,
                        base,
                    );
                    if discount > Decimal::ZERO {
                        total += discount;
                        selected_discounts.push((uc.id, discount, shop_id));
                        valid_selected.push(uc.id);
                    }
                }
            } else {
                let reason = if !in_scope {
                    "不适用于当前店铺商品"
                } else {
                    "未达到使用门槛"
                };
                let mut value = serde_json::to_value(&vo).unwrap_or_default();
                if let Some(obj) = value.as_object_mut() {
                    obj.insert("disableReason".into(), serde_json::json!(reason));
                }
                disable.push(value);
            }
        }

        // 分摊：店铺券归店铺；全局券按店铺商品金额比例
        for (_, discount, shop_id) in &selected_discounts {
            if *shop_id > 0 {
                *per_store.entry(*shop_id).or_default() += *discount;
            } else {
                for (sid, amount) in allocate_discount(*discount, store_amounts) {
                    *per_store.entry(sid).or_default() += amount;
                }
            }
        }
        // 舍入：保证 per_store 合计等于 total
        let allocated_total: Decimal = per_store.values().copied().sum();
        if allocated_total != total {
            if let Some((shop_id, _)) = per_store.iter().next().map(|(k, v)| (*k, *v)) {
                *per_store.entry(shop_id).or_default() += total - allocated_total;
            }
        }
        Ok(CouponCalc {
            total,
            per_store,
            valid_selected,
            enable,
            disable,
        })
    }

    /// 账期支付信息（Java `creditUserService.getUserCreditInfo`）。
    async fn credit_payment_info(
        &self,
        user_id: i32,
        order_amount: Decimal,
    ) -> Option<serde_json::Value> {
        let row = self
            .repos
            .payment
            .credit_user
            .find_by_user_id(user_id)
            .await
            .ok()
            .flatten()?;
        let enabled = row.status == 1;
        let available = row.available_amount.max(Decimal::ZERO);
        let can_use = enabled && available >= order_amount && order_amount > Decimal::ZERO;
        let settlement_days = if row.settlement_type == 2 { 90 } else { 30 };
        let settlement_text = if row.settlement_type == 2 {
            "季结90天"
        } else {
            "月结30天"
        };
        let disable_reason = if !enabled {
            "授信已禁用"
        } else if available < order_amount {
            "可用额度不足"
        } else {
            ""
        };
        Some(serde_json::json!({
            "creditUserId": row.credit_user_id,
            "creditLimit": row.credit_limit.to_string(),
            "usedAmount": row.used_amount.to_string(),
            "availableAmount": available.to_string(),
            "settlementType": row.settlement_type,
            "settlementTypeText": settlement_text,
            "settlementDays": settlement_days,
            "canUseCredit": can_use,
            "disableReason": disable_reason,
            "orderAmount": order_amount.to_string(),
        }))
    }

    /// 预售信息（`product_presale`）：返回 `(productPresale, deposit, finalPayment)`。
    async fn presale_info(
        &self,
        products: &HashMap<i32, Product>,
        shop_groups: &BTreeMap<i32, Vec<&Cart>>,
    ) -> Result<(Option<serde_json::Value>, Option<Decimal>, Option<Decimal>)> {
        for (_, lines) in shop_groups {
            for cart in lines {
                let Some(product) = products.get(&cart.product_id) else {
                    continue;
                };
                if product.sale_mode <= 0 {
                    continue;
                }
                let Some(presale) = self
                    .repos
                    .catalog
                    .product_manage
                    .find_presale(cart.product_id)
                    .await?
                else {
                    continue;
                };
                let line_amount =
                    product.product_price * Decimal::from(cart.quantity.max(1));
                let mut deposit = Decimal::ZERO;
                let mut final_payment = Decimal::ZERO;
                if presale.sale_mode == 2 || product.sale_mode == 2 {
                    deposit = match presale.deposit_type {
                        Some(1) => {
                            let ratio = Decimal::from(presale.deposit_ratio.unwrap_or(0));
                            let mut amount = line_amount * ratio / Decimal::ONE_HUNDRED;
                            if presale.is_round_down == Some(1) {
                                amount = amount.trunc();
                            }
                            amount
                        }
                        Some(2) => presale.deposit_fixed_amount.unwrap_or_default(),
                        _ => Decimal::ZERO,
                    }
                    .max(Decimal::ZERO)
                    .min(line_amount);
                    final_payment = (line_amount - deposit).max(Decimal::ZERO);
                }
                let value = serde_json::json!({
                    "presaleId": presale.presale_id,
                    "productId": presale.product_id,
                    "saleMode": presale.sale_mode,
                    "depositType": presale.deposit_type,
                    "depositRatio": presale.deposit_ratio,
                    "depositFixedAmount": presale.deposit_fixed_amount.map(|v| v.to_string()),
                    "isRoundDown": presale.is_round_down,
                    "deliveryType": presale.delivery_type,
                    "depositAmount": deposit.to_string(),
                    "finalPaymentAmount": final_payment.to_string(),
                    "productAmount": line_amount.to_string(),
                });
                return Ok((
                    Some(value),
                    if deposit > Decimal::ZERO { Some(deposit) } else { None },
                    if final_payment > Decimal::ZERO {
                        Some(final_payment)
                    } else {
                        None
                    },
                ));
            }
        }
        Ok((None, None, None))
    }
}

// ——————————————————————————————————————————————
// 共享：商品加载 / 运费计算（下单复用，保证口径一致）
// ——————————————————————————————————————————————

/// 秒杀活动价映射：`(product_id, sku_id)`；`sku_id=0` 表示商品级活动。
pub(crate) type SeckillPrices = HashMap<(i32, i32), SeckillItem>;

/// 批量加载商品当前进行中的秒杀明细（结算/下单共用）。
pub(crate) async fn load_seckill_prices(
    repos: &Repositories,
    lines: &[Cart],
    now: i32,
) -> Result<SeckillPrices> {
    let mut ids: Vec<i32> = Vec::new();
    for line in lines {
        if !ids.contains(&line.product_id) {
            ids.push(line.product_id);
        }
    }
    let items = repos
        .marketing
        .seckill
        .find_active_by_products(&ids, now)
        .await?;
    let mut map = HashMap::new();
    for item in items {
        let sku = item.sku_id.unwrap_or(0);
        map.entry((item.product_id.unwrap_or(0), sku)).or_insert(item);
    }
    Ok(map)
}

/// 拼团活动价（结算/下单共用）。
#[derive(Debug, Clone)]
pub(crate) struct GrouponPrice {
    pub data: GrouponCartData,
    pub price: Decimal,
    pub promotion: GrouponPromotion,
    pub item: GrouponPromotionItem,
    pub min_group_num: i32,
}

/// 批量加载购物车行上的拼团活动价：按 `(活动, 商品, SKU)` 校验并匹配阶梯价。
pub(crate) async fn load_groupon_prices(
    repos: &Repositories,
    carts: &[Cart],
    now: i64,
) -> Result<HashMap<i32, GrouponPrice>> {
    let mut result = HashMap::new();
    for cart in carts {
        let Some(data) = parse_groupon_data(cart.extra_data.as_deref()) else {
            continue;
        };
        let promotion_id = data.groupon_promotion_id.unwrap_or(0);
        let item_id = data.groupon_promotion_item_id.unwrap_or(0);
        if promotion_id <= 0 || item_id <= 0 {
            continue;
        }
        let Some(promotion) = repos
            .marketing
            .groupon_promotion
            .find_promotion(promotion_id)
            .await?
        else {
            continue;
        };
        if now < promotion.start_time || now > promotion.end_time {
            continue;
        }
        let Some(item) = repos
            .marketing
            .groupon_promotion_item
            .find_item(item_id)
            .await?
        else {
            continue;
        };
        if item.groupon_promotion_id != promotion_id
            || item.product_id != i64::from(cart.product_id)
        {
            continue;
        }
        let details = repos
            .marketing
            .groupon_promotion_item
            .find_details_by_item(item_id)
            .await?;
        let detail = details
            .iter()
            .find(|d| d.sku_id == i64::from(cart.sku_id.unwrap_or(0)))
            .or_else(|| details.first());
        let Some(detail) = detail else {
            continue;
        };
        let price = groupon_price_for(&detail.group_price, data.group_num_key)
            .or(data.price)
            .unwrap_or_default();
        if price <= Decimal::ZERO {
            continue;
        }
        let min_group_num = {
            let num = groupon_min_group_num(&item.group_num, data.group_num_key);
            if num > 0 {
                num
            } else {
                item.min_group_num
            }
        };
        result.insert(
            cart.cart_id,
            GrouponPrice {
                data,
                price,
                promotion,
                item,
                min_group_num,
            },
        );
    }
    Ok(result)
}

/// 购物车和结算共享活动单价及店铺优惠，运费和支付抵扣在结算阶段处理。
#[derive(Default)]
pub(crate) struct CartActivityPricing {
    pub prices: HashMap<i32, Decimal>,
    pub seckill: BTreeMap<i32, Decimal>,
    pub groupon: BTreeMap<i32, Decimal>,
    pub time: BTreeMap<i32, Decimal>,
    pub full_reduce: BTreeMap<i32, Decimal>,
    pub gifts: Vec<AppliedGift>,
}

pub(crate) async fn price_cart_activities(
    repos: &Arc<Repositories>,
    user_id: i32,
    carts: &[Cart],
    products: &HashMap<i32, Product>,
    skus: &HashMap<i32, nextrade_core::entities::ProductSku>,
    now: i32,
    line_price_overrides: Option<&HashMap<i32, Decimal>>,
) -> Result<CartActivityPricing> {
    let mut result = CartActivityPricing::default();
    if carts.is_empty() {
        return Ok(result);
    }
    let seckill = load_seckill_prices(repos, carts, now).await?;
    let groupon = load_groupon_prices(repos, carts, i64::from(now)).await?;
    let time = crate::marketing::time_discount::TimeDiscountService::new(repos.clone());
    for cart in carts {
        // 代客跨店履约行（我方新增）：履约门店价即最终价，不叠加任何
        // 活动价/满减，优惠分桶记 0。overrides 为空时不进此分支。
        if let Some(overrides) = line_price_overrides {
            if let Some(override_price) = overrides.get(&cart.cart_id) {
                result.prices.insert(cart.cart_id, *override_price);
                continue;
            }
        }
        let sku = super::cart::cart_line_sku(skus, cart);
        let base_price = super::cart::cart_base_price(products.get(&cart.product_id), sku, cart);
        let seckill_item = seckill
            .get(&(cart.product_id, cart.sku_id.unwrap_or(0)))
            .or_else(|| seckill.get(&(cart.product_id, 0)));
        let groupon_item = groupon.get(&cart.cart_id);
        let activity_price = groupon_item
            .map(|g| g.price)
            .or_else(|| seckill_item.and_then(|item| item.seckill_price))
            .unwrap_or(base_price)
            .min(base_price);
        let final_price = time
            .current_discount_price(cart.product_id, cart.sku_id, activity_price)
            .await?
            .unwrap_or(activity_price)
            .min(activity_price);
        let quantity = Decimal::from(cart.quantity.max(1));
        let activity_discount = (base_price - activity_price) * quantity;
        let by_shop = if groupon_item.is_some() {
            &mut result.groupon
        } else {
            &mut result.seckill
        };
        *by_shop.entry(cart.shop_id).or_default() += activity_discount;
        *result.time.entry(cart.shop_id).or_default() += (activity_price - final_price) * quantity;
        result.prices.insert(cart.cart_id, final_price);
    }
    let rank_id = repos
        .member
        .user
        .find_by_id(user_id)
        .await?
        .map(|u| u.rank_id);
    // 跨店履约行不参与满减/满赠（履约价即最终价）：从满减入参中剔除。
    let fr_carts: Vec<Cart>;
    let fr_input: &[Cart] = match line_price_overrides {
        Some(overrides) if !overrides.is_empty() => {
            fr_carts = carts
                .iter()
                .filter(|c| !overrides.contains_key(&c.cart_id))
                .cloned()
                .collect();
            &fr_carts
        }
        _ => carts,
    };
    let (_, full_reduce, gifts) =
        apply_full_reductions(repos, fr_input, products, &mut result.prices, now, rank_id).await?;
    result.full_reduce = full_reduce;
    result.gifts = gifts;
    Ok(result)
}

/// 满赠命中赠品（type=3）。
#[derive(Debug, Clone, Default)]
pub struct AppliedGift {
    pub gift_id: i32,
    pub product_id: i32,
    pub sku_id: i32,
    pub shop_id: i32,
    pub num: i32,
    pub product_name: String,
    pub product_sn: String,
    pub pic_thumb: String,
    pub product_type: i32,
    pub promotion_id: i32,
}

/// 满减/满折/满赠：按店铺命中活动，对命中商品行按比例折价（Java `ProductPromotionServiceImpl#parsePrice`）。
///
/// 返回 `(总优惠, 各店铺优惠, 命中赠品)`；`line_prices` 会被就地更新为折后单价。
/// 活动会员等级白名单：空数组/空串表示不限等级。
fn promotion_rank_allowed(raw: &str, rank_id: Option<i32>) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return true;
    };
    let Some(list) = value.as_array() else {
        return true;
    };
    if list.is_empty() {
        return true;
    }
    let Some(rank) = rank_id else {
        return false;
    };
    list.iter()
        .filter_map(|v| v.as_i64())
        .any(|v| i32::try_from(v).ok() == Some(rank))
}

pub(crate) async fn apply_full_reductions(
    repos: &Repositories,
    carts: &[Cart],
    products: &HashMap<i32, Product>,
    line_prices: &mut HashMap<i32, Decimal>,
    now: i32,
    user_rank_id: Option<i32>,
) -> Result<(Decimal, BTreeMap<i32, Decimal>, Vec<AppliedGift>)> {
    let mut total = Decimal::ZERO;
    let mut per_store: BTreeMap<i32, Decimal> = BTreeMap::new();
    let mut gifts: Vec<AppliedGift> = Vec::new();
    let mut by_shop: BTreeMap<i32, Vec<&Cart>> = BTreeMap::new();
    for cart in carts {
        by_shop.entry(cart.shop_id).or_default().push(cart);
    }
    for (shop_id, lines) in by_shop {
        let promotions = repos
            .marketing
            .product_promotion
            .find_active_by_shop(shop_id, now)
            .await?;
        for promotion in promotions {
            if !promotion_rank_allowed(&promotion.limit_user_rank, user_rank_id) {
                continue;
            }
            if promotion.promotion_type != 1
                && promotion.promotion_type != 2
                && promotion.promotion_type != 3
            {
                continue;
            }
            let include_ids: Vec<i32> = if promotion.range == 3 || promotion.range == 4 {
                serde_json::from_str(&promotion.range_data).unwrap_or_default()
            } else {
                Vec::new()
            };
            let eligible: Vec<&Cart> = lines
                .iter()
                .copied()
                .filter(|cart| match promotion.range {
                    3 => include_ids.contains(&cart.product_id),
                    4 => !include_ids.contains(&cart.product_id),
                    _ => true,
                })
                .collect();
            if eligible.is_empty() {
                continue;
            }
            let mut compare_data = Decimal::ZERO;
            let mut price_amount = Decimal::ZERO;
            for cart in &eligible {
                let price = line_prices
                    .get(&cart.cart_id)
                    .copied()
                    .unwrap_or_else(|| product_price(products, cart));
                let qty = cart.quantity.max(1);
                price_amount += price * Decimal::from(qty);
                if promotion.unit == 1 {
                    compare_data += price * Decimal::from(qty);
                } else {
                    compare_data += Decimal::from(qty);
                }
            }
            if compare_data < promotion.min_order_amount {
                continue;
            }
            let tiers = parse_promotion_tiers(&promotion.promotion_type_data);
            let Some(tier) = pick_promotion_tier(&tiers, compare_data) else {
                continue;
            };
            // 满赠：不参与价格计算，命中后记录赠品数量
            if promotion.promotion_type == 3 {
                let Some(gift_id) = tier.gift_id else {
                    continue;
                };
                let Some(gift) = repos.marketing.product_gift.find_by_id(gift_id).await? else {
                    continue;
                };
                if gift.gift_stock <= 0 {
                    continue;
                }
                if gift.sku_id > 0 {
                    match repos.marketing.product_gift.find_sku(gift.sku_id).await? {
                        Some(sku) if sku.is_enabled != 0 => {}
                        _ => continue,
                    }
                }
                let Some(product) = repos
                    .marketing
                    .product_gift
                    .find_product(gift.product_id)
                    .await?
                else {
                    continue;
                };
                let times = if promotion.rules_type == 1 {
                    Decimal::ONE
                } else {
                    (compare_data / tier.min_amount.max(Decimal::ONE))
                        .trunc()
                        .max(Decimal::ONE)
                };
                let num = (Decimal::from(tier.num.max(0)) * times)
                    .trunc()
                    .to_i32()
                    .unwrap_or(0)
                    .min(gift.gift_stock);
                if num <= 0 {
                    continue;
                }
                gifts.push(AppliedGift {
                    gift_id,
                    product_id: gift.product_id,
                    sku_id: gift.sku_id,
                    shop_id: promotion.shop_id,
                    num,
                    product_name: product.product_name,
                    product_sn: product.product_sn,
                    pic_thumb: product.pic_thumb,
                    product_type: 0,
                    promotion_id: promotion.promotion_id,
                });
                continue;
            }
            let Some((discount, proportion)) = promotion_discount(
                promotion.promotion_type,
                promotion.rules_type,
                tier,
                compare_data,
                price_amount,
            ) else {
                continue;
            };
            if proportion >= Decimal::ONE {
                continue;
            }
            for cart in &eligible {
                let old = line_prices
                    .get(&cart.cart_id)
                    .copied()
                    .unwrap_or_else(|| product_price(products, cart));
                let new_price = (old * proportion).round_dp(2);
                line_prices.insert(cart.cart_id, new_price);
                let line_discount =
                    (old - new_price).max(Decimal::ZERO) * Decimal::from(cart.quantity.max(1));
                *per_store.entry(shop_id).or_default() += line_discount;
            }
            total += discount;
            tracing::info!(
                "满减/满折命中: shop={shop_id} promotion={} type={} discount={discount}",
                promotion.promotion_id,
                promotion.promotion_type
            );
        }
    }
    // 以店铺行级优惠合计为准，避免比例取整误差与活动金额不一致
    let stored_total: Decimal = per_store.values().copied().sum();
    Ok((stored_total.max(Decimal::ZERO), per_store, gifts))
}

/// 批量加载商品（结算页/下单共用）。
pub(crate) async fn load_products(
    repos: &Repositories,
    ids: &[i32],
) -> Result<HashMap<i32, Product>> {
    Ok(repos
        .catalog
        .product
        .find_by_ids(ids)
        .await?
        .into_iter()
        .map(|p| (p.product_id, p))
        .collect())
}

/// 商品当前结算单价。
pub(crate) fn product_price(products: &HashMap<i32, Product>, cart: &Cart) -> Decimal {
    products
        .get(&cart.product_id)
        .map(|p| p.product_price)
        .unwrap_or(cart.original_price)
}

/// 计算运费（按店铺）：`shipping_tpl` + `shipping_tpl_info`。
///
/// 口径：包邮商品/包邮模板/满额包邮 = 0；按件或按重、首件首重 + 续件续重。
/// 地区规则：地址地区命中模板区域时用该区域，否则用默认区域（`is_default=1`）或首条。
pub(crate) async fn calculate_shipping(
    repos: &Repositories,
    carts: &[Cart],
    products: &HashMap<i32, Product>,
    region_ids: &[i32],
    delivery_type: Option<i32>,
) -> Result<ShippingCalc> {
    let mut per_store: HashMap<i32, Decimal> = HashMap::new();
    let mut total = Decimal::ZERO;
    // 自提/虚拟发货不收运费
    if delivery_type == Some(2) {
        for cart in carts {
            per_store.entry(cart.shop_id).or_insert(Decimal::ZERO);
        }
        return Ok(ShippingCalc { total, per_store });
    }

    let mut by_shop: BTreeMap<i32, Vec<&Cart>> = BTreeMap::new();
    for cart in carts {
        by_shop.entry(cart.shop_id).or_default().push(cart);
    }
    for (shop_id, lines) in by_shop {
        let shop_amount: Decimal = lines
            .iter()
            .map(|c| product_price(products, c) * Decimal::from(c.quantity.max(1)))
            .sum();
        // tpl_id -> (件数, 重量, 体积)
        let mut groups: BTreeMap<i32, (Decimal, Decimal, Decimal)> = BTreeMap::new();
        let mut missing_volume: BTreeMap<i32, String> = BTreeMap::new();
        for cart in &lines {
            let product = products.get(&cart.product_id);
            let free_shipping = product.map(|p| p.free_shipping == 1).unwrap_or(false);
            if free_shipping {
                continue;
            }
            let mut tpl_id = product.map(|p| p.shipping_tpl_id).unwrap_or(0);
            if tpl_id <= 0 {
                tpl_id = repos
                    .logistics
                    .shipping_tpl
                    .find_default_tpl_id(shop_id)
                    .await?
                    .unwrap_or(0);
            }
            if tpl_id <= 0 {
                continue;
            }
            let sku = match cart.sku_id {
                Some(sku_id) => repos.catalog.product_manage.find_sku(sku_id).await?,
                None => None,
            };
            let unit_weight = sku
                .as_ref()
                .and_then(|s| s.sku_weight)
                .or_else(|| product.map(|p| p.product_weight))
                .unwrap_or(Decimal::ZERO);
            let unit_volume = sku
                .as_ref()
                .and_then(|s| s.sku_volume)
                .or_else(|| product.and_then(|p| p.product_volume));
            let qty = Decimal::from(cart.quantity.max(1));
            let weight = unit_weight * qty;
            let volume = unit_volume
                .filter(|v| *v > Decimal::ZERO)
                .unwrap_or(Decimal::ZERO);
            if unit_volume.unwrap_or(Decimal::ZERO) <= Decimal::ZERO {
                if let Some(name) = product.map(|p| p.product_name.clone()) {
                    missing_volume.entry(tpl_id).or_insert(name);
                }
            }
            let entry = groups
                .entry(tpl_id)
                .or_insert((Decimal::ZERO, Decimal::ZERO, Decimal::ZERO));
            entry.0 += qty;
            entry.1 += weight;
            entry.2 += volume * qty;
        }

        let mut store_fee = Decimal::ZERO;
        for (tpl_id, (quantity, weight, volume)) in groups {
            let Some(tpl) = repos.logistics.shipping_tpl.find_tpl_by_id(tpl_id).await? else {
                continue;
            };
            if tpl.is_free == 1 {
                continue;
            }
            let infos = repos
                .logistics
                .shipping_tpl
                .find_infos_by_tpl(tpl_id)
                .await?;
            let Some(info) = select_shipping_info(&infos, region_ids) else {
                continue;
            };
            if tpl.pricing_type == 3 {
                if let Some(name) = missing_volume.get(&tpl_id) {
                    return Err(Error::biz(
                        400,
                        format!("商品【{name}】未配置有效体积，暂无法计算运费，请联系商家"),
                    ));
                }
                if info.start_number <= Decimal::ZERO || info.add_number <= Decimal::ZERO {
                    return Err(Error::biz(400, "体积运费模板的首体积和续体积必须大于0"));
                }
            }
            let rule = ShippingRule {
                pricing_type: tpl.pricing_type,
                is_free: info.is_free == 1 || tpl.is_free == 1,
                start_number: info.start_number,
                start_price: info.start_price,
                add_number: info.add_number,
                add_price: info.add_price,
                free_price: info.free_price,
            };
            store_fee += calc_shipping_fee(&rule, shop_amount, quantity, weight, volume);
        }
        per_store.insert(shop_id, store_fee);
        total += store_fee;
    }
    Ok(ShippingCalc { total, per_store })
}

/// 选择模板区域：优先命中地址地区，其次默认区域，最后首条。
fn select_shipping_info<'a>(
    infos: &'a [ShippingTplInfo],
    region_ids: &[i32],
) -> Option<&'a ShippingTplInfo> {
    if infos.is_empty() {
        return None;
    }
    if !region_ids.is_empty() {
        for info in infos {
            if region_match(info.region_data.as_deref(), region_ids) {
                return Some(info);
            }
        }
    }
    infos
        .iter()
        .find(|info| info.is_default == 1)
        .or_else(|| infos.first())
}

/// 地区是否命中（严格对齐 Java `OrderCheckServiceImpl#fetchRegion`）。
///
/// 算法规则：
/// 1. `region_ids` 为空 -> false；`region_data` 为空/解析失败 -> false
/// 2. 读取 `areaRegions`（或 `area_regions`），遍历每一条链路 `region`:
///    - 获取链路末级节点 `last`
///    - 若 `last == 1`（代表全国/全区域），直接命中返回 true
///    - 若 `region_ids.contains(last)`，命中返回 true
/// 3. 若所有区域都不匹配，返回 false。
pub fn region_match(region_data: Option<&str>, region_ids: &[i32]) -> bool {
    if region_ids.is_empty() {
        return false;
    }
    let Some(raw) = region_data else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return false;
    };
    let areas = value
        .get("areaRegions")
        .or_else(|| value.get("area_regions"))
        .and_then(serde_json::Value::as_array);
    let Some(areas) = areas else {
        return false;
    };
    for region in areas {
        if let Some(list) = region.as_array() {
            if let Some(last_val) = list.last() {
                if let Some(last_id) = last_val.as_i64() {
                    // last == 1 代表全国/全部地区
                    if last_id == 1 {
                        return true;
                    }
                    if region_ids.iter().any(|&r| i64::from(r) == last_id) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// 解析配置为 Decimal（缺失/非法回退默认值）。
fn decimal_config(
    cfg: &HashMap<String, String>,
    key: &str,
    default: Decimal,
) -> Decimal {
    cfg.get(key)
        .and_then(|v| v.trim().parse::<Decimal>().ok())
        .unwrap_or(default)
}

/// `UserCouponVO` + `Coupon` → 结算页优惠券 VO。
fn map_coupon(uc: &UserCouponVO, coupon: &Coupon, _eligible: bool) -> CheckCouponVO {
    let shop_id = coupon.shop_id.unwrap_or(0);
    CheckCouponVO {
        id: uc.id,
        coupon_id: uc.coupon_id,
        coupon_name: uc.coupon_name.clone(),
        coupon_type: uc.coupon_type,
        coupon_money: uc.coupon_money,
        min_order_amount: uc.min_order_amount,
        coupon_desc: uc.coupon_desc.clone(),
        is_global: shop_id == 0,
        coupon_discount: uc.coupon_discount,
        shop_id,
        end_date: format_epoch(uc.end_date),
        user_coupon_id: uc.id,
        selected: false,
    }
}

fn to_shipping_vo(item: &nextrade_core::entities::ShippingType) -> ShippingTypeVO {
    ShippingTypeVO {
        shipping_type_id: item.shipping_type_id,
        shipping_type_name: item.shipping_type_name.clone(),
        origin_shipping_type_name: item.shipping_type_name.clone(),
        shipping_default_id: Some(item.shipping_default_id),
        shipping_type_desc: Some(item.shipping_type_desc.clone()),
        shipping_time_desc: Some(item.shipping_time_desc.clone()),
        is_support_cod: Some(item.is_support_cod),
        sort_order: item.sort_order,
        shop_id: item.shop_id,
    }
}

fn default_shipping_types(shop_id: i32) -> Vec<ShippingTypeVO> {
    vec![ShippingTypeVO {
        shipping_type_id: 1,
        shipping_type_name: "快递发货".to_string(),
        origin_shipping_type_name: "快递发货".to_string(),
        shipping_default_id: Some(0),
        shipping_type_desc: Some("快递发货".to_string()),
        shipping_time_desc: None,
        is_support_cod: Some(0),
        sort_order: 0,
        shop_id,
    }]
}

fn format_epoch(ts: i32) -> String {
    if ts <= 0 {
        return String::new();
    }
    chrono::DateTime::from_timestamp(i64::from(ts), 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::trade::check::MSG_LOGIN_REQUIRED;

    #[test]
    fn empty_cart_message_is_explicit() {
        assert_eq!(MSG_CART_EMPTY, "请先选择要结算的商品");
        assert_eq!(MSG_LOGIN_REQUIRED, "请先登录");
    }

    #[test]
    fn shipping_fallback_has_default_type() {
        let list = default_shipping_types(7);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].shop_id, 7);
        assert_eq!(list[0].shipping_type_name, "快递发货");
    }

    #[test]
    fn epoch_formatting() {
        assert_eq!(format_epoch(0), "");
        assert_eq!(format_epoch(-1), "");
        assert!(format_epoch(1_700_000_000).starts_with("2023-"));
    }

    #[test]
    fn region_match_parses_both_key_styles() {
        let camel = r#"{"areaRegions":[[110000],[120000]]}"#;
        assert!(region_match(Some(camel), &[110000]));
        assert!(!region_match(Some(camel), &[130000]));
        let snake = r#"{"area_regions":[[110000,110100]]}"#;
        assert!(region_match(Some(snake), &[110100]));
        assert!(!region_match(None, &[110000]));
        assert!(!region_match(Some("not-json"), &[110000]));
    }

    #[test]
    fn config_decimal_parsing() {
        let mut cfg = HashMap::new();
        cfg.insert("integralScale".to_string(), "1".to_string());
        assert_eq!(
            decimal_config(&cfg, "integralScale", Decimal::ZERO),
            Decimal::ONE
        );
        assert_eq!(
            decimal_config(&cfg, "missing", Decimal::new(15, 1)),
            Decimal::new(15, 1)
        );
    }

    #[test]
    fn zero_shipping_for_pickup() {
        // 纯函数覆盖：自提场景在 calculate_shipping 中直接返回 0
        let rule = ShippingRule::default();
        assert_eq!(calc_shipping_fee(&rule, Decimal::ZERO, Decimal::ZERO, Decimal::ZERO, Decimal::ZERO), Decimal::ZERO);
    }

    fn _unused_product_type_anchor(_: Product, _: ShippingTpl) {}
}
