//! merchant 领域服务。

pub mod admin_user_shop;
pub mod deposit_account;
pub mod deposit_log;
pub mod deposit_payee_account;
pub mod deposit_recharge;
pub mod deposit_refund;
pub mod deposit_rule;
pub mod deposit_setting;
pub mod deposit_shop;
pub mod merchant;
pub mod merchant_account;
mod default_decorate;
pub mod merchant_apply;
pub mod merchant_client;
pub mod order_config;
pub mod shop;
pub mod shop_account_log;
pub mod shop_client;
pub mod shop_client_category;
pub mod shop_funds;
pub mod shop_manage;
pub mod shop_product_category;
pub mod shop_withdraw;
pub mod vendor_deposit_shop;

use std::sync::Arc;

/// merchant 领域服务集合（由组合根装配一次，api 层只持有 `Arc<Services>`）。
#[derive(Clone)]
pub struct Services {
    pub merchant_client: Arc<merchant_client::MerchantClientService>,
    pub order_config: Arc<order_config::OrderConfigService>,
    pub shop: Arc<shop::ShopService>,
    pub shop_client: Arc<shop_client::ShopClientService>,
    pub shop_client_category: Arc<shop_client_category::ShopClientCategoryService>,
    pub shop_manage: Arc<shop_manage::ShopManageService>,
    pub shop_product_category: Arc<shop_product_category::ShopProductCategoryService>,
    pub vendor_deposit_shop: Arc<vendor_deposit_shop::VendorDepositShopService>,
    pub merchant_apply: Arc<merchant_apply::MerchantApplyService>,
    pub deposit_account: Arc<deposit_account::DepositAccountService>,
    pub deposit_recharge: Arc<deposit_recharge::DepositRechargeService>,
    pub deposit_refund: Arc<deposit_refund::DepositRefundService>,
    pub merchant: Arc<merchant::MerchantService>,
    pub shop_funds: Arc<shop_funds::ShopFundsService>,
    pub admin_user_shop: Arc<admin_user_shop::AdminUserShopService>,
    pub merchant_account: Arc<merchant_account::MerchantAccountService>,
    pub shop_withdraw: Arc<shop_withdraw::ShopWithdrawService>,
    pub deposit_rule: Arc<deposit_rule::DepositRuleService>,
    pub deposit_shop: Arc<deposit_shop::DepositShopService>,
    pub deposit_log: Arc<deposit_log::DepositLogService>,
    pub deposit_payee_account: Arc<deposit_payee_account::DepositPayeeAccountService>,
    pub deposit_setting: Arc<deposit_setting::DepositSettingService>,
    pub shop_account_log: Arc<shop_account_log::ShopAccountLogService>,
}
