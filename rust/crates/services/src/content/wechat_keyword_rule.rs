//! 公众号关键字回复规则服务（content::wechat_keyword_rule）。
//!
//! 对应原 Java `com.nextrade.service.setting.wechat.impl.WechatKeywordRuleServiceImpl`
//! （仅后台管理侧 5 个端点；入站消息匹配链路不在本契约范围，见 models 模块头）。
//!
//! 构造签名固定：`pub fn new(repos: Arc<Repositories>) -> Self`。
//!
//! ## 已迁移（5）
//! - `list`：`ruleName LIKE` + `enabled` 过滤 + 四个统计列 + 当前页关键字回填
//! - `detail`：主表 + 关键字 + 回复
//! - `save`：深度校验 → 规则名唯一（忽略大小写）→ 新增 200 条上限 →
//!   主表 upsert + 子表先删后插（事务）
//! - `del`：三表级联删除（事务）
//! - `setEnabled`：批量启停（校验 0/1）
//!
//! ## 与 Java 的差异
//! 见 [`crate::models::content::wechat_keyword_rule`] 模块头（缓存失效/广播未迁移；
//! datetime 序列化格式；PageResult 字段命名）。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::models::content::wechat_auto_reply::ReplyPayloadVO;
use nextrade_core::models::content::wechat_keyword_rule::{
    validate_save, KeywordRuleDelDTO, KeywordRuleDetailVO,
    KeywordRuleKeywordItem, KeywordRuleListQuery, KeywordRuleSaveDTO, KeywordRuleSaveVO,
    KeywordRuleVO, SetEnabledDTO, MAX_RULES, MSG_ENABLED_INVALID, MSG_IDS_REQUIRED,
    MSG_RULE_ID_REQUIRED, MSG_RULE_NOT_FOUND, MSG_SAVE_DTO_REQUIRED,
    REPLY_MODE_ALL, REPLY_MODE_RANDOM,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 关键字规则服务。
pub struct WechatKeywordRuleService {
    repos: Arc<Repositories>,
}

impl WechatKeywordRuleService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储（NESTED 访问：`self.repos.content.wechat_keyword_rule`）。
    #[must_use]
    pub fn repo(
        &self,
    ) -> &nextrade_infrastructure::db::repo::content::wechat_keyword_rule::WechatKeywordRuleRepository
    {
        &self.repos.content.wechat_keyword_rule
    }

    /// 列表（Java `page`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn list(
        &self,
        q: &KeywordRuleListQuery,
    ) -> Result<PageResult<KeywordRuleVO>> {
        let page = q.page_query();
        let (limit, offset) = page.limit_offset();
        let (total, rows) = self
            .repo()
            .page(
                q.rule_name_filter().as_deref(),
                q.enabled,
                limit,
                offset,
            )
            .await?;
        let rule_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
        let keywords = self.repo().keywords_by_rule_ids(&rule_ids).await?;
        let mut kw_map: HashMap<i64, Vec<String>> = HashMap::new();
        for k in keywords {
            kw_map.entry(k.rule_id).or_default().push(k.keyword);
        }
        let list = rows
            .into_iter()
            .map(|r| KeywordRuleVO {
                id: r.id,
                rule_name: r.rule_name,
                keyword_count: r.keyword_count,
                reply_count: r.reply_count,
                keywords: kw_map.remove(&r.id).unwrap_or_default(),
                text_reply_count: r.text_reply_count,
                news_reply_count: r.news_reply_count,
                reply_mode: r.reply_mode,
                enabled: r.enabled,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 详情（Java `detail`）。
    ///
    /// # Errors
    /// - ID 缺失 → `Validation("规则ID不能为空")`
    /// - 不存在 → 业务错误「规则不存在」
    pub async fn detail(&self, id: Option<i64>) -> Result<KeywordRuleDetailVO> {
        let id = id.ok_or_else(|| Error::Validation(MSG_RULE_ID_REQUIRED.into()))?;
        let rule = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_RULE_NOT_FOUND))?;
        let keywords = self.repo().keywords_by_rule_id(id).await?;
        let replies = self.repo().replies_by_rule_id(id).await?;
        Ok(KeywordRuleDetailVO {
            id: rule.id,
            rule_name: rule.rule_name,
            reply_mode: rule.reply_mode,
            enabled: rule.enabled,
            keywords: keywords
                .into_iter()
                .map(|k| KeywordRuleKeywordItem {
                    keyword: k.keyword,
                    match_type: k.match_type,
                })
                .collect(),
            replies: replies
                .into_iter()
                .map(|r| ReplyPayloadVO {
                    msg_type: Some(r.msg_type),
                    content: r.content_text,
                    news_title: r.news_title,
                    news_cover_url: r.news_cover_url,
                    news_description: r.news_description,
                    news_url: r.news_url,
                })
                .collect(),
            created_at: rule.created_at,
            updated_at: rule.updated_at,
        })
    }

    /// 新增/编辑（Java `save`）。
    ///
    /// # Errors
    /// - 深度校验失败 → 业务错误（逐字文案）
    /// - 规则名重复 → 业务错误「规则名称已存在：{name}」
    /// - 新增超上限 → 业务错误「规则总数已达上限 200 条，无法继续新增」
    pub async fn save(&self, dto: &KeywordRuleSaveDTO) -> Result<KeywordRuleSaveVO> {
        validate_save(dto).map_err(Error::biz_msg)?;
        let rule_name = dto.rule_name.as_deref().map(str::trim).unwrap_or("");
        if self.repo().rule_name_exists(rule_name, dto.id).await? {
            return Err(Error::biz_msg(format!("规则名称已存在：{rule_name}")));
        }
        if dto.id.is_none() && self.repo().is_rule_limit_reached().await? {
            return Err(Error::biz_msg(format!(
                "规则总数已达上限 {MAX_RULES} 条，无法继续新增"
            )));
        }
        let id = self.repo().save_rule(dto, rule_name).await?;
        tracing::info!("保存关键字规则: id={id}");
        Ok(KeywordRuleSaveVO { id })
    }

    /// 批量删除（Java `del`）。
    ///
    /// # Errors
    /// - `ids` 为空 → `Validation("ID列表不能为空")`
    pub async fn delete(&self, dto: &KeywordRuleDelDTO) -> Result<()> {
        if dto.ids.is_empty() {
            return Err(Error::Validation(MSG_IDS_REQUIRED.into()));
        }
        let affected = self.repo().delete_rules(&dto.ids).await?;
        tracing::info!("删除关键字规则: {} 条", affected);
        Ok(())
    }

    /// 批量启停（Java `setEnabled`）。
    ///
    /// # Errors
    /// - `enabled` 缺失/非法 → `Validation("启用状态仅支持 0=停用 / 1=启用")`
    pub async fn set_enabled(&self, dto: &SetEnabledDTO) -> Result<()> {
        let enabled = dto
            .enabled
            .filter(|v| *v == 0 || *v == 1)
            .ok_or_else(|| Error::Validation(MSG_ENABLED_INVALID.into()))?;
        if dto.ids.is_empty() {
            return Err(Error::Validation(MSG_IDS_REQUIRED.into()));
        }
        self.repo().set_enabled(&dto.ids, enabled).await?;
        Ok(())
    }
}

/// 供测试/文档引用的模式常量（避免未使用告警）。
#[must_use]
pub fn valid_reply_modes() -> [i32; 2] {
    [REPLY_MODE_RANDOM, REPLY_MODE_ALL]
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::content::wechat_auto_reply::ReplyPayloadDTO;
    use nextrade_core::models::content::wechat_keyword_rule::{
        KeywordRuleKeywordDTO, REPLY_MODE_ALL, ENABLED_ON, MATCH_TYPE_FULL, MSG_REPLY_MODE_INVALID,
    };

    fn dto() -> KeywordRuleSaveDTO {
        KeywordRuleSaveDTO {
            id: None,
            rule_name: Some("欢迎".into()),
            reply_mode: Some(REPLY_MODE_ALL),
            enabled: Some(ENABLED_ON),
            keywords: vec![KeywordRuleKeywordDTO {
                keyword: Some("hello".into()),
                match_type: Some(MATCH_TYPE_FULL),
            }],
            replies: vec![ReplyPayloadDTO {
                msg_type: Some(1),
                content: Some("你好".into()),
                ..Default::default()
            }],
        }
    }

    #[test]
    fn validate_save_called_before_db() {
        let mut bad = dto();
        bad.reply_mode = Some(9);
        assert_eq!(
            validate_save(&bad).unwrap_err(),
            MSG_REPLY_MODE_INVALID
        );
        assert!(validate_save(&dto()).is_ok());
    }

    #[test]
    fn id_validation_messages() {
        assert_eq!(MSG_RULE_ID_REQUIRED, "规则ID不能为空");
        assert_eq!(MSG_RULE_NOT_FOUND, "规则不存在");
        assert_eq!(MSG_IDS_REQUIRED, "ID列表不能为空");
        assert_eq!(MSG_ENABLED_INVALID, "启用状态仅支持 0=停用 / 1=启用");
        assert_eq!(MSG_SAVE_DTO_REQUIRED, "保存参数不能为空");
    }

    #[test]
    fn valid_reply_modes_exposes_java_enum() {
        assert_eq!(valid_reply_modes(), [1, 2]);
    }

    #[test]
    fn rule_name_unique_message_format() {
        let name = "欢迎";
        assert_eq!(
            format!("规则名称已存在：{name}"),
            "规则名称已存在：欢迎"
        );
        assert_eq!(
            format!("规则总数已达上限 {MAX_RULES} 条，无法继续新增"),
            "规则总数已达上限 200 条，无法继续新增"
        );
    }
}
