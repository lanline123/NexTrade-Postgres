//! 代客下单 · 收货地址域服务（trade::proxy_shopping_user_address）。
//!
//! 对应原 Java：
//! - `com.nextrade.adminapi.controller.proxyshopping.ProxyShoppingUserAddressController`
//!   （`@RequestMapping("/adminapi/proxyShopping/address")`）
//! - `com.nextrade.service.user.impl.UserAddressServiceImpl`
//!   （`list` / `create` / `update` / `setSelected`）
//! - `com.nextrade.service.setting.impl.RegionServiceImpl`
//!   （`getRegionPath` / `getAllRegionTree`）
//!
//! | 本服务方法 | 原 Java |
//! |-----------|---------|
//! | [`ProxyShoppingUserAddressService::list`] | `UserAddressServiceImpl#list` |
//! | [`ProxyShoppingUserAddressService::create`] | `UserAddressServiceImpl#create` |
//! | [`ProxyShoppingUserAddressService::update`] | `UserAddressServiceImpl#update` |
//! | [`ProxyShoppingUserAddressService::set_selected`] | `UserAddressServiceImpl#setSelected` |
//! | [`ProxyShoppingUserAddressService::get_region_path`] | `RegionServiceImpl#getRegionPath` |
//! | [`ProxyShoppingUserAddressService::get_all_region_tree`] | `RegionServiceImpl#getAllRegionTree` |
//!
//! # 与 Java 的差异（逐条，均为「原样照抄」或「明确未迁移」）
//!
//! 1. **`getCurrentUserId()` 未迁移 —— 代客下单的会员ID必须由前端显式给出**。
//!    Java 在 `list` / `create` / `update` / `setSelected` 里用
//!    `SecurityUtils.getCurrentUserId()` 兜底。管理端 JWT 里**没有会员身份**
//!    （Rust `nextrade_security::web::AuthUser.id` 是 `admin_id`），照抄会把
//!    管理员自己的 `admin_id` 当成会员 `user_id` 去读写收货地址 —— 那是
//!    静默的数据污染。故本实现：
//!    - `list` / `create`：`userId` 缺失或 <= 0 → 业务错误
//!      **「请选择用户」**（code 1001，与 Java `ProxyShoppingCartController`
//!      / `ProxyShoppingOrderCheckController#validateUserIdNotEmpty` 同一文案）
//!    - `update` / `setSelected`：目标记录的 `user_id` 一律取自**库中该地址行**，
//!      不取任何「当前用户」，因此不存在拿错归属的可能
//!    详见 `uncertain_points`。
//! 2. **`is_default` 清零的作用域修正**。Java `create` / `update` 用
//!    `getCurrentUserId()`（= 管理员ID）当作用户ID去执行
//!    `UPDATE user_address SET is_default = 0 WHERE user_id = <管理员ID>`；
//!    在代客下单场景下这**既没有清掉目标会员的默认地址，又污染了别的用户**。
//!    本实现改用「本次操作的**目标会员** user_id」（create）与
//!    「**该地址行的** user_id」（update）——与 `list`/`setSelected` 之外
//!    所有链路的意图一致。这是有意的行为修正，见 `uncertain_points`。
//! 3. **`regionNames` 的顺序**。Java 用 `regionService.list(in(regionId, ids))`
//!    再 `map(getRegionName)`，未写 ORDER BY（MySQL 实际按主键升序）。
//!    本实现显式 `ORDER BY region_id ASC`，固定该事实行为 —— ⚠️ 对国内数据
//!    （`1 中国` → `330000 浙江省` → `330100 杭州市`）与前端选择的
//!    `[1, 330000, 330100]` 顺序一致；但对跨境数据（`region_id` 与行政区划
//!    层级不同序）可能与 Java 的**偶然顺序**不同，属既有不确定点。
//! 4. **`is_overseas` 取值来源**。Java 取自 `application.yaml`
//!    （全功能部署 `nextrade.is-overseas: 1`）。Rust 侧无配置服务，按
//!    `nextrade_services::identity::authority::EditionFlags` 的既有约定取 `0`
//!    （国内版 → `level > 0`）。
//! 5. **i18n 未迁移**。邮编超长文案直接内联中文
//!    （`messages.properties: error.address.postcode.too.long`），不接
//!    `MessageHelper`。多语言站点需另行接入。
//! 6. **权限未迁移**。Java 类上有
//!    `@PreAuthorize("@pms.hasPermission('proxyShoppingManage')")`；
//!    Rust 侧权限矩阵尚未实现，`require_auth` 只校验登录态
//!    （与 seckill / order_admin / brand / shop_manage / pc_navigation 的已知缺口一致）。

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use nextrade_core::entities::{Region, UserAddress};
use nextrade_core::models::trade::proxy_shopping_user_address::{
    AddressCreateRequest, AddressListRequest, AddressUpdatePatch, AddressUpdateRequest, AddressVO,
    RegionPathNode, RegionTreeNode,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::trade::proxy_shopping_user_address::{
    resolve_sort_column, ProxyShoppingUserAddressRepository, CONFIG_DEFAULT_COUNTRY,
};
use nextrade_infrastructure::db::Repositories;

use crate::identity::authority::EditionFlags;

/// Java `HttpStatus.SERVICE_DATA_ERROR`（`GlobalException(String)` 的默认 code）。
pub const SERVICE_DATA_ERROR: i32 = 1001;

/// Java `UserAddressConstants.USER_ADDRESS_NOT_EXIST`（`UserAddressServiceImpl#detail` 用）。
pub const USER_ADDRESS_NOT_EXIST: &str = "收货地址不存在";

/// 控制器 `update` 找不到地址时的文案（`ProxyShoppingUserAddressController.java:58`）。
pub const ADDRESS_NOT_EXIST: &str = "地址不存在";

/// 代客下单必须显式指定会员（对应 Java
/// `ProxyShoppingCartController#validateUserIdNotEmpty` 的「请选择用户」）。
pub const SELECT_USER_REQUIRED: &str = "请选择用户";

/// `user_address.postcode` 列长度（`varchar(60)`）。
pub const POSTCODE_MAX_LENGTH: usize = 60;

/// 邮编超长文案（Java `messages.properties: error.address.postcode.too.long`）。
pub const POSTCODE_TOO_LONG: &str = "邮政编码长度不能超过60个字符";

/// 列表默认每页大小（Java `BasePage.getSize()` 缺省 15）。
pub const DEFAULT_PAGE_SIZE: i64 = 15;
/// 列表每页大小上限（Java `BasePage.getSize()` 的 `> 100 → 100`）。
pub const MAX_PAGE_SIZE: i64 = 100;
/// `getAllRegionTree` 的树深上限（Java `TreeNodeConfig.setDeep(8)`）。
pub const REGION_TREE_MAX_DEEP: usize = 8;

/// 地区树默认按国内版过滤（`level > 0`）。
///
/// Java 取 `NexTradeProperties.getIsOverseas()`（全功能部署 `is-overseas: 1`）；
/// Rust 侧复用 identity::authority 既有约定，不在本切片新建配置服务。
#[must_use]
pub fn default_is_overseas() -> bool {
    EditionFlags::default().is_overseas == 1
}

// ————————————————————————— 纯逻辑（可直接单测） —————————————————————————

/// 分页参数归一化（Java `BasePage#getPage` / `#getSize`）。
///
/// - `page` 缺省 1，且至少 1
/// - `size` 缺省 15，上限 100，且至少 1
///
/// 返回 `(limit, offset)`，直接喂给 SQL 的 `LIMIT ? OFFSET ?`。
#[must_use]
pub fn resolve_pagination(page: Option<i64>, size: Option<i64>) -> (u64, u64) {
    let size = size.unwrap_or(DEFAULT_PAGE_SIZE).clamp(1, MAX_PAGE_SIZE);
    let page = page.unwrap_or(1).max(1);
    let limit = u64::try_from(size).unwrap_or(DEFAULT_PAGE_SIZE as u64);
    let offset = u64::try_from((page - 1).saturating_mul(size)).unwrap_or(0);
    (limit, offset)
}

/// 代客下单的会员ID解析：必须在请求体里显式给出（见模块头第 1 条）。
///
/// `user_id.map_or(false, |id| id > 0)` 与 Java
/// `ProxyShopping*Controller#validateUserIdNotEmpty` 的 `id == null || id <= 0`
/// 判定完全一致（`0` 与负数都视为「未选择用户」）。
pub fn require_user_id(user_id: Option<i32>) -> Result<i32> {
    match user_id {
        Some(id) if id > 0 => Ok(id),
        _ => Err(Error::biz(SERVICE_DATA_ERROR, SELECT_USER_REQUIRED)),
    }
}

/// 邮编规范化（Java `UserAddressServiceImpl#normalizePostcode`）。
///
/// - `None` → `None`（沿用 `updateById` 忽略 null 的兼容语义）
/// - 非 null → trim；超过 60 字符抛业务错误；否则返回 trim 结果
pub fn normalize_postcode(postcode: Option<&str>) -> Result<Option<String>> {
    match postcode {
        None => Ok(None),
        Some(p) => {
            let trimmed = p.trim();
            if trimmed.chars().count() > POSTCODE_MAX_LENGTH {
                return Err(Error::biz(SERVICE_DATA_ERROR, POSTCODE_TOO_LONG));
            }
            Ok(Some(trimmed.to_string()))
        }
    }
}

/// `region_ids` → `[JSON]`（Java `JsonUtil.toJson(list)`）。
///
/// Java 用 Jackson 默认输出（无空格分隔），这里用 `serde_json` 同样紧凑输出。
#[must_use]
pub fn encode_region_ids(region_ids: &[i32]) -> String {
    serde_json::to_string(region_ids).unwrap_or_else(|_| "[]".to_string())
}

/// `[JSON]` → `Vec<String>`（Java `JsonUtil.jsonToList(text, String.class)`）。
///
/// Java 侧解析失败会抛异常/返回 null；本实现宽容处理：
/// 空串、非法 JSON、元素非字符串 → 该元素跳过。列表接口因此不会因历史脏数据 500。
#[must_use]
pub fn decode_string_list(text: &str) -> Vec<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<Value>(trimmed) {
        Ok(Value::Array(items)) => items
            .into_iter()
            .filter_map(|v| match v {
                Value::String(s) => Some(s),
                Value::Number(n) => Some(n.to_string()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// `[JSON]` → `Vec<i32>`（Java `JsonUtil.jsonToList(text, Integer.class)`）。
#[must_use]
pub fn decode_i32_list(text: &str) -> Vec<i32> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<Value>(trimmed) {
        Ok(Value::Array(items)) => items
            .into_iter()
            .filter_map(|v| match v {
                Value::Number(n) => n.as_i64().and_then(|i| i32::try_from(i).ok()),
                Value::String(s) => s.trim().parse::<i32>().ok(),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// 地区名去重 + 拼接（Java `UserAddressServiceImpl#list` 的
/// `regionNames.size() > 1 && regionNames.get(0).equals(regionNames.get(1))`
/// → `remove(1)`，再 `String.join(" ")`）。
///
/// ⚠️ 逐字符复刻 Java 的行为：**只删下标 1 的那一个重复项**，不是全量去重。
/// 例如 `["北京市","北京市","北京市"]` → `["北京市","北京市"]` → `"北京市 北京市"`
/// （Java 也是如此）。直辖市数据正是这种形态，务必保持一致。
#[must_use]
pub fn build_region_name(region_names: &[String]) -> String {
    let mut names: Vec<&str> = region_names.iter().map(String::as_str).collect();
    if names.len() > 1 && names[0] == names[1] {
        names.remove(1);
    }
    names.join(" ")
}

/// 外部地区ID列表 → 地区名称列表（Java `regionService.list(in(...))` + `map`）。
///
/// `regions` 由仓储按 `region_id ASC` 返回；本函数只负责投影与「查不到就丢弃」
/// （Java 同样只映射查到的行）。
#[must_use]
pub fn resolve_region_names(regions: &[Region]) -> Vec<String> {
    regions.iter().map(|r| r.region_name.clone()).collect()
}

/// 单条地址行 → 出参（Java `UserAddressServiceImpl#list` 的 stream map）。
#[must_use]
pub fn address_to_vo(a: &UserAddress) -> AddressVO {
    let region_names = decode_string_list(&a.region_names);
    let region_name = build_region_name(&region_names);
    AddressVO {
        address: a.address.clone(),
        position_address: a.position_address.clone(),
        address_id: a.address_id,
        consignee: a.consignee.clone(),
        email: a.email.clone(),
        is_default: a.is_default,
        is_selected: a.is_selected,
        mobile: a.mobile.clone(),
        mobile_area_code: a.mobile_area_code.clone(),
        region_name,
        region_names,
        telephone: a.telephone.clone(),
        user_id: a.user_id,
        region_ids: decode_i32_list(&a.region_ids),
        postcode: a.postcode.clone(),
        address_tag: a.address_tag.clone(),
        latitude: a.latitude,
        longitude: a.longitude,
        is_system: a.is_system,
    }
}

/// 解析「默认国家」配置值（Java `Integer.valueOf(defaultCountryCode.getBizVal())`）。
///
/// Java 在配置缺失时抛 `GlobalException(CONFIG_NOT_EXIST)`；本实现返回 `None`
/// 并让 `get_region_path` 走「无默认国家」的完整向上遍历分支
/// （对国内数据结果相同，因为中国 `region_id = 1` 本来就是根）。
/// 配置存在但不是数字时同样按 `None` 处理（Java 会抛 `NumberFormatException` → 500）。
#[must_use]
pub fn parse_default_country(raw: Option<&str>) -> Option<i32> {
    raw.map(str::trim).filter(|s| !s.is_empty()).and_then(|s| s.parse::<i32>().ok())
}

/// 向上回溯的地区路径（Java `RegionServiceImpl#getRegionPath` 的 while 循环）。
///
/// `lookup` 返回 `Some(region)` 或 `None`（Java `getById` 未命中 → `break`）。
///
/// 与 Java 的分支差异：Java 在 `defaultCountryId == null` 时才多走一遍
/// 「继续向上找根节点」的循环；那是针对 `list.isEmpty()` 的防御性写法 ——
/// 只要第一次取到过节点，第一个循环就已经走到 `parent_id == 0` 或命中默认国家，
/// 第二个循环进入时 `currentId` 必为 `0`/`None`，**恒不执行**。
/// 故本实现只保留一个循环，行为与 Java 等价。
#[must_use]
pub fn collect_region_path<F>(region_id: i32, default_country_id: Option<i32>, lookup: F) -> Vec<Region>
where
    F: Fn(i32) -> Option<Region>,
{
    let mut path: Vec<Region> = Vec::new();
    let mut current: Option<i32> = Some(region_id);
    while let Some(id) = current {
        if id == 0 {
            break;
        }
        let Some(region) = lookup(id) else {
            break;
        };
        let parent = region.parent_id;
        path.push(region);
        // 命中默认国家即停止（中国 `region_id = 1`，避免把「世界」之类的上层也带出来）
        if default_country_id == Some(id) {
            break;
        }
        current = Some(parent);
    }
    // Java `Collections.reverse(path)`：由根到叶
    path.reverse();
    path
}

/// 由「扁平地区列表」构造树（Java `TreeUtil.build(list, 0, config, ..)`）。
///
/// `order` 决定同级节点的顺序（Java 用 `idTreeMap` 的插入序，即 `list` 的原始顺序；
/// 仓储已按 `region_id ASC` 返回，故此处直接沿用入参顺序）。
///
/// - 根：`parent_id == 0` 的节点成为顶层
/// - 父节点不在结果集中的节点会被**丢弃**（与 Hutool `buildFromMap` 一致：
///   `parentNode == null` 时既不挂父也不进 root）
/// - 深度上限 [`REGION_TREE_MAX_DEEP`]，与 `TreeNodeConfig.setDeep(8)` 一致
#[must_use]
pub fn build_region_tree(regions: Vec<Region>) -> Vec<RegionTreeNode> {
    if regions.is_empty() {
        return Vec::new();
    }

    // 1) 建节点（保留顺序），并按 region_id 索引
    let mut index: HashMap<i32, usize> = HashMap::new();
    let mut nodes: Vec<RegionTreeNode> = Vec::with_capacity(regions.len());
    for r in &regions {
        index.insert(r.region_id, nodes.len());
        nodes.push(RegionTreeNode::leaf(r.region_id, r.parent_id, r.region_name.clone()));
    }

    // 2) 记录父子关系（先收集边，避免同时借用 nodes 的两个位置）
    let mut parent_of: HashMap<usize, usize> = HashMap::new(); // child_idx -> parent_idx
    let mut top: Vec<usize> = Vec::new();
    for (idx, r) in regions.iter().enumerate() {
        if r.parent_id == 0 {
            top.push(idx);
            continue;
        }
        if let Some(&p) = index.get(&r.parent_id) {
            parent_of.insert(idx, p);
        }
    }

    // 3) 自**最深**节点向浅处逐级挂载。
    //    关键：必须按「子节点先就位」的顺序组装，否则把子节点挂到父节点的
    //    `children` 之后，父节点再被挂到祖父时用的是**尚未包含该子树的旧副本**，
    //    孙层会整体丢失（表现为树只有 2 层深）。故这里按深度降序处理。
    let mut by_depth: Vec<usize> = (0..regions.len()).collect();
    by_depth.sort_by_key(|&i| std::cmp::Reverse(depth_of(i, &parent_of)));

    // `pending[child]` 暂存「已经组装完毕、等待挂到父节点的子树」
    let mut pending: Vec<Option<RegionTreeNode>> = Vec::with_capacity(regions.len());
    for n in &nodes {
        pending.push(Some(n.clone()));
    }
    for idx in by_depth {
        let Some(&parent) = parent_of.get(&idx) else {
            continue; // 根节点或无父（孤儿）留待顶层 / 丢弃
        };
        let Some(node) = pending.get_mut(idx).and_then(Option::take) else {
            continue;
        };
        if let Some(slot) = pending.get_mut(parent).and_then(Option::as_mut) {
            slot.children.push(node);
        }
    }
    // 稳定同级顺序：children 按原始入参顺序排列（Java 沿用 `list` 的插入序）
    for node in pending.iter_mut().flatten() {
        sort_children_by_original_order(node, &index);
    }

    // 4) 顶层 + 剪枝到 8 层
    let mut roots: Vec<RegionTreeNode> = Vec::with_capacity(top.len());
    for idx in top {
        if let Some(node) = pending.get_mut(idx).and_then(Option::take) {
            roots.push(node);
        }
    }
    for root in &mut roots {
        truncate_depth(root, 1);
    }
    roots
}

/// 从 `idx` 沿 `parent_of` 向上走到根，返回层号（根 = 0）。
fn depth_of(idx: usize, parent_of: &HashMap<usize, usize>) -> usize {
    let mut depth = 0;
    let mut cur = idx;
    while let Some(&p) = parent_of.get(&cur) {
        depth += 1;
        cur = p;
        if depth > 1024 {
            break; // 防御环形数据
        }
    }
    depth
}

/// 递归按「原始入参顺序」稳定排序每一层 `children`（顺序对齐 Java 的插入序）。
fn sort_children_by_original_order(node: &mut RegionTreeNode, order: &HashMap<i32, usize>) {
    node.children
        .sort_by_key(|c| order.get(&c.region_id).copied().unwrap_or(usize::MAX));
    for child in &mut node.children {
        sort_children_by_original_order(child, order);
    }
}

/// 把子树裁剪到 `REGION_TREE_MAX_DEEP` 层（Java `TreeBuilder#cutTree`）。
///
/// `depth` 是当前节点的层号（根为 1）；到达上限即清空其 `children`。
fn truncate_depth(node: &mut RegionTreeNode, depth: usize) {
    if depth >= REGION_TREE_MAX_DEEP {
        node.children.clear();
        return;
    }
    for child in &mut node.children {
        truncate_depth(child, depth + 1);
    }
}

// ————————————————————————— 服务 —————————————————————————

/// 代客下单收货地址服务。
pub struct ProxyShoppingUserAddressService {
    repos: Arc<Repositories>,
}

impl ProxyShoppingUserAddressService {
    /// 构造：持有全局仓储集合（与 product_manage/002/…/026 同一约定）。
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 直达本域仓储（避免与其他切片的 `repo` 字段重名歧义）。
    fn repo(&self) -> &ProxyShoppingUserAddressRepository {
        &self.repos.trade.proxy_shopping_user_address
    }

    /// 收货地址列表。
    ///
    /// 对应 Java `ProxyShoppingUserAddressController.list(@RequestBody UserAddressListDTO)`
    /// → `UserAddressServiceImpl#list`。
    ///
    /// 控制器强制两件事（本实现照抄）：
    /// 1. `sortField = "is_selected"`；`sortOrder` 未设置 → Java `buildSortOrder`
    ///    按 **desc** 处理
    /// 2. `isSystem = 2` → 服务层既不加 `is_system = 1` 也不加 `= 0`，**查全部**
    ///
    /// `userId` 必填（见模块头第 1 条）。
    pub async fn list(&self, req: &AddressListRequest) -> Result<PageResult<AddressVO>> {
        let user_id = require_user_id(req.user_id)?;
        let (limit, offset) = resolve_pagination(req.page, req.size);
        // 排序字段与用户输入无关：Java 控制器恒覆盖为 is_selected；
        // 万一日后放开，也只走白名单（未命中回落主键）。
        let column = resolve_sort_column(req.sort_field.as_deref().unwrap_or("is_selected"));
        let desc = !matches!(req.sort_order.as_deref(), Some(o) if o.eq_ignore_ascii_case("asc"));

        let (total, rows) = self
            .repo()
            .page_user_addresses(user_id, req.is_system, column, desc, limit, offset)
            .await?;
        let list: Vec<AddressVO> = rows.iter().map(address_to_vo).collect();
        let total = u64::try_from(total).unwrap_or(0);
        Ok(PageResult {
            total,
            total_page: total.div_ceil(limit.max(1)),
            list,
            page_size: limit,
            page: offset / limit.max(1) + 1,
        })
    }

    /// 新增收货地址。
    ///
    /// 对应 Java `ProxyShoppingUserAddressController.create(@RequestBody UserAddressCreateDTO)`
    /// （`createDTO.setIsSystem(1)`）→ `UserAddressServiceImpl#create`。
    ///
    /// 复刻顺序：
    /// 1. `region_ids` 非空 → 反查地区名并写入 `region_names` / `region_ids`
    /// 2. `postcode` trim + 长度校验
    /// 3. `is_default == 1` → 先把**目标会员**的其它地址清 0（模块头第 2 条）
    /// 4. INSERT
    /// 5. `is_selected == 1` → 把该地址置为选中
    ///
    /// 返回自增 `address_id`（Java 同样返回主键）。
    ///
    /// ⚠️ Java 在 `region_ids` 为空时会把 `region_names` / `region_ids` 留成
    /// `null`，而表定义是 `NOT NULL`（无默认值）→ Java 侧该分支必然 SQL 报错。
    /// 本实现按「客户端未选地区」处理为 `[]`，避免 500（前端表单本就校验必填）。
    pub async fn create(&self, dto: &AddressCreateRequest) -> Result<i32> {
        let user_id = require_user_id(dto.user_id)?;
        let region_ids = dto.region_ids.clone().unwrap_or_default();

        let region_names_json = if region_ids.is_empty() {
            "[]".to_string()
        } else {
            let regions = self.repo().find_regions_by_ids(&region_ids).await?;
            serde_json::to_string(&resolve_region_names(&regions))
                .unwrap_or_else(|_| "[]".to_string())
        };

        let is_default = dto.is_default.unwrap_or(0);
        let is_selected = dto.is_selected.unwrap_or(0);

        if is_default == 1 {
            // Java 在这里用的是 getCurrentUserId()（见模块头第 2 条）。
            self.repo().clear_default_flags(user_id, None).await?;
        }

        let postcode = normalize_postcode(dto.postcode.as_deref())?;
        let address_id = self
            .repo()
            .insert_user_address(
                user_id,
                dto.address_tag.as_deref().unwrap_or(""),
                dto.consignee.as_deref().unwrap_or(""),
                dto.email.as_deref(),
                &encode_region_ids(&region_ids),
                &region_names_json,
                dto.address.as_deref().unwrap_or(""),
                dto.position_address.as_deref(),
                postcode.as_deref(),
                dto.telephone.as_deref(),
                dto.mobile.as_deref().unwrap_or(""),
                dto.mobile_area_code.as_deref(),
                is_default,
                // Java 控制器强制：createDTO.setIsSystem(1)
                Some(1),
                is_selected,
                dto.latitude,
                dto.longitude,
            )
            .await?;

        if is_selected == 1 {
            self.repo().mark_selected(address_id, user_id).await?;
        }
        Ok(address_id)
    }

    /// 修改收货地址。
    ///
    /// 对应 Java `ProxyShoppingUserAddressController.update(@RequestBody UserAddressUpdateDTO)`
    /// → `UserAddressServiceImpl#update`。
    ///
    /// 控制器先查原记录，再**强制覆盖**两个字段：
    /// `isSystem = po.getIsSystem()`、`userId = po.getUserId()`，
    /// 因此客户端无法通过 update 改归属或系统标记。本实现同样只用
    /// `address_id` 定位、并把 `user_id` / `is_system` 留在库里不动。
    ///
    /// - `addressId` 缺失或查不到 → 「地址不存在」（code 1001）
    /// - `is_default > 0` → 先把**该地址所属会员**的其它地址清 0
    /// - 只有 DTO 里**非 null** 的字段参与 UPDATE（`updateById` 语义）
    /// - 一个可更新字段都没有 → 业务错误（Java 侧会因空 SET 抛异常）
    pub async fn update(&self, dto: &AddressUpdateRequest) -> Result<()> {
        let address_id = dto.address_id.unwrap_or(0);
        let current = self.repo().find_user_address(address_id).await?;
        let Some(current) = current else {
            return Err(Error::biz(SERVICE_DATA_ERROR, ADDRESS_NOT_EXIST));
        };

        // 归属：以库中记录为准（Java `userAddress.setUserId(po.getUserId())`）
        let user_id = current.user_id;

        let mut patch = AddressUpdatePatch {
            address_tag: dto.address_tag.clone().flatten(),
            consignee: dto.consignee.clone().flatten(),
            email: dto.email.clone().flatten(),
            address: dto.address.clone().flatten(),
            position_address: dto.position_address.clone().flatten(),
            telephone: dto.telephone.clone().flatten(),
            mobile: dto.mobile.clone().flatten(),
            mobile_area_code: dto.mobile_area_code.clone().flatten(),
            is_default: dto.is_default.flatten(),
            is_selected: dto.is_selected.flatten(),
            latitude: dto.latitude.flatten(),
            longitude: dto.longitude.flatten(),
            ..Default::default()
        };

        // 邮编：Java 无论 DTO 是否给了值都会调用 normalizePostcode 并 set 进实体，
        // 但 updateById 会跳过 null —— 故仅当显式给出时才参与 UPDATE。
        if let Some(raw) = dto.postcode.clone().flatten() {
            patch.postcode = normalize_postcode(Some(&raw))?;
        }

        // 地区：仅当 DTO 给了非空 region_ids 时才重算 region_names
        // （Java `if (regionIds != null && !regionIds.isEmpty())`）
        if let Some(ids) = dto.region_ids.clone().flatten() {
            if !ids.is_empty() {
                let regions = self.repo().find_regions_by_ids(&ids).await?;
                let names = resolve_region_names(&regions);
                patch.region_ids = Some(encode_region_ids(&ids));
                patch.region_names =
                    Some(serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_string()));
            }
        }

        if patch.is_default.is_some_and(|v| v > 0) {
            self.repo()
                .clear_default_flags(user_id, Some(address_id))
                .await?;
        }

        if !patch.has_any() {
            return Err(Error::biz(SERVICE_DATA_ERROR, "没有需要更新的字段"));
        }
        self.repo().update_user_address(address_id, &patch).await?;
        Ok(())
    }

    /// 设置选中状态。
    ///
    /// 对应 Java `ProxyShoppingUserAddressController.setSelected(@RequestBody UserAddressUpdateDTO)`
    /// → `UserAddressServiceImpl#setSelected(userAddress.getId())`。
    ///
    /// Java 用 `getCurrentUserId()` 做归属条件；本实现改用**该地址行的
    /// `user_id`**（模块头第 1 条），语义上仍是「只能选中自己的地址」。
    ///
    /// - `id` 缺失或查不到 → 「地址不存在」
    /// - 先清空该会员所有地址的 `is_selected`，再把目标置 1（两步 UPDATE，同 Java）
    pub async fn set_selected(&self, dto: &AddressUpdateRequest) -> Result<()> {
        let id = dto.id.unwrap_or(0);
        let current = self.repo().find_user_address(id).await?;
        let Some(current) = current else {
            return Err(Error::biz(SERVICE_DATA_ERROR, ADDRESS_NOT_EXIST));
        };
        self.repo().clear_selected_flags(current.user_id).await?;
        self.repo().mark_selected(id, current.user_id).await?;
        Ok(())
    }

    /// 根据 regionId 获取完整地区路径（由根到叶）。
    ///
    /// 对应 Java `ProxyShoppingUserAddressController.getRegionPath`
    /// → `RegionServiceImpl#getRegionPath`。
    ///
    /// - `regionId` 缺失 → 「参数错误」（`ExceptionConstants.PARAM_ERROR` 文案）
    /// - 默认国家配置：`config WHERE biz_code = 'defaultCountry' AND is_del = 0`
    /// - 路径到「默认国家」为止（种子数据里是「中国」`region_id = 1`）
    pub async fn get_region_path(&self, region_id: Option<i32>) -> Result<Vec<RegionPathNode>> {
        let Some(region_id) = region_id else {
            return Err(Error::biz(SERVICE_DATA_ERROR, PARAM_ERROR));
        };
        let default_country = self
            .repo()
            .find_config_val(CONFIG_DEFAULT_COUNTRY)
            .await
            .ok()
            .flatten();
        let default_country_id = parse_default_country(default_country.as_deref());

        // 一次性把需要的行取出来（避免闭包里做 async 查询）：
        // 最多 8 层，逐层查与 Java 的 `getById` 循环等价。
        let mut path: Vec<Region> = Vec::new();
        let mut current = Some(region_id);
        while let Some(id) = current {
            if id == 0 {
                break;
            }
            let Some(region) = self.repo().find_region(id).await? else {
                break;
            };
            let parent = region.parent_id;
            path.push(region);
            if default_country_id == Some(id) {
                break;
            }
            current = Some(parent);
        }
        path.reverse();
        Ok(path.iter().map(region_to_path_node).collect())
    }

    /// 获取所有地区（树形结构）。
    ///
    /// 对应 Java `ProxyShoppingUserAddressController.getAllRegionTree`
    /// → `RegionServiceImpl#getAllRegionTree`。
    ///
    /// `is_overseas == 1` → `level >= 0`；否则 → `level > 0`（国内版）。
    pub async fn get_all_region_tree(&self) -> Result<Vec<RegionTreeNode>> {
        let regions = self
            .repo()
            .find_regions_for_tree(default_is_overseas())
            .await?;
        Ok(build_region_tree(regions))
    }
}

/// Java `ExceptionConstants.PARAM_ERROR` 的中文文案。
pub const PARAM_ERROR: &str = "参数错误";

/// 地区实体 → 路径节点（Java 直接把 `Region` PO 序列化，8 列全出）。
#[must_use]
pub fn region_to_path_node(r: &Region) -> RegionPathNode {
    RegionPathNode {
        region_id: r.region_id,
        region_name: r.region_name.clone(),
        parent_id: r.parent_id,
        level: r.level,
        is_hot: r.is_hot,
        sort_order: r.sort_order,
        first_word: r.first_word.clone(),
        standard_code: r.standard_code.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(id: i32, parent: i32, name: &str) -> Region {
        Region {
            region_id: id,
            level: Some(2),
            region_name: name.to_string(),
            parent_id: parent,
            is_hot: 0,
            sort_order: 50,
            first_word: String::new(),
            standard_code: None,
        }
    }

    // ————————————————————— 分页 —————————————————————

    #[test]
    fn pagination_matches_java_base_page_defaults() {
        // 缺省：page=1, size=15
        assert_eq!(resolve_pagination(None, None), (15, 0));
        // 上限 100
        assert_eq!(resolve_pagination(Some(1), Some(1000)), (100, 0));
        // 下限 1（Java 只写了上界，这里补下界避免 LIMIT 0 恒空 / 负数 OFFSET）
        assert_eq!(resolve_pagination(Some(0), Some(0)), (1, 0));
        assert_eq!(resolve_pagination(Some(-3), Some(-1)), (1, 0));
        // 第 3 页 / 每页 20
        assert_eq!(resolve_pagination(Some(3), Some(20)), (20, 40));
        // 超大页码不得 panic（本实现 clamp 到 u64 上限）
        let (limit, offset) = resolve_pagination(Some(i64::MAX), Some(100));
        assert_eq!(limit, 100);
        assert!(offset > 0);
    }

    // ————————————————————— 会员ID —————————————————————

    #[test]
    fn user_id_is_required_for_proxy_shopping() {
        assert_eq!(require_user_id(Some(9)).unwrap(), 9);
        for bad in [None, Some(0), Some(-1)] {
            match require_user_id(bad) {
                Err(Error::Biz { code, msg }) => {
                    assert_eq!(code, SERVICE_DATA_ERROR);
                    assert_eq!(msg, SELECT_USER_REQUIRED);
                }
                other => panic!("{bad:?} 应被拒绝，实际 {other:?}"),
            }
        }
    }

    // ————————————————————— 邮编 —————————————————————

    #[test]
    fn postcode_is_trimmed_and_length_checked() {
        assert_eq!(normalize_postcode(None).unwrap(), None);
        assert_eq!(
            normalize_postcode(Some("  310000  ")).unwrap(),
            Some("310000".to_string())
        );
        // 空串 trim 后仍是空串（Java 同样不拦截）
        assert_eq!(normalize_postcode(Some("   ")).unwrap(), Some(String::new()));
        // 60 字符合法
        let ok = "1".repeat(POSTCODE_MAX_LENGTH);
        assert_eq!(normalize_postcode(Some(&ok)).unwrap().unwrap().len(), 60);
        // 61 字符报错（文案与 code 对齐 Java）
        let too_long = "1".repeat(POSTCODE_MAX_LENGTH + 1);
        match normalize_postcode(Some(&too_long)) {
            Err(Error::Biz { code, msg }) => {
                assert_eq!(code, SERVICE_DATA_ERROR);
                assert_eq!(msg, POSTCODE_TOO_LONG);
            }
            other => panic!("超长邮编应被拒绝，实际 {other:?}"),
        }
    }

    // ————————————————————— JSON 列表 —————————————————————

    #[test]
    fn region_ids_json_round_trip() {
        let ids = vec![1, 330000, 330100, 330106];
        let json = encode_region_ids(&ids);
        assert_eq!(json, "[1,330000,330100,330106]");
        assert_eq!(decode_i32_list(&json), ids);
        assert_eq!(decode_i32_list("[]"), Vec::<i32>::new());
        assert_eq!(encode_region_ids(&[]), "[]");
    }

    #[test]
    fn string_list_decode_is_lenient_on_dirty_data() {
        assert_eq!(
            decode_string_list(r#"["浙江省","杭州市","西湖区"]"#),
            vec!["浙江省", "杭州市", "西湖区"]
        );
        // 历史脏数据不得让列表接口 500
        assert!(decode_string_list("").is_empty());
        assert!(decode_string_list("not json").is_empty());
        assert!(decode_string_list("null").is_empty());
        assert!(decode_string_list(r#"{"a":1}"#).is_empty());
        // 非字符串元素跳过，字符串数字保留
        assert_eq!(decode_string_list(r#"["浙江省",1,null]"#), vec!["浙江省", "1"]);
        // 数字型 region_ids 也容忍字符串形态
        assert_eq!(decode_i32_list(r#"["1","2"]"#), vec![1, 2]);
        assert!(decode_i32_list(r#"["x",null]"#).is_empty());
    }

    // ————————————————————— 地区名拼接 —————————————————————

    #[test]
    fn region_name_joins_and_drops_single_duplicate_like_java() {
        assert_eq!(build_region_name(&[]), "");
        assert_eq!(build_region_name(&["浙江省".into()]), "浙江省");
        // 直辖市：Java 会删掉下标 1 的重复项 → 「北京市 东城区」
        assert_eq!(
            build_region_name(&["北京市".into(), "北京市".into(), "东城区".into()]),
            "北京市 东城区"
        );
        // 三个相同：Java 只删一个 → 「北京市 北京市」（逐字符复刻，不做全量去重）
        assert_eq!(
            build_region_name(&["北京市".into(), "北京市".into(), "北京市".into()]),
            "北京市 北京市"
        );
        // 首尾相同但不含下标 1：不动
        assert_eq!(
            build_region_name(&["浙江省".into(), "杭州市".into(), "浙江省".into()]),
            "浙江省 杭州市 浙江省"
        );
        assert_eq!(
            build_region_name(&["浙江省".into(), "杭州市".into(), "西湖区".into()]),
            "浙江省 杭州市 西湖区"
        );
    }

    #[test]
    fn resolve_region_names_projects_and_drops_missing() {
        let regions = vec![region(1, 0, "中国"), region(330000, 1, "浙江省")];
        assert_eq!(resolve_region_names(&regions), vec!["中国", "浙江省"]);
        // 查不到的行不会出现在入参里（仓储只返回命中的行）
        assert!(resolve_region_names(&[]).is_empty());
    }

    // ————————————————————— 出参映射 —————————————————————

    #[test]
    fn address_to_vo_decodes_json_columns() {
        let a = UserAddress {
            address_id: 7,
            address_tag: "家".into(),
            user_id: 100,
            consignee: "张三".into(),
            email: Some("a@b.c".into()),
            region_ids: "[1,330000]".into(),
            region_names: r#"["浙江省","杭州市"]"#.into(),
            address: "文一西路 1 号".into(),
            position_address: Some("某大厦".into()),
            postcode: Some("310000".into()),
            telephone: None,
            mobile: "13800000000".into(),
            mobile_area_code: Some("86".into()),
            is_default: 1,
            is_selected: 0,
            is_system: Some(1),
            latitude: Some("30.1".parse().unwrap()),
            longitude: None,
            ..Default::default()
        };
        let vo = address_to_vo(&a);
        assert_eq!(vo.address_id, 7);
        assert_eq!(vo.user_id, 100);
        assert_eq!(vo.region_ids, vec![1, 330000]);
        assert_eq!(vo.region_names, vec!["浙江省", "杭州市"]);
        assert_eq!(vo.region_name, "浙江省 杭州市");
        assert_eq!(vo.is_system, Some(1));
        assert_eq!(vo.latitude, Some("30.1".parse().unwrap()));
        assert_eq!(vo.telephone, None);

        // 空/脏 JSON 不 panic
        let dirty = UserAddress {
            region_ids: String::new(),
            region_names: "oops".into(),
            ..Default::default()
        };
        let vo = address_to_vo(&dirty);
        assert!(vo.region_ids.is_empty());
        assert!(vo.region_names.is_empty());
        assert_eq!(vo.region_name, "");
    }

    // ————————————————————— 默认国家 —————————————————————

    #[test]
    fn default_country_parsing_matches_java_int_value_of() {
        assert_eq!(parse_default_country(Some("1")), Some(1));
        assert_eq!(parse_default_country(Some(" 1 ")), Some(1));
        assert_eq!(parse_default_country(Some("")), None);
        assert_eq!(parse_default_country(Some("   ")), None);
        assert_eq!(parse_default_country(Some("CN")), None);
        assert_eq!(parse_default_country(None), None);
    }

    // ————————————————————— 地区路径 —————————————————————

    #[test]
    fn region_path_stops_at_default_country_and_reverses() {
        let table = |id: i32| -> Option<Region> {
            match id {
                110101 => Some(region(110101, 110100, "东城区")),
                110100 => Some(region(110100, 110000, "北京市")),
                110000 => Some(region(110000, 1, "北京市")),
                1 => Some(region(1, 0, "中国")),
                _ => None,
            }
        };
        // 默认国家 = 中国 → 根到叶
        let path = collect_region_path(110101, Some(1), table);
        let names: Vec<&str> = path.iter().map(|r| r.region_name.as_str()).collect();
        assert_eq!(names, vec!["中国", "北京市", "北京市", "东城区"]);
        assert_eq!(path.first().unwrap().region_id, 1);
        assert_eq!(path.last().unwrap().region_id, 110101);

        // 无默认国家 → 同样走到 parent_id = 0 为止（结果一致）
        let path2 = collect_region_path(110101, None, table);
        assert_eq!(path2.len(), 4);
        assert_eq!(path2[0].region_id, 1);

        // 若默认国家是省，则在省处停止（不带上「中国」）
        let path3 = collect_region_path(110101, Some(110000), table);
        let names3: Vec<&str> = path3.iter().map(|r| r.region_name.as_str()).collect();
        assert_eq!(names3, vec!["北京市", "北京市", "东城区"]);
    }

    #[test]
    fn region_path_breaks_on_missing_node() {
        let only_leaf = |id: i32| -> Option<Region> {
            if id == 110101 {
                Some(region(110101, 110100, "东城区"))
            } else {
                None
            }
        };
        let path = collect_region_path(110101, Some(1), only_leaf);
        assert_eq!(path.len(), 1);
        assert_eq!(path[0].region_id, 110101);

        // 起点就不存在 → 空路径（Java 同样返回空列表）
        assert!(collect_region_path(999, Some(1), |_| None).is_empty());
    }

    #[test]
    fn region_path_node_carries_all_eight_columns() {
        let mut r = region(330106, 330100, "西湖区");
        r.level = Some(4);
        r.is_hot = 1;
        r.sort_order = 30;
        r.first_word = "X".into();
        r.standard_code = Some("CN".into());
        let node = region_to_path_node(&r);
        assert_eq!(node.region_id, 330106);
        assert_eq!(node.parent_id, 330100);
        assert_eq!(node.region_name, "西湖区");
        assert_eq!(node.level, Some(4));
        assert_eq!(node.is_hot, 1);
        assert_eq!(node.sort_order, 30);
        assert_eq!(node.first_word, "X");
        assert_eq!(node.standard_code.as_deref(), Some("CN"));
    }

    // ————————————————————— 地区树 —————————————————————

    #[test]
    fn region_tree_builds_from_root_and_keeps_order() {
        // 种子数据形态：中国(1,level1) → 省市 → 区
        let regions = vec![
            region(1, 0, "中国"),
            region(110000, 1, "北京市"),
            region(110100, 110000, "北京市"),
            region(110101, 110100, "东城区"),
            region(120000, 1, "天津市"),
        ];
        let tree = build_region_tree(regions);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].region_id, 1);
        assert_eq!(tree[0].region_name, "中国");
        assert_eq!(tree[0].parent_id, 0);
        assert_eq!(tree[0].children.len(), 2);
        assert_eq!(tree[0].children[0].region_id, 110000);
        assert_eq!(tree[0].children[1].region_id, 120000);
        assert_eq!(tree[0].children[0].children[0].region_id, 110100);
        assert_eq!(
            tree[0].children[0].children[0].children[0].region_id,
            110101
        );
        // 叶子 children 为空数组
        assert!(tree[0].children[0].children[0].children[0].children.is_empty());
        assert!(tree[0].contains(110101));
        assert!(!tree[0].contains(999));
    }

    #[test]
    fn region_tree_drops_orphans_and_handles_empty() {
        // 父节点不在结果集 → 丢弃（与 Hutool buildFromMap 一致）
        let tree = build_region_tree(vec![region(110101, 110100, "东城区")]);
        assert!(tree.is_empty());
        // 空输入
        assert!(build_region_tree(Vec::new()).is_empty());
        // 多个根
        let tree = build_region_tree(vec![region(1, 0, "中国"), region(2, 0, "美国")]);
        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].region_id, 1);
        assert_eq!(tree[1].region_id, 2);
    }

    #[test]
    fn region_tree_is_truncated_at_depth_eight() {
        // 造 10 层链：1 → 2 → … → 10
        let regions: Vec<Region> = (1..=10)
            .map(|id| region(id, if id == 1 { 0 } else { id - 1 }, &format!("L{id}")))
            .collect();
        let tree = build_region_tree(regions);
        // 一路向下数，最多 8 层
        let mut depth = 0;
        let mut cursor = tree.first();
        while let Some(node) = cursor {
            depth += 1;
            cursor = node.children.first();
        }
        assert_eq!(depth, REGION_TREE_MAX_DEEP);
        assert_eq!(REGION_TREE_MAX_DEEP, 8);
    }

    #[test]
    fn overseas_and_domestic_edition_flags_are_stable() {
        // 全功能部署：跨境/批发/O2O/供应商全开（与 identity::authority 的 Default 一致）
        assert!(default_is_overseas());
        assert_eq!(EditionFlags::default().is_overseas, 1);
    }

    // ————————————————————— 入参 JSON 兼容性 —————————————————————

    #[test]
    fn request_dtos_accept_frontend_payloads() {
        let list: AddressListRequest =
            serde_json::from_str(r#"{"userId":10086,"page":1,"size":15}"#).unwrap();
        // `Error` 不实现 PartialEq，逐分支断言而非 `assert_eq!(..., Ok(..))`
        assert_eq!(require_user_id(list.user_id).ok(), Some(10086));

        let create: AddressCreateRequest = serde_json::from_str(
            r#"{"userId":7,"consignee":"张三","regionIds":[1,330000],"address":"X路1号",
                "postcode":" 310000 ","isDefault":1,"isSelected":1}"#,
        )
        .unwrap();
        assert_eq!(create.is_default, Some(1));
        assert_eq!(
            normalize_postcode(create.postcode.as_deref()).unwrap(),
            Some("310000".into())
        );

        let set_selected: AddressUpdateRequest =
            serde_json::from_str(r#"{"id":12}"#).unwrap();
        assert_eq!(set_selected.id, Some(12));

        let update: AddressUpdateRequest = serde_json::from_str(
            r#"{"addressId":12,"consignee":"李四","regionIds":[1,330000],"isDefault":0}"#,
        )
        .unwrap();
        assert_eq!(update.address_id, Some(12));
        assert_eq!(update.region_ids.clone().flatten().unwrap().len(), 2);
    }
}