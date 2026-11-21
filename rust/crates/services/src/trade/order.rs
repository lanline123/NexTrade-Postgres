//! 订单服务：下单、状态流转、取消（回滚库存）、支付回调。
//! 对应原 Java `OrderServiceImpl`。

use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_common::utils;
use nextrade_core::entities::{OfflinePaymentClaim, Order, OrderItem};
use nextrade_core::models::payment::refund_apply::RefundInitiator;
use nextrade_core::models::trade::offline_claim::{
    status_name, stage_name, OfflinePaymentClaimAuditDTO, OfflinePaymentClaimSubmitDTO,
    OfflinePaymentClaimVO,
};
use nextrade_core::models::catalog::product_delivery::delivery_access;
use nextrade_core::models::marketing::gift_card::{scoped_deduct_base, GiftCardScope};
use nextrade_core::models::trade::order_admin::join_status;
use nextrade_core::models::{OrderCreateDTO, OrderCreateVO, OrderVO};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

use crate::trade::cart::CartService;
use crate::catalog::product::ProductService;

/// 订单状态常量（严格对齐 `java/database/pure.sql` 中 `order.order_status`
/// 的列注释：`0 待确认/待支付 1 已确定/待发货 2 处理中/已发货 3 已取消
/// 4 无效 5 已完成`）。
///
/// ⚠️ 历史缺陷提示：本模块曾使用 `10/20/30/40/50` 一套自造状态值，与真实库
/// 口径不符——`mark_paid` 会把 `order_status` 写成 `10`，管理端与前台
/// 按 `1`（待发货）过滤时**永远查不到该订单**。已于本次修正为库口径；
/// 任何新增代码请勿再引入非 `0..=5` 的订单状态字面量。
pub mod status {
    /// 待确认 / 待支付
    pub const UNPAID: i32 = 0;
    /// 已确定 / 待发货（已付款）
    pub const PAID: i32 = 1;
    /// 处理中 / 已发货
    pub const SHIPPED: i32 = 2;
    /// 已取消
    pub const CANCELED: i32 = 3;
    /// 无效
    pub const INVALID: i32 = 4;
    /// 已完成
    pub const FINISHED: i32 = 5;
}

#[derive(Clone)]
pub struct OrderService {
    repos: Arc<Repositories>,
    product: ProductService,
    cart: CartService,
}

/// 评价状态过滤归一（Java `OrderListPageQuery.commentStatus`）：
/// `-1` / `None` → 不过滤；`0` → 待评价（仓储按 Java 语义追加 order_status=5）；
/// `n > 0` → 已评价等值。
#[must_use]
pub fn normalize_comment_status(value: Option<i32>) -> Option<i32> {
    value.filter(|v| *v != -1)
}

/// 关键词归一：去首尾空白，空串视为不过滤（Java `StrUtil.isBlank`）。
#[must_use]
pub fn normalize_keyword(keyword: Option<&str>) -> Option<&str> {
    keyword.map(str::trim).filter(|s| !s.is_empty())
}

/// 待晒单过滤归一（Java `OrderListPageQuery.isShowed`）：`-1` / `None` → 不过滤；
/// `0` → 待晒单；`1` → 已晒单。
#[must_use]
pub fn normalize_is_showed(value: Option<i32>) -> Option<i32> {
    value.filter(|v| *v > -1)
}

/// 订单头店铺归属（原版 `OrderCheckServiceImpl` 结算建单口径，我方补齐）：
/// 全部明细同属一家店铺时，订单头 `shop_id` = 该店铺（单店铺分组时
/// Java 在建单时即把头置为该组店铺，见 `OrderCheckServiceImpl` 的
/// `cartList.size() == 1` 分支）；纯平台自营明细时为 0。
/// 跨店混合时原版保持头为 0 并在提交时按店拆子单承载归属——我方拆单
/// 模型尚未裁定，跨店单头暂维持 0，不臆测首店/主店口径。
#[must_use]
pub fn header_shop_id(items: &[OrderItem]) -> i32 {
    let mut iter = items.iter().map(|it| it.shop_id);
    let Some(first) = iter.next() else {
        return 0;
    };
    if iter.all(|shop_id| shop_id == first) {
        first
    } else {
        0
    }
}

/// 订单头配送类型快照（原版 `OrderCheckServiceImpl` 结算建单口径，我方补齐）：
/// Java 在建单时即把结算入参 `deliveryType` 写入订单头
/// `shipping_type_id`（`OrderDeliveryTypeEnum`：1 物流 / 2 自提 / 3 同城），
/// `shipping_type_name` 取枚举描述（单店物流分支若买家选了具体配送方式
/// 取该方式名——我方建单 DTO 无此入参，统一取枚举描述，即 Java 的
/// 兜底分支 `getDesc(deliveryType)`；未知值描述为空串，同 `getDesc`）。
///
/// 此前我方只把 deliveryType 落入 `shipping_method`，`shipping_type_id`
/// 从不写入（INSERT 未含该列、库默认 0），导致同城配送订单头恒 0，
/// 派送单生产者（`on_order_paid` 判 `shipping_type_id == 3`）永不触发。
#[must_use]
pub fn shipping_type_snapshot(delivery_type: i32) -> (i32, &'static str) {
    match delivery_type {
        1 => (1, "物流配送"),
        2 => (2, "到店自提"),
        3 => (3, "同城配送"),
        other => (other, ""),
    }
}

/// 从订单地址快照解析自提点 ID（建单文案 `门店自提 (自提点ID: N)`，
/// 见 `create` 的自提地址快照分支）。无标记 / ID 非法时返回 None。
fn parse_pickup_shop_id(address: &str) -> Option<i32> {
    let marker = "自提点ID:";
    let start = address.find(marker)? + marker.len();
    let digits: String = address[start..]
        .chars()
        .skip_while(|c| *c == ' ')
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse::<i32>().ok().filter(|id| *id > 0)
}

impl OrderService {
    pub fn new(repos: Arc<Repositories>, product: ProductService, cart: CartService) -> Self {
        Self { repos, product, cart }
    }

    pub fn from_repos(repos: Arc<Repositories>) -> Self {
        let product = ProductService::new(repos.clone(), None);
        let cart = CartService::new(repos.clone(), product.clone());
        Self { repos, product, cart }
    }

    /// 创建订单：
    /// 1. 读取勾选购物车行
    /// 2. 校验并扣减库存（不足则整体失败）
    /// 3. 生成订单号并写入 order + order_item
    /// 4. 清理已下单的购物车行
    pub async fn create(&self, user_id: i32, dto: &OrderCreateDTO) -> Result<OrderCreateVO> {
        let lines = self.cart.checked_lines(user_id, &dto.cart_ids).await?;
        if lines.is_empty() {
            return Err(Error::Validation("未选择任何商品".into()));
        }
        // POS 收银：价格以购物车行（POS 改价后）为准，不叠加线上营销活动。
        let pos_mode = dto.order_source.as_deref() == Some("pos");
        // 送礼物模式（我方新增，论坛帖 197742）：POS 收银不适用；
        // 礼物订单不填收货地址、付款后待好友领取，细则见校验段与
        // crate::trade::gift_order。
        let gift_mode = !pos_mode && dto.is_gift_order.unwrap_or(false);
        // 价格门控购买拦截（第 2 期，用户决策 3）：未认证用户不能下单
        // （含门控开启前已留在购物车的行）。POS 收银是商家侧当面交易，
        // 不走前台看价门控口径，豁免。
        if !pos_mode {
            crate::system::price_gate::ensure_purchase_allowed(&self.repos, user_id).await?;
        }

        // 金额计算
        let mut product_amount = Decimal::ZERO;
        let mut items: Vec<OrderItem> = Vec::with_capacity(lines.len());
        let mut order_sn = utils::generate_order_sn();
        // 礼品卡兑换流：购物车行 extra_data 的 giftCardsId 标记（useCard 写入）
        let mut gift_exchange_card_id: Option<i32> = None;
        // 礼品卡适用范围（我方新增）：主循环逐行收集 (商品id, 品类id, 行小计)，
        // 储值卡抵扣时按范围内金额限定基数（赠品行金额为 0 不入列）
        let mut gift_scope_lines: Vec<(i32, i32, Decimal)> = Vec::new();

        // 订单号防重
        let mut retries = 0;
        while self.repos.trade.order.sn_exists(&order_sn).await? && retries < 5 {
            order_sn = utils::generate_order_sn();
            retries += 1;
        }

        // 加载商品与规格，按同一口径计算购物车、结算和提交订单金额。
        let product_ids: Vec<i32> = lines.iter().map(|l| l.product_id).collect();
        let products = crate::trade::check::load_products(&self.repos, &product_ids).await?;

        // 店铺小程序作用域（我方新增）：整单复检——任一行商品不属于
        // 本店即整单拒绝（购物车跨端共享，历史行可能含他店商品；
        // POS 收银是商家侧当面交易且无小程序作用域，天然不命中）。
        if let Some(bound_shop_id) =
            nextrade_infrastructure::db::repo::member::mini_shop_scope::MiniShopScope::current()
                .shop_id()
        {
            let foreign = lines.iter().any(|l| {
                products
                    .get(&l.product_id)
                    .is_some_and(|p| p.shop_id != bound_shop_id)
            });
            if foreign {
                return Err(Error::biz(
                    1001,
                    "订单中包含不属于当前店铺的商品，请移除后再提交",
                ));
            }
        }

        // 客户黑名单（下单限制）：checked_lines 后、算价前整单复检（§4.3），
        // 任一行命中即整单拒绝，文案点名命中店铺（blacklist-design §7）。
        {
            let pairs: Vec<(i32, i32)> = lines
                .iter()
                .map(|l| {
                    (
                        l.product_id,
                        products.get(&l.product_id).map_or(l.shop_id, |p| p.shop_id),
                    )
                })
                .collect();
            let blacklist =
                crate::member::user_blacklist::UserBlacklistService::new(self.repos.clone());
            let hits = blacklist.find_order_restrictions(user_id, &pairs).await?;
            if !hits.is_empty() {
                let mut shop_ids: Vec<i32> = Vec::new();
                for hit in &hits {
                    if !shop_ids.contains(&hit.shop_id) {
                        shop_ids.push(hit.shop_id);
                    }
                }
                let titles = blacklist.shop_titles(&shop_ids).await?;
                let names = shop_ids
                    .iter()
                    .map(|id| {
                        if *id == 0 {
                            "平台自营".to_string()
                        } else {
                            titles
                                .get(id)
                                .cloned()
                                .unwrap_or_else(|| format!("店铺{id}"))
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("、");
                return Err(Error::biz(
                    1001,
                    format!(
                        "订单中包含暂时无法购买的商品（{names}），请移除后再提交；如有疑问请联系客服"
                    ),
                ));
            }
        }
        // 渠道可见性整单复检（我方新增，论坛帖 507800）：线上单含「仅 POS
        // 线下」商品、POS 单含「仅线上」商品即整单拒绝。列表/搜索/详情/
        // 加购已按渠道隐藏或拦截，此为下单接口兜底（防历史购物车行与
        // 直调接口绕过前台口径）。
        for line in &lines {
            if let Some(p) = products.get(&line.product_id) {
                if crate::catalog::product::channel_blocks_order(pos_mode, p.sale_channel) {
                    return Err(Error::biz(
                        1001,
                        format!(
                            "商品「{}」不在当前渠道销售，请移除后再提交",
                            p.product_name
                        ),
                    ));
                }
            }
        }
        let skus = super::cart::load_cart_skus(&self.repos, &lines).await?;
        let mut store_amounts: std::collections::BTreeMap<i32, Decimal> =
            std::collections::BTreeMap::new();

        // 秒杀活动价（与结算页同一映射）
        let now_activity = utils::now_seconds() as i32;
        let seckill = if pos_mode {
            std::collections::HashMap::new()
        } else {
            crate::trade::check::load_seckill_prices(&self.repos, &lines, now_activity).await?
        };
        // 拼团活动价（购物车 extra_data 携带 grouponData）
        let groupon_prices = if pos_mode {
            std::collections::HashMap::new()
        } else {
            crate::trade::check::load_groupon_prices(&self.repos, &lines, i64::from(now_activity))
                .await?
        };
        // 起售量整单复检（合并同商品行后判定）：同一商品跨 SKU/跨行
        // 合计数量须 >= 起售量；礼品卡兑换行与拼团行不计入（非普通购买）。
        {
            let mut totals: std::collections::HashMap<i32, i32> =
                std::collections::HashMap::new();
            for line in &lines {
                let is_gift_exchange = line.extra_data.as_deref().is_some_and(|raw| {
                    raw.contains("giftCardsId")
                });
                if is_gift_exchange || groupon_prices.contains_key(&line.cart_id) {
                    continue;
                }
                *totals.entry(line.product_id).or_default() += line.quantity.max(1);
            }
            for (pid, total) in &totals {
                if let Some(p) = products.get(pid) {
                    crate::catalog::product::ensure_min_buy_count(
                        &p.product_name,
                        p.min_buy_count,
                        *total,
                    )?;
                }
            }
        }
        let mut seckill_discount = Decimal::ZERO;
        // (rec_id, 数量, 商品ID, 活动开始, 活动结束, 活动ID)
        let mut seckill_lines: Vec<(i32, i32, i32, i32, i32, i32)> = Vec::new();
        // 拼团明细：(商品名, 拼团价信息)
        let mut groupon_lines: Vec<(String, crate::trade::check::GrouponPrice)> = Vec::new();

        // 代客跨店履约行价（我方新增）：cart_id → 履约门店单价。
        // 覆盖行为最终价，不叠加秒杀/拼团/限时折扣/满减（见 proxy_order_ext）。
        let empty_line_overrides: std::collections::HashMap<i32, Decimal> =
            std::collections::HashMap::new();
        let line_overrides: &std::collections::HashMap<i32, Decimal> = dto
            .line_price_overrides
            .as_ref()
            .unwrap_or(&empty_line_overrides);

        // 预计算活动单价（秒杀/拼团），再应用满减/满折得到最终结算价
        let mut line_prices: std::collections::HashMap<i32, Decimal> =
            std::collections::HashMap::new();
        for line in &lines {
            if let Some(override_price) = line_overrides.get(&line.cart_id) {
                line_prices.insert(line.cart_id, *override_price);
                continue;
            }
            let pid = line.product_id;
            let sku_id = line.sku_id.unwrap_or(0);
            let base_price = if pos_mode {
                line.original_price
            } else {
                super::cart::cart_base_price(
                    products.get(&pid), super::cart::cart_line_sku(&skus, line), line,
                )
            };
            let mut price = if pos_mode {
                base_price
            } else if let Some(g) = groupon_prices.get(&line.cart_id) {
                g.price.min(base_price)
            } else {
                seckill
                    .get(&(pid, sku_id))
                    .or_else(|| seckill.get(&(pid, 0)))
                    .and_then(|item| item.seckill_price)
                    .map(|p| p.min(base_price))
                    .unwrap_or(base_price)
            };
            if !pos_mode {
                // 限时折扣：在秒杀/拼团价基础上取更低者
                let time_service =
                    crate::marketing::time_discount::TimeDiscountService::new(self.repos.clone());
                if let Some(time_price) = time_service
                    .current_discount_price(pid, line.sku_id, price)
                    .await?
                {
                    price = time_price.min(price);
                }
            }
            line_prices.insert(line.cart_id, price);
        }
        let user_rank_id = self
            .repos
            .member
            .user
            .find_by_id(user_id)
            .await?
            .map(|u| u.rank_id);
        // 跨店履约行不参与满减/满赠：满减入参剔除覆盖行（无覆盖时原样）。
        let fr_lines: Vec<nextrade_core::entities::Cart>;
        let fr_input: &[nextrade_core::entities::Cart] = if line_overrides.is_empty() {
            &lines
        } else {
            fr_lines = lines
                .iter()
                .filter(|l| !line_overrides.contains_key(&l.cart_id))
                .cloned()
                .collect();
            &fr_lines
        };
        let applied_gifts = if pos_mode {
            Vec::new()
        } else {
            crate::trade::check::apply_full_reductions(
                &self.repos,
                fr_input,
                &products,
                &mut line_prices,
                now_activity,
                user_rank_id,
            )
            .await?
            .2
        };

        for line in &lines {
            let pid = line.product_id;
            let sku_id = line.sku_id.unwrap_or(0);
            let qty = line.quantity.max(1);
            let base_price = if pos_mode {
                line.original_price
            } else {
                super::cart::cart_base_price(
                    products.get(&pid), super::cart::cart_line_sku(&skus, line), line,
                )
            };
            // 跨店履约行（我方新增）：不挂秒杀/拼团活动语义。
            let is_override_line = line_overrides.contains_key(&line.cart_id);
            let seckill_item = if is_override_line {
                None
            } else {
                seckill
                    .get(&(pid, sku_id))
                    .or_else(|| seckill.get(&(pid, 0)))
            };
            let groupon_line = if is_override_line {
                None
            } else {
                groupon_prices.get(&line.cart_id)
            };
            let (activity_price, is_pin) = if let Some(g) = groupon_line {
                (g.price.min(base_price), true)
            } else {
                (
                    seckill_item
                        .and_then(|item| item.seckill_price)
                        .map(|p| p.min(base_price))
                        .unwrap_or(base_price),
                    false,
                )
            };
            // 满减/满折后的最终单价
            let merchandise_price = line_prices
                .get(&line.cart_id)
                .copied()
                .unwrap_or(activity_price);
            let price = merchandise_price + if pos_mode { Decimal::ZERO } else { super::cart::cart_extra_price(line) };
            let subtotal = price * Decimal::from(qty);
            product_amount += subtotal;
            gift_scope_lines.push((
                pid,
                products.get(&pid).map(|p| p.category_id).unwrap_or(0),
                subtotal,
            ));
            // 跨店履约行与基准价的差额不是活动优惠，不计入 seckill_discount。
            if !is_override_line {
                seckill_discount +=
                    (base_price - merchandise_price).max(Decimal::ZERO) * Decimal::from(qty);
            }
            *store_amounts.entry(line.shop_id).or_default() += subtotal;
            if let Some(g) = groupon_line {
                groupon_lines.push((
                    products
                        .get(&pid)
                        .map(|p| p.product_name.clone())
                        .unwrap_or_default(),
                    g.clone(),
                ));
            } else if let Some(item) = seckill_item {
                if activity_price < base_price {
                    if let (Some(rec_id), Some(start), Some(end)) =
                        (Some(item.rec_id), item.seckill_start_time, item.seckill_end_time)
                    {
                        seckill_lines.push((
                            rec_id,
                            qty,
                            pid,
                            start,
                            end,
                            item.seckill_id.unwrap_or(0),
                        ));
                    }
                }
            }

            let promotion_data = groupon_line
                .and_then(|g| serde_json::to_string(&g.data).ok());
            let product = products.get(&pid);
            // 礼品卡兑换行：关联 order_item.gift_card_id（原版兑换订单口径）
            let line_gift_card_id = line
                .extra_data
                .as_deref()
                .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
                .and_then(|v| v.get("giftCardsId").and_then(serde_json::Value::as_i64))
                .and_then(|v| i32::try_from(v).ok());
            if line_gift_card_id.is_some() && gift_exchange_card_id.is_none() {
                gift_exchange_card_id = line_gift_card_id;
            }
            items.push(OrderItem {
                order_sn: order_sn.clone(),
                user_id,
                price,
                quantity: qty,
                product_id: pid,
                product_name: product
                    .map(|p| p.product_name.clone())
                    .unwrap_or_default(),
                product_sn: product.map(|p| p.product_sn.clone()).unwrap_or_default(),
                pic_thumb: product.map(|p| p.pic_thumb.clone()).unwrap_or_default(),
                product_type: product.and_then(|p| p.product_type).unwrap_or(1),
                sku_id,
                sku_data: line.sku_data.clone().unwrap_or_default(),
                origin_price: Some(base_price),
                shop_id: line.shop_id,
                vendor_id: product.and_then(|p| p.vendor_id),
                is_pin: i32::from(is_pin),
                promotion_data,
                gift_card_id: line_gift_card_id,
                ..Default::default()
            });
        }

        // 满赠赠品行（价格 0，isGift=1；库存扣减在事务内单独处理）
        for gift in &applied_gifts {
            let product = products.get(&gift.product_id);
            items.push(OrderItem {
                order_sn: order_sn.clone(),
                user_id,
                price: Decimal::ZERO,
                quantity: gift.num,
                product_id: gift.product_id,
                product_name: product
                    .map(|p| p.product_name.clone())
                    .unwrap_or_default(),
                product_sn: product.map(|p| p.product_sn.clone()).unwrap_or_default(),
                pic_thumb: product.map(|p| p.pic_thumb.clone()).unwrap_or_default(),
                product_type: product.and_then(|p| p.product_type).unwrap_or(1),
                sku_id: gift.sku_id,
                sku_data: String::new(),
                origin_price: Some(Decimal::ZERO),
                shop_id: gift.shop_id,
                vendor_id: product.and_then(|p| p.vendor_id),
                is_gift: 1,
                promotion_data: Some(
                    serde_json::json!({
                        "giftId": gift.gift_id,
                        "promotionId": gift.promotion_id,
                        "type": 4,
                    })
                    .to_string(),
                ),
                ..Default::default()
            });
        }

        // 秒杀限购校验（按活动窗口内该用户已购数量）
        for (_, qty, pid, start, end, seckill_id) in &seckill_lines {
            if *seckill_id <= 0 {
                continue;
            }
            if let Some(record) = self.repos.marketing.seckill.find_by_id(*seckill_id).await? {
                if record.seckill_limit_num.unwrap_or(0) > 0 {
                    let bought = self
                        .repos
                        .marketing
                        .seckill
                        .count_user_purchases(user_id, *pid, *start, *end)
                        .await?;
                    if bought + i64::from(*qty) > i64::from(record.seckill_limit_num.unwrap_or(0)) {
                        return Err(Error::biz(1001, "超过秒杀商品限购数量".to_string()));
                    }
                }
            }
        }

        // —— 收货地址快照（同时取地区用于运费模板匹配）——
        // 送礼物模式刻意不取买家地址：订单先落空快照，领取时由领取人
        // 地址覆盖（运费按无地区口径计算，与未选地址的普通单一致）。
        let effective_shipping_method = dto.shipping_method.or(dto.delivery_type).unwrap_or(1);
        let addr = match if gift_mode {
            None
        } else {
            dto.address_id.filter(|id| *id > 0)
        } {
            Some(address_id) => {
                self.repos
                    .member
                    .user_address_client
                    .find_by_id(address_id)
                    .await?
            }
            None => None,
        };
        let (consignee, mobile, email, address, region_ids, region_names, region_id_list) =
            match addr {
                Some(a) => {
                    let ids: Vec<i32> =
                        serde_json::from_str(&a.region_ids).unwrap_or_default();
                    (
                        a.consignee,
                        a.mobile,
                        a.email.unwrap_or_default(),
                        a.address,
                        a.region_ids,
                        a.region_names,
                        ids,
                    )
                }
                None => {
                    if effective_shipping_method == 2 {
                        let contact_name = dto
                            .pickup_contact_name
                            .as_deref()
                            .unwrap_or("自提客户")
                            .to_string();
                        let contact_mobile = dto
                            .pickup_contact_mobile
                            .as_deref()
                            .unwrap_or_default()
                            .to_string();
                        let pickup_addr =
                            format!("门店自提 (自提点ID: {})", dto.pickup_id.unwrap_or(0));
                        (
                            contact_name,
                            contact_mobile,
                            String::new(),
                            pickup_addr,
                            "[]".to_string(),
                            "[]".to_string(),
                            Vec::new(),
                        )
                    } else {
                        (
                            String::new(),
                            String::new(),
                            String::new(),
                            String::new(),
                            "[]".to_string(),
                            "[]".to_string(),
                            Vec::new(),
                        )
                    }
                }
            };

        // —— 优惠券抵扣（店铺范围 + 门槛校验，与结算页同口径）——
        // 前端结算表单提交 `useCouponIds`；兼容旧字段 `couponId`。
        let coupon_used: Vec<i32> = dto
            .use_coupon_ids
            .clone()
            .filter(|ids| !ids.is_empty())
            .or_else(|| dto.coupon_id.filter(|id| *id > 0).map(|id| vec![id]))
            .unwrap_or_default();
        let mut coupon_amount = Decimal::ZERO;
        let mut valid_coupons: Vec<i32> = Vec::new();
        // 逐券明细（coupon_id, 券归属店铺, 本券抵扣额）：下单后落
        // order_coupon_detail / order_amount_detail，作拆单金额明细数据源。
        let mut coupon_details: Vec<(i32, i32, Decimal)> = Vec::new();
        if !coupon_used.is_empty() {
            let coupons = self
                .repos
                .marketing
                .coupon
                .find_by_ids(&coupon_used)
                .await?;
            let by_id: std::collections::HashMap<i32, nextrade_core::entities::Coupon> = coupons
                .into_iter()
                .map(|c| (c.coupon_id, c))
                .collect();
            for coupon_id in &coupon_used {
                let coupon = by_id.get(coupon_id).ok_or_else(|| {
                    Error::biz(1001, format!("优惠券不存在: {coupon_id}"))
                })?;
                let shop_id = coupon.shop_id.unwrap_or(0);
                let base = if shop_id > 0 {
                    store_amounts.get(&shop_id).copied().unwrap_or(Decimal::ZERO)
                } else {
                    product_amount
                };
                if base <= Decimal::ZERO {
                    return Err(Error::biz(1001, "优惠券不适用于当前店铺商品".to_string()));
                }
                if base < coupon.min_order_amount {
                    return Err(Error::biz(1001, "优惠券未达到使用门槛".to_string()));
                }
                let this_discount = nextrade_core::models::trade::check::coupon_rule_discount(
                    coupon.coupon_type,
                    coupon.coupon_money,
                    coupon.coupon_discount,
                    coupon.max_order_amount,
                    base,
                );
                coupon_amount += this_discount;
                coupon_details.push((*coupon_id, shop_id, this_discount));
                valid_coupons.push(*coupon_id);
            }
        }

        // —— 运费模板（与结算页同一函数）——
        let shipping = crate::trade::check::calculate_shipping(
            &self.repos,
            &lines,
            &products,
            &region_id_list,
            if effective_shipping_method == 2 {
                Some(2)
            } else {
                None
            },
        )
        .await?;
        let shipping_fee = shipping.total;

        // —— 积分抵扣（与结算页同一函数）——
        let user = self
            .repos
            .member
            .user
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| Error::biz_msg("用户不存在"))?;
        let cfg = self
            .repos
            .trade
            .check
            .find_config_values(&[
                "integralScale",
                "integralPercent",
                "integralPercentBaseAmount",
                "usePoints",
            ])
            .await?;
        let config_enabled = |key: &str| {
            cfg.get(key).map(|v| v.trim() == "1").unwrap_or(false)
        };
        let decimal_cfg = |key: &str, default: Decimal| {
            cfg.get(key)
                .and_then(|v| v.trim().parse::<Decimal>().ok())
                .unwrap_or(default)
        };
        let points = if config_enabled("usePoints") {
            nextrade_core::models::trade::check::calc_points(
                dto.use_points.unwrap_or(0),
                user.points,
                (product_amount - coupon_amount).max(Decimal::ZERO),
                decimal_cfg("integralScale", Decimal::ZERO),
                decimal_cfg("integralPercent", Decimal::ZERO),
                decimal_cfg("integralPercentBaseAmount", Decimal::ONE_HUNDRED),
            )
        } else {
            Default::default()
        };
        let points_amount = points.points_amount;

        // —— 地区税率（与结算页同一函数）——
        let (tax_rate, _) =
            crate::trade::check::resolve_tax_rate(&self.repos, &region_id_list).await;
        let taxable_base = (product_amount - coupon_amount - seckill_discount).max(Decimal::ZERO);
        let tax_rate_amount = crate::trade::check::calc_tax_amount(taxable_base, tax_rate);

        // 应付金额 = 商品 - 优惠券 + 运费 + 税额 - 积分抵扣（与结算页 totalAmount 一致）
        let total = (product_amount - coupon_amount + shipping_fee + tax_rate_amount - points_amount)
            .max(Decimal::ZERO);

        // —— 定金预售：单商品 + product_presale.sale_mode = 2 ——
        let now_ts = utils::now_seconds() as i32;
        let mut presale_row: Option<nextrade_core::entities::OrderPresale> = None;
        let mut deposit_now = total;
        if lines.len() == 1 {
            let product_id = lines[0].product_id;
            if let Some(product) = products.get(&product_id) {
                if product.sale_mode == 2 {
                    if let Some(ps) = self
                        .repos
                        .catalog
                        .product_manage
                        .find_presale(product_id)
                        .await?
                    {
                        let deposit = nextrade_core::models::trade::check::calc_deposit(
                            total,
                            ps.deposit_type,
                            ps.deposit_ratio,
                            ps.deposit_fixed_amount,
                            ps.is_round_down,
                        );
                        if deposit > Decimal::ZERO {
                            deposit_now = deposit;
                            presale_row = Some(nextrade_core::entities::OrderPresale {
                                presale_order_id: 0,
                                order_id: 0,
                                order_sn: order_sn.clone(),
                                product_id,
                                sale_mode: 2,
                                deposit_amount: deposit,
                                final_payment_amount: (total - deposit).max(Decimal::ZERO),
                                deposit_paid_time: None,
                                final_payment_start_time: None,
                                final_payment_paid_time: None,
                                presale_status: 0,
                                delivery_days: if ps.delivery_type == 1 {
                                    Some(ps.delivery_relative_days.unwrap_or(0))
                                } else {
                                    None
                                },
                                delivery_date: if ps.delivery_type == 2 {
                                    ps.delivery_date
                                } else {
                                    None
                                },
                                create_time: now_ts,
                                update_time: now_ts,
                            });
                        }
                    }
                }
            }
        }

        // —— 送礼物模式拦截校验（我方新增）：只接实物普通订单 ——
        // 活动订单（拼团/秒杀/预售）、礼品卡兑换、虚拟/卡券/付费内容
        // 商品、满赠赠品行一律不纳入，提交时明确提示而非静默降级。
        let gift_message: String = dto
            .gift_message
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_string();
        if gift_mode {
            if gift_message.chars().count() > 200 {
                return Err(Error::Validation("赠言不能超过200字".into()));
            }
            if effective_shipping_method == 2 {
                return Err(Error::biz(
                    400,
                    "送礼物订单需由好友填写收货地址，不支持到店自提".to_string(),
                ));
            }
            if gift_exchange_card_id.is_some() {
                return Err(Error::biz(
                    400,
                    "礼品卡兑换订单不能使用送礼物模式".to_string(),
                ));
            }
            if !groupon_lines.is_empty() {
                return Err(Error::biz(
                    400,
                    "拼团订单不能使用送礼物模式".to_string(),
                ));
            }
            if !seckill_lines.is_empty() {
                return Err(Error::biz(
                    400,
                    "秒杀订单不能使用送礼物模式".to_string(),
                ));
            }
            if presale_row.is_some() {
                return Err(Error::biz(
                    400,
                    "预售订单不能使用送礼物模式".to_string(),
                ));
            }
            if items.iter().any(|it| it.is_gift == 1) {
                return Err(Error::biz(
                    400,
                    "含活动赠品的订单不能使用送礼物模式".to_string(),
                ));
            }
            if items.iter().any(|it| it.product_type != 1) {
                return Err(Error::biz(
                    400,
                    "虚拟/卡券类商品不能使用送礼物模式，仅支持实物商品".to_string(),
                ));
            }
        }

        let order = Order {
            is_presale: if presale_row.is_some() { 1 } else { 0 },
            presale_type: presale_row.as_ref().map(|_| 2),
            // 订单类型：礼品卡兑换 12（原版 orderType=12）> 拼团 2 > 普通 0
            order_type: if gift_exchange_card_id.is_some() {
                Some(12)
            } else if groupon_lines.is_empty() {
                Some(0)
            } else {
                Some(2)
            },
            order_sn: order_sn.clone(),
            user_id,
            order_status: status::UNPAID,
            shipping_status: 0,
            pay_status: 1, // 1=未支付
            add_time: utils::now_seconds() as i32,
            buyer_note: dto.buyer_note.clone().unwrap_or_default(),
            shipping_method: effective_shipping_method,
            // 原版建单口径：配送类型同步写入订单头 shipping_type_id/name
            // （同城=3 是派送单生产者的触发条件，见 shipping_type_snapshot）。
            shipping_type_id: shipping_type_snapshot(effective_shipping_method).0,
            shipping_type_name: shipping_type_snapshot(effective_shipping_method)
                .1
                .to_string(),
            pay_type_id: dto.pay_type_id.unwrap_or(0),
            consignee,
            mobile,
            email,
            address,
            region_ids,
            region_names,
            use_points: points.used_points,
            points_amount,
            shop_id: header_shop_id(&items),
            vendor_id: crate::trade::order_split::header_vendor_id(&items),
            is_valid: 1,
            is_del: 0,
            total_amount: total,
            paid_amount: Decimal::ZERO,
            unpaid_amount: deposit_now,
            product_amount,
            coupon_amount,
            shipping_fee,
            discount_amount: seckill_discount,
            tax_rate: Some(tax_rate),
            tax_rate_amount: Some(tax_rate_amount),
            order_source: dto.order_source.clone().unwrap_or_else(|| "web".into()),
            pos_user_id: dto.pos_user_id,
            ..Default::default()
        };

        // 「扣库存 + 写订单」同一事务：任一行库存不足或订单写入失败，
        // 事务整体回滚，不残留已扣库存（原 Java 由 @Transactional 保证）。
        let mut groupon_record_id: Option<i64> = None;
        // 送礼物领取 token 明文（事务内生成，仅随建单响应返回一次）
        let mut gift_token: Option<String> = None;

        let order_id = {
            let mut tx = self
                .repos.trade.order
                .begin()
                .await
                .map_err(|e| Error::Database(e.to_string()))?;

            let mut deducted: Vec<(i32, Option<i32>, i32)> = Vec::with_capacity(items.len());
            let mut result: Result<i32> = Ok(0);

            for (idx, item) in items.iter().enumerate() {
                let pid = item.product_id;
                let qty = item.quantity;
                if item.is_gift == 1 {
                    let gift_id = item
                        .promotion_data
                        .as_deref()
                        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
                        .and_then(|v| v.get("giftId").and_then(|g| g.as_i64()))
                        .and_then(|g| i32::try_from(g).ok())
                        .unwrap_or(0);
                    match self
                        .repos
                        .marketing
                        .product_gift
                        .dec_gift_stock_in_tx(&mut tx, gift_id, qty)
                        .await
                    {
                        Ok(true) => {}
                        Ok(false) => {
                            result = Err(Error::InsufficientStock {
                                product_id: pid.to_string(),
                                stock: 0,
                            });
                            break;
                        }
                        Err(e) => {
                            result = Err(e);
                            break;
                        }
                    }
                    continue;
                }
                // 无 SKU 商品（sku_id 缺省）必须按 product_stock 扣减：
                // 传 Some(0) 会 UPDATE product_sku WHERE sku_id=0 → 0 行 → 误报库存不足。
                let sku = lines.get(idx).and_then(|l| l.sku_id);
                match self
                    .repos.catalog.product
                    .deduct_stock_in_tx(&mut tx, pid, sku, qty)
                    .await
                {
                    Ok(true) => deducted.push((pid, sku, qty)),
                    Ok(false) => {
                        result = Err(Error::InsufficientStock {
                            product_id: pid.to_string(),
                            stock: 0,
                        });
                        break;
                    }
                    Err(e) => {
                        result = Err(e);
                        break;
                    }
                }
            }

            if result.is_ok() {
                result = self
                    .repos.trade.order
                    .insert_order_in_tx(&mut tx, &order, &items)
                    .await;
            }

            // 拼团记录（与订单同事务；开团/参团状态机）
            if result.is_ok() && !groupon_lines.is_empty() {
                if let Some(order_id) = result.as_ref().ok().copied() {
                    for (product_name, line) in &groupon_lines {
                        match crate::trade::groupon::create_record_in_tx(
                            &mut tx,
                            &self.repos,
                            user_id,
                            &user.username,
                            order_id,
                            &order_sn,
                            product_name,
                            line,
                        )
                        .await
                        {
                            Ok(id) => {
                                if groupon_record_id.is_none() {
                                    groupon_record_id = Some(id);
                                }
                            }
                            Err(e) => {
                                result = Err(e);
                                break;
                            }
                        }
                    }
                }
            }

            // 秒杀库存扣减（与订单同事务，失败整体回滚）
            if result.is_ok() {
                for (rec_id, qty, _, _, _, _) in &seckill_lines {
                    match self
                        .repos
                        .marketing
                        .seckill
                        .dec_stock_inc_sales_in_tx(&mut tx, *rec_id, *qty)
                        .await
                    {
                        Ok(true) => {}
                        Ok(false) => {
                            result = Err(Error::biz(1001, "秒杀库存不足".to_string()));
                            break;
                        }
                        Err(e) => {
                            result = Err(e);
                            break;
                        }
                    }
                }
            }

            // 预售订单信息与订单同事务写入，避免「订单在、预售信息丢」
            if result.is_ok() {
                if let Some(mut row) = presale_row.clone() {
                    if let Some(order_id) = result.as_ref().ok() {
                        row.order_id = *order_id;
                        result = self
                            .repos
                            .payment
                            .pay
                            .insert_presale_in_tx(&mut tx, &row)
                            .await
                            .map(|_| row.order_id);
                    }
                }
            }

            // 随单发票申请与订单同事务写入（order_invoice + order.invoice_data）
            if result.is_ok() {
                if let (Some(order_id), Some(ref inv)) =
                    (result.as_ref().ok().copied(), &dto.invoice_data)
                {
                    if let Some(inv_type) = inv.invoice_type {
                        if inv_type > 0 {
                            let company_name = inv
                                .company_name
                                .as_deref()
                                .or(inv.title.as_deref())
                                .unwrap_or("个人");
                            let company_code = inv.tax_no.as_deref();
                            let title_type = if inv_type == 2 {
                                2
                            } else {
                                inv.title_type.unwrap_or(1)
                            };
                            match self
                                .repos
                                .trade
                                .order_invoice_client
                                .insert_invoice_in_tx(
                                    &mut tx,
                                    user_id,
                                    order_id,
                                    inv_type,
                                    title_type,
                                    company_name,
                                    company_code,
                                    None,
                                    inv.tax_phone.as_deref(),
                                    None,
                                    None,
                                    inv.content.as_deref(),
                                    total,
                                    inv.mobile.as_deref(),
                                    inv.email.as_deref(),
                                    now_ts,
                                )
                                .await
                            {
                                Ok(inv_id) => {
                                    let invoice_json = serde_json::json!({
                                        "id": inv_id,
                                        "orderId": order_id,
                                        "userId": user_id,
                                        "invoiceType": inv_type,
                                        "titleType": title_type,
                                        "companyName": company_name,
                                        "companyCode": company_code.unwrap_or(""),
                                        "amount": total.to_string(),
                                        "status": 0,
                                        "addTime": now_ts.to_string(),
                                    })
                                    .to_string();
                                    if let Err(e) = self
                                        .repos
                                        .trade
                                        .order
                                        .update_invoice_data_in_tx(&mut tx, order_id, &invoice_json)
                                        .await
                                    {
                                        result = Err(e);
                                    }
                                }
                                Err(e) => {
                                    result = Err(e);
                                }
                            }
                        }
                    }
                }
            }

            // 送礼物行与订单同事务写入（order_gift，迁移 014）：
            // 「有订单必有礼物行」原子成立，token 明文仅在此返回一次
            if result.is_ok() && gift_mode {
                if let Some(oid) = result.as_ref().ok().copied() {
                    match crate::trade::gift_order::GiftOrderService::insert_in_tx(
                        &mut tx,
                        oid,
                        &order_sn,
                        user_id,
                        &gift_message,
                        now_ts,
                    )
                    .await
                    {
                        Ok(token) => gift_token = Some(token),
                        Err(e) => {
                            result = Err(e);
                        }
                    }
                }
            }

            match result {
                Ok(id) => {
                    tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
                    id
                }
                Err(e) => {
                    // 事务回滚即撤消本事务内已扣的库存，无需手工补偿
                    let _ = tx.rollback().await;
                    // 库存被回滚，需失效详情缓存避免读到脏数据
                    for (pid, _, _) in &deducted {
                        self.product.invalidate_detail_cache(*pid).await;
                    }
                    return Err(e);
                }
            }
        };

        for item in items.iter_mut() {
            item.order_id = order_id;
        }

        // —— 店铺金额明细 + 优惠券明细落库（拆单数据源，对应原版结算
        // creatOrderAmountDetail / saveOrderCouponDetails）——
        // 尽力而为：失败仅告警，拆单侧有按商品额分摊的兜底口径。
        // 平台券与秒杀折扣无店铺归属，按店铺商品额分摊写入明细行，
        // 保证 Σ明细 == 父单总额（最后一店承接尾差）。
        {
            fn split_by_product(
                total: Decimal,
                store_amounts: &std::collections::BTreeMap<i32, Decimal>,
                product_amount: Decimal,
            ) -> std::collections::BTreeMap<i32, Decimal> {
                let mut out = std::collections::BTreeMap::new();
                let mut rest = total;
                let n = store_amounts.len();
                for (idx, (shop, amt)) in store_amounts.iter().enumerate() {
                    let share = if idx + 1 == n {
                        rest
                    } else if product_amount > Decimal::ZERO {
                        let s = (total * *amt / product_amount).round_dp(2);
                        rest -= s;
                        s
                    } else {
                        Decimal::ZERO
                    };
                    out.insert(*shop, share);
                }
                out
            }
            let admin_repo = &self.repos.trade.order_admin;
            let platform_coupon_total: Decimal = coupon_details
                .iter()
                .filter(|(_, s, _)| *s == 0)
                .map(|(_, _, a)| *a)
                .sum();
            let platform_shares =
                split_by_product(platform_coupon_total, &store_amounts, product_amount);
            let discount_shares =
                split_by_product(seckill_discount, &store_amounts, product_amount);
            for shop_id in store_amounts.keys() {
                let shop_coupon: Decimal = coupon_details
                    .iter()
                    .filter(|(_, s, _)| s == shop_id)
                    .map(|(_, _, a)| *a)
                    .sum();
                let detail = nextrade_core::entities::OrderAmountDetail {
                    order_discount_detail_id: 0,
                    order_id: Some(order_id),
                    shop_id: Some(*shop_id),
                    shipping_fee: Some(
                        shipping.per_store.get(shop_id).copied().unwrap_or_default(),
                    ),
                    discount_amount: Some(
                        discount_shares.get(shop_id).copied().unwrap_or_default(),
                    ),
                    coupon_amount: Some(
                        shop_coupon + platform_shares.get(shop_id).copied().unwrap_or_default(),
                    ),
                    time_discount_amount: Some(Decimal::ZERO),
                };
                if let Err(e) = admin_repo.insert_amount_detail(&detail).await {
                    tracing::warn!("订单 {order_id} 店铺金额明细写入失败: {e}");
                }
            }
            for (coupon_id, shop_id, amount) in &coupon_details {
                let row = nextrade_core::entities::OrderCouponDetail {
                    order_coupon_detail_id: 0,
                    order_id: Some(order_id),
                    shop_id: Some(*shop_id),
                    coupon_id: Some(*coupon_id),
                    coupon_fee: Some(*amount),
                };
                if let Err(e) = admin_repo.insert_coupon_detail(&row).await {
                    tracing::warn!("订单 {order_id} 优惠券明细写入失败: {e}");
                }
            }
        }

        // 清理已下单购物车行
        let cart_ids: Vec<i32> = lines.iter().map(|l| l.cart_id).collect();
        if let Err(e) = self.cart.remove(user_id, &cart_ids).await {
            tracing::warn!("清理购物车失败: {e}");
        }

        // 占用优惠券（尽力而为；失败仅告警，避免订单已建但接口失败）
        if !valid_coupons.is_empty() {
            if let Err(e) = crate::marketing::coupon::CouponService::new(self.repos.clone())
                .use_coupons(user_id, &valid_coupons, order_id)
                .await
            {
                tracing::warn!("订单 {order_id} 优惠券占用失败: {e}");
            }
        }

        // 扣减积分（与结算页口径一致；失败仅告警，避免已建订单接口失败）。
        // 注：订单事务已提交，此处 DB 异常同样只告警——曾用 `?` 上抛，
        // 会造成「接口报错但订单已建」的响应面异常（验收实测 submit
        // 多次响应错误但订单已落库的成因之一），与本函数其余提交后
        // 步骤（清购物车/占用优惠券/礼品卡/流水）的 best-effort 口径对齐。
        if points.used_points > 0 {
            match self
                .repos
                .member
                .user
                .try_debit_points(user_id, points.used_points)
                .await
            {
                Ok(true) => {}
                Ok(false) => {
                    tracing::warn!(
                        "订单 {order_id} 积分扣减失败: user={user_id}, points={}",
                        points.used_points
                    );
                }
                Err(e) => {
                    tracing::warn!("订单 {order_id} 积分扣减异常（订单已建，跳过）: {e}");
                }
            }
        }

        // 扣减成功，失效商品详情缓存
        for line in &lines {
            self.product.invalidate_detail_cache(line.product_id).await;
        }

        tracing::info!("创建订单成功: order_id={order_id}, sn={order_sn}, user={user_id}");

        // —— 礼品卡（标准对齐 D5/D6）：在余额之前抵扣 ——
        // 兑换流（购物车行带 giftCardsId）：卡已在 useCard 核销，商品金额由卡
        // 覆盖，只记 gift_card_amount（不扣储值余额、不写 usage_log）。
        // 普通流：所选储值卡在卡仓储事务内行锁扣 balance 并写 usage_log
        // type=1，失败（卡已不可用/余额为 0）仅告警不回滚订单。
        let mut gift_card_used = Decimal::ZERO;
        if presale_row.is_none() {
            if gift_exchange_card_id.is_some() {
                gift_card_used =
                    (product_amount - coupon_amount - points.points_amount).max(Decimal::ZERO);
            } else {
                let (sel_id, sel_sn) =
                    nextrade_core::models::trade::check::resolve_gift_card_selection(
                        dto.use_gift_card_id,
                        dto.use_gift_card_sn.as_deref(),
                    );
                if sel_id.is_some() || sel_sn.is_some() {
                    let now_gc = utils::now_seconds();
                    // 适用范围（我方新增）：限定卡的抵扣上限 = 范围内商品应付额
                    // （与结算 check 同口径 `scoped_deduct_base`）；全部商品卡
                    // cap = total，行为与改造前逐分一致。范围读取失败时回落
                    // total（不因范围读取失败阻断已成立的抵扣，与本段既有
                    // 「抵扣失败仅告警」口径一致）。
                    let mut cap = total;
                    if let Ok(Some(group_id)) = self
                        .repos
                        .marketing
                        .gift_card_issue
                        .find_owned_card_group_id(user_id, sel_id, sel_sn.as_deref())
                        .await
                    {
                        if let Ok(Some((scope_type, pids, cids))) = self
                            .repos
                            .marketing
                            .gift_card_groups
                            .find_scope(group_id)
                            .await
                        {
                            if scope_type != 0 {
                                let pairs = self
                                    .repos
                                    .marketing
                                    .gift_card_groups
                                    .list_category_pairs()
                                    .await
                                    .unwrap_or_default();
                                let scope =
                                    GiftCardScope::from_parts(scope_type, pids, cids, &pairs);
                                cap = scoped_deduct_base(
                                    &scope,
                                    &gift_scope_lines,
                                    product_amount,
                                    total,
                                );
                            }
                        }
                    }
                    match self
                        .repos
                        .marketing
                        .gift_card_issue
                        .deduct_stored_card(
                            user_id,
                            sel_id,
                            sel_sn.as_deref(),
                            order_id,
                            cap,
                            now_gc,
                        )
                        .await
                    {
                        Ok(Some(res)) => gift_card_used = res.amount,
                        Ok(None) => {
                            tracing::warn!(
                                "订单 {order_id} 礼品卡抵扣未生效（卡不可用）: user={user_id}"
                            );
                        }
                        Err(e) => {
                            tracing::warn!("订单 {order_id} 礼品卡抵扣失败: {e}");
                        }
                    }
                }
            }
            if gift_card_used > Decimal::ZERO {
                let _ = self
                    .repos
                    .trade
                    .order
                    .set_gift_card_amount(order_id, gift_card_used)
                    .await;
            }
        }

        // —— 余额抵扣（Java `OrderCheckServiceImpl#calcBalanceDeduction`）——
        // 定金预售：本次只支付定金，余额抵扣留给尾款阶段（避免把定金当全款结算）
        let mut online_amount = if presale_row.is_some() {
            deposit_now
        } else {
            (total - gift_card_used).max(Decimal::ZERO)
        };
        let mut return_type = 1_i32;
        let mut final_balance_used = Decimal::ZERO;
        if presale_row.is_none() {
            // 礼品卡已全额覆盖：直接标记已付（礼品卡抵扣计入 paid_amount）
            if gift_card_used >= total && total > Decimal::ZERO {
                let now = utils::now_seconds() as i32;
                // 订单已落库：标记已付的 DB 异常只告警，不再上抛造成
                // 「报错但订单已建」（状态滞后由对账兜底，见积分处注释）。
                let marked = match self
                    .repos
                    .trade
                    .order
                    .mark_paid(order_id, now, total)
                    .await
                {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!("订单 {order_id} 礼品卡全额标记已付异常（订单已建）: {e}");
                        false
                    }
                };
                if marked {
                    online_amount = Decimal::ZERO;
                    return_type = 2;
                }
            } else if gift_card_used > Decimal::ZERO {
                let _ = self
                    .repos
                    .trade
                    .order
                    .apply_partial_payment(order_id, gift_card_used)
                    .await;
            }
        if let Some(requested) = dto.use_balance {
            if requested > Decimal::ZERO {
                // 订单已落库：读余额的 DB 异常只告警并按可用 0 处理
                // （保守口径：不动钱），不再上抛造成「报错但订单已建」。
                let available = match self.repos.member.user.find_by_id(user_id).await {
                    Ok(u) => u.map(|u| u.available_balance()).unwrap_or(Decimal::ZERO),
                    Err(e) => {
                        tracing::warn!("订单 {order_id} 读取买家余额异常，跳过余额抵扣: {e}");
                        Decimal::ZERO
                    }
                };
                let mut balance_used = requested
                    .min((total - gift_card_used).max(Decimal::ZERO))
                    .min(available);
                if balance_used > Decimal::ZERO {
                    // 同上：扣款 DB 异常按未扣处理（不动钱），只告警。
                    let debited = match self
                        .repos
                        .member
                        .user
                        .try_debit_balance(user_id, balance_used)
                        .await
                    {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::warn!("订单 {order_id} 余额扣减异常，按未扣处理: {e}");
                            false
                        }
                    };
                    if !debited {
                        balance_used = Decimal::ZERO;
                    }
                }
                if balance_used > Decimal::ZERO {
                    final_balance_used = balance_used;
                    let _ = self
                        .repos
                        .trade
                        .order
                        .set_balance_amount(order_id, balance_used)
                        .await;
                    let now = utils::now_seconds() as i32;
                    if balance_used + gift_card_used >= total {
                        // 同上：标记已付 DB 异常只告警（余额已扣、状态
                        // 滞后由对账兜底），不向已落库订单的调用方报错。
                        let marked = match self
                            .repos
                            .trade
                            .order
                            .mark_paid(order_id, now, total)
                            .await
                        {
                            Ok(v) => v,
                            Err(e) => {
                                tracing::warn!(
                                    "订单 {order_id} 余额付清标记已付异常（订单已建）: {e}"
                                );
                                false
                            }
                        };
                        if marked {
                            online_amount = Decimal::ZERO;
                            return_type = 2;
                            self.write_balance_paylog(order_id, &order_sn, total, balance_used, now)
                                .await;
                        }
                    } else {
                        let _ = self
                            .repos
                            .trade
                            .order
                            .apply_partial_payment(order_id, balance_used)
                            .await;
                        online_amount =
                            (total - gift_card_used - balance_used).max(Decimal::ZERO);
                        self.write_balance_paylog(order_id, &order_sn, total, balance_used, now)
                            .await;
                    }
                }
            }
        }
        }

        // —— 分销生成链·下单节点（best-effort，失败仅告警）——
        // 微商模式（distributionMode=2）：招募方式含「消费自动招募」时，
        // 买家首单自动成为分销员（原版 WechatBizRecruitService 口径）；
        // 推广模式（=1）：按购物车行暂存的 salesman_id 归属在下单时
        // 生成推广佣金（原版 OrderCheckServiceImpl 口径）。
        {
            let attributions: Vec<(i32, Option<i32>, i32)> = lines
                .iter()
                .filter(|l| l.salesman_id > 0)
                .map(|l| (l.product_id, l.sku_id, l.salesman_id))
                .collect();
            let generation =
                crate::marketing::salesman_generation::SalesmanGenerationService::new(
                    self.repos.clone(),
                );
            if let Err(e) = generation
                .on_order_submitted(user_id, order_id, &attributions)
                .await
            {
                tracing::warn!("订单 {order_id} 分销生成链（下单节点）失败: {e}");
            }
            // 提交即全额付清（礼品卡/余额全额覆盖，return_type=2）：订单已在
            // 上方 mark_paid，不会再走支付回调，同步补触发支付节点生成
            // （on_order_paid 内含整单防重，多通道重复触发不会双记）。
            if return_type == 2 {
                if let Err(e) = generation.on_order_paid(order_id).await {
                    tracing::warn!("订单 {order_id} 分销生成链（提交即付清）失败: {e}");
                }
                // 骑手 MVP 派送单生产者（同为 best-effort；内含订单级幂等）。
                let dispatch = crate::o2o::delivery_dispatch::DeliveryDispatchService::new(
                    self.repos.clone(),
                );
                if let Err(e) = dispatch.on_order_paid(order_id).await {
                    tracing::warn!("订单 {order_id} 派送单生产（提交即付清）失败: {e}");
                }
            }
        }

        // —— 跨店拆单（方案 A，原版 splitOrderAtSubmit）——
        // 排在礼品卡/余额抵扣与分销生成之后：子单金额以抵扣落定后的父单
        // 为准，生成在父单明细上的佣金行由拆单内重映射带走。尽力而为：
        // 失败仅记错误日志，未拆订单在支付时由 splitStoreOrder 兜底拆分。
        {
            let split = crate::trade::order_split::OrderSplitService::new(self.repos.clone());
            if let Err(e) = split.split_order_at_submit(order_id).await {
                tracing::error!("订单 {order_id} 提交拆单失败（支付时兜底）: {e}");
            }
            if return_type == 2 {
                // 提交即付清：同步激活子单（原版 setOrderPaid 末尾 splitStoreOrder）。
                if let Err(e) = split.split_store_order(order_id).await {
                    tracing::error!("订单 {order_id} 提交即付清子单激活失败: {e}");
                }
            }
        }

        // 提交即付清（余额/礼品卡全额抵扣）时礼物同步转待领取；
        // 在线支付的单由付款成功钩子激活（run_post_payment_hooks）
        if gift_mode && return_type == 2 {
            let gift_svc = crate::trade::gift_order::GiftOrderService::new(self.repos.clone());
            if let Err(e) = gift_svc.activate_on_paid(order_id).await {
                tracing::warn!("送礼物订单 {order_id} 提交即付清激活失败: {e}");
            }
        }

        let out_trade_no = format!("PAY{order_sn}");
        Ok(OrderCreateVO {
            order_id,
            order_sn,
            total_amount: total,
            online_amount,
            out_trade_no,
            return_type,
            groupon_promotion_record_id: groupon_record_id.and_then(|v| i32::try_from(v).ok()),
            balance: if final_balance_used > Decimal::ZERO { Some(final_balance_used) } else { None },
            is_card_redeem: Some(false),
            gift_token,
        })
    }

    /// 余额抵扣流水（对应 Java `PaylogService#creatPayLog(payCode="balance")`）。
    async fn write_balance_paylog(
        &self,
        order_id: i32,
        order_sn: &str,
        order_amount: Decimal,
        pay_amount: Decimal,
        now: i32,
    ) {
        use nextrade_core::entities::Paylog;
        use nextrade_core::models::payment::paylog::{generate_pay_sn, PayOrderType};
        use nextrade_infrastructure::db::repo::payment::paylog::PAYLOG_STATUS_PAID;
        let paylog = Paylog {
            paylog_id: 0,
            pay_sn: generate_pay_sn(),
            pay_name: format!("余额支付-订单{order_sn}"),
            order_id,
            order_sn: order_sn.to_string(),
            order_amount,
            order_type: PayOrderType::Product as i32,
            pay_amount,
            pay_status: PAYLOG_STATUS_PAID,
            pay_code: "balance".to_string(),
            profit_sharing_channel: None,
            add_time: Some(now),
            transaction_id: format!("BAL{}", utils::now_millis()),
            notify_data: String::new(),
            refund_amount: Decimal::ZERO,
            token_code: String::new(),
            appid: None,
        };
        if let Err(e) = self.repos.payment.paylog.insert(&paylog).await {
            tracing::warn!("余额支付流水写入失败: order_id={order_id} {e}");
        }
    }

    /// 订单详情。
    pub async fn detail(&self, order_id: i32, user_id: Option<i32>) -> Result<(OrderVO, Vec<OrderItem>)> {
        let order = self
            .repos.trade.order
            .find_by_id(order_id)
            .await?
            .ok_or_else(|| Error::NotFound("订单不存在".into()))?;

        // 归属校验（非本人不可查）
        if let Some(uid) = user_id {
            if order.user_id != uid {
                return Err(Error::Forbidden("无权查看该订单".into()));
            }
        }

        let items = self.repos.trade.order.find_items(order_id).await?;
        let claim = self
            .repos
            .trade
            .offline_payment_claim
            .latest_for_order(order_id)
            .await?;
        let mut vo = to_vo(&order, &items, claim.as_ref());
        // 买家取消直退（我方新增）：附加入口动作与退款进度
        let paid_cancel_enabled = self.paid_cancel_enabled().await;
        let latest_cancel = self
            .repos
            .payment
            .refund_apply
            .find_latest_by_order_initiator(
                order_id,
                RefundInitiator::BuyerCancel as i32,
            )
            .await
            .ok()
            .flatten();
        attach_paid_cancel_refund(
            &mut vo,
            &order,
            paid_cancel_enabled,
            latest_cancel.as_ref(),
        );
        // 送礼物标记 + 礼物面板（我方新增）：详情归属已限定买家本人，
        // gift_info 只对买家下发（领取人走领取页/收到列表的无价格视图）
        {
            let gift_svc = crate::trade::gift_order::GiftOrderService::new(self.repos.clone());
            if let Ok(Some(gift)) = gift_svc.resolve_for_order(order_id).await {
                vo.is_gift_order = true;
                vo.gift_status = Some(gift.gift_status);
                if gift.buyer_user_id == order.user_id {
                    if let Ok(info) = gift_svc.buyer_info(order_id, order.user_id).await {
                        vo.gift_info = info;
                    }
                }
            }
        }
        // 拼团×自提回填（我方新增，论坛帖 407906）：拼团单成团状态 +
        // 自提单自提点详情（此前买家侧两字段恒 None，拼团自提单在订单
        // 详情看不到自提点与成团门提示）
        self.attach_groupon_pickup(&order, &mut vo).await;
        // 卡密订单：补齐每个商品项已分配的电子卡券（原版 ECardItemVO）
        let mut has_card = false;
        for (idx, item) in items.iter().enumerate() {
            let mut cards = self
                .repos
                .catalog
                .e_card
                .find_by_order_item_id(item.item_id)
                .await?;
            // 历史卡密订单可能未在支付回调里分配成功；查看详情时兜底分配一次
            if cards.is_empty() && item.product_type == 3 {
                if let Ok(Some(prod)) = self.repos.catalog.product.find_by_id(item.product_id).await {
                    if prod.card_group_id > 0 {
                        if let Ok(allocated) = self
                            .repos
                            .catalog
                            .e_card
                            .allocate_cards_to_order_item(
                                prod.card_group_id,
                                order_id,
                                item.item_id,
                                item.quantity,
                            )
                            .await
                        {
                            cards = allocated;
                        }
                    }
                }
            }
            if cards.is_empty() {
                continue;
            }
            has_card = true;
            vo.items[idx]["eCard"] = serde_json::json!(cards
                .iter()
                .map(|c| serde_json::json!({
                    "cardId": c.card_id,
                    "groupId": c.group_id,
                    "cardNumber": c.card_number,
                    "cardPwd": c.card_pwd,
                    "orderItemId": c.order_item_id,
                }))
                .collect::<Vec<_>>());
        }
        if has_card || items.iter().any(|it| it.product_type == 3) {
            vo.is_card_order = true;
        }
        // 纯卡密订单：卡券下发后直接进入“待收货”，无需物流发货
        if has_card && !items.is_empty() && items.iter().all(|it| it.product_type == 3) {
            let now = utils::now_seconds() as i32;
            if self
                .repos
                .trade
                .order
                .mark_card_shipped(order_id, now)
                .await
                .is_ok()
            {
                vo.order_status = 2;
                vo.order_status_name = "待收货".into();
                vo.status_label = "待收货".into();
                vo.shipping_status = 1;
                vo.shipping_status_name = "已发货".into();
            }
        }
        // 虚拟内容（对应 Java `OrderItemServiceImpl#getItemByOrderIds` 按商品
        // 下发 `product.virtualSample` + `OrderServiceImpl#maskUndeliveredVirtualContent`
        // 买家侧遮蔽）：待发货时 type=2/3 不下发，发货后（含部分发货）下发。
        // 本 detail() 仅买家侧调用（后台走 order_admin 独立路径），无需管理员豁免判断。
        let eff_shipping_status = vo.shipping_status;
        for (idx, item) in items.iter().enumerate() {
            let sample = match self.repos.catalog.product.find_by_id(item.product_id).await {
                Ok(Some(p)) => p.virtual_sample.filter(|s| !s.trim().is_empty()),
                _ => None,
            };
            vo.items[idx]["virtualSample"] =
                match virtual_sample_for(eff_shipping_status, item.product_type, sample) {
                    Some(s) => serde_json::json!(s),
                    None => serde_json::Value::Null,
                };
        }
        // 文件型交付（我方新增）：逐项注入附件列表，门控与 virtualSample
        // 同构（delivery_access）——未达门恒空数组、不下发文件名；附件
        // 按商品批量查询避免 N+1。下载另走令牌申领接口强校验。
        let delivery_product_ids: Vec<i32> = items
            .iter()
            .filter(|it| matches!(it.product_type, 2 | 4))
            .map(|it| it.product_id)
            .collect();
        let delivery_files = if delivery_product_ids.is_empty() {
            Vec::new()
        } else {
            self.repos
                .catalog
                .product_delivery
                .list_by_products(&delivery_product_ids)
                .await?
        };
        for (idx, item) in items.iter().enumerate() {
            let files: Vec<serde_json::Value> =
                if delivery_access(item.product_type, order.pay_status, eff_shipping_status) {
                    delivery_files
                        .iter()
                        .filter(|f| f.1 == item.product_id)
                        .map(|f| {
                            serde_json::json!({
                                "fileId": f.0,
                                "productId": f.1,
                                "fileName": f.3,
                                "fileExt": f.5,
                                "fileSize": f.6,
                                "addTime": f.8,
                            })
                        })
                        .collect()
                } else {
                    Vec::new()
                };
            vo.items[idx]["deliveryFiles"] = serde_json::json!(files);
        }
        Ok((vo, items))
    }

    /// 订单分页。
    ///
    /// 过滤语义对齐 Java `OrderServiceImpl#list`：
    /// - `comment_status = Some(0)` → 待评价（`comment_status=0` 且订单已完成）；
    /// - `comment_status = Some(n>0)` → 已评价等值；
    /// - `-1` / `None` → 不过滤（前端“全部/待付款/…”标签传 -1）。
    /// - `is_showed = Some(0/1)` → 待晒单（按 comment 表过滤，见仓储注释）。
    pub async fn list(
        &self,
        user_id: Option<i32>,
        order_status: Option<i32>,
        comment_status: Option<i32>,
        is_showed: Option<i32>,
        keyword: Option<&str>,
        page: &PageQuery,
    ) -> Result<PageResult<OrderVO>> {
        // 与 Java 一致：-1 视为不过滤；待晒单 > -1 生效
        let comment_status = normalize_comment_status(comment_status);
        let is_showed = normalize_is_showed(is_showed);
        let keyword = normalize_keyword(keyword);
        // 店铺小程序作用域（我方新增）：买家订单列表只保留含本店
        // 明细的订单（作用域中间件已按 appid 定店；平台作用域不过滤）。
        let scope_shop_id =
            nextrade_infrastructure::db::repo::member::mini_shop_scope::MiniShopScope::current()
                .shop_id();
        let (total, orders) = self
            .repos.trade.order
            .paginate(
                user_id,
                order_status,
                comment_status,
                is_showed,
                keyword,
                page.page,
                page.page_size,
                scope_shop_id,
            )
            .await?;
        let order_ids: Vec<i32> = orders.iter().map(|o| o.order_id).collect();
        let claims = self
            .repos
            .trade
            .offline_payment_claim
            .latest_for_orders(&order_ids)
            .await?;
        // 买家取消直退（我方新增）：批量预取各单最新一笔买家取消申请
        let paid_cancel_enabled = self.paid_cancel_enabled().await;
        let buyer_cancel_map = self
            .repos
            .payment
            .refund_apply
            .latest_map_by_orders_initiator(
                &order_ids,
                RefundInitiator::BuyerCancel as i32,
            )
            .await
            .unwrap_or_default();
        // 送礼物标记批量解析（我方新增；子单经父单命中）
        let gift_svc = crate::trade::gift_order::GiftOrderService::new(self.repos.clone());
        let gift_keys: Vec<(i32, Option<i32>)> =
            orders.iter().map(|o| (o.order_id, o.parent_order_id)).collect();
        let gift_map = gift_svc.resolve_map(&gift_keys).await.unwrap_or_default();
        // 拼团成团状态批量预取（我方新增，论坛帖 407906）：order_id -> join_status
        let groupon_map: std::collections::HashMap<i32, i32> = self
            .repos
            .trade
            .order_admin
            .find_groupon_records(&order_ids)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|(oid, _, js)| (oid, js))
            .collect();
        let mut list = Vec::with_capacity(orders.len());
        for o in &orders {
            let items = self.repos.trade.order.find_items(o.order_id).await?;
            let mut vo = to_vo(o, &items, claims.get(&o.order_id));
            attach_paid_cancel_refund(
                &mut vo,
                o,
                paid_cancel_enabled,
                buyer_cancel_map.get(&o.order_id),
            );
            if let Some(gift) = gift_map.get(&o.order_id) {
                vo.is_gift_order = true;
                vo.gift_status = Some(gift.gift_status);
            }
            if let Some(js) = groupon_map.get(&o.order_id) {
                vo.join_status = Some(*js);
                vo.join_status_desc = Some(join_status::description(*js));
            }
            list.push(vo);
        }
        Ok(PageResult::new(total, list, page))
    }

    /// 买家取消直退主开关（生效值；读取失败按关处理，零行为变化）。
    async fn paid_cancel_enabled(&self) -> bool {
        let raw = self
            .repos
            .system
            .config_basic
            .find_by_code(crate::system::paid_cancel_refund::PAID_CANCEL_REFUND_CODE)
            .await
            .ok()
            .flatten();
        crate::system::paid_cancel_refund::effective_paid_cancel_refund(raw.as_deref())
            .enabled
    }

    /// 拼团×自提回填（我方新增，论坛帖 407906）：拼团订单回填成团状态
    /// （`join_status` / `join_status_desc`，此前买家侧恒 None）；自提订单
    /// （`shipping_method=2`，含拼团自提单）回填自提点详情 `pickup_detail`。
    /// 均为展示增强：查询失败静默跳过，不阻断详情主链。
    async fn attach_groupon_pickup(&self, order: &Order, vo: &mut OrderVO) {
        if order.order_type == Some(2) {
            if let Ok(records) = self
                .repos
                .trade
                .order_admin
                .find_groupon_records(&[order.order_id])
                .await
            {
                if let Some((_, _, js)) = records.into_iter().next() {
                    vo.join_status = Some(js);
                    vo.join_status_desc = Some(join_status::description(js));
                }
            }
        }
        if order.shipping_method == 2 {
            vo.pickup_detail = self.pickup_detail_for(order).await;
        }
    }

    /// 自提点详情：自提点 ID 在建单时快照进地址文案
    /// （`门店自提 (自提点ID: N)`，订单表无独立 pickup 列）；解析不到或
    /// 门店不存在时回退到订单店铺的自提门店（与结算页
    /// `check::build_delivery_option` 同一查询口径）。字段口径对齐前端
    /// `PickupDetail`（shopId / shopTitle / pickupName / address）。
    async fn pickup_detail_for(&self, order: &Order) -> Option<serde_json::Value> {
        let mut shop: Option<nextrade_core::entities::Shop> = None;
        if let Some(pickup_shop_id) = parse_pickup_shop_id(&order.address) {
            shop = sqlx::query_as::<_, nextrade_core::entities::Shop>(
                "SELECT * FROM shop WHERE shop_id = $1",
            )
            .bind(pickup_shop_id)
            .fetch_optional(&self.repos.pool)
            .await
            .ok()
            .flatten();
        }
        if shop.is_none() && order.shop_id > 0 {
            shop = sqlx::query_as::<_, nextrade_core::entities::Shop>(
                "SELECT * FROM shop WHERE store_parent_id = $1 AND status = 1 \
                 ORDER BY shop_id LIMIT 1",
            )
            .bind(order.shop_id)
            .fetch_optional(&self.repos.pool)
            .await
            .ok()
            .flatten();
        }
        let shop = shop?;
        let contact: serde_json::Value = shop
            .shop_contact_json
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(serde_json::Value::Null);
        let address = contact
            .get("address")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        Some(serde_json::json!({
            "shopId": shop.shop_id,
            "shopTitle": shop.shop_title.clone().unwrap_or_default(),
            "pickupName": shop.shop_title.unwrap_or_default(),
            "address": address,
            "contactMobile": shop.contact_mobile.clone().unwrap_or_default(),
            "shopContact": contact,
            "shopOpenCloseConfig": shop
                .shop_open_close_json
                .as_deref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()),
        }))
    }

    /// 取消订单（仅待付款可取消，取消后回滚库存）。
    pub async fn cancel(&self, order_id: i32, user_id: Option<i32>, reason: Option<&str>) -> Result<()> {
        let order = self
            .repos.trade.order
            .find_by_id(order_id)
            .await?
            .ok_or_else(|| Error::NotFound("订单不存在".into()))?;
        if let Some(uid) = user_id {
            if order.user_id != uid {
                return Err(Error::Forbidden("无权操作该订单".into()));
            }
        }
        // 拍卖订单不可自助取消：保证金已抵扣，取消即资损；违约由拍卖逾期任务统一处理
        if order.order_type == Some(11) {
            return Err(Error::Conflict("拍卖订单不可取消，请按期支付尾款".into()));
        }
        if !order.is_cancellable() {
            return Err(Error::Conflict("当前订单状态不可取消".into()));
        }

        // 原子状态流转：0 → 40
        let ok = self
            .repos.trade.order
            .transition_status(order_id, &[status::UNPAID], status::CANCELED)
            .await?;
        // Java `cancelOrder(..., description)`:`order.cancel_remark = 取消原因`
        let remark = reason.map(str::trim).filter(|s| !s.is_empty());
        if let Some(remark) = remark {
            sqlx::query("UPDATE \"order\" SET cancel_remark = $1 WHERE order_id = $2")
                .bind(remark)
                .bind(order_id)
                .execute(&self.repos.pool)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
        }
        if !ok {
            return Err(Error::Conflict("订单状态已变更，请刷新后重试".into()));
        }

        // 回滚库存
        let items = self.repos.trade.order.find_items(order_id).await?;
        for item in items {
            let pid = item.product_id;
            let qty = item.quantity;
            if let Err(e) = self.product.restore_stock(pid, Some(item.sku_id), qty).await {
                tracing::error!("取消订单回滚库存失败 order={order_id} product={pid}: {e}");
            }
        }
        // 礼品卡返还（尽力而为）：按 usage_log type=1 扣减记录原额返还
        // 卡内余额并写 type=2 流水，仓储层幂等（已返还则跳过）。
        if let Err(e) = self
            .repos
            .marketing
            .gift_card_issue
            .return_stored_cards_for_order(order_id, utils::now_seconds())
            .await
        {
            tracing::warn!("取消订单 {order_id} 礼品卡返还失败: {e}");
        }
        // 送礼物联动（我方新增）：未付款取消后礼物行转「未付款取消」
        // （非礼物订单无礼物行、空转；买家礼物取消入口另有退款路径，
        // 只处理待领取状态，不经本函数）
        {
            let gift_svc = crate::trade::gift_order::GiftOrderService::new(self.repos.clone());
            if let Err(e) = gift_svc.mark_canceled_unpaid(order_id).await {
                tracing::warn!("订单 {order_id} 取消后礼物行联动失败: {e}");
            }
        }
        tracing::info!("订单已取消: {order_id}");
        Ok(())
    }

    /// 支付成功回调（由支付服务调用，幂等）。
    pub async fn on_payment_success(
        &self,
        order_id: i32,
        paid_amount: Decimal,
        out_trade_no: Option<String>,
    ) -> Result<()> {
        let order = self
            .repos.trade.order
            .find_by_id(order_id)
            .await?
            .ok_or_else(|| Error::NotFound("订单不存在".into()))?;

        if order.is_paid() {
            tracing::info!("订单已支付，跳过重复回调: {order_id}");
            return Ok(()); // 幂等
        }

        let now = utils::now_seconds() as i32;
        self.repos.trade.order
            .mark_paid(order_id, now, paid_amount)
            .await?;

        self.run_post_payment_hooks(&order, paid_amount).await;

        if let Some(no) = out_trade_no {
            tracing::info!("订单支付成功: {order_id}, trade_no={no}");
        }
        Ok(())
    }

    /// 支付成功后置钩子（全仓单一入口）。
    ///
    /// 顺序：订单日志 → 积分赠送（Node 1）→ 电子卡券分配 → 对账单出账
    /// → 分销生成 → 骑手派送 → 跨店拆单激活 → 店铺会员记账。各段均为 best-effort
    /// （失败仅告警/忽略），本函数不返回错误。
    ///
    /// 调用方：`on_payment_success`（渠道回调 / POS / 线下认款 / 后台
    /// 设置已支付）在其 mark_paid 后调用；自管 mark_paid 与回滚语义、
    /// 不经 on_payment_success 的路径（余额支付全额付清、管理端预售
    /// 尾款确认）在其状态流转落定后直接调用本入口，保证任何把订单
    /// 推到已付的路径钩子齐全且实现唯一（禁止在各支付路径内手抄本
    /// 序列的子集——2026-11-21 批次 3 验收 P0：余额路径曾只补分销+
    /// 骑手两钩，拆单激活缺失，跨店子单永不激活）。
    ///
    /// 幂等前提：调用方须保证仅在订单本次真正完成付清流转时调用一次
    /// （已付守卫 / mark_paid 条件更新 / 状态机门控）；各钩子自身另有
    /// 订单级防重（分销整单防重、派送单订单级幂等、拆单按父单状态守卫）。
    pub async fn run_post_payment_hooks(&self, order: &Order, paid_amount: Decimal) {
        let order_id = order.order_id;
        let now = utils::now_seconds() as i32;

        // 0. 送礼物订单激活（我方新增）：礼物行 待付款 → 待领取 并起算
        // 领取有效期；非礼物订单无礼物行、天然空转。提交即付清路径不
        // 经本钩子，由 create 末尾单独调用同一 activate_on_paid。
        {
            let gift_svc = crate::trade::gift_order::GiftOrderService::new(self.repos.clone());
            if let Err(e) = gift_svc.activate_on_paid(order_id).await {
                tracing::warn!("送礼物订单 {order_id} 付款激活失败: {e}");
            }
        }

        // 1. 记录订单日志
        let _ = self.repos.trade.user_order.insert_order_log(
            order_id,
            &order.order_sn,
            order.user_id,
            order.shop_id,
            "订单支付成功",
            now,
        ).await;

        // 2. 积分赠送（待发货节点 Node 1）
        let _ = self.order_success_point(order, 1).await;

        // 3. 电子卡券分配
        if let Ok(items) = self.repos.trade.order.find_items(order_id).await {
            let has_items = !items.is_empty();
            let all_card = has_items && items.iter().all(|it| it.product_type == 3);
            let mut allocated_ok = true;
            for it in &items {
                if it.product_type != 3 {
                    continue;
                }
                if let Ok(Some(prod)) = self.repos.catalog.product.find_by_id(it.product_id).await {
                    if prod.card_group_id > 0 {
                        let gid = prod.card_group_id;
                        if self
                            .repos
                            .catalog
                            .e_card
                            .allocate_cards_to_order_item(gid, order_id, it.item_id, it.quantity)
                            .await
                            .is_err()
                        {
                            allocated_ok = false;
                        }
                    }
                }
            }
            // 纯卡密订单：下发卡券后直接转为“待收货”
            if all_card && allocated_ok {
                let _ = self.repos.trade.order.mark_card_shipped(order_id, now).await;
                let _ = self.repos.trade.user_order.insert_order_log(
                    order_id,
                    &order.order_sn,
                    order.user_id,
                    order.shop_id,
                    "卡券已发放，订单待收货",
                    now,
                ).await;
            }
        }

        // 4. 对账单出账流水
        let output = nextrade_core::entities::StatementOutput {
            statement_output_id: 0,
            shop_id: Some(order.shop_id),
            vendor_id: order.vendor_id,
            income: Some(paid_amount),
            expenditure: Some(Decimal::ZERO),
            gmt_create: Some(now as i64),
            record_sn: Some(order.order_sn.clone()),
            // Java `StatementType.ORDER`（3=订单收支）。此前误写 1（手续费）。
            record_type: Some(nextrade_core::models::payment::statement::statement_type::ORDER),
            record_id: Some(order.order_id),
            settlement_status: Some(0),
            real_settlement: Some(0),
            real_settlement_time: None,
        };
        let _ = self.repos.payment.statement.insert_statement_output(&output).await;

        // 5. 分销生成链·支付节点（best-effort，失败仅告警）：微商模式在
        // 支付成功时生成佣金（原版 OrderServiceImpl 支付成功 → 生成口径）；
        // 生成服务内含整单防重，重复回调/多通道触发不会双记。
        {
            let generation =
                crate::marketing::salesman_generation::SalesmanGenerationService::new(
                    self.repos.clone(),
                );
            if let Err(e) = generation.on_order_paid(order_id).await {
                tracing::warn!("订单 {order_id} 分销生成链（支付节点）失败: {e}");
            }
        }

        // 6. 骑手 MVP 派送单生产者（best-effort，失败仅告警）：同城配送/
        // 上门服务订单付款成功后生成 delivery_order（生产者内含配置门与
        // 订单级幂等；不适用订单静默跳过）。
        {
            let dispatch = crate::o2o::delivery_dispatch::DeliveryDispatchService::new(
                self.repos.clone(),
            );
            if let Err(e) = dispatch.on_order_paid(order_id).await {
                tracing::warn!("订单 {order_id} 派送单生产（支付节点）失败: {e}");
            }
        }

        // 7. 跨店拆单激活/支付时拆单（best-effort，失败仅告警）：
        // 对应原版 setOrderPaid 末尾无条件调用 splitStoreOrder——已拆订单
        // 激活子单（父单转无效），未拆多店订单此时拆分并删除父单。
        // 排在分销生成之后：生成在父单明细上的佣金行由激活内重映射带走。
        {
            let split = crate::trade::order_split::OrderSplitService::new(self.repos.clone());
            if let Err(e) = split.split_store_order(order_id).await {
                tracing::warn!("订单 {order_id} 支付拆单/激活失败: {e}");
            }
        }

        // 8. 店铺会员记账（best-effort，失败仅告警）：原版 recordOnPaid——
        // 支付成功后按成交店铺把买家沉淀为该店会员并累计已支付订单数/金额。
        // 必须排在拆单（钩子 7）之后：成交单元以拆单落定结果为准（见
        // record_shop_member_on_paid）。记账不受店铺会员总开关控制（原版
        // 实现口径：总开关只冻结店铺端展示/写操作）。
        {
            let member = crate::member::shop_member::ShopMemberService::new(self.repos.clone());
            if let Err(e) = self
                .record_shop_member_on_paid(&member, order, paid_amount)
                .await
            {
                tracing::warn!("订单 {order_id} 店铺会员记账失败: {e}");
            }
        }

    }

    /// 店铺会员记账的成交单元解析（钩子 8 本体）。
    ///
    /// 拆单已在钩子 7 落定：有已付子单 ⇒ 逐子单按子单 `shop_id` +
    /// `paid_amount` 记账（跨店单天然分店沉淀）；无子单 ⇒ 按本单记账
    /// （重读订单取拆单回填后的 `shop_id` 与累计 `paid_amount`，读不到
    /// 回退入参快照 + 本次支付额）。子单不经 `on_payment_success`，不会
    /// 与本钩子重复记账。平台自营（shop_id<=0）在服务层跳过。
    async fn record_shop_member_on_paid(
        &self,
        member: &crate::member::shop_member::ShopMemberService,
        order: &Order,
        paid_amount: Decimal,
    ) -> Result<()> {
        let now = utils::now_seconds();
        let children = self
            .repos
            .trade
            .order_admin
            .find_children(order.order_id)
            .await?;
        // pay_status=2 为已支付；已取消子单不记账。
        let paid_children: Vec<&Order> = children
            .iter()
            .filter(|c| c.order_status != status::CANCELED && c.pay_status == 2)
            .collect();
        if !paid_children.is_empty() {
            for child in paid_children {
                member
                    .record_on_paid(child.shop_id, child.user_id, child.paid_amount, now)
                    .await?;
            }
            return Ok(());
        }
        let fresh = self.repos.trade.order.find_by_id(order.order_id).await?;
        let (shop_id, amount) = match &fresh {
            Some(o) => (o.shop_id, o.paid_amount),
            None => (order.shop_id, paid_amount),
        };
        member
            .record_on_paid(shop_id, order.user_id, amount, now)
            .await
    }

    /// 订单节点赠送积分（对齐 Java OrderServiceImpl#orderSuccessPoint）
    pub async fn order_success_point(&self, order: &Order, current_node: i32) -> Result<()> {
        let node_val = self
            .repos
            .system
            .config_basic
            .find_by_code("orderSendPointNode")
            .await?
            .unwrap_or_else(|| "1".into());
        let config_node = node_val.trim().parse::<i32>().unwrap_or(1);
        if current_node != config_node {
            return Ok(());
        }

        // 批发订单赠积分开关（`wholesaleGivePoints`，2026-11-21 接线，
        // 此前该配置无任何消费方、批发单与普通单一样赠分，与设置脱节）。
        // **整单口径**：订单含任一批发成交行（`is_wholesale_line = 1`，
        // 由批发加车链路在建单时落行）即按批发单处理——开关关 → 整单
        // 不赠分；开关开 → 与普通订单完全一致。取整单而非按行的理由：
        // 本函数按订单总金额在订单级计算积分（Java `orderSuccessPoint`
        // 原口径），积分流水也是订单级一条，无按行拆分结构；混合单
        // （批发线 + 普通行）按批发单处理与原版订单级判定一致。
        // 普通订单不读批发设置、行为逐字不变。
        {
            let items = self.repos.trade.order.find_items(order.order_id).await?;
            if items.iter().any(|it| it.is_wholesale_line == 1) {
                let setting = crate::wholesale::wholesale_setting::WholesaleSettingService::new(
                    self.repos.clone(),
                )
                .get()
                .await?;
                if setting.wholesale_give_points != 1 {
                    tracing::info!(
                        "批发订单按设置不赠送积分: order_id={}, node={}",
                        order.order_id,
                        current_node
                    );
                    return Ok(());
                }
            }
        }

        if current_node == 3 {
            let days_val = self
                .repos
                .system
                .config_basic
                .find_by_code("orderCompleteSendPointDays")
                .await?
                .unwrap_or_default();
            let delay_days = days_val.trim().parse::<i32>().unwrap_or(0);
            if delay_days > 0 {
                tracing::info!(
                    "订单已完成，积分延迟{}天后赠送: order_id={}",
                    delay_days,
                    order.order_id
                );
                return Ok(());
            }
        }

        let send_point_val = self
            .repos
            .system
            .config_basic
            .find_by_code("orderSendPoint")
            .await?
            .unwrap_or_default();
        let rate = send_point_val.trim().parse::<Decimal>().unwrap_or(Decimal::ZERO);
        if rate <= Decimal::ZERO {
            return Ok(());
        }

        let integral_name = self
            .repos
            .system
            .config_basic
            .find_by_code("integralName")
            .await?
            .unwrap_or_else(|| "积分".into());

        let base_points = (order.total_amount * rate)
            .ceil()
            .to_string()
            .parse::<i32>()
            .unwrap_or(0);
        if base_points <= 0 {
            return Ok(());
        }

        let desc = format!("下单送{}", integral_name);
        self.repos
            .member
            .user_points_log
            .inc_points(base_points, order.user_id, &desc, Some(order.order_id))
            .await?;

        tracing::info!(
            "订单节点赠送积分成功: order_id={}, user_id={}, node={}, points={}",
            order.order_id,
            order.user_id,
            current_node,
            base_points
        );
        Ok(())
    }

    /// 发货（后台操作）。
    pub async fn ship(&self, order_id: i32, tracking_no: &str, logistics_name: &str) -> Result<()> {
        // 送礼物守卫（我方新增）：待领取时地址还是空快照，禁止发货
        crate::trade::gift_order::ensure_deliverable(self.repos.clone(), order_id).await?;
        let ok = self
            .repos.trade.order
            .transition_status(order_id, &[status::PAID], status::SHIPPED)
            .await?;
        if !ok {
            return Err(Error::Conflict("订单当前状态不可发货".into()));
        }
        self.repos.trade.order
            .update_shipping(order_id, tracking_no, logistics_name, utils::now_seconds() as i32)
            .await?;
        if let Ok(Some(order)) = self.repos.trade.order.find_by_id(order_id).await {
            let _ = self.order_success_point(&order, 2).await;
        }
        tracing::info!("订单已发货: {order_id}, 运单号: {tracking_no}");
        Ok(())
    }

    // ————————————————————— 线下支付认款（v5.8.61） —————————————————————

    /// 提交线下认款申请。
    pub async fn offline_claim_submit(
        &self,
        user_id: i32,
        dto: OfflinePaymentClaimSubmitDTO,
    ) -> Result<i32> {
        let order_id = dto.order_id.ok_or_else(|| Error::biz(400, "认款参数不完整"))?;
        let order = self
            .repos
            .trade
            .order
            .find_by_id(order_id)
            .await?
            .ok_or_else(|| Error::biz(400, "订单不存在"))?;
        if order.user_id != user_id {
            return Err(Error::biz(400, "订单不存在"));
        }
        if order.pay_type_id != nextrade_core::models::trade::order_admin::pay_type::OFFLINE
            || order.pay_status != 0
            || order.order_status != status::UNPAID
        {
            return Err(Error::biz(400, "当前订单不可提交线下认款"));
        }
        if let Some(latest) = self
            .repos
            .trade
            .offline_payment_claim
            .latest_for_order(order_id)
            .await?
        {
            if latest.status == OfflinePaymentClaim::STATUS_PENDING
                || (latest.status == OfflinePaymentClaim::STATUS_APPROVED && latest.paid_time == 0)
            {
                return Err(Error::biz(400, "该订单已有认款申请，请等待财务审核"));
            }
        }
        let vouchers: Vec<String> = dto
            .voucher_urls
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .collect();
        if vouchers.is_empty() {
            return Err(Error::biz(400, "请上传付款凭证"));
        }
        let (stage, due) = self.offline_claim_stage(&order).await?;
        let now = utils::now_seconds() as i64;
        let claim = OfflinePaymentClaim {
            order_id,
            order_sn: order.order_sn.clone(),
            user_id,
            shop_id: order.shop_id,
            claimed_amount: due,
            pay_stage: stage,
            transfer_time: dto.transfer_time.unwrap_or(0),
            payer_name: dto.payer_name.unwrap_or_default().trim().to_string(),
            payer_account: dto.payer_account.unwrap_or_default().trim().to_string(),
            bank_serial_no: dto.bank_serial_no.unwrap_or_default().trim().to_string(),
            voucher_urls: Some(serde_json::to_string(&vouchers).unwrap_or_else(|_| "[]".to_string())),
            remark: dto.remark.unwrap_or_default().trim().to_string(),
            status: OfflinePaymentClaim::STATUS_PENDING,
            review_admin_id: 0,
            review_time: 0,
            paid_time: 0,
            audit_remark: String::new(),
            reject_reason: String::new(),
            add_time: now,
            update_time: now,
            claim_id: 0,
        };
        let claim_id = self
            .repos
            .trade
            .offline_payment_claim
            .insert(&claim)
            .await?;
        let _ = self
            .repos
            .trade
            .user_order
            .insert_order_log(
                order_id,
                &order.order_sn,
                user_id,
                order.shop_id,
                "用户提交线下付款凭证，等待财务审核",
                now as i32,
            )
            .await;
        Ok(claim_id)
    }

    pub async fn offline_claim_latest(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<Option<OfflinePaymentClaimVO>> {
        let claim = self
            .repos
            .trade
            .offline_payment_claim
            .latest_for_order(order_id)
            .await?;
        Ok(claim
            .filter(|c| c.user_id == user_id)
            .map(|c| to_offline_claim_vo(&c)))
    }

    pub async fn offline_claim_history(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<Vec<OfflinePaymentClaimVO>> {
        let claims = self
            .repos
            .trade
            .offline_payment_claim
            .history_for_order(order_id, Some(user_id))
            .await?;
        Ok(claims.iter().map(to_offline_claim_vo).collect())
    }

    pub async fn offline_claim_admin_detail(&self, claim_id: i32) -> Result<OfflinePaymentClaimVO> {
        let claim = self
            .repos
            .trade
            .offline_payment_claim
            .find_by_id(claim_id)
            .await?
            .ok_or_else(|| Error::biz(400, "认款单不存在"))?;
        Ok(to_offline_claim_vo(&claim))
    }

    /// 审核线下认款：通过后同步完成订单入账。
    pub async fn offline_claim_audit(
        &self,
        admin_id: i32,
        dto: OfflinePaymentClaimAuditDTO,
    ) -> Result<()> {
        let claim_id = dto.claim_id.ok_or_else(|| Error::biz(400, "审核参数不完整"))?;
        let pass = dto.pass.ok_or_else(|| Error::biz(400, "审核参数不完整"))?;
        let claim = self
            .repos
            .trade
            .offline_payment_claim
            .find_by_id(claim_id)
            .await?
            .ok_or_else(|| Error::biz(400, "认款单不存在"))?;
        if claim.status != OfflinePaymentClaim::STATUS_PENDING {
            return Err(Error::biz(400, "该认款单已审核，请勿重复操作"));
        }
        let order = self
            .repos
            .trade
            .order
            .find_by_id(claim.order_id)
            .await?
            .ok_or_else(|| Error::biz(400, "订单当前不可审核认款"))?;
        if order.pay_status != 0 || order.order_status != status::UNPAID {
            return Err(Error::biz(400, "订单当前不可审核认款"));
        }
        let audit_remark = dto.audit_remark.unwrap_or_default().trim().to_string();
        let now = utils::now_seconds() as i64;
        if pass {
            let (stage, due) = self.offline_claim_stage(&order).await?;
            if claim.pay_stage != stage || claim.claimed_amount != due {
                return Err(Error::biz(400, "订单应付金额已变更，请驳回后让用户重新提交"));
            }
            let updated = self
                .repos
                .trade
                .offline_payment_claim
                .audit(claim_id, true, admin_id, &audit_remark, "", now)
                .await?;
            if !updated {
                return Err(Error::biz(400, "认款单已被其他管理员处理"));
            }
            // 入账口径与余额/在线支付一致：定金推进预售状态，全款/尾款触发支付成功。
            if claim.pay_stage == OfflinePaymentClaim::STAGE_DEPOSIT {
                if let Some(ps) = self
                    .repos
                    .payment
                    .pay
                    .find_presale_by_order_id(order.order_id)
                    .await?
                {
                    self.repos
                        .payment
                        .pay
                        .settle_presale_deposit(order.order_id, ps.deposit_amount)
                        .await?;
                    self.repos
                        .payment
                        .pay
                        .update_presale_deposit_paid(order.order_id, now as i32, Some(now as i32))
                        .await?;
                }
            } else {
                self.on_payment_success(order.order_id, order.total_amount, None)
                    .await?;
                if claim.pay_stage == OfflinePaymentClaim::STAGE_FINAL {
                    self.repos
                        .payment
                        .pay
                        .update_presale_final_paid(order.order_id, now as i32)
                        .await?;
                }
            }
            self.repos
                .trade
                .offline_payment_claim
                .mark_paid(claim_id, now)
                .await?;
            let _ = self
                .repos
                .trade
                .user_order
                .insert_order_log(
                    order.order_id,
                    &order.order_sn,
                    order.user_id,
                    order.shop_id,
                    &format!("线下认款审核通过，认款单号：{claim_id}，已入账"),
                    now as i32,
                )
                .await;
            Ok(())
        } else {
            let reason = dto.reject_reason.unwrap_or_default().trim().to_string();
            if reason.is_empty() {
                return Err(Error::biz(400, "请填写驳回原因"));
            }
            let updated = self
                .repos
                .trade
                .offline_payment_claim
                .audit(claim_id, false, admin_id, &audit_remark, &reason, now)
                .await?;
            if !updated {
                return Err(Error::biz(400, "认款单已被其他管理员处理"));
            }
            let _ = self
                .repos
                .trade
                .user_order
                .insert_order_log(
                    order.order_id,
                    &order.order_sn,
                    order.user_id,
                    order.shop_id,
                    &format!("线下认款审核驳回，原因：{reason}"),
                    now as i32,
                )
                .await;
            Ok(())
        }
    }

    async fn offline_claim_stage(&self, order: &Order) -> Result<(i32, Decimal)> {
        if order.presale_type != Some(2) {
            return Ok((OfflinePaymentClaim::STAGE_FULL, order.unpaid_amount));
        }
        let ps = self
            .repos
            .payment
            .pay
            .find_presale_by_order_id(order.order_id)
            .await?
            .ok_or_else(|| Error::biz(400, "当前订单不可提交线下认款"))?;
        match ps.presale_status {
            0 => {
                let due = ps.deposit_amount - order.balance;
                if due <= Decimal::ZERO {
                    return Err(Error::biz(400, "当前订单不可提交线下认款"));
                }
                Ok((OfflinePaymentClaim::STAGE_DEPOSIT, due))
            }
            1 => Ok((OfflinePaymentClaim::STAGE_FINAL, order.unpaid_amount)),
            _ => Err(Error::biz(400, "当前订单不可提交线下认款")),
        }
    }
}

/// 线下认款实体 → VO。
pub(crate) fn to_offline_claim_vo(c: &OfflinePaymentClaim) -> OfflinePaymentClaimVO {
    OfflinePaymentClaimVO {
        claim_id: c.claim_id,
        order_id: c.order_id,
        order_sn: c.order_sn.clone(),
        user_id: c.user_id,
        shop_id: c.shop_id,
        claimed_amount: c.claimed_amount,
        pay_stage: c.pay_stage,
        pay_stage_name: stage_name(c.pay_stage),
        transfer_time: c.transfer_time,
        payer_name: c.payer_name.clone(),
        payer_account: c.payer_account.clone(),
        bank_serial_no: c.bank_serial_no.clone(),
        voucher_urls: c
            .voucher_urls
            .as_deref()
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_default(),
        remark: c.remark.clone(),
        status: c.status,
        status_name: status_name(c.status),
        review_admin_id: c.review_admin_id,
        review_time: c.review_time,
        paid_time: c.paid_time,
        audit_remark: c.audit_remark.clone(),
        reject_reason: c.reject_reason.clone(),
        add_time: c.add_time,
        update_time: c.update_time,
    }
}

/// 原版客户端 `OrderStatusEnum` 文案（0–5）；兼容历史内部状态 10/20/30/40/50。
fn client_status_label(status: i32) -> String {
    match status {
        0 => "待付款",
        1 => "已确认，待发货",
        2 => "待收货",
        3 => "已取消",
        4 => "无效",
        5 => "已完成",
        10 => "待发货",
        20 => "已发货",
        30 => "已完成",
        40 => "已取消",
        50 => "已关闭",
        _ => "未知状态",
    }
    .to_string()
}

/// 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`（Java `TigUtils.handelTime`：≤0 返回空串）。
fn fmt_time(secs: i32) -> String {
    if secs <= 0 {
        return String::new();
    }
    use chrono::TimeZone;
    match chrono::Local.timestamp_opt(i64::from(secs), 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => String::new(),
    }
}

/// 订单状态提示文案（对齐 Java `OrderStatusEnum.description`）。
///
/// PC 订单详情在无法展示支付倒计时时展示 `orderStatusTip`（见
/// `pc-semi/src/components/OrderDetailClient.tsx`），旧 Java `OrderVO` 会下发该字段。
#[must_use]
fn status_tip(status: i32) -> String {
    match status {
        0 => "待支付",
        1 => "已确认，待发货",
        2 => "处理中，已发货",
        3 => "已取消",
        4 => "无效",
        5 => "已完成",
        _ => "",
    }
    .to_string()
}

/// 虚拟内容下发口径（对应 Java `OrderServiceImpl#maskUndeliveredVirtualContent`）：
/// 买家侧订单待发货（shipping_status = 0）时，虚拟（2）/卡密（3）商品项的
/// 虚拟内容遮蔽不下发；一旦发货（含部分发货）即下发商品的 `virtual_sample`。
/// 其它类型商品项不受遮蔽影响（与 Java 一致）。
fn virtual_sample_for(
    shipping_status: i32,
    product_type: i32,
    sample: Option<String>,
) -> Option<String> {
    if shipping_status == 0 && matches!(product_type, 2 | 3) {
        None
    } else {
        sample
    }
}

fn to_vo(o: &Order, items: &[OrderItem], claim: Option<&OfflinePaymentClaim>) -> OrderVO {
    let status_label = client_status_label(o.order_status);
    let item_values: Vec<serde_json::Value> = items
        .iter()
        .map(|it| {
            serde_json::json!({
                "itemId": it.item_id,
                "orderId": it.order_id,
                "productId": it.product_id,
                "productName": it.product_name,
                "productSn": it.product_sn,
                "picThumb": it.pic_thumb,
                "productImage": it.pic_thumb,
                "quantity": it.quantity,
                "price": it.price,
                "subtotal": it.price * Decimal::from(it.quantity),
                "productType": it.product_type,
                "shopId": it.shop_id,
                "skuId": it.sku_id,
                "skuData": serde_json::from_str::<serde_json::Value>(&it.sku_data)
                    .unwrap_or(serde_json::Value::Array(Vec::new())),
                "extraSkuData": it
                    .extra_sku_data
                    .as_deref()
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                    .unwrap_or(serde_json::Value::Array(Vec::new())),
                "seoLink": serde_json::Value::Null,
                "aftersalesItem": serde_json::Value::Null,
                "aftersalesNumber": 0,
                "eCard": serde_json::Value::Array(Vec::new()),
                "isGift": it.is_gift,
            })
        })
        .collect();
    let order_type = o.order_type.unwrap_or(0);
    let to_pay = o.order_status == 0;
    let to_comment = o.order_status == 5;
    // 线下认款入口：线下支付订单且已有认款单时可看详情；被驳回时可修改申请
    // （已取消订单不再提供入口，与 PC/H5 前端守卫口径一致）。
    let is_offline_order =
        o.pay_type_id == nextrade_core::models::trade::order_admin::pay_type::OFFLINE;
    let claim_detail = is_offline_order && claim.is_some() && o.order_status != status::CANCELED;
    let claim_edit = is_offline_order
        && claim.is_some_and(|c| c.status == OfflinePaymentClaim::STATUS_REJECTED)
        && o.order_status != status::CANCELED;
    let available_actions = serde_json::json!({
        "toPay": to_pay,
        "cancelOrder": to_pay,
        "delOrder": matches!(o.order_status, 3 | 4 | 5 | 40 | 50),
        "confirmReceipt": o.order_status == 2,
        "toAftersales": matches!(o.order_status, 2 | 5),
        "rebuy": true,
        "toComment": to_comment,
        "offlinePaymentClaimDetail": claim_detail,
        "editOfflinePaymentClaim": claim_edit,
    });
    let pay_status_name = match o.pay_status {
        0 => "未支付",
        1 => "支付中",
        2 => "已支付",
        3 => "退款中",
        4 => "已退款",
        _ => "未知状态",
    }
    .to_string();
    let shipping_status_name = match o.shipping_status {
        0 => "未发货",
        1 => "部分发货",
        2 => "已发货",
        3 => "已收货",
        _ => "未知状态",
    }
    .to_string();
    let user_address = {
        let regions = serde_json::from_str::<Vec<String>>(&o.region_names)
            .map(|v| v.join(""))
            .unwrap_or_else(|_| o.region_names.clone());
        format!("{regions}{}", o.address)
    };
    OrderVO {
        order_id: o.order_id,
        order_sn: o.order_sn.clone(),
        shop_id: o.shop_id,
        order_status: o.order_status,
        order_status_name: status_label.clone(),
        status_label,
        order_status_tip: status_tip(o.order_status),
        pay_status: o.pay_status,
        shipping_status: o.shipping_status,
        total_amount: o.total_amount,
        paid_amount: o.paid_amount,
        consignee: o.consignee.clone(),
        mobile: o.mobile.clone(),
        add_time: fmt_time(o.add_time),
        item_count: i32::try_from(items.len()).unwrap_or(0),
        items: item_values,
        available_actions,
        cancel_remark: o.cancel_remark.clone(),
        order_type: o.order_type,
        parent_order_sn: o.parent_order_sn.clone(),
        is_card_order: o.is_card_order != 0,
        is_cycle_order: o.is_cycle_order,
        is_pickup: matches!(o.shipping_method, 2),
        presale_info: None,
        order_source: Some(o.order_source.clone()),
        cycle_card_info: None,
        invoice_data: o
            .invoice_data
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok()),
        balance: o.balance,
        shipping_fee: o.shipping_fee,
        pay_status_name,
        shipping_status_name,
        pay_end_time: None,
        pay_time: if o.pay_time > 0 { Some(fmt_time(o.pay_time)) } else { None },
        shipping_time: if o.shipping_time > 0 { Some(fmt_time(o.shipping_time)) } else { None },
        received_time: if o.received_time > 0 { Some(fmt_time(o.received_time)) } else { None },
        buyer_note: o.buyer_note.clone(),
        online_paid_amount: o.online_paid_amount,
        offline_paid_amount: o.offline_paid_amount,
        credit_amount: o.credit_amount,
        invoice_fee: o.invoice_fee,
        shipping_type_name: o.shipping_type_name.clone(),
        tracking_no: o.tracking_no.clone(),
        logistics_name: o.logistics_name.clone(),
        user_address,
        pay_type_id: o.pay_type_id,
        product_amount: o.product_amount,
        coupon_amount: o.coupon_amount,
        points_amount: o.points_amount,
        discount_amount: o.discount_amount,
        service_fee: o.service_fee,
        unpaid_amount: o.unpaid_amount,
        is_presale: o.is_presale != 0,
        auto_delivery_days: None,
        join_status: None,
        join_status_desc: None,
        after_sales_ing: false,
        pickup_detail: None,
        delivery_order_info: None,
        order_reservation: None,
        is_reservation_home: false,
        shop: None,
        offline_payment_claim_status: claim.map(|c| c.status),
        offline_payment_claim_status_name: claim.map(|c| status_name(c.status)),
        offline_payment_claim_id: claim.map(|c| c.claim_id),
        offline_payment_claim_reject_reason: claim.and_then(|c| {
            (c.status == OfflinePaymentClaim::STATUS_REJECTED && !c.reject_reason.is_empty())
                .then(|| c.reject_reason.clone())
        }),
        offline_payment_claim: claim.map(to_offline_claim_vo),
        // 买家取消直退进度由 list/detail 后置附加（需查退款申请表）
        paid_cancel_refund_status: None,
        paid_cancel_refund_status_name: None,
    }
}

/// 买家取消直退入口可见性（纯函数，供单测）：设置主开关开 +
/// 订单处于「已支付未发货」+ 无进行中的买家取消申请。
/// 与服务端 `create_buyer_cancel` 的状态门同口径（拍卖/拼团除外）。
#[must_use]
pub(crate) fn paid_cancel_action_visible(
    settings_enabled: bool,
    order_status: i32,
    pay_status: i32,
    shipping_status: i32,
    is_valid: i32,
    order_type: Option<i32>,
    has_active_apply: bool,
) -> bool {
    settings_enabled
        && !has_active_apply
        && is_valid == 1
        && order_status == status::PAID
        && pay_status == 2
        && shipping_status == 0
        && !matches!(order_type, Some(11) | Some(2))
}

/// 给买家订单 VO 附加买家取消直退的入口动作与退款进度（我方新增）。
///
/// 设置主开关关时不加动作（买家侧无入口）；已存在的申请进度
/// 无论开关都展示，保证在途退款的进度可见性不随开关丢失。
fn attach_paid_cancel_refund(
    vo: &mut OrderVO,
    order: &Order,
    settings_enabled: bool,
    latest: Option<&nextrade_core::entities::RefundApply>,
) {
    if let Some(apply) = latest {
        vo.paid_cancel_refund_status = Some(apply.refund_status);
        vo.paid_cancel_refund_status_name =
            Some(paid_cancel_status_name(apply.refund_status).to_string());
    }
    let has_active = latest.is_some_and(|a| matches!(a.refund_status, 0 | 1));
    if paid_cancel_action_visible(
        settings_enabled,
        order.order_status,
        order.pay_status,
        order.shipping_status,
        order.is_valid,
        order.order_type,
        has_active,
    ) {
        if let Some(obj) = vo.available_actions.as_object_mut() {
            obj.insert(
                "cancelPaidRefund".to_string(),
                serde_json::Value::Bool(true),
            );
        }
    }
}

/// 买家取消直退进度展示名（与退款申请状态机同口径）。
#[must_use]
pub(crate) fn paid_cancel_status_name(status: i32) -> &'static str {
    match status {
        0 => "待审核",
        1 => "退款中",
        2 => "已退款",
        3 => "已拒绝",
        4 => "已退回",
        _ => "退款中",
        // 送礼物标记由 list/detail 在 to_vo 后回填（默认非礼物）
        is_gift_order: false,
        gift_status: None,
        gift_info: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 买家取消直退入口可见性：主开关关 = 恒不可见（零行为变化）。
    #[test]
    fn paid_cancel_action_visibility_matrix() {
        // (enabled, order_status, pay_status, shipping, is_valid, type, active)
        assert!(paid_cancel_action_visible(true, 1, 2, 0, 1, Some(1), false));
        // 主开关关
        assert!(!paid_cancel_action_visible(false, 1, 2, 0, 1, Some(1), false));
        // 已有进行中申请
        assert!(!paid_cancel_action_visible(true, 1, 2, 0, 1, Some(1), true));
        // 未支付 / 已发货 / 部分发货 / 父单 / 拍卖 / 拼团
        assert!(!paid_cancel_action_visible(true, 0, 0, 0, 1, Some(1), false));
        assert!(!paid_cancel_action_visible(true, 2, 2, 2, 1, Some(1), false));
        assert!(!paid_cancel_action_visible(true, 1, 2, 1, 1, Some(1), false));
        assert!(!paid_cancel_action_visible(true, 1, 2, 0, 0, Some(1), false));
        assert!(!paid_cancel_action_visible(true, 1, 2, 0, 1, Some(11), false));
        assert!(!paid_cancel_action_visible(true, 1, 2, 0, 1, Some(2), false));
    }

    #[test]
    fn test_order_create_dto_deserialization_with_invoice_and_o2o() {
        let json_str = r#"{
            "cartIds": [1, 2],
            "deliveryType": 2,
            "pickupId": 105,
            "pickupContactName": "王五",
            "pickupContactMobile": "13800138000",
            "invoiceData": {
                "invoiceType": 2,
                "titleType": 2,
                "companyName": "佰商科技",
                "taxNo": "91360100MA35XXXXXX",
                "content": "技术服务费",
                "email": "finance@nextrade.com"
            }
        }"#;

        let dto: OrderCreateDTO =
            serde_json::from_str(json_str).expect("deserialize OrderCreateDTO");
        assert_eq!(dto.cart_ids, vec![1, 2]);
        assert_eq!(dto.delivery_type, Some(2));
        assert_eq!(dto.pickup_id, Some(105));
        assert_eq!(dto.pickup_contact_name.as_deref(), Some("王五"));
        assert_eq!(dto.pickup_contact_mobile.as_deref(), Some("13800138000"));

        let inv = dto.invoice_data.expect("invoice_data present");
        assert_eq!(inv.invoice_type, Some(2));
        assert_eq!(inv.title_type, Some(2));
        assert_eq!(inv.company_name.as_deref(), Some("佰商科技"));
        assert_eq!(inv.tax_no.as_deref(), Some("91360100MA35XXXXXX"));
        assert_eq!(inv.content.as_deref(), Some("技术服务费"));
        assert_eq!(inv.email.as_deref(), Some("finance@nextrade.com"));
    }

    #[test]
    fn test_tax_amount_calculation_and_ceiling() {
        use crate::trade::check::calc_tax_amount;

        // 100 元，10% 税率 -> 10.00
        assert_eq!(calc_tax_amount(Decimal::new(10000, 2), Decimal::new(100, 1)), Decimal::new(1000, 2));

        // 0 元或 0 税率 -> 0
        assert_eq!(calc_tax_amount(Decimal::ZERO, Decimal::new(100, 1)), Decimal::ZERO);
        assert_eq!(calc_tax_amount(Decimal::new(10000, 2), Decimal::ZERO), Decimal::ZERO);

        // 向上取整 (CEILING)：10.55 元 * 6.5% = 0.68575 -> 0.69
        let res = calc_tax_amount(Decimal::new(1055, 2), Decimal::new(65, 1));
        assert_eq!(res, Decimal::new(69, 2));
    }
}

#[cfg(test)]
mod header_shop_id_tests {
    use super::header_shop_id;
    use nextrade_core::entities::OrderItem;

    fn item(shop_id: i32) -> OrderItem {
        OrderItem {
            shop_id,
            ..Default::default()
        }
    }

    #[test]
    fn single_shop_items_set_header() {
        assert_eq!(header_shop_id(&[item(7), item(7)]), 7);
        assert_eq!(header_shop_id(&[item(7)]), 7);
    }

    #[test]
    fn platform_only_items_keep_zero() {
        assert_eq!(header_shop_id(&[item(0), item(0)]), 0);
    }

    #[test]
    fn mixed_shops_keep_zero_pending_split_model() {
        assert_eq!(header_shop_id(&[item(7), item(9)]), 0);
        assert_eq!(header_shop_id(&[item(0), item(9)]), 0);
        assert_eq!(header_shop_id(&[]), 0);
    }
}

#[cfg(test)]
mod shipping_type_snapshot_tests {
    use super::shipping_type_snapshot;

    /// 原版 `OrderDeliveryTypeEnum` + 建单写入：deliveryType 原样落
    /// 订单头 shipping_type_id，名称取枚举描述。
    #[test]
    fn known_delivery_types_map_to_enum() {
        assert_eq!(shipping_type_snapshot(1), (1, "物流配送"));
        assert_eq!(shipping_type_snapshot(2), (2, "到店自提"));
        // 同城=3 是派送单生产者的触发值，必须原样透传
        assert_eq!(shipping_type_snapshot(3), (3, "同城配送"));
    }

    #[test]
    fn parse_pickup_shop_id_from_address_snapshot() {
        use super::parse_pickup_shop_id;
        // 建单自提快照原文
        assert_eq!(parse_pickup_shop_id("门店自提 (自提点ID: 105)"), Some(105));
        assert_eq!(parse_pickup_shop_id("门店自提 (自提点ID:105)"), Some(105));
        // 普通配送地址 / 空快照 / 非法 ID → None（回退店铺自提门店）
        assert_eq!(parse_pickup_shop_id("广东省深圳市南山区xx路1号"), None);
        assert_eq!(parse_pickup_shop_id(""), None);
        assert_eq!(parse_pickup_shop_id("门店自提 (自提点ID: 0)"), None);
        assert_eq!(parse_pickup_shop_id("门店自提 (自提点ID: )"), None);
    }

    /// 未知值：id 原样透传（Java `setShippingTypeId(param.getDeliveryType())`
    /// 不做白名单过滤），名称为空串（Java `getDesc` 未命中返回 ""）。
    #[test]
    fn unknown_delivery_type_passes_through_with_empty_name() {
        assert_eq!(shipping_type_snapshot(0), (0, ""));
        assert_eq!(shipping_type_snapshot(9), (9, ""));
    }
}

#[cfg(test)]
mod order_list_filter_tests {
    use super::{normalize_comment_status, normalize_is_showed, normalize_keyword};

    #[test]
    fn comment_status_minus_one_means_no_filter() {
        assert_eq!(normalize_comment_status(None), None);
        assert_eq!(normalize_comment_status(Some(-1)), None);
    }

    #[test]
    fn comment_status_zero_means_pending_and_positive_is_exact() {
        // 0 = 待评价（仓储层追加 order_status=5 条件）；>0 = 已评价等值
        assert_eq!(normalize_comment_status(Some(0)), Some(0));
        assert_eq!(normalize_comment_status(Some(1)), Some(1));
        assert_eq!(normalize_comment_status(Some(2)), Some(2));
    }

    #[test]
    fn keyword_blank_is_no_filter() {
        assert_eq!(normalize_keyword(None), None);
        assert_eq!(normalize_keyword(Some("   ")), None);
        assert_eq!(normalize_keyword(Some(" SN001 ")), Some("SN001"));
    }

    #[test]
    fn is_showed_only_filters_from_zero_upwards() {
        assert_eq!(normalize_is_showed(None), None);
        assert_eq!(normalize_is_showed(Some(-1)), None);
        assert_eq!(normalize_is_showed(Some(0)), Some(0));
        assert_eq!(normalize_is_showed(Some(1)), Some(1));
    }
}

#[cfg(test)]
mod order_contract_tests {
    use super::status_tip;
    use super::virtual_sample_for;

    #[test]
    fn status_tip_matches_java_order_status_enum() {
        assert_eq!(status_tip(0), "待支付");
        assert_eq!(status_tip(1), "已确认，待发货");
        assert_eq!(status_tip(2), "处理中，已发货");
        assert_eq!(status_tip(3), "已取消");
        assert_eq!(status_tip(4), "无效");
        assert_eq!(status_tip(5), "已完成");
        assert_eq!(status_tip(99), "");
    }

    #[test]
    fn virtual_sample_masked_until_shipped_for_virtual_and_card_items() {
        let sample = || Some("https://example.com/delivery".to_string());
        // 待发货（0）：虚拟（2）/卡密（3）遮蔽不下发
        assert_eq!(virtual_sample_for(0, 2, sample()), None);
        assert_eq!(virtual_sample_for(0, 3, sample()), None);
        // 部分发货（1）/已发货（2）/已收货（3）：下发
        assert_eq!(virtual_sample_for(1, 2, sample()), sample());
        assert_eq!(virtual_sample_for(2, 2, sample()), sample());
        assert_eq!(virtual_sample_for(3, 3, sample()), sample());
        // 实物（1）不受遮蔽影响（与 Java maskUndeliveredVirtualContent 一致）
        assert_eq!(virtual_sample_for(0, 1, sample()), sample());
        // 商品本身无虚拟内容时恒为 None
        assert_eq!(virtual_sample_for(2, 2, None), None);
    }
}

#[cfg(test)]
mod post_payment_hooks_tests {
    //! 结构断言（源码级，无 DB）：钉死「支付成功后置钩子单一入口」不变量——
    //! 任何把订单推到已付的路径都必须经 `run_post_payment_hooks` 触发全套
    //! 钩子，禁止再在单个支付路径里手抄子集（2026-11-21 批次 3 验收 P0：
    //! 余额支付只补了分销+骑手两钩、拆单激活缺失，跨店子单永不激活）。
    //! 切片区间以真实函数定义为界，测试模块位于文件尾，不会污染区间。

    const ORDER_RS: &str = include_str!("order.rs");
    const BALANCE_PAY_RS: &str = include_str!("../payment/balance_pay.rs");
    const ORDER_ADMIN_RS: &str = include_str!("order_admin.rs");

    /// 按起止标记切出函数体。
    fn fn_body<'a>(src: &'a str, start: &str, end: &str) -> &'a str {
        let s = src
            .find(start)
            .unwrap_or_else(|| panic!("未找到起点标记: {start}"));
        let e = src[s..]
            .find(end)
            .unwrap_or_else(|| panic!("未找到终点标记: {end}"))
            + s;
        &src[s..e]
    }

    #[test]
    fn shared_hook_entry_covers_all_eight_hooks() {
        let body = fn_body(
            ORDER_RS,
            "pub async fn run_post_payment_hooks",
            "pub async fn order_success_point",
        );
        for needle in [
            "insert_order_log",             // 1 订单日志
            "order_success_point",          // 2 积分赠送 Node 1
            "allocate_cards_to_order_item", // 3 电子卡券分配
            "insert_statement_output",      // 4 对账单出账
            "SalesmanGenerationService",    // 5 分销生成
            "DeliveryDispatchService",      // 6 骑手派送
            "split_store_order",            // 7 跨店拆单激活
            "ShopMemberService",            // 8 店铺会员记账
        ] {
            assert!(
                body.contains(needle),
                "run_post_payment_hooks 缺钩子: {needle}"
            );
        }
    }

    #[test]
    fn on_payment_success_delegates_without_inlining_hooks() {
        let body = fn_body(
            ORDER_RS,
            "pub async fn on_payment_success",
            "pub async fn run_post_payment_hooks",
        );
        assert!(body.contains("run_post_payment_hooks"));
        // 钩子实现只能在共享入口里有一份，on_payment_success 不得再内联
        assert!(!body.contains("split_store_order"));
        assert!(!body.contains("insert_statement_output"));
        assert!(!body.contains("allocate_cards_to_order_item"));
    }

    #[test]
    fn balance_pay_full_pay_goes_through_shared_entry() {
        let body = fn_body(
            BALANCE_PAY_RS,
            "pub async fn pay_order",
            "pub async fn refund",
        );
        assert!(body.contains("run_post_payment_hooks"));
        // 旧手抄补偿块（只补分销+骑手两钩）必须已拆除，防止子集接线回潮
        assert!(!body.contains("SalesmanGenerationService"));
        assert!(!body.contains("DeliveryDispatchService"));
    }

    #[test]
    fn admin_set_paid_covers_both_branches() {
        let body = fn_body(
            ORDER_ADMIN_RS,
            "pub async fn set_paid",
            "pub async fn cancel_order",
        );
        // 非预售分支经 on_payment_success；预售尾款分支直调共享入口
        assert!(body.contains("on_payment_success"));
        assert!(body.contains("run_post_payment_hooks"));
    }
}
