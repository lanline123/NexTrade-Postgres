//! 机器翻译 —— 商城前台路由（system::translate_client）。
//!
//! `POST /api/common/translate/translate`：H5 通用机器翻译
//! （b2b2b 5.8.57 前端新增，**无 Java 控制器对应**）。
//!
//! ## 实现口径（任务约定）
//! - 复用管理端 `lang::translations` 的机器翻译内核
//!   （火山引擎 `TranslateText`，凭据在 `config` 表
//!   `langVolcengineAccessKey` / `langVolcengineSecret`）；
//! - **无凭据 / 翻译失败 → 原样返回** `{translation: 原文}`，不报错。
//!
//! 匿名端点（H5 页面翻译挂件在登录前也可能调用）。

use axum::{extract::State, routing::post, Json, Router};
use serde::Deserialize;
use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult};

use crate::PublicState;

/// 本域路由（`/api` 前缀之后）。
const ROUTE_TABLE: [(&str, &str, &str); 1] = [(
    "POST",
    "/common/translate/translate",
    "TranslateClientController.translate",
)];

/// 机器翻译路由（挂载于 `/api` 下，匿名）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new().route("/common/translate/translate", post(translate))
}

/// 本域已实现路由（覆盖率统计用；不在原契约内，计入 extra_routes）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, &format!("/api{p}"), j))
        .collect()
}

/// 请求体 `{code, text}`（目标语言代码 + 原文）。
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct TranslateDTO {
    /// 目标语言代码（如 `en` / `zh`）
    code: Option<String>,
    text: Option<String>,
}

/// 出参 `{translation}`（无凭据时 = 原文）。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TranslateVO {
    translation: String,
}

/// `POST /api/common/translate/translate`：机器翻译（无凭据原样返回）。
async fn translate(
    State(state): State<PublicState>,
    Json(dto): Json<TranslateDTO>,
) -> ApiResult<TranslateVO> {
    let code = dto.code.as_deref().unwrap_or("en");
    let text = dto.text.as_deref().unwrap_or_default();
    if text.is_empty() {
        return ok(TranslateVO {
            translation: String::new(),
        });
    }
    // 复用 lang 域机器翻译内核（凭据缺失/上游失败均原样返回原文）
    let repos = state.system.region_client.repos();
    let translation =
        nextrade_services::lang::translations::machine_translate(repos, code, text)
            .await
            .map_err(|e| {
                Error::Internal(format!("翻译失败: {e}"))
            })?;
    ok(TranslateVO { translation })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_table_matches_endpoint_list() {
        let docs = implemented_routes();
        assert_eq!(docs[0].path, "/api/common/translate/translate");
        assert_eq!(docs[0].method, "POST");
    }

    #[test]
    fn vo_serializes_translation_field() {
        let vo = TranslateVO {
            translation: "你好".into(),
        };
        let json = serde_json::to_string(&vo).unwrap();
        assert_eq!(json, r#"{"translation":"你好"}"#);
    }
}
