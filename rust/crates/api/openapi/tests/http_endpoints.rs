//! `/openapi/*` HTTP 层集成测试。
//!
//! 直接用 `tower::ServiceExt::oneshot` 调用路由，不需要数据库或网络端口，
//! 因此可以在 CI 中稳定运行，验证契约查询与覆盖率端点真的可用。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt; // for `oneshot`

use nextrade_openapi::{routes, Contract, OpenApiState, RouteDoc};

fn app() -> axum::Router {
    let contract = Contract::embedded();
    // 声明一条真实存在于契约中的路由，用于验证覆盖率会把它计入已实现。
    let routes_decl = vec![RouteDoc::new(
        "GET",
        "/adminapi/authority/adminUser/list",
        "AdminUserController.list",
    )];
    routes(OpenApiState::new(contract, routes_decl))
}

async fn get_json(path: &str) -> (StatusCode, serde_json::Value) {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("router responded");
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 8 * 1024 * 1024)
        .await
        .expect("read body");
    let json: serde_json::Value =
        serde_json::from_slice(&bytes).expect("response is valid JSON");
    (status, json)
}

/// 无契约时跳过（原因见 contract_integrity.rs 的同名说明）。
macro_rules! skip_without_contract {
    () => {
        if !Contract::embedded().is_available() {
            eprintln!("[skip] 未嵌入 API 契约（缺 docs/api-contract.json），跳过该用例");
            return;
        }
    };
}

#[tokio::test]
async fn summary_reports_totals_and_percent() {
    skip_without_contract!();
    let (status, body) = get_json("/summary").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["contract_available"], true);

    let total = body["total"].as_u64().expect("total is a number");
    let implemented = body["implemented"].as_u64().expect("implemented is a number");
    let missing = body["missing"].as_u64().expect("missing is a number");

    assert!(total >= 1500, "契约接口总数异常偏少: {total}");
    assert_eq!(
        implemented + missing,
        total,
        "implemented + missing 应等于 total"
    );
    assert_eq!(implemented, 1, "只声明了一条已实现路由");
    assert!(body["percent"].is_number());
}

#[tokio::test]
async fn coverage_lists_missing_and_by_module() {
    skip_without_contract!();
    let (status, body) = get_json("/coverage").await;
    assert_eq!(status, StatusCode::OK);
    let missing = body["missing"].as_array().expect("missing is an array");
    assert!(!missing.is_empty(), "绝大多数接口尚未实现，missing 不应为空");

    let by_module = body["by_module"].as_array().expect("by_module is an array");
    let modules: Vec<&str> = by_module
        .iter()
        .map(|m| m["module"].as_str().unwrap_or_default())
        .collect();
    assert!(modules.contains(&"adminapi"));
    assert!(modules.contains(&"api"));

    // 每个模块的 implemented 不应超过 total。
    for m in by_module {
        let t = m["total"].as_u64().unwrap_or(0);
        let i = m["implemented"].as_u64().unwrap_or(0);
        assert!(i <= t, "模块 {} 已实现数超过总数", m["module"]);
    }
}

#[tokio::test]
async fn contract_query_paginates_and_filters() {
    skip_without_contract!();
    let (status, body) = get_json("/contract?module=api&pageSize=5&page=1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["page"], 1);
    assert_eq!(body["page_size"], 5);

    let list = body["list"].as_array().expect("list is an array");
    assert_eq!(list.len(), 5, "pageSize=5 应返回 5 条");
    for item in list {
        assert_eq!(item["module"], "api");
        assert!(
            item["path"].as_str().unwrap_or_default().starts_with('/'),
            "路径应以 / 开头"
        );
        // 契约条目必须带可回溯的源码位置，便于核对实现。
        assert!(
            !item["source"].as_str().unwrap_or_default().is_empty(),
            "契约条目缺少 source 字段"
        );
    }

    let total = body["total"].as_u64().expect("total is a number");
    assert!(total > 5, "api 模块接口数应远大于一页");
}

#[tokio::test]
async fn contract_keyword_search_matches_path_or_summary() {
    skip_without_contract!();
    let (status, body) = get_json("/contract?keyword=adminUser&pageSize=50").await;
    assert_eq!(status, StatusCode::OK);
    let list = body["list"].as_array().expect("list is an array");
    assert!(!list.is_empty(), "应能按关键词 adminUser 命中接口");
    for item in list {
        let path = item["path"].as_str().unwrap_or_default().to_lowercase();
        let summary = item["summary"].as_str().unwrap_or_default().to_lowercase();
        let ctrl = item["controller"].as_str().unwrap_or_default().to_lowercase();
        assert!(
            path.contains("adminuser")
                || summary.contains("adminuser")
                || ctrl.contains("adminuser"),
            "关键词过滤结果不匹配: {}",
            item["path"]
        );
    }
}

#[tokio::test]
async fn missing_endpoint_respects_limit() {
    skip_without_contract!();
    let (status, body) = get_json("/missing?limit=10").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["returned"], 10);
    assert_eq!(
        body["list"].as_array().map(Vec::len),
        Some(10),
        "list 长度应与 returned 一致"
    );
    let total = body["total"].as_u64().expect("total is a number");
    assert!(total > 10);
}