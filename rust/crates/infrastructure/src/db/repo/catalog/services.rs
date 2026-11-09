//! 商品服务域（catalog::services）仓储。
//!
//! 对应原 Java：
//! - `com.nextrade.mapper.product.ServicesMapper`（`BaseMapper<Services>`）
//! - `com.nextrade.service.product.impl.ServicesServiceImpl`（list / detail /
//!   create / update）
//! - `com.nextrade.service.common.impl.BaseServiceImpl`
//!   （`del` / `batch` / `updateField` 的通用实现）
//!
//! # 列名对齐（权威来源）
//!
//! 表 `product_services`（7 列，主键 `product_service_id`）来自
//! `java/database/pure.sql`，用
//! `cd rust && python tools/schema_parser.py --table product_services` 核对：
//! `product_service_id` / `product_service_name` / `product_service_desc` /
//! `ico_img` / `sort_order` / `default_on` / `shop_id`。
//!
//! 查询统一使用运行时 `sqlx::query_as`（**不使用** `query!` 宏：
//! 本机无 `DATABASE_URL`）。
//!
//! # 金额
//!
//! 本域不涉及任何金额列，故无 `rust_decimal::Decimal`。

use sqlx::PgPool;

use nextrade_core::entities::ProductServices;
use nextrade_core::models::catalog::services::{
    to_snake, MSG_INVALID_FIELD_VALUE, SERVICES_FIELDS,
};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// `product_services` 的固定 SELECT 列清单（7 列，顺序与
/// `entities::ProductServices` 字段一致，避免 `SELECT *` 与实体字段顺序耦合）。
const SERVICES_SELECT: &str = "SELECT product_service_id, product_service_name, product_service_desc, ico_img, sort_order, default_on, shop_id FROM product_services";

/// 排序字段白名单（对应 Java `BaseServiceImpl#getColumnByPropertyName`：
/// 属性名或列名命中实体列 → 该列，未知回退主键）。
const SORTABLE_COLUMNS: [&str; 7] = [
    "product_service_id",
    "product_service_name",
    "product_service_desc",
    "ico_img",
    "sort_order",
    "default_on",
    "shop_id",
];

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct ServicesRepository {
    pool: PgPool,
}

impl ServicesRepository {
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

    /// 按主键查询商品服务
    /// （对应 Java `ServicesServiceImpl#detail` 的 `this.getById(id)`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<ProductServices>> {
        let sql = format!("{SERVICES_SELECT} WHERE product_service_id = $1 LIMIT 1");
        sqlx::query_as::<_, ProductServices>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 商品服务分页（对应 Java `ServicesServiceImpl#list`）。
    ///
    /// Java 过滤条件只有 `keyword`（`like(productServiceName, keyword)`，
    /// 且判空口径是 `keyword != null` —— 空串会生成 `LIKE '%%'`，
    /// 净效果等同不过滤）；排序走 `BaseServiceImpl#buildSortOrder`
    /// （空 `sortField` 视为主键、`desc` 或空 `sortOrder` 一律降序）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn paginate(
        &self,
        keyword: Option<&str>,
        sort_field: Option<&str>,
        sort_order: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<ProductServices>)> {
        let like = like_pattern(keyword);
        let where_sql = if like.is_some() {
            "WHERE product_service_name LIKE $1"
        } else {
            ""
        };
        let column = resolve_sort_column(sort_field);
        let dir = resolve_sort_direction(sort_order);
        let (limit, offset) = page.limit_offset();

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM product_services {where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        if let Some(like) = like.as_deref() {
            cq = cq.bind(like);
        }
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let list_sql = crate::db::to_pg_placeholders(&format!(
            "{SERVICES_SELECT} {where_sql} ORDER BY {column} {dir} LIMIT ? OFFSET ?"
        ));
        let mut lq = sqlx::query_as::<_, ProductServices>(&list_sql);
        if let Some(like) = like.as_deref() {
            lq = lq.bind(like);
        }
        let rows = lq
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    // ————————————————————— 写操作 —————————————————————

    /// 新增商品服务，返回自增主键
    /// （对应 Java `ServicesServiceImpl#create` 的 `this.save(convertToBean(dto))`）。
    ///
    /// Java `save` 走 MyBatis-Plus `NOT_NULL` 策略：DTO 为 `null` 的列不进
    /// INSERT，由 DDL 默认值兜底；Rust 显式列出全部列，故默认值由服务层
    /// 按 `product_services` 的 DDL 补齐（见
    /// [`nextrade_core::models::catalog::services::DEFAULT_SORT_ORDER`] 等）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn insert(&self, s: &ProductServices) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO product_services (product_service_name, product_service_desc, ico_img, sort_order, default_on, shop_id) VALUES ($1, $2, $3, $4, $5, $6) RETURNING product_service_id",
        )
        .bind(&s.product_service_name)
        .bind(&s.product_service_desc)
        .bind(&s.ico_img)
        .bind(s.sort_order)
        .bind(s.default_on)
        .bind(s.shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 整行更新商品服务（主键之外的 6 列）
    /// （对应 Java `ServicesServiceImpl#update` 的 `this.updateById(...)`）。
    ///
    /// Java `updateById` 的 `NOT_NULL` 语义（`null` 列不进入 SET）由服务层
    /// 先读旧行再合并实现，因此本方法可以安全整行覆盖。
    ///
    /// 返回是否命中记录。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn update(&self, s: &ProductServices) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE product_services SET product_service_name = $1, product_service_desc = $2, ico_img = $3, sort_order = $4, default_on = $5, shop_id = $6 WHERE product_service_id = $7",
        )
        .bind(&s.product_service_name)
        .bind(&s.product_service_desc)
        .bind(&s.ico_img)
        .bind(s.sort_order)
        .bind(s.default_on)
        .bind(s.shop_id)
        .bind(s.product_service_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 物理删除单条
    /// （对应 Java `BaseServiceImpl#del` 的 `this.removeById(id)`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn delete(&self, id: i32) -> Result<bool> {
        let res = sqlx::query("DELETE FROM product_services WHERE product_service_id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 批量物理删除（对应 Java `removeByIds(ids)`）。
    ///
    /// `ids` 为空时**不发 SQL**，直接返回 0（Java `removeByIds(empty)`
    /// 同样不产生语句）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn delete_batch(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; ids.len()].join(",");
        let sql = crate::db::to_pg_placeholders(&format!("DELETE FROM product_services WHERE product_service_id IN ({placeholders})"));
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        let res = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 更新单个字段
    /// （对应 Java `BaseServiceImpl#updateField` 的
    /// `.eq(keyColumn, id).set(field, val)`）。
    ///
    /// ⚠️ Java 该通用方法**不**按店铺过滤；本实现保持一致。`val` 以字符串
    /// 绑定，数值列由 MySQL 按列类型转换（与 MyBatis 绑定行为一致）。
    ///
    /// `field` 必须是 [`SERVICES_FIELDS`] 中的数据库列名；这里再校验一次。
    ///
    /// # Errors
    /// 字段不在白名单 → `Error::Validation(field 的值有误)`；数据库失败 →
    /// `Error::Database`。
    pub async fn update_field(&self, id: i32, field: &str, val: &str) -> Result<bool> {
        if !SERVICES_FIELDS.contains(&field) {
            return Err(Error::Validation(MSG_INVALID_FIELD_VALUE.into()));
        }
        let sql = format!("UPDATE product_services SET \"{field}\" = $1 WHERE product_service_id = $2");
        let res = sqlx::query(&sql)
            .bind(val)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }
}

/// LIKE 通配模式（Java `queryWrapper.like(column, keyword)` 等价物）。
///
/// Java 判空口径是 `keyword != null`：空串会生成 `LIKE '%%'`（等价不过滤），
/// 因此这里只在 `None` 时返回 `None`。
#[must_use]
pub fn like_pattern(raw: Option<&str>) -> Option<String> {
    raw.map(|s| format!("%{s}%"))
}

/// 排序字段白名单解析（防 SQL 注入）。
///
/// 对应 Java `BaseServiceImpl#getColumnByPropertyName`：属性名或列名命中
/// 实体列 → 该列；否则回退主键 `product_service_id`。
#[must_use]
pub fn resolve_sort_column(field: Option<&str>) -> &'static str {
    match field {
        Some(f) => {
            let snake = to_snake(f);
            SORTABLE_COLUMNS
                .iter()
                .find(|c| **c == snake)
                .copied()
                .unwrap_or("product_service_id")
        }
        None => "product_service_id",
    }
}

/// 排序方向（Java `buildSortOrder`）：仅显式 `asc` 升序，其余（`desc` /
/// 空 / 非法值）一律降序（非法值在 Java 里会不生成 ORDER BY，这里取
/// 降序以保证结果稳定，属实现差异已在模块文档说明）。
#[must_use]
pub fn resolve_sort_direction(order: Option<&str>) -> &'static str {
    if order.is_some_and(|o| o.eq_ignore_ascii_case("asc")) {
        "ASC"
    } else {
        "DESC"
    }
}

/// 数据库错误映射（与 brand / print_config 等仓储同一约定）。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("商品服务不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::catalog::services::MSG_SERVICE_NOT_FOUND;

    /// `SERVICES_SELECT` 覆盖 `product_services` 全部 7 列、顺序与实体一致，
    /// 且不用 `SELECT *`。
    #[test]
    fn select_covers_all_columns_in_entity_order() {
        let listed: Vec<&str> = SERVICES_SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap()
            .split(", ")
            .collect();
        assert_eq!(
            listed,
            vec![
                "product_service_id",
                "product_service_name",
                "product_service_desc",
                "ico_img",
                "sort_order",
                "default_on",
                "shop_id",
            ]
        );
        assert_eq!(listed.len(), 7);
        assert_eq!(listed, SORTABLE_COLUMNS.to_vec());
        assert!(!SERVICES_SELECT.contains('*'));
    }

    /// 排序字段白名单：属性名 / 列名都认，未知回退主键（防注入）。
    #[test]
    fn sort_column_whitelist_and_fallback() {
        assert_eq!(resolve_sort_column(None), "product_service_id");
        assert_eq!(resolve_sort_column(Some("productServiceId")), "product_service_id");
        assert_eq!(resolve_sort_column(Some("sortOrder")), "sort_order");
        assert_eq!(resolve_sort_column(Some("defaultOn")), "default_on");
        assert_eq!(resolve_sort_column(Some("productServiceName")), "product_service_name");
        // 未知字段回退主键，防 SQL 注入
        assert_eq!(
            resolve_sort_column(Some("1;DROP TABLE product_services")),
            "product_service_id"
        );
    }

    /// 排序方向：仅 `asc` 升序，其余降序。
    #[test]
    fn sort_direction_defaults_to_desc() {
        assert_eq!(resolve_sort_direction(Some("asc")), "ASC");
        assert_eq!(resolve_sort_direction(Some("ASC")), "ASC");
        assert_eq!(resolve_sort_direction(Some("desc")), "DESC");
        assert_eq!(resolve_sort_direction(None), "DESC");
    }

    /// LIKE 模式：`None` 不加条件；空串仍生成 `%%`（Java `keyword != null`）。
    #[test]
    fn like_pattern_matches_java_like() {
        assert_eq!(like_pattern(None), None);
        assert_eq!(like_pattern(Some("")).as_deref(), Some("%%"));
        assert_eq!(like_pattern(Some("包邮")).as_deref(), Some("%包邮%"));
    }

    /// 错误文案与 Java `ServicesConstants` 逐字符一致。
    #[test]
    fn not_found_message_is_verbatim() {
        assert_eq!(MSG_SERVICE_NOT_FOUND, "商品服务不存在");
        assert_eq!(MSG_INVALID_FIELD_VALUE, "field 的值有误");
    }
}
