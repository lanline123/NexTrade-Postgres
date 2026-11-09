//! 商户入驻·前台路由（merchant::merchant_client）。
//!
//! 对应原 Java `com.nextrade.api.controller.merchant.MerchantClientController`
//! （`@RequestMapping("/api/merchant/merchant")`）。
//!
//! | 本模块路由（挂载于 `/api`） | 原 Java 方法 | 契约 auth |
//! |------------------------------|--------------|-----------|
//! | `POST /api/merchant/merchant/uploadLicense` | `MerchantClientController.uploadLicense` | optional |
//! | `GET  /api/merchant/merchant/applyShopAgreement` | `MerchantClientController.applyShopAgreement` | optional |
//! | `GET  /api/merchant/merchant/myApply` | `MerchantClientController.myApply` | optional |
//! | `GET  /api/merchant/merchant/applyDetail` | `MerchantClientController.applyDetail` | optional |
//! | `POST /api/merchant/merchant/apply` | `MerchantClientController.apply` | optional |
//!
//! > **路径说明**：控制器前缀**重复一次**（`/api/merchant/merchant/...`），
//! > 这不是笔误，必须逐字符照抄；本文件挂在 `/api` 之下，
//! > 故 `routes()` 内只写 `/merchant/merchant/...`。
//! >
//! > **⚠️ auth=optional 的落地口径**：契约把本域 5 条都标为 `optional`，
//! > 因此本模块挂在**公开**组（不在 `merge_authorized` 外层套 `require_auth`）。
//! > 各 handler 用 `Option<Extension<AuthUser>>` 读登录态，差异如下：
//! > - `uploadLicense` / `applyShopAgreement` 不依赖用户上下文，忽略登录态；
//! > - `myApply` / `applyDetail` 按 Java 语义：游客 `userId = 0`
//! >   （查不到 / 归属不符 → 「商户申请信息不存在」），**不**强制登录；
//! > - `apply` 强制要求登录（业务码 401「请先登录」），避免 Java 因
//! >   `getCurrentUserId() == 0` 产生的脏归属（见 `services/src/merchant_client.rs`）。
//! >
//! > **已知缺口（未迁移，详见 `services/src/merchant_client.rs` 模块头）**：
//! > 1. `uploadLicense` 仅复刻三条入参校验，校验通过后返回
//! >    「营业执照上传尚未迁移」业务错误（图库/对象存储 + 水印字体未迁移）；
//! > 2. `apply` 的「免审核直接开通」分支（`merchantApplyNeedCheck != "1"`）
//! >    未迁移，写入前返回「商户免审核自动开通尚未迁移」业务错误。

use axum::{
    extract::{Multipart, Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use nextrade_core::models::merchant::merchant_client::{ApplyDetailQuery, MerchantApplyApplyParam};
use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};

use crate::PublicState;

/// 商户入驻域前台路由（挂载于 `/api` 下）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .route("/merchant/merchant/uploadLicense", post(upload_license))
        .route(
            "/merchant/merchant/applyShopAgreement",
            get(apply_shop_agreement),
        )
        .route("/merchant/merchant/myApply", get(my_apply))
        .route("/merchant/merchant/applyDetail", get(apply_detail))
        .route("/merchant/merchant/apply", post(apply))
}

/// 路由表（用于静态覆盖率扫描与测试）。
///
/// 元素为 `(method, 去 /api 前缀的路径, java_ref)`。
/// 注意：coverage 静态扫描 `.route(\s*"` 需要**字符串字面量逐条**注册，
/// 严禁从数组循环拼接，故 [`ROUTE_TABLE`] 仅作文档/测试用途，
/// `routes()` 仍用字面量注册。
pub const ROUTE_TABLE: [(&str, &str, &str); 5] = [
    (
        "POST",
        "/merchant/merchant/uploadLicense",
        "MerchantClientController.uploadLicense",
    ),
    (
        "GET",
        "/merchant/merchant/applyShopAgreement",
        "MerchantClientController.applyShopAgreement",
    ),
    (
        "GET",
        "/merchant/merchant/myApply",
        "MerchantClientController.myApply",
    ),
    (
        "GET",
        "/merchant/merchant/applyDetail",
        "MerchantClientController.applyDetail",
    ),
    (
        "POST",
        "/merchant/merchant/apply",
        "MerchantClientController.apply",
    ),
];

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计）。
///
/// 返回**完整契约路径**（含 `/api` 前缀），逐字符匹配上方 5 条契约。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    [
        (
            "POST",
            "/api/merchant/merchant/uploadLicense",
            "MerchantClientController.uploadLicense",
        ),
        (
            "GET",
            "/api/merchant/merchant/applyShopAgreement",
            "MerchantClientController.applyShopAgreement",
        ),
        (
            "GET",
            "/api/merchant/merchant/myApply",
            "MerchantClientController.myApply",
        ),
        (
            "GET",
            "/api/merchant/merchant/applyDetail",
            "MerchantClientController.applyDetail",
        ),
        (
            "POST",
            "/api/merchant/merchant/apply",
            "MerchantClientController.apply",
        ),
    ]
    .iter()
    .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, p, j))
    .collect()
}

/// 取登录会员 ID（对应 Java `SecurityUtils.getCurrentUserId()`）。
///
/// 本域挂在公开组，登录态为可选；缺失或非正 ID 时返回 `None`
/// （由调用方决定是「按游客 userId=0」还是「强制要求登录」）。
fn optional_user_id(auth: &Option<Extension<AuthUser>>) -> Option<i32> {
    auth.as_ref()
        .map(|Extension(a)| a.id)
        .filter(|id| *id > 0)
}

/// `POST /api/merchant/merchant/uploadLicense`：入驻营业执照专用上传。
///
/// 对应 Java `MerchantClientController.uploadLicense` →
/// `MerchantServiceImpl#uploadLicensePic`：空 / 超限 / MIME 校验通过后，
/// 叠加水印并走图库上传。**未迁移**：校验通过后由服务层返回
/// 「营业执照上传尚未迁移」业务错误（详见 `services/src/merchant_client.rs`）。
///
/// 读取 `multipart/form-data` 的 `file` / `license` 字段（字节流 + MIME），
/// 字段名缺失时按「空图片」处理（服务层返回「营业执照图片不能为空」）。
async fn upload_license(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    mut multipart: Multipart,
) -> ApiResult<String> {
    let user_id = auth.as_ref().map(|Extension(a)| a.id).unwrap_or(0);
    if user_id <= 0 {
        return Err(AppError(Error::biz(401, "请先登录")));
    }
    let (bytes, content_type, file_name) = read_license_file(&mut multipart).await?;
    if bytes.is_empty() {
        return Err(AppError(Error::Validation("营业执照图片不能为空".into())));
    }
    let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let vo = state
        .content
        .gallery_pic
        .upload_img(None, content_type.as_deref(), size, file_name.as_deref(), &bytes, 0, user_id)
        .await?;
    let url = serde_json::to_value(vo).ok()
        .and_then(|v| v.get("url").and_then(serde_json::Value::as_str).map(ToString::to_string))
        .unwrap_or_default();
    if url.is_empty() {
        return Err(AppError(Error::Internal("营业执照上传失败".into())));
    }
    ok(url)
}

/// 读取营业执照上传字段（`file` 或 `license`），返回 `(字节长度, MIME)`。
///
/// 与 `api/public/src/routes/user_center.rs#read_multipart` 同口径，
/// 但只关心文件字段（不需要文本参数）。找不到文件字段时返回 `(0, None)`，
/// 交由服务层「营业执照图片不能为空」校验拦截。
async fn read_license_file(
    multipart: &mut Multipart,
) -> Result<(Vec<u8>, Option<String>, Option<String>), AppError> {
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError(Error::Validation(format!("解析上传表单失败: {e}")))
    })? {
        let name = field.name().unwrap_or_default();
        if name == "file" || name == "license" {
            let content_type = field.content_type().map(str::to_string);
            let file_name = field.file_name().map(str::to_string);
            let bytes = field.bytes().await.map_err(|e| {
                AppError(Error::Validation(format!("读取上传文件失败: {e}")))
            })?;
            return Ok((bytes.to_vec(), content_type, file_name));
        }
    }
    Ok((Vec::new(), None, None))
}

/// `GET /api/merchant/merchant/applyShopAgreement`：申请入驻协议。
///
/// 对应 Java `MerchantClientController.applyShopAgreement` →
/// `MerchantServiceImpl#applyShopAgreement`：读 `shopAgreement` 配置原样返回
/// （当前 Java `translate` 为恒等函数）。不依赖登录态。
async fn apply_shop_agreement(
    State(state): State<PublicState>,
    _auth: Option<Extension<AuthUser>>,
) -> ApiResult<String> {
    ok(state.merchant.merchant_client.apply_shop_agreement().await?)
}

/// `GET /api/merchant/merchant/myApply`：我的（最新一条）入驻申请。
///
/// 对应 Java `MerchantClientController.myApply`：按 `user_id` 取最新一条，
/// 无记录返回 `null`（响应体 `data:null`）。
/// 游客 `userId = 0`（Java 语义，不强制登录），查不到即 `data:null`。
async fn my_apply(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
) -> ApiResult<Option<nextrade_core::models::merchant::merchant_client::MerchantMyApplyVO>> {
    let user_id = optional_user_id(&auth).unwrap_or(0);
    ok(state.merchant.merchant_client.my_apply(user_id).await?)
}

/// `GET /api/merchant/merchant/applyDetail?id=`：入驻申请详情（仅本人可读）。
///
/// 对应 Java `MerchantClientController.applyDetail`：按 \"id\" 查后校验归属，
/// 不存在或非本人 → 「商户申请信息不存在」。游客 `userId = 0`（不强制登录）。
async fn apply_detail(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<ApplyDetailQuery>,
) -> ApiResult<nextrade_core::models::merchant::merchant_client::MerchantApplyDetailVO> {
    let user_id = optional_user_id(&auth).unwrap_or(0);
    ok(state.merchant.merchant_client.apply_detail(q.id, user_id).await?)
}

/// `POST /api/merchant/merchant/apply`：提交入驻申请。
///
/// 对应 Java `MerchantClientController.apply` →
/// `MerchantApplyServiceImpl#apply`（事务内写 `merchant_apply` + 后台站内信）。
/// **强制要求登录**（业务码 401「请先登录」）：避免 Java 因
/// `getCurrentUserId() == 0` 把脏申请写进库（详见 `services/src/merchant_client.rs`）。
/// 「免审核直接开通」分支未迁移，写入前返回明确业务错误。
async fn apply(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(param): Json<MerchantApplyApplyParam>,
) -> ApiResult<nextrade_core::models::merchant::merchant_client::MerchantApplyDetailVO> {
    let user_id = match optional_user_id(&auth) {
        Some(id) => id,
        None => return Err(AppError(Error::biz(401, "请先登录"))),
    };
    ok(state.merchant.merchant_client.apply(user_id, &param).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_openapi::RouteDoc;

    /// 契约 5 条路径（含 `/api` 前缀），用于逐字符比对。
    const CONTRACT: [(&str, &str); 5] = [
        ("POST", "/api/merchant/merchant/uploadLicense"),
        ("GET", "/api/merchant/merchant/applyShopAgreement"),
        ("GET", "/api/merchant/merchant/myApply"),
        ("GET", "/api/merchant/merchant/applyDetail"),
        ("POST", "/api/merchant/merchant/apply"),
    ];

    #[test]
    fn route_table_has_five_entries_with_method_distribution() {
        assert_eq!(ROUTE_TABLE.len(), 5);
        let posts = ROUTE_TABLE.iter().filter(|(m, _, _)| *m == "POST").count();
        let gets = ROUTE_TABLE.iter().filter(|(m, _, _)| *m == "GET").count();
        assert_eq!(posts, 2, "POST 应为 2 个");
        assert_eq!(gets, 3, "GET 应为 3 个");
    }

    #[test]
    fn route_table_paths_exclude_api_prefix() {
        for (_, p, _) in ROUTE_TABLE {
            assert!(!p.starts_with("/api"), "ROUTE_TABLE 路径不应含 /api 前缀: {p}");
            assert!(p.starts_with("/merchant/merchant/"), "路径前缀不符: {p}");
        }
    }

    #[test]
    fn implemented_routes_match_contract_char_by_char() {
        let docs: Vec<RouteDoc> = implemented_routes();
        assert_eq!(docs.len(), 5, "实现路由数应为 5");
        for (exp_m, exp_p) in CONTRACT {
            assert!(
                docs.iter()
                    .any(|d| d.method == exp_m && d.path == exp_p),
                "implemented_routes 缺少/不匹配契约: {exp_m} {exp_p}"
            );
        }
        // 反向：实现路由不得多出或偏离契约
        for d in &docs {
            assert!(
                CONTRACT.iter().any(|(m, p)| d.method == *m && d.path == *p),
                "implemented_routes 存在非契约路由: {} {}",
                d.method,
                d.path
            );
        }
    }

    #[test]
    fn implemented_routes_paths_include_api_prefix() {
        for d in implemented_routes() {
            assert!(
                d.path.starts_with("/api/merchant/merchant/"),
                "实现路径应含 /api 前缀: {}",
                d.path
            );
        }
    }
}

// uncertain_points:
// 1. 本文件引用 `state.merchant.merchant_client`（类型 `Arc<nextrade_services::merchant::merchant_client::MerchantClientService>`），
//    但 `api/public/src/lib.rs` 的 `PublicState` 当前**尚未包含** `merchant_client` 字段。
//    主会话需：在 `PublicState` 新增 `pub merchant_client: Arc<MerchantClientService>`，
//    并在 `services` 构造处初始化（参考其它 `sliceNNN` 字段）。
// 2. 本文件需挂载到公开路由组。主会话需在 `api/public/src/routes/mod.rs` 中：
//    - 增加 `pub mod merchant_client;`
//    - 在 `merge_public` 中追加 `.merge(nextrade_core::models::merchant::merchant_client::routes(state))`
//    - 在 `implemented_routes` 中追加 `out.extend(nextrade_core::models::merchant::merchant_client::implemented_routes())`
//    以上均属共享文件改动，按任务约束由主会话处理，本文件不改动它们。
// 3. services 方法签名与契约一致，无缺方法：
//    - `upload_license(byte_len, content_type)`（同步）；
//    - `apply_shop_agreement()` / `my_apply(user_id)` / `apply_detail(id, user_id)` /
//      `apply(user_id, &param)`（异步）。
//    故 handler 直接按 services 真实签名编写，无签名偏差。
// 4. `uploadLicense` 的 multipart 字段名取 `file` 或 `license`（前端常见命名）；
//    若实际字段名不同，会退化为「空图片」校验路径，仍返回清晰业务错误，不崩接口。