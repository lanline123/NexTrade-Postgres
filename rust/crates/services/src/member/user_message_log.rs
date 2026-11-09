//! 会员消息管理服务（member::user_message_log）。
//!
//! 对应原 Java `UserMessageLogServiceImpl`（`java/nextrade-service/.../user/impl/UserMessageLogServiceImpl.java`）
//! 与继承的 `BaseServiceImpl`。
//!
//! 构造签名固定：`pub fn new(repos: Arc<Repositories>) -> Self`。
//!
//! ## 已迁移（5）
//! - `list`：标题关键字 + 动态排序 + 分页；发送时间格式化
//! - `create`：人群解析（全部 0 / 单个 1 / 等级 2 / 会员名 3）→ 写日志 +
//!   批量写 `user_message`（事务）
//! - `del`：按 `message_log_id` 删日志（不级联，Java `BaseServiceImpl#del`）
//! - `batch`：仅 `type == "del"` 生效
//! - `recall`：日志 `is_recall = 1` + 删除对应站内信（事务）
//!
//! ## 人群解析
//! `sendUserTarget`（统一结构，scope/会员/等级/标签并集）与 `sendUserType=4/5`
//! + `memberAudience`（历史结构）均复用 `member_audience` 仓储解析目标会员。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::UserMessageLog;
use nextrade_core::models::member::member_audience::{
    resolve as resolve_audience, Audience, MemberAudienceDTO,
};
use nextrade_core::models::member::user_message_log::{
    format_send_time, is_delete_batch, split_usernames, BatchDTO, SendUserTargetDTO,
    UserMessageLogCreateDTO, UserMessageLogListQuery, UserMessageLogPageVO,
    UserMessageLogRecallParam, MSG_LOG_ID_REQUIRED, MSG_NO_MEMBERS, MSG_USERNAME_INVALID,
    SEND_USER_TYPE_ALL, SEND_USER_TYPE_PART, SEND_USER_TYPE_RANK, SEND_USER_TYPE_SINGLE,
    SEND_USER_TYPE_TAG,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 会员消息日志服务。
pub struct UserMessageLogService {
    repos: Arc<Repositories>,
}

impl UserMessageLogService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储（NESTED 访问：`self.repos.member.user_message_log`）。
    #[must_use]
    pub fn repo(
        &self,
    ) -> &nextrade_infrastructure::db::repo::member::user_message_log::UserMessageLogRepository
    {
        &self.repos.member.user_message_log
    }

    /// 列表（Java `UserMessageLogServiceImpl#list`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn list(
        &self,
        q: &UserMessageLogListQuery,
    ) -> Result<PageResult<UserMessageLogPageVO>> {
        let page = q.page_query();
        let (total, rows) = self.repo().paginate(q).await?;
        let list = rows.iter().map(to_vo).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 创建会员消息（Java `#create`，`@Transactional`）。
    ///
    /// # Errors
    /// - 标签/组合人群 → 未迁移业务错误
    /// - 人群为空 → 「该会员选择类型下没有会员」
    /// - 按会员名发送有未命中 → 「请检查输入的每个会员名称是否正确」
    /// - 数据库失败 → `Error::Database`
    pub async fn create(&self, dto: &UserMessageLogCreateDTO) -> Result<i32> {
        let user_ids = self.resolve_user_ids(dto).await?;
        if user_ids.is_empty() {
            return Err(Error::biz_msg(MSG_NO_MEMBERS));
        }
        let now = now_seconds() as i32;
        let link = dto
            .message_link
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();
        let log = UserMessageLog {
            message_log_id: 0,
            message_type: dto.message_type.unwrap_or(0),
            send_user_type: dto.send_user_type.unwrap_or(0),
            // Java：`StrUtil.isNotBlank(userIds) ? userIds : ""`
            user_ids: dto
                .user_ids
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("")
                .to_string(),
            user_rank: dto.user_rank.unwrap_or(0),
            message_title: dto.message_title.clone().unwrap_or_default(),
            message_content: dto.message_content.clone().unwrap_or_default(),
            message_link: link,
            send_time: now,
            is_recall: 0,
        };
        let log_id = self
            .repo()
            .create_log_with_messages(&log, &user_ids, now)
            .await?;
        tracing::info!("创建会员消息: log_id={log_id}, 目标会员 {} 人", user_ids.len());
        Ok(log_id)
    }

    /// 解析目标会员（Java `findUserIds`）。
    async fn resolve_user_ids(&self, dto: &UserMessageLogCreateDTO) -> Result<Vec<i32>> {
        match send_mode(dto)? {
            SendMode::All => self.repo().all_user_ids().await,
            SendMode::Single(id) => Ok(vec![id]),
            SendMode::Rank(rank_id) => self.repo().user_ids_by_rank(rank_id).await,
            SendMode::Part => {
                let usernames = split_usernames(dto.user_ids.as_deref());
                let users = self.repo().user_ids_by_usernames(&usernames).await?;
                // Java：`Assert.isTrue(usernames.size() == users.size())`
                if usernames.len() != users.len() {
                    return Err(Error::biz_msg(MSG_USERNAME_INVALID));
                }
                Ok(users)
            }
            SendMode::Audience(audience) => {
                let audience = resolve_audience(&audience)?;
                self.repos
                    .member
                    .member_audience
                    .resolve_user_ids(&audience)
                    .await
            }
            SendMode::Target(target) => {
                if target.scope_or_default() == 0 {
                    return self.repo().all_user_ids().await;
                }
                let mut ids = std::collections::BTreeSet::new();
                if let Some(user_ids) = &target.user_ids {
                    ids.extend(
                        self.repos
                            .member
                            .member_audience
                            .filter_member_ids(user_ids)
                            .await?,
                    );
                }
                if let Some(rank_ids) = &target.rank_ids {
                    ids.extend(
                        self.repos
                            .member
                            .member_audience
                            .resolve_user_ids(&Audience::Ranks(rank_ids.clone()))
                            .await?,
                    );
                }
                if let Some(tag_ids) = &target.tag_ids {
                    ids.extend(
                        self.repos
                            .member
                            .member_audience
                            .resolve_user_ids(&Audience::Tags {
                                tag_ids: tag_ids.clone(),
                                match_all: target.match_all(),
                            })
                            .await?,
                    );
                }
                Ok(ids.into_iter().collect())
            }
            SendMode::Empty => Ok(Vec::new()),
        }
    }

    /// 删除日志（Java `#del` → `BaseServiceImpl#del`，不级联站内信）。
    ///
    /// # Errors
    /// - `id` 缺失 → `Validation("会员消息ID不能为空")`
    pub async fn delete(&self, param: &UserMessageLogRecallParam) -> Result<()> {
        let id = require_log_id(param)?;
        self.repo().delete(id).await?;
        tracing::info!("删除会员消息日志: id={id}");
        Ok(())
    }

    /// 批量操作（Java `BaseServiceImpl#batch`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if !is_delete_batch(dto) {
            return Ok(false);
        }
        let affected = self.repo().delete_many(&dto.ids).await?;
        Ok(affected > 0)
    }

    /// 撤回（Java `#recall`，`@Transactional`）：日志置撤回 + 删除站内信。
    ///
    /// # Errors
    /// - `id` 缺失 → `Validation("会员消息ID不能为空")`
    pub async fn recall(&self, param: &UserMessageLogRecallParam) -> Result<()> {
        let id = require_log_id(param)?;
        self.repo().recall(id).await?;
        tracing::info!("撤回会员消息: id={id}");
        Ok(())
    }
}

/// 目标会员解析方式（纯逻辑，便于单测覆盖 Java `findUserIds` 分支）。
#[derive(Debug, PartialEq, Eq)]
enum SendMode {
    /// 全部会员（0）。
    All,
    /// 用户集合是单个 ID（1）。
    Single(i32),
    /// 按会员等级（2）。
    Rank(i32),
    /// 按会员名 CSV（3）。
    Part,
    /// 历史人群结构（`sendUserType=4` + `memberAudience`）。
    Audience(MemberAudienceDTO),
    /// 统一人群结构（`sendUserTarget`，三组并集）。
    Target(SendUserTargetDTO),
    /// 未知/缺失发送类型 → 空人群（由调用方报「没有会员」）。
    Empty,
}

/// 解析发送类型（Java `findUserIds` 的分支判定）。
fn send_mode(dto: &UserMessageLogCreateDTO) -> Result<SendMode> {
    // Java：`sendUserTarget` 非空时一律以统一人群结构为准
    if let Some(raw) = &dto.send_user_target {
        let target: SendUserTargetDTO = serde_json::from_value(raw.clone())
            .map_err(|_| Error::Validation("会员人群参数有误".into()))?;
        return Ok(SendMode::Target(target));
    }
    match dto.send_user_type {
        Some(SEND_USER_TYPE_ALL) => Ok(SendMode::All),
        Some(SEND_USER_TYPE_SINGLE) => {
            // Java：`Integer.valueOf(dto.getUserIds())`（非法格式直接 500）
            let raw = dto.user_ids.as_deref().map(str::trim).unwrap_or("");
            let id = raw
                .parse::<i32>()
                .map_err(|_| Error::Validation("用户集合格式有误".into()))?;
            Ok(SendMode::Single(id))
        }
        Some(SEND_USER_TYPE_RANK) => Ok(SendMode::Rank(dto.user_rank.unwrap_or(0))),
        Some(SEND_USER_TYPE_PART) => Ok(SendMode::Part),
        Some(SEND_USER_TYPE_TAG) => match &dto.member_audience {
            Some(raw) => {
                let audience: MemberAudienceDTO = serde_json::from_value(raw.clone())
                    .map_err(|_| Error::Validation("会员人群参数有误".into()))?;
                Ok(SendMode::Audience(audience))
            }
            None => Ok(SendMode::Empty),
        },
        Some(other) if other == 5 => match &dto.member_audience {
            Some(raw) => {
                let audience: MemberAudienceDTO = serde_json::from_value(raw.clone())
                    .map_err(|_| Error::Validation("会员人群参数有误".into()))?;
                Ok(SendMode::Audience(audience))
            }
            None => Ok(SendMode::Empty),
        },
        _ => Ok(SendMode::Empty),
    }
}

/// 日志 ID 必填校验（Java `@NotNull(message = 会员消息ID不能为空)`）。
fn require_log_id(param: &UserMessageLogRecallParam) -> Result<i32> {
    param
        .id
        .ok_or_else(|| Error::Validation(MSG_LOG_ID_REQUIRED.into()))
}

/// 实体 → VO（Java `UserMessageLogPageVO(UserMessageLog)`）。
#[must_use]
fn to_vo(e: &UserMessageLog) -> UserMessageLogPageVO {
    UserMessageLogPageVO {
        message_log_id: e.message_log_id,
        message_type: e.message_type,
        send_user_type: e.send_user_type,
        user_ids: e.user_ids.clone(),
        user_rank: e.user_rank,
        message_title: e.message_title.clone(),
        message_content: e.message_content.clone(),
        message_link: e.message_link.clone(),
        is_recall: e.is_recall,
        send_time: format_send_time(e.send_time),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log() -> UserMessageLog {
        UserMessageLog {
            message_log_id: 3,
            message_type: 1,
            send_user_type: 0,
            user_ids: "1,2".into(),
            user_rank: 0,
            message_title: "标题".into(),
            message_content: "内容".into(),
            message_link: r#"{"path":"order"}"#.into(),
            send_time: 1_700_000_000,
            is_recall: 0,
        }
    }

    #[test]
    fn vo_maps_and_formats_send_time() {
        let vo = to_vo(&log());
        assert_eq!(vo.message_log_id, 3);
        assert_eq!(vo.message_title, "标题");
        assert_eq!(vo.send_time, "2023-11-15 06:13:20");
        // 0 → ""（Java handelTime）
        let mut zero = log();
        zero.send_time = 0;
        assert_eq!(to_vo(&zero).send_time, "");
    }

    #[test]
    fn send_mode_matches_java_find_user_ids_branches() {
        let mut dto = UserMessageLogCreateDTO {
            send_user_type: Some(SEND_USER_TYPE_ALL),
            ..Default::default()
        };
        assert_eq!(send_mode(&dto).unwrap(), SendMode::All);

        dto.send_user_type = Some(SEND_USER_TYPE_SINGLE);
        dto.user_ids = Some("9".into());
        assert_eq!(send_mode(&dto).unwrap(), SendMode::Single(9));
        dto.user_ids = Some("abc".into());
        assert!(send_mode(&dto).is_err());

        dto.send_user_type = Some(SEND_USER_TYPE_RANK);
        dto.user_rank = Some(2);
        assert_eq!(send_mode(&dto).unwrap(), SendMode::Rank(2));

        dto.send_user_type = Some(SEND_USER_TYPE_PART);
        assert_eq!(send_mode(&dto).unwrap(), SendMode::Part);

        // 历史人群结构（标签）
        dto.send_user_type = Some(SEND_USER_TYPE_TAG);
        dto.member_audience = Some(serde_json::json!({
            "audienceType": 2,
            "tagIds": [7],
            "tagMatchMode": 1
        }));
        assert_eq!(
            send_mode(&dto).unwrap(),
            SendMode::Audience(MemberAudienceDTO {
                audience_type: Some(2),
                tag_ids: Some(vec![7]),
                tag_match_mode: Some(1),
                ..Default::default()
            })
        );
        // 统一人群结构优先
        dto.send_user_type = Some(1);
        dto.send_user_target = Some(serde_json::json!({"userIds": [1]}));
        assert_eq!(
            send_mode(&dto).unwrap(),
            SendMode::Target(SendUserTargetDTO {
                user_ids: Some(vec![1]),
                ..Default::default()
            })
        );

        // 未知/缺失 → 空人群
        let dto = UserMessageLogCreateDTO::default();
        assert_eq!(send_mode(&dto).unwrap(), SendMode::Empty);
    }

    #[test]
    fn recall_param_requires_id() {
        assert_eq!(
            require_log_id(&UserMessageLogRecallParam::default())
                .unwrap_err()
                .to_string(),
            format!("参数校验失败: {MSG_LOG_ID_REQUIRED}")
        );
        assert_eq!(
            require_log_id(&UserMessageLogRecallParam { id: Some(4) }).unwrap(),
            4
        );
    }

    #[test]
    fn empty_target_message_is_verbatim() {
        assert_eq!(MSG_NO_MEMBERS, "该会员选择类型下没有会员");
        assert_eq!(MSG_USERNAME_INVALID, "请检查输入的每个会员名称是否正确");
    }
}
