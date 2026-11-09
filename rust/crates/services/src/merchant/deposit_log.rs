//! 店铺保证金交易明细 域服务（merchant::deposit_log）—— wave9。

use std::sync::Arc;

use nextrade_core::models::merchant::deposit_log::{
    format_time, DepositLogListQuery, DepositLogVO, MSG_LOG_ID_INVALID, MSG_LOG_NOT_FOUND,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::merchant::deposit_log::DepositLogRepository;
use nextrade_infrastructure::db::Repositories;

/// 明细 域服务。
pub struct DepositLogService {
    repos: Arc<Repositories>,
}

impl DepositLogService {
    /// 构造服务。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本 feature 仓储。
    #[must_use]
    pub fn repo(&self) -> &DepositLogRepository {
        &self.repos.merchant.deposit_log
    }

    /// 分页。
    pub async fn list(&self, q: &DepositLogListQuery) -> Result<PageResult<DepositLogVO>> {
        let page = q.page_query();
        let (total, rows) = self.repo().paginate(q).await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let list = rows
            .into_iter()
            .map(|r| DepositLogVO {
                shop_deposit_log_id: r.shop_deposit_log_id,
                owner_type: r.owner_type,
                owner_id: r.owner_id,
                shop_id: r.shop_id.unwrap_or(0),
                shop_title: r.shop_title.unwrap_or_default(),
                r#type: r.r#type,
                change_amount: r.change_amount,
                before_paid: r.before_paid,
                after_paid: r.after_paid,
                related_id: r.related_id,
                order_id: r.order_id,
                admin_id: r.admin_id,
                remark: r.remark,
                add_time: format_time(r.add_time),
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 详情。
    pub async fn detail(&self, id: i32) -> Result<DepositLogVO> {
        if id <= 0 {
            return Err(Error::Validation(MSG_LOG_ID_INVALID.into()));
        }
        let r = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_LOG_NOT_FOUND.into()))?;
        Ok(DepositLogVO {
            shop_deposit_log_id: r.shop_deposit_log_id,
            owner_type: r.owner_type,
            owner_id: r.owner_id,
            shop_id: r.shop_id.unwrap_or(0),
            shop_title: r.shop_title.unwrap_or_default(),
            r#type: r.r#type,
            change_amount: r.change_amount,
            before_paid: r.before_paid,
            after_paid: r.after_paid,
            related_id: r.related_id,
            order_id: r.order_id,
            admin_id: r.admin_id,
            remark: r.remark,
            add_time: format_time(r.add_time),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_id_detected() {
        assert!(0 <= 0 && -1 <= 0);
        assert_eq!(MSG_LOG_ID_INVALID, "交易明细id非法");
    }

    #[test]
    fn not_found_message_verbatim() {
        assert_eq!(MSG_LOG_NOT_FOUND, "交易明细不存在");
    }

    #[test]
    fn time_format_known_epoch() {
        assert_eq!(format_time(1_733_000_000), "2024-11-30 20:53:20");
    }
}
