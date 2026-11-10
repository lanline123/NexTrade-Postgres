//! 活动赠品域仓储（marketing::product_gift）。
//! 表 `product_gift`；只读 `product` / `product_sku` / `gift_card_groups` 校验与装配。

use std::collections::HashMap;

use rust_decimal::Decimal;
use sqlx::{FromRow,Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::promotion::ProductGift;
use nextrade_core::models::marketing::product_gift::{
    limit_offset, ProductGiftListQuery,
};
use nextrade_core::{Error, Result};

const SELECT: &str = "SELECT gift_id, gift_name, gift_stock, product_id, sku_id, shop_id FROM product_gift";

/// 商品校验行。
#[derive(Debug, Clone, Default, FromRow)]
pub struct GiftProductRow {
    pub product_id: i32,
    pub product_name: String,
    pub product_sn: String,
    pub product_stock: i32,
    pub product_price: Decimal,
    pub market_price: Decimal,
    pub pic_thumb: String,
    pub pic_url: String,
    pub gift_card_groups_id: Option<i32>,
}

/// SKU 校验行。
#[derive(Debug, Clone, Default, FromRow)]
pub struct GiftSkuRow {
    pub sku_id: i32,
    pub product_id: i32,
    pub sku_price: Option<Decimal>,
    pub sku_stock: Option<i32>,
    pub is_enabled: i32,
}

/// 活动赠品域仓储。
#[derive(Clone)]
pub struct ProductGiftRepository {
    pool: PgPool,
}

impl ProductGiftRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 分页（Java `list`：keyword 赠品名 like + `shop_id = 当前店铺`）。
    pub async fn paginate(
        &self,
        q: &ProductGiftListQuery,
        shop_id: i32,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<ProductGift>)> {
        let keyword = q.effective_keyword();
        let mut cq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM product_gift WHERE shop_id = ");
        cq.push_bind(shop_id);
        if let Some(keyword) = keyword {
            cq.push(" AND gift_name ILIKE ");
            cq.push_bind(format!("%{keyword}%"));
        }
        let total: (i64,) = cq.build_query_as().fetch_one(&self.pool).await.map_err(map_err)?;
        let mut rq: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!("{SELECT} WHERE shop_id = "));
        rq.push_bind(shop_id);
        if let Some(keyword) = keyword {
            rq.push(" AND gift_name ILIKE ");
            rq.push_bind(format!("%{keyword}%"));
        }
        rq.push(q.order_clause());
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<ProductGift> = rq.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total.0, rows))
    }

    /// 按主键 + 店铺查询（Java `detail`）。
    pub async fn find_by_id_and_shop(&self, id: i32, shop_id: i32) -> Result<Option<ProductGift>> {
        sqlx::query_as::<_, ProductGift>(&format!(
            "{SELECT} WHERE gift_id = $1 AND shop_id = $2 LIMIT 1"
        ))
        .bind(id)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按主键查询（更新合并用）。
    /// 下单事务内扣减赠品库存（Java `ProductGiftService#decStock`）。
    pub async fn dec_gift_stock_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        gift_id: i32,
        quantity: i32,
    ) -> Result<bool> {
        let affected = sqlx::query(
            "UPDATE product_gift SET gift_stock = gift_stock - $1 WHERE gift_id = $2 AND gift_stock >= $3",
        )
        .bind(quantity)
        .bind(gift_id)
        .bind(quantity)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected > 0)
    }

    pub async fn find_by_id(&self, id: i32) -> Result<Option<ProductGift>> {
        sqlx::query_as::<_, ProductGift>(&format!("{SELECT} WHERE gift_id = $1 LIMIT 1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 同商品重复校验（Java `getCount(productId, excludeId)`，不区分店铺）。
    pub async fn count_by_product(&self, product_id: i32, exclude_id: i32) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM product_gift WHERE product_id = $1 AND gift_id <> $2")
                .bind(product_id)
                .bind(exclude_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0)
    }

    pub async fn find_product(&self, product_id: i32) -> Result<Option<GiftProductRow>> {
        sqlx::query_as::<_, GiftProductRow>(
            "SELECT product_id, product_name, product_sn, product_stock, product_price, market_price, pic_thumb, pic_url, gift_card_groups_id FROM product WHERE product_id = $1 LIMIT 1",
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn find_sku(&self, sku_id: i32) -> Result<Option<GiftSkuRow>> {
        sqlx::query_as::<_, GiftSkuRow>(
            "SELECT sku_id, product_id, sku_price, sku_stock, is_enabled FROM product_sku WHERE sku_id = $1 LIMIT 1",
        )
        .bind(sku_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 礼品卡分组卡类型（`card_type=1` ⇒ 电子卡）。
    pub async fn find_gift_card_type(&self, group_id: i32) -> Result<Option<i32>> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT card_type FROM gift_card_groups WHERE card_groups_id = $1 LIMIT 1")
                .bind(group_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|(v,)| v))
    }

    pub async fn find_products(&self, ids: &[i32]) -> Result<HashMap<i32, GiftProductRow>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new("SELECT product_id, product_name, product_sn, product_stock, product_price, market_price, pic_thumb, pic_url, gift_card_groups_id FROM product WHERE product_id IN (");
        Self::push_int_list(&mut qb, ids);
        qb.push(")");
        let rows: Vec<GiftProductRow> = qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().map(|r| (r.product_id, r)).collect())
    }

    pub async fn find_skus(&self, ids: &[i32]) -> Result<HashMap<i32, GiftSkuRow>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new("SELECT sku_id, product_id, sku_price, sku_stock, is_enabled FROM product_sku WHERE sku_id IN (");
        Self::push_int_list(&mut qb, ids);
        qb.push(")");
        let rows: Vec<GiftSkuRow> = qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().map(|r| (r.sku_id, r)).collect())
    }

    pub async fn insert(&self, entity: &ProductGift) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO product_gift (gift_name, gift_stock, product_id, sku_id, shop_id) VALUES ($1, $2, $3, $4, $5) RETURNING gift_id",
        )
        .bind(&entity.gift_name)
        .bind(entity.gift_stock)
        .bind(entity.product_id)
        .bind(entity.sku_id)
        .bind(entity.shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    pub async fn update(&self, entity: &ProductGift) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE product_gift SET gift_name = $1, gift_stock = $2, product_id = $3, sku_id = $4, shop_id = $5 WHERE gift_id = $6",
        )
        .bind(&entity.gift_name)
        .bind(entity.gift_stock)
        .bind(entity.product_id)
        .bind(entity.sku_id)
        .bind(entity.shop_id)
        .bind(entity.gift_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    pub async fn delete(&self, id: i32) -> Result<bool> {
        let res = sqlx::query("DELETE FROM product_gift WHERE gift_id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn delete_batch(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new("DELETE FROM product_gift WHERE gift_id IN (");
        Self::push_int_list(&mut qb, ids);
        qb.push(")");
        let res = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    #[must_use]
    pub fn page_window(page: Option<i64>, size: Option<i64>) -> (i64, i64) {
        limit_offset(page, size)
    }

    fn push_int_list(qb: &mut QueryBuilder<'_, Postgres>, ids: &[i32]) {
        {
            let mut sep = qb.separated(", ");
            for id in ids {
                sep.push_bind(*id);
            }
        }
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::marketing::product_gift::COLUMNS;

    #[test]
    fn select_matches_columns() {
        let selected = SELECT.trim_start_matches("SELECT ").split(" FROM ").next().unwrap();
        assert_eq!(selected, COLUMNS.join(", "));
    }

    #[test]
    fn static_sql_guards() {
        let src = include_str!("product_gift.rs");
        assert!(src.contains("FROM product_gift WHERE shop_id ="));
        assert!(src.contains("FROM gift_card_groups WHERE id = $1"));
        assert!(src.contains("INSERT INTO product_gift"));
    }
}
