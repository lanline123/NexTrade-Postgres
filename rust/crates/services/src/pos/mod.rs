//! pos 领域服务。

pub mod cashier;
pub mod device;

use std::sync::Arc;

/// pos 领域服务集合（由组合根装配一次，api 层只持有 `Arc<Services>`）。
#[derive(Clone)]
pub struct Services {
    pub cashier: Arc<cashier::PosCashierService>,
    pub device: Arc<device::PosDeviceService>,
}
