//! 商城前台首页聚合路由（content::home，17 条：契约 16 + 5.8.57 新增 mobileCatNav）。
//!
//! 对应原 Java `com.nextrade.api.controller.home.HomeController`
//! （`@RequestMapping("/api/home/home")`）：
//!
//! | 本模块路由（挂载于 `/api`） | 原 Java 方法 | 契约 auth |
//! |-----------------------------|--------------|-----------|
//! | `GET /home/home/friendLinks` | `friendLinks` | optional |
//! | `GET /home/home/mobileNav` | `mobileNav` | optional |
//! | `GET /home/home/pcLogin` | `pcLogin` | anonymous |
//! | `GET /home/home/pcSearch` | `pcSearch` | optional |
//! | `GET /home/home/pcHeader` | `pcHeader` | optional |
//! | `GET /home/home/o2oIndex` | `o2oIndex` | optional |
//! | `GET /home/home/getCustomerServiceConfig` | `getConfigCustomer` | optional |
//! | `GET /home/home/getSeckill` | `getSeckill` | optional |
//! | `GET /home/home/getCoupon` | `getCoupon` | optional |
//! | `GET /home/home/memberDecorate` | `memberDecorate` | optional |
//! | `GET /home/home/h5Category` | `h5Category` | optional |
//! | `GET /home/home/h5ProductDetail` | `h5ProductDetail` | optional |
//! | `GET /home/home/h5Member` | `h5Member` | optional |
//! | `GET /home/home/h5Search` | `h5Search` | optional |
//! | `GET /home/home/mobileLogin` | `mobileLogin` | anonymous |
//! | `GET /home/home/getRecommend` | `getRecommend` | optional |
//!
//! > **路由分组**：15 条 `optional` 走 [`routes`]（模块自带 [`optional_auth`]，
//! > 无令牌放行、令牌非法 401）；`pcLogin` / `mobileLogin` 在 Java 属匿名白名单，
//! > 走 [`anonymous_routes`]（不套鉴权层）。`sync_routes.py` 会把两者都并入
//! > `merge_public`。
//!
//! > **`getSeckill` / `getCoupon` 委托**：Java 控制器注入 `SeckillService` /
//! > `CouponService`；本路由同样在 handler 内调用既有
//! > `state.marketing.seckill.active_list` / `state.marketing.coupon.claimable_list`
//! > （等价多服务注入），不在 `content/home` 重写营销查询。
//!
//! > **偏差**：`getSeckill` 的 `unStarted=1`（包含未开始场次）在 Rust
//! > `marketing::seckill::active_list` 中不支持，当前恒返回进行中场次；
//! > `getCoupon` 返回 `Vec<CouponClientVO>`（Java `Page#getRecords()` 同口径）。

use axum::{
    extract::{Query, Request, State},
    http::header,
    middleware::{self, Next},
    response::Response,
    routing::get,
    Router,
};
use nextrade_core::models::content::decorate::DecorateModuleListVO;
use nextrade_core::models::content::home::{
    ConfigCustomerVO, DecoratePreviewQuery, FriendLinkVO, MobileNavQuery, O2oDecorateVO,
    O2oIndexQuery, RecommendQuery, SeckillHomeQuery,
};
use nextrade_core::models::marketing::coupon::{CouponClientVO, CouponListQuery};
use nextrade_core::models::marketing::seckill::SeckillPromotionVO;
use nextrade_core::types::PageResult;
use nextrade_core::Error;
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult, AppError, AuthUser};
use nextrade_security::JwtService;

use crate::PublicState;

/// 本 feature 全部 17 条路由（`/api` 前缀之后的路径 + 原 Java 方法名）。
///
/// 末条 `mobileCatNav` 为 b2b2b 5.8.57 H5 前端新增（**无 Java 对应**，
/// 读管理端维护的 `decorate_mobile_cat_nav` 表输出，不在原契约内）。
const ROUTE_TABLE: [(&str, &str, &str); 17] = [
    ("GET", "/home/home/friendLinks", "HomeController.friendLinks"),
    ("GET", "/home/home/mobileNav", "HomeController.mobileNav"),
    ("GET", "/home/home/pcLogin", "HomeController.pcLogin"),
    ("GET", "/home/home/pcSearch", "HomeController.pcSearch"),
    ("GET", "/home/home/pcHeader", "HomeController.pcHeader"),
    ("GET", "/home/home/o2oIndex", "HomeController.o2oIndex"),
    (
        "GET",
        "/home/home/getCustomerServiceConfig",
        "HomeController.getConfigCustomer",
    ),
    ("GET", "/home/home/getSeckill", "HomeController.getSeckill"),
    ("GET", "/home/home/getCoupon", "HomeController.getCoupon"),
    (
        "GET",
        "/home/home/memberDecorate",
        "HomeController.memberDecorate",
    ),
    ("GET", "/home/home/h5Category", "HomeController.h5Category"),
    (
        "GET",
        "/home/home/h5ProductDetail",
        "HomeController.h5ProductDetail",
    ),
    ("GET", "/home/home/h5Member", "HomeController.h5Member"),
    ("GET", "/home/home/h5Search", "HomeController.h5Search"),
    ("GET", "/home/home/mobileLogin", "HomeController.mobileLogin"),
    ("GET", "/home/home/getRecommend", "HomeController.getRecommend"),
    ("GET", "/home/home/mobileCatNav", "HomeController.mobileCatNav"),
];

/// `optional` 路由（14 条，模块自带可选登录态中间件）。
pub fn routes(state: &PublicState) -> Router<PublicState> {
    Router::new()
        .route("/home/home/friendLinks", get(friend_links))
        .route("/home/home/mobileNav", get(mobile_nav))
        .route("/home/home/pcSearch", get(pc_search))
        .route("/home/home/pcHeader", get(pc_header))
        .route("/home/home/o2oIndex", get(o2o_index))
        .route(
            "/home/home/getCustomerServiceConfig",
            get(customer_service_config),
        )
        .route("/home/home/getSeckill", get(get_seckill))
        .route("/home/home/getCoupon", get(get_coupon))
        .route("/home/home/memberDecorate", get(member_decorate))
        .route("/home/home/h5Category", get(h5_category))
        .route("/home/home/h5ProductDetail", get(h5_product_detail))
        .route("/home/home/h5Member", get(h5_member))
        .route("/home/home/h5Search", get(h5_search))
        .route("/home/home/getRecommend", get(get_recommend))
        .route("/home/home/mobileCatNav", get(mobile_cat_nav))
        .layer(middleware::from_fn_with_state(
            state.jwt.clone(),
            optional_auth,
        ))
}

/// 匿名白名单路由（`pcLogin` / `mobileLogin`，不套鉴权层）。
pub fn anonymous_routes(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .route("/home/home/pcLogin", get(pc_login))
        .route("/home/home/mobileLogin", get(mobile_login))
}

/// 本域已实现路由（用于 `/openapi/coverage` 覆盖率统计，完整契约路径）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/api{path}"), handler))
        .collect()
}

/// 可选登录态中间件（对应 Java `permitAll` + JWT 过滤器）。
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

/// `GET /api/home/home/friendLinks`：友情链接。
async fn friend_links(State(state): State<PublicState>) -> ApiResult<Vec<FriendLinkVO>> {
    ok(state.content.home.friend_links().await?)
}

/// `GET /api/home/home/mobileNav`：移动端导航（新/旧版）。
async fn mobile_nav(
    State(state): State<PublicState>,
    Query(q): Query<MobileNavQuery>,
) -> ApiResult<serde_json::Value> {
    ok(state.content.home.mobile_nav(&q).await?)
}

/// `GET /api/home/home/pcLogin`：PC 登录页装修（匿名）。
async fn pc_login(
    State(state): State<PublicState>,
    Query(q): Query<DecoratePreviewQuery>,
) -> ApiResult<DecorateModuleListVO> {
    ok(state.content.home.pc_login(&q).await?)
}

/// `GET /api/home/home/pcSearch`：PC 搜索页装修。
async fn pc_search(
    State(state): State<PublicState>,
    Query(q): Query<DecoratePreviewQuery>,
) -> ApiResult<DecorateModuleListVO> {
    ok(state.content.home.pc_search(&q).await?)
}

/// `GET /api/home/home/pcHeader`：PC 头部装修配置。
async fn pc_header(
    State(state): State<PublicState>,
    Query(q): Query<DecoratePreviewQuery>,
) -> ApiResult<serde_json::Value> {
    ok(state.content.home.pc_header(&q).await?)
}

/// `GET /api/home/home/o2oIndex?shopId=`：门店首页装修。
async fn o2o_index(
    State(state): State<PublicState>,
    Query(q): Query<O2oIndexQuery>,
) -> ApiResult<O2oDecorateVO> {
    ok(state.content.home.o2o_index(&q).await?)
}

/// `GET /api/home/home/getCustomerServiceConfig`：客服设置。
async fn customer_service_config(
    State(state): State<PublicState>,
) -> ApiResult<ConfigCustomerVO> {
    ok(state.content.home.customer_service_config().await?)
}

/// `GET /api/home/home/getSeckill`：首页秒杀（委托 marketing 域）。
async fn get_seckill(
    State(state): State<PublicState>,
    Query(q): Query<SeckillHomeQuery>,
) -> ApiResult<PageResult<SeckillPromotionVO>> {
    let page = q.page.unwrap_or(1).max(1);
    let size = q.size.unwrap_or(15).clamp(1, 100);
    // Java `unStarted=1` 包含未开始场次；Rust `active_list` 仅返回进行中（偏差见模块头）
    if q.un_started.unwrap_or(0) == 1 {
        tracing::debug!("home getSeckill: unStarted 过滤未迁移，返回进行中场次");
    }
    ok(state
        .marketing
        .seckill
        .active_list(Some(page), Some(size))
        .await?)
}

/// `GET /api/home/home/getCoupon`：首页优惠券（委托 marketing 域）。
///
/// Java 固定 `isShow=1, validDate=1, receiveFlag=1` 后取分页 records；
/// Rust `claimable_list` 即「可领取」口径，返回列表部分。
async fn get_coupon(
    State(state): State<PublicState>,
    Query(mut q): Query<CouponListQuery>,
) -> ApiResult<Vec<CouponClientVO>> {
    q.is_show = Some(1);
    q.valid_date = Some(1);
    q.receive_flag = Some(1);
    let page = state.marketing.coupon.claimable_list(&q).await?;
    ok(page.list)
}

/// `GET /api/home/home/memberDecorate`：个人中心离散装修。
async fn member_decorate(
    State(state): State<PublicState>,
    Query(q): Query<nextrade_core::models::content::home::MemberDecorateQuery>,
) -> ApiResult<serde_json::Value> {
    ok(state
        .content
        .home
        .member_decorate(q.decorate_sn.as_deref())
        .await?)
}

/// `GET /api/home/home/h5Category`：H5 分类页装修。
async fn h5_category(
    State(state): State<PublicState>,
    Query(q): Query<DecoratePreviewQuery>,
) -> ApiResult<DecorateModuleListVO> {
    ok(state.content.home.h5_category(&q).await?)
}

/// `GET /api/home/home/h5ProductDetail`：H5 商品详情页装修。
async fn h5_product_detail(
    State(state): State<PublicState>,
    Query(q): Query<DecoratePreviewQuery>,
) -> ApiResult<DecorateModuleListVO> {
    ok(state.content.home.h5_product_detail(&q).await?)
}

/// `GET /api/home/home/h5Member`：H5 个人中心装修。
async fn h5_member(
    State(state): State<PublicState>,
    Query(q): Query<DecoratePreviewQuery>,
) -> ApiResult<DecorateModuleListVO> {
    ok(state.content.home.h5_member(&q).await?)
}

/// `GET /api/home/home/h5Search`：移动端搜索页装修。
async fn h5_search(
    State(state): State<PublicState>,
    Query(q): Query<DecoratePreviewQuery>,
) -> ApiResult<DecorateModuleListVO> {
    ok(state.content.home.h5_search(&q).await?)
}

/// `GET /api/home/home/mobileLogin`：移动端登录页装修（匿名）。
async fn mobile_login(
    State(state): State<PublicState>,
    Query(q): Query<DecoratePreviewQuery>,
) -> ApiResult<DecorateModuleListVO> {
    ok(state.content.home.mobile_login(&q).await?)
}

/// `GET /api/home/home/getRecommend`：首页推荐模块数据。
async fn get_recommend(
    State(state): State<PublicState>,
    Query(q): Query<RecommendQuery>,
) -> ApiResult<serde_json::Value> {
    ok(state.content.home.get_recommend(&q).await?)
}

/// `GET /api/home/home/mobileCatNav`：移动端首页分类导航。
///
/// b2b2b 5.8.57 H5 前端新增（无 Java 对应）：读管理端
/// `decorate_mobile_cat_nav` CRUD 维护的数据，只输出 `is_show = 1`
/// 的条目；表空返回空数组。
async fn mobile_cat_nav(
    State(state): State<PublicState>,
) -> ApiResult<Vec<nextrade_core::models::content::mobile_cat_nav::MobileCatNavVO>> {
    let q = nextrade_core::models::content::mobile_cat_nav::MobileCatNavListQuery {
        is_show: Some(1),
        keyword: None,
        sort_field: None,
        // 数字越大越靠前（表注释口径）→ 降序
        sort_order: Some("desc".into()),
        page: Some(1),
        size: Some(100),
    };
    let page = state.content.mobile_cat_nav.list(&q).await?;
    ok(page.list)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registered_pairs() -> Vec<(String, String)> {
        // routes() 注册的 15 条（把 `.route` 调用逐条复述，防止漏挂/错路径）
        let mut v: Vec<(&str, &str)> = vec![
            ("GET", "/home/home/friendLinks"),
            ("GET", "/home/home/mobileNav"),
            ("GET", "/home/home/pcSearch"),
            ("GET", "/home/home/pcHeader"),
            ("GET", "/home/home/o2oIndex"),
            ("GET", "/home/home/getCustomerServiceConfig"),
            ("GET", "/home/home/getSeckill"),
            ("GET", "/home/home/getCoupon"),
            ("GET", "/home/home/memberDecorate"),
            ("GET", "/home/home/h5Category"),
            ("GET", "/home/home/h5ProductDetail"),
            ("GET", "/home/home/h5Member"),
            ("GET", "/home/home/h5Search"),
            ("GET", "/home/home/getRecommend"),
            ("GET", "/home/home/mobileCatNav"),
        ];
        // anonymous_routes() 注册的 2 条
        v.push(("GET", "/home/home/pcLogin"));
        v.push(("GET", "/home/home/mobileLogin"));
        v.into_iter()
            .map(|(m, p)| (m.to_string(), p.to_string()))
            .collect()
    }

    #[test]
    fn implemented_routes_match_java_contract() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 17, "契约 16 条 + 5.8.57 新增 mobileCatNav");
        assert_eq!(docs.len(), ROUTE_TABLE.len());
        for (method, path, handler) in ROUTE_TABLE.iter() {
            let full = format!("/api{path}");
            assert!(
                docs.iter()
                    .any(|d| d.method == *method && d.path == full && d.java_ref == *handler),
                "缺少 {method} {full} ({handler})"
            );
        }
    }

    #[test]
    fn routes_and_anonymous_routes_cover_all_table_paths() {
        let registered = registered_pairs();
        assert_eq!(registered.len(), ROUTE_TABLE.len());
        for (method, path) in &registered {
            assert!(
                ROUTE_TABLE.iter().any(|(m, p, _)| *m == method.as_str() && *p == path.as_str()),
                "{method} {path} 不在 ROUTE_TABLE 内"
            );
        }
        for (method, path, _) in ROUTE_TABLE.iter() {
            assert!(
                registered.iter().any(|(m, p)| m == method && p == path),
                "{method} {path} 未在 routes()/anonymous_routes() 中注册"
            );
        }
    }

    #[test]
    fn anonymous_paths_are_the_two_login_pages() {
        let anon = [
            "/home/home/pcLogin",
            "/home/home/mobileLogin",
        ];
        // 这两条必须存在且不能在 optional 路由重复注册（重复会被 axum merge panic）
        for path in anon {
            assert!(
                ROUTE_TABLE.iter().any(|(m, p, _)| *m == "GET" && *p == path),
                "{path} 不在 ROUTE_TABLE"
            );
        }
        // ROUTE_TABLE 内路径不允许重复（同一 (method,path) 双注册 = merge panic）
        let mut seen: Vec<&str> = ROUTE_TABLE.iter().map(|(_, p, _)| *p).collect();
        seen.sort_unstable();
        let total = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), total, "ROUTE_TABLE 存在重复路径");
        // 匿名路由恰好两条登录页；其余全部走 optional 鉴权
        assert_eq!(anon.len(), 2);
    }
}
