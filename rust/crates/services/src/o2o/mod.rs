//! o2o 领域服务（本地生活/到店）。

pub mod config;
pub mod pickup_client;
pub mod reservation_tpl;
pub mod stock_calendar;
pub mod store_product;

use std::sync::Arc;

/// o2o 领域服务集合（由组合根装配一次，api 层只持有 `Arc<Services>`）。
#[derive(Clone)]
pub struct Services {
    pub config: Arc<config::O2oConfigService>,
    pub pickup_client: Arc<pickup_client::PickupClientService>,
    pub reservation_tpl: Arc<reservation_tpl::ReservationTplService>,
    pub stock_calendar: Arc<stock_calendar::StockCalendarService>,
    pub store_product: Arc<store_product::StoreProductService>,
}
