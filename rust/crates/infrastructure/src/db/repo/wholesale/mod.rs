//! wholesale 领域仓储。

pub mod wholesale_client;
pub mod wholesale_product;
pub mod wholesale_setting;
pub mod wholesaler;
pub mod wholesaler_grade;

use crate::db::DbPool;

#[derive(Clone)]
pub struct Repositories {
    pub wholesale_client: wholesale_client::WholesaleClientRepository,
    pub wholesale_setting: wholesale_setting::WholesaleSettingRepository,
    pub wholesaler_grade: wholesaler_grade::WholesalerGradeRepository,
    pub wholesaler: wholesaler::WholesalerRepository,
    pub wholesale_product: wholesale_product::WholesaleProductRepository,
}

impl Repositories {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self {
            wholesale_client: wholesale_client::WholesaleClientRepository::new(pool.clone()),
            wholesale_setting: wholesale_setting::WholesaleSettingRepository::new(pool.clone()),
            wholesaler_grade: wholesaler_grade::WholesalerGradeRepository::new(pool.clone()),
            wholesaler: wholesaler::WholesalerRepository::new(pool.clone()),
            wholesale_product: wholesale_product::WholesaleProductRepository::new(pool.clone()),
        }
    }
}
