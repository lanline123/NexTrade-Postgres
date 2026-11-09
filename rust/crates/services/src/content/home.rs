//! 首页聚合服务（`content/home`，16 条前台接口）。
//!
//! 对应原 Java `com.nextrade.api.controller.home.HomeController`：
//! 它是聚合控制器，注入 `FriendLinksService` / `DecorateService` /
//! `DecorateDiscreteService` / `ConfigService` / `SeckillService` / `CouponService`。
//!
//! Rust 侧边界：
//! - **本服务**：友情链接、装修页（H5/PC 首页、登录页、搜索页、分类页、
//!   商品详情页、个人中心、导航页）、离散装修（mobileNav/memberDecorate）、
//!   客服配置、门店首页、推荐模块数据；
//! - **路由层委托既有域服务**：`getSeckill` → `marketing::seckill::active_list`，
//!   `getCoupon` → `marketing::coupon::claimable_list`（与 Java 多服务注入一致）。
//!
//! # 偏差 / 未迁移（诚实声明）
//!
//! 1. **装修模块数据展开**：Java `DecorateServiceImpl#getDecorateModuleData` /
//!    `getPreviewDecorateModuleData` 会按模块类型装配商品/优惠券/文章等数据
//!    （`params{page,size}`）；本实现返回**装修页里保存的模块配置 JSON**
//!    （`getRecommend`），不做模块级数据展开 —— 需要各域配合才能补齐。
//! 2. **内置默认装修数据**：Java 用 `InitDecorateDataConstants`（H5 导航
//!    `NAV_H5_DECORATE_DATA` / O2O 首页 `O2O_H5_DECORATE_DATA`）兜底；这些
//!    大段内置 JSON 模板未迁移，本实现返回 `{"moduleList":[]}` 并记录日志。
//! 3. **O2O 默认装修落库**：Java 在门店无装修时 `decorateService.save(defaultO2OH5)`
//!    自动建一条默认装修；Rust 侧**不写库**，只返回空模块列表（避免写入
//!    未迁移的默认模板）。
//! 4. **PC 头部配置**：Java `getPcHeader(previewId)` 返回 PC 装修头部的结构化
//!    配置；本实现从装修数据里取 `pcHeader` / `pageModule.header` / 首个
//!    `type=header` 模块，取不到返回空对象。
//! 5. **多租户**：Java 首页装修固定 `shop_id = 0`（平台）与离散装修
//!    `shop_id = 0`；门店首页走显式 `shopId` 参数，Rust 与之一致。
//! 6. `translatePackage` / 多语言 `locale_id` 未迁移：装修查询统一 `locale_id = 0`。

use std::sync::Arc;

use serde_json::{json, Value};

use nextrade_core::entities::{Decorate, FriendLinks};
use nextrade_core::models::content::decorate::{parse_json_opt, DecorateModuleListVO};
use nextrade_core::models::content::home::{
    decorate_sn_or, module_visible, resolve_o2o_page_type, ConfigCustomerVO, DecoratePreviewQuery,
    FriendLinkVO, MobileNavQuery, O2oDecorateVO, O2oIndexQuery, RecommendQuery, DECORATE_TYPE_H5,
    DECORATE_TYPE_PC, PAGE_CATEGORY, PAGE_H5_PRODUCT_DETAIL, PAGE_HOME, PAGE_LOGIN, PAGE_MEMBER,
    PAGE_NAVIGATION, PAGE_SEARCH,
};
use nextrade_core::Result;
use nextrade_infrastructure::db::Repositories;

/// 平台店铺 ID（Java 首页装修固定 `shop_id = 0`）。
const PLATFORM_SHOP_ID: i32 = 0;
/// 已发布状态（Java `Decorate.status = 1`）。
const DECORATE_PUBLISHED: i32 = 1;

/// 底部导航（pageType=7）默认数据（对齐 Java InitDecorateDataConstants.NAV_H5_DECORATE_DATA）
pub const DEFAULT_NAV_H5_DECORATE_DATA: &str = r#"{
    "navList": [
        {
            "picTitle": "首页",
            "picThumb": "https://oss.vortmall.com/demo/upload/c4ca4238/2026/02/10/38ef58526a88462db5c6cbc3bf31d274.png?x-oss-process=image/resize,m_pad,h_200,h_200",
            "picUrl": "https://oss.vortmall.com/demo/upload/c4ca4238/2026/02/10/38ef58526a88462db5c6cbc3bf31d274.png",
            "picActiveThumb": "https://oss.vortmall.com/demo/upload/c4ca4238/2026/02/10/822846437e34451ab1b57e2c991bc666.png?x-oss-process=image/resize,m_pad,h_200,h_200",
            "picActiveUrl": "https://oss.vortmall.com/demo/upload/c4ca4238/2026/02/10/822846437e34451ab1b57e2c991bc666.png",
            "picLink": {"path": "base", "label": "商城首页", "name": "商城首页", "link": "/"},
            "picId": "2021034691685007360",
            "picName": "1680247587QG2y44h7a7f0M1dx9T!!pic200x200.png"
        },
        {
            "picTitle": "我的",
            "picThumb": "https://oss.vortmall.com/demo/upload/c4ca4238/2026/02/10/ecc4581c9a8f4a0c865af4261fae8053.png?x-oss-process=image/resize,m_pad,h_200,h_200",
            "picUrl": "https://oss.vortmall.com/demo/upload/c4ca4238/2026/02/10/ecc4581c9a8f4a0c865af4261fae8053.png",
            "picActiveThumb": "https://oss.vortmall.com/demo/upload/c4ca4238/2026/02/10/609f0ce77fb84f8fbd483e7bf3668239.png?x-oss-process=image/resize,m_pad,h_200,h_200",
            "picActiveUrl": "https://oss.vortmall.com/demo/upload/c4ca4238/2026/02/10/609f0ce77fb84f8fbd483e7bf3668239.png",
            "picLink": {"path": "base", "label": "会员中心", "name": "会员中心", "link": "member"},
            "picId": "2021034691664035840",
            "picName": "1680247679vqWSO1Ci9sAGJCzzZa!!pic200x200.png"
        }
    ],
    "enableAnimation": false
}"#;

pub struct HomeService {
    repos: Arc<Repositories>,
}

impl HomeService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 友情链接 —————————————————————

    /// `GET /api/home/home/friendLinks`：友情链接列表。
    pub async fn friend_links(&self) -> Result<Vec<FriendLinkVO>> {
        let rows = self.repos.content.home.list_friend_links().await?;
        Ok(rows.iter().map(to_friend_link_vo).collect())
    }

    // ————————————————————— 装修（离散 / 页面） —————————————————————

    /// `GET /api/home/home/mobileNav`：移动端导航。
    ///
    /// 对应 Java `HomeController#mobileNav`：
    /// - `isNew == 1` → 读「H5 导航页」装修（`type=H5,pageType=7,isHome=1,status=1`）
    ///   并返回解析后的 JSON 对象；
    /// - 否则 → 读旧版离散装修 `decorate_sn`（缺省 `mobileNav`）。
    pub async fn mobile_nav(&self, q: &MobileNavQuery) -> Result<Value> {
        let is_new = q.is_new.unwrap_or(0) == 1;
        if is_new {
            let decorate = self
                .repos
                .content
                .decorate
                .find_home_page(
                    Some(DECORATE_TYPE_H5),
                    Some(PAGE_NAVIGATION),
                    Some(PLATFORM_SHOP_ID),
                    Some(DECORATE_PUBLISHED),
                    Some(0),
                )
                .await?;
            let data = decorate
                .as_ref()
                .and_then(|d| parse_json_opt(&d.data))
                .unwrap_or_else(|| {
                    serde_json::from_str(DEFAULT_NAV_H5_DECORATE_DATA)
                        .unwrap_or_else(|_| json!({ "moduleList": [] }))
                });
            return Ok(data);
        }
        let sn = decorate_sn_or(q.decorate_sn.as_deref(), "mobileNav");
        Ok(self.discrete_value(&sn).await?)
    }

    /// `GET /api/home/home/memberDecorate`：个人中心（旧版离散装修）。
    pub async fn member_decorate(&self, sn: Option<&str>) -> Result<Value> {
        let sn = decorate_sn_or(sn, "memberDecorate");
        self.discrete_value(&sn).await
    }

    /// `GET /api/home/home/pcLogin`：PC 登录页装修（预览 pageType=5）。
    pub async fn pc_login(&self, q: &DecoratePreviewQuery) -> Result<DecorateModuleListVO> {
        self.decorate_module(DECORATE_TYPE_PC, PAGE_LOGIN, q.preview_id)
            .await
    }

    /// `GET /api/home/home/pcSearch`：PC 搜索页装修。
    pub async fn pc_search(&self, q: &DecoratePreviewQuery) -> Result<DecorateModuleListVO> {
        self.decorate_module(DECORATE_TYPE_PC, PAGE_SEARCH, q.preview_id)
            .await
    }

    /// `GET /api/home/home/pcHeader`：PC 头部装修配置（返回 `{"pcHeader": ...}`）。
    ///
    /// 契约要点（**必须与 Java 一致**）：Java `getPcHeader` 在
    /// `pageModule.pcHeader` 缺失时 `return pageModule.get("pcHeader")`
    /// 直接返回 **null**，控制器再包一层得到 `{"pcHeader": null}`。
    ///
    /// 前台据此判新旧版：`resolvePcHeaderConfigFromResult` 对 `null` 返回 null，
    /// `isHomeNewStyle` 为 false → 全站 `.container` 用旧版固定 1190px 宽度。
    /// 若这里返回 `{}`（空对象），前台会把它当成「已配置新版头部」，
    /// 于是旧版首页模块套上 1264–1600 的新版宽度 → 首屏 Banner/分类栏/推荐位
    /// 与下方楼层错位（**2026-10-21 首页布局错乱的根因**）。
    pub async fn pc_header(&self, q: &DecoratePreviewQuery) -> Result<Value> {
        let decorate = self
            .find_page(DECORATE_TYPE_PC, PAGE_HOME, q.preview_id)
            .await?;
        let header = decorate
            .as_ref()
            .and_then(|d| parse_json_opt(&d.data))
            .and_then(|value| pc_header_from(&value))
            .unwrap_or(Value::Null);
        Ok(json!({ "pcHeader": header }))
    }

    /// `GET /api/home/home/o2oIndex?shopId=`：门店首页装修。
    ///
    /// 对应 Java `o2oIndex`：`shop_id=shopId, status=1, is_home=1, type=H5,
    /// page_type=resolveO2oHomePageType(shopId)`。无记录时 Java 会落库默认装修；
    /// 本实现不写库，返回空模块列表（见模块头偏差 3）。
    pub async fn o2o_index(&self, q: &O2oIndexQuery) -> Result<O2oDecorateVO> {
        let shop_id = q.shop_id.unwrap_or(0);
        let page_type = resolve_o2o_page_type(Some(shop_id));
        let decorate = self
            .repos
            .content
            .decorate
            .find_home_page(
                Some(DECORATE_TYPE_H5),
                Some(page_type),
                Some(shop_id),
                Some(DECORATE_PUBLISHED),
                Some(0),
            )
            .await?;
        let data = match decorate {
            Some(d) => match parse_json_opt(&d.data) {
                Some(v) => v,
                None => {
                    return Err(nextrade_core::Error::biz_msg("数据格式错误"));
                }
            },
            None => {
                tracing::info!("门店 {shop_id} 无 O2O 首页装修，返回空模块列表");
                json!({ "moduleList": [] })
            }
        };
        Ok(O2oDecorateVO { data: Some(data) })
    }

    /// `GET /api/home/home/h5Category`：H5 分类页装修。
    pub async fn h5_category(&self, q: &DecoratePreviewQuery) -> Result<DecorateModuleListVO> {
        self.decorate_module(DECORATE_TYPE_H5, PAGE_CATEGORY, q.preview_id)
            .await
    }

    /// `GET /api/home/home/h5ProductDetail`：H5 商品详情页装修。
    pub async fn h5_product_detail(
        &self,
        q: &DecoratePreviewQuery,
    ) -> Result<DecorateModuleListVO> {
        self.decorate_module(DECORATE_TYPE_H5, PAGE_H5_PRODUCT_DETAIL, q.preview_id)
            .await
    }

    /// `GET /api/home/home/h5Member`：H5 个人中心装修。
    pub async fn h5_member(&self, q: &DecoratePreviewQuery) -> Result<DecorateModuleListVO> {
        let mut vo = self
            .decorate_module(DECORATE_TYPE_H5, PAGE_MEMBER, q.preview_id)
            .await?;
        normalize_member_account_module(&mut vo);
        normalize_member_service_links(&mut vo);
        Ok(vo)
    }

    /// `GET /api/home/home/h5Search`：移动端搜索页装修。
    pub async fn h5_search(&self, q: &DecoratePreviewQuery) -> Result<DecorateModuleListVO> {
        self.decorate_module(DECORATE_TYPE_H5, PAGE_SEARCH, q.preview_id)
            .await
    }

    /// `GET /api/home/home/mobileLogin`：移动端登录页装修。
    pub async fn mobile_login(&self, q: &DecoratePreviewQuery) -> Result<DecorateModuleListVO> {
        self.decorate_module(DECORATE_TYPE_H5, PAGE_LOGIN, q.preview_id)
            .await
    }

    // ————————————————————— 配置 / 推荐 —————————————————————

    /// `GET /api/home/home/getCustomerServiceConfig`：客服设置。
    ///
    /// 对应 Java `ConfigService#getConfigCustomer`，字段与 `ConfigCustomerDTO` 一致。
    pub async fn customer_service_config(&self) -> Result<ConfigCustomerVO> {
        let map = self
            .repos
            .content
            .home
            .config_vals(&[
                "h5Domain",
                "corpId",
                "url",
                "openType",
                "serviceType",
                "show",
            ])
            .await?;
        Ok(ConfigCustomerVO {
            h5_domain: cfg_str(&map, "h5Domain"),
            corp_id: cfg_str(&map, "corpId"),
            url: cfg_str(&map, "url"),
            open_type: cfg_int(&map, "openType"),
            service_type: cfg_int(&map, "serviceType"),
            show: cfg_int(&map, "show"),
        })
    }

    /// `GET /api/home/home/getRecommend`：首页推荐模块数据。
    ///
    /// 对应 Java `HomeController#getRecommend`：
    /// `previewId > 0` 走预览装修，否则走发布版；按 `moduleIndex` 定位模块。
    /// **偏差**：Java 会按模块类型展开商品/优惠券等数据；本实现返回装修页里
    /// 保存的模块配置（`module.data` 优先）。
    pub async fn get_recommend(&self, q: &RecommendQuery) -> Result<Value> {
        let decorate = match q.decorate_id.filter(|id| *id > 0) {
            Some(decorate_id) => self.find_decorate_by_id(decorate_id).await?,
            None => {
                self.find_page(DECORATE_TYPE_H5, PAGE_HOME, q.preview_id)
                    .await?
            }
        };
        let Some(decorate) = decorate else {
            return Ok(Value::Null);
        };
        let value = parse_json_opt(&decorate.data).unwrap_or_else(|| json!({}));
        let module = find_module(&value, q.module_index.as_deref());
        Ok(module.unwrap_or(Value::Null))
    }

    // ————————————————————— 内部辅助 —————————————————————

    /// 离散装修 detail → JSON（Java 返回 `DecorateDiscreteVO` 序列化结果）。
    async fn discrete_value(&self, sn: &str) -> Result<Value> {
        let row = self.repos.content.home.find_discrete(sn).await?;
        Ok(match row {
            Some(d) => json!({
                "id": d.id,
                "decorateSn": d.decorate_sn,
                "decorateName": d.decorate_name,
                "shopId": d.shop_id,
                "data": parse_json_opt(&Some(d.data.clone())).unwrap_or(Value::Null),
            }),
            None => Value::Null,
        })
    }

    /// 装修页面 → `DecorateModuleListVO`（预览优先，其次发布版）。
    async fn decorate_module(
        &self,
        decorate_type: i32,
        page_type: i32,
        preview_id: Option<i32>,
    ) -> Result<DecorateModuleListVO> {
        let decorate = self.find_page(decorate_type, page_type, preview_id).await?;
        Ok(decorate
            .as_ref()
            .map(module_vo)
            .unwrap_or_else(|| DecorateModuleListVO {
                is_home: Some(1),
                module_list: Some(Vec::new()),
                ..DecorateModuleListVO::default()
            }))
    }

    /// 查装修页：`preview_id > 0` 时按 ID 取（校验类型/页面类型），
    /// 否则取已发布的首页版本（Java `getAppHomeDecorate` / `getDecorateModule`）。
    async fn find_page(
        &self,
        decorate_type: i32,
        page_type: i32,
        preview_id: Option<i32>,
    ) -> Result<Option<Decorate>> {
        if let Some(preview_id) = preview_id.filter(|id| *id > 0) {
            let decorate = self.find_decorate_by_id(preview_id).await?;
            let Some(decorate) = decorate else {
                return Err(nextrade_core::Error::biz_msg("装修模板不存在!"));
            };
            // Java 预览会同时校验页面类型：不匹配时视为无数据
            if decorate.page_type == page_type {
                return Ok(Some(decorate));
            }
            return Ok(None);
        }
        self.repos
            .content
            .decorate
            .find_home_page(
                Some(decorate_type),
                Some(page_type),
                Some(PLATFORM_SHOP_ID),
                Some(DECORATE_PUBLISHED),
                Some(0),
            )
            .await
    }

    async fn find_decorate_by_id(&self, decorate_id: i32) -> Result<Option<Decorate>> {
        self.repos.content.decorate.find_by_id(decorate_id).await
    }
}

// ————————————————————— 纯函数（可单测） —————————————————————

/// `FriendLinks` 实体 → `FriendLinkVO`。
#[must_use]
fn to_friend_link_vo(f: &FriendLinks) -> FriendLinkVO {
    FriendLinkVO {
        link_id: f.link_id,
        link_title: f.link_title.clone(),
        link_logo: f.link_logo.clone(),
        link_url: f.link_url.clone(),
        sort_order: f.sort_order,
    }
}

/// 装修数据 JSON → `DecorateModuleListVO`（过滤隐藏模块，与既有
/// `DecorateService::module_vo` 同一口径）。
#[must_use]
fn module_vo(d: &Decorate) -> DecorateModuleListVO {
    let mut vo = DecorateModuleListVO {
        decorate_id: Some(d.decorate_id),
        is_home: d.is_home,
        bind_decorate_id: d.binded_h5_id,
        ..DecorateModuleListVO::default()
    };
    let Some(value) = parse_json_opt(&d.data) else {
        return vo;
    };
    vo.page_module = value
        .get("pageModule")
        .or_else(|| value.get("page_module"))
        .cloned();
    vo.module_list = value
        .get("moduleList")
        .or_else(|| value.get("module_list"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter(|m| module_visible(m))
                .cloned()
                .collect::<Vec<_>>()
        });
    vo
}

/// PC 头部配置：依次尝试 `pcHeader` / `pageModule.header` / 首个
/// `type` 含 header 的模块，取不到返回 None。
///
/// 模块 `type` 命中采用**大小写不敏感**匹配：装修数据里同一语义存在
/// `header` / `pcHeader` / `PC_HEADER` 等写法（Java 侧模块类型字符串由前端
/// 保存，大小写不保证统一），旧的 `contains("header")` 会漏掉驼峰写法
/// `pcHeader`（并导致单测 `pc_header_prefers_explicit_fields` unwrap panic）。
#[must_use]
fn pc_header_from(value: &Value) -> Option<Value> {
    if let Some(header) = value.get("pcHeader").or_else(|| value.get("pc_header")) {
        return Some(header.clone());
    }
    if let Some(header) = value
        .get("pageModule")
        .or_else(|| value.get("page_module"))
        .and_then(|pm| {
            pm.get("header")
                .or_else(|| pm.get("pcHeader"))
                .or_else(|| pm.get("pc_header"))
        })
    {
        return Some(header.clone());
    }
    value
        .get("moduleList")
        .or_else(|| value.get("module_list"))
        .and_then(Value::as_array)
        .and_then(|arr| {
            arr.iter().find(|m| {
                m.get("type")
                    .and_then(Value::as_str)
                    .is_some_and(|t| t.to_ascii_lowercase().contains("header"))
            })
        })
        .cloned()
}

/// 按 `moduleIndex` 定位模块：数字 → 数组下标；否则依次匹配 `moduleIndex` /
/// `key` / `name` / `type` 字段。
#[must_use]
fn find_module(value: &Value, module_index: Option<&str>) -> Option<Value> {
    let module_index = module_index.map(str::trim).filter(|s| !s.is_empty())?;
    let arr = value
        .get("moduleList")
        .or_else(|| value.get("module_list"))
        .and_then(Value::as_array)?;
    if let Ok(idx) = module_index.parse::<usize>() {
        return arr.get(idx).cloned();
    }
    arr.iter()
        .find(|m| {
            ["moduleIndex", "key", "name", "type"]
                .iter()
                .any(|field| m.get(*field).and_then(Value::as_str) == Some(module_index))
        })
        .cloned()
}

/// 配置字符串值（未命中返回 None；命中 NULL 也返回 None）。
#[must_use]
fn cfg_str(map: &std::collections::HashMap<String, Option<String>>, key: &str) -> Option<String> {
    map.get(key).cloned().flatten()
}

/// 配置整数值（解析失败返回 None）。
#[must_use]
fn cfg_int(map: &std::collections::HashMap<String, Option<String>>, key: &str) -> Option<i32> {
    map.get(key)
        .and_then(|v| v.as_deref())
        .and_then(|v| v.trim().parse::<i32>().ok())
}

/// 当前 H5 包没有 `account` 组件，把账号装修模块降级为 `myService` 列表，
/// 账号模块在新版 H5 有独立 `account` 组件；这里只补默认标题和标签，
/// 不再降级成 `myService`（否则会和服务模块重复，且丢失语言/地区入口）。
fn normalize_member_account_module(vo: &mut DecorateModuleListVO) {
    let Some(modules) = vo.module_list.as_mut() else {
        return;
    };
    for item in modules.iter_mut() {
        if item.get("type").and_then(Value::as_str) != Some("account") {
            continue;
        }
        item["label"] = Value::String("账号".into());
        if let Some(module) = item.get_mut("module").and_then(Value::as_object_mut) {
            module
                .entry("moduleTitle")
                .or_insert_with(|| Value::String("账号信息".into()));
        }
    }
}

/// 旧 H5 服务组件只接受字符串 URL；后台配置里的 default 链接对象需要降级。
fn normalize_member_service_links(vo: &mut DecorateModuleListVO) {
    let Some(modules) = vo.module_list.as_mut() else {
        return;
    };
    for item in modules.iter_mut() {
        if !matches!(
            item.get("type").and_then(Value::as_str),
            Some("myService" | "my-service")
        ) {
            continue;
        }
        let Some(service_list) = item
            .get_mut("module")
            .and_then(|m| m.get_mut("serviceList"))
            .and_then(Value::as_array_mut)
        else {
            continue;
        };
        for entry in service_list.iter_mut() {
            let Some(link) = entry.get("picLink") else {
                continue;
            };
            if let Some(s) = link.as_str() {
                if s.starts_with('/') {
                    continue;
                }
            }
            let normalized = link
                .get("link")
                .and_then(Value::as_str)
                .filter(|s| s.starts_with('/'))
                .map(str::to_string);
            if let Some(url) = normalized {
                entry["picLink"] = Value::String(url);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::entities::Decorate;

    fn decorate(data: &str) -> Decorate {
        Decorate {
            decorate_id: 7,
            data: Some(data.to_string()),
            decorate_type: Some(DECORATE_TYPE_H5),
            page_type: PAGE_HOME,
            is_home: Some(1),
            shop_id: 0,
            status: Some(1),
            binded_h5_id: Some(3),
            ..Default::default()
        }
    }

    #[test]
    fn module_vo_filters_hidden_modules() {
        let d = decorate(
            r#"{"moduleList":[{"type":"banner","isShow":true},{"type":"ad","isShow":false},{"type":"nav"}],"pageModule":{"title":"首页"}}"#,
        );
        let vo = module_vo(&d);
        assert_eq!(vo.decorate_id, Some(7));
        assert_eq!(vo.is_home, Some(1));
        assert_eq!(vo.bind_decorate_id, Some(3));
        assert_eq!(vo.module_list.as_ref().unwrap().len(), 2);
        assert_eq!(vo.page_module.unwrap()["title"], "首页");
    }

    #[test]
    fn module_vo_tolerates_missing_or_invalid_data() {
        let mut d = decorate("not-json");
        let vo = module_vo(&d);
        assert_eq!(vo.decorate_id, Some(7));
        assert!(vo.module_list.is_none());

        d.data = None;
        let vo = module_vo(&d);
        assert!(vo.module_list.is_none());
    }

    #[test]
    fn find_module_supports_index_and_key() {
        let value = json!({
            "moduleList": [
                {"type": "banner", "moduleIndex": "banner"},
                {"type": "product", "key": "recommend", "data": {"productIds": [1, 2]}}
            ]
        });
        assert_eq!(
            find_module(&value, Some("0")).unwrap()["type"],
            Value::String("banner".into())
        );
        let by_key = find_module(&value, Some("recommend")).unwrap();
        assert_eq!(by_key["data"]["productIds"], json!([1, 2]));
        assert!(find_module(&value, Some("missing")).is_none());
        assert!(find_module(&value, None).is_none());
    }

    #[test]
    fn pc_header_prefers_explicit_fields() {
        assert_eq!(
            pc_header_from(&json!({"pcHeader": {"logo": "a.png"}})).unwrap()["logo"],
            Value::String("a.png".into())
        );
        assert_eq!(
            pc_header_from(&json!({"pageModule": {"header": {"logo": "b.png"}}})).unwrap()["logo"],
            Value::String("b.png".into())
        );
        assert_eq!(
            pc_header_from(&json!({"moduleList": [{"type": "pcHeader"}, {"type": "banner"}]}))
                .unwrap()["type"],
            Value::String("pcHeader".into())
        );
        assert!(pc_header_from(&json!({})).is_none());
    }

    #[test]
    fn default_nav_h5_decorate_data_is_valid() {
        let val: Value = serde_json::from_str(DEFAULT_NAV_H5_DECORATE_DATA).unwrap();
        let nav_list = val.get("navList").and_then(|v| v.as_array()).unwrap();
        assert_eq!(nav_list.len(), 2);
        assert_eq!(nav_list[0]["picTitle"], "首页");
        assert_eq!(nav_list[1]["picTitle"], "我的");
        assert_eq!(val["enableAnimation"], false);
    }
}
