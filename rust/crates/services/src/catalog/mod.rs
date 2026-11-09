//! catalog 领域服务。

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
pub mod product_manage_extra;
pub mod product_search;
pub mod product_stock;
pub mod search_admin;
pub mod services;
pub mod short_url;
pub mod stock_warning;

use std::sync::Arc;

/// catalog 领域服务集合（由组合根装配一次，api 层只持有 `Arc<Services>`）。
#[derive(Clone)]
pub struct Services {
    pub attributes_tpl: Arc<attributes_tpl::AttributesTplService>,
    pub brand: Arc<brand::BrandService>,
    pub category: Arc<category::CategoryService>,
    pub category_all: Arc<category_all::CatalogService>,
    pub product: Arc<product::ProductService>,
    pub product_manage: Arc<product_manage::ProductManageService>,
    pub product_stock: Arc<product_stock::ProductStockService>,
    pub short_url: Arc<short_url::ShortUrlService>,
    pub stock_warning: Arc<stock_warning::StockWarningService>,
    pub comment: Arc<comment::CommentService>,
    pub price_inquiry: Arc<price_inquiry::PriceInquiryService>,
    pub product_group: Arc<product_group::ProductGroupService>,
    pub e_card_group: Arc<e_card_group::ECardGroupService>,
    pub e_card: Arc<e_card::ECardService>,
    pub product_batch: Arc<product_batch::ProductBatchService>,
    pub search_admin: Arc<search_admin::SearchAdminService>,
    pub product_inventory_log: Arc<product_inventory_log::ProductInventoryLogService>,
    pub product_search: Arc<product_search::ProductSearchService>,
    pub services: Arc<services::ServicesService>,
    pub exchange: Arc<exchange::ExchangeService>,
    pub product_manage_extra: Arc<product_manage_extra::ProductManageExtraService>,
    pub product_manage_client: Arc<product_manage_client::ProductManageClientService>,
}
