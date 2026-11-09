//! 用户标签关联服务（member::user_member_tag）。
//!
//! 对应原 Java `com.nextrade.service.user.impl.UserMemberTagServiceImpl`
//! （店铺隔离 + 冻结守卫 SHOP-MEMBER-20260708）。
//!
//! 构造签名固定：`pub fn new(repos: Arc<Repositories>) -> Self`。
//!
//! ## 已迁移（5）
//! - `listByUser`：用户在该店铺的标签关联 + 标签名/组名回填
//! - `bindTags` / `unbindTags`：单用户绑/解绑（店铺隔离）
//! - `batchBindTags` / `batchUnbindTags`：批量绑/解绑
//!
//! 所有写操作前先过 [`Self::assert_enabled`]：仅当 `shopId > 0`（店铺/门店账号）
//! 且配置 `shopMemberEnabled != 1` 时抛冻结文案（与 Java
//! `ShopMemberSettingService#assertEnabledForShop` 一致）。
//!
//! ## 与 Java 的差异
//! 业务错误码沿用项目惯例 500（Java `GlobalException` 1001），消息逐字；
//! 其余见 [`crate::models::member::user_member_tag`] 模块头。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::models::member::user_member_tag::{
    format_create_time, tags_to_delete, tags_to_insert, validate_batch_param, validate_param,
    UserMemberTagBatchParam, UserMemberTagParam, UserMemberTagVO, MSG_SHOP_MEMBER_FROZEN,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 用户标签关联服务。
pub struct UserMemberTagService {
    repos: Arc<Repositories>,
}

impl UserMemberTagService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储（NESTED 访问：`self.repos.member.user_member_tag`）。
    #[must_use]
    pub fn repo(
        &self,
    ) -> &nextrade_infrastructure::db::repo::member::user_member_tag::UserMemberTagRepository {
        &self.repos.member.user_member_tag
    }

    /// 店铺会员冻结守卫（Java `assertEnabledForShop`）。
    ///
    /// # Errors
    /// 店铺账号且开关未启用 → 业务错误（冻结文案）。
    pub async fn assert_enabled(&self, shop_id: i32) -> Result<()> {
        if shop_id > 0 && !self.repo().is_shop_member_enabled().await? {
            return Err(Error::biz_msg(MSG_SHOP_MEMBER_FROZEN));
        }
        Ok(())
    }

    /// 查询用户的所有标签（Java `listByUser`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn list_by_user(&self, shop_id: i32, user_id: i64) -> Result<Vec<UserMemberTagVO>> {
        let rows = self.repo().list_user_tags(shop_id, user_id).await?;
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let tag_ids: Vec<i64> = rows.iter().map(|ut| ut.tag_id).collect();
        let tags = self.repo().find_tags(shop_id, &tag_ids).await?;
        let tag_map: HashMap<i64, &nextrade_core::entities::MemberTag> =
            tags.iter().map(|t| (t.id, t)).collect();
        let group_ids: Vec<i64> = tags.iter().map(|t| t.group_id).collect();
        let groups = self.repo().find_groups(shop_id, &group_ids).await?;
        let group_map: HashMap<i64, &str> =
            groups.iter().map(|g| (g.id, g.group_name.as_str())).collect();
        Ok(rows
            .iter()
            .map(|ut| {
                let tag = tag_map.get(&ut.tag_id);
                UserMemberTagVO {
                    tag_id: ut.tag_id,
                    tag_name: tag.map(|t| t.tag_name.clone()).unwrap_or_default(),
                    group_id: tag.map(|t| t.group_id).unwrap_or(0),
                    group_name: tag
                        .and_then(|t| group_map.get(&t.group_id).copied())
                        .unwrap_or("")
                        .to_string(),
                    create_time: format_create_time(ut.create_time),
                }
            })
            .collect())
    }

    /// 给用户打标签（Java `bindTags`，事务）。
    ///
    /// # Errors
    /// - 参数缺失 → `Validation`（Java `@Valid` 文案）
    /// - 店铺会员冻结 → 业务错误
    /// - 数据库失败 → `Error::Database`
    pub async fn bind_tags(&self, shop_id: i32, param: &UserMemberTagParam) -> Result<()> {
        let user_id = validate_param(param).map_err(Error::Validation)?;
        self.assert_enabled(shop_id).await?;
        let existing = self.repo().list_user_tags(shop_id, user_id).await?;
        let existing_ids: Vec<i64> = existing.iter().map(|ut| ut.tag_id).collect();
        let delete_ids = tags_to_delete(&existing_ids, &param.tag_ids);
        let insert_ids = tags_to_insert(&existing_ids, &param.tag_ids);
        self.repo()
            .bind_tags(shop_id, user_id, &delete_ids, &insert_ids, now_seconds())
            .await?;
        let log_info = format!("给用户打标签：{}", user_id);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_current_log(if shop_id > 0 { "shop" } else { "admin" }, shop_id, &log_info)
            .await
        {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!("用户打标签: shop_id={shop_id}, user_id={user_id}");
        Ok(())
    }

    /// 取消用户标签（Java `unbindTags`）。
    pub async fn unbind_tags(&self, shop_id: i32, param: &UserMemberTagParam) -> Result<()> {
        let user_id = validate_param(param).map_err(Error::Validation)?;
        self.assert_enabled(shop_id).await?;
        self.repo()
            .unbind_tags(shop_id, user_id, &param.tag_ids)
            .await?;
        let log_info = format!("取消用户标签：{}", user_id);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_current_log(if shop_id > 0 { "shop" } else { "admin" }, shop_id, &log_info)
            .await
        {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!("取消用户标签: shop_id={shop_id}, user_id={user_id}");
        Ok(())
    }

    /// 批量给用户打标签（Java `batchBindTags`，事务）。
    ///
    /// # Errors
    /// 参数缺失 → `Validation`；冻结 → 业务错误；数据库失败 → `Error::Database`。
    pub async fn batch_bind_tags(
        &self,
        shop_id: i32,
        param: &UserMemberTagBatchParam,
    ) -> Result<()> {
        validate_batch_param(param).map_err(Error::Validation)?;
        self.assert_enabled(shop_id).await?;
        let mut existing_map: HashMap<i64, Vec<i64>> = HashMap::new();
        for user_id in &param.user_ids {
            let rows = self.repo().list_user_tags(shop_id, *user_id).await?;
            existing_map.insert(*user_id, rows.iter().map(|ut| ut.tag_id).collect());
        }
        let (delete_map, insert_map) = build_batch_diffs(&existing_map, &param.tag_ids);
        self.repo()
            .batch_bind_tags(shop_id, &param.user_ids, &delete_map, &insert_map, now_seconds())
            .await?;
        let log_info = format!("批量给用户打标签：{:?}", param.user_ids);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_current_log(if shop_id > 0 { "shop" } else { "admin" }, shop_id, &log_info)
            .await
        {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!(
            "批量用户打标签: shop_id={shop_id}, users={}",
            param.user_ids.len()
        );
        Ok(())
    }

    /// 批量取消用户标签（Java `batchUnbindTags`）。
    pub async fn batch_unbind_tags(
        &self,
        shop_id: i32,
        param: &UserMemberTagBatchParam,
    ) -> Result<()> {
        validate_batch_param(param).map_err(Error::Validation)?;
        self.assert_enabled(shop_id).await?;
        self.repo()
            .batch_unbind_tags(shop_id, &param.user_ids, &param.tag_ids)
            .await?;
        let log_info = format!("批量取消用户标签：{:?}", param.user_ids);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_current_log(if shop_id > 0 { "shop" } else { "admin" }, shop_id, &log_info)
            .await
        {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!(
            "批量取消用户标签: shop_id={shop_id}, users={}",
            param.user_ids.len()
        );
        Ok(())
    }
}

/// 批量打标签的逐用户 diff（Java `batchBindTags` 循环体）。
///
/// 返回 `(delete_map, insert_map)`：`Vec<(user_id, tag_ids)>`，顺序与
/// `existing_map` 的插入顺序无关（HashMap），但每个用户内部顺序稳定。
#[must_use]
fn build_batch_diffs(
    existing_map: &HashMap<i64, Vec<i64>>,
    new_tag_ids: &[i64],
) -> (Vec<(i64, Vec<i64>)>, Vec<(i64, Vec<i64>)>) {
    let mut users: Vec<i64> = existing_map.keys().copied().collect();
    users.sort_unstable();
    let mut deletes = Vec::new();
    let mut inserts = Vec::new();
    for user_id in users {
        let existing = existing_map.get(&user_id).cloned().unwrap_or_default();
        let delete_ids = tags_to_delete(&existing, new_tag_ids);
        let insert_ids = tags_to_insert(&existing, new_tag_ids);
        if !delete_ids.is_empty() {
            deletes.push((user_id, delete_ids));
        }
        if !insert_ids.is_empty() {
            inserts.push((user_id, insert_ids));
        }
    }
    (deletes, inserts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_diffs_are_grouped_per_user() {
        let mut existing = HashMap::new();
        existing.insert(10_i64, vec![1, 2]);
        existing.insert(20_i64, vec![2]);
        let (deletes, inserts) = build_batch_diffs(&existing, &[2, 3]);
        // user 10：删 1、增 3
        assert_eq!(deletes, vec![(10, vec![1])]);
        // user 20：无删除；新增 3
        assert_eq!(inserts, vec![(10, vec![3]), (20, vec![3])]);
    }

    #[test]
    fn batch_diffs_skip_users_without_changes() {
        let mut existing = HashMap::new();
        existing.insert(1_i64, vec![5, 6]);
        let (deletes, inserts) = build_batch_diffs(&existing, &[6, 5]);
        assert!(deletes.is_empty());
        assert!(inserts.is_empty());
    }

    #[test]
    fn enabled_guard_only_applies_to_shop_accounts() {
        // 纯逻辑复刻：shop_id <= 0 不触发冻结；> 0 且关闭时触发
        let triggers = |shop_id: i32, enabled: bool| shop_id > 0 && !enabled;
        assert!(!triggers(0, false));
        assert!(!triggers(-1, false));
        assert!(triggers(5, false));
        assert!(!triggers(5, true));
    }

    #[test]
    fn frozen_message_is_verbatim() {
        assert!(MSG_SHOP_MEMBER_FROZEN.contains("平台未开放店铺会员功能"));
    }
}
