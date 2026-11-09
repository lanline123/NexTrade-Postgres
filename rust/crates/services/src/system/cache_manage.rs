//! 配置管理 域服务（system::cache_manage）：清除缓存。
//!
//! 对应原 Java `CacheManageController.cleanup` →
//! `CacheManageServiceImpl#cleanup`（`nextrade-service` 模块，共 50 行）：
//!
//! ```java
//! private static final List<String> NO_CLEANUP_TAGS =
//!         List.of(Constants.ADMIN_TOKEN, Constants.ADMIN_USER);   // admin_token:: / admin_user::
//!
//! public void cleanup(String tag) {
//!     Set<String> allKeys = redisTemplate.keys("*");
//!     if (CollUtil.isNotEmpty(allKeys)) {
//!         Set<String> keysToDelete = allKeys.stream()
//!                 .filter(key -> NO_CLEANUP_TAGS.stream().noneMatch(key::startsWith))
//!                 .collect(Collectors.toSet());
//!         if (CollUtil.isNotEmpty(keysToDelete)) redisTemplate.delete(keysToDelete);
//!     }
//! }
//! ```
//!
//! 迁移要点：
//! - **保留前缀**逐字符照抄 Java `Constants.ADMIN_TOKEN`（`admin_token::`）
//!   与 `Constants.ADMIN_USER`（`admin_user::`），见 [`NO_CLEANUP_PREFIXES`]。
//! - 过滤语义是 `String.startsWith`（**前缀**匹配，且**大小写敏感**），
//!   不是 `contains`，见 [`partition_keys`]。
//! - Java 用 `redisTemplate.keys("*")`，Rust 用 `SCAN` 游标（见仓储层说明）。
//! - 入参 `tag` 在原 Java 中**被完全忽略**，本实现同样忽略（仅记日志/回显）。
//! - ⚠️ 原 Java 的保留清单**不含** `Constants.ADMIN_API_TOKEN`
//!   （`admin_api_token::`），即该前缀的 key **会被删除**；本实现如实复刻
//!   （见 `uncertain_points`）。

use std::sync::Arc;

use nextrade_core::models::system::cache_manage::{CacheCleanupQuery, CacheCleanupVO};
use nextrade_core::Result;
use nextrade_infrastructure::db::Repositories;

/// Java `CacheManageServiceImpl#NO_CLEANUP_TAGS`
/// （`Constants.ADMIN_TOKEN` + `Constants.ADMIN_USER`）。
///
/// 命中这些**前缀**的 key 会被保留（通常用于保住当前管理员的登录会话，
/// 避免清理缓存把自己踢下线）。
pub const NO_CLEANUP_PREFIXES: [&str; 2] = ["admin_token::", "admin_user::"];

/// 配置管理 域服务。
pub struct CacheManageService {
    repos: Arc<Repositories>,
}

impl CacheManageService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &nextrade_infrastructure::db::repo::system::cache_manage::CacheManageRepository {
        &self.repos.system.cache_manage
    }

    /// 清除缓存。
    ///
    /// 对应 Java `CacheManageController.cleanup`
    /// （`@RequestParam(value = "tag", required = false) String tag`）
    /// → `CacheManageServiceImpl#cleanup(String tag)`。
    ///
    /// 流程：`SCAN` 全量 key → 剔除保留前缀 → 批量 `DEL`。
    ///
    /// # Errors
    /// Redis 不可用 / 配置加载失败 / 命令执行失败 ⇒ [`nextrade_core::Error::Cache`]。
    /// 不会「假装成功」：Redis 异常时直接向上抛出。
    pub async fn cleanup(&self, q: &CacheCleanupQuery) -> Result<CacheCleanupVO> {
        // 原 Java 忽略 tag，这里同样不参与过滤，仅记录以便排查。
        if let Some(tag) = q.tag.as_deref() {
            tracing::debug!("缓存清理请求：tag={tag}（原 Java 忽略该参数，本实现同样忽略）");
        }

        let all_keys = self.repos.system.cache_manage.scan_all_keys().await?;
        let scanned = all_keys.len() as u64;
        let (keys_to_delete, kept) = partition_keys(all_keys, &NO_CLEANUP_PREFIXES);
        let deleted = self.repos.system.cache_manage.delete_keys(&keys_to_delete).await?;

        tracing::info!("缓存清理完成：扫描 {scanned} 个 key，删除 {deleted} 个，保留 {kept} 个");
        Ok(CacheCleanupVO {
            scanned,
            deleted,
            kept,
            tag: q.tag.clone(),
        })
    }
}

/// 纯逻辑：按**前缀**把 key 分成「应删除」与「保留」两组。
///
/// 逐字符复刻 Java：
/// `keysToDelete = allKeys.filter(key -> NO_CLEANUP_TAGS.noneMatch(key::startsWith))`。
///
/// 返回 `(应删除的 key, 保留的 key 数量)`。
///
/// 语义细节（不要凭感觉简化）：
/// - 前缀匹配用 [`str::starts_with`]，**大小写敏感**；
/// - 只做前缀匹配，`xxx_admin_token::1` 这类「包含但不以之开头」的 key
///   会被删除；
/// - `keep_prefixes` 为空 ⇒ 全部删除。
#[must_use]
pub fn partition_keys(keys: Vec<String>, keep_prefixes: &[&str]) -> (Vec<String>, u64) {
    let mut deletable = Vec::with_capacity(keys.len());
    let mut kept: u64 = 0;
    for key in keys {
        if keep_prefixes.iter().any(|p| key.starts_with(*p)) {
            kept += 1;
        } else {
            deletable.push(key);
        }
    }
    (deletable, kept)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn keeps_admin_token_and_admin_user_prefixes() {
        let (del, kept) = partition_keys(
            keys(&[
                "admin_token::admin:abc",
                "admin_user::admin",
                "product:list:1",
                "cat_floor",
            ]),
            &NO_CLEANUP_PREFIXES,
        );
        assert_eq!(kept, 2);
        assert_eq!(del, keys(&["product:list:1", "cat_floor"]));
    }

    #[test]
    fn admin_api_token_is_not_protected() {
        // Java `NO_CLEANUP_TAGS` 不含 ADMIN_API_TOKEN ⇒ 该前缀会被删除。
        let (del, kept) = partition_keys(
            keys(&["admin_api_token::k", "admin_token::k"]),
            &NO_CLEANUP_PREFIXES,
        );
        assert_eq!(kept, 1);
        assert_eq!(del, keys(&["admin_api_token::k"]));
    }

    #[test]
    fn prefix_match_is_not_substring_match() {
        // `startsWith` 语义：仅在开头命中才算保留。
        let (del, kept) = partition_keys(
            keys(&["xadmin_token::k", "admin_token::k"]),
            &NO_CLEANUP_PREFIXES,
        );
        assert_eq!(kept, 1);
        assert_eq!(del, keys(&["xadmin_token::k"]));
    }

    #[test]
    fn prefix_match_is_case_sensitive() {
        // Java `startsWith` 大小写敏感。
        let (del, kept) = partition_keys(
            keys(&["ADMIN_TOKEN::k", "admin_token::k"]),
            &NO_CLEANUP_PREFIXES,
        );
        assert_eq!(kept, 1);
        assert_eq!(del, keys(&["ADMIN_TOKEN::k"]));
    }

    #[test]
    fn empty_keep_prefixes_deletes_everything() {
        let (del, kept) = partition_keys(keys(&["a", "b"]), &[]);
        assert_eq!(kept, 0);
        assert_eq!(del, keys(&["a", "b"]));
    }

    #[test]
    fn empty_input_yields_nothing() {
        let (del, kept) = partition_keys(Vec::new(), &NO_CLEANUP_PREFIXES);
        assert!(del.is_empty());
        assert_eq!(kept, 0);
    }

    #[test]
    fn exact_prefix_key_is_kept() {
        // 前缀本身就是完整 key 时同样命中 `startsWith`。
        let (del, kept) = partition_keys(keys(&["admin_user::"]), &NO_CLEANUP_PREFIXES);
        assert!(del.is_empty());
        assert_eq!(kept, 1);
    }
}