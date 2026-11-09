//! content 领域服务。

pub mod api_wechat_live;
pub mod article;
pub mod article_batch;
pub mod article_category;
pub mod decorate;
pub mod decorate_promote;
pub mod decorate_template;
pub mod friend_links;
pub mod gallery;
pub mod gallery_pic;
pub mod gallery_video;
pub mod gallery_video_info;
pub mod home;
pub mod mobile_cat_nav;
pub mod pc_cat_floor;
pub mod pc_navigation;
pub mod seo;
pub mod share;
pub mod tenant_scope;
pub mod wechat_auto_reply;
pub mod wechat_kefu_capability;
pub mod wechat_keyword_rule;
pub mod wechat_client;
pub mod wechat_live;
pub mod wechat_live_goods;
pub mod wechat_live_order_sync;
pub mod volcengine_translate;
pub mod wechat_live_sync_callback;
pub mod wechat_menu;
pub mod wechat_video_account;

use std::sync::Arc;

/// content 领域服务集合（由组合根装配一次，api 层只持有 `Arc<Services>`）。
#[derive(Clone)]
pub struct Services {
    pub article: Arc<article::ContentService>,
    pub decorate: Arc<decorate::DecorateService>,
    pub mobile_cat_nav: Arc<mobile_cat_nav::MobileCatNavService>,
    pub pc_cat_floor: Arc<pc_cat_floor::PcCatFloorService>,
    pub pc_navigation: Arc<pc_navigation::PcNavigationService>,
    pub seo: Arc<seo::SeoService>,
    pub share: Arc<share::ShareService>,
    pub wechat_live_sync_callback: Arc<wechat_live_sync_callback::WechatLiveSyncCallbackService>,
    pub home: Arc<home::HomeService>,
    pub wechat_live: Arc<wechat_live::WechatLiveService>,
    pub gallery: Arc<gallery::GalleryService>,
    pub gallery_pic: Arc<gallery_pic::GalleryPicService>,
    pub friend_links: Arc<friend_links::FriendLinksService>,
    pub gallery_video: Arc<gallery_video::GalleryVideoService>,
    pub wechat_auto_reply: Arc<wechat_auto_reply::WechatAutoReplyService>,
    pub article_batch: Arc<article_batch::ArticleBatchService>,
    pub article_category: Arc<article_category::ArticleCategoryService>,
    pub decorate_template: Arc<decorate_template::DecorateTemplateService>,
    pub wechat_live_goods: Arc<wechat_live_goods::WechatLiveGoodsService>,
    pub wechat_live_order_sync: Arc<wechat_live_order_sync::WechatLiveOrderSyncService>,
    pub api_wechat_live: Arc<api_wechat_live::ApiWechatLiveService>,
    pub decorate_promote: Arc<decorate_promote::DecoratePromoteService>,
    pub gallery_video_info: Arc<gallery_video_info::GalleryVideoInfoService>,
    pub wechat_video_account: Arc<wechat_video_account::WechatVideoAccountService>,
    pub wechat_keyword_rule: Arc<wechat_keyword_rule::WechatKeywordRuleService>,
    pub wechat_menu: Arc<wechat_menu::WechatMenuService>,
    pub wechat_kefu_capability: Arc<wechat_kefu_capability::WechatKefuCapabilityService>,
}
