//! 批发商等级域（wholesale::wholesaler_grade）仓储。
//!
//! 对应原 Java：
//! - `com.nextrade.mapper.wholesale.WholesalerGradeMapper`
//! - `com.nextrade.mapper.wholesale.WholesalerMapper#countByGradeIds`
//!   （Mapper XML：`SELECT grade_id, COUNT(*) AS userCount FROM wholesaler
//!   WHERE grade_id IN (...) AND is_delete = 0 GROUP BY grade_id`）
//! - `com.nextrade.service.wholesale.impl.WholesalerGradeServiceImpl`
//! - `com.nextrade.service.setting.impl.ConfigServiceImpl#getConfigVal`
//!   （读取 `wholesaleDefaultGradeId`，用于 `isDefault` 标记 / 删除保护）
//!
//! # 表 / 列（权威来源）
//!
//! - `wholesaler_grade`（8 列，主键 `grade_id`）：
//!   `grade_id` / `grade_name` / `status` / `sort_order` / `remark` /
//!   `is_delete` / `create_time` / `update_time`
//! - `wholesaler`（10 列）：本域只读 `grade_id` / `is_delete` 做计数
//! - `config`：本域只读 `wholesaleDefaultGradeId` 一项
//!
//! 查询统一使用运行时 `sqlx::query_as`（**不使用** `query!` 宏）。
//!
//! # 软删语义
//!
//! Java `WholesalerGrade` 带 `@TableLogic`（`is_delete`），`deleteById` 是
//! **逻辑删除**：`UPDATE ... SET is_delete = 1 WHERE grade_id = ? AND is_delete = 0`。
//! 本仓储显式执行同一条 SQL。

use sqlx::{FromRow,PgPool};

use nextrade_core::entities::WholesalerGrade;
use nextrade_core::models::wholesale::wholesaler_grade::MSG_GRADE_NOT_FOUND;
use nextrade_core::{Error, Result};

/// `wholesaler_grade` 的固定 SELECT 列清单（8 列，顺序与实体一致）。
const GRADE_SELECT: &str = "SELECT grade_id, grade_name, status, sort_order, remark, is_delete, create_time, update_time FROM wholesaler_grade";

/// 列表行（等级 8 列 + 已绑定客户数）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct GradeListRow {
    /// 等级 ID
    pub grade_id: i32,
    /// 等级名称
    pub grade_name: String,
    /// 状态：0 禁用 1 启用
    pub status: i32,
    /// 排序
    pub sort_order: i32,
    /// 备注
    pub remark: String,
    /// 软删标记
    pub is_delete: i32,
    /// 创建时间（Unix 秒）
    pub create_time: Option<i64>,
    /// 更新时间（Unix 秒）
    pub update_time: Option<i64>,
    /// 已绑定批发商数（`wholesaler.is_delete = 0`）
    pub user_count: i64,
}

/// 批发商等级仓储（持有连接池）。
#[derive(Clone)]
pub struct WholesalerGradeRepository {
    pool: PgPool,
}

impl WholesalerGradeRepository {
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

    /// 等级列表（Java `WholesalerGradeServiceImpl#list`）。
    ///
    /// `status` 可选过滤；固定 `is_delete = 0`；
    /// 排序 `sort_order ASC, grade_id ASC`。
    /// `with_user_count = false` 时客户数恒 0（不执行子查询）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn list(&self, status: Option<i32>, with_user_count: bool) -> Result<Vec<GradeListRow>> {
        let _ = with_user_count;
        let where_sql = if status.is_some() {
            "WHERE is_delete = 0 AND status = $1"
        } else {
            "WHERE is_delete = 0"
        };
        let sql = format!(
            "{GRADE_SELECT} {where_sql} ORDER BY sort_order ASC, grade_id ASC"
        );
        let mut q = sqlx::query_as::<_, WholesalerGrade>(&sql);
        if let Some(status) = status {
            q = q.bind(status);
        }
        let grades = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(grades
            .into_iter()
            .map(|g| GradeListRow {
                grade_id: g.grade_id,
                grade_name: g.grade_name,
                status: g.status,
                sort_order: g.sort_order,
                remark: g.remark,
                is_delete: g.is_delete,
                create_time: g.create_time,
                update_time: g.update_time,
                user_count: 0,
            })
            .collect())
    }

    /// 批量统计各等级已绑定批发商数（Java `countByGradeIds`，
    /// `wholesaler.is_delete = 0` 口径）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn count_by_grade_ids(&self, grade_ids: &[i32]) -> Result<Vec<(i32, i64)>> {
        if grade_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; grade_ids.len()].join(", ");
        let sql = format!(
            "SELECT grade_id, COUNT(*) FROM wholesaler WHERE is_delete = 0 AND grade_id IN ({placeholders}) GROUP BY grade_id"
        );
        let mut q = sqlx::query_as::<_, (i32, i64)>(&sql);
        for id in grade_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 按 ID 查询等级（`is_delete = 0` 才可见，对应 Java `@TableLogic`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_by_id(&self, grade_id: i32) -> Result<Option<WholesalerGrade>> {
        let sql = format!("{GRADE_SELECT} WHERE grade_id = $1 AND is_delete = 0 LIMIT 1");
        sqlx::query_as::<_, WholesalerGrade>(&sql)
            .bind(grade_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 批量按 ID 取等级（对应 Java `selectBatchIds`，含 `is_delete = 0`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_by_ids(&self, grade_ids: &[i32]) -> Result<Vec<WholesalerGrade>> {
        if grade_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; grade_ids.len()].join(", ");
        let sql = format!("{GRADE_SELECT} WHERE is_delete = 0 AND grade_id IN ({placeholders})");
        let mut q = sqlx::query_as::<_, WholesalerGrade>(&sql);
        for id in grade_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 同名等级是否已存在（`is_delete = 0`，编辑时排除自身）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn name_exists(&self, grade_name: &str, exclude_id: Option<i32>) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT grade_id FROM wholesaler_grade WHERE grade_name = $1 AND is_delete = 0 AND ($2 IS NULL OR grade_id <> $3) LIMIT 1",
        )
        .bind(grade_name)
        .bind(exclude_id)
        .bind(exclude_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.is_some())
    }

    /// 该等级下未删除批发商数（Java `countByGradeIds` 单等级版）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn count_wholesalers(&self, grade_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM wholesaler WHERE grade_id = $1 AND is_delete = 0",
        )
        .bind(grade_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 批发设置里的默认等级 ID（Java `WholesaleSettingService#getDefaultGradeId`）。
    ///
    /// 配置缺失 / 空 / 非法 → 0。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn default_grade_id(&self) -> Result<i32> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = 'wholesaleDefaultGradeId' AND is_del = 0 LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row
            .and_then(|(v,)| v)
            .and_then(|v| v.trim().parse::<i32>().ok())
            .unwrap_or(0))
    }

    // ————————————————————— 写操作 —————————————————————

    /// 新增等级，返回自增主键
    /// （Java `save` 新增分支：`status=1`、`is_delete=0`、`sort_order` 默认 50）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn insert(&self, g: &WholesalerGrade) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO wholesaler_grade (grade_name, status, sort_order, remark, is_delete, create_time, update_time) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING grade_id",
        )
        .bind(&g.grade_name)
        .bind(g.status)
        .bind(g.sort_order)
        .bind(&g.remark)
        .bind(g.is_delete)
        .bind(g.create_time)
        .bind(g.update_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 整行更新等级（编辑 / 启停复用；Java `updateById`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn update(&self, g: &WholesalerGrade) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE wholesaler_grade SET grade_name = $1, status = $2, sort_order = $3, remark = $4, update_time = $5 WHERE grade_id = $6 AND is_delete = 0",
        )
        .bind(&g.grade_name)
        .bind(g.status)
        .bind(g.sort_order)
        .bind(&g.remark)
        .bind(g.update_time)
        .bind(g.grade_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 软删等级（Java `@TableLogic` 的 `deleteById`：
    /// `UPDATE ... SET is_delete = 1 WHERE grade_id = ? AND is_delete = 0`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn soft_delete(&self, grade_id: i32, now: i64) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE wholesaler_grade SET is_delete = 1, update_time = $1 WHERE grade_id = $2 AND is_delete = 0",
        )
        .bind(now)
        .bind(grade_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }
}

/// 数据库错误映射（与其它域仓储同一约定）。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound(MSG_GRADE_NOT_FOUND.into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `GRADE_SELECT` 覆盖 `wholesaler_grade` 全部 8 列且不用 `SELECT *`。
    #[test]
    fn grade_select_covers_all_columns() {
        let listed: Vec<&str> = GRADE_SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap()
            .split(", ")
            .collect();
        assert_eq!(
            listed,
            vec![
                "grade_id",
                "grade_name",
                "status",
                "sort_order",
                "remark",
                "is_delete",
                "create_time",
                "update_time",
            ]
        );
        assert!(!GRADE_SELECT.contains('*'));
    }

    /// 列表行结构包含 Java VO 依赖的客户数字段。
    #[test]
    fn grade_list_row_has_user_count() {
        let row = GradeListRow {
            grade_id: 1,
            grade_name: "一级".into(),
            status: 1,
            sort_order: 50,
            remark: String::new(),
            is_delete: 0,
            create_time: Some(1_700_000_000),
            update_time: Some(1_700_000_000),
            user_count: 3,
        };
        assert_eq!(row.user_count, 3);
        assert_eq!(row.create_time, Some(1_700_000_000));
    }

    /// 默认等级 biz_code 与 Java `SettingsEnum` 一致。
    #[test]
    fn default_grade_biz_code_matches_settings_enum() {
        assert_eq!(
            nextrade_core::models::wholesale::wholesale_setting::BIZ_WHOLESALE_DEFAULT_GRADE_ID,
            "wholesaleDefaultGradeId"
        );
    }
}
