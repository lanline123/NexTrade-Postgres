//! 客户端发版登记服务（system::client_release，我方新增）。
//!
//! admin 自助热更新 P4（设计见 hot-update-design/REPORT.md §5.4）：小程序与
//! App 不能热更，只能提醒。本服务只管「已发布登记」这一侧事实：
//! - 登记：平台管理员在客户端完成微信提审发布 / App 发版后人工登记
//!   （渠道 + 对应产品版本 + 已发布客户端版本），落 `sys_client_release`；
//! - 概览：逐渠道给最近登记与「是否覆盖目标产品版本」（纯函数
//!   [`is_covered`]），App 渠道附带原版 appVersion 面的现行配置版本号
//!   （只读复用 [`AppVersionAdminService::detail`]，方便登记时对照）；
//! - 消警：派生态——前端把清单标志（P1 状态口径）与本概览的 `covered`
//!   组合即得提醒存否，不落提醒行、无已读状态，不另造提醒通道。
//!
//! 与 P1/P2 的边界：清单契约（`ReleaseManifest` / `ManifestCompat`）与
//! 「系统更新」页归 P1/P2，本服务不读清单、不改其结构。

use std::sync::Arc;

use nextrade_core::models::system::client_release::{
    is_covered, is_valid_channel, ChannelReleaseVO, ClientReleaseOverviewVO, ClientReleaseRow,
    ClientReleaseVO, RegisterClientReleaseParam, CHANNEL_APP, CHANNEL_MINI_PROGRAM,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

use super::app_version_admin::AppVersionAdminService;

/// 两渠道固定顺序（概览恒两行）。
const CHANNELS: [&str; 2] = [CHANNEL_MINI_PROGRAM, CHANNEL_APP];

/// 客户端发版登记服务。
pub struct ClientReleaseService {
    repos: Arc<Repositories>,
    app_version_admin: Arc<AppVersionAdminService>,
}

impl ClientReleaseService {
    pub fn new(
        repos: Arc<Repositories>,
        app_version_admin: Arc<AppVersionAdminService>,
    ) -> Self {
        Self {
            repos,
            app_version_admin,
        }
    }

    /// 登记总览。`target_version` 为最新产品版本（前端从系统更新状态透传），
    /// 用于逐渠道计算 `covered`；未传时 `covered` 恒 false。
    pub async fn overview(
        &self,
        target_version: Option<&str>,
    ) -> Result<ClientReleaseOverviewVO> {
        let target = target_version.map(str::trim).filter(|s| !s.is_empty());
        let repo = &self.repos.system.client_release;
        let mut channels = Vec::with_capacity(CHANNELS.len());
        for channel in CHANNELS {
            let latest = repo.latest_by_channel(channel).await?;
            let covered = match (target, latest.as_ref()) {
                (Some(t), Some(row)) => is_covered(Some(&row.product_version), t),
                _ => false,
            };
            let (configured_android_version, configured_ios_version) = if channel == CHANNEL_APP
            {
                match self.app_version_admin.detail().await {
                    Ok(d) => (d.android_version, d.ios_version),
                    Err(e) => {
                        tracing::warn!("读 APP 版本配置失败（登记概览降级）: {e}");
                        (None, None)
                    }
                }
            } else {
                (None, None)
            };
            channels.push(ChannelReleaseVO {
                channel: channel.to_string(),
                label: nextrade_core::models::system::client_release::channel_label(channel)
                    .to_string(),
                latest: latest.as_ref().map(ClientReleaseVO::from),
                covered,
                configured_android_version,
                configured_ios_version,
            });
        }
        let recent = repo
            .list_recent(20)
            .await?
            .iter()
            .map(ClientReleaseVO::from)
            .collect();
        Ok(ClientReleaseOverviewVO {
            target_product_version: target.map(str::to_string),
            channels,
            recent,
        })
    }

    /// 登记一条已发布记录（只增不改；纠错走删除后重登）。
    pub async fn register(
        &self,
        param: &RegisterClientReleaseParam,
        operator_id: i32,
        operator_name: &str,
    ) -> Result<ClientReleaseVO> {
        let channel = param.channel.trim();
        if !is_valid_channel(channel) {
            return Err(Error::Validation(format!("未知渠道：{channel}")));
        }
        let product_version = param.product_version.trim();
        if product_version.is_empty() {
            return Err(Error::Validation("产品版本不能为空".into()));
        }
        let client_version = param.client_version.trim();
        if client_version.is_empty() {
            return Err(Error::Validation("已发布客户端版本不能为空".into()));
        }
        let note: String = param.note.trim().chars().take(500).collect();
        let row = ClientReleaseRow {
            id: 0,
            channel: channel.to_string(),
            product_version: product_version.to_string(),
            client_version: client_version.to_string(),
            note,
            operator_id: i64::from(operator_id),
            operator_name: operator_name.to_string(),
            created_at: chrono::Utc::now().timestamp(),
        };
        let id = self.repos.system.client_release.insert(&row).await?;
        let saved = ClientReleaseRow { id, ..row };

        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_current_log(
                "admin",
                0,
                &format!(
                    "登记客户端发版：{} 产品版本 {} → 客户端版本 {}",
                    nextrade_core::models::system::client_release::channel_label(channel),
                    saved.product_version,
                    saved.client_version
                ),
            )
            .await
        {
            tracing::warn!("写管理操作日志失败（客户端发版登记）: {e}");
        }
        Ok(ClientReleaseVO::from(&saved))
    }

    /// 删除一条登记（纠错用）。删不到行返回明确错误，不静默成功。
    pub async fn remove(&self, id: i64, operator_name: &str) -> Result<()> {
        if id <= 0 {
            return Err(Error::Validation("登记 id 非法".into()));
        }
        let removed = self.repos.system.client_release.delete_by_id(id).await?;
        if !removed {
            return Err(Error::NotFound("登记记录不存在".into()));
        }
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_current_log(
                "admin",
                0,
                &format!("删除客户端发版登记 #{id}（操作人 {operator_name}）"),
            )
            .await
        {
            tracing::warn!("写管理操作日志失败（客户端发版登记删除）: {e}");
        }
        Ok(())
    }
}
