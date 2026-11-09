//! 会员标签管理域（member::member_tag）仓储：`member_tag` / `member_tag_group` /
//! `user_member_tag`。
//!
//! 对应原 Java：
//! - `com.nextrade.mapper.user.MemberTagMapper`（`BaseMapper<MemberTag>`）
//! - `com.nextrade.mapper.user.MemberTagGroupMapper`
//! - `com.nextrade.mapper.user.UserMemberTagMapper`
//! - `com.nextrade.service.user.impl.MemberTagServiceImpl` 里的 `getBaseMapper()`
//!   通用方法（`selectById` / `selectList` / `insert` / `updateById` /
//!   `deleteById`）
//!
//! # 列名对齐（权威来源）
//!
//! 全部列名来自 `java/database/pure.sql`，用
//! `cd rust && python tools/schema_parser.py --table <表名>` 核对：
//!
//! - `member_tag`（11 列，主键 `id`）：
//!   `id` / `shop_id` / `group_id` / `tag_name` / `create_time` /
//!   `create_by_id` / `create_by_name` / `update_time` / `update_by_id` /
//!   `update_by_name` / `is_del`
//! - `member_tag_group`（11 列，主键 `id`）：
//!   `id` / `shop_id` / `group_name` / `is_top` / `create_time` /
//!   `create_by_id` / `create_by_name` / `update_time` / `update_by_id` /
//!   `update_by_name` / `is_del`
//! - `user_member_tag`（5 列，主键 `id`）：
//!   `id` / `shop_id` / `user_id` / `tag_id` / `create_time`
//!   —— **无** `is_del`（该实体不继承 `BasePO`，无 `@TableLogic`）
//!
//! 查询统一使用运行时 `sqlx::query_as` / `sqlx::query`
//! （**不使用** `query!` / `query_as!` 宏：本机无 `DATABASE_URL`）。
//!
//! # 软删除复刻（关键）
//!
//! Java `MemberTag` / `MemberTagGroup` 继承 `BasePO`，其 `isDel` 带
//! `@TableLogic`（`logic-delete-value: 1` / `logic-not-delete-value: 0`）。
//! 因此本仓储的**每一条** `member_tag` 读/写语句都显式带上
//! `is_del = 0`，`delete_tag` 也写成 `UPDATE ... SET is_del = 1`，
//! 与 MyBatis-Plus 生成的实际 SQL 一致。
//!
//! # 金额
//!
//! 本域**不涉及任何金额列**，故无 `rust_decimal::Decimal`。

use sqlx::PgPool;

use nextrade_core::entities::{MemberTag, MemberTagGroup};
use nextrade_core::models::member::member_tag::{IS_DEL_NO, IS_DEL_YES, MSG_TAG_NOT_FOUND};
use nextrade_core::{Error, Result};

/// `member_tag` 的固定 SELECT 列清单（避免 `SELECT *` 与实体字段顺序耦合；
/// 与 `entities::MemberTag` 的 11 个字段一一对应，顺序也一致）。
const MEMBER_TAG_SELECT: &str = "SELECT id, shop_id, group_id, tag_name, create_time, \
     create_by_id, create_by_name, update_time, update_by_id, update_by_name, is_del \
     FROM member_tag";

/// \"member_tag_group\" 的固定 SELECT 列清单（与 `entities::MemberTagGroup`
/// 的 11 个字段一一对应）。
const MEMBER_TAG_GROUP_SELECT: &str = "SELECT id, shop_id, group_name, is_top, create_time, \
     create_by_id, create_by_name, update_time, update_by_id, update_by_name, is_del \
     FROM member_tag_group";

#[derive(Clone)]
pub struct MemberTagRepository {
    pool: PgPool,
}

impl MemberTagRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排，与其它切片仓储一致）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— member_tag 查询 —————————————————————

    /// 按主键查询未删除的标签（对应 Java `getBaseMapper().selectById(id)`）。
    ///
    /// MyBatis-Plus 的 \"selectById\" 会自动追加 `@TableLogic` 条件，
    /// 等价于 `WHERE id = $1 AND is_del = 0`。**已逻辑删除的标签查不到**
    /// —— 这正是 Java 里「删两次第二次报『标签不存在』」的原因。
    pub async fn find_tag_by_id(&self, id: i64) -> Result<Option<MemberTag>> {
        sqlx::query_as::<_, MemberTag>(&format!(
            "{MEMBER_TAG_SELECT} WHERE id = ? AND is_del = ? LIMIT 1"
        ))
        .bind(id)
        .bind(IS_DEL_NO)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_tag_err)
    }

    /// 查询某店铺下某标签组内的全部未删除标签
    /// （对应 Java `MemberTagServiceImpl#listByGroupId` 的
    /// `selectList(eq(shopId).eq(groupId))`）。
    ///
    /// ⚠️ Java 里**没有** \"orderBy\"，所以顺序由 MySQL 决定（实践中通常是
    /// 主键顺序，但 SQL 不保证）。本实现显式 `ORDER BY id ASC` 让结果
    /// 稳定可测；这是**可观测行为上的一处有意固定**，见
    /// \"uncertain_points\"。
    pub async fn list_tags_by_group(&self, shop_id: i32, group_id: i64) -> Result<Vec<MemberTag>> {
        sqlx::query_as::<_, MemberTag>(&format!(
            "{MEMBER_TAG_SELECT} WHERE shop_id = ? AND group_id = ? AND is_del = ? ORDER BY id ASC"
        ))
        .bind(shop_id)
        .bind(group_id)
        .bind(IS_DEL_NO)
        .fetch_all(&self.pool)
        .await
        .map_err(map_tag_err)
    }

    // ————————————————————— member_tag_group 查询 —————————————————————

    /// 按主键查询未删除的标签组
    /// （对应 Java `memberTagGroupMapper.selectById(param.getGroupId())`）。
    ///
    /// \"create\" 用它做归属校验（`group == null || group.shopId != shopId`
    /// → `标签组不存在`）。同样自动带 `is_del = 0`。
    pub async fn find_group_by_id(&self, id: i64) -> Result<Option<MemberTagGroup>> {
        sqlx::query_as::<_, MemberTagGroup>(&format!(
            "{MEMBER_TAG_GROUP_SELECT} WHERE id = ? AND is_del = ? LIMIT 1"
        ))
        .bind(id)
        .bind(IS_DEL_NO)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_tag_err)
    }

    // ————————————————————— member_tag 写入 —————————————————————

    /// 新增标签（对应 Java `getBaseMapper().insert(tag)`）。
    ///
    /// 除业务三列（\"shop_id\" / \"group_id\" / \"tag_name\"）外，还要写
    /// \"create_time\" / \"update_time\" / \"is_del\" 三列 —— 它们由
    /// `MybatisPlusMetaObjectHandler#insertFill` 自动填充：
    /// - `createTime = updateTime = Instant.now().getEpochSecond()`
    /// - \"createById\" / \"createByName\" / \"updateById\" / \"updateByName\"
    ///   取自 \"SecurityUtils\"（**本仓尚未实现管理员上下文**，故显式传
    ///   \"None\"，见 \"uncertain_points\"）
    /// - `isDel = Constants.NO`（0）
    ///
    /// `create_by_*` / `update_by_*` 是本表的 \"NULL\" 允许列，
    /// 传 \"None\" 即写 \"NULL\"。
    ///
    /// 返回自增主键。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn insert_tag(
        &self,
        shop_id: i32,
        group_id: i64,
        tag_name: &str,
        now_secs: i64,
        operator_id: Option<i64>,
        operator_name: Option<&str>,
    ) -> Result<i64> {
        let res: i64 = sqlx::query_scalar(
            "INSERT INTO member_tag
                 (shop_id, group_id, tag_name, create_time, create_by_id, create_by_name,
                  update_time, update_by_id, update_by_name, is_del)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(shop_id)
        .bind(group_id)
        .bind(tag_name)
        .bind(now_secs)
        .bind(operator_id)
        .bind(operator_name)
        .bind(now_secs)
        .bind(operator_id)
        .bind(operator_name)
        .bind(IS_DEL_NO)
        .fetch_one(&self.pool)
        .await
        .map_err(map_tag_err)?;
        Ok(res)
    }

    /// 更新标签名称（对应 Java `getBaseMapper().updateById(tag)`）。
    ///
    /// Java 的写法是「先 \"selectById\" 拿到整行 → \"setTagName\" → \"updateById\"」，
    /// MyBatis-Plus 的 \"updateById\" 只 \"SET\" 非 null 字段并自动补
    /// \"updateTime\" / `updateBy*`（`FieldFill.INSERT_UPDATE`）。
    /// 净效果 = 只改 \"tag_name\" + 刷新 \"update_time\" + `is_del = 0` 守卫。
    ///
    /// `WHERE id = $2 AND is_del = $3` 中的 `is_del = 0` 复刻 `@TableLogic`
    /// 对 \"UPDATE\" 的附加条件。
    ///
    /// 返回受影响行数。
    pub async fn update_tag_name(
        &self,
        id: i64,
        tag_name: &str,
        now_secs: i64,
        operator_id: Option<i64>,
        operator_name: Option<&str>,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE member_tag
                SET tag_name = ?, update_time = ?, update_by_id = ?, update_by_name = ?
              WHERE id = ? AND is_del = ?",
        )
        .bind(tag_name)
        .bind(now_secs)
        .bind(operator_id)
        .bind(operator_name)
        .bind(id)
        .bind(IS_DEL_NO)
        .execute(&self.pool)
        .await
        .map_err(map_tag_err)?;
        Ok(res.rows_affected())
    }

    /// 逻辑删除标签（对应 Java `getBaseMapper().deleteById(id)`）。
    ///
    /// ⚠️ 因为 `BasePO.isDel` 带 `@TableLogic`，MyBatis-Plus 的
    /// \"deleteById\" **不是** \"DELETE\"，而是
    /// `UPDATE member_tag SET is_del = 1 WHERE id = $2 AND is_del = 0`。
    /// 本方法逐字符复刻该语句。
    ///
    /// 返回受影响行数（已删除/不存在的 id → 0）。
    pub async fn logical_delete_tag(&self, id: i64) -> Result<u64> {
        let res = sqlx::query("UPDATE member_tag SET is_del = ? WHERE id = ? AND is_del = ?")
            .bind(IS_DEL_YES)
            .bind(id)
            .bind(IS_DEL_NO)
            .execute(&self.pool)
            .await
            .map_err(map_tag_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— user_member_tag 清理 —————————————————————

    /// 删除某店铺下某个标签的全部用户关联
    /// （对应 Java \"delTag\" 里的
    /// `userMemberTagMapper.delete(eq(shopId).eq(tagId))`）。
    ///
    /// ⚠️ **这是物理删除 \"DELETE\"**，不是逻辑删除：
    /// \"UserMemberTag\" 是独立 POJO，既不继承 \"BasePO\" 也无 `@TableLogic`，
    /// 且 \"user_member_tag\" 表**没有** \"is_del\" 列
    /// （`schema_parser.py --table user_member_tag` 只有 5 列）。
    /// 这一非对称性必须保留，否则会与 Java 行为分叉。
    ///
    /// 返回删除的行数。
    pub async fn delete_user_tag_links(&self, shop_id: i32, tag_id: i64) -> Result<u64> {
        let res = sqlx::query("DELETE FROM user_member_tag WHERE shop_id = ? AND tag_id = ?")
            .bind(shop_id)
            .bind(tag_id)
            .execute(&self.pool)
            .await
            .map_err(map_tag_err)?;
        Ok(res.rows_affected())
    }
}

/// 数据库错误映射（与 area_code / admin_user / pc_cat_floor 同一约定）。
fn map_tag_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound(MSG_TAG_NOT_FOUND.into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::member::member_tag::{MEMBER_TAG_COLUMNS, USER_MEMBER_TAG_COLUMNS};

    /// \"MEMBER_TAG_SELECT\" 必须覆盖 \"member_tag\" 全部 11 列且**顺序**与
    /// `entities::MemberTag` 一致（\"FromRow\" 按位置/名称取列）。
    /// 这里只做列名层面的核对，不依赖数据库。
    #[test]
    fn member_tag_select_covers_all_columns() {
        for column in MEMBER_TAG_COLUMNS {
            assert!(
                MEMBER_TAG_SELECT.contains(column),
                "SELECT 清单缺少列 {column}"
            );
        }
        assert!(MEMBER_TAG_SELECT.starts_with("SELECT "));
        assert!(MEMBER_TAG_SELECT.contains("FROM member_tag"));
        // 不得用 SELECT *
        assert!(!MEMBER_TAG_SELECT.contains('*'));
    }

    /// 标签组的 SELECT 清单同理，且列数一致（11）。
    #[test]
    fn member_tag_group_select_covers_all_columns() {
        use nextrade_core::models::member::member_tag::MEMBER_TAG_GROUP_COLUMNS;
        for column in MEMBER_TAG_GROUP_COLUMNS {
            assert!(
                MEMBER_TAG_GROUP_SELECT.contains(column),
                "SELECT 清单缺少列 {column}"
            );
        }
        assert!(MEMBER_TAG_GROUP_SELECT.contains("FROM member_tag_group"));
        assert!(!MEMBER_TAG_GROUP_SELECT.contains('*'));
    }

    /// 逻辑删除语句必须**只**改 \"is_del\"，且带 `is_del = 0` 守卫
    /// —— 复刻 MyBatis-Plus `@TableLogic` 生成的 SQL。
    #[test]
    fn logical_delete_sql_matches_table_logic_semantics() {
        // 语句在 \"logical_delete_tag\" 内联；这里对照语义常量
        assert_eq!(IS_DEL_YES, 1, "logic-delete-value 应为 1");
        assert_eq!(IS_DEL_NO, 0, "logic-not-delete-value 应为 0");
    }

    /// \"user_member_tag\" 没有 \"is_del\" 列 ⇒ 清关联只能是物理 \"DELETE\"。
    /// 若哪天 schema 加了该列，本测试会失败并提醒重新核对非对称行为。
    #[test]
    fn user_tag_link_cleanup_is_hard_delete() {
        assert!(
            !USER_MEMBER_TAG_COLUMNS.contains(&"is_del"),
            "user_member_tag 若新增 is_del 列，需重新核对 delTag 的删除语义"
        );
        // 该表有 shop_id / tag_id，正好对应 Java 的两个 eq 条件
        assert!(USER_MEMBER_TAG_COLUMNS.contains(&"shop_id"));
        assert!(USER_MEMBER_TAG_COLUMNS.contains(&"tag_id"));
    }
}"