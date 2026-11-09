//! 店铺账户（merchant::shop_account_log）模型层 —— wave9。
//!
//! 对应 Java：
//! - admin `com.nextrade.adminapi.controller.shop.AccountController.index`
//!   （`GET /adminapi/merchant/shopAccount/index`）→ `AccountVO`（店铺余额汇总）
//! - public `com.nextrade.api.controller.user.AccountController`
//!   （`GET /api/user/account/list`、`GET /api/user/account/commissionList`）
//!   → 用户余额/佣金流水（`user_balance_log`，`wallet_type` 0/1）
//!
//! 说明：feature 名为 shop_account_log，但契约中的 3 条端点分别读 `shop`
//! 汇总字段与 `user_balance_log`；`shop_account_log` 表本身仅被 Java 的
//! `logList`（不在本契约）使用。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 店铺不存在。
pub const MSG_SHOP_NOT_FOUND: &str = "店铺不存在";
/// 未登录。
pub const MSG_LOGIN_REQUIRED: &str = "请先登录";

/// 店铺账户查询（Java `AccountListDTO` 的 shopId 子集）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopAccountQuery {
    /// 店铺 ID（平台账号可指定；店铺账号取请求头）
    pub shop_id: Option<i32>,
}

/// 店铺账户汇总 VO（Java `AccountVO` 子集）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopAccountVO {
    /// 店铺 ID
    pub shop_id: i32,
    /// 店铺名称
    pub shop_title: String,
    /// 店铺余额
    pub shop_money: Decimal,
    /// 冻结金额
    pub frozen_money: Decimal,
    /// 店铺状态
    pub status: i32,
}

/// 用户账户流水查询（Java `UserBalanceLogListDTO` 子集）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserAccountLogQuery {
    /// 页码
    pub page: Option<u32>,
    /// 每页条数
    pub size: Option<u32>,
    /// 关键字（变动说明）
    pub keyword: Option<String>,
    /// 钱包类型（0 余额 / 1 佣金；由端点决定，不接受前端覆盖）
    pub wallet_type: Option<i32>,
}

impl UserAccountLogQuery {
    /// 分页（默认 1/15）。
    #[must_use]
    pub fn page_query(&self) -> crate::types::PageQuery {
        crate::types::PageQuery {
            page: self.page.unwrap_or(1).max(1) as u64,
            page_size: self.size.unwrap_or(15).max(1) as u64,
        }
    }
}

/// 用户账户流水 VO（Java `UserBalanceLogVO` 子集）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserAccountLogVO {
    /// 流水 ID
    pub log_id: i32,
    /// 用户 ID
    pub user_id: i32,
    /// 变动前余额
    pub before_balance: Decimal,
    /// 本次变动金额
    pub balance: Decimal,
    /// 变动后余额
    pub after_balance: Decimal,
    /// 冻结余额
    pub frozen_balance: Decimal,
    /// 钱包类型（0 余额 / 1 佣金）
    pub wallet_type: i32,
    /// 变动说明
    pub change_desc: String,
    /// 变动类型
    pub change_type: i32,
    /// 变动时间（yyyy-MM-dd HH:mm:ss）
    pub change_time: String,
}

/// 秒级时间戳格式化（0 → ""）。
#[must_use]
pub fn format_time(ts: i32) -> String {
    if ts <= 0 {
        return String::new();
    }
    chrono::DateTime::from_timestamp(i64::from(ts), 0)
        .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_query_defaults() {
        let q: ShopAccountQuery = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(q.shop_id, None);
    }

    #[test]
    fn log_query_defaults_and_camel_case() {
        let q: UserAccountLogQuery =
            serde_json::from_value(serde_json::json!({"page":2,"size":30,"keyword":"充值"}))
                .unwrap();
        assert_eq!(q.page_query().page, 2);
        assert_eq!(q.page_query().page_size, 30);
        assert_eq!(q.keyword.as_deref(), Some("充值"));
    }

    #[test]
    fn time_format_zero_is_empty() {
        assert_eq!(format_time(0), "");
        assert_eq!(format_time(1_733_000_000).len(), 19);
    }
}
