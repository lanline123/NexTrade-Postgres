//! 积分兑换域（catalog::exchange）仓储。
//!
//! 对应原 Java：
//! - `com.nextrade.mapper.promotion.PointsExchangeMapper`
//!   （`BaseMapper<PointsExchange>`）
//! - `com.nextrade.service.promotion.impl.PointsExchangeServiceImpl#list` /
//!   `#detail`（只迁移客户端读路径）
//! - `com.nextrade.service.common.impl.BaseServiceImpl#buildSortOrder`
//! - `com.nextrade.service.cart.impl.CartServiceImpl#clearCart`（服务层
//!   只借本仓的「清空兑换类购物车」语句）
//!
//! # 表 / 列（权威来源）
//!
//! - `points_exchange`（7 列，主键 `id`）：
//!   `id` / `product_id` / `exchange_integral` / `points_deducted_amount` /
//!   `is_hot` / `is_enabled` / `sku_id`
//! - 列表/详情回填商品与 SKU 价时只读：
//!   `product`（`product_id` / `product_name` / `pic_thumb` / `pic_url` /
//!   `market_price` / `product_price` / `product_stock` / `virtual_sales` /
//!   `product_sn` / `product_status` / `is_delete`）与
//!   `product_sku`（`sku_id` / `sku_price` / `is_enabled`）
//! - 清空兑换购物车时写 `cart`（`user_id` / `type`）与
//!   `cart_global_change_price`（`user_id`）
//!
//! 用 `cd rust && python tools/schema_parser.py --table <表名>` 核对；
//! 查询统一使用运行时 `sqlx::query_as`（**不使用** `query!` 宏）。
//!
//! # 金额
//!
//! `points_deducted_amount`、`market_price`、`product_price`、`sku_price`
//! 均为 `rust_decimal::Decimal`（对应 Java `BigDecimal`）。

use rust_decimal::Decimal;
use sqlx::{FromRow,PgPool};

use nextrade_core::entities::PointsExchange;
use nextrade_core::models::catalog::exchange::CART_TYPE_EXCHANGE;
use nextrade_core::models::catalog::services::to_snake;
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// `points_exchange` 的固定 SELECT 列清单（7 列，顺序与
/// `entities::PointsExchange` 字段一致）。
const POINTS_EXCHANGE_SELECT: &str = "SELECT id, product_id, exchange_integral, points_deducted_amount, is_hot, is_enabled, sku_id FROM points_exchange";

/// 列表 JOIN 片段（商品必须存在、上架且未删；SKU 左连用于价/启用过滤）。
const POINTS_EXCHANGE_FROM: &str = "FROM points_exchange pe INNER JOIN product p ON p.product_id = pe.product_id AND p.product_status = 1 AND p.is_delete = 0 LEFT JOIN product_sku s ON s.sku_id = pe.sku_id";

/// 列表行 SELECT（活动 7 列 + 商品平铺字段 + SKU 价与「SKU 行是否存在」标记）。
///
/// `s.sku_id AS sku_row_id` 用于区分「SKU 行不存在」与「SKU 存在但价格为
/// NULL」（Java 用 `productSkuMap.get(skuId)` 是否命中区分）。
const POINTS_EXCHANGE_ROW_SELECT: &str = "SELECT pe.id, pe.product_id, pe.exchange_integral, pe.points_deducted_amount, pe.is_hot, pe.is_enabled, pe.sku_id, p.product_name, p.pic_thumb, p.pic_url, p.market_price, p.product_price, p.product_stock, p.virtual_sales, p.product_sn, s.sku_id AS sku_row_id, s.sku_price";

/// 排序字段白名单（`points_exchange` 的 7 个列名，SQL 中统一加 `pe.` 前缀
/// 避免与 JOIN 表同名列歧义；对应 Java `BaseServiceImpl#getColumnByPropertyName`
/// 的实体验证口径）。
const SORTABLE_COLUMNS: [&str; 7] = [
    "id",
    "product_id",
    "exchange_integral",
    "points_deducted_amount",
    "is_hot",
    "is_enabled",
    "sku_id",
];

/// 一行「活动 + 商品 + SKU 价」连表结果（对应 Java
/// `PointsExchangeServiceImpl#list` 里 `productMap` / `productSkuMap` 的回填）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct PointsExchangeRow {
    /// 积分活动 ID
    pub id: i32,
    /// 商品 ID
    pub product_id: i32,
    /// 所需积分
    pub exchange_integral: i32,
    /// 积分抵扣金额
    pub points_deducted_amount: Decimal,
    /// 是否热门
    pub is_hot: i32,
    /// 是否生效
    pub is_enabled: i32,
    /// 规格 ID（可空）
    pub sku_id: Option<i32>,
    /// 商品名称
    pub product_name: String,
    /// 商品缩略图
    pub pic_thumb: String,
    /// 商品图
    pub pic_url: String,
    /// 商品市场价
    pub market_price: Decimal,
    /// 商品价格
    pub product_price: Decimal,
    /// 商品库存
    pub product_stock: i32,
    /// 虚拟销量
    pub virtual_sales: i32,
    /// 商品货号
    pub product_sn: String,
    /// JOIN 到的 SKU ID（`None` = 无该 SKU 行；用于区分「无 SKU」与
    /// 「SKU 价为 NULL」）
    pub sku_row_id: Option<i32>,
    /// SKU 价（`None` 可能是无 SKU 行，也可能是 SKU 价为 `NULL`）
    pub sku_price: Option<Decimal>,
}

/// 列表过滤条件（`None` 表示不过滤）。
#[derive(Debug, Clone, Default)]
pub struct ExchangeFilters<'a> {
    /// 商品名称模糊（`p.product_name ILIKE ?`）
    pub keyword: Option<&'a str>,
    /// 是否热门（`pe.is_hot = ?`）
    pub is_hot: Option<i32>,
    /// 是否生效（`pe.is_enabled = ?`；为 `Some(1)` 时追加 SKU 启用条件）
    pub is_enabled: Option<i32>,
}

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct ExchangeRepository {
    pool: PgPool,
}

impl ExchangeRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 查询 —————————————————————

    /// 按主键查询积分活动
    /// （对应 Java `PointsExchangeServiceImpl#detail` 的 `this.getById(id)`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<PointsExchange>> {
        let sql = format!("{POINTS_EXCHANGE_SELECT} WHERE id = $1 LIMIT 1");
        sqlx::query_as::<_, PointsExchange>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 积分兑换分页（对应 Java `PointsExchangeServiceImpl#list`）。
    ///
    /// 过滤条件与 Java 的净效果一致：
    /// - 商品有效：`product_status = 1 AND is_delete = 0`（Java 用 innerJoin
    ///   或「先查商品 ID 再 IN」两条路径，净效果相同）
    /// - `is_enabled` / `is_hot` / `keyword`（商品名 ILIKE）
    /// - `is_enabled == Some(1)` 时追加
    ///   `pe.sku_id IS NULL OR pe.sku_id <= 0 OR s.is_enabled = 1`
    ///   （Java `leftJoin + and(...)` 的等价物）
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn paginate(
        &self,
        filters: &ExchangeFilters<'_>,
        sort_field: Option<&str>,
        sort_order: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<PointsExchangeRow>)> {
        let like = like_pattern(filters.keyword);

        let mut where_parts: Vec<String> = vec!["1 = 1".into()];
        if filters.is_enabled.is_some() {
            where_parts.push("pe.is_enabled = ?".into());
        }
        if filters.is_hot.is_some() {
            where_parts.push("pe.is_hot = ?".into());
        }
        if like.is_some() {
            where_parts.push("p.product_name ILIKE ?".into());
        }
        if filters.is_enabled == Some(1) {
            // Java：leftJoin(ProductSku) + and(pe.skuId is null or <=0 or isEnabled=YES)
            where_parts.push("(pe.sku_id IS NULL OR pe.sku_id <= 0 OR s.is_enabled = 1)".into());
        }
        let where_sql = format!("WHERE {}", where_parts.join(" AND "));
        let order_by = order_clause(sort_field, sort_order);
        let (limit, offset) = page.limit_offset();

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) {POINTS_EXCHANGE_FROM} {where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        cq = bind_filters(
            cq,
            filters.is_enabled,
            filters.is_hot,
            like.as_deref(),
        );
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let list_sql = crate::db::to_pg_placeholders(&format!(
            "{POINTS_EXCHANGE_ROW_SELECT} {POINTS_EXCHANGE_FROM} {where_sql} {order_by} LIMIT ? OFFSET ?"
        ));
        let mut lq = sqlx::query_as::<_, PointsExchangeRow>(&list_sql);
        lq = bind_filters(
            lq,
            filters.is_enabled,
            filters.is_hot,
            like.as_deref(),
        );
        let rows = lq
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    // ————————————————————— 购物车（兑换类） —————————————————————

    /// 清空某用户的**兑换类**购物车
    /// （对应 Java `CartServiceImpl#clearCart(userId, CartTypeEnum.TYPE_EXCHANGE)`）。
    ///
    /// Java 同时删除 `cart_global_change_price`（改价缓存行）；本实现一致。
    ///
    /// 返回删除的 `cart` 行数。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn clear_exchange_cart(&self, user_id: i32) -> Result<u64> {
        let res = sqlx::query("DELETE FROM cart WHERE user_id = $1 AND type = $2")
            .bind(user_id)
            .bind(CART_TYPE_EXCHANGE)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        sqlx::query("DELETE FROM cart_global_change_price WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }
}

/// ILIKE 通配模式（Java `StrUtil.isNotEmpty(keyword)` + `like`）。
///
/// Java 先判 `isNotEmpty`：`None` / 空串都不加条件。
#[must_use]
pub fn like_pattern(raw: Option<&str>) -> Option<String> {
    raw.filter(|s| !s.is_empty())
        .map(|s| format!("%{s}%"))
}

/// 排序字段白名单解析（防 SQL 注入）。
///
/// 属性名 / 列名命中 `points_exchange` 的列 → 加上 `pe.` 前缀返回；
/// 未知或缺失回退主键 `pe.id`（Java 回退 key column `id`）。
#[must_use]
pub fn resolve_sort_column(field: Option<&str>) -> &'static str {
    let Some(f) = field else {
        return "pe.id";
    };
    let snake = to_snake(f);
    match SORTABLE_COLUMNS.iter().copied().find(|c| *c == snake) {
        Some("id") => "pe.id",
        Some("product_id") => "pe.product_id",
        Some("exchange_integral") => "pe.exchange_integral",
        Some("points_deducted_amount") => "pe.points_deducted_amount",
        Some("is_hot") => "pe.is_hot",
        Some("is_enabled") => "pe.is_enabled",
        Some("sku_id") => "pe.sku_id",
        _ => "pe.id",
    }
}

/// 完整 ORDER BY 子句（Java `buildSortOrder`）：
/// 仅显式 `asc` 升序，`desc` / 空一律降序 → 默认 `ORDER BY pe.id DESC`。
#[must_use]
pub fn order_clause(sort_field: Option<&str>, sort_order: Option<&str>) -> String {
    let column = resolve_sort_column(sort_field);
    let dir = if sort_order.is_some_and(|o| o.eq_ignore_ascii_case("asc")) {
        "ASC"
    } else {
        "DESC"
    };
    format!("ORDER BY {column} {dir}")
}

/// 把 WHERE 片段按 [`ExchangeRepository::paginate`] 的拼接顺序绑定：
/// `pe.is_enabled` → `pe.is_hot` → `p.product_name`（ILIKE）。
fn bind_filters<'q, O>(
    q: sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments>,
    is_enabled: Option<i32>,
    is_hot: Option<i32>,
    like: Option<&'q str>,
) -> sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments> {
    let mut q = q;
    if let Some(v) = is_enabled {
        q = q.bind(v);
    }
    if let Some(v) = is_hot {
        q = q.bind(v);
    }
    if let Some(v) = like {
        q = q.bind(v);
    }
    q
}

/// 数据库错误映射（与同域仓储同一约定）。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("活动不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 列表行 SELECT 覆盖活动 7 列 + 商品 8 字段 + SKU 价，且不用 `SELECT *`。
    #[test]
    fn row_select_covers_expected_columns() {
        for column in [
            "pe.id",
            "pe.product_id",
            "pe.exchange_integral",
            "pe.points_deducted_amount",
            "pe.is_hot",
            "pe.is_enabled",
            "pe.sku_id",
            "p.product_name",
            "p.pic_thumb",
            "p.pic_url",
            "p.market_price",
            "p.product_price",
            "p.product_stock",
            "p.virtual_sales",
            "p.product_sn",
            "s.sku_id AS sku_row_id",
            "s.sku_price",
        ] {
            assert!(
                POINTS_EXCHANGE_ROW_SELECT.contains(column),
                "SELECT 缺少列 {column}"
            );
        }
        assert!(!POINTS_EXCHANGE_ROW_SELECT.contains('*'));
        // 活动单表 SELECT 覆盖实体 7 列
        for column in SORTABLE_COLUMNS {
            assert!(POINTS_EXCHANGE_SELECT.contains(column), "缺少列 {column}");
        }
    }

    /// 默认排序：`pe.id DESC`（Java 空 sortField → 主键、空 sortOrder → 降序）。
    #[test]
    fn default_order_is_primary_key_desc() {
        assert_eq!(order_clause(None, None), "ORDER BY pe.id DESC");
        // sortField 为空时回退主键列，但显式 sortOrder=asc 仍按升序（BaseServiceImpl 3 参分支）
        assert_eq!(order_clause(Some(""), Some("asc")), "ORDER BY pe.id ASC");
        assert_eq!(order_clause(Some("id"), None), "ORDER BY pe.id DESC");
    }

    /// 显式排序：属性名 / 列名都认（统一加 `pe.` 前缀防 JOIN 歧义）。
    #[test]
    fn explicit_order_uses_pe_prefix() {
        assert_eq!(
            order_clause(Some("exchangeIntegral"), Some("asc")),
            "ORDER BY pe.exchange_integral ASC"
        );
        assert_eq!(
            order_clause(Some("points_deducted_amount"), Some("DESC")),
            "ORDER BY pe.points_deducted_amount DESC"
        );
        assert_eq!(order_clause(Some("isHot"), None), "ORDER BY pe.is_hot DESC");
        // 未知字段回退主键，防 SQL 注入
        assert_eq!(
            order_clause(Some("1;DROP TABLE points_exchange"), None),
            "ORDER BY pe.id DESC"
        );
    }

    /// ILIKE 模式：`None` / 空串不加条件（Java `isNotEmpty`），其余包 `%`。
    #[test]
    fn like_pattern_matches_java_is_not_empty() {
        assert_eq!(like_pattern(None), None);
        assert_eq!(like_pattern(Some("")), None);
        assert_eq!(like_pattern(Some("耳机")).as_deref(), Some("%耳机%"));
    }

    /// 兑换购物车类型常量与 Java `CartTypeEnum.TYPE_EXCHANGE` 一致。
    #[test]
    fn cart_type_constant_matches_java_enum() {
        assert_eq!(CART_TYPE_EXCHANGE, 3);
    }
}
