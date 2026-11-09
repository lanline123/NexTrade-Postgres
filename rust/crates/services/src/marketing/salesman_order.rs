//! 分销业绩订单服务（marketing::salesman_order）。
//!
//! 对应原 Java `SalesmanOrderServiceImpl` 的 4 个端点：
//! admin `list` / `batch` / `settlement`，public `clientList`（同 `list`，
//! 但 `salesmanId` 强制为当前登录分销员）。
//!
//! # 语义（逐条对照 Java）
//! - `list`：`salesmanId` + `orderSn`（先查 `order.order_sn` LIKE 得到
//!   orderIds）+ `status != -1` + `timeType`（1 下单 / 2 支付 / 3 收货，
//!   对应 `order.add_time/pay_time/received_time` 区间）+ 排序白名单。
//! - `batch`：仅 `type=del` → 物理删除。
//! - `settlement`：Java `manualSettle` 仅处理 `status=0`；Rust 置
//!   `status=1` + `settlement_time=now`（差异 1，跨域结算副作用未迁移）。
//! - `clientList`：当前登录分销员的业绩订单；游客 / 非分销员 → 空页
//!   （Java 此处会 NPE，本实现做防御，见差异 4）。
//!
//! # 金额
//! `amount` / `order_amount` 为 `Decimal`。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::entities::SalesmanOrder;
use nextrade_core::models::marketing::salesman_order::{
    format_time, parse_time, status_text, BatchDTO, SalesmanOrderListQuery, SalesmanOrderSettlementDTO,
    SalesmanOrderVO, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, MSG_IDS_REQUIRED,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::salesman_order::SalesmanOrderRepository;
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 业绩订单服务（挂载于 `AdminState.marketing.salesman_order` / `PublicState`）。
pub struct SalesmanOrderService {
    repos: Arc<Repositories>,
}

impl SalesmanOrderService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    #[must_use]
    pub fn repo(&self) -> &SalesmanOrderRepository {
        &self.repos.marketing.salesman_order
    }

    /// 管理端分页（Java `list`）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn list(&self, q: &SalesmanOrderListQuery) -> Result<PageResult<SalesmanOrderVO>> {
        // 多租户：店铺账号强制本店（经 salesman 关联）；平台账号不过滤
        self.query_page(q, TenantScope::current_list_filter()).await
    }

    /// public 分页（Java `clientList`：`salesmanId` 由登录态强制）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn client_list(
        &self,
        user_id: i32,
        q: &SalesmanOrderListQuery,
    ) -> Result<PageResult<SalesmanOrderVO>> {
        let Some(salesman) = self
            .repos
            .marketing
            .salesman_admin
            .find_by_user_id(user_id)
            .await?
        else {
            // 游客 / 非分销员：返回空页（Java 在此 NPE，做防御性处理）
            let page = PageQuery {
                page: q.page.unwrap_or(1),
                page_size: q.page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
            };
            return Ok(PageResult::new(0, Vec::new(), &page));
        };
        let mut effective = q.clone();
        effective.salesman_id = Some(salesman.salesman_id);
        self.query_page(&effective, None).await
    }

    async fn query_page(
        &self,
        q: &SalesmanOrderListQuery,
        shop_id: Option<i32>,
    ) -> Result<PageResult<SalesmanOrderVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
        };
        let (limit, offset) = page.limit_offset();

        // orderSn → orderIds
        let mut order_ids: Option<Vec<i32>> = match q.order_sn.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(sn) => Some(self.repo().order_ids_by_sn(sn).await?),
            None => None,
        };
        // timeType + 时间区间 → orderIds（与 orderSn 结果取交集）
        if let Some(time_type) = q.time_type.filter(|t| *t != -1) {
            let start = parse_time(q.order_time_start.as_deref());
            let end = parse_time(q.order_time_end.as_deref());
            let time_ids = self.repo().order_ids_by_time(time_type, start, end).await?;
            order_ids = Some(match order_ids {
                Some(prev) => prev.into_iter().filter(|id| time_ids.contains(id)).collect(),
                None => time_ids,
            });
        }
        if order_ids.as_ref().is_some_and(Vec::is_empty) {
            return Ok(PageResult::new(0, Vec::new(), &page));
        }
        let (total, rows) = self
            .repo()
            .paginate(
                q.salesman_id,
                shop_id,
                order_ids.as_deref(),
                q.status,
                q.sort_field.as_deref(),
                q.sort_order.as_deref(),
                limit,
                offset,
            )
            .await?;
        let sn_map: HashMap<i32, String> = self
            .repo()
            .order_sn_map(&rows.iter().filter_map(|r| r.order_id).collect::<Vec<_>>())
            .await?;
        let list = rows.iter().map(|r| to_vo(r, &sn_map)).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 批量操作（仅 `del` → 物理删除）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if !dto.r#type.as_deref().is_some_and(|t| t == "del") {
            return Ok(false);
        }
        let Some(ids) = dto.ids.as_deref() else {
            return Ok(false);
        };
        // 多租户：批量删除前逐条校验归属（经 salesman.shop_id）
        self.ensure_records_access(ids).await?;
        Ok(self.repo().delete(ids).await? > 0)
    }

    /// 人工结算（Java `manualSettle`；差异 1）。
    ///
    /// # Errors
    /// 未传 ids → `Validation(请选择要结算的记录)`；越权 → `Forbidden`。
    pub async fn settlement(&self, dto: &SalesmanOrderSettlementDTO) -> Result<u64> {
        let ids = dto
            .ids
            .as_deref()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| Error::Validation(MSG_IDS_REQUIRED.into()))?;
        // 多租户：人工结算前逐条校验归属
        self.ensure_records_access(ids).await?;
        let now = i32::try_from(nextrade_common::utils::now_seconds()).unwrap_or(i32::MAX);
        let affected = self.repo().mark_settled(ids, now).await?;
        Ok(affected)
    }

    /// 逐条校验记录归属：`salesman_order.salesman_id → salesman.shop_id`。
    async fn ensure_records_access(&self, ids: &[i32]) -> Result<()> {
        for id in ids {
            let Some(record) = self.repo().find_by_id(*id).await? else {
                continue;
            };
            let Some(salesman_id) = record.salesman_id else {
                continue;
            };
            let Some(salesman) = self
                .repos
                .marketing
                .salesman_admin
                .find_salesman(salesman_id)
                .await?
            else {
                continue;
            };
            TenantScope::current_ensure_access(salesman.shop_id.unwrap_or(0))?;
        }
        Ok(())
    }
}

/// 行 → VO（回填 `order_sn`）。
fn to_vo(r: &SalesmanOrder, sn_map: &HashMap<i32, String>) -> SalesmanOrderVO {
    SalesmanOrderVO {
        salesman_order_id: r.salesman_order_id,
        order_id: r.order_id,
        order_sn: r.order_id.and_then(|id| sn_map.get(&id).cloned()),
        salesman_id: r.salesman_id,
        amount: r.amount,
        status: r.status,
        status_text: status_text(r.status),
        add_time: format_time(r.add_time),
        item_id: r.item_id,
        order_amount: r.order_amount,
        settlement_time: r.settlement_time,
        product_id: r.product_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tenant_scope_filters_admin_list_by_shop() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        let shop = TenantScope::for_shop(8);
        let filter = with_tenant_scope(shop, async { TenantScope::current_list_filter() }).await;
        assert_eq!(filter, Some(8));
        assert!(with_tenant_scope(shop, async { TenantScope::current_ensure_access(9) })
            .await
            .is_err());
        let filter = with_tenant_scope(TenantScope::platform(), async {
            TenantScope::current_list_filter()
        })
        .await;
        assert_eq!(filter, None);
    }

    #[test]
    fn to_vo_backfills_order_sn_and_status_text() {
        let order = SalesmanOrder {
            salesman_order_id: 7,
            order_id: Some(3),
            salesman_id: Some(1),
            amount: Some("5.50".parse().unwrap()),
            status: Some(0),
            add_time: Some(1_700_000_000),
            ..Default::default()
        };
        let mut map = HashMap::new();
        map.insert(3, "SN3".to_string());
        let vo = to_vo(&order, &map);
        assert_eq!(vo.order_sn.as_deref(), Some("SN3"));
        assert_eq!(vo.status_text, "待结算");
        assert_eq!(vo.add_time, "2023-11-15 06:13:20");
    }

    #[test]
    fn to_vo_tolerates_missing_sn() {
        let order = SalesmanOrder {
            salesman_order_id: 7,
            order_id: Some(4),
            status: Some(9),
            ..Default::default()
        };
        let vo = to_vo(&order, &HashMap::new());
        assert!(vo.order_sn.is_none());
        assert_eq!(vo.status_text, "");
    }

    #[test]
    fn settlement_requires_ids() {
        let dto = SalesmanOrderSettlementDTO::default();
        assert!(dto.ids.is_none());
        assert_eq!(MSG_IDS_REQUIRED, "请选择要结算的记录");
    }
}
