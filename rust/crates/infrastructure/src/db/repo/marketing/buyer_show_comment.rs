//! 买家秀评论管理域（marketing::buyer_show_comment）仓储：`buyer_show_comment` / `user`。
//!
//! 对应原 Java：
//! - `BuyerShowCommentServiceImpl#list`（`BaseServiceImpl#page` + `buildSortOrder`）
//! - `BuyerShowCommentServiceImpl#audit` / `#batch` / `#showOrHide` / `#isTop`
//!   （均为 MyBatis-Plus `lambdaUpdate()`）
//!
//! 列名严格对齐 `java/database/pure.sql`
//! （`python tools/schema_parser.py --table buyer_show_comment` 可复核）：
//! - `buyer_show_comment`：`buyer_show_comment_id`（主键）/ `parent_id` /
//!   `buyer_show_id` / `status` / `is_top` / `is_show` / `user_id` / `user_name` /
//!   `user_photo` / `content` / `add_time`（`bigint unsigned`，**秒级时间戳**）/
//!   `comment_pic`
//! - `user`：`user_id` / `username` / `nickname`（`name` 是 Java 侧的派生 getter，
//!   无对应列）
//!
//! 查询统一使用运行时 `sqlx::query_as`（**不使用 `query!` 宏**）。
//!
//! ## 排序字段白名单
//!
//! Java `BaseServiceImpl#buildSortOrder` 把属性名映射为真实列名，找不到时**回退主键**
//! （防 SQL 注入）。这里用白名单实现同一语义：`sort_field` 只接受下表列出的
//! 驼峰属性名或真实列名，其余一律回退 `buyer_show_comment_id`。

use std::collections::HashMap;

use sqlx::PgPool;
use nextrade_core::entities::BuyerShowComment;
use nextrade_core::models::marketing::buyer_show_comment::BuyerShowCommentListQuery;
use nextrade_core::{Error, Result};

/// 待绑定参数（按 `conds` 顺序 1:1 对应）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bind {
    I32(i32),
    I64(i64),
}

/// 把 `Bind` 列表按顺序绑定到查询上（宏展开以便同时适配不同返回行类型）。
///
/// 内部用 `iter()` 借用，故 `binds` 可被多条语句（COUNT / SELECT）复用。
macro_rules! bind_all {
    ($q:expr, $binds:expr) => {{
        let mut q = $q;
        for b in $binds.iter() {
            q = match b {
                Bind::I32(v) => q.bind(*v),
                Bind::I64(v) => q.bind(*v),
            };
        }
        q
    }};
}

/// `buyer_show_comment` 表仓储（买家秀评论管理域独占）。
#[derive(Clone)]
pub struct BuyerShowCommentRepository {
    pool: PgPool,
}

impl BuyerShowCommentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ————————————————————— 列表 —————————————————————

    /// 买家秀评论分页（对应 Java `BuyerShowCommentServiceImpl#list` 的分页查询）。
    ///
    /// 返回 `(总数, 当前页记录)`；排序规则见 [`order_by_clause`]，
    /// 过滤规则见 [`build_filter`]（与 Java `LambdaQueryWrapper` 逐条对照）。
    pub async fn paginate_comments(
        &self,
        q: &BuyerShowCommentListQuery,
    ) -> Result<(u64, Vec<BuyerShowComment>)> {
        let (where_sql, binds) = build_filter(q)?;
        let limit = q.page_size();
        let offset = (q.page_no() - 1) * limit;

        let total_sql = format!("SELECT COUNT(*) FROM buyer_show_comment{where_sql}");
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&total_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows_sql = format!(
            "SELECT * FROM buyer_show_comment{where_sql}{} LIMIT ? OFFSET ?",
            order_by_clause(q)
        );
        let rows = bind_all!(sqlx::query_as::<_, BuyerShowComment>(&rows_sql), binds)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 批量取用户 `user_id -> 展示名`（对应 Java `User#getName()`：
    /// `StrUtil.isNotBlank(nickname) ? nickname : username`）。
    ///
    /// Java 先整表实体查回再逐个调 `getName()`；这里只取两列，
    /// 展示名在 SQL 侧用 `IFNULL` 兜底，语义一致。
    pub async fn find_user_names(&self, user_ids: &[i32]) -> Result<HashMap<i32, String>> {
        if user_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let sql = format!(
            "SELECT user_id, COALESCE(nickname, username) FROM user WHERE user_id IN ({})",
            placeholders(user_ids.len())
        );
        let mut q = sqlx::query_as::<_, (i32, String)>(&sql);
        for id in user_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().collect())
    }

    // ————————————————————— 审核 / 批量 / 公开隐藏 / 置顶 —————————————————————

    /// 审核（对应 Java `BuyerShowCommentServiceImpl#audit`）。
    ///
    /// Java：
    /// ```text
    /// lambdaUpdate().eq(buyerShowCommentId, param.getBuyerShowCommentId())
    ///               .set(status, param.getStatus())
    ///               .set(isShow, param.getStatus() == 1 ? YES : NO)
    /// ```
    /// 返回受影响行数（Java 未校验结果，本方法把行数交给服务层决定是否提示）。
    pub async fn audit(&self, buyer_show_comment_id: i32, status: i32) -> Result<u64> {
        let is_show = if status == 1 { 1 } else { 0 };
        let affected = sqlx::query(
            "UPDATE buyer_show_comment SET status = $1, is_show = $2 WHERE buyer_show_comment_id = $3",
        )
        .bind(status)
        .bind(is_show)
        .bind(buyer_show_comment_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected)
    }

    /// 批量审核（对应 Java `BuyerShowCommentServiceImpl#batch` 的 `audit` 分支）：
    /// `status` 与 `is_show` 同时按 `value` 写入。
    ///
    /// `ids` 为空时直接返回 0（调用方已按 Java `@NotEmpty` 拦截）。
    pub async fn batch_audit(&self, ids: &[i32], value: i32) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let is_show = if value == 1 { 1 } else { 0 };
        let sql = format!(
            "UPDATE buyer_show_comment SET status = $1, is_show = $2 WHERE buyer_show_comment_id IN ({})",
            placeholders(ids.len())
        );
        let mut q = sqlx::query(&sql).bind(value).bind(is_show);
        for id in ids {
            q = q.bind(*id);
        }
        let affected = q.execute(&self.pool).await.map_err(map_err)?.rows_affected();
        Ok(affected)
    }

    /// 公开 / 隐藏（对应 Java `BuyerShowCommentServiceImpl#showOrHide`）：
    /// 仅写 `is_show`，不校验取值（Java 同样原样落库）。
    pub async fn set_show(&self, buyer_show_comment_id: i32, is_show: i32) -> Result<u64> {
        let affected =
            sqlx::query("UPDATE buyer_show_comment SET is_show = $1 WHERE buyer_show_comment_id = $2")
                .bind(is_show)
                .bind(buyer_show_comment_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?
                .rows_affected();
        Ok(affected)
    }

    /// 置顶 / 取消置顶（对应 Java `BuyerShowCommentServiceImpl#isTop`）：仅写 `is_top`。
    pub async fn set_top(&self, buyer_show_comment_id: i32, is_top: i32) -> Result<u64> {
        let affected =
            sqlx::query("UPDATE buyer_show_comment SET is_top = $1 WHERE buyer_show_comment_id = $2")
                .bind(is_top)
                .bind(buyer_show_comment_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?
                .rows_affected();
        Ok(affected)
    }
}

/// `?,?,?` 占位符（长度由调用方控制，恒为合法 SQL 片段）。
fn placeholders(n: usize) -> String {
    std::iter::repeat_n("?", n).collect::<Vec<_>>().join(",")
}

/// 构造 WHERE 子句（含前导空格）与绑定参数列表。
///
/// 逐条对照 Java `BuyerShowCommentServiceImpl#list`：
/// ```text
/// queryWrapper.eq(buyerShowId != null, buyerShowId)
///             .eq(status != null && status == -1, status, 0)
///             .ne(status != null && status > -1, status, 0)
///             .eq(status != null && status > -1, isShow, status);
/// if (isNotBlank(startTime) && isNotBlank(endTime)) {
///     queryWrapper.between(addTime, toTimestampYmd(startTime), toTimestampYmd(endTime));
/// }
/// ```
/// 因此：
/// - `status == -1` → `status = 0`（待审核）；
/// - `status > -1` → `status <> 0 AND is_show = status`；
/// - `status < -1`（如 -2）→ 两个条件都不生效，等价于**不过滤**（Java 亦然）；
/// - 时间区间只在**两个字段都非空**时生效，且 `between` 为**闭区间且按当日 00:00:00**
///   （Java `TigUtils.toTimestampYmd` 解析到当天零点，不补 23:59:59）。
fn build_filter(q: &BuyerShowCommentListQuery) -> Result<(String, Vec<Bind>)> {
    let mut conds: Vec<String> = Vec::new();
    let mut binds: Vec<Bind> = Vec::new();

    if let Some(buyer_show_id) = q.buyer_show_id {
        conds.push("buyer_show_id = ?".into());
        binds.push(Bind::I32(buyer_show_id));
    }

    match q.status {
        Some(-1) => {
            conds.push("status = ?".into());
            binds.push(Bind::I32(0));
        }
        Some(s) if s > -1 => {
            conds.push("status <> 0 AND is_show = ?".into());
            binds.push(Bind::I32(s));
        }
        // `status == null` 或 `status < -1`：无状态条件（Java 短路后两者都不生效）
        _ => {}
    }

    if let (Some(start), Some(end)) = (non_empty(&q.start_time), non_empty(&q.end_time)) {
        conds.push("add_time BETWEEN ? AND ?".into());
        binds.push(Bind::I64(day_start_seconds(start)?));
        binds.push(Bind::I64(day_start_seconds(end)?));
    }

    if conds.is_empty() {
        return Ok((String::new(), binds));
    }
    Ok((format!(" WHERE {}", conds.join(" AND ")), binds))
}

/// ORDER BY 子句（含前导空格）。
///
/// Java `BaseServiceImpl#buildSortOrder(page, sortField, sortOrder)`：
/// - `sortField` 为空或等于主键属性名 → 用主键列；
/// - `asc`（忽略大小写）升序，`desc` 或空 → 降序；
/// - 属性名经 `getColumnByPropertyName` 映射为真实列名，找不到时回退主键。
fn order_by_clause(q: &BuyerShowCommentListQuery) -> String {
    let col = q
        .sort_field
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(sort_column)
        .unwrap_or(PRIMARY_KEY);
    let dir = if q
        .sort_order
        .as_deref()
        .is_some_and(|o| o.trim().eq_ignore_ascii_case("asc"))
    {
        "ASC"
    } else {
        "DESC"
    };
    format!(" ORDER BY {col} {dir}")
}

/// 主键列名（Java `getKeyColumn()` 的返回值）。
const PRIMARY_KEY: &str = "buyer_show_comment_id";

/// 排序字段白名单：Java 属性名（驼峰）或真实列名 → 列名。
fn sort_column(field: &str) -> Option<&'static str> {
    match field {
        "buyerShowCommentId" | "buyer_show_comment_id" => Some("buyer_show_comment_id"),
        "parentId" | "parent_id" => Some("parent_id"),
        "buyerShowId" | "buyer_show_id" => Some("buyer_show_id"),
        "status" => Some("status"),
        "isTop" | "is_top" => Some("is_top"),
        "isShow" | "is_show" => Some("is_show"),
        "userId" | "user_id" => Some("user_id"),
        "userName" | "user_name" => Some("user_name"),
        "addTime" | "add_time" => Some("add_time"),
        _ => None,
    }
}

/// 取非空的去空白字符串（Java `StrUtil.isNotBlank`）。
fn non_empty(v: &Option<String>) -> Option<&str> {
    v.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

/// `yyyy-MM-dd` → 当日 00:00:00 秒级时间戳。
///
/// 对齐 Java `TigUtils.toTimestampYmd(dateStr)`：`DateUtil.parse(dateStr,
/// "yyyy-MM-dd").getTime() / 1000`（服务器本地时区，当天零点）。
fn day_start_seconds(date: &str) -> Result<i64> {
    let mut it = date.split('-');
    let y = it.next().and_then(|v| v.trim().parse::<i32>().ok());
    let m = it.next().and_then(|v| v.trim().parse::<u32>().ok());
    let d = it.next().and_then(|v| v.trim().parse::<u32>().ok());
    let (Some(y), Some(m), Some(d)) = (y, m, d) else {
        return Err(Error::Validation(format!("日期格式应为 yyyy-MM-dd: {date}")));
    };
    let naive = chrono::NaiveDate::from_ymd_opt(y, m, d)
        .ok_or_else(|| Error::Validation(format!("日期不合法: {date}")))?
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| Error::Validation(format!("日期不合法: {date}")))?;
    Ok(local_seconds(naive))
}

/// 本地时区的 naive 时间 → 秒级时间戳。
fn local_seconds(naive: chrono::NaiveDateTime) -> i64 {
    use chrono::TimeZone;
    match chrono::Local.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => dt.timestamp(),
        chrono::LocalResult::Ambiguous(dt, _) => dt.timestamp(),
        chrono::LocalResult::None => naive.and_utc().timestamp(),
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("买家秀评论不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(json: &str) -> BuyerShowCommentListQuery {
        serde_json::from_str(json).unwrap()
    }

    // ————————————————— 过滤条件 —————————————————

    #[test]
    fn filter_empty_query_has_no_condition() {
        let (sql, binds) = build_filter(&BuyerShowCommentListQuery::default()).unwrap();
        assert_eq!(sql, "");
        assert!(binds.is_empty());
    }

    #[test]
    fn filter_pending_status_maps_to_status_zero() {
        // Java：status == -1 → status = 0（待审核）；此分支**不**看 is_show
        let (sql, binds) = build_filter(&query(r#"{"status":-1}"#)).unwrap();
        assert_eq!(sql, " WHERE status = ?");
        assert_eq!(binds, vec![Bind::I32(0)]);
    }

    #[test]
    fn filter_show_status_maps_to_is_show() {
        for s in [0, 1] {
            let (sql, binds) = build_filter(&query(&format!(r#"{{"status":{s}}}"#))).unwrap();
            assert_eq!(sql, " WHERE status <> 0 AND is_show = ?");
            assert_eq!(binds, vec![Bind::I32(s)]);
        }
    }

    #[test]
    fn filter_status_below_minus_one_is_ignored() {
        // Java：status < -1 时 eq 与 ne 两条都不生效 → 无状态过滤
        let (sql, binds) = build_filter(&query(r#"{"status":-2}"#)).unwrap();
        assert_eq!(sql, "");
        assert!(binds.is_empty());
    }

    #[test]
    fn filter_combines_buyer_show_and_status_in_java_order() {
        let (sql, binds) = build_filter(&query(r#"{"buyerShowId":7,"status":1}"#)).unwrap();
        assert_eq!(sql, " WHERE buyer_show_id = ? AND status <> 0 AND is_show = ?");
        assert_eq!(binds, vec![Bind::I32(7), Bind::I32(1)]);
    }

    #[test]
    fn filter_time_range_requires_both_bounds() {
        // 只有 startTime：Java `isNotBlank(endTime)` 不成立 → 不加条件
        let (sql, binds) = build_filter(&query(r#"{"startTime":"2026-01-01"}"#)).unwrap();
        assert_eq!(sql, "");
        assert!(binds.is_empty());

        // 只有 endTime：同理
        let (sql, _) = build_filter(&query(r#"{"endTime":"2026-01-31"}"#)).unwrap();
        assert_eq!(sql, "");

        // 空白串按 Java `isNotBlank` 视为空
        let (sql, _) =
            build_filter(&query(r#"{"startTime":"  ","endTime":"2026-01-31"}"#)).unwrap();
        assert_eq!(sql, "");

        // 两端都有 → 闭区间，两边界都是当日 00:00:00
        let (sql, binds) =
            build_filter(&query(r#"{"startTime":"2026-01-01","endTime":"2026-01-31"}"#)).unwrap();
        assert_eq!(sql, " WHERE add_time BETWEEN ? AND ?");
        assert_eq!(binds.len(), 2);
        let (Bind::I64(s), Bind::I64(e)) = (&binds[0], &binds[1]) else {
            panic!("时间条件必须绑定 i64: {binds:?}");
        };
        assert_eq!(e - s, 30 * 86_400, "1月1日到1月31日应为 30 天整（不含末日 23:59:59）");
    }

    #[test]
    fn filter_rejects_bad_date() {
        assert!(build_filter(&query(r#"{"startTime":"bad","endTime":"2026-01-31"}"#)).is_err());
        assert!(build_filter(&query(r#"{"startTime":"2026-13-01","endTime":"2026-01-31"}"#)).is_err());
    }

    #[test]
    fn filter_placeholders_match_bind_count() {
        let (sql, binds) = build_filter(&query(
            r#"{"buyerShowId":7,"status":0,"startTime":"2026-01-01","endTime":"2026-01-31"}"#,
        ))
        .unwrap();
        assert_eq!(sql.matches('?').count(), binds.len());
    }

    // ————————————————— 排序 —————————————————

    #[test]
    fn order_by_defaults_to_primary_key_desc() {
        let q = BuyerShowCommentListQuery::default();
        assert_eq!(order_by_clause(&q), " ORDER BY buyer_show_comment_id DESC");
    }

    #[test]
    fn order_by_honours_field_and_direction() {
        let q = query(r#"{"sortField":"addTime","sortOrder":"asc"}"#);
        assert_eq!(order_by_clause(&q), " ORDER BY add_time ASC");

        let q = query(r#"{"sortField":"isTop","sortOrder":"DESC"}"#);
        assert_eq!(order_by_clause(&q), " ORDER BY is_top DESC");

        // 未知方向按 Java `buildSortOrder` 落到降序
        let q = query(r#"{"sortField":"status","sortOrder":"sideways"}"#);
        assert_eq!(order_by_clause(&q), " ORDER BY status DESC");
    }

    #[test]
    fn order_by_falls_back_to_primary_key_for_injection() {
        let q = query(r#"{"sortField":"1;DROP TABLE buyer_show_comment","sortOrder":"asc"}"#);
        assert_eq!(order_by_clause(&q), " ORDER BY buyer_show_comment_id ASC");

        // 主键属性名 → 主键列（Java 有同样映射）
        let q = query(r#"{"sortField":"buyerShowCommentId","sortOrder":"asc"}"#);
        assert_eq!(order_by_clause(&q), " ORDER BY buyer_show_comment_id ASC");
    }

    // ————————————————— 工具函数 —————————————————

    #[test]
    fn placeholders_count() {
        assert_eq!(placeholders(1), "?");
        assert_eq!(placeholders(3), "?,?,?");
    }

    #[test]
    fn day_start_seconds_is_local_midnight() {
        use chrono::TimeZone;
        let s = day_start_seconds("2026-03-05").unwrap();
        let expected = chrono::Local
            .with_ymd_and_hms(2026, 3, 5, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp();
        assert_eq!(s, expected);
    }
}
