//! 推荐 域仓储（system::recommend）—— wave9。
//!
//! 直接读 `product` 表（上架、未删除、店铺过滤），运行时 sqlx。

use rust_decimal::Decimal;
use sqlx::PgPool;
use sqlx::FromRow;

use nextrade_core::models::system::recommend::GuessLikeQuery;
use nextrade_core::{Error, Result};

/// 推荐商品行。
#[derive(Debug, Clone, FromRow)]
pub struct RecommendProductRow {
    /// 商品 ID
    pub product_id: i32,
    /// 商品名称
    pub product_name: String,
    /// 商品货号
    pub product_sn: String,
    /// 售价
    pub product_price: Decimal,
    /// 主图
    pub pic_url: String,
    /// 缩略图
    pub pic_thumb: String,
    /// 店铺 ID
    pub shop_id: i32,
    /// 商品类型
    pub product_type: Option<i32>,
    /// 库存
    pub product_stock: Option<i32>,
    /// 虚拟销量
    pub virtual_sales: Option<i32>,
}

/// 推荐仓储。
#[derive(Clone)]
pub struct RecommendRepository {
    pool: PgPool,
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

impl RecommendRepository {
    /// 构造仓储。
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 猜你喜欢分页（Java `getProductList`）。
    pub async fn guess_like(
        &self,
        q: &GuessLikeQuery,
    ) -> Result<(u64, Vec<RecommendProductRow>)> {
        let mut conds: Vec<String> = vec![
            "is_delete = 0".to_string(),
            "product_status = 1".to_string(),
        ];
        let mut binds: Vec<i64> = Vec::new();
        if let Some(shop_id) = q.shop_id.filter(|v| *v > 0) {
            conds.push("shop_id = ?".to_string());
            binds.push(i64::from(shop_id));
        }
        if let Some(t) = q.product_type {
            conds.push("product_type = ?".to_string());
            binds.push(i64::from(t));
        }
        if let Some(c) = q.category_id.filter(|v| *v > 0) {
            conds.push("category_id = ?".to_string());
            binds.push(i64::from(c));
        }
        let where_sql = format!(" WHERE {}", conds.join(" AND "));
        let page = q.page_query();
        let size = u64::from(page.page_size);
        let offset = u64::from(page.page - 1) * size;
        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM product{where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        for b in &binds {
            cq = cq.bind(*b);
        }
        let total = cq.fetch_one(&self.pool).await.map_err(map_err)?.0;
        let dir = if q.ascending() { "ASC" } else { "DESC" };
        let order_sql = match q.sort_field.as_deref() {
            Some("sales") => format!(" ORDER BY virtual_sales {dir}, product_id DESC"),
            Some("click") => format!(" ORDER BY click_count {dir}, product_id DESC"),
            Some("new") => format!(" ORDER BY add_time {dir}, product_id DESC"),
            _ => " ORDER BY is_best DESC, click_count DESC, product_id DESC".to_string(),
        };
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT product_id, product_name, product_sn, product_price, pic_url, pic_thumb, \
             shop_id, product_type, product_stock, virtual_sales FROM product{where_sql}{order_sql} LIMIT ? OFFSET ?"
        ));
        let mut rq = sqlx::query_as::<_, RecommendProductRow>(&sql);
        for b in &binds {
            rq = rq.bind(*b);
        }
        let rows = rq
            .bind(i64::try_from(size).unwrap_or(10))
            .bind(i64::try_from(offset).unwrap_or(0))
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total).unwrap_or(0), rows))
    }

    /// 推荐商品 ID 列表（Java \"getProductIds\"）。
    pub async fn recommend_ids(&self, limit: u32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT product_id FROM product WHERE is_delete = 0 AND product_status = 1 \
             ORDER BY is_best DESC, click_count DESC, product_id DESC LIMIT $1",
        )
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 精品 + 热销商品 ID。
    ///
    /// 对应 Java \"getRecommendedProductIds\" 的第 1、2 段（`is_best = 1 LIMIT 10`
    /// 与 `is_hot = 1 LIMIT 10`）。原版是两次查询后并集去重，这里合成一次，
    /// 用 `is_best DESC` 保证精品始终排在热销之前。
    pub async fn best_hot_ids(&self, limit: u32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT product_id FROM product WHERE is_delete = 0 AND product_status = 1 \
             AND (is_best = 1 OR is_hot = 1) \
             ORDER BY is_best DESC, click_count DESC, product_id DESC LIMIT $1",
        )
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 会员浏览记录中的商品 ID（Java `User.historyProductIds`，按最近在前截取）。
    ///
    /// Java 侧用 hutool `JSONUtil.toList(raw, Integer.class)` 解析，落库是 JSON 数组；
    /// 这里兼容可能出现的逗号分隔写法，解析失败一律当作空。
    pub async fn history_ids(&self, user_id: i32, limit: u32) -> Result<Vec<i32>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT history_product_ids FROM \"user\" WHERE user_id = $1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;

        let Some((Some(raw),)) = row else {
            return Ok(Vec::new());
        };
        let raw = raw.trim();
        if raw.is_empty() {
            return Ok(Vec::new());
        }

        let mut ids: Vec<i32> = serde_json::from_str::<Vec<i32>>(raw).unwrap_or_else(|_| {
            raw.split(',')
                .filter_map(|s| s.trim().trim_matches('"').parse::<i32>().ok())
                .collect()
        });
        ids.truncate(limit as usize);
        Ok(ids)
    }

    /// 相似用户近期购买聚合榜（扩展接口 `cohortBuys` 的数据层）。
    ///
    /// 协同过滤的轻量实现，全程只做聚合、不回任何其他用户的信息：
    /// - 登录用户（`user_id > 0`）：先取其已购商品的分类集合，再找近 `days`
    ///   天内买过相同分类商品的其他用户（同画像群体），聚合该群体近 `days`
    ///   天买过的商品，按「购买人数 → 虚拟销量」排序，并排除本人已购商品。
    ///   本人无购买记录或群体为空时返回空，由服务层走全局兜底。
    /// - 游客（`user_id = 0`）：直接聚合全体用户近 `days` 天的购买榜
    ///   （「大家最近在买」）。
    ///
    /// 订单口径与销售统计一致：`"order"` 表 `is_del = 0 AND pay_status > 0`
    /// （已支付），`add_time` 为 Unix 秒。商品须在售（未删 + 上架）。
    pub async fn cohort_buys(
        &self,
        user_id: i32,
        days: u32,
        shop_id: Option<i32>,
        limit: u64,
        offset: u64,
    ) -> Result<Vec<RecommendProductRow>> {
        let shop = shop_id.filter(|v| *v > 0);
        let mut binds: Vec<i64> = Vec::new();
        let select_cols = "pr.product_id, pr.product_name, pr.product_sn, pr.product_price, \
             pr.pic_url, pr.pic_thumb, pr.shop_id, pr.product_type, pr.product_stock, pr.virtual_sales";
        let mut sql = if user_id > 0 {
            binds.push(i64::from(user_id));
            binds.push(i64::from(user_id));
            binds.push(i64::from(days));
            binds.push(i64::from(days));
            binds.push(i64::from(user_id));
            (format!(
                "WITH my_cats AS (\
                   SELECT DISTINCT p.category_id AS cid FROM order_item i \
                   JOIN \"order\" o ON o.order_id = i.order_id \
                   JOIN product p ON p.product_id = i.product_id \
                   WHERE o.user_id = ? AND o.is_del = 0 AND o.pay_status > 0 AND p.category_id > 0\
                 ), cohort AS (\
                   SELECT DISTINCT o.user_id AS uid FROM order_item i \
                   JOIN \"order\" o ON o.order_id = i.order_id \
                   JOIN product p ON p.product_id = i.product_id \
                   WHERE o.is_del = 0 AND o.pay_status > 0 AND o.user_id <> ? \
                   AND o.add_time >= (EXTRACT(EPOCH FROM NOW())::bigint - ? * 86400) \
                   AND p.category_id IN (SELECT cid FROM my_cats)\
                 ) \
                 SELECT {select_cols} FROM order_item i \
                 JOIN \"order\" o ON o.order_id = i.order_id \
                 JOIN product pr ON pr.product_id = i.product_id \
                 WHERE o.user_id IN (SELECT uid FROM cohort) \
                 AND o.is_del = 0 AND o.pay_status > 0 \
                 AND o.add_time >= (EXTRACT(EPOCH FROM NOW())::bigint - ? * 86400) \
                 AND pr.is_delete = 0 AND pr.product_status = 1 \
                 AND i.product_id NOT IN (\
                   SELECT mi.product_id FROM order_item mi \
                   JOIN \"order\" mo ON mo.order_id = mi.order_id \
                   WHERE mo.user_id = ? AND mo.is_del = 0 AND mo.pay_status > 0\
                 )"
            )
        } else {
            binds.push(i64::from(days));
            (format!(
                "SELECT {select_cols} FROM order_item i \
                 JOIN \"order\" o ON o.order_id = i.order_id \
                 JOIN product pr ON pr.product_id = i.product_id \
                 WHERE o.is_del = 0 AND o.pay_status > 0 \
                 AND o.add_time >= (EXTRACT(EPOCH FROM NOW())::bigint - ? * 86400) \
                 AND pr.is_delete = 0 AND pr.product_status = 1"
            )
        };
        if let Some(sid) = shop {
            sql.push_str(" AND pr.shop_id = ?");
            binds.push(i64::from(sid));
        }
        sql.push_str(
            " GROUP BY pr.product_id \
             ORDER BY COUNT(DISTINCT o.user_id) DESC, pr.virtual_sales DESC, pr.product_id DESC \
             LIMIT ? OFFSET ?",
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut rq = sqlx::query_as::<_, RecommendProductRow>(&sql);
        for b in &binds {
            rq = rq.bind(*b);
        }
        let rows = rq
            .bind(i64::try_from(limit).unwrap_or(10))
            .bind(i64::try_from(offset).unwrap_or(0))
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows)
    }

    /// 兜底热门商品 ID。
    ///
    /// 对应原版第 4 段「不足 40 个时补全」。原版用 `ORDER BY RAND()` 且只从
    /// `is_hot = 1` 里取，本站热销商品仅 4 个、补不出量；这里改为按点击量取
    /// 在售商品，结果稳定、可缓存，也避免每次请求都做全表随机排序。
    pub async fn popular_ids(&self, limit: u32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT product_id FROM product WHERE is_delete = 0 AND product_status = 1 \
             ORDER BY click_count DESC, product_id DESC LIMIT $1",
        )
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }
}
