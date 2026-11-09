//! `/openapi/*` 路由：契约查询与覆盖率。
//!
//! 这些端点用于重写期间的对照与验收，不属于原 Java 契约本身，因此在覆盖率
//! 报告中会被列入 `extra_routes`。
//!
//! | 路由 | 用途 |
//! |------|------|
//! | `GET /openapi/contract` | 原始 Java 接口清单（可按 module/tag/关键词过滤） |
//! | `GET /openapi/coverage` | Rust 实现覆盖率与待办差集 |
//! | `GET /openapi/missing` | 仅返回未实现接口，可直接当开发任务清单 |
//! | `GET /openapi/summary` | 汇总计数（便于监控与 CI 断言） |

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    coverage, Contract, ContractEndpoint, Coverage, ModuleCoverage, RouteDoc,
};

/// 文档端点所需的共享状态。
#[derive(Clone)]
pub struct OpenApiState {
    /// 原 Java 契约。
    pub contract: std::sync::Arc<Contract>,
    /// 本进程实际注册的路由（由各 API crate 声明）。
    pub routes: std::sync::Arc<Vec<RouteDoc>>,
}

impl OpenApiState {
    /// 用契约与路由清单构建状态。
    #[must_use]
    pub fn new(contract: Contract, routes: Vec<RouteDoc>) -> Self {
        Self {
            contract: std::sync::Arc::new(contract),
            routes: std::sync::Arc::new(routes),
        }
    }
}

/// 构建 `/openapi` 子路由。
///
/// 调用方以 `.nest("/openapi", openapi::routes(state))` 挂载。
pub fn routes(state: OpenApiState) -> Router {
    Router::new()
        .route("/contract", get(contract_handler))
        .route("/coverage", get(coverage_handler))
        .route("/missing", get(missing_handler))
        .route("/summary", get(summary_handler))
        .with_state(state)
}

/// `GET /openapi/contract` 查询参数。
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ContractQuery {
    /// 按模块过滤：`adminapi` / `api`。
    pub module: Option<String>,
    /// 按 Swagger tag 过滤（精确匹配）。
    pub tag: Option<String>,
    /// 关键词，匹配路径、说明或控制器名。
    pub keyword: Option<String>,
    /// 1 起的页码。
    pub page: Option<usize>,
    /// 每页条数，默认 50，上限 500。
    pub page_size: Option<usize>,
}

/// 契约查询响应（分页）。
#[derive(Debug, Serialize)]
pub struct ContractPage {
    pub version: String,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub list: Vec<ContractEndpoint>,
}

/// `GET /openapi/contract`：分页查询原 Java 接口清单。
async fn contract_handler(
    State(state): State<OpenApiState>,
    Query(q): Query<ContractQuery>,
) -> Json<ContractPage> {
    let kw = q.keyword.as_deref().map(str::to_lowercase);
    let filtered: Vec<ContractEndpoint> = state
        .contract
        .endpoints
        .iter()
        .filter(|e| {
            q.module
                .as_deref()
                .is_none_or(|m| e.module.eq_ignore_ascii_case(m))
        })
        .filter(|e| q.tag.as_deref().is_none_or(|t| e.tag == t))
        .filter(|e| {
            kw.as_deref().is_none_or(|k| {
                e.path.to_lowercase().contains(k)
                    || e.summary.to_lowercase().contains(k)
                    || e.controller.to_lowercase().contains(k)
            })
        })
        .cloned()
        .collect();

    let total = filtered.len();
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(50).clamp(1, 500);
    let start = (page - 1).saturating_mul(page_size).min(total);
    let end = start.saturating_add(page_size).min(total);

    Json(ContractPage {
        version: state.contract.version.clone(),
        total,
        page,
        page_size,
        list: filtered[start..end].to_vec(),
    })
}

/// `GET /openapi/coverage`：完整覆盖率报告。
async fn coverage_handler(State(state): State<OpenApiState>) -> Json<Coverage> {
    Json(coverage(&state.contract, &state.routes))
}

/// `GET /openapi/missing` 查询参数。
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct MissingQuery {
    pub module: Option<String>,
    pub limit: Option<usize>,
}

/// 未实现接口响应。
#[derive(Debug, Serialize)]
pub struct MissingPage {
    pub total: usize,
    pub returned: usize,
    pub list: Vec<ContractEndpoint>,
}

/// `GET /openapi/missing`：尚未实现的接口清单（开发待办）。
async fn missing_handler(
    State(state): State<OpenApiState>,
    Query(q): Query<MissingQuery>,
) -> Json<MissingPage> {
    let cov = coverage(&state.contract, &state.routes);
    let filtered: Vec<ContractEndpoint> = cov
        .missing
        .into_iter()
        .filter(|e| {
            q.module
                .as_deref()
                .is_none_or(|m| e.module.eq_ignore_ascii_case(m))
        })
        .collect();
    let total = filtered.len();
    let limit = q.limit.unwrap_or(100).clamp(1, 2000);
    let list: Vec<ContractEndpoint> = filtered.into_iter().take(limit).collect();
    Json(MissingPage {
        total,
        returned: list.len(),
        list,
    })
}

/// 覆盖率摘要。
#[derive(Debug, Serialize)]
pub struct Summary {
    pub contract_version: String,
    pub contract_available: bool,
    pub total: usize,
    pub implemented: usize,
    pub missing: usize,
    pub percent: f64,
    pub by_module: Vec<ModuleCoverage>,
}

/// `GET /openapi/summary`：轻量汇总，适合 CI 与监控断言。
async fn summary_handler(State(state): State<OpenApiState>) -> Json<Summary> {
    let cov = coverage(&state.contract, &state.routes);
    Json(Summary {
        contract_version: cov.contract_version,
        contract_available: cov.contract_available,
        total: cov.total,
        implemented: cov.implemented,
        missing: cov.missing.len(),
        percent: cov.percent,
        by_module: cov.by_module,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(method: &str, path: &str, module: &str) -> ContractEndpoint {
        ContractEndpoint {
            module: module.into(),
            method: method.into(),
            path: path.into(),
            summary: "测试接口".into(),
            tag: "商品".into(),
            auth: "jwt".into(),
            permissions: vec!["productManage".into()],
            controller: "ProductController".into(),
            handler: "list".into(),
            source: "ProductController.java:1".into(),
        }
    }

    fn state() -> OpenApiState {
        let contract = Contract {
            version: "5.8.57".into(),
            generated_by: "test".into(),
            endpoint_count: 2,
            controller_count: 1,
            endpoints: vec![
                endpoint("GET", "/api/product/list", "api"),
                endpoint("POST", "/adminapi/product/create", "adminapi"),
            ],
        };
        OpenApiState::new(
            contract,
            vec![RouteDoc::new("GET", "/api/product/list", "P.list")],
        )
    }

    #[test]
    fn coverage_reflects_declared_routes() {
        let s = state();
        let cov = coverage(&s.contract, &s.routes);
        assert_eq!(cov.total, 2);
        assert_eq!(cov.implemented, 1);
        assert_eq!(cov.missing.len(), 1);
        assert_eq!(cov.missing[0].path, "/adminapi/product/create");
    }

    #[test]
    fn summary_percent_is_rounded_to_one_decimal() {
        let s = state();
        let cov = coverage(&s.contract, &s.routes);
        // 1/2 = 50.0
        assert!((cov.percent - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn filter_matches_keyword_and_module() {
        let s = state();
        let q = ContractQuery {
            module: Some("api".into()),
            keyword: Some("product".into()),
            ..Default::default()
        };
        let matched: Vec<_> = s
            .contract
            .endpoints
            .iter()
            .filter(|e| q.module.as_deref().is_none_or(|m| e.module.eq_ignore_ascii_case(m)))
            .filter(|e| {
                q.keyword
                    .as_deref()
                    .is_none_or(|k| e.path.to_lowercase().contains(&k.to_lowercase()))
            })
            .collect();
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].path, "/api/product/list");
    }
}