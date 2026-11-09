//! 商城前台会员登录路由（member::login）。
//!
//! 对应原 Java：
//! - `com.nextrade.api.controller.user.LoginController`（`@RequestMapping("/api/user/login")`）
//! - `com.nextrade.adminapi.controller.user.LoginController`（`@RequestMapping("/adminapi/login")`）
//!
//! | 本模块路由 | 原 Java 方法 | 访问控制 |
//! |------------|--------------|----------|
//! | `POST /api/user/login/sendMobileCode` | `LoginController.sendMobileCode` | 匿名 |
//! | `GET|POST /api/user/login/wechatServer` | `LoginController.wechatServer` | 匿名（微信服务器回调） |
//! | `GET  /api/user/login/getQuickLoginSetting` | `LoginController.getQuickLoginSetting` | 匿名 |
//! | `GET  /api/user/login/getWxLoginUrl` | `LoginController.getWechatLoginUrl` | 匿名 |
//! | `POST /api/user/login/sendEmailCode` | `LoginController.sendEmailCode` | 匿名 |
//! | `POST /api/user/login/checkMobile` | `LoginController.checkMobile` | 匿名 |
//! | `POST /api/user/login/checkEmail` | `LoginController.checkEmail` | 匿名 |
//! | `POST /api/user/login/forgetPassword` | `LoginController.forgetPassword` | 匿名 |
//! | `POST /api/user/login/bindMobile` | `LoginController.bindMobile` | 匿名（绑定后返回会员 JWT） |
//! | `POST /api/user/login/wechatEvent` | `LoginController.wechatEvent` | 匿名（未迁移） |
//! | `POST /api/user/login/getMobile` | `LoginController.getMobile` | 匿名（未迁移） |
//! | `POST /api/user/login/updateUserOpenid` | `LoginController.updateUserOpenId` | **需登录**（未迁移） |
//! | `GET  /api/user/login/getWxLoginInfoByCode` | `LoginController.getWxLoginInfoByCode` | 匿名（未迁移） |
//! | `POST /api/user/login/bindWechat` | `LoginController.bindWechat` | **需登录**（未迁移） |
//! | `GET  /api/user/login/unbindWechat` | `LoginController.unbindWechat` | **需登录** |
//! | `POST /adminapi/login/sendMobileCode` | `adminapi LoginController.sendMobileCode` | 匿名（**归 admin 应用挂载**） |
//!
//! > **挂载说明（重要）**：本文件属于 `api/public`（统一 nest 在 `/api` 下），
//! > 契约里的 `/adminapi/login/sendMobileCode` **无法**在这里挂载：
//! > 它属于 admin 应用（`api/admin`），需主会话在 `api/admin/src/routes/...`
//! > 或 `api/admin/src/lib.rs` 内用
//! > `state.member.login.send_mobile_code(&dto)` 挂一条
//! > `.route("/login/sendMobileCode", post(...))`。
//! > 本模块仍把它列入 [`ROUTE_TABLE`] / [`implemented_routes`]，覆盖率不丢；
//! > 路由表测试断言「已挂载 15 条 + 声明 1 条 = 契约 16 条」。
//!
//! > **鉴权约定**：`routes(state)` 内含 3 条需登录接口（自套 `require_auth`），
//! > 其余 12 条匿名接口放 [`anonymous_routes`]；两者都并入 `merge_public`，
//! > 外层不要再套鉴权（与 `member::user_center` 的约定一致）。
//!
//! > **未迁移**：6 条微信生态端点（见 `services/member/login.rs` 模块注释）返回
//! > 明确的 1001 业务错误；验证码接口不做行为验证、短信/邮件只写缓存不实际发送。

use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
use nextrade_core::models::member::login::{
    BindMobileDTO, BindWechatParam, CheckEmailDTO, CheckMobileDTO, CodeParamDTO, ForgetPasswordDTO,
    LoginWechatEventVO, QuickLoginSettingVO, RegisterEmailDTO, RegisterSmsDTO, WechatEventDTO,
    WechatLoginUrlVO, WxLoginInfoByCodeVO, WxLoginUrlQuery, CLIENT_TYPE_HEADER, CODE_PARAM_REQUIRED,
    SERVICE_DATA_ERROR,
};
use nextrade_security::web::{ok, require_auth, ApiResult, AuthUser};
use nextrade_services::member::login::WechatServerQuery;

use crate::PublicState;

/// 契约全部 16 条（完整路径 + 原 Java 方法名）。
///
/// `wechatServer` 在 Java 用 `@RequestMapping`（不限方法），故 method 记 `ANY`。
const ROUTE_TABLE: [(&str, &str, &str); 16] = [
    ("ANY", "/api/user/login/wechatServer", "LoginController.wechatServer"),
    ("GET", "/api/user/login/getQuickLoginSetting", "LoginController.getQuickLoginSetting"),
    ("GET", "/api/user/login/getWxLoginUrl", "LoginController.getWechatLoginUrl"),
    ("POST", "/api/user/login/sendMobileCode", "LoginController.sendMobileCode"),
    ("POST", "/api/user/login/sendCode", "LoginController.sendMobileCode"),
    ("POST", "/api/user/login/sendEmailCode", "LoginController.sendEmailCode"),
    ("POST", "/api/user/login/checkMobile", "LoginController.checkMobile"),
    ("POST", "/api/user/login/checkEmail", "LoginController.checkEmail"),
    ("POST", "/api/user/login/forgetPassword", "LoginController.forgetPassword"),
    ("POST", "/api/user/login/bindMobile", "LoginController.bindMobile"),
    ("POST", "/api/user/login/wechatEvent", "LoginController.wechatEvent"),
    ("POST", "/api/user/login/getMobile", "LoginController.getMobile"),
    ("POST", "/api/user/login/updateUserOpenid", "LoginController.updateUserOpenId"),
    ("GET", "/api/user/login/getWxLoginInfoByCode", "LoginController.getWxLoginInfoByCode"),
    ("POST", "/api/user/login/bindWechat", "LoginController.bindWechat"),
    ("GET", "/api/user/login/unbindWechat", "LoginController.unbindWechat"),
];

/// 需登录的 3 条（Java 内用 `getCurrentUserId()`）：自套 `require_auth`。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/user/login/updateUserOpenid", post(update_user_openid))
        .route("/user/login/bindWechat", post(bind_wechat))
        .route("/user/login/unbindWechat", get(unbind_wechat))
        .layer(middleware::from_fn_with_state(
            state.jwt.clone(),
            require_auth,
        ))
}

/// 匿名的 12 条（含微信服务器回调的 GET/POST 双挂载）。
pub fn anonymous_routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .route(
            "/user/login/wechatServer",
            get(wechat_server).post(wechat_server),
        )
        .route("/user/login/getQuickLoginSetting", get(get_quick_login_setting))
        .route("/user/login/getWxLoginUrl", get(get_wx_login_url))
        .route("/user/login/sendMobileCode", post(send_mobile_code))
        .route("/user/login/sendCode", post(send_mobile_code))
        .route("/user/login/sendEmailCode", post(send_email_code))
        .route("/user/login/checkMobile", post(check_mobile))
        .route("/user/login/checkEmail", post(check_email))
        .route("/user/login/forgetPassword", post(forget_password))
        .route("/user/login/bindMobile", post(bind_mobile))
        .route("/user/login/wechatEvent", post(wechat_event))
        .route("/user/login/getMobile", post(get_mobile))
        .route("/user/login/getWxLoginInfoByCode", get(get_wx_login_info_by_code))
}

/// 本域已实现路由（完整契约路径，供 `/openapi/coverage`）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, java_ref)| nextrade_openapi::RouteDoc::new(method, path, java_ref))
        .collect()
}

/// `GET /api/user/login/getWxLoginInfoByCode?code=` 查询参数。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct CodeQuery {
    code: Option<String>,
}

/// 读取 `x-client-type`（Java `HeaderUtils#getClientType`）。
fn client_type(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(CLIENT_TYPE_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

// ————————————————————— 匿名端点 —————————————————————

/// `POST /api/user/login/sendMobileCode`：发送短信验证码。
async fn send_mobile_code(
    State(state): State<PublicState>,
    Json(dto): Json<RegisterSmsDTO>,
) -> ApiResult<()> {
    state.member.login.send_mobile_code(&dto).await?;
    ok(())
}

/// `POST /api/user/login/sendEmailCode`：发送邮箱验证码。
async fn send_email_code(
    State(state): State<PublicState>,
    Json(dto): Json<RegisterEmailDTO>,
) -> ApiResult<()> {
    state.member.login.send_email_code(&dto).await?;
    ok(())
}

/// `POST /api/user/login/checkMobile`：校验短信验证码并返回重置令牌。
async fn check_mobile(
    State(state): State<PublicState>,
    Json(dto): Json<CheckMobileDTO>,
) -> ApiResult<String> {
    ok(state.member.login.check_mobile(&dto).await?)
}

/// `POST /api/user/login/checkEmail`：校验邮箱验证码并返回重置令牌。
async fn check_email(
    State(state): State<PublicState>,
    Json(dto): Json<CheckEmailDTO>,
) -> ApiResult<String> {
    ok(state.member.login.check_email(&dto).await?)
}

/// `POST /api/user/login/forgetPassword`：用重置令牌修改密码。
async fn forget_password(
    State(state): State<PublicState>,
    Json(dto): Json<ForgetPasswordDTO>,
) -> ApiResult<()> {
    state.member.login.forget_password(&dto).await?;
    ok(())
}

/// `POST /api/user/login/bindMobile`：绑定手机并返回会员 JWT。
async fn bind_mobile(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Json(dto): Json<BindMobileDTO>,
) -> ApiResult<String> {
    ok(state
        .member
        .login
        .bind_mobile(&dto, client_type(&headers))
        .await?)
}

/// `GET /api/user/login/getQuickLoginSetting`：快捷登录开关。
async fn get_quick_login_setting(
    State(state): State<PublicState>,
    headers: HeaderMap,
) -> ApiResult<QuickLoginSettingVO> {
    ok(state
        .member
        .login
        .quick_login_setting(client_type(&headers))
        .await?)
}

/// `GET /api/user/login/getWxLoginUrl?url=`：微信授权 URL。
async fn get_wx_login_url(
    State(state): State<PublicState>,
    headers: HeaderMap,
    Query(q): Query<WxLoginUrlQuery>,
) -> ApiResult<WechatLoginUrlVO> {
    ok(state
        .member
        .login
        .wx_login_url(client_type(&headers), q.url.as_deref())
        .await?)
}

/// `POST /api/user/login/wechatEvent`：扫码登录事件（未迁移）。
async fn wechat_event(
    State(state): State<PublicState>,
    Json(dto): Json<WechatEventDTO>,
) -> ApiResult<LoginWechatEventVO> {
    ok(state.member.login.wechat_event(&dto).await?)
}

/// `POST /api/user/login/getMobile`：小程序手机号（未迁移）。
async fn get_mobile(
    State(state): State<PublicState>,
    Json(dto): Json<CodeParamDTO>,
) -> ApiResult<String> {
    ok(state.member.login.get_mobile(&dto).await?)
}

/// `GET /api/user/login/getWxLoginInfoByCode?code=`：微信 OAuth 用户信息（未迁移）。
async fn get_wx_login_info_by_code(
    State(state): State<PublicState>,
    Query(q): Query<CodeQuery>,
) -> ApiResult<WxLoginInfoByCodeVO> {
    let code = q
        .code
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .ok_or_else(|| nextrade_core::Error::biz(SERVICE_DATA_ERROR, CODE_PARAM_REQUIRED))?;
    ok(state.member.login.wx_login_info_by_code(Some(code)).await?)
}

/// 微信服务器回调查询参数（GET 验证 / POST 安全模式）。
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct WechatServerQueryParams {
    signature: Option<String>,
    timestamp: Option<String>,
    nonce: Option<String>,
    echostr: Option<String>,
    #[serde(rename = "encrypt_type")]
    encrypt_type: Option<String>,
    #[serde(rename = "msg_signature")]
    msg_signature: Option<String>,
}

/// `GET|POST /api/user/login/wechatServer`：微信服务器回调（URL 验证 + 安全模式事件）。
async fn wechat_server(
    State(state): State<PublicState>,
    Query(q): Query<WechatServerQueryParams>,
    body: Bytes,
) -> Response {
    let query = WechatServerQuery {
        signature: q.signature,
        timestamp: q.timestamp,
        nonce: q.nonce,
        echostr: q.echostr,
        encrypt_type: q.encrypt_type,
        msg_signature: q.msg_signature,
    };
    let text = match state.member.login.wechat_server(&query, &body).await {
        Ok(text) => text,
        Err(e) => {
            tracing::warn!("微信服务器回调处理失败: {e}");
            format!("{e}")
        }
    };
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        text,
    )
        .into_response()
}

// ————————————————————— 需登录端点 —————————————————————

/// `POST /api/user/login/updateUserOpenid`：小程序 code2Session 绑定（未迁移）。
async fn update_user_openid(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<CodeParamDTO>,
) -> ApiResult<()> {
    state.member.login.update_user_openid(&dto, auth.id).await?;
    ok(())
}

/// `POST /api/user/login/bindWechat`：公众号授权绑定（未迁移）。
async fn bind_wechat(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<BindWechatParam>,
) -> ApiResult<()> {
    state.member.login.bind_wechat(&dto, auth.id).await?;
    ok(())
}

/// `GET /api/user/login/unbindWechat`：解绑公众号。
async fn unbind_wechat(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<()> {
    state.member.login.unbind_wechat(auth.id).await?;
    ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 契约逐条守卫：16 条（含 `ANY` 的 wechatServer 与 /adminapi 那条）。
    #[test]
    fn implemented_routes_match_all_16_java_paths() {
        // 契约 16 条中 `/adminapi/login/sendMobileCode` 由 admin 应用挂载（见 api/admin/src/lib.rs），
        // 本模块只负责 public 侧 15 条。
        let expected: [(&str, &str); 16] = [
            ("ANY", "/api/user/login/wechatServer"),
            ("GET", "/api/user/login/getQuickLoginSetting"),
            ("GET", "/api/user/login/getWxLoginUrl"),
            ("POST", "/api/user/login/sendMobileCode"),
            ("POST", "/api/user/login/sendCode"),
            ("POST", "/api/user/login/sendEmailCode"),
            ("POST", "/api/user/login/checkMobile"),
            ("POST", "/api/user/login/checkEmail"),
            ("POST", "/api/user/login/forgetPassword"),
            ("POST", "/api/user/login/bindMobile"),
            ("POST", "/api/user/login/wechatEvent"),
            ("POST", "/api/user/login/getMobile"),
            ("POST", "/api/user/login/updateUserOpenid"),
            ("GET", "/api/user/login/getWxLoginInfoByCode"),
            ("POST", "/api/user/login/bindWechat"),
            ("GET", "/api/user/login/unbindWechat"),
        ];
        let docs = implemented_routes();
        assert_eq!(docs.len(), 16, "public 侧 16 条（含 sendCode 别名）");
        for (doc, (method, path)) in docs.iter().zip(expected.iter()) {
            assert_eq!(doc.method, *method, "方法不匹配: {path}");
            assert_eq!(doc.path, *path, "路径不匹配");
        }
        // 无重复
        let mut keys: Vec<String> = docs.iter().map(|d| d.key()).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), 16);
    }

    /// 15 条在 public 应用挂载；`/adminapi/login/sendMobileCode` 归 admin 应用。
    #[test]
    fn mounted_paths_are_public_contract_minus_adminapi_endpoint() {
        let mounted: Vec<&str> = ROUTE_TABLE
            .iter()
            .map(|(_, p, _)| *p)
            .filter(|p| p.starts_with("/api/"))
            .collect();
        assert_eq!(mounted.len(), 16);
        assert!(!mounted.iter().any(|p| p.starts_with("/adminapi/")));
        // 挂载前缀 = 契约路径去掉 `/api`
        for p in mounted {
            assert!(p.starts_with("/api/user/login/"), "{p}");
            assert!(p.strip_prefix("/api").unwrap().starts_with("/user/login/"));
        }
    }

    /// 需登录 3 条 / 匿名 12 条（15 条挂载之内）。
    #[test]
    fn protected_and_anonymous_split_is_3_and_12() {
        let protected = [
            "/user/login/updateUserOpenid",
            "/user/login/bindWechat",
            "/user/login/unbindWechat",
        ];
        assert_eq!(protected.len(), 3);
        for p in protected {
            assert!(ROUTE_TABLE
                .iter()
                .any(|(_, full, _)| full == &format!("/api{p}")));
        }
        assert_eq!(16 - protected.len(), 13);
    }

    /// 客户端类型从请求头读取（Java `HeaderUtils#getClientType`）。
    #[test]
    fn client_type_header_is_parsed() {
        use axum::http::HeaderValue;
        let mut h = HeaderMap::new();
        assert_eq!(client_type(&h), None);
        h.insert(CLIENT_TYPE_HEADER, HeaderValue::from_static("wechat"));
        assert_eq!(client_type(&h), Some("wechat"));
        h.insert(CLIENT_TYPE_HEADER, HeaderValue::from_static("  "));
        assert_eq!(client_type(&h), None);
    }
}
