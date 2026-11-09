//! identity 领域仓储。

pub mod admin_account;
pub mod admin_log;
pub mod admin_role;
pub mod admin_user;
pub mod auth;
pub mod authority;

use crate::db::DbPool;

#[derive(Clone)]
pub struct Repositories {
    pub admin_account: admin_account::AdminAccountRepository,
    pub admin_log: admin_log::AdminLogRepository,
    pub admin_role: admin_role::AdminRoleRepository,
    pub admin_user: admin_user::AdminUserRepository,
    pub auth: auth::AdminUserRepository,
    pub authority: authority::AuthorityRepository,
}

impl Repositories {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self {
            admin_account: admin_account::AdminAccountRepository::new(pool.clone()),
            admin_log: admin_log::AdminLogRepository::new(pool.clone()),
            admin_role: admin_role::AdminRoleRepository::new(pool.clone()),
            admin_user: admin_user::AdminUserRepository::new(pool.clone()),
            auth: auth::AdminUserRepository::new(pool.clone()),
            authority: authority::AuthorityRepository::new(pool.clone()),
        }
    }
}
