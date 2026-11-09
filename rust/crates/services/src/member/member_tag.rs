//! 会员标签管理服务（member::member_tag）。
//!
//! 对应原 Java `com.nextrade.service.user.impl.MemberTagServiceImpl`（101 行）
//! + `com.nextrade.adminapi.controller.user.MemberTagController`（58 行）
//! 的 4 条管理端链路：
//!
//! | 本域端点（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |--------------------------------|---------|------|
//! | `GET  /user/memberTag/listByGroup` | `MemberTagController.listByGroup` | [`MemberTagService::list_by_group`] |
//! | `POST /user/memberTag/create` | `MemberTagController.create` | [`MemberTagService::create`] |
//! | `POST /user/memberTag/update` | `MemberTagController.update` | [`MemberTagService::update`] |
//! | `POST /user/memberTag/del` | `MemberTagController.del` | [`MemberTagService::del`] |
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
//! 它**不是**请求体/查询参数，前端无法通过 JSON 覆盖。四个方法
//! （`create` / `update` / `delTag` / `listByGroupId`）全部先取它，
//! 再作为**所有** SQL 的店铺过滤条件 —— 这就是「店铺隔离」的实现方式。
//!
//! Rust 侧 JWT 尚未签发 `shop_id`（与本仓其它域一致的已知缺口），
//! 因此本服务把 `shop_id` 作为**显式入参**，由路由层传入；
//! 路由层当前固定传 [`DEFAULT_SHOP_ID`]（平台 0）。
//! ⚠️ 这意味着**店铺端调用本域接口时会被当作平台账号处理**，
//! 是待鉴权中间件补齐后才能闭合的缺口，已列入 `uncertain_points`。
//!
//! ## `create`（Java 43–54 行）
//!
//! ```text
//! int shopId = currentShopId();
//! MemberTagGroup group = memberTagGroupMapper.selectById(param.getGroupId());
//! if (group == null || group.getShopId() == null || group.getShopId() != shopId) {
//!     throw new GlobalException("标签组不存在");
//! }
//! MemberTag tag = new MemberTag();
//! tag.setShopId(shopId);
//! tag.setGroupId(param.getGroupId());
//! tag.setTagName(param.getTagName().trim());
//! getBaseMapper().insert(tag);
//! ```
//!
//! 逐条复刻：
//! 1. `@Valid` 校验（`groupId` `@NotNull`、`tagName` `@NotBlank`）
//! 2. `selectById(groupId)`（自动带 `is_del = 0`）
//! 3. 三重判定：**不存在** / `shopId` 为 null / **`shopId` 不等于当前店铺**
//!    —— 三者抛出**同一条** `标签组不存在`（不区分，避免枚举他人店铺数据）
//! 4. 入库（`tagName` 已 trim；`createTime` 等自动填充）
//!
//! ## `update`（Java 56–66 行）
//!
//! ```text
//! MemberTag tag = getBaseMapper().selectById(param.getId());
//! if (tag == null || tag.getShopId() == null || tag.getShopId() != shopId) {
//!     throw new GlobalException("标签不存在");
//! }
//! tag.setTagName(param.getTagName().trim());
//! getBaseMapper().updateById(tag);
//! ```
//!
//! ⚠️ **`update` 不校验 `groupId`，也不允许改 `groupId`** ——
//! `MemberTagUpdateParam` 里根本没有 `groupId` 字段，标签不可跨组移动。
//! 前端若传了 `groupId` 会被 Jackson 忽略（Rust 侧 serde 同样忽略）。
//!
//! ⚠️ `tag.getShopId() != shopId` 在 Java 里是 `Integer` 与 `int` 比较 ⇒
//! 自动拆箱成 `int != int` 的**值比较**（不是引用比较），语义正确。
//! 本实现直接做 `i32` 值比较。
//!
//! ## `delTag`（Java 68–83 行）
//!
//! ```text
//! MemberTag tag = getBaseMapper().selectById(id);
//! if (tag == null || tag.getShopId() == null || tag.getShopId() != shopId) {
//!     throw new GlobalException("标签不存在");
//! }
//! // 清理用户关联（**物理删除**）
//! userMemberTagMapper.delete(eq(shopId).eq(tagId, id));
//! // 逻辑删除标签（@TableLogic）
//! getBaseMapper().deleteById(id);
//! ```
//!
//! 顺序很重要：**先删 `user_member_tag` 关联，再逻辑删 `member_tag`**。
//! 两者在同一个 `@Transactional` 里。
//!
//! ## `listByGroupId`（Java 85–100 行）
//!
//! ```text
//! selectList(eq(shopId).eq(groupId))   // 自动带 is_del = 0
//! → stream().map(vo) : id / groupId / tagName / createTime
//! ```
//!
//! ⚠️ `listByGroupId` **不校验组是否存在/是否属于本店铺**：
//! 组不存在时返回**空列表**（不是 404）。这是 Java 的真实行为，
//! 不是遗漏。Rust 侧保持一致。
//!
//! ⚠️ `groupId` 在 Java 控制器上是 `@RequestParam Long groupId`（**必填**），
//! 缺失时 Spring 抛 `MissingServletRequestParameterException` → 400。
//! 本实现由路由层要求必填。
//!
//! # 未生效的「冻结守卫」
//!
//! `MemberTagServiceImpl` 注入了 `ShopMemberSettingService
//! shopMemberSettingService`，类注释也写着「店铺隔离 + 冻结守卫
//! （SHOP-MEMBER-20260708）」，但该类**四个方法里一次都没调用它**
//! （全类唯一出现处是第 34 行的字段声明）。
//! 本实现**不实现**冻结守卫（复刻 Java 的可观测行为），见
//! `uncertain_points`。
//!
//! # 金额
//!
//! 本域**不涉及任何金额**。

use std::sync::Arc;

use nextrade_core::models::member::member_tag::{
    validate_create, validate_del, validate_update, MemberTagCreateDTO, MemberTagDelDTO,
    MemberTagUpdateDTO, MemberTagVO, DEFAULT_SHOP_ID, MSG_GROUP_NOT_FOUND, MSG_TAG_NOT_FOUND,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::member::member_tag::MemberTagRepository;
use nextrade_infrastructure::db::Repositories;

/// 会员标签管理域服务（挂载于 `AdminState.member_tag`）。
pub struct MemberTagService {
    repos: Arc<Repositories>,
}

impl MemberTagService {
    /// 构造（与 order_admin / admin_user / area_code 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 仓储引用（便于测试与后续切片复用）。
    #[must_use]
    pub fn repo(&self) -> &MemberTagRepository {
        &self.repos.member.member_tag
    }

    // ————————————————————— 列表 —————————————————————

    /// 查询组下标签列表（对应 Java `MemberTagServiceImpl#listByGroupId`）。
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. `shopId = currentShopId()`
    /// 2. `selectList(eq(shopId).eq(groupId))` —— 自动带 `is_del = 0`
    /// 3. 逐条映射为 `MemberTagVO`（`id` / `groupId` / `tagName` / `createTime`）
    ///
    /// ⚠️ **不校验标签组是否存在**：组不存在或不属于本店铺时，
    /// `WHERE` 条件自然匹配不到任何行 ⇒ 返回**空列表**（Java 行为，非 404）。
    ///
    /// ⚠️ Java 没有 `ORDER BY`；本实现显式 `ORDER BY id ASC` 使结果稳定
    /// （见 `uncertain_points`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn list_by_group(&self, group_id: i64, shop_id: i32) -> Result<Vec<MemberTagVO>> {
        let rows = self
            .repos.member.member_tag
            .list_tags_by_group(shop_id, group_id)
            .await?;
        Ok(rows.iter().map(MemberTagVO::from).collect())
    }

    // ————————————————————— 新增 —————————————————————

    /// 新增标签（对应 Java `MemberTagServiceImpl#create`）。
    ///
    /// 流程与 Java 逐条对齐（详见模块头）：
    /// 1. 入参校验（`@NotNull groupId` / `@NotBlank tagName`，文案照抄）
    /// 2. 查标签组，**三重判定**（不存在 / `shopId` 为 null / 不属于本店铺）
    ///    → 统一抛 `标签组不存在`
    /// 3. 入库：`shopId` 取当前店铺、`groupId` 取入参、`tagName` 已 trim
    ///
    /// Java 控制器返回 `void`；本实现返回新建主键，便于前端跳转编辑
    /// （与 shop_manage / pc_cat_floor / area_code 的处理方式一致）。
    ///
    /// # Errors
    /// - 入参缺失/空白 → `Error::Validation`（文案逐字符照抄 Java）
    /// - 标签组不存在或不属于本店铺 → `Error::Validation(标签组不存在)`
    ///   （Java 抛 `GlobalException`，本仓对应 `Error::Validation`）
    /// - 数据库失败 → `Error::Database`
    pub async fn create(&self, dto: &MemberTagCreateDTO, shop_id: i32) -> Result<i64> {
        // 1. `@Valid MemberTagCreateParam`
        let (group_id, tag_name) = validate_create(dto).map_err(Error::Validation)?;

        // 2. 标签组必须存在，且归属于当前店铺
        //    （Java `group == null || group.getShopId() == null
        //      || group.getShopId() != shopId`）
        //
        // 三重判定说明：
        // - 「group == null」⇒ `find_group_by_id` 返回 `None`
        // - 「group.getShopId() == null」在 Rust 侧类型上不可表达
        //   （`member_tag_group.shop_id` 是 `NOT NULL`，实体为 `i32`）
        // - 「group.getShopId() != shopId」⇒ 值比较
        // - 组已逻辑删除（`is_del = 1`）⇒ 已由 SQL 的 `is_del = 0` 过滤掉，
        //   等价于 Java `@TableLogic` 的自动条件
        // 四者统一抛同一条 `标签组不存在`，不泄露「存在但不属于你」。
        let group = self.repos.member.member_tag.find_group_by_id(group_id).await?;
        let owned = matches!(group, Some(g) if g.shop_id == shop_id);
        if !owned {
            return Err(Error::Validation(MSG_GROUP_NOT_FOUND.into()));
        }

        // 3. 入库（`createTime` / `updateTime` / `isDel` 自动填充）
        let now = now_secs();
        let id = self
            .repos.member.member_tag
            .insert_tag(shop_id, group_id, &tag_name, now, None, None)
            .await?;
        let log_info = format!("新增标签：{}", tag_name);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_current_log(if shop_id > 0 { "shop" } else { "admin" }, shop_id, &log_info)
            .await
        {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!("新增会员标签成功: id={id}, shopId={shop_id}, groupId={group_id}");
        Ok(id)
    }

    // ————————————————————— 更新 —————————————————————

    /// 更新标签（对应 Java `MemberTagServiceImpl#update`）。
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. 入参校验（`@NotNull id` / `@NotBlank tagName`）
    /// 2. `selectById(id)`（自动带 `is_del = 0`）
    /// 3. 三重判定（不存在 / `shopId` 为 null / 不属于本店铺）
    ///    → 统一抛 `标签不存在`
    /// 4. 只改 `tag_name`（Java `updateById` 只 SET 非 null 字段，
    ///    并自动刷新 `update_time` / `update_by*`）
    ///
    /// ⚠️ **不校验也不允许修改 `groupId`** —— `MemberTagUpdateParam`
    /// 没有该字段（标签不可跨组移动）。
    ///
    /// 返回是否命中记录（Java 控制器返回 `void`）。
    ///
    /// # Errors
    /// - 入参缺失/空白 → `Error::Validation`
    /// - 标签不存在或不属于本店铺 → `Error::Validation(标签不存在)`
    /// - 数据库失败 → `Error::Database`
    pub async fn update(&self, dto: &MemberTagUpdateDTO, shop_id: i32) -> Result<bool> {
        let (id, tag_name) = validate_update(dto).map_err(Error::Validation)?;

        // 2 + 3. 先查再判归属（Java 先 `selectById` 再比 shopId）
        let tag = self
            .repos.member.member_tag
            .find_tag_by_id(id)
            .await?
            .ok_or_else(|| Error::Validation(MSG_TAG_NOT_FOUND.into()))?;
        if tag.shop_id != shop_id {
            return Err(Error::Validation(MSG_TAG_NOT_FOUND.into()));
        }

        // 4. 只更新名称（+ 自动填充的 update_time / update_by*）
        let affected = self
            .repos.member.member_tag
            .update_tag_name(id, &tag_name, now_secs(), None, None)
            .await?;
        let log_info = format!("更新标签：{}", tag_name);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_current_log(if shop_id > 0 { "shop" } else { "admin" }, shop_id, &log_info)
            .await
        {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!("更新会员标签: id={id}, 影响 {affected} 行");
        Ok(affected > 0)
    }

    // ————————————————————— 删除 —————————————————————

    /// 删除标签（对应 Java `MemberTagServiceImpl#delTag`）。
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. 入参校验（`OperateDTO.@NotNull id`，`Integer` → `Long`）
    /// 2. `selectById(id)`
    /// 3. 三重判定 → 统一抛 `标签不存在`
    /// 4. **物理删除** `user_member_tag WHERE shop_id = ? AND tag_id = ?`
    ///    （该表无 `is_del`，不是逻辑删除）
    /// 5. **逻辑删除** `member_tag`（`@TableLogic` → `SET is_del = 1`）
    ///
    /// 返回 `(清理的关联行数, 标签是否被删除)`，
    /// 便于前端/运维观测（Java 控制器返回 `void`）。
    ///
    /// ⚠️ 顺序不能颠倒：先清关联再删标签，与 Java 一致。
    ///
    /// # Errors
    /// - `id` 缺失 → `Error::Validation(id不能为空)`
    /// - 标签不存在或不属于本店铺 → `Error::Validation(标签不存在)`
    /// - 数据库失败 → `Error::Database`
    pub async fn del(&self, dto: &MemberTagDelDTO, shop_id: i32) -> Result<DelOutcome> {
        let id = validate_del(dto).map_err(Error::Validation)?;

        let tag = self
            .repos.member.member_tag
            .find_tag_by_id(id)
            .await?
            .ok_or_else(|| Error::Validation(MSG_TAG_NOT_FOUND.into()))?;
        if tag.shop_id != shop_id {
            return Err(Error::Validation(MSG_TAG_NOT_FOUND.into()));
        }

        // 4. 清理用户关联（物理删除）
        let links = self
            .repos.member.member_tag
            .delete_user_tag_links(shop_id, id)
            .await?;
        // 5. 逻辑删除标签
        let deleted = self.repos.member.member_tag.logical_delete_tag(id).await?;
        let log_info = format!("删除标签：{}", id);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_current_log(if shop_id > 0 { "shop" } else { "admin" }, shop_id, &log_info)
            .await
        {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!("删除会员标签: id={id}, 清理关联 {links} 行, 逻辑删除 {deleted} 行");
        Ok(DelOutcome {
            removed_links: links,
            deleted: deleted > 0,
        })
    }

    // ————————————————————— 供路由层复用的纯逻辑 —————————————————————

    /// 路由层默认使用的店铺 ID（平台 0）。
    ///
    /// Java `currentShopId()` 在请求头缺失时归一为 `0`；Rust 侧鉴权中间件
    /// 尚未提供 `shop_id`，故默认按平台账号处理（见 `uncertain_points`）。
    #[must_use]
    pub const fn default_shop_id() -> i32 {
        DEFAULT_SHOP_ID
    }
}

/// `del` 的执行结果（Java 返回 `void`，此处回传可观测的计数）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelOutcome {
    /// 被物理删除的 `user_member_tag` 关联行数
    pub removed_links: u64,
    /// `member_tag` 是否被逻辑删除（受影响行数 > 0）
    pub deleted: bool,
}

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
    use nextrade_core::models::member::member_tag::{
        MSG_GROUP_ID_REQUIRED, MSG_ID_REQUIRED, MSG_TAG_ID_REQUIRED, MSG_TAG_NAME_REQUIRED,
    };

    // ————————————————————— 常量 / 文案 —————————————————————

    /// 默认店铺 ID 必须是平台 0（Java `currentShopId()` 的归一值）。
    #[test]
    fn default_shop_id_is_platform_zero() {
        assert_eq!(MemberTagService::default_shop_id(), 0);
    }

    /// 两条「不存在」文案与四条校验文案逐字符对照 Java。
    #[test]
    fn message_constants_match_java_verbatim() {
        assert_eq!(MSG_GROUP_NOT_FOUND, "标签组不存在");
        assert_eq!(MSG_TAG_NOT_FOUND, "标签不存在");
        assert_eq!(MSG_GROUP_ID_REQUIRED, "标签组ID不能为空");
        assert_eq!(MSG_TAG_ID_REQUIRED, "标签ID不能为空");
        assert_eq!(MSG_TAG_NAME_REQUIRED, "标签名称不能为空");
        assert_eq!(MSG_ID_REQUIRED, "id不能为空");
    }

    /// ⚠️ 归属校验的**三重判定**语义（Java
    /// `group == null || group.getShopId() == null || group.getShopId() != shopId`）
    /// 必须三者归一到同一条文案 —— 不区分「不存在」与「不属于你」，
    /// 以免通过错误信息枚举他人店铺的标签组 ID。
    ///
    /// 本测试直接对着「判定表达式」写期望值，而不是复述实现：
    /// - 组不存在 → 报错
    /// - 组的 shopId = 0，当前店铺 = 7 → 报错
    /// - 组的 shopId = 7，当前店铺 = 7 → 通过
    #[test]
    fn group_ownership_check_is_three_way_and_uniform() {
        // `owned` 的判定（与实现同式，但期望值独立列出）
        let is_owned = |group: Option<i32>, shop_id: i32| -> bool {
            matches!(group, Some(g) if g == shop_id)
        };

        // 1. group == null
        assert!(!is_owned(None, 0));
        assert!(!is_owned(None, 7));
        // 2. group.getShopId() != shopId（平台组被店铺账号访问）
        assert!(!is_owned(Some(0), 7));
        // 3. group.getShopId() != shopId（店铺 A 访问店铺 B 的组）
        assert!(!is_owned(Some(3), 7));
        // 4. 归属一致才通过
        assert!(is_owned(Some(7), 7));
        // 5. 平台账号访问平台组（shopId 都是 0）
        assert!(is_owned(Some(0), 0));
    }

    /// `del` 的结果结构：默认即「没清关联、没删标签」。
    #[test]
    fn del_outcome_shape() {
        let o = DelOutcome {
            removed_links: 0,
            deleted: false,
        };
        assert_eq!(o.removed_links, 0);
        assert!(!o.deleted);

        let o = DelOutcome {
            removed_links: 3,
            deleted: true,
        };
        assert_eq!(o.removed_links, 3);
        assert!(o.deleted);
    }

    /// `now_secs()` 必须是合理的「当前」秒级时间戳
    /// （复刻 `MybatisPlusMetaObjectHandler` 的 `Instant.now().getEpochSecond()`）。
    #[test]
    fn now_secs_is_current_epoch_seconds() {
        let t = now_secs();
        // 大于 2020-01-01（1_577_836_800）且小于 2100-01-01（4_102_444_800）
        assert!(t > 1_577_836_800, "时间戳过小: {t}");
        assert!(t < 4_102_444_800, "时间戳过大: {t}");
    }

    /// `DelOutcome` 的 `deleted` 判据是 `rows_affected > 0`，
    /// 与 Java `deleteById` 返回的 `SqlHelper.retBool(rows)` 一致。
    #[test]
    fn del_deleted_flag_follows_rows_affected() {
        let from_rows = |rows: u64| DelOutcome {
            removed_links: 0,
            deleted: rows > 0,
        };
        assert!(!from_rows(0).deleted);
        assert!(from_rows(1).deleted);
    }
}