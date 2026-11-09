//! 账户总览服务（payment::account_panel，1 个管理端端点）。
//!
//! `GET /adminapi/finance/accountPanel/list`：按 `searchStartDate/searchEndDate`
//! 汇总平台账户数据（Java `AccountPanelServiceImpl#list`）：
//! - balance：用户余额变动净值（change_type 1 加 / 2 减）
//! - frozenMoney：冻结余额变动净值
//! - voucherAmount：已支付充值合计（status=1）
//! - toCashAmount：已完成提现合计（status=1）
//! - surplus：订单余额支付合计（order.balance）
//! - usePoints：订单积分支付合计（order.use_points，按积分数量求和，Java 同款）
//!
//! 时间口径与 Java 一致：起始为传入时刻、结束取当日 23:59:59（`DateUtil.endOfDay`）。

use std::sync::Arc;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 账户总览查询（Java `AccountPanelDTO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AccountPanelQuery {
    pub search_start_date: Option<String>,
    pub search_end_date: Option<String>,
}

/// 账户总览（Java `AccountPanelVO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AccountPanelVO {
    pub balance: Decimal,
    pub frozen_money: Decimal,
    pub voucher_amount: Decimal,
    pub to_cash_amount: Decimal,
    pub surplus: Decimal,
    pub use_points: Decimal,
}

pub struct AccountPanelService {
    repos: Arc<Repositories>,
}

impl AccountPanelService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// `GET /adminapi/finance/accountPanel/list`：账户总览数据。
    pub async fn list(&self, q: &AccountPanelQuery) -> Result<AccountPanelVO> {
        let (start, end) = parse_range(
            q.search_start_date.as_deref(),
            q.search_end_date.as_deref(),
        )?;
        Ok(AccountPanelVO {
            balance: self.repos.payment.account_panel.balance_net(start, end).await?,
            frozen_money: self
                .repos
                .payment
                .account_panel
                .frozen_net(start, end)
                .await?,
            voucher_amount: self
                .repos
                .payment
                .account_panel
                .recharge_total(start, end)
                .await?,
            to_cash_amount: self
                .repos
                .payment
                .account_panel
                .withdraw_total(start, end)
                .await?,
            surplus: self
                .repos
                .payment
                .account_panel
                .order_total("balance", start, end)
                .await?,
            use_points: self
                .repos
                .payment
                .account_panel
                .order_total("use_points", start, end)
                .await?,
        })
    }
}

/// 日期区间解析（`yyyy-MM-dd` 或 `yyyy-MM-dd HH:mm:ss`）：
/// 起始 = 给定时刻（纯日期取 00:00:00）；结束 = 当日 23:59:59（Java `endOfDay`）。
#[must_use]
pub fn parse_range(start_raw: Option<&str>, end_raw: Option<&str>) -> Result<(i64, i64)> {
    let start = parse_start(start_raw)?;
    let end = parse_end(end_raw)?;
    if start_raw.is_some() || end_raw.is_some() {
        if start > 0 && end > 0 && start > end {
            return Err(Error::Validation("参数缺失".into()));
        }
    }
    Ok((start, end))
}

fn parse_start(raw: Option<&str>) -> Result<i64> {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(0);
    };
    parse_datetime(raw)
        .map(|ts| ts)
        .ok_or_else(|| Error::Validation("参数缺失".into()))
}

fn parse_end(raw: Option<&str>) -> Result<i64> {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(0);
    };
    let date_part: String = raw.chars().take(10).collect();
    let day_start = parse_date(&date_part).ok_or_else(|| Error::Validation("参数缺失".into()))?;
    Ok(day_start + 86_399)
}

fn parse_datetime(raw: &str) -> Option<i64> {
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S") {
        return Some(dt.and_utc().timestamp() - 8 * 3600);
    }
    if let Ok(date) = chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d") {
        if let Some(dt) = date.and_hms_opt(0, 0, 0) {
            return Some(dt.and_utc().timestamp() - 8 * 3600);
        }
    }
    None
}

fn parse_date(raw: &str) -> Option<i64> {
    chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp() - 8 * 3600)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_range_end_is_end_of_day() {
        let (start, end) = parse_range(Some("2026-10-01"), Some("2026-10-18")).unwrap();
        assert!(start > 0);
        assert_eq!(end - start, 17 * 86_400 + 86_399);
        // 东八区当日 23:59:59 = UTC 15:59:59 → epoch % 86400 = 57599
        assert_eq!(end % 86_400, 57_599);
    }

    #[test]
    fn empty_range_defaults_to_zero() {
        assert_eq!(parse_range(None, None).unwrap(), (0, 0));
        assert_eq!(parse_range(Some(""), Some("  ")).unwrap(), (0, 0));
    }

    #[test]
    fn invalid_date_rejected() {
        assert!(parse_range(Some("bad"), None).is_err());
        assert!(parse_range(None, Some("bad")).is_err());
        assert!(parse_range(Some("2026-10-18"), Some("2026-10-01")).is_err());
    }
}

// 保持 Decimal 的 serde 使用显式（避免未使用告警）
#[allow(dead_code)]
fn _decimal_type(_: Decimal) {}
