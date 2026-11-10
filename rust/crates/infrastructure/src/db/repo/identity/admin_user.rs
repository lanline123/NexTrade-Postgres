//! 管理员列表域（identity::admin_user）仓储：`admin_user` / `admin_api_token` /
//! `admin_role` / `admin_user_shop` / `shop` / `merchant`。
//!
//! 列名严格对齐 `java/database/pure.sql`
//! （`python tools/schema_parser.py --table <表名>`）：
//! - `admin_user`：`admin_id` / `username` / `admin_type` / `mobile` / `avatar` /
//!   `password` / `email` / `add_time` / `auth_list` / `user_id` / `suppliers_id` /
//!   `role_id` / `merchant_id` / `parent_id` / `menu_tag` / `order_export` /
//!   `extra` / `shop_id` / `is_using` / `initial_password` / `is_deleted`
//! - `admin_api_token`：`id` / `admin_id` / `token_hash` / `token_prefix` / `name` /
//!   `created_at` / `last_used_at` / `expires_at` / `is_active`
//! - `admin_role`：`role_id` / `role_name` / `role_code` / `role_desc` /
//!   `authority_list` / `admin_type` / `merchant_id` / `shop_id` / `vendor_id` /
//!   `is_system`
//! - `admin_user_shop`：`id` / `admin_id` / `user_id` / `shop_id` / `username` /
//!   `email` / `avatar` / `auth_list` / `is_using` / `is_admin` / `add_time` /
//!   `role_id`
//! - `shop`：`shop_id` / `shop_title` / `shop_logo` / `status` / `contact_mobile` /
//!   `add_time` / `merchant_id`
//! - `merchant`：`merchant_id`
//!
//! 查询统一使用运行时 `sqlx::query_as`（**不使用** `query!` / `query_as!` 宏，
//! 本机无 `DATABASE_URL`）。

use sqlx::PgPool;
use nextrade_core::entities::{AdminApiToken, AdminRole, AdminUser, AdminUserShop, Merchant, Shop};
use nextrade_core::{Error, Result};

/// 一次 `admin_user` 分页查询的筛选条件
/// （对应 Java `AdminUserPageQuery` + `AdminUserServiceImpl#list`）。
#[derive(Debug, Clone, Default)]
pub struct AdminUserPageFilter {
    /// 关键词：`username ILIKE %kw% OR mobile ILIKE %kw%`
    /// （Java `StrUtil.isNotBlank(keyword)`）
    pub keyword: Option<String>,
    /// `admin_type IN (...)`；空表示不过滤
    /// （Java `in(ObjectUtil.isNotEmpty(adminUserType), AdminUser::getAdminType, types)`）
    pub admin_types: Vec<String>,
    /// `suppliers_id = ?`（Java 是 `String`，与 `int unsigned` 列比较）
    pub suppliers_id: Option<String>,
    /// 租户店铺过滤：店铺账号只能看到自己店铺的管理员；平台为 None
    pub shop_id: Option<i32>,
    /// 每页条数（已按 `BasePage` 口径截断为 1..=100）
    pub page_size: u64,
    /// 偏移量
    pub offset: u64,
}

/// `admin_role` 查询的租户归属条件（对应 Java `AdminUserServiceImpl#getConfig`）。
#[derive(Debug, Clone, Default)]
pub struct RoleScopeFilter {
    /// `admin_role.shop_id = ?`（平台分支用 `HeaderUtils.getShopId()`）
    pub shop_id: Option<i32>,
    /// `admin_role.merchant_id = ?`
    pub merchant_id: Option<i32>,
    /// `admin_role.vendor_id = ?`
    pub vendor_id: Option<i32>,
    /// 店铺账号分支：`(merchant_id = ? AND shop_id = 0) OR shop_id = ?`
    /// （Java：`(merchantId 匹配 AND shop_id = 0) OR shop_id = 当前店铺`）。
    /// 置位时**忽略**上面三个单条件字段。
    pub merchant_shop_branch: Option<(i32, i32)>,
}

/// 待绑定参数（按 `conds` 顺序 1:1 对应占位符）。
#[derive(Debug, Clone)]
enum Bind {
    I32(i32),
    I64(i64),
    Str(String),
}

/// 把 `Bind` 列表按顺序绑定到查询上（宏展开以便同时适配不同返回行类型，
/// 与 `repo/order_admin.rs` / `repo/authority.rs` 的 `bind_all!` 做法一致）。
///
/// 定义必须早于首次使用（`macro_rules!` 为文本作用域）。
macro_rules! bind_all {
    ($q:expr, $binds:expr) => {{
        let mut q = $q;
        for b in $binds.iter() {
            q = match b {
                Bind::I32(v) => q.bind(*v),
                Bind::I64(v) => q.bind(*v),
                Bind::Str(v) => q.bind(v.as_str()),
            };
        }
        q
    }};
}

#[derive(Clone)]
pub struct AdminUserRepository {
    pool: PgPool,
}

impl AdminUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— admin_user 查询 —————————————————————

    /// 按主键查询（不区分是否软删除，Java `getById`）。
    pub async fn find_user(&self, admin_id: i32) -> Result<Option<AdminUser>> {
        sqlx::query_as::<_, AdminUser>("SELECT * FROM admin_user WHERE admin_id = $1 LIMIT 1")
            .bind(admin_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 用户名占用检查。
    ///
    /// `deleted_scope`：
    /// - `Some(false)` → 仅未删除（Java `create`：
    ///   `isNull(isDeleted).or().eq(isDeleted, 0)`）
    /// - `None` → 不区分删除（Java `buildAdminUser`：无 `is_deleted` 条件）
    pub async fn find_by_username(
        &self,
        username: &str,
        deleted_scope: Option<bool>,
    ) -> Result<Option<AdminUser>> {
        let sql = if deleted_scope.is_some() {
            "SELECT * FROM admin_user WHERE LOWER(username) = LOWER($1) AND is_deleted = 0 \
             ORDER BY admin_id DESC LIMIT 1"
        } else {
            "SELECT * FROM admin_user WHERE LOWER(username) = LOWER($1) \
             ORDER BY admin_id DESC LIMIT 1"
        };
        sqlx::query_as::<_, AdminUser>(sql)
            .bind(username)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 手机号占用检查（\"exclude_admin_id\" 用于 \"update\" 排除自身）。
    ///
    /// \"deleted_scope\" 语义同 [`Self::find_by_username`]。
    pub async fn find_by_mobile(
        &self,
        mobile: &str,
        deleted_scope: Option<bool>,
        exclude_admin_id: Option<i32>,
    ) -> Result<Option<AdminUser>> {
        let mut sql = String::from("SELECT * FROM admin_user WHERE mobile = $1");
        let mut binds = vec![Bind::Str(mobile.to_string())];
        if deleted_scope.is_some() {
            sql.push_str(" AND is_deleted = 0");
        }
        if let Some(id) = exclude_admin_id {
            sql.push_str(" AND admin_id <> ?");
            binds.push(Bind::I32(id));
        }
        sql.push_str(" ORDER BY admin_id DESC LIMIT 1");
        let sql = crate::db::to_pg_placeholders(&sql);
        bind_all!(sqlx::query_as::<_, AdminUser>(&sql), binds)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 查询「已软删除」的同手机号账号（Java
    /// \"create\"：`eq(mobile).eq(isDeleted, 1)`），用于复用账号。
    pub async fn find_deleted_by_mobile(&self, mobile: &str) -> Result<Option<AdminUser>> {
        sqlx::query_as::<_, AdminUser>(
            "SELECT * FROM admin_user WHERE mobile = $1 AND is_deleted = 1 \
             ORDER BY admin_id DESC LIMIT 1",
        )
        .bind(mobile)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 邮箱占用检查（\"exclude_admin_id\" 用于 \"update\" 排除自身）。
    ///
    /// 对应 Java \"buildAdminUser\" 的 \"emailCount\" 校验（**不排除已软删除账号**）。
    pub async fn find_by_email(
        &self,
        email: &str,
        exclude_admin_id: Option<i32>,
    ) -> Result<Option<AdminUser>> {
        let mut sql = String::from("SELECT * FROM admin_user WHERE LOWER(email) = LOWER($1)");
        let mut binds = vec![Bind::Str(email.to_string())];
        if let Some(id) = exclude_admin_id {
            sql.push_str(" AND admin_id <> ?");
            binds.push(Bind::I32(id));
        }
        sql.push_str(" ORDER BY admin_id DESC LIMIT 1");
        let sql = crate::db::to_pg_placeholders(&sql);
        bind_all!(sqlx::query_as::<_, AdminUser>(&sql), binds)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 修改密码（对应 Java `AdminUserServiceImpl#modifyManageAccounts`
    /// modifyType=2 的 \"updateById\"）。
    pub async fn update_password(&self, admin_id: i32, password_hash: &str) -> Result<bool> {
        let affected = sqlx::query("UPDATE admin_user SET password = $1 WHERE admin_id = $2")
            .bind(password_hash)
            .bind(admin_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected > 0)
    }

    /// 修改个人信息（对应 Java \"modifyManageAccounts\" modifyType=1：
    /// 只更新 \"avatar\" / \"email\"，值可为 \"NULL\"）。
    pub async fn update_profile(
        &self,
        admin_id: i32,
        avatar: Option<&str>,
        email: Option<&str>,
    ) -> Result<bool> {
        let affected =
            sqlx::query("UPDATE admin_user SET avatar = $1, email = $2 WHERE admin_id = $3")
                .bind(avatar)
                .bind(email)
                .bind(admin_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?
                .rows_affected();
        Ok(affected > 0)
    }

    /// 修改当前管理员手机号（Java \"modifyManageAccounts\" modifyType=3）。
    pub async fn update_mobile(&self, admin_id: i32, mobile: &str) -> Result<bool> {
        let affected = sqlx::query("UPDATE admin_user SET mobile = $1 WHERE admin_id = $2")
            .bind(mobile)
            .bind(admin_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected > 0)
    }

    /// 管理员分页（对应 Java `AdminUserServiceImpl#list` 的
    /// `lambdaQuery()...page(...)`）。
    ///
    /// 排序固定 `admin_id DESC`（Java `BaseServiceImpl#buildSortOrder` 在
    /// \"sortField\" 为空/非法时回退主键降序，与 `repo/order_admin.rs` 口径一致）。
    pub async fn paginate_users(&self, f: &AdminUserPageFilter) -> Result<(u64, Vec<AdminUser>)> {
        let (where_sql, binds) = build_user_filter(f);

        let total_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM admin_user{where_sql}"));
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&total_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM admin_user{where_sql} ORDER BY admin_id DESC LIMIT ? OFFSET ?"
        ));
        let rows = bind_all!(sqlx::query_as::<_, AdminUser>(&rows_sql), binds)
            .bind(f.page_size as i64)
            .bind(f.offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 新增管理员，返回自增 `admin_id`。
    ///
    /// 显式列出全部可写列（不依赖库默认值）：`auth_list` 由调用方置为 `"[]"`，
    /// \"initial_password\" 置空串，其余计数/标记列置 0，与 Java \"buildAdminUser\"
    /// + \"save\" 的落库结果一致。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_user(
        &self,
        username: &str,
        admin_type: &str,
        mobile: Option<&str>,
        avatar: &str,
        password_hash: &str,
        email: &str,
        add_time: i32,
        role_id: i32,
        suppliers_id: i32,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO admin_user
               (username, admin_type, mobile, avatar, password, email, add_time,
                auth_list, user_id, suppliers_id, role_id, merchant_id, parent_id,
                menu_tag, order_export, extra, shop_id, is_using, initial_password,
                is_deleted)
             VALUES ($1, $2, $3, $4, $5, $6, $7, '[]', 0, $8, $9, 0, 0, NULL, NULL, NULL, 0, 0, '', 0) RETURNING admin_id",
        )
        .bind(username)
        .bind(admin_type)
        .bind(mobile)
        .bind(avatar)
        .bind(password_hash)
        .bind(email)
        .bind(add_time)
        .bind(suppliers_id)
        .bind(role_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 按主键整行更新（对应 Java `BaseServiceImpl#updateById`）。
    ///
    /// Java 的 \"updateById\" 只写非 null 字段（MyBatis-Plus 默认策略），
    /// 服务层已按同样语义构造出「最终行」，这里的 \"UPDATE\" 覆盖全部可写列。
    pub async fn update_user(&self, u: &AdminUser) -> Result<bool> {
        let affected = sqlx::query(
            "UPDATE admin_user SET username = $1, admin_type = $2, mobile = $3, avatar = $4,
               password = $5, email = $6, add_time = $7, auth_list = $8, user_id = $9,
               suppliers_id = $10, role_id = $11, merchant_id = $12, parent_id = $13,
               menu_tag = $14, order_export = $15, extra = $16, shop_id = $17, is_using = $18,
               is_deleted = $19
             WHERE admin_id = $20",
        )
        .bind(&u.username)
        .bind(&u.admin_type)
        .bind(&u.mobile)
        .bind(&u.avatar)
        .bind(&u.password)
        .bind(&u.email)
        .bind(u.add_time)
        .bind(&u.auth_list)
        .bind(u.user_id)
        .bind(u.suppliers_id)
        .bind(u.role_id)
        .bind(u.merchant_id)
        .bind(u.parent_id)
        .bind(&u.menu_tag)
        .bind(&u.order_export)
        .bind(&u.extra)
        .bind(u.shop_id)
        .bind(u.is_using)
        .bind(u.is_deleted)
        .bind(u.admin_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected > 0)
    }

    /// 软删除（`is_deleted = 1`，对应 Java \"removeById\"，见
    /// \"uncertain_points\"：Java 依赖 MyBatis-Plus 逻辑删除注解，本实现显式置位）。
    pub async fn soft_delete_user(&self, admin_id: i32) -> Result<bool> {
        let affected = sqlx::query("UPDATE admin_user SET is_deleted = 1 WHERE admin_id = $1")
            .bind(admin_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected > 0)
    }

    // ————————————————————— admin_role —————————————————————

    /// 按主键查询角色。
    pub async fn find_role(&self, role_id: i32) -> Result<Option<AdminRole>> {
        sqlx::query_as::<_, AdminRole>("SELECT * FROM admin_role WHERE role_id = $1 LIMIT 1")
            .bind(role_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 批量按主键查询角色（`list` 回填 `roleName`，消除 N+1）。
    pub async fn find_roles(&self, role_ids: &[i32]) -> Result<Vec<AdminRole>> {
        if role_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; role_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT * FROM admin_role WHERE role_id IN ({placeholders})"));
        let mut q = sqlx::query_as::<_, AdminRole>(&sql);
        for id in role_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 按租户归属查询「普通角色」（对应 Java
    /// `AdminUserServiceImpl#getConfig` + `listOrdinaryRoles`）。
    ///
    /// 说明：Java 的 `listOrdinaryRoles` 条件为
    /// `role_code IS NULL OR role_code NOT IN ('pos_cashier','kefu_agent')`。
    /// MySQL 里 `NOT IN` 遇到 NULL **不匹配**，所以「`role_code` 为 NULL」的角色
    /// 必须靠 `IS NULL` 分支放行——本实现按该语义生成
    /// `role_code IS NULL OR role_code NOT IN (...)` 并在 SQL 中显式列举常量，
    /// 与 Java 逐字一致。
    pub async fn find_ordinary_roles(&self, scope: &RoleScopeFilter) -> Result<Vec<AdminRole>> {
        let mut conds: Vec<String> = Vec::new();
        let mut binds: Vec<Bind> = Vec::new();

        if let Some((merchant_id, shop_id)) = scope.merchant_shop_branch {
            // 店铺账号：商户级（shop_id = 0）角色 OR 本店铺角色
            conds.push("((merchant_id = ? AND shop_id = 0) OR shop_id = ?)".to_string());
            binds.push(Bind::I32(merchant_id));
            binds.push(Bind::I32(shop_id));
        } else {
            for (col, value) in [
                ("shop_id", scope.shop_id),
                ("merchant_id", scope.merchant_id),
                ("vendor_id", scope.vendor_id),
            ] {
                if let Some(v) = value {
                    conds.push(format!("{col} = ?"));
                    binds.push(Bind::I32(v));
                }
            }
        }

        let where_sql = if conds.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conds.join(" AND "))
        };
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM admin_role{where_sql} {} ORDER BY role_id DESC",
            role_ordinary_condition(&where_sql)
        ));
        bind_all!(sqlx::query_as::<_, AdminRole>(&sql), binds)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    // ————————————————————— admin_user_shop —————————————————————

    /// 查询「主账号」绑定（Java \"adminUserDel\"：
    /// `admin_id = ? AND is_admin = 1 LIMIT 1`）。
    pub async fn find_shop_main_account(&self, admin_id: i32) -> Result<Option<AdminUserShop>> {
        sqlx::query_as::<_, AdminUserShop>(
            "SELECT * FROM admin_user_shop WHERE admin_id = $1 AND is_admin = 1 LIMIT 1",
        )
        .bind(admin_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 查询指定管理员的全部店铺绑定（`detail` 的 `userShop`）。
    pub async fn find_user_shops(&self, admin_id: i32) -> Result<Vec<AdminUserShop>> {
        sqlx::query_as::<_, AdminUserShop>(
            "SELECT * FROM admin_user_shop WHERE admin_id = $1 ORDER BY id ASC",
        )
        .bind(admin_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 查询平台工作台绑定（Java `syncPlatformWorkbenchBinding` 中的
    /// `admin_id = ? AND shop_id = 0`）。
    pub async fn find_platform_binding(&self, admin_id: i32) -> Result<Option<AdminUserShop>> {
        sqlx::query_as::<_, AdminUserShop>(
            "SELECT * FROM admin_user_shop WHERE admin_id = $1 AND shop_id = 0 LIMIT 1",
        )
        .bind(admin_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 删除平台工作台绑定（Java \"removePlatformWorkbenchBinding\" 的 SQL 部分）。
    pub async fn delete_platform_binding(&self, admin_id: i32) -> Result<u64> {
        let affected = sqlx::query("DELETE FROM admin_user_shop WHERE admin_id = $1 AND shop_id = 0")
            .bind(admin_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected)
    }

    /// 新增店铺员工绑定，返回自增 \"id\"（Java `adminUserShopMapper.insert`）。
    pub async fn insert_user_shop(
        &self,
        b: &AdminUserShop,
        add_time: i32,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO admin_user_shop
               (admin_id, user_id, shop_id, username, email, avatar, auth_list,
                is_using, is_admin, add_time, role_id, work_on_time, default_pos_login,
                last_login_system, device_code)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15) RETURNING id",
        )
        .bind(b.admin_id)
        .bind(b.user_id)
        .bind(b.shop_id)
        .bind(&b.username)
        .bind(&b.email)
        .bind(&b.avatar)
        .bind(&b.auth_list)
        .bind(b.is_using)
        .bind(b.is_admin)
        .bind(add_time)
        .bind(b.role_id)
        .bind(b.work_on_time)
        .bind(b.default_pos_login)
        .bind(&b.last_login_system)
        .bind(&b.device_code)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 按主键更新店铺员工绑定（Java `adminUserShopMapper.updateById`）。
    pub async fn update_user_shop(&self, b: &AdminUserShop) -> Result<bool> {
        let affected = sqlx::query(
            "UPDATE admin_user_shop SET admin_id = $1, user_id = $2, shop_id = $3,
               username = $4, email = $5, avatar = $6, auth_list = $7, is_using = $8,
               is_admin = $9, add_time = $10, role_id = $11, work_on_time = $12,
               default_pos_login = $13, last_login_system = $14, device_code = $15
             WHERE id = $16",
        )
        .bind(b.admin_id)
        .bind(b.user_id)
        .bind(b.shop_id)
        .bind(&b.username)
        .bind(&b.email)
        .bind(&b.avatar)
        .bind(&b.auth_list)
        .bind(b.is_using)
        .bind(b.is_admin)
        .bind(b.add_time)
        .bind(b.role_id)
        .bind(b.work_on_time)
        .bind(b.default_pos_login)
        .bind(&b.last_login_system)
        .bind(&b.device_code)
        .bind(b.id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected > 0)
    }

    // ————————————————————— shop / merchant —————————————————————

    /// 按主键查询店铺（`getConfig` 店铺分支 / `detail` 的 `userShop`）。
    pub async fn find_shop(&self, shop_id: i32) -> Result<Option<Shop>> {
        sqlx::query_as::<_, Shop>("SELECT * FROM shop WHERE shop_id = $1 LIMIT 1")
            .bind(shop_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 批量按主键查询店铺。
    pub async fn find_shops(&self, shop_ids: &[i32]) -> Result<Vec<Shop>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; shop_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT * FROM shop WHERE shop_id IN ({placeholders})"));
        let mut q = sqlx::query_as::<_, Shop>(&sql);
        for id in shop_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 按主键查询商户。
    pub async fn find_merchant(&self, merchant_id: i32) -> Result<Option<Merchant>> {
        sqlx::query_as::<_, Merchant>("SELECT * FROM merchant WHERE merchant_id = $1 LIMIT 1")
            .bind(merchant_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    // ————————————————————— admin_api_token —————————————————————

    /// 该管理员的有效 Token 数量（Java `generateToken` 的上限校验）。
    pub async fn count_active_tokens(&self, admin_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM admin_api_token WHERE admin_id = $1 AND is_active = 1",
        )
        .bind(admin_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 新增 API Token，返回自增 \"id\"。
    pub async fn insert_api_token(
        &self,
        admin_id: i32,
        token_hash: &str,
        token_prefix: &str,
        name: &str,
        created_at: i64,
        expires_at: Option<i64>,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO admin_api_token
               (admin_id, token_hash, token_prefix, name, created_at, last_used_at,
                expires_at, is_active)
             VALUES ($1, $2, $3, $4, $5, NULL, $6, 1) RETURNING id",
        )
        .bind(admin_id)
        .bind(token_hash)
        .bind(token_prefix)
        .bind(name)
        .bind(created_at)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 列出该管理员的全部 Token（Java \"listTokens\"：\"created_at\" 降序）。
    pub async fn find_api_tokens(&self, admin_id: i32) -> Result<Vec<AdminApiToken>> {
        sqlx::query_as::<_, AdminApiToken>(
            "SELECT * FROM admin_api_token WHERE admin_id = $1 ORDER BY created_at DESC",
        )
        .bind(admin_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按主键查询 Token（`revoke` 的归属校验）。
    pub async fn find_api_token(&self, id: i32) -> Result<Option<AdminApiToken>> {
        sqlx::query_as::<_, AdminApiToken>(
            "SELECT * FROM admin_api_token WHERE id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 吊销 Token（Java `revokeToken`：`is_active = 0`），返回是否命中。
    pub async fn revoke_api_token(&self, id: i32) -> Result<bool> {
        let affected = sqlx::query("UPDATE admin_api_token SET is_active = 0 WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected > 0)
    }

    /// 按 SHA-256 哈希取有效 Token（Java `validateToken`：激活且未过期），
    /// 供 `X-Api-Token` 认证中间件使用。
    pub async fn find_active_api_token_by_hash(
        &self,
        token_hash: &str,
        now: i64,
    ) -> Result<Option<AdminApiToken>> {
        sqlx::query_as::<_, AdminApiToken>(
            "SELECT * FROM admin_api_token              WHERE token_hash = $1 AND is_active = 1                AND (expires_at IS NULL OR expires_at > $2)              LIMIT 1",
        )
        .bind(token_hash)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 回写 Token 最后使用时间（Java `validateToken` 内联 touch）。
    pub async fn touch_api_token(&self, id: i32, now: i64) -> Result<()> {
        sqlx::query("UPDATE admin_api_token SET last_used_at = $1 WHERE id = $2")
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

/// 「普通角色」过滤片段。
///
/// Java \"listOrdinaryRoles\" 把该条件**追加在租户条件之后**（`.and(qw -> ...)`），
/// 故此处用 `AND (...)` 拼接；无租户条件时降级为 `WHERE (...)`。
fn role_ordinary_condition(where_sql: &str) -> String {
    let cond = "role_code IS NULL OR role_code NOT IN ('pos_cashier', 'kefu_agent')";
    if where_sql.is_empty() {
        format!("WHERE ({cond})")
    } else {
        format!("AND ({cond})")
    }
}

/// 构造 \"admin_user\" 的 WHERE 子句（含前导空格）与绑定参数。
///
/// 对照 Java `AdminUserServiceImpl#list`：
/// - \"keyword\"：`username ILIKE %kw% OR mobile ILIKE %kw%`
/// - \"adminUserType\" 非空时：`admin_type IN (...)`
/// - 恒有 `is_deleted = 0`
/// - \"suppliersId\" 非空时：`suppliers_id = $1`
fn build_user_filter(f: &AdminUserPageFilter) -> (String, Vec<Bind>) {
    let mut conds: Vec<String> = vec!["is_deleted = 0".to_string()];
    let mut binds: Vec<Bind> = Vec::new();

    if let Some(kw) = f
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let like = format!("%{kw}%");
        conds.push("(username ILIKE ? OR mobile ILIKE ?)".to_string());
        binds.push(Bind::Str(like.clone()));
        binds.push(Bind::Str(like));
    }

    if !f.admin_types.is_empty() {
        let placeholders = vec!["?"; f.admin_types.len()].join(", ");
        conds.push(format!("admin_type IN ({placeholders})"));
        for t in &f.admin_types {
            binds.push(Bind::Str(t.clone()));
        }
    }

    if let Some(sid) = f
        .suppliers_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        conds.push("suppliers_id = ?".to_string());
        binds.push(Bind::Str(sid.to_string()));
    }

    if let Some(shop_id) = f.shop_id {
        conds.push("shop_id = ?".to_string());
        binds.push(Bind::I32(shop_id));
    }

    (format!(" WHERE {}", conds.join(" AND ")), binds)
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("管理员不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_filter_defaults_to_not_deleted() {
        let f = AdminUserPageFilter {
            page_size: 15,
            ..Default::default()
        };
        let (sql, binds) = build_user_filter(&f);
        assert_eq!(sql, " WHERE is_deleted = 0");
        assert!(binds.is_empty());
    }

    #[test]
    fn user_filter_binds_match_placeholders() {
        let f = AdminUserPageFilter {
            keyword: Some("root".into()),
            admin_types: vec!["admin".into(), "suppliers".into()],
            suppliers_id: Some("7".into()),
            shop_id: None,
            page_size: 15,
            offset: 0,
        };
        let (sql, binds) = build_user_filter(&f);
        assert!(sql.contains("is_deleted = 0"));
        assert!(sql.contains("(username ILIKE ? OR mobile ILIKE ?)"));
        assert!(sql.contains("admin_type IN (?, ?)"));
        assert!(sql.contains("suppliers_id = ?"));
        assert_eq!(sql.matches('?').count(), binds.len());
        // 绑定顺序：keyword ×2 → adminType ×2 → suppliersId
        match (&binds[0], &binds[1], &binds[2], &binds[4]) {
            (Bind::Str(a), Bind::Str(b), Bind::Str(c), Bind::Str(d)) => {
                assert_eq!(a, "%root%");
                assert_eq!(b, "%root%");
                assert_eq!(c, "admin");
                assert_eq!(d, "7");
            }
            other => panic!("绑定类型不符: {other:?}"),
        }
    }

    #[test]
    fn blank_keyword_and_suppliers_id_are_ignored() {
        let f = AdminUserPageFilter {
            keyword: Some("   ".into()),
            admin_types: vec![],
            suppliers_id: Some("".into()),
            shop_id: None,
            page_size: 15,
            offset: 0,
        };
        let (sql, binds) = build_user_filter(&f);
        assert_eq!(sql, " WHERE is_deleted = 0");
        assert!(binds.is_empty());
    }

    #[test]
    fn shop_scope_adds_shop_filter_bind() {
        let f = AdminUserPageFilter {
            shop_id: Some(9),
            page_size: 15,
            ..Default::default()
        };
        let (sql, binds) = build_user_filter(&f);
        assert!(sql.contains("shop_id = ?"));
        assert_eq!(binds.len(), 1);
        assert!(matches!(binds[0], Bind::I32(9)));
    }

    #[test]
    fn ordinary_role_condition_keeps_java_is_null_branch() {
        // 有租户条件时用 AND 追加，且必须保留 role_code IS NULL 分支
        let c = role_ordinary_condition(" WHERE shop_id = $1");
        assert!(c.starts_with("AND ("));
        assert!(c.contains("role_code IS NULL"));
        assert!(c.contains("NOT IN ('pos_cashier', 'kefu_agent')"));

        // 无租户条件时降级为 WHERE
        let c = role_ordinary_condition("");
        assert!(c.starts_with("WHERE ("));
    }
}
