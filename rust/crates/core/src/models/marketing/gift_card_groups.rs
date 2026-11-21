//! 礼品卡分组域（marketing::gift_card_groups）DTO / Query / VO / 纯函数。
//!
//! 对应原 Java：
//! - `com.nextrade.bean.model.promotion.GiftCardGroups`（表 `gift_card_groups`）
//! - `GiftCardGroupsPageQuery` / `GiftCardGroupsCreateParam` / `GiftCardGroupsUpdateParam` /
//!   `GiftCardGroupIdParam`
//! - `GiftCardGroupsVO`（+ `initTypeName`）
//! - 枚举 `GiftGroupsEnum` / `GiftCardEnum` / `ValidityTypeEnum`
//!
//! 实体 `nextrade_core::entities::GiftCardGroups`（表 `gift_card_groups`，15 列）已由
//! schema 生成；`face_amount` 是 `decimal(10,2)` → `Decimal`。
//!
//! # 关键语义 / 偏差
//!
//! - **店铺过滤**：Java `HeaderUtils.getShopId()`（`x-admin-type` / `x-shop-id`），
//!   路由层用既有 `shop_id_from_headers` 解析并传入服务层（缺失 ⇒ `0` 平台）。
//! - **`expireTime` 解析**：Java `StringUtils.getEndOfDayTimestamp` 只接受
//!   `yyyy-MM-dd HH:mm:ss`，失败抛「日期格式错误」；本实现逐字同格式 + 同文案。
//! - **商品信息**：Java VO 的 `giftCardProducts` 是 `List<ProductSkuVO>`；
//!   本实现用简化 [`GiftCardProductVO`]（商品名/图/规格/价格），字段子集偏差。
//! - **`validityType` 枚举**：Java 1 永久 / 2 天数 / 3 固定时间（与实体注释的
//!   0/1/2 不一致，以 Java 枚举码为准）。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::models::marketing::gift_card::{
    format_unix_time, GiftCardProductVO, GROUP_TYPE_EXCHANGE, GROUP_TYPE_TOP_UP,
    SCOPE_TYPE_ALL, SCOPE_TYPE_CATEGORIES, SCOPE_TYPE_PRODUCTS, VALIDITY_FIXED, VALIDITY_NUMBER,
    VALIDITY_PERMANENT,
};

/// `GiftCardGroupsPageQuery`/排序用的主键列。
pub const KEY_COLUMN: &str = "card_groups_id";
/// 主键属性名。
pub const KEY_PROPERTY: &str = "cardGroupsId";

/// 卡组不存在。
pub const MSG_GROUP_NOT_EXISTS: &str = "卡组不存在";
/// 卡组状态不正确。
pub const MSG_GROUP_STATUS_INVALID: &str = "卡组状态不正确";
/// 请填写天数有效期。
pub const MSG_DAYS_REQUIRED: &str = "请填写天数有效期";
/// 请填写固定时间有效期。
pub const MSG_EXPIRE_TIME_REQUIRED: &str = "请填写固定时间有效期";
/// 请最少选择一个商品。
pub const MSG_PRODUCTS_REQUIRED: &str = "请最少选择一个商品";
/// 请填写面额。
pub const MSG_FACE_AMOUNT_REQUIRED: &str = "请填写面额";
/// 未知的卡有效期类型。
pub const MSG_UNKNOWN_VALIDITY_TYPE: &str = "未知的卡有效期类型";
/// 该礼品卡组已经被商品绑定，不能删除。
pub const MSG_GROUP_BOUND_CANNOT_DELETE: &str = "该礼品卡组已经被商品绑定，不能删除";
/// 日期格式错误（Java `StringUtils.getEndOfDayTimestamp` 的 ParseException 文案）。
pub const MSG_DATE_FORMAT_INVALID: &str = "日期格式错误";
/// 兑换模式不正确（P1 新增：1 固定套餐 / 2 多件任选）。
pub const MSG_EXCHANGE_MODE_INVALID: &str = "兑换模式不正确";
/// 任选模式必须填写任选件数。
pub const MSG_SELECT_COUNT_REQUIRED: &str = "任选模式必须填写任选件数";
/// 任选件数不能超过绑定商品数。
pub const MSG_SELECT_COUNT_EXCEEDS_PRODUCTS: &str = "任选件数不能超过绑定商品数";

/// 适用范围类型不正确（我方新增，论坛 B 类 227890）。
pub const MSG_SCOPE_TYPE_INVALID: &str = "适用范围类型不正确";
/// 指定商品范围请至少选择一个商品（我方新增）。
pub const MSG_SCOPE_PRODUCTS_REQUIRED: &str = "指定商品范围请至少选择一个商品";
/// 指定品类范围请至少选择一个品类（我方新增）。
pub const MSG_SCOPE_CATEGORIES_REQUIRED: &str = "指定品类范围请至少选择一个品类";

/// 适用范围校验（我方新增）：仅储值卡使用（兑换卡由绑定商品池天然限定，忽略范围字段）；
/// 指定商品/品类时对应选择集合不能为空。
///
/// # Errors
/// 范围类型非法 / 指定商品未选商品 / 指定品类未选品类。
pub fn validate_scope_fields(
    group_type: i32,
    scope_type: Option<i32>,
    scope_product_count: usize,
    scope_category_count: usize,
) -> Result<(), String> {
    if group_type != GROUP_TYPE_TOP_UP {
        return Ok(());
    }
    match scope_type.unwrap_or(SCOPE_TYPE_ALL) {
        SCOPE_TYPE_ALL => Ok(()),
        SCOPE_TYPE_PRODUCTS => {
            if scope_product_count == 0 {
                Err(MSG_SCOPE_PRODUCTS_REQUIRED.to_string())
            } else {
                Ok(())
            }
        }
        SCOPE_TYPE_CATEGORIES => {
            if scope_category_count == 0 {
                Err(MSG_SCOPE_CATEGORIES_REQUIRED.to_string())
            } else {
                Ok(())
            }
        }
        _ => Err(MSG_SCOPE_TYPE_INVALID.to_string()),
    }
}

/// 兑换模式：1 固定套餐。
pub const EXCHANGE_MODE_FIXED: i32 = 1;
/// 兑换模式：2 多件任选。
pub const EXCHANGE_MODE_SELECT: i32 = 2;

/// 兑换模式校验（P1：只落库 + 编辑透传校验，任选兑换逻辑在 P2）。
///
/// 储值卡忽略两字段；兑换卡任选模式时 `select_count` 必填且
/// `1 <= select_count <= bound_product_count`（绑定数以入参商品列表长度为准）。
///
/// # Errors
/// 模式非法 / 任选件数缺失或超绑定数。
pub fn validate_exchange_fields(
    group_type: i32,
    exchange_mode: Option<i32>,
    select_count: Option<i32>,
    bound_product_count: usize,
) -> Result<(), String> {
    if group_type != GROUP_TYPE_EXCHANGE {
        return Ok(());
    }
    let mode = exchange_mode.unwrap_or(EXCHANGE_MODE_FIXED);
    if mode != EXCHANGE_MODE_FIXED && mode != EXCHANGE_MODE_SELECT {
        return Err(MSG_EXCHANGE_MODE_INVALID.to_string());
    }
    if mode == EXCHANGE_MODE_SELECT {
        let count = select_count.unwrap_or(0);
        if count < 1 {
            return Err(MSG_SELECT_COUNT_REQUIRED.to_string());
        }
        if (count as usize) > bound_product_count {
            return Err(MSG_SELECT_COUNT_EXCEEDS_PRODUCTS.to_string());
        }
    }
    Ok(())
}

/// 卡组类型名（Java `GiftGroupsEnum`）。
#[must_use]
pub fn group_type_name(group_type: i32) -> &'static str {
    match group_type {
        GROUP_TYPE_TOP_UP => "储值卡",
        GROUP_TYPE_EXCHANGE => "兑换卡",
        _ => "未知",
    }
}

/// 卡类型名（Java `GiftCardEnum`）。
#[must_use]
pub fn card_type_name(card_type: i32) -> &'static str {
    match card_type {
        1 => "电子卡",
        2 => "实体卡",
        _ => "未知",
    }
}

/// 有效期类型名（Java `ValidityTypeEnum`）。
#[must_use]
pub fn validity_type_name(validity_type: i32) -> &'static str {
    match validity_type {
        VALIDITY_PERMANENT => "永久有效",
        VALIDITY_NUMBER => "天数",
        VALIDITY_FIXED => "固定时间",
        _ => "未知",
    }
}

/// `yyyy-MM-dd HH:mm:ss` 字符串 → **当天 23:59:59** 的秒级时间戳
/// （Java `StringUtils.getEndOfDayTimestamp`；失败 →「日期格式错误」）。
pub fn parse_end_of_day_timestamp(raw: &str) -> Result<i32, String> {
    use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime};
    let parsed = NaiveDateTime::parse_from_str(raw.trim(), "%Y-%m-%d %H:%M:%S")
        .map_err(|_| MSG_DATE_FORMAT_INVALID.to_string())?;
    let date = NaiveDate::from_ymd_opt(
        parsed.date().year(),
        parsed.date().month(),
        parsed.date().day(),
    )
    .ok_or_else(|| MSG_DATE_FORMAT_INVALID.to_string())?;
    let end = date.and_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap_or_default());
    // 与全仓库口径一致：按 UTC+8 解释本地时间
    i32::try_from(end.and_utc().timestamp() - 8 * 3600)
        .map_err(|_| MSG_DATE_FORMAT_INVALID.to_string())
}

/// 商品绑定项（Java `GiftCardProducts` 的入参子集）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftCardProductDTO {
    pub gift_product_id: Option<i32>,
    pub card_groups_id: Option<i32>,
    pub product_id: Option<i32>,
    pub sku_id: Option<i32>,
}

/// 卡组分页查询（Java `GiftCardGroupsPageQuery extends BasePage`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftCardGroupsListQuery {
    /// 页码
    pub page: Option<i64>,
    /// 分页大小
    pub size: Option<i64>,
    /// 排序字段
    pub sort_field: Option<String>,
    /// 排序方向
    pub sort_order: Option<String>,
    /// 关键字（卡组名 ILIKE）
    pub keyword: Option<String>,
}

impl GiftCardGroupsListQuery {
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

    /// ORDER BY（`buildSortOrder` 三参）。
    #[must_use]
    pub fn order_clause(&self) -> String {
        let field = self.sort_field.as_deref().map(str::trim).unwrap_or("");
        let column = match field {
            "" | "cardGroupsId" | "card_groups_id" => KEY_COLUMN,
            "groupType" | "group_type" => "group_type",
            "cardType" | "card_type" => "card_type",
            "cardGroupName" | "card_group_name" => "card_group_name",
            "status" => "status",
            "addTime" | "add_time" => "add_time",
            "updateTime" | "update_time" => "update_time",
            "shopId" | "shop_id" => "shop_id",
            _ => KEY_COLUMN,
        };
        match self.sort_order.as_deref().map(str::trim) {
            Some(s) if s.eq_ignore_ascii_case("asc") => format!(" ORDER BY {column} ASC"),
            None | Some("") => format!(" ORDER BY {column} DESC"),
            Some(s) if s.eq_ignore_ascii_case("desc") => format!(" ORDER BY {column} DESC"),
            Some(_) => String::new(),
        }
    }

    /// 关键字（Java `StrUtil.isNotBlank`：空白不参与过滤）。
    #[must_use]
    pub fn effective_keyword(&self) -> Option<&str> {
        self.keyword
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }
}

/// 创建参数（Java `GiftCardGroupsCreateParam`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftCardGroupsCreateParam {
    pub group_type: Option<i32>,
    pub card_type: Option<i32>,
    pub card_group_name: Option<String>,
    pub image_url: Option<String>,
    pub validity_type: Option<i32>,
    pub validity_days: Option<i32>,
    /// Java `String expireTime`（`yyyy-MM-dd HH:mm:ss`）
    pub expire_time: Option<String>,
    pub face_amount: Option<Decimal>,
    pub status: Option<i32>,
    pub remark: Option<String>,
    pub shop_id: Option<i32>,
    pub gift_card_products: Option<Vec<GiftCardProductDTO>>,
    /// 兑换模式（P1 新增，仅兑换卡）：1 固定套餐 / 2 多件任选
    pub exchange_mode: Option<i32>,
    /// 任选件数（任选模式必填）
    pub select_count: Option<i32>,
    /// 适用范围（我方新增，仅储值卡）：0 全部商品 / 1 指定商品 / 2 指定品类
    pub scope_type: Option<i32>,
    /// 适用商品 id 集合（scope_type=1 时必填）
    pub scope_product_ids: Option<Vec<i32>>,
    /// 适用品类 id 集合（scope_type=2 时必填，含子类目按品类树展开）
    pub scope_category_ids: Option<Vec<i32>>,
}

/// 更新参数（Java `GiftCardGroupsUpdateParam`；updateStatus / productsUpdate 复用）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftCardGroupsUpdateParam {
    pub card_groups_id: Option<i32>,
    /// 仅 updateStatus 使用
    pub status: Option<i32>,
    pub card_group_name: Option<String>,
    pub image_url: Option<String>,
    pub validity_type: Option<i32>,
    pub validity_days: Option<i32>,
    pub expire_time: Option<String>,
    pub face_amount: Option<Decimal>,
    pub remark: Option<String>,
    pub gift_card_products: Option<Vec<GiftCardProductDTO>>,
    /// 兑换模式（P1 新增，仅兑换卡）：1 固定套餐 / 2 多件任选
    pub exchange_mode: Option<i32>,
    /// 任选件数（任选模式必填）
    pub select_count: Option<i32>,
    /// 适用范围（我方新增，仅储值卡）：0 全部商品 / 1 指定商品 / 2 指定品类；
    /// 仅在本次入参带了 scope_type 时才更新范围（部分更新不带时保持原范围）
    pub scope_type: Option<i32>,
    /// 适用商品 id 集合（scope_type=1 时必填）
    pub scope_product_ids: Option<Vec<i32>>,
    /// 适用品类 id 集合（scope_type=2 时必填）
    pub scope_category_ids: Option<Vec<i32>>,
}

/// 卡组 ID 参数（Java `GiftCardGroupIdParam`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftCardGroupIdParam {
    pub gift_card_groups_id: Option<i32>,
}

/// 组 VO（Java `GiftCardGroupsVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GiftCardGroupsVO {
    pub card_groups_id: Option<i32>,
    pub group_type: Option<i32>,
    pub card_type: Option<i32>,
    pub card_group_name: Option<String>,
    pub image_url: Option<String>,
    pub validity_type: Option<i32>,
    pub validity_days: Option<i32>,
    /// FIXED 时格式化为 `yyyy-MM-dd HH:mm:ss`
    pub expire_time: Option<String>,
    pub face_amount: Option<Decimal>,
    pub status: Option<i32>,
    /// 格式化为日期（Java Jackson）
    pub add_time: Option<String>,
    /// 格式化为日期（Java Jackson）
    pub update_time: Option<String>,
    pub group_type_name: Option<String>,
    pub card_type_name: Option<String>,
    pub validity_type_name: Option<String>,
    pub remark: Option<String>,
    pub gift_card_products: Vec<GiftCardProductVO>,
    pub can_delete: Option<bool>,
    /// 兑换模式（P1 新增）：1 固定套餐 / 2 多件任选
    pub exchange_mode: Option<i32>,
    /// 任选件数
    pub select_count: Option<i32>,
    /// 适用范围（我方新增，仅储值卡有意义）：0 全部商品 / 1 指定商品 / 2 指定品类
    pub scope_type: Option<i32>,
    /// 适用商品 id 集合（scope_type=1）
    pub scope_product_ids: Vec<i32>,
    /// 适用品类 id 集合（scope_type=2，运营勾选的原始品类，未展开）
    pub scope_category_ids: Vec<i32>,
}

/// `build` 后的卡组实体字段（服务层用，避免重复反序列化）。
#[derive(Debug, Clone, Default)]
pub struct GiftCardGroupsInput {
    pub card_groups_id: Option<i32>,
    pub group_type: i32,
    pub card_type: i32,
    pub card_group_name: String,
    pub image_url: Option<String>,
    pub validity_type: i32,
    pub validity_days: Option<i32>,
    pub expire_time: Option<i32>,
    pub face_amount: Option<Decimal>,
    pub status: i32,
    pub remark: Option<String>,
    pub shop_id: Option<i32>,
    pub gift_card_products: Vec<GiftCardProductDTO>,
}

impl GiftCardGroupsCreateParam {
    /// 复刻 Java `buildGiftCardGroups` 的校验分支（返回逐字中文文案）。
    ///
    /// # Errors
    /// 天数/固定时间/商品/面额缺失。
    pub fn validate(&self) -> Result<(), String> {
        let validity_type = self.validity_type.unwrap_or_default();
        let group_type = self.group_type.unwrap_or_default();
        match validity_type {
            VALIDITY_NUMBER if self.validity_days.is_none() => {
                return Err(MSG_DAYS_REQUIRED.to_string());
            }
            VALIDITY_FIXED if self.expire_time.is_none() => {
                return Err(MSG_EXPIRE_TIME_REQUIRED.to_string());
            }
            _ => {}
        }
        if group_type == GROUP_TYPE_EXCHANGE
            && self.gift_card_products.as_ref().map(Vec::len).unwrap_or(0) == 0
        {
            return Err(MSG_PRODUCTS_REQUIRED.to_string());
        }
        if group_type == GROUP_TYPE_TOP_UP && self.face_amount.is_none() {
            return Err(MSG_FACE_AMOUNT_REQUIRED.to_string());
        }
        validate_exchange_fields(
            group_type,
            self.exchange_mode,
            self.select_count,
            self.gift_card_products.as_ref().map(Vec::len).unwrap_or(0),
        )?;
        validate_scope_fields(
            group_type,
            self.scope_type,
            self.scope_product_ids.as_ref().map(Vec::len).unwrap_or(0),
            self.scope_category_ids.as_ref().map(Vec::len).unwrap_or(0),
        )
    }
}

impl GiftCardGroupsUpdateParam {
    /// 更新校验（Java `buildGiftCardGroups`）；`db_group_type` 为库中组类型。
    ///
    /// # Errors
    /// 天数/固定时间缺失，储值卡缺面额，兑换卡缺商品。
    pub fn validate(&self, db_group_type: i32) -> Result<(), String> {
        let validity_type = self.validity_type.unwrap_or_default();
        match validity_type {
            VALIDITY_NUMBER if self.validity_days.is_none() => {
                return Err(MSG_DAYS_REQUIRED.to_string());
            }
            VALIDITY_FIXED if self.expire_time.is_none() => {
                return Err(MSG_EXPIRE_TIME_REQUIRED.to_string());
            }
            _ => {}
        }
        if db_group_type == GROUP_TYPE_TOP_UP && self.face_amount.is_none() {
            return Err(MSG_FACE_AMOUNT_REQUIRED.to_string());
        }
        if db_group_type == GROUP_TYPE_EXCHANGE
            && self.gift_card_products.as_ref().map(Vec::len).unwrap_or(0) == 0
        {
            return Err(MSG_PRODUCTS_REQUIRED.to_string());
        }
        // 仅在本次入参带了兑换模式字段时校验（updateStatus 等部分更新不带这两个字段时跳过）
        if self.exchange_mode.is_some() || self.select_count.is_some() {
            validate_exchange_fields(
                db_group_type,
                self.exchange_mode,
                self.select_count,
                self.gift_card_products.as_ref().map(Vec::len).unwrap_or(0),
            )?;
        }
        // 适用范围同理：仅在本次入参带了 scope_type 时校验并更新
        if self.scope_type.is_some() {
            validate_scope_fields(
                db_group_type,
                self.scope_type,
                self.scope_product_ids.as_ref().map(Vec::len).unwrap_or(0),
                self.scope_category_ids.as_ref().map(Vec::len).unwrap_or(0),
            )?;
        }
        Ok(())
    }
}

/// 固定有效期 → VO 展示（Java `getExpireTime`：PERMANENT 清空、FIXED 格式化）。
#[must_use]
pub fn expire_time_text(validity_type: i32, expire_time: Option<i32>) -> Option<String> {
    match validity_type {
        VALIDITY_FIXED => Some(format_unix_time(expire_time)),
        _ => None,
    }
}

/// `validity_type` 的未知值校验（Java `default -> throw 未知的卡有效期类型`）。
pub fn validate_validity_type(validity_type: i32) -> Result<(), String> {
    if validity_type == VALIDITY_PERMANENT
        || validity_type == VALIDITY_NUMBER
        || validity_type == VALIDITY_FIXED
    {
        Ok(())
    } else {
        Err(MSG_UNKNOWN_VALIDITY_TYPE.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::marketing::gift_card::{
        VALIDITY_FIXED, VALIDITY_NUMBER, VALIDITY_PERMANENT,
    };

    #[test]
    fn end_of_day_parse_and_error_text() {
        // 2024-12-01 00:00:00 UTC+8 的 23:59:59 → 1733068799
        assert_eq!(
            parse_end_of_day_timestamp("2024-12-01 08:30:00").unwrap(),
            1_733_068_799
        );
        assert_eq!(
            parse_end_of_day_timestamp("2024-12-01").unwrap_err(),
            "日期格式错误"
        );
    }

    #[test]
    fn create_validation_texts_are_verbatim() {
        let p = GiftCardGroupsCreateParam {
            validity_type: Some(VALIDITY_NUMBER),
            ..Default::default()
        };
        assert_eq!(p.validate().unwrap_err(), "请填写天数有效期");
        let p = GiftCardGroupsCreateParam {
            validity_type: Some(VALIDITY_FIXED),
            ..Default::default()
        };
        assert_eq!(p.validate().unwrap_err(), "请填写固定时间有效期");
        let p = GiftCardGroupsCreateParam {
            validity_type: Some(VALIDITY_PERMANENT),
            group_type: Some(GROUP_TYPE_EXCHANGE),
            ..Default::default()
        };
        assert_eq!(p.validate().unwrap_err(), "请最少选择一个商品");
        let p = GiftCardGroupsCreateParam {
            validity_type: Some(VALIDITY_PERMANENT),
            group_type: Some(GROUP_TYPE_TOP_UP),
            ..Default::default()
        };
        assert_eq!(p.validate().unwrap_err(), "请填写面额");
        let p = GiftCardGroupsCreateParam {
            validity_type: Some(VALIDITY_PERMANENT),
            group_type: Some(GROUP_TYPE_TOP_UP),
            face_amount: Some(Decimal::new(1000, 2)),
            ..Default::default()
        };
        assert!(p.validate().is_ok());
    }

    #[test]
    fn update_validation_uses_db_group_type() {
        let p = GiftCardGroupsUpdateParam {
            validity_type: Some(VALIDITY_PERMANENT),
            ..Default::default()
        };
        assert_eq!(p.validate(GROUP_TYPE_TOP_UP).unwrap_err(), "请填写面额");
        assert_eq!(
            p.validate(GROUP_TYPE_EXCHANGE).unwrap_err(),
            "请最少选择一个商品"
        );
        let p = GiftCardGroupsUpdateParam {
            validity_type: Some(VALIDITY_NUMBER),
            validity_days: Some(30),
            face_amount: Some(Decimal::new(5000, 2)),
            ..Default::default()
        };
        assert!(p.validate(GROUP_TYPE_TOP_UP).is_ok());
    }

    #[test]
    fn names_and_validity_unknown_branch() {
        assert_eq!(group_type_name(1), "储值卡");
        assert_eq!(group_type_name(2), "兑换卡");
        assert_eq!(card_type_name(1), "电子卡");
        assert_eq!(card_type_name(2), "实体卡");
        assert_eq!(validity_type_name(1), "永久有效");
        assert_eq!(validity_type_name(2), "天数");
        assert_eq!(validity_type_name(3), "固定时间");
        assert_eq!(validate_validity_type(9).unwrap_err(), "未知的卡有效期类型");
        assert_eq!(expire_time_text(VALIDITY_PERMANENT, Some(123)), None);
        assert_eq!(
            expire_time_text(VALIDITY_FIXED, Some(0)),
            Some(String::new())
        );
    }

    #[test]
    fn exchange_fields_validation() {
        // 储值卡忽略
        assert!(validate_exchange_fields(GROUP_TYPE_TOP_UP, Some(9), None, 0).is_ok());
        // 固定套餐（缺省）不要求件数
        assert!(validate_exchange_fields(GROUP_TYPE_EXCHANGE, None, None, 3).is_ok());
        assert!(validate_exchange_fields(GROUP_TYPE_EXCHANGE, Some(1), None, 3).is_ok());
        // 模式非法
        assert_eq!(
            validate_exchange_fields(GROUP_TYPE_EXCHANGE, Some(3), None, 3).unwrap_err(),
            MSG_EXCHANGE_MODE_INVALID
        );
        // 任选缺件数 / 超绑定数
        assert_eq!(
            validate_exchange_fields(GROUP_TYPE_EXCHANGE, Some(2), None, 3).unwrap_err(),
            MSG_SELECT_COUNT_REQUIRED
        );
        assert_eq!(
            validate_exchange_fields(GROUP_TYPE_EXCHANGE, Some(2), Some(4), 3).unwrap_err(),
            MSG_SELECT_COUNT_EXCEEDS_PRODUCTS
        );
        assert!(validate_exchange_fields(GROUP_TYPE_EXCHANGE, Some(2), Some(3), 3).is_ok());
        // CreateParam 校验链：任选超数被拦
        let p = GiftCardGroupsCreateParam {
            validity_type: Some(VALIDITY_PERMANENT),
            group_type: Some(GROUP_TYPE_EXCHANGE),
            gift_card_products: Some(vec![GiftCardProductDTO::default()]),
            exchange_mode: Some(2),
            select_count: Some(2),
            ..Default::default()
        };
        assert_eq!(p.validate().unwrap_err(), MSG_SELECT_COUNT_EXCEEDS_PRODUCTS);
    }

    #[test]
    fn page_and_order_follow_base_page() {
        let q = GiftCardGroupsListQuery::default();
        assert_eq!(q.limit_offset(), (15, 0));
        assert_eq!(q.order_clause(), " ORDER BY card_groups_id DESC");
        let q = GiftCardGroupsListQuery {
            page: Some(2),
            size: Some(500),
            sort_field: Some("updateTime".into()),
            sort_order: Some("asc".into()),
            ..Default::default()
        };
        assert_eq!(q.limit_offset(), (100, 100));
        assert_eq!(q.order_clause(), " ORDER BY update_time ASC");
        let q = GiftCardGroupsListQuery {
            sort_field: Some("x; DROP".into()),
            ..Default::default()
        };
        assert_eq!(q.order_clause(), " ORDER BY card_groups_id DESC");
    }

    #[test]
    fn scope_validation_rules() {
        // 储值卡：默认全部商品通过；指定商品/品类必须有选择；非法类型报错
        assert!(validate_scope_fields(GROUP_TYPE_TOP_UP, None, 0, 0).is_ok());
        assert!(validate_scope_fields(GROUP_TYPE_TOP_UP, Some(0), 0, 0).is_ok());
        assert_eq!(
            validate_scope_fields(GROUP_TYPE_TOP_UP, Some(1), 0, 0).unwrap_err(),
            MSG_SCOPE_PRODUCTS_REQUIRED
        );
        assert!(validate_scope_fields(GROUP_TYPE_TOP_UP, Some(1), 2, 0).is_ok());
        assert_eq!(
            validate_scope_fields(GROUP_TYPE_TOP_UP, Some(2), 0, 0).unwrap_err(),
            MSG_SCOPE_CATEGORIES_REQUIRED
        );
        assert!(validate_scope_fields(GROUP_TYPE_TOP_UP, Some(2), 0, 3).is_ok());
        assert_eq!(
            validate_scope_fields(GROUP_TYPE_TOP_UP, Some(9), 0, 0).unwrap_err(),
            MSG_SCOPE_TYPE_INVALID
        );
        // 兑换卡忽略范围字段
        assert!(validate_scope_fields(GROUP_TYPE_EXCHANGE, Some(9), 0, 0).is_ok());
        // 创建参数集成：储值卡指定商品未选商品 → 报错
        let p = GiftCardGroupsCreateParam {
            group_type: Some(GROUP_TYPE_TOP_UP),
            validity_type: Some(VALIDITY_PERMANENT),
            face_amount: Some(Decimal::from(100)),
            scope_type: Some(1),
            ..Default::default()
        };
        assert_eq!(p.validate().unwrap_err(), MSG_SCOPE_PRODUCTS_REQUIRED);
    }
}
