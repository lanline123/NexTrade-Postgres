//! member 领域服务。

pub mod captcha_api;
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
pub mod wechat_mp;

use std::sync::Arc;

/// member 领域服务集合（由组合根装配一次，api 层只持有 `Arc<Services>`）。
#[derive(Clone)]
pub struct Services {
    pub feedback: Arc<feedback::FeedbackService>,
    pub member_lists: Arc<member_lists::MemberService>,
    pub member_tag: Arc<member_tag::MemberTagService>,
    pub member_tag_group: Arc<member_tag_group::MemberTagGroupService>,
    pub user: Arc<user::UserService>,
    pub user_center: Arc<user_center::UserCenterService>,
    pub user_company: Arc<user_company::UserCompanyService>,
    pub user_manage: Arc<user_manage::UserManageService>,
    pub user_points_log: Arc<user_points_log::UserPointsLogService>,
    pub user_rank: Arc<user_rank::UserRankService>,
    pub login: Arc<login::LoginService>,
    pub shop_member: Arc<shop_member::ShopMemberService>,
    pub user_address_client: Arc<user_address_client::UserAddressClientService>,
    pub user_message: Arc<user_message::UserMessageService>,
    pub user_member_tag: Arc<user_member_tag::UserMemberTagService>,
    pub user_message_log: Arc<user_message_log::UserMessageLogService>,
    pub captcha_api: Arc<captcha_api::CaptchaService>,
    pub collect_product: Arc<collect_product::CollectProductService>,
    pub user_company_client: Arc<user_company_client::UserCompanyClientService>,
    pub user_invoice_client: Arc<user_invoice_client::UserInvoiceClientService>,
    pub user_price_inquiry: Arc<user_price_inquiry::UserPriceInquiryService>,
    pub member_audience: Arc<member_audience::MemberAudienceService>,
    pub user_feedback_client: Arc<user_feedback_client::UserFeedbackClientService>,
    pub user_sign: Arc<user_sign::UserSignService>,
}
