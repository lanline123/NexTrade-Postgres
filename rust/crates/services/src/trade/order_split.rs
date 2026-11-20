//! 跨店拆单服务（方案 A：全量照迁原版拆单模型）。
//!
//! 对应原版 tigshop v5.8.61 `OrderServiceImpl` 的拆单族：
//! - `splitOrderAtSubmit`（L1511）：提交时按 shop ×（vendor + 配送组）拆子单，
//!   父单保留为支付壳（isValid=1）；单组只置 `is_store_splited=1`。
//! - `createSplitOrder`（L1866）：子单全字段复制 + 金额按店铺明细行或
//!   `allocatedAmount` 分摊 + 税费独立计算 + paylog 同 paySn 复制。
//! - `adjustSplitOrderAmounts`（L4233）/ `adjustSplitOrderTaxAmounts`（L4360）：
//!   尾差与税差由最后一个子单吸收。
//! - `splitStoreOrder`（L1415）：支付时入口——已拆则激活子单
//!   （`activateChildOrdersOnPayment` L1610，父单 isValid=0）；未拆多组则
//!   此时拆单并**删除父单**；单组只置标志。
//! - `splitSalesmanOrdersForStoreSplit`（L4190）：分销佣金行随明细重映射，
//!   失败仅记日志、永不阻断拆单。
//!
//! 我方字段口径适配（与原版的差异集中登记于本线 REPORT §9）：
//! - 子单 `product_amount` 取 Σ price×qty（成交价）：我方父单 product_amount
//!   即成交价口径、discount_amount 仅记秒杀折扣且不参与 total 计算，
//!   与原版「originPrice 总额 − discountAmount」代数等价、总额恒等式一致。
//! - 子单 total 公式与我方 `OrderService::create` 完全一致：
//!   product − coupon + shipping + tax − points（钳制 ≥ 0）；
//!   paid = gift + balance + online + offline + cash − change + credit
//!   （与我方 apply_partial_payment / mark_paid 的字段语义一致）。
//! - 子单 pay_status：父单已付 → 2，未付 → 0（原版未付子单 UNPAID=0，
//!   读侧平台视角正是按 `pay_status <> 0` 排除未付子单）。
//! - 子单 paylog 按父单**全部** paylog 行逐行复制（我方余额与渠道是两行
//!   记账，原版为一行模型；逐行 resolve 口径与原版一致）。

use std::collections::BTreeMap;
use std::sync::Arc;

use nextrade_common::utils;
use nextrade_core::entities::{Order, OrderItem, Paylog};
use nextrade_core::Result;
use nextrade_infrastructure::db::repo::trade::order_admin::OrderPatch;
use nextrade_infrastructure::db::Repositories;
use rust_decimal::{Decimal, RoundingStrategy};

use crate::trade::order::status;

/// 礼品卡兑换订单类型（我方 `OrderService::create` 口径，
/// 对应原版 `CartTypeEnum.TYPE_GIFT_EXCHANGE`）。
pub const ORDER_TYPE_GIFT_EXCHANGE: i32 = 12;
/// 虚拟商品类型（原版 `PRODUCT_TYPE_VIRTUAL`）。
pub const PRODUCT_TYPE_VIRTUAL: i32 = 2;
/// 子单 pay_status：未支付（原版 `PaymentStatus.UNPAID`）。
const PAY_STATUS_UNPAID: i32 = 0;
/// 子单 pay_status：已支付（原版 `PaymentStatus.PAID`）。
const PAY_STATUS_PAID: i32 = 2;
/// paylog 已支付状态（paylog 枚举：1=已支付）。
const PAYLOG_STATUS_PAID: i32 = 1;

fn zero() -> Decimal {
    Decimal::ZERO
}

fn round2(v: Decimal) -> Decimal {
    v.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

/// 订单头供应商：明细循环逐行覆盖的最后一行 vendor
/// （原版结算 `defaultVendorId`，OrderCheckServiceImpl L907/L1127/L1355）。
#[must_use]
pub fn header_vendor_id(items: &[OrderItem]) -> Option<i32> {
    items.last().and_then(|i| i.vendor_id)
}

/// 配送分组（原版 `resolveSplitDeliveryGroup` L1596）：
/// 仅礼品卡兑换单的虚拟商品逐件单独成组（返回 1），其余返回 0。
#[must_use]
pub fn resolve_delivery_group(gift_exchange: bool, item: &OrderItem) -> i32 {
    if gift_exchange && item.product_type == PRODUCT_TYPE_VIRTUAL {
        1
    } else {
        0
    }
}

/// 一个拆分分组：同店铺、同供应商、同配送组的明细集合。
#[derive(Debug, Clone)]
pub struct SplitGroup {
    /// 店铺 id。
    pub shop_id: i32,
    /// 供应商 id（无供应商记 0）。
    pub vendor_id: i32,
    /// 配送组键：普通组 0；礼品卡兑换虚拟商品组为该明细 item_id（逐件成组）。
    pub delivery_group: i32,
    /// 组内明细（保持父单明细原序）。
    pub items: Vec<OrderItem>,
}

impl SplitGroup {
    /// 组商品金额（成交价口径：Σ price × qty，与我方父单 product_amount 同口径）。
    #[must_use]
    pub fn product_amount(&self) -> Decimal {
        self.items
            .iter()
            .map(|i| i.price * Decimal::from(i.quantity))
            .sum()
    }

    /// 组内最后一行明细的供应商（原版 createSplitOrder 循环覆盖语义）。
    #[must_use]
    pub fn last_item_vendor(&self) -> Option<i32> {
        self.items.last().and_then(|i| i.vendor_id)
    }

    /// 是否为「无需配送」组（礼品卡兑换虚拟商品组）。
    #[must_use]
    pub fn is_no_shipping(&self) -> bool {
        self.delivery_group != 0
    }
}

/// 按 shop ×（vendor + 配送组）稳定保序分组
/// （原版用 HashMap 无序；我方定序使尾差落点确定，语义等价）。
#[must_use]
pub fn group_items(items: &[OrderItem], gift_exchange: bool) -> Vec<SplitGroup> {
    let mut groups: Vec<SplitGroup> = Vec::new();
    for item in items {
        let delivery_group = if resolve_delivery_group(gift_exchange, item) == 1 {
            item.item_id
        } else {
            0
        };
        let key = (item.shop_id, item.vendor_id.unwrap_or(0), delivery_group);
        match groups
            .iter_mut()
            .find(|g| (g.shop_id, g.vendor_id, g.delivery_group) == key)
        {
            Some(g) => g.items.push(item.clone()),
            None => groups.push(SplitGroup {
                shop_id: key.0,
                vendor_id: key.1,
                delivery_group: key.2,
                items: vec![item.clone()],
            }),
        }
    }
    groups
}

/// 金额分摊（原版 `allocatedAmount` L5302）：
/// ratio = value / parent_base（10 位 HALF_UP）；share = child_base × ratio（2 位 HALF_UP）。
/// 原版调用点以 `value > 0` 守卫，我方内建零守卫。
#[must_use]
pub fn allocated_amount(value: Decimal, parent_base: Decimal, child_base: Decimal) -> Decimal {
    if value <= zero() || parent_base <= zero() || child_base <= zero() {
        return zero();
    }
    let ratio =
        (value / parent_base).round_dp_with_strategy(10, RoundingStrategy::MidpointAwayFromZero);
    round2(child_base * ratio)
}

/// 子单税费（原版 `refreshSplitOrderTaxAmount` L4417）：
/// 基数 = product + shipping − discount（钳制 ≥ 0）；税率按百分比语义
/// /100（8 位 HALF_UP）；金额 CEILING 2 位。
#[must_use]
pub fn child_tax_amount(
    product_amount: Decimal,
    shipping_fee: Decimal,
    discount_amount: Decimal,
    tax_rate: Option<Decimal>,
) -> Decimal {
    let rate = tax_rate.unwrap_or_default();
    if rate <= zero() {
        return zero();
    }
    let base = (product_amount + shipping_fee - discount_amount).max(zero());
    if base <= zero() {
        return zero();
    }
    let normalized = (rate / Decimal::from(100))
        .round_dp_with_strategy(8, RoundingStrategy::MidpointAwayFromZero);
    (base * normalized).round_dp_with_strategy(2, RoundingStrategy::ToPositiveInfinity)
}

/// 我方 total 公式（与 `OrderService::create` 完全一致）：
/// product − coupon + shipping + tax − points，钳制 ≥ 0。
#[must_use]
pub fn total_formula(
    product_amount: Decimal,
    coupon_amount: Decimal,
    shipping_fee: Decimal,
    tax_rate_amount: Decimal,
    points_amount: Decimal,
) -> Decimal {
    (product_amount - coupon_amount + shipping_fee + tax_rate_amount - points_amount).max(zero())
}

/// 按我方订单字段语义重算 total / paid / unpaid / unrefund
/// （等价原版 `updateOrderMoney` L2121 在我方口径下的形态：
/// total 公式同 create；paid 含礼品卡抵扣份额，同 apply_partial_payment 口径）。
pub fn recompute_totals(o: &mut Order) {
    o.total_amount = (o.product_amount - o.coupon_amount
        + o.shipping_fee
        + o.tax_rate_amount.unwrap_or_default()
        - o.points_amount)
        .max(zero());
    o.paid_amount = o.gift_card_amount
        + o.balance
        + o.online_paid_amount
        + o.offline_paid_amount
        + (o.cash_amount - o.cash_change_amount)
        + o.credit_amount;
    o.unpaid_amount = (o.total_amount - o.paid_amount).max(zero());
    o.unrefund_amount = (o.paid_amount - o.total_amount).max(zero());
}

/// 店铺金额明细行提供的三项（单 vendor 分支取行值）。
#[derive(Debug, Clone, Copy, Default)]
pub struct ShopDetailAmounts {
    /// 优惠券金额。
    pub coupon: Decimal,
    /// 运费。
    pub shipping: Decimal,
    /// 优惠/折扣金额。
    pub discount: Decimal,
}

/// 子单金额计划（`compute_child_amounts` 的产物）。
#[derive(Debug, Clone)]
pub struct ChildAmounts {
    /// 商品金额（成交价口径）。
    pub product_amount: Decimal,
    /// 优惠券金额。
    pub coupon_amount: Decimal,
    /// 运费。
    pub shipping_fee: Decimal,
    /// 优惠/折扣金额。
    pub discount_amount: Decimal,
    /// 礼品卡抵扣分摊。
    pub gift_card_amount: Decimal,
    /// 积分抵扣分摊。
    pub points_amount: Decimal,
    /// 手续费分摊。
    pub service_fee: Decimal,
    /// 发票费分摊。
    pub invoice_fee: Decimal,
    /// 抹分分摊。
    pub round_off_cents: Decimal,
    /// 抹角分摊。
    pub round_off_dime: Decimal,
    /// 税费（子单口径独立计算）。
    pub tax_rate_amount: Decimal,
}

/// 计算子单金额（原版 createSplitOrder 金额段 L1900-2071 的我方口径形态）。
///
/// - 单 vendor 店铺且有金额明细行 → coupon/shipping/discount 取行值；
///   否则三者按商品金额分摊（原版明细缺失时取 0、靠尾差补齐；
///   我方直接分摊更稳健，尾差仍保证总额恒等）。
/// - gift/points/service/invoice/roundOff 恒按商品金额分摊；
/// - 税费独立计算；total/paid/unpaid 由 `recompute_totals` 收口。
#[must_use]
pub fn compute_child_amounts(
    parent: &Order,
    group_product: Decimal,
    detail: Option<ShopDetailAmounts>,
) -> ChildAmounts {
    let (coupon_amount, shipping_fee, discount_amount) = match detail {
        Some(d) => (d.coupon, d.shipping, d.discount),
        None => (
            allocated_amount(parent.coupon_amount, parent.product_amount, group_product),
            allocated_amount(parent.shipping_fee, parent.product_amount, group_product),
            allocated_amount(parent.discount_amount, parent.product_amount, group_product),
        ),
    };
    ChildAmounts {
        product_amount: group_product,
        coupon_amount,
        shipping_fee,
        discount_amount,
        gift_card_amount: allocated_amount(
            parent.gift_card_amount,
            parent.product_amount,
            group_product,
        ),
        points_amount: allocated_amount(parent.points_amount, parent.product_amount, group_product),
        service_fee: allocated_amount(parent.service_fee, parent.product_amount, group_product),
        invoice_fee: allocated_amount(parent.invoice_fee, parent.product_amount, group_product),
        round_off_cents: allocated_amount(
            parent.round_off_cents_amount,
            parent.product_amount,
            group_product,
        ),
        round_off_dime: allocated_amount(
            parent.round_off_dime_amount,
            parent.product_amount,
            group_product,
        ),
        tax_rate_amount: child_tax_amount(
            group_product,
            shipping_fee,
            discount_amount,
            parent.tax_rate,
        ),
    }
}

/// 子单 paylog 金额（原版 `resolveSplitPaylogAmount` L4450，我方单金额形态）：
/// 父 paylog 已支付 → balance 码取子单 balance、offline 码取 offlinePaid、
/// 其余取 onlinePaid；未支付 → 子单 unpaid。
#[must_use]
pub fn resolve_split_paylog_amount(pay_code: &str, pay_status: i32, child: &Order) -> Decimal {
    if pay_status == PAYLOG_STATUS_PAID {
        match pay_code {
            "balance" => child.balance,
            "offline" => child.offline_paid_amount,
            _ => child.online_paid_amount,
        }
    } else {
        child.unpaid_amount
    }
}

/// 父单「线上已付」的有效值：我方支付流不写 `online_paid_amount`
/// （恒 0），已付构成以 `paid_amount` 为准——字段有值时用字段，
/// 否则以 paid − gift − balance − offline − (cash − change) − credit 推导
/// （提交时部分抵扣阶段该推导恒为 0，与事实一致）。
#[must_use]
pub fn effective_online_paid(parent: &Order) -> Decimal {
    if parent.online_paid_amount > zero() {
        return parent.online_paid_amount;
    }
    (parent.paid_amount
        - parent.gift_card_amount
        - parent.balance
        - parent.offline_paid_amount
        - (parent.cash_amount - parent.cash_change_amount)
        - parent.credit_amount)
        .max(zero())
}

/// 支付激活的金额分摊（原版 `activateChildOrdersOnPayment` L1610）：
/// paid/online/offline = 父单值 × (子单 total / 父单 total，10 位 HALF_UP → 2 位）；
/// 父单对应值 ≤ 0 时返回 None（不覆盖子单原值）。
#[derive(Debug, Clone, Copy, Default)]
pub struct ActivationShares {
    /// 已付总额分摊。
    pub paid: Option<Decimal>,
    /// 线上已付分摊。
    pub online: Option<Decimal>,
    /// 线下已付分摊。
    pub offline: Option<Decimal>,
}

/// 计算激活分摊。
#[must_use]
pub fn activation_shares(parent: &Order, child: &Order) -> ActivationShares {
    let ratio = if parent.total_amount > zero() {
        (child.total_amount / parent.total_amount)
            .round_dp_with_strategy(10, RoundingStrategy::MidpointAwayFromZero)
    } else {
        zero()
    };
    let share = |v: Decimal| -> Option<Decimal> {
        if v > zero() {
            Some(round2(v * ratio))
        } else {
            None
        }
    };
    ActivationShares {
        paid: share(parent.paid_amount),
        online: share(effective_online_paid(parent)),
        offline: share(parent.offline_paid_amount),
    }
}

// ———————————————————————— 服务实现 ————————————————————————

/// 一次建单的产物：子单（含新 id）+ 明细新旧 id 映射（old_item_id → new_item_id）。
struct CreatedChild {
    order: Order,
    item_map: Vec<(i32, i32)>,
}

/// 跨店拆单服务。所有方法可重入：重复调用收敛到同一终态
/// （已拆标志 / 激活同值重写 / paylog 按 paySn+payCode 幂等）。
pub struct OrderSplitService {
    repos: Arc<Repositories>,
}

impl OrderSplitService {
    /// 组合根外按需构造（与 `DeliveryDispatchService` 同模式）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn admin(
        &self,
    ) -> &nextrade_infrastructure::db::repo::trade::order_admin::OrderAdminRepository {
        &self.repos.trade.order_admin
    }

    /// 根父单 id（子单再拆时明细/paylog 仍挂根父单，对应原版
    /// createSplitOrder 的 parentOrderId 归根处理）。
    fn root_id(order: &Order) -> i32 {
        order
            .parent_order_id
            .filter(|id| *id > 0)
            .unwrap_or(order.order_id)
    }

    fn root_sn(order: &Order) -> String {
        order
            .parent_order_sn
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| order.order_sn.clone())
    }

    /// 明细供应商回填：下单路径的明细 INSERT 不落 vendor_id，
    /// 拆单分组以商品表为权威源补齐（仅当明细本身缺失时）。
    async fn fill_item_vendors(&self, items: &mut [OrderItem]) {
        let missing: Vec<i32> = items
            .iter()
            .filter(|i| i.vendor_id.is_none())
            .map(|i| i.product_id)
            .collect();
        if missing.is_empty() {
            return;
        }
        if let Ok(products) = self.repos.catalog.product.find_by_ids(&missing).await {
            let by_id: std::collections::HashMap<i32, Option<i32>> = products
                .iter()
                .map(|p| (p.product_id, p.vendor_id))
                .collect();
            for item in items.iter_mut() {
                if item.vendor_id.is_none() {
                    item.vendor_id = by_id.get(&item.product_id).copied().flatten();
                }
            }
        }
    }

    /// 提交时拆单（原版 `splitOrderAtSubmit`）。
    ///
    /// 返回值：true = 已处理（单组置标志或多组建子单）；false = 无需/无法处理。
    pub async fn split_order_at_submit(&self, order_id: i32) -> Result<bool> {
        let Some(parent) = self.admin().find_order(order_id).await? else {
            return Ok(false);
        };
        if parent.is_store_splited == 1 {
            return Ok(false);
        }
        let mut items = self.admin().find_items(order_id).await?;
        if items.is_empty() {
            return Ok(false);
        }
        self.fill_item_vendors(&mut items).await;
        let gift_exchange = parent.order_type == Some(ORDER_TYPE_GIFT_EXCHANGE);
        let groups = group_items(&items, gift_exchange);
        if groups.len() <= 1 {
            // 单组：只置标志；头缺失时顺手补头（存量自愈，仅 shop_id=0 时）。
            let mut patch = OrderPatch::new()
                .i32_val("is_store_splited", 1)
                .i32_val("is_valid", 1);
            if parent.shop_id == 0 && groups[0].shop_id > 0 {
                patch = patch.i32_val("shop_id", groups[0].shop_id);
                patch = patch.opt_i32("vendor_id", groups[0].last_item_vendor());
            }
            self.admin().update_order(order_id, &patch).await?;
            return Ok(true);
        }
        let children = self.create_children(&parent, &groups).await?;
        self.split_salesman_orders(&parent, &children).await;
        self.adjust_amounts_and_taxes(&parent, &children).await;
        self.admin()
            .update_order(
                order_id,
                &OrderPatch::new()
                    .i32_val("is_store_splited", 1)
                    .i32_val("is_valid", 1),
            )
            .await?;
        let _ = self
            .admin()
            .quick_order_log(
                order_id,
                &parent.order_sn,
                "订单商品来自不同店铺，提交时已拆分",
            )
            .await;
        Ok(true)
    }

    /// 支付时拆单 / 激活（原版 `splitStoreOrder`）。
    ///
    /// - 已拆且有子单 → 激活子单（父单 isValid=0）；
    /// - 已拆无子单（单店）→ false（预售配送由分发钩子自理）；
    /// - 未拆单组 → 只置标志（+ 头缺失补头）；
    /// - 未拆多组 → 建子单 + 尾差 + 删除父单行与明细。
    pub async fn split_store_order(&self, order_id: i32) -> Result<bool> {
        let Some(parent) = self.admin().find_order(order_id).await? else {
            return Ok(false);
        };
        if parent.is_store_splited == 1 {
            let children = self.admin().find_children(order_id).await?;
            let active: Vec<Order> = children
                .into_iter()
                .filter(|c| c.order_status != status::CANCELED)
                .collect();
            if active.is_empty() {
                return Ok(false);
            }
            if parent.order_status == status::CANCELED {
                return Ok(false);
            }
            // 未支付父单：子单已在提交时建好，激活必须等支付完成
            // （原版该分支仅由 setOrderPaid 到达；我方管理端亦可达此入口，
            // 故显式守卫，避免未付子单被误标已付）。
            if parent.pay_status != PAY_STATUS_PAID {
                return Ok(true);
            }
            self.activate_children(&parent, &active).await?;
            return Ok(true);
        }
        let mut items = self.admin().find_items(order_id).await?;
        if items.is_empty() {
            return Ok(false);
        }
        self.fill_item_vendors(&mut items).await;
        // 支付时分组无配送组维度（原版 splitStoreOrder 分组键仅 shop→vendor）。
        let groups = group_items(&items, false);
        if groups.len() <= 1 {
            let mut patch = OrderPatch::new().i32_val("is_store_splited", 1);
            if parent.shop_id == 0 && groups[0].shop_id > 0 {
                patch = patch.i32_val("shop_id", groups[0].shop_id);
                patch = patch.opt_i32("vendor_id", groups[0].last_item_vendor());
            }
            self.admin().update_order(order_id, &patch).await?;
            return Ok(true);
        }
        let children = self.create_children(&parent, &groups).await?;
        // 父单已支付：逐子单触发配送分发（原版在建子单后逐单 createDeliveryOrderIfNeeded）。
        if parent.pay_status == PAY_STATUS_PAID {
            let dispatch =
                crate::o2o::delivery_dispatch::DeliveryDispatchService::new(self.repos.clone());
            for child in &children {
                if let Err(e) = dispatch.on_order_paid(child.order.order_id).await {
                    tracing::warn!("拆单子单 {} 派送单生产失败: {e}", child.order.order_id);
                }
            }
        }
        self.split_salesman_orders(&parent, &children).await;
        self.adjust_amounts_and_taxes(&parent, &children).await;
        let _ = self
            .admin()
            .quick_order_log(order_id, &parent.order_sn, "订单商品来自不同店铺，已拆分")
            .await;
        // 删除父单行与全部明细（原版 removeById + 明细 remove）。
        self.admin().delete_items(order_id).await?;
        self.admin().delete_order_row(order_id).await?;
        Ok(true)
    }

    /// 激活子单（原版 `activateChildOrdersOnPayment`）。
    async fn activate_children(&self, parent: &Order, children: &[Order]) -> Result<()> {
        let now = utils::now_seconds() as i32;
        let dispatch =
            crate::o2o::delivery_dispatch::DeliveryDispatchService::new(self.repos.clone());
        for child in children {
            let shares = activation_shares(parent, child);
            let mut patch = OrderPatch::new()
                .i32_val("pay_status", PAY_STATUS_PAID)
                .i32_val("order_status", status::PAID)
                .i32_val("pay_time", now)
                .decimal("unpaid_amount", zero());
            if let Some(v) = shares.paid {
                patch = patch.decimal("paid_amount", v);
            }
            if let Some(v) = shares.online {
                patch = patch.decimal("online_paid_amount", v);
            }
            if let Some(v) = shares.offline {
                patch = patch.decimal("offline_paid_amount", v);
            }
            self.admin().update_order(child.order_id, &patch).await?;
            // 子单 paylog 补齐/同步到已支付口径（退款通路前置）。
            let mut synced = child.clone();
            if let Some(v) = shares.paid {
                synced.paid_amount = v;
            }
            if let Some(v) = shares.online {
                synced.online_paid_amount = v;
            }
            if let Some(v) = shares.offline {
                synced.offline_paid_amount = v;
            }
            synced.unpaid_amount = zero();
            if let Err(e) = self.sync_child_paylogs(parent, &synced).await {
                tracing::warn!("子单 {} paylog 同步失败: {e}", child.order_id);
            }
            // 逐子单配送分发（原版 createDeliveryOrderIfNeeded，失败不阻断）。
            if let Err(e) = dispatch.on_order_paid(child.order_id).await {
                tracing::warn!("子单 {} 派送单生产失败: {e}", child.order_id);
            }
        }
        // 分销佣金行重映射（支付节点生成在父单明细上的行随子单走）。
        let parent_items = self.admin().find_items(parent.order_id).await?;
        let mut child_items: Vec<(i32, Vec<OrderItem>)> = Vec::new();
        for child in children {
            let items = self.admin().find_items(child.order_id).await?;
            child_items.push((child.order_id, items));
        }
        let item_map = build_item_map(&parent_items, &child_items);
        self.remap_salesman_rows(parent.order_id, &item_map).await;
        // 父单标记无效（留痕不删除）。
        self.admin()
            .update_order(parent.order_id, &OrderPatch::new().i32_val("is_valid", 0))
            .await?;
        Ok(())
    }

    /// 按分组创建全部子单（原版 createSplitOrder 逐组调用）。
    async fn create_children(
        &self,
        parent: &Order,
        groups: &[SplitGroup],
    ) -> Result<Vec<CreatedChild>> {
        let root_id = Self::root_id(parent);
        let root_sn = Self::root_sn(parent);
        let details = self
            .admin()
            .find_amount_details(root_id)
            .await
            .unwrap_or_default();
        let coupon_details = self
            .admin()
            .find_coupon_details(root_id)
            .await
            .unwrap_or_default();
        let mut shop_group_count: BTreeMap<i32, usize> = BTreeMap::new();
        for g in groups {
            *shop_group_count.entry(g.shop_id).or_default() += 1;
        }
        let parent_data = serde_json::to_string(parent).unwrap_or_default();
        let now = utils::now_seconds() as i32;
        let mut out: Vec<CreatedChild> = Vec::new();
        for group in groups {
            let single_vendor = shop_group_count.get(&group.shop_id).copied().unwrap_or(0) == 1;
            let detail = if single_vendor {
                details
                    .iter()
                    .find(|d| d.shop_id == Some(group.shop_id))
                    .map(|d| ShopDetailAmounts {
                        coupon: d.coupon_amount.unwrap_or_default(),
                        shipping: d.shipping_fee.unwrap_or_default(),
                        discount: d.discount_amount.unwrap_or_default(),
                    })
            } else {
                None
            };
            let amounts = compute_child_amounts(parent, group.product_amount(), detail);
            let mut child = parent.clone();
            child.order_id = 0;
            child.order_sn = self.unique_order_sn().await?;
            child.shop_id = group.shop_id;
            child.vendor_id = group.last_item_vendor();
            child.parent_order_id = Some(root_id);
            child.parent_order_sn = Some(root_sn.clone());
            child.is_store_splited = 1;
            child.is_valid = 1;
            child.pay_status = if parent.pay_status == PAY_STATUS_PAID {
                PAY_STATUS_PAID
            } else {
                PAY_STATUS_UNPAID
            };
            child.product_amount = amounts.product_amount;
            child.coupon_amount = amounts.coupon_amount;
            child.shipping_fee = amounts.shipping_fee;
            child.discount_amount = amounts.discount_amount;
            child.gift_card_amount = amounts.gift_card_amount;
            child.points_amount = amounts.points_amount;
            child.service_fee = amounts.service_fee;
            child.invoice_fee = amounts.invoice_fee;
            child.round_off_cents_amount = amounts.round_off_cents;
            child.round_off_dime_amount = amounts.round_off_dime;
            child.tax_rate_amount = Some(amounts.tax_rate_amount);
            child.total_amount = total_formula(
                child.product_amount,
                child.coupon_amount,
                child.shipping_fee,
                amounts.tax_rate_amount,
                child.points_amount,
            );
            // 已付构成按 total 之比分摊（原版 balance/online/offline 口径；
            // cash/credit 一并分摊，避免多子单重复计父单全额）。
            child.balance =
                allocated_amount(parent.balance, parent.total_amount, child.total_amount);
            child.online_paid_amount = allocated_amount(
                effective_online_paid(parent),
                parent.total_amount,
                child.total_amount,
            );
            child.offline_paid_amount = allocated_amount(
                parent.offline_paid_amount,
                parent.total_amount,
                child.total_amount,
            );
            child.cash_amount =
                allocated_amount(parent.cash_amount, parent.total_amount, child.total_amount);
            child.cash_change_amount = allocated_amount(
                parent.cash_change_amount,
                parent.total_amount,
                child.total_amount,
            );
            child.credit_amount = allocated_amount(
                parent.credit_amount,
                parent.total_amount,
                child.total_amount,
            );
            recompute_totals(&mut child);
            if group.is_no_shipping() {
                child.shipping_method = 3;
                child.shipping_type_name = "无需配送".to_string();
            }
            let child_id = self.admin().insert_order_full(&child).await?;
            child.order_id = child_id;
            // 明细：全新行插入（新 item_id），永不回指父单明细。
            let mut item_map: Vec<(i32, i32)> = Vec::new();
            for item in &group.items {
                let mut ni = item.clone();
                ni.item_id = 0;
                ni.order_id = child_id;
                ni.order_sn = child.order_sn.clone();
                let new_id = self.admin().insert_order_item_full(&ni).await?;
                item_map.push((item.item_id, new_id));
            }
            // 店铺金额明细按子单重存（单 vendor 分支的数据行随子单走）。
            if single_vendor {
                if let Some(d) = details.iter().find(|d| d.shop_id == Some(group.shop_id)) {
                    let mut nd = d.clone();
                    nd.order_discount_detail_id = 0;
                    nd.order_id = Some(child_id);
                    let _ = self.admin().insert_amount_detail(&nd).await;
                }
            }
            // 优惠券明细复制：本店行 + 平台行（shop_id=0）。
            for cd in &coupon_details {
                if cd.shop_id == Some(group.shop_id) || cd.shop_id == Some(0) {
                    let mut nd = cd.clone();
                    nd.order_coupon_detail_id = 0;
                    nd.order_id = Some(child_id);
                    let _ = self.admin().insert_coupon_detail(&nd).await;
                }
            }
            // 拆单日志（每子单一条，父单数据快照为根父单）。
            let _ = self
                .admin()
                .insert_order_split_log(root_id, child_id, now, &parent_data)
                .await;
            // paylog 复制（父单已有支付记录时）。
            if let Err(e) = self.copy_parent_paylogs(parent, &child).await {
                tracing::warn!("子单 {child_id} paylog 复制失败: {e}");
            }
            out.push(CreatedChild {
                order: child,
                item_map,
            });
        }
        Ok(out)
    }

    /// 生成不与现有订单冲突的订单号（与下单路径同一生成器 + 存在性重试）。
    async fn unique_order_sn(&self) -> Result<String> {
        for _ in 0..5 {
            let sn = utils::generate_order_sn();
            if !self.repos.trade.order.sn_exists(&sn).await? {
                return Ok(sn);
            }
        }
        Ok(utils::generate_order_sn())
    }

    /// 子单 paylog 应记金额：先按字段口径 resolve；若父 paylog 已支付且
    /// 金额为正、而字段口径得出 0（我方部分已付构成不落订单字段），
    /// 回退为父 paylog 金额按 total 之比分摊，保证子单可退额度不丢。
    fn child_paylog_amount(parent: &Order, parent_log: &Paylog, child: &Order) -> Decimal {
        let amount =
            resolve_split_paylog_amount(&parent_log.pay_code, parent_log.pay_status, child);
        if amount <= zero()
            && parent_log.pay_status == PAYLOG_STATUS_PAID
            && parent_log.pay_amount > zero()
            && parent.total_amount > zero()
        {
            let ratio = (child.total_amount / parent.total_amount)
                .round_dp_with_strategy(10, RoundingStrategy::MidpointAwayFromZero);
            return round2(parent_log.pay_amount * ratio);
        }
        amount
    }

    /// 父单 paylog 逐行复制到子单（同 paySn；金额按子单口径 resolve）。
    ///
    /// 幂等：子单已有同 (pay_sn, pay_code) 行时跳过该行。
    async fn copy_parent_paylogs(&self, parent: &Order, child: &Order) -> Result<()> {
        // paylog 挂在被拆单本身（原版 createSplitOrder 读 order.getOrderId() 的
        // paylog）；金额/券明细才按根父单读取，两者不要混。
        let parent_logs = self
            .repos
            .payment
            .paylog
            .find_all_by_order(parent.order_id, 0)
            .await?;
        if parent_logs.is_empty() {
            return Ok(());
        }
        let child_logs = self
            .repos
            .payment
            .paylog
            .find_all_by_order(child.order_id, 0)
            .await?;
        let now = utils::now_seconds() as i32;
        for pl in &parent_logs {
            if child_logs
                .iter()
                .any(|c| c.pay_sn == pl.pay_sn && c.pay_code == pl.pay_code)
            {
                continue;
            }
            let amount = Self::child_paylog_amount(parent, pl, child);
            let row = Paylog {
                paylog_id: 0,
                pay_sn: pl.pay_sn.clone(),
                pay_name: pl.pay_name.clone(),
                order_id: child.order_id,
                order_sn: child.order_sn.clone(),
                order_amount: child.total_amount,
                order_type: 0,
                pay_amount: amount,
                pay_status: pl.pay_status,
                pay_code: pl.pay_code.clone(),
                profit_sharing_channel: pl.profit_sharing_channel.clone(),
                add_time: Some(now),
                transaction_id: pl.transaction_id.clone(),
                notify_data: String::new(),
                refund_amount: zero(),
                token_code: String::new(),
                appid: pl.appid.clone(),
            };
            self.repos.payment.paylog.insert(&row).await?;
        }
        Ok(())
    }

    /// 激活时子单 paylog 同步：先补齐缺失行，再逐行按子单当前金额
    /// 对齐金额/状态/交易号（原版 `syncSplitOrderPaylogAmount` 的落点）。
    async fn sync_child_paylogs(&self, parent: &Order, child: &Order) -> Result<()> {
        self.copy_parent_paylogs(parent, child).await?;
        let parent_logs = self
            .repos
            .payment
            .paylog
            .find_all_by_order(parent.order_id, 0)
            .await?;
        let child_logs = self
            .repos
            .payment
            .paylog
            .find_all_by_order(child.order_id, 0)
            .await?;
        for cl in &child_logs {
            if let Some(pl) = parent_logs
                .iter()
                .find(|p| p.pay_sn == cl.pay_sn && p.pay_code == cl.pay_code)
            {
                let amount = Self::child_paylog_amount(parent, pl, child);
                self.repos
                    .payment
                    .paylog
                    .sync_split_amount(
                        cl.paylog_id,
                        amount,
                        child.total_amount,
                        pl.pay_status,
                        &pl.transaction_id,
                    )
                    .await?;
            }
        }
        Ok(())
    }

    /// 分销佣金行重映射（建子单后）：把父单明细上的 salesman_order 行
    /// 改挂到子单的新明细行（原版 splitSalesmanOrdersForStoreSplit）。
    async fn split_salesman_orders(&self, parent: &Order, children: &[CreatedChild]) {
        let mut map: Vec<(i32, i32, i32)> = Vec::new();
        for child in children {
            for (old_id, new_id) in &child.item_map {
                map.push((*old_id, child.order.order_id, *new_id));
            }
        }
        self.remap_salesman_rows(parent.order_id, &map).await;
    }

    /// 执行重映射（失败仅记日志，永不阻断拆单——原版同口径）。
    async fn remap_salesman_rows(&self, parent_order_id: i32, map: &[(i32, i32, i32)]) {
        if map.is_empty() {
            return;
        }
        let rows: std::result::Result<Vec<(i32, Option<i32>)>, sqlx::Error> = sqlx::query_as(
            "SELECT salesman_order_id, item_id FROM salesman_order WHERE order_id = $1",
        )
        .bind(parent_order_id)
        .fetch_all(&self.repos.pool)
        .await;
        let rows = match rows {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("拆单分销行读取失败 parent={parent_order_id}: {e}");
                return;
            }
        };
        for (row_id, item_id) in rows {
            let Some(old_item) = item_id else { continue };
            let Some((_, child_order, new_item)) = map.iter().find(|(o, _, _)| *o == old_item)
            else {
                continue;
            };
            if let Err(e) = sqlx::query(
                "UPDATE salesman_order SET order_id = $1, item_id = $2 WHERE salesman_order_id = $3",
            )
            .bind(*child_order)
            .bind(*new_item)
            .bind(row_id)
            .execute(&self.repos.pool)
            .await
            {
                tracing::warn!("拆单分销行重映射失败 row={row_id}: {e}");
            }
        }
    }

    /// 尾差 + 税差调整（原版 adjustSplitOrderAmounts / adjustSplitOrderTaxAmounts；
    /// 两段均为失败仅记日志、不阻断主流程）。
    async fn adjust_amounts_and_taxes(&self, parent: &Order, children: &[CreatedChild]) {
        if children.len() < 2 {
            return;
        }
        if let Err(e) = self.adjust_amounts_inner(parent, children).await {
            tracing::warn!("拆单尾差补齐失败 parent={}: {e}", parent.order_id);
        }
        if let Err(e) = self.adjust_taxes_inner(parent, children).await {
            tracing::warn!("拆单税费补齐失败 parent={}: {e}", parent.order_id);
        }
    }

    async fn adjust_amounts_inner(&self, parent: &Order, children: &[CreatedChild]) -> Result<()> {
        let mut orders: Vec<Order> = Vec::new();
        for c in children {
            let o = self
                .admin()
                .find_order(c.order.order_id)
                .await?
                .unwrap_or_else(|| c.order.clone());
            orders.push(o);
        }
        let (last, firsts) = orders
            .split_last()
            .expect("children >= 2 checked by caller");
        let mut last = last.clone();
        let sum = |f: fn(&Order) -> Decimal| -> Decimal { firsts.iter().map(|o| f(o)).sum() };
        let mut changed = false;
        let mut apply = |field: &mut Decimal, parent_v: Decimal, s: Decimal| {
            let adjusted = (parent_v - s).max(zero());
            if adjusted != *field {
                *field = adjusted;
                changed = true;
            }
        };
        apply(&mut last.balance, parent.balance, sum(|o| o.balance));
        // 线上已付的尾差目标取有效值（我方 online_paid_amount 字段恒 0，
        // 建子单时按 effective_online_paid 分摊，此处须同口径收口）。
        apply(
            &mut last.online_paid_amount,
            effective_online_paid(parent),
            sum(|o| o.online_paid_amount),
        );
        apply(
            &mut last.offline_paid_amount,
            parent.offline_paid_amount,
            sum(|o| o.offline_paid_amount),
        );
        apply(
            &mut last.coupon_amount,
            parent.coupon_amount,
            sum(|o| o.coupon_amount),
        );
        apply(
            &mut last.discount_amount,
            parent.discount_amount,
            sum(|o| o.discount_amount),
        );
        apply(
            &mut last.points_amount,
            parent.points_amount,
            sum(|o| o.points_amount),
        );
        apply(
            &mut last.shipping_fee,
            parent.shipping_fee,
            sum(|o| o.shipping_fee),
        );
        apply(
            &mut last.service_fee,
            parent.service_fee,
            sum(|o| o.service_fee),
        );
        apply(
            &mut last.invoice_fee,
            parent.invoice_fee,
            sum(|o| o.invoice_fee),
        );
        apply(
            &mut last.gift_card_amount,
            parent.gift_card_amount,
            sum(|o| o.gift_card_amount),
        );
        if !changed {
            return Ok(());
        }
        // 尾差落点子单税费按其新字段重算，再重算总额，最后同步其 paylog。
        last.tax_rate_amount = Some(child_tax_amount(
            last.product_amount,
            last.shipping_fee,
            last.discount_amount,
            last.tax_rate,
        ));
        recompute_totals(&mut last);
        self.persist_amounts(&last).await?;
        if let Err(e) = self.sync_child_paylogs(parent, &last).await {
            tracing::warn!("尾差子单 {} paylog 同步失败: {e}", last.order_id);
        }
        Ok(())
    }

    async fn adjust_taxes_inner(&self, parent: &Order, children: &[CreatedChild]) -> Result<()> {
        let mut orders: Vec<Order> = Vec::new();
        for c in children {
            let o = self
                .admin()
                .find_order(c.order.order_id)
                .await?
                .unwrap_or_else(|| c.order.clone());
            orders.push(o);
        }
        // 逐子单按字段重算税费（原版先 refresh 再比差）。
        let mut computed: Vec<Decimal> = orders
            .iter()
            .map(|o| {
                child_tax_amount(
                    o.product_amount,
                    o.shipping_fee,
                    o.discount_amount,
                    o.tax_rate,
                )
            })
            .collect();
        let computed_total: Decimal = computed.iter().sum();
        let parent_tax = round2(parent.tax_rate_amount.unwrap_or_default());
        let mut diff = round2(parent_tax - computed_total);
        if diff != zero() {
            for i in (0..orders.len()).rev() {
                if diff == zero() {
                    break;
                }
                let adjusted = computed[i] + diff;
                if adjusted < zero() {
                    diff = adjusted;
                    computed[i] = zero();
                } else {
                    computed[i] = round2(adjusted);
                    diff = zero();
                }
            }
        }
        for (o, tax) in orders.iter_mut().zip(computed.iter()) {
            if o.tax_rate_amount.unwrap_or_default() != *tax {
                o.tax_rate_amount = Some(*tax);
                recompute_totals(o);
                self.persist_amounts(o).await?;
            }
        }
        Ok(())
    }

    /// 全额字段回写（尾差/税差调整后的子单金额落库）。
    async fn persist_amounts(&self, o: &Order) -> Result<()> {
        let patch = OrderPatch::new()
            .decimal("total_amount", o.total_amount)
            .decimal("paid_amount", o.paid_amount)
            .decimal("unpaid_amount", o.unpaid_amount)
            .decimal("unrefund_amount", o.unrefund_amount)
            .decimal("product_amount", o.product_amount)
            .decimal("coupon_amount", o.coupon_amount)
            .decimal("points_amount", o.points_amount)
            .decimal("discount_amount", o.discount_amount)
            .decimal("balance", o.balance)
            .decimal("credit_amount", o.credit_amount)
            .decimal("online_paid_amount", o.online_paid_amount)
            .decimal("offline_paid_amount", o.offline_paid_amount)
            .decimal("service_fee", o.service_fee)
            .decimal("shipping_fee", o.shipping_fee)
            .decimal("invoice_fee", o.invoice_fee)
            .decimal("gift_card_amount", o.gift_card_amount)
            .decimal("cash_amount", o.cash_amount)
            .decimal("cash_change_amount", o.cash_change_amount)
            .decimal("round_off_dime_amount", o.round_off_dime_amount)
            .decimal("round_off_cents_amount", o.round_off_cents_amount)
            .opt_decimal("tax_rate_amount", o.tax_rate_amount);
        self.admin().update_order(o.order_id, &patch).await?;
        Ok(())
    }
}

/// 由既有父/子明细构建重映射表（激活时用）：
/// 按 (product_id, sku_id) 出现次序逐一配对父明细与各子单明细，
/// 返回 (old_item_id, child_order_id, new_item_id) 列表。
fn build_item_map(
    parent_items: &[OrderItem],
    child_items: &[(i32, Vec<OrderItem>)],
) -> Vec<(i32, i32, i32)> {
    let mut pool: Vec<&OrderItem> = parent_items.iter().collect();
    let mut out: Vec<(i32, i32, i32)> = Vec::new();
    for (child_order_id, items) in child_items {
        for item in items {
            if let Some(pos) = pool
                .iter()
                .position(|p| p.product_id == item.product_id && p.sku_id == item.sku_id)
            {
                let old = pool.remove(pos);
                out.push((old.item_id, *child_order_id, item.item_id));
            }
        }
    }
    out
}

// ———————————————————————— 单元测试 ————————————————————————

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn item(item_id: i32, shop_id: i32, vendor: Option<i32>, price: &str, qty: i32) -> OrderItem {
        OrderItem {
            item_id,
            shop_id,
            vendor_id: vendor,
            price: d(price),
            quantity: qty,
            product_id: 1000 + item_id,
            sku_id: 0,
            product_type: 1,
            ..Default::default()
        }
    }

    fn order_with(total: &str, paid: &str) -> Order {
        Order {
            total_amount: d(total),
            paid_amount: d(paid),
            ..Default::default()
        }
    }

    #[test]
    fn header_vendor_is_last_item() {
        let items = vec![item(1, 1, Some(7), "10", 1), item(2, 1, Some(9), "10", 1)];
        assert_eq!(header_vendor_id(&items), Some(9));
        // 最后一行无供应商 → 覆盖为 None（原版循环覆盖语义）
        let items2 = vec![item(1, 1, Some(7), "10", 1), item(2, 1, None, "10", 1)];
        assert_eq!(header_vendor_id(&items2), None);
        assert_eq!(header_vendor_id(&[]), None);
    }

    #[test]
    fn grouping_by_shop_vendor_and_delivery() {
        let items = vec![
            item(1, 1, Some(7), "10", 1),
            item(2, 2, None, "20", 1),
            item(3, 1, Some(7), "30", 1),
            item(4, 1, Some(8), "40", 1),
        ];
        let groups = group_items(&items, false);
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].shop_id, 1);
        assert_eq!(groups[0].items.len(), 2);
        assert_eq!(groups[0].product_amount(), d("40"));
        assert_eq!(groups[1].shop_id, 2);
        assert_eq!(groups[2].vendor_id, 8);
    }

    #[test]
    fn grouping_gift_exchange_virtual_items_split_singly() {
        let mut v1 = item(1, 1, None, "10", 1);
        v1.product_type = PRODUCT_TYPE_VIRTUAL;
        let mut v2 = item(2, 1, None, "10", 1);
        v2.product_type = PRODUCT_TYPE_VIRTUAL;
        let physical = item(3, 1, None, "10", 1);
        let groups = group_items(&[v1, v2, physical], true);
        // 两个虚拟商品各成一组 + 实物一组
        assert_eq!(groups.len(), 3);
        assert!(groups.iter().take(2).all(|g| g.is_no_shipping()));
        assert!(!groups[2].is_no_shipping());
        // 非兑换单：虚拟商品不单独成组
        let mut v3 = item(4, 1, None, "10", 1);
        v3.product_type = PRODUCT_TYPE_VIRTUAL;
        let groups2 = group_items(&[v3, item(5, 1, None, "10", 1)], false);
        assert_eq!(groups2.len(), 1);
    }

    #[test]
    fn allocated_amount_matches_java() {
        // ratio = 100/300 @10dp = 0.3333333333；share = 100 × ratio = 33.33
        assert_eq!(allocated_amount(d("100"), d("300"), d("100")), d("33.33"));
        // 全额
        assert_eq!(
            allocated_amount(d("66.60"), d("66.60"), d("66.60")),
            d("66.60")
        );
        // 零守卫
        assert_eq!(allocated_amount(d("0"), d("300"), d("100")), zero());
        assert_eq!(allocated_amount(d("100"), d("0"), d("100")), zero());
        assert_eq!(allocated_amount(d("100"), d("300"), d("0")), zero());
    }

    #[test]
    fn child_tax_uses_ceiling_and_percent_rate() {
        // 基数 100 + 10 − 5 = 105，税率 10% → 10.50
        assert_eq!(
            child_tax_amount(d("100"), d("10"), d("5"), Some(d("10"))),
            d("10.50")
        );
        // 基数 10.01 × 13% = 1.3013 → CEILING 1.31（HALF_UP 会得 1.30）
        assert_eq!(
            child_tax_amount(d("10.01"), d("0"), d("0"), Some(d("13"))),
            d("1.31")
        );
        // 无税率 / 基数非正
        assert_eq!(child_tax_amount(d("100"), d("0"), d("0"), None), zero());
        assert_eq!(
            child_tax_amount(d("0"), d("0"), d("5"), Some(d("10"))),
            zero()
        );
    }

    #[test]
    fn child_amounts_detail_branch_and_prorate_branch() {
        let mut parent = order_with("300", "0");
        parent.product_amount = d("300");
        parent.coupon_amount = d("30");
        parent.shipping_fee = d("15");
        parent.discount_amount = d("6");
        parent.gift_card_amount = d("60");
        parent.points_amount = d("3");
        // 明细行分支：三项取行值
        let with_detail = compute_child_amounts(
            &parent,
            d("100"),
            Some(ShopDetailAmounts {
                coupon: d("12"),
                shipping: d("9"),
                discount: d("2"),
            }),
        );
        assert_eq!(with_detail.coupon_amount, d("12"));
        assert_eq!(with_detail.shipping_fee, d("9"));
        assert_eq!(with_detail.discount_amount, d("2"));
        // gift 按商品额分摊：60 × 100/300 = 20.00
        assert_eq!(with_detail.gift_card_amount, d("20.00"));
        assert_eq!(with_detail.points_amount, d("1.00"));
        // 分摊分支：coupon 30 × 100/300 = 10.00；shipping 15 → 5.00
        let prorated = compute_child_amounts(&parent, d("100"), None);
        assert_eq!(prorated.coupon_amount, d("10.00"));
        assert_eq!(prorated.shipping_fee, d("5.00"));
        assert_eq!(prorated.discount_amount, d("2.00"));
    }

    #[test]
    fn recompute_totals_follows_create_formula() {
        let mut o = Order {
            product_amount: d("100"),
            coupon_amount: d("10"),
            shipping_fee: d("5"),
            tax_rate_amount: Some(d("9.50")),
            points_amount: d("1"),
            gift_card_amount: d("20"),
            balance: d("30"),
            ..Default::default()
        };
        recompute_totals(&mut o);
        assert_eq!(o.total_amount, d("103.50"));
        assert_eq!(o.paid_amount, d("50"));
        assert_eq!(o.unpaid_amount, d("53.50"));
        assert_eq!(o.unrefund_amount, d("0"));
    }

    #[test]
    fn paylog_amount_resolution() {
        let mut child = order_with("100", "0");
        child.balance = d("30");
        child.offline_paid_amount = d("10");
        child.online_paid_amount = d("60");
        child.unpaid_amount = d("40");
        assert_eq!(
            resolve_split_paylog_amount("balance", PAYLOG_STATUS_PAID, &child),
            d("30")
        );
        assert_eq!(
            resolve_split_paylog_amount("offline", PAYLOG_STATUS_PAID, &child),
            d("10")
        );
        assert_eq!(
            resolve_split_paylog_amount("wechat", PAYLOG_STATUS_PAID, &child),
            d("60")
        );
        assert_eq!(resolve_split_paylog_amount("wechat", 0, &child), d("40"));
    }

    #[test]
    fn effective_online_paid_derivation() {
        let mut parent = order_with("300", "300");
        parent.gift_card_amount = d("50");
        parent.balance = d("50");
        // 字段未写（恒 0）时由 paid 推导：300 − 50 − 50 = 200
        assert_eq!(effective_online_paid(&parent), d("200"));
        // 提交阶段部分抵扣：paid = gift + balance → 推导 0
        parent.paid_amount = d("100");
        assert_eq!(effective_online_paid(&parent), d("0"));
        // 字段有值优先
        parent.online_paid_amount = d("123");
        assert_eq!(effective_online_paid(&parent), d("123"));
    }

    #[test]
    fn activation_shares_by_total_ratio() {
        let parent = order_with("300", "300");
        let child = order_with("100", "0");
        let shares = activation_shares(&parent, &child);
        assert_eq!(shares.paid, Some(d("100.00")));
        // 父单 online 字段为 0 但 paid 推导为 300 → 子单 online 100.00
        assert_eq!(shares.online, Some(d("100.00")));
        assert_eq!(shares.offline, None);
        // 父单未付：全 None（不覆盖）
        let unpaid_parent = order_with("300", "0");
        let shares2 = activation_shares(&unpaid_parent, &child);
        assert_eq!(shares2.paid, None);
        assert_eq!(shares2.online, None);
    }

    #[test]
    fn item_map_pairs_by_occurrence() {
        let p1 = item(11, 1, None, "10", 1);
        let p2 = item(12, 1, None, "10", 1);
        let mut c1 = item(21, 1, None, "10", 1);
        c1.product_id = p1.product_id;
        let mut c2 = item(22, 1, None, "10", 1);
        c2.product_id = p2.product_id;
        // 同 product+sku 两行：按出现次序配对
        let mut p1b = item(13, 1, None, "10", 1);
        p1b.product_id = p1.product_id;
        let mut c3 = item(23, 1, None, "10", 1);
        c3.product_id = p1.product_id;
        let map = build_item_map(
            &[p1.clone(), p1b, p2],
            &[(501, vec![c1, c3]), (502, vec![c2])],
        );
        assert_eq!(map.len(), 3);
        assert!(map.contains(&(11, 501, 21)));
        assert!(map.contains(&(13, 501, 23)));
        assert!(map.contains(&(12, 502, 22)));
    }

    /// 源码不变量：子单明细只能全新插入，拆单模块不得出现回指父单明细的调用
    /// （手工拆单旧实现的双写根源即 insert + update_item_order_id 并用）。
    #[test]
    fn split_never_rehangs_parent_items() {
        let src = include_str!("order_split.rs");
        let body_start = src
            .find("pub struct OrderSplitService")
            .expect("服务应存在");
        let body = &src[body_start..src.find("#[cfg(test)]").expect("测试段应存在")];
        assert!(
            !body.contains("update_item_order_id"),
            "拆单不得回指父单明细（双写）"
        );
        assert!(
            body.contains("insert_order_item_full"),
            "子单明细应以全列新行插入"
        );
    }
}
