//! 商品收藏域仓储（member::collect_product）。表 `collect_product` + `product` 摘要。

use sqlx::PgPool;
use nextrade_core::models::member::collect_product::CollectProductVO;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct CollectProductRepository {
    pool: PgPool,
}

impl CollectProductRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 收藏分页（当前用户，含商品名/主图）。
    pub async fn paginate(
        &self,
        user_id: i32,
        page: u64,
        size: u64,
    ) -> Result<(u64, Vec<CollectProductVO>)> {
        let total: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM collect_product WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        let rows: Vec<(i32, i32, i32, i32, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT c.collect_id, c.product_id, c.shop_id, c.add_time, p.product_name, p.pic_thumb FROM collect_product c LEFT JOIN product p ON p.product_id = c.product_id WHERE c.user_id = $1 ORDER BY c.collect_id DESC LIMIT $2 OFFSET $3",
        )
        .bind(user_id)
        .bind(i64::try_from(size).unwrap_or(15))
        .bind(i64::try_from(page.saturating_sub(1) * size).unwrap_or(0))
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        let list = rows
            .into_iter()
            .map(|(collect_id, product_id, shop_id, add_time, product_name, pic_thumb)| {
                CollectProductVO {
                    collect_id,
                    product_id,
                    shop_id,
                    add_time,
                    product_name,
                    pic_thumb,
                }
            })
            .collect();
        Ok((u64::try_from(total.0).unwrap_or(0), list))
    }

    /// 商品是否存在（`product` 软删过滤）。
    pub async fn product_exists(&self, product_id: i32) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT product_id FROM product WHERE product_id = $1 AND is_delete = 0 LIMIT 1",
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.is_some())
    }

    /// 是否已收藏。
    pub async fn exists(&self, user_id: i32, product_id: i32) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT collect_id FROM collect_product WHERE user_id = $1 AND product_id = $2 LIMIT 1",
        )
        .bind(user_id)
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.is_some())
    }

    /// 新增收藏（返回 collect_id）。
    pub async fn insert(&self, user_id: i32, product_id: i32, shop_id: i32, now: i32) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO collect_product (user_id, product_id, shop_id, add_time) VALUES ($1, $2, $3, $4) RETURNING collect_id",
        )
        .bind(user_id)
        .bind(product_id)
        .bind(shop_id)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 取消收藏（返回是否删除）。
    pub async fn delete(&self, user_id: i32, product_id: i32) -> Result<bool> {
        let r = sqlx::query("DELETE FROM collect_product WHERE user_id = $1 AND product_id = $2")
            .bind(user_id)
            .bind(product_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 商品所属店铺（收藏落库的 shop_id）。
    pub async fn product_shop_id(&self, product_id: i32) -> Result<Option<i32>> {
        let row: Option<(Option<i32>,)> =
            sqlx::query_as("SELECT shop_id FROM product WHERE product_id = $1 LIMIT 1")
                .bind(product_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|(s,)| s.unwrap_or(0)))
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}
