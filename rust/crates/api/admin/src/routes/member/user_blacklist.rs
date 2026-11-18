//! 管理端客户黑名单路由（member::user_blacklist）。
//!
//! 设计定稿见 `~/workspace/blacklist-design/proposal.md` §5.1：
//! 列表 / 详情 / 加入（单个+批量）/ 移除（单个+批量）/ 操作记录 / 开关读写
//! （Phase 1）+ 导入模板 / 批量导入 / 失败明细 / 导出（Phase 3，照会员
//! 导入三件套模式：Multipart 上传 + `system::excel` 读写 + `xlsx_response`）。
//!
//! | 本模块路由 | 权限（拟） | 认证 |
//! |-----------|--------|------|
//! | `GET  /adminapi/user/blacklist/list` | `userBlacklistManage` | 登录 |
//! | `GET  /adminapi/user/blacklist/detail` | `userBlacklistManage` | 登录 |
//! | `GET  /adminapi/user/blacklist/logList` | `userBlacklistManage` | 登录 |
//! | `POST /adminapi/user/blacklist/add` | `userBlacklistAddManage` | 登录 |
//! | `POST /adminapi/user/blacklist/batchAdd` | `userBlacklistAddManage` | 登录 |
//! | `POST /adminapi/user/blacklist/remove` | `userBlacklistRemoveManage` | 登录 |
//! | `POST /adminapi/user/blacklist/batchRemove` | `userBlacklistRemoveManage` | 登录 |
//! | `GET  /adminapi/user/blacklist/setting/get` | 登录即可 | 登录 |
//! | `POST /adminapi/user/blacklist/setting/save` | `userBlacklistSettingManage` | 登录（仅平台） |
//!
//! > 归属口径：owner 一律由 `AuthUser` 推导（平台 / 店铺本店），请求体不设
//! > owner 字段；作用域校验在服务层（`UserBlacklistService`）。

use axum::{
    extract::{Multipart, Query, State},
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;

use nextrade_core::models::member::user_blacklist::{
    behavior_name, product_name, BlacklistAddParam, BlacklistAddResultVO, BlacklistBatchAddParam,
    BlacklistBatchAddResultVO, BlacklistBatchRemoveParam, BlacklistBatchRemoveResultVO,
    BlacklistEntryVO, BlacklistImportResultVO, BlacklistListQuery, BlacklistLogQuery,
    BlacklistLogVO, BlacklistOperator, BlacklistRemoveParam, BlacklistSettingParam,
    BlacklistSettingVO, APPLY_SELECTED, EXPORT_HEADERS, IMPORT_ERROR_HEADERS, IMPORT_HEADERS,
    OWNER_SHOP, STATUS_ACTIVE,
};
use nextrade_core::types::PageResult;
use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};

use crate::AdminState;

/// 本模块路由表（去掉 `/adminapi` 前缀）。
const ROUTE_TABLE: [(&str, &str, &str); 13] = [
    ("GET", "/user/blacklist/list", "UserBlacklistController.list"),
    ("GET", "/user/blacklist/detail", "UserBlacklistController.detail"),
    ("GET", "/user/blacklist/logList", "UserBlacklistController.logList"),
    ("POST", "/user/blacklist/add", "UserBlacklistController.add"),
    ("POST", "/user/blacklist/batchAdd", "UserBlacklistController.batchAdd"),
    ("POST", "/user/blacklist/remove", "UserBlacklistController.remove"),
    ("POST", "/user/blacklist/batchRemove", "UserBlacklistController.batchRemove"),
    ("GET", "/user/blacklist/setting/get", "UserBlacklistController.settingGet"),
    ("POST", "/user/blacklist/setting/save", "UserBlacklistController.settingSave"),
    ("GET", "/user/blacklist/downloadImportTemplate", "UserBlacklistController.downloadImportTemplate"),
    ("POST", "/user/blacklist/import", "UserBlacklistController.import"),
    ("POST", "/user/blacklist/getImportErrorData", "UserBlacklistController.getImportErrorData"),
    ("GET", "/user/blacklist/export", "UserBlacklistController.export"),
];

/// 需登录路由（挂载于 `/adminapi` 下，外层 `require_auth`）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/user/blacklist/list", get(list))
        .route("/user/blacklist/detail", get(detail))
        .route("/user/blacklist/logList", get(log_list))
        .route("/user/blacklist/add", post(add))
        .route("/user/blacklist/batchAdd", post(batch_add))
        .route("/user/blacklist/remove", post(remove))
        .route("/user/blacklist/batchRemove", post(batch_remove))
        .route("/user/blacklist/setting/get", get(setting_get))
        .route("/user/blacklist/setting/save", post(setting_save))
        .route("/user/blacklist/downloadImportTemplate", get(download_import_template))
        .route("/user/blacklist/import", post(import))
        .route("/user/blacklist/getImportErrorData", post(import_error_data))
        .route("/user/blacklist/export", get(export))
}

/// 本模块已实现路由（覆盖率统计，完整契约路径）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, java_ref)| {
            nextrade_openapi::RouteDoc::new(method, &format!("/adminapi{path}"), java_ref)
        })
        .collect()
}

// ——————————————————————————————————————————————
// handler
// ——————————————————————————————————————————————

/// 由可信 JWT 身份推导操作人上下文（归属不信任请求体）。
fn operator(auth: &AuthUser) -> BlacklistOperator {
    BlacklistOperator {
        admin_id: auth.id,
        admin_name: auth.username.clone(),
        admin_type: auth.admin_type.clone().unwrap_or_default(),
        shop_id: auth.scoped_shop_id().unwrap_or(0),
        is_platform: auth.is_platform(),
    }
}

/// `detail?id=` 查询参数。
#[derive(Debug, Deserialize)]
pub struct BlacklistDetailQuery {
    pub id: i64,
}

async fn list(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<BlacklistListQuery>,
) -> ApiResult<PageResult<BlacklistEntryVO>> {
    ok(state.member.user_blacklist.list(&q, &operator(&auth)).await?)
}

async fn detail(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<BlacklistDetailQuery>,
) -> ApiResult<BlacklistEntryVO> {
    ok(state.member.user_blacklist.detail(q.id, &operator(&auth)).await?)
}

async fn log_list(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<BlacklistLogQuery>,
) -> ApiResult<PageResult<BlacklistLogVO>> {
    ok(state.member.user_blacklist.log_list(&q, &operator(&auth)).await?)
}

async fn add(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(param): Json<BlacklistAddParam>,
) -> ApiResult<BlacklistAddResultVO> {
    ok(state.member.user_blacklist.add(&param, &operator(&auth)).await?)
}

async fn batch_add(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(param): Json<BlacklistBatchAddParam>,
) -> ApiResult<BlacklistBatchAddResultVO> {
    ok(state.member.user_blacklist.batch_add(&param, &operator(&auth)).await?)
}

async fn remove(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(param): Json<BlacklistRemoveParam>,
) -> ApiResult<()> {
    state.member.user_blacklist.remove(&param, &operator(&auth)).await?;
    ok(())
}

async fn batch_remove(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(param): Json<BlacklistBatchRemoveParam>,
) -> ApiResult<BlacklistBatchRemoveResultVO> {
    ok(state.member.user_blacklist.batch_remove(&param, &operator(&auth)).await?)
}

async fn setting_get(State(state): State<AdminState>) -> ApiResult<BlacklistSettingVO> {
    ok(state.member.user_blacklist.setting_get().await?)
}

async fn setting_save(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(param): Json<BlacklistSettingParam>,
) -> ApiResult<()> {
    state
        .member
        .user_blacklist
        .setting_save(&param, &operator(&auth))
        .await?;
    ok(())
}

// ——————————————————————————————————————————————
// Phase 3：导入模板 / 批量导入 / 失败明细 / 导出
// ——————————————————————————————————————————————

/// 读取 multipart 中的 `file` 字段（照 user_manage 路由同名小函数）。
async fn read_upload_file(multipart: &mut Multipart) -> Result<Vec<u8>, AppError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError(Error::Validation(format!("读取上传文件失败: {e}"))))?
    {
        if field.name() == Some("file") {
            return field
                .bytes()
                .await
                .map(|b| b.to_vec())
                .map_err(|e| AppError(Error::Validation(format!("读取上传文件失败: {e}"))));
        }
    }
    Err(AppError(Error::Validation("请上传文件".into())))
}

/// `GET /adminapi/user/blacklist/downloadImportTemplate`：导入模板 xlsx
/// （列定义见方案 §5.2，表头即解析校验口径）。
async fn download_import_template(
    State(_state): State<AdminState>,
) -> std::result::Result<Response, AppError> {
    let bytes = nextrade_services::system::excel::build_xlsx(&IMPORT_HEADERS, &[], "黑名单导入模板")
        .map_err(AppError)?;
    Ok(crate::support::excel_response::xlsx_response(
        bytes,
        "blacklist_import_template.xlsx",
    ))
}

/// `POST /adminapi/user/blacklist/import`：批量导入（Multipart `file`）。
/// 返回结构化结果（部分失败不走 Err，失败逐行带原因，§5.1）。
async fn import(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    mut multipart: Multipart,
) -> ApiResult<BlacklistImportResultVO> {
    let bytes = read_upload_file(&mut multipart).await?;
    let sheet = nextrade_services::system::excel::read_xlsx(&bytes)?;
    ok(state
        .member
        .user_blacklist
        .import_rows(&sheet, &operator(&auth))
        .await?)
}

/// `POST /adminapi/user/blacklist/getImportErrorData`：失败明细 xlsx。
/// 与会员导入同模式：重传原文件 → 服务层只校验不落库 → 错误表下载。
async fn import_error_data(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    mut multipart: Multipart,
) -> std::result::Result<Response, AppError> {
    let bytes = read_upload_file(&mut multipart).await?;
    let sheet = nextrade_services::system::excel::read_xlsx(&bytes).map_err(AppError)?;
    let errors = state
        .member
        .user_blacklist
        .import_error_rows(&sheet, &operator(&auth))
        .await
        .map_err(AppError)?;
    let data: Vec<Vec<String>> = errors
        .iter()
        .map(|e| {
            vec![
                e.row.to_string(),
                e.user_id.clone(),
                e.mobile.clone(),
                e.username.clone(),
                e.product.clone(),
                e.behavior.clone(),
                e.shop_ids.clone(),
                e.error_msg.clone(),
            ]
        })
        .collect();
    let bytes = nextrade_services::system::excel::build_xlsx(
        &IMPORT_ERROR_HEADERS,
        &data,
        "黑名单导入错误",
    )
    .map_err(AppError)?;
    Ok(crate::support::excel_response::xlsx_response(
        bytes,
        "blacklist_import_error.xlsx",
    ))
}

/// 条目 VO → 导出行（列序同 EXPORT_HEADERS；范围/规则口径与列表页一致）。
fn export_row(entry: &BlacklistEntryVO) -> Vec<String> {
    let scope = if entry.owner_type == OWNER_SHOP {
        entry.owner_shop_name.clone().unwrap_or_else(|| "本店".to_string())
    } else if entry.apply_mode == APPLY_SELECTED {
        if entry.shop_names.is_empty() {
            format!("指定店铺 {} 家", entry.shop_ids.len())
        } else {
            entry.shop_names.join("、")
        }
    } else {
        "全平台".to_string()
    };
    let products = entry
        .rules
        .iter()
        .map(|r| product_name(&r.product).to_string())
        .collect::<Vec<_>>()
        .join("、");
    let behaviors = entry
        .rules
        .iter()
        .map(|r| behavior_name(&r.behavior).to_string())
        .collect::<Vec<_>>()
        .join("、");
    let user = entry.user.as_ref();
    vec![
        entry.user_id.to_string(),
        user.and_then(|u| u.username.clone()).unwrap_or_default(),
        user.and_then(|u| u.nickname.clone()).unwrap_or_default(),
        user.and_then(|u| u.mobile.clone()).unwrap_or_default(),
        products,
        behaviors,
        scope,
        entry.reason.clone(),
        if entry.status == STATUS_ACTIVE { "生效中".to_string() } else { "已移除".to_string() },
        entry.create_admin_name.clone(),
        nextrade_core::models::analytics::sales_statistics::format_time(entry.create_time),
    ]
}

/// `GET /adminapi/user/blacklist/export`：按列表筛选导出 xlsx（查询参数同
/// list，含加入时间范围；无分页；作用域与列表一致，§5.1）。
async fn export(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<BlacklistListQuery>,
) -> std::result::Result<Response, AppError> {
    let entries = state
        .member
        .user_blacklist
        .export_entries(&q, &operator(&auth))
        .await
        .map_err(AppError)?;
    let data: Vec<Vec<String>> = entries.iter().map(export_row).collect();
    let bytes = nextrade_services::system::excel::build_xlsx(&EXPORT_HEADERS, &data, "客户黑名单")
        .map_err(AppError)?;
    Ok(crate::support::excel_response::xlsx_response(
        bytes,
        "blacklist_export.xlsx",
    ))
}
