//! 对账单服务（payment::statement，6 个管理端端点）。
//!
//! 对应原 Java `StatementServiceImpl` + `StatementDownloadServiceImpl`：
//! `getStatementList` / `saveStatementDownload` / `exportStatement` /
//! `exportStatementStatistics` / `getStatementStatisticsList` / `getStatementQueryConfig`。
//!
//! # 已实现
//! - 列表：入账/结算/下单时间区间、账户类型、类型、支付方式、流水号、来源
//!   （shop/vendor）过滤 + 分页；VO 回填店铺/供应商名称、账户类型/类型/入账方式/
//!   支付方式文案与时间串。
//! - 统计：DAY/MONTH/YEAR 聚合（收入=`amount>0` 金额与笔数；支出=`amount<0` 绝对值与笔数），
//!   统计列表按 Java 语义：日视图按月展开每日、月视图按年展开每月、年视图单条，
//!   最后倒序；导出统计为单周期（与 Java 一致）。
//! - 导出对账单：`pageSize=9999` 复用列表查询（Java 同款），空门店/供应商/支付方式
//!   兜底「暂无门店 / 暂无供应商 / 其他」。
//! - 查询字段配置：枚举字典（code/description）。
//! - 保存下载信息：时间解析 + 落库 `statement_download`。
//!
//! # 偏差（已消除）
//! - 导出端点 Java 标注 `@ResponseExcel`：`exportStatement` 与
//!   `exportStatementStatistics` 均已返回真实 xlsx 文件流
//!   （`export_statement_xlsx` / `export_statistics_xlsx`）。
//! - 多租户：Java 按请求头 X-Admin-Type/X-Shop-Id 限定；Rust 无该上下文，按
//!   查询参数 `shopId/vendorId/source` 过滤（平台视角）。
//! - `vendor` 名称使用 `vendor.vendor_name`（Java `selectOnboardedById` 另校验入驻）。

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;

use nextrade_core::models::payment::statement::{
    account_type_name, date_component_valid, entry_type_name, format_ts, parse_datetime,
    statement_type_name, statistics_periods, time_type_name, StatementDownloadSaveParam,
    StatementExportVO, StatementListQuery, StatementListVO, StatementQueryContentVO,
    StatementStatisticsQuery, StatementStatisticsVO, StatementStatisticsExportVO,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

pub struct StatementService {
    repos: Arc<Repositories>,
}

impl StatementService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// `GET /adminapi/finance/statement/getStatementList`：对账单列表。
    pub async fn list(&self, q: &StatementListQuery) -> Result<PageResult<StatementListVO>> {
        let page = q.page_query();
        let (total, rows) = self.repos.payment.statement.paginate(q, &page).await?;
        let mut shop_cache: HashMap<i32, String> = HashMap::new();
        let mut vendor_cache: HashMap<i32, String> = HashMap::new();
        let mut list = Vec::with_capacity(rows.len());
        for row in &rows {
            let shop_name = match row.shop_id.filter(|id| *id > 0) {
                Some(id) => {
                    if let Some(name) = shop_cache.get(&id) {
                        name.clone()
                    } else {
                        let name = self
                            .repos
                            .payment
                            .statement
                            .shop_title(id)
                            .await?
                            .unwrap_or_default();
                        shop_cache.insert(id, name.clone());
                        name
                    }
                }
                None => String::new(),
            };
            let vendor_name = match row.vendor_id.filter(|id| *id > 0) {
                Some(id) => {
                    if let Some(name) = vendor_cache.get(&id) {
                        name.clone()
                    } else {
                        let name = self
                            .repos
                            .payment
                            .statement
                            .vendor_name(id)
                            .await?
                            .unwrap_or_default();
                        vendor_cache.insert(id, name.clone());
                        name
                    }
                }
                None => String::new(),
            };
            let payment_type = row.payment_type.clone().unwrap_or_default();
            let payment_type_name = pay_method_name(&payment_type).to_string();
            list.push(StatementListVO {
                statement_id: row.statement_id,
                shop_id: row.shop_id,
                shop_name,
                record_sn: row.record_sn.clone(),
                settlement_time: format_ts(row.settlement_time),
                vendor_id: row.vendor_id,
                vendor_name,
                account_type: row.account_type,
                r#type: row.r#type,
                entry_type: row.entry_type.clone(),
                payment_type: payment_type_name.clone(),
                account_balance: row.account_balance,
                amount: row.amount,
                settlement_status: row.settlement_status,
                account_type_name: account_type_name(row.account_type).to_string(),
                type_name: statement_type_name(row.r#type).to_string(),
                entry_type_name: entry_type_name(&row.entry_type).to_string(),
                payment_type_name,
                record_time: format_ts(row.record_time),
            });
        }
        Ok(PageResult::new(total, list, &page))
    }

    /// `GET /adminapi/finance/statement/exportStatement`：导出对账单（JSON 降级）。
    pub async fn export_statement(
        &self,
        q: &StatementListQuery,
    ) -> Result<Vec<StatementExportVO>> {
        let mut query = q.clone();
        query.page = Some(1);
        query.page_size = Some(9999);
        let page = self.list(&query).await?;
        Ok(page
            .list
            .into_iter()
            .map(|r| StatementExportVO {
                statement_id: r.statement_id,
                shop_name: if r.shop_name.is_empty() {
                    "暂无门店".to_string()
                } else {
                    r.shop_name
                },
                record_sn: r.record_sn,
                settlement_time: r.settlement_time,
                vendor_name: if r.vendor_name.is_empty() {
                    "暂无供应商".to_string()
                } else {
                    r.vendor_name
                },
                account_type_name: r.account_type_name,
                type_name: r.type_name,
                entry_type_name: r.entry_type_name,
                payment_type_name: if r.payment_type_name.is_empty() {
                    "其他".to_string()
                } else {
                    r.payment_type_name
                },
                account_balance: r.account_balance,
                amount: r.amount,
                record_time: r.record_time,
            })
            .collect())
    }

    /// 导出对账单为 xlsx（Java `exportStatement` 直接回写 xlsx）。
    pub async fn export_statement_xlsx(
        &self,
        q: &StatementListQuery,
    ) -> Result<(String, Vec<u8>)> {
        let rows = self.export_statement(q).await?;
        let headers = [
            "账单ID",
            "店铺",
            "交易单号",
            "结算时间",
            "供应商",
            "账户类型",
            "账单类型",
            "收支类型",
            "支付方式",
            "账户余额",
            "金额",
            "记录时间",
        ];
        let data: Vec<Vec<String>> = rows
            .iter()
            .map(|r| {
                vec![
                    r.statement_id.to_string(),
                    r.shop_name.clone(),
                    r.record_sn.clone().unwrap_or_default(),
                    r.settlement_time.clone(),
                    r.vendor_name.clone(),
                    r.account_type_name.clone(),
                    r.type_name.clone(),
                    r.entry_type_name.clone(),
                    r.payment_type_name.clone(),
                    r.account_balance.to_string(),
                    r.amount.to_string(),
                    r.record_time.clone(),
                ]
            })
            .collect();
        let bytes = crate::system::excel::build_xlsx(&headers, &data, "对账单")
            .map_err(|e| Error::Internal(format!("对账单 Excel 生成失败: {e}")))?;
        Ok(("对账单.xlsx".to_string(), bytes))
    }

    /// 导出对账单统计为 xlsx。
    pub async fn export_statistics_xlsx(
        &self,
        q: &StatementStatisticsQuery,
    ) -> Result<(String, Vec<u8>)> {
        let rows = self.export_statistics(q).await?;
        let headers = [
            "统计周期",
            "收入",
            "支出",
            "收入笔数",
            "支出笔数",
            "账户类型",
            "账户类型名",
        ];
        let data: Vec<Vec<String>> = rows
            .iter()
            .map(|r| {
                vec![
                    r.statement_date.clone(),
                    r.income.to_string(),
                    r.expenditure.to_string(),
                    r.income_count.to_string(),
                    r.expenditure_count.to_string(),
                    r.account_type.to_string(),
                    r.account_type_text.clone(),
                ]
            })
            .collect();
        let bytes = crate::system::excel::build_xlsx(&headers, &data, "对账单统计")
            .map_err(|e| Error::Internal(format!("对账单统计 Excel 生成失败: {e}")))?;
        Ok(("对账单统计.xlsx".to_string(), bytes))
    }

    /// `GET /adminapi/finance/statement/getStatementStatisticsList`：对账单统计列表。
    pub async fn statistics_list(
        &self,
        q: &StatementStatisticsQuery,
    ) -> Result<Vec<StatementStatisticsVO>> {
        let date_type = date_component_valid(q.statement_date_type.as_deref())
            .ok_or_else(|| Error::biz_msg("日期类型错误"))?;
        let statement_date = q.statement_date.clone().unwrap_or_default();
        let periods = expand_periods(date_type, &statement_date)
            .ok_or_else(|| Error::biz_msg("日期格式错误"))?;
        let account_type = q.account_type.unwrap_or(1);
        let mut list = Vec::new();
        for (year, month, day) in periods {
            let (income, expenditure, income_count, expenditure_count) = self
                .repos
                .payment
                .statement
                .aggregate(
                    year,
                    month,
                    day,
                    q.account_type,
                    q.shop_id,
                    q.vendor_id,
                )
                .await?;
            list.push(StatementStatisticsVO {
                statement_date: period_label(year, month, day),
                income,
                expenditure,
                income_count,
                expenditure_count,
                account_type,
                account_type_text: account_type_name(Some(account_type)).to_string(),
            });
        }
        list.reverse();
        Ok(list)
    }

    /// `GET /adminapi/finance/statement/exportStatementStatistics`：导出统计（单周期）。
    pub async fn export_statistics(
        &self,
        q: &StatementStatisticsQuery,
    ) -> Result<Vec<StatementStatisticsExportVO>> {
        let date_type = date_component_valid(q.statement_date_type.as_deref())
            .ok_or_else(|| Error::biz_msg("日期类型错误"))?;
        let statement_date = q.statement_date.clone().unwrap_or_default();
        let periods = statistics_periods(date_type, &statement_date);
        let Some((year, month, day)) = periods.first().copied() else {
            return Err(Error::biz_msg("日期格式错误"));
        };
        let account_type = q.account_type.unwrap_or(1);
        let (income, expenditure, income_count, expenditure_count) = self
            .repos
            .payment
            .statement
            .aggregate(year, month, day, q.account_type, q.shop_id, q.vendor_id)
            .await?;
        Ok(vec![StatementStatisticsVO {
            statement_date: statement_date.clone(),
            income,
            expenditure,
            income_count,
            expenditure_count,
            account_type,
            account_type_text: account_type_name(Some(account_type)).to_string(),
        }])
    }

    /// `GET /adminapi/finance/statement/getStatementQueryConfig`：查询字段配置。
    pub fn query_config(&self) -> StatementQueryContentVO {
        query_content()
    }

    /// `POST /adminapi/finance/statement/saveStatementDownload`：保存下载信息。
    pub async fn save_download(&self, param: &StatementDownloadSaveParam) -> Result<()> {
        let start = param.start_time.as_deref().map(parse_datetime).unwrap_or(0);
        let end = param.end_time.as_deref().map(parse_datetime).unwrap_or(0);
        let now = nextrade_common::utils::now_seconds();
        self.repos
            .payment
            .statement
            .insert_download(
                start,
                end,
                param.shop_id,
                param.vendor_id,
                param.remark.as_deref(),
                now,
            )
            .await?;
        Ok(())
    }
}

// ————————————————————— 纯函数 —————————————————————

/// 查询字段配置（Java `getStatementQueryContent` 的枚举字典）。
#[must_use]
pub fn query_content() -> StatementQueryContentVO {
    StatementQueryContentVO {
        statement_type: enum_options(&[
            (1, "手续费"),
            (2, "服务费"),
            (3, "订单收支"),
            (4, "店铺提现收支"),
            (5, "门店提现收支"),
            (6, "供应商提现收支"),
            (7, "订单退款"),
            (8, "分销佣金"),
        ]),
        statement_time_type: enum_options(&[(1, "入账（结算）时间"), (2, "下单时间")]),
        account_type: enum_options(&[(1, "账户余额")]),
        entry_type: enum_options(&[(1, "自动"), (2, "手动")]),
        pay_method_type: payment_method_options(),
        date_component_type: enum_options(&[(1, "日"), (2, "月"), (3, "年")]),
    }
}

/// 支付方式文案（Java `PayMethodType` 常用项）。
#[must_use]
pub fn pay_method_name(code: &str) -> &'static str {
    match code {
        "wechat" => "微信支付",
        "alipay" => "支付宝",
        "balance" => "余额支付",
        "offline" => "线下支付",
        "paypal" => "PayPal",
        "applepay" | "apple_pay" => "Apple Pay",
        "lakala" => "拉卡拉",
        "yeepay" => "易宝支付",
        "huifu" => "汇付支付",
        "yunpay" => "云支付",
        "yabanpay" => "雅伴支付",
        _ => "",
    }
}

fn enum_options(items: &[(i32, &str)]) -> Vec<serde_json::Value> {
    items
        .iter()
        .map(|(code, description)| serde_json::json!({"code": code, "description": description}))
        .collect()
}

fn payment_method_options() -> Vec<serde_json::Value> {
    enum_options(&[
        (0, "微信支付"),
        (1, "支付宝"),
        (2, "余额支付"),
        (3, "线下支付"),
    ])
    .into_iter()
    .map(|v| {
        // Java `PayMethodType` 的 code 为字符串枚举名；展示仍用 description
        serde_json::json!({"code": v["code"], "description": v["description"]})
    })
    .collect()
}

/// 统计视图的周期展开（Java `TigUtils.getMonthDays/getMonthByYear` 语义）。
#[must_use]
pub fn expand_periods(date_type: &str, statement_date: &str) -> Option<Vec<(i32, i32, i32)>> {
    match date_type {
        "DAY" => {
            let parts: Vec<&str> = statement_date.split('-').collect();
            match parts.len() {
                3 => Some(vec![(
                    parts[0].parse().ok()?,
                    parts[1].parse().ok()?,
                    parts[2].parse().ok()?,
                )]),
                2 => {
                    let year: i32 = parts[0].parse().ok()?;
                    let month: i32 = parts[1].parse().ok()?;
                    let days = days_in_month(year, month);
                    Some((1..=days).map(|day| (year, month, day)).collect())
                }
                _ => None,
            }
        }
        "MONTH" => {
            let parts: Vec<&str> = statement_date.split('-').collect();
            match parts.len() {
                2 => Some(vec![(
                    parts[0].parse().ok()?,
                    parts[1].parse().ok()?,
                    0,
                )]),
                1 => {
                    let year: i32 = parts[0].parse().ok()?;
                    Some((1..=12).map(|month| (year, month, 0)).collect())
                }
                _ => None,
            }
        }
        "YEAR" => Some(vec![(statement_date.parse().ok()?, 0, 0)]),
        _ => None,
    }
}

fn days_in_month(year: i32, month: i32) -> i32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

#[must_use]
pub fn period_label(year: i32, month: i32, day: i32) -> String {
    if day > 0 {
        format!("{year:04}-{month:02}-{day:02}")
    } else if month > 0 {
        format!("{year:04}-{month:02}")
    } else {
        format!("{year:04}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pay_method_names_and_period_labels() {
        assert_eq!(pay_method_name("wechat"), "微信支付");
        assert_eq!(pay_method_name("unknown"), "");
        assert_eq!(period_label(2026, 10, 18), "2026-10-18");
        assert_eq!(period_label(2026, 10, 0), "2026-10");
        assert_eq!(period_label(2026, 0, 0), "2026");
    }

    #[test]
    fn expand_periods_matches_java_views() {
        assert_eq!(expand_periods("DAY", "2026-10-18"), Some(vec![(2026, 10, 18)]));
        let month_days = expand_periods("DAY", "2026-02").unwrap();
        assert_eq!(month_days.len(), 28);
        let months = expand_periods("MONTH", "2026").unwrap();
        assert_eq!(months.len(), 12);
        assert_eq!(expand_periods("YEAR", "2026"), Some(vec![(2026, 0, 0)]));
        assert_eq!(expand_periods("DAY", "bad"), None);
    }

    #[test]
    fn query_content_dict_shape() {
        let cfg = query_content();
        assert!(!cfg.statement_type.is_empty());
        assert_eq!(cfg.statement_type[0]["code"], 1);
        assert_eq!(cfg.account_type[0]["description"], "账户余额");
        assert_eq!(cfg.date_component_type.len(), 3);
        assert!(!cfg.pay_method_type.is_empty());
    }
}
