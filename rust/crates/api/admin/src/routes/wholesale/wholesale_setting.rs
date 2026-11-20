//! 批发设置 —— 管理端路由（wholesale::wholesale_setting）。
//!
//! 对应原 Java `com.nextrade.adminapi.controller.wholesale.WholesaleSettingController`
//! （`@RequestMapping("/adminapi/wholesale/setting")`，2 个端点）。
//!
//! ## 原 Java 路径对照表（本模块挂载于 `/adminapi` 之下）
//!
//! | 本模块路由 | 原 Java 完整路径 | 原 Java 方法 | 权限 |
//! |-----------|------------------|----------------|------|
//! | `GET  /wholesale/setting/get` | `GET /adminapi/wholesale/setting/get` | `WholesaleSettingController.get` | 仅需登录（Java 无 `@PreAuthorize`） |
//! | `POST /wholesale/setting/save` | `POST /adminapi/wholesale/setting/save` | `WholesaleSettingController.save` | 仅需登录 |
//!
//! # 路径 / 返回结构
//!
//! - 类级 `@RequestMapping` = `/adminapi/wholesale/setting`，方法级 `/get`、`/save`。
//! - `get` 返回 `WholesaleSettingVO`；`save` 返回 `void` → Rust `ApiResult<()>`
//!   （`data` 为 `null`）。
//! - Rust 侧 `require_auth` 目前只校验登录态，权限矩阵未实现（全局已知缺口）。

use axum::{
    extract::State,
    routing::{get, post},
    Extension, Json, Router,
};
use nextrade_core::models::wholesale::wholesale_setting::{
    WholesaleSettingSaveDTO, WholesaleSettingVO, WholesaleShopSwitchSaveDTO, WholesaleShopSwitchVO,
};
use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::AdminState;

/// 本域全部 2 条路由（去掉 `/adminapi` 前缀后的路径 + 原 Java handler）。
const ROUTE_TABLE: [(&str, &str, &str); 2] = [
    (
        "GET",
        "/wholesale/setting/get",
        "WholesaleSettingController.get",
    ),
    (
        "POST",
        "/wholesale/setting/save",
        "WholesaleSettingController.save",
    ),
];

/// 批发设置路由（挂载于 `/adminapi` 下，全局 `require_auth` 已生效）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/wholesale/setting/get", get(get_setting))
        .route("/wholesale/setting/save", post(save_setting))
        // 我方新增（2026-11-21，论坛帖 777891）：店级批发开关。原版无此
        // 接口，故**不**进 ROUTE_TABLE（该表是与 Java 契约逐条对应的
        // 2 条），另有守护单测确认不混入契约表。
        .route("/wholesale/setting/shopSwitch", get(get_shop_switch))
        .route(
            "/wholesale/setting/shopSwitch/save",
            post(save_shop_switch),
        )
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

/// `GET /adminapi/wholesale/setting/get`：查询批发设置。
///
/// 对应 Java `WholesaleSettingController.get()` → `WholesaleSettingService#get`。
async fn get_setting(State(state): State<AdminState>) -> ApiResult<WholesaleSettingVO> {
    ok(state.wholesale.wholesale_setting.get().await?)
}

/// `POST /adminapi/wholesale/setting/save`：保存批发设置。
///
/// 对应 Java `WholesaleSettingController.save(@Valid @RequestBody ...)`。
async fn save_setting(
    State(state): State<AdminState>,
    Json(dto): Json<WholesaleSettingSaveDTO>,
) -> ApiResult<()> {
    state.wholesale.wholesale_setting.save(&dto).await?;
    ok(())
}

/// `GET /adminapi/wholesale/setting/shopSwitch`：店级批发开关三值
/// （**我方新增**，论坛帖 777891）。
///
/// 作用域：商家账号 → 本店；平台账号 → shop 0 语义（店级恒开、有效值
/// 只看全局）；缺作用域的店铺账号直接 403（fail-closed，不回退平台语义）。
async fn get_shop_switch(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<WholesaleShopSwitchVO> {
    let shop_id = shop_switch_scope(&auth)?;
    ok(state
        .wholesale
        .wholesale_setting
        .get_shop_switch(shop_id)
        .await?)
}

/// 店级开关查询的作用域解析（纯函数）：缺作用域 → 403；平台 → 0；
/// 商家 → 本店；vendor → 0（平台语义只读视图）。
fn shop_switch_scope(auth: &AuthUser) -> Result<i32, Error> {
    if auth.shop_scope_missing() {
        return Err(Error::Forbidden(
            "当前账号未绑定店铺，无法访问店铺数据".into(),
        ));
    }
    Ok(auth.scoped_shop_id().unwrap_or(0))
}

/// `POST /adminapi/wholesale/setting/shopSwitch/save`：保存本店批发开关
/// （**我方新增**，论坛帖 777891）。
///
/// 仅商家账号可写（平台账号走批发设置总开关，`require_shop_scope`
/// 对平台返回 403 提示指定店铺，本入口不开放平台代改）。
async fn save_shop_switch(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<WholesaleShopSwitchSaveDTO>,
) -> ApiResult<WholesaleShopSwitchVO> {
    let enabled = dto
        .enabled
        .ok_or_else(|| Error::Validation("enabled 不能为空".into()))?;
    let shop_id = auth.require_shop_scope(None)?;
    ok(state
        .wholesale
        .wholesale_setting
        .save_shop_switch(shop_id, enabled)
        .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 路由表与契约 2 条逐字符一致（含 camelCase 之外的大小写）。
    #[test]
    fn route_table_matches_java_contract() {
        assert_eq!(ROUTE_TABLE.len(), 2);
        let paths: Vec<&str> = ROUTE_TABLE.iter().map(|(_, p, _)| *p).collect();
        assert_eq!(
            paths,
            vec!["/wholesale/setting/get", "/wholesale/setting/save"]
        );
        for p in &paths {
            assert!(p.starts_with("/wholesale/setting/"), "{p} 前缀错误");
            assert!(!p.starts_with("/adminapi"));
        }
    }

    /// `implemented_routes()` 返回完整契约路径（含 `/adminapi`）。
    #[test]
    fn implemented_routes_are_full_contract_paths() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 2);
        let actual: Vec<(String, String)> = docs
            .iter()
            .map(|d| (d.method.clone(), d.path.clone()))
            .collect();
        assert_eq!(
            actual,
            vec![
                (
                    "GET".to_string(),
                    "/adminapi/wholesale/setting/get".to_string()
                ),
                (
                    "POST".to_string(),
                    "/adminapi/wholesale/setting/save".to_string()
                ),
            ]
        );
    }

    /// method 与 Java 注解一致：get→GET，save→POST。
    #[test]
    fn http_methods_match_java_annotations() {
        assert_eq!(ROUTE_TABLE[0].0, "GET");
        assert_eq!(ROUTE_TABLE[1].0, "POST");
    }

    /// `routes()` 注册路径与 ROUTE_TABLE 完全一致。
    #[test]
    fn routes_registers_all_table_paths() {
        let registered = ["/wholesale/setting/get", "/wholesale/setting/save"];
        assert_eq!(registered.len(), ROUTE_TABLE.len());
        for path in registered {
            assert!(ROUTE_TABLE.iter().any(|(_, p, _)| *p == path));
        }
    }

    /// 我方新增的店级开关 2 条路径刻意**不**进 ROUTE_TABLE（契约表只含
    /// Java 原版 2 条），但必须已在 `routes()` 注册（见 routes 函数体）。
    #[test]
    fn shop_switch_routes_stay_out_of_contract_table() {
        for path in [
            "/wholesale/setting/shopSwitch",
            "/wholesale/setting/shopSwitch/save",
        ] {
            assert!(
                !ROUTE_TABLE.iter().any(|(_, p, _)| *p == path),
                "{path} 不应混入 Java 契约表"
            );
        }
    }
}
