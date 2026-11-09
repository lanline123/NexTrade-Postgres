//! 视频号账号服务（content::wechat_video_account）。
//!
//! 对应原 Java `com.nextrade.service.content.impl.WechatVideoAccountServiceImpl`。
//!
//! 构造签名固定：`pub fn new(repos: Arc<Repositories>) -> Self`。
//!
//! ## 已迁移（5）
//! `list`（finderId LIKE + 关联时间范围 + 店铺范围 + 动态排序）、`detail`、
//! `create`（finderId 唯一性校验 + 审计时间）、`update`（存在性 + 唯一性）、
//! `delete`（存在性校验）。
//!
//! 店铺范围：Java `getShopId()`；`admin` 为 `null`（无过滤），
//! 店铺/门店账号为具体店铺（仅 `> 0` 时过滤）。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::WechatVideoAccount;
use nextrade_core::models::content::wechat_video_account::{
    format_timestamp, validate_finder_id, OperateDTO, WechatVideoAccountCreateDTO,
    WechatVideoAccountListQuery, WechatVideoAccountUpdateDTO, WechatVideoAccountVO,
    MSG_FINDER_ID_EXISTS, MSG_ID_REQUIRED, MSG_NOT_FOUND,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

use crate::content::tenant_scope::{ensure_media_access, ensure_media_write, media_write_shop_id};

/// 视频号账号服务。
pub struct WechatVideoAccountService {
    repos: Arc<Repositories>,
}

impl WechatVideoAccountService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储（NESTED 访问：`self.repos.content.wechat_video_account`）。
    #[must_use]
    pub fn repo(
        &self,
    ) -> &nextrade_infrastructure::db::repo::content::wechat_video_account::WechatVideoAccountRepository
    {
        &self.repos.content.wechat_video_account
    }

    /// 列表（Java `#list`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn list(
        &self,
        shop_scope: Option<i32>,
        q: &WechatVideoAccountListQuery,
    ) -> Result<PageResult<WechatVideoAccountVO>> {
        let page = q.page_query();
        let (total, rows) = self.repo().paginate(shop_scope, q).await?;
        let list = rows.iter().map(to_vo).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 详情（Java `#detail`）。
    ///
    /// # Errors
    /// - 记录不存在 → 业务错误「视频号账号不存在」
    pub async fn detail(&self, id: Option<i32>) -> Result<WechatVideoAccountVO> {
        let id = id.ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let row = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_NOT_FOUND))?;
        // 多租户：平台账号可读全部；店铺账号读本店（平台素材 shop_id=0 可读）
        ensure_media_access(row.shop_id)?;
        Ok(to_vo(&row))
    }

    /// 新增（Java `#create`）。
    ///
    /// # Errors
    /// - finderId 缺失/超长 → `Validation`
    /// - finderId 已关联 → 业务错误「该视频号ID已关联」
    pub async fn create(
        &self,
        shop_scope: Option<i32>,
        dto: &WechatVideoAccountCreateDTO,
    ) -> Result<i32> {
        let finder_id = validate_finder_id(dto.finder_id.as_deref()).map_err(Error::Validation)?;
        if self
            .repo()
            .finder_exists(shop_scope, &finder_id, None)
            .await?
        {
            return Err(Error::biz_msg(MSG_FINDER_ID_EXISTS));
        }
        let now = now_seconds();
        let row = WechatVideoAccount {
            id: 0,
            finder_id,
            add_time: now,
            update_time: now,
            shop_id: media_write_shop_id(shop_scope)?,
        };
        let id = self.repo().insert(&row).await?;
        tracing::info!("新增视频号账号: id={id}");
        Ok(id)
    }

    /// 更新（Java `#update`）。
    ///
    /// # Errors
    /// - id/finderId 校验失败 → `Validation`
    /// - 账号不存在 → 业务错误「视频号账号不存在」
    /// - finderId 已关联 → 业务错误「该视频号ID已关联」
    pub async fn update(
        &self,
        shop_scope: Option<i32>,
        dto: &WechatVideoAccountUpdateDTO,
    ) -> Result<()> {
        let id = dto.id.ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let finder_id = validate_finder_id(dto.finder_id.as_deref()).map_err(Error::Validation)?;
        let existing = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_NOT_FOUND))?;
        // 多租户：仅本店账号可改
        ensure_media_write(existing.shop_id)?;
        if self
            .repo()
            .finder_exists(shop_scope, &finder_id, Some(id))
            .await?
        {
            return Err(Error::biz_msg(MSG_FINDER_ID_EXISTS));
        }
        self.repo()
            .update_finder(id, &finder_id, now_seconds())
            .await?;
        tracing::info!("更新视频号账号: id={id}");
        Ok(())
    }

    /// 删除（Java `#delete`）。
    ///
    /// # Errors
    /// - id 缺失 → `Validation("视频号账号ID不能为空")`
    /// - 账号不存在 → 业务错误「视频号账号不存在」
    pub async fn delete(&self, dto: &OperateDTO) -> Result<()> {
        let id = dto.id.ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let existing = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_NOT_FOUND))?;
        // 多租户：仅本店账号可删
        ensure_media_write(existing.shop_id)?;
        self.repo().delete(id).await?;
        tracing::info!("删除视频号账号: id={id}");
        Ok(())
    }
}

/// 实体 → VO（Java `convertToVO` + `formatTimestamp`）。
#[must_use]
fn to_vo(e: &WechatVideoAccount) -> WechatVideoAccountVO {
    WechatVideoAccountVO {
        id: e.id,
        finder_id: e.finder_id.clone(),
        relation_time: format_timestamp(e.add_time),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account() -> WechatVideoAccount {
        WechatVideoAccount {
            id: 3,
            finder_id: "finder_1".into(),
            add_time: 1_700_000_000,
            update_time: 1_700_000_100,
            shop_id: 0,
        }
    }

    #[test]
    fn to_vo_formats_relation_time() {
        let vo = to_vo(&account());
        assert_eq!(vo.id, 3);
        assert_eq!(vo.finder_id, "finder_1");
        assert_eq!(vo.relation_time, "2023-11-15 06:13:20");
        let mut zero = account();
        zero.add_time = 0;
        assert_eq!(to_vo(&zero).relation_time, "");
    }

    #[test]
    fn verbatim_messages() {
        assert_eq!(MSG_ID_REQUIRED, "视频号账号ID不能为空");
        assert_eq!(MSG_NOT_FOUND, "视频号账号不存在");
        assert_eq!(MSG_FINDER_ID_EXISTS, "该视频号ID已关联");
    }

    #[test]
    fn shop_scope_only_applies_when_positive() {
        // 与 Java `shopId != null && shopId > 0` 同义
        let applies = |scope: Option<i32>| scope.filter(|s| *s > 0);
        assert_eq!(applies(None), None);
        assert_eq!(applies(Some(0)), None);
        assert_eq!(applies(Some(-1)), None);
        assert_eq!(applies(Some(5)), Some(5));
    }
}
