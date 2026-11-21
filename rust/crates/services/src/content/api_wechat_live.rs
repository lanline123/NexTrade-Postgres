//! 微信视频号直播记录域服务（content::api_wechat_live）。
//!
//! `getFinderLiveRecord` 已接通（2026-11-21 补齐，此前恒返回「未迁移」）：
//! 经**直播小程序**凭据的 access_token 调微信「视频号助手」开放接口
//! 拉取当前直播记录、落库 `finder_live_record`（迁移 028），再按选取
//! 口径（精确匹配 export_id 优先、未命中回退首条，见 core 模型模块头）
//! 返回单条；未在直播 → None（前台空值降级，与 Java 返回 null 同义）；
//! 未配置凭据 / 微信 errcode → 明确业务错误，不编造直播数据。
//! 凭据与 token 底座见 [`crate::content::wechat_finder`]。

use std::sync::Arc;

use nextrade_core::models::content::api_wechat_live::{
    select_record, FinderLiveRecordQuery, FinderLiveRecordVO,
};
use nextrade_core::Result;
use nextrade_infrastructure::db::repo::content::api_wechat_live::ApiWechatLiveRepository;
use nextrade_infrastructure::db::Repositories;

use crate::content::wechat_finder::fetch_and_persist_live_records;

/// 微信视频号直播记录服务。
pub struct ApiWechatLiveService {
    repos: Arc<Repositories>,
}

impl ApiWechatLiveService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本域仓储（NESTED：`self.repos.content.api_wechat_live`）。
    #[must_use]
    pub fn repo(&self) -> &ApiWechatLiveRepository {
        &self.repos.content.api_wechat_live
    }

    /// 当前直播记录（拉取 → 落库 → 选取单条）。
    ///
    /// # Errors
    /// - 直播小程序凭据未配置 → 明确业务错误（列明缺失配置位）
    /// - 微信 errcode / 网络错误 → 业务/内部错误（原文保留）
    /// - 落库失败 → 数据库错误
    pub async fn finder_live_record(
        &self,
        q: &FinderLiveRecordQuery,
    ) -> Result<Option<FinderLiveRecordVO>> {
        let records = fetch_and_persist_live_records(&self.repos).await?;
        Ok(select_record(&records, q.id.as_deref()).cloned())
    }
}
