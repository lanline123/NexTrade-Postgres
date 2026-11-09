//! 店铺员工域服务（merchant::admin_user_shop）。
//!
//! 构造签名 `pub fn new(repos: Arc<Repositories>) -> Self`；仓储访问为**嵌套**命名：
//! `self.repos.merchant.admin_user_shop.<method>()`。
//!
//! 对应原 Java `com.nextrade.service.shop.impl.AdminUserShopImpl`。
//!
//! # 已实现
//! `list`/`detail`/`info`/`checkIsAdmin`/`create`/`update`/`del`/`batch`/`modifyUser`，
//! 含 Java 校验顺序与逐字文案：当前店铺归属校验（「店铺员工信息不存在」）、
//! 不能删除自己（「不能删除自己」）、用户名/手机号唯一（「管理员名称已存在或被商户账号占用」/
//! 「管理员手机号已经存在」）、已绑定其它店铺不可改手机号、员工上限
//! （「店铺员工数已达上限,上限为N个,如需修改,请前往配置」）、重复绑定
//! （「此店铺已存在此员工」）。
//!
//! # 未迁移 / 简化（均为有意降级，Java 原行为见注释）
//! - 当前店铺来自 `x-shop-id` 请求头；Rust 侧 JWT 未签发 shop_id，用查询参数 `shopId` 兜底，
//!   缺省时 `list` 返回空页（与 Java shopId==null 时一致），其余接口报「店铺员工信息不存在」。
//! - `ShopEmployeeRolePolicy#requireAssignableRole` 简化为「角色必须存在」；
//! - `merchant_user` 关联写入、客服生命周期（`ShopEmployeeRoleLifecycle`）、员工操作日志
//!   （`adminLogService`）未迁移；
//! - 新建后台账号的初始密码：Java 使用常量 `PASSWORD` 的 bcrypt 哈希；本实现用
//!   `nextrade_security::hash_password("123456")`（默认初始密码口径，见报告）。
//! - `authList` 输出按 Java `handleRes`：角色权限优先，其次员工自身 auth_list。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::models::merchant::admin_user_shop::{
    default_avatar, format_time, is_admin_flag, resolve_auth_list, staff_limit_message,
    AdminUserShopCreateDTO, AdminUserShopModifyDTO, AdminUserShopPageQuery, AdminUserShopUpdateDTO,
    AdminUserShopVO, ADMIN_USER_SHOP_ALREADY_EXIST, ADMIN_USER_SHOP_NOT_DELETE_SELF,
    ADMIN_USER_SHOP_NOT_EXIST, MOBILE_IS_EXIST, MOBILE_LOCKED_BY_OTHER_SHOP, ROLE_NOT_ASSIGNABLE,
    SERVICE_DATA_ERROR, SHOP_NOT_EXIST, USERNAME_IS_EXIST, USER_NOT_EXIST,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::merchant::admin_user_shop::AdminUserShopRepository;
use nextrade_infrastructure::db::Repositories;
use nextrade_security::hash_password;

/// 默认初始密码（Java `PASSWORD` 常量口径；实际值见报告 deviations）。
pub const DEFAULT_PASSWORD: &str = "123456";
/// 员工上限配置 `SettingsEnum.MAX_SUB_ADMINISTRATOR` 的 bizCode。
pub const MAX_SUB_ADMINISTRATOR: &str = "maxSubAdministrator";

/// 店铺员工域服务。
pub struct AdminUserShopService {
    repos: Arc<Repositories>,
}

impl AdminUserShopService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    #[must_use]
    pub fn repo(&self) -> &AdminUserShopRepository {
        &self.repos.merchant.admin_user_shop
    }

    /// 员工分页（Java `list`）。
    pub async fn list(
        &self,
        q: &AdminUserShopPageQuery,
        shop_id: Option<i32>,
    ) -> Result<PageResult<AdminUserShopVO>> {
        let (page, size) = q.page_params();
        let pq = PageQuery {
            page,
            page_size: size,
        };
        let Some(shop_id) = shop_id.filter(|s| *s > 0) else {
            // Java：shopId == null → 返回空 Page
            return Ok(PageResult::empty(&pq));
        };
        let (total, rows) = self.repo().paginate(shop_id, q).await?;
        let user_ids: Vec<i32> = rows.iter().map(|r| r.user_id).collect();
        let admin_ids: Vec<i32> = rows.iter().map(|r| r.admin_id).collect();
        let role_ids: Vec<i32> = rows.iter().map(|r| r.role_id).collect();
        let users = self.repo().find_users(&user_ids).await?;
        let admins = self.repo().find_admin_users(&admin_ids).await?;
        let roles = self.repo().find_roles(&role_ids).await?;
        let user_map: std::collections::HashMap<i32, serde_json::Value> = users
            .into_iter()
            .map(|(id, username, nickname, mobile, avatar)| {
                (
                    id,
                    serde_json::json!({
                        "userId": id, "username": username, "nickname": nickname,
                        "mobile": mobile, "avatar": avatar,
                    }),
                )
            })
            .collect();
        let admin_map: std::collections::HashMap<i32, serde_json::Value> = admins
            .into_iter()
            .map(|(id, username, mobile, email, avatar, merchant_id)| {
                (
                    id,
                    serde_json::json!({
                        "adminId": id, "username": username, "mobile": mobile,
                        "email": email, "avatar": avatar, "merchantId": merchant_id,
                    }),
                )
            })
            .collect();
        let role_map: std::collections::HashMap<i32, (String, Option<String>)> = roles
            .into_iter()
            .map(|(id, name, auth)| (id, (name, auth)))
            .collect();

        let mut list = Vec::with_capacity(rows.len());
        for row in &rows {
            let role = role_map.get(&row.role_id);
            let auth = resolve_auth_list(
                row.role_id,
                role.and_then(|(_, a)| a.as_deref()),
                row.auth_list.as_deref(),
            );
            list.push(AdminUserShopVO {
                id: row.id,
                admin_id: row.admin_id,
                user_id: row.user_id,
                shop_id: row.shop_id,
                username: row.username.clone(),
                email: row.email.clone(),
                avatar: row.avatar.clone(),
                is_using: row.is_using,
                is_admin: row.is_admin,
                role_id: row.role_id,
                auth_list: auth,
                add_time: format_time(row.add_time),
                user: user_map.get(&row.user_id).cloned(),
                admin_user: admin_map.get(&row.admin_id).cloned(),
                role: role.map(|(name, _)| {
                    serde_json::json!({ "roleId": row.role_id, "roleName": name })
                }),
            });
        }
        Ok(PageResult::new(total, list, &pq))
    }

    /// 员工详情（Java `detail`：当前店铺归属 + user/adminUser 装配）。
    pub async fn detail(
        &self,
        id: Option<i32>,
        shop_id: Option<i32>,
    ) -> Result<AdminUserShopVO> {
        let binding = self.require_binding(id, shop_id).await?;
        Ok(self.assemble(&binding).await?)
    }

    /// 员工信息（Java `info`：按 `adminId + shopId` 查）。
    pub async fn info(
        &self,
        admin_id: Option<i32>,
        shop_id: Option<i32>,
    ) -> Result<AdminUserShopVO> {
        let admin_id = admin_id.ok_or_else(|| biz(ADMIN_USER_SHOP_NOT_EXIST))?;
        let shop_id = shop_id.ok_or_else(|| biz(ADMIN_USER_SHOP_NOT_EXIST))?;
        let binding = self
            .repo()
            .find_by_admin_shop(admin_id, shop_id)
            .await?
            .ok_or_else(|| biz(ADMIN_USER_SHOP_NOT_EXIST))?;
        Ok(self.assemble(&binding).await?)
    }

    /// 是否当前店铺主账号（Java `checkIsAdmin`：异常时返回 false）。
    pub async fn check_is_admin(&self, admin_id: i32, shop_id: Option<i32>) -> Result<bool> {
        let Ok(vo) = self.info(Some(admin_id), shop_id).await else {
            return Ok(false);
        };
        Ok(is_admin_flag(vo.is_admin))
    }

    /// 创建员工（Java `createAdminUserShop`）。
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        dto: &AdminUserShopCreateDTO,
        shop_id: Option<i32>,
    ) -> Result<i32> {
        let shop_id = shop_id
            .filter(|s| *s > 0)
            .ok_or_else(|| biz(SHOP_NOT_EXIST))?;
        let mobile = dto.mobile.as_deref().unwrap_or("").trim().to_string();
        if mobile.is_empty() {
            return Err(biz(USER_NOT_EXIST));
        }
        let email = dto.email.clone().unwrap_or_default();
        // 角色必须存在（Java rolePolicy.requireAssignableRole 的简化）
        let role_id = dto.role_id.unwrap_or(0);
        let role = self
            .repo()
            .find_role(role_id)
            .await?
            .ok_or_else(|| biz(ROLE_NOT_ASSIGNABLE))?;
        let merchant_id = self
            .repo()
            .shop_merchant_and_type(shop_id)
            .await?
            .map(|(m, _)| m)
            .ok_or_else(|| biz(SHOP_NOT_EXIST))?;
        let username = dto
            .username
            .clone()
            .filter(|u| !u.trim().is_empty())
            .unwrap_or_else(|| mobile.clone());
        // 用户名唯一（Java：eq(username) 且 ne(mobile)）
        if self
            .repo()
            .count_username_conflict(&username, &mobile)
            .await?
            > 0
        {
            return Err(biz(USERNAME_IS_EXIST));
        }
        // 员工数上限（Java：排除收银员角色的员工数 >= 配置上限）
        let cashier_role_ids = self.repo().pos_cashier_role_ids().await?;
        let staff_count = self.repo().count_staff(shop_id, &cashier_role_ids).await?;
        let limit = self
            .repo()
            .config_value(MAX_SUB_ADMINISTRATOR)
            .await?
            .and_then(|v| v.trim().parse::<i64>().ok())
            .unwrap_or(0);
        if staff_count >= limit {
            return Err(biz(&staff_limit_message(limit)));
        }
        // 头像缺省
        let avatar = dto
            .avatar
            .clone()
            .filter(|a| !a.is_empty())
            .unwrap_or_else(|| default_avatar(now_seconds() as u32));
        let now = i32::try_from(now_seconds()).unwrap_or(0);
        // 管理员账号：按手机号复用，否则创建（Java 同逻辑）
        let (admin_id, user_id) = match self.repo().admin_user_by_mobile(&mobile).await? {
            Some((admin_id, user_id, _)) => (admin_id, user_id),
            None => {
                let password = hash_password(DEFAULT_PASSWORD)?;
                let admin_id = self
                    .repo()
                    .insert_admin_user(
                        &username,
                        &mobile,
                        &email,
                        &admin_type_by_shop_type(
                            self.repo()
                                .shop_merchant_and_type(shop_id)
                                .await?
                                .and_then(|(_, t)| t),
                        ),
                        merchant_id,
                        now,
                        &password,
                    )
                    .await?;
                (admin_id, 0)
            }
        };
        if self.repo().count_by_admin_shop(admin_id, shop_id).await? > 0 {
            return Err(biz(ADMIN_USER_SHOP_ALREADY_EXIST));
        }
        let _ = role;
        self.repo()
            .insert_binding(
                admin_id,
                user_id,
                shop_id,
                &username,
                &email,
                &avatar,
                dto.is_using.unwrap_or(0),
                0,
                role_id,
                now,
            )
            .await
    }

    /// 更新员工（Java `updateAdminUserShop`）。
    pub async fn update(
        &self,
        dto: &AdminUserShopUpdateDTO,
        shop_id: Option<i32>,
    ) -> Result<()> {
        let binding = self.require_binding(dto.id, shop_id).await?;
        let current_shop_id = binding.shop_id;
        let admin_id = binding.admin_id;
        let role_id = dto.role_id.unwrap_or(binding.role_id);
        if self.repo().find_role(role_id).await?.is_none() {
            return Err(biz(ROLE_NOT_ASSIGNABLE));
        }
        let username = dto
            .username
            .clone()
            .unwrap_or_else(|| binding.username.clone());
        let mobile = dto.mobile.clone().unwrap_or_default();
        if !mobile.trim().is_empty() {
            if self
                .repo()
                .count_username_conflict_except(&username, admin_id)
                .await?
                > 0
            {
                return Err(biz(USERNAME_IS_EXIST));
            }
            if self
                .repo()
                .count_mobile_conflict_except(&mobile, admin_id)
                .await?
                > 0
            {
                return Err(biz(MOBILE_IS_EXIST));
            }
            let admin_user = self
                .repo()
                .find_admin_users(&[admin_id])
                .await?
                .into_iter()
                .next()
                .ok_or_else(|| biz(USER_NOT_EXIST))?;
            if admin_user.2 != mobile {
                if self
                    .repo()
                    .count_other_shop_bindings(admin_id, current_shop_id)
                    .await?
                    > 0
                {
                    return Err(biz(MOBILE_LOCKED_BY_OTHER_SHOP));
                }
                self.repo()
                    .update_admin_user_mobile_email(
                        admin_id,
                        &mobile,
                        dto.email.as_deref().unwrap_or(&admin_user.3),
                    )
                    .await?;
            }
        }
        let avatar = dto
            .avatar
            .clone()
            .filter(|a| !a.is_empty())
            .unwrap_or_else(|| default_avatar(now_seconds() as u32));
        self.repo()
            .update_binding(
                binding.id,
                &username,
                dto.email.as_deref().unwrap_or(&binding.email),
                &avatar,
                dto.is_using.unwrap_or(binding.is_using),
                role_id,
            )
            .await?;
        Ok(())
    }

    /// 删除员工（Java `delShop`：不能删除自己）。
    pub async fn del(
        &self,
        id: Option<i32>,
        actor_admin_id: i32,
        shop_id: Option<i32>,
    ) -> Result<()> {
        let binding = self.require_binding(id, shop_id).await?;
        if binding.admin_id == actor_admin_id {
            return Err(biz(ADMIN_USER_SHOP_NOT_DELETE_SELF));
        }
        self.repo().delete(binding.id).await?;
        let log_info = format!("删除员工信息：{}", binding.username);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        let sid = binding.shop_id;
        if let Err(e) = admin_log
            .record_current_log(if sid > 0 { "shop" } else { "admin" }, sid, &log_info)
            .await
        {
            tracing::warn!("写管理操作日志失败（删除员工）: {e}");
        }
        Ok(())
    }

    /// 批量删除（Java `batch`：仅 `type=del`，逐个校验归属与「不能删除自己」）。
    pub async fn batch(
        &self,
        ids: Option<&[i32]>,
        op_type: Option<&str>,
        actor_admin_id: i32,
        shop_id: Option<i32>,
    ) -> Result<()> {
        if op_type != Some("del") {
            return Ok(());
        }
        let ids = ids.unwrap_or(&[]);
        let shop_id = shop_id.ok_or_else(|| biz(ADMIN_USER_SHOP_NOT_EXIST))?;
        let mut targets = Vec::with_capacity(ids.len());
        let mut target_names = Vec::with_capacity(ids.len());
        for id in ids {
            let binding = self.repo().find_scoped(*id, shop_id).await?;
            let binding = binding.ok_or_else(|| biz(ADMIN_USER_SHOP_NOT_EXIST))?;
            if binding.admin_id == actor_admin_id {
                return Err(biz(ADMIN_USER_SHOP_NOT_DELETE_SELF));
            }
            targets.push(binding.id);
            target_names.push(binding.username);
        }
        self.repo().delete_many(&targets).await?;
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        for name in target_names {
            let log_info = format!("删除员工信息：{name}");
            if let Err(e) = admin_log
                .record_current_log("shop", shop_id, &log_info)
                .await
            {
                tracing::warn!("写管理操作日志失败（批量删除员工）: {e}");
            }
        }
        Ok(())
    }

    /// 修改员工头像/邮箱（Java `modifyUser`）。
    pub async fn modify_user(
        &self,
        dto: &AdminUserShopModifyDTO,
        shop_id: Option<i32>,
    ) -> Result<()> {
        let binding = self.require_binding(dto.id, shop_id).await?;
        let avatar = dto
            .avatar
            .clone()
            .filter(|a| !a.is_empty())
            .unwrap_or_else(|| default_avatar(now_seconds() as u32));
        self.repo()
            .update_avatar_email(
                binding.id,
                &avatar,
                dto.email.as_deref().unwrap_or(&binding.email),
            )
            .await?;
        Ok(())
    }

    /// Java `requireCurrentShopBinding`：id/shopId 缺失或跨店 → 「店铺员工信息不存在」。
    async fn require_binding(
        &self,
        id: Option<i32>,
        shop_id: Option<i32>,
    ) -> Result<nextrade_core::entities::AdminUserShop> {
        let Some(id) = id else {
            return Err(biz(ADMIN_USER_SHOP_NOT_EXIST));
        };
        let Some(shop_id) = shop_id else {
            return Err(biz(ADMIN_USER_SHOP_NOT_EXIST));
        };
        self.repo()
            .find_scoped(id, shop_id)
            .await?
            .ok_or_else(|| biz(ADMIN_USER_SHOP_NOT_EXIST))
    }

    /// 组装详情 VO（user + adminUser + role + authList）。
    async fn assemble(
        &self,
        binding: &nextrade_core::entities::AdminUserShop,
    ) -> Result<AdminUserShopVO> {
        let users = self.repo().find_users(&[binding.user_id]).await?;
        let admins = self.repo().find_admin_users(&[binding.admin_id]).await?;
        let role = self.repo().find_role(binding.role_id).await?;
        let user = users.first().map(|(id, username, nickname, mobile, avatar)| {
            serde_json::json!({
                "userId": id, "username": username, "nickname": nickname,
                "mobile": mobile, "avatar": avatar,
            })
        });
        let admin_user = admins.first().map(|(id, username, mobile, email, avatar, merchant_id)| {
            serde_json::json!({
                "adminId": id, "username": username, "mobile": mobile,
                "email": email, "avatar": avatar, "merchantId": merchant_id,
            })
        });
        let auth_list = resolve_auth_list(
            binding.role_id,
            role.as_ref().and_then(|(_, _, a)| a.as_deref()),
            binding.auth_list.as_deref(),
        );
        Ok(AdminUserShopVO {
            id: binding.id,
            admin_id: binding.admin_id,
            user_id: binding.user_id,
            shop_id: binding.shop_id,
            username: binding.username.clone(),
            email: binding.email.clone(),
            avatar: binding.avatar.clone(),
            is_using: binding.is_using,
            is_admin: binding.is_admin,
            role_id: binding.role_id,
            auth_list,
            add_time: format_time(binding.add_time),
            user,
            admin_user,
            role: role.map(|(id, name, _)| serde_json::json!({ "roleId": id, "roleName": name })),
        })
    }
}

/// 店铺类型 → 后台账号类型（Java `resolveAdminTypeByShopId`：
/// 1店铺；2门店→store；3自提点→pickup）。
#[must_use]
pub fn admin_type_by_shop_type(shop_type: Option<i32>) -> String {
    match shop_type {
        Some(2) => "store".to_string(),
        Some(3) => "pickup".to_string(),
        _ => "shop".to_string(),
    }
}

fn biz(msg: &str) -> Error {
    Error::biz(SERVICE_DATA_ERROR, msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_type_mapping_matches_java_resolve_by_shop_type() {
        assert_eq!(admin_type_by_shop_type(Some(1)), "shop");
        assert_eq!(admin_type_by_shop_type(Some(2)), "store");
        assert_eq!(admin_type_by_shop_type(Some(3)), "pickup");
        assert_eq!(admin_type_by_shop_type(None), "shop");
        assert_eq!(admin_type_by_shop_type(Some(9)), "shop");
    }

    #[test]
    fn staff_limit_and_role_defaults() {
        assert_eq!(
            staff_limit_message(10),
            "店铺员工数已达上限,上限为10个,如需修改,请前往配置"
        );
        assert_eq!(DEFAULT_PASSWORD, "123456");
        assert_eq!(MAX_SUB_ADMINISTRATOR, "maxSubAdministrator");
    }
}
