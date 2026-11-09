//! 多语言/币种仓储（system::locales）。
//!
//! 对应原 Java（overseas 版）：
//! - `api/controller/common/I18nController` → `service/lang/impl/*`
//! - `api/controller/common/CurrencyController` → `service/lang/impl/CurrencyServiceImpl`
//!
//! 表：`locales` / `locales_relation` / `translations_data` / `currency`
//! （schema 见 overseas 工程 `database/pure.sql`）。

use serde::Serialize;
use sqlx::PgPool;
use nextrade_core::{Error, Result};

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

/// 语言（Java `LocalesVO`）。
#[derive(Debug, Clone, Default, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LocalesVO {
    pub id: i32,
    pub locale_code: String,
    pub language: String,
    pub flag_picture: String,
    pub last_updated: i64,
    pub is_enabled: i32,
    pub is_default: i32,
    pub currency_id: i32,
    pub sort: Option<i32>,
}

/// 语言 + 关联币种（Java `LocalesCurrencyVO`）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LocalesCurrencyVO {
    #[serde(flatten)]
    pub locales: LocalesVO,
    pub currency: Option<CurrencyVO>,
}

/// 币种（Java `CurrencyVO`）。
#[derive(Debug, Clone, Default, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CurrencyVO {
    pub id: i32,
    pub name: String,
    pub symbol: String,
    pub is_default: i32,
    pub rate: String,
}

/// 翻译数据行（仅取页面类型）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TranslationRow {
    pub translation_name: Option<String>,
    pub translation_value: Option<String>,
}

#[derive(Clone)]
pub struct LocalesRepository {
    pool: PgPool,
}

impl LocalesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 全部币种（Java `CurrencyServiceImpl#getCurrency`）。
    pub async fn currencies(&self) -> Result<Vec<CurrencyVO>> {
        sqlx::query_as::<_, CurrencyVO>(
            "SELECT id, name, symbol, is_default, rate FROM currency ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 启用语言列表（Java `LocalesServiceImpl#getLocales`，含关联币种）。
    pub async fn locales_with_currency(&self) -> Result<Vec<LocalesCurrencyVO>> {
        let locales = sqlx::query_as::<_, LocalesVO>(
            "SELECT id, locale_code, language, flag_picture, last_updated, is_enabled, \
             is_default, currency_id, sort FROM locales \
             WHERE is_enabled = 1 ORDER BY sort ASC, id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let currencies = self.currencies().await?;
        Ok(locales
            .into_iter()
            .map(|l| LocalesCurrencyVO {
                currency: currencies.iter().find(|c| c.id == l.currency_id).cloned(),
                locales: l,
            })
            .collect())
    }

    /// 默认语言（Java `LocalesRelationServiceImpl#getDefaultLocale`）。
    ///
    /// code 非空时先按 `locales_relation.code` 映射；查不到回落 `is_default=1`。
    pub async fn default_locale(&self, code: Option<&str>) -> Result<Option<LocalesVO>> {
        if let Some(code) = code.filter(|c| !c.trim().is_empty()) {
            let row = sqlx::query_as::<_, LocalesVO>(
                "SELECT l.id, l.locale_code, l.language, l.flag_picture, l.last_updated, \
                 l.is_enabled, l.is_default, l.currency_id, l.sort \
                 FROM locales_relation r JOIN locales l ON l.id = r.locales_id \
                 WHERE r.code = ? LIMIT 1",
            )
            .bind(code.trim())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?;
            if row.is_some() {
                return Ok(row);
            }
        }
        sqlx::query_as::<_, LocalesVO>(
            "SELECT id, locale_code, language, flag_picture, last_updated, is_enabled, \
             is_default, currency_id, sort FROM locales \
             WHERE is_default = 1 LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 页面翻译包（Java `TranslationsDataServiceImpl#getLocaleTranslations`，
    /// `data_type=0` 页面类型；name/value 任一为 NULL 的行丢弃）。
    pub async fn locale_translations(
        &self,
        locale_code: &str,
    ) -> Result<Vec<(String, String)>> {
        let rows = sqlx::query_as::<_, TranslationRow>(
            "SELECT t.translation_name, t.translation_value \
             FROM translations_data t \
             JOIN locales l ON l.id = t.locale_id \
             WHERE l.locale_code = ? AND l.is_enabled = 1 AND t.data_type = 0",
        )
        .bind(locale_code)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .filter_map(|r| Some((r.translation_name?, r.translation_value?)))
            .collect())
    }
}
