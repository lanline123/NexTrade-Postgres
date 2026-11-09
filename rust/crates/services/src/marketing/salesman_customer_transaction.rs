//! 分销客户交易服务（marketing::salesman_customer_transaction）。
//!
//! 对应原 Java `SalesmanOrderServiceImpl#listCustomerTransaction`
//! （1 个管理端端点 `GET /adminapi/salesman/customerTransaction/list`）。
//!
//! # 语义（逐条对照 Java）
//! - `salesmanId` 必填（Java `CustomerTransactionPageQuery.salesmanId`）。
//! - 分页 `salesman_order JOIN order`，按 `order.add_time` 区间过滤，
//!   `order_id DESC`（Java Mapper XML 未显式排序，本实现固定倒序）。
//! - 回填分销员会员名与买家会员名（Java 分别查 `Salesman`/`User`）。
//! - `isExport` 忽略（差异 1）。
//!
//! # 金额
//! `total_amount` / `paid_amount` 为 `Decimal`。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::models::marketing::salesman_customer_transaction::{
    format_time, parse_time, CustomerTransactionQuery, CustomerTransactionVO, DEFAULT_PAGE_SIZE,
    MAX_PAGE_SIZE,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::Result;
use nextrade_infrastructure::db::repo::marketing::salesman_customer_transaction::SalesmanCustomerTransactionRepository;
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 客户交易服务（挂载于 `AdminState.marketing.salesman_customer_transaction`）。
pub struct SalesmanCustomerTransactionService {
    repos: Arc<Repositories>,
}

impl SalesmanCustomerTransactionService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    #[must_use]
    pub fn repo(&self) -> &SalesmanCustomerTransactionRepository {
        &self.repos.marketing.salesman_customer_transaction
    }

    /// 客户交易分页（Java `listCustomerTransaction`）。
    ///
    /// `salesmanId` **可选**：Java `CustomerTransactionPageQuery` 没有 `@NotNull`，
    /// 服务层直接透传（为空即查全部成交记录）。原版 admin 页面把该字段初始化为
    /// 空字符串提交，Jackson 绑定成 `null` → 正常返回；Rust 早期实现强制必填
    /// 导致页面 400（2026-10-21 巡检 `/admin/salesman/customer-transactions/list`）。
    ///
    /// # Errors
    /// 数据库错误 → `Database`；店铺账号越权 → `Forbidden`。
    pub async fn list(
        &self,
        q: &CustomerTransactionQuery,
    ) -> Result<PageResult<CustomerTransactionVO>> {
        let salesman_id = q.salesman_id.filter(|id| *id > 0);
        // 多租户：店铺账号只能查看本店分销员的客户交易
        if let Some(id) = salesman_id {
            if let Some(salesman) = self
                .repos
                .marketing
                .salesman_admin
                .find_salesman(id)
                .await?
            {
                TenantScope::current_ensure_access(salesman.shop_id.unwrap_or(0))?;
            }
        }
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
        };
        let (limit, offset) = page.limit_offset();
        let start = parse_time(q.start_time.as_deref());
        let end = parse_time(q.end_time.as_deref());
        let (total, rows) = self
            .repo()
            .paginate(salesman_id, start, end, limit, offset)
            .await?;
        if rows.is_empty() {
            return Ok(PageResult::new(total, Vec::new(), &page));
        }
        // 分销员名按**结果集里出现的** salesman_id 批量回填（未指定过滤时为多值）。
        let mut salesman_ids: Vec<i32> = rows.iter().map(|r| r.salesman_id).collect();
        salesman_ids.sort_unstable();
        salesman_ids.dedup();
        let salesman_names: HashMap<i32, Option<String>> = self
            .repo()
            .salesman_names(&salesman_ids)
            .await?
            .into_iter()
            .collect();
        let buyer_ids: Vec<i32> = rows.iter().map(|r| r.user_id).collect();
        let usernames: HashMap<i32, Option<String>> = self
            .repo()
            .usernames(&buyer_ids)
            .await?
            .into_iter()
            .collect();
        let list: Vec<CustomerTransactionVO> = rows
            .iter()
            .map(|r| CustomerTransactionVO {
                salesman_id: r.salesman_id,
                salesman_name: salesman_names.get(&r.salesman_id).cloned().flatten(),
                order_id: r.order_id,
                order_sn: r.order_sn.clone(),
                user_id: r.user_id,
                username: usernames.get(&r.user_id).cloned().flatten(),
                add_time: format_time(r.add_time),
                total_amount: r.total_amount,
                paid_amount: r.paid_amount,
                order_status: r.order_status,
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tenant_scope_blocks_cross_shop_transaction() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{with_tenant_scope, TenantScope};
        let shop = TenantScope::for_shop(8);
        assert!(
            with_tenant_scope(shop, async { TenantScope::current_ensure_access(9) })
                .await
                .is_err()
        );
        assert!(
            with_tenant_scope(shop, async { TenantScope::current_ensure_access(8) })
                .await
                .is_ok()
        );
    }

    /// `salesmanId` 不再是必填：Java `CustomerTransactionPageQuery` 无 `@NotNull`，
    /// 原版页面把该字段以空字符串提交，Jackson 绑成 null → 查全部。
    /// 这里验证「缺失 / 空串 / 非法串」都解析成 `None`，不再抛校验错误。
    #[test]
    fn empty_salesman_id_means_no_filter() {
        let q: CustomerTransactionQuery =
            serde_json::from_str(r#"{"page":1,"size":10,"salesmanId":""}"#).unwrap();
        assert_eq!(q.salesman_id, None);
        let q2: CustomerTransactionQuery = serde_json::from_str(r#"{"salesmanId":"abc"}"#).unwrap();
        assert_eq!(q2.salesman_id, None);
        let q3: CustomerTransactionQuery = serde_json::from_str(r#"{"salesmanId":"7"}"#).unwrap();
        assert_eq!(q3.salesman_id, Some(7));
        let q4 = CustomerTransactionQuery::default();
        assert_eq!(q4.salesman_id, None);
    }

    #[test]
    fn page_defaults_match_base_page() {
        let q = CustomerTransactionQuery::default();
        assert_eq!(q.page.unwrap_or(1), 1);
        assert_eq!(q.page_size.unwrap_or(DEFAULT_PAGE_SIZE), 15);
        assert_eq!(MAX_PAGE_SIZE, 100);
    }
}
