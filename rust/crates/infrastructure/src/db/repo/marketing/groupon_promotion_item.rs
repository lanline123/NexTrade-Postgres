//! 拼团活动商品仓储（`marketing::groupon_promotion_item`）。
//!
//! 表：`groupon_promotion_item`（11 列）/ `groupon_promotion_detail`（9 列）/
//! `groupon_promotion`（活动时间校验）/ `promotion`（type=7 拼团活动 range 同步）/
//! `seckill_item` + `time_discount_item`（商品冲突校验）/ `product` + `product_sku`
//! （商品/SKU 展示）/ `product_group`（商品分组过滤）。
//!
//! 全部 SQL 为运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`（禁止 `query!` 宏）。

use sqlx::PgPool;
use sqlx::{FromRow,Postgres, QueryBuilder};

use nextrade_core::entities::{
    GrouponPromotion, GrouponPromotionDetail, GrouponPromotionItem, Product, ProductSku,
};
use nextrade_core::models::marketing::groupon_promotion_item::{ItemPageQuery, ProductPageQuery};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// 商品列表/可选商品列表 JOIN 行。
#[derive(Debug, Clone, Default, FromRow)]
pub struct ItemPageRow {
    pub groupon_promotion_item_id: i64,
    pub groupon_promotion_id: i64,
    pub product_id: i64,
    pub group_type: i32,
    pub group_num: String,
    pub order_limit_num: i32,
    pub is_auto_group: i32,
    pub min_group_num: i32,
    pub sort: i32,
    pub add_time: i64,
    pub min_group_price: Option<rust_decimal::Decimal>,
    pub product_name: Option<String>,
    pub product_sn: Option<String>,
    pub pic_thumb: Option<String>,
    pub pic_large: Option<String>,
    pub pic_url: Option<String>,
    pub product_price: Option<rust_decimal::Decimal>,
    pub product_brief: Option<String>,
}

#[derive(Clone)]
pub struct GrouponPromotionItemRepository {
    pool: PgPool,
}

impl GrouponPromotionItemRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ————————————————————— 活动 / 商品 —————————————————————

    pub async fn find_promotion(&self, id: i64) -> Result<Option<GrouponPromotion>> {
        sqlx::query_as::<_, GrouponPromotion>(
            "SELECT * FROM groupon_promotion WHERE groupon_promotion_id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn find_item(&self, id: i64) -> Result<Option<GrouponPromotionItem>> {
        sqlx::query_as::<_, GrouponPromotionItem>(
            "SELECT * FROM groupon_promotion_item WHERE groupon_promotion_item_id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn insert_item(&self, item: &GrouponPromotionItem) -> Result<i64> {
        let r: i64 = sqlx::query_scalar(
            "INSERT INTO groupon_promotion_item (groupon_promotion_id, product_id, group_type,
                group_num, order_limit_num, is_auto_group, min_group_num, sort, add_time, min_group_price)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING groupon_promotion_item_id",
        )
        .bind(item.groupon_promotion_id)
        .bind(item.product_id)
        .bind(item.group_type)
        .bind(&item.group_num)
        .bind(item.order_limit_num)
        .bind(item.is_auto_group)
        .bind(item.min_group_num)
        .bind(item.sort)
        .bind(item.add_time)
        .bind(item.min_group_price)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    pub async fn update_item(&self, item: &GrouponPromotionItem) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE groupon_promotion_item SET groupon_promotion_id = $1, product_id = $2,
                group_type = $3, group_num = $4, order_limit_num = $5, is_auto_group = $6,
                min_group_num = $7, sort = $8, min_group_price = $9
             WHERE groupon_promotion_item_id = $10",
        )
        .bind(item.groupon_promotion_id)
        .bind(item.product_id)
        .bind(item.group_type)
        .bind(&item.group_num)
        .bind(item.order_limit_num)
        .bind(item.is_auto_group)
        .bind(item.min_group_num)
        .bind(item.sort)
        .bind(item.min_group_price)
        .bind(item.groupon_promotion_item_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    pub async fn delete_items(&self, ids: &[i64]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "DELETE FROM groupon_promotion_item WHERE groupon_promotion_item_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        Ok(qb.build().execute(&self.pool).await.map_err(map_err)?.rows_affected())
    }

    // ————————————————————— 拼团明细（SKU 拼团价） —————————————————————

    pub async fn find_details_by_item(&self, item_id: i64) -> Result<Vec<GrouponPromotionDetail>> {
        sqlx::query_as::<_, GrouponPromotionDetail>(
            "SELECT * FROM groupon_promotion_detail WHERE groupon_promotion_item_id = $1 ORDER BY groupon_detail_id ASC",
        )
        .bind(item_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn find_details_by_items(
        &self,
        item_ids: &[i64],
    ) -> Result<Vec<GrouponPromotionDetail>> {
        if item_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT * FROM groupon_promotion_detail WHERE groupon_promotion_item_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in item_ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(") ORDER BY groupon_detail_id ASC");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    pub async fn find_details_by_promotion(
        &self,
        promotion_id: i64,
    ) -> Result<Vec<GrouponPromotionDetail>> {
        sqlx::query_as::<_, GrouponPromotionDetail>(
            "SELECT * FROM groupon_promotion_detail WHERE groupon_promotion_id = $1 ORDER BY groupon_detail_id ASC",
        )
        .bind(promotion_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn delete_details_by_items(&self, item_ids: &[i64]) -> Result<u64> {
        if item_ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "DELETE FROM groupon_promotion_detail WHERE groupon_promotion_item_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in item_ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        Ok(qb.build().execute(&self.pool).await.map_err(map_err)?.rows_affected())
    }

    pub async fn insert_detail(&self, d: &GrouponPromotionDetail) -> Result<i64> {
        let r: i64 = sqlx::query_scalar(
            "INSERT INTO groupon_promotion_detail (groupon_promotion_id, groupon_promotion_item_id,
                product_id, sku_id, group_price, limit_num, is_join, add_time)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING groupon_detail_id",
        )
        .bind(d.groupon_promotion_id)
        .bind(d.groupon_promotion_item_id)
        .bind(d.product_id)
        .bind(d.sku_id)
        .bind(&d.group_price)
        .bind(d.limit_num)
        .bind(d.is_join)
        .bind(d.add_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    // ————————————————————— 冲突校验 —————————————————————

    /// 时间段内已参与其它拼团活动计数（Java Mapper `getInGroupOnProductCount`）。
    pub async fn count_groupon_conflict(
        &self,
        product_id: i64,
        exclude_promotion_id: i64,
        current_time: i64,
        end_time: i64,
    ) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(1) FROM groupon_promotion_item gpi
             JOIN groupon_promotion gp ON gpi.groupon_promotion_id = gp.groupon_promotion_id
             WHERE gpi.product_id = $1 AND gp.groupon_promotion_id <> $2
               AND (gp.end_time > $3 AND gp.start_time < $4)",
        )
        .bind(product_id)
        .bind(exclude_promotion_id)
        .bind(current_time)
        .bind(end_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 秒杀冲突（Java `checkPromotionByProduct` 第一段）。
    pub async fn count_seckill_conflict(
        &self,
        product_id: i64,
        start_time: i64,
        end_time: i64,
    ) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(1) FROM seckill_item WHERE product_id = $1
               AND seckill_start_time <= $2 AND seckill_end_time >= $3",
        )
        .bind(product_id)
        .bind(start_time)
        .bind(end_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 限时折扣冲突（Java `checkPromotionByProduct` 第二段）。
    pub async fn count_time_discount_conflict(
        &self,
        product_id: i64,
        start_time: i64,
        end_time: i64,
    ) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(1) FROM time_discount_item WHERE product_id = $1
               AND start_time <= $2 AND end_time >= $3",
        )
        .bind(product_id)
        .bind(start_time)
        .bind(end_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// SKU 是否启用且属于该商品（Java `checkSkuEnabled`）。
    pub async fn sku_enabled(&self, sku_id: i64, product_id: i64) -> Result<bool> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(1) FROM product_sku WHERE sku_id = $1 AND product_id = $2 AND is_enabled = 1",
        )
        .bind(sku_id)
        .bind(product_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0 > 0)
    }

    // ————————————————————— 商品 / SKU —————————————————————

    pub async fn find_product(&self, id: i64) -> Result<Option<Product>> {
        sqlx::query_as::<_, Product>("SELECT * FROM product WHERE product_id = $1 LIMIT 1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    pub async fn find_enabled_skus(&self, product_ids: &[i64]) -> Result<Vec<ProductSku>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT * FROM product_sku WHERE is_enabled = 1 AND product_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in product_ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(") ORDER BY sku_id ASC");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 商品分组内商品 id 列表（Java `ProductGroupService`：`product_ids` CSV 解析）。
    pub async fn product_group_ids(&self, group_id: i32) -> Result<Option<Vec<i32>>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT product_ids FROM product_group WHERE product_group_id = $1 LIMIT 1")
                .bind(group_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|(raw,)| {
            raw.as_deref()
                .map(|s| {
                    s.split(',')
                        .filter_map(|v| v.trim().parse::<i32>().ok())
                        .collect()
                })
                .unwrap_or_default()
        }))
    }

    // ————————————————————— 分页 —————————————————————

    /// 拼团商品列表（Java Mapper `page`：LEFT JOIN product + 过滤 + 排序）。
    pub async fn page_items(
        &self,
        q: &ItemPageQuery,
        page: &PageQuery,
        group_product_ids: Option<&[i32]>,
    ) -> Result<(u64, Vec<ItemPageRow>)> {
        let (where_sql, binds) = build_item_filters(q, group_product_ids);
        let count_sql = format!(
            "SELECT COUNT(1) FROM groupon_promotion_item gi LEFT JOIN product p ON gi.product_id = p.product_id{where_sql}"
        );
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        for b in &binds {
            cq = cq.bind(b.as_str());
        }
        let total = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let (limit, offset) = page.limit_offset();
        let dir = if q.ascending() { "ASC" } else { "DESC" };
        let list_sql = format!(
            "SELECT gi.groupon_promotion_item_id, gi.groupon_promotion_id, gi.product_id,
                gi.group_type, gi.group_num, gi.order_limit_num, gi.is_auto_group,
                gi.min_group_num, gi.sort, gi.add_time, gi.min_group_price,
                p.product_name, p.product_sn, p.pic_thumb, p.pic_large, p.pic_url,
                p.product_price, p.product_brief
             FROM groupon_promotion_item gi LEFT JOIN product p ON gi.product_id = p.product_id
             {where_sql} ORDER BY {} {dir} LIMIT ? OFFSET ?",
            q.sort_column()
        );
        let mut lq = sqlx::query_as::<_, ItemPageRow>(&list_sql);
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

    /// 可选商品分页（Java `productService.productList`：上架未删除 + 名称/品牌过滤）。
    pub async fn page_products(
        &self,
        q: &ProductPageQuery,
        page: &PageQuery,
    ) -> Result<(u64, Vec<Product>)> {
        let mut wh = String::from(" WHERE product_status = 1 AND is_delete = 0");
        let mut binds: Vec<String> = Vec::new();
        if let Some(name) = q
            .product_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            wh.push_str(" AND product_name LIKE ?");
            binds.push(format!("%{name}%"));
        }
        if let Some(brand) = q.brand_id {
            wh.push_str(" AND brand_id = ?");
            binds.push(brand.to_string());
        }
        let total_sql = format!("SELECT COUNT(*) FROM product{wh}");
        let mut tq = sqlx::query_as::<_, (i64,)>(&total_sql);
        for b in &binds {
            tq = tq.bind(b.as_str());
        }
        let total = tq.fetch_one(&self.pool).await.map_err(map_err)?;
        let (limit, offset) = page.limit_offset();
        let dir = if q.ascending() { "ASC" } else { "DESC" };
        let list_sql = format!(
            "SELECT * FROM product{wh} ORDER BY {} {dir} LIMIT $1 OFFSET $2",
            q.sort_column()
        );
        let mut lq = sqlx::query_as::<_, Product>(&list_sql);
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

    /// 当前活动已绑定的商品 id。
    pub async fn bound_product_ids(&self, promotion_id: i64) -> Result<Vec<i64>> {
        let rows: Vec<(i64,)> = sqlx::query_as(
            "SELECT product_id FROM groupon_promotion_item WHERE groupon_promotion_id = $1",
        )
        .bind(promotion_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 同步 `promotion`（type=7 拼团）的 range_data / sku_ids（Java `updPromotion`）。
    pub async fn update_groupon_promotion_range(
        &self,
        promotion_id: i64,
        product_ids: &[i64],
        sku_ids: &[i64],
    ) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE promotion SET range_data = $1, sku_ids = $2 WHERE type = 7 AND relation_id = $3",
        )
        .bind(join_i64(product_ids))
        .bind(join_i64(sku_ids))
        .bind(promotion_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }
}

/// `Vec<i64>` → Java `List.toString()` 形态（`[1, 2]`），保持与历史数据兼容。
#[must_use]
pub fn join_i64(items: &[i64]) -> String {
    if items.is_empty() {
        return String::new();
    }
    let inner = items
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{inner}]")
}

fn build_item_filters(
    q: &ItemPageQuery,
    group_product_ids: Option<&[i32]>,
) -> (String, Vec<String>) {
    let mut wh = String::from(" WHERE 1 = 1");
    let mut binds: Vec<String> = Vec::new();
    if let Some(promotion_id) = q.groupon_promotion_id {
        wh.push_str(" AND gi.groupon_promotion_id = ?");
        binds.push(promotion_id.to_string());
    }
    if let Some(kw) = q.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        wh.push_str(" AND p.product_name LIKE ?");
        binds.push(format!("%{kw}%"));
    }
    if let Some(sn) = q
        .product_sn
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        wh.push_str(" AND p.product_sn LIKE ?");
        binds.push(format!("%{sn}%"));
    }
    if let Some(name) = q
        .product_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        wh.push_str(" AND p.product_name LIKE ?");
        binds.push(format!("%{name}%"));
    }
    if let Some(brand) = q.brand_id {
        wh.push_str(" AND p.brand_id = ?");
        binds.push(brand.to_string());
    }
    if let Some(ids) = group_product_ids {
        if ids.is_empty() {
            wh.push_str(" AND p.product_id IN (0)");
        } else {
            let placeholders = vec!["?"; ids.len()].join(", ");
            wh.push_str(&format!(" AND p.product_id IN ({placeholders})"));
            binds.extend(ids.iter().map(|id| id.to_string()));
        }
    }
    (wh, binds)
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("未知的拼团活动商品".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_i64_matches_java_list_tostring() {
        assert_eq!(join_i64(&[]), "");
        assert_eq!(join_i64(&[1]), "[1]");
        assert_eq!(join_i64(&[1, 2, 3]), "[1, 2, 3]");
    }

    #[test]
    fn item_filters_build_parameterized_where() {
        let q = ItemPageQuery {
            groupon_promotion_id: Some(9),
            keyword: Some("鞋".into()),
            brand_id: Some(2),
            ..Default::default()
        };
        let (wh, binds) = build_item_filters(&q, Some(&[1, 2]));
        assert!(wh.contains("gi.groupon_promotion_id = ?"));
        assert!(wh.contains("p.product_id IN (?, ?)"));
        assert_eq!(binds.len(), 5); // 活动 + keyword + 品牌 + 2 商品
        // 空分组 → IN (0) 兜底（Java `List.of(0)`）
        let (wh, _) = build_item_filters(&ItemPageQuery::default(), Some(&[]));
        assert!(wh.contains("IN (0)"));
    }
}
