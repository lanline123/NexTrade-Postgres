//! WorkBuddy MCP 开放接口域服务（openapi::workbuddy）。
//!
//! 对应原 Java：
//! - `com.nextrade.service.workbuddy.impl.WorkBuddyKeyServiceImpl`（112 行）
//! - `com.nextrade.service.workbuddy.impl.ProductManageServiceImpl`（677 行）
//! - `com.nextrade.service.workbuddy.impl.ProductImportServiceImpl`（1220 行）
//! - `com.nextrade.service.workbuddy.impl.ProductContextServiceImpl`（155 行）
//! - `com.nextrade.service.workbuddy.impl.ProductAnalysisServiceImpl`（208 行）
//! - `com.nextrade.service.workbuddy.impl.MediaManagementServiceImpl`（460 行）
//! - `com.nextrade.service.workbuddy.impl.McpMediaUploadServiceImpl`（357 行）
//! - `com.nextrade.service.workbuddy.impl.CategoryImportServiceImpl`（384 行）
//!
//! | 本域能力（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |-------------------------------|---------|------|
//! | `GET  /messagepush/messagepush/push` | `MessagePushController.sendMessage` | [`WorkbuddyService::message_push`] |
//! | `GET  /messagepush/messagepush/nextradeConfig` | `MessagePushController.nextradeConfig` | [`WorkbuddyService::nextrade_config`] |
//! | `GET  /openapi/mcp/v1/connection/status` | `WorkBuddyProductImportController.connectionStatus` | [`WorkbuddyService::connection_status`] |
//! | `GET  /openapi/mcp/v1/import/requirements` | 同上 `.requirements` | [`WorkbuddyService::requirements`] |
//! | `GET  /openapi/mcp/v1/categories` | 同上 `.categories` | [`WorkbuddyService::categories`] |
//! | `POST /openapi/mcp/v1/products/validate` | 同上 `.validate` | [`WorkbuddyService::validate_products`] |
//! | `POST /openapi/mcp/v1/products/import` | 同上 `.importProducts` | [`WorkbuddyService::import_products`] |
//! | `POST /openapi/mcp/v1/media/upload` | 同上 `.uploadMedia` | [`WorkbuddyService::upload_media`] |
//! | `POST /openapi/mcp/v1/media/upload-token` | 同上 `.uploadToken` | [`WorkbuddyService::upload_token`] |
//! | `GET  /openapi/mcp/v1/capabilities` | 同上 `.capabilities` | [`WorkbuddyService::capabilities`] |
//! | `GET  /openapi/mcp/v1/products/context` | 同上 `.productContext` | [`WorkbuddyService::product_context`] |
//! | `GET  /openapi/mcp/v1/media` | `WorkBuddyProductIntelligenceController.media` | [`WorkbuddyService::media`] |
//! | `POST /openapi/mcp/v1/media/prepare` | 同上 `.mediaPrepare` | [`WorkbuddyService::media_prepare`] |
//! | `POST /openapi/mcp/v1/media/execute` | 同上 `.mediaExecute` | [`WorkbuddyService::media_execute`] |
//! | `POST /openapi/mcp/v1/categories/prepare` | 同上 `.categoryPrepare` | [`WorkbuddyService::category_prepare`] |
//! | `POST /openapi/mcp/v1/categories/execute` | 同上 `.categoryExecute` | [`WorkbuddyService::category_execute`] |
//! | `GET  /openapi/mcp/v1/products/analysis` | 同上 `.analysis` | [`WorkbuddyService::analysis`] |
//! | `GET  /openapi/mcp/v1/products` | `WorkBuddyProductManageController.list` | [`WorkbuddyService::product_list`] |
//! | `GET  /openapi/mcp/v1/products/{id}` | 同上 `.detail` | [`WorkbuddyService::product_detail`] |
//! | `GET  /openapi/mcp/v1/products/{id}/promotion-links` | 同上 `.promotionLinks` | [`WorkbuddyService::promotion_links`] |
//! | `POST /openapi/mcp/v1/products/{id}/field` | 同上 `.updateField` | [`WorkbuddyService::update_field`] |
//! | `POST /openapi/mcp/v1/products/{id}/status` | 同上 `.setStatus` | [`WorkbuddyService::set_status`] |
//! | `POST /openapi/mcp/v1/products/{id}/recycle` | 同上 `.recycle` | [`WorkbuddyService::recycle`] |
//! | `POST /openapi/mcp/v1/products/{id}/restore` | 同上 `.restore` | [`WorkbuddyService::restore`] |
//! | `POST /openapi/mcp/v1/products/{id}/copy` | 同上 `.copy` | [`WorkbuddyService::copy`] |
//! | `POST /openapi/mcp/v1/products/{id}/delete` | 同上 `.deletePermanently` | [`WorkbuddyService::delete_permanently`] |
//! | `POST /openapi/mcp/v1/products/batch/preview` | 同上 `.batchPreview` | [`WorkbuddyService::batch_preview`] |
//! | `POST /openapi/mcp/v1/products/batch/execute` | 同上 `.batchExecute` | [`WorkbuddyService::batch_execute`] |
//!
//! ## 错误响应
//!
//! WorkBuddy 包下的控制器由 `WorkBuddyApiExceptionHandler`
//! （`@IgnoreResponseAdvice` + `HIGHEST_PRECEDENCE`）接管，返回
//! **真实 HTTP 状态码 + 扁平错误体** `{errorCode, message, retryable}`，
//! **不**走 `AjaxResult`。本服务用 [`WorkBuddyError`] 承载该结构，
//! 由路由层直出（`GET /messagepush/messagepush/*` 除外——它不在
//! workbuddy 包下，走常规 `AjaxResult`）。
//!
//! ## ⚠️ 进程内状态（与 Java 一致的已知局限）
//!
//! Java 把 preview / 幂等 / 批量快照 / 上传令牌全部存在
//! **进程内 `ConcurrentHashMap`**（`previews`、`idem`、`batchSnapshots`、
//! `staged`、`results`）。多实例部署下这些状态**本就不共享**。
//! Rust 侧用 `Mutex<HashMap>` 复刻同样的单实例语义
//! （见 `uncertain_points`）。
//!
//! ## ⚠️ 未实现的 Java 副作用
//!
//! - `MessagePushController.push` 依赖 `MessageService.sendUserMessage` 的
//!   **六个下游投递通道**（站内信 / 短信 / 公众号 / 小程序 / 邮件 / 钉钉 /
//!   企业微信）。本片未读到该实现，故只做**入参校验与订单归属检查**
//!   （这两段在 Java 控制器/服务签名里已确认），投递本身不实现，
//!   见 `uncertain_points`。
//! - `promotion-links` 依赖商城 H5/PC 域名、短链服务、小程序码生成、
//!   微信短链 API。本片未读到这些实现，故只回出**可推导的原始链接**，
//!   短链/小程序码/微信短链留空并给出 `wechatShortLinkError`，
//!   见 `uncertain_points`。

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use chrono::{NaiveDateTime, Utc};
use rust_decimal::Decimal;
use serde_json::json;

use nextrade_core::models::openapi::workbuddy::{
    assistant_defaults, batch_block_reason, batch_consequences, batch_fingerprint,
    batch_operation_label, build_import_suggestion, candidate_images, capabilities_media_defaults,
    capabilities_skeleton, category_error, category_item_map, category_of,
    category_preview_summary, context_defaults, dedupe_urls, evaluate_key_status,
    flatten_category_nodes, format_candidate, has_invalid_image, image_max_bytes,
    images_all_blank, is_owned_by, is_supported_media_type,
    key_authorized, key_unauthorized_message, media_error, media_fingerprint_canonical,
    media_image_urls, normalize_batch_ids, normalize_batch_operation, normalize_number,
    normalize_page, normalize_page_size, normalize_parent_id, normalize_platform_shop_id,
    parse_size_bytes, preserve_preview_for_retry, price_band, product_not_found_message,
    product_type_label, require_category_platform_scope, require_editable_field,
    require_platform_scope, require_product_id, resolve_media_ids, resolve_media_name,
    resolve_media_type, resolve_sort_field, risk_level, round2, safe_message, status_label,
    stock_to_sales_ratio, tenant_is_platform, trim_opt, update_consequences,
    validate_context_params, AnalysisCounters, BatchBlocked, BatchFailure, BatchPreviewVO,
    BatchResultVO, CategoryImportDTO, CategoryImportNodeResult, CategoryImportPreviewVO,
    CategoryNodeState, EncodingDecision, MediaAssetVO, MediaGroupVO, MediaOperationDTO,
    MediaQueryDTO, MediaUploadFileDTO, MediaUploadRequestDTO, MediaUploadResultVO,
    ProductAnalysisVO, ProductCategoryVO, ProductContextFallback, ProductContextMetrics,
    ProductContextVO, ProductDetailSku, ProductDetailVO, ProductDraftDTO, ProductImportItem,
    ProductImportResultVO, ProductImportVerification, ProductMediaDraftDTO, ProductRequirementField,
    ProductRequirementsVO, ProductScope, ProductSearchVO, ProductSummaryVO, ProductValidationItem,
    ProductValidationVO, ProductWriteResultVO, PromotionLinksVO, NexTradeConfigVO,
    WorkBuddyErrorBody, WorkBuddyKeyStatusVO, ANALYSIS_DATA_SCOPE, ANALYSIS_DEFAULT_SAMPLE_SIZE,
    ANALYSIS_FALLBACK_MESSAGE, ANALYSIS_OPERATIONS, ANALYSIS_RANKING_LIMIT, BATCH_MAX_IDS,
    BATCH_SORT_LIMIT, CATEGORY_IMPORT_PAGE_SIZE, CHECK_STATUS_APPROVED, CHECK_STATUS_REJECTED,
    CODE_ALREADY_RECYCLED, CODE_AUDIT_PENDING, CODE_AUDIT_REJECTED, CODE_BATCH_ITEM_BLOCKED,
    CODE_BATCH_LIMIT_EXCEEDED, CODE_CATEGORY_CREATE_FAILED, CODE_CATEGORY_ID_NOT_FOUND,
    CODE_CATEGORY_NOT_FOUND, CODE_CATEGORY_NOT_LEAF, CODE_CONFIRMATION_REQUIRED,
    CODE_EMPTY_PRODUCT_IDS, CODE_FILE_TOKEN_EXPIRED, CODE_IDEMPOTENCY_KEY_CONFLICT,
    CODE_IDEMPOTENCY_KEY_REQUIRED, CODE_INVALID_ARGUMENT, CODE_INVALID_BASE64,
    CODE_INVALID_CATEGORY_ID,
    CODE_INVALID_FILE, CODE_INVALID_FILE_COUNT, CODE_INVALID_GROUP, CODE_INVALID_MEDIA,
    CODE_INVALID_OPERATION, CODE_INVALID_PATH, CODE_INVALID_PRODUCT_ID, CODE_INVALID_ROLE,
    CODE_INVALID_SAMPLE_SIZE, CODE_INVALID_SCOPE, CODE_INVALID_SHOP_ID, CODE_INVALID_TIME_RANGE,
    CODE_INVALID_TYPE, CODE_INVALID_URL, CODE_LOCAL_PATH_NOT_ALLOWED, CODE_MEDIA_IN_USE,
    CODE_MEDIA_SCOPE_MISMATCH, CODE_MISSING_FILTER, CODE_MISSING_PARENT, CODE_MISSING_VALUE,
    CODE_MUST_UNLIST_FIRST, CODE_NOT_RECYCLED, CODE_NOT_FOUND, CODE_NO_CHANGES,
    CODE_OPERATION_FAILED, CODE_PLATFORM_SCOPE_REQUIRED, CODE_PREVIEW_EXPIRED,
    CODE_PREVIEW_MISMATCH, CODE_PREVIEW_NOT_FOUND, CODE_PREVIEW_NOT_IMPORTABLE,
    CODE_PREVIEW_REQUIRED, CODE_PREVIEW_REQUEST_MISMATCH, CODE_PREVIEW_SCOPE_MISMATCH,
    CODE_PRODUCT_CREATE_FAILED, CODE_PRODUCT_NOT_FOUND, CODE_PRODUCT_SN_EXIST,
    CODE_PRODUCTS_REQUIRED, CODE_PRODUCTS_TOO_MANY, CODE_REQUEST_ID_REQUIRED, CODE_REQUIRED,
    CODE_SERVICE_UNAVAILABLE, CODE_SPEC_DUPLICATED, CODE_SPEC_NAME_MISMATCH, CODE_SPEC_REQUIRED,
    CODE_SKU_SN_DUPLICATED,
    CODE_SKU_SN_EXIST, CODE_SKU_SN_TOO_LONG, CODE_STORAGE_NOT_CONFIGURED, CODE_UNSUPPORTED_MIME,
    CODE_UPLOAD_MODE_REQUIRED, CODE_UPLOAD_RETRYABLE, CONFIG_CODE_CUSTOM_PRODUCT_NAME_LENGTH,
    CONFIG_CODE_IMAGE_UPLOAD_MAX_SIZE, CONFIG_CODE_MARKET_PRICE_RATE,
    CONFIG_CODE_WORKBUDDY_API_KEY, CONFIG_CODE_WORKBUDDY_API_KEY_EXPIRES_AT, CONTEXT_SAMPLES_CAP,
    DEFAULT_MULTIPART_MAX_FILE_SIZE, DEFAULT_MARKET_PRICE_RATE_STR, DOMINANT_ENCODING_THRESHOLD,
    EDITABLE_FIELDS, MAX_FILES, MAX_PRODUCTS, MAX_SKU_SN_LENGTH, MEDIA_SUPPORTED_OPERATIONS,
    MIN_DOMINANT_ENCODING_SAMPLES, MSG_COPY_CREATED_UNKNOWN, MSG_COPY_NEED_LIST,
    MSG_COPY_NEW_FIRST, MSG_DELETE_PERMANENT, MSG_ENCODING_DOMINANT_EXACT,
    MSG_ENCODING_DOMINANT_MAJORITY, MSG_ENCODING_INCONSISTENT, MSG_ENCODING_OVERFLOW,
    MSG_ENCODING_TOO_FEW, MSG_KEY_INVALID, MSG_OFF_SALE_B2C_GROUP_BUY, MSG_OFF_SALE_CONSEQUENCE,
    MSG_ON_SALE_CONSEQUENCE, MSG_RECOMMEND_CATEGORY, MSG_RECOMMEND_CATEGORY_REASON,
    MSG_RECOMMEND_CLEARANCE, MSG_RECOMMEND_PRICING, MSG_RECOMMEND_PRICING_REASON,
    MSG_RECOMMEND_REPLENISH, MSG_RECYCLE_B2C_UNLISTED, MSG_RECYCLE_CONSEQUENCE,
    MSG_RESTORE_CONSEQUENCE, PREVIEW_TTL_MILLIS, PRODUCT_STATUS_ON_SALE, PRODUCT_TYPE_NORMAL,
    RESULT_TTL_MILLIS, SKU_TYPE_MULTI, SKU_TYPE_SINGLE, TOKEN_TTL_MILLIS,
};
use nextrade_core::models::openapi::workbuddy::dec;
use nextrade_infrastructure::db::repo::openapi::workbuddy::{
    NewProduct, ProductRow, ProductSearchQuery, WorkbuddyRepository,
};
use nextrade_infrastructure::db::Repositories;

// ——————————————————————————————————————————————
// 错误类型
// ——————————————————————————————————————————————

/// WorkBuddy 开放接口的业务异常。
///
/// 对应 Java `com.nextrade.service.workbuddy.WorkBuddyApiException`
/// （继承 `GlobalException`，`code` 复用为 HTTP 状态码，
/// 额外带 `errorCode` 与 `retryable`）。
///
/// `http_status` 由 [`resolve_http_status`] 归一（白名单外 ⇒ 422）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkBuddyError {
    pub error_code: String,
    pub message: String,
    pub http_status: i32,
    pub retryable: bool,
}

impl WorkBuddyError {
    /// 构造（对应 Java `new WorkBuddyApiException(errorCode, message, httpStatus, retryable)`）。
    #[must_use]
    pub fn new(error_code: &str, message: &str, http_status: i32, retryable: bool) -> Self {
        Self {
            error_code: error_code.to_string(),
            message: message.to_string(),
            http_status,
            retryable,
        }
    }

    /// 400，且**不可**重试（Java 里最常用的形态）。
    #[must_use]
    pub fn bad_request(error_code: &str, message: &str) -> Self {
        Self::new(error_code, message, 400, false)
    }

    /// 401（密钥类错误，不可重试）。
    #[must_use]
    pub fn unauthorized(message: &str) -> Self {
        Self::new(MSG_KEY_INVALID, message, 401, false)
    }

    /// 403（租户作用域不匹配）。
    #[must_use]
    pub fn forbidden(error_code: &str, message: &str) -> Self {
        Self::new(error_code, message, 403, false)
    }

    /// 404。
    #[must_use]
    pub fn not_found(error_code: &str, message: &str) -> Self {
        Self::new(error_code, message, 404, false)
    }

    /// 422（Java `WorkBuddyApiExceptionHandler` 的归一化状态码）。
    #[must_use]
    pub fn unprocessable(error_code: &str, message: &str) -> Self {
        Self::new(error_code, message, 422, false)
    }

    /// 转成对外错误体（对应 Java `ErrorBody`）。
    #[must_use]
    pub fn body(&self) -> WorkBuddyErrorBody {
        WorkBuddyErrorBody::new(&self.error_code, &self.message, self.retryable)
    }

    /// 归一化后的真实 HTTP 状态码。
    #[must_use]
    pub fn status(&self) -> i32 {
        resolve_http_status(self.http_status)
    }
}

impl std::fmt::Display for WorkBuddyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.error_code, self.message)
    }
}

impl std::error::Error for WorkBuddyError {}

impl From<WorkBuddyError> for nextrade_core::Error {
    fn from(e: WorkBuddyError) -> Self {
        nextrade_core::Error::Biz {
            code: e.status(),
            msg: e.message,
        }
    }
}

/// Java `WorkBuddyApiExceptionHandler.resolveStatus` 的本地别名。
#[must_use]
pub const fn resolve_http_status(business_code: i32) -> i32 {
    nextrade_core::models::openapi::workbuddy::resolve_http_status(business_code)
}

/// 服务层结果别名（避免与 `nextrade_core::Result` 冲突时仍需显式导入）。
pub type WbResult<T> = std::result::Result<T, WorkBuddyError>;

/// 把仓储错误转成「服务内部错误」。
///
/// Java 的 `handleException` 兜底分支返回 500 + `INTERNAL_ERROR` + `retryable=true`。
fn internal(e: nextrade_core::Error) -> WorkBuddyError {
    WorkBuddyError::new(
        nextrade_core::models::openapi::workbuddy::CODE_INTERNAL_ERROR,
        &format!("{}: {e}", nextrade_core::models::openapi::workbuddy::MSG_INTERNAL_ERROR),
        500,
        true,
    )
}

// ——————————————————————————————————————————————
// 进程内状态（对应 Java 的 ConcurrentHashMap）
// ——————————————————————————————————————————————

/// 商品导入预览（Java `previews`，TTL 30 分钟）。
#[derive(Debug, Clone)]
struct StoredPreview {
    preview_id: String,
    request_id: Option<String>,
    shop_id: i32,
    /// 草稿在**校验后归一化**的形态（Java 存的是校验结果）。
    items: Vec<ProductDraftDTO>,
    missing_fields: Vec<String>,
    /// 校验是否全部通过。
    importable: bool,
    created_at_ms: i64,
    /// `confirmation == true` 才算已确认（Java `Preview.confirmed`）。
    confirmed: bool,
}

/// 幂等记录（Java `idem`，TTL 24 小时）。
#[derive(Debug, Clone)]
struct StoredIdempotency {
    fingerprint: String,
    result: String,
    created_at_ms: i64,
}

/// 批量操作快照（Java `batchSnapshots`，TTL 30 分钟）。
#[derive(Debug, Clone)]
struct StoredBatchSnapshot {
    fingerprint: String,
    ids: Vec<i32>,
    created_at_ms: i64,
}

/// 上传令牌（Java `staged`，TTL 30 分钟）。
#[derive(Debug, Clone)]
struct StoredUploadToken {
    file_token: String,
    shop_id: i32,
    file_name: String,
    mime_type: String,
    size: i64,
    created_at_ms: i64,
}

/// 分类导入预览（Java `previews`，TTL 30 分钟）。
#[derive(Debug, Clone)]
struct StoredCategoryPreview {
    preview_id: String,
    request_id: String,
    shop_id: i32,
    nodes: Vec<CategoryImportNodeResult>,
    /// 可导入的展平节点（`(path, parent_path, name, explicit_id)`）。
    importable: Vec<(String, String, String, Option<i32>)>,
    importable_flag: bool,
    created_at_ms: i64,
}

/// 媒体操作预览（Java `MediaManagementServiceImpl.previews`，TTL 30 分钟）。
#[derive(Debug, Clone)]
struct StoredMediaPreview {
    preview_id: String,
    fingerprint: String,
    shop_id: i32,
    operation: String,
    result: serde_json::Value,
    created_at_ms: i64,
}

/// 分类导入结果缓存（Java `results`，TTL 24 小时）。
#[derive(Debug, Clone)]
struct StoredCategoryResult {
    result: serde_json::Value,
    created_at_ms: i64,
}

/// WorkBuddy MCP 域服务（挂载于 `AdminState.workbuddy`）。
///
/// 进程内状态与 Java 的 `ConcurrentHashMap` 一一对应，
/// 用 `Mutex` 保证并发安全（Java 用 `ConcurrentHashMap`）。
pub struct WorkbuddyService {
    repos: Arc<Repositories>,
    previews: Mutex<HashMap<String, StoredPreview>>,
    idem: Mutex<HashMap<String, StoredIdempotency>>,
    batch_snapshots: Mutex<HashMap<String, StoredBatchSnapshot>>,
    staged_tokens: Mutex<HashMap<String, StoredUploadToken>>,
    category_previews: Mutex<HashMap<String, StoredCategoryPreview>>,
    category_results: Mutex<HashMap<String, StoredCategoryResult>>,
    media_previews: Mutex<HashMap<String, StoredMediaPreview>>,
}

impl WorkbuddyService {
    /// 构造（与 product_manage/075 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self {
            repos,
            previews: Mutex::new(HashMap::new()),
            idem: Mutex::new(HashMap::new()),
            batch_snapshots: Mutex::new(HashMap::new()),
            staged_tokens: Mutex::new(HashMap::new()),
            category_previews: Mutex::new(HashMap::new()),
            category_results: Mutex::new(HashMap::new()),
            media_previews: Mutex::new(HashMap::new()),
        }
    }

    /// 仓储访问（便于测试与复用）。
    #[must_use]
    pub fn repos(&self) -> &Arc<Repositories> {
        &self.repos
    }

    /// 本域仓储的便捷取用。
    fn r(&self) -> &WorkbuddyRepository {
        &self.repos.openapi.workbuddy
    }

    /// 当前毫秒时间戳（Java `System.currentTimeMillis()`）。
    fn now_ms() -> i64 {
        chrono::Utc::now().timestamp_millis()
    }

    // ————————————————————— 密钥鉴权 —————————————————————

    /// 读 `workbuddyApiKey` 与 `workbuddyApiKeyExpiresAt` 后判定密钥状态
    /// （对应 Java `WorkBuddyKeyServiceImpl.status`）。
    pub async fn key_status(&self, key: Option<&str>) -> WbResult<WorkBuddyKeyStatusVO> {
        let expected = self
            .r()
            .find_config_value(CONFIG_CODE_WORKBUDDY_API_KEY)
            .await
            .map_err(internal)?;
        let expires = self
            .r()
            .find_config_value(CONFIG_CODE_WORKBUDDY_API_KEY_EXPIRES_AT)
            .await
            .map_err(internal)?;
        Ok(evaluate_key_status(
            key,
            expected.as_deref(),
            expires.as_deref(),
            chrono::Utc::now().timestamp(),
        ))
    }

    /// `auth(r, scope)`：先取状态，`null || !valid || shopId == null` 时抛
    /// 401（对应 Java `WorkBuddyProductImportController.auth`）。
    ///
    /// ⚠️ `WorkBuddyKeyService.requireValid(key, shopId, requiredScope)` 在
    /// **单密钥模式**下**不**校验 `requiredScope`（只校验密钥本身），
    /// 故 `scope` 参数在此仅作签名保留。
    pub async fn require_key(&self, key: Option<&str>) -> WbResult<WorkBuddyKeyStatusVO> {
        let status = self.key_status(key).await?;
        if key_authorized(Some(&status)) {
            Ok(status)
        } else {
            Err(WorkBuddyError::unauthorized(key_unauthorized_message(
                Some(&status),
            )))
        }
    }

    /// 租户作用域：平台账号（`shopId == 0`）返回 `0`，
    /// 店铺账号返回其 `shopId`（对应 Java `HeaderUtils.getShopId()`）。
    fn scope_shop_id(status: &WorkBuddyKeyStatusVO) -> i32 {
        status.shop_id.unwrap_or(0)
    }

    // ————————————————————— MessagePushController —————————————————————

    /// `GET /adminapi/messagepush/messagepush/nextradeConfig`
    /// （对应 Java `MessagePushController.nextradeConfig`）。
    ///
    /// ⚠️ Java 直接返回 `String`（纯文本，**不是** JSON 对象）；
    /// Rust 侧统一用 `NexTradeConfigVO` 承载该文本，见 `uncertain_points`。
    ///
    /// 配置读取与 `application.yaml` 的 `nextrade.*` 一致：
    /// `isMerchant: 1`、`isB2b: 1`、`isPro: 1`、其它为 0。
    #[must_use]
    pub fn nextrade_config(&self) -> NexTradeConfigVO {
        // Java 从 `NexTradeProperties`（配置前缀 `nextrade`）取值。
        // 本片未读到 `NexTradeProperties` 的**运行时**赋值链路
        // （它由 `application.yaml` + Spring 绑定），
        // 故此处按该 yaml 的声明值构造，见 `uncertain_points`。
        nextrade_core::models::openapi::workbuddy::nextrade_config_text(
            Some(1), // is-merchant
            Some(1), // is-b2b
            Some(1), // is-pro
            Some(0), // is-overseas
            Some(0), // is-vendor
            Some(0), // is-o2o
        )
        .into()
    }

    /// `GET /adminapi/messagepush/messagepush/push`
    /// （对应 Java `MessagePushController.sendMessage`）。
    ///
    /// Java 调用 `messageService.sendUserMessage(userId, orderId, NEW_ORDER_SHOP)`；
    /// 该服务的 4 参重载在 `orderId < 0` 时做「补发」特判，否则按订单归属校验。
    ///
    /// ⚠️ 六个下游投递通道（站内信/短信/公众号/小程序/邮件/钉钉/企业微信）
    /// 本片未读到实现，故此处**只做入参与归属校验**并返回成功，
    /// 不做实际投递，见 `uncertain_points`。
    pub async fn message_push(&self, user_id: i32, order_id: i32) -> WbResult<()> {
        // 归属校验：orderId >= 0 时必须能在库中查到且 user_id 匹配。
        if order_id >= 0 {
            match self.r().find_order_user_id(order_id).await.map_err(internal)? {
                None => {
                    return Err(WorkBuddyError::not_found(
                        CODE_NOT_FOUND,
                        &format!("找不到订单（ID {order_id}）"),
                    ))
                }
                Some(owner) if owner != user_id => {
                    return Err(WorkBuddyError::forbidden(
                        CODE_INVALID_ARGUMENT,
                        &format!("订单（ID {order_id}）不属于用户 {user_id}"),
                    ))
                }
                Some(_) => {}
            }
        } else {
            // Java 在 `orderId < 0` 时把 `userId` 与 `orderId` 直接塞进订单对象
            // （补发路径），不做归属校验。
        }
        Ok(())
    }

    // ————————————————————— MCP 连接 / 能力 —————————————————————

    /// `GET /openapi/mcp/v1/connection/status`
    /// （对应 Java `WorkBuddyProductImportController.connectionStatus`）。
    ///
    /// 未通过鉴权时 Java 抛 401 并由 `WorkBuddyApiExceptionHandler` 直出错误体；
    /// 本方法把失败也编码进成功返回，由路由层决定是否直出错误码，
    /// 以匹配「连接状态查询本身要能报告未连接」的语义。
    pub async fn connection_status(&self, key: Option<&str>) -> WorkBuddyKeyStatusVO {
        self.key_status(key)
            .await
            .unwrap_or_else(|e| WorkBuddyKeyStatusVO::invalid(&e.message))
    }

    /// `GET /openapi/mcp/v1/import/requirements`
    /// （对应 Java `WorkBuddyProductImportController.requirements`）。
    ///
    /// ⚠️ Java `requirements()` 末尾显式 `v.setTemplate(null)`
    /// （模板已废用），故返回值**不含** `template` 键。
    #[must_use]
    pub fn requirements(&self) -> ProductRequirementsVO {
        let section_omitted = ProductRequirementField::SECTION_OMITTED;
        let section_required = ProductRequirementField::SECTION_REQUIRED;

        let fields = vec![
            ProductRequirementField::new("productName", "商品名称", "string", true)
                .description("商品在前台展示的名称")
                .constraint("长度不超过配置的 customProductNameLength（默认 200）")
                .example("纯棉短袖T恤 男款")
                .template_section(section_required),
            ProductRequirementField::new("productSn", "商品货号", "string", true)
                .description("商品的唯一货号，用于库存与对账")
                .constraint("全站唯一；多规格时可作为规格编码前缀")
                .example("TS2026-001")
                .template_section(section_required),
            ProductRequirementField::new("productPrice", "售价", "number", true)
                .description("商品的本店售价")
                .constraint("大于 0，最多两位小数")
                .example("99.00")
                .template_section(section_required),
            ProductRequirementField::new("categoryId", "商品分类", "integer", false)
                .description("分类 ID；与 categoryName 二选一")
                .constraint("必须是已存在的叶子分类")
                .example("12")
                .template_section(section_required),
            ProductRequirementField::new("categoryName", "分类名称", "string", false)
                .description("分类名称路径，用 / 或 | 分隔；与 categoryId 二选一")
                .constraint("逐级匹配已有分类时可用")
                .example("女鞋/运动系列")
                .template_section(section_required),
            ProductRequirementField::new("images", "商品图片", "array<string>", true)
                .description("商品图片 URL 数组，首张为主图")
                .constraint("必须是 http:// 或 https:// 开头的可访问地址")
                .example(r#"["https://cdn.example.com/a.jpg"]"#)
                .template_section(section_required),
            ProductRequirementField::new("productStock", "库存", "integer", false)
                .description("单规格商品的库存；多规格时按规格分别给")
                .constraint("不小于 0；缺省按 0 处理")
                .default_value("0")
                .template_example("100")
                .template_section(section_omitted),
            ProductRequirementField::new("productStatus", "上架状态", "integer", false)
                .description("1 上架，0 下架")
                .constraint("缺省为 1（上架）")
                .default_value("1")
                .template_example("1")
                .template_section(section_omitted),
            ProductRequirementField::new("productBrief", "商品简介", "string", false)
                .description("列表页展示的一句话简介")
                .constraint("建议不超过 200 字")
                .template_section(section_omitted),
            ProductRequirementField::new("productDesc", "商品详情", "string", false)
                .description("详情页富文本或纯文本")
                .template_section(section_omitted),
            ProductRequirementField::new("skus", "多规格数据", "array<object>", false)
                .description("多规格商品的规格组合；单规格商品留空")
                .constraint("同一批 SKU 的 specs 键必须一致；价格/库存分别给")
                .template_section(section_omitted),
            ProductRequirementField::new("media", "结构化素材", "array<object>", false)
                .description("图片/视频素材引用；等价于 images 但带角色")
                .constraint("type=image 且 role=main_image/gallery_image 计入商品图")
                .template_section(section_omitted),
            ProductRequirementField::new("marketPrice", "市场价", "number", false)
                .description("前台的划线价")
                .constraint("不给时按 售价 × marketPriceRate（默认 1.2）推算")
                .template_section(section_omitted),
            ProductRequirementField::new("keywords", "关键词", "string", false)
                .description("搜索引擎关键词，逗号分隔")
                .template_section(section_omitted),
        ];

        ProductRequirementsVO {
            fields,
            workflow: vec![
                "先用 validate 校验，拿到 previewId".to_string(),
                "把预览结果展示给用户确认".to_string(),
                "带上 confirmation=true 与 previewId 调用 import".to_string(),
                "同一批商品重复提交时带上相同的 Idempotency-Key".to_string(),
            ],
            unsupported: vec![
                "external_market_data".to_string(),
                "content_generation".to_string(),
                "price_negotiation".to_string(),
            ],
        }
    }

    /// `GET /openapi/mcp/v1/capabilities`
    /// （对应 Java `WorkBuddyProductImportController.capabilities`）。
    pub async fn capabilities(&self, key: Option<&str>) -> WbResult<serde_json::Value> {
        let status = self.require_key(key).await?;
        let shop_id = Self::scope_shop_id(&status);

        let max_file_bytes = parse_size_bytes(
            self.config_value(CONFIG_CODE_IMAGE_UPLOAD_MAX_SIZE).await.as_deref(),
        );
        let multipart = parse_size_bytes(Some(DEFAULT_MULTIPART_MAX_FILE_SIZE));
        let image_bytes = image_max_bytes(
            self.config_value(CONFIG_CODE_IMAGE_UPLOAD_MAX_SIZE)
                .await
                .as_deref(),
            multipart,
        );

        let mut media = capabilities_media_defaults();
        if let Some(obj) = media.as_object_mut() {
            // Java 用配置值覆盖默认的字节上限。
            obj.insert("maxFileBytes".to_string(), json!(max_file_bytes));
            obj.insert("imageMaxFileBytes".to_string(), json!(image_bytes));
            obj.insert("videoMaxFileBytes".to_string(), json!(max_file_bytes));
            obj.insert("defaultGroupId".to_string(), json!(shop_id));
        }

        let mut skeleton = capabilities_skeleton();
        if let Some(obj) = skeleton.as_object_mut() {
            obj.insert("media".to_string(), media);
            obj.insert("assistant".to_string(), assistant_defaults());
            obj.insert(
                "tenant".to_string(),
                json!({
                    "shopId": shop_id,
                    "platform": shop_id == 0,
                }),
            );
        }
        Ok(skeleton)
    }

    /// 读单个配置值（失败时静默为 `None`，与 Java
    /// `WorkBuddyKeyServiceImpl.configValue` 的 catch 行为一致）。
    async fn config_value(&self, code: &str) -> Option<String> {
        self.r().find_config_value(code).await.ok().flatten()
    }

    // ————————————————————— 商品分类（只读） —————————————————————

    /// `GET /openapi/mcp/v1/categories`
    /// （对应 Java `WorkBuddyProductImportController.categories`）。
    pub async fn categories(
        &self,
        key: Option<&str>,
        query: &CategoryQuery,
    ) -> WbResult<Vec<ProductCategoryVO>> {
        self.require_key(key).await?;
        let rows = self
            .r()
            .find_categories(
                trim_opt(query.keyword.as_deref()).as_deref(),
                query.parent_id,
                i64::from(normalize_page_size(query.size)),
            )
            .await
            .map_err(internal)?;

        let ids: Vec<i32> = rows.iter().map(|c| c.category_id).collect();
        let with_children = self
            .r()
            .categories_with_children(&ids)
            .await
            .map_err(internal)?;
        // 路径需要全量分类（父级链），单次拉取并缓存本次请求内使用。
        let all = self
            .r()
            .find_all_categories(CATEGORY_IMPORT_PAGE_SIZE)
            .await
            .map_err(internal)?;
        let by_id: BTreeMap<i32, (String, i32)> = all
            .into_iter()
            .map(|c| (c.category_id, (c.category_name, c.parent_id)))
            .collect();

        Ok(rows
            .into_iter()
            .map(|c| ProductCategoryVO {
                category_id: Some(c.category_id),
                category_name: Some(c.category_name.clone()),
                path: Some(build_category_path(&by_id, c.category_id)),
                parent_id: Some(c.parent_id),
                has_children: with_children.contains(&c.category_id),
            })
            .collect())
    }

    // ————————————————————— 商品列表 / 详情 —————————————————————

    /// `GET /openapi/mcp/v1/products`
    /// （对应 Java `WorkBuddyProductManageController.list`）。
    pub async fn product_list(
        &self,
        key: Option<&str>,
        query: &ProductListQuery,
    ) -> WbResult<ProductSearchVO> {
        let status = self.require_key(key).await?;

        let scope = ProductScope::parse(query.scope.as_deref()).ok_or_else(|| {
            WorkBuddyError::bad_request(
                CODE_INVALID_SCOPE,
                &format!(
                    "scope 参数不支持：{}，可用的有：{}",
                    query.scope.as_deref().unwrap_or(""),
                    ProductScope::supported_values()
                ),
            )
        })?;
        let sort_field = resolve_sort_field(query.sort_by.as_deref())
            .map_err(|e| WorkBuddyError::bad_request(CODE_INVALID_ARGUMENT, &e))?;

        let page = normalize_page(query.page);
        let size = normalize_page_size(query.size);
        let shop_id = if query.mine_only == Some(true) {
            let s = Self::scope_shop_id(&status);
            if s > 0 {
                Some(s)
            } else {
                None
            }
        } else {
            None
        };

        let q = ProductSearchQuery {
            keyword: trim_opt(query.keyword.as_deref()),
            category_id: query.category_id,
            scope,
            shop_id,
            brand_id: query.brand_id,
            product_status: None,
            in_stock_only: query.in_stock_only == Some(true),
            added_start_time: None,
            added_end_time: None,
            sort_field,
            ascending: query.order.as_deref().is_some_and(|o| o.eq_ignore_ascii_case("asc")),
            page,
            size,
        };
        let (total, rows) = self.r().search_products(&q).await.map_err(internal)?;

        let ids: Vec<i32> = rows.iter().map(|p| p.product_id).collect();
        let sku_counts = self.r().count_skus(&ids).await.map_err(internal)?;
        let cat_ids: Vec<i32> = rows.iter().map(|p| p.category_id).collect();
        let cat_names = self.r().find_category_names(&cat_ids).await.map_err(internal)?;

        let items = rows
            .iter()
            .map(|p| to_summary(p, &sku_counts, &cat_names))
            .collect();

        Ok(ProductSearchVO {
            items,
            total: Some(total),
            page: Some(page),
            size: Some(size),
            scope: Some(scope.label().to_string()),
            sorted_by: sort_field.map(|f| {
                nextrade_core::models::openapi::workbuddy::sorted_by_text(
                    f,
                    query.order.as_deref().is_some_and(|o| o.eq_ignore_ascii_case("asc")),
                )
            }),
        })
    }

    /// `GET /openapi/mcp/v1/products/{id}`
    /// （对应 Java `WorkBuddyProductManageController.detail`）。
    pub async fn product_detail(&self, key: Option<&str>, id: Option<i32>) -> WbResult<ProductDetailVO> {
        let status = self.require_key(key).await?;
        let id = require_product_id(id)
            .map_err(|(c, m)| WorkBuddyError::bad_request(c, m))?;

        let product = self
            .r()
            .find_product(id)
            .await
            .map_err(internal)?
            .ok_or_else(|| {
                WorkBuddyError::not_found(CODE_PRODUCT_NOT_FOUND, &product_not_found_message(id))
            })?;
        self.assert_owned(&product, &status)?;

        let skus = self.r().find_skus(id).await.map_err(internal)?;
        let category_name = self
            .r()
            .find_category_names(&[product.category_id])
            .await
            .map_err(internal)?
            .remove(&product.category_id);
        let brand_name = if product.brand_id > 0 {
            self.r().find_brand_names(&[product.brand_id]).await.map_err(internal)?.remove(&product.brand_id)
        } else {
            None
        };

        Ok(ProductDetailVO {
            product_id: Some(product.product_id),
            product_name: Some(product.product_name.clone()),
            product_sn: Some(product.product_sn.clone()),
            price: Some(product.product_price),
            market_price: Some(product.market_price),
            stock: Some(product.product_stock),
            status: status_label(Some(product.product_status)),
            audit_status: nextrade_core::models::openapi::workbuddy::audit_label(Some(product.check_status)),
            audit_reason: if product.check_status == CHECK_STATUS_REJECTED {
                trim_opt(Some(&product.check_reason))
            } else {
                None
            },
            product_type: product_type_label(product.product_type),
            category_name,
            brand_name,
            brief: trim_opt(Some(&product.product_brief)),
            pic_url: trim_opt(Some(&product.pic_url)),
            sales: Some(product.virtual_sales),
            sort_order: Some(product.sort_order),
            recycled: product.is_delete == 1,
            skus: skus
                .iter()
                .map(|s| ProductDetailSku {
                    sku_id: Some(s.sku_id),
                    specs: trim_opt(Some(&s.sku_value)),
                    sku_sn: s.sku_sn.clone().and_then(|v| trim_opt(Some(&v))),
                    price: s.sku_price,
                    stock: s.sku_stock,
                })
                .collect(),
        })
    }

    /// `GET /openapi/mcp/v1/products/{id}/promotion-links`
    /// （对应 Java `WorkBuddyProductManageController.promotionLinks`）。
    ///
    /// ⚠️ 短链服务 / 小程序码 / 微信短链 API 本片未读到实现，
    /// 这些字段留空并给出 `wechatShortLinkError`，见 `uncertain_points`。
    pub async fn promotion_links(
        &self,
        key: Option<&str>,
        id: Option<i32>,
    ) -> WbResult<PromotionLinksVO> {
        let status = self.require_key(key).await?;
        let id = require_product_id(id).map_err(|(c, m)| WorkBuddyError::bad_request(c, m))?;
        let product = self
            .r()
            .find_product(id)
            .await
            .map_err(internal)?
            .ok_or_else(|| {
                WorkBuddyError::not_found(CODE_PRODUCT_NOT_FOUND, &product_not_found_message(id))
            })?;
        self.assert_owned(&product, &status)?;

        Ok(PromotionLinksVO {
            product_id: Some(product.product_id),
            product_name: Some(product.product_name),
            h5_short_link: None,
            h5_original_link: None,
            pc_short_link: None,
            pc_original_link: None,
            mini_code: None,
            wechat_short_link: None,
            wechat_short_link_error: Some(
                "推广短链与小程序码依赖商城域名与短链服务配置，当前实例未返回".to_string(),
            ),
            poster_available: false,
        })
    }

    /// 店铺隔离校验（对应 Java `ProductManageServiceImpl.assertOwnedBy`）。
    fn assert_owned(&self, p: &ProductRow, status: &WorkBuddyKeyStatusVO) -> WbResult<()> {
        let scope = Self::scope_shop_id(status);
        if is_owned_by(Some(p.shop_id), Some(scope)) {
            Ok(())
        } else {
            Err(WorkBuddyError::not_found(
                CODE_PRODUCT_NOT_FOUND,
                &product_not_found_message(p.product_id),
            ))
        }
    }

    // ————————————————————— 单商品写操作 —————————————————————

    /// `POST /openapi/mcp/v1/products/{id}/field`
    /// （对应 Java `WorkBuddyProductManageController.updateField`）。
    pub async fn update_field(
        &self,
        key: Option<&str>,
        id: Option<i32>,
        body: &FieldBody,
    ) -> WbResult<ProductWriteResultVO> {
        let status = self.require_key(key).await?;
        let id = require_product_id(id).map_err(|(c, m)| WorkBuddyError::bad_request(c, m))?;
        let label = require_editable_field(body.field.as_str())
            .map_err(|e| WorkBuddyError::bad_request(CODE_INVALID_ARGUMENT, &e))?;

        let product = self
            .r()
            .find_product(id)
            .await
            .map_err(internal)?
            .ok_or_else(|| {
                WorkBuddyError::not_found(CODE_PRODUCT_NOT_FOUND, &product_not_found_message(id))
            })?;
        self.assert_owned(&product, &status)?;

        let raw = body.value.as_deref().map(str::trim).unwrap_or("");
        if raw.is_empty() {
            return Err(WorkBuddyError::bad_request(
                CODE_MISSING_VALUE,
                &format!("{label}不能为空"),
            ));
        }

        // 逐字段的记录值、回读值与连带后果。
        let (column, before, after, consequences, stock_after) = match body.field.as_str() {
            "productName" => {
                let before = product.product_name.clone();
                self.r()
                    .update_product_field(id, "product_name", raw, Self::now_ms())
                    .await
                    .map_err(internal)?;
                let after = self.read_back(id, |p| p.product_name.clone()).await?;
                ("商品名称", before, after, Vec::new(), None)
            }
            "productSn" => {
                if self.r().product_sn_exists(raw, Some(id)).await.map_err(internal)? {
                    return Err(WorkBuddyError::bad_request(
                        CODE_PRODUCT_SN_EXIST,
                        &format!("商品货号「{raw}」已被其它商品使用"),
                    ));
                }
                let before = product.product_sn.clone();
                self.r()
                    .update_product_field(id, "product_sn", raw, Self::now_ms())
                    .await
                    .map_err(internal)?;
                let after = self.read_back(id, |p| p.product_sn.clone()).await?;
                ("商品货号", before, after, Vec::new(), None)
            }
            "productPrice" => {
                let value = raw.parse::<Decimal>().map_err(|_| {
                    WorkBuddyError::bad_request(
                        CODE_INVALID_ARGUMENT,
                        &format!("价格「{raw}」不是合法数字"),
                    )
                })?;
                if value < Decimal::ZERO {
                    return Err(WorkBuddyError::bad_request(
                        CODE_INVALID_ARGUMENT,
                        "价格不能为负数",
                    ));
                }
                let before = product.product_price.normalize().to_string();
                self.r()
                    .update_product_field(id, "product_price", raw, Self::now_ms())
                    .await
                    .map_err(internal)?;
                let after = self
                    .read_back(id, |p| p.product_price.normalize().to_string())
                    .await?;
                let cons = update_consequences("productPrice", None);
                ("价格", before, after, cons, None)
            }
            "productStock" => {
                let value = raw.parse::<i32>().map_err(|_| {
                    WorkBuddyError::bad_request(
                        CODE_INVALID_ARGUMENT,
                        &format!("库存「{raw}」不是合法整数"),
                    )
                })?;
                if value < 0 {
                    return Err(WorkBuddyError::bad_request(
                        CODE_INVALID_ARGUMENT,
                        "库存不能为负数",
                    ));
                }
                let before = product.product_stock.to_string();
                self.r()
                    .update_product_field(id, "product_stock", raw, Self::now_ms())
                    .await
                    .map_err(internal)?;
                let after = self.read_back(id, |p| p.product_stock.to_string()).await?;
                let cons = update_consequences("productStock", Some(value));
                ("库存", before, after, cons, Some(value))
            }
            "sortOrder" => {
                let value = raw.parse::<i32>().map_err(|_| {
                    WorkBuddyError::bad_request(
                        CODE_INVALID_ARGUMENT,
                        &format!("排序值「{raw}」不是合法整数"),
                    )
                })?;
                let before = product.sort_order.to_string();
                self.r()
                    .update_product_field(id, "sort_order", raw, Self::now_ms())
                    .await
                    .map_err(internal)?;
                let after = self.read_back(id, |p| p.sort_order.to_string()).await?;
                ("排序值", before, after, Vec::new(), None)
            }
            // `productSku` 是多规格库存，需按规格维度改写；本片未读到
            // Java 该分支的逐条实现，故显式拒绝而不是猜测。
            other => {
                return Err(WorkBuddyError::unprocessable(
                    CODE_OPERATION_FAILED,
                    &format!(
                        "{other}（多规格库存）需要按规格维度改写，当前接口未支持；\
                         请改用商品后台的规格编辑"
                    ),
                ))
            }
        };

        // \"productSku\" 分支的 stock_after 仅用于上面已处理的 productStock。
        let _ = stock_after;

        Ok(ProductWriteResultVO {
            product_id: Some(id),
            product_name: Some(product.product_name),
            changed_field: Some(label.to_string()),
            before: Some(before),
            after: Some(after),
            consequences,
        })
    }

    /// 写后回读单个字段（对应 Java \"updateById\" 后重新 \"getById\"）。
    async fn read_back<F>(&self, id: i32, f: F) -> WbResult<String>
    where
        F: FnOnce(&ProductRow) -> String,
    {
        let p = self
            .r()
            .find_product(id)
            .await
            .map_err(internal)?
            .ok_or_else(|| {
                WorkBuddyError::not_found(CODE_PRODUCT_NOT_FOUND, &product_not_found_message(id))
            })?;
        Ok(f(&p))
    }

    /// `POST /openapi/mcp/v1/products/{id}/status`
    /// （对应 Java `WorkBuddyProductManageController.setStatus`）。
    pub async fn set_status(
        &self,
        key: Option<&str>,
        id: Option<i32>,
        body: &StatusBody,
    ) -> WbResult<ProductWriteResultVO> {
        let status = self.require_key(key).await?;
        let id = require_product_id(id).map_err(|(c, m)| WorkBuddyError::bad_request(c, m))?;
        let on_sale = body.on_sale.unwrap_or(false);

        let product = self
            .r()
            .find_product(id)
            .await
            .map_err(internal)?
            .ok_or_else(|| {
                WorkBuddyError::not_found(CODE_PRODUCT_NOT_FOUND, &product_not_found_message(id))
            })?;
        self.assert_owned(&product, &status)?;

        if on_sale {
            // Java：审核未通过/未审核不允许上架。
            if product.check_status != CHECK_STATUS_APPROVED {
                return Err(WorkBuddyError::bad_request(
                    CODE_AUDIT_REJECTED,
                    "审核未通过或尚未审核，审核通过后才能上架",
                ));
            }
        }

        let before = status_label(Some(product.product_status)).unwrap_or_default();
        self.r()
            .update_product_status(id, on_sale, Self::now_ms())
            .await
            .map_err(internal)?;
        let after = self.read_back(id, |p| status_label(Some(p.product_status)).unwrap_or_default()).await?;

        // 连带后果：上架 / 下架 / 下架且 B2C。
        let mut consequences = if on_sale {
            vec![MSG_ON_SALE_CONSEQUENCE.to_string()]
        } else {
            let mut v = vec![MSG_OFF_SALE_CONSEQUENCE.to_string()];
            // Java `isB2c()`：`isO2o == 0`。
            if nextrade_core::models::openapi::workbuddy::is_b2c(Some(self.is_o2o().await)) {
                v.push(MSG_OFF_SALE_B2C_GROUP_BUY.to_string());
            }
            v
        };
        if before == after {
            consequences = Vec::new();
        }

        Ok(ProductWriteResultVO {
            product_id: Some(id),
            product_name: Some(product.product_name),
            changed_field: Some("上下架状态".to_string()),
            before: Some(before),
            after: Some(after),
            consequences,
        })
    }

    /// 读 `is-o2o` 配置（Java `NexTradeProperties.getIsO2o()`）。
    ///
    /// ⚠️ 该配置在 `application.yaml` 里是 `nextrade.is-o2o: 0`，
    /// 本片未读到运行时绑定链路，故固定按 \"0\"（B2C）处理，
    /// 见 \"uncertain_points\"。
    async fn is_o2o(&self) -> i32 {
        0
    }

    /// `POST /openapi/mcp/v1/products/{id}/recycle`
    /// （对应 Java `WorkBuddyProductManageController.recycle`）。
    pub async fn recycle(
        &self,
        key: Option<&str>,
        id: Option<i32>,
    ) -> WbResult<ProductWriteResultVO> {
        let status = self.require_key(key).await?;
        let id = require_product_id(id).map_err(|(c, m)| WorkBuddyError::bad_request(c, m))?;
        let product = self
            .r()
            .find_product(id)
            .await
            .map_err(internal)?
            .ok_or_else(|| {
                WorkBuddyError::not_found(CODE_PRODUCT_NOT_FOUND, &product_not_found_message(id))
            })?;
        self.assert_owned(&product, &status)?;

        if product.is_delete == 1 {
            return Err(WorkBuddyError::bad_request(
                CODE_ALREADY_RECYCLED,
                "商品已经在回收站",
            ));
        }

        let was_on_sale = product.product_status == PRODUCT_STATUS_ON_SALE;
        self.r()
            .recycle_product(id, Self::now_ms())
            .await
            .map_err(internal)?;

        let mut consequences = vec![MSG_RECYCLE_CONSEQUENCE.to_string()];
        if was_on_sale && nextrade_core::models::openapi::workbuddy::is_b2c(Some(self.is_o2o().await)) {
            consequences.push(MSG_RECYCLE_B2C_UNLISTED.to_string());
        }

        Ok(ProductWriteResultVO {
            product_id: Some(id),
            product_name: Some(product.product_name),
            changed_field: Some("回收站".to_string()),
            before: Some("商品列表".to_string()),
            after: Some("回收站".to_string()),
            consequences,
        })
    }

    /// `POST /openapi/mcp/v1/products/{id}/restore`
    /// （对应 Java `WorkBuddyProductManageController.restore`）。
    pub async fn restore(
        &self,
        key: Option<&str>,
        id: Option<i32>,
    ) -> WbResult<ProductWriteResultVO> {
        let status = self.require_key(key).await?;
        let id = require_product_id(id).map_err(|(c, m)| WorkBuddyError::bad_request(c, m))?;
        let product = self
            .r()
            .find_product(id)
            .await
            .map_err(internal)?
            .ok_or_else(|| {
                WorkBuddyError::not_found(CODE_PRODUCT_NOT_FOUND, &product_not_found_message(id))
            })?;
        self.assert_owned(&product, &status)?;

        if product.is_delete != 1 {
            return Err(WorkBuddyError::bad_request(
                CODE_NOT_RECYCLED,
                "商品不在回收站，无需还原",
            ));
        }

        self.r()
            .restore_product(id, Self::now_ms())
            .await
            .map_err(internal)?;

        Ok(ProductWriteResultVO {
            product_id: Some(id),
            product_name: Some(product.product_name),
            changed_field: Some("回收站".to_string()),
            before: Some("回收站".to_string()),
            after: Some("商品列表（下架）".to_string()),
            consequences: vec![MSG_RESTORE_CONSEQUENCE.to_string()],
        })
    }

    /// `POST /openapi/mcp/v1/products/{id}/copy`
    /// （对应 Java `WorkBuddyProductManageController.copy`）。
    ///
    /// Java 复制为新商品（名称相同、下架、库存 0），并在回捞失败时改变文案。
    /// 本实现的落库范围与 [`Self::import_products`] 一致（见那里的说明）。
    pub async fn copy(
        &self,
        key: Option<&str>,
        id: Option<i32>,
    ) -> WbResult<ProductWriteResultVO> {
        let status = self.require_key(key).await?;
        let id = require_product_id(id).map_err(|(c, m)| WorkBuddyError::bad_request(c, m))?;
        let product = self
            .r()
            .find_product(id)
            .await
            .map_err(internal)?
            .ok_or_else(|| {
                WorkBuddyError::not_found(CODE_PRODUCT_NOT_FOUND, &product_not_found_message(id))
            })?;
        self.assert_owned(&product, &status)?;

        let now = Self::now_ms();
        // 货号必须唯一：追加时间戳后缀。
        let new_sn = format!("{}-copy-{}", product.product_sn, now);
        let mut tx = self.r().pool_clone().begin().await.map_err(|e| internal(nextrade_core::Error::Database(e.to_string())))?;
        let new_id = WorkbuddyRepository::insert_product(
            &mut tx,
            &NewProduct {
                product_name: product.product_name.clone(),
                product_sn: new_sn,
                product_tsn: None,
                // Java：复制出来的新商品库存为 0。
                product_stock: 0,
                product_price: product.product_price,
                market_price: product.market_price,
                shipping_tpl_id: 0,
                // 下架状态。
                product_status: 0,
                product_type: product.product_type,
                delivery_mode: 1,
                sale_mode: 0,
                category_id: product.category_id,
                brand_id: product.brand_id,
                shop_id: product.shop_id,
                shop_category_id: 0,
                check_status: product.check_status,
                check_reason: String::new(),
                product_brief: product.product_brief.clone(),
                product_desc: None,
                pic_url: product.pic_url.clone(),
                pic_thumb: product.pic_url.clone(),
                pic_original: product.pic_url.clone(),
                pic_large: product.pic_url.clone(),
                add_time: i32::try_from(now / 1000).unwrap_or(i32::MAX),
                sort_order: product.sort_order,
                last_update: i32::try_from(now / 1000).unwrap_or(i32::MAX),
                virtual_sales: 0,
            },
        )
        .await
        .map_err(internal)?;
        tx.commit()
            .await
            .map_err(|e| internal(nextrade_core::Error::Database(e.to_string())))?;

        // Java 回捞新商品；此处必然成功（刚插入），保留失败兜底文案。
        let (after_name, consequences) = match self.r().find_product(new_id).await.map_err(internal)? {
            Some(p) => (
                p.product_name,
                vec![MSG_COPY_NEED_LIST.to_string()],
            ),
            None => (
                product.product_name.clone(),
                vec![MSG_COPY_CREATED_UNKNOWN.to_string(), MSG_COPY_NEW_FIRST.to_string()],
            ),
        };

        Ok(ProductWriteResultVO {
            product_id: Some(new_id),
            product_name: Some(after_name),
            changed_field: Some("复制商品".to_string()),
            before: Some(product.product_name),
            after: Some(format!("新商品（ID {new_id}，下架，库存 0）")),
            consequences,
        })
    }

    /// `POST /openapi/mcp/v1/products/{id}/delete`
    /// （对应 Java `WorkBuddyProductManageController.deletePermanently`）。
    ///
    /// Java 要求先移入回收站，再彻底删除。
    pub async fn delete_permanently(
        &self,
        key: Option<&str>,
        id: Option<i32>,
    ) -> WbResult<ProductWriteResultVO> {
        let status = self.require_key(key).await?;
        let id = require_product_id(id).map_err(|(c, m)| WorkBuddyError::bad_request(c, m))?;
        let product = self
            .r()
            .find_product(id)
            .await
            .map_err(internal)?
            .ok_or_else(|| {
                WorkBuddyError::not_found(CODE_PRODUCT_NOT_FOUND, &product_not_found_message(id))
            })?;
        self.assert_owned(&product, &status)?;

        if product.is_delete != 1 {
            return Err(WorkBuddyError::bad_request(
                CODE_MUST_UNLIST_FIRST,
                "需先移入回收站再删除",
            ));
        }

        let mut tx = self
            .r()
            .pool_clone()
            .begin()
            .await
            .map_err(|e| internal(nextrade_core::Error::Database(e.to_string())))?;
        WorkbuddyRepository::delete_product_cascade(&mut tx, id)
            .await
            .map_err(internal)?;
        tx.commit()
            .await
            .map_err(|e| internal(nextrade_core::Error::Database(e.to_string())))?;

        Ok(ProductWriteResultVO {
            product_id: Some(id),
            product_name: Some(product.product_name),
            changed_field: Some("彻底删除".to_string()),
            before: Some("回收站".to_string()),
            after: Some("已删除".to_string()),
            consequences: vec![MSG_DELETE_PERMANENT.to_string()],
        })
    }

    // ————————————————————— 批量操作 —————————————————————

    /// `POST /openapi/mcp/v1/products/batch/preview`
    /// （对应 Java `WorkBuddyProductManageController.batchPreview`）。
    pub async fn batch_preview(
        &self,
        key: Option<&str>,
        body: &BatchBody,
    ) -> WbResult<BatchPreviewVO> {
        let status = self.require_key(key).await?;
        let operation =
            normalize_batch_operation(body.operation.as_deref()).map_err(|(c, m)| {
                WorkBuddyError::bad_request(c, m)
            })?;
        let ids = normalize_batch_ids(body.product_ids.as_deref())
            .map_err(|(c, m)| WorkBuddyError::bad_request(c, m))?;
        if ids.len() > BATCH_MAX_IDS {
            return Err(WorkBuddyError::bad_request(
                CODE_BATCH_LIMIT_EXCEEDED,
                &format!("一次最多处理 {BATCH_MAX_IDS} 个商品"),
            ));
        }

        let rows = self.r().find_products_for_batch(&ids).await.map_err(internal)?;
        let scope = Self::scope_shop_id(&status);

        // Java 在批量里同样做店铺隔离；不属于当前店铺的视为「找不到」。
        let visible: Vec<_> = rows
            .iter()
            .filter(|p| is_owned_by(Some(p.shop_id), Some(scope)))
            .collect();

        // 保持入参顺序。
        let by_id: BTreeMap<i32, &nextrade_infrastructure::db::repo::openapi::workbuddy::BatchProductRow> =
            visible.iter().map(|p| (p.product_id, *p)).collect();

        let mut affected = Vec::new();
        let mut blocked = Vec::new();
        for pid in &ids {
            match by_id.get(pid) {
                None => blocked.push(BatchBlocked {
                    product_id: Some(*pid),
                    product_name: None,
                    reason: Some("找不到该商品，可能已被彻底删除或不属于当前店铺".to_string()),
                }),
                Some(p) => {
                    if let Some(reason) = batch_block_reason(&operation, p.check_status, p.is_delete == 1)
                    {
                        blocked.push(BatchBlocked {
                            product_id: Some(p.product_id),
                            product_name: Some(p.product_name.clone()),
                            reason: Some(reason.to_string()),
                        });
                    } else {
                        affected.push(ProductSummaryVO {
                            product_id: Some(p.product_id),
                            product_name: Some(p.product_name.clone()),
                            product_sn: Some(p.product_sn.clone()),
                            price: Some(p.product_price),
                            stock: Some(p.product_stock),
                            status: status_label(Some(p.product_status)),
                            audit_status: nextrade_core::models::openapi::workbuddy::audit_label(Some(
                                p.check_status,
                            )),
                            product_type: product_type_label(p.product_type),
                            category_name: None,
                            sku_count: None,
                            sales: Some(p.virtual_sales),
                            recycled: p.is_delete == 1,
                        });
                    }
                }
            }
        }

        // Java 的 \"affected\" 上限 20（`limit(20)`），超出部分只计数。
        if affected.len() > BATCH_SORT_LIMIT {
            affected.truncate(BATCH_SORT_LIMIT);
        }

        let preview_id = new_uuid();
        let fingerprint = batch_fingerprint(&operation, &ids);
        if let Ok(mut m) = self.batch_snapshots.lock() {
            evict_expired_batch(&mut m);
            m.insert(
                preview_id.clone(),
                StoredBatchSnapshot {
                    fingerprint,
                    ids: ids.clone(),
                    created_at_ms: Self::now_ms(),
                },
            );
        }

        Ok(BatchPreviewVO {
            preview_id: Some(preview_id),
            operation: Some(batch_operation_label(&operation)),
            consequences: batch_consequences(&operation),
            affected,
            blocked,
            irreversible: operation == "del",
        })
    }

    /// `POST /openapi/mcp/v1/products/batch/execute`
    /// （对应 Java `WorkBuddyProductManageController.batchExecute`）。
    pub async fn batch_execute(
        &self,
        key: Option<&str>,
        body: &BatchExecuteBody,
    ) -> WbResult<BatchResultVO> {
        let status = self.require_key(key).await?;
        if body.confirmation != Some(true) {
            return Err(WorkBuddyError::bad_request(
                CODE_CONFIRMATION_REQUIRED,
                "写入前必须明确 confirmation: true",
            ));
        }
        let operation = normalize_batch_operation(body.operation.as_deref())
            .map_err(|(c, m)| WorkBuddyError::bad_request(c, m))?;
        let ids = normalize_batch_ids(body.product_ids.as_deref())
            .map_err(|(c, m)| WorkBuddyError::bad_request(c, m))?;
        if ids.len() > BATCH_MAX_IDS {
            return Err(WorkBuddyError::bad_request(
                CODE_BATCH_LIMIT_EXCEEDED,
                &format!("一次最多处理 {BATCH_MAX_IDS} 个商品"),
            ));
        }

        let preview_id = body
            .preview_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                WorkBuddyError::bad_request(CODE_PREVIEW_REQUIRED, "请先调用 batch/preview 获取 previewId")
            })?;
        let fingerprint = batch_fingerprint(&operation, &ids);
        let snapshot = self
            .batch_snapshots
            .lock()
            .ok()
            .and_then(|mut m| {
                evict_expired_batch(&mut m);
                m.remove(preview_id)
            })
            .ok_or_else(|| {
                WorkBuddyError::unprocessable(CODE_PREVIEW_EXPIRED, "previewId 已过期或不存在，请重新预览")
            })?;
        if snapshot.fingerprint != fingerprint {
            return Err(WorkBuddyError::bad_request(
                CODE_PREVIEW_MISMATCH,
                "previewId 与本次的商品集合或操作类型不一致，请重新预览",
            ));
        }

        let rows = self.r().find_products_for_batch(&ids).await.map_err(internal)?;
        let scope = Self::scope_shop_id(&status);
        let by_id: BTreeMap<i32, _> = rows.iter().map(|p| (p.product_id, p)).collect();

        let mut affected_ids = Vec::new();
        let mut failed = Vec::new();
        for pid in &ids {
            match by_id.get(pid) {
                None => failed.push(BatchFailure {
                    product_id: Some(*pid),
                    product_name: None,
                    message: Some("找不到该商品，可能已被彻底删除或不属于当前店铺".to_string()),
                }),
                Some(p) => {
                    if !is_owned_by(Some(p.shop_id), Some(scope)) {
                        failed.push(BatchFailure {
                            product_id: Some(*pid),
                            product_name: Some(p.product_name.clone()),
                            message: Some("该商品不属于当前店铺".to_string()),
                        });
                        continue;
                    }
                    if let Some(reason) = batch_block_reason(&operation, p.check_status, p.is_delete == 1)
                    {
                        failed.push(BatchFailure {
                            product_id: Some(*pid),
                            product_name: Some(p.product_name.clone()),
                            message: Some(reason.to_string()),
                        });
                        continue;
                    }
                    affected_ids.push(*pid);
                }
            }
        }

        let now = Self::now_ms();
        if !affected_ids.is_empty() {
            match operation.as_str() {
                "up" => {
                    self.r()
                        .batch_set_status(&affected_ids, true, now)
                        .await
                        .map_err(internal)?;
                }
                "down" => {
                    self.r()
                        .batch_set_status(&affected_ids, false, now)
                        .await
                        .map_err(internal)?;
                }
                "recycle" => {
                    self.r()
                        .batch_set_deleted(&affected_ids, true, false, now)
                        .await
                        .map_err(internal)?;
                }
                "restore" => {
                    self.r()
                        .batch_set_deleted(&affected_ids, false, true, now)
                        .await
                        .map_err(internal)?;
                }
                "del" => {
                    let mut tx = self
                        .r()
                        .pool_clone()
                        .begin()
                        .await
                        .map_err(|e| internal(nextrade_core::Error::Database(e.to_string())))?;
                    for pid in &affected_ids {
                        WorkbuddyRepository::delete_product_cascade(&mut tx, *pid)
                            .await
                            .map_err(internal)?;
                    }
                    tx.commit()
                        .await
                        .map_err(|e| internal(nextrade_core::Error::Database(e.to_string())))?;
                }
                _ => {
                    return Err(WorkBuddyError::bad_request(
                        CODE_INVALID_OPERATION,
                        "仅支持 up、down、recycle、restore、del",
                    ))
                }
            }
        }

        // 回读受影响商品用于结果展示。
        let after_rows = self.r().find_products_for_batch(&affected_ids).await.map_err(internal)?;
        let affected: Vec<ProductSummaryVO> = if operation == "del" {
            // 已删除，回读为空；用执行前的快照出参。
            affected_ids
                .iter()
                .filter_map(|pid| by_id.get(pid).map(|p| summary_from_batch(p)))
                .collect()
        } else {
            after_rows
                .iter()
                .map(summary_from_batch)
                .collect()
        };

        Ok(BatchResultVO {
            operation: Some(batch_operation_label(&operation)),
            success_count: Some(i32::try_from(affected_ids.len()).unwrap_or(i32::MAX)),
            affected,
            consequences: batch_consequences(&operation),
            failed,
        })
    }

    // ————————————————————— 商品导入 —————————————————————

    /// `POST /openapi/mcp/v1/products/validate`
    /// （对应 Java `WorkBuddyProductImportController.validate`）。
    pub async fn validate_products(
        &self,
        key: Option<&str>,
        body: &ProductListBody,
    ) -> WbResult<ProductValidationVO> {
        let status = self.require_key(key).await?;
        let shop_id = Self::scope_shop_id(&status);

        if body.products.is_empty() {
            return Err(WorkBuddyError::bad_request(
                CODE_PRODUCTS_REQUIRED,
                "products 不能为空",
            ));
        }
        if body.products.len() > usize::try_from(MAX_PRODUCTS).unwrap_or(usize::MAX) {
            return Err(WorkBuddyError::bad_request(
                CODE_PRODUCTS_TOO_MANY,
                &format!("一次最多导入 {MAX_PRODUCTS} 个商品"),
            ));
        }

        let max_name_len = nextrade_core::models::openapi::workbuddy::max_product_name_length(
            self.config_value(CONFIG_CODE_CUSTOM_PRODUCT_NAME_LENGTH)
                .await
                .as_deref(),
        );
        let rate = nextrade_core::models::openapi::workbuddy::market_price_rate(
            self.config_value(CONFIG_CODE_MARKET_PRICE_RATE).await.as_deref(),
        );

        let items = self
            .validate_items(&body.products, max_name_len, rate)
            .await?;

        // 并集：所有商品缺失字段的并集（保持首次出现顺序）。
        let mut seen = BTreeSet::new();
        let mut missing_fields = Vec::new();
        for it in &items {
            for f in &it.missing_fields {
                if seen.insert(f.clone()) {
                    missing_fields.push(f.clone());
                }
            }
        }
        let all_valid = items.iter().all(|i| i.valid);
        let preview_id = new_uuid();

        // Java 把校验结果按 previewId 存起来供 import 复用。
        if let Ok(mut m) = self.previews.lock() {
            evict_expired_previews(&mut m);
            m.insert(
                preview_id.clone(),
                StoredPreview {
                    preview_id: preview_id.clone(),
                    request_id: None,
                    shop_id,
                    items: items.iter().map(|i| i.product.clone().unwrap_or_default()).collect(),
                    missing_fields: missing_fields.clone(),
                    importable: all_valid,
                    created_at_ms: Self::now_ms(),
                    confirmed: false,
                },
            );
        }

        Ok(ProductValidationVO {
            preview_id: Some(preview_id),
            total: i32::try_from(body.products.len()).unwrap_or(i32::MAX),
            valid: all_valid,
            missing_fields,
            items,
        })
    }

    /// 逐条校验（Java `validate(item)`）并归一化草稿。
    async fn validate_items(
        &self,
        products: &[ProductDraftDTO],
        max_name_len: i32,
        rate: Decimal,
    ) -> WbResult<Vec<ProductValidationItem>> {
        // 一次取回相关分类，避免逐条查库。
        let all = self
            .r()
            .find_all_categories(CATEGORY_IMPORT_PAGE_SIZE)
            .await
            .map_err(internal)?;
        let by_id: BTreeMap<i32, (String, i32)> = all
            .iter()
            .map(|c| (c.category_id, (c.category_name.clone(), c.parent_id)))
            .collect();
        let leaf_ids: BTreeSet<i32> = {
            let parents: BTreeSet<i32> = all.iter().map(|c| c.parent_id).collect();
            all.iter()
                .map(|c| c.category_id)
                .filter(|id| !parents.contains(id))
                .collect()
        };
        let by_path: BTreeMap<String, i32> = build_path_index(&all);

        // 商品货号查重（含本批内部重复）。
        let sns: Vec<&str> = products
            .iter()
            .filter_map(|p| p.product_sn.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        // ⚠️ \"existing_sns\" 用 `HashSet<&str>`（借用 \"sns\" 里的切片），
        // 与下面 \"batch_sn_count\" 的键类型一致，避免
        // `contains(sn)` 传 `&str` 时对 `HashSet<String>` 报 E0308。
        let mut existing_sns: HashSet<&str> = HashSet::new();
        for sn in &sns {
            if self.r().product_sn_exists(sn, None).await.map_err(internal)? {
                existing_sns.insert(sn);
            }
        }
        let mut batch_sn_count: HashMap<&str, usize> = HashMap::new();
        for sn in &sns {
            *batch_sn_count.entry(sn).or_insert(0) += 1;
        }

        let mut items = Vec::new();
        for (idx, raw) in products.iter().enumerate() {
            let mut item = ProductValidationItem {
                index: i32::try_from(idx).unwrap_or(i32::MAX),
                product_name: trim_opt(raw.product_name.as_deref()),
                product_sn: trim_opt(raw.product_sn.as_deref()),
                valid: true,
                errors: Vec::new(),
                warnings: Vec::new(),
                source: raw.field_sources.clone(),
                product: None,
                missing_fields: Vec::new(),
            };

            // —— productName ——
            match trim_opt(raw.product_name.as_deref()) {
                None => {
                    item.errors
                        .push(nextrade_core::models::openapi::workbuddy::ProductFieldError::new(
                            "productName",
                            CODE_REQUIRED,
                        ));
                    item.missing_fields.push("productName".to_string());
                }
                Some(name) => {
                    if i32::try_from(name.chars().count()).unwrap_or(i32::MAX) > max_name_len {
                        item.errors
                            .push(nextrade_core::models::openapi::workbuddy::ProductFieldError::new(
                                "productName",
                                CODE_INVALID_ARGUMENT,
                            ));
                    }
                }
            }

            // —— productSn ——
            match trim_opt(raw.product_sn.as_deref()) {
                None => {
                    item.errors
                        .push(nextrade_core::models::openapi::workbuddy::ProductFieldError::new(
                            "productSn",
                            CODE_REQUIRED,
                        ));
                    item.missing_fields.push("productSn".to_string());
                }
                Some(sn) => {
                    // `sn` 是 `String`；`HashSet<&str>` / `HashMap<&str, _>`
                    // 的查找键要传 `&str`（`sn.as_str()`），
                    // 直接传 `String` 会报 E0308。
                    if existing_sns.contains(sn.as_str()) {
                        item.errors
                            .push(nextrade_core::models::openapi::workbuddy::ProductFieldError::new(
                                "productSn",
                                CODE_PRODUCT_SN_EXIST,
                            ));
                    } else if batch_sn_count.get(sn.as_str()).copied().unwrap_or(0) > 1 {
                        item.errors
                            .push(nextrade_core::models::openapi::workbuddy::ProductFieldError::new(
                                "productSn",
                                CODE_SKU_SN_DUPLICATED,
                            ));
                    }
                }
            }

            // —— productPrice ——
            match raw.product_price {
                None => {
                    item.errors
                        .push(nextrade_core::models::openapi::workbuddy::ProductFieldError::new(
                            "productPrice",
                            CODE_REQUIRED,
                        ));
                    item.missing_fields.push("productPrice".to_string());
                }
                Some(p) if p <= Decimal::ZERO => {
                    item.errors
                        .push(nextrade_core::models::openapi::workbuddy::ProductFieldError::new(
                            "productPrice",
                            CODE_INVALID_ARGUMENT,
                        ));
                }
                Some(_) => {}
            }

            // —— categoryId / categoryName ——
            let mut resolved_category: Option<i32> = None;
            if let Some(cid) = raw.category_id {
                match by_id.get(&cid) {
                    None => {
                        item.errors
                            .push(nextrade_core::models::openapi::workbuddy::ProductFieldError::new(
                                "categoryId",
                                CODE_CATEGORY_NOT_FOUND,
                            ));
                    }
                    Some(_) => {
                        if !leaf_ids.contains(&cid) {
                            item.errors
                                .push(nextrade_core::models::openapi::workbuddy::ProductFieldError::new(
                                    "categoryId",
                                    CODE_CATEGORY_NOT_LEAF,
                                ));
                        } else {
                            resolved_category = Some(cid);
                        }
                    }
                }
            } else if let Some(name) = trim_opt(raw.category_name.as_deref()) {
                let normalized = name.replace('|', "/");
                match by_path.get(&normalized) {
                    Some(cid) => resolved_category = Some(*cid),
                    None => {
                        item.errors
                            .push(nextrade_core::models::openapi::workbuddy::ProductFieldError::new(
                                "categoryName",
                                CODE_CATEGORY_NOT_FOUND,
                            ));
                    }
                }
            } else {
                item.errors
                    .push(nextrade_core::models::openapi::workbuddy::ProductFieldError::new(
                        "categoryId",
                        CODE_REQUIRED,
                    ));
                item.missing_fields.push("categoryId".to_string());
            }

            // —— images ——
            let images = dedupe_urls(&candidate_images(raw));
            if images.is_empty() || images_all_blank(&images) {
                item.errors
                    .push(nextrade_core::models::openapi::workbuddy::ProductFieldError::new(
                        "images",
                        CODE_REQUIRED,
                    ));
                item.missing_fields.push("images".to_string());
            } else if has_invalid_image(&images) {
                item.errors
                    .push(nextrade_core::models::openapi::workbuddy::ProductFieldError::new(
                        "images",
                        CODE_INVALID_URL,
                    ));
            }

            // —— skus ——
            let mut combined_sn_seen = false;
            if !raw.skus.is_empty() {
                // 规格键必须一致。
                let first_keys: BTreeSet<&String> =
                    raw.skus[0].specs.keys().collect();
                if first_keys.is_empty() {
                    item.errors
                        .push(nextrade_core::models::openapi::workbuddy::ProductFieldError::new(
                            "skus",
                            CODE_SPEC_REQUIRED,
                        ));
                }
                for sku in &raw.skus {
                    let keys: BTreeSet<&String> = sku.specs.keys().collect();
                    if keys != first_keys {
                        item.errors
                            .push(nextrade_core::models::openapi::workbuddy::ProductFieldError::new(
                                "skus",
                                CODE_SPEC_NAME_MISMATCH,
                            ));
                        break;
                    }
                }
                // 规格编码长度与批内重复。
                let mut seen_sn = HashSet::new();
                for sku in &raw.skus {
                    if let Some(sn) = sku.sku_sn.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                        if sn.chars().count() > MAX_SKU_SN_LENGTH {
                            combined_sn_seen = true;
                            item.errors
                                .push(nextrade_core::models::openapi::workbuddy::ProductFieldError::new(
                                    "skus",
                                    CODE_SKU_SN_TOO_LONG,
                                ));
                        }
                        if !seen_sn.insert(sn.to_string()) {
                            combined_sn_seen = true;
                            item.errors
                                .push(nextrade_core::models::openapi::workbuddy::ProductFieldError::new(
                                    "skus",
                                    CODE_SKU_SN_DUPLICATED,
                                ));
                        }
                    }
                }
                // 组合去重：同一组 specs 出现两次。
                let mut combo = HashSet::new();
                for sku in &raw.skus {
                    let key = nextrade_core::models::openapi::workbuddy::build_sku_value(
                        &nextrade_core::models::openapi::workbuddy::sku_pairs(&sku.specs),
                    );
                    if !combo.insert(key) {
                        item.errors
                            .push(nextrade_core::models::openapi::workbuddy::ProductFieldError::new(
                                "skus",
                                CODE_SPEC_DUPLICATED,
                            ));
                        break;
                    }
                }
            }
            let _ = combined_sn_seen;

            item.valid = item.errors.is_empty();
            item.product = Some(normalize_draft(raw, resolved_category, &images, rate));
            items.push(item);
        }
        Ok(items)
    }

    /// `POST /openapi/mcp/v1/products/import`
    /// （对应 Java `WorkBuddyProductImportController.importProducts`）。
    pub async fn import_products(
        &self,
        key: Option<&str>,
        idempotency_key: Option<&str>,
        body: &ImportBody,
    ) -> WbResult<ProductImportResultVO> {
        let status = self.require_key(key).await?;
        let shop_id = Self::scope_shop_id(&status);

        if body.confirmation != Some(true) {
            return Err(WorkBuddyError::bad_request(
                CODE_CONFIRMATION_REQUIRED,
                "写入前必须明确 confirmation: true",
            ));
        }
        if body.products.is_empty() {
            return Err(WorkBuddyError::bad_request(
                CODE_PRODUCTS_REQUIRED,
                "products 不能为空",
            ));
        }

        // 幂等：必须有 Idempotency-Key，且同一键的重复提交返回首次结果。
        let idem_key = idempotency_key
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                WorkBuddyError::bad_request(
                    CODE_IDEMPOTENCY_KEY_REQUIRED,
                    "缺少 Idempotency-Key 请求头",
                )
            })?;
        let fingerprint = format!(
            "{}:{}",
            shop_id,
            serde_json::to_string(&body.products).unwrap_or_default()
        );

        if let Ok(mut m) = self.idem.lock() {
            evict_expired_idem(&mut m);
            if let Some(prev) = m.get(idem_key) {
                if prev.fingerprint != fingerprint {
                    return Err(WorkBuddyError::new(
                        CODE_IDEMPOTENCY_KEY_CONFLICT,
                        "同一 Idempotency-Key 已用于不同的商品集合",
                        409,
                        false,
                    ));
                }
                if let Ok(v) = serde_json::from_str::<ProductImportResultVO>(&prev.result) {
                    return Ok(v);
                }
            }
        }

        // 预览必须存在、属于当前租户、且可导入。
        let preview_id_req = body
            .preview_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                WorkBuddyError::bad_request(CODE_PREVIEW_REQUIRED, "缺少 previewId，请先调用 validate")
            })?;
        let preview = self
            .previews
            .lock()
            .ok()
            .and_then(|mut m| {
                evict_expired_previews(&mut m);
                m.get(preview_id_req).cloned()
            });
        let preview = match preview {
            None => {
                // 没有预览也能导入（Java 允许直接 import），但必须至少过一遍校验。
                None
            }
            Some(p) => {
                if p.shop_id != shop_id {
                    return Err(WorkBuddyError::forbidden(
                        CODE_PREVIEW_SCOPE_MISMATCH,
                        "previewId 不属于当前租户",
                    ));
                }
                if !p.importable {
                    return Err(WorkBuddyError::bad_request(
                        CODE_PREVIEW_NOT_IMPORTABLE,
                        "该预览存在校验错误，修正后才能导入",
                    ));
                }
                Some(p)
            }
        };

        let max_name_len = nextrade_core::models::openapi::workbuddy::max_product_name_length(
            self.config_value(CONFIG_CODE_CUSTOM_PRODUCT_NAME_LENGTH)
                .await
                .as_deref(),
        );
        let rate = nextrade_core::models::openapi::workbuddy::market_price_rate(
            self.config_value(CONFIG_CODE_MARKET_PRICE_RATE).await.as_deref(),
        );

        // 真实落库前再校验一次（防绕过 validate 直接 import）。
        let validated = self.validate_items(&body.products, max_name_len, rate).await?;
        if let Some(v) = validated.iter().find(|i| !i.valid) {
            return Err(WorkBuddyError::bad_request(
                CODE_PREVIEW_NOT_IMPORTABLE,
                &format!(
                    "第 {} 个商品校验未通过：{}",
                    v.index + 1,
                    v.errors
                        .iter()
                        .filter_map(|e| e.field.clone())
                        .collect::<Vec<_>>()
                        .join("、")
                ),
            ));
        }

        let mut items = Vec::new();
        for (idx, draft) in validated.iter().enumerate() {
            let d = draft.product.clone().unwrap_or_default();
            let result = self
                .create_product(&d, shop_id)
                .await;
            match result {
                Ok((pid, sn, pstatus)) => {
                    let verification = self.verify_created(pid, &d).await;
                    items.push(ProductImportItem {
                        index: i32::try_from(idx).unwrap_or(i32::MAX),
                        success: true,
                        product_id: Some(pid),
                        product_name: trim_opt(d.product_name.as_deref()),
                        product_sn: Some(sn),
                        product_status: Some(pstatus),
                        error_code: None,
                        message: None,
                        suggestion: Some(build_import_suggestion(
                            Some(pstatus),
                            d.product_stock,
                            &[],
                        )),
                        retryable: false,
                        verification: Some(verification),
                    });
                }
                Err(e) => {
                    let retryable = e.retryable;
                    items.push(ProductImportItem {
                        index: i32::try_from(idx).unwrap_or(i32::MAX),
                        success: false,
                        product_id: None,
                        product_name: trim_opt(d.product_name.as_deref()),
                        product_sn: trim_opt(d.product_sn.as_deref()),
                        product_status: None,
                        error_code: Some(e.error_code.clone()),
                        message: Some(e.message.clone()),
                        suggestion: None,
                        retryable,
                        verification: None,
                    });
                }
            }
        }

        let success_count = items.iter().filter(|i| i.success).count();
        let total = i32::try_from(items.len()).unwrap_or(i32::MAX);
        let result = ProductImportResultVO {
            request_id: serde_json::to_value(&body.preview_id)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string)),
            preview_id: preview.map(|p| p.preview_id),
            total,
            success_count: i32::try_from(success_count).unwrap_or(i32::MAX),
            failure_count: total - i32::try_from(success_count).unwrap_or(i32::MAX),
            items,
        };

        // 记住本次结果（幂等键 + 24h）。
        if let Ok(mut m) = self.idem.lock() {
            evict_expired_idem(&mut m);
            m.insert(
                idem_key.to_string(),
                StoredIdempotency {
                    fingerprint,
                    result: serde_json::to_string(&result).unwrap_or_default(),
                    created_at_ms: Self::now_ms(),
                },
            );
        }
        Ok(result)
    }

    /// 落库单个商品（对应 Java `ProductImportServiceImpl.createProduct`）。
    ///
    /// ⚠️ Java 会写 80 余列并联动属性/规格/相册表，本片只能确认导入流程
    /// 必需且规则明确的列（见 \"uncertain_points\"）。
    async fn create_product(
        &self,
        d: &ProductDraftDTO,
        shop_id: i32,
    ) -> WbResult<(i32, String, i32)> {
        let sn = trim_opt(d.product_sn.as_deref()).unwrap_or_default();
        if sn.is_empty() {
            return Err(WorkBuddyError::bad_request(CODE_REQUIRED, "商品货号不能为空"));
        }
        let name = trim_opt(d.product_name.as_deref()).unwrap_or_default();
        let price = d.product_price.unwrap_or(Decimal::ZERO);
        let status = d.product_status.unwrap_or(PRODUCT_STATUS_ON_SALE);
        let images = dedupe_urls(&candidate_images(d));
        let main = images.first().cloned().unwrap_or_default();

        let rate = nextrade_core::models::openapi::workbuddy::market_price_rate(
            self.config_value(CONFIG_CODE_MARKET_PRICE_RATE).await.as_deref(),
        );
        let market_price =
            nextrade_core::models::openapi::workbuddy::resolve_market_price(d.market_price, Some(price), rate)
                .unwrap_or(price);

        // 库存：单规格取 productStock；多规格取各规格之和。
        let stock: i32 = if d.skus.is_empty() {
            d.product_stock.unwrap_or(0)
        } else {
            d.skus.iter().map(|s| s.stock.unwrap_or(0)).sum()
        };

        let now = Self::now_ms();
        let mut tx = self
            .r()
            .pool_clone()
            .begin()
            .await
            .map_err(|e| internal(nextrade_core::Error::Database(e.to_string())))?;

        let new_id = WorkbuddyRepository::insert_product(
            &mut tx,
            &NewProduct {
                product_name: name.clone(),
                product_sn: sn.clone(),
                product_tsn: trim_opt(d.product_tsn.as_deref()),
                product_stock: stock,
                product_price: price,
                market_price,
                shipping_tpl_id: 0,
                product_status: status,
                product_type: Some(PRODUCT_TYPE_NORMAL),
                delivery_mode: 1,
                sale_mode: 0,
                category_id: d.category_id.unwrap_or(0),
                brand_id: 0,
                shop_id,
                shop_category_id: 0,
                // 平台账号建的商品直接审核通过（Java 的平台/店铺分支）。
                check_status: CHECK_STATUS_APPROVED,
                check_reason: String::new(),
                product_brief: trim_opt(d.product_brief.as_deref()).unwrap_or_default(),
                product_desc: trim_opt(d.product_desc.as_deref()),
                pic_url: main.clone(),
                pic_thumb: main.clone(),
                pic_original: main.clone(),
                pic_large: main.clone(),
                add_time: i32::try_from(now / 1000).unwrap_or(i32::MAX),
                sort_order: 0,
                last_update: i32::try_from(now / 1000).unwrap_or(i32::MAX),
                virtual_sales: 0,
            },
        )
        .await
        .map_err(internal)?;

        // 商品相册。
        for (i, url) in images.iter().enumerate() {
            WorkbuddyRepository::insert_product_gallery(
                &mut tx,
                new_id,
                url,
                i32::try_from(i).unwrap_or(i32::MAX),
                i64::from(shop_id),
            )
            .await
            .map_err(internal)?;
        }

        tx.commit()
            .await
            .map_err(|e| internal(nextrade_core::Error::Database(e.to_string())))?;

        Ok((new_id, sn, status))
    }

    /// 导入后回读校验（对应 Java `ProductImportResultVO.Verification`）。
    async fn verify_created(&self, product_id: i32, d: &ProductDraftDTO) -> ProductImportVerification {
        let row = match self.r().find_product(product_id).await {
            Ok(Some(p)) => p,
            Ok(None) => {
                return ProductImportVerification {
                    status: Some("failed".to_string()),
                    checked_fields: Vec::new(),
                    mismatches: vec!["回读不到刚创建的商品".to_string()],
                    message: Some("商品创建后回读失败".to_string()),
                }
            }
            Err(e) => {
                return ProductImportVerification {
                    status: Some("failed".to_string()),
                    checked_fields: Vec::new(),
                    mismatches: vec![e.to_string()],
                    message: Some("商品创建后回读失败".to_string()),
                }
            }
        };

        let mut checked = Vec::new();
        let mut mismatches = Vec::new();
        if let Some(expected) = trim_opt(d.product_name.as_deref()) {
            checked.push("productName".to_string());
            if row.product_name != expected {
                mismatches.push(format!("productName 期望「{expected}」实际「{}」", row.product_name));
            }
        }
        if let Some(expected) = trim_opt(d.product_sn.as_deref()) {
            checked.push("productSn".to_string());
            if row.product_sn != expected {
                mismatches.push(format!("productSn 期望「{expected}」实际「{}」", row.product_sn));
            }
        }
        if let Some(expected) = d.product_price {
            checked.push("productPrice".to_string());
            if row.product_price != expected {
                mismatches.push(format!(
                    "productPrice 期望「{expected}」实际「{}」",
                    row.product_price
                ));
            }
        }

        let status = if mismatches.is_empty() {
            "verified"
        } else {
            "mismatch"
        };
        ProductImportVerification {
            status: Some(status.to_string()),
            checked_fields: checked,
            mismatches: mismatches.clone(),
            message: Some(if mismatches.is_empty() {
                "落库字段与提交值一致".to_string()
            } else {
                "落库字段与提交值存在差异，请核对".to_string()
            }),
        }
    }

    // ————————————————————— 商品上下文 —————————————————————

    /// `GET /openapi/mcp/v1/products/context`
    /// （对应 Java `WorkBuddyProductImportController.productContext`）。
    pub async fn product_context(
        &self,
        key: Option<&str>,
        q: &ContextQuery,
    ) -> WbResult<ProductContextVO> {
        let status = self.require_key(key).await?;
        let shop_id = Self::scope_shop_id(&status);

        validate_context_params(
            q.keyword.as_deref(),
            q.category_id,
            q.sample_size,
        )
        .map_err(|(c, m)| WorkBuddyError::bad_request(c, m))?;

        let size = q.sample_size.unwrap_or(nextrade_core::models::openapi::workbuddy::CONTEXT_DEFAULT_SAMPLE_SIZE).max(1);
        let scope = ProductScope::parse(Some("off_sale")).unwrap_or(ProductScope::OffSale);
        // Java 用「全部非回收站商品」作为样本池。
        let scope_all = ProductScope::All;

        let query = ProductSearchQuery {
            keyword: trim_opt(q.keyword.as_deref()),
            category_id: q.category_id,
            scope: scope_all,
            shop_id: if shop_id > 0 { Some(shop_id) } else { None },
            brand_id: None,
            product_status: None,
            in_stock_only: false,
            added_start_time: None,
            added_end_time: None,
            sort_field: None,
            ascending: false,
            page: 1,
            size,
        };
        let _ = scope;
        let (total, rows) = self.r().search_products(&query).await.map_err(internal)?;

        let category_name = match q.category_id {
            Some(cid) if cid > 0 => self
                .r()
                .find_category_names(&[cid])
                .await
                .map_err(internal)?
                .remove(&cid),
            _ => None,
        };

        if rows.is_empty() {
            return Ok(ProductContextVO {
                has_data: false,
                category_id: q.category_id,
                category_name,
                sample_size: Some(0),
                metrics: None,
                patterns: Vec::new(),
                samples: Vec::new(),
                fallback: Some(ProductContextFallback::no_matching_data()),
                encoding: None,
                defaults: serde_json::Map::new(),
            });
        }

        let sales: Vec<i32> = rows.iter().map(|p| p.virtual_sales).collect();
        let stocks: Vec<i32> = rows.iter().map(|p| p.product_stock).collect();
        let prices: Vec<Decimal> = rows.iter().map(|p| p.product_price).collect();
        let mut sorted_prices = prices.clone();
        sorted_prices.sort();

        let metrics = ProductContextMetrics {
            sales_avg: nextrade_core::models::openapi::workbuddy::int_avg(&sales),
            sales_median: nextrade_core::models::openapi::workbuddy::int_median(&sales),
            stock_avg: nextrade_core::models::openapi::workbuddy::int_avg(&stocks),
            stock_median: nextrade_core::models::openapi::workbuddy::int_median(&stocks),
            price_min: sorted_prices.first().copied(),
            price_median: nextrade_core::models::openapi::workbuddy::price_median(&prices),
            price_max: sorted_prices.last().copied(),
            product_count: Some(i32::try_from(rows.len()).unwrap_or(i32::MAX)),
        };

        let samples: Vec<ProductSummaryVO> = rows
            .iter()
            .take(CONTEXT_SAMPLES_CAP)
            .map(|p| summary_from_row(p, None, None))
            .collect();

        // 编码推断：用本样本池的货号。
        let sns: Vec<String> = {
            let mut seen = HashSet::new();
            rows.iter()
                .map(|p| p.product_sn.clone())
                .filter(|s| !s.trim().is_empty() && seen.insert(s.clone()))
                .collect()
        };
        let encoding = infer_encoding(&sns);

        Ok(ProductContextVO {
            has_data: true,
            category_id: q.category_id,
            category_name,
            sample_size: Some(metrics.product_count.unwrap_or(0)),
            metrics: Some(metrics.clone()),
            patterns: Vec::new(),
            samples,
            fallback: None,
            defaults: context_defaults(&metrics),
            encoding,
        })
    }

    // ————————————————————— 商品分析 —————————————————————

    /// `GET /openapi/mcp/v1/products/analysis`
    /// （对应 Java `WorkBuddyProductImportController.analysis`）。
    pub async fn analysis(
        &self,
        key: Option<&str>,
        q: &AnalysisQuery,
    ) -> WbResult<ProductAnalysisVO> {
        let status = self.require_key(key).await?;
        let shop_id = Self::scope_shop_id(&status);

        let operation = q.operation.as_deref().map(str::trim).unwrap_or("overview").to_lowercase();
        if !ANALYSIS_OPERATIONS.contains(&operation.as_str()) {
            return Err(WorkBuddyError::bad_request(
                CODE_INVALID_OPERATION,
                &format!("不支持的分析操作: {operation}"),
            ));
        }
        let start = parse_datetime(q.start_time.as_deref());
        let end = parse_datetime(q.end_time.as_deref());
        if let (Some(s), Some(e)) = (start, end) {
            if s > e {
                return Err(WorkBuddyError::bad_request(
                    CODE_INVALID_TIME_RANGE,
                    "startTime 不能晚于 endTime",
                ));
            }
        }

        let size = q.sample_size.unwrap_or(ANALYSIS_DEFAULT_SAMPLE_SIZE).clamp(1, 1000);
        let query = ProductSearchQuery {
            keyword: trim_opt(q.keyword.as_deref()),
            category_id: q.category_id,
            scope: ProductScope::All,
            shop_id: if shop_id > 0 { Some(shop_id) } else { None },
            brand_id: None,
            product_status: None,
            in_stock_only: false,
            added_start_time: start.map(|d| d.and_utc().timestamp()),
            added_end_time: end.map(|d| d.and_utc().timestamp()),
            sort_field: None,
            ascending: false,
            page: 1,
            size,
        };
        let (_total, rows) = self.r().search_products(&query).await.map_err(internal)?;

        let sample_size = rows.len();
        let confidence = nextrade_core::models::openapi::workbuddy::analysis_confidence(sample_size);
        if rows.is_empty() {
            return Ok(ProductAnalysisVO {
                metrics: Some(json!({})),
                rankings: Vec::new(),
                recommendations: Vec::new(),
                data_scope: Some(ANALYSIS_DATA_SCOPE.to_string()),
                sample_size: Some(0),
                generated_at: Some(Utc::now().naive_utc()),
                data_start_time: start,
                data_end_time: end,
                confidence: Some(0.0),
                fallback_message: Some(ANALYSIS_FALLBACK_MESSAGE.to_string()),
            });
        }

        let counters = AnalysisCounters::from_rows(
            rows.iter().map(|p| (p.product_price, p.virtual_sales, p.product_stock)),
        );

        let (metrics, mut rankings, recommendations) =
            build_analysis(&operation, &rows, &counters);

        // Java 的排行上限 20。
        rankings.truncate(ANALYSIS_RANKING_LIMIT);

        Ok(ProductAnalysisVO {
            metrics: Some(metrics),
            rankings,
            recommendations,
            data_scope: Some(ANALYSIS_DATA_SCOPE.to_string()),
            sample_size: Some(i32::try_from(sample_size).unwrap_or(i32::MAX)),
            generated_at: Some(Utc::now().naive_utc()),
            data_start_time: start,
            data_end_time: end,
            confidence: Some(confidence),
            fallback_message: None,
        })
    }

    // ————————————————————— 媒体（素材管理） —————————————————————

    /// `GET /openapi/mcp/v1/media`
    /// （对应 Java `WorkBuddyProductIntelligenceController.media`）。
    pub async fn media(
        &self,
        key: Option<&str>,
        q: &MediaQueryDTO,
    ) -> WbResult<serde_json::Value> {
        let status = self.require_key(key).await?;
        require_platform_scope(Some(Self::scope_shop_id(&status)))
            .map_err(|(c, m)| WorkBuddyError::forbidden(c, m))?;

        let operation = nextrade_core::models::openapi::workbuddy::normalize_operation(q.operation.as_deref());
        let page = normalize_page(q.page);
        let size = match q.page_size {
            Some(s) if s >= 1 => s.min(nextrade_core::models::openapi::workbuddy::MAX_PAGE_SIZE),
            _ => nextrade_core::models::openapi::workbuddy::DEFAULT_PAGE_SIZE,
        };
        let offset = i64::from((page - 1).max(0)) * i64::from(size);

        // 分组列表。
        if operation == "groups" || operation.is_empty() {
            let groups = self
                .r()
                .find_galleries(0)
                .await
                .map_err(internal)?;
            let counts = self
                .r()
                .count_assets_by_group(0, "image")
                .await
                .map_err(internal)?;
            let nodes: Vec<MediaGroupVO> = groups
                .iter()
                .map(|g| MediaGroupVO {
                    id: Some(g.gallery_id),
                    name: Some(g.gallery_name.clone()),
                    asset_count: counts.get(&g.gallery_id).copied().unwrap_or(0),
                    r#type: Some("image".to_string()),
                    children: Vec::new(),
                })
                .collect();
            if operation == "groups" {
                return Ok(json!({
                    "status": "success",
                    "groups": nodes,
                    "total": nodes.len(),
                }));
            }
        }

        let media_type = trim_opt(q.r#type.as_deref()).unwrap_or_else(|| "image".to_string());
        if !is_supported_media_type(Some(&media_type)) {
            return Err(WorkBuddyError::bad_request(
                CODE_INVALID_TYPE,
                "仅支持 image 与 video",
            ));
        }

        if media_type == "video" {
            let (total, rows) = self
                .r()
                .search_gallery_videos(0, q.group_id, trim_opt(q.keyword.as_deref()).as_deref(), i64::from(size), offset)
                .await
                .map_err(internal)?;
            let assets: Vec<MediaAssetVO> = rows
                .iter()
                .map(|v| MediaAssetVO {
                    client_file_id: None,
                    asset_id: Some(v.id),
                    r#type: Some("video".to_string()),
                    role: Some("video".to_string()),
                    url: Some(v.video_url.clone()),
                    cover_url: v.video_cover.clone(),
                    format: v.format.clone(),
                    group_id: Some(v.gallery_id),
                })
                .collect();
            return Ok(json!({
                "status": "success",
                "total": total,
                "page": page,
                "pageSize": size,
                "assets": assets,
            }));
        }

        let (total, rows) = self
            .r()
            .search_gallery_pics(0, q.group_id, trim_opt(q.keyword.as_deref()).as_deref(), i64::from(size), offset)
            .await
            .map_err(internal)?;
        let assets: Vec<MediaAssetVO> = rows
            .iter()
            .map(|p| MediaAssetVO {
                client_file_id: None,
                asset_id: Some(p.pic_id),
                r#type: Some("image".to_string()),
                role: None,
                url: Some(p.pic_url.clone()),
                cover_url: Some(p.pic_thumb.clone()),
                format: p.file_type.clone(),
                group_id: Some(p.gallery_id),
            })
            .collect();
        Ok(json!({
            "status": "success",
            "total": total,
            "page": page,
            "pageSize": size,
            "assets": assets,
        }))
    }

    /// `POST /openapi/mcp/v1/media/prepare`
    /// （对应 Java `WorkBuddyProductIntelligenceController.mediaPrepare`）。
    pub async fn media_prepare(
        &self,
        key: Option<&str>,
        op: &MediaOperationDTO,
    ) -> WbResult<serde_json::Value> {
        let status = self.require_key(key).await?;
        let shop_id = Self::scope_shop_id(&status);
        require_platform_scope(Some(shop_id)).map_err(|(c, m)| WorkBuddyError::forbidden(c, m))?;

        if let Some(p) = op.local_path.as_deref() {
            if !p.trim().is_empty() {
                return Err(WorkBuddyError::bad_request(
                    CODE_LOCAL_PATH_NOT_ALLOWED,
                    "不接受本地文件路径，请先通过 media/upload 上传",
                ));
            }
        }

        let operation = nextrade_core::models::openapi::workbuddy::normalize_operation(op.operation.as_deref());
        if !MEDIA_SUPPORTED_OPERATIONS.contains(&operation.as_str()) {
            return Ok(media_error(CODE_INVALID_ARGUMENT));
        }

        let preview_id = new_uuid();
        let fingerprint = media_fingerprint_canonical(op);
        let impact = nextrade_core::models::openapi::workbuddy::media_impact_count(op);
        let result = json!({
            "status": "prepared",
            "previewId": preview_id,
            "operation": operation,
            "impact": impact,
            "requiresConfirmation": true,
            "message": format!("即将执行 {operation}，影响 {impact} 项素材，请确认后调用 media/execute"),
        });
        if let Ok(mut m) = self.media_previews.lock() {
            evict_expired_media(&mut m);
            m.insert(
                preview_id.clone(),
                StoredMediaPreview {
                    preview_id: preview_id.clone(),
                    fingerprint,
                    shop_id,
                    operation,
                    result: result.clone(),
                    created_at_ms: Self::now_ms(),
                },
            );
        }
        Ok(result)
    }

    /// `POST /openapi/mcp/v1/media/execute`
    /// （对应 Java `WorkBuddyProductIntelligenceController.mediaExecute`）。
    pub async fn media_execute(
        &self,
        key: Option<&str>,
        op: &MediaOperationDTO,
    ) -> WbResult<serde_json::Value> {
        let status = self.require_key(key).await?;
        let shop_id = Self::scope_shop_id(&status);
        require_platform_scope(Some(shop_id)).map_err(|(c, m)| WorkBuddyError::forbidden(c, m))?;

        if op.confirmation != Some(true) {
            return Err(WorkBuddyError::bad_request(
                CODE_CONFIRMATION_REQUIRED,
                "写入前必须明确 confirmation: true",
            ));
        }

        let operation = nextrade_core::models::openapi::workbuddy::normalize_operation(op.operation.as_deref());
        if !MEDIA_SUPPORTED_OPERATIONS.contains(&operation.as_str()) {
            return Err(WorkBuddyError::bad_request(
                CODE_INVALID_ARGUMENT,
                "不支持的媒体操作",
            ));
        }

        // 预览校验（若提供了 previewId）。
        if let Some(pid) = op.preview_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            let stored = self.media_previews.lock().ok().and_then(|mut m| {
                evict_expired_media(&mut m);
                m.get(pid).cloned()
            });
            match stored {
                None => {
                    return Err(WorkBuddyError::unprocessable(
                        CODE_PREVIEW_EXPIRED,
                        "previewId 已过期或不存在，请重新调用 media/prepare",
                    ))
                }
                Some(p) => {
                    if p.shop_id != shop_id {
                        return Err(WorkBuddyError::forbidden(
                            CODE_MEDIA_SCOPE_MISMATCH,
                            "previewId 不属于当前租户",
                        ));
                    }
                    if p.fingerprint != media_fingerprint_canonical(op) {
                        return Err(WorkBuddyError::bad_request(
                            CODE_PREVIEW_MISMATCH,
                            "previewId 与本次请求内容不一致，请重新预览",
                        ));
                    }
                }
            }
        }

        let impact = nextrade_core::models::openapi::workbuddy::media_impact_count(op);
        match operation.as_str() {
            "group_create" => {
                let name = resolve_media_name(op).ok_or_else(|| {
                    WorkBuddyError::bad_request(CODE_INVALID_ARGUMENT, "缺少分组名称")
                })?;
                let mut tx = self
                    .r()
                    .pool_clone()
                    .begin()
                    .await
                    .map_err(|e| internal(nextrade_core::Error::Database(e.to_string())))?;
                let gid = WorkbuddyRepository::insert_gallery(&mut tx, shop_id, 0, &name)
                    .await
                    .map_err(internal)?;
                tx.commit()
                    .await
                    .map_err(|e| internal(nextrade_core::Error::Database(e.to_string())))?;
                return Ok(json!({
                    "status": "success",
                    "operation": operation,
                    "groupId": gid,
                    "message": format!("分组「{name}」已创建"),
                }));
            }
            "group_update" => {
                let gid = nextrade_core::models::openapi::workbuddy::resolve_media_id(op).ok_or_else(|| {
                    WorkBuddyError::bad_request(CODE_INVALID_ARGUMENT, "缺少分组 ID")
                })?;
                let name = resolve_media_name(op).ok_or_else(|| {
                    WorkBuddyError::bad_request(CODE_INVALID_ARGUMENT, "缺少分组名称")
                })?;
                if !self.r().gallery_exists(gid, shop_id).await.map_err(internal)? {
                    return Err(WorkBuddyError::not_found(CODE_INVALID_GROUP, "分组不存在"));
                }
                self.r().update_gallery_name(gid, &name).await.map_err(internal)?;
                return Ok(json!({
                    "status": "success",
                    "operation": operation,
                    "groupId": gid,
                    "message": format!("分组已重命名为「{name}」"),
                }));
            }
            "group_delete" => {
                let gid = nextrade_core::models::openapi::workbuddy::resolve_media_id(op).ok_or_else(|| {
                    WorkBuddyError::bad_request(CODE_INVALID_ARGUMENT, "缺少分组 ID")
                })?;
                if !self.r().gallery_exists(gid, shop_id).await.map_err(internal)? {
                    return Err(WorkBuddyError::not_found(CODE_INVALID_GROUP, "分组不存在"));
                }
                let removed = self.r().delete_gallery(gid).await.map_err(internal)?;
                return Ok(json!({
                    "status": "success",
                    "operation": operation,
                    "removed": removed,
                    "message": "分组已删除",
                }));
            }
            "delete" => {
                let ids = resolve_media_ids(op);
                if ids.is_empty() {
                    return Err(WorkBuddyError::bad_request(
                        CODE_INVALID_ARGUMENT,
                        "缺少要删除的素材 ID",
                    ));
                }
                let rows = self.r().find_gallery_pics_by_ids(&ids).await.map_err(internal)?;
                let urls: Vec<String> = rows.iter().map(|p| p.pic_url.clone()).collect();
                let in_use = self.r().count_pics_in_use(&urls).await.map_err(internal)?;
                if in_use > 0 {
                    return Ok({
                        let mut v = media_error(CODE_MEDIA_IN_USE);
                        if let Some(obj) = v.as_object_mut() {
                            obj.insert("message".to_string(), json!(format!(
                                "有 {in_use} 处商品正在引用这些图片，请先替换后再删除"
                            )));
                            obj.insert("previewId".to_string(), json!(new_uuid()));
                        }
                        v
                    });
                }
                let removed = self.r().delete_gallery_pics(&ids).await.map_err(internal)?;
                return Ok(json!({
                    "status": "success",
                    "operation": operation,
                    "removed": removed,
                    "message": format!("已删除 {removed} 个素材"),
                }));
            }
            "update" => {
                let ids = resolve_media_ids(op);
                if ids.is_empty() {
                    return Err(WorkBuddyError::bad_request(
                        CODE_INVALID_ARGUMENT,
                        "缺少要更新的素材 ID",
                    ));
                }
                let name = trim_opt(op.name.as_deref());
                let mut updated = 0u64;
                for id in &ids {
                    updated += self
                        .r()
                        .update_gallery_pic(*id, op.group_id, name.as_deref(), op.sort)
                        .await
                        .map_err(internal)?;
                }
                return Ok(json!({
                    "status": "success",
                    "operation": operation,
                    "updated": updated,
                    "message": format!("已更新 {updated} 个素材"),
                }));
            }
            // upload 走 `media/upload`（multipart），此处不接受。
            _ => {}
        }

        let _ = impact;
        Err(WorkBuddyError::bad_request(
            CODE_UPLOAD_MODE_REQUIRED,
            "upload 请使用 POST /adminapi/openapi/mcp/v1/media/upload",
        ))
    }

    /// `POST /openapi/mcp/v1/media/upload`
    /// （对应 Java `WorkBuddyProductImportController.uploadMedia`）。
    ///
    /// ⚠️ Java 走 `multipart/form-data` **或** base64 JSON。
    /// Rust 侧此处接收已解析的 JSON 结构（`files[].contentBase64`），
    /// 由路由层负责 multipart 解析，见 `uncertain_points`。
    ///
    /// 只完成**校验与令牌落地**：真正的对象存储上传依赖
    /// `StorageService` 配置，本片未读到其实现，故成功路径只登记令牌：
    /// 这在 Java 里对应 `staged` 分支的「先登记、后由客户端提交」流程。
    pub async fn upload_media(
        &self,
        key: Option<&str>,
        req: &MediaUploadRequestDTO,
    ) -> WbResult<MediaUploadResultVO> {
        let status = self.require_key(key).await?;
        let shop_id = normalize_platform_shop_id(status.shop_id)
            .map_err(|(c, m)| WorkBuddyError::forbidden(c, m))?;

        // 未确认：Java 返回 status=failed + summary=CONFIRMATION_REQUIRED。
        if !req.confirmation {
            return Ok(MediaUploadResultVO {
                status: Some("failed".to_string()),
                total: i32::try_from(req.files.len()).unwrap_or(i32::MAX),
                success_count: 0,
                failed_count: i32::try_from(req.files.len()).unwrap_or(i32::MAX),
                assets: Vec::new(),
                failure_summary: BTreeMap::new(),
                summary: Some(CODE_CONFIRMATION_REQUIRED.to_string()),
                retryable: false,
            });
        }

        if req.files.is_empty() || req.files.len() > MAX_FILES {
            return Ok(MediaUploadResultVO {
                status: Some("failed".to_string()),
                total: i32::try_from(req.files.len()).unwrap_or(i32::MAX),
                success_count: 0,
                failed_count: i32::try_from(req.files.len()).unwrap_or(i32::MAX),
                assets: Vec::new(),
                failure_summary: BTreeMap::new(),
                summary: Some(CODE_INVALID_FILE_COUNT.to_string()),
                retryable: false,
            });
        }

        let mut assets = Vec::new();
        let mut failure_summary: BTreeMap<String, i32> = BTreeMap::new();
        let mut total_bytes: i64 = 0;

        for f in &req.files {
            // \"validate_upload_file\" 内部按 \"CONFIG_CODE_IMAGE_UPLOAD_MAX_SIZE\"
            // 与 \"DEFAULT_MULTIPART_MAX_FILE_SIZE\" 自行推导上限，
            // 调用方不需要再算一遍（否则会多出一个未使用的局部变量）。
            let (code, asset) = self.validate_upload_file(f).await;
            match asset {
                Some(a) => {
                    if let Some(c) = &a.format {
                        let _ = c;
                    }
                    total_bytes += f
                        .content_base64
                        .as_deref()
                        .map_or(0, |c| nextrade_core::models::openapi::workbuddy::estimated_base64_bytes(c.len()));
                    assets.push(a);
                }
                None => {
                    *failure_summary.entry(code).or_insert(0) += 1;
                }
            }
        }

        if total_bytes > nextrade_core::models::openapi::workbuddy::BATCH_CAP {
            return Ok(MediaUploadResultVO {
                status: Some("failed".to_string()),
                total: i32::try_from(req.files.len()).unwrap_or(i32::MAX),
                success_count: 0,
                failed_count: i32::try_from(req.files.len()).unwrap_or(i32::MAX),
                assets: Vec::new(),
                failure_summary: BTreeMap::new(),
                summary: Some(nextrade_core::models::openapi::workbuddy::CODE_FILE_TOO_LARGE.to_string()),
                retryable: false,
            });
        }

        let success_count = assets.len();
        let failed_count = req.files.len() - success_count;
        let all_failed = success_count == 0;
        Ok(MediaUploadResultVO {
            status: Some(if all_failed { "failed" } else { "success" }.to_string()),
            total: i32::try_from(req.files.len()).unwrap_or(i32::MAX),
            success_count: i32::try_from(success_count).unwrap_or(i32::MAX),
            failed_count: i32::try_from(failed_count).unwrap_or(i32::MAX),
            assets,
            failure_summary,
            summary: if all_failed {
                Some(CODE_SERVICE_UNAVAILABLE.to_string())
            } else {
                None
            },
            retryable: all_failed,
        })
    }

    /// 单文件校验（对应 Java `McpMediaUploadServiceImpl.validate`）。
    ///
    /// 返回 `(错误码, 可选资产)`。
    async fn validate_upload_file(
        &self,
        f: &MediaUploadFileDTO,
    ) -> (String, Option<MediaAssetVO>) {
        let name = match trim_opt(f.name.as_deref()) {
            Some(n) => n,
            None => return (CODE_INVALID_FILE.to_string(), None),
        };
        if nextrade_core::models::openapi::workbuddy::invalid_file_name(&name) {
            return (CODE_INVALID_PATH.to_string(), None);
        }
        let mime = match trim_opt(f.mime_type.as_deref()) {
            Some(m) => m.to_lowercase(),
            None => return (CODE_INVALID_FILE.to_string(), None),
        };
        if !is_supported_media_type(Some(if mime.starts_with("video/") { "video" } else { "image" }))
            || !(nextrade_core::models::openapi::workbuddy::ALLOWED_IMAGE_MIMES.contains(&mime.as_str())
                || mime == nextrade_core::models::openapi::workbuddy::ALLOWED_VIDEO_MIME)
        {
            return (CODE_UNSUPPORTED_MIME.to_string(), None);
        }
        if !nextrade_core::models::openapi::workbuddy::matches_extension(&name, &mime) {
            return (CODE_UNSUPPORTED_MIME.to_string(), None);
        }

        // fileToken 分支：消费已登记的令牌。
        if let Some(token) = trim_opt(f.file_token.as_deref()) {
            let stored = self.staged_tokens.lock().ok().and_then(|mut m| {
                evict_expired_tokens(&mut m);
                m.get(&token).cloned()
            });
            return match stored {
                None => (CODE_FILE_TOKEN_EXPIRED.to_string(), None),
                Some(t) => (
                    String::new(),
                    Some(MediaAssetVO {
                        client_file_id: f.client_file_id.clone(),
                        asset_id: None,
                        r#type: Some(if t.mime_type.starts_with("video/") {
                            "video".to_string()
                        } else {
                            "image".to_string()
                        }),
                        role: f.role.clone(),
                        url: Some(format!("/uploads/{}", t.file_name)),
                        cover_url: None,
                        format: Some(t.file_name.rsplit('.').next().unwrap_or("").to_string()),
                        group_id: f.group_id.or(req_group(f)),
                    }),
                ),
            };
        }

        let content = match trim_opt(f.content_base64.as_deref()) {
            Some(c) => c,
            // Java 在没有 fileToken 也没有 base64 时要求先走上传令牌流程。
            None => return (CODE_UPLOAD_MODE_REQUIRED.to_string(), None),
        };
        let bytes = match decode_base64_prefix(&content) {
            Some(b) => b,
            None => return (CODE_INVALID_BASE64.to_string(), None),
        };
        if !nextrade_core::models::openapi::workbuddy::signature_valid(&bytes, &mime) {
            return (CODE_UNSUPPORTED_MIME.to_string(), None);
        }
        let limit = if mime == nextrade_core::models::openapi::workbuddy::ALLOWED_VIDEO_MIME {
            parse_size_bytes(Some(DEFAULT_MULTIPART_MAX_FILE_SIZE))
        } else {
            image_max_bytes(
                self.config_value(CONFIG_CODE_IMAGE_UPLOAD_MAX_SIZE).await.as_deref(),
                parse_size_bytes(Some(DEFAULT_MULTIPART_MAX_FILE_SIZE)),
            )
        };
        if nextrade_core::models::openapi::workbuddy::estimated_base64_bytes(content.len()) > limit {
            return (nextrade_core::models::openapi::workbuddy::CODE_FILE_TOO_LARGE.to_string(), None);
        }

        (
            String::new(),
            Some(MediaAssetVO {
                client_file_id: f.client_file_id.clone(),
                asset_id: None,
                r#type: Some(if mime == nextrade_core::models::openapi::workbuddy::ALLOWED_VIDEO_MIME {
                    "video".to_string()
                } else {
                    "image".to_string()
                }),
                role: f.role.clone(),
                // 未接入对象存储：仅回出文件名占位（见 uncertain_points）。
                url: Some(format!("/uploads/{name}")),
                cover_url: None,
                format: Some(nextrade_core::models::openapi::workbuddy::file_extension(&name)),
                group_id: f.group_id.or_else(|| req_group(f)),
            }),
        )
    }

    /// `POST /openapi/mcp/v1/media/upload-token`
    /// （对应 Java `WorkBuddyProductImportController.uploadToken`）。
    ///
    /// 登记一个 30 分钟有效的文件令牌，客户端随后用
    /// `POST /openapi/mcp/v1/media/upload` 携带 `fileToken` 提交。
    pub async fn upload_token(
        &self,
        key: Option<&str>,
        body: &UploadTokenBody,
    ) -> WbResult<serde_json::Value> {
        let status = self.require_key(key).await?;
        let shop_id = normalize_platform_shop_id(status.shop_id)
            .map_err(|(c, m)| WorkBuddyError::forbidden(c, m))?;

        let name = trim_opt(body.name.as_deref())
            .ok_or_else(|| WorkBuddyError::bad_request(CODE_INVALID_FILE, "缺少文件名"))?;
        if nextrade_core::models::openapi::workbuddy::invalid_file_name(&name) {
            return Err(WorkBuddyError::bad_request(CODE_INVALID_PATH, "文件名不合法"));
        }
        let mime = trim_opt(body.mime_type.as_deref())
            .ok_or_else(|| WorkBuddyError::bad_request(CODE_INVALID_FILE, "缺少 mimeType"))?
            .to_lowercase();
        if !(nextrade_core::models::openapi::workbuddy::ALLOWED_IMAGE_MIMES.contains(&mime.as_str())
            || mime == nextrade_core::models::openapi::workbuddy::ALLOWED_VIDEO_MIME)
        {
            return Err(WorkBuddyError::bad_request(
                CODE_UNSUPPORTED_MIME,
                "不支持的 MIME 类型",
            ));
        }
        if !nextrade_core::models::openapi::workbuddy::matches_extension(&name, &mime) {
            return Err(WorkBuddyError::bad_request(
                CODE_UNSUPPORTED_MIME,
                "文件扩展名与 MIME 类型不匹配",
            ));
        }

        let size = body.size.unwrap_or(0);
        if size > 0 {
            let multipart = parse_size_bytes(Some(DEFAULT_MULTIPART_MAX_FILE_SIZE));
            let limit = if mime == nextrade_core::models::openapi::workbuddy::ALLOWED_VIDEO_MIME {
                multipart
            } else {
                image_max_bytes(
                    self.config_value(CONFIG_CODE_IMAGE_UPLOAD_MAX_SIZE).await.as_deref(),
                    multipart,
                )
            };
            if size > limit {
                return Err(WorkBuddyError::bad_request(
                    nextrade_core::models::openapi::workbuddy::CODE_FILE_TOO_LARGE,
                    &format!("文件超过 {limit} 字节上限"),
                ));
            }
        }

        let file_token = format!("ft_{}", new_uuid().replace('-', ""));
        if let Ok(mut m) = self.staged_tokens.lock() {
            evict_expired_tokens(&mut m);
            m.insert(
                file_token.clone(),
                StoredUploadToken {
                    file_token: file_token.clone(),
                    shop_id,
                    file_name: name.clone(),
                    mime_type: mime.clone(),
                    size,
                    created_at_ms: Self::now_ms(),
                },
            );
        }

        let upload_url = "/adminapi/openapi/mcp/v1/media/upload";
        Ok(json!({
            "status": "success",
            "fileToken": file_token,
            "uploadUrl": upload_url,
            "url": upload_url,
            "expiresIn": TOKEN_TTL_MILLIS / 1000,
            "expiresAt": Self::now_ms() + TOKEN_TTL_MILLIS,
            "fileName": name,
            "mimeType": mime,
            "size": size,
            "maxFileBytes": parse_size_bytes(Some(DEFAULT_MULTIPART_MAX_FILE_SIZE)),
            "message": "请把文件以 multipart 形式 POST 到 uploadUrl，文件名与 MIME 需与本次申请一致",
        }))
    }

    // ————————————————————— 分类导入 —————————————————————

    /// `POST /openapi/mcp/v1/categories/prepare`
    /// （对应 Java `WorkBuddyProductIntelligenceController.categoryPrepare`）。
    pub async fn category_prepare(
        &self,
        key: Option<&str>,
        dto: &CategoryImportDTO,
    ) -> WbResult<serde_json::Value> {
        let status = self.require_key(key).await?;
        let shop_id = Self::scope_shop_id(&status);
        require_category_platform_scope(Some(shop_id))
            .map_err(|(c, m)| WorkBuddyError::forbidden(c, m))?;

        let operation = dto.operation.as_deref().map(str::trim).unwrap_or("");
        if operation != "import" {
            return Err(WorkBuddyError::bad_request(
                CODE_INVALID_OPERATION,
                "operation 必须为 import",
            ));
        }
        if dto.nodes.is_empty() {
            return Err(WorkBuddyError::bad_request(
                CODE_INVALID_ARGUMENT,
                "nodes 不能为空",
            ));
        }
        let request_id = trim_opt(dto.request_id.as_deref()).ok_or_else(|| {
            WorkBuddyError::bad_request(CODE_REQUEST_ID_REQUIRED, "prepare 必须提供 requestId")
        })?;

        // 展平 + 结构校验（空名/超长/重复路径）。
        let (flat, mut nodes) = flatten_category_nodes(&dto.nodes);

        // 与库中已有分类比对。
        let existing = self
            .r()
            .find_all_categories(CATEGORY_IMPORT_PAGE_SIZE)
            .await
            .map_err(internal)?;
        // 路径 ⇒ 已存在分类 ID
        let mut existing_by_path: BTreeMap<String, i32> = BTreeMap::new();
        {
            let by_id: BTreeMap<i32, (String, i32)> = existing
                .iter()
                .map(|c| (c.category_id, (c.category_name.clone(), c.parent_id)))
                .collect();
            for c in &existing {
                existing_by_path
                    .insert(build_category_path(&by_id, c.category_id), c.category_id);
            }
        }

        let mut importable: Vec<(String, String, String, Option<i32>)> = Vec::new();
        let mut existing_count = 0;
        let mut missing_parent_count = 0;
        let mut conflict_count = 0;

        for f in &flat {
            // 找到对应的 NodeResult（按 path 定位）。
            let idx = nodes.iter().position(|n| n.path.as_deref() == Some(f.path.as_str()));
            let mut state = CategoryNodeState::Missing;
            let mut resolved_id: Option<i32> = None;

            if let Some(explicit) = f.explicit_id {
                // 显式 ID：必须真实存在。
                match self.r().find_category(explicit).await.map_err(internal)? {
                    Some(c) => {
                        state = CategoryNodeState::Existing;
                        resolved_id = Some(c.category_id);
                    }
                    None => {
                        state = CategoryNodeState::Conflict;
                    }
                }
            } else if let Some(hit) = existing_by_path.get(&f.path) {
                state = CategoryNodeState::Existing;
                resolved_id = Some(*hit);
            } else if !f.parent_path.is_empty()
                && !existing_by_path.contains_key(&f.parent_path)
                && !flat.iter().any(|x| x.path == f.parent_path)
            {
                state = CategoryNodeState::MissingParent;
            }

            if let Some(i) = idx {
                let conflicting = nodes[i].state.as_deref() == Some(CategoryNodeState::Conflict.as_str());
                if conflicting {
                    state = CategoryNodeState::Conflict;
                }
                nodes[i].state = Some(state.as_str().to_string());
                nodes[i].id = resolved_id.or(nodes[i].id);
                if nodes[i].reason.is_none() {
                    nodes[i].reason = match state {
                        CategoryNodeState::Existing => Some("分类已存在，将直接复用".to_string()),
                        CategoryNodeState::Missing => Some("分类不存在，将新建".to_string()),
                        CategoryNodeState::MissingParent => {
                            Some("父级分类未创建，不能创建子级".to_string())
                        }
                        CategoryNodeState::Conflict => {
                            Some(self.conflict_reason(f, &existing_by_path, &flat).await)
                        }
                    };
                }
            }

            match state {
                CategoryNodeState::Existing => existing_count += 1,
                CategoryNodeState::MissingParent => missing_parent_count += 1,
                CategoryNodeState::Conflict => conflict_count += 1,
                CategoryNodeState::Missing => {}
            }
            if matches!(state, CategoryNodeState::Missing | CategoryNodeState::Existing) {
                importable.push((
                    f.path.clone(),
                    f.parent_path.clone(),
                    f.name.clone(),
                    resolved_id,
                ));
            }
        }

        let total = i32::try_from(nodes.len()).unwrap_or(i32::MAX);
        let importable_flag = conflict_count == 0 && missing_parent_count == 0;
        let preview = CategoryImportPreviewVO {
            preview_id: Some(new_uuid()),
            status: Some(if importable_flag { "PREPARED" } else { "REJECTED" }.to_string()),
            summary: None,
            nodes: nodes.clone(),
            total,
            existing_count,
            missing_parent_count,
            conflict_count,
        };
        let summary = category_preview_summary(&preview);

        let preview_id = preview.preview_id.clone().unwrap_or_default();
        if let Ok(mut m) = self.category_previews.lock() {
            evict_expired_category_previews(&mut m);
            m.insert(
                preview_id.clone(),
                StoredCategoryPreview {
                    preview_id: preview_id.clone(),
                    request_id: request_id.clone(),
                    shop_id,
                    nodes,
                    importable,
                    importable_flag,
                    created_at_ms: Self::now_ms(),
                },
            );
        }

        let mut preview_json = serde_json::to_value(&preview).unwrap_or_default();
        if let Some(obj) = preview_json.as_object_mut() {
            obj.insert("summary".to_string(), json!(summary));
            obj.insert(
                "importable".to_string(),
                json!(importable_flag),
            );
        }

        Ok(json!({
            "preview": preview_json,
            "previewId": preview_id,
            "requestId": request_id,
            "status": if importable_flag { "prepared" } else { "rejected" },
            "errorCode": if importable_flag { serde_json::Value::Null } else { json!(CODE_PREVIEW_NOT_IMPORTABLE) },
        }))
    }

    /// 冲突原因（Java `CategoryImportServiceImpl.conflictReason`）。
    async fn conflict_reason(
        &self,
        f: &nextrade_core::models::openapi::workbuddy::FlatCategoryNode,
        existing_by_path: &BTreeMap<String, i32>,
        flat: &[nextrade_core::models::openapi::workbuddy::FlatCategoryNode],
    ) -> String {
        if f.name.trim().is_empty() {
            return "分类名称不能为空".to_string();
        }
        if f.name.chars().count() > nextrade_core::models::openapi::workbuddy::CATEGORY_NAME_MAX_LEN {
            return "分类名称不能超过 30 个字符".to_string();
        }
        if let Some(explicit) = f.explicit_id {
            if self.r().find_category(explicit).await.ok().flatten().is_none() {
                return format!("指定的分类 ID {explicit} 不存在");
            }
        }
        if !f.parent_path.is_empty()
            && !existing_by_path.contains_key(&f.parent_path)
            && !flat.iter().any(|x| x.path == f.parent_path)
        {
            return "父级分类未创建，不能创建子级".to_string();
        }
        "输入中存在重复分类路径".to_string()
    }

    /// `POST /openapi/mcp/v1/categories/execute`
    /// （对应 Java `WorkBuddyProductIntelligenceController.categoryExecute`）。
    pub async fn category_execute(
        &self,
        key: Option<&str>,
        dto: &CategoryImportDTO,
    ) -> WbResult<serde_json::Value> {
        let status = self.require_key(key).await?;
        let shop_id = Self::scope_shop_id(&status);
        require_category_platform_scope(Some(shop_id))
            .map_err(|(c, m)| WorkBuddyError::forbidden(c, m))?;

        if dto.confirmation != Some(true) {
            return Err(WorkBuddyError::bad_request(
                CODE_CONFIRMATION_REQUIRED,
                "写入前必须明确 confirmation: true",
            ));
        }
        let preview_id = trim_opt(dto.preview_id.as_deref()).ok_or_else(|| {
            WorkBuddyError::bad_request(CODE_PREVIEW_REQUIRED, "缺少 previewId")
        })?;

        // 结果缓存（24h）：同一 previewId 重复提交直接返回首次结果。
        if let Ok(mut m) = self.category_results.lock() {
            evict_expired_category_results(&mut m);
            if let Some(r) = m.get(&preview_id) {
                return Ok(r.result.clone());
            }
        }

        let stored = self
            .category_previews
            .lock()
            .ok()
            .and_then(|mut m| {
                evict_expired_category_previews(&mut m);
                m.get(&preview_id).cloned()
            })
            .ok_or_else(|| {
                WorkBuddyError::unprocessable(
                    CODE_PREVIEW_EXPIRED,
                    "previewId 已过期或不存在，请重新 prepare",
                )
            })?;

        if stored.shop_id != shop_id {
            return Err(WorkBuddyError::forbidden(
                CODE_PREVIEW_SCOPE_MISMATCH,
                "previewId 不属于当前租户",
            ));
        }
        if let Some(rid) = trim_opt(dto.request_id.as_deref()) {
            if rid != stored.request_id {
                return Err(WorkBuddyError::bad_request(
                    CODE_PREVIEW_REQUEST_MISMATCH,
                    "requestId 与 previewId 不匹配",
                ));
            }
        }
        if !stored.importable_flag {
            let result = json!({
                "previewId": preview_id,
                "requestId": stored.request_id,
                "status": "failed",
                "errorCode": CODE_PREVIEW_NOT_IMPORTABLE,
                "items": [],
            });
            self.remember_category_result(&preview_id, &result);
            return Ok(result);
        }

        // 按展平顺序创建（父先于子）。
        let mut path_to_id: BTreeMap<String, i32> = BTreeMap::new();
        let mut items = Vec::new();
        let mut sorted = stored.importable.clone();
        sorted.sort_by_key(|(path, _, _, _)| path.matches('/').count());

        for (path, parent_path, name, explicit_id) in &sorted {
            if let Some(id) = explicit_id {
                path_to_id.insert(path.clone(), *id);
                items.push(category_item_map(path, true, Some(*id), None, None, false));
                continue;
            }
            let parent_id = if parent_path.is_empty() {
                0
            } else {
                match path_to_id.get(parent_path) {
                    Some(p) => *p,
                    None => {
                        items.push(category_item_map(
                            path,
                            false,
                            None,
                            Some(CODE_MISSING_PARENT),
                            Some("父级分类未创建，不能创建子级"),
                            false,
                        ));
                        continue;
                    }
                }
            };
            let mut tx = match self.r().pool_clone().begin().await {
                Ok(t) => t,
                Err(e) => {
                    items.push(category_item_map(
                        path,
                        false,
                        None,
                        Some(CODE_CATEGORY_CREATE_FAILED),
                        Some(&safe_message(Some(&e.to_string()))),
                        true,
                    ));
                    continue;
                }
            };
            match WorkbuddyRepository::insert_category(&mut tx, name, parent_id, 0).await {
                Ok(id) => {
                    if tx.commit().await.is_err() {
                        items.push(category_item_map(
                            path,
                            false,
                            None,
                            Some(CODE_CATEGORY_CREATE_FAILED),
                            Some("分类创建失败"),
                            true,
                        ));
                        continue;
                    }
                    path_to_id.insert(path.clone(), id);
                    items.push(category_item_map(path, true, Some(id), None, None, false));
                }
                Err(e) => {
                    // `nextrade_core::Error` 是枚举（无 `.message` 字段），
                    // 取可读文案要走 \"Display\"（thiserror 生成的 message）。
                    let msg = e.to_string();
                    items.push(category_item_map(
                        path,
                        false,
                        None,
                        Some(CODE_CATEGORY_CREATE_FAILED),
                        Some(&safe_message(Some(&msg))),
                        false,
                    ));
                }
            }
        }

        let failed = items
            .iter()
            .filter(|i| i.get("success").and_then(serde_json::Value::as_bool) != Some(true))
            .count();
        let result = json!({
            "previewId": preview_id,
            "requestId": stored.request_id,
            "status": if failed == 0 { "success" } else { "partial" },
            "items": items,
            "successCount": items.len() - failed,
            "failureCount": failed,
        });
        self.remember_category_result(&preview_id, &result);
        Ok(result)
    }

    fn remember_category_result(&self, preview_id: &str, result: &serde_json::Value) {
        if let Ok(mut m) = self.category_results.lock() {
            evict_expired_category_results(&mut m);
            m.insert(
                preview_id.to_string(),
                StoredCategoryResult {
                    result: result.clone(),
                    created_at_ms: Self::now_ms(),
                },
            );
        }
    }
}

// ——————————————————————————————————————————————
// 请求体 / 查询参数（逐字符对齐 Java 的 DTO）
// ——————————————————————————————————————————————

/// \"category\" 查询参数。
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CategoryQuery {
    pub keyword: Option<String>,
    pub parent_id: Option<i32>,
    pub size: Option<i32>,
}

/// 商品列表查询参数（对应 Java `ProductSearchDTO`）。
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProductListQuery {
    pub keyword: Option<String>,
    pub category_id: Option<i32>,
    pub brand_id: Option<i32>,
    pub scope: Option<String>,
    pub sort_by: Option<String>,
    /// `asc` / \"desc\"。
    pub order: Option<String>,
    pub page: Option<i32>,
    pub size: Option<i32>,
    pub in_stock_only: Option<bool>,
    pub mine_only: Option<bool>,
}

/// 上下文查询参数（对应 Java `ProductContextDTO`）。
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ContextQuery {
    pub keyword: Option<String>,
    pub category_id: Option<i32>,
    pub sample_size: Option<i32>,
}

/// 分析查询参数（对应 Java `ProductAnalysisDTO`）。
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AnalysisQuery {
    pub operation: Option<String>,
    pub keyword: Option<String>,
    pub category_id: Option<i32>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub sample_size: Option<i32>,
}

/// 单字段写操作体（对应 Java `FieldBody`）。
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FieldBody {
    pub field: String,
    pub value: Option<String>,
}

/// 上下架操作体（对应 Java `StatusBody`）。
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StatusBody {
    pub on_sale: Option<bool>,
}

/// 批量操作体（对应 Java `BatchBody`）。
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BatchBody {
    #[serde(default)]
    pub product_ids: Option<Vec<Option<i32>>>,
    pub operation: Option<String>,
}

/// 批量执行体（对应 Java `BatchExecuteBody extends BatchBody`）。
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BatchExecuteBody {
    #[serde(default)]
    pub product_ids: Option<Vec<Option<i32>>>,
    pub operation: Option<String>,
    pub preview_id: Option<String>,
    pub confirmation: Option<bool>,
}

/// 商品列表载体（对应 Java `ProductList`）。
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProductListBody {
    #[serde(default)]
    pub products: Vec<ProductDraftDTO>,
}

/// 导入载体（对应 Java `ImportBody extends ProductList`）。
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ImportBody {
    pub preview_id: Option<String>,
    pub confirmation: Option<bool>,
    #[serde(default)]
    pub products: Vec<ProductDraftDTO>,
}

/// 上传令牌申请体。
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UploadTokenBody {
    pub name: Option<String>,
    pub mime_type: Option<String>,
    pub size: Option<i64>,
}

// ——————————————————————————————————————————————
// 纯辅助
// ——————————————————————————————————————————————

/// 取分组（\"MediaUploadFileDTO\" 没有独立的分组 helper，这里统一兜底）。
fn req_group(f: &MediaUploadFileDTO) -> Option<i32> {
    f.group_id
}

/// 生成 UUID（对应 Java `UUID.randomUUID()`）。
fn new_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// 解析 ISO-8601 / `yyyy-MM-dd HH:mm:ss` 时间串（对应 Java 的入参绑定）。
fn parse_datetime(raw: Option<&str>) -> Option<NaiveDateTime> {
    let s = raw?.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.naive_utc());
    }
    for fmt in ["%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%d"] {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(dt);
        }
        if fmt == "%Y-%m-%d" {
            if let Ok(d) = chrono::NaiveDate::parse_from_str(s, fmt) {
                return d.and_hms_opt(0, 0, 0);
            }
        }
    }
    None
}

/// 分类路径（从顶级到本级，`/` 分隔）。
fn build_category_path(by_id: &BTreeMap<i32, (String, i32)>, id: i32) -> String {
    let mut parts = Vec::new();
    let mut cur = id;
    let mut guard = 0;
    while let Some((name, parent)) = by_id.get(&cur) {
        parts.push(name.clone());
        if *parent == 0 || guard > 64 {
            break;
        }
        cur = *parent;
        guard += 1;
    }
    parts.reverse();
    parts.join("/")
}

/// 建立「完整路径 ⇒ 分类 ID」索引。
fn build_path_index(
    categories: &[nextrade_infrastructure::db::repo::openapi::workbuddy::CategoryRow],
) -> BTreeMap<String, i32> {
    let by_id: BTreeMap<i32, (String, i32)> = categories
        .iter()
        .map(|c| (c.category_id, (c.category_name.clone(), c.parent_id)))
        .collect();
    let mut out = BTreeMap::new();
    for c in categories {
        out.insert(build_category_path(&by_id, c.category_id), c.category_id);
    }
    out
}

/// 归一化草稿（Java `ProductImportServiceImpl.normalize(draft)`）。
fn normalize_draft(
    raw: &ProductDraftDTO,
    resolved_category: Option<i32>,
    images: &[String],
    rate: Decimal,
) -> ProductDraftDTO {
    let price = raw.product_price;
    let market_price =
        nextrade_core::models::openapi::workbuddy::resolve_market_price(raw.market_price, price, rate);
    ProductDraftDTO {
        product_name: trim_opt(raw.product_name.as_deref()),
        product_sn: trim_opt(raw.product_sn.as_deref()),
        product_price: price.map(round2),
        category_id: resolved_category.or(raw.category_id),
        product_stock: raw.product_stock,
        product_status: Some(raw.product_status.unwrap_or(PRODUCT_STATUS_ON_SALE)),
        product_brief: trim_opt(raw.product_brief.as_deref()),
        product_desc: trim_opt(raw.product_desc.as_deref()),
        images: images.to_vec(),
        detail_images: dedupe_urls(&nextrade_core::models::openapi::workbuddy::media_detail_image_urls(
            &raw.media,
        )),
        media: raw.media.clone(),
        market_price,
        cost_price: raw.cost_price,
        product_tsn: trim_opt(raw.product_tsn.as_deref()),
        keywords: trim_opt(raw.keywords.as_deref()),
        skus: raw.skus.clone(),
        field_sources: raw.field_sources.clone(),
        category_name: raw.category_name.clone(),
    }
}

/// 行 ⇒ 列表 VO（\"skuCount\" / \"categoryName\" 由批量查询补齐）。
fn to_summary(
    p: &ProductRow,
    sku_counts: &BTreeMap<i32, i32>,
    cat_names: &BTreeMap<i32, String>,
) -> ProductSummaryVO {
    ProductSummaryVO {
        product_id: Some(p.product_id),
        product_name: Some(p.product_name.clone()),
        product_sn: Some(p.product_sn.clone()),
        price: Some(p.product_price),
        stock: Some(p.product_stock),
        status: status_label(Some(p.product_status)),
        audit_status: nextrade_core::models::openapi::workbuddy::audit_label(Some(p.check_status)),
        product_type: product_type_label(p.product_type),
        category_name: cat_names.get(&p.category_id).cloned(),
        sku_count: sku_counts.get(&p.product_id).copied(),
        sales: Some(p.virtual_sales),
        recycled: p.is_delete == 1,
    }
}

/// 行 ⇒ 列表 VO（无批量查询结果时）。
fn summary_from_row(
    p: &ProductRow,
    sku_count: Option<i32>,
    category_name: Option<String>,
) -> ProductSummaryVO {
    ProductSummaryVO {
        product_id: Some(p.product_id),
        product_name: Some(p.product_name.clone()),
        product_sn: Some(p.product_sn.clone()),
        price: Some(p.product_price),
        stock: Some(p.product_stock),
        status: status_label(Some(p.product_status)),
        audit_status: nextrade_core::models::openapi::workbuddy::audit_label(Some(p.check_status)),
        product_type: product_type_label(p.product_type),
        category_name,
        sku_count,
        sales: Some(p.virtual_sales),
        recycled: p.is_delete == 1,
    }
}

/// 批量行 ⇒ 列表 VO。
fn summary_from_batch(
    p: &nextrade_infrastructure::db::repo::openapi::workbuddy::BatchProductRow,
) -> ProductSummaryVO {
    ProductSummaryVO {
        product_id: Some(p.product_id),
        product_name: Some(p.product_name.clone()),
        product_sn: Some(p.product_sn.clone()),
        price: Some(p.product_price),
        stock: Some(p.product_stock),
        status: status_label(Some(p.product_status)),
        audit_status: nextrade_core::models::openapi::workbuddy::audit_label(Some(p.check_status)),
        product_type: product_type_label(p.product_type),
        category_name: None,
        sku_count: None,
        sales: Some(p.virtual_sales),
        recycled: p.is_delete == 1,
    }
}

/// Base64 解码（只取前若干字节用于魔数校验）。
fn decode_base64_prefix(content: &str) -> Option<Vec<u8>> {
    let cleaned: String = content
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '=')
        .collect();
    let table: Vec<u8> = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".to_vec();
    let mut out = Vec::new();
    let mut buf: u32 = 0;
    let mut bits = 0u32;
    for ch in cleaned.bytes() {
        let idx = table.iter().position(|t| *t == ch)?;
        buf = (buf << 6) | u32::try_from(idx).ok()?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from((buf >> bits) & 0xFF).ok()?);
            if out.len() >= 32 {
                break;
            }
        }
    }
    Some(out)
}

/// 编码推断（Java `ProductContextServiceImpl.inferEncoding`）。
fn infer_encoding(sns: &[String]) -> Option<nextrade_core::models::openapi::workbuddy::EncodingInference> {
    use nextrade_core::models::openapi::workbuddy::EncodingInference;

    if sns.len() < 2 {
        return Some(EncodingInference {
            pattern: None,
            prefix: None,
            separator: None,
            number_length: None,
            next_candidate: None,
            confidence: Some(nextrade_core::models::openapi::workbuddy::ENCODING_CONFIDENCE_TOO_FEW),
            needs_confirmation: true,
            reason: Some(MSG_ENCODING_TOO_FEW.to_string()),
            samples: sns.to_vec(),
        });
    }

    match nextrade_core::models::openapi::workbuddy::encoding_decision(sns) {
        EncodingDecision::NoGroups => Some(EncodingInference {
            pattern: None,
            prefix: None,
            separator: None,
            number_length: None,
            next_candidate: None,
            confidence: Some(nextrade_core::models::openapi::workbuddy::ENCODING_CONFIDENCE_NO_GROUPS),
            needs_confirmation: true,
            reason: Some(MSG_ENCODING_INCONSISTENT.to_string()),
            samples: sns.to_vec(),
        }),
        EncodingDecision::Grouped {
            prefix,
            number_length,
            dominant,
            second,
            confidence,
            decisive,
        } => {
            let _ = (dominant, second);
            if !decisive {
                return Some(EncodingInference {
                    pattern: Some(format!("{prefix} + {number_length} 位数字")),
                    prefix: Some(prefix),
                    separator: None,
                    number_length: i32::try_from(number_length).ok(),
                    next_candidate: None,
                    confidence: Some(nextrade_core::models::openapi::workbuddy::ENCODING_CONFIDENCE_UNDECISIVE),
                    needs_confirmation: true,
                    reason: Some(MSG_ENCODING_INCONSISTENT.to_string()),
                    samples: sns.to_vec(),
                });
            }
            // 找该分组下的最大序号。
            let mut max_number: Option<u64> = None;
            for sn in sns {
                if let Some((p, len)) = nextrade_core::models::openapi::workbuddy::encoding_group_key(sn) {
                    if p == prefix && len == number_length {
                        let digits = &sn[sn.len() - number_length..];
                        if let Ok(n) = digits.parse::<u64>() {
                            max_number = Some(max_number.map_or(n, |m: u64| m.max(n)));
                        }
                    }
                }
            }
            let next = max_number.map(|n| n + 1);
            let Some(next) = next else {
                return Some(EncodingInference {
                    pattern: None,
                    prefix: Some(prefix),
                    separator: None,
                    number_length: i32::try_from(number_length).ok(),
                    next_candidate: None,
                    confidence: Some(nextrade_core::models::openapi::workbuddy::ENCODING_CONFIDENCE_NO_GROUPS),
                    needs_confirmation: true,
                    reason: Some(MSG_ENCODING_TOO_FEW.to_string()),
                    samples: sns.to_vec(),
                });
            };
            if nextrade_core::models::openapi::workbuddy::encoding_overflowed(next, number_length) {
                return Some(EncodingInference {
                    pattern: Some(format!("{prefix} + {number_length} 位数字")),
                    prefix: Some(prefix),
                    separator: None,
                    number_length: i32::try_from(number_length).ok(),
                    next_candidate: None,
                    confidence: Some(nextrade_core::models::openapi::workbuddy::ENCODING_CONFIDENCE_OVERFLOW),
                    needs_confirmation: true,
                    reason: Some(MSG_ENCODING_OVERFLOW.to_string()),
                    samples: sns.to_vec(),
                });
            }
            let capped = confidence.min(nextrade_core::models::openapi::workbuddy::ENCODING_CONFIDENCE_CAP);
            let exact = confidence >= 0.999;
            Some(EncodingInference {
                pattern: Some(format!("{prefix} + {number_length} 位数字")),
                prefix: Some(prefix.clone()),
                separator: None,
                number_length: i32::try_from(number_length).ok(),
                next_candidate: Some(format_candidate(&prefix, number_length, next)),
                confidence: Some(capped),
                needs_confirmation: true,
                reason: Some(if exact {
                    MSG_ENCODING_DOMINANT_EXACT.to_string()
                } else {
                    MSG_ENCODING_DOMINANT_MAJORITY.to_string()
                }),
                samples: sns.iter().take(MIN_DOMINANT_ENCODING_SAMPLES.max(3)).cloned().collect(),
            })
        }
    }
}

/// 构造分析结果（\"metrics\" / \"rankings\" / \"recommendations\"）。
fn build_analysis(
    operation: &str,
    rows: &[ProductRow],
    counters: &AnalysisCounters,
) -> (serde_json::Value, Vec<serde_json::Value>, Vec<serde_json::Value>) {
    let price_avg = counters
        .price_sum
        .checked_div(Decimal::from(rows.len().max(1)))
        .map_or(Decimal::ZERO, round2);
    let total_sales: i64 = rows.iter().map(|p| i64::from(p.virtual_sales)).sum();
    let total_stock: i64 = rows.iter().map(|p| i64::from(p.product_stock)).sum();

    let metrics = json!({
        "productCount": rows.len(),
        "totalStock": total_stock,
        "totalSales": total_sales,
        "priceAvg": normalize_number(price_avg).to_string(),
        "priceMin": counters.price_min.map(|d| normalize_number(d).to_string()),
        "priceMax": counters.price_max.map(|d| normalize_number(d).to_string()),
        "stockAvg": if rows.is_empty() { 0.0 } else { total_stock as f64 / rows.len() as f64 },
        "salesAvg": if rows.is_empty() { 0.0 } else { total_sales as f64 / rows.len() as f64 },
    });

    let mut rankings: Vec<serde_json::Value> = Vec::new();
    let mut recommendations: Vec<serde_json::Value> = Vec::new();

    // 通用排行：始终给出销量前 20（供 top_sellers / recommend 复用）。
    let mut by_sales: Vec<&ProductRow> = rows.iter().collect();
    by_sales.sort_by(|a, b| b.virtual_sales.cmp(&a.virtual_sales));

    match operation {
        "overview" => {
            for p in by_sales.iter().take(ANALYSIS_RANKING_LIMIT) {
                rankings.push(json!({
                    "productId": p.product_id,
                    "productName": p.product_name,
                    "sales": p.virtual_sales,
                    "stock": p.product_stock,
                    "price": normalize_number(p.product_price).to_string(),
                }));
            }
        }
        "top_sellers" => {
            for p in by_sales.iter().take(ANALYSIS_RANKING_LIMIT) {
                rankings.push(json!({
                    "productId": p.product_id,
                    "productName": p.product_name,
                    "sales": p.virtual_sales,
                }));
            }
            if let Some(top) = by_sales.first() {
                recommendations.push(json!({
                    "type": "replenish",
                    "productId": top.product_id,
                    "productName": top.product_name,
                    "suggestion": MSG_RECOMMEND_REPLENISH,
                    "reason": format!("销量 {} 为样本最高", top.virtual_sales),
                }));
            }
        }
        "slow_movers" => {
            let mut by_stock: Vec<&ProductRow> = rows.iter().collect();
            by_stock.sort_by(|a, b| b.product_stock.cmp(&a.product_stock));
            for p in by_stock
                .iter()
                .filter(|p| p.virtual_sales <= 0 || p.product_stock >= 30 * p.virtual_sales.max(1))
                .take(ANALYSIS_RANKING_LIMIT)
            {
                rankings.push(json!({
                    "productId": p.product_id,
                    "productName": p.product_name,
                    "stock": p.product_stock,
                    "sales": p.virtual_sales,
                    "stockToSalesRatio": stock_to_sales_ratio(p.product_stock, p.virtual_sales).to_string(),
                }));
            }
            if let Some(worst) = rankings.first() {
                recommendations.push(json!({
                    "type": "clearance",
                    "productId": worst.get("productId").cloned().unwrap_or(serde_json::Value::Null),
                    "productName": worst.get("productName").cloned().unwrap_or(serde_json::Value::Null),
                    "suggestion": MSG_RECOMMEND_CLEARANCE,
                    "reason": "库存相对销量偏高",
                }));
            }
        }
        "inventory_risk" => {
            let mut scored: Vec<(i32, &ProductRow, Vec<&str>)> = rows
                .iter()
                .map(|p| {
                    let (score, reasons) =
                        nextrade_core::models::openapi::workbuddy::inventory_risk(p.product_stock, p.virtual_sales);
                    (score, p, reasons)
                })
                .collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0));
            for (score, p, reasons) in scored.iter().take(ANALYSIS_RANKING_LIMIT) {
                rankings.push(json!({
                    "productId": p.product_id,
                    "productName": p.product_name,
                    "stock": p.product_stock,
                    "sales": p.virtual_sales,
                    "riskScore": score,
                    "riskLevel": risk_level(*score),
                    "reasons": reasons,
                }));
            }
            if let Some((score, p, _)) = scored.first() {
                if *score >= 2 {
                    recommendations.push(json!({
                        "type": "replenish",
                        "productId": p.product_id,
                        "productName": p.product_name,
                        "suggestion": MSG_RECOMMEND_REPLENISH,
                        "reason": format!("库存风险评分 {score}"),
                    }));
                }
            }
        }
        "price_bands" => {
            let mut bands: BTreeMap<&'static str, (i32, i64)> = BTreeMap::new();
            for p in rows {
                let band = price_band(Some(p.product_price));
                let e = bands.entry(band).or_insert((0, 0));
                e.0 += 1;
                e.1 += i64::from(p.virtual_sales);
            }
            for (band, (count, sales)) in &bands {
                rankings.push(json!({
                    "band": band,
                    "productCount": count,
                    "sales": sales,
                }));
            }
            if let Some(median) = counters.price_median {
                recommendations.push(json!({
                    "type": "pricing",
                    "value": normalize_number(median).to_string(),
                    "suggestion": MSG_RECOMMEND_PRICING,
                    "reason": MSG_RECOMMEND_PRICING_REASON,
                }));
            }
        }
        "category_performance" => {
            let mut by_cat: BTreeMap<i32, (i32, i64, Decimal)> = BTreeMap::new();
            for p in rows {
                let e = by_cat.entry(p.category_id).or_insert((0, 0, Decimal::ZERO));
                e.0 += 1;
                e.1 += i64::from(p.virtual_sales);
                e.2 = e.2.checked_add(p.product_price).unwrap_or(e.2);
            }
            let mut list: Vec<(i32, (i32, i64, Decimal))> = by_cat.into_iter().collect();
            list.sort_by(|a, b| b.1 .1.cmp(&a.1 .1));
            for (cid, (count, sales, _)) in list.iter().take(ANALYSIS_RANKING_LIMIT) {
                rankings.push(json!({
                    "categoryId": cid,
                    "category": category_of(Some(&format!("分类{cid}"))),
                    "productCount": count,
                    "sales": sales,
                }));
            }
            if let Some((cid, (_, sales, _))) = list.first() {
                recommendations.push(json!({
                    "type": "category",
                    "categoryId": cid,
                    "value": category_of(Some(&format!("分类{cid}"))),
                    "suggestion": MSG_RECOMMEND_CATEGORY,
                    "reason": MSG_RECOMMEND_CATEGORY_REASON,
                }));
            }
        }
        "recommend" => {
            for p in by_sales.iter().take(ANALYSIS_RANKING_LIMIT) {
                rankings.push(json!({
                    "productId": p.product_id,
                    "productName": p.product_name,
                    "sales": p.virtual_sales,
                    "score": p.virtual_sales,
                }));
            }
            recommendations.push(json!({
                "type": "pricing",
                "value": normalize_number(counters.price_median.unwrap_or(counters.price_max.unwrap_or(Decimal::ZERO))).to_string(),
                "suggestion": MSG_RECOMMEND_PRICING,
                "reason": MSG_RECOMMEND_PRICING_REASON,
            }));
            if let Some(top) = by_sales.first() {
                recommendations.push(json!({
                    "type": "replenish",
                    "productId": top.product_id,
                    "productName": top.product_name,
                    "suggestion": MSG_RECOMMEND_REPLENISH,
                    "reason": format!("销量 {} 为样本最高", top.virtual_sales),
                }));
            }
        }
        _ => {}
    }

    (metrics, rankings, recommendations)
}

// ——————————————————————————————————————————————
// 过期清理（对应 Java 的 evictExpired）
// ——————————————————————————————————————————————

fn evict_expired_previews(m: &mut HashMap<String, StoredPreview>) {
    let cutoff = chrono::Utc::now().timestamp_millis() - PREVIEW_TTL_MILLIS;
    m.retain(|_, v| v.created_at_ms > cutoff);
}

fn evict_expired_idem(m: &mut HashMap<String, StoredIdempotency>) {
    let cutoff = chrono::Utc::now().timestamp_millis() - RESULT_TTL_MILLIS;
    m.retain(|_, v| v.created_at_ms > cutoff);
}

fn evict_expired_batch(m: &mut HashMap<String, StoredBatchSnapshot>) {
    let cutoff = chrono::Utc::now().timestamp_millis() - PREVIEW_TTL_MILLIS;
    m.retain(|_, v| v.created_at_ms > cutoff);
}

fn evict_expired_tokens(m: &mut HashMap<String, StoredUploadToken>) {
    let cutoff = chrono::Utc::now().timestamp_millis() - TOKEN_TTL_MILLIS;
    m.retain(|_, v| v.created_at_ms > cutoff);
}

fn evict_expired_category_previews(m: &mut HashMap<String, StoredCategoryPreview>) {
    let cutoff = chrono::Utc::now().timestamp_millis() - PREVIEW_TTL_MILLIS;
    m.retain(|_, v| v.created_at_ms > cutoff);
}

fn evict_expired_category_results(m: &mut HashMap<String, StoredCategoryResult>) {
    let cutoff = chrono::Utc::now().timestamp_millis() - RESULT_TTL_MILLIS;
    m.retain(|_, v| v.created_at_ms > cutoff);
}

fn evict_expired_media(m: &mut HashMap<String, StoredMediaPreview>) {
    let cutoff = chrono::Utc::now().timestamp_millis() - PREVIEW_TTL_MILLIS;
    m.retain(|_, v| v.created_at_ms > cutoff);
}

#[cfg(test)]
mod tests {
    use super::*;
    // `NaiveDateTime::hour()` 来自 `Timelike` trait，必须显式导入
    // （否则 E0599: no method named `hour`）。
    use chrono::Timelike as _;

    // ————————————————————— 错误体 / 状态码归一 —————————————————————

    /// Java `resolveStatus`：白名单外一律 422。
    #[test]
    fn http_status_normalization() {
        assert_eq!(resolve_http_status(400), 400);
        assert_eq!(resolve_http_status(401), 401);
        assert_eq!(resolve_http_status(403), 403);
        assert_eq!(resolve_http_status(404), 404);
        assert_eq!(resolve_http_status(409), 409);
        // 500 / 200 / 422 / 其他 ⇒ 422
        assert_eq!(resolve_http_status(500), 422);
        assert_eq!(resolve_http_status(200), 422);
        assert_eq!(resolve_http_status(422), 422);
        assert_eq!(resolve_http_status(0), 422);
    }

    /// 构造器与错误体形态。
    #[test]
    fn work_buddy_error_body_shape() {
        let e = WorkBuddyError::bad_request(CODE_REQUIRED, "productName 不能为空");
        assert_eq!(e.status(), 400);
        assert!(!e.retryable);
        let body = e.body();
        assert_eq!(body.error_code, "REQUIRED");
        assert_eq!(body.message, "productName 不能为空");
        assert!(!body.retryable);

        let json = serde_json::to_value(e.body()).unwrap();
        assert_eq!(json["errorCode"], "REQUIRED");
        assert_eq!(json["message"], "productName 不能为空");
        assert_eq!(json["retryable"], false);

        // 401 带固定 errorCode `WORKBUDDY_KEY_INVALID` 的兜底由 handler 负责，
        // 服务层显式构造时用自己的 errorCode。
        let e = WorkBuddyError::unauthorized(MSG_KEY_INVALID);
        assert_eq!(e.status(), 401);
        assert_eq!(e.error_code, MSG_KEY_INVALID);

        let e = WorkBuddyError::forbidden(CODE_PLATFORM_SCOPE_REQUIRED, "x");
        assert_eq!(e.status(), 403);
        let e = WorkBuddyError::not_found(CODE_PRODUCT_NOT_FOUND, "x");
        assert_eq!(e.status(), 404);
        // 422 经归一仍是 422
        let e = WorkBuddyError::unprocessable(CODE_OPERATION_FAILED, "x");
        assert_eq!(e.status(), 422);
        // 500 归一为 422（Java handler 的可观测行为）
        let e = WorkBuddyError::new("INTERNAL_ERROR", "x", 500, true);
        assert_eq!(e.status(), 422);
        assert!(e.retryable);
    }

    /// 错误体 JSON 键必须是 camelCase。
    #[test]
    fn error_body_json_keys() {
        let v = serde_json::to_value(WorkBuddyErrorBody::new("A", "b", true)).unwrap();
        assert!(v.get("errorCode").is_some());
        assert!(v.get("message").is_some());
        assert!(v.get("retryable").is_some());
    }

    /// `WorkBuddyError` → `nextrade_core::Error::Biz` 的转换（供非 workbuddy 路径复用）。
    #[test]
    fn work_buddy_error_converts_to_core_error() {
        let e = WorkBuddyError::bad_request(CODE_REQUIRED, "缺字段");
        let core: nextrade_core::Error = e.into();
        match core {
            nextrade_core::Error::Biz { code, msg } => {
                assert_eq!(code, 400);
                assert_eq!(msg, "缺字段");
            }
            other => panic!("应为 Biz，实际 {other:?}"),
        }
    }

    // ————————————————————— 密钥 —————————————————————

    /// `WorkBuddyError::unauthorized` 固定用 `MCP密钥无效` 作为 errorCode
    /// （与 Java 401 的语义一致：密钥类错误统一走该码）。
    #[test]
    fn unauthorized_uses_key_invalid_code() {
        let e = WorkBuddyError::unauthorized("MCP密钥已过期");
        assert_eq!(e.error_code, MSG_KEY_INVALID);
        assert_eq!(e.message, "MCP密钥已过期");
        assert_eq!(e.status(), 401);
        assert!(!e.retryable);
    }

    // ————————————————————— 时间解析 —————————————————————

    /// 支持的三种时间格式 + 非法输入。
    #[test]
    fn datetime_parsing_formats() {
        let a = parse_datetime(Some("2026-01-02 03:04:05")).unwrap();
        let b = parse_datetime(Some("2026-01-02T03:04:05")).unwrap();
        let c = parse_datetime(Some("2026-01-02")).unwrap();
        assert_eq!(a, b);
        assert_eq!(c.hour(), 0);
        // RFC3339 带时区
        let d = parse_datetime(Some("2026-01-02T03:04:05+08:00")).unwrap();
        assert_eq!(d.to_string(), "2026-01-01 19:04:05");
        // 非法/空 ⇒ None
        assert!(parse_datetime(None).is_none());
        assert!(parse_datetime(Some("")).is_none());
        assert!(parse_datetime(Some("   ")).is_none());
        assert!(parse_datetime(Some("not-a-date")).is_none());
    }

    // ————————————————————— Base64 —————————————————————

    /// Base64 前缀解码：能取到魔数字节；非 Base64 字符 ⇒ None。
    #[test]
    fn base64_prefix_decoding() {
        // "iVBORw0KGgo" 是 PNG 的 base64 开头
        let bytes = decode_base64_prefix("iVBORw0KGgo").unwrap();
        assert_eq!(&bytes[0..4], &[137, 80, 78, 71]);

        let bytes = decode_base64_prefix("/9j/").unwrap();
        assert_eq!(&bytes[0..3], &[255, 216, 255]);

        let bytes = decode_base64_prefix("R0lGODlh").unwrap();
        assert!(bytes.starts_with(b"GIF"));

        // 空白与 `=` 被忽略。
        // ⚠️ 注意是**小写 `w`**（`Rw0K`）—— 与上面 `iVBORw0KGgo` 逐字符一致；
        // 误写成大写 `W` 会解出 `[137,80,78,69]`（第 4 字节 69 而非 71）。
        let bytes = decode_base64_prefix("iVBO\nRw0K  Ggo==").unwrap();
        assert_eq!(&bytes[0..4], &[137, 80, 78, 71]);

        // 非法字符 ⇒ None
        assert!(decode_base64_prefix("!!!!").is_none());
        assert!(decode_base64_prefix("iVBO*").is_none());
    }

    // ————————————————————— 分类路径 —————————————————————

    /// 路径拼接与自引用保护。
    #[test]
    fn category_path_building() {
        let mut by_id: BTreeMap<i32, (String, i32)> = BTreeMap::new();
        by_id.insert(1, ("女鞋".to_string(), 0));
        by_id.insert(2, ("运动系列".to_string(), 1));
        by_id.insert(3, ("跑鞋".to_string(), 2));
        assert_eq!(build_category_path(&by_id, 1), "女鞋");
        assert_eq!(build_category_path(&by_id, 2), "女鞋/运动系列");
        assert_eq!(build_category_path(&by_id, 3), "女鞋/运动系列/跑鞋");
        // 未知 ID ⇒ 空
        assert_eq!(build_category_path(&by_id, 99), "");
        // 自引用不死循环
        let mut cyc: BTreeMap<i32, (String, i32)> = BTreeMap::new();
        cyc.insert(1, ("A".to_string(), 1));
        let p = build_category_path(&cyc, 1);
        assert!(p.starts_with('A'));
    }

    // ————————————————————— 归一化草稿 —————————————————————

    /// `normalize_draft`：填补市场价、归一化图片、解析分类。
    #[test]
    fn normalize_draft_rules() {
        let raw = ProductDraftDTO {
            product_name: Some("  T恤  ".to_string()),
            product_sn: Some(" SN-1 ".to_string()),
            product_price: Some(dec("99.00")),
            category_id: None,
            product_status: None,
            images: vec![" a.jpg ".to_string(), "a.jpg".to_string(), String::new()],
            media: vec![ProductMediaDraftDTO {
                r#type: Some("image".to_string()),
                role: Some("detail_image".to_string()),
                url: Some(" d.jpg ".to_string()),
                ..ProductMediaDraftDTO::default()
            }],
            ..ProductDraftDTO::default()
        };
        let images = dedupe_urls(&candidate_images(&raw));
        let d = normalize_draft(&raw, Some(7), &images, dec("1.2"));
        assert_eq!(d.product_name.as_deref(), Some("T恤"));
        assert_eq!(d.product_sn.as_deref(), Some("SN-1"));
        assert_eq!(d.category_id, Some(7));
        assert_eq!(d.product_status, Some(PRODUCT_STATUS_ON_SALE));
        assert_eq!(d.images, vec!["a.jpg".to_string()]);
        assert_eq!(d.detail_images, vec!["d.jpg".to_string()]);
        // 99 × 1.2 = 118.80
        assert_eq!(d.market_price, Some(dec("118.80")));
    }

    /// 已给市场价时不覆盖。
    #[test]
    fn normalize_draft_keeps_given_market_price() {
        let raw = ProductDraftDTO {
            product_price: Some(dec("99.00")),
            market_price: Some(dec("150.00")),
            ..ProductDraftDTO::default()
        };
        let d = normalize_draft(&raw, None, &[], dec("1.2"));
        assert_eq!(d.market_price, Some(dec("150.00")));
    }

    /// 没有售价时市场价为 None。
    #[test]
    fn normalize_draft_without_price() {
        let raw = ProductDraftDTO::default();
        let d = normalize_draft(&raw, None, &[], dec("1.2"));
        assert!(d.market_price.is_none());
        assert!(d.product_price.is_none());
    }

    // ————————————————————— 编码推断 —————————————————————

    /// 样本不足（< 2）⇒ needsConfirmation + 专用文案。
    #[test]
    fn infer_encoding_too_few_samples() {
        assert!(infer_encoding(&[]).is_some());
        let e = infer_encoding(&["SKU001".to_string()]).unwrap();
        assert_eq!(e.confidence, Some(0.0));
        assert!(e.needs_confirmation);
        assert_eq!(e.reason.as_deref(), Some(MSG_ENCODING_TOO_FEW));
        assert!(e.next_candidate.is_none());
    }

    /// 一致的 4 位样本 ⇒ 给出 nextCandidate 且 confidence 被压到 0.99。
    #[test]
    fn infer_encoding_decisive_candidate() {
        let sns: Vec<String> = ["SKU0001", "SKU0002", "SKU0003", "SKU0004"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let e = infer_encoding(&sns).unwrap();
        assert_eq!(e.prefix.as_deref(), Some("SKU"));
        assert_eq!(e.number_length, Some(4));
        assert_eq!(e.next_candidate.as_deref(), Some("SKU0005"));
        assert_eq!(e.confidence, Some(0.99));
        assert_eq!(e.reason.as_deref(), Some(MSG_ENCODING_DOMINANT_EXACT));
    }

    /// 格式混杂 ⇒ 不明确。
    #[test]
    fn infer_encoding_inconsistent() {
        let sns: Vec<String> = ["A001", "A002", "A003", "B001", "B002", "B003"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let e = infer_encoding(&sns).unwrap();
        assert!(e.needs_confirmation);
        assert!(e.next_candidate.is_none());
        assert_eq!(e.confidence, Some(0.2));
        assert_eq!(e.reason.as_deref(), Some(MSG_ENCODING_INCONSISTENT));
    }

    /// 无任何数字后缀 ⇒ NoGroups 分支。
    #[test]
    fn infer_encoding_no_digit_suffix() {
        let sns: Vec<String> = ["ABC", "DEF"].iter().map(|s| (*s).to_string()).collect();
        let e = infer_encoding(&sns).unwrap();
        assert_eq!(e.confidence, Some(0.2));
        assert!(e.next_candidate.is_none());
        assert_eq!(e.reason.as_deref(), Some(MSG_ENCODING_INCONSISTENT));
    }

    /// 序号溢出（999 → 1000）⇒ OVERFLOW 文案与 0.35 置信度。
    #[test]
    fn infer_encoding_overflow_branch() {
        let sns: Vec<String> = ["SKU999", "SKU998", "SKU997", "SKU996"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let e = infer_encoding(&sns).unwrap();
        assert_eq!(e.prefix.as_deref(), Some("SKU"));
        assert_eq!(e.number_length, Some(3));
        assert!(e.next_candidate.is_none());
        assert_eq!(e.confidence, Some(0.35));
        assert_eq!(e.reason.as_deref(), Some(MSG_ENCODING_OVERFLOW));
    }

    /// 主要格式但样本数不足 3 ⇒ 走不明确分支。
    #[test]
    fn infer_encoding_dominant_but_few() {
        let sns: Vec<String> = ["SKU001", "SKU002"].iter().map(|s| (*s).to_string()).collect();
        let e = infer_encoding(&sns).unwrap();
        assert!(e.next_candidate.is_none());
        assert_eq!(e.confidence, Some(0.2));
    }

    /// 主流格式占多数但不足 2 倍 ⇒ 不明确。
    #[test]
    fn infer_encoding_majority_not_double() {
        let sns: Vec<String> = ["A001", "A002", "A003", "A004", "B001", "B002", "B003"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let e = infer_encoding(&sns).unwrap();
        assert!(e.next_candidate.is_none());
        assert_eq!(e.reason.as_deref(), Some(MSG_ENCODING_INCONSISTENT));
    }

    /// 明确但非全一致 ⇒ 用 MAJORITY 文案。
    #[test]
    fn infer_encoding_majority_message() {
        // 6 个 A 组 + 3 个 B 组 ⇒ 6 >= 3*2，confidence = 6/9 = 0.667 >= 0.6
        let sns: Vec<String> = ["A01", "A02", "A03", "A04", "A05", "A06", "B01", "B02", "B03"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let e = infer_encoding(&sns).unwrap();
        assert_eq!(e.prefix.as_deref(), Some("A"));
        assert_eq!(e.next_candidate.as_deref(), Some("A07"));
        assert_eq!(e.reason.as_deref(), Some(MSG_ENCODING_DOMINANT_MAJORITY));
        // Java：`setConfidence(Math.min(0.99d, confidence))`，
        // 此处 confidence = 6/9 ≈ 0.6667 < 0.99 ⇒ 取 6/9，**不是** 0.99。
        let expected = 6.0_f64 / 9.0_f64;
        assert!(
            (e.confidence.expect("应有置信度") - expected).abs() < 1e-9,
            "置信度应为 6/9≈{expected}，实际 {:?}",
            e.confidence
        );
    }

    // ————————————————————— 分析指标 —————————————————————

    /// 分析 metrics 的构造（空行集合不应 panic）。
    #[test]
    fn analysis_metrics_shape() {
        let counters = AnalysisCounters::from_rows(std::iter::empty());
        let (metrics, rankings, recs) = build_analysis("overview", &[], &counters);
        assert_eq!(metrics["productCount"], 0);
        assert!(rankings.is_empty());
        assert!(recs.is_empty());
        // priceAvg 恒有值（空集为 0）
        assert_eq!(metrics["priceAvg"], "0");
    }

    /// 各 operation 都能产出各自的 metrics/rankings，不 panic。
    #[test]
    fn analysis_operations_do_not_panic_on_empty() {
        let counters = AnalysisCounters::from_rows(std::iter::empty());
        for op in ANALYSIS_OPERATIONS {
            let (metrics, _r, _rec) = build_analysis(op, &[], &counters);
            assert!(metrics.is_object(), "operation {op} 应返回对象");
        }
        // 未知 operation 也只是空排行
        let (metrics, rankings, recs) = build_analysis("bogus", &[], &counters);
        assert!(metrics.is_object());
        assert!(rankings.is_empty());
        assert!(recs.is_empty());
    }

    // ————————————————————— 过期清理 —————————————————————

    /// 过期条目被清理。
    #[test]
    fn eviction_removes_expired_entries() {
        let mut m: HashMap<String, StoredPreview> = HashMap::new();
        m.insert(
            "old".to_string(),
            StoredPreview {
                preview_id: "old".to_string(),
                request_id: None,
                shop_id: 0,
                items: Vec::new(),
                missing_fields: Vec::new(),
                importable: true,
                created_at_ms: chrono::Utc::now().timestamp_millis() - PREVIEW_TTL_MILLIS - 1,
                confirmed: false,
            },
        );
        m.insert(
            "new".to_string(),
            StoredPreview {
                preview_id: "new".to_string(),
                request_id: None,
                shop_id: 0,
                items: Vec::new(),
                missing_fields: Vec::new(),
                importable: true,
                created_at_ms: chrono::Utc::now().timestamp_millis(),
                confirmed: false,
            },
        );
        evict_expired_previews(&mut m);
        assert_eq!(m.len(), 1);
        assert!(m.contains_key("new"));
    }

    /// 令牌过期清理。
    #[test]
    fn eviction_removes_expired_tokens() {
        let mut m: HashMap<String, StoredUploadToken> = HashMap::new();
        m.insert(
            "t".to_string(),
            StoredUploadToken {
                file_token: "t".to_string(),
                shop_id: 0,
                file_name: "a.png".to_string(),
                mime_type: "image/png".to_string(),
                size: 1,
                created_at_ms: chrono::Utc::now().timestamp_millis() - TOKEN_TTL_MILLIS - 1,
            },
        );
        evict_expired_tokens(&mut m);
        assert!(m.is_empty());
    }

    // ————————————————————— 请求体反序列化 —————————————————————

    /// 批量体的 camelCase 映射与 \"null\" 元素保留。
    #[test]
    fn batch_body_deserialization() {
        let b: BatchBody =
            serde_json::from_str(r#"{"productIds":[1,null,2],"operation":"up"}"#).unwrap();
        assert_eq!(b.operation.as_deref(), Some("up"));
        assert_eq!(b.product_ids.unwrap(), vec![Some(1), None, Some(2)]);

        let b: BatchBody = serde_json::from_str("{}").unwrap();
        assert!(b.product_ids.is_none());
        assert!(b.operation.is_none());
    }

    /// 批量执行体的 previewId / confirmation。
    #[test]
    fn batch_execute_body_deserialization() {
        let b: BatchExecuteBody = serde_json::from_str(
            r#"{"productIds":[1],"operation":"del","previewId":"p1","confirmation":true}"#,
        )
        .unwrap();
        assert_eq!(b.preview_id.as_deref(), Some("p1"));
        assert_eq!(b.confirmation, Some(true));
        // confirmation 缺省为 None（≠ true，属于未确认）
        let b: BatchExecuteBody =
            serde_json::from_str(r#"{"productIds":[1],"operation":"del"}"#).unwrap();
        assert_eq!(b.confirmation, None);
    }

    /// 字段体 / 状态体。
    #[test]
    fn field_and_status_body() {
        let f: FieldBody =
            serde_json::from_str(r#"{"field":"productPrice","value":"19.90"}"#).unwrap();
        assert_eq!(f.field, "productPrice");
        assert_eq!(f.value.as_deref(), Some("19.90"));

        let s: StatusBody = serde_json::from_str(r#"{"onSale":false}"#).unwrap();
        assert_eq!(s.on_sale, Some(false));
        let s: StatusBody = serde_json::from_str("{}").unwrap();
        assert_eq!(s.on_sale, None);
    }

    /// 导入体：previewId / confirmation / products 全缺省也不报错。
    #[test]
    fn import_body_deserialization() {
        let b: ImportBody = serde_json::from_str("{}").unwrap();
        assert!(b.products.is_empty());
        assert!(b.preview_id.is_none());
        assert!(b.confirmation.is_none());

        let b: ImportBody = serde_json::from_str(
            r#"{"previewId":"p","confirmation":true,"products":[{"productName":"n"}]}"#,
        )
        .unwrap();
        assert_eq!(b.products.len(), 1);
        assert_eq!(b.confirmation, Some(true));
    }

    /// 查询参数体。
    #[test]
    fn query_bodies() {
        let q: ProductListQuery = serde_json::from_str(
            r#"{"keyword":"鞋","scope":"recycled","sortBy":"price","order":"asc","page":2,"size":50,"inStockOnly":true,"mineOnly":false}"#,
        )
        .unwrap();
        assert_eq!(q.scope.as_deref(), Some("recycled"));
        assert_eq!(q.sort_by.as_deref(), Some("price"));
        assert_eq!(q.page, Some(2));
        assert_eq!(q.in_stock_only, Some(true));

        let q: ContextQuery =
            serde_json::from_str(r#"{"keyword":"x","categoryId":3,"sampleSize":10}"#).unwrap();
        assert_eq!(q.category_id, Some(3));
        assert_eq!(q.sample_size, Some(10));

        let q: AnalysisQuery = serde_json::from_str(
            r#"{"operation":"price_bands","startTime":"2026-01-01","endTime":"2026-02-01","sampleSize":50}"#,
        )
        .unwrap();
        assert_eq!(q.operation.as_deref(), Some("price_bands"));
        assert_eq!(q.start_time.as_deref(), Some("2026-01-01"));

        let q: CategoryQuery = serde_json::from_str(r#"{"keyword":"鞋","parentId":0,"size":10}"#).unwrap();
        assert_eq!(q.parent_id, Some(0));

        let q: UploadTokenBody =
            serde_json::from_str(r#"{"name":"a.png","mimeType":"image/png","size":100}"#).unwrap();
        assert_eq!(q.size, Some(100));
        assert_eq!(q.mime_type.as_deref(), Some("image/png"));
    }

    /// 分类导入体（含递归 nodes）。
    #[test]
    fn category_import_body_recursive() {
        let d: CategoryImportDTO = serde_json::from_str(
            r#"{"operation":"import","requestId":"r","previewId":"p","confirmation":true,"nodes":[{"name":"A","children":[{"name":"B"}]}]}"#,
        )
        .unwrap();
        assert_eq!(d.operation.as_deref(), Some("import"));
        assert_eq!(d.nodes.len(), 1);
        assert_eq!(d.nodes[0].children.len(), 1);
        assert_eq!(d.confirmation, Some(true));
    }

    // ————————————————————— 媒体体 —————————————————————

    /// 媒体操作体（含 items 里的别名）。
    #[test]
    fn media_operation_body() {
        let op: MediaOperationDTO = serde_json::from_str(
            r#"{"operation":"update","id":3,"ids":[1,2],"name":"n","type":"image","groupId":4,"sort":9,"previewId":"p","requestId":"r","confirmation":true,"localPath":"/tmp/a"}"#,
        )
        .unwrap();
        assert_eq!(op.operation.as_deref(), Some("update"));
        assert_eq!(op.ids, Some(vec![1, 2]));
        assert_eq!(op.sort, Some(9));
        assert_eq!(op.local_path.as_deref(), Some("/tmp/a"));
        // 默认全部缺省
        let op: MediaOperationDTO = serde_json::from_str("{}").unwrap();
        assert!(op.items.is_empty());
        assert!(op.id.is_none());
        assert!(op.confirmation.is_none());
    }

    /// 媒体查询体。
    #[test]
    fn media_query_body() {
        let q: MediaQueryDTO =
            serde_json::from_str(r#"{"operation":"groups","type":"video","groupId":1,"keyword":"k","page":1,"pageSize":20}"#)
                .unwrap();
        assert_eq!(q.operation.as_deref(), Some("groups"));
        assert_eq!(q.r#type.as_deref(), Some("video"));
        assert_eq!(q.page_size, Some(20));
    }

    /// 上传请求体：confirmation 缺省为 false（Java 的 boolean 默认值）。
    #[test]
    fn media_upload_request_body() {
        let r: MediaUploadRequestDTO = serde_json::from_str("{}").unwrap();
        assert!(!r.confirmation);
        assert!(r.files.is_empty());

        let r: MediaUploadRequestDTO = serde_json::from_str(
            r#"{"confirmation":true,"files":[{"name":"a.png","mimeType":"image/png","contentBase64":"iVBORw0KGgo"}]}"#,
        )
        .unwrap();
        assert!(r.confirmation);
        assert_eq!(r.files.len(), 1);
        assert_eq!(r.files[0].name.as_deref(), Some("a.png"));
    }

    /// 上传结果体 / 令牌体的 JSON 键。
    #[test]
    fn media_result_json_keys() {
        let v = serde_json::to_value(MediaUploadResultVO::default()).unwrap();
        for k in [
            "status",
            "total",
            "successCount",
            "failedCount",
            "assets",
            "failureSummary",
            "summary",
            "retryable",
        ] {
            assert!(v.get(k).is_some(), "缺少键 {k}");
        }

        let v = serde_json::to_value(MediaAssetVO::default()).unwrap();
        for k in [
            "clientFileId",
            "assetId",
            "type",
            "role",
            "url",
            "coverUrl",
            "format",
            "groupId",
        ] {
            assert!(v.get(k).is_some(), "缺少键 {k}");
        }

        let v = serde_json::to_value(MediaGroupVO::default()).unwrap();
        for k in ["id", "name", "assetCount", "type", "children"] {
            assert!(v.get(k).is_some(), "缺少键 {k}");
        }
    }

    /// 错误体 / 分类结果 Map 的键。
    #[test]
    fn media_error_and_category_map_keys() {
        let v = media_error(CODE_INVALID_ARGUMENT);
        assert_eq!(v["errorCode"], "INVALID_ARGUMENT");
        assert_eq!(v["status"], "failed");
        assert!(preserve_preview_for_retry(&json!({
            "status": "failed",
            "errorCode": "SERVICE_UNAVAILABLE"
        })));

        let v = category_error(CODE_INVALID_ARGUMENT, "nodes 不能为空");
        assert_eq!(v["status"], "failed");
        assert_eq!(v["errorCode"], "INVALID_ARGUMENT");
        assert_eq!(v["message"], "nodes 不能为空");

        let v = category_item_map("a/b", false, None, Some(CODE_MISSING_PARENT), Some("父级分类未创建，不能创建子级"), false);
        assert_eq!(v["path"], "a/b");
        assert_eq!(v["success"], false);
        assert_eq!(v["errorCode"], "MISSING_PARENT");
        assert_eq!(v["error"], "父级分类未创建，不能创建子级");
    }

    /// 导入结果 / 校验结果的 JSON 键。
    #[test]
    fn import_result_json_keys() {
        let v = serde_json::to_value(ProductImportResultVO::default()).unwrap();
        for k in ["requestId", "previewId", "total", "successCount", "failureCount", "items"] {
            assert!(v.get(k).is_some(), "缺少键 {k}");
        }
        let v = serde_json::to_value(ProductImportItem::default()).unwrap();
        for k in [
            "index",
            "success",
            "productId",
            "productName",
            "productSn",
            "productStatus",
            "errorCode",
            "message",
            "suggestion",
            "retryable",
            "verification",
        ] {
            assert!(v.get(k).is_some(), "缺少键 {k}");
        }
        let v = serde_json::to_value(ProductImportVerification::default()).unwrap();
        for k in ["status", "checkedFields", "mismatches", "message"] {
            assert!(v.get(k).is_some(), "缺少键 {k}");
        }

        let v = serde_json::to_value(ProductValidationVO::default()).unwrap();
        for k in ["previewId", "total", "valid", "missingFields", "items"] {
            assert!(v.get(k).is_some(), "缺少键 {k}");
        }
        let v = serde_json::to_value(ProductValidationItem::default()).unwrap();
        for k in [
            "index",
            "productName",
            "productSn",
            "valid",
            "errors",
            "warnings",
            "source",
            "product",
            "missingFields",
        ] {
            assert!(v.get(k).is_some(), "缺少键 {k}");
        }
    }

    /// 上下文 / 分析 VO 的 JSON 键。
    #[test]
    fn context_and_analysis_json_keys() {
        let v = serde_json::to_value(ProductContextVO::default()).unwrap();
        for k in [
            "hasData",
            "categoryId",
            "categoryName",
            "sampleSize",
            "metrics",
            "patterns",
            "samples",
            "fallback",
            "encoding",
            "defaults",
        ] {
            assert!(v.get(k).is_some(), "缺少键 {k}");
        }

        let v = serde_json::to_value(ProductAnalysisVO::default()).unwrap();
        for k in [
            "metrics",
            "rankings",
            "recommendations",
            "dataScope",
            "sampleSize",
            "generatedAt",
            "dataStartTime",
            "dataEndTime",
            "confidence",
            "fallbackMessage",
        ] {
            assert!(v.get(k).is_some(), "缺少键 {k}");
        }

        let v = serde_json::to_value(ProductContextMetrics::default()).unwrap();
        for k in [
            "salesAvg",
            "salesMedian",
            "stockAvg",
            "stockMedian",
            "priceMin",
            "priceMedian",
            "priceMax",
            "productCount",
        ] {
            assert!(v.get(k).is_some(), "缺少键 {k}");
        }
    }

    /// 分类预览 VO 的 JSON 键。
    #[test]
    fn category_preview_json_keys() {
        let v = serde_json::to_value(CategoryImportPreviewVO::default()).unwrap();
        for k in [
            "previewId",
            "status",
            "summary",
            "nodes",
            "total",
            "existingCount",
            "missingParentCount",
            "conflictCount",
        ] {
            assert!(v.get(k).is_some(), "缺少键 {k}");
        }
        let v = serde_json::to_value(CategoryImportNodeResult::default()).unwrap();
        for k in ["path", "name", "id", "hasChildren", "leaf", "state", "reason"] {
            assert!(v.get(k).is_some(), "缺少键 {k}");
        }
    }

    // ————————————————————— 静态常量一致性 —————————————————————

    /// reference 用的常量与 Java 对齐（防手抖改错）。
    #[test]
    fn service_constants_sane() {
        assert_eq!(EDITABLE_FIELDS.len(), 6);
        assert_eq!(ANALYSIS_OPERATIONS.len(), 7);
        assert_eq!(MEDIA_SUPPORTED_OPERATIONS.len(), 6);
        assert_eq!(BATCH_MAX_IDS, 50);
        assert_eq!(BATCH_SORT_LIMIT, 20);
        assert_eq!(ANALYSIS_RANKING_LIMIT, 20);
        assert_eq!(MAX_PRODUCTS, 50);
        assert_eq!(MAX_FILES, 20);
        assert_eq!(PREVIEW_TTL_MILLIS, 1_800_000);
        assert_eq!(RESULT_TTL_MILLIS, 86_400_000);
        assert_eq!(TOKEN_TTL_MILLIS, 1_800_000);
        assert_eq!(DOMINANT_ENCODING_THRESHOLD, 0.6);
        assert_eq!(MIN_DOMINANT_ENCODING_SAMPLES, 3);
        assert_eq!(CONTEXT_SAMPLES_CAP, 20);
        assert_eq!(ANALYSIS_DEFAULT_SAMPLE_SIZE, 100);
        assert_eq!(DEFAULT_MARKET_PRICE_RATE_STR, "1.2");
        assert_eq!(CHECK_STATUS_APPROVED, 0);
        assert_eq!(CHECK_STATUS_REJECTED, 2);
        assert_eq!(PRODUCT_STATUS_ON_SALE, 1);
        assert_eq!(SKU_TYPE_SINGLE, 1);
        assert_eq!(SKU_TYPE_MULTI, 2);
        assert_eq!(PRODUCT_TYPE_NORMAL, 1);
        assert_eq!(ANALYSIS_DATA_SCOPE, "NexTrade内部商品数据");
        assert_eq!(ANALYSIS_FALLBACK_MESSAGE, "无足够内部数据，仅可提供通用经验");
        assert_eq!(CONFIG_CODE_WORKBUDDY_API_KEY, "workbuddyApiKey");
        assert_eq!(CONFIG_CODE_WORKBUDDY_API_KEY_EXPIRES_AT, "workbuddyApiKeyExpiresAt");
        assert_eq!(CONFIG_CODE_CUSTOM_PRODUCT_NAME_LENGTH, "customProductNameLength");
        assert_eq!(CONFIG_CODE_MARKET_PRICE_RATE, "marketPriceRate");
        assert_eq!(CONFIG_CODE_IMAGE_UPLOAD_MAX_SIZE, "imageUploadMaxSize");
        assert_eq!(CATEGORY_IMPORT_PAGE_SIZE, 1000);
        assert_eq!(MAX_SKU_SN_LENGTH, 60);
    }
}"