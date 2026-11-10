//! 电子卡券组域仓储：对应原 Java `ECardGroupMapper` + `ProductMapper` 的只读查询。
//!
//! 列名严格对齐权威 schema（`python tools/schema_parser.py --table e_card_group`）：
//! `group_id` / `group_name` / `shop_id` / `remark` / `is_use` / `add_time` /
//! `up_time`。
//!
//! 另读 `product` 表两列组合（`card_group_id` / `is_delete` /
//! `product_status`）判断「分组是否被上架商品占用」——这是 Java
//! `ECardGroupServiceImpl#del` 与 `#updateField` 的同一段查询：
//! ```text
//! eq(Product::getCardGroupId, id)
//! .eq(Product::getIsDelete, 0)      // DeleteType.NOT_DELETE
//! .eq(Product::getProductStatus, 1) // ProductStatus.ON_SALE
//! ```
//!
//! 查询统一使用运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`
//! （**不使用** `query!` / `query_as!` 宏：本机无 `DATABASE_URL`）。

use sqlx::{Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::ECardGroup;
use nextrade_core::models::catalog::e_card_group::ECardGroupListQuery;
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// `e_card_group` 固定 SELECT 列清单（顺序与实体字段一致）。
const GROUP_SELECT: &str = "SELECT group_id, group_name, shop_id, remark, is_use, add_time, up_time FROM e_card_group";

/// 「上架商品占用分组」计数 SQL（Java `del` / `updateField` 的同一段条件）。
const BOUND_ON_SALE_PRODUCT_COUNT_SQL: &str =
    "SELECT COUNT(*) FROM product WHERE card_group_id = $1 AND is_delete = 0 AND product_status = 1";

/// 新增电子卡券组时待写入的字段。
///
/// `group_id` 由库自增；`is_use` 由服务层显式写 `0`（Java DTO 无该字段，
/// 走列默认值 0，两者等价）。
#[derive(Debug, Clone, Default)]
pub struct ECardGroupInsert {
    /// 分组名称
    pub group_name: String,
    /// 店铺 ID（服务层用请求头解析值）
    pub shop_id: i32,
    /// 备注
    pub remark: Option<String>,
    /// 0:不使用 1:使用
    pub is_use: i32,
    /// 添加时间（秒级时间戳）
    pub add_time: i32,
    /// 更新时间（秒级时间戳）
    pub up_time: i32,
}

/// 电子卡券组域仓储（持有连接池）。
#[derive(Clone)]
pub struct ECardGroupRepository {
    pool: PgPool,
}

impl ECardGroupRepository {
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

    /// 按主键查询（对应 Java `getById` / `getCount(id) > 0`）。
    pub async fn find_by_id(&self, group_id: i32) -> Result<Option<ECardGroup>> {
        sqlx::query_as::<_, ECardGroup>(&format!("{GROUP_SELECT} WHERE group_id = $1 LIMIT 1"))
            .bind(group_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 统计同名分组数量（对应 Java `checkCardGroupUnique`）。
    ///
    /// ⚠️ Java 的查询**不带 `shop_id`**（`lambdaQuery().eq(groupName, ...)`），
    /// 即分组名平台内全局唯一 —— 本实现保持原样。
    pub async fn count_by_group_name(&self, group_name: &str) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM e_card_group WHERE group_name = $1")
                .bind(group_name)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0)
    }

    /// 统计绑定了该分组的上架商品数量（Java `del` / `updateField` 的前置校验）。
    ///
    /// 条件与 Java 完全一致：`card_group_id = ? AND is_delete = 0
    /// AND product_status = 1`。
    pub async fn count_on_sale_products_bound(&self, group_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(BOUND_ON_SALE_PRODUCT_COUNT_SQL)
            .bind(group_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0)
    }

    /// 分组分页（对应 Java `ECardGroupServiceImpl#list`）。
    ///
    /// - `shop_id = ?`：**恒定**归属过滤（Java `eq(shopId, getShopId())`）
    /// - `group_name ILIKE %keyword%`：仅当 keyword 非空
    /// - ORDER BY 由 [`ECardGroupListQuery::order_clause`] 生成
    pub async fn paginate(
        &self,
        q: &ECardGroupListQuery,
        shop_id: i32,
        page: &PageQuery,
    ) -> Result<(u64, Vec<ECardGroup>)> {
        let (limit, offset) = page.limit_offset();

        let mut count_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM e_card_group WHERE 1 = 1");
        Self::push_filters(&mut count_qb, q, shop_id);
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut row_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{GROUP_SELECT} WHERE 1 = 1"));
        Self::push_filters(&mut row_qb, q, shop_id);
        // ORDER BY 列名来自 models 白名单（未知回退主键），方向二选一，
        // LIMIT/OFFSET 为 u64 字面量，均无注入面。
        row_qb.push(q.order_clause());
        row_qb.push(format!(" LIMIT {limit} OFFSET {offset}"));

        let rows: Vec<ECardGroup> = row_qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 追加 `list` 的 WHERE 片段（count 与取行共用，保证条件一致）。
    fn push_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &ECardGroupListQuery, shop_id: i32) {
        // 归属过滤恒定存在（Java `eq(ECardGroup::getShopId, getShopId())`）
        qb.push(" AND shop_id = ");
        qb.push_bind(shop_id);
        if let Some(keyword) = q.effective_keyword() {
            qb.push(" AND group_name ILIKE ");
            qb.push_bind(format!("%{keyword}%"));
        }
    }

    // ————————————————————— 写操作 —————————————————————

    /// 新增分组，返回自增主键（对应 Java `save`）。
    pub async fn insert(&self, row: &ECardGroupInsert) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO e_card_group (group_name, shop_id, remark, is_use, add_time, up_time) VALUES ($1, $2, $3, $4, $5, $6) RETURNING group_id",
        )
        .bind(&row.group_name)
        .bind(row.shop_id)
        .bind(&row.remark)
        .bind(row.is_use)
        .bind(row.add_time)
        .bind(row.up_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 更新分组（对应 Java `updateById` 的 NOT_NULL 语义）。
    ///
    /// - `group_name` / `up_time` 总是写入（DTO `groupName @NotNull`、Java 显式
    ///   设置 `upTime`）
    /// - `remark` 仅 DTO 传了才写（`None` ⇒ 保留原值）
    /// - `shop_id` / `is_use` 不在 DTO 里 ⇒ 保留原值
    pub async fn update(
        &self,
        group_id: i32,
        group_name: &str,
        remark: Option<&str>,
        up_time: i32,
    ) -> Result<u64> {
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("UPDATE e_card_group SET group_name = ");
        qb.push_bind(group_name);
        if let Some(remark) = remark {
            qb.push(", remark = ");
            qb.push_bind(remark);
        }
        qb.push(", up_time = ");
        qb.push_bind(up_time);
        qb.push(" WHERE group_id = ");
        qb.push_bind(group_id);
        let res = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 单字段更新（对应 Java `BaseServiceImpl#updateField`）。
    ///
    /// `column` **必须**来自
    /// [`nextrade_core::models::catalog::e_card_group::E_CARD_GROUP_FIELDS`]
    /// （服务层负责），此处仅调试断言。
    pub async fn update_field(&self, group_id: i32, column: &str, val: &str) -> Result<u64> {
        debug_assert!(
            nextrade_core::models::catalog::e_card_group::E_CARD_GROUP_FIELDS.contains(&column),
            "update_field 列名未经过白名单映射: {column}"
        );
        let sql = format!("UPDATE e_card_group SET {column} = $1 WHERE group_id = $2");
        let res = sqlx::query(&sql)
            .bind(val)
            .bind(group_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 按主键硬删除（对应 Java `removeById`；表无删除标记列）。
    pub async fn delete(&self, group_id: i32) -> Result<bool> {
        let res = sqlx::query("DELETE FROM e_card_group WHERE group_id = $1")
            .bind(group_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 批量删除（对应 Java `removeByIds`）。返回实际删除行数。
    ///
    /// ⚠️ Java `BaseServiceImpl#batch` **不做**「分组被上架商品占用」校验
    /// （该校验只在 `del` / `updateField` 里）；本实现保持原样。
    pub async fn delete_batch(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("DELETE FROM e_card_group WHERE group_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        let res = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 批量插入电子卡券（Java `eCardService.saveBatch(eCards)`）。
    pub async fn insert_e_cards(&self, group_id: i32, cards: &[(String, String)], now: i32) -> Result<usize> {
        if cards.is_empty() {
            return Ok(0);
        }
        let mut count = 0;
        for chunk in cards.chunks(500) {
            let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
                "INSERT INTO e_card (group_id, card_number, card_pwd, is_use, order_id, order_item_id, add_time) ",
            );
            qb.push_values(chunk, |mut b, (num, pwd)| {
                b.push_bind(group_id)
                    .push_bind(num)
                    .push_bind(pwd)
                    .push_bind(0)
                    .push_bind(0)
                    .push_bind(0)
                    .push_bind(now);
            });
            let res = qb.build().execute(&self.pool).await.map_err(map_err)?;
            count += res.rows_affected() as usize;
        }
        Ok(count)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => {
            Error::NotFound(nextrade_core::models::catalog::e_card_group::GROUP_NOT_EXISTS.into())
        }
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::catalog::e_card_group::GROUP_COLUMNS;

    #[test]
    fn select_list_covers_entity_columns() {
        assert_eq!(
            GROUP_COLUMNS,
            [
                "group_id",
                "group_name",
                "shop_id",
                "remark",
                "is_use",
                "add_time",
                "up_time",
            ]
        );
        let selected = GROUP_SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap();
        assert_eq!(selected, GROUP_COLUMNS.join(", "));
    }

    #[test]
    fn insert_defaults_are_zeroed() {
        let row = ECardGroupInsert::default();
        assert_eq!(row.is_use, 0);
        assert_eq!(row.add_time, 0);
        assert_eq!(row.up_time, 0);
        assert!(row.group_name.is_empty());
        assert_eq!(row.remark, None);
    }

    #[test]
    fn bound_product_query_targets_product_table() {
        // 静态守卫：占用校验必须查 product 的 card_group_id/is_delete/product_status
        // （改 SQL 时若误删条件，这里会提示重新核对 Java）
        assert_eq!(
            BOUND_ON_SALE_PRODUCT_COUNT_SQL,
            "SELECT COUNT(*) FROM product WHERE card_group_id = $1 AND is_delete = 0 AND product_status = 1"
        );
    }
}
