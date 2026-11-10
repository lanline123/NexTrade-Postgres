//! 会员标签组管理域（member::member_tag_group）仓储：`member_tag_group` / `member_tag` /
//! `user_member_tag`。
//!
//! 对应原 Java：
//! - `com.nextrade.mapper.user.MemberTagGroupMapper`
//!   （`BaseMapper<MemberTagGroup>` + `@Select selectMaxIsTop`）
//! - `com.nextrade.mapper.user.MemberTagMapper`（`BaseMapper<MemberTag>`）
//! - `com.nextrade.mapper.user.UserMemberTagMapper`（`BaseMapper<UserMemberTag>`）
//! - `com.nextrade.service.user.impl.MemberTagGroupServiceImpl` 里的
//!   `getBaseMapper()` 通用方法（`selectById` / `selectList` / `insert` /
//!   `updateById` / `deleteById`）
//!
//! # 列名对齐（权威来源）
//!
//! 全部列名来自 `java/database/pure.sql`，用
//! `cd rust && python tools/schema_parser.py --table <表名>` 核对：
//!
//! - `member_tag_group`（11 列，主键 `id`）：
//!   `id` / `shop_id` / `group_name` / `is_top` / `create_time` /
//!   `create_by_id` / `create_by_name` / `update_time` / `update_by_id` /
//!   `update_by_name` / `is_del`
//! - `member_tag`（11 列，主键 `id`）：
//!   `id` / `shop_id` / `group_id` / `tag_name` / `create_time` /
//!   `create_by_id` / `create_by_name` / `update_time` / `update_by_id` /
//!   `update_by_name` / `is_del`
//! - `user_member_tag`（5 列，主键 `id`）：
//!   `id` / `shop_id` / `user_id` / `tag_id` / `create_time`
//!   —— **无** `is_del`（该实体不继承 `BasePO`，无 `@TableLogic`）
//!
//! 查询统一使用运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`
//! （**不使用** `query!` / `query_as!` 宏：本机无 `DATABASE_URL`）。
//!
//! # 软删除复刻（关键）
//!
//! `MemberTagGroup` / `MemberTag` 继承 `BasePO`，其 `isDel` 带
//! `@TableLogic`（`logic-delete-value: 1` / `logic-not-delete-value: 0`）。
//! 因此本仓储**每一条** `member_tag_group` / `member_tag` 的读写语句都显式
//! 带 `is_del = 0`，删除写成 `UPDATE ... SET is_del = 1 WHERE ... AND is_del = 0`
//! —— 与 MyBatis-Plus 生成的实际 SQL 一致。
//!
//! # 事务
//!
//! Java 的 `create` / `update` / `delGroup` / `setTop` / `cancelTop` 都带
//! `@Transactional(rollbackFor = Exception.class)`，且是「先查后改」的多语句
//! 组合。本仓储因此把写方法统一接收 `&mut Transaction`，由**服务层**用
//! [`MemberTagGroupRepository::begin`] 开启事务并编排，保证与 Java 同一事务语义。
//!
//! # 金额
//!
//! 本域**不涉及任何金额列**，故无 `rust_decimal::Decimal`。

use sqlx::{PgPool, QueryBuilder};

use nextrade_core::entities::{MemberTag, MemberTagGroup};
use nextrade_core::models::member::member_tag_group::{
    AdminOperator, ExistingTag, IS_DEL_NO, IS_DEL_YES, MSG_GROUP_NOT_FOUND,
};
use nextrade_core::{Error, Result};

/// `member_tag_group` 的固定 SELECT 列清单（避免 `SELECT *` 与实体字段顺序
/// 耦合；与 `entities::MemberTagGroup` 的 11 个字段一一对应，顺序也一致）。
///
/// ⚠️ 刻意写成**单行**：`tools/check_sql_columns.py` 按「行内引号字面量」
/// 提取 SQL，多行 `\` 续行写法会让它整条跳过，从而失去列名校验。
const MEMBER_TAG_GROUP_SELECT: &str = "SELECT id, shop_id, group_name, is_top, create_time, create_by_id, create_by_name, update_time, update_by_id, update_by_name, is_del FROM member_tag_group";

/// `member_tag` 的固定 SELECT 列清单（与 `entities::MemberTag` 的 11 个字段
/// 一一对应）。
const MEMBER_TAG_SELECT: &str = "SELECT id, shop_id, group_id, tag_name, create_time, create_by_id, create_by_name, update_time, update_by_id, update_by_name, is_del FROM member_tag";

/// `member_tag_group` 的 INSERT 列清单
/// （`id` 自增；`is_top` 由本层显式写 0，等价 Java 的列默认值）。
const MEMBER_TAG_GROUP_INSERT: &str = "INSERT INTO member_tag_group (shop_id, group_name, is_top, create_time, create_by_id, create_by_name, update_time, update_by_id, update_by_name, is_del) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)";

/// `member_tag` 的 INSERT 列清单。
const MEMBER_TAG_INSERT: &str = "INSERT INTO member_tag (shop_id, group_id, tag_name, create_time, create_by_id, create_by_name, update_time, update_by_id, update_by_name, is_del) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)";

#[derive(Clone)]
pub struct MemberTagGroupRepository {
    pool: PgPool,
}

impl MemberTagGroupRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排，与其它切片仓储一致）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 开启事务（服务层编排用，避免服务层直接依赖 `sqlx::Transaction`
    /// 的构造细节；与 user_company / message_type 同一做法）。
    pub async fn begin(&self) -> Result<sqlx::Transaction<'static, sqlx::Postgres>> {
        self.pool.begin().await.map_err(map_err)
    }

    // ————————————————————— 查询（无事务） —————————————————————

    /// 按主键查询未删除的标签组
    /// （对应 Java `getBaseMapper().selectById(id)`）。
    ///
    /// MyBatis-Plus 的 `selectById` 会自动追加 `@TableLogic` 条件，
    /// 等价于 `WHERE id = ? AND is_del = 0`。**已逻辑删除的组查不到**
    /// —— 这正是 Java 里「删两次第二次报『标签组不存在』」的原因。
    ///
    /// ⚠️ 本方法**不**过滤 `shop_id`：归属校验由服务层用 `group.shop_id`
    /// 与当前店铺比较完成（Java 也是先 `selectById` 再比 `shopId`，
    /// 这样才能对「存在但不属于你」与「不存在」返回同一条文案）。
    pub async fn find_group_by_id(&self, id: i64) -> Result<Option<MemberTagGroup>> {
        sqlx::query_as::<_, MemberTagGroup>(&format!(
            "{MEMBER_TAG_GROUP_SELECT} WHERE id = $1 AND is_del = $2 LIMIT 1"
        ))
        .bind(id)
        .bind(IS_DEL_NO)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 查询某店铺下某标签组内的全部未删除标签
    /// （对应 Java `detail` / `delGroup` 里的
    /// `selectList(eq(shopId).eq(groupId, id))`）。
    ///
    /// ⚠️ Java 里**没有** `orderBy`，顺序由 MySQL 决定（实践中通常主键序，
    /// 但 SQL 不保证）。本实现显式 `ORDER BY id ASC` 让结果稳定可测；
    /// 这是**可观测行为上的一处有意固定**（见 `uncertain_points`）。
    pub async fn list_tags_by_group(&self, shop_id: i32, group_id: i64) -> Result<Vec<MemberTag>> {
        sqlx::query_as::<_, MemberTag>(&format!(
            "{MEMBER_TAG_SELECT} WHERE shop_id = $1 AND group_id = $2 AND is_del = $3 ORDER BY id ASC"
        ))
        .bind(shop_id)
        .bind(group_id)
        .bind(IS_DEL_NO)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 当前店铺的最大置顶权重
    /// （对应 Java `MemberTagGroupMapper#selectMaxIsTop`：
    /// `SELECT MAX(is_top) FROM member_tag_group WHERE is_del = 0 AND shop_id = ?`）。
    ///
    /// 无任何组（或全部已删除）时 `MAX` 返回 `NULL` ⇒ `Ok(None)`，
    /// 与 Java 的 `Integer maxTop == null` 分支一致。
    pub async fn select_max_is_top(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        shop_id: i32,
    ) -> Result<Option<i32>> {
        let row: (Option<i32>,) = sqlx::query_as(
            "SELECT MAX(is_top) FROM member_tag_group WHERE is_del = $1 AND shop_id = $2",
        )
        .bind(IS_DEL_NO)
        .bind(shop_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 事务内按主键查询未删除的标签组（`delGroup` / `update` / `setTop` 的
    /// 「先查后改」必须在**同一事务**里，避免检查与写入之间被并发删除）。
    pub async fn find_group_by_id_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i64,
    ) -> Result<Option<MemberTagGroup>> {
        sqlx::query_as::<_, MemberTagGroup>(&format!(
            "{MEMBER_TAG_GROUP_SELECT} WHERE id = $1 AND is_del = $2 LIMIT 1"
        ))
        .bind(id)
        .bind(IS_DEL_NO)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_err)
    }

    /// 事务内查询某店铺下某标签组内的全部未删除标签
    /// （对应 Java `update` / `delGroup` 里的 `selectList`）。
    pub async fn list_tags_by_group_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        shop_id: i32,
        group_id: i64,
    ) -> Result<Vec<MemberTag>> {
        sqlx::query_as::<_, MemberTag>(&format!(
            "{MEMBER_TAG_SELECT} WHERE shop_id = $1 AND group_id = $2 AND is_del = $3 ORDER BY id ASC"
        ))
        .bind(shop_id)
        .bind(group_id)
        .bind(IS_DEL_NO)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_err)
    }

    // ————————————————————— 标签组写入（事务内） —————————————————————

    /// 新增标签组，返回自增主键
    /// （对应 Java `getBaseMapper().insert(group)`）。
    ///
    /// Java 只 `setShopId` / `setGroupName`，其余列由
    /// `MybatisPlusMetaObjectHandler#insertFill` 自动填充：
    /// - `createTime = updateTime = Instant.now().getEpochSecond()`
    /// - `createById` / `updateById` = `SecurityUtils.getCurrentAdminId()`
    /// - `createByName` / `updateByName` = 用户名（空白 → `"unknown"`）
    /// - `isDel = Constants.NO`（0）
    ///
    /// ⚠️ Java **没有**设置 `isTop` ⇒ 实体字段为 null ⇒ NOT_NULL 策略跳过该列
    /// ⇒ MySQL 列默认值 `0` 生效。本实现显式绑定 `0`，净结果相同。
    pub async fn insert_group(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        shop_id: i32,
        group_name: &str,
        operator: &AdminOperator,
        now_secs: i64,
    ) -> Result<i64> {
        let res: i64 = sqlx::query_scalar(&format!("{} RETURNING id", MEMBER_TAG_GROUP_INSERT))
            .bind(shop_id)
            .bind(group_name)
            .bind(0_i32)
            .bind(now_secs)
            .bind(operator.id)
            .bind(operator.name_ref())
            .bind(now_secs)
            .bind(operator.id)
            .bind(operator.name_ref())
            .bind(IS_DEL_NO)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_err)?;
        Ok(res)
    }

    /// 更新标签组名称
    /// （对应 Java `update` 里的 `group.setGroupName(...); updateById(group)`）。
    ///
    /// Java 的 `updateById` 会 `SET` 所有**非 null** 字段（`group` 是从库里
    /// `selectById` 出来的整行，字段基本都非 null），并自动刷新
    /// `updateTime` / `updateBy*`。其中 `shop_id` / `group_name` / `is_top`
    /// 被写回**原值**，净效果只有 `group_name` 与 `update_time` / `update_by*`
    /// 变化。本方法直接表达该净效果，避免无意义的整行回写。
    ///
    /// `WHERE id = ? AND is_del = ?` 中的 `is_del = 0` 复刻 `@TableLogic`
    /// 对 `UPDATE` 的附加条件。
    pub async fn update_group_name(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i64,
        group_name: &str,
        operator: &AdminOperator,
        now_secs: i64,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE member_tag_group SET group_name = $1, update_time = $2, update_by_id = $3, update_by_name = $4 WHERE id = $5 AND is_del = $6",
        )
        .bind(group_name)
        .bind(now_secs)
        .bind(operator.id)
        .bind(operator.name_ref())
        .bind(id)
        .bind(IS_DEL_NO)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 更新标签组的置顶权重
    /// （对应 Java `setTop` / `cancelTop` 里的 `group.setIsTop(v); updateById(group)`）。
    ///
    /// 净效果同 [`Self::update_group_name`]：只改 `is_top` +
    /// `update_time` / `update_by*`。
    pub async fn update_is_top(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i64,
        is_top: i32,
        operator: &AdminOperator,
        now_secs: i64,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE member_tag_group SET is_top = $1, update_time = $2, update_by_id = $3, update_by_name = $4 WHERE id = $5 AND is_del = $6",
        )
        .bind(is_top)
        .bind(now_secs)
        .bind(operator.id)
        .bind(operator.name_ref())
        .bind(id)
        .bind(IS_DEL_NO)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 逻辑删除标签组（对应 Java `getBaseMapper().deleteById(id)`）。
    ///
    /// ⚠️ 因为 `BasePO.isDel` 带 `@TableLogic`，MyBatis-Plus 的 `deleteById`
    /// **不是** `DELETE`，而是
    /// `UPDATE member_tag_group SET is_del = 1 WHERE id = ? AND is_del = 0`。
    /// 本方法逐字符复刻该语句。
    pub async fn logical_delete_group(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i64,
    ) -> Result<u64> {
        let res =
            sqlx::query("UPDATE member_tag_group SET is_del = $1 WHERE id = $2 AND is_del = $3")
                .bind(IS_DEL_YES)
                .bind(id)
                .bind(IS_DEL_NO)
                .execute(&mut **tx)
                .await
                .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— 标签写入（事务内） —————————————————————

    /// 新增标签，返回自增主键（对应 Java `memberTagMapper.insert(tag)`）。
    ///
    /// 业务三列 `shop_id` / `group_id` / `tag_name` 由调用方给出，
    /// 其余列走 `MybatisPlusMetaObjectHandler#insertFill`（见
    /// [`Self::insert_group`] 的说明）。
    pub async fn insert_tag(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        shop_id: i32,
        group_id: i64,
        tag_name: &str,
        operator: &AdminOperator,
        now_secs: i64,
    ) -> Result<i64> {
        let res: i64 = sqlx::query_scalar(&format!("{} RETURNING id", MEMBER_TAG_INSERT))
            .bind(shop_id)
            .bind(group_id)
            .bind(tag_name)
            .bind(now_secs)
            .bind(operator.id)
            .bind(operator.name_ref())
            .bind(now_secs)
            .bind(operator.id)
            .bind(operator.name_ref())
            .bind(IS_DEL_NO)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_err)?;
        Ok(res)
    }

    /// 更新标签名称
    /// （对应 Java `existing.setTagName(name); memberTagMapper.updateById(existing)`）。
    ///
    /// 净效果 = 只改 `tag_name` + 刷新 `update_time` / `update_by*`
    /// （`shop_id` / `group_id` 被写回原值）。
    pub async fn update_tag_name(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i64,
        tag_name: &str,
        operator: &AdminOperator,
        now_secs: i64,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE member_tag SET tag_name = $1, update_time = $2, update_by_id = $3, update_by_name = $4 WHERE id = $5 AND is_del = $6",
        )
        .bind(tag_name)
        .bind(now_secs)
        .bind(operator.id)
        .bind(operator.name_ref())
        .bind(id)
        .bind(IS_DEL_NO)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 逻辑删除标签（对应 Java `memberTagMapper.deleteById(id)`）。
    ///
    /// 同 [`Self::logical_delete_group`]，`@TableLogic` 使其成为
    /// `UPDATE member_tag SET is_del = 1 WHERE id = ? AND is_del = 0`。
    pub async fn logical_delete_tag(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i64,
    ) -> Result<u64> {
        let res = sqlx::query("UPDATE member_tag SET is_del = $1 WHERE id = $2 AND is_del = $3")
            .bind(IS_DEL_YES)
            .bind(id)
            .bind(IS_DEL_NO)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— user_member_tag 清理（事务内） —————————————————————

    /// 删除某店铺下某个标签的全部用户关联
    /// （对应 Java `update` 里的
    /// `userMemberTagMapper.delete(eq(shopId).eq(tagId, t.getId()))`）。
    ///
    /// ⚠️ **这是物理删除 `DELETE`**，不是逻辑删除：`UserMemberTag` 是独立
    /// POJO，既不继承 `BasePO` 也无 `@TableLogic`，且 `user_member_tag` 表
    /// **没有** `is_del` 列（`schema_parser.py --table user_member_tag` 只有
    /// 5 列）。这一非对称性必须保留，否则会与 Java 行为分叉。
    pub async fn delete_user_tag_links(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        shop_id: i32,
        tag_id: i64,
    ) -> Result<u64> {
        let res = sqlx::query("DELETE FROM user_member_tag WHERE shop_id = $1 AND tag_id = $2")
            .bind(shop_id)
            .bind(tag_id)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 批量删除某店铺下多个标签的用户关联
    /// （对应 Java `delGroup` 里的
    /// `userMemberTagMapper.delete(eq(shopId).in(tagId, tagIds))`）。
    ///
    /// `tag_ids` 为空时**不执行** SQL 直接返回 0
    /// —— 复刻 Java 的 `if (!tagIds.isEmpty())` 守卫
    /// （`IN ()` 在 MySQL 里是语法错误）。
    pub async fn delete_user_tag_links_in(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        shop_id: i32,
        tag_ids: &[i64],
    ) -> Result<u64> {
        if tag_ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("DELETE FROM user_member_tag WHERE shop_id = ");
        qb.push_bind(shop_id);
        qb.push(" AND tag_id IN (");
        let mut sep = qb.separated(", ");
        for id in tag_ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        let res = qb.build().execute(&mut **tx).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }
}

/// 数据库错误映射（与 member_tag / area_code / tax_rate 同一约定）。
///
/// ⚠️ 文案用 [`MSG_GROUP_NOT_FOUND`]：本域所有「查不到」语义最终都由服务层
/// 归一到 `标签组不存在`，这里只是兜底（`fetch_optional` 不产生
/// `RowNotFound`，仅 `fetch_one` 会产生）。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound(MSG_GROUP_NOT_FOUND.into()),
        other => Error::Database(other.to_string()),
    }
}

/// 把实体列表转成差异计算用的精简结构
/// （对应 Java `existingTags` → `Map<Long, MemberTag>` 的键值取用点）。
#[must_use]
pub fn to_existing_tags(tags: &[MemberTag]) -> Vec<ExistingTag> {
    tags.iter()
        .map(|t| ExistingTag {
            id: t.id,
            tag_name: t.tag_name.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::member::member_tag_group::{MEMBER_TAG_COLUMNS, MEMBER_TAG_GROUP_COLUMNS};

    /// `MEMBER_TAG_GROUP_SELECT` 必须覆盖 `member_tag_group` 全部 11 列
    /// （`FromRow` 按列名取列），且不得用 `SELECT *`。
    #[test]
    fn member_tag_group_select_covers_all_columns() {
        for column in MEMBER_TAG_GROUP_COLUMNS {
            assert!(
                MEMBER_TAG_GROUP_SELECT.contains(column),
                "SELECT 清单缺少列 {column}"
            );
        }
        assert!(MEMBER_TAG_GROUP_SELECT.starts_with("SELECT "));
        assert!(MEMBER_TAG_GROUP_SELECT.contains("FROM member_tag_group"));
        assert!(!MEMBER_TAG_GROUP_SELECT.contains('*'));
    }

    /// `MEMBER_TAG_SELECT` 同理（11 列）。
    #[test]
    fn member_tag_select_covers_all_columns() {
        for column in MEMBER_TAG_COLUMNS {
            assert!(
                MEMBER_TAG_SELECT.contains(column),
                "SELECT 清单缺少列 {column}"
            );
        }
        assert!(MEMBER_TAG_SELECT.contains("FROM member_tag"));
        assert!(!MEMBER_TAG_SELECT.contains('*'));
    }

    /// INSERT 列清单必须覆盖除主键外的全部列，且列数与占位符数一致
    /// （数量不匹配会在运行期报错，编译器查不出）。
    #[test]
    fn insert_statements_have_matching_placeholder_counts() {
        for (sql, table, expected_cols) in [
            (MEMBER_TAG_GROUP_INSERT, "member_tag_group", 10_usize),
            (MEMBER_TAG_INSERT, "member_tag", 10_usize),
        ] {
            assert!(sql.contains(table), "语句未指向 {table}: {sql}");
            let cols_part = sql
                .split('(')
                .nth(1)
                .and_then(|s| s.split(')').next())
                .expect("INSERT 应带列清单");
            let cols: Vec<&str> = cols_part.split(',').map(str::trim).collect();
            assert_eq!(cols.len(), expected_cols, "{table} 列数不对: {cols:?}");
            let placeholders = sql.matches('$').count();
            assert_eq!(placeholders, expected_cols, "{table} 占位符数不对: {sql}");
        }
    }

    /// INSERT 的列必须全部命中 schema 白名单（`id` 是自增列，不写）。
    #[test]
    fn insert_columns_exist_in_schema() {
        let group_cols_part = MEMBER_TAG_GROUP_INSERT
            .split('(')
            .nth(1)
            .and_then(|s| s.split(')').next())
            .unwrap();
        let group_cols: Vec<&str> = group_cols_part.split(',').map(str::trim).collect();
        for c in &group_cols {
            assert!(
                MEMBER_TAG_GROUP_COLUMNS.contains(c),
                "member_tag_group 无此列: {c}"
            );
        }
        // 自增主键 `id` 不应显式写入
        assert!(!group_cols.contains(&"id"), "自增主键不应显式写入: {group_cols:?}");

        let tag_cols_part = MEMBER_TAG_INSERT
            .split('(')
            .nth(1)
            .and_then(|s| s.split(')').next())
            .unwrap();
        let tag_cols: Vec<&str> = tag_cols_part.split(',').map(str::trim).collect();
        for c in &tag_cols {
            assert!(MEMBER_TAG_COLUMNS.contains(c), "member_tag 无此列: {c}");
        }
        assert!(!tag_cols.contains(&"id"), "自增主键不应显式写入: {tag_cols:?}");
    }

    /// 逻辑删除的取值必须与 `application.yml` 的 `@TableLogic` 配置一致。
    #[test]
    fn logical_delete_values_match_table_logic() {
        assert_eq!(IS_DEL_YES, 1, "logic-delete-value 应为 1");
        assert_eq!(IS_DEL_NO, 0, "logic-not-delete-value 应为 0");
    }

    /// `user_member_tag` 无 `is_del` 列 ⇒ 清关联只能是物理 `DELETE`。
    /// 若哪天 schema 加了该列，本测试会失败并提醒重新核对非对称行为。
    #[test]
    fn user_tag_link_cleanup_is_hard_delete() {
        use nextrade_core::models::member::member_tag_group::USER_MEMBER_TAG_COLUMNS;
        assert!(
            !USER_MEMBER_TAG_COLUMNS.contains(&"is_del"),
            "user_member_tag 若新增 is_del 列，需重新核对 delGroup 的删除语义"
        );
        assert!(USER_MEMBER_TAG_COLUMNS.contains(&"shop_id"));
        assert!(USER_MEMBER_TAG_COLUMNS.contains(&"tag_id"));
    }

    /// 实体 → 差异计算结构的转换只取 `id` / `tag_name`。
    #[test]
    fn to_existing_tags_keeps_id_and_name_only() {
        let tags = vec![MemberTag {
            id: 3,
            shop_id: 0,
            group_id: 9,
            tag_name: "A".into(),
            is_del: IS_DEL_NO,
            ..Default::default()
        }];
        let out = to_existing_tags(&tags);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, 3);
        assert_eq!(out[0].tag_name, "A");
    }
}