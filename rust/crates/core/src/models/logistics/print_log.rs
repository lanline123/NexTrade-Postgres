//! 打印日志域（logistics::print_log）DTO / VO / 查询参数 + 枚举映射。
//!
//! 对应原 Java：
//! - `com.nextrade.adminapi.controller.print.PrintLogController`
//!   （`@RequestMapping("/adminapi/print/printLog")`）
//! - `com.nextrade.service.print.impl.PrintLogServiceImpl#page`
//! - `com.nextrade.bean.query.print.PrintLogPageQuery`（extends `BasePage`）
//! - `com.nextrade.bean.vo.print.PrintLogPageVO`
//! - `com.nextrade.bean.enums.print.PrintPlatformEnum` /
//!   `PrintLogStatusEnum` / `PrintTriggerTypeEnum`
//! - 表 `print_log`（18 列，权威列清单
//!   [`super::print_config::PRINT_LOG_COLUMNS`]）
//!
//! # 端点（1 条，路径逐字符对齐 `docs/api-contract.json`）
//!
//! | method | path | 原 Java | 落点（services 层） |
//! |--------|------|---------|---------------------|
//! | `GET` | `/adminapi/print/printLog/list` | `PrintLogController.list` | `PrintLogService::page` |
//!
//! 注意路径中段是**小驼峰 `printLog`**（不是 `print_log`），大小写敏感。
//!
//! # 分页 / 排序（对照 Java `PrintLogServiceImpl#page`）
//!
//! 1. `Page<PrintLog> page = buildSortOrder(query)`：`BasePage` 的
//!    `sortField` / `sortOrder` 决定**第一排序键**；空 `sortField` 视为主键
//!    `log_id`，`desc` 或空 `sortOrder` 一律降序 → 即默认 `log_id DESC`；
//! 2. wrapper 再叠加过滤条件，并在 `sortField` 为空时追加
//!    `ORDER BY add_time DESC`（Java 注释：「默认按 addTime 降序」）。
//!
//! 两者叠加后的净效果（本实现）：`sortField` 为空 →
//! `ORDER BY add_time DESC, log_id DESC`；显式 `sortField` →
//! `ORDER BY <列> ASC|DESC`。见仓库层 `resolve_order_clause`。
//!
//! # 多租户（`shopId`）
//!
//! Java `PrintLogServiceImpl#page` 用 `getShopId()` 过滤；与同域
//! `print` / `print_config` 一致，Rust 侧由路由层传入 `shop_id`，
//! 缺省 [`super::print_config::DEFAULT_SHOP_ID`]（平台 `0`）。
//!
//! # 金额
//!
//! 本域不涉及任何金额列，故无 `rust_decimal::Decimal`。

use serde::{Deserialize, Serialize};

use super::print::{platform_desc, PLATFORM_FEIEYUN, PLATFORM_SUNMI};

// ——————————————————————————————————————————————
// 常量（逐字符照抄 Java 枚举）
// ——————————————————————————————————————————————

/// 日志状态：待推送（Java `PrintLogStatusEnum.PENDING.getCode()`）。
pub const LOG_STATUS_PENDING: i32 = 0;

/// 日志状态：推送成功（Java `PrintLogStatusEnum.PUSHED.getCode()`）。
pub const LOG_STATUS_PUSHED: i32 = 1;

/// 日志状态：打印成功（Java `PrintLogStatusEnum.PRINTED.getCode()`）。
pub const LOG_STATUS_PRINTED: i32 = 2;

/// 日志状态：推送失败（Java `PrintLogStatusEnum.PUSH_FAILED.getCode()`）。
pub const LOG_STATUS_PUSH_FAILED: i32 = 3;

/// 日志状态：打印机离线（Java `PrintLogStatusEnum.OFFLINE.getCode()`）。
pub const LOG_STATUS_OFFLINE: i32 = 4;

/// 触发方式：自动（Java `PrintTriggerTypeEnum.AUTO.getCode()`）。
pub const TRIGGER_TYPE_AUTO: i32 = 1;

/// 触发方式：手动补打（Java `PrintTriggerTypeEnum.MANUAL.getCode()`）。
pub const TRIGGER_TYPE_MANUAL: i32 = 2;

/// Java `PrintLogStatusEnum#getDescByCode` 的兜底文案（也是
/// `PrintTriggerTypeEnum` 的兜底文案）。
pub const MSG_UNKNOWN: &str = "未知";

// ——————————————————————————————————————————————
// 枚举映射
// ——————————————————————————————————————————————

/// 日志状态名称（Java `PrintLogStatusEnum#getDescByCode`）：
/// 未知值（Java 里是 `null`）→ [`MSG_UNKNOWN`]。
///
/// 数据库列 `status` 是 `TINYINT NOT NULL`，Rust 实体是 `i32`，
/// 因此入参用 `i32`；与 Java `Integer` 入参的差异仅在「无法表达
/// `null`」，而 `null` 在库里不可能出现。
#[must_use]
pub fn log_status_desc(status: i32) -> &'static str {
    match status {
        LOG_STATUS_PENDING => "待推送",
        LOG_STATUS_PUSHED => "推送成功",
        LOG_STATUS_PRINTED => "打印成功",
        LOG_STATUS_PUSH_FAILED => "推送失败",
        LOG_STATUS_OFFLINE => "打印机离线",
        _ => MSG_UNKNOWN,
    }
}

/// 触发方式名称（Java `PrintTriggerTypeEnum#getDescByCode`）。
#[must_use]
pub fn trigger_type_desc(trigger_type: i32) -> &'static str {
    match trigger_type {
        TRIGGER_TYPE_AUTO => "自动",
        TRIGGER_TYPE_MANUAL => "手动补打",
        _ => MSG_UNKNOWN,
    }
}

// ——————————————————————————————————————————————
// 查询参数
// ——————————————————————————————————————————————

/// 打印日志分页查询参数
/// （Java `com.nextrade.bean.query.print.PrintLogPageQuery extends BasePage`）。
///
/// `BasePage` 提供 `page` / `size` / `sortField` / `sortOrder`；
/// `keyword` 在 Java 该查询里**未被使用**，故不定义（多传也不会报错，
/// serde 默认忽略未知字段）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PrintLogPageQuery {
    /// 平台 1 飞鹅云 2 商米云
    pub platform: Option<i32>,
    /// 状态 0 待推送 1 推送成功 2 打印成功 3 推送失败 4 打印机离线
    pub status: Option<i32>,
    /// 触发方式 1 自动 2 手动补打
    pub trigger_type: Option<i32>,
    /// 订单 ID
    pub order_id: Option<i32>,
    /// 打印机 SN（模糊）
    pub print_sn: Option<String>,
    /// 开始时间（Unix 秒，`add_time >= ?`）
    pub start_time: Option<i32>,
    /// 结束时间（Unix 秒，`add_time <= ?`）
    pub end_time: Option<i32>,
    /// 页码，缺省 1
    pub page: Option<u64>,
    /// 每页条数（Java `BasePage.size`，缺省 15，上限 100）
    #[serde(rename = "pageSize", alias = "size")]
    pub page_size: Option<u64>,
    /// 排序字段（Java 实体属性名或数据库列名，未知回退主键）
    pub sort_field: Option<String>,
    /// `asc` / `desc`（缺省降序）
    pub sort_order: Option<String>,
}

// ——————————————————————————————————————————————
// 出参
// ——————————————————————————————————————————————

/// 打印日志列表行（Java `com.nextrade.bean.vo.print.PrintLogPageVO`）。
///
/// Java 的 `printLogPageVO` 构造器**不返回** `pushResponse` /
/// `callbackPayload`（可能很大且含敏感报文），这里同样不定义。
///
/// ⚠️ 时间列保持**原始 Unix 秒整数**（Java VO 是 `Integer`，未做
/// `TigUtils#handelTime` 格式化），前端自行格式化。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintLogVO {
    /// 日志 ID（`BIGINT` → `i64`）
    pub log_id: i64,
    /// 打印机 ID
    pub print_id: i32,
    /// 打印机 SN 快照
    pub print_sn: Option<String>,
    /// 平台 1 飞鹅云 2 商米云
    pub platform: i32,
    /// 平台名称（展示用）
    pub platform_name: String,
    /// 订单 ID
    pub order_id: i32,
    /// 触发方式 1 自动 2 手动补打
    pub trigger_type: i32,
    /// 触发方式描述（展示用）
    pub trigger_type_desc: String,
    /// 状态 0~4
    pub status: i32,
    /// 状态描述（展示用）
    pub status_desc: String,
    /// 平台错误码
    pub error_code: Option<String>,
    /// 错误消息
    pub error_msg: Option<String>,
    /// 重试次数
    pub retry_count: i32,
    /// 推送时间（Unix 秒）
    pub pushed_time: Option<i32>,
    /// 打印成功时间（Unix 秒）
    pub printed_time: Option<i32>,
    /// 创建时间（Unix 秒）
    pub add_time: Option<i32>,
}

/// 平台常量再导出，便于路由/服务层只依赖本域模型。
///
/// 与 [`super::print`] 共用同一组值（Java 是同一个 `PrintPlatformEnum`），
/// 避免两处硬编码漂移。
pub const PLATFORM_FEIEYUN_CODE: i32 = PLATFORM_FEIEYUN;
/// 商米云平台码（见 [`PLATFORM_FEIEYUN_CODE`] 的说明）。
pub const PLATFORM_SUNMI_CODE: i32 = PLATFORM_SUNMI;

/// 平台名称（复用 [`super::print::platform_desc`] 的 Java 文案）。
#[must_use]
pub fn platform_name(platform: i32) -> &'static str {
    platform_desc(Some(platform))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 状态文案逐字对齐 Java `PrintLogStatusEnum`。
    #[test]
    fn log_status_descriptions_match_java_enum() {
        assert_eq!(log_status_desc(LOG_STATUS_PENDING), "待推送");
        assert_eq!(log_status_desc(LOG_STATUS_PUSHED), "推送成功");
        assert_eq!(log_status_desc(LOG_STATUS_PRINTED), "打印成功");
        assert_eq!(log_status_desc(LOG_STATUS_PUSH_FAILED), "推送失败");
        assert_eq!(log_status_desc(LOG_STATUS_OFFLINE), "打印机离线");
        // 与枚举数值逐条对齐
        assert_eq!(LOG_STATUS_PENDING, 0);
        assert_eq!(LOG_STATUS_PUSHED, 1);
        assert_eq!(LOG_STATUS_PRINTED, 2);
        assert_eq!(LOG_STATUS_PUSH_FAILED, 3);
        assert_eq!(LOG_STATUS_OFFLINE, 4);
        // 未知值兜底「未知」（Java `getDescByCode` 的 default 分支）
        assert_eq!(log_status_desc(-1), MSG_UNKNOWN);
        assert_eq!(log_status_desc(99), MSG_UNKNOWN);
    }

    /// 触发方式 / 平台文案逐字对齐 Java 枚举。
    #[test]
    fn trigger_and_platform_descriptions_match_java_enums() {
        assert_eq!(trigger_type_desc(TRIGGER_TYPE_AUTO), "自动");
        assert_eq!(trigger_type_desc(TRIGGER_TYPE_MANUAL), "手动补打");
        assert_eq!(trigger_type_desc(0), MSG_UNKNOWN);
        assert_eq!(TRIGGER_TYPE_AUTO, 1);
        assert_eq!(TRIGGER_TYPE_MANUAL, 2);

        assert_eq!(platform_name(PLATFORM_FEIEYUN_CODE), "飞鹅云");
        assert_eq!(platform_name(PLATFORM_SUNMI_CODE), "商米云");
        assert_eq!(platform_name(99), "未知平台");
        assert_eq!((PLATFORM_FEIEYUN_CODE, PLATFORM_SUNMI_CODE), (1, 2));
    }

    /// 查询参数：camelCase + `size` / `pageSize` 双写法。
    #[test]
    fn page_query_accepts_camel_case_and_size_alias() {
        let q: PrintLogPageQuery = serde_json::from_str(
            r#"{"platform":2,"status":3,"triggerType":2,"orderId":88,
                "printSn":"SN","startTime":1700000000,"endTime":1700000100,
                "page":2,"size":30,"sortField":"addTime","sortOrder":"asc"}"#,
        )
        .expect("PrintLogPageQuery 应接受 camelCase");
        assert_eq!(q.platform, Some(2));
        assert_eq!(q.status, Some(3));
        assert_eq!(q.trigger_type, Some(2));
        assert_eq!(q.order_id, Some(88));
        assert_eq!(q.print_sn.as_deref(), Some("SN"));
        assert_eq!(q.start_time, Some(1_700_000_000));
        assert_eq!(q.end_time, Some(1_700_000_100));
        assert_eq!(q.page, Some(2));
        assert_eq!(q.page_size, Some(30));
        assert_eq!(q.sort_field.as_deref(), Some("addTime"));
        assert_eq!(q.sort_order.as_deref(), Some("asc"));

        let q2: PrintLogPageQuery = serde_json::from_str(r#"{"pageSize":20}"#).unwrap();
        assert_eq!(q2.page_size, Some(20));
    }

    /// VO 序列化：camelCase、描述字段、原始时间戳、大整数主键。
    #[test]
    fn vo_serializes_camel_case_like_java() {
        let vo = PrintLogVO {
            log_id: 9_000_000_001,
            print_id: 3,
            print_sn: Some("SN-1".into()),
            platform: PLATFORM_SUNMI_CODE,
            platform_name: platform_name(PLATFORM_SUNMI_CODE).to_string(),
            order_id: 88,
            trigger_type: TRIGGER_TYPE_AUTO,
            trigger_type_desc: trigger_type_desc(TRIGGER_TYPE_AUTO).to_string(),
            status: LOG_STATUS_PRINTED,
            status_desc: log_status_desc(LOG_STATUS_PRINTED).to_string(),
            error_code: None,
            error_msg: Some("ok".into()),
            retry_count: 0,
            pushed_time: Some(1_700_000_001),
            printed_time: Some(1_700_000_002),
            add_time: Some(1_700_000_000),
        };
        let json = serde_json::to_value(&vo).unwrap();
        assert_eq!(json["logId"], 9_000_000_001_i64);
        assert_eq!(json["printId"], 3);
        assert_eq!(json["platformName"], "商米云");
        assert_eq!(json["triggerTypeDesc"], "自动");
        assert_eq!(json["statusDesc"], "打印成功");
        assert_eq!(json["retryCount"], 0);
        // 原始秒级时间戳，未格式化
        assert_eq!(json["addTime"], 1_700_000_000);
        assert!(json["errorCode"].is_null());
        // 大报文列不返回（与 Java VO 一致）
        assert!(json.get("pushResponse").is_none());
        assert!(json.get("callbackPayload").is_none());
    }
}
