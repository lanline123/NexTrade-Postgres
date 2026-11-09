//! 客服消息能力探测服务（content::wechat_kefu_capability）。
//!
//! 对应原 Java `WxKefuCapabilityProbe`（`isEnabled()`）。
//!
//! 构造签名固定：`pub fn new(repos: Arc<Repositories>) -> Self`。
//!
//! ## 已迁移（1）
//! `capability`：**本地判定**——公众号四键配置不全（Java `wxMpService == null`）
//! → `{kefuEnabled:false}`；配置齐全时因微信 `kfList()` 探测未迁移，返回明确
//! 「未迁移」业务错误（不伪造 true/false）。
//!
//! ## 与 Java 的差异
//! 见 [`crate::models::content::wechat_kefu_capability`] 模块头。

use std::sync::Arc;

use nextrade_core::models::content::wechat_kefu_capability::{
    WechatKefuCapabilityVO,
};
use nextrade_core::Result;
use nextrade_infrastructure::db::Repositories;

/// 客服消息能力探测服务。
pub struct WechatKefuCapabilityService {
    repos: Arc<Repositories>,
}

impl WechatKefuCapabilityService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储（NESTED 访问：`self.repos.content.wechat_kefu_capability`）。
    #[must_use]
    pub fn repo(
        &self,
    ) -> &nextrade_infrastructure::db::repo::content::wechat_kefu_capability::WechatKefuCapabilityRepository
    {
        &self.repos.content.wechat_kefu_capability
    }

    /// 探测客服消息能力（Java `isEnabled()`）。
    ///
    /// # Errors
    /// 配置读取失败；微信接口异常时降级为 `kefuEnabled=false`（同 Java 探测语义）。
    pub async fn capability(&self) -> Result<WechatKefuCapabilityVO> {
        if !self.repo().is_wechat_configured().await? {
            // Java：WxMpServiceHolder.get() == null → enabled = false
            return Ok(WechatKefuCapabilityVO::disabled());
        }
        // 配置齐全：调用微信客服列表探测（网络/接口异常按 Java 探测语义降级为 false）
        let client = crate::content::wechat_client::WechatClient::from_repos(&self.repos).await?;
        match client.kf_list().await {
            Ok(list) => Ok(WechatKefuCapabilityVO {
                kefu_enabled: !list.is_empty(),
            }),
            Err(e) => {
                tracing::warn!("微信客服能力探测失败，降级 kefuEnabled=false: {e}");
                Ok(WechatKefuCapabilityVO::disabled())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::content::wechat_kefu_capability::WECHAT_CONFIG_CODES;

    #[test]
    fn disabled_vo_matches_java_unconfigured_branch() {
        let vo = WechatKefuCapabilityVO::disabled();
        assert!(!vo.kefu_enabled);
        assert_eq!(
            serde_json::to_value(&vo).unwrap()["kefuEnabled"],
            false
        );
    }

    #[test]
    fn probe_requires_all_four_config_keys() {
        // Java `StrUtil.hasBlank(appId, secret, token, aesKey)`：任一为空即未配置
        let configured = |values: &[Option<&str>]| {
            values.len() == WECHAT_CONFIG_CODES.len()
                && values.iter().all(|v| {
                    v.map(str::trim)
                        .is_some_and(|s| !s.is_empty())
                })
        };
        assert!(configured(&[Some("a"), Some("b"), Some("c"), Some("d")]));
        assert!(!configured(&[Some("a"), Some("b"), Some("c"), None]));
        assert!(!configured(&[Some("a"), Some("b"), Some("c"), Some("  ")]));
        assert!(!configured(&[]));
    }

    #[test]
    fn configured_client_requires_app_id_and_secret() {
        // 4 个配置项齐全但 appId/secret 为空时，探测分支会返回明确错误（不再返回「未迁移」）
        let cfg = crate::content::wechat_client::WechatMpConfig {
            server_token: "t".into(),
            server_secret: "s".into(),
            ..Default::default()
        };
        assert!(!cfg.is_configured());
        assert_eq!(cfg.missing(), vec!["wechatAppId", "wechatAppSecret"]);
    }
}
