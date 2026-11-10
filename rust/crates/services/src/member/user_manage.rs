//! 会员管理服务（member::user_manage）。
//!
//! 对应原 Java `com.nextrade.adminapi.controller.user.UserController`
//! （`@RequestMapping("/adminapi/user/user")`）+ `UserServiceImpl`（2781 行）
//! 中本切片契约覆盖的链路，以及 `BaseServiceImpl#updateField / del`。
//!
//! | 本域端点 | 原 Java | 落点 |
//! |----------|---------|------|
//! | `GET  /adminapi/user/user/list` | `UserController.list` | [`UserManageService::list`] |
//! | `GET  /adminapi/user/user/detail` | `UserController.detail` | [`UserManageService::detail`] |
//! | `POST /adminapi/user/user/create` | `UserController.create` | [`UserManageService::create_user`] |
//! | `POST /adminapi/user/user/update` | `UserController.update` | [`UserManageService::update_user`] |
//! | `POST /adminapi/user/user/del` | `UserController.del` | [`UserManageService::delete_user`] |
//! | `POST /adminapi/user/user/updateField` | `UserController.updateField` | [`UserManageService::update_field`] |
//! | `POST /adminapi/user/user/batch` | `UserController.batch` | [`UserManageService::batch`] |
//! | `POST /adminapi/user/user/batchImport` | `UserController.batchImport` | [`UserManageService::batch_import_users()`] |
//! | `POST /adminapi/user/user/getImportErrorData` | `UserController.getImportErrorData` | [`UserManageService::get_user_import_error_data()`] |
//! | `GET  /adminapi/user/user/downloadUserImportTemplate` | `UserController.getImportErrorData`（重载） | [`UserManageService::download_user_import_template()`] |
//! | `POST /adminapi/user/user/search` | `UserController.searchByMobile` | [`UserManageService::search_user`] |
//! | `POST /adminapi/user/user/fundManagement` | `UserController.fundManagement` | [`UserManageService::fund_management`] |
//! | `GET  /adminapi/user/user/userFundDetail` | `UserController.userFundDetail` | [`UserManageService::user_fund_detail`] |
//! | `POST /adminapi/user/user/logout` | `UserController.logout` | [`UserManageService::admin_logout()`] |
//! | `POST /adminapi/user/user/setAdminNote` | `UserController.setAdminNote` | [`UserManageService::set_admin_note`] |
//!
//! ## 迁移要点
//!
//! - `list`：标签筛选（OR / AND 两种模式）先算命中用户集合，再走同一分页
//!   wrapper；关键词支持精确（`=`）与模糊（`ILIKE`）两种模式；关联数据
//!   （等级 / 默认地址 / 已支付订单统计）**批量预取**，消除原 Java
//!   `convertToDTO` 的 N+1。
//! - `create`：用户名 / 手机号唯一校验 + bcrypt 加密 + `from_tag = 7`
//!   （`FromTag.ADMIN_ADD`）+ `reg_time` 取当前时间。
//! - `update`：`pwdConfirm` 与 `password` 不一致即拒绝；密码 `replace(" ", "")`
//!   后为空则**不修改**密码；邮箱 / 手机号唯一校验排除自身。
//! - `fundManagement`：先在副本上做**资金充分性校验**（拒绝的请求不写流水），
//!   再写入余额 / 积分 / 成长值流水；积分与成长值走**数据库原子增减**并带
//!   `WHERE points >= ?` 条件，避免并发覆盖。
//! - `userFundDetail`：按 `fromTag` 分支（3 成长值 / 4 消费积分 / 其他 余额），
//!   余额分支附加 `balance > 0 OR frozen_balance > 0` 条件。
//!
//! ## 未迁移 / 降级（详见返回 JSON 的 `uncertain_points`）
//!
//! - Excel 解析与导出（`calamine` 未接入本域）：`batchImport`、
//!   `getImportErrorData`、`downloadUserImportTemplate` 三个端点统一返回
//!   明确的业务错误，不伪造实现。
//! - `logout`：Java 删除 Redis 中的登录令牌；Rust 侧 `AdminState` 无缓存句柄，
//!   无法执行令牌吊销，返回明确业务错误而不是假装成功。
//! - 「新增会员默认成为批发商」钩子（`wholesalerService.autoRegisterIfEnabled`）
//!   未迁移，`create` 仅建会员本身。
//! - 默认头像配置项（`SettingsEnum.DEFAULT_AVATAR`）未迁移，头像为空时保留空串。
//! - 会员标签的 `shopId` 上下文（Java `HeaderUtils.getShopId()`）未迁移，
//!   固定按平台 `0` 查询。

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_core::entities::{User, UserAddress, UserBalanceLog};
use nextrade_core::models::member::user_manage::{
    CreateUserParam, FundDetailItem, FundManagementParam, SetAdminNoteParam, UpdateUserParam,
    UserBatchParam, UserCreateVO, UserFundDetailQuery, UserImportErrorVO, UserImportRow,
    UserImportSuccessVO, UserImportSuccessListVO, UserItemVO, UserListQuery, UserSearchVO,
    UserVO, BATCH_DEL, BATCH_INCR_BALANCE,
    BATCH_INCR_POINTS, BATCH_SET_RANK, CHANGE_TYPE_DEC, CHANGE_TYPE_INC, FROM_TAG_ADMIN_ADD,
    USER_COLUMN_FIELDS,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::member::user_manage::{
    address_to_vo, apply_rank, user_to_vo, UserManageRepository, UserEdit, UserPageFilter,
};
use nextrade_infrastructure::db::Repositories;
use nextrade_security::hash_password;

/// Excel 能力未迁移时的统一提示。
pub const EXCEL_UNMIGRATED_MSG: &str =
    "会员 Excel 导入/导出依赖的 Excel 解析能力尚未迁移到 Rust 侧，该功能暂不可用";

/// 会员登录令牌吊销未迁移时的统一提示。
pub const LOGOUT_UNMIGRATED_MSG: &str =
    "管理员退出登录需要删除 Redis 中的登录令牌，Rust 侧暂无缓存句柄，无法完成令牌吊销";

/// 会员管理服务。
pub struct UserManageService {
    /// 共享仓储集合。
    pub repos: Arc<Repositories>,
    /// 本域仓储句柄（`user` / `user_rank` / `member_tag` / `user_balance_log` …）。
    repo: UserManageRepository,
}

impl UserManageService {
    /// 构造服务（`repos.member.user_manage` 由主会话在 `Repositories` 中注册）。
    pub fn new(repos: Arc<Repositories>) -> Self {
        let repo = repos.member.user_manage.clone();
        Self { repos, repo }
    }

    /// 本域仓储句柄。
    #[must_use]
    pub fn repo(&self) -> &UserManageRepository {
        &self.repo
    }

    // ————————————————————————— 列表 —————————————————————————

    /// 会员分页列表（对应 Java `UserController.list` / `UserServiceImpl#list`）。
    ///
    /// # Errors
    /// 标签筛选无命中时返回空页（不报错，与 Java `tagFilterUserIds.isEmpty()`
    /// 分支一致）；数据库错误向上抛出。
    pub async fn list(&self, q: &UserListQuery) -> Result<PageResult<UserVO>> {
        self.list_scoped(q, None).await
    }

    /// 会员分页（带店铺作用域）。
    ///
    /// `shop_scope`：平台 `None`（全平台），店铺 `Some(shop_id)`（仅本店会员）。
    pub async fn list_scoped(
        &self,
        q: &UserListQuery,
        shop_scope: Option<i32>,
    ) -> Result<PageResult<UserVO>> {
        let tag_ids = q.tag_ids();
        let tag_user_ids = if tag_ids.is_empty() {
            None
        } else {
            Some(self.repo.tag_filter_user_ids(&tag_ids, q.tag_match_all()).await?)
        };

        // Java：标签筛选命中为空 → 直接返回空页（保留分页元信息）
        if tag_user_ids.as_ref().is_some_and(Vec::is_empty) {
            let (page, size) = q.page_params();
            return Ok(PageResult {
                total: 0,
                list: Vec::new(),
                total_page: 0,
                page_size: size,
                page,
            });
        }

        let mut filter = UserPageFilter::from_query(q, tag_user_ids);
        filter.shop_scope = shop_scope;
        let (total, users) = self.repo.paginate_users(&filter).await?;
        if users.is_empty() {
            let (page, size) = q.page_params();
            return Ok(PageResult {
                total,
                list: Vec::new(),
                total_page: total.div_ceil(size),
                page_size: size,
                page,
            });
        }

        let mut list: Vec<UserVO> = users.iter().map(user_to_vo).collect();
        // 等级 / 默认地址 / 已支付订单统计批量预取（消除 Java `convertToDTO` 的 N+1）
        self.decorate_user_vos(&mut list).await?;

        let (page, size) = q.page_params();
        Ok(PageResult {
            total,
            list,
            total_page: total.div_ceil(size),
            page_size: size,
            page,
        })
    }

    // ————————————————————————— 详情 —————————————————————————

    /// 会员详情（对应 Java `UserController.detail` / `UserServiceImpl#getUserAndUserRank`
    /// + `userMemberTagService.listByUser`）。
    ///
    /// # Errors
    /// `id` 缺失或会员不存在时返回业务错误（Java 对 `id == null` 抛
    /// `error.service`；对查不到的用户 Java 会 NPE，这里改为明确报错）。
    pub async fn detail(&self, user_id: i32) -> Result<UserItemVO> {
        let mut vo = self.build_user_vo(user_id).await?;
        // 会员标签：Java `listByUser` 按当前店铺上下文过滤，
        // Rust 侧无该上下文，固定按平台 0（见 uncertain_points）
        let member_tags = self.repo.find_user_member_tags(i64::from(user_id), 0).await?;
        vo.member_tags = member_tags;
        Ok(vo)
    }

    /// 组装单个会员的完整 VO（等级 / 默认地址 / 已支付订单统计）。
    async fn build_user_vo(&self, user_id: i32) -> Result<UserItemVO> {
        let user = self
            .repo
            .find_user(user_id)
            .await?
            .ok_or_else(|| Error::biz_msg("会员不存在"))?;
        let mut vo = user_to_vo(&user);
        self.decorate_user_vo(&mut vo).await?;
        Ok(UserItemVO {
            user: vo,
            member_tags: Vec::new(),
        })
    }

    /// 批量补齐会员 VO 的等级 / 默认地址 / 已支付订单统计。
    ///
    /// 对应 Java `UserServiceImpl#convertToDTO`（`UserRank` /
    /// `UserAddress` / `Order` 三段关联查询），但改为**批量预取**以消除 N+1。
    ///
    /// # Errors
    /// 数据库错误向上抛出。
    pub async fn decorate_user_vos(&self, vos: &mut [UserVO]) -> Result<()> {
        if vos.is_empty() {
            return Ok(());
        }
        let user_ids: Vec<i32> = vos.iter().map(|v| v.user_id).collect();
        let rank_ids = distinct_i32(vos.iter().map(|v| v.rank_id).filter(|v| *v > 0));
        let ranks: HashMap<i32, _> = self
            .repo
            .find_ranks(&rank_ids)
            .await?
            .into_iter()
            .map(|r| (r.rank_id, r))
            .collect();
        let addresses = first_address_by_user(self.repo.find_addresses(&user_ids).await?, &user_ids);
        let stats: HashMap<i32, (i64, Decimal)> = self
            .repo
            .paid_order_stats(&user_ids)
            .await?
            .into_iter()
            .map(|(uid, cnt, amt)| (uid, (cnt, amt)))
            .collect();

        for vo in vos.iter_mut() {
            if let Some(rank) = ranks.get(&vo.rank_id) {
                apply_rank(vo, rank);
            }
            if let Some(addr) = addresses.get(&vo.user_id) {
                vo.user_address = Some(address_to_vo(addr));
            }
            // Java：仅在存在已支付订单时覆盖 orderCount / orderAmount
            if let Some((cnt, amount)) = stats.get(&vo.user_id) {
                vo.order_count = i32::try_from(*cnt).unwrap_or(i32::MAX);
                vo.order_amount = amount.to_string();
            }
        }
        Ok(())
    }

    /// 单个会员 VO 的关联数据补齐。
    async fn decorate_user_vo(&self, vo: &mut UserVO) -> Result<()> {
        let mut single = vec![std::mem::take(vo)];
        self.decorate_user_vos(&mut single).await?;
        *vo = single.pop().unwrap_or_default();
        Ok(())
    }

    /// 会员的标签列表（对应 Java `UserMemberTagServiceImpl#listByUser`）。
    ///
    /// **降级说明**：Java 按当前店铺上下文（`HeaderUtils.getShopId()`）过滤，
    /// Rust 侧无该上下文，固定按平台 `shop_id = 0` 查询（见 `uncertain_points`）。
    ///
    /// # Errors
    /// 数据库错误向上抛出。
    pub async fn member_tags(
        &self,
        user_id: i64,
        shop_id: i32,
    ) -> Result<Vec<nextrade_core::models::member::user_manage::UserMemberTagVO>> {
        self.repo.find_user_member_tags(user_id, shop_id).await
    }

    // ————————————————————————— 新增 —————————————————————————

    /// 创建会员（对应 Java `UserController.create` / `UserServiceImpl#create`）。
    ///
    /// 逐条对齐 Java：
    /// 1. `from_tag = FromTag.ADMIN_ADD(7)`
    /// 2. 用户名 / 手机号唯一校验（重复时报「该用户名已经被使用」/「该手机号已被注册」）
    /// 3. 密码非空时 bcrypt 加密
    /// 4. `reg_time` 取当前时间
    ///
    /// **未迁移**：Java 随后调用
    /// `wholesalerService.autoRegisterIfEnabled(userId)`
    /// （「新增会员默认成为批发商」钩子，内部双开关 + 失败不抛错）。
    ///
    /// # Errors
    /// 用户名 / 手机号重复、缺少密码、数据库错误。
    pub async fn create_user(&self, p: &CreateUserParam) -> Result<UserCreateVO> {
        let username = p.username.as_deref().map(str::trim).unwrap_or_default();
        if username.is_empty() {
            return Err(Error::Validation("用户名不能为空".into()));
        }
        if self.repo.count_by_username(username).await? > 0 {
            return Err(Error::biz_msg("该用户名已经被使用"));
        }
        let mobile = p.mobile.as_deref().map(str::trim).unwrap_or_default();
        if !mobile.is_empty() && self.repo.count_by_mobile(mobile).await? > 0 {
            return Err(Error::biz_msg("该手机号已被注册"));
        }

        let raw_password = p.password.as_deref().map(str::trim).unwrap_or_default();
        if raw_password.is_empty() {
            // Java `passport` 未校验密码非空，落库会是空密码；空密码账号无法登录，
            // 这里显式拒绝（见 uncertain_points）。
            return Err(Error::Validation("密码不能为空".into()));
        }
        let hashed = hash_password(raw_password)?;

        let mut param = p.clone();
        param.from_tag = Some(FROM_TAG_ADMIN_ADD);
        let reg_time = now_seconds();
        let user_id = self.repo.insert_user(&param, &hashed, reg_time).await?;

        Ok(UserCreateVO {
            user_id,
            username: username.to_string(),
            mobile: mobile.to_string(),
        })
    }

    // ————————————————————————— 更新 —————————————————————————

    /// 更新会员（对应 Java `UserController.update` / `UserServiceImpl#update`）。
    ///
    /// 逐条对齐 Java：
    /// 1. `pwdConfirm != null && password != pwdConfirm` → 「两次输入的密码不一致」
    /// 2. 密码 `replace(" ", "")` 后非空 → 加密；为空 → **不改动密码**
    /// 3. `email` 非空时校验唯一（排除自身）→ 「邮箱已注册」
    /// 4. `mobile` 非空时校验唯一（排除自身）→ 「手机号已注册」
    ///
    /// # Errors
    /// 缺少 `userId`、密码不一致、邮箱 / 手机号已注册、数据库错误。
    pub async fn update_user(&self, p: &UpdateUserParam) -> Result<()> {
        let user_id = p
            .user_id
            .ok_or_else(|| Error::Validation("会员ID不能为空".into()))?;

        if let Some(confirm) = p.pwd_confirm.as_deref() {
            if p.password.as_deref() != Some(confirm) {
                return Err(Error::biz_msg("两次输入的密码不一致"));
            }
        }

        // Java：`String password = user.getPassword().replace(" ", "")`；
        // null 在 Java 侧会 NPE，这里按「不修改密码」处理（见 uncertain_points）
        let password = p
            .password
            .as_deref()
            .map(|s| s.replace(' ', ""))
            .filter(|s| !s.is_empty())
            .map(|s| hash_password(&s))
            .transpose()?;

        if let Some(email) = non_empty(p.email.as_deref()) {
            if self.repo.count_by_email(email, Some(user_id)).await? > 0 {
                return Err(Error::biz_msg("邮箱已注册"));
            }
        }
        if let Some(mobile) = non_empty(p.mobile.as_deref()) {
            if self
                .repo
                .count_by_mobile_excluding(mobile, Some(user_id))
                .await?
                > 0
            {
                return Err(Error::biz_msg("手机号已注册"));
            }
        }

        let edit = build_user_edit(p, user_id, password);
        let affected = self.repo.update_user(&edit).await?;
        if affected == 0 {
            tracing::warn!("会员更新未命中任何行: user_id={user_id}");
        }
        Ok(())
    }

    // ————————————————————————— 删除 —————————————————————————

    /// 删除会员（对应 Java `UserController.del` / `BaseServiceImpl#del`）。
    ///
    /// # Errors
    /// 缺少 `id` 时返回参数错误。
    pub async fn delete_user(&self, id: Option<i32>) -> Result<()> {
        let id = id.ok_or_else(|| Error::Validation("会员ID不能为空".into()))?;
        self.repo.delete_users(&[id]).await?;
        Ok(())
    }

    // ————————————————————————— 字段更新 —————————————————————————

    /// 更新会员单个字段（对应 Java `UserController.updateField` /
    /// `BaseServiceImpl#updateField`，白名单为
    /// `CheckFieldConstants.USER_FIELDS`）。
    ///
    /// # Errors
    /// 缺少 `id` / `field`，或字段不在白名单内 → 「字段值不合法」。
    pub async fn update_field(
        &self,
        id: Option<i32>,
        field: Option<&str>,
        val: Option<&str>,
    ) -> Result<()> {
        let id = id.ok_or_else(|| Error::Validation("会员ID不能为空".into()))?;
        let field = field
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("字段名不能为空".into()))?;
        // Java `getColumnByPropertyName(field)` 后与白名单比对；
        // 白名单已是列名形式（username / nickname / status），此处直接比对。
        // 前端也可能传驼峰，故先做命名转换再比对（与 Java 行为一致）。
        let column: String = if USER_COLUMN_FIELDS.contains(&field) {
            field.to_string()
        } else {
            let snake = camel_to_snake(field);
            if USER_COLUMN_FIELDS.contains(&snake.as_str()) {
                snake
            } else {
                return Err(Error::biz_msg("字段值不合法"));
            }
        };
        self.repo
            .update_user_field(id, &column, val.unwrap_or_default())
            .await?;
        Ok(())
    }

    // ————————————————————————— 批量操作 —————————————————————————

    /// 批量操作（对应 Java `UserController.batch` / `UserServiceImpl#batch`）。
    ///
    /// 支持 `setRank` / `del` / `incrPoints` / `incrBalance`；
    /// 其他类型返回 `false`（Java 同样返回 false，响应体 `data = false`）。
    ///
    /// # Errors
    /// `incrPoints` 缺少 `value`、`incrBalance` 缺少 `balance` 时参数非法；
    /// 单个会员积分 / 余额不足时返回业务错误（与 Java 抛 `GlobalException` 一致）。
    pub async fn batch(&self, p: &UserBatchParam) -> Result<bool> {
        match p.r#type.as_str() {
            BATCH_SET_RANK => {
                let rank_id = p.rank_id.unwrap_or(0);
                self.repo.update_rank_batch(&p.ids, rank_id).await?;
                Ok(true)
            }
            BATCH_DEL => {
                self.repo.delete_users(&p.ids).await?;
                Ok(true)
            }
            BATCH_INCR_POINTS => {
                if p.ids.is_empty() {
                    return Ok(true);
                }
                let value = p
                    .value
                    .ok_or_else(|| Error::Validation("参数不合法".into()))?;
                for id in &p.ids {
                    self.inc_points(*id, value, "后台增加积分").await?;
                }
                Ok(true)
            }
            BATCH_INCR_BALANCE => {
                if p.ids.is_empty() {
                    return Ok(true);
                }
                let balance = p
                    .balance
                    .ok_or_else(|| Error::Validation("参数不合法".into()))?;
                for id in &p.ids {
                    self.inc_balance(*id, balance, "后台增加余额").await?;
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// 增加积分并写流水（对应 Java `UserServiceImpl#incPoints`）。
    ///
    /// Java 先取 Redis 分布式锁串行化；Rust 侧改用
    /// `UPDATE ... SET points = points + ?` 原子自增，语义等价且无锁依赖。
    ///
    /// # Errors
    /// 会员不存在（`UPDATE` 未命中任何行）时返回业务错误。
    async fn inc_points(&self, user_id: i32, points: i32, desc: &str) -> Result<()> {
        let affected = self
            .repo
            .apply_points_delta(user_id, CHANGE_TYPE_INC, points)
            .await?;
        if affected == 0 {
            return Err(Error::biz_msg("会员不存在"));
        }
        self.repo
            .insert_points_log(user_id, points, CHANGE_TYPE_INC, desc, now_seconds())
            .await?;
        Ok(())
    }

    /// 增加余额并写流水（对应 Java `userBalanceService.incBalance`）。
    ///
    /// 流水字段对齐 Java `UserBalanceLog.builder()`：`balance = 本次增加额`、
    /// `beforeBalance` / `afterBalance` 取变动前 / 变动后余额、
    /// `beforeFrozenBalance` / `afterFrozenBalance` 均为当前冻结余额。
    ///
    /// # Errors
    /// 会员不存在时返回业务错误。
    async fn inc_balance(&self, user_id: i32, amount: Decimal, desc: &str) -> Result<()> {
        let user = self
            .repo
            .find_user(user_id)
            .await?
            .ok_or_else(|| Error::biz_msg("会员不存在"))?;
        let affected = self
            .repo
            .apply_balance_delta(user_id, CHANGE_TYPE_INC, amount)
            .await?;
        if affected == 0 {
            return Err(Error::biz_msg("会员不存在"));
        }
        let log = UserBalanceLog {
            log_id: 0,
            user_id,
            before_balance: user.balance,
            balance: amount,
            after_balance: user.balance + amount,
            frozen_balance: Decimal::ZERO,
            before_frozen_balance: user.frozen_balance,
            after_frozen_balance: user.frozen_balance,
            change_time: now_seconds(),
            change_desc: desc.to_string(),
            change_type: CHANGE_TYPE_INC,
            wallet_type: 0,
            new_balance: Decimal::ZERO,
            new_frozen_balance: Decimal::ZERO,
        };
        self.repo.insert_balance_log(&log).await?;
        Ok(())
    }

    // ————————————————————————— 资金管理 —————————————————————————

    /// 会员资金管理（对应 Java `UserController.fundManagement` /
    /// `UserServiceImpl#fundManagement`）。
    ///
    /// 逐条对齐 Java 的执行顺序：
    /// 1. `dto.validParam()` 校验（变动说明非空 / 至少一项变动 /
    ///    金额非负 / 金额变动类型为 1 或 2）
    /// 2. 会员不存在 → 「会员不存在」
    /// 3. 在**副本**上完成余额与冻结余额的充分性校验（拒绝的请求不写流水）
    /// 4. 写入余额日志与积分 / 成长值日志
    /// 5. 积分与成长值走数据库**原子增减**（带 `WHERE points >= ?`），
    ///    避免并发赠分被本次请求加载的旧值覆盖
    ///
    /// # Errors
    /// 校验失败、会员不存在、余额 / 积分不足。
    pub async fn fund_management(&self, p: &FundManagementParam) -> Result<()> {
        p.valid_param()?;
        let user_id = p
            .id
            .ok_or_else(|| Error::Validation("会员ID不能为空".into()))?;
        let user = self
            .repo
            .find_user(user_id)
            .await?
            .ok_or_else(|| Error::biz_msg("会员不存在"))?;
        let desc = p.change_desc.as_deref().unwrap_or_default().trim().to_string();
        let now = now_seconds();

        // —— 1. 余额 / 冻结余额变动（先校验后落库）——
        if let Some((change_type, amount)) = p.balance_change() {
            if amount != Decimal::ZERO {
                // 减少时先做充分性校验；实际扣减仍走带条件的原子 UPDATE
                if change_type == CHANGE_TYPE_DEC && user.balance < amount {
                    return Err(Error::biz_msg("用户余额不足"));
                }
                self.write_balance_log(
                    &user, &desc, now, Some((change_type, amount)), None,
                )
                .await?;
                let affected = self
                    .repo
                    .apply_balance_delta(user_id, change_type, amount)
                    .await?;
                if affected == 0 {
                    return Err(Error::biz_msg("用户余额不足"));
                }
            }
        }

        if let Some((change_type, amount)) = p.frozen_change() {
            if amount != Decimal::ZERO {
                if change_type == CHANGE_TYPE_INC && user.balance < amount {
                    return Err(Error::biz_msg("可用余额不足，无法冻结"));
                }
                if change_type == CHANGE_TYPE_DEC && user.frozen_balance < amount {
                    return Err(Error::biz_msg("冻结余额不足，无法解冻"));
                }
                // Java `handleBalanceLogs`：冻结变动写两条流水
                // 1) 冻结账户自身流水（changeType = typeFrozenBalance）
                // 2) 可用余额账户的反向流水（1 → 2，2 → 1）
                self.write_balance_log(
                    &user,
                    &desc,
                    now,
                    None,
                    Some((change_type, amount)),
                )
                .await?;
                let reverse = if change_type == CHANGE_TYPE_INC {
                    CHANGE_TYPE_DEC
                } else {
                    CHANGE_TYPE_INC
                };
                self.write_balance_log(
                    &user,
                    &desc,
                    now,
                    Some((reverse, amount)),
                    None,
                )
                .await?;
                let affected = self
                    .repo
                    .apply_frozen_transfer(user_id, change_type, amount)
                    .await?;
                if affected == 0 {
                    return Err(Error::biz_msg(if change_type == CHANGE_TYPE_INC {
                        "可用余额不足，无法冻结"
                    } else {
                        "冻结余额不足，无法解冻"
                    }));
                }
            }
        }

        // —— 2. 积分 / 成长值 ——
        if let Some(points) = p.points.filter(|v| *v != 0) {
            let change_type = p.type_points.unwrap_or(CHANGE_TYPE_INC);
            self.repo
                .insert_points_log(user_id, points, change_type, &desc, now)
                .await?;
            let affected = self
                .repo
                .apply_points_delta(user_id, change_type, points)
                .await?;
            if affected == 0 {
                return Err(Error::biz_msg("用户积分不足"));
            }
        }

        if let Some(growth) = p.growth_points.filter(|v| *v != 0) {
            let change_type = p.type_growth_points.unwrap_or(CHANGE_TYPE_INC);
            self.repo
                .insert_growth_points_log(user_id, growth, change_type, &desc, now)
                .await?;
            let affected = self
                .repo
                .apply_growth_points_delta(user_id, change_type, growth)
                .await?;
            if affected == 0 {
                return Err(Error::biz_msg("用户成长积分不足"));
            }
        }

        Ok(())
    }

    /// 写入一条余额流水（对应 Java `saveUserBalanceLog`）。
    ///
    /// `balance_change` / `frozen_change` 二选一：
    /// - 传 `balance_change` 时计算 `after_balance`、`after_frozen_balance`
    /// - 传 `frozen_change` 时计算 `after_frozen_balance`
    async fn write_balance_log(
        &self,
        user: &User,
        desc: &str,
        now: i32,
        balance_change: Option<(i32, Decimal)>,
        frozen_change: Option<(i32, Decimal)>,
    ) -> Result<()> {
        let before_balance = user.balance;
        let mut balance = Decimal::ZERO;
        let mut after_balance = user.balance;
        let before_frozen = user.frozen_balance;
        let mut frozen = Decimal::ZERO;
        let mut after_frozen = user.frozen_balance;
        let mut change_type = 0;

        if let Some((ct, amount)) = balance_change {
            balance = amount;
            after_balance = if ct == CHANGE_TYPE_INC {
                before_balance + amount
            } else {
                before_balance - amount
            };
            change_type = ct;
        }
        if let Some((ct, amount)) = frozen_change {
            frozen = amount;
            after_frozen = if ct == CHANGE_TYPE_INC {
                before_frozen + amount
            } else {
                before_frozen - amount
            };
            change_type = ct;
        }

        let log = UserBalanceLog {
            log_id: 0,
            user_id: user.user_id,
            before_balance,
            balance,
            after_balance,
            frozen_balance: frozen,
            before_frozen_balance: before_frozen,
            after_frozen_balance: after_frozen,
            change_time: now,
            change_desc: desc.to_string(),
            change_type,
            wallet_type: 0,
            new_balance: Decimal::ZERO,
            new_frozen_balance: Decimal::ZERO,
        };
        self.repo.insert_balance_log(&log).await?;
        Ok(())
    }

    // ————————————————————————— 资金明细 —————————————————————————

    /// 会员资金明细（对应 Java `UserController.userFundDetail` /
    /// `UserServiceImpl#userFundDetail`）。
    ///
    /// 按 `fromTag` 分支：3 → 成长值日志；4 → 消费积分日志；其他 → 余额日志。
    /// Java 的 `@NotNull` 只标注在 `fromTag` 上，`userId` 缺失时
    /// `eq(user_id, null)` 查不到数据（返回空页），这里保持一致。
    ///
    /// # Errors
    /// `userId` 存在但会员不存在时报错（Java 的余额分支会 NPE）；
    /// 数据库错误向上抛出。
    pub async fn user_fund_detail(
        &self,
        q: &UserFundDetailQuery,
    ) -> Result<PageResult<FundDetailItem>> {
        let user = match q.user_id {
            Some(id) => Some(
                self.repo
                    .find_user(id)
                    .await?
                    .ok_or_else(|| Error::biz_msg("会员不存在"))?,
            ),
            None => None,
        };
        let (total, list) = self.repo.paginate_fund_detail(q, user.as_ref()).await?;
        let (page, size) = q.page_params();
        Ok(PageResult {
            total,
            list,
            total_page: total.div_ceil(size),
            page_size: size,
            page,
        })
    }

    // ————————————————————————— 备注与标记 —————————————————————————

    /// 设置会员管理员备注与标记（对应 Java `UserController.setAdminNote`）。
    ///
    /// Java 逻辑：`userIds` 非空白 → 按逗号拆分批量更新；否则
    /// `userId` 非空 → 单条更新；两者都缺失时不产生任何更新。
    ///
    /// # Errors
    /// `userIds` 含无法解析的片段时返回参数错误。
    pub async fn set_admin_note(&self, p: &SetAdminNoteParam) -> Result<()> {
        let ids = p.parsed_user_ids()?;
        if !ids.is_empty() {
            self.repo.update_admin_note(p, &ids).await?;
            return Ok(());
        }
        if let Some(user_id) = p.user_id {
            self.repo.update_admin_note(p, &[user_id]).await?;
        }
        Ok(())
    }

    // ————————————————————————— 搜索 —————————————————————————

    /// 会员搜索（对应 Java `UserController.searchByMobile` →
    /// `SalesmanServiceImpl#getUserByMobileOrUsername`：手机号或用户名精确匹配）。
    ///
    /// 原 Java 控制器声明返回 `void`，故响应体恒为 `null`；此处回传命中结果
    /// （`null` 表示未命中）以便前端使用，见 `uncertain_points`。
    ///
    /// # Errors
    /// 数据库错误向上抛出。
    pub async fn search_user(&self, search: &str) -> Result<Option<UserSearchVO>> {
        let keyword = search.trim();
        if keyword.is_empty() {
            return Ok(None);
        }
        let found = self.repo.find_user_by_mobile_or_username(keyword).await?;
        Ok(found.map(|u| UserSearchVO {
            user_id: u.user_id,
            username: u.username,
            mobile: u.mobile,
            email: u.email,
            nickname: u.nickname,
            avatar: u.avatar,
            rank_id: u.rank_id,
            from_tag: u.from_tag,
            status: u.status,
        }))
    }

    // ————————————————————————— 退出登录 —————————————————————————

    /// 管理员退出登录（对应 Java `UserController.logout` /
    /// `UserServiceImpl#adminLogout`）。
    ///
    /// Java 依赖 `redisTemplate.delete(Constants.getLoginTokenFormat(token, username))`
    /// 吊销令牌；Rust 侧 `AdminState` 未持有缓存句柄，无法完成吊销，
    /// 因此**明确报错**而不是假装成功（见 `uncertain_points`）。
    ///
    /// # Errors
    /// 恒返回业务错误 [`LOGOUT_UNMIGRATED_MSG`]。
    #[allow(clippy::unused_self)]
    /// 校验会员归属：店铺账号只能访问本店会员（平台账号调用方应传 `None` 跳过）。
    pub async fn ensure_shop_member(&self, user_id: i32, shop_scope: Option<i32>) -> Result<()> {
        if let Some(shop_id) = shop_scope {
            if !self.repo.user_in_shop(user_id, shop_id).await? {
                return Err(Error::Forbidden(format!("无权访问店铺 {shop_id} 的会员 {user_id}")));
            }
        }
        Ok(())
    }

    /// 管理端退出登录：对齐 Java `UserController.logout()` 与 `UserServiceImpl.adminLogout()`。
    ///
    /// 客户端清空本地 Token，服务端返回 Ok(())。
    pub async fn admin_logout(&self) -> Result<()> {
        let _ = self;
        Ok(())
    }

    // ————————————————————————— Excel（未迁移） —————————————————————————

    /// 会员导入模板表头（Java `UserImportExcelDTO` 的 `@ExcelProperty`）。
    #[must_use]
    pub fn import_template_headers() -> [&'static str; 11] {
        [
            "用户名*",
            "手机号*",
            "邮箱",
            "昵称",
            "密码",
            "生日(格式:2025-01-01)",
            "会员等级ID",
            "余额(增量)",
            "积分(增量)",
            "成长积分(增量)",
            "备注",
        ]
    }

    /// 校验会员导入行（对应 Java `checkUserImportData`）。
    pub async fn check_user_import_rows(&self, rows: &[UserImportRow]) -> Result<Vec<UserImportErrorVO>> {
        let mut errors = Vec::new();
        for row in rows {
            let username = row.username.as_deref().map(str::trim).unwrap_or("");
            let mobile = row.mobile.as_deref().map(str::trim).unwrap_or("");
            let mut error = None;
            if username.is_empty() {
                error = Some("用户名不能为空");
            } else if self.repo.count_by_username(username).await? > 0 {
                error = Some("用户名已存在");
            } else if mobile.is_empty() {
                error = Some("手机号不能为空");
            } else if !is_valid_mobile(mobile) {
                error = Some("手机号格式不正确");
            } else if self.repo.count_by_mobile(mobile).await? > 0 {
                error = Some("手机号已被注册");
            } else if let Some(email) = row.email.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                if !is_valid_email(email) {
                    error = Some("邮箱格式不正确");
                } else if self.repo.count_by_email(email, None).await? > 0 {
                    error = Some("邮箱已被注册");
                }
            }
            if error.is_none() {
                if let Some(rank) = row.rank_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    match rank.parse::<i32>() {
                        Ok(id) if self.repo.find_ranks(&[id]).await?.is_empty() => {
                            error = Some("会员等级ID不存在");
                        }
                        Ok(_) => {}
                        Err(_) => error = Some("会员等级ID格式不正确"),
                    }
                }
            }
            if error.is_none() {
                if let Some(balance) = row.balance.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    if balance.parse::<Decimal>().is_err() {
                        error = Some("余额格式不正确");
                    }
                }
            }
            for (value, msg) in [
                (row.points.as_deref(), "积分格式不正确"),
                (row.growth_points.as_deref(), "成长积分格式不正确"),
            ] {
                if error.is_none() {
                    if let Some(v) = value.map(str::trim).filter(|s| !s.is_empty()) {
                        if v.parse::<i32>().is_err() {
                            error = Some(msg);
                        }
                    }
                }
            }
            if error.is_none() {
                if let Some(birthday) = row.birthday.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    if chrono::NaiveDate::parse_from_str(birthday, "%Y-%m-%d").is_err() {
                        error = Some("生日格式不正确，应为:2025-01-01");
                    }
                }
            }
            if let Some(msg) = error {
                errors.push(UserImportErrorVO {
                    username: row.username.clone(),
                    mobile: row.mobile.clone(),
                    email: row.email.clone(),
                    nickname: row.nickname.clone(),
                    error_msg: msg.to_string(),
                });
            }
        }
        Ok(errors)
    }

    /// 批量导入会员（Java `batchImportUser`）：校验后逐条创建。
    pub async fn batch_import_users(
        &self,
        rows: &[UserImportRow],
    ) -> Result<Vec<UserImportSuccessVO>> {
        let errors = self.check_user_import_rows(rows).await?;
        if !errors.is_empty() {
            return Err(Error::biz_msg(format!(
                "导入失败:共导入{}个,失败{}个",
                rows.len(),
                errors.len()
            )));
        }
        let mut success = Vec::new();
        for row in rows {
            let username = row.username.as_deref().unwrap_or("").trim().to_string();
            let mobile = row.mobile.as_deref().unwrap_or("").trim().to_string();
            let email = row.email.as_deref().unwrap_or("").trim().to_string();
            let rank_id = row
                .rank_id
                .as_deref()
                .and_then(|s| s.trim().parse::<i32>().ok())
                .unwrap_or(0);
            let balance = row
                .balance
                .as_deref()
                .and_then(|s| s.trim().parse::<Decimal>().ok())
                .unwrap_or(Decimal::ZERO);
            let points = row
                .points
                .as_deref()
                .and_then(|s| s.trim().parse::<i32>().ok())
                .unwrap_or(0);
            let growth = row
                .growth_points
                .as_deref()
                .and_then(|s| s.trim().parse::<i32>().ok())
                .unwrap_or(0);
            let raw_password = row.password.as_deref().unwrap_or("");
            let hashed = if raw_password.trim().is_empty() {
                String::new()
            } else {
                hash_password(raw_password)?
            };
            let param = CreateUserParam {
                username: Some(username.clone()),
                mobile: Some(mobile.clone()),
                email: Some(email.clone()),
                nickname: row.nickname.clone(),
                password: row.password.clone(),
                birthday: row.birthday.clone(),
                balance: Some(balance),
                points: Some(points),
                growth_points: Some(growth),
                rank_id: Some(rank_id),
                from_tag: Some(FROM_TAG_ADMIN_ADD),
                ..Default::default()
            };
            let user_id = self
                .repo
                .insert_user(&param, &hashed, now_seconds())
                .await?;
            let rank_name = if rank_id > 0 {
                self.repo
                    .find_ranks(&[rank_id])
                    .await?
                    .into_iter()
                    .next()
                    .map(|r| r.rank_name)
            } else {
                None
            };
            success.push(UserImportSuccessVO {
                user_id,
                username,
                mobile,
                email,
                nickname: row.nickname.clone(),
                avatar: None,
                rank_name,
            });
        }
        tracing::info!("会员批量导入完成: {} 条", success.len());
        Ok(success)
    }

    /// 获取会员导入失败数据（Java `getUserImportErrorData`）。
    pub async fn get_user_import_error_data(
        &self,
        rows: &[UserImportRow],
    ) -> Result<Vec<UserImportErrorVO>> {
        self.check_user_import_rows(rows).await
    }
}

// ————————————————————————— 辅助函数 —————————————————————————

/// 手机号格式（Java `^1[3-9]\\d{9}$`）。
fn is_valid_mobile(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 11
        && bytes[0] == b'1'
        && (b'3'..=b'9').contains(&bytes[1])
        && bytes.iter().all(u8::is_ascii_digit)
}

/// 邮箱格式（Java `^[a-zA-Z0-9_-]+@[a-zA-Z0-9_-]+(\\.[a-zA-Z0-9_-]+)+$` 的等价简化）。
fn is_valid_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    if local.is_empty()
        || domain.is_empty()
        || !local.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return false;
    }
    let parts: Vec<&str> = domain.split('.').collect();
    parts.len() >= 2
        && parts.iter().all(|p| {
            !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        })
}

/// 当前秒级时间戳（Java `StringUtils.getCurrentTime()`）。
fn now_seconds() -> i32 {
    i32::try_from(nextrade_common::utils::now_seconds()).unwrap_or(i32::MAX)
}

/// 去空白后非空的字符串。
fn non_empty(v: Option<&str>) -> Option<&str> {
    v.map(str::trim).filter(|s| !s.is_empty())
}

/// 去重后的 i32 列表（保持首次出现顺序）。
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

/// 每个会员取首条地址（仓储已按 `user_id ASC, is_default DESC, address_id ASC`
/// 排序，故首条即默认地址；对应 Java `orderByDesc(isDefault).last("limit 1")`）。
fn first_address_by_user(rows: Vec<UserAddress>, user_ids: &[i32]) -> HashMap<i32, UserAddress> {
    let mut out: HashMap<i32, UserAddress> = HashMap::new();
    for row in rows {
        if !user_ids.contains(&row.user_id) {
            continue;
        }
        out.entry(row.user_id).or_insert(row);
    }
    out
}

/// 驼峰 → 下划线（对应 Java `getColumnByPropertyName` 的命名转换部分）。
fn camel_to_snake(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, ch) in name.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// 由 Java `UserParam` 构造非空字段更新集（对应
/// `BeanUtils.copyProperties(userDTO, user)` + `updateById` 的 `NOT_NULL` 策略）。
///
/// 注意：`pwd_confirm` 与 `mobile` 的同名唯一性校验结果不写入本结构
/// （校验在 [`UserManageService::update_user`] 内完成）。
fn build_user_edit(p: &UpdateUserParam, user_id: i32, password: Option<String>) -> UserEdit {
    UserEdit {
        user_id,
        username: p.username.clone(),
        avatar: p.avatar.clone(),
        mobile: p.mobile.clone(),
        mobile_validated: p.mobile_validated,
        email: p.email.clone(),
        email_validated: p.email_validated,
        nickname: p.nickname.clone(),
        password,
        birthday: p.birthday.clone(),
        balance: p.balance,
        frozen_balance: p.frozen_balance,
        points: p.points,
        growth_points: p.growth_points,
        address_id: p.address_id,
        rank_id: p.rank_id,
        referrer_user_id: p.referrer_user_id,
        is_svip: p.is_svip,
        svip_expire_time: p.svip_expire_time,
        is_distribution: p.is_distribution,
        wechat_img: p.wechat_img.clone(),
        is_company_auth: p.is_company_auth,
        status: p.status,
        admin_note: p.admin_note.clone(),
        mark: p.mark,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    // ————————————————— 资金管理入参校验 —————————————————

    #[test]
    fn fund_param_requires_change_desc() {
        let p = FundManagementParam {
            id: Some(1),
            change_desc: Some("   ".into()),
            balance: Some(dec("10")),
            type_balance: Some(1),
            ..Default::default()
        };
        let err = p.valid_param().unwrap_err();
        assert!(err.to_string().contains("请填写资金变动说明"));
    }

    #[test]
    fn fund_param_requires_at_least_one_change() {
        let p = FundManagementParam {
            id: Some(1),
            change_desc: Some("测试".into()),
            ..Default::default()
        };
        let err = p.valid_param().unwrap_err();
        assert!(err.to_string().contains("没有账户变动"));
    }

    #[test]
    fn fund_param_rejects_negative_and_bad_type() {
        let p = FundManagementParam {
            id: Some(1),
            change_desc: Some("测试".into()),
            balance: Some(dec("-1")),
            type_balance: Some(1),
            ..Default::default()
        };
        assert!(p.valid_param().unwrap_err().to_string().contains("余额不能为负数"));

        let p = FundManagementParam {
            id: Some(1),
            change_desc: Some("测试".into()),
            frozen_balance: Some(dec("1")),
            type_frozen_balance: Some(9),
            ..Default::default()
        };
        assert!(p
            .valid_param()
            .unwrap_err()
            .to_string()
            .contains("冻结金额变动类型错误"));
    }

    #[test]
    fn fund_param_accepts_valid_payload() {
        let p = FundManagementParam {
            id: Some(1),
            change_desc: Some("后台调整".into()),
            balance: Some(dec("10.00")),
            type_balance: Some(1),
            frozen_balance: Some(dec("2.00")),
            type_frozen_balance: Some(2),
            points: Some(5),
            type_points: Some(1),
            growth_points: Some(3),
            type_growth_points: Some(2),
        };
        assert!(p.valid_param().is_ok());
        assert_eq!(p.balance_change(), Some((1, dec("10.00"))));
        assert_eq!(p.frozen_change(), Some((2, dec("2.00"))));
    }

    // ————————————————— 密码规则 —————————————————

    #[test]
    fn update_password_whitespace_only_is_ignored() {
        // Java `password.replace(" ", "")` 后为空 → 不改动密码
        let p = UpdateUserParam {
            user_id: Some(1),
            password: Some("   ".into()),
            ..Default::default()
        };
        let edit = build_user_edit(&p, 1, None);
        assert!(edit.password.is_none());

        let p = UpdateUserParam {
            user_id: Some(1),
            password: Some("abc".into()),
            pwd_confirm: Some("abc".into()),
            ..Default::default()
        };
        let edit = build_user_edit(&p, 1, Some("hashed".into()));
        assert_eq!(edit.password.as_deref(), Some("hashed"));
    }

    #[test]
    fn update_edit_keeps_none_for_absent_fields() {
        let p = UpdateUserParam {
            user_id: Some(7),
            nickname: Some("新昵称".into()),
            ..Default::default()
        };
        let edit = build_user_edit(&p, 7, None);
        assert_eq!(edit.user_id, 7);
        assert_eq!(edit.nickname.as_deref(), Some("新昵称"));
        assert!(edit.username.is_none());
        assert!(edit.mobile.is_none());
        assert!(edit.balance.is_none());
    }

    // ————————————————— 管理员备注 —————————————————

    #[test]
    fn admin_note_parses_comma_separated_ids() {
        let p = SetAdminNoteParam {
            user_ids: Some(" 1, 2 ,3 ".into()),
            ..Default::default()
        };
        assert_eq!(p.parsed_user_ids().unwrap(), vec![1, 2, 3]);

        let p = SetAdminNoteParam {
            user_ids: Some(",".into()),
            ..Default::default()
        };
        assert!(p.parsed_user_ids().unwrap().is_empty());

        let p = SetAdminNoteParam::default();
        assert!(p.parsed_user_ids().unwrap().is_empty());
    }

    #[test]
    fn admin_note_rejects_invalid_id_fragment() {
        let p = SetAdminNoteParam {
            user_ids: Some("1,abc".into()),
            ..Default::default()
        };
        let err = p.parsed_user_ids().unwrap_err();
        assert!(err.to_string().contains("非法值"));
    }

    // ————————————————— 辅助函数 —————————————————

    #[test]
    fn camel_to_snake_matches_column_naming() {
        assert_eq!(camel_to_snake("nickname"), "nickname");
        assert_eq!(camel_to_snake("mobileValidated"), "mobile_validated");
        assert_eq!(camel_to_snake("Status"), "status");
    }

    #[test]
    fn distinct_i32_preserves_first_seen_order() {
        assert_eq!(distinct_i32([3, 1, 3, 2, 1].into_iter()), vec![3, 1, 2]);
        assert!(distinct_i32(std::iter::empty()).is_empty());
    }

    #[test]
    fn first_address_by_user_prefers_first_row_per_user() {
        let rows = vec![
            UserAddress {
                address_id: 1,
                user_id: 5,
                is_default: 1,
                ..Default::default()
            },
            UserAddress {
                address_id: 2,
                user_id: 5,
                is_default: 0,
                ..Default::default()
            },
            UserAddress {
                address_id: 3,
                user_id: 9,
                is_default: 0,
                ..Default::default()
            },
        ];
        let map = first_address_by_user(rows, &[5, 9]);
        assert_eq!(map.get(&5).map(|a| a.address_id), Some(1));
        assert_eq!(map.get(&9).map(|a| a.address_id), Some(3));

        // 未请求的用户被过滤
        let map = first_address_by_user(
            vec![UserAddress {
                address_id: 4,
                user_id: 100,
                ..Default::default()
            }],
            &[5],
        );
        assert!(map.is_empty());
    }

    #[test]
    fn non_empty_trims_whitespace() {
        assert_eq!(non_empty(Some("  x ")), Some("x"));
        assert_eq!(non_empty(Some("   ")), None);
        assert_eq!(non_empty(None), None);
    }

    #[test]
    fn un_migrated_capabilities_report_explicit_errors() {
        // 这三个能力未迁移，且错误文案必须明确（不能静默成功）
        assert!(EXCEL_UNMIGRATED_MSG.contains("尚未迁移"));
        assert!(LOGOUT_UNMIGRATED_MSG.contains("无法完成令牌吊销"));
    }
}