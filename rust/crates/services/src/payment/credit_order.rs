//! 账期订单服务（payment::credit_order，1 个管理端端点）。
//!
//! `GET /adminapi/credit/order/list`：账期支付订单分页（Java `CreditOrderServiceImpl#list`）。
//!
//! 过滤：订单号 ILIKE、会员 ID、用户名/手机号（子查询 user）、订单状态（子查询 order）、
//! 结算状态（单值/列表）、回款流水单号 ILIKE、结算记录 ID、下单时间区间；按 `order_time` 倒序。
//! VO 回填：用户名/手机号、企业名称、订单金额/状态文案、已退款金额、结算状态文案、
//! 下单/结算时间字符串。
//!
//! # 偏差
//! - 已退款金额：Java 通过售后单（`aftersales`）退款金额并处理运费退还；
//!   本实现汇总财务审核通过（`refund_apply.refund_status=2`）的
//!   `online_balance+offline_balance+refund_balance`，不含运费拆分调整。
//! - 订单状态文案复用 canonical `0待支付 1待发货 2已发货 3已取消 4无效 5已完成`。

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use nextrade_core::types::PageResult;
use nextrade_core::Result;
use nextrade_infrastructure::db::Repositories;
use nextrade_infrastructure::db::repo::payment::credit_order::CreditOrderFilter;

/// 账期订单列表查询（Java `CreditOrderListDTO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CreditOrderListQuery {
    pub page: Option<u64>,
    #[serde(rename = "pageSize", alias = "size")]
    pub page_size: Option<u64>,
    pub sort_field: Option<String>,
    pub sort_order: Option<String>,
    pub order_sn: Option<String>,
    pub user_id: Option<i32>,
    pub username: Option<String>,
    pub mobile: Option<String>,
    pub order_status: Option<i32>,
    pub settlement_status: Option<i32>,
    pub settlement_status_list: Option<Vec<i32>>,
    pub settlement_sn: Option<String>,
    pub settlement_id: Option<i32>,
    pub order_time_start: Option<String>,
    pub order_time_end: Option<String>,
}

impl CreditOrderListQuery {
    #[must_use]
    pub fn page_query(&self) -> nextrade_core::types::PageQuery {
        nextrade_core::types::PageQuery {
            page: self.page.unwrap_or(1).max(1),
            page_size: self.page_size.unwrap_or(15).clamp(1, 200),
        }
    }

    #[must_use]
    pub fn to_filter(&self) -> CreditOrderFilter {
        CreditOrderFilter {
            order_sn: self.order_sn.clone(),
            user_id: self.user_id,
            username: self.username.clone(),
            mobile: self.mobile.clone(),
            order_status: self.order_status,
            settlement_status: self.settlement_status,
            settlement_status_list: self.settlement_status_list.clone(),
            settlement_sn: self.settlement_sn.clone(),
            settlement_id: self.settlement_id,
            order_time_start: self
                .order_time_start
                .as_deref()
                .and_then(parse_day_start),
            order_time_end: self.order_time_end.as_deref().and_then(parse_day_end),
            shop_scope: None,
        }
    }
}

/// 账期订单 VO（Java `CreditOrderVO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CreditOrderVO {
    pub credit_order_id: i32,
    pub order_id: i32,
    pub order_sn: String,
    pub user_id: i32,
    pub username: String,
    pub mobile: String,
    pub company_name: String,
    pub order_amount: Decimal,
    pub credit_amount: Decimal,
    pub refunded_amount: Decimal,
    pub order_status: i32,
    pub order_status_text: String,
    pub settlement_status: i32,
    pub settlement_status_text: String,
    pub settlement_sn: String,
    pub order_time: String,
    pub settlement_time: String,
}

pub struct CreditOrderService {
    repos: Arc<Repositories>,
}

impl CreditOrderService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// `GET /adminapi/credit/order/list`：账期订单分页。
    /// `shop_scope`：平台 `None`（全平台），店铺 `Some(shop_id)`（仅本店会员）。
    pub async fn list(
        &self,
        q: &CreditOrderListQuery,
        shop_scope: Option<i32>,
    ) -> Result<PageResult<CreditOrderVO>> {
        let page = q.page_query();
        let mut filter = q.to_filter();
        filter.shop_scope = shop_scope;
        let (total, rows) = self
            .repos
            .payment
            .credit_order
            .paginate(&filter, &page)
            .await?;
        if rows.is_empty() {
            return Ok(PageResult::new(total, Vec::new(), &page));
        }
        let user_ids: Vec<i32> = rows.iter().map(|r| r.user_id).collect();
        let order_ids: Vec<i32> = rows.iter().map(|r| r.order_id).collect();
        let users = self
            .repos
            .payment
            .credit_order
            .find_users(&dedup(user_ids))
            .await?;
        let companies = self
            .repos
            .payment
            .credit_order
            .find_companies(&dedup(rows.iter().map(|r| r.user_id).collect()))
            .await?;
        let orders = self
            .repos
            .payment
            .credit_order
            .find_orders(&dedup(order_ids))
            .await?;
        let refunded = self
            .repos
            .payment
            .credit_order
            .refunded_amount_map(&dedup(rows.iter().map(|r| r.order_id).collect()))
            .await?;
        let list = rows
            .iter()
            .map(|row| {
                let (username, mobile) = users.get(&row.user_id).cloned().unwrap_or_default();
                let (order_amount, order_status_code) = orders
                    .get(&row.order_id)
                    .copied()
                    .unwrap_or((Decimal::ZERO, 0));
                CreditOrderVO {
                    credit_order_id: row.credit_order_id,
                    order_id: row.order_id,
                    order_sn: row.order_sn.clone(),
                    user_id: row.user_id,
                    username,
                    mobile,
                    company_name: companies.get(&row.user_id).cloned().unwrap_or_default(),
                    order_amount,
                    credit_amount: row.credit_amount,
                    refunded_amount: refunded
                        .get(&row.order_id)
                        .copied()
                        .unwrap_or(Decimal::ZERO),
                    order_status: order_status_code,
                    order_status_text: order_status_text(order_status_code).to_string(),
                    settlement_status: row.settlement_status,
                    settlement_status_text: settlement_status_text(row.settlement_status)
                        .to_string(),
                    settlement_sn: row.settlement_sn.clone().unwrap_or_default(),
                    order_time: format_ts(row.order_time),
                    settlement_time: format_ts(row.settlement_time),
                }
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }
}

// ————————————————————— 纯函数 —————————————————————

/// 结算状态文案（Java `CreditOrderSettlementStatusEnum`）。
#[must_use]
pub fn settlement_status_text(code: i32) -> &'static str {
    match code {
        0 => "待结算",
        1 => "已结算",
        2 => "已取消",
        3 => "退款中",
        _ => "",
    }
}

/// 订单状态文案（canonical `0待支付 1待发货 2已发货 3已取消 4无效 5已完成`）。
#[must_use]
pub fn order_status_text(code: i32) -> &'static str {
    match code {
        0 => "待支付",
        1 => "待发货",
        2 => "已发货",
        3 => "已取消",
        4 => "无效",
        5 => "已完成",
        _ => "",
    }
}

/// 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`（UTC+8）；空/0 → 空串。
#[must_use]
pub fn format_ts(ts: Option<i64>) -> String {
    let Some(ts) = ts.filter(|v| *v > 0) else {
        return String::new();
    };
    match chrono::DateTime::from_timestamp(ts, 0) {
        Some(utc) => (utc.naive_utc() + chrono::Duration::seconds(8 * 3600))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        None => String::new(),
    }
}

fn parse_day_start(raw: &str) -> Option<i64> {
    parse_date(raw).map(|ts| ts)
}

fn parse_day_end(raw: &str) -> Option<i64> {
    parse_date(raw).map(|ts| ts + 86_399)
}

fn parse_date(raw: &str) -> Option<i64> {
    let date_part: String = raw.trim().chars().take(10).collect();
    chrono::NaiveDate::parse_from_str(&date_part, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp() - 8 * 3600)
}

fn dedup(mut ids: Vec<i32>) -> Vec<i32> {
    ids.sort_unstable();
    ids.dedup();
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settlement_status_texts_match_java_enum() {
        assert_eq!(settlement_status_text(0), "待结算");
        assert_eq!(settlement_status_text(1), "已结算");
        assert_eq!(settlement_status_text(2), "已取消");
        assert_eq!(settlement_status_text(3), "退款中");
        assert_eq!(settlement_status_text(9), "");
    }

    #[test]
    fn order_status_texts_follow_canonical() {
        assert_eq!(order_status_text(0), "待支付");
        assert_eq!(order_status_text(1), "待发货");
        assert_eq!(order_status_text(5), "已完成");
    }

    #[test]
    fn time_filter_parses_day_bounds() {
        let start = parse_day_start("2026-10-18").unwrap();
        let end = parse_day_end("2026-10-18").unwrap();
        assert_eq!(end - start, 86_399);
        assert!(parse_day_start("bad").is_none());
        assert_eq!(format_ts(None), "");
        assert_eq!(format_ts(Some(0)), "");
    }
}
