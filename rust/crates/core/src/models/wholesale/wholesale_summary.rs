//! 批发汇总域（wholesale::wholesale_summary）DTO / VO / 纯逻辑。
//!
//! **我方新增**（论坛帖 127889 评估 F3「总部后台看所有门店批发汇总」+
//! 门店级批发清单）：原版 v5.8.61 批发域无任何汇总/统计端点（快照仓
//! 批发域全域无 summary/statistics 落点；帖子评论区官方口头确认的汇总
//! 视图属其 O2O 产品线口径）。本模块为二期补齐：
//!
//! | method | path（挂载于 `/adminapi`） | 返回 |
//! |--------|---------------------------|------|
//! | `GET` | `/wholesale/summary/overview` | [`WholesaleSummaryOverviewVO`] |
//! | `GET` | `/wholesale/summary/shopPage` | `PageResult<WholesaleShopSummaryVO>` |
//! | `GET` | `/wholesale/summary/productPage` | `PageResult<WholesaleShopProductVO>` |
//!
//! # 统计口径（与现行口径对齐，不另造体系）
//!
//! - 成交源：`order_item.is_wholesale_line = 1` 的批发行（加车时打标、
//!   下单落行的一期既有标记），按明细行 `shop_id` 归属门店（订单头
//!   `shop_id` 在跨店单下只是首店口径，逐行归属才准）。
//! - 有效订单：订单头 `is_del = 0 AND pay_status = 2 AND
//!   order_status <> 3`（已支付且未取消，与订单记账链
//!   `services::trade::order` 的入账过滤同口径；金额 =
//!   `SUM(明细成交单价 price × quantity)`，与结算成交价一致）。
//! - 时间范围按订单头 `add_time`（下单时间，epoch 秒）过滤，与销售
//!   统计（analytics::sales_statistics）的 `add_time` 区间口径一致；
//!   缺省 = 全部时间。
//! - 「已配置批发商品数」= `product_wholesale_config` 中 `status = 1`
//!   且 `is_delete = 0` 的配置数（启用中的有效批发商品）。
//! - 门店批发有效开关 = 全局 `wholesaleEnabled` AND 店级
//!   `wholesale_shop_setting`（未配置默认开、shop 0 只看全局），复用
//!   [`super::wholesale_setting::effective_shop_wholesale`] 判定，不另写。
//!
//! # 权限口径
//!
//! - 平台账号（作用域 shop_id = 0）：见全平台，可按 `shopId` 缩到单店。
//! - 商家账号（作用域 shop_id > 0）：强制只见本店，`shopId` 参数忽略。
//! - 缺作用域的店铺账号（作用域哨兵 -1）：fail-closed，恒空结果。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::wholesale_product::WholesaleProductVO;

/// 平台作用域 shop_id（平台直营 / 平台账号）。
pub const PLATFORM_SHOP_ID: i32 = 0;

/// 订单状态：已取消（与 `services::trade::order::status::CANCELED` 同值）。
pub const ORDER_STATUS_CANCELED: i32 = 3;

/// 支付状态：已支付（与 `models::payment::pay::PAY_STATUS_PAID` 同值）。
pub const PAY_STATUS_PAID: i32 = 2;

/// 汇总查询（overview / shopPage 共用）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WholesaleSummaryQuery {
    /// 开始时间（下单时间，epoch 秒；缺省 = 不限）
    pub start_time: Option<i64>,
    /// 结束时间（下单时间，epoch 秒；缺省 = 不限）
    pub end_time: Option<i64>,
    /// 平台可指定单店；商家账号忽略（强制本店）
    pub shop_id: Option<i32>,
    /// 页码（shopPage 用）
    pub page: Option<u64>,
    /// 每页条数（shopPage 用）
    #[serde(rename = "pageSize", alias = "size")]
    pub page_size: Option<u64>,
}

/// 门店级批发清单查询（productPage 用）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WholesaleShopProductPageQuery {
    pub page: Option<u64>,
    #[serde(rename = "pageSize", alias = "size")]
    pub page_size: Option<u64>,
    /// 商品名/货号模糊
    pub keyword: Option<String>,
    /// 起批维度筛选：1=SPU 2=SKU
    pub moq_dimension: Option<i32>,
    /// 平台可指定单店；商家账号忽略（强制本店）
    pub shop_id: Option<i32>,
}

/// 汇总总览（overview 返回）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WholesaleSummaryOverviewVO {
    /// 作用域：`None` = 全平台（仅平台账号可得）；`Some(shop_id)` = 单店
    pub scope_shop_id: Option<i32>,
    /// 区间内有批发成交的门店数（平台口径；单店口径为 0/1）
    pub shop_count: i64,
    /// 启用中的已配置批发商品数（当前时点，不随时间范围变化）
    pub configured_product_count: i64,
    /// 启用批发商总数（买家全局身份，仅平台口径返回；单店口径为 `None`）
    pub wholesaler_count: Option<i64>,
    /// 批发订单数（含批发行的有效订单去重计数）
    pub order_count: i64,
    /// 批发行数
    pub line_count: i64,
    /// 批发件数（SUM quantity）
    pub total_quantity: i64,
    /// 批发成交金额（SUM 成交单价 × 数量）
    pub total_amount: Decimal,
    /// 批发买家数（去重 user_id）
    pub buyer_count: i64,
}

/// 按门店汇总行（shopPage 返回）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WholesaleShopSummaryVO {
    pub shop_id: i32,
    /// 门店名（shop.shop_title；平台 shop 0 显示「平台」）
    pub shop_title: String,
    /// 本店批发有效开关（全局 AND 店级，见模块头口径）
    pub wholesale_enabled: bool,
    /// 启用中的已配置批发商品数
    pub configured_product_count: i64,
    /// 批发订单数
    pub order_count: i64,
    /// 批发买家数
    pub buyer_count: i64,
    /// 批发件数
    pub total_quantity: i64,
    /// 批发成交金额
    pub total_amount: Decimal,
}

/// 门店级批发清单行（productPage 返回）：批发商品 VO + 归属门店。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WholesaleShopProductVO {
    pub shop_id: i32,
    /// 门店名（shop.shop_title；平台 shop 0 显示「平台」）
    pub shop_title: String,
    /// 批发商品行（与批发商品列表 `/wholesale/product/page` 同结构同口径）
    #[serde(flatten)]
    pub product: WholesaleProductVO,
}

/// 汇总作用域裁决（纯函数）：返回 `None` = 全平台（仅平台账号），
/// `Some(shop_id)` = 限定单店。
///
/// - 商家（caller > 0）：恒本店，忽略请求参数；
/// - 平台（caller = 0）：按请求 `shopId` 缩到单店（`shopId = 0` = 只看
///   平台直营），缺省（不传参）才是全平台；
/// - 缺作用域哨兵（caller < 0）：恒 `Some(-1)`，查询命中空集（fail-closed，
///   与批发商品路由层既有约定一致）。
#[must_use]
pub fn resolve_summary_scope(caller_shop_id: i32, requested: Option<i32>) -> Option<i32> {
    if caller_shop_id < 0 {
        return Some(caller_shop_id);
    }
    if caller_shop_id > 0 {
        return Some(caller_shop_id);
    }
    requested.filter(|id| *id >= 0)
}

/// 门店显示名：shop 0 恒「平台」，其余取 `shop.shop_title`，缺失回退空串由
/// 调用方处理。
#[must_use]
pub fn shop_display_title(shop_id: i32, title: Option<&str>) -> String {
    if shop_id == PLATFORM_SHOP_ID {
        return "平台".to_string();
    }
    title.unwrap_or_default().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_platform_defaults_to_all() {
        assert_eq!(resolve_summary_scope(0, None), None);
        assert_eq!(resolve_summary_scope(0, Some(-3)), None);
    }

    #[test]
    fn scope_platform_zero_means_platform_shop_only() {
        assert_eq!(resolve_summary_scope(0, Some(0)), Some(0));
    }

    #[test]
    fn scope_platform_can_narrow_to_one_shop() {
        assert_eq!(resolve_summary_scope(0, Some(7)), Some(7));
    }

    #[test]
    fn scope_merchant_forced_to_own_shop() {
        assert_eq!(resolve_summary_scope(5, None), Some(5));
        assert_eq!(resolve_summary_scope(5, Some(9)), Some(5));
        assert_eq!(resolve_summary_scope(5, Some(0)), Some(5));
    }

    #[test]
    fn scope_missing_scope_is_fail_closed() {
        assert_eq!(resolve_summary_scope(-1, None), Some(-1));
        assert_eq!(resolve_summary_scope(-1, Some(3)), Some(-1));
    }

    #[test]
    fn shop_title_platform_and_fallback() {
        assert_eq!(shop_display_title(0, Some("某店")), "平台");
        assert_eq!(shop_display_title(3, Some("三号店")), "三号店");
        assert_eq!(shop_display_title(3, None), "");
    }

    #[test]
    fn overview_vo_serializes_camel_case() {
        let vo = WholesaleSummaryOverviewVO {
            scope_shop_id: None,
            shop_count: 2,
            total_amount: Decimal::new(12345, 2),
            ..Default::default()
        };
        let v = serde_json::to_value(&vo).unwrap();
        assert_eq!(v["scopeShopId"], serde_json::Value::Null);
        assert_eq!(v["shopCount"], 2);
        assert_eq!(v["totalAmount"], "123.45");
    }
}
