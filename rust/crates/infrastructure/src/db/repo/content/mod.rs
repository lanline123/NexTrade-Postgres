//! content 领域仓储。

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
pub mod wechat_auto_reply;
pub mod wechat_kefu_capability;
pub mod wechat_keyword_rule;
pub mod wechat_live;
pub mod wechat_live_goods;
pub mod wechat_live_order_sync;
pub mod wechat_live_sync_callback;
pub mod wechat_menu;
pub mod wechat_video_account;

use crate::db::DbPool;

#[derive(Clone)]
pub struct Repositories {
    pub article: article::ArticleRepository,
    pub decorate: decorate::DecorateRepository,
    pub mobile_cat_nav: mobile_cat_nav::MobileCatNavRepository,
    pub pc_cat_floor: pc_cat_floor::PcCatFloorRepository,
    pub pc_navigation: pc_navigation::PcNavigationRepository,
    pub seo: seo::SeoRepository,
    pub share: share::ShareRepository,
    pub wechat_live_sync_callback: wechat_live_sync_callback::WechatLiveSyncCallbackRepository,
    pub home: home::HomeRepository,
    pub wechat_live: wechat_live::WechatLiveRepository,
    pub gallery: gallery::GalleryRepository,
    pub gallery_pic: gallery_pic::GalleryPicRepository,
    pub friend_links: friend_links::FriendLinksRepository,
    pub gallery_video: gallery_video::GalleryVideoRepository,
    pub wechat_auto_reply: wechat_auto_reply::WechatAutoReplyRepository,
    pub article_batch: article_batch::ArticleBatchRepository,
    pub article_category: article_category::ArticleCategoryRepository,
    pub decorate_template: decorate_template::DecorateTemplateRepository,
    pub wechat_live_goods: wechat_live_goods::WechatLiveGoodsRepository,
    pub wechat_live_order_sync: wechat_live_order_sync::WechatLiveOrderSyncRepository,
    pub api_wechat_live: api_wechat_live::ApiWechatLiveRepository,
    pub decorate_promote: decorate_promote::DecoratePromoteRepository,
    pub gallery_video_info: gallery_video_info::GalleryVideoInfoRepository,
    pub wechat_video_account: wechat_video_account::WechatVideoAccountRepository,
    pub wechat_keyword_rule: wechat_keyword_rule::WechatKeywordRuleRepository,
    pub wechat_menu: wechat_menu::WechatMenuRepository,
    pub wechat_kefu_capability: wechat_kefu_capability::WechatKefuCapabilityRepository,
}

impl Repositories {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self {
            article: article::ArticleRepository::new(pool.clone()),
            decorate: decorate::DecorateRepository::new(pool.clone()),
            mobile_cat_nav: mobile_cat_nav::MobileCatNavRepository::new(pool.clone()),
            pc_cat_floor: pc_cat_floor::PcCatFloorRepository::new(pool.clone()),
            pc_navigation: pc_navigation::PcNavigationRepository::new(pool.clone()),
            seo: seo::SeoRepository::new(pool.clone()),
            share: share::ShareRepository::new(pool.clone()),
            wechat_live_sync_callback: wechat_live_sync_callback::WechatLiveSyncCallbackRepository::new(pool.clone()),
            home: home::HomeRepository::new(pool.clone()),
            wechat_live: wechat_live::WechatLiveRepository::new(pool.clone()),
            gallery: gallery::GalleryRepository::new(pool.clone()),
            gallery_pic: gallery_pic::GalleryPicRepository::new(pool.clone()),
            friend_links: friend_links::FriendLinksRepository::new(pool.clone()),
            gallery_video: gallery_video::GalleryVideoRepository::new(pool.clone()),
            wechat_auto_reply: wechat_auto_reply::WechatAutoReplyRepository::new(pool.clone()),
            article_batch: article_batch::ArticleBatchRepository::new(pool.clone()),
            article_category: article_category::ArticleCategoryRepository::new(pool.clone()),
            decorate_template: decorate_template::DecorateTemplateRepository::new(pool.clone()),
            wechat_live_goods: wechat_live_goods::WechatLiveGoodsRepository::new(pool.clone()),
            wechat_live_order_sync: wechat_live_order_sync::WechatLiveOrderSyncRepository::new(pool.clone()),
            api_wechat_live: api_wechat_live::ApiWechatLiveRepository::new(pool.clone()),
            decorate_promote: decorate_promote::DecoratePromoteRepository::new(pool.clone()),
            gallery_video_info: gallery_video_info::GalleryVideoInfoRepository::new(pool.clone()),
            wechat_video_account: wechat_video_account::WechatVideoAccountRepository::new(pool.clone()),
            wechat_keyword_rule: wechat_keyword_rule::WechatKeywordRuleRepository::new(pool.clone()),
            wechat_menu: wechat_menu::WechatMenuRepository::new(pool.clone()),
            wechat_kefu_capability: wechat_kefu_capability::WechatKefuCapabilityRepository::new(pool.clone()),
        }
    }
}
