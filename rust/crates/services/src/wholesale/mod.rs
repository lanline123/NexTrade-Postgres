//! wholesale 领域服务。

pub mod wholesale_client;
pub mod wholesale_product;
pub mod wholesale_setting;
pub mod wholesaler;
pub mod wholesaler_grade;

use std::sync::Arc;

/// wholesale 领域服务集合（由组合根装配一次，api 层只持有 `Arc<Services>`）。
#[derive(Clone)]
pub struct Services {
    pub wholesale_client: Arc<wholesale_client::WholesaleClientService>,
    pub wholesale_setting: Arc<wholesale_setting::WholesaleSettingService>,
    pub wholesaler_grade: Arc<wholesaler_grade::WholesalerGradeService>,
    pub wholesaler: Arc<wholesaler::WholesalerService>,
    pub wholesale_product: Arc<wholesale_product::WholesaleProductService>,
}
