//! 前台充值域（payment::recharge_order）DTO / VO / 纯函数。
//!
//! 对应 Java `com.nextrade.api.controller.user.RechargeOrderController`（7 端点，
//! 契约 `auth=optional`）→ `UserRechargeOrderServiceImpl` 的 client 部分 +
//! `RechargeSettingServiceImpl#settingPage` + `PaymentServiceImpl#getAvailablePayment`。
//!
//! 表：`recharge_setting`（5 列）、`user_recharge_order`、`user_withdraw_apply`、
//! `paylog`、`user`。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 充值状态文案（i18n `recharge.status.*` 的 zh 值）。
#[must_use]
pub fn recharge_status_text(status: i32) -> &'static str {
    match status {
        0 => "待确认",
        1 => "已支付",
        2 => "无效",
        _ => "",
    }
}

/// 提现状态文案（i18n `withdraw.status.*` 的 zh 值）。
#[must_use]
pub fn withdraw_status_text(status: i32) -> &'static str {
    match status {
        0 => "待处理",
        1 => "已完成",
        2 => "拒绝申请",
        _ => "",
    }
}

/// 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`（UTC+8），`0` ⇒ `""`。
#[must_use]
pub fn format_time(secs: i32) -> String {
    if secs == 0 {
        return String::new();
    }
    let Some(utc) = chrono::DateTime::from_timestamp(i64::from(secs), 0) else {
        return String::new();
    };
    (utc.naive_utc() + chrono::Duration::seconds(8 * 3600))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

/// `(LIMIT, OFFSET)`（BasePage 口径：默认 15、上限 100）。
#[must_use]
pub fn limit_offset(page: Option<i64>, size: Option<i64>) -> (i64, i64) {
    let page = page.unwrap_or(1).max(1);
    let size = match size {
        None => 15,
        Some(v) if v > 100 => 100,
        Some(v) if v < 1 => 1,
        Some(v) => v,
    };
    (size, (page - 1) * size)
}

// 文案（i18n zh 值）
pub const MSG_PARAM_MISSING: &str = "参数缺失";
pub const MSG_RECHARGE_NOT_EXIST: &str = "充值申请不存在";
pub const MSG_RECHARGE_PAID: &str = "充值申请已支付";
pub const MSG_USER_NOT_EXIST: &str = "用户不存在";
pub const MSG_AMOUNT_INVALID: &str = "充值金额错误";
pub const MSG_PAY_METHOD_NOT_SELECTED: &str = "未选择支付方式";
pub const MSG_CASH_NOT_ALLOWED: &str = "请重新选择充值支付方式";
pub const MSG_CHANNEL_UNMIGRATED: &str =
    "支付渠道客户端（微信/支付宝等）尚未迁移到 Rust 侧，当前未创建支付单";
pub const MSG_ADD_TIME_MISSING: &str = "充值记录不存在";

/// 前台充值记录列表查询（Java `UserRechargeOrderListDTO`，userId 由登录态注入）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ClientRechargeListQuery {
    pub status: Option<i32>,
    pub user_id: Option<i32>,
    pub keyword: Option<String>,
    pub page: Option<i64>,
    pub size: Option<i64>,
    pub sort_field: Option<String>,
    pub sort_order: Option<String>,
}

impl ClientRechargeListQuery {
    #[must_use]
    pub fn limit_offset(&self) -> (i64, i64) {
        limit_offset(self.page, self.size)
    }
}

/// 充值配置分页查询（Java `RechargeSettingPageQuery`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RechargeSettingPageQuery {
    pub is_show: Option<i32>,
    pub page: Option<i64>,
    pub size: Option<i64>,
    pub sort_field: Option<String>,
    pub sort_order: Option<String>,
}

impl RechargeSettingPageQuery {
    #[must_use]
    pub fn limit_offset(&self) -> (i64, i64) {
        limit_offset(self.page, self.size)
    }
}

/// 充值配置行（Java `RechargeSettingVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RechargeSettingVO {
    pub recharge_id: i32,
    pub money: Option<Decimal>,
    pub discount_money: Option<Decimal>,
    pub sort_order: Option<i32>,
    pub is_show: Option<i32>,
}

/// 充值申请入参（Java `ClientUserRechargeOrderUpdateDTO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ClientRechargeUpdateDTO {
    pub user_id: Option<i32>,
    pub id: Option<i32>,
    pub amount: Option<Decimal>,
}

/// 充值申请返回（Java `RechargeUpdateOrderVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RechargeUpdateOrderVO {
    pub order_id: i32,
}

/// 充值支付查询入参（Java `ClientUserRechargeOrderPayDTO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ClientRechargePayQuery {
    pub order_id: Option<i32>,
    pub user_id: Option<i32>,
}

/// 充值支付创建入参（Java `UserRechargeOrderPayDTO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RechargeCreatePayDTO {
    pub id: Option<i32>,
    pub user_id: Option<i32>,
    pub r#type: Option<String>,
    pub cash_amount: Option<Decimal>,
    pub cash_change_amount: Option<Decimal>,
    pub code: Option<String>,
    pub auth_code: Option<String>,
}

/// 充值支付信息（Java `UserRechargeOrderPayVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRechargeOrderPayVO {
    pub status_type: String,
    pub order_id: i32,
    pub user_id: i32,
    pub amount: Decimal,
    pub discount_money: Option<Decimal>,
    pub pay_type: Option<String>,
    pub cash_amount: Option<Decimal>,
    pub cash_change_amount: Option<Decimal>,
    pub add_time: String,
    pub paid_time: String,
    pub postscript: String,
    pub status: i32,
    pub user: Option<RechargePayUserVO>,
}

/// `UserRechargeOrderPayVO.UserVO`。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RechargePayUserVO {
    pub username: String,
    pub nickname: Option<String>,
    pub user_id: i32,
    pub mobile: String,
}

/// 前台充值/提现记录行（Java `ClientUserRechargeOrderVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientUserRechargeOrderVO {
    pub status_type: String,
    pub amount: Decimal,
    pub add_time: String,
    pub add_time_long: i64,
    pub postscript: String,
    pub status: i32,
    pub r#type: String,
}

/// `pay` 返回结构（Java `Map.of("order", VO, "paymentList", list)`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RechargePayVO {
    pub order: UserRechargeOrderPayVO,
    pub payment_list: Vec<String>,
}

/// 列表分页 + 合并排序（Java clientList：提现 + 充值 合并按 addTimeLong 倒序分页）。
#[must_use]
pub fn merge_and_page(
    mut items: Vec<ClientUserRechargeOrderVO>,
    page: i64,
    size: i64,
) -> Vec<ClientUserRechargeOrderVO> {
    items.sort_by(|a, b| b.add_time_long.cmp(&a.add_time_long));
    let page = page.max(1);
    let size = size.max(1);
    let skip = usize::try_from((page - 1) * size).unwrap_or(0);
    items
        .into_iter()
        .skip(skip)
        .take(usize::try_from(size).unwrap_or(15))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts_and_time() {
        assert_eq!(recharge_status_text(0), "待确认");
        assert_eq!(recharge_status_text(2), "无效");
        assert_eq!(withdraw_status_text(1), "已完成");
        assert_eq!(format_time(1_700_000_000), "2023-11-15 06:13:20");
        assert_eq!(format_time(0), "");
        assert_eq!(limit_offset(None, None), (15, 0));
        assert_eq!(limit_offset(Some(2), Some(10)), (10, 10));
    }

    #[test]
    fn merge_sort_and_pagination() {
        let mk = |t: i64| ClientUserRechargeOrderVO {
            add_time_long: t,
            ..Default::default()
        };
        let page = merge_and_page(vec![mk(1), mk(3), mk(2)], 1, 2);
        assert_eq!(page.len(), 2);
        assert_eq!(page[0].add_time_long, 3);
        assert_eq!(page[1].add_time_long, 2);
        let page2 = merge_and_page(vec![mk(1), mk(3), mk(2)], 2, 2);
        assert_eq!(page2.len(), 1);
        assert_eq!(page2[0].add_time_long, 1);
    }

    #[test]
    fn query_deserializes_camel_case() {
        let q: ClientRechargeUpdateDTO =
            serde_json::from_str(r#"{"id": 5, "amount": "100.00"}"#).unwrap();
        assert_eq!(q.id, Some(5));
        assert_eq!(q.amount, Some(Decimal::new(10000, 2)));
        let q: RechargeCreatePayDTO =
            serde_json::from_str(r#"{"id": 5, "type": "wechat", "cashAmount": 1}"#).unwrap();
        assert_eq!(q.r#type.as_deref(), Some("wechat"));
    }
}
