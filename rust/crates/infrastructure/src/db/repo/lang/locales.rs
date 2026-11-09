//! 语言地区维护表仓储（lang::locales）。
//!
//! Java 来源（overseas）：`LocalesMapper`（MyBatis-Plus LambdaQueryWrapper，
//! `LocalesServiceImpl` 的 list/detail/create/update/batch/updateField/
//! getLocalesLimit3）。表 `locales`（b2b2b 增量列 `domain`）。
//!
//! 另含 `locales_lang` 全表读取（`LocalesRelationController.config` →
//! `localesLangService.list()`）。
//!
//! SQL 一律运行时 `sqlx::query_as`（本机无 `DATABASE_URL`，不用宏）。

use sqlx::PgPool;

use nextrade_core::entities::{Locales, LocalesLang};
use nextrade_core::{Error, Result};

/// `locales` 固定 SELECT 列清单（顺序与实体一致）。
pub const LOCALES_COLUMNS: [&str; 10] = [
    "id",
    "locale_code",
    "language",
    "domain",
    "flag_picture",
    "last_updated",
    "is_enabled",
    "is_default",
    "currency_id",
    "sort",
];

/// 语言地区仓储。
#[derive(Clone)]
pub struct LocalesRepository {
    pool: PgPool,
}

impl LocalesRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn select_sql(where_sql: &str) -> String {
        format!(
            "SELECT {} FROM locales {where_sql}",
            LOCALES_COLUMNS.join(", ")
        )
    }

    /// 分页列表（Java `list`：`isEnabled` 等值过滤 + 排序白名单）。
    ///
    /// `order_by` 由调用方按白名单生成（缺省 `id ASC`）。
    pub async fn paginate(
        &self,
        is_enabled: Option<i32>,
        order_by: &str,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<Locales>)> {
        let where_sql = match is_enabled {
            Some(v) => "WHERE is_enabled = $1".to_string(),
            None => String::new(),
        };
        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM locales {where_sql}"));
        let count: i64 = if is_enabled.is_some() {
            sqlx::query_scalar(&count_sql)
                .bind(is_enabled.unwrap_or(1))
                .fetch_one(&self.pool)
                .await
        } else {
            sqlx::query_scalar(&count_sql).fetch_one(&self.pool).await
        }
        .map_err(map_err)?;
        let list_sql = Self::select_sql(&format!("{where_sql} ORDER BY {order_by} LIMIT ? OFFSET ?"));
        let mut q = sqlx::query_as::<_, Locales>(&list_sql);
        if let Some(v) = is_enabled {
            q = q.bind(v);
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
    pub async fn find_by_id(&self, id: i32) -> Result<Option<Locales>> {
        let sql = Self::select_sql("WHERE id = $1 LIMIT 1");
        sqlx::query_as::<_, Locales>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 全表（Java `localesService.list()`，`exchangeRateConfig` 等场景）。
    pub async fn find_all(&self) -> Result<Vec<Locales>> {
        let sql = Self::select_sql("ORDER BY id ASC");
        sqlx::query_as::<_, Locales>(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 非中文启用语言前 N 条（Java `getLocalesLimit3`：
    /// `is_enabled = 1 AND locale_code <> 'zh' LIMIT n`）。
    pub async fn find_enabled_non_zh_limit(&self, limit: u64) -> Result<Vec<Locales>> {
        let sql = Self::select_sql(
            "WHERE is_enabled = 1 AND locale_code <> 'zh' ORDER BY id ASC LIMIT $1",
        );
        sqlx::query_as::<_, Locales>(&sql)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按 ID 集合批量查（`list` 补货币、`batch` 删除前取行）。
    pub async fn find_by_ids(&self, ids: &[i32]) -> Result<Vec<Locales>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql = Self::select_sql(&format!(
            "WHERE id IN ({placeholders}) ORDER BY id ASC"
        ));
        let mut q = sqlx::query_as::<_, Locales>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 插入（Java `save`）。
    pub async fn insert(&self, row: &Locales) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO locales (locale_code, language, domain, flag_picture, last_updated, is_enabled, is_default, currency_id, sort) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
        )
        .bind(&row.locale_code)
        .bind(&row.language)
        .bind(&row.domain)
        .bind(&row.flag_picture)
        .bind(row.last_updated)
        .bind(row.is_enabled)
        .bind(row.is_default)
        .bind(row.currency_id)
        .bind(row.sort)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 按主键全量更新（Java `updateById`）。
    pub async fn update(&self, row: &Locales) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE locales SET locale_code = $1, language = $2, domain = $3, flag_picture = $4, last_updated = $5, is_enabled = $6, is_default = $7, currency_id = $8, sort = $9 WHERE id = $10",
        )
        .bind(&row.locale_code)
        .bind(&row.language)
        .bind(&row.domain)
        .bind(&row.flag_picture)
        .bind(row.last_updated)
        .bind(row.is_enabled)
        .bind(row.is_default)
        .bind(row.currency_id)
        .bind(row.sort)
        .bind(row.id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 单字段更新（Java `updateField`，列名已在服务层白名单校验）。
    pub async fn update_field(&self, id: i32, column: &str, value: &str) -> Result<u64> {
        let sql = format!("UPDATE locales SET {column} = $1 WHERE id = $2");
        let res = sqlx::query(&sql)
            .bind(value)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 清空默认标记（`isDefault` 联动：`UPDATE locales SET is_default = 0`）。
    pub async fn clear_default(&self) -> Result<u64> {
        let res = sqlx::query("UPDATE locales SET is_default = 0 WHERE is_default = 1")
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
        let sql = crate::db::to_pg_placeholders(&format!("DELETE FROM locales WHERE id IN ({placeholders})"));
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(id);
        }
        let res = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }
}

/// `locales_lang` 仓储（仅全表读取）。
#[derive(Clone)]
pub struct LocalesLangRepository {
    pool: PgPool,
}

impl LocalesLangRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 全表（Java `localesLangService.list()`，`localesRelation/config` 出参）。
    pub async fn find_all(&self) -> Result<Vec<LocalesLang>> {
        sqlx::query_as::<_, LocalesLang>(
            "SELECT code, name, id, name_en FROM locales_lang ORDER BY id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
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
        assert_eq!(LOCALES_COLUMNS.len(), 10);
        assert!(LOCALES_COLUMNS.contains(&"domain"), "b2b2b 增量列 domain");
    }
}
