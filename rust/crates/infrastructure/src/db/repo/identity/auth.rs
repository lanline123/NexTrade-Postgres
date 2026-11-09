//! `admin_user` 仓储，替代 `AdminUserMapper`。

use sqlx::PgPool;
use nextrade_core::entities::AdminUser;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct AdminUserRepository {
    pool: PgPool,
}

impl AdminUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 按登录名查询（含已删除/停用，由调用方判断）。
    pub async fn find_by_username(&self, username: &str) -> Result<Option<AdminUser>> {
        sqlx::query_as::<_, AdminUser>(
            "SELECT * FROM admin_user WHERE username = $1 AND is_deleted = 0 LIMIT 1",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按手机号查询（后台手机验证码登录）。
    pub async fn find_by_mobile(&self, mobile: &str) -> Result<Option<AdminUser>> {
        sqlx::query_as::<_, AdminUser>(
            "SELECT * FROM admin_user WHERE mobile = $1 AND is_deleted = 0 LIMIT 1",
        )
        .bind(mobile)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按 ID 查询。
    pub async fn find_by_id(&self, admin_id: i32) -> Result<Option<AdminUser>> {
        sqlx::query_as::<_, AdminUser>("SELECT * FROM admin_user WHERE admin_id = $1 LIMIT 1")
            .bind(admin_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 后台用户分页（对应原 AdminUserPageQuery）。
    pub async fn paginate(
        &self,
        keyword: Option<&str>,
        admin_type: Option<&str>,
        page: u64,
        page_size: u64,
    ) -> Result<(u64, Vec<AdminUser>)> {
        let offset = page.saturating_sub(1) * page_size;
        let kw = keyword.map(|s| format!("%{s}%"));

        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM admin_user
             WHERE is_deleted = 0
               AND ($1 IS NULL OR username LIKE $2)
               AND ($3 IS NULL OR admin_type = $4)",
        )
        .bind(&kw)
        .bind(&kw)
        .bind(admin_type)
        .bind(admin_type)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        let rows = sqlx::query_as::<_, AdminUser>(
            "SELECT * FROM admin_user
             WHERE is_deleted = 0
               AND ($1 IS NULL OR username LIKE $2)
               AND ($3 IS NULL OR admin_type = $4)
             ORDER BY admin_id DESC
             LIMIT $5 OFFSET $6",
        )
        .bind(&kw)
        .bind(&kw)
        .bind(admin_type)
        .bind(admin_type)
        .bind(page_size as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok((total.0 as u64, rows))
    }

    /// 重置密码。
    pub async fn update_password(&self, admin_id: i32, new_hash: &str) -> Result<()> {
        sqlx::query("UPDATE admin_user SET password = $1 WHERE admin_id = $2")
            .bind(new_hash)
            .bind(admin_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("记录不存在".into()),
        other => Error::Database(other.to_string()),
    }
}
