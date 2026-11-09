//! 会员标签组管理服务（member::member_tag_group）。
//!
//! 对应原 Java `com.nextrade.service.user.impl.MemberTagGroupServiceImpl`（318 行）
//! + `com.nextrade.adminapi.controller.user.MemberTagGroupController`（81 行）
//! 的 6 条管理端链路：
//!
//! | 本域端点（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |--------------------------------|---------|------|
//! | `GET  /user/memberTagGroup/detail` | `MemberTagGroupController.detail` | [`MemberTagGroupService::detail`] |
//! | `POST /user/memberTagGroup/create` | `MemberTagGroupController.create` | [`MemberTagGroupService::create`] |
//! | `POST /user/memberTagGroup/update` | `MemberTagGroupController.update` | [`MemberTagGroupService::update`] |
//! | `POST /user/memberTagGroup/del` | `MemberTagGroupController.del` | [`MemberTagGroupService::del_group`] |
//! | `POST /user/memberTagGroup/setTop` | `MemberTagGroupController.setTop` | [`MemberTagGroupService::set_top`] |
//! | `POST /user/memberTagGroup/cancelTop` | `MemberTagGroupController.cancelTop` | [`MemberTagGroupService::cancel_top`] |
//!
//! > `GET /adminapi/user/memberTagGroup/list` **不在本切片**：它已由
//! > `crates/api/admin/src/routes/member.rs` 注册（早期域占位实现），
//! > 本域不重复注册（重复的 `(method, path)` 会让 axum 启动时 panic）。
//!
//! # 迁移要点（逐条对照 Java 源）
//!
//! ## `shopId` 从哪来（**核心语义**）
//!
//! Java 的 `currentShopId()` 读**请求头** `HeaderUtils.getShopId()`：
//! ```text
//! Integer s = HeaderUtils.getShopId();
//! return s == null ? 0 : s;      // null → 0（平台）
//! ```
//! 它**不是**请求体/查询参数，前端无法通过 JSON 覆盖。本域 6 个方法
//! **每一个**都先取它，并作为所有 SQL 的店铺过滤/归属校验条件 ——
//! 这就是「店铺隔离（SHOP-MEMBER-20260708）」的实现方式。
//!
//! Rust 侧 JWT 尚未签发 `shop_id`，故由路由层按同一规则解析请求头后
//! 作为**显式入参**传入（与 product_stock / stock_warning / aftersales 一致）。
//!
//! ## 归属校验：三重判定 → **同一条**文案
//!
//! `detail` / `update` / `delGroup` / `setTop` / `cancelTop` 五处都是：
//! ```text
//! if (group == null || group.getShopId() == null || group.getShopId() != shopId) {
//!     throw new GlobalException("标签组不存在");
//! }
//! ```
//! 三者（不存在 / `shopId` 为 null / 不属于本店铺）**统一**抛
//! `标签组不存在`，不区分「不存在」与「存在但不是你的」
//! —— 以免通过错误文案枚举他人店铺的标签组 ID。本实现用
//! [`group_is_owned`] 复刻该判定。
//!
//! 其中 `group.getShopId() == null` 在 Rust 侧**类型上不可表达**
//! （`member_tag_group.shop_id` 是 `NOT NULL`，实体为 `i32`），
//! 属于已消失的分支。
//!
//! ## `create`（Java 190–206 行）
//!
//! ```text
//! MemberTagGroup group = new MemberTagGroup();
//! group.setShopId(shopId);
//! group.setGroupName(param.getGroupName());   // 注意：不 trim
//! getBaseMapper().insert(group);
//! for (String tagName : param.getTagNames()) {
//!     MemberTag tag = new MemberTag();
//!     tag.setShopId(shopId);
//!     tag.setGroupId(group.getId());          // 依赖 insert 回填的自增主键
//!     tag.setTagName(tagName.trim());         // 注意：trim
//!     memberTagMapper.insert(tag);
//! }
//! ```
//! ⚠️ **先插组、拿到自增主键、再逐条插标签**，全部在同一个
//! `@Transactional(rollbackFor = Exception.class)` 里 —— 因此本实现必须
//! 用事务（否则插组成功而插标签失败会留下空组）。
//!
//! ⚠️ `groupName` **不 trim**，`tagName` **要 trim**（Java 源码就是
//! 这种不对称写法，已逐字符复刻）。
//!
//! ⚠️ Java **没有**设置 `isTop`（实体字段为 null ⇒ NOT_NULL 策略跳过该列
//! ⇒ MySQL 列默认值 `0` 生效）；本实现显式写 `0`，净结果相同。
//!
//! ## `update`（Java 208–260 行）
//!
//! ```text
//! group = selectById(param.getId());  // 归属校验，失败抛「标签组不存在」
//! group.setGroupName(param.getGroupName());
//! updateById(group);
//!
//! existingTags = selectList(eq(shopId).eq(groupId, id));
//! existingMap  = {id: tag};
//! submittedIds = param.getTags().stream().map(getId).filter(nonNull).toSet();
//!
//! // ① 删除：不在提交列表里的已有标签（连带物理删除 user_member_tag）
//! existingTags.filter(t -> !submittedIds.contains(t.getId())).forEach(t -> {
//!     memberTagMapper.deleteById(t.getId());                 // 逻辑删除
//!     userMemberTagMapper.delete(eq(shopId).eq(tagId, t.getId()));  // 物理删除
//! });
//!
//! // ② 有 ID → 改名；无 ID → 新增
//! for (item : param.getTags()) {
//!     name = item.getTagName().trim();
//!     if (item.getId() != null) {
//!         existing = existingMap.get(item.getId());
//!         if (existing != null && !existing.getTagName().equals(name)) {
//!             existing.setTagName(name);
//!             memberTagMapper.updateById(existing);
//!         }
//!     } else {
//!         // 新增
//!     }
//! }
//! ```
//!
//! ⚠️ 顺序不能变：**先删除、再改名/新增**（本实现由
//! [`plan_tag_sync`] 一次性算出三段动作，再按「删除 → 改名 → 新增」
//! 的等价顺序执行；删除集合与提交集合天然互斥，故顺序无副作用差异）。
//!
//! ⚠️ 提交了**不属于本组**的标签 ID 时：`existingMap.get(id)` 为 null
//! ⇒ **静默忽略**（既不新增也不报错）。这是 Java 的真实行为。
//!
//! ## `delGroup`（Java 262–290 行）
//!
//! ```text
//! group = selectById(id);            // 归属校验
//! tags  = selectList(eq(shopId).eq(groupId, id));
//! tagIds = tags.map(getId);
//! if (!tagIds.isEmpty()) {
//!     userMemberTagMapper.delete(eq(shopId).in(tagId, tagIds));  // 物理删除
//! }
//! tags.forEach(t -> memberTagMapper.deleteById(t.getId()));      // 逻辑删除
//! getBaseMapper().deleteById(id);                                // 逻辑删除
//! ```
//!
//! ⚠️ 注释里写的「逻辑删除」与实际一致（`@TableLogic` ⇒ `SET is_del = 1`），
//! 但**清 `user_member_tag` 是物理 `DELETE`**（该表无 `is_del` 列）。
//! 这一非对称性必须保留。
//!
//! ⚠️ `delGroup` **不清理**被删标签在 `member_tag` 之外的其他引用；
//! 本实现同样不做。
//!
//! ## `setTop`（Java 292–305 行）
//!
//! ```text
//! group = selectById(id);                    // 归属校验
//! Integer maxTop = selectMaxIsTop(shopId);   // SELECT MAX(is_top) WHERE is_del=0 AND shop_id=?
//! int nextTop = (maxTop == null ? 0 : maxTop) + 1;
//! group.setIsTop(nextTop);
//! updateById(group);
//! ```
//!
//! ⚠️ **不是**「置顶权重 = 1」：而是「当前店铺最大值 + 1」，因此重复置顶
//! 同一组会让权重持续增大。本实现逐字符复刻（见 [`next_top`]）。
//!
//! ⚠️ 并发下两个请求可能拿到同一个 `maxTop` 从而得到相同权重
//! （Java 也没有锁/唯一约束）。本实现在事务内取 `MAX` 并写入，
//! 但 MySQL 默认隔离级别（REPEATABLE READ）下 `SELECT MAX` 是**快照读**，
//! 仍可能重号 —— 与 Java 行为一致，**不做**额外加锁（见 `uncertain_points`）。
//!
//! ## `cancelTop`（Java 307–317 行）
//!
//! ```text
//! group = selectById(id);   // 归属校验
//! group.setIsTop(0);        // 直接归零，不重排其它组的权重
//! updateById(group);
//! ```
//!
//! ⚠️ 归零后**不重排**其余组的权重，因此取消置顶会留下权重空洞
//! （例如 1,3 → 取消 1 后仍是 0,3）。这是 Java 的真实行为。
//!
//! # 未生效的「冻结守卫」
//!
//! `MemberTagGroupServiceImpl` 注入了
//! `ShopMemberSettingService shopMemberSettingService`，类注释也写着
//! 「店铺端写操作受总开关冻结守卫约束」，但该类**六个方法里一次都没调用它**
//! （全类唯一出现处是第 50 行的字段声明）。
//! 本实现**不实现**冻结守卫（复刻 Java 的可观测行为），见 `uncertain_points`。
//!
//! # 金额
//!
//! 本域**不涉及任何金额**。
//!
//! # 事务
//!
//! Java 的 `create` / `update` / `delGroup` / `setTop` / `cancelTop` 五个
//! 方法都带 `@Transactional(rollbackFor = Exception.class)`，且都是
//! 「先查后改」的多语句组合。本实现在服务层用
//! [`nextrade_infrastructure::db::repo::member::member_tag_group::MemberTagGroupRepository::begin`]
//! 开启事务并编排，出错即 drop（自动 rollback），与 Java 语义一致。

use std::sync::Arc;

use nextrade_core::models::member::member_tag_group::{
    next_top, plan_tag_sync, validate_create, validate_update, AdminOperator,
    MemberTagGroupCreateDTO, MemberTagGroupDetailVO, MemberTagGroupUpdateDTO, MemberTagVO,
    OperateDTO, DEFAULT_SHOP_ID, MSG_GROUP_NOT_FOUND, MSG_ID_REQUIRED,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::member::member_tag_group::{to_existing_tags, MemberTagGroupRepository};
use nextrade_infrastructure::db::Repositories;

/// 会员标签组管理域服务（挂载于 `AdminState.member_tag_group`）。
pub struct MemberTagGroupService {
    repos: Arc<Repositories>,
}

impl MemberTagGroupService {
    /// 构造（与 member_tag / tax_rate / area_code 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 仓储引用（便于测试与后续切片复用）。
    #[must_use]
    pub fn repo(&self) -> &MemberTagGroupRepository {
        &self.repos.member.member_tag_group
    }

    // ————————————————————— 详情 —————————————————————

    /// 标签组详情（对应 Java `MemberTagGroupServiceImpl#detail`）。
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. `shopId = currentShopId()`
    /// 2. `selectById(id)`（自动带 `is_del = 0`）
    /// 3. 归属校验失败 → `标签组不存在`（404，同 Java `GlobalException` 文案）
    /// 4. 查组下标签 `selectList(eq(shopId).eq(groupId, id))` → `List<MemberTagVO>`
    ///
    /// ⚠️ 返回的 `tags` 里**每个**标签都带 `groupId`（Java `tagVO.setGroupId`），
    /// 不是只有组 ID。
    ///
    /// ⚠️ Java 无 `ORDER BY`；本实现用 `ORDER BY id ASC` 固定顺序
    /// （见 `uncertain_points`）。
    ///
    /// # Errors
    /// - 标签组不存在或不属于当前店铺 → `Error::NotFound(标签组不存在)`
    /// - 数据库失败 → `Error::Database`
    pub async fn detail(&self, id: i64, shop_id: i32) -> Result<MemberTagGroupDetailVO> {
        let group = self
            .repos.member.member_tag_group
            .find_group_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_GROUP_NOT_FOUND.into()))?;
        if !group_is_owned(Some(group.shop_id), shop_id) {
            return Err(Error::NotFound(MSG_GROUP_NOT_FOUND.into()));
        }

        let tags = self
            .repos.member.member_tag_group
            .list_tags_by_group(shop_id, id)
            .await?;
        let tag_vos: Vec<MemberTagVO> = tags.iter().map(MemberTagVO::from).collect();

        Ok(MemberTagGroupDetailVO::from_parts(
            group.id,
            &group.group_name,
            group.create_time,
            tag_vos,
        ))
    }

    // ————————————————————— 新增 —————————————————————

    /// 新增标签组（对应 Java `MemberTagGroupServiceImpl#create`）。
    ///
    /// 流程与 Java 逐条对齐（详见模块头）：
    /// 1. `@Valid` 校验（`groupName` `@NotBlank`；`tagNames` `@NotNull`
    ///    + `@Size(min = 2)`），文案逐字符照抄；
    /// 2. 开事务；
    /// 3. 插组（`shopId` 取当前店铺、`groupName` **不 trim**、`isTop = 0`、
    ///    `createTime` / `updateTime` / 操作人 / `isDel` 由本层填充）；
    /// 4. 用回填的自增主键逐条插标签（`tagName` 已 trim）；
    /// 5. 提交。
    ///
    /// Java 控制器返回 `void`；本实现返回新建的**标签组主键**，
    /// 便于前端跳转编辑（与 shop_manage / pc_cat_floor / area_code 的处理方式一致）。
    ///
    /// # Errors
    /// - 入参不合法 → `Error::Validation`（文案逐字符照抄 Java 注解）
    /// - 数据库失败 → `Error::Database`（事务自动回滚）
    pub async fn create(
        &self,
        dto: &MemberTagGroupCreateDTO,
        shop_id: i32,
        operator: &AdminOperator,
    ) -> Result<i64> {
        // 1. `@Valid MemberTagGroupCreateParam`
        let (group_name, tag_names) = validate_create(dto).map_err(Error::Validation)?;

        // Java 两次独立调用 `Instant.now().getEpochSecond()`；本实现取一次，
        // 保证组与其标签的时间戳一致（见 `uncertain_points`）。
        let now = now_secs();

        // 2–5. 同一事务：先插组拿主键，再逐条插标签（Java `@Transactional`）
        let mut tx = self.repos.member.member_tag_group.begin().await?;
        let group_id = self
            .repos.member.member_tag_group
            .insert_group(&mut tx, shop_id, &group_name, operator, now)
            .await?;
        for tag_name in &tag_names {
            self.repos.member.member_tag_group
                .insert_tag(&mut tx, shop_id, group_id, tag_name, operator, now)
                .await?;
        }
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let log_info = format!("新增标签组：{}", group_name);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_current_log(if shop_id > 0 { "shop" } else { "admin" }, shop_id, &log_info)
            .await
        {
            tracing::warn!("写管理操作日志失败: {e}");
        }

        tracing::info!(
            "新增会员标签组成功: id={group_id}, shopId={shop_id}, 标签数={}",
            tag_names.len()
        );
        Ok(group_id)
    }

    // ————————————————————— 更新 —————————————————————

    /// 更新标签组（含标签全量同步）
    /// （对应 Java `MemberTagGroupServiceImpl#update`）。
    ///
    /// 流程与 Java 逐条对齐（详见模块头）：
    /// 1. `@Valid` 校验（`id` / `groupName` / `tags` 必填、`tags` ≥ 2、
    ///    每个 `tagName` 非空白）；
    /// 2. 开事务；
    /// 3. `selectById(id)` + 归属校验 → 失败抛 `标签组不存在`；
    /// 4. 改名（`updateById(group)` 的净效果：只写 `group_name` +
    ///    `update_time` / `update_by*`）；
    /// 5. 查组下已有标签；
    /// 6. 按 [`plan_tag_sync`] 的计划执行：
    ///    - **删除**不在提交列表里的标签（逻辑删除 `member_tag`
    ///      + 物理删除其 `user_member_tag` 关联）
    ///    - **改名**（仅当名称确实变化）
    ///    - **新增**（提交项无 ID）
    /// 7. 提交。
    ///
    /// 返回可观测的计数（Java 控制器返回 `void`）。
    ///
    /// # Errors
    /// - 入参不合法 → `Error::Validation`
    /// - 标签组不存在或不属于当前店铺 → `Error::NotFound(标签组不存在)`
    /// - 数据库失败 → `Error::Database`（事务自动回滚）
    pub async fn update(
        &self,
        dto: &MemberTagGroupUpdateDTO,
        shop_id: i32,
        operator: &AdminOperator,
    ) -> Result<UpdateOutcome> {
        let (id, group_name, items) = validate_update(dto).map_err(Error::Validation)?;
        let now = now_secs();

        let mut tx = self.repos.member.member_tag_group.begin().await?;

        // 3. 归属校验（Java `selectById` + 三重判定）
        let group = self
            .repos.member.member_tag_group
            .find_group_by_id_in_tx(&mut tx, id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_GROUP_NOT_FOUND.into()))?;
        if !group_is_owned(Some(group.shop_id), shop_id) {
            return Err(Error::NotFound(MSG_GROUP_NOT_FOUND.into()));
        }

        // 4. `group.setGroupName(...); updateById(group)`
        self.repos.member.member_tag_group
            .update_group_name(&mut tx, id, &group_name, operator, now)
            .await?;

        // 5. 组下已有标签
        let existing_rows = self
            .repos.member.member_tag_group
            .list_tags_by_group_in_tx(&mut tx, shop_id, id)
            .await?;
        let existing = to_existing_tags(&existing_rows);

        // 6. 差异计算（纯函数，见 models::nextrade_core::models::member::member_tag_group::plan_tag_sync）
        let plan = plan_tag_sync(&existing, &items);

        // 6① 删除：逻辑删标签 + 物理清关联（顺序与 Java 一致：先清关联）
        for tag_id in &plan.delete_ids {
            self.repos.member.member_tag_group
                .delete_user_tag_links(&mut tx, shop_id, *tag_id)
                .await?;
            self.repos.member.member_tag_group
                .logical_delete_tag(&mut tx, *tag_id)
                .await?;
        }

        // 6② 改名（Java 只在名称变化时才 `updateById`）
        for (tag_id, name) in &plan.rename_ids {
            self.repos.member.member_tag_group
                .update_tag_name(&mut tx, *tag_id, name, operator, now)
                .await?;
        }

        // 6③ 新增（提交项无 ID）
        for name in &plan.insert_names {
            self.repos.member.member_tag_group
                .insert_tag(&mut tx, shop_id, id, name, operator, now)
                .await?;
        }

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let log_info = format!("更新标签组：{}", group_name);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_current_log(if shop_id > 0 { "shop" } else { "admin" }, shop_id, &log_info)
            .await
        {
            tracing::warn!("写管理操作日志失败: {e}");
        }

        let outcome = UpdateOutcome {
            group_id: id,
            deleted: u64::try_from(plan.delete_ids.len()).unwrap_or(u64::MAX),
            renamed: u64::try_from(plan.rename_ids.len()).unwrap_or(u64::MAX),
            inserted: u64::try_from(plan.insert_names.len()).unwrap_or(u64::MAX),
        };
        tracing::info!(
            "更新会员标签组: id={id}, 删除 {} / 改名 {} / 新增 {}",
            outcome.deleted,
            outcome.renamed,
            outcome.inserted
        );
        Ok(outcome)
    }

    // ————————————————————— 删除 —————————————————————

    /// 删除标签组（含其下所有标签与用户关联）
    /// （对应 Java `MemberTagGroupServiceImpl#delGroup`）。
    ///
    /// 流程与 Java 逐条对齐（详见模块头）：
    /// 1. `OperateDTO.id` 必填（Java 无 `@Valid`，null 会 NPE）；
    /// 2. 开事务；
    /// 3. `selectById(id)` + 归属校验 → 失败抛 `标签组不存在`；
    /// 4. 查组下标签 ID；
    /// 5. `tagIds` 非空时**物理删除** `user_member_tag`
    ///    （`eq(shopId).in(tagId, tagIds)`）；
    /// 6. 逐条**逻辑删除** `member_tag`；
    /// 7. **逻辑删除** `member_tag_group`；
    /// 8. 提交。
    ///
    /// ⚠️ 删除组本身**不会**检查组内是否还有标签 —— 直接连同标签一起删。
    ///
    /// # Errors
    /// - `id` 缺失 → `Error::Validation(id不能为空)`
    /// - 标签组不存在或不属于当前店铺 → `Error::NotFound(标签组不存在)`
    /// - 数据库失败 → `Error::Database`（事务自动回滚）
    pub async fn del_group(&self, dto: &OperateDTO, shop_id: i32) -> Result<DelOutcome> {
        // 1. Java 在 `operateDTO.getId().longValue()` 处对 null 抛 NPE（500）；
        //    本实现按项目统一口径返回 400（见 `uncertain_points`）。
        let id = dto
            .group_id()
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;

        let mut tx = self.repos.member.member_tag_group.begin().await?;

        // 3. 归属校验
        let group = self
            .repos.member.member_tag_group
            .find_group_by_id_in_tx(&mut tx, id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_GROUP_NOT_FOUND.into()))?;
        if !group_is_owned(Some(group.shop_id), shop_id) {
            return Err(Error::NotFound(MSG_GROUP_NOT_FOUND.into()));
        }

        // 4. 组下标签
        let tags = self
            .repos.member.member_tag_group
            .list_tags_by_group_in_tx(&mut tx, shop_id, id)
            .await?;
        let tag_ids: Vec<i64> = tags.iter().map(|t| t.id).collect();

        // 5. 清理用户关联（物理删除；`tag_ids` 为空时仓储层直接返回 0，
        //    复刻 Java `if (!tagIds.isEmpty())` 守卫）
        let removed_links = self
            .repos.member.member_tag_group
            .delete_user_tag_links_in(&mut tx, shop_id, &tag_ids)
            .await?;

        // 6. 逻辑删除标签
        let mut deleted_tags = 0_u64;
        for tag_id in &tag_ids {
            deleted_tags += self
                .repos.member.member_tag_group
                .logical_delete_tag(&mut tx, *tag_id)
                .await?;
        }

        // 7. 逻辑删除标签组
        let deleted_group = self
            .repos.member.member_tag_group
            .logical_delete_group(&mut tx, id)
            .await?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let log_info = format!("删除标签组：{}", id);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_current_log(if shop_id > 0 { "shop" } else { "admin" }, shop_id, &log_info)
            .await
        {
            tracing::warn!("写管理操作日志失败: {e}");
        }

        tracing::info!(
            "删除会员标签组: id={id}, 标签 {deleted_tags} 个, 清理关联 {removed_links} 行"
        );
        Ok(DelOutcome {
            group_id: id,
            removed_links,
            deleted_tags,
            deleted_group: deleted_group > 0,
        })
    }

    // ————————————————————— 置顶 / 取消置顶 —————————————————————

    /// 置顶标签组（对应 Java `MemberTagGroupServiceImpl#setTop`）。
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. 开事务；
    /// 2. `selectById(id)` + 归属校验 → 失败抛 `标签组不存在`；
    /// 3. `selectMaxIsTop(shopId)`（`SELECT MAX(is_top) ... WHERE is_del = 0
    ///    AND shop_id = ?`，无行时 `NULL` → `0`）；
    /// 4. `nextTop = maxTop + 1`，`updateById(group)`；
    /// 5. 提交。
    ///
    /// 返回写入的置顶权重（Java 控制器返回 `void`）。
    ///
    /// ⚠️ **不是**「置顶权重 = 1」：重复置顶同一组会让权重持续增大
    /// （Java 真实行为，逐字符复刻）。
    ///
    /// # Errors
    /// - `id` 缺失 → `Error::Validation(id不能为空)`
    /// - 标签组不存在或不属于当前店铺 → `Error::NotFound(标签组不存在)`
    /// - 数据库失败 → `Error::Database`
    pub async fn set_top(
        &self,
        dto: &OperateDTO,
        shop_id: i32,
        operator: &AdminOperator,
    ) -> Result<i32> {
        let id = dto
            .group_id()
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let now = now_secs();

        let mut tx = self.repos.member.member_tag_group.begin().await?;

        let group = self
            .repos.member.member_tag_group
            .find_group_by_id_in_tx(&mut tx, id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_GROUP_NOT_FOUND.into()))?;
        if !group_is_owned(Some(group.shop_id), shop_id) {
            return Err(Error::NotFound(MSG_GROUP_NOT_FOUND.into()));
        }

        // `selectMaxIsTop` 与写入同事务（Java 也在同一 `@Transactional` 内）
        let max_top = self
            .repos.member.member_tag_group
            .select_max_is_top(&mut tx, shop_id)
            .await?;
        let top = next_top(max_top);

        self.repos.member.member_tag_group
            .update_is_top(&mut tx, id, top, operator, now)
            .await?;
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let log_info = format!("置顶标签组：{}", id);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_current_log(if shop_id > 0 { "shop" } else { "admin" }, shop_id, &log_info)
            .await
        {
            tracing::warn!("写管理操作日志失败: {e}");
        }

        tracing::info!("置顶会员标签组: id={id}, isTop={top}, shopId={shop_id}");
        Ok(top)
    }

    /// 取消置顶标签组（对应 Java `MemberTagGroupServiceImpl#cancelTop`）。
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. 开事务；
    /// 2. `selectById(id)` + 归属校验 → 失败抛 `标签组不存在`；
    /// 3. `setIsTop(0)`，`updateById(group)`；
    /// 4. 提交。
    ///
    /// ⚠️ **不重排**其余组的权重（Java 真实行为）：取消置顶会留下权重空洞。
    ///
    /// # Errors
    /// - `id` 缺失 → `Error::Validation(id不能为空)`
    /// - 标签组不存在或不属于当前店铺 → `Error::NotFound(标签组不存在)`
    /// - 数据库失败 → `Error::Database`
    pub async fn cancel_top(
        &self,
        dto: &OperateDTO,
        shop_id: i32,
        operator: &AdminOperator,
    ) -> Result<u64> {
        let id = dto
            .group_id()
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let now = now_secs();

        let mut tx = self.repos.member.member_tag_group.begin().await?;

        let group = self
            .repos.member.member_tag_group
            .find_group_by_id_in_tx(&mut tx, id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_GROUP_NOT_FOUND.into()))?;
        if !group_is_owned(Some(group.shop_id), shop_id) {
            return Err(Error::NotFound(MSG_GROUP_NOT_FOUND.into()));
        }

        let affected = self
            .repos.member.member_tag_group
            .update_is_top(&mut tx, id, CANCEL_TOP_VALUE, operator, now)
            .await?;
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let log_info = format!("取消置顶标签组：{}", id);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_current_log(if shop_id > 0 { "shop" } else { "admin" }, shop_id, &log_info)
            .await
        {
            tracing::warn!("写管理操作日志失败: {e}");
        }

        tracing::info!("取消置顶会员标签组: id={id}, shopId={shop_id}");
        Ok(affected)
    }

    // ————————————————————— 供路由层复用的纯逻辑 —————————————————————

    /// 路由层在无法解析店铺时的默认店铺 ID（平台 0）。
    ///
    /// Java `currentShopId()` 在 `HeaderUtils.getShopId() == null` 时归一为
    /// `0`；本实现由路由层按请求头解析，缺失时取本常量。
    #[must_use]
    pub const fn default_shop_id() -> i32 {
        DEFAULT_SHOP_ID
    }
}

/// `cancelTop` 写入的权重值（Java `group.setIsTop(0)`）。
pub const CANCEL_TOP_VALUE: i32 = 0;

/// 归属判定（Java `group == null || group.getShopId() == null
/// || group.getShopId() != shopId` 的等价物）。
///
/// - `group_shop_id`：`None` 表示「组不存在」（或 `shop_id` 为 null，
///   在 Rust 侧类型上不可表达）
/// - 返回 `true` 仅当组存在**且** `shop_id` 与当前店铺**值相等**
///
/// 五处调用点（`detail` / `update` / `delGroup` / `setTop` / `cancelTop`）
/// 都把 `false` 映射为**同一条** `标签组不存在`，不区分
/// 「不存在」与「不属于你」。
#[must_use]
pub const fn group_is_owned(group_shop_id: Option<i32>, shop_id: i32) -> bool {
    match group_shop_id {
        Some(owner) => owner == shop_id,
        None => false,
    }
}

/// `update` 的执行结果（Java 返回 `void`，此处回传可观测的计数）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateOutcome {
    /// 被更新的标签组主键
    pub group_id: i64,
    /// 被逻辑删除的标签数
    pub deleted: u64,
    /// 被改名的标签数
    pub renamed: u64,
    /// 新增的标签数
    pub inserted: u64,
}

/// `delGroup` 的执行结果（Java 返回 `void`，此处回传可观测的计数）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelOutcome {
    /// 被删除的标签组主键
    pub group_id: i64,
    /// 被**物理**删除的 `user_member_tag` 关联行数
    pub removed_links: u64,
    /// 被**逻辑**删除的 `member_tag` 行数
    pub deleted_tags: u64,
    /// `member_tag_group` 是否被逻辑删除（受影响行数 > 0）
    pub deleted_group: bool,
}

// 事务提交说明（不写成辅助函数）
//
// ⚠️ 本 crate **不直接依赖 `sqlx`**（见 `crates/services/Cargo.toml`），
// 因此**无法**写出 `sqlx::Transaction` 这样的类型名来做 `async fn commit(tx)`
// 辅助函数 —— 只能在每个调用点用「由仓储 `begin()` 推导出的 `tx`」直接
// `tx.commit().await.map_err(...)`，与 user_company / message_type 的写法一致。
//
// 出错时**不要**显式 rollback：`tx` 在 drop 时会自动 rollback
// （与 Java `@Transactional(rollbackFor = Exception.class)` 一致）。

/// 当前 Unix 秒（对应 Java `Instant.now().getEpochSecond()`
/// —— `MybatisPlusMetaObjectHandler` 的 `createTime` / `updateTime` 值）。
fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::member::member_tag_group::{
        ExistingTag, MemberTagItemDTO, TagItem, MSG_GROUP_ID_REQUIRED, MSG_GROUP_NAME_REQUIRED,
        MSG_TAG_NAME_REQUIRED, MSG_TAG_NAMES_MIN, MSG_TAGS_MIN,
    };

    // ————————————————————— 归属判定（三重判定 → 同一条文案） —————————————————————

    /// Java `group == null || group.getShopId() == null
    /// || group.getShopId() != shopId` 的三重判定：
    /// 五处调用点必须把三种失败情形归一到**同一条**文案。
    #[test]
    fn ownership_check_is_three_way_and_uniform() {
        // ① 组不存在（Java `group == null`；也覆盖 `getShopId() == null`
        //    在 Rust 侧不可表达的情形）
        assert!(!group_is_owned(None, 0));
        assert!(!group_is_owned(None, 7));
        // ② 平台组被店铺账号访问（0 != 7）
        assert!(!group_is_owned(Some(0), 7));
        // ③ 店铺 A 访问店铺 B 的组
        assert!(!group_is_owned(Some(3), 7));
        // ④ 归属一致才通过
        assert!(group_is_owned(Some(7), 7));
        // ⑤ 平台账号访问平台组
        assert!(group_is_owned(Some(0), 0));

        // 三种失败情形映射到同一条文案（不泄露「存在但不属于你」）
        for owner in [None, Some(0), Some(3)] {
            if !group_is_owned(owner, 7) {
                let msg = MSG_GROUP_NOT_FOUND;
                assert_eq!(msg, "标签组不存在");
            }
        }
    }

    // ————————————————————— 文案 —————————————————————

    /// 六条校验文案 + 不存在文案 + `OperateDTO` 文案逐字符对照 Java。
    #[test]
    fn message_constants_match_java_verbatim() {
        assert_eq!(MSG_GROUP_NAME_REQUIRED, "组名称不能为空");
        assert_eq!(MSG_GROUP_ID_REQUIRED, "标签组ID不能为空");
        assert_eq!(MSG_TAG_NAME_REQUIRED, "标签名称不能为空");
        assert_eq!(MSG_TAG_NAMES_MIN, "至少填写两个标签值");
        assert_eq!(MSG_TAGS_MIN, "至少保留两个标签");
        assert_eq!(MSG_ID_REQUIRED, "id不能为空");
        assert_eq!(MSG_GROUP_NOT_FOUND, "标签组不存在");
    }

    // ————————————————————— 状态流转 —————————————————————

    /// `cancelTop` 写入 0（Java `group.setIsTop(0)`）。
    #[test]
    fn cancel_top_value_is_zero() {
        assert_eq!(CANCEL_TOP_VALUE, 0);
    }

    /// `setTop` 的权重递推：`(maxTop == null ? 0 : maxTop) + 1`
    /// —— 不是固定 1，重复置顶会持续增大。
    #[test]
    fn set_top_weight_increments_from_shop_max() {
        assert_eq!(next_top(None), 1, "空店铺首个置顶权重应为 1");
        assert_eq!(next_top(Some(1)), 2, "已有权重 1 → 新置顶为 2");
        assert_eq!(next_top(Some(9)), 10);
        // 与「固定置顶 = 1」的误读形成对照
        assert_ne!(next_top(Some(9)), 1);
    }

    /// 取消置顶**不重排**其余组：3 → 0 后剩下的权重集合仍有空洞
    /// （Java 真实行为，本测试固化该语义，防止有人「顺手重排」）。
    #[test]
    fn cancel_top_leaves_weight_gaps() {
        let before = [1_i32, 3];
        let after: Vec<i32> = before
            .iter()
            .map(|v| if *v == 1 { CANCEL_TOP_VALUE } else { *v })
            .collect();
        assert_eq!(after, [0, 3]);
        // 没有重排 ⇒ 不是 [0, 1]
        assert_ne!(after, [0, 1]);
    }

    // ————————————————————— 标签同步（update 的三段动作） —————————————————————

    fn existing(id: i64, name: &str) -> ExistingTag {
        ExistingTag {
            id,
            tag_name: name.to_string(),
        }
    }

    fn item(id: Option<i64>, name: &str) -> TagItem {
        TagItem {
            id,
            tag_name: name.to_string(),
        }
    }

    /// 一次 `update` 同时触发「删除 + 改名 + 新增」，计数必须与 Java 的
    /// 三段循环一致。
    #[test]
    fn update_outcome_counts_match_java_three_phases() {
        let cur = vec![existing(1, "A"), existing(2, "B")];
        // 1 改名；2 未提交 → 删除；D / E 新增
        let submitted = vec![item(Some(1), "A2"), item(None, "D"), item(None, "E")];
        let plan = plan_tag_sync(&cur, &submitted);

        let outcome = UpdateOutcome {
            group_id: 9,
            deleted: u64::try_from(plan.delete_ids.len()).unwrap(),
            renamed: u64::try_from(plan.rename_ids.len()).unwrap(),
            inserted: u64::try_from(plan.insert_names.len()).unwrap(),
        };
        assert_eq!(outcome.deleted, 1, "2 未提交 → 删除");
        assert_eq!(outcome.renamed, 1, "1 改名");
        assert_eq!(outcome.inserted, 2, "D / E 新增");
        assert_eq!(outcome.group_id, 9);
    }

    /// 提交内容与库中完全一致时三个计数都为 0（不产生任何写 SQL）。
    #[test]
    fn update_outcome_is_zero_when_nothing_changes() {
        let cur = vec![existing(1, "A"), existing(2, "B")];
        let submitted = vec![item(Some(1), "A"), item(Some(2), "B")];
        let plan = plan_tag_sync(&cur, &submitted);
        assert_eq!(plan.delete_ids.len(), 0);
        assert_eq!(plan.rename_ids.len(), 0);
        assert_eq!(plan.insert_names.len(), 0);
    }

    /// `update` 校验失败时**不产生**任何写操作（返回 Err，事务都未开启）。
    #[test]
    fn update_validation_rejects_before_any_write() {
        // tags 少于 2 个
        let dto = MemberTagGroupUpdateDTO {
            id: Some(1),
            group_name: Some("g".into()),
            tags: Some(vec![MemberTagItemDTO {
                id: None,
                tag_name: Some("A".into()),
            }]),
        };
        assert_eq!(
            validate_update(&dto).unwrap_err(),
            MSG_TAGS_MIN,
            "少于两个标签必须被拒"
        );

        // 标签名空白
        let dto = MemberTagGroupUpdateDTO {
            id: Some(1),
            group_name: Some("g".into()),
            tags: Some(vec![
                MemberTagItemDTO {
                    id: None,
                    tag_name: Some("A".into()),
                },
                MemberTagItemDTO {
                    id: None,
                    tag_name: Some("  ".into()),
                },
            ]),
        };
        assert_eq!(validate_update(&dto).unwrap_err(), MSG_TAG_NAME_REQUIRED);
    }

    // ————————————————————— 结果结构 —————————————————————

    /// `delGroup` 的结果结构：默认即「什么都没删」。
    #[test]
    fn del_outcome_shape() {
        let o = DelOutcome {
            group_id: 1,
            removed_links: 0,
            deleted_tags: 0,
            deleted_group: false,
        };
        assert_eq!(o.removed_links, 0);
        assert!(!o.deleted_group);

        let o = DelOutcome {
            group_id: 1,
            removed_links: 3,
            deleted_tags: 2,
            deleted_group: true,
        };
        assert_eq!(o.removed_links, 3);
        assert_eq!(o.deleted_tags, 2);
        assert!(o.deleted_group);
    }

    /// `deleted_group` 的判据是 `rows_affected > 0`
    /// （Java `deleteById` 返回的 `SqlHelper.retBool(rows)`）。
    #[test]
    fn deleted_group_flag_follows_rows_affected() {
        let from_rows = |rows: u64| rows > 0;
        assert!(!from_rows(0));
        assert!(from_rows(1));
    }

    /// 默认店铺 ID 是平台 0（Java `currentShopId()` 的归一值）。
    #[test]
    fn default_shop_id_is_platform_zero() {
        assert_eq!(MemberTagGroupService::default_shop_id(), 0);
        assert_eq!(MemberTagGroupService::default_shop_id(), DEFAULT_SHOP_ID);
    }

    /// `now_secs()` 必须是合理的「当前」秒级时间戳
    /// （复刻 `MybatisPlusMetaObjectHandler` 的 `Instant.now().getEpochSecond()`）。
    #[test]
    fn now_secs_is_current_epoch_seconds() {
        let t = now_secs();
        assert!(t > 1_577_836_800, "时间戳过小: {t}");
        assert!(t < 4_102_444_800, "时间戳过大: {t}");
    }

    /// `create` 的入参校验（`@NotBlank groupName` / `@NotNull tagNames`
    /// / `@Size(min = 2)`）—— 校验失败时事务都未开启。
    #[test]
    fn create_validation_guards_transaction() {
        let dto = MemberTagGroupCreateDTO {
            group_name: None,
            tag_names: None,
        };
        assert_eq!(validate_create(&dto).unwrap_err(), MSG_GROUP_NAME_REQUIRED);

        let dto = MemberTagGroupCreateDTO {
            group_name: Some("g".into()),
            tag_names: Some(vec!["A".into()]),
        };
        assert_eq!(validate_create(&dto).unwrap_err(), MSG_TAG_NAMES_MIN);
    }
}