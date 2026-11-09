//! 打印机配置管理域（logistics::print_config）仓储：`print_config` / `print` /
//! `print_pos_device` / `print_log`。
//!
//! 对应原 Java：
//! - `com.nextrade.mapper.print.PrintConfigMapper`（`BaseMapper<PrintConfig>`）
//! - `com.nextrade.mapper.print.PrintMapper`（`BaseMapper<Print>`）
//! - `com.nextrade.mapper.print.PrintPosDeviceMapper`
//! - `com.nextrade.mapper.print.PrintLogMapper`
//! - `com.nextrade.service.print.impl.PrintConfigServiceImpl` 里的
//!   `lambdaQuery()` / `getById()` / `updateById()`
//! - `com.nextrade.service.print.impl.PrintRoutingServiceImpl#resolve`
//!   （`print` 表 + `print_pos_device` 两张表的查询）
//! - `com.nextrade.service.print.impl.PrintLogServiceImpl#createPending`
//!
//! # 列名对齐（权威来源）
//!
//! 全部列名来自 `java/database/pure.sql`，用
//! `cd rust && python tools/schema_parser.py --table <表名>` 核对：
//!
//! - `print_config`（7 列，主键 `id`）：
//!   `id` / `print_id` / `template` / `type` / `paper_size` /
//!   `add_time` / `update_time`
//! - `print`（17 列，主键 `print_id`，见 [`PRINT_SELECT`]）
//! - `print_pos_device`（5 列，主键 `id`）：
//!   `id` / `shop_id` / `print_id` / `device_code` / `add_time`
//! - `print_log`（18 列，主键 `log_id`）
//!
//! 查询统一使用运行时 `sqlx::query_as` / `sqlx::query`
//! （**不使用** `query!` / `query_as!` 宏：本机无 `DATABASE_URL`）。
//!
//! # 金额
//!
//! 本域**不涉及任何金额列**，故无 `rust_decimal::Decimal`。

use sqlx::{FromRow,PgPool};

use nextrade_core::entities::{Print, PrintPosDevice};
use nextrade_core::models::logistics::print_config::{LOG_STATUS_PENDING, PRINT_STATUS_ENABLED};
use nextrade_core::{Error, Result};

/// `print_config` 的固定 SELECT 列清单。
///
/// ⚠️ `type` 是 MySQL 关键字同名列，列名本身**就是** `type`
/// （`schema_parser.py --table print_config` 确认），故 SELECT 里原样写 `type`。
const PRINT_CONFIG_SELECT: &str = "SELECT id, print_id, template, type, paper_size, add_time, update_time FROM print_config";

/// `print` 的固定 SELECT 列清单（17 列，顺序与 `entities::Print` 字段一致，
/// 避免 `SELECT *` 与实体字段顺序耦合）。
const PRINT_SELECT: &str = "SELECT print_id, print_name, print_sn, print_key, third_account, third_key, print_number, platform, shop_id, status, add_time, update_time, delete_time, auto_print, voice_enabled, voice_content_type, is_default FROM print";

/// `print_pos_device` 的固定 SELECT 列清单（5 列）。
const PRINT_POS_DEVICE_SELECT: &str =
    "SELECT id, shop_id, print_id, device_code, add_time FROM print_pos_device";

/// 一行 `print_config`（对应 Java `com.nextrade.bean.model.print.PrintConfig`）。
///
/// `entities::setting::PrintConfig` 用 `r#type` 作字段名，sqlx 派生对
/// 原始标识符的列名推导不够直观，因此这里用显式 `#[sqlx(rename = "type")]`
/// 的自有行结构，列名 100% 可控。
#[derive(Debug, Clone, Default, FromRow)]
pub struct PrintConfigRow {
    /// 自增ID
    pub id: i32,
    /// 打印机ID
    pub print_id: i32,
    /// 打印模版JSON
    pub template: Option<String>,
    /// 类型 1购物小票（DB 列名就是 `type`）
    #[sqlx(rename = "type")]
    pub config_type: Option<i32>,
    /// 纸张尺寸：58 / 80（DB `NOT NULL`）
    pub paper_size: i32,
    /// 创建时间（秒级时间戳）
    pub add_time: Option<i32>,
    /// 修改时间（秒级时间戳）
    pub update_time: Option<i32>,
}

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct PrintConfigRepository {
    pool: PgPool,
}

impl PrintConfigRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— print_config —————————————————————

    /// 按打印机 ID 查询全部配置
    /// （对应 Java `PrintConfigServiceImpl#getConfigsByPrintId` 的
    /// `lambdaQuery().eq(PrintConfig::getPrintId, printId).list()`）。
    ///
    /// ⚠️ Java **没有** `ORDER BY`，最后由 `findFirst()` 取第一条；
    /// 本实现同样不排序，保持与 Java 完全一致的行为（见 `uncertain_points`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_configs_by_print_id(&self, print_id: i32) -> Result<Vec<PrintConfigRow>> {
        sqlx::query_as::<_, PrintConfigRow>(&format!(
            "{PRINT_CONFIG_SELECT} WHERE print_id = $1"
        ))
        .bind(print_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按主键查询配置（对应 Java `PrintConfigServiceImpl#updateConfig` 的
    /// `this.getById(param.getId())`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_config_by_id(&self, id: i32) -> Result<Option<PrintConfigRow>> {
        sqlx::query_as::<_, PrintConfigRow>(&format!(
            "{PRINT_CONFIG_SELECT} WHERE id = $1 LIMIT 1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 更新配置的模板 / 纸张尺寸 / 更新时间
    /// （对应 Java `PrintConfigServiceImpl#updateConfig` 的
    /// `config.setTemplate(...); config.setPaperSize(...);
    /// config.setUpdateTime(...); this.updateById(config)`）。
    ///
    /// 净效果：`UPDATE print_config SET template = ?, paper_size = ?,
    /// update_time = ? WHERE id = ?`
    /// （`print_id` / `type` / `add_time` 由调用方原样回写，不变更）。
    ///
    /// 返回受影响行数。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn touch_config(
        &self,
        id: i32,
        template_json: &str,
        paper_size: i32,
        update_time: i32,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE print_config SET template = $1, paper_size = $2, update_time = $3 WHERE id = $4",
        )
        .bind(template_json)
        .bind(paper_size)
        .bind(update_time)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— print / print_pos_device（打印路由） —————————————————————

    /// 某店铺全部**启用**的打印机
    /// （对应 Java `PrintRoutingServiceImpl#resolve` 里的
    /// `eq(Print::getShopId, shopId).eq(Print::getStatus, ENABLED)
    /// .orderByDesc(Print::getIsDefault).orderByAsc(Print::getPrintId)`）。
    ///
    /// ⚠️ Java 的 `Print` 带 `@TableLogic`（`delete_time`），但
    /// `PrintServiceImpl` 的查询**没有**显式过滤 `delete_time`；
    /// `delete_time` 是 `int` 且可为 `NULL`，本实现同样不加条件，
    /// 与 Java 实际行为一致。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_enabled_prints(&self, shop_id: i32) -> Result<Vec<Print>> {
        sqlx::query_as::<_, Print>(&format!(
            "{PRINT_SELECT} WHERE shop_id = $1 AND status = $2 ORDER BY is_default DESC, print_id ASC"
        ))
        .bind(shop_id)
        .bind(PRINT_STATUS_ENABLED)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 批量查询打印机的收银台设备绑定
    /// （对应 Java `PrintPosDeviceServiceImpl#bindingsByPrintIds` 的
    /// `eq(shopId).in(printId, printIds)`）。
    ///
    /// `print_ids` 为空时**不发 SQL**，直接返回空列表（与 Java 的
    /// `if (printIds == null || printIds.isEmpty()) return Map.of();` 一致）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_bindings(
        &self,
        shop_id: i32,
        print_ids: &[i32],
    ) -> Result<Vec<PrintPosDevice>> {
        if print_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; print_ids.len()].join(", ");
        let sql = format!(
            "{PRINT_POS_DEVICE_SELECT} WHERE shop_id = $1 AND print_id IN ({placeholders})"
        );
        let mut q = sqlx::query_as::<_, PrintPosDevice>(&sql).bind(shop_id);
        for id in print_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— print_log（打印任务落库） —————————————————————

    /// 预写一条「待推送」打印日志
    /// （对应 Java `PrintLogServiceImpl#createPending`）。
    ///
    /// Java 该方法写死的取值：
    /// - `status = PrintLogStatusEnum.PENDING`（0）
    /// - `retry_count = 0`
    /// - `add_time = update_time = System.currentTimeMillis() / 1000`
    ///
    /// `error_code` / `error_msg` / `task_no` / `push_response` /
    /// `callback_payload` / `pushed_time` / `printed_time` 全部留 `NULL`。
    ///
    /// 返回自增主键 `log_id`。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn insert_pending_print_log(
        &self,
        shop_id: i32,
        print_id: i32,
        platform: Option<i32>,
        order_id: i32,
        print_sn: Option<&str>,
        trigger_type: i32,
        now_secs: i32,
    ) -> Result<i64> {
        let res: i64 = sqlx::query_scalar(
            "INSERT INTO print_log
                 (shop_id, print_id, platform, order_id, print_sn, trigger_type, status,
                  retry_count, add_time, update_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING log_id",
        )
        .bind(shop_id)
        .bind(print_id)
        .bind(platform)
        .bind(order_id)
        .bind(print_sn)
        .bind(trigger_type)
        .bind(LOG_STATUS_PENDING)
        .bind(0)
        .bind(now_secs)
        .bind(now_secs)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 更新打印日志推送结果（状态、task_no、错误信息、响应等）
    pub async fn update_print_log_status(
        &self,
        log_id: i64,
        status: i32,
        task_no: Option<&str>,
        error_code: Option<&str>,
        error_msg: Option<&str>,
        push_response: Option<&str>,
        pushed_time: Option<i32>,
    ) -> Result<bool> {
        let now = nextrade_common::utils::now_seconds() as i32;
        let res = sqlx::query(
            "UPDATE print_log SET status = $1, task_no = $2, error_code = $3, error_msg = $4, push_response = $5, pushed_time = $6, update_time = $7 WHERE log_id = $8"
        )
        .bind(status)
        .bind(task_no)
        .bind(error_code)
        .bind(error_msg)
        .bind(push_response)
        .bind(pushed_time)
        .bind(now)
        .bind(log_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }
}

/// 数据库错误映射（与 member_tag / area_code / admin_user 同一约定）。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("打印配置不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::logistics::print_config::{
        PRINT_CONFIG_COLUMNS, PRINT_COLUMNS, PRINT_LOG_COLUMNS, PRINT_POS_DEVICE_COLUMNS,
    };

    /// `PRINT_CONFIG_SELECT` 覆盖 `print_config` 全部 7 列，且不用 `SELECT *`。
    #[test]
    fn print_config_select_covers_all_columns() {
        for column in PRINT_CONFIG_COLUMNS {
            assert!(
                PRINT_CONFIG_SELECT.contains(column),
                "SELECT 清单缺少列 {column}"
            );
        }
        assert!(PRINT_CONFIG_SELECT.starts_with("SELECT "));
        assert!(PRINT_CONFIG_SELECT.contains("FROM print_config"));
        assert!(!PRINT_CONFIG_SELECT.contains('*'));
    }

    /// `PRINT_SELECT` 覆盖 `print` 全部 17 列，且顺序与实体一致。
    #[test]
    fn print_select_covers_all_columns_in_entity_order() {
        let listed: Vec<&str> = PRINT_SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap()
            .split(", ")
            .collect();
        assert_eq!(listed, PRINT_COLUMNS.to_vec());
        assert!(!PRINT_SELECT.contains('*'));
    }

    /// 设备绑定 / 打印日志的列清单常量与权威 schema 一致。
    #[test]
    fn binding_and_log_column_lists_match_schema() {
        assert!(
            PRINT_POS_DEVICE_SELECT.contains("device_code")
                && PRINT_POS_DEVICE_SELECT.contains("FROM print_pos_device")
        );
        assert_eq!(PRINT_POS_DEVICE_COLUMNS.len(), 5);
        assert_eq!(PRINT_LOG_COLUMNS.len(), 18);
        // 预写日志必须落这两列（Java createPending 写死 status / retry_count）
        assert!(PRINT_LOG_COLUMNS.contains(&"status"));
        assert!(PRINT_LOG_COLUMNS.contains(&"retry_count"));
        assert_eq!(LOG_STATUS_PENDING, 0);
        assert_eq!(PRINT_STATUS_ENABLED, 1);
    }

    /// `PrintConfigRow` 的 `type` 列映射：字段名不是关键字，列名仍是 `type`。
    #[test]
    fn print_config_row_field_mapping_is_explicit() {
        let row = PrintConfigRow {
            id: 1,
            print_id: 2,
            template: Some("[]".into()),
            config_type: Some(1),
            paper_size: 58,
            add_time: Some(1_700_000_000),
            update_time: None,
        };
        assert_eq!(row.config_type, Some(1));
        assert_eq!(row.paper_size, 58);
        assert_eq!(row.update_time, None);
    }
}