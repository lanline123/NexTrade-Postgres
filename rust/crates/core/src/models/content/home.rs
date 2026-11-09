//! 首页聚合（`content/home`）DTO / VO / 查询参数。
//!
//! 对应原 Java：
//! - `com.nextrade.api.controller.home.HomeController`（`/api/home/home`，16 条）
//! - `com.nextrade.bean.model.setting.FriendLinks`（友情链接）
//! - `com.nextrade.bean.dto.setting.ConfigCustomerDTO`（客服设置）
//! - `com.nextrade.bean.vo.o2o.O2oDecorateVO`（门店首页装修）
//! - `com.nextrade.bean.vo.decorate.DecorateModuleListVO`（复用 `models::content::decorate`）
//!
//! HomeController 是**聚合控制器**：它把友情链接、装修（Decorate/DecorateDiscrete）、
//! 配置、秒杀、优惠券等多个域的数据拼给前台首页。Rust 侧：
//! - 装修 / 友情链接 / 客服配置 / 门店首页 / 推荐模块在本 feature 内实现；
//! - `getSeckill` / `getCoupon` 在路由层直接委托既有 `marketing` 域服务
//!   （与 Java 控制器的多服务注入一致），不在本文件建模。

use serde::{Deserialize, Serialize};

/// 装修类型：H5（Java `DecorateType.H5`）。
pub const DECORATE_TYPE_H5: i32 = 1;
/// 装修类型：PC（Java `DecorateType.PC`）。
pub const DECORATE_TYPE_PC: i32 = 2;

/// 页面类型：商城首页（Java `DecoratePageType.HOME`）。
pub const PAGE_HOME: i32 = 1;
/// 页面类型：个人中心（`MEMBER`）。
pub const PAGE_MEMBER: i32 = 2;
/// 页面类型：分类页（`CATEGORY`）。
pub const PAGE_CATEGORY: i32 = 4;
/// 页面类型：登录页（`LOGIN`）。
pub const PAGE_LOGIN: i32 = 5;
/// 页面类型：搜索页（`SEARCH`）。
pub const PAGE_SEARCH: i32 = 6;
/// 页面类型：导航页（`NAVIGATION`）。
pub const PAGE_NAVIGATION: i32 = 7;
/// 页面类型：H5 商品详情页（`H5_PRODUCT_DETAIL`）。
pub const PAGE_H5_PRODUCT_DETAIL: i32 = 8;
/// 页面类型：平台 O2O 门店首页（`O2O_PLATFORM_HOME`）。
pub const PAGE_O2O_PLATFORM_HOME: i32 = 9;

/// 友情链接（对应 Java `FriendLinks` 实体；实体的 serde 为 snake_case，
/// 前台契约要求 camelCase，故单独建模）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FriendLinkVO {
    pub link_id: i32,
    pub link_title: String,
    pub link_logo: String,
    pub link_url: String,
    pub sort_order: i32,
}

/// 客服设置（对应 Java `ConfigCustomerDTO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ConfigCustomerVO {
    pub h5_domain: Option<String>,
    pub corp_id: Option<String>,
    pub url: Option<String>,
    pub open_type: Option<i32>,
    pub service_type: Option<i32>,
    pub show: Option<i32>,
}

/// 门店首页装修（对应 Java `O2oDecorateVO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct O2oDecorateVO {
    pub data: Option<serde_json::Value>,
}

/// `GET /api/home/home/mobileNav` 查询参数。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MobileNavQuery {
    /// 离散装修 SN（缺省 `mobileNav`）
    pub decorate_sn: Option<String>,
    /// 1 = 读取新版 H5 导航装修页；0 = 读取旧版离散装修
    pub is_new: Option<i32>,
}

/// 装修预览/页面查询参数（pcLogin/pcSearch/pcHeader/h5* /mobileLogin 共用）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DecoratePreviewQuery {
    pub preview_id: Option<i32>,
    pub id: Option<i32>,
    pub decorate_type: Option<i32>,
}

/// 门店首页查询参数（`GET /api/home/home/o2oIndex?shopId=`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct O2oIndexQuery {
    pub shop_id: Option<i32>,
}

/// 会员中心离散装修查询（`GET /api/home/home/memberDecorate?decorateSn=`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MemberDecorateQuery {
    pub decorate_sn: Option<String>,
}

/// 首页推荐模块数据查询（`GET /api/home/home/getRecommend`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RecommendQuery {
    /// 装修页 ID；0 时按首页发布版查找
    pub decorate_id: Option<i32>,
    /// 模块索引（数字下标或模块 key）
    pub module_index: Option<String>,
    pub page: Option<u64>,
    pub preview_id: Option<i32>,
}

/// 首页秒杀查询（`GET /api/home/home/getSeckill`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SeckillHomeQuery {
    /// 每页条数（Java 缺省 15）
    pub size: Option<u64>,
    /// 页码（Java 缺省 1）
    pub page: Option<u64>,
    /// 1 = 包含未开始场次（Rust `marketing::seckill` 仅返回进行中，见路由层偏差）
    pub un_started: Option<i32>,
}

/// 门店首页 pageType（Java `Decorate.resolveO2oHomePageType`）：
/// `shopId` 为空/0 → 平台 O2O 首页（9），否则 → 商城首页（1）。
#[must_use]
pub fn resolve_o2o_page_type(shop_id: Option<i32>) -> i32 {
    match shop_id {
        None | Some(0) => PAGE_O2O_PLATFORM_HOME,
        Some(_) => PAGE_HOME,
    }
}

/// 装修模块是否展示（兼容 `isShow` 布尔 / 数字与 `is_show`；缺省展示）。
#[must_use]
pub fn module_visible(m: &serde_json::Value) -> bool {
    let fallback = serde_json::Value::Bool(true);
    let flag = m
        .get("isShow")
        .or_else(|| m.get("is_show"))
        .unwrap_or(&fallback);
    match flag {
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_i64() != Some(0),
        _ => true,
    }
}

/// 离散装修 SN 缺省值（Java `@RequestParam(defaultValue = ...)`）。
#[must_use]
pub fn decorate_sn_or(raw: Option<&str>, default: &str) -> String {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(default)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o2o_page_type_matches_java() {
        assert_eq!(resolve_o2o_page_type(None), 9);
        assert_eq!(resolve_o2o_page_type(Some(0)), 9);
        assert_eq!(resolve_o2o_page_type(Some(3)), 1);
    }

    #[test]
    fn module_visibility_accepts_bool_number_and_default() {
        assert!(!module_visible(&serde_json::json!({"isShow": false})));
        assert!(!module_visible(&serde_json::json!({"isShow": 0})));
        assert!(!module_visible(&serde_json::json!({"is_show": 0})));
        assert!(module_visible(&serde_json::json!({"isShow": true})));
        assert!(module_visible(&serde_json::json!({"isShow": 1})));
        assert!(module_visible(&serde_json::json!({})));
    }

    #[test]
    fn decorate_sn_defaults_when_blank() {
        assert_eq!(decorate_sn_or(None, "mobileNav"), "mobileNav");
        assert_eq!(decorate_sn_or(Some("  "), "mobileNav"), "mobileNav");
        assert_eq!(decorate_sn_or(Some(" custom "), "mobileNav"), "custom");
    }
}
