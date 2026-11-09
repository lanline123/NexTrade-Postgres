//! 分销员域服务：分销员、分组、客户、订单、素材与内容。
//! 对应原 Java `SalesmanServiceImpl` / `SalesmanGroupServiceImpl` /
//! `SalesmanCustomerServiceImpl` / `SalesmanOrderServiceImpl` 等。
//!
//! TODO(域实现者)：对照 Java 源补全业务逻辑。

use std::sync::Arc;

use nextrade_core::models::{SalesmanListQuery, SalesmanVO};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::Result;
use nextrade_infrastructure::db::Repositories;

pub struct SalesmanService {
    repos: Arc<Repositories>,
}

impl SalesmanService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 分销员分页。
    pub async fn list(&self, q: &SalesmanListQuery) -> Result<PageResult<SalesmanVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        let (total, rows) = self
            .repos.marketing.salesman
            .paginate(q.keyword.as_deref(), &page)
            .await?;
        let list = rows.iter().map(SalesmanVO::from).collect();
        Ok(PageResult::new(total, list, &page))
    }
}