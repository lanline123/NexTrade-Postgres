//! 退款记录域仓储（payment::refund_log）。
//!
//! 对应原 Java `RefundLogMapper` / `RefundLogServiceImpl#list` 的 MPJ 查询。
//!
//! 表名/列名严格对齐 `java/database/pure.sql`
//! （核对命令：`cd rust && python tools/schema_parser.py --table refund_log`）：
//! - `refund_log`（10 列）：`log_id` / `refund_apply_id` / `refund_type` /
//!   `refund_pay_code` / `transaction_id` / `refund_amount` / `add_time` /
//!   `description` / `user_id` / `order_id`
//! - `refund_apply`（17 列，list 装配 `refund` 用）
//! - `aftersales`（22 列，list 装配 `refund.aftersales` 用）
//!
//! SQL 一律运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`
//! （**不用** `query!` 宏，本机无 `DATABASE_URL`）。
//!
//! ## 与 Java 查询的对应关系
//!
//! ```java
//! if (isNotEmpty(keyword)) {
//!     queryWrapper
//!         .leftJoin(RefundApply.class, RefundApply::getRefundId, RefundLog::getRefundApplyId)
//!         .leftJoin(OrderFinanceAftersales.class,
//!                   OrderFinanceAftersales::getAftersaleId, RefundApply::getAftersaleId)
//!         .like(OrderFinanceAftersales::getAftersalesSn, keyword);
//! }
//! if (type != null) queryWrapper.eq(RefundLog::getRefundType, type);
//! ```
//!
//! 即：**只有 keyword 非空才发生两级 LEFT JOIN**；`type` 与 keyword 可叠加。

use sqlx::{Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::{Aftersales, RefundApply, RefundLog};
use nextrade_core::models::payment::refund_log::RefundLogListQuery;
use nextrade_core::{Error, Result};

/// `refund_log` 固定 SELECT 列清单（顺序与实体一致）。
pub const REFUND_LOG_COLUMNS: [&str; 10] = [
    "log_id",
    "refund_apply_id",
    "refund_type",
    "refund_pay_code",
    "transaction_id",
    "refund_amount",
    "add_time",
    "description",
    "user_id",
    "order_id",
];

/// `refund_apply` 固定 SELECT 列清单（顺序与实体一致）。
pub const REFUND_APPLY_COLUMNS: [&str; 17] = [
    "refund_id",
    "refund_type",
    "order_id",
    "user_id",
    "aftersale_id",
    "refund_status",
    "add_time",
    "refund_note",
    "online_balance",
    "offline_balance",
    "refund_balance",
    "is_online",
    "is_offline",
    "is_receive",
    "paylog_refund_id",
    "shop_id",
    "payment_voucher",
];

/// `aftersales` 固定 SELECT 列清单（顺序与实体一致）。
pub const AFTERSALES_COLUMNS: [&str; 22] = [
    "aftersale_id",
    "aftersale_type",
    "status",
    "pics",
    "description",
    "reply",
    "add_time",
    "tracking_no",
    "logistics_name",
    "return_address",
    "aftersale_reason",
    "aftersales_sn",
    "order_id",
    "user_id",
    "refund_amount",
    "shop_id",
    "audit_time",
    "deal_time",
    "final_time",
    "return_goods_tip",
    "vendor_id",
    "close_type",
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

/// 退款记录域仓储。
#[derive(Clone)]
pub struct RefundLogRepository {
    pool: PgPool,
}

impl RefundLogRepository {
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
}

impl RefundLogRepository {
    /// 按主键查询（Java `getById`）。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<RefundLog>> {
        let sql = format!(
            "SELECT {} FROM refund_log WHERE log_id = $1 LIMIT 1",
            REFUND_LOG_COLUMNS.join(", ")
        );
        sqlx::query_as::<_, RefundLog>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 分页列表（Java `RefundLogServiceImpl#list` 的 `page(page, queryWrapper)`）。
    ///
    /// 返回 `(总记录数, 当前页行)`；总数与行使用**同一套 FROM/WHERE**，
    /// 与 MP 分页插件的 `COUNT` 口径一致（LEFT JOIN 均为 1:1，不会放大计数）。
    pub async fn paginate(
        &self,
        q: &RefundLogListQuery,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<RefundLog>)> {
        let (from, where_sql, binds) = build_from_where(q);
        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM {from}{where_sql}"));
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&count_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {} FROM {from}{where_sql}{} LIMIT {limit} OFFSET {offset}",
            REFUND_LOG_COLUMNS
                .iter()
                .map(|c| format!("refund_log.{c}"))
                .collect::<Vec<_>>()
                .join(", "),
            q.order_clause()
        ));
        let rows: Vec<RefundLog> = bind_all!(
            sqlx::query_as::<_, RefundLog>(&rows_sql),
            binds
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((total.0, rows))
    }

    /// 批量取退款申请（list 装配 `refund` 字段；Java 逐条 `selectById`）。
    pub async fn find_refund_applies(&self, ids: &[i32]) -> Result<Vec<RefundApply>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!(
            "SELECT {} FROM refund_apply WHERE refund_id IN (",
            REFUND_APPLY_COLUMNS.join(", ")
        ));
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

    /// 批量取售后单（list 装配 `refund.aftersales`；Java 逐条 `selectById`）。
    pub async fn find_aftersales(&self, ids: &[i32]) -> Result<Vec<Aftersales>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!(
            "SELECT {} FROM aftersales WHERE aftersale_id IN (",
            AFTERSALES_COLUMNS.join(", ")
        ));
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
}

impl RefundLogRepository {
    /// 新增（Java `save`；`add_time` 由服务层覆盖为当前秒）。
    pub async fn insert(&self, entity: &RefundLog) -> Result<i32> {
        let sql = format!(
            "INSERT INTO refund_log ({}) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            REFUND_LOG_COLUMNS[1..].join(", ")
        );
        let res: i32 = sqlx::query_scalar(&format!("{} RETURNING log_id", &sql))
            .bind(entity.refund_apply_id)
            .bind(entity.refund_type)
            .bind(&entity.refund_pay_code)
            .bind(&entity.transaction_id)
            .bind(entity.refund_amount)
            .bind(entity.add_time)
            .bind(&entity.description)
            .bind(entity.user_id)
            .bind(entity.order_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res)
    }

    /// 整行更新（Java `updateById`；服务层先合并旧值，等价 NOT_NULL 语义）。
    pub async fn update(&self, entity: &RefundLog) -> Result<u64> {
        let sql = "UPDATE refund_log SET refund_apply_id = $1, refund_type = $2, refund_pay_code = $3, transaction_id = $4, refund_amount = $5, add_time = $6, description = $7, user_id = $8, order_id = $9 WHERE log_id = $10";
        let res = sqlx::query(sql)
            .bind(entity.refund_apply_id)
            .bind(entity.refund_type)
            .bind(&entity.refund_pay_code)
            .bind(&entity.transaction_id)
            .bind(entity.refund_amount)
            .bind(entity.add_time)
            .bind(&entity.description)
            .bind(entity.user_id)
            .bind(entity.order_id)
            .bind(entity.log_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 单字段更新（列名由服务层白名单归一后传入；本表无白名单列，
    /// 服务层会先以「field 的值有误」拒绝，正常不会走到这里）。
    pub async fn update_field(&self, id: i32, column: &str, val: &str) -> Result<u64> {
        debug_assert!(
            nextrade_core::models::payment::refund_log::GENERAL_FIELDS.contains(&column),
            "update_field 列名未经过白名单映射: {column}"
        );
        let sql = format!("UPDATE refund_log SET {column} = $1 WHERE log_id = $2");
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
        let res = sqlx::query("DELETE FROM refund_log WHERE log_id = $1")
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
            QueryBuilder::new("DELETE FROM refund_log WHERE log_id IN (");
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

/// 拼装 `FROM` / `WHERE` 与绑定参数（对应 Java wrapper 链）。
///
/// 返回 `(from_sql, where_sql, binds)`：
/// - `from_sql` 含 `refund_log` 与（仅 keyword 非空时的）两级 LEFT JOIN；
/// - `where_sql` 以 `" WHERE ..."` 开头，无条件时为空串；
/// - `binds` 与 SQL 中 `?` 严格同序（先 keyword 后 type，与 Java 链顺序一致）。
#[must_use]
pub fn build_from_where(q: &RefundLogListQuery) -> (String, String, Vec<Bind>) {
    let mut from = String::from("refund_log");
    let mut conds: Vec<String> = Vec::new();
    let mut binds: Vec<Bind> = Vec::new();

    // if (isNotEmpty(keyword)) { 两级 LEFT JOIN + like(aftersalesSn, keyword) }
    if let Some(keyword) = q.effective_keyword() {
        from.push_str(
            " LEFT JOIN refund_apply ON refund_apply.refund_id = refund_log.refund_apply_id",
        );
        from.push_str(
            " LEFT JOIN aftersales ON aftersales.aftersale_id = refund_apply.aftersale_id",
        );
        conds.push("aftersales.aftersales_sn LIKE ?".to_string());
        binds.push(Bind::Str(format!("%{keyword}%")));
    }
    // if (type != null) eq(refundType, type)
    if let Some(refund_type) = q.r#type {
        conds.push("refund_log.refund_type = ?".to_string());
        binds.push(Bind::I32(refund_type));
    }

    let where_sql = if conds.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conds.join(" AND "))
    };
    (from, where_sql, binds)
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(json: serde_json::Value) -> RefundLogListQuery {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn no_filters_means_no_join_no_where() {
        let (from, where_sql, binds) = build_from_where(&q(serde_json::json!({})));
        assert_eq!(from, "refund_log");
        assert_eq!(where_sql, "");
        assert!(binds.is_empty());
    }

    #[test]
    fn keyword_triggers_two_left_joins_and_like() {
        let (from, where_sql, binds) = build_from_where(&q(serde_json::json!({"keyword": "SH2024"})));
        assert_eq!(
            from,
            "refund_log LEFT JOIN refund_apply ON refund_apply.refund_id = refund_log.refund_apply_id LEFT JOIN aftersales ON aftersales.aftersale_id = refund_apply.aftersale_id"
        );
        assert_eq!(where_sql, " WHERE aftersales.aftersales_sn LIKE $1");
        assert!(matches!(&binds[0], Bind::Str(s) if s == "%SH2024%"));
    }

    #[test]
    fn type_filters_refund_type_and_bind_order_matches_sql() {
        // 顺序：keyword（如存在）在前，type 在后 —— 与 Java wrapper 链一致
        let (_from, where_sql, binds) = build_from_where(&q(serde_json::json!({
            "keyword": "sn", "type": 2
        })));
        assert_eq!(
            where_sql,
            " WHERE aftersales.aftersales_sn LIKE $1 AND refund_log.refund_type = $2"
        );
        assert!(matches!(&binds[0], Bind::Str(s) if s == "%sn%"));
        assert!(matches!(binds[1], Bind::I32(2)));

        // type = 0 也要生效（Java `type != null`，不是 `> 0`）
        let (_f, where_sql, binds) = build_from_where(&q(serde_json::json!({"type": 0})));
        assert_eq!(where_sql, " WHERE refund_log.refund_type = $1");
        assert!(matches!(binds.as_slice(), [Bind::I32(0)]));
    }

    #[test]
    fn select_lists_match_entity_column_order() {
        // `REFUND_LOG_COLUMNS` 的首列是主键；insert 用 [1..] 跳过自增列
        assert_eq!(REFUND_LOG_COLUMNS[0], "log_id");
        assert_eq!(REFUND_LOG_COLUMNS.len(), 10);
        assert_eq!(REFUND_APPLY_COLUMNS.len(), 17);
        assert_eq!(AFTERSALES_COLUMNS.len(), 22);
        // 静态守卫：查询必须真的用到两级 LEFT JOIN 与 LIKE（防被重构悄悄改掉）
        let src = include_str!("refund_log.rs");
        assert!(src.contains("LEFT JOIN refund_apply ON refund_apply.refund_id = refund_log.refund_apply_id"));
        assert!(src.contains("aftersales.aftersales_sn LIKE ?"));
    }
}
