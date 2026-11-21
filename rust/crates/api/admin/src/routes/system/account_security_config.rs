//! 管理端 账号安全设置路由（system::account_security_config，2 条）。
//!
//! 平台级全站配置：登录失败计数与临时锁定（论坛 B 类第 6 项，用户
//! 2026-11-21 拍板），存 `config` 表 `accountSecurity` 一行 JSON，登录链
//! （会员 `UserService::login` / 后台 `AuthService::login`）实时读取生效。
//! 契约为本平台新增（非 Java 迁移端点），与购买按钮样式的命名保持一致：
//! `GET /accountSecuritySettings`、`POST /saveAccountSecurity`。

use axum::{extract::State, routing::{get, post}, Json, Router};
use serde_json::Value;
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult};

use crate::support::tenant::platform_only;

use crate::AdminState;

/// 本域 2 条路由（`/adminapi` 前缀之后的路径）。
const ROUTE_TABLE: [(&str, &str, &str); 2] = [
    (
        "GET",
        "/setting/config/accountSecuritySettings",
        "AccountSecurityConfigController.getAccountSecuritySettings",
    ),
    (
        "POST",
        "/setting/config/saveAccountSecurity",
        "AccountSecurityConfigController.saveAccountSecurity",
    ),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/setting/config/accountSecuritySettings", get(get_settings))
        .route("/setting/config/saveAccountSecurity", post(save_settings))
        .layer(axum::middleware::from_fn(platform_only))
}

/// 本域已实现路由（完整契约路径，用于覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/adminapi{path}"), handler))
        .collect()
}

/// `GET /adminapi/setting/config/accountSecuritySettings`：读取生效中的
/// 账号安全设置（未保存过时返回内置默认：开启、5 次 / 15 分钟）。
async fn get_settings(State(state): State<AdminState>) -> ApiResult<Value> {
    ok(state.system.config_basic.get_account_security().await?)
}

/// `POST /adminapi/setting/config/saveAccountSecurity`：保存账号安全设置
/// （整块覆盖，保存前经服务层清洗校验）。
async fn save_settings(
    State(state): State<AdminState>,
    Json(payload): Json<Value>,
) -> ApiResult<()> {
    state.system.config_basic.save_account_security(&payload).await?;
    ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract_exactly() {
        let expected = [
            ("GET", "/adminapi/setting/config/accountSecuritySettings"),
            ("POST", "/adminapi/setting/config/saveAccountSecurity"),
        ];
        let docs = implemented_routes();
        assert_eq!(docs.len(), 2);
        for (doc, (method, path)) in docs.iter().zip(expected.iter()) {
            assert_eq!(doc.method, *method);
            assert_eq!(doc.path, *path);
        }
    }
}
