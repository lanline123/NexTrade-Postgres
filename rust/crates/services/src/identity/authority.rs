//! 权限管理服务（identity::authority）。
//!
//! 对应原 Java `com.nextrade.adminapi.controller.authority.AuthorityController`
//! + `com.nextrade.service.authority.impl.AuthorityServiceImpl`
//! （1029 行）+ `BaseServiceImpl#updateField / batch / del`。
//!
//! | 本域端点 | 原 Java | 落点 |
//! |----------|---------|------|
//! | `GET  /adminapi/authority/authority/getAllAuthority` | `getAllAuthority` | [`AuthorityService::get_all_authority`] |
//! | `GET  /adminapi/authority/authority/getAuthority` | `getAuthority` | [`AuthorityService::get_authority`] |
//! | `GET  /adminapi/authority/authority/list` | `list` | [`AuthorityService::list`] |
//! | `GET  /adminapi/authority/authority/getAuthorityParentName` | `getAuthorityParentName` | [`AuthorityService::get_authority_parent_name`] |
//! | `POST /adminapi/authority/authority/batch` | `batch`（`del` → `batchDel`） | [`AuthorityService::batch`] |
//! | `POST /adminapi/authority/authority/create` | `create` | [`AuthorityService::create`] |
//! | `POST /adminapi/authority/authority/del` | `del` | [`AuthorityService::del`] |
//! | `GET  /adminapi/authority/authority/detail` | `detail` | [`AuthorityService::detail`] |
//! | `POST /adminapi/authority/authority/update` | `update` | [`AuthorityService::update`] |
//! | `POST /adminapi/authority/authority/updateField` | `updateField` | [`AuthorityService::update_field`] |
//!
//! ## 迁移要点
//!
//! - `list`：先分页（`admin_type` / `parent_id` 条件）再在内存里按版本下架规则
//!   `removeIf`，**与原 Java 顺序完全一致**（因此当前页条数可能少于 `size`）；
//!   `hasChildren` 是子级**数量**而非布尔（`childrenAuthorities` 一次取回，消除 N+1）。
//! - `create` / `update`：先做「`authority_sn` + `admin_type` 唯一」校验，
//!   `child_auth` 空列表序列化为 `"[]"`（Java `Optional.filter(!isEmpty).orElse("[]")`）。
//! - `del`：存在任意子级即报 `error.authority.has.children`。
//! - `batchDel`：按「祖先中也在删除集内的层数」倒序（深度大的先删），
//!   并逐条校验是否存在**不在本次删除集内**的子级（存在则整批拒绝）。
//! - `updateField`：字段名先映射到列名，再比对
//!   [`AUTHORITY_COLUMN_FIELDS`] 白名单（对应 Java `CheckFieldConstants.AUTHORITY_FIELDS`），
//!   不在白名单则报「字段值不合法」。
//!
//! ## 未迁移 / 降级（详见返回 JSON 的 `uncertain_points`）
//!
//! - `NexTradeProperties` 的版本开关（`is-overseas` / `is-merchant` / `is-pro` /
//!   `is-o2o` / `is-vendor` / `is-b2b` / `is-demo`）在 Rust 侧**无配置服务**，
//!   代码里是硬编码常量（`supplyChainCompiled = false`、`enabled = true`、
//!   `boolean = false`…）。因此与版本相关的整组菜单下架规则**只能在代码里
//!   按常量复刻**；切换版本必须改代码重新编译。
//! - 菜单名称国际化（`translatePackage.translate(name, DataType.PAGE)`）未迁移，
//!   `getAuthority` 返回的 `authorityName` 为数据库原文。
//! - `getAuthorityByName`（Java 公开方法，但本切片契约内无对应端点）未实现。
//! - 全局下架规则在三个方法间的**细微差异**已逐条对照复刻，
//!   见 [`MenuFilter::for_get_all_authority`] / [`MenuFilter::for_get_authority`] /
//!   [`MenuFilter::for_list`]。
//! - 平台「店铺会员总开关」（Java `ShopMemberSettingService.isEnabled()`）未迁移，
//!   故 `getAuthority` 中「店铺端隐藏
//!   `user` / `userManagement` / `shopMemberCustomerManage` / `memberTagManage`」
//!   这段规则**未复刻**（默认开关视为开启 → 不下架）。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use nextrade_core::entities::Authority;
use nextrade_core::models::identity::authority::{
    AuthorityChildDTO, AuthorityContext, AuthorityDTO, AuthorityListQuery, AuthorityTreeNode,
    AuthorityVO, BatchDTO, UpdateFieldDTO,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::identity::authority::{
    AuthorityPageFilter, AuthoritySelected, AuthorityRepository, AUTHORITY_COLUMN_FIELDS,
};
use nextrade_infrastructure::db::Repositories;

/// 账期支付菜单：父级 `termPayManagement` 及其三个子级。
///
/// 需整组列举——扁平返回的接口不建树，只删父级的话子级会漏出；
/// 且不能用 sn 前缀/包含匹配，`promoterManage` 小写后含 `"term"` 会被误删。
/// （逐字对齐 Java `AuthorityServiceImpl.TERM_PAY_MENU_SN`。）
pub const TERM_PAY_MENU_SN: &[&str] = &[
    "termPayManagement",
    "termOrderManage",
    "termPayManage",
    "termMemberManage",
];

/// 供应链菜单及其权限点。演示站只在供应商（S2B2C）、跨境或多门店版本展示。
/// （逐字对齐 Java `SUPPLY_CHAIN_MENU_SN`；该集合在运行时**恒定被移除**，
/// 因为 Java `isSupplyChainMenuEnabled()` 里 `supplyChainCompiled = false`。）
pub const SUPPLY_CHAIN_MENU_SN: &[&str] = &[
    "supplyChain",
    "purchaseManagement",
    "selfVendorManagement",
    "supplyChainSetting",
    "purchaseOrderManage",
    "purchaseOrderModifyManage",
    "purchaseReceiptManage",
    "purchaseReceiptModifyManage",
    "purchaseReturnManage",
    "purchaseReturnModifyManage",
    "purchaseReconciliationManage",
    "purchaseReconciliationModifyManage",
    "selfVendorManage",
    "selfVendorModifyManage",
    "selfVendorCategoryManage",
    "selfVendorCategoryModifyManage",
    "selfVendorProductManage",
    "selfVendorProductModifyManage",
    "purchaseSettingManage",
    "purchaseSettingModifyManage",
];

/// 旧版小票打印菜单（O2O 版本改由收银台设置承载）。
pub const LEGACY_RECEIPT_MENU_SN: &[&str] = &["baseReceiptManage", "shopBaseReceiptManage"];

/// 权限名称长度下限（Java `AuthorityDTO` 的 `@Size(max = 30, min = 1)`）。
pub const AUTHORITY_NAME_MIN_LEN: usize = 1;
/// 权限名称长度上限。
pub const AUTHORITY_NAME_MAX_LEN: usize = 30;

// ————————————————————— 版本开关（NexTradeProperties 的 Rust 替代） —————————————————————

/// 版本 / 平台开关，对应 Java `NexTradeProperties`。
///
/// **产品决策（2026-10-25，业主确认）**：Rust 后端为**全功能版**——
/// 接口能力永远完整，版本区分（跨境/批发/O2O/供应商等）由**前端**结合
/// 后续授权体系控制，后端不做接口级裁剪。因此 `Default` 全开。
///
/// 这些开关只影响**权限树展示**（`getAuthority`/`list`/`getAllAuthority`
/// 的菜单下架），不影响端点鉴权（端点权限来自 `@PreAuthorize` 契约表）。
/// 全开默认下按 Java 同配置语义仅下架两族互斥菜单：O2O 开启时的
/// 旧版小票（由收银台设置承载）与保证金菜单。
///
/// Java 里这些值来自 `application.yaml`（`nextrade.is-overseas` 等）；
/// Java 对照测试用 [`EditionFlags`] 的全零「国内演示版」显式构造
/// （见 tests 里的 `java_default`），勿依赖 `Default` 断言 Java 原生行为。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditionFlags {
    pub is_overseas: i32,
    pub is_merchant: i32,
    pub is_pro: i32,
    pub is_o2o: i32,
    pub is_vendor: i32,
    pub is_b2b: i32,
    pub is_demo: i32,
}

impl Default for EditionFlags {
    fn default() -> Self {
        // 当前部署需要展示授权表中的全部菜单：多商户、O2O、供应商、B2B、
        // 跨境、Pro 能力全部打开，避免菜单被版本常量过滤。
        Self {
            is_overseas: 1,
            is_merchant: 1,
            is_pro: 1,
            is_o2o: 1,
            is_vendor: 1,
            is_b2b: 1,
            is_demo: 0,
        }
    }
}

impl EditionFlags {
    /// Java `isWholesaleMenuEnabled()`：演示站按版本判断，产品环境恒为 `true`。
    #[must_use]
    pub fn wholesale_menu_enabled(&self) -> bool {
        if self.is_demo == 1 {
            return self.is_b2b == 1
                || self.is_vendor == 1
                || self.is_overseas == 1
                || self.is_o2o == 1;
        }
        // Java：`boolean enabled = false; enabled = true; return enabled;`
        true
    }

    /// Java `isSupplyChainMenuEnabled()`：当前部署要求展示供应链菜单。
    #[must_use]
    pub fn supply_chain_menu_enabled(&self) -> bool {
        true
    }

    /// Java `isPosMenuAuthority(authoritySn)`。
    #[must_use]
    pub fn is_pos_menu(authority_sn: &str) -> bool {
        authority_sn == "groupPos"
            || authority_sn.starts_with("pos")
            || authority_sn.starts_with("cashRegisterManage")
            || authority_sn.starts_with("cashierList")
    }

    /// Java `filterReceiptPrintMenuByEdition(list, isO2o() == 1)`：
    /// - O2O 开启 → 下架旧版小票菜单（由收银台设置承载），保留 POS 收银菜单；
    /// - O2O 关闭 → 下架 POS 收银菜单，保留旧版小票。
    ///
    /// 全功能部署（`Default`：全开）走 O2O 分支。
    #[must_use]
    pub fn keep_receipt_or_pos(&self, authority_sn: &str) -> bool {
        if self.is_o2o == 1 {
            !LEGACY_RECEIPT_MENU_SN.contains(&authority_sn)
        } else {
            !Self::is_pos_menu(authority_sn)
        }
    }

    /// 该权限是否应下架（返回 `true` 表示移除）。flags 的公共部分。
    fn should_remove_common(&self, sn: &str) -> bool {
        if self.is_overseas == 0
            && (sn == "multilingual"
                || sn == "mobileuserUserOverseasManage"
                || sn == "taxRateManage")
        {
            return true;
        }
        if self.is_merchant == 0
            && (sn == "shop"
                || sn == "shopManagement"
                || sn == "shopManage"
                || sn == "shopSettingsManage")
        {
            return true;
        }
        if self.is_pro == 0 && (sn == "salesman" || sn == "eCardManage" || sn == "userLevelProManage") {
            return true;
        }
        if !(self.is_o2o == 1 || self.is_overseas == 1 || self.is_vendor == 1 || self.is_b2b == 1)
            && (sn == "enquiryManage" || sn == "quotationManage")
        {
            return true;
        }
        if self.is_b2b == 0
            && (sn == "userCertificationManage"
                || sn == "realNameAuthentication"
                || sn == "userAuthenticationManage"
                || TERM_PAY_MENU_SN.contains(&sn))
        {
            return true;
        }
        if self.is_vendor == 0 && sn.contains("vendor") {
            return true;
        }
        if self.is_merchant == 0 && self.is_o2o == 0 && self.is_vendor == 0 && sn == "organize" {
            return true;
        }
        if self.is_merchant == 0
            && self.is_o2o == 0
            && (sn == "adminMerchant" || sn == "profitsharingManage" || sn == "shopUserfundsManagement")
        {
            return true;
        }
        if !self.wholesale_menu_enabled() && sn.contains("wholesale") {
            return true;
        }
        if !self.supply_chain_menu_enabled() && SUPPLY_CHAIN_MENU_SN.contains(&sn) {
            return true;
        }
        false
    }

    /// O2O 版本额外移除的菜单（`getAuthority` / `list` 有，`getAllAuthority` 无）。
    fn should_remove_o2o_only(&self, sn: &str) -> bool {
        if self.is_o2o == 0 {
            return false;
        }
        sn.contains("deposit") || sn.contains("Deposit")
    }

    /// 商家版本为 0 时额外移除的菜单（`getAuthority` / `list` 有）。
    fn should_remove_merchant_deposit(&self, sn: &str) -> bool {
        if self.is_merchant != 0 {
            return false;
        }
        sn.contains("deposit") || sn.contains("Deposit")
    }

    /// 非 O2O 版本移除的门店/配送族菜单。
    ///
    /// `getAllAuthority` 与 `getAuthority`/`list` 的清单**不同**
    /// （前者多 `intraCityDeliveryManage` 两次、少 `StoreDecorate` /
    /// `staffFundManagement` / `cycleBuyManage`），由 `variant` 区分。
    fn should_remove_non_o2o(&self, sn: &str, variant: FilterVariant) -> bool {
        if self.is_o2o != 0 {
            return false;
        }
        if sn.contains("store") || sn == "storeEntryRuleSetting" {
            return true;
        }
        match variant {
            FilterVariant::AllAuthority => {
                sn.contains("intraCityDeliveryManage")
                    || sn.contains("deliverymanList")
                    || sn.contains("servicemanList")
                    || sn.contains("homeServiceManage")
                    || sn.contains("cycleBuyManage")
            }
            FilterVariant::Authority | FilterVariant::List => {
                sn.contains("intraCityDeliveryManage")
                    || sn.contains("StoreDecorate")
                    || sn.contains("staffFundManagement")
                    || sn.contains("deliverymanList")
                    || sn.contains("servicemanList")
                    || sn.contains("homeServiceManage")
                    || sn.contains("cycleBuyManage")
            }
        }
    }
}

/// 三个方法的下架清单差异（Java 源码逐条对照结果）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilterVariant {
    /// `getAllAuthority`
    AllAuthority,
    /// `getAuthority`
    Authority,
    /// `list`（在 `getAuthority` 基础上多 `cycleBuyManage`）
    List,
}

/// 菜单下架过滤器。
#[derive(Debug, Clone, Copy, Default)]
pub struct MenuFilter;

impl MenuFilter {
    /// `getAllAuthority` 的下架规则。
    #[must_use]
    pub fn for_get_all_authority(flags: EditionFlags) -> impl Fn(&str) -> bool {
        move |sn: &str| {
            flags.should_remove_common(sn)
                || flags.should_remove_non_o2o(sn, FilterVariant::AllAuthority)
                || !flags.keep_receipt_or_pos(sn)
        }
    }

    /// `getAuthority` 的下架规则。    #[must_use]
    pub fn for_get_authority(flags: EditionFlags) -> impl Fn(&str) -> bool {
        move |sn: &str| {
            flags.should_remove_common(sn)
                || flags.should_remove_non_o2o(sn, FilterVariant::Authority)
                || !flags.keep_receipt_or_pos(sn)
                || flags.should_remove_o2o_only(sn)
                || flags.should_remove_merchant_deposit(sn)
        }
    }

    /// `list` 的下架规则。
    #[must_use]
    pub fn for_list(flags: EditionFlags) -> impl Fn(&str) -> bool {
        move |sn: &str| {
            flags.should_remove_common(sn)
                || flags.should_remove_non_o2o(sn, FilterVariant::List)
                || !flags.keep_receipt_or_pos(sn)
                || flags.should_remove_o2o_only(sn)
                || flags.should_remove_merchant_deposit(sn)
        }
    }
}

// ————————————————————— 纯逻辑辅助函数 —————————————————————

/// `child_auth`（`Vec<AuthorityChildDTO>`）→ JSON 文本。
///
/// Java：`Optional.ofNullable(list).filter(!isEmpty).map(JSONUtil::toJsonStr).orElse("[]")`
/// → `None` / 空表 / 解析失败都落 `"[]"`。
#[must_use]
pub fn encode_child_auth(list: Option<&[AuthorityChildDTO]>) -> String {
    match list {
        Some(items) if !items.is_empty() => {
            serde_json::to_string(items).unwrap_or_else(|_| "[]".to_string())
        }
        _ => "[]".to_string(),
    }
}

/// `child_auth` JSON 文本 → `Vec<AuthorityChildDTO>`。
///
/// Java：`StringUtils.isNotBlank(childAuth)` 时才反序列化，否则字段保持 `null`
/// （Hutool `Tree` 的 `childAuth` 则为**空列表**，见 [`build_tree`]）。
#[must_use]
pub fn decode_child_auth(raw: Option<&str>) -> Option<Vec<AuthorityChildDTO>> {
    let text = raw?;
    if text.trim().is_empty() {
        return None;
    }
    match serde_json::from_str::<Vec<AuthorityChildDTO>>(text) {
        Ok(items) => Some(items),
        Err(err) => {
            tracing::warn!(child_auth = text, error = %err, "child_auth 反序列化失败");
            None
        }
    }
}

/// 建树用的 `childAuth` 列表（Java `convertTreeList`：空串或 `"[]"` → 空表）。
#[must_use]
pub fn child_auth_for_tree(raw: Option<&str>) -> Vec<AuthorityChildDTO> {
    match raw {
        Some(text) if !text.trim().is_empty() && text.trim() != "[]" => {
            serde_json::from_str::<Vec<AuthorityChildDTO>>(text).unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

/// 角色 / 绑定行的 `auth_list` JSON 文本 → `Vec<String>`。
#[must_use]
pub fn parse_authority_list(raw: Option<&str>) -> Vec<String> {
    let Some(text) = raw else {
        return Vec::new();
    };
    if text.trim().is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<Vec<String>>(text) {
        Ok(list) => list,
        Err(err) => {
            tracing::warn!(auth_list = text, error = %err, "auth_list 反序列化失败");
            Vec::new()
        }
    }
}

/// Java `hasAllAuthority = CollUtil.isEmpty(authorities) || any("all")`。
#[must_use]
pub fn is_unrestricted(authorities: &[String]) -> bool {
    authorities.is_empty() || authorities.iter().any(|a| a == "all")
}

/// Java `AdminTypeEnum` 的合法 `code` 集合
/// （`admin` / `shop` / `vendor` / `store` / `pickup` / `pos`；
/// `suppliers` 不在枚举内，调用点会先经 [`normalize_admin_type`] 归一为 `admin`）。
///
/// 注意：Java `AdminTypeEnum.fromCode` 对未知值**抛 `IllegalArgumentException`**
/// （并非返回 null），`getAllAuthority` 会因此 500。Rust 侧选择「记录告警后按平台账号
/// 继续处理」以免把 JWT 字段差异放大成 500 —— 已列入 `uncertain_points`。
#[must_use]
pub fn is_known_admin_type(t: &str) -> bool {
    matches!(t, "admin" | "shop" | "vendor" | "store" | "pickup" | "pos")
}

/// `admin_type` 归一化：`suppliers` → `admin`（Java 两处均如此）。
#[must_use]
pub fn normalize_admin_type(t: &str) -> &str {
    if t == "suppliers" {
        "admin"
    } else {
        t
    }
}

/// `batchDel` 的删除顺序：按「祖先也在删除集内的层数」**降序**
/// （深度大的子级先删），深度相同按原顺序稳定排序。
///
/// 对应 Java `calculateDepth` + `sorted((a1,a2) -> Integer.compare(depth2, depth1))`。
/// Java 的 `Stream.sorted` 是稳定排序，Rust `sort_by` 同样稳定。
#[must_use]
pub fn sort_ids_by_depth_desc(authorities: &[Authority]) -> Vec<i32> {
    let by_id: HashMap<i32, &Authority> = authorities.iter().map(|a| (a.authority_id, a)).collect();
    let mut items: Vec<(i32, usize)> = authorities
        .iter()
        .map(|a| (a.authority_id, depth_in_set(a, &by_id)))
        .collect();
    items.sort_by(|a, b| b.1.cmp(&a.1));
    items.into_iter().map(|(id, _)| id).collect()
}

/// 计算某权限在删除集内的深度（Java `calculateDepth`）。
fn depth_in_set(a: &Authority, by_id: &HashMap<i32, &Authority>) -> usize {
    let mut depth = 0usize;
    let mut parent_id = a.parent_id;
    // Java `while (parentId != null && parentId != 0 && map.containsKey(parentId))`
    // 递归深度最多等于集合大小；这里加硬上限防止脏数据形成环。
    let limit = by_id.len() + 1;
    while parent_id != 0 && depth <= limit {
        let Some(parent) = by_id.get(&parent_id) else {
            break;
        };
        depth += 1;
        parent_id = parent.parent_id;
    }
    depth
}

/// `updateField` 的字段映射 + 白名单校验。
///
/// 依次复刻 Java `BaseServiceImpl#updateField`：
/// 1. `getColumnByPropertyName(field)`：属性名 / 列名 → 列名，
///    都匹配不到时**回退主键列**（安全修复，防 SQL 注入）；
/// 2. `Arrays.asList(allowFields).contains(column)` 否则报「字段值不合法」。
///
/// 因此请求字段为**列名**（如 `authority_name`）时同样命中 `authority_name`
/// 并在白名单内 → 更新成功（Java 不退化为主键列）；只有**完全未知**的字段
/// 才回退主键列 `authority_id`，而白名单不含主键列 → 抛「字段值不合法」，
/// 这条回退路径同时充当防 SQL 注入的兜底。
pub fn map_update_field(field: &str, allowed: &[&str]) -> Result<&'static str> {
    let key = "authority_id";
    let column = nextrade_core::models::identity::authority::authority_field_to_column(field.trim())
        .unwrap_or(key);
    if !allowed.contains(&column) {
        return Err(Error::biz_msg("字段值不合法"));
    }
    Ok(column)
}

/// `AuthorityDTO` 的字段校验（Java `@NotNull` / `@Size`）。
pub fn validate_dto(dto: &AuthorityDTO, require_id: bool) -> Result<()> {
    if require_id && dto.authority_id.is_none() {
        return Err(Error::Validation("权限id不能为空".into()));
    }
    let sn = dto.authority_sn.as_deref().unwrap_or("").trim();
    if sn.is_empty() {
        return Err(Error::Validation("权限编号不能为空".into()));
    }
    let name = dto.authority_name.as_deref().unwrap_or("");
    let name_len = name.chars().count();
    if !(AUTHORITY_NAME_MIN_LEN..=AUTHORITY_NAME_MAX_LEN).contains(&name_len) {
        return Err(Error::Validation("权限名称长度为1-30位".into()));
    }
    Ok(())
}

// ————————————————————— 服务 —————————————————————

pub struct AuthorityService {
    /// 共享仓储集合（保留完整句柄，便于后续接入 `shop_member_setting` 等跨域依赖）。
    repos: Arc<Repositories>,
    /// 版本开关（Java `NexTradeProperties`；Rust 侧为常量，见 [`EditionFlags`]）。
    flags: EditionFlags,
    /// 本域仓储句柄（`authority` / `admin_user_shop` / `admin_user_vendor` /
    /// `admin_role` / `admin_user`）。
    repo: AuthorityRepository,
}

impl AuthorityService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        let repo = repos.identity.authority.clone();
        Self {
            repos,
            flags: EditionFlags::default(),
            repo,
        }
    }

    /// 共享仓储集合（只读访问）。
    #[must_use]
    pub fn repos(&self) -> &Repositories {
        &self.repos
    }

    /// 本域仓储句柄。
    #[must_use]
    pub fn repo(&self) -> &AuthorityRepository {
        &self.repo
    }

    /// 当前版本开关。
    #[must_use]
    pub fn flags(&self) -> EditionFlags {
        self.flags
    }

    /// 自定义版本开关（供测试与后续接入配置服务）。
    #[must_use]
    pub fn with_flags(mut self, flags: EditionFlags) -> Self {
        self.flags = flags;
        self
    }

    // ————————————————————— 列表 —————————————————————

    /// 权限分页（Java `AuthorityServiceImpl#list`）。
    pub async fn list(&self, q: &AuthorityListQuery) -> Result<PageResult<AuthorityDTO>> {
        let (page, size) = q.page_params();
        let (_, offset) = q.limit_offset();
        let filter = AuthorityPageFilter {
            admin_type: q.admin_type.clone(),
            parent_id: q.parent_id,
            sort_field: q.sort_field.clone(),
            sort_order: q.sort_order.clone(),
        };
        let (total, records) = self.repo.paginate(&filter, size as i64, offset).await?;
        if records.is_empty() {
            return Ok(PageResult::new(total, Vec::new(), &page_query(page, size)));
        }

        let ids: Vec<i32> = records.iter().map(|r| r.authority_id).collect();
        let children = self.repo.find_children_of(&ids).await?;

        // Java 在分页**之后**才 removeIf，故 total 仍是未过滤的计数。
        let remove = MenuFilter::for_list(self.flags);
        let mut out: Vec<AuthorityDTO> = Vec::with_capacity(records.len());
        for a in &records {
            if remove(a.authority_sn.as_str()) {
                continue;
            }
            let mut dto = dto_from_authority(a);
            dto.has_children = Some(
                children
                    .iter()
                    .filter(|c| c.parent_id == a.authority_id)
                    .count() as i64,
            );
            out.push(dto);
        }

        Ok(PageResult::new(total, out, &page_query(page, size)))
    }

    // ————————————————————— 详情 —————————————————————

    /// 权限详情（Java `AuthorityServiceImpl#detail`，不存在 → 「权限不存在」）。
    pub async fn detail(&self, authority_id: i32) -> Result<AuthorityDTO> {
        let a = self.repo.find_or_err(authority_id).await?;
        Ok(dto_from_authority(&a))
    }

    // ————————————————————— 新建 / 更新 —————————————————————

    /// 新建权限（Java `create` → `save(buildChildAuth(dto))`）。
    pub async fn create(&self, dto: &AuthorityDTO) -> Result<i32> {
        validate_dto(dto, false)?;
        let entity = self.build_entity(dto).await?;
        self.repo.insert(&entity).await
    }

    /// 更新权限（Java `update` → `updateById(buildChildAuth(dto))`）。
    pub async fn update(&self, dto: &AuthorityDTO) -> Result<()> {
        validate_dto(dto, true)?;
        let entity = self.build_entity(dto).await?;
        let affected = self.repo.update(&entity).await?;
        if affected == 0 {
            // Java `updateById` 命中 0 行也返回 false，Controller 不检查返回值；
            // Rust 侧显式报错，避免「静默成功」掩盖主键写错。
            return Err(Error::biz_msg("权限不存在"));
        }
        Ok(())
    }

    /// `buildChildAuth(AuthorityDTO) -> Authority`：
    /// 唯一性校验 + 子权限 JSON 归一化（Java 同名方法）。
    async fn build_entity(&self, dto: &AuthorityDTO) -> Result<Authority> {
        let sn = dto.authority_sn.as_deref().unwrap_or("").trim().to_string();
        let admin_type = dto.admin_type.clone().unwrap_or_default();
        let dup = self
            .repo
            .count_same_sn(&sn, &admin_type, dto.authority_id)
            .await?;
        if dup > 0 {
            return Err(Error::biz_msg("权限编号已存在"));
        }

        // 缺省值：Java 实体为包装类型，null 会写入 NULL；但 `authority` 表
        // `sort_order` / `is_show` / `is_system` / `route_link` / `authority_ico`
        // 均为 NOT NULL，故给 0 / 空串（与 MyBatis-Plus 默认插入行为一致性说明见
        // `uncertain_points`）。
        Ok(Authority {
            authority_id: dto.authority_id.unwrap_or(0),
            authority_sn: sn,
            authority_name: dto.authority_name.clone().unwrap_or_default(),
            parent_id: dto.parent_id.unwrap_or(0),
            sort_order: dto.sort_order.unwrap_or(0),
            is_show: dto.is_show.unwrap_or(0),
            child_auth: Some(encode_child_auth(dto.child_auth.as_deref())),
            route_link: dto.route_link.clone().unwrap_or_default(),
            authority_ico: dto.authority_ico.clone().unwrap_or_default(),
            is_system: dto.is_system.unwrap_or(0),
            admin_type,
        })
    }

    // ————————————————————— 删除 —————————————————————

    /// 删除权限（Java `AuthorityController#del`：**不能有子级**）。
    pub async fn del(&self, id: i32) -> Result<()> {
        if self.repo.count_children(id).await? > 0 {
            // 对应消息键 `error.authority.has.children`
            return Err(Error::biz_msg("请先删除该权限的子权限"));
        }
        self.repo.delete_by_id(id).await?;
        Ok(())
    }

    /// 批量操作（Java `AuthorityController#batch`）：
    /// `type == "del"` 走 [`AuthorityService::batch_del`]，否则对应
    /// `BaseServiceImpl#batch`（非 `del` 类型直接返回 `false`，即无操作）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<()> {
        if dto.r#type == "del" {
            self.batch_del(&dto.ids).await
        } else {
            // Java：`return false;`（`BaseServiceImpl#batch`）
            Err(Error::biz_msg("不支持的批量操作类型"))
        }
    }

    /// 批量删除（Java `AuthorityServiceImpl#batchDel`，`@Transactional`）。
    ///
    /// 先按深度倒序排定删除顺序，再**逐条**校验是否存在不在本次删除集内的子级；
    /// 任一条不合格则整批拒绝（Java 抛异常触发事务回滚，此处尚未执行任何删除）。
    pub async fn batch_del(&self, ids: &[i32]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let authorities = self.repo.find_by_ids(ids).await?;
        if authorities.is_empty() {
            return Ok(());
        }
        let sorted = sort_ids_by_depth_desc(&authorities);
        let name_by_id: HashMap<i32, &str> = authorities
            .iter()
            .map(|a| (a.authority_id, a.authority_name.as_str()))
            .collect();

        for id in &sorted {
            if self.repo.find_blocking_child(*id, ids).await?.is_some() {
                let name = name_by_id.get(id).copied().unwrap_or("");
                return Err(Error::biz_msg(format!(
                    "权限[{name}]下有子权限,无法删除"
                )));
            }
        }
        for id in &sorted {
            self.repo.delete_by_id(*id).await?;
        }
        Ok(())
    }

    // ————————————————————— 单字段更新 —————————————————————

    /// 单字段更新（Java `updateField(updateField, AUTHORITY_FIELDS)`）。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<()> {
        let column = map_update_field(&dto.field, AUTHORITY_COLUMN_FIELDS)?;
        self.repo.update_field(dto.id, column, &dto.val).await?;
        Ok(())
    }

    // ————————————————————— 父级名称 —————————————————————

    /// 父级权限名称（Java `getAuthorityParentName`）。
    ///
    /// Java 实现存在缺陷：`getOne(...select(authorityName))` 返回的实体
    /// **未查询 `parent_id`**，随后 `authority.getParentId()` 必为 `null`，
    /// 因此永远返回 `null`；且 `authority == null` 时直接 NPE。
    /// Rust 侧按**意图**实现（取父级名称），并对缺失记录返回 `None`。
    pub async fn get_authority_parent_name(&self, id: i32) -> Result<Option<String>> {
        let Some(parent_id) = self.repo.find_parent_id(id).await? else {
            return Ok(None);
        };
        if parent_id <= 0 {
            return Ok(None);
        }
        self.repo.find_name_by_id(parent_id).await
    }

    // ————————————————————— 当前用户可见权限 —————————————————————

    /// 解析当前账号的权限编号集合 + 生效的 `admin_type`
    /// （Java `getAllAuthority` / `getAuthority` 里那段 shop/vendor 分支）。
    ///
    /// 返回 `(admin_type, authorities)`；`authorities` 含 `all` 或为空表时
    /// 表示不受限（由调用方通过 [`is_unrestricted`] 判定）。
    async fn resolve_authorities(&self, ctx: &AuthorityContext) -> Result<(String, Vec<String>)> {
        let mut admin_type = ctx
            .admin_type
            .clone()
            .unwrap_or_else(|| "admin".to_string());
        if admin_type == "suppliers" {
            admin_type = "admin".to_string();
        }
        let mut authorities = ctx.granted.clone();

        // Java：suppliers 归一为 admin 后不再走店铺/供应商分支
        let original = ctx
            .admin_type
            .as_deref()
            .map(normalize_admin_type)
            .unwrap_or("admin");
        if !is_known_admin_type(original) {
            // Java `AdminTypeEnum.fromCode` 对未知值返回 null，后续分支全部跳过
            tracing::warn!(admin_type = original, "未知的后台账号类型，按平台账号处理");
        }

        let shop_id = ctx.shop_id.unwrap_or(0);
        if matches!(original, "shop" | "store" | "pickup") && shop_id > 0 {
            admin_type = "shop".to_string();
            if let Some((role_id, auth_list)) =
                self.repo.find_shop_binding(ctx.admin_id, shop_id).await?
            {
                authorities = self.authorities_from_binding(role_id, auth_list).await?;
            }
        }

        let vendor_id = ctx.vendor_id.unwrap_or(0);
        if vendor_id > 0 {
            admin_type = "vendor".to_string();
            if let Some((role_id, auth_list)) =
                self.repo.find_vendor_binding(ctx.admin_id, vendor_id).await?
            {
                authorities = self.authorities_from_binding(role_id, auth_list).await?;
            }
        }

        // 未迁移依赖降级：Java 的权限项来自 Spring Security 的
        // `Authentication.getAuthorities()`（登录时由 `AdminUser.auth_list`
        // 或角色 `admin_role.authority_list` 填充）。Rust JWT 未签发 authority 列表，
        // 故在上下文为空时回查 `admin_user`（`auth_list` 即 Java 登录时装载的同一份）。
        if authorities.is_empty() {
            if let Some((_, auth_list)) = self.repo.find_admin_auth(ctx.admin_id).await? {
                authorities = parse_authority_list(auth_list.as_deref());
            }
        }

        Ok((admin_type, authorities))
    }

    /// 角色优先、其次绑定行 `auth_list`（Java 两处分支的共同逻辑）。
    async fn authorities_from_binding(
        &self,
        role_id: i32,
        auth_list: Option<String>,
    ) -> Result<Vec<String>> {
        if role_id > 0 {
            if let Some(list) = self.repo.find_role_authority_list(role_id).await? {
                return Ok(parse_authority_list(Some(list.as_str())));
            }
        }
        Ok(parse_authority_list(auth_list.as_deref()))
    }

    /// 所有权限（树）（Java `getAllAuthority`）。
    ///
    /// `query_admin_type` 对应 Java 里由前端传入的 `adminType` 请求参数
    /// （仅当当前账号是 `admin` 且该参数非空时生效）。
    pub async fn get_all_authority(
        &self,
        ctx: &AuthorityContext,
        query_admin_type: Option<&str>,
    ) -> Result<Option<Vec<AuthorityTreeNode>>> {
        let (mut admin_type, authorities) = self.resolve_authorities(ctx).await?;
        // Java：`AdminTypeEnum.fromCode(adminType) == ADMIN && isNotEmpty(requestAdminType)`
        let current_is_admin =
            normalize_admin_type(ctx.admin_type.as_deref().unwrap_or("admin")) == "admin";
        if current_is_admin {
            if let Some(t) = query_admin_type.map(str::trim).filter(|t| !t.is_empty()) {
                admin_type = t.to_string();
            }
        }

        let unrestricted = is_unrestricted(&authorities);
        let sn_filter = (!unrestricted).then(|| authorities.clone());
        let rows = self
            .repo
            .find_visible(&admin_type, sn_filter.as_deref(), false)
            .await?;

        let remove = MenuFilter::for_get_all_authority(self.flags);
        let kept: Vec<AuthoritySelected> = rows
            .into_iter()
            .filter(|r| !remove(r.authority_sn.as_str()))
            .collect();

        // Java：`convertTreeList` 在空表时返回 null
        if kept.is_empty() {
            return Ok(None);
        }
        Ok(Some(build_tree(&kept, TREE_MAX_DEPTH)))
    }

    /// 当前账号权限列表（扁平）（Java `getAuthority`）。
    pub async fn get_authority(&self, ctx: &AuthorityContext) -> Result<Vec<AuthorityVO>> {
        // Java `getAuthority` 的 adminType 取自 **adminUser.getAdminType()**
        // （Redis 缓存的登录用户），而非请求头；Rust 侧 JWT 已带 `typ`，
        // 但为对齐「以库为准」语义，在 JWT 未提供时回查 `admin_user`。
        let mut effective = ctx.clone();
        if effective.admin_type.is_none() {
            if let Some((admin_type, _)) = self.repo.find_admin_auth(ctx.admin_id).await? {
                effective.admin_type = Some(admin_type);
            }
        }
        let (admin_type, authorities) = self.resolve_authorities(&effective).await?;

        // Java：`if (ObjectUtil.isEmpty(authorities)) return new ArrayList<>();`
        if authorities.is_empty() {
            return Ok(Vec::new());
        }
        let unrestricted = is_unrestricted(&authorities);
        let sn_filter = (!unrestricted).then(|| authorities.clone());
        let rows = self
            .repo
            .find_visible(&admin_type, sn_filter.as_deref(), true)
            .await?;

        let remove = MenuFilter::for_get_authority(self.flags);
        Ok(rows
            .into_iter()
            .filter(|r| !remove(r.authority_sn.as_str()))
            .map(vo_from_selected)
            .collect())
    }
}

/// Java `TreeNodeConfig#setDeep(5)`。
pub const TREE_MAX_DEPTH: u32 = 5;

/// `Authority` → `AuthorityDTO`（Java `buildChildAuth(Authority)`）。
#[must_use]
pub fn dto_from_authority(a: &Authority) -> AuthorityDTO {
    AuthorityDTO {
        authority_id: Some(a.authority_id),
        authority_sn: Some(a.authority_sn.clone()),
        authority_name: Some(a.authority_name.clone()),
        parent_id: Some(a.parent_id),
        sort_order: Some(a.sort_order),
        has_children: None,
        is_show: Some(a.is_show),
        child_auth: decode_child_auth(a.child_auth.as_deref()),
        route_link: Some(a.route_link.clone()),
        authority_ico: Some(a.authority_ico.clone()),
        is_system: Some(a.is_system),
        admin_type: Some(a.admin_type.clone()),
    }
}

/// 部分列投影 → `AuthorityVO`。
#[must_use]
pub fn vo_from_selected(r: AuthoritySelected) -> AuthorityVO {
    AuthorityVO {
        authority_id: r.authority_id,
        authority_sn: r.authority_sn,
        authority_name: r.authority_name,
        parent_id: r.parent_id,
        sort_order: r.sort_order,
        is_show: r.is_show,
        child_auth: r.child_auth,
        route_link: r.route_link,
        authority_ico: r.authority_ico,
        // Java 该接口只 select 了 sn / routeLink / name / parentId，
        // 未查询 `is_system`；MyBatis-Plus 实体默认值为 null。
        is_system: 0,
        // 同上：Java 未 select `admin_type`，序列化为 null；
        // Rust 无 null 语义的 String 字段，用空串表示。
        admin_type: String::new(),
    }
}

/// 列表接口把 `(page, size)` 装回 `PageQuery` 以复用 [`PageResult`]。
fn page_query(page: u64, size: u64) -> nextrade_core::types::PageQuery {
    nextrade_core::types::PageQuery {
        page,
        page_size: size,
    }
}

/// 构建 Hutool 风格权限树（Java `convertTreeList`）。
///
/// - 根节点：`parent_id == 0`（`TreeUtil.build(list, 0, config, ...)`）；
/// - 深度上限 `deep`（Java `config.setDeep(5)`）；
/// - 节点字段 = Hutool 默认键（`id` / `name` / `weight`）+ `putExtra` 的
///   `authorityId` / `parentId` / `authorityName` / `sortOrder` / `isShow` /
///   `childAuth` / `routeLink` / `authorityIco` / `authoritySn`。
///
/// 入参顺序即列表顺序（SQL 已按 `parent_id, sort_order, authority_id` 排序），
/// 因此同级节点顺序与原 Java 一致。
#[must_use]
pub fn build_tree(rows: &[AuthoritySelected], max_depth: u32) -> Vec<AuthorityTreeNode> {
    let mut children_of: HashMap<i32, Vec<usize>> = HashMap::new();
    for (idx, r) in rows.iter().enumerate() {
        children_of.entry(r.parent_id).or_default().push(idx);
    }
    build_children(&children_of, rows, 0, max_depth, 0, &mut HashSet::new())
}

/// 递归构建子树；`exclude_root` 为当前根的 id（`TreeUtil.build` 不把 root 自己挂进去）。
fn build_children(
    children_of: &HashMap<i32, Vec<usize>>,
    rows: &[AuthoritySelected],
    parent_id: i32,
    max_depth: u32,
    depth: u32,
    visited: &mut HashSet<i32>,
) -> Vec<AuthorityTreeNode> {
    if depth >= max_depth {
        return Vec::new();
    }
    let Some(indexes) = children_of.get(&parent_id) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(indexes.len());
    for idx in indexes {
        let r = &rows[*idx];
        // 防御脏数据（自引用 / 环）：Hutool 通过 deep 限制同样不会无限递归。
        if !visited.insert(r.authority_id) {
            continue;
        }
        let children = build_children(
            children_of,
            rows,
            r.authority_id,
            max_depth,
            depth + 1,
            visited,
        );
        out.push(node_from(r, children));
        visited.remove(&r.authority_id);
    }
    out
}

/// 单个树节点。
fn node_from(r: &AuthoritySelected, children: Vec<AuthorityTreeNode>) -> AuthorityTreeNode {
    let mut extra = serde_json::Map::with_capacity(9);
    extra.insert("authorityId".into(), serde_json::json!(r.authority_id));
    extra.insert("parentId".into(), serde_json::json!(r.parent_id));
    extra.insert(
        "authorityName".into(),
        serde_json::json!(r.authority_name),
    );
    extra.insert("sortOrder".into(), serde_json::json!(r.sort_order));
    extra.insert("isShow".into(), serde_json::json!(r.is_show));
    extra.insert(
        "childAuth".into(),
        serde_json::to_value(child_auth_for_tree(r.child_auth.as_deref()))
            .unwrap_or(serde_json::Value::Array(Vec::new())),
    );
    extra.insert("routeLink".into(), serde_json::json!(r.route_link));
    extra.insert("authorityIco".into(), serde_json::json!(r.authority_ico));
    extra.insert("authoritySn".into(), serde_json::json!(r.authority_sn));

    AuthorityTreeNode {
        id: r.authority_id,
        name: r.authority_name.clone(),
        weight: r.sort_order,
        children: (!children.is_empty()).then_some(children),
        extra,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: i32, sn: &str, parent_id: i32, sort: i32, child_auth: Option<&str>) -> AuthoritySelected {
        AuthoritySelected {
            authority_id: id,
            authority_sn: sn.to_string(),
            authority_name: format!("菜单{id}"),
            parent_id,
            sort_order: sort,
            is_show: 1,
            child_auth: child_auth.map(str::to_string),
            route_link: format!("/route/{id}"),
            authority_ico: String::new(),
        }
    }

    // ————— child_auth 编解码 —————

    #[test]
    fn encode_child_auth_normalizes_empty_to_empty_json_array() {
        assert_eq!(encode_child_auth(None), "[]");
        assert_eq!(encode_child_auth(Some(&[])), "[]");
        let items = vec![AuthorityChildDTO {
            auth_name: Some("查看".into()),
            auth_sn: Some("view".into()),
        }];
        let encoded = encode_child_auth(Some(&items));
        assert_eq!(encoded, r#"[{"authName":"查看","authSn":"view"}]"#);
    }

    #[test]
    fn decode_child_auth_blank_is_none_and_roundtrips() {
        assert!(decode_child_auth(None).is_none());
        assert!(decode_child_auth(Some("")).is_none());
        assert!(decode_child_auth(Some("   ")).is_none());
        // 非法 JSON：Java 会抛异常；Rust 记录告警并返回 None
        assert!(decode_child_auth(Some("{not json")).is_none());
        let items = decode_child_auth(Some(r#"[{"authSn":"view"}]"#)).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].auth_sn.as_deref(), Some("view"));
        assert_eq!(items[0].auth_name, None);
    }

    #[test]
    fn child_auth_for_tree_treats_empty_array_as_empty() {
        assert!(child_auth_for_tree(Some("[]")).is_empty());
        assert!(child_auth_for_tree(None).is_empty());
        assert_eq!(child_auth_for_tree(Some(r#"[{"authSn":"a"}]"#)).len(), 1);
    }

    // ————— 权限集合判定 —————

    #[test]
    fn unrestricted_matches_java_collutil_and_all_check() {
        assert!(is_unrestricted(&[]));
        assert!(is_unrestricted(&["all".to_string()]));
        assert!(is_unrestricted(&["a".into(), "all".into()]));
        assert!(!is_unrestricted(&["a".into()]));
    }

    #[test]
    fn parse_authority_list_handles_bad_json() {
        assert!(parse_authority_list(None).is_empty());
        assert!(parse_authority_list(Some("")).is_empty());
        assert!(parse_authority_list(Some("nope")).is_empty());
        assert_eq!(
            parse_authority_list(Some(r#"["a","b"]"#)),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn admin_type_normalization() {
        assert_eq!(normalize_admin_type("suppliers"), "admin");
        assert_eq!(normalize_admin_type("shop"), "shop");
        assert!(is_known_admin_type("pickup"));
        assert!(!is_known_admin_type("unknown"));
    }

    // ————— 参数校验 —————

    #[test]
    fn validate_dto_enforces_java_constraints() {
        let mut dto = AuthorityDTO::default();
        assert!(validate_dto(&dto, false).is_err()); // sn 为空
        dto.authority_sn = Some("dashboard".into());
        assert!(validate_dto(&dto, false).is_err()); // name 为空
        dto.authority_name = Some("仪表盘".into());
        assert!(validate_dto(&dto, false).is_ok());
        assert!(validate_dto(&dto, true).is_err()); // update 需 id
        dto.authority_id = Some(3);
        assert!(validate_dto(&dto, true).is_ok());

        // 名称长度 1-30（按字符计数）
        dto.authority_name = Some("字".repeat(30));
        assert!(validate_dto(&dto, false).is_ok());
        dto.authority_name = Some("字".repeat(31));
        assert!(validate_dto(&dto, false).is_err());
        dto.authority_name = Some("  ".into());
        assert!(validate_dto(&dto, false).is_ok()); // 空白串长度合法（Java @Size 只数长度）
    }

    // ————— updateField 白名单 —————

    #[test]
    fn update_field_whitelist_matches_java_check_field_constants() {
        // 属性名 → 列名 → 白名单
        assert_eq!(
            map_update_field("authorityName", AUTHORITY_COLUMN_FIELDS).unwrap(),
            "authority_name"
        );
        assert_eq!(
            map_update_field("sortOrder", AUTHORITY_COLUMN_FIELDS).unwrap(),
            "sort_order"
        );
        assert_eq!(
            map_update_field("isShow", AUTHORITY_COLUMN_FIELDS).unwrap(),
            "is_show"
        );
        assert_eq!(
            map_update_field("routeLink", AUTHORITY_COLUMN_FIELDS).unwrap(),
            "route_link"
        );
        assert_eq!(
            map_update_field("authoritySn", AUTHORITY_COLUMN_FIELDS).unwrap(),
            "authority_sn"
        );
        // 不在白名单：authority_ico / admin_type / parent_id / child_auth
        assert!(map_update_field("authorityIco", AUTHORITY_COLUMN_FIELDS).is_err());
        assert!(map_update_field("adminType", AUTHORITY_COLUMN_FIELDS).is_err());
        assert!(map_update_field("parentId", AUTHORITY_COLUMN_FIELDS).is_err());
        // 传入**列名**时同样命中白名单：Java `getColumnByPropertyName` 的判据是
        // `fieldInfo.getProperty().equals(name) || fieldInfo.getColumn().equals(name)`，
        // 即属性名与列名都接受，故 `authority_name` 合法解析为 `authority_name`
        // 并在白名单内 → `Ok`（Java 在此**不**退化为主键列）。
        assert_eq!(
            map_update_field("authority_name", AUTHORITY_COLUMN_FIELDS).unwrap(),
            "authority_name"
        );
        // 完全未知字段退化为「主键列」→ 白名单不含主键 → 报错（防注入路径）
        assert!(map_update_field("1=1 OR 1=1", AUTHORITY_COLUMN_FIELDS).is_err());
    }

    // ————— batchDel 深度排序 —————

    fn auth(id: i32, parent_id: i32, name: &str) -> Authority {
        Authority {
            authority_id: id,
            authority_sn: format!("sn{id}"),
            authority_name: name.to_string(),
            parent_id,
            ..Default::default()
        }
    }

    #[test]
    fn batch_delete_orders_children_before_parents() {
        // 1(root) -> 2 -> 3，删除集 {1,2,3}：深度 3>2>1 → 3,2,1
        let list = vec![auth(1, 0, "根"), auth(2, 1, "子"), auth(3, 2, "孙")];
        assert_eq!(sort_ids_by_depth_desc(&list), vec![3, 2, 1]);
    }

    #[test]
    fn batch_delete_unrelated_parent_not_counted_in_depth() {
        // 5 的父级 9 不在删除集内 → 深度 0
        let list = vec![auth(4, 0, "A"), auth(5, 9, "B"), auth(6, 4, "C")];
        let order = sort_ids_by_depth_desc(&list);
        assert_eq!(order[0], 6); // 深度 1
        // 4 与 5 深度均为 0，稳定排序保持原序
        assert_eq!(&order[1..], &[4, 5]);
    }

    #[test]
    fn batch_delete_cycles_do_not_hang() {
        // 脏数据：7 <-> 8 互为父子
        let list = vec![auth(7, 8, "X"), auth(8, 7, "Y")];
        let order = sort_ids_by_depth_desc(&list);
        assert_eq!(order.len(), 2);
    }

    // ————— 版本下架规则 —————

    /// Java `application.yaml` 全零原生默认（区别于面向当前部署全开的 `Default`）。
    fn java_default() -> EditionFlags {
        EditionFlags {
            is_overseas: 0,
            is_merchant: 0,
            is_pro: 0,
            is_o2o: 0,
            is_vendor: 0,
            is_b2b: 0,
            is_demo: 0,
        }
    }

    #[test]
    fn wholesale_menu_enabled_follows_java_is_wholesale_menu_enabled() {
        // 非演示站 → true（Java `enabled = true`；部署默认也非演示站）
        assert!(EditionFlags::default().wholesale_menu_enabled());
        // 演示站：仅 B2B / 供应商 / 跨境 / O2O 版本展示
        let demo = EditionFlags {
            is_demo: 1,
            ..java_default()
        };
        assert!(!demo.wholesale_menu_enabled());
        let demo_b2b = EditionFlags {
            is_demo: 1,
            is_b2b: 1,
            ..java_default()
        };
        assert!(demo_b2b.wholesale_menu_enabled());
    }

    #[test]
    fn supply_chain_menu_enabled() {
        let f = EditionFlags::default();
        assert!(f.supply_chain_menu_enabled());
        let remove = MenuFilter::for_get_authority(f);
        assert!(!remove("supplyChain"));
        assert!(!remove("purchaseOrderManage"));
        assert!(!remove("supplyChainSetting"));
    }

    #[test]
    fn common_removals_match_java_default_edition() {
        let f = java_default();
        let remove = MenuFilter::for_get_authority(f);
        // is_overseas == 0
        assert!(remove("multilingual"));
        assert!(remove("mobileuserUserOverseasManage"));
        assert!(remove("taxRateManage"));
        // is_merchant == 0
        assert!(remove("shop"));
        assert!(remove("shopManagement"));
        assert!(remove("shopManage"));
        assert!(remove("shopSettingsManage"));
        // is_pro == 0
        assert!(remove("salesman"));
        assert!(remove("eCardManage"));
        assert!(remove("userLevelProManage"));
        // 非 O2O/跨境/供应商/B2B
        assert!(remove("enquiryManage"));
        assert!(remove("quotationManage"));
        // is_b2b == 0（含账期支付整组）
        assert!(remove("userCertificationManage"));
        assert!(remove("realNameAuthentication"));
        assert!(remove("userAuthenticationManage"));
        assert!(remove("termPayManagement"));
        assert!(remove("termOrderManage"));
        assert!(remove("termPayManage"));
        assert!(remove("termMemberManage"));
        // is_vendor == 0（包含匹配）
        assert!(remove("vendorManage"));
        // is_merchant == 0 && is_o2o == 0
        assert!(remove("adminMerchant"));
        assert!(remove("profitsharingManage"));
        assert!(remove("shopUserfundsManagement"));
        // is_merchant == 0 && is_o2o == 0 && is_vendor == 0
        assert!(remove("organize"));
        // 非 O2O：store 族 + 配送族
        assert!(remove("storeManage"));
        assert!(remove("storeEntryRuleSetting"));
        assert!(remove("intraCityDeliveryManage"));
        assert!(remove("StoreDecorate"));
        assert!(remove("staffFundManagement"));
        assert!(remove("deliverymanList"));
        assert!(remove("servicemanList"));
        assert!(remove("homeServiceManage"));
        assert!(remove("cycleBuyManage"));
        // 非 O2O：收银菜单整组（filterReceiptPrintMenuByEdition）
        assert!(remove("groupPos"));
        assert!(remove("posCashier"));
        assert!(remove("cashRegisterManageList"));
        assert!(remove("cashierListIndex"));
        // is_merchant == 0：存款族
        assert!(remove("depositManage"));
        assert!(remove("DepositSetting"));
        // 保留项
        assert!(!remove("dashboard"));
        assert!(!remove("orderManage"));
        assert!(!remove("productManage"));
        assert!(!remove("baseReceiptManage")); // 非 O2O 不移除旧小票菜单
    }

    #[test]
    fn list_variant_matches_get_authority_variant() {
        let f = EditionFlags::default();
        let one = MenuFilter::for_get_authority(f);
        let list = MenuFilter::for_list(f);
        for sn in [
            "depositManage",
            "DepositSetting",
            "cycleBuyManage",
            "StoreDecorate",
            "staffFundManagement",
            "intraCityDeliveryManage",
            "storeManage",
            "vendorManage",
            "supplyChain",
        ] {
            assert_eq!(one(sn), list(sn), "getAuthority 与 list 的下架规则应一致: {sn}");
        }
    }

    #[test]
    fn get_all_authority_variant_differs_from_list_variant() {
        let f = java_default();
        let all = MenuFilter::for_get_all_authority(f);
        let one = MenuFilter::for_get_authority(f);
        let list = MenuFilter::for_list(f);

        // getAllAuthority 移除 intraCityDeliveryManage，但**不**移除
        // StoreDecorate / staffFundManagement
        assert!(all("intraCityDeliveryManage"));
        assert!(!all("StoreDecorate"));
        assert!(!all("staffFundManagement"));
        // getAllAuthority 多移除 cycleBuyManage
        assert!(all("cycleBuyManage"));
        assert!(one("cycleBuyManage"));
        assert!(list("cycleBuyManage"));

        // getAllAuthority **不**套用 deposit 下架（getAuthority / list 才套）
        assert!(!all("depositManage"));
        assert!(one("depositManage"));
        assert!(list("depositManage"));
    }

    #[test]
    fn o2o_edition_keeps_receipt_removes_pos_and_store() {
        let o2o = EditionFlags {
            is_o2o: 1,
            ..java_default()
        };
        let remove = MenuFilter::for_get_authority(o2o);
        // O2O：移除旧版小票菜单
        assert!(remove("baseReceiptManage"));
        assert!(remove("shopBaseReceiptManage"));
        // O2O：保留收银菜单
        assert!(!remove("groupPos"));
        assert!(!remove("posCashier"));
        // O2O：保留 store 族
        assert!(!remove("storeManage"));
        // O2O：deposit 族被移除
        assert!(remove("depositManage"));
    }

    // ————— 建树 —————

    #[test]
    fn build_tree_roots_at_parent_zero_and_nests_children() {
        let rows = vec![
            row(1, "rootA", 0, 1, None),
            row(2, "childA1", 1, 1, Some(r#"[{"authSn":"a"}]"#)),
            row(3, "childA2", 1, 2, Some("[]")),
            row(4, "rootB", 0, 2, None),
            // 孤立节点：父级不在结果集中 → 不出现（Java TreeUtil 同样丢弃）
            row(9, "orphan", 99, 1, None),
        ];
        let tree = build_tree(&rows, TREE_MAX_DEPTH);
        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].id, 1);
        assert_eq!(tree[1].id, 4);
        let children = tree[0].children.as_ref().unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].id, 2);
        assert_eq!(children[1].id, 3);
        assert!(children[0].children.is_none());
        // Hutool 默认键 + putExtra 平铺
        let v = serde_json::to_value(&tree[0]).unwrap();
        assert_eq!(v["id"], 1);
        assert_eq!(v["name"], "菜单1");
        assert_eq!(v["weight"], 1);
        assert_eq!(v["authorityId"], 1);
        assert_eq!(v["authoritySn"], "rootA");
        assert_eq!(v["parentId"], 0);
        assert_eq!(v["isShow"], 1);
        assert_eq!(v["routeLink"], "/route/1");
        assert_eq!(v["childAuth"].as_array().unwrap().len(), 0);
        // 子节点的 childAuth 解析成功
        let cv = serde_json::to_value(&children[0]).unwrap();
        assert_eq!(cv["childAuth"][0]["authSn"], "a");
    }

    #[test]
    fn build_tree_respects_max_depth() {
        // 链 1->2->3->4，deep = 2
        let rows = vec![
            row(1, "a", 0, 1, None),
            row(2, "b", 1, 1, None),
            row(3, "c", 2, 1, None),
            row(4, "d", 3, 1, None),
        ];
        let tree = build_tree(&rows, 2);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].children.as_ref().unwrap()[0].id, 2);
        // 深度 2 已到上限，不再展开
        assert!(tree[0].children.as_ref().unwrap()[0].children.is_none());
    }

    #[test]
    fn build_tree_survives_self_reference() {
        let rows = vec![row(1, "a", 0, 1, None), row(2, "self", 2, 1, None)];
        let tree = build_tree(&rows, TREE_MAX_DEPTH);
        // 2 的父级是它自己 → 不会挂到 0 下，也不会无限递归
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].id, 1);
    }

    // ————— DTO 映射 —————

    #[test]
    fn dto_from_authority_decodes_child_auth() {
        let a = Authority {
            authority_id: 7,
            authority_sn: "orderManage".into(),
            authority_name: "订单管理".into(),
            parent_id: 3,
            sort_order: 5,
            is_show: 1,
            child_auth: Some(r#"[{"authName":"查看","authSn":"view"}]"#.into()),
            route_link: "/order".into(),
            authority_ico: "icon".into(),
            is_system: 1,
            admin_type: "admin".into(),
        };
        let dto = dto_from_authority(&a);
        assert_eq!(dto.authority_id, Some(7));
        assert_eq!(dto.parent_id, Some(3));
        assert_eq!(dto.is_system, Some(1));
        assert_eq!(dto.has_children, None);
        let child = dto.child_auth.unwrap();
        assert_eq!(child[0].auth_sn.as_deref(), Some("view"));

        // child_auth 为 "[]" 时 Java 也会反序列化为空表（isNotBlank 为真）
        let a2 = Authority {
            child_auth: Some("[]".into()),
            ..a
        };
        assert_eq!(dto_from_authority(&a2).child_auth, Some(Vec::new()));
    }

    #[test]
    fn vo_from_selected_keeps_raw_child_auth_text() {
        let r = row(1, "a", 0, 1, Some(r#"[{"authSn":"x"}]"#));
        let vo = vo_from_selected(r);
        assert_eq!(vo.child_auth.as_deref(), Some(r#"[{"authSn":"x"}]"#));
        assert_eq!(vo.is_system, 0);
        assert!(vo.admin_type.is_empty());
    }

    #[test]
    fn page_query_defaults_match_java_base_page() {
        let q = AuthorityListQuery::default();
        assert_eq!(q.page_params(), (1, 15));
        assert_eq!(q.limit_offset(), (15, 0));
    }
}