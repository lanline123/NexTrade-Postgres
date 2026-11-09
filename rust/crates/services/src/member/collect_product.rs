//! 商品收藏域服务（member::collect_product）。
//! 构造 `new(repos: Arc<Repositories>)`；仓储嵌套 `self.repos.member.collect_product`。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::models::member::collect_product::{
    format_time, CollectListQuery, CollectProductDTO, CollectProductVO, COLLECT_NOT_EXIST,
    PRODUCT_ALREADY_COLLECTED, PRODUCT_ID_REQUIRED, PRODUCT_NOT_EXIST, SERVICE_DATA_ERROR,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::member::collect_product::CollectProductRepository;
use nextrade_infrastructure::db::Repositories;

/// 商品收藏域服务。
pub struct CollectProductService {
    repos: Arc<Repositories>,
}

impl CollectProductService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    #[must_use]
    pub fn repo(&self) -> &CollectProductRepository {
        &self.repos.member.collect_product
    }

    /// 收藏列表（Java `list`）。
    pub async fn list(
        &self,
        user_id: i32,
        q: &CollectListQuery,
    ) -> Result<PageResult<CollectProductVO>> {
        let (page, size) = q.page_params();
        let (total, rows) = self.repo().paginate(user_id, page, size).await?;
        let list = rows
            .into_iter()
            .map(|mut vo| {
                let _ = format_time(Some(vo.add_time));
                vo
            })
            .collect();
        Ok(PageResult::new(
            total,
            list,
            &PageQuery {
                page,
                page_size: size,
            },
        ))
    }

    /// 收藏（Java `save`）。
    pub async fn save(&self, user_id: i32, dto: &CollectProductDTO) -> Result<i32> {
        let product_id = dto
            .product_id
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, PRODUCT_ID_REQUIRED))?;
        if !self.repo().product_exists(product_id).await? {
            return Err(Error::biz(SERVICE_DATA_ERROR, PRODUCT_NOT_EXIST));
        }
        if self.repo().exists(user_id, product_id).await? {
            return Err(Error::biz(SERVICE_DATA_ERROR, PRODUCT_ALREADY_COLLECTED));
        }
        let shop_id = self.repo().product_shop_id(product_id).await?.unwrap_or(0);
        self.repo()
            .insert(
                user_id,
                product_id,
                shop_id,
                i32::try_from(now_seconds()).unwrap_or(0),
            )
            .await
    }

    /// 取消收藏（Java `cancel`）。
    pub async fn cancel(&self, user_id: i32, dto: &CollectProductDTO) -> Result<()> {
        let product_id = dto
            .product_id
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, PRODUCT_ID_REQUIRED))?;
        if !self.repo().delete(user_id, product_id).await? {
            return Err(Error::biz(SERVICE_DATA_ERROR, COLLECT_NOT_EXIST));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_dto_requires_product() {
        let dto = CollectProductDTO::default();
        // 纯逻辑校验（无 DB）：缺 productId → 对应文案
        assert!(dto.product_id.is_none());
        assert_eq!(PRODUCT_ID_REQUIRED, "商品ID不能为空");
    }
}
