//! PC 前台公共域（system::pc）仓储：`pc_navigation` / `pc_cat_floor` /
//! `category` / `config`。
//!
//! 列名严格对齐 `java/database/pure.sql`（用
//! `python tools/schema_parser.py --table <表名>` 核对）；
//! 查询统一使用运行时 `sqlx::query_as`（**不使用** `query!` 宏）。
//!
//! 说明：
//! - `pc_navigation.sort_order` / `parent_id` 排序对应 Java
//!   `queryWrapper.orderByAsc(sortOrder).orderByAsc(parentId)`。
//! - `category` 的排序：Java 侧 `PcCatFloorServiceImpl#getCatFloor` 走
//!   `CategoryServiceImpl#getCategoryByIds`（`in(...)`，**无 ORDER BY**）；
//!   InnoDB 按主键顺序返回，这里显式写成 `ORDER BY category_id ASC` 使结果
//!   确定（语义与 InnoDB 的实际返回一致）。
//! - `config` 读取对应 Java `ConfigServiceImpl#getConfigVal`：
//!   `biz_code = ? AND is_del = 0`（缺行 → 调用方给默认值）。

use sqlx::PgPool;
use nextrade_core::entities::{Category, PcCatFloor, PcNavigation};
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct PcRepository {
    pool: PgPool,
}

impl PcRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— pc_navigation —————————————————————

    /// 前台可见导航（`PcNavigationServiceImpl#getAllNav`）。
    ///
    /// Java：
    /// ```text
    /// eq(isShow, 1)
    /// [type > 0 时 eq(type, ?)]            // 调用方传 0 → 不过滤
    /// [isMerchant == 0 && isO2o == 0 时 ne(id, 42)]
    /// orderByAsc(sortOrder).orderByAsc(parentId)
    /// ```
    ///
    /// `exclude_id`：需要排除的导航 id（对应 Java 的 42「商家入驻」，
    /// 仅在本项目 `is-merchant = 0` 且 `is-o2o = 0` 时才生效，
    /// 见服务层常量 `IS_MERCHANT` / `IS_O2O`）。
    pub async fn find_client_navs(&self, exclude_id: Option<i32>) -> Result<Vec<PcNavigation>> {
        match exclude_id {
            Some(id) => sqlx::query_as::<_, PcNavigation>(
                "SELECT * FROM pc_navigation WHERE is_show = 1 AND id <> $1 \
                 ORDER BY sort_order ASC, parent_id ASC",
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err),
            None => sqlx::query_as::<_, PcNavigation>(
                "SELECT * FROM pc_navigation WHERE is_show = 1 \
                 ORDER BY sort_order ASC, parent_id ASC",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(map_err),
        }
    }

    // ————————————————————— pc_cat_floor —————————————————————

    /// 前台展示的分类抽屉楼层（`PcCatFloorServiceImpl#getCatFloor`）。
    ///
    /// Java：`eq(isShow, 1).orderByAsc(sortOrder)`。
    pub async fn find_shown_cat_floors(&self) -> Result<Vec<PcCatFloor>> {
        sqlx::query_as::<_, PcCatFloor>(
            "SELECT * FROM pc_cat_floor WHERE is_show = 1 ORDER BY sort_order ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— category —————————————————————

    /// 可见分类（`CategoryServiceImpl#loadVisibleTreeSnapshot`）。
    ///
    /// Java：`lambdaQuery().eq(Category::getIsShow, Constants.YES).list()`
    /// —— **无 ORDER BY**；这里与 `repo/category.rs` 保持同样的确定性排序
    /// （`sort_order ASC, category_id ASC`，与 InnoDB 主键扫描顺序一致）。
    pub async fn find_visible_categories(&self) -> Result<Vec<Category>> {
        sqlx::query_as::<_, Category>(
            "SELECT * FROM category WHERE is_show = 1 ORDER BY sort_order ASC, category_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按 id 批量取分类（`CategoryServiceImpl#getCategoryByIds`）。
    ///
    /// Java：`in(Category::getCategoryId, ids)`，**不过滤 `is_show`**。
    /// `ids` 为空时 Java 直接返回空列表（不发 SQL）。
    pub async fn find_categories_by_ids(&self, ids: &[i32]) -> Result<Vec<Category>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM category WHERE category_id IN ({placeholders}) \
             ORDER BY category_id ASC"
        ));
        let mut q = sqlx::query_as::<_, Category>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— config —————————————————————

    /// 按 `biz_code` 取未删除配置值（Java `ConfigService#getConfigVal`）。
    ///
    /// `None` ↔ Java 查不到记录（Java 此时返回空串，由调用方 `unwrap_or_default`）。
    pub async fn find_config_val(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1")
                .bind(biz_code)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }
}

/// SQL 错误 → `nextrade_core::Error`。
fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}