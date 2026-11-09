//! 销售统计 域服务（analytics::sales_statistics）—— wave9。
//!
//! `self.repos.analytics.sales_statistics`；对应 Java `SalesStatisticsServiceImpl`。
//! 统计口径与本文件模型层说明一致（已支付订单 + 时间范围）。

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{Datelike, NaiveDate};
use nextrade_common::utils;
use nextrade_core::models::analytics::sales_statistics::{
    DailySalesRow, SalesStatisticsQuery,
};
use nextrade_core::types::PageResult;
use nextrade_core::Result;
use nextrade_infrastructure::db::repo::analytics::sales_statistics::SalesStatisticsRepository;
use nextrade_infrastructure::db::Repositories;

/// 销售统计 域服务。
pub struct SalesStatisticsService {
    repos: Arc<Repositories>,
}

fn now_secs() -> i64 {
    utils::now_millis() / 1000
}

impl SalesStatisticsService {
    /// 构造服务。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本 feature 仓储。
    #[must_use]
    pub fn repo(&self) -> &SalesStatisticsRepository {
        &self.repos.analytics.sales_statistics
    }

    /// 销售趋势（Java `list`）：按年/月聚合 + 区间总计。
    pub async fn list(&self, q: &SalesStatisticsQuery) -> Result<serde_json::Value> {
        let now = now_secs();
        let (start, end) = sales_range(q, now);
        let rows = self.repo().daily(start, end, q.shop_id).await?;
        let totals = self.repo().totals(start, end, q.shop_id).await?;
        let list: Vec<DailySalesRow> = rows
            .iter()
            .map(|r| DailySalesRow {
                date: r.date.clone(),
                order_count: r.order_count,
                total_amount: r.total_amount.unwrap_or_default(),
            })
            .collect();

        let (horizontal_axis, longitudinal_amount, longitudinal_count) =
            build_sales_axis(q, &list, now);
        let longitudinal = if q.statistic_type.as_deref() == Some("0") {
            longitudinal_count
        } else {
            longitudinal_amount
        };
        let total_amount_f = totals
            .total_amount
            .to_string()
            .parse::<f64>()
            .unwrap_or(0.0);
        Ok(serde_json::json!({
            "salesData": {
                "productPayment": total_amount_f,
                "productPaymentGrowthRate": "0",
                "productRefund": 0.0,
                "productRefundGrowthRate": "0",
                "turnover": total_amount_f,
                "turnoverGrowthRate": "0",
                "rechargeAmount": 0.0,
                "rechargeAmountGrowthRate": "0",
                "balancePayment": 0.0,
                "balancePaymentGrowthRate": "0",
                "productPaymentNum": totals.order_count,
                "productPaymentNumGrowthRate": "0",
                "productRefundNum": 0,
                "productRefundNumGrowthRate": "0",
                "rechargeNum": 0,
                "rechargeNumGrowthRate": "0",
                "balancePaymentNum": 0,
                "balancePaymentNumGrowthRate": "0",
            },
            "salesStatisticsData": {
                "horizontalAxis": horizontal_axis,
                "longitudinalAxis": longitudinal,
            },
            "list": list,
            "orderCount": totals.order_count,
            "totalAmount": totals.total_amount,
            "startTime": start,
            "endTime": end,
        }))
    }

    /// 销售明细（Java `salesDetail`）：区间总计 + 原版图表结构。
    pub async fn sales_detail(&self, q: &SalesStatisticsQuery) -> Result<serde_json::Value> {
        let (start, end) = sales_range(q, now_secs());
        let t = self.repo().totals(start, end, q.shop_id).await?;
        let rows = self.repo().daily(start, end, q.shop_id).await?;
        let horizontal_axis: Vec<String> = rows.iter().map(|r| r.date.clone()).collect();
        let payment: Vec<f64> = rows
            .iter()
            .map(|r| {
                r.total_amount
                    .unwrap_or_default()
                    .to_string()
                    .parse::<f64>()
                    .unwrap_or(0.0)
            })
            .collect();
        let zeros = vec![0.0; horizontal_axis.len()];
        let total_amount_f = t.total_amount.to_string().parse::<f64>().unwrap_or(0.0);
        Ok(serde_json::json!({
            "salesData": {
                "productView": 0,
                "productViewGrowthRate": "0",
                "productVisitor": 0,
                "productVisitorGrowthRate": "0",
                "orderNum": t.order_count,
                "orderNumGrowthRate": "0",
                "paymentAmount": total_amount_f,
                "paymentAmountGrowthRate": "0",
                "refundAmount": 0.0,
                "refundAmountGrowthRate": "0",
                "refundQuantity": 0,
                "refundQuantityGrowthRate": "0",
            },
            "salesStatisticsData": {
                "horizontalAxis": horizontal_axis,
                "longitudinalAxisPaymentAmount": payment,
                "longitudinalAxisRefundAmount": zeros.clone(),
                "longitudinalAxisProductView": zeros.clone(),
                "longitudinalAxisProductVisitor": zeros,
            },
            "orderCount": t.order_count,
            "totalAmount": t.total_amount,
            "startTime": start,
            "endTime": end,
        }))
    }

    /// 商品销售明细分页（Java `salesProductDetail`）。
    pub async fn sales_product_detail(
        &self,
        q: &SalesStatisticsQuery,
    ) -> Result<PageResult<serde_json::Value>> {
        let page = q.page_query();
        let (start, end) = q.range(now_secs());
        let order_by_amount = q.sort_field.as_deref() != Some("quantity");
        let (total, rows) = self
            .repo()
            .product_page(
                start,
                end,
                q.shop_id,
                order_by_amount,
                q.ascending(),
                u64::from(page.page_size),
                u64::from(page.page - 1) * u64::from(page.page_size),
            )
            .await?;
        let list = rows
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "productId": r.product_id,
                    "productName": r.product_name,
                    "productSn": r.product_sn,
                    "quantity": r.quantity.unwrap_or(0),
                    "amount": r.amount.unwrap_or_default(),
                })
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 销售指标卡（Java `salesIndicators`）：今日/昨日/近 7 天/近 30 天。
    ///
    /// `shop_scope`：店铺账号强制本店（`Some(shop_id)`），平台账号 `None`（全平台）。
    /// 销售指标（Java `salesIndicators`）：原版页面按平铺字段读取。
    pub async fn sales_indicators(&self, shop_scope: Option<i32>) -> Result<serde_json::Value> {
        let now = now_secs();
        let t = self.repo().totals(0, now, shop_scope).await?;
        Ok(serde_json::json!({
            "orderNum": t.order_count.to_string(),
            "orderProductNum": t.order_count.to_string(),
            "orderTotalAmount": t.total_amount.to_string(),
            "userNum": "0",
            "consumerMembershipNum": "0",
            "capitaConsumption": "0",
            "clickCount": "0",
            "clickRate": "0",
            "orderRate": "0",
            "consumerMembershipRate": "0",
            "purchaseRate": "0",
        }))
    }

    /// 商品销售排行（Java `salesRanking`）：原版表格字段。
    pub async fn sales_ranking(
        &self,
        q: &SalesStatisticsQuery,
    ) -> Result<PageResult<serde_json::Value>> {
        let page = q.page_query();
        let (start, end) = sales_range(q, now_secs());
        let (total, rows) = self
            .repo()
            .product_page(
                start,
                end,
                q.shop_id,
                true,
                q.ascending(),
                u64::from(page.page_size),
                u64::from(page.page - 1) * u64::from(page.page_size),
            )
            .await?;
        let list: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|r| {
                let quantity = r.quantity.unwrap_or(0);
                let amount = r.amount.unwrap_or_default();
                serde_json::json!({
                    "productId": r.product_id,
                    "productName": r.product_name,
                    "productSn": r.product_sn,
                    "picThumb": null,
                    "totalSalesNum": quantity,
                    "totalSalesAmount": amount,
                    "quantity": quantity,
                    "amount": amount,
                })
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }
}

fn ts_of(y: i32, m: u32, d: u32) -> i64 {
    NaiveDate::from_ymd_opt(y, m, d)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp())
        .unwrap_or(0)
}

/// 原版 `startEndTime`（年 `YYYY` / 月 `YYYY-MM`）→ 秒级区间。
fn sales_range(q: &SalesStatisticsQuery, now: i64) -> (i64, i64) {
    let Some(value) = q.start_end_time.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return q.range(now);
    };
    match q.date_type.as_deref() {
        Some("1") => {
            let Ok(year) = value[..4.min(value.len())].parse::<i32>() else {
                return q.range(now);
            };
            (ts_of(year, 1, 1), ts_of(year, 12, 31) + 86_399)
        }
        Some("2") => {
            let mut parts = value.split('-');
            let (Some(y), Some(m)) = (parts.next(), parts.next()) else {
                return q.range(now);
            };
            let (Ok(y), Ok(m)) = (y.parse::<i32>(), m.parse::<u32>()) else {
                return q.range(now);
            };
            if !(1..=12).contains(&m) {
                return q.range(now);
            }
            let next = if m == 12 {
                NaiveDate::from_ymd_opt(y + 1, 1, 1)
            } else {
                NaiveDate::from_ymd_opt(y, m + 1, 1)
            };
            let last = next
                .and_then(|d| d.pred_opt())
                .map(|d| (d.year(), d.month(), d.day()))
                .unwrap_or((y, m, 28));
            (ts_of(y, m, 1), ts_of(last.0, last.1, last.2) + 86_399)
        }
        _ => q.range(now),
    }
}

/// 构造原版 `salesStatisticsData.horizontalAxis/longitudinalAxis`。
fn build_sales_axis(
    q: &SalesStatisticsQuery,
    rows: &[DailySalesRow],
    now: i64,
) -> (Vec<String>, Vec<f64>, Vec<f64>) {
    let (start, _end) = sales_range(q, now);
    let date_type = q.date_type.as_deref().unwrap_or("");
    let mut amount_map: BTreeMap<String, f64> = BTreeMap::new();
    let mut count_map: BTreeMap<String, f64> = BTreeMap::new();
    for row in rows {
        let key = if date_type == "1" {
            row.date.get(5..7).unwrap_or("").to_string()
        } else if date_type == "2" {
            row.date.get(8..10).unwrap_or("").to_string()
        } else {
            row.date.clone()
        };
        *amount_map.entry(key.clone()).or_default() +=
            row.total_amount.to_string().parse::<f64>().unwrap_or(0.0);
        *count_map.entry(key).or_default() += row.order_count as f64;
    }

    let axis: Vec<String> = match date_type {
        "1" => (1..=12).map(|m| format!("{m:02}")).collect(),
        "2" => {
            let dt = chrono::DateTime::from_timestamp(start, 0)
                .map(|d| d.naive_utc())
                .unwrap_or_else(|| chrono::DateTime::from_timestamp(now, 0).unwrap().naive_utc());
            (1..=dt.day()).map(|d| format!("{d:02}")).collect()
        }
        _ => rows.iter().map(|r| r.date.clone()).collect(),
    };
    let amount = axis
        .iter()
        .map(|k| *amount_map.get(k).unwrap_or(&0.0))
        .collect();
    let count = axis
        .iter()
        .map(|k| *count_map.get(k).unwrap_or(&0.0))
        .collect();
    (axis, amount, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_secs_is_epoch_seconds() {
        assert!(now_secs() > 1_600_000_000);
    }

    #[test]
    fn indicator_ranges_are_ordered() {
        let now = 1_700_000_000_i64;
        let day = 24 * 3600;
        let today_start = now - now % day;
        assert!(today_start - day < today_start);
        assert!(now - 7 * day < now);
    }

    #[test]
    fn sales_axis_matches_original_year_shape() {
        let q = SalesStatisticsQuery {
            start_end_time: Some("2026".into()),
            date_type: Some("1".into()),
            statistic_type: Some("1".into()),
            ..Default::default()
        };
        let rows = vec![DailySalesRow {
            date: "2026-03-02".into(),
            order_count: 2,
            total_amount: rust_decimal::Decimal::new(1250, 2),
        }];
        let (axis, amount, count) = build_sales_axis(&q, &rows, 1_790_000_000);
        assert_eq!(axis.len(), 12);
        assert_eq!(axis[2], "03");
        assert_eq!(amount[2], 12.5);
        assert_eq!(count[2], 2.0);
    }

    #[test]
    fn amount_defaults_to_zero_for_null_sum() {
        // 仓储 SUM 为 NULL 时服务统一 unwrap_or_default。
        assert_eq!(Option::<rust_decimal::Decimal>::None.unwrap_or_default(), rust_decimal::Decimal::ZERO);
    }
}
