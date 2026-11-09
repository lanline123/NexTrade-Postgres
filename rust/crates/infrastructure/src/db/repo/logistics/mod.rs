//! logistics 领域仓储。

pub mod logistics_company;
pub mod logistics_company_config;
pub mod print;
pub mod print_config;
pub mod print_log;
pub mod shipping_list;
pub mod shipping_tpl;
pub mod shipping_type;
pub mod sunmi_print_callback;

use crate::db::DbPool;

#[derive(Clone)]
pub struct Repositories {
    pub logistics_company: logistics_company::LogisticsCompanyRepository,
    pub logistics_company_config: logistics_company_config::LogisticsCompanyConfigRepository,
    pub print: print::PrintRepository,
    pub print_config: print_config::PrintConfigRepository,
    pub print_log: print_log::PrintLogRepository,
    pub shipping_list: shipping_list::LogisticsRepository,
    pub shipping_tpl: shipping_tpl::ShippingTplRepository,
    pub shipping_type: shipping_type::ShippingTypeRepository,
    pub sunmi_print_callback: sunmi_print_callback::SunmiPrintCallbackRepository,
}

impl Repositories {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self {
            logistics_company: logistics_company::LogisticsCompanyRepository::new(pool.clone()),
            logistics_company_config: logistics_company_config::LogisticsCompanyConfigRepository::new(pool.clone()),
            print: print::PrintRepository::new(pool.clone()),
            print_config: print_config::PrintConfigRepository::new(pool.clone()),
            print_log: print_log::PrintLogRepository::new(pool.clone()),
            shipping_list: shipping_list::LogisticsRepository::new(pool.clone()),
            shipping_tpl: shipping_tpl::ShippingTplRepository::new(pool.clone()),
            shipping_type: shipping_type::ShippingTypeRepository::new(pool.clone()),
            sunmi_print_callback: sunmi_print_callback::SunmiPrintCallbackRepository::new(pool.clone()),
        }
    }
}
