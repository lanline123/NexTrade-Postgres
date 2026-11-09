//! 代客下单购物车域（trade::proxy_shopping_cart）DTO / VO / 纯函数。
//!
//! 对应原 Java `com.nextrade.adminapi.controller.proxyshopping.ProxyShoppingCartController`
//! （`@RequestMapping("/adminapi/proxyShopping/cart")`，7 个端点）。
//!
//! # 口径说明
//!
//! - 购物车类型固定为 `CartTypeEnum.TYPE_ADMIN_SHOPPING = 15`，按 `userId + shopId` 隔离。
//! - 商品类型 → 购物车类型映射与 Java `adminShopping2CartType` 一致（见
//!   [`cart_type_from_product_type`]）。
//! - **价格/活动计算引擎未迁移**：列表返回 DB 快照价格与「快照金额小计」，
//!   所有优惠/活动/改价相关字段返回 `null`（不编造数值），逐端点记录于服务层文档。
//! - 金额 `rust_decimal::Decimal`；时间 i32 秒。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ——————————————————————————————————————————————
// 常量（文案逐字对齐 Java / i18n）
// ——————————————————————————————————————————————

/// `请选择用户`（控制器 `validateUserIdNotEmpty`）。
pub const MSG_USER_REQUIRED: &str = "请选择用户";
/// `商品不存在`（i18n `error.product.not.exist`）。
pub const MSG_PRODUCT_NOT_EXIST: &str = "商品不存在";
/// `代客下单不支持该商品类型`（控制器 `validateProductTypeForProxyShopping`）。
pub const MSG_PRODUCT_TYPE_UNSUPPORTED: &str = "代客下单不支持该商品类型";
/// `购物车内已有商品不可与此商品同时加入购物车，请分开结算`
/// （i18n `error.cart.product.conflict`）。
pub const MSG_CART_CONFLICT: &str = "购物车内已有商品不可与此商品同时加入购物车，请分开结算";
/// `购物车不存在此商品`（i18n `error.cart.item.not.exist`）。
pub const MSG_CART_ITEM_NOT_EXIST: &str = "购物车不存在此商品";
/// `请选择购买数量`（`Assert.notNull(quantity, ...)`）。
pub const MSG_QUANTITY_REQUIRED: &str = "请选择购买数量";
/// `商品数量错误！`（i18n `error.product.quantity.invalid`）。
pub const MSG_QUANTITY_INVALID: &str = "商品数量错误！";
/// `商品库存不足`（i18n `error.product.stock.insufficient`）。
pub const MSG_STOCK_INSUFFICIENT: &str = "商品库存不足";
/// `该商品已下架！`（i18n `error.product.off.sale`）。
pub const MSG_OFF_SALE: &str = "该商品已下架！";
/// `商品规格已禁用，请重新选择`（`ProductSkuServiceImpl#checkSkuEnabled`）。
pub const MSG_SKU_DISABLED: &str = "商品规格已禁用，请重新选择";
/// `购物车ID不能为空`（`updateCheck`）。
pub const MSG_CART_ID_REQUIRED: &str = "购物车ID不能为空";
/// `购物车选中状态不正确`（`updateCheck`）。
pub const MSG_CHECK_INVALID: &str = "购物车选中状态不正确";

/// 代客下单购物车类型（Java `CartTypeEnum.TYPE_ADMIN_SHOPPING`）。
pub const CART_TYPE_ADMIN_SHOPPING: i32 = 15;
/// 代客下单支持的商品类型（Java `ADMIN_PROXY_SHOPPING_PRODUCT_TYPES`）：
/// 1 实物 / 2 虚拟 / 3 卡密 / 4 付费内容 / 7 实体礼品卡 / 8 电子礼品卡。
pub const ADMIN_PROXY_SHOPPING_PRODUCT_TYPES: [i32; 6] = [1, 2, 3, 4, 7, 8];
/// 单买商品类型（Java `isSingleBuy`）：2 虚拟 / 3 卡密 / 4 付费 / 7 实体礼品卡 / 8 电子礼品卡。
pub const SINGLE_BUY_PRODUCT_TYPES: [i32; 5] = [2, 3, 4, 7, 8];
/// 商品状态：上架（Java `ProductStatus.ON_SALE`）。
pub const PRODUCT_STATUS_ON_SALE: i32 = 1;

// ——————————————————————————————————————————————
// 纯函数
// ——————————————————————————————————————————————

/// 商品类型 → 购物车类型（Java `adminShopping2CartType`）：1→1、2→6、3→8、
/// 4→7（付费）、7/8→11（礼品卡）、其它→1。
#[must_use]
pub fn cart_type_from_product_type(product_type: i32) -> i32 {
    match product_type {
        2 => 6,
        3 => 8,
        4 => 7,
        7 | 8 => 11,
        _ => 1,
    }
}

/// 是否单买商品（Java `isSingleBuy`）。
#[must_use]
pub fn is_single_buy(product_type: i32) -> bool {
    SINGLE_BUY_PRODUCT_TYPES.contains(&product_type)
}

/// 是否代客下单支持的商品类型。
#[must_use]
pub fn is_proxy_supported(product_type: i32) -> bool {
    ADMIN_PROXY_SHOPPING_PRODUCT_TYPES.contains(&product_type)
}

/// 校验用户 id（Java `validateUserIdNotEmpty`）。
#[must_use]
pub fn validate_user_id(user_id: Option<i32>) -> Result<i32, String> {
    user_id
        .filter(|id| *id > 0)
        .ok_or_else(|| MSG_USER_REQUIRED.to_string())
}

/// 校验商品类型（Java `validateProductTypeForProxyShopping`；null 放行）。
#[must_use]
pub fn validate_product_type(product_type: Option<i32>) -> Result<(), String> {
    match product_type {
        None => Ok(()),
        Some(t) if is_proxy_supported(t) => Ok(()),
        Some(_) => Err(MSG_PRODUCT_TYPE_UNSUPPORTED.to_string()),
    }
}

/// 校验选中状态取值（Java `Constants.YES/NO` = 1/0）。
#[must_use]
pub fn validate_checked(value: Option<i32>) -> Result<i32, String> {
    match value {
        Some(1) => Ok(1),
        Some(0) => Ok(0),
        _ => Err(MSG_CHECK_INVALID.to_string()),
    }
}

/// `getCount` 汇总数量（Java `map(quantity).reduce(0, Integer::sum)`）。
#[must_use]
pub fn sum_quantity(quantities: impl IntoIterator<Item = i32>) -> i32 {
    quantities.into_iter().fold(0_i32, i32::saturating_add)
}

// ——————————————————————————————————————————————
// 入参
// ——————————————————————————————————————————————

/// 购物车列表查询（Java `ListDTO extends BaseChangePriceParam`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProxyCartListDTO {
    /// 用户 id（必填，控制器校验「请选择用户」）
    pub user_id: Option<i32>,
    /// 门店 id：Java `HeaderUtils.getShopId()` 覆盖请求体值（服务层用 header）
    pub shop_id: Option<i32>,
    /// 已勾选筛选（Java `isChecked`）
    pub is_checked: Option<i32>,
    /// 商品 id 筛选（Java `productId`）
    pub product_id: Option<i32>,
}

/// 加入购物车（Java `AddToCartDTO` 相关字段）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProxyAddToCartDTO {
    /// 商品 id（Java `id`）
    pub id: Option<i32>,
    /// 数量（Java `number`，缺省 1）
    pub number: Option<i32>,
    /// SKU id（缺省 0）
    pub sku_id: Option<i32>,
    /// 用户 id（代客下单必填）
    pub user_id: Option<i32>,
}

/// 清空购物车（Java `ProxyShoppingUserParam`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProxyClearCartDTO {
    pub user_id: Option<i32>,
}

/// 删除购物车商品（Java `RemoveItemDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProxyRemoveItemDTO {
    pub cart_ids: Option<Vec<i32>>,
}

/// 更新购物车数量（Java `UpdateItemDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProxyUpdateItemDTO {
    pub cart_id: Option<i32>,
    pub data: Option<ProxyUpdateItemData>,
}

/// Java `UpdateItemDTO.updateItemData`。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProxyUpdateItemData {
    pub quantity: Option<i32>,
}

/// 更新选中状态（Java `UpdateCheckDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProxyUpdateCheckDTO {
    pub user_id: Option<i32>,
    pub data: Option<Vec<ProxyUpdateCheckItem>>,
}

/// Java `UpdateCheckDTO.data` 的 Map 元素（cartId + isChecked）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProxyUpdateCheckItem {
    pub cart_id: Option<i32>,
    pub is_checked: Option<i32>,
}

/// `getCount` 查询参数（Java `@RequestParam shopId/userId`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProxyCartCountQuery {
    pub shop_id: Option<i32>,
    pub user_id: Option<i32>,
}

// ——————————————————————————————————————————————
// 出参
// ——————————————————————————————————————————————

/// 购物车商品行（Java `CartVO` 的 DB 可提供子集）。
///
/// ⚠️ 原版 admin 页面 `views/order/proxyShopping/Index.vue` 的「已选商品」表格
/// 直接读 `row.price` 与 `row.subtotal`：
/// ```text
/// 单价：{{ !row.price ? "-" : priceFormat(Number(row.price), false) }}
/// 小计：{{ !row.subtotal ? "-" : priceFormat(Number(row.subtotal), false) }}
/// ```
/// Java `CartVO` 也确实同时返回 `price` / `subtotal`（`buildCartPromotion`
/// 里 `cart.setPrice(...)` + `cart.setSubtotal(price * quantity)`）。
/// Rust 早期只返回 `originalPrice`，导致「已选商品」两列恒为 `-`
/// （2026-10-21 代客下单页显示空的问题之一）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CartItemVO {
    pub cart_id: i32,
    pub user_id: i32,
    pub product_id: i32,
    pub sku_id: Option<i32>,
    pub quantity: i32,
    pub is_checked: i32,
    pub product_sn: String,
    pub pic_thumb: Option<String>,
    pub market_price: Option<Decimal>,
    pub original_price: Decimal,
    /// 当前单价（Java `CartVO.price`）：本实现取 DB 快照价（价格/活动引擎未迁移）。
    pub price: Decimal,
    /// 行小计（Java `CartVO.subtotal` = `price * quantity`）。
    pub subtotal: Decimal,
    pub product_type: i32,
    pub shop_id: i32,
    pub sku_data: Option<String>,
    pub update_time: i64,
    /// 商品名（联表 product；未迁移价格/活动引擎，仅展示 DB 值）
    pub product_name: Option<String>,
    pub product_status: Option<i32>,
    pub product_stock: Option<i32>,
}

/// 门店购物车分组（Java `CartByStoreVO` 的 DB 可提供子集）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CartByStoreVO {
    pub shop_id: i32,
    pub shop_title: Option<String>,
    pub carts: Vec<CartItemVO>,
}

/// 购物车合计（Java `CartListByStoreVO.CartTotal`）。
///
/// 优惠/活动/改价类字段**未迁移计算引擎**，统一返回 `null`（不编造数值）；
/// `productAmount` 为 DB 快照价小计（`sum(original_price * quantity)`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CartTotalVO {
    pub product_amount: Decimal,
    pub total_count: i64,
    pub checked_count: i64,
    pub discounts: Option<Decimal>,
    pub discount_after: Option<Decimal>,
    pub discount_change_price: Option<Decimal>,
    pub discount_coupon_amount: Option<Decimal>,
    pub discount_seckill_amount: Option<Decimal>,
    pub discount_product_promotion_amount: Option<Decimal>,
    pub discount_time_discount_amount: Option<Decimal>,
    pub discount_group_buy_amount: Option<Decimal>,
    pub discount_discount_amount: Option<Decimal>,
    pub round_off_cents_amount: Option<Decimal>,
    pub round_off_dime_amount: Option<Decimal>,
    pub service_fee: Option<Decimal>,
    pub pos_other_discount_amount: Option<Decimal>,
}

/// 购物车列表（Java `CartListByStoreVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CartListByStoreVO {
    pub total: CartTotalVO,
    pub has_change_price: bool,
    pub cart_list: Vec<CartByStoreVO>,
    pub is_wholesale_buyer: bool,
}

/// 加入购物车返回（Java `ExchangeProductToCartVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExchangeProductToCartVO {
    pub item: bool,
    /// 代客下单 flowType = 15
    pub flow_type: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validations_match_java_messages() {
        assert_eq!(validate_user_id(None).unwrap_err(), MSG_USER_REQUIRED);
        assert_eq!(validate_user_id(Some(0)).unwrap_err(), MSG_USER_REQUIRED);
        assert_eq!(validate_user_id(Some(3)).unwrap(), 3);
        assert!(validate_product_type(None).is_ok());
        assert!(validate_product_type(Some(1)).is_ok());
        assert_eq!(
            validate_product_type(Some(5)).unwrap_err(),
            MSG_PRODUCT_TYPE_UNSUPPORTED
        );
        assert_eq!(validate_checked(Some(1)).unwrap(), 1);
        assert_eq!(validate_checked(Some(0)).unwrap(), 0);
        assert_eq!(validate_checked(Some(2)).unwrap_err(), MSG_CHECK_INVALID);
        assert_eq!(validate_checked(None).unwrap_err(), MSG_CHECK_INVALID);
    }

    #[test]
    fn cart_type_mapping_matches_java_switch() {
        assert_eq!(cart_type_from_product_type(1), 1);
        assert_eq!(cart_type_from_product_type(2), 6);
        assert_eq!(cart_type_from_product_type(3), 8);
        assert_eq!(cart_type_from_product_type(4), 7);
        assert_eq!(cart_type_from_product_type(7), 11);
        assert_eq!(cart_type_from_product_type(8), 11);
        assert_eq!(cart_type_from_product_type(99), 1);
    }

    #[test]
    fn single_buy_and_supported_types() {
        assert!(is_single_buy(2));
        assert!(is_single_buy(3));
        assert!(is_single_buy(4));
        assert!(is_single_buy(7));
        assert!(is_single_buy(8));
        assert!(!is_single_buy(1));
        assert!(is_proxy_supported(1));
        assert!(is_proxy_supported(8));
        assert!(!is_proxy_supported(5));
        assert_eq!(sum_quantity(vec![1, 2, 3]), 6);
        assert_eq!(sum_quantity(Vec::<i32>::new()), 0);
    }

    #[test]
    fn dtos_accept_java_names() {
        let dto: ProxyAddToCartDTO =
            serde_json::from_str(r#"{"id":7,"number":2,"skuId":9,"userId":3}"#).unwrap();
        assert_eq!(dto.id, Some(7));
        assert_eq!(dto.sku_id, Some(9));
        let dto: ProxyUpdateCheckDTO =
            serde_json::from_str(r#"{"userId":3,"data":[{"cartId":1,"isChecked":1}]}"#).unwrap();
        assert_eq!(dto.user_id, Some(3));
        assert_eq!(dto.data.unwrap()[0].is_checked, Some(1));
        let q: ProxyCartCountQuery = serde_json::from_str(r#"{"shopId":2,"userId":3}"#).unwrap();
        assert_eq!(q.user_id, Some(3));
    }

    #[test]
    fn vo_serializes_camel_case_with_nulls_for_unmigrated() {
        let vo = CartListByStoreVO {
            total: CartTotalVO {
                product_amount: Decimal::new(100, 0),
                total_count: 2,
                checked_count: 1,
                ..Default::default()
            },
            cart_list: vec![CartByStoreVO {
                shop_id: 1,
                shop_title: Some("店".into()),
                carts: vec![CartItemVO {
                    cart_id: 1,
                    quantity: 2,
                    ..Default::default()
                }],
            }],
            ..Default::default()
        };
        let value = serde_json::to_value(&vo).unwrap();
        assert_eq!(value["total"]["productAmount"], "100");
        assert_eq!(value["total"]["totalCount"], 2);
        assert!(value["total"]["discounts"].is_null());
        assert_eq!(value["cartList"][0]["shopTitle"], "店");
        assert_eq!(value["cartList"][0]["carts"][0]["quantity"], 2);
        let add = ExchangeProductToCartVO {
            item: true,
            flow_type: CART_TYPE_ADMIN_SHOPPING,
        };
        let value = serde_json::to_value(&add).unwrap();
        assert_eq!(value["item"], true);
        assert_eq!(value["flowType"], 15);
    }
}
