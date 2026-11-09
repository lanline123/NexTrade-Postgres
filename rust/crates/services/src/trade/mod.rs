//! trade 领域服务。

pub mod aftersales;
pub mod aftersales_client;
pub mod cart;
pub mod groupon;
pub mod cart_summary;
pub mod check;
pub mod order;
pub mod order_admin;
pub mod order_invoice_client;
pub mod order_log;
pub mod proxy_shopping_cart;
pub mod proxy_shopping_delivery;
pub mod proxy_shopping_order_check;
pub mod proxy_shopping_pickup_contact;
pub mod proxy_shopping_product;
pub mod proxy_shopping_user;
pub mod proxy_shopping_user_address;
pub mod proxy_shopping_user_coupon;
pub mod settlement;
pub mod shop_order_config;
pub mod user_order;

use std::sync::Arc;

/// trade 领域服务集合（由组合根装配一次，api 层只持有 `Arc<Services>`）。
#[derive(Clone)]
pub struct Services {
    pub aftersales: Arc<aftersales::AftersalesService>,
    pub aftersales_client: Arc<aftersales_client::AftersalesClientService>,
    pub cart: Arc<cart::CartService>,
    pub order: Arc<order::OrderService>,
    pub order_admin: Arc<order_admin::OrderAdminService>,
    pub proxy_shopping_order_check: Arc<proxy_shopping_order_check::ProxyShoppingOrderCheckService>,
    pub proxy_shopping_pickup_contact: Arc<proxy_shopping_pickup_contact::ProxyShoppingPickupContactService>,
    pub proxy_shopping_product: Arc<proxy_shopping_product::ProxyShoppingProductService>,
    pub proxy_shopping_user: Arc<proxy_shopping_user::ProxyShoppingUserService>,
    pub proxy_shopping_user_address: Arc<proxy_shopping_user_address::ProxyShoppingUserAddressService>,
    pub proxy_shopping_user_coupon: Arc<proxy_shopping_user_coupon::ProxyShoppingUserCouponService>,
    pub user_order: Arc<user_order::UserOrderService>,
    pub proxy_shopping_cart: Arc<proxy_shopping_cart::ProxyShoppingCartService>,
    pub check: Arc<check::CheckService>,
    pub order_log: Arc<order_log::OrderLogService>,
    pub order_invoice_client: Arc<order_invoice_client::OrderInvoiceClientService>,
    pub cart_summary: Arc<cart_summary::CartSummaryService>,
    pub proxy_shopping_delivery: Arc<proxy_shopping_delivery::ProxyShoppingDeliveryService>,
    pub shop_order_config: Arc<shop_order_config::ShopOrderConfigService>,
}
