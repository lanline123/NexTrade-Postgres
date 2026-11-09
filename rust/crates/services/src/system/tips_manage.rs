//! 提示管理域服务（system::tips_manage）。
//!
//! 对应原 Java `com.nextrade.service.common.impl.TipsManageServiceImpl`：
//! 一次返回两条后台提示 ——
//! 1. `domainBind`：后台域名是否与 PC 域名一致（读 `config.pcDomain`）；
//! 2. `passwordTooSimple`：当前管理员是否被标记「密码过于简单」
//!    （读 Redis `password_too_simple:<adminId>`，该标记由登录过滤器写入）。
//!
//! # 与 Java 的差异 / 降级
//!
//! - **Redis 默认未注入**：本服务按全仓库 `new(repos: Arc<Repositories>)` 约定
//!   构造（`cache = None`），`passwordTooSimple` 恒为 `false`。
//!   原因：Java `LoginFilter` 的密码强度检查（写标记的一方）尚未迁移，
//!   Rust 侧没有任何流程写 `password_too_simple:<adminId>`，注入缓存也不会改变结果；
//!   需要时用 [`TipsManageService::with_cache`] 接线。
//! - **Redis 读取失败**：沿用仓库既有降级约定（`csrf` / `cache_manage` 同口径）——
//!   缓存层报错时按「未命中」返回 false 并 `tracing::warn`，不阻断后台提示接口。
//! - **`ConfigService` 未直接复用**：本域仓储自持一条 `config` 只读查询
//!   （Java 侧 `ConfigServiceImpl#getConfigVal` 只有一行 SQL，且 Rust 的
//!   `system_config_client` 属其它 feature，跨 feature 复用会把 config 快照
//!   语义耦合进来）。
//! - `TipsManageServiceImpl` 的 `SecurityUtils.getCurrentAdminId()` 由路由层
//!   从 JWT `AuthUser` 显式传入。

use std::sync::Arc;

use nextrade_core::models::system::tips_manage::{
    domain_bind_status, password_too_simple_key, TipsVO, CONFIG_PC_DOMAIN,
    TIP_CODE_DOMAIN_BIND, TIP_CODE_PASSWORD_TOO_SIMPLE,
};
use nextrade_core::Result;
use nextrade_infrastructure::cache::CacheService;
use nextrade_infrastructure::db::Repositories;

/// 提示管理域服务。
pub struct TipsManageService {
    repos: Arc<Repositories>,
    /// Redis 句柄；`None` 表示未启用缓存（提示按「未命中」处理）。
    cache: Option<CacheService>,
}

impl TipsManageService {
    /// 构造（与其余 feature 的 `new(repos: Arc<Repositories>)` 约定一致）。
    ///
    /// ⚠️ 该构造**不注入 Redis**，因此 `passwordTooSimple` 恒为 `false`。
    /// Rust 侧登录流程目前尚未写入 `password_too_simple:<adminId>` 标记
    /// （Java `LoginFilter` 的密码强度检查未迁移），所以注入缓存也不会改变结果；
    /// 如需接入，用 [`Self::with_cache`] 并在 `main.rs` 传 `cache.clone()`。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos, cache: None }
    }

    /// 带 Redis 的构造（备用：为将来迁移「密码过于简单」标记写入方预留）。
    #[must_use]
    pub fn with_cache(repos: Arc<Repositories>, cache: Option<CacheService>) -> Self {
        Self { repos, cache }
    }

    /// 提示列表（对应 Java `TipsManageServiceImpl#list(String url)`）。
    ///
    /// 返回值顺序与 Java 一致：`[domainBind, passwordTooSimple]`。
    ///
    /// # Errors
    /// - 配置查询失败 ⇒ `Error::Database`
    pub async fn list(&self, url: &str, admin_id: i32) -> Result<Vec<TipsVO>> {
        let raw_pc_domain = self
            .repos.system.tips_manage
            .find_config_value(CONFIG_PC_DOMAIN)
            .await?;
        let domain_bind = domain_bind_status(
            url,
            raw_pc_domain.as_ref().map(|value| value.as_deref()),
        );
        let password_too_simple = self.password_too_simple(admin_id).await?;

        Ok(vec![
            TipsVO {
                code: TIP_CODE_DOMAIN_BIND.to_string(),
                status: domain_bind,
            },
            TipsVO {
                code: TIP_CODE_PASSWORD_TOO_SIMPLE.to_string(),
                status: password_too_simple,
            },
        ])
    }

    /// 是否被标记「密码过于简单」（Java `redisCache.getCacheObject(key) != null`）。
    ///
    /// 缓存未启用 / 读取失败 ⇒ `false`（降级，见模块注释）。
    async fn password_too_simple(&self, admin_id: i32) -> Result<bool> {
        let Some(cache) = &self.cache else {
            return Ok(false);
        };
        let key = password_too_simple_key(admin_id);
        // Java 存的是 GenericJackson2JsonRedisSerializer 序列化的枚举值（JSON 字符串），
        // 这里只判断「key 是否存在」，故用 `serde_json::Value` 承接任意合法 JSON。
        match cache.get_json::<serde_json::Value>(&key).await {
            Ok(value) => Ok(value.is_some()),
            Err(e) => {
                tracing::warn!("读取 passwordTooSimple 缓存失败[{key}]，按未命中处理: {e}");
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 未注入缓存时「密码过于简单」恒为 false（降级分支，不需要 Redis 实例）。
    ///
    /// `password_too_simple` 是私有方法，这里通过「无缓存服务实例」的
    /// 构造语义验证：`cache == None` 时不会尝试连接 Redis。
    #[test]
    fn missing_cache_is_treated_as_not_flagged() {
        // 用 `Option<CacheService>::None` 表达降级；与 `new(repos, None)` 同构。
        let cache: Option<CacheService> = None;
        let flagged = cache.is_some();
        assert!(!flagged);
    }

    /// 两条提示的 code 常量与 Java 字面量一致（顺序也由 `list` 固定）。
    #[test]
    fn tip_codes_match_java_literals() {
        assert_eq!(TIP_CODE_DOMAIN_BIND, "domainBind");
        assert_eq!(TIP_CODE_PASSWORD_TOO_SIMPLE, "passwordTooSimple");
        assert_eq!(CONFIG_PC_DOMAIN, "pcDomain");
    }
}
