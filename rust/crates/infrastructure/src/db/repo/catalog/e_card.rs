//! 电子卡券域仓储：对应原 Java `ECardMapper`。
//!
//! 列名严格对齐权威 schema（`python tools/schema_parser.py --table e_card`）：
//! `card_id` / `group_id` / `card_number` / `card_pwd` / `is_use` /
//! `order_id` / `order_item_id` / `add_time` / `up_time`。
//!
//! 所有列表 SQL 显式列出列名，不用 `SELECT *`。
//!
//! 查询统一使用运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`
//! （**不使用** `query!` / `query_as!` 宏：本机无 `DATABASE_URL`）。
//!
//! # Java 语义对照
//!
//! - `list`：`eq(group_id, pageQuery.groupId)`（**无店铺过滤** —— `e_card`
//!   没有 `shop_id` 列）+ `keyword` 对 `card_number` 做 `ILIKE` +
//!   `buildSortOrder(BasePage)`（支持 `sortItems`，见 models）。
//! - `create` / `update`：MyBatis-Plus `save` / `updateById` 的
//!   `FieldStrategy.NOT_NULL` —— 只写非 null 字段。`ECardEditParam.createECard`
//!   显式把 `addTime` 置空 ⇒ 更新时**不写** `add_time`。
//! - `updateField`：`UPDATE e_card SET <白名单列> = ? WHERE card_id = ?`
//!   （列名由服务层用 `E_CARD_FIELDS` 白名单归一后传入）。

use sqlx::{Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::ECard;
use nextrade_core::models::catalog::e_card::ECardPageQuery;
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// `e_card` 固定 SELECT 列清单（顺序与实体字段一致）。
const CARD_SELECT: &str = "SELECT card_id, group_id, card_number, card_pwd, is_use, order_id, order_item_id, add_time, up_time FROM e_card";

/// 新增电子卡券时待写入的字段。
///
/// `card_id` 由库自增；`order_id` / `order_item_id` 不传，由列默认值 `0` 兜底
/// （等价 Java `save` 的 NOT_NULL 策略）。
#[derive(Debug, Clone, Default)]
pub struct ECardInsert {
    /// 卡券分组 ID
    pub group_id: i32,
    /// 卡号
    pub card_number: String,
    /// 卡号密码
    pub card_pwd: String,
    /// 0:未使用 1:已使用（Java 未传时由列默认值 0 兜底，服务层已补齐）
    pub is_use: i32,
    /// 添加时间（秒级时间戳）
    pub add_time: i32,
    /// 更新时间（秒级时间戳）
    pub up_time: i32,
}

/// 更新电子卡券时的字段集合（对应 Java `updateById` + NOT_NULL 策略）。
///
/// `group_id` / `card_number` / `card_pwd` / `up_time` 总是写入（Java
/// `ECardEditParam.createECard` 必带）；`is_use` 仅在 DTO 传了值时写入；
/// `add_time` 被 Java 显式置空 ⇒ **不写**（保留原值）。
#[derive(Debug, Clone, Default)]
pub struct ECardUpdate {
    /// 卡券分组 ID
    pub group_id: i32,
    /// 卡号
    pub card_number: String,
    /// 卡号密码
    pub card_pwd: String,
    /// 是否使用（`None` ⇒ 不更新该列）
    pub is_use: Option<i32>,
    /// 更新时间（秒级时间戳）
    pub up_time: i32,
}

/// 电子卡券域仓储（持有连接池）。
#[derive(Clone)]
pub struct ECardRepository {
    pool: PgPool,
}

impl ECardRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层事务编排；本域当前无多语句事务）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 查询 —————————————————————

    /// 按主键查询（对应 Java `getById`）。
    pub async fn find_by_id(&self, card_id: i32) -> Result<Option<ECard>> {
        sqlx::query_as::<_, ECard>(&format!("{CARD_SELECT} WHERE card_id = $1 LIMIT 1"))
            .bind(card_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 统计同卡号数量（对应 Java `checkCardUnique` 的
    /// `lambdaQuery().eq(cardNumber, ...).count()`；**全局唯一**，不区分分组）。
    pub async fn count_by_card_number(&self, card_number: &str) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM e_card WHERE card_number = $1")
                .bind(card_number)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0)
    }

    /// 卡券分页（对应 Java `ECardServiceImpl#list`）。
    ///
    /// - `group_id = ?`：**恒定**过滤（Java 校验 `groupId @NotNull` 后必传）
    /// - `card_number ILIKE %keyword%`：仅当 keyword 非空
    /// - ORDER BY 由 [`ECardPageQuery::order_clause`] 生成（支持 sortItems）
    pub async fn paginate(
        &self,
        q: &ECardPageQuery,
        group_id: i32,
        page: &PageQuery,
    ) -> Result<(u64, Vec<ECard>)> {
        let (limit, offset) = page.limit_offset();

        let mut count_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM e_card WHERE 1 = 1");
        Self::push_filters(&mut count_qb, q, group_id);
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut row_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{CARD_SELECT} WHERE 1 = 1"));
        Self::push_filters(&mut row_qb, q, group_id);
        // ORDER BY 列名来自 models 白名单（未知回退主键），方向二选一，
        // LIMIT/OFFSET 为 u64 字面量，均无注入面。
        row_qb.push(q.order_clause());
        row_qb.push(format!(" LIMIT {limit} OFFSET {offset}"));

        let rows: Vec<ECard> = row_qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 追加 `list` 的 WHERE 片段（count 与取行共用，保证条件一致）。
    fn push_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &ECardPageQuery, group_id: i32) {
        // 分组过滤恒定存在（Java `eq(ECard::getGroupId, pageQuery.getGroupId())`）
        qb.push(" AND group_id = ");
        qb.push_bind(group_id);
        if let Some(keyword) = q.effective_keyword() {
            qb.push(" AND card_number ILIKE ");
            qb.push_bind(format!("%{keyword}%"));
        }
    }

    // ————————————————————— 写操作 —————————————————————

    /// 新增电子卡券，返回自增主键（对应 Java `save`）。
    pub async fn insert(&self, row: &ECardInsert) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO e_card (group_id, card_number, card_pwd, is_use, add_time, up_time) VALUES ($1, $2, $3, $4, $5, $6) RETURNING card_id",
        )
        .bind(row.group_id)
        .bind(&row.card_number)
        .bind(&row.card_pwd)
        .bind(row.is_use)
        .bind(row.add_time)
        .bind(row.up_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 更新电子卡券（对应 Java `updateById` 的 NOT_NULL 语义）。
    ///
    /// `add_time` **不写**（Java `ECardEditParam.createECard` 把它置空），
    /// `order_id` / `order_item_id` 也不在 DTO 里 ⇒ 保留原值。
    pub async fn update(&self, card_id: i32, patch: &ECardUpdate) -> Result<u64> {
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new("UPDATE e_card SET group_id = ");
        qb.push_bind(patch.group_id);
        qb.push(", card_number = ");
        qb.push_bind(&patch.card_number);
        qb.push(", card_pwd = ");
        qb.push_bind(&patch.card_pwd);
        if let Some(is_use) = patch.is_use {
            qb.push(", is_use = ");
            qb.push_bind(is_use);
        }
        qb.push(", up_time = ");
        qb.push_bind(patch.up_time);
        qb.push(" WHERE card_id = ");
        qb.push_bind(card_id);
        let res = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 单字段更新（对应 Java `BaseServiceImpl#updateField`）。
    ///
    /// `column` **必须**来自 [`nextrade_core::models::catalog::e_card::E_CARD_FIELDS`]
    /// （服务层负责），此处仅调试断言；`val` 以文本绑定，由 MySQL 按列类型转。
    pub async fn update_field(&self, card_id: i32, column: &str, val: &str) -> Result<u64> {
        debug_assert!(
            nextrade_core::models::catalog::e_card::E_CARD_FIELDS.contains(&column),
            "update_field 列名未经过白名单映射: {column}"
        );
        let sql = format!("UPDATE e_card SET {column} = $1 WHERE card_id = $2");
        let res = sqlx::query(&sql)
            .bind(val)
            .bind(card_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 按主键硬删除（对应 Java `removeById`；表无删除标记列）。
    pub async fn delete(&self, card_id: i32) -> Result<bool> {
        let res = sqlx::query("DELETE FROM e_card WHERE card_id = $1")
            .bind(card_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 批量删除（对应 Java `removeByIds`）。返回实际删除行数。
    pub async fn delete_batch(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new("DELETE FROM e_card WHERE card_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        let res = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 订单商品项已分配的卡密（客户端订单详情 `eCard` 字段）。
    pub async fn find_by_order_item_id(&self, order_item_id: i32) -> Result<Vec<ECard>> {
        sqlx::query_as::<_, ECard>(&format!(
            "{CARD_SELECT} WHERE order_item_id = $1 ORDER BY card_id ASC"
        ))
        .bind(order_item_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 为订单分配未使用的卡密（对应 Java eCardService.getNewCardByGroupId）。
    pub async fn allocate_cards_to_order_item(
        &self,
        group_id: i32,
        order_id: i32,
        order_item_id: i32,
        count: i32,
    ) -> Result<Vec<ECard>> {
        let cards: Vec<ECard> = sqlx::query_as(
            "SELECT card_id, group_id, card_number, card_pwd, is_use, order_id, order_item_id, add_time, up_time FROM e_card WHERE group_id = $1 AND is_use = 0 LIMIT $2",
        )
        .bind(group_id)
        .bind(count)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        if cards.len() < count as usize {
            return Err(Error::biz_msg("电子卡券库存不足"));
        }

        let now = nextrade_common::utils::now_seconds() as i32;
        for c in &cards {
            sqlx::query(
                "UPDATE e_card SET is_use = 1, order_id = $1, order_item_id = $2, up_time = $3 WHERE card_id = $4",
            )
            .bind(order_id)
            .bind(order_item_id)
            .bind(now)
            .bind(c.card_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        }

        Ok(cards)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => {
            Error::NotFound(nextrade_core::models::catalog::e_card::CARD_NOT_EXISTS.into())
        }
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::catalog::e_card::CARD_COLUMNS;

    #[test]
    fn select_list_covers_entity_columns() {
        assert_eq!(
            CARD_COLUMNS,
            [
                "card_id",
                "group_id",
                "card_number",
                "card_pwd",
                "is_use",
                "order_id",
                "order_item_id",
                "add_time",
                "up_time",
            ]
        );
        let selected = CARD_SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap();
        assert_eq!(selected, CARD_COLUMNS.join(", "));
    }

    #[test]
    fn insert_defaults_are_zeroed() {
        let row = ECardInsert::default();
        assert_eq!(row.group_id, 0);
        assert_eq!(row.is_use, 0);
        assert_eq!(row.add_time, 0);
        assert_eq!(row.up_time, 0);
        assert!(row.card_number.is_empty());
    }

    #[test]
    fn update_patch_only_is_use_is_optional() {
        let patch = ECardUpdate::default();
        assert_eq!(patch.is_use, None);
        assert_eq!(patch.up_time, 0);
    }
}
