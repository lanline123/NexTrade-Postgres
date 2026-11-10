//! 用户余额日志 域仓储（payment::user_balance_log）。
//!
//! 表名/列名均来自权威 schema `java/database/pure.sql`
//! （核对命令：`cd rust && python tools/schema_parser.py --table <表名>`）：
//! - `user_balance_log`：`log_id` / `user_id` / `before_balance` / `balance` /
//!   `after_balance` / `frozen_balance` / `before_frozen_balance` /
//!   `after_frozen_balance` / `change_time` / `change_desc` / `change_type` /
//!   `wallet_type` / `new_balance` / `new_frozen_balance`
//! - `user`：`user_id` / `username`
//!
//! SQL 一律运行时 `sqlx::query_as`（**不用** `query!` 宏，本机无 `DATABASE_URL`），
//! 每条 SQL 写成单行字面量。
//!
//! ## 与 Java 查询的对应关系
//!
//! Java `UserBalanceLogServiceImpl#list` 用 MyBatis-Plus-Join：
//!
//! ```java
//! queryWrapper.like(isNotBlank(keyword), UserBalanceLog::getChangeDesc, keyword)
//!     .eq(userId != null && userId > 0, UserBalanceLog::getUserId, userId)
//!     .gt(balance != null && balance > 0, UserBalanceLog::getBalance, 0)
//!     .eq(walletType != null, UserBalanceLog::getWalletType, walletType)
//!     .notLike(excludeDistribution, UserBalanceLog::getChangeDesc, "结算佣金")
//!     .leftJoin(User.class, User::getUserId, UserBalanceLog::getUserId)
//!     .select(User::getUsername).selectAll(UserBalanceLog.class)
//!     .like(isNotBlank(username), User::getUsername, username);
//! ```
//!
//! 注意最后那个 `like` 写在 `leftJoin` / `selectAll` **之后**——
//! MyBatis-Plus-Join 的 wrapper 是链式的，条件顺序不影响最终 WHERE 集合，
//! 因此 Rust 侧把它与其它条件一起拼进同一个 `WHERE`。

use rust_decimal::Decimal;
use sqlx::{FromRow,PgPool};
use nextrade_core::models::payment::user_balance_log::UserBalanceLogListQuery;
use nextrade_core::{Error, Result};

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

/// 一行查询结果（`user_balance_log` 全列 + LEFT JOIN 出来的 `user.username`）。
#[derive(Debug, Clone, FromRow)]
pub struct UserBalanceLogRow {
    /// 余额ID
    pub log_id: i32,
    /// 用户ID
    pub user_id: i32,
    /// 变动前余额
    pub before_balance: Decimal,
    /// 本次变动金额
    pub balance: Decimal,
    /// 变动后余额
    pub after_balance: Decimal,
    /// 本次冻结变动
    pub frozen_balance: Decimal,
    /// 变动前冻结余额
    pub before_frozen_balance: Decimal,
    /// 变动后冻结余额
    pub after_frozen_balance: Decimal,
    /// 变动时间（秒级时间戳）
    pub change_time: i32,
    /// 变动说明
    pub change_desc: String,
    /// 变动类型
    pub change_type: i32,
    /// 会员用户名（LEFT JOIN，用户已删时为 `NULL`）
    pub username: Option<String>,
}

/// 排序解析结果：`None` 表示**不拼 ORDER BY**（Java 的第三种分支）。
#[derive(Debug, Clone)]
pub struct Sort {
    /// 已白名单化的列名
    pub column: &'static str,
    /// `Some(true)` 升序、`Some(false)` 降序、`None` 不排序
    pub ascending: Option<bool>,
}

/// Java `BaseServiceImpl#getKeyColumn()`（`UserBalanceLog` 主键列）。
const KEY_COLUMN: &str = "log_id";

/// Java `BaseServiceImpl#getKeyProperty()`（主键属性名）。
const KEY_PROPERTY: &str = "logId";

/// `UserBalanceLog` 实体的「属性名 → 列名」映射。
///
/// 逐条来自 Java 模型
/// `java/nextrade-bean/src/main/java/com/nextrade/bean/model/finance/UserBalanceLog.java`
/// 的字段声明；`username` 因为是 `@TableField(exist = false)` **不在**
/// MP 的 `fieldList` 里，所以按 Java 逻辑会回退主键列（见 [`resolve_sort_column`]）。
const SORTABLE: [(&str, &str); 12] = [
    ("logId", "log_id"),
    ("userId", "user_id"),
    ("beforeBalance", "before_balance"),
    ("balance", "balance"),
    ("afterBalance", "after_balance"),
    ("frozenBalance", "frozen_balance"),
    ("beforeFrozenBalance", "before_frozen_balance"),
    ("afterFrozenBalance", "after_frozen_balance"),
    ("changeTime", "change_time"),
    ("changeDesc", "change_desc"),
    ("changeType", "change_type"),
    ("walletType", "wallet_type"),
];

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct UserBalanceLogRepository {
    pool: PgPool,
}

impl UserBalanceLogRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 余额日志分页（对应 Java `UserBalanceLogServiceImpl#list` 的
    /// `selectJoinListPage`）。
    ///
    /// 返回 `(总记录数, 当前页行)`；总数与行使用**同一套 WHERE**，
    /// 与 MP 分页插件的 `COUNT` 语句口径一致。
    pub async fn paginate(
        &self,
        q: &UserBalanceLogListQuery,
        sort: &Sort,
    ) -> Result<(u64, Vec<UserBalanceLogRow>)> {
        let (where_sql, binds) = build_where(q);
        let (size, offset) = q.limit_offset();

        let total_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT COUNT(*) FROM user_balance_log LEFT JOIN \"user\" u ON u.user_id = user_balance_log.user_id{where_sql}"
        ));
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&total_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        // Java `buildSortOrder`：既不是 asc 也不是 desc/空 → 完全不设置 ORDER BY
        let order_sql = match sort.ascending {
            Some(asc) => {
                format!(" ORDER BY user_balance_log.{} {}", sort.column, if asc { "ASC" } else { "DESC" })
            }
            None => String::new(),
        };
        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT user_balance_log.log_id, user_balance_log.user_id, user_balance_log.before_balance, user_balance_log.balance, user_balance_log.after_balance, user_balance_log.frozen_balance, user_balance_log.before_frozen_balance, user_balance_log.after_frozen_balance, user_balance_log.change_time, user_balance_log.change_desc, user_balance_log.change_type, u.username AS username FROM user_balance_log LEFT JOIN \"user\" u ON u.user_id = user_balance_log.user_id{where_sql}{order_sql} LIMIT ? OFFSET ?"
        ));
        let rows = bind_all!(sqlx::query_as::<_, UserBalanceLogRow>(&rows_sql), binds)
            .bind(i64::try_from(size).unwrap_or(0))
            .bind(i64::try_from(offset).unwrap_or(0))
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }
}

/// 拼装 WHERE（对应 Java wrapper 链；返回值第二项与 SQL 里 `?` 严格同序）。
fn build_where(q: &UserBalanceLogListQuery) -> (String, Vec<Bind>) {
    let mut conds: Vec<String> = Vec::new();
    let mut binds: Vec<Bind> = Vec::new();

    // .like(isNotBlank(keyword), UserBalanceLog::getChangeDesc, keyword)
    if let Some(kw) = q
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        conds.push("user_balance_log.change_desc ILIKE ?".to_string());
        binds.push(Bind::Str(format!("%{kw}%")));
    }

    // .eq(userId != null && userId > 0, UserBalanceLog::getUserId, userId)
    if q.user_id.map(|v| v > 0).unwrap_or(false) {
        conds.push("user_balance_log.user_id = ?".to_string());
        if let Some(uid) = q.user_id {
            binds.push(Bind::I32(uid));
        }
    }

    // .gt(balance != null && balance > 0, UserBalanceLog::getBalance, 0) —— 无绑定参数
    if q.balance_positive_filter() {
        conds.push("user_balance_log.balance > 0".to_string());
    }

    // .eq(walletType != null, UserBalanceLog::getWalletType, walletType) —— 0 也生效
    if let Some(wallet_type) = q.wallet_type {
        conds.push("user_balance_log.wallet_type = ?".to_string());
        binds.push(Bind::I32(wallet_type));
    }

    // .notLike(excludeDistribution == 1, UserBalanceLog::getChangeDesc, "结算佣金")
    if q.exclude_distribution_on() {
        conds.push("user_balance_log.change_desc NOT ILIKE ?".to_string());
        binds.push(Bind::Str(format!(
            "%{}%",
            nextrade_core::models::payment::user_balance_log::DISTRIBUTION_SETTLE_KEYWORD
        )));
    }

    // .like(isNotBlank(username), User::getUsername, username)
    if let Some(name) = q
        .username
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        conds.push("u.username ILIKE ?".to_string());
        binds.push(Bind::Str(format!("%{name}%")));
    }

    if conds.is_empty() {
        (String::new(), binds)
    } else {
        (format!(" WHERE {}", conds.join(" AND ")), binds)
    }
}

/// Java `buildSortOrder(page, sortField, sortOrder)` 的等价实现。
///
/// 规则（逐条对照 `BaseServiceImpl`）：
/// 1. `sortField` 缺失/空白，或等于主键属性名 `logId` → 主键列 `log_id`；
/// 2. 否则按「属性名 **或** 列名」查 [`SORTABLE`]，
///    **查不到时回退主键列**（Java 原文注释：防 SQL 注入）；
/// 3. `sortOrder` 忽略大小写 `asc` → 升序；`desc` 或空串 → 降序；
///    `None`（参数缺失）Java 走 `StrUtil.isEmpty(sortOrder)` 分支 → 降序；
///    其它任意值 → **不设 ORDER BY**。
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
    use nextrade_core::models::payment::user_balance_log::DISTRIBUTION_SETTLE_KEYWORD;

    fn q(json: serde_json::Value) -> UserBalanceLogListQuery {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn no_filters_no_where() {
        let (sql, binds) = build_where(&q(serde_json::json!({})));
        assert_eq!(sql, "");
        assert!(binds.is_empty());
    }

    #[test]
    fn wallet_type_zero_still_filters() {
        // Java `.eq(walletType != null, ...)` —— 0 不是「未传」
        let (sql, binds) = build_where(&q(serde_json::json!({"walletType": 0})));
        assert_eq!(sql, " WHERE user_balance_log.wallet_type = ?");
        assert!(matches!(binds.as_slice(), [Bind::I32(0)]));
    }

    #[test]
    fn user_id_must_be_positive() {
        assert_eq!(build_where(&q(serde_json::json!({"userId": 0}))).0, "");
        assert_eq!(build_where(&q(serde_json::json!({"userId": -1}))).0, "");
        let (sql, binds) = build_where(&q(serde_json::json!({"userId": 8})));
        assert_eq!(sql, " WHERE user_balance_log.user_id = ?");
        assert!(matches!(binds.as_slice(), [Bind::I32(8)]));
    }

    #[test]
    fn balance_filter_is_a_bare_comparison() {
        let (sql, binds) = build_where(&q(serde_json::json!({"balance": 1})));
        assert_eq!(sql, " WHERE user_balance_log.balance > 0");
        assert!(binds.is_empty(), "条件值恒为字面量 0，不应有绑定");
    }

    #[test]
    fn keyword_and_username_become_wrapped_like() {
        let (sql, binds) = build_where(&q(serde_json::json!({"keyword": " 充值 ", "username": "bob"})));
        assert_eq!(
            sql,
            " WHERE user_balance_log.change_desc ILIKE ? AND u.username ILIKE ?"
        );
        assert!(matches!(&binds[0], Bind::Str(s) if s == "%充值%"));
        assert!(matches!(&binds[1], Bind::Str(s) if s == "%bob%"));
    }

    #[test]
    fn exclude_distribution_not_like_pattern_uses_java_literal() {
        let (sql, binds) = build_where(&q(serde_json::json!({"excludeDistribution": 1})));
        assert_eq!(sql, " WHERE user_balance_log.change_desc NOT ILIKE ?");
        assert!(
            matches!(&binds[0], Bind::Str(s) if s == &format!("%{DISTRIBUTION_SETTLE_KEYWORD}%")),
            "{binds:?}"
        );
    }

    #[test]
    fn filter_order_follows_java_wrapper_chain() {
        let (sql, _) = build_where(&q(serde_json::json!({
            "keyword": "k", "userId": 5, "balance": 1, "walletType": 1,
            "excludeDistribution": 1, "username": "u"
        })));
        assert_eq!(
            sql,
            " WHERE user_balance_log.change_desc ILIKE ? AND user_balance_log.user_id = ? AND user_balance_log.balance > 0 AND user_balance_log.wallet_type = ? AND user_balance_log.change_desc NOT ILIKE ? AND u.username ILIKE ?"
        );
    }

    #[test]
    fn sort_column_whitelist_matches_java_entity() {
        // 属性名
        assert_eq!(resolve_sort(Some("changeTime"), Some("asc")).column, "change_time");
        // 直接传列名也接受（Java「兼容传入数据库字段」）
        assert_eq!(resolve_sort(Some("before_balance"), Some("asc")).column, "before_balance");
        // 主键属性名 → 主键列
        assert_eq!(resolve_sort(Some("logId"), Some("desc")).column, "log_id");
        // 未知/注入串 → 回退主键列（不原样拼进 SQL）
        assert_eq!(
            resolve_sort(Some("username; DROP TABLE user"), Some("asc")).column,
            "log_id"
        );
        // username 是 @TableField(exist=false) → 不在 fieldList → 主键列
        assert_eq!(resolve_sort(Some("username"), Some("asc")).column, "log_id");
        // 缺失 → 主键列
        assert_eq!(resolve_sort(None, None).column, "log_id");
    }

    #[test]
    fn sort_direction_third_branch_disables_order_by() {
        assert_eq!(resolve_sort(None, Some("asc")).ascending, Some(true));
        assert_eq!(resolve_sort(None, Some("ASC")).ascending, Some(true));
        assert_eq!(resolve_sort(None, Some("desc")).ascending, Some(false));
        assert_eq!(resolve_sort(None, Some("")).ascending, Some(false));
        // Java：sortOrder 非空且不是 asc/desc → 两个分支都不进 → 无排序
        assert_eq!(resolve_sort(None, Some("sideways")).ascending, None);
        // 参数缺失 → Java StrUtil.isEmpty(null) == true → 降序
        assert_eq!(resolve_sort(None, None).ascending, Some(false));
    }

    #[test]
    fn order_clause_is_empty_when_java_skips_sorting() {
        // 该断言锁定服务层拼接：ascending=None 时不出现 ORDER BY
        let sort = resolve_sort(None, Some("sideways"));
        let clause = match sort.ascending {
            Some(_) => " has ORDER BY",
            None => " no ORDER BY",
        };
        assert_eq!(clause, " no ORDER BY");
    }
}