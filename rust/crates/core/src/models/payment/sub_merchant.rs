//! 子商户域（payment::sub_merchant）DTO / VO / 纯函数。
//! Java `SubMerchantController`（3 端点，admin）+ `SubMerchantServiceImpl`。
//! 表 `sub_merchant`（12 列，主键 id BIGINT）。

use serde::{Deserialize, Serialize};

/// `sub_merchant` 12 列。
pub const SUB_MERCHANT_COLUMNS: [&str; 12] = [
    "id",
    "shop_id",
    "pay_channel",
    "sub_mch_id",
    "sub_appid",
    "merchant_name",
    "status",
    "application_id",
    "max_split_ratio",
    "is_del",
    "create_time",
    "modify_time",
];
pub const KEY_COLUMN: &str = "id";
/// 状态：待激活。
pub const STATUS_INACTIVE: i32 = 0;
/// 状态：已激活。
pub const STATUS_ACTIVE: i32 = 1;
/// 状态：已冻结。
pub const STATUS_FROZEN: i32 = 2;
/// 默认渠道。
pub const DEFAULT_CHANNEL: &str = "WECHAT";
/// 平台自营展示名（非 O2O 形态；O2O 为「自营门店」）。
pub const SELF_OPERATED_SHOP_NAME: &str = "自营店铺";

pub const MSG_SUB_MCH_REQUIRED: &str = "子商户号不能为空";
pub const MSG_NOT_EXISTS: &str = "子商户不存在";
/// 渠道归属校验未迁移（不落库、不伪造成功）。
pub const MSG_CHANNEL_UNMIGRATED: &str =
    "分账渠道 API（微信服务商）尚未迁移到 Rust 侧，子商户归属校验与绑定未提交到渠道";

/// 状态文案（Java `SubMerchantStatus`）。
#[must_use]
pub fn status_desc(status: i32) -> Option<&'static str> {
    match status {
        STATUS_INACTIVE => Some("待激活"),
        STATUS_ACTIVE => Some("已激活"),
        STATUS_FROZEN => Some("已冻结"),
        _ => None,
    }
}

/// 平台自营展示名（Java `selfOperatedShopName`：O2O 形态「自营门店」）。
#[must_use]
pub fn self_operated_shop_name(is_o2o: bool) -> &'static str {
    if is_o2o {
        "自营门店"
    } else {
        SELF_OPERATED_SHOP_NAME
    }
}

/// 排序字段 → 列名（未知回退主键）。
#[must_use]
pub fn map_order_column(field: &str) -> &'static str {
    match field {
        "id" => "id",
        "shopId" | "shop_id" => "shop_id",
        "payChannel" | "pay_channel" => "pay_channel",
        "subMchId" | "sub_mch_id" => "sub_mch_id",
        "subAppid" | "sub_appid" => "sub_appid",
        "merchantName" | "merchant_name" => "merchant_name",
        "status" => "status",
        "applicationId" | "application_id" => "application_id",
        "createTime" | "create_time" => "create_time",
        "modifyTime" | "modify_time" => "modify_time",
        _ => KEY_COLUMN,
    }
}

/// ORDER BY（默认主键 DESC）。
#[must_use]
pub fn order_clause(sort_field: Option<&str>, sort_order: Option<&str>) -> String {
    let field = sort_field.map(str::trim).unwrap_or("");
    let column = if field.is_empty() {
        KEY_COLUMN
    } else {
        map_order_column(field)
    };
    let dir = if sort_order
        .map(|s| s.trim().eq_ignore_ascii_case("asc"))
        .unwrap_or(false)
    {
        "ASC"
    } else {
        "DESC"
    };
    format!(" ORDER BY {column} {dir}")
}

/// `(LIMIT, OFFSET)`。
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

/// 列表查询（Java `SubMerchantPageQuery extends BasePage`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SubMerchantPageQuery {
    pub shop_id: Option<i64>,
    pub sub_mch_id: Option<String>,
    pub pay_channel: Option<String>,
    pub status: Option<i32>,
    pub page: Option<i64>,
    pub size: Option<i64>,
    pub sort_field: Option<String>,
    pub sort_order: Option<String>,
}

impl SubMerchantPageQuery {
    #[must_use]
    pub fn limit_offset(&self) -> (i64, i64) {
        limit_offset(self.page, self.size)
    }
    #[must_use]
    pub fn order_clause(&self) -> String {
        order_clause(self.sort_field.as_deref(), self.sort_order.as_deref())
    }
    #[must_use]
    pub fn effective_sub_mch_id(&self) -> Option<&str> {
        self.sub_mch_id.as_deref().filter(|s| !s.is_empty())
    }
    #[must_use]
    pub fn effective_pay_channel(&self) -> Option<&str> {
        self.pay_channel.as_deref().filter(|s| !s.is_empty())
    }
}

/// 绑定入参（Java `SubMerchantBindParam`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SubMerchantBindParam {
    pub shop_id: Option<i64>,
    pub pay_channel: Option<String>,
    pub sub_mch_id: Option<String>,
    pub sub_appid: Option<String>,
    pub merchant_name: Option<String>,
}

/// 校验后的绑定参数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubMerchantBind {
    pub shop_id: i64,
    pub pay_channel: String,
    pub sub_mch_id: String,
    pub sub_appid: Option<String>,
    pub merchant_name: Option<String>,
}

impl SubMerchantBindParam {
    /// 本地校验：渠道默认 WECHAT、`subMchId` 必填（随后渠道归属校验未迁移）。
    pub fn validate(&self) -> Result<SubMerchantBind, String> {
        let sub_mch_id = self
            .sub_mch_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| MSG_SUB_MCH_REQUIRED.to_string())?;
        let channel = self
            .pay_channel
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(DEFAULT_CHANNEL);
        Ok(SubMerchantBind {
            shop_id: self.shop_id.unwrap_or(0),
            pay_channel: channel.to_string(),
            sub_mch_id: sub_mch_id.to_string(),
            sub_appid: self.sub_appid.clone(),
            merchant_name: self.merchant_name.clone(),
        })
    }
}

/// 子商户 VO（Java `SubMerchantVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubMerchantVO {
    pub id: i64,
    pub shop_id: i64,
    pub shop_name: String,
    pub pay_channel: String,
    pub sub_mch_id: String,
    pub sub_appid: String,
    pub merchant_name: String,
    pub status: i32,
    pub status_desc: Option<String>,
    pub application_id: i64,
    pub create_time: String,
    pub modify_time: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_and_shop_names() {
        assert_eq!(status_desc(0), Some("待激活"));
        assert_eq!(status_desc(1), Some("已激活"));
        assert_eq!(status_desc(2), Some("已冻结"));
        assert_eq!(status_desc(9), None);
        assert_eq!(self_operated_shop_name(false), "自营店铺");
        assert_eq!(self_operated_shop_name(true), "自营门店");
    }

    #[test]
    fn bind_validation() {
        let dto = SubMerchantBindParam::default();
        assert_eq!(dto.validate().unwrap_err(), MSG_SUB_MCH_REQUIRED);
        let dto = SubMerchantBindParam {
            shop_id: Some(5),
            pay_channel: Some("  ".into()),
            sub_mch_id: Some(" 123 ".into()),
            ..Default::default()
        };
        let input = dto.validate().unwrap();
        assert_eq!(input.shop_id, 5);
        assert_eq!(input.pay_channel, DEFAULT_CHANNEL);
        assert_eq!(input.sub_mch_id, "123");
    }

    #[test]
    fn paging_and_sort() {
        assert_eq!(limit_offset(None, None), (15, 0));
        assert_eq!(limit_offset(Some(2), Some(10)), (10, 10));
        assert_eq!(order_clause(None, None), " ORDER BY id DESC");
        assert_eq!(
            order_clause(Some("createTime"), Some("asc")),
            " ORDER BY create_time ASC"
        );
        assert_eq!(
            order_clause(Some("x; DROP TABLE sub_merchant"), None),
            " ORDER BY id DESC"
        );
    }
}
