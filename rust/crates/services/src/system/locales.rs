//! 多语言/币种服务（system::locales）。
//!
//! 对应原 Java（overseas 版）
//! `api/controller/common/I18nController` + `CurrencyController`
//! 的 4 个只读方法（语言列表/默认语言/语言包/币种列表）。

use std::sync::Arc;

use nextrade_infrastructure::db::repo::system::locales::{
    CurrencyVO, LocalesCurrencyVO, LocalesVO,
};
use nextrade_infrastructure::db::Repositories;
use nextrade_core::Result;

#[derive(Clone)]
pub struct LocalesService {
    repos: Arc<Repositories>,
}

impl LocalesService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 币种列表（Java `CurrencyServiceImpl#getCurrency`）。
    pub async fn currencies(&self) -> Result<Vec<CurrencyVO>> {
        self.repos.system.locales.currencies().await
    }

    /// 启用语言列表（含关联币种）。
    pub async fn locales(&self) -> Result<Vec<LocalesCurrencyVO>> {
        self.repos.system.locales.locales_with_currency().await
    }

    /// 默认语言（浏览器 code 可映射则映射，否则 is_default=1）。
    pub async fn default_locale(&self, code: Option<&str>) -> Result<Option<LocalesVO>> {
        self.repos.system.locales.default_locale(code).await
    }

    /// 页面翻译包（name → value）。
    pub async fn locale_translations(
        &self,
        locale_code: &str,
    ) -> Result<Vec<(String, String)>> {
        self.repos
            .system
            .locales
            .locale_translations(locale_code)
            .await
    }
}
