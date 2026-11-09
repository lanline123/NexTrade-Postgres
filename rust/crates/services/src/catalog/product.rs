//! 商品服务：商品查询、详情缓存、库存操作。

use std::sync::Arc;

use nextrade_core::entities::Product;
use nextrade_core::models::catalog::product_group::parse_product_ids;
use nextrade_core::models::{ProductListQuery, ProductVO};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::cache::CacheService;
use nextrade_infrastructure::db::Repositories;

/// 商品详情缓存 TTL（秒）。
const PRODUCT_CACHE_TTL: u64 = 300;

#[derive(Clone)]
pub struct ProductService {
    repos: Arc<Repositories>,
    cache: Option<CacheService>,
}

impl ProductService {
    pub fn new(repos: Arc<Repositories>, cache: Option<CacheService>) -> Self {
        Self { repos, cache }
    }

    /// 商品分页列表。
    pub async fn list(&self, query: &ProductListQuery) -> Result<PageResult<ProductVO>> {
        let page = PageQuery {
            page: query.page.unwrap_or(1),
            page_size: query.page_size.unwrap_or(20),
        };
        let mut expanded = query.clone();
        // 原版首页/分类页的分类筛选包含子分类；公开列表此前只做精确
        // `category_id = ?`，导致父分类下的商品在首页为空。
        if expanded.category_ids.is_empty() {
            if let Some(root) = expanded.category_id.filter(|v| *v > 0) {
                let cats = self.repos.catalog.category.find_categories(false).await?;
                let pairs: Vec<(i32, i32)> = cats
                    .iter()
                    .map(|c| (c.category_id, c.parent_id))
                    .collect();
                expanded.category_ids =
                    crate::catalog::product_manage::descendant_category_ids(&pairs, root);
            }
        }
        // 装修 `pc-product-group` 的 productGroup 数据源：分组商品存于
        // `product_group.product_ids`（JSON），先展开成分组内商品 ID。
        if let Some(group_id) = expanded.product_group_id {
            let group = self
                .repos
                .catalog
                .product_group
                .find_by_id(group_id)
                .await?;
            let ids = group
                .as_ref()
                .map(|item| parse_product_ids(item.product_ids.as_deref()))
                .unwrap_or_default();
            if ids.is_empty() {
                return Ok(PageResult::new(0, Vec::new(), &page));
            }
            if expanded.ids.is_none() {
                expanded.ids = Some(ids.iter().map(i32::to_string).collect::<Vec<_>>().join(","));
            }
        }
        let (total, products) = self.repos.catalog.product.paginate(&expanded).await?;
        let list = products.into_iter().map(to_vo).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 商品详情（缓存优先，替代原 Java @Cacheable）。
    pub async fn detail(&self, product_id: i32) -> Result<ProductVO> {
        let key = format!("product:detail:v2:{product_id}");

        let loader = || async {
            let p = self
                .repos.catalog.product
                .find_by_id(product_id)
                .await?
                .ok_or_else(|| Error::NotFound(format!("商品 {product_id} 不存在")))?;
            if !p.is_on_sale() {
                return Err(Error::NotFound("商品已下架".into()));
            }
            Ok(to_vo(p))
        };

        match &self.cache {
            Some(cache) => cache.get_or_load(&key, PRODUCT_CACHE_TTL, loader).await,
            None => loader().await,
        }
    }

    /// 下单前校验可购买性。
    pub async fn check_purchasable(
        &self,
        product_id: i32,
        sku_id: Option<i32>,
        quantity: i32,
    ) -> Result<()> {
        if quantity <= 0 {
            return Err(Error::Validation("购买数量必须大于0".into()));
        }
        // v5.8.61 商品禁售：加入购物车/结算/下单共用同一校验。
        let sale_state = self
            .repos
            .catalog
            .product_sale
            .setting(product_id, None)
            .await?;
        if sale_state.sale_status == 0 {
            return Err(Error::biz(400, "商品已下架".to_string()));
        }
        if !self
            .repos.catalog.product
            .check_purchasable(product_id, sku_id, quantity)
            .await?
        {
            return Err(Error::InsufficientStock {
                product_id: product_id.to_string(),
                stock: 0,
            });
        }
        Ok(())
    }

    /// 扣减库存（下单核心，失败即库存不足）。
    pub async fn deduct_stock(
        &self,
        product_id: i32,
        sku_id: Option<i32>,
        quantity: i32,
    ) -> Result<()> {
        let ok = self
            .repos.catalog.product
            .deduct_stock(product_id, sku_id, quantity)
            .await?;
        if !ok {
            return Err(Error::InsufficientStock {
                product_id: product_id.to_string(),
                stock: 0,
            });
        }
        // 库存变更后失效详情缓存
        self.invalidate_detail_cache(product_id).await;
        Ok(())
    }

    /// 回滚库存（取消订单/退款）。
    pub async fn restore_stock(
        &self,
        product_id: i32,
        sku_id: Option<i32>,
        quantity: i32,
    ) -> Result<()> {
        self.repos.catalog.product
            .restore_stock(product_id, sku_id, quantity)
            .await?;
        self.invalidate_detail_cache(product_id).await;
        Ok(())
    }

    /// 失效商品详情缓存（库存变更后调用）。无缓存时为空操作。
    pub async fn invalidate_detail_cache(&self, product_id: i32) {
        if let Some(cache) = &self.cache {
            let _ = cache.del(&format!("product:detail:v2:{product_id}")).await;
        }
    }
}

/// 实体 → VO 转换。
fn to_vo(p: Product) -> ProductVO {
    // 虚拟商品不管理库存，库里 stock 常为 0；原版前端用 stock > 0 判断售罄，
    // 因此对虚拟类商品返回一个可售库存值。
    let product_stock = if p.is_virtual_product() {
        999_999
    } else {
        p.product_stock
    };
    ProductVO {
        product_id: p.product_id,
        product_name: p.product_name,
        product_sn: p.product_sn,
        product_price: p.product_price,
        market_price: p.market_price,
        product_stock,
        product_status: p.product_status,
        category_id: p.category_id,
        brand_id: p.brand_id,
        shop_id: p.shop_id,
        pic_url: p.pic_url,
        pic_thumb: p.pic_thumb,
        sales: p.virtual_sales,
        price: Some(p.product_price),
        virtual_sales: p.virtual_sales,
        add_time: p.add_time,
        store_product_name: None,
        store_product_price: None,
        store_pic_url: None,
        store_product_status: None,
        store_product_stock: None,
    }
}