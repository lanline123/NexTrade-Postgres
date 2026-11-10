//! 发票资质域仓储（payment::user_invoice）。
//!
//! 对应原 Java `UserInvoiceMapper` / `UserInvoiceServiceImpl#list` 的
//! `LambdaQueryWrapper` 查询。
//!
//! 表名/列名严格对齐 `java/database/pure.sql`
//! （核对命令：`cd rust && python tools/schema_parser.py --table user_invoice`）：
//! - `user_invoice`（15 列）：`invoice_id` / `user_id` / `status` / `apply_reply` /
//!   `add_time` / `title_type` / `company_name` / `company_code` /
//!   `company_address` / `company_phone` / `company_bank` / `company_account` /
//!   `mobile` / `email` / `invoice_content`
//! - `user`（只读 `user_id` / `username`，用于列表关键字与 `username` 装配）
//!
//! SQL 一律运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`
//! （**不用** `query!` 宏，本机无 `DATABASE_URL`）。
//!
//! ## 与 Java 查询的对应关系
//!
//! ```java
//! if (status != null && status > -1) queryWrapper.eq(UserInvoice::getStatus, status);
//! if (isNotEmpty(keyword)) {
//!     List<Integer> matchingUserIds = userMapper.selectList(
//!         new LambdaQueryWrapper<User>().like(User::getUsername, keyword)) ...;
//!     queryWrapper.and(wrapper -> {
//!         wrapper.like(UserInvoice::getCompanyName, keyword);
//!         if (!matchingUserIds.isEmpty()) wrapper.or().in(UserInvoice::getUserId, matchingUserIds);
//!     });
//! }
//! ```
//!
//! 即：`status` 与 keyword 可叠加；keyword 的 IN 集合来自 `user.username LIKE`，
//! 集合为空时**只**按 `company_name` 过滤（不产生空 `IN ()`）。

use sqlx::{Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::UserInvoice;
use nextrade_core::models::payment::user_invoice::UserInvoiceListQuery;
use nextrade_core::{Error, Result};

/// `user_invoice` 固定 SELECT 列清单（顺序与实体一致）。
pub const USER_INVOICE_COLUMNS: [&str; 15] = [
    "invoice_id",
    "user_id",
    "status",
    "apply_reply",
    "add_time",
    "title_type",
    "company_name",
    "company_code",
    "company_address",
    "company_phone",
    "company_bank",
    "company_account",
    "mobile",
    "email",
    "invoice_content",
];

/// 待绑定参数（按 SQL 中 `?` 的出现顺序 1:1 对应）。
#[derive(Debug, Clone)]
pub enum Bind {
    /// 32 位整数
    I32(i32),
    /// 字符串
    Str(String),
}

/// 把 `Bind` 列表按顺序绑定到查询上。
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

/// 发票资质域仓储。
#[derive(Clone)]
pub struct UserInvoiceRepository {
    pool: PgPool,
}

impl UserInvoiceRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 查询 —————————————————————

    /// 按主键查询（Java `getById`）。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<UserInvoice>> {
        let sql = format!(
            "SELECT {} FROM user_invoice WHERE invoice_id = $1 LIMIT 1",
            USER_INVOICE_COLUMNS.join(", ")
        );
        sqlx::query_as::<_, UserInvoice>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 分页列表（Java `UserInvoiceServiceImpl#list` 的 `page(page, queryWrapper)`）。
    ///
    /// 返回 `(总记录数, 当前页行)`；总数与行使用**同一套 WHERE**。
    pub async fn paginate(
        &self,
        q: &UserInvoiceListQuery,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<UserInvoice>)> {
        let user_ids = match q.effective_keyword() {
            Some(keyword) => self.find_user_ids_by_username(keyword).await?,
            None => Vec::new(),
        };
        let (where_sql, binds) = build_where(q, q.effective_keyword(), &user_ids);

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM user_invoice{where_sql}"));
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&count_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {} FROM user_invoice{where_sql}{} LIMIT {limit} OFFSET {offset}",
            USER_INVOICE_COLUMNS.join(", "),
            q.order_clause()
        ));
        let rows: Vec<UserInvoice> =
            bind_all!(sqlx::query_as::<_, UserInvoice>(&rows_sql), binds)
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok((total.0, rows))
    }

    /// 用户名模糊命中的 `user_id`（`user.username LIKE %kw%`）。
    pub async fn find_user_ids_by_username(&self, keyword: &str) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as("SELECT user_id FROM \"user\" WHERE username LIKE $1")
            .bind(format!("%{keyword}%"))
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 批量取用户名（列表 `username` 列 / detail 单个用户名）。
    pub async fn find_usernames(&self, ids: &[i32]) -> Result<Vec<(i32, String)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT user_id, username FROM \"user\" WHERE user_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    // ————————————————————— 写操作 —————————————————————

    /// 新增（Java `save`；`add_time` 由服务层覆盖为当前秒）。
    pub async fn insert(&self, entity: &UserInvoice) -> Result<i32> {
        let sql = format!(
            "INSERT INTO user_invoice ({}) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
            USER_INVOICE_COLUMNS[1..].join(", ")
        );
        let res: i32 = sqlx::query_scalar(&format!("{} RETURNING invoice_id", &sql))
            .bind(entity.user_id)
            .bind(entity.status)
            .bind(&entity.apply_reply)
            .bind(entity.add_time)
            .bind(entity.title_type)
            .bind(&entity.company_name)
            .bind(&entity.company_code)
            .bind(&entity.company_address)
            .bind(&entity.company_phone)
            .bind(&entity.company_bank)
            .bind(&entity.company_account)
            .bind(&entity.mobile)
            .bind(&entity.email)
            .bind(&entity.invoice_content)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res)
    }

    /// 整行更新（Java `updateById`；服务层先合并旧值，等价 NOT_NULL 语义）。
    pub async fn update(&self, entity: &UserInvoice) -> Result<u64> {
        let sql = "UPDATE user_invoice SET user_id = $1, status = $2, apply_reply = $3, add_time = $4, title_type = $5, company_name = $6, company_code = $7, company_address = $8, company_phone = $9, company_bank = $10, company_account = $11, mobile = $12, email = $13, invoice_content = $14 WHERE invoice_id = $15";
        let res = sqlx::query(sql)
            .bind(entity.user_id)
            .bind(entity.status)
            .bind(&entity.apply_reply)
            .bind(entity.add_time)
            .bind(entity.title_type)
            .bind(&entity.company_name)
            .bind(&entity.company_code)
            .bind(&entity.company_address)
            .bind(&entity.company_phone)
            .bind(&entity.company_bank)
            .bind(&entity.company_account)
            .bind(&entity.mobile)
            .bind(&entity.email)
            .bind(&entity.invoice_content)
            .bind(entity.invoice_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 单字段更新（列名由服务层白名单归一后传入）。
    pub async fn update_field(&self, id: i32, column: &str, val: &str) -> Result<u64> {
        debug_assert!(
            nextrade_core::models::payment::user_invoice::GENERAL_FIELDS.contains(&column),
            "update_field 列名未经过白名单映射: {column}"
        );
        let sql = format!("UPDATE user_invoice SET {column} = $1 WHERE invoice_id = $2");
        let res = sqlx::query(&sql)
            .bind(val)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 按主键删除（Java `removeById`）。
    pub async fn delete(&self, id: i32) -> Result<bool> {
        let res = sqlx::query("DELETE FROM user_invoice WHERE invoice_id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 批量删除（Java `removeByIds`）。
    pub async fn delete_batch(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("DELETE FROM user_invoice WHERE invoice_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        let res = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }
}

/// 拼装 WHERE 与绑定参数（对应 Java wrapper 链；返回的 SQL 以 `" WHERE "` 开头）。
///
/// `user_ids` 是 `user.username LIKE keyword` 命中的集合：
/// 非空时生成 `(company_name LIKE ? OR user_id IN (?, ...))`，空时只按
/// `company_name` 过滤（与 Java 分支一致）。
#[must_use]
pub fn build_where(
    q: &UserInvoiceListQuery,
    keyword: Option<&str>,
    user_ids: &[i32],
) -> (String, Vec<Bind>) {
    let mut conds: Vec<String> = Vec::new();
    let mut binds: Vec<Bind> = Vec::new();

    if let Some(keyword) = keyword {
        if user_ids.is_empty() {
            conds.push("(company_name LIKE ?)".to_string());
        } else {
            let placeholders = vec!["?"; user_ids.len()].join(", ");
            conds.push(format!(
                "(company_name LIKE ? OR user_id IN ({placeholders}))"
            ));
        }
        binds.push(Bind::Str(format!("%{keyword}%")));
        for id in user_ids {
            binds.push(Bind::I32(*id));
        }
    }

    // Java: `status != null && status > -1`
    if let Some(status) = q.effective_status() {
        conds.push("status = ?".to_string());
        binds.push(Bind::I32(status));
    }

    if conds.is_empty() {
        (String::new(), binds)
    } else {
        (format!(" WHERE {}", conds.join(" AND ")), binds)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(json: serde_json::Value) -> UserInvoiceListQuery {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn no_filters_no_where() {
        let (sql, binds) = build_where(&q(serde_json::json!({})), None, &[]);
        assert_eq!(sql, "");
        assert!(binds.is_empty());
    }

    #[test]
    fn keyword_without_matched_users_filters_company_name_only() {
        let (sql, binds) = build_where(
            &q(serde_json::json!({"keyword": "NexTrade"})),
            Some("NexTrade"),
            &[],
        );
        assert_eq!(sql, " WHERE (company_name LIKE ?)");
        assert!(matches!(&binds[0], Bind::Str(s) if s == "%NexTrade%"));
    }

    #[test]
    fn keyword_with_matched_users_adds_or_in_clause() {
        let (sql, binds) = build_where(
            &q(serde_json::json!({"keyword": "bob", "status": 2})),
            Some("bob"),
            &[3, 8],
        );
        assert_eq!(
            sql,
            " WHERE (company_name LIKE ? OR user_id IN (?, ?)) AND status = ?"
        );
        // 绑定顺序：keyword → user_ids → status
        assert!(matches!(&binds[0], Bind::Str(s) if s == "%bob%"));
        assert!(matches!(binds[1], Bind::I32(3)));
        assert!(matches!(binds[2], Bind::I32(8)));
        assert!(matches!(binds[3], Bind::I32(2)));
    }

    #[test]
    fn status_zero_applies_but_minus_one_is_ignored() {
        // Java: status != null && status > -1（0 是有效筛选值）
        let (sql, binds) = build_where(&q(serde_json::json!({"status": 0})), None, &[]);
        assert_eq!(sql, " WHERE status = ?");
        assert!(matches!(binds.as_slice(), [Bind::I32(0)]));
        let (sql, binds) = build_where(&q(serde_json::json!({"status": -1})), None, &[]);
        assert_eq!(sql, "");
        assert!(binds.is_empty());
    }

    #[test]
    fn select_list_and_static_guards() {
        assert_eq!(USER_INVOICE_COLUMNS.len(), 15);
        assert_eq!(USER_INVOICE_COLUMNS[0], "invoice_id");
        // insert 使用 [1..] 跳过自增主键
        assert!(!USER_INVOICE_COLUMNS[1..].contains(&"invoice_id"));
        let src = include_str!("user_invoice.rs");
        assert!(src.contains(r#"SELECT user_id FROM \"user\" WHERE username LIKE $1"#));
        assert!(src.contains("OR user_id IN ({placeholders})"));
    }
}
