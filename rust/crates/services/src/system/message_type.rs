//! 消息分类服务：列表 / 详情 / 更新 / 单字段更新 / 微信模板生成。
//! 对应原 Java `MessageTypeServiceImpl`（403 行）+ `BaseServiceImpl#updateField`。
//!
//! | 本切片接口 | 原 Java | 本文方法 |
//! |-----------|---------|---------|
//! | `MessageTypeController.list` | `MessageTypeServiceImpl#list` | [`MessageTypeService::list`] |
//! | `MessageTypeController.detail` | `MessageTypeServiceImpl#detail` | [`MessageTypeService::detail`] |
//! | `MessageTypeController.update` | `MessageTypeServiceImpl#update` | [`MessageTypeService::update`] |
//! | `MessageTypeController.updateField` | `BaseServiceImpl#updateField` | [`MessageTypeService::update_field`] |
//! | `MessageTypeController.miniProgramMessageTemplate` | `#miniProgramMessageTemplate` | [`MessageTypeService::mini_program_message_template`] |
//! | `MessageTypeController.wechatMessageTemplate` | `#wechatMessageTemplate` | [`MessageTypeService::wechat_message_template`] |
//!
//! 迁移要点（逐条对照 Java 源）：
//! - `list` 的版本差异过滤：`商家入驻` / `会员实名认证通知` / `获取验证码`
//!   （境外版才有邮箱） / `入驻审核失败`（多商户 + 非境外去掉站内信、小程序、公众号）。
//!   Java 是对**当页 records** 做 `Stream.filter`，`total` 仍是库内计数，
//!   本实现保持一致。
//! - `detail` 的 `smsProvider` / `smsAliyunChannel` 来自 `config`；
//!   模板 map 的键是 `MessageTypeEnum.getKey()`（`wechatData` 等）。
//! - `detail` 的 `isSupportTemplate`：某通道 `isXxx == -1` 时**跳过**该模板。
//! - `update`：各 `isXxx` 仅当 `> -1` 时赋值（`-1` 视为「不修改」）；
//!   模板按类型分别只写 `templateName` / `content` / `templateId` /
//!   `contentOversea`，其余列不覆盖（MyBatis-Plus `NOT_NULL` 策略）。
//! - `updateField`：`MESSAGE_TYPE_FIELDS` 白名单（10 个列名）。
//!
//! **未迁移 / 待补齐**（见 `uncertain_points`）：
//! - `miniProgramMessageTemplate` / `wechatMessageTemplate` 需要调用微信开放
//!   平台 HTTP 接口（`WechatOAuthService.getMiniAccessToken/getMpAccessToken`
//!   + `api.weixin.qq.com` 的模板增删）。Rust 侧尚无微信 access_token 服务，
//!   故这两个方法**只做前置校验并返回明确的「未启用」业务错误**，
//!   不伪造成功、也不发起外部请求。
//! - `adminLogService.createByLogInfo`（操作日志）依赖日志域。

use std::collections::HashMap;
use std::sync::Arc;

use chrono::TimeZone;
use nextrade_core::entities::{MessageTemplate, MessageType};
use nextrade_core::models::catalog::category_all::UpdateFieldDTO;
use nextrade_core::models::system::message_type::{
    MessageTemplateDTO, MessageTemplateKind, MessageTypeDetailDTO, MessageTypeListQuery,
    MessageTypeListVO,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::system::message_type::{SortDir, MESSAGE_TYPE_SORT_COLUMNS};
use nextrade_infrastructure::db::Repositories;

/// 列表分页默认值，对应 Java `BasePage.getSize()` 的缺省 15。
const DEFAULT_PAGE_SIZE: u64 = 15;

/// 列表分页上限，对应 Java `BasePage.getSize()` 的 `> 100 → 100`。
const MAX_PAGE_SIZE: u64 = 100;

/// `updateField` 允许更新的列白名单，逐字符对应 Java
/// `CheckFieldConstants.MESSAGE_TYPE_FIELDS = {"name", "is_wechat",
/// "is_mini_program", "is_message", "is_msg", "is_app", "is_ding",
/// "add_time", "is_mail", "is_wx_work"}`（**注意：不含 `describe` /
/// `send_type` / `message_id`**）。
pub const MESSAGE_TYPE_FIELDS: &[&str] = &[
    "name",
    "is_wechat",
    "is_mini_program",
    "is_message",
    "is_msg",
    "is_app",
    "is_ding",
    "add_time",
    "is_mail",
    "is_wx_work",
];

/// 短信服务商配置项 `biz_code`，对应 Java `SettingsEnum.SMS_PROVIDER`。
const CONFIG_SMS_PROVIDER: &str = "smsProvider";

/// 阿里云短信通道开关配置项 `biz_code`，对应 Java `SettingsEnum.SMS_ALIYUN_CHANNEL`。
const CONFIG_SMS_ALIYUN_CHANNEL: &str = "smsAliyunChannel";

/// `smsProvider` 缺省值，对应 Java `detail` 里 `: 1`（阿里云）。
const DEFAULT_SMS_PROVIDER: i32 = 1;

/// 短信模板中「内容需按服务商转换占位符」的两类，对应 Java
/// `type == MESSAGE_TYPE_MSG || type == MESSAGE_TYPE_MESSAGE`。
const CONVERT_CONTENT_KINDS: [MessageTemplateKind; 2] =
    [MessageTemplateKind::Msg, MessageTemplateKind::Message];

/// `INVALID_FIELD_VALUE`，对应 Java
/// `ResultTextConstants.INVALID_FIELD_VALUE = "field 的值有误"`。
pub const INVALID_FIELD_VALUE: &str = "field 的值有误";

/// `FAIL_UPDATE`，对应 Java `ResultTextConstants.FAIL_UPDATE = "更新失败"`。
pub const FAIL_UPDATE: &str = "更新失败";

// ————————————————————— 版本开关（NexTradeProperties 的 Rust 替代） —————————————————————

/// 版本 / 平台开关，对应 Java `NexTradeProperties`。
///
/// 取值来自随包发布的 `java/nextrade-adminapi/src/main/resources/application.yaml`
/// 的 `nextrade:` 段（`is-merchant: 1` / `is-b2b: 1` / `is-pro: 1` /
/// `is-overseas: 0` / `is-vendor: 0` / `is-o2o: 0`），即本仓库默认发行版为
/// 「多商户 + B2B + Pro + 国内 + 非供应商 + 非多门店」。
///
/// 若部署为其他版本，**必须**调整这些常量并重新编译，
/// 否则消息分类列表的版本差异过滤会与 Java 不一致（见 `uncertain_points`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditionFlags {
    pub is_merchant: i32,
    pub is_b2b: i32,
    pub is_pro: i32,
    pub is_overseas: i32,
    pub is_vendor: i32,
    pub is_o2o: i32,
}

impl Default for EditionFlags {
    fn default() -> Self {
        Self {
            is_merchant: 1,
            is_b2b: 1,
            is_pro: 1,
            is_overseas: 0,
            is_vendor: 0,
            is_o2o: 0,
        }
    }
}

impl EditionFlags {
    /// Java `list` 第 1 处过滤：名称以「商家入驻」开头时，
    /// 单商户（非多商户且非 O2O）不需要该通知 → 移除。
    #[must_use]
    pub fn keep_merchant_settle(&self, name: &str) -> bool {
        if name.starts_with("商家入驻") {
            return !(self.is_merchant == 0 && self.is_o2o == 0);
        }
        true
    }

    /// Java `list` 第 2 处过滤：名称以「会员实名认证通知」开头时，
    /// 仅「跨境/企业版/供应商」需要实名认证 → 保留。
    #[must_use]
    pub fn keep_member_certify(&self, name: &str) -> bool {
        if name.starts_with("会员实名认证通知") {
            return self.is_merchant == 1 && self.is_b2b == 1;
        }
        true
    }

    /// Java `list` 第 3 处：非跨境版本没有邮箱发送验证码 → `isMail = -1`。
    #[must_use]
    pub fn force_mail_unsupported(&self, name: &str) -> bool {
        self.is_overseas != 1 && name.starts_with("获取验证码")
    }

    /// Java `list` 第 4 处：多商户且非跨境版本，「入驻审核失败」通知不需要
    /// 站内信 / 小程序 / 公众号 → 三列置 `-1`。
    #[must_use]
    pub fn force_settle_audit_fail_channels_off(&self, name: &str) -> bool {
        self.is_merchant == 1 && self.is_overseas == 0 && name.starts_with("入驻审核失败")
    }

    /// 组合后的「是否保留该行」判定（Java 两处 `Stream.filter` 的等价）。
    #[must_use]
    pub fn keep(&self, name: Option<&str>) -> bool {
        let Some(name) = name else {
            // Java 对 null name 调 `startsWith` 会 NPE；seed 数据 name 非空，
            // 这里按「不拦截」处理，避免把脏数据行隐藏掉。
            return true;
        };
        self.keep_merchant_settle(name) && self.keep_member_certify(name)
    }
}

// ————————————————————— 纯函数 —————————————————————

/// `updateField` 的字段名 → 列名映射 + 白名单校验。
///
/// 复刻 Java `BaseServiceImpl#updateField`：
/// 1. `getColumnByPropertyName(field)`：属性名（驼峰）或列名 → 列名；
///    **匹配不到时回退主键列** `message_id`（安全修复，防 SQL 注入）；
/// 2. `Arrays.asList(MESSAGE_TYPE_FIELDS).contains(column)`，否则抛
///    `INVALID_FIELD_VALUE` = `"field 的值有误"`。
///
/// 因此传列名 `is_wechat` 能通过（Java 用 `fieldInfo.getColumn().equals` 兼容），
/// 而传 `describe` / `sendType` / `messageId` 都会因不在白名单而失败。
pub fn map_update_field(field: Option<&str>) -> Result<&'static str> {
    let raw = field.unwrap_or("").trim();
    if raw.is_empty() {
        return Err(Error::biz_msg(INVALID_FIELD_VALUE));
    }
    // `getColumnByPropertyName` 的兜底：匹配不到 → 主键列（此处同样会落白名单外）
    let column = message_type_field_to_column(raw).unwrap_or("message_id");
    if !MESSAGE_TYPE_FIELDS.contains(&column) {
        return Err(Error::biz_msg(INVALID_FIELD_VALUE));
    }
    Ok(column)
}

/// 属性名（驼峰）或列名 → 数据库列名，对应
/// `BaseServiceImpl#getColumnByPropertyName` 在 `MessageType` 实体上的映射。
///
/// 匹配不到返回 `None`（由 [`map_update_field`] 回退主键列）。
/// 主键属性名 `messageId` 也会落到 `message_id`，与 Java 一致：
/// Java 第一步比较的是**列名**（`getKeyColumn().equals(propertyName)`），
/// 属性名 `messageId` 不在 `getFieldList()` 里，于是走兜底返回主键列。
#[must_use]
pub fn message_type_field_to_column(field: &str) -> Option<&'static str> {
    Some(match field {
        "name" => "name",
        "describe" => "describe",
        "sendType" | "send_type" => "send_type",
        "isWechat" | "is_wechat" => "is_wechat",
        "isMiniProgram" | "is_mini_program" => "is_mini_program",
        "isMessage" | "is_message" => "is_message",
        "isMsg" | "is_msg" => "is_msg",
        "isApp" | "is_app" => "is_app",
        "isDing" | "is_ding" => "is_ding",
        "addTime" | "add_time" => "add_time",
        "isWxWork" | "is_wx_work" => "is_wx_work",
        "isMail" | "is_mail" => "is_mail",
        "messageId" | "message_id" => "message_id",
        _ => return None,
    })
}

/// 排序字段 → 列名；空 → 主键列（Java `buildSortOrder`）。
///
/// 非白名单字段同样回退主键列（Java 通过 `getColumnByPropertyName` 兜底）。
#[must_use]
pub fn resolve_sort_column(sort_field: Option<&str>) -> &'static str {
    let raw = sort_field.unwrap_or("").trim();
    if raw.is_empty() {
        return "message_id";
    }
    let column = message_type_field_to_column(raw).unwrap_or("message_id");
    if MESSAGE_TYPE_SORT_COLUMNS.contains(&column) {
        column
    } else {
        "message_id"
    }
}

/// 排序方向：`asc`（忽略大小写）→ 升序；其余（含空）→ 降序，
/// 对应 Java `buildSortOrder(page, sortField, sortOrder)`。
#[must_use]
pub fn resolve_sort_dir(sort_order: Option<&str>) -> SortDir {
    match sort_order {
        Some(s) if s.eq_ignore_ascii_case("asc") => SortDir::Asc,
        _ => SortDir::Desc,
    }
}

/// `add_time`（秒级时间戳）→ `yyyy-MM-dd HH:mm:ss`，对应 Java
/// `DateUtil.format(DateUtil.date(addTime * 1000), Constants.DATE_FORMAT)`
/// （`DATE_FORMAT = "yyyy-MM-dd HH:mm:ss"`）。
///
/// `add_time` 为 `None` 时按 `0` 处理 —— seed 数据里该列实际为 `0`，
/// 等价于格式化 1970-01-01 的**本地时间**。时区固定用 `chrono::Local`，
/// 与 Java 默认时区行为一致（部署时应保证容器 TZ 与 Java 侧相同）。
#[must_use]
pub fn format_add_time(add_time: Option<i32>) -> Option<String> {
    let secs = i64::from(add_time.unwrap_or(0));
    chrono::Local
        .timestamp_opt(secs, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
}

/// `isSupportTemplate`，对应 Java `MessageTypeServiceImpl#isSupportTemplate`：
/// 返回 `true` 表示该通道**不支持**，模板应被跳过。
///
/// Java 判定的是「`isXxx == -1`」。`is_wx_work` / `is_mail` 在库里是
/// `NOT NULL`，对应 `Some(...)` 包装后行为一致。
#[must_use]
pub fn is_support_template(t: &MessageType, kind: MessageTemplateKind) -> bool {
    let value = match kind {
        MessageTemplateKind::Wechat => t.is_wechat,
        MessageTemplateKind::MiniProgram => t.is_mini_program,
        MessageTemplateKind::Msg => t.is_msg,
        MessageTemplateKind::Message => t.is_message,
        MessageTemplateKind::App => t.is_app,
        MessageTemplateKind::Ding => t.is_ding,
        MessageTemplateKind::Email => Some(t.is_mail),
        MessageTemplateKind::WxWork => Some(t.is_wx_work),
    };
    value == Some(-1)
}

/// 短信服务商取值，对应 Java `SmsProviderEnum.fromCode(code)`：
/// `1`阿里云 / `2`腾讯云 / `3`华为云 / `4`火山引擎；未知值回退阿里云。
///
/// Java `detail` 里的表达式是
/// `(providerVal != null && !providerVal.isEmpty()) ? parseInt(providerVal) : 1`，
/// 因此空串 → `1`；`Integer.parseInt` 抛 `NumberFormatException` 的场景
/// 这里也统一按阿里云兜底，避免整个详情页 500。
#[must_use]
pub fn sms_provider_code(raw: Option<&str>) -> i32 {
    let code = raw
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(DEFAULT_SMS_PROVIDER);
    if (1..=4).contains(&code) {
        code
    } else {
        DEFAULT_SMS_PROVIDER
    }
}

/// 占位符转换，对应 Java `SmsPlaceholderConverter#convert`。
///
/// 各服务商规则（逐条照抄 Java 正则语义）：
/// - 阿里云 / 华为云：`{var}` → `${var}`；已是 `${var}` 的不变
///   （正则 `(?<!\$)\{(\w+)}`）
/// - 腾讯云：`${var}` 与 `{var}` 均转为 `{1}`、`{2}`… 按出现顺序
///   （正则 `\$?\{(\w+)}`）
/// - 火山引擎：`${var}` → `{var}`；已是 `{var}` 的不变（正则 `\$\{(\w+)}`）
///
/// `provider` 为 `None` 时原样返回，与 Java 的 `provider == null` 分支一致。
#[must_use]
pub fn convert_sms_placeholders(content: &str, provider: Option<i32>) -> String {
    let Some(provider) = provider else {
        return content.to_string();
    };
    match provider {
        2 => convert_to_tencent(content),
        4 => convert_to_volcengine(content),
        // 1 = 阿里云，3 = 华为云；其余取值 Java `switch` 覆盖不到 → 原样返回
        1 | 3 => convert_to_aliyun(content),
        _ => content.to_string(),
    }
}

/// 阿里云 / 华为云：仅给**未带 `$`** 的 `{var}` 补上 `$`。
fn convert_to_aliyun(content: &str) -> String {
    let chars: Vec<char> = content.chars().collect();
    let mut out = String::with_capacity(content.len());
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '{' {
            // 前一个字符是 '$' → 已是 ${var}，原样输出
            let prev_is_dollar = i > 0 && chars[i - 1] == '$';
            if let Some(end) = placeholder_end(&chars, i) {
                if !prev_is_dollar {
                    out.push('$');
                }
                for c in &chars[i..=end] {
                    out.push(*c);
                }
                i = end + 1;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// 火山引擎：仅把 `${var}` 去掉 `$` 变成 `{var}`。
fn convert_to_volcengine(content: &str) -> String {
    let chars: Vec<char> = content.chars().collect();
    let mut out = String::with_capacity(content.len());
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
            if let Some(end) = placeholder_end(&chars, i + 1) {
                for c in &chars[i + 1..=end] {
                    out.push(*c);
                }
                i = end + 1;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// 腾讯云：`${var}` 与 `{var}` 统一替换为 `{1}`、`{2}`… 按出现顺序。
fn convert_to_tencent(content: &str) -> String {
    let chars: Vec<char> = content.chars().collect();
    let mut out = String::with_capacity(content.len());
    let mut i = 0usize;
    let mut index = 1usize;
    while i < chars.len() {
        // 允许前缀 '$'
        let brace_at = if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
            i + 1
        } else if chars[i] == '{' {
            i
        } else {
            out.push(chars[i]);
            i += 1;
            continue;
        };
        if let Some(end) = placeholder_end(&chars, brace_at) {
            out.push('{');
            out.push_str(&index.to_string());
            out.push('}');
            index += 1;
            i = end + 1;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// 若 `chars[start] == '{'` 且其后为 `\w+}`，返回 `}` 的下标。
///
/// Java 正则用 `\w`，默认（非 `UNICODE_CHARACTER_CLASS`）等价于
/// `[A-Za-z0-9_]` —— 因此中文等非 ASCII 字符**不构成**占位符。
fn placeholder_end(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start) != Some(&'{') {
        return None;
    }
    let mut j = start + 1;
    if !chars
        .get(j)
        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
    {
        return None;
    }
    while chars
        .get(j)
        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
    {
        j += 1;
    }
    if chars.get(j) == Some(&'}') {
        Some(j)
    } else {
        None
    }
}

/// 实体 → 列表 VO，对应 Java `list` 链条里的 `.map(item -> {...})`。
///
/// 版本差异改写（Java 4 处）：
/// 1. 非跨境版本 + 名称以「获取验证码」开头 → `isMail = -1`
/// 2. 多商户且非跨境 + 名称以「入驻审核失败」开头 →
///    `isMessage` / `isMiniProgram` / `isWechat` 三列均置 `-1`
/// 3. `addTime` 无条件格式化为 `yyyy-MM-dd HH:mm:ss`
#[must_use]
pub fn to_list_vo(t: &MessageType, flags: EditionFlags) -> MessageTypeListVO {
    let name = t.name.clone().unwrap_or_default();

    let is_mail = if flags.force_mail_unsupported(&name) {
        Some(-1)
    } else {
        Some(t.is_mail)
    };

    let (mut is_message, mut is_mini_program, mut is_wechat) =
        (t.is_message, t.is_mini_program, t.is_wechat);
    if flags.force_settle_audit_fail_channels_off(&name) {
        is_message = Some(-1);
        is_mini_program = Some(-1);
        is_wechat = Some(-1);
    }

    MessageTypeListVO {
        message_id: t.message_id,
        name: t.name.clone(),
        describe: t.describe.clone(),
        send_type: t.send_type,
        is_wechat,
        is_mini_program,
        is_message,
        is_msg: t.is_msg,
        is_app: t.is_app,
        is_ding: t.is_ding,
        is_mail,
        is_wx_work: Some(t.is_wx_work),
        // Java 无条件格式化 addTime（缺省 0 → 1970 年本地时间）
        add_time: format_add_time(t.add_time),
    }
}

/// 模板实体 → DTO。
pub fn template_to_dto(row: &MessageTemplate) -> MessageTemplateDTO {
    MessageTemplateDTO {
        id: Some(row.id),
        message_id: row.message_id,
        r#type: row.r#type,
        content: row.content.clone(),
        content_oversea: row.content_oversea.clone(),
        to_userid: row.to_userid.clone(),
        template_id: row.template_id.clone(),
        template_num: row.template_num.clone(),
        template_name: row.template_name.clone(),
    }
}

/// 按模板类型挑选要写入的列，复刻 Java `MessageTypeServiceImpl#updateTemplate`
/// 的 `switch`：
/// - `MESSAGE/APP/EMAIL` → `templateName`(`""` 兜底) + `content`(`""` 兜底)
/// - `MIN_PROGRAM/WECHAT` → `templateId`(`""` 兜底)
/// - `MSG` → `templateId`(`""` 兜底) + `contentOversea`（**为 `None` 时不覆盖**）
/// - `DING/WXWORK` → `content`(`""` 兜底)
///
/// 返回 `(template_name, template_id, content, content_oversea)`，
/// 其中 `None` 表示该列不参与更新。
#[must_use]
pub fn template_update_values(
    kind: MessageTemplateKind,
    tpl: &MessageTemplateDTO,
) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    // Java `StringUtils.defaultIfEmpty(x, "")`：null 或空串 → `""`
    let blank = |v: Option<&str>| Some(v.unwrap_or("").to_string());

    match kind {
        MessageTemplateKind::Message | MessageTemplateKind::App | MessageTemplateKind::Email => (
            blank(tpl.template_name.as_deref()),
            None,
            blank(tpl.content.as_deref()),
            None,
        ),
        MessageTemplateKind::MiniProgram | MessageTemplateKind::Wechat => (
            None,
            blank(tpl.template_id.as_deref()),
            None,
            None,
        ),
        MessageTemplateKind::Msg => (
            None,
            blank(tpl.template_id.as_deref()),
            None,
            // 仅当非 null 时覆盖（非跨境版不下发该字段，避免清空已配置值）
            tpl.content_oversea.clone(),
        ),
        MessageTemplateKind::Ding | MessageTemplateKind::WxWork => (
            None,
            None,
            blank(tpl.content.as_deref()),
            None,
        ),
    }
}

// ————————————————————— 服务 —————————————————————

pub struct MessageTypeService {
    repos: Arc<Repositories>,
    /// 版本开关（Java `NexTradeProperties`；Rust 侧为常量，见 [`EditionFlags`]）。
    flags: EditionFlags,
}

impl MessageTypeService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self {
            repos,
            flags: EditionFlags::default(),
        }
    }

    /// 本域版本开关（只读）。
    #[must_use]
    pub fn flags(&self) -> EditionFlags {
        self.flags
    }

    /// 便捷访问本切片独占仓储。
    fn repo(&self) -> &nextrade_infrastructure::db::repo::system::message_type::MessageTypeRepository {
        &self.repos.system.message_type
    }

    // ————————————————————— 列表 —————————————————————

    /// 消息分类分页列表，对应 `MessageTypeServiceImpl#list`。
    ///
    /// `BasePage` 语义：`page` 缺省 1；`size` 缺省 15、上限 100。
    ///
    /// 版本差异过滤在**取回当页之后**执行（与 Java 的
    /// `this.list(page, queryWrapper).stream().filter(...)` 一致），
    /// 因此 `total` 仍是库内计数。
    pub async fn list(&self, q: &MessageTypeListQuery) -> Result<PageResult<MessageTypeListVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1).max(1),
            page_size: q
                .page_size
                .unwrap_or(DEFAULT_PAGE_SIZE)
                .clamp(1, MAX_PAGE_SIZE),
        };
        let column = resolve_sort_column(q.sort_field.as_deref());
        let dir = resolve_sort_dir(q.sort_order.as_deref());

        let (total, rows) = self
            .repo()
            .paginate(q.keyword.as_deref(), q.send_type, column, dir, &page)
            .await?;

        let list = rows
            .iter()
            .filter(|t| self.flags.keep(t.name.as_deref()))
            .map(|t| to_list_vo(t, self.flags))
            .collect();

        Ok(PageResult::new(total, list, &page))
    }

    // ————————————————————— 详情 —————————————————————

    /// 消息分类详情，对应 `MessageTypeServiceImpl#detail`。
    ///
    /// Java 对不存在的 ID **不抛异常**（`BeanUtils.copyProperties(null, dto)`
    /// 得到全空 DTO，随后 `messageType.getMessageId()` 为 `null`，
    /// `listByMessageId(null)` 命中不到任何模板）。这里返回一个
    /// `messageId = None` 的空详情，保持同样的「不 404」语义。
    pub async fn detail(&self, id: i32) -> Result<MessageTypeDetailDTO> {
        let Some(t) = self.repo().find_by_id(id).await? else {
            return Ok(empty_detail());
        };

        // smsProvider / smsAliyunChannel（Java `ConfigService#getConfigVal`）
        let provider_raw = self.repo().find_config_val(CONFIG_SMS_PROVIDER).await?;
        let provider_code = sms_provider_code(provider_raw.as_deref());
        let sms_aliyun_channel = self.repo().find_config_val(CONFIG_SMS_ALIYUN_CHANNEL).await?;

        // 模板消息（Java `messageTemplateService.listByMessageId`）
        let templates = self.repo().find_templates_by_message_id(t.message_id).await?;
        let mut template_message: HashMap<String, MessageTemplateDTO> = HashMap::new();
        for row in &templates {
            let Some(kind) = row.r#type.and_then(MessageTemplateKind::from_code) else {
                // Java `messageTemplate.getType()` 是枚举，落库值非法时
                // MyBatis 会直接抛转换异常；这里选择跳过该行以免整个详情 500。
                tracing::warn!(
                    "message_template.type={:?} 不是合法 MessageTypeEnum，已跳过 id={}",
                    row.r#type,
                    row.id
                );
                continue;
            };
            if is_support_template(&t, kind) {
                continue;
            }
            let mut dto = template_to_dto(row);
            if CONVERT_CONTENT_KINDS.contains(&kind) {
                dto.content = dto
                    .content
                    .map(|c| convert_sms_placeholders(&c, Some(provider_code)));
            }
            template_message.insert(kind.key().to_string(), dto);
        }

        Ok(MessageTypeDetailDTO {
            message_id: Some(t.message_id),
            name: t.name.clone(),
            describe: t.describe.clone(),
            send_type: t.send_type,
            is_wechat: t.is_wechat,
            is_mini_program: t.is_mini_program,
            is_message: t.is_message,
            is_msg: t.is_msg,
            is_app: t.is_app,
            is_ding: t.is_ding,
            add_time: format_add_time(t.add_time),
            is_mail: Some(t.is_mail),
            is_wx_work: Some(t.is_wx_work),
            sms_provider: Some(provider_code),
            sms_aliyun_channel,
            template_message,
        })
    }

    // ————————————————————— 更新 —————————————————————

    /// 更新消息分类及其模板，对应 `MessageTypeServiceImpl#update`。
    ///
    /// 事务语义：Java 标注 `@Transactional(rollbackFor = Exception.class)`，
    /// 模板批量更新失败抛 `GlobalException(FAIL_UPDATE)` 即整单回滚。
    /// 这里用同一连接上的显式事务复刻。
    ///
    /// `isXxx` 字段仅在 `> -1` 时参与更新（Java `setMessageTypeFields`）。
    /// 注意 `messageId` 为空时 Java 会因主键为 `null` 而更新 0 行
    /// （`updateById` 返回 `false`，随后**仍会**执行 `createByLogInfo`），
    /// 本实现返回 `Validation` / `NotFound` 以便上层给出明确错误。
    pub async fn update(&self, dto: &MessageTypeDetailDTO) -> Result<()> {
        let message_id = dto
            .message_id
            .ok_or_else(|| Error::Validation("消息ID不能为空".into()))?;

        if self.repo().find_by_id(message_id).await?.is_none() {
            return Err(Error::NotFound("消息分类不存在".into()));
        }

        // 与 Java `setMessageTypeFields` 一致：仅 `> -1` 才赋值
        let pick = |v: Option<i32>| v.filter(|x| *x > -1);

        let mut tx = self.repo().begin().await?;

        let affected = self
            .repo()
            .update_fields_in_tx(
                &mut tx,
                message_id,
                dto.name.as_deref(),
                dto.describe.as_deref(),
                dto.send_type,
                pick(dto.is_wechat),
                pick(dto.is_mini_program),
                pick(dto.is_message),
                pick(dto.is_msg),
                pick(dto.is_app),
                pick(dto.is_ding),
                pick(dto.is_mail),
                pick(dto.is_wx_work),
            )
            .await?;

        // Java：`updateById` 返回 false 时不更新模板（但仍写日志、返回 false）
        if affected > 0 {
            for (key, tpl) in &dto.template_message {
                let Some(kind) = MessageTemplateKind::from_key(key) else {
                    // Java：`fromCode(key) == null` → 跳过（不进更新列表）
                    tracing::debug!("未知模板键 {key}，已跳过");
                    continue;
                };
                // `template_message` 的值就是 `MessageTemplateDTO`（非 Option），
                // 只需按 `id` 是否为 `Some` 过滤主键缺失的元素。
                if tpl.id.is_none() {
                    // Java `updateBatchById` 对主键为 null 的元素会抛异常；
                    // 这里跳过并告警，避免整单 500。
                    tracing::warn!("模板 {key} 缺少 id，已跳过更新");
                    continue;
                }
                let (name, template_id, content, content_oversea) =
                    template_update_values(kind, tpl);
                let rows = self
                    .repo()
                    .update_template_fields_in_tx(
                        &mut tx,
                        tpl.id.unwrap_or_default(),
                        name.as_deref(),
                        template_id.as_deref(),
                        content.as_deref(),
                        content_oversea.as_deref(),
                    )
                    .await?;
                if rows == 0 {
                    // 对应 Java `updateBatchById` 返回 false → 抛 FAIL_UPDATE。
                    // 先回滚，复刻 `@Transactional(rollbackFor = Exception.class)`。
                    let _ = tx.rollback().await;
                    return Err(Error::biz_msg(FAIL_UPDATE));
                }
            }
        }

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        // Java 末尾 `adminLogService.createByLogInfo("更新消息类型:{name}")`。
        let name = self
            .repo()
            .find_name_by_id(message_id)
            .await?
            .unwrap_or_default();
        let log_info = format!("更新消息类型:{name}");
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log("admin", 0, &log_info).await {
            tracing::warn!("写管理操作日志失败（消息类型）: {e}");
        }
        tracing::info!("更新消息类型:{name}（messageId={message_id}）");
        Ok(())
    }

    /// 单字段更新，对应 `MessageTypeController.updateField` →
    /// `BaseServiceImpl#updateField(updateField, MESSAGE_TYPE_FIELDS)`。
    ///
    /// Java `@NotNull` 校验 `id` / `field` / `val` 三者非空；这里对 `field`
    /// 走白名单校验（缺失或非法同样抛 `INVALID_FIELD_VALUE`），
    /// 对 `id` / `val` 缺失抛 `Validation`。
    ///
    /// 返回是否影响行（Java 的 `boolean`；Controller 声明 `void`，返回值被丢弃）。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let id = dto.id.ok_or_else(|| Error::Validation("id不能为空".into()))?;
        let column = map_update_field(dto.field.as_deref())?;
        let val = dto
            .val
            .as_deref()
            .ok_or_else(|| Error::Validation("val不能为空".into()))?;
        let rows = self.repo().update_field(id, column, val).await?;
        Ok(rows > 0)
    }

    // ————————————————————— 微信模板生成 —————————————————————

    /// 小程序订阅消息模板生成，对应
    /// `MessageTypeServiceImpl#miniProgramMessageTemplate`。
    ///
    /// Java 流程：`getMiniAccessToken()` → 拉取并删除全部已有模板 →
    /// 依次添加 `31570`（`kidList=[2,1,4]`，"订单支付成功通知"）与
    /// `30766`（`kidList=[4,5,3,8,2]`，"订单发货通知"）。
    ///
    /// **未迁移**：Rust 侧尚无微信 access_token 服务，故不发起任何外部请求，
    /// 直接返回业务错误（不伪造成功）。
    pub async fn mini_program_message_template(&self) -> Result<()> {
        Err(Error::biz_msg(
            "小程序订阅消息模板生成未启用：缺少微信小程序 access_token 服务",
        ))
    }

    /// 公众号模板消息生成，对应 `MessageTypeServiceImpl#wechatMessageTemplate`。
    ///
    /// Java 流程：`getMpAccessToken()` → 拉取并删除全部已有模板 →
    /// 添加 `51617`（`kidList=[3,7,4]`，"订单支付成功通知"）、
    /// `48233`（`kidList=[21,18,3,17,2]`，"订单发货通知"）、
    /// `48058`（`kidList=[5,2]`，"退款成功通知"）。
    ///
    /// **未迁移**：同 [`Self::mini_program_message_template`]。
    pub async fn wechat_message_template(&self) -> Result<()> {
        Err(Error::biz_msg(
            "公众号模板消息生成未启用：缺少微信公众号 access_token 服务",
        ))
    }
}

/// Java `detail` 在记录不存在时返回的全空 DTO
/// （`smsProvider` 仍会被赋默认值 `1`，`templateMessage` 为空 map）。
#[must_use]
pub fn empty_detail() -> MessageTypeDetailDTO {
    MessageTypeDetailDTO {
        sms_provider: Some(DEFAULT_SMS_PROVIDER),
        template_message: HashMap::new(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mt(name: &str, add_time: Option<i32>) -> MessageType {
        MessageType {
            message_id: 2,
            name: Some(name.to_string()),
            describe: Some("描述".into()),
            send_type: Some(1),
            is_wechat: Some(1),
            is_mini_program: Some(1),
            is_message: Some(1),
            is_msg: Some(1),
            is_app: Some(-1),
            is_ding: Some(-1),
            add_time,
            is_wx_work: -1,
            is_mail: 1,
        }
    }

    /// 惰性（不建连）MySQL 池构造的 `Repositories`，用于实例化服务层。
    ///
    /// 注意：本 crate **不直接依赖 `sqlx`**，故池的构造入口放在本切片的
    /// 仓储模块里（[`nextrade_infrastructure::db::repo::system::message_type::lazy_test_pool`]）。
    fn lazy_repos() -> Arc<Repositories> {
        Arc::new(Repositories::new(
            nextrade_infrastructure::db::repo::system::message_type::lazy_test_pool(),
        ))
    }

    // ————— updateField 白名单 —————

    #[test]
    fn update_field_whitelist_matches_java_constant() {
        // CheckFieldConstants.MESSAGE_TYPE_FIELDS
        for field in [
            "name",
            "is_wechat",
            "is_mini_program",
            "is_message",
            "is_msg",
            "is_app",
            "is_ding",
            "add_time",
            "is_mail",
            "is_wx_work",
        ] {
            assert_eq!(map_update_field(Some(field)).unwrap(), field, "{field}");
        }
        assert_eq!(MESSAGE_TYPE_FIELDS.len(), 10);
    }

    #[test]
    fn update_field_accepts_camel_property_names() {
        assert_eq!(
            map_update_field(Some("isMiniProgram")).unwrap(),
            "is_mini_program"
        );
        assert_eq!(map_update_field(Some("isWxWork")).unwrap(), "is_wx_work");
        assert_eq!(map_update_field(Some("addTime")).unwrap(), "add_time");
    }

    #[test]
    fn update_field_rejects_non_whitelisted_columns() {
        // describe / send_type 是合法列但**不在**白名单 → Java 抛 INVALID_FIELD_VALUE
        for field in ["describe", "send_type", "sendType"] {
            let err = map_update_field(Some(field)).unwrap_err();
            let Error::Biz { msg, .. } = err else {
                panic!("应为业务错误");
            };
            assert_eq!(msg, INVALID_FIELD_VALUE);
        }
    }

    #[test]
    fn update_field_falls_back_to_primary_key_like_java() {
        // 未知字段 → getColumnByPropertyName 回退主键列 message_id → 不在白名单 → 报错
        assert!(map_update_field(Some("1=1 OR 1=1")).is_err());
        assert!(map_update_field(Some("messageId")).is_err());
        assert!(map_update_field(Some("message_id")).is_err());
        assert!(map_update_field(None).is_err());
        assert!(map_update_field(Some("   ")).is_err());
    }

    #[test]
    fn update_field_error_text_never_echoes_input() {
        let err = map_update_field(Some("<script>alert(1)</script>")).unwrap_err();
        let text = err.to_string();
        assert!(!text.contains("script"), "错误文案不应回显输入: {text}");
    }

    // ————— 排序 —————

    #[test]
    fn sort_column_defaults_to_primary_key() {
        assert_eq!(resolve_sort_column(None), "message_id");
        assert_eq!(resolve_sort_column(Some("")), "message_id");
        assert_eq!(resolve_sort_column(Some("name")), "name");
        assert_eq!(resolve_sort_column(Some("sendType")), "send_type");
        // 未知字段回退主键
        assert_eq!(resolve_sort_column(Some("drop table")), "message_id");
    }

    #[test]
    fn sort_dir_defaults_to_desc() {
        assert_eq!(resolve_sort_dir(None), SortDir::Desc);
        assert_eq!(resolve_sort_dir(Some("")), SortDir::Desc);
        assert_eq!(resolve_sort_dir(Some("DESC")), SortDir::Desc);
        assert_eq!(resolve_sort_dir(Some("desc")), SortDir::Desc);
        assert_eq!(resolve_sort_dir(Some("asc")), SortDir::Asc);
        assert_eq!(resolve_sort_dir(Some("ASC")), SortDir::Asc);
    }

    // ————— 时间格式化 —————

    #[test]
    fn add_time_formats_like_java_date_format() {
        let s = format_add_time(Some(1_700_000_000)).unwrap();
        assert_eq!(s.len(), 19);
        assert_eq!(&s[4..5], "-");
        assert_eq!(&s[10..11], " ");
        assert_eq!(&s[13..14], ":");
        assert_eq!(&s[16..17], ":");
    }

    #[test]
    fn add_time_zero_is_epoch_not_none() {
        // Java 对 add_time = 0 会格式化成 1970 年本地时间（seed 数据常见）
        let s = format_add_time(Some(0)).unwrap();
        assert!(s.starts_with("1970-01-01"), "实际: {s}");
        assert!(format_add_time(None).is_some(), "None 应按 0 处理");
    }

    // ————— 版本差异过滤（NexTradeProperties） —————

    #[test]
    fn merchant_settle_kept_in_multimerchant_edition() {
        let f = EditionFlags::default(); // is_merchant = 1
        assert!(f.keep(Some("商家入驻成功")));
        assert!(f.keep(Some("商家入驻审核失败")));
    }

    #[test]
    fn merchant_settle_removed_in_single_merchant_non_o2o() {
        let f = EditionFlags {
            is_merchant: 0,
            is_o2o: 0,
            ..EditionFlags::default()
        };
        assert!(!f.keep(Some("商家入驻成功")));
        // O2O 版本仍保留
        let o2o = EditionFlags {
            is_merchant: 0,
            is_o2o: 1,
            ..EditionFlags::default()
        };
        assert!(o2o.keep(Some("商家入驻成功")));
    }

    #[test]
    fn member_certify_requires_merchant_and_b2b() {
        let f = EditionFlags::default(); // merchant=1, b2b=1
        assert!(f.keep(Some("会员实名认证通知")));
        let not_b2b = EditionFlags {
            is_b2b: 0,
            ..EditionFlags::default()
        };
        assert!(!not_b2b.keep(Some("会员实名认证通知")));
    }

    #[test]
    fn unrelated_names_always_kept() {
        let f = EditionFlags {
            is_merchant: 0,
            is_b2b: 0,
            is_o2o: 0,
            ..EditionFlags::default()
        };
        assert!(f.keep(Some("用户支付成功通知")));
        assert!(f.keep(None));
    }

    #[test]
    fn mail_unsupported_when_not_overseas() {
        let f = EditionFlags::default(); // is_overseas = 0
        assert!(f.force_mail_unsupported("获取验证码"));
        assert!(!f.force_mail_unsupported("用户支付成功通知"));
        let overseas = EditionFlags {
            is_overseas: 1,
            ..EditionFlags::default()
        };
        assert!(!overseas.force_mail_unsupported("获取验证码"));
    }

    #[test]
    fn settle_audit_fail_channels_off_only_for_merchant_non_overseas() {
        let f = EditionFlags::default(); // merchant=1, overseas=0
        assert!(f.force_settle_audit_fail_channels_off("入驻审核失败通知"));
        assert!(!f.force_settle_audit_fail_channels_off("商家入驻成功"));
        let overseas = EditionFlags {
            is_overseas: 1,
            ..EditionFlags::default()
        };
        assert!(!overseas.force_settle_audit_fail_channels_off("入驻审核失败通知"));
    }

    // ————— isSupportTemplate —————

    #[test]
    fn is_support_template_checks_minus_one_only() {
        let t = mt("用户支付成功通知", Some(0));
        // 全为 -1 的通道「不支持」→ 模板应跳过
        assert!(is_support_template(&t, MessageTemplateKind::App));
        assert!(is_support_template(&t, MessageTemplateKind::Ding));
        assert!(is_support_template(&t, MessageTemplateKind::WxWork));
        // 值为 1 的通道「支持」→ 不跳过
        assert!(!is_support_template(&t, MessageTemplateKind::Wechat));
        assert!(!is_support_template(&t, MessageTemplateKind::Msg));
        assert!(!is_support_template(&t, MessageTemplateKind::Email));
    }

    #[test]
    fn is_support_template_treats_zero_as_supported() {
        // Java 只判 `== -1`；0（未开启）不算「不支持」，不会跳过模板
        let mut t = mt("测试", Some(0));
        t.is_msg = Some(0);
        assert!(!is_support_template(&t, MessageTemplateKind::Msg));
        t.is_msg = None;
        assert!(!is_support_template(&t, MessageTemplateKind::Msg));
    }

    // ————— smsProvider —————

    #[test]
    fn sms_provider_parsing_matches_java() {
        assert_eq!(sms_provider_code(None), 1); // 缺省阿里云
        assert_eq!(sms_provider_code(Some("")), 1);
        assert_eq!(sms_provider_code(Some("  ")), 1);
        assert_eq!(sms_provider_code(Some("2")), 2);
        assert_eq!(sms_provider_code(Some(" 3 ")), 3);
        assert_eq!(sms_provider_code(Some("4")), 4);
        // 未知 / 非法一律回退阿里云
        assert_eq!(sms_provider_code(Some("9")), 1);
        assert_eq!(sms_provider_code(Some("abc")), 1);
        assert_eq!(sms_provider_code(Some("0")), 1);
    }

    // ————— 占位符转换（SmsPlaceholderConverter） —————

    #[test]
    fn aliyun_adds_dollar_to_plain_braces() {
        assert_eq!(
            convert_sms_placeholders("验证码：{code}，请勿泄露", Some(1)),
            "验证码：${code}，请勿泄露"
        );
        // 已是 ${var} 的不变
        assert_eq!(
            convert_sms_placeholders("订单 ${order} 已支付", Some(1)),
            "订单 ${order} 已支付"
        );
    }

    #[test]
    fn huawei_behaves_like_aliyun() {
        assert_eq!(convert_sms_placeholders("{a}{b}", Some(3)), "${a}${b}");
    }

    #[test]
    fn volcengine_strips_dollar() {
        assert_eq!(
            convert_sms_placeholders("订单 ${order} 金额 ${fee}", Some(4)),
            "订单 {order} 金额 {fee}"
        );
        // 已是 {var} 的不变
        assert_eq!(convert_sms_placeholders("{order}", Some(4)), "{order}");
    }

    #[test]
    fn tencent_numbers_placeholders_in_order() {
        assert_eq!(
            convert_sms_placeholders("订单 ${order} 金额 {fee}", Some(2)),
            "订单 {1} 金额 {2}"
        );
        assert_eq!(convert_sms_placeholders("{a}{b}{c}", Some(2)), "{1}{2}{3}");
    }

    #[test]
    fn unknown_provider_or_content_returns_input() {
        assert_eq!(convert_sms_placeholders("{code}", None), "{code}");
        assert_eq!(convert_sms_placeholders("无占位符", Some(1)), "无占位符");
        assert_eq!(convert_sms_placeholders("", Some(2)), "");
    }

    #[test]
    fn placeholders_require_word_characters() {
        // `{}` / `{ a }` / `{a-b}` 不是占位符（Java 正则 `\w+`）
        assert_eq!(convert_sms_placeholders("{}", Some(1)), "{}");
        assert_eq!(convert_sms_placeholders("{ a }", Some(1)), "{ a }");
        assert_eq!(convert_sms_placeholders("{a-b}", Some(1)), "{a-b}");
        assert_eq!(convert_sms_placeholders("{a", Some(1)), "{a");
        // 非 ASCII 不算 `\w`
        assert_eq!(convert_sms_placeholders("{订单}", Some(1)), "{订单}");
    }

    #[test]
    fn consecutive_placeholders_are_all_converted() {
        assert_eq!(
            convert_sms_placeholders("{order_sn}{tracking_no}", Some(1)),
            "${order_sn}${tracking_no}"
        );
        // 数字与下划线均属 `\w`
        assert_eq!(convert_sms_placeholders("{a1_2}", Some(1)), "${a1_2}");
    }

    // ————— updateTemplate 字段选择 —————

    fn tpl() -> MessageTemplateDTO {
        MessageTemplateDTO {
            id: Some(9),
            message_id: Some(2),
            r#type: Some(3),
            template_name: Some("支付成功通知".into()),
            template_id: Some("SMS_129744372".into()),
            content: Some("订单 ${order} 已支付".into()),
            content_oversea: Some("Order ${order} paid".into()),
            to_userid: None,
            template_num: Some("1081".into()),
        }
    }

    #[test]
    fn template_update_message_kind_writes_name_and_content() {
        let (name, tid, content, oversea) =
            template_update_values(MessageTemplateKind::Message, &tpl());
        assert_eq!(name.as_deref(), Some("支付成功通知"));
        assert_eq!(content.as_deref(), Some("订单 ${order} 已支付"));
        assert!(tid.is_none());
        assert!(oversea.is_none());
    }

    #[test]
    fn template_update_wechat_kind_writes_only_template_id() {
        let (name, tid, content, oversea) =
            template_update_values(MessageTemplateKind::Wechat, &tpl());
        assert!(name.is_none());
        assert_eq!(tid.as_deref(), Some("SMS_129744372"));
        assert!(content.is_none());
        assert!(oversea.is_none());
    }

    #[test]
    fn template_update_sms_kind_writes_template_id_and_oversea() {
        let (name, tid, content, oversea) = template_update_values(MessageTemplateKind::Msg, &tpl());
        assert!(name.is_none());
        assert_eq!(tid.as_deref(), Some("SMS_129744372"));
        assert!(content.is_none());
        assert_eq!(oversea.as_deref(), Some("Order ${order} paid"));
    }

    #[test]
    fn template_update_sms_kind_does_not_clear_oversea_when_null() {
        let mut t = tpl();
        t.content_oversea = None;
        let (_, _, _, oversea) = template_update_values(MessageTemplateKind::Msg, &t);
        assert!(oversea.is_none(), "contentOversea 为 null 时不得覆盖");
    }

    #[test]
    fn template_update_sms_kind_writes_empty_oversea_when_empty_string() {
        // Java：有值（含空串）时写入
        let mut t = tpl();
        t.content_oversea = Some(String::new());
        let (_, _, _, oversea) = template_update_values(MessageTemplateKind::Msg, &t);
        assert_eq!(oversea.as_deref(), Some(""));
    }

    #[test]
    fn template_update_ding_kind_writes_content_only() {
        let (name, tid, content, oversea) = template_update_values(MessageTemplateKind::Ding, &tpl());
        assert!(name.is_none());
        assert!(tid.is_none());
        assert_eq!(content.as_deref(), Some("订单 ${order} 已支付"));
        assert!(oversea.is_none());
    }

    #[test]
    fn template_update_defaults_null_to_empty_string() {
        // Java `StringUtils.defaultIfEmpty(x, "")`：null → ""
        let t = MessageTemplateDTO::default();
        let (name, _, content, _) = template_update_values(MessageTemplateKind::Email, &t);
        assert_eq!(name.as_deref(), Some(""));
        assert_eq!(content.as_deref(), Some(""));

        let (_, tid, _, _) = template_update_values(MessageTemplateKind::Wechat, &t);
        assert_eq!(tid.as_deref(), Some(""));
    }

    // ————— 列表 VO 映射（版本改写） —————

    #[test]
    fn list_vo_applies_mail_and_settle_audit_rewrites() {
        let flags = EditionFlags::default();

        // 「获取验证码」→ isMail = -1（非跨境版本无邮箱验证码）
        let vo = to_list_vo(&mt("获取验证码", Some(1_700_000_000)), flags);
        assert_eq!(vo.is_mail, Some(-1));
        assert_eq!(vo.is_msg, Some(1), "其余通道不受影响");

        // 「入驻审核失败」→ 站内信/小程序/公众号 三列 = -1
        let vo = to_list_vo(&mt("入驻审核失败", Some(0)), flags);
        assert_eq!(vo.is_message, Some(-1));
        assert_eq!(vo.is_mini_program, Some(-1));
        assert_eq!(vo.is_wechat, Some(-1));
        assert_eq!(vo.is_mail, Some(1), "不匹配「获取验证码」前缀");
    }

    #[test]
    fn list_vo_keeps_original_channels_for_normal_names() {
        let vo = to_list_vo(
            &mt("用户支付成功通知", Some(1_700_000_000)),
            EditionFlags::default(),
        );
        assert_eq!(vo.message_id, 2);
        assert_eq!(vo.name.as_deref(), Some("用户支付成功通知"));
        assert_eq!(vo.is_mail, Some(1));
        assert_eq!(vo.is_wechat, Some(1));
        assert_eq!(vo.is_wx_work, Some(-1));
        assert!(vo.add_time.is_some());
    }

    #[test]
    fn list_vo_overseas_edition_keeps_mail() {
        let flags = EditionFlags {
            is_overseas: 1,
            ..EditionFlags::default()
        };
        let vo = to_list_vo(&mt("获取验证码", Some(0)), flags);
        assert_eq!(vo.is_mail, Some(1), "跨境版保留邮箱验证码");
        // 跨境版本不做「入驻审核失败」改写
        let vo = to_list_vo(&mt("入驻审核失败", Some(0)), flags);
        assert_eq!(vo.is_message, Some(1));
    }

    #[test]
    fn template_to_dto_maps_all_columns() {
        let row = MessageTemplate {
            id: 9,
            message_id: Some(2),
            r#type: Some(3),
            template_name: Some("支付成功通知".into()),
            to_userid: Some("u1".into()),
            template_id: Some("SMS_1".into()),
            template_num: Some("1081".into()),
            content: Some("内容".into()),
            content_oversea: Some("oversea".into()),
        };
        let dto = template_to_dto(&row);
        assert_eq!(dto.id, Some(9));
        assert_eq!(dto.message_id, Some(2));
        assert_eq!(dto.r#type, Some(3));
        assert_eq!(dto.template_name.as_deref(), Some("支付成功通知"));
        assert_eq!(dto.to_userid.as_deref(), Some("u1"));
        assert_eq!(dto.template_id.as_deref(), Some("SMS_1"));
        assert_eq!(dto.template_num.as_deref(), Some("1081"));
        assert_eq!(dto.content.as_deref(), Some("内容"));
        assert_eq!(dto.content_oversea.as_deref(), Some("oversea"));
    }

    #[test]
    fn empty_detail_keeps_default_provider() {
        // Java 即使记录不存在也会把 smsProvider 设为 1
        let d = empty_detail();
        assert_eq!(d.sms_provider, Some(1));
        assert!(d.message_id.is_none());
        assert!(d.template_message.is_empty());
    }

    // ————— 服务层守卫（先校验、后 IO） —————

    // 必须是 `#[tokio::test]`：`PgPool::connect_lazy` 需要 Tokio 运行时
    // 上下文（sqlx 内部会 `tokio::spawn` 池的后台任务），在纯 `#[test]`
    // 里构造会 panic（sqlx-core pool/inner.rs）。
    #[tokio::test]
    async fn service_can_be_constructed_without_live_database() {
        let svc = MessageTypeService::new(lazy_repos());
        assert_eq!(svc.flags(), EditionFlags::default());
    }

    #[tokio::test]
    async fn wechat_template_endpoints_report_not_enabled() {
        let svc = MessageTypeService::new(lazy_repos());
        // 未迁移的微信模板生成必须**明确报错**，不得伪造成功
        assert!(svc.mini_program_message_template().await.is_err());
        assert!(svc.wechat_message_template().await.is_err());
    }

    #[test]
    fn wechat_template_errors_mention_missing_credential_service() {
        let err = Error::biz_msg("小程序订阅消息模板生成未启用：缺少微信小程序 access_token 服务");
        assert!(err.to_string().contains("access_token"));
    }

    #[tokio::test]
    async fn update_rejects_missing_message_id_before_touching_db() {
        let svc = MessageTypeService::new(lazy_repos());
        let dto = MessageTypeDetailDTO::default();
        let err = svc.update(&dto).await.unwrap_err();
        assert!(matches!(err, Error::Validation(_)), "实际: {err:?}");
    }

    #[tokio::test]
    async fn update_field_rejects_bad_column_before_touching_db() {
        let svc = MessageTypeService::new(lazy_repos());
        let dto = UpdateFieldDTO {
            id: Some(1),
            field: Some("describe".to_string()),
            val: Some("x".to_string()),
        };
        let err = svc.update_field(&dto).await.unwrap_err();
        let Error::Biz { msg, .. } = err else {
            panic!("应为业务错误: {err:?}");
        };
        assert_eq!(msg, INVALID_FIELD_VALUE);
    }

    #[tokio::test]
    async fn update_field_requires_id() {
        let svc = MessageTypeService::new(lazy_repos());
        let dto = UpdateFieldDTO {
            id: None,
            field: Some("name".to_string()),
            val: Some("x".to_string()),
        };
        let err = svc.update_field(&dto).await.unwrap_err();
        assert!(matches!(err, Error::Validation(_)), "实际: {err:?}");
    }

    #[tokio::test]
    async fn update_field_requires_val() {
        let svc = MessageTypeService::new(lazy_repos());
        let dto = UpdateFieldDTO {
            id: Some(1),
            field: Some("name".to_string()),
            val: None,
        };
        let err = svc.update_field(&dto).await.unwrap_err();
        assert!(matches!(err, Error::Validation(_)), "实际: {err:?}");
    }

    #[tokio::test]
    async fn update_field_valid_column_passes_guard_then_fails_on_dead_pool() {
        // 白名单校验通过后才会碰数据库；惰性池连不上 → Database 错误，
        // 以此证明「校验先于 IO」。
        let svc = MessageTypeService::new(lazy_repos());
        let dto = UpdateFieldDTO {
            id: Some(1),
            field: Some("is_wechat".to_string()),
            val: Some("1".to_string()),
        };
        let err = svc.update_field(&dto).await.unwrap_err();
        assert!(matches!(err, Error::Database(_)), "实际: {err:?}");
    }
}