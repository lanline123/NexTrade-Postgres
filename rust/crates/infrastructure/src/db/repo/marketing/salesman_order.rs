//! 分销业绩订单仓储（marketing::salesman_order）。
//!
//! 主表 `salesman_order`（12 列，`python tools/schema_parser.py --table
//! salesman_order`）；只读关联 `order`：
//! - `order_sn` 回填（VO）
//! - 时间类型过滤（`order.add_time` / `order.pay_time` / `order.received_time`）
//! - `order_sn` 模糊查询先取 `order_id` 集合再 `IN`
//!
//! 查询统一运行时 `sqlx::query_as`。
//! 表无逻辑删除列：`batch(del)` 物理删除。

use std::collections::HashMap;

use sqlx::PgPool;

use nextrade_core::entities::SalesmanOrder;
use nextrade_core::models::marketing::salesman_order::MSG_ORDER_NOT_FOUND;
use nextrade_core::{Error, Result};

/// `salesman_order` 固定 SELECT 列（12 列）。
pub const ORDER_COLUMNS: &str = "salesman_order_id, order_id, salesman_id, amount, status, add_time, item_id, salesman_product_data, order_amount, salesman_settlement_data, settlement_time, product_id";

/// 业绩订单仓储。
#[derive(Clone)]
pub struct SalesmanOrderRepository {
    pool: PgPool,
}

impl SalesmanOrderRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 按订单号模糊匹配 `order_id`（Java `orderService.lambdaQuery().like(orderSn)`）。
    pub async fn order_ids_by_sn(&self, order_sn: &str) -> Result<Vec<i32>> {
        let like = format!("%{order_sn}%");
        let rows: Vec<(i32,)> =
            sqlx::query_as("SELECT order_id FROM \"order\" WHERE order_sn LIKE $1 LIMIT 5000")
                .bind(like)
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 按时间类型筛选出 `order_id` 集合（`timeType`: 1 下单 / 2 支付 / 3 收货）。
    pub async fn order_ids_by_time(
        &self,
        time_type: i32,
        start: Option<i64>,
        end: Option<i64>,
    ) -> Result<Vec<i32>> {
        let column = match time_type {
            2 => "pay_time",
            3 => "received_time",
            _ => "add_time",
        };
        let mut where_parts: Vec<String> = Vec::new();
        if start.is_some() {
            where_parts.push(format!("\"{column}\" >= ?"));
        }
        if end.is_some() {
            where_parts.push(format!("\"{column}\" <= ?"));
        }
        let where_sql = if where_parts.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_parts.join(" AND "))
        };
        let sql = format!("SELECT order_id FROM \"order\" {where_sql} LIMIT 5000");
        let mut q = sqlx::query_as::<_, (i32,)>(&sql);
        if let Some(v) = start {
            q = q.bind(v);
        }
        if let Some(v) = end {
            q = q.bind(v);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 分页（Java `list` 的过滤组合：`salesmanId` + `orderIds` + `status` +
    /// 多租户 `shop_id`（经 `salesman` 关联））。
    pub async fn paginate(
        &self,
        salesman_id: Option<i32>,
        shop_id: Option<i32>,
        order_ids: Option<&[i32]>,
        status: Option<i32>,
        sort_field: Option<&str>,
        sort_order: Option<&str>,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<SalesmanOrder>)> {
        let mut where_parts: Vec<String> = Vec::new();
        if salesman_id.is_some() {
            where_parts.push("salesman_id = ?".into());
        }
        // 多租户：店铺账号只看到本店分销员的订单（经 salesman.shop_id 关联）
        if let Some(shop_id) = shop_id {
            where_parts.push(format!(
                "salesman_id IN (SELECT salesman_id FROM salesman WHERE shop_id = {shop_id})"
            ));
        }
        if let Some(ids) = order_ids {
            if ids.is_empty() {
                return Ok((0, Vec::new()));
            }
            let ph = vec!["?"; ids.len()].join(", ");
            where_parts.push(format!("order_id IN ({ph})"));
        }
        if let Some(v) = status.filter(|s| *s != -1) {
            where_parts.push("status = ?".into());
            let _ = v;
        }
        let where_sql = if where_parts.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_parts.join(" AND "))
        };
        let column = resolve_sort_column(sort_field);
        let dir = if sort_order.is_some_and(|o| o.eq_ignore_ascii_case("asc")) {
            "ASC"
        } else {
            "DESC"
        };

        let count_sql = format!("SELECT COUNT(*) FROM salesman_order {where_sql}");
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        if let Some(v) = salesman_id {
            cq = cq.bind(v);
        }
        if let Some(ids) = order_ids {
            for id in ids {
                cq = cq.bind(*id);
            }
        }
        if let Some(v) = status.filter(|s| *s != -1) {
            cq = cq.bind(v);
        }
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let list_sql = format!(
            "SELECT {ORDER_COLUMNS} FROM salesman_order {where_sql} ORDER BY {column} {dir} LIMIT ? OFFSET ?"
        );
        let mut lq = sqlx::query_as::<_, SalesmanOrder>(&list_sql);
        if let Some(v) = salesman_id {
            lq = lq.bind(v);
        }
        if let Some(ids) = order_ids {
            for id in ids {
                lq = lq.bind(*id);
            }
        }
        if let Some(v) = status.filter(|s| *s != -1) {
            lq = lq.bind(v);
        }
        let rows = lq
            .bind(limit.max(1) as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 按主键查询。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<SalesmanOrder>> {
        let sql = format!(
            "SELECT {ORDER_COLUMNS} FROM salesman_order WHERE salesman_order_id = $1 LIMIT 1"
        );
        sqlx::query_as::<_, SalesmanOrder>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 订单号回填（`order_id → order_sn`）。
    pub async fn order_sn_map(&self, order_ids: &[i32]) -> Result<HashMap<i32, String>> {
        let ids: Vec<i32> = order_ids.iter().copied().filter(|id| *id > 0).collect();
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let ph = vec!["?"; ids.len()].join(", ");
        let sql = format!("SELECT order_id, order_sn FROM \"order\" WHERE order_id IN ({ph})");
        let mut q = sqlx::query_as::<_, (i32, String)>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        Ok(q.fetch_all(&self.pool)
            .await
            .map_err(map_err)?
            .into_iter()
            .collect())
    }

    /// 人工结算：仅 `status=0` 记录置为已结算并写结算时间（差异 1）。
    pub async fn mark_settled(&self, ids: &[i32], now: i32) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let ph = vec!["?"; ids.len()].join(", ");
        let sql = format!(
            "UPDATE salesman_order SET status = 1, settlement_time = $1 WHERE status = 0 AND salesman_order_id IN ({ph})"
        );
        let mut q = sqlx::query(&sql).bind(now);
        for id in ids {
            q = q.bind(*id);
        }
        let res = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 物理删除（`batch(type=del)`）。
    pub async fn delete(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let ph = vec!["?"; ids.len()].join(", ");
        let sql = format!("DELETE FROM salesman_order WHERE salesman_order_id IN ({ph})");
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        let res = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }
}

/// 排序字段白名单（`salesman_order` 实存列；未知回退主键）。
#[must_use]
pub fn resolve_sort_column(field: Option<&str>) -> &'static str {
    match field {
        Some("salesmanOrderId") | Some("salesman_order_id") => "salesman_order_id",
        Some("addTime") | Some("add_time") => "add_time",
        Some("amount") => "amount",
        Some("status") => "status",
        Some("settlementTime") | Some("settlement_time") => "settlement_time",
        _ => "salesman_order_id",
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound(MSG_ORDER_NOT_FOUND.into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_cover_entity() {
        assert_eq!(ORDER_COLUMNS.split(", ").count(), 12);
        assert!(!ORDER_COLUMNS.contains('*'));
    }

    #[test]
    fn sort_whitelist_falls_back_to_primary_key() {
        assert_eq!(resolve_sort_column(None), "salesman_order_id");
        assert_eq!(resolve_sort_column(Some("addTime")), "add_time");
        assert_eq!(resolve_sort_column(Some("settlementTime")), "settlement_time");
        assert_eq!(resolve_sort_column(Some("1;DROP TABLE salesman_order")), "salesman_order_id");
    }
}
