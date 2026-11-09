//! 公众号自动回复服务（content::wechat_auto_reply）。
//!
//! 对应原 Java `com.nextrade.service.setting.wechat.impl.WechatAutoReplyServiceImpl`。
//!
//! 构造签名固定：`pub fn new(repos: Arc<Repositories>) -> Self`。
//!
//! ## 已迁移（6）
//! `getSubscribe` / `getDefault` / `getLoginSuccess`（读 `config` + JSON 解析 +
//! 失败回落空载荷）、`saveSubscribe` / `saveDefault` / `saveLoginSuccess`
//! （六条本地校验 + upsert）。
//!
//! ## 与 Java 的差异
//! 本域 6 个端点**不调用任何微信外部 API**（Java 实现同样只读写 `config` 表），
//! 因此无「未迁移」分支；其余差异见
//! [`crate::models::content::wechat_auto_reply`] 模块头。

use std::sync::Arc;

use nextrade_core::models::content::wechat_auto_reply::{
    parse_payload, to_storage_json, validate_payload, ReplyPayloadDTO, ReplyPayloadVO,
    BIZ_DEFAULT, BIZ_LOGIN_SUCCESS, BIZ_SUBSCRIBE,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 公众号自动回复服务。
pub struct WechatAutoReplyService {
    repos: Arc<Repositories>,
}

impl WechatAutoReplyService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储（NESTED 访问：`self.repos.content.wechat_auto_reply`）。
    #[must_use]
    pub fn repo(
        &self,
    ) -> &nextrade_infrastructure::db::repo::content::wechat_auto_reply::WechatAutoReplyRepository
    {
        &self.repos.content.wechat_auto_reply
    }

    /// 关注后自动回复（Java `getSubscribe`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn get_subscribe(&self) -> Result<ReplyPayloadVO> {
        self.load(BIZ_SUBSCRIBE).await
    }

    /// 默认自动回复（Java `getDefault`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn get_default(&self) -> Result<ReplyPayloadVO> {
        self.load(BIZ_DEFAULT).await
    }

    /// 扫码登录成功回复（Java `getLoginSuccess`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn get_login_success(&self) -> Result<ReplyPayloadVO> {
        self.load(BIZ_LOGIN_SUCCESS).await
    }

    /// 保存关注后自动回复（Java `saveSubscribe`）。
    ///
    /// # Errors
    /// - 六条校验失败 → 对应逐字文案（`Error::biz_msg`）
    /// - 数据库失败 → `Error::Database`
    pub async fn save_subscribe(&self, dto: &ReplyPayloadDTO) -> Result<()> {
        self.save(BIZ_SUBSCRIBE, dto).await
    }

    /// 保存默认自动回复（Java `saveDefault`）。
    pub async fn save_default(&self, dto: &ReplyPayloadDTO) -> Result<()> {
        self.save(BIZ_DEFAULT, dto).await
    }

    /// 保存扫码登录成功回复（Java `saveLoginSuccess`）。
    pub async fn save_login_success(&self, dto: &ReplyPayloadDTO) -> Result<()> {
        self.save(BIZ_LOGIN_SUCCESS, dto).await
    }

    /// 读配置并反序列化（Java `loadByCode`）。
    async fn load(&self, biz_code: &str) -> Result<ReplyPayloadVO> {
        let raw = self.repo().find_biz_val(biz_code).await?;
        let value = raw.flatten();
        Ok(parse_payload(value.as_deref()))
    }

    /// 校验 + upsert（Java `upsert`）。
    async fn save(&self, biz_code: &str, dto: &ReplyPayloadDTO) -> Result<()> {
        validate_payload(dto).map_err(Error::biz_msg)?;
        let json = to_storage_json(dto);
        self.repo().upsert(biz_code, &json).await?;
        tracing::info!("保存公众号自动回复配置: biz_code={biz_code}");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::content::wechat_auto_reply::{
        MSG_CONTENT_REQUIRED, MSG_MSG_TYPE_REQUIRED, MSG_TYPE_TEXT,
    };

    #[test]
    fn save_validation_rejects_invalid_payload_before_db() {
        // 纯校验分支（不触库）
        let dto = ReplyPayloadDTO::default();
        assert_eq!(
            validate_payload(&dto).unwrap_err(),
            MSG_MSG_TYPE_REQUIRED
        );
        let dto = ReplyPayloadDTO {
            msg_type: Some(MSG_TYPE_TEXT),
            content: Some("  ".into()),
            ..Default::default()
        };
        assert_eq!(
            validate_payload(&dto).unwrap_err(),
            MSG_CONTENT_REQUIRED
        );
    }

    #[test]
    fn storage_json_round_trip_via_service_helpers() {
        let dto = ReplyPayloadDTO {
            msg_type: Some(1),
            content: Some("欢迎关注".into()),
            ..Default::default()
        };
        let json = to_storage_json(&dto);
        let vo = parse_payload(Some(&json));
        assert_eq!(vo.msg_type, Some(1));
        assert_eq!(vo.content.as_deref(), Some("欢迎关注"));
    }

    #[test]
    fn default_payload_matches_java_fallback() {
        let vo = parse_payload(None);
        assert_eq!(vo, ReplyPayloadVO::default_payload());
        assert_eq!(vo.msg_type, Some(1));
        assert_eq!(vo.content.as_deref(), Some(""));
    }

    #[test]
    fn biz_codes_are_used_verbatim() {
        assert_eq!(BIZ_SUBSCRIBE, "wechatSubscribeReply");
        assert_eq!(BIZ_DEFAULT, "wechatDefaultReply");
        assert_eq!(BIZ_LOGIN_SUCCESS, "wechatLoginSuccessReply");
    }
}
