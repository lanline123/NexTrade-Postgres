//! member 领域仓储。

pub mod collect_product;
pub mod feedback;
pub mod login;
pub mod member_audience;
pub mod member_lists;
pub mod member_tag;
pub mod member_tag_group;
pub mod shop_member;
pub mod user;
pub mod user_address_client;
pub mod user_center;
pub mod user_company;
pub mod user_company_client;
pub mod user_feedback_client;
pub mod user_invoice_client;
pub mod user_manage;
pub mod user_member_tag;
pub mod user_message;
pub mod user_message_log;
pub mod user_points_log;
pub mod user_price_inquiry;
pub mod user_rank;
pub mod user_sign;

use crate::db::DbPool;

#[derive(Clone)]
pub struct Repositories {
    pub feedback: feedback::FeedbackRepository,
    pub login: login::LoginRepository,
    pub member_lists: member_lists::MemberRepository,
    pub member_tag: member_tag::MemberTagRepository,
    pub member_tag_group: member_tag_group::MemberTagGroupRepository,
    pub user: user::UserRepository,
    pub user_center: user_center::UserCenterRepository,
    pub user_company: user_company::UserCompanyRepository,
    pub user_manage: user_manage::UserManageRepository,
    pub user_points_log: user_points_log::UserPointsLogRepository,
    pub user_rank: user_rank::UserRankRepository,
    pub shop_member: shop_member::ShopMemberRepository,
    pub user_address_client: user_address_client::UserAddressRepository,
    pub user_message: user_message::UserMessageRepository,
    pub user_member_tag: user_member_tag::UserMemberTagRepository,
    pub user_message_log: user_message_log::UserMessageLogRepository,
    pub collect_product: collect_product::CollectProductRepository,
    pub user_company_client: user_company_client::UserCompanyClientRepository,
    pub user_invoice_client: user_invoice_client::UserInvoiceClientRepository,
    pub user_price_inquiry: user_price_inquiry::UserPriceInquiryRepository,
    pub member_audience: member_audience::MemberAudienceRepository,
    pub user_feedback_client: user_feedback_client::UserFeedbackClientRepository,
    pub user_sign: user_sign::UserSignRepository,
}

impl Repositories {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self {
            feedback: feedback::FeedbackRepository::new(pool.clone()),
            login: login::LoginRepository::new(pool.clone()),
            member_lists: member_lists::MemberRepository::new(pool.clone()),
            member_tag: member_tag::MemberTagRepository::new(pool.clone()),
            member_tag_group: member_tag_group::MemberTagGroupRepository::new(pool.clone()),
            user: user::UserRepository::new(pool.clone()),
            user_center: user_center::UserCenterRepository::new(pool.clone()),
            user_company: user_company::UserCompanyRepository::new(pool.clone()),
            user_manage: user_manage::UserManageRepository::new(pool.clone()),
            user_points_log: user_points_log::UserPointsLogRepository::new(pool.clone()),
            user_rank: user_rank::UserRankRepository::new(pool.clone()),
            shop_member: shop_member::ShopMemberRepository::new(pool.clone()),
            user_address_client: user_address_client::UserAddressRepository::new(pool.clone()),
            user_message: user_message::UserMessageRepository::new(pool.clone()),
            user_member_tag: user_member_tag::UserMemberTagRepository::new(pool.clone()),
            user_message_log: user_message_log::UserMessageLogRepository::new(pool.clone()),
            collect_product: collect_product::CollectProductRepository::new(pool.clone()),
            user_company_client: user_company_client::UserCompanyClientRepository::new(pool.clone()),
            user_invoice_client: user_invoice_client::UserInvoiceClientRepository::new(pool.clone()),
            user_price_inquiry: user_price_inquiry::UserPriceInquiryRepository::new(pool.clone()),
            member_audience: member_audience::MemberAudienceRepository::new(pool.clone()),
            user_feedback_client: user_feedback_client::UserFeedbackClientRepository::new(pool.clone()),
            user_sign: user_sign::UserSignRepository::new(pool.clone()),
        }
    }
}
