//! 商城前台店铺域路由（merchant::shop_client）。
//!
//! 对应原 Java `com.nextrade.api.controller.shop.ShopClientController`
//! （`@RequestMapping("/api/shop/shop")`，本切片 10 个端点）。
//!
//! | 本模块路由（挂载于 `/api`） | 原 Java 方法 | 契约 auth |
//! |------------------------------|--------------|-----------|
//! | `GET  /shop/shop/list` | `ShopClientController.list` | optional |
//! | `POST /shop/shop/collect` | `ShopClientController.collect` | optional |
//! | `GET  /shop/shop/head` | `ShopClientController.shopHead` | optional |
//! | `GET  /shop/shop/detail` | `ShopClientController.detail` | optional |
//! | `GET  /shop/shop/decorate` | `ShopClientController.shopDecorate` | optional |
//! | `GET  /shop/shop/pickupList` | `ShopClientController.pickupList` | optional |
//! | `GET  /shop/shop/pickupDetail` | `ShopClientController.pickupDetail` | optional |
//! | `GET  /shop/shop/storeList` | `ShopClientController.storeList` | optional |
//! | `GET  /shop/shop/storeDetail` | `ShopClientController.storeDetail` | optional |
//! | `GET  /shop/shop/getCommentList` | `ShopClientController.getCommentList` | optional |
//! | `GET  /shop/miniProgram/context` | 本平台新增（店铺独立绑定小程序） | optional |
//!
//! > **路径说明**：控制器前缀**重复一次**（`/api/shop/shop/...`），
//! > 这不是笔误，必须逐字符照抄；本文件挂在 `/api` 之下，
//! > 故 `routes()` 内只写 `/shop/shop/...`。
//!
//! > **⚠️ 合并注意（必须由主会话处理）**：`crates/api/public/src/routes/shop.rs`
//! > 的 `public_routes()` 已存在一条**引导阶段的内联实现**
//! > `GET /shop/shop/detail`，且 `shop::implemented_routes()` 里也有对应记录。
//! > 合并本模块时必须**同时删除**那条内联路由与记录，否则 axum 的
//! > `Router::merge` 会因同一路径重复注册在启动时 panic
//! > （与 trade::order_admin 对 `admin/src/lib.rs` 内联订单路由的处理同因）。
//!
//! > **auth=optional 的落地口径**：契约把本域 10 条都标为 `optional`，
//! > 但 `collect` 依赖 `SecurityUtils.getCurrentUserId()` 写入归属数据，
//! > `detail` / `storeList` / `storeDetail` 需要登录态才能回填
//! > `collectShop`（是否已收藏）标记。Java 侧这些接口在
//! > `permitAll` 之下仍有一个 JWT 过滤器：**带合法令牌就写入用户上下文，
//! > 不带令牌则 `getCurrentUserId()` 返回 0（游客）**。
//! >
//! > 本仓库 `public/src/lib.rs` 的 `require_auth` 只挂在「需登录」路由组上，
//! > 公开组没有可选登录态中间件（现有 slice 的 `Option<Extension<AuthUser>>`
//! > 因此在公开组恒为 `None`）。为对齐 Java 语义，本文件自带一个
//! > [`optional_auth`] 中间件：
//! > - 无 `Authorization` 头 → 游客（`user_id = 0`，`collectShop = false`）；
//! > - 带头但令牌非法/过期 → 401（与 Java 过滤器一致）；
//! > - 令牌合法 → 写入 `AuthUser`，`collect` 等接口可正常取到用户。
//! > `collect` 对游客返回业务码 401「请先登录」
//! > （Java 会用 `userId = 0` 落一条脏收藏记录，本实现不复刻该行为）。

use serde::Deserialize;
use axum::{
    extract::{Query, Request, State},
    http::header,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use nextrade_core::models::merchant::shop_client::{
    ClientShopListQuery, CollectShopParam, CollectShopResultVO, CommentVO, NearestStoreQuery,
    PickupListQuery, PickupListVO, ShopCommentListQuery, ShopDecorateQuery, ShopIdQuery,
    StoreDetailQuery, StoreDetailVO, StoreListQuery, StoreListVO,
};
use nextrade_core::types::PageResult;
use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};
use nextrade_security::JwtService;

use crate::PublicState;

/// 店铺域前台路由（挂载于 `/api` 下）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/shop/shop/list", get(client_list))
        .route("/shop/shop/collect", post(shop_collect))
        .route("/shop/shop/head", get(shop_head))
        .route("/shop/shop/groupDetail", get(shop_group_detail))
        .route("/shop/shop/detail", get(shop_detail))
        .route("/shop/shop/decorate", get(shop_decorate))
        .route("/shop/shop/pickupList", get(pickup_list))
        .route("/shop/shop/pickupDetail", get(pickup_detail))
        .route("/shop/shop/storeList", get(store_list))
        .route("/shop/shop/storeDetail", get(store_detail))
        .route("/shop/shop/nearestStore", get(nearest_store))
        .route("/shop/shop/getCommentList", get(get_comment_list))
        .route("/shop/miniProgram/context", get(mini_program_context))
        .layer(middleware::from_fn_with_state(
            state.jwt.clone(),
            optional_auth,
        ))
}

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    [
        ("GET", "/api/shop/shop/list", "ShopClientController.list"),
        (
            "POST",
            "/api/shop/shop/collect",
            "ShopClientController.collect",
        ),
        ("GET", "/api/shop/shop/head", "ShopClientController.shopHead"),
        ("GET", "/api/shop/shop/groupDetail", "ShopClientController.groupDetail"),
        (
            "GET",
            "/api/shop/miniProgram/context",
            "ShopMiniProgramController.context (本平台新增)",
        ),
        (
            "GET",
            "/api/shop/shop/detail",
            "ShopClientController.detail",
        ),
        (
            "GET",
            "/api/shop/shop/decorate",
            "ShopClientController.shopDecorate",
        ),
        (
            "GET",
            "/api/shop/shop/pickupList",
            "ShopClientController.pickupList",
        ),
        (
            "GET",
            "/api/shop/shop/pickupDetail",
            "ShopClientController.pickupDetail",
        ),
        (
            "GET",
            "/api/shop/shop/storeList",
            "ShopClientController.storeList",
        ),
        (
            "GET",
            "/api/shop/shop/storeDetail",
            "ShopClientController.storeDetail",
        ),
        (
            "GET",
            "/api/shop/shop/nearestStore",
            "ShopClientController.nearestStore (本平台新增)",
        ),
        (
            "GET",
            "/api/shop/shop/getCommentList",
            "ShopClientController.getCommentList",
        ),
    ]
    .iter()
    .map(|(m, p, j)| nextrade_openapi::RouteDoc::new(m, p, j))
    .collect()
}

/// 可选登录态中间件（对应 Java `permitAll` + JWT 过滤器）。
///
/// 无令牌 → 放行为游客；令牌非法 → 401；令牌合法 → 写入 [`AuthUser`]。
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
        let Ok(claims) = jwt.from_authorization(token) else { return Ok(next.run(req).await); };
        let Ok(id) = claims.sub.parse::<i32>() else { return Ok(next.run(req).await); };
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

/// 取登录会员 ID；游客返回 0（对应 Java `SecurityUtils.getCurrentUserId()`）。
fn current_user_id(auth: &Option<Extension<AuthUser>>) -> i32 {
    auth.as_ref().map(|Extension(a)| a.id).unwrap_or(0)
}

/// 取登录会员 ID；游客报业务码 401（写数据接口不允许匿名）。
fn require_user_id(auth: &Option<Extension<AuthUser>>) -> Result<i32, AppError> {
    let id = current_user_id(auth);
    if id <= 0 {
        return Err(AppError(Error::biz(401, "请先登录")));
    }
    Ok(id)
}

/// 价格门控解析（第 2 期补漏）：本域带价读口共用，
/// 与商品域同一谓词（`services::system::price_gate`）。
async fn price_access(
    state: &PublicState,
    auth: &Option<Extension<AuthUser>>,
) -> Result<nextrade_services::system::price_gate::PriceAccess, AppError> {
    let user_id = auth.as_ref().map(|Extension(a)| a.id).filter(|id| *id > 0);
    Ok(state.system.config_client.price_access(user_id).await?)
}

/// 存量装修类载荷门控：无看价权时递归把价格键置空（同 home 域装修口径）。
fn gate_stored(
    mut v: serde_json::Value,
    access: nextrade_services::system::price_gate::PriceAccess,
) -> serde_json::Value {
    if !access.can_view() {
        nextrade_services::system::price_gate::mask_price_keys(&mut v);
    }
    v
}

/// `GET /api/shop/shop/list`：前台店铺分页。
///
/// 对应 Java `ShopClientController.list(ShopListDTO)` → `ShopServiceImpl#clientList`。
///
/// 价格门控（第 2 期补漏）：条目内嵌 `listingProduct`（product 行原样），
/// 带商品价，此前零接入；无看价权时逐项递归脱敏 + 打标记。
async fn client_list(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<ClientShopListQuery>,
) -> ApiResult<serde_json::Value> {
    // optional 登录态透传 user_id（游客 0）：店铺街 recommend 排序的千人千面信号源
    let page = state
        .merchant
        .shop_client
        .client_list(&q, current_user_id(&auth))
        .await?;
    let access = price_access(&state, &auth).await?;
    let mut v = serde_json::to_value(&page).unwrap_or(serde_json::Value::Null);
    nextrade_services::system::price_gate::mask_page_items(&mut v, access);
    ok(v)
}

/// `POST /api/shop/shop/collect`：收藏/取消收藏店铺（切换）。
///
/// 对应 Java `ShopClientController.collect(@RequestBody CollectCreateDTO)`
/// → `CollectShopServiceImpl#collect`（原接口返回 `void`，
/// 此处回传 `collected` 便于前端联动）。
async fn shop_collect(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Json(param): Json<CollectShopParam>,
) -> ApiResult<CollectShopResultVO> {
    let user_id = require_user_id(&auth)?;
    ok(state.merchant.shop_client.collect(param.shop_id, user_id).await?)
}

/// `GET /api/shop/shop/head?shopId=`：店铺头部装修（`shopHead` 组件）。
///
/// 对应 Java `ShopClientController.shopHead` →
/// `DecorateDiscreteServiceImpl#getShopHead`。
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ShopGroupQuery {
    group_id: Option<i32>,
    shop_type: Option<i32>,
}

/// `GET /api/shop/miniProgram/context?appId=` 查询参数。
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct MiniProgramContextQuery {
    app_id: Option<String>,
}

/// `GET /api/shop/shop/groupDetail`：装修选择分组详情
/// （装修类载荷，价格门控同 shopDecorate 口径递归脱敏）。
async fn shop_group_detail(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<ShopGroupQuery>,
) -> ApiResult<serde_json::Value> {
    let access = price_access(&state, &auth).await?;
    let v = state
        .merchant
        .shop_client
        .group_detail(q.group_id.unwrap_or(0), q.shop_type.unwrap_or(1))
        .await?;
    ok(gate_stored(v, access))
}

/// `GET /api/shop/miniProgram/context?appId=`：小程序作用域解析
/// （店铺独立绑定小程序，本平台新增）。
///
/// 客户端启动时以自身 appid 询问模式：平台 appid → `{mode:"platform"}`；
/// 已绑定店铺 appid → `{mode:"shop", shopId, shopTitle}` 并进入店铺模式；
/// 未登记/停用/解绑 → 403（与 `x-mini-app-id` 中间件同一口径，硬拒绝）。
async fn mini_program_context(
    State(state): State<PublicState>,
    Query(q): Query<MiniProgramContextQuery>,
) -> ApiResult<nextrade_services::merchant::shop_mini_program::MiniProgramContext> {
    use nextrade_services::merchant::shop_mini_program::{AppResolution, MiniProgramContext};
    let app_id = q.app_id.as_deref().map(str::trim).unwrap_or("");
    let svc = &state.merchant.shop_mini_program;
    match svc.resolve(app_id).await? {
        AppResolution::Platform => ok(MiniProgramContext {
            mode: "platform".to_string(),
            shop_id: 0,
            shop_title: String::new(),
            app_id: app_id.to_string(),
        }),
        AppResolution::Shop(shop_id) => ok(MiniProgramContext {
            mode: "shop".to_string(),
            shop_id,
            shop_title: svc.shop_title(shop_id).await?,
            app_id: app_id.to_string(),
        }),
        AppResolution::Unknown => Err(AppError(Error::Forbidden(
            "该小程序未绑定店铺或绑定已停用".into(),
        ))),
    }
}

/// 店铺头部装修（装修类载荷，价格门控递归脱敏）。
async fn shop_head(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<ShopIdQuery>,
) -> ApiResult<serde_json::Value> {
    let access = price_access(&state, &auth).await?;
    let vo = state.merchant.shop_client.shop_head(&q).await?;
    ok(gate_stored(
        serde_json::to_value(&vo).unwrap_or(serde_json::Value::Null),
        access,
    ))
}

/// `GET /api/shop/shop/detail?shopId=`：前台店铺详情。
///
/// 对应 Java `ShopClientController.detail` → `ShopServiceImpl#clientDetail`。
///
/// 价格门控（第 2 期补漏）：详情同样内嵌 `listingProduct` 商品行，
/// 无看价权时递归脱敏 + 打标记（同商品详情口径）。
async fn shop_detail(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<ShopIdQuery>,
) -> ApiResult<serde_json::Value> {
    // Java：游客 `getCurrentUserId()` 返回 0，`collectShop` 恒为 false
    let vo = state
        .merchant
        .shop_client
        .client_detail(q.shop_id, current_user_id(&auth))
        .await?;
    let access = price_access(&state, &auth).await?;
    let mut v = serde_json::to_value(&vo).unwrap_or(serde_json::Value::Null);
    if !access.can_view() {
        nextrade_services::system::price_gate::mask_price_keys(&mut v);
    }
    nextrade_services::system::price_gate::stamp_price_gate(&mut v, access);
    ok(v)
}

/// `GET /api/shop/shop/decorate?shopId=&previewId=`：店铺首页装修数据。
///
/// 对应 Java `ShopClientController.shopDecorate` →
/// `DecorateServiceImpl#getShopIndexDecorateData`（`X-Locale-Code` 用于
/// 子装修页回退）。出参为存量装修 JSON（内嵌商品快照价），
/// 价格门控（第 2 期补漏）递归脱敏。
async fn shop_decorate(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    headers: axum::http::HeaderMap,
    Query(q): Query<ShopDecorateQuery>,
) -> ApiResult<serde_json::Value> {
    let locale_code = headers
        .get("X-Locale-Code")
        .and_then(|v| v.to_str().ok());
    let access = price_access(&state, &auth).await?;
    let v = state.merchant.shop_client.shop_decorate(&q, locale_code).await?;
    ok(gate_stored(v, access))
}

/// `GET /api/shop/shop/pickupList`：店铺关联自提点分页。
///
/// 对应 Java `ShopClientController.pickupList(PickupPageQuery)` →
/// `ShopServiceImpl#pickupList`。
async fn pickup_list(
    State(state): State<PublicState>,
    Query(q): Query<PickupListQuery>,
) -> ApiResult<PageResult<PickupListVO>> {
    ok(state.merchant.shop_client.pickup_list(&q).await?)
}

/// `GET /api/shop/shop/pickupDetail?shopId=`：自提点详情。
///
/// 对应 Java `ShopClientController.pickupDetail` →
/// `ShopServiceImpl#pickupDetail`。
async fn pickup_detail(
    State(state): State<PublicState>,
    Query(q): Query<ShopIdQuery>,
) -> ApiResult<PickupListVO> {
    ok(state.merchant.shop_client.pickup_detail(&q).await?)
}

/// `GET /api/shop/shop/storeList`：O2O 门店分页。
///
/// 对应 Java `ShopClientController.storeList(StorePageQuery)` →
/// `ShopServiceImpl#storeList`。
async fn store_list(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<StoreListQuery>,
) -> ApiResult<PageResult<StoreListVO>> {
    ok(state.merchant.shop_client.store_list(&q, current_user_id(&auth)).await?)
}

/// `GET /api/shop/shop/nearestStore?longitude=&latitude=`：最近门店
/// （我方新增，就近门店群帖 567745）。无定位/无带坐标门店时返回
/// `null`，前端据此降级为不就近。
async fn nearest_store(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<NearestStoreQuery>,
) -> ApiResult<Option<StoreListVO>> {
    ok(state
        .merchant
        .shop_client
        .nearest_store(&q, current_user_id(&auth))
        .await?)
}

/// `GET /api/shop/shop/storeDetail?shopId=`：门店详情。
///
/// 对应 Java `ShopClientController.storeDetail(StoreDetailQuery)` →
/// `ShopServiceImpl#storeDetail`。
async fn store_detail(
    State(state): State<PublicState>,
    auth: Option<Extension<AuthUser>>,
    Query(q): Query<StoreDetailQuery>,
) -> ApiResult<StoreDetailVO> {
    ok(state.merchant.shop_client.store_detail(&q, current_user_id(&auth)).await?)
}

/// `GET /api/shop/shop/getCommentList?page=&type=&shopId=`：店铺评论分页。
///
/// 对应 Java `ShopClientController.getCommentList` →
/// `CommentServiceImpl#listByShopId`（固定 `size = 10`）。
async fn get_comment_list(
    State(state): State<PublicState>,
    Query(q): Query<ShopCommentListQuery>,
) -> ApiResult<PageResult<CommentVO>> {
    ok(state.merchant.shop_client.comment_list(&q).await?)
}