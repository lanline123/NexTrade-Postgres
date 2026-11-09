//! Web 层公共组件：统一错误响应、JWT 认证中间件。
//!
//! 对应原 Java：
//! - `GlobalExceptionHandler` → [`AppError`]（`IntoResponse`）
//! - Spring Security `JwtAuthenticationFilter` → [`require_auth`] 中间件

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use nextrade_core::error::ApiResponse;
use nextrade_core::Error;

use crate::JwtService;

/// 从请求头解析客户端 IP（`X-Forwarded-For` 首个地址，其次 `X-Real-IP`）。
///
/// 缺省返回 `"unknown"`；用于管理审计日志，避免把 actor/IP 写成固定值。
#[must_use]
pub fn client_ip(headers: &axum::http::HeaderMap) -> String {
    headers
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            headers
                .get("X-Real-IP")
                .and_then(|v| v.to_str().ok())
                .map(str::trim)
                .filter(|s| !s.is_empty())
        })
        .unwrap_or("unknown")
        .to_string()
}

/// 全局错误响应包装：任意 `nextrade_core::Error` → 标准 JSON 响应。
///
/// ⚠️ 必须派生 `Debug`：测试里 `Result::expect(...)` / `unwrap()` 要求
/// 错误类型实现 `Debug`（否则 E0277: `AppError` doesn't implement `Debug`）。
#[derive(Debug)]
pub struct AppError(pub Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.0.status_code();
        let body: ApiResponse<()> = self.0.into();
        // 业务错误保持 HTTP 200（兼容原 Java ResultWrapper 风格）；
        // 认证/权限/参数错误使用真实 HTTP 状态码。
        (status, Json(body)).into_response()
    }
}

impl From<Error> for AppError {
    fn from(e: Error) -> Self {
        Self(e)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        Self(Error::Internal(e.to_string()))
    }
}

/// `Result<T>` 的 API 层别名。
pub type ApiResult<T> = Result<Json<ApiResponse<T>>, AppError>;

/// 成功响应便捷函数。
pub fn ok<T: serde::Serialize>(data: T) -> ApiResult<T> {
    Ok(Json(ApiResponse::success(data)))
}

/// 已认证身份（写入请求扩展，handler 用 `Extension<AuthUser>` 提取）。
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: i32,
    pub username: String,
    pub role_id: Option<i32>,
    pub admin_type: Option<String>,
    /// 店铺 ID（0 或 None 为平台管理员；>0 为入驻商户/门店）
    pub shop_id: Option<i32>,
}

impl AuthUser {
    /// 构造包含店铺信息的完整身份
    pub fn new(
        id: i32,
        username: String,
        role_id: Option<i32>,
        admin_type: Option<String>,
        shop_id: Option<i32>,
    ) -> Self {
        Self {
            id,
            username,
            role_id,
            admin_type,
            shop_id,
        }
    }

    /// 构造普通会员/前台商城身份（无关联商户店铺）
    pub fn member(id: i32, username: String) -> Self {
        Self {
            id,
            username,
            role_id: None,
            admin_type: Some("member".into()),
            shop_id: None,
        }
    }

    /// 是否为平台总管理账号（shop_id 为 0 或 None，且不是店铺/供应商账号类型）
    #[must_use]
    pub fn is_platform(&self) -> bool {
        match self.admin_type.as_deref() {
            Some("shop" | "store" | "pos" | "merchant" | "vendor" | "suppliers" | "supplier") => {
                false
            }
            _ => self.shop_id.unwrap_or(0) == 0,
        }
    }

    /// 是否为供应商/供货商账号。
    #[must_use]
    pub fn is_vendor(&self) -> bool {
        matches!(
            self.admin_type.as_deref(),
            Some("vendor" | "suppliers" | "supplier")
        )
    }

    /// 获取需要进行租户隔离的店铺 ID
    /// - 平台账号返回 None（查询不限制店铺）
    /// - 商户账号返回 Some(shop_id)（必须限制 WHERE shop_id = ?）
    /// - 供应商账号没有店铺维度，返回 `Some(0)` 作为**安全兜底**，避免其
    ///   误用店铺接口时拿到全量数据；供应商接口应改用 `TenantContext::vendor_scope`。
    #[must_use]
    pub fn scoped_shop_id(&self) -> Option<i32> {
        if self.is_platform() {
            None
        } else if self.is_vendor() {
            Some(0)
        } else {
            self.shop_id.filter(|&id| id > 0)
        }
    }

    /// 解析查询/写入请求中的 `shopId`：
    /// - 店铺账号只能使用自己的 shop_id，传入其他值直接 403；
    /// - 平台账号可使用请求值（列表筛选/代运营），未传则不限制。
    pub fn resolve_shop_scope(&self, requested: Option<i32>) -> Result<Option<i32>, Error> {
        if let Some(scope) = self.scoped_shop_id() {
            if let Some(requested) = requested.filter(|id| *id > 0 && *id != scope) {
                return Err(Error::Forbidden(format!(
                    "无权访问店铺 {requested} 的数据"
                )));
            }
            return Ok(Some(scope));
        }
        Ok(requested.filter(|id| *id > 0))
    }

    /// 写入必须落到一个具体店铺：店铺账号固定为自己，平台账号必须显式传 shopId。
    pub fn require_shop_scope(&self, requested: Option<i32>) -> Result<i32, Error> {
        if let Some(scope) = self.scoped_shop_id() {
            if let Some(requested) = requested.filter(|id| *id > 0 && *id != scope) {
                return Err(Error::Forbidden(format!(
                    "无权写入店铺 {requested} 的数据"
                )));
            }
            return Ok(scope);
        }
        requested
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::Forbidden("平台账号执行该写操作时必须指定 shopId".into()))
    }

    /// 校验对指定店铺的访问权限（平台放行）。
    pub fn ensure_shop_access(&self, shop_id: i32) -> Result<(), Error> {
        match self.scoped_shop_id() {
            Some(scope) if scope != shop_id => Err(Error::Forbidden(format!(
                "无权访问店铺 {shop_id} 的数据"
            ))),
            _ => Ok(()),
        }
    }
}

/// 统一租户上下文（来自 JWT，不信任客户端请求头）。
///
/// - 平台管理员：`shop_scope()/vendor_scope()` 均为 None；
/// - 店铺/门店/POS：`shop_scope()` 为店铺 ID；
/// - 供应商/供货商：`vendor_scope()` 为供应商 ID。
#[derive(Debug, Clone, Default)]
pub struct TenantContext {
    pub admin_type: Option<String>,
    pub shop_id: Option<i32>,
    pub vendor_id: Option<i32>,
}

impl TenantContext {
    #[must_use]
    pub fn new(
        admin_type: Option<String>,
        shop_id: Option<i32>,
        vendor_id: Option<i32>,
    ) -> Self {
        Self {
            admin_type,
            shop_id: shop_id.filter(|id| *id > 0),
            vendor_id: vendor_id.filter(|id| *id > 0),
        }
    }

    #[must_use]
    pub fn is_shop_account(&self) -> bool {
        matches!(
            self.admin_type.as_deref(),
            Some("shop" | "store" | "pos" | "merchant")
        )
    }

    #[must_use]
    pub fn is_vendor_account(&self) -> bool {
        matches!(
            self.admin_type.as_deref(),
            Some("vendor" | "suppliers" | "supplier")
        )
    }

    #[must_use]
    pub fn is_platform(&self) -> bool {
        !self.is_shop_account() && !self.is_vendor_account()
    }

    /// 店铺隔离 ID：仅店铺/门店/POS 账号返回 Some。
    #[must_use]
    pub fn shop_scope(&self) -> Option<i32> {
        if self.is_shop_account() {
            self.shop_id
        } else {
            None
        }
    }

    /// 供应商隔离 ID：仅供应商/供货商账号返回 Some。
    #[must_use]
    pub fn vendor_scope(&self) -> Option<i32> {
        if self.is_vendor_account() {
            self.vendor_id
        } else {
            None
        }
    }

    /// 店铺账号必须带店铺 ID，否则拒绝（防止配置错误的账号拿到全量数据）。
    pub fn require_shop(&self) -> Result<i32, Error> {
        if self.is_platform() {
            return Err(Error::Forbidden("平台账号不能执行店铺写操作".into()));
        }
        self.shop_scope()
            .ok_or_else(|| Error::Forbidden("缺少店铺租户上下文".into()))
    }

    /// 判断当前租户能否访问指定店铺（平台放行）。
    #[must_use]
    pub fn can_access_shop(&self, shop_id: i32) -> bool {
        match self.shop_scope() {
            Some(scope) => scope == shop_id,
            None => self.is_platform(),
        }
    }

    /// 判断当前租户能否访问指定供应商（平台放行）。
    #[must_use]
    pub fn can_access_vendor(&self, vendor_id: i32) -> bool {
        match self.vendor_scope() {
            Some(scope) => scope == vendor_id,
            None => self.is_platform(),
        }
    }
}

/// 认证中间件：校验 `Authorization: Bearer <JWT>`，
/// 通过后将已认证身份写入请求扩展。
///
/// 对应原 Java `JwtAuthenticationFilter.doFilterInternal` + `HeaderUtils.getShopId()`。
pub async fn require_auth(
    State(jwt): State<JwtService>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // 前置认证中间件（如管理端 `X-Api-Token`）已注入身份时直接放行，
    // 对应 Java `ApiTokenAuthenticationFilter` 命中后短路 JWT 过滤链的语义。
    if req.extensions().get::<AuthUser>().is_some() {
        return Ok(next.run(req).await);
    }
    let header_token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    // 原版前端在部分直达页/刷新场景只带登录 cookie，不带 Authorization；
    // header 缺失或为 `Bearer null` 时回退读取 cookie，避免误报未登录。
    let cookie_token: Option<String> = req
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|raw| {
            raw.split(';').find_map(|pair| {
                let (name, value) = pair.split_once('=')?;
                matches!(name.trim(), "accessToken" | "token" | "adminToken")
                    .then(|| value.trim().trim_matches('"').to_string())
                    .filter(|v| !v.is_empty() && v != "null")
            })
        });
    let cookie_bearer = cookie_token.map(|t| format!("Bearer {t}"));
    let token = match header_token {
        Some(t) if !t.trim().eq_ignore_ascii_case("bearer null") && !t.trim().is_empty() => Some(t),
        _ => cookie_bearer.as_deref(),
    };

    if let Some(raw) = token {
        let bare = raw.trim().strip_prefix("Bearer ").unwrap_or(raw.trim());
        if crate::token_revocation::is_revoked(bare) {
            return Err(AppError(Error::Unauthorized("登录已失效，请重新登录".into())));
        }
    }
    let claims = jwt.from_authorization(token)?;
    let id = claims
        .sub
        .parse::<i32>()
        .map_err(|_| Error::Unauthorized("令牌主体无效".into()))?;

    // JWT 声明是唯一可信来源：x-admin-type / x-shop-id / x-vendor-id 只作为
    // 旧令牌（无 typ/shop 声明）的兼容回退，绝不允许覆盖已签发声明。
    let header_shop_id = req
        .headers()
        .get("x-shop-id")
        .or_else(|| req.headers().get("X-Shop-Id"))
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<i32>().ok());

    let header_admin_type = req
        .headers()
        .get("x-admin-type")
        .or_else(|| req.headers().get("X-Admin-Type"))
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    let legacy_token = claims.typ.is_none() && claims.shop_id.is_none();
    let effective_admin_type = claims.typ.clone().or_else(|| {
        if legacy_token {
            header_admin_type
        } else {
            None
        }
    });
    let effective_shop_id = claims.shop_id.or_else(|| {
        if legacy_token {
            header_shop_id
        } else {
            None
        }
    });
    let effective_vendor_id = claims.vendor_id;

    // 用可信值覆盖请求头，供尚未迁移到 Extension 的 Handler 读取，
    // 防止客户端伪造 x-shop-id / x-vendor-id 越权。
    {
        let headers = req.headers_mut();
        headers.remove("x-shop-id");
        headers.remove("X-Shop-Id");
        headers.remove("x-admin-type");
        headers.remove("X-Admin-Type");
        headers.remove("x-vendor-id");
        headers.remove("X-Vendor-Id");
        if let Some(shop_id) = effective_shop_id.filter(|id| *id > 0) {
            if let Ok(value) = axum::http::HeaderValue::from_str(&shop_id.to_string()) {
                headers.insert("x-shop-id", value);
            }
        }
        if let Some(admin_type) = effective_admin_type.as_deref() {
            if let Ok(value) = axum::http::HeaderValue::from_str(admin_type) {
                headers.insert("x-admin-type", value);
            }
        }
        if let Some(vendor_id) = effective_vendor_id.filter(|id| *id > 0) {
            if let Ok(value) = axum::http::HeaderValue::from_str(&vendor_id.to_string()) {
                headers.insert("x-vendor-id", value);
            }
        }
    }

    let tenant = TenantContext::new(
        effective_admin_type.clone(),
        effective_shop_id,
        effective_vendor_id,
    );
    req.extensions_mut().insert(tenant);
    req.extensions_mut().insert(AuthUser {
        id,
        username: claims.usr,
        role_id: claims.role,
        admin_type: effective_admin_type,
        shop_id: effective_shop_id,
    });
    let ip = client_ip(req.headers());
    let actor = crate::audit::AuditActor {
        admin_id: id,
        ip,
    };
    Ok(crate::audit::with_actor(actor, next.run(req)).await)
}

// ————————————————————— 内部接口密钥校验 —————————————————————

/// 内部 API 密钥请求头（Java `InternalApiFilter.INTERNAL_API_KEY_HEADER`）。
pub const INTERNAL_API_KEY_HEADER: &str = "X-Internal-Api-Key";

/// 内部密钥校验结果：`(HTTP 状态码, 文案)`，与 Java `InternalApiFilter` 逐条对齐。
#[must_use]
pub fn check_internal_api_key(
    configured: &str,
    provided: Option<&str>,
) -> Result<(), (u16, &'static str)> {
    if configured.trim().is_empty() {
        return Err((403, "内部接口未正确配置"));
    }
    let Some(provided) = provided.map(str::trim).filter(|v| !v.is_empty()) else {
        return Err((401, "缺少内部 API 密钥"));
    };
    if provided != configured {
        return Err((401, "内部 API 密钥验证失败"));
    }
    Ok(())
}

/// 内部接口密钥中间件（对应 Java `InternalApiFilter`，保护 `/api/socket/send`）。
///
/// 未配置密钥 → 403；缺头 → 401；密钥不符 → 401。
/// 错误体与 Java 一致：`{"code":<status>,"message":…,"success":false}`。
/// 通过后放行后续链（如 \"optional_auth\"）。
pub async fn require_internal_api_key(
    State(configured): State<String>,
    req: Request,
    next: Next,
) -> Response {
    let provided = req
        .headers()
        .get(INTERNAL_API_KEY_HEADER)
        .and_then(|v| v.to_str().ok());
    if let Err((status, message)) = check_internal_api_key(&configured, provided) {
        let code =
            StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let path = req.uri().path().to_string();
        tracing::warn!("内部 API 密钥校验未通过：{message}（{path}）");
        return (
            code,
            Json(serde_json::json!({
                "code": status,
                "message": message,
                "success": false,
            })),
        )
            .into_response();
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_api_key_matches_java_filter_semantics() {
        // 未配置密钥 → 403「内部接口未正确配置」
        assert_eq!(
            check_internal_api_key("", Some("k")).unwrap_err(),
            (403, "内部接口未正确配置")
        );
        // 缺头 / 空头 → 401「缺少内部 API 密钥」
        assert_eq!(
            check_internal_api_key("secret", None).unwrap_err(),
            (401, "缺少内部 API 密钥")
        );
        assert_eq!(
            check_internal_api_key("secret", Some("  ")).unwrap_err(),
            (401, "缺少内部 API 密钥")
        );
        // 密钥不符 → 401「内部 API 密钥验证失败」
        assert_eq!(
            check_internal_api_key("secret", Some("wrong")).unwrap_err(),
            (401, "内部 API 密钥验证失败")
        );
        // 正确密钥（含首尾空白）→ 放行
        assert!(check_internal_api_key("secret", Some("secret")).is_ok());
        assert!(check_internal_api_key("secret", Some(" secret ")).is_ok());
    }

    #[test]
    fn auth_user_tenant_isolation_rules() {
        // Platform admin (typ = "admin", shop_id = 0 or None)
        let platform_admin = AuthUser {
            id: 1,
            username: "admin".into(),
            role_id: Some(1),
            admin_type: Some("admin".into()),
            shop_id: Some(0),
        };
        assert!(platform_admin.is_platform());
        assert_eq!(platform_admin.scoped_shop_id(), None);

        // Merchant admin (typ = "merchant", shop_id = 88)
        let merchant_admin = AuthUser {
            id: 2,
            username: "vendor88".into(),
            role_id: Some(2),
            admin_type: Some("merchant".into()),
            shop_id: Some(88),
        };
        assert!(!merchant_admin.is_platform());
        assert_eq!(merchant_admin.scoped_shop_id(), Some(88));

        // Unknown/unspecified type with shop_id > 0
        let other_vendor = AuthUser {
            id: 3,
            username: "vendor99".into(),
            role_id: None,
            admin_type: None,
            shop_id: Some(99),
        };
        assert!(!other_vendor.is_platform());
        assert_eq!(other_vendor.scoped_shop_id(), Some(99));

        // Superadmin with no shop_id and no typ
        let superadmin = AuthUser {
            id: 1,
            username: "super".into(),
            role_id: Some(1),
            admin_type: None,
            shop_id: None,
        };
        assert!(superadmin.is_platform());
        assert_eq!(superadmin.scoped_shop_id(), None);
    }

    #[test]
    fn shop_scope_rejects_forged_requested_shop() {
        let shop = AuthUser {
            id: 2,
            username: "shop9".into(),
            role_id: None,
            admin_type: Some("shop".into()),
            shop_id: Some(9),
        };
        assert_eq!(shop.resolve_shop_scope(None).unwrap(), Some(9));
        assert_eq!(shop.resolve_shop_scope(Some(9)).unwrap(), Some(9));
        assert!(shop.resolve_shop_scope(Some(10)).is_err());
        assert_eq!(shop.require_shop_scope(None).unwrap(), 9);
        assert!(shop.require_shop_scope(Some(10)).is_err());
        assert!(shop.ensure_shop_access(10).is_err());

        let platform = AuthUser {
            id: 1,
            username: "admin".into(),
            role_id: None,
            admin_type: Some("admin".into()),
            shop_id: None,
        };
        assert_eq!(platform.resolve_shop_scope(Some(10)).unwrap(), Some(10));
        assert!(platform.require_shop_scope(None).is_err());
        assert_eq!(platform.require_shop_scope(Some(10)).unwrap(), 10);

        let vendor = AuthUser {
            id: 3,
            username: "vendor".into(),
            role_id: None,
            admin_type: Some("vendor".into()),
            shop_id: None,
        };
        assert!(!vendor.is_platform());
        assert!(vendor.is_vendor());
        // 供应商误用店铺接口时不能放行全量数据
        assert_eq!(vendor.scoped_shop_id(), Some(0));
    }

    #[test]
    fn tenant_context_scopes_shop_and_vendor() {
        let platform = TenantContext::new(Some("admin".into()), None, None);
        assert!(platform.is_platform());
        assert_eq!(platform.shop_scope(), None);
        assert_eq!(platform.vendor_scope(), None);
        assert!(platform.can_access_shop(9));

        let shop = TenantContext::new(Some("shop".into()), Some(9), None);
        assert_eq!(shop.shop_scope(), Some(9));
        assert_eq!(shop.require_shop().unwrap(), 9);
        assert!(!shop.can_access_shop(10));
        assert!(shop.can_access_shop(9));
        assert!(!shop.can_access_vendor(3));

        let vendor = TenantContext::new(Some("vendor".into()), None, Some(3));
        assert_eq!(vendor.vendor_scope(), Some(3));
        assert!(vendor.can_access_vendor(3));
        assert!(!vendor.can_access_vendor(4));
        assert!(vendor.require_shop().is_err());
    }

    #[test]
    fn client_ip_prefers_forwarded_for() {
        let mut headers = axum::http::HeaderMap::new();
        assert_eq!(client_ip(&headers), "unknown");
        headers.insert("X-Real-IP", "10.0.0.2".parse().unwrap());
        assert_eq!(client_ip(&headers), "10.0.0.2");
        headers.insert("X-Forwarded-For", "1.2.3.4, 5.6.7.8".parse().unwrap());
        assert_eq!(client_ip(&headers), "1.2.3.4");
    }
}"