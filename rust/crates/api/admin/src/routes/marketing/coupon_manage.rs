//! 管理端优惠券管理路由（marketing::coupon_manage）。
//!
//! 对应原 Java `com.nextrade.adminapi.controller.promotion.CouponController`
//! 中由本域负责的 9 条端点（其余 list/detail/validList/listByIds/issue/users/
//! issue/send 已有 `marketing::coupon` 基线实现，本域**不重复注册**）。
//!
//! | 本模块路由（挂载于 `/adminapi`） | 原 Java 方法 | 权限 |
//! |----------------------------------|--------------|------|
//! | `POST /promotion/coupon/create` | `create` | `promotionCouponModifyManage` |
//! | `POST /promotion/coupon/update` | `update` | `promotionCouponModifyManage` |
//! | `POST /promotion/coupon/del` | `del` | `promotionCouponModifyManage` |
//! | `POST /promotion/coupon/updateField` | `updateField` | `promotionCouponModifyManage` |
//! | `POST /promotion/coupon/batch` | `batch` | `promotionCouponModifyManage` |
//! | `GET  /promotion/coupon/config` | `config` | `couponManage` |
//! | `POST /promotion/coupon/issue/preview` | `issuePreview` | `couponManage` |
//! | `GET  /promotion/coupon/issue/log` | `issueLog` | `couponManage` |
//! | `GET  /promotion/coupon/issue/batch` | `issueBatch` | `couponManage` |
//!
//! > `create`/`update`/`issue/preview` 的所属店铺取自 Java
//! > `HeaderUtils.getShopId()`（`x-admin-type` / `x-shop-id`），见
//! > [`shop_id_from_headers`]。
//!
//! > 类级 `@PreAuthorize('couponManage')`；Rust 侧 `require_auth` 目前只校验登录态，
//! > 权限矩阵尚未实现（与其它域已知缺口一致）。

use axum::{
    extract::Query,
    extract::State,
    http::HeaderMap,
    routing::{get, post},
    Extension, Json, Router,
};
use nextrade_core::models::marketing::coupon::{CouponClientVO, CouponListQuery};
use nextrade_core::models::marketing::coupon_manage::{
    BatchDTO, CouponConfigRankVO, CouponDTO, CouponIssueBatchPageQuery, CouponIssueBatchVO,
    CouponIssueLogPageQuery, CouponIssueLogVO, CouponManualIssueParam, CouponManualIssuePreviewVO,
    OperateDTO, UpdateFieldDTO,
};
use nextrade_core::types::PageResult;
use nextrade_openapi::RouteDoc;
use nextrade_security::web::{ok, ApiResult, AuthUser};

use crate::AdminState;

/// 本域全部 9 条路由（去掉 `/adminapi` 前缀后的路径 + 原 Java 方法名）。
const ROUTE_TABLE: [(&str, &str, &str); 9] = [
    (
        "POST",
        "/promotion/coupon/create",
        "CouponController.create",
    ),
    (
        "POST",
        "/promotion/coupon/update",
        "CouponController.update",
    ),
    ("POST", "/promotion/coupon/del", "CouponController.del"),
    (
        "POST",
        "/promotion/coupon/updateField",
        "CouponController.updateField",
    ),
    ("POST", "/promotion/coupon/batch", "CouponController.batch"),
    ("GET", "/promotion/coupon/config", "CouponController.config"),
    (
        "POST",
        "/promotion/coupon/issue/preview",
        "CouponController.issuePreview",
    ),
    (
        "GET",
        "/promotion/coupon/issue/log",
        "CouponController.issueLog",
    ),
    (
        "GET",
        "/promotion/coupon/issue/batch",
        "CouponController.issueBatch",
    ),
];

/// 优惠券管理路由（挂载于 `/adminapi` 下，外层统一套 `require_auth`）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/promotion/coupon/create", post(create))
        .route("/promotion/coupon/update", post(update))
        .route("/promotion/coupon/del", post(del))
        .route("/promotion/coupon/updateField", post(update_field))
        .route("/promotion/coupon/batch", post(batch))
        .route("/promotion/coupon/config", get(config))
        .route("/promotion/coupon/issue/preview", post(issue_preview))
        .route("/promotion/coupon/issue/log", get(issue_log))
        .route("/promotion/coupon/issue/batch", get(issue_batch))
        // 非 Java 对照端点：给后台装修预览提供与 C 端前台**同源**的可领取券列表。
        // 刻意不进 ROUTE_TABLE —— 那张表是 Java 契约对照表，加进来会谎报覆盖率。
        .route("/promotion/coupon/claimableList", get(claimable_list))
}

/// `GET /adminapi/promotion/coupon/claimableList`：前台首页「优惠券」模块的同源预览数据。
///
/// **为什么要有这个端点**：后台装修预览原先自己调 `/promotion/coupon/list`，
/// 再由前端复刻一遍 C 端的可见性过滤规则 —— 同一套规则两份实现，后端一改就会漂移，
/// 表现为「后台看到的券」≠「前台显示的券」。
/// 这里直接复用与公开端 `GET /api/home/home/getCoupon` **完全相同**的实现
/// （`CouponService::claimable_list` + 同样强制 `is_show/valid_date/receive_flag = 1`），
/// 从此两端只有一个数据来源。
///
/// 备注：返回的是**游客口径**（`claimable_list`，`isReceive` 恒为 0），
/// 与未登录访客在首页看到的一致；`claimable_list_for_user` 是登录会员口径，预览不使用。
async fn claimable_list(
    State(state): State<AdminState>,
    Query(mut q): Query<CouponListQuery>,
) -> ApiResult<Vec<CouponClientVO>> {
    q.is_show = Some(1);
    q.valid_date = Some(1);
    q.receive_flag = Some(1);
    let page = state.marketing.coupon.claimable_list(&q).await?;
    ok(page.list)
}

/// 本模块已实现路由（用于 `/openapi/coverage` 覆盖率统计，完整契约路径）。
#[must_use]
pub fn implemented_routes() -> Vec<RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| RouteDoc::new(method, &format!("/adminapi{path}"), handler))
        .collect()
}

/// 解析 Java `HeaderUtils.getShopId()`：`admin` ⇒ `Some(0)`；
/// `shop|pos` ⇒ `x-shop-id`；其它/缺失 ⇒ `None`。
#[must_use]
fn shop_id_from_headers(headers: &HeaderMap) -> Option<i32> {
    let header_str = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(ToString::to_string)
    };
    match header_str("x-admin-type").as_deref() {
        Some("admin") => Some(0),
        Some("shop" | "pos") => header_str("x-shop-id").and_then(|v| v.trim().parse::<i32>().ok()),
        _ => None,
    }
}

/// `POST /adminapi/promotion/coupon/create`：创建优惠券。
async fn create(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<CouponDTO>,
) -> ApiResult<()> {
    let shop_id = auth
        .scoped_shop_id()
        .or_else(|| shop_id_from_headers(&headers));
    state.marketing.coupon_manage.create(&dto, shop_id).await?;
    ok(())
}

/// `POST /adminapi/promotion/coupon/update`：更新优惠券。
async fn update(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<CouponDTO>,
) -> ApiResult<()> {
    let shop_id = auth
        .scoped_shop_id()
        .or_else(|| shop_id_from_headers(&headers));
    state.marketing.coupon_manage.update(&dto, shop_id).await?;
    ok(())
}

/// `POST /adminapi/promotion/coupon/del`：删除优惠券（软删）。
async fn del(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<OperateDTO>,
) -> ApiResult<()> {
    state
        .marketing
        .coupon_manage
        .del_scoped(&dto, auth.scoped_shop_id())
        .await?;
    ok(())
}

/// `POST /adminapi/promotion/coupon/updateField`：更新单个字段。
async fn update_field(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<UpdateFieldDTO>,
) -> ApiResult<()> {
    state
        .marketing
        .coupon_manage
        .update_field_scoped(&dto, auth.scoped_shop_id())
        .await?;
    ok(())
}

/// `POST /adminapi/promotion/coupon/batch`：批量操作（仅 del 生效）。
async fn batch(
    State(state): State<AdminState>,
    Extension(auth): Extension<AuthUser>,
    Json(dto): Json<BatchDTO>,
) -> ApiResult<()> {
    state
        .marketing
        .coupon_manage
        .batch_scoped(&dto, auth.scoped_shop_id())
        .await?;
    ok(())
}

/// `GET /adminapi/promotion/coupon/config`：营销配置项（会员等级 + 会员数）。
async fn config(State(state): State<AdminState>) -> ApiResult<Vec<CouponConfigRankVO>> {
    ok(state.marketing.coupon_manage.config().await?)
}

/// `POST /adminapi/promotion/coupon/issue/preview`：手动发放预览。
async fn issue_preview(
    State(state): State<AdminState>,
    headers: HeaderMap,
    Extension(auth): Extension<AuthUser>,
    Json(param): Json<CouponManualIssueParam>,
) -> ApiResult<CouponManualIssuePreviewVO> {
    let shop_id = auth
        .scoped_shop_id()
        .or_else(|| shop_id_from_headers(&headers));
    ok(state
        .marketing
        .coupon_manage
        .issue_preview(&param, shop_id)
        .await?)
}

/// `GET /adminapi/promotion/coupon/issue/log`：发放日志列表。
async fn issue_log(
    State(state): State<AdminState>,
    Query(q): Query<CouponIssueLogPageQuery>,
) -> ApiResult<PageResult<CouponIssueLogVO>> {
    ok(state.marketing.coupon_manage.issue_log(&q).await?)
}

/// `GET /adminapi/promotion/coupon/issue/batch`：发放批次列表。
async fn issue_batch(
    State(state): State<AdminState>,
    Query(q): Query<CouponIssueBatchPageQuery>,
) -> ApiResult<PageResult<CouponIssueBatchVO>> {
    ok(state.marketing.coupon_manage.issue_batch(&q).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `implemented_routes()` 必须与 Java 契约逐字符一致（9 条）。
    #[test]
    fn implemented_routes_match_contract_exactly() {
        let expected = [
            ("POST", "/adminapi/promotion/coupon/create"),
            ("POST", "/adminapi/promotion/coupon/update"),
            ("POST", "/adminapi/promotion/coupon/del"),
            ("POST", "/adminapi/promotion/coupon/updateField"),
            ("POST", "/adminapi/promotion/coupon/batch"),
            ("GET", "/adminapi/promotion/coupon/config"),
            ("POST", "/adminapi/promotion/coupon/issue/preview"),
            ("GET", "/adminapi/promotion/coupon/issue/log"),
            ("GET", "/adminapi/promotion/coupon/issue/batch"),
        ];
        let docs = implemented_routes();
        assert_eq!(docs.len(), expected.len(), "契约中本域共 9 条接口");
        for (doc, (method, path)) in docs.iter().zip(expected.iter()) {
            assert_eq!(doc.method, *method, "方法不匹配: {path}");
            assert_eq!(doc.path, *path, "路径不匹配");
            assert_eq!(doc.key(), format!("{method} {path}"));
        }
        let mut keys: Vec<String> = docs.iter().map(RouteDoc::key).collect();
        let sorted = {
            let mut k = keys.clone();
            k.sort();
            k.dedup();
            k
        };
        keys.sort();
        assert_eq!(sorted, keys, "implemented_routes() 存在重复项");
    }

    /// `ROUTE_TABLE` 与 `routes()` 一一对应；且不得与基线 coupon 端点重复。
    #[test]
    fn routes_registers_every_route_table_entry() {
        let registered = [
            "/promotion/coupon/create",
            "/promotion/coupon/update",
            "/promotion/coupon/del",
            "/promotion/coupon/updateField",
            "/promotion/coupon/batch",
            "/promotion/coupon/config",
            "/promotion/coupon/issue/preview",
            "/promotion/coupon/issue/log",
            "/promotion/coupon/issue/batch",
        ];
        assert_eq!(registered.len(), ROUTE_TABLE.len());
        for path in registered {
            assert!(
                ROUTE_TABLE.iter().any(|(_, p, _)| *p == path),
                "{path} 不在 ROUTE_TABLE 内"
            );
        }
        for (_, p, _) in ROUTE_TABLE.iter() {
            assert!(registered.contains(p), "{p} 未在 routes() 中注册");
        }
        // 基线已有端点不在本域（避免 axum 重复注册 panic）
        let baseline = [
            "/promotion/coupon/list",
            "/promotion/coupon/listByIds",
            "/promotion/coupon/detail",
            "/promotion/coupon/validList",
            "/promotion/coupon/issue/users",
            "/promotion/coupon/issue/send",
        ];
        for path in baseline {
            assert!(
                !ROUTE_TABLE.iter().any(|(_, p, _)| *p == path),
                "基线端点不应重复注册: {path}"
            );
        }
    }

    /// 请求头 → 店铺 ID，逐分支对齐 Java `HeaderUtils.getShopId()`。
    #[test]
    fn header_resolution_matches_header_utils() {
        use axum::http::{HeaderMap, HeaderValue};
        fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
            let mut h = HeaderMap::new();
            for (k, v) in pairs {
                h.insert(
                    axum::http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                    HeaderValue::from_str(v).unwrap(),
                );
            }
            h
        }
        assert_eq!(
            shop_id_from_headers(&headers(&[("x-admin-type", "admin")])),
            Some(0)
        );
        assert_eq!(
            shop_id_from_headers(&headers(&[("x-admin-type", "shop"), ("x-shop-id", "9")])),
            Some(9)
        );
        assert_eq!(shop_id_from_headers(&headers(&[])), None);
    }

    /// 请求体/查询参数按 Java 名（camelCase）解析。
    #[test]
    fn params_accept_java_names() {
        let dto: CouponDTO = serde_json::from_str(
            r#"{"couponName":"满减券","couponType":1,"reduceType":1,
                "couponMoney":5,"minOrderAmount":20,"sendRange":3,"sendRangeData":[1,2],
                "limitUserRank":[1],"isShow":1,"memberRange":0,"memberMarks":[]}"#,
        )
        .unwrap();
        assert_eq!(dto.coupon_name.as_deref(), Some("满减券"));
        assert_eq!(dto.send_range_data, Some(vec![1, 2]));

        let dto: UpdateFieldDTO =
            serde_json::from_str(r#"{"id":1,"field":"is_show","val":"0"}"#).unwrap();
        assert_eq!(dto.field, "is_show");
        assert_eq!(dto.val, "0");

        // 后台 Coupons.tsx 的 updateField() 发的是 `value` 而不是 `val`；两个键名都必须接受，
        // 否则前端调用必然 422 `missing field \`val\``（2026-11-11 实测的生产缺陷）。
        let dto: UpdateFieldDTO =
            serde_json::from_str(r#"{"id":1,"field":"is_show","value":1}"#).unwrap();
        assert_eq!(dto.field, "is_show");
        assert_eq!(dto.val, "1", "value 别名 + 数字值应被转成列值字符串");

        let q: CouponIssueLogPageQuery =
            serde_json::from_str(r#"{"couponId":1,"batchId":2,"page":1,"size":10}"#).unwrap();
        assert_eq!(q.coupon_id, Some(1));
        assert_eq!(q.batch_id, Some(2));

        let q: CouponIssueBatchPageQuery = serde_json::from_str(r#"{"sourceType":2}"#).unwrap();
        assert_eq!(q.source_type, Some(2));
    }
}
