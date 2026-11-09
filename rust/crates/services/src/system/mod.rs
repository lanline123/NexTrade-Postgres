//! system 领域服务。

pub mod admin_msg;
pub mod app_version_admin;
pub mod area_code;
pub mod cache_manage;
pub mod config_basic;
pub mod config_client;
pub mod csrf;
pub mod example;
pub mod captcha;
pub mod excel;
pub mod files;
pub mod locales;
pub mod log;
pub mod mail_templates;
pub mod message_type;
pub mod pc;
pub mod recommend;
pub mod region;
pub mod notify;
pub mod scheduler;
pub mod region_client;
pub mod tips_manage;
pub mod util;

use std::sync::Arc;

/// system 领域服务集合（由组合根装配一次，api 层只持有 `Arc<Services>`）。
#[derive(Clone)]
pub struct Services {
    pub app_version_admin: Arc<app_version_admin::AppVersionAdminService>,
    pub area_code: Arc<area_code::AreaCodeService>,
    pub cache_manage: Arc<cache_manage::CacheManageService>,
    pub config_client: Arc<config_client::ConfigClientService>,
    pub example: Arc<example::ExampleService>,
    pub files: Arc<files::FilesService>,
    pub mail_templates: Arc<mail_templates::MailTemplatesService>,
    pub message_type: Arc<message_type::MessageTypeService>,
    pub pc: Arc<pc::PcService>,
    pub region: Arc<region::RegionService>,
    pub region_client: Arc<region_client::RegionClientService>,
    pub csrf: Arc<csrf::CsrfService>,
    pub config_basic: Arc<config_basic::ConfigBasicService>,
    pub tips_manage: Arc<tips_manage::TipsManageService>,
    pub locales: Arc<locales::LocalesService>,
    pub log: Arc<log::LogService>,
    pub util: Arc<util::UtilService>,
    pub admin_msg: Arc<admin_msg::AdminMsgService>,
    pub recommend: Arc<recommend::RecommendService>,
}
