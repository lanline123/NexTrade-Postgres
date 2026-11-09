//! 商城前台用户中心路由（member::user_center）。
//!
//! 对应原 Java `com.nextrade.api.controller.user.UserController`
//! （`@RequestMapping("/api/user/user")`）。
//!
//! | 本模块路由（挂载于 `/api`） | 原 Java | 权限 |
//! |------------------------------|---------|------|
//! | `GET  /user/user/creditDetail` | `UserController.creditDetail` | 登录 |
//! | `POST /user/user/updateInformation` | `UserController.updateInformation` | 登录 |
//! | `GET  /user/user/memberCenter` | `UserController.memberCenter` | 登录 |
//! | `POST /user/user/sendMobileCodeByModifyPassword` | `UserController.sendMobileCodeByModifyPassword` | 登录 |
//! | `POST /user/user/checkModifyPasswordMobileCode` | `UserController.checkModifyPasswordMobileCode` | 登录 |
//! | `POST /user/user/sendMobileCodeByMobileValidate` | `UserController.sendMobileCodeByMobileValidate` | 登录 |
//! | `POST /user/user/sendEmailCodeByEmailValidate` | `UserController.sendEmailCodeByEmailValidate` | 登录 |
//! | `POST /user/user/sendMobileCodeByModifyMobile` | `UserController.sendMobileCodeByModifyMobile` | 登录 |
//! | `POST /user/user/sendEmailCodeByModifyEmail` | `UserController.sendEmailCodeByModifyEmail` | 登录 |
//! | `POST /user/user/modifyMobile` | `UserController.modifyMobile` | 登录 |
//! | `POST /user/user/modifyEmail` | `UserController.modifyEmail` | 登录 |
//! | `POST /user/user/mobileValidate` | `UserController.mobileValidate` | 登录 |
//! | `POST /user/user/emailValidateNew` | `UserController.emailValidateNew` | 登录 |
//! | `POST /user/user/emailValidate` | `UserController.emailValidate` | 登录 |
//! | `GET  /user/user/historyProduct` | `UserController.historyProduct` | 登录 |
//! | `POST /user/user/delHistoryProduct` | `UserController.delHistoryProduct` | 登录 |
//! | `POST /user/user/modifyAvatar` | `UserController.modifyAvatar` | 登录 |
//! | `GET  /user/user/collectionShop` | `UserController.collectionShop` | 登录 |
//! | `POST /user/user/logout` | `UserController.logout` | 登录（未迁移：服务端吊销） |
//! | `POST /user/user/close` | `UserController.close` | 登录 |
//! | `POST /user/user/uploadImg` | `UserController.uploadImg` | 游客可访问 |
//! | `POST /user/user/uploadVideo` | `UserController.uploadVideo` | 游客可访问 |
//! | `GET  /user/user/levelList` | `UserController.levelList` | 游客可访问 |
//! | `GET  /user/user/levelInfo` | `UserController.levelInfo` | 游客可访问 |
//!
//! > **路径说明**：控制器前缀**重复一次**（`/api/user/user/...`），
//! > 这不是笔误，必须逐字符照抄。
//!
//! > **认证说明**：接口契约把本组标为 `auth = optional`，但 Java 实现里
//! > 除 `uploadImg` / `uploadVideo` / `levelList` / `levelInfo` 外**全都**调用
//! > `SecurityUtils.getCurrentUserId()`（未登录直接抛异常）。因此本模块把 20 条
//! > 需要登录的接口放在**自带 `require_auth` 中间件**的子路由里，
//! > 4 条游客可用接口放在无鉴权子路由里（`routes/mod.rs` 的约定：
//! > 「需登录的接口由各域文件自行套 `require_auth` 中间件」）。
//! > **合并注意**：本模块必须并入 `merge_public`（外层不要再套 `require_auth`），
//! > 否则 `levelList` / `levelInfo` / `uploadImg` / `uploadVideo` 会错误地要求登录。
//!
//! > **未迁移**（详见 `nextrade_services::member::user_center` 模块注释）：
//! > 短信/邮件验证码 9 条链路、文件上传 3 条链路、`logout`（服务端令牌吊销）
//! > 均返回明确的业务错误，其余接口按 Java 语义实现。

use std::collections::HashMap;

use axum::{
    extract::{Multipart, Query, State},
    middleware,
    routing::{get, post},
    Extension, Json, Router,
};
use nextrade_core::models::member::user_center::{
    CheckEmailModifyDTO, CheckModifyEmailDTO, CheckModifyPasswordDTO, CollectShopQuery, CollectShopVO,
    CreditDetailVO, DelHistoryProductDTO, GalleryPicUploadVO, HistoryProductVO, MemberCenterVO,
    ModifyEmailParam, ModifyMobileParam, RegisterEmailDTO, RegisterSmsDTO, UpdateInformationDTO,
    UploadVideoInfoVO, UserLevelInfoVO, UserLevelListVO,
};
use nextrade_core::types::PageResult;
use nextrade_security::web::{ok, require_auth, ApiResult, AppError, AuthUser};

use crate::PublicState;

/// 用户中心域前台路由（挂载于 `/api` 下）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    let authorized = Router::new()
        .route("/user/user/creditDetail", get(credit_detail))
        .route("/user/user/updateInformation", post(update_information))
        .route("/user/user/memberCenter", get(member_center))
        .route(
            "/user/user/sendMobileCodeByModifyPassword",
            post(send_mobile_code_by_modify_password),
        )
        .route(
            "/user/user/checkModifyPasswordMobileCode",
            post(check_modify_password_mobile_code),
        )
        .route(
            "/user/user/sendMobileCodeByMobileValidate",
            post(send_mobile_code_by_mobile_validate),
        )
        .route(
            "/user/user/sendEmailCodeByEmailValidate",
            post(send_email_code_by_email_validate),
        )
        .route(
            "/user/user/sendMobileCodeByModifyMobile",
            post(send_mobile_code_by_modify_mobile),
        )
        .route(
            "/user/user/sendEmailCodeByModifyEmail",
            post(send_email_code_by_modify_email),
        )
        .route("/user/user/modifyMobile", post(modify_mobile))
        .route("/user/user/modifyEmail", post(modify_email))
        .route("/user/user/mobileValidate", post(mobile_validate))
        .route("/user/user/emailValidateNew", post(email_validate_new))
        .route("/user/user/emailValidate", post(email_validate))
        .route("/user/user/historyProduct", get(history_product))
        .route("/user/user/delHistoryProduct", post(del_history_product))
        .route("/user/user/modifyAvatar", post(modify_avatar))
        .route("/user/user/collectionShop", get(collection_shop))
        .route("/user/user/collectionShop/save", post(collection_shop_save))
        .route(
            "/user/user/collectionShop/cancel",
            post(collection_shop_cancel),
        )
        .route("/user/user/logout", post(logout))
        .route("/user/user/close", post(close))
        .layer(middleware::from_fn_with_state(
            state.jwt.clone(),
            require_auth,
        ));

    let anonymous = Router::new()
        .route("/user/user/uploadImg", post(upload_img))
        .route("/user/user/uploadVideo", post(upload_video))
        .route("/user/user/levelList", get(level_list))
        .route("/user/user/levelInfo", get(level_info));

    authorized.merge(anonymous)
}

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    [
        ("GET", "/api/user/user/creditDetail", "UserController.creditDetail"),
        (
            "POST",
            "/api/user/user/updateInformation",
            "UserController.updateInformation",
        ),
        ("GET", "/api/user/user/memberCenter", "UserController.memberCenter"),
        (
            "POST",
            "/api/user/user/sendMobileCodeByModifyPassword",
            "UserController.sendMobileCodeByModifyPassword",
        ),
        (
            "POST",
            "/api/user/user/checkModifyPasswordMobileCode",
            "UserController.checkModifyPasswordMobileCode",
        ),
        (
            "POST",
            "/api/user/user/sendMobileCodeByMobileValidate",
            "UserController.sendMobileCodeByMobileValidate",
        ),
        (
            "POST",
            "/api/user/user/sendEmailCodeByEmailValidate",
            "UserController.sendEmailCodeByEmailValidate",
        ),
        (
            "POST",
            "/api/user/user/sendMobileCodeByModifyMobile",
            "UserController.sendMobileCodeByModifyMobile",
        ),
        (
            "POST",
            "/api/user/user/sendEmailCodeByModifyEmail",
            "UserController.sendEmailCodeByModifyEmail",
        ),
        ("POST", "/api/user/user/modifyMobile", "UserController.modifyMobile"),
        ("POST", "/api/user/user/modifyEmail", "UserController.modifyEmail"),
        ("POST", "/api/user/user/mobileValidate", "UserController.mobileValidate"),
        (
            "POST",
            "/api/user/user/emailValidateNew",
            "UserController.emailValidateNew",
        ),
        ("POST", "/api/user/user/emailValidate", "UserController.emailValidate"),
        ("GET", "/api/user/user/historyProduct", "UserController.historyProduct"),
        (
            "POST",
            "/api/user/user/delHistoryProduct",
            "UserController.delHistoryProduct",
        ),
        ("POST", "/api/user/user/uploadImg", "UserController.uploadImg"),
        ("POST", "/api/user/user/uploadVideo", "UserController.uploadVideo"),
        ("POST", "/api/user/user/modifyAvatar", "UserController.modifyAvatar"),
        ("GET", "/api/user/user/collectionShop", "UserController.collectionShop"),
        (
            "POST",
            "/api/user/user/collectionShop/save",
            "UserController.collectionShopSave",
        ),
        (
            "POST",
            "/api/user/user/collectionShop/cancel",
            "UserController.collectionShopCancel",
        ),
        ("GET", "/api/user/user/levelList", "UserController.levelList"),
        ("GET", "/api/user/user/levelInfo", "UserController.levelInfo"),
        ("POST", "/api/user/user/logout", "UserController.logout"),
        ("POST", "/api/user/user/close", "UserController.close"),
    ]
    .iter()
    .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, p, j))
    .collect()
}

// ————————————————————— 账期 / 资料 / 会员中心 —————————————————————

/// `GET /api/user/user/creditDetail`：用户账期支付信息详情。
///
/// 对应 Java `UserController.creditDetail`（`CreditUserServiceImpl.getUserCreditDetail`）。
/// 后台未开启账期支付时 Java 返回 `null`，这里返回 `None`（响应体无 `data`）。
async fn credit_detail(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<Option<CreditDetailVO>> {
    ok(state.member.user_center.credit_detail(auth.id).await?)
}

/// `POST /api/user/user/updateInformation`：更新会员信息。
///
/// 对应 Java `UserController.updateInformation(@RequestBody UpdateInformationDTO)`。
/// 原方法返回 `void`，这里回传提示文案（项目内既有约定）。
async fn update_information(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<UpdateInformationDTO>,
) -> ApiResult<&'static str> {
    state.member.user_center.update_information(auth.id, &dto).await?;
    ok("更新成功")
}

/// `GET /api/user/user/memberCenter`：会员中心。
///
/// 对应 Java `UserController.memberCenter()`。
async fn member_center(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<MemberCenterVO> {
    ok(state.member.user_center.member_center(auth.id).await?)
}

// ————————————————————— 手机/邮箱验证码（未迁移） —————————————————————

/// `POST /api/user/user/sendMobileCodeByModifyPassword`：修改密码前发送手机验证码。
///
/// 对应 Java `UserController.sendMobileCodeByModifyPassword(@RequestBody RegisterSmsDTO)`
/// （`event = "modify_password"`，手机号统一取当前登录用户）。
/// **未迁移**：短信通道与验证码存储。
async fn send_mobile_code_by_modify_password(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(_dto): Json<RegisterSmsDTO>,
) -> ApiResult<&'static str> {
    state.member.user_center
        .send_sms_code(auth.id, "modify_password", None)
        .await?;
    ok("验证码已发送")
}

/// `POST /api/user/user/checkModifyPasswordMobileCode`：校验手机验证码并改密码。
///
/// 对应 Java `UserController.checkModifyPasswordMobileCode(@RequestBody
/// CheckModifyPasswordDTO)`。**未迁移**：验证码存在 Redis（写入方为未迁移的短信通道）。
async fn check_modify_password_mobile_code(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<CheckModifyPasswordDTO>,
) -> ApiResult<&'static str> {
    state.member.user_center
        .check_modify_password_mobile_code(auth.id, &dto)
        .await?;
    ok("密码修改成功")
}

/// `POST /api/user/user/sendMobileCodeByMobileValidate`：手机验证获取验证码。
///
/// 对应 Java `UserController.sendMobileCodeByMobileValidate`（`event = "mobile_validate"`）。
/// **未迁移**：短信通道与验证码存储。
async fn send_mobile_code_by_mobile_validate(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(_dto): Json<RegisterSmsDTO>,
) -> ApiResult<&'static str> {
    state.member.user_center
        .send_sms_code(auth.id, "mobile_validate", None)
        .await?;
    ok("验证码已发送")
}

/// `POST /api/user/user/sendEmailCodeByEmailValidate`：邮箱验证获取验证码。
///
/// 对应 Java `UserController.sendEmailCodeByEmailValidate`（`event = "email_validate"`）。
/// **未迁移**：邮件通道与验证码存储。
async fn send_email_code_by_email_validate(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(_dto): Json<RegisterEmailDTO>,
) -> ApiResult<&'static str> {
    state.member.user_center
        .send_email_code(auth.id, "email_validate", None)
        .await?;
    ok("验证码已发送")
}

/// `POST /api/user/user/sendMobileCodeByModifyMobile`：换绑手机获取验证码。
///
/// 对应 Java `UserController.sendMobileCodeByModifyMobile`（`event = "modify_mobile"`，
/// 新手机号必填）。**未迁移**：短信通道与验证码存储。
async fn send_mobile_code_by_modify_mobile(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<RegisterSmsDTO>,
) -> ApiResult<&'static str> {
    state.member.user_center
        .send_sms_code(auth.id, "modify_mobile", dto.mobile.as_deref())
        .await?;
    ok("验证码已发送")
}

/// `POST /api/user/user/sendEmailCodeByModifyEmail`：换绑邮箱获取验证码。
///
/// 对应 Java `UserController.sendEmailCodeByModifyEmail`（`event = "modify_email"`，
/// 新邮箱必填）。**未迁移**：邮件通道与验证码存储。
async fn send_email_code_by_modify_email(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<RegisterEmailDTO>,
) -> ApiResult<&'static str> {
    state.member.user_center
        .send_email_code(auth.id, "modify_email", dto.email.as_deref())
        .await?;
    ok("验证码已发送")
}

/// `POST /api/user/user/modifyMobile`：修改手机号。
///
/// 对应 Java `UserController.modifyMobile(@RequestBody @Validated ModifyMobileParam)`。
/// **未迁移**：依赖 Redis 验证码。
async fn modify_mobile(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(param): Json<ModifyMobileParam>,
) -> ApiResult<&'static str> {
    state.member.user_center.modify_mobile(auth.id, &param).await?;
    ok("手机号修改成功")
}

/// `POST /api/user/user/modifyEmail`：修改邮箱。
///
/// 对应 Java `UserController.modifyEmail(@RequestBody @Validated ModifyEmailParam)`。
/// **未迁移**：依赖 Redis 验证码。
async fn modify_email(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(param): Json<ModifyEmailParam>,
) -> ApiResult<&'static str> {
    state.member.user_center.modify_email(auth.id, &param).await?;
    ok("邮箱修改成功")
}

/// `POST /api/user/user/mobileValidate`：手机验证。
///
/// 对应 Java `UserController.mobileValidate(@RequestBody CheckModifyPasswordDTO)`。
/// **未迁移**：依赖 Redis 验证码。
async fn mobile_validate(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<CheckModifyPasswordDTO>,
) -> ApiResult<&'static str> {
    state.member.user_center.mobile_validate(auth.id, &dto).await?;
    ok("验证通过")
}

/// `POST /api/user/user/emailValidateNew`：邮箱验证。
///
/// 对应 Java `UserController.emailValidateNew(@RequestBody CheckModifyEmailDTO)`。
/// **未迁移**：依赖 Redis 验证码。
async fn email_validate_new(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<CheckModifyEmailDTO>,
) -> ApiResult<&'static str> {
    state.member.user_center.email_validate_new(auth.id, &dto).await?;
    ok("验证通过")
}

/// `POST /api/user/user/emailValidate`：邮箱验证（可直接改绑）。
///
/// 对应 Java `UserController.emailValidate(@RequestBody CheckEmailModifyDTO)`：
/// 该方法**不**校验 Redis 验证码，仅做唯一性校验，`type > 0` 时改绑。
async fn email_validate(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<CheckEmailModifyDTO>,
) -> ApiResult<&'static str> {
    state.member.user_center.email_validate(auth.id, &dto).await?;
    ok("验证通过")
}

// ————————————————————— 浏览历史 —————————————————————

/// `GET /api/user/user/historyProduct`：最近浏览。
///
/// 对应 Java `UserController.historyProduct()`。
async fn history_product(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<Vec<HistoryProductVO>> {
    ok(state.member.user_center.history_product(auth.id).await?)
}

/// `POST /api/user/user/delHistoryProduct`：删除最近浏览。
///
/// 对应 Java `UserController.delHistoryProduct(@RequestBody DelHistoryProductDTO)`。
async fn del_history_product(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<DelHistoryProductDTO>,
) -> ApiResult<&'static str> {
    state.member.user_center.del_history_product(auth.id, &dto).await?;
    ok("删除成功")
}

// ————————————————————— 上传（未迁移） —————————————————————

/// `POST /api/user/user/uploadImg`：上传图片（`multipart/form-data`）。
///
/// 对应 Java `UserController.uploadImg(@RequestParam("file") MultipartFile file,
/// @RequestParam(value = "hidden", required = false, defaultValue = "false") boolean isHidden)`。
/// **未迁移**：图库/对象存储服务。
async fn upload_img(
    State(state): State<PublicState>,
    mut multipart: Multipart,
) -> ApiResult<GalleryPicUploadVO> {
    let form = read_multipart(&mut multipart).await?;
    let is_hidden = form
        .params
        .get("hidden")
        .is_some_and(|v| v.eq_ignore_ascii_case("true") || v == "1");
    ok(state.member.user_center
        .upload_img(form.file_name.as_deref(), &form.bytes, is_hidden).await?)
}

/// `POST /api/user/user/uploadVideo`：上传视频（`multipart/form-data`）。
///
/// 对应 Java `UserController.uploadVideo(@RequestParam("file") MultipartFile file)`。
/// **未迁移**：视频/对象存储服务。
async fn upload_video(
    State(state): State<PublicState>,
    mut multipart: Multipart,
) -> ApiResult<UploadVideoInfoVO> {
    let form = read_multipart(&mut multipart).await?;
    ok(state.member.user_center
        .upload_video(form.file_name.as_deref(), &form.bytes).await?)
}

/// `POST /api/user/user/modifyAvatar`：修改头像（`multipart/form-data`）。
///
/// 对应 Java `UserController.modifyAvatar(@RequestParam("file") MultipartFile file)`。
/// **未迁移**：图库上传 + 微信小程序内容安全校验。
async fn modify_avatar(
    State(state): State<PublicState>,
    mut multipart: Multipart,
) -> ApiResult<&'static str> {
    let form = read_multipart(&mut multipart).await?;
    state.member.user_center
        .modify_avatar(form.file_name.as_deref(), &form.bytes).await?;
    ok("头像修改成功")
}

/// 解析后的 multipart 表单：`file` 字段 + 其余文本参数。
struct UploadForm {
    file_name: Option<String>,
    bytes: Vec<u8>,
    params: HashMap<String, String>,
}

/// 读取 `multipart/form-data`：`file` 字段字节流 + 其它文本参数
/// （对应 Java 的 `@RequestParam("file") MultipartFile` 与普通 `@RequestParam`）。
async fn read_multipart(multipart: &mut Multipart) -> Result<UploadForm, AppError> {
    let mut form = UploadForm {
        file_name: None,
        bytes: Vec::new(),
        params: HashMap::new(),
    };
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError(nextrade_core::Error::Validation(format!("解析上传表单失败: {e}"))))?
    {
        let name = field.name().unwrap_or_default().to_string();
        if name == "file" {
            form.file_name = field.file_name().map(str::to_string);
            form.bytes = field
                .bytes()
                .await
                .map_err(|e| {
                    AppError(nextrade_core::Error::Validation(format!(
                        "读取上传文件失败: {e}"
                    )))
                })?
                .to_vec();
        } else {
            let value = field.text().await.map_err(|e| {
                AppError(nextrade_core::Error::Validation(format!(
                    "读取表单字段失败: {e}"
                )))
            })?;
            form.params.insert(name, value);
        }
    }
    Ok(form)
}

// ————————————————————— 收藏店铺 / 会员等级 —————————————————————

/// `GET /api/user/user/collectionShop`：收藏店铺（分页）。
///
/// 对应 Java `UserController.collectionShop(CollectShopListDTO collectShopListDTO)`
/// → `CollectShopServiceImpl.list`。Java 返回 MyBatis-Plus `Page<CollectShopVO>`，
/// Rust 统一使用项目既有的 `PageResult`。
async fn collection_shop(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<CollectShopQuery>,
) -> ApiResult<PageResult<CollectShopVO>> {
    ok(state.member.user_center.collection_shop(auth.id, &q).await?)
}

/// `POST /api/user/user/collectionShop/save`：收藏店铺（b2b2b 5.8.57 前端新增，
/// 无 Java 对应；payload `{shopId}`，幂等）。
#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct CollectShopSaveDTO {
    shop_id: Option<i32>,
}

async fn collection_shop_save(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<CollectShopSaveDTO>,
) -> ApiResult<()> {
    state
        .member
        .user_center
        .save_collect_shop(auth.id, dto.shop_id.unwrap_or(0))
        .await?;
    ok(())
}

/// `POST /api/user/user/collectionShop/cancel`：取消收藏（payload `{id}` 或 `{shopId}`）。
#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct CollectShopCancelDTO {
    id: Option<i32>,
    shop_id: Option<i32>,
}

async fn collection_shop_cancel(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<CollectShopCancelDTO>,
) -> ApiResult<()> {
    state
        .member
        .user_center
        .cancel_collect_shop(auth.id, dto.id, dto.shop_id)
        .await?;
    ok(())
}

/// `GET /api/user/user/levelList`：会员等级列表。
///
/// 对应 Java `UserController.levelList()`（不依赖登录态）。
async fn level_list(State(state): State<PublicState>) -> ApiResult<UserLevelListVO> {
    ok(state.member.user_center.level_list().await?)
}

/// `GET /api/user/user/levelInfo?rankId=`：会员等级权益信息。
///
/// 对应 Java `UserController.levelInfo(@RequestParam("rankId") Integer rankId)`。
async fn level_info(
    State(state): State<PublicState>,
    Query(q): Query<RankIdQuery>,
) -> ApiResult<UserLevelInfoVO> {
    ok(state.member.user_center.level_info(q.rank_id).await?)
}

/// `levelInfo` 查询参数（Java `@RequestParam("rankId")` 必填）。
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RankIdQuery {
    rank_id: i32,
}

// ————————————————————— 退出 / 注销 —————————————————————

/// `POST /api/user/user/logout`：前台用户退出登录。
///
/// 对应 Java `UserController.logout()` → `UserServiceImpl.clientLogout`：
/// 原实现删除 Redis 中的登录令牌完成**服务端吊销**。Rust 侧 JWT 无状态
/// （无令牌存储/黑名单），无法真正吊销，故返回明确的「未迁移」业务错误，
/// 由前端丢弃本地令牌（详见 `nextrade_services::member::user_center` 模块注释与
/// `uncertain_points`）。
async fn logout(
    State(state): State<PublicState>,
    Extension(_auth): Extension<AuthUser>,
    headers: axum::http::HeaderMap,
) -> ApiResult<()> {
    if let Some(raw) = headers.get(axum::http::header::AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        let bare = raw.trim().strip_prefix("Bearer ").unwrap_or(raw.trim());
        if let Ok(claims) = state.jwt.decode(bare) {
            nextrade_security::token_revocation::revoke(bare, claims.exp);
        }
    }
    state.member.user_center.logout()?;
    ok(())
}

/// `POST /api/user/user/close`：禁用（注销）当前用户。
///
/// 对应 Java `UserController.close()` → `UserServiceImpl.closeUser`（`status = 0`）。
async fn close(
    State(state): State<PublicState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<&'static str> {
    state.member.user_center.close(auth.id).await?;
    ok("账号已注销")
}