//! 店铺域前台（merchant::shop_client）仓储：`shop` / `collect_shop` / `comment` / `product` /
//! `order_item` / `decorate` / `decorate_discrete` / `locales` / `merchant` /
//! `merchant_apply` / `region` / `config` / `store_product` / `store_tip` / `tip` /
//! `shop_local_delivery_config` / `shop_home_service_config` / `promotion`。
//!
//! 列名严格对齐 `java/database/pure.sql`
//! （`python tools/schema_parser.py --table <表名>` 可复核）：
//! - `shop`：`shop_id` / `shop_title` / `add_time` / `shop_logo` / `click_count` /
//!   `status` / `merchant_id` / `shop_money` / `frozen_money` / `contact_mobile` /
//!   `description` / `kefu_phone` / `kefu_weixin` / `kefu_link` / `is_contact_kefu` /
//!   `kefu_inlet` / `shop_type` / `store_parent_id` / `shop_cover_picture` /
//!   `shop_contact_json` / `shop_open_close_json` / `shop_region_id` /
//!   `shop_detailed_address` / `shop_longitude` / `shop_latitude` / `shop_star` /
//!   `shop_star_num` / `shop_sales` / `is_delete`
//! - `collect_shop`：`collect_id` / `user_id` / `shop_id` / `add_time`
//! - `comment`：`comment_id` / `user_id` / `username` / `is_anonymous` / `avatar` /
//!   `product_id` / `order_id` / `order_item_id` / `comment_rank` / `content` /
//!   `add_time` / `status` / `parent_id` / `usefull` / `useless` / `comment_tag` /
//!   `show_pics` / `is_recommend` / `is_top` / `is_showed` / `is_default` /
//!   `sort_order` / `shop_id`
//! - `decorate`：`decorate_id` / `data` / `decorate_type` / `page_type` / `is_home` /
//!   `shop_id` / `status` / `locale_id` / `parent_id`
//! - `decorate_discrete`：`id` / `decorate_sn` / `decorate_name` / `data` / `shop_id`
//! - `store_tip`：`store_tip_id` / `tip_id` / `shop_id` / `sort` / `add_time`
//! - `tip`：`tip_id` / `tip_name` / `status` / `add_time`
//! - `config`：`biz_code` / `biz_val` / `is_del`
//! - `promotion`：`type` / `relation_id` / `is_available` / `is_delete` / `end_time`
//! - `order`（保留字，需反引号）：`is_valid` / `is_del` / `order_status`
//!
//! 查询统一使用运行时 `sqlx::query_as`（**不使用 `query!` 宏**）。
//! 过滤条件一律 `?` 占位 + 有序绑定（[`Bind`]），动态片段只拼接固定字面量，
//! 不做字符串插值，避免注入。

use std::collections::HashMap;

use rust_decimal::Decimal;
use sqlx::PgPool;
use nextrade_core::entities::{
    CollectShop, Comment, Decorate, DecorateDiscrete, Locales, Merchant, OrderItem, Product, Shop,
    ShopHomeServiceConfig, ShopLocalDeliveryConfig, StoreProduct, StoreTip, Tip,
};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// 有序绑定参数（与 SQL 中 `?` 的出现顺序 1:1 对应）。
#[derive(Debug, Clone)]
pub enum Bind {
    I32(i32),
    I64(i64),
    Dec(Decimal),
    Str(String),
    /// `NULL`（用于 `? IS NULL` 判定，如未传经纬度的距离 CASE）
    Null,
}

/// 把 `Bind` 列表按顺序绑定到查询上（宏展开以适配不同返回行类型）。
macro_rules! bind_all {
    ($q:expr, $binds:expr) => {{
        let mut q = $q;
        for b in $binds.iter() {
            q = match b {
                Bind::I32(v) => q.bind(*v),
                Bind::I64(v) => q.bind(*v),
                Bind::Dec(v) => q.bind(*v),
                Bind::Str(v) => q.bind(v.as_str()),
                Bind::Null => q.bind(Option::<Decimal>::None),
            };
        }
        q
    }};
}

/// 门店列表投影（对应 `ShopMapper.storeListByLocation` / `storeListByRegion`）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct StoreListRow {
    pub shop_id: i32,
    pub shop_logo: Option<String>,
    pub status: Option<i32>,
    /// `shop_detailed_address`
    pub address: Option<String>,
    pub shop_title: Option<String>,
    pub longitude: Option<Decimal>,
    pub latitude: Option<Decimal>,
    /// `shop_contact_json` 原文
    pub contact_json: Option<String>,
    /// SQL `calculateShopStar`：评价数 < 100/空/0 → 4.8，否则 `shop_star`
    pub shop_star: Option<Decimal>,
    pub shop_star_num: Option<i32>,
    pub description: Option<String>,
    pub shop_sales: Option<i32>,
    /// `shop_open_close_json` 原文
    pub open_close_json: Option<String>,
    /// 距离（米），无经纬度时为 `NULL`
    pub distance: Option<Decimal>,
}

/// `storeList` 过滤条件（`ShopServiceImpl#storeList` + 两个 XML 查询）。
#[derive(Debug, Clone, Default)]
pub struct StoreListFilter {
    pub keyword: Option<String>,
    pub region_id: Option<String>,
    /// 显式指定店铺集合（非空时按 `IN` 过滤）
    pub shop_ids: Vec<i32>,
    /// 配送类型多选（含 4 过滤同城配送、含 2 过滤预约上门）
    pub delivery_types: Vec<i32>,
    /// `withinDeliveryRange == 1` 且带有效经纬度时按半径过滤
    pub within_delivery_range: bool,
    /// 选中门店：`ORDER BY shop_id = ? DESC` 置顶
    pub selected_shop_id: Option<i32>,
    pub sort_field: Option<String>,
    pub sort_order: Option<String>,
}

/// 自提点列表投影（对应 `ShopMapper.pickupListByLocation`）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct PickupRow {
    pub shop_id: i32,
    pub shop_title: Option<String>,
    /// 距离（米）
    pub distance: Option<Decimal>,
    pub longitude: Option<Decimal>,
    pub latitude: Option<Decimal>,
    /// `shop_contact_json` 原文
    pub contact_json: Option<String>,
    /// `shop_open_close_json` 原文
    pub open_close_json: Option<String>,
    /// `shop_region_id` 原文（JSON 数组串）
    pub region_ids_json: Option<String>,
    /// `shop_detailed_address`
    pub address: Option<String>,
}

/// 店铺 SQL 排序白名单（`Shop` 实体列）。
const SHOP_SORTABLE: &[(&str, &str)] = &[
    ("shopId", "shop_id"),
    ("shopTitle", "shop_title"),
    ("addTime", "add_time"),
    ("shopLogo", "shop_logo"),
    ("clickCount", "click_count"),
    ("status", "status"),
    ("merchantId", "merchant_id"),
    ("shopMoney", "shop_money"),
    ("frozenMoney", "frozen_money"),
    ("contactMobile", "contact_mobile"),
    ("shopStar", "shop_star"),
    ("shopStarNum", "shop_star_num"),
    ("shopSales", "shop_sales"),
    ("sortOrder", "sort_order"),
];

#[derive(Clone)]
pub struct ShopClientRepository {
    pool: PgPool,
}

impl ShopClientRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 店铺 —————————————————————

    /// 前台店铺分页（`ShopServiceImpl#clientList` 的主查询）。
    ///
    /// `status = 1`、`keyword` 模糊匹配 `shop_title`；排序走白名单。
    pub async fn paginate_client_shops(
        &self,
        keyword: Option<&str>,
        sort_field: Option<&str>,
        sort_order: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<Shop>)> {
        let count_sql = "SELECT COUNT(*) FROM shop
             WHERE status = 1
               AND ($1 IS NULL OR shop_title ILIKE CONCAT('%', $2, '%'))";
        let total: (i64,) = sqlx::query_as(count_sql)
            .bind(keyword)
            .bind(keyword)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let order = order_by(sort_field, sort_order, "shop_id", SHOP_SORTABLE);
        let (limit, offset) = page.limit_offset();
        let sql = format!(
            "SELECT * FROM shop
             WHERE status = 1
               AND ($1 IS NULL OR shop_title ILIKE CONCAT('%', $2, '%'))
             {order} LIMIT $3 OFFSET $4"
        );
        let rows = sqlx::query_as::<_, Shop>(&sql)
            .bind(keyword)
            .bind(keyword)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 按主键查询店铺。
    pub async fn find_shop(&self, shop_id: i32) -> Result<Option<Shop>> {
        sqlx::query_as::<_, Shop>("SELECT * FROM shop WHERE shop_id = $1 LIMIT 1")
            .bind(shop_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按主键集合查询店铺。
    pub async fn find_shops_by_ids(&self, ids: &[i32]) -> Result<Vec<Shop>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!("SELECT * FROM shop WHERE shop_id IN ({})", placeholders(ids.len()));
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, Shop>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 门店列表（`storeListByLocation` / `storeListByRegion`）。
    ///
    /// `location` 为 `Some((longitude, latitude))` 时走经纬度变体
    /// （SELECT 内计算 `distance` 并按 `distance` 排序），否则走地区变体
    /// （`distance` 恒为 `NULL`）。
    pub async fn paginate_stores(
        &self,
        f: &StoreListFilter,
        location: Option<(Decimal, Decimal)>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<StoreListRow>)> {
        let (select_sql, select_binds) = self.store_select(location);
        let (where_sql, where_binds) = self.store_where(f, location);
        let (order_sql, order_binds) = self.store_order(f, location);

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM shop {where_sql}"));
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&count_sql), &where_binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut binds = select_binds;
        binds.extend(where_binds);
        binds.extend(order_binds);

        let (limit, offset) = page.limit_offset();
        let sql = crate::db::to_pg_placeholders(&format!("{select_sql} {where_sql} {order_sql} LIMIT ? OFFSET ?"));
        let mut q = bind_all!(sqlx::query_as::<_, StoreListRow>(&sql), &binds);
        q = q.bind(limit).bind(offset);
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 门店 SELECT 列表 + 其中的绑定（距离表达式占位）。
    fn store_select(&self, location: Option<(Decimal, Decimal)>) -> (String, Vec<Bind>) {
        let mut binds = Vec::new();
        let distance = match location {
            // 与 `ShopMapper.storeListByLocation` 的距离表达式一致（球面余弦定理，半径 6371000m）
            Some((lng, lat)) => {
                binds.push(Bind::Dec(lat));
                binds.push(Bind::Dec(lng));
                binds.push(Bind::Dec(lat));
                "CAST(6371000 * acos(cos(radians(?)) * cos(radians(shop_latitude)) \
                 * cos(radians(shop_longitude) - radians(?)) \
                 + sin(radians(?)) * sin(radians(shop_latitude))) AS DECIMAL(20,6))"
            }
            None => "CAST(NULL AS DECIMAL(20,6))",
        };
        let sql = format!(
            "SELECT shop_id, shop_logo, status, shop_detailed_address AS address, shop_title, \
             shop_longitude AS longitude, shop_latitude AS latitude, \
             shop_contact_json AS contact_json, \
             CASE WHEN shop_star_num < 100 OR shop_star_num IS NULL OR shop_star_num = 0 \
                  THEN 4.8 ELSE shop_star END AS shop_star, \
             shop_star_num, description, shop_sales, \
             shop_open_close_json AS open_close_json, \
             {distance} AS distance \
             FROM shop"
        );
        (sql, binds)
    }

    /// 门店 WHERE 片段 + 绑定。
    fn store_where(
        &self,
        f: &StoreListFilter,
        location: Option<(Decimal, Decimal)>,
    ) -> (String, Vec<Bind>) {
        let mut sql = String::from("WHERE shop_type = 2 AND (status = 1 OR status = 10)");
        let mut binds = Vec::new();

        if let Some(region) = f.region_id.as_deref().filter(|s| !s.is_empty()) {
            sql.push_str(" AND shop_region_id ILIKE CONCAT('%', ?, '%')");
            binds.push(Bind::Str(region.to_string()));
        }
        if let Some(kw) = f.keyword.as_deref().filter(|s| !s.is_empty()) {
            sql.push_str(" AND shop_title ILIKE CONCAT('%', ?, '%')");
            binds.push(Bind::Str(kw.to_string()));
        }
        if !f.shop_ids.is_empty() {
            sql.push_str(&format!(" AND shop_id IN ({})", placeholders(f.shop_ids.len())));
            for id in &f.shop_ids {
                binds.push(Bind::I32(*id));
            }
        }

        // 配送类型过滤：多个类型之间是 OR（与 XML `<trim prefixOverrides="OR">` 一致）
        let has_local = f.delivery_types.contains(&4);
        let has_home = f.delivery_types.contains(&2);
        if has_local || has_home {
            let mut parts: Vec<&str> = Vec::new();
            if has_local {
                parts.push(
                    "EXISTS (SELECT 1 FROM shop_local_delivery_config sldc \
                     WHERE sldc.shop_id = shop.shop_id AND sldc.enabled = 1)",
                );
            }
            if has_home {
                parts.push(
                    "EXISTS (SELECT 1 FROM shop_home_service_config shsc \
                     WHERE shsc.shop_id = shop.shop_id AND shsc.enabled = 1)",
                );
            }
            sql.push_str(&format!(" AND ({})", parts.join(" OR ")));
        }

        // 仅在传了有效经纬度时生效
        if f.within_delivery_range {
            if let Some((lng, lat)) = location {
                sql.push_str(
                    " AND EXISTS (SELECT 1 FROM shop_local_delivery_config sldc \
                     WHERE sldc.shop_id = shop.shop_id AND sldc.enabled = 1 \
                       AND sldc.delivery_radius IS NOT NULL AND sldc.delivery_radius > 0 \
                       AND (6371000 * acos(cos(radians(?)) * cos(radians(shop.shop_latitude)) \
                            * cos(radians(shop.shop_longitude) - radians(?)) \
                            + sin(radians(?)) * sin(radians(shop.shop_latitude)))) \
                           <= sldc.delivery_radius)",
                );
                binds.push(Bind::Dec(lat));
                binds.push(Bind::Dec(lng));
                binds.push(Bind::Dec(lat));
            }
        }

        (sql, binds)
    }

    /// 门店 ORDER BY 片段 + 其中的绑定（\"selectedShopId\" 置顶占位）。
    fn store_order(
        &self,
        f: &StoreListFilter,
        location: Option<(Decimal, Decimal)>,
    ) -> (String, Vec<Bind>) {
        let mut binds: Vec<Bind> = Vec::new();
        let mut orders: Vec<String> = Vec::new();
        if let Some(sel) = f.selected_shop_id.filter(|v| *v > 0) {
            orders.push("shop_id = ? DESC".to_string());
            binds.push(Bind::I32(sel));
        }
        let dir = if f
            .sort_order
            .as_deref()
            .map(|o| o.eq_ignore_ascii_case("asc"))
            .unwrap_or(false)
        {
            "ASC"
        } else {
            "DESC"
        };
        if location.is_some() {
            // 经纬度变体：sortField=distance 时按距离，否则按评星
            if f.sort_field.as_deref() == Some("distance") {
                // PG forbids referencing a SELECT alias inside an ORDER BY expression.
                orders.push(format!("distance {dir} NULLS LAST"));
            } else {
                orders.push(format!("shop_star {dir}"));
            }
        } else {
            // 地区变体固定按评星
            orders.push(format!("shop_star {dir}"));
        }
        (format!("ORDER BY {}", orders.join(", ")), binds)
    }

    // ————————————————————— 商品 —————————————————————

    /// 批量取上架商品（按 `product_id` 倒序，供列表页每店截取前 N 个）。
    pub async fn find_products_by_shops(&self, shop_ids: &[i32]) -> Result<Vec<Product>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM product WHERE shop_id IN ({}) AND product_status = 1 AND is_delete = 0 \
             ORDER BY product_id DESC",
            placeholders(shop_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, Product>(&sql);
        for id in shop_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 按主键批量取商品（评价列表回填商品名/货号/缩略图，**不限状态**）。
    pub async fn find_products_by_ids(&self, product_ids: &[i32]) -> Result<Vec<Product>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM product WHERE product_id IN ({})",
            placeholders(product_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, Product>(&sql);
        for id in product_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 各店铺上架商品数（`product_status = 1 AND is_delete = 0`）。
    pub async fn count_products_by_shops(&self, shop_ids: &[i32]) -> Result<Vec<(i32, i64)>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT shop_id, COUNT(*) AS cnt FROM product \
             WHERE shop_id IN ({}) AND product_status = 1 AND is_delete = 0 GROUP BY shop_id",
            placeholders(shop_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, (i32, i64)>(&sql);
        for id in shop_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 某店铺上架商品（平台自营占位用，`limit <= 0` 时不查）。
    pub async fn find_shop_products(&self, shop_id: i32, limit: i64) -> Result<Vec<Product>> {
        if limit <= 0 {
            return Ok(Vec::new());
        }
        sqlx::query_as::<_, Product>(
            "SELECT * FROM product WHERE shop_id = $1 AND product_status = 1 AND is_delete = 0 \
             ORDER BY product_id DESC LIMIT $2",
        )
        .bind(shop_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 某店铺上架商品数。
    pub async fn count_shop_products_online(&self, shop_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM product WHERE shop_id = $1 AND product_status = 1 AND is_delete = 0",
        )
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 某店铺商品总数（Java `productService.count(eq(shop_id, id))`，**不限状态**）。
    pub async fn count_shop_products_all(&self, shop_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM product WHERE shop_id = $1")
            .bind(shop_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0)
    }

    /// 某店铺新品数（`is_new = 1`，不限状态）。
    pub async fn count_shop_new_products(&self, shop_id: i32) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM product WHERE shop_id = $1 AND is_new = 1")
                .bind(shop_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0)
    }

    /// 一批商品中满足 `is_new = 1` 的数量（门店分配新品）。
    pub async fn count_new_products_in(&self, product_ids: &[i32]) -> Result<i64> {
        if product_ids.is_empty() {
            return Ok(0);
        }
        let sql = format!(
            "SELECT COUNT(*) FROM product WHERE product_id IN ({}) AND is_new = 1",
            placeholders(product_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, (i64,)>(&sql);
        for id in product_ids {
            q = q.bind(*id);
        }
        let row = q.fetch_one(&self.pool).await.map_err(map_err)?;
        Ok(row.0)
    }

    // ————————————————————— 收藏 —————————————————————

    /// 查询某会员对某店铺的收藏记录。
    pub async fn find_collect(&self, user_id: i32, shop_id: i32) -> Result<Option<CollectShop>> {
        sqlx::query_as::<_, CollectShop>(
            "SELECT * FROM collect_shop WHERE user_id = $1 AND shop_id = $2 LIMIT 1",
        )
        .bind(user_id)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 新增收藏（\"add_time\" 与原 Java 一致落库默认 0）。
    pub async fn insert_collect(&self, user_id: i32, shop_id: i32) -> Result<i32> {
        let r: i32 = sqlx::query_scalar("INSERT INTO collect_shop (user_id, shop_id, add_time) VALUES ($1, $2, 0) RETURNING collect_id")
            .bind(user_id)
            .bind(shop_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r)
    }

    /// 删除收藏。
    pub async fn delete_collect(&self, collect_id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM collect_shop WHERE collect_id = $1")
            .bind(collect_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 各店铺收藏人数（`COUNT(DISTINCT user_id)`）。
    pub async fn count_collect_users_by_shops(
        &self,
        shop_ids: &[i32],
    ) -> Result<Vec<(i32, i64)>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT shop_id, COUNT(DISTINCT user_id) AS cnt FROM collect_shop \
             WHERE shop_id IN ({}) GROUP BY shop_id",
            placeholders(shop_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, (i32, i64)>(&sql);
        for id in shop_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 某店铺收藏人数。
    pub async fn count_collect_users(&self, shop_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT user_id) FROM collect_shop WHERE shop_id = $1",
        )
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 本页店铺的全部收藏行（用于聚合收藏数 + 当前用户收藏集合）。
    pub async fn find_collects_by_shops(&self, shop_ids: &[i32]) -> Result<Vec<CollectShop>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM collect_shop WHERE shop_id IN ({})",
            placeholders(shop_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, CollectShop>(&sql);
        for id in shop_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 评价 —————————————————————

    /// 各店铺商品评分均值（`AVG(comment_rank)`）。
    pub async fn avg_comment_rank_by_shops(
        &self,
        shop_ids: &[i32],
    ) -> Result<Vec<(i32, Decimal)>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT shop_id, AVG(comment_rank) AS avg_score FROM comment \
             WHERE shop_id IN ({}) GROUP BY shop_id",
            placeholders(shop_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, (i32, Decimal)>(&sql);
        for id in shop_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 单店铺商品评分均值（带 `GROUP BY shop_id`：无评价时返回 `None`）。
    pub async fn avg_comment_rank(&self, shop_id: i32) -> Result<Option<Decimal>> {
        let row: Option<(Decimal,)> = sqlx::query_as(
            "SELECT AVG(comment_rank) FROM comment WHERE shop_id = $1 GROUP BY shop_id",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|r| r.0))
    }

    /// 店铺评价分页（`CommentServiceImpl#listByShopId` 的主查询）。
    ///
    /// \"is_showed\" / \"type\" 为 \"None\" 时不过滤；\"type\"：2=好评(>=4)、3=中评(=3)、
    /// 4=差评(<=2)。
    pub async fn paginate_comments(
        &self,
        shop_id: i32,
        is_showed: Option<i32>,
        r#type: Option<i32>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<Comment>)> {
        let mut where_sql = String::from("WHERE shop_id = $1 AND parent_id = 0");
        let mut binds: Vec<Bind> = vec![Bind::I32(shop_id)];
        if let Some(showed) = is_showed {
            where_sql.push_str(" AND is_showed = ?");
            binds.push(Bind::I32(showed));
        }
        match r#type {
            Some(2) => where_sql.push_str(" AND comment_rank >= 4"),
            Some(3) => where_sql.push_str(" AND comment_rank = 3"),
            Some(4) => where_sql.push_str(" AND comment_rank <= 2"),
            _ => {}
        }

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM comment {where_sql}"));
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&count_sql), &binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let (limit, offset) = page.limit_offset();
        // Java：`buildSortOrder(page, "isTop", "desc")` + wrapper `orderByDesc(addTime)`
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM comment {where_sql} ORDER BY is_top DESC, add_time DESC LIMIT ? OFFSET ?"
        ));
        let mut q = bind_all!(sqlx::query_as::<_, Comment>(&sql), &binds);
        q = q.bind(limit).bind(offset);
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 子评论（`parent_id IN (...)`）。
    pub async fn find_comment_replies(&self, parent_ids: &[i32]) -> Result<Vec<Comment>> {
        if parent_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM comment WHERE parent_id IN ({})",
            placeholders(parent_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, Comment>(&sql);
        for id in parent_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量取订单商品（补 `skuId` / `skuData`）。
    pub async fn find_order_items(&self, item_ids: &[i32]) -> Result<Vec<OrderItem>> {
        if item_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM \"order_item\" WHERE item_id IN ({})",
            placeholders(item_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, OrderItem>(&sql);
        for id in item_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 配置项 —————————————————————

    /// 按 `biz_code` 取未删除配置值（Java `SettingsHelper#getRaw`）。
    pub async fn find_config_val(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1")
                .bind(biz_code)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }

    /// 批量取配置值（键为 `biz_code`，仅包含命中的行）。
    pub async fn find_config_vals(
        &self,
        biz_codes: &[&str],
    ) -> Result<HashMap<String, Option<String>>> {
        if biz_codes.is_empty() {
            return Ok(HashMap::new());
        }
        let sql = format!(
            "SELECT biz_code, biz_val FROM config WHERE is_del = 0 AND biz_code IN ({})",
            placeholders(biz_codes.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, (String, Option<String>)>(&sql);
        for code in biz_codes {
            q = q.bind(*code);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().collect())
    }

    // ————————————————————— 装修 —————————————————————

    /// 店铺离散装修组件（`decorate_sn` + `shop_id` 唯一）。
    pub async fn find_discrete(
        &self,
        shop_id: i32,
        decorate_sn: &str,
    ) -> Result<Option<DecorateDiscrete>> {
        sqlx::query_as::<_, DecorateDiscrete>(
            "SELECT * FROM decorate_discrete WHERE shop_id = $1 AND decorate_sn = $2 LIMIT 1",
        )
        .bind(shop_id)
        .bind(decorate_sn)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按 ID 取装修页（`previewId` 预览态）。
    pub async fn find_decorate(&self, decorate_id: i32) -> Result<Option<Decorate>> {
        sqlx::query_as::<_, Decorate>("SELECT * FROM decorate WHERE decorate_id = $1 LIMIT 1")
            .bind(decorate_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 店铺首页装修（`shop_id` + `is_home = 1` + `status = 1` + `decorate_type = 1`）。
    pub async fn find_shop_home_decorate(&self, shop_id: i32) -> Result<Option<Decorate>> {
        sqlx::query_as::<_, Decorate>(
            "SELECT * FROM decorate WHERE shop_id = $1 AND is_home = 1 AND status = 1 \
             AND decorate_type = 1 LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按 locale_code 取语言（`X-Locale-Code` 头）。
    pub async fn find_locale_by_code(&self, locale_code: &str) -> Result<Option<Locales>> {
        sqlx::query_as::<_, Locales>("SELECT * FROM locales WHERE locale_code = $1 LIMIT 1")
            .bind(locale_code)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 取某装修页在指定语言下的子装修页。
    pub async fn find_decorate_child(
        &self,
        parent_id: i32,
        locale_id: i32,
    ) -> Result<Option<Decorate>> {
        sqlx::query_as::<_, Decorate>(
            "SELECT * FROM decorate WHERE parent_id = $1 AND locale_id = $2 LIMIT 1",
        )
        .bind(parent_id)
        .bind(locale_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— 商户 —————————————————————

    /// 按 ID 取商户。
    pub async fn find_merchant(&self, merchant_id: i32) -> Result<Option<Merchant>> {
        sqlx::query_as::<_, Merchant>("SELECT * FROM merchant WHERE merchant_id = $1 LIMIT 1")
            .bind(merchant_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 取商户入驻申请状态（`getApprovedLicensePic` 的审核通过校验）。
    pub async fn find_merchant_apply_status(&self, apply_id: i32) -> Result<Option<i32>> {
        let row: Option<(Option<i32>,)> =
            sqlx::query_as("SELECT status FROM merchant_apply WHERE merchant_apply_id = $1 LIMIT 1")
                .bind(apply_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }

    // ————————————————————— 地区 —————————————————————

    /// 地区 ID → 名称（`getRegionNamesByRegionIds`，按 `region_id` 升序稳定输出）。
    pub async fn find_region_names(&self, ids: &[i32]) -> Result<Vec<(i32, String)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT region_id, region_name FROM region WHERE region_id IN ({}) ORDER BY region_id ASC",
            placeholders(ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, (i32, String)>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— O2O 门店商品 / 标签 / 配送配置 —————————————————————

    /// 平台分配记录：按商品取门店列表（`store_product`）。
    pub async fn find_store_products_by_product(&self, product_id: i32) -> Result<Vec<StoreProduct>> {
        sqlx::query_as::<_, StoreProduct>(
            "SELECT * FROM store_product WHERE product_id = $1 AND is_delete = 0 \
             AND product_status = 1",
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 某门店的平台分配商品数（上架未删）。
    pub async fn count_store_products_of_shop(&self, shop_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM store_product WHERE shop_id = $1 AND product_status = 1 \
             AND is_delete = 0",
        )
        .bind(i64::from(shop_id))
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 某门店的平台分配商品（上架未删）。
    pub async fn find_store_products_of_shop(&self, shop_id: i32) -> Result<Vec<StoreProduct>> {
        sqlx::query_as::<_, StoreProduct>(
            "SELECT * FROM store_product WHERE shop_id = $1 AND product_status = 1 AND is_delete = 0",
        )
        .bind(i64::from(shop_id))
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 门店标签关联（按 \"sort\" 升序，全局排序后由调用方分组）。
    pub async fn find_store_tips(&self, shop_ids: &[i32]) -> Result<Vec<StoreTip>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM store_tip WHERE shop_id IN ({}) ORDER BY sort ASC",
            placeholders(shop_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, StoreTip>(&sql);
        for id in shop_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 某门店的标签关联（按 `sort` 升序）。
    pub async fn find_shop_store_tips(&self, shop_id: i32) -> Result<Vec<StoreTip>> {
        sqlx::query_as::<_, StoreTip>(
            "SELECT * FROM store_tip WHERE shop_id = $1 ORDER BY sort ASC",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按 ID 批量取标签。
    pub async fn find_tips(&self, tip_ids: &[i32]) -> Result<Vec<Tip>> {
        if tip_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM tip WHERE tip_id IN ({}) ORDER BY tip_id ASC",
            placeholders(tip_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, Tip>(&sql);
        for id in tip_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 某标签（自营门店标签名回填）。
    pub async fn find_tip(&self, tip_id: i32) -> Result<Option<Tip>> {
        sqlx::query_as::<_, Tip>("SELECT * FROM tip WHERE tip_id = $1 LIMIT 1")
            .bind(tip_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 门店同城配送配置（批量）。
    pub async fn find_local_delivery_configs(
        &self,
        shop_ids: &[i32],
    ) -> Result<Vec<ShopLocalDeliveryConfig>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM shop_local_delivery_config WHERE shop_id IN ({})",
            placeholders(shop_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, ShopLocalDeliveryConfig>(&sql);
        for id in shop_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 门店同城配送配置（单店）。
    pub async fn find_local_delivery_config(
        &self,
        shop_id: i32,
    ) -> Result<Option<ShopLocalDeliveryConfig>> {
        sqlx::query_as::<_, ShopLocalDeliveryConfig>(
            "SELECT * FROM shop_local_delivery_config WHERE shop_id = $1 LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 门店预约上门配置（批量）。
    pub async fn find_home_service_configs(
        &self,
        shop_ids: &[i32],
    ) -> Result<Vec<ShopHomeServiceConfig>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM shop_home_service_config WHERE shop_id IN ({})",
            placeholders(shop_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, ShopHomeServiceConfig>(&sql);
        for id in shop_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 门店预约上门配置（单店）。
    pub async fn find_home_service_config(
        &self,
        shop_id: i32,
    ) -> Result<Option<ShopHomeServiceConfig>> {
        sqlx::query_as::<_, ShopHomeServiceConfig>(
            "SELECT * FROM shop_home_service_config WHERE shop_id = $1 LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 有「有效店铺券」的店铺集合（`promotion`：type=2 且可用未删未过期）。
    pub async fn find_coupon_shop_ids(&self, shop_ids: &[i32], now: i64) -> Result<Vec<i32>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT DISTINCT relation_id FROM promotion \
             WHERE type = 2 AND is_available = 1 AND is_delete = 0 AND end_time > ? \
               AND relation_id IN ({})",
            placeholders(shop_ids.len())
        ));
        let mut q = sqlx::query_as::<_, (i32,)>(&sql).bind(now);
        for id in shop_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    // ————————————————————— 距离 / 销量 —————————————————————

    /// 一批门店到用户位置的距离（米），复刻 Java `GpsDistanceUtil#getDistance`
    /// （半径 6378137m 的 haversine 公式，SQL 内一次算完，避免逐行 N+1）。
    ///
    /// 返回 `shop_id -> 距离`；门店无经纬度时该行不出现在结果中，
    /// 由调用方按 Java 语义取 `0`。
    pub async fn gps_distances_for_shops(
        &self,
        lng: Decimal,
        lat: Decimal,
        shop_ids: &[i32],
    ) -> Result<HashMap<i32, Decimal>> {
        if shop_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT shop_id, CAST(6378137 * 2 * asin(sqrt( \
                 pow(sin((radians(?) - radians(shop_latitude)) / 2), 2) \
                 + cos(radians(?)) * cos(radians(shop_latitude)) \
                   * pow(sin((radians(?) - radians(shop_longitude)) / 2), 2))) \
             AS DECIMAL(20,6)) AS distance \
             FROM shop WHERE shop_id IN ({})",
            placeholders(shop_ids.len())
        ));
        let mut q = sqlx::query_as::<_, (i32, Option<Decimal>)>(&sql)
            .bind(lat)
            .bind(lat)
            .bind(lng);
        for id in shop_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows
            .into_iter()
            .filter_map(|(id, d)| d.map(|d| (id, d)))
            .collect())
    }

    /// 店铺销量（`OrderItemMapper#getShopSalesByShopId`）。
    pub async fn shop_sales_by_shop_id(&self, shop_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT CAST(COALESCE(SUM(oi.quantity), 0) AS BIGINT) FROM \"order_item\" oi \
             LEFT JOIN \"order\" o ON o.order_id = oi.order_id \
             WHERE oi.shop_id = $1 AND o.is_valid = 1 AND o.order_status != 3 AND o.is_del = 0",
        )
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    // ————————————————————— 商品（单条） / 选中门店 —————————————————————

    /// 按主键取商品（`storeList` 的 `productId` 分支）。
    pub async fn find_product(&self, product_id: i32) -> Result<Option<Product>> {
        sqlx::query_as::<_, Product>("SELECT * FROM product WHERE product_id = $1 LIMIT 1")
            .bind(product_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 选中门店置顶查询（`shop_type = 2` 且 `status IN (1, 10)`）。
    pub async fn find_selected_store(&self, shop_id: i32) -> Result<Option<Shop>> {
        sqlx::query_as::<_, Shop>(
            "SELECT * FROM shop WHERE shop_id = $1 AND shop_type = 2 \
             AND (status = 1 OR status = 10) LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— 自提点（pickupListByLocation） —————————————————————

    /// 自提点分页（`shop_type = 3`、`status = 1`、`store_parent_id = shopId`）。
    ///
    /// \"location\" 为 `Some((longitude, latitude))` 时按距离升序，否则按 \"shop_id\" 升序。
    pub async fn paginate_pickups(
        &self,
        parent_shop_id: i32,
        region_id: Option<&str>,
        keyword: Option<&str>,
        location: Option<(Decimal, Decimal)>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<PickupRow>)> {
        let mut where_sql = String::from("WHERE shop_type = 3 AND status = 1 AND store_parent_id = ?");
        let mut where_binds: Vec<Bind> = vec![Bind::I32(parent_shop_id)];
        if let Some(region) = region_id.filter(|s| !s.is_empty()) {
            where_sql.push_str(" AND shop_region_id ILIKE CONCAT('%', ?, '%')");
            where_binds.push(Bind::Str(region.to_string()));
        }
        if let Some(kw) = keyword.filter(|s| !s.is_empty()) {
            where_sql.push_str(" AND shop_detailed_address ILIKE CONCAT('%', ?, '%')");
            where_binds.push(Bind::Str(kw.to_string()));
        }

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM shop {where_sql}"));
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&count_sql), &where_binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let (lat, lng) = match location {
            Some((lng, lat)) => (Some(lat), Some(lng)),
            None => (None, None),
        };
        // SELECT 内距离 CASE 的占位符：lat, lng, lat, lng, lat
        // （Java 只把纬度归零成 null，经度保留 0，故未传纬度时 CASE 走 \"shop_id\" 分支）
        let mut binds: Vec<Bind> = vec![
            opt_dec(lat),
            Bind::Dec(lng.unwrap_or_default()),
            opt_dec(lat),
            Bind::Dec(lng.unwrap_or_default()),
            opt_dec(lat),
        ];
        binds.extend(where_binds);
        // ORDER BY 内 CASE 的占位符：lat, lng
        binds.push(opt_dec(lat));
        binds.push(Bind::Dec(lng.unwrap_or_default()));

        let (limit, offset) = page.limit_offset();
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT shop_id, shop_title, \
             CASE WHEN ? IS NULL OR ? IS NULL THEN CAST(0 AS DECIMAL(20,6)) \
                  ELSE CAST(6371000 * acos(cos(radians(?)) * cos(radians(shop_latitude)) \
                       * cos(radians(shop_longitude) - radians(?)) \
                       + sin(radians(?)) * sin(radians(shop_latitude))) AS DECIMAL(20,6)) \
             END AS distance, \
             shop_longitude AS longitude, shop_latitude AS latitude, \
             shop_contact_json AS contact_json, shop_open_close_json AS open_close_json, \
             shop_region_id AS region_ids_json, shop_detailed_address AS address \
             FROM shop {where_sql} \
             ORDER BY CASE WHEN ? IS NULL OR ? IS NULL THEN shop_id ELSE 0 END ASC, distance ASC \
             LIMIT ? OFFSET ?"
        ));
        let mut q = bind_all!(sqlx::query_as::<_, PickupRow>(&sql), &binds);
        q = q.bind(limit).bind(offset);
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }
}

/// `Option<Decimal>` → `Bind`（`None` → `NULL`，供 `? IS NULL` 判定）。
fn opt_dec(v: Option<Decimal>) -> Bind {
    match v {
        Some(v) => Bind::Dec(v),
        None => Bind::Null,
    }
}

/// 排序白名单：属性名/列名 → 列名；未知回落主键，方向非 `asc` 即 \"desc\"。
/// 对齐原 `BaseServiceImpl#buildSortOrder` + `getColumnByPropertyName`。
fn order_by(
    field: Option<&str>,
    order: Option<&str>,
    key: &str,
    pairs: &[(&str, &str)],
) -> String {
    let f = field.unwrap_or("").trim();
    let col = pairs
        .iter()
        .find(|(p, c)| p.eq_ignore_ascii_case(f) || c.eq_ignore_ascii_case(f))
        .map(|(_, c)| *c)
        .unwrap_or(key);
    let dir = if order
        .map(|o| o.eq_ignore_ascii_case("asc"))
        .unwrap_or(false)
    {
        "ASC"
    } else {
        "DESC"
    };
    format!("ORDER BY \"{col}\" {dir}")
}

/// `?,?,?` 占位符（长度由调用方控制，恒为合法 SQL 片段）。
fn placeholders(n: usize) -> String {
    std::iter::repeat_n("?", n).collect::<Vec<_>>().join(",")
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("店铺数据不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_by_whitelist_falls_back_to_key() {
        assert_eq!(
            order_by(Some("shopStar"), Some("asc"), "shop_id", SHOP_SORTABLE),
            "ORDER BY \"shop_star\" ASC"
        );
        assert_eq!(
            order_by(Some("evil; DROP TABLE"), Some("desc"), "shop_id", SHOP_SORTABLE),
            "ORDER BY \"shop_id\" DESC"
        );
        // 缺省排序方向为 DESC（Java `buildSortOrder` 的 otherwise 分支）
        assert_eq!(order_by(None, None, "shop_id", SHOP_SORTABLE), "ORDER BY \"shop_id\" DESC");
    }

    #[test]
    fn placeholders_count() {
        assert_eq!(placeholders(1), "?");
        assert_eq!(placeholders(3), "?,?,?");
    }
}