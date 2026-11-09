//! 代客下单订单结算域服务（trade::proxy_shopping_order_check）。
//!
//! 对应原 Java `com.nextrade.adminapi.controller.proxyshopping.ProxyShoppingOrderCheckController`
//! （`@RequestMapping("/adminapi/proxyShopping/order/check")`，
//! `ProxyShoppingOrderCheckController.java` 共 222 行）。
//!
//! | 本服务方法 | 原 Java |
//! |-----------|---------|
//! | [`ProxyShoppingOrderCheckService::available_payment_types`] | 控制器 `getAvailablePaymentType` → `OrderCheckServiceImpl#getAvailablePaymentType`（`:2990-3036`） |
//! | [`ProxyShoppingOrderCheckService::pickup_list`] | 控制器 `pickupList` → `ShopServiceImpl#pickupList`（`:1620-1641`） |
//! | [`ProxyShoppingOrderCheckService::check_index`] | 控制器 `index`（`:73-88`）→ `getOrderCheckData` |
//! | [`ProxyShoppingOrderCheckService::check_update`] | 控制器 `update`（`:111-126`）→ `getOrderCheckData` |
//! | [`ProxyShoppingOrderCheckService::check_update_coupon`] | 控制器 `updateCoupon`（`:128-138`）→ `getOrderCheckData` |
//! | [`ProxyShoppingOrderCheckService::change_price`] | 控制器 `changePrice`（`:140-152`）→ `getOrderCheckData` |
//! | [`ProxyShoppingOrderCheckService::change_price_show`] | 控制器 `changePriceShow`（`:154-171`）→ `getOrderCheckData` |
//! | [`ProxyShoppingOrderCheckService::submit`] | 控制器 `submit`（`:173-195`）→ `OrderCheckService#submit` |
//!
//! # 迁移状态（诚实声明，逐条）
//!
//! ## 1. 已完整迁移：`getAvailablePaymentType`
//!
//! `OrderCheckServiceImpl#getAvailablePaymentType` 只依赖：
//! `configService.getConfigVal`（已迁移，读 `config` 表）+
//! `paymentService.getAvailablePayment("")`（**只用来判空**，见下）。
//!
//! `paymentService.getAvailablePayment("")` 走的是
//! `PaymentServiceImpl#getAvailablePayment`（`PaymentServiceImpl.java:47-154`）：
//! 它逐个读 `useWechat` / `useAlipay` / `usePaypal` / `useApplePay` /
//! `useLakala` / `useYabanpay*` / `useYunpay` / `useHuifu` / `useYeepay` 开关，
//! 全部是「读配置 → `== 1`」的机械判断（外加 `isApplePayConfigComplete()`
//! 的 8 个配置项齐备判断）。本实现按**同一批配置键**逐条复刻，见
//! [`online_payment_available`]。
//!
//! ⚠️ 该接口**只判断列表是否为空**，不返回渠道列表 —— 因此本实现无需
//! 迁移渠道枚举与 `paymentOrder` 排序（排序不改变「是否为空」）。
//!
//! ## 2. 已完整迁移：`pickupList`
//!
//! 单表 `shop`（`shop_type = 3`，按距离升序）分页 + `region` 表取名，
//! 全部字段均在 schema 中真实存在。
//!
//! ## 3. 结算页数据：**已实现**
//!
//! `index` / `update` / `updateCoupon` / `changePrice` / `changePriceShow`
//! 完成控制器校验后委托 `trade::check::CheckService` 的
//! `order_check_index/update/update_coupon`，由 `build_check` 装配：
//! 购物车按店聚合、地址/配送方式、优惠券、积分/余额、运费、金额汇总等。
//! 与 Java 促销引擎的差异记录在 `trade::check` 模块头。
//!
//! ## 4. `OrderCheckService#submit`：**已实现**
//!
//! 控制器校验（`flowType=15`、非自提必须传地址、`posUserId`、`isCartList`、
//! `orderSource`、`cashAmount`、`shopId`、`fillUsePointAndBalance`）后，
//! 复用 `trade::order::OrderService::create` 完成下单，并回写
//! `order_source/pos_user_id`、改价差额和管理操作日志。
//! 拆单/库存/优惠券/积分余额/支付单等由既有下单服务负责。
//!
//! # 与 Java 的其它差异
//!
//! 1. **权限未迁移**：Java 类级
//!    `@PreAuthorize("@pms.hasPermission('proxyShoppingManage')")`，
//!    Rust 侧权限矩阵尚未实现，`require_auth` 只校验登录态 ——
//!    任何已登录管理员都能调用本域全部接口（与 seckill / order_admin / 003 /
//!    005 / 014 / 018 / 026 的已知缺口一致）。
//! 2. **`getShopId()` 的 `null` 归一化**：Java `HeaderUtils.getShopId()` 在
//!    `x-admin-type` 非 `admin`/`shop`/`pos` 时返回 `null`，而
//!    `OrderCheckParam.shopId` 是包装类型；本实现与 product_stock 的既有约定一致，
//!    统一归一为 `0`（平台视角）。
//! 3. **`Integer.parseInt` 抛错语义**：Java `ConfigServiceImpl#getConfigVal`
//!    返回字符串，调用方 `Integer.parseInt` 在脏数据（非数字）时抛
//!    `NumberFormatException` → 500。本实现的 [`parse_int_config`] 把脏数据
//!    当作 `0`（`SettingsEnum` 的默认值语义），**不抛错**。
//!    `x-shop-id` 的解析同理（见路由层 `shop_id_from_headers`）。
//! 4. **`ConfigService` 未迁移**：本项目没有共享的 `ConfigService`，各切片
//!    各自读 `config` 表（user_center/009/013/014/015/016/019/022 同一做法）。
//!    本实现的 `getConfigVal` 语义 = `biz_val` 缺失时返回 `""`。
//! 5. **`getConfigByCode(USE_OFFLINE)` 缺失时 Java 抛 `CONFIG_NOT_EXIST`**，
//!    而本实现按 `getConfigVal` 语义返回 `""`（线下支付 `isShow = false`）。
//!    已在 `uncertain_points` 标注。
//! 6. **服务层不依赖 `axum`**：`nextrade-services` crate 没有 `axum` 依赖，故
//!    请求头不在服务层解析；路由层把 `x-admin-type` / `x-shop-id` 解析成
//!    [`RequestContext`] 后传入（与 product_stock `StockOperator`、stock_warning
//!    `resolve_shop_id` 的既有做法一致）。

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_core::models::trade::proxy_shopping_order_check::{
    AvailablePaymentTypeQuery, CartChangePrice, OrderCheckParam, PaymentTypeVO, PickupListVO,
    PickupPageQuery, ShippingType, CART_TYPE_ADMIN_SHOPPING, FLOW_TYPE_GIFT_CARD,
    ORDER_DELIVERY_TYPE_PICKUP, PAYMENT_TYPE_OFFLINE, PAYMENT_TYPE_ONLINE,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::trade::proxy_shopping_order_check::{PickupQuery, ProxyShoppingOrderCheckRepository};
use nextrade_infrastructure::db::Repositories;

/// `CartTypeEnum.TYPE_ADMIN_SHOPPING`（Java `CartTypeEnum.java:39`）。
pub const FLOW_TYPE_ADMIN_SHOPPING: i32 = CART_TYPE_ADMIN_SHOPPING;

/// 代客下单订单来源标识
/// （Java `ProxyShoppingOrderCheckController.ORDER_SOURCE_PROXY_SHOPPING`，`:55`）。
pub const ORDER_SOURCE_PROXY_SHOPPING: &str = "代客下单";

/// 控制器 `validateUserIdNotEmpty` 的错误文案（Java `:218`，勿改）。
pub const MSG_SELECT_USER: &str = "请选择用户";

/// 控制器 `validateChangePriceValue` 的错误文案（Java `:205`，勿改）。
pub const MSG_REDUCE_GT_PAYABLE: &str = "减价金额不能大于应付金额";

/// 控制器 `submit` 的收货地址校验文案（Java `:182`，勿改）。
pub const MSG_SELECT_ADDRESS: &str = "请选择收货地址";

/// Java `ShopServiceImpl#pickupList` 的门店不存在文案（Java `:1623`，勿改）。
pub const MSG_SHOP_NOT_FOUND: &str = "没有此门店";

/// Java `GlobalException(String)` 的默认 code（`HttpStatus.SERVICE_DATA_ERROR`）。
pub const SERVICE_DATA_ERROR: i32 = 1001;

/// Java 在 `submit` 里写入的 `cashAmount`/`cashChangeAmount`（Java `:189-190`）。
pub const CASH_AMOUNT_ZERO: Decimal = Decimal::ZERO;

// ————————————————————— config 键（Java `SettingsEnum`） —————————————————————

/// `SettingsEnum.USE_OFFLINE`（`"useOffline"`）。
pub const CFG_USE_OFFLINE: &str = "useOffline";
/// `SettingsEnum.USE_SURPLUS`（`"useSurplus"`）。
pub const CFG_USE_SURPLUS: &str = "useSurplus";
/// `SettingsEnum.USE_COUPON`（`"useCoupon"`）。
pub const CFG_USE_COUPON: &str = "useCoupon";
/// `SettingsEnum.ADMIN_SHOPPING_USE_BALANCE`（`"adminShoppingUseBalance"`）。
pub const CFG_ADMIN_SHOPPING_USE_BALANCE: &str = "adminShoppingUseBalance";
/// `SettingsEnum.ADMIN_SHOPPING_USE_POINT`（`"adminShoppingUsePoint"`）。
pub const CFG_ADMIN_SHOPPING_USE_POINT: &str = "adminShoppingUsePoint";
/// `SettingsEnum.STORE_SHOW_OTHER_CITY_STORE`（`"storeShowOtherCityStore"`）。
pub const CFG_STORE_SHOW_OTHER_CITY_STORE: &str = "storeShowOtherCityStore";

/// `SettingsEnum.USE_WECHAT`。
pub const CFG_USE_WECHAT: &str = "useWechat";
/// `SettingsEnum.USE_ALIPAY`。
pub const CFG_USE_ALIPAY: &str = "useAlipay";
/// `SettingsEnum.USE_PAYPAL`。
pub const CFG_USE_PAYPAL: &str = "usePaypal";
/// `SettingsEnum.USE_APPLE_PAY`。
pub const CFG_USE_APPLE_PAY: &str = "useApplePay";
/// `SettingsEnum.USE_LAKALA`。
pub const CFG_USE_LAKALA: &str = "useLakala";
/// `SettingsEnum.USE_YABANPAY`。
pub const CFG_USE_YABANPAY: &str = "useYabanpay";
/// `SettingsEnum.USE_YABANPAY_WECHAT`。
pub const CFG_USE_YABANPAY_WECHAT: &str = "useYabanpayWechat";
/// `SettingsEnum.USE_YABANPAY_ALIPAY`。
pub const CFG_USE_YABANPAY_ALIPAY: &str = "useYabanpayAlipay";
/// `SettingsEnum.USE_YUNPAY`。
pub const CFG_USE_YUNPAY: &str = "useYunpay";
/// `SettingsEnum.USE_HUIFU`。
pub const CFG_USE_HUIFU: &str = "useHuifu";
/// `SettingsEnum.USE_YEEPAY`。
pub const CFG_USE_YEEPAY: &str = "useYeepay";

/// `getAvailablePaymentType` 需要读取的全部配置键（一次批量查询）。
///
/// 顺序 = Java 读取顺序，便于人工对照。
pub const PAYMENT_CONFIG_KEYS: [&str; 18] = [
    CFG_USE_OFFLINE,
    CFG_USE_SURPLUS,
    CFG_USE_WECHAT,
    CFG_USE_ALIPAY,
    CFG_USE_PAYPAL,
    CFG_USE_APPLE_PAY,
    CFG_USE_LAKALA,
    CFG_USE_YABANPAY,
    CFG_USE_YABANPAY_WECHAT,
    CFG_USE_YABANPAY_ALIPAY,
    CFG_USE_YUNPAY,
    CFG_USE_HUIFU,
    CFG_USE_YEEPAY,
    // Apple Pay 配置完整性校验（`isApplePayConfigComplete`，`PaymentServiceImpl.java:205-218`）
    "applePayAcquirerProvider",
    "applePayMerchantId",
    "applePayDisplayName",
    "applePayMerchantIdentityCert",
    "applePayMerchantIdentityKey",
];

/// Apple Pay 完整性校验余下的配置键（处理证书私钥 + 收单三件套）。
pub const APPLE_PAY_EXTRA_KEYS: [&str; 4] = [
    "applePayProcessingPrivateKey",
    "applePayAcquirerGateway",
    "applePayAcquirerMerchantNo",
    "applePayAcquirerKey",
];

/// `applePayAcquirerProvider` 的默认值（Java `StrUtil.blankToDefault(..., "generic")`）。
pub const APPLE_PAY_PROVIDER_DEFAULT: &str = "generic";

// ————————————————————— 请求上下文（替代 Java HeaderUtils） —————————————————————

/// 由路由层解析好的请求上下文（替代 Java `HeaderUtils`）。
///
/// **为什么不用 `axum::http::HeaderMap`**：`nextrade-services` crate 并不依赖
/// `axum`（见 `crates/services/Cargo.toml`），且服务层与 Web 框架解耦是本项目
/// 既有约定 —— product_stock 的 `StockOperator`、stock_warning 的
/// `resolve_shop_id(requested, admin_type, header_shop_id)` 都是同一做法。
/// 请求头的提取留在 `api/admin/src/routes/proxy_shopping_order_check.rs`。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestContext {
    /// Java `HeaderUtils.getShopId()` 的结果：`admin` → `Some(0)`；
    /// `shop`/`pos` → `x-shop-id`（缺失/非数字 → `None`）；其他/缺失 → `None`。
    pub shop_id: Option<i32>,
    /// Java `HeaderUtils.getAdminType()` 的原始 `x-admin-type` 取值
    /// （本域只需区分 `"admin"` 与非 `"admin"`）。
    pub admin_type: Option<String>,
}

impl RequestContext {
    /// 构造。
    #[must_use]
    pub fn new(shop_id: Option<i32>, admin_type: Option<String>) -> Self {
        Self {
            shop_id,
            admin_type,
        }
    }

    /// `OrderCheckParam.shopId` 的取值：Java 的 `null` 在本域归一为 `0`
    /// （见模块头第 2 条）。
    #[must_use]
    pub fn shop_id_or_zero(&self) -> i32 {
        self.shop_id.unwrap_or(0)
    }

    /// `admin_type` 的 `&str` 视图（供 [`fill_use_point_and_balance`] 使用）。
    #[must_use]
    pub fn admin_type(&self) -> Option<&str> {
        self.admin_type.as_deref()
    }
}

// ————————————————————— 纯逻辑：控制器校验 —————————————————————

/// 复刻 Java `validateUserIdNotEmpty(param)`（`:215-221`）。
///
/// Java：`userId == null || userId <= 0` → `GlobalException("请选择用户")`。
/// `index` / `update` / `updateCoupon` / `changePrice` / `changePriceShow`
/// 五条接口都调用它；`submit` **不调用**（Java `:175-195` 没有该校验）。
pub fn validate_user_id_not_empty(param: &OrderCheckParam) -> Result<i32> {
    match param.user_id {
        Some(id) if id > 0 => Ok(id),
        _ => Err(Error::biz(SERVICE_DATA_ERROR, MSG_SELECT_USER)),
    }
}

/// 复刻 Java `validateChangePriceValue(param)`（`:200-207`）。
///
/// Java：
/// ```text
/// if (3 == param.getTotalChangePriceType()
///     && param.getChangePriceValue() != null
///     && param.getOriginalPrice() != null
///     && param.getChangePriceValue().compareTo(param.getOriginalPrice()) > 0)
///     throw new GlobalException("减价金额不能大于应付金额");
/// ```
///
/// `Integer.valueOf(3).equals(...)` 是值比较，故 `totalChangePriceType` 恰为
/// `3` 时生效；`changePriceValue > originalPrice` 才报错，**相等不报错**。
pub fn validate_change_price_value(param: &OrderCheckParam) -> Result<()> {
    if param.total_change_price_type == Some(3) {
        if let (Some(v), Some(orig)) = (param.change_price_value, param.original_price) {
            if v > orig {
                return Err(Error::biz(SERVICE_DATA_ERROR, MSG_REDUCE_GT_PAYABLE));
            }
        }
    }
    Ok(())
}

/// 复刻 Java `submit` 的收货地址校验（`:179-184`）。
///
/// Java：`deliveryType != 2`（自提）时 `addressId` 必须非空，否则
/// `GlobalException("请选择收货地址")`。`deliveryType == null` 也属于
/// 「非自提」→ 同样要求地址。
pub fn validate_submit_address(param: &OrderCheckParam) -> Result<()> {
    if param.delivery_type != Some(ORDER_DELIVERY_TYPE_PICKUP) && param.address_id.is_none() {
        return Err(Error::biz(SERVICE_DATA_ERROR, MSG_SELECT_ADDRESS));
    }
    Ok(())
}

/// 复刻 Java `changePriceShow` 的 `isCartList` 判定（`:161-166`）。
///
/// Java：
/// ```text
/// if (param.getChangePriceValue() == null
///     && (param.getCartChangePrices() == null || param.getCartChangePrices().isEmpty()))
///     param.setIsCartList(true);
/// else
///     param.setIsCartList(false);
/// ```
///
/// 即「完全没传改价信息」时才按购物车列表（DB 数据）展示。
#[must_use]
pub fn resolve_change_price_show_is_cart_list(param: &OrderCheckParam) -> bool {
    param.change_price_value.is_none() && cart_change_prices_is_empty(param)
}

/// 复刻 Java `update` 的 `isCartList` 判定（`:117-122`）。
///
/// Java：`Boolean.FALSE.equals(enableChangePrice)` 时才置 `isCartList = false`
/// 且 `onlyShow = false`；否则 `isCartList = true`（不碰 `onlyShow`）。
/// ⚠️ `enableChangePrice == null` 属于 `true` 分支（`Boolean.FALSE.equals(null)`
/// 为 `false`），这是最容易写错的一处。
///
/// 返回 `(isCartList, 是否改写 onlyShow)`。
#[must_use]
pub fn resolve_update_flags(param: &OrderCheckParam) -> (bool, Option<bool>) {
    if param.enable_change_price == Some(false) {
        (false, Some(false))
    } else {
        (true, None)
    }
}

/// `cartChangePrices` 是否为空（Java `== null || isEmpty()`）。
#[must_use]
pub fn cart_change_prices_is_empty(param: &OrderCheckParam) -> bool {
    match param.cart_change_prices.as_ref() {
        None => true,
        Some(v) => v.is_empty(),
    }
}

/// Java `Integer.parseInt` 的安全版本：脏数据（非数字）返回 `0`。
///
/// 对应 Java `SettingsEnum` 的默认值语义；`None`（配置缺失，Java 为 `""`）
/// 同样返回 `0`。
#[must_use]
pub fn parse_int_config(val: Option<&str>) -> i32 {
    val.map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0)
}

/// 复刻 Java `fillUsePointAndBalance(param)`（`:90-108`）中**仅依赖配置的部分**。
///
/// Java：
/// ```text
/// String adminType = HeaderUtils.getAdminType();
/// if ("admin".equals(adminType)) {
///     OrderSettingsVO s = configService.getOrderSettings();
///     if (s != null) {
///         if (!Integer.valueOf(1).equals(s.getAdminShoppingUseBalance()))
///             param.setUseBalance(BigDecimal.ZERO);
///         if (!Integer.valueOf(1).equals(s.getAdminShoppingUsePoint()))
///             param.setUsePoint(0);
///     }
/// } else {
///     param.setUseBalance(BigDecimal.ZERO);
///     param.setUsePoint(0);
/// }
/// ```
///
/// ⚠️ `Integer.valueOf(1).equals(null)` 为 `false` → 配置项缺失时**同样清零**。
///
/// # 未迁移
///
/// Java 的 `configService.getOrderSettings()` 走 `SettingsHelper` 反射装配
/// （`ConfigServiceImpl#instantiateAndPopulate`）。本实现只读取本方法真正用到的
/// 两个配置键（`adminShoppingUseBalance` / `adminShoppingUsePoint`），
/// 二者都是标量配置项，语义与 Java 一致；`OrderSettingsVO` 其余字段
/// 本控制器不使用，不迁移。
pub fn fill_use_point_and_balance(
    param: &mut OrderCheckParam,
    admin_type: Option<&str>,
    admin_shopping_use_balance: Option<&str>,
    admin_shopping_use_point: Option<&str>,
) {
    if admin_type == Some("admin") {
        if parse_int_config(admin_shopping_use_balance) != 1 {
            param.use_balance = Some(Decimal::ZERO);
        }
        if parse_int_config(admin_shopping_use_point) != 1 {
            param.use_point = Some(0);
        }
    } else {
        param.use_balance = Some(Decimal::ZERO);
        param.use_point = Some(0);
    }
}

/// 复刻 Java `PaymentServiceImpl#isApplePayConfigComplete`（`:205-218`）。
///
/// `provider` 缺省按 `"generic"`；`generic` 时要求支付处理证书私钥已上传，
/// 其余收单机构（ums/adyen/stripe）由收单方解密，不要求。
#[must_use]
pub fn apple_pay_config_complete(cfg: &HashMap<String, String>) -> bool {
    let get = |k: &str| cfg.get(k).map(String::as_str).unwrap_or("");
    let provider = {
        let v = get("applePayAcquirerProvider");
        if v.trim().is_empty() {
            APPLE_PAY_PROVIDER_DEFAULT
        } else {
            v
        }
    };
    let processing_key_ready = !provider.eq_ignore_ascii_case(APPLE_PAY_PROVIDER_DEFAULT)
        || get("applePayProcessingPrivateKey") == "1";
    !get("applePayMerchantId").trim().is_empty()
        && !get("applePayDisplayName").trim().is_empty()
        && get("applePayMerchantIdentityCert") == "1"
        && get("applePayMerchantIdentityKey") == "1"
        && processing_key_ready
        && !get("applePayAcquirerGateway").trim().is_empty()
        && !get("applePayAcquirerMerchantNo").trim().is_empty()
        && !get("applePayAcquirerKey").trim().is_empty()
}

/// 复刻 Java `PaymentServiceImpl#getAvailablePayment("")` 的**判空结果**
/// （`:47-154`）。
///
/// 返回「是否存在任一已启用的在线支付渠道」。逐条对应 Java 的
/// `StrUtil.isNotBlank(val) && Integer.parseInt(val) == 1`：
/// 空/缺失 → 不算启用；非数字 → Java 抛 `NumberFormatException`，
/// 本实现按「未启用」处理（不抛错，避免一个脏配置打挂整个接口）。
///
/// `clientType` 相关分支（`miniProgram` / `wechat` 下不加 alipay 系）
/// **只影响列表内容，不影响是否为空**，故此处不参与。
#[must_use]
pub fn online_payment_available(cfg: &HashMap<String, String>) -> bool {
    let on = |k: &str| cfg.get(k).map(String::as_str).unwrap_or("") == "1";
    if on(CFG_USE_WECHAT) || on(CFG_USE_ALIPAY) || on(CFG_USE_PAYPAL) || on(CFG_USE_LAKALA) {
        return true;
    }
    if on(CFG_USE_APPLE_PAY) && apple_pay_config_complete(cfg) {
        return true;
    }
    // 亚班支付：总开关 + 至少一个子渠道
    if on(CFG_USE_YABANPAY) && (on(CFG_USE_YABANPAY_WECHAT) || on(CFG_USE_YABANPAY_ALIPAY)) {
        return true;
    }
    // 云支付 / 汇付 / 易宝：总开关开启即至少加入一个渠道
    on(CFG_USE_YUNPAY) || on(CFG_USE_HUIFU) || on(CFG_USE_YEEPAY)
}

/// 归一化 `PickupPageQuery`（Java `BasePage` 语义 + `pickupList` 的两处改写）。
///
/// 对应 Java `ShopServiceImpl#pickupList`（`:1620-1641`）：
/// 1. `STORE_SHOW_OTHER_CITY_STORE == "1"` → `regionId = null`；
/// 2. `latitude == null || latitude == 0` → `latitude = null`（经度**不**清零）。
///
/// 返回 `(region_id, latitude)`。
#[must_use]
pub fn normalize_pickup_region_and_lat(
    region_id: Option<String>,
    show_other_city_store: &str,
    latitude: Option<Decimal>,
) -> (Option<String>, Option<Decimal>) {
    let region = if show_other_city_store == "1" {
        None
    } else {
        region_id.filter(|v| !v.is_empty())
    };
    let lat = match latitude {
        Some(v) if v.is_zero() => None,
        other => other,
    };
    (region, lat)
}

// ————————————————————— 服务 —————————————————————

/// 代客下单订单结算服务。
pub struct ProxyShoppingOrderCheckService {
    repos: Arc<Repositories>,
}

impl ProxyShoppingOrderCheckService {
    /// 构造：持有全局仓储集合（与 paylog_batch 等同一约定）。
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &ProxyShoppingOrderCheckRepository {
        &self.repos.trade.proxy_shopping_order_check
    }

    /// `getConfigVal` 语义：缺失 → `""`。
    async fn config_val(&self, code: &str) -> Result<String> {
        Ok(self.repo().find_config_val(code).await?.unwrap_or_default())
    }

    /// `GET /adminapi/proxyShopping/order/check/getAvailablePaymentType`：获取可用支付方式。
    ///
    /// 对应 Java `OrderCheckServiceImpl#getAvailablePaymentType(flowType)`
    /// （`:2990-3036`）。完整迁移，逻辑逐条见该 Java 方法与本模块头第 1 条。
    pub async fn available_payment_types(
        &self,
        q: &AvailablePaymentTypeQuery,
    ) -> Result<Vec<PaymentTypeVO>> {
        // flowType == 12（礼品卡兑换流程）：只返回礼品卡支付，且无条件返回。
        if q.flow_type == Some(FLOW_TYPE_GIFT_CARD) {
            return Ok(vec![PaymentTypeVO {
                type_id: Some(PAYMENT_TYPE_ONLINE),
                type_name: Some("礼品卡兑换".to_string()),
                disabled: false,
                disabled_desc: String::new(),
                is_show: true,
            }]);
        }

        let mut keys: Vec<&str> = PAYMENT_CONFIG_KEYS.to_vec();
        keys.extend(APPLE_PAY_EXTRA_KEYS);
        let cfg = self.repo().find_config_vals(&keys).await?;
        // `getConfigVal` 语义：缺失 → `""`
        let val = |k: &str| cfg.get(k).map(String::as_str).unwrap_or("");

        // Java 用 `getConfigByCode(USE_OFFLINE)`（缺失抛 CONFIG_NOT_EXIST），
        // 本实现按 `getConfigVal` 语义处理：缺失 → `""` → `isShow = false`。
        // 差异已记录在模块头第 5 条与 uncertain_points。
        let use_offline = val(CFG_USE_OFFLINE);

        let mut result = Vec::new();

        // `isSurplus`：Java 用 try/catch 包住 getConfigByCode，配置读取异常 →
        // false（与「配置为 0」同效）。
        let is_surplus = val(CFG_USE_SURPLUS) == "1";

        if is_surplus || online_payment_available(&cfg) {
            result.push(PaymentTypeVO {
                type_id: Some(PAYMENT_TYPE_ONLINE),
                type_name: Some("在线支付".to_string()),
                disabled: false,
                disabled_desc: String::new(),
                is_show: true,
            });
        }

        // 线下支付：Java 无条件加入列表，`isShow` 取决于配置。
        result.push(PaymentTypeVO {
            type_id: Some(PAYMENT_TYPE_OFFLINE),
            type_name: Some("线下支付".to_string()),
            disabled: false,
            disabled_desc: String::new(),
            is_show: use_offline == "1",
        });

        Ok(result)
    }

    /// `GET /adminapi/proxyShopping/order/check/pickupList`：获取自提点列表。
    ///
    /// 对应 Java `ShopServiceImpl#pickupList`（`:1620-1641`）+
    /// `ShopMapper.pickupListByLocation`（`ShopMapper.xml:162-198`）。完整迁移。
    pub async fn pickup_list(&self, q: &PickupPageQuery) -> Result<PageResult<PickupListVO>> {
        // Java：`getById(shopId) == null && shopId > 0` → 「没有此门店」
        if let Some(shop_id) = q.shop_id {
            if shop_id > 0 && !self.repo().shop_exists(shop_id).await? {
                return Err(Error::biz(SERVICE_DATA_ERROR, MSG_SHOP_NOT_FOUND));
            }
        }

        let show_other_city = self.config_val(CFG_STORE_SHOW_OTHER_CITY_STORE).await?;
        let (region_id, latitude) =
            normalize_pickup_region_and_lat(q.region_id.clone(), &show_other_city, q.latitude);

        let page = PageQuery {
            page: q.page_no(),
            page_size: q.page_size(),
        };
        let (limit, offset) = page.limit_offset();
        let query = PickupQuery {
            parent_shop_id: q.shop_id,
            region_id,
            keyword: q.keyword.clone().filter(|v| !v.is_empty()),
            latitude,
            longitude: q.longitude,
            limit,
            offset,
        };

        let total = self.repo().count_pickups(&query).await?;
        let rows = self.repo().pickup_list(&query).await?;

        // Java：逐行 `JSONUtil.toList` / `toBean` 解析
        // + `regionService.getRegionNamesByRegionIds`
        let mut all_region_ids: Vec<i32> = Vec::new();
        let mut parsed: Vec<(PickupListVO, Vec<i32>)> = Vec::with_capacity(rows.len());
        for row in rows {
            // 先借用原始 JSON 串完成解析，再移出其余字段。
            let region_ids = parse_region_ids(row.region_ids_json.as_deref());
            let contact_configs = parse_json_value(row.contact_json.as_deref());
            let open_close_config = parse_json_value(row.open_close_json.as_deref());
            all_region_ids.extend(region_ids.iter().copied());

            let vo = PickupListVO {
                shop_id: Some(row.shop_id),
                shop_title: row.shop_title,
                distance: row.distance,
                longitude: row.longitude,
                latitude: row.latitude,
                contact_configs,
                shop_open_close_config: open_close_config,
                shop_region_ids: Some(region_ids.clone()),
                shop_region_names: None,
                address: row.address,
            };
            parsed.push((vo, region_ids));
        }

        // 批量取地区名（Java 逐行调用，行为等价）
        all_region_ids.sort_unstable();
        all_region_ids.dedup();
        let names: HashMap<i32, String> = self
            .repo()
            .find_region_names(&all_region_ids)
            .await?
            .into_iter()
            .map(|r| (r.region_id, r.region_name))
            .collect();

        let list = parsed
            .into_iter()
            .map(|(mut vo, ids)| {
                vo.shop_region_names = Some(
                    ids.iter()
                        .filter_map(|id| names.get(id).cloned())
                        .collect::<Vec<_>>(),
                );
                vo
            })
            .collect::<Vec<_>>();

        Ok(PageResult::new(total, list, &page))
    }

    /// `POST /adminapi/proxyShopping/order/check/index`：获取结算页信息。
    ///
    /// 对应 Java 控制器 `index`（`:73-88`）。控制器流程：
    /// `flowType = 15` → `validateUserIdNotEmpty`
    /// （`userId <= 0` 报「请选择用户」）→ `indexInitParam()`
    /// （`selectFrom = 1`、余额/礼品卡/积分清零）→ `useDefaultCouponIds = useCoupon`
    /// → `isCartList = true` → `shopId`（请求头）→ `fillUsePointAndBalance`。
    /// 随后调用 `orderCheckService.getOrderCheckData(param)`。
    pub async fn check_index(
        &self,
        param: &OrderCheckParam,
        ctx: &RequestContext,
    ) -> Result<nextrade_core::models::trade::check::OrderCheckVO> {
        let mut param = param.clone();
        param.flow_type = Some(FLOW_TYPE_ADMIN_SHOPPING);
        let user_id = validate_user_id_not_empty(&param)?;
        param.index_init_param();
        let use_coupon = self.config_val(CFG_USE_COUPON).await?;
        param.use_default_coupon_ids = Some(parse_int_config(Some(use_coupon.as_str())));
        param.is_cart_list = Some(true);
        param.shop_id = Some(ctx.shop_id_or_zero());
        self.apply_admin_settings(&mut param, ctx).await?;
        let check_param = convert_to_check_param(&param)?;
        let check_service = crate::trade::check::CheckService::new(self.repos.clone());
        check_service.order_check_index(user_id, &check_param).await
    }

    /// `POST /adminapi/proxyShopping/order/check/update`：更新结算信息。
    ///
    /// 对应 Java 控制器 `update`（`:111-126`）。
    pub async fn check_update(
        &self,
        param: &OrderCheckParam,
        ctx: &RequestContext,
    ) -> Result<nextrade_core::models::trade::check::OrderCheckVO> {
        let mut param = param.clone();
        param.flow_type = Some(FLOW_TYPE_ADMIN_SHOPPING);
        let user_id = validate_user_id_not_empty(&param)?;
        param.select_from = Some(SELECT_FROM_CHECK_INDEX);
        let (is_cart_list, only_show) = resolve_update_flags(&param);
        param.is_cart_list = Some(is_cart_list);
        if let Some(v) = only_show {
            param.only_show = Some(v);
        }
        param.shop_id = Some(ctx.shop_id_or_zero());
        self.apply_admin_settings(&mut param, ctx).await?;
        let check_param = convert_to_check_param(&param)?;
        let check_service = crate::trade::check::CheckService::new(self.repos.clone());
        check_service.order_check_update(user_id, &check_param).await
    }

    /// `POST /adminapi/proxyShopping/order/check/updateCoupon`：更新优惠券。
    ///
    /// 对应 Java 控制器 `updateCoupon`（`:128-138`）。
    pub async fn check_update_coupon(
        &self,
        param: &OrderCheckParam,
        ctx: &RequestContext,
    ) -> Result<nextrade_core::models::trade::check::OrderCheckVO> {
        let mut param = param.clone();
        param.flow_type = Some(FLOW_TYPE_ADMIN_SHOPPING);
        let user_id = validate_user_id_not_empty(&param)?;
        param.select_from = Some(SELECT_FROM_CHECK_UPDATE);
        param.is_cart_list = Some(true);
        param.shop_id = Some(ctx.shop_id_or_zero());
        self.apply_admin_settings(&mut param, ctx).await?;
        let check_param = convert_to_check_param(&param)?;
        let check_service = crate::trade::check::CheckService::new(self.repos.clone());
        check_service.order_check_update_coupon(user_id, &check_param).await
    }

    /// `POST /adminapi/proxyShopping/order/check/changePrice`：改价。
    ///
    /// 对应 Java 控制器 `changePrice`（`:140-152`）。
    pub async fn change_price(
        &self,
        param: &OrderCheckParam,
        ctx: &RequestContext,
    ) -> Result<nextrade_core::models::trade::check::OrderCheckVO> {
        let mut param = param.clone();
        param.flow_type = Some(FLOW_TYPE_ADMIN_SHOPPING);
        let user_id = validate_user_id_not_empty(&param)?;
        validate_change_price_value(&param)?;
        param.select_from = Some(SELECT_FROM_CHECK_INDEX);
        param.shop_id = Some(ctx.shop_id_or_zero());
        param.is_cart_list = Some(false);
        param.only_show = Some(false);
        self.apply_admin_settings(&mut param, ctx).await?;
        let check_param = convert_to_check_param(&param)?;
        let check_service = crate::trade::check::CheckService::new(self.repos.clone());
        check_service.order_check_update(user_id, &check_param).await
    }

    /// `POST /adminapi/proxyShopping/order/check/changePriceShow`：改价展示。
    ///
    /// 对应 Java 控制器 `changePriceShow`（`:154-171`）。
    pub async fn change_price_show(
        &self,
        param: &OrderCheckParam,
        ctx: &RequestContext,
    ) -> Result<nextrade_core::models::trade::check::OrderCheckVO> {
        let mut param = param.clone();
        param.flow_type = Some(FLOW_TYPE_ADMIN_SHOPPING);
        let user_id = validate_user_id_not_empty(&param)?;
        validate_change_price_value(&param)?;
        param.shop_id = Some(ctx.shop_id_or_zero());
        param.is_cart_list = Some(resolve_change_price_show_is_cart_list(&param));
        param.only_show = Some(true);
        param.index_init_param();
        self.apply_admin_settings(&mut param, ctx).await?;
        let check_param = convert_to_check_param(&param)?;
        let check_service = crate::trade::check::CheckService::new(self.repos.clone());
        check_service.order_check_update(user_id, &check_param).await
    }

    /// `POST /adminapi/proxyShopping/order/check/submit`：提交订单。
    ///
    /// 对应 Java 控制器 `submit`（`:173-195`）。控制器流程（**不调用**
    /// `validateUserIdNotEmpty`）：
    /// `flowType = 15` → 非自提必须传 `addressId`（否则「请选择收货地址」）
    /// → `posUserId = 当前管理员 id` → `isCartList = true`
    /// → `orderSource = "代客下单"` → `cashAmount = cashChangeAmount = 0`
    /// → `shopId` → `fillUsePointAndBalance`
    /// → `orderCheckService.submit(param)` —— 本实现复用
    /// `OrderService::create` 完成下单，见模块头第 4 条。
    ///
    /// `admin_id` 取 JWT 身份（Java `SecurityUtils.getCurrentAdminId()`）。
    pub async fn submit(
        &self,
        param: &OrderCheckParam,
        ctx: &RequestContext,
        admin_id: i32,
    ) -> Result<nextrade_core::models::OrderCreateVO> {
        let mut param = param.clone();
        param.flow_type = Some(FLOW_TYPE_ADMIN_SHOPPING);
        validate_submit_address(&param)?;
        let user_id = param
            .user_id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, MSG_SELECT_USER))?;
        param.pos_user_id = Some(admin_id);
        param.is_cart_list = Some(true);
        param.order_source = Some(ORDER_SOURCE_PROXY_SHOPPING.to_string());
        param.cash_amount = Some(CASH_AMOUNT_ZERO);
        param.cash_change_amount = Some(CASH_AMOUNT_ZERO);
        param.shop_id = Some(ctx.shop_id_or_zero());
        self.apply_admin_settings(&mut param, ctx).await?;

        // 查找该用户在代客下单（type=15）中已勾选的购物车行
        let cart_rows = self
            .repos
            .trade
            .proxy_shopping_cart
            .list_carts(user_id, None, None, Some(1))
            .await?;
        let cart_ids: Vec<i32> = if !cart_rows.is_empty() {
            cart_rows.into_iter().map(|c| c.cart_id).collect()
        } else {
            let all_rows = self
                .repos
                .trade
                .proxy_shopping_cart
                .list_carts(user_id, None, None, None)
                .await?;
            if all_rows.is_empty() {
                return Err(Error::biz(SERVICE_DATA_ERROR, "请先添加商品到购物车".to_string()));
            }
            let ids: Vec<i32> = all_rows.iter().map(|c| c.cart_id).collect();
            for cid in &ids {
                let _ = self
                    .repos
                    .trade
                    .proxy_shopping_cart
                    .update_checked(*cid, 1, None)
                    .await;
            }
            ids
        };

        let invoice_data = param.invoice_data.as_ref().map(|inv| nextrade_core::models::trade::order::InvoiceDataDTO {
            invoice_type: inv.invoice_type,
            title_type: inv.title_type,
            title: inv.title.clone().or_else(|| inv.company_name.clone()),
            tax_no: inv.tax_no.clone(),
            content: inv.content.clone(),
            tax_phone: inv.tax_phone.clone(),
            company_name: inv.company_name.clone(),
            email: None,
            mobile: None,
        });

        let create_dto = nextrade_core::models::OrderCreateDTO {
            cart_ids,
            address_id: param.address_id,
            shipping_method: param.delivery_type.or(Some(1)),
            delivery_type: param.delivery_type,
            pickup_id: param.pickup_id,
            pickup_contact_name: param.pickup_contact_name.clone(),
            pickup_contact_mobile: param.pickup_contact_mobile.clone(),
            coupon_id: param.use_coupon_ids.as_ref().and_then(|ids| ids.first().copied()),
            use_points: param.use_point,
            buyer_note: param.buyer_note.clone(),
            pay_type_id: param.pay_type_id,
            use_balance: param.use_balance,
            use_coupon_ids: param.use_coupon_ids.clone(),
            invoice_data,
            order_source: Some(ORDER_SOURCE_PROXY_SHOPPING.to_string()),
            pos_user_id: Some(admin_id),
        };

        let product_svc = crate::catalog::product::ProductService::new(self.repos.clone(), None);
        let cart_svc = crate::trade::cart::CartService::new(self.repos.clone(), product_svc.clone());
        let order_service = crate::trade::order::OrderService::new(self.repos.clone(), product_svc, cart_svc);
        let vo = order_service.create(user_id, &create_dto).await?;

        let mut patch = nextrade_infrastructure::db::repo::trade::order_admin::OrderPatch::new()
            .str_val("order_source", ORDER_SOURCE_PROXY_SHOPPING)
            .i32_val("pos_user_id", admin_id);

        if let Some(change_val) = param.change_price_value {
            if change_val > Decimal::ZERO {
                patch = patch.decimal("discount_amount", change_val);
            }
        }

        let _ = self
            .repos
            .trade
            .order_admin
            .update_order(vo.order_id, &patch)
            .await;

        let log_svc = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        let _ = log_svc
            .create_by_log_info(
                admin_id,
                format!("代客下单创建订单: 订单号={}, 会员ID={}", vo.order_sn, user_id),
            )
            .await;

        Ok(vo)
    }

    /// `fillUsePointAndBalance` 的 DB 部分：取 `adminShoppingUse*` 两个配置后应用。
    async fn apply_admin_settings(
        &self,
        param: &mut OrderCheckParam,
        ctx: &RequestContext,
    ) -> Result<()> {
        let balance = self.config_val(CFG_ADMIN_SHOPPING_USE_BALANCE).await?;
        let point = self.config_val(CFG_ADMIN_SHOPPING_USE_POINT).await?;
        fill_use_point_and_balance(
            param,
            ctx.admin_type(),
            Some(balance.as_str()),
            Some(point.as_str()),
        );
        Ok(())
    }
}

/// `SelectFromEnum.CHECK_INDEX`（Java `SelectFromEnum.java:15`，值 2）。
pub const SELECT_FROM_CHECK_INDEX: i32 = 2;
/// `SelectFromEnum.CHECK_UPDATE`（Java `SelectFromEnum.java:16`，值 3）。
pub const SELECT_FROM_CHECK_UPDATE: i32 = 3;

/// `JSONUtil.toList(shopRegionIdsJson, Integer.class)` 的等价实现。
///
/// Java 在 JSON 非法时抛异常 → 整个接口失败。本实现返回空列表并继续
/// （**更宽松**）：原库里 `shop_region_id` 是 VARCHAR，存在历史脏数据
/// （空串 / `"null"` 字面量），严格抛错会让自提点列表整体不可用。
/// 已在 `uncertain_points` 标注。
#[must_use]
pub fn parse_region_ids(raw: Option<&str>) -> Vec<i32> {
    let Some(raw) = raw else {
        return Vec::new();
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return Vec::new();
    }
    serde_json::from_str::<Vec<i32>>(raw).unwrap_or_default()
}

/// `JSONUtil.toList` / `toBean` 的宽松版：解析失败或为空 → `None`。
///
/// 对应 `PickupListVO.contactConfigs` / `shopOpenCloseConfig`。Java 侧那两个
/// 原始 JSON 字段带 `@JsonIgnore`，不出现在响应里；本实现把解析结果原样透传
/// （`StoreExtendedDto.ShopContactConfig` / `ShopOpenCloseConfig` 的类型定义
/// 属门店扩展域，未迁移，故用 `serde_json::Value` 承载）。
#[must_use]
pub fn parse_json_value(raw: Option<&str>) -> Option<serde_json::Value> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }
    serde_json::from_str(raw).ok()
}

/// 按店铺 ID 取配送方式明细（对应 Java `OrderCheckParam.shippingType` 的 Map 语义）。
///
/// 本切片未在已迁移链路上使用，保留以表达该契约。
#[must_use]
pub fn shipping_type_of<'a>(
    map: Option<&'a HashMap<i32, ShippingType>>,
    shop_id: i32,
) -> Option<&'a ShippingType> {
    map?.get(&shop_id)
}

/// 判断某项改价是否为「一口价」（Java `CartChangePrice.changePriceType == 1`）。
#[must_use]
pub fn is_fixed_price_change(row: &CartChangePrice) -> bool {
    row.change_price_type == Some(1)
}

/// 将代客下单参数转换为结算域通用入参
pub fn convert_to_check_param(
    param: &OrderCheckParam,
) -> Result<nextrade_core::models::trade::check::OrderCheckParam> {
    let json_val = serde_json::to_value(param)
        .map_err(|e| nextrade_core::Error::biz(500, e.to_string()))?;
    serde_json::from_value(json_val)
        .map_err(|e| nextrade_core::Error::biz(500, e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::trade::proxy_shopping_order_check::{CartChangePrice, InvoiceData};

    fn param() -> OrderCheckParam {
        OrderCheckParam::default()
    }

    fn cfg(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    // ————————————————— 控制器校验：userId —————————————————

    #[test]
    fn validate_user_id_requires_positive_value() {
        assert!(validate_user_id_not_empty(&param()).is_err());
        let mut p = param();
        p.user_id = Some(0);
        assert!(validate_user_id_not_empty(&p).is_err());
        p.user_id = Some(-3);
        assert!(validate_user_id_not_empty(&p).is_err());
        p.user_id = Some(1);
        assert_eq!(validate_user_id_not_empty(&p).unwrap(), 1);
    }

    #[test]
    fn validate_user_id_error_matches_java_text_and_code() {
        match validate_user_id_not_empty(&param()) {
            Err(Error::Biz { code, msg }) => {
                assert_eq!(code, SERVICE_DATA_ERROR);
                assert_eq!(msg, MSG_SELECT_USER);
                assert_eq!(msg, "请选择用户");
            }
            other => panic!("应为业务错误，实际 {other:?}"),
        }
    }

    // ————————————————— 控制器校验：改价 —————————————————

    #[test]
    fn change_price_reduce_only_checked_for_type_3() {
        let mut p = param();
        p.total_change_price_type = Some(3);
        p.change_price_value = Some(Decimal::new(200, 0));
        p.original_price = Some(Decimal::new(100, 0));
        assert!(validate_change_price_value(&p).is_err());

        p.total_change_price_type = Some(1);
        assert!(validate_change_price_value(&p).is_ok());
        p.total_change_price_type = Some(2);
        assert!(validate_change_price_value(&p).is_ok());
    }

    #[test]
    fn change_price_equal_amount_is_allowed() {
        let mut p = param();
        p.total_change_price_type = Some(3);
        p.change_price_value = Some(Decimal::new(1000, 2));
        p.original_price = Some(Decimal::new(1000, 2));
        assert!(validate_change_price_value(&p).is_ok());
    }

    #[test]
    fn change_price_missing_operands_are_skipped() {
        let mut p = param();
        p.total_change_price_type = Some(3);
        assert!(validate_change_price_value(&p).is_ok());
        p.change_price_value = Some(Decimal::new(999, 0));
        assert!(validate_change_price_value(&p).is_ok());
        p.change_price_value = None;
        p.original_price = Some(Decimal::new(1, 0));
        assert!(validate_change_price_value(&p).is_ok());
    }

    #[test]
    fn change_price_error_text_matches_java() {
        let mut p = param();
        p.total_change_price_type = Some(3);
        p.change_price_value = Some(Decimal::new(2, 0));
        p.original_price = Some(Decimal::new(1, 0));
        match validate_change_price_value(&p) {
            Err(Error::Biz { code, msg }) => {
                assert_eq!(code, SERVICE_DATA_ERROR);
                assert_eq!(msg, MSG_REDUCE_GT_PAYABLE);
                assert_eq!(msg, "减价金额不能大于应付金额");
            }
            other => panic!("应为业务错误，实际 {other:?}"),
        }
    }

    // ————————————————— 控制器校验：提交收货地址 —————————————————

    #[test]
    fn submit_address_required_unless_pickup() {
        assert!(validate_submit_address(&param()).is_err());

        let mut p = param();
        p.delivery_type = Some(ORDER_DELIVERY_TYPE_PICKUP);
        assert!(validate_submit_address(&p).is_ok(), "自提不要求地址");

        p.delivery_type = Some(1);
        assert!(validate_submit_address(&p).is_err());
        p.address_id = Some(7);
        assert!(validate_submit_address(&p).is_ok());

        p.delivery_type = Some(3);
        p.address_id = None;
        assert!(validate_submit_address(&p).is_err());
    }

    // ————————————————— update / changePriceShow 的 isCartList —————————————————

    #[test]
    fn update_flags_follow_boolean_false_equals() {
        let p = param();
        assert_eq!(resolve_update_flags(&p), (true, None));

        let mut p = param();
        p.enable_change_price = Some(true);
        assert_eq!(resolve_update_flags(&p), (true, None));

        p.enable_change_price = Some(false);
        assert_eq!(resolve_update_flags(&p), (false, Some(false)));
    }

    #[test]
    fn change_price_show_is_cart_list_only_when_no_change_info() {
        assert!(resolve_change_price_show_is_cart_list(&param()));

        let mut p = param();
        p.cart_change_prices = Some(vec![]);
        assert!(resolve_change_price_show_is_cart_list(&p), "空列表等同未传");

        p.cart_change_prices = Some(vec![CartChangePrice {
            cart_id: Some(1),
            ..Default::default()
        }]);
        assert!(!resolve_change_price_show_is_cart_list(&p));

        let mut p = param();
        p.change_price_value = Some(Decimal::ZERO);
        assert!(!resolve_change_price_show_is_cart_list(&p));
    }

    // ————————————————— 积分/余额开关 —————————————————

    #[test]
    fn non_admin_always_clears_balance_and_point() {
        let mut p = param();
        p.use_balance = Some(Decimal::new(500, 2));
        p.use_point = Some(100);
        fill_use_point_and_balance(&mut p, Some("shop"), Some("1"), Some("1"));
        assert_eq!(p.use_balance, Some(Decimal::ZERO));
        assert_eq!(p.use_point, Some(0));
    }

    #[test]
    fn admin_keeps_balance_only_when_config_is_one() {
        let mut p = param();
        p.use_balance = Some(Decimal::new(1234, 2));
        p.use_point = Some(77);
        fill_use_point_and_balance(&mut p, Some("admin"), Some("1"), Some("0"));
        assert_eq!(p.use_balance, Some(Decimal::new(1234, 2)), "配置=1 保留余额");
        assert_eq!(p.use_point, Some(0), "配置=0 清零积分");
    }

    #[test]
    fn admin_missing_config_clears_like_java_equals_null() {
        let mut p = param();
        p.use_balance = Some(Decimal::new(100, 0));
        p.use_point = Some(5);
        fill_use_point_and_balance(&mut p, Some("admin"), None, None);
        assert_eq!(p.use_balance, Some(Decimal::ZERO));
        assert_eq!(p.use_point, Some(0));
    }

    #[test]
    fn admin_type_missing_is_treated_like_non_admin() {
        let mut p = param();
        p.use_balance = Some(Decimal::new(900, 0));
        p.use_point = Some(9);
        fill_use_point_and_balance(&mut p, None, Some("1"), Some("1"));
        assert_eq!(p.use_balance, Some(Decimal::ZERO));
        assert_eq!(p.use_point, Some(0));
    }

    #[test]
    fn parse_int_config_is_lenient_about_dirty_values() {
        assert_eq!(parse_int_config(Some("1")), 1);
        assert_eq!(parse_int_config(Some(" 2 ")), 2);
        assert_eq!(parse_int_config(Some("-1")), -1);
        assert_eq!(parse_int_config(None), 0);
        assert_eq!(parse_int_config(Some("")), 0);
        assert_eq!(parse_int_config(Some("abc")), 0);
        assert_eq!(parse_int_config(Some("1.5")), 0);
    }

    // ————————————————— indexInitParam —————————————————

    #[test]
    fn index_init_param_zeroes_balance_gift_card_point() {
        let mut p = param();
        p.use_balance = Some(Decimal::new(5000, 2));
        p.use_gift_card = Some(Decimal::new(200, 0));
        p.use_point = Some(30);
        p.select_from = Some(4);
        p.index_init_param();
        assert_eq!(p.select_from, Some(1));
        assert_eq!(p.use_balance, Some(Decimal::ZERO));
        assert_eq!(p.use_gift_card, Some(Decimal::ZERO));
        assert_eq!(p.use_point, Some(0));
    }

    // ————————————————— 自提点归一化 —————————————————

    #[test]
    fn pickup_region_dropped_when_show_other_city_enabled() {
        let (region, lat) = normalize_pickup_region_and_lat(
            Some("110000".into()),
            "1",
            Some(Decimal::new(30000000, 6)),
        );
        assert_eq!(region, None, "配置开启时清空 regionId");
        assert!(lat.is_some(), "纬度非 0 时保留");
    }

    #[test]
    fn pickup_region_kept_and_empty_string_dropped() {
        let (region, _) =
            normalize_pickup_region_and_lat(Some("110000".into()), "0", Some(Decimal::ONE));
        assert_eq!(region, Some("110000".to_string()));
        let (region, _) = normalize_pickup_region_and_lat(Some(String::new()), "0", None);
        assert_eq!(region, None, "空串等同未传");
    }

    #[test]
    fn pickup_latitude_zero_is_nulled_but_longitude_is_not() {
        let (_, lat) = normalize_pickup_region_and_lat(None, "0", Some(Decimal::ZERO));
        assert_eq!(lat, None);
        let (_, lat) = normalize_pickup_region_and_lat(None, "0", None);
        assert_eq!(lat, None);
        let (_, lat) = normalize_pickup_region_and_lat(None, "0", Some(Decimal::new(-1, 6)));
        assert_eq!(lat, Some(Decimal::new(-1, 6)));
    }

    // ————————————————— 在线支付可用性 —————————————————

    #[test]
    fn online_payment_single_switches() {
        assert!(online_payment_available(&cfg(&[("useWechat", "1")])));
        assert!(online_payment_available(&cfg(&[("useAlipay", "1")])));
        assert!(online_payment_available(&cfg(&[("usePaypal", "1")])));
        assert!(online_payment_available(&cfg(&[("useLakala", "1")])));
        assert!(online_payment_available(&cfg(&[("useYunpay", "1")])));
        assert!(online_payment_available(&cfg(&[("useHuifu", "1")])));
        assert!(online_payment_available(&cfg(&[("useYeepay", "1")])));
        assert!(!online_payment_available(&cfg(&[("useWechat", "0")])));
        assert!(!online_payment_available(&cfg(&[("useWechat", "")])));
        assert!(!online_payment_available(&cfg(&[("useWechat", "abc")])));
        assert!(!online_payment_available(&cfg(&[])));
    }

    #[test]
    fn online_payment_yabanpay_needs_child_switch() {
        assert!(!online_payment_available(&cfg(&[("useYabanpay", "1")])));
        assert!(online_payment_available(&cfg(&[
            ("useYabanpay", "1"),
            ("useYabanpayWechat", "1"),
        ])));
        assert!(online_payment_available(&cfg(&[
            ("useYabanpay", "1"),
            ("useYabanpayAlipay", "1"),
        ])));
        assert!(!online_payment_available(&cfg(&[("useYabanpayWechat", "1")])));
    }

    #[test]
    fn online_payment_apple_pay_needs_complete_config() {
        let base = [
            ("useApplePay", "1"),
            ("applePayMerchantId", "merchant.com.x"),
            ("applePayDisplayName", "Shop"),
            ("applePayMerchantIdentityCert", "1"),
            ("applePayMerchantIdentityKey", "1"),
            ("applePayAcquirerGateway", "https://gw"),
            ("applePayAcquirerMerchantNo", "M001"),
            ("applePayAcquirerKey", "KEY"),
            ("applePayProcessingPrivateKey", "1"),
        ];
        assert!(online_payment_available(&cfg(&base)));

        let mut missing = base.to_vec();
        missing[1] = ("applePayMerchantId", "");
        assert!(!online_payment_available(&cfg(&missing)));

        let mut missing = base.to_vec();
        missing[3] = ("applePayMerchantIdentityCert", "0");
        assert!(!online_payment_available(&cfg(&missing)));

        let mut missing = base.to_vec();
        missing[8] = ("applePayProcessingPrivateKey", "0");
        assert!(
            !online_payment_available(&cfg(&missing)),
            "generic 必须上传处理私钥"
        );

        let mut non_generic = base.to_vec();
        non_generic[8] = ("applePayProcessingPrivateKey", "0");
        non_generic.push(("applePayAcquirerProvider", "stripe"));
        assert!(
            online_payment_available(&cfg(&non_generic)),
            "非 generic 不要求处理私钥"
        );
    }

    #[test]
    fn apple_pay_provider_defaults_to_generic_when_blank() {
        let mut c = cfg(&[
            ("applePayMerchantId", "m"),
            ("applePayDisplayName", "n"),
            ("applePayMerchantIdentityCert", "1"),
            ("applePayMerchantIdentityKey", "1"),
            ("applePayAcquirerGateway", "g"),
            ("applePayAcquirerMerchantNo", "no"),
            ("applePayAcquirerKey", "k"),
        ]);
        assert!(!apple_pay_config_complete(&c));
        c.insert("applePayProcessingPrivateKey".into(), "1".into());
        assert!(apple_pay_config_complete(&c));
        c.insert("applePayAcquirerProvider".into(), "   ".into());
        assert!(apple_pay_config_complete(&c));
    }

    // ————————————————— JSON 宽松解析 —————————————————

    #[test]
    fn parse_region_ids_handles_typical_and_dirty_values() {
        assert_eq!(parse_region_ids(Some("[1,2,3]")), vec![1, 2, 3]);
        assert_eq!(
            parse_region_ids(Some(" [110000,110100] ")),
            vec![110000, 110100]
        );
        assert!(parse_region_ids(Some("[]")).is_empty());
        assert!(parse_region_ids(Some("")).is_empty());
        assert!(parse_region_ids(Some("null")).is_empty());
        assert!(parse_region_ids(Some("110000")).is_empty());
        assert!(parse_region_ids(None).is_empty());
    }

    #[test]
    fn parse_json_value_passes_through_and_tolerates_dirt() {
        let v = parse_json_value(Some(r#"[{"name":"x"}]"#)).unwrap();
        assert!(v.is_array());
        assert!(parse_json_value(Some("")).is_none());
        assert!(parse_json_value(Some("not-json")).is_none());
        assert!(parse_json_value(None).is_none());
    }

    // ————————————————— 请求上下文（替代 Java HeaderUtils） —————————————————

    #[test]
    fn request_context_normalizes_shop_id() {
        // 路由层解析请求头后的语义（`admin` → 0；`shop`/`pos` → x-shop-id；
        // 缺失 → None → 归一为 0），与 product_stock 的 `shop_id_from_headers` 一致。
        let admin = RequestContext::new(Some(0), Some("admin".into()));
        assert_eq!(admin.shop_id_or_zero(), 0, "平台视角恒 0");
        assert_eq!(admin.admin_type(), Some("admin"));

        let shop = RequestContext::new(Some(42), Some("shop".into()));
        assert_eq!(shop.shop_id_or_zero(), 42);

        let pos = RequestContext::new(Some(7), Some("pos".into()));
        assert_eq!(pos.shop_id_or_zero(), 7);

        let missing = RequestContext::default();
        assert_eq!(missing.shop_id_or_zero(), 0, "缺失 → 0（Java null 归一）");
        assert_eq!(missing.admin_type(), None);

        let no_shop_id = RequestContext::new(None, Some("shop".into()));
        assert_eq!(no_shop_id.shop_id_or_zero(), 0, "缺 x-shop-id → 0");
    }

    #[test]
    fn request_context_admin_type_drives_balance_switch() {
        // `fill_use_point_and_balance` 只看 admin_type 是否恰为 "admin"
        let mut p = param();
        p.use_balance = Some(Decimal::new(100, 0));
        p.use_point = Some(3);
        let ctx = RequestContext::new(Some(9), Some("shop".into()));
        fill_use_point_and_balance(&mut p, ctx.admin_type(), Some("1"), Some("1"));
        assert_eq!(p.use_balance, Some(Decimal::ZERO));
        assert_eq!(p.use_point, Some(0));

        let mut p = param();
        p.use_balance = Some(Decimal::new(100, 0));
        p.use_point = Some(3);
        let ctx = RequestContext::new(Some(0), Some("admin".into()));
        fill_use_point_and_balance(&mut p, ctx.admin_type(), Some("1"), Some("1"));
        assert_eq!(p.use_balance, Some(Decimal::new(100, 0)));
        assert_eq!(p.use_point, Some(3));
    }

    // ————————————————— DTO JSON 兼容性 —————————————————

    #[test]
    fn pickup_query_accepts_frontend_camel_case_and_defaults() {
        let q: PickupPageQuery = serde_json::from_str(r#"{"shopId":3}"#).unwrap();
        assert_eq!(q.shop_id, Some(3));
        assert_eq!(q.page_no(), 1, "缺省 1（Java BasePage）");
        assert_eq!(q.page_size(), 15, "缺省 15（Java BasePage）");

        let q: PickupPageQuery =
            serde_json::from_str(r#"{"page":2,"size":500,"latitude":31.2,"longitude":121.4}"#)
                .unwrap();
        assert_eq!(q.page_no(), 2);
        assert_eq!(q.page_size(), 100, "超过 100 截断为 100");
        assert_eq!(q.latitude, Some(Decimal::new(312, 1)));
        assert_eq!(q.longitude, Some(Decimal::new(1214, 1)));
    }

    #[test]
    fn available_payment_type_query_maps_camel_case_query_param() {
        // 查询参数名是 `flowType`（Java `@RequestParam("flowType")`）。
        // 这里用 JSON 验证 `rename_all = "camelCase"` 的映射
        // （`serde_urlencoded` 不是本 crate 的依赖，故不做 URL 解析断言）。
        let q: AvailablePaymentTypeQuery = serde_json::from_str(r#"{"flowType":12}"#).unwrap();
        assert_eq!(q.flow_type, Some(FLOW_TYPE_GIFT_CARD));
        // 缺省（不传）→ None（Java `required = false`）
        let q: AvailablePaymentTypeQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(q.flow_type, None);
    }

    #[test]
    fn order_check_param_accepts_full_frontend_payload() {
        let raw = r#"{
            "userId": 9,
            "flowType": 15,
            "deliveryType": 2,
            "addressId": null,
            "usePoint": 0,
            "useBalance": "0.00",
            "useCouponIds": [1,2],
            "selectUserCouponIds": [3],
            "enableChangePrice": false,
            "totalChangePriceType": 3,
            "changePriceValue": "5.00",
            "originalPrice": "100.00",
            "cartChangePrices": [{"cartId": 1, "productId": 2, "changePriceType": 1}],
            "invoiceData": {"invoiceType": 1, "titleType": 2, "title": "T"},
            "productExtra": {"k": "v"},
            "shippingType": {"1": {"typeId": 1, "shopId": 1, "pickupId": 5}}
        }"#;
        let p: OrderCheckParam = serde_json::from_str(raw).unwrap();
        assert_eq!(p.user_id, Some(9));
        assert_eq!(p.flow_type, Some(FLOW_TYPE_ADMIN_SHOPPING));
        assert_eq!(p.delivery_type, Some(ORDER_DELIVERY_TYPE_PICKUP));
        assert_eq!(p.use_balance, Some(Decimal::ZERO));
        assert_eq!(p.use_coupon_ids.as_deref(), Some([1, 2].as_slice()));
        assert_eq!(p.enable_change_price, Some(false));
        assert_eq!(p.change_price_value, Some(Decimal::new(500, 2)));
        assert!(!cart_change_prices_is_empty(&p));
        assert!(is_fixed_price_change(&p.cart_change_prices.as_ref().unwrap()[0]));
        assert!(p.invoice_data.is_some());
        assert_eq!(
            p.product_extra.as_ref().unwrap().get("k").map(String::as_str),
            Some("v")
        );
        // `shippingType` 的键在 JSON 里是字符串，serde_json 会解析为 i32 键 1
        let st = p.shipping_type.as_ref().expect("shippingType 应解析成功");
        assert_eq!(st.get(&1).and_then(|v| v.type_id), Some(1));
        assert_eq!(st.get(&1).and_then(|v| v.pickup_id), Some(5));
        assert_eq!(
            shipping_type_of(p.shipping_type.as_ref(), 1).and_then(|v| v.type_id),
            Some(1)
        );
    }

    #[test]
    fn order_check_param_defaults_are_none_not_zero() {
        let p: OrderCheckParam = serde_json::from_str("{}").unwrap();
        assert_eq!(p.enable_change_price, None);
        assert_eq!(p.use_balance, None);
        assert_eq!(p.use_point, None);
        assert_eq!(p.user_id, None);
        assert!(p.invoice_data.is_none());
    }

    #[test]
    fn payment_type_vo_serializes_with_is_show_camel_case() {
        let vo = PaymentTypeVO {
            type_id: Some(PAYMENT_TYPE_ONLINE),
            type_name: Some("在线支付".into()),
            disabled: false,
            disabled_desc: String::new(),
            is_show: true,
        };
        let s = serde_json::to_string(&vo).unwrap();
        assert!(s.contains(r#""typeId":1"#), "实际: {s}");
        assert!(s.contains(r#""isShow":true"#), "实际: {s}");
        assert!(s.contains(r#""disabledDesc":"""#), "实际: {s}");
    }

    #[test]
    fn pickup_list_vo_omits_ignored_raw_json_fields() {
        let vo = PickupListVO {
            shop_id: Some(1),
            shop_title: Some("T".into()),
            distance: Some(Decimal::new(12345, 2)),
            longitude: None,
            latitude: None,
            contact_configs: None,
            shop_open_close_config: None,
            shop_region_ids: Some(vec![110000]),
            shop_region_names: Some(vec!["北京".into()]),
            address: None,
        };
        let s = serde_json::to_string(&vo).unwrap();
        assert!(!s.contains("contactJson"), "实际: {s}");
        assert!(!s.contains("shopOpenCloseConfigJson"), "实际: {s}");
        assert!(!s.contains("shopRegionIdsJson"), "实际: {s}");
        assert!(s.contains(r#""shopId":1"#), "实际: {s}");
        assert!(s.contains(r#""shopRegionNames":["北京"]"#), "实际: {s}");
    }

    #[test]
    fn invoice_data_and_shipping_type_use_camel_case() {
        let inv: InvoiceData = serde_json::from_str(r#"{"taxNo":"T1","companyName":"C"}"#).unwrap();
        assert_eq!(inv.tax_no.as_deref(), Some("T1"));
        assert_eq!(inv.company_name.as_deref(), Some("C"));

        let st: ShippingType =
            serde_json::from_str(r#"{"typeId":1,"expectPickupTime":"10:00"}"#).unwrap();
        assert_eq!(st.type_id, Some(1));
        assert_eq!(st.expect_pickup_time.as_deref(), Some("10:00"));
    }

    // ————————————————— 常量 —————————————————

    #[test]
    fn proxy_order_constants_match_java() {
        assert_eq!(MSG_SHOP_NOT_FOUND, "没有此门店");
        assert_eq!(ORDER_SOURCE_PROXY_SHOPPING, "代客下单");
        assert_eq!(FLOW_TYPE_ADMIN_SHOPPING, 15);
        assert_eq!(CASH_AMOUNT_ZERO, Decimal::ZERO);
        assert_eq!(SERVICE_DATA_ERROR, 1001);
    }
}