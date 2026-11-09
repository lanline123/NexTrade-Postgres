//! `product` / `product_sku` 仓储，替代 `ProductMapper` / `ProductSkuMapper`。

use rust_decimal::Decimal;
use sqlx::PgPool;
use nextrade_core::entities::{Product, ProductSku};
use nextrade_core::models::ProductListQuery;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct ProductRepository {
    pool: PgPool,
}

impl ProductRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 按 ID 查询商品。
    pub async fn find_by_id(&self, product_id: i32) -> Result<Option<Product>> {
        sqlx::query_as::<_, Product>("SELECT * FROM product WHERE product_id = $1 LIMIT 1")
            .bind(product_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 商品分页查询（关键词/分类/品牌/状态/价格/ids 过滤 + 排序）。
    pub async fn paginate(
        &self,
        q: &ProductListQuery,
    ) -> Result<(u64, Vec<Product>)> {
        let page = q.page.unwrap_or(1).max(1);
        let page_size = q.page_size.unwrap_or(20).clamp(1, 200);
        let offset = (page - 1) * page_size;

        let keyword = q.keyword.as_ref().map(|k| format!("%{k}%"));
        let (price_min, price_max) = (q.price_min, q.price_max);

        // —— `ids` 精确筛选（Java `ProductListDTO.ids`）——
        // 原版前端首页楼层 / 推荐位用 `?ids=322,321,320` 指定商品；
        // Java 同时用它做 IN 过滤 + `ORDER BY FIELD(product_id, ids)` 排序。
        // 只接受「数字 + 逗号」（同 Java `ids.matches("^[0-9,]+$")`）；
        // 非法输入降级为「无筛选」而不是放宽成全表，避免楼层串货。
        let id_list: Option<Vec<i32>> = q.ids.as_deref().and_then(|raw| {
            let raw = raw.trim();
            if raw.is_empty() || !raw.chars().all(|c| c.is_ascii_digit() || c == ',') {
                return None;
            }
            let ids: Vec<i32> = raw
                .split(',')
                .filter(|s| !s.is_empty())
                .filter_map(|s| s.parse::<i32>().ok())
                .collect();
            if ids.is_empty() { None } else { Some(ids) }
        });
        // `ids` 非空时追加 `AND product_id IN (...)`（占位符个数与 id 数一致）。
        let id_clause = match id_list.as_ref().map(Vec::len) {
            Some(n) => format!(" AND product_id IN ({})", vec!["?"; n].join(", ")),
            None => String::new(),
        };

        let sale_clause = super::product_sale::ProductSaleRepository::sale_condition();

        // 分类筛选：服务层展开后的 category_ids 优先（包含子分类）；
        // 没有展开时保持原来的精确 category_id 语义。
        let category_ids: Vec<i32> = if q.category_ids.is_empty() {
            q.category_id.into_iter().collect()
        } else {
            q.category_ids.clone()
        };
        let category_clause = if category_ids.is_empty() {
            " AND (? IS NULL OR category_id = ?)".to_string()
        } else {
            format!(" AND category_id IN ({})", vec!["?"; category_ids.len()].join(", "))
        };

        let total_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT COUNT(*) FROM product
             WHERE is_delete = 0
               AND ($1 IS NULL OR product_name LIKE $2 OR product_sn LIKE $3)
               {category_clause}
               AND ($4 IS NULL OR brand_id = $5)
               AND ($6 IS NULL OR shop_id = $7)
               AND ($8 IS NULL OR product_status = $9)
               AND ($10 IS NULL OR product_price >= $11)
               AND ($12 IS NULL OR product_price <= $13){sale_clause}{id_clause}"
        ));
        let mut total_q = sqlx::query_as::<_, (i64,)>(&total_sql)
            .bind(&keyword).bind(&keyword).bind(&keyword);
        if category_ids.is_empty() {
            total_q = total_q.bind(q.category_id).bind(q.category_id);
        } else {
            for id in &category_ids {
                total_q = total_q.bind(*id);
            }
        }
        total_q = total_q
            .bind(q.brand_id).bind(q.brand_id)
            .bind(q.shop_id).bind(q.shop_id)
            .bind(q.product_status).bind(q.product_status)
            .bind(price_min).bind(price_min)
            .bind(price_max).bind(price_max);
        if let Some(ids) = &id_list {
            for id in ids {
                total_q = total_q.bind(*id);
            }
        }
        let total: (i64,) = total_q.fetch_one(&self.pool).await.map_err(map_err)?;

        // 排序白名单，防止 SQL 注入。
        // 传了 `ids` 时按 IDs 给定顺序排序（Java `ORDER BY FIELD(product_id, ...)`），
        // 否则前端楼层里「第一个位置」的商品会随机换人。
        let order_by = match id_list.as_ref() {
            Some(ids) => format!(
                "FIELD(product_id, {})",
                ids.iter().map(i32::to_string).collect::<Vec<_>>().join(", ")
            ),
            None => match q.sort_by.as_deref() {
                Some("price") => "product_price",
                Some("sales") => "(virtual_sales + click_count)",
                _ => "add_time",
            }
            .to_string(),
        };
        // `FIELD()` 返回期望顺序，必须 ASC 才是按 ids 顺序。
        let dir = if id_list.is_some() {
            "ASC"
        } else if q.sort_order.as_deref() == Some("asc") {
            "ASC"
        } else {
            "DESC"
        };
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM product
             WHERE is_delete = 0
               AND ($1 IS NULL OR product_name LIKE $2 OR product_sn LIKE $3)
               {category_clause}
               AND ($4 IS NULL OR brand_id = $5)
               AND ($6 IS NULL OR shop_id = $7)
               AND ($8 IS NULL OR product_status = $9)
               AND ($10 IS NULL OR product_price >= $11)
               AND ($12 IS NULL OR product_price <= $13){sale_clause}{id_clause}
             ORDER BY {order_by} {dir}, product_id DESC
             LIMIT $14 OFFSET $15"
        ));

        let mut rows_q = sqlx::query_as::<_, Product>(&sql)
            .bind(&keyword).bind(&keyword).bind(&keyword);
        if category_ids.is_empty() {
            rows_q = rows_q.bind(q.category_id).bind(q.category_id);
        } else {
            for id in &category_ids {
                rows_q = rows_q.bind(*id);
            }
        }
        rows_q = rows_q
            .bind(q.brand_id).bind(q.brand_id)
            .bind(q.shop_id).bind(q.shop_id)
            .bind(q.product_status).bind(q.product_status)
            .bind(price_min).bind(price_min)
            .bind(price_max).bind(price_max);
        if let Some(ids) = &id_list {
            for id in ids {
                rows_q = rows_q.bind(*id);
            }
        }
        let rows = rows_q
            .bind(page_size as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0 as u64, rows))
    }

    /// 按主键批量查询商品（结算页/导入等场景的 N+1 优化）。
    ///
    /// # Errors
    /// 数据库错误。
    pub async fn find_by_ids(&self, ids: &[i32]) -> Result<Vec<Product>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT * FROM product WHERE product_id IN ({placeholders})"));
        let mut q = sqlx::query_as::<_, Product>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 查询商品 SKU 列表。
    pub async fn find_skus(&self, product_id: i32) -> Result<Vec<ProductSku>> {
        sqlx::query_as::<_, ProductSku>(
            "SELECT * FROM product_sku WHERE product_id = $1 ORDER BY sku_id ASC",
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 扣减库存（乐观并发安全：WHERE 保证库存充足才更新）。
    ///
    /// 返回实际扣减数量；库存不足时返回 0，调用方转 `Error::InsufficientStock`。
    pub async fn deduct_stock(
        &self,
        product_id: i32,
        sku_id: Option<i32>,
        quantity: i32,
    ) -> Result<bool> {
        if let Some(p) = self.find_by_id(product_id).await? {
            if p.is_virtual_product() {
                // 虚拟/卡密/付费内容/礼品卡不扣库存。
                return Ok(true);
            }
        }
        let affected = match sku_id {
            Some(sku) => {
                let r = sqlx::query(
                    "UPDATE product_sku SET sku_stock = sku_stock - $1
                     WHERE sku_id = $2 AND sku_stock >= $3 AND is_enabled = 1",
                )
                .bind(quantity)
                .bind(sku)
                .bind(quantity)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
                r.rows_affected()
            }
            None => {
                let r = sqlx::query(
                    "UPDATE product SET product_stock = product_stock - $1
                     WHERE product_id = $2 AND product_stock >= $3",
                )
                .bind(quantity)
                .bind(product_id)
                .bind(quantity)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
                r.rows_affected()
            }
        };
        Ok(affected > 0)
    }

    /// 回滚库存（取消/退款场景）。
    pub async fn restore_stock(
        &self,
        product_id: i32,
        sku_id: Option<i32>,
        quantity: i32,
    ) -> Result<()> {
        match sku_id {
            Some(sku) => {
                sqlx::query("UPDATE product_sku SET sku_stock = sku_stock + $1 WHERE sku_id = $2")
                    .bind(quantity)
                    .bind(sku)
                    .execute(&self.pool)
                    .await
                    .map_err(map_err)?;
            }
            None => {
                sqlx::query("UPDATE product SET product_stock = product_stock + $1 WHERE product_id = $2")
                    .bind(quantity)
                    .bind(product_id)
                    .execute(&self.pool)
                    .await
                    .map_err(map_err)?;
            }
        }
        Ok(())
    }

    /// 商品是否上架且库存充足。
    pub async fn check_purchasable(
        &self,
        product_id: i32,
        sku_id: Option<i32>,
        quantity: i32,
    ) -> Result<bool> {
        if let Some(p) = self.find_by_id(product_id).await? {
            if !p.is_on_sale() {
                return Ok(false);
            }
            if p.is_virtual_product() {
                return Ok(true);
            }
            if let Some(sku) = sku_id {
                let skus = self.find_skus(product_id).await?;
                let found = skus.iter().find(|s| s.sku_id == sku);                return Ok(match found {
                    Some(s) => s.available_stock() >= i64::from(quantity),
                    None => false,
                });
            }
            return Ok(i64::from(p.product_stock) >= i64::from(quantity));
        }
        Ok(false)
    }

    /// 查询商品当前售价（促销价优先）。
    pub async fn current_price(&self, product_id: i32) -> Result<Option<Decimal>> {
        let p = self.find_by_id(product_id).await?;
        Ok(p.map(|p| p.current_price()))
    }

    /// 批量查询商品售价（消除下单循环内 N+1 查询）。
    ///
    /// 返回 `product_id -> product_price`；库中不存在的 ID 不出现在结果里，
    /// 调用方需自行回退（如取购物车行原价）。空列表直接返回空 map，不查库。
    pub async fn current_prices(
        &self,
        product_ids: &[i32],
    ) -> Result<std::collections::HashMap<i32, Option<Decimal>>> {
        let mut map = std::collections::HashMap::with_capacity(product_ids.len());
        if product_ids.is_empty() {
            return Ok(map);
        }

        let placeholders = vec!["?"; product_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT product_id, product_price FROM product WHERE product_id IN ({placeholders})"
        ));
        let mut query = sqlx::query_as::<_, (i32, Option<Decimal>)>(&sql);
        for pid in product_ids {
            query = query.bind(pid);
        }
        for (pid, price) in query.fetch_all(&self.pool).await.map_err(map_err)? {
            map.insert(pid, price);
        }
        Ok(map)
    }

    /// 事务内扣减库存（供下单使用，随外层事务一起提交或回滚）。
    pub async fn deduct_stock_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        product_id: i32,
        sku_id: Option<i32>,
        quantity: i32,
    ) -> Result<bool> {
        if let Some(p) = self.find_by_id(product_id).await? {
            if p.is_virtual_product() {
                return Ok(true);
            }
        }
        let affected = match sku_id {
            Some(sku) => {
                sqlx::query(
                    "UPDATE product_sku SET sku_stock = sku_stock - $1
                     WHERE sku_id = $2 AND sku_stock >= $3 AND is_enabled = 1",
                )
                .bind(quantity)
                .bind(sku)
                .bind(quantity)
                .execute(&mut **tx)
                .await
                .map_err(map_err)?
                .rows_affected()
            }
            None => {
                sqlx::query(
                    "UPDATE product SET product_stock = product_stock - $1
                     WHERE product_id = $2 AND product_stock >= $3",
                )
                .bind(quantity)
                .bind(product_id)
                .bind(quantity)
                .execute(&mut **tx)
                .await
                .map_err(map_err)?
                .rows_affected()
            }
        };
        Ok(affected > 0)
    }

    /// 事务内回滚库存（下单失败时补偿）。
    pub async fn restore_stock_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        product_id: i32,
        sku_id: Option<i32>,
        quantity: i32,
    ) -> Result<()> {
        match sku_id {
            Some(sku) => {
                sqlx::query("UPDATE product_sku SET sku_stock = sku_stock + $1 WHERE sku_id = $2")
                    .bind(quantity)
                    .bind(sku)
                    .execute(&mut **tx)
                    .await
                    .map_err(map_err)?;
            }
            None => {
                sqlx::query("UPDATE product SET product_stock = product_stock + $1 WHERE product_id = $2")
                    .bind(quantity)
                    .bind(product_id)
                    .execute(&mut **tx)
                    .await
                    .map_err(map_err)?;
            }
        }
        Ok(())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("商品不存在".into()),
        other => Error::Database(other.to_string()),
    }
}
