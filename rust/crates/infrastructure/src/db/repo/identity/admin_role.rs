//! 后台角色域仓储（identity::admin_role）。
//!
//! 表/列均来自权威 schema `java/database/pure.sql`
//! （核对命令：`cd rust && python tools/schema_parser.py --table admin_role`）：
//! - `admin_role`：`role_id` / `role_name` / `role_code` / `role_desc` /
//!   `authority_list` / `admin_type` / `merchant_id` / `shop_id` / `vendor_id` /
//!   `is_system`（主键 `role_id`，10 列）
//! - `shop`：`shop_id` / `merchant_id` / `shop_type`（租户解析）
//! - `merchant`：`merchant_id`（存在性校验）
//! - `admin_user` / `admin_user_shop` / `admin_user_vendor`：`role_id`
//!   （删除前的绑定计数 / 删除后的解绑）
//!
//! SQL 一律运行时 `sqlx::query_as`（**不用** `query!` 宏），每条 SQL 单行字面量。
//!
//! 对应 Java：
//! - `AdminRoleServiceImpl#list`（`this.page(page, queryWrapper)`）
//! - `#detail` / `#getAccessibleRole`（`this.getById`）
//! - `#create`（`this.save`）
//! - `#update`（`this.updateById`）
//! - `#updateField`（`BaseServiceImpl#updateField` 的 `UpdateWrapper`）
//! - `#delRole` / `#del`（`this.removeById` + 三张绑定表的 `role_id = 0`）

use sqlx::PgPool;
use nextrade_core::entities::AdminRole;
use nextrade_core::models::identity::admin_role::{
    role_field_to_column, RoleVisibility, ShopScope, KEFU_ROLE_CODE,
};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// `admin_role` 的 10 列（顺序与 `AdminRole` 实体字段一致）。
const ROLE_COLUMNS: &str = "role_id, role_name, role_code, role_desc, authority_list, admin_type, merchant_id, shop_id, vendor_id, is_system";

/// 排序解析结果。
///
/// `ascending == None` 对应 Java `buildSortOrder` 的「既非 asc 也非 desc/空」
/// 分支：`page.setOrders(...)` 一次都没调用 → 生成的 SQL **没有 ORDER BY**。
#[derive(Debug, Clone)]
pub struct Sort {
    /// 白名单化后的列名
    pub column: &'static str,
    /// 升/降/不排序
    pub ascending: Option<bool>,
}

/// 新建角色待写字段（对应 Java `convertToBean` 产出的实体）。
///
/// 说明：Java `AdminRole` 实体上 `merchant_id` / `shop_id` / `vendor_id` /
/// `is_system` 都有 DDL 默认值 0，`role_name` / `role_desc` 默认 `''`；
/// MyBatis-Plus `save` 的 `NOT_NULL` 策略会把这些 `null` 字段**排除出 INSERT**，
/// 从而套用列默认值。这里显式补齐同样的默认值（与 `catalog::brand` 的处理一致）：
/// - `merchant_id` 恒 0：`BeanUtils.copyProperties` 不会把 DTO 的
///   `String merchantId` 拷进实体的 `Integer merchantId`，故实体里恒 `null`；
/// - `role_code` 恒 NULL：DTO 没有该字段；
/// - `is_system` 恒 0：`convertToBean` 显式 `setIsSystem(null)`；
/// - `admin_type` 在 Java 里可能为 NULL（请求头缺失）→ DB 默认 `'admin'`，
///   本实现由服务层补成 `"admin"`。
#[derive(Debug, Clone, Default)]
pub struct NewRole {
    /// 显式主键（Java `save` 会带上 DTO 里拷贝来的 `roleId`；`None` 走自增）
    pub role_id: Option<i32>,
    /// `role_name`
    pub role_name: String,
    /// `role_desc`
    pub role_desc: String,
    /// `authority_list`（JSON 文本，`null` 时为字面量 `"null"`）
    pub authority_list: String,
    /// `admin_type`
    pub admin_type: String,
    /// `shop_id`（Java `HeaderUtils#getShopId()`，缺省 0）
    pub shop_id: i32,
    /// `vendor_id`（Java `HeaderUtils#getVendorId()`，缺省 0）
    pub vendor_id: i32,
}

/// 更新角色待写字段（对应 Java `updateById(updated)` 的净效果）。
///
/// MyBatis-Plus `updateById` 默认 `FieldStrategy.NOT_NULL`：**只写非 null 字段**。
/// 服务层已经把恒非 null 的字段（`authority_list` / `merchant_id` / `shop_id` /
/// `vendor_id` / `is_system`）补齐，`role_name` / `role_desc` / `role_code` /
/// `admin_type` 则保留 `Option`，`None` 时不出现在 SET 里（不覆盖库中原值）。
#[derive(Debug, Clone, Default)]
pub struct RolePatch {
    /// `role_name`（DTO 未传则不改）
    pub role_name: Option<String>,
    /// `role_desc`（DTO 未传则不改）
    pub role_desc: Option<String>,
    /// `authority_list`（`convertToBean` 总会写入，`None` 时是 `"null"`）
    pub authority_list: String,
    /// `admin_type`（Java 用库中原值覆盖，原值为 NULL 时不写）
    pub admin_type: Option<String>,
    /// `merchant_id`（库中原值）
    pub merchant_id: i32,
    /// `shop_id`（库中原值）
    pub shop_id: i32,
    /// `vendor_id`（库中原值）
    pub vendor_id: i32,
    /// `role_code`（库中原值；为 NULL 时不写）
    pub role_code: Option<String>,
    /// `is_system`（库中原值）
    pub is_system: i32,
}

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct AdminRoleRepository {
    pool: PgPool,
}

impl AdminRoleRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 开启事务（`AdminRoleServiceImpl#delRole` / `#batch` 的 `@Transactional`）。
    ///
    /// # Errors
    /// 连接池获取连接失败 → `Error::Database`。
    pub async fn begin(&self) -> Result<sqlx::Transaction<'static, sqlx::Postgres>> {
        self.pool.begin().await.map_err(map_err)
    }

    // ————————————————————— 查询 —————————————————————

    /// 按主键查询角色（Java `this.getById(id)`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn find_by_id(&self, role_id: i32) -> Result<Option<AdminRole>> {
        sqlx::query_as::<_, AdminRole>(&format!(
            "SELECT {ROLE_COLUMNS} FROM admin_role WHERE role_id = $1 LIMIT 1"
        ))
        .bind(role_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 事务内按主键查询角色（`delRole` 的读取也在 Java `@Transactional` 内）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn find_by_id_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        role_id: i32,
    ) -> Result<Option<AdminRole>> {
        sqlx::query_as::<_, AdminRole>(&format!(
            "SELECT {ROLE_COLUMNS} FROM admin_role WHERE role_id = $1 LIMIT 1"
        ))
        .bind(role_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_err)
    }

    /// 角色分页（对应 Java `this.page(page, queryWrapper)`）。
    ///
    /// WHERE = 租户可见性 **AND** `(role_code IS NULL OR role_code <> 'kefu_agent')`
    /// **AND** 关键词 `(role_name LIKE ? OR role_desc LIKE ?)`。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn paginate(
        &self,
        visibility: &RoleVisibility,
        keyword: Option<&str>,
        sort: &Sort,
        page: &PageQuery,
    ) -> Result<(u64, Vec<AdminRole>)> {
        let (where_sql, like_bind) = build_where(visibility, keyword);
        let (limit, offset) = page.limit_offset();

        let count_sql = format!("SELECT COUNT(*) FROM admin_role{where_sql}");
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        if let Some(like) = like_bind.as_deref() {
            cq = cq.bind(like).bind(like);
        }
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let order_sql = match sort.ascending {
            Some(asc) => format!(
                " ORDER BY {} {}",
                sort.column,
                if asc { "ASC" } else { "DESC" }
            ),
            None => String::new(),
        };
        let list_sql = format!(
            "SELECT {ROLE_COLUMNS} FROM admin_role{where_sql}{order_sql} LIMIT ? OFFSET ?"
        );
        let mut lq = sqlx::query_as::<_, AdminRole>(&list_sql);
        if let Some(like) = like_bind.as_deref() {
            lq = lq.bind(like).bind(like);
        }
        let rows = lq
            .bind(i64::try_from(limit).unwrap_or(0))
            .bind(i64::try_from(offset).unwrap_or(0))
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 店铺租户解析（Java `shopMapper.selectById(shopId)` 用到的三列）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn shop_scope(&self, shop_id: i32) -> Result<Option<ShopScope>> {
        let row: Option<(i32, Option<i32>, Option<i32>)> = sqlx::query_as(
            "SELECT shop_id, merchant_id, shop_type FROM shop WHERE shop_id = $1 LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(shop_id, merchant_id, shop_type)| ShopScope {
            shop_id,
            merchant_id,
            shop_type,
        }))
    }

    /// 商户存在性（Java `merchantMapper.selectById(merchantId) == null` 判断）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn merchant_exists(&self, merchant_id: i32) -> Result<bool> {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM merchant WHERE merchant_id = $1")
                .bind(merchant_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(count > 0)
    }

    // ————————————————————— 写入 —————————————————————

    /// 新建角色（Java `this.save(convertToBean(dto))`）。
    ///
    /// 返回新角色主键（Java 侧 `save` 后实体回填 `roleId`，控制器返回 void）。
    ///
    /// # Errors
    /// 数据库失败（如 `authority_list` 过长、唯一键冲突）→ `Error::Database`。
    pub async fn insert(&self, role: &NewRole) -> Result<i32> {
        let result: i32 = match role.role_id {
            Some(role_id) => {
                sqlx::query_scalar(
                    "INSERT INTO admin_role (role_id, role_name, role_code, role_desc, authority_list, admin_type, merchant_id, shop_id, vendor_id, is_system) VALUES ($1, $2, NULL, $3, $4, $5, 0, $6, $7, 0) RETURNING role_id",
                )
                .bind(role_id)
                .bind(&role.role_name)
                .bind(&role.role_desc)
                .bind(&role.authority_list)
                .bind(&role.admin_type)
                .bind(role.shop_id)
                .bind(role.vendor_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?
            }
            None => {
                sqlx::query_scalar(
                    "INSERT INTO admin_role (role_name, role_code, role_desc, authority_list, admin_type, merchant_id, shop_id, vendor_id, is_system) VALUES ($1, NULL, $2, $3, $4, 0, $5, $6, 0) RETURNING role_id",
                )
                .bind(&role.role_name)
                .bind(&role.role_desc)
                .bind(&role.authority_list)
                .bind(&role.admin_type)
                .bind(role.shop_id)
                .bind(role.vendor_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?
            }
        };
        Ok(result)
    }

    /// 更新角色（Java `this.updateById(updated)`，`NOT_NULL` 策略）。
    ///
    /// 返回值等价 Java `updateById` 的布尔结果：影响行数 > 0。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn update(&self, role_id: i32, patch: &RolePatch) -> Result<bool> {
        let mut sets: Vec<&str> = Vec::with_capacity(8);
        if patch.role_name.is_some() {
            sets.push("role_name = ?");
        }
        if patch.role_desc.is_some() {
            sets.push("role_desc = ?");
        }
        sets.push("authority_list = ?");
        if patch.admin_type.is_some() {
            sets.push("admin_type = ?");
        }
        sets.push("merchant_id = ?");
        sets.push("shop_id = ?");
        sets.push("vendor_id = ?");
        if patch.role_code.is_some() {
            sets.push("role_code = ?");
        }
        sets.push("is_system = ?");

        let sql = format!(
            "UPDATE admin_role SET {} WHERE role_id = $1",
            sets.join(", ")
        );
        let mut q = sqlx::query(&sql);
        if let Some(role_name) = patch.role_name.as_deref() {
            q = q.bind(role_name);
        }
        if let Some(role_desc) = patch.role_desc.as_deref() {
            q = q.bind(role_desc);
        }
        q = q
            .bind(&patch.authority_list)
            .bind(patch.admin_type.as_deref())
            .bind(patch.merchant_id)
            .bind(patch.shop_id)
            .bind(patch.vendor_id)
            .bind(patch.role_code.as_deref())
            .bind(patch.is_system)
            .bind(role_id);
        let result = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(result.rows_affected() > 0)
    }

    /// `BASE_UPDATE`：`admin_role.<column> = val WHERE role_id = ?`。
    ///
    /// 对应 Java `BaseServiceImpl#updateField` 的
    /// `updateWrapper.eq(keyColumn, id).set(field, val)`；
    /// `column` 已由服务层白名单化为 `&'static str`（仅 `role_name`），
    /// `val == None` 会写成 SQL NULL（`role_name` 是 `NOT NULL`，故触发数据库错误，
    /// 与 Java 把 `null` 交给 MySQL 的行为一致）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn update_field(
        &self,
        role_id: i32,
        column: &'static str,
        val: Option<&str>,
    ) -> Result<bool> {
        let sql = format!("UPDATE admin_role SET {column} = $1 WHERE role_id = $2");
        let result = sqlx::query(&sql)
            .bind(val)
            .bind(role_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(result.rows_affected() > 0)
    }

    // ————————————————————— 删除 / 解绑 —————————————————————

    /// 该角色在 `admin_user` / `admin_user_shop` / `admin_user_vendor`
    /// 三张表的绑定数之和（Java `delRole` 的 `bindCount`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn count_bindings(&self, role_id: i32) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT (SELECT COUNT(*) FROM admin_user WHERE role_id = $1) + (SELECT COUNT(*) FROM admin_user_shop WHERE role_id = $2) + (SELECT COUNT(*) FROM admin_user_vendor WHERE role_id = $3)",
        )
        .bind(role_id)
        .bind(role_id)
        .bind(role_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(count)
    }

    /// 事务内绑定计数（`batch` 的 `@Transactional` 语义）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn count_bindings_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        role_id: i32,
    ) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT (SELECT COUNT(*) FROM admin_user WHERE role_id = $1) + (SELECT COUNT(*) FROM admin_user_shop WHERE role_id = $2) + (SELECT COUNT(*) FROM admin_user_vendor WHERE role_id = $3)",
        )
        .bind(role_id)
        .bind(role_id)
        .bind(role_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(count)
    }

    /// 事务内删除角色（Java `this.del(id)` → `removeById`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn delete_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        role_id: i32,
    ) -> Result<bool> {
        let result = sqlx::query("DELETE FROM admin_role WHERE role_id = $1")
            .bind(role_id)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?;
        Ok(result.rows_affected() > 0)
    }

    /// 事务内把三张绑定表的 `role_id` 置 0（Java 删除成功后的三条
    /// `LambdaUpdateWrapper.set(roleId, 0)`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn reset_bindings_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        role_id: i32,
    ) -> Result<()> {
        for sql in [
            "UPDATE admin_user SET role_id = 0 WHERE role_id = $1",
            "UPDATE admin_user_shop SET role_id = 0 WHERE role_id = $1",
            "UPDATE admin_user_vendor SET role_id = 0 WHERE role_id = $1",
        ] {
            sqlx::query(sql)
                .bind(role_id)
                .execute(&mut **tx)
                .await
                .map_err(map_err)?;
        }
        Ok(())
    }
}

/// 拼装 WHERE（含前导空格；无条件时返回空串）。
///
/// 第二个返回值是关键词 LIKE 的绑定值：Java 的
/// `and(qw -> qw.like(roleName, kw).or().like(roleDesc, kw))` 两个 `like`
/// 共用同一个 `%kw%`。
///
/// 租户条件里的 ID 全部来自 `i32` 类型的上下文，直接内插不会引入注入面。
fn build_where(visibility: &RoleVisibility, keyword: Option<&str>) -> (String, Option<String>) {
    let mut conds: Vec<String> = Vec::new();

    // Java: switch (adminTypeEnum) { ... }
    conds.push(match visibility {
        RoleVisibility::Admin { shop_id } => {
            format!("(shop_id = {shop_id} AND admin_type = 'admin')")
        }
        RoleVisibility::Shop {
            shop_id,
            merchant_id: Some(merchant_id),
        } => format!(
            "(((merchant_id = {merchant_id} AND shop_id = 0) OR shop_id = {shop_id}))"
        ),
        RoleVisibility::Shop {
            shop_id,
            merchant_id: None,
        } => format!("(shop_id = {shop_id})"),
        RoleVisibility::Vendor {
            vendor_id: Some(vendor_id),
        } => format!("(vendor_id = {vendor_id} AND admin_type = 'vendor')"),
        // Java `eq(column, null)` 生成的 `vendor_id = NULL` 恒不匹配；
        // 用 `1 = 0` 表达同一语义（避免 MySQL 对 `= NULL` 的隐式转换告警）。
        RoleVisibility::Vendor { vendor_id: None } => "(1 = 0)".to_string(),
    });

    // Java: queryWrapper.and(qw -> qw.isNull(roleCode).or().ne(roleCode, "kefu_agent"))
    conds.push(format!(
        "(role_code IS NULL OR role_code <> '{KEFU_ROLE_CODE}')"
    ));

    // Java: StringUtils.isNotEmpty(keyword)（**不 trim**）
    let like = keyword
        .filter(|k| !k.is_empty())
        .map(|k| format!("%{k}%"));
    if like.is_some() {
        conds.push("(role_name LIKE ? OR role_desc LIKE ?)".to_string());
    }

    (format!(" WHERE {}", conds.join(" AND ")), like)
}

/// Java `BaseServiceImpl#buildSortOrder(page, sortField, sortOrder)` 的等价实现。
///
/// - 字段：`sortField` 为空 / 等于主键属性名时用主键列；其余按
///   `getColumnByPropertyName` 映射（属性名或列名，未知值回退主键列）；
/// - 方向：`asc`（忽略大小写）→ 升序；`desc` / 空串 / 缺失 → 降序；
///   其它任意值 → 一次都不 `setOrders`，即**不输出 ORDER BY**。
#[must_use]
pub fn resolve_sort(sort_field: Option<&str>, sort_order: Option<&str>) -> Sort {
    let column = role_field_to_column(sort_field);
    let ascending = match sort_order {
        Some(order) if order.eq_ignore_ascii_case("asc") => Some(true),
        Some(order) if order.eq_ignore_ascii_case("desc") || order.is_empty() => Some(false),
        None => Some(false),
        Some(_) => None,
    };
    Sort { column, ascending }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_where_matches_java_wrapper() {
        let (sql, bind) = build_where(&RoleVisibility::Admin { shop_id: 0 }, None);
        assert_eq!(
            sql,
            " WHERE (shop_id = 0 AND admin_type = 'admin') AND (role_code IS NULL OR role_code <> 'kefu_agent')"
        );
        assert!(bind.is_none());

        // 平台账号的 shopId 恒 0（HeaderUtils#getShopId），即使请求头带了别的值
        let (sql, _) = build_where(&RoleVisibility::Admin { shop_id: 0 }, Some("客服"));
        assert!(sql.starts_with(" WHERE (shop_id = 0 AND admin_type = 'admin')"));

        // 店铺（无商户）：只按本店过滤
        let (sql, _) = build_where(
            &RoleVisibility::Shop {
                shop_id: 12,
                merchant_id: None,
            },
            None,
        );
        assert!(sql.contains("(shop_id = 12)"));

        // 店铺（有商户）：商户级角色（shop_id = 0）或本店角色
        let (sql, _) = build_where(
            &RoleVisibility::Shop {
                shop_id: 12,
                merchant_id: Some(5),
            },
            None,
        );
        assert!(sql.contains("(((merchant_id = 5 AND shop_id = 0) OR shop_id = 12))"));

        // 供应商
        let (sql, _) = build_where(
            &RoleVisibility::Vendor {
                vendor_id: Some(9),
            },
            None,
        );
        assert!(sql.contains("(vendor_id = 9 AND admin_type = 'vendor')"));

        // getVendorId() 为 null → 查不到任何行
        let (sql, _) = build_where(&RoleVisibility::Vendor { vendor_id: None }, None);
        assert!(sql.contains("(1 = 0)"));
    }

    #[test]
    fn keyword_becomes_two_like_binds() {
        let (sql, bind) = build_where(&RoleVisibility::Admin { shop_id: 0 }, Some("客服"));
        assert!(sql.ends_with("(role_name LIKE ? OR role_desc LIKE ?)"));
        assert_eq!(bind.as_deref(), Some("%客服%"));

        // Java `StringUtils.isNotEmpty`：只排除空串，空白串也要参与 LIKE
        let (sql, bind) = build_where(&RoleVisibility::Admin { shop_id: 0 }, Some("  "));
        assert!(sql.contains("role_name LIKE ?"));
        assert_eq!(bind.as_deref(), Some("%  %"));

        let (sql, bind) = build_where(&RoleVisibility::Admin { shop_id: 0 }, Some(""));
        assert!(!sql.contains("role_name LIKE"));
        assert!(bind.is_none());
    }

    #[test]
    fn sort_whitelist_matches_java_entity() {
        assert_eq!(resolve_sort(Some("roleName"), Some("asc")).column, "role_name");
        assert_eq!(resolve_sort(Some("role_name"), Some("asc")).column, "role_name");
        assert_eq!(resolve_sort(Some("roleDesc"), Some("asc")).column, "role_desc");
        assert_eq!(resolve_sort(Some("adminType"), Some("asc")).column, "admin_type");
        assert_eq!(resolve_sort(Some("isSystem"), Some("asc")).column, "is_system");
        // 主键属性名 / 空 → 主键列
        assert_eq!(resolve_sort(Some("roleId"), Some("asc")).column, "role_id");
        assert_eq!(resolve_sort(Some(""), Some("asc")).column, "role_id");
        assert_eq!(resolve_sort(None, None).column, "role_id");
        // 未知字段 / 注入串 → 主键列
        assert_eq!(resolve_sort(Some("nickname"), Some("asc")).column, "role_id");
        assert_eq!(
            resolve_sort(Some("role_id; DROP TABLE admin_role"), Some("asc")).column,
            "role_id"
        );
    }

    #[test]
    fn sort_direction_third_branch_disables_order_by() {
        assert_eq!(resolve_sort(None, Some("ASC")).ascending, Some(true));
        assert_eq!(resolve_sort(None, Some("Asc")).ascending, Some(true));
        assert_eq!(resolve_sort(None, Some("Desc")).ascending, Some(false));
        assert_eq!(resolve_sort(None, Some("")).ascending, Some(false));
        assert_eq!(resolve_sort(None, None).ascending, Some(false));
        // 非 asc/desc 的非空值：Java 不会调用 setOrders → 无 ORDER BY
        assert_eq!(resolve_sort(None, Some("random")).ascending, None);
    }
}
