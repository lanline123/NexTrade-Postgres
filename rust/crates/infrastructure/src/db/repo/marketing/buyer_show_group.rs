//! 买家秀分组域（marketing::buyer_show_group）仓储：`buyer_show_group` / `buyer_show` / `product_group`。
//!
//! 对应原 Java `BuyerShowGroupMapper`（+ `BaseServiceImpl` 的通用 CRUD）
//! 与 `BuyerShowMapper#selectCount` / `ProductGroupService#lambdaQuery`。
//!
//! 列名严格对齐 `java/database/pure.sql`
//! （`python tools/schema_parser.py --table buyer_show_group` 可复核）：
//! - `buyer_show_group`：`buyer_show_group_id`（主键）/ `group_name` /
//!   `product_group_id`（varchar(500)，存 JSON 数组文本）/ `shop_id`
//! - `buyer_show`：`group_data`（varchar(255)，存 JSON 数组文本，元素为该表的
//!   `buyer_show_group_id`）
//! - `product_group`：`product_group_id` / `product_group_name`
//!
//! 查询统一使用运行时 `sqlx::query_as::<_, T>(...)`（**不使用 `query!` 宏**，
//! 本机无 `DATABASE_URL`）。`ORDER BY` 列只允许来自 [`SORT_COLUMNS`] 白名单，
//! 其余条件值一律 `?` 占位绑定，不做字符串插值。
//!
//! > **写法约定**：本文件的每条 SQL 都写成**单行字符串字面量**，
//! > 以便 `python tools/check_sql_columns.py` 能完整提取并逐列校验
//! > （该工具无法解析跨行的多行字面量）。

use sqlx::PgPool;
use nextrade_core::entities::BuyerShowGroup;
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// 允许出现在 `ORDER BY` 中的列白名单（对应 Java
/// `BaseServiceImpl#getColumnByPropertyName` 的字段映射，且兜底为主键）。
const SORT_COLUMNS: [&str; 4] = [
    "buyer_show_group_id",
    "group_name",
    "product_group_id",
    "shop_id",
];

/// 缺省排序列（Java `getKeyColumn()`）。
const KEY_COLUMN: &str = "buyer_show_group_id";

/// 买家秀分组域仓储。
#[derive(Clone)]
pub struct BuyerShowGroupRepository {
    pool: PgPool,
}

impl BuyerShowGroupRepository {
    /// 构造仓储。
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— buyer_show_group —————————————————————

    /// 统计同名分组数（对应 Java `BuyerShowGroupServiceImpl#verifyGroupName` 的 `count()`）。
    ///
    /// Java 的 wrapper **不带** `shop_id` 条件（即名称在全库范围内唯一），
    /// `exclude_id` 对应 `ne(buyerShowGroupId != null, ...)`（更新时排除自身）。
    pub async fn count_group_by_name(
        &self,
        group_name: &str,
        exclude_id: Option<i32>,
    ) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM buyer_show_group WHERE group_name = $1 AND ($2 IS NULL OR buyer_show_group_id <> $3)",
        )
        .bind(group_name)
        .bind(exclude_id)
        .bind(exclude_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 按主键取分组（Java `BaseServiceImpl#getById`）。
    pub async fn find_by_id(&self, buyer_show_group_id: i32) -> Result<Option<BuyerShowGroup>> {
        sqlx::query_as::<_, BuyerShowGroup>(
            "SELECT buyer_show_group_id, group_name, product_group_id, shop_id FROM buyer_show_group WHERE buyer_show_group_id = $1 LIMIT 1",
        )
        .bind(buyer_show_group_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 新建分组，返回自增主键（Java `BaseServiceImpl#save`）。
    pub async fn insert_group(
        &self,
        group_name: &str,
        product_group_id: &str,
        shop_id: i32,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO buyer_show_group (group_name, product_group_id, shop_id) VALUES ($1, $2, $3) RETURNING buyer_show_group_id",
        )
        .bind(group_name)
        .bind(product_group_id)
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 按主键更新分组名与商品分组（Java `BaseServiceImpl#updateById`，此处只写变化列）。
    ///
    /// 返回受影响行数（`0` 表示行已被并发删除；Java 侧同样静默，不抛错）。
    pub async fn update_group(
        &self,
        buyer_show_group_id: i32,
        group_name: &str,
        product_group_id: &str,
    ) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE buyer_show_group SET group_name = $1, product_group_id = $2 WHERE buyer_show_group_id = $3",
        )
        .bind(group_name)
        .bind(product_group_id)
        .bind(buyer_show_group_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 按主键删除分组（Java `BaseServiceImpl#del` → `removeById`）。
    ///
    /// 返回受影响行数（`0` 表示不存在；Java 侧返回 `false` 后同样被控制器忽略）。
    pub async fn delete_group(&self, buyer_show_group_id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM buyer_show_group WHERE buyer_show_group_id = $1")
            .bind(buyer_show_group_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 分组分页（Java `BuyerShowGroupServiceImpl#list` 的 `this.page(...)`）。
    ///
    /// - `shop_id` 过滤对应 Java `queryWrapper.eq(BuyerShowGroup::getShopId, getShopId())`
    ///   （调用方保证该值来自 [`nextrade_core::models::marketing::buyer_show_group::BuyerShowGroupScope`]）；
    /// - `order_column` 必须来自 [`SORT_COLUMNS`]，否则回退主键；
    /// - `ascending` 为 `None` 时**不加 `ORDER BY`**（复刻 Java `buildSortOrder`
    ///   在 `sortOrder` 既非 asc 也非 desc/空 时的行为）。
    pub async fn paginate_groups(
        &self,
        shop_id: i32,
        order_column: &str,
        ascending: Option<bool>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<BuyerShowGroup>)> {
        let (limit, offset) = page.limit_offset();

        let total: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM buyer_show_group WHERE shop_id = $1")
                .bind(shop_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;

        let column = safe_sort_column(order_column);
        let order = match ascending {
            Some(true) => format!(" ORDER BY {column} ASC"),
            Some(false) => format!(" ORDER BY {column} DESC"),
            None => String::new(),
        };
        let sql = format!(
            "SELECT buyer_show_group_id, group_name, product_group_id, shop_id FROM buyer_show_group WHERE shop_id = $1{order} LIMIT $2 OFFSET $3"
        );
        let rows = sqlx::query_as::<_, BuyerShowGroup>(&sql)
            .bind(shop_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    // ————————————————————— buyer_show（计数） —————————————————————

    /// 统计某买家秀分组下的买家秀数量。
    ///
    /// 逐字对应 Java `BuyerShowGroupServiceImpl#list` 里的
    /// `buyerShowMapper.selectCount(new LambdaQueryWrapper<BuyerShow>()
    ///  .apply("JSON_CONTAINS(group_data, {0})", id.toString()))`：
    /// MySQL 会把绑定的 `"1"` 当作 JSON 字面量 `1` 与 `group_data` 数组元素比较。
    ///
    /// 说明：`group_data` 为 `NULL` 或 `JSON_CONTAINS` 结果为 `NULL` 时不计入
    /// （与 Java 的 `COUNT` 语义一致）；`group_data` 非法 JSON 时 MySQL 报错，
    /// 同样与 Java 行为一致。
    pub async fn count_buyer_shows_in_group(&self, buyer_show_group_id: i32) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM buyer_show WHERE group_data::jsonb @> ($1::text)::jsonb")
                .bind(buyer_show_group_id.to_string())
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0)
    }

    // ————————————————————— product_group（名称） —————————————————————

    /// 按商品分组 ID 批量取名称（Java
    /// `productGroupService.lambdaQuery().in(ProductGroup::getProductGroupId, ids).list()`）。
    ///
    /// `ids` 为空时直接返回空表（**不**生成 Java 那样的 `IN ()` 语法错误语句）。
    /// 结果显式按主键升序：Java 无 `ORDER BY`，InnoDB 对该主键 `IN` 查询按索引
    /// 升序返回，调用方依赖该顺序输出 `productGroupName`。
    pub async fn find_product_group_names(
        &self,
        ids: &[i32],
    ) -> Result<Vec<(i32, Option<String>)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT product_group_id, product_group_name FROM product_group WHERE product_group_id IN ({placeholders}) ORDER BY product_group_id ASC"
        ));
        let mut q = sqlx::query_as::<_, (i32, Option<String>)>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }
}

/// `ORDER BY` 列白名单校验（防注入兜底）：不在白名单内一律回退主键列。
#[must_use]
pub fn safe_sort_column(column: &str) -> &'static str {
    SORT_COLUMNS
        .iter()
        .find(|c| c.eq_ignore_ascii_case(column))
        .copied()
        .unwrap_or(KEY_COLUMN)
}

/// `sqlx::Error` → 项目统一错误。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("数据不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_column_whitelist_falls_back_to_key() {
        assert_eq!(safe_sort_column("group_name"), "group_name");
        assert_eq!(safe_sort_column("GROUP_NAME"), "group_name");
        assert_eq!(safe_sort_column("shop_id"), "shop_id");
        // 注入尝试 / 未知列 → 主键
        assert_eq!(safe_sort_column("group_name; DROP TABLE x"), KEY_COLUMN);
        assert_eq!(safe_sort_column("unknown"), KEY_COLUMN);
    }

    #[test]
    fn map_err_maps_row_not_found() {
        match map_err(sqlx::Error::RowNotFound) {
            Error::NotFound(msg) => assert_eq!(msg, "数据不存在"),
            other => panic!("期望 Error::NotFound，实际 {other:?}"),
        }
        assert!(matches!(
            map_err(sqlx::Error::Protocol("boom".into())),
            Error::Database(_)
        ));
    }
}