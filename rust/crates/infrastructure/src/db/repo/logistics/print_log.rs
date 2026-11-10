//! 打印日志域（logistics::print_log）仓储。
//!
//! 对应原 Java：
//! - `com.nextrade.mapper.print.PrintLogMapper`（`BaseMapper<PrintLog>`）
//! - `com.nextrade.service.print.impl.PrintLogServiceImpl#page`
//! - `com.nextrade.service.common.impl.BaseServiceImpl#buildSortOrder`
//!
//! # 列名对齐（权威来源）
//!
//! 表 `print_log`（18 列，主键 `log_id`）来自 `java/database/pure.sql`，
//! 用 `cd rust && python tools/schema_parser.py --table print_log` 核对；
//! 列清单复用 [`nextrade_core::models::logistics::print_config::PRINT_LOG_COLUMNS`]
//! （同一批表只维护一份）。
//!
//! 查询统一使用运行时 `sqlx::query_as`（**不使用** `query!` 宏：
//! 本机无 `DATABASE_URL`）。
//!
//! # 排序（对照 Java）
//!
//! Java `PrintLogServiceImpl#page` 有**两个**排序来源：
//! 1. `Page` 的 `orders`（`buildSortOrder(query)`）：`sortField` 空 → 主键
//!    `log_id`；`sortOrder` 仅 `asc` 升序，其余降序；
//! 2. wrapper 在 `sortField` 为空时追加 `ORDER BY add_time DESC`。
//!
//! MyBatis-Plus 的分页插件把 `Page.orders` **拼到** wrapper 的 ORDER BY
//! **之后**，故默认净效果是 `ORDER BY add_time DESC, log_id DESC`
//! （后者为同秒日志的稳定次序）。本仓储的 [`order_clause`] 完整复刻该语义。
//!
//! # 金额
//!
//! 本域不涉及任何金额列，故无 `rust_decimal::Decimal`。

use sqlx::PgPool;

use nextrade_core::entities::PrintLog;
use nextrade_core::models::logistics::print::to_snake;
use nextrade_core::models::logistics::print_config::PRINT_LOG_COLUMNS;
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// `print_log` 的固定 SELECT 列清单（18 列，顺序与 `entities::PrintLog`
/// 字段一致，避免 `SELECT *` 与实体字段顺序耦合）。
const PRINT_LOG_SELECT: &str = "SELECT log_id, shop_id, print_id, platform, order_id, print_sn, trigger_type, status, error_code, error_msg, task_no, push_response, callback_payload, retry_count, pushed_time, printed_time, add_time, update_time FROM print_log";

/// 分页过滤条件（`None` 表示不过滤），对应 Java `PrintLogPageQuery` 的
/// 六个等值/范围条件 + `printSn` 模糊。
#[derive(Debug, Clone, Default)]
pub struct PrintLogFilters<'a> {
    /// 平台精确（`platform = ?`）
    pub platform: Option<i32>,
    /// 状态精确（`status = ?`）
    pub status: Option<i32>,
    /// 触发方式精确（`trigger_type = ?`）
    pub trigger_type: Option<i32>,
    /// 订单 ID 精确（`order_id = ?`）
    pub order_id: Option<i32>,
    /// 打印机 SN 模糊（`print_sn ILIKE ?`）
    pub print_sn: Option<&'a str>,
    /// 开始时间（`add_time >= ?`，Unix 秒）
    pub start_time: Option<i32>,
    /// 结束时间（`add_time <= ?`，Unix 秒）
    pub end_time: Option<i32>,
}

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct PrintLogRepository {
    pool: PgPool,
}

impl PrintLogRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 打印日志分页（对应 Java `PrintLogServiceImpl#page`）。
    ///
    /// `WHERE shop_id = ?` 恒成立（Java 用 `getShopId()` 过滤），其余条件
    /// 按查询参数可选叠加；排序见 [`order_clause`]。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn paginate(
        &self,
        shop_id: i32,
        filters: &PrintLogFilters<'_>,
        sort_field: Option<&str>,
        sort_order: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<PrintLog>)> {
        let like = like_pattern(filters.print_sn);

        let mut where_parts: Vec<String> = vec!["shop_id = ?".into()];
        if filters.platform.is_some() {
            where_parts.push("platform = ?".into());
        }
        if filters.status.is_some() {
            where_parts.push("status = ?".into());
        }
        if filters.trigger_type.is_some() {
            where_parts.push("trigger_type = ?".into());
        }
        if filters.order_id.is_some() {
            where_parts.push("order_id = ?".into());
        }
        if like.is_some() {
            where_parts.push("print_sn ILIKE ?".into());
        }
        if filters.start_time.is_some() {
            where_parts.push("add_time >= ?".into());
        }
        if filters.end_time.is_some() {
            where_parts.push("add_time <= ?".into());
        }
        let where_sql = format!("WHERE {}", where_parts.join(" AND "));
        let (limit, offset) = page.limit_offset();
        let order_by = order_clause(sort_field, sort_order);

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM print_log {where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        cq = bind_filters(
            cq,
            shop_id,
            filters.platform,
            filters.status,
            filters.trigger_type,
            filters.order_id,
            like.as_deref(),
            filters.start_time,
            filters.end_time,
        );
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let list_sql = crate::db::to_pg_placeholders(&format!("{PRINT_LOG_SELECT} {where_sql} {order_by} LIMIT ? OFFSET ?"));
        let mut lq = sqlx::query_as::<_, PrintLog>(&list_sql);
        lq = bind_filters(
            lq,
            shop_id,
            filters.platform,
            filters.status,
            filters.trigger_type,
            filters.order_id,
            like.as_deref(),
            filters.start_time,
            filters.end_time,
        );
        let rows = lq
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }
}

/// ILIKE 通配模式（Java `wrapper.like(column, value)` 等价物）。
///
/// Java 先 `StrUtil.isNotBlank(printSn)` 再 `like`：空白串**不**加条件。
#[must_use]
pub fn like_pattern(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{s}%"))
}

/// 排序字段白名单解析（防 SQL 注入，对齐
/// `BaseServiceImpl#getColumnByPropertyName`）：属性名 / 列名命中 →
/// 该列；未知回退主键 `log_id`。
#[must_use]
pub fn resolve_sort_column(field: Option<&str>) -> &'static str {
    match field {
        Some(f) => {
            let snake = to_snake(f);
            PRINT_LOG_COLUMNS
                .iter()
                .find(|c| **c == snake)
                .copied()
                .unwrap_or("log_id")
        }
        None => "log_id",
    }
}

/// 完整 ORDER BY 子句（见模块头「排序」）。
///
/// - `sort_field` 缺失 / 空白 → `ORDER BY add_time DESC, log_id DESC`
///   （Java wrapper 的 `orderByDesc(addTime)` + Page 的 `log_id DESC`）；
/// - 否则 → `ORDER BY <列> ASC|DESC`（仅 `asc` 升序，其余降序）。
#[must_use]
pub fn order_clause(sort_field: Option<&str>, sort_order: Option<&str>) -> String {
    match sort_field.map(str::trim).filter(|s| !s.is_empty()) {
        None => "ORDER BY add_time DESC, log_id DESC".to_string(),
        Some(field) => {
            let column = resolve_sort_column(Some(field));
            let dir = if sort_order.is_some_and(|o| o.eq_ignore_ascii_case("asc")) {
                "ASC"
            } else {
                "DESC"
            };
            format!("ORDER BY {column} {dir}")
        }
    }
}

/// 把 WHERE 片段按 [`PrintLogRepository::paginate`] 的拼接顺序绑定到查询上。
///
/// 顺序：`shop_id` → `platform` → `status` → `trigger_type` → `order_id`
/// → `print_sn` → `start_time` → `end_time`。
#[allow(clippy::too_many_arguments)]
fn bind_filters<'q, O>(
    q: sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments>,
    shop_id: i32,
    platform: Option<i32>,
    status: Option<i32>,
    trigger_type: Option<i32>,
    order_id: Option<i32>,
    like: Option<&'q str>,
    start_time: Option<i32>,
    end_time: Option<i32>,
) -> sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments> {
    let mut q = q.bind(shop_id);
    if let Some(v) = platform {
        q = q.bind(v);
    }
    if let Some(v) = status {
        q = q.bind(v);
    }
    if let Some(v) = trigger_type {
        q = q.bind(v);
    }
    if let Some(v) = order_id {
        q = q.bind(v);
    }
    if let Some(v) = like {
        q = q.bind(v);
    }
    if let Some(v) = start_time {
        q = q.bind(v);
    }
    if let Some(v) = end_time {
        q = q.bind(v);
    }
    q
}

/// 数据库错误映射（与同域 print / print_config 同一约定）。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("打印日志不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `PRINT_LOG_SELECT` 覆盖 `print_log` 全部 18 列，顺序与实体一致，
    /// 且不用 `SELECT *`。
    #[test]
    fn print_log_select_covers_all_columns_in_entity_order() {
        let listed: Vec<&str> = PRINT_LOG_SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap()
            .split(", ")
            .collect();
        assert_eq!(listed, PRINT_LOG_COLUMNS.to_vec());
        assert_eq!(listed.len(), 18);
        assert!(!PRINT_LOG_SELECT.contains('*'));
    }

    /// 默认排序：`add_time DESC, log_id DESC`（wrapper 的 add_time 优先，
    /// Page 的主键降序作稳定次序）。
    #[test]
    fn default_order_is_add_time_desc_then_log_id() {
        assert_eq!(order_clause(None, None), "ORDER BY add_time DESC, log_id DESC");
        assert_eq!(
            order_clause(Some(""), Some("asc")),
            "ORDER BY add_time DESC, log_id DESC",
            "空 sortField 走默认分支（与 Java sortField.isEmpty 一致）"
        );
        assert_eq!(
            order_clause(Some("   "), None),
            "ORDER BY add_time DESC, log_id DESC"
        );
    }

    /// 显式排序：属性名 / 列名都认，仅 `asc` 升序，其余降序。
    #[test]
    fn explicit_order_resolves_column_and_direction() {
        assert_eq!(
            order_clause(Some("orderId"), Some("asc")),
            "ORDER BY order_id ASC"
        );
        assert_eq!(
            order_clause(Some("order_id"), Some("ASC")),
            "ORDER BY order_id ASC"
        );
        assert_eq!(
            order_clause(Some("addTime"), None),
            "ORDER BY add_time DESC"
        );
        assert_eq!(
            order_clause(Some("logId"), Some("desc")),
            "ORDER BY log_id DESC"
        );
        // 未知字段回退主键，防 SQL 注入
        assert_eq!(
            order_clause(Some("1;DROP TABLE print_log"), None),
            "ORDER BY log_id DESC"
        );
    }

    /// 排序白名单就是 `print_log` 全部列。
    #[test]
    fn sort_column_whitelist_covers_all_columns() {
        assert_eq!(resolve_sort_column(None), "log_id");
        assert_eq!(resolve_sort_column(Some("logId")), "log_id");
        assert_eq!(resolve_sort_column(Some("status")), "status");
        assert_eq!(resolve_sort_column(Some("pushedTime")), "pushed_time");
        assert_eq!(resolve_sort_column(Some("taskNo")), "task_no");
        assert_eq!(resolve_sort_column(Some("unknownField")), "log_id");
        // 每一列都能被属性名 / 列名命中
        for column in PRINT_LOG_COLUMNS {
            assert_eq!(resolve_sort_column(Some(column)), column);
        }
    }

    /// ILIKE 模式：`None` / 空白不加条件，其余 trim 后包 `%`
    /// （Java `isNotBlank` + `like`）。
    #[test]
    fn like_pattern_matches_java_like() {
        assert_eq!(like_pattern(None), None);
        assert_eq!(like_pattern(Some("")), None);
        assert_eq!(like_pattern(Some("   ")), None);
        assert_eq!(like_pattern(Some("SN-1")).as_deref(), Some("%SN-1%"));
        assert_eq!(like_pattern(Some(" SN ")).as_deref(), Some("%SN%"));
    }
}
