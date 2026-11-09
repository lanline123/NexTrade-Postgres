//! 客服人员管理 域服务（im::admin_user_kefu）。
//!
//! 构造签名固定 `pub fn new(repos: Arc<Repositories>) -> Self`；仓储经嵌套
//! 聚合根访问：`self.repos.im.admin_user_kefu`。
//!
//! 对应 Java `AdminUserKefuServiceImpl`
//! （`java/nextrade-service/.../im/impl/AdminUserKefuServiceImpl.java`，22KB）。
//! 客服人员 = `admin_user_shop` 绑定 + `admin_user` 账号 + `kefu_agent` 角色。
//!
//! ## 已登记偏差
//!
//! 1. **客服候选角色策略**（`ShopEmployeeWorkbenchCandidatePolicy` 的
//!    `kefuWorkbenchAuthSn` / 授权签名）未迁移：本实现按
//!    `admin_role.role_code='kefu_agent'` 识别客服角色，绑定侧允许 `role_id IS NULL`
//!    的历史数据，与 Java 的候选策略存在差异。
//! 2. **联动 `im_servant` 在线状态**（`shopEmployeeRoleLifecycle.removeKefu`）未迁移：
//!    删除客服只删绑定，不清理 IM 在线状态记录。
//! 3. **操作日志**（`adminLogService.createByLogInfo`）未迁移。
//! 4. 新建管理员头像：Java 用随机数 1..34，这里按手机号字节和取模（确定性），
//!    避免引入随机源。
//! 5. 权限矩阵未实现：仅 `require_auth`，权限码记录在路由表。

use std::sync::Arc;

use nextrade_common::utils;
use nextrade_core::models::im::admin_user_kefu::{
    format_time, AdminUserKefuCreateDTO, AdminUserKefuPageQuery, AdminUserKefuRow,
    AdminUserKefuUpdateDTO, AdminUserKefuVO, KefuBatchDTO, KefuRoleSimpleVO, KefuUpdateFieldDTO,
    MSG_BINDING_NOT_EXIST, MSG_KEFU_ROLE_NOT_EXIST, MSG_MOBILE_EXIST, MSG_NOT_DELETE_SELF,
    MSG_USERNAME_EXIST,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::im::admin_user_kefu::AdminUserKefuRepository;
use nextrade_infrastructure::db::Repositories;

/// 客服人员管理 域服务。
pub struct AdminUserKefuService {
    repos: Arc<Repositories>,
}

impl AdminUserKefuService {
    /// 构造服务（组合根唯一装配点）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本 feature 仓储。
    #[must_use]
    pub fn repo(&self) -> &AdminUserKefuRepository {
        &self.repos.im.admin_user_kefu
    }

    /// 客服人员分页（Java `listUser`）。
    pub async fn list(
        &self,
        shop_id: i32,
        q: &AdminUserKefuPageQuery,
    ) -> Result<PageResult<AdminUserKefuVO>> {
        let page = q.page_query();
        let (total, rows) = self.repo().paginate(shop_id, q).await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        Ok(PageResult::new(total, rows.into_iter().map(to_vo).collect(), &page))
    }

    /// 客服详情（Java `detail`：不存在返回 null，不报错）。
    pub async fn detail(&self, id: i32, shop_id: i32) -> Result<Option<AdminUserKefuVO>> {
        Ok(self.repo().find_binding(id, shop_id).await?.map(to_vo))
    }

    /// 客服信息（Java `info`：按 adminId 查，不存在抛「店铺员工信息不存在」）。
    pub async fn info(&self, admin_id: i32, shop_id: i32) -> Result<AdminUserKefuVO> {
        self.repo()
            .find_binding_by_admin(admin_id, shop_id)
            .await?
            .map(to_vo)
            .ok_or_else(|| Error::NotFound(MSG_BINDING_NOT_EXIST.into()))
    }

    /// 客服角色列表（Java `roles`：无客服角色时返回空数组）。
    pub async fn roles(&self, shop_id: i32) -> Result<Vec<KefuRoleSimpleVO>> {
        Ok(self
            .repo()
            .find_kefu_role(shop_id)
            .await?
            .map(|r| {
                vec![KefuRoleSimpleVO {
                    role_id: r.role_id,
                    role_name: r.role_name,
                    role_code: r.role_code,
                    shop_id: r.shop_id,
                }]
            })
            .unwrap_or_default())
    }

    /// 创建客服（Java `createAdminUserKefu`）。
    pub async fn create(&self, shop_id: i32, dto: &AdminUserKefuCreateDTO) -> Result<()> {
        let role = self
            .repo()
            .find_kefu_role(shop_id)
            .await?
            .ok_or_else(|| Error::Validation(MSG_KEFU_ROLE_NOT_EXIST.into()))?;
        let username = dto.username.trim();
        let mobile = dto.mobile.trim();
        let existing = self.repo().find_admin_by_mobile(mobile).await?;
        let exclude = existing.as_ref().map(|a| a.admin_id);
        if self.repo().count_username(username, exclude).await? > 0 {
            return Err(Error::Conflict(MSG_USERNAME_EXIST.into()));
        }
        if let Some(admin) = existing {
            // 复用已有管理员（Java 按手机号命中则复用 adminId）。
            if self.repo().find_binding_by_admin(admin.admin_id, shop_id).await?.is_some() {
                return Err(Error::Conflict("该管理员已是客服人员".into()));
            }
            let avatar = dto.avatar.clone().unwrap_or_else(|| {
                if admin.avatar.is_empty() {
                    default_avatar(mobile)
                } else {
                    admin.avatar.clone()
                }
            });
            let email = dto
                .email
                .clone()
                .unwrap_or_else(|| admin.email.clone());
            let is_using = dto.is_using.unwrap_or(1);
            let now = now_secs();
            self.repo()
                .update_admin(
                    admin.admin_id, username, mobile, &email, &avatar, is_using,
                    Some(role.role_id),
                )
                .await?;
            return self
                .repo()
                .insert_binding(
                    admin.admin_id,
                    admin.user_id,
                    shop_id,
                    username,
                    &email,
                    &avatar,
                    is_using,
                    Some(role.role_id),
                    now,
                )
                .await;
        }
        let avatar = dto
            .avatar
            .clone()
            .unwrap_or_else(|| default_avatar(mobile));
        let is_using = dto.is_using.unwrap_or(1);
        let now = now_secs();
        let email = dto.email.clone().unwrap_or_default();
        let admin_id = i32::try_from(
            self.repo()
                .insert_admin(
                    username,
                    mobile,
                    &email,
                    &avatar,
                    Some(role.role_id),
                    shop_id,
                    is_using,
                    now,
                )
                .await?,
        )
        .unwrap_or(0);
        self.repo()
            .insert_binding(
                admin_id, 0, shop_id, username, &email, &avatar, is_using,
                Some(role.role_id), now,
            )
            .await
    }

    /// 更新客服（Java `updateAdminUserKefu`）。
    pub async fn update(&self, shop_id: i32, dto: &AdminUserKefuUpdateDTO) -> Result<()> {
        let binding = self
            .repo()
            .find_binding(dto.id, shop_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_BINDING_NOT_EXIST.into()))?;
        let role = self
            .repo()
            .find_kefu_role(shop_id)
            .await?
            .ok_or_else(|| Error::Validation(MSG_KEFU_ROLE_NOT_EXIST.into()))?;
        let admin_id = binding.admin_id;
        let username = dto.username.trim();
        let mobile = dto.mobile.trim();
        if self.repo().count_username(username, Some(admin_id)).await? > 0 {
            return Err(Error::Conflict(MSG_USERNAME_EXIST.into()));
        }
        if self.repo().count_mobile(mobile, Some(admin_id)).await? > 0 {
            return Err(Error::Conflict(MSG_MOBILE_EXIST.into()));
        }
        let avatar = dto
            .avatar
            .clone()
            .unwrap_or_else(|| binding.avatar.clone());
        let email = dto.email.clone().unwrap_or_else(|| binding.email.clone());
        let is_using = dto.is_using.unwrap_or(binding.is_using);
        self.repo()
            .update_admin(
                admin_id, username, mobile, &email, &avatar, is_using,
                Some(role.role_id),
            )
            .await?;
        self.repo()
            .update_binding(
                dto.id, shop_id, username, &email, &avatar, is_using,
                Some(role.role_id),
            )
            .await?;
        Ok(())
    }

    /// 删除客服（Java `delKefu`：禁止删除自己）。
    pub async fn del(&self, id: i32, current_admin_id: i32, shop_id: i32) -> Result<()> {
        let binding = self
            .repo()
            .find_binding(id, shop_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_BINDING_NOT_EXIST.into()))?;
        if binding.admin_id == current_admin_id {
            return Err(Error::Forbidden(MSG_NOT_DELETE_SELF.into()));
        }
        self.repo().delete_binding(id, shop_id).await?;
        let log_info = format!("删除客服人员信息：{}", binding.username);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_current_log(if shop_id > 0 { "shop" } else { "admin" }, shop_id, &log_info)
            .await
        {
            tracing::warn!("写管理操作日志失败（删除客服）: {e}");
        }
        Ok(())
    }

    /// 批量删除（Java `batch`：仅 `type='del'` 生效；禁止删除自己）。
    pub async fn batch(&self, shop_id: i32, current_admin_id: i32, dto: &KefuBatchDTO) -> Result<()> {
        if dto.type_.as_deref() != Some("del") {
            // Java `batch` 对非 del 直接 `return false`；HTTP 层 void，故无操作。
            return Ok(());
        }
        for id in &dto.ids {
            let binding = self
                .repo()
                .find_binding(*id, shop_id)
                .await?
                .ok_or_else(|| Error::NotFound(MSG_BINDING_NOT_EXIST.into()))?;
            if binding.admin_id == current_admin_id {
                return Err(Error::Forbidden(MSG_NOT_DELETE_SELF.into()));
            }
        }
        self.repo().delete_bindings(&dto.ids, shop_id).await?;
        Ok(())
    }

    /// 更新字段（Java `updateUsing`：本 feature 只允许 `isUsing`）。
    pub async fn update_field(&self, shop_id: i32, dto: &KefuUpdateFieldDTO) -> Result<()> {
        if dto.field != "isUsing" {
            return Err(Error::Validation("仅支持更新启用状态字段 isUsing".into()));
        }
        let val = match dto.val.as_str() {
            "0" => 0,
            "1" => 1,
            _ => return Err(Error::Validation("无效的状态值".into())),
        };
        let binding = self
            .repo()
            .find_binding(dto.id, shop_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_BINDING_NOT_EXIST.into()))?;
        self.repo()
            .update_binding(
                dto.id,
                shop_id,
                &binding.username,
                &binding.email,
                &binding.avatar,
                val,
                binding.role_id,
            )
            .await?;
        Ok(())
    }
}

/// 行 → VO。
#[must_use]
pub fn to_vo(row: AdminUserKefuRow) -> AdminUserKefuVO {
    AdminUserKefuVO {
        id: row.id,
        admin_id: row.admin_id,
        shop_id: row.shop_id,
        username: row.username,
        email: row.email,
        avatar: row.avatar,
        is_using: row.is_using,
        role_id: row.role_id,
        auth_list: parse_auth_list(row.auth_list.as_deref()),
        add_time: format_time(row.add_time),
        role_name: row.role_name,
    }
}

/// `auth_list` JSON → `Vec<String>`；非法/空 → 空数组。
#[must_use]
pub fn parse_auth_list(raw: Option<&str>) -> Vec<String> {
    raw.and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .unwrap_or_default()
}

/// 默认头像（Java：`../assets/avatar/{1..34}.jpeg`）。
#[must_use]
pub fn default_avatar(mobile: &str) -> String {
    let sum: i64 = mobile.bytes().map(i64::from).sum();
    let n = sum.rem_euclid(34) + 1;
    format!("../assets/avatar/{n}.jpeg")
}

/// 当前时间（秒级时间戳）。
fn now_secs() -> i32 {
    i32::try_from(utils::now_millis() / 1000).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::im::admin_user_kefu::KEFU_AGENT_ROLE_CODE;

    fn row() -> AdminUserKefuRow {
        AdminUserKefuRow {
            id: 5,
            admin_id: 9,
            shop_id: 3,
            username: "kefu01".into(),
            email: "k@x.com".into(),
            avatar: "../assets/avatar/2.jpeg".into(),
            is_using: 1,
            role_id: Some(7),
            auth_list: Some("[\"all\"]".into()),
            add_time: 1_733_000_000,
            role_name: Some("客服".into()),
        }
    }

    #[test]
    fn to_vo_parses_auth_list_and_time() {
        let vo = to_vo(row());
        assert_eq!(vo.auth_list, vec!["all".to_string()]);
        assert_eq!(vo.add_time.len(), 19);
        assert_eq!(vo.role_name.as_deref(), Some("客服"));
        let j = serde_json::to_value(&vo).unwrap();
        assert_eq!(j["adminId"], 9);
        assert_eq!(j["authList"][0], "all");
    }

    #[test]
    fn invalid_auth_list_is_empty_vec() {
        assert!(parse_auth_list(None).is_empty());
        assert!(parse_auth_list(Some("not-json")).is_empty());
        assert_eq!(parse_auth_list(Some("[]")), Vec::<String>::new());
    }

    #[test]
    fn default_avatar_in_java_range() {
        for m in ["13800000000", "18812345678", ""] {
            let a = default_avatar(m);
            let n: i32 = a
                .trim_start_matches("../assets/avatar/")
                .trim_end_matches(".jpeg")
                .parse()
                .unwrap();
            assert!((1..=34).contains(&n), "avatar index {n}");
        }
        assert_eq!(KEFU_AGENT_ROLE_CODE, "kefu_agent");
    }
}
