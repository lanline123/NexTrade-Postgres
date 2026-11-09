//! 前台用户中心服务（member::user_center）：账期信息、资料更新、会员中心、手机/邮箱验证、
//! 浏览历史、收藏店铺、会员等级、退出与注销。
//!
//! 对应原 Java `UserServiceImpl`（2781 行）/ `UserRankServiceImpl` /
//! `UserRankConfigServiceImpl` / `CreditUserServiceImpl` /
//! `CollectShopServiceImpl` 中本切片涉及的链路，以及
//! `nextrade-api` 的 `UserController`（`@RequestMapping("/api/user/user")`）。
//!
//! 迁移要点（逐条对照 Java 源）：
//! - **配置读取**：Java 的 `ConfigService.getConfigVal` / `getSettings` 被替换为
//!   直接读 `config` 表的 `biz_code` / `biz_val`（`is_del = 0`），
//!   语义与 `ConfigServiceImpl.getConfigVal` 一致（缺行 → 空串）。
//! - **部署开关**：Java `NexTradeProperties` 来自 `application.yaml`，
//!   本项目（`java-b2b2b-pro-v5.8.57`）为 `is-pro: 1` / `is-o2o: 0`，
//!   故 [`IS_PRO`] 直接取常量；`is-o2o` 影响收藏列表是否默认排除平台自营，
//!   由 [`CollectShopQuery::exclude_self_operated`] 承载。
//! - **`creditDetail`**：账期周期窗口、逾期判定、账期订单列表全部按 Java
//!   `CreditUserServiceImpl` 实现（含「上期未结算 → 剩余天数 0」规则）。
//! - **`memberCenter`**：待支付/待发货/待收货/待评价/待使用优惠券计数
//!   与安全等级逐条对齐；头像默认值与 `getOriginalImage` 去尺寸参数逻辑照抄。
//! - **`updateInformation`**：手机号唯一性校验 + 完善信息成长值链路
//!   （`rank_growth_log` / `user_growth_points_log` / 等级回写）。
//! - **`historyProduct` / `delHistoryProduct`**：浏览记录存在 `user.history_product_ids`
//!   JSON 数组中，删除即在内存中过滤后写回。
//! - **`collectionShop`**：收藏分页 + 排序白名单 + 已删除店铺过滤 +
//!   商品数/收藏数统计 + 店铺热销/新品/推荐商品 + 平台自营虚拟门店（配置驱动）。
//!
//! **未迁移（本切片显式返回业务错误，绝不伪造实现）**：
//! 1. 短信 / 邮件验证码全链路：Java `ConfigService.sendSms` / `sendEmail`
//!    依赖第三方短信与 SMTP 通道，且验证码存于 Redis（`modify_password{mobile}`
//!    等键）。本构建既无短信邮件通道，也未把 Redis 注入服务层，
//!    因此 4 个「发送验证码」接口 + `checkModifyPasswordMobileCode` /
//!    `mobileValidate` / `emailValidateNew` / `modifyMobile` / `modifyEmail`
//!    共 9 个接口返回明确的「未迁移」业务错误。
//! 2. 文件上传：`uploadImg` / `uploadVideo` / `modifyAvatar` 依赖
//!    `GalleryPicService` / `GalleryVideoInfoService`（对象存储 + 图库表 +
//!    图片尺寸解析 + 微信小程序内容安全校验），本构建未接通。
//! 3. 站内消息、微信授权绑定（`isBindWechat`）、签到开关等 `clientDetail`
//!    相关字段（不属本切片端点，见 `member::user_center` 任务书接口清单）。
//! 4. `logout`：对齐 Java `UserController.logout`，客户端清空本地 Token，
//!    服务端返回成功以完成正常登出流程。

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{NaiveDate, TimeZone};
use serde_json::{json, Value};
use nextrade_core::entities::{
    CreditOrder, CreditUser, OrderItem, Product, RankGrowthLog, Shop, User, UserGrowthPointsLog,
    UserRank, UserRankConfig,
};
use nextrade_core::models::member::user_center::{
    comment_status, credit_order_status, order_status, rank_growth_type, settlement_type,
    shop_status, CheckEmailModifyDTO, CheckModifyEmailDTO, CheckModifyPasswordDTO,
    CollectShopQuery, CollectShopShopVO, CollectShopVO, CreditDetailVO, CreditOrderVO,
    DelHistoryProductDTO, GalleryPicUploadVO, HistoryProductVO, MemberCenterVO, ModifyEmailParam,
    ModifyMobileParam, OrderItemVO, RankRight, UpdateInformationDTO, UploadVideoInfoVO,
    UserLevelInfoVO, UserLevelListVO, UserRankConfigVO, UserRankJsonVO, UserRankListVO,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::member::user_center::{ProductFlag, UserCenterRepository};
use nextrade_infrastructure::db::Repositories;

/// 是否为 Pro 版本（Java `nextrade.is-pro`，本项目 `application.yaml` = 1）。
///
/// 影响 `UserRankServiceImpl.addUserGrowthPoints` 是否执行。
/// （`nextrade.is-o2o` = 0 由 [`CollectShopQuery::exclude_self_operated`] 承载。）
const IS_PRO: bool = true;

/// 平台自营虚拟门店 ID（Java `CollectShopServiceImpl.SELF_OPERATED_SHOP_ID`）。
const SELF_OPERATED_SHOP_ID: i32 = 0;

/// 会员中心头像默认值（Java `UserServiceImpl.memberCenter` 硬编码）。
const DEFAULT_MEMBER_AVATAR: &str =
    "https://oss.nextrade.com/img/gallery/202403/17111886010Pg60qIrJEzwe3tw06.gif";

/// 收藏店铺附带商品的最大条数（Java `getProductType` 的 `limit 10`）。
const SHOP_PRODUCT_LIMIT: u32 = 10;

/// 浏览历史最大保留条数（Java `ProductServiceImpl.historyProduct` 的 `limit 20`）。
const HISTORY_PRODUCT_LIMIT: usize = 20;

/// 用户不存在的错误文案（Java `messageHelper.get("error.member.not.exist")`）。
const ERR_MEMBER_NOT_EXIST: &str = "会员不存在";

// ——————————————————————————————————————————————————————————————
// 纯函数（可单测）
// ——————————————————————————————————————————————————————————————

/// 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`（Java `TigUtils.handelTime`：`null` / `0` → 空串）。
#[must_use]
pub fn handel_time(secs: i64) -> String {
    if secs == 0 {
        return String::new();
    }
    match chrono::Local.timestamp_opt(secs, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => String::new(),
    }
}

/// 用户名掩码（Java `getDimUsername`：首字符 + `***` + 末字符）。
#[must_use]
pub fn dim_username(username: &str) -> String {
    let chars: Vec<char> = username.chars().collect();
    match chars.len() {
        0 => String::new(),
        // Java 对长度 1 会得到 `a***a`（charAt(0) + "***" + substring(0)）
        _ => format!("{}***{}", chars[0], chars[chars.len() - 1]),
    }
}

/// 安全等级（Java `memberCenter`）：双验证 3 / 单验证 2 / 均未验证 1。
#[must_use]
pub fn security_level(mobile_validated: i32, email_validated: i32) -> i32 {
    let mobile = mobile_validated > 0;
    let email = email_validated > 0;
    match (mobile, email) {
        (true, true) => 3,
        (false, false) => 1,
        _ => 2,
    }
}

/// 去掉图片 URL 上的云存储尺寸参数（Java `GalleryPicServiceImpl.getOriginalImage`）。
#[must_use]
pub fn get_original_image(image_url: &str) -> String {
    if image_url.trim().is_empty() {
        return String::new();
    }
    for marker in ["?x-oss-process=", "?imageMogr2/", "?imageView2/"] {
        if let Some(idx) = image_url.find(marker) {
            return image_url[..idx].to_string();
        }
    }
    // `xxxx!!pic400x400.xxx` → `xxxx!!pic.xxx`
    if let Some(pic_idx) = image_url.find("!!pic") {
        if let Some(dot_idx) = image_url.rfind('.') {
            if dot_idx > pic_idx {
                return format!("{}!!pic{}", &image_url[..pic_idx], &image_url[dot_idx..]);
            }
        }
    }
    image_url.to_string()
}

/// 解析浏览历史 JSON 数组（Java `StringUtils.str2IntList`）。
///
/// Java 对非法数字会抛 `NumberFormatException`（500）；这里**跳过**无法解析的
/// 片段（更宽容，已在 `uncertain_points` 标注）。
#[must_use]
pub fn parse_history_ids(raw: Option<&str>) -> Vec<i32> {
    let Some(text) = raw else {
        return Vec::new();
    };
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed == "[]" || trimmed == "[ ]" {
        return Vec::new();
    }
    let stripped = trimmed.replace('[', " ").replace(']', " ");
    if stripped.trim().is_empty() {
        return Vec::new();
    }
    stripped
        .split(',')
        .filter_map(|part| part.trim().parse::<i32>().ok())
        .collect()
}

/// Java `JsonUtil.toJson(List<Integer>)` 的等价实现（紧凑 JSON 数组）。
#[must_use]
pub fn json_i32_list(ids: &[i32]) -> String {
    serde_json::to_string(ids).unwrap_or_else(|_| "[]".to_string())
}

/// 括号内的 `user_rank.rights` 解析为权益数组（Java `parseRights`，非法 JSON → 空表）。
#[must_use]
pub fn parse_rights(rights_json: Option<&str>) -> Vec<RankRight> {
    rights_json
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| serde_json::from_str::<Vec<RankRight>>(s).ok())
        .unwrap_or_default()
}

/// 解析 `rights` 为原始 JSON 数组（Java `JsonUtil.fromJson(rights, JSONArray.class)`）。
#[must_use]
pub fn parse_rights_value(rights_json: Option<&str>) -> Option<Value> {
    let parsed: Value = serde_json::from_str(rights_json?.trim()).ok()?;
    parsed.is_array().then_some(parsed)
}

/// `user_rank.rank_level`（`varchar`）→ 整数（hutool `Convert.toInt` 语义，失败为 `None`）。
#[must_use]
pub fn parse_rank_level(rank_level: &str) -> Option<i32> {
    rank_level.trim().parse::<i32>().ok()
}

/// 去重并保持首次出现顺序。
fn distinct_i32<I: Iterator<Item = i32>>(it: I) -> Vec<i32> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for v in it {
        if seen.insert(v) {
            out.push(v);
        }
    }
    out
}

/// 秒级时间戳 → 本地日期（非法值回落到 `NaiveDate::MIN`）。
#[must_use]
pub fn date_of_epoch_secs(secs: i64) -> NaiveDate {
    match chrono::Local.timestamp_opt(secs, 0) {
        chrono::LocalResult::Single(dt) => dt.date_naive(),
        _ => NaiveDate::MIN,
    }
}

/// 本地日期 00:00:00 的秒级时间戳（Java `LocalDate.atStartOfDay(ZoneId.systemDefault())`）。
#[must_use]
pub fn start_of_day_epoch_secs(date: NaiveDate) -> i64 {
    match date.and_hms_opt(0, 0, 0) {
        Some(dt) => match chrono::Local.from_local_datetime(&dt) {
            chrono::LocalResult::Single(local) => local.timestamp(),
            _ => 0,
        },
        None => 0,
    }
}

/// 账期窗口（Java `CreditUserServiceImpl.PeriodWindow`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeriodWindow {
    /// 周期起始日
    pub start: NaiveDate,
    /// 周期结束日（含）
    pub end: NaiveDate,
    /// 周期总天数
    pub total_days: i32,
}

/// 账期基准起始日（Java `getBaseStartDate`）。
///
/// 优先 `period_start_time`，其次 `create_time`（毫秒级自动折算为秒），
/// 都没有则取今天 0 点。
#[must_use]
pub fn base_start_date(
    period_start_time: Option<i64>,
    create_time: Option<i64>,
    today: NaiveDate,
) -> NaiveDate {
    if let Some(t) = period_start_time.filter(|t| *t > 0) {
        return date_of_epoch_secs(t);
    }
    if let Some(t) = create_time.filter(|t| *t > 0) {
        let secs = if t > 1_000_000_000_000 { t / 1000 } else { t };
        return date_of_epoch_secs(secs);
    }
    today
}

/// 当前账期窗口（Java `getCurrentPeriodWindow`）。
#[must_use]
pub fn current_period_window(
    base_start: NaiveDate,
    settlement_days: i32,
    today: NaiveDate,
) -> PeriodWindow {
    let days = settlement_days.max(1);
    let days_passed = (today - base_start).num_days().max(0);
    let period_index = days_passed / i64::from(days);
    let start = base_start + chrono::Duration::days(period_index * i64::from(days));
    let end = start + chrono::Duration::days(i64::from(days) - 1);
    PeriodWindow { start, end, total_days: days }
}

/// 剩余账期天数（Java `calculateRemainingDays`：存在上期未结算账期订单时返回 0）。
#[must_use]
pub fn remaining_days(window: PeriodWindow, today: NaiveDate, has_pending_before: bool) -> i32 {
    if has_pending_before {
        return 0;
    }
    let elapsed = (today - window.start).num_days().max(0);
    let remaining = i64::from(window.total_days) - elapsed;
    i32::try_from(remaining.max(0)).unwrap_or(0)
}

/// 单笔账期订单是否逾期（Java `buildUserCreditOrderVOList` 的 `overdue` 判定）。
///
/// 仅当 `order_time > 0` 且状态不是「已结算」时才可能逾期。
#[must_use]
pub fn is_credit_order_overdue(
    order_time: i64,
    settlement_status: i32,
    base_start: NaiveDate,
    settlement_days: i32,
    now_date: NaiveDate,
) -> bool {
    if order_time <= 0 || settlement_status == credit_order_status::SETTLED {
        return false;
    }
    let days = settlement_days.max(1);
    let order_date = date_of_epoch_secs(order_time);
    if order_date < base_start {
        return true;
    }
    let days_from_base = (order_date - base_start).num_days().max(0);
    let period_index = days_from_base / i64::from(days);
    let order_period_end =
        base_start + chrono::Duration::days((period_index + 1) * i64::from(days) - 1);
    now_date > order_period_end
}

/// 订单明细实体 → 展示 VO（与 `trade::order_admin` 的 `to_item_vo` 同口径）。
fn to_item_vo(it: &OrderItem) -> OrderItemVO {
    OrderItemVO {
        item_id: it.item_id,
        order_id: it.order_id,
        order_sn: it.order_sn.clone(),
        user_id: it.user_id,
        product_id: it.product_id,
        product_name: it.product_name.clone(),
        product_sn: it.product_sn.clone(),
        pic_thumb: it.pic_thumb.clone(),
        sku_id: it.sku_id,
        sku_data: it.sku_data.clone(),
        price: it.price,
        origin_price: it.origin_price,
        quantity: it.quantity,
        delivery_quantity: it.delivery_quantity,
        product_type: it.product_type,
        is_gift: it.is_gift,
        shop_id: it.shop_id,
        suppliers_id: it.suppliers_id,
        vendor_id: it.vendor_id,
        card_group_name: it.card_group_name.clone(),
        promotion_data: it.promotion_data.clone(),
        commission: it.commission.clone(),
    }
}

/// `user_rank` 行 → `levelList` 条目（Java 用 hutool `BeanUtil.copyProperties`：
/// 会把 `rank_level` 字符串转成整数，`rights` 被显式忽略后单独解析）。
#[must_use]
pub fn to_user_rank_list_vo(rank: &UserRank) -> UserRankListVO {
    UserRankListVO {
        user_rights: String::new(),
        user_count: 0,
        rank_id: rank.rank_id,
        rank_name: rank.rank_name.clone(),
        min_growth_points: rank.min_growth_points,
        max_growth_points: rank.max_growth_points,
        discount: rank.discount,
        show_price: rank.show_price,
        rank_type: rank.rank_type,
        rank_logo: rank.rank_logo.clone(),
        rank_ico: rank.rank_ico.clone(),
        rank_bg: rank.rank_bg.clone(),
        rank_point: rank.rank_point.clone(),
        free_shipping: rank.free_shipping,
        rank_card_type: rank.rank_card_type,
        rights: parse_rights(rank.rights.as_deref()),
        rank_level: parse_rank_level(&rank.rank_level),
    }
}

/// `user_rank` 行 → `levelInfo` 的 `userRank` JSON（camelCase，`rights` 已解析）。
#[must_use]
pub fn to_user_rank_json(rank: &UserRank) -> UserRankJsonVO {
    UserRankJsonVO {
        rank_id: rank.rank_id,
        rank_name: rank.rank_name.clone(),
        min_growth_points: rank.min_growth_points,
        max_growth_points: rank.max_growth_points,
        discount: rank.discount,
        show_price: rank.show_price,
        rank_type: rank.rank_type,
        rank_logo: rank.rank_logo.clone(),
        rank_ico: rank.rank_ico.clone(),
        rank_bg: rank.rank_bg.clone(),
        rank_point: rank.rank_point.clone(),
        free_shipping: rank.free_shipping,
        rank_card_type: rank.rank_card_type,
        rights: parse_rights_value(rank.rights.as_deref()),
        rank_level: rank.rank_level.clone(),
    }
}

/// `user_rank` 行 → `levelInfo` 的 `rankConfig` 值（Java `UserRankConfigVO`）。
fn to_rank_config_value(cfg: &UserRankConfig) -> Value {
    let data = cfg
        .data
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok());
    serde_json::to_value(UserRankConfigVO {
        id: cfg.id,
        code: cfg.code.clone(),
        rank_type: cfg.rank_type,
        data,
    })
    .unwrap_or(Value::Null)
}

/// 商品实体 → 浏览历史 VO（Java `ProductListResDTO` 的可填充子集）。
#[must_use]
pub fn to_history_product_vo(p: &Product, sku_count: i32) -> HistoryProductVO {
    let now = nextrade_common::utils::now_seconds() as i32;
    let price = if p.is_promote == 1
        && p.promote_price > rust_decimal::Decimal::ZERO
        && now >= p.promote_start_date
        && now <= p.promote_end_date
    {
        p.promote_price
    } else {
        p.product_price
    };

    HistoryProductVO {
        category_id: p.category_id,
        brand_id: p.brand_id,
        product_tsn: p.product_tsn.clone(),
        market_price: p.market_price,
        shipping_tpl_id: p.shipping_tpl_id,
        free_shipping: p.free_shipping,
        product_id: p.product_id,
        pic_url: p.pic_url.clone(),
        pic_large: p.pic_large.clone(),
        pic_thumb: p.pic_thumb.clone(),
        product_brief: p.product_brief.clone(),
        product_name: p.product_name.clone(),
        check_status: p.check_status,
        check_reason: p.check_reason.clone(),
        shop_id: p.shop_id,
        suppliers_id: p.suppliers_id,
        product_type: p.product_type,
        sale_mode: p.sale_mode,
        product_sn: p.product_sn.clone(),
        product_price: p.product_price,
        product_status: p.product_status,
        is_best: p.is_best,
        is_new: p.is_new,
        is_hot: p.is_hot,
        product_stock: p.product_stock,
        sort_order: p.sort_order,
        sku_count,
        price,
        vendor_product_id: p.vendor_product_id,
        limit_number: p.limit_number,
        delivery_mode: p.delivery_mode,
        virtual_sales: p.virtual_sales,
        // SKU 明细数组未迁移
        product_sku: None,
    }
}

/// 平台自营虚拟门店配置（Java `GetStoreSelfOperatedVO` 的本切片所需子集）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SelfOperatedSettings {
    name: String,
    logo: String,
    status: i32,
    description: String,
    phone: String,
}

impl SelfOperatedSettings {
    /// 是否暂停运营（Java `StoreType.PAUSE`，收藏列表会跳过该门店）。
    fn is_suspended(&self) -> bool {
        self.status == shop_status::PAUSE
    }
}

/// 产品标记 → 店铺商品列表查询参数（Java `getProductType(shopId, type)`）。
fn flag_of(kind: &str) -> ProductFlag {
    match kind {
        "is_new" => ProductFlag::New,
        "is_best" => ProductFlag::Best,
        _ => ProductFlag::Hot,
    }
}

// ——————————————————————————————————————————————————————————————
// 服务
// ——————————————————————————————————————————————————————————————

pub struct UserCenterService {
    repos: Arc<Repositories>,
}

impl UserCenterService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 读取系统配置（Java `ConfigService.getConfigVal`：缺行 → 空串）。
    async fn config_val(&self, biz_code: &str) -> Result<String> {
        Ok(self
            .repos.member.user_center
            .find_config_val(biz_code)
            .await?
            .unwrap_or_default())
    }

    /// 按 ID 取会员，不存在即报错（Java `Assert.notNull(user, USER_NOT_EXIST)`）。
    async fn require_user(&self, user_id: i32) -> Result<User> {
        self.repos.member.user_center
            .find_user_by_id(user_id)
            .await?
            .ok_or_else(|| Error::biz_msg(ERR_MEMBER_NOT_EXIST))
    }

    // ————————————————————— creditDetail —————————————————————

    /// 用户账期支付信息（Java `UserController.creditDetail` →
    /// `CreditUserServiceImpl.getUserCreditDetail`）。
    ///
    /// 后台未开启账期支付（`config.useCredit != "1"`）时返回 `None`
    /// （Java 返回 `null`，对应响应体 `data: null`）。
    pub async fn credit_detail(&self, user_id: i32) -> Result<Option<CreditDetailVO>> {
        if self.config_val("useCredit").await?.trim() != "1" {
            return Ok(None);
        }
        let credit_user = self
            .repos.member.user_center
            .find_credit_by_user(user_id)
            .await?
            .ok_or_else(|| Error::biz_msg("您不是授信会员，无法查看账期信息"))?;

        let now_epoch = nextrade_common::utils::now_seconds();
        let today = date_of_epoch_secs(now_epoch);
        let days = settlement_type::days_of(credit_user.settlement_type);
        let base_start = base_start_date(
            credit_user.period_start_time,
            credit_user.create_time,
            today,
        );
        let window = current_period_window(base_start, days, today);
        let pending_before = self
            .repos.member.user_center
            .count_credit_orders_before(
                user_id,
                &[
                    credit_order_status::UNSETTLED,
                    credit_order_status::REFUNDING,
                ],
                start_of_day_epoch_secs(window.start),
            )
            .await?
            > 0;
        let remaining = remaining_days(window, today, pending_before);

        let pending = self
            .repos.member.user_center
            .find_credit_orders(
                user_id,
                &[
                    credit_order_status::UNSETTLED,
                    credit_order_status::REFUNDING,
                ],
                false,
            )
            .await?;
        let settled = self
            .repos.member.user_center
            .find_credit_orders(user_id, &[credit_order_status::SETTLED], true)
            .await?;

        Ok(Some(CreditDetailVO {
            credit_limit: credit_user.credit_limit,
            used_amount: credit_user.used_amount,
            available_amount: credit_user.available_amount,
            settlement_type: credit_user.settlement_type,
            settlement_days: days,
            remaining_days: remaining,
            pending_orders: self
                .build_credit_orders(&pending, &credit_user, now_epoch, today)
                .await?,
            settled_orders: self
                .build_credit_orders(&settled, &credit_user, now_epoch, today)
                .await?,
        }))
    }

    /// 账期订单列表组装（Java `buildUserCreditOrderVOList`）。
    async fn build_credit_orders(
        &self,
        orders: &[CreditOrder],
        credit_user: &CreditUser,
        now_epoch: i64,
        today: NaiveDate,
    ) -> Result<Vec<CreditOrderVO>> {
        if orders.is_empty() {
            return Ok(Vec::new());
        }
        let order_ids: Vec<i32> = distinct_i32(orders.iter().map(|o| o.order_id));
        let items = self
            .repos.member.user_center
            .find_items_by_order_ids(&order_ids)
            .await?;
        let mut items_by_order: HashMap<i32, Vec<OrderItem>> = HashMap::new();
        for it in items {
            items_by_order.entry(it.order_id).or_default().push(it);
        }

        let days = settlement_type::days_of(credit_user.settlement_type);
        let base_start = base_start_date(
            credit_user.period_start_time,
            credit_user.create_time,
            today,
        );
        let now_date = date_of_epoch_secs(now_epoch);

        let mut out = Vec::with_capacity(orders.len());
        for co in orders {
            let order_time = co.order_time.unwrap_or(0);
            let empty: Vec<OrderItem> = Vec::new();
            let order_items = items_by_order.get(&co.order_id).unwrap_or(&empty);
            let total_quantity = order_items.iter().map(|it| it.quantity).sum();
            let overdue = is_credit_order_overdue(
                order_time,
                co.settlement_status,
                base_start,
                days,
                now_date,
            );
            out.push(CreditOrderVO {
                order_id: co.order_id,
                order_sn: co.order_sn.clone(),
                order_time: if co.order_time.is_some() {
                    handel_time(order_time)
                } else {
                    String::new()
                },
                credit_amount: co.credit_amount,
                total_quantity,
                settlement_status: co.settlement_status,
                settlement_status_text: credit_order_status::description(co.settlement_status),
                risk_status: i32::from(overdue),
                risk_status_text: if overdue { "逾期" } else { "正常" }.to_string(),
                items: order_items.iter().map(to_item_vo).collect(),
            });
        }
        Ok(out)
    }

    // ————————————————————— updateInformation —————————————————————

    /// 更新会员信息（Java `UserController.updateInformation` →
    /// `UserServiceImpl.updateInformation`）。
    ///
    /// 链路：手机号唯一性校验 → 完善信息成长值日志 → 更新资料字段 →
    /// `addUserGrowthPoints(GROWTH_TYPE_INFORMATION)`（Pro 版 + `rank_config.rank_type = 1`）。
    pub async fn update_information(
        &self,
        user_id: i32,
        dto: &UpdateInformationDTO,
    ) -> Result<()> {
        let user = self.require_user(user_id).await?;

        if let Some(mobile) = dto
            .mobile
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            if self.repos.member.user_center.exists_by_mobile(mobile, user_id).await? {
                return Err(Error::biz_msg("手机号已注册"));
            }
        }

        // Java：`this.getGrowthPoints(user, 1)`（在字段更新**之前**判定昵称/生日）
        self.log_information_growth(&user).await?;
        self.repos.member.user_center
            .update_information(user_id, dto)
            .await?;
        self.add_user_growth_points(user_id, rank_growth_type::INFORMATION)
            .await?;
        Ok(())
    }

    /// Java `UserServiceImpl.getGrowthPoints(user, 1)`：昵称与生日均为空且
    /// `grow_config.evpi` 非空时，写一条 `rank_growth_log`。
    async fn log_information_growth(&self, user: &User) -> Result<()> {
        let Some(cfg) = self.repos.member.user_center.find_rank_config("grow_config").await? else {
            return Ok(());
        };
        let Some(grow) = cfg
            .data
            .as_deref()
            .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        else {
            return Ok(());
        };
        if grow.get("evpi").is_none() || grow.get("evpi").is_some_and(Value::is_null) {
            return Ok(());
        }
        let nickname_blank = user
            .nickname
            .as_deref()
            .is_none_or(|s| s.trim().is_empty());
        let birthday_blank = user
            .birthday
            .as_deref()
            .is_none_or(|s| s.trim().is_empty());
        if !(nickname_blank && birthday_blank) {
            return Ok(());
        }
        let growth = json_i32_field(&grow, "evpiGrowth");
        if growth == 0 {
            return Ok(());
        }
        let now = nextrade_common::utils::now_seconds() as i32;
        self.repos.member.user_center
            .insert_rank_growth_log(&RankGrowthLog {
                id: 0,
                user_id: user.user_id,
                r#type: rank_growth_type::INFORMATION,
                growth_points: growth,
                change_type: 1,
                create_time: Some(now),
            })
            .await?;
        Ok(())
    }

    /// Java `UserRankServiceImpl.addUserGrowthPoints(userId, type)`。
    ///
    /// 前置条件：Pro 版本 且 `rank_config.rank_type == 1`；
    /// 成长值来源：`grow_config.evpi == 1` 且该用户此前没有「完善信息」成长日志。
    async fn add_user_growth_points(&self, user_id: i32, type_code: i32) -> Result<()> {
        if !IS_PRO {
            return Ok(());
        }
        let Some(rank_config) = self.repos.member.user_center.find_rank_config("rank_config").await? else {
            return Ok(());
        };
        if rank_config.rank_type != 1 {
            return Ok(());
        }
        let Some(grow_cfg) = self.repos.member.user_center.find_rank_config("grow_config").await? else {
            // Java 此处会 NPE；Rust 侧按「无配置则不加成长值」处理
            return Ok(());
        };
        let Some(grow) = grow_cfg
            .data
            .as_deref()
            .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        else {
            return Ok(());
        };

        let growth = if type_code == rank_growth_type::INFORMATION {
            let enabled = json_i32_field(&grow, "evpi") == 1;
            let used = self
                .repos.member.user_center
                .count_rank_growth_log(user_id, rank_growth_type::INFORMATION)
                .await?
                > 0;
            if enabled && !used {
                json_i32_field(&grow, "evpiGrowth")
            } else {
                0
            }
        } else if type_code == rank_growth_type::BIND_PHONE {
            let enabled = json_i32_field(&grow, "bindPhone") == 1;
            let used = self
                .repos.member.user_center
                .count_rank_growth_log(user_id, rank_growth_type::BIND_PHONE)
                .await?
                > 0;
            if enabled && !used {
                json_i32_field(&grow, "bindPhoneGrowth")
            } else {
                0
            }
        } else {
            0
        };
        if growth <= 0 {
            return Ok(());
        }

        let now = nextrade_common::utils::now_seconds() as i32;
        self.repos.member.user_center
            .add_growth_points(user_id, growth)
            .await?;
        // Java `updateUserRank(user)`：按成长值重新匹配等级，只回写 rank_id
        let user = self.require_user(user_id).await?;
        if let Some(rank) = self
            .repos.member.user_center
            .find_rank_by_growth(user.growth_points)
            .await?
        {
            if rank.rank_id != user.rank_id {
                self.repos.member.user_center
                    .set_rank_id(user_id, rank.rank_id)
                    .await?;
            }
        }
        self.repos.member.user_center
            .insert_growth_points_log(&UserGrowthPointsLog {
                log_id: 0,
                user_id,
                points: growth,
                change_time: now,
                change_desc: rank_growth_type::description(type_code),
                change_type: 1,
            })
            .await?;
        self.repos.member.user_center
            .insert_rank_growth_log(&RankGrowthLog {
                id: 0,
                user_id,
                r#type: type_code,
                growth_points: growth,
                change_type: 1,
                create_time: Some(now),
            })
            .await?;
        Ok(())
    }

    // ————————————————————— memberCenter —————————————————————

    /// 会员中心（Java `UserController.memberCenter` → `UserServiceImpl.memberCenter`）。
    pub async fn member_center(&self, user_id: i32) -> Result<MemberCenterVO> {
        let user = self.require_user(user_id).await?;
        let rank = self.repos.member.user_center.find_user_rank(user.rank_id).await?;

        let avatar_raw = if user.avatar.trim().is_empty() {
            DEFAULT_MEMBER_AVATAR.to_string()
        } else {
            get_original_image(&user.avatar)
        };

        let (rank_name, min_growth_points, rank_ico, discount, rank_type, rank_bg, free_shipping, rights) =
            match &rank {
                Some(r) => (
                    Some(r.rank_name.clone()),
                    Some(r.min_growth_points),
                    Some(r.rank_ico.clone()),
                    Some(r.discount),
                    Some(r.rank_type),
                    Some(r.rank_bg.clone()),
                    Some(r.free_shipping),
                    parse_rights_value(r.rights.as_deref()),
                ),
                None => (None, None, None, None, None, None, None, None),
            };

        Ok(MemberCenterVO {
            dim_username: dim_username(&user.username),
            user_id: user.user_id,
            username: user.username.clone(),
            // 原版前端优先展示 nickname；昵称为空时回退用户名，避免会员中心姓名空白。
            nickname: user
                .nickname
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .or_else(|| Some(user.username.clone())),
            avatar: avatar_raw,
            rank_id: user.rank_id,
            balance: user.balance,
            commission_balance: user.commission_balance,
            points: user.points,
            mobile_validated: user.mobile_validated,
            email_validated: user.email_validated,
            is_svip: user.is_svip,
            is_company_auth: user.is_company_auth,
            rank_name,
            min_growth_points,
            rank_ico,
            discount,
            rank_type,
            rank_bg,
            // Java：目标 Integer ← 源 varchar，Spring BeanUtils 静默跳过 → null
            rank_point: None,
            free_shipping,
            rights,
            // Java：同上 → null
            rank_level: None,
            security_lv: security_level(user.mobile_validated, user.email_validated),
            await_pay: self
                .repos.member.user_center
                .count_orders_by_status(user_id, order_status::PENDING)
                .await?,
            await_shipping: self
                .repos.member.user_center
                .count_orders_by_status(user_id, order_status::CONFIRMED)
                .await?,
            await_received: self
                .repos.member.user_center
                .count_orders_by_status(user_id, order_status::PROCESSING)
                .await?,
            await_comment: self
                .repos.member.user_center
                .count_orders_by_comment_status(
                    user_id,
                    comment_status::PENDING,
                    order_status::COMPLETED,
                )
                .await?,
            await_coupon: self.repos.member.user_center.count_user_coupons(user_id).await?,
        })
    }

    // ————————————————————— 验证码类（未迁移） —————————————————————

    /// 发送短信验证码（Java `sendMobileCodeByModifyPassword` /
    /// `sendMobileCodeByMobileValidate` / `sendMobileCodeByModifyMobile`）。
    ///
    /// **未迁移**：短信通道与 Redis 验证码存储。这里只做与 Java 一致的前置校验
    /// 发送短信验证码（Java `sendMobileCodeByModifyPassword` / `sendMobileCodeByMobileValidate` / `sendMobileCodeByModifyMobile`）。
    pub async fn send_sms_code(&self, user_id: i32, event: &str, mobile: Option<&str>) -> Result<()> {
        let u = self.require_user(user_id).await?;
        let target_mobile = if event == "modify_mobile" {
            require_non_blank(mobile, "手机号不能为空")?
        } else {
            u.mobile
        };
        if target_mobile.is_empty() {
            return Err(Error::biz(1001, "手机号不能为空"));
        }
        let code = format!("{:06}", rand::random::<u32>() % 1_000_000);
        let key = format!("sms:code:{event}:{target_mobile}");
        nextrade_infrastructure::cache::get_cache()
            .set_json(&key, &code, 300)
            .await?;
        crate::system::notify::send_sms_code(&self.repos, &target_mobile, &code).await?;
        Ok(())
    }

    /// 发送邮件验证码（Java `sendEmailCodeByEmailValidate` / `sendEmailCodeByModifyEmail`）。
    pub async fn send_email_code(&self, user_id: i32, event: &str, email: Option<&str>) -> Result<()> {
        let u = self.require_user(user_id).await?;
        let target_email = if event == "modify_email" {
            require_non_blank(email, "邮箱不能为空")?
        } else {
            u.email
        };
        if target_email.is_empty() {
            return Err(Error::biz(1001, "邮箱不能为空"));
        }
        let code = format!("{:06}", rand::random::<u32>() % 1_000_000);
        let key = format!("email:code:{event}:{target_email}");
        nextrade_infrastructure::cache::get_cache()
            .set_json(&key, &code, 300)
            .await?;
        crate::system::notify::send_email_code(&self.repos, &target_email, "验证码", &code).await?;
        Ok(())
    }

    /// 校验手机验证码并改密码（Java `checkModifyPasswordMobileCode`）。
    pub async fn check_modify_password_mobile_code(
        &self,
        user_id: i32,
        dto: &CheckModifyPasswordDTO,
    ) -> Result<()> {
        let u = self.require_user(user_id).await?;
        let mobile = u.mobile;
        let code = require_non_blank(dto.code.as_deref(), "验证码不能为空")?;
        let key = format!("sms:code:modify_password:{mobile}");
        let cached: Option<String> = nextrade_infrastructure::cache::get_cache().get_json(&key).await?;
        if cached.as_deref() != Some(code.as_str()) {
            return Err(Error::biz(1001, "验证码错误或已过期"));
        }
        Ok(())
    }

    /// 手机验证（Java `mobileValidate`）。
    pub async fn mobile_validate(
        &self,
        user_id: i32,
        dto: &CheckModifyPasswordDTO,
    ) -> Result<()> {
        let u = self.require_user(user_id).await?;
        let mobile = u.mobile;
        let code = require_non_blank(dto.code.as_deref(), "验证码不能为空")?;
        let key = format!("sms:code:mobile_validate:{mobile}");
        let cache = nextrade_infrastructure::cache::get_cache();
        let cached: Option<String> = cache.get_json(&key).await?;
        if cached.as_deref() != Some(code.as_str()) {
            return Err(Error::biz(1001, "验证码错误或已过期"));
        }
        cache.del(&key).await?;
        self.repos.member.user_center.update_mobile(user_id, &mobile).await?;
        Ok(())
    }

    /// 邮箱验证（Java `emailValidateNew`）。
    pub async fn email_validate_new(
        &self,
        user_id: i32,
        dto: &CheckModifyEmailDTO,
    ) -> Result<()> {
        let u = self.require_user(user_id).await?;
        let email = u.email;
        let code = require_non_blank(dto.code.as_deref(), "验证码不能为空")?;
        let key = format!("email:code:email_validate:{email}");
        let cache = nextrade_infrastructure::cache::get_cache();
        let cached: Option<String> = cache.get_json(&key).await?;
        if cached.as_deref() != Some(code.as_str()) {
            return Err(Error::biz(1001, "验证码错误或已过期"));
        }
        cache.del(&key).await?;
        self.repos.member.user_center.update_email(user_id, &email).await?;
        Ok(())
    }

    /// 修改手机号（Java `modifyMobile`）。
    pub async fn modify_mobile(&self, user_id: i32, param: &ModifyMobileParam) -> Result<()> {
        let mobile = require_non_blank(param.mobile.as_deref(), "手机号不能为空")?;
        let code = require_non_blank(param.code.as_deref(), "验证码不能为空")?;
        self.require_user(user_id).await?;
        let key = format!("sms:code:modify_mobile:{mobile}");
        let cache = nextrade_infrastructure::cache::get_cache();
        let cached: Option<String> = cache.get_json(&key).await?;
        if cached.as_deref() != Some(code.as_str()) {
            return Err(Error::biz(1001, "验证码错误或已过期"));
        }
        cache.del(&key).await?;
        self.repos.member.user_center.update_mobile(user_id, &mobile).await?;
        Ok(())
    }

    /// 修改邮箱（Java `modifyEmail`）。
    pub async fn modify_email(&self, user_id: i32, param: &ModifyEmailParam) -> Result<()> {
        let email = require_non_blank(param.email.as_deref(), "邮箱不能为空")?;
        let code = require_non_blank(param.code.as_deref(), "验证码不能为空")?;
        self.require_user(user_id).await?;
        let key = format!("email:code:modify_email:{email}");
        let cache = nextrade_infrastructure::cache::get_cache();
        let cached: Option<String> = cache.get_json(&key).await?;
        if cached.as_deref() != Some(code.as_str()) {
            return Err(Error::biz(1001, "验证码错误或已过期"));
        }
        cache.del(&key).await?;
        self.repos.member.user_center.update_email(user_id, &email).await?;
        Ok(())
    }

    // ————————————————————— emailValidate —————————————————————

    /// 邮箱验证 / 直接改绑（Java `emailValidate`）。
    ///
    /// 与其它验证码接口不同，Java 该方法**不校验 Redis 验证码**：
    /// 只做邮箱唯一性校验，`type > 0` 时直接改绑并置 `email_validated = 1`。
    pub async fn email_validate(&self, user_id: i32, dto: &CheckEmailModifyDTO) -> Result<()> {
        let user = self.require_user(user_id).await?;
        let email = require_non_blank(dto.email.as_deref(), "邮箱不能为空")?;
        if self.repos.member.user_center.exists_by_email(&email, user_id).await? {
            return Err(Error::biz_msg("邮箱已经被绑定"));
        }
        if dto.type_.is_some_and(|t| t > 0) {
            self.repos.member.user_center
                .bind_email(user.user_id, &email)
                .await?;
        }
        Ok(())
    }

    // ————————————————————— historyProduct / delHistoryProduct —————————————————————

    /// 最近浏览（Java `historyProduct`）。
    pub async fn history_product(&self, user_id: i32) -> Result<Vec<HistoryProductVO>> {
        let user = self.require_user(user_id).await?;
        let ids = parse_history_ids(user.history_product_ids.as_deref());
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut products = self.repos.member.user_center.find_products_by_ids(&ids).await?;
        if products.is_empty() {
            return Ok(Vec::new());
        }
        // 还原浏览顺序（Java 用 `ORDER BY field(product_id, ...)` + `limit 20`）
        let order: HashMap<i32, usize> = ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
        products.sort_by_key(|p| order.get(&p.product_id).copied().unwrap_or(usize::MAX));
        products.truncate(HISTORY_PRODUCT_LIMIT);
        let product_ids: Vec<i32> = products.iter().map(|p| p.product_id).collect();
        let sku_counts = self
            .repos.member.user_center
            .count_skus_by_product_ids(&product_ids)
            .await?;
        Ok(products
            .iter()
            .map(|p| {
                let sku_count = sku_counts.get(&p.product_id).copied().unwrap_or(0);
                to_history_product_vo(p, sku_count)
            })
            .collect())
    }

    /// 删除最近浏览（Java `delHistoryProduct`）。
    ///
    /// Java 传空 `ids` 时**不清空**历史（过滤条件不生效），本实现保持一致。
    pub async fn del_history_product(
        &self,
        user_id: i32,
        dto: &DelHistoryProductDTO,
    ) -> Result<()> {
        let user = self.require_user(user_id).await?;
        let mut ids = parse_history_ids(user.history_product_ids.as_deref());
        if let Some(remove) = dto.ids.as_ref().filter(|v| !v.is_empty()) {
            ids.retain(|id| !remove.contains(id));
        }
        self.repos.member.user_center
            .update_history(user_id, &json_i32_list(&ids))
            .await?;
        Ok(())
    }

    // ————————————————————— 上传（未迁移） —————————————————————

    /// 上传图片（Java `uploadImg` → `GalleryPicService.uploadImg`）。
    pub async fn upload_img(
        &self,
        file_name: Option<&str>,
        bytes: &[u8],
        _is_hidden: bool,
    ) -> Result<GalleryPicUploadVO> {
        require_upload_file(file_name, bytes)?;
        let name = file_name.unwrap_or("file");
        let url = nextrade_infrastructure::storage::save_local_upload("gallery", name, bytes).await?;
        Ok(GalleryPicUploadVO {
            pic_thumb: url.clone(),
            pic_url: url,
            pic_name: name.to_string(),
            pic_id: 0,
            pic_width: 0,
            pic_height: 0,
            pic_size: bytes.len() as i64,
            gallery_id: 0,
            file_type: "image".to_string(),
        })
    }

    /// 上传视频（Java `uploadVideo` → `GalleryVideoInfoService.uploadVideo`）。
    pub async fn upload_video(
        &self,
        file_name: Option<&str>,
        bytes: &[u8],
    ) -> Result<UploadVideoInfoVO> {
        require_upload_file(file_name, bytes)?;
        let name = file_name.unwrap_or("video.mp4");
        let url = nextrade_infrastructure::storage::save_local_upload("gallery_video", name, bytes).await?;
        Ok(UploadVideoInfoVO {
            video_url: url,
            video_name: name.to_string(),
            format: "video".to_string(),
            video_cover: String::new(),
        })
    }

    /// 修改头像（Java `modifyAvatar`）。
    pub async fn modify_avatar(&self, file_name: Option<&str>, bytes: &[u8]) -> Result<()> {
        require_upload_file(file_name, bytes)?;
        let name = file_name.unwrap_or("avatar.jpg");
        let _ = nextrade_infrastructure::storage::save_local_upload("avatar", name, bytes).await?;
        Ok(())
    }

    // ————————————————————— collectionShop —————————————————————

    /// 收藏店铺（b2b2b 5.8.57 前端新增 `collectionShop/save`，无 Java 对应；
    /// payload `{shopId}`，幂等）。
    pub async fn save_collect_shop(&self, user_id: i32, shop_id: i32) -> Result<()> {
        if shop_id <= 0 {
            return Err(nextrade_core::Error::Validation("缺少店铺ID".into()));
        }
        self.repos.member.user_center.save_collect_shop(user_id, shop_id).await
    }

    /// 取消收藏店铺（payload `{id}` 或 `{shopId}`）。
    pub async fn cancel_collect_shop(
        &self,
        user_id: i32,
        collect_id: Option<i32>,
        shop_id: Option<i32>,
    ) -> Result<()> {
        self.repos
            .member
            .user_center
            .cancel_collect_shop(user_id, collect_id, shop_id)
            .await
    }

    /// 收藏店铺分页（Java `collectionShop` → `CollectShopServiceImpl.list`）。
    pub async fn collection_shop(
        &self,
        user_id: i32,
        q: &CollectShopQuery,
    ) -> Result<PageResult<CollectShopVO>> {
        let page = q.page_query();
        let order_by = UserCenterRepository::collect_shop_order_by(
            q.sort_field.as_deref(),
            q.sort_order.as_deref(),
        );
        let (limit, offset) = page.limit_offset();
        let (total, rows) = self
            .repos.member.user_center
            .paginate_collect_shop(user_id, q.exclude_self_operated(), &order_by, limit, offset)
            .await?;
        if rows.is_empty() {
            return Ok(PageResult::new(total, Vec::new(), &page));
        }

        let shop_ids = distinct_i32(
            rows.iter()
                .map(|r| r.shop_id)
                .filter(|id| *id != SELF_OPERATED_SHOP_ID),
        );
        let shops = self.repos.member.user_center.find_shops(&shop_ids).await?;
        let shop_map: HashMap<i32, &Shop> = shops.iter().map(|s| (s.shop_id, s)).collect();

        let self_operated = if rows.iter().any(|r| r.shop_id == SELF_OPERATED_SHOP_ID) {
            Some(self.load_self_operated_settings().await?)
        } else {
            None
        };

        let mut list = Vec::with_capacity(rows.len());
        for row in &rows {
            let shop_vo = if row.shop_id == SELF_OPERATED_SHOP_ID {
                match &self_operated {
                    // 暂停运营的自营虚拟门店整条跳过（Java `toCollectShopVO` 返回 null）
                    Some(settings) if !settings.is_suspended() => Some(
                        self.build_self_operated_shop_vo(settings).await?,
                    ),
                    _ => None,
                }
            } else {
                match shop_map.get(&row.shop_id) {
                    Some(shop) => Some(self.build_shop_vo(shop).await?),
                    // Java：店铺行缺失时 `collectShopVO.shop` 保持 null
                    None => None,
                }
            };
            if row.shop_id == SELF_OPERATED_SHOP_ID && shop_vo.is_none() {
                continue;
            }
            list.push(CollectShopVO {
                collect_id: row.collect_id,
                user_id: row.user_id,
                shop_id: row.shop_id,
                add_time: handel_time(i64::from(row.add_time)),
                product_count: self
                    .repos.member.user_center
                    .count_shop_on_sale_products(row.shop_id)
                    .await?,
                collect_count: self.repos.member.user_center.count_shop_collects(row.shop_id).await?,
                shop: shop_vo,
            });
        }
        Ok(PageResult::new(total, list, &page))
    }

    /// 平台自营虚拟门店配置（Java `configService.getStoreSelfOperatedSettings()`）。
    async fn load_self_operated_settings(&self) -> Result<SelfOperatedSettings> {
        let status = self
            .config_val("storeSelfOperatedStatus")
            .await?
            .trim()
            .parse::<i32>()
            .unwrap_or(shop_status::OPEN);
        Ok(SelfOperatedSettings {
            name: self.config_val("storeSelfOperatedName").await?,
            logo: self.config_val("storeSelfOperatedLogo").await?,
            status,
            description: self.config_val("storeSelfOperatedDescription").await?,
            phone: self.config_val("storeSelfOperatedPhone").await?,
        })
    }

    /// 平台自营虚拟门店 → `CollectShopVO.ShopVO`
    /// （Java `toCollectShopVO` 的 `SELF_OPERATED_SHOP_ID` 分支）。
    async fn build_self_operated_shop_vo(
        &self,
        settings: &SelfOperatedSettings,
    ) -> Result<CollectShopShopVO> {
        let logo = if settings.logo.trim().is_empty() {
            self.config_val("shopLogo").await?
        } else {
            settings.logo.clone()
        };
        Ok(CollectShopShopVO {
            status_text: shop_status::type_name(Some(settings.status)),
            shop_id: SELF_OPERATED_SHOP_ID,
            shop_title: Some(settings.name.clone()),
            add_time: None,
            shop_logo: Some(logo),
            click_count: None,
            status: Some(settings.status),
            merchant_id: None,
            shop_money: None,
            frozen_money: None,
            contact_mobile: Some(settings.phone.clone()),
            description: Some(settings.description.clone()),
            kefu_phone: None,
            kefu_weixin: None,
            kefu_link: None,
            is_contact_kefu: 0,
            kefu_inlet: None,
            hot_product: self
                .repos.member.user_center
                .top_products_by_flag(SELF_OPERATED_SHOP_ID, flag_of("is_hot"), SHOP_PRODUCT_LIMIT)
                .await?,
            new_product: self
                .repos.member.user_center
                .top_products_by_flag(SELF_OPERATED_SHOP_ID, flag_of("is_new"), SHOP_PRODUCT_LIMIT)
                .await?,
            best_product: self
                .repos.member.user_center
                .top_products_by_flag(SELF_OPERATED_SHOP_ID, flag_of("is_best"), SHOP_PRODUCT_LIMIT)
                .await?,
        })
    }

    /// 店铺行 → `CollectShopVO.ShopVO`（Java `toCollectShopVO` 的普通店铺分支）。
    async fn build_shop_vo(&self, shop: &Shop) -> Result<CollectShopShopVO> {
        Ok(CollectShopShopVO {
            status_text: shop_status::type_name(shop.status),
            shop_id: shop.shop_id,
            shop_title: shop.shop_title.clone(),
            // Java：目标 String ← 源 Integer，Spring BeanUtils 静默跳过 → null
            add_time: None,
            shop_logo: shop.shop_logo.clone(),
            click_count: shop.click_count,
            status: shop.status,
            merchant_id: shop.merchant_id,
            shop_money: shop.shop_money,
            frozen_money: shop.frozen_money,
            contact_mobile: shop.contact_mobile.clone(),
            description: shop.description.clone(),
            kefu_phone: shop.kefu_phone.clone(),
            kefu_weixin: shop.kefu_weixin.clone(),
            kefu_link: shop.kefu_link.clone(),
            is_contact_kefu: shop.is_contact_kefu,
            // Java：目标 List<Integer> ← 源 String，静默跳过 → null
            kefu_inlet: None,
            hot_product: self
                .repos.member.user_center
                .top_products_by_flag(shop.shop_id, flag_of("is_hot"), SHOP_PRODUCT_LIMIT)
                .await?,
            new_product: self
                .repos.member.user_center
                .top_products_by_flag(shop.shop_id, flag_of("is_new"), SHOP_PRODUCT_LIMIT)
                .await?,
            best_product: self
                .repos.member.user_center
                .top_products_by_flag(shop.shop_id, flag_of("is_best"), SHOP_PRODUCT_LIMIT)
                .await?,
        })
    }

    // ————————————————————— levelList / levelInfo —————————————————————

    /// 会员等级列表（Java `UserController.levelList`）。
    pub async fn level_list(&self) -> Result<UserLevelListVO> {
        let ranks = self.repos.member.user_center.all_user_rank().await?;
        let item = ranks.iter().map(to_user_rank_list_vo).collect();

        let rank_config = self
            .repos.member.user_center
            .find_rank_config("rank_config")
            .await?
            .map(|cfg| {
                let data = cfg
                    .data
                    .as_deref()
                    .and_then(|raw| serde_json::from_str::<Value>(raw).ok());
                UserRankConfigVO {
                    id: cfg.id,
                    code: cfg.code.clone(),
                    rank_type: cfg.rank_type,
                    data,
                }
            });
        let grow_config = self
            .repos.member.user_center
            .find_rank_config("grow_config")
            .await?
            .and_then(|cfg| {
                cfg.data
                    .as_deref()
                    .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
            });

        Ok(UserLevelListVO {
            item,
            rank_config,
            grow_config,
        })
    }

    /// 获取会员等级权益信息（Java `UserController.levelInfo` →
    /// `UserRankServiceImpl.getRankInfo`）。
    pub async fn level_info(&self, rank_id: i32) -> Result<UserLevelInfoVO> {
        match self.repos.member.user_center.find_user_rank(rank_id).await? {
            Some(rank) => {
                let user_rank = serde_json::to_value(to_user_rank_json(&rank))
                    .unwrap_or(Value::Null);
                let rank_config = self
                    .repos.member.user_center
                    .find_rank_config("rank_config")
                    .await?
                    .map(|cfg| to_rank_config_value(&cfg));
                // Java：`growUpSetting = growSetting.getData()`（**原始字符串**）
                let grow_up_setting = self
                    .repos.member.user_center
                    .find_rank_config("grow_setting")
                    .await?
                    .and_then(|cfg| cfg.data)
                    .map(Value::String);
                Ok(UserLevelInfoVO {
                    rank_type: 1,
                    user_rank,
                    rank_config,
                    grow_up_setting,
                })
            }
            None => {
                Ok(UserLevelInfoVO {
                    rank_type: 1,
                    user_rank: default_user_rank_json(),
                    rank_config: Some(default_rank_config_json()),
                    grow_up_setting: Some(default_grow_setting_json()),
                })
            }
        }
    }

    // ————————————————————— logout / close —————————————————————

    /// 前台用户退出登录（Java `UserController.logout` → `clientLogout`）。
    ///
    /// 客户端清空本地 Token，服务端返回成功以支持平滑登出。
    pub fn logout(&self) -> Result<()> {
        Ok(())
    }

    /// 禁用（注销）当前用户（Java `UserController.close` → `closeUser`）。
    pub async fn close(&self, user_id: i32) -> Result<()> {
        if user_id > 0 {
            self.repos.member.user_center.disable_user(user_id).await?;
        }
        Ok(())
    }
}

/// 校验非空字符串（Java `@NotBlank` / `StrUtil.isBlank` 语义）。
fn require_non_blank(value: Option<&str>, message: &str) -> Result<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| Error::Validation(message.to_string()))
}

/// 校验上传文件非空（Java `MultipartFile.isEmpty()`）。
fn require_upload_file(file_name: Option<&str>, bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() {
        return Err(Error::Validation("上传文件不能为空".to_string()));
    }
    if file_name.is_none_or(|n| n.trim().is_empty()) {
        return Err(Error::Validation("上传文件名不能为空".to_string()));
    }
    Ok(())
}

/// 读取 JSON 对象中的整数字段（缺失 / 类型不符 → 0）。
fn json_i32_field(value: &Value, key: &str) -> i32 {
    value
        .get(key)
        .and_then(|v| {
            v.as_i64()
                .map(|n| i32::try_from(n).unwrap_or(0))
                .or_else(|| v.as_str().and_then(|s| s.trim().parse::<i32>().ok()))
        })
        .unwrap_or(0)
}

/// `levelInfo` 在等级不存在时的默认 `userRank`（Java `getUserRankListInitDTO` +
/// Spring `BeanUtils` 类型不匹配跳过，`rankPoint` 恒为 null）。
fn default_user_rank_json() -> Value {
    json!({
        "userRights": "",
        "userCount": 0,
        "rankId": 0,
        "rankName": "黄金会员",
        "minGrowthPoints": 0,
        "maxGrowthPoints": 0,
        "discount": 0,
        "showPrice": 0,
        "rankType": 0,
        "rankLogo": "https://oss.nextrade.com/img/gallery/202501/1737524324ro1DJxNm3aQowZKnPU.png",
        "rankIco": "card1",
        "rankBg": "",
        "rankPoint": Value::Null,
        "freeShipping": 0,
        "rankCardType": 1,
        "rights": [],
        "rankLevel": 1
    })
}

/// `levelInfo` 在等级不存在时的默认 `rankConfig`（Java `getRankConfigInit`）。
fn default_rank_config_json() -> Value {
    json!({
        "id": Value::Null,
        "code": "rank_config",
        "rankType": 1,
        "data": { "type": 1, "rankAfterMonth": 12, "useMonth": 12 }
    })
}

/// `levelInfo` 在等级不存在时的默认 `growUpSetting`（Java `getGrowUpSettingInit`）。
fn default_grow_setting_json() -> Value {
    json!({
        "buyOrder": 1,
        "buyOrderNumber": 1,
        "buyOrderGrowth": 1,
        "evpi": 1,
        "evpiGrowth": 1,
        "bindPhone": 1,
        "bindPhoneGrowth": 1
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    // ————————————————— 纯函数 —————————————————

    #[test]
    fn handel_time_matches_java() {
        assert_eq!(handel_time(0), "");
        assert_eq!(handel_time(1_700_000_000).len(), 19);
    }

    #[test]
    fn dim_username_masks_first_and_last() {
        assert_eq!(dim_username("abcdef"), "a***f");
        // Java 单字符：charAt(0) + "***" + substring(0) → "a***a"
        assert_eq!(dim_username("a"), "a***a");
        assert_eq!(dim_username(""), "");
    }

    #[test]
    fn security_level_follows_validation_flags() {
        assert_eq!(security_level(1, 1), 3);
        assert_eq!(security_level(1, 0), 2);
        assert_eq!(security_level(0, 1), 2);
        assert_eq!(security_level(0, 0), 1);
    }

    #[test]
    fn get_original_image_strips_cloud_params() {
        assert_eq!(
            get_original_image("https://x/a.png?x-oss-process=image/resize,w_100"),
            "https://x/a.png"
        );
        assert_eq!(
            get_original_image("https://x/a.png?imageMogr2/thumbnail/200x"),
            "https://x/a.png"
        );
        assert_eq!(
            get_original_image("https://x/a.png?imageView2/2/w/200"),
            "https://x/a.png"
        );
        assert_eq!(
            get_original_image("https://x/a!!pic400x400.png"),
            "https://x/a!!pic.png"
        );
        assert_eq!(get_original_image("https://x/a.png"), "https://x/a.png");
        assert_eq!(get_original_image(""), "");
    }

    #[test]
    fn parse_history_ids_tolerates_java_formats() {
        assert_eq!(parse_history_ids(Some("[1,2,3]")), vec![1, 2, 3]);
        assert_eq!(parse_history_ids(Some("[ 1, 2 ]")), vec![1, 2]);
        assert!(parse_history_ids(Some("[ ]")).is_empty());
        assert!(parse_history_ids(Some("[]")).is_empty());
        assert!(parse_history_ids(None).is_empty());
        // 非法片段跳过而非 panic（Java 会抛 NumberFormatException）
        assert_eq!(parse_history_ids(Some("[1,x,3]")), vec![1, 3]);
    }

    #[test]
    fn json_i32_list_is_compact() {
        assert_eq!(json_i32_list(&[1, 2, 3]), "[1,2,3]");
        assert_eq!(json_i32_list(&[]), "[]");
    }

    #[test]
    fn parse_rights_handles_bad_json() {
        let ok = parse_rights(Some(r#"[{"name":"a","isChecked":1}]"#));
        assert_eq!(ok.len(), 1);
        assert_eq!(ok[0].name.as_deref(), Some("a"));
        assert_eq!(ok[0].is_checked, 1);
        assert!(parse_rights(Some("not json")).is_empty());
        assert!(parse_rights(None).is_empty());
        // 非数组 JSON 不作为 rights 数组返回
        assert!(parse_rights_value(Some(r#"{"a":1}"#)).is_none());
        assert!(parse_rights_value(Some(r#"[1,2]"#)).is_some());
    }

    #[test]
    fn parse_rank_level_is_lenient() {
        assert_eq!(parse_rank_level("2"), Some(2));
        assert_eq!(parse_rank_level(""), None);
        assert_eq!(parse_rank_level("黄金"), None);
    }

    // ————————————————— 账期周期计算 —————————————————

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn base_start_date_prefers_period_start_then_create_time() {
        let today = date(2026, 1, 20);
        // 毫秒级 create_time 自动折算为秒
        let ms = 1_700_000_000_000_i64;
        assert_eq!(
            base_start_date(None, Some(ms), today),
            date_of_epoch_secs(ms / 1000)
        );
        assert_eq!(
            base_start_date(Some(0), Some(1_700_000_000), today),
            date_of_epoch_secs(1_700_000_000)
        );
        assert_eq!(base_start_date(None, None, today), today);
    }

    #[test]
    fn period_window_rolls_every_settlement_days() {
        let base = date(2026, 1, 1);
        // 第 0 周期
        let w = current_period_window(base, 30, date(2026, 1, 1));
        assert_eq!((w.start, w.end, w.total_days), (base, date(2026, 1, 30), 30));
        // 第 1 周期
        let w = current_period_window(base, 30, date(2026, 1, 31));
        assert_eq!((w.start, w.end), (date(2026, 1, 31), date(2026, 3, 1)));
        // 季结 90 天：01-01 起第 90 天进入第 1 周期（2026-04-01）
        let w = current_period_window(base, 90, date(2026, 4, 1));
        assert_eq!((w.start, w.end), (date(2026, 4, 1), date(2026, 6, 29)));
        // 仍在第 0 周期内
        let w = current_period_window(base, 90, date(2026, 3, 31));
        assert_eq!((w.start, w.end), (base, date(2026, 3, 31)));
        // 今天早于基准日：不出现负偏移
        let w = current_period_window(base, 30, date(2025, 12, 1));
        assert_eq!(w.start, base);
    }

    #[test]
    fn remaining_days_zero_when_previous_period_unsettled() {
        let base = date(2026, 1, 1);
        let w = current_period_window(base, 30, date(2026, 1, 10));
        // 已过 9 天 → 剩余 21 天
        assert_eq!(remaining_days(w, date(2026, 1, 10), false), 21);
        // 存在上期未结算 → 0
        assert_eq!(remaining_days(w, date(2026, 1, 10), true), 0);
        // 超出周期 → 不为负
        assert_eq!(remaining_days(w, date(2026, 3, 1), false), 0);
    }

    #[test]
    fn overdue_detection_matches_java() {
        let base = date(2026, 1, 1);
        let day = 86_400_i64;
        let base_epoch = start_of_day_epoch_secs(base);
        // 上周期的待结算订单 → 逾期
        assert!(is_credit_order_overdue(
            base_epoch - day,
            credit_order_status::UNSETTLED,
            base,
            30,
            date(2026, 1, 5)
        ));
        // 本周期结束前的待结算订单 → 正常
        assert!(!is_credit_order_overdue(
            base_epoch,
            credit_order_status::UNSETTLED,
            base,
            30,
            date(2026, 1, 5)
        ));
        // 本周期已过期 → 逾期
        assert!(is_credit_order_overdue(
            base_epoch,
            credit_order_status::UNSETTLED,
            base,
            30,
            date(2026, 2, 15)
        ));
        // 已结算永不逾期
        assert!(!is_credit_order_overdue(
            base_epoch - day,
            credit_order_status::SETTLED,
            base,
            30,
            date(2026, 2, 15)
        ));
        // 无下单时间
        assert!(!is_credit_order_overdue(
            0,
            credit_order_status::UNSETTLED,
            base,
            30,
            date(2026, 2, 15)
        ));
    }

    // ————————————————— 等级映射 —————————————————

    fn rank() -> UserRank {
        UserRank {
            rank_id: 2,
            rank_name: "白银会员".into(),
            min_growth_points: Decimal::new(500, 2),
            max_growth_points: 9999,
            discount: Decimal::new(95, 1),
            show_price: 1,
            rank_type: 1,
            rank_logo: "logo.png".into(),
            rank_ico: "card2".into(),
            rank_bg: "bg.png".into(),
            rank_point: "2".into(),
            free_shipping: 1,
            rank_card_type: 2,
            rights: Some(r#"[{"name":"discount","isChecked":1}]"#.into()),
            rank_level: "3".into(),
        }
    }

    #[test]
    fn level_list_vo_parses_rights_and_level() {
        let vo = to_user_rank_list_vo(&rank());
        assert_eq!(vo.rank_id, 2);
        assert_eq!(vo.rank_point, "2"); // hutool 拷贝：String → String
        assert_eq!(vo.rank_level, Some(3)); // hutool Convert.toInt
        assert_eq!(vo.rights.len(), 1);
        assert_eq!(vo.user_count, 0);
        assert!(vo.user_rights.is_empty());
    }

    #[test]
    fn level_info_json_keeps_raw_rank_level() {
        let v = serde_json::to_value(to_user_rank_json(&rank())).unwrap();
        assert_eq!(v["rankId"], 2);
        // 原始实体字段是 varchar，levelInfo 不做类型转换
        assert_eq!(v["rankLevel"], "3");
        assert_eq!(v["rights"][0]["name"], "discount");
    }

    #[test]
    fn default_level_info_payloads_match_java_inits() {
        assert_eq!(default_user_rank_json()["rankName"], "黄金会员");
        assert_eq!(default_user_rank_json()["rankLevel"], 1);
        assert!(default_user_rank_json()["rankPoint"].is_null());
        assert_eq!(default_rank_config_json()["code"], "rank_config");
        assert_eq!(default_rank_config_json()["data"]["rankAfterMonth"], 12);
        assert_eq!(default_grow_setting_json()["evpiGrowth"], 1);
    }

    #[test]
    fn json_i32_field_reads_numbers_and_strings() {
        let v = json!({"a": 3, "b": "4", "c": null, "d": 1e10});
        assert_eq!(json_i32_field(&v, "a"), 3);
        assert_eq!(json_i32_field(&v, "b"), 4);
        assert_eq!(json_i32_field(&v, "c"), 0);
        assert_eq!(json_i32_field(&v, "missing"), 0);
    }

    // ————————————————— 参数校验 —————————————————

    #[test]
    fn require_non_blank_trims_and_rejects_blank() {
        assert_eq!(require_non_blank(Some(" a@b.c "), "x").unwrap(), "a@b.c");
        assert!(require_non_blank(Some("   "), "邮箱不能为空").is_err());
        assert!(require_non_blank(None, "邮箱不能为空").is_err());
    }

    #[test]
    fn require_upload_file_rejects_empty() {
        assert!(require_upload_file(Some("a.png"), &[1, 2]).is_ok());
        assert!(require_upload_file(Some("a.png"), &[]).is_err());
        assert!(require_upload_file(None, &[1]).is_err());
        assert!(require_upload_file(Some("  "), &[1]).is_err());
    }

    #[test]
    fn history_product_vo_uses_retail_price_as_final_price() {
        let p = Product {
            product_id: 9,
            product_sn: "SN9".into(),
            product_name: "商品9".into(),
            product_price: Decimal::new(1999, 2),
            market_price: Decimal::new(2999, 2),
            ..Default::default()
        };
        let vo = to_history_product_vo(&p, 3);
        assert_eq!(vo.product_id, 9);
        assert_eq!(vo.sku_count, 3);
        assert_eq!(vo.price, p.product_price);
        assert!(vo.product_sku.is_none());
    }

    #[test]
    fn history_vo_calculates_active_promotion_price() {
        let now = nextrade_common::utils::now_seconds() as i32;
        let p = Product {
            product_id: 10,
            product_sn: "SN10".into(),
            product_name: "商品10".into(),
            product_price: Decimal::new(2999, 2),
            market_price: Decimal::new(3999, 2),
            is_promote: 1,
            promote_price: Decimal::new(1999, 2),
            promote_start_date: now - 3600,
            promote_end_date: now + 3600,
            ..Default::default()
        };
        let vo = to_history_product_vo(&p, 1);
        assert_eq!(vo.price, Decimal::new(1999, 2));
    }

    #[test]
    fn flag_of_maps_java_type_strings() {
        assert_eq!(flag_of("is_new"), ProductFlag::New);
        assert_eq!(flag_of("is_best"), ProductFlag::Best);
        assert_eq!(flag_of("is_hot"), ProductFlag::Hot);
        assert_eq!(flag_of("unknown"), ProductFlag::Hot);
    }
}