//! 礼品卡分组域仓储（marketing::gift_card_groups）。
//!
//! 对应原 Java `GiftCardGroupsMapper` / `GiftCardProductsMapper` / `ProductMapper`。
//!
//! 表名/列名严格对齐 `java/database/pure.sql`
//! （核对命令：`cd rust && python tools/schema_parser.py --table gift_card_groups`）：
//! - `gift_card_groups`（15 列）
//! - `gift_card_products`：`gift_product_id` / `card_groups_id` / `product_id` / `sku_id`
//! - `product`：读 `gift_card_groups_id`（绑定判断）、`product_name` / `pic_thumb`
//! - `product_sku`：读 `sku_value` / `sku_price`
//!
//! SQL 一律运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`。

use rust_decimal::Decimal;
use sqlx::{Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::{GiftCardGroups, GiftCardProducts};
use nextrade_core::models::marketing::gift_card::GiftCardProductVO;
use nextrade_core::models::marketing::gift_card_groups::GiftCardGroupsListQuery;
use nextrade_core::{Error, Result};

/// `gift_card_groups` 固定 SELECT 列清单（15 列，顺序与实体一致）。
pub const GROUPS_COLUMNS: [&str; 15] = [
    "card_groups_id",
    "group_type",
    "card_type",
    "card_group_name",
    "image_url",
    "validity_type",
    "validity_days",
    "expire_time",
    "face_amount",
    "status",
    "add_time",
    "update_time",
    "remark",
    "shop_id",
    "availability_stock",
];

/// `gift_card_products` 固定 SELECT 列清单（4 列）。
pub const PRODUCTS_COLUMNS: [&str; 4] =
    ["gift_product_id", "card_groups_id", "product_id", "sku_id"];

/// 待绑定参数。
#[derive(Debug, Clone)]
pub enum Bind {
    /// 32 位整数
    I32(i32),
    /// 字符串
    Str(String),
}

/// 礼品卡分组域仓储。
#[derive(Clone)]
pub struct GiftCardGroupsRepository {
    pool: PgPool,
}

impl GiftCardGroupsRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }
}

impl GiftCardGroupsRepository {
    // ————————————————————— 查询 —————————————————————

    /// 按主键查询（Java `getById`）。
    pub async fn find_by_id(&self, card_groups_id: i32) -> Result<Option<GiftCardGroups>> {
        let sql = format!(
            "SELECT {} FROM gift_card_groups WHERE card_groups_id = $1 LIMIT 1",
            GROUPS_COLUMNS.join(", ")
        );
        sqlx::query_as::<_, GiftCardGroups>(&sql)
            .bind(card_groups_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 卡组分页（Java `list`：`like(cardGroupName, keyword)` + `eq(shopId)`）。
    pub async fn paginate(
        &self,
        q: &GiftCardGroupsListQuery,
        shop_id: i32,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<GiftCardGroups>)> {
        let (where_sql, binds) = build_where(q, shop_id);
        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM gift_card_groups{where_sql}"));
        let mut count_q = sqlx::query_as::<_, (i64,)>(&count_sql);
        for b in &binds {
            count_q = match b {
                Bind::I32(v) => count_q.bind(*v),
                Bind::Str(v) => count_q.bind(v.as_str()),
            };
        }
        let total: (i64,) = count_q.fetch_one(&self.pool).await.map_err(map_err)?;

        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {} FROM gift_card_groups{where_sql}{} LIMIT {limit} OFFSET {offset}",
            GROUPS_COLUMNS.join(", "),
            q.order_clause()
        ));
        let mut rows_q = sqlx::query_as::<_, GiftCardGroups>(&rows_sql);
        for b in &binds {
            rows_q = match b {
                Bind::I32(v) => rows_q.bind(*v),
                Bind::Str(v) => rows_q.bind(v.as_str()),
            };
        }
        let rows = rows_q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total.0, rows))
    }

    /// 按卡类型取启用的卡组（Java `getGiftCardGroupsByCardType`），并排除已绑定商品的组。
    pub async fn list_by_card_type(
        &self,
        card_type: i32,
        shop_id: i32,
        exclude_group_ids: &[i32],
    ) -> Result<Vec<GiftCardGroups>> {
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!(
            "SELECT {} FROM gift_card_groups WHERE card_type = ",
            GROUPS_COLUMNS.join(", ")
        ));
        qb.push_bind(card_type);
        qb.push(" AND status = 1 AND shop_id = ");
        qb.push_bind(shop_id);
        if !exclude_group_ids.is_empty() {
            qb.push(" AND card_groups_id NOT IN (");
            let mut sep = qb.separated(", ");
            for id in exclude_group_ids {
                sep.push_bind(*id);
            }
            sep.push_unseparated(")");
            drop(sep);
        }
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 已被商品绑定的卡组 ID 列表（Java：`product.gift_card_groups_id IS NOT NULL`,
    /// 可选排除当前组；Java 此处**不**加店铺过滤，逐字保持）。
    pub async fn bound_group_ids(
        &self,
        exclude_group_id: Option<i32>,
    ) -> Result<Vec<i32>> {
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT DISTINCT gift_card_groups_id FROM product WHERE gift_card_groups_id IS NOT NULL",
        );
        if let Some(id) = exclude_group_id {
            qb.push(" AND gift_card_groups_id <> ");
            qb.push_bind(id);
        }
        let rows: Vec<(i32,)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 有效商品绑定的卡组数量（Java `productMapper.selectCount(...)`；
    /// 条件：`gift_card_groups_id` / `shop_id` / `product_status=1` / `is_delete=0`）。
    pub async fn bound_product_count(&self, card_groups_id: i32, shop_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM product WHERE gift_card_groups_id = $1 AND shop_id = $2 AND product_status = 1 AND is_delete = 0",
        )
        .bind(card_groups_id)
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 卡组绑定的商品记录（`gift_card_products`）。
    pub async fn find_group_products(
        &self,
        card_groups_id: i32,
    ) -> Result<Vec<GiftCardProducts>> {
        let sql = format!(
            "SELECT {} FROM gift_card_products WHERE card_groups_id = $1",
            PRODUCTS_COLUMNS.join(", ")
        );
        sqlx::query_as::<_, GiftCardProducts>(&sql)
            .bind(card_groups_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 卡组商品明细（Java `productMapper.selectProduct(productId, skuId)` 的批量简化版）。
    pub async fn find_product_vos(&self, card_groups_id: i32) -> Result<Vec<GiftCardProductVO>> {
        let rows: Vec<(i32, i32, String, String, Option<String>, Option<Decimal>)> =
            sqlx::query_as(
                "SELECT gp.product_id, gp.sku_id, p.product_name, p.pic_thumb, s.sku_value, s.sku_price FROM gift_card_products gp INNER JOIN product p ON p.product_id = gp.product_id INNER JOIN product_sku s ON s.sku_id = gp.sku_id WHERE gp.card_groups_id = $1",
            )
            .bind(card_groups_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(
                |(product_id, sku_id, product_name, pic_thumb, sku_value, sku_price)| {
                    GiftCardProductVO {
                        product_id,
                        sku_id,
                        product_name,
                        pic_thumb,
                        sku_value,
                        sku_price,
                    }
                },
            )
            .collect())
    }
}

impl GiftCardGroupsRepository {
    // ————————————————————— 写操作 —————————————————————

    /// 新建卡组（Java `save`），返回自增主键。
    pub async fn insert(&self, group: &GiftCardGroups) -> Result<i32> {
        let sql = format!(
            "INSERT INTO gift_card_groups ({}) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
            GROUPS_COLUMNS[1..].join(", ")
        );
        let res: i32 = sqlx::query_scalar(&format!("{} RETURNING card_groups_id", &sql))
            .bind(group.group_type)
            .bind(group.card_type)
            .bind(&group.card_group_name)
            .bind(&group.image_url)
            .bind(group.validity_type)
            .bind(group.validity_days)
            .bind(group.expire_time)
            .bind(group.face_amount)
            .bind(group.status)
            .bind(group.add_time)
            .bind(group.update_time)
            .bind(&group.remark)
            .bind(group.shop_id)
            .bind(group.availability_stock)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res)
    }

    /// 整行更新（Java `updateById`；服务层已合并旧值）。
    pub async fn update(&self, group: &GiftCardGroups) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE gift_card_groups SET group_type = $1, card_type = $2, card_group_name = $3, image_url = $4, validity_type = $5, validity_days = $6, expire_time = $7, face_amount = $8, status = $9, add_time = $10, update_time = $11, remark = $12, shop_id = $13, availability_stock = $14 WHERE card_groups_id = $15",
        )
        .bind(group.group_type)
        .bind(group.card_type)
        .bind(&group.card_group_name)
        .bind(&group.image_url)
        .bind(group.validity_type)
        .bind(group.validity_days)
        .bind(group.expire_time)
        .bind(group.face_amount)
        .bind(group.status)
        .bind(group.add_time)
        .bind(group.update_time)
        .bind(&group.remark)
        .bind(group.shop_id)
        .bind(group.availability_stock)
        .bind(group.card_groups_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 更新状态 + 更新时间（Java `lambdaUpdate().set(updateTime).set(status)`）。
    pub async fn update_status(
        &self,
        card_groups_id: i32,
        status: i32,
        update_time: i32,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE gift_card_groups SET status = $1, update_time = $2 WHERE card_groups_id = $3",
        )
        .bind(status)
        .bind(update_time)
        .bind(card_groups_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 读取卡组兑换字段（P1 新增列 exchange_mode / select_count）。
    pub async fn find_exchange_fields(&self, card_groups_id: i32) -> Result<Option<(i32, i32)>> {
        let row: Option<(i32, i32)> = sqlx::query_as(
            "SELECT exchange_mode, select_count FROM gift_card_groups WHERE card_groups_id = $1 LIMIT 1",
        )
        .bind(card_groups_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row)
    }

    /// 写入卡组兑换字段（P1 新增列；create/update 透传用）。
    pub async fn update_exchange_fields(
        &self,
        card_groups_id: i32,
        exchange_mode: i32,
        select_count: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE gift_card_groups SET exchange_mode = $1, select_count = $2 WHERE card_groups_id = $3",
        )
        .bind(exchange_mode)
        .bind(select_count)
        .bind(card_groups_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 读取卡组适用范围（我方新增，迁移 012）：scope_type 列 + 两张范围明细表。
    /// 返回 (scope_type, 商品 id 集合, 品类 id 集合-原始未展开)；卡组不存在返回 None。
    pub async fn find_scope(
        &self,
        card_groups_id: i32,
    ) -> Result<Option<(i32, Vec<i32>, Vec<i32>)>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT scope_type FROM gift_card_groups WHERE card_groups_id = $1 LIMIT 1",
        )
        .bind(card_groups_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        let Some((scope_type,)) = row else {
            return Ok(None);
        };
        if scope_type == 0 {
            return Ok(Some((scope_type, Vec::new(), Vec::new())));
        }
        let product_ids: Vec<i32> = sqlx::query_scalar(
            "SELECT product_id FROM gift_card_group_scope_products WHERE card_groups_id = $1 ORDER BY id",
        )
        .bind(card_groups_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        let category_ids: Vec<i32> = sqlx::query_scalar(
            "SELECT category_id FROM gift_card_group_scope_categories WHERE card_groups_id = $1 ORDER BY id",
        )
        .bind(card_groups_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(Some((scope_type, product_ids, category_ids)))
    }

    /// 覆盖写入卡组适用范围（事务：更新 scope_type 列 + 全量替换两张明细表）。
    pub async fn replace_scope(
        &self,
        card_groups_id: i32,
        scope_type: i32,
        product_ids: &[i32],
        category_ids: &[i32],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        sqlx::query("UPDATE gift_card_groups SET scope_type = $1 WHERE card_groups_id = $2")
            .bind(scope_type)
            .bind(card_groups_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        sqlx::query("DELETE FROM gift_card_group_scope_products WHERE card_groups_id = $1")
            .bind(card_groups_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        sqlx::query("DELETE FROM gift_card_group_scope_categories WHERE card_groups_id = $1")
            .bind(card_groups_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        if scope_type == 1 {
            for product_id in product_ids {
                sqlx::query(
                    "INSERT INTO gift_card_group_scope_products (card_groups_id, product_id) VALUES ($1, $2) ON CONFLICT (card_groups_id, product_id) DO NOTHING",
                )
                .bind(card_groups_id)
                .bind(product_id)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
            }
        }
        if scope_type == 2 {
            for category_id in category_ids {
                sqlx::query(
                    "INSERT INTO gift_card_group_scope_categories (card_groups_id, category_id) VALUES ($1, $2) ON CONFLICT (card_groups_id, category_id) DO NOTHING",
                )
                .bind(card_groups_id)
                .bind(category_id)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
            }
        }
        tx.commit().await.map_err(map_err)?;
        Ok(())
    }

    /// 全量品类父子对 (category_id, parent_id)，供范围品类按树展开子类目。
    pub async fn list_category_pairs(&self) -> Result<Vec<(i32, i32)>> {
        let rows: Vec<(i32, i32)> =
            sqlx::query_as("SELECT category_id, parent_id FROM category")
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows)
    }

    /// 删除卡组（Java `removeById`；注意 Java 是物理删除）。
    /// 我方新增：同事务清理适用范围明细行，避免孤儿范围数据。
    pub async fn delete(&self, card_groups_id: i32) -> Result<u64> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        sqlx::query("DELETE FROM gift_card_group_scope_products WHERE card_groups_id = $1")
            .bind(card_groups_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        sqlx::query("DELETE FROM gift_card_group_scope_categories WHERE card_groups_id = $1")
            .bind(card_groups_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        let res = sqlx::query("DELETE FROM gift_card_groups WHERE card_groups_id = $1")
            .bind(card_groups_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        let affected = res.rows_affected();
        tx.commit().await.map_err(map_err)?;
        Ok(affected)
    }

    /// 删除卡组下的商品绑定（Java `giftCardProductsMapper.delete(...)`）。
    pub async fn delete_products(&self, card_groups_id: i32) -> Result<u64> {
        let res = sqlx::query("DELETE FROM gift_card_products WHERE card_groups_id = $1")
            .bind(card_groups_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 批量插入商品绑定（Java `giftCardProductsMapper.insert(list)`）。
    pub async fn insert_products(&self, items: &[(i32, i32, i32)]) -> Result<u64> {
        if items.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("INSERT INTO gift_card_products (card_groups_id, product_id, sku_id) VALUES ");
        // 不要用 separated()：它会在每次 push 前自动加分隔符，再手动 push(", ")
        // 会得到 `?, , , ?, , , ?, )` 这种非法 SQL。
        for (idx, (group_id, product_id, sku_id)) in items.iter().enumerate() {
            if idx > 0 {
                qb.push(", ");
            }
            qb.push("(")
                .push_bind(*group_id)
                .push(", ")
                .push_bind(*product_id)
                .push(", ")
                .push_bind(*sku_id)
                .push(")");
        }
        let res = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }
}

/// 拼装卡组列表 WHERE（Java：`.like(isNotBlank(keyword), cardGroupName).eq(shopId)`）。
#[must_use]
pub fn build_where(q: &GiftCardGroupsListQuery, shop_id: i32) -> (String, Vec<Bind>) {
    let mut conds = vec!["shop_id = ?".to_string()];
    let mut binds = vec![Bind::I32(shop_id)];
    if let Some(keyword) = q.effective_keyword() {
        conds.push("card_group_name ILIKE ?".to_string());
        binds.push(Bind::Str(format!("%{keyword}%")));
    }
    (format!(" WHERE {}", conds.join(" AND ")), binds)
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(json: serde_json::Value) -> GiftCardGroupsListQuery {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn where_filters_shop_and_optional_keyword() {
        let (sql, binds) = build_where(&q(serde_json::json!({})), 7);
        assert_eq!(sql, " WHERE shop_id = ?");
        assert!(matches!(binds.as_slice(), [Bind::I32(7)]));
        let (sql, binds) = build_where(&q(serde_json::json!({"keyword": " 新年 "})), 7);
        assert_eq!(sql, " WHERE shop_id = ? AND card_group_name ILIKE ?");
        assert!(matches!(&binds[1], Bind::Str(s) if s == "%新年%"));
    }

    #[test]
    fn select_columns_match_entities() {
        assert_eq!(GROUPS_COLUMNS.len(), 15);
        assert_eq!(GROUPS_COLUMNS[0], "card_groups_id");
        assert_eq!(PRODUCTS_COLUMNS.len(), 4);
        // INSERT 跳过自增主键，列数 = 15 - 1
        let src = include_str!("gift_card_groups.rs");
        assert!(src.contains("INSERT INTO gift_card_groups (group_type, card_type"));
        assert!(src.contains("DELETE FROM gift_card_products WHERE card_groups_id = $1"));
    }
}
