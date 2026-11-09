//! 礼品卡域仓储（marketing::gift_card）。
//!
//! 对应原 Java `GiftCardsMapper`（MP LambdaQueryWrapper / `getOne` / `updateById`）。
//!
//! 表名/列名严格对齐 `java/database/pure.sql`
//! （核对命令：`cd rust && python tools/schema_parser.py --table gift_cards`）：
//! - `gift_cards`（14 列）：`gift_cards_id` / `card_groups_id` / `card_number` /
//!   `card_password` / `gift_card_sn` / `is_available` / `status` / `used_time` /
//!   `user_id` / `add_time` / `expired_time` / `claimed_user_id` /
//!   `claimed_status` / `order_time`
//! - `gift_card_groups`（本域只读组信息 + 调整 `availability_stock`）
//! - `shop`（读 `shop_title` / `shop_logo`）
//! - `user`（读 `username` / `mobile`，兑换详情用）
//! - `order_item` + `order`（兑换详情用，`order_item.gift_card_id` 关联）
//! - `user_recharge_order`（储值卡兑换时插入充值单）
//!
//! SQL 一律运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`
//! （**不用** `query!` 宏，本机无 `DATABASE_URL`）。

use rust_decimal::Decimal;
use sqlx::PgPool;

use nextrade_core::entities::{GiftCardGroups, GiftCards, Order};
use nextrade_core::models::marketing::gift_card::GiftCardsListQuery;
use nextrade_core::{Error, Result};

/// `gift_cards` 固定 SELECT 列清单（顺序与实体一致，14 列）。
pub const GIFT_CARDS_COLUMNS: [&str; 14] = [
    "gift_cards_id",
    "card_groups_id",
    "card_number",
    "card_password",
    "gift_card_sn",
    "is_available",
    "status",
    "used_time",
    "user_id",
    "add_time",
    "expired_time",
    "claimed_user_id",
    "claimed_status",
    "order_time",
];

/// `gift_card_groups` 固定 SELECT 列清单（15 列，服务层只读）。
pub const GIFT_CARD_GROUPS_COLUMNS: [&str; 15] = [
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

/// 待绑定参数（按 SQL 中 `?` 的出现顺序 1:1 对应）。
#[derive(Debug, Clone)]
pub enum Bind {
    /// 32 位整数
    I32(i32),
    /// 字符串
    Str(String),
}

/// 礼品卡域仓储。
#[derive(Clone)]
pub struct GiftCardRepository {
    pool: PgPool,
}

impl GiftCardRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }
}

impl GiftCardRepository {
    // ————————————————————— 礼品卡查询 —————————————————————

    /// 按主键查询（Java `getById`）。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<GiftCards>> {
        let sql = format!(
            "SELECT {} FROM gift_cards WHERE gift_cards_id = $1 LIMIT 1",
            GIFT_CARDS_COLUMNS.join(", ")
        );
        sqlx::query_as::<_, GiftCards>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按卡序列号查询（Java `.eq(giftCardSn, sn).last("limit 1")`）。
    pub async fn find_by_sn(&self, sn: &str) -> Result<Option<GiftCards>> {
        let sql = format!(
            "SELECT {} FROM gift_cards WHERE gift_card_sn = $1 LIMIT 1",
            GIFT_CARDS_COLUMNS.join(", ")
        );
        sqlx::query_as::<_, GiftCards>(&sql)
            .bind(sn)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按卡号+卡密查询（Java `exchangeCard` 的关键字条件）。
    pub async fn find_for_exchange(
        &self,
        card_number: &str,
        card_password: &str,
    ) -> Result<Option<GiftCards>> {
        let sql = format!(
            "SELECT {} FROM gift_cards WHERE card_number = $1 AND card_password = $2 AND is_available = 1 AND status = 0 LIMIT 1",
            GIFT_CARDS_COLUMNS.join(", ")
        );
        sqlx::query_as::<_, GiftCards>(&sql)
            .bind(card_number)
            .bind(card_password)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 卡分页（Java `list`）。
    ///
    /// Java `.eq(cardGroupsId, query.cardGroupsId)`：`cardGroupsId` 为 null 时
    /// SQL 条件 `card_groups_id = NULL` 恒不成立 ⇒ **空页**（此处直接返回空）。
    pub async fn paginate(
        &self,
        q: &GiftCardsListQuery,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<GiftCards>)> {
        let Some(card_groups_id) = q.card_groups_id else {
            return Ok((0, Vec::new()));
        };
        let (where_sql, binds) = build_where(q, card_groups_id);
        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM gift_cards{where_sql}"));
        let mut count_q = sqlx::query_as::<_, (i64,)>(&count_sql);
        for b in &binds {
            count_q = match b {
                Bind::I32(v) => count_q.bind(*v),
                Bind::Str(v) => count_q.bind(v.as_str()),
            };
        }
        let total: (i64,) = count_q
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {} FROM gift_cards{where_sql}{} LIMIT {limit} OFFSET {offset}",
            GIFT_CARDS_COLUMNS.join(", "),
            q.order_clause()
        ));
        let mut rows_q = sqlx::query_as::<_, GiftCards>(&rows_sql);
        for b in &binds {
            rows_q = match b {
                Bind::I32(v) => rows_q.bind(*v),
                Bind::Str(v) => rows_q.bind(v.as_str()),
            };
        }
        let rows = rows_q
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0, rows))
    }

    // ————————————————————— 组 / 店铺 / 用户 —————————————————————

    /// 按卡组取全部卡（Java `lambdaQuery().eq(cardGroupsId)`，exportList 用）。
    pub async fn find_by_group(&self, card_groups_id: i32) -> Result<Vec<GiftCards>> {
        let sql = format!(
            "SELECT {} FROM gift_cards WHERE card_groups_id = $1 ORDER BY gift_cards_id ASC",
            GIFT_CARDS_COLUMNS.join(", ")
        );
        sqlx::query_as::<_, GiftCards>(&sql)
            .bind(card_groups_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 查询卡组（Java `giftCardGroupsService.getById`）。
    pub async fn find_group(&self, card_groups_id: i32) -> Result<Option<GiftCardGroups>> {
        let sql = format!(
            "SELECT {} FROM gift_card_groups WHERE card_groups_id = $1 LIMIT 1",
            GIFT_CARD_GROUPS_COLUMNS.join(", ")
        );
        sqlx::query_as::<_, GiftCardGroups>(&sql)
            .bind(card_groups_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 调整卡组可用库存（Java `cardGroups.setAvailabilityStock(...)` + `updateById`）。
    pub async fn update_group_stock(&self, card_groups_id: i32, stock: i32) -> Result<u64> {
        let res = sqlx::query("UPDATE gift_card_groups SET availability_stock = $1 WHERE card_groups_id = $2")
            .bind(stock)
            .bind(card_groups_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 店铺名/logo（Java `shopService.getById(shopId)`）。
    pub async fn find_shop(&self, shop_id: i32) -> Result<Option<(Option<String>, Option<String>)>> {
        let row: Option<(Option<String>, Option<String>)> =
            sqlx::query_as("SELECT shop_title, shop_logo FROM shop WHERE shop_id = $1 LIMIT 1")
                .bind(shop_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row)
    }

    /// 卡组绑定的商品（简化版 Java `productMapper.selectProduct(productId, skuId)`）。
    ///
    /// 返回 `GiftCardProductVO`（只含商品名/图/规格/价格）；商品或 SKU 已删除时
    /// INNER JOIN 自然跳过（与 Java `selectProduct` 返回 null 后 `continue` 等价）。
    pub async fn find_group_products(
        &self,
        card_groups_id: i32,
    ) -> Result<Vec<nextrade_core::models::marketing::gift_card::GiftCardProductVO>> {
        let rows: Vec<(
            i32,
            i32,
            String,
            String,
            Option<String>,
            Option<Decimal>,
        )> = sqlx::query_as(
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
                    nextrade_core::models::marketing::gift_card::GiftCardProductVO {
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

    /// 用户 `username` + `mobile`（Java `userMapper.selectById`）。
    pub async fn find_user_brief(&self, user_id: i32) -> Result<Option<(String, String)>> {
        let row: Option<(String, String)> =
            sqlx::query_as("SELECT username, mobile FROM \"user\" WHERE user_id = $1 LIMIT 1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row)
    }

    /// 礼品卡关联的订单（Java `orderItemMapper.selectOne(giftCardId=..)` + `orderMapper.selectById`）。
    pub async fn find_order_by_gift_card(&self, gift_cards_id: i32) -> Result<Option<Order>> {
        let item: Option<(i32,)> = sqlx::query_as(
            "SELECT order_id FROM order_item WHERE gift_card_id = $1 LIMIT 1",
        )
        .bind(gift_cards_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        let Some((order_id,)) = item else {
            return Ok(None);
        };
        sqlx::query_as::<_, Order>("SELECT * FROM \"order\" WHERE order_id = $1 LIMIT 1")
            .bind(order_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }
}

impl GiftCardRepository {
    // ————————————————————— 礼品卡写操作 —————————————————————

    /// 整行更新（Java `updateById`；服务层已合并旧值）。
    pub async fn update(&self, card: &GiftCards) -> Result<u64> {
        let sql = "UPDATE gift_cards SET card_groups_id = $1, card_number = $2, card_password = $3, gift_card_sn = $4, is_available = $5, status = $6, used_time = $7, user_id = $8, add_time = $9, expired_time = $10, claimed_user_id = $11, claimed_status = $12, order_time = $13 WHERE gift_cards_id = $14";
        let res = sqlx::query(sql)
            .bind(card.card_groups_id)
            .bind(&card.card_number)
            .bind(&card.card_password)
            .bind(&card.gift_card_sn)
            .bind(card.is_available)
            .bind(card.status)
            .bind(card.used_time)
            .bind(card.user_id)
            .bind(card.add_time)
            .bind(card.expired_time)
            .bind(card.claimed_user_id)
            .bind(card.claimed_status)
            .bind(card.order_time)
            .bind(card.gift_cards_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 单字段更新：可用状态（Java `giftCards.setIsAvailable(...)`）。
    pub async fn update_available(&self, gift_cards_id: i32, is_available: i32) -> Result<u64> {
        let res = sqlx::query("UPDATE gift_cards SET is_available = $1 WHERE gift_cards_id = $2")
            .bind(is_available)
            .bind(gift_cards_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 按主键删除（Java `removeById`）。
    pub async fn delete(&self, gift_cards_id: i32) -> Result<u64> {
        let res = sqlx::query("DELETE FROM gift_cards WHERE gift_cards_id = $1")
            .bind(gift_cards_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 查询给定卡号中已存在的（Java `lambdaQuery().in(cardNumber, cardNos)`）。
    pub async fn find_existing_card_numbers(
        &self,
        card_numbers: &[String],
    ) -> Result<Vec<String>> {
        if card_numbers.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; card_numbers.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT card_number FROM gift_cards WHERE card_number IN ({placeholders})"
        ));
        let mut q = sqlx::query_as::<_, (String,)>(&sql);
        for no in card_numbers {
            q = q.bind(no);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().map(|(n,)| n).collect())
    }

    /// 批量插入礼品卡（Java `saveBatch(cards, 500)`）。
    pub async fn insert_many(&self, cards: &[GiftCards]) -> Result<u64> {
        let mut affected = 0u64;
        for card in cards {
            let res = sqlx::query(
                "INSERT INTO gift_cards (card_groups_id, card_number, card_password, gift_card_sn, is_available, status, used_time, user_id, add_time, expired_time, claimed_user_id, claimed_status, order_time) VALUES ($1, $2, $3, $4, $5, $6, 0, 0, $7, $8, 0, 0, 0)",
            )
            .bind(card.card_groups_id)
            .bind(&card.card_number)
            .bind(&card.card_password)
            .bind(&card.gift_card_sn)
            .bind(card.is_available)
            .bind(card.status)
            .bind(card.add_time)
            .bind(card.expired_time)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
            affected += res.rows_affected();
        }
        Ok(affected)
    }

    /// 插入充值单（Java `userRechargeOrderService.rechargeOperation` 的落库部分）。
    pub async fn insert_recharge_order(
        &self,
        user_id: i32,
        amount: Decimal,
        postscript: &str,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO user_recharge_order (user_id, amount, discount_money, add_time, paid_time, postscript, status) VALUES ($1, $2, 0, $3, 0, $4, 1) RETURNING order_id",
        )
        .bind(user_id)
        .bind(amount)
        .bind(now_seconds_i32())
        .bind(postscript)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }
}

/// 拼装卡列表 WHERE（Java：`.like(keyword != null, cardNumber, kw).eq(cardGroupsId)`）。
///
/// 注意 Java 判的是 `keyword != null`（**空串也会 LIKE '%%'**，与 `isNotEmpty`
/// 的其它域不同），此处逐字保持。
#[must_use]
pub fn build_where(q: &GiftCardsListQuery, card_groups_id: i32) -> (String, Vec<Bind>) {
    let mut conds = vec!["card_groups_id = ?".to_string()];
    let mut binds = vec![Bind::I32(card_groups_id)];
    if let Some(keyword) = q.keyword.as_deref() {
        conds.push("card_number LIKE ?".to_string());
        binds.push(Bind::Str(format!("%{keyword}%")));
    }
    (format!(" WHERE {}", conds.join(" AND ")), binds)
}

fn now_seconds_i32() -> i32 {
    i32::try_from(nextrade_common::utils::now_seconds()).unwrap_or(0)
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(json: serde_json::Value) -> GiftCardsListQuery {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn where_always_filters_group_and_optional_keyword() {
        let (sql, binds) = build_where(&q(serde_json::json!({})), 5);
        assert_eq!(sql, " WHERE card_groups_id = $1");
        assert!(matches!(binds.as_slice(), [Bind::I32(5)]));

        // Java `keyword != null`：空串也参与 LIKE
        let (sql, binds) = build_where(&q(serde_json::json!({"keyword": ""})), 5);
        assert_eq!(sql, " WHERE card_groups_id = $1 AND card_number LIKE $2");
        assert!(matches!(&binds[1], Bind::Str(s) if s == "%%"));

        let (sql, binds) = build_where(&q(serde_json::json!({"keyword": "GC20"})), 9);
        assert_eq!(sql, " WHERE card_groups_id = $1 AND card_number LIKE $2");
        assert!(matches!(&binds[1], Bind::Str(s) if s == "%GC20%"));
    }

    #[test]
    fn select_columns_match_entity_order() {
        assert_eq!(GIFT_CARDS_COLUMNS.len(), 14);
        assert_eq!(GIFT_CARDS_COLUMNS[0], "gift_cards_id");
        assert_eq!(GIFT_CARD_GROUPS_COLUMNS.len(), 15);
        assert_eq!(GIFT_CARD_GROUPS_COLUMNS[0], "card_groups_id");
        // INSERT 充值单列清单与表结构一致
        let src = include_str!("gift_card.rs");
        assert!(src.contains("INSERT INTO user_recharge_order (user_id, amount, discount_money, add_time, paid_time, postscript, status)"));
        assert!(src.contains("WHERE gift_card_id = $1 LIMIT 1"));
    }

    #[test]
    fn update_sql_touches_all_mutable_columns() {
        let src = include_str!("gift_card.rs");
        for column in [
            "card_groups_id = ?",
            "card_number = ?",
            "card_password = ?",
            "gift_card_sn = ?",
            "is_available = ?",
            "status = ?",
            "used_time = ?",
            "user_id = ?",
            "add_time = ?",
            "expired_time = ?",
            "claimed_user_id = ?",
            "claimed_status = ?",
            "order_time = ?",
        ] {
            assert!(src.contains(column), "UPDATE 缺少列 {column}");
        }
    }
}
