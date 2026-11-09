//! 管理端方法级权限校验（对应 Java `PermissionHandler.hasPermission`）。
//!
//! - 权限来源：`admin_user.auth_list`（JSON 数组，元素可能是嵌套 JSON 字符串）；
//! - 语义：**任意一个**所需权限与已授予权限匹配即放行；`all` 为超级权限；
//! - 匹配：Spring `PatternMatchUtils.simpleMatch` 风格（仅 `*` 通配，区分大小写）；
//! - 缓存：按 `admin_id` 缓存 30 秒，角色/账号变更可调用 [`PermissionService::invalidate`]。
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use nextrade_core::Result;
use nextrade_infrastructure::db::Repositories;

/// 超级权限（Java `PermissionHandler` 里的 `all`）。
pub const SUPER_PERMISSION: &str = "all";
/// 权限缓存有效期。
const CACHE_TTL: Duration = Duration::from_secs(30);

/// 权限服务。
pub struct PermissionService {
    repos: Arc<Repositories>,
    cache: Mutex<HashMap<i32, (Vec<String>, Instant)>>,
}

impl PermissionService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self {
            repos,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// 查询管理员权限列表（带 30s 进程内缓存）。
    ///
    /// # Errors
    /// 管理员不存在或数据库错误。
    pub async fn auth_list(&self, admin_id: i32) -> Result<Vec<String>> {
        if let Ok(cache) = self.cache.lock() {
            if let Some((list, fetched)) = cache.get(&admin_id) {
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
        let list = parse_auth_list(&user.auth_list);
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(admin_id, (list.clone(), Instant::now()));
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
        let granted = self.auth_list(admin_id).await?;
        Ok(Self::matches(&granted, required))
    }

    /// 使指定管理员权限缓存失效（角色/账号变更后调用）。
    pub fn invalidate(&self, admin_id: i32) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.remove(&admin_id);
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
    fn simple_match_wildcards() {
        assert!(simple_match("order*", "orderManage"));
        assert!(simple_match("*Manage", "orderManage"));
        assert!(simple_match("*der*", "orderManage"));
        assert!(!simple_match("order*", "productManage"));
        assert!(simple_match("exact", "exact"));
        assert!(!simple_match("exact", "exactly"));
    }
}
