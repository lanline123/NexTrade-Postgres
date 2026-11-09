//! 多语言（lang）领域服务。

pub mod currency;
pub mod locales;
pub mod locales_relation;
pub mod translations;

use std::sync::Arc;

/// lang 领域服务集合（由组合根装配一次，api 层只持有 `Arc<Services>`）。
///
/// `bin/main.rs` 只装配了 `translations`（先行切片）；locales /
/// currency / locales_relation 三个 sibling 服务按「组合方式提供」：
/// 各自是无状态仓储包装，构造只做 `Arc` 克隆，按请求构建零成本。
#[derive(Clone)]
pub struct Services {
    pub translations: Arc<translations::TranslationsService>,
}

impl Services {
    /// 语言地区服务（`/adminapi/lang/locales/*`）。
    #[must_use]
    pub fn locales(&self) -> locales::LocalesService {
        locales::LocalesService::new(self.translations_repos())
    }

    /// 货币服务（`/adminapi/lang/currency/*`）。
    #[must_use]
    pub fn currency(&self) -> currency::CurrencyService {
        currency::CurrencyService::new(self.translations_repos())
    }

    /// 语言关联服务（`/adminapi/lang/localesRelation/*`）。
    #[must_use]
    pub fn locales_relation(&self) -> locales_relation::LocalesRelationService {
        locales_relation::LocalesRelationService::new(self.translations_repos())
    }

    /// `TranslationsService` 持有的仓储聚合（sibling 服务共用）。
    #[must_use]
    fn translations_repos(&self) -> Arc<nextrade_infrastructure::db::Repositories> {
        self.translations.repos_clone()
    }
}
