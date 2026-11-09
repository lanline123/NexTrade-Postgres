//! 管理员日志 域仓储（identity::admin_log）。
//!
//! 表名/列名均来自权威 schema `java/database/pure.sql`
//! （核对命令：`cd rust && python tools/schema_parser.py --table <表名>`）：
//! - `admin_log`：`log_id` / `log_time` / `user_id` / `scope_type` / `scope_id` /
//!   `log_info` / `ip_address` / `request_log_id`
//! - `admin_user`：`admin_id` / `username`
//! - `request_log`：`request_log_id` / `url` / `http_method` / `class_method` /
//!   `request_params` / `add_time` / `admin_username` / `admin_id` / `admin_ip` /
//!   `platform`
//!
//! SQL 一律运行时 `sqlx::query_as`（**不用** `query!` 宏），每条 SQL 单行字面量。

use sqlx::{FromRow,PgPool};
use nextrade_core::models::identity::admin_log::{AdminLogPageQuery, AdminLogRequestLogVO, Scope};
use nextrade_core::{Error, Result};

/// 一行管理员日志（只取 Java DTO 用得上的列）。
#[derive(Debug, Clone, FromRow)]
pub struct AdminLogRow {
    /// 日志ID
    pub log_id: i32,
    /// 写日志时间（秒级时间戳）
    pub log_time: i32,
    /// 操作者ID（对应 `admin_user.admin_id`）
    pub user_id: i32,
    /// 日志内容
    pub log_info: String,
    /// 登录 IP
    pub ip_address: String,
    /// 关联请求日志ID（可空）
    pub request_log_id: Option<i32>,
}

/// 排序解析结果（`None` = 不拼 ORDER BY，对应 Java 的第三分支）。
#[derive(Debug, Clone)]
pub struct Sort {
    /// 白名单化后的列名
    pub column: &'static str,
    /// 升/降/不排序
    pub ascending: Option<bool>,
}

/// Java `AdminLog` 主键列（`BaseServiceImpl#getKeyColumn()`）。
const KEY_COLUMN: &str = "log_id";
/// Java `AdminLog` 主键属性名（`getKeyProperty()`）。
const KEY_PROPERTY: &str = "logId";

/// `AdminLog` 实体的「属性名 → 列名」映射。
///
/// 逐条来自 Java 模型
/// `java/nextrade-bean/src/main/java/com/nextrade/bean/model/authority/AdminLog.java`
/// 的 `@TableField` 声明（该实体**没有** `exist = false` 字段）。
const SORTABLE: [(&str, &str); 8] = [
    ("logId", "log_id"),
    ("logTime", "log_time"),
    ("userId", "user_id"),
    ("scopeType", "scope_type"),
    ("scopeId", "scope_id"),
    ("logInfo", "log_info"),
    ("ipAddress", "ip_address"),
    ("requestLogId", "request_log_id"),
];

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct AdminLogRepository {
    pool: PgPool,
}

impl AdminLogRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 管理员日志分页（对应 Java `AdminLogServiceImpl#adminLogPage` 的
    /// `this.page(page, queryWrapper)`）。
    ///
    /// WHERE = `keyword` 的 `(log_info LIKE OR ip_address LIKE)` **AND**
    /// 作用域条件（[`Scope::All`] 不加条件）。
    pub async fn paginate(
        &self,
        q: &AdminLogPageQuery,
        scope: &Scope,
        sort: &Sort,
    ) -> Result<(u64, Vec<AdminLogRow>)> {
        let (where_sql, kw_bind) = build_where(q, scope);
        let (size, offset) = q.limit_offset();

        let total_sql = format!("SELECT COUNT(*) FROM admin_log{where_sql}");
        let total: (i64,) = if let Some(v) = kw_bind.as_ref() {
            sqlx::query_as::<_, (i64,)>(&total_sql)
                .bind(v)
                .bind(v)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?
        } else {
            sqlx::query_as::<_, (i64,)>(&total_sql)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?
        };

        let order_sql = match sort.ascending {
            Some(asc) => format!(
                " ORDER BY {} {}",
                sort.column,
                if asc { "ASC" } else { "DESC" }
            ),
            None => String::new(),
        };
        let rows_sql = format!(
            "SELECT log_id, log_time, user_id, log_info, ip_address, request_log_id FROM admin_log{where_sql}{order_sql} LIMIT ? OFFSET ?"
        );
        let rows = if let Some(v) = kw_bind.as_ref() {
            sqlx::query_as::<_, AdminLogRow>(&rows_sql)
                .bind(v)
                .bind(v)
                .bind(i64::try_from(size).unwrap_or(0))
                .bind(i64::try_from(offset).unwrap_or(0))
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?
        } else {
            sqlx::query_as::<_, AdminLogRow>(&rows_sql)
                .bind(i64::try_from(size).unwrap_or(0))
                .bind(i64::try_from(offset).unwrap_or(0))
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?
        };

        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 批量取管理员用户名（对应 Java
    /// `adminUserMapper.selectBatchIds(userIds)` → `Collectors.toMap(
    /// AdminUser::getAdminId, AdminUser::getUsername)`）。
    ///
    /// 返回 `admin_id → username`；查不到的 id 不出现在映射里，
    /// 服务层据此把 `username` 置 `null`（与 Java 一致）。
    pub async fn usernames_by_admin_id(&self, admin_ids: &[i32]) -> Result<Vec<(i32, String)>> {
        if admin_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; admin_ids.len()].join(", ");
        let sql = format!("SELECT admin_id, username FROM admin_user WHERE admin_id IN ({placeholders})");
        let mut q = sqlx::query_as::<_, (i32, String)>(&sql);
        for id in admin_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows)
    }

    /// 批量取请求日志（对应 Java 在循环里逐条
    /// `requestLogMapper.selectById(requestLogId)`；这里按 `id` 去重后一次
    /// `IN` 查询，结果集与逐条查询完全相同，只是少了往返）。
    pub async fn request_logs_by_ids(
        &self,
        request_log_ids: &[i32],
    ) -> Result<Vec<AdminLogRequestLogVO>> {
        if request_log_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; request_log_ids.len()].join(", ");
        let sql = format!(
            "SELECT request_log_id, url, http_method, class_method, request_params, add_time, admin_username, admin_id, admin_ip, platform FROM request_log WHERE request_log_id IN ({placeholders})"
        );
        let mut q = sqlx::query_as::<_, RequestLogRow>(&sql);
        for id in request_log_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().map(AdminLogRequestLogVO::from).collect())
    }

    /// 插入管理日志记录（对应 Java AdminLogService#createByLogInfo / save）。
    pub async fn insert_log(
        &self,
        log_time: i32,
        user_id: i32,
        scope_type: &str,
        scope_id: i32,
        log_info: &str,
        ip_address: &str,
        request_log_id: Option<i32>,
    ) -> Result<i32> {
        let sql = "INSERT INTO admin_log (log_time, user_id, scope_type, scope_id, log_info, ip_address, request_log_id) VALUES ($1, $2, $3, $4, $5, $6, $7)";
        let res: i32 = sqlx::query_scalar(&format!("{} RETURNING log_id", sql))
            .bind(log_time)
            .bind(user_id)
            .bind(scope_type)
            .bind(scope_id)
            .bind(log_info)
            .bind(ip_address)
            .bind(request_log_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res)
    }
}

/// `request_log` 行（列顺序与上面 SELECT 一致）。
#[derive(Debug, Clone, FromRow)]
pub struct RequestLogRow {
    /// 主键
    pub request_log_id: i32,
    /// 地址
    pub url: String,
    /// 请求类型
    pub http_method: String,
    /// 类名 + 方法名
    pub class_method: String,
    /// 请求参数
    pub request_params: Option<String>,
    /// 操作时间（varchar）
    pub add_time: Option<String>,
    /// 后台用户名称
    pub admin_username: Option<String>,
    /// 后台用户id
    pub admin_id: Option<i32>,
    /// ip地址
    pub admin_ip: Option<String>,
    /// 平台信息
    pub platform: Option<String>,
}

impl From<RequestLogRow> for AdminLogRequestLogVO {
    fn from(r: RequestLogRow) -> Self {
        Self {
            request_log_id: r.request_log_id,
            url: r.url,
            http_method: r.http_method,
            class_method: r.class_method,
            request_params: r.request_params,
            add_time: r.add_time,
            admin_username: r.admin_username,
            admin_id: r.admin_id,
            admin_ip: r.admin_ip,
            platform: r.platform,
        }
    }
}

/// 拼装 WHERE。第二个返回值是关键字 LIKE 的绑定值
/// （Java 的 `and(w -> w.like(logInfo, kw).or().like(ipAddress, kw))`
/// 里两个 `like` 共用同一个 keyword）。
fn build_where(q: &AdminLogPageQuery, scope: &Scope) -> (String, Option<String>) {
    let mut conds: Vec<String> = Vec::new();
    let mut kw_bind: Option<String> = None;

    // 关键字：log_info LIKE 或 ip_address LIKE（同一个 %kw%）
    if let Some(kw) = q
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        conds.push("(log_info LIKE ? OR ip_address LIKE ?)".to_string());
        kw_bind = Some(format!("%{kw}%"));
    }

    // 作用域：admin → 无条件；shop / vendor → scope_type + scope_id
    match scope {
        Scope::All => {}
        Scope::Shop { shop_id } => {
            conds.push(format!("scope_type = 'shop' AND scope_id = {shop_id}"));
        }
        Scope::Vendor { vendor_id } => {
            conds.push(format!("scope_type = 'vendor' AND scope_id = {vendor_id}"));
        }
        // 服务层已短路，不会走到这里；兜底成恒假
        Scope::Nothing => conds.push("1 = 0".to_string()),
    }

    if conds.is_empty() {
        (String::new(), None)
    } else {
        (format!(" WHERE {}", conds.join(" AND ")), kw_bind)
    }
}

/// Java `buildSortOrder(page, sortField, sortOrder)` 的等价实现
/// （规则与 user_balance_log 的同名函数一致，只是白名单换成 `AdminLog` 实体）。
#[must_use]
pub fn resolve_sort(sort_field: Option<&str>, sort_order: Option<&str>) -> Sort {
    let column = match sort_field {
        None | Some("") | Some(KEY_PROPERTY) => KEY_COLUMN,
        Some(raw) => SORTABLE
            .iter()
            .find(|(property, column)| *property == raw || *column == raw)
            .map_or(KEY_COLUMN, |(_, column)| *column),
    };
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

    fn q(json: serde_json::Value) -> AdminLogPageQuery {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn no_keyword_no_scope_means_no_where() {
        let (sql, bind) = build_where(&q(serde_json::json!({})), &Scope::All);
        assert_eq!(sql, "");
        assert!(bind.is_none());
    }

    #[test]
    fn keyword_becomes_single_bind_shared_by_two_likes() {
        let (sql, bind) = build_where(&q(serde_json::json!({"keyword": " 10.0.0.1 " })), &Scope::All);
        assert_eq!(sql, " WHERE (log_info LIKE ? OR ip_address LIKE ?)");
        assert_eq!(bind.as_deref(), Some("%10.0.0.1%"));
    }

    #[test]
    fn blank_keyword_is_ignored_like_java_isnotblank() {
        let (sql, bind) = build_where(&q(serde_json::json!({"keyword": "   "})), &Scope::All);
        assert_eq!(sql, "");
        assert!(bind.is_none());
    }

    #[test]
    fn scope_conditions_follow_java_wrapper() {
        let (sql, _) = build_where(
            &q(serde_json::json!({"keyword": "x"})),
            &Scope::Shop { shop_id: 7 },
        );
        assert_eq!(
            sql,
            " WHERE (log_info LIKE ? OR ip_address LIKE ?) AND scope_type = 'shop' AND scope_id = 7"
        );

        let (sql, _) = build_where(&q(serde_json::json!({})), &Scope::Vendor { vendor_id: 3 });
        assert_eq!(sql, " WHERE scope_type = 'vendor' AND scope_id = 3");

        let (sql, _) = build_where(&q(serde_json::json!({})), &Scope::Nothing);
        assert_eq!(sql, " WHERE 1 = 0");
    }

    #[test]
    fn sort_whitelist_matches_java_admin_log_entity() {
        assert_eq!(resolve_sort(Some("logTime"), Some("asc")).column, "log_time");
        assert_eq!(resolve_sort(Some("ip_address"), Some("asc")).column, "ip_address");
        assert_eq!(resolve_sort(Some("scopeType"), Some("desc")).column, "scope_type");
        assert_eq!(resolve_sort(Some("logId"), None).column, "log_id");
        // 未知字段 / 注入串 → 主键列
        assert_eq!(resolve_sort(Some("username"), Some("asc")).column, "log_id");
        assert_eq!(
            resolve_sort(Some("log_id; DELETE admin_log"), Some("asc")).column,
            "log_id"
        );
        assert_eq!(resolve_sort(None, None).column, "log_id");
    }

    #[test]
    fn sort_direction_third_branch_disables_order_by() {
        assert_eq!(resolve_sort(None, Some("ASC")).ascending, Some(true));
        assert_eq!(resolve_sort(None, Some("Desc")).ascending, Some(false));
        assert_eq!(resolve_sort(None, Some("")).ascending, Some(false));
        assert_eq!(resolve_sort(None, None).ascending, Some(false));
        assert_eq!(resolve_sort(None, Some("random")).ascending, None);
    }

    #[test]
    fn scope_nothing_falls_back_to_always_false() {
        // 服务层会先短路返回空页；仓储兜底条件保证万一被调用也查不到行
        let (sql, _) = build_where(&q(serde_json::json!({"keyword": "k"})), &Scope::Nothing);
        assert_eq!(
            sql,
            " WHERE (log_info LIKE ? OR ip_address LIKE ?) AND 1 = 0"
        );
    }
}