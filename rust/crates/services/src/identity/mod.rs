//! identity 领域服务。

pub mod admin_account;
pub mod admin_log;
pub mod admin_role;
pub mod admin_user;
pub mod auth;
pub mod authority;
pub mod permission;

use std::sync::Arc;

/// identity 领域服务集合（由组合根装配一次，api 层只持有 `Arc<Services>`）。
#[derive(Clone)]
pub struct Services {
    pub admin_log: Arc<admin_log::AdminLogService>,
    pub admin_user: Arc<admin_user::AdminUserService>,
    pub auth: Arc<auth::AuthService>,
    pub authority: Arc<authority::AuthorityService>,
    pub admin_role: Arc<admin_role::AdminRoleService>,
    pub admin_account: Arc<admin_account::AdminAccountService>,
    pub permission: Arc<permission::PermissionService>,
}
