//! openapi 领域仓储。

pub mod workbuddy;

use crate::db::DbPool;

#[derive(Clone)]
pub struct Repositories {
    pub workbuddy: workbuddy::WorkbuddyRepository,
}

impl Repositories {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self {
            workbuddy: workbuddy::WorkbuddyRepository::new(pool.clone()),
        }
    }
}
