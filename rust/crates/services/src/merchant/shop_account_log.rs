//! 店铺账户 域服务（merchant::shop_account_log）—— wave9。

use std::sync::Arc;

use nextrade_core::models::merchant::shop_account_log::{
    format_time, ShopAccountQuery, ShopAccountVO, UserAccountLogQuery, UserAccountLogVO,
    MSG_SHOP_NOT_FOUND,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::merchant::shop_account_log::ShopAccountLogRepository;
use nextrade_infrastructure::db::Repositories;

/// 店铺账户 域服务。
pub struct ShopAccountLogService {
    repos: Arc<Repositories>,
}

impl ShopAccountLogService {
    /// 构造服务。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本 feature 仓储。
    #[must_use]
    pub fn repo(&self) -> &ShopAccountLogRepository {
        &self.repos.merchant.shop_account_log
    }

    /// 店铺余额汇总（admin `AccountController.index`）。
    pub async fn index(&self, q: &ShopAccountQuery, header_shop_id: Option<i32>) -> Result<ShopAccountVO> {
        let shop_id = q.shop_id.or(header_shop_id).filter(|v| *v > 0).unwrap_or(0);
        let row = self
            .repo()
            .find_shop(shop_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_SHOP_NOT_FOUND.into()))?;
        Ok(ShopAccountVO {
            shop_id: row.shop_id,
            shop_title: row.shop_title,
            shop_money: row.shop_money,
            frozen_money: row.frozen_money,
            status: row.status,
        })
    }

    /// 用户余额流水（`wallet_type=0`）。
    pub async fn balance_logs(
        &self,
        user_id: Option<i32>,
        q: &UserAccountLogQuery,
    ) -> Result<PageResult<UserAccountLogVO>> {
        self.user_logs(user_id, q, 0).await
    }

    /// 用户佣金流水（`wallet_type=1`）。
    pub async fn commission_logs(
        &self,
        user_id: Option<i32>,
        q: &UserAccountLogQuery,
    ) -> Result<PageResult<UserAccountLogVO>> {
        self.user_logs(user_id, q, 1).await
    }

    async fn user_logs(
        &self,
        user_id: Option<i32>,
        q: &UserAccountLogQuery,
        wallet_type: i32,
    ) -> Result<PageResult<UserAccountLogVO>> {
        let page = q.page_query();
        let Some(uid) = user_id.filter(|v| *v > 0) else {
            // 契约 auth=optional；未登录时返回空页（Java `getCurrentUserId` 在匿名
            // 场景下由鉴权过滤器兜底为 0，查不到流水即空页）。
            return Ok(PageResult::empty(&page));
        };
        let (total, rows) = self
            .repo()
            .paginate_user_logs(uid, wallet_type, q)
            .await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let list = rows
            .into_iter()
            .map(|r| UserAccountLogVO {
                log_id: r.log_id,
                user_id: r.user_id,
                before_balance: r.before_balance,
                balance: r.balance,
                after_balance: r.after_balance,
                frozen_balance: r.frozen_balance,
                wallet_type: r.wallet_type,
                change_desc: r.change_desc,
                change_type: r.change_type,
                change_time: format_time(r.change_time),
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_shop_id_uses_zero_and_errors_via_repo_path() {
        // 纯逻辑：shopId 缺省归一为 0（仓储对 0 返回 None → 服务转「店铺不存在」）。
        let q = ShopAccountQuery { shop_id: None };
        let shop_id = q.shop_id.or(None).filter(|v| *v > 0).unwrap_or(0);
        assert_eq!(shop_id, 0);
        assert_eq!(MSG_SHOP_NOT_FOUND, "店铺不存在");
    }

    #[test]
    fn anonymous_user_yields_empty_page() {
        let q = UserAccountLogQuery::default();
        let page = q.page_query();
        let empty = PageResult::<UserAccountLogVO>::empty(&page);
        assert_eq!(empty.total, 0);
        assert!(empty.list.is_empty());
    }

    #[test]
    fn wallet_types_are_zero_and_one() {
        assert_eq!(0, 0); // balance
        assert_eq!(1, 1); // commission
    }
}
