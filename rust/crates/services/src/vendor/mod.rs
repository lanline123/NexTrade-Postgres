//! vendor 领域服务。

pub mod vendor;
pub mod vendor_product;

use std::sync::Arc;

/// vendor 领域服务集合（由组合根装配一次，api 层只持有 `Arc<Services>`）。
#[derive(Clone)]
pub struct Services {
    pub vendor: Arc<vendor::VendorService>,
    pub vendor_product: Arc<vendor_product::VendorProductService>,
}
