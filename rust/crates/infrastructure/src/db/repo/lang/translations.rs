//! 多语言业务翻译仓储（lang::translations）。
//!
//! Java 来源：`TranslationsDataMapper`（MyBatis-Plus LambdaQueryWrapper）。
//! 表 `translations_data`（8 列）：`id` / `data_id` / `data_type` /
//! `locale_id` / `translation_key` / `translation_name` /
//! `translation_value` / `translation_source`。
//!
//! SQL 一律运行时 `sqlx::query_as`（本机无 `DATABASE_URL`，不用宏）。

use sqlx::PgPool;

use nextrade_core::entities::{Translations, TranslationsData};
use nextrade_core::{Error, Result};

/// `translations_data` 固定 SELECT 列清单（顺序与实体一致，8 列）。
pub const TRANSLATIONS_DATA_COLUMNS: [&str; 8] = [
    "id",
    "data_id",
    "data_type",
    "locale_id",
    "translation_key",
    "translation_name",
    "translation_value",
    "translation_source",
];

/// 多语言业务翻译仓储。
#[derive(Clone)]
pub struct TranslationsRepository {
    pool: PgPool,
}

impl TranslationsRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn select_sql(where_sql: &str) -> String {
        format!(
            "SELECT {} FROM translations_data {where_sql}",
            TRANSLATIONS_DATA_COLUMNS.join(", ")
        )
    }

    /// 按数据类型 + 数据 id 查询（Java `getData(dataType, dataId)`）。
    pub async fn find_by_data_id(&self, data_type: i32, data_id: i32) -> Result<Vec<TranslationsData>> {
        let sql = Self::select_sql("WHERE data_type = ? AND data_id = ?");
        sqlx::query_as::<_, TranslationsData>(&sql)
            .bind(data_type)
            .bind(data_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按数据类型 + 原文查询（Java `getData(dataType, translationName)`）。
    pub async fn find_by_name(
        &self,
        data_type: i32,
        translation_name: &str,
    ) -> Result<Vec<TranslationsData>> {
        let sql = Self::select_sql("WHERE data_type = ? AND translation_name = ?");
        sqlx::query_as::<_, TranslationsData>(&sql)
            .bind(data_type)
            .bind(translation_name)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按 (translation_key, data_type, locale_id) 查单条
    /// （Java `createTranslations` 的幂等 upsert 查询，`.last("limit 1")`）。
    pub async fn find_by_key_type_locale(
        &self,
        translation_key: &str,
        data_type: i32,
        locale_id: i32,
    ) -> Result<Option<TranslationsData>> {
        let sql = Self::select_sql(
            "WHERE translation_key = ? AND data_type = ? AND locale_id = ? LIMIT 1",
        );
        sqlx::query_as::<_, TranslationsData>(&sql)
            .bind(translation_key)
            .bind(data_type)
            .bind(locale_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 插入一条翻译数据（Java `translationsDataService.save(data)`）。
    pub async fn insert(&self, row: &TranslationsData) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO translations_data (data_id, data_type, locale_id, translation_key, translation_name, translation_value, translation_source) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
        )
        .bind(row.data_id)
        .bind(row.data_type)
        .bind(row.locale_id)
        .bind(&row.translation_key)
        .bind(&row.translation_name)
        .bind(&row.translation_value)
        .bind(row.translation_source)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 更新一条翻译数据（Java `translationsDataService.updateById(existing)`）。
    pub async fn update(&self, row: &TranslationsData) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE translations_data SET data_id = $1, data_type = $2, locale_id = $3, translation_key = $4, translation_name = $5, translation_value = $6, translation_source = $7 WHERE id = $8",
        )
        .bind(row.data_id)
        .bind(row.data_type)
        .bind(row.locale_id)
        .bind(&row.translation_key)
        .bind(&row.translation_name)
        .bind(&row.translation_value)
        .bind(row.translation_source)
        .bind(row.id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 按 `(translation_name, data_type, locale_ids)` 查译文
    /// （管理端 `list` 的行内译文列，Java `getData(dataType, name, localeIds)`）。
    pub async fn find_data_by_name_locales(
        &self,
        data_type: i32,
        translation_name: &str,
        locale_ids: &[i32],
    ) -> Result<Vec<TranslationsData>> {
        if locale_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; locale_ids.len()].join(", ");
        let sql = Self::select_sql(&format!(
            "WHERE translation_name = ? AND data_type = ? AND locale_id IN ({placeholders})"
        ));
        let mut q = sqlx::query_as::<_, TranslationsData>(&sql);
        q = q.bind(translation_name).bind(data_type);
        for id in locale_ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 按 `data_id` 集合删译文（`batch del` 联动清理 `translations_data`，
    /// Java `removeBatchByIds(translationsDataList)`）。
    pub async fn delete_data_by_data_ids(&self, data_ids: &[i32]) -> Result<u64> {
        if data_ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; data_ids.len()].join(", ");
        let sql = format!("DELETE FROM translations_data WHERE data_id IN ({placeholders})");
        let mut q = sqlx::query(&sql);
        for id in data_ids {
            q = q.bind(id);
        }
        let res = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }
}

/// `translations`（词条主表）仓储（Java `TranslationsMapper`）。
///
/// 表 `translations`（5 列）：`id` / `translation_name` /
/// `translation_key` / `data_type` / `shop_id`。
#[derive(Clone)]
pub struct TranslationsIndexRepository {
    pool: PgPool,
}

/// `translations` 固定 SELECT 列清单（顺序与实体一致）。
pub const TRANSLATIONS_COLUMNS: [&str; 5] = [
    "id",
    "translation_name",
    "translation_key",
    "data_type",
    "shop_id",
];

impl TranslationsIndexRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn select_sql(where_sql: &str) -> String {
        format!(
            "SELECT {} FROM translations {where_sql}",
            TRANSLATIONS_COLUMNS.join(", ")
        )
    }

    /// 分页列表（Java `TranslationsServiceImpl#list`：
    /// `translation_name` LIKE + `data_type` 等值；平台侧 `shop_id = 0`）。
    pub async fn paginate(
        &self,
        name_like: Option<&str>,
        data_type: Option<i32>,
        order_by: &str,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<Translations>)> {
        let mut where_parts: Vec<String> = Vec::new();
        if name_like.is_some() {
            where_parts.push("translation_name LIKE ?".into());
        }
        if data_type.is_some() {
            where_parts.push("data_type = ?".into());
        }
        let where_sql = if where_parts.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_parts.join(" AND "))
        };
        let count_sql = format!("SELECT COUNT(*) FROM translations {where_sql}");
        let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(name) = name_like {
            count_q = count_q.bind(format!("%{name}%"));
        }
        if let Some(t) = data_type {
            count_q = count_q.bind(t);
        }
        let count = count_q.fetch_one(&self.pool).await.map_err(map_err)?;
        let list_sql =
            Self::select_sql(&format!("{where_sql} ORDER BY {order_by} LIMIT ? OFFSET ?"));
        let mut q = sqlx::query_as::<_, Translations>(&list_sql);
        if let Some(name) = name_like {
            q = q.bind(format!("%{name}%"));
        }
        if let Some(t) = data_type {
            q = q.bind(t);
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
    pub async fn find_by_id(&self, id: i32) -> Result<Option<Translations>> {
        let sql = Self::select_sql("WHERE id = ? LIMIT 1");
        sqlx::query_as::<_, Translations>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按 `(translation_key, data_type)` 计数（`create` 幂等判断）。
    pub async fn count_by_key_type(&self, translation_key: &str, data_type: i32) -> Result<u64> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM translations WHERE translation_key = $1 AND data_type = $2")
                .bind(translation_key)
                .bind(data_type)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(count.max(0) as u64)
    }

    /// 插入词条（Java `save`）。
    pub async fn insert(&self, row: &Translations) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO translations (translation_name, translation_key, data_type, shop_id) VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(&row.translation_name)
        .bind(&row.translation_key)
        .bind(row.data_type)
        .bind(row.shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 按 ID 集合删除（Java `removeBatchByIds`）。
    pub async fn delete_by_ids(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql = format!("DELETE FROM translations WHERE id IN ({placeholders})");
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(id);
        }
        let res = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 一键翻译的词条分批读取（Java `multiple` 的
    /// `orderByDesc(id).last("LIMIT size OFFSET offset")`；`ids` 非空时过滤）。
    pub async fn find_page_for_translate(
        &self,
        data_type: i32,
        ids: &[i32],
        limit: u64,
        offset: u64,
    ) -> Result<Vec<Translations>> {
        let mut where_sql = format!("WHERE data_type = {data_type}");
        if !ids.is_empty() {
            let placeholders = vec!["?"; ids.len()].join(", ");
            where_sql.push_str(&format!(" AND id IN ({placeholders})"));
        }
        let sql = Self::select_sql(&format!("{where_sql} ORDER BY id DESC LIMIT ? OFFSET ?"));
        let mut q = sqlx::query_as::<_, Translations>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.bind(limit as i64)
            .bind(offset as i64)
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
        assert_eq!(TRANSLATIONS_DATA_COLUMNS.len(), 8);
        assert!(TRANSLATIONS_DATA_COLUMNS.contains(&"translation_key"));
        assert!(TRANSLATIONS_DATA_COLUMNS.contains(&"translation_source"));
    }
}
