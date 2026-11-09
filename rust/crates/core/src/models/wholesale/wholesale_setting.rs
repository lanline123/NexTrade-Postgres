//! 批发设置域（wholesale::wholesale_setting）DTO / VO + 纯逻辑。
//!
//! 对应原 Java：
//! - `com.nextrade.adminapi.controller.wholesale.WholesaleSettingController`
//!   （`@RequestMapping("/adminapi/wholesale/setting")`，2 个端点）
//! - `com.nextrade.service.wholesale.impl.WholesaleSettingServiceImpl`（174 行）
//! - `com.nextrade.bean.dto.wholesale.WholesaleSettingSaveDTO`
//! - `com.nextrade.bean.vo.wholesale.WholesaleSettingVO`
//! - `com.nextrade.bean.enums.setting.SettingsEnum` 的 4 个 `WHOLESALE_*` 配置项
//!
//! # 端点（2 条，路径逐字符对齐 `docs/api-contract.json`）
//!
//! | method | path | 原 Java | 落点（services 层） |
//! |--------|------|---------|---------------------|
//! | `GET`  | `/adminapi/wholesale/setting/get` | `WholesaleSettingController.get` | `WholesaleSettingService::get` |
//! | `POST` | `/adminapi/wholesale/setting/save` | `WholesaleSettingController.save` | `WholesaleSettingService::save` |
//!
//! # 配置项（落 `config` 表，`biz_code` 与 JSON 字段、`SettingsEnum.bizCode`
//! 三方完全同名）
//!
//! | biz_code | 默认值 | 含义 |
//! |----------|--------|------|
//! | `wholesaleEnabled` | `"0"` | 批发功能总开关 0 关闭 / 1 启用 |
//! | `wholesaleAutoRegister` | `"0"` | 新会员注册自动转批发商 |
//! | `wholesaleDefaultGradeId` | `"0"` | 自动转批发的默认等级 ID（0=未配置） |
//! | `wholesaleGivePoints` | `"0"` | 批发订单是否赠送积分 |
//!
//! 派生字段 `defaultGradeName` / `isMultiShop` 不带前缀（CR-VOS-4）。
//!
//! # 与 Java 的差异（`uncertain_points`）
//!
//! 1. **多店铺判定**：Java `isMultiShop()` 在 v2 单店铺前提下**恒 false**
//!    （5330 校验事实空跑）；本实现同样返回 `false` 并保留 `isMultiShop`
//!    字段，`save` 里的 5330 分支保留但不可达。
//! 2. **配置缓存**：Java `ConfigValCache#invalidateAndBroadcast` 与
//!    `wholesalerService.invalidateAllBuyerCache()` 未迁移（本仓无对应
//!    Redis 写路径）；本实现直接读写 `config` 表，下一次读取即为新值。
//! 3. **错误码映射**：Java `GlobalException(msg, code)` 带业务码 5303/5304/
//!    5331；Rust 侧 `Error::biz(code, msg)` 保留同一业务码。
//!
//! # 金额
//!
//! 本域不涉及任何金额列，故无 `rust_decimal::Decimal`。

use serde::{Deserialize, Serialize};

// ——————————————————————————————————————————————
// 配置 biz_code（逐字符照抄 Java `SettingsEnum`）
// ——————————————————————————————————————————————

/// 批发功能总开关（Java `SettingsEnum.WHOLESALE_ENABLED`）。
pub const BIZ_WHOLESALE_ENABLED: &str = "wholesaleEnabled";
/// 新会员注册自动转批发商（Java `SettingsEnum.WHOLESALE_AUTO_REGISTER`）。
pub const BIZ_WHOLESALE_AUTO_REGISTER: &str = "wholesaleAutoRegister";
/// 自动转批发的默认等级 ID（Java `SettingsEnum.WHOLESALE_DEFAULT_GRADE_ID`）。
pub const BIZ_WHOLESALE_DEFAULT_GRADE_ID: &str = "wholesaleDefaultGradeId";
/// 批发订单是否赠送积分（Java `SettingsEnum.WHOLESALE_GIVE_POINTS`）。
pub const BIZ_WHOLESALE_GIVE_POINTS: &str = "wholesaleGivePoints";

/// 4 个批发配置项的缺省值（Java `SettingsEnum#getDefaultValue()` 均为 `"0"`）。
pub const DEFAULT_INT_VALUE: i32 = 0;

// ——————————————————————————————————————————————
// 业务错误码（Java `HttpStatus` 5300 段）
// ——————————————————————————————————————————————

/// Java `HttpStatus.BAD_REQUEST`。
pub const CODE_BAD_REQUEST: i32 = 400;
/// Java `HttpStatus.WHOLESALE_GRADE_NOT_FOUND`（5303）。
pub const CODE_GRADE_NOT_FOUND: i32 = 5303;
/// Java `HttpStatus.WHOLESALE_GRADE_DISABLED`（5304）。
pub const CODE_GRADE_DISABLED: i32 = 5304;
/// Java `HttpStatus.WHOLESALE_MULTI_SHOP_NOT_SUPPORT`（5330）。
pub const CODE_MULTI_SHOP_NOT_SUPPORT: i32 = 5330;
/// Java `HttpStatus.WHOLESALE_DEFAULT_GRADE_NOT_CONFIGURED`（5331）。
pub const CODE_DEFAULT_GRADE_NOT_CONFIGURED: i32 = 5331;

// ——————————————————————————————————————————————
// 错误文案（逐字符照抄 Java）
// ——————————————————————————————————————————————

/// Java `save` 校验 1（C8 多店铺）。
pub const MSG_MULTI_SHOP_NOT_SUPPORT: &str = "多店铺暂不支持批发功能";
/// Java `save` 校验 2。
pub const MSG_DEFAULT_GRADE_NOT_CONFIGURED: &str = "启用批发功能前必须配置默认批发等级";
/// Java `WholesalerGradeServiceImpl#requireEnabled`：等级不存在。
pub const MSG_GRADE_NOT_FOUND: &str = "批发等级不存在";
/// Java `WholesalerGradeServiceImpl#requireEnabled`：等级已禁用。
pub const MSG_GRADE_DISABLED: &str = "批发等级已禁用";
/// Java `WholesaleSettingSaveDTO` 各字段 `@NotNull` 文案。
pub const MSG_ENABLED_REQUIRED: &str = "wholesaleEnabled 不能为空";
/// Java `@NotNull(message = "wholesaleAutoRegister 不能为空")`。
pub const MSG_AUTO_REGISTER_REQUIRED: &str = "wholesaleAutoRegister 不能为空";
/// Java `@NotNull(message = "wholesaleDefaultGradeId 不能为空")`。
pub const MSG_DEFAULT_GRADE_REQUIRED: &str = "wholesaleDefaultGradeId 不能为空";
/// Java `@NotNull(message = "wholesaleGivePoints 不能为空")`。
pub const MSG_GIVE_POINTS_REQUIRED: &str = "wholesaleGivePoints 不能为空";

// ——————————————————————————————————————————————
// 入参 / 出参
// ——————————————————————————————————————————————

/// 批发设置保存入参（Java `WholesaleSettingSaveDTO`）。
///
/// Java 4 个字段均 `@NotNull`；`0/1` 取值由 `@Min(0)` 兜底（未限制上界，
/// 本实现同样只校验非负）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WholesaleSettingSaveDTO {
    /// 批发功能总开关：0=关闭 1=启用（必填）
    pub wholesale_enabled: Option<i32>,
    /// 新会员注册自动转批发商：0=否 1=是（必填）
    pub wholesale_auto_register: Option<i32>,
    /// 自动转批发的默认等级 ID：0=未配置（必填）
    pub wholesale_default_grade_id: Option<i32>,
    /// 批发订单是否赠送积分：0=否 1=是（必填）
    pub wholesale_give_points: Option<i32>,
}

/// 批发设置 VO（Java `WholesaleSettingVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WholesaleSettingVO {
    /// 批发功能总开关：0=关闭 1=启用
    pub wholesale_enabled: i32,
    /// 新会员注册自动转批发商：0=否 1=是
    pub wholesale_auto_register: i32,
    /// 自动转批发的默认等级 ID：0=未配置
    pub wholesale_default_grade_id: i32,
    /// 批发订单是否赠送积分：0=否 1=是
    pub wholesale_give_points: i32,
    /// 默认等级名称（联表回填；不存在/已禁用时为 null）
    pub default_grade_name: Option<String>,
    /// 当前是否多店铺环境（v2 恒 false，见模块头差异第 1 条）
    pub is_multi_shop: bool,
}

// ——————————————————————————————————————————————
// 纯函数
// ——————————————————————————————————————————————

/// 校验保存入参（Java `@Valid WholesaleSettingSaveDTO` 的等价物）。
///
/// # Errors
/// 返回与 Java 注解 `message` 逐字符一致的文案。
pub fn validate_save(dto: &WholesaleSettingSaveDTO) -> std::result::Result<(), String> {
    if dto.wholesale_enabled.is_none() {
        return Err(MSG_ENABLED_REQUIRED.to_string());
    }
    if dto.wholesale_auto_register.is_none() {
        return Err(MSG_AUTO_REGISTER_REQUIRED.to_string());
    }
    if dto.wholesale_default_grade_id.is_none() {
        return Err(MSG_DEFAULT_GRADE_REQUIRED.to_string());
    }
    if dto.wholesale_give_points.is_none() {
        return Err(MSG_GIVE_POINTS_REQUIRED.to_string());
    }
    Ok(())
}

/// 配置值解析（Java `parseIntOrNull`）：空 / 非法 → `None`。
#[must_use]
pub fn parse_int(raw: Option<&str>) -> Option<i32> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<i32>().ok())
}

/// 读取配置的整数值（Java `readInt`）：缺失 / 非法回退
/// [`DEFAULT_INT_VALUE`]（`SettingsEnum#getDefaultValue()` = `"0"`）。
#[must_use]
pub fn parse_int_or_default(raw: Option<&str>) -> i32 {
    parse_int(raw).unwrap_or(DEFAULT_INT_VALUE)
}

/// `save` 的 3 项前置校验（不落库，便于单测）。
///
/// 逐条对照 Java `WholesaleSettingServiceImpl#save`：
/// 1. 多店铺 + 启用 → 5330（`is_multi_shop` 恒 false，事实不可达）
/// 2. 启用批发但未配置默认等级 → 5331
/// 3. 默认等级必须存在且启用 → 5303 / 5304（由调用方先查等级再传入）
///
/// `grade_enabled` 语义：`Some(true)` = 存在且启用；`Some(false)` = 已禁用；
/// `None` = 不存在（仅当 `default_grade_id > 0` 时才会被检查）。
///
/// # Errors
/// 返回 `(业务码, 文案)`，与 Java `GlobalException` 的 code/message 一致。
pub fn validate_business_rules(
    dto: &WholesaleSettingSaveDTO,
    is_multi_shop: bool,
    grade_enabled: Option<bool>,
) -> std::result::Result<(), (i32, String)> {
    let enabled = dto.wholesale_enabled.unwrap_or(DEFAULT_INT_VALUE);
    if is_multi_shop && enabled == 1 {
        return Err((
            CODE_MULTI_SHOP_NOT_SUPPORT,
            MSG_MULTI_SHOP_NOT_SUPPORT.to_string(),
        ));
    }
    let default_grade_id = dto.wholesale_default_grade_id.unwrap_or(DEFAULT_INT_VALUE);
    if enabled == 1 && default_grade_id <= 0 {
        return Err((
            CODE_DEFAULT_GRADE_NOT_CONFIGURED,
            MSG_DEFAULT_GRADE_NOT_CONFIGURED.to_string(),
        ));
    }
    if default_grade_id > 0 {
        match grade_enabled {
            Some(true) => {}
            Some(false) => return Err((CODE_GRADE_DISABLED, MSG_GRADE_DISABLED.to_string())),
            None => return Err((CODE_GRADE_NOT_FOUND, MSG_GRADE_NOT_FOUND.to_string())),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 4 个 biz_code 与 `SettingsEnum` 逐字符一致。
    #[test]
    fn biz_codes_match_settings_enum() {
        assert_eq!(BIZ_WHOLESALE_ENABLED, "wholesaleEnabled");
        assert_eq!(BIZ_WHOLESALE_AUTO_REGISTER, "wholesaleAutoRegister");
        assert_eq!(BIZ_WHOLESALE_DEFAULT_GRADE_ID, "wholesaleDefaultGradeId");
        assert_eq!(BIZ_WHOLESALE_GIVE_POINTS, "wholesaleGivePoints");
    }

    /// `readInt` 语义：缺失 / 空 / 非法 → 默认值 0。
    #[test]
    fn parse_int_falls_back_to_default() {
        assert_eq!(parse_int(None), None);
        assert_eq!(parse_int(Some("")), None);
        assert_eq!(parse_int(Some("  ")), None);
        assert_eq!(parse_int(Some("abc")), None);
        assert_eq!(parse_int(Some(" 1 ")), Some(1));
        assert_eq!(parse_int_or_default(None), 0);
        assert_eq!(parse_int_or_default(Some("x")), 0);
        assert_eq!(parse_int_or_default(Some("3")), 3);
    }

    /// `@Valid` 文案逐字符对齐 Java 注解。
    #[test]
    fn validate_save_messages_are_verbatim() {
        let mut dto = WholesaleSettingSaveDTO::default();
        assert_eq!(validate_save(&dto).unwrap_err(), MSG_ENABLED_REQUIRED);
        dto.wholesale_enabled = Some(1);
        assert_eq!(validate_save(&dto).unwrap_err(), MSG_AUTO_REGISTER_REQUIRED);
        dto.wholesale_auto_register = Some(0);
        assert_eq!(validate_save(&dto).unwrap_err(), MSG_DEFAULT_GRADE_REQUIRED);
        dto.wholesale_default_grade_id = Some(1);
        assert_eq!(validate_save(&dto).unwrap_err(), MSG_GIVE_POINTS_REQUIRED);
        dto.wholesale_give_points = Some(0);
        assert!(validate_save(&dto).is_ok());
    }

    /// 业务校验：5331 → 5303 → 5304 三条链路与 Java 一致。
    #[test]
    fn validate_business_rules_match_java() {
        let mut dto = WholesaleSettingSaveDTO {
            wholesale_enabled: Some(1),
            wholesale_auto_register: Some(0),
            wholesale_default_grade_id: Some(0),
            wholesale_give_points: Some(0),
        };
        // 启用但未配置默认等级 → 5331
        assert_eq!(
            validate_business_rules(&dto, false, None).unwrap_err(),
            (5331, MSG_DEFAULT_GRADE_NOT_CONFIGURED.to_string())
        );

        dto.wholesale_default_grade_id = Some(7);
        // 等级不存在 → 5303
        assert_eq!(
            validate_business_rules(&dto, false, None).unwrap_err(),
            (5303, MSG_GRADE_NOT_FOUND.to_string())
        );
        // 等级已禁用 → 5304
        assert_eq!(
            validate_business_rules(&dto, false, Some(false)).unwrap_err(),
            (5304, MSG_GRADE_DISABLED.to_string())
        );
        // 等级启用 → 通过
        assert!(validate_business_rules(&dto, false, Some(true)).is_ok());

        // 多店铺 + 启用 → 5330（v2 恒不可达，保留链路）
        assert_eq!(
            validate_business_rules(&dto, true, Some(true)).unwrap_err(),
            (5330, MSG_MULTI_SHOP_NOT_SUPPORT.to_string())
        );

        // 关闭批发且默认等级 0 → 不校验等级
        let closed = WholesaleSettingSaveDTO {
            wholesale_enabled: Some(0),
            wholesale_default_grade_id: Some(0),
            ..Default::default()
        };
        assert!(validate_business_rules(&closed, false, None).is_ok());
    }

    /// VO 序列化字段名与 Java `WholesaleSettingVO` 一致。
    #[test]
    fn setting_vo_serializes_camel_case() {
        let vo = WholesaleSettingVO {
            wholesale_enabled: 1,
            wholesale_auto_register: 0,
            wholesale_default_grade_id: 2,
            wholesale_give_points: 1,
            default_grade_name: Some("一级批发商".into()),
            is_multi_shop: false,
        };
        let json = serde_json::to_value(&vo).unwrap();
        assert_eq!(json["wholesaleEnabled"], 1);
        assert_eq!(json["wholesaleAutoRegister"], 0);
        assert_eq!(json["wholesaleDefaultGradeId"], 2);
        assert_eq!(json["wholesaleGivePoints"], 1);
        assert_eq!(json["defaultGradeName"], "一级批发商");
        assert_eq!(json["isMultiShop"], false);
    }
}
