//! PC 前台公共接口路由（system::pc）。
//!
//! 对应原 Java `com.nextrade.api.controller.common.PCController`
//! （`@RequestMapping("/api/common/pc")`，本切片 2 个端点）。
//!
//! | 本模块路由（挂载于 `/api`） | 原 Java 方法 | 契约 auth |
//! |------------------------------|--------------|-----------|
//! | `GET /common/pc/getNav` | `PCController.getNav` | optional |
//! | `GET /common/pc/getCatFloor` | `PCController.getCatFloor` | optional |
//!
//! > **路径说明**：`@RequestMapping("/api/common/pc")` +
//! > `@GetMapping("/getNav")` = `/api/common/pc/getNav`，
//! > 本域**没有**「控制器名重复一次」的情况；本文件挂在 `/api` 之下，
//! > 故 `routes()` 内写 `/common/pc/...`。**仍以契约逐字符为准**
//! > （`rust/docs/api-contract.json`）。
//!
//! > **auth=optional 的落地口径**：两个 handler 都不读取登录态
//! > —— `getNav` 只查导航配置，`getCatFloor` 只读分类抽屉（Java 侧
//! > `ThreadLocalUtil.getValue()` 仅用于 Redis 缓存键与翻译词条，
//! > 见服务层模块注释「未迁移依赖」）。因此本模块**不挂任何鉴权中间件**
//! > （游客与登录用户响应一致），符合 Java `permitAll` 语义。
//!
//! > **未迁移**：多语言翻译（`TranslatePackage`，按原文返回）与
//! > `getCatFloor` 的 Redis 缓存（每次回源，结果等价）。

use axum::{extract::State, routing::get, Router};
use nextrade_core::models::system::pc::{CatFloorWrapperVO, NavVO};
use nextrade_security::web::{ok, ApiResult};

use crate::PublicState;

/// PC 公共域前台路由（挂载于 `/api` 下）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .route("/common/pc/getNav", get(get_nav))
        .route("/common/pc/getCatFloor", get(get_cat_floor))
}

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    [
        ("GET", "/api/common/pc/getNav", "PCController.getNav"),
        (
            "GET",
            "/api/common/pc/getCatFloor",
            "PCController.getCatFloor",
        ),
    ]
    .iter()
    .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, p, j))
    .collect()
}

/// `GET /api/common/pc/getNav`：获取导航（主导航 / 顶部栏 / 底部栏 / 边栏）。
///
/// 对应 Java `PCController.getNav` → `PcNavigationServiceImpl#getAllNav(0)`
/// （`is_show = 1` 的导航按 `sort_order` 建两层树后按类型分桶）。
async fn get_nav(State(state): State<PublicState>) -> ApiResult<NavVO> {
    ok(state.system.pc.get_nav().await?)
}

/// `GET /api/common/pc/getCatFloor`：获取阶梯菜单（首页分类抽屉）。
///
/// 对应 Java `PCController.getCatFloor` → `PcCatFloorServiceImpl#getCatFloor`
/// （楼层 `catFloor` + 自定义 ICO 图标库 `icoDefinedCss`）。
async fn get_cat_floor(State(state): State<PublicState>) -> ApiResult<CatFloorWrapperVO> {
    ok(state.system.pc.get_cat_floor().await?)
}