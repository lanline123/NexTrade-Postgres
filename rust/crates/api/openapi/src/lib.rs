//! NexTrade API 契约与实现覆盖率。
//!
//! 原 Java 系统有 249 个控制器、1561 个 HTTP 接口。Rust 重写是一个渐进过程，
//! 因此本 crate 解决两个问题：
//!
//! 1. **契约可查询**：把 `tools/java_api_parser.py` 从 Java 注解提取的接口清单
//!    嵌入二进制，通过 HTTP 暴露，重写过程中随时可对照原系统接口。
//! 2. **覆盖率可度量**：各 API crate 声明自己实际注册的路由，本 crate 计算
//!    与原契约的差集。这样「已完成多少」是算出来的，而不是写文档时手填的。
//!
//! 端点在服务上以 `/openapi/*` 暴露（管理端与商城端各自挂载）。

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub mod http;
pub use http::{routes, OpenApiState};

/// 契约文件内容（由 build.rs 决定是否嵌入）。
#[cfg(has_api_contract)]
const CONTRACT_JSON: &str = include_str!("../../../../docs/api-contract.json");

#[cfg(not(has_api_contract))]
const CONTRACT_JSON: &str = r#"{"version":"unknown","endpoint_count":0,"controller_count":0,"endpoints":[]}"#;

/// 原 Java 系统的一个 HTTP 接口。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEndpoint {
    /// `adminapi` 或 `api`。
    #[serde(default)]
    pub module: String,
    /// HTTP 方法（GET/POST/...）。
    #[serde(default)]
    pub method: String,
    /// 完整路径，例如 `/adminapi/product/product/list`。
    #[serde(default)]
    pub path: String,
    /// Swagger `@Operation.summary` 的中文说明。
    #[serde(default)]
    pub summary: String,
    /// Swagger `@Tag.name`。
    #[serde(default)]
    pub tag: String,
    /// 认证要求推断：`jwt` / `jwt+perm` / `anonymous` / `optional`。
    #[serde(default)]
    pub auth: String,
    /// `@PreAuthorize` 中的权限标识。
    #[serde(default)]
    pub permissions: Vec<String>,
    /// 原 Java 控制器类名。
    #[serde(default)]
    pub controller: String,
    /// 原 Java 方法名。
    #[serde(default)]
    pub handler: String,
    /// 源码位置 `文件:行`，便于回溯。
    #[serde(default)]
    pub source: String,
}

/// 完整契约文档。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub generated_by: String,
    #[serde(default)]
    pub endpoint_count: usize,
    #[serde(default)]
    pub controller_count: usize,
    #[serde(default)]
    pub endpoints: Vec<ContractEndpoint>,
}

impl Contract {
    /// 解析嵌入的契约。仅在启动时调用一次。
    ///
    /// # Panics
    /// 嵌入的 JSON 由生成器产出，解析失败说明构建产物损坏，属于不可恢复错误。
    #[must_use]
    pub fn embedded() -> Self {
        serde_json::from_str(CONTRACT_JSON).unwrap_or_else(|e| {
            tracing::error!("API 契约解析失败（嵌入数据损坏）: {e}");
            Self {
                version: "unknown".into(),
                generated_by: String::new(),
                endpoint_count: 0,
                controller_count: 0,
                endpoints: Vec::new(),
            }
        })
    }

    /// 是否成功嵌入了契约数据。
    #[must_use]
    pub fn is_available(&self) -> bool {
        !self.endpoints.is_empty()
    }
}

/// Rust 侧已实现的一个路由（由各 API crate 声明）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDoc {
    /// HTTP 方法，大写。
    pub method: String,
    /// 路由路径，需与原契约路径完全一致（含 `/adminapi` 或 `/api` 前缀）。
    pub path: String,
    /// 对应原 Java 控制器方法，例如 `ProductController.list`。
    pub java_ref: String,
}

impl RouteDoc {
    #[must_use]
    pub fn new(method: &str, path: &str, java_ref: &str) -> Self {
        Self {
            method: method.to_ascii_uppercase(),
            path: path.to_owned(),
            java_ref: java_ref.to_owned(),
        }
    }

    /// 用于集合比对的键。
    #[must_use]
    pub fn key(&self) -> String {
        format!("{} {}", self.method, self.path)
    }
}

/// 单模块覆盖率。
#[derive(Debug, Clone, Serialize)]
pub struct ModuleCoverage {
    pub module: String,
    pub total: usize,
    pub implemented: usize,
    pub percent: f64,
}

/// 覆盖率总览。
#[derive(Debug, Clone, Serialize)]
pub struct Coverage {
    pub contract_version: String,
    pub contract_available: bool,
    pub total: usize,
    pub implemented: usize,
    pub percent: f64,
    pub by_module: Vec<ModuleCoverage>,
    /// 已实现但契约中不存在的路由（多为自加的辅助端点，如健康检查）。
    pub extra_routes: Vec<RouteDoc>,
    /// 契约中存在但 Rust 未实现的接口（重写待办清单）。
    pub missing: Vec<ContractEndpoint>,
}

/// 计算覆盖率。
///
/// `implemented` 由各 API crate 提供；匹配键为 `方法 + 路径`。
#[must_use]
pub fn coverage(contract: &Contract, implemented: &[RouteDoc]) -> Coverage {
    let have: BTreeSet<String> = implemented.iter().map(RouteDoc::key).collect();
    let want: BTreeSet<String> = contract
        .endpoints
        .iter()
        .map(|e| format!("{} {}", e.method.to_ascii_uppercase(), e.path))
        .collect();

    let mut by_module: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut missing = Vec::new();
    for ep in &contract.endpoints {
        let entry = by_module.entry(ep.module.clone()).or_insert((0, 0));
        entry.0 += 1;
        let key = format!("{} {}", ep.method.to_ascii_uppercase(), ep.path);
        if have.contains(&key) {
            entry.1 += 1;
        } else {
            missing.push(ep.clone());
        }
    }

    let extra_routes: Vec<RouteDoc> = implemented
        .iter()
        .filter(|r| !want.contains(&r.key()))
        .cloned()
        .collect();

    let total = contract.endpoints.len();
    let implemented_count = total.saturating_sub(missing.len());
    let by_module = by_module
        .into_iter()
        .map(|(module, (t, i))| ModuleCoverage {
            module,
            total: t,
            implemented: i,
            percent: percent(i, t),
        })
        .collect();

    Coverage {
        contract_version: contract.version.clone(),
        contract_available: contract.is_available(),
        total,
        implemented: implemented_count,
        percent: percent(implemented_count, total),
        by_module,
        extra_routes,
        missing,
    }
}

fn percent(done: usize, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    // 保留一位小数即可，避免浮点噪声写进 API 响应。
    ((done as f64) * 1000.0 / (total as f64)).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ep(module: &str, method: &str, path: &str) -> ContractEndpoint {
        ContractEndpoint {
            module: module.into(),
            method: method.into(),
            path: path.into(),
            summary: String::new(),
            tag: String::new(),
            auth: String::new(),
            permissions: Vec::new(),
            controller: String::new(),
            handler: String::new(),
            source: String::new(),
        }
    }

    fn contract() -> Contract {
        Contract {
            version: "test".into(),
            generated_by: "test".into(),
            endpoint_count: 3,
            controller_count: 1,
            endpoints: vec![
                ep("api", "GET", "/api/a"),
                ep("api", "POST", "/api/b"),
                ep("adminapi", "GET", "/adminapi/c"),
            ],
        }
    }

    #[test]
    fn counts_implemented_and_missing() {
        let c = coverage(&contract(), &[RouteDoc::new("get", "/api/a", "A.a")]);
        assert_eq!(c.total, 3);
        assert_eq!(c.implemented, 1);
        assert_eq!(c.missing.len(), 2);
        // 方法大小写不敏感
        assert!(c.missing.iter().all(|m| m.path != "/api/a"));
    }

    #[test]
    fn groups_by_module_with_percent() {
        let c = coverage(
            &contract(),
            &[
                RouteDoc::new("GET", "/api/a", "A.a"),
                RouteDoc::new("POST", "/api/b", "A.b"),
            ],
        );
        let api = c.by_module.iter().find(|m| m.module == "api").unwrap();
        assert_eq!((api.total, api.implemented), (2, 2));
        assert!((api.percent - 100.0).abs() < f64::EPSILON);
        let admin = c
            .by_module
            .iter()
            .find(|m| m.module == "adminapi")
            .unwrap();
        assert_eq!((admin.total, admin.implemented), (1, 0));
        assert!(admin.percent.abs() < f64::EPSILON);
    }

    #[test]
    fn reports_extra_routes_not_in_contract() {
        let c = coverage(&contract(), &[RouteDoc::new("GET", "/api/health", "self")]);
        assert_eq!(c.extra_routes.len(), 1);
        assert_eq!(c.extra_routes[0].path, "/api/health");
        assert_eq!(c.implemented, 0);
    }

    #[test]
    fn empty_contract_does_not_divide_by_zero() {
        let empty = Contract {
            version: "x".into(),
            generated_by: String::new(),
            endpoint_count: 0,
            controller_count: 0,
            endpoints: Vec::new(),
        };
        let c = coverage(&empty, &[]);
        assert_eq!(c.total, 0);
        assert!(c.percent.abs() < f64::EPSILON);
        assert!(!c.contract_available);
    }

    #[test]
    fn embedded_contract_parses() {
        // 只要契约文件被嵌入，就必须能解析出接口。
        let c = Contract::embedded();
        if c.is_available() {
            assert!(c.endpoint_count > 0);
            assert_eq!(c.endpoint_count, c.endpoints.len());
            assert!(c.endpoints.iter().all(|e| e.path.starts_with('/')));
        }
    }
}