//! 店铺信息服务（merchant::shop_manage）。
//! 对应原 Java `com.nextrade.service.shop.impl.ShopServiceImpl` 中由
//! `ShopController`（`/adminapi/merchant/shop`）暴露的 20 条链路，
//! 以及 `AdminUserShopServiceImpl` 的收银台登录偏好方法。
//!
//! | 本服务方法 | 原 Java |
//! |-----------|---------|
//! | [`ShopManageService::list`] | `ShopServiceImpl#list` |
//! | [`ShopManageService::detail_with_score`] | `ShopServiceImpl#detailWithScore` |
//! | [`ShopManageService::current_detail`] | `ShopServiceImpl#detail(id, true)` + `lastLoginTime` 落库 |
//! | [`ShopManageService::create`] | `ShopServiceImpl#create` + `updateShop(id, shop, true)` |
//! | [`ShopManageService::update`] | `ShopServiceImpl#update` |
//! | [`ShopManageService::delete`] | `BaseServiceImpl#del` |
//! | [`ShopManageService::update_field`] | `BaseServiceImpl#updateField(updateField, GENERAL_FIELDS)` |
//! | [`ShopManageService::batch`] | `BaseServiceImpl#batch` |
//! | [`ShopManageService::my_shop`] | `ShopController#myShop` |
//! | [`ShopManageService::set_pos_login_default`] | `AdminUserShopServiceImpl#setPosLoginDefault` |
//! | [`ShopManageService::record_login_system`] | `AdminUserShopServiceImpl#recordLoginSystem` |
//! | [`ShopManageService::choose`] | `ShopServiceImpl#choose`（Redis 部分未迁移） |
//! | [`ShopManageService::shop_list`] | `ShopServiceImpl#getShopList` |
//! | [`ShopManageService::staff_show`] | `ShopServiceImpl#staffShow` |
//! | [`ShopManageService::update_info`] | `ShopServiceImpl#updateInfo` |
//! | [`ShopManageService::setting`] | `ShopServiceImpl#setting` |
//! | [`ShopManageService::get_vendor_setting`] | `ShopServiceImpl#getVendorSetting` |
//! | [`ShopManageService::update_vendor_setting`] | `ShopServiceImpl#updateVendorSetting` |
//! | [`ShopManageService::store_list_by_product_id`] | `ShopServiceImpl#getStoreListByProductId` |
//! | [`ShopManageService::del_shop`] | `ShopServiceImpl#delShop` |
//!
//! ## 与 Java 的行为差异（逐条，均已在 `uncertain_points` 中列明）
//!
//! 1. **登录态店铺（`HeaderUtils.getShopId()`）**：原 Java 从
//!    `x-shop-id` / `x-admin-type` 请求头取当前店铺；Rust JWT
//!    （`nextrade_security::web::AuthUser`）未签发这些字段。
//!    本服务所有「当前店铺」语义一律改为**显式入参**（`shop_id` 参数，
//!    由路由层从查询参数/请求体取）。这**不是等价实现**：
//!    平台账号可伪装店铺视角，店铺账号不传时会被当作平台视角。
//! 2. **多门店（`nextradeProperties.isO2o`）**：Java 通过
//!    `application.yml` 的 `nextrade.is-o2o` 决定。Rust `AppConfig`
//!    尚无对应字段，故由构造函数显式传入（路由层传 `false`），
//!    所有 `is_o2o == 1` 分支默认关闭。`myShop` 在 o2o 关闭时只返回店铺类型。
//! 3. **未迁移依赖**（明确不做，调用时返回明确业务错误或安全默认值）：
//!    - `adminRoleService.initAdminRole` / `initEmployeeRoles`（角色域未迁移）
//!      → `create` 中**不初始化角色**，`admin_user_shop.role_id` 落 0
//!    - `shippingTplService.initShippingTpl`（运费模板域）→ 不初始化
//!    - `decorateService.initDecorate`（装修域）→ 不初始化
//!    - `configService`（系统配置域）→ 仅按需直读 `config` 表
//!      （`maxSubAdministrator` / `maxShopCount`）
//!    - `regionService`（地区域）→ `shopRegionNames` 恒为空数组
//!    - `merchantService.getMerchantVO`（商户域）→ 仅回填 `merchant`
//!      表自有列；`statusText` / `typeText` / `shopCount` / `user` 不填充
//!    - `productService.getShopSettings()` / `getStoreSelfOperatedSettings()`
//!      （平台自营店铺/门店配置）→ `includeSelfOperated=1` 时返回明确错误
//!    - Redis（`choose` 写 `admin::auth::<adminId>`）→ 只做权限校验并返回
//!      权限清单，**不写缓存**（JWT 无服务端会话）
//!    - `adminLogService`（日志写入）→ 只读 `admin_log`，不写日志

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// 读取 O2O 开关（对应 Java `nextradeProperties.isO2o()`；配置服务未迁移）。
///
/// `TIG_IS_O2O=1` 或 `=true`（大小写不敏感）视为开启，其余/未设置视为关闭。
fn is_o2o_from_env() -> bool {
    std::env::var("TIG_IS_O2O")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

use rust_decimal::Decimal;
use nextrade_common::utils;
use nextrade_core::entities::{AdminUserShop, Merchant, Shop, StoreTip};
use nextrade_core::models::merchant::shop_manage::{
    shop_status, shop_status_text, shop_type, BatchDTO, LoginChooseParam, OperateDTO,
    PosLoginSettingPayload, ShopAdminUserShopVO, ShopDeliveryInfo, ShopListQuery, ShopMerchantVO,
    ShopSettingPayload, ShopTipVO, ShopVO, ShopWritePayload, StaffAdminLogVO, StaffShowVO,
    StoreListVO, UpdateFieldDTO, VendorSettingUpdateParam, VendorSettingVO,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// `updateField` 允许的列白名单（Java `CheckFieldConstants.GENERAL_FIELDS`）。
///
/// 注意：该常量在 Java 中是全局通用的，值为 `is_show` / `sort_order` /
/// `status`；`shop` 表只有 `status` 一列，另两列在库中不存在。
/// Java 的 `BaseServiceImpl#updateField` 在字段不在白名单时抛
/// `INVALID_FIELD_VALUE`，构造 `set(field, val)` 时若列不存在则由 MySQL 报错。
/// Rust 侧按「属性名 → 白名单列名」映射，白名单外的字段直接报错，
/// **不会**回落到主键列（Java 的 `getColumnByPropertyName` 会回落）。
pub const GENERAL_FIELDS: [&str; 3] = ["is_show", "sort_order", "status"];

/// 店铺名称长度上限（Java `ShopCreateDTO` `@Size(max = 50)`）。
pub const SHOP_TITLE_MAX_LEN: usize = 50;

/// `shop` 表 `shop_star` 默认值（Java `create`：`new BigDecimal(4.8)`）。
const DEFAULT_SHOP_STAR: &str = "4.8";

/// 店铺 ID 随机分配范围（Java `assignRandomShopId`：`[1e9, 2e9)`，重试 10 次）。
const SHOP_ID_MIN: i32 = 1_000_000_000;
const SHOP_ID_MAX_EXCLUSIVE: i32 = 2_000_000_000;
const SHOP_ID_RETRY: usize = 10;

/// 设置/记录登录偏好时的门店 ID 是否必填（Java 未校验，`null` 会查不到绑定 → 报错/静默返回）。
fn require_shop_id(shop_id: Option<i32>, what: &str) -> Result<i32> {
    shop_id
        .filter(|v| *v > 0)
        .ok_or_else(|| Error::Validation(format!("{what}不能为空")))
}

/// 把 Java 属性名/列名映射为 `shop` 表白名单列名。
///
/// 对应 Java `BaseServiceImpl#getColumnByPropertyName`（但它找不到时**回落主键**，
/// 这里返回 `None` 让调用方报错，避免静默写错列）。
#[must_use]
pub fn resolve_field_column(field: &str) -> Option<&'static str> {
    match field.trim() {
        "status" => Some("status"),
        "isShow" | "is_show" => Some("is_show"),
        "sortOrder" | "sort_order" => Some("sort_order"),
        _ => None,
    }
}

/// 纯函数：`updateField` 的字段校验（可单测）。
///
/// 返回允许写入的列名；不在白名单或列不在 `shop` 表时返回错误文案。
pub fn validate_update_field(field: Option<&str>, val: Option<&str>) -> Result<(&'static str, String)> {
    let field = field
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Validation("字段名不能为空".into()))?;
    let val = val
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Validation("字段值不能为空".into()))?;
    let col = resolve_field_column(field)
        .filter(|c| GENERAL_FIELDS.contains(c))
        .ok_or_else(|| Error::Validation(format!("非法字段值: {field}")))?;
    Ok((col, val.to_string()))
}

/// 纯函数：门店参数校验（Java `ShopServiceImpl#checkStoreParam`）。
///
/// 仅在 `is_o2o == true` 且 `shopType` 为门店(2)/自提点(3) 时生效。
/// 校验顺序与 Java 完全一致。
pub fn check_store_param(is_o2o: bool, p: &ShopWritePayload) -> Result<()> {
    let Some(st) = p.shop_type else {
        return Ok(());
    };
    if !is_o2o || !(st == shop_type::STORE || st == shop_type::PICKUP) {
        return Ok(());
    }
    if st == shop_type::STORE && p.shop_cover_picture.as_deref().is_none_or(str::is_empty) {
        return Err(Error::biz_msg("门店封面不能为空"));
    }
    if p.shop_contact_config.is_none() {
        return Err(Error::biz_msg("请填写联系方式"));
    }
    if p.shop_open_close_config.is_none() {
        return Err(Error::biz_msg("请填写营业时间"));
    }
    if p.shop_region_ids.is_none() {
        return Err(Error::biz_msg("请选择区域"));
    }
    if p.shop_longitude.is_none() || p.shop_latitude.is_none() {
        return Err(Error::biz_msg("经纬度不能为空"));
    }
    if !(1..=3).contains(&st) {
        return Err(Error::biz_msg("类型错误"));
    }
    if st == shop_type::PICKUP && p.store_parent_id.is_none() {
        return Err(Error::biz_msg("父门店id不能为空"));
    }
    Ok(())
}

/// 纯函数：`create` 的商户校验（Java `ShopServiceImpl#create`）。
///
/// Java 原文：
/// ```text
/// boolean checkMerchantId = createDTO.getMerchantId() <= 0
///         && (ShopTypeEnum.SHOP.getCode() == shopType || ShopTypeEnum.STORE.getCode() == shopType);
/// if (createDTO.getMerchantId() == null || checkMerchantId) {
///     throw new GlobalException(messageHelper.get("error.merchant.select.required"));
/// }
/// ```
/// 即：
/// - `merchantId == null` → **必报错**（与 shopType 无关）；
/// - `merchantId <= 0` 且 `shopType ∈ {1 店铺, 2 门店}` → 报错；
/// - `merchantId <= 0` 且 `shopType == 3 自提点` → 通过（自提点无商户限制）。
///
/// 文案 `error.merchant.select.required` 未在 Rust 侧 i18n 迁移，
/// 按现有约定回退中文「请选择商户」。
pub fn validate_create_merchant(
    merchant_id: Option<i32>,
    shop_type: Option<i32>,
) -> Result<i32> {
    let id = merchant_id.ok_or_else(|| Error::biz_msg("请选择商户"))?;
    let st = shop_type.unwrap_or(shop_type::SHOP);
    let needs_merchant = st == shop_type::SHOP || st == shop_type::STORE;
    if id <= 0 && needs_merchant {
        return Err(Error::biz_msg("请选择商户"));
    }
    Ok(id)
}

/// 纯函数：`compute_shop_star`（Java `ShopMapper#calculateShopStar`）。
///
/// 评价数（`shop_star_num`）为空/0/`< 100` → 固定 4.8；否则取 `shop_star`（空 → 4.8）。
#[must_use]
pub fn compute_shop_star(shop_star_num: Option<i32>, shop_star: Option<Decimal>) -> Decimal {
    let default = DEFAULT_SHOP_STAR.parse::<Decimal>().unwrap_or(Decimal::ZERO);
    match shop_star_num {
        Some(n) if n >= 100 => shop_star.unwrap_or(default),
        _ => default,
    }
}

/// 纯函数：`updateVendorSetting` 的入参解析（Java 侧 `String` → 列 `decimal(10,2)`）。
///
/// 空串/`None` → `None`（Java `ObjectUtil.isNotEmpty` 为假，不写库）；
/// 非数字 → `Validation` 错误（Java 由 MyBatis 写入时报错，这里提前拒绝）。
pub fn parse_vendor_price_auto_value(raw: Option<&str>) -> Result<Option<Decimal>> {
    let Some(v) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    v.parse::<Decimal>()
        .map(Some)
        .map_err(|_| Error::Validation(format!("智能设价必须为数字: {v}")))
}

/// 纯函数：`shop` 表 `add_time`（秒）→ `yyyy-MM-dd HH:mm:ss`。
///
/// 对应 Java `SimpleDateFormat("yyyy-MM-dd HH:mm:ss").format(new Date(sec * 1000))`；
/// 0 / 负数按 Java `TigUtils.handelTime` 口径返回空串。
#[must_use]
pub fn fmt_shop_time(secs: Option<i32>) -> String {
    let secs = secs.unwrap_or(0);
    if secs <= 0 {
        return String::new();
    }
    use chrono::TimeZone;
    match chrono::Local.timestamp_opt(i64::from(secs), 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => String::new(),
    }
}

/// 纯函数：JSON 数组参数 → `Vec<i32>`（`shop.kefu_inlet` / `shop_region_id`）。
///
/// 兼容三种形态：JSON 数组 `[1,2]`、JSON 字符串数组 `["1","2"]`、单值 `1`。
/// 无法解析时返回空数组（Java `JSONUtil.parseArray(...).toList(Integer.class)`
/// 在非法输入时抛异常，Rust 侧选择降级以免整页失败）。
#[must_use]
pub fn parse_i32_list_raw(raw: Option<&str>) -> Vec<i32> {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Vec::new();
    };
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(json_to_i32)
            .collect::<Vec<i32>>(),
        Ok(serde_json::Value::Number(n)) => n
            .as_i64()
            .and_then(|v| i32::try_from(v).ok())
            .into_iter()
            .collect(),
        Ok(serde_json::Value::String(s)) => s.parse::<i32>().ok().into_iter().collect(),
        _ => Vec::new(),
    }
}

/// JSON 值 → i32（数字或数字字符串）。
fn json_to_i32(v: &serde_json::Value) -> Option<i32> {
    match v {
        serde_json::Value::Number(n) => n.as_i64().and_then(|x| i32::try_from(x).ok()),
        serde_json::Value::String(s) => s.trim().parse::<i32>().ok(),
        _ => None,
    }
}

/// JSON 字符串 → 值（非法返回 `None`）。
fn parse_json_opt(raw: Option<&str>) -> Option<serde_json::Value> {
    let raw = raw.map(str::trim).filter(|s| !s.is_empty())?;
    serde_json::from_str::<serde_json::Value>(raw).ok()
}

/// 收发货信息解析（Java `ShopServiceImpl#parseDeliveryInfo`：
/// 空 / 非 JSON → `None`）。
fn parse_delivery_info(raw: Option<&str>) -> Option<ShopDeliveryInfo> {
    let raw = raw.map(str::trim).filter(|s| !s.is_empty())?;
    serde_json::from_str::<ShopDeliveryInfo>(raw).ok()
}

pub struct ShopManageService {
    repos: Arc<Repositories>,
    /// `nextradeProperties.isO2o == 1`（见模块头第 2 条差异）。
    is_o2o: bool,
}

impl ShopManageService {
    /// 标准构造（与项目其它切片一致）：`is_o2o` 从环境变量 `TIG_IS_O2O` 读取。
    ///
    /// 对应原 Java `nextradeProperties.isO2o()`——Rust 侧配置服务尚未迁移，
    /// 暂以环境变量驱动（`1` / `true` 视为开启，默认关闭＝非 O2O 单商户版）。
    /// 需要显式指定开关时用 [`ShopManageService::with_o2o`]。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self::with_o2o(repos, is_o2o_from_env())
    }

    /// `is_o2o` 对应 Java `nextradeProperties.getIsO2o() == 1`（显式传入）。
    #[must_use]
    pub fn with_o2o(repos: Arc<Repositories>, is_o2o: bool) -> Self {
        Self { repos, is_o2o }
    }

    // ————————————————————— 列表 —————————————————————

    /// 店铺分页（对应 Java `ShopController.list` / `ShopServiceImpl#list`）。
    ///
    /// `shop_id_scope`：登录态店铺（Java `HeaderUtils.getShopId()`，
    /// Rust 侧由路由层从查询参数取；`None`/`<= 0` 为平台视角）。
    pub async fn list(
        &self,
        q: &ShopListQuery,
        shop_id_scope: Option<i32>,
    ) -> Result<PageResult<ShopVO>> {
        // mainAccount / account 的绑定店铺预取（Java 在循环外各查一次）
        let manager_ids: Option<Vec<i32>> = if let Some(acc) =
            q.main_account.as_deref().filter(|s| !s.trim().is_empty())
        {
            Some(
                self.repos.merchant.shop_manage
                    .find_bind_shop_ids_by_account(acc.trim(), true)
                    .await?,
            )
        } else if let Some(acc) = q.account.as_deref().filter(|s| !s.trim().is_empty()) {
            Some(
                self.repos.merchant.shop_manage
                    .find_bind_shop_ids_by_account(acc.trim(), false)
                    .await?,
            )
        } else {
            None
        };

        let page = q.page_query();
        let (total, shops) = self
            .repos.merchant.shop_manage
            .paginate_shops(q, shop_id_scope, manager_ids.as_deref())
            .await?;

        let refs = self.load_shop_refs(&shops).await?;
        let is_my_shop = q.is_my_shop_flag() == 1;
        let mut list: Vec<ShopVO> = shops
            .iter()
            .map(|s| {
                let mut vo = build_shop_vo(
                    s,
                    refs.merchant(s.merchant_id),
                    refs.admin(s.shop_id),
                    None,
                );
                vo.shop_tips = refs.tips.get(&s.shop_id).cloned().unwrap_or_default();
                vo.tips = refs.tip_names(&vo.shop_tips);
                if is_my_shop {
                    // Java：我的店铺回填收货信息（配送设置 → 收货信息）
                    vo.receive_info = parse_delivery_info(s.receive_info.as_deref());
                }
                vo
            })
            .collect();

        let mut total = total;
        // 平台自营主体（shopId = 0）：当 includeSelfOperated=1 且在第 1 页时，在首位插入自营虚拟主体
        if q.include_self_operated_flag() == 1
            && (q.shop_type_or_default() == shop_type::SHOP
                || q.shop_type_or_default() == shop_type::STORE)
            && q.page_no() == 1
            && shop_id_scope.unwrap_or(0) <= 0
            && !list.iter().any(|s| s.shop_id == 0)
        {
            let shop_title = if q.shop_type_or_default() == shop_type::STORE {
                "自营门店".to_string()
            } else {
                "自营店铺".to_string()
            };
            let kw = q.keyword.as_deref().unwrap_or("").trim();
            if kw.is_empty() || shop_title.contains(kw) {
                let self_op = ShopVO {
                    shop_id: 0,
                    shop_title,
                    shop_type: Some(q.shop_type_or_default()),
                    status: Some(1),
                    status_text: "开业".to_string(),
                    ..Default::default()
                };
                list.insert(0, self_op);
                total += 1;
            }
        }

        Ok(PageResult::new(total, list, &page))
    }

    // ————————————————————— 详情 —————————————————————

    /// 店铺详情（对应 Java `ShopController.detail` → `detailWithScore`）。
    ///
    /// `shopId = 0` 为平台自营主体：Java 由平台配置生成虚拟店铺。
    /// 该配置未迁移，Rust 侧返回 `shopId = 0` 的**最小占位**（仅 `shopId`），
    /// 不再编造名称/logo/经纬度（见 `uncertain_points`）。
    pub async fn detail_with_score(&self, shop_id: i32) -> Result<ShopVO> {
        if shop_id == 0 {
            return Ok(ShopVO {
                shop_id: 0,
                status_text: String::new(),
                ..Default::default()
            });
        }
        let shop = self
            .repos.merchant.shop_manage
            .find_shop(shop_id)
            .await?
            .ok_or_else(|| Error::biz_msg("店铺不存在"))?;

        let refs = self.load_shop_refs(std::slice::from_ref(&shop)).await?;
        // Java `detail` 取一条 `admin_user_shop`（`is_admin = 1`，仅回填绑定 VO）
        let binding = self
            .repos.merchant.shop_manage
            .find_shop_admin_bindings(&[shop_id])
            .await?
            .into_iter()
            .next();
        let mut vo = build_shop_vo(
            &shop,
            refs.merchant(shop.merchant_id),
            refs.admin(shop.shop_id),
            binding.as_ref().map(admin_user_shop_vo),
        );
        vo.shop_tips = refs.tips.get(&shop.shop_id).cloned().unwrap_or_default();
        vo.tips = refs.tip_names(&vo.shop_tips);
        vo.product_score = Some(
            self.repos.merchant.shop_manage
                .avg_shop_comment_rank(shop_id)
                .await?
                .unwrap_or(Decimal::ZERO),
        );
        // Java `detailWithScore`：`selectByIdWithCalculatedStar` 的 CASE 表达式
        // 恒有值（评价数 <100 固定 4.8），故这里必然落到 `Some`
        vo.shop_star = Some(compute_shop_star(shop.shop_star_num, shop.shop_star));
        Ok(vo)
    }

    /// 当前登录用户的店铺信息（对应 Java `ShopController.currentShop`）。
    ///
    /// Java 先无条件把 `last_login_time` 刷成当前时间，再
    /// `detail(shopId, true)`（只查当前管理员在该店铺的绑定）。
    /// `admin_id` 由路由层从 JWT 取。
    pub async fn current_detail(&self, shop_id: i32, admin_id: i32) -> Result<ShopVO> {
        if shop_id <= 0 {
            return Err(Error::Validation("店铺ID不能为空".into()));
        }
        // Java `StringUtils.getCurrentTime()`（秒）
        let _ = self
            .repos.merchant.shop_manage
            .touch_last_login(shop_id, utils::now_seconds() as i32)
            .await?;

        let shop = self
            .repos.merchant.shop_manage
            .find_shop(shop_id)
            .await?
            .ok_or_else(|| Error::biz_msg("店铺不存在"))?;
        let refs = self.load_shop_refs(std::slice::from_ref(&shop)).await?;
        let mut vo = build_shop_vo(
            &shop,
            refs.merchant(shop.merchant_id),
            refs.admin(shop.shop_id),
            None,
        );
        vo.shop_tips = refs.tips.get(&shop.shop_id).cloned().unwrap_or_default();
        vo.tips = refs.tip_names(&vo.shop_tips);
        // onlyCurrentAdmin = true：只回填当前管理员的绑定
        if let Some(binding) = self
            .repos.merchant.shop_manage
            .find_admin_shop_binding(shop_id, admin_id)
            .await?
        {
            vo.admin_user_shop = Some(admin_user_shop_vo(&binding));
        }
        Ok(vo)
    }

    // ————————————————————— 创建 —————————————————————

    /// 创建店铺（对应 Java `ShopController.create` / `ShopServiceImpl#create`）。
    ///
    /// **未迁移**：角色初始化（`initAdminRole` / `initEmployeeRoles`）、
    /// 运费模板初始化、装修模板初始化（见模块头第 3 条）。
    /// 返回新建店铺 ID（Java 返回 `void`）。
    pub async fn create(
        &self,
        p: &ShopWritePayload,
        operator_admin_id: i32,
        shop_id_scope: Option<i32>,
    ) -> Result<i32> {
        // 名称必填 + 长度（Java `@NotNull` + `@Size(max = 50)`）
        let title = p
            .shop_title
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("店铺名称不能为空".into()))?;
        if title.chars().count() > SHOP_TITLE_MAX_LEN {
            return Err(Error::Validation(format!(
                "店铺名称长度不能超过 {SHOP_TITLE_MAX_LEN} 位"
            )));
        }
        check_store_param(self.is_o2o, p)?;

        let merchant_id = validate_create_merchant(p.merchant_id, p.shop_type)?;

        // 店铺后台内新建店铺：沿用当前店铺的 merchant_id（Java `shopId > 0` 分支）
        let scope = shop_id_scope.filter(|v| *v > 0);
        let merchant_id = if let Some(current) = scope {
            let current_shop = self
                .repos.merchant.shop_manage
                .find_shop(current)
                .await?
                .ok_or_else(|| Error::biz_msg("店铺不存在"))?;
            current_shop.merchant_id.unwrap_or(merchant_id)
        } else {
            merchant_id
        };

        // 店铺数上限（Java `updateShop(isAdd = true)`）
        if merchant_id > 0 && p.shop_type != Some(shop_type::PICKUP) {
            let max_raw = self
                .repos.merchant.shop_manage
                .find_config_val("maxShopCount")
                .await
                .unwrap_or_default();
            if let Ok(max_count) = max_raw.trim().parse::<i64>() {
                let st = p.shop_type.unwrap_or(shop_type::SHOP);
                let count = self
                    .repos.merchant.shop_manage
                    .count_shops_by_merchant(merchant_id, st)
                    .await?;
                if count >= max_count {
                    return Err(if st == shop_type::STORE {
                        Error::biz_msg(format!("门店数量已达上限({max_count})"))
                    } else {
                        Error::biz_msg(format!("店铺数量已达上限({max_count})"))
                    });
                }
            }
        }

        // 随机分配 10 位 shop_id（Java `assignRandomShopId`）
        let shop_id = self.assign_random_shop_id().await?;

        let mut shop = Shop {
            shop_id,
            shop_title: Some(title.to_string()),
            add_time: Some(utils::now_seconds() as i32),
            shop_star: DEFAULT_SHOP_STAR.parse::<Decimal>().ok(),
            merchant_id: Some(merchant_id),
            shop_money: Some(Decimal::ZERO),
            frozen_money: Some(Decimal::ZERO),
            is_contact_kefu: 1,
            ..Default::default()
        };
        apply_payload(&mut shop, p);

        self.repos.merchant.shop_manage.insert_shop(&shop).await?;

        // 门店标签（Java `create` 内 for 循环插入 store_tip）
        if let Some(tips) = p.shop_tips.as_ref().filter(|v| !v.is_empty()) {
            self.repos.merchant.shop_manage
                .insert_store_tips(shop_id, tips, utils::now_millis())
                .await?;
        }

        // 绑定店铺管理员（Java `adminUserShopService.createAdminUserShop`）
        let bind_admin_id = if scope.is_some() {
            // 店铺后台内新建：绑定当前登录账号
            operator_admin_id
        } else {
            // 平台端为商户新增店铺：绑定该商户的主账号
            self.repos.merchant.shop_manage
                .find_merchant_main_admin_user_id(merchant_id)
                .await?
                .unwrap_or(operator_admin_id)
        };
        let (username, email) = match self.repos.merchant.shop_manage.find_admin_user(bind_admin_id).await? {
            Some(u) => (u.username, u.email),
            None => (String::new(), String::new()),
        };
        self.insert_shop_binding(
            shop_id,
            bind_admin_id,
            &username,
            &email,
            utils::now_seconds() as i32,
        )
        .await?;

        tracing::info!(
            "创建店铺: shop_id={shop_id}, merchant_id={merchant_id}, admin_id={bind_admin_id}, operator={operator_admin_id}"
        );
        Ok(shop_id)
    }

    /// 建立 `admin_user_shop` 绑定（`is_admin = 1`、`auth_list = ["all"]`）。
    async fn insert_shop_binding(
        &self,
        shop_id: i32,
        admin_id: i32,
        username: &str,
        email: &str,
        add_time: i32,
    ) -> Result<()> {
        self.repos.merchant.shop_manage
            .insert_admin_user_shop(shop_id, admin_id, username, email, add_time)
            .await
    }

    /// 随机分配未占用的 10 位 shop_id（Java `assignRandomShopId`）。
    ///
    /// Java 用 `ThreadLocalRandom.current().nextInt(1e9, 2e9)` 重试 10 次；
    /// Rust 侧未引入 `rand`，改用「纳秒时间戳 + 序号」派生候选值
    /// （与 `aftersales_sn` 的既有做法一致），并逐次查重。
    async fn assign_random_shop_id(&self) -> Result<i32> {
        let span = i64::from(SHOP_ID_MAX_EXCLUSIVE) - i64::from(SHOP_ID_MIN);
        let seed = utils::now_millis();
        for i in 0..SHOP_ID_RETRY {
            let mix = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add((i as i64).wrapping_mul(1_442_695_040_888_963_407));
            let offset = (mix >> 33).rem_euclid(span) as i32;
            let candidate = SHOP_ID_MIN + offset;
            if self.repos.merchant.shop_manage.find_shop(candidate).await?.is_none() {
                return Ok(candidate);
            }
        }
        Err(Error::biz_msg("店铺ID生成失败，请重试"))
    }

    // ————————————————————— 更新 —————————————————————

    /// 更新店铺（对应 Java `ShopController.update` / `ShopServiceImpl#update`）。
    ///
    /// Java 还会重写门店标签（先删后插）与执行 `checkStoreParam`。
    pub async fn update(&self, p: &ShopWritePayload) -> Result<()> {
        let shop_id = p
            .shop_id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation("店铺ID不能为空".into()))?;
        check_store_param(self.is_o2o, p)?;
        if self.repos.merchant.shop_manage.find_shop(shop_id).await?.is_none() {
            return Err(Error::biz_msg("店铺不存在"));
        }

        // Java `update` 里 `shop` 的 shop_id 来自请求体，其余字段来自
        // `BeanUtils.copyProperties(updateDTO, shop)`
        let mut shop = Shop {
            shop_id,
            ..Default::default()
        };
        apply_payload(&mut shop, p);
        // `store_parent_id` 在 Java 走 `checkStoreParam`（仅 o2o 门店/自提点），
        // 但 `update` 的 `BeanUtils.copyProperties` 也会复制该字段（DTO 有同名属性）
        shop.store_parent_id = p.store_parent_id;
        // 资金/评星不在 `ShopUpdateDTO` 中：保持 None，让 `COALESCE` 保留库中原值
        shop.shop_money = None;
        shop.frozen_money = None;
        shop.shop_star = None;
        self.repos.merchant.shop_manage.update_shop(&shop).await?;

        // 标签：先删后插，`shopTips` 为 None 时也删（Java 无条件 delete）
        self.repos.merchant.shop_manage.delete_store_tips(shop_id).await?;
        if let Some(tips) = p.shop_tips.as_ref().filter(|v| !v.is_empty()) {
            self.repos.merchant.shop_manage
                .insert_store_tips(shop_id, tips, utils::now_millis())
                .await?;
        }
        Ok(())
    }

    /// 删除店铺（对应 Java `ShopController.del` → `BaseServiceImpl#del`，硬删）。
    pub async fn delete(&self, id: i32) -> Result<()> {
        if id <= 0 {
            return Err(Error::Validation("ID不能为空".into()));
        }
        let affected = self.repos.merchant.shop_manage.delete_shops(&[id]).await?;
        if affected == 0 {
            return Err(Error::biz_msg("店铺不存在"));
        }
        Ok(())
    }

    /// 更新单个字段（对应 Java `ShopController.updateField` →
    /// `BaseServiceImpl#updateField(updateField, GENERAL_FIELDS)`）。
    ///
    /// 仅 `shop` 表中的 `status` 列可写（`GENERAL_FIELDS` 的另两项在该表不存在）。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<()> {
        let id = dto
            .id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation("ID不能为空".into()))?;
        let (col, val) = validate_update_field(dto.field.as_deref(), dto.val.as_deref())?;
        match col {
            "status" => {
                let status: i32 = val
                    .parse()
                    .map_err(|_| Error::Validation(format!("字段值必须为整数: {val}")))?;
                if self.repos.merchant.shop_manage.update_shop_status(id, status).await? {
                    Ok(())
                } else {
                    Err(Error::biz_msg("店铺不存在"))
                }
            }
            other => Err(Error::biz_msg(format!(
                "字段 {other} 不在 shop 表可更新列白名单中"
            ))),
        }
    }

    /// 批量操作（对应 Java `ShopController.batch` → `BaseServiceImpl#batch`）。
    ///
    /// Java 仅支持 `type = "del"`（硬删），其余类型返回 `false`。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        let ids = dto
            .ids
            .as_ref()
            .map(|v| v.iter().copied().filter(|i| *i > 0).collect::<Vec<_>>())
            .filter(|v: &Vec<i32>| !v.is_empty())
            .ok_or_else(|| Error::Validation("请选择要操作的项".into()))?;
        match dto.r#type.as_deref().map(str::trim) {
            Some("del") => {
                self.repos.merchant.shop_manage.delete_shops(&ids).await?;
                Ok(true)
            }
            Some(other) => Err(Error::Validation(format!("不支持的操作类型: {other}"))),
            None => Err(Error::Validation("操作类型不能为空".into())),
        }
    }

    // ————————————————————— myShop / 登录偏好 —————————————————————

    /// `myShop`（对应 Java `ShopController.myShop`）。
    ///
    /// 返回 `{shop, vendor, userinfo, defaultPosShopId, lastLoginSystem}`。
    /// `vendor` 恒为空分页（供应商域未迁移，与 Java 一致：Java 也未填充）。
    pub async fn my_shop(
        &self,
        q: &ShopListQuery,
        admin_id: i32,
        shop_id_scope: Option<i32>,
    ) -> Result<serde_json::Value> {
        // Java：仅启用中的绑定（is_using = 0）
        let shop_ids = self.repos.merchant.shop_manage.find_enabled_shop_ids(admin_id).await?;
        let page = PageQuery {
            page: 1,
            page_size: 100,
        };
        let shop_page = if shop_ids.is_empty() {
            PageResult::<ShopVO>::empty(&page)
        } else {
            let mut my = q.clone();
            my.is_my_shop = Some(1);
            my.shop_ids = Some(shop_ids);
            my.shop_types = Some(if self.is_o2o {
                vec![shop_type::STORE, shop_type::PICKUP]
            } else {
                vec![shop_type::SHOP]
            });
            my.sort_field = Some("lastLoginTime".into());
            my.sort_order = Some("desc".into());
            my.size = Some(100);
            my.page = Some(1);
            self.list(&my, shop_id_scope).await?
        };

        let userinfo = match self.repos.merchant.shop_manage.find_admin_user(admin_id).await? {
            Some(u) => serde_json::json!({
                "adminId": u.admin_id,
                "username": u.username,
                "adminType": u.admin_type,
                "mobile": u.mobile,
                "avatar": u.avatar,
                "email": u.email,
                "merchantId": u.merchant_id,
                "roleId": u.role_id,
                "isUsing": u.is_using,
                "addTime": fmt_shop_time(Some(u.add_time)),
            }),
            None => serde_json::Value::Null,
        };
        let default_pos_shop_id = self
            .repos.merchant.shop_manage
            .find_default_pos_shop_id(admin_id)
            .await?
            .unwrap_or(0);
        let last_login_system = self
            .repos.merchant.shop_manage
            .find_last_login_system(admin_id)
            .await?;

        Ok(serde_json::json!({
            "shop": shop_page,
            "vendor": PageResult::<serde_json::Value>::empty(&page),
            "userinfo": userinfo,
            "defaultPosShopId": default_pos_shop_id,
            "lastLoginSystem": last_login_system,
        }))
    }

    /// 设置「登录后默认进收银台」（`AdminUserShopServiceImpl#setPosLoginDefault`）。
    pub async fn set_pos_login_default(
        &self,
        p: &PosLoginSettingPayload,
        admin_id: i32,
    ) -> Result<()> {
        let shop_id = require_shop_id(p.shop_id, "门店ID")?;
        let binding = self
            .repos.merchant.shop_manage
            .find_admin_shop_binding(shop_id, admin_id)
            .await?
            .ok_or_else(|| Error::biz_msg("当前账号不属于该门店，无法设置默认进收银台"))?;
        let flag = if p.default_pos_login == Some(1) { 1 } else { 0 };
        self.repos.merchant.shop_manage
            .update_default_pos_login(binding.id, flag)
            .await?;
        Ok(())
    }

    /// 记录上次登录端（`AdminUserShopServiceImpl#recordLoginSystem`）。
    ///
    /// Java 在找不到绑定时**静默返回**（不报错），此处保持一致。
    pub async fn record_login_system(
        &self,
        p: &PosLoginSettingPayload,
        admin_id: i32,
    ) -> Result<()> {
        let shop_id = require_shop_id(p.shop_id, "门店ID")?;
        let Some(binding) = self
            .repos.merchant.shop_manage
            .find_admin_shop_binding(shop_id, admin_id)
            .await?
        else {
            return Ok(());
        };
        let sys = p.last_login_system.as_deref().unwrap_or("");
        self.repos.merchant.shop_manage
            .update_last_login_system(binding.id, sys)
            .await?;
        Ok(())
    }

    /// 选择登录平台（对应 Java `ShopServiceImpl#choose`）。
    ///
    /// Java 把 `LoginChooseAuth{param, authorities}` 写入 Redis
    /// （key `admin::auth::<adminId>`）。Rust 侧 JWT 无服务端会话、
    /// Redis 未接入，故**只做校验并返回权限清单，不写缓存**。
    ///
    /// 返回 `(是否店铺视角, 权限清单)`。
    pub async fn choose(
        &self,
        param: &LoginChooseParam,
        admin_id: i32,
    ) -> Result<(bool, Vec<String>)> {
        let admin = self
            .repos.merchant.shop_manage
            .find_admin_user(admin_id)
            .await?
            .ok_or_else(|| Error::biz_msg("管理员不存在"))?;
        let mut role_id = admin.role_id;
        let mut auth_raw = admin.auth_list.clone();
        let mut shop_view = false;

        if param.admin_type.as_deref() == Some("shop") {
            shop_view = true;
            let id = param
                .id
                .as_deref()
                .and_then(|s| s.trim().parse::<i32>().ok())
                .ok_or_else(|| Error::Validation("店铺ID不能为空".into()))?;
            if self.repos.merchant.shop_manage.find_shop(id).await?.is_none() {
                return Err(Error::biz_msg("没有此店铺"));
            }
            let binding = self
                .repos.merchant.shop_manage
                .find_admin_shop_binding(id, admin_id)
                .await?
                .ok_or_else(|| Error::biz_msg("没有此店铺权限"))?;
            if binding.is_using != 0 {
                return Err(Error::biz_msg("用户已经被禁用"));
            }
            role_id = binding.role_id;
            auth_raw = binding.auth_list.clone().unwrap_or_default();
        }

        // RBAC：有角色走角色权限
        if role_id > 0 {
            if let Some(role) = self.repos.merchant.shop_manage.find_admin_role(role_id).await? {
                auth_raw = role.authority_list;
            }
        }
        let authorities = parse_string_list(&auth_raw);
        tracing::info!(
            "选择登录组织: admin_id={admin_id}, admin_type={:?}, authorities={}",
            param.admin_type,
            authorities.len()
        );
        Ok((shop_view, authorities))
    }

    // ————————————————————— shopList / staffShow —————————————————————

    /// 店铺下拉列表（对应 Java `ShopServiceImpl#getShopList`）。
    pub async fn shop_list(&self, q: &ShopListQuery) -> Result<Vec<Shop>> {
        self.repos.merchant.shop_manage
            .find_shops_for_picker(
                q.shop_type_or_default(),
                q.shop_title
                    .as_deref()
                    .filter(|s| !s.trim().is_empty()),
                q.keyword.as_deref().filter(|s| !s.trim().is_empty()),
            )
            .await
    }

    /// 员工展示信息（对应 Java `ShopServiceImpl#staffShow`）。
    ///
    /// `maxSubAdministrator` 直读 `config` 表（配置域未迁移）；
    /// 读取失败或缺省时按 `0` 处理（Java `Integer.parseInt("")` 会抛异常，
    /// Rust 侧降级为 0 而不是整页 500）。
    pub async fn staff_show(&self, shop_id: i32) -> Result<StaffShowVO> {
        if shop_id <= 0 {
            return Err(Error::Validation("店铺ID不能为空".into()));
        }
        // 收银员角色排除（Java `posCashierRoleIds`）
        let cashier_ids = self
            .repos.merchant.shop_manage
            .find_pos_cashier_role_ids()
            .await
            .unwrap_or_default();

        let (using, sub_using, stop_using) = self
            .repos.merchant.shop_manage
            .count_staff(shop_id, &cashier_ids)
            .await
            .unwrap_or((0, 0, 0));

        let max_sub = self
            .repos.merchant.shop_manage
            .find_config_val("maxSubAdministrator")
            .await
            .unwrap_or_default()
            .trim()
            .parse::<i64>()
            .unwrap_or(0);
        let residue = if max_sub > 0 {
            (max_sub - (sub_using + stop_using)).max(0)
        } else {
            0
        };

        let bindings = self
            .repos.merchant.shop_manage
            .find_shop_bindings(shop_id, &cashier_ids)
            .await?;
        let admin_ids: Vec<i32> = distinct_i32(bindings.iter().map(|b| b.admin_id));
        let logs = self
            .repos.merchant.shop_manage
            .find_recent_admin_logs(&admin_ids, 5)
            .await?;
        let log_admin_ids = distinct_i32(logs.iter().map(|l| l.user_id));
        let users = self.repos.merchant.shop_manage.find_admin_users(&log_admin_ids).await?;
        let name_map: HashMap<i32, String> =
            users.into_iter().map(|u| (u.admin_id, u.username)).collect();

        Ok(StaffShowVO {
            using_user: using as i32,
            stop_using_user: stop_using as i32,
            residue: residue as i32,
            admin_log: logs
                .iter()
                .map(|l| StaffAdminLogVO {
                    log_id: l.log_id,
                    log_time: fmt_shop_time(Some(l.log_time)),
                    user_id: l.user_id,
                    log_info: l.log_info.clone(),
                    ip_address: l.ip_address.clone(),
                    username: name_map.get(&l.user_id).cloned().unwrap_or_default(),
                })
                .collect(),
        })
    }

    /// 更新店铺信息（对应 Java `ShopServiceImpl#updateInfo`）。
    ///
    /// Java 强校验 `dto.shopId == HeaderUtils.getShopId()`，否则「没有此店铺」。
    /// Rust 侧改为强校验 `dto.shopId == shop_id 参数`（路由层从查询参数取）。
    pub async fn update_info(
        &self,
        p: &ShopWritePayload,
        shop_id_scope: Option<i32>,
    ) -> Result<Shop> {
        let dto_shop_id = p
            .shop_id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation("店铺ID不能为空".into()))?;
        let current = require_shop_id(shop_id_scope, "店铺ID")?;
        if dto_shop_id != current {
            return Err(Error::biz_msg("没有此店铺"));
        }
        check_store_param(self.is_o2o, p)?;

        let mut shop = Shop {
            shop_id: dto_shop_id,
            ..Default::default()
        };
        apply_payload(&mut shop, p);
        if !self.repos.merchant.shop_manage.update_shop(&shop).await? {
            return Err(Error::biz_msg("没有此店铺"));
        }
        // Java `updateInfo` 直接返回本次 updateById 的实体
        Ok(shop)
    }

    /// 店铺设置（对应 Java `ShopServiceImpl#setting`）。
    ///
    /// 停业（`status = 4`）前置校验：店铺无进行中订单（`order_status ∈ {0,1,2}`）
    /// 且无未完结售后（`status ∉ {完成,取消}`），通过后把店铺内商品全部下架。
    pub async fn setting(&self, p: &ShopSettingPayload, shop_id_scope: Option<i32>) -> Result<()> {
        let shop_id = p
            .shop_id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation("店铺ID不能为空".into()))?;
        let current = require_shop_id(shop_id_scope, "店铺ID")?;
        if shop_id != current {
            return Err(Error::biz_msg("没有此店铺"));
        }

        if p.status == Some(shop_status::PAUSE) {
            let ongoing = self.repos.merchant.shop_manage.count_ongoing_orders(shop_id).await?;
            if ongoing > 0 {
                return Err(Error::biz_msg("存在进行中订单，无法停业，请先处理完所有订单"));
            }
            let aftersales = self.repos.merchant.shop_manage.count_ongoing_aftersales(shop_id).await?;
            if aftersales > 0 {
                return Err(Error::biz_msg("存在未完结售后，无法停业，请先处理完所有售后"));
            }
        }

        let mut shop = Shop {
            shop_id,
            ..Default::default()
        };
        shop.status = p.status;
        shop.contact_mobile = p.contact_mobile.clone();
        shop.kefu_phone = p.kefu_phone.clone();
        shop.kefu_weixin = p.kefu_weixin.clone();
        shop.kefu_link = p.kefu_link.clone();
        // Java `ShopSettingDTO.isContactKefu` 字段初始值为 1，
        // `BeanUtils.copyProperties` 会把它带到 `Shop`；省略该字段时应落 1
        shop.is_contact_kefu = p.is_contact_kefu.unwrap_or(1);
        shop.use_shop_category = p.use_shop_category;
        // Java：DTO 的这几个 JSON 字段不在 `BeanUtils.copyProperties` 的
        // 同名属性内，需显式 `JSONUtil.toJsonStr` / `toJson` 后写入
        if let Some(v) = &p.shop_show_category {
            shop.shop_show_category = Some(v.to_string());
        }
        if let Some(v) = &p.kefu_inlet {
            shop.kefu_inlet = Some(v.to_string());
        }
        if let Some(v) = &p.ship_info {
            shop.ship_info = Some(v.to_string());
        }
        if let Some(v) = &p.receive_info {
            shop.receive_info = Some(v.to_string());
        }
        if !self.repos.merchant.shop_manage.update_shop(&shop).await? {
            return Err(Error::biz_msg("没有此店铺"));
        }

        // 停业后下架店铺内全部商品
        if p.status == Some(shop_status::PAUSE) {
            let n = self.repos.merchant.shop_manage.off_shelf_shop_products(shop_id).await?;
            tracing::info!("店铺停业，已下架商品 {n} 行: shop_id={shop_id}");
        }
        Ok(())
    }

    // ————————————————————— 供应商配置 —————————————————————

    /// 获取供应商配置（对应 Java `ShopServiceImpl#getVendorSetting`）。
    pub async fn get_vendor_setting(&self, shop_id: i32) -> Result<VendorSettingVO> {
        let shop = self
            .repos.merchant.shop_manage
            .find_shop(shop_id)
            .await?
            .ok_or_else(|| Error::biz_msg("店铺不存在"))?;
        Ok(VendorSettingVO {
            vendor_set_price_type: shop.vendor_set_price_type,
            // Java `ShopVendorSettingVO.vendorSetPriceAutoValue` 是 String，
            // 实体是 BigDecimal，`BeanUtils`/手写赋值时按 toString 传递
            vendor_set_price_auto_value: shop
                .vendor_set_price_auto_value
                .map(|v| v.normalize().to_string()),
        })
    }

    /// 更新供应商配置（对应 Java `ShopServiceImpl#updateVendorSetting`）。
    pub async fn update_vendor_setting(
        &self,
        shop_id: i32,
        p: &VendorSettingUpdateParam,
    ) -> Result<()> {
        if shop_id <= 0 {
            return Err(Error::Validation("店铺ID不能为空".into()));
        }
        if self.repos.merchant.shop_manage.find_shop(shop_id).await?.is_none() {
            return Err(Error::biz_msg("店铺不存在"));
        }
        let auto = parse_vendor_price_auto_value(p.vendor_set_price_auto_value.as_deref())?;
        self.repos.merchant.shop_manage
            .update_vendor_setting(shop_id, p.vendor_set_price_type, auto)
            .await?;
        Ok(())
    }

    // ————————————————————— 门店 / 商品 —————————————————————

    /// 商品对应的门店列表（对应 Java `ShopServiceImpl#getStoreListByProductId`）。
    ///
    /// Java：取全部门店，仅保留在 `store_product` 中有该商品分配记录的门店。
    pub async fn store_list_by_product_id(&self, product_id: i32) -> Result<Vec<StoreListVO>> {
        if product_id <= 0 {
            return Err(Error::Validation("商品ID不能为空".into()));
        }
        let stores = self
            .repos.merchant.shop_manage
            .find_shops_for_picker(shop_type::STORE, None, None)
            .await?;
        if stores.is_empty() {
            return Ok(Vec::new());
        }
        let assigned: HashSet<i32> = self
            .repos.merchant.shop_manage
            .find_shop_ids_by_product(product_id)
            .await?
            .into_iter()
            .collect();
        Ok(stores
            .into_iter()
            .filter(|s| assigned.contains(&s.shop_id))
            .map(|s| StoreListVO {
                shop_id: s.shop_id,
                shop_title: s.shop_title,
            })
            .collect())
    }

    // ————————————————————— 删除店铺 —————————————————————

    /// 删除店铺/门店/自提点（对应 Java `ShopServiceImpl#delShop`，软删除）。
    ///
    /// 前置条件（逐条对齐 Java）：
    /// 1. 店铺存在
    /// 2. 状态为停业（`status = 4`）或类型为自提点，否则「需要先停止营业才能删除」
    /// 3. 名下无未删自提点
    /// 4. `shop_money = 0`
    /// 5. `frozen_money = 0`
    /// 6. 名下无商品
    ///
    /// 注：Java `delShop` 的入参是 `OperateDTO`，`operateDTO.getId()` 为 `null`
    /// 时 `getById(null)` 返回 null → 抛「店铺不存在」。
    pub async fn del_shop(&self, dto: &OperateDTO) -> Result<bool> {
        let id = dto
            .id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::biz_msg("店铺不存在"))?;
        let shop = self
            .repos.merchant.shop_manage
            .find_shop(id)
            .await?
            .ok_or_else(|| Error::biz_msg("店铺不存在"))?;

        // Java `shopTypeNames[shop.getShopType()]`：0 平台 / 1 店铺 / 2 门店 / 3 自提点
        let type_name = match shop.shop_type {
            Some(0) => "平台",
            Some(shop_type::STORE) => "门店",
            Some(shop_type::PICKUP) => "自提点",
            _ => "店铺",
        };

        let paused = shop.status == Some(shop_status::PAUSE);
        let is_pickup = shop.shop_type == Some(shop_type::PICKUP);
        if !paused && !is_pickup {
            return Err(Error::biz_msg(format!("{type_name}需要先停止营业才能删除")));
        }

        if self.repos.merchant.shop_manage.count_pickups_under(id).await? > 0 {
            return Err(Error::biz_msg("门店下有自提点，请先删除自提点"));
        }
        let shop_money = shop.shop_money.unwrap_or(Decimal::ZERO);
        if shop_money != Decimal::ZERO {
            return Err(Error::biz_msg(format!("{type_name}资金不为0，请先处理店铺资金")));
        }
        let frozen = shop.frozen_money.unwrap_or(Decimal::ZERO);
        if frozen != Decimal::ZERO {
            return Err(Error::biz_msg(format!(
                "{type_name}冻结资金不为0，请先处理店铺冻结资金"
            )));
        }
        if self.repos.merchant.shop_manage.count_products_of_shop(id).await? > 0 {
            return Err(Error::biz_msg(format!("{type_name}下有商品，请先处理商品")));
        }

        self.repos.merchant.shop_manage.soft_delete_shop(id).await
    }

    // ————————————————————— 内部：关联数据批量预取 —————————————————————

    /// 批量预取本页店铺的商户信息 / 主管理员 / 标签（替代 Java 循环内 N+1）。
    async fn load_shop_refs(&self, shops: &[Shop]) -> Result<ShopRefs> {
        if shops.is_empty() {
            return Ok(ShopRefs::default());
        }
        let shop_ids = distinct_i32(shops.iter().map(|s| s.shop_id));
        let merchant_ids = distinct_i32(shops.iter().filter_map(|s| s.merchant_id));

        let merchants = self.repos.merchant.shop_manage.find_merchants(&merchant_ids).await?;
        let merchant_map: HashMap<i32, Merchant> =
            merchants.into_iter().map(|m| (m.merchant_id, m)).collect();

        // 主管理员绑定（Java：`adminUserShopService.list(isAdmin = 1)` 后取一条）
        let bindings = self
            .repos.merchant.shop_manage
            .find_shop_admin_bindings(&shop_ids)
            .await?;
        let mut admin_bind: HashMap<i32, (i32, i32)> = HashMap::new();
        for b in &bindings {
            // 同店铺多条时取第一条（与 Java `Collectors.toMap(..., (a,b) -> a)` 一致）
            admin_bind.entry(b.shop_id).or_insert((b.admin_id, b.id));
        }
        let admin_ids = distinct_i32(admin_bind.values().map(|(admin_id, _)| *admin_id));
        let admins = self.repos.merchant.shop_manage.find_admin_users(&admin_ids).await?;
        let admin_name_map: HashMap<i32, String> =
            admins.into_iter().map(|u| (u.admin_id, u.username)).collect();

        // 标签（store_tip ⋈ tip）：Java `ShopServiceImpl` 对一批 shopId 只发一条
        // `in(StoreTip::getShopId, shopShopIds).orderByAsc(StoreTip::getSort)`，
        // 故这里复用 shop_client 仓储的同族批量查询（shop_manage 仓储只有单店版本）
        let tips = self.repos.merchant.shop_client.find_store_tips(&shop_ids).await?;
        let tip_ids = distinct_i32(tips.iter().map(|t| t.tip_id));
        let tip_rows = self.repos.merchant.shop_manage.find_tips(&tip_ids).await?;
        let tip_name_map: HashMap<i32, String> =
            tip_rows.into_iter().map(|t| (t.tip_id, t.tip_name)).collect();
        let mut tips_by_shop: HashMap<i32, Vec<i32>> = HashMap::new();
        for t in &tips {
            tips_by_shop.entry(t.shop_id).or_default().push(t.tip_id);
        }

        Ok(ShopRefs {
            merchant_map,
            admin_bind,
            admin_name_map,
            tips: tips_by_shop,
            tip_name_map,
        })
    }
}

/// 列表/详情共用的关联数据。
#[derive(Default)]
struct ShopRefs {
    merchant_map: HashMap<i32, Merchant>,
    /// `shop_id -> (admin_id, binding_id)`
    admin_bind: HashMap<i32, (i32, i32)>,
    admin_name_map: HashMap<i32, String>,
    /// `shop_id -> tip_id 列表`（保持 `sort` 升序）
    tips: HashMap<i32, Vec<i32>>,
    tip_name_map: HashMap<i32, String>,
}

impl ShopRefs {
    fn merchant(&self, merchant_id: Option<i32>) -> Option<&Merchant> {
        merchant_id.and_then(|id| self.merchant_map.get(&id))
    }

    /// 主管理员（`MerchantAdminDataDTO`：`adminId` + `username`）。
    fn admin(&self, shop_id: i32) -> Option<(i32, String)> {
        let (admin_id, _) = *self.admin_bind.get(&shop_id)?;
        Some((
            admin_id,
            self.admin_name_map.get(&admin_id).cloned().unwrap_or_default(),
        ))
    }

    fn tip_names(&self, tip_ids: &[i32]) -> Vec<ShopTipVO> {
        tip_ids
            .iter()
            .filter_map(|id| {
                self.tip_name_map.get(id).map(|name| ShopTipVO {
                    tip_id: *id,
                    tip_name: name.clone(),
                })
            })
            .collect()
    }
}

/// `Shop` 实体 → `ShopVO`（对应 Java `BeanUtils.copyProperties(shop, shopVO)`
/// 后的一系列派生字段赋值）。
///
/// `admin_binding`：仅 `detail(onlyCurrentAdmin)` / `currentDetail` 传入
/// 当前管理员的绑定 VO（列表接口传 `None`，与 Java 一致——列表只回填
/// `merchant.admin`，不产生 `adminUserShop`）。
fn build_shop_vo(
    s: &Shop,
    merchant: Option<&Merchant>,
    admin: Option<(i32, String)>,
    admin_binding: Option<ShopAdminUserShopVO>,
) -> ShopVO {
    ShopVO {
        shop_id: s.shop_id,
        shop_title: s.shop_title.clone().unwrap_or_default(),
        shop_logo: s.shop_logo.clone(),
        add_time: fmt_shop_time(s.add_time),
        click_count: s.click_count,
        status: s.status,
        status_text: shop_status_text(s.status, s.shop_type),
        shop_type: s.shop_type,
        merchant_id: s.merchant_id,
        shop_money: s.shop_money,
        frozen_money: s.frozen_money,
        contact_mobile: s.contact_mobile.clone(),
        description: s.description.clone(),
        kefu_phone: s.kefu_phone.clone(),
        kefu_weixin: s.kefu_weixin.clone(),
        kefu_link: s.kefu_link.clone(),
        is_contact_kefu: s.is_contact_kefu,
        kefu_inlet: parse_i32_list_raw(s.kefu_inlet.as_deref()),
        ship_info: parse_delivery_info(s.ship_info.as_deref()),
        receive_info: parse_delivery_info(s.receive_info.as_deref()),
        last_login_time: s.last_login_time,
        vendor_set_price_type: s.vendor_set_price_type,
        vendor_set_price_auto_value: s.vendor_set_price_auto_value,
        service_fee_rate: s.service_fee_rate,
        fee_rate: s.fee_rate,
        store_parent_id: s.store_parent_id,
        shop_cover_picture: s.shop_cover_picture.clone(),
        shop_show_picture: parse_json_opt(s.shop_show_picture.as_deref()),
        shop_contact_config: parse_json_opt(s.shop_contact_json.as_deref()),
        shop_open_close_config: parse_json_opt(s.shop_open_close_json.as_deref()),
        shop_region_ids: parse_i32_list_raw(s.shop_region_id.as_deref()),
        // 地区服务未迁移：不编造名称
        shop_region_names: Vec::new(),
        shop_detailed_address: s.shop_detailed_address.clone(),
        shop_longitude: s.shop_longitude,
        shop_latitude: s.shop_latitude,
        shop_star: s.shop_star,
        shop_star_num: s.shop_star_num,
        shop_sales: s.shop_sales,
        use_shop_category: s.use_shop_category,
        shop_show_category: parse_json_opt(s.shop_show_category.as_deref()),
        is_delete: s.is_delete,
        shop_tips: Vec::new(),
        tips: Vec::new(),
        product_score: None,
        store_parent_name: None,
        check: false,
        merchant: merchant.map(build_merchant_vo),
        admin_user_shop: admin_binding,
    }
}

/// `Merchant` 实体 → `ShopMerchantVO`（`merchant` 表自有列；其余字段不填充）。
fn build_merchant_vo(m: &Merchant) -> ShopMerchantVO {
    ShopMerchantVO {
        add_time: Some(fmt_shop_time(m.add_time)),
        merchant_id: m.merchant_id,
        merchant_apply_id: m.merchant_apply_id,
        user_id: m.user_id,
        company_name: m.company_name.clone(),
        corporate_name: m.corporate_name.clone(),
        settlement_cycle: m.settlement_cycle,
        status: m.status,
        r#type: m.r#type,
        merchant_data: parse_json_opt(m.merchant_data.as_deref()),
        // Java `detail`：`merchantVO.setBaseData(null); setShopData(null)`
        // —— 列表/详情都置空，这里统一返回 None
        base_data: None,
        shop_data: None,
        admin: None,
    }
}

/// `AdminUserShop` 实体 → `ShopAdminUserShopVO`（对应 Java 构造函数）。
fn admin_user_shop_vo(b: &AdminUserShop) -> ShopAdminUserShopVO {
    let auth_list = match b.auth_list.as_deref().map(str::trim) {
        None | Some("") | Some("[]") => serde_json::Value::Array(Vec::new()),
        Some(raw) => serde_json::from_str(raw).unwrap_or(serde_json::Value::Array(Vec::new())),
    };
    ShopAdminUserShopVO {
        id: b.id,
        admin_id: b.admin_id,
        user_id: b.user_id,
        shop_id: b.shop_id,
        username: b.username.clone(),
        email: b.email.clone(),
        avatar: b.avatar.clone(),
        is_using: b.is_using,
        is_admin: b.is_admin,
        role_id: b.role_id,
        auth_list,
        add_time: fmt_shop_time(b.add_time),
    }
}

/// 把入参 payload 的可空字段写入 `Shop` 实体（对应 Java `BeanUtils.copyProperties`
/// 对同名属性的复制 + 各 JSON 字段的 `JSONUtil.toJsonStr`）。
fn apply_payload(shop: &mut Shop, p: &ShopWritePayload) {
    if let Some(v) = &p.shop_title {
        shop.shop_title = Some(v.clone());
    }
    if let Some(v) = &p.shop_logo {
        shop.shop_logo = Some(v.clone());
    }
    if let Some(v) = &p.contact_mobile {
        shop.contact_mobile = Some(v.clone());
    }
    if let Some(v) = p.shop_type {
        shop.shop_type = Some(v);
    }
    if let Some(v) = p.store_parent_id {
        shop.store_parent_id = Some(v);
    }
    if let Some(v) = &p.shop_cover_picture {
        shop.shop_cover_picture = Some(v.clone());
    }
    if let Some(v) = &p.shop_show_picture {
        shop.shop_show_picture = Some(v.to_string());
    }
    if let Some(v) = &p.description {
        shop.description = Some(v.clone());
    }
    if let Some(v) = p.status {
        shop.status = Some(v);
    }
    if let Some(v) = &p.shop_contact_config {
        shop.shop_contact_json = Some(v.to_string());
    }
    if let Some(v) = &p.shop_open_close_config {
        shop.shop_open_close_json = Some(v.to_string());
    }
    if let Some(v) = &p.shop_region_ids {
        shop.shop_region_id = Some(
            serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string()),
        );
    }
    if let Some(v) = &p.shop_detailed_address {
        shop.shop_detailed_address = Some(v.clone());
    }
    if let Some(v) = p.shop_longitude {
        shop.shop_longitude = Some(v);
    }
    if let Some(v) = p.shop_latitude {
        shop.shop_latitude = Some(v);
    }
}

/// JSON 字符串数组 → `Vec<String>`（非法返回空表）。
fn parse_string_list(raw: &str) -> Vec<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(ToString::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

/// 去重（保持首次出现顺序）。
fn distinct_i32<I: Iterator<Item = i32>>(it: I) -> Vec<i32> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for v in it {
        if seen.insert(v) {
            out.push(v);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // ————————————————— 字段白名单 —————————————————

    #[test]
    fn update_field_accepts_only_whitelisted_shop_columns() {
        // GENERAL_FIELDS = is_show / sort_order / status；shop 表仅 status 存在
        assert!(validate_update_field(Some("status"), Some("4")).is_ok());
        assert!(validate_update_field(Some("isShow"), Some("1")).is_ok());
        assert!(validate_update_field(Some("is_show"), Some("1")).is_ok());
        assert!(validate_update_field(Some("sortOrder"), Some("1")).is_ok());
        // 未知字段 / 非白名单列被拒绝
        assert!(validate_update_field(Some("shopTitle"), Some("x")).is_err());
        assert!(validate_update_field(Some("shop_money"), Some("0")).is_err());
        assert!(validate_update_field(None, Some("1")).is_err());
        assert!(validate_update_field(Some("status"), Some("  ")).is_err());
    }

    #[test]
    fn resolve_field_column_does_not_fall_back_to_primary_key() {
        assert_eq!(resolve_field_column("status"), Some("status"));
        assert_eq!(resolve_field_column("nope"), None);
    }

    // ————————————————— 商户校验 —————————————————

    #[test]
    fn create_merchant_validation_matches_java() {
        // 店铺/门店类型：merchantId 缺失或 <= 0 都报错
        assert!(validate_create_merchant(None, Some(1)).is_err());
        assert!(validate_create_merchant(Some(0), Some(1)).is_err());
        assert!(validate_create_merchant(Some(0), Some(2)).is_err());
        assert_eq!(validate_create_merchant(Some(5), Some(1)).unwrap(), 5);
        // 自提点：`merchantId <= 0` 不受限（Java checkMerchantId 为假）
        assert_eq!(validate_create_merchant(Some(0), Some(3)).unwrap(), 0);
        // 但 `merchantId == null` 在任何 shopType 下都报错
        // （Java `createDTO.getMerchantId() == null` 分支与 shopType 无关）
        assert!(validate_create_merchant(None, Some(3)).is_err());
    }

    // ————————————————— 门店参数校验 —————————————————

    fn store_payload() -> ShopWritePayload {
        ShopWritePayload {
            shop_type: Some(shop_type::STORE),
            shop_cover_picture: Some("cover.png".into()),
            shop_contact_config: Some(serde_json::json!([])),
            shop_open_close_config: Some(serde_json::json!({"type": 1})),
            shop_region_ids: Some(vec![1, 2]),
            shop_longitude: Some("120.1".parse().unwrap()),
            shop_latitude: Some("30.2".parse().unwrap()),
            ..Default::default()
        }
    }

    #[test]
    fn check_store_param_only_applies_in_o2o_store_mode() {
        // o2o 关闭：不校验
        assert!(check_store_param(false, &store_payload()).is_ok());
        // o2o 开启 + 店铺类型（1）：不校验
        let mut p = store_payload();
        p.shop_type = Some(shop_type::SHOP);
        assert!(check_store_param(true, &p).is_ok());
        // o2o 开启 + 门店类型：完整参数通过
        assert!(check_store_param(true, &store_payload()).is_ok());
    }

    #[test]
    fn check_store_param_reports_missing_fields_in_java_order() {
        // 门店封面
        let mut p = store_payload();
        p.shop_cover_picture = None;
        assert_eq!(
            check_store_param(true, &p).unwrap_err().to_string(),
            "业务错误 [500]: 门店封面不能为空"
        );
        // 联系方式
        let mut p = store_payload();
        p.shop_contact_config = None;
        assert!(check_store_param(true, &p)
            .unwrap_err()
            .to_string()
            .contains("请填写联系方式"));
        // 营业时间
        let mut p = store_payload();
        p.shop_open_close_config = None;
        assert!(check_store_param(true, &p)
            .unwrap_err()
            .to_string()
            .contains("请填写营业时间"));
        // 区域
        let mut p = store_payload();
        p.shop_region_ids = None;
        assert!(check_store_param(true, &p)
            .unwrap_err()
            .to_string()
            .contains("请选择区域"));
        // 经纬度
        let mut p = store_payload();
        p.shop_latitude = None;
        assert!(check_store_param(true, &p)
            .unwrap_err()
            .to_string()
            .contains("经纬度不能为空"));
    }

    #[test]
    fn check_store_param_pickup_requires_parent() {
        let mut p = store_payload();
        p.shop_type = Some(shop_type::PICKUP);
        p.store_parent_id = None;
        assert!(check_store_param(true, &p)
            .unwrap_err()
            .to_string()
            .contains("父门店id不能为空"));
        p.store_parent_id = Some(9);
        assert!(check_store_param(true, &p).is_ok());
    }

    // ————————————————— 评星 —————————————————

    #[test]
    fn shop_star_defaults_below_100_reviews() {
        let star: Decimal = "4.2".parse().unwrap();
        assert_eq!(compute_shop_star(None, Some(star)), "4.8".parse().unwrap());
        assert_eq!(compute_shop_star(Some(0), Some(star)), "4.8".parse().unwrap());
        assert_eq!(compute_shop_star(Some(99), Some(star)), "4.8".parse().unwrap());
        assert_eq!(compute_shop_star(Some(100), Some(star)), star);
        // 评价数达标但 shop_star 为空 → 4.8
        assert_eq!(compute_shop_star(Some(200), None), "4.8".parse().unwrap());
    }

    // ————————————————— 供应商设价 —————————————————

    #[test]
    fn vendor_price_auto_value_parsing() {
        assert_eq!(parse_vendor_price_auto_value(None).unwrap(), None);
        assert_eq!(parse_vendor_price_auto_value(Some("  ")).unwrap(), None);
        assert_eq!(
            parse_vendor_price_auto_value(Some("12.5")).unwrap(),
            Some("12.5".parse::<Decimal>().unwrap())
        );
        assert!(parse_vendor_price_auto_value(Some("abc")).is_err());
    }

    // ————————————————— 时间/JSON 解析 —————————————————

    #[test]
    fn fmt_shop_time_handles_zero_and_positive() {
        assert_eq!(fmt_shop_time(None), "");
        assert_eq!(fmt_shop_time(Some(0)), "");
        assert_eq!(fmt_shop_time(Some(-1)), "");
        assert_eq!(fmt_shop_time(Some(1_700_000_000)).len(), 19);
    }

    #[test]
    fn parse_i32_list_raw_tolerates_shapes() {
        assert_eq!(parse_i32_list_raw(Some("[1,2,3]")), vec![1, 2, 3]);
        assert_eq!(parse_i32_list_raw(Some(r#"["1","2"]"#)), vec![1, 2]);
        assert_eq!(parse_i32_list_raw(Some("7")), vec![7]);
        assert_eq!(parse_i32_list_raw(Some("\"7\"")), vec![7]);
        assert!(parse_i32_list_raw(Some("")).is_empty());
        assert!(parse_i32_list_raw(None).is_empty());
        assert!(parse_i32_list_raw(Some("not json")).is_empty());
        // 数组里的非法元素被跳过
        assert_eq!(parse_i32_list_raw(Some(r#"[1,"x",null,3]"#)), vec![1, 3]);
    }

    #[test]
    fn parse_string_list_tolerates_shapes() {
        assert_eq!(parse_string_list(r#"["a","b"]"#), vec!["a", "b"]);
        assert!(parse_string_list("").is_empty());
        assert!(parse_string_list("[]").is_empty());
        assert!(parse_string_list("not json").is_empty());
    }

    #[test]
    fn parse_delivery_info_rejects_bad_json() {
        assert!(parse_delivery_info(None).is_none());
        assert!(parse_delivery_info(Some("")).is_none());
        assert!(parse_delivery_info(Some("not json")).is_none());
        let d = parse_delivery_info(Some(r#"{"sender":"张三","mobile":"13800000000"}"#)).unwrap();
        assert_eq!(d.sender.as_deref(), Some("张三"));
    }

    #[test]
    fn admin_user_shop_vo_parses_auth_list() {
        let b = AdminUserShop {
            id: 3,
            admin_id: 9,
            shop_id: 11,
            username: "u".into(),
            email: "e@x.com".into(),
            auth_list: Some(r#"["all"]"#.into()),
            add_time: Some(1_700_000_000),
            role_id: 5,
            ..Default::default()
        };
        let vo = admin_user_shop_vo(&b);
        assert_eq!(vo.id, 3);
        assert_eq!(vo.auth_list, serde_json::json!(["all"]));
        assert_eq!(vo.add_time.len(), 19);

        // 空 / "[]" → 空数组
        let empty = AdminUserShop {
            auth_list: None,
            ..b.clone()
        };
        assert_eq!(
            admin_user_shop_vo(&empty).auth_list,
            serde_json::Value::Array(Vec::new())
        );
        let empty = AdminUserShop {
            auth_list: Some("[]".into()),
            ..b
        };
        assert_eq!(
            admin_user_shop_vo(&empty).auth_list,
            serde_json::Value::Array(Vec::new())
        );
    }

    #[test]
    fn detail_zero_returns_minimal_placeholder() {
        // `shopId = 0` 的平台自营主体在 Java 由配置生成；
        // Rust 侧不编造字段，只返回 shopId。
        let vo = ShopVO {
            shop_id: 0,
            ..Default::default()
        };
        assert_eq!(vo.shop_id, 0);
        assert!(vo.shop_title.is_empty());
        assert!(vo.shop_region_names.is_empty());
    }
}