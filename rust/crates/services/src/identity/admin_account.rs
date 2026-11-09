//! 账号管理域服务：主账号查询 / 店铺宿主分页 / 绑定 / 改资料 / 改密码 /
//! 账号分页 / 删除。
//!
//! 对应原 Java `com.nextrade.service.account.impl.AdminAccountServiceImpl`（451 行）
//! 与管理端 `AdminAccountController` 的 7 个端点。
//!
//! # 迁移要点（逐条对照 Java 源）
//!
//! - `getMainAccount`：`adminId > 0` 优先直接查账号；否则要求 `id` + `type`
//!   非空，`type ∈ {shop, store, pickup}`，按 `admin_user_shop(shop_id, is_admin=1)`
//!   找主账号；查不到抛「未找到对应的主账号信息」。
//! - `pageShopOrVendor`：`adminId` 为空 / `type` 空白 ⇒ 返回**空分页**
//!   （Java `return new Page<>()`，current=1 / size=10）；类型非
//!   `shop|store|pickup` ⇒ 「类型参数错误，只支持shop或vendor」。
//! - `bindMainAccount`（Java `@Transactional`）：
//!   1. `getAdmin`：`adminData.type == 1`（会员）→ 查 `user`，手机号不能为空；
//!      按手机号复用已有管理员，没有则新建（默认密码 `admin123321`、
//!      `auth_list = "[]"`、`role_id = 1`、随机默认头像）；
//!      `adminData.type != 1`（管理员）→ 按 `adminData.adminId` 取管理员；
//!   2. 查「该管理员 + 该店铺」旧绑定；
//!   3. 查店铺主账号：没有 ⇒ 恢复/新建主账号（`shop` 不存在报「店铺不存在」，
//!      `admin_role` 系统内置角色按店型初始化）；有 ⇒ 把主账号改绑到当前管理员，
//!      并把旧绑定降级为普通员工。
//!   全部写入在同一事务内（等价 Java `@Transactional`）。
//! - `updateMainAccount`：用户名 / 手机号 / 邮箱**非空才改**，且分别做
//!   「排除自身」的唯一性校验，文案逐字对齐 Java。
//! - `updateMainAccountPwd`：两次密码必须一致，`passwordEncoder.encode` 后落库。
//! - `pageAdminUser`：控制器恒置 `isDeleted = 0`；关键词匹配
//!   `username` / `mobile`；`admin_id DESC`；每行补店铺数 / 门店数 / 供应商数，
//!   三者任一 > 0 则 `isAdmin = 1`。
//! - `deleteAdminUser`：账号不存在 ⇒「用户不存在」；仍是某店铺主账号 ⇒
//!   「主账号不允许删除」；否则软删除（`is_deleted = 1`）。
//!
//! # 未迁移 / 不确定点
//!
//! - Java 侧 `adminUserService.create` 的「商品消息 / 申请单」等副作用在本方法
//!   （`getAdmin` 新建账号）里不存在，无需迁移。
//! - `getMainAccount` 传了不存在的 `adminId` 时 Java `BeanUtils.copyProperties(null, ...)`
//!   会 NPE → 500；本实现返回字段全 `null` 的 VO（见 `uncertain_points`）。
//! - `bindMainAccount` 缺 `adminData` 时 Java NPE → 500；本实现返回
//!   `Error::Internal`（同样 500，但不抛 panic）。
//! - `updateMainAccountPwd` 缺 `password` 时 Java NPE → 500；本实现按空串参与
//!   比对，通常得到「两次密码不一致」（业务 500）而非 panic。
//! - `addTime` 格式化沿用仓库既有 `chrono::Local` 口径（Java 固定
//!   `Asia/Shanghai`），见 `uncertain_points`。
//! - MyBatis `.one()` 在「多行命中」时会抛 `TooManyResultsException`；
//!   本实现统一 `LIMIT 1` 取首行（关联查询/角色初始化/手机号复用）。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::AdminUser;
use nextrade_core::models::identity::admin_account::{
    admin_info, admin_type, admin_type_for_shop, default_avatar, format_timestamp, is_shop_like_type,
    shop_type, AdminInfoParam, AdminUserAccountListVO, AdminUserAccountVO, AdminUserPageParam,
    ShopOrVendorBindParam, ShopOrVendorListVO, ShopOrVendorPageParam, UpdateMainAccountParam,
    UpdateMainAccountPwdParam, AUTH_ALL_JSON, AUTH_EMPTY_JSON, DEFAULT_PASSWORD, DEFAULT_ROLE_ID,
    IS_ADMIN_NO, IS_ADMIN_YES, MSG_ADMIN_DATA_REQUIRED, MSG_ADMIN_NOT_EXIST,
    MSG_ADMIN_USER_NOT_EXIST, MSG_EMAIL_USED, MSG_MAIN_ACCOUNT_NOT_DELETABLE,
    MSG_MAIN_ACCOUNT_NOT_FOUND, MSG_MOBILE_USED, MSG_PARAM_REQUIRED, MSG_PASSWORD_MISMATCH,
    MSG_SHOP_NOT_FOUND, MSG_TYPE_INVALID, MSG_USERNAME_EXISTS, MSG_USER_MOBILE_REQUIRED,
    MSG_USER_NOT_EXIST,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;
use nextrade_security::hash_password;

/// Java `new Page<>()` 的默认分页尺寸（`Page` 无参构造：current=1, size=10）。
const JAVA_EMPTY_PAGE_SIZE: u64 = 10;

/// 账号管理域服务。
pub struct AdminAccountService {
    repos: Arc<Repositories>,
}

impl AdminAccountService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 查询 —————————————————————

    /// 主账号信息（对应 Java `getMainAccountByShopOrVendorId`）。
    ///
    /// # Errors
    /// - `adminId` 未提供时 `id` / `type` 缺失 ⇒ `参数不能为空`
    /// - 类型不在 `shop|store|pickup` ⇒ `类型参数错误，只支持shop或vendor`
    /// - 找不到主账号 ⇒ `未找到对应的主账号信息`
    pub async fn get_main_account(
        &self,
        admin_id: Option<i32>,
        id: Option<i32>,
        kind: Option<&str>,
    ) -> Result<AdminUserAccountVO> {
        if let Some(admin_id) = admin_id.filter(|v| *v > 0) {
            let admin = self.repos.identity.admin_account.find_admin(admin_id).await?;
            return Ok(to_account_vo(admin.as_ref()));
        }

        let Some(id) = id else {
            return Err(Error::biz_msg(MSG_PARAM_REQUIRED));
        };
        let kind = kind.unwrap_or("");
        if kind.trim().is_empty() {
            return Err(Error::biz_msg(MSG_PARAM_REQUIRED));
        }
        if !is_shop_like_type(kind) {
            return Err(Error::biz_msg(MSG_TYPE_INVALID));
        }

        let repo = &self.repos.identity.admin_account;
        let binding = repo.find_shop_main_account(id).await?;
        let main_admin = match binding {
            Some(binding) => repo.find_admin(binding.admin_id).await?,
            None => None,
        };
        match main_admin {
            Some(admin) => Ok(to_account_vo(Some(&admin))),
            None => Err(Error::biz_msg(MSG_MAIN_ACCOUNT_NOT_FOUND)),
        }
    }

    /// 店铺/供应商分页（对应 Java `pageShopOrVendor` → `pageShops`）。
    ///
    /// 本域只实现了 `shop|store|pickup` 分支（Java 也只实现这些；
    /// 文案里的 `vendor` 分支并不存在）。
    ///
    /// # Errors
    /// - 类型既非店铺类也非空 ⇒ `类型参数错误，只支持shop或vendor`
    pub async fn page_shop_or_vendor(
        &self,
        p: &ShopOrVendorPageParam,
    ) -> Result<PageResult<ShopOrVendorListVO>> {
        let page = p.page_query();
        let Some(admin_id) = p.admin_id else {
            return Ok(java_empty_page());
        };
        let kind = p.r#type.as_deref().unwrap_or("");
        if kind.trim().is_empty() {
            return Ok(java_empty_page());
        }
        if !is_shop_like_type(kind) {
            return Err(Error::biz_msg(MSG_TYPE_INVALID));
        }

        let repo = &self.repos.identity.admin_account;
        let bindings = repo.find_admin_main_shops(admin_id).await?;
        if bindings.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let shop_ids: Vec<i32> = bindings.iter().map(|b| b.shop_id).collect();
        let shop_type_filter = match kind {
            admin_type::STORE => Some(shop_type::STORE),
            admin_type::PICKUP => Some(shop_type::PICKUP),
            _ => Some(shop_type::SHOP),
        };
        let (total, shops) = repo
            .paginate_shops(&shop_ids, shop_type_filter, p.effective_keyword(), &page)
            .await?;

        // 回填商户名 / 主账号名（Java 逐行查询；这里批量取）。
        let merchant_ids: Vec<i32> = shops
            .iter()
            .filter_map(|s| s.merchant_id.filter(|id| *id > 0))
            .collect();
        let merchants: HashMap<i32, String> = repo
            .find_merchants(&merchant_ids)
            .await?
            .into_iter()
            .filter_map(|m| m.corporate_name.map(|name| (m.merchant_id, name)))
            .collect();
        let main_accounts: HashMap<i32, String> = repo
            .find_main_account_usernames(&shop_ids)
            .await?
            .into_iter()
            .collect();
        // 只保留当前页命中的店铺；上面按全部 bindings 查询，取 first-wins。
        let mut admin_usernames: HashMap<i32, String> = HashMap::new();
        for (shop_id, username) in main_accounts {
            admin_usernames.entry(shop_id).or_insert(username);
        }

        let list = shops
            .iter()
            .map(|shop| ShopOrVendorListVO {
                id: shop.shop_id,
                name: shop.shop_title.clone(),
                logo: shop.shop_logo.clone(),
                merchant_name: shop
                    .merchant_id
                    .filter(|id| *id > 0)
                    .and_then(|id| merchants.get(&id).cloned()),
                status: shop.status,
                admin_username: admin_usernames.get(&shop.shop_id).cloned(),
                add_time: shop.add_time.map(format_timestamp).unwrap_or_default(),
                r#type: admin_type::SHOP.to_string(),
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 账号管理分页（对应 Java `pageAdminUser`）。
    ///
    /// 控制器恒置 `isDeleted = 0`，故入参里的 `isDeleted` 不生效。
    pub async fn page_admin_user(
        &self,
        p: &AdminUserPageParam,
    ) -> Result<PageResult<AdminUserAccountListVO>> {
        let page = p.page_query();
        let repo = &self.repos.identity.admin_account;
        let (total, rows) = repo
            .paginate_admin_users(p.effective_keyword(), 0, &page)
            .await?;
        let admin_ids: Vec<i32> = rows.iter().map(|r| r.admin_id).collect();

        // 批量统计店铺 / 门店 / 供应商数（对应 Java 每行 3 次 count）。
        let mut shop_counts: HashMap<(i32, i32), i64> = HashMap::new();
        for (admin_id, shop_type_value, count) in
            repo.count_shop_bindings_by_type(&admin_ids).await?
        {
            if let Some(shop_type_value) = shop_type_value {
                shop_counts.insert((admin_id, shop_type_value), count);
            }
        }
        let vendor_counts: HashMap<i32, i64> =
            repo.count_vendor_bindings(&admin_ids).await?.into_iter().collect();

        let list = rows
            .iter()
            .map(|admin| {
                let shop_count = *shop_counts
                    .get(&(admin.admin_id, shop_type::SHOP))
                    .unwrap_or(&0);
                let store_count = *shop_counts
                    .get(&(admin.admin_id, shop_type::STORE))
                    .unwrap_or(&0);
                let vendor_count = *vendor_counts.get(&admin.admin_id).unwrap_or(&0);
                let is_admin = if shop_count > 0 || store_count > 0 || vendor_count > 0 {
                    IS_ADMIN_YES
                } else {
                    IS_ADMIN_NO
                };
                AdminUserAccountListVO {
                    admin_id: Some(admin.admin_id),
                    avatar: admin.avatar.clone(),
                    username: Some(admin.username.clone()),
                    mobile: admin.mobile.clone(),
                    email: Some(admin.email.clone()),
                    add_time: format_timestamp(admin.add_time),
                    shop_count: Some(shop_count),
                    vendor_count: Some(vendor_count),
                    store_count: Some(store_count),
                    is_admin: Some(is_admin),
                }
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    // ————————————————————— 绑定主账号 —————————————————————

    /// 绑定主账号（对应 Java `bindMainAccount`，**单事务**）。
    ///
    /// # Errors
    /// - 类型非店铺类 ⇒ `类型参数错误，只支持shop或vendor`
    /// - `adminData` 缺失 ⇒ `Internal`（Java 为 NPE）
    /// - 会员不存在 / 手机号为空 / 管理员不存在 / 店铺不存在，以及默认密码哈希失败
    pub async fn bind_main_account(&self, p: &ShopOrVendorBindParam) -> Result<()> {
        let kind = p.r#type.as_deref().unwrap_or("");
        if !is_shop_like_type(kind) {
            return Err(Error::biz_msg(MSG_TYPE_INVALID));
        }
        let info: &AdminInfoParam = p
            .admin_data
            .as_ref()
            .ok_or_else(|| Error::Internal(MSG_ADMIN_DATA_REQUIRED.into()))?;
        let repo = &self.repos.identity.admin_account;
        let mut tx = repo.begin().await?;

        // —— 1. Java `getAdmin(createParam)` ——
        let admin = if info.r#type == Some(admin_info::USER) {
            // 会员来源：查会员 → 手机号复用/新建管理员。
            let user_id = info.user_id.unwrap_or(0);
            let user = repo
                .find_user_in_tx(&mut tx, user_id)
                .await?
                .ok_or_else(|| Error::biz_msg(MSG_USER_NOT_EXIST))?;
            if user.mobile.is_empty() {
                return Err(Error::biz_msg(MSG_USER_MOBILE_REQUIRED));
            }
            match repo
                .find_admin_by_mobile_in_tx(&mut tx, &user.mobile)
                .await?
            {
                Some(admin) => admin,
                None => {
                    let mut created = AdminUser {
                        username: user.username.clone(),
                        user_id: user.user_id,
                        mobile: Some(user.mobile.clone()),
                        email: user.email.clone(),
                        password: hash_password(DEFAULT_PASSWORD)?,
                        admin_type: kind.to_string(),
                        is_using: 0,
                        auth_list: AUTH_EMPTY_JSON.to_string(),
                        role_id: DEFAULT_ROLE_ID,
                        avatar: Some(default_avatar(uuid::Uuid::new_v4().as_u128())),
                        ..Default::default()
                    };
                    let admin_id = repo.insert_admin_in_tx(&mut tx, &created).await?;
                    created.admin_id = admin_id;
                    created
                }
            }
        } else {
            // 管理员来源：按 adminId 取已有管理员。
            let admin_id = info.admin_id.unwrap_or(0);
            repo.find_admin_in_tx(&mut tx, admin_id)
                .await?
                .ok_or_else(|| Error::biz_msg(MSG_ADMIN_USER_NOT_EXIST))?
        };

        // —— 2. Java `bindShopMainAccount(...)` ——
        let shop_id = p.id.unwrap_or(0);
        let old_user_shop = repo
            .find_user_shop_in_tx(&mut tx, admin.admin_id, shop_id)
            .await?;
        let main_account = repo.find_shop_main_account_in_tx(&mut tx, shop_id).await?;
        match main_account {
            None => {
                // 2a. `recoverShopMainAccount(...)`：店铺必须存在。
                let shop = repo
                    .find_shop_in_tx(&mut tx, shop_id)
                    .await?
                    .ok_or_else(|| Error::biz_msg(MSG_SHOP_NOT_FOUND))?;
                let role_id = repo
                    .ensure_system_role_in_tx(
                        &mut tx,
                        admin_type_for_shop(shop.shop_type),
                        shop.merchant_id.unwrap_or(0),
                        shop_id,
                    )
                    .await?;

                let create_record = old_user_shop.is_none();
                let mut row = old_user_shop.unwrap_or_default();
                row.shop_id = shop_id;
                row.admin_id = admin.admin_id;
                row.user_id = resolve_bound_user_id(p, info);
                row.username = admin.username.clone();
                row.email = admin.email.clone();
                row.avatar = admin.avatar.clone();
                row.auth_list = Some(AUTH_ALL_JSON.to_string());
                row.is_using = 0;
                row.is_admin = IS_ADMIN_YES;
                row.role_id = role_id;
                if create_record {
                    row.add_time = Some(now_seconds() as i32);
                    repo.insert_user_shop_in_tx(&mut tx, &row).await?;
                } else {
                    repo.update_user_shop_in_tx(&mut tx, &row).await?;
                }
            }
            Some(main_account) => {
                // 2b. 已有主账号：改绑 + 旧绑定降级。
                let old_admin_id = main_account.admin_id;
                let old_admin_username = main_account.username.clone();
                let mut updated = main_account.clone();
                updated.admin_id = admin.admin_id;
                updated.username = admin.username.clone();
                updated.auth_list = Some(AUTH_ALL_JSON.to_string());
                updated.is_admin = IS_ADMIN_YES;
                repo.update_user_shop_in_tx(&mut tx, &updated).await?;

                if let Some(old) = old_user_shop {
                    if old.id != updated.id {
                        let mut old_row = old;
                        old_row.admin_id = old_admin_id;
                        old_row.is_admin = IS_ADMIN_NO;
                        old_row.username = old_admin_username;
                        repo.update_user_shop_in_tx(&mut tx, &old_row).await?;
                    }
                }
            }
        }

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        tracing::info!(
            "绑定主账号成功: shop_id={shop_id}, admin_id={}, type={kind}",
            admin.admin_id
        );
        Ok(())
    }

    // ————————————————————— 修改 —————————————————————

    /// 修改主账号信息（对应 Java `updateMainAccount`）。
    ///
    /// 用户名 / 手机号 / 邮箱仅在**非空**时更新，并各自做排除自身的唯一性校验。
    ///
    /// # Errors
    /// - 账号不存在 ⇒ `用户不存在`
    /// - 用户名 / 手机号 / 邮箱被占用 ⇒ 对应 Java 文案
    pub async fn update_main_account(&self, p: &UpdateMainAccountParam) -> Result<()> {
        let repo = &self.repos.identity.admin_account;
        let admin_id = p.admin_id.unwrap_or(0);
        let Some(mut admin) = repo.find_admin(admin_id).await? else {
            return Err(Error::biz_msg(MSG_ADMIN_NOT_EXIST));
        };

        if let Some(username) = p.username.as_deref().filter(|s| !s.is_empty()) {
            if repo
                .count_admins_by_username(username, admin.admin_id)
                .await?
                > 0
            {
                return Err(Error::biz_msg(MSG_USERNAME_EXISTS));
            }
            admin.username = username.to_string();
        }
        if let Some(mobile) = p.mobile.as_deref().filter(|s| !s.is_empty()) {
            if repo
                .count_admins_by_mobile(mobile, admin.admin_id)
                .await?
                > 0
            {
                return Err(Error::biz_msg(MSG_MOBILE_USED));
            }
            admin.mobile = Some(mobile.to_string());
        }
        if let Some(email) = p.email.as_deref().filter(|s| !s.is_empty()) {
            if repo.count_admins_by_email(email, admin.admin_id).await? > 0 {
                return Err(Error::biz_msg(MSG_EMAIL_USED));
            }
            admin.email = email.to_string();
        }

        repo.update_admin_contact(
            admin.admin_id,
            &admin.username,
            admin.mobile.as_deref(),
            &admin.email,
        )
        .await?;
        Ok(())
    }

    /// 修改主账号密码（对应 Java `updateMainAccountPwd`）。
    ///
    /// # Errors
    /// - 账号不存在 ⇒ `用户不存在`
    /// - 两次密码不一致 ⇒ `两次密码不一致`
    /// - 密码哈希失败 ⇒ `Internal`
    pub async fn update_main_account_pwd(&self, p: &UpdateMainAccountPwdParam) -> Result<()> {
        let repo = &self.repos.identity.admin_account;
        let admin_id = p.admin_id.unwrap_or(0);
        let Some(admin) = repo.find_admin(admin_id).await? else {
            return Err(Error::biz_msg(MSG_ADMIN_NOT_EXIST));
        };
        // Java：`getPassword().equals(getPwdConfirm())`（password 为 null 时 NPE）。
        let password = p.password.as_deref().unwrap_or("");
        if Some(password) != p.pwd_confirm.as_deref() {
            return Err(Error::biz_msg(MSG_PASSWORD_MISMATCH));
        }
        let hash = hash_password(password)?;
        repo.update_admin_password(admin.admin_id, &hash).await?;
        Ok(())
    }

    /// 删除管理员账号（对应 Java `deleteAdminUser`，软删除）。
    ///
    /// # Errors
    /// - 账号不存在 ⇒ `用户不存在`
    /// - 仍是某店铺主账号 ⇒ `主账号不允许删除`
    pub async fn delete_admin_user(&self, admin_id: i32) -> Result<()> {
        let repo = &self.repos.identity.admin_account;
        let admin = repo
            .find_admin(admin_id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_ADMIN_NOT_EXIST))?;
        if repo.find_admin_main_binding(admin.admin_id).await?.is_some() {
            return Err(Error::biz_msg(MSG_MAIN_ACCOUNT_NOT_DELETABLE));
        }
        repo.soft_delete_admin(admin.admin_id).await?;
        Ok(())
    }
}

/// `admin_user` 实体 → `AdminUserAccountVO`（Java `BeanUtils.copyProperties`）。
///
/// 实体为 `None` 时返回字段全 `None` 的 VO（Java 此处会 NPE，见模块注释）。
fn to_account_vo(admin: Option<&AdminUser>) -> AdminUserAccountVO {
    match admin {
        Some(admin) => AdminUserAccountVO {
            admin_id: Some(admin.admin_id),
            username: Some(admin.username.clone()),
            mobile: admin.mobile.clone(),
            email: Some(admin.email.clone()),
        },
        None => AdminUserAccountVO::default(),
    }
}

/// Java `resolveBoundUserId`：会员来源取 `adminData.userId`，否则 `0`。
fn resolve_bound_user_id(p: &ShopOrVendorBindParam, info: &AdminInfoParam) -> i32 {
    let _ = p;
    if info.r#type == Some(admin_info::USER) {
        info.user_id.unwrap_or(0)
    } else {
        0
    }
}

/// Java `new Page<>()`：current=1 / size=10 / total=0。
fn java_empty_page<T>() -> PageResult<T> {
    PageResult::empty(&PageQuery {
        page: 1,
        page_size: JAVA_EMPTY_PAGE_SIZE,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::entities::AdminUser;

    // ———————————————— 纯函数 ————————————————

    #[test]
    fn to_account_vo_copies_four_fields() {
        let admin = AdminUser {
            admin_id: 7,
            username: "root".into(),
            mobile: Some("13800000000".into()),
            email: "a@b.c".into(),
            ..Default::default()
        };
        let vo = to_account_vo(Some(&admin));
        assert_eq!(vo.admin_id, Some(7));
        assert_eq!(vo.username.as_deref(), Some("root"));
        assert_eq!(vo.mobile.as_deref(), Some("13800000000"));
        assert_eq!(vo.email.as_deref(), Some("a@b.c"));

        // 实体缺失 ⇒ 全空（Java 会 NPE）
        let vo = to_account_vo(None);
        assert_eq!(vo.admin_id, None);
        assert_eq!(vo.username, None);
        assert_eq!(vo.mobile, None);
        assert_eq!(vo.email, None);
    }

    #[test]
    fn resolve_bound_user_id_follows_admin_info_type() {
        let p = ShopOrVendorBindParam::default();
        let user_info = AdminInfoParam {
            r#type: Some(admin_info::USER),
            user_id: Some(42),
            admin_id: Some(9),
        };
        assert_eq!(resolve_bound_user_id(&p, &user_info), 42);

        let admin_info_param = AdminInfoParam {
            r#type: Some(admin_info::ADMIN_USER),
            user_id: Some(42),
            admin_id: Some(9),
        };
        assert_eq!(resolve_bound_user_id(&p, &admin_info_param), 0);

        // type 缺失 ⇒ 走管理员分支 ⇒ 0（Java equals(null, 1) = false）
        let empty = AdminInfoParam::default();
        assert_eq!(resolve_bound_user_id(&p, &empty), 0);
    }

    #[test]
    fn java_empty_page_is_current_1_size_10() {
        let page: PageResult<u8> = java_empty_page();
        assert_eq!(page.page, 1);
        assert_eq!(page.page_size, JAVA_EMPTY_PAGE_SIZE);
        assert_eq!(page.total, 0);
        assert_eq!(page.total_page, 0);
        assert!(page.list.is_empty());
    }

    // ———————————————— 校验文案（与 Java 逐字对齐） ————————————————

    #[test]
    fn error_messages_match_java_sources() {
        assert_eq!(MSG_PARAM_REQUIRED, "参数不能为空");
        assert_eq!(MSG_TYPE_INVALID, "类型参数错误，只支持shop或vendor");
        assert_eq!(MSG_MAIN_ACCOUNT_NOT_FOUND, "未找到对应的主账号信息");
        assert_eq!(MSG_SHOP_NOT_FOUND, "店铺不存在");
        assert_eq!(MSG_USER_NOT_EXIST, "会员不存在");
        assert_eq!(MSG_USER_MOBILE_REQUIRED, "会员手机号码不能为空");
        assert_eq!(MSG_ADMIN_USER_NOT_EXIST, "管理员不存在");
        assert_eq!(MSG_ADMIN_NOT_EXIST, "用户不存在");
        assert_eq!(MSG_USERNAME_EXISTS, "账号名称已经存在");
        assert_eq!(MSG_MOBILE_USED, "手机号已经使用");
        assert_eq!(MSG_EMAIL_USED, "邮箱已经使用");
        assert_eq!(MSG_PASSWORD_MISMATCH, "两次密码不一致");
        assert_eq!(MSG_MAIN_ACCOUNT_NOT_DELETABLE, "主账号不允许删除");
        assert_eq!(DEFAULT_PASSWORD, "admin123321");
        assert_eq!(AUTH_ALL_JSON, "[\"all\"]");
        assert_eq!(AUTH_EMPTY_JSON, "[]");
        assert_eq!(DEFAULT_ROLE_ID, 1);
    }

    /// 密码不一致分支：`password` 缺失时按空串比对，得到业务错误而非 panic。
    #[test]
    fn password_mismatch_detects_differences() {
        let eq = |a: Option<&str>, b: Option<&str>| Some(a.unwrap_or("")) == b;
        assert!(eq(Some("a"), Some("a")));
        assert!(!eq(Some("a"), Some("b")));
        assert!(!eq(Some("a"), None));
        assert!(!eq(None, None)); // Java：null.equals(...) 会 NPE，这里按空串处理
        assert!(eq(None, Some("")));
    }
}
