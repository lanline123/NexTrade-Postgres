//! 站点搬家仓储组（site_clone 上下文）。
//!
//! 表见 `rust/migrations/015_site_clone_tables.sql`；目录域读写（分类/品牌/
//! 运费模板/商品全保真导出与对齐落库）在 `catalog_data`。

pub mod catalog_data;
pub mod clone;

use sqlx::PgPool;

/// 站点搬家仓储聚合（挂载于 `Repositories.site_clone`）。
#[derive(Clone)]
pub struct Repositories {
    pub clone: clone::SiteCloneRepository,
    pub catalog: catalog_data::SiteCloneCatalogRepository,
}

impl Repositories {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            clone: clone::SiteCloneRepository::new(pool.clone()),
            catalog: catalog_data::SiteCloneCatalogRepository::new(pool),
        }
    }
}
