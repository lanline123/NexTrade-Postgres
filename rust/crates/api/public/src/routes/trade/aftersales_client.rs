//! 商城前台用户售后路由（trade::aftersales_client）。
//!
//! 对应原 Java `com.nextrade.api.controller.user.AfterSalesController`
//! （`@RequestMapping("/api/user/aftersales")`）。
//!
//! | 本模块路由（挂载于 `/api`） | 原 Java 方法 | 契约 auth |
//! |------------------------------|--------------|-----------|
//! | `GET  /user/aftersales/list` | `AfterSalesController.list` | optional |
//! | `GET  /user/aftersales/config` | `AfterSalesController.config` | optional |
//! | `GET  /user/aftersales/applyData` | `AfterSalesController.applyData` | optional |
//! | `POST /user/aftersales/create` | `AfterSalesController.create` | optional |
//! | `POST /user/aftersales/update` | `AfterSalesController.update` | optional |
//! | `GET  /user/aftersales/getRecord` | `AfterSalesController.getRecord` | optional |
//! | `GET  /user/aftersales/detail` | `AfterSalesController.detail` | optional |
//! | `GET  /user/aftersales/detailLog` | `AfterSalesController.detailLog` | optional |
//! | `POST /user/aftersales/feedback` | `AfterSalesController.feedback` | optional |
//! | `POST /user/aftersales/cancel` | `AfterSalesController.cancel` | optional |
//!
//! > **路径说明**：控制器前缀**重复一次**（`/api/user/aftersales/...`），
//! > 这不是笔误，必须逐字符照抄；本文件挂在 `/api` 之下，
//! > 故 `routes()` 内只写 `/user/aftersales/...`。
//! >
//! > **⚠️ auth=optional 的落地口径**：契约把本域 10 条都标为 `optional`，
//! > 但 Java 实现**全部**无条件调用 `SecurityUtils.getCurrentUserId()`
//! > 来限定数据归属（`list` / `getRecord` / `applyData` / `update` /
//! > `detail` / `detailLog` / `feedback` / `cancel` 一进方法就取 userId），
//! > 匿名访问在 Java 侧同样无法得到可用的数据或必然越权。
//! > 因此本实现：
//! > - 保持路由挂在**公开**组（不因缺 `Authorization` 而 401 直接拦截，
//! >   兼容契约标记）；
//! > - 每个 handler 用 `Option<Extension<AuthUser>>` 读登录态，
//! >   缺失时返回业务码 `401`「请先登录」，而不是静默当成游客。
//! > `config` 是唯一真正不依赖用户上下文的接口，但为保持前端一致性
//! > 同样要求登录。
//! >
//! > **未迁移**：多语言翻译（`translatePackage` / `MessageHelper`，文案取
//! > `i18n/messages.properties` 静态中文）、后台消息中心、微信小程序订单同步、
//! > 账期订单结算状态、RabbitMQ 自动同意退货 / 自动确认收货延时消息、
//! > 退款执行与退款申请（`refund_apply`）关联。

use axum::{
    extract::{Query, Request, State},
    http::header,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
use nextrade_core::models::trade::aftersales_client::{
    AfterSalesApplyDataVO, AfterSalesClientDetailVO, AfterSalesClientVO, AftersalesCancelParam,
    AftersalesConfigVO, AftersalesFeedbackParam, AftersalesListQuery, AftersalesLogVO,
    AftersalesVO, ClientAfterSalesUpdateParam, CreateAftersalesParam,
};
use nextrade_core::types::PageResult;
use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};
use nextrade_security::JwtService;

use crate::PublicState;

/// 售后域前台路由（挂载于 `/api` 下）。
///
/// **必须自带 [`optional_auth`]**：本域 10 条全部依赖登录会员上下文
/// （`current_user_id` 读 `Extension<AuthUser>`）。若不像其它 41 个 optional
/// 路由那样在本模块挂上中间件，`AuthUser` 永远不会被写入 extensions，
/// 于是**即使是已登录账号也会被判成未登录** → 全部返回业务码 401
/// （2026-10-21 实测：`/member/return/list` 页面 401 的根因）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/user/aftersales/list", get(aftersales_list))
        .route("/user/aftersales/config", get(aftersales_config))
        .route("/user/aftersales/applyData", get(aftersales_apply_data))
        .route("/user/aftersales/create", post(aftersales_create))
        .route("/user/aftersales/update", post(aftersales_update))
        .route("/user/aftersales/getRecord", get(aftersales_get_record))
        .route("/user/aftersales/detail", get(aftersales_detail))
        .route("/user/aftersales/detailLog", get(aftersales_detail_log))
        .route("/user/aftersales/feedback", post(aftersales_feedback))
        .route("/user/aftersales/cancel", post(aftersales_cancel))
        .layer(middleware::from_fn_with_state(
            state.jwt.clone(),
            optional_auth,
        ))
}

/// 可选登录态中间件（对应 Java `permitAll` + JWT 过滤器；同 `content/seo.rs`）：
///
/// - 合法 token → 写入 [`AuthUser`]；
/// - 无 token / 空 token / `Bearer null` / 过期 token → 游客，不返回 401。
async fn optional_auth(
    State(jwt): State<JwtService>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    if token.is_some() {
        let Ok(claims) = jwt.from_authorization(token) else {
            return Ok(next.run(req).await);
        };
        let Ok(id) = claims.sub.parse::<i32>() else {
            return Ok(next.run(req).await);
        };
        req.extensions_mut().insert(AuthUser {
            id,
            username: claims.usr,
            role_id: claims.role,
            admin_type: claims.typ,
            shop_id: claims.shop_id,
        });
    }
    Ok(next.run(req).await)
}

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    [
        (
            "GET",
            "/api/user/aftersales/list",
            "AfterSalesController.list",
        ),
        (
            "GET",
            "/api/user/aftersales/config",
            "AfterSalesController.config",
        ),
        (
            "GET",
            "/api/user/aftersales/applyData",
            "AfterSalesController.applyData",
        ),
        (
            "POST",
            "/api/user/aftersales/create",
            "AfterSalesController.create",
        ),
        (
            "POST",
            "/api/user/aftersales/update",
            "AfterSalesController.update",
        ),
        (
            "GET",
            "/api/user/aftersales/getRecord",
            "AfterSalesController.getRecord",
        ),
        (
            "GET",
            "/api/user/aftersales/detail",
            "AfterSalesController.detail",
        ),
        (
            "GET",
            "/api/user/aftersales/detailLog",
            "AfterSalesController.detailLog",
        ),
        (
            "POST",
            "/api/user/aftersales/feedback",
            "AfterSalesController.feedback",
        ),
        (
            "POST",
            "/api/user/aftersales/cancel",
            "AfterSalesController.cancel",
        ),
    ]
    .iter()
    .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, p, j))
    .collect()
}

/// 取登录会员 ID（对应 Java `SecurityUtils.getCurrentUserId()`）。
///
/// 本域挂在公开组，登录态为可选；缺失时返回业务码 `401`（与 Java
/// 「无登录态即无 userId」的失败路径等价，但不伪装成游客数据）。
fn current_user_id(auth: &Option<Extension<AuthUser>>) -> Result<i32, AppError> {
    auth.as_ref()
        .map(|Extension(a)| a.id)
        .filter(|id| *id > 0)
        .ok_or_else(|| AppError(Error::biz(401, "请先登录")))
}
/// `GET /api/user/aftersales/list`：可按订单申请售后的订单+商品列表。
///
/// 对应 Java `AfterSalesController.list(AftersalesListPageQuery)`
/// → `OrderServiceImpl#afterSalesList`。
async fn aftersales_list(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(mut q): Query<AftersalesListQuery>,
) -> ApiResult<PageResult<AfterSalesClientVO>> {
    let user_id = current_user_id(&auth)?;
    // Java：`listDTO.setUserId(getCurrentUserId())` 覆盖请求参数
    q.user_id = Some(user_id);
    ok(state.trade.aftersales_client.list(&q, user_id).await?)
}

/// `GET /api/user/aftersales/config?orderId=`：售后类型与原因配置。
///
/// 对应 Java `AfterSalesController.config(@RequestParam(required = false) orderId)`。
/// Java 中 `orderId` 被接收后**从未使用**（方法体里只有一个空 `if`），
/// 这里保留参数以兼容前端并显式忽略。
async fn aftersales_config(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<ConfigQuery>,
) -> ApiResult<AftersalesConfigVO> {
    let _ = current_user_id(&auth)?;
    ok(state.trade.aftersales_client.config(q.order_id).await?)
}

/// `/user/aftersales/config` 查询参数。
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ConfigQuery {
    order_id: Option<i32>,
}

/// `GET /api/user/aftersales/applyData?itemId=&orderId=`：售后申请页数据。
///
/// 对应 Java `AfterSalesController.applyData(itemId, orderId)`。
async fn aftersales_apply_data(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<ApplyDataQuery>,
) -> ApiResult<AfterSalesApplyDataVO> {
    let _ = current_user_id(&auth)?;
    ok(state
        .trade
        .aftersales_client
        .apply_data(q.item_id, q.order_id)
        .await?)
}

/// `/user/aftersales/applyData` 查询参数。
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ApplyDataQuery {
    item_id: Option<i32>,
    order_id: Option<i32>,
}

/// `POST /api/user/aftersales/create`：提交售后申请。
///
/// 对应 Java `AfterSalesController.create(@RequestBody @Validated
/// AftersalesApplyParam)`（原返回 `void`）。
/// 这里额外回传新建售后单的状态回执，便于前端联调（不改动原字段语义）。
async fn aftersales_create(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(param): Json<CreateAftersalesParam>,
) -> ApiResult<nextrade_services::trade::aftersales_client::CreateAftersalesResult> {
    let user_id = current_user_id(&auth)?;
    ok(state
        .trade
        .aftersales_client
        .create(&param, user_id)
        .await?)
}

/// `POST /api/user/aftersales/update`：修改售后申请。
///
/// 对应 Java `AfterSalesController.update(@RequestBody
/// ClientAfterSalesUpdateDTO)`（原返回 `void`）。
async fn aftersales_update(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(param): Json<ClientAfterSalesUpdateParam>,
) -> ApiResult<&'static str> {
    let user_id = current_user_id(&auth)?;
    state
        .trade
        .aftersales_client
        .update(&param, user_id)
        .await?;
    ok("修改成功")
}

/// `GET /api/user/aftersales/getRecord`：售后申请记录分页。
///
/// 对应 Java `AfterSalesController.getRecord(AftersalesListPageQuery)`
/// → `AftersalesServiceImpl#list`。
async fn aftersales_get_record(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(mut q): Query<AftersalesListQuery>,
) -> ApiResult<PageResult<AftersalesVO>> {
    let user_id = current_user_id(&auth)?;
    q.user_id = Some(user_id);
    ok(state
        .trade
        .aftersales_client
        .get_record(&q, user_id)
        .await?)
}

/// `GET /api/user/aftersales/detail?id=`：售后详情（含关联订单）。
///
/// 对应 Java `AfterSalesController.detail(@RequestParam Integer id)`。
async fn aftersales_detail(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<IdQuery>,
) -> ApiResult<AfterSalesClientDetailVO> {
    let user_id = current_user_id(&auth)?;
    ok(state.trade.aftersales_client.detail(q.id, user_id).await?)
}

/// `/user/aftersales/detail`、`/user/aftersales/detailLog` 查询参数。
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct IdQuery {
    id: i32,
}

/// `GET /api/user/aftersales/detailLog?id=`：售后日志。
///
/// 对应 Java `AfterSalesController.detailLog(@RequestParam Integer id)`
/// （内部先 `clientDetail(id, userId)` 做归属校验）。
async fn aftersales_detail_log(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<IdQuery>,
) -> ApiResult<Vec<AftersalesLogVO>> {
    let user_id = current_user_id(&auth)?;
    ok(state
        .trade
        .aftersales_client
        .detail_log(q.id, user_id)
        .await?)
}

/// `POST /api/user/aftersales/feedback`：提交回寄物流/反馈记录。
///
/// 对应 Java `AfterSalesController.feedback(@RequestBody
/// AfterSalesFeedbackDTO)`（原返回 `void`）。
async fn aftersales_feedback(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(param): Json<AftersalesFeedbackParam>,
) -> ApiResult<&'static str> {
    let user_id = current_user_id(&auth)?;
    state
        .trade
        .aftersales_client
        .feedback(&param, user_id)
        .await?;
    ok("提交成功")
}

/// `POST /api/user/aftersales/cancel`：取消售后申请。
///
/// 对应 Java `AfterSalesController.cancel(@RequestBody AfterSalesCancelDTO)`
/// （原返回 `void`）。
async fn aftersales_cancel(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(param): Json<AftersalesCancelParam>,
) -> ApiResult<&'static str> {
    let user_id = current_user_id(&auth)?;
    state
        .trade
        .aftersales_client
        .cancel(&param, user_id)
        .await?;
    ok("已取消")
}
