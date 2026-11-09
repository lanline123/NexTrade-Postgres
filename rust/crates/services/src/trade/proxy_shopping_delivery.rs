//! 代客下单配送方式域服务（trade::proxy_shopping_delivery）。
//!
//! 两个端点均先执行 Java 校验（「请选择用户」等），再返回明确「未迁移」业务错误：
//! O2O 配送距离计算 / 运费模板引擎尚未迁移，不编造配送数据。

use std::sync::Arc;

use nextrade_core::models::trade::proxy_shopping_delivery::{
    validate_user_id, DeliveryOptionVO, DeliveryOptionsQuery, ProxyStoreShippingTypeQuery,
    ShippingTypeVO, CART_TYPE_ADMIN_SHOPPING,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::trade::proxy_shopping_delivery::ProxyShoppingDeliveryRepository;
use nextrade_infrastructure::db::Repositories;

/// 代客下单配送方式服务。
pub struct ProxyShoppingDeliveryService {
    repos: Arc<Repositories>,
}

impl ProxyShoppingDeliveryService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本域仓储（NESTED：`self.repos.trade.proxy_shopping_delivery`）。
    #[must_use]
    pub fn repo(&self) -> &ProxyShoppingDeliveryRepository {
        &self.repos.trade.proxy_shopping_delivery
    }

    /// 配送选项（对应 Java `ProxyShoppingDeliveryController#getDeliveryOptions`）。
    pub async fn get_delivery_options(
        &self,
        query: &DeliveryOptionsQuery,
        shop_id: Option<i32>,
    ) -> Result<DeliveryOptionVO> {
        let user_id = validate_user_id(query.user_id).map_err(Error::Validation)?;
        let items = self.repo().find_cart_items(user_id, shop_id).await?;
        if items.is_empty() {
            return Ok(DeliveryOptionVO::default());
        }

        let product_ids: Vec<i32> = items.iter().map(|i| i.product_id).collect();
        let products = self.repo().find_products_delivery_attr(&product_ids).await?;
        let product_map: std::collections::HashMap<i32, _> = products
            .into_iter()
            .map(|p| (p.product_id, p))
            .collect();

        let mut delivery_products = Vec::new();
        let mut shop_delivery_products = Vec::new();
        let mut no_shipping_products = Vec::new();

        for item in &items {
            let Some(p) = product_map.get(&item.product_id) else {
                continue;
            };
            if p.no_shipping == 1 && p.product_type != 1 {
                no_shipping_products.push(serde_json::json!({
                    "productId": p.product_id,
                    "productName": p.product_name,
                    "picThumb": p.pic_thumb,
                }));
            }
            if p.is_logistics == 1 {
                delivery_products.push(serde_json::json!({
                    "productId": p.product_id,
                    "productName": p.product_name,
                    "picThumb": p.pic_thumb,
                }));
            }
            if p.is_shop_delivery == 1 {
                shop_delivery_products.push(serde_json::json!({
                    "productId": p.product_id,
                    "productName": p.product_name,
                    "picThumb": p.pic_thumb,
                }));
            }
        }

        if !no_shipping_products.is_empty() {
            return Ok(DeliveryOptionVO {
                delivery_type: Some(3), // DELIVERY
                delivery_products: no_shipping_products,
                pickup_products: Vec::new(),
                shop_delivery_data: Vec::new(),
                shop_delivery_products: Vec::new(),
            });
        }

        let total = items.len();
        let delivery_count = delivery_products.len();
        let shop_delivery_count = shop_delivery_products.len();
        let pickup_count = 0;

        let delivery_type = compute_delivery_type(total, delivery_count, shop_delivery_count, pickup_count);

        Ok(DeliveryOptionVO {
            delivery_type: Some(delivery_type),
            delivery_products,
            pickup_products: Vec::new(),
            shop_delivery_data: Vec::new(),
            shop_delivery_products,
        })
    }

    /// 店铺配送方式（Java `getStoreShippingType`）。
    pub async fn get_store_shipping_type(
        &self,
        query: &ProxyStoreShippingTypeQuery,
        shop_id: Option<i32>,
    ) -> Result<Vec<(i32, Vec<ShippingTypeVO>)>> {
        let user_id = validate_user_id(query.user_id).map_err(Error::Validation)?;
        let effective_shop = query.shop_id.or(shop_id);
        let check_service = crate::trade::check::CheckService::new(self.repos.clone());
        let store_query = nextrade_core::models::trade::check::StoreShippingTypeQuery {
            address_id: query.address_id,
            flow_type: Some(CART_TYPE_ADMIN_SHOPPING),
            shop_id: effective_shop,
            user_id: Some(user_id),
            pay_type_id: query.pay_type_id,
            use_point: query.use_point,
            use_balance: query.use_balance,
            use_coupon_ids: query.use_coupon_ids.clone(),
            exclude_quick_product: Some(false),
        };
        let map = check_service.store_shipping_types(&store_query).await?;
        let mut list: Vec<(i32, Vec<ShippingTypeVO>)> = Vec::new();
        for (sid, types) in map {
            let vos: Vec<ShippingTypeVO> = types
                .into_iter()
                .map(|t| ShippingTypeVO {
                    shipping_type_id: t.shipping_type_id,
                    shipping_type_name: t.shipping_type_name.clone(),
                    origin_shipping_type_name: t.shipping_type_name,
                    shipping_default_id: Some(0),
                    shipping_type_desc: Some(String::new()),
                    shipping_time_desc: Some(String::new()),
                    is_support_cod: t.is_support_cod,
                })
                .collect();
            list.push((sid, vos));
        }
        list.sort_by_key(|(sid, _)| *sid);
        Ok(list)
    }
}

/// 计算配送模式（对应 Java `ShopDeliveryTypeEnum`）。
#[must_use]
pub fn compute_delivery_type(
    total: usize,
    delivery_count: usize,
    shop_delivery_count: usize,
    pickup_count: usize,
) -> i32 {
    if total == 0 {
        return 3;
    }
    if pickup_count == total && delivery_count == total && shop_delivery_count == total {
        1 // ALL_CHOOSE
    } else if pickup_count == 0 && delivery_count == total && shop_delivery_count == total {
        1 // ALL_CHOOSE
    } else if delivery_count == total && pickup_count == 0 && shop_delivery_count == 0 {
        3 // DELIVERY
    } else if shop_delivery_count == total && pickup_count == 0 && delivery_count == 0 {
        5 // CITY
    } else if pickup_count == total && delivery_count == 0 && shop_delivery_count == 0 {
        4 // SELF_PICK_UP
    } else {
        2 // PART_CHOOSE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_validation_reused() {
        assert!(validate_user_id(None).is_err());
    }

    #[test]
    fn test_compute_delivery_type() {
        assert_eq!(compute_delivery_type(2, 2, 0, 0), 3); // DELIVERY
        assert_eq!(compute_delivery_type(2, 0, 2, 0), 5); // CITY
        assert_eq!(compute_delivery_type(2, 0, 0, 2), 4); // SELF_PICK_UP
        assert_eq!(compute_delivery_type(2, 2, 2, 0), 1); // ALL_CHOOSE
        assert_eq!(compute_delivery_type(2, 2, 2, 2), 1); // ALL_CHOOSE
        assert_eq!(compute_delivery_type(2, 1, 0, 0), 2); // PART_CHOOSE
    }
}
