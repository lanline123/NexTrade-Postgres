//! 批发汇总 —— 管理端路由（wholesale::wholesale_summary）。**我方新增**
//! （论坛帖 127889 评估 F3「总部后台看所有门店批发汇总」+ 门店级批发
//! 清单，二期补齐；原版批发域无对应控制器）。
//!
//! | method | path（本模块，挂载于 `/adminapi`） | 返回 |
//! |--------|------------------------------------|------|
//! | `GET`  | `/wholesale/summary/overview` | `WholesaleSummaryOverviewVO` |
//! | `GET`  | `/wholesale/summary/shopPage` | `PageResult<WholesaleShopSummaryVO>` |
//! | `GET`  | `/wholesale/summary/productPage` | `PageResult<WholesaleShopProductVO>` |
//!
//! # 多租户
//!
//! 与批发商品路由同口径：从 `Extension<AuthUser>` 取租户作用域
//! （平台 → 0、商家 → 本店、缺作用域 → -1 哨兵 fail-closed），作用域
//! 裁决在模型层 `resolve_summary_scope` 单点完成（平台见全量、可按
//! `shopId` 缩到单店；商家强制本店）。

use axum::{extract::Query, extract::State, routing::get, Extension, Router};
use nextrade_core::models::wholesale::wholesale_summary::{
    WholesaleShopProductPageQuery, WholesaleShopProductVO, WholesaleShopSummaryVO,
    WholesaleSummaryOverviewVO, WholesaleSummaryQuery,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::AdminState;

/// 本域全部 3 条路由（去掉 `/adminapi` 前缀后的路径 + 说明）。
const ROUTE_TABLE: [(&str, &str, &str); 3] = [
    (
        "GET",
        "/wholesale/summary/overview",
        "WholesaleSummary.overview（我方新增）",
    ),
    (
        "GET",
        "/wholesale/summary/shopPage",
        "WholesaleSummary.shopPage（我方新增）",
    ),
    (
        "GET",
        "/wholesale/summary/productPage",
        "WholesaleSummary.productPage（我方新增）",
    ),
];

/// 批发汇总路由（挂载于 `/adminapi` 下，全局 `require_auth` 已生效）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/wholesale/summary/overview", get(overview))
        .route("/wholesale/summary/shopPage", get(shop_page))
        .route("/wholesale/summary/productPage", get(product_page))
}

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, java_ref)| {
            let full = format!("/adminapi{path}");
            nextrade_openapi::RouteDoc::new(method, &full, java_ref)
        })
        .collect()
}

/// 当前操作者的店铺作用域（与批发商品路由同口径）：平台 → 0、
/// 商家 → 本店 shop_id、缺作用域 → -1 哨兵。
fn shop_id_of(auth: &AuthUser) -> i32 {
    auth.scoped_shop_id().unwrap_or(0)
}

/// `GET /adminapi/wholesale/summary/overview`
async fn overview(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<WholesaleSummaryQuery>,
) -> ApiResult<WholesaleSummaryOverviewVO> {
    ok(state
        .wholesale
        .wholesale_summary
        .overview(&q, shop_id_of(&auth))
        .await?)
}

/// `GET /adminapi/wholesale/summary/shopPage`
async fn shop_page(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<WholesaleSummaryQuery>,
) -> ApiResult<PageResult<WholesaleShopSummaryVO>> {
    ok(state
        .wholesale
        .wholesale_summary
        .shop_page(&q, shop_id_of(&auth))
        .await?)
}

/// `GET /adminapi/wholesale/summary/productPage`
async fn product_page(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<WholesaleShopProductPageQuery>,
) -> ApiResult<PageResult<WholesaleShopProductVO>> {
    ok(state
        .wholesale
        .wholesale_summary
        .product_page(&q, shop_id_of(&auth))
        .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 路由表 3 条、路径无重复且均为 GET。
    #[test]
    fn route_table_shape() {
        assert_eq!(ROUTE_TABLE.len(), 3);
        let paths: Vec<&str> = ROUTE_TABLE.iter().map(|(_, p, _)| *p).collect();
        assert_eq!(
            paths,
            vec![
                "/wholesale/summary/overview",
                "/wholesale/summary/shopPage",
                "/wholesale/summary/productPage",
            ]
        );
        assert!(ROUTE_TABLE.iter().all(|(m, _, _)| *m == "GET"));
        let mut unique = paths.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), paths.len());
    }

    /// `implemented_routes()` 返回完整契约路径（含 `/adminapi`）。
    #[test]
    fn implemented_routes_are_full_paths() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 3);
        for d in &docs {
            assert!(
                d.path.starts_with("/adminapi/wholesale/summary/"),
                "{} 缺少前缀",
                d.path
            );
        }
    }

    /// query 参数 camelCase（startTime/shopId/pageSize）。
    #[test]
    fn summary_query_parses_camel_case() {
        let q: WholesaleSummaryQuery =
            serde_json::from_str(r#"{"startTime":100,"endTime":200,"shopId":7,"pageSize":50}"#)
                .unwrap();
        assert_eq!(q.start_time, Some(100));
        assert_eq!(q.end_time, Some(200));
        assert_eq!(q.shop_id, Some(7));
        assert_eq!(q.page_size, Some(50));
    }
}
