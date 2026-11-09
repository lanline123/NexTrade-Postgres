//! system 领域仓储。

pub mod admin_msg;
pub mod app_version_admin;
pub mod area_code;
pub mod cache_manage;
pub mod config_basic;
pub mod config_client;
pub mod example;
pub mod files;
pub mod locales;
pub mod log;
pub mod mail_templates;
pub mod message_type;
pub mod pc;
pub mod recommend;
pub mod region;
pub mod region_client;
pub mod tips_manage;
pub mod util;

use crate::db::DbPool;

#[derive(Clone)]
pub struct Repositories {
    pub app_version_admin: app_version_admin::AppVersionAdminRepository,
    pub area_code: area_code::AreaCodeRepository,
    pub cache_manage: cache_manage::CacheManageRepository,
    pub config_basic: config_basic::ConfigBasicRepository,
    pub config_client: config_client::ConfigClientRepository,
    pub example: example::ExampleRepository,
    pub files: files::FilesRepository,
    pub locales: locales::LocalesRepository,
    pub log: log::LogRepository,
    pub mail_templates: mail_templates::MailTemplatesRepository,
    pub message_type: message_type::MessageTypeRepository,
    pub pc: pc::PcRepository,
    pub region: region::RegionRepository,
    pub region_client: region_client::RegionClientRepository,
    pub tips_manage: tips_manage::TipsManageRepository,
    pub util: util::UtilRepository,
    pub admin_msg: admin_msg::AdminMsgRepository,
    pub recommend: recommend::RecommendRepository,
}

impl Repositories {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self {
            app_version_admin: app_version_admin::AppVersionAdminRepository::new(pool.clone()),
            area_code: area_code::AreaCodeRepository::new(pool.clone()),
            cache_manage: cache_manage::CacheManageRepository::new(pool.clone()),
            config_basic: config_basic::ConfigBasicRepository::new(pool.clone()),
            config_client: config_client::ConfigClientRepository::new(pool.clone()),
            example: example::ExampleRepository::new(pool.clone()),
            files: files::FilesRepository::new(pool.clone()),
            locales: locales::LocalesRepository::new(pool.clone()),
            log: log::LogRepository::new(pool.clone()),
            mail_templates: mail_templates::MailTemplatesRepository::new(pool.clone()),
            message_type: message_type::MessageTypeRepository::new(pool.clone()),
            pc: pc::PcRepository::new(pool.clone()),
            region: region::RegionRepository::new(pool.clone()),
            region_client: region_client::RegionClientRepository::new(pool.clone()),
            tips_manage: tips_manage::TipsManageRepository::new(pool.clone()),
            util: util::UtilRepository::new(pool.clone()),
            admin_msg: admin_msg::AdminMsgRepository::new(pool.clone()),
            recommend: recommend::RecommendRepository::new(pool.clone()),
        }
    }
}
