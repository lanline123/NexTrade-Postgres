//! vendor 领域仓储。

pub mod vendor;
pub mod vendor_product;

use crate::db::DbPool;

#[derive(Clone)]
pub struct Repositories {
    pub vendor: vendor::VendorRepository,
    pub vendor_product: vendor_product::VendorProductRepository,
}

impl Repositories {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self {
            vendor: vendor::VendorRepository::new(pool.clone()),
            vendor_product: vendor_product::VendorProductRepository::new(pool),
        }
    }
}
