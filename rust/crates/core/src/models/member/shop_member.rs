//! 店铺会员域（member::shop_member）DTO / VO / 查询参数 / 纯函数。
//!
//! 对应原 Java：
//! - `com.nextrade.bean.param.user.{ShopMemberSettingParam, ShopMemberAddParam,
//!   ShopMemberQueryPage, ShopMemberShopQueryPage}`
//! - `com.nextrade.bean.vo.user.{ShopMemberSettingVO, ShopMemberListVO,
//!   ShopMemberShopListVO, UserImportSuccessListVO}`
//! - `com.nextrade.bean.enums.user.UserTypeEnum`
//! - `com.nextrade.service.user.impl.{ShopMemberServiceImpl,
//!   ShopMemberSettingServiceImpl}`
//!
//! 实体 `nextrade_core::entities::ShopMember`（表 `shop_member`，10 列）已由
//! schema 生成；金额 `order_amount` 是 `decimal(10,2)` → `Decimal`。
//!
//! # 关键语义
//!
//! - 平台/店铺隔离：`HeaderUtils.getShopId()` → 平台=0、店铺/门店=自身；店铺端
//!   强制忽略入参 `shopId`，平台端可显式指定。
//! - 脱敏（`shouldMask`）：仅店铺端生效；总开关关闭 ⇒ 强制脱敏；总开关开 ⇒
//!   由「店铺客户信息脱敏」开关决定。手机号 `138****8000`、姓名首字 + `**`。
//! - 总开关四项配置：`shopMemberEnabled` / `shopMemberDesensitize` /
//!   `shopMemberAllowAdd` / `shopMemberAllowImport`（`SettingsEnum`，默认 0）。
//! - 店铺端新增客户：总开关冻结校验、子开关校验、平台账号拒绝、手机号查会员、
//!   已是本店会员拒绝。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 店铺会员导入行（Java `ShopMemberImportExcelDTO`）。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShopMemberImportRow {
    pub username: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub nickname: Option<String>,
    pub birthday: Option<String>,
}

/// 店铺会员导入失败行（Java `ShopMemberImportErrorExcelVO`）。
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShopMemberImportErrorVO {
    pub username: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub nickname: Option<String>,
    pub birthday: Option<String>,
    pub error_msg: String,
}

// ——————————————————————————————————————————————
// 常量
// ——————————————————————————————————————————————

/// `shop_member` 的 10 个列。
pub const MEMBER_COLUMNS: [&str; 10] = [
    "id",
    "user_id",
    "shop_id",
    "order_count",
    "order_amount",
    "join_time",
    "last_consume_time",
    "create_time",
    "update_time",
    "is_del",
];

/// 主键列。
pub const KEY_COLUMN: &str = "id";
/// 主键属性名。
pub const KEY_PROPERTY: &str = "id";

/// 总开关配置键（Java `SettingsEnum.SHOP_MEMBER_ENABLED`）。
pub const CONFIG_ENABLED: &str = "shopMemberEnabled";
/// 脱敏配置键。
pub const CONFIG_DESENSITIZE: &str = "shopMemberDesensitize";
/// 允许添加客户配置键。
pub const CONFIG_ALLOW_ADD: &str = "shopMemberAllowAdd";
/// 允许导入客户配置键。
pub const CONFIG_ALLOW_IMPORT: &str = "shopMemberAllowImport";
/// 默认头像配置键（`SettingsEnum.DEFAULT_AVATAR`）。
pub const CONFIG_DEFAULT_AVATAR: &str = "defaultAvatar";

/// 会员用户类型（Java `UserTypeEnum.MEMBER`）。
pub const USER_TYPE_MEMBER: i32 = 0;

/// 平台未开放店铺会员功能（冻结态）。
pub const MSG_SETTING_FROZEN: &str =
    "平台未开放店铺会员功能，当前所有会员功能处于冻结状态，如有疑问请联系平台客服。";
/// 平台未开放「店铺添加客户」。
pub const MSG_ADD_DISABLED: &str = "平台未开放「店铺添加客户」功能。";
/// 平台未开放「店铺导入客户」。
pub const MSG_IMPORT_DISABLED: &str = "平台未开放「店铺导入客户」功能。";
/// 仅店铺端可新增客户。
pub const MSG_SHOP_ONLY_ADD: &str = "仅店铺端可新增客户";
/// 仅店铺端可导入客户。
pub const MSG_SHOP_ONLY_IMPORT: &str = "仅店铺端可导入客户";
/// 手机号必填。
pub const MSG_MOBILE_REQUIRED: &str = "手机号不能为空";
/// 找不到手机号对应会员（Java 为动态拼接，见 [`MSG_MOBILE_NOT_FOUND_PREFIX`]）。
pub const MSG_MOBILE_NOT_FOUND_PREFIX: &str = "找不到手机号为：";
/// 已是本店会员。
pub const MSG_ALREADY_MEMBER: &str = "该客户已是本店会员";
/// 会员不存在。
pub const MSG_MEMBER_NOT_EXISTS: &str = "会员不存在";
/// 设置项取值只能 0/1。
pub const MSG_SETTING_RANGE: &str = "取值 0/1";
/// `shopMemberEnabled` 必填。
pub const MSG_ENABLED_REQUIRED: &str = "shopMemberEnabled 不能为空";
/// Excel 模板下载未迁移。
pub const MSG_TEMPLATE_UNMIGRATED: &str =
    "Excel 导入模板下载尚未迁移到 Rust 侧（Java 返回 xlsx 二进制流）";
/// Excel 批量导入未迁移。
pub const MSG_IMPORT_UNMIGRATED: &str =
    "Excel 批量导入尚未迁移到 Rust 侧（Java 解析 xlsx 后批量绑定店铺会员）";
/// Excel 失败行导出未迁移。
pub const MSG_IMPORT_ERROR_UNMIGRATED: &str =
    "Excel 导入失败行导出尚未迁移到 Rust 侧（Java 返回 xlsx 二进制流）";
/// 请上传文件。
pub const MSG_FILE_REQUIRED: &str = "请上传文件";

// ——————————————————————————————————————————————
// 纯函数
// ——————————————————————————————————————————————

/// 手机号脱敏（Hutool `DesensitizedUtil.mobilePhone`：前 3 + `****` + 后 4）。
#[must_use]
pub fn mask_mobile(mobile: &str) -> String {
    if mobile.is_empty() {
        return mobile.to_string();
    }
    let chars: Vec<char> = mobile.chars().collect();
    if chars.len() < 7 {
        return mobile.to_string();
    }
    let head: String = chars[..3].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}****{tail}")
}

/// 姓名脱敏（Java `maskName`：长度 1 ⇒ `x*`，否则首字 + `**`）。
#[must_use]
pub fn mask_name(name: &str) -> String {
    if name.is_empty() {
        return name.to_string();
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap_or_default();
    if name.chars().count() == 1 {
        format!("{first}*")
    } else {
        format!("{first}**")
    }
}

/// 是否需要脱敏（Java `shouldMask`）。
#[must_use]
pub fn should_mask(context_shop_id: i32, enabled: bool, desensitize: bool) -> bool {
    if context_shop_id <= 0 {
        return false;
    }
    if !enabled {
        return true;
    }
    desensitize
}

/// 配置值解析成 int（Java `parseIntOrNull`，非法/空 ⇒ `None`）。
#[must_use]
pub fn parse_config_int(raw: Option<&str>) -> Option<i32> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<i32>().ok())
}

/// 配置 int 读取口径（Java `readInt`：缺失/非法 ⇒ 默认值）。
#[must_use]
pub fn config_int_or(raw: Option<&str>, default: i32) -> i32 {
    parse_config_int(raw).unwrap_or(default)
}

/// 有效店铺 ID（Java `memberList`/`memberDetail` 的越权防线）：
/// 店铺端（context > 0）强制本店；平台端（context == 0）允许入参、缺省 0。
#[must_use]
pub fn effective_shop_id(context_shop_id: i32, query_shop_id: Option<i32>) -> i32 {
    if context_shop_id > 0 {
        context_shop_id
    } else {
        query_shop_id.unwrap_or(context_shop_id)
    }
}

/// 排序字段 → 列名（未知回退主键）。
#[must_use]
pub fn map_order_column(field: &str) -> &'static str {
    match field {
        "id" => "id",
        "userId" | "user_id" => "user_id",
        "shopId" | "shop_id" => "shop_id",
        "orderCount" | "order_count" => "order_count",
        "orderAmount" | "order_amount" => "order_amount",
        "joinTime" | "join_time" => "join_time",
        "lastConsumeTime" | "last_consume_time" => "last_consume_time",
        _ => KEY_COLUMN,
    }
}

// ——————————————————————————————————————————————
// 入参
// ——————————————————————————————————————————————

/// 总开关保存入参（Java `ShopMemberSettingParam`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShopMemberSettingParam {
    /// 总开关（`@NotNull` + 0/1）
    pub shop_member_enabled: Option<i32>,
    /// 脱敏开关（0/1）
    pub shop_member_desensitize: Option<i32>,
    /// 允许添加客户（0/1）
    pub shop_member_allow_add: Option<i32>,
    /// 允许导入客户（0/1）
    pub shop_member_allow_import: Option<i32>,
}

impl ShopMemberSettingParam {
    /// 注解校验（`@NotNull` + `@Min(0)`/`@Max(1)`）。
    pub fn validate(&self) -> Result<i32, String> {
        let enabled = self
            .shop_member_enabled
            .ok_or_else(|| MSG_ENABLED_REQUIRED.to_string())?;
        for v in [
            Some(enabled),
            self.shop_member_desensitize,
            self.shop_member_allow_add,
            self.shop_member_allow_import,
        ]
        .into_iter()
        .flatten()
        {
            if !matches!(v, 0 | 1) {
                return Err(MSG_SETTING_RANGE.to_string());
            }
        }
        Ok(enabled)
    }
}

/// 总开关 VO（Java `ShopMemberSettingVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopMemberSettingVO {
    pub shop_member_enabled: i32,
    pub shop_member_desensitize: i32,
    pub shop_member_allow_add: i32,
    pub shop_member_allow_import: i32,
}

/// 会员列表查询（Java `ShopMemberQueryPage extends BasePage`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShopMemberQueryPage {
    /// 店铺 ID（平台端可指定；店铺端忽略）
    pub shop_id: Option<i32>,
    /// 客户 ID
    pub user_id: Option<i32>,
    /// 用户类型（未传默认会员=0）
    pub user_type: Option<i32>,
    /// 关键字（昵称/手机号/邮箱）
    pub keyword: Option<String>,
    /// 消费金额下限
    pub order_amount_min: Option<Decimal>,
    /// 消费金额上限
    pub order_amount_max: Option<Decimal>,
    /// 入会时间起（Unix 秒）
    pub join_time_start: Option<i64>,
    /// 入会时间止
    pub join_time_end: Option<i64>,
    /// 最近消费时间起
    pub last_consume_time_start: Option<i64>,
    /// 最近消费时间止
    pub last_consume_time_end: Option<i64>,
    /// 页码
    pub page: Option<i64>,
    /// 每页条数（Java `size`）
    pub size: Option<i64>,
    /// 排序字段（Java 固定按 `join_time desc`）
    pub sort_field: Option<String>,
    /// 排序方向
    pub sort_order: Option<String>,
}

impl ShopMemberQueryPage {
    /// `(LIMIT, OFFSET)`。
    #[must_use]
    pub fn limit_offset(&self) -> (i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let size = match self.size {
            None => 15,
            Some(v) if v > 100 => 100,
            Some(v) if v < 1 => 1,
            Some(v) => v,
        };
        (size, (page - 1) * size)
    }

    /// 有效关键字（Java `StrUtil.isNotEmpty`：只判空串）。
    #[must_use]
    pub fn effective_keyword(&self) -> Option<&str> {
        self.keyword.as_deref().filter(|s| !s.is_empty())
    }

    /// 有效用户类型（未传默认会员 0）。
    #[must_use]
    pub fn effective_user_type(&self) -> i32 {
        self.user_type.unwrap_or(USER_TYPE_MEMBER)
    }
}

/// 平台端店铺列表查询（Java `ShopMemberShopQueryPage extends BasePage`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShopMemberShopQueryPage {
    /// 店铺名称（模糊）
    pub shop_title: Option<String>,
    /// 店铺类型
    pub shop_type: Option<i32>,
    /// 页码
    pub page: Option<i64>,
    /// 每页条数
    pub size: Option<i64>,
}

impl ShopMemberShopQueryPage {
    /// `(LIMIT, OFFSET)`。
    #[must_use]
    pub fn limit_offset(&self) -> (i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let size = match self.size {
            None => 15,
            Some(v) if v > 100 => 100,
            Some(v) if v < 1 => 1,
            Some(v) => v,
        };
        (size, (page - 1) * size)
    }

    /// 有效店铺名（`StrUtil.isNotEmpty`）。
    #[must_use]
    pub fn effective_shop_title(&self) -> Option<&str> {
        self.shop_title.as_deref().filter(|s| !s.is_empty())
    }
}

/// 新增客户入参（Java `ShopMemberAddParam`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShopMemberAddParam {
    /// 手机号（`@NotBlank`）
    pub mobile: Option<String>,
    /// 昵称
    pub nickname: Option<String>,
    /// 用户名
    pub username: Option<String>,
    /// 邮箱
    pub email: Option<String>,
}

impl ShopMemberAddParam {
    /// 注解校验（`@NotBlank(message = "手机号不能为空")`）。
    pub fn validate(&self) -> Result<String, String> {
        match self.mobile.as_deref().map(str::trim) {
            Some(mobile) if !mobile.is_empty() => Ok(mobile.to_string()),
            _ => Err(MSG_MOBILE_REQUIRED.to_string()),
        }
    }
}

/// `detail?shopId=&userId=` 查询参数。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShopMemberDetailQuery {
    pub shop_id: Option<i32>,
    pub user_id: Option<i32>,
}

// ——————————————————————————————————————————————
// 出参 VO
// ——————————————————————————————————————————————

/// 会员列表行（Java `ShopMemberListVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopMemberListVO {
    pub user_id: i32,
    pub nickname: Option<String>,
    pub mobile: Option<String>,
    pub avatar: Option<String>,
    pub order_count: i32,
    pub order_amount: Decimal,
    pub join_time: Option<i64>,
    pub last_consume_time: Option<i64>,
    /// Java 占位字段 ⇒ 恒为 null
    pub rank_name: Option<String>,
    /// Java 占位字段 ⇒ 恒为 null
    pub shop_points: Option<i32>,
    /// Java 占位字段 ⇒ 恒为 null
    pub growth: Option<i32>,
}

/// 平台端店铺列表行（Java `ShopMemberShopListVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopMemberShopListVO {
    pub shop_id: i32,
    pub shop_title: Option<String>,
    pub shop_logo: Option<String>,
    pub status: Option<i32>,
    pub shop_type: Option<i32>,
    pub member_count: i64,
}

/// 导入成功行（Java `UserImportSuccessListVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserImportSuccessListVO {
    pub user_id: i32,
    pub username: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub nickname: Option<String>,
    pub rank_name: Option<String>,
    pub avatar: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_mobile_keeps_head_and_tail() {
        assert_eq!(mask_mobile("13800008000"), "138****8000");
        assert_eq!(mask_mobile(""), "");
        assert_eq!(mask_mobile("123456"), "123456", "太短不脱敏");
    }

    #[test]
    fn mask_name_matches_java() {
        assert_eq!(mask_name("张"), "张*");
        assert_eq!(mask_name("张三"), "张**");
        assert_eq!(mask_name("张三丰"), "张**");
        assert_eq!(mask_name(""), "");
    }

    #[test]
    fn should_mask_only_for_shop_context() {
        // 平台端不脱敏
        assert!(!should_mask(0, false, false));
        assert!(!should_mask(0, true, true));
        // 店铺端：总开关关 ⇒ 强制脱敏
        assert!(should_mask(3, false, false));
        // 正常态由脱敏开关决定
        assert!(!should_mask(3, true, false));
        assert!(should_mask(3, true, true));
    }

    #[test]
    fn effective_shop_id_enforces_shop_isolation() {
        assert_eq!(effective_shop_id(5, Some(9)), 5, "店铺端忽略入参");
        assert_eq!(effective_shop_id(0, Some(9)), 9, "平台端可用入参");
        assert_eq!(effective_shop_id(0, None), 0);
    }

    #[test]
    fn setting_param_validation() {
        let mut p = ShopMemberSettingParam::default();
        assert_eq!(p.validate().unwrap_err(), MSG_ENABLED_REQUIRED);
        p.shop_member_enabled = Some(1);
        assert_eq!(p.validate().unwrap(), 1);
        p.shop_member_desensitize = Some(2);
        assert_eq!(p.validate().unwrap_err(), MSG_SETTING_RANGE);
    }

    #[test]
    fn add_param_requires_mobile() {
        let mut p = ShopMemberAddParam::default();
        assert_eq!(p.validate().unwrap_err(), MSG_MOBILE_REQUIRED);
        p.mobile = Some("  ".into());
        assert_eq!(p.validate().unwrap_err(), MSG_MOBILE_REQUIRED);
        p.mobile = Some(" 13800008000 ".into());
        assert_eq!(p.validate().unwrap(), "13800008000");
    }

    #[test]
    fn config_int_parsing() {
        assert_eq!(parse_config_int(None), None);
        assert_eq!(parse_config_int(Some("")), None);
        assert_eq!(parse_config_int(Some("abc")), None);
        assert_eq!(parse_config_int(Some(" 1 ")), Some(1));
        assert_eq!(config_int_or(None, 0), 0);
        assert_eq!(config_int_or(Some("1"), 0), 1);
    }

    #[test]
    fn list_query_defaults_and_paging() {
        let q = ShopMemberQueryPage::default();
        assert_eq!(q.effective_user_type(), USER_TYPE_MEMBER);
        assert_eq!(q.effective_keyword(), None);
        assert_eq!(q.limit_offset(), (15, 0));
        let q = ShopMemberQueryPage {
            page: Some(3),
            size: Some(1000),
            keyword: Some("".into()),
            ..Default::default()
        };
        assert_eq!(q.limit_offset(), (100, 200));
        assert_eq!(q.effective_keyword(), None);
        assert_eq!(map_order_column("orderAmount"), "order_amount");
        assert_eq!(map_order_column("nope"), "id");
    }
}
