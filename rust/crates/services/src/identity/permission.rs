//! 管理端方法级权限校验（对应 Java `PermissionHandler.hasPermission`）。
//!
//! - 权限来源：`admin_user.auth_list`（JSON 数组，元素可能是嵌套 JSON 字符串）；
//!   店铺型账号在此基础上并入其店铺绑定解析出的权限（与选店 `choose` 同一
//!   口径：绑定角色存在时以角色 `authority_list` 为准，否则用绑定自带
//!   `auth_list`；仅启用绑定 `is_using = 0` 参与）——历史上商家账号自身
//!   `auth_list` 恒为空、真实权限只在绑定/角色里，只读账号口径会让商家
//!   在任何有权限条目的路由上恒 403。
//! - 语义：**任意一个**所需权限与已授予权限匹配即放行；`all` 为超级权限；
//! - 匹配：Spring `PatternMatchUtils.simpleMatch` 风格（仅 `*` 通配，区分大小写）；
//! - 缓存：按 `(admin_id, 店铺作用域)` 缓存 30 秒，角色/账号变更可调用
//!   [`PermissionService::invalidate`]。
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use nextrade_core::Result;
use nextrade_infrastructure::db::Repositories;

/// 超级权限（Java `PermissionHandler` 里的 `all`）。
pub const SUPER_PERMISSION: &str = "all";
/// 权限缓存有效期。
const CACHE_TTL: Duration = Duration::from_secs(30);

/// 店铺型后台账号类型集合（与 `AuthUser` 的店铺型判定同集）。
#[must_use]
pub(crate) fn is_shop_family_type(admin_type: &str) -> bool {
    matches!(admin_type, "shop" | "store" | "pos" | "merchant")
}

/// 权限服务。
pub struct PermissionService {
    repos: Arc<Repositories>,
    cache: Mutex<HashMap<(i32, Option<i32>), (Vec<String>, Instant)>>,
}

impl PermissionService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self {
            repos,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// 查询管理员权限列表（不区分店铺作用域；店铺型账号并入全部启用
    /// 绑定的权限）。带 30s 进程内缓存。
    ///
    /// # Errors
    /// 管理员不存在或数据库错误。
    pub async fn auth_list(&self, admin_id: i32) -> Result<Vec<String>> {
        self.auth_list_scoped(admin_id, None).await
    }

    /// 查询管理员在指定店铺作用域下的权限列表（带 30s 进程内缓存）。
    ///
    /// 店铺型账号：账号自身 `auth_list` ∪ 当前作用域绑定的有效权限
    /// （`shop_scope` 为 None 时并入全部启用绑定；指定店铺时只并入该店
    /// 绑定）。平台 / 供应商账号不查绑定，与历史口径一致。
    ///
    /// # Errors
    /// 管理员不存在或数据库错误。
    pub async fn auth_list_scoped(
        &self,
        admin_id: i32,
        shop_scope: Option<i32>,
    ) -> Result<Vec<String>> {
        let scope = shop_scope.filter(|id| *id > 0);
        let key = (admin_id, scope);
        if let Ok(cache) = self.cache.lock() {
            if let Some((list, fetched)) = cache.get(&key) {
                if fetched.elapsed() < CACHE_TTL {
                    return Ok(list.clone());
                }
            }
        }
        let user = self
            .repos
            .identity
            .auth
            .find_by_id(admin_id)
            .await?
            .ok_or_else(|| nextrade_core::Error::NotFound("管理员不存在".into()))?;
        let mut list = parse_auth_list(&user.auth_list);
        if is_shop_family_type(&user.admin_type) {
            let bindings = self
                .repos
                .merchant
                .shop_manage
                .find_admin_bindings(admin_id)
                .await?;
            for binding in bindings {
                if binding.is_using != 0 {
                    continue;
                }
                if let Some(scope) = scope {
                    if binding.shop_id != scope {
                        continue;
                    }
                }
                let role_authority = if binding.role_id > 0 {
                    self.repos
                        .merchant
                        .shop_manage
                        .find_admin_role(binding.role_id)
                        .await?
                        .map(|role| role.authority_list)
                } else {
                    None
                };
                merge_grants(
                    &mut list,
                    binding_effective_list(binding.auth_list.as_deref(), role_authority.as_deref()),
                );
            }
        }
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(key, (list.clone(), Instant::now()));
        }
        Ok(list)
    }

    /// 判断权限集合是否满足任意一个所需权限。
    #[must_use]
    pub fn matches(granted: &[String], required: &[&str]) -> bool {
        if granted.iter().any(|g| g == SUPER_PERMISSION) {
            return true;
        }
        required.iter().any(|req| {
            granted
                .iter()
                .filter(|g| !g.trim().is_empty())
                .any(|g| simple_match(req, g))
        })
    }

    /// 校验管理员是否拥有任意一个所需权限。
    ///
    /// # Errors
    /// 权限读取失败。
    pub async fn check(&self, admin_id: i32, required: &[&str]) -> Result<bool> {
        self.check_scoped(admin_id, None, required).await
    }

    /// 校验管理员在指定店铺作用域下是否拥有任意一个所需权限。
    ///
    /// # Errors
    /// 权限读取失败。
    pub async fn check_scoped(
        &self,
        admin_id: i32,
        shop_scope: Option<i32>,
        required: &[&str],
    ) -> Result<bool> {
        let granted = self.auth_list_scoped(admin_id, shop_scope).await?;
        Ok(Self::matches(&granted, required))
    }

    /// 使指定管理员权限缓存失效（角色/账号变更后调用；全部作用域一并失效）。
    pub fn invalidate(&self, admin_id: i32) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.retain(|(id, _), _| *id != admin_id);
        }
    }
}

/// 单个店铺绑定的有效权限（与选店 `choose` 的解析口径一致）：
/// 绑定角色存在时以角色 `authority_list` 为准，否则用绑定自带 `auth_list`。
#[must_use]
pub fn binding_effective_list(
    binding_auth_list: Option<&str>,
    role_authority_list: Option<&str>,
) -> Vec<String> {
    if let Some(authority) = role_authority_list {
        return parse_auth_list(authority);
    }
    parse_auth_list(binding_auth_list.unwrap_or_default())
}

/// 把 `extra` 并入 `base`（去重、保序）。
fn merge_grants(base: &mut Vec<String>, extra: Vec<String>) {
    for item in extra {
        if !base.contains(&item) {
            base.push(item);
        }
    }
}

/// 解析 `auth_list`：JSON 数组（元素可能是嵌套 JSON 字符串），
/// 非法 JSON 时退化为逗号分隔。
#[must_use]
pub fn parse_auth_list(raw: &str) -> Vec<String> {
    fn collect(value: &serde_json::Value, out: &mut Vec<String>) {
        match value {
            serde_json::Value::String(s) => {
                let trimmed = s.trim();
                if trimmed.starts_with('[') {
                    if let Ok(inner) = serde_json::from_str::<serde_json::Value>(trimmed) {
                        collect(&inner, out);
                        return;
                    }
                }
                if !trimmed.is_empty() {
                    out.push(trimmed.to_string());
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    collect(item, out);
                }
            }
            _ => {}
        }
    }

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<serde_json::Value>(trimmed) {
        Ok(value) => {
            let mut out = Vec::new();
            collect(&value, &mut out);
            out
        }
        Err(_) => trimmed
            .split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
            .collect(),
    }
}

/// Spring `PatternMatchUtils.simpleMatch(pattern, text)`：`*` 匹配任意长度。
#[must_use]
pub fn simple_match(pattern: &str, text: &str) -> bool {
    if !pattern.contains('*') {
        return pattern == text;
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == text;
    }
    // 首个片段必须前缀匹配
    if !text.starts_with(parts[0]) {
        return false;
    }
    let mut pos = parts[0].len();
    // 中间片段顺序查找
    for part in &parts[1..parts.len() - 1] {
        if part.is_empty() {
            continue;
        }
        match text[pos..].find(part) {
            Some(idx) => pos += idx + part.len(),
            None => return false,
        }
    }
    // 末尾片段必须后缀匹配（末段为空表示可任意结尾）
    let last = parts[parts.len() - 1];
    if last.is_empty() {
        return true;
    }
    text.len() >= pos + last.len() && text.ends_with(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_all_and_plain_list() {
        assert_eq!(parse_auth_list(r#"["all"]"#), vec!["all".to_string()]);
        assert_eq!(
            parse_auth_list(r#"["orderManage","productManage"]"#),
            vec!["orderManage".to_string(), "productManage".to_string()]
        );
    }

    #[test]
    fn parse_nested_json_strings() {
        // 真实脏数据：数组元素本身是 JSON 字符串
        let raw = r#"["[\"all\"]"]"#;
        assert_eq!(parse_auth_list(raw), vec!["all".to_string()]);
    }

    #[test]
    fn parse_fallback_comma_separated() {
        assert_eq!(
            parse_auth_list("orderManage, productManage"),
            vec!["orderManage".to_string(), "productManage".to_string()]
        );
    }

    #[test]
    fn matches_semantics() {
        let granted = vec!["orderManage".to_string()];
        assert!(PermissionService::matches(&granted, &["orderManage", "productManage"]));
        assert!(!PermissionService::matches(&granted, &["productManage"]));
        // 超级权限
        let all = vec!["all".to_string()];
        assert!(PermissionService::matches(&all, &["anything"]));
        // 无所需权限 → 不通过
        assert!(!PermissionService::matches(&granted, &[]));
    }

    #[test]
    fn binding_effective_list_prefers_role_authority() {
        // 角色存在 → 以角色 authority_list 为准（与 choose 同口径）
        assert_eq!(
            binding_effective_list(
                Some(r#"["bindingOnly"]"#),
                Some(r#"["roleA","roleB"]"#),
            ),
            vec!["roleA".to_string(), "roleB".to_string()]
        );
        // 无角色 → 回退绑定自带 auth_list；绑定无 auth_list → 空
        assert_eq!(
            binding_effective_list(Some(r#"["bindingOnly"]"#), None),
            vec!["bindingOnly".to_string()]
        );
        assert!(binding_effective_list(None, None).is_empty());
    }

    #[test]
    fn merge_grants_dedups_and_preserves_order() {
        let mut base = vec!["a".to_string(), "b".to_string()];
        merge_grants(&mut base, vec!["b".to_string(), "c".to_string()]);
        assert_eq!(base, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }

    #[test]
    fn simple_match_wildcards() {
        assert!(simple_match("order*", "orderManage"));
        assert!(simple_match("*Manage", "orderManage"));
        assert!(simple_match("*der*", "orderManage"));
        assert!(!simple_match("order*", "productManage"));
        assert!(simple_match("exact", "exact"));
        assert!(!simple_match("exact", "exactly"));
    }
}
