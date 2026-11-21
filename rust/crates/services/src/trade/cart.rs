//! 购物车服务：加购、改量、勾选、删除、价格汇总。
//! 对应原 Java `CartServiceImpl`。

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;
use serde::Serialize;
use nextrade_core::entities::{Cart, Product, ProductSku};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

use crate::catalog::product::ProductService;

/// 带价附加属性解析结果（加购落库用）。
#[derive(Debug, Default)]
struct ExtraAttrsResolved {
    /// 命中的 `attributes_id`（用于判断是否携带附加属性）
    ids: Vec<i32>,
    /// `cart.extra_sku_data` JSON 串（未命中为 None）
    json: Option<String>,
    /// Σ`attr_price`（计入行到手价）
    total_price: Decimal,
}

/// 购物车行 VO（Java `CartVO` 前端使用子集）。
///
/// H5/PC 购物车页直接读取：cartId/productStatus/stock/price/originPrice/
/// quantity/isChecked(bool)/skuData(数组)/productName/picThumb/skuId 等
/// （java/view/uniapp `pages/cart/index.vue`）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CartItemVO {
    pub cart_id: i32,
    pub user_id: i32,
    pub product_id: i32,
    pub pic_thumb: String,
    pub market_price: Decimal,
    /// Java `originalPrice` 为 String（划线价展示）。
    pub original_price: String,
    pub quantity: i32,
    pub sku_id: Option<i32>,
    pub sku: Option<CartSkuState>,
    pub shop_id: i32,
    pub r#type: i32,
    pub update_time: i64,
    pub product_sn: String,
    pub product_name: String,
    pub product_type: Option<i32>,
    pub product_stock: i32,
    /// 限购上限（Java `CartServiceImpl:433` `cartVO.setLimitNumber(product.getLimitNumber())`）。
    /// 0 表示不限购。
    pub limit_number: i32,
    /// 起售量（件）：0/1 不限，>=2 生效（购物车步进器下限与改量校验共用）。
    pub min_buy_count: i32,
    /// 1 上架 / 0 下架。
    pub product_status: i32,
    pub sale_status: i32,
    pub sale_mode: i32,
    pub product_price: Decimal,
    pub member_price: Decimal,
    /// 行生效单价。
    pub price: Decimal,
    /// 划线原价。
    pub origin_price: Decimal,
    /// 当前可售库存。
    pub stock: i32,
    pub is_checked: bool,
    /// 规格值数组（Java JSONArray；H5 `getSkuValueText` 按数组读）。
    pub sku_data: serde_json::Value,
    pub has_sku: bool,
    pub subtotal: Decimal,
    pub is_disabled: bool,
    pub disabled_reason: String,
    pub activity_info: Vec<serde_json::Value>,
    pub extra_sku_data: Vec<serde_json::Value>,
    pub service_fee: String,
    pub wholesale_multiple: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CartSkuState {
    pub is_enabled: i32,
}

/// 店铺分组合计（Java `CartByStoreVO.CartStoreTotal`，按已勾选且可购买的商品计算）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CartStoreTotalVO {
    pub coupon_ids: Vec<i32>,
    pub product_amount: Decimal,
    pub total_count: i64,
    pub checked_count: i64,
    pub discounts: Decimal,
    pub discount_after: Decimal,
    pub service_fee: Decimal,
}

/// 店铺分组（Java `CartByStoreVO`）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CartByStoreVO {
    pub total: CartStoreTotalVO,
    pub carts: Vec<CartItemVO>,
    pub used_promotions: Vec<serde_json::Value>,
    pub enable_use_promotion: Vec<serde_json::Value>,
    pub gift: Vec<serde_json::Value>,
    pub shop_id: i32,
    pub shop_title: String,
    pub no_shipping: i32,
    pub is_not_reach_min_order_amount: bool,
    pub min_order_amount_diff: Decimal,
    pub wholesale_group_hint: String,
    pub wholesale_configured: bool,
    /// H5 店铺勾选框 v-model（Java 端由前端回填，这里直接给初值）。
    pub is_checked: bool,
    /// H5 列表 :key 用（Java 端无此字段，取 shopId 保证稳定）。
    pub cart_id: i32,
}

/// 购物车总合计（Java `CartListByStoreVO.CartTotal`，按已勾选且可购买的商品计算）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CartTotalVO {
    pub product_amount: Decimal,
    pub checked_count: i64,
    pub discounts: Decimal,
    pub discount_after: Decimal,
    pub discount_change_price: Decimal,
    pub pos_other_discount_amount: Decimal,
    pub total_count: i64,
    pub discount_coupon_amount: Decimal,
    pub discount_seckill_amount: Decimal,
    pub discount_product_promotion_amount: Decimal,
    pub discount_time_discount_amount: Decimal,
    pub discount_group_buy_amount: Decimal,
    pub discount_discount_amount: Decimal,
    pub round_off_cents_amount: Decimal,
    pub round_off_dime_amount: Decimal,
    pub service_fee: Decimal,
}

/// 购物车列表（Java `CartListByStoreVO`，按店铺分组）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CartListByStoreVO {
    pub total: CartTotalVO,
    pub has_change_price: bool,
    pub cart_list: Vec<CartByStoreVO>,
    pub is_wholesale_buyer: bool,
}

#[derive(Clone)]
pub struct CartService {
    repos: Arc<Repositories>,
    product: ProductService,
}

impl CartService {
    pub fn new(repos: Arc<Repositories>, product: ProductService) -> Self {
        Self { repos, product }
    }

    /// 加入购物车（同款累加数量；拼团上下文以独立行保存）。
    ///
    /// `is_quick = true` 表示“立即购买”：先取消该会员其它购物车行的勾选，
    /// 保证结算页只带当前这一件商品。
    ///
    /// `extra_attr_ids`：带价附加属性 `product_attributes.attributes_id` 逗号串
    /// （旧版 `ProductAttr.vue` `attr_type=2` 选中后提交，Java `AddToCartDTO.extraAttrIds`）。
    /// 对齐 Java `CartServiceImpl`：Σ`attr_price` 计入行到手价，并把属性明细
    /// （attributesId/attrName/attrValue/attrPrice）JSON 落 `cart.extra_sku_data`。
    pub async fn add(
        &self,
        user_id: i32,
        product_id: i32,
        sku_id: Option<i32>,
        quantity: i32,
        groupon_data: Option<&nextrade_core::models::trade::check::GrouponCartData>,
        is_quick: bool,
        extra_attr_ids: Option<&str>,
    ) -> Result<i32> {
        if quantity <= 0 {
            return Err(Error::Validation("数量必须大于0".into()));
        }
        // 店铺小程序作用域（我方新增）：他店商品禁止加购。显式按商品
        // 归属校验——下方 detail() 的作用域错误会被 unwrap_or(0) 吞掉，
        // 不能依赖它拦截。
        if let Some(bound_shop_id) =
            nextrade_infrastructure::db::repo::member::mini_shop_scope::MiniShopScope::current()
                .shop_id()
        {
            let product = self
                .repos
                .catalog
                .product
                .find_by_id(product_id)
                .await?
                .ok_or_else(|| {
                    Error::NotFound(format!("商品 {product_id} 不存在"))
                })?;
            if product.shop_id != bound_shop_id {
                return Err(Error::Forbidden(
                    "该商品不属于当前店铺，无法在此小程序购买".into(),
                ));
            }
        }
        // 价格门控购买拦截（第 2 期，用户决策 3）：isIdentity 开启时未认证
        // 用户不能加购，否则购物车漏价绕过看价门控。门控关闭时直接放行。
        // 询价豁免（2026-11-20 裁定）：命中询价商品时不在此拦截，由下方
        // check_purchasable 给出询价一期文案「该商品为询价商品，请提交询价」。
        let inquiry_exempt = self.product.is_inquiry_target(product_id, sku_id).await?;
        crate::system::price_gate::ensure_purchase_allowed_for(
            &self.repos,
            user_id,
            inquiry_exempt,
        )
        .await?;
        // 商品归属店铺：黑名单判定与购物车行 shop_id 落值共用同一口径。
        // 购物车行 shop_id 必须等于商品归属店铺——下游购物车按店分组、结算
        // 分店金额 / 运费模板 / 满减匹配、订单明细 shop_id 归属都取行上的
        // shop_id，漏赋值恒为 0 会把商家商品全记到平台自营名下。
        let product_shop_id = self
            .product
            .detail(product_id)
            .await
            .map(|v| v.shop_id)
            .unwrap_or(0);
        // 客户黑名单（下单限制，先人后货）：按商品归属店铺判定，命中即拒绝。
        // 文案见 blacklist-design §7（用户端不出现「黑名单」字样）。
        {
            crate::member::user_blacklist::UserBlacklistService::new(self.repos.clone())
                .ensure_allowed(
                    user_id,
                    nextrade_core::models::member::user_blacklist::PRODUCT_MALL,
                    nextrade_core::models::member::user_blacklist::BEHAVIOR_PLACE_ORDER,
                    Some(product_shop_id),
                )
                .await?;
        }
        // 起售量校验（合并同商品行后判定）：本会员购物车内该商品现有
        // 普通行数量合计 + 本次数量须 >= 起售量；拼团行是活动独立行不计入。
        if groupon_data.is_none() {
            let existing_total: i32 = self
                .repos
                .trade
                .cart
                .find_by_user(user_id, None)
                .await?
                .iter()
                .filter(|l| l.product_id == product_id && l.extra_data.is_none())
                .map(|l| l.quantity)
                .sum();
            self.product
                .ensure_min_buy(product_id, existing_total + quantity)
                .await?;
        }
        // 校验商品可购
        self.product.check_purchasable(product_id, sku_id, quantity).await?;

        // 附加属性明细 + 价格合计（Java `CartServiceImpl:1615` str2IntList →
        // `getProductAttributesByIds` → `CarExtraSkuDataDTO` 落 `cart.extraSkuData`）
        let extra = self.resolve_extra_attrs(extra_attr_ids).await?;
        let price = self
            .product
            .detail(product_id)
            .await
            .map(|v| v.product_price)
            .unwrap_or_default()
            + extra.total_price;

        if is_quick {
            self.repos.trade.cart.uncheck_all(user_id).await?;
        }

        // 已存在同款则累加（拼团商品不与普通购物车行合并）
        if groupon_data.is_none() {
            if let Some(existing) = self
                .repos.trade.cart
                .find_line(user_id, product_id, sku_id)
                .await?
            {
                let new_qty = existing.quantity + quantity;
                self.product.check_purchasable(product_id, sku_id, new_qty).await?;
                let cart_id = existing.cart_id;
                self.repos.trade.cart.update_quantity(cart_id, user_id, new_qty).await?;
                // Java 合并是「删旧行 + 按本次选择重插」，extraSkuData / 到手价随之刷新
                if !extra.ids.is_empty() {
                    self.repos.trade.cart
                        .update_extra_sku_data(cart_id, user_id, extra.json.as_deref(), price)
                        .await?;
                }
                if is_quick {
                    self.repos.trade.cart.update_checked(cart_id, user_id, true).await?;
                }
                return Ok(cart_id);
            }
        }

        let cart = Cart {
            user_id,
            product_id,
            sku_id,
            quantity,
            original_price: price,
            market_price: Some(price),
            is_checked: 1,
            shop_id: product_shop_id,
            r#type: 0,
            extra_data: groupon_data
                .and_then(|g| serde_json::to_string(g).ok()),
            extra_sku_data: extra.json,
            ..Default::default()
        };
        self.repos.trade.cart.insert(&cart).await
    }

    /// 回填加购归因分销员（原版 cart.salesman_id 暂存归因）：
    /// 加购请求带 salesmanId 时由调用方在 `add` 成功后调用；仅当行上现值
    /// 为 0 时才写入，不覆盖既有归因（防串单抢归因）。
    pub async fn attribute_salesman(&self, user_id: i32, cart_id: i32, salesman_id: i32) -> Result<()> {
        if salesman_id <= 0 {
            return Ok(());
        }
        self.repos
            .trade
            .cart
            .update_salesman_id(cart_id, user_id, salesman_id)
            .await
    }

    /// 礼品卡兑换专用加车（useCard 调用）：独立行写入，不与普通行合并，
    /// `extra_data` 标记 `{"giftCardsId": …}` 供结算 flowType=12 与下单
    /// 关联 order_item.gift_card_id；首行先取消该会员其它行勾选，保证
    /// 兑换结算只带本批兑换商品。库存/可购校验走 `check_purchasable` 封顶。
    pub async fn add_gift_exchange_line(
        &self,
        user_id: i32,
        product_id: i32,
        sku_id: i32,
        quantity: i32,
        gift_cards_id: i32,
        uncheck_others: bool,
    ) -> Result<i32> {
        if quantity <= 0 {
            return Err(Error::Validation("数量必须大于0".into()));
        }
        let sku = if sku_id > 0 { Some(sku_id) } else { None };
        self.product.check_purchasable(product_id, sku, quantity).await?;
        if uncheck_others {
            self.repos.trade.cart.uncheck_all(user_id).await?;
        }
        let price = self
            .product
            .detail(product_id)
            .await
            .map(|v| v.product_price)
            .unwrap_or_default();
        let cart = Cart {
            user_id,
            product_id,
            sku_id: sku,
            quantity,
            original_price: price,
            market_price: Some(price),
            is_checked: 1,
            r#type: 0,
            extra_data: Some(
                serde_json::json!({ "giftCardsId": gift_cards_id }).to_string(),
            ),
            ..Default::default()
        };
        self.repos.trade.cart.insert(&cart).await
    }

    /// 解析带价附加属性：id 串 → `product_attributes` 明细 + 价格合计。
    ///
    /// JSON 形状对齐 Java `CarExtraSkuDataDTO`（`JSONUtil.toJsonStr`）：
    /// `[{attributesId, attrName, attrValue, attrPrice}]`。
    async fn resolve_extra_attrs(
        &self,
        extra_attr_ids: Option<&str>,
    ) -> Result<ExtraAttrsResolved> {
        let ids = super::proxy_shopping_product::parse_extra_attr_ids(extra_attr_ids);
        if ids.is_empty() {
            return Ok(ExtraAttrsResolved::default());
        }
        let rows = self
            .repos
            .trade
            .proxy_shopping_product
            .find_attribute_details(&ids)
            .await?;
        let mut total = Decimal::ZERO;
        let items: Vec<serde_json::Value> = rows
            .iter()
            .map(|(id, name, value, price)| {
                if let Some(p) = price {
                    total += *p;
                }
                serde_json::json!({
                    "attributesId": id,
                    "attrName": name,
                    "attrValue": value,
                    "attrPrice": price.map(|p| p.to_string()),
                })
            })
            .collect();
        let json = if items.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&items).unwrap_or_default())
        };
        Ok(ExtraAttrsResolved { ids, json, total_price: total })
    }

    /// 修改数量。
    pub async fn update_quantity(
        &self,
        user_id: i32,
        cart_id: i32,
        quantity: i32,
    ) -> Result<()> {
        if quantity <= 0 {
            return Err(Error::Validation("数量必须大于0".into()));
        }
        let cart = self
            .repos.trade.cart
            .find_owned(cart_id, user_id)
            .await?
            .ok_or_else(|| Error::NotFound("购物车记录不存在".into()))?;
        // 起售量校验（合并同商品行后判定）：其余普通行合计 + 本次目标
        // 数量须 >= 起售量（删除行走 remove，不受此限）。
        {
            let other_total: i32 = self
                .repos
                .trade
                .cart
                .find_by_user(user_id, None)
                .await?
                .iter()
                .filter(|l| {
                    l.product_id == cart.product_id && l.cart_id != cart_id && l.extra_data.is_none()
                })
                .map(|l| l.quantity)
                .sum();
            self.product
                .ensure_min_buy(cart.product_id, other_total + quantity)
                .await?;
        }
        // 价格门控购买拦截（第 2 期，用户决策 3）：与加购同一口径，
        // 含询价豁免（2026-11-20 裁定）——命中询价时由下方
        // check_purchasable 给出询价一期文案。
        let sku_id = cart.sku_id.filter(|id| *id > 0);
        let inquiry_exempt = self
            .product
            .is_inquiry_target(cart.product_id, sku_id)
            .await?;
        crate::system::price_gate::ensure_purchase_allowed_for(
            &self.repos,
            user_id,
            inquiry_exempt,
        )
        .await?;
        self.product
            .check_purchasable(cart.product_id, sku_id, quantity)
            .await?;
        self.repos.trade.cart.update_quantity(cart_id, user_id, quantity).await
    }

    /// 勾选/取消勾选。
    pub async fn set_checked(&self, user_id: i32, cart_id: i32, checked: bool) -> Result<()> {
        self.repos.trade.cart.update_checked(cart_id, user_id, checked).await
    }

    /// 删除指定行。
    pub async fn remove(&self, user_id: i32, cart_ids: &[i32]) -> Result<()> {
        self.repos.trade.cart.delete(cart_ids, user_id).await?;
        Ok(())
    }

    /// 清空。
    pub async fn clear(&self, user_id: i32) -> Result<()> {
        self.repos.trade.cart.clear(user_id).await?;
        Ok(())
    }

    /// 购物车列表：实时计算可售状态、SKU 现价和已选商品优惠。
    pub async fn summary(&self, user_id: i32) -> Result<CartListByStoreVO> {
        let carts = self.repos.trade.cart.find_by_user(user_id, None).await?;
        let product_ids: Vec<_> = carts.iter().map(|cart| cart.product_id).collect();
        let products = super::check::load_products(&self.repos, &product_ids).await?;
        // 店铺小程序作用域（我方新增）：购物车跨端共享，店铺小程序内
        // 只展示本店商品行（按商品归属过滤，不信任行上可能过期的
        // shop_id 落值）；平台作用域不过滤，行为不变。
        let carts = match nextrade_infrastructure::db::repo::member::mini_shop_scope::MiniShopScope::current()
            .shop_id()
        {
            Some(bound_shop_id) => carts
                .into_iter()
                .filter(|c| {
                    products
                        .get(&c.product_id)
                        .is_some_and(|p| p.shop_id == bound_shop_id)
                })
                .collect::<Vec<_>>(),
            None => carts,
        };
        let skus = load_cart_skus(&self.repos, &carts).await?;
        let mut sale_states = HashMap::new();
        for product in products.values().filter(|product| product.is_on_sale()) {
            let state = self
                .repos
                .catalog
                .product_sale
                .setting(product.product_id, None)
                .await?;
            sale_states.insert(product.product_id, state.sale_status);
        }

        let mut selected = Vec::new();
        let mut invalid_checked_ids = Vec::new();
        for cart in carts.iter().filter(|cart| cart.is_checked()) {
            let product = products.get(&cart.product_id);
            let sku = cart_line_sku(&skus, cart);
            let sale_status = sale_states.get(&cart.product_id).copied().unwrap_or(1);
            if disabled_reason(product, sku, cart, sale_status).is_empty() {
                selected.push(cart.clone());
            } else {
                invalid_checked_ids.push(cart.cart_id);
            }
        }
        self.repos
            .trade
            .cart
            .uncheck_lines(user_id, &invalid_checked_ids)
            .await?;

        let now = nextrade_common::utils::now_seconds() as i32;
        let pricing = super::check::price_cart_activities(
            &self.repos,
            user_id,
            &selected,
            &products,
            &skus,
            now,
            None,
        )
        .await?;
        let promotions = if carts.is_empty() {
            Vec::new()
        } else {
            self.repos
                .marketing
                .promotion
                .find_available(None, i64::from(now))
                .await?
        };

        let mut shop_order = Vec::new();
        let mut groups: HashMap<i32, Vec<Cart>> = HashMap::new();
        for cart in carts {
            groups
                .entry(cart.shop_id)
                .or_insert_with(|| {
                    shop_order.push(cart.shop_id);
                    Vec::new()
                })
                .push(cart);
        }

        let mut total = CartTotalVO::default();
        let mut cart_list = Vec::with_capacity(shop_order.len());
        for shop_id in shop_order {
            let shop_title = if shop_id > 0 {
                self.repos
                    .merchant
                    .shop
                    .find_by_id(shop_id)
                    .await?
                    .and_then(|shop| shop.shop_title)
                    .unwrap_or_default()
            } else {
                String::new()
            };
            let mut store = CartStoreTotalVO::default();
            let mut lines = Vec::new();
            for cart in groups.remove(&shop_id).unwrap_or_default() {
                let product = products.get(&cart.product_id);
                let sku = cart_line_sku(&skus, &cart);
                let sale_status = sale_states.get(&cart.product_id).copied().unwrap_or(1);
                let reason = disabled_reason(product, sku, &cart, sale_status);
                let sku_id = cart.sku_id.filter(|id| *id > 0);
                let is_checked = cart.is_checked() && reason.is_empty();
                let quantity = Decimal::from(cart.quantity);
                let base_price = cart_base_price(product, sku, &cart);
                let price = pricing
                    .prices
                    .get(&cart.cart_id)
                    .copied()
                    .unwrap_or(base_price);
                let stock = cart_line_stock(product, sku, &cart);
                let (extra_sku_data, service_fee) = cart_extra_prices(&cart);
                let market_price = sku
                    .and_then(|s| s.market_price)
                    .or_else(|| product.map(|p| p.market_price))
                    .unwrap_or_default();
                let origin_price = if price < base_price {
                    base_price
                } else {
                    market_price.max(price)
                };
                let sku_data = parse_cart_json_array(
                    sku.and_then(|s| s.sku_data.as_deref())
                        .or(cart.sku_data.as_deref()),
                );
                let has_sku = sku_id.is_some() || !sku_data.is_empty();

                let product_shop_id = product.map_or(shop_id, |p| p.shop_id);
                let activity_info = promotions.iter()
                    .filter(|pr| pr.shop_id == product_shop_id)
                    .filter(|pr| crate::catalog::product_manage_client::promotion_covers_product(pr, cart.product_id))
                    .map(|pr| serde_json::json!({
                        "promotionId": pr.promotion_id,
                        "promotionName": pr.promotion_name,
                        "startTime": pr.start_time,
                        "endTime": pr.end_time,
                        "type": pr.r#type,
                        "shopId": pr.shop_id,
                        "relationId": pr.relation_id,
                        "range": pr.range,
                        "typeText": nextrade_core::models::marketing::promotion::type_text(pr.r#type),
                    }))
                    .collect();

                let subtotal = price * quantity;
                if is_checked {
                    store.checked_count += i64::from(cart.quantity);
                    store.product_amount += base_price * quantity;
                    store.discount_after += subtotal + service_fee;
                    store.discounts += (base_price - price).max(Decimal::ZERO) * quantity;
                    store.service_fee += service_fee;
                }
                store.total_count += i64::from(cart.quantity);
                lines.push(CartItemVO {
                    cart_id: cart.cart_id,
                    user_id,
                    product_id: cart.product_id,
                    pic_thumb: product
                        .map(|p| p.pic_thumb.clone())
                        .or(cart.pic_thumb)
                        .unwrap_or_default(),
                    market_price,
                    original_price: origin_price.to_string(),
                    quantity: cart.quantity,
                    sku_id,
                    sku: sku_id.map(|_| CartSkuState {
                        is_enabled: sku.map_or(0, |s| s.is_enabled),
                    }),
                    shop_id,
                    r#type: cart.r#type,
                    update_time: i64::from(cart.update_time),
                    product_sn: product
                        .map(|p| p.product_sn.clone())
                        .unwrap_or(cart.product_sn),
                    product_name: product.map(|p| p.product_name.clone()).unwrap_or_default(),
                    product_type: product
                        .and_then(|p| p.product_type)
                        .or(Some(cart.product_type)),
                    product_stock: product.map_or(0, |p| p.product_stock),
                    limit_number: product.map_or(0, |p| p.limit_number),
                    min_buy_count: product.map_or(0, |p| p.min_buy_count),
                    product_status: i32::from(product.is_some_and(Product::is_on_sale)),
                    sale_status,
                    sale_mode: product.map_or(0, |p| p.sale_mode),
                    product_price: product.map_or(base_price, |p| p.product_price),
                    member_price: base_price,
                    price,
                    origin_price,
                    stock,
                    is_checked,
                    has_sku,
                    sku_data: serde_json::Value::Array(sku_data),
                    subtotal,
                    is_disabled: !reason.is_empty(),
                    disabled_reason: reason.to_owned(),
                    activity_info,
                    extra_sku_data,
                    service_fee: service_fee.to_string(),
                    wholesale_multiple: 1,
                });
            }

            total.checked_count += store.checked_count;
            total.total_count += store.total_count;
            total.product_amount += store.product_amount;
            total.discount_after += store.discount_after;
            total.discounts += store.discounts;
            total.service_fee += store.service_fee;
            let mut selectable = lines.iter().filter(|line| !line.is_disabled).peekable();
            let is_checked = selectable.peek().is_some() && selectable.all(|line| line.is_checked);
            let gift = pricing
                .gifts
                .iter()
                .filter(|gift| gift.shop_id == shop_id)
                .map(|gift| {
                    serde_json::json!({
                        "giftId": gift.gift_id,
                        "productId": gift.product_id,
                        "skuId": gift.sku_id,
                        "productName": gift.product_name,
                        "productSn": gift.product_sn,
                        "picThumb": gift.pic_thumb,
                        "num": gift.num,
                        "promotionId": gift.promotion_id,
                    })
                })
                .collect();
            cart_list.push(CartByStoreVO {
                total: store,
                carts: lines,
                used_promotions: vec![],
                enable_use_promotion: vec![],
                gift,
                shop_id,
                shop_title,
                no_shipping: 0,
                is_not_reach_min_order_amount: false,
                min_order_amount_diff: Decimal::ZERO,
                wholesale_group_hint: String::new(),
                wholesale_configured: false,
                is_checked,
                cart_id: shop_id,
            });
        }
        total.discount_seckill_amount = pricing.seckill.values().copied().sum();
        total.discount_group_buy_amount = pricing.groupon.values().copied().sum();
        total.discount_time_discount_amount = pricing.time.values().copied().sum();
        total.discount_product_promotion_amount = pricing.full_reduce.values().copied().sum();
        total.discount_discount_amount = total.discounts;
        Ok(CartListByStoreVO {
            total,
            has_change_price: false,
            cart_list,
            is_wholesale_buyer: false,
        })
    }

    /// 获取已勾选且可结算的购物车行（下单用）。
    pub async fn checked_lines(&self, user_id: i32, cart_ids: &[i32]) -> Result<Vec<Cart>> {
        let carts = self.repos.trade.cart.find_by_user(user_id, None).await?;
        Ok(carts
            .into_iter()
            .filter(|c| c.is_checked() && (cart_ids.is_empty() || cart_ids.contains(&c.cart_id)))
            .collect())
    }
}

pub(crate) async fn load_cart_skus(
    repos: &Repositories,
    carts: &[Cart],
) -> Result<HashMap<i32, ProductSku>> {
    let ids: Vec<_> = carts
        .iter()
        .filter_map(|cart| cart.sku_id.filter(|id| *id > 0))
        .collect();
    Ok(repos
        .catalog
        .product
        .find_skus_by_ids(&ids)
        .await?
        .into_iter()
        .map(|sku| (sku.sku_id, sku))
        .collect())
}

pub(crate) fn cart_line_sku<'a>(
    skus: &'a HashMap<i32, ProductSku>,
    cart: &Cart,
) -> Option<&'a ProductSku> {
    cart.sku_id
        .and_then(|id| skus.get(&id))
        .filter(|sku| sku.product_id == cart.product_id)
}

pub(crate) fn cart_base_price(
    product: Option<&Product>,
    sku: Option<&ProductSku>,
    cart: &Cart,
) -> Decimal {
    sku.and_then(|s| s.sku_price)
        .or_else(|| product.map(|p| p.product_price))
        .unwrap_or(cart.original_price)
}

pub(crate) fn cart_line_stock(
    product: Option<&Product>,
    sku: Option<&ProductSku>,
    cart: &Cart,
) -> i32 {
    if cart.sku_id.is_some_and(|id| id > 0) {
        sku.map_or(0, |s| s.available_stock() as i32)
    } else {
        product.map_or(0, |p| p.product_stock.max(0))
    }
}

fn disabled_reason(
    product: Option<&Product>,
    sku: Option<&ProductSku>,
    cart: &Cart,
    sale_status: i32,
) -> &'static str {
    let Some(product) = product.filter(|product| product.is_on_sale()) else {
        return "product_offline";
    };
    if sale_status == 0 {
        return "product_offline";
    }
    if cart.sku_id.is_some_and(|id| id > 0) && !sku.is_some_and(ProductSku::is_available) {
        return "sku_disabled";
    }
    if product.is_inquiry == 1 || sku.is_some_and(|s| s.is_inquiry == 1) {
        return "inquiry";
    }
    if !product.is_virtual_product()
        && !nextrade_core::models::marketing::groupon_promotion_item::is_reservation_product(
            product.product_type,
        )
    {
        let stock = cart_line_stock(Some(product), sku, cart);
        if stock == 0 {
            return "sold_out";
        }
        if stock < cart.quantity {
            return "insufficient_stock";
        }
    }
    ""
}

fn parse_cart_json_array(raw: Option<&str>) -> Vec<serde_json::Value> {
    raw.and_then(|value| serde_json::from_str(value).ok())
        .unwrap_or_default()
}

fn extra_attr_price(attr: &serde_json::Value) -> Decimal {
    let Some(price) = attr.get("attrPrice") else {
        return Decimal::ZERO;
    };
    match price {
        serde_json::Value::String(value) => value.parse().unwrap_or_default(),
        serde_json::Value::Number(value) => value.to_string().parse().unwrap_or_default(),
        _ => Decimal::ZERO,
    }
}

pub(crate) fn cart_extra_price(cart: &Cart) -> Decimal {
    parse_cart_json_array(cart.extra_sku_data.as_deref())
        .iter()
        .map(extra_attr_price)
        .sum()
}

fn cart_extra_prices(cart: &Cart) -> (Vec<serde_json::Value>, Decimal) {
    let mut attrs = parse_cart_json_array(cart.extra_sku_data.as_deref());
    let mut total = Decimal::ZERO;
    for attr in &mut attrs {
        let price = (extra_attr_price(attr) * Decimal::from(cart.quantity)).round_dp(2);
        if let Some(fields) = attr.as_object_mut() {
            fields.insert(
                "totalAttrPrice".into(),
                serde_json::json!(price.to_string()),
            );
        }
        total += price;
    }
    (attrs, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cart_uses_selected_sku_price_and_stock_for_existing_rows() {
        let product = Product {
            product_id: 20,
            product_status: 1,
            product_stock: 100,
            product_price: Decimal::from(40),
            ..Default::default()
        };
        let cart = Cart {
            product_id: 20,
            sku_id: Some(5),
            quantity: 3,
            original_price: Decimal::from(35),
            ..Default::default()
        };
        let sku = ProductSku {
            sku_id: 5,
            product_id: 20,
            sku_price: Some(Decimal::from(45)),
            sku_stock: Some(2),
            is_enabled: 1,
            ..Default::default()
        };
        let skus = HashMap::from([(5, sku)]);
        let sku = cart_line_sku(&skus, &cart);
        assert_eq!(
            cart_base_price(Some(&product), sku, &cart),
            Decimal::from(45)
        );
        assert_eq!(cart_line_stock(Some(&product), sku, &cart), 2);
        assert_eq!(
            disabled_reason(Some(&product), sku, &cart, 1),
            "insufficient_stock"
        );
        assert_eq!(
            disabled_reason(Some(&product), None, &cart, 1),
            "sku_disabled"
        );
        assert_eq!(
            disabled_reason(Some(&product), sku, &cart, 0),
            "product_offline"
        );
    }

    #[test]
    fn cart_adds_each_extra_fee_once_per_item() {
        let cart = Cart {
            quantity: 3,
            extra_sku_data: Some(r#"[{"attrPrice":"1.25"},{"attrPrice":2}]"#.into()),
            ..Default::default()
        };
        assert_eq!(cart_extra_price(&cart), Decimal::new(325, 2));
        let (attrs, total) = cart_extra_prices(&cart);
        assert_eq!(attrs[0]["totalAttrPrice"], "3.75");
        assert_eq!(total, Decimal::new(975, 2));
    }
}
