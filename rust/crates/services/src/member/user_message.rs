//! 站内信（C 端）服务（member::user_message）。
//!
//! 对应原 Java `UserMessageServiceImpl`（`java/nextrade-service/.../user/impl/UserMessageServiceImpl.java`）
//! 与继承的 `BaseServiceImpl`。
//!
//! 构造签名固定：`pub fn new(repos: Arc<Repositories>) -> Self`。
//!
//! ## 已迁移（6）
//! - `list`：`user_id` + 未读 + 类型过滤 + 动态排序 + 分页；时间三段格式化；`link` JSON
//! - `setAllMessageRead`：站内信全部置读 **+ `im_message` 置读**（Java 调
//!   `ImMessageService#setAllRead`，语义为 `type = 2` 的 IM 消息）
//! - `setOrderMessageRead`：`type = 1` 的订单通知置读
//! - `setMessageRead`：按 `user_id + message_id` 置读
//! - `del`：校验归属后删除
//! - `getUnreadMessageCount`：站内信 / 订单 / IM 三个未读数
//!
//! ## 与 Java 的差异（诚实记录）
//! 1. **`orderInfo` 未迁移**：Java 对订单通知会查 `OrderService#detail` 并在
//!    「拆单父单不可见」时隐藏通知；本实现返回原始通知、`orderInfo = null`
//!    （见 models 模块头）。
//! 2. **`messageTemplate` 翻译未迁移**：`content` 返回库中原文。
//! 3. 登录校验：Java 控制器在 `updateAllRead` / `updateOrderMessageRead` 里
//!    匿名访问抛 `RuntimeException("用户未登录")`，本实现用 `Error::biz_msg`
//!    保留同一文案（业务错误码沿用项目惯例 500，Java `GlobalException` 为 1001）。
//! 4. `del` 的 id 缺失：Java 公共 `OperateDTO` 有 `@Valid` →「id不能为空」；
//!    本实现按同一文案提前校验（`updateMessageRead` 的 Java DTO 无校验，缺 id 返回 `false`）。

use std::sync::Arc;

use nextrade_core::entities::UserMessage;
use nextrade_core::models::member::user_message::{
    format_message_times, parse_link, UserMessageIdDTO, UserMessageListQuery, UserMessageVO,
    UnreadMessageCountVO, MSG_DEL_NOT_FOUND, MSG_DEL_NO_PERMISSION, MSG_ID_REQUIRED,
    MSG_USER_NOT_LOGIN,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 站内信服务。
pub struct UserMessageService {
    repos: Arc<Repositories>,
}

impl UserMessageService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储（NESTED 访问：`self.repos.member.user_message`）。
    #[must_use]
    pub fn repo(&self) -> &nextrade_infrastructure::db::repo::member::user_message::UserMessageRepository
    {
        &self.repos.member.user_message
    }

    /// 列表（Java `UserMessageServiceImpl#list`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn list(
        &self,
        user_id: i32,
        q: &UserMessageListQuery,
    ) -> Result<PageResult<UserMessageVO>> {
        let page = q.page_query();
        let (total, rows) = self.repo().paginate(user_id, q).await?;
        let list = rows.iter().map(to_vo).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 全部置为已读（Java 控制器 `setAllMessageRead` + `UserMessageServiceImpl`）。
    ///
    /// 匿名（`user_id == 0`）→ 「用户未登录」（Java `RuntimeException`）。
    ///
    /// # Errors
    /// - 未登录 → 业务错误「用户未登录」
    /// - 数据库失败 → `Error::Database`
    pub async fn set_all_read(&self, user_id: i32) -> Result<()> {
        ensure_logged_in(user_id)?;
        self.repo().update_all_read(user_id).await?;
        // Java：`imMessageService.setAllRead(userId)`（type=2 的 IM 消息置读）
        self.repo().update_all_im_read(user_id).await?;
        tracing::info!("用户站内信全部置读: user_id={user_id}");
        Ok(())
    }

    /// 订单消息置为已读（Java `setOrderMessageRead`）。
    ///
    /// # Errors
    /// - 未登录 → 业务错误「用户未登录」
    /// - 数据库失败 → `Error::Database`
    pub async fn set_order_read(&self, user_id: i32) -> Result<()> {
        ensure_logged_in(user_id)?;
        self.repo().update_order_read(user_id).await?;
        tracing::info!("用户订单消息置读: user_id={user_id}");
        Ok(())
    }

    /// 单条消息置为已读（Java `setMessageRead`）。
    ///
    /// Java `UserMessageUpdateDTO{id}` 无校验：`id` 缺失时服务返回 `false`
    /// （无副作用）。本实现同样返回 `bool`、不报错。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn set_message_read(&self, user_id: i32, dto: &UserMessageIdDTO) -> Result<bool> {
        let Some(message_id) = dto.id else {
            return Ok(false);
        };
        let affected = self.repo().set_read(user_id, message_id).await?;
        Ok(affected > 0)
    }

    /// 删除站内信（Java `deleteUserMessage`）。
    ///
    /// # Errors
    /// - `id` 缺失 → `Validation("id不能为空")`（Java `@Valid OperateDTO`）
    /// - `id == 0` → 业务错误「删除失败，消息不存在」
    /// - 非本人消息 → 业务错误「删除失败，您没有权限删除此消息」
    /// - 数据库失败 → `Error::Database`
    pub async fn delete(&self, user_id: i32, dto: &UserMessageIdDTO) -> Result<()> {
        let id = validate_delete_id(dto.id)?;
        if let Some(row) = self.repo().find_by_id(id).await? {
            if row.user_id != user_id {
                return Err(Error::biz_msg(MSG_DEL_NO_PERMISSION));
            }
        }
        // Java：记录不存在时 removeById(id) 影响 0 行，不报错
        self.repo().delete(id).await?;
        tracing::info!("删除站内信: user_id={user_id}, message_id={id}");
        Ok(())
    }

    /// 未读统计（Java `getUnreadMessageCount`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn unread_count(&self, user_id: i32) -> Result<UnreadMessageCountVO> {
        Ok(UnreadMessageCountVO {
            user_message_count: self.repo().count_unread_all(user_id).await?,
            order_message_count: self.repo().count_unread_order(user_id).await?,
            im_message_count: self.repo().count_unread_im(user_id).await?,
        })
    }
}

/// 登录校验（Java 控制器 `RuntimeException("用户未登录")`）。
fn ensure_logged_in(user_id: i32) -> Result<()> {
    if user_id == 0 {
        return Err(Error::biz_msg(MSG_USER_NOT_LOGIN));
    }
    Ok(())
}

/// `del` 的 id 校验：缺失 → 「id不能为空」；0 → 「删除失败，消息不存在」。
fn validate_delete_id(id: Option<i32>) -> Result<i32> {
    let id = id.ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
    if id == 0 {
        return Err(Error::biz_msg(MSG_DEL_NOT_FOUND));
    }
    Ok(id)
}

/// 实体 → VO（Java `BeanUtils.copyProperties` + 时间格式化 + `link` 解析）。
#[must_use]
fn to_vo(e: &UserMessage) -> UserMessageVO {
    let (add_time, add_time_date, add_time_hms) = format_message_times(e.add_time);
    UserMessageVO {
        message_id: e.message_id,
        title: e.title.clone(),
        message_log_id: e.message_log_id,
        user_id: e.user_id,
        user_rank: e.user_rank,
        is_read: e.is_read,
        content: e.content.clone(),
        link: parse_link(&e.link),
        add_time,
        add_time_date,
        add_time_hms,
        // trade::order 用例层未迁移 → 不伪造订单数据
        order_info: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message() -> UserMessage {
        UserMessage {
            message_id: 11,
            message_log_id: 3,
            user_id: 5,
            user_rank: 0,
            is_read: 0,
            title: "发货通知".into(),
            content: "您的订单已发货".into(),
            message_template: None,
            link: r#"{"path":"order","id":9,"data":"{}"}"#.into(),
            add_time: 1_700_000_000,
            r#type: 1,
        }
    }

    #[test]
    fn to_vo_formats_times_and_parses_link() {
        let vo = to_vo(&message());
        assert_eq!(vo.message_id, 11);
        assert_eq!(vo.title, "发货通知");
        assert_eq!(vo.add_time, "2023-11-15 06:13:20");
        assert_eq!(vo.add_time_date, "2023-11-15");
        assert_eq!(vo.add_time_hms, "06:13:20");
        let link = vo.link.expect("合法 JSON 应解析为对象");
        assert_eq!(link["path"], "order");
        assert_eq!(link["id"], 9);
        assert!(vo.order_info.is_none());
    }

    #[test]
    fn to_vo_tolerates_invalid_link() {
        let mut m = message();
        m.link = String::new();
        assert!(to_vo(&m).link.is_none());
        m.link = "not-json".into();
        assert!(to_vo(&m).link.is_none());
        m.link = "null".into();
        assert!(to_vo(&m).link.is_none());
    }

    #[test]
    fn delete_id_validation_matches_java() {
        assert_eq!(
            validate_delete_id(None).unwrap_err().to_string(),
            format!("参数校验失败: {MSG_ID_REQUIRED}")
        );
        match validate_delete_id(Some(0)) {
            Err(Error::Biz { msg, .. }) => assert_eq!(msg, MSG_DEL_NOT_FOUND),
            other => panic!("应为业务错误，实际: {other:?}"),
        }
        assert_eq!(validate_delete_id(Some(7)).unwrap(), 7);
    }

    #[test]
    fn login_guard_blocks_anonymous() {
        match ensure_logged_in(0) {
            Err(Error::Biz { msg, .. }) => assert_eq!(msg, MSG_USER_NOT_LOGIN),
            other => panic!("应为业务错误，实际: {other:?}"),
        }
        assert!(ensure_logged_in(1).is_ok());
    }
}
