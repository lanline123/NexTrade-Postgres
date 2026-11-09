//! 微信直播商品库域服务（content::wechat_live_goods）。
//!
//! 三个端点均依赖微信开放平台「直播商品库」API（导入/审核/查询），
//! 未迁移 → 保留端点与本地校验后返回明确「未迁移」业务错误，不编造商品数据。

use std::sync::Arc;

use nextrade_core::models::content::wechat_live_goods::{
    validate_import, validate_query, WechatLiveGoodsImportParam, WechatLiveGoodsPageQuery,
    WechatLiveGoodsQuery, WechatLiveGoodsVO, MSG_NOT_MIGRATED,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::content::wechat_live_goods::WechatLiveGoodsRepository;
use nextrade_infrastructure::db::Repositories;

/// 微信直播商品库服务。
pub struct WechatLiveGoodsService {
    repos: Arc<Repositories>,
}

impl WechatLiveGoodsService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本域仓储（NESTED：`self.repos.content.wechat_live_goods`）。
    #[must_use]
    pub fn repo(&self) -> &WechatLiveGoodsRepository {
        &self.repos.content.wechat_live_goods
    }

    /// 导入商品到微信直播商品库（未迁移）。
    pub async fn import(&self, param: &WechatLiveGoodsImportParam) -> Result<()> {
        validate_import(param).map_err(Error::Validation)?;
        Err(Error::biz_msg(MSG_NOT_MIGRATED))
    }

    /// 商品库列表：读取本地缓存表（微信侧同步未迁移，缓存为空时返回空页）。
    pub async fn list(
        &self,
        q: &WechatLiveGoodsPageQuery,
    ) -> Result<PageResult<WechatLiveGoodsVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.size.unwrap_or(20),
        };
        let (total, rows) = self
            .repo()
            .paginate(
                q.room_id,
                q.audit_status,
                q.keyword.as_deref(),
                page.page,
                page.page_size,
            )
            .await?;
        Ok(PageResult::new(total, rows.iter().map(to_vo).collect(), &page))
    }

    /// 单商品查询：读取本地缓存表。
    pub async fn query(&self, param: &WechatLiveGoodsQuery) -> Result<Option<WechatLiveGoodsVO>> {
        validate_query(param).map_err(Error::Validation)?;
        Ok(self
            .repo()
            .find_one(param.wechat_goods_id, param.product_id)
            .await?
            .as_ref()
            .map(to_vo))
    }
}

fn to_vo(e: &nextrade_core::entities::WechatLiveGoods) -> WechatLiveGoodsVO {
    WechatLiveGoodsVO {
        id: e.id,
        room_id: e.room_id,
        wechat_goods_id: e.wechat_goods_id,
        product_id: e.product_id,
        product_sku_id: e.product_sku_id,
        goods_name: e.goods_name.clone(),
        cover_img: e.cover_img.clone(),
        price: e.price,
        price_type: e.price_type,
        url: e.url.clone(),
        audit_status: e.audit_status,
        sort: e.sort,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_runs_before_not_migrated() {
        assert!(validate_import(&WechatLiveGoodsImportParam::default()).is_err());
        assert!(MSG_NOT_MIGRATED.contains("尚未迁移"));
    }
}
