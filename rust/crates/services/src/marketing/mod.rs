//! marketing 领域服务。

pub mod buyer_show;
pub mod buyer_show_client;
pub mod buyer_show_comment;
pub mod buyer_show_group;
pub mod coupon;
pub mod coupon_manage;
pub mod gift_card;
pub mod gift_card_groups;
pub mod groupon_promotion;
pub mod groupon_promotion_item;
pub mod groupon_promotion_record;
pub mod groupon_records;
pub mod lottery;
pub mod lottery_activity;
pub mod overview;
pub mod points_exchange;
pub mod product_gift;
pub mod product_promotion;
pub mod product_style_admin;
pub mod promotion;
pub mod recharge_setting;
pub mod salesman;
pub mod salesman_admin;
pub mod salesman_config;
pub mod salesman_content;
pub mod salesman_customer_transaction;
pub mod salesman_group;
pub mod salesman_material;
pub mod salesman_material_category;
pub mod salesman_order;
pub mod salesman_product;
pub mod seckill;
pub mod seckill_batch;
pub mod sign_in_setting;
pub mod time_discount;
pub mod time_discount_batch;

use std::sync::Arc;

/// marketing 领域服务集合（由组合根装配一次，api 层只持有 `Arc<Services>`）。
#[derive(Clone)]
pub struct Services {
    pub buyer_show: Arc<buyer_show::BuyerShowService>,
    pub buyer_show_client: Arc<buyer_show_client::BuyerShowClientService>,
    pub buyer_show_comment: Arc<buyer_show_comment::BuyerShowCommentService>,
    pub buyer_show_group: Arc<buyer_show_group::BuyerShowGroupService>,
    pub coupon: Arc<coupon::CouponService>,
    pub product_style_admin: Arc<product_style_admin::ProductStyleAdminService>,
    pub salesman: Arc<salesman::SalesmanService>,
    pub salesman_material_category: Arc<salesman_material_category::SalesmanMaterialCategoryService>,
    pub seckill: Arc<seckill::SeckillService>,
    pub time_discount: Arc<time_discount::TimeDiscountService>,
    pub groupon_promotion: Arc<groupon_promotion::GrouponPromotionService>,
    pub coupon_manage: Arc<coupon_manage::CouponManageService>,
    pub salesman_content: Arc<salesman_content::SalesmanContentService>,
    pub gift_card: Arc<gift_card::GiftCardService>,
    pub gift_card_groups: Arc<gift_card_groups::GiftCardGroupsService>,
    pub salesman_admin: Arc<salesman_admin::SalesmanAdminService>,
    pub lottery_activity: Arc<lottery_activity::LotteryActivityService>,
    pub salesman_product: Arc<salesman_product::SalesmanProductService>,
    pub product_promotion: Arc<product_promotion::ProductPromotionService>,
    pub groupon_promotion_item: Arc<groupon_promotion_item::GrouponPromotionItemService>,
    pub time_discount_batch: Arc<time_discount_batch::TimeDiscountBatchService>,
    pub points_exchange: Arc<points_exchange::PointsExchangeService>,
    pub product_gift: Arc<product_gift::ProductGiftService>,
    pub recharge_setting: Arc<recharge_setting::RechargeSettingService>,
    pub sign_in_setting: Arc<sign_in_setting::SignInSettingService>,
    pub salesman_material: Arc<salesman_material::SalesmanMaterialService>,
    pub salesman_group: Arc<salesman_group::SalesmanGroupService>,
    pub salesman_config: Arc<salesman_config::SalesmanConfigService>,
    pub salesman_order: Arc<salesman_order::SalesmanOrderService>,
    pub salesman_customer_transaction: Arc<salesman_customer_transaction::SalesmanCustomerTransactionService>,
    pub lottery: Arc<lottery::LotteryService>,
    pub seckill_batch: Arc<seckill_batch::SeckillBatchService>,
    pub promotion: Arc<promotion::PromotionService>,
    pub groupon_promotion_record: Arc<groupon_promotion_record::GrouponPromotionRecordService>,
    pub groupon_records: Arc<groupon_records::GrouponRecordsService>,
    pub overview: Arc<overview::OverviewService>,
}
