//! 客户黑名单域（member::user_blacklist）DTO / VO / 查询参数 / 常量 / 纯函数。
//!
//! 设计定稿见 `~/workspace/blacklist-design/proposal.md` §3/§4/§5：
//! - 条目主表 `user_blacklist` + 规则子表 `user_blacklist_rule`（产品 × 行为）
//!   + 范围子表 `user_blacklist_scope_shop` + 流水表 `user_blacklist_log`。
//! - 判定叠加：平台全平台 / 平台选定店铺 / 店铺本店取并集，任一命中即限制；
//!   店铺条目仅在平台开关 `shopBlacklistAllow = 1` 时生效（§9 定死项 2）。
//! - 用户端文案不出现「黑名单」字样（§7），只在后端 `Error::biz(1001, …)` 中使用。

use serde::{Deserialize, Serialize};

// ——————————————————————————————————————————————
// 枚举常量（§3.2 定死：3 产品 × 6 行为）
// ——————————————————————————————————————————————

/// 产品：商城。
pub const PRODUCT_MALL: &str = "mall";
/// 产品：分销。
pub const PRODUCT_DISTRIBUTION: &str = "distribution";
/// 产品：买家秀。
pub const PRODUCT_BUYER_SHOW: &str = "buyer_show";

/// 行为：下单（加购 + 提交双检）。
pub const BEHAVIOR_PLACE_ORDER: &str = "place_order";
/// 行为：商品评价。
pub const BEHAVIOR_PRODUCT_COMMENT: &str = "product_comment";
/// 行为：用户自助成为分销员。
pub const BEHAVIOR_BECOME_SALESMAN: &str = "become_salesman";
/// 行为：佣金发放。
pub const BEHAVIOR_COMMISSION_GRANT: &str = "commission_grant";
/// 行为：发布买家秀。
pub const BEHAVIOR_BUYER_SHOW_PUBLISH: &str = "buyer_show_publish";
/// 行为：评论买家秀。
pub const BEHAVIOR_BUYER_SHOW_COMMENT: &str = "buyer_show_comment";

/// 全部合法（产品, 行为）组合白名单。
pub const VALID_COMBOS: [(&str, &str); 6] = [
    (PRODUCT_MALL, BEHAVIOR_PLACE_ORDER),
    (PRODUCT_MALL, BEHAVIOR_PRODUCT_COMMENT),
    (PRODUCT_DISTRIBUTION, BEHAVIOR_BECOME_SALESMAN),
    (PRODUCT_DISTRIBUTION, BEHAVIOR_COMMISSION_GRANT),
    (PRODUCT_BUYER_SHOW, BEHAVIOR_BUYER_SHOW_PUBLISH),
    (PRODUCT_BUYER_SHOW, BEHAVIOR_BUYER_SHOW_COMMENT),
];

/// Phase 1 加入弹窗/接口开放的产品（仅商城；分销/买家秀拦截在 Phase 2 上线，
/// §8 定死 Phase 1 只开放商城两项，避免配了不生效的假配置）。
/// Phase 2 起六个行为全部开放，本常量仅作历史记录，校验层不再引用。
pub const PHASE1_PRODUCTS: [&str; 1] = [PRODUCT_MALL];

/// 归属：平台。
pub const OWNER_PLATFORM: i32 = 1;
/// 归属：店铺。
pub const OWNER_SHOP: i32 = 2;

/// 平台条目生效范围：全平台。
pub const APPLY_ALL: i32 = 0;
/// 平台条目生效范围：选定店铺。
pub const APPLY_SELECTED: i32 = 1;

/// 条目状态：生效中。
pub const STATUS_ACTIVE: i32 = 1;
/// 条目状态：已移除。
pub const STATUS_REMOVED: i32 = 0;

/// 流水操作类型：加入。
pub const OP_ADD: i32 = 1;
/// 流水操作类型：移除。
pub const OP_REMOVE: i32 = 2;
/// 流水操作类型：批量加入。
pub const OP_BATCH_ADD: i32 = 3;
/// 流水操作类型：批量移除。
pub const OP_BATCH_REMOVE: i32 = 4;
/// 流水操作类型：导入加入。
pub const OP_IMPORT_ADD: i32 = 5;
/// 流水操作类型：修改规则（合并）。
pub const OP_MERGE: i32 = 6;
/// 流水操作类型：开关变更。
pub const OP_SETTING: i32 = 7;

/// 来源：手动单个。
pub const SOURCE_MANUAL: i32 = 0;
/// 来源：会员列表批量。
pub const SOURCE_MEMBER_BATCH: i32 = 1;
/// 来源：黑名单页批量。
pub const SOURCE_PAGE_BATCH: i32 = 2;
/// 来源：Excel 导入。
pub const SOURCE_IMPORT: i32 = 3;

/// 平台开关 config 键：是否允许店铺拉黑客户（默认 0）。
pub const CONFIG_SHOP_BLACKLIST_ALLOW: &str = "shopBlacklistAllow";

// ——————————————————————————————————————————————
// 文案（admin 端校验/权限；用户端拦截文案见服务层按 §7 组装）
// ——————————————————————————————————————————————

/// 店铺建条目前置：平台未开启店铺黑名单功能。
pub const MSG_SHOP_BLACKLIST_DISABLED: &str = "平台未开启店铺黑名单功能";
/// 限制原因必填。
pub const MSG_REASON_REQUIRED: &str = "限制原因不能为空";
/// 限制规则必填。
pub const MSG_RULES_REQUIRED: &str = "请至少选择一项限制规则";
/// 会员不存在。
pub const MSG_USER_NOT_EXISTS: &str = "会员不存在";
/// 条目不存在。
pub const MSG_ENTRY_NOT_EXISTS: &str = "黑名单条目不存在";
/// 条目已移除。
pub const MSG_ENTRY_ALREADY_REMOVED: &str = "该条目已移除";
/// 店铺只能操作本店客户。
pub const MSG_NOT_SHOP_MEMBER: &str = "该客户不是本店客户，无法加入黑名单";
/// 平台移除店铺条目时的提示（§6 定死文案口径）。
pub const MSG_PLATFORM_CANNOT_REMOVE_SHOP: &str = "店铺黑名单由店铺自行移除";
/// 店铺移除他人条目时的提示。
pub const MSG_SHOP_CANNOT_REMOVE_OTHERS: &str = "只能移除本店的黑名单条目";
/// 选定店铺模式必须选店铺。
pub const MSG_SCOPE_SHOPS_REQUIRED: &str = "请选择生效店铺";
/// 仅平台可修改开关。
pub const MSG_SETTING_PLATFORM_ONLY: &str = "仅平台账号可修改黑名单设置";

// ——————————————————————————————————————————————
// 纯函数：组合校验 / 名称 / 判定叠加 / 规则合并
// ——————————————————————————————————————————————

/// （产品, 行为）是否为合法组合。
#[must_use]
pub fn is_valid_combo(product: &str, behavior: &str) -> bool {
    VALID_COMBOS.contains(&(product, behavior))
}

/// 产品中文名（未知码原样返回）。
#[must_use]
pub fn product_name(code: &str) -> &str {
    match code {
        PRODUCT_MALL => "商城",
        PRODUCT_DISTRIBUTION => "分销",
        PRODUCT_BUYER_SHOW => "买家秀",
        _ => "未知产品",
    }
}

/// 行为中文名（未知码原样返回）。
#[must_use]
pub fn behavior_name(code: &str) -> &str {
    match code {
        BEHAVIOR_PLACE_ORDER => "下单",
        BEHAVIOR_PRODUCT_COMMENT => "评价",
        BEHAVIOR_BECOME_SALESMAN => "自助成为分销员",
        BEHAVIOR_COMMISSION_GRANT => "佣金发放",
        BEHAVIOR_BUYER_SHOW_PUBLISH => "买家秀发布",
        BEHAVIOR_BUYER_SHOW_COMMENT => "买家秀评论",
        _ => "未知行为",
    }
}

/// 一条生效条目在判定时的视图（纯逻辑层；条目须已按 status=1 过滤，
/// 移除后的条目不进入候选集 ⇒ 天然不命中）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestrictionCandidate {
    /// 归属：1 平台 / 2 店铺。
    pub owner_type: i32,
    /// 店铺归属的 shop_id（平台为 0）。
    pub owner_shop_id: i32,
    /// 平台条目范围：0 全平台 / 1 选定店铺。
    pub apply_mode: i32,
    /// apply_mode=1 时的生效店铺集合。
    pub scope_shop_ids: Vec<i32>,
}

/// 判定一条候选条目是否命中给定店铺上下文（§4.2 叠加逻辑的单条部分）。
///
/// - 平台全平台：任何上下文（含 `None`）命中；
/// - 平台选定店铺：仅 `shop_id = Some(s)` 且 `s` 在范围内命中，`None` 不命中；
/// - 店铺条目：仅开关开启且 `shop_id = Some(本店)` 命中，`None` 不命中。
#[must_use]
pub fn candidate_matches(
    candidate: &RestrictionCandidate,
    shop_id: Option<i32>,
    shop_entries_enabled: bool,
) -> bool {
    match candidate.owner_type {
        OWNER_PLATFORM => {
            if candidate.apply_mode == APPLY_SELECTED {
                match shop_id {
                    Some(s) => candidate.scope_shop_ids.contains(&s),
                    None => false,
                }
            } else {
                true
            }
        }
        OWNER_SHOP => {
            if !shop_entries_enabled {
                return false;
            }
            matches!(shop_id, Some(s) if s == candidate.owner_shop_id)
        }
        _ => false,
    }
}

/// 在候选集上做叠加判定：任一命中即限制（取并集，无反向豁免）。
#[must_use]
pub fn any_candidate_matches(
    candidates: &[RestrictionCandidate],
    shop_id: Option<i32>,
    shop_entries_enabled: bool,
) -> bool {
    candidates
        .iter()
        .any(|c| candidate_matches(c, shop_id, shop_entries_enabled))
}

/// 佣金发放判定的店铺上下文（§4.2 定死）：优先收佣分销员的
/// `salesman.shop_id`（0/None 视为无店铺归属）；分销员无店铺归属时以订单的
/// `order.shop_id` 兜底；两者皆无 → `None`（仅平台全平台规则可命中）。
#[must_use]
pub fn commission_shop_context(
    salesman_shop_id: Option<i32>,
    order_shop_id: Option<i32>,
) -> Option<i32> {
    match salesman_shop_id {
        Some(s) if s > 0 => Some(s),
        _ => order_shop_id,
    }
}

// ——————————————————————————————————————————————
// Phase 3：导入解析纯函数（§5.2 定死口径）
// ——————————————————————————————————————————————

/// 导入模板列头（顺序即解析列序，§5.2；导入时校验表头名防错版）。
pub const IMPORT_HEADERS: [&str; 7] = [
    "会员ID",
    "手机号",
    "用户名",
    "限制产品",
    "限制行为",
    "生效店铺ID",
    "限制原因",
];

/// 失败明细列头（会员导入错误表同款风格：原列 + 错误信息）。
pub const IMPORT_ERROR_HEADERS: [&str; 8] = [
    "行号",
    "会员ID",
    "手机号",
    "用户名",
    "限制产品",
    "限制行为",
    "生效店铺ID",
    "错误信息",
];

/// 导出列头（§5.1：与列表展示口径一致）。
pub const EXPORT_HEADERS: [&str; 11] = [
    "会员ID",
    "用户名",
    "昵称",
    "手机号",
    "限制产品",
    "限制行为",
    "限制范围",
    "原因",
    "状态",
    "操作人",
    "加入时间",
];

/// 单次导入数据行上限（照 shop_member 导入先例，§5.2）。
pub const IMPORT_MAX_ROWS: usize = 1000;

/// 导入行「限制产品」解析：中文名或码双认（§5.2）。
#[must_use]
pub fn parse_product_code(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        "商城" | PRODUCT_MALL => Some(PRODUCT_MALL),
        "分销" | PRODUCT_DISTRIBUTION => Some(PRODUCT_DISTRIBUTION),
        "买家秀" | PRODUCT_BUYER_SHOW => Some(PRODUCT_BUYER_SHOW),
        _ => None,
    }
}

/// 导入行「限制行为」解析：中文名或码双认（§5.2）。
/// 中文名以 §3.2 的行为名为准，另兼容「商品评价 / 发布买家秀 / 评论买家秀」说法。
#[must_use]
pub fn parse_behavior_code(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        "下单" | BEHAVIOR_PLACE_ORDER => Some(BEHAVIOR_PLACE_ORDER),
        "评价" | "商品评价" | BEHAVIOR_PRODUCT_COMMENT => Some(BEHAVIOR_PRODUCT_COMMENT),
        "自助成为分销员" | BEHAVIOR_BECOME_SALESMAN => Some(BEHAVIOR_BECOME_SALESMAN),
        "佣金发放" | BEHAVIOR_COMMISSION_GRANT => Some(BEHAVIOR_COMMISSION_GRANT),
        "买家秀发布" | "发布买家秀" | BEHAVIOR_BUYER_SHOW_PUBLISH => {
            Some(BEHAVIOR_BUYER_SHOW_PUBLISH)
        }
        "买家秀评论" | "评论买家秀" | BEHAVIOR_BUYER_SHOW_COMMENT => {
            Some(BEHAVIOR_BUYER_SHOW_COMMENT)
        }
        _ => None,
    }
}

/// 导入行「生效店铺ID」解析：多个以逗号（半角/全角）或顿号分隔。
/// 空单元格 → 空集合；任一 token 非数字 → Err。
pub fn parse_import_shop_ids(raw: &str) -> Result<Vec<i32>, &'static str> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for token in raw.split([',', '，', '、']) {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let id: i32 = token.parse().map_err(|_| "生效店铺ID格式不正确")?;
        if !out.contains(&id) {
            out.push(id);
        }
    }
    Ok(out)
}

/// 导入行会员匹配（§5.2 定死）：会员ID > 手机号 > 用户名，全部精确匹配；
/// 三者指向同一人才算成功。
///
/// 入参为三层语义：外层 `None` = 该列未填；内层 `None` = 已填但查无此人。
/// 错误文案与 §5.2 失败情形逐字对应。
pub fn match_import_user(
    by_id: Option<Option<i32>>,
    by_mobile: Option<Option<i32>>,
    by_username: Option<Option<i32>>,
) -> Result<i32, &'static str> {
    if by_id.is_none() && by_mobile.is_none() && by_username.is_none() {
        return Err("会员ID、手机号、用户名至少填写一项");
    }
    // 按优先级逐个校验存在性：先报错的是优先级最高的未命中标识。
    let mut found: Vec<i32> = Vec::new();
    for resolved in [by_id, by_mobile, by_username].into_iter().flatten() {
        match resolved {
            Some(user_id) => found.push(user_id),
            None => return Err(MSG_USER_NOT_EXISTS),
        }
    }
    // flatten 后 found 的顺序即 ID > 手机号 > 用户名；不一致即失败。
    let first = found[0];
    if found.iter().any(|id| *id != first) {
        return Err("标识不一致");
    }
    Ok(first)
}

/// 规则合并（重复加入语义，§4.4）：已有规则 ∪ 新增规则，保持已有顺序、
/// 新增按入参顺序追加，重复组合只保留一份。返回 (合并后全集, 实际新增数)。
#[must_use]
pub fn merge_rule_keys(
    existing: &[(String, String)],
    new_rules: &[(String, String)],
) -> (Vec<(String, String)>, usize) {
    let mut merged: Vec<(String, String)> = existing.to_vec();
    let mut added = 0usize;
    for rule in new_rules {
        if !merged.contains(rule) {
            merged.push(rule.clone());
            added += 1;
        }
    }
    (merged, added)
}

// ——————————————————————————————————————————————
// 操作人上下文（路由层由 AuthUser 推导，服务层不信任请求体中的归属字段）
// ——————————————————————————————————————————————

/// 管理端操作人上下文。
#[derive(Debug, Clone, Default)]
pub struct BlacklistOperator {
    /// 管理员 admin_user.user_id。
    pub admin_id: i32,
    /// 管理员用户名快照。
    pub admin_name: String,
    /// admin_type 快照（admin/shop/store/…）。
    pub admin_type: String,
    /// 作用域店铺：平台 = 0，店铺账号 = 本店 shop_id。
    pub shop_id: i32,
    /// 是否平台账号。
    pub is_platform: bool,
}

impl BlacklistOperator {
    /// 本操作人新建条目的归属类型。
    #[must_use]
    pub fn owner_type(&self) -> i32 {
        if self.is_platform {
            OWNER_PLATFORM
        } else {
            OWNER_SHOP
        }
    }

    /// 本操作人新建条目的归属店铺（平台 = 0）。
    #[must_use]
    pub fn owner_shop_id(&self) -> i32 {
        if self.is_platform {
            0
        } else {
            self.shop_id
        }
    }
}

// ——————————————————————————————————————————————
// 入参
// ——————————————————————————————————————————————

/// 一条「产品 × 行为」规则。
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BlacklistRuleDTO {
    /// 产品码（mall / distribution / buyer_show）。
    pub product: String,
    /// 行为码（见 §3.2）。
    pub behavior: String,
}

/// 条目列表查询。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BlacklistListQuery {
    /// 关键词：用户名/昵称/手机号/邮箱模糊。
    pub keyword: Option<String>,
    /// 会员 ID 精确。
    pub user_id: Option<i32>,
    /// 产品码过滤（经规则子表 EXISTS）。
    pub product_code: Option<String>,
    /// 行为码过滤（经规则子表 EXISTS）。
    pub behavior_code: Option<String>,
    /// 归属过滤：1 平台 / 2 店铺。
    pub owner_type: Option<i32>,
    /// 店铺归属过滤（owner_shop_id）。
    pub shop_id: Option<i32>,
    /// 状态：1 生效中 / 0 已移除（不传 = 全部）。
    pub status: Option<i32>,
    /// 加入时间起（秒）。
    pub create_time_start: Option<i64>,
    /// 加入时间止（秒）。
    pub create_time_end: Option<i64>,
    pub page: Option<i64>,
    pub size: Option<i64>,
}

impl BlacklistListQuery {
    /// `(LIMIT, OFFSET)`（口径同 shop_member：默认 15、上限 100）。
    #[must_use]
    pub fn limit_offset(&self) -> (i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let size = match self.size {
            None => 15,
            Some(v) if v > 100 => 100,
            Some(v) if v < 1 => 1,
            Some(v) => v,
        };
        (size, (page - 1) * size)
    }

    /// 有效关键字（空串视为不过滤）。
    #[must_use]
    pub fn effective_keyword(&self) -> Option<&str> {
        self.keyword.as_deref().filter(|s| !s.is_empty())
    }
}

/// 单个加入入参（owner 一律由操作人推导，请求体不设 owner 字段）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BlacklistAddParam {
    /// 会员定位三选一：ID。
    pub user_id: Option<i32>,
    /// 会员定位三选一：手机号（精确）。
    pub mobile: Option<String>,
    /// 会员定位三选一：用户名（精确）。
    pub username: Option<String>,
    /// 限制规则（产品 × 行为）列表。
    pub rules: Vec<BlacklistRuleDTO>,
    /// 平台专用：0 全平台 / 1 选定店铺（店铺账号传了也忽略）。
    pub apply_mode: Option<i32>,
    /// 平台 apply_mode=1 时的生效店铺。
    pub shop_ids: Vec<i32>,
    /// 限制原因（必填，≤500 字）。
    pub reason: String,
    /// 来源（0 手动单个 / 1 会员列表批量 / 2 黑名单页批量 / 3 导入）。
    pub source: Option<i32>,
}

/// 批量加入入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BlacklistBatchAddParam {
    pub user_ids: Vec<i32>,
    pub rules: Vec<BlacklistRuleDTO>,
    pub apply_mode: Option<i32>,
    pub shop_ids: Vec<i32>,
    pub reason: String,
    pub source: Option<i32>,
}

/// 单个移除入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BlacklistRemoveParam {
    pub id: i64,
    pub remove_reason: Option<String>,
}

/// 批量移除入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BlacklistBatchRemoveParam {
    pub ids: Vec<i64>,
    pub remove_reason: Option<String>,
}

/// 操作记录查询。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BlacklistLogQuery {
    pub user_id: Option<i32>,
    pub blacklist_id: Option<i64>,
    pub op_type: Option<i32>,
    pub create_time_start: Option<i64>,
    pub create_time_end: Option<i64>,
    pub page: Option<i64>,
    pub size: Option<i64>,
}

impl BlacklistLogQuery {
    /// `(LIMIT, OFFSET)`。
    #[must_use]
    pub fn limit_offset(&self) -> (i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let size = match self.size {
            None => 15,
            Some(v) if v > 100 => 100,
            Some(v) if v < 1 => 1,
            Some(v) => v,
        };
        (size, (page - 1) * size)
    }
}

/// 开关保存入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BlacklistSettingParam {
    /// 是否允许店铺将客户加入店铺黑名单（0/1）。
    pub shop_blacklist_allow: Option<i32>,
}

// ——————————————————————————————————————————————
// 出参 VO
// ——————————————————————————————————————————————

/// 会员摘要（列表/详情共用；店铺视角下 mobile/nickname 已按脱敏口径处理）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlacklistUserVO {
    pub user_id: i32,
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub mobile: Option<String>,
    pub avatar: Option<String>,
}

/// 条目列表/详情行。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlacklistEntryVO {
    pub id: i64,
    pub user_id: i32,
    /// 会员摘要（会员已删除时为 None）。
    pub user: Option<BlacklistUserVO>,
    /// 归属：1 平台 / 2 店铺。
    pub owner_type: i32,
    pub owner_shop_id: i32,
    /// 归属店铺名（平台条目为 None）。
    pub owner_shop_name: Option<String>,
    /// 平台条目范围：0 全平台 / 1 选定店铺。
    pub apply_mode: i32,
    /// apply_mode=1 时的生效店铺 ID。
    pub shop_ids: Vec<i32>,
    /// 生效店铺名（与 shop_ids 对齐，未知店铺为 None 占位过滤后按序对应名称表）。
    pub shop_names: Vec<String>,
    /// 规则组合（产品 × 行为）。
    pub rules: Vec<BlacklistRuleDTO>,
    pub reason: String,
    pub source: i32,
    /// 1 生效中 / 0 已移除。
    pub status: i32,
    pub create_admin_id: i32,
    pub create_admin_name: String,
    pub create_admin_type: String,
    pub create_time: i64,
    pub update_time: i64,
    pub remove_admin_id: i32,
    pub remove_admin_name: String,
    pub remove_time: i64,
    pub remove_reason: String,
    /// 当前操作人是否可移除该条目（平台只可移除平台条目，店铺只可移除本店条目）。
    pub can_remove: bool,
}

/// 加入结果。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlacklistAddResultVO {
    /// 条目 ID（新建或合并到的既有条目）。
    pub id: i64,
    /// 是否合并进既有生效条目（重复加入语义）。
    pub merged: bool,
}

/// 批量操作失败项。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlacklistFailureVO {
    /// 目标（批量加入 = userId，批量移除 = 条目 id）。
    pub target_id: i64,
    pub msg: String,
}

/// 批量加入结果。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlacklistBatchAddResultVO {
    pub success_count: i32,
    pub merged_count: i32,
    pub failures: Vec<BlacklistFailureVO>,
}

/// 批量移除结果。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlacklistBatchRemoveResultVO {
    pub success_count: i32,
    pub failures: Vec<BlacklistFailureVO>,
}

/// 操作记录行。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlacklistLogVO {
    pub id: i64,
    pub blacklist_id: i64,
    pub user_id: i32,
    /// 1 加入 / 2 移除 / 3 批量加入 / 4 批量移除 / 5 导入加入 / 6 修改规则 / 7 开关变更。
    pub op_type: i32,
    pub op_content: String,
    /// 本次操作后的规则快照 [{product, behavior}]。
    pub rules_snapshot: serde_json::Value,
    /// 本次范围快照 {ownerType, ownerShopId, applyMode, shopIds}。
    pub scope_snapshot: serde_json::Value,
    pub operator_id: i32,
    pub operator_name: String,
    pub operator_type: String,
    pub operator_shop_id: i32,
    pub create_time: i64,
}

/// 开关 VO。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlacklistSettingVO {
    pub shop_blacklist_allow: i32,
}

/// 导入失败行（导入结果与失败明细下载共用，§5.1/§5.2）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlacklistImportErrorVO {
    /// 表格行号（表头为第 1 行，首条数据行 = 2）。
    pub row: i64,
    /// 原始单元格：会员ID。
    pub user_id: String,
    /// 原始单元格：手机号。
    pub mobile: String,
    /// 原始单元格：用户名。
    pub username: String,
    /// 原始单元格：限制产品。
    pub product: String,
    /// 原始单元格：限制行为。
    pub behavior: String,
    /// 原始单元格：生效店铺ID。
    pub shop_ids: String,
    pub error_msg: String,
}

/// 批量导入结果（不以 Err 表达部分失败，§5.1）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlacklistImportResultVO {
    /// 有效数据行数（不含表头与全空行）。
    pub total: i32,
    /// 成功行数（归属会员成功加入/合并的行）。
    pub success_count: i32,
    /// 合并到既有生效条目的会员数。
    pub merged_count: i32,
    /// 失败行数（= errors 条数）。
    pub fail_count: i32,
    pub errors: Vec<BlacklistImportErrorVO>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(owner_type: i32, owner_shop_id: i32, apply_mode: i32, scope: &[i32]) -> RestrictionCandidate {
        RestrictionCandidate {
            owner_type,
            owner_shop_id,
            apply_mode,
            scope_shop_ids: scope.to_vec(),
        }
    }

    #[test]
    fn combo_whitelist() {
        assert!(is_valid_combo("mall", "place_order"));
        assert!(is_valid_combo("mall", "product_comment"));
        assert!(is_valid_combo("distribution", "become_salesman"));
        assert!(is_valid_combo("distribution", "commission_grant"));
        assert!(is_valid_combo("buyer_show", "buyer_show_publish"));
        assert!(is_valid_combo("buyer_show", "buyer_show_comment"));
        assert!(!is_valid_combo("mall", "become_salesman"));
        assert!(!is_valid_combo("buyer_show", "place_order"));
        assert!(!is_valid_combo("unknown", "place_order"));
    }

    #[test]
    fn platform_all_matches_any_context() {
        let c = cand(OWNER_PLATFORM, 0, APPLY_ALL, &[]);
        assert!(candidate_matches(&c, Some(7), false), "任意店铺命中");
        assert!(candidate_matches(&c, Some(0), false), "平台自营命中");
        assert!(candidate_matches(&c, None, false), "无店铺上下文也命中");
    }

    #[test]
    fn platform_selected_matches_only_listed_shops() {
        let c = cand(OWNER_PLATFORM, 0, APPLY_SELECTED, &[3, 5]);
        assert!(candidate_matches(&c, Some(3), false));
        assert!(candidate_matches(&c, Some(5), false));
        assert!(!candidate_matches(&c, Some(4), false), "无关店铺不命中");
        assert!(!candidate_matches(&c, None, false), "无店铺上下文不命中");
    }

    #[test]
    fn shop_entry_matches_only_own_shop_and_only_when_enabled() {
        let c = cand(OWNER_SHOP, 9, APPLY_ALL, &[]);
        assert!(candidate_matches(&c, Some(9), true), "本店命中");
        assert!(!candidate_matches(&c, Some(10), true), "无关店铺不命中");
        assert!(!candidate_matches(&c, None, true), "无店铺上下文不命中");
        assert!(!candidate_matches(&c, Some(9), false), "开关关闭时店铺条目暂停生效");
    }

    #[test]
    fn overlay_union_and_removed_entries_absent() {
        // 店铺条目 + 平台选定条目叠加：任一命中即限制
        let candidates = vec![
            cand(OWNER_SHOP, 9, APPLY_ALL, &[]),
            cand(OWNER_PLATFORM, 0, APPLY_SELECTED, &[3]),
        ];
        assert!(any_candidate_matches(&candidates, Some(9), true));
        assert!(any_candidate_matches(&candidates, Some(3), true));
        assert!(!any_candidate_matches(&candidates, Some(4), true));
        // 移除后条目不进入候选集（仓储按 status=1 过滤）⇒ 不命中
        let after_remove: Vec<RestrictionCandidate> = Vec::new();
        assert!(!any_candidate_matches(&after_remove, Some(9), true));
        assert!(!any_candidate_matches(&after_remove, None, true));
    }

    #[test]
    fn merge_rules_dedups_and_counts_added() {
        let existing = vec![
            ("mall".to_string(), "place_order".to_string()),
            ("mall".to_string(), "product_comment".to_string()),
        ];
        let new_rules = vec![
            ("mall".to_string(), "place_order".to_string()),
            ("buyer_show".to_string(), "buyer_show_publish".to_string()),
        ];
        let (merged, added) = merge_rule_keys(&existing, &new_rules);
        assert_eq!(added, 1);
        assert_eq!(merged.len(), 3);
        // 全部重复时新增为 0（合并语义：不报错、不产生第二份规则）
        let (_, added0) = merge_rule_keys(&existing, &existing);
        assert_eq!(added0, 0);
    }

    #[test]
    fn operator_owner_derivation() {
        let platform = BlacklistOperator {
            is_platform: true,
            shop_id: 0,
            ..Default::default()
        };
        assert_eq!(platform.owner_type(), OWNER_PLATFORM);
        assert_eq!(platform.owner_shop_id(), 0);
        let shop = BlacklistOperator {
            is_platform: false,
            shop_id: 12,
            ..Default::default()
        };
        assert_eq!(shop.owner_type(), OWNER_SHOP);
        assert_eq!(shop.owner_shop_id(), 12);
    }

    #[test]
    fn buyer_show_context_matches_only_platform_all() {
        // 买家秀无 shop_id：判定上下文恒为 None（§1.4/§4.2）。
        // 平台全平台规则命中；平台选定店铺与店铺条目一律不命中。
        let platform_all = cand(OWNER_PLATFORM, 0, APPLY_ALL, &[]);
        let platform_selected = cand(OWNER_PLATFORM, 0, APPLY_SELECTED, &[3, 5]);
        let shop_entry = cand(OWNER_SHOP, 3, APPLY_ALL, &[]);
        assert!(candidate_matches(&platform_all, None, true));
        assert!(!candidate_matches(&platform_selected, None, true));
        assert!(!candidate_matches(&shop_entry, None, true));
        // 分销自助成为分销员同为 None 上下文，同一结论。
        let candidates = vec![platform_selected, shop_entry];
        assert!(!any_candidate_matches(&candidates, None, true));
    }

    #[test]
    fn commission_context_prefers_salesman_shop_then_order_shop() {
        assert_eq!(commission_shop_context(Some(7), Some(9)), Some(7), "分销员店铺优先");
        assert_eq!(commission_shop_context(Some(0), Some(9)), Some(9), "分销员 shop_id=0 视为无归属，走订单店铺兜底");
        assert_eq!(commission_shop_context(None, Some(9)), Some(9));
        assert_eq!(commission_shop_context(Some(7), None), Some(7));
        assert_eq!(commission_shop_context(Some(0), None), None);
        assert_eq!(commission_shop_context(None, None), None);
    }

    #[test]
    fn import_parse_product_and_behavior_bilingual() {
        assert_eq!(parse_product_code("商城"), Some(PRODUCT_MALL));
        assert_eq!(parse_product_code("mall"), Some(PRODUCT_MALL));
        assert_eq!(parse_product_code(" 分销 "), Some(PRODUCT_DISTRIBUTION));
        assert_eq!(parse_product_code("买家秀"), Some(PRODUCT_BUYER_SHOW));
        assert_eq!(parse_product_code("buyer_show"), Some(PRODUCT_BUYER_SHOW));
        assert_eq!(parse_product_code("超市"), None);
        assert_eq!(parse_behavior_code("下单"), Some(BEHAVIOR_PLACE_ORDER));
        assert_eq!(parse_behavior_code("place_order"), Some(BEHAVIOR_PLACE_ORDER));
        assert_eq!(parse_behavior_code("评价"), Some(BEHAVIOR_PRODUCT_COMMENT));
        assert_eq!(parse_behavior_code("自助成为分销员"), Some(BEHAVIOR_BECOME_SALESMAN));
        assert_eq!(parse_behavior_code("佣金发放"), Some(BEHAVIOR_COMMISSION_GRANT));
        assert_eq!(parse_behavior_code("买家秀发布"), Some(BEHAVIOR_BUYER_SHOW_PUBLISH));
        assert_eq!(parse_behavior_code("买家秀评论"), Some(BEHAVIOR_BUYER_SHOW_COMMENT));
        assert_eq!(parse_behavior_code("评论买家秀"), Some(BEHAVIOR_BUYER_SHOW_COMMENT));
        assert_eq!(parse_behavior_code("点赞"), None);
        // 解析出的组合须过白名单（跨产品组合在服务层被 is_valid_combo 拒绝）。
        assert!(is_valid_combo(
            parse_product_code("买家秀").unwrap(),
            parse_behavior_code("买家秀发布").unwrap()
        ));
        assert!(!is_valid_combo(
            parse_product_code("买家秀").unwrap(),
            parse_behavior_code("下单").unwrap()
        ));
    }

    #[test]
    fn import_parse_shop_ids_splits_and_dedups() {
        assert_eq!(parse_import_shop_ids("").unwrap(), Vec::<i32>::new());
        assert_eq!(parse_import_shop_ids("3").unwrap(), vec![3]);
        assert_eq!(parse_import_shop_ids("3,5，5、7").unwrap(), vec![3, 5, 7]);
        assert_eq!(parse_import_shop_ids("abc").unwrap_err(), "生效店铺ID格式不正确");
    }

    #[test]
    fn import_match_priority_consistency_and_existence() {
        // 只有手机号 → 按手机号命中（优先级 ID > 手机号 > 用户名）。
        assert_eq!(match_import_user(None, Some(Some(42)), None), Ok(42));
        // 只有用户名 → 按用户名命中。
        assert_eq!(match_import_user(None, None, Some(Some(7))), Ok(7));
        // 三者一致 → 命中（值取优先级最高的会员ID解析结果）。
        assert_eq!(
            match_import_user(Some(Some(9)), Some(Some(9)), Some(Some(9))),
            Ok(9)
        );
        // 指向不同人 → 标识不一致。
        assert_eq!(
            match_import_user(Some(Some(9)), Some(Some(10)), None),
            Err("标识不一致")
        );
        assert_eq!(
            match_import_user(None, Some(Some(9)), Some(Some(10))),
            Err("标识不一致")
        );
        // 已填但查无此人 → 会员不存在（按优先级先报会员ID）。
        assert_eq!(match_import_user(Some(None), Some(Some(9)), None), Err(MSG_USER_NOT_EXISTS));
        assert_eq!(match_import_user(None, Some(None), None), Err(MSG_USER_NOT_EXISTS));
        // 全空 → 至少填写一项。
        assert_eq!(
            match_import_user(None, None, None),
            Err("会员ID、手机号、用户名至少填写一项")
        );
    }

    #[test]
    fn list_query_paging_defaults() {
        let q = BlacklistListQuery::default();
        assert_eq!(q.limit_offset(), (15, 0));
        let q = BlacklistListQuery {
            page: Some(2),
            size: Some(500),
            keyword: Some(String::new()),
            ..Default::default()
        };
        assert_eq!(q.limit_offset(), (100, 100));
        assert_eq!(q.effective_keyword(), None);
    }
}
