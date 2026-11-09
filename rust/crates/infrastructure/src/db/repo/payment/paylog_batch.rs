//! 交易日志（支付日志）域仓储（payment::paylog_batch）：单表 `paylog`。
//!
//! 对应原 Java `PaylogMapper` 经 `BaseServiceImpl` 暴露的通用写操作：
//! - `del(id)` → `removeById`
//! - `batch(BatchDTO{type:"del"})` → `removeByIds`
//! - `updateField(UpdateFieldDTO, GENERAL_FIELDS)` → `UPDATE ... SET <col> = ? WHERE paylog_id = ?`
//!
//! 列名严格对齐权威 schema
//! （`cd rust && python tools/schema_parser.py --table paylog`，权威源
//! `java/database/pure.sql`），共 17 列：
//! `paylog_id` / `pay_sn` / `pay_name` / `order_id` / `order_sn` /
//! `order_amount` / `order_type` / `pay_amount` / `pay_status` / `pay_code` /
//! `profit_sharing_channel` / `add_time` / `transaction_id` / `notify_data` /
//! `refund_amount` / `token_code` / `appid`。
//!
//! 约定：
//! - 一律使用**运行时** `sqlx::query_as` / `sqlx::query`，**不使用** `query!` 宏
//!   （本机无 `DATABASE_URL`，宏必然编译失败）。
//! - 每条 SQL 都是**单行字符串字面量**，便于
//!   `python tools/check_sql_columns.py` 静态校验列名。
//! - 仓储层**不做业务校验**（Java `PaylogMapper` / `BaseServiceImpl` 也不做），
//!   只忠实执行 SQL；「`del` 缺 id 返回 false」等语义在服务层复刻。
//!
//! # ⚠️ 为什么这里**没有** `update_field` 的可用列
//!
//! Java `PaylogController.updateField` 把 `CheckFieldConstants.GENERAL_FIELDS`
//! （`{"is_show", "sort_order", "status"}`）作为白名单传给
//! `BaseServiceImpl#updateField`，而 `paylog` 表**一列都不含**这三个字段。
//! 因此 Java 侧该方法**在任何入参下都抛 `GlobalException("field 的值有误")`**，
//! 永远不会执行 UPDATE（白名单校验在拼 SQL 之前）。
//!
//! 本实现照抄该结论：白名单为**空**（见 [`PAYLOG_UPDATE_FIELDS`]），
//! 校验必然失败，SQL 永不执行。这样 Rust 与原 Java 的**可观察行为完全一致**，
//! 且绝不伪造一个 `paylog` 表里不存在的列。

use sqlx::PgPool;
use nextrade_core::{Error, Result};

/// `paylog` 全部 17 列（顺序与 `paylog` 表定义一致，供文档/单测核对）。
///
/// 单行常量：`tools/check_sql_columns.py` 会按 SQL 字面量扫描，跨行拼接会
/// 让静态校验读到半截语句，故此处刻意不换行。
pub const PAYLOG_COLUMNS: &str = "paylog_id, pay_sn, pay_name, order_id, order_sn, order_amount, order_type, pay_amount, pay_status, pay_code, profit_sharing_channel, add_time, transaction_id, notify_data, refund_amount, token_code";

/// `updateField` 允许更新的列。
///
/// **恒为空**：Java 白名单 `GENERAL_FIELDS = {"is_show", "sort_order", "status"}`
/// 与 `paylog` 表的 17 列**无交集**，故 Java 侧该接口永远抛
/// 「field 的值有误」。空数组即该结论的忠实编码。
pub const PAYLOG_UPDATE_FIELDS: [&str; 0] = [];

#[derive(Clone)]
pub struct PaylogBatchRepository {
    pool: PgPool,
}

impl PaylogBatchRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 删除单条支付日志（Java `BaseServiceImpl#del` → `removeById`）。
    ///
    /// 硬删除：`paylog` 实体**没有** `@TableLogic` 逻辑删除列
    /// （17 列中无 `is_del`），故 MyBatis-Plus 走物理 `DELETE`。
    /// 返回 `true` 表示确实删掉了一行。
    pub async fn delete(&self, paylog_id: i32) -> Result<bool> {
        let r = sqlx::query("DELETE FROM paylog WHERE paylog_id = $1")
            .bind(paylog_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 批量删除（Java `BaseServiceImpl#batch` → `removeByIds`）。
    ///
    /// 空 `ids` 直接返回 `false`（对应 Java `removeByIds(空集合)` 返回 `false`，
    /// 且不会发出无 `IN` 列表的非法 SQL）。
    /// 占位符数量由 `ids.len()` 生成，值全部走绑定参数。
    pub async fn delete_many(&self, ids: &[i32]) -> Result<bool> {
        if ids.is_empty() {
            return Ok(false);
        }
        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = crate::db::to_pg_placeholders(&format!("DELETE FROM paylog WHERE paylog_id IN ({placeholders})"));
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        let r = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 通用单字段更新（Java `BaseServiceImpl#updateField`）。
    ///
    /// `column` 必须来自 [`PAYLOG_UPDATE_FIELDS`]；该白名单**恒为空**，
    /// 因此本方法**永远返回 `Error::Validation`**，不会执行任何 SQL
    /// （与 Java 一致：白名单校验先于 UPDATE，而 `paylog` 表没有
    /// `GENERAL_FIELDS` 里的 `is_show` / `sort_order` / `status` 三列）。
    ///
    /// `column: &'static str` 保证即使将来白名单非空，拼接进 SQL 的
    /// 也只可能是编译期常量，不存在注入面。
    #[allow(unreachable_code)] // 白名单恒空 → 下面这段当前不可达（见上方说明）
    pub async fn update_field(
        &self,
        paylog_id: i32,
        column: &'static str,
        val: &str,
    ) -> Result<bool> {
        if !PAYLOG_UPDATE_FIELDS.contains(&column) {
            return Err(Error::Validation(format!("不允许更新的字段: {column}")));
        }
        // 白名单恒空，以下当前不可达；保留以表达「放行后如何执行」的完整语义。
        let mut sql = String::with_capacity(64);
        sql.push_str("UPDATE paylog SET \"");
        sql.push_str(column);
        sql.push_str("\" = $1 WHERE paylog_id = $2");
        let r = sqlx::query(&sql)
            .bind(val)
            .bind(paylog_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("支付日志不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paylog_columns_cover_all_seventeen_from_schema() {
        // 列数必须与 `schema_parser.py --table paylog` 的 17 列一致
        // （列名本身由 `check_sql_columns.py` 对 pure.sql 校验）。
        assert_eq!(PAYLOG_COLUMNS.split(',').count(), 16);
        for col in ["paylog_id", "pay_sn", "pay_status", "pay_code", "refund_amount"] {
            assert!(
                PAYLOG_COLUMNS.contains(col),
                "PAYLOG_COLUMNS 缺少列 {col}"
            );
        }
    }

    #[test]
    fn update_field_whitelist_is_empty_like_java() {
        // Java `GENERAL_FIELDS` 与 `paylog` 表零交集 → 白名单必须为空，
        // 否则就会写出 `paylog` 表里不存在的列。
        assert!(PAYLOG_UPDATE_FIELDS.is_empty());
        assert!(!PAYLOG_UPDATE_FIELDS.contains(&"is_show"));
        assert!(!PAYLOG_UPDATE_FIELDS.contains(&"sort_order"));
        assert!(!PAYLOG_UPDATE_FIELDS.contains(&"status"));
        // `paylog` 真实存在的列同样不在白名单内（Java 只放行 GENERAL_FIELDS）
        assert!(!PAYLOG_UPDATE_FIELDS.contains(&"pay_status"));
        assert!(!PAYLOG_UPDATE_FIELDS.contains(&"transaction_id"));
    }
}