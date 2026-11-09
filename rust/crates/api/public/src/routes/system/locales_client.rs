//! 多语言/币种 —— 商城前台路由（system::locales_client）。
//!
//! 原 Java（overseas 版）：
//! - `api/controller/common/I18nController`（`@RequestMapping("/api/common/i18n")`）
//! - `api/controller/common/CurrencyController`（`@RequestMapping("/api/common/currency")`）
//!
//! | 本模块路由 | 原 Java 完整路径 | 原 Java 方法 |
//! |-----------|------------------|--------------|
//! | `GET /common/i18n/getLocales` | `GET /api/common/i18n/getLocales` | `I18nController.getLocales` |
//! | `GET /common/i18n/getDefaultLocale` | `GET /api/common/i18n/getDefaultLocale` | `I18nController.getDefaultLocale` |
//! | `GET /common/i18n/getLocaleTranslations` | `GET /api/common/i18n/getLocaleTranslations` | `I18nController.getLocaleTranslations` |
//! | `GET /common/currency/getCurrency` | `GET /api/common/currency/getCurrency` | `CurrencyController.getCurrency` |
//!
//! 4 个端点均为匿名只读（Java `permitAll`），无登录态读取。
//! 本仓库 5.8.57 版 Java 未带该控制器（overseas 版才有），H5/PC 前端
//! Nuxt i18n 初始化与币种切换会调用，缺失时整站 404 报错。

use axum::{
    extract::{Query, State},
    routing::get,
    Router,
};
use serde::Deserialize;
use serde_json::{Map, Value};
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult};

use crate::PublicState;

/// 本域 4 条路由。
const ROUTE_TABLE: [(&str, &str, &str); 4] = [
    (
        "GET",
        "/common/i18n/getLocales",
        "I18nController.getLocales",
    ),
    (
        "GET",
        "/common/i18n/getDefaultLocale",
        "I18nController.getDefaultLocale",
    ),
    (
        "GET",
        "/common/i18n/getLocaleTranslations",
        "I18nController.getLocaleTranslations",
    ),
    (
        "GET",
        "/common/currency/getCurrency",
        "CurrencyController.getCurrency",
    ),
];

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocaleCodeQuery {
    /// 语言地区代码（如 zh-CN）；`getDefaultLocale` 传 `code`，翻译包传 `localeCode`。
    #[serde(alias = "code", alias = "locale_code")]
    pub locale_code: Option<String>,
}

/// 多语言/币种路由（挂载于 `/api` 下）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .route("/common/i18n/getLocales", get(get_locales))
        .route("/common/i18n/getDefaultLocale", get(get_default_locale))
        .route(
            "/common/i18n/getLocaleTranslations",
            get(get_locale_translations),
        )
        .route("/common/currency/getCurrency", get(get_currency))
}

/// 已实现路由（契约完整路径）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/api{path}"), handler))
        .collect()
}

/// `GET /api/common/i18n/getLocales`：启用语言列表（含币种）。
async fn get_locales(
    State(state): State<PublicState>,
) -> ApiResult<Vec<nextrade_infrastructure::db::repo::system::locales::LocalesCurrencyVO>> {
    ok(state.system.locales.locales().await?)
}

/// `GET /api/common/i18n/getDefaultLocale?code=`：默认语言。
async fn get_default_locale(
    State(state): State<PublicState>,
    Query(q): Query<LocaleCodeQuery>,
) -> ApiResult<Value> {
    let vo = state.system.locales.default_locale(q.locale_code.as_deref()).await?;
    match vo {
        Some(v) => ok(serde_json::to_value(v).unwrap_or(Value::Null)),
        None => ok(Value::Null),
    }
}

/// `GET /api/common/i18n/getLocaleTranslations?localeCode=`：页面翻译包。
async fn get_locale_translations(
    State(state): State<PublicState>,
    Query(q): Query<LocaleCodeQuery>,
) -> ApiResult<Value> {
    let code = q.locale_code.unwrap_or_default();
    let pairs = state.system.locales.locale_translations(&code).await?;
    let mut map = Map::new();
    for (k, v) in pairs {
        map.insert(k, Value::String(v));
    }
    ok(Value::Object(map))
}

/// `GET /api/common/currency/getCurrency`：币种列表。
async fn get_currency(
    State(state): State<PublicState>,
) -> ApiResult<Vec<nextrade_infrastructure::db::repo::system::locales::CurrencyVO>> {
    ok(state.system.locales.currencies().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 4);
        assert_eq!(docs[0].path, "/api/common/i18n/getLocales");
        assert_eq!(docs[1].path, "/api/common/i18n/getDefaultLocale");
        assert_eq!(docs[2].path, "/api/common/i18n/getLocaleTranslations");
        assert_eq!(docs[3].path, "/api/common/currency/getCurrency");
    }
}
