//! 前台 运营代码与验证配置路由（system::ops_code_client，1 条）。
//!
//! 本平台新增端点（非 Java 契约）：`GET /api/common/config/opsCode`
//! 返回后台「运营代码与验证」的生效配置——站长验证（六平台 meta
//! 验证码 + 验证文件名/内容）、第三方代码（`headCode`/`bodyCode`）
//! 与原版客服 JS 两键（`kefuJavascript` PC / `wapKefuJavascript`
//! 移动端），供 PC layout 服务端注入与 H5 启动注入消费。未配置时
//! 全部为空串，消费端零注入。与 `config_client` 的 Java 契约文件
//! 分开，避免污染其契约测试。
//!
//! 公开只读的正当性：下发的全部内容本来就会出现在商城页面源码里
//! （meta 标签、验证文件、注入脚本），无敏感面；写入面在管理端
//! `platform_only` 端点（见 admin `ops_code_config`）。

use axum::{extract::State, routing::get, Router};
use serde_json::Value;
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult};

use crate::PublicState;

/// 本域 1 条路由（`/api` 前缀之后的路径）。
const ROUTE_TABLE: [(&str, &str, &str); 1] = [(
    "GET",
    "/common/config/opsCode",
    "OpsCodeController.getOpsCode",
)];

pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new().route("/common/config/opsCode", get(get_ops_code))
}

/// 本域已实现路由（完整契约路径，用于覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/api{path}"), handler))
        .collect()
}

/// `GET /api/common/config/opsCode`：运营代码与验证配置（公开只读）。
async fn get_ops_code(State(state): State<PublicState>) -> ApiResult<Value> {
    ok(state.system.config_client.ops_code().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_contract_exactly() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].method, "GET");
        assert_eq!(docs[0].path, "/api/common/config/opsCode");
    }
}
