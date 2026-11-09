//! logistics 领域服务。

pub mod kdniao;
pub mod kuaidi100;
pub mod logistics_company;
pub mod logistics_company_config;
pub mod print;
pub mod print_config;
pub mod print_log;
pub mod shipping_list;
pub mod shipping_tpl;
pub mod shipping_type;
pub mod sunmi_print_callback;

use std::sync::Arc;

/// logistics 领域服务集合（由组合根装配一次，api 层只持有 `Arc<Services>`）。
#[derive(Clone)]
pub struct Services {
    pub logistics_company: Arc<logistics_company::LogisticsCompanyService>,
    pub logistics_company_config: Arc<logistics_company_config::LogisticsCompanyConfigService>,
    pub print_config: Arc<print_config::PrintConfigService>,
    pub shipping_list: Arc<shipping_list::LogisticsService>,
    pub shipping_tpl: Arc<shipping_tpl::ShippingTplService>,
    pub shipping_type: Arc<shipping_type::ShippingTypeService>,
    pub sunmi_print_callback: Arc<sunmi_print_callback::SunmiPrintCallbackService>,
    pub print: Arc<print::PrintService>,
    pub print_log: Arc<print_log::PrintLogService>,
}
