//! 账号管理域（identity::admin_account）仓储。
//!
//! 覆盖原 Java `AdminAccountServiceImpl` 用到的 7 张表：
//! - `admin_user`：`admin_id` / `username` / `admin_type` / `mobile` / `avatar` /
//!   `password` / `email` / `add_time` / `auth_list` / `user_id` / `suppliers_id` /
//!   `role_id` / `merchant_id` / `parent_id` / `shop_id` / `is_using` /
//!   `initial_password` / `is_deleted`
//! - `admin_user_shop`：`id` / `admin_id` / `user_id` / `shop_id` / `username` /
//!   `email` / `avatar` / `auth_list` / `is_using` / `is_admin` / `add_time` /
//!   `role_id` / `work_on_time` / `default_pos_login` / `last_login_system` /
//!   `device_code`
//! - `admin_user_vendor`：`admin_id` / `is_admin`
//! - `shop`：`shop_id` / `shop_title` / `shop_logo` / `status` / `add_time` /
//!   `merchant_id` / `shop_type` / `is_delete`
//! - `merchant`：`merchant_id` / `corporate_name`
//! - `user`（会员）：`user_id` / `username` / `avatar` / `mobile` / `email`
//! - `admin_role`：`role_id` / `role_name` / `role_code` / `role_desc` /
//!   `authority_list` / `admin_type` / `merchant_id` / `shop_id` / `vendor_id` /
//!   `is_system`
//!
//! 列名严格对齐 `java/database/pure.sql`
//! （可 `python tools/schema_parser.py --table <表名>` 复核）。
//!
//! 查询统一使用运行时 `sqlx::query_as`（**不使用** `query!` / `query_as!` 宏，
//! 本机无 `DATABASE_URL`）。
//!
//! # 事务
//!
//! `bindMainAccount` 在 Java 上是 `@Transactional`（读主账号 + 可能插/改
//! `admin_user_shop`、初始化 `admin_role`、必要时插入 `admin_user`），
//! 本仓储提供 [`AdminAccountRepository::begin`] + `*_in_tx` 方法，
//! 由服务层把整段逻辑收进同一事务（与 `catalog::price_inquiry` 同一做法）。

use sqlx::{Postgres,PgPool};

use nextrade_core::entities::{AdminRole, AdminUser, AdminUserShop, Merchant, Shop, User};
use nextrade_core::models::identity::admin_account::{
    admin_type, system_super_role_name, AUTH_ALL_JSON,
};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// 系统内置角色描述（Java `AdminRole.builder().roleDesc("系统内置角色")`）。
const SYSTEM_ROLE_DESC: &str = "系统内置角色";

/// 待绑定参数（按 `conds` 顺序 1:1 对应占位符）。
#[derive(Debug, Clone)]
enum Bind {
    I32(i32),
    Str(String),
}

/// 把 `Bind` 列表按顺序绑定到查询上（宏展开以便同时适配不同返回行类型，
/// 与 `repo/identity/admin_user.rs` / `repo/order_admin.rs` 的 `bind_all!` 一致）。
///
/// 定义必须早于首次使用（`macro_rules!` 为文本作用域）。
macro_rules! bind_all {
    ($q:expr, $binds:expr) => {{
        let mut q = $q;
        for b in $binds.iter() {
            q = match b {
                Bind::I32(v) => q.bind(*v),
                Bind::Str(v) => q.bind(v.as_str()),
            };
        }
        q
    }};
}

/// 账号管理域仓储（持有连接池）。
#[derive(Clone)]
pub struct AdminAccountRepository {
    pool: PgPool,
}

impl AdminAccountRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 开启事务（服务层编排 `bindMainAccount` 的多次写入）。
    pub async fn begin(&self) -> Result<sqlx::Transaction<'static, Postgres>> {
        self.pool.begin().await.map_err(map_err)
    }

    // ————————————————————— admin_user 读 —————————————————————

    /// 按主键查询管理员。
    ///
    /// ⚠️ **不过滤 `is_deleted`**：Java `adminUserMapper.selectById` 也没有逻辑删除
    /// 过滤（`AdminUser` 实体无 `@TableLogic`，项目也未开启 mybatis-plus 全局
    /// `logic-delete-field`），因此软删账号仍可被查到（与 Java 一致）。
    pub async fn find_admin(&self, admin_id: i32) -> Result<Option<AdminUser>> {
        sqlx::query_as::<_, AdminUser>("SELECT * FROM admin_user WHERE admin_id = $1 LIMIT 1")
            .bind(admin_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按手机号查询管理员（Java `AdminUserServiceImpl#getAdminUserByUseMobile`）。
    ///
    /// Java：`lambdaQuery().eq(mobile, mobile).one()` —— **无 `is_deleted` 条件**，
    /// 多条命中时 MyBatis `one()` 会抛 `TooManyResultsException`；这里取第一条
    /// （订单金额等无关字段不参与判定），见 `uncertain_points`。
    pub async fn find_admin_by_mobile(&self, mobile: &str) -> Result<Option<AdminUser>> {
        sqlx::query_as::<_, AdminUser>(
            "SELECT * FROM admin_user WHERE mobile = $1 ORDER BY admin_id ASC LIMIT 1",
        )
        .bind(mobile)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按主键查询会员（Java `userMapper.selectById`）。
    pub async fn find_user(&self, user_id: i32) -> Result<Option<User>> {
        sqlx::query_as::<_, User>("SELECT * FROM \"user\" WHERE user_id = $1 LIMIT 1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按主键查询店铺（Java `shopService.getById`）。
    pub async fn find_shop(&self, shop_id: i32) -> Result<Option<Shop>> {
        sqlx::query_as::<_, Shop>("SELECT * FROM shop WHERE shop_id = $1 LIMIT 1")
            .bind(shop_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按主键查询商户（Java `merchantService.getById`）。
    pub async fn find_merchant(&self, merchant_id: i32) -> Result<Option<Merchant>> {
        sqlx::query_as::<_, Merchant>("SELECT * FROM merchant WHERE merchant_id = $1 LIMIT 1")
            .bind(merchant_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 批量按主键查询商户（列表页回填 `merchantName`，避免 N+1）。
    pub async fn find_merchants(&self, merchant_ids: &[i32]) -> Result<Vec<Merchant>> {
        if merchant_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; merchant_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT * FROM merchant WHERE merchant_id IN ({placeholders})"));
        let mut q = sqlx::query_as::<_, Merchant>(&sql);
        for id in merchant_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— admin_user_shop 读 —————————————————————

    /// 查询「指定管理员在指定店铺」的绑定
    /// （Java `bindShopMainAccount` 的 `oldUserShop`：
    /// `eq(shop_id).eq(admin_id)`，`selectOne` 多条会抛异常，这里取第一条）。
    pub async fn find_user_shop(
        &self,
        admin_id: i32,
        shop_id: i32,
    ) -> Result<Option<AdminUserShop>> {
        sqlx::query_as::<_, AdminUserShop>(
            "SELECT * FROM admin_user_shop WHERE admin_id = $1 AND shop_id = $2 \
             ORDER BY id ASC LIMIT 1",
        )
        .bind(admin_id)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 查询店铺主账号绑定（Java `eq(shop_id).eq(is_admin, 1)` + `LIMIT 1`）。
    pub async fn find_shop_main_account(&self, shop_id: i32) -> Result<Option<AdminUserShop>> {
        sqlx::query_as::<_, AdminUserShop>(
            "SELECT * FROM admin_user_shop WHERE shop_id = $1 AND is_admin = 1 \
             ORDER BY id ASC LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 查询某管理员名下的主账号绑定（Java \"deleteAdminUser\"：
    /// `eq(admin_id).eq(is_admin, 1).last("limit 1")`）。
    pub async fn find_admin_main_binding(&self, admin_id: i32) -> Result<Option<AdminUserShop>> {
        sqlx::query_as::<_, AdminUserShop>(
            "SELECT * FROM admin_user_shop WHERE admin_id = $1 AND is_admin = 1 \
             ORDER BY id ASC LIMIT 1",
        )
        .bind(admin_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 某管理员作为主账号关联的全部店铺绑定
    /// （Java `pageShops`：`eq(admin_id).eq(is_admin, 1)`）。
    pub async fn find_admin_main_shops(&self, admin_id: i32) -> Result<Vec<AdminUserShop>> {
        sqlx::query_as::<_, AdminUserShop>(
            "SELECT * FROM admin_user_shop WHERE admin_id = $1 AND is_admin = 1 ORDER BY id ASC",
        )
        .bind(admin_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 批量回填「店铺 → 主账号登录名」（Java \"pageShops\" 里逐个
    /// `selectOne(shop_id, is_admin=1 LIMIT 1)` 再查 \"admin_user\"）。
    ///
    /// 同一店铺若有多条 `is_admin = 1` 绑定（脏数据），按 `id ASC` 取第一条，
    /// 与 Java `LIMIT 1`（无 ORDER BY，实际取存储顺序首行）语义等价。
    pub async fn find_main_account_usernames(
        &self,
        shop_ids: &[i32],
    ) -> Result<Vec<(i32, String)>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; shop_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT admin_user_shop.shop_id, admin_user.username \
             FROM admin_user_shop \
             JOIN admin_user ON admin_user.admin_id = admin_user_shop.admin_id \
             WHERE admin_user_shop.shop_id IN ({placeholders}) \
               AND admin_user_shop.is_admin = 1 \
             ORDER BY admin_user_shop.id ASC"
        ));
        let mut q = sqlx::query_as::<_, (i32, String)>(&sql);
        for id in shop_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 占用校验 —————————————————————

    /// 用户名占用数（排除自身；对应 Java \"updateMainAccount\" 的 \"selectCount\"）。
    pub async fn count_admins_by_username(
        &self,
        username: &str,
        exclude_admin_id: i32,
    ) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM admin_user WHERE LOWER(username) = LOWER($1) AND admin_id <> $2",
        )
        .bind(username)
        .bind(exclude_admin_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 手机号占用数（排除自身）。
    pub async fn count_admins_by_mobile(
        &self,
        mobile: &str,
        exclude_admin_id: i32,
    ) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM admin_user WHERE mobile = $1 AND admin_id <> $2")
                .bind(mobile)
                .bind(exclude_admin_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0)
    }

    /// 邮箱占用数（排除自身）。
    pub async fn count_admins_by_email(&self, email: &str, exclude_admin_id: i32) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM admin_user WHERE LOWER(email) = LOWER($1) AND admin_id <> $2")
                .bind(email)
                .bind(exclude_admin_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0)
    }

    // ————————————————————— 分页 —————————————————————

    /// \"pageShopOrVendor\" 的店铺分页（Java \"pageShops\"）。
    ///
    /// 条件：`shop_id IN (...)` + 可选 `shop_type = $2` + 未删除
    /// （`is_delete IS NULL OR is_delete = 0`）+ 可选 `shop_title ILIKE $3`，
    /// 排序固定 `shop_id DESC`。
    pub async fn paginate_shops(
        &self,
        shop_ids: &[i32],
        shop_type: Option<i32>,
        keyword: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<Shop>)> {
        if shop_ids.is_empty() {
            return Ok((0, Vec::new()));
        }
        let placeholders = vec!["?"; shop_ids.len()].join(", ");
        let mut conds: Vec<String> = vec![format!("shop_id IN ({placeholders})")];
        let mut binds: Vec<Bind> = shop_ids.iter().map(|id| Bind::I32(*id)).collect();
        if let Some(shop_type) = shop_type {
            conds.push("shop_type = ?".to_string());
            binds.push(Bind::I32(shop_type));
        }
        conds.push("(is_delete IS NULL OR is_delete = 0)".to_string());
        if let Some(keyword) = keyword {
            conds.push("shop_title ILIKE ?".to_string());
            binds.push(Bind::Str(format!("%{keyword}%")));
        }
        let where_sql = format!(" WHERE {}", conds.join(" AND "));

        let total_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM shop{where_sql}"));
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&total_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let (limit, offset) = page.limit_offset();
        let rows_sql = crate::db::to_pg_placeholders(&format!("SELECT * FROM shop{where_sql} ORDER BY shop_id DESC LIMIT ? OFFSET ?"));
        let rows = bind_all!(sqlx::query_as::<_, Shop>(&rows_sql), binds)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// \"pageAdminUser\" 的管理员分页（Java \"pageAdminUser\"）。
    ///
    /// 条件：可选 `(username ILIKE $2 OR mobile ILIKE $3)` + `is_deleted = $4`，
    /// 排序固定 `admin_id DESC`。\"is_deleted\" 由控制器恒置 0。
    pub async fn paginate_admin_users(
        &self,
        keyword: Option<&str>,
        is_deleted: i32,
        page: &PageQuery,
    ) -> Result<(u64, Vec<AdminUser>)> {
        let mut conds: Vec<String> = Vec::new();
        let mut binds: Vec<Bind> = Vec::new();
        if let Some(keyword) = keyword {
            let like = format!("%{keyword}%");
            conds.push("(username ILIKE ? OR mobile ILIKE ?)".to_string());
            binds.push(Bind::Str(like.clone()));
            binds.push(Bind::Str(like));
        }
        conds.push("is_deleted = ?".to_string());
        binds.push(Bind::I32(is_deleted));
        let where_sql = format!(" WHERE {}", conds.join(" AND "));

        let total_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM admin_user{where_sql}"));
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&total_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let (limit, offset) = page.limit_offset();
        let rows_sql =
            crate::db::to_pg_placeholders(&format!("SELECT * FROM admin_user{where_sql} ORDER BY admin_id DESC LIMIT ? OFFSET ?"));
        let rows = bind_all!(sqlx::query_as::<_, AdminUser>(&rows_sql), binds)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    // ————————————————————— 聚合计数 —————————————————————

    /// 批量统计每个管理员「主账号关联的店铺/门店数」。
    ///
    /// 对应 Java `AdminUserShopMapper#selectShopCountByAdminIdAndType`：
    /// `SELECT COUNT(1) FROM admin_user_shop LEFT JOIN shop ON ... WHERE admin_id = $2
    /// AND shop_type = $3 AND admin_user_shop.is_admin = 1`。
    /// 这里用一条 `GROUP BY (admin_id, shop_type)` 批量取回（避免 N+1）；
    /// 返回 `(admin_id, shop_type, count)`，服务层只读 \"shop_type\" 1/2。
    pub async fn count_shop_bindings_by_type(
        &self,
        admin_ids: &[i32],
    ) -> Result<Vec<(i32, Option<i32>, i64)>> {
        if admin_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; admin_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT admin_user_shop.admin_id, shop.shop_type, COUNT(*) \
             FROM admin_user_shop \
             LEFT JOIN shop ON shop.shop_id = admin_user_shop.shop_id \
             WHERE admin_user_shop.admin_id IN ({placeholders}) \
               AND admin_user_shop.is_admin = 1 \
             GROUP BY admin_user_shop.admin_id, shop.shop_type"
        ));
        let mut q = sqlx::query_as::<_, (i32, Option<i32>, i64)>(&sql);
        for id in admin_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量统计每个管理员的供应商主账号数（Java `selectCount(admin_user_vendor
    /// where admin_id = $1 and is_admin = 1)`）。
    pub async fn count_vendor_bindings(&self, admin_ids: &[i32]) -> Result<Vec<(i32, i64)>> {
        if admin_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; admin_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT admin_id, COUNT(*) FROM admin_user_vendor \
             WHERE admin_id IN ({placeholders}) AND is_admin = 1 GROUP BY admin_id"
        ));
        let mut q = sqlx::query_as::<_, (i32, i64)>(&sql);
        for id in admin_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— admin_role —————————————————————

    /// 查询系统内置角色（Java `initAdminRole`：
    /// `admin_type = ? AND merchant_id = ? AND shop_id = ? AND is_system = 1`，
    /// `one()` 多行会抛异常，这里取第一条）。
    pub async fn find_system_role(
        &self,
        admin_type: &str,
        merchant_id: i32,
        shop_id: i32,
    ) -> Result<Option<AdminRole>> {
        sqlx::query_as::<_, AdminRole>(
            "SELECT * FROM admin_role WHERE admin_type = $1 AND merchant_id = $2 \
             AND shop_id = $3 AND is_system = 1 ORDER BY role_id ASC LIMIT 1",
        )
        .bind(admin_type)
        .bind(merchant_id)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— 写（无事务，单语句） —————————————————————

    /// 更新账号基本信息（Java \"updateMainAccount\" 的 \"updateById\"：
    /// 最终值 = 读取到的实体 + 本次被赋值的字段）。
    pub async fn update_admin_contact(
        &self,
        admin_id: i32,
        username: &str,
        mobile: Option<&str>,
        email: &str,
    ) -> Result<bool> {
        let affected =
            sqlx::query("UPDATE admin_user SET username = $1, mobile = $2, email = $3 WHERE admin_id = $4")
                .bind(username)
                .bind(mobile)
                .bind(email)
                .bind(admin_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?
                .rows_affected();
        Ok(affected > 0)
    }

    /// 重置管理员密码（Java \"updateMainAccountPwd\" 的 \"updateById\"）。
    pub async fn update_admin_password(&self, admin_id: i32, password_hash: &str) -> Result<bool> {
        let affected = sqlx::query("UPDATE admin_user SET password = $1 WHERE admin_id = $2")
            .bind(password_hash)
            .bind(admin_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected > 0)
    }

    /// 软删除管理员（Java `adminUser.setIsDeleted(1); updateById`）。
    pub async fn soft_delete_admin(&self, admin_id: i32) -> Result<bool> {
        let affected = sqlx::query("UPDATE admin_user SET is_deleted = 1 WHERE admin_id = $1")
            .bind(admin_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected > 0)
    }

    // ————————————————————— 事务内 —————————————————————

    /// 事务内按主键查询管理员。
    pub async fn find_admin_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        admin_id: i32,
    ) -> Result<Option<AdminUser>> {
        sqlx::query_as::<_, AdminUser>("SELECT * FROM admin_user WHERE admin_id = $1 LIMIT 1")
            .bind(admin_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_err)
    }

    /// 事务内按手机号查询管理员。
    pub async fn find_admin_by_mobile_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        mobile: &str,
    ) -> Result<Option<AdminUser>> {
        sqlx::query_as::<_, AdminUser>(
            "SELECT * FROM admin_user WHERE mobile = $1 ORDER BY admin_id ASC LIMIT 1",
        )
        .bind(mobile)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_err)
    }

    /// 事务内按主键查询会员。
    pub async fn find_user_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        user_id: i32,
    ) -> Result<Option<User>> {
        sqlx::query_as::<_, User>("SELECT * FROM \"user\" WHERE user_id = $1 LIMIT 1")
            .bind(user_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_err)
    }

    /// 事务内按主键查询店铺。
    pub async fn find_shop_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        shop_id: i32,
    ) -> Result<Option<Shop>> {
        sqlx::query_as::<_, Shop>("SELECT * FROM shop WHERE shop_id = $1 LIMIT 1")
            .bind(shop_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_err)
    }

    /// 事务内查询「管理员 + 店铺」绑定（`oldUserShop`）。
    pub async fn find_user_shop_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        admin_id: i32,
        shop_id: i32,
    ) -> Result<Option<AdminUserShop>> {
        sqlx::query_as::<_, AdminUserShop>(
            "SELECT * FROM admin_user_shop WHERE admin_id = $1 AND shop_id = $2 \
             ORDER BY id ASC LIMIT 1",
        )
        .bind(admin_id)
        .bind(shop_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_err)
    }

    /// 事务内查询店铺主账号绑定。
    pub async fn find_shop_main_account_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        shop_id: i32,
    ) -> Result<Option<AdminUserShop>> {
        sqlx::query_as::<_, AdminUserShop>(
            "SELECT * FROM admin_user_shop WHERE shop_id = $1 AND is_admin = 1 \
             ORDER BY id ASC LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_err)
    }

    /// 事务内查询系统内置角色。
    pub async fn find_system_role_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        admin_type: &str,
        merchant_id: i32,
        shop_id: i32,
    ) -> Result<Option<AdminRole>> {
        sqlx::query_as::<_, AdminRole>(
            "SELECT * FROM admin_role WHERE admin_type = $1 AND merchant_id = $2 \
             AND shop_id = $3 AND is_system = 1 ORDER BY role_id ASC LIMIT 1",
        )
        .bind(admin_type)
        .bind(merchant_id)
        .bind(shop_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_err)
    }

    /// 事务内「确保店铺系统内置角色存在」，返回 \"role_id\"
    /// （对应 Java `AdminRoleServiceImpl#initAdminRole`）。
    ///
    /// 逻辑逐条对齐 Java：
    /// 1. 按「店型管理员类型 + 商户 + 店铺 + `is_system = 1`」查角色；
    /// 2. 查不到且店型不是 \"shop\" ⇒ 复用同商户/店铺的 \"shop\" 内置角色并改标
    ///    \"admin_type\"（Java \"setAdminType\" + \"updateById\"）；
    /// 3. 仍没有 ⇒ 新建（\"role_name\" 取店型超管名，`authority_list = ["all"]`，
    ///    `role_desc = "系统内置角色"`，`is_system = 1`）。
    pub async fn ensure_system_role_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        admin_type_code: &str,
        merchant_id: i32,
        shop_id: i32,
    ) -> Result<i32> {
        if let Some(role) = self
            .find_system_role_in_tx(tx, admin_type_code, merchant_id, shop_id)
            .await?
        {
            return Ok(role.role_id);
        }
        if admin_type_code != admin_type::SHOP {
            if let Some(role) = self
                .find_system_role_in_tx(tx, admin_type::SHOP, merchant_id, shop_id)
                .await?
            {
                self.update_role_admin_type_in_tx(tx, role.role_id, admin_type_code)
                    .await?;
                return Ok(role.role_id);
            }
        }
        let role = AdminRole {
            role_id: 0,
            role_name: system_super_role_name(admin_type_code)
                .unwrap_or(SYSTEM_ROLE_DESC)
                .to_string(),
            role_code: None,
            role_desc: Some(SYSTEM_ROLE_DESC.to_string()),
            authority_list: AUTH_ALL_JSON.to_string(),
            admin_type: Some(admin_type_code.to_string()),
            merchant_id,
            shop_id,
            vendor_id: 0,
            is_system: 1,
        };
        self.insert_role_in_tx(tx, &role).await
    }

    /// 事务内插入管理员（Java `adminUserService.save(adminUserCreate)`）。
    ///
    /// Java 实体未赋值的字段由 MyBatis-Plus \"NOT_NULL\" 策略**排除出 INSERT**，
    /// 落库取 DDL 默认值；本实现显式绑定同样的默认值（\"0\" / `''`），保证结果一致。
    pub async fn insert_admin_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        admin: &AdminUser,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO admin_user
               (username, admin_type, mobile, avatar, password, email, add_time,
                auth_list, user_id, suppliers_id, role_id, merchant_id, parent_id,
                shop_id, is_using, initial_password, is_deleted)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17) RETURNING admin_id",
        )
        .bind(&admin.username)
        .bind(&admin.admin_type)
        .bind(&admin.mobile)
        .bind(&admin.avatar)
        .bind(&admin.password)
        .bind(&admin.email)
        .bind(admin.add_time)
        .bind(&admin.auth_list)
        .bind(admin.user_id)
        .bind(admin.suppliers_id)
        .bind(admin.role_id)
        .bind(admin.merchant_id)
        .bind(admin.parent_id)
        .bind(admin.shop_id)
        .bind(admin.is_using)
        .bind(&admin.initial_password)
        .bind(admin.is_deleted)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 事务内插入店铺员工/主账号绑定（Java `adminUserShopMapper.insert`）。
    ///
    /// 未赋值的 \"work_on_time\" / \"device_code\" 显式写 \"NULL\"；
    /// \"default_pos_login\" / \"last_login_system\" 取 DDL 默认（0 / 空串）。
    pub async fn insert_user_shop_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        row: &AdminUserShop,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO admin_user_shop
               (admin_id, user_id, shop_id, username, email, avatar, auth_list,
                is_using, is_admin, add_time, role_id, work_on_time, default_pos_login,
                last_login_system, device_code)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15) RETURNING id",
        )
        .bind(row.admin_id)
        .bind(row.user_id)
        .bind(row.shop_id)
        .bind(&row.username)
        .bind(&row.email)
        .bind(&row.avatar)
        .bind(&row.auth_list)
        .bind(row.is_using)
        .bind(row.is_admin)
        .bind(row.add_time)
        .bind(row.role_id)
        .bind(row.work_on_time)
        .bind(row.default_pos_login)
        .bind(&row.last_login_system)
        .bind(&row.device_code)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 事务内整行更新店铺员工/主账号绑定（Java \"updateById\" 的等价实现）。
    pub async fn update_user_shop_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        row: &AdminUserShop,
    ) -> Result<bool> {
        let affected = sqlx::query(
            "UPDATE admin_user_shop SET admin_id = $1, user_id = $2, shop_id = $3,
               username = $4, email = $5, avatar = $6, auth_list = $7, is_using = $8,
               is_admin = $9, add_time = $10, role_id = $11, work_on_time = $12,
               default_pos_login = $13, last_login_system = $14, device_code = $15
             WHERE id = $16",
        )
        .bind(row.admin_id)
        .bind(row.user_id)
        .bind(row.shop_id)
        .bind(&row.username)
        .bind(&row.email)
        .bind(&row.avatar)
        .bind(&row.auth_list)
        .bind(row.is_using)
        .bind(row.is_admin)
        .bind(row.add_time)
        .bind(row.role_id)
        .bind(row.work_on_time)
        .bind(row.default_pos_login)
        .bind(&row.last_login_system)
        .bind(&row.device_code)
        .bind(row.id)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected > 0)
    }

    /// 事务内插入系统内置角色（Java \"initAdminRole\" 的 \"save\"）。
    ///
    /// 未赋值的 \"role_code\" 显式写 \"NULL\"；\"vendor_id\" 取 DDL 默认 0。
    pub async fn insert_role_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        role: &AdminRole,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO admin_role
               (role_name, role_code, role_desc, authority_list, admin_type,
                merchant_id, shop_id, vendor_id, is_system)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING role_id",
        )
        .bind(&role.role_name)
        .bind(&role.role_code)
        .bind(&role.role_desc)
        .bind(&role.authority_list)
        .bind(&role.admin_type)
        .bind(role.merchant_id)
        .bind(role.shop_id)
        .bind(role.vendor_id)
        .bind(role.is_system)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 事务内把「店铺内置角色」改标为门店/自提点（Java \"initAdminRole\"：
    /// `adminRole.setAdminType(adminType); updateById`）。
    pub async fn update_role_admin_type_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        role_id: i32,
        admin_type: &str,
    ) -> Result<bool> {
        let affected = sqlx::query("UPDATE admin_role SET admin_type = $1 WHERE role_id = $2")
            .bind(admin_type)
            .bind(role_id)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected > 0)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("账号数据不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_enum_carries_int_and_string_values() {
        let binds = vec![Bind::I32(3), Bind::Str("%kw%".into())];
        match (&binds[0], &binds[1]) {
            (Bind::I32(v), Bind::Str(s)) => {
                assert_eq!(*v, 3);
                assert_eq!(s, "%kw%");
            }
            other => panic!("绑定类型不符: {other:?}"),
        }
    }

    /// 分页 SQL 的占位符数量必须与绑定数量一致（防运行期错位）。
    #[test]
    fn paginate_shops_placeholder_count_matches_binds() {
        // 复刻 paginate_shops 的条件拼装逻辑（不触库）。
        let shop_ids = [11, 22, 33];
        let placeholders = vec!["?"; shop_ids.len()].join(", ");
        let mut conds: Vec<String> = vec![format!("shop_id IN ({placeholders})")];
        let mut binds: Vec<Bind> = shop_ids.iter().map(|id| Bind::I32(*id)).collect();
        conds.push("shop_type = ?".to_string());
        binds.push(Bind::I32(1));
        conds.push("(is_delete IS NULL OR is_delete = 0)".to_string());
        conds.push("shop_title ILIKE ?".to_string());
        binds.push(Bind::Str("%店%".into()));
        let where_sql = format!(" WHERE {}", conds.join(" AND "));
        assert_eq!(where_sql.matches('?').count(), binds.len());
        assert!(where_sql.contains("shop_id IN (?, ?, ?)"));
        assert!(where_sql.contains("(is_delete IS NULL OR is_delete = 0)"));
    }

    /// \"paginate_admin_users\" 的 WHERE 片段：缺省只有 `is_deleted = $1`。
    #[test]
    fn paginate_admin_users_where_shape() {
        let mut conds: Vec<String> = Vec::new();
        let mut binds: Vec<Bind> = Vec::new();
        conds.push("is_deleted = ?".to_string());
        binds.push(Bind::I32(0));
        let where_sql = format!(" WHERE {}", conds.join(" AND "));
        assert_eq!(where_sql, " WHERE is_deleted = ?");
        assert_eq!(where_sql.matches('?').count(), binds.len());

        let like = "%root%".to_string();
        let mut conds: Vec<String> = vec!["(username ILIKE ? OR mobile ILIKE ?)".to_string()];
        let mut binds: Vec<Bind> = vec![Bind::Str(like.clone()), Bind::Str(like)];
        conds.push("is_deleted = ?".to_string());
        binds.push(Bind::I32(0));
        let where_sql = format!(" WHERE {}", conds.join(" AND "));
        assert_eq!(where_sql.matches('?').count(), binds.len());
    }
}
