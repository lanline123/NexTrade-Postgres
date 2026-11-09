//! analytics 领域仓储。

pub mod panel;
pub mod sales_statistics;
pub mod statistics;

use crate::db::DbPool;

#[derive(Clone)]
pub struct Repositories {
    pub statistics: statistics::StatisticsRepository,
    pub panel: panel::PanelRepository,
    pub sales_statistics: sales_statistics::SalesStatisticsRepository,
}

impl Repositories {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self {
            statistics: statistics::StatisticsRepository::new(pool.clone()),
            panel: panel::PanelRepository::new(pool.clone()),
            sales_statistics: sales_statistics::SalesStatisticsRepository::new(pool.clone()),
        }
    }
}
