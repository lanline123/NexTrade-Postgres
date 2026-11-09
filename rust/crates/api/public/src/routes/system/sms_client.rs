//! 短信验证码 —— 商城前台路由（system::sms_client）。
//!
//! `POST /api/sys/sms/getCode`：H5 通用「获取短信验证码」
//! （b2b2b 5.8.57 前端新增，**无 Java 控制器对应**）。
//!
//! ## 实现口径（任务约定）
//! - 复用既有验证码存储与校验体系：与
//!   `POST /api/user/login/sendMobileCode` 同一套缓存键
//!   （`core::models::member::login::sms_code_key`，即 `{event}{mobile}`，
//!   5 分钟 TTL），注册 / 登录 / 找回密码的 `check` 链路可直接校验；
//! - 配置了短信渠道（`config` 表 smsProvider 等 / 环境变量）才真发
//!   （`services::system::notify::send_sms_code`，阿里云/腾讯云/华为/火山）；
//! - **当前未配置凭据 → 只写缓存、不真发**，仍返回 code:0（no-op）。
//!
//! 匿名端点（H5 登录/注册页在登录前调用），不套 `require_auth`。

use axum::{
    extract::{Query, State},
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use nextrade_core::models::member::login::{sms_code_key, RegisterSmsDTO, CODE_TTL_SECS};
use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult};

use crate::PublicState;

/// 本域路由（`/api` 前缀之后）。
const ROUTE_TABLE: [(&str, &str, &str); 1] =
    [("POST", "/sys/sms/getCode", "SmsClientController.getCode")];

/// 短信验证码路由（挂载于 `/api` 下，匿名）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new().route("/sys/sms/getCode", post(get_code).get(get_code_query))
}

/// 本域已实现路由（覆盖率统计用；不在原契约内，计入 extra_routes）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, &format!("/api{p}"), j))
        .collect()
}

/// `POST /api/sys/sms/getCode`：获取短信验证码（body `{mobile, event?}`）。
async fn get_code(
    State(state): State<PublicState>,
    Json(dto): Json<RegisterSmsDTO>,
) -> ApiResult<()> {
    send_code(&state, dto).await?;
    ok(())
}

/// GET 形态（query 兜底，H5 部分页面用 GET）。
async fn get_code_query(
    State(state): State<PublicState>,
    Query(q): Query<RegisterSmsQuery>,
) -> ApiResult<()> {
    send_code(
        &state,
        RegisterSmsDTO {
            mobile: q.mobile,
            event: q.event,
            ..Default::default()
        },
    )
    .await?;
    ok(())
}

/// GET query 形态。
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct RegisterSmsQuery {
    mobile: Option<String>,
    event: Option<String>,
}

/// 生成 → 写缓存 → 尝试真发（无凭据吞掉错误，no-op）。
async fn send_code(state: &PublicState, dto: RegisterSmsDTO) -> Result<(), Error> {
    let mobile = dto
        .mobile
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .ok_or_else(|| Error::Validation("手机号码不能为空".into()))?
        .to_string();
    let code = nextrade_services::member::login::generate_sms_code();
    let key = sms_code_key(dto.event.as_deref(), &mobile);
    nextrade_infrastructure::cache::get_cache()
        .set_json(&key, &code, CODE_TTL_SECS)
        .await?;
    // 真发（配置了短信渠道才会成功；未配置按任务约定 no-op，不报错）
    let repos = state.system.region_client.repos();
    if let Err(e) =
        nextrade_services::system::notify::send_sms_code(repos, &mobile, &code).await
    {
        tracing::warn!("sys/sms/getCode：短信未发出（仅写缓存）: {e}");
    }
    tracing::info!("短信验证码已生成: mobile={mobile} event={:?}", dto.event);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_table_matches_endpoint_list() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].path, "/api/sys/sms/getCode");
        assert_eq!(docs[0].method, "POST");
    }

    #[test]
    fn key_matches_login_scheme() {
        // 与 login/sendMobileCode 同键：event 前缀 + 手机号
        assert_eq!(sms_code_key(None, "13800000000"), "13800000000");
        assert_eq!(
            sms_code_key(Some("register"), "13800000000"),
            "register13800000000"
        );
    }
}
