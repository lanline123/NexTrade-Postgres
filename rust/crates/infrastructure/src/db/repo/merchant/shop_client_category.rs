//! 店铺分类域仓储（merchant::shop_client_category）：`shop_product_category`。
//!
//! 原 Java 侧没有独立 Mapper，`ShopProductCategoryServiceImpl` 直接继承
//! `BaseServiceImpl<ShopProductCategoryMapper, ShopProductCategory>` 使用
//! MyBatis-Plus 的 `getById` / `list(wrapper)`。本文件把这两类调用落成
//! 显式 SQL。
//!
//! 列名严格对齐 `java/database/pure.sql`（`python tools/schema_parser.py
//! --table shop_product_category`）：
//! - `category_id` / `category_name` / `shop_id` / `parent_id` /
//!   `sort_order` / `is_show` / `add_time` / `category_pic`
//!
//! SQL 一律运行时 `sqlx::query_as`（不用 `query!` 宏）。

use sqlx::PgPool;
use nextrade_core::entities::ShopProductCategory;
use nextrade_core::{Error, Result};

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct ShopClientCategoryRepository {
    pool: PgPool,
}

impl ShopClientCategoryRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 店铺分类树数据源（`ShopProductCategoryServiceImpl#tree` 的查询）：
    /// 仅取 `is_show = 1`，按 `parent_id, sort_order, category_id` 升序。
    ///
    /// 排序键与原 Java `LambdaQueryWrapper` 的三级 `orderByAsc` 一致，
    /// 保证树内同层节点顺序稳定。
    pub async fn find_categories_for_tree(
        &self,
        shop_id: i32,
    ) -> Result<Vec<ShopProductCategory>> {
        sqlx::query_as::<_, ShopProductCategory>(
            "SELECT * FROM shop_product_category
             WHERE shop_id = $1 AND is_show = 1
             ORDER BY parent_id ASC, sort_order ASC, category_id ASC",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按主键查分类（`parentTree` 逐级上溯用）。
    ///
    /// 对应 Java `BaseServiceImpl#getById(id)`：**不过滤 `is_show`**，
    /// 因此下架的分类仍可能出现在父级链中（与原 Java 行为一致）。
    pub async fn find_by_id(&self, category_id: i32) -> Result<Option<ShopProductCategory>> {
        sqlx::query_as::<_, ShopProductCategory>(
            "SELECT * FROM shop_product_category WHERE category_id = $1 LIMIT 1",
        )
        .bind(category_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 某父级下的同层分类（`parentTree` 的 `catList` 数据源）。
    ///
    /// 对应 Java：`eq(parent_id, ...).eq(is_show, 1).orderByAsc(sort_order)`。
    /// 注意原 Java **未按 `shop_id` 过滤**，此处保持一致。
    pub async fn find_siblings(&self, parent_id: i32) -> Result<Vec<ShopProductCategory>> {
        sqlx::query_as::<_, ShopProductCategory>(
            "SELECT * FROM shop_product_category
             WHERE parent_id = $1 AND is_show = 1
             ORDER BY sort_order ASC",
        )
        .bind(parent_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("店铺分类不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    // 需要真实 MySQL 的集成测试见 tests/ 目录。
    // 本文件的树构建/父级链纯逻辑由 service 层测试覆盖。
}