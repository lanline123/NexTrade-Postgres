//! 库存预警域仓储（catalog::stock_warning）：对应原 Java `StockWarningRuleMapper`（含
//! `StockWarningRuleMapper.xml`）与 MyBatis-Plus 的 `StockWarningSettingMapper`。
//!
//! 列名严格对齐权威 schema（`python tools/schema_parser.py --table <表名>`）：
//! - `stock_warning_rule`：`id` / `shop_id` / `product_id` / `sku_id` /
//!   `warn_enabled` / `lower_limit` / `upper_limit` / `last_warn_status` /
//!   `last_warn_time` / `is_delete` / `create_time` / `update_time`
//! - `stock_warning_setting`：`shop_id` / `notify_enabled` / `create_time` /
//!   `update_time`（主体级开关以 `shop_id` 为主键，无自增主键）
//! - 关联表只读列：`product`（`product_id` / `product_name` / `pic_thumb` /
//!   `product_sn` / `product_tsn` / `shop_id` / `product_related` /
//!   `is_delete` / `vendor_id` / `product_type` / `product_stock`）、
//!   `product_sku`（`sku_id` / `product_id` / `sku_value` / `sku_sn` /
//!   `sku_stock`）、`store_product`（`store_product_id` / `product_id` /
//!   `shop_id` / `product_stock` / `is_delete`）、`store_product_sku`
//!   （`store_product_id` / `product_sku_id` / `sku_stock`）、
//!   `purchase_order`（`id` / `is_del` / `type` / `status` / `shop_id`）、
//!   `purchase_order_item`（`order_id` / `product_id` / `sku_id` / `quantity` /
//!   `received_quantity` / `is_del`）、`shop`（`shop_id` / `shop_title` /
//!   `shop_type`）
//!
//! 全部使用运行时 `sqlx::query_as::<_, T>(...)`（本机无 `DATABASE_URL`，
//! 不能用 `query!` 宏）。
//!
//! **unsigned 列必须 CAST**：sqlx 的 `i32` / `i64` 只与**有符号**整型列
//! 兼容（`sqlx_mysql::types::int::int_compatible` 显式排除
//! `ColumnFlags::UNSIGNED`），因此本模块对所有 unsigned 列
//! （`product.product_id` / `product.shop_id`、`product_sku.sku_id`、
//! `stock_warning_rule.id`、`*_time bigint unsigned`）以及
//! `SUM()` / `COUNT()`（MySQL 返回 DECIMAL / BIGINT）统一写
//! `CAST(... AS BIGINT)`。漏掉会编译通过、列名校验也过，但一接真库就报
//! `mismatched types`。
//!
//! **书写约定**：SQL 字面量必须写在「行尾只剩 `,` 或 `)`」的位置，
//! `tools/check_sql_columns.py` 的字符串提取逻辑才会把它当作一条独立语句
//! 逐列校验（写成 `let sql = "...";` 会因行尾 `;` 无法闭合，导致该文件
//! 之后的语句被并成一条、跳过列名校验）。

use sqlx::postgres::PgArguments;
use sqlx::query::QueryAs;
use sqlx::{FromRow,Postgres,PgPool};
use nextrade_core::entities::{StockWarningRule, StockWarningSetting};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// 未配置规则的商品/SKU 在列表里的库存状态码（Java `stockStatusExpr` 的 9）。
const STOCK_STATUS_UNCONFIGURED: i32 = 9;

/// 列表 / 导出的查询条件（由服务层归一化后传入；仓储层只拼 SQL）。
#[derive(Debug, Clone, Default)]
pub struct WarningQuerySpec {
    /// 归属主体：0=平台自营，>0=店铺或门店
    pub shop_id: i32,
    /// 是否按门店独立库存口径取数（服务层按主体与配置计算）
    pub use_store_stock: bool,
    /// 门店独立库存与总库存同时启用时，可售库存取两者较小值
    pub use_total_stock: bool,
    /// 导出时勾选的行（`(product_id, sku_id)`），空表示不限定具体行
    pub targets: Vec<(i32, i32)>,
    /// 商品类型
    pub product_type: Option<i32>,
    /// 关键词的 LIKE 模式（`%kw%`）
    pub keyword_like: Option<String>,
    /// 库存状态筛选
    pub stock_status: Option<i32>,
    /// 预警开关筛选
    pub warn_enabled: Option<i32>,
    /// 排序字段（白名单外的值回退商品 ID 倒序）
    pub sort_field: Option<String>,
    /// 排序方向（仅 `asc` / `ASC` 为升序）
    pub sort_order: Option<String>,
}

/// 库存预警列表 / 导出的数据库查询结果行（对应 Java `StockWarningRowVO`）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct StockWarningRow {
    pub product_id: i32,
    pub sku_id: i32,
    pub product_name: Option<String>,
    pub product_sku_name: Option<String>,
    pub product_pic_thumb: Option<String>,
    pub product_sn: Option<String>,
    pub sku_sn: Option<String>,
    pub product_tsn: Option<String>,
    pub product_shop_id: Option<i32>,
    pub store_shop_id: Option<i64>,
    pub product_related: Option<String>,
    /// 规则主键，为空表示该商品/SKU 未配置规则
    pub rule_id: Option<i32>,
    pub warn_enabled: Option<i32>,
    pub lower_limit: Option<i32>,
    pub upper_limit: Option<i32>,
    pub update_time: Option<i64>,
    pub current_stock: Option<i32>,
    pub transit_stock: Option<i64>,
    /// 由 SQL 计算的库存状态，口径与筛选条件保持一致
    pub stock_status: Option<i32>,
}

/// 导入时按编码解析出的商品/SKU 行（对应 Java `StockWarningSnTargetVO`）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct WarningSnTarget {
    /// 命中的编码：多规格是 `product_sku.sku_sn`，单规格是 `product.product_sn`
    pub sn: String,
    pub product_id: i32,
    /// 商品编码命中时为 0
    pub sku_id: i32,
    /// 1=命中规格编码（多规格），2=命中商品编码
    pub match_type: i32,
    /// 命中商品编码时该商品的规格数，用于区分单规格与多规格
    pub sku_count: i32,
}

/// 归属主体（仓库列展示）所需的店铺精简行。
#[derive(Debug, Clone, Default, FromRow)]
pub struct ShopBrief {
    pub shop_id: i32,
    pub shop_title: Option<String>,
    pub shop_type: Option<i32>,
}

#[derive(Clone)]
pub struct StockWarningRepository {
    pool: PgPool,
}

impl StockWarningRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ————————————————————— 通知总开关 —————————————————————

    /// 按主体读取通知总开关（Java `stockWarningSettingMapper.selectById(shopId)`）。
    pub async fn find_setting(&self, shop_id: i32) -> Result<Option<StockWarningSetting>> {
        // `create_time` / `update_time` 是 `bigint unsigned`：sqlx 的 `i64::compatible`
        // 只接受**有符号**整型列，unsigned 列必须显式 CAST，否则运行时报
        // `mismatched types`（列名校验脚本查不出这类问题）。
        sqlx::query_as::<_, StockWarningSetting>(
            "SELECT shop_id, notify_enabled, CAST(create_time AS BIGINT) AS create_time, CAST(update_time AS BIGINT) AS update_time FROM stock_warning_setting WHERE shop_id = $1 LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 新增通知总开关（`shop_id` 为主键，非自增）。
    pub async fn insert_setting(&self, setting: &StockWarningSetting) -> Result<()> {
        sqlx::query(
            "INSERT INTO stock_warning_setting (shop_id, notify_enabled, create_time, update_time) VALUES ($1, $2, $3, $4)",
        )
        .bind(setting.shop_id)
        .bind(setting.notify_enabled)
        .bind(setting.create_time)
        .bind(setting.update_time)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 更新通知总开关（Java `updateById`：只写开关与更新时间）。
    pub async fn update_setting(&self, shop_id: i32, notify_enabled: i32, now: i64) -> Result<()> {
        sqlx::query(
            "UPDATE stock_warning_setting SET notify_enabled = $1, update_time = $2 WHERE shop_id = $3",
        )
        .bind(notify_enabled)
        .bind(now)
        .bind(shop_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    // ————————————————————— 预警规则 —————————————————————
    //
    // Java `getRule` 走 MyBatis-Plus 的 `@TableLogic`，查询自动附加
    // `is_delete = 0`；`updateById` 同样附加该条件，且不会写 `is_delete`。

    /// 按主体 + 商品 + SKU 读取规则（Java `getRule`）。
    pub async fn find_rule(
        &self,
        shop_id: i32,
        product_id: i32,
        sku_id: i32,
    ) -> Result<Option<StockWarningRule>> {
        // `id` 是 `int unsigned`，`last_warn_time` / `create_time` / `update_time`
        // 是 `bigint unsigned`，均需 CAST 成有符号整型才能解码进 `i32` / `i64`。
        sqlx::query_as::<_, StockWarningRule>(
            "SELECT CAST(id AS BIGINT) AS id, shop_id, product_id, sku_id, warn_enabled, lower_limit, upper_limit, last_warn_status, CAST(last_warn_time AS BIGINT) AS last_warn_time, is_delete, CAST(create_time AS BIGINT) AS create_time, CAST(update_time AS BIGINT) AS update_time FROM stock_warning_rule WHERE shop_id = $1 AND product_id = $2 AND sku_id = $3 AND is_delete = 0 LIMIT 1",
        )
        .bind(shop_id)
        .bind(product_id)
        .bind(sku_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 新增规则，返回自增主键（Java `baseMapper.insert`）。
    pub async fn insert_rule(&self, rule: &StockWarningRule) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO stock_warning_rule (shop_id, product_id, sku_id, warn_enabled, lower_limit, upper_limit, last_warn_status, last_warn_time, is_delete, create_time, update_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id",
        )
        .bind(rule.shop_id)
        .bind(rule.product_id)
        .bind(rule.sku_id)
        .bind(rule.warn_enabled)
        .bind(rule.lower_limit)
        .bind(rule.upper_limit)
        .bind(rule.last_warn_status)
        .bind(rule.last_warn_time)
        .bind(rule.is_delete)
        .bind(rule.create_time)
        .bind(rule.update_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 按主键更新规则（Java `updateById`：写开关、上下限、最近预警状态与更新时间）。
    pub async fn update_rule(&self, rule: &StockWarningRule) -> Result<bool> {
        let r = sqlx::query(
            "UPDATE stock_warning_rule SET warn_enabled = $1, lower_limit = $2, upper_limit = $3, last_warn_status = $4, update_time = $5 WHERE id = $6 AND is_delete = 0",
        )
        .bind(rule.warn_enabled)
        .bind(rule.lower_limit)
        .bind(rule.upper_limit)
        .bind(rule.last_warn_status)
        .bind(rule.update_time)
        .bind(rule.id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    // ————————————————————— 列表 / 导出 —————————————————————
    //
    // 列表与导出共用同一份查询（Java `warningQuery`），保证导出内容与页面所见一致。

    /// 库存预警分页（Java `selectWarningPage`）。
    ///
    /// MyBatis-Plus 的 `PaginationInnerInterceptor` 会把原语句包成
    /// `SELECT COUNT(*) FROM (原语句去掉 ORDER BY)`，故这里对「同一份
    /// 过滤条件」分别构造 count 与 list 两条 SQL，绑定参数顺序完全一致。
    pub async fn paginate_warning(
        &self,
        spec: &WarningQuerySpec,
        page: &PageQuery,
    ) -> Result<(u64, Vec<StockWarningRow>)> {
        let base = build_warning_sql(spec);
        // `base` 里全是 `?`（内层 CASE 4 个 → sp/r/po/p/sp 的 shop_id 5 个 → 勾选行/商品类型/
        // 关键词/外层筛选），必须统一编号，否则 `?` 会原样进 PG 被当成 jsonb 存在运算符。
        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) AS c FROM ({base}) cnt"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        cq = bind_warning(cq, spec);
        let (total,): (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let (limit, offset) = page.limit_offset();
        let list_sql = crate::db::to_pg_placeholders(&format!(
            "{base}ORDER BY {}, sku_id ASC LIMIT ? OFFSET ?",
            sort_expression(spec.sort_field.as_deref(), spec.sort_order.as_deref())
        ));
        let mut lq = sqlx::query_as::<_, StockWarningRow>(&list_sql);
        lq = bind_warning(lq, spec);
        let rows = lq
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.max(0) as u64, rows))
    }

    /// 库存预警不分页查询（Java `selectWarningList`，导出用）。
    pub async fn list_warning(&self, spec: &WarningQuerySpec) -> Result<Vec<StockWarningRow>> {
        let base = build_warning_sql(spec);
        let sql = crate::db::to_pg_placeholders(&format!(
            "{base}ORDER BY {}, sku_id ASC",
            sort_expression(spec.sort_field.as_deref(), spec.sort_order.as_deref())
        ));
        let mut q = sqlx::query_as::<_, StockWarningRow>(&sql);
        q = bind_warning(q, spec);
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 导入辅助 —————————————————————
    //
    // 批量查询商品编码和 SKU 编码两个输入列的候选：两类编码都可能重复或互相撞车，
    // 服务层按命中类型及商品 ID 交叉匹配，确保 SKU 属于输入商品并唯一定位；
    // 主体与删除条件和列表保持一致。

    /// 按编码批量解析商品/SKU 行（Java `selectTargetsBySn`）。
    pub async fn select_targets_by_sn(
        &self,
        shop_id: i32,
        sn_list: &[String],
    ) -> Result<Vec<WarningSnTarget>> {
        if sn_list.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; sn_list.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT ps.sku_sn AS sn, CAST(p.product_id AS BIGINT) AS product_id, CAST(ps.sku_id AS BIGINT) AS sku_id, CAST(1 AS BIGINT) AS match_type, CAST(0 AS BIGINT) AS sku_count FROM product_sku ps INNER JOIN product p ON p.product_id = ps.product_id LEFT JOIN store_product sp ON sp.product_id = p.product_id AND sp.shop_id = ? WHERE p.is_delete = 0 AND p.vendor_id IS NULL AND (p.shop_id = ? OR sp.shop_id = ?) AND ps.sku_sn IN ({placeholders}) UNION ALL SELECT p.product_sn AS sn, CAST(p.product_id AS BIGINT) AS product_id, CAST(0 AS BIGINT) AS sku_id, CAST(2 AS BIGINT) AS match_type, CAST((SELECT COUNT(*) FROM product_sku s WHERE s.product_id = p.product_id) AS BIGINT) AS sku_count FROM product p LEFT JOIN store_product sp ON sp.product_id = p.product_id AND sp.shop_id = ? WHERE p.is_delete = 0 AND p.vendor_id IS NULL AND (p.shop_id = ? OR sp.shop_id = ?) AND p.product_sn IN ({placeholders})"
        ));
        let mut q = sqlx::query_as::<_, WarningSnTarget>(&sql);
        q = q.bind(shop_id).bind(shop_id).bind(shop_id);
        for sn in sn_list {
            q = q.bind(sn);
        }
        q = q.bind(shop_id).bind(shop_id).bind(shop_id);
        for sn in sn_list {
            q = q.bind(sn);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量查询当前主体可见的相关商品 ID（Java `selectVisibleProductIds`）。
    ///
    /// 相关商品按库存预警当前主体过滤：平台商品必须已分配给当前门店，
    /// 门店自建商品只允许当前主体自己的商品；平台自营主体只取 `shop_id = 0`。
    pub async fn select_visible_product_ids(
        &self,
        shop_id: i32,
        product_ids: &[i32],
    ) -> Result<Vec<i32>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; product_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT DISTINCT CAST(p.product_id AS BIGINT) FROM product p LEFT JOIN store_product sp ON sp.product_id = p.product_id AND sp.shop_id = $1 AND sp.is_delete = 0 WHERE p.is_delete = 0 AND p.vendor_id IS NULL AND (p.shop_id = $2 OR (p.shop_id = 0 AND sp.shop_id = $3)) AND p.product_id IN ({placeholders})"
        ));
        let mut q = sqlx::query_as::<_, (i32,)>(&sql);
        q = q.bind(shop_id).bind(shop_id).bind(shop_id);
        for id in product_ids {
            q = q.bind(id);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 主体（店铺 / 门店）名称与类型（Java `shopMapper.selectById`）。
    pub async fn find_shop_brief(&self, shop_id: i32) -> Result<Option<ShopBrief>> {
        sqlx::query_as::<_, ShopBrief>(
            "SELECT shop_id, shop_title, shop_type FROM shop WHERE shop_id = $1 LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }
}

/// 按 [`WarningQuerySpec`] 拼出（不含 ORDER BY / LIMIT 的）完整查询。
///
/// SQL 与 Java `warningQuery` 逐子句对应，只有两处为等价改写：
/// 1. 外层状态筛选用派生表列 `t.stock_status` 代替重复的 CASE 表达式；
/// 2. 列别名统一改为 snake_case（Rust `FromRow` 直接按列名取值）。
///
/// 库存状态与 5.3 的展示规则一致：低于下限为低库存，高于上限为高库存，
/// 区间内为正常库存，无规则为未配置。放在外层基于内层已算好的
/// `current_stock` 判断，使状态筛选可下推 SQL，分页总数才准确。
///
/// 库存口径与 `ProductSkuMapper.getProductStockPageByO2O` 一致：门店独立库存优先；
/// 门店独立库存与总库存同时启用时两处都会扣减，可售库存取较小值。
///
/// 在途库存 = 采购单已下单但尚未入库的数量，与采购单详情的「剩余待收」同源。
/// 只取 `type = 1` 的采购订单（`type = 2` 是直录入库单，创建即已加库存），
/// 状态只取 1 待入库与 2 部分入库；已完成(3)与已关闭(4)的未收部分不再视为在途。
/// 供应链功能关闭时表为空表，聚合结果为 0，不影响列表。
///
/// **占位符顺序即绑定顺序**：内层库存口径 CASE（4 个）→ `sp.shop_id`
/// → `r.shop_id` → 在途子查询 `po.shop_id` → 主体范围（2 个）→ 勾选行
/// → 商品类型 → 关键词（3 个）→ 库存状态 → 预警开关。绑定见 [`bind_warning`]。
#[must_use]
pub fn build_warning_sql(spec: &WarningQuerySpec) -> String {
    let mut sql = String::from(concat!(
        "SELECT t.*, CAST(CASE WHEN t.rule_id IS NULL THEN 9 WHEN t.current_stock < t.lower_limit THEN 1 WHEN t.current_stock > t.upper_limit THEN 2 ELSE 0 END AS BIGINT) AS stock_status FROM ( ",
        "SELECT CAST(p.product_id AS BIGINT) AS product_id, CAST(COALESCE(ps.sku_id, 0) AS BIGINT) AS sku_id, p.product_name AS product_name, ps.sku_value AS product_sku_name, p.pic_thumb AS product_pic_thumb, p.product_sn AS product_sn, ps.sku_sn AS sku_sn, p.product_tsn AS product_tsn, CAST(p.shop_id AS BIGINT) AS product_shop_id, sp.shop_id AS store_shop_id, p.product_related AS product_related, CAST(r.id AS BIGINT) AS rule_id, r.warn_enabled AS warn_enabled, r.lower_limit AS lower_limit, r.upper_limit AS upper_limit, CAST(r.update_time AS BIGINT) AS update_time, CAST(CASE WHEN ? AND sp.store_product_id IS NOT NULL AND ps.sku_id IS NOT NULL THEN CASE WHEN ? THEN LEAST(COALESCE(sps.sku_stock, 0), COALESCE(ps.sku_stock, 0)) ELSE COALESCE(sps.sku_stock, 0) END WHEN ? AND sp.store_product_id IS NOT NULL THEN CASE WHEN ? THEN LEAST(COALESCE(sp.product_stock, 0), COALESCE(p.product_stock, 0)) ELSE COALESCE(sp.product_stock, 0) END WHEN ps.sku_id IS NOT NULL THEN COALESCE(ps.sku_stock, 0) ELSE COALESCE(p.product_stock, 0) END AS BIGINT) AS current_stock, CAST(COALESCE(pt.transit_stock, 0) AS BIGINT) AS transit_stock ",
        "FROM product p ",
        "LEFT JOIN product_sku ps ON ps.product_id = p.product_id ",
        "LEFT JOIN store_product sp ON sp.product_id = p.product_id AND sp.shop_id = ? ",
        "LEFT JOIN store_product_sku sps ON sps.store_product_id = sp.store_product_id AND sps.product_sku_id = ps.sku_id ",
        "LEFT JOIN stock_warning_rule r ON r.product_id = p.product_id AND r.sku_id = COALESCE(ps.sku_id, 0) AND r.shop_id = ? AND r.is_delete = 0 ",
        "LEFT JOIN ( SELECT poi.product_id AS product_id, poi.sku_id AS sku_id, SUM(GREATEST(poi.quantity - COALESCE(poi.received_quantity, 0), 0)) AS transit_stock FROM purchase_order_item poi INNER JOIN purchase_order po ON po.id = poi.order_id WHERE po.is_del = 0 AND poi.is_del = 0 AND po.type = 1 AND po.status IN (1, 2) AND po.shop_id = ? GROUP BY poi.product_id, poi.sku_id ) pt ON pt.product_id = p.product_id AND pt.sku_id = COALESCE(ps.sku_id, 0) ",
        "WHERE p.is_delete = 0 AND p.vendor_id IS NULL AND (p.shop_id = ? OR sp.shop_id = ?) ",
    ));

    // 导出按勾选行取数；放在内层可先收窄结果集，主体与删除条件仍然生效。
    if !spec.targets.is_empty() {
        sql.push_str("AND (p.product_id, COALESCE(ps.sku_id, 0)) IN (");
        for i in 0..spec.targets.len() {
            if i > 0 {
                sql.push_str(", ");
            }
            sql.push_str("(?, ?)");
        }
        sql.push_str(") ");
    }
    if spec.product_type.is_some() {
        sql.push_str("AND p.product_type = ? ");
    }
    if spec.keyword_like.is_some() {
        sql.push_str("AND (p.product_name LIKE ? OR p.product_sn LIKE ? OR ps.sku_sn LIKE ?) ");
    }
    sql.push_str(") t ");

    // 外层筛选：低/高/正常等已配置状态仅展示已开启库存预警的商品；
    // 未配置状态(9)仍需保留未配置项。
    let mut outer: Vec<&str> = Vec::new();
    if spec.stock_status.is_some() {
        outer.push("t.stock_status = ?");
        if spec.stock_status != Some(STOCK_STATUS_UNCONFIGURED) {
            outer.push("t.warn_enabled = 1");
        }
    }
    if spec.warn_enabled.is_some() {
        outer.push("t.warn_enabled = ?");
    }
    if !outer.is_empty() {
        sql.push_str("WHERE ");
        sql.push_str(&outer.join(" AND "));
        sql.push(' ');
    }
    sql
}

/// 排序字段采用白名单映射，避免直接拼接前端参数导致 SQL 注入。
///
/// 支持页面四个库存列：实物库存 `currentStock`（兼容 `physicalStock` /
/// `productStock`）、在途库存 `transitStock`、库存下限 `lowerLimit`、
/// 库存上限 `upperLimit`。未传或非法字段回退商品 ID 倒序；方向仅接受
/// `asc` / `ASC`（与 Java `<when test="query.sortOrder == 'asc' or
/// query.sortOrder == 'ASC'">` 一致），其余按 `desc` 处理。
#[must_use]
pub fn sort_expression(sort_field: Option<&str>, sort_order: Option<&str>) -> &'static str {
    let asc = matches!(sort_order, Some("asc" | "ASC"));
    match sort_field {
        Some("currentStock" | "physicalStock" | "productStock") => {
            if asc {
                "current_stock ASC"
            } else {
                "current_stock DESC"
            }
        }
        Some("transitStock") => {
            if asc {
                "transit_stock ASC"
            } else {
                "transit_stock DESC"
            }
        }
        Some("lowerLimit") => {
            if asc {
                "lower_limit ASC"
            } else {
                "lower_limit DESC"
            }
        }
        Some("upperLimit") => {
            if asc {
                "upper_limit ASC"
            } else {
                "upper_limit DESC"
            }
        }
        _ => "product_id DESC",
    }
}

/// 按 [`build_warning_sql`] 的占位符顺序绑定参数（详见该函数的顺序说明）。
fn bind_warning<'q, O>(
    mut q: QueryAs<'q, Postgres, O, PgArguments>,
    spec: &'q WarningQuerySpec,
) -> QueryAs<'q, Postgres, O, PgArguments> {
    let store_stock = i32::from(spec.use_store_stock);
    let total_stock = i32::from(spec.use_total_stock);
    // 内层库存口径 CASE：门店独立库存开关与总库存开关各出现两次
    q = q
        .bind(store_stock)
        .bind(total_stock)
        .bind(store_stock)
        .bind(total_stock);
    q = q.bind(spec.shop_id); // sp.shop_id（门店分配记录）
    q = q.bind(spec.shop_id); // r.shop_id（预警规则）
    q = q.bind(spec.shop_id); // po.shop_id（在途采购单，子查询内）
    q = q.bind(spec.shop_id); // p.shop_id
    q = q.bind(spec.shop_id); // sp.shop_id
    for (product_id, sku_id) in &spec.targets {
        q = q.bind(*product_id).bind(*sku_id);
    }
    if let Some(product_type) = spec.product_type {
        q = q.bind(product_type);
    }
    if let Some(keyword) = spec.keyword_like.as_deref() {
        q = q.bind(keyword).bind(keyword).bind(keyword);
    }
    if let Some(stock_status) = spec.stock_status {
        q = q.bind(stock_status);
    }
    if let Some(warn_enabled) = spec.warn_enabled {
        q = q.bind(warn_enabled);
    }
    q
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("库存预警数据不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_expression_follows_java_whitelist() {
        // 未传 / 非法字段回退商品 ID 倒序
        assert_eq!(sort_expression(None, None), "product_id DESC");
        assert_eq!(
            sort_expression(Some("1;DROP TABLE product"), None),
            "product_id DESC"
        );
        // 实物库存三种别名等价
        assert_eq!(
            sort_expression(Some("currentStock"), Some("asc")),
            "current_stock ASC"
        );
        assert_eq!(
            sort_expression(Some("physicalStock"), Some("ASC")),
            "current_stock ASC"
        );
        assert_eq!(
            sort_expression(Some("productStock"), Some("desc")),
            "current_stock DESC"
        );
        // 仅 asc / ASC 升序，其余（含 Asc）一律降序
        assert_eq!(
            sort_expression(Some("transitStock"), Some("Asc")),
            "transit_stock DESC"
        );
        assert_eq!(sort_expression(Some("lowerLimit"), None), "lower_limit DESC");
        assert_eq!(
            sort_expression(Some("upperLimit"), Some("asc")),
            "upper_limit ASC"
        );
    }

    #[test]
    fn warning_sql_placeholder_count_matches_binds() {
        // 无可选条件：4（库存口径）+ 3（主体 join/子查询）+ 2（主体范围）= 9
        let spec = WarningQuerySpec {
            shop_id: 7,
            ..Default::default()
        };
        let sql = build_warning_sql(&spec);
        assert_eq!(sql.matches('?').count(), 9);
        assert!(sql.contains("FROM product p"));
        assert!(sql.contains(") t "));
        // 无外层筛选时不出现外层 WHERE（内层 WHERE 仍在）
        assert!(!sql.contains("WHERE t."));

        // 勾选行 2 条（4 个占位符）+ 商品类型 + 关键词 3 + 状态 + 开关
        let spec = WarningQuerySpec {
            shop_id: 7,
            targets: vec![(1, 0), (2, 55)],
            product_type: Some(1),
            keyword_like: Some("%手机%".into()),
            stock_status: Some(1),
            warn_enabled: Some(1),
            ..Default::default()
        };
        let sql = build_warning_sql(&spec);
        assert_eq!(sql.matches('?').count(), 19);
        assert!(sql
            .contains("WHERE t.stock_status = ? AND t.warn_enabled = 1 AND t.warn_enabled = ?"));
    }

    #[test]
    fn warning_sql_unconfigured_status_keeps_unconfigured_rows() {
        let spec = WarningQuerySpec {
            shop_id: 0,
            stock_status: Some(9),
            ..Default::default()
        };
        let sql = build_warning_sql(&spec);
        // stockStatus = 9 时不附加 warn_enabled = 1，未配置项仍可见
        assert!(sql.contains("WHERE t.stock_status = ? "));
        assert!(!sql.contains("t.warn_enabled = 1"));

        let spec = WarningQuerySpec {
            shop_id: 0,
            stock_status: Some(2),
            ..Default::default()
        };
        let sql = build_warning_sql(&spec);
        assert!(sql.contains("t.stock_status = ? AND t.warn_enabled = 1"));
    }

    #[test]
    fn warning_sql_targets_use_row_constructor_in() {
        let spec = WarningQuerySpec {
            shop_id: 3,
            targets: vec![(10, 0)],
            ..Default::default()
        };
        let sql = build_warning_sql(&spec);
        assert!(sql.contains("AND (p.product_id, COALESCE(ps.sku_id, 0)) IN ((?, ?))"));
    }

    #[test]
    fn warning_sql_keeps_java_subject_and_delete_filters() {
        let spec = WarningQuerySpec {
            shop_id: 5,
            keyword_like: Some("%A%".into()),
            product_type: Some(2),
            ..Default::default()
        };
        let sql = build_warning_sql(&spec);
        // 主体范围与删除条件
        assert!(sql.contains("WHERE p.is_delete = 0 AND p.vendor_id IS NULL"));
        assert!(sql.contains("AND (p.shop_id = ? OR sp.shop_id = ?)"));
        // 规则 join 必须带 is_delete = 0 与主体条件
        assert!(sql.contains("r.is_delete = 0"));
        // 在途库存只统计 type=1 且状态 1/2 的采购单
        assert!(sql.contains("po.type = 1 AND po.status IN (1, 2)"));
    }
}