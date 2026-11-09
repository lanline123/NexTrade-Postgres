//! 代客下单 · 会员域服务（trade::proxy_shopping_user）。
//!
//! 对应原 Java `com.nextrade.adminapi.controller.proxyshopping.ProxyShoppingUserController`
//! （`@RequestMapping("/adminapi/proxyShopping/user")`，
//! 类级 `@PreAuthorize("@pms.hasPermission('proxyShoppingManage')")`），
//! 两条只读链路最终都落在 `com.nextrade.service.user.UserService`：
//!
//! | 本域端点 | 原 Java 控制器方法 | 服务实现 |
//! |----------|--------------------|----------|
//! | `GET /adminapi/proxyShopping/user/list` | `ProxyShoppingUserController.list(UserListDTO)` | [`ProxyShoppingUserService::list`] |
//! | `GET /adminapi/proxyShopping/user/detail` | `ProxyShoppingUserController.detail(Integer id)` | [`ProxyShoppingUserService::detail`] |
//!
//! # 与 `/adminapi/user/user/*`（member::user_manage）的关系
//!
//! 两个控制器调用的是**同一个** `UserServiceImpl` 方法，差异只在控制器里：
//! 1. `list`：代客下单控制器在调用前按 `x-admin-type` **覆盖** `isExact`
//!    （`ProxyShoppingUserController.java:38-43`）：
//!    ```java
//!    String adminType = HeaderUtils.getAdminType();
//!    if ("admin".equals(adminType) && Objects.equals(userList.getIsExact(), true)) {
//!        userList.setIsExact(true);      // 平台号且前端要精确 → 精确
//!    } else {
//!        userList.setIsExact(!"admin".equals(adminType));  // 非平台号 → 恒精确
//!    }
//!    ```
//!    归一化后的布尔表（`A = adminType == "admin"`，`E = 前端传入的 isExact`）：
//!
//!    | A | E | 结果 |
//!    |---|---|------|
//!    | true  | `true`  | `true` |
//!    | true  | `false` / 缺省 | `false` |
//!    | false | 任意 | `true` |
//!
//!    即：**只有平台账号（`admin`）才可能走模糊匹配**；店铺 / 门店 / 供应商 /
//!    收银台等非平台账号一律强制精确匹配（无法用关键词模糊搜出别人的会员）。
//!    缺失 `x-admin-type` 时 `getAdminType()` 返回 `null` → 同样落入
//!    「非平台」分支 → 强制精确（见 [`resolve_is_exact`]）。
//! 2. `detail`：代客下单控制器**不**追加会员标签；
//!    `/adminapi/user/user/detail` 才会 `setMemberTags(...)`。因此本域
//!    [`ProxyShoppingUserService::detail`] 返回的 `memberTags` 恒为空数组（与 Java 一致）。
//! 3. `list` / `detail` 的密码脱敏：代客下单控制器额外
//!    `user.setPassword(null)`，故两个接口的 `password` 在响应里都是 `null`
//!    （而 member::user_manage 的 `UserDTO.password` 在 `convertToDTO` 里被设为 `""`）。
//!    服务层由 [`mask_password`] 清空该字段（空串），
//!    路由层再用 `MemberVO` 把它序列化为 JSON `null`。
//!
//! # 复用而非重写
//!
//! 为避免同一段 `UserServiceImpl#list` / `getUserAndUserRank` 语义出现两份
//! 会互相漂移的实现，本服务**委托** [`UserManageService`]（会员管理域服务）
//! 执行查询，自己只负责代客下单控制器的三处差异（`isExact` 覆盖、
//! `password` 置空、不加会员标签）。
//!
//! # 未迁移 / 降级（详见返回 JSON 的 `uncertain_points`）
//!
//! - **权限未迁移**：Java 类级 `@PreAuthorize("@pms.hasPermission('proxyShoppingManage')")`
//!   在 Rust 侧没有对应白名单，`require_auth` 只校验登录态（与
//!   product_manage/002/005/012/014/018 的既有缺口一致）。
//! - **`x-shop-type` 属性未迁移**：Java `HeaderUtils.getAdminType()` 对
//!   `x-admin-type: shop` 且 `x-shop-type: 2|3` 的账号返回 `store` / `pickup`
//!   而不是 `shop`。本域只关心「是否等于 `admin`」，故该细化不影响结论
//!   （`store` / `pickup` 都不是 `admin`，同样落入强制精确分支）。
//! - **默认头像配置项**（`SettingsEnum.DEFAULT_AVATAR`）未迁移，头像为空时
//!   保留空串（继承 member::user_manage 的既有降级）。
//! - **会员标签的店铺上下文**未迁移，固定按平台 `shop_id = 0`（仅 `detail`
//!   的 member::user_manage 版本用到；本域 `detail` 与 Java 一致不返回标签）。

use std::sync::Arc;

use nextrade_core::models::member::user_manage::{UserItemVO, UserListQuery, UserVO};
use nextrade_core::types::PageResult;
use nextrade_core::Result;
use nextrade_infrastructure::db::repo::trade::proxy_shopping_user::ProxyShoppingUserRepository;
use nextrade_infrastructure::db::Repositories;

use crate::member::user_manage::UserManageService;

/// Java `AdminTypeEnum` / `HeaderUtils.getAdminType()` 中的平台账号标识。
///
/// 只有该值才允许模糊匹配（见模块头的布尔表）；缺省 / 其它值
/// （`shop` / `store` / `pickup` / `pos` / `vendor`）都强制精确。
pub const ADMIN_TYPE_ADMIN: &str = "admin";

/// 代客下单会员域服务。
pub struct ProxyShoppingUserService {
    /// 共享仓储集合（保留给本域独占的只读诊断查询）。
    #[allow(dead_code)]
    repos: Arc<Repositories>,
    /// 本域仓储句柄（`user` 单表只读查询，见 [`ProxyShoppingUserService::count_users_by_keyword`]）。
    repo: ProxyShoppingUserRepository,
    /// 会员管理域服务（`UserServiceImpl` 的完整 Rust 实现）。
    ///
    /// 本域与 `/adminapi/user/user/*` 共用同一批查询，**委托**而非复制，
    /// 避免两份 `list` / `detail` SQL 随时间漂移。
    user_manage: Arc<UserManageService>,
}

impl ProxyShoppingUserService {
    /// 构造服务（`repos.trade.proxy_shopping_user` 由主会话在 `Repositories` 中注册）。
    ///
    /// 内部用**同一个** `Arc<Repositories>` 构造一份 [`UserManageService`]：
    /// 它是无状态的纯句柄集合（`repos.member.user_manage` 是 `Clone` 的连接池包装），
    /// 与 `AdminState.user_manage` 指向同一份仓储与同一个连接池，
    /// 不存在两份状态漂移的问题。
    ///
    /// 之所以不收第二个构造参数：`tools/register_slices.py` 统一按
    /// `SliceNNNService::new(repos.clone())` 生成装配代码，本片保持该签名
    /// 即可全程走自动注册，无需手工改 `bin/src/main.rs`。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        let repo = repos.trade.proxy_shopping_user.clone();
        let user_manage = Arc::new(UserManageService::new(repos.clone()));
        Self {
            repos,
            repo,
            user_manage,
        }
    }

    /// 本域仓储句柄。
    #[must_use]
    pub fn repo(&self) -> &ProxyShoppingUserRepository {
        &self.repo
    }

    // ————————————————————————— 列表 —————————————————————————

    /// 会员分页列表（对应 Java `ProxyShoppingUserController.list`）。
    ///
    /// `admin_type` 为 Java `HeaderUtils.getAdminType()` 的取值（`x-admin-type`
    /// 请求头，缺失 / 空白为 `None`）。本方法按模块头第 1 条覆盖 `isExact`
    /// 后委托 [`UserManageService::list`]，并清空每条记录的凭证字段。
    ///
    /// 返回的 `password` 为空串（`UserVO` 的 member::user_manage 约定）；
    /// **响应层的 `MemberVO` 会把它序列化为 JSON `null`**（Java `setPassword(null)`）。
    ///
    /// # Errors
    /// 数据库错误向上抛出；标签筛选无命中时返回空页（与 Java 一致，不报错）。
    pub async fn list(
        &self,
        q: &UserListQuery,
        admin_type: Option<&str>,
    ) -> Result<PageResult<UserVO>> {
        let mut effective = q.clone();
        effective.is_exact = Some(resolve_is_exact(admin_type, q.is_exact));

        let mut page = self.user_manage.list(&effective).await?;
        for vo in page.list.iter_mut() {
            mask_password(vo);
        }
        Ok(page)
    }

    /// 会员详情（对应 Java `ProxyShoppingUserController.detail`）。
    ///
    /// 与 Java 一致：`UserItemVO` 的 `memberTags` 为**空数组**
    /// （代客下单控制器不追加会员标签），凭证字段同样被清空
    /// （响应层序列化为 JSON `null`）。
    ///
    /// # Errors
    /// 会员不存在时返回业务错误（Java 对查不到的用户会 NPE，
    /// 这里改为明确报错）。
    pub async fn detail(&self, user_id: i32) -> Result<UserItemVO> {
        let mut vo = self.user_manage.detail(user_id).await?;
        vo.member_tags.clear();
        mask_password(&mut vo.user);
        Ok(vo)
    }

    /// 按 Java `UserServiceImpl#list` 的关键词语义统计命中会员数
    /// （自检 / 诊断用，见 `repo::proxy_shopping_user` 模块头）。
    ///
    /// # Errors
    /// 数据库错误向上抛出。
    pub async fn count_users_by_keyword(
        &self,
        keyword: Option<&str>,
        exact: bool,
    ) -> Result<i64> {
        self.repo.count_by_keyword(keyword, exact).await
    }

    /// 供运维排查：当前 `list` 在给定账号类型下实际使用的精确匹配开关。
    #[must_use]
    pub fn effective_is_exact(&self, admin_type: Option<&str>, raw: Option<bool>) -> bool {
        resolve_is_exact(admin_type, raw)
    }
}

// ————————————————————————— 纯逻辑 —————————————————————————

/// 代客下单 `list` 的 `isExact` 覆盖规则
/// （`ProxyShoppingUserController.java:38-43`）。
///
/// `admin_type` 为 Java `HeaderUtils.getAdminType()`；
/// `raw` 为前端传入的 `UserListDTO.isExact`（`Boolean`，可为 `null`）。
///
/// 与 Java 逐分支等价（见模块头布尔表）：
/// - `admin_type == "admin"` → 原样保留 `raw`（`null` 视为 `false`）
/// - 否则（含 `None`）→ 恒 `true`
///
/// 注意 `admin_type` 比较**区分大小写**（Java 用 `"admin".equals(...)`）；
/// 但会先 `trim`，避免 HTTP 头尾随空格（如 `"admin "`）导致语义漂移 ——
/// Java 的 `StrUtil.isEmpty` 不 trim，此处为**有意的健壮性增强**，
/// 只会把「误判为非平台 → 强制精确」变成「按平台 → 尊重前端」，
/// 见 `uncertain_points`。
#[must_use]
pub fn resolve_is_exact(admin_type: Option<&str>, raw: Option<bool>) -> bool {
    let is_admin = admin_type.map(str::trim) == Some(ADMIN_TYPE_ADMIN);
    if is_admin {
        // Java `Objects.equals(isExact, true)`：`null` / `false` 都保持 false
        raw == Some(true)
    } else {
        // Java `userList.setIsExact(!"admin".equals(adminType))` → true
        true
    }
}

/// 代客下单 `list` / `detail` 的密码脱敏
/// （`ProxyShoppingUserController.java:46-50` / `60-61` 的 `setPassword(null)`）。
///
/// ⚠️ 与 member::user_manage 的差异：`UserServiceImpl#convertToDTO` 把 `password`
/// 设为**空串** `""`，而本域控制器再把它覆盖成 `null`。因此本域必须输出
/// `null`（`Option<String> = None`）而不是 `""`。member::user_manage 的 `UserVO.password`
/// 是 `String`（空串），本域在**响应层**把它映射为 `None`。
/// 详见 `crates/api/admin/src/routes/proxy_shopping_user.rs` 的响应结构说明。
pub fn mask_password(vo: &mut UserVO) {
    vo.password.clear();
    // `UserVO.password` 是 `String`（member::user_manage 约定：恒为空串）；
    // 响应层按 Java 代客下单语义把它序列化为 `null`，见路由层 `MemberVO`。
}

#[cfg(test)]
mod tests {
    use super::*;

    // ————————————————————— isExact 覆盖规则 —————————————————————

    #[test]
    fn platform_admin_respects_frontend_flag() {
        // A = true：完全按前端传入值
        assert!(resolve_is_exact(Some("admin"), Some(true)));
        assert!(!resolve_is_exact(Some("admin"), Some(false)));
        // 前端未传（Java `null`）→ `Objects.equals(null, true)` 为 false
        assert!(!resolve_is_exact(Some("admin"), None));
    }

    #[test]
    fn non_platform_accounts_are_forced_exact() {
        // A = false：无论前端传什么（含缺省）都强制精确
        for raw in [None, Some(true), Some(false)] {
            for t in ["shop", "store", "pickup", "pos", "vendor", "suppliers"] {
                assert!(
                    resolve_is_exact(Some(t), raw),
                    "账号类型 {t} 应强制精确匹配（raw={raw:?}）"
                );
            }
        }
        // 缺省 x-admin-type：Java getAdminType() 返回 null → 非平台分支 → true
        assert!(resolve_is_exact(None, None));
        assert!(resolve_is_exact(None, Some(false)));
    }

    #[test]
    fn admin_type_comparison_is_case_sensitive_like_java() {
        // Java `"admin".equals(adminType)` 区分大小写
        assert!(resolve_is_exact(Some("Admin"), None));
        assert!(resolve_is_exact(Some("ADMIN"), Some(false)));
        // 空串不是平台账号
        assert!(resolve_is_exact(Some(""), Some(false)));
    }

    #[test]
    fn admin_type_is_trimmed() {
        // HTTP 头尾随空格不应被当成「非平台」（有意的健壮性增强，见文档）
        assert!(!resolve_is_exact(Some(" admin "), None));
        assert!(resolve_is_exact(Some("  shop  "), Some(false)));
    }

    #[test]
    fn effective_is_exact_matches_controller_branch_table() {
        // 模块头布尔表的四行逐一断言
        assert!(resolve_is_exact(Some("admin"), Some(true)));
        assert!(!resolve_is_exact(Some("admin"), Some(false)));
        assert!(resolve_is_exact(Some("shop"), Some(false)));
        assert!(resolve_is_exact(Some("shop"), Some(true)));
    }

    // ————————————————————— 密码脱敏 —————————————————————

    #[test]
    fn mask_password_clears_credential() {
        let mut vo = UserVO {
            password: "$2a$10$abcdefghijklmnopqrstuv".into(),
            ..Default::default()
        };
        mask_password(&mut vo);
        // member::user_manage 约定空串；路由层再序列化为 null（Java setPassword(null)）
        assert!(vo.password.is_empty());
    }

    // ————————————————————— 归一化后的查询 —————————————————————

    #[test]
    fn override_keeps_other_query_fields_untouched() {
        // 只应覆盖 `is_exact`，其余筛选条件逐字段保持
        let q = UserListQuery {
            keyword: Some("alice".into()),
            is_exact: Some(false),
            user_type: Some(0),
            rank_id: Some(3),
            page: Some(2),
            size: Some(30),
            mark: Some(1),
            ..Default::default()
        };
        let mut effective = q.clone();
        effective.is_exact = Some(resolve_is_exact(Some("shop"), q.is_exact));

        assert_eq!(effective.is_exact, Some(true));
        assert_eq!(effective.keyword.as_deref(), Some("alice"));
        assert_eq!(effective.user_type, Some(0));
        assert_eq!(effective.rank_id, Some(3));
        assert_eq!(effective.page_params(), (2, 30));
        assert_eq!(effective.mark, Some(1));
        // 原查询对象不被就地修改（Java 是就地 set，Rust 侧用副本避免借用问题）
        assert_eq!(q.is_exact, Some(false));
    }

    #[test]
    fn service_handle_types_are_send_and_sync() {
        // `ProxyShoppingUserService` 会被包进 `Arc` 放进 `AdminState`（跨 await 共享），
        // 故必须 `Send + Sync`；这里做编译期校验。
        // 构造只取仓储句柄、不做任何 IO，因此上面所有纯逻辑测试无需数据库。
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ProxyShoppingUserService>();
        assert_send_sync::<ProxyShoppingUserRepository>();
    }
}