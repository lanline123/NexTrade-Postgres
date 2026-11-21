//! 批发汇总域（wholesale::wholesale_summary）仓储。**我方新增**
//! （论坛帖 127889 评估 F3 + 门店级批发清单，二期补齐）。
//!
//! 覆盖表（只读聚合）：
//! - `order_item`（批发行标记 `is_wholesale_line` / 成交单价 `price` /
//!   数量 `quantity` / 归属 `shop_id`）
//! - `order`（订单头：`is_del` / `pay_status` / `order_status` /
//!   `add_time` 有效性过滤）
//! - `product_wholesale_config`（已配置批发商品计数）
//! - `wholesale_shop_setting`（店级开关原始值）
//! - `config`（全局批发开关，经 [`super::wholesale_setting::read_config_val`]）
//! - `wholesaler`（启用批发商计数）
//! - `shop`（门店名 `shop_title`）
//!
//! 口径见 `core::models::wholesale::wholesale_summary` 模块头：已支付
//! （`pay_status = 2`）、未取消（`order_status <> 3`）、未删
//! （`is_del = 0`）订单的批发行，按明细行 `shop_id` 归属门店，
//! 金额 = 成交单价 × 数量，时间按订单头 `add_time` 区间。
//!
//! 查询统一使用运行时 `sqlx::query_as`（**不使用** `query!` 宏）。

use rust_decimal::Decimal;
use sqlx::PgPool;
use sqlx::FromRow;

use nextrade_core::models::wholesale::wholesale_setting::BIZ_WHOLESALE_ENABLED;
use nextrade_core::{Error, Result};

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

/// 批发成交聚合行（overview 与 shopPage 共用形状）。
#[derive(Debug, Clone, FromRow)]
pub struct WholesaleAggSqlRow {
    /// 门店 ID（overview 时为哨兵 0，不使用）
    pub shop_id: i32,
    /// 批发订单数（去重 order_id）
    pub order_count: i64,
    /// 批发行数
    pub line_count: i64,
    /// 批发件数
    pub total_quantity: Option<i64>,
    /// 批发成交金额
    pub total_amount: Option<Decimal>,
    /// 批发买家数（去重 user_id）
    pub buyer_count: i64,
    /// 有批发成交的门店数（仅 overview 使用）
    pub shop_count: i64,
}

/// 有效批发成交的过滤片段（订单头别名 `o`、明细别名 `oi`）。
const VALID_WHOLESALE_LINE_WHERE: &str = "oi.is_wholesale_line = 1 \
    AND o.is_del = 0 AND o.pay_status = 2 AND o.order_status <> 3 \
    AND o.add_time >= $1 AND o.add_time <= $2";

/// 批发汇总仓储（持有连接池）。
#[derive(Clone)]
pub struct WholesaleSummaryRepository {
    pool: PgPool,
}

impl WholesaleSummaryRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 区间总计（`scope = None` 全平台；`Some(shop_id)` 限定单店）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn aggregate(
        &self,
        scope: Option<i32>,
        start: i64,
        end: i64,
    ) -> Result<WholesaleAggSqlRow> {
        let mut sql = format!(
            "SELECT 0 AS shop_id, \
             COUNT(DISTINCT oi.order_id) AS order_count, \
             COUNT(*) AS line_count, \
             SUM(oi.quantity) AS total_quantity, \
             SUM(oi.price * oi.quantity) AS total_amount, \
             COUNT(DISTINCT oi.user_id) AS buyer_count, \
             COUNT(DISTINCT oi.shop_id) AS shop_count \
             FROM order_item oi JOIN \"order\" o ON o.order_id = oi.order_id \
             WHERE {VALID_WHOLESALE_LINE_WHERE}"
        );
        if scope.is_some() {
            sql.push_str(" AND oi.shop_id = $3");
        }
        let mut q = sqlx::query_as::<_, WholesaleAggSqlRow>(&sql)
            .bind(start)
            .bind(end);
        if let Some(sid) = scope {
            q = q.bind(sid);
        }
        q.fetch_one(&self.pool).await.map_err(map_err)
    }

    /// 按门店聚合（`scope = None` 全部门店；`Some(shop_id)` 仅该店）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn aggregate_by_shop(
        &self,
        scope: Option<i32>,
        start: i64,
        end: i64,
    ) -> Result<Vec<WholesaleAggSqlRow>> {
        let mut sql = format!(
            "SELECT oi.shop_id AS shop_id, \
             COUNT(DISTINCT oi.order_id) AS order_count, \
             COUNT(*) AS line_count, \
             SUM(oi.quantity) AS total_quantity, \
             SUM(oi.price * oi.quantity) AS total_amount, \
             COUNT(DISTINCT oi.user_id) AS buyer_count, \
             0 AS shop_count \
             FROM order_item oi JOIN \"order\" o ON o.order_id = oi.order_id \
             WHERE {VALID_WHOLESALE_LINE_WHERE}"
        );
        if scope.is_some() {
            sql.push_str(" AND oi.shop_id = $3");
        }
        sql.push_str(" GROUP BY oi.shop_id");
        let mut q = sqlx::query_as::<_, WholesaleAggSqlRow>(&sql)
            .bind(start)
            .bind(end);
        if let Some(sid) = scope {
            q = q.bind(sid);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 各门店启用中的已配置批发商品数（`status = 1 AND is_delete = 0`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn configured_counts_by_shop(
        &self,
        scope: Option<i32>,
    ) -> Result<Vec<(i32, i64)>> {
        let rows: Vec<(i32, i64)> = match scope {
            Some(sid) => {
                sqlx::query_as(
                    "SELECT shop_id, COUNT(*) FROM product_wholesale_config \
                     WHERE status = 1 AND is_delete = 0 AND shop_id = $1 GROUP BY shop_id",
                )
                .bind(sid)
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?
            }
            None => {
                sqlx::query_as(
                    "SELECT shop_id, COUNT(*) FROM product_wholesale_config \
                     WHERE status = 1 AND is_delete = 0 GROUP BY shop_id",
                )
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?
            }
        };
        Ok(rows)
    }

    /// 启用批发商总数（`status = 1 AND is_delete = 0`，买家全局身份）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn active_wholesaler_count(&self) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM wholesaler WHERE status = 1 AND is_delete = 0")
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0)
    }

    /// 全局批发开关（`config.wholesaleEnabled`，缺失按关）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn global_wholesale_enabled(&self) -> Result<bool> {
        let val = super::wholesale_setting::read_config_val(&self.pool, BIZ_WHOLESALE_ENABLED)
            .await?;
        Ok(val.as_deref().map(str::trim) == Some("1"))
    }

    /// 店级批发开关原始值全量（`shop_id → enabled`；未配置的店不在结果内，
    /// 由服务层按「未配置默认开」口径解读）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn shop_switch_values(&self) -> Result<Vec<(i32, i32)>> {
        let rows: Vec<(i32, i32)> =
            sqlx::query_as("SELECT shop_id, enabled FROM wholesale_shop_setting")
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows)
    }

    /// 批量门店名（`shop_id → shop_title`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn shop_titles(&self, ids: &[i32]) -> Result<Vec<(i32, String)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows: Vec<(i32, String)> = sqlx::query_as(
            "SELECT shop_id, shop_title FROM shop WHERE shop_id = ANY($1)",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows)
    }
}
