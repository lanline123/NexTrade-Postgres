//! 分销客户交易（marketing::salesman_customer_transaction）DTO / VO。
//!
//! 对应原 Java `com.nextrade.adminapi.controller.salesman.SalesmanCustomerTransactionController`
//! （`@RequestMapping("/adminapi/salesman/customerTransaction")`，1 个端点）+
//! `SalesmanOrderServiceImpl#listCustomerTransaction` +
//! `CustomerTransactionPageQuery` / `CustomerTransactionVO` /
//! `CustomerTransactionOrderVO`。
//!
//! 数据来源：`salesman_order` JOIN `order`（无独立交易表）。
//!
//! # 端点（1）
//! `GET /adminapi/salesman/customerTransaction/list`
//!
//! # 与 Java 的差异（uncertain_points）
//! 1. Java 逐单回填 `order` 全字段（83 列）+ 订单商品行 + 售后 + 店铺 +
//!    会员头像等，并支持 Excel 导出；Rust 只返回交易主字段
//!    （`salesmanId/salesmanName/orderId/orderSn/userId/username/addTime/
//!    totalAmount/paidAmount/orderStatus`），`isExport` 忽略（无 Excel 依赖）。
//! 2. 时间过滤口径：Java Mapper XML 对 `order` 的下单时间过滤；Rust 同口径
//!    （`order.add_time` 区间）。
//!
//! # 金额：`total_amount` / `paid_amount` 为 `Decimal`。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 分页默认值（Java `BasePage`）。
pub const DEFAULT_PAGE_SIZE: u64 = 15;
pub const MAX_PAGE_SIZE: u64 = 100;
pub const MSG_SALESMAN_ID_REQUIRED: &str = "salesmanId不能为空";

/// 客户交易列表查询（Java `CustomerTransactionPageQuery`）。
///
/// ⚠️ `salesmanId` **不是必填**：Java `CustomerTransactionPageQuery` 上该字段
/// 没有任何 `@NotNull`，服务层只是把它透传给 Mapper（为空则不加过滤）。
/// 原版 admin 页面 `views/salesman/customerTransactions/List.vue` 更是把
/// `salesmanId` 初始化成**空字符串 `""`** 后原样提交；Jackson 把 `""` 绑成
/// `null`，于是「全部客户的成交记录」正常返回。
///
/// Rust 早期实现：serde 严格解析 `""` → 整个请求 422/400，且服务层强制
/// `salesmanId` 必填 → 页面永远拿不到数据
/// （2026-10-21 巡检 `/admin/salesman/customer-transactions/list` 的根因）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CustomerTransactionQuery {
    /// 分销员 ID（可选；`""` / 非法串 → `None` = 不过滤）
    #[serde(default, deserialize_with = "de_opt_i32_lenient")]
    pub salesman_id: Option<i32>,
    #[serde(default, deserialize_with = "de_opt_i32_lenient")]
    pub is_export: Option<i32>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    #[serde(default, deserialize_with = "de_opt_u64_lenient")]
    pub page: Option<u64>,
    #[serde(
        rename = "pageSize",
        alias = "size",
        default,
        deserialize_with = "de_opt_u64_lenient"
    )]
    pub page_size: Option<u64>,
    pub sort_field: Option<String>,
    pub sort_order: Option<String>,
}

/// 宽松地把 JSON 值解析为 `Option<i32>`：`""` / 非数字串 / 其它类型 → `None`。
fn de_opt_i32_lenient<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(serde_json::Value::Number(n)) => n.as_i64().and_then(|v| i32::try_from(v).ok()),
        Some(serde_json::Value::String(s)) => s.trim().parse::<i32>().ok(),
        _ => None,
    })
}

/// 宽松地把 JSON 值解析为 `Option<u64>`（用于 `page` / `size`）。
fn de_opt_u64_lenient<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(serde_json::Value::Number(n)) => n.as_u64(),
        Some(serde_json::Value::String(s)) => s.trim().parse::<u64>().ok(),
        _ => None,
    })
}

/// 客户交易行（Java `CustomerTransactionOrderVO` 精简子集）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerTransactionVO {
    pub salesman_id: i32,
    pub salesman_name: Option<String>,
    pub order_id: i32,
    pub order_sn: String,
    pub user_id: i32,
    pub username: Option<String>,
    pub add_time: String,
    pub total_amount: Option<Decimal>,
    pub paid_amount: Option<Decimal>,
    pub order_status: Option<i32>,
}

/// `yyyy-MM-dd` / `yyyy-MM-dd HH:mm:ss` → Unix 秒（东八区；非法 → None）。
#[must_use]
pub fn parse_time(raw: Option<&str>) -> Option<i64> {
    use chrono::TimeZone;
    let text = raw.map(str::trim).filter(|s| !s.is_empty())?;
    let tz = chrono::FixedOffset::east_opt(8 * 3600)?;
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S") {
        return tz.from_local_datetime(&dt).single().map(|v| v.timestamp());
    }
    let date = chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()?;
    tz.from_local_datetime(&date.and_hms_opt(0, 0, 0)?)
        .single()
        .map(|v| v.timestamp())
}

/// Unix 秒 → `yyyy-MM-dd HH:mm:ss`（东八区；0/None → 空串）。
#[must_use]
pub fn format_time(secs: Option<i32>) -> String {
    use chrono::TimeZone;
    let Some(secs) = secs else {
        return String::new();
    };
    if secs == 0 {
        return String::new();
    }
    chrono::FixedOffset::east_opt(8 * 3600)
        .and_then(|tz| tz.timestamp_opt(i64::from(secs), 0).single())
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_helpers_match_java() {
        assert_eq!(parse_time(Some("2023-11-15")), Some(1_699_977_600));
        assert_eq!(parse_time(Some("2023-11-15 06:13:20")), Some(1_700_000_000));
        assert_eq!(parse_time(Some("bad")), None);
        assert_eq!(format_time(Some(1_700_000_000)), "2023-11-15 06:13:20");
        assert_eq!(format_time(None), "");
    }

    #[test]
    fn query_parses_camel_case() {
        let q: CustomerTransactionQuery = serde_json::from_str(
            r#"{"salesmanId":3,"isExport":1,"startTime":"2023-11-01","endTime":"2023-11-30","size":20}"#,
        )
        .unwrap();
        assert_eq!(q.salesman_id, Some(3));
        assert_eq!(q.is_export, Some(1));
        assert_eq!(q.page_size, Some(20));
    }

    #[test]
    fn vo_serializes_camel_case_money_as_string() {
        let vo = CustomerTransactionVO {
            salesman_id: 3,
            order_id: 9,
            order_sn: "SN9".into(),
            user_id: 5,
            total_amount: Some("100.00".parse().unwrap()),
            paid_amount: Some("99.00".parse().unwrap()),
            ..Default::default()
        };
        let json = serde_json::to_value(&vo).unwrap();
        assert_eq!(json["salesmanId"], 3);
        assert_eq!(json["totalAmount"], "100.00");
        assert_eq!(json["paidAmount"], "99.00");
    }
}
