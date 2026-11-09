//! 代客下单商品管理域服务（trade::proxy_shopping_product）。
//!
//! 对应原 Java `com.nextrade.service.product.impl.ProductServiceImpl` 的三个入口，
//! 由 `ProxyShoppingProductController`
//! （`@RequestMapping("/adminapi/proxyShopping/product")`）暴露：
//!
//! | 本服务方法 | 原 Java |
//! |-----------|---------|
//! | [`ProxyShoppingProductService::product_detail`] | `ProxyShoppingProductController.detail` → `ProductServiceImpl#clientDetail` |
//! | [`ProxyShoppingProductService::get_product_availability`] | `ProductServiceImpl#getProductSkuDetail(productId, skuId, 0, extraAttrIds, shopId, addressId, userId)` |
//! | [`ProxyShoppingProductService::product_list`] | `ProxyShoppingProductController.list` → `ProductServiceImpl#list` |
//!
//! # 多租户说明（`shopId` 从请求头取）
//!
//! Java `list` 方法里执行 `DTO.setShopId(HeaderUtils.getShopId())`，
//! 即 **平台账号（`x-admin-type: admin`）强制 `shopId = 0`**，
//! `shop` / `pos` 账号取 `x-shop-id`，其它情况为 `null`（不加店铺条件）。
//! 本实现按**同一规则解析请求头**（见 `crates/api/admin/src/routes/proxy_shopping_product.rs`
//! 的 `shop_id_from_headers`），不读取请求体里的 `shopId` —— 请求体同名字段会被
//! 请求头结果**覆盖**，与 Java 一致。
//!
//! # 与 Java 的差异（逐条，均为「原样照抄」或「明确未迁移」）
//!
//! 1. **`productDetail` 缺参行为**：Java 先无条件
//!    `productService.lambdaQuery().eq(Product::getProductTsn, productTsn)...one()`，
//!    再判断 `productSn`；两个参数都为空时 `product == null` 会抛 NPE（500）。
//!    本实现把这一形状保留为「先按 `productTsn` 查，再按 `productSn` 分流」，
//!    但**不制造 panic**：查不到时返回业务错误 400「商品不存在」。
//! 2. **`getProductAvailability` 的促销链路未迁移**（本片最大缺口）：
//!    Java `getProductSkuDetailWithCycle` 会调用
//!    `getProductsPromotion(...)` 拉取秒杀 / 限时折扣 / 拼团 / 满减活动，
//!    命中后 **覆盖 `price` / `stock` / `originPrice`**；还会读会员等级折扣
//!    （`memberBenefitService.resolveDiscount`）、积分兑换商品标记
//!    （`points_exchange`）、批发隐藏活动（`isWholesalePricedForUser`）。
//!    以上依赖**尚未迁移**的促销域 / 会员域 / 积分域 / 批发域，
//!    本实现**不编造**，`promotion` 恒为空数组、`price` 恒为「SKU 价（或商品价）
//!    + 附加规格价」、`originPrice` 恒等于 `price`（Java 在无活动价时
//!    `originPrice == price`，故无活动场景下两者一致）。
//!    ⚠️ **若商品正在参加秒杀/限时折扣/拼团，本接口的 `price` 与 `stock`
//!    会与 Java 不同**（Java 用活动价与活动库存）。
//! 3. **礼品卡（`product_type = 7/8`）库存已迁移**：Java 读
//!    `gift_card_groups.status == 1 ? min(availability_stock, product_stock) : 0`，
//!    并且当 `product.product_stock <= 0` 时直接置 0。本实现照抄（见
//!    [`ProxyShoppingProductService::stock_by_product_kind`]）。
//! 4. **卡密商品（`product_type = 3`）库存已迁移**：Java
//!    `updateStockByECard` 把 `productStock` 改写成
//!    `COUNT(e_card WHERE group_id = product.card_group_id AND is_use = 0)`。
//!    本实现照抄；注意该计数**不带店铺/时间条件**。
//! 5. **预约商品（`product_type = 5/6`）库存未迁移**：Java 在 O2O 开关打开时
//!    用「预约日历 / 预约库存表」重算库存（`calculateReservationStockForListDisplay`），
//!    依赖预约域，本实现不伪造，保持 `product_stock` / `sku_stock` 原值。
//! 6. **预售（`sale_mode = 2`）字段未迁移**：Java 计算
//!    `depositAmount` / `finalPaymentAmount`（`product_presale` 域），
//!    本实现恒为 `None`。**说明**：本域的列表接口（`/product/getProduct`）
//!    把 `saleMode` 强制为 0，因此前端在该流程中通常不会拿到预售商品；
//!    但 `getProductAvailability` 的 `id` 是前端自选的，仍可能命中预售商品。
//! 7. **O2O 门店覆盖/会员折扣/距离已实现**：
//!    - `shopId > 0` 时用 `store_product` / `store_product_sku` 覆盖价与库存；
//!    - `userId > 0` 时套会员等级折扣；
//!    - 传 `addressId` 时按用户地址与门店/自营店经纬度算 Haversine 距离。
//!    促销（秒杀/限时折扣/拼团）仍未迁移，见第 2 条。
//! 8. **权限未迁移**：Java 类级
//!    `@PreAuthorize("@pms.hasPermission('proxyShoppingManage')")`，
//!    Rust 侧权限矩阵尚未实现，`require_auth` 只校验登录态
//!    （与 seckill / product_manage/002/005/010/014/018 的已知缺口一致）。
//! 9. **`price` 的舍入**：Java 用 `BigDecimal.setScale(2, RoundingMode.HALF_UP)`，
//!    本实现用 `round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)`
//!    —— 金额恒为非负数，两者**逐值等价**（与 merchant::shop_client / wholesale::wholesale_client 同一做法）。
//! 10. **`attr_price` 为 NULL 的附加规格**：Java
//!     `list.map(getAttrPrice).reduce(ZERO, add)` 在遇到 `NULL` 时抛 NPE；
//!     本实现的仓储方法 `ProxyShoppingProductRepository::find_attribute_prices`
//!     直接**跳过**该行（按 0 计），不制造 panic。

use std::str::FromStr;
use std::sync::Arc;

use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::{Decimal, RoundingStrategy};
use nextrade_core::entities::{Product, ProductSku};
use nextrade_core::models::catalog::product_manage::{
    AdminProductListQuery, ClientProductDetailVO, ProductListItemVO, ProductSkuVO,
};
use nextrade_core::models::trade::proxy_shopping_product::{
    AvailabilitySkuVO, ProductAvailabilityVO, ProxyProductAvailabilityQuery,
    ProxyProductDetailQuery, SkuAvailabilityDataVO,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::trade::proxy_shopping_product::{price_map, sum_attr_price};
use nextrade_infrastructure::db::Repositories;

use crate::catalog::product_manage::ProductManageService;

/// Java `ProductConstants.PRODUCT_NOT_EXIST`（提示语逐字一致）。
pub const ERR_PRODUCT_NOT_EXIST: &str = "商品不存在";

/// 代客下单允许的商品类型，对应 Java
/// `ProxyShoppingProductController.ADMIN_PROXY_SHOPPING_PRODUCT_TYPES`：
/// 1 普通商品 / 2 虚拟商品 / 3 卡密商品 / 4 付费内容 / 7 实体礼品卡 /
/// 8 电子礼品卡（**不含** 5 预约上门、6 预约到店）。
pub const ADMIN_PROXY_SHOPPING_PRODUCT_TYPES: [i32; 6] = [1, 2, 3, 4, 7, 8];

/// `ProductType.PRODUCT_TYPE_CARD.getCode()`（卡密商品）。
pub const PRODUCT_TYPE_CARD: i32 = 3;
/// `ProductType.PRODUCT_TYPE_GIFT_CARD.getCode()`（实体礼品卡）。
pub const PRODUCT_TYPE_GIFT_CARD: i32 = 7;
/// `ProductType.PRODUCT_TYPE_GIFT_CARD_VIRTUAL.getCode()`（电子礼品卡）。
pub const PRODUCT_TYPE_GIFT_CARD_VIRTUAL: i32 = 8;

/// 只有单个启用 SKU 时自动选中该 SKU（Java
/// `getProductSkuDetailWithCycle` 的 `skuId == null || skuId == 0` 分支）。
///
/// `skus` 为库中该商品**含禁用**的全部规格（Java
/// `productSkuService.getProductSkusByPid(productId)` 不过滤 `is_enabled`）。
///
/// 返回 `Some(sku_id)` 的条件与 Java 完全一致：列表**非空且长度恰为 1**。
#[must_use]
pub fn resolve_single_sku_id(skus: &[ProductSkuVO]) -> Option<i32> {
    if skus.len() == 1 {
        Some(skus[0].sku_id)
    } else {
        None
    }
}

/// 归一化「提交的规格 ID」：`None` 与 `0` 等价（Java
/// `if (skuId == null || skuId == 0)`）。`<= 0` 的其它取值同样视为未选。
#[must_use]
pub fn normalize_requested_sku_id(sku_id: Option<i32>) -> Option<i32> {
    sku_id.filter(|v| *v > 0)
}

/// 解析 `extraAttrIds`（逗号分隔的 `attributes_id`）。
///
/// 对应 Java `StringUtils.str2IntList(extraAttrIds)`：非数字片段被忽略
/// （Java 侧 `str2IntList` 对无法解析的项同样跳过）。空白串 → 空集合。
#[must_use]
pub fn parse_extra_attr_ids(raw: Option<&str>) -> Vec<i32> {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Vec::new();
    };
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<i32>().ok())
        .collect()
}

/// 价格舍入到 2 位小数（Java `setScale(2, RoundingMode.HALF_UP)`）。
///
/// 金额恒为非负数，故 `RoundingStrategy::MidpointAwayFromZero` 与 Java
/// `HALF_UP` 完全等价（与 merchant::shop_client / wholesale::wholesale_client / time_discount 的既有做法一致）。
#[must_use]
pub fn round_price(v: Decimal) -> Decimal {
    v.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

/// Java `GpsDistanceUtil.getDistance`：Haversine，单位米；任一坐标为空返回 0。
#[must_use]
pub fn haversine_meters(
    lng1: Option<Decimal>,
    lat1: Option<Decimal>,
    lng2: Option<Decimal>,
    lat2: Option<Decimal>,
) -> Decimal {
    let (Some(lng1), Some(lat1), Some(lng2), Some(lat2)) = (lng1, lat1, lng2, lat2) else {
        return Decimal::ZERO;
    };
    let rad = |v: Decimal| v.to_f64().unwrap_or(0.0).to_radians();
    let (lng1, lat1, lng2, lat2) = (rad(lng1), rad(lat1), rad(lng2), rad(lat2));
    let a = lat1 - lat2;
    let b = lng1 - lng2;
    let s = 2.0
        * ((a / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (b / 2.0).sin().powi(2))
        .sqrt()
        .asin();
    Decimal::from_f64(6_378_137.0 * s).unwrap_or(Decimal::ZERO)
}


/// 代客下单商品管理服务。
pub struct ProxyShoppingProductService {
    repos: Arc<Repositories>,
    /// 复用 catalog::product_manage 已实现的商品列表分页 / 前台详情
    /// （`repo::product_manage` 与 `services::product_manage` 均为他人独占文件，不得修改；
    /// 其方法签名已满足本域需求，故不重复实现）。
    products: Arc<ProductManageService>,
}

impl ProxyShoppingProductService {
    pub fn new(repos: Arc<Repositories>, products: Arc<ProductManageService>) -> Self {
        Self { repos, products }
    }

    // ————————————————————— 商品详情 —————————————————————

    /// 商品详情（对应 Java `ProxyShoppingProductController.detail`）。
    ///
    /// Java 逻辑（`ProxyShoppingProductController.java:41-52`）：
    /// ```text
    /// Product product = lambdaQuery().eq(productTsn, productTsn).last("limit 1").one();
    /// if (productSn != null) { return clientDetail(productSn, shopId, nearestShopId); }
    /// return clientDetail(product.getProductSn(), shopId, nearestShopId);
    /// ```
    ///
    /// 因此：
    /// - `productTsn` 为空或查不到时，Java 的 `product` 为 `null`；若此时
    ///   `productSn` 也为空 → **NPE（500）**。本实现返回 400「商品不存在」。
    /// - `productSn` 非空时**不依赖** `productTsn` 的查询结果（Java 直接分流），
    ///   本实现同样只把 `productTsn` 查询当作前置步骤。
    ///
    /// ⚠️ **一处细微差异**（详见 `routes/proxy_shopping_product.rs` 模块头）：Java 第一步是
    /// 只按 `product_tsn` 精确匹配且**不过滤 `is_delete`**；本实现复用
    /// `repo::nextrade_core::models::catalog::product_manage::find_product_by_sn`（`product_sn = ? OR product_tsn = ?`
    /// 且 `is_delete = 0`）。见返回值的 `uncertain_points`。
    pub async fn product_detail(
        &self,
        q: &ProxyProductDetailQuery,
    ) -> Result<ClientProductDetailVO> {
        let tsn = q
            .product_tsn
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        // Java 无条件先按 productTsn 查一行（`getProductBySn` 的 tsn 分支语义）
        let by_tsn: Option<Product> = match tsn {
            Some(tsn) => self.repos.catalog.product_manage.find_product_by_sn(tsn).await?,
            None => None,
        };

        let sn = match q
            .product_sn
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            // Java `if (productSn != null)` 分支：直接用入参货号
            Some(sn) => sn.to_string(),
            // Java else 分支：用上一步查到的商品的 `productSn`
            // （`product == null` 时 Java NPE → 这里返回业务错误，见模块头第 1 条）
            None => by_tsn
                .map(|p| p.product_sn)
                .ok_or_else(|| Error::biz(400, ERR_PRODUCT_NOT_EXIST))?,
        };

        self.products
            .client_detail(&sn, q.shop_id, q.nearest_shop_id)
            .await
    }

    // ————————————————————— 商品可用数据 —————————————————————

    /// 商品可用数据（对应 Java
    /// `ProxyShoppingProductController.getProductAvailability`
    /// → `ProductServiceImpl#getProductSkuDetail(productId, skuId, 0, extraAttrIds, shopId, addressId, userId)`）。
    ///
    /// `excludeActivity` 由控制器**硬编码为 0**（即「允许带活动价」），
    /// 这正是模块头第 2 条缺口最明显的地方：Java 在该分支会去拉促销。
    /// 本实现跳过促销（`promotion` 恒空），其余链路逐条照抄：
    ///
    /// 1. `product` 不存在时：Java `getProductDetail` 返回**空** `ProductECardGroupDTO`
    ///    （不报错），`productPrice` 为 `null`；
    ///    - `extraAttrIds` 为空 → `productPrice.add(ZERO)` == `null` → 最终
    ///      `price = null`（JSON `null`，Java **不会**变成 0.00），`stock = null`；
    ///    - `extraAttrIds` 非空 → `null.add(attrPrice)` 抛 **NPE（500）**。
    ///    本实现照抄：前者下发 `price: null`，后者返回 400（不制造 panic）。
    /// 2. `skuId` 为空或 `0` 且该商品**恰好只有 1 个规格** → 自动选中该规格。
    /// 3. 命中规格时：`startProductPrice = price = sku_price`、
    ///    `stock = is_enabled == 0 ? 0 : sku_stock`；否则用商品价 / 商品库存。
    /// 4. `price += Σ(extraAttrIds 对应 product_attributes.attr_price)`。
    /// 5. 卡密商品（type 3）用未使用卡券数覆盖库存；礼品卡（type 7/8）用
    ///    礼品卡组可用库存覆盖（见 [`Self::stock_by_product_kind`]）。
    /// 6. `originPrice = price`（Java 无活动价时同值）。
    pub async fn get_product_availability(
        &self,
        q: &ProxyProductAvailabilityQuery,
    ) -> Result<ProductAvailabilityVO> {
        let Some(product_id) = q.product_id.filter(|v| *v > 0) else {
            // Java `@RequestParam("id") Integer productId` 必填；缺参时
            // Spring 直接 400。这里给出同语义的业务错误。
            return Err(Error::Validation("id 不能为空".into()));
        };

        let product = self.repos.catalog.product_manage.find_product(product_id).await?;
        let extra_ids = parse_extra_attr_ids(q.extra_attr_ids.as_deref());

        // Java：`productDetail` 为空 DTO → `productPrice` 为 null
        let Some(mut product) = product else {
            if !extra_ids.is_empty() {
                // Java `null.add(attrPrice)` → NPE（500）。不制造 panic，
                // 返回明确业务错误（见模块头第 1 条）。
                return Err(Error::biz(
                    400,
                    "商品不存在：无法计算附加规格价（Java 侧此处抛 NPE）",
                ));
            }
            return Ok(ProductAvailabilityVO {
                id: product_id,
                product_id,
                // Java 直接下发 `price = null`（不是 0.00）
                price: None,
                start_product_price: None,
                stock: None,
                market_price: None,
                origin_price: None,
                ..Default::default()
            });
        };

        // O2O：门店上下文优先用 store_product / store_product_sku 覆盖价与库存。
        let store_product = if let Some(store_shop_id) = q.shop_id.filter(|v| *v > 0) {
            self.repos
                .catalog
                .product_stock
                .find_store_product(product_id, store_shop_id)
                .await?
        } else {
            None
        };
        if let Some(store) = &store_product {
            if let Some(price) = store.product_price {
                product.product_price = price;
            }
            if let Some(stock) = store.product_stock {
                product.product_stock = stock;
            }
        }

        // Java `productSkuService.getProductSkusByPid(productId)`：含禁用规格
        let sku_rows = self.repos.catalog.product_manage.find_skus(product_id, true).await?;
        let all_skus: Vec<ProductSkuVO> = sku_rows.iter().map(to_availability_sku_vo).collect();

        // 解析实际使用的 skuId（含「仅 1 个规格自动选中」）
        let requested = normalize_requested_sku_id(q.sku_id);
        let resolved_sku_id = match requested {
            Some(v) => Some(v),
            None => resolve_single_sku_id(&all_skus),
        };
        let mut sku: Option<ProductSkuVO> = resolved_sku_id
            .and_then(|id| all_skus.iter().find(|s| s.sku_id == id).cloned());
        if let (Some(store), Some(s)) = (&store_product, sku.as_mut()) {
            if let Some(store_sku) = self
                .repos
                .catalog
                .product_stock
                .find_store_product_sku(store.store_product_id, s.sku_id)
                .await?
            {
                if let Some(price) = store_sku.sku_price {
                    s.sku_price = Some(price);
                }
                if let Some(stock) = store_sku.sku_stock {
                    s.sku_stock = stock;
                }
            }
        }

        // 库存与价格基线（Java：命中规格时用规格价 / 规格库存）
        let (mut stock, mut price, start_price, sku_data) = match &sku {
            Some(s) => {
                let p = s.sku_price.unwrap_or(Decimal::ZERO);
                (s.sku_stock, p, p, s.sku_data.clone())
            }
            None => (
                product.product_stock,
                product.product_price,
                product.product_price,
                None,
            ),
        };

        // 卡密 / 礼品卡库存覆盖（模块头第 3、4 条）
        stock = self.stock_by_product_kind(&product, stock).await?;

        // 会员等级折扣（Java `memberBenefitService.resolveDiscount`：
        // price * discount / 10，HALF_UP 保留 2 位）。积分兑换商品暂不迁移，
        // 因此这里始终按「非积分兑换商品」套折扣。
        if let Some(user_id) = q.user_id.filter(|v| *v > 0) {
            if let Some(discount) = self.member_discount(user_id).await? {
                if discount > Decimal::ZERO {
                    price = (price * discount / Decimal::from(10))
                        .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
                }
            }
        }

        // 附加规格价（Java `attrPrice`）
        let attr_price = if extra_ids.is_empty() {
            Decimal::ZERO
        } else {
            let rows = self
                .repos.trade.proxy_shopping_product
                .find_attribute_prices(&extra_ids)
                .await?;
            sum_attr_price(&price_map(&rows), &extra_ids)
        };
        price += attr_price;

        // 规格缩略图（Java `skuData.forEach(... getProductAttribute ...)`）
        let data = self
            .build_sku_data(&product, sku_data.as_ref())
            .await?;

        let distance = self.resolve_distance(q).await?;

        let final_price = round_price(price);
        Ok(ProductAvailabilityVO {
            id: product_id,
            data,
            // Java `setSku(productSkuDetail)`：直接下发完整 SKU 明细
            sku: sku.as_ref().map(to_availability_sku),
            // 未迁移：预售定金 / 尾款
            deposit_amount: None,
            final_payment_amount: None,
            // Java：无活动价时 `originPrice == productPrice`（含 attrPrice）
            origin_price: Some(final_price),
            market_price: Some(product.market_price),
            start_product_price: Some(start_price),
            price: Some(final_price),
            stock: Some(stock),
            // 未迁移：促销（秒杀 / 限时折扣 / 拼团）
            promotion: Vec::new(),
            product_id,
            distance,
            // 未迁移：礼品卡组 ID（Java 在该分支也没有 set，恒为 null）
            gift_card_groups_id: None,
        })
    }

    /// 会员等级折扣（Java `MemberBenefitService#resolveDiscount`）。
    async fn member_discount(&self, user_id: i32) -> Result<Option<Decimal>> {
        let Some(user) = self.repos.member.user_manage.find_user(user_id).await? else {
            return Ok(None);
        };
        let Some(rank) = self
            .repos
            .member
            .user_center
            .find_user_rank(user.rank_id)
            .await?
        else {
            return Ok(None);
        };
        Ok(Some(rank.discount))
    }

    /// O2O 距离（Java `GpsDistanceUtil.getDistance`，单位米）。
    ///
    /// 只有传了 `addressId` 才计算：
    /// - `shopId > 0`：用户地址经纬度 ↔ 门店经纬度；
    /// - `shopId` 为空：Java 用自营门店配置经纬度；本实现缺少该配置，
    ///   在地址存在但无门店时返回 0（与 Java 任一坐标为空时返回 0 一致）。
    async fn resolve_distance(
        &self,
        q: &ProxyProductAvailabilityQuery,
    ) -> Result<Option<Decimal>> {
        let Some(address_id) = q.address_id.filter(|v| *v > 0) else {
            return Ok(None);
        };
        let Some(address) = self
            .repos
            .member
            .user_address_client
            .find_by_id(address_id)
            .await?
        else {
            return Ok(None);
        };
        let Some(shop_id) = q.shop_id.filter(|v| *v > 0) else {
            // Java 无门店时用自营门店配置经纬度；读不到配置则按 0 处理。
            let pairs = self
                .repos
                .system
                .config_basic
                .find_by_codes(&["storeSelfOperatedLongitude", "storeSelfOperatedLatitude"])
                .await?;
            let map: std::collections::HashMap<String, String> = pairs.into_iter().collect();
            let lng = map
                .get("storeSelfOperatedLongitude")
                .and_then(|s| Decimal::from_str(s.trim()).ok());
            let lat = map
                .get("storeSelfOperatedLatitude")
                .and_then(|s| Decimal::from_str(s.trim()).ok());
            return Ok(Some(haversine_meters(address.longitude, address.latitude, lng, lat)));
        };
        let Some(shop) = self.repos.identity.admin_user.find_shop(shop_id).await? else {
            return Ok(Some(Decimal::ZERO));
        };
        Ok(Some(haversine_meters(
            address.longitude,
            address.latitude,
            shop.shop_longitude,
            shop.shop_latitude,
        )))
    }

    /// 卡密 / 礼品卡的库存覆盖（对应 Java `updateStockByECard`
    /// + `getProductSkuDetailWithCycle` 的礼品卡分支）。
    ///
    /// 覆盖顺序与 Java 一致：先按「命中规格的库存」得到 `stock`，
    /// 再按商品类型覆写：
    ///
    /// - `product_type == 3`（卡密）：`COUNT(e_card WHERE group_id = card_group_id AND is_use = 0)`
    /// - `product_type == 7 / 8`（礼品卡）：
    ///   `gift_card_groups.status == 1 ? (product.product_stock > 0 ? min(availability_stock, product.product_stock) : 0) : 0`
    ///   （`availability_stock` 为 `NULL` 时 Java 会 NPE；这里按 0 处理）
    /// - 其它类型：原样返回
    async fn stock_by_product_kind(&self, product: &Product, stock: i32) -> Result<i32> {
        match product.product_type {
            Some(PRODUCT_TYPE_CARD) => {
                let count = self
                    .repos.trade.proxy_shopping_product
                    .count_unused_ecards(product.card_group_id)
                    .await?;
                Ok(i32::try_from(count).unwrap_or(i32::MAX))
            }
            Some(PRODUCT_TYPE_GIFT_CARD) | Some(PRODUCT_TYPE_GIFT_CARD_VIRTUAL) => {
                let Some(group_id) = product.gift_card_groups_id else {
                    // Java `giftCardGroupsMapper.selectById(null)` 返回 null → 库存 0
                    return Ok(0);
                };
                let Some((status, availability_stock)) = self
                    .repos.trade.proxy_shopping_product
                    .find_gift_card_group_stock(group_id)
                    .await?
                else {
                    return Ok(0);
                };
                if status != 1 {
                    return Ok(0);
                }
                let availability = availability_stock.unwrap_or(0);
                if product.product_stock > 0 {
                    Ok(availability.min(product.product_stock))
                } else {
                    Ok(0)
                }
            }
            _ => Ok(stock),
        }
    }

    /// 组装 `skuData`（Java `productSkuDetail.getSkuData()`）。
    ///
    /// 库中 `product_sku.sku_data` 是 JSON 文本（`[{"name":..,"value":..}]`），
    /// 解析失败时退回空数组（Java `JSON.parseArray` 失败同样得到空列表）。
    ///
    /// 随后逐项调用 `getProductAttribute(productId, name, value)` 填
    /// `attrPicThumb`（属性缩略图，回退商品主图 `pic_thumb`）。
    async fn build_sku_data(
        &self,
        product: &Product,
        raw: Option<&serde_json::Value>,
    ) -> Result<Vec<SkuAvailabilityDataVO>> {
        let Some(raw) = raw else {
            return Ok(Vec::new());
        };
        let mut items: Vec<SkuAvailabilityDataVO> = match raw {
            serde_json::Value::Array(arr) => arr
                .iter()
                .map(|v| SkuAvailabilityDataVO {
                    name: v
                        .get("name")
                        .and_then(|x| x.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    value: v
                        .get("value")
                        .and_then(|x| x.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    attr_pic_thumb: None,
                })
                .collect(),
            _ => Vec::new(),
        };
        if items.is_empty() {
            return Ok(items);
        }

        let pairs: Vec<(String, String)> = items
            .iter()
            .map(|i| (i.name.clone(), i.value.clone()))
            .collect();
        let thumbs = self
            .repos.trade.proxy_shopping_product
            .find_sku_attr_pic_thumbs(product.product_id, &pairs)
            .await?;
        for (item, thumb) in items.iter_mut().zip(thumbs) {
            // Java：属性缩略图为空时回退商品主图 `pic_thumb`
            item.attr_pic_thumb = Some(thumb.unwrap_or_else(|| product.pic_thumb.clone()));
        }
        Ok(items)
    }

    // ————————————————————— 商品列表 —————————————————————

    /// 代客下单商品分页（对应 Java `ProxyShoppingProductController.list`）。
    ///
    /// Java（`ProxyShoppingProductController.java:67-74`）在调用
    /// `productService.list(DTO)` 之前**覆盖**了五个字段：
    ///
    /// | 字段 | 强制值 | 本实现 |
    /// |------|--------|--------|
    /// | `productTypes` | 1/2/3/4/7/8 | 覆盖 |
    /// | `isDelete` | 0 | 覆盖 |
    /// | `productStatus` | 1 | 覆盖 |
    /// | `shopId` | `HeaderUtils.getShopId()` | 覆盖（调用方从请求头解析） |
    /// | `saleMode` | 0（排除预售） | 覆盖 |
    ///
    /// 其余入参（`keyword` / `categoryId` / `brandId` / 价格区间 / 分页 / 排序 …）
    /// 原样透传给 catalog::product_manage 的 `admin_list`（其查询条件与提示语逐条对齐
    /// `ProductServiceImpl#buildQueryWrapper`）。
    pub async fn product_list(
        &self,
        q: &AdminProductListQuery,
        header_shop_id: Option<i32>,
    ) -> Result<PageResult<ProductListItemVO>> {
        let mut dto = q.clone();
        dto.product_types = Some(ADMIN_PROXY_SHOPPING_PRODUCT_TYPES.to_vec());
        dto.is_delete = Some(0);
        dto.product_status = Some(1);
        dto.shop_id = header_shop_id;
        dto.sale_mode = Some(0);
        dto.only_show_me = Some(false);
        // Java `Page<>(listDTO.getPage(), listDTO.getSize())`：MyBatis-Plus 的
        // `current` 从 1 起，与 Rust 侧 `PageQuery` 语义一致，无需换算。
        self.products.admin_list(&dto).await
    }
}

/// `ProductSku` → 可用数据里的 SKU VO（字段与 Java `ProductSkuDTO` 对齐）。
fn to_availability_sku_vo(s: &nextrade_core::entities::ProductSku) -> ProductSkuVO {
    ProductSkuVO {
        sku_id: s.sku_id,
        product_id: s.product_id,
        sku_value: s.sku_value.clone(),
        sku_data: s
            .sku_data
            .as_deref()
            .and_then(|raw| serde_json::from_str(raw).ok()),
        sku_sn: s.sku_sn.clone(),
        sku_stock: s.sku_stock.unwrap_or(0),
        sku_tsn: s.sku_tsn.clone(),
        sku_weight: s.sku_weight,
        sku_volume: s.sku_volume,
        sku_price: s.sku_price,
        is_enabled: s.is_enabled,
        market_price: s.market_price,
        cost_price: s.cost_price,
        delivery_service_fee_type: s.delivery_service_fee_type,
        delivery_service_fee_value: s.delivery_service_fee_value,
        server_service_fee_type: s.server_service_fee_type,
        server_service_fee_value: s.server_service_fee_value,
        vendor_product_sku_id: s.vendor_product_sku_id,
    }
}

/// `ProductSkuVO` → 响应体 SKU（字段名与 Java `ProductSkuDTO` 逐一对齐）。
///
/// 单规格商品的 `sku_data` 在库中常为空串，此时 Java `JSON.parseArray("")`
/// 得到空列表，本实现同样得到 `None`（JSON `null`，前端按空处理）。
#[must_use]
pub fn to_availability_sku(s: &ProductSkuVO) -> AvailabilitySkuVO {
    AvailabilitySkuVO {
        sku_id: s.sku_id,
        product_id: s.product_id,
        sku_value: s.sku_value.clone(),
        sku_data: s.sku_data.clone(),
        sku_sn: s.sku_sn.clone(),
        sku_stock: s.sku_stock,
        sku_tsn: s.sku_tsn.clone(),
        sku_price: s.sku_price,
        is_enabled: s.is_enabled,
        market_price: s.market_price,
        cost_price: s.cost_price,
        delivery_service_fee_type: s.delivery_service_fee_type,
        delivery_service_fee_value: s.delivery_service_fee_value,
        server_service_fee_type: s.server_service_fee_type,
        server_service_fee_value: s.server_service_fee_value,
        vendor_product_sku_id: s.vendor_product_sku_id,
    }
}

// 让 `ProductAvailabilityVO.sku` 直接携带 Java `ProductSkuDTO` 的字段集。
// 注意：不能用 `impl From<&ProductSkuVO> for AvailabilitySkuVO`——两个类型都定义在
// `nextrade-core`，在 services 侧实现会触发 E0117（孤儿规则），统一走
// [`to_availability_sku`] 函数。

/// 便于服务层把 `ProductSku` 实体列表整体转成响应 VO（保留顺序与条数）。
#[must_use]
pub fn to_availability_sku_list(rows: &[ProductSku]) -> Vec<AvailabilitySkuVO> {
    rows.iter()
        .map(|s| to_availability_sku(&to_availability_sku_vo(s)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn sku(id: i32, stock: i32, price: &str, enabled: i32) -> ProductSkuVO {
        ProductSkuVO {
            sku_id: id,
            product_id: 1,
            sku_value: format!("规格{id}"),
            sku_stock: stock,
            sku_price: Some(Decimal::from_str(price).unwrap()),
            is_enabled: enabled,
            ..Default::default()
        }
    }

    /// 带库存 / 启用状态的可变 SKU 实体（供实体 → VO 转换测试）。
    fn sku_entity(id: i32, stock: Option<i32>, enabled: i32) -> ProductSku {
        ProductSku {
            sku_id: id,
            product_id: 2,
            sku_value: format!("规格{id}"),
            sku_stock: stock,
            is_enabled: enabled,
            ..Default::default()
        }
    }

    // ————————————————————— 商品类型白名单 —————————————————————

    #[test]
    fn proxy_shopping_product_types_match_java() {
        // Java `ADMIN_PROXY_SHOPPING_PRODUCT_TYPES`：
        // NORMAL(1), VIRTUAL(2), CARD(3), PAID(4), GIFT_CARD(7), GIFT_CARD_VIRTUAL(8)
        assert_eq!(ADMIN_PROXY_SHOPPING_PRODUCT_TYPES, [1, 2, 3, 4, 7, 8]);
        // 预约商品（5 上门 / 6 到店）必须被排除
        assert!(!ADMIN_PROXY_SHOPPING_PRODUCT_TYPES.contains(&5));
        assert!(!ADMIN_PROXY_SHOPPING_PRODUCT_TYPES.contains(&6));
    }

    // ————————————————————— skuId 归一化 / 自动选中 —————————————————————

    #[test]
    fn normalize_requested_sku_id_treats_zero_and_negative_as_absent() {
        assert_eq!(normalize_requested_sku_id(None), None);
        assert_eq!(normalize_requested_sku_id(Some(0)), None); // Java `skuId == 0`
        assert_eq!(normalize_requested_sku_id(Some(-3)), None);
        assert_eq!(normalize_requested_sku_id(Some(11)), Some(11));
    }

    #[test]
    fn single_sku_is_auto_selected_only_when_exactly_one() {
        // Java：`CollUtil.isNotEmpty(list) && list.size() == 1`
        assert_eq!(resolve_single_sku_id(&[]), None);
        assert_eq!(resolve_single_sku_id(&[sku(7, 3, "1.00", 1)]), Some(7));
        assert_eq!(
            resolve_single_sku_id(&[sku(7, 3, "1.00", 1), sku(8, 0, "2.00", 0)]),
            None
        );
        // 只有一个但**已禁用**的规格也会被选中（Java 不过滤 is_enabled，
        // 之后 `skuEnabled == false` 会把库存压成 0）
        assert_eq!(resolve_single_sku_id(&[sku(9, 5, "1.00", 0)]), Some(9));
    }

    // ————————————————————— extraAttrIds 解析与累加 —————————————————————

    #[test]
    fn parse_extra_attr_ids_matches_java_str2intlist() {
        assert_eq!(parse_extra_attr_ids(None), Vec::<i32>::new());
        assert_eq!(parse_extra_attr_ids(Some("")), Vec::<i32>::new());
        assert_eq!(parse_extra_attr_ids(Some("   ")), Vec::<i32>::new());
        assert_eq!(parse_extra_attr_ids(Some("1,2,3")), vec![1, 2, 3]);
        assert_eq!(parse_extra_attr_ids(Some(" 1 , 2 ")), vec![1, 2]);
        // 非数字片段被忽略（Java `str2IntList` 同样跳过无法解析的项）
        assert_eq!(parse_extra_attr_ids(Some("1,x,3")), vec![1, 3]);
        // 逗号分隔串里的空段忽略
        assert_eq!(parse_extra_attr_ids(Some("1,,3")), vec![1, 3]);
    }

    #[test]
    fn attr_price_is_added_to_final_price() {
        // Java：`productPrice = productPrice.add(attrPrice)`
        let base = Decimal::from_str("10.00").unwrap();
        let ids = parse_extra_attr_ids(Some("5,6"));
        let prices = price_map(&[(5, Decimal::from_str("1.25").unwrap())]);
        // 6 未命中 → 只加 1.25
        assert_eq!(base + sum_attr_price(&prices, &ids), Decimal::from_str("11.25").unwrap());
    }

    // ————————————————————— 舍入 —————————————————————

    #[test]
    fn round_price_matches_java_half_up() {
        let d = |s: &str| Decimal::from_str(s).unwrap();
        // 常见四舍五入场景
        assert_eq!(round_price(d("1.0051")), d("1.01"));
        assert_eq!(round_price(d("1.0049")), d("1.00"));
        assert_eq!(round_price(d("10")), d("10.00"));
        assert_eq!(round_price(d("2.345")), d("2.35"));
        // Java `RoundingMode.HALF_UP`：midpoint 一律进位（1.005 → 1.01）
        assert_eq!(round_price(d("1.005")), d("1.01"));
        assert_eq!(round_price(d("1.015")), d("1.02"));
        assert_eq!(round_price(Decimal::ZERO), d("0.00"));
    }

    // ————————————————————— 列表强制条件 —————————————————————

    #[test]
    fn list_forces_java_overrides_on_a_clone() {
        // 直接验证「覆盖而非修改入参」的语义（入参来自请求体，不应被写回）。
        let q = AdminProductListQuery {
            keyword: Some("手机".into()),
            product_types: Some(vec![5, 6]),
            is_delete: Some(1),
            product_status: Some(0),
            shop_id: Some(99),
            sale_mode: Some(2),
            ..Default::default()
        };
        let mut dto = q.clone();
        dto.product_types = Some(ADMIN_PROXY_SHOPPING_PRODUCT_TYPES.to_vec());
        dto.is_delete = Some(0);
        dto.product_status = Some(1);
        dto.shop_id = Some(0);
        dto.sale_mode = Some(0);

        // 原入参保持不变
        assert_eq!(q.product_types, Some(vec![5, 6]));
        assert_eq!(q.is_delete, Some(1));
        assert_eq!(q.shop_id, Some(99));
        assert_eq!(q.sale_mode, Some(2));
        // 透传字段保持原值
        assert_eq!(dto.keyword.as_deref(), Some("手机"));
        // 覆盖字段生效
        assert_eq!(dto.product_types, Some(vec![1, 2, 3, 4, 7, 8]));
        assert_eq!(dto.is_delete, Some(0));
        assert_eq!(dto.product_status, Some(1));
        assert_eq!(dto.shop_id, Some(0));
        assert_eq!(dto.sale_mode, Some(0));
    }

    // ————————————————————— SKU 字段映射 —————————————————————

    #[test]
    fn sku_vo_carries_stock_and_enabled_flag() {
        let s = sku(3, 0, "9.90", 0);
        let vo = to_availability_sku(&s);
        assert_eq!(vo.sku_id, 3);
        assert_eq!(vo.sku_stock, 0);
        assert_eq!(vo.is_enabled, 0);
        assert_eq!(vo.sku_price, Some(Decimal::from_str("9.90").unwrap()));
    }

    #[test]
    fn sku_vo_list_conversion_keeps_order_and_count() {
        let rows = vec![
            sku_entity(1, Some(4), 1),
            sku_entity(2, None, 0),
        ];
        let out = to_availability_sku_list(&rows);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].sku_id, 1);
        assert_eq!(out[0].sku_stock, 4);
        assert_eq!(out[1].sku_id, 2);
        // `sku_stock` 为 NULL → 0（Java `Integer` 为 null 时前端按 0 处理）
        assert_eq!(out[1].sku_stock, 0);
        assert_eq!(out[1].is_enabled, 0);
    }

    #[test]
    fn haversine_matches_java_gps_distance() {
        let d = |s: &str| Decimal::from_str(s).unwrap();
        assert_eq!(
            haversine_meters(
                Some(d("115.857963")),
                Some(d("28.712094")),
                Some(d("115.857963")),
                Some(d("28.712094"))
            ),
            Decimal::ZERO
        );
        let one_meter = haversine_meters(
            Some(d("115.857963")),
            Some(d("28.712094")),
            Some(d("115.857972")),
            Some(d("28.712094")),
        );
        assert!(one_meter > Decimal::ZERO && one_meter < Decimal::from(2));
        assert_eq!(
            haversine_meters(None, Some(d("1")), Some(d("1")), Some(d("1"))),
            Decimal::ZERO
        );
    }
}