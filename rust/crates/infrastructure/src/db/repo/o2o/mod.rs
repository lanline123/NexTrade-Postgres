//! o2o 领域仓储（本地生活/到店）。

pub mod config;
pub mod pickup_client;
pub mod reservation_tpl;
pub mod stock_calendar;
pub mod store_product;

use crate::db::DbPool;

#[derive(Clone)]
pub struct Repositories {
    pub config: config::O2oConfigRepository,
    pub pickup_client: pickup_client::PickupClientRepository,
    pub reservation_tpl: reservation_tpl::ReservationTplRepository,
    pub stock_calendar: stock_calendar::StockCalendarRepository,
    pub store_product: store_product::StoreProductRepository,
}

impl Repositories {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self {
            config: config::O2oConfigRepository::new(pool.clone()),
            pickup_client: pickup_client::PickupClientRepository::new(pool.clone()),
            reservation_tpl: reservation_tpl::ReservationTplRepository::new(pool.clone()),
            stock_calendar: stock_calendar::StockCalendarRepository::new(pool.clone()),
            store_product: store_product::StoreProductRepository::new(pool),
        }
    }
}
