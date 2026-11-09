//! analytics 领域服务。

pub mod panel;
pub mod sales_statistics;
pub mod statistics;

use std::sync::Arc;

/// analytics 领域服务集合（由组合根装配一次，api 层只持有 `Arc<Services>`）。
#[derive(Clone)]
pub struct Services {
    pub statistics: Arc<statistics::StatisticsService>,
    pub panel: Arc<panel::PanelService>,
    pub sales_statistics: Arc<sales_statistics::SalesStatisticsService>,
}
