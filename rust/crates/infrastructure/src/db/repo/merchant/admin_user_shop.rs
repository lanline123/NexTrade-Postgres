//! 店铺员工域仓储（merchant::admin_user_shop）。
//!
//! 表：`admin_user_shop`（员工绑定）/ `admin_user`（后台账号）/ `user`（前台会员）/
//! `admin_role`（角色）/ `shop`（店铺 merchant_id、shop_type）/ `config`（员工数上限）。
//!
//! 对应 Java `AdminUserShopImpl`（`AdminUserShopMapper` + `adminUserMapper` + `userMapper`
//! + `adminRoleMapper` + `shopMapper` + `configService`）。
//! 运行时 `sqlx`（**不用** `query!` 宏）；每条 SQL 单行字面量。

use sqlx::PgPool;
use nextrade_core::entities::AdminUserShop;
use nextrade_core::models::merchant::admin_user_shop::AdminUserShopPageQuery;
use nextrade_core::{Error, Result};

/// 员工绑定 16 列。
const BINDING_COLUMNS: &str = "id, admin_id, user_id, shop_id, username, email, avatar, auth_list, is_using, is_admin, add_time, role_id, work_on_time, default_pos_login, last_login_system, device_code";

/// 店铺员工域仓储。
#[derive(Clone)]
pub struct AdminUserShopRepository {
    pool: PgPool,
}

impl AdminUserShopRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 员工分页（Java `list`：`eq(shopId)` + `like(username, keyword)` + `buildSortOrder`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn paginate(
        &self,
        shop_id: i32,
        q: &AdminUserShopPageQuery,
    ) -> Result<(u64, Vec<AdminUserShop>)> {
        let kw = q
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{s}%"));
        let where_sql = if kw.is_some() {
            " WHERE shop_id = $1 AND username LIKE $2"
        } else {
            " WHERE shop_id = $1"
        };
        let (page, size) = q.page_params();
        let sort = sort_column(q);
        let total: (i64,) = if let Some(v) = kw.as_deref() {
            sqlx::query_as(&format!("SELECT COUNT(*) FROM admin_user_shop{where_sql}"))
                .bind(shop_id)
                .bind(v)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?
        } else {
            sqlx::query_as(&format!("SELECT COUNT(*) FROM admin_user_shop{where_sql}"))
                .bind(shop_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?
        };
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {BINDING_COLUMNS} FROM admin_user_shop{where_sql} ORDER BY {sort} LIMIT ? OFFSET ?"
        ));
        let mut query = sqlx::query_as::<_, AdminUserShop>(&sql);
        query = query.bind(shop_id);
        if let Some(v) = kw.as_deref() {
            query = query.bind(v);
        }
        let rows = query
            .bind(i64::try_from(size).unwrap_or(15))
            .bind(i64::try_from((page - 1) * size).unwrap_or(0))
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 按主键 + 店铺查绑定（Java `requireCurrentShopBinding`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn find_scoped(&self, id: i32, shop_id: i32) -> Result<Option<AdminUserShop>> {
        sqlx::query_as::<_, AdminUserShop>(&format!(
            "SELECT {BINDING_COLUMNS} FROM admin_user_shop WHERE id = $1 AND shop_id = $2 LIMIT 1"
        ))
        .bind(id)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按管理员 + 店铺查绑定（Java `info`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn find_by_admin_shop(
        &self,
        admin_id: i32,
        shop_id: i32,
    ) -> Result<Option<AdminUserShop>> {
        sqlx::query_as::<_, AdminUserShop>(&format!(
            "SELECT {BINDING_COLUMNS} FROM admin_user_shop WHERE admin_id = $1 AND shop_id = $2 LIMIT 1"
        ))
        .bind(admin_id)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 批量取会员展示信息（Java `userMapper.selectBatchIds`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn find_users(
        &self,
        user_ids: &[i32],
    ) -> Result<Vec<(i32, String, Option<String>, String, String)>> {
        let ids: Vec<i32> = user_ids.iter().copied().filter(|i| *i > 0).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT user_id, username, nickname, mobile, avatar FROM \"user\" WHERE user_id IN ({ph})"
        ));
        let mut q = sqlx::query_as::<_, (i32, String, Option<String>, String, String)>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量取后台管理员（Java `adminUserMapper.selectBatchIds`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn find_admin_users(
        &self,
        admin_ids: &[i32],
    ) -> Result<Vec<(i32, String, String, String, Option<String>, i32)>> {
        let ids: Vec<i32> = admin_ids.iter().copied().filter(|i| *i > 0).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT admin_id, username, mobile, email, avatar, merchant_id FROM admin_user WHERE admin_id IN ({ph})"
        ));
        let mut q = sqlx::query_as::<_, (i32, String, String, String, Option<String>, i32)>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量取角色（Java `adminRoleMapper.selectBatchIds`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn find_roles(
        &self,
        role_ids: &[i32],
    ) -> Result<Vec<(i32, String, Option<String>)>> {
        let ids: Vec<i32> = role_ids.iter().copied().filter(|i| *i > 0).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT role_id, role_name, authority_list FROM admin_role WHERE role_id IN ({ph})"));
        let mut q = sqlx::query_as::<_, (i32, String, Option<String>)>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 单个角色（`create`/`update` 的可分配校验与 `handleRes`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn find_role(&self, role_id: i32) -> Result<Option<(i32, String, Option<String>)>> {
        let row: Option<(i32, String, Option<String>)> = sqlx::query_as(
            "SELECT role_id, role_name, authority_list FROM admin_role WHERE role_id = $1 LIMIT 1",
        )
        .bind(role_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row)
    }

    /// 该店铺是否已存在此员工（Java `count(eq(shopId).eq(adminId)) > 0`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn count_by_admin_shop(&self, admin_id: i32, shop_id: i32) -> Result<i64> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM admin_user_shop WHERE admin_id = $1 AND shop_id = $2",
        )
        .bind(admin_id)
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(n)
    }

    /// 店铺员工数（用于员工上限校验；Java 排除收银员角色）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn count_staff(&self, shop_id: i32, exclude_role_ids: &[i32]) -> Result<i64> {
        if exclude_role_ids.is_empty() {
            let (n,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM admin_user_shop WHERE shop_id = $1 AND is_admin = 0",
            )
            .bind(shop_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
            return Ok(n);
        }
        let ph = vec!["?"; exclude_role_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT COUNT(*) FROM admin_user_shop WHERE shop_id = $1 AND is_admin = 0 AND role_id NOT IN ({ph})"
        ));
        let mut q = sqlx::query_as::<_, (i64,)>(&sql).bind(shop_id);
        for rid in exclude_role_ids {
            q = q.bind(*rid);
        }
        let (n,): (i64,) = q.fetch_one(&self.pool).await.map_err(map_err)?;
        Ok(n)
    }

    /// 收银员角色 id 列表（Java `posCashierRoleIds`：`role_code = 'pos_cashier'`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn pos_cashier_role_ids(&self) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT role_id FROM admin_role WHERE role_code = 'pos_cashier'",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 插入员工绑定（Java `this.save(adminUsershop)`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
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
        is_admin: i32,
        role_id: i32,
        add_time: i32,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO admin_user_shop (admin_id, user_id, shop_id, username, email, avatar, auth_list, is_using, is_admin, add_time, role_id) VALUES ($1, $2, $3, $4, $5, $6, '[]', $7, $8, $9, $10) RETURNING id",
        )
        .bind(admin_id)
        .bind(user_id)
        .bind(shop_id)
        .bind(username)
        .bind(email)
        .bind(avatar)
        .bind(is_using)
        .bind(is_admin)
        .bind(add_time)
        .bind(role_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 更新员工绑定（Java `updateById(adminUserShop)` 的净效果字段）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    #[allow(clippy::too_many_arguments)]
    pub async fn update_binding(
        &self,
        id: i32,
        username: &str,
        email: &str,
        avatar: &str,
        is_using: i32,
        role_id: i32,
    ) -> Result<bool> {
        let r = sqlx::query(
            "UPDATE admin_user_shop SET username = $1, email = $2, avatar = $3, is_using = $4, role_id = $5, auth_list = '[]' WHERE id = $6",
        )
        .bind(username)
        .bind(email)
        .bind(avatar)
        .bind(is_using)
        .bind(role_id)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 修改头像/邮箱（Java `modifyUser`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn update_avatar_email(&self, id: i32, avatar: &str, email: &str) -> Result<bool> {
        let r = sqlx::query("UPDATE admin_user_shop SET avatar = $1, email = $2 WHERE id = $3")
            .bind(avatar)
            .bind(email)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 删除绑定（Java `removeById`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn delete(&self, id: i32) -> Result<bool> {
        let r = sqlx::query("DELETE FROM admin_user_shop WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 批量删除（Java `batch` 的 `del` 分支）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn delete_many(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let ph = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("DELETE FROM admin_user_shop WHERE id IN ({ph})"));
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        Ok(q.execute(&self.pool).await.map_err(map_err)?.rows_affected())
    }

    // ————————————— admin_user 相关（Java `adminUserMapper`） —————————————

    /// 按手机号查后台账号（Java `selectOne(eq(mobile))`）。
    ///
    /// 返回 `(admin_id, user_id, username)`。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn admin_user_by_mobile(
        &self,
        mobile: &str,
    ) -> Result<Option<(i32, i32, String)>> {
        let row: Option<(i32, i32, String)> = sqlx::query_as(
            "SELECT admin_id, user_id, username FROM admin_user WHERE mobile = $1 LIMIT 1",
        )
        .bind(mobile)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row)
    }

    /// 用户名冲突数（Java `count(eq(username).ne(mobile, mobile))`，create 场景）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn count_username_conflict(&self, username: &str, exclude_mobile: &str) -> Result<i64> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM admin_user WHERE username = $1 AND mobile <> $2",
        )
        .bind(username)
        .bind(exclude_mobile)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(n)
    }

    /// 用户名冲突数（update 场景：`eq(username).ne(adminId, adminId)`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn count_username_conflict_except(
        &self,
        username: &str,
        admin_id: i32,
    ) -> Result<i64> {
        let (n,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM admin_user WHERE username = $1 AND admin_id <> $2")
                .bind(username)
                .bind(admin_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(n)
    }

    /// 手机号冲突数（update 场景）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn count_mobile_conflict_except(&self, mobile: &str, admin_id: i32) -> Result<i64> {
        let (n,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM admin_user WHERE mobile = $1 AND admin_id <> $2")
                .bind(mobile)
                .bind(admin_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(n)
    }

    /// 该管理员是否已绑定其它店铺（Java `count(eq(adminId).ne(shopId))`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn count_other_shop_bindings(&self, admin_id: i32, shop_id: i32) -> Result<i64> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM admin_user_shop WHERE admin_id = $1 AND shop_id <> $2",
        )
        .bind(admin_id)
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(n)
    }

    /// 插入后台账号（Java `adminUserMapper.insert(adminUserSave)`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_admin_user(
        &self,
        username: &str,
        mobile: &str,
        email: &str,
        admin_type: &str,
        merchant_id: i32,
        add_time: i32,
        password_hash: &str,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO admin_user (username, mobile, email, password, admin_type, merchant_id, user_id, role_id, auth_list, add_time) VALUES ($1, $2, $3, $4, $5, $6, 0, 0, '[]', $7) RETURNING admin_id",
        )
        .bind(username)
        .bind(mobile)
        .bind(email)
        .bind(password_hash)
        .bind(admin_type)
        .bind(merchant_id)
        .bind(add_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 更新后台账号手机号/邮箱（Java `adminUserMapper.updateById(...)`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn update_admin_user_mobile_email(
        &self,
        admin_id: i32,
        mobile: &str,
        email: &str,
    ) -> Result<()> {
        sqlx::query("UPDATE admin_user SET mobile = $1, email = $2 WHERE admin_id = $3")
            .bind(mobile)
            .bind(email)
            .bind(admin_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    // ————————————— shop / config —————————————

    /// 店铺所属商户与店铺类型（Java `shopMapper.selectById` 的字段）。
    ///
    /// 返回 `(merchant_id, shop_type)`；店铺不存在 → `None`。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn shop_merchant_and_type(&self, shop_id: i32) -> Result<Option<(i32, Option<i32>)>> {
        let row: Option<(Option<i32>, Option<i32>)> =
            sqlx::query_as("SELECT merchant_id, shop_type FROM shop WHERE shop_id = $1 LIMIT 1")
                .bind(shop_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|(m, t)| (m.unwrap_or(0), t)))
    }

    /// 配置值（Java `configService.getConfigByCode`，员工数上限 `maxSubAdministrator`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn config_value(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = $1 AND (is_del IS NULL OR is_del = 0) LIMIT 1",
        )
        .bind(biz_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }
}

/// 排序字段（Java `buildSortOrder`：空 → 主键列；仅接受白名单列名，未知回退 `id`）。
#[must_use]
fn sort_column(q: &AdminUserShopPageQuery) -> String {
    const SORTABLE: [(&str, &str); 6] = [
        ("id", "id"),
        ("username", "username"),
        ("addTime", "add_time"),
        ("isUsing", "is_using"),
        ("isAdmin", "is_admin"),
        ("roleId", "role_id"),
    ];
    let field = q
        .sort_field
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("id");
    let column = SORTABLE
        .iter()
        .find(|(p, c)| *p == field || *c == field)
        .map_or("id", |(_, c)| *c);
    let dir = if q
        .sort_order
        .as_deref()
        .is_some_and(|o| o.eq_ignore_ascii_case("asc"))
    {
        "ASC"
    } else {
        "DESC"
    };
    format!("{column} {dir}")
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_column_is_whitelisted() {
        let q = AdminUserShopPageQuery::default();
        assert_eq!(sort_column(&q), "id DESC");
        let q = AdminUserShopPageQuery {
            sort_field: Some("addTime".into()),
            sort_order: Some("asc".into()),
            ..Default::default()
        };
        assert_eq!(sort_column(&q), "add_time ASC");
        let q = AdminUserShopPageQuery {
            sort_field: Some("add_time".into()),
            sort_order: Some("desc".into()),
            ..Default::default()
        };
        assert_eq!(sort_column(&q), "add_time DESC");
        // 注入/未知字段 → id
        let q = AdminUserShopPageQuery {
            sort_field: Some("id; DROP TABLE admin_user_shop".into()),
            ..Default::default()
        };
        assert_eq!(sort_column(&q), "id DESC");
    }
}
