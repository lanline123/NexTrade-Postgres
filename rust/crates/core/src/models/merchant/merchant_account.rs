//! 商户账户域（merchant::merchant_account）DTO / VO / 纯函数。
//!
//! 对应原 Java：
//! - `com.nextrade.adminapi.controller.merchant.MerchantAccountController`
//!   （`@RequestMapping("/adminapi/merchant/account")`，7 个端点）
//! - `com.nextrade.service.merchant.impl.MerchantAccountServiceImpl`
//! - DTO：`MerchantAccountListDTO` / `MerchantAccountCreateDTO` / `MerchantAccountUpdateDTO`；
//!   VO：`MerchantAccountVO`；枚举：`MerchantAccountType`（1银行卡 2支付宝 3微信）
//!
//! # 与 Java 的差异
//! - Java 用请求头 `x-shop-id` 定位店铺；Rust JWT 未签发 shop_id，用查询参数 `shopId` 兜底。
//! - `list` 的账户类型白名单来自 `SettingsEnum.WITHDRAWAL_RECEIPT_METHOD`（JSON 数组，
//!   缺省 `[1,2,3]`）；本实现读 `config` 表同一 bizCode，解析失败回退全类型。
//! - `detail` 的归属校验（`checkMerchantAuth`）：`admin_user.merchant_id != 账户 merchant_id`
//!   → 「数据不存在」（Java 逐字文案）。

use serde::{Deserialize, Serialize};

/// Java `HttpStatus.SERVICE_DATA_ERROR`。
pub const SERVICE_DATA_ERROR: i32 = 1001;

/// Java `checkMerchantAuth` 越权文案。
pub const DATA_NOT_EXIST: &str = "数据不存在";
/// Java 店铺不存在。
pub const SHOP_NOT_EXIST: &str = "店铺不存在";
/// Java 商户不存在。
pub const MERCHANT_NOT_EXIST: &str = "商户不存在";
/// Java `MerchantAccountType` 未知类型的错误码文案来源 `ExceptionConstants.DATA_TYPE_ERROR`。
pub const DATA_TYPE_ERROR: &str = "数据类型错误";

/// `SettingsEnum.WITHDRAWAL_RECEIPT_METHOD` 的 bizCode。
pub const WITHDRAWAL_RECEIPT_METHOD: &str = "withdrawalReceiptMethod";

/// 提现账户类型文案（Java `MerchantAccountType.getTypeName`）。
#[must_use]
pub fn account_type_text(account_type: i32) -> &'static str {
    match account_type {
        1 => "银行卡",
        2 => "支付宝",
        3 => "微信",
        _ => "",
    }
}

/// 配置接口返回的类型列表（Java `MerchantAccountType.getTypeList`：
/// `[{accountType, accountTypeText}, ...]`）。
#[must_use]
pub fn account_type_options() -> serde_json::Value {
    serde_json::json!([
        { "accountType": 1, "accountTypeText": "银行卡" },
        { "accountType": 2, "accountTypeText": "支付宝" },
        { "accountType": 3, "accountTypeText": "微信" },
    ])
}

/// 解析 `withdrawalReceiptMethod`（JSON 数字数组）→ 允许的账户类型。
///
/// 缺失/非法/空数组 → `[1,2,3]`（Java：`StrUtil.isEmpty` 时用 `Arrays.asList(1,2,3)`；
/// 非空数组则 `in(accountType, ja)`）。
#[must_use]
pub fn allowed_account_types(raw: Option<&str>) -> Vec<i32> {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return vec![1, 2, 3];
    };
    match serde_json::from_str::<Vec<i32>>(raw) {
        Ok(v) if !v.is_empty() => v,
        _ => vec![1, 2, 3],
    }
}

/// 时间展示 `yyyy-MM-dd HH:mm:ss`（Java `TigUtils.handelTime`）。
#[must_use]
pub fn format_time(ts: Option<i32>) -> String {
    let Some(ts) = ts.filter(|t| *t > 0) else {
        return String::new();
    };
    chrono::DateTime::from_timestamp(i64::from(ts), 0)
        .map(|dt| {
            dt.with_timezone(
                &chrono::FixedOffset::east_opt(8 * 3600)
                    .unwrap_or_else(|| chrono::FixedOffset::east_opt(0).expect("utc")),
            )
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
        })
        .unwrap_or_default()
}

/// 列表查询（Java `MerchantAccountListDTO extends BasePage`，仅 `shopId` 字段 + 分页/排序）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MerchantAccountListQuery {
    pub shop_id: Option<i32>,
    pub page: Option<u64>,
    pub size: Option<u64>,
    pub sort_field: Option<String>,
    pub sort_order: Option<String>,
}

impl MerchantAccountListQuery {
    /// Java `BasePage` 归一化。
    #[must_use]
    pub fn page_params(&self) -> (u64, u64) {
        (
            self.page.unwrap_or(1).max(1),
            self.size.unwrap_or(15).clamp(1, 100),
        )
    }
}

/// 创建账户（Java `MerchantAccountCreateDTO`；`merchantId`/`shopId` 由服务端覆盖）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MerchantAccountCreateDTO {
    pub account_type: Option<i32>,
    pub account_name: Option<String>,
    pub bank_name: Option<String>,
    pub account_no: Option<String>,
    pub bank_branch: Option<String>,
    pub merchant_id: Option<i32>,
    pub shop_id: Option<i32>,
}

/// 更新账户（Java `MerchantAccountUpdateDTO`，含兼容字段 `accountId` / `id`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MerchantAccountUpdateDTO {
    pub account_id: Option<i32>,
    pub id: Option<i32>,
    pub account_type: Option<i32>,
    pub account_name: Option<String>,
    pub bank_name: Option<String>,
    pub account_no: Option<String>,
    pub bank_branch: Option<String>,
    pub shop_id: Option<i32>,
}

impl MerchantAccountUpdateDTO {
    /// 主键：`accountId` 优先，回退兼容字段 `id`。
    #[must_use]
    pub fn resolved_id(&self) -> Option<i32> {
        self.account_id.or(self.id)
    }
}

/// 账户 VO（Java `MerchantAccountVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MerchantAccountVO {
    pub account_type_text: String,
    pub account_id: i32,
    pub merchant_id: i32,
    pub account_type: i32,
    pub account_name: String,
    pub account_no: String,
    pub bank_name: String,
    pub add_time: String,
    pub bank_branch: Option<String>,
    pub shop_id: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_type_texts_and_options_match_java_enum() {
        assert_eq!(account_type_text(1), "银行卡");
        assert_eq!(account_type_text(2), "支付宝");
        assert_eq!(account_type_text(3), "微信");
        assert_eq!(account_type_text(9), "");
        let opts = account_type_options();
        assert_eq!(opts.as_array().map(Vec::len), Some(3));
        assert_eq!(opts[0]["accountType"], 1);
        assert_eq!(opts[1]["accountTypeText"], "支付宝");
    }

    #[test]
    fn allowed_types_fall_back_to_all_three() {
        assert_eq!(allowed_account_types(None), vec![1, 2, 3]);
        assert_eq!(allowed_account_types(Some("")), vec![1, 2, 3]);
        assert_eq!(allowed_account_types(Some("[]")), vec![1, 2, 3]);
        assert_eq!(allowed_account_types(Some("oops")), vec![1, 2, 3]);
        assert_eq!(allowed_account_types(Some("[1,2]")), vec![1, 2]);
    }

    #[test]
    fn paging_and_update_id_resolution() {
        assert_eq!(MerchantAccountListQuery::default().page_params(), (1, 15));
        let q = MerchantAccountListQuery {
            page: Some(2),
            size: Some(0),
            ..Default::default()
        };
        assert_eq!(q.page_params(), (2, 1));
        let dto = MerchantAccountUpdateDTO {
            account_id: Some(5),
            id: Some(6),
            ..Default::default()
        };
        assert_eq!(dto.resolved_id(), Some(5));
        let dto = MerchantAccountUpdateDTO {
            account_id: None,
            id: Some(6),
            ..Default::default()
        };
        assert_eq!(dto.resolved_id(), Some(6));
    }

    #[test]
    fn time_format_and_vo_serde() {
        assert_eq!(format_time(None), "");
        assert_eq!(format_time(Some(1_700_000_000)), "2023-11-15 06:13:20");
        let vo = MerchantAccountVO {
            account_id: 3,
            account_type: 2,
            account_type_text: account_type_text(2).into(),
            ..Default::default()
        };
        let v = serde_json::to_value(&vo).unwrap();
        assert_eq!(v["accountId"], 3);
        assert_eq!(v["accountTypeText"], "支付宝");
    }
}
