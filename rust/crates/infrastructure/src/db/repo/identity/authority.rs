//! 权限管理域（identity::authority）仓储：`authority` / `admin_user_shop` /
//! `admin_user_vendor` / `admin_role`。
//!
//! 列名严格对齐 `java/database/pure.sql`
//! （`python tools/schema_parser.py --table authority`）：
//! `authority_id` / `authority_sn` / `authority_name` / `parent_id` /
//! `sort_order` / `is_show` / `child_auth` / `route_link` /
//! `authority_ico` / `is_system` / `admin_type`。
//!
//! `admin_user_shop`：`id` / `admin_id` / `shop_id` / `auth_list` / `role_id`；
//! `admin_user_vendor`：`id` / `admin_id` / `vendor_id` / `auth_list` / `role_id`；
//! `admin_role`：`role_id` / `authority_list`。
//!
//! 查询统一使用运行时 `sqlx::query_as`（不使用 `query!` 宏，本机无 `DATABASE_URL`）。
//! `authority` 实体（`nextrade_core::entities::Authority`）由 schema 自动生成，
//! 可直接作为 `FromRow` 目标。

use sqlx::{PgPool, QueryBuilder};
use nextrade_core::entities::Authority;
use nextrade_core::{Error, Result};

/// `authority` 表全列清单（显式列举，避免依赖 `SELECT *` 的列序）。
const AUTHORITY_SELECT: &str = "authority_id, authority_sn, authority_name, parent_id, \
     sort_order, is_show, child_auth, route_link, authority_ico, is_system, admin_type";

/// \"list\" 的 \"WHERE\" 绑定值。
#[derive(Debug, Clone)]
enum Bind {
    I32(i32),
    Str(String),
}

/// 把 \"Bind\" 列表按顺序绑定到查询上（宏展开以便同时适配不同返回行类型，
/// 与 `repo/order_admin.rs` 的 `bind_all!` 做法一致）。
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

/// \"updateField\" 允许更新的列白名单。
///
/// 权威来源：`java/nextrade-common/.../CheckFieldConstants.java:23`
/// `AUTHORITY_FIELDS = {"authority_name", "authority_sn", "route_link", "is_show", "sort_order"}`。
/// 注意 Java 侧该常量存的是**列名**，而 `BaseServiceImpl#updateField`
/// 只做 `property → column` 单向后转换，因此前端若传列名会退化为「更新主键列」。
/// Rust 侧同时接受属性名与列名（两者都映射到列名），语义见 `service::map_update_field`。
pub const AUTHORITY_COLUMN_FIELDS: &[&str] = &[
    "authority_name",
    "authority_sn",
    "route_link",
    "is_show",
    "sort_order",
];

/// 一次 \"list\" 查询的筛选条件（对应 Java \"AuthorityListDTO\" + `AuthorityServiceImpl#list`）。
#[derive(Debug, Clone, Default)]
pub struct AuthorityPageFilter {
    /// `authority.admin_type` 精确匹配（为空则不过滤）。
    pub admin_type: Option<String>,
    /// `authority.parent_id` 精确匹配（Java：`parentId != null && parentId >= 0`）。
    pub parent_id: Option<i32>,
    /// 排序字段（Java `BasePage.sortField`，为空时用主键）。
    pub sort_field: Option<String>,
    /// 排序方向（\"asc\" / \"desc\"，为空视为 \"desc\"）。
    pub sort_order: Option<String>,
}

/// 部分列投影（\"getAllAuthority\" / \"getAuthority\" / 父级名称链只取这几列）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct AuthoritySelected {
    pub authority_id: i32,
    pub authority_sn: String,
    pub authority_name: String,
    pub parent_id: i32,
    pub sort_order: i32,
    pub is_show: i32,
    pub child_auth: Option<String>,
    pub route_link: String,
    pub authority_ico: String,
}

#[derive(Clone)]
pub struct AuthorityRepository {
    pool: PgPool,
}

impl AuthorityRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 通用查询 —————————————————————

    /// 按主键查询完整实体。
    pub async fn find_by_id(&self, authority_id: i32) -> Result<Option<Authority>> {
        let sql = crate::db::to_pg_placeholders(&format!("SELECT {AUTHORITY_SELECT} FROM authority WHERE authority_id = ? LIMIT 1"));
        sqlx::query_as::<_, Authority>(&sql)
            .bind(authority_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 权限详情（Java `AuthorityServiceImpl#detail`：不存在则抛「权限不存在」）。
    pub async fn find_or_err(&self, authority_id: i32) -> Result<Authority> {
        self.find_by_id(authority_id)
            .await?
            .ok_or_else(|| Error::biz_msg("权限不存在"))
    }

    /// `authority_sn` + `admin_type` 重复校验
    /// （Java `buildChildAuth`：`ne(id != null, authorityId).eq(sn).eq(adminType)`）。
    pub async fn count_same_sn(
        &self,
        authority_sn: &str,
        admin_type: &str,
        exclude_id: Option<i32>,
    ) -> Result<i64> {
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM authority WHERE authority_sn = ");
        qb.push_bind(authority_sn)
            .push(" AND admin_type = ")
            .push_bind(admin_type);
        if let Some(id) = exclude_id {
            qb.push(" AND authority_id <> ").push_bind(id);
        }
        let row: (i64,) = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0)
    }

    /// `parent_id = ?` 是否存在（Java `del`：不能有子级）。
    pub async fn count_children(&self, parent_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM authority WHERE parent_id = $1")
            .bind(parent_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0)
    }

    /// 按 `parent_id` 批量取子权限（Java `list`：一次取回当前页所有子级，消除 N+1）。
    pub async fn find_children_of(&self, parent_ids: &[i32]) -> Result<Vec<Authority>> {
        if parent_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT {AUTHORITY_SELECT} FROM authority WHERE parent_id IN ({})",
            placeholders(parent_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, Authority>(&sql);
        for id in parent_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 删除（Java `BaseServiceImpl#del`：\"removeById\"）。
    pub async fn delete_by_id(&self, authority_id: i32) -> Result<u64> {
        let res = sqlx::query("DELETE FROM authority WHERE authority_id = $1")
            .bind(authority_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— 列表分页 —————————————————————

    /// 权限分页（Java `AuthorityServiceImpl#list` 的 `lambdaQuery().page(page)`）。
    ///
    /// 返回 `(total, records)`；\"records\" 为分页**之后**再由服务层按下架规则过滤，
    /// 与原 Java 完全一致（Java 也是先分页再 \"removeIf\"，故当前页条数可能少于 \"size\"）。
    pub async fn paginate(
        &self,
        f: &AuthorityPageFilter,
        limit: i64,
        offset: i64,
    ) -> Result<(u64, Vec<Authority>)> {
        let (where_sql, binds) = build_page_where(f);

        let total_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM authority{where_sql}"));
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&total_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {AUTHORITY_SELECT} FROM authority{where_sql}{} LIMIT ? OFFSET ?",
            order_by_clause(f)
        ));
        let rows = bind_all!(sqlx::query_as::<_, Authority>(&rows_sql), binds)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    // ————————————————————— 当前用户可见权限 —————————————————————

    /// 当前账号可见的权限（Java \"getAllAuthority\" / \"getAuthority\" 共享查询体）。
    ///
    /// - \"admin_type\"：\"admin\" / \"shop\" / \"vendor\"（Java 会把 \"suppliers\" 归一为 \"admin\"）。
    /// - \"sn_filter\"：`Some(list)` 时追加 `authority_sn IN (...)`；
    ///   \"None\" 表示该账号拥有 \"all\"（或无权限项）→ 不加限制。
    ///   `Some(空表)` 对应 Java `in(..., [])` → MySQL 下恒假 → 空结果集。
    /// - `is_show = 1` 恒定条件。
    /// - 排序：\"getAllAuthority\" 为 `parent_id ASC, sort_order ASC, authority_id ASC`；
    ///   \"getAuthority\" 为 `parent_id ASC, sort_order DESC, authority_id ASC`
    ///   （故 `sort_order_desc = true` 对应后者）。
    pub async fn find_visible(
        &self,
        admin_type: &str,
        sn_filter: Option<&[String]>,
        sort_order_desc: bool,
    ) -> Result<Vec<AuthoritySelected>> {
        if sn_filter.is_some_and(<[String]>::is_empty) {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT authority_id, authority_sn, authority_name, parent_id, sort_order, is_show, child_auth, route_link, authority_ico FROM authority WHERE admin_type = ");
        qb.push_bind(admin_type);
        if let Some(list) = sn_filter {
            qb.push(" AND authority_sn IN (");
            let mut sep = qb.separated(", ");
            for sn in list {
                sep.push_bind(sn.as_str());
            }
            sep.push_unseparated(")");
        }
        qb.push(" AND is_show = 1 ORDER BY parent_id ASC, sort_order ");
        qb.push(if sort_order_desc { "DESC" } else { "ASC" });
        qb.push(", authority_id ASC");
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按 \"authority_id\" 取名称（Java \"getParentName\"，只 `select authority_name`）。
    pub async fn find_name_by_id(&self, authority_id: i32) -> Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT authority_name FROM authority WHERE authority_id = $1 LIMIT 1")
                .bind(authority_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|r| r.0))
    }

    /// 取 `parent_id`（Java `getAuthorityParentName` 先按 id 查父级 id）。
    pub async fn find_parent_id(&self, authority_id: i32) -> Result<Option<i32>> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT parent_id FROM authority WHERE authority_id = $1 LIMIT 1")
                .bind(authority_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|r| r.0))
    }

    /// 批量按主键取权限（Java `batchDel` 的 `listByIds`）。
    pub async fn find_by_ids(&self, ids: &[i32]) -> Result<Vec<Authority>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT {AUTHORITY_SELECT} FROM authority WHERE authority_id IN ({})",
            placeholders(ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, Authority>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 是否存在 `parent_id = ?` 且 `authority_id NOT IN (排除集)` 的子级
    /// （Java `batchDel`：本次要删的 id 之外的子级会阻止删除）。
    pub async fn find_blocking_child(
        &self,
        parent_id: i32,
        excluded: &[i32],
    ) -> Result<Option<Authority>> {
        if excluded.is_empty() {
            let sql =
                crate::db::to_pg_placeholders(&format!("SELECT {AUTHORITY_SELECT} FROM authority WHERE parent_id = ? LIMIT 1"));
            return sqlx::query_as::<_, Authority>(&sql)
                .bind(parent_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err);
        }
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {AUTHORITY_SELECT} FROM authority WHERE parent_id = ? \
             AND authority_id NOT IN ({}) LIMIT 1",
            placeholders(excluded.len())
        ));
        let mut q = sqlx::query_as::<_, Authority>(&sql).bind(parent_id);
        for id in excluded {
            q = q.bind(*id);
        }
        q.fetch_optional(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 写入 —————————————————————

    /// 新建权限（Java `AuthorityServiceImpl#create` → \"save\"）。返回自增主键。
    pub async fn insert(&self, a: &Authority) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO authority
               (authority_id, authority_sn, authority_name, parent_id, sort_order,
                is_show, child_auth, route_link, authority_ico, is_system, admin_type)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING authority_id",
        )
        .bind(a.authority_id)
        .bind(&a.authority_sn)
        .bind(&a.authority_name)
        .bind(a.parent_id)
        .bind(a.sort_order)
        .bind(a.is_show)
        .bind(&a.child_auth)
        .bind(&a.route_link)
        .bind(&a.authority_ico)
        .bind(a.is_system)
        .bind(&a.admin_type)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 更新权限全部业务列（Java `AuthorityServiceImpl#update` → \"updateById\"）。
    pub async fn update(&self, a: &Authority) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE authority SET
               authority_sn = $1, authority_name = $2, parent_id = $3, sort_order = $4,
               is_show = $5, child_auth = $6, route_link = $7, authority_ico = $8,
               is_system = $9, admin_type = $10
             WHERE authority_id = $11",
        )
        .bind(&a.authority_sn)
        .bind(&a.authority_name)
        .bind(a.parent_id)
        .bind(a.sort_order)
        .bind(a.is_show)
        .bind(&a.child_auth)
        .bind(&a.route_link)
        .bind(&a.authority_ico)
        .bind(a.is_system)
        .bind(&a.admin_type)
        .bind(a.authority_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 单字段更新（Java `BaseServiceImpl#updateField`）。
    ///
    /// \"column\" 必须由 `service::map_update_field` 经白名单映射得到，
    /// 只可能是 [\"AUTHORITY_COLUMN_FIELDS\"] 中的固定列名（无注入面）。
    /// \"val\" 以文本绑定，MySQL 按目标列类型隐式转换
    /// （与 MyBatis-Plus `set(column, "字符串值")` 行为一致）。
    pub async fn update_field(&self, id: i32, column: &str, val: &str) -> Result<u64> {
        debug_assert!(
            AUTHORITY_COLUMN_FIELDS.contains(&column),
            "update_field 列名未经过白名单映射: {column}"
        );
        let sql = crate::db::to_pg_placeholders(&format!("UPDATE authority SET {column} = ? WHERE authority_id = ?"));
        let res = sqlx::query(&sql)
            .bind(val)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— 店铺 / 供应商账号授权 —————————————————————

    /// 店铺员工绑定（Java `AdminUserShopMapper.selectOne(adminId + shopId)`）。
    /// 返回 `(role_id, auth_list)`。
    pub async fn find_shop_binding(
        &self,
        admin_id: i32,
        shop_id: i32,
    ) -> Result<Option<(i32, Option<String>)>> {
        let row: Option<(i32, Option<String>)> = sqlx::query_as(
            "SELECT role_id, auth_list FROM admin_user_shop \
             WHERE admin_id = $1 AND shop_id = $2 LIMIT 1",
        )
        .bind(admin_id)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row)
    }

    /// 供应商员工绑定（Java `AdminUserVendorMapper.selectOne(adminId + vendorId)`）。
    pub async fn find_vendor_binding(
        &self,
        admin_id: i32,
        vendor_id: i32,
    ) -> Result<Option<(i32, Option<String>)>> {
        let row: Option<(i32, Option<String>)> = sqlx::query_as(
            "SELECT role_id, auth_list FROM admin_user_vendor \
             WHERE admin_id = $1 AND vendor_id = $2 LIMIT 1",
        )
        .bind(admin_id)
        .bind(vendor_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row)
    }

    /// 角色授权列表（Java `AdminRoleService#getById` → `authority_list` JSON）。
    pub async fn find_role_authority_list(&self, role_id: i32) -> Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT authority_list FROM admin_role WHERE role_id = $1 LIMIT 1")
                .bind(role_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|r| r.0))
    }

    /// 账号 `admin_type` + `auth_list`
    /// （Java `adminUser.getAdminType()` / `adminUser.getAuthList()`，
    /// 来源为 Redis 缓存的登录用户；Rust 侧直接读 `admin_user`）。
    pub async fn find_admin_auth(&self, admin_id: i32) -> Result<Option<(String, Option<String>)>> {
        let row: Option<(String, Option<String>)> =
            sqlx::query_as("SELECT admin_type, auth_list FROM admin_user WHERE admin_id = $1 LIMIT 1")
                .bind(admin_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row)
    }
}

/// 统一把 `sqlx::Error` 映射为 `Error::Database`。
fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

/// `?` 占位符串（`n` 个）。
fn placeholders(n: usize) -> String {
    let mut s = String::with_capacity(n * 3);
    for i in 0..n {
        if i > 0 {
            s.push_str(", ");
        }
        s.push('?');
    }
    s
}

/// 构造 \"list\" 的 \"WHERE\" 子句（列名全部硬编码，无注入面）。
fn build_page_where(f: &AuthorityPageFilter) -> (String, Vec<Bind>) {
    let mut sql = String::from(" WHERE 1 = 1");
    let mut binds: Vec<Bind> = Vec::new();
    if let Some(t) = f.admin_type.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        sql.push_str(" AND admin_type = ?");
        binds.push(Bind::Str(t.to_string()));
    }
    // Java：`eq(parentId != null && parentId >= 0, parentId)`
    if let Some(p) = f.parent_id.filter(|p| *p >= 0) {
        sql.push_str(" AND parent_id = ?");
        binds.push(Bind::I32(p));
    }
    (sql, binds)
}

/// 排序子句（Java `BaseServiceImpl#buildSortOrder`：`sortField` 为空或等于主键属性
/// 时用主键列；方向 `asc` 升序，其余（含空）按 \"desc\"）。
fn order_by_clause(f: &AuthorityPageFilter) -> String {
    let key = "authority_id";
    let field = f.sort_field.as_deref().unwrap_or("").trim();
    let column = if field.is_empty() || field == "authorityId" || field == key {
        key
    } else {
        map_sort_column(field).unwrap_or(key)
    };
    let asc = f
        .sort_order
        .as_deref()
        .is_some_and(|o| o.eq_ignore_ascii_case("asc"));
    let dir = if asc { "ASC" } else { "DESC" };
    format!(" ORDER BY {column} {dir}")
}

/// `authority` 的属性名 / 列名 → 列名（仅允许已知列，其余返回 `None`）。
///
/// 对应 Java `BaseServiceImpl#getColumnByPropertyName`：识别不到时回退主键列，
/// 这里把 `None` 交给调用方处理（当前实现同样回退主键）。
#[must_use]
pub fn map_sort_column(name: &str) -> Option<&'static str> {
    Some(match name {
        "authorityId" | "authority_id" => "authority_id",
        "authoritySn" | "authority_sn" => "authority_sn",
        "authorityName" | "authority_name" => "authority_name",
        "parentId" | "parent_id" => "parent_id",
        "sortOrder" | "sort_order" => "sort_order",
        "isShow" | "is_show" => "is_show",
        "childAuth" | "child_auth" => "child_auth",
        "routeLink" | "route_link" => "route_link",
        "authorityIco" | "authority_ico" => "authority_ico",
        "isSystem" | "is_system" => "is_system",
        "adminType" | "admin_type" => "admin_type",
        _ => return None,
    })
}

/// \"authority\" 全列清单（供文档与调试使用）。
#[must_use]
pub fn authority_columns() -> &'static str {
    AUTHORITY_SELECT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_column_mapping_covers_all_columns() {
        for (prop, col) in [
            ("authorityId", "authority_id"),
            ("authoritySn", "authority_sn"),
            ("authorityName", "authority_name"),
            ("parentId", "parent_id"),
            ("sortOrder", "sort_order"),
            ("isShow", "is_show"),
            ("childAuth", "child_auth"),
            ("routeLink", "route_link"),
            ("authorityIco", "authority_ico"),
            ("isSystem", "is_system"),
            ("adminType", "admin_type"),
        ] {
            assert_eq!(map_sort_column(prop), Some(col));
            assert_eq!(map_sort_column(col), Some(col));
        }
        // 未知字段回退主键（Java `getColumnByPropertyName` 的安全回退）
        assert_eq!(map_sort_column("evil; DROP TABLE authority"), None);
    }

    #[test]
    fn order_by_defaults_to_primary_key_desc() {
        let f = AuthorityPageFilter::default();
        assert_eq!(order_by_clause(&f), " ORDER BY authority_id DESC");

        let f = AuthorityPageFilter {
            sort_field: Some(String::new()),
            sort_order: Some("asc".into()),
            ..Default::default()
        };
        assert_eq!(order_by_clause(&f), " ORDER BY authority_id ASC");

        let f = AuthorityPageFilter {
            sort_field: Some("sortOrder".into()),
            sort_order: Some("DESC".into()),
            ..Default::default()
        };
        assert_eq!(order_by_clause(&f), " ORDER BY sort_order DESC");

        // 注入串回退主键列
        let f = AuthorityPageFilter {
            sort_field: Some("authority_id, (SELECT 1)".into()),
            sort_order: None,
            ..Default::default()
        };
        assert_eq!(order_by_clause(&f), " ORDER BY authority_id DESC");
    }

    #[test]
    fn page_where_skips_blank_admin_type_and_negative_parent() {
        let (sql, binds) = build_page_where(&AuthorityPageFilter::default());
        assert_eq!(sql, " WHERE 1 = 1");
        assert!(binds.is_empty());

        let f = AuthorityPageFilter {
            admin_type: Some("  ".into()),
            parent_id: Some(-1),
            ..Default::default()
        };
        let (sql, binds) = build_page_where(&f);
        assert_eq!(sql, " WHERE 1 = 1");
        assert!(binds.is_empty());

        let f = AuthorityPageFilter {
            admin_type: Some("admin".into()),
            parent_id: Some(0),
            ..Default::default()
        };
        let (sql, binds) = build_page_where(&f);
        assert_eq!(sql, " WHERE 1 = 1 AND admin_type = ? AND parent_id = ?");
        assert_eq!(binds.len(), 2);
    }

    #[test]
    fn placeholders_and_columns() {
        assert_eq!(placeholders(0), "");
        assert_eq!(placeholders(1), "?");
        assert_eq!(placeholders(3), "?, ?, ?");

        let cols: Vec<&str> = authority_columns().split(", ").collect();
        assert_eq!(cols.len(), 11);
        assert_eq!(cols[0], "authority_id");
        assert_eq!(cols[10], "admin_type");
    }
}
