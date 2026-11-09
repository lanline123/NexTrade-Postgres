//! 店铺保证金交易明细（merchant::deposit_log）模型层 —— wave9。
//!
//! 对应 Java `com.nextrade.adminapi.controller.deposit.DepositLogController`
//! （`@RequestMapping("/adminapi/deposit/log")`），2 个端点。
//! 表 `shop_deposit_log`（平台端全局明细）：`shop_deposit_log_id` / `owner_type` /
//! `owner_id` / `type` / `change_amount` / `before_paid` / `after_paid` /
//! `related_id` / `order_id` / `admin_id` / `remark` / `add_time`。

use serde::{Deserialize, Serialize};

/// 交易明细 id 非法（Java `GlobalException("交易明细id非法")`）。
pub const MSG_LOG_ID_INVALID: &str = "交易明细id非法";
/// 交易明细不存在（Java `GlobalException("交易明细不存在")`）。
pub const MSG_LOG_NOT_FOUND: &str = "交易明细不存在";

/// 列表查询（Java `DepositLogListDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositLogListQuery {
    /// 归属主体类型（0 平台 / 1 店铺 / 2 供应商）
    pub owner_type: Option<i32>,
    /// 主体类型（兼容字段）
    pub subject_type: Option<i32>,
    /// 收款账户类型
    pub payee_account_type: Option<i32>,
    /// 归属主体名称
    pub owner_name: Option<String>,
    /// 店铺名称
    pub shop_title: Option<String>,
    /// 交易单号
    pub trade_no: Option<String>,
    /// 商户名称
    pub merchant_name: Option<String>,
    /// 变动类型
    pub r#type: Option<i32>,
    /// 时间维度（day/week/month）
    pub date_type: Option<String>,
    /// 开始时间（秒）
    pub start_time: Option<i64>,
    /// 结束时间（秒）
    pub end_time: Option<i64>,
    /// 页码
    pub page: Option<u32>,
    /// 每页条数
    pub size: Option<u32>,
}

impl DepositLogListQuery {
    /// 分页（默认 1/15）。
    #[must_use]
    pub fn page_query(&self) -> crate::types::PageQuery {
        crate::types::PageQuery {
            page: self.page.unwrap_or(1).max(1) as u64,
            page_size: self.size.unwrap_or(15).max(1) as u64,
        }
    }
}

/// 交易明细 VO（Java `DepositLogVO` 的子集，金额 Decimal）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositLogVO {
    /// 明细 ID
    pub shop_deposit_log_id: i32,
    /// 归属主体类型
    pub owner_type: i32,
    /// 归属主体 ID
    pub owner_id: i32,
    /// 店铺 ID
    pub shop_id: i32,
    /// 店铺名称
    pub shop_title: String,
    /// 变动类型
    pub r#type: i32,
    /// 变动金额
    pub change_amount: rust_decimal::Decimal,
    /// 变动前保证金
    pub before_paid: rust_decimal::Decimal,
    /// 变动后保证金
    pub after_paid: rust_decimal::Decimal,
    /// 关联 ID
    pub related_id: i32,
    /// 订单 ID
    pub order_id: i32,
    /// 管理员 ID
    pub admin_id: i32,
    /// 备注
    pub remark: String,
    /// 变动时间（yyyy-MM-dd HH:mm:ss）
    pub add_time: String,
}

/// 秒级时间戳格式化（0 → ""）。
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
        let q: DepositLogListQuery = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(q.page_query().page, 1);
        assert_eq!(q.page_query().page_size, 15);
    }

    #[test]
    fn list_query_reads_type_keyword() {
        let q: DepositLogListQuery =
            serde_json::from_value(serde_json::json!({"ownerType":1,"type":2})).unwrap();
        assert_eq!(q.owner_type, Some(1));
        assert_eq!(q.r#type, Some(2));
    }

    #[test]
    fn time_format_zero_is_empty() {
        assert_eq!(format_time(0), "");
        assert_eq!(format_time(1_733_000_000).len(), 19);
    }
}
