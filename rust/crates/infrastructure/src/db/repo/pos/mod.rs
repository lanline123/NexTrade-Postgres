//! pos 领域仓储。

pub mod cashier;
pub mod device;

use crate::db::DbPool;

#[derive(Clone)]
pub struct Repositories {
    pub cashier: cashier::PosCashierRepository,
    pub device: device::PosDeviceRepository,
}

impl Repositories {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self {
            cashier: cashier::PosCashierRepository::new(pool.clone()),
            device: device::PosDeviceRepository::new(pool),
        }
    }
}
