//! 前台用户留言服务（member::user_feedback_client）。
//! 构造 `new(repos)`；嵌套 `self.repos.member.user_feedback_client`。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::models::member::user_feedback_client::{
    contains_sensitive, FeedbackListQuery, FeedbackCreateDTO, UserFeedbackVO, CONTENT_REQUIRED,
    MEMBER_NOT_FOUND, NOT_OWN_ORDER, ORDER_NOT_EXIST, SENSITIVE_WORD, SERVICE_DATA_ERROR,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::member::user_feedback_client::UserFeedbackClientRepository;
use nextrade_infrastructure::db::Repositories;

pub struct UserFeedbackClientService {
    repos: Arc<Repositories>,
}

impl UserFeedbackClientService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    #[must_use]
    pub fn repo(&self) -> &UserFeedbackClientRepository {
        &self.repos.member.user_feedback_client
    }

    /// `GET /api/user/feedback/list`。
    pub async fn list(
        &self,
        user_id: i32,
        q: &FeedbackListQuery,
    ) -> Result<PageResult<UserFeedbackVO>> {
        let (total, list) = self.repo().paginate(user_id, q).await?;
        let (page, size) = q.page_params();
        Ok(PageResult::new(
            total,
            list,
            &PageQuery {
                page,
                page_size: size,
            },
        ))
    }

    /// `POST /api/user/feedback/submit`（XSS/敏感词/订单归属本地校验 + 落库）。
    pub async fn submit(&self, user_id: i32, dto: &FeedbackCreateDTO) -> Result<i32> {
        let content = dto
            .content
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, CONTENT_REQUIRED))?;
        let title = dto.title.as_deref().unwrap_or("").trim();
        // 敏感词检查（Java 配置项 `msgHackWord`）
        let hack_words = self.repo().config_value("msgHackWord").await?;
        let to_check = format!("{content}{title}");
        if contains_sensitive(&to_check, hack_words.as_deref()) {
            return Err(Error::biz(SERVICE_DATA_ERROR, SENSITIVE_WORD));
        }
        let Some((username, user_mobile)) = self.repo().user_exists(user_id).await? else {
            return Err(Error::biz(SERVICE_DATA_ERROR, MEMBER_NOT_FOUND));
        };
        // 订单归属（Java：orderId > 0 时校验）
        if let Some(order_id) = dto.order_id.filter(|id| *id > 0) {
            match self.repo().order_owner(order_id).await? {
                None => return Err(Error::biz(SERVICE_DATA_ERROR, ORDER_NOT_EXIST)),
                Some(owner) if owner != user_id => {
                    return Err(Error::biz(SERVICE_DATA_ERROR, NOT_OWN_ORDER))
                }
                Some(_) => {}
            }
        }
        let pics = dto
            .feedback_pics
            .as_ref()
            .map(|p| serde_json::to_string(p).unwrap_or_else(|_| "[]".into()))
            .unwrap_or_else(|| "[]".into());
        let mut tx = self.repo().begin().await?;
        let id = self
            .repo()
            .insert(
                &mut tx,
                user_id,
                &username,
                dto.r#type.unwrap_or(3),
                title,
                content,
                dto.product_id,
                dto.order_id,
                dto.shop_id,
                dto.complaint_info.as_deref(),
                dto.email.as_deref().or(Some("")),
                dto.mobile.as_deref().or(Some(user_mobile.as_str())),
                &pics,
                i32::try_from(now_seconds()).unwrap_or(0),
            )
            .await?;
        tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_required_and_sensitive() {
        assert_eq!(CONTENT_REQUIRED, "留言内容不能为空");
        assert!(contains_sensitive("有违禁词", Some("违禁词")));
    }

    #[test]
    fn messages_verbatim() {
        assert_eq!(MEMBER_NOT_FOUND, "会员不存在");
        assert_eq!(ORDER_NOT_EXIST, "订单不存在");
        assert_eq!(NOT_OWN_ORDER, "非本人订单，无权提交留言");
        assert_eq!(SENSITIVE_WORD, "留言内容包含敏感词");
    }
}
