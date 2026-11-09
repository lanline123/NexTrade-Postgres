//! 商品促销仓储（`marketing::product_promotion`）。
//!
//! 表：`product_promotion`（16 列，主键 `promotion_id` 自增）、
//! `promotion`（活动表，`relation_id` + `type` 关联到 product_promotion）、
//! `user_rank`（配置页会员等级）、`product_gift`（赠品回填）。
//!
//! 全部 SQL 为运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`（禁止 `query!` 宏）。

use rust_decimal::Decimal;
use sqlx::PgPool;
use sqlx::{FromRow,Postgres, QueryBuilder};

use nextrade_core::entities::{ProductGift, ProductPromotion, Promotion, UserRank};
use nextrade_core::models::marketing::product_promotion::{
    parse_datetime, sort_column, ConflictPageQuery, ProductPromotionListQuery, RANGE_PRODUCT,
    TIME_STATUS_FORTHCOMING, TIME_STATUS_OFF, TIME_STATUS_ON,
};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// `promotion` 表中由商品促销维护的类型（满减/满折/满赠）。
pub const GAIN_TYPES: [i32; 3] = [3, 4, 5];

#[derive(Clone)]
pub struct ProductPromotionRepository {
    pool: PgPool,
}

impl ProductPromotionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ————————————————————— 商品促销 CRUD —————————————————————

    /// 分页（Java `list`：关键词 LIKE + 类型 IN + 店铺租户 + 时间状态 + 排序白名单）。
    ///
    /// `shop_id`：`None`（平台账号）不过滤；`Some(id)` 强制 `WHERE shop_id = ?`。
    pub async fn paginate(
        &self,
        q: &ProductPromotionListQuery,
        page: &PageQuery,
        now: i64,
        shop_id: Option<i32>,
    ) -> Result<(u64, Vec<ProductPromotion>)> {
        let mut wh = String::from(" WHERE 1 = 1");
        let mut binds: Vec<String> = Vec::new();
        if let Some(kw) = q.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            wh.push_str(" AND promotion_name LIKE ?");
            binds.push(format!("%{kw}%"));
        }
        let types = q.promotion_types();
        if !types.is_empty() {
            let placeholders = vec!["?"; types.len()].join(", ");
            wh.push_str(&format!(" AND promotion_type IN ({placeholders})"));
            binds.extend(types.iter().map(|t| t.to_string()));
        }
        // 多租户：店铺账号强制过滤（见 `marketing::tenant`）
        if let Some(shop_id) = shop_id {
            wh.push_str(" AND shop_id = ?");
            binds.push(shop_id.to_string());
        }
        // 时间状态检索（Java `PromotionTimeStatusEnum`）
        if let Some(is_going) = q.is_going.filter(|v| *v > 0) {
            match is_going {
                TIME_STATUS_ON => {
                    wh.push_str(" AND start_time <= ? AND end_time >= ?");
                    binds.push(now.to_string());
                    binds.push(now.to_string());
                }
                TIME_STATUS_FORTHCOMING => {
                    wh.push_str(" AND start_time > ?");
                    binds.push(now.to_string());
                }
                TIME_STATUS_OFF => {
                    wh.push_str(" AND end_time < ?");
                    binds.push(now.to_string());
                }
                _ => {}
            }
        }
        let total_sql = format!("SELECT COUNT(*) FROM product_promotion{wh}");
        let total_sql = crate::db::to_pg_placeholders(&total_sql);
        let mut tq = sqlx::query_as::<_, (i64,)>(&total_sql);
        for b in &binds {
            tq = tq.bind(b.as_str());
        }
        let total = tq.fetch_one(&self.pool).await.map_err(map_err)?;
        let (limit, offset) = page.limit_offset();
        let dir = if q.ascending() { "ASC" } else { "DESC" };
        let list_sql = format!(
            "SELECT * FROM product_promotion{wh} ORDER BY {} {dir} LIMIT $1 OFFSET $2",
            sort_column(q.sort_field.as_deref())
        );
        let list_sql = crate::db::to_pg_placeholders(&list_sql);
        let mut lq = sqlx::query_as::<_, ProductPromotion>(&list_sql);
        for b in &binds {
            lq = lq.bind(b.as_str());
        }
        let rows = lq
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    pub async fn find_by_id(&self, id: i32) -> Result<Option<ProductPromotion>> {
        sqlx::query_as::<_, ProductPromotion>(
            "SELECT * FROM product_promotion WHERE promotion_id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 插入（主键自增，返回 id）。
    pub async fn insert(&self, e: &ProductPromotion) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO product_promotion (promotion_name, start_time, end_time, limit_user_rank,
                \"range\", range_data, min_order_amount, max_order_amount, promotion_type,
                promotion_type_data, is_available, sort_order, shop_id, rules_type, unit)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15) RETURNING promotion_id",
        )
        .bind(&e.promotion_name)
        .bind(e.start_time)
        .bind(e.end_time)
        .bind(&e.limit_user_rank)
        .bind(e.range)
        .bind(&e.range_data)
        .bind(e.min_order_amount)
        .bind(e.max_order_amount)
        .bind(e.promotion_type)
        .bind(&e.promotion_type_data)
        .bind(e.is_available)
        .bind(e.sort_order)
        .bind(e.shop_id)
        .bind(e.rules_type)
        .bind(e.unit)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 整行更新（Java `updateById`）。
    pub async fn update(&self, e: &ProductPromotion) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE product_promotion SET promotion_name = $1, start_time = $2, end_time = $3,
                limit_user_rank = $4, \"range\" = $5, range_data = $6, min_order_amount = $7,
                max_order_amount = $8, promotion_type = $9, promotion_type_data = $10,
                is_available = $11, sort_order = $12, shop_id = $13, rules_type = $14, unit = $15
             WHERE promotion_id = $16",
        )
        .bind(&e.promotion_name)
        .bind(e.start_time)
        .bind(e.end_time)
        .bind(&e.limit_user_rank)
        .bind(e.range)
        .bind(&e.range_data)
        .bind(e.min_order_amount)
        .bind(e.max_order_amount)
        .bind(e.promotion_type)
        .bind(&e.promotion_type_data)
        .bind(e.is_available)
        .bind(e.sort_order)
        .bind(e.shop_id)
        .bind(e.rules_type)
        .bind(e.unit)
        .bind(e.promotion_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 批量删除（Java `removeByIds`）。
    pub async fn delete_by_ids(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("DELETE FROM product_promotion WHERE promotion_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        Ok(qb.build().execute(&self.pool).await.map_err(map_err)?.rows_affected())
    }

    /// 单字段更新（白名单列，Java `updateField`）。
    pub async fn update_field(&self, id: i32, column: &str, val: &str) -> Result<u64> {
        let sql =
            format!("UPDATE product_promotion SET {column} = $1 WHERE promotion_id = $2");
        let sql = crate::db::to_pg_placeholders(&sql);
        let r = sqlx::query(&sql)
            .bind(val)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    // ————————————————————— promotion 表同步 —————————————————————

    /// 按 `relation_id` + 商品促销类型查 `promotion` 行（Java `savePromotion` 存在性判断）。
    /// 某店铺（含平台通用 shop_id=0）当前进行中的满减/满折活动。
    ///
    /// # Errors
    /// 数据库错误。
    pub async fn find_active_by_shop(
        &self,
        shop_id: i32,
        now: i32,
    ) -> Result<Vec<ProductPromotion>> {
        sqlx::query_as::<_, ProductPromotion>(
            "SELECT * FROM product_promotion
             WHERE shop_id IN (0, $1) AND is_available = 1
               AND start_time <= $2 AND end_time >= $3
             ORDER BY sort_order DESC, promotion_id ASC",
        )
        .bind(shop_id)
        .bind(now)
        .bind(now)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn find_promotion_row(&self, relation_id: i32) -> Result<Option<Promotion>> {
        sqlx::query_as::<_, Promotion>(
            "SELECT \"promotion_id\", \"promotion_name\", \"start_time\", \"end_time\", \"type\", \"shop_id\", \"relation_id\", \"range\", \"range_data\", \"sku_ids\", \"is_available\", \"is_delete\", \"member_range\", \"member_marks\", \"member_scope\"::text AS \"member_scope\" FROM promotion WHERE relation_id = $1 AND type IN (3, 4, 5) LIMIT 1",
        )
        .bind(relation_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 插入 `promotion` 行（Java `promotionService.save`）。
    pub async fn insert_promotion_row(&self, p: &Promotion) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO promotion (promotion_name, start_time, end_time, type, shop_id,
                relation_id, \"range\", range_data, is_available, member_range, member_marks, member_scope)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NULLIF($12, '')::jsonb) RETURNING promotion_id",
        )
        .bind(&p.promotion_name)
        .bind(p.start_time)
        .bind(p.end_time)
        .bind(p.r#type)
        .bind(p.shop_id)
        .bind(p.relation_id)
        .bind(p.range)
        .bind(&p.range_data)
        .bind(p.is_available)
        .bind(p.member_range)
        .bind(&p.member_marks)
        .bind(&p.member_scope)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 更新 `promotion` 行（Java `promotionService.update`：type/时间/名称/range/member 维度）。
    pub async fn update_promotion_row(&self, p: &Promotion) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE promotion SET type = $1, start_time = $2, end_time = $3, promotion_name = $4,
                is_available = 1, \"range\" = $5, range_data = $6, member_range = $7,
                member_marks = $8, member_scope = NULLIF($9, '')::jsonb
             WHERE relation_id = $10 AND type IN (3, 4, 5)",
        )
        .bind(p.r#type)
        .bind(p.start_time)
        .bind(p.end_time)
        .bind(&p.promotion_name)
        .bind(p.range)
        .bind(&p.range_data)
        .bind(p.member_range)
        .bind(&p.member_marks)
        .bind(&p.member_scope)
        .bind(p.relation_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 启停同步到 `promotion`（Java `updateField` 中 isAvailable 分支）。
    pub async fn update_promotion_available(&self, relation_id: i32, val: i32) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE promotion SET is_available = $1 WHERE relation_id = $2 AND type IN (3, 4, 5)",
        )
        .bind(val)
        .bind(relation_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 删除 `promotion` 行（Java `del` 中按 type + relation_id 删除）。
    pub async fn delete_promotion_row(&self, relation_id: i32, gain_type: i32) -> Result<u64> {
        let r = sqlx::query(
            "DELETE FROM promotion WHERE type = $1 AND relation_id = $2",
        )
        .bind(gain_type)
        .bind(relation_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    // ————————————————————— 冲突 —————————————————————

    /// 创建/更新时的冲突列表（Java `getConflictList`）：
    /// 时间区间重叠 + 店铺 + 类型 + 启用 + 排除自身 + 指定商品 `JSON_CONTAINS`。
    pub async fn find_conflicts(
        &self,
        start_ts: i64,
        end_ts: i64,
        range: i32,
        range_data: &[i32],
        promotion_type: i32,
        shop_id: i32,
        exclude_id: i32,
    ) -> Result<Vec<ProductPromotion>> {
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT * FROM product_promotion WHERE 1 = 1");
        if start_ts > 0 && end_ts > 0 {
            qb.push(" AND start_time <= ");
            qb.push_bind(end_ts);
            qb.push(" AND end_time >= ");
            qb.push_bind(start_ts);
        }
        qb.push(" AND shop_id = ");
        qb.push_bind(shop_id);
        qb.push(" AND promotion_type = ");
        qb.push_bind(promotion_type);
        qb.push(" AND is_available = 1 AND promotion_id <> ");
        qb.push_bind(exclude_id);
        if range == RANGE_PRODUCT && !range_data.is_empty() {
            qb.push(" AND (");
            for (i, product_id) in range_data.iter().enumerate() {
                if i > 0 {
                    qb.push(" OR ");
                }
                qb.push("range_data::jsonb @> ");
                qb.push_bind(product_id.to_string());
                qb.push("::text::jsonb");
            }
            qb.push(")");
        }
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 冲突活动分页（Java `conflictPage`：启用 + 店铺 + 时间重叠 + 类型 + 指定商品范围）。
    pub async fn conflict_paginate(
        &self,
        q: &ConflictPageQuery,
        page: &PageQuery,
    ) -> Result<(u64, Vec<ProductPromotion>)> {
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM product_promotion WHERE is_available = 1");
        self.push_conflict_filters(&mut qb, q);
        let total: (i64,) = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let (limit, offset) = page.limit_offset();
        let dir = if q
            .sort_order
            .as_deref()
            .is_some_and(|o| o.eq_ignore_ascii_case("asc"))
        {
            "ASC"
        } else {
            "DESC"
        };
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT * FROM product_promotion WHERE is_available = 1");
        self.push_conflict_filters(&mut qb, q);
        qb.push(format!(
            " ORDER BY {} {dir} LIMIT ",
            sort_column(q.sort_field.as_deref())
        ));
        qb.push_bind(limit);
        qb.push(" OFFSET ");
        qb.push_bind(offset);
        let rows = qb
            .build_query_as::<ProductPromotion>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    fn push_conflict_filters<'a>(
        &self,
        qb: &mut QueryBuilder<'a, Postgres>,
        q: &ConflictPageQuery,
    ) {
        qb.push(" AND shop_id = ");
        qb.push_bind(q.shop_id.unwrap_or(0));
        if let (Some(start), Some(end)) = (
            q.start_time.as_deref().map(parse_datetime).filter(|v| *v > 0),
            q.end_time.as_deref().map(parse_datetime).filter(|v| *v > 0),
        ) {
            qb.push(" AND start_time <= ");
            qb.push_bind(end);
            qb.push(" AND end_time >= ");
            qb.push_bind(start);
        }
        if let Some(t) = q.promotion_type {
            qb.push(" AND promotion_type = ");
            qb.push_bind(t);
        }
        if q.range.unwrap_or(-1) == RANGE_PRODUCT {
            for pid in q.range_product_ids() {
                qb.push(" AND range_data::jsonb @> ");
                qb.push_bind(pid.to_string());
                qb.push("::text::jsonb");
            }
        }
    }

    // ————————————————————— 配置 / 赠品 —————————————————————

    /// 会员等级列表（Java `userRankService.getUserRank()`）。
    pub async fn rank_list(&self) -> Result<Vec<UserRank>> {
        sqlx::query_as::<_, UserRank>("SELECT * FROM user_rank ORDER BY rank_id ASC")
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按 id 批量取赠品（Java `productGiftService.listByIds`）。
    pub async fn gifts_by_ids(&self, ids: &[i32]) -> Result<Vec<ProductGift>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT * FROM product_gift WHERE gift_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 店铺在售商品总数（Java `conflictPage` 中「不含商品」范围去重比较用）。
    pub async fn count_on_sale_products(&self, shop_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM product WHERE product_status = 1 AND is_delete = 0 AND shop_id = $1",
        )
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 指定商品 id 的在售商品集合（Java `conflictPage` 指定商品范围）。
    pub async fn on_sale_product_ids(&self, shop_id: i32, ids: &[i32]) -> Result<Vec<i32>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT product_id FROM product WHERE product_status = 1 AND is_delete = 0 AND shop_id = ",
        );
        qb.push_bind(shop_id);
        qb.push(" AND product_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        let rows: Vec<(i32,)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }
}

/// `product_promotion` 行转实体所需的空值填充（Java `dataJudge` 的默认值）。
#[must_use]
pub fn empty_entity() -> ProductPromotion {
    ProductPromotion {
        max_order_amount: Decimal::ZERO,
        limit_user_rank: String::new(),
        range_data: String::new(),
        promotion_type_data: String::new(),
        ..Default::default()
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("优惠活动不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_update_placeholder_counts() {
        let insert = "INSERT INTO product_promotion (promotion_name, start_time, end_time, limit_user_rank,
                \"range\", range_data, min_order_amount, max_order_amount, promotion_type,
                promotion_type_data, is_available, sort_order, shop_id, rules_type, unit)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)";
        assert_eq!(insert.matches('?').count(), 15);
        let update = "UPDATE product_promotion SET promotion_name = $1, start_time = $2, end_time = $3,
                limit_user_rank = $4, \"range\" = $5, range_data = $6, min_order_amount = $7,
                max_order_amount = $8, promotion_type = $9, promotion_type_data = $10,
                is_available = $11, sort_order = $12, shop_id = $13, rules_type = $14, unit = $15
             WHERE promotion_id = $16";
        assert_eq!(update.matches('?').count(), 16);
        assert_eq!(GAIN_TYPES, [3, 4, 5]);
        let _ = empty_entity();
    }
}
