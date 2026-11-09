//! 代客下单订单结算域仓储（trade::proxy_shopping_order_check）。
//!
//! 对应原 Java：
//! - `ShopMapper.pickupListByLocation`（自提点分页，`ShopMapper.xml:162-198`）
//! - `ConfigServiceImpl#getConfigVal` / `#getConfigByCode`（只读）
//! - `PaymentServiceImpl#getAvailablePayment`（只读配置开关，见服务层说明）
//! - `RegionService#getRegionNamesByRegionIds`（`region` 表只读）
//!
//! 列名严格对齐权威 schema（`cd rust && python tools/schema_parser.py --table shop`
//! / `--table config` / `--table region`，权威源 `java/database/pure.sql`）。
//!
//! 约定：
//! - 一律使用**运行时** `sqlx::query_as` / `sqlx::query`，**不使用** `query!` 宏
//!   （本机无 `DATABASE_URL`，宏必然编译失败）。
//! - 每条 SQL 都是**单行字符串字面量**（`\` 续行仍在同一 Rust 行内），便于
//!   `python tools/check_sql_columns.py` 静态校验列名。
//! - 仓储层**不做业务校验**（Java Mapper 也不做），只忠实执行 SQL；
//!   「没有此门店」「配置缺失」等语义在服务层复刻。
//!
//! # 与 merchant::shop_client 的关系
//!
//! merchant::shop_client 已有一段**同名同源**的距离排序查询（`ShopMapper.storeListByLocation`，
//! 面向 `shop_type = 2` 的门店列表）。本切片面向 `shop_type = 3` 的**自提点**
//! （`pickupListByLocation`），过滤条件与投影字段不同（自提点带
//! `store_parent_id = query.shopId`），且 merchant::shop_client 的文件属其他实现者的独占文件，
//! 本切片不得修改，故在此独立实现。

use rust_decimal::Decimal;
use sqlx::PgPool;
use nextrade_core::{Error, Result};

/// 距离表达式（球面余弦定理，半径 6371000m），与
/// `ShopMapper.xml:168-174` 逐字一致。
///
/// 等价 Java：
/// ```text
/// CASE WHEN #{query.latitude} IS NULL OR #{query.longitude} IS NULL THEN 0
///      ELSE (6371000 * acos(cos(radians(#{query.latitude}))
///                * cos(radians(shop_latitude))
///                * cos(radians(shop_longitude) - radians(#{query.longitude}))
///                + sin(radians(#{query.latitude})) * sin(radians(shop_latitude))))
/// END
/// ```
/// 占位符顺序：`latitude`（IS NULL）、`longitude`（IS NULL）、
/// `latitude`、`longitude`、`latitude`。
///
/// `CAST(... AS DECIMAL(20,6))` 与原 Java 的 `Double` 返回不同：本实现保留
/// `Decimal`（项目硬约束禁用 f64/f32），精度取 6 位小数（与 `shop_latitude`
/// / `shop_longitude` 的 `DECIMAL(10,6)` 同精度）。
const DISTANCE_EXPR: &str = "CASE WHEN ? IS NULL OR ? IS NULL THEN CAST(0 AS DECIMAL(20,6)) \
     ELSE CAST(6371000 * acos(cos(radians(?)) * cos(radians(shop_latitude)) \
          * cos(radians(shop_longitude) - radians(?)) \
          + sin(radians(?)) * sin(radians(shop_latitude))) AS DECIMAL(20,6)) END";

/// `shop` 表的筛选条件（对应 `ShopMapper.xml:183-191` 的固定条件 + 三个 `<if>`）。
///
/// 与 Java 的差异：
/// - Java 的 `<if test="query.regionId != null and query.regionId != ''">` 在
///   MyBatis 里对 `"0"` 也生效（非空串），本实现同样只判「非空」；
/// - `is_delete = 0` 是 MyBatis-Plus `@TableLogic` 自动追加的条件
///   （\"shop\" 实体带逻辑删除），Java XML 里看不到，必须显式补上，
///   否则会把已删除门店当成自提点返回。
const PICKUP_WHERE: &str = "shop_type = 3 AND status = 1 AND is_delete = 0";

/// 自提点行投影（对应 \"PickupListVO\" 的 SQL 列别名，`ShopMapper.xml:163-181`）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct PickupRow {
    pub shop_id: i32,
    pub shop_title: Option<String>,
    pub distance: Option<Decimal>,
    pub longitude: Option<Decimal>,
    pub latitude: Option<Decimal>,
    /// \"shop_contact_json\" 原文（Java `@JsonIgnore`，仅用于反序列化为 \"contactConfigs\"）
    pub contact_json: Option<String>,
    /// \"shop_open_close_json\" 原文（Java `@JsonIgnore`）
    pub open_close_json: Option<String>,
    /// \"shop_region_id\" 原文（Java `@JsonIgnore`，形如 `[1,2,3]`）
    pub region_ids_json: Option<String>,
    pub address: Option<String>,
}

/// 地区名行投影（对应 `RegionService#getRegionNamesByRegionIds`）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RegionNameRow {
    pub region_id: i32,
    pub region_name: String,
}

/// 自提点查询参数（服务层已按 Java \"PickupPageQuery\" 语义归一化）。
#[derive(Debug, Clone, Default)]
pub struct PickupQuery {
    /// `shop.store_parent_id`（Java `query.shopId`，必填才有结果）
    pub parent_shop_id: Option<i32>,
    /// 地区 ID（`STORE_SHOW_OTHER_CITY_STORE = 1` 时已置 \"None\"）
    pub region_id: Option<String>,
    /// 关键字（\"shop_detailed_address\" 模糊）
    pub keyword: Option<String>,
    /// 纬度（为 \"None\" 时距离恒 0 且按 \"shop_id\" 升序，同 Java）
    pub latitude: Option<Decimal>,
    /// 经度
    pub longitude: Option<Decimal>,
    /// \"LIMIT\"
    pub limit: u64,
    /// \"OFFSET\"
    pub offset: u64,
}

#[derive(Clone)]
pub struct ProxyShoppingOrderCheckRepository {
    pool: PgPool,
}

impl ProxyShoppingOrderCheckRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— config（只读） —————————————————————

    /// `ConfigServiceImpl#getConfigVal(SettingsEnum)`：
    /// 按 \"biz_code\" 取 \"biz_val\"，**缺失返回 \"None\"**（Java 返回 `""`，
    /// 由服务层用 `unwrap_or_default()` 还原空串语义）。
    pub async fn find_config_val(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as("SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1")
            .bind(biz_code)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }

    /// 批量读取多个 `biz_code` 的 `biz_val`（对应 Java 多次 `getConfigVal`；
    /// 本实现合并为一次查询，语义等价 —— `bizCode` 唯一且过滤 `is_del = 0`）。
    ///
    /// 返回 `biz_code -> biz_val`（缺失的键不在 map 里，服务层按空串处理）。
    pub async fn find_config_vals(
        &self,
        biz_codes: &[&str],
    ) -> Result<std::collections::HashMap<String, String>> {
        let mut out = std::collections::HashMap::new();
        if biz_codes.is_empty() {
            return Ok(out);
        }
        let placeholders = std::iter::repeat_n("?", biz_codes.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT biz_code, biz_val FROM config WHERE is_del = 0 AND biz_code IN ({placeholders})"));
        let mut q = sqlx::query_as::<_, (String, Option<String>)>(&sql);
        for code in biz_codes {
            q = q.bind(*code);
        }
        for (code, val) in q.fetch_all(&self.pool).await.map_err(map_err)? {
            // 与 Java \"getConfigVal\" 的 `one()` 一致：bizCode 唯一；重复时保留首个
            out.entry(code).or_insert_with(|| val.unwrap_or_default());
        }
        Ok(out)
    }

    // ————————————————————— 自提点 —————————————————————

    /// `ShopServiceImpl#pickupList` 的前置校验：
    /// `getById(shopId) == null && shopId > 0` → 抛「没有此门店」。
    ///
    /// 返回该门店是否存在（`is_delete = 0`，同 MyBatis-Plus 逻辑删除语义）。
    pub async fn shop_exists(&self, shop_id: i32) -> Result<bool> {
        let row: Option<(i64,)> = sqlx::query_as("SELECT COUNT(*) FROM shop WHERE shop_id = $1 AND is_delete = 0")
            .bind(shop_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.map(|(n,)| n > 0).unwrap_or(false))
    }

    /// 自提点总数（\"LIMIT\" 之外的分页总数）。
    pub async fn count_pickups(&self, q: &PickupQuery) -> Result<u64> {
        let mut sql = String::from("SELECT COUNT(*) FROM shop WHERE ");
        sql.push_str(PICKUP_WHERE);
        if q.parent_shop_id.is_some() {
            sql.push_str(" AND store_parent_id = ?");
        }
        if q.region_id.is_some() {
            sql.push_str(" AND shop_region_id LIKE CONCAT('%', ?, '%')");
        }
        if q.keyword.is_some() {
            sql.push_str(" AND shop_detailed_address LIKE CONCAT('%', ?, '%')");
        }
        let mut query = sqlx::query_as::<_, (i64,)>(&sql);
        if let Some(v) = q.parent_shop_id {
            query = query.bind(v);
        }
        if let Some(v) = q.region_id.as_deref() {
            query = query.bind(v);
        }
        if let Some(v) = q.keyword.as_deref() {
            query = query.bind(v);
        }
        let row = query.fetch_optional(&self.pool).await.map_err(map_err)?;
        Ok(row.map(|(n,)| n.max(0) as u64).unwrap_or(0))
    }

    /// `ShopMapper.pickupListByLocation`：按距离升序返回自提点分页。
    ///
    /// 与 Java XML 一致：
    /// - 固定条件 `shop_type = 3 AND status = 1 AND store_parent_id = #{query.shopId}`；
    /// - \"regionId\" / \"keyword\" 走 `<if>` 动态拼接（`LIKE CONCAT('%', ?, '%')`）；
    /// - 排序 `CASE WHEN 经纬度为空 THEN shop_id ELSE distance END ASC`。
    ///
    /// 经纬度为 \"None\" 时以 \"NULL\" 绑定，SQL 内 `? IS NULL` 判定成立 → 距离 0、
    /// 按 \"shop_id\" 升序（与原 Java 完全一致）。
    pub async fn pickup_list(&self, q: &PickupQuery) -> Result<Vec<PickupRow>> {
        let mut sql = String::from("SELECT shop_id, shop_title, ");
        sql.push_str(DISTANCE_EXPR);
        sql.push_str(
            " AS distance, shop_longitude AS longitude, shop_latitude AS latitude, \
             shop_contact_json AS contact_json, shop_open_close_json AS open_close_json, \
             shop_region_id AS region_ids_json, shop_detailed_address AS address FROM shop WHERE ",
        );
        sql.push_str(PICKUP_WHERE);
        if q.parent_shop_id.is_some() {
            sql.push_str(" AND store_parent_id = ?");
        }
        if q.region_id.is_some() {
            sql.push_str(" AND shop_region_id LIKE CONCAT('%', ?, '%')");
        }
        if q.keyword.is_some() {
            sql.push_str(" AND shop_detailed_address LIKE CONCAT('%', ?, '%')");
        }
        sql.push_str(" ORDER BY CASE WHEN ? IS NULL OR ? IS NULL THEN shop_id ELSE distance END ASC LIMIT ? OFFSET ?");

        let mut query = sqlx::query_as::<_, PickupRow>(&sql);
        // 距离表达式占位符
        query = query.bind(q.latitude).bind(q.longitude);
        query = query.bind(q.latitude).bind(q.longitude).bind(q.latitude);
        // WHERE 动态条件
        if let Some(v) = q.parent_shop_id {
            query = query.bind(v);
        }
        if let Some(v) = q.region_id.as_deref() {
            query = query.bind(v);
        }
        if let Some(v) = q.keyword.as_deref() {
            query = query.bind(v);
        }
        // ORDER BY 内的 `? IS NULL` 判定
        query = query.bind(q.latitude).bind(q.longitude);
        query = query
            .bind(i64::try_from(q.limit).unwrap_or(i64::MAX))
            .bind(i64::try_from(q.offset).unwrap_or(i64::MAX));
        query.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// `RegionService#getRegionNamesByRegionIds`：按地区 ID 取名称（只读）。
    ///
    /// 空 \"ids\" 直接返回空（不发出无 \"IN\" 列表的非法 SQL）。
    pub async fn find_region_names(&self, ids: &[i32]) -> Result<Vec<RegionNameRow>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT region_id, region_name FROM region WHERE region_id IN ({placeholders})"));
        let mut query = sqlx::query_as::<_, RegionNameRow>(&sql);
        for id in ids {
            query = query.bind(*id);
        }
        query.fetch_all(&self.pool).await.map_err(map_err)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("门店不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_expr_has_five_placeholders() {
        // 距离 CASE 的占位符顺序（lat 判空, lng 判空, lat, lng, lat）必须与
        // `pickup_list` 的绑定顺序一致；数量对不上会在运行时报参数个数错误。
        assert_eq!(DISTANCE_EXPR.matches('?').count(), 5);
        assert!(DISTANCE_EXPR.contains("6371000"));
        assert!(DISTANCE_EXPR.contains("AS DECIMAL(20,6)"));
    }

    #[test]
    fn distance_expr_matches_java_case_shape() {
        // Java：经纬度任一为 null → 0，否则球面余弦距离。
        assert!(DISTANCE_EXPR.starts_with("CASE WHEN ? IS NULL OR ? IS NULL THEN CAST(0 AS DECIMAL(20,6))"));
        assert!(DISTANCE_EXPR.ends_with("END"));
    }

    #[test]
    fn pickup_where_keeps_java_filters_and_logical_delete() {
        // `shop_type = 3` 自提点；`status = 1` 启用；`is_delete = 0` 是
        // MyBatis-Plus `@TableLogic` 的隐式条件（XML 里看不到，必须显式补）。
        assert!(PICKUP_WHERE.contains("shop_type = 3"));
        assert!(PICKUP_WHERE.contains("status = 1"));
        assert!(PICKUP_WHERE.contains("is_delete = 0"));
    }
}