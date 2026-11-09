//! 货币管理表仓储（lang::currency）。
//!
//! Java 来源（overseas）：`CurrencyMapper`（`CurrencyServiceImpl` 的
//! list/detail/create/update/updateField/batch）。表 `currency`
//! （b2b2b 增量列 `code`，ISO 4217）。

use sqlx::PgPool;

use nextrade_core::entities::Currency;
use nextrade_core::{Error, Result};

/// `currency` 固定 SELECT 列清单（顺序与实体一致）。
pub const CURRENCY_COLUMNS: [&str; 6] = [
    "id",
    "name",
    "symbol",
    "is_default",
    "rate",
    "code",
];

/// 货币仓储。
#[derive(Clone)]
pub struct CurrencyRepository {
    pool: PgPool,
}

impl CurrencyRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn select_sql(where_sql: &str) -> String {
        format!(
            "SELECT {} FROM currency {where_sql}",
            CURRENCY_COLUMNS.join(", ")
        )
    }

    /// 分页列表（Java `list`：`name` LIKE 过滤 + 排序白名单）。
    pub async fn paginate(
        &self,
        name_like: Option<&str>,
        order_by: &str,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<Currency>)> {
        let where_sql = match name_like {
            Some(_) => "WHERE name LIKE $1".to_string(),
            None => String::new(),
        };
        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM currency {where_sql}"));
        let count: i64 = if let Some(name) = name_like {
            sqlx::query_scalar(&count_sql)
                .bind(format!("%{name}%"))
                .fetch_one(&self.pool)
                .await
        } else {
            sqlx::query_scalar(&count_sql).fetch_one(&self.pool).await
        }
        .map_err(map_err)?;
        let list_sql =
            Self::select_sql(&format!("{where_sql} ORDER BY {order_by} LIMIT ? OFFSET ?"));
        let mut q = sqlx::query_as::<_, Currency>(&list_sql);
        if let Some(name) = name_like {
            q = q.bind(format!("%{name}%"));
        }
        let rows = q
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((count.max(0) as u64, rows))
    }

    /// 按主键查单条（Java `getById`）。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<Currency>> {
        let sql = Self::select_sql("WHERE id = $1 LIMIT 1");
        sqlx::query_as::<_, Currency>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 全表（`exchangeRateConfig` / `syncExchangeRateNow` 场景）。
    pub async fn find_all(&self) -> Result<Vec<Currency>> {
        let sql = Self::select_sql("ORDER BY id ASC");
        sqlx::query_as::<_, Currency>(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 默认货币（Java `syncDefaultCurrencyToConfig` 的 `is_default = 1 limit 1`）。
    pub async fn find_default(&self) -> Result<Option<Currency>> {
        let sql = Self::select_sql("WHERE is_default = 1 ORDER BY id ASC LIMIT 1");
        sqlx::query_as::<_, Currency>(&sql)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按 ID 集合批量查（`batch` 删除前取行 / `locales.list` 补货币）。
    pub async fn find_by_ids(&self, ids: &[i32]) -> Result<Vec<Currency>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql =
            Self::select_sql(&format!("WHERE id IN ({placeholders}) ORDER BY id ASC"));
        let mut q = sqlx::query_as::<_, Currency>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 插入（Java `save`）。
    pub async fn insert(&self, row: &Currency) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO currency (name, symbol, is_default, rate, code) VALUES ($1, $2, $3, $4, $5) RETURNING id",
        )
        .bind(&row.name)
        .bind(&row.symbol)
        .bind(row.is_default)
        .bind(&row.rate)
        .bind(&row.code)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 按主键全量更新（Java `updateById`；未传字段置默认语义由服务层处理）。
    pub async fn update(&self, row: &Currency) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE currency SET name = $1, symbol = $2, is_default = $3, rate = $4, code = $5 WHERE id = $6",
        )
        .bind(&row.name)
        .bind(&row.symbol)
        .bind(row.is_default)
        .bind(&row.rate)
        .bind(&row.code)
        .bind(row.id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 单字段更新（Java `updateField`，列名已在服务层白名单校验）。
    pub async fn update_field(&self, id: i32, column: &str, value: &str) -> Result<u64> {
        let sql = format!("UPDATE currency SET {column} = $1 WHERE id = $2");
        let res = sqlx::query(&sql)
            .bind(value)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 更新汇率（`syncExchangeRateNow` 有凭据时的落库路径）。
    pub async fn update_rate(&self, id: i32, rate: &str) -> Result<u64> {
        let res = sqlx::query("UPDATE currency SET rate = $1 WHERE id = $2")
            .bind(rate)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 清空默认标记（`isDefault` 联动）。
    pub async fn clear_default(&self) -> Result<u64> {
        let res = sqlx::query("UPDATE currency SET is_default = 0 WHERE is_default = 1")
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 按 ID 集合删除（Java `removeByIds`）。
    pub async fn delete_by_ids(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("DELETE FROM currency WHERE id IN ({placeholders})"));
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(id);
        }
        let res = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_list_matches_entity() {
        assert_eq!(CURRENCY_COLUMNS.len(), 6);
        assert!(CURRENCY_COLUMNS.contains(&"code"), "b2b2b 增量列 code");
    }
}
