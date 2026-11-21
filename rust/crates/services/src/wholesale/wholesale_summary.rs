//! 批发汇总域服务（wholesale::wholesale_summary）。**我方新增**
//! （论坛帖 127889 评估 F3「总部批发汇总」+ 门店级批发清单，二期补齐）。
//!
//! | 端点（挂载于 `/adminapi`） | 方法 |
//! |---------------------------|------|
//! | `GET /wholesale/summary/overview` | [`WholesaleSummaryService::overview`] |
//! | `GET /wholesale/summary/shopPage` | [`WholesaleSummaryService::shop_page`] |
//! | `GET /wholesale/summary/productPage` | [`WholesaleSummaryService::product_page`] |
//!
//! 口径与权限见 `core::models::wholesale::wholesale_summary` 模块头：
//! 成交按批发行归属门店聚合（已支付、未取消、未删），金额 = 成交单价
//! × 数量（与结算口径一致）；平台见全量、商家强制本店、缺作用域
//! fail-closed。门店级批发清单委托 [`WholesaleProductService::page_multi`]
//! 装配，与批发商品列表同口径。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::models::wholesale::wholesale_setting::effective_shop_wholesale;
use nextrade_core::models::wholesale::wholesale_summary::{
    resolve_summary_scope, shop_display_title, WholesaleShopProductPageQuery,
    WholesaleShopProductVO, WholesaleShopSummaryVO, WholesaleSummaryOverviewVO,
    WholesaleSummaryQuery,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::Result;
use nextrade_infrastructure::db::repo::wholesale::wholesale_summary::WholesaleSummaryRepository;
use nextrade_infrastructure::db::Repositories;

use super::wholesale_product::WholesaleProductService;

/// 默认分页大小（与批发商品列表一致的量级）。
const DEFAULT_PAGE_SIZE: u64 = 20;
/// 分页大小上限。
const MAX_PAGE_SIZE: u64 = 100;

/// 批发汇总域服务（挂载于 `AdminState.wholesale.wholesale_summary`）。
pub struct WholesaleSummaryService {
    repos: Arc<Repositories>,
    wholesale_product: Arc<WholesaleProductService>,
}

impl WholesaleSummaryService {
    /// 构造（清单装配复用批发商品服务，与组合根既有跨服务注入同形）。
    #[must_use]
    pub fn new(
        repos: Arc<Repositories>,
        wholesale_product: Arc<WholesaleProductService>,
    ) -> Self {
        Self {
            repos,
            wholesale_product,
        }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &WholesaleSummaryRepository {
        &self.repos.wholesale.wholesale_summary
    }

    /// 时间区间归一：缺省 = 全部时间；起止颠倒时交换（宽容前端传参）。
    fn range(q: &WholesaleSummaryQuery) -> (i64, i64) {
        let start = q.start_time.unwrap_or(0).max(0);
        let end = q.end_time.unwrap_or(i64::MAX);
        if start > end {
            (end, start)
        } else {
            (start, end)
        }
    }

    /// 汇总总览。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn overview(
        &self,
        q: &WholesaleSummaryQuery,
        caller_shop_id: i32,
    ) -> Result<WholesaleSummaryOverviewVO> {
        let scope = resolve_summary_scope(caller_shop_id, q.shop_id);
        let (start, end) = Self::range(q);
        let agg = self.repo().aggregate(scope, start, end).await?;
        let configured: i64 = self
            .repo()
            .configured_counts_by_shop(scope)
            .await?
            .into_iter()
            .map(|(_, n)| n)
            .sum();
        // 批发商是买家全局身份：仅平台口径返回总数，单店口径不回填。
        let wholesaler_count = match scope {
            None => Some(self.repo().active_wholesaler_count().await?),
            Some(_) => None,
        };
        Ok(WholesaleSummaryOverviewVO {
            scope_shop_id: scope,
            shop_count: agg.shop_count,
            configured_product_count: configured,
            wholesaler_count,
            order_count: agg.order_count,
            line_count: agg.line_count,
            total_quantity: agg.total_quantity.unwrap_or(0),
            total_amount: agg.total_amount.unwrap_or_default(),
            buyer_count: agg.buyer_count,
        })
    }

    /// 按门店汇总（成交聚合 ∪ 已配置商品的门店，按成交金额降序分页）。
    ///
    /// 单店作用域（商家 / 平台指定单店）下即使该店暂无数据也返回一行
    /// 全零行，保证前端有稳定的本店视图。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn shop_page(
        &self,
        q: &WholesaleSummaryQuery,
        caller_shop_id: i32,
    ) -> Result<PageResult<WholesaleShopSummaryVO>> {
        let scope = resolve_summary_scope(caller_shop_id, q.shop_id);
        let (start, end) = Self::range(q);
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q
                .page_size
                .unwrap_or(DEFAULT_PAGE_SIZE)
                .min(MAX_PAGE_SIZE),
        };

        let agg_rows = self.repo().aggregate_by_shop(scope, start, end).await?;
        let configured: HashMap<i32, i64> = self
            .repo()
            .configured_counts_by_shop(scope)
            .await?
            .into_iter()
            .collect();
        let global_enabled = self.repo().global_wholesale_enabled().await?;
        let switch_values: HashMap<i32, i32> = self
            .repo()
            .shop_switch_values()
            .await?
            .into_iter()
            .collect();

        // 门店全集 = 有成交 ∪ 有配置；单店作用域恒含本店一行。
        let mut shop_ids: Vec<i32> = agg_rows.iter().map(|r| r.shop_id).collect();
        for sid in configured.keys() {
            if !shop_ids.contains(sid) {
                shop_ids.push(*sid);
            }
        }
        if let Some(sid) = scope {
            if !shop_ids.contains(&sid) {
                shop_ids.push(sid);
            }
        }
        let titles: HashMap<i32, String> = self
            .repo()
            .shop_titles(&shop_ids)
            .await?
            .into_iter()
            .collect();
        let agg_by_shop: HashMap<i32, _> =
            agg_rows.into_iter().map(|r| (r.shop_id, r)).collect();

        let mut rows: Vec<WholesaleShopSummaryVO> = shop_ids
            .iter()
            .map(|sid| {
                let agg = agg_by_shop.get(sid);
                WholesaleShopSummaryVO {
                    shop_id: *sid,
                    shop_title: shop_display_title(
                        *sid,
                        titles.get(sid).map(String::as_str),
                    ),
                    wholesale_enabled: effective_shop_wholesale(
                        global_enabled,
                        *sid,
                        switch_values.get(sid).copied(),
                    ),
                    configured_product_count: configured.get(sid).copied().unwrap_or(0),
                    order_count: agg.map_or(0, |r| r.order_count),
                    buyer_count: agg.map_or(0, |r| r.buyer_count),
                    total_quantity: agg.and_then(|r| r.total_quantity).unwrap_or(0),
                    total_amount: agg.and_then(|r| r.total_amount).unwrap_or_default(),
                }
            })
            .collect();
        // 成交金额降序、同额按 shop_id 升序（稳定口径）。
        rows.sort_by(|a, b| {
            b.total_amount
                .cmp(&a.total_amount)
                .then(a.shop_id.cmp(&b.shop_id))
        });

        let total = rows.len() as u64;
        let (limit, offset) = page.limit_offset();
        let list = rows
            .into_iter()
            .skip(offset.max(0) as usize)
            .take(limit.max(1) as usize)
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 门店级批发清单（跨店批发商品列表；装配委托批发商品服务）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn product_page(
        &self,
        q: &WholesaleShopProductPageQuery,
        caller_shop_id: i32,
    ) -> Result<PageResult<WholesaleShopProductVO>> {
        let scope = resolve_summary_scope(caller_shop_id, q.shop_id);
        self.wholesale_product.page_multi(q, scope).await
    }
}
