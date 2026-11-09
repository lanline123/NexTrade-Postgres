//! openapi 领域服务。

pub mod workbuddy;

use std::sync::Arc;

/// openapi 领域服务集合（由组合根装配一次，api 层只持有 `Arc<Services>`）。
#[derive(Clone)]
pub struct Services {
    pub workbuddy: Arc<workbuddy::WorkbuddyService>,
}
