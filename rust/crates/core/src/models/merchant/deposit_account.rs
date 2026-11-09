//! 保证金账户域（merchant::deposit_account）DTO / VO / 纯函数。
//!
//! 对应原 Java：
//! - `com.nextrade.adminapi.controller.deposit.DepositAccountController`
//!   （`@RequestMapping("/adminapi/deposit/account")`，8 个端点）
//! - `com.nextrade.service.deposit.impl.DepositAccountAdminServiceImpl`
//! - DTO：`DepositAccountListDTO` / `DepositAdjustDTO` / `DepositDeductDTO` / `DepositSendWarningDTO`
//! - VO：`DepositAccountVO` / `DepositAccountDetailVO` / `DepositLogVO` / `DepositAdjustLogVO`
//!
//! 金额字段统一 `rust_decimal::Decimal`（与实体一致）；秒级时间戳 `i32`。
//!
//! # 未迁移
//! - `sendWarning`：Java 经站内信 + 短信/推送发送预警（message 域未迁移）。
//! - `runDueCheck`：Java 的 `DepositDueCheckTask` 做保证金到期/惩罚/下架生命周期扫描，
//!   依赖 `DepositLifecycleService`（未迁移），本实现为空操作成功（运维端点）。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Java `HttpStatus.SERVICE_DATA_ERROR`。
pub const SERVICE_DATA_ERROR: i32 = 1001;

/// Java 逐字错误文案（`DepositAccountAdminServiceImpl`）。
pub const OWNER_ID_INVALID: &str = "持有者id非法";
pub const ADJUST_TYPE_INVALID: &str = "调整方式非法";
pub const REQUIRED_MANUAL_INVALID: &str = "手动应缴金额非法";
pub const DEDUCT_AMOUNT_INVALID: &str = "扣除金额非法";
pub const SHOP_NOT_EXIST: &str = "店铺不存在";
pub const VENDOR_NOT_EXIST: &str = "供应商不存在";
pub const PARAM_INVALID: &str = "参数非法";

/// 持有者类型：店铺。
pub const OWNER_SHOP: i32 = 1;
/// 持有者类型：供应商。
pub const OWNER_SUPPLIER: i32 = 2;

/// 保证金状态文案（Java `DepositStatus` 枚举：1正常 2不足 3冻结 4未缴纳）。
#[must_use]
pub fn deposit_status_text(status: i32) -> &'static str {
    match status {
        1 => "正常",
        2 => "不足",
        3 => "冻结",
        4 => "未缴纳",
        _ => "",
    }
}

/// 持有者文案。
#[must_use]
pub fn owner_type_text(owner_type: i32) -> &'static str {
    match owner_type {
        OWNER_SHOP => "店铺",
        OWNER_SUPPLIER => "供应商",
        _ => "",
    }
}

/// 主体类型（商户性质）文案：1个人 2企业。
#[must_use]
pub fn subject_type_text(subject_type: i32) -> &'static str {
    match subject_type {
        1 => "个人",
        2 => "企业",
        _ => "",
    }
}

/// 流水类型文案（Java `DepositLogType`：1充值入账 2提现出账 3退还出账 4平台扣除 5平台调整）。
#[must_use]
pub fn log_type_text(log_type: i32) -> &'static str {
    match log_type {
        1 => "充值入账",
        2 => "提现出账",
        3 => "退还出账",
        4 => "平台扣除",
        5 => "平台调整",
        _ => "",
    }
}

/// 调整来源文案（Java `DepositAdjustSource`：1按类目最高 2手动固定）。
#[must_use]
pub fn adjust_source_text(source: i32) -> &'static str {
    match source {
        1 => "按类目最高",
        2 => "手动固定",
        _ => "",
    }
}

/// 金额文本 → JSON（`Decimal` 保留两位小数，与 Java `BigDecimal` 序列化一致）。
#[must_use]
pub fn money(v: Decimal) -> serde_json::Value {
    serde_json::json!(v.round_dp(2).normalize().to_string())
        .as_str()
        .map(|s| {
            s.parse::<f64>()
                .ok()
                .and_then(serde_json::Number::from_f64)
                .map_or(serde_json::Value::Null, serde_json::Value::Number)
        })
        .unwrap_or(serde_json::Value::Null)
}

/// Java 的「保证金状态」判定（基于应缴 `required` 与实缴 `paid`）：
/// - `required <= 0 && paid <= 0` → 4 未缴纳
/// - `paid >= required` → 1 正常
/// - 其余 → 2 不足
/// （3 冻结由惩罚状态置位，本实现不推断，保持 Java 中由 `penalty_status` 驱动的语义缺口。）
#[must_use]
pub fn compute_deposit_status(required: Decimal, paid: Decimal, penalty_status: i32) -> i32 {
    if penalty_status != 0 {
        return 3;
    }
    if required <= Decimal::ZERO && paid <= Decimal::ZERO {
        return 4;
    }
    if paid >= required {
        1
    } else {
        2
    }
}

/// 可用金额 = max(0, 实缴 - 应缴)；冻结金额 = min(实缴, 应缴)（Java 详情页口径）。
#[must_use]
pub fn available_amount(paid: Decimal, required: Decimal) -> Decimal {
    let v = paid - required;
    if v <= Decimal::ZERO {
        Decimal::ZERO
    } else {
        v
    }
}

/// 待补缴 = max(0, 应缴 - 实缴)。
#[must_use]
pub fn pending_amount(paid: Decimal, required: Decimal) -> Decimal {
    let v = required - paid;
    if v <= Decimal::ZERO {
        Decimal::ZERO
    } else {
        v
    }
}

/// 账户列表查询（Java `DepositAccountListDTO extends BasePage`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DepositAccountListQuery {
    pub owner_type: Option<i32>,
    pub subject_type: Option<i32>,
    pub owner_name: Option<String>,
    pub shop_title: Option<String>,
    pub merchant_name: Option<String>,
    pub category_id: Option<i32>,
    pub deposit_status: Option<i32>,
    pub join_time_start: Option<i64>,
    pub join_time_end: Option<i64>,
    pub page: Option<u64>,
    pub size: Option<u64>,
}

impl DepositAccountListQuery {
    /// Java `BasePage` 分页归一化。
    #[must_use]
    pub fn page_params(&self) -> (u64, u64) {
        (
            self.page.unwrap_or(1).max(1),
            self.size.unwrap_or(15).clamp(1, 100),
        )
    }
}

/// 手动调整应缴（Java `DepositAdjustDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DepositAdjustDTO {
    pub owner_type: Option<i32>,
    pub owner_id: Option<i32>,
    pub shop_id: Option<i32>,
    /// 调整方式 1按类目最高 2手动固定
    pub adjust_type: Option<i32>,
    /// 手动固定应缴（adjustType=2 时必填且不可为负）
    pub required_manual: Option<Decimal>,
    pub reason: Option<String>,
}

impl DepositAdjustDTO {
    /// Java `resolveOwnerId`：优先 ownerId，回退 shopId。
    #[must_use]
    pub fn resolved_owner_id(&self) -> Option<i32> {
        self.owner_id.or(self.shop_id)
    }
}

/// 赔付扣除（Java `DepositDeductDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DepositDeductDTO {
    pub owner_type: Option<i32>,
    pub owner_id: Option<i32>,
    pub shop_id: Option<i32>,
    /// 扣除金额（正数）
    pub amount: Option<Decimal>,
    pub reason: Option<String>,
    /// 关联订单 id
    pub order_id: Option<i32>,
}

impl DepositDeductDTO {
    #[must_use]
    pub fn resolved_owner_id(&self) -> Option<i32> {
        self.owner_id.or(self.shop_id)
    }
}

/// 发送预警（Java `DepositSendWarningDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DepositSendWarningDTO {
    pub owner_type: Option<i32>,
    pub owner_id: Option<i32>,
    pub shop_id: Option<i32>,
    pub title: Option<String>,
    pub content: Option<String>,
}

impl DepositSendWarningDTO {
    #[must_use]
    pub fn resolved_owner_id(&self) -> Option<i32> {
        self.owner_id.or(self.shop_id)
    }
}

/// 账户列表项（Java `DepositAccountVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DepositAccountVO {
    pub owner_type: i32,
    pub owner_type_text: String,
    pub subject_type: i32,
    pub subject_type_text: String,
    pub owner_id: i32,
    pub owner_name: String,
    pub shop_id: Option<i32>,
    pub shop_title: Option<String>,
    pub merchant_name: Option<String>,
    pub business_category: Option<String>,
    pub required: String,
    pub paid: String,
    pub pending: String,
    pub deposit_status: i32,
    pub deposit_status_text: String,
}

/// 账户详情（Java `DepositAccountDetailVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DepositAccountDetailVO {
    pub owner_type: i32,
    pub owner_type_text: String,
    pub owner_id: i32,
    pub owner_name: String,
    pub shop_id: Option<i32>,
    pub shop_title: Option<String>,
    pub subject_type: i32,
    pub subject_type_text: String,
    pub deposit_status: i32,
    pub deposit_status_text: String,
    pub required: String,
    pub paid: String,
    pub frozen: String,
    pub available: String,
    pub pending: String,
}

/// 交易记录（Java `DepositLogVO`，仅保留表格列 + 关键关联字段）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DepositLogVO {
    pub shop_deposit_log_id: i32,
    pub owner_type: i32,
    pub owner_type_text: String,
    pub owner_id: i32,
    pub owner_name: String,
    pub r#type: i32,
    pub type_text: String,
    pub change_amount: String,
    pub before_paid: String,
    pub after_paid: String,
    pub related_id: i32,
    pub order_id: i32,
    pub admin_id: i32,
    pub remark: Option<String>,
    pub add_time: String,
}

/// 调整记录（Java `DepositAdjustLogVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DepositAdjustLogVO {
    pub shop_deposit_adjust_id: i32,
    pub owner_type: i32,
    pub owner_type_text: String,
    pub owner_id: i32,
    pub owner_name: String,
    pub shop_id: Option<i32>,
    pub before_required: String,
    pub after_required: String,
    pub source: i32,
    pub source_text: String,
    pub admin_id: i32,
    pub admin_name: Option<String>,
    pub reason: Option<String>,
    pub add_time: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn d(s: &str) -> Decimal {
        s.parse().expect("decimal")
    }

    #[test]
    fn deposit_status_and_amounts_match_java_semantics() {
        assert_eq!(compute_deposit_status(d("0"), d("0"), 0), 4);
        assert_eq!(compute_deposit_status(d("100"), d("100"), 0), 1);
        assert_eq!(compute_deposit_status(d("100"), d("120"), 0), 1);
        assert_eq!(compute_deposit_status(d("100"), d("50"), 0), 2);
        // 惩罚状态 → 冻结
        assert_eq!(compute_deposit_status(d("100"), d("100"), 1), 3);

        assert_eq!(available_amount(d("120"), d("100")), d("20"));
        assert_eq!(available_amount(d("80"), d("100")), Decimal::ZERO);
        assert_eq!(pending_amount(d("80"), d("100")), d("20"));
        assert_eq!(pending_amount(d("120"), d("100")), Decimal::ZERO);
    }

    #[test]
    fn texts_match_java_enums() {
        assert_eq!(owner_type_text(1), "店铺");
        assert_eq!(owner_type_text(2), "供应商");
        assert_eq!(subject_type_text(1), "个人");
        assert_eq!(subject_type_text(2), "企业");
        for (code, text) in [
            (1, "充值入账"),
            (2, "提现出账"),
            (3, "退还出账"),
            (4, "平台扣除"),
            (5, "平台调整"),
        ] {
            assert_eq!(log_type_text(code), text);
        }
        assert_eq!(adjust_source_text(1), "按类目最高");
        assert_eq!(adjust_source_text(2), "手动固定");
        assert_eq!(deposit_status_text(1), "正常");
        assert_eq!(deposit_status_text(4), "未缴纳");
    }

    #[test]
    fn queries_and_dtos_resolve_owner_id_and_paging() {
        let q = DepositAccountListQuery::default();
        assert_eq!(q.page_params(), (1, 15));
        let q = DepositAccountListQuery {
            page: Some(2),
            size: Some(500),
            ..Default::default()
        };
        assert_eq!(q.page_params(), (2, 100));

        // Java `resolveOwnerId`：ownerId 优先，回退兼容字段 shopId
        let dto = DepositAdjustDTO {
            owner_id: None,
            shop_id: Some(9),
            ..Default::default()
        };
        assert_eq!(dto.resolved_owner_id(), Some(9));
        let dto = DepositDeductDTO {
            owner_id: Some(3),
            shop_id: Some(9),
            ..Default::default()
        };
        assert_eq!(dto.resolved_owner_id(), Some(3));
    }

    #[test]
    fn money_serializes_as_json_number() {
        let v = money(d("12.30"));
        assert_eq!(v, serde_json::json!(12.3));
        let v = money(Decimal::ZERO);
        assert_eq!(v, serde_json::json!(0.0));
    }
}
