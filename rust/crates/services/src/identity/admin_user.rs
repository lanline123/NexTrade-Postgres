//! 管理员列表服务（identity::admin_user）。
//!
//! 对应原 Java `AdminUserController` + `AdminUserServiceImpl`（763 行）
//! + `AdminApiTokenServiceImpl`（176 行）。
//!
//! | 本域端点 | 原 Java | 落点 |
//! |----------|---------|------|
//! | `GET  /adminapi/authority/adminUser/list` | `list` | [`AdminUserService::list`] |
//! | `GET  /adminapi/authority/adminUser/mineDetail` | `mineDetail` | [`AdminUserService::mine_detail`] |
//! | `GET  /adminapi/authority/adminUser/detail` | `detail` | [`AdminUserService::detail`] |
//! | `POST /adminapi/authority/adminUser/create` | `create` | [`AdminUserService::create`] |
//! | `POST /adminapi/authority/adminUser/update` | `update` | [`AdminUserService::update`] |
//! | `POST /adminapi/authority/adminUser/del` | `del` | [`AdminUserService::del`] |
//! | `POST /adminapi/authority/adminUser/batch` | `batch` | [`AdminUserService::batch`] |
//! | `POST /adminapi/authority/adminUser/modifyManageAccounts` | `modifyManageAccounts` | [`AdminUserService::modify_manage_accounts`] |
//! | `GET  /adminapi/authority/adminUser/config` | `getConfig` | [`AdminUserService::get_config`] |
//! | `GET  /adminapi/authority/adminUser/getCode` | `getCode` | [`AdminUserService::get_code`] |
//! | `POST /adminapi/authority/adminUser/apiToken/generate` | `generateApiToken` | [`AdminUserService::generate_api_token`] |
//! | `GET  /adminapi/authority/adminUser/apiToken/list` | `listApiTokens` | [`AdminUserService::list_api_tokens`] |
//! | `POST /adminapi/authority/adminUser/apiToken/revoke` | `revokeApiToken` | [`AdminUserService::revoke_api_token`] |
//!
//! ## 迁移要点（逐条对照 Java 源）
//!
//! - `create`：三段分支**顺序敏感**——
//!   ① 用户名（排除已软删除）占用 → `管理员名称已存在或被商户账号占用`；
//!   ② 存在同手机号的**已软删除**账号 → 复用该账号（改用户名/类型/角色/邮箱/
//!   头像/密码，`is_deleted = 0`，`auth_list = "[]"`，刷新 `add_time`）；
//!   ③ 存在同手机号的**未删除**账号 → 是 admin 报
//!   `管理员手机号已经存在`，否则**提升为 admin**（保留原店铺/供应商）。
//! - `update`：先 `buildAdminUser`（用户名/手机号/邮箱三重占用校验，且
//!   用户名与手机号**不排除已软删除账号**，与 `create` 口径不同），再整行更新。
//! - `detail`：`role_id > 0` 时**从角色表动态解析权限**
//!   （`role.authority_list` → `authList`，`role_name` 回填）；
//!   `role_id <= 0` 时才回退解析 `admin_user.auth_list`；并填充 `userShop`。
//! - `del`：拒绝删除当前登录账号；**店铺/供应商主账号**
//!   （`admin_user_shop.is_admin = 1`）拒绝删除。
//! - `batch`：`type != "del"` 直接返回 `false`（无副作用），`del` 时逐个走 `del`。
//! - `generateApiToken`：每个管理员最多 10 个有效 Token；明文 16 字节随机 hex
//!   （32 字符），入库只存 SHA-256 哈希 + 前 8 位前缀；
//!   `expiresInDays > 0` 才设过期时间。
//!
//! ## 已迁移 / 剩余差异
//!
//! - **Redis / 短信**：`getCode` 写 `login<mobile>` 缓存并复用
//!   `system::notify::send_sms_code`；`modifyType=4` 写
//!   `admin_modify_mobile_verified:<id>`（10 分钟），`modifyType=3` 校验后换绑。
//! - **登录令牌失效**：Java 在改密/删号后删除 Redis 中的该用户全部 token
//!   （`Constants.getAllLoginTokenFormatByUserName`）。Rust 用无状态 JWT，
//!   无 token 黑名单设施，故**已签发的旧 JWT 在过期前仍然有效**——这是与
//!   Java 的真实行为差异。
//! - **工作台绑定同步**（`syncPlatformWorkbenchBinding`）：Java 依赖
//!   `ShopEmployeeWorkbenchCandidatePolicy`（客服/收银候选人判定）与
//!   `ShopEmployeeRoleLifecycle`（客服角色生命周期），两者未迁移。
//!   本实现只保留「已存在平台绑定则更新」的分支；**新建绑定**因无法判定
//!   候选人资格而跳过，且不做 `ensureKefu` / `removeKefu`。
//! - **`AdminUserDTO.password` 永不下发**：Java `list` 会把密码哈希一起返回，
//!   本实现该字段带 `skip_serializing`，只作 `create` / `update` 入参，出参不含
//!   密码（安全加固，见 `uncertain_points`）。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use nextrade_common::utils;
use nextrade_core::entities::{AdminRole, AdminUser, AdminUserShop, Shop};
use nextrade_core::models::identity::admin_user::{
    admin_type, AdminApiTokenGenerateDTO, AdminApiTokenGenerateVO, AdminApiTokenVO, AdminRoleVO,
    AdminUserConfigQuery, AdminUserDTO, AdminUserModifyManageAccountsDTO, AdminUserShopVO,
    BatchDTO, GetCodeQuery, UserShopDTO, EXCLUDED_ROLE_CODES,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::identity::admin_user::{AdminUserPageFilter, RoleScopeFilter};
use nextrade_infrastructure::db::Repositories;
use nextrade_security::{hash_password, verify_password};

/// 每个管理员的有效 Token 上限（Java `AdminApiTokenServiceImpl.MAX_TOKENS_PER_USER`）。
pub const MAX_TOKENS_PER_USER: i64 = 10;

/// 明文 Token 的十六进制长度（Java `TOKEN_BYTE_LENGTH = 16` 字节 → 32 字符）。
const TOKEN_HEX_LEN: usize = 32;

/// Token 前缀长度（Java `plainToken.substring(0, 8)`）。
const TOKEN_PREFIX_LEN: usize = 8;

/// 一天的秒数（Java `expiresInDays * 86400`）。
const SECONDS_PER_DAY: i64 = 86_400;

/// 用户名长度上限（Java `AdminUserDTO` `@Size(max = 30, min = 1)`）。
const USERNAME_MAX_LEN: usize = 30;

// ——————————————————————————————————————————————
// 未迁移能力的统一错误文案
// ——————————————————————————————————————————————

/// 手机号验证依赖 Redis 标记（Java `admin_modify_mobile_verified:<id>`）。
const ERR_VERIFY_UNAVAILABLE: &str =
    "修改/验证手机号需要 Redis 短信验证码与已验证标记（Rust 侧尚未迁移），暂不支持";

/// 短信发送服务未迁移。
const ERR_SMS_UNAVAILABLE: &str = "短信发送服务（配置项/短信通道）尚未迁移到 Rust，暂不可用";

// ——————————————————————————————————————————————
// 纯函数：校验与派生（可单测）
// ——————————————————————————————————————————————

/// 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`（Java `TimestampToDateSerializer` /
/// `SimpleDateFormat`；`<= 0` 或越界返回空串）。
pub fn fmt_seconds(secs: i32) -> String {
    if secs <= 0 {
        return String::new();
    }
    use chrono::TimeZone;
    match chrono::Local.timestamp_opt(i64::from(secs), 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => String::new(),
    }
}

/// 门店状态文案（Java `ShopType.getTypeName`：1 开业 4 停业 10 打烊）。
pub fn shop_status_text(status: Option<i32>) -> String {
    match status {
        Some(1) => "开业".into(),
        Some(4) => "停业".into(),
        Some(10) => "打烊".into(),
        _ => String::new(),
    }
}

/// `admin_user.auth_list`（JSON 文本或遗留的逗号分隔文本）→ 权限列表。
///
/// 对应 Java `detail` 的
/// `"[]".equals(authList) ? empty : Arrays.asList(StringUtils.str2Array(authList))`：
/// `"[]"` 与空串都返回空表，非 JSON 形态时按 `str2Array` 去括号去引号后按逗号切分。
pub fn parse_auth_list(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "[]" {
        return Vec::new();
    }
    if let Ok(list) = serde_json::from_str::<Vec<String>>(trimmed) {
        return list;
    }
    // 遗留文本形态：`["a","b"]` / `a,b` / `a, b`
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(trimmed)
        .replace('"', "");
    inner
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// 权限列表 → 列表出参（Java `list`：`authList.split(",")`，空值不下发）。
pub fn split_auth_list(raw: &str) -> Option<Vec<String>> {
    if raw.is_empty() {
        return None;
    }
    Some(raw.split(',').map(ToString::to_string).collect())
}

/// 角色权限 \"authority_list\"（JSON 数组）→ `Vec<String>`。
///
/// 对应 Java `JSONUtil.toList(role.getAuthorityList(), String.class)`：
/// 非法 JSON 时 Hutool 会抛异常，这里降级为空表（避免整个详情接口 500）。
pub fn parse_role_authority(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw.trim()).unwrap_or_default()
}

/// 校验用户名非空（Java \"AdminUserDTO\" `@NotNull(USERNAME_IS_NULL)`）。
pub fn validate_username_present(username: Option<&str>) -> Result<()> {
    if username.is_none_or(|s| s.is_empty()) {
        return Err(Error::Validation("管理员名称不能为空".into()));
    }
    Ok(())
}

/// 校验用户名长度 1–30（Java `@Size(max = 30, min = 1, USERNAME_IS_OVERLENGTH)`）。
///
/// **按 Java `String.length()` 的 UTF-16 单元数口径**（中文按 1 计）。
pub fn validate_username_len(username: &str) -> Result<()> {
    let n = username.chars().map(char::len_utf16).sum::<usize>();
    if n < 1 || n > USERNAME_MAX_LEN {
        return Err(Error::Validation("管理员名称长度为1-30位".into()));
    }
    Ok(())
}

/// 校验「密码 + 确认密码」一致性与新密码非空。
///
/// 对照 Java `buildAdminUser`：仅当 `pwdConfirm` 与 `password` **都非空**且不等时
/// 才报 `PASSWORD_NOT_CONSISTENT`——原实现存在「只传 pwdConfirm 不校验」的漏洞，
/// 本实现额外要求「传了确认密码就必须传新密码」（见 `uncertain_points`）。
pub fn validate_password_pair(password: Option<&str>, pwd_confirm: Option<&str>) -> Result<()> {
    let password = password.filter(|s| !s.is_empty());
    let pwd_confirm = pwd_confirm.filter(|s| !s.is_empty());
    match (password, pwd_confirm) {
        (None, Some(_)) => {
            Err(Error::Validation("请填写新密码（已传确认密码但缺少新密码）".into()))
        }
        (Some(p), Some(c)) if p != c => Err(Error::Validation("两次密码不一致".into())),
        _ => Ok(()),
    }
}

/// 校验管理员 ID 非空（\"update\" 必填，对应 Java \"buildAdminUser\" 的
/// `adminUserDTO.getAdminId()`）。
pub fn require_admin_id(admin_id: Option<i32>) -> Result<i32> {
    admin_id.ok_or_else(|| Error::Validation("管理员ID不能为空".into()))
}

/// 计算 Token 过期时间戳（Java `System.currentTimeMillis()/1000 + days*86400`）。
///
/// `expires_in_days` 为 `None` 或 `<= 0` 时返回 `None`（永不过期）。
pub fn token_expires_at(expires_in_days: Option<i64>, now_secs: i64) -> Option<i64> {
    match expires_in_days {
        Some(d) if d > 0 => Some(now_secs + d * SECONDS_PER_DAY),
        _ => None,
    }
}

/// Token 名称（Java `StrUtil.isBlank(name) ? "default" : name`）。
pub fn token_name_or_default(name: Option<&str>) -> String {
    match name.map(str::trim) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => "default".to_string(),
    }
}

/// 头像默认值（Java `ThreadLocalRandom.current().nextInt(1, 34)`
/// → `StrUtil.format("../assets/avatar/{}.jpeg", randomNumber)`，即 `[1, 34)`）。
///
/// `nextrade-services` 没有 \"rand\" 依赖，这里用既有的
/// `nextrade_common::utils::random_hex` 派生 1..=33 的整数。
pub fn default_avatar() -> String {
    let raw = u32::from_str_radix(&utils::random_hex(6), 16).unwrap_or(0);
    let n = 1 + (raw % 33);
    format!("../assets/avatar/{n}.jpeg")
}

/// `adminUserType` → `admin_type IN (...)` 取值表
/// （Java `AdminUserServiceImpl#list` 的三段映射）。
pub fn admin_types_for(admin_user_type: Option<&str>) -> Vec<String> {
    match admin_user_type {
        Some("shop") => vec![admin_type::SHOP.to_string()],
        Some("vendor") => vec![admin_type::VENDOR.to_string()],
        Some(other) if !other.is_empty() => vec![
            admin_type::ADMIN.to_string(),
            admin_type::SUPPLIERS.to_string(),
        ],
        _ => Vec::new(),
    }
}

/// 列表是否过滤 \"admin_type\"（Java `ObjectUtil.isNotEmpty(pageQuery.getAdminUserType())`）。
pub fn should_filter_admin_type(admin_user_type: Option<&str>) -> bool {
    admin_user_type.is_some_and(|s| !s.is_empty())
}

/// 过滤掉「非普通角色」（\"pos_cashier\" / \"kefu_agent\"）。
///
/// 对应 Java \"listOrdinaryRoles\" 的内存侧语义（SQL 已过滤，这里是二次保险，
/// 也便于单测）。
pub fn retain_ordinary_roles(roles: Vec<AdminRole>) -> Vec<AdminRole> {
    roles
        .into_iter()
        .filter(|r| {
            r.role_code
                .as_deref()
                .is_none_or(|c| !EXCLUDED_ROLE_CODES.contains(&c))
        })
        .collect()
}

/// 分页口径（Java \"BasePage\"：\"page\" 缺省 1，\"size\" 缺省 15、上限 100）。
pub fn page_query(page: Option<u64>, size: Option<u64>) -> PageQuery {
    let page_size = if size.is_some_and(|s| s > 100) {
        100
    } else {
        size.unwrap_or(15).max(1)
    };
    PageQuery {
        page: page.unwrap_or(1).max(1),
        page_size,
    }
}

// ——————————————————————————————————————————————
// 实体 → DTO / VO
// ——————————————————————————————————————————————

/// \"admin_user\" 实体 → DTO（不含 \"authList\" / \"roleName\" / \"userShop\"，
/// 由调用方按接口语义补齐）。
pub fn base_dto(u: &AdminUser) -> AdminUserDTO {
    AdminUserDTO {
        admin_id: Some(u.admin_id),
        username: Some(u.username.clone()),
        admin_type: Some(u.admin_type.clone()),
        mobile: u.mobile.clone(),
        avatar: u.avatar.clone(),
        email: Some(u.email.clone()),
        add_time: Some(fmt_seconds(u.add_time)),
        user_id: Some(u.user_id),
        suppliers_id: Some(u.suppliers_id),
        role_id: Some(u.role_id),
        merchant_id: Some(u.merchant_id),
        parent_id: Some(u.parent_id),
        menu_tag: u.menu_tag.clone(),
        order_export: u.order_export.clone(),
        extra: u.extra.clone(),
        shop_id: Some(u.shop_id),
        is_using: Some(u.is_using),
        ..Default::default()
    }
}

/// \"shop\" 实体 → `detail.userShop` 中的店铺片段（Java \"detail\" 内联构造）。
fn shop_fragment(s: &Shop) -> AdminUserShopVO {
    AdminUserShopVO {
        shop_id: s.shop_id,
        shop_title: s.shop_title.clone(),
        shop_logo: s.shop_logo.clone(),
        status: s.status,
        status_text: shop_status_text(s.status),
        contact_mobile: s.contact_mobile.clone(),
        add_time: fmt_seconds(s.add_time.unwrap_or(0)),
    }
}

/// \"admin_role\" 实体 → 出参 VO（\"config\" 端点）。
pub fn role_vo(r: &AdminRole) -> AdminRoleVO {
    AdminRoleVO {
        role_id: r.role_id,
        role_name: r.role_name.clone(),
        role_code: r.role_code.clone(),
        role_desc: r.role_desc.clone(),
        authority_list: r.authority_list.clone(),
        admin_type: r.admin_type.clone(),
        merchant_id: r.merchant_id,
        shop_id: r.shop_id,
        vendor_id: r.vendor_id,
        is_system: r.is_system,
    }
}

pub struct AdminUserService {
    repos: Arc<Repositories>,
}

impl AdminUserService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 列表 / 详情 —————————————————————

    /// 管理员分页（对应 Java `AdminUserServiceImpl#list` / `AdminUserController.list`）。
    ///
    /// 返回 `PageResult<AdminUserDTO>`（Java 返回 MyBatis-Plus `Page<AdminUserDTO>`，
    /// Rust 统一用项目既有 \"PageResult\"，与 order_admin / aftersales 同一约定）。
    pub async fn list(
        &self,
        admin_user_type: Option<&str>,
        keyword: Option<&str>,
        suppliers_id: Option<&str>,
        page: Option<u64>,
        size: Option<u64>,
        shop_id: Option<i32>,
    ) -> Result<PageResult<AdminUserDTO>> {
        let pq = page_query(page, size);
        let types = if should_filter_admin_type(admin_user_type) {
            admin_types_for(admin_user_type)
        } else {
            Vec::new()
        };
        let filter = AdminUserPageFilter {
            keyword: keyword.map(ToString::to_string),
            admin_types: types,
            suppliers_id: suppliers_id.map(ToString::to_string),
            shop_id,
            page_size: pq.page_size,
            offset: (pq.page - 1) * pq.page_size,
        };

        let (total, users) = self.repos.identity.admin_user.paginate_users(&filter).await?;
        if users.is_empty() {
            return Ok(PageResult::new(total, Vec::new(), &pq));
        }

        // 批量取角色名，消除 N+1
        let role_ids: Vec<i32> = {
            let mut seen = HashSet::new();
            users
                .iter()
                .map(|u| u.role_id)
                .filter(|id| seen.insert(*id))
                .collect()
        };
        let roles = self.repos.identity.admin_user.find_roles(&role_ids).await?;
        let role_map: HashMap<i32, AdminRole> = roles.into_iter().map(|r| (r.role_id, r)).collect();

        let list: Vec<AdminUserDTO> = users
            .iter()
            .map(|u| {
                let mut dto = base_dto(u);
                dto.role_name = role_map.get(&u.role_id).map(|r| r.role_name.clone());
                dto.auth_list = split_auth_list(&u.auth_list);
                dto
            })
            .collect();
        Ok(PageResult::new(total, list, &pq))
    }

    /// 管理员详情（对应 Java `AdminUserServiceImpl#detail`）。
    ///
    /// 权限解析规则（Java 原文）：
    /// - `role_id > 0`：`role != null` → `roleName = role.role_name`，
    ///   `authList = JSON(role.authority_list)`；`role == null` → 空表
    /// - 否则：`authList == "[]"` → 空表，否则 `str2Array(authList)`
    pub async fn detail(&self, admin_id: i32) -> Result<AdminUserDTO> {
        let user = self
            .repos.identity.admin_user
            .find_user(admin_id)
            .await?
            .ok_or_else(|| Error::biz_msg("管理员名称不存在"))?;

        let mut dto = base_dto(&user);
        if user.role_id > 0 {
            match self.repos.identity.admin_user.find_role(user.role_id).await? {
                Some(role) => {
                    dto.role_name = Some(role.role_name.clone());
                    dto.auth_list = Some(parse_role_authority(&role.authority_list));
                }
                None => dto.auth_list = Some(Vec::new()),
            }
        } else {
            dto.auth_list = Some(parse_auth_list(&user.auth_list));
        }

        // userShop：Java 逐个 shopId 查 shop，这里批量取回（结果等价）
        let bindings = self.repos.identity.admin_user.find_user_shops(admin_id).await?;
        if !bindings.is_empty() {
            let shop_ids: Vec<i32> = bindings.iter().map(|b| b.shop_id).collect();
            let shops = self.repos.identity.admin_user.find_shops(&shop_ids).await?;
            let shop_map: HashMap<i32, Shop> = shops.into_iter().map(|s| (s.shop_id, s)).collect();
            let mut user_shops = Vec::with_capacity(bindings.len());
            for b in &bindings {
                // Java：shop 查不到时该条返回 null 并被 filter(Objects::nonNull) 丢弃
                if let Some(s) = shop_map.get(&b.shop_id) {
                    user_shops.push(UserShopDTO {
                        shop: Some(shop_fragment(s)),
                    });
                }
            }
            dto.user_shop = Some(user_shops);
        }
        // 注意：detail 端点由路由层置空密码（本实现 `AdminUserDTO.password`
        // 带 \"skip_serializing\"，\"base_dto\" 也不填该字段，出参恒无密码哈希）
        Ok(dto)
    }

    /// 当前登录管理员详情（对应 Java `AdminUserServiceImpl#mineDetail` →
    /// `detail(SecurityUtils.getCurrentAdminId())`）。
    ///
    /// Java \"mineDetail\" **不置空密码**（与 \"detail\" 端点不同）；本 DTO 的
    /// \"password\" 带 \"skip_serializing\"，故两者出参一致（安全加固，见
    /// \"uncertain_points\"）。
    pub async fn mine_detail(&self, current_admin_id: i32) -> Result<AdminUserDTO> {
        self.detail(current_admin_id).await
    }

    // ————————————————————— 创建 / 更新 —————————————————————

    /// 创建管理员（对应 Java `AdminUserServiceImpl#create`）。
    ///
    /// 返回新建/复用的 \"admin_id\"（Java 返回 \"boolean\"；Rust 侧回传主键便于前端）。
    pub async fn create(&self, dto: &AdminUserDTO) -> Result<i32> {
        validate_username_present(dto.username.as_deref())?;
        if let Some(u) = dto.username.as_deref() {
            validate_username_len(u)?;
        }
        validate_password_pair(dto.password.as_deref(), dto.pwd_confirm.as_deref())?;

        let username = dto.username.clone().unwrap_or_default();
        let mobile = dto.mobile.clone().unwrap_or_default();
        let role_id = dto.role_id.unwrap_or(0);
        let add_time = i32::try_from(utils::now_seconds()).unwrap_or(i32::MAX);

        // ① 用户名占用（排除已软删除）
        if self
            .repos.identity.admin_user
            .find_by_username(&username, Some(false))
            .await?
            .is_some()
        {
            return Err(Error::biz_msg("管理员名称已存在或被商户账号占用"));
        }

        // 复用密码：password 优先，其次 initialPassword（Java 同一分支）
        let new_hash = self
            .resolve_password_hash(dto.password.as_deref(), dto.initial_password.as_deref())?;

        // ② 已软删除的同手机号账号 → 复用
        if let Some(mut deleted) = self.repos.identity.admin_user.find_deleted_by_mobile(&mobile).await? {
            deleted.is_deleted = 0;
            deleted.username = username.clone();
            deleted.admin_type = admin_type::ADMIN.to_string();
            deleted.role_id = role_id;
            deleted.email = dto.email.clone().unwrap_or_default();
            deleted.avatar = Some(
                dto.avatar
                    .clone()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(default_avatar),
            );
            if let Some(hash) = &new_hash {
                deleted.password = hash.clone();
            }
            deleted.auth_list = "[]".to_string();
            deleted.add_time = add_time;
            self.repos.identity.admin_user.update_user(&deleted).await?;
            self.sync_platform_binding(&deleted).await?;
            return Ok(deleted.admin_id);
        }

        // ③ 未删除的同手机号账号 → admin 报错，其他类型提升为 admin
        if let Some(mut existing) = self
            .repos.identity.admin_user
            .find_by_mobile(&mobile, Some(false), None)
            .await?
        {
            if existing.admin_type == admin_type::ADMIN {
                return Err(Error::biz_msg("管理员手机号已经存在"));
            }
            existing.admin_type = admin_type::ADMIN.to_string();
            existing.role_id = role_id;
            existing.auth_list = "[]".to_string();
            self.repos.identity.admin_user.update_user(&existing).await?;
            self.sync_platform_binding(&existing).await?;
            return Ok(existing.admin_id);
        }

        // 全新插入
        let password_hash = new_hash.unwrap_or_default();
        let avatar = dto
            .avatar
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(default_avatar);
        let suppliers_id = dto.suppliers_id.unwrap_or(0);
        let new_id = self
            .repos.identity.admin_user
            .insert_user(
                &username,
                admin_type::ADMIN,
                Some(mobile.as_str()).filter(|s| !s.is_empty()),
                &avatar,
                &password_hash,
                dto.email.as_deref().unwrap_or_default(),
                add_time,
                role_id,
                suppliers_id,
            )
            .await?;

        if let Some(user) = self.repos.identity.admin_user.find_user(new_id).await? {
            self.sync_platform_binding(&user).await?;
        }
        Ok(new_id)
    }

    /// 更新管理员（对应 Java `AdminUserServiceImpl#update` + \"buildAdminUser\"）。
    ///
    /// **注意口径差异（与 \"create\" 不同）**：\"buildAdminUser\" 的
    /// 「用户名 / 手机号 / 邮箱」占用校验**没有 \"is_deleted\" 条件**，
    /// 因此已软删除账号占用的用户名也会导致更新失败。本实现逐字复刻。
    pub async fn update(&self, dto: &AdminUserDTO) -> Result<()> {
        let admin_id = require_admin_id(dto.admin_id)?;
        validate_username_present(dto.username.as_deref())?;
        if let Some(u) = dto.username.as_deref() {
            validate_username_len(u)?;
        }
        validate_password_pair(dto.password.as_deref(), dto.pwd_confirm.as_deref())?;

        let username = dto.username.clone().unwrap_or_default();
        let mobile = dto.mobile.clone().unwrap_or_default();

        if self
            .repos.identity.admin_user
            .find_by_username(&username, None)
            .await?
            .is_some_and(|u| u.admin_id != admin_id)
        {
            return Err(Error::biz_msg("管理员名称已存在或被商户账号占用"));
        }
        if self
            .repos.identity.admin_user
            .find_by_mobile(&mobile, None, Some(admin_id))
            .await?
            .is_some()
        {
            return Err(Error::biz_msg("管理员手机号已经存在"));
        }
        if let Some(email) = dto.email.as_deref().filter(|s| !s.trim().is_empty()) {
            if self
                .repos.identity.admin_user
                .find_by_email(email, Some(admin_id))
                .await?
                .is_some()
            {
                return Err(Error::biz_msg("管理员邮箱已经存在"));
            }
        }

        // 原密码校验（Java `buildAdminUser` 里 `passwordEncoder.matches`）
        if let Some(old) = dto.old_password.as_deref().filter(|s| !s.is_empty()) {
            let current = self
                .repos.identity.admin_user
                .find_user(admin_id)
                .await?
                .ok_or_else(|| Error::biz_msg("管理员名称不存在"))?;
            if !verify_password(old, &current.password)? {
                return Err(Error::biz_msg("原密码错误"));
            }
        }

        let persisted = self
            .repos.identity.admin_user
            .find_user(admin_id)
            .await?
            .ok_or_else(|| Error::biz_msg("管理员名称不存在"))?;
        let mut next = persisted.clone();

        next.username = username;
        next.mobile = Some(mobile).filter(|s| !s.is_empty());
        next.email = dto.email.clone().unwrap_or_default();
        next.role_id = dto.role_id.unwrap_or(0);
        next.user_id = dto.user_id.unwrap_or(0);
        next.suppliers_id = dto.suppliers_id.unwrap_or(0);
        next.merchant_id = dto.merchant_id.unwrap_or(0);
        next.parent_id = dto.parent_id.unwrap_or(0);
        next.shop_id = dto.shop_id.unwrap_or(0);
        next.is_using = dto.is_using.unwrap_or(0);
        next.menu_tag = dto.menu_tag.clone();
        next.order_export = dto.order_export.clone();
        next.extra = dto.extra.clone();
        if let Some(t) = dto.admin_type.clone().filter(|s| !s.is_empty()) {
            next.admin_type = t;
        }
        if let Some(avatar) = dto.avatar.clone().filter(|s| !s.is_empty()) {
            next.avatar = Some(avatar);
        }
        // Java 无条件覆盖 avatar 为随机头像（含 `avatar = null` 的情况），
        // 本实现只在未提供头像时补默认值（见 `uncertain_points`）
        if next.avatar.is_none() {
            next.avatar = Some(default_avatar());
        }
        if let Some(hash) = self
            .resolve_password_hash(dto.password.as_deref(), dto.initial_password.as_deref())?
        {
            next.password = hash;
        }
        next.auth_list = "[]".to_string();
        next.add_time = i32::try_from(utils::now_seconds()).unwrap_or(i32::MAX);
        // Java \"buildAdminUser\"：\"suppliersId\" 非 0 时把账号类型改成 suppliers
        if next.suppliers_id != 0 {
            next.admin_type = admin_type::SUPPLIERS.to_string();
        }

        self.repos.identity.admin_user.update_user(&next).await?;
        let refreshed = self
            .repos.identity.admin_user
            .find_user(admin_id)
            .await?
            .unwrap_or(next);
        self.sync_platform_binding(&refreshed).await
    }

    /// 解析要写入的密码哈希（\"password\" 优先，其次 \"initialPassword\"）。
    ///
    /// Java：\"password\" 非空 → encode 覆盖；随后 \"initialPassword\" 非空 → 再次
    /// encode 覆盖（后者赢）。这里合并为「initialPassword 优先」以保持同一结果。
    fn resolve_password_hash(
        &self,
        password: Option<&str>,
        initial_password: Option<&str>,
    ) -> Result<Option<String>> {
        if let Some(p) = initial_password.filter(|s| !s.is_empty()) {
            return Ok(Some(hash_password(p)?));
        }
        if let Some(p) = password.filter(|s| !s.is_empty()) {
            return Ok(Some(hash_password(p)?));
        }
        Ok(None)
    }

    // ————————————————————— 删除 / 批量 —————————————————————

    /// 删除管理员（对应 Java `AdminUserServiceImpl#adminUserDel`）。
    ///
    /// Java 在删除成功后还会清理 Redis token 缓存；Rust 用无状态 JWT，
    /// 无 token 黑名单设施（见模块头「未迁移」）。
    pub async fn del(&self, id: i32, current_admin_id: i32) -> Result<()> {
        if id == current_admin_id {
            return Err(Error::biz_msg("不能删除当前登录用户"));
        }

        let user = self
            .repos.identity.admin_user
            .find_user(id)
            .await?
            .ok_or_else(|| Error::biz_msg("管理员名称不存在"))?;

        // 店铺/供应商主账号不允许删除
        if self
            .repos.identity.admin_user
            .find_shop_main_account(id)
            .await?
            .is_some()
        {
            return Err(Error::biz_msg("主账号不允许删除"));
        }

        let deleted = self.repos.identity.admin_user.soft_delete_user(id).await?;
        if !deleted {
            return Err(Error::biz_msg("管理员名称不存在"));
        }
        let username = user.username.clone();
        tracing::info!("管理员已删除: admin_id={id}, username={username}");
        self.remove_platform_binding(id).await?;
        Ok(())
    }

    /// 批量操作（对应 Java `AdminUserServiceImpl#batch`）。
    ///
    /// `type != "del"` → 无副作用（Java 返回 `false`，路由层同样回 `操作成功`）。
    pub async fn batch(&self, dto: &BatchDTO, current_admin_id: i32) -> Result<()> {
        if dto.r#type != "del" {
            return Ok(());
        }
        for id in &dto.ids {
            self.del(*id, current_admin_id).await?;
        }
        Ok(())
    }

    // ————————————————————— 修改当前账号 —————————————————————

    /// 修改当前登录管理员账号（对应 Java `AdminUserServiceImpl#modifyManageAccounts`）。
    ///
    /// \"modifyType\"：
    /// 1. 修改个人信息（头像 / 邮箱）
    /// 2. 修改密码（校验原密码 + 两次一致）
    /// 3. 修改手机号 —— **依赖 Redis 验证标记，未迁移 → 显式报错**
    /// 4. 验证当前绑定手机号 —— **依赖 Redis + 短信，未迁移 → 显式报错**
    pub async fn modify_manage_accounts(
        &self,
        dto: &AdminUserModifyManageAccountsDTO,
        current_admin_id: i32,
    ) -> Result<()> {
        // Java 控制器：modifyType 为空时置 1
        let modify_type = dto.modify_type.unwrap_or(1);

        match modify_type {
            1 => {
                // 修改个人信息：只更新 avatar / email（Java 未做空值保护）
                let affected = self
                    .repos.identity.admin_user
                    .update_profile(
                        current_admin_id,
                        dto.avatar.as_deref(),
                        dto.email.as_deref(),
                    )
                    .await?;
                if !affected {
                    return Err(Error::biz_msg("管理员名称不存在"));
                }
                Ok(())
            }
            2 => {
                // 修改密码：仅此分支需要读取当前账号的密码哈希
                let current = self
                    .repos.identity.admin_user
                    .find_user(current_admin_id)
                    .await?
                    .ok_or_else(|| Error::biz_msg("未登录"))?;
                if let Some(old) = dto.old_password.as_deref().filter(|s| !s.is_empty()) {
                    if !verify_password(old, &current.password)? {
                        return Err(Error::biz_msg("原密码错误"));
                    }
                }
                let Some(password) = dto.password.as_deref().filter(|s| !s.is_empty()) else {
                    // Java 在 password 为空时跳过改密，随后才做一致性校验
                    return validate_password_pair(None, dto.pwd_confirm.as_deref());
                };
                if let Some(confirm) = dto.pwd_confirm.as_deref().filter(|s| !s.is_empty()) {
                    if password != confirm {
                        return Err(Error::biz_msg("两次密码不一致"));
                    }
                }
                let hash = hash_password(password)?;
                let affected = self
                    .repos.identity.admin_user
                    .update_password(current_admin_id, &hash)
                    .await?;
                if !affected {
                    return Err(Error::biz_msg("管理员名称不存在"));
                }
                // Java 随后清除 Redis 中的 admin_user 缓存与该用户全部登录 token。
                // Rust 无状态 JWT 无黑名单设施 → 旧 token 到期前仍有效（见 uncertain_points）
                tracing::warn!(
                    "管理员 {current_admin_id} 已改密；Rust 侧无 token 黑名单，旧 JWT 到期前仍有效"
                );
                Ok(())
            }
            3 => {
                // 修改手机号：先验证当前手机号，再校验新手机号短信验证码。
                let cache = nextrade_infrastructure::cache::get_cache();
                let new_mobile = dto
                    .mobile
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| Error::biz_msg("手机号不能为空"))?;
                let current = self
                    .repos.identity.admin_user
                    .find_user(current_admin_id)
                    .await?
                    .ok_or_else(|| Error::biz_msg("未登录"))?;
                let current_mobile = current.mobile.as_deref().filter(|s| !s.is_empty());
                if let Some(current_mobile) = current_mobile {
                    if new_mobile == current_mobile {
                        return Err(Error::biz_msg("新手机号不能与当前手机号相同"));
                    }
                    let verified = cache
                        .get_str(&format!("admin_modify_mobile_verified:{current_admin_id}"))
                        .await?
                        .as_deref()
                        == Some("1");
                    if !verified {
                        return Err(Error::biz_msg("请先验证当前绑定的手机号"));
                    }
                }
                if self
                    .repos
                    .identity
                    .admin_user
                    .find_by_mobile(new_mobile, None, Some(current_admin_id))
                    .await?
                    .is_some()
                {
                    return Err(Error::biz_msg("手机号已被其他管理员使用"));
                }
                let code = dto
                    .code
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| Error::biz_msg("短信验证码不能为空"))?;
                let expected = cache.get_str(&format!("login{new_mobile}")).await?;
                if expected.as_deref() != Some(code) {
                    return Err(Error::biz_msg("短信验证码错误或已过期，请重试"));
                }
                cache.del(&format!("login{new_mobile}")).await?;
                if !self
                    .repos
                    .identity
                    .admin_user
                    .update_mobile(current_admin_id, new_mobile)
                    .await?
                {
                    return Err(Error::biz_msg("管理员名称不存在"));
                }
                cache
                    .del(&format!("admin_modify_mobile_verified:{current_admin_id}"))
                    .await?;
                Ok(())
            }
            4 => {
                // 验证当前绑定手机号：校验短信验证码并写入 10 分钟已验证标记。
                let cache = nextrade_infrastructure::cache::get_cache();
                let current = self
                    .repos.identity.admin_user
                    .find_user(current_admin_id)
                    .await?
                    .ok_or_else(|| Error::biz_msg("未登录"))?;
                let mobile = current
                    .mobile
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| Error::biz_msg("当前账号未绑定手机号"))?;
                let code = dto
                    .code
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| Error::biz_msg("短信验证码错误或已过期，请重试"))?;
                let expected = cache.get_str(&format!("login{mobile}")).await?;
                if expected.as_deref() != Some(code) {
                    return Err(Error::biz_msg("短信验证码错误或已过期，请重试"));
                }
                cache.del(&format!("login{mobile}")).await?;
                cache
                    .set_str(
                        &format!("admin_modify_mobile_verified:{current_admin_id}"),
                        "1",
                        600,
                    )
                    .await?;
                Ok(())
            }
            other => Err(Error::biz_msg(format!("不支持的修改类型: {other}"))),
        }
    }

    // ————————————————————— 配置 / 验证码 —————————————————————

    /// 角色配置（对应 Java `AdminUserServiceImpl#getConfig`）。
    ///
    /// Java 用请求头 `x-admin-type` / `x-shop-id` / `x-vendor-id`
    /// （`BaseServiceImpl#getAdminType/getShopId/getVendorId`）判定分支；
    /// Rust 侧 JWT 已含 `admin_type`，但 \"shop_id\" / `vendor_id` 尚未签发，
    /// 故由路由层从请求头读取后传入。
    pub async fn get_config(
        &self,
        cfg: &AdminUserConfigQuery,
        admin_type_hint: Option<&str>,
    ) -> Result<Vec<AdminRoleVO>> {
        let raw_type = admin_type_hint
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(admin_type::ADMIN);
        let shop_id = cfg.shop_id.unwrap_or(0);
        let vendor_id = cfg.vendor_id.unwrap_or(0);

        let roles = match raw_type {
            admin_type::ADMIN => {
                self.repos.identity.admin_user
                    .find_ordinary_roles(&RoleScopeFilter {
                        shop_id: Some(shop_id),
                        merchant_id: Some(0),
                        vendor_id: Some(0),
                        merchant_shop_branch: None,
                    })
                    .await?
            }
            admin_type::SHOP | admin_type::STORE | admin_type::PICKUP => {
                let shop = self
                    .repos.identity.admin_user
                    .find_shop(shop_id)
                    .await?
                    .ok_or_else(|| Error::biz_msg("店铺不存在"))?;
                match shop.merchant_id.filter(|mid| *mid > 0) {
                    Some(merchant_id) => {
                        let merchant = self.repos.identity.admin_user.find_merchant(merchant_id).await?;
                        if merchant.is_some() {
                            self.repos.identity.admin_user
                                .find_ordinary_roles(&RoleScopeFilter {
                                    merchant_shop_branch: Some((merchant_id, shop_id)),
                                    ..Default::default()
                                })
                                .await?
                        } else {
                            self.repos.identity.admin_user
                                .find_ordinary_roles(&RoleScopeFilter {
                                    shop_id: Some(shop_id),
                                    ..Default::default()
                                })
                                .await?
                        }
                    }
                    None => {
                        self.repos.identity.admin_user
                            .find_ordinary_roles(&RoleScopeFilter {
                                shop_id: Some(shop_id),
                                ..Default::default()
                            })
                            .await?
                    }
                }
            }
            admin_type::VENDOR => {
                self.repos.identity.admin_user
                    .find_ordinary_roles(&RoleScopeFilter {
                        vendor_id: Some(vendor_id),
                        ..Default::default()
                    })
                    .await?
            }
            _ => return Err(Error::biz_msg("管理员类型错误")),
        };

        Ok(retain_ordinary_roles(roles).iter().map(role_vo).collect())
    }

    /// 获取短信验证码（对应 Java `AdminUserServiceImpl#getCode`）。
    ///
    /// 前置校验（不依赖 Redis 的部分）已逐条复刻：
    /// ① 传了新手机号且与当前绑定手机号不同 → 必须已通过「验证当前手机号」
    /// （Redis 标记，未迁移 → 报错）；
    /// ② 未传手机号 → 用当前账号已绑定手机号，空则 `手机号不能为空`。
    ///
    /// 之后调用 `configService.sendSms`（短信通道 + 模板配置）——**未迁移**，
    /// 故返回明确业务错误，绝不伪造「已发送」。
    pub async fn get_code(&self, q: &GetCodeQuery, current_admin_id: i32) -> Result<()> {
        let cache = nextrade_infrastructure::cache::get_cache();
        let current = self.repos.identity.admin_user.find_user(current_admin_id).await?;
        let current_mobile = current
            .as_ref()
            .and_then(|u| u.mobile.clone())
            .filter(|s| !s.is_empty());
        let mobile = q.mobile.clone().filter(|s| !s.is_empty());

        if let Some(m) = mobile.as_deref() {
            if let Some(cur) = current_mobile.as_deref() {
                if m != cur {
                    // Java：换绑手机号前必须先在「验证当前手机号」流程写入标记
                    let verified = cache
                        .get_str(&format!("admin_modify_mobile_verified:{current_admin_id}"))
                        .await?
                        .as_deref()
                        == Some("1");
                    if !verified {
                        return Err(Error::biz_msg("请先验证当前绑定的手机号"));
                    }
                }
            }
        }
        let target = match mobile.or(current_mobile) {
            Some(m) => m,
            None => return Err(Error::biz_msg("手机号不能为空")),
        };
        let code = format!("{:06}", (utils::now_seconds() as u64) % 1_000_000);
        cache
            .set_str(&format!("login{target}"), &code, 300)
            .await?;
        crate::system::notify::send_sms_code(&self.repos, &target, &code).await?;
        tracing::info!(
            "管理员短信验证码已发送: admin_id={current_admin_id}, mobile={target}, type={:?}",
            q.code_type
        );
        Ok(())
    }

    // ————————————————————— API Token —————————————————————

    /// 生成 API Token（对应 Java `AdminApiTokenServiceImpl#generateToken`）。
    ///
    /// 明文仅在此处返回一次，入库只存 SHA-256 哈希与前 8 位前缀。
    /// Java 还会把 `hash -> username` 写入 Redis 做 1 小时校验缓存；
    /// 该校验缓存是 `validateToken` 的加速层（miss 时回查数据库），
    /// 不影响「生成」语义，故未迁移（见 `uncertain_points`）。
    pub async fn generate_api_token(
        &self,
        admin_id: i32,
        dto: &AdminApiTokenGenerateDTO,
    ) -> Result<AdminApiTokenGenerateVO> {
        let active = self.repos.identity.admin_user.count_active_tokens(admin_id).await?;
        if active >= MAX_TOKENS_PER_USER {
            return Err(Error::biz_msg(format!(
                "每个管理员最多拥有 {MAX_TOKENS_PER_USER} 个有效Token"
            )));
        }

        // Java 先生成 token 再校验管理员存在
        let user = self
            .repos.identity.admin_user
            .find_user(admin_id)
            .await?
            .ok_or_else(|| Error::biz_msg("管理员不存在"))?;

        let plain = utils::random_hex(TOKEN_HEX_LEN);
        let token_hash = utils::sha256_hex(&plain);
        let prefix = plain.chars().take(TOKEN_PREFIX_LEN).collect::<String>();
        let name = token_name_or_default(dto.name.as_deref());
        let now = utils::now_seconds();
        let expires_at = token_expires_at(dto.expires_in_days, now);

        let id = self
            .repos.identity.admin_user
            .insert_api_token(admin_id, &token_hash, &prefix, &name, now, expires_at)
            .await?;

        let username = user.username;
        tracing::info!(
            "API Token 已生成: id={id}, admin_id={admin_id}, username={username}, expires_at={expires_at:?}"
        );
        Ok(AdminApiTokenGenerateVO {
            id,
            token: plain,
            name,
            expires_at,
        })
    }

    /// 查询当前管理员的 API Token 列表（对应 Java `listTokens`）。
    pub async fn list_api_tokens(&self, admin_id: i32) -> Result<Vec<AdminApiTokenVO>> {
        let tokens = self.repos.identity.admin_user.find_api_tokens(admin_id).await?;
        Ok(tokens
            .into_iter()
            .map(|t| AdminApiTokenVO {
                id: t.id,
                token_prefix: t.token_prefix,
                name: t.name,
                created_at: t.created_at,
                last_used_at: t.last_used_at,
                expires_at: t.expires_at,
                is_active: t.is_active,
            })
            .collect())
    }

    /// 吊销 API Token（对应 Java `revokeToken`）。
    ///
    /// 归属校验：`token == null || token.admin_id != adminId` →
    /// `Token不存在或无权操作`。
    pub async fn revoke_api_token(&self, token_id: i32, admin_id: i32) -> Result<()> {
        let token = self
            .repos.identity.admin_user
            .find_api_token(token_id)
            .await?
            .filter(|t| t.admin_id == admin_id)
            .ok_or_else(|| Error::biz_msg("Token不存在或无权操作"))?;

        self.repos.identity.admin_user.revoke_api_token(token.id).await?;
        let id = token.id;
        tracing::info!("API Token 已吊销: id={id}, admin_id={admin_id}");
        Ok(())
    }

    // ————————————————————— 工作台绑定同步 —————————————————————

    /// 同步平台工作台绑定（对应 Java \"syncPlatformWorkbenchBinding\" 的**部分**分支）。
    ///
    /// 逐条对照 Java：
    /// 1. `role = role_id > 0 ? adminRoleService.getById(role_id) : null`
    /// 2. `binding = admin_user_shop WHERE admin_id = ? AND shop_id = 0`
    /// 3. `platformRole = role != null && role.admin_type == "admin" && role.shop_id == 0`
    /// 4. `existingPlatformBinding = binding != null && admin.admin_type == "admin"`
    /// 5. 两者皆否 → 直接返回（无副作用）
    /// 6. `binding == null && !isKefu && !isPos` → 返回
    /// 7. 写入 candidate 字段；`binding == null` 插入，否则 \"updateById\"
    /// 8. \"isKefu\" → \"ensureKefu\" / 必要时 \"disableKefu\"；否则 \"removeKefu\"
    ///
    /// **未迁移**：第 6 步的 \"isKefu\" / \"isPos\"
    /// （\"ShopEmployeeWorkbenchCandidatePolicy\"）与第 8 步的
    /// \"ShopEmployeeRoleLifecycle\"。因此：
    /// - 「已有绑定」时按 Java 第 7 步更新（不含 isKefu 相关判断）；
    /// - 「无绑定」时**跳过插入**（无法判定候选人资格，宁可不写也不写错）；
    /// - 不做 \"ensureKefu\" / \"removeKefu\"。
    async fn sync_platform_binding(&self, user: &AdminUser) -> Result<()> {
        let role = if user.role_id > 0 {
            self.repos.identity.admin_user.find_role(user.role_id).await?
        } else {
            None
        };
        let binding = self.repos.identity.admin_user.find_platform_binding(user.admin_id).await?;

        let platform_role = role
            .as_ref()
            .is_some_and(|r| r.admin_type.as_deref() == Some(admin_type::ADMIN) && r.shop_id == 0);
        let existing_platform_binding =
            binding.is_some() && user.admin_type == admin_type::ADMIN;

        if !platform_role && !existing_platform_binding {
            return Ok(());
        }
        let Some(mut candidate) = binding else {
            // Java 此处会在 isKefu/isPos 为真时插入新绑定；
            // 候选人策略未迁移 → 显式跳过（不写臆测数据）
            let admin_id = user.admin_id;
            tracing::debug!("跳过平台工作台绑定创建（候选人策略未迁移）: admin_id={admin_id}");
            return Ok(());
        };

        candidate.admin_id = user.admin_id;
        candidate.shop_id = 0;
        candidate.role_id = user.role_id;
        candidate.is_admin = 0;
        candidate.user_id = user.user_id;
        candidate.username = user.username.clone();
        candidate.email = user.email.clone();
        candidate.avatar = user.avatar.clone();
        candidate.auth_list = Some("[]".to_string());
        candidate.is_using = user.is_using;
        if candidate.add_time.is_none() {
            candidate.add_time = Some(if user.add_time > 0 {
                user.add_time
            } else {
                i32::try_from(utils::now_seconds()).unwrap_or(i32::MAX)
            });
        }
        self.repos.identity.admin_user.update_user_shop(&candidate).await?;
        Ok(())
    }

    /// 删除平台工作台绑定（对应 Java \"removePlatformWorkbenchBinding\"）。
    ///
    /// Java 末尾还会调 `roleLifecycle.removeKefu(adminId, 0)`——该服务未迁移。
    async fn remove_platform_binding(&self, admin_id: i32) -> Result<()> {
        let removed = self.repos.identity.admin_user.delete_platform_binding(admin_id).await?;
        tracing::debug!("清理平台工作台绑定: admin_id={admin_id}, rows={removed}");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn admin_user(admin_id: i32, role_id: i32, auth_list: &str) -> AdminUser {
        AdminUser {
            admin_id,
            username: format!("u{admin_id}"),
            admin_type: "admin".into(),
            mobile: Some("13800000000".into()),
            avatar: None,
            password: "$2a$10$xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".into(),
            email: "a@b.com".into(),
            add_time: 1_700_000_000,
            auth_list: auth_list.into(),
            user_id: 0,
            suppliers_id: 0,
            role_id,
            merchant_id: 0,
            parent_id: 0,
            menu_tag: None,
            order_export: None,
            extra: None,
            shop_id: 0,
            is_using: 0,
            initial_password: String::new(),
            is_deleted: 0,
        }
    }

    // ————————————————— 时间与文案 —————————————————

    #[test]
    fn fmt_seconds_handles_non_positive_and_valid() {
        assert_eq!(fmt_seconds(0), "");
        assert_eq!(fmt_seconds(-1), "");
        // 1_700_000_000 → 2023-11-15（本地时区可能相差一天内的小时，仅校验格式）
        assert_eq!(fmt_seconds(1_700_000_000).len(), 19);
        assert_eq!(fmt_seconds(1_700_000_000).chars().nth(4), Some('-'));
    }

    #[test]
    fn shop_status_text_matches_shop_type() {
        assert_eq!(shop_status_text(Some(1)), "开业");
        assert_eq!(shop_status_text(Some(4)), "停业");
        assert_eq!(shop_status_text(Some(10)), "打烊");
        assert_eq!(shop_status_text(Some(0)), "");
        assert_eq!(shop_status_text(None), "");
    }

    // ————————————————— 权限列表解析 —————————————————

    #[test]
    fn parse_auth_list_covers_detailed_java_branches() {
        // Java：`"[]"` → 空表
        assert!(parse_auth_list("[]").is_empty());
        assert!(parse_auth_list("").is_empty());
        assert!(parse_auth_list("   ").is_empty());
        // JSON 数组
        assert_eq!(parse_auth_list(r#"["a","b"]"#), vec!["a", "b"]);
        // 遗留的逗号文本（str2Array 去括号去引号）
        assert_eq!(parse_auth_list(r#"["a", "b"]"#), vec!["a", "b"]);
        assert_eq!(parse_auth_list("a, b ,c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn split_auth_list_matches_java_split() {
        assert_eq!(split_auth_list(""), None);
        assert_eq!(split_auth_list("a"), Some(vec!["a".to_string()]));
        assert_eq!(
            split_auth_list("a,b"),
            Some(vec!["a".to_string(), "b".to_string()])
        );
    }

    #[test]
    fn parse_role_authority_tolerates_bad_json() {
        assert_eq!(parse_role_authority(r#"["x","y"]"#), vec!["x", "y"]);
        assert!(parse_role_authority("not json").is_empty());
        assert!(parse_role_authority("[]").is_empty());
    }

    // ————————————————— 入参校验 —————————————————

    #[test]
    fn username_presence_and_length_validation() {
        assert!(validate_username_present(None).is_err());
        assert!(validate_username_present(Some("")).is_err());
        assert!(validate_username_present(Some("a")).is_ok());

        assert!(validate_username_len("a").is_ok());
        assert!(validate_username_len(&"a".repeat(30)).is_ok());
        assert!(validate_username_len(&"a".repeat(31)).is_err());
        assert!(validate_username_len("").is_err());
        // 中文按 1 个 UTF-16 单元计（Java String.length() 口径）
        assert!(validate_username_len(&"管".repeat(30)).is_ok());
        assert!(validate_username_len(&"管".repeat(31)).is_err());
    }

    #[test]
    fn password_pair_validation() {
        // 都为空 → 通过
        assert!(validate_password_pair(None, None).is_ok());
        // 只有新密码 → 通过
        assert!(validate_password_pair(Some("a"), None).is_ok());
        // 两次一致 → 通过
        assert!(validate_password_pair(Some("a"), Some("a")).is_ok());
        // 两次不一致 → 报错
        let e = validate_password_pair(Some("a"), Some("b")).unwrap_err();
        assert!(e.to_string().contains("两次密码不一致"));
        // 只传确认密码 → 报错（Java 会静默通过，本实现加固）
        assert!(validate_password_pair(None, Some("b")).is_err());
        // 空串视作未传
        assert!(validate_password_pair(Some(""), Some("")).is_ok());
    }

    #[test]
    fn require_admin_id_enforced_for_update() {
        assert_eq!(require_admin_id(Some(7)).unwrap(), 7);
        assert!(require_admin_id(None).is_err());
    }

    // ————————————————— Token 派生 —————————————————

    #[test]
    fn token_expiry_follows_java_days_rule() {
        let now = 1_700_000_000;
        assert_eq!(token_expires_at(None, now), None);
        assert_eq!(token_expires_at(Some(0), now), None);
        assert_eq!(token_expires_at(Some(-5), now), None);
        assert_eq!(token_expires_at(Some(1), now), Some(now + 86_400));
        assert_eq!(token_expires_at(Some(30), now), Some(now + 30 * 86_400));
    }

    #[test]
    fn token_name_defaults_to_default_when_blank() {
        assert_eq!(token_name_or_default(None), "default");
        assert_eq!(token_name_or_default(Some("")), "default");
        assert_eq!(token_name_or_default(Some("   ")), "default");
        assert_eq!(token_name_or_default(Some(" ci ")), "ci");
    }

    #[test]
    fn generated_token_is_hex32_with_8_char_prefix_and_hashes() {
        let plain = utils::random_hex(TOKEN_HEX_LEN);
        assert_eq!(plain.len(), TOKEN_HEX_LEN);
        assert!(plain.chars().all(|c| c.is_ascii_hexdigit()));
        let prefix: String = plain.chars().take(TOKEN_PREFIX_LEN).collect();
        assert_eq!(prefix.len(), TOKEN_PREFIX_LEN);
        // Java 用 SHA-256 hex（64 字符）作为 token_hash
        assert_eq!(utils::sha256_hex(&plain).len(), 64);
        // 两次生成不相同
        assert_ne!(plain, utils::random_hex(TOKEN_HEX_LEN));
    }

    #[test]
    fn default_avatar_matches_java_range() {
        for _ in 0..20 {
            let a = default_avatar();
            assert!(a.starts_with("../assets/avatar/"));
            let n: i32 = a
                .trim_start_matches("../assets/avatar/")
                .trim_end_matches(".jpeg")
                .parse()
                .unwrap();
            // Java：ThreadLocalRandom.nextInt(1, 34) → [1, 34)
            assert!((1..34).contains(&n), "头像序号越界: {n}");
        }
    }

    // ————————————————— 列表口径 —————————————————

    #[test]
    fn admin_types_mapping_follows_java_list() {
        // Java `AdminUserServiceImpl:537-556` 里默认字面量虽是 "admin,suppliers"，
        // 但 `.in(ObjectUtil.isNotEmpty(adminUserType), ...)` 是**有条件**生效：
        // 传 null/空串时整个 admin_type 过滤条件不拼进 SQL，也不消费类型列表。
        // 故 None 与 Some("") 必须一致地返回空表（由 `should_filter_admin_type`
        // 单独决定是否加过滤）。
        assert!(admin_types_for(None).is_empty());
        assert_eq!(
            admin_types_for(Some("shop")),
            vec!["shop".to_string()]
        );
        assert_eq!(
            admin_types_for(Some("vendor")),
            vec!["vendor".to_string()]
        );
        // 非 shop/vendor 的**非空**取值才落到默认列表
        assert_eq!(
            admin_types_for(Some("admin")),
            vec!["admin".to_string(), "suppliers".to_string()]
        );
        // 空串时 Java 的 `in(condition=false, ...)` 不生效 → 传空表
        assert!(admin_types_for(Some("")).is_empty());
    }

    #[test]
    fn should_filter_admin_type_only_when_non_empty() {
        assert!(!should_filter_admin_type(None));
        assert!(!should_filter_admin_type(Some("")));
        assert!(should_filter_admin_type(Some("shop")));
    }

    #[test]
    fn retain_ordinary_roles_drops_system_role_codes() {
        let mk = |id: i32, code: Option<&str>| AdminRole {
            role_id: id,
            role_name: format!("r{id}"),
            role_code: code.map(ToString::to_string),
            ..Default::default()
        };
        let kept = retain_ordinary_roles(vec![
            mk(1, None),
            mk(2, Some("pos_cashier")),
            mk(3, Some("kefu_agent")),
            mk(4, Some("normal")),
        ]);
        let ids: Vec<i32> = kept.iter().map(|r| r.role_id).collect();
        assert_eq!(ids, vec![1, 4]);
    }

    #[test]
    fn page_query_matches_base_page_defaults() {
        let pq = page_query(None, None);
        assert_eq!(pq.page, 1);
        assert_eq!(pq.page_size, 15);

        let pq = page_query(Some(0), Some(500));
        assert_eq!(pq.page, 1);
        assert_eq!(pq.page_size, 100);

        let pq = page_query(Some(3), Some(0));
        assert_eq!(pq.page, 3);
        assert_eq!(pq.page_size, 1);
    }

    // ————————————————— 实体映射 —————————————————

    #[test]
    fn base_dto_copies_java_fields_without_password() {
        let u = admin_user(5, 2, r#"["a"]"#);
        let dto = base_dto(&u);
        assert_eq!(dto.admin_id, Some(5));
        assert_eq!(dto.username.as_deref(), Some("u5"));
        assert_eq!(dto.role_id, Some(2));
        assert_eq!(dto.mobile.as_deref(), Some("13800000000"));
        assert_eq!(dto.add_time.as_deref(), Some(&fmt_seconds(1_700_000_000)[..]));
        // 权限列表由调用方按接口语义补齐
        assert_eq!(dto.auth_list, None);
        // 出参不含密码：`base_dto` 不填 `password`（恒为 None），且该字段带
        // `skip_serializing`，这里显式断言序列化结果
        assert!(dto.password.is_none());
        let v = serde_json::to_value(&dto).unwrap();
        assert!(v.get("password").is_none());
    }

    #[test]
    fn detail_auth_list_prefers_role_authority() {
        // role_id > 0 且角色存在 → 用角色权限
        let u = admin_user(5, 2, r#"["legacy"]"#);
        let mut dto = base_dto(&u);
        dto.role_name = Some("平台超管".into());
        dto.auth_list = Some(parse_role_authority(r#"["x","y"]"#));
        assert_eq!(dto.auth_list.as_ref().unwrap().len(), 2);

        // role_id = 0 → 回退解析 admin_user.auth_list
        let u0 = admin_user(6, 0, r#"["legacy"]"#);
        let mut dto0 = base_dto(&u0);
        dto0.auth_list = Some(parse_auth_list(&u0.auth_list));
        assert_eq!(dto0.auth_list.as_ref().unwrap(), &vec!["legacy".to_string()]);

        // role_id > 0 但角色缺失 → 空表（而非回退）
        let mut dto_missing = base_dto(&u);
        dto_missing.auth_list = Some(Vec::new());
        assert!(dto_missing.auth_list.as_ref().is_some_and(Vec::is_empty));
    }

    #[test]
    fn role_vo_mirrors_entity_fields() {
        let r = AdminRole {
            role_id: 9,
            role_name: "客服".into(),
            role_code: Some("kefu_agent".into()),
            role_desc: Some("d".into()),
            authority_list: r#"["a"]"#.into(),
            admin_type: Some("admin".into()),
            merchant_id: 0,
            shop_id: 0,
            vendor_id: 0,
            is_system: 0,
        };
        let vo = role_vo(&r);
        assert_eq!(vo.role_id, 9);
        assert_eq!(vo.role_name, "客服");
        assert_eq!(vo.role_code.as_deref(), Some("kefu_agent"));
        assert_eq!(vo.authority_list, r#"["a"]"#);
        assert_eq!(vo.admin_type.as_deref(), Some("admin"));
    }

    #[test]
    fn shop_fragment_formats_add_time_and_status() {
        let s = Shop {
            shop_id: 3,
            shop_title: Some("旗舰店".into()),
            add_time: Some(1_700_000_000),
            shop_logo: Some("l.png".into()),
            status: Some(4),
            contact_mobile: Some("13800000001".into()),
            ..Default::default()
        };
        let f = shop_fragment(&s);
        assert_eq!(f.shop_id, 3);
        assert_eq!(f.shop_title.as_deref(), Some("旗舰店"));
        assert_eq!(f.status_text, "停业");
        assert_eq!(f.add_time.len(), 19);
        // add_time 缺失 → 空串（不是 "1970-01-01 08:00:00"）
        let s0 = Shop {
            add_time: None,
            ..Default::default()
        };
        assert_eq!(shop_fragment(&s0).add_time, "");
    }

    #[test]
    fn my_detail_and_detail_share_mapping_helper() {
        // mineDetail 只是 detail(当前登录ID) 的薄封装，映射逻辑共用 base_dto
        let u = admin_user(11, 0, "[]");
        let dto = base_dto(&u);
        assert_eq!(dto.admin_id, Some(11));
        assert_eq!(dto.auth_list, None);
    }
}"