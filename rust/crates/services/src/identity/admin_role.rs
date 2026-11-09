//! 后台角色域服务（identity::admin_role）。
//!
//! 构造签名固定 `pub fn new(repos: Arc<Repositories>) -> Self`
//! （与 `identity::admin_log` / `identity::authority` 一致）。
//!
//! 对应原 Java：
//! - `com.nextrade.adminapi.controller.authority.AdminRoleController`
//! - `com.nextrade.service.authority.impl.AdminRoleServiceImpl`
//!   （`extends BaseServiceImpl<AdminRoleMapper, AdminRole>`）
//! - `com.nextrade.service.common.impl.BaseServiceImpl#updateField / #batch / #buildSortOrder`
//!
//! # 租户视角（与原 Java 的差异，务必先读）
//!
//! Java 的 `getAdminType()` / `getShopId()` / `getVendorId()` 全部来自请求头
//! `x-admin-type` / `x-shop-id` / `x-vendor-id`（`HeaderUtils`）。
//! Rust 侧 `AuthUser` 只有 `admin_type`，没有店铺/供应商 ID，故 API 层按
//! 同一优先级解析后传入 [`AdminRoleScope`]：请求头 → JWT → 查询参数兜底
//! （详见 `routes/identity/admin_role.rs::resolve_scope`）。
//!
//! # 未迁移的 Java 副作用（**已知缺口**）
//!
//! `AdminRoleServiceImpl#update` 在写完 `admin_role` 后还会：
//! 1. `syncKefuCandidate(binding, existing, updated)` —— 按
//!    `ShopEmployeeWorkbenchCandidatePolicy` 重新判定客服候选，调用
//!    `ShopEmployeeRoleLifecycle#ensureKefu / #removeKefu`；
//! 2. `syncPlatformWorkbenchBindings(updated)` —— 平台角色变客服/收银候选时，
//!    为 `admin_user` 补写 `admin_user_shop(shop_id=0)` 绑定。
//!
//! 这两段依赖「店铺员工 / 客服工作台」域（`ShopEmployeeRoleLifecycle`、
//! `ShopEmployeeWorkbenchCandidatePolicy`、`im` 域客服账号），本工作区**尚未迁移**
//! （见 `merchant/shop_manage.rs` 的同类说明）。因此本实现只做角色本身的写入，
//! 不产生上述联动 —— 属于有意的、已记录的缺口，不是遗漏。
//!
//! # 与本模块业务规则的逐条对齐
//!
//! | 端点 | Java | 本实现 |
//! |------|------|--------|
//! | `list` | 按 adminType 分支拼 `LambdaQueryWrapper` + `buildSortOrder` + `page` | [`AdminRoleService::list`] |
//! | `detail` | `getAccessibleRole` → `convertToDTO` | [`AdminRoleService::detail`] |
//! | `create` | `convertToBean` → `save` | [`AdminRoleService::create`] |
//! | `update` | 系统角色保护 → `updateById` | [`AdminRoleService::update`]（联动缺口见上） |
//! | `updateField` | 系统角色保护 → `BaseServiceImpl#updateField(ROLE_FIELDS)` | [`AdminRoleService::update_field`] |
//! | `batch` | `type == "del"` → 逐个 `delRole` | [`AdminRoleService::batch`] |
//! | `del` | `delRole`（系统角色保护 + 绑定计数 + 解绑） | [`AdminRoleService::del`] |

use std::sync::Arc;

use nextrade_core::entities::AdminRole;
use nextrade_core::models::identity::admin_role::{
    authority_list_from_json, authority_list_to_json, is_role_field_allowed, role_field_to_column,
    AdminRoleDTO, AdminRoleListQuery, AdminRoleScope, AdminType, BatchDTO, OperateDTO, RoleVisibility,
    ShopScope, UpdateFieldDTO, ADMIN_TYPE_ERROR, IS_SYSTEM_YES, MERCHANT_NOT_EXIST, ROLE_NOT_EXIST,
    SERVICE_DATA_ERROR, SHOP_NOT_EXIST, SYSTEM_ROLE_CANNOT_DELETE, SYSTEM_ROLE_CANNOT_MODIFY,
    UNSELECTED_ITEM, INVALID_FIELD_VALUE,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::identity::admin_role::{
    resolve_sort, AdminRoleRepository, NewRole, RolePatch,
};
use nextrade_infrastructure::db::Repositories;

/// 后台角色域服务。
pub struct AdminRoleService {
    repos: Arc<Repositories>,
}

impl AdminRoleService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本特性仓储（扁平字段命名 `<ctx>_<feature>`；后续会机械重组为
    /// `self.repos.<ctx>.<feature>`）。
    #[must_use]
    pub fn repo(&self) -> &AdminRoleRepository {
        &self.repos.identity.admin_role
    }

    // ————————————————————— 查询 —————————————————————

    /// 角色分页列表（Java `AdminRoleServiceImpl#list`）。
    ///
    /// 租户过滤规则完全复刻 Java 的 `switch (adminTypeEnum)`：
    /// - `admin`：`shop_id = 0 AND admin_type = 'admin'`
    /// - `shop` / `store` / `pickup`：店铺存在性校验（`店铺不存在`）、
    ///   商户存在性校验（`商户不存在`），再按「商户级角色或本店角色」过滤
    /// - `vendor`：`vendor_id = 本账号 AND admin_type = 'vendor'`
    /// - `pos`：`管理员类型错误`
    ///
    /// 另外固定排除系统内置客服角色：`role_code IS NULL OR role_code <> 'kefu_agent'`。
    ///
    /// # Errors
    /// - 账号类型非法 / 店铺或商户不存在 → `Error::Biz { code: 1001, .. }`
    /// - 数据库失败 → `Error::Database`
    /// - 库中 `authority_list` 不是合法 JSON 字符串数组 → `Error::Biz { code: 1001, .. }`
    pub async fn list(
        &self,
        q: &AdminRoleListQuery,
        scope: &AdminRoleScope,
    ) -> Result<PageResult<AdminRoleDTO>> {
        let visibility = self.resolve_visibility(scope).await?;
        let sort = resolve_sort(q.sort_field.as_deref(), q.sort_order.as_deref());
        let page = q.page_query();

        let (total, rows) = self
            .repo()
            .paginate(&visibility, q.keyword.as_deref(), &sort, &page)
            .await?;

        // Java `PageUtil.convertPage(adminRolePage, adminRoles)`：行数为 0 时
        // 仍返回带分页信息（total/size/current）的空页。
        let list = rows
            .iter()
            .map(convert_to_dto)
            .collect::<Result<Vec<AdminRoleDTO>>>()?;
        Ok(PageResult::new(total, list, &page))
    }

    /// 角色详情（Java `AdminRoleServiceImpl#detail` → `getAccessibleRole`）。
    ///
    /// # Errors
    /// 角色不存在 / 越权访问 → `Error::Biz { code: 1001, msg: "角色不存在" }`。
    pub async fn detail(&self, role_id: i32, scope: &AdminRoleScope) -> Result<AdminRoleDTO> {
        let visibility = self.resolve_visibility(scope).await?;
        let role = self.get_accessible_role(Some(role_id), &visibility).await?;
        convert_to_dto(&role)
    }

    // ————————————————————— 写入 —————————————————————

    /// 新建角色（Java `AdminRoleServiceImpl#create`）。
    ///
    /// Java 的 `convertToBean` 会：
    /// 1. `BeanUtils.copyProperties(dto, entity)` —— 只拷同名且类型可赋值的字段；
    /// 2. 用**请求头**覆盖 `shopId` / `vendorId` / `adminType`；
    /// 3. `authorityList` 用 `JSONUtil.toJsonStr` 序列化（`null` → 字面量 `"null"`）；
    /// 4. `setIsSystem(null)` —— 由 DDL 默认值 0 兜底。
    ///
    /// **Java 不校验角色名唯一性**：`admin_role` 上唯一的索引是
    /// `uniq_role_code_shop_type(role_code, shop_id, admin_type)`，而新建角色的
    /// `role_code` 恒 NULL（MySQL 唯一键允许多个 NULL），因此同名角色可以重复创建。
    /// 本实现同样不做唯一性校验（任务书里"按 Java 校验"的结论是：Java 没有该校验）。
    ///
    /// 请求体里的 `adminType` / `merchantId` / `shopId` 一律**忽略**：Java 用
    /// 请求头（`getAdminType` / `getShopId` / `getVendorId`）覆盖，且
    /// `merchantId` 因 DTO 是 `String` 而根本拷不进实体（最终落 DDL 默认 0）。
    ///
    /// Rust 侧没有「店铺类型请求属性」，但 `store` / `pickup` 的 `admin_type`
    /// 由真实 `shop.shop_type` 推导（Java 在 JWT 过滤器里预置），故这里在
    /// 当前账号是 `shop` 且带合法 `shopId` 时补一次 `shop` 查询；查不到就按
    /// `shop` 落库（Java 在拿不到店铺时同样不会报错——`create` 不校验店铺存在）。
    ///
    /// # Errors
    /// - `adminType` 非法 → `Error::Biz { code: 1001, .. }`
    /// - 数据库失败（如 `authority_list` 超长、唯一键冲突）→ `Error::Database`
    pub async fn create(&self, dto: &AdminRoleDTO, scope: &AdminRoleScope) -> Result<i32> {
        let admin_type = self.write_admin_type(scope).await?;
        let role = NewRole {
            role_id: dto.role_id,
            role_name: dto.role_name.clone().unwrap_or_default(),
            role_desc: dto.role_desc.clone().unwrap_or_default(),
            authority_list: authority_list_to_json(dto.authority_list.as_deref()),
            admin_type,
            // `getShopId()` / `getVendorId()` 缺失时 Java 写入 NULL，
            // `NOT NULL DEFAULT 0` 的列最终落 0。
            shop_id: scope.shop_id.unwrap_or(0),
            vendor_id: scope.vendor_id.unwrap_or(0),
        };
        self.repo().insert(&role).await
    }

    /// 更新角色（Java `AdminRoleServiceImpl#update`）。
    ///
    /// 与 Java 一致：
    /// 1. `getAccessibleRole(dto.roleId)` —— 校验存在性与租户归属；
    /// 2. `is_system == 1` → 「系统内置角色不可修改」；
    /// 3. 租户字段（`admin_type` / `merchant_id` / `shop_id` / `vendor_id` /
    ///    `role_code` / `is_system`）一律以**库中原值**覆盖，不接受请求体篡改；
    /// 4. `authority_list` 恒被写入（DTO 未传时写 `"null"`）。
    ///
    /// ⚠️ **权限会被清空的坑（与 Java 一致）**：`update` 必须带上
    /// `authorityList`；若省略，Java 的 `JSONUtil.toJsonStr(null)` 会写字符串
    /// `"null"`，前端再查询时权限列表就变为空。本实现原样保留这一行为。
    ///
    /// ⚠️ 客服/工作台联动未迁移，见模块头「未迁移的 Java 副作用」。
    ///
    /// # Errors
    /// 角色不存在 → 1001「角色不存在」；系统内置角色 → 1001「系统内置角色不可修改」；
    /// 数据库失败 → `Error::Database`。
    pub async fn update(&self, dto: &AdminRoleDTO, scope: &AdminRoleScope) -> Result<()> {
        let visibility = self.resolve_visibility(scope).await?;
        let existing = self.get_accessible_role(dto.role_id, &visibility).await?;
        if existing.is_system == IS_SYSTEM_YES {
            return Err(Error::biz(
                SERVICE_DATA_ERROR,
                SYSTEM_ROLE_CANNOT_MODIFY,
            ));
        }
        let patch = RolePatch {
            role_name: dto.role_name.clone(),
            role_desc: dto.role_desc.clone(),
            authority_list: authority_list_to_json(dto.authority_list.as_deref()),
            admin_type: existing.admin_type.clone(),
            merchant_id: existing.merchant_id,
            shop_id: existing.shop_id,
            vendor_id: existing.vendor_id,
            role_code: existing.role_code.clone(),
            is_system: existing.is_system,
        };
        // Java 忽略 `updateById` 的布尔结果（控制器返回 void）。
        self.repo().update(existing.role_id, &patch).await?;
        Ok(())
    }

    /// 更新单个字段（Java `AdminRoleServiceImpl#updateField` →
    /// `BaseServiceImpl#updateField(updateField, ROLE_FIELDS)`）。
    ///
    /// 顺序与 Java 一致：
    /// 1. `getAccessibleRole(updateField.id)` —— `id == null` 时同样抛
    ///    「角色不存在」（Java 侧没有 `@Valid`，不会在绑定阶段拦截）；
    /// 2. 系统内置角色 → 「系统内置角色不可修改」；
    /// 3. `getColumnByPropertyName(field)` —— **先映射**，再把映射后的列名与
    ///    `ROLE_FIELDS = ["role_name"]` 比较，不在白名单 → 「field 的值有误」；
    /// 4. 执行 `UPDATE admin_role SET <column> = ? WHERE role_id = ?`。
    ///
    /// # Errors
    /// 角色不存在 / 系统内置角色 / 字段不在白名单 → 1001 业务错误；
    /// 数据库失败（如 `val` 为 null 触发 `NOT NULL` 约束）→ `Error::Database`。
    pub async fn update_field(
        &self,
        dto: &UpdateFieldDTO,
        scope: &AdminRoleScope,
    ) -> Result<bool> {
        let visibility = self.resolve_visibility(scope).await?;
        let existing = self.get_accessible_role(dto.id, &visibility).await?;
        if existing.is_system == IS_SYSTEM_YES {
            return Err(Error::biz(
                SERVICE_DATA_ERROR,
                SYSTEM_ROLE_CANNOT_MODIFY,
            ));
        }
        // Java `getColumnByPropertyName`：属性名/列名 → 列名（未知回退主键列）
        let column = role_field_to_column(dto.field.as_deref());
        if !is_role_field_allowed(column) {
            return Err(Error::biz(SERVICE_DATA_ERROR, INVALID_FIELD_VALUE));
        }
        self.repo()
            .update_field(existing.role_id, column, dto.val.as_deref())
            .await
    }

    /// 删除单个角色（Java `AdminRoleServiceImpl#delRole`）。
    ///
    /// # Errors
    /// 角色不存在 / 系统内置角色 / 仍被账号绑定 → 1001 业务错误；
    /// 数据库失败 → `Error::Database`（事务整体回滚）。
    pub async fn del(&self, dto: &OperateDTO, scope: &AdminRoleScope) -> Result<()> {
        let Some(role_id) = dto.id else {
            return Err(Error::biz(SERVICE_DATA_ERROR, ROLE_NOT_EXIST));
        };
        let visibility = self.resolve_visibility(scope).await?;
        self.delete_roles(&[role_id], &visibility).await
    }

    /// 批量操作（Java `AdminRoleServiceImpl#batch` → `BaseServiceImpl#batch`）。
    ///
    /// Java 语义：`type == "del"` 时对 `ids` 逐个调 `delRole`，否则返回 `false`
    /// （**不报错、无副作用**）；整个 batch 在一个 `@Transactional` 里，
    /// 任一角色删除失败则整批回滚。
    ///
    /// 返回是否真的执行了删除（与 Java 的布尔返回值同义）。
    ///
    /// # Errors
    /// `type == "del"` 但 `ids` 缺失 → 1001「未选择项目」（Java 因无 `@Valid`
    /// 会 NPE 成 1001，这里按注解本意给可读文案）；
    /// 其它业务/数据库错误同 [`AdminRoleService::del`]。
    pub async fn batch(&self, dto: &BatchDTO, scope: &AdminRoleScope) -> Result<bool> {
        let ids = match batch_del_ids(dto)? {
            Some(ids) => ids,
            None => return Ok(false),
        };
        let visibility = self.resolve_visibility(scope).await?;
        self.delete_roles(&ids, &visibility).await?;
        Ok(true)
    }

    // ————————————————————— 内部 —————————————————————

    /// 按 Java `AdminRoleServiceImpl#list` 的 `switch` 解析租户可见性。
    ///
    /// 需要店铺/商户信息时先查库，再交给纯函数 [`plan_visibility`] 判定
    /// （这样分支语义可脱离数据库单测）。
    async fn resolve_visibility(&self, scope: &AdminRoleScope) -> Result<RoleVisibility> {
        // 先解析类型：非法类型在查库前就报错（Java 同理）
        let admin_type = AdminType::from_code(scope.admin_type.as_deref())?;
        let shop = match admin_type {
            AdminType::Shop | AdminType::Store | AdminType::Pickup => match scope.shop_id {
                Some(shop_id) => self.repo().shop_scope(shop_id).await?,
                None => None,
            },
            _ => None,
        };
        let merchant_exists = match shop.as_ref().and_then(|s| s.merchant_id).filter(|m| *m > 0) {
            Some(merchant_id) => self.repo().merchant_exists(merchant_id).await?,
            None => false,
        };
        plan_visibility(
            scope.admin_type.as_deref(),
            scope.shop_id,
            scope.vendor_id,
            shop.as_ref(),
            merchant_exists,
        )
    }

    /// 解析 `create` 要落库的 `admin_type`（Java `convertToBean` +
    /// `HeaderUtils#getAdminType`）。
    ///
    /// - 请求头缺失 → Java 过滤器缺省 `admin`；
    /// - `shop` → 按真实 `shop.shop_type` 升级为 `store` / `pickup`；
    /// - 其它类型原样。
    async fn write_admin_type(&self, scope: &AdminRoleScope) -> Result<String> {
        let admin_type = match scope.admin_type.as_deref() {
            None => AdminType::Admin,
            Some(code) => AdminType::from_code(Some(code))?,
        };
        let resolved = if admin_type == AdminType::Shop {
            match scope.shop_id {
                Some(shop_id) => match self.repo().shop_scope(shop_id).await? {
                    Some(shop) => AdminType::from_shop_type(shop.shop_type),
                    None => AdminType::Shop,
                },
                None => AdminType::Shop,
            }
        } else {
            admin_type
        };
        Ok(resolved.code().to_string())
    }

    /// Java `getAccessibleRole`：按主键取角色并校验租户归属。
    ///
    /// `role_id == None` 与「查不到」与「越权」三种情况**都**返回同一个
    /// 「角色不存在」（Java 有意不区分，避免泄露其它租户的角色存在性）。
    async fn get_accessible_role(
        &self,
        role_id: Option<i32>,
        visibility: &RoleVisibility,
    ) -> Result<AdminRole> {
        let Some(role_id) = role_id else {
            return Err(Error::biz(SERVICE_DATA_ERROR, ROLE_NOT_EXIST));
        };
        let role = self
            .repo()
            .find_by_id(role_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, ROLE_NOT_EXIST))?;
        if !role_accessible(visibility, &role) {
            return Err(Error::biz(SERVICE_DATA_ERROR, ROLE_NOT_EXIST));
        }
        Ok(role)
    }

    /// 批量删除（单个 `del` 也走这里），整体一个事务（Java `@Transactional`）。
    ///
    /// 每个角色的校验顺序与 Java `delRole` 一致：
    /// 存在性/越权 → 系统内置保护 → 绑定计数 → 删除 → 三表解绑。
    async fn delete_roles(&self, role_ids: &[i32], visibility: &RoleVisibility) -> Result<()> {
        let repo = self.repo();
        let mut tx = repo.begin().await?;
        for role_id in role_ids {
            let role = repo
                .find_by_id_in_tx(&mut tx, *role_id)
                .await?
                .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, ROLE_NOT_EXIST))?;
            if !role_accessible(visibility, &role) {
                return Err(Error::biz(SERVICE_DATA_ERROR, ROLE_NOT_EXIST));
            }
            if role.is_system == IS_SYSTEM_YES {
                return Err(Error::biz(
                    SERVICE_DATA_ERROR,
                    SYSTEM_ROLE_CANNOT_DELETE,
                ));
            }
            // Java: `delRole` 的绑定计数一次性跨三张表累加
            let bind_count = repo.count_bindings_in_tx(&mut tx, *role_id).await?;
            if bind_count > 0 {
                return Err(Error::biz(
                    SERVICE_DATA_ERROR,
                    format!("该角色下有 {bind_count} 个账号，请先重新分配角色"),
                ));
            }
            let deleted = repo.delete_in_tx(&mut tx, *role_id).await?;
            if deleted {
                // Java: `if (del) { 三张表 role_id = 0 }`
                repo.reset_bindings_in_tx(&mut tx, *role_id).await?;
            }
        }
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }
}

/// 解析租户可见性（纯函数，便于无库单测）。
///
/// `shop` / `merchant_exists` 由调用方按需查库后传入：
/// - 非店铺类账号传 `None` / `false` 即可；
/// - 店铺类账号必须传真实 `shop` 行（`None` → 「店铺不存在」）；
/// - `shop.merchant_id > 0` 时必须传 `merchant_exists` 的真实结果
///   （`false` → 「商户不存在」）。
///
/// # Errors
/// - `adminType` 非法 → 1001「无效 adminType: …」
/// - `pos` → 1001「管理员类型错误」（Java `switch` 的 `default` 分支）
/// - 店铺/商户不存在 → 1001「店铺不存在」/「商户不存在」
pub fn plan_visibility(
    admin_type: Option<&str>,
    shop_id: Option<i32>,
    vendor_id: Option<i32>,
    shop: Option<&ShopScope>,
    merchant_exists: bool,
) -> Result<RoleVisibility> {
    let admin_type = AdminType::from_code(admin_type)?;
    match admin_type {
        // Java `HeaderUtils#getShopId()` 对 admin 恒返回 0（忽略 x-shop-id）
        AdminType::Admin => Ok(RoleVisibility::Admin { shop_id: 0 }),
        AdminType::Shop | AdminType::Store | AdminType::Pickup => {
            // Java `shopMapper.selectById(getShopId())`：id 为 null 或查不到 → 店铺不存在
            if shop_id.is_none() {
                return Err(Error::biz(SERVICE_DATA_ERROR, SHOP_NOT_EXIST));
            }
            let shop = shop.ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, SHOP_NOT_EXIST))?;
            let merchant_id = match shop.merchant_id {
                Some(merchant_id) if merchant_id > 0 => {
                    if !merchant_exists {
                        return Err(Error::biz(SERVICE_DATA_ERROR, MERCHANT_NOT_EXIST));
                    }
                    Some(merchant_id)
                }
                _ => None,
            };
            Ok(RoleVisibility::Shop {
                shop_id: shop.shop_id,
                merchant_id,
            })
        }
        // Java: `eq(vendorId, getVendorId())`；getVendorId() 为 null 时查不到行
        AdminType::Vendor => Ok(RoleVisibility::Vendor { vendor_id }),
        AdminType::Pos => Err(Error::biz(SERVICE_DATA_ERROR, ADMIN_TYPE_ERROR)),
    }
}

/// Java `AdminRoleServiceImpl#isRoleAccessible`（含 `#isShopRoleAccessible`）。
///
/// 注意平台分支还要求角色的 `admin_type == 'admin'`，供应商分支要求
/// `admin_type == 'vendor'` —— 只比 ID 会放过跨类型角色。
#[must_use]
pub fn role_accessible(visibility: &RoleVisibility, role: &AdminRole) -> bool {
    match visibility {
        RoleVisibility::Admin { shop_id } => {
            role.shop_id == *shop_id && role.admin_type.as_deref() == Some("admin")
        }
        RoleVisibility::Shop {
            shop_id,
            merchant_id,
        } => {
            // Java `isShopRoleAccessible` 的三类角色；`currentShop == null` 的
            // 情况由 `plan_visibility` 提前排除。
            let type_ok = matches!(
                role.admin_type.as_deref(),
                Some("shop") | Some("store") | Some("pickup")
            );
            if !type_ok {
                return false;
            }
            role.shop_id == *shop_id
                || (role.shop_id == 0
                    && merchant_id.is_some_and(|m| m > 0 && role.merchant_id == m))
        }
        RoleVisibility::Vendor { vendor_id } => {
            vendor_id.is_some_and(|v| role.vendor_id == v)
                && role.admin_type.as_deref() == Some("vendor")
        }
    }
}

/// 解析 `batch` 的待删 ID 列表（纯函数）。
///
/// - `type != "del"` → `Ok(None)`（Java 返回 `false`，无副作用）；
/// - `type == "del"` 且 `ids` 缺失 → 「未选择项目」；
/// - `type == "del"` 且 `ids == []` → `Ok(None)`（Java `forEach` 空集合无操作）；
/// - 其余 → `Ok(Some(ids))`。
///
/// # Errors
/// `ids` 缺失 → 1001「未选择项目」。
pub fn batch_del_ids(dto: &BatchDTO) -> Result<Option<Vec<i32>>> {
    if dto.r#type.as_deref() != Some("del") {
        return Ok(None);
    }
    let ids = dto
        .ids
        .clone()
        .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, UNSELECTED_ITEM))?;
    if ids.is_empty() {
        return Ok(None);
    }
    Ok(Some(ids))
}

/// 实体 → DTO（Java `convertToDTO`）。
///
/// `merchantId` / `shopId` 恒为 `None`：Java 的 `AdminRoleDTO` 把这两列声明成
/// `String`，实体的 `Integer` 无法被 `BeanUtils.copyProperties` 赋值，出参就是
/// `null`。见 `models/identity/admin_role.rs` 模块注释。
///
/// # Errors
/// `authority_list` 不是合法 JSON 字符串数组 → 1001 业务错误。
pub fn convert_to_dto(role: &AdminRole) -> Result<AdminRoleDTO> {
    Ok(AdminRoleDTO {
        role_id: Some(role.role_id),
        role_name: Some(role.role_name.clone()),
        role_desc: role.role_desc.clone(),
        authority_list: Some(authority_list_from_json(&role.authority_list)?),
        admin_type: role.admin_type.clone(),
        merchant_id: None,
        shop_id: None,
        is_system: Some(role.is_system),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn role(
        role_id: i32,
        admin_type: &str,
        shop_id: i32,
        merchant_id: i32,
        vendor_id: i32,
        is_system: i32,
    ) -> AdminRole {
        AdminRole {
            role_id,
            role_name: "角色".into(),
            role_code: None,
            role_desc: Some("描述".into()),
            authority_list: r#"["all"]"#.into(),
            admin_type: Some(admin_type.into()),
            merchant_id,
            shop_id,
            vendor_id,
            is_system,
        }
    }

    fn shop(shop_id: i32, merchant_id: Option<i32>, shop_type: Option<i32>) -> ShopScope {
        ShopScope {
            shop_id,
            merchant_id,
            shop_type,
        }
    }

    #[test]
    fn plan_visibility_matches_java_switch() {
        // admin：shopId 恒 0（忽略请求头里的 x-shop-id）
        assert_eq!(
            plan_visibility(Some("admin"), Some(7), Some(3), None, false).unwrap(),
            RoleVisibility::Admin { shop_id: 0 }
        );

        // shop 无商户：只看本店
        assert_eq!(
            plan_visibility(Some("shop"), Some(12), None, Some(&shop(12, None, Some(1))), false)
                .unwrap(),
            RoleVisibility::Shop {
                shop_id: 12,
                merchant_id: None
            }
        );

        // shop 有商户：商户级（shop_id = 0）或本店角色；商户必须存在
        assert_eq!(
            plan_visibility(
                Some("shop"),
                Some(12),
                None,
                Some(&shop(12, Some(5), Some(1))),
                true
            )
            .unwrap(),
            RoleVisibility::Shop {
                shop_id: 12,
                merchant_id: Some(5)
            }
        );
        let err =
            plan_visibility(Some("shop"), Some(12), None, Some(&shop(12, Some(5), Some(1))), false)
                .unwrap_err();
        assert!(err.to_string().contains("商户不存在"));

        // store / pickup 与 shop 业务分支完全相同
        assert_eq!(
            plan_visibility(Some("store"), Some(12), None, Some(&shop(12, None, Some(2))), false)
                .unwrap(),
            RoleVisibility::Shop {
                shop_id: 12,
                merchant_id: None
            }
        );

        // vendor
        assert_eq!(
            plan_visibility(Some("vendor"), None, Some(9), None, false).unwrap(),
            RoleVisibility::Vendor { vendor_id: Some(9) }
        );
        // pos → 管理员类型错误（Java switch default）
        let err = plan_visibility(Some("pos"), Some(1), None, None, false).unwrap_err();
        assert!(err.to_string().contains("管理员类型错误"));
        // 未知类型 → 无效 adminType
        let err = plan_visibility(Some("boss"), None, None, None, false).unwrap_err();
        assert!(err.to_string().contains("无效 adminType: boss"));
        // 缺失类型 → 无效 adminType: null
        let err = plan_visibility(None, None, None, None, false).unwrap_err();
        assert!(err.to_string().contains("无效 adminType: null"));
    }

    #[test]
    fn plan_visibility_requires_existing_shop_for_shop_types() {
        // Java `shopMapper.selectById(null)` / 查不到 → 「店铺不存在」
        let err = plan_visibility(Some("shop"), None, None, None, false).unwrap_err();
        assert!(err.to_string().contains("店铺不存在"));
        let err = plan_visibility(Some("shop"), Some(12), None, None, false).unwrap_err();
        assert!(err.to_string().contains("店铺不存在"));
    }

    #[test]
    fn role_accessible_matches_java_tenant_rules() {
        // —— 平台 ——
        let admin = RoleVisibility::Admin { shop_id: 0 };
        assert!(role_accessible(&admin, &role(1, "admin", 0, 0, 0, 0)));
        // 店铺角色对平台不可见（admin_type 不同）
        assert!(!role_accessible(&admin, &role(2, "shop", 0, 0, 0, 0)));
        // 别的 shop_id 的平台角色不可见
        assert!(!role_accessible(&admin, &role(3, "admin", 5, 0, 0, 0)));

        // —— 店铺 ——
        let s = RoleVisibility::Shop {
            shop_id: 12,
            merchant_id: Some(5),
        };
        assert!(role_accessible(&s, &role(4, "shop", 12, 5, 0, 0)));
        assert!(role_accessible(&s, &role(5, "store", 12, 5, 0, 0)));
        // 商户级角色：shop_id = 0 且 merchant_id 相同
        assert!(role_accessible(&s, &role(6, "shop", 0, 5, 0, 0)));
        // 商户级角色但商户不同
        assert!(!role_accessible(&s, &role(7, "shop", 0, 6, 0, 0)));
        // 别的店铺
        assert!(!role_accessible(&s, &role(8, "shop", 13, 5, 0, 0)));
        // 平台角色对店铺不可见
        assert!(!role_accessible(&s, &role(9, "admin", 0, 5, 0, 0)));
        // 供应商角色对店铺不可见
        assert!(!role_accessible(&s, &role(10, "vendor", 0, 0, 9, 0)));

        // 无商户的店铺：不允许 merchant_id = 0 的商户级角色被误判
        let s_no_merchant = RoleVisibility::Shop {
            shop_id: 12,
            merchant_id: None,
        };
        assert!(!role_accessible(&s_no_merchant, &role(11, "shop", 0, 0, 0, 0)));

        // —— 供应商 ——
        let v = RoleVisibility::Vendor {
            vendor_id: Some(9),
        };
        assert!(role_accessible(&v, &role(12, "vendor", 0, 0, 9, 0)));
        assert!(!role_accessible(&v, &role(13, "vendor", 0, 0, 8, 0)));
        assert!(!role_accessible(&v, &role(14, "shop", 0, 0, 9, 0)));
        // getVendorId() 为 null → 谁都不匹配
        assert!(!role_accessible(
            &RoleVisibility::Vendor { vendor_id: None },
            &role(15, "vendor", 0, 0, 9, 0)
        ));
    }

    #[test]
    fn convert_to_dto_matches_java_bean_copy_quirks() {
        let dto = convert_to_dto(&role(7, "shop", 12, 5, 0, 0)).unwrap();
        assert_eq!(dto.role_id, Some(7));
        assert_eq!(dto.role_name.as_deref(), Some("角色"));
        assert_eq!(dto.role_desc.as_deref(), Some("描述"));
        assert_eq!(dto.authority_list, Some(vec!["all".to_string()]));
        assert_eq!(dto.admin_type.as_deref(), Some("shop"));
        assert_eq!(dto.is_system, Some(0));
        // Java BeanUtils: Integer → String 不兼容 → 出参恒 null
        assert_eq!(dto.merchant_id, None);
        assert_eq!(dto.shop_id, None);

        // authority_list 为字面量 "null"（Java JSONUtil.toJsonStr(null) 的产物）
        let mut r = role(8, "admin", 0, 0, 0, 0);
        r.authority_list = "null".into();
        assert_eq!(convert_to_dto(&r).unwrap().authority_list, Some(vec![]));

        // 非法 JSON → 1001 业务错误（Java JSONException）
        let mut bad = role(9, "admin", 0, 0, 0, 0);
        bad.authority_list = "{oops".into();
        assert!(convert_to_dto(&bad).is_err());
    }

    #[test]
    fn batch_del_ids_matches_java_batch_semantics() {
        // 非 del 类型：Java 返回 false，无副作用
        let dto: BatchDTO = serde_json::from_str(r#"{"ids":[1,2],"type":"status"}"#).unwrap();
        assert_eq!(batch_del_ids(&dto).unwrap(), None);
        // 缺少 type：同样 false
        let dto: BatchDTO = serde_json::from_str(r#"{"ids":[1]}"#).unwrap();
        assert_eq!(batch_del_ids(&dto).unwrap(), None);
        // del + 正常 ids
        let dto: BatchDTO = serde_json::from_str(r#"{"ids":[3,4],"type":"del"}"#).unwrap();
        assert_eq!(batch_del_ids(&dto).unwrap(), Some(vec![3, 4]));
        // del + 空 ids：Java forEach 空集合 → 无操作
        let dto: BatchDTO = serde_json::from_str(r#"{"ids":[],"type":"del"}"#).unwrap();
        assert_eq!(batch_del_ids(&dto).unwrap(), None);
        // del + 缺失 ids：按 @NotEmpty 本意给「未选择项目」
        let dto: BatchDTO = serde_json::from_str(r#"{"type":"del"}"#).unwrap();
        let err = batch_del_ids(&dto).unwrap_err();
        assert!(err.to_string().contains("未选择项目"));
    }

    #[test]
    fn system_role_guards_use_java_messages() {
        // 消息常量逐字符对齐 Java 源码
        assert_eq!(SYSTEM_ROLE_CANNOT_MODIFY, "系统内置角色不可修改");
        assert_eq!(SYSTEM_ROLE_CANNOT_DELETE, "系统内置角色不可删除");
        assert_eq!(ROLE_NOT_EXIST, "角色不存在");
        assert_eq!(INVALID_FIELD_VALUE, "field 的值有误");
        // 绑定计数文案（Java 字符串拼接）
        let bind_count = 3;
        assert_eq!(
            format!("该角色下有 {bind_count} 个账号，请先重新分配角色"),
            "该角色下有 3 个账号，请先重新分配角色"
        );
        // 系统角色判定值
        assert_eq!(IS_SYSTEM_YES, 1);
    }

    #[test]
    fn update_field_whitelist_uses_mapped_column_name() {
        // ROLE_FIELDS = {"role_name"}；先映射属性名，再比对列名
        assert!(is_role_field_allowed(&role_field_to_column(Some("roleName"))));
        assert!(is_role_field_allowed(&role_field_to_column(Some("role_name"))));
        assert!(!is_role_field_allowed(&role_field_to_column(Some("roleDesc"))));
        assert!(!is_role_field_allowed(&role_field_to_column(Some("isSystem"))));
        assert!(!is_role_field_allowed(&role_field_to_column(None)));
    }
}
