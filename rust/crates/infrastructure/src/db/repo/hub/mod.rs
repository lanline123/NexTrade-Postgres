//! 平台联邦仓储组（hub 上下文）。

pub mod federation;

use sqlx::PgPool;

/// 联邦仓储聚合（挂载于 `Repositories.hub`）。
#[derive(Clone)]
pub struct Repositories {
    pub federation: federation::HubFederationRepository,
}

impl Repositories {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            federation: federation::HubFederationRepository::new(pool),
        }
    }
}
