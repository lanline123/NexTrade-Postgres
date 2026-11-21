//! 管理端 运营代码与验证设置路由（system::ops_code_config，2 条）。
//!
//! 平台级运营自助配置（论坛帖 477914 立项批次）：站长验证（六平台
//! meta 验证码 + 单个验证文件）与第三方代码位（`headCode` 头部代码 /
//! `bodyCode` 底部代码，统计代码粘后者），存 `config` 表
//! `opsCodeSettings` 一行 JSON，前台经 `GET /api/common/config/opsCode`
//! 消费。客服 JS 代码位不在本块——它走原版客服设置组
//! （`kefuJavascript`/`wapKefuJavascript`，见 EP8/EP9 字段注册）。
//! 契约为本平台新增（非 Java 迁移端点），命名与购买按钮样式设置
//! 保持一致：`GET /opsCodeSettings`、`POST /saveOpsCode`。
//!
//! ⚠️ 权限：整域挂 `platform_only`。管理员粘贴的代码将以原始形态
//! 在商城 PC/H5 页面执行——这是「预留 JS 位」的语义本身，权限边界
//! 就是平台管理员这一层，绝不下放商户/店铺账号。

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
        "/setting/config/opsCodeSettings",
        "OpsCodeConfigController.getOpsCodeSettings",
    ),
    (
        "POST",
        "/setting/config/saveOpsCode",
        "OpsCodeConfigController.saveOpsCode",
    ),
];

pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/setting/config/opsCodeSettings", get(get_settings))
        .route("/setting/config/saveOpsCode", post(save_settings))
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

/// `GET /adminapi/setting/config/opsCodeSettings`：读取生效中的运营
/// 代码与验证配置（未保存过时返回内置默认——全部为空）。
async fn get_settings(State(state): State<AdminState>) -> ApiResult<Value> {
    ok(state.system.config_basic.get_ops_code().await?)
}

/// `POST /adminapi/setting/config/saveOpsCode`：保存运营代码与验证
/// 配置（整块覆盖，保存前经服务层严格校验：验证文件名非法或文件
/// 名/内容只填其一时明确报错）。
async fn save_settings(
    State(state): State<AdminState>,
    Json(payload): Json<Value>,
) -> ApiResult<()> {
    state.system.config_basic.save_ops_code(&payload).await?;
    ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract_exactly() {
        let expected = [
            ("GET", "/adminapi/setting/config/opsCodeSettings"),
            ("POST", "/adminapi/setting/config/saveOpsCode"),
        ];
        let docs = implemented_routes();
        assert_eq!(docs.len(), 2);
        for (doc, (method, path)) in docs.iter().zip(expected.iter()) {
            assert_eq!(doc.method, *method);
            assert_eq!(doc.path, *path);
        }
    }
}
