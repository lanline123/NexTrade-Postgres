//! catalog 领域仓储。

pub mod attributes_tpl;
pub mod brand;
pub mod category;
pub mod category_all;
pub mod comment;
pub mod e_card;
pub mod e_card_group;
pub mod exchange;
pub mod price_inquiry;
pub mod product;
pub mod product_batch;
pub mod product_group;
pub mod product_inventory_log;
pub mod product_manage;
pub mod product_manage_client;
pub mod product_sale;
pub mod product_search;
pub mod product_stock;
pub mod search_admin;
pub mod services;
pub mod short_url;
pub mod stock_warning;

use crate::db::DbPool;

#[derive(Clone)]
pub struct Repositories {
    pub attributes_tpl: attributes_tpl::AttributesTplRepository,
    pub brand: brand::BrandRepository,
    pub category: category::CategoryRepository,
    pub category_all: category_all::CatalogRepository,
    pub comment: comment::CommentRepository,
    pub e_card: e_card::ECardRepository,
    pub e_card_group: e_card_group::ECardGroupRepository,
    pub exchange: exchange::ExchangeRepository,
    pub price_inquiry: price_inquiry::PriceInquiryRepository,
    pub product: product::ProductRepository,
    pub product_batch: product_batch::ProductBatchRepository,
    pub product_group: product_group::ProductGroupRepository,
    pub product_inventory_log: product_inventory_log::ProductInventoryLogRepository,
    pub product_manage: product_manage::ProductManageRepository,
    pub product_search: product_search::ProductSearchRepository,
    pub product_stock: product_stock::ProductStockRepository,
    pub search_admin: search_admin::SearchAdminRepository,
    pub services: services::ServicesRepository,
    pub short_url: short_url::ShortUrlRepository,
    pub stock_warning: stock_warning::StockWarningRepository,
    pub product_manage_client: product_manage_client::ProductManageClientRepository,
    pub product_sale: product_sale::ProductSaleRepository,
}

impl Repositories {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self {
            attributes_tpl: attributes_tpl::AttributesTplRepository::new(pool.clone()),
            brand: brand::BrandRepository::new(pool.clone()),
            category: category::CategoryRepository::new(pool.clone()),
            category_all: category_all::CatalogRepository::new(pool.clone()),
            comment: comment::CommentRepository::new(pool.clone()),
            e_card: e_card::ECardRepository::new(pool.clone()),
            e_card_group: e_card_group::ECardGroupRepository::new(pool.clone()),
            exchange: exchange::ExchangeRepository::new(pool.clone()),
            price_inquiry: price_inquiry::PriceInquiryRepository::new(pool.clone()),
            product: product::ProductRepository::new(pool.clone()),
            product_batch: product_batch::ProductBatchRepository::new(pool.clone()),
            product_group: product_group::ProductGroupRepository::new(pool.clone()),
            product_inventory_log: product_inventory_log::ProductInventoryLogRepository::new(pool.clone()),
            product_manage: product_manage::ProductManageRepository::new(pool.clone()),
            product_search: product_search::ProductSearchRepository::new(pool.clone()),
            product_stock: product_stock::ProductStockRepository::new(pool.clone()),
            search_admin: search_admin::SearchAdminRepository::new(pool.clone()),
            services: services::ServicesRepository::new(pool.clone()),
            short_url: short_url::ShortUrlRepository::new(pool.clone()),
            stock_warning: stock_warning::StockWarningRepository::new(pool.clone()),
            product_manage_client: product_manage_client::ProductManageClientRepository::new(pool.clone()),
            product_sale: product_sale::ProductSaleRepository::new(pool.clone()),
        }
    }
}
