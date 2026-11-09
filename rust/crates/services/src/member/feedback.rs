//! 会员留言域服务（member::feedback）。
//!
//! 对应原 Java `com.nextrade.service.user.impl.FeedbackServiceImpl`（622 行）
//! + `com.nextrade.adminapi.controller.user.FeedbackController`（75 行）里的
//! 5 条**管理端**链路：
//!
//! | 本域端点（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |--------------------------------|---------|------|
//! | `GET  /user/feedback/list`   | `FeedbackController.list`   | [`FeedbackService::list`] |
//! | `GET  /user/feedback/detail` | `FeedbackController.detail` | [`FeedbackService::detail`] |
//! | `POST /user/feedback/del`    | `FeedbackController.del`    | [`FeedbackService::del`] |
//! | `POST /user/feedback/update` | `FeedbackController.update` | [`FeedbackService::update`] |
//! | `POST /user/feedback/batch`  | `FeedbackController.batch`  | [`FeedbackService::batch`] |
//!
//! # 迁移要点（逐条对照 Java 源）
//!
//! ## 租户作用域（本域最容易出错的地方）
//!
//! Java 的 `currentAdminTenantScope()` 在 `list` / `detail` / `del` /
//! `update` / `batch` **每一条**链路上都会被调用（`del` / `batch` 通过
//! `requireAdminAccessibleFeedback` 间接调用），因此**五个接口都要带作用域**。
//! 作用域由请求头解析，由**路由层**构造 [`TenantScope`] 后传入：
//! - 平台（`admin`）：列表不强制 `shop_id`，但叠加
//!   `AND (shop_id = 0 OR type NOT IN (2))`；
//! - 店铺 / 门店（`shop` / `store`）：强制 `shop_id = 本店` 且 `type IN (2)`，
//!   且**不**叠加平台可见性条件；
//! - 其余（`vendor` / `pos` / `pickup` / 未知）：直接业务错误。
//!
//! ## `list`（Java `listForAdmin` → 私有 `list(pageQuery, tenantScope)`）
//!
//! 过滤条件与 Java 的 `lambdaQuery()` 链逐条对齐（见仓储
//! [`nextrade_infrastructure::db::repo::member::feedback::FeedbackRepository::paginate`]），
//! 其中三处**容易写错**：
//! 1. `productId` / `status` 只判 `!= null` ⇒ **值为 0 也要加条件**；
//!    `userId` 要求 `> 0` ⇒ **0 不加**；
//! 2. `isOrder`：`1` → `order_id = 0`；`> 1` → `order_id > 0`；
//!    `null` / `0` / 负数 → **都不加**；
//! 3. 店铺账号请求了类型但过滤后为空集 ⇒ 追加恒假 `1 = 0`
//!    （得到空页，而不是「不过滤」）。
//!
//! 回查一律走 `IN (...)` 批量查询（Java 也是批量），避免 N+1；
//! 最新回复靠 `ORDER BY id DESC` + 「首个胜出」实现。
//!
//! ## `detail`
//!
//! - 先 `requireAdminAccessibleFeedback(id)`（`id` 为 null 或查不到
//!   → `GlobalException("留言不存在")`）；
//! - 回复列表按 `id DESC`；**空列表时 `replyList` 保持 `null`**
//!   （Java 只在 `isNotEmpty` 时 `setReplyList`）；
//! - `feedbackPics` 仅在「非空白且长度 > 3」时解析，否则 `null`。
//!
//! ## `del`
//!
//! - `requireAdminAccessibleFeedback(id)` → `removeById(id)`：
//!   **物理删除**（表无软删列，schema 已核对）；
//! - Java 控制器**没有** `@Valid` ⇒ `id` 为 null 一路传到服务层，
//!   最终是业务错误「留言不存在」而不是参数校验 400。
//!
//! ## `update`（两条分支，`@Transactional`）
//!
//! 判定依据是 **`Objects.equals(dto.getParentId(), 0)`**：
//! - `parentId` **显式为 0** → 新增回复分支；
//! - `parentId` 为 `null` 或**非 0** → 编辑已有回复分支。注意 `null` 会
//!   先经过 `requireAdminAccessibleFeedback(dto.getParentId())` ⇒ 直接
//!   报「留言不存在」（Java 的真实行为，不是「编辑成功」）。
//!
//! 新增分支：回复继承父留言的 `type` / `productId` / `orderId` / `shopId`，
//! `title` 固定 `"reply"`，`status` 固定 `0`（待回复），
//! `email` / `mobile` / `content` 用 `blankToDefault(x, "")` ⇒ 写空串，
//! `feedbackPics` 为空时写 `"[]"`；插入成功后把**父留言**置为 `1`（已回复）。
//!
//! 编辑分支：先取「要编辑的回复」（`dto.id` 非空且不等于父留言 ID 时按
//! `dto.id` 取并校验 `parentId` 匹配，否则取该父留言最新一条），
//! 然后只改 `user_id` / `username` / `email` / `mobile` / `content` /
//! `feedback_pics` / `type` / `product_id` / `order_id` / `shop_id`，
//! **不改** `status` / `title` / `add_time` / `parent_id`。
//!
//! ## `batch`
//!
//! - `ids` 为空 / `type != "del"` → Java 直接 `return false`
//!   （**不报错**，HTTP 200）；
//! - 否则逐个 `requireAdminAccessibleFeedback` 校验，再 `removeByIds`
//!   （**任一**无权即整批失败，`@Transactional` 回滚）。
//!
//! # ⚠️ 相对 Java 的**有意差异**（已列入 `uncertain_points`）
//!
//! 1. Java 的 `detail` / `update` 里 `feedback.getOrderId() > 0` /
//!    `getShopId() > 0` 对 `NULL` 列会抛 NPE（HTTP 500）；本实现把 `NULL`
//!    当 `0` 处理（不崩、走「无订单/无店铺」分支）。
//! 2. Java 的 `updateById(parentFeedback)` 会写回整行非 null 字段
//!    （值都来自刚读出的同一行）⇒ 净效果只改了 `status`；本实现显式
//!    只 `SET status`（见仓储文档说明）。
//! 3. `GlobalException(String)` 的码是 `1001`（`SERVICE_DATA_ERROR`），
//!    本实现统一用 [`Error::biz`] 保留该码与**原文案**，
//!    因此「留言不存在」返回的是 `1001` 而**不是** 404。
//! 4. Java `update` 会先 `XssUtils.clean(dto.getTitle())`，但回复的 `title`
//!    随后被**硬编码为 `"reply"`** ⇒ 该清洗不可观测；本实现不做该死代码。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use nextrade_common::utils;
use nextrade_core::entities::user::Feedback;
use nextrade_core::models::member::feedback::{
    blank_to_default, build_detail_vo, build_list_vo, feedback_status, is_blank, list_order_clause,
    resolve_platform_feedback_types, resolve_shop_feedback_types, to_feedback_pics, xss_clean,
    BatchDTO, FeedbackListQuery, FeedbackLookups, FeedbackUpdateDTO, TenantScope,
    MSG_ADMIN_NOT_FOUND, MSG_FEEDBACK_NOT_FOUND, MSG_NO_PERMISSION_FEEDBACK, MSG_REPLY_NOT_BELONG,
    MSG_REPLY_NOT_FOUND, REPLY_TITLE, SERVICE_DATA_ERROR, SHOP_FEEDBACK_TYPES,
};
use nextrade_core::types::PageResult;
use nextrade_core::Error;
use nextrade_infrastructure::db::repo::member::feedback::{
    FeedbackListFilter, FeedbackReplyInsert, FeedbackReplyPatch, FeedbackRepository,
};
use nextrade_infrastructure::db::Repositories;

/// 结果类型别名（与其它切片服务一致）。
type Result<T> = nextrade_core::Result<T>;

/// 会员留言域服务（挂载于 `AdminState.feedback`）。
pub struct FeedbackService {
    repos: Arc<Repositories>,
}

impl FeedbackService {
    /// 构造（与 member_tag_group / vendor_deposit_shop / tax_rate 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储（便于测试与后续切片复用）。
    #[must_use]
    pub fn repo(&self) -> &FeedbackRepository {
        &self.repos.member.feedback
    }

    // ————————————————————————————————— 列表 —————————————————————————————————

    /// 会员留言分页（对应 Java `FeedbackServiceImpl#listForAdmin` →
    /// 私有 `list(pageQuery, tenantScope)`）。
    ///
    /// # Errors
    /// - 未知 `type` 码 → `Error::biz(1001, "数据类型异常")`
    ///   （Java `UserFeedbackType#getTypeName` 抛异常，整页失败）；
    /// - 数据库异常 → `Error::Database`。
    pub async fn list(
        &self,
        q: &FeedbackListQuery,
        scope: TenantScope,
    ) -> Result<PageResult<nextrade_core::models::member::feedback::UserFeedbackVO>> {
        // Java：`scopedTypeRequested = tenantScope.scoped() && StrUtil.isNotBlank(type)`
        let scoped_type_requested = scope.is_scoped() && !is_blank(q.r#type.as_deref());
        // Java：店铺账号走 resolveShopFeedbackTypes；平台账号走 StrUtil.split
        let types = if scope.is_scoped() {
            resolve_shop_feedback_types(q.r#type.as_deref())
        } else {
            resolve_platform_feedback_types(q.r#type.as_deref())
        };
        // Java：`queryShopId = scoped ? tenantScope.shopId() : pageQuery.getShopId()`
        let query_shop_id = if scope.is_scoped() {
            scope.shop_id()
        } else {
            q.shop_id
        };
        let filter = if scope.platform_admin() {
            FeedbackListFilter::platform(types.clone(), query_shop_id)
        } else {
            FeedbackListFilter::shop(
                types.clone(),
                scope.shop_id().unwrap_or(0),
                scoped_type_requested && types.is_empty(),
            )
        };

        let order_clause = list_order_clause(q);
        let (total, rows) = self.repo().paginate(q, &filter, &order_clause).await?;

        let page = q.page_query();
        if rows.is_empty() {
            return Ok(PageResult::new(total, Vec::new(), &page));
        }

        // 收集关联 ID（Java 用 stream().filter(...).collect(toSet())）。
        let mut user_ids: HashSet<i32> = HashSet::new();
        let mut order_ids: HashSet<i32> = HashSet::new();
        let mut product_ids: HashSet<i32> = HashSet::new();
        let mut shop_ids: HashSet<i32> = HashSet::new();
        let mut feedback_ids: Vec<i32> = Vec::with_capacity(rows.len());
        for f in &rows {
            user_ids.insert(f.user_id);
            if let Some(v) = f.order_id.filter(|v| *v > 0) {
                order_ids.insert(v);
            }
            if let Some(v) = f.product_id.filter(|v| *v > 0) {
                product_ids.insert(v);
            }
            if let Some(v) = f.shop_id.filter(|v| *v > 0) {
                shop_ids.insert(v);
            }
            feedback_ids.push(f.id);
        }

        let lookups = FeedbackLookups {
            users: self.repo().find_nicknames(&sorted_ids(&user_ids)).await?,
            orders: self.repo().find_order_sns(&sorted_ids(&order_ids)).await?,
            products: self
                .repo()
                .find_product_names(&sorted_ids(&product_ids))
                .await?,
            shops: self.repo().find_shop_titles(&sorted_ids(&shop_ids)).await?,
        };

        // 每个父留言只保留「最新一条」回复（Java `orderByDesc(id)` + putIfAbsent）。
        let reply_rows = self
            .repo()
            .find_replies_by_parent_ids(&feedback_ids)
            .await?;
        let mut replies: HashMap<i32, Feedback> = HashMap::new();
        for r in reply_rows {
            replies.entry(r.parent_id).or_insert(r);
        }

        let mut list = Vec::with_capacity(rows.len());
        for f in &rows {
            list.push(build_list_vo(f, &replies, &lookups)?);
        }
        Ok(PageResult::new(total, list, &page))
    }

    // ————————————————————————————————— 详情 —————————————————————————————————

    /// 会员留言详情（对应 Java `FeedbackServiceImpl#detail`）。
    ///
    /// # Errors
    /// - `id` 为 `None` / 记录不存在 / 越权 → `Error::biz(1001, ...)`；
    /// - 未知 `type` / `status` 码 → `Error::biz(1001, "数据类型异常")`。
    pub async fn detail(
        &self,
        id: Option<i32>,
        scope: TenantScope,
    ) -> Result<nextrade_core::models::member::feedback::UserFeedbackDetailVO> {
        let f = self.require_accessible(id, scope).await?;

        let nickname = self.repo().find_user_nickname(f.user_id).await?;
        // Java `if (feedback.getOrderId() > 0)`；NULL 列在 Java 会 NPE，
        // 本实现按「无订单」处理（见模块头差异 1）。
        let order = match f.order_id.filter(|v| *v > 0) {
            Some(order_id) => self.repo().find_order_brief(order_id).await?,
            None => None,
        };
        let product = match f.product_id.filter(|v| *v > 0) {
            Some(product_id) => self.repo().find_product_brief(product_id).await?,
            None => None,
        };
        let shop = match f.shop_id.filter(|v| *v > 0) {
            Some(shop_id) => self.repo().find_shop_brief(shop_id).await?,
            None => None,
        };
        let replies = self.repo().find_all_replies_by_parent_id(f.id).await?;

        build_detail_vo(&f, nickname, order, product, shop, &replies)
    }

    // ————————————————————————————————— 删除 —————————————————————————————————

    /// 删除一条留言（对应 Java `FeedbackServiceImpl#del`）。
    ///
    /// 物理删除（表无软删列）；`id` 为 `None` / 不存在 / 越权 → 业务错误。
    pub async fn del(&self, id: Option<i32>, scope: TenantScope) -> Result<()> {
        let f = self.require_accessible(id, scope).await?;
        self.repo().delete_by_id(f.id).await?;
        tracing::info!("删除会员留言: id={}", f.id);
        Ok(())
    }

    // ————————————————————————————————— 更新 / 回复 —————————————————————————————————

    /// 新增或编辑回复（对应 Java `FeedbackServiceImpl#update`，`@Transactional`）。
    ///
    /// `admin_id` 取自 JWT 身份（Java `SecurityUtils.getCurrentAdminId()`）。
    ///
    /// # Errors
    /// - 管理员不存在 → `Error::biz(1001, "管理员不存在")`；
    /// - 留言不存在 / 越权 / 回复不属于该留言 → 对应业务错误。
    pub async fn update(
        &self,
        dto: &FeedbackUpdateDTO,
        admin_id: i32,
        scope: TenantScope,
    ) -> Result<()> {
        // Java：`if (dto.getContent() != null) dto.setContent(XssUtils.clean(...))`
        let content = dto.content.as_deref().map(xss_clean);

        // Java：管理员查不到 → `GlobalException("管理员不存在")`。
        let admin_username = self
            .repo()
            .find_admin_username(admin_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, MSG_ADMIN_NOT_FOUND))?;

        // Java：判定依据是 `Objects.equals(dto.getParentId(), 0)`。
        if dto.parent_id == Some(0) {
            let parent = self.require_accessible(dto.id, scope).await?;
            let reply = build_reply_insert(
                &parent,
                admin_id,
                &admin_username,
                content.as_deref(),
                dto.feedback_pics.as_deref(),
                dto.email.as_deref(),
                dto.mobile.as_deref(),
                now_seconds_i32(),
            );
            let mut tx = self.repo().begin().await?;
            self.repo().insert_reply(&mut tx, &reply).await?;
            self.repo().mark_parent_replied(&mut tx, parent.id).await?;
            tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
            tracing::info!("新增留言回复: parent_id={}", parent.id);
            return Ok(());
        }

        // 编辑分支：`parentId` 为 `null` / 非 0。
        // ⚠️ `null` 时 `require_accessible(None, ...)` 直接报「留言不存在」。
        let parent = self.require_accessible(dto.parent_id, scope).await?;
        let target = if let Some(rid) = dto.id.filter(|rid| *rid != parent.id) {
            let reply = self.require_accessible(Some(rid), scope).await?;
            if reply.parent_id != parent.id {
                return Err(Error::biz(SERVICE_DATA_ERROR, MSG_REPLY_NOT_BELONG));
            }
            reply
        } else {
            self.repo()
                .find_latest_reply_by_parent_id(parent.id)
                .await?
                .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, MSG_REPLY_NOT_FOUND))?
        };

        let patch = build_reply_patch(
            &parent,
            admin_id,
            &admin_username,
            content.as_deref(),
            dto.feedback_pics.as_deref(),
            dto.email.as_deref(),
            dto.mobile.as_deref(),
        );
        let mut tx = self.repo().begin().await?;
        self.repo().update_reply(&mut tx, target.id, &patch).await?;
        tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        tracing::info!("编辑留言回复: id={}", target.id);
        Ok(())
    }

    // ————————————————————————————————— 批量 —————————————————————————————————

    /// 批量删除留言（对应 Java `FeedbackServiceImpl#batch`，`@Transactional`）。
    ///
    /// `ids` 为空 / `type != "del"` → 直接返回成功（Java 返回 `false`，
    /// 控制器忽略返回值，对外仍 HTTP 200）。
    ///
    /// # Errors
    /// - 任一条留言不存在 / 越权 → 业务错误（整批回滚）。
    pub async fn batch(&self, dto: &BatchDTO, scope: TenantScope) -> Result<()> {
        let ids = match dto.ids.as_deref() {
            Some(v) if !v.is_empty() => v,
            _ => return Ok(()),
        };
        if dto.r#type.as_deref() != Some("del") {
            return Ok(());
        }

        let mut tx = self.repo().begin().await?;
        for id in ids {
            let f = self
                .repo()
                .find_by_id_in_tx(&mut tx, *id)
                .await?
                .ok_or_else(feedback_not_found)?;
            let (feedback_type, shop_id) = if f.parent_id > 0 {
                let parent = self
                    .repo()
                    .find_by_id_in_tx(&mut tx, f.parent_id)
                    .await?
                    .ok_or_else(feedback_not_found)?;
                (parent.r#type, parent.shop_id)
            } else {
                (f.r#type, f.shop_id)
            };
            if scope_denied(feedback_type, shop_id, scope) {
                return Err(no_permission());
            }
        }
        self.repo().delete_by_ids(&mut tx, ids).await?;
        tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        tracing::info!("批量删除会员留言: {} 条", ids.len());
        Ok(())
    }

    // ————————————————————————————————— 内部 —————————————————————————————————

    /// 复刻 Java `FeedbackServiceImpl#requireAdminAccessibleFeedback`：
    /// 取记录 → （若为回复）取父留言 → 按租户作用域鉴权 → 返回原记录。
    ///
    /// `id` 为 `None` / 记录不存在 / 父留言不存在 → 「留言不存在」；
    /// 越权 → 「无权操作该留言」。
    async fn require_accessible(&self, id: Option<i32>, scope: TenantScope) -> Result<Feedback> {
        let id = id.ok_or_else(feedback_not_found)?;
        let f = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(feedback_not_found)?;

        let (feedback_type, shop_id) = if f.parent_id > 0 {
            let parent = self
                .repo()
                .find_by_id(f.parent_id)
                .await?
                .ok_or_else(feedback_not_found)?;
            (parent.r#type, parent.shop_id)
        } else {
            (f.r#type, f.shop_id)
        };
        if scope_denied(feedback_type, shop_id, scope) {
            return Err(no_permission());
        }
        Ok(f)
    }
}

// ——————————————————————————————————————————————
// 纯逻辑（可单测）
// ——————————————————————————————————————————————

/// 鉴权判定（对应 Java `requireAdminAccessibleFeedback` 的作用域部分）。
///
/// `feedback_type` / `shop_id` 是**父留言**（主留言）的字段：
/// Java 对回复会先回溯到 `parent_id` 指向的主留言再判定。
///
/// - 平台（`admin`）：命中 `SHOP_FEEDBACK_TYPES`（即 `type = 2`）**且**
///   `shop_id != 0` → 拒绝（`null` 也拒绝，因为 `!Objects.equals(null, 0)`）；
/// - 店铺 / 门店：`shop_id != 本店` **或** 非店铺留言类型 → 拒绝。
#[must_use]
fn scope_denied(feedback_type: i32, shop_id: Option<i32>, scope: TenantScope) -> bool {
    if scope.platform_admin() {
        SHOP_FEEDBACK_TYPES.contains(&feedback_type) && shop_id != Some(0)
    } else {
        shop_id != scope.shop_id() || !SHOP_FEEDBACK_TYPES.contains(&feedback_type)
    }
}

/// 组装「新增回复」的插入实体（对应 Java `update` 的 `parentId == 0` 分支）。
///
/// 逐条对照 Java：
/// - `addTime` = 当前秒级时间戳；
/// - `userId` / `username` = 当前管理员；
/// - `email` / `mobile` / `content` = `blankToDefault(x, "")`（**总是写空串**）；
/// - `feedbackPics` = `toFeedbackPics(...)`（空 → `"[]"`）；
/// - `parentId` / `type` / `productId` / `orderId` / `shopId` 继承主留言；
/// - `title` 固定 `"reply"`，`status` 固定 `WAIT_REPLY`(0)。
#[must_use]
fn build_reply_insert(
    parent: &Feedback,
    admin_id: i32,
    admin_username: &str,
    content: Option<&str>,
    pics: Option<&[String]>,
    email: Option<&str>,
    mobile: Option<&str>,
    now: i32,
) -> FeedbackReplyInsert {
    FeedbackReplyInsert {
        parent_id: parent.id,
        user_id: admin_id,
        username: admin_username.to_string(),
        title: REPLY_TITLE.to_string(),
        r#type: parent.r#type,
        status: feedback_status::WAIT_REPLY,
        content: blank_to_default(content, ""),
        add_time: now,
        feedback_pics: to_feedback_pics(pics),
        // Java 总是 `setEmail(blankToDefault(...))` ⇒ 空串也要写列
        email: Some(blank_to_default(email, "")),
        mobile: Some(blank_to_default(mobile, "")),
        // 继承主留言；主留言该列为 NULL 时**不写**该列（NOT_NULL 策略）
        product_id: parent.product_id,
        order_id: parent.order_id,
        shop_id: parent.shop_id,
    }
}

/// 组装「编辑回复」的部分更新补丁（对应 Java `update` 的 `parentId != 0` 分支）。
///
/// 只改 Java 显式 `setXxx` 的列；`product_id` / `order_id` / `shop_id` 继承主留言，
/// 主留言该列为 `NULL` 时**不写**该列（保留旧值，等价 Java `NOT_NULL` 策略）。
#[must_use]
fn build_reply_patch(
    parent: &Feedback,
    admin_id: i32,
    admin_username: &str,
    content: Option<&str>,
    pics: Option<&[String]>,
    email: Option<&str>,
    mobile: Option<&str>,
) -> FeedbackReplyPatch {
    FeedbackReplyPatch {
        user_id: Some(Some(admin_id)),
        username: Some(Some(admin_username.to_string())),
        email: Some(Some(blank_to_default(email, ""))),
        mobile: Some(Some(blank_to_default(mobile, ""))),
        content: Some(Some(blank_to_default(content, ""))),
        feedback_pics: Some(Some(to_feedback_pics(pics))),
        r#type: Some(Some(parent.r#type)),
        product_id: Some(parent.product_id),
        order_id: Some(parent.order_id),
        shop_id: Some(parent.shop_id),
    }
}

/// `HashSet<i32>` → 升序 `Vec<i32>`（保证 `IN (...)` 绑定顺序稳定）。
#[must_use]
fn sorted_ids(ids: &HashSet<i32>) -> Vec<i32> {
    let mut v: Vec<i32> = ids.iter().copied().collect();
    v.sort_unstable();
    v
}

/// 当前秒级时间戳（Java `StringUtils.getCurrentTime()` = `currentTimeMillis()/1000`）。
#[must_use]
fn now_seconds_i32() -> i32 {
    i32::try_from(utils::now_seconds()).unwrap_or(i32::MAX)
}

/// 「留言不存在」（Java `GlobalException("留言不存在")`，码 `1001`）。
#[must_use]
fn feedback_not_found() -> Error {
    Error::biz(SERVICE_DATA_ERROR, MSG_FEEDBACK_NOT_FOUND)
}

/// 「无权操作该留言」（Java `GlobalException("无权操作该留言")`，码 `1001`）。
#[must_use]
fn no_permission() -> Error {
    Error::biz(SERVICE_DATA_ERROR, MSG_NO_PERMISSION_FEEDBACK)
}

// 事务提交已内联到 del/update/batch 的调用处（tx.commit() 是 sqlx::Transaction 的固有方法，
// 避免 services 层直接依赖 sqlx —— 事务边界仍由 infrastructure 的 repo.begin() 提供）。

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::member::feedback::feedback_type;

    fn feedback(id: i32, r#type: i32, shop_id: Option<i32>) -> Feedback {
        Feedback {
            id,
            parent_id: 0,
            user_id: 9,
            username: "buyer".into(),
            email: None,
            mobile: None,
            title: "标题".into(),
            r#type,
            status: feedback_status::WAIT_REPLY,
            content: "内容".into(),
            add_time: 1_700_000_000,
            feedback_pics: None,
            product_id: Some(0),
            order_id: Some(0),
            complaint_info: None,
            shop_id,
        }
    }

    // ————————————— 鉴权判定 —————————————

    #[test]
    fn platform_rejects_shop_feedback_with_nonzero_shop() {
        let platform = TenantScope::Platform;
        // type = 2（商品）+ shop_id != 0 → 拒绝
        assert!(scope_denied(feedback_type::PRODUCT, Some(3), platform));
        assert!(scope_denied(feedback_type::PRODUCT, Some(-1), platform));
        // shop_id = 0 或为 NULL → 放行
        assert!(!scope_denied(feedback_type::PRODUCT, Some(0), platform));
        // Java uses `!Objects.equals(getShopId(), 0)`; Objects.equals(null, 0) == false
        // => shop_id = NULL is also denied.
        assert!(scope_denied(feedback_type::PRODUCT, None, platform));
        // 非店铺类型一律放行
        for t in [
            feedback_type::SUGGESTION,
            feedback_type::COMPLAINT,
            feedback_type::OTHER,
            feedback_type::SHOP_COMPLAINT,
            feedback_type::ORDER_PROBLEM,
            feedback_type::ORDER_CONSULTATION,
        ] {
            assert!(!scope_denied(t, Some(3), platform), "type={t}");
        }
    }

    #[test]
    fn shop_only_sees_its_own_product_feedback() {
        let shop = TenantScope::Shop { shop_id: 7 };
        assert!(!scope_denied(feedback_type::PRODUCT, Some(7), shop));
        // 不是本店
        assert!(scope_denied(feedback_type::PRODUCT, Some(8), shop));
        // shop_id 为 NULL
        assert!(scope_denied(feedback_type::PRODUCT, None, shop));
        // 非店铺类型
        assert!(scope_denied(feedback_type::SUGGESTION, Some(7), shop));
    }

    // ————————————— 新增回复 —————————————

    #[test]
    fn insert_reply_inherits_parent_and_defaults() {
        let mut parent = feedback(5, feedback_type::PRODUCT, Some(3));
        parent.product_id = Some(11);
        parent.order_id = None;

        let insert = build_reply_insert(
            &parent,
            42,
            "admin",
            Some("  回复内容  "),
            Some(&["a.jpg".to_string()]),
            Some("   "),
            None,
            1_700_000_500,
        );
        assert_eq!(insert.parent_id, 5);
        assert_eq!(insert.user_id, 42);
        assert_eq!(insert.username, "admin");
        assert_eq!(insert.title, "reply");
        assert_eq!(insert.r#type, feedback_type::PRODUCT);
        assert_eq!(insert.status, feedback_status::WAIT_REPLY);
        // blankToDefault 不 trim：非空白保留原文
        assert_eq!(insert.content, "  回复内容  ");
        assert_eq!(insert.add_time, 1_700_000_500);
        assert_eq!(insert.feedback_pics, r#"["a.jpg"]"#);
        // 空白邮箱 → 空串（仍写列）
        assert_eq!(insert.email.as_deref(), Some(""));
        assert_eq!(insert.mobile.as_deref(), Some(""));
        assert_eq!(insert.product_id, Some(11));
        // 主留言 order_id 为 NULL ⇒ 不写该列
        assert_eq!(insert.order_id, None);
        assert_eq!(insert.shop_id, Some(3));
    }

    #[test]
    fn insert_reply_defaults_blank_content_to_empty_string_and_pics_to_empty_array() {
        let parent = feedback(5, feedback_type::SUGGESTION, Some(0));
        let insert = build_reply_insert(
            &parent,
            1,
            "a",
            None,
            None,
            None,
            None,
            1,
        );
        assert_eq!(insert.content, "");
        assert_eq!(insert.feedback_pics, "[]");
        assert_eq!(insert.product_id, Some(0));
        assert_eq!(insert.shop_id, Some(0));
    }

    // ————————————— 编辑回复 —————————————

    #[test]
    fn patch_writes_all_edited_columns() {
        let mut parent = feedback(5, feedback_type::ORDER_PROBLEM, None);
        parent.product_id = None;
        parent.order_id = Some(77);
        parent.shop_id = Some(9);

        let patch = build_reply_patch(
            &parent,
            42,
            "admin",
            Some("c"),
            None,
            Some("e"),
            Some("m"),
        );
        assert!(!patch.is_empty());
        assert_eq!(patch.user_id, Some(Some(42)));
        assert_eq!(patch.username, Some(Some("admin".to_string())));
        assert_eq!(patch.email, Some(Some("e".to_string())));
        assert_eq!(patch.mobile, Some(Some("m".to_string())));
        assert_eq!(patch.content, Some(Some("c".to_string())));
        assert_eq!(patch.feedback_pics, Some(Some("[]".to_string())));
        assert_eq!(patch.r#type, Some(Some(feedback_type::ORDER_PROBLEM)));
        // 主留言 product_id 为 NULL ⇒ 补丁 Some(None) ⇒ 不写列（保留旧值）
        assert_eq!(patch.product_id, Some(None));
        assert_eq!(patch.order_id, Some(Some(77)));
        assert_eq!(patch.shop_id, Some(Some(9)));
    }

    // ————————————— 杂项 —————————————

    #[test]
    fn sorted_ids_is_ascending_and_deduped() {
        let set: HashSet<i32> = [5, 1, 3, 1].into_iter().collect();
        assert_eq!(sorted_ids(&set), vec![1, 3, 5]);
        assert!(sorted_ids(&HashSet::new()).is_empty());
    }

    #[test]
    fn now_seconds_i32_is_reasonable() {
        let now = now_seconds_i32();
        assert!(now > 1_500_000_000, "now={now}");
        assert!(now > 0);
    }

    #[test]
    fn error_helpers_carry_java_code_and_message() {
        assert_eq!(feedback_not_found().business_code(), SERVICE_DATA_ERROR);
        assert_eq!(feedback_not_found().to_string(), format!("业务错误 [{SERVICE_DATA_ERROR}]: 留言不存在"));
        assert_eq!(no_permission().business_code(), SERVICE_DATA_ERROR);
        assert!(no_permission().to_string().contains(MSG_NO_PERMISSION_FEEDBACK));
        assert!(Error::biz(SERVICE_DATA_ERROR, MSG_ADMIN_NOT_FOUND)
            .to_string()
            .contains(MSG_ADMIN_NOT_FOUND));
        assert!(Error::biz(SERVICE_DATA_ERROR, MSG_REPLY_NOT_FOUND)
            .to_string()
            .contains(MSG_REPLY_NOT_FOUND));
        assert!(Error::biz(SERVICE_DATA_ERROR, MSG_REPLY_NOT_BELONG)
            .to_string()
            .contains(MSG_REPLY_NOT_BELONG));
    }
}