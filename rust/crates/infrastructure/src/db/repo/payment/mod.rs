//! payment 领域仓储。

pub mod account_panel;
pub mod apple_pay;
pub mod apple_pay_config;
pub mod category_fee_rule;
pub mod credit_order;
pub mod credit_settlement;
pub mod credit_user;
pub mod huifu_config;
pub mod lakala_config;
pub mod merchant_application;
pub mod order_invoice;
pub mod pay;
pub mod paylog;
pub mod paylog_batch;
pub mod profit_sharing;
pub mod recharge_order;
pub mod refund_apply;
pub mod refund_apply_admin;
pub mod refund_log;
pub mod statement;
pub mod sub_merchant;
pub mod tax_rate;
pub mod user_balance_log;
pub mod user_invoice;
pub mod user_recharge_order;
pub mod user_withdraw_apply;
pub mod with_draw_apply_client;
pub mod yeepay_config;

use crate::db::DbPool;

#[derive(Clone)]
pub struct Repositories {
    pub paylog: paylog::PaylogRepository,
    pub paylog_batch: paylog_batch::PaylogBatchRepository,
    pub refund_apply: refund_apply::RefundApplyRepository,
    pub tax_rate: tax_rate::TaxRateRepository,
    pub user_balance_log: user_balance_log::UserBalanceLogRepository,
    pub merchant_application: merchant_application::MerchantApplicationRepository,
    pub category_fee_rule: category_fee_rule::CategoryFeeRuleRepository,
    pub order_invoice: order_invoice::OrderInvoiceRepository,
    pub with_draw_apply_client: with_draw_apply_client::WithDrawApplyClientRepository,
    pub refund_log: refund_log::RefundLogRepository,
    pub credit_settlement: credit_settlement::CreditSettlementRepository,
    pub user_invoice: user_invoice::UserInvoiceRepository,
    pub credit_user: credit_user::CreditUserRepository,
    pub user_recharge_order: user_recharge_order::UserRechargeOrderRepository,
    pub user_withdraw_apply: user_withdraw_apply::UserWithdrawApplyRepository,
    pub pay: pay::PayRepository,
    pub sub_merchant: sub_merchant::SubMerchantRepository,
    pub profit_sharing: profit_sharing::ProfitSharingRepository,
    pub recharge_order: recharge_order::RechargeOrderRepository,
    pub statement: statement::StatementRepository,
    pub apple_pay: apple_pay::ApplePayRepository,
    pub apple_pay_config: apple_pay_config::ApplePayConfigRepository,
    pub huifu_config: huifu_config::HuifuConfigRepository,
    pub lakala_config: lakala_config::LakalaConfigRepository,
    pub yeepay_config: yeepay_config::YeepayConfigRepository,
    pub refund_apply_admin: refund_apply_admin::RefundApplyAdminRepository,
    pub account_panel: account_panel::AccountPanelRepository,
    pub credit_order: credit_order::CreditOrderRepository,
}

impl Repositories {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self {
            paylog: paylog::PaylogRepository::new(pool.clone()),
            paylog_batch: paylog_batch::PaylogBatchRepository::new(pool.clone()),
            refund_apply: refund_apply::RefundApplyRepository::new(pool.clone()),
            tax_rate: tax_rate::TaxRateRepository::new(pool.clone()),
            user_balance_log: user_balance_log::UserBalanceLogRepository::new(pool.clone()),
            merchant_application: merchant_application::MerchantApplicationRepository::new(pool.clone()),
            category_fee_rule: category_fee_rule::CategoryFeeRuleRepository::new(pool.clone()),
            order_invoice: order_invoice::OrderInvoiceRepository::new(pool.clone()),
            with_draw_apply_client: with_draw_apply_client::WithDrawApplyClientRepository::new(pool.clone()),
            refund_log: refund_log::RefundLogRepository::new(pool.clone()),
            credit_settlement: credit_settlement::CreditSettlementRepository::new(pool.clone()),
            user_invoice: user_invoice::UserInvoiceRepository::new(pool.clone()),
            credit_user: credit_user::CreditUserRepository::new(pool.clone()),
            user_recharge_order: user_recharge_order::UserRechargeOrderRepository::new(pool.clone()),
            user_withdraw_apply: user_withdraw_apply::UserWithdrawApplyRepository::new(pool.clone()),
            pay: pay::PayRepository::new(pool.clone()),
            sub_merchant: sub_merchant::SubMerchantRepository::new(pool.clone()),
            profit_sharing: profit_sharing::ProfitSharingRepository::new(pool.clone()),
            recharge_order: recharge_order::RechargeOrderRepository::new(pool.clone()),
            statement: statement::StatementRepository::new(pool.clone()),
            apple_pay: apple_pay::ApplePayRepository::new(pool.clone()),
            apple_pay_config: apple_pay_config::ApplePayConfigRepository::new(pool.clone()),
            huifu_config: huifu_config::HuifuConfigRepository::new(pool.clone()),
            lakala_config: lakala_config::LakalaConfigRepository::new(pool.clone()),
            yeepay_config: yeepay_config::YeepayConfigRepository::new(pool.clone()),
            refund_apply_admin: refund_apply_admin::RefundApplyAdminRepository::new(pool.clone()),
            account_panel: account_panel::AccountPanelRepository::new(pool.clone()),
            credit_order: credit_order::CreditOrderRepository::new(pool.clone()),
        }
    }
}
