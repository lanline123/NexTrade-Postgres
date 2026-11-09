//! 限时折扣域仓储：`time_discount` / `time_discount_item`。
//! 对应原 Java `TimeDiscountMapper` / `TimeDiscountItemMapper`。
//!
//! 列名严格对齐 `java/database/pure.sql`：
//! - `time_discount`：`discount_id` / `promotion_name` / `start_time` /
//!   `end_time` / `shop_id` / `add_time`
//! - `time_discount_item`：`item_id` / `discount_id` / `start_time` /
//!   `end_time` / `product_id` / `sku_ids` / `value` / `discount_type`

use rust_decimal::Decimal;
use sqlx::PgPool;
use nextrade_core::entities::{TimeDiscount, TimeDiscountItem};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct TimeDiscountRepository {
    pool: PgPool,
}

impl TimeDiscountRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层扩展查询）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 活动 —————————————————————

    /// 按 ID 查询活动。
    pub async fn find_by_id(&self, discount_id: i32) -> Result<Option<TimeDiscount>> {
        sqlx::query_as::<_, TimeDiscount>(
            "SELECT * FROM time_discount WHERE discount_id = $1 LIMIT 1",
        )
        .bind(discount_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 活动分页（关键词匹配活动名，可按店铺过滤）。
    pub async fn paginate(
        &self,
        keyword: Option<&str>,
        shop_id: Option<i32>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<TimeDiscount>)> {
        let like = keyword
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(|k| format!("%{k}%"));
        let (limit, offset) = page.limit_offset();

        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM time_discount
             WHERE ($1 IS NULL OR promotion_name LIKE $2)
               AND ($3 IS NULL OR shop_id = $4)",
        )
        .bind(&like)
        .bind(&like)
        .bind(shop_id)
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        let rows = sqlx::query_as::<_, TimeDiscount>(
            "SELECT * FROM time_discount
             WHERE ($1 IS NULL OR promotion_name LIKE $2)
               AND ($3 IS NULL OR shop_id = $4)
             ORDER BY discount_id DESC
             LIMIT $5 OFFSET $6",
        )
        .bind(&like)
        .bind(&like)
        .bind(shop_id)
        .bind(shop_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 新增活动，返回 `discount_id`。
    pub async fn insert(&self, d: &TimeDiscount) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO time_discount
             (promotion_name, start_time, end_time, shop_id, add_time)
             VALUES ($1, $2, $3, $4, $5) RETURNING discount_id",
        )
        .bind(&d.promotion_name)
        .bind(d.start_time)
        .bind(d.end_time)
        .bind(d.shop_id)
        .bind(d.add_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 更新活动。
    pub async fn update(&self, d: &TimeDiscount) -> Result<()> {
        let affected = sqlx::query(
            "UPDATE time_discount SET
               promotion_name = $1, start_time = $2, end_time = $3
             WHERE discount_id = $4",
        )
        .bind(&d.promotion_name)
        .bind(d.start_time)
        .bind(d.end_time)
        .bind(d.discount_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        if affected == 0 {
            return Err(Error::NotFound("限时折扣活动不存在".into()));
        }
        Ok(())
    }

    /// 删除活动及其明细（事务）。
    pub async fn delete(&self, discount_id: i32) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        sqlx::query("DELETE FROM time_discount_item WHERE discount_id = $1")
            .bind(discount_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        sqlx::query("DELETE FROM time_discount WHERE discount_id = $1")
            .bind(discount_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        tx.commit().await.map_err(map_err)?;
        Ok(())
    }

    // ————————————————————— 活动明细 —————————————————————

    /// 某活动下的全部明细。
    pub async fn find_items(&self, discount_id: i32) -> Result<Vec<TimeDiscountItem>> {
        sqlx::query_as::<_, TimeDiscountItem>(
            "SELECT * FROM time_discount_item WHERE discount_id = $1 ORDER BY item_id ASC",
        )
        .bind(discount_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 新增明细，返回 `item_id`。
    pub async fn insert_item(&self, item: &TimeDiscountItem) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO time_discount_item
             (discount_id, start_time, end_time, product_id, sku_ids,
              value, discount_type)
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING item_id",
        )
        .bind(item.discount_id)
        .bind(item.start_time)
        .bind(item.end_time)
        .bind(item.product_id)
        .bind(&item.sku_ids)
        .bind(item.value)
        .bind(item.discount_type)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 清空某活动的全部明细（重建场景）。
    pub async fn delete_items(&self, discount_id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM time_discount_item WHERE discount_id = $1")
            .bind(discount_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 查询商品**当前进行中**的折扣明细。
    ///
    /// 对应 Java `TimeDiscountServiceImpl#canUserJoinTimeDiscount` 中
    /// `le(start_time, now).ge(end_time, now)` 的时间窗条件。
    pub async fn find_active_items(
        &self,
        product_id: i32,
        now: i32,
    ) -> Result<Vec<TimeDiscountItem>> {
        sqlx::query_as::<_, TimeDiscountItem>(
            "SELECT * FROM time_discount_item
             WHERE product_id = $1
               AND start_time <= $2
               AND end_time >= $3
             ORDER BY item_id DESC",
        )
        .bind(product_id)
        .bind(now)
        .bind(now)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 查询某活动下适用于指定 SKU 的明细（结算页取价）。
    ///
    /// `sku_ids` 为 JSON 数组字符串（空/`null` 表示作用于该商品全部规格）。
    /// SQL 层用 `LIKE` 粗筛，精确匹配在服务层用 JSON 解析完成 ——
    /// 避免 MySQL JSON 函数对非法值的容错差异。
    pub async fn find_items_for_product(
        &self,
        discount_id: i32,
        product_id: i32,
    ) -> Result<Vec<TimeDiscountItem>> {
        sqlx::query_as::<_, TimeDiscountItem>(
            "SELECT * FROM time_discount_item
             WHERE discount_id = $1 AND product_id = $2",
        )
        .bind(discount_id)
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 校验商品在给定时间窗内是否已参与**其他限时折扣**。
    ///
    /// 对应 Java `checkPromotionByProduct` 的第一段：
    /// 时间窗重叠判定为 `existing.start <= new.end AND existing.end >= new.start`。
    pub async fn count_conflicting_discount(
        &self,
        product_ids: &[i32],
        start_time: i32,
        end_time: i32,
        exclude_discount_id: Option<i32>,
    ) -> Result<i64> {
        if product_ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; product_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT COUNT(*) FROM time_discount_item
             WHERE product_id IN ({placeholders})
               AND start_time <= ?
               AND end_time >= ?
               AND (? IS NULL OR discount_id <> ?)"
        ));
        let mut q = sqlx::query_as::<_, (i64,)>(&sql);
        for pid in product_ids {
            q = q.bind(pid);
        }
        let row = q
            .bind(end_time)
            .bind(start_time)
            .bind(exclude_discount_id)
            .bind(exclude_discount_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0)
    }

    /// 校验商品在给定时间窗内是否已参与**秒杀活动**。
    ///
    /// 对应 Java `checkPromotionByProduct` 的第二段（跨域冲突校验）。
    pub async fn count_conflicting_seckill(
        &self,
        product_ids: &[i32],
        start_time: i32,
        end_time: i32,
    ) -> Result<i64> {
        if product_ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; product_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT COUNT(*) FROM seckill_item
             WHERE product_id IN ({placeholders})
               AND seckill_start_time <= ?
               AND seckill_end_time >= ?"
        ));
        let mut q = sqlx::query_as::<_, (i64,)>(&sql);
        for pid in product_ids {
            q = q.bind(pid);
        }
        let row = q
            .bind(end_time)
            .bind(start_time)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0)
    }

    /// 当前进行中的折扣商品明细（前台折扣专区）。
    pub async fn find_active_page(
        &self,
        now: i32,
        page: &PageQuery,
    ) -> Result<(u64, Vec<TimeDiscountItem>)> {
        let (limit, offset) = page.limit_offset();

        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM time_discount_item
             WHERE start_time <= $1 AND end_time >= $2",
        )
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        let rows = sqlx::query_as::<_, TimeDiscountItem>(
            "SELECT * FROM time_discount_item
             WHERE start_time <= $1 AND end_time >= $2
             ORDER BY item_id DESC
             LIMIT $3 OFFSET $4",
        )
        .bind(now)
        .bind(now)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("限时折扣活动不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

/// 折扣计算结果（供结算页使用）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscountHit {
    pub product_id: i32,
    pub sku_id: i32,
    pub original_price: Decimal,
    pub discount_price: Decimal,
    /// 单件优惠金额（不为负）
    pub saved: Decimal,
}