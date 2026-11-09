//! 公众号关键字回复规则（content::wechat_keyword_rule）DTO / VO / 纯逻辑。
//!
//! 对应原 Java：
//! - 控制器 `com.nextrade.adminapi.controller.setting.wechat.WechatKeywordRuleController`
//!   （`@RequestMapping("/adminapi/wechat/keywordRule")`，5 个端点）
//! - 服务 `WechatKeywordRuleServiceImpl`（主表 upsert + 子表先删后插）
//! - 入参 `KeywordRuleSaveDTO` / `KeywordRuleListDTO` / `KeywordRuleDelDTO` /
//!   `SetEnabledDTO`；出参 `KeywordRuleVO` / `KeywordRuleDetailVO` / `KeywordRuleSaveVO`
//! - 表 `wechat_keyword_rule` / `wechat_keyword_rule_keyword` /
//!   `wechat_keyword_rule_reply`（实体见 [`crate::entities::WechatKeywordRule`]）
//!
//! # 端点（5，全部 admin）
//! `GET /adminapi/wechat/keywordRule/{list,detail}`、
//! `POST /adminapi/wechat/keywordRule/{save,del,setEnabled}`。
//!
//! # 与 Java 的差异（诚实记录）
//! 1. **规则匹配缓存未迁移**：Java 保存/删除/启停后在事务提交后失效
//!    `WechatKeywordRuleCache` 并通过 Redis Pub/Sub 广播（`adminapi` 与 `api`
//!    是两个进程）。Rust 侧本批只迁移后台 5 个管理端点，**入站消息匹配链路
//!    （`findFirstHitRule`）不在本契约范围**，故不做缓存失效/广播；DB 为唯一
//!    事实源。若后续迁移匹配侧，需补缓存与广播。
//! 2. `createdAt` / `updatedAt` 为 `yyyy-MM-dd HH:mm:ss` 字符串（DB DATETIME）；
//!    Java LocalDateTime 默认 Jackson 输出为 ISO-8601（带 `T`），前端如有差异
//!    属序列化格式差异（记录在案）。
//! 3. 分页以 [`crate::types::PageResult`] 表达（字段命名与 Java Page 不同）。
//! 4. 业务错误码沿用项目惯例 500（Java `GlobalException` 1001），消息逐字。

use serde::{Deserialize, Serialize};

use crate::models::content::wechat_auto_reply::ReplyPayloadVO;
use crate::types::PageQuery;

/// Java 业务上限：全局规则数（`MAX_RULES`）。
pub const MAX_RULES: usize = 200;
/// 单规则关键字数量下限 / 上限。
pub const MIN_KEYWORDS: usize = 1;
/// 单规则关键字数量上限。
pub const MAX_KEYWORDS: usize = 10;
/// 单规则回复数量下限 / 上限。
pub const MIN_REPLIES: usize = 1;
/// 单规则回复数量上限。
pub const MAX_REPLIES: usize = 5;
/// 规则名最大长度。
pub const MAX_RULE_NAME_LEN: usize = 60;
/// 关键字最大长度。
pub const MAX_KEYWORD_LEN: usize = 30;
/// 关键字回复文本最大长度（关键字上下文 300，区别于自动回复 600）。
pub const MAX_CONTENT_LEN: usize = 300;
/// 图文标题最大长度。
pub const MAX_NEWS_TITLE_LEN: usize = 64;
/// 图文摘要最大长度。
pub const MAX_NEWS_DESC_LEN: usize = 120;
/// URL 最大长度。
pub const MAX_URL_LEN: usize = 500;

/// 回复模式：1=随机一条。
pub const REPLY_MODE_RANDOM: i32 = 1;
/// 回复模式：2=全部回复。
pub const REPLY_MODE_ALL: i32 = 2;
/// 启用状态：停用。
pub const ENABLED_OFF: i32 = 0;
/// 启用状态：启用。
pub const ENABLED_ON: i32 = 1;
/// 匹配方式：全匹配。
pub const MATCH_TYPE_FULL: i32 = 1;
/// 匹配方式：半匹配。
pub const MATCH_TYPE_PARTIAL: i32 = 2;
/// 消息类型：文本。
pub const MSG_TYPE_TEXT: i32 = 1;
/// 消息类型：图文。
pub const MSG_TYPE_NEWS: i32 = 2;

/// Java `KeywordRuleSaveDTO.ruleName` 的 `@NotBlank`。
pub const MSG_RULE_NAME_REQUIRED: &str = "规则名称不能为空";
/// Java `KeywordRuleSaveDTO.replyMode` 的 `@NotNull`。
pub const MSG_REPLY_MODE_REQUIRED: &str = "回复模式不能为空";
/// Java `KeywordRuleSaveDTO.enabled` 的 `@NotNull`。
pub const MSG_ENABLED_REQUIRED: &str = "启用状态不能为空";
/// Java 删除参数校验。
pub const MSG_IDS_REQUIRED: &str = "ID列表不能为空";
/// Java `detail` 的 ID 校验。
pub const MSG_RULE_ID_REQUIRED: &str = "规则ID不能为空";
/// Java `detail` / `save` 的缺行文案。
pub const MSG_RULE_NOT_FOUND: &str = "规则不存在";
/// Java `save` 入参为空。
pub const MSG_SAVE_DTO_REQUIRED: &str = "保存参数不能为空";
/// Java 回复模式非法。
pub const MSG_REPLY_MODE_INVALID: &str = "回复模式仅支持 1=随机一条 / 2=全部回复";
/// Java 启用状态非法。
pub const MSG_ENABLED_INVALID: &str = "启用状态仅支持 0=停用 / 1=启用";

/// Java `BasePage` 默认页大小。
pub const DEFAULT_PAGE_SIZE: u64 = 15;
/// Java `BasePage#getSize` 页大小上限。
pub const MAX_PAGE_SIZE: u64 = 100;

/// 关键字项入参（Java `KeywordRuleKeywordDTO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KeywordRuleKeywordDTO {
    /// 关键字（`@NotBlank(message = 关键字不能为空)`）。
    pub keyword: Option<String>,
    /// 匹配方式（`@NotNull(message = 匹配方式不能为空)`）。
    pub match_type: Option<i32>,
}

/// 保存入参（Java `KeywordRuleSaveDTO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KeywordRuleSaveDTO {
    /// 规则 ID（编辑时传入，新增留空）。
    pub id: Option<i64>,
    /// 规则名称（≤60，全局唯一，忽略大小写）。
    pub rule_name: Option<String>,
    /// 回复模式：1=随机一条 / 2=全部回复。
    pub reply_mode: Option<i32>,
    /// 启用状态：0/1。
    pub enabled: Option<i32>,
    /// 关键字列表（1..10）。
    pub keywords: Vec<KeywordRuleKeywordDTO>,
    /// 回复内容列表（1..5，复用自动回复载荷结构）。
    pub replies: Vec<crate::models::content::wechat_auto_reply::ReplyPayloadDTO>,
}

/// 列表查询参数（Java `KeywordRuleListDTO extends BasePage`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KeywordRuleListQuery {
    /// 页码。
    pub page: Option<u64>,
    /// 页大小。
    pub size: Option<u64>,
    /// 排序字段（Java 本接口未使用，固定 `created_at DESC, id DESC`）。
    pub sort_field: Option<String>,
    /// 排序方向（同上）。
    pub sort_order: Option<String>,
    /// 关键字（Java `BasePage.keyword`；未使用）。
    pub keyword: Option<String>,
    /// 规则名（LIKE 过滤，trim 后使用）。
    pub rule_name: Option<String>,
    /// 启用状态（精确）。
    pub enabled: Option<i32>,
}

impl KeywordRuleListQuery {
    /// 归一化分页。
    #[must_use]
    pub fn page_query(&self) -> PageQuery {
        PageQuery {
            page: self.page.unwrap_or(1).max(1),
            page_size: self
                .size
                .unwrap_or(DEFAULT_PAGE_SIZE)
                .clamp(1, MAX_PAGE_SIZE),
        }
    }

    /// trim 后的规则名过滤值（Java `StrUtil.isBlank ? null : trim`）。
    #[must_use]
    pub fn rule_name_filter(&self) -> Option<String> {
        self.rule_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
    }
}

/// 删除入参（Java `KeywordRuleDelDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KeywordRuleDelDTO {
    /// 规则 ID 列表（`@NotEmpty(message = ID列表不能为空)`）。
    pub ids: Vec<i64>,
}

/// 批量启停入参（Java `SetEnabledDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SetEnabledDTO {
    /// 规则 ID 列表。
    pub ids: Vec<i64>,
    /// 启用状态（0/1）。
    pub enabled: Option<i32>,
}

/// 列表行 VO（Java `KeywordRuleVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeywordRuleVO {
    /// 主键。
    pub id: i64,
    /// 规则名称。
    pub rule_name: String,
    /// 关键字数量。
    pub keyword_count: i64,
    /// 回复数量。
    pub reply_count: i64,
    /// 关键字文本列表。
    pub keywords: Vec<String>,
    /// 文本回复数。
    pub text_reply_count: i64,
    /// 图文回复数。
    pub news_reply_count: i64,
    /// 回复模式。
    pub reply_mode: i32,
    /// 启用状态。
    pub enabled: i32,
    /// 创建时间（`yyyy-MM-dd HH:mm:ss`）。
    pub created_at: String,
    /// 更新时间（`yyyy-MM-dd HH:mm:ss`）。
    pub updated_at: String,
}

/// 详情中的关键字项（Java `KeywordRuleDetailVO.KeywordRuleKeywordItem`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeywordRuleKeywordItem {
    /// 关键字。
    pub keyword: String,
    /// 匹配方式。
    pub match_type: i32,
}

/// 详情 VO（Java `KeywordRuleDetailVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeywordRuleDetailVO {
    /// 主键。
    pub id: i64,
    /// 规则名称。
    pub rule_name: String,
    /// 回复模式。
    pub reply_mode: i32,
    /// 启用状态。
    pub enabled: i32,
    /// 关键字列表。
    pub keywords: Vec<KeywordRuleKeywordItem>,
    /// 回复载荷列表。
    pub replies: Vec<ReplyPayloadVO>,
    /// 创建时间。
    pub created_at: String,
    /// 更新时间。
    pub updated_at: String,
}

/// 保存返回 VO（Java `KeywordRuleSaveVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeywordRuleSaveVO {
    /// 规则 ID。
    pub id: i64,
}

/// 当前时间 → `yyyy-MM-dd HH:mm:ss`（东八区，DB DATETIME 字符串）。
#[must_use]
pub fn now_datetime_string() -> String {
    use chrono::TimeZone;
    chrono::FixedOffset::east_opt(8 * 3600)
        .and_then(|tz| tz.timestamp_opt(chrono::Utc::now().timestamp(), 0).single())
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

/// 保存深度校验（Java `validateSave`，消息逐字，含插值）。
///
/// # Errors
/// 失败 → 对应逐字中文文案。
pub fn validate_save(dto: &KeywordRuleSaveDTO) -> Result<(), String> {
    // 规则名
    let rule_name = dto.rule_name.as_deref().map(str::trim).unwrap_or("");
    if rule_name.is_empty() {
        return Err(MSG_RULE_NAME_REQUIRED.to_string());
    }
    if rule_name.chars().count() > MAX_RULE_NAME_LEN {
        return Err(format!("规则名称长度需在 1..{MAX_RULE_NAME_LEN} 之间"));
    }
    // 回复模式
    match dto.reply_mode {
        Some(REPLY_MODE_RANDOM | REPLY_MODE_ALL) => {}
        _ => return Err(MSG_REPLY_MODE_INVALID.to_string()),
    }
    // 启用状态
    match dto.enabled {
        Some(ENABLED_OFF | ENABLED_ON) => {}
        _ => return Err(MSG_ENABLED_INVALID.to_string()),
    }
    // 关键字
    if dto.keywords.len() < MIN_KEYWORDS || dto.keywords.len() > MAX_KEYWORDS {
        return Err(format!(
            "关键字数量需在 {MIN_KEYWORDS}..{MAX_KEYWORDS} 之间"
        ));
    }
    let mut dedup: Vec<(i32, String)> = Vec::new();
    for k in &dto.keywords {
        let keyword = k.keyword.as_deref().map(str::trim).unwrap_or("");
        if keyword.is_empty() {
            return Err("关键字不能为空".to_string());
        }
        if keyword.chars().count() > MAX_KEYWORD_LEN {
            return Err(format!("关键字「{keyword}」长度不能超过 {MAX_KEYWORD_LEN}"));
        }
        let Some(mt) = k.match_type else {
            return Err(format!(
                "关键字「{keyword}」匹配方式仅支持 1=全匹配 / 2=半匹配"
            ));
        };
        if mt != MATCH_TYPE_FULL && mt != MATCH_TYPE_PARTIAL {
            return Err(format!(
                "关键字「{keyword}」匹配方式仅支持 1=全匹配 / 2=半匹配"
            ));
        }
        let key = (mt, keyword.to_lowercase());
        if dedup.contains(&key) {
            return Err(format!("同一匹配方式下关键字不可重复：「{keyword}」"));
        }
        dedup.push(key);
    }
    // 回复
    if dto.replies.len() < MIN_REPLIES || dto.replies.len() > MAX_REPLIES {
        return Err(format!("回复数量需在 {MIN_REPLIES}..{MAX_REPLIES} 之间"));
    }
    for (idx, reply) in dto.replies.iter().enumerate() {
        validate_reply_payload(reply, idx)?;
    }
    Ok(())
}

/// 单条回复校验（Java `validateReplyPayload`；关键字上下文文本 ≤ 300）。
fn validate_reply_payload(
    dto: &crate::models::content::wechat_auto_reply::ReplyPayloadDTO,
    idx: usize,
) -> Result<(), String> {
    let pos = format!("第 {} 条回复", idx + 1);
    let Some(msg_type) = dto.msg_type else {
        return Err(format!("{pos}：消息类型不能为空"));
    };
    if msg_type != MSG_TYPE_TEXT && msg_type != MSG_TYPE_NEWS {
        return Err(format!("{pos}：消息类型仅支持 1=文本 / 2=图文"));
    }
    if msg_type == MSG_TYPE_TEXT {
        let content = dto.content.as_deref().map(str::trim).unwrap_or("");
        if content.is_empty() {
            return Err(format!("{pos}：文本内容不能为空"));
        }
        if content.chars().count() > MAX_CONTENT_LEN {
            return Err(format!("{pos}：文本内容长度不能超过 {MAX_CONTENT_LEN}"));
        }
        return Ok(());
    }
    let title = dto.news_title.as_deref().map(str::trim).unwrap_or("");
    if title.is_empty() || title.chars().count() > MAX_NEWS_TITLE_LEN {
        return Err(format!(
            "{pos}：图文标题不能为空且长度不能超过 {MAX_NEWS_TITLE_LEN}"
        ));
    }
    let desc = dto.news_description.as_deref().map(str::trim).unwrap_or("");
    if desc.is_empty() || desc.chars().count() > MAX_NEWS_DESC_LEN {
        return Err(format!(
            "{pos}：图文摘要不能为空且长度不能超过 {MAX_NEWS_DESC_LEN}"
        ));
    }
    if !crate::models::content::wechat_auto_reply::is_valid_http_url(dto.news_cover_url.as_deref())
    {
        return Err(format!(
            "{pos}：图文封面图 URL 必须为合法的 http/https 链接，且长度不能超过 {MAX_URL_LEN}"
        ));
    }
    if !crate::models::content::wechat_auto_reply::is_valid_http_url(dto.news_url.as_deref()) {
        return Err(format!(
            "{pos}：图文跳转链接必须为合法的 http/https 链接，且长度不能超过 {MAX_URL_LEN}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::content::wechat_auto_reply::ReplyPayloadDTO;

    fn keyword(k: &str, mt: i32) -> KeywordRuleKeywordDTO {
        KeywordRuleKeywordDTO {
            keyword: Some(k.into()),
            match_type: Some(mt),
        }
    }

    fn text_reply(content: &str) -> ReplyPayloadDTO {
        ReplyPayloadDTO {
            msg_type: Some(1),
            content: Some(content.into()),
            ..Default::default()
        }
    }

    fn base_dto() -> KeywordRuleSaveDTO {
        KeywordRuleSaveDTO {
            id: None,
            rule_name: Some("欢迎".into()),
            reply_mode: Some(REPLY_MODE_ALL),
            enabled: Some(ENABLED_ON),
            keywords: vec![keyword("hello", MATCH_TYPE_FULL)],
            replies: vec![text_reply("你好")],
        }
    }

    #[test]
    fn valid_payload_passes() {
        assert!(validate_save(&base_dto()).is_ok());
    }

    #[test]
    fn rule_name_validation() {
        let mut dto = base_dto();
        dto.rule_name = None;
        assert_eq!(validate_save(&dto).unwrap_err(), MSG_RULE_NAME_REQUIRED);
        dto.rule_name = Some("x".repeat(MAX_RULE_NAME_LEN + 1));
        assert_eq!(
            validate_save(&dto).unwrap_err(),
            format!("规则名称长度需在 1..{MAX_RULE_NAME_LEN} 之间")
        );
    }

    #[test]
    fn mode_and_enabled_validation() {
        let mut dto = base_dto();
        dto.reply_mode = Some(3);
        assert_eq!(validate_save(&dto).unwrap_err(), MSG_REPLY_MODE_INVALID);
        dto.reply_mode = Some(REPLY_MODE_RANDOM);
        dto.enabled = Some(2);
        assert_eq!(validate_save(&dto).unwrap_err(), MSG_ENABLED_INVALID);
    }

    #[test]
    fn keyword_validation_messages() {
        let mut dto = base_dto();
        dto.keywords = vec![];
        assert_eq!(
            validate_save(&dto).unwrap_err(),
            "关键字数量需在 1..10 之间"
        );
        dto.keywords = vec![keyword("   ", MATCH_TYPE_FULL)];
        assert_eq!(validate_save(&dto).unwrap_err(), "关键字不能为空");
        dto.keywords = vec![keyword(&"k".repeat(MAX_KEYWORD_LEN + 1), MATCH_TYPE_FULL)];
        assert_eq!(
            validate_save(&dto).unwrap_err(),
            format!(
                "关键字「{}」长度不能超过 {MAX_KEYWORD_LEN}",
                "k".repeat(MAX_KEYWORD_LEN + 1)
            )
        );
        dto.keywords = vec![keyword("hello", 3)];
        assert!(validate_save(&dto).unwrap_err().contains("匹配方式仅支持"));
        // 同 matchType 忽略大小写重复
        dto.keywords = vec![
            keyword("Hello", MATCH_TYPE_FULL),
            keyword("hello", MATCH_TYPE_FULL),
        ];
        assert_eq!(
            validate_save(&dto).unwrap_err(),
            "同一匹配方式下关键字不可重复：「hello」"
        );
        // 不同 matchType 允许同名
        dto.keywords = vec![
            keyword("hello", MATCH_TYPE_FULL),
            keyword("hello", MATCH_TYPE_PARTIAL),
        ];
        assert!(validate_save(&dto).is_ok());
    }

    #[test]
    fn reply_validation_is_position_prefixed_and_300_capped() {
        let mut dto = base_dto();
        dto.replies = vec![text_reply(&"x".repeat(MAX_CONTENT_LEN + 1))];
        assert_eq!(
            validate_save(&dto).unwrap_err(),
            format!("第 1 条回复：文本内容长度不能超过 {MAX_CONTENT_LEN}")
        );
        dto.replies = vec![text_reply(&"x".repeat(MAX_CONTENT_LEN))];
        assert!(validate_save(&dto).is_ok());
        dto.replies = vec![ReplyPayloadDTO::default()];
        assert_eq!(
            validate_save(&dto).unwrap_err(),
            "第 1 条回复：消息类型不能为空"
        );
        dto.replies = vec![];
        assert_eq!(validate_save(&dto).unwrap_err(), "回复数量需在 1..5 之间");
    }

    #[test]
    fn query_helpers_and_vo_camel_case() {
        let q: KeywordRuleListQuery =
            serde_json::from_value(serde_json::json!({"ruleName": "  欢迎  ", "enabled": 1}))
                .unwrap();
        assert_eq!(q.rule_name_filter().as_deref(), Some("欢迎"));
        assert_eq!(q.page_query().page_size, 15);
        let vo = KeywordRuleVO {
            id: 1,
            rule_name: "欢迎".into(),
            keyword_count: 2,
            reply_count: 1,
            keywords: vec!["hello".into()],
            text_reply_count: 1,
            news_reply_count: 0,
            reply_mode: 1,
            enabled: 1,
            created_at: "2026-04-23 10:00:00".into(),
            updated_at: "2026-04-23 10:00:00".into(),
        };
        let json = serde_json::to_value(&vo).unwrap();
        assert_eq!(json["ruleName"], "欢迎");
        assert_eq!(json["keywords"][0], "hello");
        assert_eq!(json["textReplyCount"], 1);
        assert_eq!(json["createdAt"], "2026-04-23 10:00:00");
    }

    #[test]
    fn datetime_string_format() {
        let s = now_datetime_string();
        assert_eq!(s.len(), 19);
        assert!(s.contains('-') && s.contains(':'));
    }
}
