//! 分销概览域服务（marketing::overview）。
//!
//! Java `OverviewController`（2 端点）→ `OverviewServiceImpl`。
//!
//! # 偏差
//!
//! 1. `coreTrend` 横轴按 `dateType`（1 月/2 日/3 小时/其它逐日）生成；
//!    Java 无分销员时 `longitudinalAxis` 返回空数组，Rust 同。
//! 2. `coreSummary`：`summaryType == 0` 视为今日、其余为累计（Java 语义）。
//!    今日销售额/客户数用 `salesman_order.order_amount` / 订单去重用户；
//!    累计销售额用 `salesman.sale_amount`；佣金 `status=1`（今日再按
//!    `settlement_time` 过滤）。
//! 3. `startEndTime` 解析失败/缺省回退今天（Java 可能 NPE）。

use std::sync::Arc;

use rust_decimal::Decimal;

use nextrade_core::models::marketing::overview::{
    bucket_label, day_range_secs, horizontal_axis, is_today, parse_start_end, range_date,
    OverviewCoreSummaryVO, OverviewCoreTrendVO, OverviewSummaryQuery, OverviewTrendQuery,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::overview::OverviewRepository;
use nextrade_infrastructure::db::Repositories;

/// 分销概览域服务。
pub struct OverviewService {
    repos: Arc<Repositories>,
}

impl OverviewService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &OverviewRepository {
        &self.repos.marketing.overview
    }

    /// 趋势（Java `coreTrend`）。
    pub async fn core_trend(
        &self,
        q: &OverviewTrendQuery,
        shop_id: Option<i32>,
    ) -> Result<OverviewCoreTrendVO> {
        let date_type = q.date_type.unwrap_or(0);
        let (start, end) = parse_start_end(q.start_end_time.as_deref());
        let (start, end) = range_date(date_type, start, end);
        let axis = horizontal_axis(date_type, start, end);
        let (start_ts, end_ts) = day_range_secs(start, end);
        let salesman_ids = self.repo().scoped_salesman_ids(shop_id).await?;
        if salesman_ids.is_empty() {
            return Ok(OverviewCoreTrendVO {
                horizontal_axis: axis,
                longitudinal_axis: Vec::new(),
            });
        }
        let rows = self
            .repo()
            .trend_rows(&salesman_ids, start_ts, end_ts)
            .await?;
        let mut buckets: std::collections::HashMap<String, Decimal> = std::collections::HashMap::new();
        for (time, amount) in rows {
            let label = bucket_label(date_type, i64::from(time));
            *buckets.entry(label).or_insert(Decimal::ZERO) += amount;
        }
        let longitudinal_axis = axis
            .iter()
            .map(|label| buckets.get(label).copied().unwrap_or(Decimal::ZERO))
            .collect();
        Ok(OverviewCoreTrendVO {
            horizontal_axis: axis,
            longitudinal_axis,
        })
    }

    /// 汇总（Java `coreSummary`）。
    pub async fn core_summary(
        &self,
        q: &OverviewSummaryQuery,
        shop_id: Option<i32>,
    ) -> Result<OverviewCoreSummaryVO> {
        let today = is_today(q.summary_type);
        let today_range = if today {
            let now = chrono::Utc::now().naive_utc() + chrono::Duration::hours(8);
            let date = now.date();
            Some(day_range_secs(date, date))
        } else {
            None
        };
        let new_salesman_count = self.repo().count_salesmen(shop_id, today_range).await?;
        let salesman_ids = self.repo().scoped_salesman_ids(shop_id).await?;
        let (salesman_amount, custom_num, salesman_commission) = match today_range {
            Some((start, end)) => (
                self.repo()
                    .sum_order_amount(&salesman_ids, start, end)
                    .await?,
                self.repo()
                    .count_customers(&salesman_ids, Some(start), Some(end))
                    .await?,
                self.repo()
                    .sum_commission(&salesman_ids, Some(start), Some(end))
                    .await?,
            ),
            None => (
                self.repo().sum_sale_amount(shop_id).await?,
                self.repo().count_customers(&salesman_ids, None, None).await?,
                self.repo().sum_commission(&salesman_ids, None, None).await?,
            ),
        };
        Ok(OverviewCoreSummaryVO {
            new_salesman_count,
            salesman_amount,
            custom_num,
            salesman_commission,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::marketing::overview::{
        bucket_label, day_range_secs, horizontal_axis, parse_start_end,
    };

    #[test]
    fn trend_axis_and_buckets() {
        let (start, end) = parse_start_end(Some("2026-02-10,2026-02-12"));
        let axis = horizontal_axis(4, start, end);
        assert_eq!(axis.len(), 3);
        let (s, e) = day_range_secs(start, end);
        assert!(e > s);
        assert_eq!(bucket_label(4, s), "2026-02-10");
    }

    #[test]
    fn summary_flags() {
        assert!(is_today(Some(0)));
        assert!(!is_today(Some(1)));
    }
}
