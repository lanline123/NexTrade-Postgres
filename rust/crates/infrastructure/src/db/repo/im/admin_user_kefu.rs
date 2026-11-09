//! 客服人员管理 域仓储（im::admin_user_kefu）。
//!
//! 表（列名以 `java/database/pure.sql` 为准）：
//! - `admin_user_shop`：`id` / `admin_id` / `user_id` / `shop_id` / `username` /
//!   `email` / `avatar` / `auth_list` / `is_using` / `add_time` / `role_id`
//! - `admin_user`：`admin_id` / `username` / `mobile` / `email` / `avatar` /
//!   `role_id` / `shop_id` / `is_using` / `admin_type` / `user_id` / `add_time`
//! - `admin_role`：`role_id` / `role_name` / `role_code` / `shop_id`
//!
//! 运行时 sqlx（**不**使用 `query!` 宏）。客服角色通过
//! `admin_role.role_code = 'kefu_agent'` 识别（Java `KEFU_AGENT_ROLE_CODE`）。

use sqlx::PgPool;
use sqlx::FromRow;

use nextrade_core::models::im::admin_user_kefu::{AdminUserKefuPageQuery, AdminUserKefuRow};
use nextrade_core::{Error, Result};

/// 客服角色行。
#[derive(Debug, Clone, FromRow)]
pub struct KefuRoleRow {
    /// 角色 ID
    pub role_id: i32,
    /// 角色名称
    pub role_name: String,
    /// 角色编码
    pub role_code: String,
    /// 店铺 ID
    pub shop_id: i32,
}

/// `admin_user` 中客服需要的最小字段。
#[derive(Debug, Clone, FromRow)]
pub struct KefuAdminUserRow {
    /// 管理员 ID
    pub admin_id: i32,
    /// 管理员名称
    pub username: String,
    /// 手机号
    pub mobile: String,
    /// 邮箱
    pub email: String,
    /// 头像
    pub avatar: String,
    /// 关联会员 ID
    pub user_id: i32,
}

enum Bind {
    I32(i32),
    I64(i64),
    Str(String),
}

/// 按顺序绑定 `Bind` 列表（与模板 `repo/payment/user_balance_log.rs` 同构）。
macro_rules! bind_all {
    ($q:expr, $binds:expr) => {{
        let mut q = $q;
        for b in $binds.iter() {
            q = match b {
                Bind::I32(v) => q.bind(*v),
                Bind::I64(v) => q.bind(*v),
                Bind::Str(v) => q.bind(v.clone()),
            };
        }
        q
    }};
}

/// 客服人员仓储。
#[derive(Clone)]
pub struct AdminUserKefuRepository {
    pool: PgPool,
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

impl AdminUserKefuRepository {
    /// 构造仓储。
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 客服角色（Java `findKefuAgentRole`：role_code='kefu_agent' + shop_id，limit 1）。
    pub async fn find_kefu_role(&self, shop_id: i32) -> Result<Option<KefuRoleRow>> {
        sqlx::query_as::<_, KefuRoleRow>(
            "SELECT role_id, role_name, role_code, shop_id FROM admin_role \
             WHERE role_code = 'kefu_agent' AND shop_id = $1 LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 客服绑定分页（Java `listUser`：admin_user_shop LEFT JOIN admin_user/admin_role）。
    pub async fn paginate(
        &self,
        shop_id: i32,
        q: &AdminUserKefuPageQuery,
    ) -> Result<(u64, Vec<AdminUserKefuRow>)> {
        let mut conds = vec![
            "s.shop_id = ?".to_string(),
            "(r.role_code = 'kefu_agent' OR s.role_id IS NULL)".to_string(),
        ];
        let mut binds: Vec<Bind> = vec![Bind::I32(shop_id)];
        if let Some(v) = q.is_using {
            conds.push("s.is_using = ?".to_string());
            binds.push(Bind::I32(v));
        }
        if let Some(kw) = q.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            conds.push("(s.username LIKE ? OR u.mobile LIKE ?)".to_string());
            binds.push(Bind::Str(format!("%{kw}%")));
            binds.push(Bind::Str(format!("%{kw}%")));
        }
        if let Some(start) = q.search_start_time.as_deref().and_then(day_start_secs) {
            conds.push("s.add_time >= ?".to_string());
            binds.push(Bind::I64(start));
        }
        if let Some(end) = q.search_end_time.as_deref().and_then(day_end_secs) {
            conds.push("s.add_time <= ?".to_string());
            binds.push(Bind::I64(end));
        }
        let where_sql = conds.join(" AND ");
        let (size, offset) = q.limit_offset();
        let base = crate::db::to_pg_placeholders(&format!(
            " FROM admin_user_shop s LEFT JOIN admin_user u ON u.admin_id = s.admin_id \
             LEFT JOIN admin_role r ON r.role_id = s.role_id WHERE {where_sql}"
        ));

        let count_sql = format!("SELECT COUNT(*){base}");
        let total = bind_all!(
            sqlx::query_as::<_, (i64,)>(&count_sql),
            &binds
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?
        .0;

        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT s.id, s.admin_id, s.shop_id, s.username, s.email, s.avatar, s.is_using, \
             s.role_id, s.auth_list, s.add_time, r.role_name{base} \
             ORDER BY s.id DESC LIMIT ? OFFSET ?"
        ));
        let rows = bind_all!(sqlx::query_as::<_, AdminUserKefuRow>(&sql), &binds)
            .bind(i64::try_from(size).unwrap_or(15))
            .bind(i64::try_from(offset).unwrap_or(0))
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total).unwrap_or(0), rows))
    }

    /// 按绑定 ID 查客服（必须同店铺）。
    pub async fn find_binding(&self, id: i32, shop_id: i32) -> Result<Option<AdminUserKefuRow>> {
        self.find_binding_where("s.id = ?", id, shop_id).await
    }

    /// 按管理员 ID 查客服（必须同店铺）。
    pub async fn find_binding_by_admin(&self, admin_id: i32, shop_id: i32) -> Result<Option<AdminUserKefuRow>> {
        self.find_binding_where("s.admin_id = ?", admin_id, shop_id).await
    }

    async fn find_binding_where(&self, key_cond: &str, key: i32, shop_id: i32) -> Result<Option<AdminUserKefuRow>> {
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT s.id, s.admin_id, s.shop_id, s.username, s.email, s.avatar, s.is_using, \
             s.role_id, s.auth_list, s.add_time, r.role_name FROM admin_user_shop s \
             LEFT JOIN admin_role r ON r.role_id = s.role_id WHERE {key_cond} AND s.shop_id = ? LIMIT 1"
        ));
        sqlx::query_as::<_, AdminUserKefuRow>(&sql)
            .bind(key)
            .bind(shop_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 管理员名称是否被占用（Java `admin_user.username` 唯一性校验）。
    pub async fn count_username(&self, username: &str, exclude_admin_id: Option<i32>) -> Result<u64> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM admin_user WHERE username = $1 AND ($2 IS NULL OR admin_id <> $3)",
        )
        .bind(username)
        .bind(exclude_admin_id)
        .bind(exclude_admin_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    /// 手机号是否被占用。
    pub async fn count_mobile(&self, mobile: &str, exclude_admin_id: Option<i32>) -> Result<u64> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM admin_user WHERE mobile = $1 AND ($2 IS NULL OR admin_id <> $3)",
        )
        .bind(mobile)
        .bind(exclude_admin_id)
        .bind(exclude_admin_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    /// 按手机号查管理员（Java `selectOne(eq(AdminUser::getMobile, mobile))`）。
    pub async fn find_admin_by_mobile(&self, mobile: &str) -> Result<Option<KefuAdminUserRow>> {
        sqlx::query_as::<_, KefuAdminUserRow>(
            "SELECT admin_id, username, mobile, email, avatar, user_id FROM admin_user \
             WHERE mobile = $1 LIMIT 1",
        )
        .bind(mobile)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按 ID 查管理员。
    pub async fn find_admin_by_id(&self, admin_id: i32) -> Result<Option<KefuAdminUserRow>> {
        sqlx::query_as::<_, KefuAdminUserRow>(
            "SELECT admin_id, username, mobile, email, avatar, user_id FROM admin_user WHERE admin_id = $1",
        )
        .bind(admin_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 插入管理员（新建客服且手机号不存在时，Java \"AdminUser\" 落库）。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_admin(
        &self,
        username: &str,
        mobile: &str,
        email: &str,
        avatar: &str,
        role_id: Option<i32>,
        shop_id: i32,
        is_using: i32,
        now: i32,
    ) -> Result<i64> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO admin_user \
             (username, admin_type, mobile, avatar, password, email, add_time, auth_list, user_id, \
              role_id, shop_id, is_using, initial_password, is_deleted) \
             VALUES ($1, '', $2, $3, '', $4, $5, '[\"all\"]', 0, $6, $7, $8, '', 0) RETURNING admin_id",
        )
        .bind(username)
        .bind(mobile)
        .bind(avatar)
        .bind(email)
        .bind(now)
        .bind(role_id)
        .bind(shop_id)
        .bind(is_using)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(i64::from(r))
    }

    /// 更新管理员基础信息。
    #[allow(clippy::too_many_arguments)]
    pub async fn update_admin(
        &self,
        admin_id: i32,
        username: &str,
        mobile: &str,
        email: &str,
        avatar: &str,
        is_using: i32,
        role_id: Option<i32>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE admin_user SET username = $1, mobile = $2, email = $3, avatar = $4, \
             is_using = $5, role_id = $6 WHERE admin_id = $7",
        )
        .bind(username)
        .bind(mobile)
        .bind(email)
        .bind(avatar)
        .bind(is_using)
        .bind(role_id)
        .bind(admin_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 插入店铺员工（客服绑定）。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_binding(
        &self,
        admin_id: i32,
        user_id: i32,
        shop_id: i32,
        username: &str,
        email: &str,
        avatar: &str,
        is_using: i32,
        role_id: Option<i32>,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO admin_user_shop \
             (admin_id, user_id, shop_id, username, email, avatar, auth_list, is_using, is_admin, \
              add_time, role_id, work_on_time, default_pos_login) \
             VALUES ($1, $2, $3, $4, $5, $6, '[\"all\"]', $7, 0, $8, $9, 0, 0)",
        )
        .bind(admin_id)
        .bind(user_id)
        .bind(shop_id)
        .bind(username)
        .bind(email)
        .bind(avatar)
        .bind(is_using)
        .bind(now)
        .bind(role_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 更新店铺员工（客服绑定）。
    #[allow(clippy::too_many_arguments)]
    pub async fn update_binding(
        &self,
        id: i32,
        shop_id: i32,
        username: &str,
        email: &str,
        avatar: &str,
        is_using: i32,
        role_id: Option<i32>,
    ) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE admin_user_shop SET username = $1, email = $2, avatar = $3, is_using = $4, role_id = $5 \
             WHERE id = $6 AND shop_id = $7",
        )
        .bind(username)
        .bind(email)
        .bind(avatar)
        .bind(is_using)
        .bind(role_id)
        .bind(id)
        .bind(shop_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 删除单个绑定（同店铺）。
    pub async fn delete_binding(&self, id: i32, shop_id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM admin_user_shop WHERE id = $1 AND shop_id = $2")
            .bind(id)
            .bind(shop_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 批量删除绑定（同店铺）。
    pub async fn delete_bindings(&self, ids: &[i32], shop_id: i32) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let marks = vec!["?"; ids.len()].join(",");
        let sql = crate::db::to_pg_placeholders(&format!("DELETE FROM admin_user_shop WHERE shop_id = ? AND id IN ({marks})"));
        let mut q = sqlx::query(&sql).bind(shop_id);
        for id in ids {
            q = q.bind(*id);
        }
        let r = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 按 admin_id 删绑定（联动删除时不需要，保留给批量场景）。
    pub async fn delete_binding_by_admin(&self, admin_id: i32, shop_id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM admin_user_shop WHERE admin_id = $1 AND shop_id = $2")
            .bind(admin_id)
            .bind(shop_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }
}

/// `yyyy-MM-dd` → 当天 00:00:00 秒级时间戳。
fn day_start_secs(s: &str) -> Option<i64> {
    chrono::NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp())
}

/// `yyyy-MM-dd` → 当天 23:59:59 秒级时间戳。
fn day_end_secs(s: &str) -> Option<i64> {
    chrono::NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(23, 59, 59))
        .map(|dt| dt.and_utc().timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_range_helpers_parse_iso_date() {
        assert_eq!(day_start_secs("2026-01-02"), Some(1_767_312_000));
        assert_eq!(day_end_secs("2026-01-02"), Some(1_767_398_399));
        assert_eq!(day_start_secs("bad"), None);
    }
}
