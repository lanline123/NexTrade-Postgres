//! 地区域仓储：`region` 表（system::region_client）。
//! 对应原 Java `RegionMapper`（`RegionServiceImpl extends BaseServiceImpl<RegionMapper, Region>`）。
//!
//! 列名严格对齐 `java/database/pure.sql` 的 `region` 表（8 列，已用
//! `python tools/schema_parser.py --table region` 核对）：
//!
//! | 列 | 类型 | 可空 |
//! |----|------|------|
//! | `region_id` | int unsigned | NOT NULL（主键） |
//! | `level` | tinyint unsigned | NULL（Java `getLevel` 可返回 null） |
//! | `region_name` | varchar(200) | NOT NULL |
//! | `parent_id` | mediumint unsigned | NOT NULL |
//! | `is_hot` | tinyint unsigned | NOT NULL |
//! | `sort_order` | int | NOT NULL |
//! | `first_word` | varchar(50) | NOT NULL |
//! | `standard_code` | varchar(8) | NULL |
//!
//! 另读 `config` 表（`biz_code` / `biz_val` / `is_del`）取「默认国家」
//! （Java `ConfigServiceImpl#getConfigByCode("defaultCountry")`），
//! 与 proxy_shopping_user_address 的 `CONFIG_DEFAULT_COUNTRY` 同一配置项；
//! 本文件为独占文件，不跨切片复用其常量，故在此重新声明。

use sqlx::PgPool;
use nextrade_core::entities::Region;
use nextrade_core::{Error, Result};

/// 系统配置项「默认国家」的 `biz_code`。
///
/// 来自 Java `SettingsEnum.DEFAULT_COUNTRY("defaultCountry", "默认国家", ...)`。
pub const CONFIG_DEFAULT_COUNTRY: &str = "defaultCountry";

#[derive(Clone)]
pub struct RegionClientRepository {
    pool: PgPool,
}

impl RegionClientRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供上层在事务/并发场景复用）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— region —————————————————————

    /// 按主键取地区（Java `BaseServiceImpl#getById`）。
    pub async fn find_by_id(&self, region_id: i32) -> Result<Option<Region>> {
        sqlx::query_as::<_, Region>("SELECT * FROM region WHERE region_id = $1 LIMIT 1")
            .bind(region_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按 ID 集合取地区（Java `getRegionNamesByRegionIds` 的 `in(regionId, ...)`）。
    pub async fn find_by_ids(&self, region_ids: &[i32]) -> Result<Vec<Region>> {
        if region_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; region_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT * FROM region WHERE region_id IN ({placeholders})"));
        let mut q = sqlx::query_as::<_, Region>(&sql);
        for id in region_ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 按父级 ID 取直接子节点（Java `getRegionByParentId` 的
    /// `eq(Region::getParentId, parentId)`）。
    ///
    /// 排序在服务层做（要按 `first_word` + 全拼两段排序），此处只保证
    /// 结果稳定可复现：`ORDER BY region_id ASC`。
    pub async fn find_children(&self, parent_id: i32) -> Result<Vec<Region>> {
        sqlx::query_as::<_, Region>(
            "SELECT * FROM region WHERE parent_id = $1 ORDER BY region_id ASC",
        )
        .bind(parent_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按层级取地区（Java `getProvinceList` 的 `eq(Region::getLevel, 2)`）。
    pub async fn find_by_level(&self, level: i32) -> Result<Vec<Region>> {
        sqlx::query_as::<_, Region>(
            "SELECT * FROM region WHERE level = $1 ORDER BY region_id ASC",
        )
        .bind(level)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按层级 + 热门标记取地区（Java `getCityRegion` 的
    /// `eq(level, 3).eq(is_hot, 1)`）。
    pub async fn find_by_level_hot(&self, level: i32, is_hot: i32) -> Result<Vec<Region>> {
        sqlx::query_as::<_, Region>(
            "SELECT * FROM region
             WHERE level = $1 AND is_hot = $2
             ORDER BY region_id ASC",
        )
        .bind(level)
        .bind(is_hot)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按层级取地区并按 `first_word` / `region_id` 升序（Java `getCityRegion`
    /// 的 `eq(level, 3).orderByAsc(firstWord).orderByAsc(regionId)`）。
    ///
    /// 注意：Java 的 `getProvinceList` 排序走的是 `sortByFirstWord`
    /// （把空的 `first_word` 排到最后），而这里用的是纯列升序，
    /// 两者口径不同，故拆成两个方法。
    pub async fn find_by_level_ordered(&self, level: i32) -> Result<Vec<Region>> {
        sqlx::query_as::<_, Region>(
            "SELECT * FROM region
             WHERE level = $1
             ORDER BY first_word ASC, region_id ASC",
        )
        .bind(level)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 地区名模糊匹配（Java `like(Region::getRegionName, region)` 的 `%keyword%`）。
    ///
    /// 可选叠加层级过滤（`getCityListByCityName` 会带 `level = 3`）。
    ///
    /// 反斜杠转义与 Java `SqlLike.like(..., SqlLike.DEFAULT_ESCAPE)` 对齐，
    /// 避免用户输入 `%` / `_` 时把 ILIKE 语义撑开。
    pub async fn find_by_name_like(
        &self,
        keyword: &str,
        level: Option<i32>,
    ) -> Result<Vec<Region>> {
        let pattern = format!("%{}%", escape_like(keyword));
        sqlx::query_as::<_, Region>(
            "SELECT * FROM region
             WHERE region_name ILIKE $1 ESCAPE '\\'
               AND ($2 IS NULL OR level = $3)
             ORDER BY region_id ASC",
        )
        .bind(&pattern)
        .bind(level)
        .bind(level)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 取全表第一条（Java `this.list().getFirst()`，`getUserRegion` 的兜底分支）。
    ///
    /// `ORDER BY region_id ASC` 对应 MyBatis-Plus `list()` 无 ORDER BY 时
    /// InnoDB 按主键返回的实际顺序。
    pub async fn find_first(&self) -> Result<Option<Region>> {
        sqlx::query_as::<_, Region>("SELECT * FROM region ORDER BY region_id ASC LIMIT 1")
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    // ————————————————————— config —————————————————————

    /// 读取系统配置项（Java `ConfigServiceImpl#getConfigByCode`）。
    ///
    /// `biz_code = ? AND is_del = 0 LIMIT 1`，缺行 → `None`。
    pub async fn find_config_val(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1",
        )
        .bind(biz_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }
}

/// 对 ILIKE 模式里的 `\` `%` `_` 做转义（配合 SQL 的 `ESCAPE '\\'`）。
pub fn escape_like(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("地区不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_like_escapes_wildcards() {
        assert_eq!(escape_like("北京"), "北京");
        assert_eq!(escape_like("50%"), "50\\%");
        assert_eq!(escape_like("a_b"), "a\\_b");
        assert_eq!(escape_like("c\\d"), "c\\\\d");
    }

    #[test]
    fn default_country_code_matches_java_enum() {
        // Java `SettingsEnum.DEFAULT_COUNTRY("defaultCountry", "默认国家", ...)`
        assert_eq!(CONFIG_DEFAULT_COUNTRY, "defaultCountry");
    }
}