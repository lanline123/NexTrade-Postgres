//! 订单日志（trade::order_log）模型层 —— wave9。
//!
//! 对应 Java `com.nextrade.adminapi.controller.order.OrderLogController`
//! （`@RequestMapping("/adminapi/order/orderLog")`），2 个端点。
//! 数据表 `order_log`：`log_id` / `order_id` / `order_sn` / `admin_id` /
//! `user_id` / `description` / `log_time` / `shop_id`。

use serde::{Deserialize, Serialize};

/// 订单日志创建人不是订单本人时展示为「管理员」。
pub const OPERATOR_ADMIN: &str = "管理员";
/// 系统操作展示名。
pub const OPERATOR_SYSTEM: &str = "系统";

/// 列表查询（Java `OrderLogListDTO`：`orderId` + `keyword`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderLogListQuery {
    /// 订单 ID
    pub order_id: Option<i32>,
    /// 关键字（订单号 / 日志内容）
    pub keyword: Option<String>,
    /// 页码
    pub page: Option<u32>,
    /// 每页条数
    pub size: Option<u32>,
}

impl OrderLogListQuery {
    /// 分页（默认 1/15）。
    #[must_use]
    pub fn page_query(&self) -> crate::types::PageQuery {
        crate::types::PageQuery {
            page: self.page.unwrap_or(1).max(1) as u64,
            page_size: self.size.unwrap_or(15).max(1) as u64,
        }
    }

    /// LIMIT/OFFSET。
    #[must_use]
    pub fn limit_offset(&self) -> (i64, i64) {
        let p = self.page_query();
        (
            p.page_size as i64,
            ((p.page - 1) * p.page_size) as i64,
        )
    }
}

/// 订单日志 VO（Java `OrderLogVO`）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderLogVO {
    /// 日志 ID
    pub log_id: i32,
    /// 订单 ID
    pub order_id: i32,
    /// 订单号
    pub order_sn: String,
    /// 操作人展示名
    pub operator: String,
    /// 管理员 ID
    pub admin_id: i32,
    /// 用户 ID
    pub user_id: i32,
    /// 日志内容
    pub description: String,
    /// 操作时间（yyyy-MM-dd HH:mm:ss）
    pub log_time: String,
    /// 店铺 ID
    pub shop_id: i32,
}

/// 创建订单日志（Java `OrderLogCreateDTO`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderLogCreateDTO {
    /// 订单 ID
    pub order_id: i32,
    /// 订单号
    pub order_sn: Option<String>,
    /// 管理员 ID
    pub admin_id: Option<i32>,
    /// 用户 ID
    pub user_id: Option<i32>,
    /// 日志内容（Java `description`）
    pub description: Option<String>,
    /// 日志内容（Java 兼容字段 `orderLogDesc`）
    pub order_log_desc: Option<String>,
    /// 操作时间（秒；缺省取当前时间）
    pub log_time: Option<i64>,
    /// 店铺 ID
    pub shop_id: Option<i32>,
}

/// 时间戳（秒）→ `yyyy-MM-dd HH:mm:ss`；0 → ""。
#[must_use]
pub fn format_time(ts: i64) -> String {
    if ts <= 0 {
        return String::new();
    }
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_query_defaults() {
        let q: OrderLogListQuery = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(q.page_query().page, 1);
        assert_eq!(q.page_query().page_size, 15);
        assert_eq!(q.limit_offset(), (15, 0));
    }

    #[test]
    fn create_dto_reads_compat_fields() {
        let d: OrderLogCreateDTO = serde_json::from_value(serde_json::json!({
            "orderId": 9, "description": "发货", "logTime": 1733000000
        }))
        .unwrap();
        assert_eq!(d.order_id, 9);
        assert_eq!(d.description.as_deref(), Some("发货"));
        assert_eq!(d.log_time, Some(1_733_000_000));
    }

    #[test]
    fn time_format_zero_is_empty() {
        assert_eq!(format_time(0), "");
        assert_eq!(format_time(1_733_000_000).len(), 19);
    }
}
